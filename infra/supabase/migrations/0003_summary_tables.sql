CREATE TABLE workflow_job_summaries (
  job_id text PRIMARY KEY,
  org_id text NOT NULL,
  job_kind text NOT NULL,
  status text NOT NULL,
  skill_path text,
  started_at timestamptz,
  completed_at timestamptz,
  error_type text,
  owner_id text NOT NULL,
  row_version bigint NOT NULL DEFAULT 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE error_manifest_summaries (
  manifest_id text PRIMARY KEY,
  org_id text NOT NULL,
  job_id text,
  error_type text NOT NULL,
  error_code text,
  incident_signature text,
  owner_id text NOT NULL,
  row_version bigint NOT NULL DEFAULT 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE validation_verdict_summaries (
  verdict_id text PRIMARY KEY,
  org_id text NOT NULL,
  worker_job_id text NOT NULL,
  verdict text NOT NULL,
  basis_sufficiency text,
  enforcement_eligible boolean NOT NULL DEFAULT false,
  owner_id text NOT NULL,
  row_version bigint NOT NULL DEFAULT 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE viewer_sync_status (
  org_id text PRIMARY KEY,
  sync_health text NOT NULL DEFAULT 'offline'
    CHECK (sync_health IN ('healthy', 'degraded', 'offline')),
  last_push_success_at timestamptz,
  last_push_attempt_at timestamptz,
  last_acked_local_seq bigint,
  pending_outbox_count integer NOT NULL DEFAULT 0,
  schema_epoch integer NOT NULL,
  projection_generation bigint NOT NULL DEFAULT 0,
  daemon_version text,
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE executors (
  executor_id text PRIMARY KEY,
  org_id text NOT NULL,
  heartbeat_at timestamptz NOT NULL,
  daemon_version text,
  started_at timestamptz NOT NULL,
  owner_id text NOT NULL,
  row_version bigint NOT NULL DEFAULT 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_workflow_job_summaries_status ON workflow_job_summaries(status);
CREATE INDEX idx_error_manifest_summaries_incident ON error_manifest_summaries(incident_signature);
CREATE INDEX idx_validation_verdict_summaries_worker ON validation_verdict_summaries(worker_job_id);
CREATE INDEX idx_executors_org_heartbeat ON executors(org_id, heartbeat_at DESC);
