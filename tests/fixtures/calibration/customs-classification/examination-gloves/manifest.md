# Examination Gloves Gold-Set Manifest

## Category Definition

Medical examination gloves used for clinical, laboratory, dental, and care settings. The calibration slice covers rubber medical gloves under HS Chapter 40, plus non-medical alternative branches in Chapters 61 and 42 for branch-divergence and material-swap challenges.

Fixture surface: calibration-corpus.

## Regulatory Profile

- Primary headings: 4015110000 and 4015190010.
- Alternative branches: 6116102000 for coated knitted gloves and 4203291000 for leather gloves.
- Medical device status: Class I MHRA evidence is required before VAT relief or medical-use treatment is accepted.
- Sterility is material: sterile surgical-use evidence and non-sterile examination-use evidence are scored separately.
- Supplementary unit evidence is pairs.

## Corpus Stats

- gold: 20 cases with stable public evidence references.
- silver: 64 cases with two independent reviewer rationales per case.
- bronze: 36 challenge-generation cases; these cannot certify production without the gold/silver cases.
- pairwise: 12 pairs, 24 cases total, each pair changes exactly one material evidence dimension.
- total: 120 cases, which keeps the zero-event rule-of-three upper bound at 250 bps for an eligible scoreCategory run.

## Mutation Coverage

- clean: stable latex, nitrile, vinyl, and sterile rubber glove cases.
- weird: ambiguous tender descriptions, low-evidence marketplace wording, language variants, and non-sterile/sterile ambiguity.
- mutated: HS6 perturbation, sterility evidence removal, material swap, medical-device class swap, VAT relief evidence removal, origin swap, supplementary-unit removal, currency swap, and Incoterm swap.
- pairwise: 12 controlled pairs covering medical-use evidence, sterility, material, chapter branch, VAT relief, supplementary units, origin, currency, Incoterm, additional-code evidence, compliance evidence, and description quality.

## Production Gate Expectations

All 120 scoring cases use conservative outputs: auto-pass only when medical-use and supplementary-unit evidence are present, review-required for incomplete evidence, and broker-required where material or sterility uncertainty would make the classification unsafe. The expected scoreCategory result is `production_gate: eligible`.
