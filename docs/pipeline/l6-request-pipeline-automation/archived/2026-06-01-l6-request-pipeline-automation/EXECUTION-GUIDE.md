# L6 Request Pipeline Automation — Pipeline Execution Guide

> Generated: 2026-05-21 by pipemaxxer · Pattern B (existing execution-grade layer doc)
> **Prerequisite:** L5 Expanded Commercial Intelligence must be COMPLETE
> (`docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-2026-05-20.md`
> verdict COMPLETE; 0 fix beads; `ready_for_l6: true`; `l5_to_l6_gate_producible: true`).
> L0-L4 must already be built (confirmed via L4/L5 completion audits).

---

## Pipeline Entry Point: Batch 1 (Validate Existing Spec)

**Source spec:** `docs/strategy/layers/layer-6-request-pipeline-automation.md` (383 lines, 11 DoD criteria, 8 invariants, **14 state-machine transitions** (§1.3 table has 14 From→To rows), 19 bid-workflow flow steps, 11-step skill playbook (with 10a/10b branching alternates of step 10), 4 risks, M6 capstone).

The L6 spec is execution-grade — well-structured but missing `L6-FR-X.Y.Z` IDs. Batch 1 adds those + transcribes 5 pre-resolved operator decisions (recorded 2026-05-21 via pipemaxxer AskUserQuestion in `manifest.yaml#recorded_decisions`).

**Total handoffs:** 15 (`01-validate-spec` → `15-archive`)

---

## Overview Table

| Batch | Step | Skill | Output | Sub-Opt |
|-------|------|-------|--------|---------|
| 1 SPEC | 01 validate-spec | (manual) | spec validated in-place (+ L6-FR IDs, + §1.0 decisions) | yes |
| 1 SPEC | 02 optimize-spec | (manual) | spec audited fresh-eyes | yes |
| 2 TDD | 03 create-tdd | `/spec-to-tdd` | `l6-*-tdd.md` + `l6-*-bootstrap.md` | yes |
| 2 TDD | 04 optimize-tdd | (manual) | TDD + bootstrap audited fresh-eyes | yes |
| 3 STRATEGY | 05 create-strategy | `/spec-to-strategy` | `l6-*-strategy.md` | yes |
| 3 STRATEGY | 06 optimize-strategy | (manual) | strategy audited fresh-eyes | yes |
| 4 BEADS | 07 create-beads | `/beads-from-sources` | beads + `l6-*-bulk-import.md` + `l6-bead-alias-map.json` | yes |
| 4 BEADS | 08 optimize-beads | (manual) | beads audited fresh-eyes | yes |
| 5 AUDIT | 09 audit-beads | `/bead-plan-audit` | beads audited zero-drift | yes |
| 6 KICKOFF | 10 generate-kickoff | `/beads-to-execution` | `l6-*-{kickoff,push,status}.{md,yaml}` | NO (deliverable) |
| 6 KICKOFF | 11 push (use as needed) | (template) | (used during execution) | — |
| POST | 12 completion-audit | `/bead-completion-audit` | `l6-completion-audit-YYYY-MM-DD.md` + `l6-audit-fix` beads | yes |
| POST | 13 fix-beads-kickoff | (template) | fix beads agent onboarding | NO (deliverable) |
| POST | 14 fix-beads-push | (template) | fix beads continuation nudge | NO (deliverable) |
| POST | 15 archive | (manual) | pipeline archived; status updated; MEMORY updated | — |

---

## Execution Order

### Batch 1 — SPEC (validate + optimize the existing layer doc)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 1 | `01-validate-spec.md` | manual | — |
| 2 | `02-optimize-spec.md` | manual | session 1 |

**What to do:** L6 doc IS already a structured spec. Session 1 verifies + adds requirement IDs (`L6-FR-X.Y.Z`), bullet-extracts ACs, transcribes the **5 pre-resolved decisions** from `manifest.yaml#recorded_decisions`, and scope-drift-scans:

