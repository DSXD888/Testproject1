# L5 Expanded Commercial Intelligence — Completion Audit

## Summary

| Metric | Value |
|--------|-------|
| **Date** | 2026-05-20 |
| **Auditor** | claude-opus-4-7[1m] via `bead-completion-audit` v4.0 |
| **Scope** | L5 Expanded Commercial Intelligence — 130 `L5-FR-*` requirements, 98 `L5-T-*` stable IDs |
| **Language** | Mixed (Rust daemon + TypeScript `packages/intelligence` + skill YAML/JSON) |
| **Audit Iterations** | 1 (Phase 1-2 deep read + Phase 3 build/test gate + Phase 9 walkthrough) |
| **Inline Fixes Applied** | 0 (audit-only per kickoff directive) |
| **Fix Beads Created** | 0 |
| **Fix Beads Completed** | 0 |
| **Spec Questions (Human Input)** | 0 |
| **Verdict** | **COMPLETE** |

L5 implementation is complete relative to spec, TDD, and bootstrap. Every audit dimension passes, every CI-L5 gate is green, every Definition-of-Done criterion has linked test evidence and live-demo evidence, and the L5→L6 readiness gate fixture asserts every blocker class. No audit-fix beads needed.

The spec header at `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md:9` still reads "Layer 5 is **PLANNED — not yet built**", which is stale authoring metadata predating the L5 implementation epic (`testproject1-hn05`, closed 2026-05-19) — this is a documentation hygiene observation, not a code gap.

---

## Per-Dimension Findings (13 L5-specific dimensions)

### 1. Schema Population Verification — PASS

Test `L5-T-customs_classification-036` at `packages/intelligence/src/customs/customs-classification.test.ts:848-862` reads the actual `landed_cost_estimates` DDL from `migrations/0001_sqlite_state_initial.sql`, parses out the column list, and asserts every column is accounted for in one of three buckets:

- `L5_LANDED_COST_POPULATED_COLUMNS`
- `L5_LANDED_COST_EXPLICIT_NULL_COLUMNS` (must include `cbam_estimate_gbp_pence`)
- `L5_LANDED_COST_SYNC_COLUMNS`

Test fails if any column is orphaned. Daemon companion `l5_t_customs_classification_007_036_and_measure_interaction_019_complete_landed_cost_row_uses_push_owned_outbox` (`crates/daemon/tests/customs_classification.rs:176`) verifies the row populates through the L4 push-owned outbox.

### 2. ErrorCode Emission Verification — PASS

All 10 new `ErrorCode` variants live at three required sites in `crates/daemon/src/errors/manifest.rs`:

- Enum declaration (lines 33-42): `ClassificationAmbiguous`, `TariffBranchDivergence`, `MeasureInteractionUnknown`, `ExporterUnresolved`, `SupplementaryUnitMissing`, `ValuationModeConflict`, `FpoAccessUnavailable` (reused, not duplicated), `TradeRemedyWatchStale`, `FxRateMissing`, `FreightEstimateFailed`
- `as_str()` matcher (lines 82-91): all 10 mapped to their `SCREAMING_SNAKE` strings
- `from_str` parser (lines 379-388): bidirectional string ↔ enum

Emission paths exist in `packages/intelligence/src/customs/*.ts`, `fx/*.ts`, and `logistics/*.ts` (e.g., `EXPORTER_UNRESOLVED` in `exporter-resolution.ts:148`; `FREIGHT_ESTIMATE_FAILED` in `searates-skill-adapter.ts`/`freight-engine.ts`). Test `L5-T-measure_interaction-018` (the error-vocabulary contract) asserts all 10 strings are runtime-recognized. No duplicate `FPO_UNAVAILABLE` (verified by greps and CI-L5-007).

### 3. Conservative Defaults Verification — PASS

All 6 spec §1.12 defaults are implemented with deterministic triggers, evidence-leverage records, and dedicated tests:

