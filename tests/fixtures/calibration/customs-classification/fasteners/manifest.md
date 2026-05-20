# Fasteners Gold-Set Manifest

## Category Definition

Fixture surface: calibration-corpus.

Fasteners cover iron, steel, and aluminium articles used as screws, bolts,
nuts, rivets, washers, threaded inserts, and structural fastening components.
The validated slice focuses on HS Chapter 73 and HS Chapter 76 families where
UK10 classification, supplementary kg evidence, exporter basis, and remedy
state are decision-grade.

## Trade Remedy Profile

- Steel AD/CVD active for remedy-aware structural fastener scenarios.
- Steel fasteners under HS 7318 exercise active remedy-aware classification.
- Aluminium fasteners under HS 7616 exercise exporter-specific versus residual
  rate handling.
- Every actionable row preserves supplementary kg evidence; missing kg rows
  route to broker or review in the mutation set.

## Corpus Stats

- gold: 47 cases
- silver: 63 cases, each with two independent reviewer rationales
- bronze: 10 cases, retained for challenge/training only
- pairwise: 24 pairs
- total: 120 cases

## Mutation Coverage

- clean: broker-confirmed steel screw and bolt families with stable UK10 and kg evidence.
- weird: mixed plating, alloy, translation, and low-result-count descriptions that remain valid.
- mutated: HS6 perturbation, exporter swap, origin swap, currency swap, packaging weight perturbation, supplementary-unit removal, Incoterm swap, additional-code evidence removal, VAT relief evidence removal, and TARIC/HMRC disagreement injection.
- pairwise: 24 pairs sharing `pairId`, each changing exactly one material customs dimension.
- audit overlays: source_identity, page_kind, currency_locale, unit_basis, product_identity, warning_omission, count_mismatch, fabricated_evidence, continuation_integrity.

## Production Gate Expectation

The corpus is constructed so the accuracy harness reports:

- `production_gate: eligible`
- `review_trigger_recall_bps: 10000`
- zero catastrophic understatement events
- zero unsafe auto-classification events
- rule-of-three upper bounds at or below 300 bps

The fasteners slice contributes the fasteners portion of
`L5-T-customs_classification-027` and feeds the evidence-tiered accuracy tests
`L5-T-customs_classification-028..032,035`.
