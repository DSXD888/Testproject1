CREATE TABLE IF NOT EXISTS sync_applied_row_hashes (
  entity_kind text NOT NULL,
  entity_id text NOT NULL,
  row_version bigint NOT NULL,
  content_hash text NOT NULL,
  last_local_seq bigint NOT NULL,
  batch_id text,
  updated_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (entity_kind, entity_id)
);

ALTER TABLE sync_applied_row_hashes ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS sync_applied_row_hashes_admin_daemon_select
  ON sync_applied_row_hashes;
CREATE POLICY sync_applied_row_hashes_admin_daemon_select
  ON sync_applied_row_hashes
  FOR SELECT
  USING (l4_is_any_role(ARRAY['admin', 'daemon']));

DROP POLICY IF EXISTS sync_applied_row_hashes_admin_daemon_all
  ON sync_applied_row_hashes;
CREATE POLICY sync_applied_row_hashes_admin_daemon_all
  ON sync_applied_row_hashes
  FOR ALL
  USING (l4_is_any_role(ARRAY['admin', 'daemon']))
  WITH CHECK (l4_is_any_role(ARRAY['admin', 'daemon']));

GRANT SELECT, INSERT, UPDATE ON sync_applied_row_hashes TO authenticated;
GRANT ALL ON sync_applied_row_hashes TO service_role;

CREATE OR REPLACE FUNCTION l4_push_owned_table_allowed(table_name text)
RETURNS boolean
LANGUAGE sql
IMMUTABLE
AS $$
  SELECT table_name = ANY (ARRAY[
    'procurement_processes',
    'contract_notices',
    'notice_lots',
    'tender_document_sets',
    'qualification_runs',
    'product_candidates',
    'search_profiles',
    'search_attempts',
    'supplier_quotes',
    'candidate_pricing_assessments',
    'landed_cost_estimates',
    'manifests',
    'pipeline_assessments',
    'bid_workflow_states',
    'process_stage_status',
    'process_stage_heads',
    'process_relations',
    'process_aliases',
    'document_extractions',
    'document_semantic_extractions',
    'workflow_job_summaries',
    'error_manifest_summaries',
    'validation_verdict_summaries',
    'executors',
    'operator_decision_summaries'
  ]);
$$;

CREATE OR REPLACE FUNCTION l4_canonical_json(payload jsonb)
RETURNS text
LANGUAGE sql
IMMUTABLE
AS $$
  SELECT CASE jsonb_typeof(payload)
    WHEN 'object' THEN
      '{' || COALESCE((
        SELECT string_agg(to_jsonb(key)::text || ':' || l4_canonical_json(value), ',' ORDER BY key)
        FROM jsonb_each(payload) AS object_item(key, value)
      ), '') || '}'
    WHEN 'array' THEN
      '[' || COALESCE((
        SELECT string_agg(l4_canonical_json(value), ',' ORDER BY item_index)
        FROM jsonb_array_elements(payload) WITH ORDINALITY AS array_item(value, item_index)
      ), '') || ']'
    ELSE payload::text
  END;
$$;

CREATE OR REPLACE FUNCTION l4_content_hash(payload jsonb)
RETURNS text
LANGUAGE sql
IMMUTABLE
AS $$
  SELECT encode(digest(l4_canonical_json(payload), 'sha256'), 'hex');
$$;

CREATE OR REPLACE FUNCTION l4_apply_push_owned_payload(
  target_table text,
  row_payload jsonb,
  local_seq bigint
)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
  apply_payload jsonb := row_payload || jsonb_build_object('last_local_seq', local_seq);
  unknown_columns text[];
  insert_columns text;
  select_columns text;
  update_columns text;
