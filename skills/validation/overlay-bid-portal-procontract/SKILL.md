---
skill_id: validation/overlay-bid-portal-procontract
skill_version: 0.1.0
interaction_profile: headless_contract
result_schema_version: 1.0
checkpoint_schema_version: 1.0
validator_overlay_version: 0.1.0
validator_overlay_compatible_versions:
  worker_behavior_versions:
    - bid-portal/procontract@0.1.0+1.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: read_only
validation_policy: always
promotion_stage: draft
worker_skill_id: bid-portal/procontract
worker_behavior_version: bid-portal/procontract@0.1.0+1.0
validator_behavior_version: validation/overlay-bid-portal-procontract@0.1.0+1.0
version_change_affects_behavior: true
gate_a_family_key: bid-portal-procontract
gate_a_family_prequalified: false
gate_b_exact_slice_key: bid-portal-procontract::procurement::bid-portal/procontract@0.1.0+1.0::0.1.0
gate_b_exact_slice_ready: false
rubric_version: bid-portal-procontract-rubric@1.0
evidence_inputs:
  - run_manifest
  - daemon_evidence_appendix
  - portal_screenshots
  - bid_scope_payload
  - validator_rubric_yaml
---

# validation/overlay-bid-portal-procontract

## Use when
Use this overlay when validating a `bid-portal/procontract` worker manifest for an approved L6 bid workflow. Apply it after the generic validator has checked manifest structure, daemon appendix integrity, and schema conformance.

## Do not use when
Do not use this overlay to operate Procontract, refresh authentication, discover opportunities, mutate bid workflow state, or inspect live browser state. It is read-only and must evaluate only the supplied evidence bundle.

## Inputs
- Worker run manifest for `bid-portal/procontract@0.1.0+1.0`.
- Daemon evidence appendix containing the bid scope summary, side-effect progression, screenshot inventory, result summary, and override metadata.
- `BidScopePayload` with `processId`, `lotIds`, `candidateIds`, `selectedQuoteIds`, `portalOpportunityRef`, `decisionBasisHash`, and `sideEffectCap`.
- Portal screenshots and validator rubric evidence references supplied by the daemon.

## Outputs
- A validator verdict payload matching `result.schema.json`.
- Domain issues that cite manifest, screenshot, or daemon appendix evidence references.
- A verdict of `approved`, `rejected`, `flagged`, or `outage`; `rejected`, `flagged`, and `outage` block `bid_submitted`.

## Non-goals
Do not reinterpret operator decisions, override L5 readiness blockers, repair missing screenshots, infer hidden portal state, or copy raw `BidScopePayload` or company-profile JSON into output.

## Domain Checks

### Assertion 1 - Opportunity reference match
The run manifest must cite `BidScopePayload.portalOpportunityRef`, and `daemon_evidence_appendix.bid_scope_summary.portal_opportunity_ref` must carry the same reference or its recorded stable hash. Any mismatch rejects the run with `validator_rubric_ref: opportunity_ref_mismatch`.

### Assertion 2 - Process, lot, candidate, quote, and profile scope match
The manifest narrative and daemon appendix must cite the exact `processId`, every `lotId`, every `candidateId`, every `selectedQuoteId`, the `companyProfileVersion`, and the `decisionBasisHash` recorded at run start. Missing or extra scope IDs reject with `validator_rubric_ref: scope_mismatch`.

### Assertion 3 - Screenshot completeness
The screenshot inventory must prove each observed playbook checkpoint. A full `draft_saved` or `bid_submitted` run needs at least 11 screenshots: steps 1-9, either step-10a or step-10b, and step-11. A continuation from `draft_saved` must include at least step-10b and step-11. Missing evidence rejects with `validator_rubric_ref: screenshot_incomplete`.

### Assertion 4 - Side-effect progression and cap
`daemon_evidence_appendix.side_effect_progression` must be monotonic for mutating steps, allowing interleaved `READ_ONLY` capture-only steps. The highest observed class must be less than or equal to `BidScopePayload.sideEffectCap`. Violations reject with `validator_rubric_ref: side_effect_non_monotonic` or `side_effect_above_cap`.

### Assertion 5 - Confirmation or draft evidence shape
If the run reached `bid_submitted`, `SubmissionResult.confirmation_ref` must be a non-empty portal reference with stable alphanumeric or separator characters and must not conflict with the scoped opportunity. If the run reached `draft_saved`, `DraftSaveResult.portal_draft_ref` must be present. Malformed or missing evidence rejects with `validator_rubric_ref: confirmation_ref_malformed`.

### Assertion 6 - Citation completeness
The agent narrative must cite lot IDs, candidate IDs, selected quote IDs, `companyProfileVersion`, and each `operator_override.id` and `operator_override.kind` used by the run. Missing citations reject with `validator_rubric_ref: citation_missing`.

### Assertion 7 - PII redaction
The validator output, worker narrative, and daemon evidence appendix must not expose company-profile sentinels, raw `BidScopePayload` JSON, local upload paths, screenshot paths, portal artifact paths, contact names, company registration numbers, VAT numbers, addresses, policy numbers, or monetary cover amounts. Any leakage rejects with `validator_rubric_ref: pii_leakage`.

### Assertion 8 - Override anomaly flagging
If override metadata is present, `operator_override.kind` must be one of `quote_role_floor_only` or `validation_gate`. Unknown kinds reject. Known kinds with short, reused, stale, malformed, or weakly cited rationales are `flagged` with `validator_rubric_ref: override_anomaly` instead of rejected when assertions 1-7 otherwise pass.

### Assertion 9 - Clean-run approval
Return `approved` only when assertions 1-7 pass and assertion 8 produces no flag. Return `rejected` for evidence contradictions, `flagged` for override anomalies requiring operator review, and `outage` when validator evidence is unavailable or unreadable.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- BID_READINESS_GATE_FAILED
- BID_SIDE_EFFECT_CAP_EXCEEDED
- BID_SUBMISSION_FAILED
- BID_PORTAL_CHANGED
- VALIDATOR_OUTAGE

## Resume Strategy
Resume from the supplied manifest and daemon evidence appendix. If an evidence reference is missing, emit a rejected or flagged validator issue; do not open the portal, replay browser actions, or infer missing screenshots from operator memory.

## Canary
Run canaries for a clean draft save, a clean formal submission, a scope-mismatched manifest, a screenshot-deficient manifest, a side-effect cap breach, a malformed confirmation reference, a citation gap, a PII leak, and an override anomaly.

## Run Manifest Instructions
Return only the structured validator result. Anchor every issue to evidence references from the run manifest, portal screenshot inventory, daemon appendix, bid scope summary, or validator rubric. Required manifest context tokens include `manifest_version`, `skill_id`, `skill_version`, `job_id`, `session_id`, `execution_mode`, `Summary`, `Steps Taken`, `Decisions Made`, `Warnings`, and `Evidence`. Do not rely on hidden browser state or operator memory.
