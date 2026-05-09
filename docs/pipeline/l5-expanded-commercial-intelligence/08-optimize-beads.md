# L5 Expanded Commercial Intelligence — Beads Optimize (Fresh-Eyes Audit)

## Your Mission

A different agent has just created the L5 bead inventory. You did NOT do that work. You are auditing fresh — bead self-containment, coverage, dependency correctness, ACs sharpness, scope-drift across the bead corpus.

Bead audit is the LAST chance to catch problems before the dedicated `/bead-plan-audit` step (Step 09) runs. Your job is to clean up obvious gaps now so step 09 can focus on systemic issues.

---

## What To Read (Do This First)

### Documents to Audit
```
br list --json                                              — full bead inventory
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-bead-alias-map.json
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-bulk-import.md
```

### Verify Against
```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md   — Spec
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md
```

### Quality Bar
```
docs/pipeline/l4-shared-visibility/archived/2026-05-08-l4-shared-visibility/08-optimize-beads.md
docs/pipeline/l4-shared-visibility/artifacts/l4-bead-alias-map.json
```

### Codebase
```
crates/daemon/src/errors/manifest.rs:20-55       — ErrorCode (Phase 1 bead target)
crates/daemon/src/sources/registry.rs:570-680      — RegisteredSource (Phase 1 bead target)
crates/daemon/src/scheduler/cron.rs:16-34          — cron (Phase 1 bead target)
crates/daemon/src/main.rs:5639-6350+               — landed-cost paths (Phase 6 refactor target)
packages/intelligence/src/qualification/goods-qualification.ts  — 376 lines; scale reference
skills/supplier-price-enrichment/alibaba/SKILL.md  — 241 lines; Phase 5 maturation bead reference
```

---

## Think Independently — 9 Audit Dimensions

### 1. Bead Self-Containment

Open the 5 largest beads (by description length). For each:
- Are file paths absolute or repo-relative? Never "find the file".
- Are type signatures or function names included where applicable?
- Are test IDs explicit (not "the relevant tests")?
- Are dependency bead IDs explicit (not "after Phase 1 closes")?
- Could a fresh agent claim this bead and implement from it alone, without re-reading the spec? If not, the bead is under-specified.

### 2. Coverage Completeness

```bash
SPEC_IDS=$(rg -o 'L5-FR-[0-9]+\.[0-9]+\.[0-9]+' docs/strategy/layers/layer-5-expanded-commercial-intelligence.md | sort -u)
TDD_IDS=$(rg -o 'L5-T-[a-z_]+-[0-9]+' docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md | sort -u)
ALIAS_FILE=docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-bead-alias-map.json

# Spec coverage
for id in $SPEC_IDS; do
  jq -e --arg id "$id" '.[] | select(.spec_ids | contains([$id]))' "$ALIAS_FILE" >/dev/null || echo "MISSING SPEC: $id"
done

# Test ownership
for id in $TDD_IDS; do
  jq -e --arg id "$id" '.[] | select(.test_ids | contains([$id]))' "$ALIAS_FILE" >/dev/null || echo "MISSING TEST: $id"
done

# Test duplication (each test owned by exactly one bead)
jq -r '.[] | "\(.bead_id) \(.test_ids[])"' "$ALIAS_FILE" | awk '{print $2}' | sort | uniq -d
```

Any output is a gap. Fix.

### 3. Dependency DAG Correctness

```bash
br ready --json | jq -r '.[].id'    # Phase 1 beads should be ready
br list --json | jq -r '.[] | "\(.id) \(.status) \(.blocked_by)"' | head -60
```

Verify:
- Phase 1 beads have no `blocked_by` (or only block on the L5 epic).
- Phase 2/3/5 beads block on Phase 1 closure.
- Phase 4 beads block on Phase 1 + 2.
- Phase 6 beads block on Phase 4 + 3 + 5 closure.
- DoD walkthrough bead blocks on ALL prior beads.

If the DAG is wrong, beads will run in the wrong order or be falsely-ready.

### 4. AC Sharpness

For each Phase 4 customs bead, verify ACs:
- Numeric thresholds with explicit values (e.g., `customs_cost_spread_bps > 1500 → emit CLASSIFICATION_AMBIGUOUS`).
- Field equality checks with exact values (e.g., `exporter_rate_basis = 'residual_default'` when no exporter proof).
- Set comparisons with sorted lists (e.g., `candidate_recall_agreement contains [hmrc_fpo, cpv_priors, rule_based]`).

Vague ACs (e.g., "produces correct duty estimate") are insufficient. Tighten.

### 5. ErrorCode Bead Completeness

Find the Phase 1 ErrorCode-extension bead. Open it. Verify:
- All 10 new variant names listed verbatim (not "the 10 from spec §3.5").
- `as_str()` matcher updates listed.
- Clippy propagation sites listed (or noted as "all sites surfaced by `cargo clippy --all-targets -- -D warnings` after enum addition; complete in same bead").
- Test ID for emission of each new variant referenced.

### 6. Migration Bead Correctness

