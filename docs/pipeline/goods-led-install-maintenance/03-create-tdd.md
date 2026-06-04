# Goods-Led Install/Maintenance — Create TDD

## First Action

Invoke the `/spec-to-tdd` skill.

---

## Your Mission

Produce the **Test-Driven Development document (SOT #2)** + **bootstrap doc** for the goods-led install/maintenance feature. The project rule is **REAL E2E, no mocks/stubs** (AGENTS.md) — every test runs against a real harness: real SQLite seeding, the real rollup/gate code, the real skill canary harness, the real `skill_lint.rs`. The research doc already authored a near-complete test strategy (§6.3, §6.4, Appendices A/E/F) — your job is to formalize it into a TDD where **every Tier-A spec requirement maps to a named test with an expected verdict**, and the three Tier-B items appear in the coverage matrix as research-spikes (no fabricated tests — see the Cardinal Rule below).

> **{ENRICHMENT — after Batch 1}** This handoff was generated before the spec existed. If the enrichment pass has run, the spec's actual requirement IDs, group names, and total count are pinned here: `<requirement-id-range>`, `<group-names>`, `<N requirements>`. If you still see this raw marker, open the spec at `.../goods-led-install-maintenance-spec.md` and read its requirement IDs directly — your coverage matrix maps to *those* IDs, not the illustrative `FR-Q.*/FR-L.*` scheme used below.

---

## Source Documents (Read In This Order)

### Primary Input (CREATE THE TDD FROM THIS)
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md   — SOT #1 — READ FULLY
```

### Source of Truth Context
```
docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md   — §6.3/§6.4 testing strategy, Appendix A (11 scenarios), Appendix E (11 E2E manifests), Appendix F (canary corpus) — READ §6 + Appendices A/E/F FULLY
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-architecture.md   — REFERENCE
```

### Codebase Test Infrastructure (STUDY THESE — they are the harnesses you mirror)
```
crates/daemon/tests/customs_fx_freight_engine.rs   — the E2E pattern to mirror: seed_landed_cost_inputs + landed_cost_insert_sql + assert_l5_product_commodity_codes_schema_applied + user_version == DEFAULT_SCHEMA_EPOCH assert
crates/daemon/tests/decision_basis_hash.rs          — seed_candidate_quote_and_landed_cost; direct landed_cost_estimates INSERT; hash-mutation pattern
crates/daemon/tests/skill_canary_harness.rs         — canary corpus harness (requires canary.inputs.jsonl + golden.assertions.yaml + validator-rubric.yaml w/ rubric_version + metadata.json w/ row_count + manifest.md mentioning Scenario)
crates/daemon/tests/skill_lint.rs                   — frontmatter + accepted interaction_profile values + per-stage required files
packages/intelligence/src/qualification/goods-qualification.test.ts   — TS-QUAL-* style; TS-QUAL-003 is the one that BREAKS
packages/intelligence/src/margins/coverage-rollup.test.ts             — TS-COV-* style
packages/intelligence/src/customs/customs-classification.test.ts      — the bun:test style to mirror for the NEW description-quality.test.ts
tests/fixtures/calibration/customs-classification/                    — the per-domain calibration corpus layout to mirror
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md   — best existing TDD (80KB; test-ID discipline, coverage matrix, CI tier mapping)
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**Every Tier-A spec requirement gets named test coverage; every Tier-B item is accounted for in the coverage matrix (as a research-spike). Nothing is silently unpinned.**

You may improve HOW tests are structured. You may NEVER drop a Tier-A requirement's coverage, silently skip a scenario, or label a Tier-A test "optional." Two tiers apply (operator-decided 2026-06-02 — see `manifest.yaml` SCOPE TIERS):
- **TIER A** (MVP + §2.4 "Phase 2 / Should Have": recurring-maintenance, access-equipment auto-selection, rate-refresh skill, `overlay-labour-cost`, basic banded permit/TM model) — **every one needs named tests**. A Tier-A requirement with no test is a BLOCKING gap.
- **TIER B** (the three §2.4 "Future" / §10 "requiring further research" items: live enrichment lane, installer-quote ingestion, detailed per-authority/per-zone permit sub-model) — the spec carries these as bounded research-spike entries, NOT fully-spec'd behaviour. Do NOT invent E2E/unit tests for them; instead note in the coverage matrix that each is a research-spike (its "test" is the research output / extension-point design, verified at the research-spike bead, not a code assertion). Do NOT silently omit them from the matrix either — list them with a "research-spike, no code test" marker.

---

## Key Context From Codebase Research

- **`goods-qualification.test.ts` TS-QUAL-003 WILL BREAK** and must be split, not patched away. It currently expects `pass` for a separable mixed contract with `goodsAddressableValuePence=360_000`/`totalValuePence=900_000` (40% contract-value share). The Phase-1 change downgrades separable-mixed `pass` to defer to Phase 2 unless the early share ≥ floor; 40% < 55–60% floor flips it `pass → review`. Split into **TS-QUAL-003a** (raise to ≥0.60×total = 540_000 → still `pass`; proves the early contract-value short-circuit) and **TS-QUAL-003b** (keep 360_000 → now `review`; proves deferral to Phase 2). Do NOT silently keep the old expectation.
- **`description-quality.test.ts` is NEW — zero coverage exists today.** This is the highest-risk gate. The TDD must define the full 8-row truth table (§6.4.c) as `bun:test` cases mirroring `customs-classification.test.ts`.
- **Legacy direct-INSERT tests must keep passing.** `customs_fx_freight_engine.rs` and `decision_basis_hash.rs` INSERT `landed_cost_estimates` rows directly with explicit column lists; the new labour/permit columns MUST be nullable/default-0 or those INSERTs fail at runtime. The rollup-sums-labour assertion belongs in the NEW `installation_labour_engine.rs` target.
- **The Scenario-1 NW band is the engine-acceptance hard assert:** `210000 ≤ installation_labour_cost_gbp_pence ≤ 265000`. If the engine misses this band, the rate table / scope model is wrong.
- **The full green bar is the AGENTS.md stage gate**, not just `cargo test -p daemon --bin daemon`: fmt + check + clippy(-D warnings) + bin test + **every** `crates/daemon/tests/*.rs` + `bun test`.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)
Read the spec FULLY; research §6 + Appendices A/E/F FULLY; then the 5 Rust test harnesses + 3 TS test files above to learn the exact seeding/assertion idioms you must mirror.

