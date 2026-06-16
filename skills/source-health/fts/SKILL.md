---
skill_id: source-health/fts
skill_version: 2026.03.30-1
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
validator_overlay_version: 2026.03.30-1
source_structure_version: fts-2026-03-30
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
promotion_stage: stable
worker_behavior_version: source-health/fts@2026.03.30-1+1.0.0
validator_behavior_version: 2026.03.30-1
version_change_affects_behavior: true
gate_a_family_key: source-health
gate_a_family_prequalified: false
gate_b_exact_slice_key: source-health/fts::fts::source-health/fts@2026.03.30-1+1.0.0::2026.03.30-1
gate_b_exact_slice_ready: false
requires_cdp: true
cdp_default_port: 9222
maxPages: 1
maxScrollCycles: 0
---

# source-health/fts

## Use when
Use this skill when the daemon needs a lightweight Find a Tender source-health canary that proves reachability, page fingerprint, locale, and minimal extraction before scheduled procurement ingestion proceeds.

## Do not use when
Do not use this skill for full procurement ingestion, relation resolution, supplier enrichment, manifest generation, or any workflow that needs to persist authoritative notice batches.

## Inputs
- A source identifier of `fts`.
- Expected locale and currency expectations for the source.
- A known-good minimal extraction query or recency window.
- A small time budget appropriate for a canary.

## Outputs
- A six-check source-health result (`reachability`, `fingerprint`, `auth_state`, `locale`, `challenge_rate`, `minimal_extraction`).
- A summarized health outcome (`green`, `amber`, `red`) with failure classification when relevant.
- Minimal sample metadata proving extraction succeeded without committing a production batch.

## Non-goals
- Full OCDS batch ingestion or cursor advancement.
- Auth refresh for supplier marketplaces.
- Commercial intelligence or downstream scoring.

## Failure Codes
- AGENT_RUN_FAILED
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- SOURCE_SOFT_BLOCKED
- SOURCE_CAPTCHA_BLOCKED
- AGENT_SKILL_FAILED
- AGENT_STALLED

## Resume Strategy
Resume is a fresh canary run. If a prior canary failed, rerun from the source landing page and compare the new fingerprint and minimal extraction result against the last passing baseline rather than continuing mid-session.

## Pagination Bounds
- `maxPages`: 1
- `maxScrollCycles`: 0

## Canary
Fetch the minimal viable FTS notice set, verify the expected page fingerprint, verify `en-GB` locale and `GBP` value expectations when value data is present, ensure no challenge markers are visible, and confirm that at least one valid notice-level result can be extracted.

## Playbook

### Mandatory CDP Discipline

Use the operator persistent CDP browser for this public canary so health checks happen on the same observable browser surface as production browser skills. Do not launch a fresh browser profile and do not close the operator browser.

Before navigation:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# Ensure CDP is up. The daemon's pre-flight gate SHOULD have done this already,
# but defense in depth: bin/ensure-cdp-9222 is idempotent and fast (<1s no-op
# when CDP is already up; otherwise launches Chrome with the operator's
# persistent profile at ~/.chrome-automation and polls up to
# 15s). Returns non-zero only if launch genuinely failed. Absolute path because
# the skill subprocess cwd is the per-skill workdir, not the project root.
~/Desktop/testproject1/bin/ensure-cdp-9222

# Defensive reachability check. If this fails (helper exited non-zero, or a
# rare post-launch race), follow the "If CDP is unreachable" guidance below:
# write the skill-specific blocked result.json and stop.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, write a valid source-health result with `health_state: "red"`, `checks.reachability: "failed"`, and `failure_classification: "CDP_CONNECT_FAILED"`. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

Open the public Find a Tender canary URL through CDP:

```bash
agent-browser --cdp ${CDP_PORT} open "https://www.find-tender.service.gov.uk/Search/Results"
agent-browser --cdp ${CDP_PORT} wait 2000
agent-browser --cdp ${CDP_PORT} snapshot
```

Verify fingerprint and locale/currency expectations, then write the source-health result. If a challenge page appears, mark `checks.challenge_rate: "failed"` and `failure_classification: "SOURCE_CAPTCHA_BLOCKED"` rather than waiting.

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