Find the Phase 1 migration bead. Verify:
- File path `migrations/NNNN_l5_product_commodity_codes.sql` (NEXT-FREE NNNN, not the spec's incorrect `sqlite/migrations/0002_*`).
- DDL fields named (likely `(product_candidate_id, commodity_code, recall_source, agreement_score, created_at)` plus PK / FK constraints).
- Sync hookup verified: `local_outbox` should auto-detect mutations to the new table; verify the L4 sync.rs PUSH_OWNED_RESEED_TABLES list is updated to include `product_commodity_codes` (this is a bead requirement — sync.rs edit).
- Supabase projection alignment: confirm `infra/supabase/migrations/0001_projection_tables.sql` has the projection table (it does, per Subagent 1 research). No new Supabase migration needed.

### 7. Marketplace Skill Maturation Beads

Find Phase 5 beads for alibaba/amazon/1688. For each:
- Bead description references the existing SKILL.md by path (`skills/supplier-price-enrichment/alibaba/SKILL.md`).
- Bead lists EXACT deltas (not "mature the skill"):
  - Alibaba: Chinese retry on zero/accessory hits; Ready-to-Ship filter with `quoteUsabilityConfidence` bump; source-health canary expansion; anti-bot circuit breaker; `promotion_stage: candidate → stable`.
  - 1688: Chinese-only short clusters (2-4 terms); domestic wholesale floor pricing in result; auth-refresh/1688 binding; `promotion_stage: candidate → stable`.
  - Amazon: alias families; structured title vocabulary; browser-first (PA-API 5.0 sunset 2026-04-30); `promotion_stage: candidate → stable`.
- Test IDs include canary harness AND golden journeys AND promotion gate.

### 8. Phase 6 Refactor Bead Sizing

Find the Phase 6 daemon main.rs landed-cost refactor bead. Verify:
- Target file `crates/daemon/src/main.rs` lines 5639-6350+ (or whatever range the actual landed-cost code occupies).
- Strategy explicit: feature-flag wired (decision-grade flag default OFF in early Phase 6, ON at end).
- Bead sized for 3-5 days of focused work — single bead is OK; consider splitting into 2-3 beads if separation is natural (e.g., one for delegation interface, one for migration of existing logic, one for feature-flag flip).
- Stage 11 live demo bead is its own dedicated bead (not bundled into the refactor bead).
- Regression test gate explicit: existing landed-cost integration tests must pass before bead closes.

### 9. PCS Bead Existence (UNCONDITIONAL — Scotland IN-scope)

Operator confirmed Scotland IN-SCOPE on 2026-05-08 (manifest `recorded_decisions.decision_2_pcs_scotland_scope`). The PCS skill bead set MUST exist. Verify:
- A bead for `skills/procurement-ingestion/pcs/` skill directory creation exists in Phase 5.
- A bead for daemon `RegisteredSource` entry for PCS in `crates/daemon/src/sources/registry.rs` exists.
- A bead for PCS canary fixtures in `tests/fixtures/calibration/marketplace-search/pcs/` (or `procurement-ingestion/pcs/`) exists.
- A bead for bootstrap doc updates listing PCS OCDS Web API account state exists.
- No "PCS optional" or "PCS deferred" wording in any bead.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan

```bash
br list --json | jq -r '.[].description' | rg -in 'post.?mvp|phase 7\+|stretch goal|nice to have|deferred|out of scope|future iteration|simplified|optional|skip for now|later iteration'
```

Any output is a BLOCKING failure. Fix before proceeding to step 09.

### Coverage Verification

For every L5-FR-X.Y.Z requirement → at least one bead. Use the `jq` script in dimension #2.

### Phase Coverage Balance

```bash
br list --json | jq -r '.[] | select(.id | startswith("l5")) | .labels[]' | grep -oE 'phase-[0-9]+' | sort | uniq -c
```

Compare phase bead counts to expected:
- Phase 1: 4-6 beads
- Phase 2: 4-6 beads
- Phase 3: 5-7 beads
- Phase 4: 12-20 beads
- Phase 5: 7-10 beads (alibaba/amazon/1688 maturation + PCS skill creation + canary harness + optional validator overlay; Scotland IN-scope)
- Phase 6: 8-12 beads

If a phase is materially below its expected range, late-phase de-scoping is suspected. Investigate.

### Drift Checklist

- [ ] Every L5-FR-X.Y.Z mapped to a bead.
- [ ] No bead description contains red-flag language.
- [ ] Every test ID owned by exactly one bead.
- [ ] Phase 1 includes ErrorCode + migration + registry + cron + env-var beads.
- [ ] Phase 4 includes all 6 customs files + integration beads.
- [ ] Phase 5 includes all 3 marketplace skill maturations (alibaba/amazon/1688) + PCS skill creation + canary harness + (optional) validator overlay.
- [ ] Phase 6 includes gold-set per category + CBAM placeholder + accuracy harness + 3 comparison adapters + main.rs refactor + Stage 11 demo + DoD walkthrough.
- [ ] No duplicate test IDs across beads.
- [ ] No "TODO" or "[placeholder]" in any bead description.

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

1. Re-run the spec-coverage and test-ownership grep scripts. Verify zero misses, zero duplicates.

2. Open `br ready --json`. Phase 1 beads should be ready. Verify the readiness ordering matches the strategy's phase plan.

3. Open `crates/daemon/src/main.rs` and grep `landed_cost`. Count actual lines touching landed-cost. Verify the Phase 6 refactor bead is sized accordingly.

4. Open `tests/fixtures/calibration/` and look at the existing largest variant. Each Phase 6 gold-set bead must reference the calibration corpus extension explicitly.

5. Open `skills/supplier-price-enrichment/alibaba/SKILL.md` and check the existing Chinese retry section. Phase 5's alibaba bead description must list new edits to add (not just "improve Chinese retry").

6. Did your fixes accidentally introduce duplicates? Re-run the duplicate-test-ID check.

7. Run `bin/br-sync && git add .beads/ && git commit -m "l5-optimize-beads: <one-line fix summary>" && git push`.

---

## Output

The bead inventory updated in place. Alias map + bulk-import doc updated to reflect any added/removed beads.

---

## Key Reminders

- Bead self-containment > spec hierarchy. Each bead must stand alone.
- Phase coverage balance is the silent-de-scoping detector — investigate any thin late phase.
- ZERO SCOPE DRIFT in bead descriptions.