1. **Decision 1** — Proactis === Procontract (same portal `procontract.due-north.com`). New files use `procontract` naming throughout. Capability unchanged — naming reconciliation only.
2. **Decision 2** — Reuse existing `skills/auth-signin/procontract/` (v1.0.0, Touch ID requirement removed). Bid-portal-procontract delegates auth to this skill when session expired.
3. **Decision 3** — Include `skills/validation/overlay-bid-portal-procontract/` as L6 deliverable (frontmatter follows overlay-supplier-enrichment shape). Gives `validation_policy: always` domain-specific assertions.
4. **Decision 4** — Sister implementation epic (close existing `testproject1-rc0` doc epic; create new `Epic: L6 Implementation — Request Pipeline Automation`). Matches L4/L5 pattern.
5. **Decision 5** — Procontract access already solved (existing CDP browser + saved Chrome credential for `sales@sorelon.com`). Live demo feasible from day one; no operator-built skill prerequisites needed.

Session 2 is a fresh-eyes audit by an independent agent. Both sessions run scope-drift greps.

**Gate:** Spec has unique `L6-FR-X.Y.Z` requirement IDs, every requirement has explicit ACs, no scope-drift language anywhere, all 5 decisions transcribed in §1.0, naming reconciliation banner present, L5→L6 readiness gate (5 blocker conditions + quote-role + freshness + company-profile + upstream-trust) preserved verbatim, Implementation Status callout bumped to "VALIDATED FOR IMPLEMENTATION PIPELINE (2026-05-21)".

### Enrichment after Batch 1

Read the validated spec. Extract:
- Requirement ID range (e.g., `L6-FR-1.1.1` … `L6-FR-2.8.N`) and total count (target 80-120)
- Section count (2 main sections: §1 Overview + Bid Workflow System, §2 Testing/DoD/Risks/Dependencies)
- DoD criterion count (11 in §2.1)
- Invariant count (8 in §1.10)
- State-machine transition count (11 in §1.3)
- Skill playbook step count (11 in §1.7)
- Audit-trail artifact type count (6 in §1.8)
- Confirmed file naming: `procontract` (not `proactis`)
- Confirmed validator-overlay inclusion + sister-epic structure decisions

Edit handoffs **03-15** — replace `{ENRICHMENT: spec metadata}` markers with actual numbers.

---

### Batch 2 — TDD (test specification + bootstrap)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 3 | `03-create-tdd.md` | `/spec-to-tdd` | Batch 1 |
| 4 | `04-optimize-tdd.md` | manual | session 3 |

**What to do:** Session 3 invokes `/spec-to-tdd` to produce both the TDD (test catalog with stable `L6-T-*` IDs, coverage matrix L6-FR → L6-T, static CI gates, test pyramid T1-T4) and the bootstrap doc (env prerequisites + validation script + local-only quality gate sequence). Session 4 audits both with fresh eyes.

**Gate:** Every L6-FR-X.Y.Z has ≥1 test ID, every DoD criterion (1-11) maps to verifying tests, the 3 spec-named test files (`bid-workflow-states.test.ts`, `portal-side-effects.test.ts`, `bid-readiness-gate.test.ts`) are concretely scoped with exact assertions, calibration corpus structure spec'd for `tests/fixtures/calibration/bid-portal-procontract/{simple,medium,complex}/`, PII redaction tests cover all 13+ sensitive `BidScopePayload`+`CompanyProfile` fields, all TDD-determined NEW ErrorCode additions (typically 3-6 after auditing existing codes for redundancy) have round-trip tests + propagation-site coverage, validator overlay tests cover fail-closed (rejection AND outage), live demo (DoD #8) documented as T4 manual smoke, bootstrap covers all 6 prerequisite categories + local quality gate sequence verbatim, ZERO new paid commercial accounts required (skill-first per decision_5).

### Enrichment after Batch 2

