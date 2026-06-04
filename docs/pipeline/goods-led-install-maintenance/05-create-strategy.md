# Goods-Led Install/Maintenance — Create Strategy

## First Action

Invoke the `/spec-to-strategy` skill.

---

## Your Mission

Produce the **implementation strategy (SOT #3)** — a lightweight routing table that defines phase ordering, dependency graph, domain batches, and gate criteria. It must NOT duplicate spec content. It answers ONE question: **in what order do we build this, and what gates each phase?**

**Phases are EXECUTION ORDER, not importance tiers. Every phase ships. Later phases get the SAME depth as early phases.**

> **{ENRICHMENT — after Batch 2}** If the enrichment pass has run, the actual TDD test IDs each phase gate references are pinned here: `<TS-QUAL-* / TS-COV-* / installation_labour_engine.rs manifest / canary-case IDs>`. If you still see this raw marker, open the TDD at `.../goods-led-install-maintenance-tdd.md` and pull the real test IDs into your gate criteria — gates must name concrete tests, not "tests pass."

---

## Source Documents (Read In This Order)

```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md   — SOT #1 — READ FULLY
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md     — SOT #2 — READ (gate criteria come from here)
docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md                   — §11 recommended sequence; §9 dependencies — REFERENCE
```

### Codebase Context (the natural dependency spine)
```
crates/daemon/src/main.rs:5361-5432,8086,8324,9136-9290   — the write struct + rollup + live loop must exist before the gate can read labour
packages/contracts-schema/src/index.ts                     — types must land before TS consumers + Rust schema mirror
packages/intelligence/src/qualification/goods-qualification.ts   — Phase-1 branch depends on no new infra; Phase-2 gate depends on the rollup
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md   — prior strategy (phase structure, gate criteria, dependency spine)
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**Phases are execution ORDER, not importance tiers.** Every spec feature appears in exactly one phase. Two tiers apply (operator-decided 2026-06-02 — see `manifest.yaml` SCOPE TIERS):
- **TIER A** (MVP + §2.4 "Phase 2 / Should Have": recurring maintenance, access-equipment auto-selection, rate-refresh skill, overlay): assign to phases; later Tier-A phases get the SAME depth + full gate criteria as early ones. Do NOT drop or push to a non-existent later release.
- **TIER B** (the three §2.4 "Future" / §10 "requiring further research" items): assign each to a **tracked late phase OR a research-spike bead** whose gate is a research/design output (extension-point design, decision on the §10 question), NOT a code test. They are in scope (named, tracked) but are NOT expected to ship full behaviour in this pipeline — that is their operator-decided treatment, not scope drift.

You may NEVER:
- Create a phase plan where some spec features have no phase.
- Label a phase "optional" or "if time permits."
- Give later phases thinner coverage than early ones.

---

## Key Context From Codebase Research — the natural dependency spine

The feature has a clear build order driven by data dependencies (not importance):

1. **Schema + types foundation** — migration (labour/permit fields on `landed_cost_estimates`, `goods_value_share_bps` + coverage on `pipeline_assessments`, `InstallationRequirement` on `product_candidates`), additive/nullable/default-0; Supabase projection migration; schema-epoch bump 9→10 across 4 constants; TS contract types in `contracts-schema`. **Everything else reads these.**
2. **Reference data + labour skill** — the 4 skill-local reference JSON files, then the `uk-installation-labour-estimate` skill (`http_public`) with the Installation Profile Resolver (A2) + Permits/TM resolver (A3) + 3-point estimate. Skill-local data must exist before the skill runs.
3. **Rollup integration** — `load_skill` + sequential invocation in `run_landed_cost_live_loop` after FX; labour `.saturating_add()` at BOTH rollup sites; persist goods-only subtotal; new job kind `landed_cost_uk_installation_labour`.
4. **Qualification changes** — Phase-1 provisional-admit branch + separable-mixed downgrade; service-gate `goods_led_mixed` split; no-list/bespoke flag; goods-value-share calculator (cost proxy) in `coverage-rollup.ts`; Phase-2 dominance gate at decision synthesis. (Depends on rollup producing labour.)
5. **Recurring maintenance** — `MaintenancePeriodEstimate` entity + per-period annualised margin check (capital-event-vs-opex split).
6. **Surfacing** — manifest viewer + contract card labour/permit breakdown rows.
7. **Validation overlay + gold-set + rate-refresh** — `overlay-labour-cost` (`headless_contract`); Appendix-A gold-set corpus + L0 doc update; quarterly rate-refresh enrichment skill.
8. **Tier-B research spikes** — live per-estimate enrichment lane; installer-quote ingestion (`quote_class = installer_quote`); detailed per-authority/per-zone permit sub-model. Each gets a tracked late phase OR a single research-spike bead (gate = research/design output, not a code test).

This is a guide — let the spec's requirement structure drive the final phase boundaries. The key invariant: **types/schema before consumers; labour skill + rollup before the Phase-2 gate; all Tier-A ships; Tier-B is tracked as research spikes.**

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)
Read the spec FULLY, the TDD (for gate criteria), and the L6 strategy as the quality bar.

### Step 1: Invoke `/spec-to-strategy`
- **Spec:** `.../goods-led-install-maintenance-spec.md`
- **TDD:** `.../goods-led-install-maintenance-tdd.md`
- **Strategy output:** `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-strategy.md`

### Step 2: Feature-Specific Guidance
- Make each phase's **gate criteria** reference actual TDD test IDs (e.g. "Phase N green when TS-QUAL-006/007/008 + the relevant `installation_labour_engine.rs` manifests pass + full AGENTS.md stage gate"). Don't invent generic gates.
- Encode the hard cross-phase dependency: the **Phase-2 dominance gate cannot ship before the rollup produces labour cost** (ADR-001 — the two halves ship together). The filter must not go live ahead of the engine.
- Encode the **schema-epoch bump** as a foundation-phase task that touches all 4 constants + Supabase — it is a known footgun if split across phases.
- Encode the **migration nullability constraint** (nullable/default-0) as a foundation-phase gate so legacy direct-INSERT tests never break.
- Note the **L0 expansion rule**: a new goods category enters production only after its own gold-set validation — so the gold-set phase gates category go-live.

### Step 3: Verify Output
- [ ] Strategy exists at the path above.
- [ ] Every spec requirement is assigned to exactly one phase.
- [ ] No phase is labelled optional; later phases have full gate criteria.
- [ ] Gate criteria reference real TDD test IDs.
- [ ] The "two halves ship together" dependency is explicit.
- [ ] Schema-epoch bump + migration nullability are foundation-phase gates.
- [ ] Expected bead count per phase is estimated.

### Step 4: Follow-Up Pass (Sub-Optimization)
> Send as a follow-up message in the same session.

ULTRATHINK. Re-read your phase plan against these three files:
1. Open `packages/contracts-schema/src/index.ts` (LandedCostEstimateRow:166, PipelineAssessmentRow:216). Does any phase depend on a type/field that a later phase creates? If the Phase-2 gate reads `installation_labour_cost_gbp_pence` but that field lands in a later phase, the order is wrong — fix it.
2. Open `crates/daemon/src/main.rs:9131-9290` (`run_landed_cost_live_loop`, the skill load+invoke site) and `:5502` (`build_decision_grade_landed_cost_candidate`, where the Phase-2 gate sits). Is the labour skill (+ its 4 reference JSONs) in an EARLIER phase than the rollup-integration phase that calls it, and is that earlier than the Phase-2-gate phase?
3. Count beads-per-phase. Do later phases (recurring maintenance, overlay, gold-set, Future-tier) have a realistic, NON-trivial bead count — or did they get thin treatment? Thin late phases are a scope-drift signal. Equalize.
4. Is recurring maintenance (`MaintenancePeriodEstimate`) given a real phase with real gates, or was it hand-waved? It's a genuinely novel modelling piece — it deserves full depth.
5. Are the three Tier-B items (live enrichment lane, installer-quote ingestion, detailed per-zone permit sub-model) each assigned to a tracked late phase or a research-spike bead with a research/design gate (NOT a code-test gate)? They must be tracked (not dropped) but not over-scoped into full delivery phases.

### Step 5: Scope-Preservation Check (MANDATORY — BLOCKING)
1. Enumerate every spec requirement (both tiers).
2. Verify each TIER-A requirement is assigned to a delivery phase; each TIER-B item is assigned to a tracked late phase or a research-spike bead.
3. Scan the strategy for "post-MVP," "Phase 2+," "stretch goal," "optional phase," "if time permits," "future release," "deferred" attached to **TIER-A** content. (A Tier-B research-spike phase legitimately carries "research / later" framing — that is correct.)
4. Verify TIER-A phase coverage equality — later Tier-A phases are NOT thinner than early ones.
5. If any feature (either tier) has no phase/spike → BLOCKING failure. Assign it.

---

## Key Reminders

- Strategy = ordering + gates ONLY. Do not duplicate spec content.
- The two halves (filter + engine) ship together — the filter cannot be trusted without the engine.
- Types/schema first; labour skill + rollup before the Phase-2 gate; all Tier-A ships.
- **Phases are execution order, not importance tiers. Every Tier-A phase ships with full gate criteria; Tier-B is tracked as research spikes. Nothing is silently dropped. ZERO SCOPE DRIFT.**
