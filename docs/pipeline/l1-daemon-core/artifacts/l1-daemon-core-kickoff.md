# L1 Daemon Core — Execution Kickoff

> **Role:** You are a Rust implementation agent building the Layer 1 daemon core for a UK public procurement intelligence platform. This is a greenfield Rust crate — no existing Rust code in the repo. You will build a thin Rust daemon that launches AI agent sessions via PTY and contract mode, captures forensic artifacts, supervises process lifecycles, and runs the first live-source pipeline loop.
>
> **Deliverable:** A working `crates/daemon` crate with 17 production source files, 2 skill directories, 1 migration, 10 integration test files, and 6 static CI gates — all passing `cargo check && cargo clippy && cargo test`.
>
> **Key constraint:** This is Layer 1 — a *thin* local runner. No cron scheduler, no resource arbiter, no desktop UI, no sync engine, no validation enforcement. Manual CLI trigger only. Single-database model (one `state.db`). The intelligence pipeline logic lives in Layer 0's TypeScript — Layer 1 only wraps it in daemon-managed execution.

---

## 1. Project Context

### What This Project Is

A UK public procurement intelligence platform that ingests official contract notice data, qualifies it for physical-goods supply opportunities, enriches notices with supplier pricing and landed-cost estimates, and surfaces high-margin opportunities. The architecture is a local Rust daemon orchestrating AI agent sessions, with local SQLite as operational authority.

### What Layer 0 Provides (Already Built)

Layer 0 is the TypeScript intelligence pipeline and evaluation harness:

