# Goods-Led Install/Maintenance + UK Labour-Cost Engine — Create Spec

## First Action

Invoke the `/research-to-spec` skill.

---

## Your Mission

Turn the deeply-iterated research doc into a formal **specification (SOT #1)** + **architecture doc** for the Goods-Led Supply-and-Install / Maintenance expansion and the UK installation/maintenance labour-cost engine. This feature does two coupled things that **ship together**:

1. **Widen the qualification filter** so goods-LED supply-and-install / supply-and-maintain contracts surface (two-phase qualification + a relaxed description-quality service gate), while **still rejecting service-only contracts** — decided by a configurable goods value-share rule.
2. **Build a modular UK labour-cost engine** as a first-class peer to freight/customs/VAT in the landed-cost rollup (a new transient-loaded landed-cost skill + skill-local reference tables).

The research doc has already been through two researchmaxer passes (improvement + audit) and is verified against codebase reality (~40 claims checked, file:line refs throughout). It IS the source of truth. Your job is to make it spec-grade: stable requirement IDs, explicit acceptance criteria, parseable structure — **without dropping a single feature**.

---

## Source Documents (Read In This Order)

### Primary Input (CREATE THE SPEC FROM THIS)
```
docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md   — READ FULLY (1246 lines)
```
This is the contract. Pay special attention to: §2.3 Success Criteria, §2.4 Scope Definition (MVP + Phase 2 + Future — **all in scope**), §3.2–§3.8 integration points, §5.2–§5.6 component/data-model/IO design, §7 ADRs, §8 file map, §9.3 schema-epoch checklist, Appendices A/B/E/F.

### Codebase Context (STUDY FOR PATTERNS — verified file:line)
```
packages/intelligence/src/qualification/goods-qualification.ts       — determineQualificationDecision() 319-372; computeAddressableValue() 290-317; computeGoodsDensityScore() 264-288
packages/intelligence/src/customs/description-quality.ts             — gateDescriptionQuality() 100-113; DescriptionQualityVerdict union 23-36; isServiceDescription() 360
crates/daemon/src/main.rs                                            — LandedCostEstimateWrite 5361-5432; rollup 8086 (domestic) + 8324 (import); run_landed_cost_live_loop 9136-9150
packages/contracts-schema/src/index.ts                               — ProductCandidateRow:36, LandedCostEstimateRow:166, PipelineAssessmentRow:216
skills/landed-cost/simple-fx-lookup/SKILL.md                         — the http_public profile to copy for the labour skill
skills/landed-cost/freightos-freight-estimate/                       — result-envelope shape to mirror (NOT the profile — it's CDP)
docs/strategy/layers/layer-0-gold-set-evaluation.md                  — §1.6 whitelist + expansion rule
```

### Quality Bar (MATCH OR EXCEED)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md          — the L6 layer doc used as the prior spec (structure, requirement-ID style, AC density)
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md  — quality reference for downstream
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**Every feature in the research doc is accounted for in the spec — nothing is silently dropped.** (The research doc's own §2.5 exclusions stay excluded; everything else is represented, at the tier defined below.)

You may improve HOW (cleaner requirement structure, sharper ACs), resequence WHEN, and strengthen vague language — but you may not make a feature *disappear*. Two tiers govern HOW each feature is represented (operator-decided 2026-06-02 — see `manifest.yaml` SCOPE TIERS):

- **TIER A — committed, full ACs.** MVP (§2.4) + the §2.4 **"Phase 2 (Should Have)"** items: recurring-maintenance modelling, access-equipment auto-selection, the quarterly rate-refresh skill, and the validation overlay. These are fully designed in the research doc (§5 data models + components) — give each explicit, testable acceptance criteria at the SAME depth as MVP. Do NOT defer them, do NOT label them "Phase 2+"/"post-MVP"/"nice to have."
- **TIER B — bounded in-scope + research spike (NO fabricated ACs).** The three §2.4 **"Future (Could Have)"** items that §10 ALSO flags as *"Requiring further research"*: the live per-estimate enrichment lane, installer-quote ingestion (`quote_class = installer_quote`), and the **detailed per-authority/per-zone** permit & lane-rental sub-model. For EACH: NAME it in the spec, define its scope boundary + the extension point it hangs off (e.g. installer-quote ingestion extends `SupplierQuote`; the per-zone model refines Component A3), and cite the §10 open research question. The strategy will give each a tracked late phase or a research-spike bead. Do NOT invent testable ACs for them (the research doc itself says they need more research first), and do NOT silently drop them. **Note:** the *basic banded* permit/TM model + responsibility flag (Component A3) is MVP/Tier A — only the per-zone *refinement* is Tier B.

You may NEVER:
- Remove features from scope (either tier).
- Label Tier-A items "nice to have," "stretch goal," "post-MVP," or "out of scope" (except the §2.5 explicit exclusions — service-only contracts, heavily-regulated goods, firm pricing, doing the installation; those stay excluded).
- Reduce Tier-A acceptance criteria below what §2.3 + Appendices A/E/F define.

---

## Key Context From Codebase Research

- **Two-phase qualification is forced by an ordering constraint.** `determineQualificationDecision` runs at the early `qualify_lot` stage, *before* Stage 11 landed cost. So the **cost-based** goods value-share cannot be an input there (circular dependency). Phase 1 = provisional-admit on goods-presence; Phase 2 = cost-based dominance gate at decision synthesis. BUT a **contract-value** share (`computeAddressableValue` → `addressableValuePence / totalValuePence`) IS available early and is the *preferred* basis when the tender itemises goods vs labour.
- **The service gate is the highest-risk change and has ZERO test coverage today.** `description-quality.ts` has no test file. The split must ADD a `goods_led_mixed` variant to the existing `DescriptionQualityVerdict` union — not build a parallel path.
- **The labour skill is `http_public`, not CDP.** It reads skill-local reference JSON and does bounded LLM archetype inference with no browser. It mirrors the freight skill's *result envelope* but NOT its `pty_interactive`/`requires_cdp` profile. It must NOT be added to `CDP_BROWSER_SKILLS`/`SIGNED_IN_BROWSER_SKILLS`.
- **No `registry.rs` edit is required.** Landed-cost skills are loaded transiently via `load_skill(...)` in `run_landed_cost_live_loop`, NOT registered as sources and NOT chained via `next_job_kind`. New job kind `landed_cost_uk_installation_labour` slots in after FX, before synthesis.
- **The rollup edit is multi-site.** Labour base must be summed at BOTH `build_domestic_landed_cost_estimate` (main.rs:8114) and `build_import_landed_cost_estimate` (main.rs:8548). New columns MUST be nullable/default-0 or legacy direct-INSERT tests break.
- **Schema-epoch bump 9→10 touches 4 constants** (daemon sqlite.rs=9, desktop main.rs=9, daemon_bridge.rs=8, query_service.rs=8 — the last two must also be reconciled) + Supabase `schema_metadata` + a Supabase projection migration. This is a known project gotcha.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY — before doing anything)
Read in order: (1) the research doc FULLY; (2) `goods-qualification.ts` and `description-quality.ts` (the two filter-change sites); (3) `main.rs` rollup + live-loop sections; (4) `contracts-schema/src/index.ts`; (5) the L6 layer doc as the quality bar.

### Step 1: Invoke `/research-to-spec`
- **Source:** `docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md`
- **Spec output:** `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md`
- **Architecture output:** `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-architecture.md`

**Context to give the skill:**
- Transcribe the **resolved operator decisions** verbatim into the spec as fixed constraints (do NOT re-decide): category breadth = low-reg only; in-scope rule = goods value-share threshold (default 5500–6000 bps); labour scope = both one-off install/de-install AND recurring maintenance; estimates screening-grade ±20%.
- The §10 open questions have **recommended defaults** (see manifest `open_defaults_recommended`). Encode the recommendations as the spec's defaults and mark them as operator-confirmable knobs — do not leave them as open holes.

### Step 2: Feature-Specific Guidance
Structure the spec so downstream coverage verification is mechanical. Suggested requirement groups (give each a stable ID, e.g. `FR-Q.*` qualification, `FR-L.*` labour engine, `FR-R.*` rollup/schema, `FR-U.*` UI, `FR-V.*` validation/gold-set):
- **FR-Q** — Phase-1 provisional-admit branch; separable-mixed `pass`-branch downgrade; Phase-2 dominance gate; service-gate `goods_led_mixed` split; contract-value-basis preference; no-list/bespoke flag; `GOODS_PRESENCE_FLOOR` + `GOODS_DOMINANCE_FLOOR_BPS` config.
- **FR-L** — labour skill (`http_public`); Installation-Requirement Extractor (A); Installation Profile Resolver (A2, catalogue + constrained LLM fallback); Permits & TM resolver (A3); goods-value-share calculator (D); one-off install + de-install; recurring maintenance (`MaintenancePeriodEstimate`, capital-event vs annualised-opex split); access-equipment auto-selection; quarterly rate-refresh skill; overlay-labour-cost; skill-local reference tables (4 files).
- **FR-R** — explicit labour + permit fields on `LandedCostEstimate` summed at BOTH rollup sites; `goods_value_share_bps` + coverage on `PipelineAssessment`; `InstallationRequirement` on `ProductCandidate`; migration (additive, nullable/default-0) + Supabase projection migration + schema-epoch bump checklist; persist goods-only subtotal so the ratio is reconstructable.
- **FR-U** — manifest viewer labour + permit breakdown rows; contract card `labourCoverageBps` metric.
- **FR-V** — gold-set corpus (Appendix A, 11 scenarios); Appendix-E E2E manifests; Appendix-F canary corpus; L0 whitelist doc update + per-category gold-set gate.
- **FR-Z (Bounded future scope — TIER B, no full ACs)** — give the three research-gated items their OWN spec section so they have a home and aren't lost: live per-estimate enrichment lane; installer-quote ingestion (`quote_class = installer_quote`); detailed per-authority/per-zone permit & lane-rental sub-model. For EACH, write only: a one-line scope statement, the extension point it hangs off (enrichment lane → operator-triggered re-run of `run_landed_cost_live_loop`; installer-quote → `SupplierQuote` with a new `quote_class`; per-zone permit → refines Component A3's banded model), and the §10 research-question reference. Do NOT write testable acceptance criteria here — these are research spikes (operator decision 2026-06-02), and inventing ACs is the false-precision error.

