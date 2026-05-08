---
skill_id: validation/overlay-document-acquisition
skill_version: 2026.05.06-1
interaction_profile: headless_contract
result_schema_version: 1.0
checkpoint_schema_version: 1.0
validator_overlay_version: 2026.05.06-1
validator_overlay_compatible_versions:
  worker_behavior_versions:
    - document-acquisition/*@2026.05.06-1+1.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: read_only
promotion_stage: candidate
worker_behavior_version: document-acquisition/*@2026.05.06-1+1.0
validator_behavior_version: validation/overlay-document-acquisition@2026.05.06-1+1.0
version_change_affects_behavior: true
gate_a_family_key: document-acquisition
gate_a_family_prequalified: false
gate_b_exact_slice_key: document-acquisition::procurement-documents::document-acquisition/*@2026.05.06-1+1.0::2026.05.06-1
gate_b_exact_slice_ready: false
---

# validation/overlay-document-acquisition

## Use when
Use this overlay when validating document-acquisition worker manifests that download, classify, and inventory procurement notice attachments. Apply it after the generic validator has checked manifest structure and daemon appendix consistency.

## Do not use when
Do not use this overlay for procurement notice ingestion, supplier enrichment, product extraction, margin scoring, or bid workflow decisions. Do not mutate pipeline state or attempt live document downloads.

## Inputs
- The worker run manifest and daemon evidence appendix for a document-acquisition job.
- Document inventory rows, attachment metadata, MIME or content-type observations, byte counts, content hashes, and download status evidence supplied by the daemon.
- Calibration provenance tags identifying clean-real, weird-valid, mutated-bad, and pairwise canary cases.

## Outputs
- A validator verdict payload matching `result.schema.json`.
- Issue entries that use the document-acquisition domain categories and cite manifest or daemon appendix evidence refs.
- A low-basis or rejected verdict when document evidence is insufficient for a confident domain judgment.

## Domain Checks
- `document_type_identification`: acquired files are classified as the expected procurement document types from evidence, not filename guesswork alone.
- `download_completeness`: every required attachment is downloaded or has an explicit, evidence-backed unavailable status.
- `content_type_verification`: MIME type, extension, and lightweight content observations agree enough to trust parsing decisions.
- `file_size_plausibility`: byte counts are plausible for the claimed document type and not empty, truncated, or unexpectedly huge without rationale.
- `document_set_hash_consistency`: recorded hashes and document-set summaries are stable across manifest and daemon evidence.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_SKILL_FAILED
- DOCUMENT_DOWNLOAD_INCOMPLETE
- SOURCE_CHANGED
- DOCUMENT_EVIDENCE_INSUFFICIENT

## Resume Strategy
Resume from the same manifest and daemon evidence appendix. If document type, download, content-type, file-size, or hash evidence is missing, emit a validator issue instead of fetching live data.

## Canary
Run four canaries: clean-real approval, weird-valid non-rejection, mutated-bad rejection, and one pairwise clean/corrupted comparison over download completeness and document-set hash evidence.

## Run Manifest Instructions
Return only the structured validator result. Anchor every domain issue to evidence references from the worker manifest, document inventory, or daemon appendix. Do not rely on hidden browser state or operator memory.
