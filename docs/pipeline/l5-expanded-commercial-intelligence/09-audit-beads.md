# L5 Expanded Commercial Intelligence — Bead Audit (Spec Drift Detection)

## First Action

Invoke the `/bead-plan-audit` skill.

---

## Your Mission

You are performing the **last gate before execution** for L5. A dedicated, systematic audit of every L5 bead against the spec, TDD, and strategy. Your verdict is binary: ZERO drift means execution can proceed; any drift means beads must be fixed before kickoff.

Read EVERY bead. Walk EVERY spec L5-FR. Walk EVERY TDD test ID. Verify ZERO coverage gap. Scan for ZERO scope-reduction language.

---

## Source Documents

### Beads (TARGET — audit EVERY bead)
```bash
br list --json                       # Full inventory
br show <id> --json                  # Individual bead detail
br list --label phase-N --json       # Per-phase view
```
Read EVERY bead. Not a sample — every single one.

### Verify Against
```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md   — Spec (SOT #1)
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md  — TDD (SOT #2)
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md  — Strategy (SOT #3)
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-bead-alias-map.json
```

### Codebase (FOR VERIFYING BEAD REFERENCES ARE ACCURATE)
```
crates/daemon/src/errors/manifest.rs:20-55       — ErrorCode current state
crates/daemon/src/sources/registry.rs:570-680
crates/daemon/src/scheduler/cron.rs:16-34
crates/daemon/src/main.rs:5639-6350+
crates/daemon/src/db/sync.rs:24-49                 — PUSH_OWNED_RESEED_TABLES
migrations/                                         — flat NNNN_*.sql convention
packages/intelligence/src/                         — current 3 subdirs
skills/supplier-price-enrichment/                  — 4 existing skill dirs
infra/supabase/migrations/                         — projection + RLS
tests/fixtures/calibration/                        — gold-set + canary corpus
```

### Quality Bar
```
docs/pipeline/l4-shared-visibility/archived/2026-05-08-l4-shared-visibility/09-audit-beads.md
docs/pipeline/l4-shared-visibility/artifacts/l4-completion-audit-2026-05-08.md  — what L4's COMPLETE verdict looked like
```

---

## Cardinal Rule: ZERO SCOPE DRIFT — BLOCKING DIMENSION

**This is the single most important audit dimension.** If you detect ANY drift between the spec and the beads, this audit FAILS and beads must be fixed before kickoff. There is no acceptable level of drift.

The L5 spec went through three planmaxer cycles + harmonymaxer Run 002 — its content is high-quality. Everything in it must have a bead.

---

## Audit Methodology

`/bead-plan-audit` provides the framework. Layer these L5-specific checks on top.

### 1. Coverage Matrix Verification

Build a 2D matrix:

```
Rows: every L5-FR-X.Y.Z requirement (~60-100 IDs after step 02 enrichment)
Cols: every L5 bead (~40-55 beads)
Cell: 1 if the bead implements the requirement, 0 otherwise
```

For every requirement row:
- Sum across cols. Sum must be ≥ 1. Sum = 0 → BLOCKING failure (uncovered requirement).

For every bead col:
- Sum across rows. Sum should be ≥ 1 (every bead implements something). Sum = 0 → either junk bead or the alias map is broken.

### 2. Test Coverage Verification

Build a similar matrix for L5-T test IDs:

```
Rows: every L5-T-{file-stem}-{NNN} test ID
Cols: every L5 bead
Cell: 1 if the bead owns the test, 0 otherwise
```

For every test row:
- Sum must be exactly 1. Sum = 0 → orphaned test (BLOCKING). Sum > 1 → duplicate ownership (BLOCKING).

### 3. Dependency DAG Audit

Run `br ready --json` and `br list --json | jq -r '.[] | "\(.id) \(.blocked_by)"'`. Build the DAG mentally:

- **Phase 1 root:** beads have no `blocked_by` (or only the L5 epic).
- **Phase 2/3/5 second tier:** block on Phase 1 closure.
- **Phase 4 third tier:** block on Phase 1 + Phase 2 closure.
- **Phase 6 final tier:** block on Phase 4 + Phase 3 + Phase 5 closure.
- **DoD walkthrough bead:** blocks on ALL prior beads.

Any cycle is a BLOCKING failure (beads can never run).
Any false-ready (a Phase 4 bead claiming readiness without Phase 1 closing) is a BLOCKING failure.

### 4. Bead Self-Containment Audit