Carry the **basis caveat** (§5.5: the cost share is a proxy that systematically understates goods-dominance → near-floor cases route to `review`, never auto-reject; prefer contract-value basis) into the spec as a first-class non-functional requirement, not a footnote. Carry the **§5.6 warning-code set** (all 8) and the **§5.4 skill I/O contract** as exact ACs.

### Step 3: Verify Output
- [ ] Spec exists at the path above; architecture doc exists.
- [ ] Every §2.3 success criterion maps to ≥1 requirement ID with explicit ACs.
- [ ] All four resolved operator decisions are transcribed verbatim.
- [ ] Recurring maintenance is a first-class requirement (not folded away) with the capital-event-vs-opex split spelled out.
- [ ] The §6.4.c service-gate truth table (8 rows) is captured as ACs.
- [ ] The Scenario-1 NW hard assert (210000–265000 pence) is in the spec as an acceptance criterion.
- [ ] Schema-epoch 4-constant checklist is in the spec.

### Step 4: Follow-Up Pass (Sub-Optimization)
> Send as a follow-up message in the same session after the spec is drafted.

ULTRATHINK. Re-read the spec you just wrote against the codebase:
1. Open `packages/intelligence/src/qualification/goods-qualification.ts:319-372`. Does the spec's Phase-1 branch placement match reality — *after* line 353, *before* the line-355 `mixed|unknown→review` branch, and does it account for the line-339 `goodsConfidenceBps < 6000 → review` early-return firing first (audit finding #1)? Does the spec say the separable-mixed `pass` branch (line 347/352) is downgraded to defer to Phase 2 unless the early contract-value share ≥ floor?
2. Open `packages/intelligence/src/customs/description-quality.ts:23-36`. Does the spec ADD `goods_led_mixed` to *this* union (not a parallel mechanism)? Does it require a goods head-noun **and** a supply signal to flip `installation`/`maintenance` away from `reject_service` (guard against regex over-admission)?
3. Open `crates/daemon/src/main.rs:8114` and `:8548`. Does the spec require the labour `.saturating_add()` at **both** sites, and that labour is added only when an `InstallationRequirement` is extracted (so Scenario 8 pure-goods stays at labour=0)?
4. Does the spec keep `permit_traffic_mgmt` as its OWN field (not lumped into labour) because its `_low`/`_high` spread is dominated by the `permit_responsibility` unknown and must be visible in the manifest?
5. Count: §2.4 lists MVP + Phase 2 (4 items, Tier A) + Future (3 items, Tier B). Are all 4 Phase-2 items present as **full requirements with ACs**? Are all 3 Future items present as **named, bounded, research-spike entries** (scope boundary + extension point + §10 citation, NOT fabricated ACs)? A Tier-A item missing or labelled "deferred" is a scope-drift failure; a Tier-B item silently dropped (not even named) is also a scope-drift failure. Tier-B items given invented testable ACs is the OPPOSITE error (false precision) — bound them, don't over-specify them.