| Default | Implementation | Test |
|---------|----------------|------|
| Origin preference MFN | `compute-confidence-breakdown.ts` `origin_preference` | `L5-T-customs_classification-013` |
| Exporter residual rate | `exporter-resolution.ts:99-106,137-160` (unconditional `residual_default` when proof absent) | `L5-T-customs_classification-014`, `L5-T-measure_interaction-011/012` |
| Quota out-of-quota | measure-scanner conditional family + additional-code unsatisfied | `L5-T-customs_classification-015`, `L5-T-measure_interaction-003` |
| VAT standard rate | `measure-scanner.ts:24` `STANDARD_IMPORT_VAT_RATE_BPS = 2_000` + `vatRateState: "unresolved"` | `L5-T-customs_classification-016`, `L5-T-measure_interaction-015` |
| Incoterm EXW/FOB/CIF boundary | `freight-engine.ts` boundary scenarios | `L5-T-customs_classification-017`, `L5-T-freight_engine-007` |
| Supplementary unit abstention | `measure-scanner.ts` `SUPPLEMENTARY_UNIT_MISSING` emission + null duty component | `L5-T-customs_classification-018`, `L5-T-measure_interaction-008/010` |

Distributor/marketplace-seller/"factory direct" claims are explicitly *not* accepted as named-exporter proof (`exporter-resolution.ts:301-311`).

### 4. Multi-Source Recall Independence — PASS

`packages/intelligence/src/customs/multi-source-recall.ts` enumerates exactly 5 independent `SourceFamily` values plus a separate `taric_cross_check` advisory family. The constitutional rule "FPO alone is not legal evidence" is **type-encoded**:

- `RecallCandidate.legalEvidence: false` (literal type)
- `RecallCandidate.authorityRole: "retrieval_relevance_only" | "advisory_cross_check"` (no "legal" value exists)
- `HmrcEndpointDecision.fpoEndpoint.procurementActionCreated: false` (literal)
- `MultiSourceRecallResult.autoClassificationAllowed` is gated on agreement count

Tests `L5-T-hmrc_fpo-004` (5 agree), `L5-T-hmrc_fpo-005` (divergence → CLASSIFICATION_AMBIGUOUS), `L5-T-hmrc_fpo-006` (FPO-only non-auto), `L5-T-hmrc_fpo-010` (query waves count as one method per source family), and `L5-T-hmrc_fpo-012` (4 non-FPO families sufficient if HS Ping declined).

### 5. Multi-Family Expansion Verification — PASS

`expandRecallLatticeToTariffGraph` at `ten-digit-resolution.ts:454-523` does **non-greedy** per-family expansion:

- Iterates over *every* `candidateRecallAgreement` family in parallel (line 460)
- Expands ALL descendants per family via `graph.descendants(familyInput.familyCode)` (line 478)
- Shortlists `shortlistPerFamily` (default 3) per family AFTER scoring (lines 488-497)
- `selectedCandidate` chosen AFTER all families are shortlisted (line 499)
- `greedyDescentDetected: false` is a literal type (line 152) — flipping would require a type change
- `TenDigitLatticeSeparation.shortlistedBeforeCommercialScenarios: true` similarly type-encoded (line 378)
- `flat_list` operator surface mode throws `"flat ungrouped UK10 lists are not an acceptable operator surface"` (line 390)
- `rejectionReasonForTopRanked` preserved (line 369)

Test `L5-T-customs_classification-001` exercises the 3923/4819/4823 deliberately non-greedy fixture.

### 6. Measure Interaction Golden Cases — PASS

Every golden case asserts exact pence-level arithmetic:

| Scenario | Test | Assertion |
|----------|------|-----------|
| Steel AD/CVD/safeguard substitutive | `L5-T-measure_interaction-002` (line 107) | `selectedDutyPence === 250_000 && not.toBe(461_000)` — greater-of-rule, no double-charge |
| Aluminium named/residual | `L5-T-measure_interaction-012` (line 610) | `appliedTradeRemedyDutyGbpPence === 351_000` (residual on missing proof) + 3510 bps named-exporter delta |
| Compound duty preservation | `L5-T-measure_interaction-007` | `DutyExpression { kind: "compound", components: [...] }` not flattened |
| Supplementary unit missing | `L5-T-measure_interaction-008/010` | `SUPPLEMENTARY_UNIT_MISSING` emitted, duty component abstained (null), no averaging |
| Unknown interaction | `L5-T-measure_interaction-004` | `MEASURE_INTERACTION_UNKNOWN` emitted, auto-decision blocked |
| Additional-code satisfiability | `L5-T-measure_interaction-005` | Prerequisites stored separately from eligibility; unknown → rate-without-code |
| GIR 3 set | `L5-T-measure_interaction-009` | Manual flag, no forced decomposition |

### 7. Marketplace Skill Maturation — PASS

