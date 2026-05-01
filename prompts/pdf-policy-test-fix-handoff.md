# Pre-existing Test Failure: `pdf::policy::tests::test_role_from_structural_content`

## CONTEXT

While auditing the Stage 11 real-data validation, the full daemon test suite was run and one test fails:

```
test pdf::policy::tests::test_role_from_structural_content ... FAILED

thread 'pdf::policy::tests::test_role_from_structural_content' panicked at crates/daemon/src/pdf/policy.rs:272:9:
assertion `left == right` failed
  left: PricingSchedule
 right: MixedTenderPack

test result: FAILED. 59 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

This is **not related to Stage 11**. The failure has been latent since commit `0df0e0b testproject1-vt96.5.1: Add PDF extraction policy module`. The Stage 11 MVP manifest claimed `cargo test` PASS by running only the `landed_cost` filter (`cargo test -p daemon --bin daemon landed_cost`), missing this. With the failure fixed, `cargo test -p daemon --bin daemon` will be fully green.

## ROOT CAUSE

`crates/daemon/src/pdf/policy.rs` defines:

```rust
fn normalize(raw: &str) -> String {
    raw.chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}
```

This **strips all whitespace**. So `"# Tender Pack\n\n## Lot 1\n## Pricing Schedule\n| Item | Qty | Rate |"` becomes `"tenderpacklot1pricingscheduleitemqtyrate"`.

Then `role_from_structural_content` builds the `tender` flag with these substring checks:

```rust
let tender = normalized.contains("lot ")              // never matches — space stripped
    || normalized.contains("technical specification") // never matches
    || normalized.contains("statement of requirements") // never matches
    || normalized.contains("invitation to tender")    // never matches
    || normalized.contains("standard")                // matches if substring present
    || normalized.contains("bs en");                  // never matches
```

The five substring needles that contain a space can **never** match the normalized haystack. The test's input contains the literal token `Lot 1`, which after normalization is `lot1`. With `tender = false`, the `tender && (pricing || boq)` branch never fires, so the function returns `PricingSchedule` instead of `MixedTenderPack`.

## FIX OPTIONS (pick one)

### Option A — fix the function (preferred)

Update the substring needles in `role_from_structural_content` (and the analogous logic in `looks_like_*` helpers, if any) to match the normalized form:

```rust
let tender = normalized.contains("lot1")
    || normalized.contains("lot2")  // or detect generally with regex / numeric suffix
    || normalized.contains("technicalspecification")
    || normalized.contains("statementofrequirements")
    || normalized.contains("invitationtotender")
    || normalized.contains("standard")
    || normalized.contains("bsen");
```

This is the minimum-change fix that makes the test pass. Audit the rest of the file for the same defect — every check that contains a space inside a string literal is suspicious. There is at least one other instance: line 165 (`"combined pack"` as a `normalized.contains` argument in the post-normalization block) — it is dead code today.

### Option B — change `normalize` to preserve spaces

If you want the substring needles to read naturally (`"lot "` etc.), update `normalize` to map non-alphanumeric ASCII to a single space and collapse adjacent whitespace. This is a more invasive change and may regress other call sites (e.g. `looks_like_pricing_schedule` and `looks_like_boq` and the OCDS/filename normalizations earlier in the file). If you go this route, run all `pdf::*` tests — not just `policy::tests` — and watch for new failures.

### Option C — delete or skip the test

Only acceptable if the function is being deprecated. It is not — it is the structural-content fallback used when neither OCDS document type nor filename gives a clear signal, which is a real production code path.

## DELIVERABLE

1. Apply Option A or B and ensure `cargo test -p daemon --bin daemon test_role_from_structural_content` passes.
2. Run the full suite — `cargo test -p daemon --bin daemon` — and confirm 60/60 pass.
3. Run `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings`.
4. Commit with message along the lines of: `fix(pdf): role_from_structural_content normalization mismatch`.

## CONSTRAINT

Do not change the test's expected output (`MixedTenderPack`) — the test correctly captures the intended classification: a tender pack containing both a `Lot` heading and a `Pricing Schedule` table is a `MixedTenderPack`, not a pure pricing schedule. The bug is in the function, not the assertion.

## REMINDERS

- ULTRATHINK if you choose Option B; the normalize change has wide blast radius.
- This is independent of Stage 11; the fix should be a small isolated commit.
