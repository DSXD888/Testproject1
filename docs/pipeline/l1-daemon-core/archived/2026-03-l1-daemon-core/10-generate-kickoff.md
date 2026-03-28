# L1 Daemon Core — Generate Kickoff + Push Prompts

## First Action

Invoke the `/beads-to-execution` skill.

---

## Your Mission

Generate the **kickoff prompt** (full agent onboarding), **push prompt** (continuation nudge), and **status file** for Layer 1 Daemon Core implementation. These are the documents agents will actually USE during implementation.

The kickoff prompt must onboard a fresh Rust implementation agent to this greenfield project — explain the workspace structure, the daemon architecture, the PTY/contract adapter model, the 17-file crate layout, the testing strategy, and point to the beads for work items.

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
docs/strategy/layers/layer-1-thin-local-runner.md                   — Spec (SOT #1)
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md        — TDD (SOT #2)
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md  — Bootstrap (environment setup)
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md   — Strategy (SOT #3)
```

### Codebase (study for integration surface)

```
AGENTS.md                                            — Agent conventions, beads workflow, toolchain, AgentMail section, session protocol
docs/strategy/layers/layer-0-gold-set-evaluation.md  — L0 schema (§2) the daemon builds on
docs/strategy/layer-strategy.md                      — Cross-layer contracts
docs/pipeline/l1-daemon-core/manifest.yaml           — All paths and metadata
```

---

## Output Files

| File | Path | Purpose |
|------|------|---------|
| Kickoff | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-kickoff.md` | Full agent onboarding (~200-350 lines) |
| Push | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-push.md` | Continuation nudge (~60-90 lines) |
| Status | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-status.yaml` | YAML pointer to all SOT docs |

---

## Required Kickoff Sections (ALL mandatory, in this order)

### 1. Mission + Organization Model

- Feature description: Layer 1 thin Rust daemon core — PTY, process supervision, agent adapters, artifact capture, stage tracking
- Completion milestones: Experiment 4 (10 consecutive batch runs, >=90% success, 0 orphans) + first live operational loop
- **Organization: FLAT** — no team hierarchy, pick next ready bead
- **Coordination: AGENTMAIL-FIRST** — register, check inbox, announce claims BEFORE touching code
- **Beads are SELF-CONTAINED** — implement from bead alone, spec as tiebreaker only

### 2. Source of Truth Reading List

Ordered list with READ/SKIM/REFERENCE annotations. Must include:
1. `AGENTS.md` — **READ FIRST** (coding conventions, toolchain, br workflow, AgentMail coordination, session protocol)
2. Spec (SOT #1) — `docs/strategy/layers/layer-1-thin-local-runner.md` — READ for orientation
3. TDD (SOT #2) — `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md` — REFERENCE (test IDs per bead)
4. Bootstrap — `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md` — REFERENCE (environment setup)
5. Strategy (SOT #3) — `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md` — SKIM (phase ordering)
6. L0 Schema — `docs/strategy/layers/layer-0-gold-set-evaluation.md` §2 — REFERENCE (DDL)
7. Layer Strategy — `docs/strategy/layer-strategy.md` — REFERENCE (cross-layer contracts)
8. Manifest — `docs/pipeline/l1-daemon-core/manifest.yaml` — REFERENCE (all paths, key types)

### 3. AgentMail Registration (exact MCP calls)

Must include BOTH options:

**Option A: Macro (fast):**
```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l1-daemon-executor",
  model="<your-model>"
)
```

**Option B: Granular:**
```
ensure_project(human_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(project_key, program="l1-daemon-executor", model="<your-model>", task_description="Implementing L1 daemon core beads")
```

### 4. Situational Awareness (check before picking work)

Must include these exact calls:
```
fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="<your-name>", include_bodies=true)
search_messages(project_key="/Users/alinaaitova/Desktop/Testproject1", query="in_progress OR claimed")
```
```bash
br ready --json    # Current work queue
```
KEY PRINCIPLE: If another agent claimed a bead, DO NOT take it. If inbox has coordination messages, act on them FIRST.

### 5. Claim Protocol (announce + reserve)

Full protocol with exact MCP calls:
```bash
br update <id> --status=in_progress
```
```
file_reservation_paths(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  paths=["crates/daemon/src/<target-files>"],
  ttl_seconds=3600,
  exclusive=true,
  reason="br-testproject1-63g.N"
)
```
```
send_message(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  sender_name="<your-name>",
  to=["broadcast"],
  thread_id="br-testproject1-63g.N",
  subject="[br-testproject1-63g.N] Claimed: <bead-title>",
  body="Starting implementation. Files reserved: <list>",
  ack_required=true
)
```
FILE_RESERVATION_CONFLICT handling: pick a different bead, do NOT wait.

### 6. Beads ↔ AgentMail Mapping

| Concept | Value |
|---------|-------|
| Mail `thread_id` | Bead ID (e.g., `br-testproject1-63g.N`) |
| Mail subject prefix | `[br-testproject1-63g.N]` |
| File reservation `reason` | Bead ID |
| Commit message prefix | Bead ID |

### 7. Quality Gates (actual commands)

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test
```

Constitutional constraints:
- `#![forbid(unsafe_code)]` — no exceptions
- No `unwrap()` in production code
- Every session must get a cleanup verdict
- Every skill run must produce a run manifest

### 8. Execution Loop

The full cycle, step by step:
1. **Claim** — `br update <id> --status=in_progress`
2. **Reserve files** — `file_reservation_paths(...)` with exclusive=true
3. **Announce** — `send_message(...)` with thread_id=bead-id
4. **Implement** — read bead description, write code + tests
5. **Verify** — run quality gates (build + lint + format + test)
6. **Commit** — `br close <id>`, `br sync --flush-only`, `git add`, `git commit -m "feat(daemon): ... br-testproject1-63g.N"`
7. **Push** — `git pull --rebase && git push`
8. **Release reservations** — `release_file_reservations(project_key, agent_name)`
9. **Announce completion** — `send_message(...)` with subject "[br-...] Completed"
10. **Loop** — back to situational awareness (check inbox, `br ready`, pick next)

### 9. Session Landing Protocol

When fully blocked (no ready beads, all remaining beads depend on other agents' work):
1. Push all committed work: `git pull --rebase && git push`
2. Release ALL file reservations: `release_file_reservations(project_key, agent_name)`
3. Announce handoff via mail: `send_message(...)` with subject "Session landing — blocked on [bead-ids]" and body listing what's done, what's blocked, and why
4. Do NOT leave file reservations active — they block other agents

---

## Required Push Sections (ALL mandatory)

The generated push prompt MUST include these sections:

1. **"Re-read AGENTS.md"** — first instruction, before anything else
2. **Project key** — `/Users/alinaaitova/Desktop/Testproject1`
3. **"Finish in-progress work first"** — if you have an in-progress bead, complete it before picking new work
4. **"Check inbox"** with exact `fetch_inbox` call:
   ```
   fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="<your-name>", include_bodies=true)
   ```
5. **"Run `br ready`"** — pick highest-priority unblocked bead with prefix `testproject1-63g.*`
6. **Full claim protocol** — update status → reserve files → announce → implement → verify → commit → push → release → announce completion → loop
7. **Quality gate commands:**
   ```bash
   cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
   ```
8. **"Do NOT stop to ask what to work on"** — run `br ready` and continue
9. **Session landing protocol** — when blocked: push all work, release all reservations, announce handoff via mail

---

## Kickoff Content Requirements (additional L1-specific sections)

In addition to the 9 mandatory sections above, the kickoff should also include:

### Project Context (50-80 lines)

- What this project is (UK procurement intelligence platform)
- What Layer 1 is (thin Rust daemon core — PTY, process supervision, agent adapters, artifact capture, stage tracking)
- What Layer 0 provides (TypeScript intelligence pipeline, SQLite schema, skills, evaluation harness)
- What Layer 1 does NOT include (no desktop, no sync, no cron, no validation enforcement)

### Architecture Overview (30-50 lines)

- 17-file daemon crate structure with brief descriptions
- Two-lane adapter model (OutputFrame + ControlEvent)
- Session lifecycle (7 phases)
- Single-database model (state.db + filesystem artifact store)
- Commit fence (commit_seq / visibleAfterSeq)

### Technical Stack (20-30 lines)

- Rust 2024 (nightly), Tokio, rusqlite, nix, portable-pty
- `#![forbid(unsafe_code)]` — constitutional constraint
- Build/lint/test/format commands

### Phase Ordering (from Strategy)

- The 10-phase dependency DAG
- First ready bead pointer

---

## NO sub-optimization — these ARE the deliverables.
