# L5 Expanded Commercial Intelligence — Pipeline Execution Guide

> Generated: 2026-05-08 by pipemaxxer · Pattern B (layer doc → spec validation)
> **Prerequisite:** L4 Shared Visibility must be COMPLETE (audit verdict 2026-05-08 = COMPLETE; zero gaps). L4 sync engine, web portal, RLS, validation enforcement, and Supabase projections must all be operational before starting L5.

---

## Pipeline Entry Point: Batch 1 (Validate Existing Spec)

**Source spec:** `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` (520 lines, 12 DoD criteria, 10 new customs error types, 5 §3.6 Open Questions of which Q3 PCS Scotland is RESOLVED 2026-05-08 IN-SCOPE, 19 new TS files in `packages/intelligence/`, 1 new SQLite migration, 4 cron entries, 3 marketplace skill maturations + 1 truly-new PCS skill, 3 gold-set categories: fasteners + packaging-cartons + examination-gloves, CBAM forward placeholder).

**Total handoffs:** 15 (01-validate-spec → 15-archive)

---

## Overview Table

| Batch | Step | Skill | Output | Sub-Opt |
|-------|------|-------|--------|---------|
| 1 SPEC | 01 validate-spec | (manual) | spec validated in-place | yes |
| 1 SPEC | 02 optimize-spec | (manual) | spec audited fresh-eyes | yes |
| 2 TDD | 03 create-tdd | `/spec-to-tdd` | `l5-*-tdd.md` + `l5-*-bootstrap.md` | yes |
| 2 TDD | 04 optimize-tdd | (manual) | TDD audited fresh-eyes | yes |
| 3 STRATEGY | 05 create-strategy | `/spec-to-strategy` | `l5-*-strategy.md` | yes |
| 3 STRATEGY | 06 optimize-strategy | (manual) | strategy audited fresh-eyes | yes |
| 4 BEADS | 07 create-beads | `/beads-from-sources` | `l5-*` beads + `l5-bulk-import.md` + `l5-bead-alias-map.json` | yes |
| 4 BEADS | 08 optimize-beads | (manual) | beads audited fresh-eyes | yes |
| 5 AUDIT | 09 audit-beads | `/bead-plan-audit` | beads audited zero-drift | yes |
| 6 KICKOFF | 10 generate-kickoff | `/beads-to-execution` | `l5-*-{kickoff,push,status}.{md,yaml}` | NO (deliverable) |
| 6 KICKOFF | 11 push (use as needed) | (template) | (used during execution) | — |
| POST | 12 completion-audit | `/bead-completion-audit` | audit report + `audit-fix` beads | yes |
| POST | 13 fix-beads-kickoff | (template) | fix beads agent onboarding | NO (deliverable) |
| POST | 14 fix-beads-push | (template) | fix beads continuation | NO (deliverable) |
| POST | 15 archive | (manual) | pipeline archived; status updated | — |

---

## Execution Order

### Batch 1 — SPEC (validate + optimize the existing layer doc)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 1 | `01-validate-spec.md` | manual | — |
| 2 | `02-optimize-spec.md` | manual | session 1 |

**What to do:** L5 doc IS already a structured spec. Session 1 verifies requirement IDs (`L5-FR-X.Y.Z`), ACs, structure, completeness, scope-drift cleanliness, and resolves the **seven cross-domain spec callouts** (per `manifest.yaml#spec_callouts`):
1. `product_commodity_codes` local SQLite junction (recommended option A: L5 creates `migrations/0013_l5_product_commodity_codes.sql`).
2. Marketplace skill authorship framing (alibaba/amazon/1688 already exist as `candidate` — reframe §1.15 as "mature to stable", PCS as "truly new" since Scotland is IN-scope per operator decision 2026-05-08).
3. PCS Scotland scope decision deadline (must precede step 07 bead creation).
4. ErrorCode enum extension — clarify NOT plug-and-play (Rust enum, recompilation + clippy propagation).
5. HMRC FPO endpoint name verification — preserve as TDD prerequisite.
6. Cron scheduler 4 new entries — pattern reference to existing entries at `crates/daemon/src/scheduler/cron.rs:77-86`.
7. No `.github/workflows/` — bootstrap doc is the local quality gate runbook.

Session 2 is a fresh-eyes audit by an independent agent. Both sessions run scope-drift greps.