- **TypeScript packages:** `packages/contracts-schema/` (domain types, Zod schemas), `packages/intelligence/` (pipeline logic — qualification, extraction, search, customs screening, margins, evaluation)
- **Skills:** 5 CLI-invocable skills (document-acquisition, lot-extraction, product-extraction, preliminary-qualification, supplier-price-enrichment/ebay) — each with `SKILL.md` + schemas + canary inputs + golden assertions
- **SQLite schema:** `migrations/0001_sqlite_state_initial.sql` — full pipeline schema (47 tables) including dormant tables for future layers. All push-owned tables include sync tracking columns (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`) from creation. All columns needed by later layers exist as nullable from creation.
- **Evaluation harness:** Gold-set lot corpus, per-lot precision/recall metrics, experiments 1-3b validated
- **Workspace root:** `package.json`, `bunfig.toml`, `.env.example` (Bun workspace)

Skills in Layer 0 run from CLI (`bun run skill-name`). Layer 1 wraps the same skills in daemon-managed execution with full lifecycle supervision.

### What Layer 1 Is

The thin Rust daemon core — just enough to:

1. **Launch skills** via adapter-mediated sessions (PTY for browser automation, contract mode for deterministic tasks)
2. **Capture artifacts** — compressed raw PTY bytes, stream indexes, run manifests
3. **Supervise processes** — process-group management, SIGCHLD/waitpid reaping, cleanup verdicts (`clean`/`forced`/`leaked`)
4. **Track pipeline stages** — `stage_run_id` atomic commit cohorts, `process_stage_status`/`process_stage_heads`
5. **Generate error manifests** — structured Markdown error manifests for every pipeline failure
6. **Run the first live loop** — manual CLI trigger ingests live FTS notices through the full local pipeline

Two capabilities proven at exit:

- **Experiment 4 (Runtime reliability):** 10 consecutive batch runs, >=90% skill success rate, 0 orphaned processes
- **First live operational loop:** `notice ingestion -> pipeline processing -> verifiable local output` (CLI summary + local markdown manifest artifact)

### What Layer 1 Does NOT Include

- Desktop UI (Layer 2)
- Supabase sync engine (Layer 4)
- Cron scheduler (Layer 3) — manual CLI trigger only
- Resource arbiter (Layer 3) — no concurrency limits
- Validation system (Layer 3 dry-run, Layer 4 enforcement)
- Relation resolution (Layer 3)
- Contracts Finder source (Layer 2)
- Multi-source marketplace search (Layer 5)
- PTY hardening beyond basic supervision (Layer 3)

---

## 2. Architecture Overview

### Daemon Crate Structure (17 Core Files)

```
crates/daemon/src/
  main.rs                    # CLI entry, arg parsing, daemon bootstrap, signal handling
  db/sqlite.rs               # SQLite WAL, single-writer, migration runner, commit_seq, PRAGMAs
  pty/session.rs             # PTY pair creation, setsid, TIOCSCTTY, CLOEXEC, pgid spawn, geometry
  pty/capture.rs             # Chunked byte capture, zstd compression, stream.index.jsonl, EIO handling
  pty/health.rs              # No-output stall, no-progress stall, auth challenge, host sleep detection
  jobs/runner.rs             # Job Runner: claim, execute via adapter, stage_run_id, commit fence
  agents/adapter.rs          # AgentAdapter trait, OutputFrame/ControlEvent enums, LaunchSpec, mode selection
  agents/codex.rs            # Codex adapter: probe, launch spec, JSONL parsing, canary mode
  agents/claude.rs           # Claude adapter: probe, launch spec, JSON/stream-json parsing
  agents/artifacts.rs        # Artifact store: compressed chunks, stream index, run manifests
  agents/contracts.rs        # Result contract validator, OQG checks, checkpoint sequencing
  prompt/renderer.rs         # Kickoff prompt composition, SKILL.md injection, resume tokens
  errors/manifest.rs         # Error manifest generation with full runtime context
  errors/markdown.rs         # Error manifest Markdown rendering engine
  ipc/descriptor.rs          # daemon.info.json, daemon.lock, stale-socket cleanup
  ipc/handshake.rs           # hello/welcome protocol, version negotiation, schema epoch
  sessions/reaper.rs         # SIGCHLD handling, waitpid drain, pid->session map, cleanup verdicts

  bin/test_agent.rs          # Helper binary for integration tests (behind test-support feature)
```

The crate is a single binary. No `lib.rs` split. Parent `mod` declarations live inline in `main.rs`. The 17-file inventory must not silently expand with extra `mod.rs` files.

### Two-Lane Adapter Model

Every agent session — whether PTY or contract — produces a unified event stream through the `AgentAdapter` trait:

- **Output lane:** `OutputFrame` — `Pty(Bytes)` | `Stdout(Bytes)` | `Stderr(Bytes)`. Sequenced losslessly for capture before semantic parsing.
- **Control lane:** `ControlEvent` — `Started` | `Checkpoint` | `Progress` | `AuthChallenge` | `Warning` | `FinalResultReady` | `Exited`. Derived from output frames by the adapter's parser.

Mode selection: Skills declare an `InteractionProfile` (`PtyRequired` | `ContractRequired` | `ContractPreferred { allow_pty_fallback }`). The adapter probes the agent CLI and resolves execution mode from `SkillManifest + CapabilityReport`.

### Session Lifecycle (7 Phases)

```
Created -> Spawned -> Active -> Terminating -> Exited -> Reaped -> Archived
```

Every session runs in a dedicated process group (`pgid`). Every session gets a cleanup verdict: `Clean` | `Forced` | `Leaked`. Every session produces a run manifest (agent narrative + daemon evidence appendix; fallback machine-authored if agent fails to narrate).

### Single-Database Model

One `state.db` file. Layer 0's `0001_sqlite_state_initial.sql` creates the pipeline schema. Layer 1's `0003_sqlite_runtime.sql` adds daemon tables: `agent_session_logs`, `raw_documents`, `error_manifests`, `executors`, `source_cursors`, `commit_clock`. Filesystem artifact store for compressed PTY bytes (not inline in SQLite).

### Commit Fence

The daemon maintains a monotonic `commit_seq` via the `commit_clock` table. Every batch of writes gets a `visibleAfterSeq`. The desktop (Layer 2) can read up to the last committed `commit_seq` — but in Layer 1, only the daemon writes and reads.

---

## 3. Technical Stack

- **Language:** Rust 2024 edition (nightly required — see `rust-toolchain.toml`)
- **Async runtime:** Tokio (multi-thread)
- **Database:** rusqlite (bundled SQLite)
- **PTY:** portable-pty + nix (signal, process, term)
- **Compression:** zstd for artifact chunks
- **Serialization:** serde + serde_json + serde_yaml
- **CLI:** clap (derive)
- **Hashing:** sha2 (SHA-256 for artifact integrity)
- **Unsafe policy:** `#![forbid(unsafe_code)]` — constitutional constraint, no exceptions

### Build / Lint / Test Commands

```bash
# Build gate (must pass before every commit)
cargo check --all-targets

# Lint gate (pedantic + nursery enabled at workspace level)
cargo clippy --all-targets -- -D warnings

# Format gate
cargo fmt --check

# Test gate
cargo test

# Combined pre-commit gate
cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
```

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1 | async runtime (rt-multi-thread, macros, process, signal, sync, time) |
| `rusqlite` | 0.32 | SQLite with bundled feature |
| `portable-pty` | 0.8 | PTY pair creation |
| `nix` | 0.29 | signal, process, term (setsid, waitpid, killpg) |
| `clap` | 4 | CLI argument parsing (derive) |
| `zstd` | 0.13 | artifact compression |
| `anyhow` | 1 | error handling in application code |
| `serde` / `serde_json` | 1 | serialization |
| `uuid` | 1 | v4 UUIDs for session/job IDs |
| `fs4` | 0.8 | file locking (daemon.lock) |

---

## 4. Beads Workflow

### Finding Work

```bash
br ready --json        # Shows beads with no unresolved blockers
br ready               # Human-readable list
br show <id>           # Full bead details with dependencies and description
```

There are **22 task beads** (L1-01 through L1-22) plus 1 epic container. The bead IDs follow the pattern `testproject1-63g.N`. Always pick the highest-priority non-epic bead from `br ready`.

### Claiming Work

```bash
br update <id> --status=in_progress
```

Before editing files, reserve them via AgentMail:

```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l1-daemon-executor",
  model="your-model"
)

file_reservation_paths(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="your-name",
  paths=["crates/daemon/src/**"],
  ttl_seconds=3600,
  exclusive=true,
  reason="br-testproject1-63g.N"
)
```

Announce start via AgentMail `send_message` with `thread_id="br-testproject1-63g.N"`.

### Completing Work

```bash
# Close the bead
br close <id> --reason "Completed: <brief summary>"

# Sync beads to JSONL (no git ops)
br sync --flush-only

# Commit and push
git pull --rebase
git add crates/daemon/ migrations/ skills/ .beads/
git commit -m "feat(daemon): <description>

br-testproject1-63g.N"
git push
```

Release file reservations and announce completion via AgentMail after pushing.

### Key Rules

- Commit messages MUST include `br-testproject1-63g.N` for traceability
- `br` is non-invasive — it NEVER runs git commands automatically
- Always `br sync --flush-only && git add .beads/` before committing
- Never stop to ask what to work on — run `br ready` and pick the next bead

---

## 5. Quality Gates

### Per-Bead Gate (Every Bead Must Pass)

```bash
cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test
```

### Constitutional Constraints

- **No `unsafe` code** — `#![forbid(unsafe_code)]` enforced on all production and test targets
- **No `unwrap()` in production code** — use `anyhow::Result`, `thiserror`, or explicit error handling. `unwrap()` is allowed only in test code.
- **No floating-point money** — all monetary values use integer pence (`_pence` suffix, `i64`)
- **No floating-point confidence** — all confidence/percentage use integer basis points (`_bps` suffix, 0-10000)
- **No mixed-write tables** — every table is either push-owned, remote-owned, or local-only

### Mandatory Per-Session Invariants

- Every agent session MUST get a cleanup verdict (`Clean` | `Forced` | `Leaked`)
- Every skill execution MUST produce a run manifest (agent narrative + daemon evidence appendix; fallback machine-authored manifest if agent fails to narrate)
- Every pipeline failure MUST produce an error manifest (structured Markdown with full runtime context)

### CI Tiers

| Tier | Budget | Cadence | Scope |
|------|--------|---------|-------|
| T1 | <90s | every commit | static gates, pure parser tests, mode resolution, skill lint |
| T2 | <4min | every commit | SQLite integration, IPC, PTY/session smoke, result contracts |
| T3 | <20min | nightly | crash recovery, chunk recovery, stubborn cleanup, captured-evidence canary |
| T4 | <30min | scheduled | live FTS smoke, live CLI canary, live adapter checks |
| Manual | operator | milestone | 10-run Experiment 4, live manifest inspection, zero-orphan on target host |

### Test Inventory

150 test IDs total across 10 test files:
- 6 static gates (`CI-CORE-001` through `CI-CORE-006`)
- 47 unit tests
- 57 integration tests
- 40 E2E / scheduled-manual tests

---

## 6. Source of Truth Documents

Read these IN THIS ORDER before starting implementation:

| # | Document | Path | Role |
|---|----------|------|------|
| 1 | **AGENTS.md** | `AGENTS.md` | Coding conventions, toolchain, br workflow, AgentMail coordination, session protocol. **READ FIRST.** |
| 2 | **Spec (SOT #1)** | `docs/strategy/layers/layer-1-thin-local-runner.md` | WHAT to build — full daemon spec with 138 `L1-FR-*` requirement IDs |
| 3 | **TDD (SOT #2)** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md` | HOW to verify — 150 test IDs with exact assertions and tier assignments |
| 4 | **Strategy (SOT #3)** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md` | WHAT ORDER — 10-phase dependency DAG with gate criteria |
| 5 | **Bootstrap** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md` | Environment setup, Cargo.toml templates, test workspace layout |
| 6 | **L0 Schema** | `docs/strategy/layers/layer-0-gold-set-evaluation.md` §2 | DDL for `0001_sqlite_state_initial.sql` — the schema Layer 1 builds on |
| 7 | **Layer Strategy** | `docs/strategy/layer-strategy.md` | Cross-layer contracts, schema stability matrix, interface contracts |
| 8 | **Manifest** | `docs/pipeline/l1-daemon-core/manifest.yaml` | All paths, key types, bead prefix, tech stack summary |

### Phase Ordering (From Strategy)

```
Phase 1  Workspace + SQLite
   |
   v
Phase 2  Walking skeleton startup
   |
   v
Phase 3  PTY spawn + capture
   |  \
   |   +-> Phase 4  Health + reaper
   |
   +-----> Phase 5  Adapter trait + {Codex, Claude}
                |
                v
           Phase 6  {Artifacts, Contracts}
                \      /
                 v    v
          Phase 7  Runner commit/stage tracking
             |  \
             |   +-> Phase 8  Error manifests
             |
             +-----> Phase 9  Prompt + skills + canary
               \        /
                v      v
           Phase 10  Full live loop + Experiment 4
```

The first ready bead is **L1-01: Workspace Bootstrap** (`testproject1-63g.1`). Start there.
