# L2 Desktop MVP — Pipeline Execution Guide

## Pipeline Entry Point: Batch 1* (Validate Spec)
## Total Handoffs: 14
## Pattern: B — Layer/Plan Implementation (Greenfield from Existing Spec)

---

## Quick Reference

| Item | Path |
|------|------|
| **Spec (SOT #1)** | `docs/strategy/layers/layer-2-single-operator-desktop.md` |
| **TDD (SOT #2)** | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` |
| **Strategy (SOT #3)** | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` |
| **Bootstrap** | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md` |
| **Kickoff** | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-kickoff.md` |
| **Push** | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-push.md` |
| **Status** | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-status.yaml` |
| **Manifest** | `docs/pipeline/l2-desktop-mvp/manifest.yaml` |
| **Bead prefix** | `l2` |
| **Bead CLI** | `br` |
| **Tech stack** | Mixed: Rust 2024 (Tauri backend + daemon) + TypeScript/React (frontend + packages, Bun) |
| **Quality bar** | L1 pipeline: `docs/pipeline/l1-daemon-core/` (30 beads, 138 requirements, 150 tests) |

---

## Execution Order

### Batch 1*: SPEC VALIDATION

| # | Session | File | Depends On | Output |
|---|---------|------|-----------|--------|
| 1 | Fresh | `01-validate-spec.md` | — | Validated spec with `L2-FR-*` requirement IDs |
| 2 | Fresh | `02-optimize-spec.md` | Session 1 | Audited and improved spec |

**Time estimate:** 1-2 sessions

**Output artifact:** Updated `docs/strategy/layers/layer-2-single-operator-desktop.md` with `L2-FR-*` requirement IDs and structured acceptance criteria.

### Enrichment: Update Subsequent Handoffs

After Batch 1* completes:
1. Read the updated spec
2. Extract: requirement ID range (`L2-FR-QS.1` through `L2-FR-A11Y.N`), total count, subsystem distribution
3. Edit handoffs 03-14 — replace `{ENRICHMENT: ...}` markers with specific data:
   - Add requirement ID count and range to context sections
   - Add subsystem breakdown to TDD guidance
   - Add specific requirement IDs to bead audit checks

---

### Batch 2: TDD

| # | Session | File | Depends On | Output |
|---|---------|------|-----------|--------|
| 3 | Fresh | `03-create-tdd.md` | Batch 1* complete | TDD + Bootstrap docs |
| 4 | Fresh | `04-optimize-tdd.md` | Session 3 | Audited TDD + Bootstrap |

**Time estimate:** 2-3 sessions (TDD is the most complex artifact — three test surfaces to cover)

**Output artifacts:**
- `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md`
- `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md`

### Enrichment: Update Subsequent Handoffs

After Batch 2 completes:
1. Read the TDD
2. Extract: test ID range, total count (by tier: Playwright/Tauri/Agent E2E), CI gate commands
3. Edit handoffs 05-14:
   - Add test count and tier distribution to strategy context
   - Add specific test IDs to bead gate criteria references
   - Add CI gate commands to audit checks

---

### Batch 3: STRATEGY

| # | Session | File | Depends On | Output |
|---|---------|------|-----------|--------|
| 5 | Fresh | `05-create-strategy.md` | Batch 2 complete | Implementation strategy (phase DAG) |
| 6 | Fresh | `06-optimize-strategy.md` | Session 5 | Audited strategy |

**Time estimate:** 1-2 sessions

**Output artifact:** `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md`

### Enrichment: Update Subsequent Handoffs

After Batch 3 completes:
1. Read the strategy
2. Extract: phase count, phase names, tasks per phase, gate criteria, expected bead count
3. Edit handoffs 07-14:
   - Add phase names and ordering to beads context
   - Add expected bead count to audit expectations
   - Add phase-to-subsystem mapping to kickoff content

---

### Batch 4: BEADS

| # | Session | File | Depends On | Output |
|---|---------|------|-----------|--------|
| 7 | Fresh | `07-create-beads.md` | Batch 3 complete | Beads created via `br` |
| 8 | Fresh | `08-optimize-beads.md` | Session 7 | Optimized beads |

**Time estimate:** 1-2 sessions

**Output:** Beads in `.beads/` (managed via `br` CLI)

### Enrichment: Update Subsequent Handoffs

After Batch 4 completes:
1. Run `br list --json | jq 'length'`
2. Extract: total count, label distribution, dependency depth
3. Edit handoffs 09-14:
   - Add actual bead count to audit expectations
   - Add bead ID ranges to kickoff references

---

### Batch 5: AUDIT

| # | Session | File | Depends On | Output |
|---|---------|------|-----------|--------|
| 9 | Fresh | `09-audit-beads.md` | Batch 4 complete | Zero-drift verified beads |

**Time estimate:** 1 session

**Output:** Audited beads — any issues fixed in-place or via new beads.

---

### Batch 6: KICKOFF + PUSH

| # | Session | File | Depends On | Output |
|---|---------|------|-----------|--------|
| 10 | Fresh | `10-generate-kickoff.md` | Batch 5 complete | Kickoff + Push + Status |

**Time estimate:** 1 session

**Output artifacts:**
- `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-kickoff.md`
- `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-push.md`
- `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-status.yaml`

## MANDATORY STOP — Human Authorizes Execution

> **Before proceeding:** Review the kickoff prompt. Verify it accurately represents the implementation plan. Check that the AgentMail coordination workflow is complete. Verify quality gate commands cover both Rust and TypeScript. Authorize execution by proceeding to the next step.

---

### EXECUTION PHASE

Use the kickoff prompt (`l2-desktop-mvp-kickoff.md`) to onboard implementation agents. Use the push prompt (`l2-desktop-mvp-push.md` or `11-push.md`) as a continuation nudge when agents pause or complete beads.

**Expected execution time:** Multiple sessions across multiple agents, depending on parallelization. L2 has more files than L1 but many are simpler (React components vs Rust modules).

**Monitoring:** Check progress via:
```bash
br list --status=open --label l2    # Remaining work
br list --status=closed --label l2  # Completed work
bv --robot-triage                   # Recommended next action
```

**Quality monitoring (run periodically):**
```bash
# Rust
cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test

# TypeScript
bun run build && bun run lint && bun run test

# Playwright (once UI pages exist)
bunx playwright test
```

## MANDATORY STOP — Human Triggers Completion Audit

> **Before proceeding:** All beads should be closed. Both Rust and TypeScript builds should pass. All Playwright tests should pass. Trigger the completion audit.

---

### Post-Implementation: COMPLETION AUDIT

| # | Session | File | Depends On | Output |
|---|---------|------|-----------|--------|
| 11 | Fresh | `12-completion-audit.md` | All beads closed | Gap analysis + fix beads |
| 12 | Fresh | `13-execute-fix-beads.md` | Session 11 | Fixes implemented |

**Time estimate:** 1-3 sessions (depends on gap count)

**Loop:** If completion audit finds gaps → create fix beads → execute → re-audit. Repeat until 100% or BLOCKED on human input.

---

### Post-Implementation: ARCHIVE

| # | Session | File | Depends On | Output |
|---|---------|------|-----------|--------|
| 13 | Fresh | `14-archive.md` | All fix beads closed | Archived pipeline |

**Time estimate:** 1 session

---

## Parallelization Notes

- **Batches 1-5 are sequential** — each depends on the previous batch's output
- **Within Batch 6**, kickoff, push, and status are generated together
- **During execution**, multiple agents can work on independent beads in parallel:
  - Rust-only beads (IPC extension, CF skill) are parallel with TypeScript beads
  - Different React pages can be built in parallel
  - Shared packages can be built in parallel with IPC extension
- **Post-implementation** sessions are sequential (audit → fix → re-audit → archive)

---

## L2-Specific Execution Notes

### Mixed-Stack Coordination
- **Agents working on Rust beads** need: `cargo check`, `cargo clippy`, `cargo fmt`, `cargo test`
- **Agents working on TypeScript beads** need: `bun install`, `bun run build`, `bun run lint`, `bun run test`
- **Agents working on Playwright beads** need: running daemon + `bun run dev` + `bunx playwright test`
- An agent should NOT attempt to run Playwright tests unless the dev bridge and at least one page exist

### Build Order Dependencies
```
Phase 1: Workspace scaffold (package.json, Tauri project)
  ↓
Phase 2: packages/* (ui, manifests, intelligence)     Phase 4: IPC extension (Rust)     Phase 9: CF skill
  ↓                                                      ↓
Phase 3: Transport shim + dev bridge                   Phase 5: QueryService + DaemonBridge (Tauri Rust)
  ↓                                                      ↓
Phase 6: React pages ← depends on Phase 2, 3, 5
  ↓
Phase 7: Live streaming (Channel<T>, session stores) ← depends on Phase 5
  ↓
Phase 8: Decision capture + coverage metrics ← depends on Phase 5, 6
  ↓
Phase 10: Playwright E2E + security tests ← depends on Phase 6, 7, 8
  ↓
Phase 11: Tauri integration + full assembly ← depends on Phase 10
```

### Definition of Done (from spec §4.2)
All 16 criteria must pass:
1. FTS ingestion skill scrapes live notices
2. Contracts Finder ingestion skill scrapes live notices
3. Notice normalization produces canonical records
4. Goods/services classifier correctly categorizes
5. Desktop control panel launches (QueryService + DaemonBridge + bootstrap)
6. Notice list view with keyset pagination
7. Opportunity detail view with margins, candidates, quotes
8. Decision capture with reviewed-basis metadata
9. Agent session monitoring with live terminal output
10. Coverage metrics displayed
11. Decision-basis staleness indicators
12. Single operator runs daily for 1 week
13. Desktop auto-launches daemon
14. Stop/input/retry controls
15. Commit fence consistency verification
16. Daemon restart detection via instanceId mismatch

---

## Total Pipeline

| Phase | Sessions | Produces |
|-------|----------|---------|
| Batch 1* (Spec) | 2 | Validated spec with `L2-FR-*` requirement IDs |
| Batch 2 (TDD) | 2-3 | TDD + Bootstrap |
| Batch 3 (Strategy) | 1-2 | Implementation strategy (phase DAG) |
| Batch 4 (Beads) | 1-2 | 20-30 executable beads |
| Batch 5 (Audit) | 1 | Zero-drift verified beads |
| Batch 6 (Kickoff) | 1 | Agent onboarding docs |
| **STOP** | — | Human authorizes execution |
| Execution | 5-20 | Layer 2 implemented (mixed Rust + TypeScript) |
| **STOP** | — | Human triggers audit |
| Completion Audit | 1-3 | Gaps found and fixed |
| Archive | 1 | Pipeline cleaned up |
| **Total** | **15-35 sessions** | **Layer 2 complete** |

---

## Next Pipeline Run

After Layer 2 is complete and archived, start Layer 3:

```
/pipemaxxer docs/strategy/layers/layer-3-operational-hardening.md
```