**Gate:** Spec has unique `L5-FR-X.Y.Z` requirement IDs, every requirement has explicit ACs, no scope-drift language outside §3.6 Open Questions and §1.14 CBAM placeholder, all 7 callouts have recorded resolutions or deliberate non-resolutions, the `product_commodity_codes` ownership question is resolved, and the L5→L6 readiness gate (5 blocker conditions) is preserved.

### Enrichment after Batch 1

Read the validated spec. Extract:
- Requirement ID range (e.g., `L5-FR-1.1.1` … `L5-FR-3.10.N`) and total count.
- Section count (3 main sections: §1 customs+overview, §2 FX/freight/marketplace, §3 testing/DoD/risks/OQ).
- DoD criterion count (12 base; 13 if no-CI criterion was added per Callout 7).
- Customs error type count (10 in §3.5).
- Updated marketplace skill row count (3 mature + 0-1 new = 3 or 4).
- Decision on PCS Scotland scope (in/out/undecided).
- Decision on `product_commodity_codes` ownership (Option A or B per Callout 1).

Edit handoffs **03-15** — replace `{ENRICHMENT: spec metadata}` markers and any references to "TBD" with actuals.

---

### Batch 2 — TDD (test specification + bootstrap)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 3 | `03-create-tdd.md` | `/spec-to-tdd` | Batch 1 |
| 4 | `04-optimize-tdd.md` | manual | session 3 |

**What to do:** Session 3 invokes `/spec-to-tdd` to produce both the TDD (test file inventory, test catalog, coverage matrix) and the bootstrap doc (env prerequisites, validation script, external API setup). Session 4 audits both with fresh eyes.

**Gate:** Every L5-FR-X.Y.Z has at least one test ID, every DoD criterion (1-12 or 1-13) maps to a verifying test, the 5 planned test files (`hmrc-fpo`, `customs-classification`, `measure-interaction`, `fx-engine`, `freight-engine`) are concretely scoped with exact assertions, the calibration harness has tests for all mutation families per L4 precedent + customs-specific mutations, redaction-boundary tests cover all new sensitive `landed_cost_estimates` fields, the bootstrap doc covers all 6 mandatory + 4 optional external API accounts, all 4 cron entries with cron expressions, the ≥3 gold-set categories named, and the local-only quality gate sequence verbatim.

### Enrichment after Batch 2

Read the TDD. Extract:
- Test ID range (e.g., `L5-T-customs_classification-001` … `L5-T-fx_engine-NN`).
- Test counts by file and by type (unit / integration / E2E / canary / calibration).
- CI tier distribution (T1/T2/T3/T4 budgets).
- Coverage matrix summary (DoD criterion → test IDs).
- Environment prerequisites from bootstrap (HMRC keys, Xe key, Freightos/SeaRates keys, Taric Support, optional clients).
- Final gold-set category list (post-decision).

Edit handoffs **05-15** — replace `{ENRICHMENT: tdd metadata}` markers with actuals.

---

### Batch 3 — STRATEGY (phase ordering + dependency DAG)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 5 | `05-create-strategy.md` | `/spec-to-strategy` | Batch 2 |
| 6 | `06-optimize-strategy.md` | manual | session 5 |

**What to do:** Session 5 invokes `/spec-to-strategy` to produce phase ordering only (no spec/TDD content duplication). Session 6 audits.

**Gate:** Every L5-FR-X.Y.Z appears in some phase, phases are labeled execution-order (NOT importance tiers), later phases have SAME depth as early phases, the 6-phase shape (Foundation → FX → Freight → Customs → Skills → Integration+Gold-Set+CBAM) respects critical-path dependencies, all 5 §3.6 Open Questions explicitly addressed (PCS decision deadline, customs cost spread placeholder threshold, freight defaults, packaging risk, CBAM rate timing), L5→L6 gate producer trace complete, no scope-drift language present.

### Suggested phase shape (heuristic — validate against critical-path analysis in Step 5)

