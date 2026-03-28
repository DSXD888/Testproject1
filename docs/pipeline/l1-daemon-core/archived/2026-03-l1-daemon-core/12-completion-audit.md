# L1 Daemon Core — Completion Audit

## First Action

Invoke the `/bead-completion-audit` skill.

---

## Your Mission

Deep-read the spec, TDD, and codebase for Layer 1 Daemon Core. Find EVERY gap between what the spec requires and what's actually implemented. Create fix beads for gaps. Do NOT execute the fixes — just create and scope them.

This is the moment of truth: does the codebase match the spec?

---

## Source Documents

```
docs/strategy/layers/layer-1-thin-local-runner.md                   — Spec (SOT #1) — Walk EVERY L1-FR-* requirement
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md        — TDD (SOT #2) — Verify EVERY test exists with correct assertions
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md  — Bootstrap — Verify environment matches
```

### Codebase (THE IMPLEMENTATION — read deeply)

```
crates/daemon/src/main.rs                            — CLI entry, daemon bootstrap
crates/daemon/src/db/sqlite.rs                       — SQLite connection management
crates/daemon/src/pty/session.rs                     — PTY session creation
crates/daemon/src/pty/capture.rs                     — Chunked byte capture
crates/daemon/src/pty/health.rs                      — Session health monitoring
crates/daemon/src/jobs/runner.rs                     — Job Runner
crates/daemon/src/agents/adapter.rs                  — Agent Adapter trait
crates/daemon/src/agents/codex.rs                    — Codex adapter
crates/daemon/src/agents/claude.rs                   — Claude adapter
crates/daemon/src/agents/artifacts.rs                — Artifact store
crates/daemon/src/agents/contracts.rs                — Result contract validator
crates/daemon/src/prompt/renderer.rs                 — Kickoff prompt renderer
crates/daemon/src/errors/manifest.rs                 — Error manifest system
crates/daemon/src/errors/markdown.rs                 — Error manifest Markdown renderer
crates/daemon/src/ipc/descriptor.rs                  — Daemon runtime descriptor
crates/daemon/src/ipc/handshake.rs                   — Desktop-daemon handshake
crates/daemon/src/sessions/reaper.rs                 — Session Reaper
```

### Test Files

```
crates/daemon/tests/agent_pty_session.rs
crates/daemon/tests/agent_contract_session.rs
crates/daemon/tests/agent_procurement_ingestion.rs
crates/daemon/tests/adapter_codex.rs
crates/daemon/tests/adapter_claude.rs
crates/daemon/tests/adapter_result_contract.rs
crates/daemon/tests/skill_lint.rs
crates/daemon/tests/skill_canary_harness.rs
crates/daemon/tests/session_reaper.rs
crates/daemon/tests/ipc_handshake.rs
```

### Skills

```
skills/procurement-ingestion/fts/SKILL.md
skills/procurement-ingestion/fts/result.schema.json
skills/manifest-generation/SKILL.md
skills/manifest-generation/result.schema.json
```

### Migration

```
migrations/0003_sqlite_runtime.sql
```

---

## Audit Checklist

### 1. Spec Requirement Walk

For every `L1-FR-*` requirement in the spec:
- [ ] Implementation exists in the correct source file
- [ ] All acceptance criteria are satisfied by the code
- [ ] Corresponding test(s) exist and verify the requirement

### 2. Definition of Done (13 criteria)

Walk §5.1 Definition of Done:
1. `cargo check --all-targets` passes
2. `cargo clippy --all-targets -- -D warnings` passes
3. Skills launch via CLI (PTY and contract mode)
4. Live FTS batch ingestion works
5. Full E2E loop completes (normalize → ... → manifest)
6. Verifiable output exists (CLI summary + local manifest artifact)
7. Process-group supervision verified (pgid, cleanup verdict)
8. Artifact capture works (compressed bytes, session metadata)
9. Run manifests generated (agent narrative + daemon appendix + fallback)
10. Error manifest system operational
11. Skill linting CI gate validates skill directories
12. Smoke canary harness executes
13. Experiment 4: 10 consecutive batch runs, >=90% success, 0 orphans

### 3. Test Coverage

```bash
cargo test                                    # All tests pass
cargo test -- --list 2>&1 | wc -l           # Total test count
```

Cross-reference with TDD test IDs — every test ID should correspond to a real test function.

### 4. Quality Gates

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

All must pass with zero warnings.

---

## Follow-Up Pass (Sub-Optimization)

> After creating all fix beads, send this as a follow-up message in the same session.

ULTRATHINK and verify with fresh eyes:

1. **Re-read the fix beads you created.** Are they specific enough? Does each one reference the right `L1-FR-*` spec section and TDD test IDs? Could an agent implement each fix from the bead description alone?

2. **Open `crates/daemon/src/agents/adapter.rs`** — does every `AgentAdapter` trait method (`probe`, `resolve_mode`, `build_launch_spec`, `parser`) have a corresponding test in BOTH `tests/adapter_codex.rs` and `tests/adapter_claude.rs`? If not, is there a fix bead for the missing coverage?

3. **Open `crates/daemon/src/sessions/reaper.rs`** — does the reaper produce a cleanup verdict for EVERY session lifecycle path? Cross-reference with the 7-phase lifecycle in the spec (§2.4). Are there fix beads for any untested transition?

4. **Open `crates/daemon/src/pty/capture.rs`** — verify chunk boundary behavior is tested. Does the test for exactly 32KB chunks exist? What about 0-byte sessions? Are these covered by fix beads if missing?

5. **Open `crates/daemon/src/errors/manifest.rs`** — cross-reference with the 12 error types in spec §1.9/§4.3. Is every error type exercised by at least one test? Create fix beads for any missing coverage.

6. **Open `crates/daemon/src/jobs/runner.rs`** — verify `stage_run_id` scoping is tested for both set-replacement and additive semantics. Does the test cover partial failure within a stage?

7. **Check the migration** — open `migrations/0003_sqlite_runtime.sql` and verify every table column matches the spec's §2.7/§2.9 schema. Any schema drift = fix bead.

8. **Are the fix beads ordered?** Some fixes may depend on other fixes. Use `br dep add` to establish ordering if needed.

9. **Estimate total fix beads.** If more than 10, something went wrong in the original implementation — flag it to the operator before proceeding to execution.
