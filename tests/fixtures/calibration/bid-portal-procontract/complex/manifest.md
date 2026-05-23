# Procontract Bid Portal Complex Corpus

Fixture surface: bid-portal-procontract complex forms.

## Scenario Mix

- multi-lot draft: `complex-multi-lot-draft-01`
- multi-document upload: `complex-multi-document-upload-01`
- conditional declarations: `complex-conditional-declarations-01`
- legal declaration boundary: `complex-legal-declaration-boundary-01`
- final submit authorized: `complex-final-submit-authorized-01`
- final submit blocked: `complex-final-submit-blocked-01`
- validator outage: `complex-validator-outage-01`
- validation override: `complex-validation-override-01`
- remote draft changed externally: `complex-remote-draft-changed-externally-01`
- unknown portal state reconciliation: `complex-unknown-portal-state-reconciliation-01`

## Evidence Bundle

All rows replay against `tests/fixtures/cassettes/procontract-bid-complex.json`.
The aggregate cassette is keyed by `case_id` and carries portal-state
verification, branch decisions, screenshot checkpoints, override citations, and
highest side-effect class tracking.