Open every bead. Score each on:
- File paths present? (must be repo-relative or absolute, no "find the file")
- Type signatures or function names where applicable?
- Test IDs explicit?
- Dependency bead IDs explicit?
- Acceptance criteria specific (numeric thresholds, exact field values, sorted set comparisons)?

Beads scoring less than 4/5 are under-specified. The bead may be feasible to implement, but it leaves clarification load on the implementing agent — that's drift through dilution.

### 5. Feature-Specific L5 Audit Checks

#### 5.1 ErrorCode Completeness

Find the Phase 1 ErrorCode-extension bead.
- Lists 10 new variants verbatim? ✓ / ✗
- Names `as_str()` matcher? ✓ / ✗
- Acknowledges clippy propagation? ✓ / ✗
- Test IDs for each variant emission? ✓ / ✗

#### 5.2 Multi-Source Recall Audit

Find the Phase 4 multi-source-recall bead(s).
- Names all 5 recall sources (HMRC FPO, CPV-to-chapter priors, rule-based hints, tariff keyword search, ATR codebook reuse)? ✓ / ✗
- Specifies agreement scoring? ✓ / ✗
- Test IDs cover: 5-agree, 5-disagree, FPO-alone is not evidence, Taric must-not-overrule-HMRC?

#### 5.3 Multi-Family Expansion Audit

Find the Phase 4 ten-digit-resolution bead.
- Names "never greedy descent"? ✓ / ✗
- Names locally-cached HMRC tariff hierarchy graph? ✓ / ✗
- Names slot/measure compatibility scoring? ✓ / ✗
- Names shortlist top 3-5 non-dominated branches? ✓ / ✗
- Test IDs cover ≥3 plausible families on a single product?

#### 5.4 Conservative Defaults Audit

Find the Phase 4 beads that wire conservative defaults.
- All 6 default rows from spec §1.12 named (origin → MFN, exporter → residual, quota → out-of-quota, VAT → standard, Incoterm → boundary scenarios, supp units → abstain)? ✓ / ✗
- Test IDs for each parametric default trigger?

#### 5.5 Marketplace Skill Maturation Audit

For each of alibaba/amazon/1688/[pcs]:
- Bead exists? ✓ / ✗
- Existing SKILL.md path referenced (vs net-new)? ✓ / ✗ (alibaba/amazon/1688 must reference existing files at `promotion_stage: candidate`; pcs is net-new — operator decision 2026-05-08 confirmed Scotland IN-scope, so PCS skill creation bead MUST exist)
- Specific deltas listed (not "mature the skill")?
- `promotion_stage: candidate → stable` transition called out?
- Canary harness + golden journey + promotion-gate test IDs assigned?

#### 5.6 Gold-Set Category Coverage

Find Phase 6 gold-set beads.
- ≥3 categories named explicitly? ✓ / ✗
- Each category has a bead? ✓ / ✗
- Calibration corpus path specified per category (`tests/fixtures/calibration/customs-classification/{category}/`)?

#### 5.7 CBAM Forward Placeholder

Find the Phase 6 CBAM bead.
- Placeholder logic only (no rate-bearing logic)? ✓ / ✗
- 2027-01-01 boundary explicit? ✓ / ✗
- Tender-window crossing test ID assigned? ✓ / ✗
- 5 in-scope commodity families named (aluminium, cement, fertiliser, hydrogen, iron and steel)? ✓ / ✗
- Glass/ceramics explicitly out-of-scope? ✓ / ✗

#### 5.8 L5→L6 Gate Producer Trace

For each of the 5 L5→L6 gate conditions:
- `CLASSIFICATION_AMBIGUOUS` — find the bead emitting it.
- `TARIFF_BRANCH_DIVERGENCE` — find the bead emitting it.
- `SUPPLEMENTARY_UNIT_MISSING` — find the bead emitting it.
- `decision_grade = broker_required` — find the bead(s) integrating it.
- classification freshness expired — find the bead with the freshness logic.

Any unproduced gate condition is a BLOCKING failure (L5 closes but L6 cannot gate).

#### 5.9 Daemon Refactor Sizing

Find the Phase 6 main.rs landed-cost refactor bead.
- Sized for 3-5 days (or split into 2-3 dependent beads)?
- Feature-flag wiring explicit?
- Stage 11 live demo bead is separate?
- Regression test gate explicit?

#### 5.10 No Cross-Layer Leakage

