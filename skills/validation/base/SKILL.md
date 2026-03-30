---
skill_id: validation/base
skill_version: 2026.03.30-1
interaction_profile: headless_contract
result_schema_version: 1.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: read_only
---

# validation/base

## Use when
Use this skill when the daemon needs a generic semantic validator for a completed skill run and must judge manifest completeness, agent-vs-daemon agreement, narrative plausibility, result-sample consistency, `basis_sufficiency`, and structured drift deltas.

## Do not use when
Do not use this skill for schema validation already enforced by OQG, domain-specific business judgment that belongs in later overlays, raw forensic byte review, browser automation, or any workflow that mutates authoritative pipeline state.

## Inputs
- The full run manifest with YAML frontmatter, agent narrative sections, and daemon evidence appendix.
- A distilled `validator-rubric.yaml` for generic semantic quality checks.
- Structured drift deltas and OQG anomaly signals supplied by the daemon.
- An anomaly-stratified sample of result-contract items rather than the full result set.
- Explicit provenance tags for calibration corpus items.

## Outputs
- A validator verdict payload with `verdict`, `basis_sufficiency`, `enforcement_eligible`, `confidence_bps`, `issues`, and `summary`.
- Issue entries anchored to manifest or daemon appendix evidence references.
- `basis_sufficiency: low` when the manifest is too thin for a confident semantic judgment.

## Non-goals
- Recomputing schema validity or replacing OQG checks.
- Making domain-specific procurement, pricing, or bid decisions.
- Reading raw PTY/contract forensic bytes directly.
- Mutating stage heads, outbox rows, or enforcement state in Layer 3.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_SKILL_FAILED
- AGENT_RUN_FAILED
- AGENT_INTERNAL_ERROR
- AGENT_PARTIAL_FAILURE
- PAGE_SHAPE_DRIFT
- AGENT_AUTH_CHALLENGE

## Resume Strategy
Resume from the last validated manifest input if the validator emitted a checkpoint-free partial failure. If no durable verdict payload was produced, restart from the same manifest and rubric inputs rather than synthesizing missing evidence.

## Canary
Run three synthetic canaries: one clean-real manifest that should approve, one mutated-bad manifest that should reject, and one insufficient-basis manifest that should flag or downgrade basis sufficiency without guessing.

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