BEGIN
  IF NOT l4_push_owned_table_allowed(target_table) THEN
    RAISE EXCEPTION 'unsupported push-owned table: %', target_table
      USING ERRCODE = '22023';
  END IF;

  SELECT array_agg(key ORDER BY key)
    INTO unknown_columns
  FROM jsonb_object_keys(apply_payload) AS key
  WHERE NOT EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = target_table
      AND column_name = key
  );

  IF unknown_columns IS NOT NULL THEN
    RAISE EXCEPTION 'payload contains columns absent from %.%: %',
      'public', target_table, unknown_columns
      USING ERRCODE = '22023';
  END IF;

  SELECT string_agg(format('%I', key), ', ' ORDER BY key),
         string_agg(format('(record_value).%I', key), ', ' ORDER BY key),
         string_agg(format('%1$I = EXCLUDED.%1$I', key), ', ' ORDER BY key)
    INTO insert_columns, select_columns, update_columns
  FROM jsonb_object_keys(apply_payload) AS key
  WHERE key <> 'created_at';

  IF insert_columns IS NULL OR position('id' IN insert_columns) = 0 THEN
    RAISE EXCEPTION 'push-owned payload for % must contain id', target_table
      USING ERRCODE = '22023';
  END IF;

  EXECUTE format(
    'INSERT INTO %1$I (%2$s)
       SELECT %3$s
       FROM (SELECT jsonb_populate_record(NULL::%1$I, $1) AS record_value) hydrated
     ON CONFLICT (id) DO UPDATE SET %4$s',
    target_table,
    insert_columns,
    select_columns,
    update_columns
  )
  USING apply_payload;
END;
$$;

CREATE OR REPLACE FUNCTION apply_push_batch(batch_json jsonb, writer_epoch integer)
RETURNS jsonb
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
  org text := COALESCE(NULLIF(batch_json ->> 'org_id', ''), 'default');
  rows_json jsonb := batch_json -> 'rows';
  batch_id text := COALESCE(batch_json ->> 'batch_id', batch_json ->> 'stream_id', 'unknown');
  batch_executor_id text := batch_json ->> 'executor_id';
  batch_schema_epoch integer;
  batch_writer_epoch integer;
  remote_schema_epoch integer;
  remote_writer_holder text;
  remote_lease_expires_at timestamptz;
  remote_writer_epoch integer;
  row_item jsonb;
  row_payload jsonb;
  row_table text;
  row_entity_id text;
  row_content_hash text;
  expected_content_hash text;
  row_version bigint;
  row_schema_epoch integer;
  row_writer_epoch integer;
  row_stage_run_id text;
  row_commit_group_id text;
  row_local_seq bigint;
  row_index integer := 0;
  remote_row_version bigint;
  remote_content_hash text;
  invalid_rows jsonb := '[]'::jsonb;
  applied_rows jsonb := '[]'::jsonb;
  noop_rows jsonb := '[]'::jsonb;
