# L1 Daemon Core — Create Beads

## First Action

Invoke the `/beads-from-sources` skill.

---

## Your Mission

Create **self-contained executable beads** for Layer 1 implementation. Each bead embeds spec and TDD content verbatim so an implementation agent can execute it without opening other documents. The strategy provides ordering — beads provide the actual work units.

---

## Source Documents (Read In This Order)

### Source of Truth Documents (ALL THREE REQUIRED)

```
docs/strategy/layers/layer-1-thin-local-runner.md                   — Spec (SOT #1) — READ FULLY
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md        — TDD (SOT #2) — READ FULLY
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md   — Strategy (SOT #3) — READ FULLY
```

### Codebase Context

```
AGENTS.md                                            — Beads workflow (br CLI), coding discipline, toolchain
docs/pipeline/l1-daemon-core/manifest.yaml           — All file paths, bead prefix, tech stack
```

{ENRICHMENT: This section will be updated after Batches 1-3 with actual requirement IDs, test IDs, and phase names from the created artifacts. For now, read the artifacts directly.}

---

## Key Context From Codebase Research

- **Bead prefix:** `l1`
- **Bead CLI:** `br`
- **Expected bead count:** 15-25 (based on 17 daemon files + 2 skills + 1 migration + 10 test files grouped into 8-12 strategy phases)
- **Each bead must be self-contained:** Embed the spec section verbatim, list the TDD test IDs that verify it, specify the exact files to create/modify, and include the acceptance criteria inline
- **Dependency tracking:** Use `br dep add <bead> <depends-on>` to mirror the strategy phase dependencies
- **Labels:** Use subsystem labels matching the requirement ID prefixes: `pty`, `adapter`, `runner`, `artifacts`, `errors`, `ipc`, `reaper`, `db`, `daemon`, `skills`, `pipeline`

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)

1. `docs/strategy/layers/layer-1-thin-local-runner.md` — **READ FULLY**
2. `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md` — **READ FULLY**
3. `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md` — **READ FULLY**
4. `AGENTS.md` — **SKIM** (beads workflow)

### Step 1: Invoke the Skill

Run `/beads-from-sources`.

**Inputs:**
- Spec: `docs/strategy/layers/layer-1-thin-local-runner.md`
- TDD: `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md`
- Strategy: `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md`
- Bead prefix: `l1`
- Bead CLI: `br`

**Key arguments/context:**
- Each bead must embed spec and TDD content VERBATIM — no paraphrasing
- Strategy provides phase ordering → bead dependencies
- Files per bead: list the exact Rust source files and test files to create
- Build/test commands per bead: `cargo check --all-targets` + `cargo test <specific_test>`
- Each bead's acceptance criteria come directly from the spec's ACs for the requirement IDs it covers

### Step 2: Feature-Specific Guidance

While the skill runs, pay attention to:

- **Foundation beads (workspace + DB + migration) must be very specific** about:
  - Exact `Cargo.toml` contents (workspace members, daemon crate dependencies)
  - `rust-toolchain.toml` contents (channel = "nightly", edition = "2024")
  - Migration file structure and L0 schema dependency

- **PTY beads need the test helper binary defined.** Include the specification for a minimal test-agent binary (a simple program that accepts commands via stdin and responds, can be SIGTERM'd, can fork grandchildren on command).

- **Adapter beads should create the trait FIRST in its own bead**, then Codex and Claude adapters in separate beads (parallelizable).

- **The integration bead (full pipeline loop) should be the LAST bead** and depends on ALL prior beads. Its acceptance criteria map to the spec's Definition of Done and Experiment 4.

- **Bead descriptions must include the `#![forbid(unsafe_code)]` constraint** — this prevents agents from reaching for `unsafe` in PTY/signal code.

### Step 3: Verify Output

After the skill completes, verify:

- [ ] Beads created via `br list --json`
- [ ] Total bead count between 15-25
- [ ] Every bead has a clear title, description with embedded spec content, and file listing
- [ ] Every bead has TDD test IDs listed as verification criteria
- [ ] Dependencies mirror the strategy phase ordering (no circular deps)
- [ ] Labels are applied (subsystem tags)
- [ ] No bead says "see spec for details" — content must be INLINE
- [ ] First bead creates the workspace foundation (compilable skeleton)
- [ ] Last bead proves the full integration loop

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete.

ULTRATHINK and verify:

1. Run `br list --json | jq '.[] | .title'` — are titles specific enough to be unambiguous? ("Implement PTY session" is vague; "Create pty/session.rs: openpty, setsid, TIOCSCTTY, CLOEXEC, pgid spawn, default 120x40 geometry" is specific)
2. Check each bead's file listing — does it include BOTH source files AND corresponding test files? (A bead that creates `pty/session.rs` should also create `tests/agent_pty_session.rs` or at least the relevant test cases)
3. Verify that the foundation bead includes the L0 schema as a dependency/prerequisite (L1 migration `0003_sqlite_runtime.sql` runs ON TOP of L0's `0001_sqlite_state_initial.sql`)
4. Check for "orphan files" — files listed in the manifest but not assigned to any bead
5. Run `bv --robot-triage` to verify the dependency graph is healthy and has clear execution tracks

---

## Key Reminders

- Beads embed spec + TDD content VERBATIM — strategy provides ordering only
- Each bead is self-contained: an agent should implement it without opening other documents
- Use `br` CLI for bead management (not `bd`)
- Bead prefix: `l1`
- The spec's `#![forbid(unsafe_code)]` constraint must be in every bead description
- Build gate for every bead: `cargo check --all-targets && cargo clippy --all-targets -- -D warnings`
- Test gate for every bead: `cargo test <specific_tests_for_this_bead>`
