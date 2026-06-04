# Researchmaxer — PASS 2: Audit & Test Pass

> **Target document:** `docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md`
> **Run:** researchmaxer run 0 · generated 2026-06-01
> **Prerequisite:** Run **Pass 1 (improvement-pass.md) first.** This pass assumes the drift corrections are already applied. Read the "Improvement Pass — change log" at the top of the doc before you start.
> **You run this in ONE session:** audit → find gaps → define the full testing strategy → edit the target doc → fresh-eyes verify.

---

## 1. MISSION

You are the **last quality gate before implementation**. The improvement pass made the doc match the codebase; your job is to (a) hunt the remaining design gaps, gotchas, and contradictions, and (b) author a **complete, REAL E2E testing strategy** the doc currently only sketches.

Project rule (from `AGENTS.md`): **REAL end-to-end agent tests — no mocks, no stubs.** Every behavioural claim in this feature must be pinned by a named, runnable test with an expected verdict. The doc already has an excellent **Appendix A scenario corpus (11 cases)** — your testing strategy must turn those into concrete fixtures wired to the real harnesses that exist in this repo, and fill every coverage gap.

**The target document is the ONLY file you may edit.** Codebase, L0 doc, AGENTS.md are READ-ONLY.

---

## 2. WHAT TO READ
- The improved target doc (all of it, especially Appendix A, §6.3, §6.4, §8.4) + its Pass-1 change log.
- `docs/researchmaxer/active/improvement-pass.md` — for the embedded findings (do not re-verify; reuse).
- **Test harnesses (verified to exist — these are what your strategy must target):**
  - `crates/daemon/tests/skill_canary_harness.rs` — loads `canary.inputs.jsonl` (one JSON case/line) + asserts against `golden.assertions.yaml`; validates corpus dir has `metadata.json` (`row_count` must match), `validator-rubric.yaml`, `manifest.md`.
  - `crates/daemon/tests/decision_basis_hash.rs` — the **qualify→candidate→quote→landed-cost** seeding pattern (`seed_candidate_quote_and_landed_cost` inserts into `product_candidates`, `supplier_quotes`, `landed_cost_estimates` with deterministic values; asserts FK integrity + reproducible cost).
  - `crates/daemon/tests/customs_fx_freight_engine.rs` — landed-cost E2E + outbox + **schema-epoch assertion** pattern (`assert … user_version == DEFAULT_SCHEMA_EPOCH`).
  - `crates/daemon/tests/skill_lint.rs` — required frontmatter (`skill_id, skill_version, interaction_profile, result_schema_version, model_profile, agent_cli, side_effect_class`); required SKILL.md sections (`## Non-goals`, `## Failure Codes`, `## Resume Strategy`, run-manifest tokens); promotion-required files for overlays; `CDP_BROWSER_SKILLS`.
  - `packages/intelligence/src/qualification/goods-qualification.test.ts` — **5 cases** (TS-QUAL-001…005), `bun:test`, fixture shape: `qualifyGoodsOpportunity({ cpvCodes, totalValuePence, goodsAddressableValuePence, goodsSeparability, goodsDensitySignals, … })` → assert `legalCategory`, `addressableValuePence`, `qualificationResult`.
  - `packages/intelligence/src/margins/coverage-rollup.test.ts` — **6 cases** (TS-COV-001…006), `bun:test`, fixture: `computeCoverageRollup({ lots:[{ candidates:[{ candidateValueShareBps, … }] }] })`.
  - **NO `description-quality.test.ts` exists** (the riskiest gate has zero coverage today).
- `docs/strategy/layers/layer-0-gold-set-evaluation.md` lines 96–122 — whitelist (6 cats) + no-list (8) + the expansion rule "Each new category requires its own gold-set validation before entering production" (this is a **test gate** the feature must satisfy per added category).
- Stage-validation gate in `AGENTS.md`: a stage manifest is NOT green on `cargo test -p daemon --bin daemon` alone — it must also run **each** `crates/daemon/tests/*.rs` integration target. Your E2E plan must respect this gate.

---

## 3. EMBEDDED TESTING LANDSCAPE (verified)

**What exists and is reusable:**
| Layer | Real harness | Pattern to mirror |
|---|---|---|
| Unit (TS) | `bun:test` | `goods-qualification.test.ts` (TS-QUAL-*), `coverage-rollup.test.ts` (TS-COV-*) |
| Unit (Rust) | `cargo test -p daemon --bin daemon` `#[test]` | rollup/persistence assertions |
| Skill canary | `skill_canary_harness.rs` + `{canary.inputs.jsonl, golden.assertions.yaml, validator-rubric.yaml, metadata.json, manifest.md}` corpus | the existing calibration corpora under `tests/fixtures/calibration/` |
| Gold-set | **fixture corpus + integration tests** (NOT `evaluation_runs`/`evaluation_case_results` — those tables do not exist; L0 is PARTIALLY BUILT) | `skill_canary_harness.rs` |
| E2E pipeline | `crates/daemon/tests/*.rs` seeding real rows, no mocks | `decision_basis_hash.rs`, `customs_fx_freight_engine.rs` |

