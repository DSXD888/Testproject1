# L5 Expanded Commercial Intelligence — Strategy Optimize (Fresh-Eyes Audit)

## Your Mission

A different agent has just produced the L5 strategy doc. You are auditing fresh — phase ordering correctness, dependency DAG integrity, exit-gate testability, scope-drift across phases.

Strategy is short; mistakes are loud. A misplaced phase boundary becomes a multi-week implementation pile-up. Audit hard.

---

## What To Read (Do This First)

### Document to Audit
```
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md
```

### Verify Against
```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md   — Spec (SOT #1)
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md  — TDD (SOT #2)
docs/strategy/layer-strategy.md                                    — §10 Interface Contracts (L5↔L6)
```

### Quality Bar
```
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md   — THE bar
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md  — secondary
```

### Codebase
```
crates/daemon/src/main.rs                              — landed-cost paths at lines 5639-6350+
crates/daemon/src/errors/manifest.rs:20-55           — ErrorCode enum (Phase 1 extension)
crates/daemon/src/sources/registry.rs:570-680          — source registration (Phase 1 extension)
crates/daemon/src/scheduler/cron.rs:16-34              — cron pattern (Phase 1 extension)
packages/intelligence/src/                             — current 3-subdir structure (Phase 2-6 add 4 subdirs)
migrations/0001_sqlite_state_initial.sql:322-394       — landed_cost_estimates schema
infra/supabase/migrations/0001_projection_tables.sql   — L4 projection (L5 mutations auto-flow)
```

---

## Think Independently — 9 Audit Dimensions

### 1. Critical Path Honesty

The strategy claims a phase ordering. Verify it's the actual critical path:

- **Phase 1 → Phase 2/3** parallel? FX (Phase 2) and Freight (Phase 3) only depend on Phase 1 (env vars + ErrorCode + cron entries). They're truly independent. ✓ if strategy parallelizes them; ✗ if strategy serializes.
- **Phase 4 (Customs) depends on FX?** Only for valuation in `customs/exporter-resolution.ts` and `customs/measure-scanner.ts` (commodity-value-bearing logic). If Phase 4 starts with description-quality-gate and multi-source-recall (no FX dependency), parallelization is feasible inside Phase 4 too. Verify.
- **Phase 5 (Skills) parallel to Phase 4?** Marketplace skill maturation (alibaba/amazon/1688) is independent of customs engine work. PCS skill creation is independent. Validator overlay binding depends on the validator pattern from L0/L3 — independent of customs engine. Verify.
- **Phase 6 (Integration + Gold-set + CBAM) is the only true tail.** It depends on all prior phases. Verify the exit gate truly captures "all decision-grade landed-cost flows produce decision-grade outputs end-to-end".

### 2. ErrorCode Propagation Sizing

Phase 1 extends `ErrorCode` with 10 new variants. The enum itself + its `as_str()` matcher live in `crates/daemon/src/errors/manifest.rs` lines 20-55+ (NOT `validation/policy.rs` — that file holds `ValidationPolicyMode` / `ValidationTrigger` for L3 logic, a separate enum family). Adding 10 variants surfaces 5-15 exhaustive `match` sites across the daemon when clippy runs. Typical propagation sites:

- `crates/daemon/src/errors/manifest.rs` (the enum + `as_str()` matcher — the single source)
- `crates/daemon/src/validation/policy.rs` (validation routing that pattern-matches on ErrorCode)
- `crates/daemon/src/validation/executor.rs` (job dispatch by code)
- `crates/daemon/src/errors/markdown.rs` (specific code → human label)
- `crates/daemon/src/tracing/structured.rs` (tracing tags)
- `ipc/handlers/*` (any handler that returns codes to desktop)
- Any other place that pattern-matches on ErrorCode

Did the strategy size Phase 1's bead for the full propagation, or assume a 1-line enum edit? If the latter, the bead is underscoped. If the strategy lists `validation/policy.rs` as the enum's home, that's wrong — the enum lives in `errors/manifest.rs`.

### 3. L5→L6 Gate Producer Trace

The 5 L5→L6 readiness gate conditions:

| Condition | Producing Phase | Producing Module |
|-----------|-----------------|------------------|
| `CLASSIFICATION_AMBIGUOUS` | Phase 4 | `customs/multi-source-recall.ts` (when sources diverge) |
| `TARIFF_BRANCH_DIVERGENCE` | Phase 4 | `customs/ten-digit-resolution.ts` (when UK10 branches differ on remedy) |
| `SUPPLEMENTARY_UNIT_MISSING` | Phase 4 | `customs/measure-scanner.ts` (when supp unit required but absent) |
| `decision_grade = broker_required` | Phase 4 + 6 (set by various; integrated in Phase 6) | Multiple |
| classification freshness expired | Phase 4 (sets fields) + Phase 1 (cron schedules invalidation) | `customs/classification-freshness.ts` + cron |

