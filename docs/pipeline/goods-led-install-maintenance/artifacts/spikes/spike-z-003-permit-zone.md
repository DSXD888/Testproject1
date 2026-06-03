# SPIKE-Z-003 - Per-Authority/Per-Zone Permit and Lane-Rental Model

## Scope

This spike covers `FR-Z.3`: refining Component A3 from broad banded permit and traffic-management ranges into detailed authority- and zone-specific fee modelling.

The tracked TDD coverage is `SPIKE-Z-003`, not a code test. This artifact records the research decision and the future extension point. It does not implement production behavior and does not change Tier-A row semantics.

## Current Tier-A Baseline

Tier-A already ships the basic permit/TM surface:

- `reference/uk-permit-tm-bands-2026-q2.json` is the skill-local reference table.
- `PermitTrafficMgmtLineItem` rows stay separate from labour and access-equipment rows.
- `permit_responsibility` is one of `authority_provided`, `contractor_provided`, `shared`, or `unknown`.
- `authority_provided` prices contractor exposure at zero.
- `contractor_provided` prices the selected band at full low/base/high.
- `shared` uses split/conservative multipliers from the reference rule.
- `unknown` keeps conservative base/high exposure, may use zero low, and emits `PERMIT_RESPONSIBILITY_UNKNOWN`.
- Lane-rental-sensitive locations emit `LANE_RENTAL_ZONE`.
- TTRO exposure emits `TTRO_LEAD_TIME_RISK`.

The existing banded table is still correct for screening. The Tier-B scope is only the refinement from broad bands to authority/zone-specific fees where reliable source data exists.

## Research Findings

The research question is whether highway/high-street jobs should move beyond a flat permit/TM band. They should, but only as a layered refinement.

Public-realm jobs can involve several distinct costs: TTRO or temporary traffic order, street-works notice or permit, lane rental, parking-bay suspension, highway or scaffold licence, and physical traffic-management crew/equipment. These costs are not interchangeable, and they must not be folded into labour.

Authority-specific fee data is operationally useful but fragmented. The right model is not a live scrape per estimate. It is a versioned reference dataset with effective dates, source URLs, source authority, and confidence. The resolver should use exact authority/zone matches only when the site location and scheme are sufficiently clear; otherwise it must fall back to the Tier-A banded model.

`permit_responsibility` remains the first-order uncertainty. Per-zone fee precision is not trustworthy if the contract does not say who pays. Unknown responsibility should continue to route review and carry conservative high-side exposure.

## Decision

Adopt a layered authority/zone reference model for future implementation:

1. Keep `uk-permit-tm-bands-*` as the stable fallback.
2. Add optional authority/zone-specific profiles inside the same reference family or a clearly linked companion file.
3. Resolve exact authority/zone fees only when postcode/site evidence and scheme metadata are high-confidence.
4. Apply the existing `permit_responsibility` pricing policy after selecting the authority/zone fee components.
5. Persist outputs as `PermitTrafficMgmtLineItem` rows, never as labour or access-equipment costs.

This decision preserves Tier-A behavior while allowing future high-confidence refinements for zones such as London lane-rental exposure, Kent lane rental, parking suspension schemes, and authority-specific TTRO/street-works fees.

### Rationale

- The current banded model is appropriate for fast screening and incomplete site data.
- Exact fees are valuable only when location, authority, scheme, and responsibility are known.
- A versioned reference table is auditable and testable; live fee scraping per estimate is brittle and slow.
- Keeping permit/TM as separate line items protects the goods-share denominator and operator-facing explanations.
- Conservative fallback behavior prevents false precision when the authority boundary or payer responsibility is unresolved.

### Rejected Alternatives

1. Replace Tier-A bands with authority-specific fees everywhere.
   Rejected because many notices lack enough site/authority detail at triage, and precision would be false.

2. Live-scrape each authority website during landed-cost estimation.
   Rejected because authority pages and fee schedules change shape, scraping adds latency, and failures would add noise to a screening-stage cost.

3. Fold permit, traffic-management, and lane-rental pence into labour.
   Rejected because the shipped contract requires separate `PermitTrafficMgmtLineItem` rows and separate operator visibility.

4. Assume the authority pays when the contracting authority is a council or highway authority.
   Rejected because the tender decides responsibility. The default remains `unknown` until evidence resolves it.

5. Build a complete UK authority fee map before using the model.
   Rejected because a sparse, source-backed table with fallback bands gives better near-term value and avoids stale all-authority coverage.

## Reference Data Extension

Future reference data should keep the existing band table and add authority/zone refinements with explicit provenance. The shape below is design-level, not a current runtime contract:

