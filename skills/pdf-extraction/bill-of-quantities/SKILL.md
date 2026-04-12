---
skill_id: pdf-extraction/bill-of-quantities
skill_version: 1.0.0
interaction_profile: headless_contract
result_schema_version: 1.0.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: none
---

# pdf-extraction/bill-of-quantities

## Use when
Use this skill for bills of quantities, schedules of quantities, quantified works packages, and long tabular annexes where quantities, units, rates, and subtotal continuity matter more than prose interpretation.

## Do not use when
Do not use this skill for pricing schedules whose primary output is rate-card pricing, tender specifications, raw PDF parsing, browser automation, or documents that only contain narrative requirements without quantity rows.

## Inputs
- Docling markdown for the full document or a chunk-scoped extraction packet.
- Structural table fragments, headings, and page anchors from the structural extraction stage.
- `document_metadata` including role hints, extraction policy version, and any known chunk-merge context.
- Lot and process context for associating BoQ rows with the correct lot when the table itself is ambiguous.

## Outputs
- `items`: merge-safe BoQ rows with deterministic `item_key` values, explicit `continuation_of_item_key` links for split rows, and one evidence anchor per row.
- `summary`: item-count and aggregate-value totals derived from the extracted rows.
- `field_confidences`, `extraction_confidence_bps`, `warnings`, and `evidence_anchors` for daemon-side persistence and audit.
- `extraction_metadata` with `target_domain = "quantities"` and a required `extraction_scope` for `full_document` vs `chunk_merged:<plan_hash>` persistence.

## Non-goals
- Acting as the primary pricing overlay for tender pricing schedules.
- Reconstructing document layout instead of using Docling structure.
- Leaving row identity to prose summaries or free-form merge descriptions.
- Writing authoritative quantity rows directly; daemon-side persistence owns those mutations.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_SKILL_FAILED
- AGENT_RUN_FAILED
- AGENT_PARTIAL_FAILURE

## Resume Strategy
When a BoQ document is chunked, preserve stable `item_key` values across retries by deriving them from the row’s section, item reference, and normalized description rather than chunk-local position alone. If a retry reprocesses split rows, reuse the same `continuation_of_item_key` links instead of minting new identities.

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

Major claims must be anchored to observable evidence such as table row labels, repeated headers, section refs, quantity cells, rate cells, or page references in the structural extraction payload. Do not claim BoQ continuity unless the evidence section identifies the fragment anchors being merged.
