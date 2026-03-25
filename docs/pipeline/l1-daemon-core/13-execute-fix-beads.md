# L1 Daemon Core — Execute Fix Beads

## Your Mission

Execute the fix beads created by the completion audit. For each fix bead:
1. Read the bead — it's self-contained with spec refs and TDD test IDs
2. Implement the fix
3. Run the specific TDD tests referenced in the bead
4. Verify acceptance criteria
5. Commit, mark bead complete

---

## Source Documents

```
docs/strategy/layers/layer-1-thin-local-runner.md                   — Spec (SOT #1) — Tiebreaker if fix bead is ambiguous
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md        — TDD (SOT #2) — Test definitions for verification
```

### Fix Beads

```bash
br list --label audit-fix --json    # All fix beads from completion audit
br ready --label audit-fix          # Ready fix beads (no blockers)
```

---

## Quality Gates

After EVERY fix:

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test <bead_specific_tests>
```

ALL must pass before marking the bead complete.

---

## Workflow Per Fix Bead

```bash
# 1. Pick ready fix bead
br ready --label audit-fix

# 2. Claim it
br update <id> --status=in_progress

# 3. Reserve files
# file_reservation_paths(...)

# 4. Implement the fix (the bead description tells you exactly what)

# 5. Run quality gates
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test <specific_tests>

# 6. Complete
br close <id> --reason "Fix: <description>"
br sync --flush-only
git add .beads/ crates/ skills/ migrations/
git commit -m "l1: fix — <bead-title> [br-<id>]"
git push

# 7. Release files
# release_file_reservations(...)
```

---

## Mini Push (After Each Fix)

If gaps remain after this pass, go back to the completion audit (step 12) and re-audit.
Check inbox for coordination messages. Check next fix bead.
Keep going until ALL fix beads are complete or you are BLOCKED on a SPEC_QUESTION.
Do NOT stop to ask what to work on next.

---

## Constitutional Constraints

- `#![forbid(unsafe_code)]` — no exceptions, even in fixes
- No `unwrap()` in production code
- Every session must have a cleanup verdict
- Every skill run must produce a run manifest
- Build + lint + test must pass after every fix
