---
skill_id: source-health/alibaba
skill_version: 2026.03.30-1
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
validator_overlay_version: 2026.03.30-1
source_structure_version: alibaba-2026-03-30
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
promotion_stage: stable
worker_behavior_version: source-health/alibaba@2026.03.30-1+1.0.0
validator_behavior_version: 2026.03.30-1
version_change_affects_behavior: true
gate_a_family_key: source-health
gate_a_family_prequalified: false
gate_b_exact_slice_key: source-health/alibaba::alibaba::source-health/alibaba@2026.03.30-1+1.0.0::2026.03.30-1
gate_b_exact_slice_ready: false
requires_cdp: true
cdp_default_port: 9222
maxPages: 1
maxScrollCycles: 0
---

# source-health/alibaba

## Use when
Use this skill when the daemon needs a lightweight Alibaba source-health canary that proves reachability, authentication, locale/currency correctness, and a minimal search result before expensive supplier enrichment runs proceed.

## Do not use when
Do not use this skill for full supplier extraction, auth refresh, pricing capture, relation resolution, manifest generation, or any workflow that persists authoritative listing evidence.

## Inputs
- A source identifier of `alibaba`.
- An auth-state handle when available.
- Expected locale and currency expectations for the marketplace.
- A known-good minimal search query.
- A small time budget appropriate for a canary.

## Outputs
- A six-check source-health result (`reachability`, `fingerprint`, `auth_state`, `locale`, `challenge_rate`, `minimal_extraction`).
- A summarized health outcome (`green`, `amber`, `red`) with failure classification when relevant.
- Minimal sample metadata proving the known-good search still yields a usable listing.

## Non-goals
- Full supplier enrichment or listing evidence persistence.
- Session establishment or login repair.
- Cross-source cache sharing or relation decisions.

## Failure Codes
- AGENT_AUTH_CHALLENGE
- SOURCE_CAPTCHA_BLOCKED
- SOURCE_SOFT_BLOCKED
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- AGENT_SKILL_FAILED
- AGENT_STALLED
- AGENT_RUN_FAILED

## Resume Strategy
Resume is a fresh canary run. If the last canary failed, restart from the Alibaba landing/search page with the current auth-state handle, compare the observed fingerprint to the last passing baseline, and do not continue from a partially blocked page state.

## Pagination Bounds
- `maxPages`: 1
- `maxScrollCycles`: 0

## Canary
Verify Alibaba reachability, authenticated state, expected locale/currency, absence of challenge markers, and the presence of at least one usable result from a known-good search without advancing into full extraction.

## Playbook

### Mandatory CDP Discipline

This canary must attach to the operator's already-authenticated persistent Alibaba browser session. Do not launch a fresh browser profile, do not attempt login, do not solve captchas, do not log out, and do not close the operator browser.

Before navigation:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, write a valid source-health result with `health_state: "red"`, `checks.reachability: "failed"`, `checks.auth_state: "blocked"`, and `failure_classification: "CDP_CONNECT_FAILED"`. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

Open Alibaba through CDP:

```bash
agent-browser --cdp ${CDP_PORT} open "https://www.alibaba.com/"
agent-browser --cdp ${CDP_PORT} wait 2000
agent-browser --cdp ${CDP_PORT} snapshot
```

Check whether the persistent session is authenticated, verify locale/currency expectations, and perform only the minimal known-good search needed for source health. If a sign-in page, expired-session page, or captcha appears, write a structured red health result with `failure_classification: "AGENT_AUTH_CHALLENGE"` or `"SOURCE_CAPTCHA_BLOCKED"` and stop.

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
