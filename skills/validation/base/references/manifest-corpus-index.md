# Manifest Corpus Index

This index tracks the validator-base calibration and canary corpus provenance.

## Corpora

| Corpus | Purpose | Provenance Source | Initial Target |
| --- | --- | --- | --- |
| `clean-real` | Baseline approval behavior on valid manifests | Live manifests from successful runs | 25-30 cases |
| `weird-but-valid` | Protect against false positives on unusual but acceptable runs | Gold-set edge cases and adjudicated oddities | 15-20 cases |
| `mutated-bad` | Ensure corrupted manifests are caught consistently | Synthetic mutations of real manifests | 30-50 cases |

## Indexed Cases

| Case ID | Corpus | Manifest Path | Provenance | Notes |
| --- | --- | --- | --- | --- |
| `clean-real-001` | `clean-real` | `artifacts/corpus/clean-real/manifest-001.md` | live-manifest | Complete manifest with strong daemon appendix evidence. |
| `weird-but-valid-001` | `weird-but-valid` | `artifacts/corpus/weird-but-valid/manifest-001.md` | gold-set-edge-case | Unusual narrative/result shape that should not be hard-rejected. |
| `mutated-bad-001` | `mutated-bad` | `artifacts/corpus/mutated-bad/manifest-001.md` | synthetic-mutation | Deliberately corrupted claim/evidence alignment for rejection testing. |

## Calibration Targets

- Mutated-bad catch rate: `>=90%`
- Clean-real approval rate: `>=95%`
- False positive rate: `<5%`
- Wilson-bound precision confidence: `95%`

## Cross-Model Audit

Cross-model audit cases must remain tagged separately from the primary calibration corpus. Divergences are recorded as audit outcomes, not overwrites of the primary judgment.
