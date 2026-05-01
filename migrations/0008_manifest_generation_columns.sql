ALTER TABLE manifests ADD COLUMN manifest_basis_hash TEXT;
ALTER TABLE manifests ADD COLUMN stoplight TEXT;
ALTER TABLE manifests ADD COLUMN suggested_action TEXT;
ALTER TABLE manifests ADD COLUMN suggested_action_reasoning TEXT;
ALTER TABLE manifests ADD COLUMN clarification_prompts_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE manifests ADD COLUMN manifest_template_version TEXT NOT NULL DEFAULT 'v1.0-2026-05-01';
ALTER TABLE manifests ADD COLUMN candidate_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE manifests ADD COLUMN priced_candidate_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE manifests ADD COLUMN clarification_prompt_count INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_manifests_stoplight ON manifests(stoplight);
CREATE INDEX IF NOT EXISTS idx_manifests_basis ON manifests(manifest_basis_hash);
