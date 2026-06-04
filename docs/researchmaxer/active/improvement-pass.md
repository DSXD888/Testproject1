# Researchmaxer — PASS 1: Improvement Pass

> **Target document:** `docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md` (1009 lines)
> **Run:** researchmaxer run 0 · generated 2026-06-01
> **You run this in ONE session:** research → improve → edit the target doc in place → fresh-eyes verify.

---

## 1. MISSION

You are improving a **research document** for a UK public procurement platform feature: broaden the pipeline to admit **goods-led supply-and-install / supply-and-maintain** contracts (while still rejecting service-only work), and add a **UK installation-labour + access-equipment + permits cost engine** as a new peer term in the existing landed-cost rollup.

**You are NOT implementing.** You are researching, analysing, and improving the *document* so that it precisely matches codebase reality and is implementation-ready. The doc is already strong and heavily grounded — it cites `file:line` throughout. Your job is **not** to rewrite it but to **correct the places where it has drifted from the actual code, fill the gaps the drift exposes, and sharpen the modelling**.

This skill has already done deep codebase research for you (Phase-1 verification of ~40 claims). **The verified findings and confirmed drifts are embedded below — treat them as ground truth.** Do not re-verify from scratch; spend your effort on the *deeper* analysis the embedded findings unlock.

**The target document is the ONLY file you may edit.** Everything else (codebase, strategy docs, L0 doc) is READ-ONLY context.

---

## 2. SOURCE-OF-TRUTH HIERARCHY

1. **The live codebase** = ground truth for what exists today. Where the doc and the code disagree, the code wins and the doc must be corrected.
2. **`docs/strategy/layers/layer-0-gold-set-evaluation.md`** = authoritative on category whitelist / no-list / expansion rule (READ-ONLY).
3. **`AGENTS.md`** = project rules (money in integer pence; bps for ratios; no mocks/stubs in E2E; skills are registry-bound *where applicable*; Rust 2024 nightly, `forbid(unsafe_code)`; Bun for TS).
4. **The target doc** = the artifact you improve.

---

## 3. WHAT TO READ

Read these before editing. Focus notes tell you what to extract.

### Target doc
- `docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md` — read all 1009 lines twice. First pass: what it claims. Second pass: where the embedded drift findings (§4 below) force corrections, and what *downstream* claims those corrections invalidate.

### Codebase ground truth (read the regions cited; they are verified-current)
- `packages/intelligence/src/qualification/goods-qualification.ts` — **lines 319–372** (`determineQualificationDecision`), **lines 290–317** (`computeAddressableValue` — the existing surface the doc MISSED, see §5), **lines 264–288** (`computeGoodsDensityScore`), **lines 50–59** (access-route sets), **lines 21–34 / 14–19** (input types). Call site at **line 108** (`qualifyGoodsOpportunity`).
- `packages/intelligence/src/customs/description-quality.ts` — **lines 100–113** (`gateDescriptionQuality`), **lines 360–364** (`isServiceDescription`), **lines 23–36** (the `DescriptionQualityVerdict` union — note it already has `split_safe` and `reject_bundle_unsafe` variants the doc ignores).
- `packages/intelligence/src/margins/coverage-rollup.ts` — `computeCoverageRollup` (lines 92–152) and the `CoverageMetrics` interface (lines 23–31). Note: it *consumes* `candidateValueShareBps` but computes **no** value-share itself.
- `packages/contracts-schema/src/index.ts` — `ProductCandidateRow` (36–75), `SupplierQuoteRow` (77–130), `LandedCostEstimateRow` (166–191), `PipelineAssessmentRow` (216–241).
- `crates/daemon/src/main.rs` — `LandedCostEstimateWrite` struct (**5361–5432**, NOT –5421); `uk_final_mile_cost_gbp_pence` (field 5403, set `0` at 8213+8630, persisted 8933+8992, read 9064); import rollup `build_import_landed_cost_estimate` (**8548–8557**, `.saturating_add` chain at 8548–8552; estimate written at 8632); domestic rollup `build_domestic_landed_cost_estimate` (**line 8114**, supplier-cost-only); `net_margin_bps()` (8033); the sequential landed-cost driver `run_landed_cost_live_loop` (~9140); skill loads at **9131–9151**; decision synthesis `build_decision_grade_landed_cost_candidate` (5502) + `authoritative_landed_cost_estimate` (~9210/9493).
- `crates/daemon/src/sources/registry.rs` — `PromotionStage` (123–131), `SkillBinding` (157–166), `SourceDefinition` (188–199), `builtin_definitions()` (581+). **Note: NO landed-cost skill is registered here** — only procurement/marketplace sources.
- `skills/landed-cost/freightos-freight-estimate/` and `skills/landed-cost/searates-freight-estimate/` — the freight skill templates (see drift §4.6). Read the frontmatter of both, of `simple-fx-lookup` (`http_public`) and `hmrc-customs-classification` (`pty_interactive`).
- `skills/validation/overlay-supplier-enrichment/` — the overlay template (7 files incl. `validator-rubric.yaml`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `checkpoint.schema.json`).
- `crates/daemon/tests/skill_lint.rs` — required frontmatter fields + required files for promotion + `CDP_BROWSER_SKILLS`.