L5 beads must NOT modify L4 sync engine, L3 validator infrastructure (beyond ErrorCode extension), L2 desktop, or L1 daemon core. If a bead's file list includes:
- `crates/daemon/src/db/sync.rs` (other than the PUSH_OWNED_RESEED_TABLES update)
- `crates/daemon/src/errors/manifest.rs` (other than the ErrorCode extension lines 20-55+ and as_str() matcher)
- `crates/daemon/src/validation/policy.rs` (no L5 changes expected — ValidationPolicyMode/Trigger are L3 surface)
- `apps/desktop/` files
- `apps/web/` files (other than read-only verification of L5-populated fields rendering correctly)

That's cross-layer leakage. Justify or reject.

---

## Scope Drift Detection (MANDATORY — BLOCKING DIMENSION)

### Red-Flag Language Scan

```bash
br list --json | jq -r '.[].description' | rg -in 'post.?mvp|phase 7\+|stretch goal|nice to have|deferred|out of scope|future iteration|simplified|optional|skip for now|later iteration'
br list --json | jq -r '.[].title' | rg -in 'post.?mvp|phase 7\+|stretch goal|nice to have|deferred|optional'
```

Allowed (rare):
- The CBAM placeholder bead's "rate-bearing logic awaits secondary legislation (post-2027 trial)" — bounded by regulation, not de-scoping.
- §3.6 Open Question references with research back-references.

Any other match is a BLOCKING failure.

### Coverage Verification

For EVERY L5-FR-X.Y.Z in the spec:
- Identify the bead(s) implementing it.
- If ANY spec feature has ZERO corresponding beads → BLOCKING failure.
- Create missing beads before proceeding to step 10.

### Phase Coverage Equality

Count beads per phase:
- Phase 1: 4-6
- Phase 2: 4-6
- Phase 3: 5-7
- Phase 4: 12-20
- Phase 5: 7-10 (Scotland IN-scope; PCS bead set adds ~3-5 to the marketplace maturation set)
- Phase 6: 8-12

If late phases have significantly fewer beads → investigate. Phases are execution order, not importance tiers.

---

## Verdict

After all checks:

### PASS Criteria (audit succeeds)

- [ ] Every L5-FR-X.Y.Z mapped to ≥1 bead.
- [ ] Every L5-T test ID owned by exactly 1 bead.
- [ ] No red-flag language in any bead title or description.
- [ ] Phase coverage equality holds.
- [ ] Dependency DAG is acyclic and reflects strategy phase ordering.
- [ ] All bead self-containment scores ≥ 4/5.
- [ ] All 5 L5→L6 gate conditions have producing beads.
- [ ] All 6 conservative-default rows have implementing beads.
- [ ] All 5 multi-source recall sources have implementation reference in beads.
- [ ] 3 marketplace skill maturation beads (alibaba/amazon/1688) name explicit deltas verbatim from spec §1.15 + §2.3 (Chinese retry, Chinese-only short clusters, alias families, source-health canary expansion, validator overlay binding, candidate→stable promotion). Operator confirmed 2026-05-08 these worked cleanly during QA stages 1-14, so no extra gap-fill beyond spec.
- [ ] PCS skill creation bead exists (Scotland IN-scope, net-new skill directory).
- [ ] ≥3 gold-set categories named in Phase 6 beads.
- [ ] CBAM placeholder bead lists 2027-01-01 boundary, 5 in-scope families, 0 out-of-scope.
- [ ] Daemon main.rs landed-cost refactor bead sized realistically.

### FAIL Criteria (audit fails — execution BLOCKED)

If ANY pass criterion is violated, the audit FAILS. Fix beads. Re-run audit. Do not proceed to step 10 until all pass criteria green.

---

## Follow-Up Pass

> After the audit is complete and all fixes applied:

1. Re-run the spec-coverage script and test-ownership script. Both must produce zero violations.

2. Re-read your fixes. Did fixing one bead break another's dependencies? Re-traverse the DAG.

3. Re-count beads per phase. Did your fixes accidentally rebalance into something less natural?

4. Verify ZERO scope-drift language remains in any bead title or description.

5. Verify every spec feature has at least one bead mapping (re-run script).

6. `bin/br-sync && git add .beads/ docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-bead-alias-map.json && git commit -m "l5-audit-beads: <one-line summary of fixes>" && git push`.

7. **Final assessment:** state explicitly "L5 BEAD AUDIT: PASS" or "L5 BEAD AUDIT: FAIL — <reason>". The pass/fail signal gates step 10.

---

## Output

A clear PASS/FAIL verdict + an updated bead inventory if fixes were applied.

---

## Key Reminders

- Audit is binary. Any drift is a fail.
- Fix beads before re-running. Do not let drift slip to kickoff.
- L5→L6 gate is constitutional — every gate condition must be produced.
- ZERO SCOPE DRIFT.
