# Packaging-Cartons Gold-Set Manifest

fixture_surface: "calibration-corpus"

## Category Definition

Packaging-cartons covers the validated easy slice for paper and paperboard cartons, boxes, cases, sleeves, and mailers under chapter 48. The primary selected family is HS 4819, with HS 4823 retained as a plausible paper-article alternative and HS 3923 retained only as a rejected material-mismatch branch for plastic conveyance articles.

## Explicit Exclusions

The following dangerous packaging subfamilies are outside this gold-set slice and route to broker/manual review until a separate gold-set slice exists:

- aluminium_foil_packaging
- glass_packaging
- tin_mill_packaging
- metal_cans
- composite_packaging

## Corpus Stats

- gold: 40 cases
- silver: 40 cases with two-reviewer reasoning
- bronze: 40 cases for challenge/training only
- pairwise: 12 pairs sharing `pair_id` and changing exactly one material evidence dimension

## Mutation Coverage

- clean: stable paperboard carton and corrugated mailer cases
- weird: ECT, FEFCO, FSC, and postal-format variants that remain valid
- mutated: HS6 perturbation, material perturbation, currency swap, supplementary-unit removal, Incoterm swap
- pairwise: material, wall grade, format, dimension, recycled-content, and supplier-origin dimensions

## Tariff And VAT Profile

- primary UK10 families: 4819100000, 4819200000
- alternate paper-article family: 4823908580
- rejected family: 3923 plastic articles for conveyance
- trade remedy active: false
- VAT treatment: standard rate unless a future documentary relief path is proven