1. **Foundation** — `migrations/NNNN_l5_product_commodity_codes.sql`; `errors/manifest.rs` ErrorCode +10 variants + as_str() + propagation across daemon; `sources/registry.rs` +4-5 entries; `scheduler/cron.rs` +4 entries; `db/sync.rs` PUSH_OWNED_RESEED_TABLES update; env-var loader. **Root of DAG.**
2. **FX Engine** — `packages/intelligence/src/fx/{hmrc-exchange-rates,xe-commercial-rates,fx-engine,fx-types}.ts`; monthly rollover; basis delta; `FX_RATE_MISSING`. **Parallel to 3, 5 after 1 closes.**
3. **Freight Engine** — `packages/intelligence/src/logistics/{mode-selector,freightos-client,searates-client,freight-engine,freight-types}.ts`; dual-source cross-check; mode locks; boundary scenarios; freight error codes. **Parallel to 2, 5.**
4. **Customs Engine** — `packages/intelligence/src/customs/{description-quality-gate,multi-source-recall,ten-digit-resolution,measure-scanner,exporter-resolution,classification-freshness}.ts`; 7 customs error codes; 5-source recall; multi-family expansion; measure interaction; conservative defaults. **Depends on 1 + 2 (FX needed for valuation in measure-scanner / exporter-resolution).**
5. **Skills (Marketplace + Validator Overlay)** — alibaba/amazon/1688 candidate→stable maturation with explicit deltas; PCS skill creation (Scotland IN-scope per operator decision 2026-05-08); recommended `skills/validation/overlay-customs-classification/`; canary harness extension. **Parallel to 2/3/4 after 1.**
6. **Integration + Gold-Set + CBAM + main.rs Refactor** — `customs/accuracy.ts`; 3 comparison adapters; gold-set extension for ≥3 categories; CBAM placeholder logic; daemon main.rs landed-cost refactor with feature flag; Stage 11 live demo. **Depends on 4 + 3 + 5.**

### Enrichment after Batch 3

Read the strategy. Extract: phase names (likely the 6 above), tasks/beads per phase, gate criteria. Edit handoffs **07-15** with actuals.

---

### Batch 4 — BEADS (executable units)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 7 | `07-create-beads.md` | `/beads-from-sources` | Batch 3 |
| 8 | `08-optimize-beads.md` | manual | session 7 |

**What to do:** Session 7 (after PCS Scotland decision is recorded — STOP if undecided) invokes `/beads-from-sources` to materialize beads from the 3 SOTs. Session 8 audits coverage and uniqueness, runs `br lint`, and verifies bead self-containment.

**Pre-conditions for step 07 (verify recorded in spec):** All three step 01 decisions must be present and resolved.

- Decision 1 — `product_commodity_codes` ownership.
- Decision 2 — PCS Scotland scope (IN/OUT). If UNDECIDED, step 07 STOPS and escalates.
- Decision 3 — L5 implementation epic structure. If Option (b) sister epic, the new epic ID is captured.

If any decision is missing, step 07 cannot proceed; re-open step 01.

**Gate:** Bead prefix is `l5`. Every L5-FR-X.Y.Z has a bead. Every L5-T-* test ID is owned by exactly one bead. No bead is empty (every bead has full file paths, type definitions, test specs, ACs, dependency bead IDs). Phase counts are in expected ranges (Phase 1: 4-6, Phase 2: 4-6, Phase 3: 5-7, Phase 4: 12-20, Phase 5: 7-10 [Scotland IN-scope adds PCS], Phase 6: 8-12). Total ~43-58 beads. Run `bin/br-sync && git add .beads/ && git commit && git push`.

### Enrichment after Batch 4

Run `br list --json | jq '.[] | select(.id | startswith("l5"))'`. Extract: total bead count, ready-bead count, distribution per phase. Edit handoffs **09-15** with actuals.

---

### Batch 5 — AUDIT (last gate before execution)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 9 | `09-audit-beads.md` | `/bead-plan-audit` | Batch 4 |

**What to do:** Systematic spec-drift audit against beads. Read EVERY bead, walk EVERY spec L5-FR, walk EVERY TDD test ID, verify ZERO coverage gap, scan for ZERO scope-reduction language. Trace each L5→L6 gate condition to a producing bead.

**Gate:** Coverage matrix shows 100% L5-FR → bead mapping. Zero "post-MVP" / "Phase 7+" / "deferred" / "stretch goal" / "nice to have" language anywhere in the bead corpus. Phase coverage is balanced (no anomalously thin late phase). Verdict: PASS or FAIL.

If FAIL: fix beads, re-run audit. Do NOT proceed to step 10.

---