```json
{
  "schema_version": "2.0.0",
  "reference_kind": "uk_permit_tm_bands",
  "rate_table_version": "uk-installation-labour-2026-q3",
  "money_unit": "pence",
  "ratio_unit": "bps",
  "authority_zone_profiles": [
    {
      "profile_id": "tfl_red_route_public_realm_2026_q3",
      "authority": "Transport for London",
      "region": "London",
      "zone_type": "lane_rental",
      "zone_ids": ["tfl_red_route"],
      "match_evidence": {
        "postcode_prefixes": ["E", "EC", "N", "NW", "SE", "SW", "W", "WC"],
        "requires_lane_rental_zone": true,
        "requires_public_realm": true
      },
      "components": [
        {
          "component": "lane_rental",
          "basis": "authority_zone_fee",
          "low_gbp_pence": 50000,
          "base_gbp_pence": 120000,
          "high_gbp_pence": 250000,
          "confidence_bps": 7200,
          "source_url": "https://example.invalid/authority-fee-schedule",
          "effective_from": "2026-07-01",
          "effective_to": "2026-09-30"
        }
      ],
      "fallback_band_id": "london_lane_rental_high_street",
      "warning_codes": ["LANE_RENTAL_ZONE", "TTRO_LEAD_TIME_RISK"]
    }
  ]
}
```

Required properties:

- Every fee component carries source URL, source authority, effective date window, and confidence in bps.
- Values remain integer GBP pence.
- The existing `component_catalogue`, `permit_bands`, `lane_rental_zones`, and responsibility policy remain available as fallback.
- Authority/zone rows must identify the fallback band they refine.
- The reference table version changes when source fee data changes.

## Resolver Design

Future `PermitTrafficMgmtLineItem` resolution should follow this order:

1. Build a permit context from `InstallationRequirement`: postcode or region, `public_realm`, `site_access`, `lane_rental_zone`, `permit_responsibility`, and relevant extracted tender text.
2. Normalize permit responsibility. Missing or ambiguous values remain `unknown`.
3. Try an exact authority/zone profile match using deterministic evidence such as postcode, known authority, zone flag, public-realm flag, and site-access terms.
4. If no exact profile matches, use the existing Tier-A permit band.
5. Select component line items: TTRO, street-works notice/permit, lane rental, parking suspension, highway/scaffold licence, and physical traffic management.
6. Apply `permit_responsibility` multipliers after selecting the components.
7. Emit `PermitTrafficMgmtLineItem` rows with `component`, `responsibility`, `low_gbp_pence`, `base_gbp_pence`, `high_gbp_pence`, `basis`, source/provenance, and warning codes.

The resolver should treat exact profiles as confidence improvements, not automatic pass signals. Unknown responsibility, authority-boundary ambiguity, stale reference rows, or conflicting tender evidence should still route review.

## Basis and Warnings

Recommended future `basis` values:

- `tier_a_band`: existing broad band.
- `authority_zone_fee`: exact source-backed authority or lane-rental profile.
- `authority_zone_fee_with_band_fallback`: partial authority data plus banded high-side fallback.
- `operator_reviewed_authority_zone`: operator-confirmed authority/zone evidence.

Recommended future warning additions, once promoted to implementation scope:

- `PERMIT_AUTHORITY_UNRESOLVED`: postcode/site text does not identify the authority confidently.
- `PERMIT_ZONE_FEE_STALE`: authority/zone fee row is outside its effective date window.
- `PERMIT_ZONE_PARTIAL_COVERAGE`: some components used exact fees while others used fallback bands.

The existing `PERMIT_RESPONSIBILITY_UNKNOWN`, `LANE_RENTAL_ZONE`, and `TTRO_LEAD_TIME_RISK` warnings remain unchanged.

## Extension Points

Future implementation should add the refinement without changing Tier-A row meaning:

1. Extend `uk-permit-tm-bands-*` reference data, or add a linked `uk-permit-tm-authority-zones-*` file under the same skill-local reference directory.
2. Keep the existing banded resolver as fallback.
3. Add an authority/zone matcher before band selection.
4. Carry provenance and effective-date metadata through `PermitTrafficMgmtLineItem` evidence.
5. Preserve the existing responsibility multipliers and warnings.
6. Include authority/zone basis and stale/partial-coverage warnings in validation overlay and manifest surfacing once the spike becomes implementation scope.

## Successor Coverage

`SPIKE-Z-003` remains the only current coverage for `FR-Z.3`. When this spike is promoted into implementation scope, the spec/TDD should replace it with concrete requirement IDs and tests such as:

- Exact London lane-rental profile selects authority/zone fees and keeps permit/TM separate from labour.
- Unknown responsibility still emits `PERMIT_RESPONSIBILITY_UNKNOWN` after exact zone matching.
- Stale authority/zone fee rows fall back or route review.
- Ambiguous authority boundary uses the Tier-A band and emits authority-unresolved review evidence.
- Partial authority coverage combines exact known components with band fallback and marks the basis accordingly.
- All money remains integer pence and confidence remains bps.

## Non-Delivery Confirmation

This spike does not add code, migrations, tests, or runtime behavior. It records the accepted design direction for `FR-Z.3` and preserves the shipped Tier-A banded resolver, `permit_responsibility`, and `PermitTrafficMgmtLineItem` semantics.
