# Autonomous Daemon Scheduling — Step 10: Generate Kickoff + Push Prompts

## First Action

Invoke the `/beads-to-execution` skill.

---

## Your Mission

Generate the **kickoff prompt** (full agent onboarding), **push prompt** (continuation nudge), and **status file** for autonomous daemon scheduling. These are the documents agents will actually USE during implementation — deeply feature-specific, referencing actual file paths (manifest `key_files`), actual commands (the full Stage Validation Test Gate), and the AgentMail coordination workflow.

**CRITICAL:** The kickoff and push MUST include the full AgentMail coordination workflow. Multiple agents may work in parallel. Leave ZERO ambiguity about: FLAT organization, AgentMail registration, inbox checks, claiming (via `bin/br-claim`), file-reservation conflicts, the full execution loop, session landing, the bead completion policy (fully complete every bead — no partial work), and the commit workflow (complete → verify → commit with bead-ID prefix → push → `bin/br-complete`).

---

## Source Documents
```
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md       — SOT #1
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md        — SOT #2
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-bootstrap.md  — environment setup
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-strategy.md   — SOT #3 (ordering)
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-architecture.md — thread model + WAL correctness
docs/pipeline/autonomous-daemon-scheduling/manifest.yaml                                         — key_files, quality_gates, agentmail, bead_label
AGENTS.md                                                                                        — Rule 2 wrappers, Stage Validation Test Gate, toolchain
```

### Codebase (study for integration surface — verified file:lines in manifest `key_files`)
```
crates/daemon/src/main.rs:3260 (SyncLeaseHeartbeat::spawn — the thread pattern), :1703 (run_next_workflow_job), :3697 (enqueue_startup_cron_jobs — folded), :3819 (serve loop), :23323 (run_deterministic_pipeline — the in-process precedent)
crates/daemon/src/scheduler/cron.rs:120-397; jobs/runner.rs:580/1626; jobs/claim.rs:72/176; agents/arbiter.rs:14/82-107; db/sqlite.rs:89/215-218
```

### Quality Bar (MATCH THIS LEVEL OF DETAIL — study the AgentMail + work-loop sections)
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-kickoff.md   (770 lines — best existing kickoff)
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-push.md
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-kickoff.md
```

---

## Output Files
| File | Path | Purpose |
|------|------|---------|
| Kickoff | `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-kickoff.md` | Full agent onboarding (~250–400 lines) |
| Push | `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-push.md` | Continuation nudge (~80–120 lines) |
| Status | `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-status.yaml` | YAML pointer to all SOT docs |

---

## Required Kickoff Sections (ALL mandatory — in this order)

### 1. Mission
- WHAT: make the daemon fire its scheduled scraping runs autonomously while it sits open (no quit-and-reopen, no manual clicks), drain the queue end-to-end, harden ProContract resumption, and auto-advance the commercial stages. WHY: today the schedule is evaluated only once at bootstrap and no worker drains the cron kinds (research §2.2).
- Scope: `crates/daemon/` (Rust runtime); a skill-side touch in `skills/procurement-ingestion/procontract-enrich-tender/`; optional minor `apps/desktop/` touch (Q7).
- Milestones: G1 tick → G2 worker → G6 commercial sweep → G4 ProContract resumption → G5 soak.
- "Work from the bead graph. Claim task beads, not epics. Each bead is self-contained — implement from the bead alone."

### 2. Implementation Boundary
- **EXISTS (build on top of):** the cron scheduler (`scheduler/cron.rs` — `dispatch_due_jobs`), the job queue + `JobRunner` (`jobs/runner.rs`, `jobs/claim.rs`), the resource arbiter (`agents/arbiter.rs`), all live-loop scrapers (`run_*_live_loop` in `main.rs`), the `SyncLeaseHeartbeat` background-thread pattern (`main.rs:3260`), WAL + busy_timeout (`sqlite.rs:215-218`), `run_deterministic_pipeline` (`main.rs:23323`). These RUN today via `--ingest`/`--run-next-job`/`--pipeline` CLI flags.
- **DOES NOT EXIST (you're building):** the recurring scheduler tick thread; the always-on two-lane worker; handlers for source_health/auth_refresh/validation_*/commercial_sweep; the queue-driven shared source body (refactor); the `--run-scheduler-once --tick-now` entrypoint; the ProContract awaiting-access tracking + work-list sweep; the 07:00 commercial_sweep slot.
- "Do NOT assume unbuilt infrastructure exists. The scrapers exist; the autonomy layer does not."

### 3. Core Contracts / Non-Negotiables
- The scheduler tick and worker run on SEPARATE threads — a long PTY run must never block the tick.
- The tick body takes an injected `now` — NO inline `Utc::now()` (clock seam is a design requirement).
- Two lanes by job class: PTY/browser (sync_source/source_health/auth_refresh) + coverage/deterministic (commercial_sweep/validation_*).
- Defer-on-denial: denied admission → `pending`, NO attempt_count bump, NO held lease, backoff before re-claim.
- `procurement_source_lanes=1` serializes same-source runs (R7) — never run two concurrent same-source scrapes.
- §4.4 landmine: NEVER overwrite `interest_registration.state` (stays `'completed'`).
- `commercial_sweep` runs IN-PROCESS on the coverage lane — NEVER call `trigger_pipeline_rerun`.
- Schema-epoch coupling: if Q4=table, bump epoch 10→11 across ALL constants (manifest `schema_epoch_constants`) in ONE commit, or desktop reads fail wholesale.
- `#![forbid(unsafe_code)]`; no backwards-compat shims (AGENTS.md); no new files unless genuinely new functionality.
- "If code conflicts with this contract, the contract wins."

