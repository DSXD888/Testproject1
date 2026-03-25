# L1 Daemon Core — Push Prompt (Continuation Nudge)

## When To Use

Use this prompt to continue an implementation session that paused, got stuck, or completed a bead and needs direction for the next one. Paste this into a fresh or continuing session.

---

## The Push

You are implementing Layer 1 (Thin Local Runner) of the UK procurement intelligence platform. Your implementation artifacts are:

```
Spec:      docs/strategy/layers/layer-1-thin-local-runner.md
TDD:       docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md
Strategy:  docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md
Bootstrap: docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md
Kickoff:   docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-kickoff.md
```

### What To Do Now

1. **Check your inbox:**
   ```
   fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="<your-name>")
   ```
   Act on any messages before picking new work.

2. **Find ready work:**
   ```bash
   br ready --json
   ```
   Pick the highest-priority bead with zero blockers.

3. **Claim it:**
   ```bash
   br update <id> --status=in_progress
   ```

4. **Reserve files:**
   ```
   file_reservation_paths(project_key, agent_name, ["crates/daemon/src/**"], ttl_seconds=3600, exclusive=true, reason="br-<id>")
   ```

5. **Implement the bead.** The bead description is self-contained — it has the spec requirements, TDD test IDs, file listing, and acceptance criteria inline.

6. **Build gate:**
   ```bash
   cargo check --all-targets
   cargo clippy --all-targets -- -D warnings
   cargo fmt --check
   ```

7. **Test gate:**
   ```bash
   cargo test <bead_specific_tests>
   ```

8. **Complete:**
   ```bash
   br close <id> --reason "Completed: <brief summary>"
   br sync --flush-only
   git add .beads/ crates/ skills/ migrations/
   git commit -m "l1: <bead-title> [br-<id>]"
   git push
   ```

9. **Release files:**
   ```
   release_file_reservations(project_key, agent_name, paths=["crates/daemon/src/**"])
   ```

10. **Pick next bead.** Go to step 2. Do NOT stop to ask what to work on next.

### If Blocked

- Dependency not met → check if the blocker bead is in progress by another agent. Wait or pick a different ready bead.
- Compilation error from another agent's code → `git pull --rebase` first. If still broken, create an error manifest and pick a different bead.
- Spec ambiguity → check `docs/strategy/layers/layer-1-thin-local-runner.md` for the authoritative answer. If genuinely ambiguous, create a `br create --type=question` bead and move on.

### Constitutional Constraints

- `#![forbid(unsafe_code)]` — no exceptions
- No `unwrap()` in production code
- Process-group cleanup verdict mandatory for every session
- Run manifest mandatory for every skill execution
- Integer pence for money (`_pence` suffix)
- Integer basis points for confidence (`_bps` suffix, 0-10000)
- Single-writer SQLite enforcement
