---
skill_id: pdf-extraction/base
skill_version: 1.0.0
interaction_profile: headless_contract
result_schema_version: 1.0.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: none
---

# pdf-extraction/base

## Use when
Use this skill when the daemon needs a domain-agnostic semantic pass over Docling-produced tender-document structure before routing to a narrower overlay or when the document role is uncertain and a generic evidence-bearing extraction is still useful.

## Do not use when
Do not use this skill for raw PDF parsing, browser navigation, authoritative database writes, pricing-only normalization that already has a dedicated overlay, or any workflow that should skip semantic extraction entirely for administrative-only documents.

## Inputs
- Docling-produced markdown for the full document or a deterministic chunk.
- Structured section and table summaries from the structural extraction stage.
- Document context such as process title, buyer, notice metadata, and any upstream role hints.
- Extraction policy context describing the target domain, scope, and expected overlay routing.

## Outputs
- `document_classification` with a non-empty resolved document role and target domain.
- `extracted_entities` containing generic semantic findings that are anchored to concrete document evidence.
- `extraction_warnings` describing ambiguity, thin evidence, or missing expected content.
- `extraction_metadata` describing scope, policy version, and source-hash lineage for daemon persistence.

## Non-goals
- Reconstructing layout that should already be present in Docling output.
- Mutating authoritative procurement tables or deciding supersession.
- Acting as a substitute for overlay-specific extraction logic such as pricing schedules or BoQs.
- Guessing classifications or entities without citing evidence anchors from the provided document content.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_SKILL_FAILED
- AGENT_RUN_FAILED
- AGENT_PARTIAL_FAILURE

## Resume Strategy
This base skill should normally run as a single-pass contract step. If the daemon retries after an interrupted run, restart from the same structural extraction payload instead of synthesizing missing state from partial output.

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

Major claims must be anchored to observable evidence such as section headings, table cells, page references, extracted entity anchors, or daemon-supplied structural metadata. Do not rely on hidden reasoning or unquoted assumptions.
