# L2 Desktop MVP — Generate Kickoff + Push Prompts

## First Action

Invoke the `/beads-to-execution` skill.

---

## Your Mission

Generate the **kickoff prompt** (full agent onboarding), **push prompt** (continuation nudge), and **status file** for L2 Desktop MVP.

These are the documents agents will actually USE during implementation. They must be deeply feature-specific — reference actual file paths, actual types, actual commands, and the full AgentMail coordination workflow.

**CRITICAL: The kickoff and push prompts MUST include the full AgentMail coordination workflow.** Assume multiple agents may work in parallel. The prompts must leave ZERO ambiguity about:
- Organization model (FLAT — no hierarchy, agents pick next ready bead)
- How to register with AgentMail (exact MCP calls)
- How to check inbox before picking work (situational awareness)
- How to claim a bead (update status + reserve files + announce via mail)
- How to handle file reservation conflicts
- The full execution loop (claim → reserve → announce → implement → verify → commit → push → release → loop)
- Session landing protocol (when fully blocked)

---

## Source Documents

```
docs/strategy/layers/layer-2-single-operator-desktop.md                — Spec (SOT #1)
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md          — TDD (SOT #2)
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md     — Bootstrap (environment setup)
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md      — Strategy (SOT #3)
docs/pipeline/l2-desktop-mvp/manifest.yaml                             — All paths, key types, bead prefix
AGENTS.md                                                               — Agent coordination, toolchain, br workflow
```

### Codebase (study for integration surface)
```
crates/daemon/src/main.rs                  — Daemon entry point (3,816 lines)
crates/daemon/src/ipc/handshake.rs         — IPC protocol the desktop will use
crates/daemon/src/agents/adapter.rs        — AgentAdapter, ControlEvent, OutputFrame
crates/daemon/src/db/sqlite.rs             — commit_clock, StateDb
crates/daemon/src/jobs/runner.rs           — Job Runner (1,600 lines)
crates/daemon/src/pty/session.rs           — SessionPhase, PTY lifecycle
crates/daemon/src/sessions/reaper.rs       — CleanupVerdict
crates/daemon/src/agents/artifacts.rs      — Artifact store structure
migrations/0001_sqlite_state_initial.sql   — Full L0 schema
migrations/0003_sqlite_runtime.sql         — L1 additions
skills/procurement-ingestion/fts/          — FTS skill (reference for CF skill)
```

### Quality Bar (MATCH THIS LEVEL OF DETAIL)
```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-kickoff.md       — L1 kickoff (477 lines, full AgentMail section)
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-push.md         — L1 push (217 lines)
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-status.yaml     — L1 status (pipeline completion snapshot)
```

---

## Output Files