### Step 5: Scope-Preservation Check (MANDATORY — BLOCKING)
1. **Enumerate** every feature in the manifest's `feature_surface` block (Tier A) + the `future_tier_b` block + every §2.4 item + every Appendix-A/E/F scenario.
2. **Verify** each Tier-A feature is represented in the spec with explicit ACs; each Tier-B feature is named + bounded + research-spike-tagged (no fabricated ACs).
3. **Scan** the spec for red-flag language on TIER-A content: "post-MVP," "Phase 2+," "stretch goal," "nice to have," "deferred," "out of scope," "future iteration," "simplified for initial implementation," "can be added later." (Exceptions: the §2.5 explicit exclusions — service-only, heavily-regulated goods, firm pricing, doing the install — are legitimate and stay; and the three Tier-B items may legitimately carry a "requires further research / later phase" note, since that is their operator-decided treatment.)
4. **If any feature (either tier) is missing entirely** → BLOCKING failure. Add it (Tier A with ACs, Tier B as a bounded entry) before proceeding.

---

## Key Reminders

- The two halves ship together — the filter is not trustworthy without the labour engine (ADR-001).
- Estimates are screening-grade triage numbers (±20%), never firm quotes — carry the VAT + "what's excluded" flags.
- Fail-soft is mandatory: a `blocked` labour result degrades confidence and routes to `review`, it never crashes the pipeline (mirror the freight skill).
- The two weakest seams (research §6.6) — the service-gate regex split (silent failure) and the cost-proxy false-reject — both mandate default-to-`review` and decision-logging. Make this a spec requirement.
- **ZERO SCOPE DRIFT: every feature in the research doc — including its "Phase 2" and "Future" sections — must appear in the spec.**
