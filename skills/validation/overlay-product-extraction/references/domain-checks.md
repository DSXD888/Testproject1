# Product Extraction Overlay Domain Checks

This reference defines the domain checks used by `validation/overlay-product-extraction`.

| Check | Validator Question | Reject When |
| --- | --- | --- |
| `candidate_type_plausibility` | Is each extracted product candidate plausible for the CPV family and notice scope? | The candidate type conflicts with CPV family, lot scope, or source document evidence. |
| `archetype_assignment_consistency` | Does each candidate use an archetype consistent with evidence and downstream pricing logic? | The archetype assignment conflicts with required pricing, quantity, or sourcing behavior. |
| `slot_completeness` | Are required archetype slots populated or explicitly unavailable with evidence? | A required slot is missing without an evidence-backed unavailable state. |
| `quantity_unit_plausibility` | Are quantities, units, pack sizes, and basis conversions internally consistent? | Unit or quantity evidence changes order economics, pack interpretation, or MOQ handling. |
| `bundle_decomposition_correctness` | Are bundles, kits, lots, and mixed sets decomposed without duplication or loss? | Quoteable items are duplicated, dropped, or misallocated between bundle components. |

The overlay may flag weird-valid cases with partial optional slots or unusual units, but it must not reject them unless one of the listed reject conditions is present.
