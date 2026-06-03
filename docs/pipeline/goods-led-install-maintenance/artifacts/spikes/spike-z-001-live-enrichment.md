# SPIKE-Z-001 - Live Per-Estimate Enrichment Lane

## Scope

This spike covers `FR-Z.1`: an operator-triggered high-value-bid enrichment lane that can rerun or augment `run_landed_cost_live_loop` for one landed-cost estimate using fresher labour, access-equipment, permit, and traffic-management evidence.

The tracked TDD coverage is `SPIKE-Z-001`, not a code test. This artifact records the research decision and the future extension point. It does not implement production behavior and does not change Tier-A row semantics.

## Current Tier-A Baseline

The shipped Tier-A path is still the authoritative baseline:

- `run_landed_cost_live_loop` runs Stage 11 landed-cost work from daemon `job_parameters`.
- The UK installation labour skill is transient-loaded as `landed_cost_uk_installation_labour`.
- Labour runs after FX and before landed-cost synthesis.
- The labour skill receives `installation_requirement`, destination postcode, landed goods cost in pence, and `rate_table_version`.
- The skill is table-backed and fail-soft. Blocked labour results lower confidence and route review rather than crashing the pipeline.

The baseline intentionally avoids live per-estimate scraping. Research recommends curated tables for normal triage and a light live/manual lane only for volatile access-equipment checks or high-value bids where an operator wants extra assurance.

## Research Findings

The open research questions behind `FR-Z.1` are live/manual enrichment and whether to derive `at_height_m` or MEWP class from tender attachments.

Access-equipment hire is the strongest candidate for a live/manual enrichment lane. Short-hire equipment costs can move materially between rate-table refreshes, and high-street or public-realm work can require operated equipment plus permits or traffic management. These inputs can swing the non-goods cost enough to alter review routing.

Permit responsibility is not reliably knowable from first-pass notice text. Tier-A correctly treats unknown responsibility conservatively and raises a warning. A future enrichment lane can accept operator-confirmed or document-extracted responsibility, but it must preserve provenance and keep unresolved conflicts in review.

Automatic derivation of `at_height_m` and MEWP class from drawings is not ready for Tier-A. Drawings and attachments need a document-extraction upgrade before they can be trusted as automatic cost drivers. The future lane should accept attachment-derived facts only when the extraction step produces explicit evidence references and confidence, or when an operator supplies the fact manually.

## Decision

Adopt an operator-triggered rerun/augment extension point for high-value bid review. Do not live-scrape every Stage 11 estimate, and do not replace the existing Tier-A estimate silently.

The enriched run creates a new estimate snapshot for the same process, candidate, and quote with a distinct enrichment basis marker. The previous Tier-A estimate remains available for comparison. An operator may promote the enriched estimate to authoritative only through an explicit future operator action.

### Rationale

- It keeps normal Stage 11 fast, deterministic, and table-backed.
- It focuses live/manual work on the few estimates where the cost swing matters.
- It preserves auditability by separating the original basis from the enriched basis.
- It lets attachment-derived height/equipment facts enter the system with provenance instead of becoming untraceable overrides.
- It keeps all cost values in integer pence and confidence/ratio values in basis points.

### Rejected Alternatives

1. Always live-scrape labour and equipment on every landed-cost estimate.
   Rejected because labour rates move slowly, live sources are noisy, and triage would become slower and less reliable.

2. Replace the Tier-A landed-cost estimate in place.
   Rejected because it would obscure the original decision basis and make audit/hash drift harder to explain.

3. Auto-derive `at_height_m` and MEWP class from attachments in this spike.
   Rejected because attachment extraction is a separate capability and needs its own tests before it can drive costs automatically.

4. Treat this research spike as an implementation bead.
   Rejected because spec Section 8 explicitly requires a research/design output before full acceptance criteria are invented.

## Rerun Contract

Future implementation should accept an enriched `job_parameters` envelope that targets exactly one estimate and carries a distinct basis marker. The shape below is design-level, not a current runtime contract:

