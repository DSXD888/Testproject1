---
skill_id: source-health/l5-marketplace-commercial-intelligence
skill_version: 2026.05.19-1
interaction_profile: headless_contract
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
validator_overlay_version: 2026.03.30-1
source_structure_version: l5-marketplace-commercial-intelligence-2026-05-19
model_profile: tier_1_lightweight
agent_cli: claude
side_effect_class: none
promotion_stage: candidate
auth_requirements: none
covered_sources:
  - alibaba
  - 1688
  - amazon
  - pcs
  - searates
  - freightos
  - taric
worker_behavior_version: source-health/l5-marketplace-commercial-intelligence@2026.05.19-1+1.0.0
validator_behavior_version: 2026.03.30-1
version_change_affects_behavior: true
gate_a_family_key: source-health
gate_a_family_prequalified: false
gate_b_exact_slice_key: source-health/l5-marketplace-commercial-intelligence::l5-marketplace-commercial-intelligence::source-health/l5-marketplace-commercial-intelligence@2026.05.19-1+1.0.0::2026.03.30-1
gate_b_exact_slice_ready: false
---

# source-health/l5-marketplace-commercial-intelligence

## Use when
Use this skill when the L5 marketplace source-health cron needs a lightweight aggregate canary for Alibaba, 1688, Amazon, PCS Scotland, SeaRates, Freightos, and TARIC readiness before commercial-intelligence jobs consume those sources.

## Do not use when
Do not use this skill for supplier quote extraction, freight quote extraction, tariff classification, procurement ingestion, authentication repair, paid connector setup, or any workflow that writes production supplier_quotes, landed_cost_estimates, or notice rows.

## Inputs
- A scheduled source-health canary request for `source_id = "l5-marketplace-commercial-intelligence"`.
- The seven expected source bindings: Alibaba, 1688, Amazon, PCS, SeaRates, Freightos, and TARIC.
- Latest available source-specific canary artifacts or live lightweight probe instructions for each source.
- Auth-state references for Alibaba and 1688 when present; the aggregate itself must not type credentials or procure accounts.
- A small time budget appropriate for a readiness sweep.

## Outputs
- A result with `checked_at` and `per_source_health` entries for all seven covered sources.
- A source-level status of `healthy`, `degraded`, `blocked`, `auth_required`, or `unknown`.
- Evidence paths or artifact references for each source verdict.
- Failure codes only when a source is unusable; healthy sources use `failure_code: null`.

## Non-goals
- Full browser scraping or direct page navigation.
- Paid API authentication, account procurement, or connector setup.
- Replacing source-specific skills such as `source-health/alibaba`, `auth-refresh/1688`, `landed-cost/searates-freight-estimate`, `landed-cost/freightos-freight-estimate`, `landed-cost/taric-cross-check`, or `procurement-ingestion/pcs`.
- Promotion of marketplace skills to stable; that remains owned by the marketplace fixture-promotion gate.

## Failure Codes
- SOURCE_CAPTCHA_BLOCKED
- SOURCE_SOFT_BLOCKED
- AUTH_STATE_STALE
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- SESSION_REFRESH_FAILED
- SOURCE_AUTH_STATE_INVALID
- PCS_OCDS_EMPTY_RESULT
- FREIGHT_ESTIMATE_FAILED
- TARIFF_REFERENCE_UNAVAILABLE
- AGENT_RUN_FAILED
- AGENT_STALLED

## Resume Strategy
Resume is a fresh aggregate sweep. Re-read the latest source-specific canary artifacts for all seven sources, rerun only the missing lightweight probes, and do not reuse partial `per_source_health` output from an interrupted attempt unless each retained source has an evidence path and a `checked_at` timestamp from the same cron window.

## Canary
Run one minimal case per covered source. The aggregate canary verifies that the expected source-specific skill exists, that its canary fixture is parseable, that no paid API auth check is requested, and that each source returns a health verdict with evidence. Alibaba and 1688 may report `auth_required` only through their source-specific auth-state checks; PCS, TARIC, SeaRates, Freightos, and Amazon must remain public/free or browser/CDP skill based.

## Playbook

1. Load the seven canary cases from `canary.inputs.jsonl`.
2. For each case, verify the referenced skill directory and canary file exist.
3. Confirm `paid_api_auth_check` is `false` and `expected_auth_requirement` is one of `none`, `operator_cookie`, or `local_auth_state`.
4. For Alibaba, use the latest `source-health/alibaba` result when available; if stale or missing, return `auth_required` or `unknown` with evidence instead of attempting login.
5. For 1688, verify the `auth-refresh/1688` coordination path exists before any production search skill can run; if auth-state evidence is stale, return `auth_required`.
6. For PCS, TARIC, SeaRates, Freightos, and Amazon, verify the public/free or browser-skill binding and emit `blocked` only when the source-specific canary evidence names an actual blocker.
7. Emit exactly one `per_source_health` entry for each covered source. Do not omit sources to make the aggregate look healthier.

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
