# Goods-Led Install/Maintenance — Optimize Spec (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the spec — SOT #1 for the Goods-Led Supply-and-Install / Maintenance expansion + UK labour-cost engine. This feature changes **what the procurement platform bids on** (high behavioural blast radius) and adds a brand-new cost dimension (UK installation labour) to the landed-cost rollup. A weak spec here propagates into bad tests, bad beads, and a filter that either starves the funnel or lets service-only contracts through.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md          — created in step 01
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-architecture.md   — companion
```

### Verify Against (SOURCE OF TRUTH)
```
docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md   — the contract; every feature here must be in the spec
```

### Codebase (GROUND TRUTH — verify the spec's claims are accurate)
```
packages/intelligence/src/qualification/goods-qualification.ts:264-372   — density score, addressable value, decision gate
packages/intelligence/src/customs/description-quality.ts:23-36,100-113,360 — verdict union + service gate
crates/daemon/src/main.rs:5361-5432,8086-8216,8324-8632,9136-9290         — write struct + both rollups + live loop
packages/contracts-schema/src/index.ts:36,166,216                         — the entities to extend
docs/strategy/layers/layer-0-gold-set-evaluation.md                       — whitelist + expansion rule
```

### Quality Bar (MATCH OR EXCEED)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md   — prior spec; requirement-ID discipline + AC density
```

---

## Think Independently

Don't just check for obvious errors. Think critically about these dimensions:

### 1. Two-phase qualification correctness
Does the spec correctly separate Phase 1 (early, goods-presence, no cost share) from Phase 2 (post-landed-cost dominance gate)? Does it handle the ordering trap (line-339 confidence-floor early-return fires before the category branches)? Does it specify that the early **contract-value** share (`computeAddressableValue`) is *preferred* and can settle dominance at Phase 1, with the Phase-2 cost proxy as fallback? Verify against `goods-qualification.ts:319-372`.

### 2. Service-gate split safety
The split is the single highest-risk change (zero coverage today, fuzzy regex, **silent** failure). Does the spec require adding `goods_led_mixed` to the *existing* `DescriptionQualityVerdict` union, require BOTH a goods head-noun AND a supply signal to flip away from `reject_service`, and pin the 8-row truth table (§6.4.c) as ACs? Cross-check the union at `description-quality.ts:23-36`.

