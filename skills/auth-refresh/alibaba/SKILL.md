---
skill_id: auth-refresh/alibaba
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
validator_overlay_version: 2026.03.30-1
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: local_state
promotion_stage: stable
worker_behavior_version: auth-refresh/alibaba@1.0.0+1.0.0
validator_behavior_version: 2026.03.30-1
version_change_affects_behavior: true
gate_a_family_key: auth-refresh
gate_a_family_prequalified: false
gate_b_exact_slice_key: auth-refresh/alibaba::alibaba::auth-refresh/alibaba@1.0.0+1.0.0::2026.03.30-1
gate_b_exact_slice_ready: false
maxPages: 2
maxScrollCycles: 1
---

# auth-refresh/alibaba

## Use when
Use this skill when the daemon needs to refresh Alibaba authenticated browser state after a canary or extraction skill reports stale auth, challenge screens, or expired session state.

## Do not use when
Do not use this skill for product extraction, search-result scraping, pricing capture, relation resolution, manifest generation, or any deterministic local-only transformation.

## Inputs
- A source identifier of `alibaba`.
- Existing auth-state metadata if available.
- Expected locale/currency expectations for the Alibaba session.
- A short time budget for interactive login recovery.

## Outputs
- A refreshed Alibaba auth-state handle suitable for later extraction skills.
- Explicit auth outcome metadata (`authenticated`, `challenge_detected`, `locale_state`, `currency_state`).
- `blocked_human_action` if the flow requires CAPTCHA or manual intervention.

## Non-goals
- Product discovery or candidate listing extraction.
- Publishing any authoritative commercial data.
- Reusing browser state across unrelated sources.

## Failure Codes
- AGENT_AUTH_CHALLENGE
- SOURCE_CAPTCHA_BLOCKED
- SOURCE_SOFT_BLOCKED
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- AGENT_STALLED
- AGENT_RUN_FAILED

## Resume Strategy
Resume from the most recent auth checkpoint if the login flow already reached the account or verification page. Otherwise restart from the Alibaba sign-in entry page with a clean browser context and the last known auth-state handle marked stale.

## Pagination Bounds
- `maxPages`: 2
- `maxScrollCycles`: 1

## Canary
Verify that a fresh Alibaba session reaches an authenticated landing page, preserves the expected locale/currency, and emits a reusable auth-state handle without scraping marketplace data.

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