All four marketplace skills (`alibaba`, `1688`, `amazon`, `ebay`) at `promotion_stage: stable` with the full 6-file standard (`SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`). PCS Scotland (`skills/procurement-ingestion/pcs/`) exists with the same 6-file standard. Spec §1.15 / §2.3 deltas confirmed in SKILL.md bodies (Chinese retry, Ready-to-Ship, alias families, browser-first, source-health canary). Daemon `sources/registry.rs:685-820` wires pcs/alibaba/1688/amazon production bindings with `worker_behavior_version` strings.

### 8. Gold-Set Coverage — PASS

Three categories present, each with:

- 4 root files: `canary.inputs.jsonl`, `golden.assertions.yaml`, `metadata.json`, `manifest.md`
- 4 variant directories: `clean/`, `weird/`, `mutated/`, `pairwise/`

`tests/fixtures/calibration/customs-classification/fasteners/golden.assertions.yaml` declares 47 gold + 63 silver + 10 bronze labels, 120 total cases, 24 pairwise pairs, `production_gate: eligible`, full 13-family mutation taxonomy + 10 customs-specific dimensions.

Accuracy harness (`L5-T-customs_classification-027` through `032`, `035`) verifies separate gold/silver/bronze reporting, Wilson lower-bound ≥9500 bps, rule-of-three upper bound ≤300 bps, bronze-only → `ineligible_bronze_only`, fabricated evidence → `ineligible_catastrophic`, correct abstention as positive class.

### 9. CBAM Placeholder Verification — PASS

`cbam_estimate_gbp_pence` is enforced in `L5_LANDED_COST_EXPLICIT_NULL_COLUMNS` (asserted at `customs-classification.test.ts:861`). Tests:

- `L5-T-customs_classification-025` — aluminium/iron-steel/cement/fertiliser/hydrogen with delivery crossing 2027-01-01 → `cbam_exposure_flag = 1`, estimate stays null
- `L5-T-customs_classification-026` — glass/ceramics OR delivery before 2027-01-01 → flag false/null, estimate null

### 10. L5→L6 Readiness Gate — PASS

`L5-T-customs_classification-034` (`customs-classification.test.ts:1603-1683`) is the readiness gate guard. It produces and tests every blocker class:

- `CLASSIFICATION_AMBIGUOUS` → `decision_grade: broker_required`
- `TARIFF_BRANCH_DIVERGENCE` → `decision_grade: broker_required`
- Required-component `SUPPLEMENTARY_UNIT_MISSING` → `broker_required`
- Residual exporter (`EXPORTER_UNRESOLVED` with no proof) → `broker_required`
- Expired `classification_fresh_until` → `broker_required`
- Floor-only quote-role (Alibaba `optimistic_floor` or 1688 `domestic_wholesale_floor` only, no operator override) → `broker_required`
- Operator override on floor-only → unblocks

Confirmed by `dod-readiness-gate.md:88-106` (targeted fixture rerun, PASS, 1/36 filtered).

### 11. Sync Engine Hookup — PASS

`crates/daemon/src/db/sync.rs:24-37` `PUSH_OWNED_RESEED_TABLES` includes `product_commodity_codes`. Reseed loop at line 659 iterates this list. `landed_cost_estimates` was already in the L4 push set per `crates/daemon/tests/customs_classification.rs:23` comment chain.

Outbox semantics: `ProductCommodityCodeOutboxEntry` carries `commitGroupId`, `contentHash`, `rowVersion`, `stageRunId` (ten-digit-resolution.ts:187-197). Set-replacement semantics enforced via `replaceProductCommodityCodes(productCandidateId)` (line 305) — daemon test `l5_t_customs_classification_004_006_product_commodity_codes_replace_under_one_stage_run` (line 117) verifies atomic candidate-scoped replacement.

RLS: `tests/fixtures/l5/redaction/sentinels.yaml` seeds unique sentinel values per field; `L5-T-measure_interaction-020` (package test line 662 + 726) verifies viewer cannot read automation-sensitive sentinels (`exporter_rate_basis`, `exporter_identity_confidence_bps`, `confidence_breakdown.evidence_leverage`, etc.) while operator can.

### 12. Stage 11 Live Demo — PASS

`docs/pipeline/l5-expanded-commercial-intelligence/live-demo/contracts-finder-lot-evidence.md` (257 lines) replays a real Contracts Finder notice end-to-end:

