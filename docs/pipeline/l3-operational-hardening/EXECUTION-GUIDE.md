# L3 Operational Hardening — Pipeline Execution Guide

## Pipeline Entry Point: Batch 1* (Validate Spec)
## Total Handoffs: 14
## Feature: Layer 3 — Operational Hardening (Milestone M3)

---

## Overview

This pipeline transforms the L3 layer specification into production code through 7 batches:

| Batch | Steps | What It Produces | Sessions |
|-------|-------|-----------------|----------|
| **1: SPEC** | 01-validate, 02-optimize | Validated spec (SOT #1) | 2 sequential |
| **2: TDD** | 03-create, 04-optimize | TDD + Bootstrap (SOT #2) | 2 sequential |
| **3: STRATEGY** | 05-create, 06-optimize | Strategy (SOT #3) | 2 sequential |
| **4: BEADS** | 07-create, 08-optimize | Executable beads | 2 sequential |
| **5: AUDIT** | 09-audit | Audited beads (zero drift) | 1 |
| **6: KICKOFF** | 10-generate, 11-push | Kickoff + Push prompts | 1 |
| **POST** | 12-audit, 13-fix, 14-archive | Verified implementation | Loop until 100% |

---

## Execution Order

### Batch 1: SPEC (Validate + Optimize)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 1 | `01-validate-spec.md` | No skill — manual validation | — |
| 2 | `02-optimize-spec.md` | No skill — fresh-eyes audit | Session 1 |

**What to do:** Open a fresh Claude Code session. Paste the contents of `01-validate-spec.md`. After it completes (including the sub-optimization follow-up), start a new session and paste `02-optimize-spec.md`.

**Gate:** Spec has systematic requirement IDs (L3-FR-x.y.z), testable ACs, complete structure. Requirement count reported.

### Enrichment: Update Subsequent Handoffs

After Batch 1 completes, read the validated spec and update handoffs 03-14:
1. Read `docs/strategy/layers/layer-3-operational-hardening.md`
2. Extract: requirement ID range (L3-FR-1.1 to L3-FR-x.y), total requirement count, subsystem count, key section names
3. Edit handoffs 03-14 — replace `{ENRICHMENT: ...}` markers with actual data:
   - "The spec has {N} requirements across {M} subsystems. Key areas: {list}. IDs range from L3-FR-1.1 to L3-FR-{last}."

---

### Batch 2: TDD (Create + Optimize)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 3 | `03-create-tdd.md` | `/spec-to-tdd` | Batch 1 complete |
| 4 | `04-optimize-tdd.md` | No skill — fresh-eyes audit | Session 3 |

**What to do:** Open a fresh session. Paste `03-create-tdd.md`. The agent invokes `/spec-to-tdd` and generates the TDD + Bootstrap. Then start a new session for `04-optimize-tdd.md`.

**Gate:** Every requirement has a test ID. Coverage matrix is 100%. CI tiering defined. Test count is 150-250.

### Enrichment: Update Subsequent Handoffs

After Batch 2 completes:
1. Read `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md`
2. Extract: test ID range, total test count (by type: UT/IT/E2E), CI tier distribution, coverage matrix summary
3. Edit handoffs 05-14 — replace enrichment markers:
   - "The TDD has {N} tests ({UT}/{IT}/{E2E}). Coverage matrix maps {N} spec requirements to test IDs."
   - Update strategy handoff gate criteria with actual TDD test IDs

---

### Batch 3: STRATEGY (Create + Optimize)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 5 | `05-create-strategy.md` | `/spec-to-strategy` | Batch 2 complete |
| 6 | `06-optimize-strategy.md` | No skill — fresh-eyes audit | Session 5 |

**What to do:** Open a fresh session. Paste `05-create-strategy.md`. The agent invokes `/spec-to-strategy` and generates the strategy. Then `06-optimize-strategy.md` in a new session.

**Gate:** Phases ordered correctly, no dependency cycles, gates reference TDD test IDs, estimated bead count 25-45.

### Enrichment: Update Subsequent Handoffs

After Batch 3 completes:
1. Read `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md`
2. Extract: phase names, phase count, tasks per phase, gate criteria, expected bead count
3. Edit handoffs 07-14 — replace enrichment markers:
   - "The strategy has {N} phases. Phase ordering: {list}. Expected bead count: ~{N}."
   - Update beads handoff with phase-to-epic mapping

---

### Batch 4: BEADS (Create + Optimize)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 7 | `07-create-beads.md` | `/beads-from-sources` | Batch 3 complete |
| 8 | `08-optimize-beads.md` | No skill — fresh-eyes audit | Session 7 |

**What to do:** Open a fresh session. Paste `07-create-beads.md`. The agent invokes `/beads-from-sources` and creates all beads. Then `08-optimize-beads.md` in a new session.

**Gate:** Bead count 25-45, DAG with no cycles, 100% spec coverage, 100% TDD coverage, all file paths valid.

### Enrichment: Update Subsequent Handoffs

After Batch 4 completes:
1. Run `br list --json | grep l3`
2. Extract: bead prefix, total count, ready count, phase distribution
3. Edit handoffs 09-14 — replace enrichment markers:
   - "Bead prefix: l3. Total: {N}. Per phase: {breakdown}."
   - Update audit handoff with expected coverage numbers

---

### Batch 5: AUDIT (Bead Plan Audit)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 9 | `09-audit-beads.md` | `/bead-plan-audit` | Batch 4 complete |

**What to do:** Open a fresh session. Paste `09-audit-beads.md`. The agent performs zero-drift spec coverage verification.

**Gate:** 100% requirement coverage, 100% test coverage, no dependency issues, all file paths valid.

---

### Batch 6: KICKOFF + PUSH

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 10 | `10-generate-kickoff.md` | `/beads-to-execution` | Batch 5 complete |

**What to do:** Open a fresh session. Paste `10-generate-kickoff.md`. The agent generates the kickoff prompt, push prompt, and status file.

**Output:**
- `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-kickoff.md`
- `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-push.md`
- `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-status.yaml`

---

## MANDATORY STOP — Human Authorizes Execution

**Before proceeding to execution, YOU (the human) must:**

1. Review the kickoff prompt — does it accurately describe the L3 implementation scope?
2. Review the push prompt — is it sufficient for continuation?
3. Verify all 3 SOT documents exist and look correct:
   - Spec: `docs/strategy/layers/layer-3-operational-hardening.md`
   - TDD: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md`
   - Strategy: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md`
4. Verify beads are ready: `br ready --json`
5. **Authorize execution by pasting the kickoff prompt into a fresh session**

---

## Execution Phase

Use the kickoff prompt to start implementation. Use the push prompt for continuation sessions.

Multiple agents can work in parallel — the kickoff and push prompts include full AgentMail coordination.

When all beads are complete, proceed to post-implementation.

---

## MANDATORY STOP — Human Triggers Completion Audit

**After all beads are executed, YOU (the human) must trigger the completion audit.**

---

### Post-Implementation: Completion Audit + Fix Loop

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| N | `12-completion-audit.md` | `/bead-completion-audit` | All beads executed |
| N+1 | `13-execute-fix-beads.md` | No skill — execute fix beads | Session N |
| — | Repeat 12-13 until 100% | — | — |
| Final | `14-archive.md` | No skill — archive pipeline | 100% audit pass |

**Loop:** If the completion audit finds gaps, fix beads are created (step 12). Execute them (step 13). Re-audit (step 12 again). Repeat until the audit returns COMPLETE with 100% coverage.

---

## Quick Reference

| Item | Path |
|------|------|
| Spec (SOT #1) | `docs/strategy/layers/layer-3-operational-hardening.md` |
| TDD (SOT #2) | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` |
| Bootstrap | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md` |
| Strategy (SOT #3) | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` |
| Kickoff | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-kickoff.md` |
| Push | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-push.md` |
| Status | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-status.yaml` |
| Manifest | `docs/pipeline/l3-operational-hardening/manifest.yaml` |
| Bead prefix | `l3` |
| Bead CLI | `br` |
| Quality gates (Rust) | `cargo check --all-targets && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` |
| Quality gates (TS) | `bun run build && bun run test && bun run lint` |
| Quality gates (pre-commit) | `ubs $(git diff --name-only --cached)` |

---

## Timeline Estimate

| Batch | Sessions | Typical Duration |
|-------|----------|-----------------|
| 1: SPEC | 2 | Validation + audit |
| 2: TDD | 2 | Generation + audit |
| 3: STRATEGY | 2 | Generation + audit |
| 4: BEADS | 2 | Creation + audit |
| 5: AUDIT | 1 | Zero-drift verification |
| 6: KICKOFF | 1 | Prompt generation |
| EXECUTION | N | Multiple agents, parallel |
| POST | 1-3 loops | Audit → fix → re-audit |
| **Total pre-execution** | **10 sessions** | — |
