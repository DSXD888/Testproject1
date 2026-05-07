CREATE SEQUENCE shared_company_profile_change_seq AS bigint;
CREATE SEQUENCE shared_source_configs_change_seq AS bigint;
CREATE SEQUENCE operator_decision_heads_change_seq AS bigint;

CREATE TABLE shared_company_profile (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id text NOT NULL,
  company_name text NOT NULL,
  registration_number text,
  vat_number text,
  address_json jsonb,
  contact_json jsonb,
  bid_profile_json jsonb,
  remote_change_seq bigint NOT NULL DEFAULT nextval('shared_company_profile_change_seq'),
  remote_revision bigint NOT NULL DEFAULT 1,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE shared_source_configs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id text NOT NULL,
  source_id text NOT NULL UNIQUE,
  source_type text NOT NULL CHECK (source_type IN ('procurement', 'supplier')),
  display_name text NOT NULL,
  enabled boolean NOT NULL DEFAULT true,
  skill_path text NOT NULL,
  schedule_cron text,
  config_json jsonb,
  remote_change_seq bigint NOT NULL DEFAULT nextval('shared_source_configs_change_seq'),
  remote_revision bigint NOT NULL DEFAULT 1,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE operator_decision_heads (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id text NOT NULL,
  process_id text NOT NULL UNIQUE,
  disposition text NOT NULL CHECK (disposition IN ('pending', 'watching', 'approved', 'rejected')),
  reviewed_basis_hash text,
  reviewed_manifest_hash text,
  reviewed_at_stage_run_id text,
  decision_stale boolean NOT NULL DEFAULT false,
  operator_id text,
  command_id text,
  remote_change_seq bigint NOT NULL DEFAULT nextval('operator_decision_heads_change_seq'),
  remote_revision bigint NOT NULL DEFAULT 1,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_shared_company_profile_org ON shared_company_profile(org_id);
CREATE INDEX idx_shared_source_configs_org ON shared_source_configs(org_id);
CREATE INDEX idx_shared_source_configs_enabled ON shared_source_configs(enabled);
CREATE INDEX idx_operator_decisions_process ON operator_decision_heads(process_id);
CREATE INDEX idx_operator_decisions_disposition ON operator_decision_heads(disposition);

CREATE OR REPLACE FUNCTION l4_remote_owned_touch()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
  IF TG_OP = 'INSERT' THEN
    IF NEW.remote_change_seq IS NULL THEN
      NEW.remote_change_seq := nextval(TG_ARGV[0]::regclass);
    END IF;
    IF NEW.remote_revision IS NULL THEN
      NEW.remote_revision := 1;
    END IF;
    NEW.created_at := COALESCE(NEW.created_at, now());
    NEW.updated_at := COALESCE(NEW.updated_at, now());
  ELSE
    NEW.remote_change_seq := nextval(TG_ARGV[0]::regclass);
    NEW.remote_revision := OLD.remote_revision + 1;
    NEW.created_at := OLD.created_at;
    NEW.updated_at := now();
  END IF;

  RETURN NEW;
END;
$$;

CREATE TRIGGER shared_company_profile_remote_change_seq
  BEFORE INSERT OR UPDATE ON shared_company_profile
  FOR EACH ROW
  EXECUTE FUNCTION l4_remote_owned_touch('shared_company_profile_change_seq');

CREATE TRIGGER shared_source_configs_remote_change_seq
  BEFORE INSERT OR UPDATE ON shared_source_configs
  FOR EACH ROW
  EXECUTE FUNCTION l4_remote_owned_touch('shared_source_configs_change_seq');

CREATE TRIGGER operator_decision_heads_remote_change_seq
  BEFORE INSERT OR UPDATE ON operator_decision_heads
  FOR EACH ROW
  EXECUTE FUNCTION l4_remote_owned_touch('operator_decision_heads_change_seq');