- Notice ID `c06b7550-179e-40df-bc37-bbdb48503412` (Falmouth Exeter Plus — Supply of Cleaning Consumables, Hygiene Products, PPE & Associated Services)
- Real endpoint `POST https://www.contractsfinder.service.gov.uk/api/rest/2/search_notices/json` + `GET /get_published_notice/json/...`
- Real keyword search "nitrile gloves" with date window 2026-01-01 → 2026-05-20
- Operator (NavyStone) signed off with conservative-blocking outcome (no auto-pass) because the lot lacks final supplier/exporter identity, pair-count supplementary-unit evidence, and exact freight route — expected L5 conservative behavior

### 13. Definition of Done Walkthrough — PASS (12/12)

Per `docs/pipeline/l5-expanded-commercial-intelligence/dod-readiness-gate.md` (executor NavyStone, 2026-05-20T09:03:28Z):

| # | DoD Criterion | Linked Tests | Status |
|---|---------------|--------------|--------|
| 1 | Alibaba/1688 search with Chinese keyword clusters | `L5-T-freight_engine-009/010/013` | PASS |
| 2 | Amazon browser-first search | `L5-T-freight_engine-011/013` | PASS |
| 3 | PCS Scotland ingestion (IN-SCOPE per operator 2026-05-08) | `L5-T-freight_engine-014` + daemon source-registry canaries | PASS |
| 4 | HMRC FPO/search-suggestion investigation | `L5-T-hmrc_fpo-002/003/008` + `hmrc-fpo-endpoint-decision.md` | PASS |
| 5 | Multi-source recall ≥2 independent sources | `L5-T-hmrc_fpo-004/005/010` | PASS |
| 6 | UK 10-digit resolution via HMRC | `L5-T-customs_classification-001/002/004` | PASS |
| 7 | Measure scanner detects AD/CVD/safeguard/quota | `L5-T-measure_interaction-001..006` | PASS |
| 8 | HMRC customs FX | `L5-T-fx_engine-001/005/009` | PASS |
| 9 | Frankfurter commercial spot FX (replaces Xe — operator 2026-05-08) | `L5-T-fx_engine-002/003/011` | PASS |
| 10 | Freightos + SeaRates CDP skills (no paid API — operator 2026-05-08) | `L5-T-freight_engine-003/004/005` | PASS |
| 11 | Decision-grade landed cost with audit trail | `L5-T-customs_classification-009/012/036`, `L5-T-fx_engine-010`, `live-demo/contracts-finder-lot-evidence.md` | PASS |
| 12 | Gold-set expansion (fasteners/packaging-cartons/examination-gloves) | `L5-T-customs_classification-027..032` | PASS |

---

## CI-L5 Gate Status (8/8)

| Gate | Command | Result |
|------|---------|--------|
| CI-L5-001 | `cargo check --all-targets` | PASS |
| CI-L5-002 | `cargo clippy --all-targets -- -D warnings` | PASS (exhaustive matches updated for all 10 L5 codes) |
| CI-L5-003 | `cargo fmt --check` | PASS |
| CI-L5-004 | `bun run build && bun run test` | PASS (138 tests, 8 files) |
| CI-L5-005 | 19 package files + 5 test files + 3 daemon integration targets | PASS (counts verified) |
| CI-L5-006 | No paid API env vars (Xe, paid SeaRates/Freightos/Taric Support, HS Ping, Easyship/Shippo, CurrencyBeacon/OANDA, Dutify/Zonos/Avalara) | PASS |
| CI-L5-007 | No ignored/skipped/pending tests, no duplicate `FPO_UNAVAILABLE`, no deprecated `bd` in L5 docs | PASS |
| CI-L5-008 | Every `L5-FR-*` mapped to ≥1 `L5-T-*` in TDD coverage matrix | PASS (130/130, alias-map total=98 unique=98) |

---

## L5-T-* Stable ID Coverage

| Test File | Expected IDs | Unique IDs Found |
|-----------|-------------:|-----------------:|
| `hmrc-fpo.test.ts` | 12 | 12 |
| `customs-classification.test.ts` | 36 | 36 |
| `measure-interaction.test.ts` | 20 | 20 |
| `fx-engine.test.ts` | 12 | 10 (009 + 010 in daemon) |
| `freight-engine.test.ts` | 18 | 17 (015 in daemon) |
| **Package subtotal** | **98** | **96** |
| Daemon Rust tests (cite IDs in fn names) | (TDD permits) | +2 (`L5-T-fx_engine-009`, `L5-T-freight_engine-015`) |
| **Effective total** | **98** | **98** |

Both Tier-T2 IDs that the TDD allows to live in daemon Rust tests are covered:

