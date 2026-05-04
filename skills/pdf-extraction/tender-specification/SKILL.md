---
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
interaction_profile: headless_contract
result_schema_version: 1.0.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: none
---

# pdf-extraction/tender-specification

## Use when
Use this skill for tender specifications, statements of requirements, invitations to tender, and mixed tender packs whose resolved overlay set includes `pdf-extraction/tender-specification`. The goal is one evidence-bearing payload that identifies lot structure, product-requirement seeds, evaluation criteria, CPV references, and explicit technical standards from Docling-produced structure.

## Do not use when
Do not use this skill for pricing schedules, bills of quantities, raw PDF parsing, browser automation, or administrative-only documents that should skip semantic extraction. Do not use it to mutate authoritative rows directly; daemon-side persistence owns those writes.

## Inputs
- Docling markdown for the full document or a chunk-scoped extraction packet.
- `document_metadata` including resolved role hints, extraction policy version, and document identity fields.
- `process_context` including buyer, notice title, and any CPV signals already present on the process.
- Existing notice-level lot metadata for cross-reference only; do not treat pre-existing lot rows as authoritative truth if the tender document contradicts them.

## Outputs
- `lots`: one authoritative array describing lot titles, descriptions, requirements, evaluation criteria, CPV codes, product requirements, technical standards, and one primary evidence anchor per lot.
- `product_requirement_seeds`: candidate-seed records that the daemon can translate into `product_candidates` writes while keeping `document_semantic_extractions.target_domain = "lots"`.
- `field_confidences`, `extraction_confidence_bps`, `warnings`, and `evidence_anchors` for daemon-side persistence and audit.
- `extraction_metadata` with `target_domain = "lots"` and a required `extraction_scope` so the daemon can distinguish `full_document` from `chunk_merged:<plan_hash>`.

## CRITICAL: Result Contract

You MUST write your output as a valid JSON file named `result.json` in the current working directory. The JSON must match `result.schema.json` exactly. Do NOT output a narrative text response. Do NOT explain your analysis in prose. Write ONLY the structured JSON result file.

```bash
# Write result.json to the current working directory
cat > result.json << 'RESULT_EOF'
{
  "lots": [...],
  "product_requirement_seeds": [...],
  "field_confidences": {...},
  "extraction_confidence_bps": 7500,
  "warnings": [...],
  "evidence_anchors": [...],
  "extraction_metadata": {
    "target_domain": "lots",
    "extraction_scope": "full_document",
    "source_markdown_hash": "...",
    "policy_version": "2026.04.12"
  }
}
RESULT_EOF
```

The daemon reads this file after the session completes. If `result.json` is missing or not valid JSON matching the schema, the job is marked as failed.

## Non-goals
- Parsing layout or table structure that should already be present in Docling output.
- Emitting separate semantic rows for `"candidates"` or `"requirements"` target domains.
- Performing pricing normalization or full bill-of-quantities costing.
- Ignoring explicit technical quantity requirements. When the tender text or
  schedule states counts such as `"2no boilers"`, `"1No. cylinder"`, or
  `"3 units"`, extract them as evidence-backed `quantity_hint` values and,
  when allocation is unambiguous, structured `quantity` / `quantity_unit`
  fields on the matching product requirement seed.
- Guessing lot boundaries, standards, or product requirements without citing evidence anchors.

## Quantity Extraction Rules
- Do extract explicit product counts that describe what must be supplied or
  installed. These are technical requirement signals, not pricing
  normalization.
- Do not extract money, unit rates, extensions, or BoQ pricing columns as
  quantities.
- If a total count maps cleanly to unique schedule references, allocate it
  deterministically. Example: a boiler schedule has B01 and B02, and prose says
  `2no gas fired heating boilers`; emit `quantity = 1`,
  `quantity_unit = "unit"` for each of B01 and B02.
- If the total count exists but candidate-level allocation is ambiguous, keep
  `quantity` absent, include an honest `quantity_hint`, and add a warning with
  evidence references rather than inventing per-candidate quantities.
- Quantity evidence must cite both the schedule/reference row and the explicit
  quantity sentence when both are needed to justify allocation.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_SKILL_FAILED
- AGENT_RUN_FAILED
- AGENT_PARTIAL_FAILURE

## Resume Strategy
Prefer a single-pass run when the tender-spec payload fits within the overlay budget. If the daemon retries a chunked or interrupted run, resume from the same structural extraction packet and preserve stable lot numbering, seed keys, and evidence anchors instead of re-inventing identifiers from partial output.

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

Major claims must be anchored to observable evidence such as lot headings, schedule rows, standards references, section headings, or page references present in the structural extraction payload. Do not claim a lot split, product seed, or standard reference unless the evidence section cites the relevant anchor.
