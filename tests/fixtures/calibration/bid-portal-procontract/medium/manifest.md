# Procontract Bid Portal Medium Corpus

Fixture surface: bid-portal-procontract medium forms.

## Scenario Mix

- happy draft: `medium-happy-draft-01`
- upload success: `medium-upload-success-01`
- upload hash mismatch: `medium-upload-hash-mismatch-01`
- quote override: `medium-quote-override-01`
- validation rejection: `medium-validation-rejection-01`
- selector drift: `medium-selector-drift-01`
- resume draft: `medium-resume-draft-01`
- duplicate submission: `medium-duplicate-submission-01`

## Evidence Bundle

All rows replay against `tests/fixtures/cassettes/procontract-bid-medium.json`.
The aggregate cassette is keyed by `case_id` and carries branch decisions,
portal-state verification, screenshot expectations, and side-effect checkpoints.
