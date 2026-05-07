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
