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
