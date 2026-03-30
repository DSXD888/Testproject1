---
skill_id: manifest-generation
skill_version: 1.0.0
interaction_profile: headless_contract
result_schema_version: 1.0.0
validation_policy: always
validation_sample_rate: 10000
validation_triggers:
  - fill_rate_drop
  - first_run_after_version_change
  - suspect_output_in_last_five
  - operator_requested
  - source_health_amber_or_red
  - page_fingerprint_unknown
  - continuation_chain_length_gt_one
  - high_value_opportunity
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: none
maxPages: 0
maxScrollCycles: 0
---

# manifest-generation

## Use when
Use this skill when the daemon needs a deterministic, process-scoped opportunity manifest synthesized from local pipeline data only.

## Do not use when
Do not use this skill for browser navigation, live source ingestion, supplier enrichment, or any workflow that depends on external auth or side effects.

## Inputs
- Local pipeline assessment data for one process.
- Candidate, pricing, landed-cost, and blocker data already persisted in SQLite.
- The process identifier and any precomputed resume-free job parameters.

## Outputs
- A process-scoped opportunity manifest artifact.
- A deterministic `decisionBasisHash`.
- Contract-mode result payload suitable for daemon validation.

## Non-goals
- Live browsing.
- Incremental cursor management.
- Any write outside the local manifest/result contract surfaces.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_INTERNAL_ERROR

## Resume Strategy
Resume is not applicable. This skill is single-process and deterministic over local inputs, so the daemon should rerun the whole contract step rather than partial continuation.

## Pagination Bounds
- `maxPages`: 0
- `maxScrollCycles`: 0

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