### 4. Source of Truth Loading (MANDATORY — before any implementation)
Ordered list with READ/SKIM/REFERENCE: AGENTS.md (Rule 2 + Stage Gate), then spec, TDD, strategy, the bead graph (`br ready --label autonomous-scheduling --json`), then the key codebase files.

### 5. AgentMail Setup (Do This ONCE At Session Start)
```
ensure_project(project_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(project_key, program="claude-code", model="opus")   # export AGENT_MAIL_NAME
set_contact_policy(project_key, agent_name, policy="open")
# Bootstrap contact handshakes before any bin/br-claim:
#   list_agents(project_key) → same-program live peers →
#   macro_contact_handshake(project_key, requester=agent_name, target=peer["name"])
#   verify each appears in list_contacts(project_key, agent_name)
fetch_inbox(project_key, agent_name, limit=20)
```
"Read and acknowledge any pending messages before starting work."

### 6. Bead Completion Policy (CRITICAL — include verbatim)
Include the standard verbatim policy: use `bin/br-claim <id>` / `bin/br-complete <id>` ONLY (never raw `br update`/`br close` — AGENTS.md Rule 2 constitutional; precommit hook enforces). MUST fully complete every claimed bead (no partial work, no mid-way commits, no "finish later"). A bead is COMPLETE when: all code implemented, all tests pass, the FULL per-file gate passes, committed with `<bead-id>:` prefix AND pushed, `bin/br-complete` ran clean. On blocker: diagnose + fix yourself; only escalate via AgentMail (reply in the bead thread, file:line) if the blocker is in another agent's code; do NOT release reservations while blocked.
- **Single-agent runs:** if the operator runs ONE agent (no peers), `bin/br-claim`/`bin/br-complete` accept `--solo` (or `AGENT_MAIL_ALLOW_SOLO=1`) for legitimate solo operation — it still emits a loud warning rather than acting silently (AGENTS.md Rule 2). The wrappers remain mandatory either way; `--solo` is NOT a bypass of reservations, only of peer-mail when there are genuinely no peers.