---

## 4. EMBEDDED RESEARCH FINDINGS — VERIFIED TRUTH & CONFIRMED DRIFTS

The skill verified the doc's claims against the code. **Below, ✅ = doc is correct (leave alone); ⚠️ = DRIFT, the doc is wrong and you must correct it.**

### ✅ VERIFIED CORRECT (do not touch these — they are accurate and well-cited)
- `determineQualificationDecision` at **319–372** with the exact branch logic the doc quotes. The `goodsConfidenceBps < 6000 → review` floor is at **line 339** (and rationale at 97). Call at **line 108**. ✅
- `categoryForDivision` 225–239 (45→works, 14–44→goods, 50–98→services); `classifyLegalCategory` returns `mixed` when ≥2 categories. ✅
- `REVIEWABLE_ACCESS_ROUTES` / `REJECTED_ACCESS_ROUTES` at 50–59, exactly as quoted. ✅
- `gateDescriptionQuality` 100–113 and the `isServiceDescription` regex at 360–364, verbatim match. ✅
- `LandedCostEstimateWrite.uk_final_mile_cost_gbp_pence` (5403) is **always `0`** (8213 domestic, 8630 import), yet **persisted** (8933, 8992) and **read back** (9064). The ADR-003 "zero-migration fallback" claim is sound. ✅
- The import rollup is `supplier + freight + duty + import_vat` via `.saturating_add` (8548–8552). ✅
- All four contracts-schema row types have the fields the doc claims (`candidate_value_share_bps`, `critical_attributes_json`, `incoterm`, `economic_landed_cost_gbp_pence`, `net_margin_{low,base,high}_bps`, `landed_cost_coverage_bps`, etc.). `candidate_type` is an unconstrained `string` (so `installed_goods` is *allowed* but not an enforced enum — the doc slightly overstates "already supports values like installed_goods"; soften to "the column is a free string, so a new value needs no migration"). ✅-ish
- `SkillBinding`/`SourceDefinition`/`PromotionStage`/`builtin_definitions()` at the cited lines. ✅
- 6 validation overlays exist; `overlay-supplier-enrichment` exists with `validator-rubric.yaml`. ✅
- L0 doc has exactly 6 whitelist categories + the 8-item no-list + the expansion rule "Each new category requires its own gold-set validation before entering production" — **verbatim** as the doc claims. The L0 doc *already* flags metal furniture/shelving as HIGH trade-remedy and packaging as split easy/dangerous. ✅
- Real E2E test patterns exist (`crates/daemon/tests/decision_basis_hash.rs`, `customs_fx_freight_engine.rs` seed process→candidate→quote→landed-cost with no mocks). Skill canary harness exists (`skill_canary_harness.rs` + JSONL/YAML fixtures). ✅