| File | Path | Purpose |
|------|------|---------|
| Kickoff | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-kickoff.md` | Full agent onboarding (~200-400 lines) |
| Push | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-push.md` | Continuation nudge (~30-50 lines) |
| Status | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-status.yaml` | YAML pointer to all SOT docs |

---

## Required Kickoff Sections (ALL mandatory)

### 1. Mission + Organization Model
- Feature description: Layer 2 is the first user-facing layer — a Tauri v2 desktop app with shared UI components, live streaming, decision capture, and a new Contracts Finder ingestion skill
- Deliverable: Working desktop control panel at `apps/desktop/` with ~28+ files, shared packages at `packages/ui` + `packages/manifests` + `packages/intelligence`, and CF skill
- Key constraint: This is a MIXED-STACK layer (Rust + TypeScript). Both `cargo test` and `bun run test` / `bunx playwright test` must pass.
- **Organization: FLAT** — no team hierarchy, pick next ready bead
- **Coordination: AGENTMAIL-FIRST** — register, check inbox, announce claims BEFORE touching code
- **Beads are SELF-CONTAINED** — implement from bead alone, spec as tiebreaker only

### 2. What Layer 0 + Layer 1 Provide (Already Built)
- L0: SQLite schema (47 tables, all columns), TypeScript contracts-schema, intelligence pipeline logic, 5 skills, evaluation harness
- L1: Rust daemon crate (`crates/daemon/`) — IPC, PTY sessions, artifact capture, job runner, error manifests, commit fence
- Key L1 integration surface: IPC handshake protocol, commit_clock, ControlEvent/OutputFrame, SessionPhase, CleanupVerdict

### 3. What Layer 2 Does NOT Include
- Cron scheduling (Layer 3), Resource arbiter (Layer 3), Sync engine (Layer 4), Web portal (Layer 4), Multi-source search (Layer 5), Decision-grade landed cost (Layer 5), Bid automation (Layer 6)

### 4. Source of Truth Reading List
Ordered list with READ/SKIM/REFERENCE annotations:
1. `AGENTS.md` — **READ FIRST** — coding conventions, toolchain, br workflow, AgentMail
2. Spec — **READ for tiebreaker** — L2-FR-* requirement IDs
3. TDD — **REFERENCE** — test IDs and assertions
4. Strategy — **REFERENCE** — phase ordering
5. Bootstrap — **REFERENCE** — environment setup
6. Manifest — **REFERENCE** — all paths and types
7. Key codebase files (list from integration surface above)

### 5. AgentMail Registration (exact MCP calls)
- `ensure_project(human_key="/Users/alinaaitova/Desktop/Testproject1")`
- `register_agent(project_key, program="l2-desktop-executor", model="<your-model>", task_description="Implementing L2 desktop MVP beads")`
- Or `macro_start_session(project_key="/Users/alinaaitova/Desktop/Testproject1", program="l2-desktop-executor", model="<your-model>")`

### 6. Situational Awareness (check before picking work)
- `fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="<your-name>", include_bodies=true)`
- `search_messages(project_key="/Users/alinaaitova/Desktop/Testproject1", query="in_progress OR claimed")`
- `br ready --json` — current work queue
- KEY PRINCIPLE: if another agent claimed a bead, DO NOT take it

### 7. Claim Protocol (announce + reserve)
- `br update <id> --status=in_progress`
- `file_reservation_paths(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="<your-name>", paths=[...], ttl_seconds=3600, exclusive=true, reason="<bead-id>")`
- `send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name="<your-name>", to=["<peers>"], thread_id="<bead-id>", subject="[<bead-id>] Claimed: ...", ack_required=true)`  # from list_contacts — NEVER "broadcast"
- FILE_RESERVATION_CONFLICT handling: pick a different ready bead

### 8. Beads ↔ AgentMail Mapping
| Concept | Value |
|---------|-------|
| Mail `thread_id` | Bead ID |
| Mail subject prefix | `[<bead-id>]` |
| File reservation `reason` | Bead ID |
| Commit message prefix | Bead ID |

### 9. Architecture Overview
- Desktop architecture diagram (from §1.3)
- Three communication channels: Snapshot reads, Mutations/triggers, Live streams
- Key L1 types the desktop integrates with
- Contracts Finder skill structure

### 10. Quality Gates (actual commands)
```bash
# Rust gates (always)
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test

# TypeScript gates (when TS files changed)
bun install
bun run build
bun run lint
bun run test

# Playwright gates (when UI files changed)
bunx playwright test
```

### 11. Execution Loop
Claim → Reserve files → Announce → Implement → Verify (both Rust AND TS gates as applicable) → Commit → Push → Release reservations → Announce completion → Loop

### 12. Session Landing Protocol
When fully blocked: push all work, release all reservations, announce handoff via mail.

---

## Required Push Sections (ALL mandatory)

1. "Re-read AGENTS.md"
2. Project key: `/Users/alinaaitova/Desktop/Testproject1`
3. "Finish in-progress work first" (`br list --status=in_progress`)
4. "Check inbox" with exact `fetch_inbox` command
5. "Pull latest" (`git pull --rebase`)
6. "Run `br ready --json`" with bead prefix filter
7. Full claim protocol (update status → reserve files → announce → implement → verify → commit → push → release → loop)
8. Quality gate commands (BOTH Rust AND TypeScript)
9. "Do NOT stop to ask what to work on"
10. Session landing protocol when blocked

---

## NO sub-optimization — these ARE the deliverables.
