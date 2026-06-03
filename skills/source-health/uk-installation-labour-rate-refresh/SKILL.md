---
skill_id: source-health/uk-installation-labour-rate-refresh
skill_version: 2026.06.03-1
interaction_profile: http_public
result_schema_version: 1.0.0
source_structure_version: uk-installation-labour-rate-refresh-2026-q2
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
promotion_stage: candidate
auth_requirements: none
requires_cdp: false
covered_reference_families:
  - labour_rates
  - access_equipment_bands
  - installation_profiles
  - permit_tm_bands
worker_behavior_version: source-health/uk-installation-labour-rate-refresh@2026.06.03-1+1.0.0
version_change_affects_behavior: true
gate_a_family_key: source-health
gate_a_family_prequalified: false
gate_b_exact_slice_key: source-health/uk-installation-labour-rate-refresh::uk-installation-labour-rate-refresh::source-health/uk-installation-labour-rate-refresh@2026.06.03-1+1.0.0::none
gate_b_exact_slice_ready: false
---

# source-health/uk-installation-labour-rate-refresh

## Use when
Use this skill for the quarterly source-health refresh of the UK installation labour reference pack used by `landed-cost/uk-installation-labour-estimate`.

## Do not use when
Do not use this skill during a live landed-cost estimate, procurement qualification, supplier quote extraction, or any workflow that needs to mutate production reference JSON files. This is not a live landed-cost estimate skill.

## Inputs
- `source_reference_version`, normally `uk-installation-labour-2026-q2`.
- `candidate_version`, the proposed next reviewed rate-table version.
- `production_reference_root`, normally `skills/landed-cost/uk-installation-labour-estimate/reference`.
- `candidate_output_root`, an artifact directory outside the production reference root.
- Four requested families: `labour_rates`, `access_equipment_bands`, `installation_profiles`, and `permit_tm_bands`.
- Public source descriptors with `source_name`, `source_url`, `family`, and an expected `as_of` date.

## Outputs
- A result matching `result.schema.json`.
- Candidate updated JSON artifacts for all four families when public source evidence supports a change.
- Per-family provenance with source URLs, `as_of`, observed timestamp, and source role.
- A changed-cell ledger that names the family, table key, JSON field path, previous value, candidate value, source URL, and provenance id.
- `production_reference_mutation_performed: false` for every run.

## Non-goals
- No live per-estimate scraping or postcode-specific quote lookup.
- No browser, CDP, account sign-in, paid connector, or private rate-card access.
- No firm quote generation; candidate values remain screening-grade until reviewed.
- No in-place mutation of `skills/landed-cost/uk-installation-labour-estimate/reference/*.json` without an explicit reviewed update path.

## Failure Codes
- PUBLIC_SOURCE_UNAVAILABLE
- PUBLIC_SOURCE_SHAPE_DRIFT
- RATE_FAMILY_SOURCE_MISSING
- CANDIDATE_OUTPUT_WRITE_BLOCKED
- PROVENANCE_INCOMPLETE
- CHANGED_CELL_LEDGER_EMPTY
- PRODUCTION_REFERENCE_MUTATION_ATTEMPTED
- AGENT_RUN_FAILED
- AGENT_STALLED

## Resume Strategy
Resume as a fresh quarterly source-health sweep. Re-read the production reference files, discard partial candidate JSON unless its provenance and changed-cell ledger were completed in the same run, and write a new candidate artifact set under a new `candidate_output_root` run id. Never resume by editing production reference files in place.

## Canary
The canary feeds a stale `uk-installation-labour-2026-q2` request and expects candidate JSON outputs for `labour_rates`, `access_equipment_bands`, `installation_profiles`, and `permit_tm_bands`. It must include public source URLs, `as_of`, changed cells, provenance, and `production_reference_mutation_performed: false`.

## Playbook
1. Read the current production reference JSON files as immutable inputs.
2. Check public source descriptors for every requested family.
3. For labour rates, compare trade-rate and regional-multiplier evidence, storing ratios as bps and money as integer pence where applicable.
4. For access-equipment bands, compare operated-hire band evidence without folding plant or equipment costs into labour rates.
5. For installation profiles, compare catalogue scope, trade-hour map, warning-code, and maintenance-model evidence without inventing bespoke profiles.
6. For permit/TM bands, compare public authority and statutory charge evidence while keeping permit and traffic-management costs separate from labour.
7. Write candidate JSON files only under `candidate_output_root`.
8. Emit a changed-cell ledger for every candidate delta and link each delta to provenance.
9. If any family lacks source evidence, return `refresh_status: "partial"` or `"blocked"` with blockers rather than fabricating a candidate.
10. Confirm `production_reference_mutation_performed: false` and that no candidate output path starts with the production reference root.

## Run Manifest Instructions
Run Manifest Requirements

You must produce a run manifest that the daemon can finalize into the durable `run-manifest.md` artifact.

Required YAML frontmatter fields:
- manifest_version
- skill_id
- skill_version
- job_id
- session_id
- execution_mode
- started_at
- completed_at
- outcome_class
- items_processed
- items_skipped
- items_failed
- warnings_count
- agent_claimed_pages_visited
- agent_claimed_source_url
- agent_claimed_fingerprint_state
- agent_claimed_locale_state
- agent_claimed_currency_state

Required narrative sections:
- Summary
- Steps Taken
- Decisions Made
- Warnings
- Evidence

Major claims must be anchored to observable evidence such as result-contract fields, daemon appendix fields, or artifact refs. Do not rely on operator memory or hidden context.
