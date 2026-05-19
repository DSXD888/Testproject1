# L5 Expanded Commercial Intelligence — Spec Optimize (Fresh-Eyes Audit)

## Your Mission

A different agent has just completed the validate-spec pass on the L5 layer doc. You are performing an independent, fresh-eyes audit. You did NOT do the validation work. You are reading, analyzing, and improving — not writing code.

The L5 spec covers expanded commercial intelligence: full customs classification (multi-source recall, 10-digit resolution, measure interaction, exporter resolution, classification freshness), dual FX engine, real freight estimates, multi-source marketplace search, evidence-tiered accuracy, and CBAM forward placeholder. It is the gateway between L4's shared-visibility platform and L6's request pipeline automation.

**This is a comprehensive read-and-improve pass.** Bring strong skepticism.

---

## What To Read (Do This First)

### Document to Audit (TARGET)
```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md   — validated by Step 01
```
Read fully.

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layer-strategy.md                                    — §10 Interface Contracts (L5↔L6 gate)
docs/strategy/cross-reference-index.md                             — L5 cross-references
docs/strategy/layers/layer-0-gold-set-evaluation.md                — gold-set + canary infrastructure
docs/research/2026-03-08-public-procurement-intelligence-platform-research.md  — §10 Further Research items #2/#3/#8/#9/#10 (the Open Questions back-references)
```

### Codebase (GROUND TRUTH)
```
migrations/0001_sqlite_state_initial.sql:243-296   — supplier_quotes table
migrations/0001_sqlite_state_initial.sql:322-394   — landed_cost_estimates table (36 nullable decision-grade columns)
crates/daemon/src/errors/manifest.rs:20-55       — 44-variant ErrorCode enum
crates/daemon/src/sources/registry.rs:570-680      — supplier-enrichment / procurement-ingestion source rows
crates/daemon/src/scheduler/cron.rs:16-34          — cron pattern
skills/supplier-price-enrichment/alibaba/SKILL.md  — most developed marketplace skill (241 lines)
skills/supplier-price-enrichment/{ebay,amazon,1688}/SKILL.md
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l4-shared-visibility/archived/2026-05-08-l4-shared-visibility/02-optimize-spec.md
docs/strategy/layers/layer-4-shared-visibility.md   — for spec rigor reference
```

---

## Think Independently — 10 Audit Dimensions

### 1. Requirement ID Density

The L5 spec is 520 lines and covers 30 subsections (16 in §1, 4 in §2, 10 in §3). After Step 01 validation, every requirement should have an `L5-FR-X.Y.Z` ID. Count IDs per subsection. Subsections with fewer than 2 IDs are either trivially small (acceptable for §1.1 Layer Narrative) or under-decomposed (a problem). §1.4 Multi-Source HS Classification Recall, §1.7 Measure Interaction Engine, §1.8 Compound/Specific Duty Expressions, §1.11 Classification Freshness System, §1.12 Conservative Defaults, §2.1 Dual FX Engine, §2.2 Full Freight Engine, §3.5 Customs Error Types — each should have ≥4 IDs.

### 2. Acceptance Criteria Testability

Walk every `L5-FR-X.Y.Z`. Imagine writing a test for it. If you can't articulate a concrete test (specific input, specific expected output or assertion) in one sentence, the AC is too vague. Examples to scrutinize:

- "Multi-source recall produces candidates from ≥2 independent sources" — what's the test? Is "independent" defined elsewhere?
- "AD/CVD/safeguard/quota interaction is correctly modeled" — golden cases listed in §3.3 should be the test inputs; verify §3.3 actually contains the golden cases for steel and aluminium (it does, partially).
- "Conservative defaults applied when resolution incomplete" — six rows in §1.12 specify the defaults. Each needs a test scenario.

### 3. Cross-Layer Contract Integrity

L5 sits between L4 and L6. Verify:

- **L4 → L5:** All decision-grade columns on `landed_cost_estimates` are populated by L5 logic. Sync is automatic via L4's outbox. Verify §2.4 lists ALL columns matching `migrations/0001_sqlite_state_initial.sql:322-394`. Missing columns mean L5 silently de-scoped a populating requirement.
- **L5 → L6:** The L5→L6 readiness gate (`layer-strategy.md` §10 Interface Contracts; `cross-reference-index.md` row 229) lists 5 blocker conditions: `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`, classification freshness expired. The spec's §3.8 Forward-Compatibility says these are the L6 gate. Verify: every gate condition is producible by an L5 requirement (i.e., a code path that emits the error code or sets the field). If a gate condition has no producer, L5 leaves L6 unable to gate on it — a critical inter-layer break.

### 4. Open Questions Discipline

§3.6 has 5 Open Questions all back-referenced to research §10 Further Research. Walk each:

- **#2 product-family-specific logistics assumptions** — does §2.2 Freight Engine state how the engine BEHAVES while the question is open? (defaults? abstention?)
- **#3 Chinese supplier source strategy** — does §2.3 Alibaba/1688 state behavior under both possible decisions?
- **#8 customs cost spread materiality threshold** — does §1.13 state the placeholder threshold the engine uses pending calibration?
- **#9 packaging materials sub-category** — does §1.12 / §1.13 say what happens to packaging today?
- **#10 CBAM rate timeline** — §1.14 already covers the placeholder approach.

A live system can't wait for an open question to close. Each must specify present-tense behavior.

### 5. Conservative Defaults Coverage

§1.12 has 6 rows of constitutional conservative defaults. The unlockable_margin/evidence_leverage feature is non-trivial — it's a way to surface false-negative impact without relaxing defaults. Does the spec say HOW the engine computes `margin_delta_bps` for each evidence type? If not, that's an under-specified requirement. Add a line: "computation rule deferred to step 03 TDD design — must produce decision-grade outputs".

### 6. Evidence-Tiered Accuracy Coherence

§3.4 introduces gold/silver/bronze evidence tiers and 8 primary metrics. Verify:

- Each metric has a definition (most do — duty-outcome accuracy, trade-remedy yes/no, etc.). Any that don't, surface.
- Abstention as a positive class is explicit. Confirm the metric definition treats correct-abstention as success, not failure.
- The §3.4 framework anchors the gold-set work for ≥3 categories required by DoD criterion 12. Verify §3.1 DoD #12 cross-references §3.4 (or §1.16 customs/accuracy.ts) so the relationship is explicit.

### 7. Skill Promotion Flow Anchoring

The spec mentions "production skill directory standard from L0 S4" in §2.3. Does §2.3 enumerate the skill directory contents (SKILL.md, result.schema.json, checkpoint.schema.json, canary.inputs.jsonl, golden.assertions.yaml, validator-rubric.yaml)? If not, add a one-line reference: "All four skills follow the L0 S4 directory standard: SKILL.md + JSON schemas + canary fixtures + golden assertions + validator rubric." Without this anchor, downstream beads may produce inconsistent skill structures.

### 8. CBAM Placeholder Boundary Conditions

Today is 2026-05-08. CBAM trial sector rate Q4 2026 (~6 months out). CBAM full from 2027-01-01. Tender delivery windows that cross 2027-01-01 are within reach of any tender with a 6+ month delivery period. Verify §1.14:

- Specifies the future-regulatory-exposure flag emission rule (when to set, when to clear).
- Explicitly bounds the scope to aluminium / cement / fertiliser / hydrogen / iron and steel for 2027.
- States that `cbamEstimateGbp` remains null until secondary legislation finalises rates — placeholder logic is tracked separately from rate-bearing logic.

### 9. Test Scenarios vs DoD Coverage

§3.3 has 12 test scenarios. §3.1 has 12 DoD criteria. They should not be 1:1 — DoD is end-state; scenarios are concrete tests. But every DoD criterion should be reachable from at least one test scenario. Build a rough mental matrix:

- DoD #1 Alibaba/1688 operational ↔ §3.3 row "Alibaba search with Chinese retry" ✓
- DoD #2 Amazon operational ↔ ? (no obvious row — surface for TDD step)
- DoD #3 PCS operational ↔ Scotland is IN-SCOPE (operator pre-resolved 2026-05-08 — unconditional). A test scenario IS needed; verify §3.3 includes one for PCS OCDS Web API ingestion. If the spec leaves PCS scope conditional, that's a transcription failure of Decision 2.
- DoD #4 HMRC FPO integration ↔ §3.3 multi-family expansion
- DoD #5 ≥2 sources of recall ↔ same
- DoD #6 10-digit resolution ↔ same
- DoD #7 measure scanner ↔ §3.3 trade remedy / compound duty / supplementary unit / additional code rows
- DoD #8 HMRC FX ↔ §3.3 FX rollover row
- DoD #9 Xe ↔ ? (FX rollover row covers HMRC; commercial spot rate has no scenario — surface for TDD)
- DoD #10 Freightos + SeaRates ↔ ? (no scenario — surface for TDD)
- DoD #11 decision-grade landed cost per category ↔ §3.3 conservative-to-actual uplift
- DoD #12 ≥3 categories with gold-set ↔ implicit; should be an explicit scenario

Surface any DoD criterion lacking a scenario row. The TDD step will need them.

### 10. Risk Coverage Honesty

§3.7 lists 4 risks. For an undertaking of L5's complexity (5 external APIs, 10 new error codes, 19 new files, 4 marketplace skills maturing), 4 risks is conservative. Consider whether these are also risks worth surfacing:

- **HMRC FPO endpoint name unverified** (§1.4 callout) — Likelihood Medium, Impact Medium (additional discovery work).
- **Tariff cache freshness gap** — daemon downtime longer than `measuresFreshUntil` window leaves all classifications stale; a multi-day outage could mass-invalidate the working set.
- **Cross-language matching corruption** — Chinese keyword retry on Alibaba returns plausible-looking but wrong matches if the Chinese term is ambiguous; without a back-translation gate, false positives accumulate.
- **Validator overlay drift between worker and validator versions** — §3.5 references "validator overlay version" mechanics; misalignment between worker SKILL.md and validator overlay results in calibration regression.
- **CBAM rate publication delay** — if HMRC publishes 2027 rates late, the placeholder logic must hold for longer than designed.

Don't artificially inflate risk count, but if a real risk is missing, add it.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

This is the single most important part of the audit. The L5 layer doc has been deeply iterated on through three planmaxer cycles and a harmonymaxer integration. Your job is to verify the validated spec preserves 100% of it.

### Red-Flag Language Scan

Run:
```bash
rg -in 'post.?mvp|phase 2\+|stretch goal|nice to have|deferred to|out of scope|future iteration|simplified for initial|can be added later' docs/strategy/layers/layer-5-expanded-commercial-intelligence.md
```

Allowed matches:
- Inside §3.6 Open Questions with research back-references (research §10 Further Research #2/#3/#8/#9/#10).
- Inside §1.14 CBAM Forward Placeholder where the deferral is bounded by regulation timing.

Any match outside these contexts is a BLOCKING failure.

### Coverage Verification

Enumerate every requirement ID from §1.1 through §3.10. For each:
- Does it have an AC (bullet list, table row, or scenario in §3.3 mapped via ID)?
- Is it traceable to a populating action on a `landed_cost_estimates` column, an `ErrorCode` variant, a skill directory, a freshness vector, or a test scenario?

If any requirement has no AC and no traceable populating action — BLOCKING failure. Either extract the AC or rewrite the requirement to be testable.

### Drift Checklist

- [ ] Every requirement from the L5 layer doc original (520 lines, pre-validation) is represented in the validated version.
- [ ] No content was silently dropped during validation (compare to git diff if needed).
- [ ] All 9 cross-domain spec callouts have recorded resolutions or deliberate non-resolutions (the 7 from `manifest.yaml#spec_callouts` + 2 added 2026-05-08 for existing landed-cost skills + operator-built prerequisites; see step 01 §2 for the canonical 1-9 list).
- [ ] No `post-MVP` / `Phase 2+` / `stretch goal` / `nice to have` / `deferred` outside Open Questions and CBAM placeholder.
- [ ] §1.16 Intelligence Package Additions still totals 19 files (6 customs + 4 fx + 5 logistics + 3 validation + 1 accuracy.ts).
- [ ] §3.2 Test Files Planned still totals 5 (hmrc-fpo, customs-classification, measure-interaction, fx-engine, freight-engine).
- [ ] §3.5 Customs Error Types still totals 10.
- [ ] DoD criteria still total 12.
- [ ] Open Questions still total 5 with research back-references intact.
- [ ] Marketplace skills section reflects "mature from candidate" framing per Callout 2 — does NOT silently drop the existing alibaba/amazon/1688 directories' work.
- [ ] PCS skill row remains in §1.15 with explicit "IN-SCOPE 2026-05-08" annotation (per operator decision recorded in step 01 §6).
- [ ] L5→L6 readiness gate (§3.8) lists all 5 blocker conditions verbatim.

