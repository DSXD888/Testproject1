# L3 Operational Hardening — Agent Kickoff Prompt

> **Copy this entire file into a fresh Claude Code / Codex session to onboard an implementation agent.**

---

## 1. Your Mission

You are implementing **Layer 3 — Operational Hardening** of the Procurement Intelligence Platform. This layer transforms the single-operator tool from Layer 2 into an **unattended daily automation pipeline**: cron scheduling, resource management, PTY hardening, relation resolution, source health monitoring, skill promotion, and the worker-validator system in dry-run mode.

Layer 3 builds on **Layer 0** (SQLite schema, intelligence pipeline, gold set, evaluation harness), **Layer 1** (Rust daemon — IPC, PTY sessions, artifact capture, job runner, error manifests, commit fence), and **Layer 2** (desktop shell, pipeline data model, Contracts Finder skill). **L3 does NOT modify L0/L1/L2 production code** — it extends the daemon with 12 new module surfaces, 5 new SQLite tables, 4 operational skills, 1 validator base skill, and 2 shared TypeScript packages.

**Deliverable:** Daemon runtime with cron scheduling (`scheduler/cron.rs`), resource arbiter (`agents/arbiter.rs`), job claim extraction (`jobs/claim.rs`), PTY hardening (`pty/hardening.rs`), four-layer evidence (`tracing/structured.rs`, `tracing/forensic.rs`), validation pipeline (`validation/policy.rs`, `validation/executor.rs`, `validation/manifest_writer.rs`), source registry (`sources/registry.rs`), checkpoint store (`db/checkpoints.rs`), 4 operational skills under `skills/`, base validator under `skills/validation/base/`, relation engine at `packages/intelligence/src/relations/`, and error renderer at `packages/manifests/`.

**Key constraint:** This is a **MIXED-STACK layer**. Both `cargo test` and `bun run test` must pass. Rust 2024 (nightly) + TypeScript (Bun).

**Exit criteria:** System runs unattended for 5 consecutive days with >=95% skill success rate and no manual intervention.

**Organization: FLAT.** No team hierarchy. Pick the next ready bead, implement it, move on. Never ask "what should I work on next?" — run `br ready` and pick the highest-priority unblocked task.

**Coordination: AGENTMAIL-FIRST.** Before touching ANY code: register with AgentMail, check inbox, announce which bead you're claiming, reserve the files you'll edit. This prevents conflicts and keeps the swarm coherent.

**Beads are SELF-CONTAINED.** Each bead has full file paths, type definitions, test specs, acceptance criteria, and dependency info. Implement from the bead alone. The spec is only needed as a tiebreaker.

---

## 2. Source of Truth Reading List (MANDATORY)

Read these IN THIS ORDER before writing any code:

