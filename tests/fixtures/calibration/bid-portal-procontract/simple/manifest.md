# Procontract Bid Portal Simple Corpus

Fixture surface: bid-portal-procontract simple forms.

The simple corpus keeps the historical regression rows from the Phase 4 skill
beads and adds the remaining TDD-required simple scenarios. The TDD contract
sets six rows as the minimum required mix; this corpus intentionally carries
extra rows because focused skill tests rely on the auth, lookup, upload, draft,
and formal-submit examples.

## Required Scenario Mix

- happy draft: `simple-happy-01`, `simple-draft-saved-01`
- missing company profile: `simple-missing-company-profile-01`
- stale auth: `simple-auth-stale-01`
- duplicate draft: `simple-duplicate-draft-01`
- cap breach: `simple-cap-breach-01`
- portal reference not found: `simple-portal-reference-not-found-01`

## Regression Rows

- duplicate submission: `simple-duplicate-01`
- portal shape drift: `simple-portal-drift-01`
- form fill and upload success: `simple-form-fill-upload-01`
- upload hash mismatch: `simple-upload-hash-mismatch-01`
- formal submission terminal path: `simple-formal-submit-01`

## Evidence Bundles

Rows either point at deterministic per-case cassettes under
`simple/fixtures/cassettes/` or the aggregate
`tests/fixtures/cassettes/procontract-bid-simple.json` bundle for shell-only
cases. No live portal access is required for T2 replay.