### Required Decisions Recorded (BLOCKING for step 03)

All four operator-required decisions were **pre-resolved 2026-05-08** via pipemaxxer AskUserQuestion (manifest `recorded_decisions`). Step 01 §6 is a TRANSCRIPTION pass — not a decision pass. Verify each is present in the spec verbatim (front-matter or `## L5 Pipeline Decisions Recorded 2026-05-08` section):

- [ ] **Decision 1** — `product_commodity_codes` ownership: pre-resolved **Option A** (L5 SQLite migration `migrations/0013_l5_product_commodity_codes.sql`). If the spec does not say "Option A" with the migration path, the validator-agent failed to transcribe — escalate to step 01.
- [ ] **Decision 2** — PCS Scotland scope: pre-resolved **IN-SCOPE** (manifest `recorded_decisions.decision_2_pcs_scotland_scope`). Verify the validator-agent transcribed this verbatim and the §3.6 Q3 historical back-reference is preserved.
- [ ] **Decision 3** — L5 implementation epic structure: pre-resolved **Option (b) sister epic**. The new epic ID is captured DURING step 07 (when `br create --type=epic ...` runs) — so the spec entry should currently read "New epic ID: <to be filled in step 07 after `br create` runs>" until step 07 fills it in. If the spec says (a) or (c), the validator-agent overrode the operator decision — escalate.
- [ ] **Decision 4** — Gold-set categories: pre-resolved **fasteners + packaging-cartons + examination-gloves**. Verify all three names appear in the transcribed decisions section.