BEGIN
  IF NOT l4_is_any_role(ARRAY['daemon', 'admin']) THEN
    RAISE EXCEPTION 'apply_push_batch requires daemon role'
      USING ERRCODE = '42501';
  END IF;

  IF writer_epoch IS NULL OR writer_epoch < 0 THEN
    RAISE EXCEPTION 'apply_push_batch requires non-negative writer_epoch'
      USING ERRCODE = '22023';
  END IF;

  IF jsonb_typeof(batch_json) <> 'object'
     OR jsonb_typeof(rows_json) <> 'array'
     OR jsonb_array_length(rows_json) = 0 THEN
    RAISE EXCEPTION 'apply_push_batch requires batch_json.rows as a non-empty array'
      USING ERRCODE = '22023';
  END IF;

  batch_schema_epoch := (batch_json ->> 'schema_epoch')::integer;
  batch_writer_epoch := (batch_json ->> 'writer_epoch')::integer;

  IF batch_executor_id IS NULL OR btrim(batch_executor_id) = '' THEN
    RETURN jsonb_build_object(
      'status', 'rejected',
      'accepted', false,
      'error', 'missing_executor_id',
      'batch_id', batch_id
    );
  END IF;

  IF batch_writer_epoch <> writer_epoch THEN
    RETURN jsonb_build_object(
      'status', 'rejected',
      'accepted', false,
      'error', 'writer_epoch_argument_mismatch',
      'batch_writer_epoch', batch_writer_epoch,
      'writer_epoch', writer_epoch
    );
  END IF;

  SELECT schema_epoch
    INTO remote_schema_epoch
  FROM schema_metadata
  WHERE org_id = org;

  IF remote_schema_epoch IS NULL OR remote_schema_epoch <> batch_schema_epoch THEN
    RETURN jsonb_build_object(
      'status', 'rejected',
      'accepted', false,
      'error', 'schema_epoch_mismatch',
      'remote_schema_epoch', remote_schema_epoch,
      'batch_schema_epoch', batch_schema_epoch
    );
  END IF;

  SELECT active_writer_lease.holder_executor_id,
         active_writer_lease.lease_expires_at,
         active_writer_lease.writer_epoch
    INTO remote_writer_holder,
         remote_lease_expires_at,
         remote_writer_epoch
  FROM active_writer_lease
  WHERE org_id = org
  FOR UPDATE;

  IF remote_writer_epoch IS NULL
     OR remote_writer_holder IS NULL
     OR remote_lease_expires_at IS NULL
     OR remote_lease_expires_at < now() THEN
    RETURN jsonb_build_object(
      'status', 'rejected',
      'accepted', false,
      'error', 'writer_lease_required',
      'current_holder_executor_id', remote_writer_holder,
      'lease_expires_at', remote_lease_expires_at,
      'next_safe_action', 'switch this install to viewer-only or perform explicit rehome'
    );
  END IF;

  IF remote_writer_epoch <> writer_epoch THEN
    RETURN jsonb_build_object(
      'status', 'rejected',
      'accepted', false,
      'error', 'stale_writer',
      'remote_writer_epoch', remote_writer_epoch,
      'writer_epoch', writer_epoch
    );
  END IF;

  IF remote_writer_holder <> batch_executor_id THEN
    RETURN jsonb_build_object(
      'status', 'rejected',
      'accepted', false,
      'error', 'writer_lease_conflict',
      'current_holder_executor_id', remote_writer_holder,
      'batch_executor_id', batch_executor_id,
      'lease_expires_at', remote_lease_expires_at,
      'writer_epoch', writer_epoch,
      'next_safe_action', 'switch this install to viewer-only or perform explicit rehome'
    );
  END IF;

  FOR row_item IN SELECT value FROM jsonb_array_elements(rows_json) AS batch_row(value) LOOP
    row_index := row_index + 1;
    row_table := row_item ->> 'entity_kind';
    row_entity_id := row_item ->> 'entity_id';
    row_payload := row_item -> 'payload_json';
    row_content_hash := row_item ->> 'content_hash';
    row_version := (row_item ->> 'row_version')::bigint;
    row_schema_epoch := (row_item ->> 'schema_epoch')::integer;
    row_writer_epoch := (row_item ->> 'writer_epoch')::integer;
    row_stage_run_id := row_item ->> 'stage_run_id';
    row_commit_group_id := row_item ->> 'commit_group_id';
    row_local_seq := (row_item ->> 'local_seq')::bigint;

    IF row_payload IS NULL
       OR jsonb_typeof(row_payload) <> 'object'
       OR NOT l4_push_owned_table_allowed(row_table)
       OR row_entity_id IS NULL
       OR row_payload ->> 'id' IS DISTINCT FROM row_entity_id
       OR (row_payload ->> 'row_version')::bigint IS DISTINCT FROM row_version
       OR row_schema_epoch <> batch_schema_epoch
       OR row_writer_epoch <> writer_epoch
       OR row_stage_run_id IS NULL
       OR row_stage_run_id = ''
       OR row_commit_group_id IS NULL
       OR row_commit_group_id = ''
       OR row_local_seq IS NULL THEN
      invalid_rows := invalid_rows || jsonb_build_array(jsonb_build_object(
        'row_index', row_index,
        'entity_kind', row_table,
        'entity_id', row_entity_id,
        'error', 'invalid_batch_row'
      ));
      CONTINUE;
    END IF;

    expected_content_hash := l4_content_hash(row_payload);
    IF expected_content_hash <> row_content_hash THEN
      invalid_rows := invalid_rows || jsonb_build_array(jsonb_build_object(
        'row_index', row_index,
        'entity_kind', row_table,
        'entity_id', row_entity_id,
        'error', 'content_hash_mismatch',
        'expected_content_hash', expected_content_hash,
        'content_hash', row_content_hash
      ));
      CONTINUE;
    END IF;

    SELECT applied.row_version, applied.content_hash
      INTO remote_row_version, remote_content_hash
    FROM sync_applied_row_hashes AS applied
    WHERE applied.entity_kind = row_table
      AND applied.entity_id = row_entity_id;

    IF remote_row_version IS NOT NULL THEN
      IF row_version = remote_row_version AND row_content_hash = remote_content_hash THEN
        CONTINUE;
      ELSIF row_version = remote_row_version AND row_content_hash <> remote_content_hash THEN
        invalid_rows := invalid_rows || jsonb_build_array(jsonb_build_object(
          'row_index', row_index,
          'entity_kind', row_table,
          'entity_id', row_entity_id,
          'error', 'hard_conflict',
          'classification', 'data_corruption'
        ));
      ELSIF row_version = remote_row_version + 1 THEN
        CONTINUE;
      ELSE
        invalid_rows := invalid_rows || jsonb_build_array(jsonb_build_object(
          'row_index', row_index,
          'entity_kind', row_table,
          'entity_id', row_entity_id,
          'error', 'stale_or_out_of_order',
          'remote_row_version', remote_row_version,
          'incoming_row_version', row_version
        ));
      END IF;
    END IF;
  END LOOP;

  IF jsonb_array_length(invalid_rows) > 0 THEN
    RETURN jsonb_build_object(
      'status', 'rejected',
      'accepted', false,
      'invalid_rows', invalid_rows,
      'row_count', jsonb_array_length(rows_json),
      'writer_epoch', writer_epoch
    );
  END IF;

  FOR row_item IN SELECT value FROM jsonb_array_elements(rows_json) AS batch_row(value) LOOP
    row_table := row_item ->> 'entity_kind';
    row_entity_id := row_item ->> 'entity_id';
    row_payload := row_item -> 'payload_json';
    row_content_hash := row_item ->> 'content_hash';
    row_version := (row_item ->> 'row_version')::bigint;
    row_local_seq := (row_item ->> 'local_seq')::bigint;

    SELECT applied.row_version, applied.content_hash
      INTO remote_row_version, remote_content_hash
    FROM sync_applied_row_hashes AS applied
    WHERE applied.entity_kind = row_table
      AND applied.entity_id = row_entity_id;

    IF remote_row_version IS NOT NULL
       AND row_version = remote_row_version
       AND row_content_hash = remote_content_hash THEN
      noop_rows := noop_rows || jsonb_build_array(jsonb_build_object(
        'entity_kind', row_table,
        'entity_id', row_entity_id,
        'row_version', row_version,
        'local_seq', row_local_seq
      ));
      CONTINUE;
    END IF;

    PERFORM l4_apply_push_owned_payload(row_table, row_payload, row_local_seq);

    INSERT INTO sync_applied_row_hashes (
      entity_kind, entity_id, row_version, content_hash, last_local_seq, batch_id, updated_at
    ) VALUES (
      row_table, row_entity_id, row_version, row_content_hash, row_local_seq, batch_id, now()
    )
    ON CONFLICT (entity_kind, entity_id) DO UPDATE
      SET row_version = EXCLUDED.row_version,
          content_hash = EXCLUDED.content_hash,
          last_local_seq = EXCLUDED.last_local_seq,
          batch_id = EXCLUDED.batch_id,
          updated_at = now();

    applied_rows := applied_rows || jsonb_build_array(jsonb_build_object(
      'entity_kind', row_table,
      'entity_id', row_entity_id,
      'row_version', row_version,
      'local_seq', row_local_seq
    ));
  END LOOP;

  RETURN jsonb_build_object(
    'status', 'applied',
    'accepted', true,
    'row_count', jsonb_array_length(rows_json),
    'applied_rows', applied_rows,
    'noop_rows', noop_rows,
    'writer_epoch', writer_epoch
  );
END;
$$;

CREATE OR REPLACE FUNCTION update_operator_decision(
  process_id text,
  disposition text,
  base_revision bigint,
  command_id text
)
RETURNS jsonb
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
  IF NOT l4_is_any_role(ARRAY['operator', 'admin']) THEN
    RAISE EXCEPTION 'update_operator_decision requires operator role'
      USING ERRCODE = '42501';
  END IF;

  IF disposition NOT IN ('pending', 'watching', 'approved', 'rejected') THEN
    RAISE EXCEPTION 'invalid operator disposition: %', disposition
      USING ERRCODE = '22023';
  END IF;

  RETURN jsonb_build_object(
    'status', 'not_implemented',
    'accepted', false,
    'process_id', process_id,
    'base_revision', base_revision,
    'command_id', command_id
  );
END;
$$;

GRANT EXECUTE ON FUNCTION apply_push_batch(jsonb, integer) TO authenticated, service_role;
GRANT EXECUTE ON FUNCTION update_operator_decision(text, text, bigint, text) TO authenticated, service_role;