### 3. Rollup multi-site + non-regression
Does the spec require summing labour at **both** `build_domestic_landed_cost_estimate` (8114) and `build_import_landed_cost_estimate` (8548)? Does it require new columns be nullable/default-0 (so `decision_basis_hash.rs` / `customs_fx_freight_engine.rs` direct INSERTs don't break)? Does it require labour added **only** when an `InstallationRequirement` exists (Scenario 8 stays labour=0)?

### 4. Goods-only subtotal reconstructability
Phase-2 needs the goods-only numerator separately from the labour-inclusive total. Does the spec require persisting the goods-only subtotal OR the labour/permit line items so `goods_value_share_bps` is reconstructable from a single persisted row (audit finding #5)?

### 5. Recurring maintenance modelling
Does the spec treat dominance as judged on the **capital event only**, with maintenance as a **separate annualised per-period** check via a `MaintenancePeriodEstimate` entity (not capitalised into the denominator, not metadata-overloaded onto the capital-event row)? This is research §5.5 + audit finding #6 — easy to lose.

### 6. Permits & traffic-management as a peer cost
Is `permit_traffic_mgmt` its own field (not lumped into labour), sitting on the NON-goods side of the share ratio, with `permit_responsibility` defaulting to `unknown` → conservative band in `high` + `PERMIT_RESPONSIBILITY_UNKNOWN`? Are `LANE_RENTAL_ZONE` and `TTRO_LEAD_TIME_RISK` warnings present?

### 7. Skill self-containment + lint contract
Does the spec require `interaction_profile: http_public`, `side_effect_class: none`, `## Non-goals`/`## Failure Codes`/`## Resume Strategy` sections, the run-manifest token block, `agent_cli`, and skill-local reference JSON — and explicitly forbid CDP membership (§6.6 finding #3, Appendix F.2)?

### 8. Schema-epoch + projection completeness
Does the spec carry the 9→10 bump across ALL FOUR constants (daemon sqlite.rs, desktop main.rs, daemon_bridge.rs, query_service.rs) + Supabase `schema_metadata` + the Supabase projection migration? Does it ask the spec-stage question of whether `pipeline_assessments` is also projected?

---

## Scope Drift Detection (MANDATORY — BLOCKING)

**This is the single most important part of the audit.**

### Red-Flag Language Scan
Search the ENTIRE spec for: "post-MVP," "Phase 2+," "stretch goal," "nice to have," "future iteration," "out of scope," "can be added later," "simplified for initial implementation," "deferred to." If found **attached to a TIER-A feature** (MVP or §2.4 "Phase 2 / Should Have"), BLOCKING failure — remove and replace with proper content. **Two legitimate exceptions:** (1) the §2.5 exclusions — service-only contracts, heavily-regulated goods, firm pricing, doing the install; (2) the three TIER-B items, which by operator decision (2026-06-02) carry a "requires further research / later phase" note — that note is correct for them, NOT a drift signal. Anything ELSE wearing this language is drift.

### Coverage Verification (two tiers — operator-decided 2026-06-02; see `manifest.yaml` SCOPE TIERS)
Enumerate every feature/subsystem in the research doc. Verify by tier:
- **TIER A** (MVP + §2.4 "Phase 2 / Should Have"): appears with explicit ACs at equal depth to MVP.
- **TIER B** (§2.4 "Future / Could Have" = §10 "requiring further research"): appears NAMED + BOUNDED (scope boundary + extension point + §10 citation), WITHOUT fabricated testable ACs.
Both tiers: nothing silently dropped. A Tier-A item with thin/absent ACs is drift. A Tier-B item given invented full ACs is the opposite error (false precision) — flag it for downgrade to a bounded entry.

### Drift Checklist
- [ ] Every §2.3 success criterion → ≥1 requirement with ACs
- [ ] **TIER A** Recurring maintenance modelling is present, first-class, full ACs
- [ ] **TIER A** Access-equipment auto-selection is present, full ACs
- [ ] **TIER A** Quarterly rate-refresh enrichment skill is present, full ACs
- [ ] **TIER A** Validation overlay `overlay-labour-cost` is present, full ACs
- [ ] **TIER A** Basic banded permit/TM model + responsibility flag (Component A3) present, full ACs
- [ ] **TIER B** Live per-estimate enrichment lane — named + bounded (no fabricated ACs)
- [ ] **TIER B** Installer-quote ingestion `quote_class = installer_quote` — named + bounded
- [ ] **TIER B** Detailed per-authority/per-zone permit sub-model (the refinement only) — named + bounded
- [ ] All 11 Appendix-A scenarios → spec ACs; all 11 Appendix-E manifests referenced
- [ ] All 8 §5.6 warning codes present
- [ ] No red-flag language on TIER-A content (except legitimate §2.5 exclusions; Tier-B "later/needs-research" notes are allowed)
- [ ] Later requirement groups (FR-R/U/V) have the SAME AC depth as FR-Q/FR-L

---

## Follow-Up Pass (Run After Fixes)
> After making your fixes, send this as a follow-up message in the same session.

Re-read the fixes you just made. Did they introduce contradictions? Specifically:
- If you tightened the service-gate ACs, re-check they still admit "Supply of maintenance kits" → `accept` (the good is the head noun; "maintenance" only qualifies it) AND "Supply and installation of steel shelving" → `goods_led_mixed`, while still rejecting "Installation services" and "Reactive and planned electrical maintenance across estate."
- If you adjusted the value-share math, re-verify `goods_value_share_bps = round(goods_landed × 10000 / (goods_landed + labour + permits))` and that permits sit on the non-goods side (a permit-heavy high-street job correctly pushes the share DOWN).
- Re-count the feature surface one final time: research §2.4 (MVP + Phase 2 + Future) + Appendices A/E/F. Is every item still present after your edits?