1. **`AGENTS.md`** — **READ FULLY** — coding conventions, toolchain (`br` not `bd`, Bun not npm), git rules, AgentMail, beads workflow, session protocol, UBS bug scanner
2. **Spec (SOT #1)** — `docs/strategy/layers/layer-3-operational-hardening.md` — **READ FULLY** — 12 daemon module surfaces, resource arbiter typed quotas, cron scheduler explicit skill binding, PTY hardening bundle, worker-validator dry-run system, skill promotion 6-stage flow, source health and circuit breaker, relation resolution, outcome classes, fail-open semantics, trust-state lattice, calibration corpora
3. **TDD (SOT #2)** — `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — **SKIM** — 21 test files, T1-T4 lanes plus Manual Gate, test ID pattern `L3-T-{suite}-{NNN}`
4. **Strategy (SOT #3)** — `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — **SKIM** — 6 phases, 36 beads, phase ordering DAG, gate criteria per phase
5. **Bootstrap** — `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md` — **SKIM** — environment prerequisites, migration scaffold, skill directory scaffold, test workspace layout, fixture families, command matrix
6. **Key codebase files** (study for integration surface):
   - `crates/daemon/src/main.rs` — daemon entry, module declarations, `bootstrap_runtime()`, ~131K
   - `crates/daemon/src/jobs/runner.rs` — JobRunner central type, `claim_next_job`, `enqueue_job`, `persist_success`, ~25K
   - `crates/daemon/src/sessions/reaper.rs` — SessionReaper, CleanupVerdict, ~17K
   - `crates/daemon/src/agents/artifacts.rs` — RunManifestFrontmatter, DaemonEvidenceAppendix, ArtifactStore, ~15K
   - `crates/daemon/src/agents/contracts.rs` — ValidationDisposition, OQG signals, checkpoint sequencing, ~9.2K
   - `crates/daemon/src/errors/manifest.rs` — ErrorCode enum (28 variants), ~14K
   - `crates/daemon/src/pty/session.rs` — SessionPhase (7 phases), PtyCommand, SpawnOptions, ~15K
   - `crates/daemon/src/agents/adapter.rs` — AgentAdapter trait, ControlEvent, ExecutionMode, ~3.7K
   - `crates/daemon/src/db/sqlite.rs` — StateDb, commit_clock, `with_write_transaction`, ~9.3K
   - `crates/daemon/src/ipc/command_router.rs` — IPC dispatch, manual-trigger command boundary, ~9.2K
   - `crates/daemon/Cargo.toml` — current dependencies (Phase 1 adds `cron` + `sysinfo`)
   - `packages/intelligence/src/qualification/goods-qualification.ts` — TS pattern reference
   - `packages/manifests/render-opportunity.ts` — TS rendering pattern reference

---

## 3. AgentMail Registration

**Project key:** `/Users/alinaaitova/Desktop/Testproject1`

**Option A — Macro (preferred):**
```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="claude-code",
  model="claude-opus-4-6",
  task_description="L3 Operational Hardening implementation"
)
```

**Option B — Granular:**
```
ensure_project(human_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="claude-code",
  model="claude-opus-4-6"
)
```

After registration you will receive your `agent_name`. Use it for all subsequent calls.

---

## 4. Situational Awareness (Check BEFORE Picking Work)

**Every session, before touching code:**

```
fetch_inbox(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  include_bodies=true
)
```

```
search_messages(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  query="l3 in_progress OR claimed"
)
```

```bash
git pull --rebase
br ready --json   # Current work queue — only pick beads with `l3-` prefix in title
```

**KEY PRINCIPLE:** If another agent claimed a bead, DO NOT take it. Pick a different one.

Bead IDs for L3 are `testproject1-hmw.1` through `testproject1-hmw.36`. Filter by `l3-` in the title.

---

## 5. Claim Protocol (Announce + Reserve)

When you pick a bead:

**Step 1 — Claim in beads:**
```bash
br update <id> --status=in_progress
```

**Step 2 — Reserve files:**
```
file_reservation_paths(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  paths=["crates/daemon/src/validation/**", "crates/daemon/tests/validation_*"],
  ttl_seconds=3600,
  exclusive=true,
  reason="<bead-id>"
)
```
Adjust `paths` to match the bead's file list. Use glob patterns for the directories your bead touches.

**Step 3 — Announce:**
```
send_message(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  sender_name="<your-name>",
  to=["broadcast"],
  thread_id="<bead-id>",
  subject="[<bead-id>] Claimed: <bead-title>",
  body_md="Starting implementation of <bead-title>. Files reserved: <list>.",
  ack_required=true
)
```

**FILE_RESERVATION_CONFLICT handling:** If you get a conflict, pick a different bead. Do not force or wait.

---

## 6. Beads <-> AgentMail Mapping

| Concept | Value |
|---------|-------|
| Mail `thread_id` | Bead ID (e.g., `testproject1-hmw.17`) |
| Mail subject prefix | `[testproject1-hmw.17]` |
| File reservation `reason` | Bead ID |
| Commit message prefix | Bead ID |

---

## 7. Bead Inventory (36 Beads, 6 Phases)

**Phase 1 — Schema, Claim, Admission Foundation (P0, 5 beads):**
| ID | Title | Depends On |
|----|-------|------------|
| `hmw.1` | l3-p1-deps-migration | — |
| `hmw.2` | l3-p1-job-claim | hmw.1 |
| `hmw.3` | l3-p1-arbiter | hmw.1 |
| `hmw.4` | l3-p1-runner-main-integration | hmw.2, hmw.3 |
| `hmw.5` | l3-p1-tests | hmw.4 |

**Phase 2 — Scheduler, Registry, Source Health (P1, 5 beads):**
| ID | Title | Depends On |
|----|-------|------------|
| `hmw.6` | l3-p2-source-registry | hmw.5 |
| `hmw.7` | l3-p2-cron-scheduler | hmw.6 |
| `hmw.8` | l3-p2-operational-skills | hmw.6 |
| `hmw.9` | l3-p2-runner-main-integration | hmw.7 |
| `hmw.10` | l3-p2-tests | hmw.8, hmw.9 |

**Phase 3 — PTY Reliability, Four-Layer Evidence (P1, 5 beads):**
| ID | Title | Depends On |
|----|-------|------------|
| `hmw.11` | l3-p3-pty-hardening | hmw.5 |
| `hmw.12` | l3-p3-reaper-health | hmw.11 |
| `hmw.13` | l3-p3-structured-tracing | hmw.12 |
| `hmw.14` | l3-p3-forensic-capture | hmw.13 |
| `hmw.15` | l3-p3-main-tests | hmw.14 |

**Phase 4 — Manifest, Validation Dry-Run Core (P1, 10 beads):**
| ID | Title | Depends On |
|----|-------|------------|
| `hmw.16` | l3-p4-artifacts-extensions | hmw.10, hmw.15 |
| `hmw.17` | l3-p4-manifest-writer | hmw.16 |
| `hmw.18` | l3-p4-contracts-checkpoints | hmw.17 |
| `hmw.19` | l3-p4-validation-policy | hmw.17 |
| `hmw.20` | l3-p4-validator-skill | hmw.17 |
| `hmw.21` | l3-p4-validation-executor | hmw.18, hmw.19, hmw.20 |
| `hmw.22` | l3-p4-runner-main-integration | hmw.21 |
| `hmw.23` | l3-p4-policy-tests | hmw.19 |
| `hmw.24` | l3-p4-executor-dry-run-tests | hmw.22 |
| `hmw.25` | l3-p4-golden-partial-tests | hmw.22 |

**Phase 5 — Relations, Error-Surface Packages (P1, 4 beads):**
| ID | Title | Depends On |
|----|-------|------------|
| `hmw.26` | l3-p5-relation-engine | hmw.5 |
| `hmw.27` | l3-p5-relation-tests | hmw.26 |
| `hmw.28` | l3-p5-error-renderer | hmw.5 |
| `hmw.29` | l3-p5-package-config | hmw.27, hmw.28 |

**Phase 6 — Promotion, Status Surfaces, Unattended Gate (P2, 7 beads):**
| ID | Title | Depends On |
|----|-------|------------|
| `hmw.30` | l3-p6-promotion-bookkeeping | hmw.23, hmw.24, hmw.25, hmw.29 |
| `hmw.31` | l3-p6-canary-blackout | hmw.30 |
| `hmw.32` | l3-p6-status-surfaces | hmw.31 |
| `hmw.33` | l3-p6-command-router | hmw.32 |
| `hmw.34` | l3-p6-runner-main-capstone | hmw.33 |
| `hmw.35` | l3-p6-lifecycle-tests | hmw.30 |
| `hmw.36` | l3-p6-capstone-tests | hmw.34 |

**Parallelism opportunities:** After Phase 1, Phases 2, 3, and 5 can proceed in parallel. Phase 4 requires both Phase 2 and Phase 3. Phase 6 requires Phase 4 and Phase 5.

```text
Phase 1 -> Phase 2 -> Phase 4 -> Phase 6
    |           ^          ^
    |           |          |
    +-> Phase 3 +----------+
    |
    +-> Phase 5 -----------+
```

---

## 8. Quality Gates (Actual Commands)

Run ALL applicable gates before every commit:

```bash
# Rust (ALWAYS — even if you only changed TS, these must not regress)
cargo check --all-targets
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# TypeScript (only if TS files changed — packages/intelligence, packages/manifests)
bun run test
(cd packages/intelligence && bun run test)
(cd packages/manifests && bun test)

# UBS bug scanner (before commit — scope to changed files)
ubs $(git diff --name-only --cached)
```

**Constitutional constraints:**
- `#![forbid(unsafe_code)]` — no unsafe Rust, ever
- No `unwrap()` in production code — use `?`, `expect("reason")`, or proper error handling
- Rust 2024 edition (nightly) — `rust-toolchain.toml` governs
- Bun only — never npm/yarn/pnpm for TypeScript

---

## 9. Execution Loop

Repeat until all `l3-*` beads are done or you are fully blocked:

1. **Situational awareness** — `fetch_inbox` + `search_messages` for in-progress L3 work
2. **Pull latest** — `git pull --rebase`
3. **Find ready work** — `br ready --json` — pick highest-priority `l3-*` bead
4. **Claim** — `br update <id> --status=in_progress`
5. **Reserve files** — `file_reservation_paths(...)` with bead's file list
6. **Announce** — `send_message(...)` to broadcast with `[<bead-id>] Claimed: ...`
7. **Read the bead** — `br show <id>` — it's self-contained
8. **Study codebase** — read the existing files your bead integrates with
9. **Implement** — write production code + tests per bead spec
10. **Verify** — run ALL applicable quality gates (Rust always, TS when changed)
11. **Commit + Push** — `git add <files> && git commit -m "<bead-id>: <summary>" && git push`
12. **Release** — `release_file_reservations(project_key, agent_name)`
13. **Announce completion** — `send_message(...)` with `[<bead-id>] Completed: ...`
14. **Sync beads** — `br close <id> --reason "Completed" && bin/br-sync && git add .beads/ && git commit -m "beads: close <bead-id>" && git push`
15. **Loop** — go to step 1

---

## 10. Session Landing Protocol

When fully blocked (no ready `l3-*` beads and nothing in-progress):

1. **Push all work:**
   ```bash
   git pull --rebase
   git add <files>
   git commit -m "l3: work in progress"
   git push
   ```

2. **Release all reservations:**
   ```
   release_file_reservations(
     project_key="/Users/alinaaitova/Desktop/Testproject1",
     agent_name="<your-name>"
   )
   ```

3. **Sync beads:**
   ```bash
   bin/br-sync
   git add .beads/
   git commit -m "beads: sync l3 state"
   git push
   ```

4. **Announce handoff:**
   ```
   send_message(
     project_key="/Users/alinaaitova/Desktop/Testproject1",
     sender_name="<your-name>",
     to=["broadcast"],
     subject="Session handoff: <your-name> signing off",
     body_md="## Completed\n- <list beads completed>\n## In Progress\n- <any partial work>\n## Notes\n- <context for next agent>",
     importance="normal"
   )
   ```

---

## 11. Critical Rules

1. **Beads are self-contained** — implement from the bead alone; spec is tiebreaker only
2. **Never delete files** without express permission (AGENTS.md Rule #1)
3. **Never stash/revert other agents' work** — treat unfamiliar changes as your own
4. **`br` not `bd`** — always use `br` (beads_rust), never `bd`
5. **`bin/br-sync`** then manually `git add .beads/ && git commit` — br never runs git
6. **Extend, don't fork** — `artifacts.rs`, `contracts.rs`, `runner.rs` are extend-only surfaces
7. **`jobs/runner.rs` is the single enqueue/order boundary** — claim, cron, validator sub-jobs all route through it
8. **Schema first** — `migrations/0005_l3_operational_hardening.sql` must land before any L3 write path
9. **Fail-open is constitutional** — validator failures never block the pipeline
10. **Dry-run only in L3** — verdicts logged, not enforced; enforcement is Layer 4
11. **`main.rs` edits are phase-end integration beads** — don't scatter edits across the file
12. **Do NOT ask what to work on** — run `br ready`, pick the highest-priority unblocked `l3-*` bead, go
13. **Work is NOT done until `git push` succeeds** — always push before ending a session

---

## 12. Quick Reference

| Resource | Path |
|----------|------|
| AGENTS.md | `AGENTS.md` |
| Spec (SOT #1) | `docs/strategy/layers/layer-3-operational-hardening.md` |
| TDD (SOT #2) | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` |
| Strategy (SOT #3) | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` |
| Bootstrap | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md` |
| L3 Migration | `migrations/0005_l3_operational_hardening.sql` |
| Daemon entry | `crates/daemon/src/main.rs` |
| Job runner | `crates/daemon/src/jobs/runner.rs` |
| Manifest types | `crates/daemon/src/agents/artifacts.rs` |
| Validation types | `crates/daemon/src/agents/contracts.rs` |
| Error codes | `crates/daemon/src/errors/manifest.rs` |
| PTY session | `crates/daemon/src/pty/session.rs` |
| IPC dispatch | `crates/daemon/src/ipc/command_router.rs` |
| State DB | `crates/daemon/src/db/sqlite.rs` |
| Relation engine | `packages/intelligence/src/relations/resolve-process-relations.ts` |
| Error renderer | `packages/manifests/render-error-manifest.ts` |