### Batch 6 — KICKOFF (generate execution prompts)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 10 | `10-generate-kickoff.md` | `/beads-to-execution` | Batch 5 |
| (11) | `11-push.md` | (template; used as needed) | session 10 |

**What to do:** Session 10 produces the kickoff (~400-550 lines, 12 sections matching L4 quality bar), push (~80-130 lines), and status YAML. The kickoff and push are the documents agents use during implementation. Session 11 is a documentation note about how the push is used; no separate skill invocation.

⛔ **MANDATORY STOP — Human Authorizes Execution**

Before agents begin executing L5 implementation:
1. Read the kickoff/push (review against L4 quality bar).
2. Verify AgentMail registration block uses `/Users/alinaaitova/Desktop/Testproject1`.
3. Verify `br ready --json` shows the L5 bead graph in expected order.
4. Verify env vars from bootstrap are set or scoped delegation explicit.
5. Verify quality gates list correct Rust + TS + UBS commands.
6. Authorize parallel agent execution.

---

### Execution Phase

After authorization, agents claim L5 beads and execute. The push prompt (11) is a fresh-context restart for any continuation agent. The kickoff (10) is for agents who haven't seen this work yet.

⛔ **MANDATORY STOP — Human Triggers Completion Audit**

When `br list --status=open --label=l5` returns empty (or only `audit-fix` candidates), the human triggers Batch 12.

---

### POST — Completion Audit + Fix Loop

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 12 | `12-completion-audit.md` | `/bead-completion-audit` | execution complete |
| 13 | `13-fix-beads-kickoff.md` | (template) | session 12 |
| 14 | `14-fix-beads-push.md` | (template) | execution started |
| 15 | `15-archive.md` | manual | all complete |

**What to do:** Session 12 walks every L5-FR-X.Y.Z against the codebase + tests; gaps become `audit-fix` beads. Sessions 13-14 onboard agents to those fix beads. Session 15 archives the pipeline.

**Gate:** Final DoD walkthrough confirms all 12 (or 13) acceptance criteria are met. Stage 11 live demo against a real Contracts Finder lot produces decision-grade landed cost end-to-end. L5→L6 readiness gate (5 blocker conditions) verified producible. Quality gates green. Two-user verification (desktop + web on same dataset) optional but recommended.

---

## Quick Reference

| | |
|---|---|
| Spec | `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` |
| TDD | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md` |
| Bootstrap | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md` |
| Strategy | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md` |
| Kickoff | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-kickoff.md` |
| Push | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-push.md` |
| Status | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-status.yaml` |
| Handoff dir | `docs/pipeline/l5-expanded-commercial-intelligence/` |
| Manifest | `docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml` |
| Bead prefix | `l5` |
| Bead CLI | `br` (NEVER `bd`) |
| Bead sync | `bin/br-sync` (or `br sync --flush-only` + manual `git add .beads/`) |
| Layer-doc epic (existing) | `testproject1-4gh` (children 4gh.1-4 already authored the layer doc; implementation pipeline creates a sister epic) |
| Build | `cargo check --all-targets && bun run build` |
| Test (Rust full) | `cargo test -p daemon --bin daemon && for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" \|\| exit 1; done` |
| Test (TS) | `bun run test` |
| Lint | `cargo clippy --all-targets -- -D warnings && cargo fmt --check && bun run lint` |
| UBS | `ubs $(git diff --name-only --cached)` |
| AgentMail key | `/Users/alinaaitova/Desktop/Testproject1` |

---

## Mandatory Stops Recap