### 7. Work Loop (Full Visual — claim through release)
Generate the full ASCII work-loop diagram (PICK → CLAIM via `bin/br-claim` → IMPLEMENT TDD → VERIFY → COMMIT+PUSH → COMPLETE via `bin/br-complete` → LOOP). Use the project's actual commands (below).

### 8. Verification Commands (the FULL Stage Validation Test Gate — verbatim)
```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon                      # binary unit tests (INSUFFICIENT alone)
for t in crates/daemon/tests/*.rs; do                  # EACH integration file is its own target
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
# (only if Q7 desktop surfacing is taken:)  bun run build && bun run test && bun run lint
```
"`cargo test -p daemon --bin daemon` ALONE is NOT green — it silently skips every integration file (~61+ `.rs` targets), including the net-new scheduler_tick.rs / worker_pump.rs / procontract_resumption.rs. The `for` loop is mandatory (AGENTS.md Stage Validation Test Gate). Use debug builds for fast iteration."

### 9. File-Surface Map by Epic/Lane
Per phase/sub-epic, the specific files in scope (from manifest `key_files`): Foundations → main.rs (refactor sites), clock-seam entrypoint; Tick → main.rs (new thread) + scheduler/cron.rs; Worker → main.rs + jobs/{runner,claim}.rs + agents/arbiter.rs; Commercial → main.rs (commercial passes) + scheduler/cron.rs + sources/registry.rs; G4 → main.rs (interest/enrich) + migrations/0015_* + skills/.../procontract-enrich-tender/ + epoch constants; Tests → crates/daemon/tests/{scheduler_tick,worker_pump,procontract_resumption}.rs.

### 10. Dependency Spine
Visual: Foundations (Q1, clock seam, refactor) → Tick (G1) → Worker+handlers (G2) → {Commercial sweep (G6), ProContract resumption (G4)} → Concurrency soak (G5) → Desktop (Q7, optional). "Trust `br ready`, but use this spine as your mental model." (Per AGENTS.md, `bv` is available for graph-aware triage if you want a richer ready-set view than `br ready`; either works.)

### 11. AgentMail Quick Reference
Include the standard compact reference: NO broadcast; `bin/br-claim`/`bin/br-complete` handle Start/Complete mail + reservations; discover peers via `list_contacts`; `send_message` only for blockers/questions/handoffs; `fetch_inbox` between beads.

---

## Required Push Sections (ALL mandatory)
1. Re-Orient (re-read AGENTS.md; feature 1-liner; spec/strategy paths; `br ready --label autonomous-scheduling --json`)
2. Core Contracts (compact: separate threads, clock seam, two lanes, defer-on-denial no-attempt-bump, source-lane-1 serialization, landmine, no-trigger_pipeline_rerun, epoch coupling)
3. Bead Completion Policy (compact but firm — wrappers only; full completion; full gate; push before complete)
4. Work Loop (compact 9-step)
5. Verification Commands (the full per-file gate, compact)
6. Dependency Spine (one line per phase)
7. File Surfaces (compact table)
8. AgentMail Quick Reference (compact)

---

## Status File (`autonomous-daemon-scheduling-status.yaml`)
Mirror the goods-led status format: feature, status: in_progress, SOT doc paths, implementation_epic (from step 07), bead_label: autonomous-scheduling, total_beads, schema_epoch (10, or 11 if Q4=table), and a `gates` block referencing the full Stage Validation Test Gate.

---

## NO sub-optimization — these ARE the deliverables.

After generating, verify: (a) the full per-file test gate appears verbatim in BOTH kickoff and push; (b) the §4.4 landmine and the no-`trigger_pipeline_rerun` contract appear in both; (c) `bin/br-claim`/`bin/br-complete` are the only claim/complete paths shown; (d) the clock-seam/one-shot-entrypoint is called out as the test harness; (e) project_key is the absolute path `/Users/alinaaitova/Desktop/Testproject1`.
