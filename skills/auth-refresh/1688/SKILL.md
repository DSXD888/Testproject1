---
skill_id: auth-refresh/1688
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: local_state
maxPages: 2
maxScrollCycles: 1
---

# auth-refresh/1688

## Use when
Use this skill when the daemon needs to refresh 1688 authenticated browser state after challenge markers, stale login cookies, or explicit auth-failure signals.

## Do not use when
Do not use this skill for Chinese product extraction, pricing capture, relation resolution, manifest generation, or any browser workflow that is not specifically about restoring valid 1688 auth state.

## Inputs
- A source identifier of `1688`.
- Existing auth-state metadata if available.
- Expected locale/currency expectations for the 1688 session.
- A short interactive login recovery budget.

## Outputs
- A refreshed 1688 auth-state handle suitable for later extraction skills.
- Explicit auth outcome metadata (`authenticated`, `challenge_detected`, `locale_state`, `currency_state`).
- `blocked_human_action` if CAPTCHA or manual verification prevents completion.

## Non-goals
- Product search, listing extraction, or enrichment.
- Publishing any authoritative marketplace data.
- Reusing auth state for non-1688 sources.

## Failure Codes
- AGENT_AUTH_CHALLENGE
- SOURCE_CAPTCHA_BLOCKED
- SOURCE_SOFT_BLOCKED
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- AGENT_STALLED
- AGENT_RUN_FAILED

## Resume Strategy
Resume from the last checkpointed auth phase if the flow already reached an account or verification boundary. Otherwise restart from the 1688 sign-in landing page with a clean browser context and the prior auth-state handle marked stale.

## Pagination Bounds
- `maxPages`: 2
- `maxScrollCycles`: 1

## Canary
Verify that a fresh 1688 session reaches an authenticated landing page, preserves the expected Chinese locale and currency context, and emits a reusable auth-state handle without scraping listing data.

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