### Step 1: Invoke `/spec-to-tdd`
- **Spec:** `.../goods-led-install-maintenance-spec.md`
- **TDD output:** `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md`
- **Bootstrap output:** `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-bootstrap.md`

### Step 2: Feature-Specific Guidance — the test layers (research §6.3)
Define tests across these real harnesses, each with a command:
| Layer | What | Real harness | Command |
|-------|------|--------------|---------|
| Unit (TS) | Phase-1 branch; service-gate split; goods-value-share math; rate×multiplier | `goods-qualification.test.ts`, `coverage-rollup.test.ts`, NEW `description-quality.test.ts` | `bun test` |
| Unit (Rust) | rollup sums labour at BOTH sites; labour/permit field persistence | `#[test]` in daemon bin | `cargo test -p daemon --bin daemon` |
| Skill canary | labour skill 3-point band over canned `job_parameters` | `skill_canary_harness.rs` + NEW `tests/fixtures/calibration/landed-cost/uk-installation-labour/` | `cargo test -p daemon --test skill_canary_harness` |
| Skill lint | labour skill frontmatter + `http_public` | `skill_lint.rs` | `cargo test -p daemon --test skill_lint` |
| Gold-set | Appendix A corpus; accept/reject precision; L0 expansion gate | fixture corpus + `skill_canary_harness.rs` (NO `evaluation_*` tables — they don't exist) | `cargo test -p daemon --test installation_labour_engine` |
| E2E (deterministic) | seed → assert rollup total, `goods_value_share_bps`, decision | NEW `installation_labour_engine.rs` (mirror `customs_fx_freight_engine.rs`) | `cargo test -p daemon --test installation_labour_engine` |
| E2E (live smoke) | `run_landed_cost_live_loop` invokes the labour skill (no browser) | live landed-cost loop | nightly smoke target |
| UI (browser) | manifest + contract card render labour/permit breakdown | `/agent-browser` skill | agent-browser flow |

Then formalize:
- **6.4.a existing-test impact** — TS-QUAL-003 split into 003a/003b; legacy direct-INSERT migration nullability; `decision_basis_hash` basis decision (does labour join the hash vector?); `coverage-rollup.test.ts` extension.
- **6.4.b new tests** — TS-QUAL-006/007/008; the NEW `description-quality.test.ts`; TS-COV-007/008/009; Rust both-sites + round-trip; the NEW `installation_labour_engine.rs` (11 Appendix-E manifests).
- **6.4.c service-gate truth table** — all 8 rows as named cases.
- **6.4.d promotion gate** — 100% negative+regression controls (Scenarios 5,6,7,8); acceptance precision ≥ floor (Scenarios 1,2,3,4,9); Scenario-1 band hard-assert; skill lints; canary passes (incl. `metadata.json` row_count match + `validator-rubric.yaml` rubric_version + `manifest.md` Scenario mentions); full AGENTS.md stage gate green; missing `validator-rubric.yaml` = stable-production blocker.
- **6.4.e coverage matrix** — every §2.3 criterion → named test(s). Reproduce and complete the matrix.
- **Appendix F warning-code coverage** — all 8 §5.6 codes pinned to canary cases.

### Step 3: Verify Output
- [ ] TDD + bootstrap exist at the paths above.
- [ ] Every spec requirement ID appears in the coverage matrix with ≥1 test ID.
- [ ] TS-QUAL-003a/b split is explicit (old `pass` expectation NOT retained).
- [ ] The NEW `description-quality.test.ts` 8-row truth table is fully specified.
- [ ] All 11 Appendix-E manifests are mapped to `installation_labour_engine.rs` cases with pence-explicit asserts.
- [ ] Scenario-1 NW band hard-assert (210000–265000) is present.
- [ ] All 8 warning codes pinned to canary cases.
- [ ] The bootstrap doc lists the full AGENTS.md stage-gate command sequence + schema-epoch bump checklist.

### Step 4: Follow-Up Pass (Sub-Optimization)
> Send as a follow-up message in the same session.

ULTRATHINK. Verify your tests are runnable against the REAL harnesses:
1. Open `crates/daemon/tests/customs_fx_freight_engine.rs`. Does your `installation_labour_engine.rs` plan use the same `L4TestStateDb::new()` → `assert user_version == DEFAULT_SCHEMA_EPOCH` (=10 after bump) pattern and `landed_cost_insert_sql` direct-seed approach? Does it assert the `0014_installation_labour_fields` migration is present in `schema_migrations`? (Next free SQLite migration number is **0014** — current max is `0013_l5_product_commodity_codes.sql`; the matching Supabase projection migration is **0010** — current max is `0009_l5_field_level_projection.sql`.)
2. Open `crates/daemon/tests/skill_canary_harness.rs`. Does your canary fixture plan provide ALL required files (`canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml` with `rubric_version`, `metadata.json` with `row_count` == JSONL line count, `manifest.md` mentioning `Scenario`)? The harness asserts `row_count` equals the JSONL line count — does your `metadata.json` (row_count: 7) match the 7 canary cases in Appendix F.1?
3. Open `packages/intelligence/src/qualification/goods-qualification.test.ts`. Find the actual TS-QUAL-003 case. Does your split keep the exact `goodsAddressableValuePence`/`totalValuePence` numbers (360_000/900_000) for 003b and raise to 540_000 for 003a?
4. Does every negative control (Scenarios 5/6/7) have BOTH a `description-quality.test.ts` assertion (`reject_service`) AND an `installation_labour_engine.rs` assertion (candidate never reaches Stage 11 / share < floor)?
5. Recount: is there a test for EVERY **Tier-A** spec requirement, including recurring-maintenance (Scenario 4 + `MaintenancePeriodEstimate` distinct-entity assert), access-equipment auto-selection, the rate-refresh skill, and the overlay? Are the three **Tier-B** items listed in the matrix as "research-spike, no code test" (not silently omitted, not given invented tests)?

### Step 5: Scope-Preservation Check (MANDATORY — BLOCKING)
1. Enumerate every spec requirement (both tiers).
2. Verify each TIER-A requirement maps to ≥1 named test; each TIER-B item appears in the matrix marked "research-spike, no code test."
3. Scan the TDD for "optional test," "skip for now," "deferred coverage," "post-MVP" on TIER-A content. If found → BLOCKING; replace with real coverage. (A Tier-B "research-spike" marker is correct, not drift.)
4. If any Tier-A requirement is unpinned, or any Tier-B item is missing from the matrix entirely → BLOCKING failure. Add it.

---

## Key Reminders

- REAL E2E only — no mocks, no stubs. Every row seeded into real SQLite; real rollup/gate code exercised.
- The service gate has zero coverage today — the NEW `description-quality.test.ts` is a hard prerequisite, not a nicety.
- Scenario 8 (pure paper) is the non-regression guard: labour=0, no `InstallationRequirement`, rollup total unchanged.
- The full green bar = the AGENTS.md stage gate (fmt + check + clippy + bin + every integration target + bun test).
- **ZERO SCOPE DRIFT: every Tier-A spec requirement must have a named test with an expected verdict; every Tier-B item appears in the matrix as a research-spike (no fabricated test). Nothing is silently unpinned.**