### ⚠️ DRIFT 1 — Landed-cost skills are NOT registered in `registry.rs` (corrupts ADR-002 + §3.4 + §3.6 + §8.2)
**The doc's central integration claim is wrong.** §3.6 and ADR-002 say the labour engine is "registered in `registry.rs` `builtin_definitions()` … exactly like the existing freight/customs skills." **It is not.** `registry.rs` registers only *procurement/marketplace sources* (FTS, Contracts Finder, Alibaba, …). The landed-cost skills (`searates-freight-estimate`, `hmrc-customs-classification`, `shipment-sizing`, `simple-fx-lookup`) are **loaded as transient skill artifacts** via `load_landed_cost_skill(..., "skills/landed-cost/<name>/SKILL.md")` at **main.rs:9131–9151** and invoked **sequentially inside `run_landed_cost_live_loop` (~main.rs:9140)**.

**Correction required:** rewrite ADR-002, §3.4, §3.6, and the §8.2 "registry.rs" row. The real integration for the labour skill is:
1. add a `load_landed_cost_skill(...)` call alongside the others at ~9131–9151, and
2. add a sequential invocation in `run_landed_cost_live_loop` **after the FX step, before `build_import_landed_cost_estimate` (decision synthesis)**.
The `registry.rs` edit the doc lists as a "modified file" is likely **unnecessary** (unless the team decides to promote landed-cost skills into the registry as a *separate* refactor — call that out as out-of-scope). Net effect: **more** `main.rs` surgery than the doc implies, **less** registry work. Re-examine the "Estimated Complexity / Quick Stats / §8.2" accordingly.