```json
{
  "enrichment_mode": "operator_live_enrichment",
  "enrichment_request_id": "enr_2026_06_03_001",
  "enrichment_basis_marker": "live-enrichment:enr_2026_06_03_001:evidence_sha256",
  "target": {
    "process_id": "proc_123",
    "product_candidate_id": "cand_123",
    "selected_quote_id": "quote_123",
    "prior_landed_cost_estimate_id": "lce_123"
  },
  "override_policy": "augment_unresolved_only",
  "evidence": {
    "operator_requested_at": "2026-06-03T00:00:00Z",
    "source_documents": [
      {
        "document_id": "doc_123",
        "attachment_id": "att_456",
        "evidence_type": "drawing_or_spec_extract",
        "confidence_bps": 8200
      }
    ],
    "manual_facts": {
      "at_height_m": 8,
      "mewp_class": "operated_boom",
      "permit_responsibility": "contractor_provided",
      "public_realm": true
    },
    "fresh_quotes": {
      "access_equipment_hire": [
        {
          "supplier_name": "example_access_hire_supplier",
          "quote_date": "2026-06-03",
          "hire_cost_gbp_pence": 55000,
          "operator_cost_gbp_pence": 25000,
          "vat_included": false,
          "source_url": "https://example.invalid/access-hire"
        }
      ]
    }
  }
}
```

Required semantics:

- `enrichment_mode = "operator_live_enrichment"` activates the future single-estimate lane.
- `target` constrains the live loop to one process/candidate/quote/previous-estimate tuple. It must not process the whole Stage 11 batch.
- `enrichment_basis_marker` becomes part of the future decision basis input, so an enriched result has a distinct `decision_basis_hash` even when the point costs match the Tier-A estimate.
- `evidence` is additive. It can supply manual facts, attachment-derived facts, or fresh access-equipment evidence, but each fact must retain provenance.
- Money fields remain integer GBP pence. Confidence remains basis points.

## Augment vs Replace

The default future policy should be `augment_unresolved_only`.

Under `augment_unresolved_only`, the enriched run may resolve Tier-A uncertainty such as unknown permit responsibility, assumed height/equipment class, or stale equipment-rate evidence. It produces a comparison snapshot and may improve confidence, but it does not silently overwrite the existing authoritative estimate.

A second policy, `operator_replace_authoritative`, can exist only as an explicit operator action after the enriched snapshot is reviewed. Promotion must write a new basis marker/hash and leave the previous estimate auditable.

A read-only `shadow_compare` mode is also useful for future QA: run the enriched input, calculate deltas, and surface review reasons without changing the authoritative path.

## InstallationRequirement Overlay Rules

The future lane should build from the existing `InstallationRequirement` rather than replace it wholesale.

- If a field is missing or assumed in Tier-A, enriched evidence may fill it.
- If enriched evidence conflicts with an extracted field, keep the estimate in review and require an operator choice.
- `permit_responsibility` can move from `unknown` to `authority_provided`, `contractor_provided`, or `shared` only with evidence provenance.
- `at_height_m`, `site_access`, `equipment_hint`, `public_realm`, and `lane_rental_zone` are eligible enrichment fields because they drive MEWP and permit/TM costs.
- Maintenance remains opex and outside the capital-event dominance denominator.

## Extension Points

Future implementation should add the extension without changing the Tier-A row meaning:

1. Add an operator action that creates an enrichment request for one existing landed-cost estimate.
2. Pass `enrichment_mode`, `target`, `enrichment_basis_marker`, and `evidence` through `job_parameters`.
3. Teach `run_landed_cost_live_loop` to detect the enrichment mode and constrain assessment loading to the target tuple.
4. Overlay enriched `InstallationRequirement` facts before building `uk_installation_labour_job_parameters`.
5. Include the enrichment marker and evidence hash in the decision-basis hash input.
6. Persist the enriched estimate as a separate snapshot/comparison record, then require explicit operator promotion before replacing the authoritative view.
7. Surface the marker, evidence provenance, and delta from the prior Tier-A estimate in the manifest/desktop review UI.

## Successor Coverage

`SPIKE-Z-001` remains the only current coverage for `FR-Z.1`. When this spike is promoted into implementation scope, the spec/TDD should replace it with concrete requirement IDs and tests such as:

- Targeted rerun processes exactly one estimate.
- Distinct `enrichment_basis_marker` changes the decision-basis hash.
- Enriched evidence can augment unresolved Tier-A fields without mutating the prior estimate.
- Conflicting attachment/operator facts route review.
- Explicit operator promotion is required before an enriched estimate becomes authoritative.
- The labour skill remains `http_public`; no CDP/browser requirement is introduced by enrichment.

## Non-Delivery Confirmation

This spike does not add code, migrations, tests, or runtime behavior. It records the accepted design direction for `FR-Z.1` and preserves the shipped Tier-A semantics.
