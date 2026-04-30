# PDF Policy Structural-Content Test Fix Validation Manifest

> Generated: 2026-04-30T20:54:04Z
> Status: GREEN

## Summary

Fixed the latent PDF document-role classifier failure where
`pdf::policy::tests::test_role_from_structural_content` returned
`PricingSchedule` instead of `MixedTenderPack`. The fix keeps the existing
alphanumeric-only `normalize` contract and updates post-normalization role
checks to compare against normalized phrase spellings.

The fix is pushed to `main` as:

```
651c3c0 fix(pdf): role_from_structural_content normalization mismatch
```

## Root Cause

`normalize` strips all non-alphanumeric characters, including whitespace. The
structural-content classifier and related helpers then searched the normalized
haystack with spaced needles such as `pricing schedule`, `invitation to tender`,
`combined pack`, and `item ref`. Those branches were unreachable after
normalization.

The failing test input:

```
# Tender Pack

## Lot 1
## Pricing Schedule
| Item | Qty | Rate |
```

normalizes into an alphanumeric stream, so `Lot 1` becomes `lot1` and
`Pricing Schedule` becomes `pricingschedule`.

## Changes Made

| File | Change |
|---|---|
| `crates/daemon/src/pdf/policy.rs` | Replaced spaced post-normalization substring checks with normalized forms such as `pricingschedule`, `scheduleofrates`, `billofquantities`, `statementofrequirements`, `invitationtotender`, `combinedpack`, `coverletter`, `termsandconditions`, and `itemref`. |
| `crates/daemon/src/pdf/policy.rs` | Added `contains_lot_heading` so structural content with a normalized `lot` heading followed by a numeric suffix, such as `## Lot 1`, is treated as tender-pack evidence without changing the global normalization behavior. |

## Validation Results

| Command | Result |
|---|---|
| `cargo test -p daemon --bin daemon test_role_from_structural_content` | PASS - 1 passed, 59 filtered out |
| `cargo test -p daemon --bin daemon` | PASS - 60 passed, 0 failed |
| `cargo fmt --check` | PASS |
| `cargo check --all-targets` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `ubs crates/daemon/src/pdf/policy.rs` | PASS - exit 0; no critical issues. UBS reported existing test `assert!` warning inventory and informational items only. |

## Push Record

The first local commit was created as `4e766ac`, but the push was rejected
because `origin/main` advanced. The fix was replayed cleanly onto the new
remote tip and pushed as `651c3c0`.

Remote verification:

```
651c3c033a2f97e681db2f725ce1d42127ed6aa9 refs/heads/main
```

## Scope

This was an isolated PDF policy fix. No Stage 11 behavior, landed-cost skill
prompt, daemon job-runner, runtime state, or database file was intentionally
changed by this fix.
