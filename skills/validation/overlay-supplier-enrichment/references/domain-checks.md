# Supplier Enrichment Overlay Domain Checks

This reference defines the domain checks used by `validation/overlay-supplier-enrichment`.

| Check | Validator Question | Reject When |
| --- | --- | --- |
| `price_plausibility` | Is the quoted price plausible for the category, quantity, and supplier context? | Price is implausibly high or low without an evidence-backed rationale. |
| `currency_correctness` | Do claimed currencies match listing evidence and conversion context? | Claimed currency conflicts with listing, locale, or FX evidence. |
| `pack_basis_consistency` | Are unit, box, case, pallet, and MOQ bases consistent across manifest and daemon evidence? | Pack basis conflicts change unit economics or margin interpretation. |
| `seller_diversity` | Does the enrichment satisfy required independent seller coverage? | Required seller diversity is missing, duplicated, or ambiguous. |
| `listing_freshness` | Are listing and scrape timestamps fresh enough for the claimed price signal? | Listing evidence is stale or timestamp provenance is missing. |
| `regulated_goods_compliance_evidence` | Are regulated-goods claims backed by certification, restriction, or compliance evidence? | Required compliance evidence is absent for restricted or regulated goods. |

The overlay may flag weird-valid cases with partial seller or freshness evidence, but it must not reject them unless one of the listed reject conditions is present.