### ⚠️ DRIFT 2 — No `next_job_kind` chaining for Stage 11 (corrupts §3.4)
§3.4 says the landed-cost skills are "chained by `next_job_kind` (`jobs/runner.rs`)." **False.** They run **sequentially in a single async function** (`run_landed_cost_live_loop`), not via the job-runner's `next_job_kind` mechanism. Job IDs use a `job-stage11-{kind}-{uuid}` *prefix* (main.rs:7527), which is probably where the doc's "stage11_*" intuition came from — but the **job-kind identifier strings are `landed_cost_*`**: `landed_cost_searates_freight`, `landed_cost_simple_fx`, `landed_cost_customs_classification`, `landed_cost_shipment_sizing`. The real execution order is **Customs → Sizing → Freight → FX → synthesis**. Correct §3.4 (and remove the "confirm the precise naming at spec stage" hedge — it's now confirmed: `landed_cost_uk_installation_labour` would be the new kind).

### ⚠️ DRIFT 3 — `interaction_profile` for the labour worker skill (corrupts §5.2 Component B)
Component B proposes the labour skill be `interaction_profile: headless_contract`, "closer to `simple-fx-lookup`/`hmrc-customs-classification`." But: `simple-fx-lookup` is **`http_public`**, `hmrc-customs-classification` is **`pty_interactive`** (browser/CDP!). `headless_contract` is the profile used by **validation overlays** and manifest-generation, **not** by operational landed-cost workers. So the analogy is broken. Resolve this: the labour skill reads curated tables + does bounded LLM inference with **no browser** → the honest fit is **`http_public`** (no `requires_cdp`) or a justified `headless_contract` if the team treats it as a pure contract-mode compute skill. Either way, correct the false "like fx/hmrc" framing, and confirm against `skill_lint.rs` which profile values are accepted. (Component F, the *overlay*, IS correctly `headless_contract` — overlays use it.)

### ⚠️ DRIFT 4 — `data/reference/` does not exist; no versioned-reference-JSON loader (corrupts §5.2 Comp C, ADR-004, §8.3, §9.4, Quick Stats)
The doc proposes 4 files under `data/reference/` loaded "at daemon start (parallel to freight provider config)." There is **no `data/reference/` directory, no startup reference-JSON loader, and no 'freight provider config' loaded at startup** — freight skills fetch live; reference/seed data today lives as **cassette fixtures under `tests/fixtures/cassettes/`** (e.g. `l5/fx/hmrc-monthly-current.json`). This is a **new convention** the feature must *establish*, not reuse. Correct the doc to either: (a) establish a new `data/reference/` dir + a new daemon loader (and own that as real new work — bumps complexity), or (b) ship the rate tables **inside the skill directory** (`skills/landed-cost/uk-installation-labour-estimate/reference/*.json`) read by the skill itself (no daemon loader needed — likely the cleaner fit, since the skill, not the daemon, needs the rates). Reflect the choice in ADR-004, §5.2-C, §8.3, §9.4, and the modularity checklist.

### ⚠️ DRIFT 5 — Freight skill template specifics (corrupts §3.6, §8.1, §8.3)
Both `freightos-freight-estimate` and `searates-freight-estimate` exist (the doc names only `freightos`). **Neither has a `checkpoint.schema.json`** — the doc repeatedly claims the freight skill has one and lists it as a file to create/mirror. `freightos` has `{SKILL.md, eval.js, result.schema.json}`; `searates` has `{SKILL.md, result.schema.json}`. The result envelope uses `freight_base_amount_minor` / `freight_low_amount_minor` / `freight_high_amount_minor` (+ `evidence`/`warnings`/`blockers`), confirming the doc's "`*_base/low/high_amount_minor` shape" intuition. Both freight skills are `pty_interactive` + `requires_cdp: true`. **Correction:** name both freight skills; drop `checkpoint.schema.json` from the worker-skill file list (it belongs to *overlays*, per `skill_lint.rs` promotion requirements); keep the result-envelope shape claim.

### ⚠️ DRIFT 6 — Gold-set harness tables don't exist (corrupts §6.3, §6.4)
The doc cites a gold-set harness using tables `evaluation_runs` / `evaluation_case_results`. **These tables do not exist** in any migration. L0 is **"PARTIALLY BUILT"** (per the L0 doc's own status note): gold-set lives as **JSONL + `golden.assertions.yaml` fixtures** under `tests/fixtures/calibration/` exercised by Rust integration tests (`skill_canary_harness.rs`), not as DB tables. Correct §6.3/§6.4 to describe the *actual* harness (fixture-corpus + canary harness + integration tests), and note that a persistent evaluation-table harness would itself be net-new work if desired.

### ⚠️ DRIFT 7 — `description-quality.test.ts` does not exist (corrupts §8.4)
§8.4 lists `description-quality.test.ts` as "(extend)". There is **no** test file for `description-quality.ts` (the customs dir has `customs-classification.test.ts`, `hmrc-fpo.test.ts`, `measure-interaction.test.ts`). The service-gate split is the **highest-risk change in the feature** (per the doc's own complexity table) yet has **zero existing test coverage** — so this is a **new** test file and the doc should flag the current zero-coverage as a risk amplifier. Correct "(extend)" → "(NEW)".

### ⚠️ DRIFT 8 — Schema epoch is live-inconsistent (sharpens §9.3 authority note)
Daemon `DEFAULT_SCHEMA_EPOCH = 9` (`crates/daemon/src/db/sqlite.rs:82`). Desktop is **internally split**: `main.rs` = 9, but `daemon_bridge.rs:16` and `query_service.rs:14` = **8**. Supabase `schema_metadata` = 9. A new migration bumping to **10** must update the daemon constant, **all three** desktop constants, and the Supabase metadata, or reads break (see the project's known "desktop/daemon schema-epoch coupling" gotcha). Sharpen §9.3's authority-matrix note from a vague "confirm at spec stage" into this concrete checklist, and confirm whether `landed_cost_estimates` / `pipeline_assessments` are remote-projected (`infra/supabase/migrations/0009_l5_field_level_projection.sql` is the place to check).

---

## 5. THE MISSED SURFACE — existing early `addressableValuePence` (HIGH-VALUE INSIGHT)

The doc asserts (§3.2) that "the goods value-share **cannot** exist at the early stage." That is true **only for the cost-based share**. But the codebase **already** computes a **contract-value-based goods portion at qualification time**:

- `qualifyGoodsOpportunity` accepts `totalValuePence` + `goodsAddressableValuePence` and returns `addressableValuePence`.
- `computeAddressableValue()` (goods-qualification.ts **290–317**): for `goods` → whole `totalValuePence`; for `mixed`/`works` **with separability** → the passed-in `goodsAddressableValuePence`; else `null`.
- This is **exactly the "prefer a contract-value / quantity goods-share basis" mitigation** the doc recommends in §5.5 as superior to the cost proxy — and it is **already partially wired**.

**Improvement directive:** integrate this. The doc's §5.5 "basis caveat" and ADR-001 should explicitly reconcile the **new** Phase-2 `goods_value_share_bps` (cost-proxy, post-Stage-11) with the **existing** early `addressableValuePence` (contract-value basis). Concretely: when `goodsAddressableValuePence` is supplied (tender itemises goods vs labour/works), the *contract-value* share `addressableValuePence / totalValuePence` is the **preferred** dominance basis and is available **at Phase 1** — only when it is absent does the pipeline fall back to the Phase-2 cost proxy. This materially strengthens the two-phase design (it may let some dominance decisions happen early) and directly de-risks the §5.5 "false reject" problem. Also note `computeGoodsDensityScore` (264–288) is **text-signal-based** (CPV base + mention counts), so the proposed Phase-1 `GOODS_PRESENCE_FLOOR` maps naturally onto `goodsDensityScore` — make that mapping explicit.

---

## 6. IMPROVEMENT DIRECTIVES (each grounded in the findings above)

**CORRECT (drift fixes — highest priority):**
- §3.4, §3.6, ADR-002, §8.2: rewrite the skill-integration model per **Drift 1 & 2** (transient `load_landed_cost_skill` + sequential `run_landed_cost_live_loop` insertion after FX; job-kind `landed_cost_uk_installation_labour`; registry edit likely unnecessary).
- §5.2 Component B: fix the `interaction_profile` per **Drift 3** (`http_public` or justified `headless_contract`; kill the false fx/hmrc analogy).
- §5.2-C, ADR-004, §8.3, §9.4, Quick Stats: fix the reference-data convention per **Drift 4** (no `data/reference/` today; choose skill-local vs new-loader and own the consequence).
- §3.6, §8.1, §8.3: fix freight-template specifics per **Drift 5** (two skills; no `checkpoint.schema.json` for workers).
- §6.3, §6.4: fix gold-set harness per **Drift 6** (fixtures + canary harness + integration tests; no `evaluation_*` tables).
- §8.4: `description-quality.test.ts` is **NEW**, and current zero coverage on the riskiest gate is itself a risk (**Drift 7**).
- §9.3: concrete schema-epoch checklist per **Drift 8**.

**EXPAND / STRENGTHEN:**
- §5.5 + ADR-001: integrate the existing `addressableValuePence` contract-value basis (**§5 above**) as the preferred Phase-1 dominance basis with the cost proxy as fallback.
- §3.2: map `GOODS_PRESENCE_FLOOR` onto the actual `goodsDensityScore` derivation; show the concrete new branch position relative to the verified 319–372 code (it sits after line 353, before the line-355 branch, exactly as the doc sketches — confirm the sketch still lines up after you re-read).
- §3.3: the `DescriptionQualityVerdict` union **already** has `split_safe` and `reject_bundle_unsafe` variants. The service-gate split should reuse/extend that taxonomy (add a `goods_led_mixed` verdict) rather than inventing a parallel mechanism — strengthen §3.3 with the real union shape.
- §5.3 / Quick Stats: the labour cost must be added at **both** the domestic (main.rs:8114, currently supplier-only) and import (8548) construction sites, plus both `0`-literal sites (8213, 8630). The doc under-counts the rollup touch-points — make them explicit.

**ALIGN (terminology / project conventions):**
- The bidder is **Sorellon** (double-L). The doc and the codebase carry a systemic single-L `sorelon`/`sorelon.com` typo. Do **not** mass-fix code, but ensure the *research doc* uses the canonical "Sorellon" spelling in prose.
- Keep money as integer pence, ratios as bps — the doc already does this; verify no new field violates it.

**THINK INDEPENDENTLY (after absorbing the above):**
Generate your own questions and pressure-test the design. The surface drifts are flagged for you — what remains are the *non-obvious* issues. For example: Does adding labour to `economic_landed_cost_gbp_pence` silently change the **existing** net-margin gate behaviour for contracts that happen to already carry an installation CPV but were previously passing? Is there a backfill/recompute concern for already-assessed processes? Does the Phase-2 gate's position (decision synthesis) actually have access to the goods-only landed cost separately from the labour-inclusive total (it must, to compute the ratio)? Challenge the recurring-maintenance "capital event only" basis: is the per-period margin check actually representable in the current `LandedCostEstimate`/`PipelineAssessment` schema, or does it need its own entity? What would an adversary architect attack first?

---

## 7. NON-NEGOTIABLES (do not change)
- Operator-locked decisions (the three ✅ in §1.5: low-reg category breadth, goods-value-share rule, both one-off + recurring labour scope).
- "Screening-grade triage, never a firm quote" framing.
- Skills/overlays/config/reference-data modularity mandate — **do not** propose hardcoding rules into the daemon.
- The PRE-BID vs POST-WIN regulatory split (Sorellon estimates labour cost; it does not perform regulated work).
- Money = integer pence; ratios/confidence = bps.

## 8. WHAT I DON'T WANT
- Don't rewrite the strong, verified sections (Executive Summary thesis, §4 external research, Appendix A/B/C content) — they're good.
- Don't edit any file other than the target research doc.
- Don't soften the corrections into "consider whether…" — where the code contradicts the doc, **state the corrected fact**.
- Don't add fluff. Every added sentence should carry a verified fact or a sharpened decision.
- Don't delete the doc's honest hedges where they're still warranted — replace only the ones the research has now resolved.

## 9. WHAT A GREAT OUTCOME LOOKS LIKE
- Every ⚠️ drift corrected in-place with the real `file:line` truth, and every downstream claim the drift touched reconciled (the Quick Stats table, §8.2/§8.3 file lists, ADRs, and complexity estimate all consistent with the corrected integration model).
- The existing `addressableValuePence` surface integrated into the two-phase design and the §5.5 basis caveat.
- A reader who knows the codebase finds **nothing** in the doc that contradicts the code.
- The riskiest change (service-gate split) is honestly framed as zero-coverage-today and tied to the real `DescriptionQualityVerdict` union.
- Effort/complexity re-estimated against the corrected reality (more `main.rs` work, reference-data convention to establish, schema-epoch coordination across 4 constants).

---

## 10. FOLLOW-UP FRESH-EYES PASS (run in the SAME session, after you finish editing)

Re-read the **entire** improved doc top-to-bottom as if you'd never seen it. Verify:
1. **Coherence:** Do the corrected §3.4/§3.6/ADR-002 agree with §8.2, §8.3, Quick Stats, and §6.1 complexity? No section still says "register in registry.rs" or "chained by next_job_kind."
2. **No orphaned claims:** Search the doc for `data/reference/`, `checkpoint.schema.json`, `evaluation_runs`, `next_job_kind`, `stage11/`, `freightos` (sole-named), `headless_contract` — every surviving occurrence must now be *correct* in context.
3. **No bolted-on feel:** the `addressableValuePence` integration reads as part of the design, not an appendix.
4. **Nothing weakened:** the verified-correct sections are intact; the operator-locked decisions are untouched.
5. **Terminology:** "Sorellon" in prose; pence/bps conventions held.
Fix anything that fails, then write a 6–10 line "Improvement Pass — change log" at the very top of the doc (under the status line) summarising what you corrected and why, so Pass 2 (audit/test) knows what moved.