- `l5_t_fx_engine_009_hmrc_fx_monthly_rollover_cron_dispatches` (cargo test result: ok)
- `l5_t_freight_engine_015_operator_prerequisites_gate_claims_and_stable_promotion` (cargo test result: ok)

Per TDD §6: *"The daemon integration targets below must be added or extended and must cite these same IDs in Rust test names, assertion labels, or manifest evidence."*

---

## Test Gate Snapshot (2026-05-20 audit run)

| Gate | Result |
|------|--------|
| `cargo fmt --check` | exit 0 |
| `cargo check --all-targets` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test -p daemon --bin daemon` | 36 passed, 0 failed, 0 ignored |
| `cargo test --test customs_classification --test customs_classification_accuracy --test customs_fx_freight_engine` | all passed (visible IDs: `l5_t_freight_engine_014`, `l5_t_freight_engine_011_012_014_016`, `l5_t_freight_engine_015`, `l5_t_fx_engine_009`, `l5_t_freight_engine_016`, `l5_customs_fx_freight_engine_smoke_l5_t_fx_engine_005_006_009_010_freight_engine_012_015_016`, `l5_t_freight_engine_004_005`, `l5_t_fx_engine_010`) |
| `bun run build` | exit 0 |
| `bun run test` | 8 vitest files + bun src (129) + bun render tests (9) = 138 pass, 0 fail |
| Skipped / ignored / TODO in L5 code | 0 |
| Duplicate `FPO_UNAVAILABLE` | 0 |

---

## Fix Beads Created

None — verdict is COMPLETE.

| Bead ID | Gap | Status |
|---------|-----|--------|
| — | — | — |

---

## Spec Questions (Human Input Required)

None — no SPEC_QUESTION items raised during the audit.

---

## Loop History

| Iteration | Gaps Found | Inline Fixes | Fix Beads | New Gaps After Fix | Trend |
|-----------|-----------|-------------|-----------|--------------------|-------|
| 1 | 0 | 0 | 0 | 0 | COMPLETE |

---

## Observations (Documentation-Hygiene, Not Code Gaps)

These do not require fix beads; they are notes for documentation maintainers:

1. **Spec status header is stale.** `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md:9` reads `"Layer 5 is **PLANNED — not yet built**"` (authored 2026-05-02 framing). The implementation epic `testproject1-hn05` and 53+ task beads were executed and closed between 2026-05-19 and 2026-05-20, and `dod-readiness-gate.md` documents GREEN verdict. Spec authors may want to add an implementation-complete banner when this becomes the next maintained-spec revision; this is a doc hygiene item only.

2. **Daemon-side test ID covering convention.** Two of the 98 stable `L5-T-*` IDs (`L5-T-fx_engine-009`, `L5-T-freight_engine-015`) live in daemon Rust integration test function names rather than in the package Vitest files. This is explicitly permitted by TDD §6 ("daemon integration targets … must cite these same IDs in Rust test names, assertion labels, or manifest evidence") and matches the natural tier of those IDs (T2: cron-definition harness, operator-built prerequisite gate). No remediation required; flagged here so future ID-coverage tools count IDs across both medium families.

3. **Operator-built skills have minimum prerequisite files, not full 6-file standard yet.** `skills/landed-cost/freightos-freight-estimate/` and `skills/landed-cost/taric-cross-check/` contain `SKILL.md` + `result.schema.json` + READY `TEST_REPORT.md` (with canary + failure-path evidence in `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/`). This satisfies the L5-FR-3.9.2 AC1+AC2 bead-claim gate. Per bootstrap §4.3 and §13: *"No `checkpoint.schema.json` is expected for these landed-cost prerequisite skills."* The stable-production gate (L5-FR-3.9.2 AC4: canary/golden/rubric inside the skill dir) would be earned later if/when those skills are promoted past prerequisite status — not in L5 scope.

---

## Verdict

**COMPLETE**

L5 Expanded Commercial Intelligence has been implemented to spec. Every L5-FR-* requirement has corresponding implementation code and test coverage; every L5-T-* stable ID exists in either a package Vitest function or a daemon Rust test fn-name (per TDD §6); every conservative-default trigger has a deterministic implementation; every customs error code has emission + matcher + parser sites; every DoD criterion has linked test evidence; the L5→L6 readiness gate fixture asserts every blocker class; a real-notice live-demo evidence document exists. All 8 CI-L5 gates and the full Stage Validation Test Gate run green at the time of this audit.

Proceed to step 15 (archive).
