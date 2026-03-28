# L1 Daemon Core — Pipeline Execution Guide

## Pipeline Entry Point: Batch 1* (Validate Spec)
## Total Handoffs: 14
## Pattern: B — Layer/Plan Implementation (Greenfield from Existing Spec)

---

## Quick Reference

| Item | Path |
|------|------|
| **Spec (SOT #1)** | `docs/strategy/layers/layer-1-thin-local-runner.md` |
| **TDD (SOT #2)** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md` |
| **Strategy (SOT #3)** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md` |
| **Bootstrap** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md` |
| **Kickoff** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-kickoff.md` |
| **Push** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-push.md` |
| **Status** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-status.yaml` |
| **Manifest** | `docs/pipeline/l1-daemon-core/manifest.yaml` |
| **Bead prefix** | `l1` |
| **Bead CLI** | `br` |
| **Tech stack** | Rust 2024 (nightly), Cargo, Tokio, rusqlite, nix, portable-pty |

---

## Execution Order

### Batch 1*: SPEC VALIDATION

| # | Session | File | Depends On | Output |
|---|---------|------|-----------|--------|
| 1 | Fresh | `01-validate-spec.md` | — | Validated spec with `L1-FR-*` requirement IDs |
| 2 | Fresh | `02-optimize-spec.md` | Session 1 | Audited and improved spec |

**Time estimate:** 1-2 sessions

**Output artifact:** Updated `docs/strategy/layers/layer-1-thin-local-runner.md` with requirement IDs and structured ACs.

### Enrichment: Update Subsequent Handoffs

After Batch 1* completes:
1. Read the updated spec
2. Extract: requirement ID range (L1-FR-PTY.1 through L1-FR-PIPELINE.N), total count, subsystem distribution
3. Edit handoffs 03-14 — replace generic references with specific data:
   - Add requirement ID count and range to context sections
   - Add subsystem breakdown to TDD guidance
   - Add specific requirement IDs to bead audit checks

---

### Batch 2: TDD

| # | Session | File | Depends On | Output |
|---|---------|------|-----------|--------|
| 3 | Fresh | `03-create-tdd.md` | Batch 1* complete | TDD + Bootstrap docs |
| 4 | Fresh | `04-optimize-tdd.md` | Session 3 | Audited TDD + Bootstrap |

**Time estimate:** 2-3 sessions (TDD is the most complex artifact)

**Output artifacts:**
- `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md`
- `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md`

### Enrichment: Update Subsequent Handoffs

After Batch 2 completes:
1. Read the TDD
2. Extract: test ID range, total count (by tier: unit/integration/E2E), CI tier distribution
3. Edit handoffs 05-14:
   - Add test count and tier distribution to strategy context
   - Add specific test IDs to bead gate criteria references
   - Add CI tier constraints to audit checks

---

### Batch 3: STRATEGY

| # | Session | File | Depends On | Output |
|---|---------|------|-----------|--------|
| 5 | Fresh | `05-create-strategy.md` | Batch 2 complete | Implementation strategy |
| 6 | Fresh | `06-optimize-strategy.md` | Session 5 | Audited strategy |

**Time estimate:** 1-2 sessions

**Output artifact:** `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md`

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
- `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-kickoff.md`
- `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-push.md`
- `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-status.yaml`

## MANDATORY STOP — Human Authorizes Execution

> **Before proceeding:** Review the kickoff prompt. Verify it accurately represents the implementation plan. Authorize execution by proceeding to the next step.

---

### EXECUTION PHASE

Use the kickoff prompt (`l1-daemon-core-kickoff.md`) to onboard implementation agents. Use the push prompt (`l1-daemon-core-push.md` or `11-push.md`) as a continuation nudge when agents pause or complete beads.

**Expected execution time:** Multiple sessions across multiple agents, depending on parallelization.

**Monitoring:** Check progress via:
```bash
br list --status=open --label l1    # Remaining work
br list --status=closed --label l1  # Completed work
bv --robot-triage                   # Recommended next action
```

## MANDATORY STOP — Human Triggers Completion Audit

> **Before proceeding:** All beads should be closed. Build and tests should pass. Trigger the completion audit.

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
- **Within Batch 6**, kickoff and push are generated together
- **During execution**, multiple agents can work on independent beads in parallel
- **Post-implementation** sessions are sequential (audit → fix → re-audit → archive)

---

## Total Pipeline

| Phase | Sessions | Produces |
|-------|----------|---------|
| Batch 1* (Spec) | 2 | Validated spec with requirement IDs |
| Batch 2 (TDD) | 2-3 | TDD + Bootstrap |
| Batch 3 (Strategy) | 1-2 | Implementation strategy |
| Batch 4 (Beads) | 1-2 | 15-25 executable beads |
| Batch 5 (Audit) | 1 | Zero-drift verified beads |
| Batch 6 (Kickoff) | 1 | Agent onboarding docs |
| **STOP** | — | Human authorizes execution |
| Execution | 5-15 | Layer 1 implemented |
| **STOP** | — | Human triggers audit |
| Completion Audit | 1-3 | Gaps found and fixed |
| Archive | 1 | Pipeline cleaned up |
| **Total** | **15-30 sessions** | **Layer 1 complete** |

---

## Next Pipeline Run

After Layer 1 is complete and archived, start Layer 2:

```
/pipemaxxer docs/strategy/layers/layer-2-single-operator-desktop.md
```
