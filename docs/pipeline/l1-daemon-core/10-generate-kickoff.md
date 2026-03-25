# L1 Daemon Core — Generate Kickoff + Push Prompts

## First Action

Invoke the `/beads-to-execution` skill.

---

## Your Mission

Generate the **kickoff prompt** (full agent onboarding), **push prompt** (continuation nudge), and **status file** for Layer 1 Daemon Core implementation. These are the documents agents will actually USE during implementation.

The kickoff prompt must onboard a fresh Rust implementation agent to this greenfield project — explain the workspace structure, the daemon architecture, the PTY/contract adapter model, the 17-file crate layout, the testing strategy, and point to the beads for work items.

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
AGENTS.md                                            — Agent conventions, beads workflow, toolchain
docs/strategy/layers/layer-0-gold-set-evaluation.md  — L0 schema (§2) the daemon builds on
docs/strategy/layer-strategy.md                      — Cross-layer contracts
docs/pipeline/l1-daemon-core/manifest.yaml           — All paths and metadata
```

### Quality Bar

```
docs/layermaxxer/active/execution-kickoff.md         — Existing kickoff prompt for reference
docs/layermaxxer/active/execution-push.md            — Existing push prompt for reference
```

---

## Output Files

| File | Path | Purpose |
|------|------|---------|
| Kickoff | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-kickoff.md` | Full agent onboarding (~200-300 lines) |
| Push | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-push.md` | Continuation nudge (~40-60 lines) |
| Status | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-status.yaml` | YAML pointer to all SOT docs |

---

## Kickoff Content Requirements

The kickoff prompt must include:

### 1. Project Context (50-80 lines)

- What this project is (UK procurement intelligence platform)
- What Layer 1 is (thin Rust daemon core — PTY, process supervision, agent adapters, artifact capture, stage tracking)
- What Layer 0 provides (TypeScript intelligence pipeline, SQLite schema, skills, evaluation harness)
- What Layer 1 does NOT include (no desktop, no sync, no cron, no validation enforcement)

### 2. Architecture Overview (30-50 lines)

- 17-file daemon crate structure with brief descriptions
- Two-lane adapter model (OutputFrame + ControlEvent)
- Session lifecycle (7 phases)
- Single-database model (state.db + filesystem artifact store)
- Commit fence (commit_seq / visibleAfterSeq)

### 3. Technical Stack (20-30 lines)

- Rust 2024 (nightly), Tokio, rusqlite, nix, portable-pty
- `#![forbid(unsafe_code)]` — constitutional constraint
- Build: `cargo check --all-targets`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Test: `cargo test`
- Format: `cargo fmt --check`

### 4. Beads Workflow (30-40 lines)

- How to find work: `br ready`
- How to claim: `br update <id> --status=in_progress`
- How to complete: `br close <id>`
- How to sync: `br sync --flush-only && git add .beads/`
- File reservations via AgentMail
- Commit message convention: include `br-###`

### 5. Quality Gates (20-30 lines)

- Every bead must pass: `cargo check && cargo clippy && cargo test <bead_tests>`
- No `unsafe` code
- No `unwrap()` in production code (use `anyhow::Result` or `thiserror`)
- Mandatory: process-group cleanup verdict for every session
- Mandatory: run manifest for every skill execution

### 6. SOT Document Pointers (10-20 lines)

- Spec: `docs/strategy/layers/layer-1-thin-local-runner.md`
- TDD: `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md`
- Strategy: `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md`
- Bootstrap: `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md`
- AGENTS.md: coding conventions, toolchain, git workflow

---

## Push Content Requirements

The push prompt is a concise continuation nudge (~40-60 lines):

1. Check inbox for messages from other agents
2. Check beads: `br ready` — pick the highest-priority unblocked bead
3. If current bead is stuck, check dependencies and file reservations
4. Build gate: `cargo check --all-targets && cargo clippy --all-targets -- -D warnings`
5. Test gate: `cargo test <bead_specific_tests>`
6. On completion: `br close <id>`, `br sync --flush-only`, commit and push
7. Pick next bead. Do NOT stop to ask what to work on.

---

## NO sub-optimization — these ARE the deliverables.
