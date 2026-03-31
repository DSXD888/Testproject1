# L3 Operational Hardening — Generate Kickoff + Push Prompts

## First Action

Invoke the `/beads-to-execution` skill.

---

## Your Mission

Generate the **kickoff prompt** (full agent onboarding), **push prompt** (continuation nudge), and **status file** for L3 Operational Hardening.

These are the documents agents will actually USE during implementation. They must be deeply feature-specific — reference actual file paths, actual types, actual commands.

**CRITICAL: The kickoff and push prompts MUST include the full AgentMail coordination workflow.** Assume multiple agents may work in parallel. The prompts must leave ZERO ambiguity about coordination.

---

## Source Documents

```
docs/strategy/layers/layer-3-operational-hardening.md    — Spec (SOT #1)
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md — TDD (SOT #2)
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md — Bootstrap
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md — Strategy (SOT #3)
AGENTS.md                                                — Project-wide agent rules
```

### Codebase (study for integration surface)
```
crates/daemon/src/main.rs                                — Daemon entry, module declarations (131K)
crates/daemon/src/jobs/runner.rs                         — JobRunner central type (25K)
crates/daemon/src/sessions/reaper.rs                     — SessionReaper (17K)
crates/daemon/src/agents/artifacts.rs                    — Run manifest types (15K)
crates/daemon/src/agents/contracts.rs                    — Validation types (9.2K)
crates/daemon/src/errors/manifest.rs                     — Error codes (14K)
crates/daemon/src/pty/session.rs                         — PTY session (15K)
crates/daemon/src/agents/adapter.rs                      — AgentAdapter trait (3.7K)
crates/daemon/src/db/sqlite.rs                           — StateDb (9.3K)
crates/daemon/src/ipc/command_router.rs                  — IPC dispatch (9.2K)
crates/daemon/Cargo.toml                                 — Dependencies
packages/intelligence/src/qualification/goods-qualification.ts — TS pattern
packages/manifests/render-opportunity.ts                 — TS rendering pattern
```

### Quality Bar (MATCH THIS LEVEL OF DETAIL)
```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-kickoff.md — Best existing kickoff
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-push.md    — Best existing push prompt
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-kickoff.md — L2 kickoff reference
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-push.md    — L2 push reference
```

---

## Output Files

| File | Path | Purpose |
|------|------|---------|
| Kickoff | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-kickoff.md` | Full agent onboarding (~150-300 lines) |
| Push | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-push.md` | Continuation nudge (~30-50 lines) |
| Status | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-status.yaml` | YAML pointer to all SOT docs |

---

## Required Kickoff Sections (ALL mandatory)

### 1. Mission + Organization Model
- L3 Operational Hardening: cron scheduling, resource management, PTY hardening, relation resolution, source health, skill promotion, worker-validator dry-run
- **Organization: FLAT** — no team hierarchy, pick next ready bead
- **Coordination: AGENTMAIL-FIRST** — register, check inbox, announce claims BEFORE touching code
- **Beads are SELF-CONTAINED** — implement from bead alone, spec as tiebreaker only
- Exit criteria: System runs unattended 5 consecutive days with >=95% skill success rate

### 2. Source of Truth Reading List
- `AGENTS.md` — **READ FIRST** (project rules, toolchain, session protocol)
- `docs/strategy/layers/layer-3-operational-hardening.md` — Spec (SOT #1) — **READ FULLY**
- `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — TDD (SOT #2) — **SKIM**
- `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md` — Bootstrap — **SKIM**
- `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Strategy (SOT #3) — **SKIM**
- Key codebase files: `crates/daemon/src/main.rs`, `crates/daemon/src/jobs/runner.rs`, `crates/daemon/src/agents/artifacts.rs`

### 3. AgentMail Registration (exact MCP calls)
```
ensure_project(human_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(project_key, program="claude-code", model="claude-opus-4-6", task_description="L3 Operational Hardening implementation")
```
Or macro: `macro_start_session(project_key, program, model, task_description)`

### 4. Situational Awareness (check before picking work)
```
fetch_inbox(project_key, agent_name, include_bodies=true)
search_messages(project_key, query="l3 in_progress OR claimed")
br ready --json  # Current work queue
```
KEY PRINCIPLE: if another agent claimed a bead, DO NOT take it.

### 5. Claim Protocol (announce + reserve)
```bash
br update <id> --status=in_progress
```
```
file_reservation_paths(project_key, agent_name, paths=["crates/daemon/src/validation/**"], ttl_seconds=3600, exclusive=true, reason="<bead-id>")
send_message(project_key, sender_name, to=["all"], subject="[<bead-id>] Claimed: ...", ack_required=true)
```
FILE_RESERVATION_CONFLICT handling: if conflict, pick a different bead.

### 6. Beads <-> AgentMail Mapping
| Concept | Value |
|---------|-------|
| Mail `thread_id` | Bead ID (e.g., `br-123`) |
| Mail subject prefix | `[br-123]` |
| File reservation `reason` | Bead ID |
| Commit message prefix | Bead ID |

### 7. Quality Gates (actual commands)
```bash
# Rust
cargo check --all-targets
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# TypeScript (only if TS files changed)
bun run build
bun run test
bun run lint

# UBS (before commit)
ubs $(git diff --name-only --cached)
```

### 8. Execution Loop
Claim → Reserve files → Announce → Implement → Verify (quality gates) → Commit → Push → Release reservations → Announce completion → bin/br-sync → Loop

### 9. Session Landing Protocol
When fully blocked:
1. Push all work: `git add . && git commit && git push`
2. Release all reservations: `release_file_reservations(project_key, agent_name)`
3. Sync beads: `bin/br-sync && git add .beads/ && git commit && git push`
4. Announce handoff via mail with context for next agent

---

## Required Push Sections (ALL mandatory)

1. "Re-read AGENTS.md"
2. Project key: `/Users/alinaaitova/Desktop/Testproject1`
3. "Finish in-progress work first"
4. "Check inbox": `fetch_inbox(project_key, agent_name, include_bodies=true)`
5. "Run `br ready`" with l3 bead prefix filter
6. Full claim protocol (update status → reserve files → announce → implement → verify → commit → push → release → loop)
7. Quality gate commands (cargo check/test/clippy + bun build/test/lint)
8. "Do NOT stop to ask what to work on"
9. Session landing protocol when blocked

---

## NO sub-optimization — these ARE the deliverables.
