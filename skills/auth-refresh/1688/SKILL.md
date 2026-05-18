---
skill_id: auth-refresh/1688
skill_version: 2026.03.30-1
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
validator_overlay_version: 2026.03.30-1
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: local_state
promotion_stage: stable
worker_behavior_version: auth-refresh/1688@2026.03.30-1+1.0.0
validator_behavior_version: 2026.03.30-1
version_change_affects_behavior: true
gate_a_family_key: auth-refresh
gate_a_family_prequalified: false
gate_b_exact_slice_key: auth-refresh/1688::1688::auth-refresh/1688@2026.03.30-1+1.0.0::2026.03.30-1
gate_b_exact_slice_ready: false
requires_cdp: true
cdp_default_port: 9222
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
- A short time budget to verify operator-completed session recovery in CDP.

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
Resume by rechecking the operator persistent CDP browser. If the browser still shows an account or verification boundary requiring human action, emit `blocked_human_action` and leave the prior auth-state handle marked stale.

## Pagination Bounds
- `maxPages`: 2
- `maxScrollCycles`: 1

## Canary
Verify that the operator persistent CDP session reaches an authenticated 1688 landing page, preserves the expected Chinese locale and currency context, and emits a reusable auth-state handle without scraping listing data.

## Playbook

### Mandatory CDP Discipline

This skill must inspect the operator's already-authenticated persistent 1688 browser session and refresh local auth-state metadata only after the operator has completed any interactive login or verification manually. Do not launch a fresh browser profile, do not type credentials, do not solve captchas, do not log out, and do not close the operator browser.

Before navigation:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# Ensure CDP is up. The daemon's pre-flight gate SHOULD have done this already,
# but defense in depth: bin/ensure-cdp-9222 is idempotent and fast (<1s no-op
# when CDP is already up; otherwise launches Chrome with the operator's
# persistent profile at /Users/alinaaitova/.chrome-automation and polls up to
# 15s). Returns non-zero only if launch genuinely failed. Absolute path because
# the skill subprocess cwd is the per-skill workdir, not the project root.
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222

# Defensive reachability check. If this fails (helper exited non-zero, or a
# rare post-launch race), follow the "If CDP is unreachable" guidance below:
# write the skill-specific blocked result.json and stop.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, write a valid result with `authenticated: false`, `auth_state_handle: ""`, `challenge_detected: false`, `locale_state: "unknown"`, `currency_state: "unknown"`, and `blocked_human_action: "CDP_CONNECT_FAILED"`. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

Open 1688 through CDP:

```bash
agent-browser --cdp ${CDP_PORT} open "https://www.1688.com/"
agent-browser --cdp ${CDP_PORT} wait 2000
agent-browser --cdp ${CDP_PORT} snapshot
```

Inspect the visible session state. If authenticated, refresh only local auth-state metadata and write `authenticated: true`. If a sign-in page, expired-session page, verification step, or captcha appears, write `authenticated: false`, set `challenge_detected` appropriately, include `blocked_human_action` with `AGENT_AUTH_CHALLENGE` or `SOURCE_CAPTCHA_BLOCKED`, and stop.

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
