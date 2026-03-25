---
skill_id: procurement-ingestion/fts
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
side_effect_class: none
maxPages: 5
maxScrollCycles: 2
---

# procurement-ingestion/fts

## Use when
Use this skill when the daemon needs to ingest live Find a Tender OCDS notice batches from the public FTS source using cursor-driven retrieval and browser-compatible navigation semantics.

## Do not use when
Do not use this skill for manifest synthesis, supplier pricing, relation resolution, bidder portals, or any deterministic local-only transformation.

## Inputs
- Cursor input from `source_cursors` with the current safe watermark.
- Optional `updatedFrom`, `updatedTo`, `cursor`, and `limit` job parameters.
- A time budget so the skill can stop at a safe continuation boundary.

## Outputs
- A batch of structured OCDS notice records.
- Checkpoints written at notice-batch boundaries.
- `continuation_required` when more safe work remains after the time budget.

## Non-goals
- Relation resolution across notices or processes.
- Private portal login flows.
- Commercial enrichment or manifest rendering.

## Failure Codes
- SOURCE_CHANGED
- AGENT_PROTOCOL_MISMATCH
- SOURCE_SOFT_BLOCKED
- SOURCE_CAPTCHA_BLOCKED
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- AGENT_STALLED

## Resume Strategy
Resume with a cursor backtrack window. Re-fetch a small overlap after each committed checkpoint so a cursor advance never skips notices if the source mutates while the run is in flight.

## Pagination Bounds
- `maxPages`: 5
- `maxScrollCycles`: 2

## Canary
Fetch the five most recent notices, verify OCDS structure, verify locale/currency expectations, and fail fast if the FTS page fingerprint drifts.

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
