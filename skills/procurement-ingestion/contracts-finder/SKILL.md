---
skill_id: procurement-ingestion/contracts-finder
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
maxPages: 5
maxScrollCycles: 2
---

# procurement-ingestion/contracts-finder

## Use when
Use this skill when the daemon needs incremental Contracts Finder notice retrieval for the source-batched ingestion loop using the official OCDS search surface and cursor-aware continuation semantics.

## Do not use when
Do not use this skill for Find a Tender retrieval, manifest synthesis, supplier enrichment, relation resolution, portal submission flows, or any local-only deterministic transformation.

## Inputs
- Cursor input from `source_cursors` with the current safe watermark.
- Optional `publishedFrom`, `publishedTo`, `stages`, `cursor`, and `limit` job parameters.
- A time budget so the skill can stop at a safe continuation boundary before rate-limit pressure or session drift.

## Outputs
- A batch of structured Contracts Finder notice records.
- Checkpoints written at committed batch boundaries.
- `continuation_required` when safe remaining work exceeds the current session budget.

## Non-goals
- Private bidder login or downstream submission-portal work.
- Cross-notice relation resolution or process merge decisions.
- Commercial enrichment, manifest rendering, or margin calculation.

## Failure Codes
- SOURCE_CHANGED
- AGENT_PROTOCOL_MISMATCH
- SOURCE_SOFT_BLOCKED
- SOURCE_CAPTCHA_BLOCKED
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- AGENT_STALLED

## Resume Strategy
Resume with a cursor backtrack window and a small overlap on the publication boundary. Re-fetch a short overlap after each committed checkpoint so an in-flight cursor move never skips a notice when Contracts Finder mutates or reorders recent releases.

## Pagination Bounds
- `maxPages`: 5
- `maxScrollCycles`: 2

## Canary
Fetch the five most recent notices from the official Contracts Finder OCDS search endpoint, verify release-package shape, verify `language = en-GB`, verify `GBP` contract-value expectations when value data is present, and treat HTTP `403` as a documented rate-limit condition that requires at least a five-minute backoff before retry.

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
