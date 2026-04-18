---
skill_id: pdf-extraction/pricing-schedule
skill_version: 1.0.0
interaction_profile: headless_contract
result_schema_version: 1.0.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: none
---

# pdf-extraction/pricing-schedule

## Use when
Use this skill for pricing schedules, rate cards, schedule-of-rates packs, or mixed tender documents where the resolved overlay set includes pricing extraction and the daemon needs evidence-bearing line items for `target_domain = "pricing"`.

## Do not use when
Do not use this skill for tender specifications whose primary output is lot structure and requirements, bills of quantities that need deterministic quantity-merge keys, raw PDF parsing, browser tasks, or any workflow that mutates authoritative tables directly.

## Inputs
- Docling-produced markdown for the full document or deterministic chunk under review.
- Structured table fragments, table headers, and repeated-header hints from the structural extraction stage.
- Document metadata including resolved role hints, process context, and lot metadata for cross-reference only.
- Extraction-policy metadata including the expected target domain, extraction scope, and policy version.

## Outputs
- `line_items` with evidence-bearing pricing rows including quantities, units, rates, totals, and table-fragment lineage.
- `pricing_summary` with total value, currency, and lot-count rollups suitable for daemon-side persistence.
- `field_confidences` keyed by stable field paths so daemon-side persistence can reason about thin evidence.
- `warnings`, `evidence_anchors`, and `extraction_metadata` that make repeated-header normalization and extraction scope auditable.

## CRITICAL: Result Contract

You MUST write your output as a valid JSON file named `result.json` in the current working directory. The JSON must match `result.schema.json` exactly. Do NOT output a narrative text response. Write ONLY the structured JSON result file. The daemon reads this file after the session completes.

## Non-goals
- Reconstructing layout or OCR that should already exist in the Docling structural output.
- Guessing currencies, unit rates, or totals when the source table is ambiguous or incomplete.
- Emitting candidate or lot rows as separate target domains; this overlay persists a single semantic row for `pricing`.
- Hiding repeated-header ambiguity or page-fragment uncertainty instead of surfacing it in warnings and evidence anchors.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_SKILL_FAILED
- AGENT_RUN_FAILED
- AGENT_PARTIAL_FAILURE

## Resume Strategy
This overlay should normally complete in one contract pass over a full document or deterministic chunk. On retry, restart from the same structural payload and preserve stable `source_table_key` naming instead of inventing new row identities from partial output.

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

Major claims must be anchored to observable evidence such as table headers, repeated-header fragments, page references, lot references, or explicit line-item anchors from the supplied structural payload. If a rate, quantity, or total cannot be defended with evidence, omit it or downgrade confidence instead of guessing.