Walk each. Did the strategy explicitly trace each gate condition to a producing phase? If `decision_grade = broker_required` is left implicit ("emerges from various conditions"), that's under-specified — name the conditions.

### 4. Open Question Handling

The 5 §3.6 Open Questions:

- **Q2 logistics defaults** — Phase 3 must set defaults explicitly (the engine ships with placeholder freight-mode heuristics; refinement is post-implementation calibration with research-back-reference).
- **Q3 PCS Scotland** — RESOLVED 2026-05-08 as IN-SCOPE (manifest `recorded_decisions.decision_2_pcs_scotland_scope`). Strategy includes PCS bead set unconditionally in Phase 5. Verify Phase 5 has a PCS skill creation bead.
- **Q8 customs cost spread threshold** — Phase 6 calibration sets the production threshold; Phase 4 ships with a placeholder (e.g., 1500 bps).
- **Q9 packaging reclassification** — surfaces as a Phase 4 risk (in spec §1.12 packaging is "split: easy/dangerous subfamilies"); strategy must flag this as a known calibration item.
- **Q10 CBAM rate timing** — Phase 6 ships placeholder logic only; rate-bearing logic awaits regulation (post-L5 by definition for trial Q4 2026, gating logic starts shipping near 2027-01-01).

Verify each is addressed. Vague handling ("we'll figure it out during implementation") is a fail.

### 5. Skill Maturation Granularity

Phase 5 matures 3 existing skills (alibaba, amazon, 1688) and creates ≤1 new one (pcs, gated). Each is its own bead. Verify the strategy doesn't bundle them under a single "skills phase" bead — they have independent canary fixtures, independent calibration targets, and independent promotion-flow stages. They should be 3-4 separate beads.

### 6. Gold-Set Category Naming

DoD #12 requires ≥3 categories. The operator pre-resolved the trio on 2026-05-08 (manifest `recorded_decisions.decision_4_gold_set_categories`): **fasteners + packaging-cartons + examination-gloves** (clear differentiation: remedy-active / packaging-easy / regulated medical). The strategy MUST name all three verbatim in Phase 6 (gold-set extension). If the strategy says "≥3 categories TBD" or proposes alternative candidates, that's stale wording — replace with the pre-resolved trio. No "surface for owner decision" language: the decision is closed.

### 7. CBAM Phase Placement

CBAM trial Q4 2026 is 5 months out. Phase 6 ships CBAM placeholder logic. Verify:

