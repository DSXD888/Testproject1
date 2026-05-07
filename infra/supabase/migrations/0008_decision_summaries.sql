CREATE TABLE operator_decision_summaries (
  process_id text PRIMARY KEY,
  org_id text NOT NULL,
  disposition text NOT NULL CHECK (disposition IN ('pending', 'watching', 'approved', 'rejected')),
  decision_stale boolean NOT NULL DEFAULT false,
  reviewed_at_stage_run_id text,
  owner_id text NOT NULL,
  row_version bigint NOT NULL DEFAULT 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE operator_decision_summaries ENABLE ROW LEVEL SECURITY;

CREATE POLICY operator_decision_summaries_viewer_operator_admin_select
  ON operator_decision_summaries
  FOR SELECT
  USING (l4_is_any_role(ARRAY['viewer', 'operator', 'admin']));

GRANT SELECT ON operator_decision_summaries TO anon, authenticated;
GRANT ALL ON operator_decision_summaries TO service_role;