If any decision is missing or contradicts the manifest, this audit FAILS and step 01 must be re-opened. The validator-agent's role is transcription, not re-deciding. Step 03 cannot proceed without all four decisions present and correct.

---

## Follow-Up Pass (Run After Fixes)

> After making your audit fixes, send this as a follow-up message in the same session.

Re-read your fixes specifically:

1. Open `crates/daemon/src/errors/manifest.rs` lines 20-55 (the actual ErrorCode enum location — NOT `validation/policy.rs`, which holds `ValidationPolicyMode`). List the existing ~44 variants. Cross-check against your spec's §3.5 — confirm none of the 10 new variants are duplicates of existing names (e.g., `FpoAccessUnavailable` (existing) vs `FPO_UNAVAILABLE` (new) — same? distinct? rename one?). Resolve any collision in the spec, not by deleting one of the two.

2. Open `migrations/0001_sqlite_state_initial.sql` lines 322-394. For every column on `landed_cost_estimates`, verify your spec's §2.4 row exists AND links to a populating requirement ID in §1 or §2. If §2.4 is missing a column, add the row. If a column has no populator, that's a missed requirement — add a requirement.

3. Open `skills/supplier-price-enrichment/alibaba/SKILL.md` (the 241-line one). Compare the Chinese retry section, anti-bot logic, and result schema to your spec's §2.3 Alibaba row. Do they agree? If SKILL.md describes behavior the spec doesn't, bring it into the spec — the running skill is closer to ground truth than a months-old layer doc.

4. Re-run the scope-drift grep one more time against the post-fix spec. Re-count requirement IDs per subsection. Confirm DoD criteria still total 12 (or 13 if you added the no-CI criterion per Callout 9).

5. Did your fixes accidentally introduce vagueness? Re-read any AC you tightened — if it now says "as appropriate" or "where relevant", you weakened it. Tighten to a measurable threshold or remove.

6. Open `docs/strategy/cross-reference-index.md` and grep for `L5`. Every reference must resolve to a stable `L5-FR-X.Y.Z` after your fixes. Any unresolved reference indicates the index needs updating — do that now.

7. Final check: would a TDD author receiving this validated spec be able to produce a test catalog without a single follow-up question? If you'd ask any clarifying question yourself, that's the question they'd ask — fix it now, not later.

---

## Output

The same spec file at `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`, audited and improved. Commit per pipemaxxer convention: `l5-optimize-spec: <one-line summary of largest improvement>`.

Do NOT create a sibling file. Do NOT split sections. Edit in place.

The validated spec exits this step ready for `/spec-to-tdd` invocation in step 03.
