CREATE TABLE operator_decision_commands (
  command_id text PRIMARY KEY,
  process_id text NOT NULL,
  base_revision bigint NOT NULL,
  applied_revision bigint,
  disposition text NOT NULL CHECK (disposition IN ('pending', 'watching', 'approved', 'rejected')),
  result text NOT NULL CHECK (result IN ('applied', 'conflict', 'superseded')),
  response_payload_json jsonb,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_operator_decision_commands_process ON operator_decision_commands(process_id);
CREATE INDEX idx_operator_decision_commands_created ON operator_decision_commands(created_at);

CREATE OR REPLACE FUNCTION gc_operator_decision_commands(
  retention interval DEFAULT interval '30 days'
)
RETURNS integer
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
  deleted_count integer;
BEGIN
  IF retention IS NULL OR retention < interval '1 day' THEN
    RAISE EXCEPTION 'operator decision command ledger retention must be at least one day'
      USING ERRCODE = '22023';
  END IF;

  DELETE FROM operator_decision_commands
  WHERE created_at < now() - retention;

  GET DIAGNOSTICS deleted_count = ROW_COUNT;
  RETURN deleted_count;
END;
$$;

COMMENT ON FUNCTION gc_operator_decision_commands(interval)
  IS 'Daily maintenance entrypoint for operator_decision_commands; default retention is 30 days.';

ALTER TABLE operator_decision_commands ENABLE ROW LEVEL SECURITY;

CREATE POLICY operator_decision_commands_admin_select
  ON operator_decision_commands
  FOR SELECT
  USING (l4_request_role() = 'admin');

CREATE POLICY operator_decision_commands_daemon_select
  ON operator_decision_commands
  FOR SELECT
  USING (l4_request_role() = 'daemon');

GRANT SELECT ON operator_decision_commands TO authenticated;
GRANT ALL ON operator_decision_commands TO service_role;
GRANT EXECUTE ON FUNCTION gc_operator_decision_commands(interval) TO service_role;
