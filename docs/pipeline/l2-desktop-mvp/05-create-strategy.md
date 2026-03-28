# L2 Desktop MVP — Create Strategy

## First Action

Invoke the `/spec-to-strategy` skill.

---

## Your Mission

Create the **Implementation Strategy** (SOT #3) for Layer 2 (Single-Operator Desktop MVP). The strategy defines the WHAT ORDER — a phase dependency DAG with gate criteria, bead grouping guidance, and expected bead count. It does NOT duplicate spec content — it routes to the spec and TDD for substance.

This is a **mixed-stack layer** with a specific build order constraint: TypeScript packages must be buildable before the desktop app, and the Tauri Rust backend must connect to the L1 daemon (which already exists). The strategy must account for this dependency chain.

---

## Source Documents (Read In This Order)

### Primary Input (CREATE FROM THESE TWO)
```
docs/strategy/layers/layer-2-single-operator-desktop.md                — Spec (SOT #1) — READ FULLY
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md          — TDD (SOT #2) — READ FULLY
```

### Source of Truth Context
```
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md     — Bootstrap — SKIM (workspace structure)
docs/strategy/layer-strategy.md                                         — Cross-layer contracts — SKIM §10-§11
```

### Codebase Context (STUDY FOR DEPENDENCIES)
```
Cargo.toml                                 — Workspace root (L2 adds apps/desktop/src-tauri to workspace)
crates/daemon/src/main.rs                  — Daemon entry point — L2 must not break this
crates/daemon/src/ipc/handshake.rs         — IPC protocol — must be extended before DaemonBridge can work
crates/daemon/src/ipc/descriptor.rs        — daemon.info.json — bootstrap reads this
crates/daemon/src/jobs/runner.rs           — Job Runner — CF skill needs this infrastructure
skills/procurement-ingestion/fts/          — FTS skill directory — CF skill follows same structure
migrations/0001_sqlite_state_initial.sql   — Schema that QueryService reads
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md      — L1 strategy (10 phases, dependency DAG)
```

---

## Key Context From Codebase Research

- **L1 strategy had 10 phases with a dependency DAG** and ~22 task beads — L2 should have a similar structure
- **Build order constraint:** `packages/*` → `apps/desktop/src/*` → `apps/desktop/src-tauri/*` → integration testing
- **The daemon already exists** — L2 extends it (adds IPC commands) but doesn't rebuild it
- **The schema already exists** — L0 created all 47 tables with all columns; L2 activates tables, doesn't add schema
- **No `packages/` or `apps/` directories exist yet** — Phase 1 must create the workspace scaffold
- **The Contracts Finder skill is independent of the desktop** — it can be developed in parallel

{ENRICHMENT: This section will be updated after Batch 2 with actual test ID ranges, test counts by tier, and coverage matrix summary from the TDD.}

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY — before doing anything)
Read these IN THIS ORDER:
1. `docs/strategy/layers/layer-2-single-operator-desktop.md` — Spec — **READ FULLY**
2. `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — TDD — **READ FULLY**
3. `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md` — L1 strategy — **STUDY structure, phase model, gate criteria**
4. `crates/daemon/src/ipc/handshake.rs` — **STUDY for IPC extension phasing**
5. `Cargo.toml` — **CHECK workspace structure**

### Step 1: Invoke the Skill
Run `/spec-to-strategy`.

**Inputs:**
- Spec: `docs/strategy/layers/layer-2-single-operator-desktop.md`
- TDD: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md`
- Output: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md`

**Key arguments/context to provide the skill:**
- Mixed stack (Rust + TypeScript) — the strategy must define build order across languages
- Bead prefix: `l2`
- Expected bead count: 20-30 (L2 has more files but many are simpler than L1's Rust modules)
- Gate criteria must include BOTH `cargo test` AND `bun run test` / `bunx playwright test`
- The strategy is a ROUTING TABLE — phase names, ordering, dependencies, and bead grouping. It does NOT contain spec content.

### Step 2: Feature-Specific Guidance
While the skill runs, pay attention to:

- **Phase 1 must be workspace scaffold:** Bun workspace root, TypeScript config, Tauri project setup, Playwright config. Nothing compiles or runs before this exists.
- **Shared packages before desktop app:** `packages/ui`, `packages/manifests`, `packages/intelligence` must be buildable before `apps/desktop` imports them.
- **IPC extension before DaemonBridge:** The daemon needs new IPC commands (10 from §1.5) before the Tauri DaemonBridge can call them. This is a Rust-side change in `crates/daemon/`.
- **Transport shim before pages:** `src/lib/transport.ts` and `vite-plugin-tauri-dev-bridge.ts` must exist before any page component can invoke the daemon.
- **Contracts Finder skill is independent:** It can be developed in parallel with the desktop, since it runs in the existing daemon infrastructure.
- **Security tests should be late:** `security.spec.ts` requires all rendering components to exist first.
- **Tauri integration tests are gated on full assembly:** `tauri-integration.spec.ts` requires a working Tauri build — this is the final tier.

**Suggested phase structure (adjust based on analysis):**

| Phase | Focus | Dependencies |
|-------|-------|-------------|
| 1 | Workspace scaffold + TypeScript config + Tauri project | None |
| 2 | Shared packages: ui, manifests, intelligence | Phase 1 |
| 3 | Transport shim + dev bridge | Phase 1 |
| 4 | IPC extension (daemon-side commands for L2) | Phase 1 (Rust only) |
| 5 | QueryService + DaemonBridge (Tauri Rust) | Phase 3, 4 |
| 6 | React pages: list, detail, sessions, errors | Phase 2, 3, 5 |
| 7 | Live streaming: Channel<T>, session registry/buffer | Phase 5 |
| 8 | Decision capture + coverage metrics + staleness | Phase 5, 6 |
| 9 | Contracts Finder skill | Phase 4 (parallel with 5-8) |
| 10 | Playwright E2E tests + security tests | Phase 6, 7, 8 |
| 11 | Tauri integration + full assembly + Tier 2 tests | Phase 10 |

### Step 3: Verify Output
After the skill completes, verify:
- [ ] Strategy file exists at `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md`
- [ ] Every phase has: name, description, dependencies, files created/modified, gate criteria, expected bead count
- [ ] Dependency DAG is acyclic (no circular dependencies)
- [ ] Build order respects: `packages/*` → `apps/desktop/src/*` → `apps/desktop/src-tauri/*`
- [ ] IPC extension (daemon-side) happens before DaemonBridge (desktop-side)
- [ ] Contracts Finder skill is independent of desktop phases
- [ ] Gate criteria include BOTH Rust and TypeScript quality commands
- [ ] Expected total bead count is 20-30
- [ ] Strategy does NOT duplicate spec content (uses references like "see L2-FR-QS.1 through L2-FR-QS.N")

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete. Use ULTRATHINK.

1. **Verify the dependency DAG is minimal:** For each dependency edge, ask "would removing this edge create a compilation error or a test failure?" If neither, the edge is unnecessary and adds sequencing overhead.

2. **Check parallelization opportunities:** Can Phase 2 (shared packages) and Phase 4 (IPC extension) run in parallel? They're in different languages with no file overlap. Can Phase 9 (CF skill) run in parallel with Phases 5-8? It should be fully independent.

3. **Verify gate criteria specificity:** Each phase's gate criteria should name specific tests or test files, not just "all tests pass." For example, Phase 5 gate: "`cargo test --test ipc_handshake` passes AND `bun run test -- query_service` passes." Cross-check against the TDD's test ID assignments.

4. **Check the bead granularity:** Each bead should represent 1-4 hours of work for a single agent. A bead like "implement all 6 React pages" is too coarse. A bead like "add `key` prop to ContractCardFrame" is too fine. Check that file groupings in the strategy produce right-sized beads.

5. **Verify the critical path:** What's the longest chain of sequential phases? This determines minimum pipeline time. If it's > 8 phases, look for ways to increase parallelism.

---

## Key Reminders

- **Strategy is ORDERING ONLY** — no spec content, no test definitions, no implementation details
- **Bead prefix is `l2`** — all beads created from this strategy use this prefix
- **Mixed-stack gates:** Every phase gate must specify WHICH quality commands are relevant (Rust-only phases don't need `bun run test`)
- **The CF skill is a parallelism opportunity** — don't gate it on desktop work
- **L1's 10-phase, 22-bead structure is the quality bar** — match its granularity and rigor
- **No phase should create more than 5 files** — if it does, split it
