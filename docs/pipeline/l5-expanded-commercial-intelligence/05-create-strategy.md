# L5 Expanded Commercial Intelligence — Create Strategy

## First Action

Invoke the `/spec-to-strategy` skill.

---

## Your Mission

Generate the **strategy doc** (SOT #3) — phase ordering and dependency DAG for L5 implementation. This is NOT a re-derivation of the spec or TDD; it's the execution plan: what's built first, what depends on what, how phases gate, what the merge order is.

The strategy doc is intentionally lightweight. Spec defines WHAT. TDD defines HOW TO VERIFY. Strategy defines WHAT ORDER. Do not duplicate spec/TDD content; reference them by ID.

---

## Source Documents (Read In This Order)

### Primary Inputs (CREATE FROM THESE)
```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md   — Spec (SOT #1) — READ FULLY
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md  — TDD (SOT #2) — READ FULLY
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md  — Bootstrap — REFERENCE
```

### Cross-Layer Dependencies
```
docs/strategy/layer-strategy.md                                    — §10 Interface Contracts (L4↔L5↔L6)
docs/strategy/cross-reference-index.md                             — L5 cross-references
docs/pipeline/l4-shared-visibility/artifacts/l4-completion-audit-2026-05-08.md  — L4 finished 2026-05-08; what L5 plugs into
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md   — THE bar; same shape, similar depth
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md  — 478 lines; secondary reference
```

### Codebase Context (STUDY FOR DEPENDENCY ANALYSIS)
```
crates/daemon/src/main.rs                              — landed-cost paths at lines 5639-6350+ (will be refactored)
crates/daemon/src/errors/manifest.rs:20-55           — ErrorCode enum (L5 extends with 10 variants)
crates/daemon/src/sources/registry.rs:570-680          — source registration (L5 adds entries)
crates/daemon/src/scheduler/cron.rs:16-34              — cron pattern (L5 adds 4 watchers)
packages/intelligence/src/                             — current 6-file structure (L5 adds 19 files in 4 new subdirs)
skills/supplier-price-enrichment/{ebay,alibaba,amazon,1688}/  — existing skills to mature
infra/supabase/migrations/0001_projection_tables.sql   — L4 projection (L5 mutations auto-flow)
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**Every L5-FR-X.Y.Z requirement appears in some phase. Every phase ships. Phases are execution ORDER, not importance tiers. Later phases get the SAME depth as early phases.**

The strategy may NOT:
- Defer features to "post-MVP", "Phase 7+", or any non-existent later phase
- Treat any phase as optional or "nice to have"
- Have phase 5 or phase 6 with significantly thinner content than phase 1 (a signal that late features were silently de-scoped)
- Drop the 5 §3.6 Open Questions; they remain explicit risks/decisions in their respective phases

The strategy MAY:
- Choose phase boundaries based on critical-path dependencies (e.g., HMRC tariff cache must precede multi-family expansion)
- Suggest parallelization where dependencies allow
- Group related requirements into single phases for execution efficiency

---

## Key Context From Codebase Research

1. **L4 is complete** (audit 2026-05-08, zero gaps). L5 sits on a working foundation: sync engine, projections, validators, web portal, RLS, CAS, command ledger. L5 does not need to build any of that.

2. **Schema is already in place** for the L5 surface. `landed_cost_estimates` has all 36 decision-grade columns nullable from L0. `supplier_quotes` supports new sources. `local_outbox` auto-detects mutations. The only schema work is `product_commodity_codes` local SQLite junction (per validate-spec callout #1) — a single short migration.

3. **Daemon code changes are concentrated:**
   - `crates/daemon/src/errors/manifest.rs` — extend `ErrorCode` enum (10 variants at line 20+) + `as_str()` matcher (same file) + exhaustive switch sites across daemon surfaced by `cargo clippy --all-targets -- -D warnings` (typically 5-15 sites in `validation/policy.rs`, `validation/executor.rs`, `errors/markdown.rs`, `tracing/structured.rs`, `ipc/handlers/*`).
   - `crates/daemon/src/sources/registry.rs` — add 5 RegisteredSource entries: alibaba, amazon, 1688 (maturation; entries may already exist as candidate-stage), pcs (truly new — Scotland IN-scope per operator decision 2026-05-08).
   - `crates/daemon/src/scheduler/cron.rs` — add 4 CronJobDefinition entries.
   - Possibly `crates/daemon/src/main.rs` — refactor landed-cost paths to delegate to `packages/intelligence/src/customs/`.

4. **Most of L5 is in `packages/intelligence/`** — 19 new TypeScript files across 4 new subdirectories (customs/, fx/, logistics/, validation/). This is decoupled from the daemon and can largely be built before the daemon glue is wired.

5. **L5→L6 readiness gate is constitutional** (per cross-reference-index.md row 229; layer-strategy.md §10). The 5 blocker conditions (CLASSIFICATION_AMBIGUOUS, TARIFF_BRANCH_DIVERGENCE, SUPPLEMENTARY_UNIT_MISSING, decision_grade=broker_required, classification freshness expired) MUST be producible by the time L5 ships. The strategy must order this so the gate-producing logic ships before the L6 gate is wired.

6. **Open Questions are NOT phase blockers** — they are decisions made within phases. PCS Scotland scope (§3.6 Q3) was pre-resolved 2026-05-08 as IN-SCOPE (manifest `recorded_decisions`). Customs cost spread materiality threshold (§3.6 Q8) is calibrated during gold-set work; the engine ships with a placeholder threshold (e.g., 1500 bps spread = blocker).

7. **Marketplace skill maturation is parallelizable.** Each of alibaba/amazon/1688/[pcs] is independent — they can mature in parallel as soon as the validator overlay is in place.

8. **Gold-set work is gating for production.** DoD #12 requires ≥3 categories with gold-set validation before L5 is shippable. This must be a phase, not a footnote.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)

1. The validated spec — **READ FULLY**.
2. The TDD — **READ FULLY**.
3. The bootstrap doc — **REFERENCE** (informs phase 0/1 prerequisites).
4. `docs/strategy/layer-strategy.md` §10 — **READ** (L5↔L6 readiness gate).
5. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` — **STUDY** (this is your shape and bar).

### Step 1: Invoke `/spec-to-strategy`

Inputs:
- Spec: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`
- TDD: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md`
- Bootstrap: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md`
- Output: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md`

Key context to provide:
- L5 is post-L4 (L4 audit verdict 2026-05-08 = COMPLETE). Schema exists; sync exists; projections exist; web portal exists. L5 enriches existing rows.
- L5 has 19 new TS files (`packages/intelligence/`) + ~3 daemon code edits + 1 short SQLite migration + skill maturation work + cron entries.
- Bead prefix: `l5`. Bead CLI: `br`. AgentMail enabled.
- L5→L6 readiness gate is constitutional and must produce its 5 blocker conditions before L5 closes.
- Open Questions: 5 from spec §3.6 with research back-references; PCS Scotland scope (Q3) pre-resolved 2026-05-08 as IN-SCOPE; remaining 4 stay open with present-tense behavior specified.

### Step 2: Suggested Phase Shape (heuristic — validate against critical-path analysis)

The skill proposes phases. You may override with feature-specific reasoning if the heuristic shape is wrong. Suggested L5 phase shape:

#### Phase 1 — Foundation (Schema + Daemon Glue)
- Add `migrations/NNNN_l5_product_commodity_codes.sql` (next-free `NNNN`, currently 0013 after L4).
- Extend `crates/daemon/src/errors/manifest.rs` with 10 new `ErrorCode` variants + `as_str()` updates + fix all clippy-surfaced exhaustive matches across the daemon.
- Add 4-5 `RegisteredSource` entries in `crates/daemon/src/sources/registry.rs`.
- Add 4 `CronJobDefinition` entries in `crates/daemon/src/scheduler/cron.rs` (HMRC tariff news, TRA case register, FX monthly rollover, marketplace source-health canary).
- Wire env-var loading in bootstrap (HMRC, Xe, Freightos, SeaRates, Taric Support).
- **Exit gate:** `cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon` green; 4 cron entries visible in `cron list` output (or equivalent test).

#### Phase 2 — FX Engine
- Build `packages/intelligence/src/fx/{hmrc-exchange-rates,xe-commercial-rates,fx-engine,fx-types}.ts` (4 files).
- HMRC Exchange Rate Service client with monthly rolling rate cache + rollover detection.
- Xe Currency Data API client with TTL + rate-limit handling.
- Dual-rate computation, `fx_basis_delta_bps` storage, `FX_RATE_MISSING` emission, monthly-rollover-triggered recalculation queue.
- Tests: `fx-engine.test.ts` per TDD (every test ID `L5-T-fx_engine-*`).
- **Exit gate:** All `L5-T-fx_engine-*` tests pass; FX cron entry triggers correctly in dev; basis delta visible on a test `landed_cost_estimates` row.

#### Phase 3 — Freight Engine
- Build `packages/intelligence/src/logistics/{mode-selector,freightos-client,searates-client,freight-engine,freight-types}.ts` (5 files).
- Freightos and SeaRates clients with dual-source cross-check.
- Mode selector with hard mode locks (hazmat, oversized, fragile, reefer, batteries).
- Dual-packing scenarios when packaging state unknown.
- Boundary scenarios (EXW/FOB/CIF) when Incoterm absent.
- `FREIGHT_ESTIMATE_FAILED`, `FREIGHT_PROVIDER_UNSUPPORTED`, `FREIGHT_MODE_LOW_CONFIDENCE` emission.
- Tests: `freight-engine.test.ts` per TDD.
- **Exit gate:** All `L5-T-freight_engine-*` tests pass; dual-source disagreement triggers `FREIGHT_MODE_LOW_CONFIDENCE` in test fixture.

#### Phase 4 — Customs Engine (Recall + Resolution + Measure Interaction)
- Build `packages/intelligence/src/customs/{description-quality-gate,multi-source-recall,ten-digit-resolution,measure-scanner,exporter-resolution,classification-freshness}.ts` (6 files).
- Description-quality gate with archetype-keyed missing-facts requests + bundle splitter (incl. GIR 3 set-candidate escape hatch).
- Multi-source recall: HMRC FPO + CPV-to-chapter priors + rule-based hints + tariff keyword search + ATR codebook reuse. Five sources, each with their confidence basis.
- 10-digit resolution: locally-cached HMRC tariff hierarchy graph; multi-family expansion (NEVER greedy descent); slot/measure compatibility scoring; shortlist top 3-5 non-dominated branches per family; external validation only on shortlisted candidates.
- Measure scanner: AD/CVD/safeguard/quota detection with family-specific interaction rules (additive vs substitutive vs conditional). Compound/specific duty expression preservation. Supplementary unit handling.
- Exporter resolution: `named_exporter` / `annex_exporter` / `residual_default` with confidence basis points. Default to residual.
- Classification freshness: 4 vectors (structure, measures, remedies, FX) with independent windows.
- All 7 customs error codes integrated (`CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `MEASURE_INTERACTION_UNKNOWN`, `EXPORTER_UNRESOLVED`, `SUPPLEMENTARY_UNIT_MISSING`, `VALUATION_MODE_CONFLICT`, `FPO_UNAVAILABLE`, `TRADE_REMEDY_WATCH_STALE`).
- Tests: `hmrc-fpo.test.ts`, `customs-classification.test.ts`, `measure-interaction.test.ts` per TDD.
- **Exit gate:** Every spec §1.4-§1.13 requirement has its test passing; multi-family expansion exercises ≥3 plausible HS6 families on a single product; conservative defaults active under partial resolution.

#### Phase 5 — Marketplace Skill Maturation + Customs Validator Overlay
- Mature alibaba/, amazon/, 1688/ from `promotion_stage: candidate` to `stable` per L0 S4 promotion flow.
- Add Chinese retry on Alibaba; Chinese-only short clusters on 1688; alias families on Amazon; PCS skill creation (Scotland IN-scope per operator decision 2026-05-08).
- Source-health canaries; auth-refresh skill bindings; anti-bot circuit breakers.
- Optional: add `skills/validation/overlay-customs-classification/` validator overlay if recommended by step 03 TDD.
- Tests: marketplace skill canary harness runs all four through promotion gates; customs validator overlay calibrated per L0 S4 calibration harness.
- **Exit gate:** All four marketplace skills at `stable` stage; canary harness green; if validator overlay present, calibration corpus pairwise challenges pass.

#### Phase 6 — Gold-Set Validation + Evidence-Tiered Accuracy + CBAM Placeholder
- Build `packages/intelligence/src/customs/accuracy.ts` (1 file) — evidence-tiered accuracy harness.
- Build `packages/intelligence/src/validation/{dutify,zonos,avalara}-adapter.ts` (3 files) — comparison-only (NOT canonical).
- Extend gold-set fixtures for ≥3 categories per DoD #12. Categories proposed in bootstrap.
- Calibration harness with 3 corpora (clean, weird-but-valid, mutated bad), mutation bank, pairwise challenges.
- CBAM forward placeholder: `cbam_exposure_flag` emission for tender delivery windows crossing 2027-01-01 affecting in-scope commodity families (aluminium, cement, fertiliser, hydrogen, iron and steel).
- Decision-grade landed cost integration: refactor daemon main.rs landed-cost paths to delegate to `packages/intelligence/src/customs/`. Stage 11 calibration-aware persistence updated.
- Tests: `customs-classification-accuracy.test.ts` plus all calibration corpus runs.
- **Exit gate:** All 12 DoD criteria passing on gold-set categories; CBAM placeholder tests green; comparison adapters emit warnings (not blockers) on disagreement; Stage 11 live demo against a real Contracts Finder lot produces decision-grade landed cost with full audit trail.

### Step 3: Verify Output

After `/spec-to-strategy` completes:
- [ ] `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md` exists.
- [ ] Strategy line count is 350-550 (L4 strategy was ~478 lines).
- [ ] Every phase has: name, requirement IDs covered, test IDs covered, exit gate criteria, parallelization notes.
- [ ] Every spec L5-FR-X.Y.Z appears in some phase.
- [ ] Every TDD test file is owned by some phase.
- [ ] Phases are labeled execution order. Explicit statement: "Phases are execution ORDER, not importance tiers. Every phase ships."
- [ ] Critical-path dependency DAG is shown (visual or table). The DAG must show: Phase 1 (Foundation) is root; Phase 2-3 (FX, Freight) are parallel; Phase 4 (Customs) depends on Phase 1 + 2 (FX needed for valuation); Phase 5 (Skills) depends on Phase 1; Phase 6 (Gold-set + CBAM + integration) depends on all prior phases.
- [ ] L5→L6 readiness gate addressed: which phase produces each of the 5 blocker conditions? Trace each.
- [ ] Open Questions handled: PCS Scotland decision pre-resolved 2026-05-08 (IN-SCOPE); customs cost spread threshold calibrated in Phase 6; logistics defaults set in Phase 3 with placeholder values; packaging reclassification surfaces as a Phase 4 risk; CBAM rate timing handled in Phase 6.

### Step 4: Follow-Up Pass (Sub-Optimization)

> Run after Step 3.

Re-read your strategy specifically:

1. Open `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` and your L5 strategy side-by-side. Are the section structures parallel? Did L4 use a phase table you skipped? L4 had: Pre-flight, 6 phases, Cross-cutting concerns, Risk register, Decision log. Match that shape unless you have a strong reason.

2. Open `crates/daemon/src/errors/manifest.rs:20-55` and walk the `ErrorCode` enum. Phase 1's exit gate says "10 new variants + clippy clean". Realistically, adding the variants will surface 5-15 exhaustive switch sites that need updating across the daemon (tracing/structured.rs, errors/markdown.rs, validation/policy.rs, validation/executor.rs, ipc/handlers/* at minimum). Did your strategy account for this work, or did it assume a 1-line enum edit? If the latter, it's underscoped — the bead for Phase 1 ErrorCode extension should be sized for the full propagation.

3. Open `packages/intelligence/src/` and note the 3 existing subdirs (margins, qualification, relations). L5 adds 4 new subdirs. Phase 2/3/4/6 each create files in a new subdir. The lint rule on `packages/ui` (no Tauri/daemon imports) does NOT apply to `packages/intelligence` — but check whether a similar rule exists. If `packages/intelligence` has any architectural constraints (e.g., no top-level state, pure-function preference), surface them in the strategy as cross-cutting concerns.

4. Open `migrations/` and look at the 12 existing migrations. Your Phase 1 migration name follows the pattern `NNNN_l5_*.sql`. Confirm `0013` is the next-free number (or whatever's next at the time). The L4 manifest noted the same convention. Do NOT use the `sqlite/migrations/` path that the L5 spec originally suggested for `product_commodity_codes` — that path doesn't exist in this repo; use flat `migrations/NNNN_*.sql`.

5. Walk the L5→L6 gate analysis. The 5 blocker conditions are: `CLASSIFICATION_AMBIGUOUS` (Phase 4 produces), `TARIFF_BRANCH_DIVERGENCE` (Phase 4), `SUPPLEMENTARY_UNIT_MISSING` (Phase 4), `decision_grade = broker_required` (Phase 4 / Phase 6 integration), classification freshness expired (Phase 4 / Phase 1 cron). Did you trace each? If a gate condition isn't producible until Phase 6, L6 implementation has to wait — that's a long pole. Be explicit.

6. PCS Scotland scope is RESOLVED — IN-SCOPE per operator decision 2026-05-08 (manifest `recorded_decisions`). Strategy unconditionally includes the PCS bead set in Phase 5 (skill creation + canary fixtures + daemon registry entry + bootstrap PCS account note). No "if Scotland-in/out" branches.

7. Did you size phases approximately equally? L4's strategy phases were comparable in scope (6 phases, ~10-15 beads each). If your L5 phase 4 (Customs) is 30 beads and phase 2 (FX) is 5 beads, that's an honest signal that customs is the largest L5 surface — confirm with the spec, not artificially balance.

### Step 5: Scope-Preservation Check (MANDATORY — BLOCKING)

1. **Enumerate** every requirement ID from the validated spec.
2. **Verify** each one appears in some phase.
3. **Scan** the strategy for: `post-MVP`, `Phase 7+`, `stretch goal`, `nice to have`, `deferred`, `out of scope`, `future iteration`, `simplified`. (Note: `Phase 6+` is acceptable IF Phase 6 is the last phase; but `defer to Phase 7` means a phantom phase, which is BLOCKING.)
4. **Verify** no phase is significantly thinner than others (a signal of late-phase de-scoping).
5. **Verify** the 5 §3.6 Open Questions are explicitly addressed (decision point identified) — not silently dropped.

---

## Output

The strategy doc at `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md`, ~400-550 lines.

---

## Key Reminders

- L5 is the largest non-foundational layer. Customs alone is bigger than most L1-L4 features. Honest phase sizing matters.
- Phase exit gates are the bead-readiness handoff — gates green = phase complete = next phase eligible.
- Critical path runs through Phase 1 → Phase 4 (Customs is the heaviest). Phase 2 (FX) feeds Phase 4 (valuation). Phase 3 (Freight) is independent. Phase 5 (Skills) is independent. Phase 6 (Integration + Gold-set + CBAM) closes everything.
- L5→L6 readiness gate is constitutional. The 5 producing requirements MUST be in scope and produced before L5 closes.
- ZERO SCOPE DRIFT — every requirement is in some phase.
