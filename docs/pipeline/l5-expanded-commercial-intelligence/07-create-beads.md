# L5 Expanded Commercial Intelligence — Create Beads

## First Action

Invoke the `/beads-from-sources` skill.

---

## Your Mission

Materialize beads from the three SOTs (spec, TDD, strategy) for L5 implementation. Every L5-FR requirement gets at least one bead. Every L5-T test ID is assigned to exactly one bead. Beads are SELF-CONTAINED — an agent must be able to implement from a single bead alone, without re-reading the spec.

Bead prefix: `l5`. Bead CLI: `br` (NEVER `bd`). Bead sync: `bin/br-sync`.

---

## Source Documents (Read In This Order)

### Primary Inputs (READ FULLY)
```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md   — Spec (SOT #1)
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md  — TDD (SOT #2)
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md  — Strategy (SOT #3)
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md  — Bootstrap (env prerequisites)
```

### Quality Bar
```
docs/pipeline/l4-shared-visibility/archived/2026-05-08-l4-shared-visibility/07-create-beads.md  — THE bar
docs/pipeline/l4-shared-visibility/artifacts/l4-bead-alias-map.json                              — bead → spec/test mapping (reference shape)
```

### Codebase Context (REFERENCED IN BEAD ACCEPTANCE CRITERIA)
```
crates/daemon/src/errors/manifest.rs:20-55       — ErrorCode enum (L5 extends with 10 variants)
crates/daemon/src/sources/registry.rs:570-680      — RegisteredSource (L5 adds 4-5 entries)
crates/daemon/src/scheduler/cron.rs:16-34          — CronJobDefinition (L5 adds 4 entries)
crates/daemon/src/main.rs:5639-6350+               — landed-cost paths (Phase 6 refactor target)
packages/intelligence/src/                         — current 3 subdirs (L5 adds 4 new)
migrations/0001_sqlite_state_initial.sql:322-394   — landed_cost_estimates schema (no DDL change for the table)
infra/supabase/migrations/                         — projection / RLS surfaces
skills/supplier-price-enrichment/                  — 4 existing skill dirs (alibaba/amazon/1688/ebay)
tests/fixtures/calibration/                        — gold-set + canary corpus structure
```