Read the TDD. Extract:
- Test ID range (e.g., `L6-T-bid_workflow_states-001` … `L6-T-live_demo-NN`) and total count (target 80-120, matching L5's 98 density)
- Test counts by file and by type (unit / integration / E2E / canary / calibration / manual smoke)
- CI tier distribution (T1/T2/T3/T4 budgets)
- Coverage matrix summary (DoD criterion → test IDs; invariant → enforcement test)
- Bootstrap prerequisite list (CDP profile + AgentMail + L5-status verification + opportunity-ref availability)

Edit handoffs **05-15** — replace `{ENRICHMENT: tdd metadata}` markers with actuals.

---

### Batch 3 — STRATEGY (phase ordering + dependency DAG)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 5 | `05-create-strategy.md` | `/spec-to-strategy` | Batch 2 |
| 6 | `06-optimize-strategy.md` | manual | session 5 |

**What to do:** Session 5 invokes `/spec-to-strategy` to produce phase ordering only (no spec/TDD content duplication). Session 6 audits.

**Gate:** Every L6-FR-X.Y.Z appears in some phase, phases are labeled execution-order (NOT importance tiers), later phases have SAME depth as early phases, the 7-phase shape (Foundation → Daemon Runtime → State Machine → Portal Skill ∥ Validator Overlay+packages/bidding → Desktop+Web UI → Live Demo) respects critical-path dependencies (ErrorCode propagation is root), Phase 7 has ≥4 beads (no thin M6 capstone), operator-action gates for Phase 7 explicit (CDP profile + opportunity ref + sideEffectCap=REMOTE_DRAFT for first run + separate FORMAL_SUBMISSION authorization), no scope-drift language.

### Suggested phase shape (heuristic — validate against critical-path analysis)

1. **Foundation** — ErrorCode +3-6 NEW variants (TDD-determined after redundancy audit) + propagation across daemon (5-15 sites); **`packages/bidding/bid-scope-types.ts`** (BidScopePayload + SideEffectClass + BidWorkflow + BidDocument + SubmissionResult + DraftSaveResult + PortalStateVerification — Phase 1 because Phase 4's portal skill `result.schema.json` references SubmissionResult/DraftSaveResult AND Phase 2's IPC handler accepts BidScopePayload at its boundary); contracts-schema re-exports if needed; PUSH_OWNED_RESEED_TABLES check (no-op confirmation). **Root of DAG.**
2. **Daemon Runtime** — `crates/daemon/src/main.rs` `"run_bid_workflow" => run_bid_workflow_job(...)` dispatch arm at line 1513+ (next to existing `extract_document_structure` / `extract_document_semantics` arms; handler function follows `run_extract_document_structure_job` at line 1529 as template); `crates/daemon/src/ipc/handlers/jobs.rs` `trigger_bid_workflow` handler (following `trigger_ingestion` at line 45 + `trigger_enrichment` at line 95) with L5 readiness pre-check; `crates/daemon/src/main.rs` `--run-bid-workflow` CLI subcommand; optional `crates/daemon/src/bid_workflow/` module scaffold (Phase 3 decides extraction). **CRITICAL: dispatch arm is in main.rs, NOT in jobs/runner.rs (which holds shared types only).** **Depends on 1.**
3. **State Machine + Readiness Gate** — Rust enforcement of 7 lifecycle values; **14 transitions** (spec §1.3 — not 11); decisionBasisHash computation hooks; readiness-gate wrapper of existing L5 checks; PII redaction extensions. **Depends on 2.**
4. **Portal Skill** — `skills/bid-portal-procontract/` (SKILL.md + schemas + canary inputs + golden assertions); auth-signin delegation per decision_2; CDP attach via `bin/ensure-cdp-9222`; canary harness extension (`e2e_can_010+`); golden journey extension (`l3_t_skill_golden_journeys_009+`); per-step screenshots. **Parallel to 5 after 1+3 close.**
5. **Validator Overlay + packages/bidding (workflow)** — `skills/validation/overlay-bid-portal-procontract/` (per decision_3); `packages/bidding/{workflow-registry,procontract-workflow,company-profile-schema}.ts` (bid-scope-types.ts is Phase 1) + 3 test files + package.json + tsconfig.json + vitest.config.ts; calibration corpus structure for bid-portal-procontract. **Parallel to 4 after 1+3.**
6. **Desktop + Web UI** — `packages/ui/src/{BidWorkflowStatusBadge,BidScopeSummary,DraftSavedReviewPanel,BidWorkflowStartButton}.tsx`; desktop BidWorkflow tab integration; web read-only bid_state badge + RLS-bounded view; desktop + web E2E tests. **HARD-DEPENDS on Phase 2** (Start Bid button calls the `trigger_bid_workflow` IPC handler — non-functional otherwise) + Phase 5 (UI displays validator verdicts).
7. **Live Demo + Integration + Calibration Validation** — first live demo (operator-driven, sideEffectCap=REMOTE_DRAFT → draft_saved); operator-authorized second live demo (FORMAL_SUBMISSION → bid_submitted); calibration corpus runs; final quality gate sweep. **Depends on 1-6.**

### Enrichment after Batch 3

Read the strategy. Extract: phase names (likely the 7 above, but strategy author may consolidate to 6 or split to 8), beads-per-phase estimate, gate criteria. Edit handoffs **07-15** with actuals.

**IMPORTANT — Phase-number reconciliation:** Handoffs 07-15 currently use "Phase 7" as the hardcoded label for "live demo + integration + calibration validation + final quality gate sweep" (the M6 capstone). If strategy step 05 picked a different phase shape (e.g., 6 phases where live demo is Phase 6, or 8 phases where it's Phase 8), do a find-and-replace across handoffs 07-15 to update the phase number consistently. Also update the "Phase X HARD-DEPENDS on Phase Y" cross-references. Use `grep -n "Phase [0-9]" docs/pipeline/l6-request-pipeline-automation/*.md` to find all references.

---

### Batch 4 — BEADS (executable units)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 7 | `07-create-beads.md` | `/beads-from-sources` | Batch 3 |
| 8 | `08-optimize-beads.md` | manual | session 7 |

**What to do:** Session 7's FIRST ACTION (before invoking `/beads-from-sources`):
1. Close `testproject1-rc0` (doc epic) per decision_4
2. Create sister epic: `br create --type=epic --priority=P0 --title="Epic: L6 Implementation — Request Pipeline Automation" --labels l6` (NOTE: `br create` uses `--labels` flag — singular `--label` is rejected by clap)
3. Capture new epic ID into manifest

Then invoke `/beads-from-sources` to materialize beads from the 3 SOTs. Session 8 audits coverage and uniqueness, runs `br lint`, verifies bead self-containment.

**Pre-conditions for step 07** (verify recorded in spec from step 01):
- All 5 decisions from `manifest.yaml#recorded_decisions` are transcribed in spec §1.0
- Naming reconciliation banner (Proactis=Procontract) present in §1.5/§1.6/§1.7
- No "Proactis" references in callouts about new files (only in narrative explaining alias)

If any pre-condition fails, step 07 STOPS — re-open step 01.

**Gate:** Bead prefix `l6`. Every L6-FR-X.Y.Z has a bead. Every L6-T-* test ID owned by exactly one bead. Every bead has exact `Files to Create/Modify` (parsed by `bin/br-claim`). Phase distribution balanced (Phase 1: 3-6, Phase 2: 4-7, Phase 3: 5-8, Phase 4: 6-10, Phase 5: 6-10, Phase 6: 6-10, Phase 7: 4-7). Total ~40-67 beads (incl. epic + review-gate beads). Sister epic created; rc0 closed. Run `bin/br-sync && git add .beads/ && git commit && git push`.

### Enrichment after Batch 4

Run `br list --label l6 --json | jq '.issues | length'`. Extract: total bead count, ready count, per-phase distribution. Edit handoffs **09-15** with actuals.

---

### Batch 5 — AUDIT (last gate before execution)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 9 | `09-audit-beads.md` | `/bead-plan-audit` | Batch 4 |

**What to do:** Systematic 5-pass spec-drift audit against beads + 8 L6-specific checks (ErrorCode propagation, naming reconciliation, auth delegation, validator overlay, live demo gates, sister epic, PII redaction, sync engine no-op). Read EVERY bead, walk EVERY L6-FR + EVERY L6-T, verify ZERO coverage gap, scan for ZERO scope-reduction language.

**Gate:** Coverage matrix shows 100% L6-FR → bead mapping. Test ID ownership 1:1. Zero "post-MVP" / "Phase 8+" / "deferred" / "stretch goal" / "nice to have" language. Phase coverage balanced (no anomalously thin late phase). Validator overlay bead present (decision_3). Live demo bead present with operator-action gates (decision_5). Verdict: PASS or FAIL.

If FAIL: fix beads, re-run audit. Do NOT proceed to step 10.

---

### Batch 6 — KICKOFF (generate execution prompts)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 10 | `10-generate-kickoff.md` | `/beads-to-execution` | Batch 5 |
| (11) | `11-push.md` | (template; used as needed) | session 10 |

**What to do:** Session 10 produces the kickoff (~400-700 lines, 13 sections matching L4/L5 quality bar including the CDP discipline + live demo constraints sections), push (~120-200 lines, 9 sections), and status YAML. The kickoff and push are the documents agents USE during implementation. Session 11 is a documentation note about how the push is used; no separate skill invocation.

⛔ **MANDATORY STOP — Human Authorizes Execution**

Before agents begin executing L6 implementation:
1. Read the kickoff/push (review against L4/L5 quality bar)
2. Verify AgentMail registration block uses `/Users/alinaaitova/Desktop/Testproject1`
3. Verify `br ready --label l6 --json` shows the L6 bead graph in expected order (Phase 1 roots ready)
4. Verify env vars + CDP profile path are correct (`/Users/alinaaitova/.chrome-automation`, port 9222)
5. Verify quality gates list correct Rust + TS + UBS commands
6. Authorize parallel agent execution

---

### Execution Phase

After authorization, agents claim L6 beads and execute. The push prompt (11) is a fresh-context restart for any continuation agent. The kickoff (10) is for agents who haven't seen this work yet.

⛔ **MANDATORY STOP — Human Triggers Completion Audit**

When `br list --status=open --label=l6` returns empty (or only `audit-fix` candidates), the human triggers Batch 12.

---

### POST — Completion Audit + Fix Loop + Archive

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 12 | `12-completion-audit.md` | `/bead-completion-audit` | execution complete |
| 13 | `13-fix-beads-kickoff.md` | (template) | session 12 (only if INCOMPLETE) |
| 14 | `14-fix-beads-push.md` | (template) | execution started (only if INCOMPLETE) |
| 15 | `15-archive.md` | manual | all complete |

**What to do:** Session 12 walks every L6-FR-X.Y.Z against the codebase + tests + executes the live demo; gaps become `l6-audit-fix` beads. Sessions 13-14 onboard agents to those fix beads (skipped if audit verdict = COMPLETE first time). Session 15 archives the pipeline.

**Gate:** Final DoD walkthrough confirms all 11 acceptance criteria met. Live demo first run (sideEffectCap=REMOTE_DRAFT) reached `draft_saved` on an operator-chosen Procontract opportunity. Operator-authorized second live demo (FORMAL_SUBMISSION) reached `bid_submitted` with portal confirmation reference captured. Quality gates green. Web read-only RLS verification passes. Validator overlay enforces fail-closed for portal submission.

---

## Quick Reference

| | |
|---|---|
| Spec | `docs/strategy/layers/layer-6-request-pipeline-automation.md` |
| TDD | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md` |
| Bootstrap | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md` |
| Strategy | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md` |
| Kickoff | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-kickoff.md` |
| Push | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-push.md` |
| Status | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-status.yaml` |
| Completion Audit | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-completion-audit-YYYY-MM-DD.md` |
| Handoff dir | `docs/pipeline/l6-request-pipeline-automation/` |
| Manifest | `docs/pipeline/l6-request-pipeline-automation/manifest.yaml` |
| Bead prefix | `l6` |
| Bead label | `layer-6` |
| Bead CLI | `br` (NEVER `bd`) |
| Bead sync | `bin/br-sync` (NEVER raw `br sync`) |
| Bead claim wrapper | `bin/br-claim <id>` (per AGENTS.md Rule 2 — constitutional) |
| Bead complete wrapper | `bin/br-complete <id>` (per AGENTS.md Rule 2 — constitutional) |
| Layer-doc epic (existing, will be closed in step 07) | `testproject1-rc0` |
| Implementation epic | created step 07, populated in manifest after creation |
| Build | `cargo check --all-targets && bun run build` |
| Test (Rust full) | `cargo test -p daemon --bin daemon && for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" \|\| exit 1; done` |
| Test (TS) | `bun run test` |
| Test (Desktop E2E) | `(cd apps/desktop && bun run test:e2e)` |
| Test (Web E2E) | `(cd apps/web && bun run test:e2e)` |
| Lint | `cargo clippy --all-targets -- -D warnings && cargo fmt --check && bun run lint` |
| UBS | `ubs $(git diff --name-only --cached)` |
| AgentMail key | `/Users/alinaaitova/Desktop/Testproject1` |
| AgentMail health | `curl -s http://127.0.0.1:8765/health/liveness` |
| CDP profile | `/Users/alinaaitova/.chrome-automation` |
| CDP port | 9222 |
| CDP launch | `bin/ensure-cdp-9222` |

---

## Mandatory Stops Recap

1. **All five operator decisions PRE-RESOLVED 2026-05-21** via pipemaxxer AskUserQuestion (recorded in `manifest.yaml#recorded_decisions`):
   - **Decision 1** — Proactis === Procontract (same portal `procontract.due-north.com`). New files use `procontract` naming.
   - **Decision 2** — REUSE existing `skills/auth-signin/procontract/` (Touch ID requirement removed). Bid-portal-procontract delegates portal login.
   - **Decision 3** — Include `skills/validation/overlay-bid-portal-procontract/` as L6 deliverable. Validator overlay provides domain-specific assertions for `validation_policy: always`.
   - **Decision 4** — Sister implementation epic. Close `testproject1-rc0` doc epic; create new `Epic: L6 Implementation — Request Pipeline Automation` in step 07.
   - **Decision 5** — Procontract access already solved (CDP browser + saved Chrome credential for `sales@sorelon.com` with Touch ID removed). Live demo feasible from day one; no operator-built skill prerequisites needed.

   Step 01 §6 transcribes these verbatim into the spec. Step 02 verifies the transcription. No re-deciding.

2. **Skill-first architecture (consistent with L5) — ZERO commercial API procurements needed.** L6 uses CDP browser automation against the operator's existing Procontract account. No new external accounts.

3. **Before Batch 6 step 10:** human reviews kickoff + push, authorizes execution.

4. **After parallel execution closes all task beads:** human triggers Batch 12 completion audit.

5. **After fix beads close (if any):** human verifies live demo + DoD #8 capstone completion, then authorizes archive (15).

---

## Timeline Estimate

| Batch | Approx | Notes |
|-------|--------|------|
| 1 SPEC | 1.5-3 hours | Validation + 5-decision transcription (all pre-resolved 2026-05-21) + naming reconciliation banner + fresh-eyes audit. Validator-agent transcribes operator decisions; does NOT re-decide. |
| 2 TDD | 4-8 hours | TDD generation (3 named TS test files + Rust integration extensions + validator overlay tests + calibration corpus + live demo manual smoke), bootstrap parallelizable |
| 3 STRATEGY | 1.5-3 hours | Phase ordering only — lightweight; 7-phase shape with explicit operator-action gates for Phase 7 |
| 4 BEADS | 3-7 hours | 40-67 beads expected (Phase 4 portal skill is the largest; Phase 7 live demo has ≥4 beads). Sister-epic creation upfront. |
| 5 AUDIT | 2-4 hours | Systematic 5-pass walk + 8 L6-specific checks |
| 6 KICKOFF | 1-2 hours | Single session output |
| Execution | days-weeks | Multi-agent parallel; depends on team size + Procontract live-demo iteration cycles |
| POST | 1-3 days | Audit → fix beads (if any) → re-verify loop → archive |

---

## Notes for Pipemaxxer Re-Runs

- This pipeline targets L6 ONLY. **Do not** mix L5 or any non-L6 work into these handoffs.
- If something in the L6 spec proves impossible during execution, prefer **fix beads + spec amendment** over silent scope reduction. Scope reduction = blocking failure.
- All 5 operator decisions PRE-RESOLVED 2026-05-21 — the validate-spec step TRANSCRIBES verbatim; does NOT re-decide.
- Naming reconciliation (Proactis=Procontract per decision_1) is the largest spec callout. New files use `procontract`. Spec text remains intelligible referring to either name.
- Procontract is the operator's real procurement account (`sales@sorelon.com` via Chrome saved credentials). Touch ID requirement removed 2026-05 (operator confirmed). Live demo runs against operator-chosen real opportunities — first run constrained to sideEffectCap=REMOTE_DRAFT (draft only); second run requires explicit operator authorization for FORMAL_SUBMISSION.
- ErrorCode enum extension (6 new variants) is a Rust compile-time change. The Phase 1 ErrorCode bead must be sized for clippy-surfaced exhaustive match propagation across ~5-15 sites in the daemon (per L5 precedent).
- L6 = final layer. No L7 readiness gate exists. Status YAML's `ready_for_next_layer` field is N/A.
- The M6 capstone is achieved when DoD #8 (live demo) passes — both first AND second runs.
- After archive, the procurement intelligence platform's layered architecture is COMPLETE.
- The historical `prompts/stage-15-bid-workflow-handoff-DEFERRED.md` is SUPERSEDED by L6 implementation. Step 15 archive flags it for cleanup or rewrite if any historical reference is needed.
