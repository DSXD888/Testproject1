CREATE TABLE schema_metadata (
  org_id text PRIMARY KEY,
  schema_epoch integer NOT NULL,
  projection_generation bigint NOT NULL DEFAULT 0,
  applied_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE active_writer_lease (
  org_id text PRIMARY KEY,
  holder_executor_id text,
  lease_expires_at timestamptz,
  writer_epoch integer NOT NULL DEFAULT 0,
  updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO schema_metadata (org_id, schema_epoch, projection_generation)
VALUES ('default', 9, 0)
ON CONFLICT (org_id) DO NOTHING;

CREATE OR REPLACE FUNCTION acquire_active_writer_lease(
  target_org_id text,
  target_executor_id text,
  lease_ttl_seconds integer DEFAULT 120
)
RETURNS jsonb
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
  org text := COALESCE(NULLIF(target_org_id, ''), 'default');
  ttl_seconds integer := GREATEST(COALESCE(lease_ttl_seconds, 120), 120);
  current_holder text;
  current_expiry timestamptz;
  current_epoch integer;
  next_epoch integer;
  new_expiry timestamptz := now() + make_interval(secs => ttl_seconds);
  lease_status text := 'acquired';
BEGIN
  IF NOT l4_is_any_role(ARRAY['admin', 'daemon']) THEN
    RAISE EXCEPTION 'acquire_active_writer_lease requires daemon role'
      USING ERRCODE = '42501';
  END IF;

  IF target_executor_id IS NULL OR btrim(target_executor_id) = '' THEN
    RAISE EXCEPTION 'acquire_active_writer_lease requires executor_id'
      USING ERRCODE = '22023';
  END IF;

  INSERT INTO active_writer_lease (
    org_id, holder_executor_id, lease_expires_at, writer_epoch
  ) VALUES (
    org, NULL, NULL, 0
  )
  ON CONFLICT (org_id) DO NOTHING;

  SELECT holder_executor_id, lease_expires_at, writer_epoch
    INTO current_holder, current_expiry, current_epoch
  FROM active_writer_lease
  WHERE org_id = org
  FOR UPDATE;

  IF current_holder = target_executor_id
     AND current_expiry IS NOT NULL
     AND current_expiry >= now() THEN
    next_epoch := current_epoch;
    lease_status := 'renewed';
  ELSIF current_holder IS NULL
        OR current_holder = target_executor_id
        OR current_expiry IS NULL
        OR current_expiry < now() THEN
    next_epoch := current_epoch + 1;
  ELSE
    RETURN jsonb_build_object(
      'status', 'conflict',
      'accepted', false,
      'org_id', org,
      'current_holder_executor_id', current_holder,
      'lease_expires_at', current_expiry,
      'writer_epoch', current_epoch,
      'next_safe_action', 'switch this install to viewer-only or perform explicit rehome'
    );
  END IF;

  UPDATE active_writer_lease
  SET holder_executor_id = target_executor_id,
      lease_expires_at = new_expiry,
      writer_epoch = next_epoch,
      updated_at = now()
  WHERE org_id = org;

  RETURN jsonb_build_object(
    'status', lease_status,
    'accepted', true,
    'org_id', org,
    'holder_executor_id', target_executor_id,
    'lease_expires_at', new_expiry,
    'writer_epoch', next_epoch,
    'heartbeat_interval_seconds', 30,
    'lease_ttl_seconds', ttl_seconds
  );
END;
$$;

CREATE OR REPLACE FUNCTION heartbeat_active_writer_lease(
  target_org_id text,
  target_executor_id text,
  lease_ttl_seconds integer DEFAULT 120
)
RETURNS jsonb
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
  org text := COALESCE(NULLIF(target_org_id, ''), 'default');
  ttl_seconds integer := GREATEST(COALESCE(lease_ttl_seconds, 120), 120);
  current_holder text;
  current_expiry timestamptz;
  current_epoch integer;
  new_expiry timestamptz := now() + make_interval(secs => ttl_seconds);
BEGIN
  IF NOT l4_is_any_role(ARRAY['admin', 'daemon']) THEN
    RAISE EXCEPTION 'heartbeat_active_writer_lease requires daemon role'
      USING ERRCODE = '42501';
  END IF;

  IF target_executor_id IS NULL OR btrim(target_executor_id) = '' THEN
    RAISE EXCEPTION 'heartbeat_active_writer_lease requires executor_id'
      USING ERRCODE = '22023';
  END IF;

  SELECT holder_executor_id, lease_expires_at, writer_epoch
    INTO current_holder, current_expiry, current_epoch
  FROM active_writer_lease
  WHERE org_id = org
  FOR UPDATE;

  IF current_holder IS DISTINCT FROM target_executor_id
     OR current_expiry IS NULL
     OR current_expiry < now() THEN
    RETURN jsonb_build_object(
      'status', 'conflict',
      'accepted', false,
      'org_id', org,
      'current_holder_executor_id', current_holder,
      'lease_expires_at', current_expiry,
      'writer_epoch', current_epoch,
      'next_safe_action', 'switch this install to viewer-only or perform explicit rehome'
    );
  END IF;

  UPDATE active_writer_lease
  SET lease_expires_at = new_expiry,
      updated_at = now()
  WHERE org_id = org
    AND holder_executor_id = target_executor_id;

  RETURN jsonb_build_object(
    'status', 'heartbeat',
    'accepted', true,
    'org_id', org,
    'holder_executor_id', target_executor_id,
    'lease_expires_at', new_expiry,
    'writer_epoch', current_epoch,
    'heartbeat_interval_seconds', 30,
    'lease_ttl_seconds', ttl_seconds
  );
END;
$$;

CREATE OR REPLACE FUNCTION release_active_writer_lease(
  target_org_id text,
  target_executor_id text
)
RETURNS jsonb
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
  org text := COALESCE(NULLIF(target_org_id, ''), 'default');
  released_epoch integer;
BEGIN
  IF NOT l4_is_any_role(ARRAY['admin', 'daemon']) THEN
    RAISE EXCEPTION 'release_active_writer_lease requires daemon role'
      USING ERRCODE = '42501';
  END IF;

  IF target_executor_id IS NULL OR btrim(target_executor_id) = '' THEN
    RAISE EXCEPTION 'release_active_writer_lease requires executor_id'
      USING ERRCODE = '22023';
  END IF;

  UPDATE active_writer_lease
  SET holder_executor_id = NULL,
      lease_expires_at = NULL,
      updated_at = now()
  WHERE org_id = org
    AND holder_executor_id = target_executor_id
  RETURNING writer_epoch INTO released_epoch;

  IF released_epoch IS NULL THEN
    RETURN jsonb_build_object(
      'status', 'not_held',
      'accepted', false,
      'org_id', org,
      'executor_id', target_executor_id
    );
  END IF;

  RETURN jsonb_build_object(
    'status', 'released',
    'accepted', true,
    'org_id', org,
    'executor_id', target_executor_id,
    'writer_epoch', released_epoch
  );
END;
$$;

ALTER TABLE schema_metadata ENABLE ROW LEVEL SECURITY;
ALTER TABLE active_writer_lease ENABLE ROW LEVEL SECURITY;

CREATE POLICY schema_metadata_admin_daemon_select
  ON schema_metadata
  FOR SELECT
  USING (l4_is_any_role(ARRAY['admin', 'daemon']));

CREATE POLICY schema_metadata_admin_daemon_update
  ON schema_metadata
  FOR UPDATE
  USING (l4_is_any_role(ARRAY['admin', 'daemon']))
  WITH CHECK (l4_is_any_role(ARRAY['admin', 'daemon']));

CREATE POLICY active_writer_lease_admin_daemon_select
  ON active_writer_lease
  FOR SELECT
  USING (l4_is_any_role(ARRAY['admin', 'daemon']));

CREATE POLICY active_writer_lease_admin_daemon_insert
  ON active_writer_lease
  FOR INSERT
  WITH CHECK (l4_is_any_role(ARRAY['admin', 'daemon']));

CREATE POLICY active_writer_lease_admin_daemon_update
  ON active_writer_lease
  FOR UPDATE
  USING (l4_is_any_role(ARRAY['admin', 'daemon']))
  WITH CHECK (l4_is_any_role(ARRAY['admin', 'daemon']));

GRANT SELECT, UPDATE ON schema_metadata TO authenticated;
GRANT SELECT, INSERT, UPDATE ON active_writer_lease TO authenticated;
GRANT ALL ON schema_metadata, active_writer_lease TO service_role;
GRANT EXECUTE ON FUNCTION acquire_active_writer_lease(text, text, integer) TO authenticated, service_role;
GRANT EXECUTE ON FUNCTION heartbeat_active_writer_lease(text, text, integer) TO authenticated, service_role;
GRANT EXECUTE ON FUNCTION release_active_writer_lease(text, text) TO authenticated, service_role;