1. **All four operator decisions PRE-RESOLVED 2026-05-08** via pipemaxxer AskUserQuestion (recorded in `manifest.yaml#recorded_decisions`):
   - **Decision 1** — `product_commodity_codes` ownership: **Option A** (L5 SQLite migration `migrations/0013_l5_product_commodity_codes.sql`).
   - **Decision 2** — PCS Scotland scope: **IN-SCOPE**.
   - **Decision 3** — L5 implementation epic structure: **Option (b) sister epic** — `br create --type=epic --priority=P0 --title="Epic: L5 Implementation — Expanded Commercial Intelligence" --label l5` runs in step 07; resulting epic ID captured into spec.
   - **Decision 4** — Gold-set categories (DoD #12): **fasteners + packaging-cartons + examination-gloves**.

   Step 01 §6 transcribes these verbatim into the spec. Step 02 verifies the transcription. No re-deciding.

2. **Skill-first architecture (operator-confirmed 2026-05-08) — ZERO commercial API procurements needed.** Three existing landed-cost skills already cover HMRC Trade Tariff (public API, no key), commercial FX (Frankfurter, no key), SeaRates (CDP browser, no key). Two new skills are **operator-built prerequisites** that must exist before step 09 bead audit can pass:
   - `skills/landed-cost/freightos-freight-estimate/` — CDP browser skill targeting Freightos public rate explorer; **operator-built**. **Build handoff:** `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/HANDOFF-freightos-freight-estimate.md`.
   - `skills/landed-cost/taric-cross-check/` — CDP browser skill targeting public EU TARIC database (taric.ec.europa.eu); **operator-built**. **Build handoff:** `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/HANDOFF-taric-cross-check.md`.

   These are NOT pipeline beads. Step 07 (bead creation) verifies their existence via Step 1.5; step 09 (bead audit) re-verifies before authorizing execution. Operator's free time can run in parallel with steps 01-08 of the pipeline. Estimated operator effort: ~2-3 hours per skill. See `manifest.yaml#recorded_decisions.external_api_account_state` for full inventory + cost-savings analysis, and `operator-built-skills/README.md` for the build sequence.

3. **Before Batch 6 step 10:** human reviews kickoff + push, authorizes execution.
4. **After parallel execution closes all task beads:** human triggers Batch 12 completion audit.
5. **After fix beads close:** human verifies Stage 11 live demo + L5→L6 gate producible, then authorizes archive (15).

---

## Timeline Estimate

| Batch | Approx | Notes |
|-------|--------|------|
| 1 SPEC | 2-4 hours | Validation + 7-callout resolution (PCS Scotland pre-resolved IN-scope; product_commodity_codes pre-resolved Option A; epic structure pre-resolved Option (b); gold-sets pre-resolved fasteners+cartons+gloves; QA marketplace baseline pre-confirmed clean) + fresh-eyes audit. Validator-agent transcribes operator decisions; does not re-decide. |
| 2 TDD | 5-9 hours | TDD generation is largest artifact (5 test files spanning many ID ranges); bootstrap is independent and can parallelize |
| 3 STRATEGY | 2-4 hours | Phase ordering only |
| 4 BEADS | 4-10 hours | 40-55 beads expected (customs phase alone is 12-20 beads) |
| 5 AUDIT | 2-4 hours | Systematic walk |
| 6 KICKOFF | 1-2 hours | Single session output |
| Execution | weeks | Multi-agent parallel; depends on team size, customs engine complexity, gold-set calibration cycles |
| POST | 2-5 days | Audit → fix beads → re-verify loop |

---

## Notes for Pipemaxxer Re-Runs

- This pipeline targets L5 only. **Do not** mix L4 or L6 work into these handoffs.
- If something in the L5 spec proves impossible during execution, prefer **fix beads + spec amendment** over silent scope reduction. Scope reduction = blocking failure.
- The `product_commodity_codes` table is L5's single new local SQLite table. The recommended path is `migrations/0013_l5_product_commodity_codes.sql` (or whatever next-free `NNNN` is at the time). The L5 spec's original `sqlite/migrations/0002_*` reference is incorrect for this repo — flat `migrations/NNNN_*.sql` is the convention.
- Marketplace skill maturation framing: alibaba/amazon/1688 already exist as `promotion_stage: candidate`. L5's job is to mature them to `stable` with specific behavioral deltas — NOT to create them from scratch. Operator confirmed 2026-05-08 these worked cleanly during QA stages 1-14, so maturation is spec-listed deltas only. PCS is the only truly net-new skill (Scotland IN-scope per operator decision 2026-05-08).
- ErrorCode enum extension is a Rust compile-time change. The Phase 1 ErrorCode bead must be sized for clippy-surfaced exhaustive match propagation across ~5-15 sites in the daemon.
- L5→L6 readiness gate is constitutional. The 5 blocker conditions (`CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`, classification freshness expired) MUST be producible by the end of L5. L6 cannot start until L5's archive verifies this.
- CBAM is placeholder-only in L5 (rate-bearing logic awaits secondary legislation; trial Q4 2026, full from 2027-01-01 — within tender-window reach today). The future-regulatory-exposure flag emission ships; rate computation does NOT.