**Coverage gaps (verified):**
- `description-quality.ts` — **no test file at all**. The service-gate split is rated Medium-High risk; it ships untested today.
- No existing test asserts a labour/permit cost component (the fields don't exist yet).
- No existing test exercises the **Phase-2 dominance gate** (it doesn't exist yet) or the **no-list/bespoke flag** (new).
- The labour skill needs a **new** canary corpus dir mirroring the calibration-corpus file set.

**E2E seeding reality:** `decision_basis_hash.rs` shows landed-cost rows are seeded with explicit pence values (`supplier_cost_gbp_pence`, `freight_*_gbp_pence`, `customs_duty_gbp_pence`, `import_vat_gbp_pence`, …). Your E2E manifests for labour must extend this exact seeding pattern with the new labour/permit columns and assert the `economic_landed_cost_gbp_pence` rollup *includes* them and that `goods_value_share_bps` is computed and gates correctly.

---

## 4. AUDIT DIRECTIVES

Audit each subsystem for implementation-readiness. For each, the doc must answer: complete enough to implement? failure modes documented? edge cases addressed? integrates cleanly with the corrected (Pass-1) integration model? what gotchas will an implementer hit?

**Per-subsystem:**
1. **Two-phase qualification.** Does the Phase-1 branch (after line 353, before 355) actually produce the intended routing for every Appendix-A class, given the *verified* 319–372 logic? Trace each of the 11 scenarios through the real branch order (note `goodsConfidenceBps < 6000 → review` at 339 fires **before** the category branches — does that interact with acceptance scenarios?). Does downgrading the line-347 separable-mixed `pass` branch regress Scenario 8 (pure paper) or TS-QUAL-003 (which currently expects `pass` for a separable mixed contract)? **TS-QUAL-003 is a live test that expects `pass` for mixed+separable — the Phase-1 change may break it. Flag this explicitly.**
2. **Service-gate split** (`isServiceDescription`). Zero existing coverage. Enumerate the false-positive/false-negative space: "supply **and installation** of shelving" (must pass), "installation services" (must reject), "maintenance of grounds" (reject), "supply of maintenance kits" (goods! must pass). Does the split reuse the existing `DescriptionQualityVerdict` union (`split_safe`/`reject_bundle_unsafe`) or invent a parallel path? Audit for regex catastrophe / over-admission.
3. **Labour skill + reference data.** With the Pass-1 correction (skill-local reference JSON, no `data/reference/` loader), is the skill self-contained? Does it pass `skill_lint.rs` (required frontmatter, `## Non-goals`/`## Failure Codes`/`## Resume Strategy`, run-manifest tokens)? Is its `interaction_profile` defensible and does it avoid the CDP requirement?
4. **Rollup integration.** Labour must land at **both** domestic (8114) and import (8548) sites. Does the doc's rollup edit cover both? Does adding labour to `economic_landed_cost_gbp_pence` change behaviour for **already-assessed** processes / existing tests in `customs_fx_freight_engine.rs`?
5. **Phase-2 dominance gate.** At decision synthesis, is the **goods-only** landed cost available separately from the labour-inclusive total to compute the ratio? Where exactly (function + line)?
6. **Recurring maintenance.** Can "capital-event-only dominance + separate annualised per-period margin" be represented in the current `LandedCostEstimateRow`/`PipelineAssessmentRow` schema, or does it need a new entity/columns? This is under-specified — audit hard.
7. **No-list/bespoke flag.** New runtime check. Where does it live (qualification? a new pass?), and how is it tested for Scenario 11 (bespoke windows, goods-dominant)?

**Cross-cutting:** contradictions between sections after Pass-1 edits; dependency cycles; the weakest point; what a hostile reviewer challenges first; the schema-epoch bump coordination (daemon + 3 desktop constants + Supabase) as an implementation gotcha.

---

## 5. TESTING STRATEGY REQUIREMENTS (the core deliverable)

Rewrite/expand §6.3 + §6.4 (and add an Appendix if needed) so the doc contains a **complete** strategy. Required parts:

**a. Existing-test impact.** For every existing test the feature touches, name it and state the change. **Mandatory:** `goods-qualification.test.ts` TS-QUAL-003 expects `pass` for separable mixed — state whether the Phase-1 change alters its expected verdict and how the test must be updated. Audit `customs_fx_freight_engine.rs` / `decision_basis_hash.rs` for rollup-total assertions that will shift once labour is summed.

**b. New unit/integration tests.** Enumerate concretely, each referencing the exemplar to mirror:
- `goods-qualification.test.ts` (extend): Phase-1 provisional-admit branch; the `addressableValuePence` contract-value dominance basis (preferred) vs cost-proxy fallback.
- `description-quality.test.ts` (**NEW** — mirror `bun:test` style of customs-classification.test.ts): the full goods-led-mixed vs pure-service truth table.
- `coverage-rollup.test.ts` (extend): `goods_value_share_bps` math + labour/permit coverage metrics (mirror TS-COV-* fixtures).
- Rust `#[test]`: rollup includes labour at both domestic+import sites; field persistence (mirror `decision_basis_hash` seeding).

**c. E2E test manifests (REAL — no stubs).** For **each of the 11 Appendix-A scenarios**, write a manifest: scenario name, preconditions (seeded process/notice/lot/candidate/quote rows, the labour reference-table version), steps (run the real deterministic pipeline + the labour skill invocation in `run_landed_cost_live_loop`), expected outcome (legal category, `goods_value_share_bps` band, labour line items, permit line + responsibility flag, final decision), and verification method (which assertion in which integration-test file). Anchor them to the `decision_basis_hash.rs` seeding pattern. Scenario 1 (festive lights) is the **engine acceptance** case: its base labour estimate must land inside the §4.5 NW band (~£2,100–2,650) or the rate table is wrong — make that a hard assertion.

**d. Skill canary corpus.** Define the `uk-installation-labour-estimate` canary corpus: the file set (`canary.inputs.jsonl` with `metadata.json` row_count match, `golden.assertions.yaml`, `validator-rubric.yaml`, `manifest.md`) mirroring the calibration corpora, and the canned `job_parameters` → asserted 3-point band cases (incl. Scenario 1 and Scenario 9 CCTV to prove the resolver generalises across goods→trade).

**e. Browser-based tests.** The labour skill itself is headless (no CDP) — but the **desktop manifest/contract-card rendering** of the labour breakdown is UI. Per `AGENTS.md`, browser/UI testing uses the `/agent-browser` skill. Define a real UI verification: render a `review`-routed goods-led contract, assert the labour line (low/base/high/confidence) + permit row + responsibility flag appear in the manifest viewer and contract card. Specify it as a real agent-browser flow, not a stub.

**f. Verification criteria.** For each subsystem, define what "correctly implemented" looks like + the exact command/assertion that proves it + what indicates regression. Respect the `AGENTS.md` stage gate: the green bar is `cargo fmt --check` + `cargo check --all-targets` + `cargo clippy --all-targets -- -D warnings` + `cargo test -p daemon --bin daemon` + **every** `crates/daemon/tests/*.rs` target + the `bun test` suites. The **promotion gate** (per L0 expansion rule + the doc's own §6.4): 100% of negative + regression controls pass, acceptance precision ≥ agreed floor, evaluated through the canary/fixture harness before Canary→Shadow promotion.

---

## 6. WHAT A GREAT OUTCOME LOOKS LIKE
- Every Appendix-A scenario has a real, runnable E2E manifest anchored to an actual harness file — a fresh implementer could build + verify the feature from the doc + manifests alone.
- The zero-coverage service gate now has a complete truth-table test plan.
- The TS-QUAL-003 regression risk (and any rollup-assertion shifts) are surfaced with the exact test edits.
- The labour-skill canary corpus is fully specified against the verified corpus file-set and `skill_lint.rs` requirements.
- No mock/stub anywhere; all tests exercise real runtime paths.
- Verification criteria are specific enough to gate promotion, and consistent with the `AGENTS.md` stage gate.

## 7. WHAT I DON'T WANT
- No mock/stub-based E2E (project constitutional rule).
- Don't edit any file but the target doc.
- Don't invent harnesses/tables that don't exist (no `evaluation_runs`); target the real ones.
- Don't duplicate Appendix-A prose — *operationalise* it into fixtures + assertions.

## 8. FOLLOW-UP FRESH-EYES PASS (run in the SAME session, after editing)
Re-read the doc + the new testing strategy cold. Verify:
1. **No test-coverage gap:** every behavioural claim (each MVP success criterion in §2.3, each Appendix-A scenario, each new warning code in §5.6) maps to at least one named test.
2. **E2E manifests are complete & real:** preconditions/steps/expected/verification all present; each names a real harness file; none rely on mocks.
3. **Verification criteria are specific:** concrete commands + expected outputs, not "verify it works."
4. **No contradictions:** the testing strategy agrees with the Pass-1-corrected integration model (skill loaded transiently, sequential invocation, skill-local reference data, schema-epoch bump).
5. **Promotion gate is explicit and matches the L0 expansion rule + AGENTS.md stage gate.**
Fix anything that fails, then append a 6–10 line "Audit & Test Pass — change log" beneath the Pass-1 change log, listing the gaps you closed and the test artifacts you specified. The doc is then optimized and test-strategy-complete → ready for `/research-to-spec`.