### Bead Discipline
```
AGENTS.md:262-305                                  — authoritative `br` rule (constitutional)
AGENTS.md:21-33                                    — Rule 2 (br-claim / br-complete wrappers)
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**100% spec coverage is MANDATORY.** Every L5-FR-X.Y.Z requirement must be implemented by at least one bead. If any requirement has zero corresponding beads, that is a BLOCKING failure.

You MAY:
- Group multiple closely-related requirements into a single bead (with explicit ID list in the bead description).
- Split a single requirement across multiple beads if the implementation has natural seams.
- Resequence beads within a phase based on dependencies.

You MAY NOT:
- Create beads that cover only a subset of the spec.
- Defer requirements to "post-L5", "Phase 7+", or any non-existent later layer.
- Mark beads "optional" or "stretch".
- Reduce acceptance criteria below what the spec / TDD define.
- Have late-phase beads with thinner acceptance criteria than early-phase beads.

---

## Key Context From Codebase Research

1. **L5 layer doc epic** `testproject1-4gh` exists with documentation children `4gh.1` through `4gh.4` from the layer-doc authoring work. **DECISION REQUIRED:** Does the implementation pipeline (a) reuse `testproject1-4gh` as the implementation epic; (b) create a sister implementation epic; or (c) thread implementation beads as new children of `4gh`? **RECOMMENDED: option (b)** — clean separation. Create a new epic `Epic: L5 Implementation — Expanded Commercial Intelligence` with the bead prefix `l5`. Document the choice in the bead inventory.

2. **L4 used `testproject1-4zq` as the existing L4 epic with no children at bead-creation time.** L4 added all implementation beads as children. The L5 case differs because the documentation children already exist. Hence the recommendation for a sister epic.

3. **PCS Scotland scope: IN-SCOPE** (resolved 2026-05-08 by operator; manifest `recorded_decisions.decision_2_pcs_scotland_scope`). Phase 5 unconditionally creates `skills/procurement-ingestion/pcs/` (full skill directory standard) plus daemon registry entry. ~3-5 PCS beads in Phase 5.

4. **Bead self-containment is non-negotiable.** Each bead must contain: full file paths, type definitions or signatures, test IDs to satisfy, acceptance criteria, dependency bead IDs, and a Files-to-Create-or-Modify section. An implementation agent must be able to claim the bead and implement from the bead alone.

5. **All 36 decision-grade columns on `landed_cost_estimates` already exist as nullable from L0.** Beads that populate these columns reference them by exact column name (snake_case from the migration). NO DDL changes for the table.

6. **`product_commodity_codes` local SQLite junction is L5's single new table** (per validate-spec callout #1). The bead that creates the migration is in Phase 1.

7. **Marketplace skill maturation beads** must reference the existing SKILL.md files (alibaba: 241 lines, amazon: 92, 1688: 90, ebay: 86) and specify the exact deltas — Chinese retry, Ready-to-Ship filter, alias families, etc. — not handwave "mature the skill".

8. **All of L5's daemon code edits are surgical.** Phase 1 beads should each name the target file path and the lines to edit. ErrorCode bead names `crates/daemon/src/errors/manifest.rs:20-55+` (enum + as_str() matcher), plus the 5-15 propagation sites that clippy will surface across `validation/policy.rs`, `validation/executor.rs`, `errors/markdown.rs`, `tracing/structured.rs`, `ipc/handlers/*`.

{ENRICHMENT: strategy metadata — after step 06, this section will name the exact phase count, expected bead count, and the phase-to-bead mapping.}

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)

1. The validated spec — **READ FULLY**.
2. The TDD — **READ FULLY**.
3. The strategy — **READ FULLY** (this drives bead grouping).
4. The bootstrap — **REFERENCE** (informs Phase 1 env-var beads).
5. `AGENTS.md:21-33, 262-305` — **READ** (br wrapper discipline).
6. `docs/pipeline/l4-shared-visibility/archived/2026-05-08-l4-shared-visibility/07-create-beads.md` — **STUDY** (this is your shape).

### Step 1: Verify Pre-Conditions (BEFORE Invoking Skill)

The three project-level decisions from step 01 §6 must already be recorded in the validated spec. If any are missing, STOP and re-open step 01.

- [ ] **Decision 1 (product_commodity_codes ownership)** — verified recorded; resolution is Option A (new SQLite migration) or Option B (JSONB on product_candidates). Phase 1 bead behavior follows the recorded decision.
- [ ] **Decision 2 (PCS Scotland scope)** — verified recorded as IN-SCOPE (operator decision 2026-05-08). PCS bead set is unconditional in Phase 5.
- [ ] **Decision 3 (epic structure)** — verified recorded; if Option (b) sister epic, the new epic ID is captured in the spec entry. Beads will be created under that epic.
- [ ] **L5→L6 gate trace from strategy** — confirm which phase produces each of the 5 blocker conditions (`CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`, classification freshness expired).
- [ ] **Skill state** — confirm the alibaba/amazon/1688 SKILL.md files still exist at `promotion_stage: candidate` (no other agent has matured them mid-pipeline).

### Step 2: Invoke `/beads-from-sources`

Inputs:
- Spec: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`
- TDD: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md`
- Strategy: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md`
- Bead prefix: `l5`
- Bead CLI: `br`
- Epic decision: per pre-condition resolution

Key context to provide:
- L4 audit verdict 2026-05-08 was COMPLETE — L5 builds on a finished platform.
- L5 has 6 phases per strategy (Foundation, FX, Freight, Customs, Skills, Integration+Gold-set+CBAM).
- Total bead estimate: ~40-55 beads.
- Each bead must reference: target file paths, type signatures (where applicable), test IDs to satisfy, acceptance criteria, dependency bead IDs.
- Use `br create` (NOT `bd`) for bead creation.

### Step 3: Feature-Specific Guidance

While the skill creates beads, ensure these are honored:

#### Phase 1 — Foundation (4-6 beads)
- One bead per: SQLite migration (`product_commodity_codes`), ErrorCode enum extension (10 variants + matcher + clippy-surfaced sites), source registry entries (4-5 RegisteredSource entries), cron scheduler entries (4 CronJobDefinition entries), env-var loader (HMRC/Xe/Freightos/SeaRates/Taric Support).
- ErrorCode bead: name target file `crates/daemon/src/errors/manifest.rs:20-55+` (enum) + same-file `as_str()` matcher, list the 10 new variants verbatim, list clippy-propagation expectations across `validation/policy.rs`, `validation/executor.rs`, `errors/markdown.rs`, `tracing/structured.rs`, `ipc/handlers/*`.
- Migration bead: name file `migrations/0013_l5_product_commodity_codes.sql` (or next-free `NNNN`), name DDL fields, include downstream sync hookup verification.

#### Phase 2 — FX Engine (4-6 beads)
- One bead per file: `hmrc-exchange-rates.ts`, `xe-commercial-rates.ts`, `fx-engine.ts`, `fx-types.ts`. Plus integration bead: monthly rollover detection + recalculation queue + `FX_RATE_MISSING` emission.
- Each bead names the exact file path under `packages/intelligence/src/fx/`.

#### Phase 3 — Freight Engine (5-7 beads)
- One bead per file: `mode-selector.ts`, `freightos-client.ts`, `searates-client.ts`, `freight-engine.ts`, `freight-types.ts`. Plus integration bead: dual-source cross-check + `FREIGHT_MODE_LOW_CONFIDENCE` emission + boundary scenario logic.

#### Phase 4 — Customs Engine (12-20 beads)
- One bead per file: `description-quality-gate.ts`, `multi-source-recall.ts`, `ten-digit-resolution.ts`, `measure-scanner.ts`, `exporter-resolution.ts`, `classification-freshness.ts`. Plus integration beads.
- Multi-source recall is multi-bead inside itself (5 source adapters + agreement scoring).
- Measure scanner is multi-bead inside (per measure family: AD/CVD, safeguard, quota; plus interaction rule logic; plus compound/specific duty preservation; plus supplementary-unit handling).
- Classification freshness: 4 vectors get either 1 bead (single file orchestrating all 4) or 4 beads (one per vector). Pick based on file decomposition.

#### Phase 5 — Marketplace Skill Maturation + Validator Overlay (4-6 beads)
- One bead per skill maturation: alibaba (Chinese retry, Ready-to-Ship filter, source-health canary), 1688 (Chinese-only short clusters, domestic wholesale floor), amazon (alias families, browser-first, alias families). Each bead lists the EXACT delta vs current SKILL.md.
- One bead for PCS skill creation (mandatory — Scotland IN-scope per operator decision 2026-05-08): `skills/procurement-ingestion/pcs/{SKILL.md,result.schema.json,checkpoint.schema.json,canary.inputs.jsonl,golden.assertions.yaml}` + daemon `RegisteredSource` entry.
- One bead for validator overlay (`skills/validation/overlay-customs-classification/`) IF recommended by step 03 TDD.

#### Phase 6 — Integration + Gold-Set + CBAM (8-12 beads)
- One bead per gold-set category (3+ beads).
- One bead for CBAM placeholder logic.
- One bead for evidence-tiered accuracy harness (`packages/intelligence/src/customs/accuracy.ts`).
- One bead per comparison adapter (3 beads): dutify, zonos, avalara.
- One bead for daemon main.rs landed-cost path refactor (largest single bead in L5 — size for 3-5 days).
- One bead for Stage 11 live demo against a real Contracts Finder lot.
- One bead for L5 DoD walkthrough (final exit gate).

### Step 4: Verify Output

After `/beads-from-sources` completes:
- [ ] Run `br list --json | jq '. | length'` — total bead count is in **40-55 range** (Scotland is IN-scope, so PCS adds ~3-5 beads).
- [ ] Run `br ready --json` — Phase 1 beads are ready (no upstream dependencies).
- [ ] Every L5-FR-X.Y.Z appears in some bead's description or ACs:
      ```bash
      SPEC_IDS=$(rg -o 'L5-FR-[0-9]+\.[0-9]+\.[0-9]+' docs/strategy/layers/layer-5-expanded-commercial-intelligence.md | sort -u)
      BEAD_DESCS=$(br list --json | jq -r '.[].description' 2>/dev/null)
      for id in $SPEC_IDS; do echo "$BEAD_DESCS" | grep -q "$id" || echo "MISSING: $id"; done
      ```
      The check must produce zero MISSING lines.
- [ ] Every L5-T test ID is owned by exactly one bead (no duplication).
- [ ] Every bead has: target file paths, ACs, test IDs, dependency bead IDs.
- [ ] No bead is empty.
- [ ] Run `bin/br-sync` (NEVER raw `br sync`). Confirm `.beads/` is updated and clean.
- [ ] `git add .beads/ && git commit -m "l5: create implementation beads from 3 SOTs"`.
- [ ] `git pull --rebase && git push`.

### Step 5: Follow-Up Pass (Sub-Optimization)

> Run after Step 4.

1. Open `docs/pipeline/l4-shared-visibility/artifacts/l4-bead-alias-map.json` and study the bead → test ID + bead → spec ID mapping shape. Generate the analogous `l5-bead-alias-map.json` for L5. This is the trace artifact step 09 audit will use.

2. Open `crates/daemon/src/errors/manifest.rs:20-55`. Walk the existing 44 ErrorCode variants. For each of L5's 10 new variants, verify the Phase 1 bead description names them verbatim (not "the 10 customs error variants from spec §3.5"). Bead self-containment requires the names IN the bead.

3. Open `packages/intelligence/src/qualification/goods-qualification.ts` (376 lines) — the largest existing intelligence file. This is your scale reference for what a single L5 customs file might look like (likely 300-700 lines for the larger ones like multi-source-recall.ts and ten-digit-resolution.ts). If your beads imply files <100 lines each, you've under-specified scope (a bead's worth of work is ~half-day to 2-day cycle). If beads imply files >1000 lines each, you've over-bundled (split into multiple beads).

4. Open `tests/fixtures/calibration/` and pick the largest existing calibration variant. Each Phase 6 gold-set category bead must name the calibration corpus extension explicitly: `tests/fixtures/calibration/customs-classification/{category}/canary.inputs.jsonl` + `golden.assertions.yaml`. If the bead just says "create gold-set for X", that's underspecified.

5. Open `skills/supplier-price-enrichment/alibaba/SKILL.md` (the 241-line one). Read the existing Chinese retry section and anti-bot logic. Phase 5's alibaba maturation bead must list specific deltas — not "improve the skill". Add the exact bullet list of edits to the bead description.

6. Walk the bead dependency graph. Run `br ready --json` and look at the readiness order. Phase 1 beads should be ready immediately. Phase 2 + 3 + 5 should become ready when Phase 1 closes (parallel). Phase 4 should become ready when Phase 1 + 2 close. Phase 6 should become ready when Phase 4 + 3 + 5 close. If the dependency graph doesn't reflect this, re-do the `blocked_by` declarations.

7. For each marketplace skill bead, verify the test ID assignment includes the canary harness test (`L5-T-marketplace_canary_*`) AND the promotion-flow test (`L5-T-skill_golden_journeys_marketplace_*` or similar). Both must run for the skill to advance from candidate → stable.

8. Run `br lint` (or whichever lint command the bead CLI provides). Fix any warnings before commit.

### Step 6: Scope-Preservation Check (MANDATORY — BLOCKING)

1. **Coverage matrix:** every L5-FR-X.Y.Z → at least one bead. Run the verification grep from Step 4.
2. **No duplication:** every L5-T test ID owned by exactly one bead. Verify via `jq` over the bead list.
3. **Red-flag scan:** grep all bead descriptions for `post-MVP`, `Phase 7+`, `stretch goal`, `nice to have`, `deferred`, `out of scope`, `future iteration`, `simplified`, `optional`. Any match in a bead description is a BLOCKING failure.
4. **Phase coverage equality:** count beads per phase. Late phases must not be materially thinner than early phases. If Phase 6 has 3 beads and Phase 4 has 18, that's likely fine (customs IS the largest surface). If Phase 6 has 1 bead, that's de-scoping disguised as bundling.

---

## Output

A complete bead inventory under bead prefix `l5`, plus:
- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-bead-alias-map.json` (bead → spec ID + test ID mapping).
- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-bulk-import.md` (human-readable mirror of the bead inventory, per L4 convention).

---

## Key Reminders

- Bead self-containment is non-negotiable. An agent claims a bead and implements from it alone.
- Bead CLI is `br`. Bead sync is `bin/br-sync`. NEVER raw `br sync`.
- Use `br create` (verify the exact subcommand against AGENTS.md if uncertain).
- ErrorCode enum extension is harder than it looks — clippy will surface 5-15 propagation sites. Size Phase 1 beads accordingly.
- PCS Scotland scope: RESOLVED 2026-05-08 as IN-SCOPE. Phase 5 includes PCS bead set unconditionally.
- ZERO SCOPE DRIFT: 100% spec coverage. No "stretch goal" or "nice to have" anywhere.
