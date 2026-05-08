CREATE OR REPLACE FUNCTION l4_request_role()
RETURNS text
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
  claim_role text;
  claims text;
BEGIN
  claim_role := current_setting('request.jwt.claim.role', true);
  IF claim_role IS NOT NULL AND claim_role <> '' THEN
    RETURN claim_role;
  END IF;

  claims := current_setting('request.jwt.claims', true);
  IF claims IS NOT NULL AND claims <> '' THEN
    claim_role := claims::jsonb ->> 'role';
    IF claim_role IS NOT NULL AND claim_role <> '' THEN
      RETURN claim_role;
    END IF;
  END IF;

  claim_role := current_setting('app.current_role', true);
  IF claim_role IS NOT NULL AND claim_role <> '' THEN
    RETURN claim_role;
  END IF;

  RETURN current_user;
END;
$$;

CREATE OR REPLACE FUNCTION l4_is_any_role(allowed_roles text[])
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
  SELECT l4_request_role() = ANY (allowed_roles);
$$;

CREATE OR REPLACE FUNCTION l4_tombstone_retention_days()
RETURNS integer
LANGUAGE sql
STABLE
AS $$
  SELECT 14;
$$;

COMMENT ON FUNCTION l4_tombstone_retention_days() IS
  'L4-FR-3.6.17: default tombstone retention is at least 14 days; shorter retention requires anti-entropy/full-hash diff.';

DO $$
DECLARE
  table_name text;
BEGIN
  FOREACH table_name IN ARRAY ARRAY[
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
    'notice_cpv_codes',
    'product_commodity_codes',
    'lot_cpv_codes'
  ] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', table_name);
    EXECUTE format(
      'CREATE POLICY %I ON %I FOR SELECT USING (l4_is_any_role(ARRAY[''viewer'',''operator'',''admin'']))',
      table_name || '_viewer_operator_admin_select',
      table_name
    );
  END LOOP;

  FOREACH table_name IN ARRAY ARRAY[
    'workflow_job_summaries',
    'error_manifest_summaries',
    'validation_verdict_summaries',
    'viewer_sync_status',
    'executors'
  ] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', table_name);
    EXECUTE format(
      'CREATE POLICY %I ON %I FOR SELECT USING (l4_is_any_role(ARRAY[''viewer'',''operator'',''admin'']))',
      table_name || '_viewer_operator_admin_select',
      table_name
    );
  END LOOP;

  FOREACH table_name IN ARRAY ARRAY[
    'shared_company_profile',
    'shared_source_configs'
  ] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', table_name);
    EXECUTE format(
      'CREATE POLICY %I ON %I FOR SELECT USING (l4_request_role() = ''daemon'' OR l4_request_role() = ''admin'')',
      table_name || '_admin_daemon_select',
      table_name
    );
    EXECUTE format(
      'CREATE POLICY %I ON %I FOR ALL USING (l4_request_role() = ''admin'') WITH CHECK (l4_request_role() = ''admin'')',
      table_name || '_admin_all',
      table_name
    );
  END LOOP;
END;
$$;

ALTER TABLE operator_decision_heads ENABLE ROW LEVEL SECURITY;
CREATE POLICY operator_decision_heads_operator_admin_daemon_select
  ON operator_decision_heads
  FOR SELECT
  USING (
    l4_request_role() = 'daemon'
    OR l4_request_role() = 'admin'
    OR (l4_request_role() = 'operator' AND deleted_at IS NULL)
  );
CREATE POLICY operator_decision_heads_admin_all
  ON operator_decision_heads
  FOR ALL
  USING (l4_request_role() = 'admin')
  WITH CHECK (l4_request_role() = 'admin');

GRANT USAGE ON SCHEMA public TO anon, authenticated, service_role;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO anon, authenticated;
GRANT INSERT, UPDATE, DELETE ON shared_company_profile, shared_source_configs, operator_decision_heads TO authenticated;
GRANT ALL ON ALL TABLES IN SCHEMA public TO service_role;