- Strategy treats CBAM as Phase 6, not Phase 7 (which doesn't exist).
- Placeholder logic ships, including the future-regulatory-exposure flag emission for tender windows crossing 2027-01-01.
- Rate-bearing logic is explicitly out of scope for L5 (held until secondary legislation finalises rates).

### 8. Decision-Grade Refactoring

Phase 6 includes "refactor daemon main.rs landed-cost paths to delegate to packages/intelligence/src/customs/". This is the largest single risk in L5 — it touches `crates/daemon/src/main.rs:5639-6350+` (~700 lines of existing screening-grade logic) and rewires it to call into TS packages.

Verify the strategy:

- Sizes this bead realistically (likely 3-5 days of focused work, 1 large refactor or 2-3 incremental steps).
- Has explicit feature-flag wiring (decision-grade feature flag default OFF in early Phase 6, default ON at end of Phase 6). This allows running new and old paths side-by-side during validation.
- Names a regression-test gate (Stage 11 live demo against a real Contracts Finder lot; existing landed-cost integration tests pass).

### 8A. L5 Field-Level Redaction Risk

The current L4 RLS policy in `infra/supabase/migrations/0004_rls_policies.sql` grants viewer/operator/admin SELECT on the whole `landed_cost_estimates` projection table. L5 TDD classifies new decision-grade fields into display, control, automation-sensitive, and local-only sensitive groups; for example `exporter_rate_basis`, `exporter_identity_confidence_bps`, `origin_evidence_state`, `trade_remedy_cases_json`, `duty_breakdown_json`, `measure_interaction_json`, and `confidence_breakdown.evidence_leverage` are automation-sensitive and viewer-redacted.

This is a sync/RLS boundary risk, not a TDD-only concern. Verify the strategy assigns a Phase 6 integration bead to implement or validate the field-level projection boundary, such as a redacted view, summary projection, RPC serializer, or equivalent role-scoped projection. Do not let Phase 6 close with table-level RLS alone if `L5-T-measure_interaction-020` expects viewer/operator field differences.

### 9. Phase Equality

L4 strategy phases ranged ~10-15 beads each. L5 should be similar. Walk the strategy:

- Phase 1 (Foundation): 4-6 beads expected (migration, ErrorCode, registry, cron, env-var loader).
- Phase 2 (FX): 4-6 beads (per file + integration test set).
- Phase 3 (Freight): 5-7 beads (per file + integration test set; mode selector is non-trivial).
- Phase 4 (Customs): 12-20 beads (largest phase by spec content).
- Phase 5 (Skills): 4-6 beads (one per skill maturation/creation + canary harness + validator overlay).
- Phase 6 (Integration + Gold-set + CBAM): 8-12 beads (gold-set per category + CBAM placeholder + accuracy harness + comparison adapters + main.rs refactor + Stage 11 live demo).

Total: ~40-55 beads. If your strategy implies <30 or >70 beads, sanity-check.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan

```bash
rg -in 'post.?mvp|phase 7\+|phase 8\+|stretch goal|nice to have|deferred|out of scope|future iteration|simplified|skip this phase|optional phase' \
  docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md
```

Allowed:
- CBAM rate-bearing logic explicitly held until regulation (with date reference).
- §3.6 Open Question references (with research back-references).

Any other match is a BLOCKING failure.

### Coverage Verification

For every L5-FR-X.Y.Z in the spec:
- Find the phase that owns it.
- If a requirement has no phase assignment, that's silent de-scoping. Add it.

### Phase Coverage Equality

- [ ] No phase has fewer than 3 requirements assigned to it.
- [ ] No phase exit gate is materially weaker than another's.
- [ ] Phases are labeled execution-order. Explicit text: "Phases are execution ORDER, not importance tiers. Every phase ships."
- [ ] No phase is "optional".
- [ ] No requirement is assigned to "Phase 6+" or "later".

### Drift Checklist

- [ ] All 12 DoD criteria mapped to phase exit gates.
- [ ] All 19 new TS files assigned to a phase.
- [ ] All 3-4 daemon code edits assigned to a phase.
- [ ] All 4 cron entries assigned to a phase.
- [ ] All 1 SQLite migration assigned to a phase.
- [ ] All 4 marketplace skill maturation tasks assigned to a phase.
- [ ] All 5 §3.6 Open Questions explicitly addressed.
- [ ] L5→L6 readiness gate (5 conditions) producer-traced.
- [ ] CBAM placeholder shipping in L5 explicit.

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

1. Open `crates/daemon/src/errors/manifest.rs:20-55`. Walk the existing 44 ErrorCode variants. For each of L5's 10 new variants, find the phase that adds it and the phase that has tests for it. Are they the same phase, or split? If split, the testing phase is gated on the implementing phase — verify the dependency DAG reflects this.

2. Open `crates/daemon/src/main.rs` and grep for `landed_cost`. Count the lines that touch landed-cost logic. Phase 6's refactor bead claims to migrate these to `packages/intelligence/src/customs/`. Is the bead sized for the actual line count?

3. Open `tests/fixtures/calibration/` and look at the existing variant structure. Phase 6's gold-set extension lists ≥3 categories. Confirm the strategy lists category candidates by name (not "TBD"). If the strategy says TBD, this is a deferred decision — convert it now or surface as a Phase 0 decision-required item.

4. Open `infra/supabase/migrations/0004_rls_policies.sql` and identify viewer/operator field classification. Phase 6 integration must verify L5's new field populations comply with RLS. Is this in the exit gate? If not, add it.

5. Walk the bead-count estimate. ~40-55 beads is typical for this surface size. If your strategy suggests <30, you've under-decomposed (each "bead" will become 3-day work, blocking parallelism). If >70, you've over-decomposed (overhead exceeds benefit). Adjust phase decomposition.

6. Did you parallelize where possible? Phase 2 + 3 + 5 are largely independent — they should run in parallel after Phase 1 closes. Phase 4 may overlap Phase 5 but only after the daemon glue (Phase 1) is in place. Document parallelization explicitly.

7. Did you set a clear "L5 closes" criterion? When all phase exit gates green and DoD all 12 criteria pass, L5 is done. State this explicitly. Don't leave the closure criterion ambiguous.

---

## Output

The strategy doc at `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md`, audited and improved.

---

## Key Reminders

- A bad phase boundary kills parallelism. A bad exit gate creates rework. Audit hard.
- L5→L6 gate is constitutional — every gate condition must be producible by L5's end.
- Open Questions stay open as Open Questions; they don't become silent de-scopings.
- ZERO SCOPE DRIFT.
