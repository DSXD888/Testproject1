# Validate Branch A: Qualification Pipeline (Extract Lots → Prequalify → Qualify Lot)

ULTRATHINK and be meticulous.

## FRESH-EYES CURRENT-STATE OVERRIDE (2026-04-26)

This validation handoff predates the full rerun. The Branch A validation manifest is already GREEN for the earlier 103-notice snapshot, and the current DB has moved to 133 notices.

Current read-only DB snapshot:

```text
contract_notices=133
notice_lots=179
qualification_runs=308
process_stage_heads:
  extract lots=123
  prequalify notice=133
  qualify lot=179
```

Use this handoff as the Branch A validation checklist, but update all expected counts from the live DB before judging pass/fail. Do not assume the old 103-notice baseline. If validation fails, fix the current code/data path and write an updated manifest.

## MISSION

Run, validate, and fix the **Branch A qualification pipeline** against the current live DB until every stage is green. This pipeline determines which ingested contracts are viable physical-goods opportunities; current data has 133 notices, not the old 103-notice baseline. Use the validation queries below, update all expected counts from the live database, and iterate until Branch A is clean.

**Three stages to execute (in this order — the daemon's execution order):**

```
Stage 6:  Extract Lots        →  Ensure every notice has ≥1 lot (FK prerequisite)
Stage 4a: Prequalify Notice   →  CPV goods/services classification + expiry check
Stage 4b: Qualify Lot         →  Per-lot qualification inheriting from notice
```

**Already validated (prerequisites, NOT re-run here):**

```
Stages 1-2: Ingest + Normalize     ✅  103 notices across 3 sources
Stage 5:    Acquire + Extract Docs  ✅  202 structural, 108 semantic extractions
```

**You must work in a loop.** Run the stages, validate the output, diagnose any failures, fix them, and re-run. Do NOT stop after the first attempt. Keep iterating until ALL validation checks pass with zero failures. Only report completion when Branch A is fully green.

```
┌─────────────────────────────────────────────────────────┐
│                    THE LOOP                               │
│                                                          │
│   1. Run stages                                          │
│   2. Validate output (queries below)                     │
│   3. Any failures?                                       │
│      ├─ YES → Diagnose root cause, fix code/data,       │
│      │        go back to step 1                          │
│      └─ NO  → Branch A is GREEN. Report results.         │
│                                                          │
│   DO NOT STOP UNTIL STEP 3 = NO                          │
└─────────────────────────────────────────────────────────┘
```

**Out of scope:** Branch B (Stage 3: Relate) is tested separately after Branch A is green.

---

## EXECUTION ORDER & DEPENDENCIES

```
                       WHAT THE DAEMON RUNS (in order)
                       ───────────────────────────────
  ┌──── run_extract_lots_stage(source_id) ──── Stage 6
  │     Creates implicit lots for notices with zero lots.
  │     Without this, Stage 4a crashes on the lot_id FK.
  │
  ├──── run_prequalify_stage(source_id) ────── Stage 4a
  │     CPV classification + expiry check per notice.
  │     Writes qualification_runs (basis='notice_only').
  │
  ├──── run_qualify_lot_stage() ───────────── Stage 4b
  │     Per-lot qualification inheriting from notice.
  │     Writes qualification_runs (basis='lot_level').
  │     Runs globally (all sources, not filtered).
  │
  └──── run_relate_stage() ────────────────── Stage 3 (Branch B)
        OCID dedup across sources. Harmless, let it run.
        Runs globally. We validate this separately later.
```

**EXIT CRITERIA — Branch A is GREEN when ALL pass:**
  - [ ] Every notice has ≥1 lot in `notice_lots`
  - [ ] Every notice has a `qualification_runs` entry with `basis='notice_only'`
  - [ ] Every lot has a `qualification_runs` entry with `basis='lot_level'`
  - [ ] Stage heads exist for `extract lots`, `prequalify notice`, `qualify lot`
  - [ ] Zero orphaned records (FK integrity)
  - [ ] Qualification categories are sensible (not ALL unknown)
  - [ ] `cargo check + clippy` clean (if any code changes were needed)

---

## KEY CONCEPTS

Before reading any code, understand how the daemon tracks pipeline progress:

- **`process_stage_status`** — one row per stage execution per scope unit (notice, lot, process). Records whether a stage `succeeded`, `failed`, or `quarantined` for a given entity. Has a `stage_run_id` that groups all writes from one execution batch.
- **`process_stage_heads`** — one row per (process, stage, scope_kind, scope_id). Points to the LATEST successful `stage_run_id`. Used as a "has this stage been done?" check — if a head exists, the stage doesn't re-run for that entity.
- **`persist_stage()`** helper — writes both the `process_stage_status` row and advances the `process_stage_heads` entry atomically.

When you see a function check `NOT EXISTS (SELECT 1 FROM process_stage_heads WHERE stage = '...')`, it's asking: "has this stage already been completed for this entity?" This is how stages are idempotent — re-running `--pipeline` skips already-completed work.

**Read AGENTS.md first** — it is the single source of truth for all agent instructions in this project.

---

## PREREQUISITES

### Verify daemon compiles

```bash
cd /Users/zacfarrell/Desktop/Testproject1
cargo check --all-targets
```

### Verify pre-Branch-A data is healthy

```bash
sqlite3 state/state.db "
SELECT '=== PRE-BRANCH-A STATE ===';
SELECT 'Notices:                   ' || COUNT(*) FROM contract_notices;
SELECT 'Processes:                 ' || COUNT(*) FROM procurement_processes;
SELECT 'Lots (current):            ' || COUNT(*) FROM notice_lots;
SELECT 'Notices WITHOUT lots:      ' || COUNT(*) FROM contract_notices cn
  WHERE NOT EXISTS (SELECT 1 FROM notice_lots nl WHERE nl.notice_id = cn.id);
SELECT 'Qualification runs:        ' || COUNT(*) FROM qualification_runs;
SELECT 'Document extractions:      ' || COUNT(*) FROM document_extractions;
SELECT 'Semantic extractions:      ' || COUNT(*) FROM document_semantic_extractions;
SELECT 'Product candidates:        ' || COUNT(*) FROM product_candidates;
SELECT '';
SELECT '=== STAGE HEADS (pre-run) ===';
SELECT stage, scope_kind, COUNT(*) FROM process_stage_heads GROUP BY stage, scope_kind ORDER BY stage;
"
```

Expected: ~103 notices, ~48 lots, ~93 lotless notices, 0 qualification_runs, 202 doc_extractions, 108 semantic_extractions, 545 product_candidates. Stage heads should exist for `Normalize` and `acquire docs` but NOT for `extract lots`, `prequalify notice`, or `qualify lot`.

---

## PHASE 1: RESEARCH (mandatory before execution)

### 1A. Read the Pipeline Functions

Find and read these functions in `crates/daemon/src/main.rs` (search by name — line numbers may have shifted):

```bash
grep -n "fn run_deterministic_pipeline\|fn run_extract_lots_stage\|fn run_prequalify_stage\|fn run_qualify_lot_stage\|fn run_relate_stage" crates/daemon/src/main.rs
```

**`run_deterministic_pipeline()`** — orchestrator. Calls stages in this exact order:
1. `run_extract_lots_stage(source_id)` — creates implicit lots
2. `run_prequalify_stage(source_id)` — CPV classification
3. `run_qualify_lot_stage()` — lot-level qualification (global, no source filter)
4. `run_relate_stage()` — OCID dedup (global) — will also run, that's fine

**`run_extract_lots_stage(source_id)`** — Stage 6:
- SQL: finds notices WHERE source_id matches AND no lots exist (`NOT EXISTS (SELECT 1 FROM notice_lots nl WHERE nl.notice_id = cn.id)`)
- Creates one implicit lot per notice: id = `{notice_id}-lot-implicit`, lot_key = `implicit-0`
- Writes `process_stage_status` for `'extract lots'` stage
- This is a trivial deterministic operation (no LLM, no files)

**`run_prequalify_stage(source_id)`** — Stage 4a:
- SQL: finds notices WHERE `Normalize` stage head exists AND `prequalify notice` stage head does NOT exist
- Extracts CPV division from `raw_documents.payload_json` → OCDS field path: `tender.classification.id` → first 2 digits
- Classification rules:
  ```
  CPV 03-44  → "goods"    (density 8500 bps)
  CPV 45     → "works"    (density 3000 bps)
  CPV 48-49  → "goods"    (density 8500 bps)
  CPV 50-98  → "services" (density 1500 bps)
  No CPV     → "unknown"  (density 4500 bps)
  ```
- Checks deadline expiry: `deadline >= today` (expired = rejected)
- Writes `qualification_runs` with `qualification_basis = 'notice_only'`
- **CRITICAL FK:** Uses `(SELECT nl.id FROM notice_lots nl WHERE nl.notice_id = cn.id LIMIT 1)` for `lot_id` — this is `NOT NULL`, so it crashes if no lots exist. **This is why Stage 6 MUST run first.**

**`run_qualify_lot_stage()`** — Stage 4b (global, no source filter):
- SQL: finds lots WHERE parent notice has `prequalify notice` head AND lot does NOT have `qualify lot` head
- Inherits `expiry_qualified`, `goods_qualified`, `goods_confidence_bps`, `legal_category` from the notice's latest `qualification_runs` entry
- If no parent qualification found, defaults to: expiry=true, goods=false, confidence=4500, category="unknown"
- Writes `qualification_runs` with `qualification_basis = 'lot_level'`

You must be able to answer:
1. Why must `run_extract_lots_stage` run before `run_prequalify_stage`? (FK constraint on lot_id)
2. What CPV divisions map to "goods"? (3-44 and 48-49)
3. What does `run_qualify_lot_stage` do when the parent notice qualification doesn't exist? (defaults to unknown)

### 1B. Source Filter Behavior

`run_extract_lots_stage()` and `run_prequalify_stage()` take a `source_id` parameter — they only process notices from that source. `run_qualify_lot_stage()` and `run_relate_stage()` are global (no source filter).

We have 3 sources: `procontract`, `contracts-finder`, `fts`. The `--pipeline {source_id}` handler runs for one source at a time. **You must run it for each source.**

### 1C. The `--pipeline` CLI Handler

Search for `cli.pipeline` in main.rs to find the handler. It calls `run_deterministic_pipeline()` which runs ALL four stages including `run_relate_stage()` (Stage 3: Relate). Relate is technically Branch B scope, but it's harmless — just creates `process_relations` for OCID duplicates and marks processes as related. Let it run. Focus your VALIDATION on Branch A stages (6, 4a, 4b).

### 1D. What If the Daemon Crashes

If `--pipeline` exits with an error:
1. Read the error message — it will reference a function name and error context
2. Check if it's a DB constraint error (like the FK or UNIQUE violations we've seen before)
3. Search for the error text in the Rust source to find the exact line
4. Fix the code issue, recompile, re-run
5. If it's a data issue (missing rows), investigate with SQL queries

---

## PHASE 2: EXECUTION

Run the deterministic pipeline for ALL THREE sources:

```bash
cargo run --release --bin daemon -- --pipeline procontract \
  --state-root state --runtime-dir runtime --artifact-root artifacts

cargo run --release --bin daemon -- --pipeline contracts-finder \
  --state-root state --runtime-dir runtime --artifact-root artifacts

cargo run --release --bin daemon -- --pipeline fts \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

Each invocation outputs a summary like:
```
deterministic pipeline complete: lots_created=X prequalified=Y lots_qualified=Z relations=W
```

**Record all three summary lines.** If any invocation errors, diagnose before continuing.

---

## PHASE 3: VALIDATION (run ALL checks, report results)

### Check 1: Every notice has at least one lot

```bash
sqlite3 state/state.db "
SELECT 'Notices without lots: ' || COUNT(*) FROM contract_notices cn
  WHERE NOT EXISTS (SELECT 1 FROM notice_lots nl WHERE nl.notice_id = cn.id);
SELECT 'Total lots:           ' || COUNT(*) FROM notice_lots;
"
```

**PASS condition:** Notices without lots = 0. Total lots >= 103 (one per notice minimum).

### Check 2: Every notice has a notice-level qualification

```bash
sqlite3 state/state.db "
SELECT 'Notices with qualification:    ' || COUNT(DISTINCT notice_id) FROM qualification_runs WHERE qualification_basis = 'notice_only';
SELECT 'Notices without qualification: ' || COUNT(*) FROM contract_notices cn
  WHERE NOT EXISTS (SELECT 1 FROM qualification_runs qr WHERE qr.notice_id = cn.id AND qr.qualification_basis = 'notice_only');
SELECT 'Total notices:                 ' || COUNT(*) FROM contract_notices;
"
```

**PASS condition:** Notices without qualification = 0.

### Check 3: Every lot has a lot-level qualification

```bash
sqlite3 state/state.db "
SELECT 'Lots with qualification:    ' || COUNT(DISTINCT lot_id) FROM qualification_runs WHERE qualification_basis = 'lot_level';
SELECT 'Lots without qualification: ' || COUNT(*) FROM notice_lots nl
  WHERE NOT EXISTS (SELECT 1 FROM qualification_runs qr WHERE qr.lot_id = nl.id AND qr.qualification_basis = 'lot_level');
SELECT 'Total lots:                 ' || COUNT(*) FROM notice_lots;
"
```

**PASS condition:** Lots without qualification = 0.

### Check 4: Stage heads exist for all Branch A stages

```bash
sqlite3 state/state.db "
SELECT '=== STAGE HEADS ===';
SELECT stage, scope_kind, COUNT(*) FROM process_stage_heads
  WHERE stage IN ('extract lots', 'prequalify notice', 'qualify lot')
  GROUP BY stage, scope_kind ORDER BY stage;
"
```

**PASS condition:** All three stages have entries. `extract lots` and `prequalify notice` have scope_kind='notice'. `qualify lot` has scope_kind='lot'.

### Check 5: Qualification category breakdown

```bash
sqlite3 state/state.db "
SELECT '=== NOTICE-LEVEL CATEGORIES ===';
SELECT legal_category, COUNT(*) as count,
       SUM(CASE WHEN goods_qualified = 1 THEN 1 ELSE 0 END) as goods_pass,
       SUM(CASE WHEN expiry_qualified = 1 THEN 1 ELSE 0 END) as expiry_pass
FROM qualification_runs WHERE qualification_basis = 'notice_only'
GROUP BY legal_category ORDER BY count DESC;
SELECT '';
SELECT '=== QUALIFICATION OUTCOME ===';
SELECT
  CASE
    WHEN expiry_qualified = 0 THEN 'REJECTED: expired'
    WHEN goods_qualified = 1 AND goods_confidence_bps >= 6000 THEN 'PASSED: goods'
    WHEN goods_qualified = 1 THEN 'REVIEW: low confidence goods'
    WHEN legal_category = 'unknown' THEN 'REVIEW: unknown category'
    ELSE 'REJECTED: services/works'
  END as outcome,
  COUNT(*)
FROM qualification_runs WHERE qualification_basis = 'notice_only'
GROUP BY outcome ORDER BY COUNT(*) DESC;
"
```

**PASS condition:** At least 2 different categories appear.

**IMPORTANT DATA MIX CONTEXT:** ~100 of 103 notices are from ProContract (HTML scraping — no OCDS CPV codes). Only ~3 are from FTS/Contracts Finder (OCDS API — has CPV codes). Expect the majority of notices to classify as `"unknown"` because the prequalify stage reads CPV codes from `raw_documents.payload_json` using the OCDS field path `tender.classification.id`, which ProContract notices don't have. This is EXPECTED behavior, not a bug. The FTS/CF notices should show proper goods/services/works classification. If ALL 103 are "unknown" (including FTS/CF), THAT would be a bug.

### Check 6: FK integrity — zero orphans

```bash
sqlite3 state/state.db "
SELECT 'Orphaned qual_runs (lot missing): ' || COUNT(*) FROM qualification_runs qr
  LEFT JOIN notice_lots nl ON nl.id = qr.lot_id WHERE nl.id IS NULL;
SELECT 'Orphaned qual_runs (notice missing): ' || COUNT(*) FROM qualification_runs qr
  LEFT JOIN contract_notices cn ON cn.id = qr.notice_id WHERE cn.id IS NULL;
SELECT 'Orphaned qual_runs (process missing): ' || COUNT(*) FROM qualification_runs qr
  LEFT JOIN procurement_processes pp ON pp.id = qr.process_id WHERE pp.id IS NULL;
"
```

**PASS condition:** All three counts = 0.

### Check 7: Process stage status consistency

```bash
sqlite3 state/state.db "
SELECT '=== ALL STAGE STATUS ===';
SELECT stage, state, COUNT(*) FROM process_stage_status
  GROUP BY stage, state ORDER BY stage, state;
"
```

**PASS condition:** Stages `extract lots`, `prequalify notice`, `qualify lot` all have entries with state = `succeeded`.

---

## PHASE 4: FIX LOOP

If ANY validation check fails:

1. **Identify which check failed** and what the actual vs expected values are
2. **Diagnose the root cause:**
   - Is it a code bug? (Read the relevant function, trace the logic)
   - Is it a data issue? (Missing lots, missing raw_documents, NULL fields)
   - Is it a constraint violation? (FK, NOT NULL, UNIQUE)
3. **Fix the root cause:**
   - If code change needed: edit, `cargo check --all-targets && cargo clippy --all-targets -- -D warnings`, rebuild release
   - If data cleanup needed: write targeted SQL to fix, document what you did
4. **Re-run the affected stages** (may need to reset stage heads first):
   ```bash
   # To re-run a stage for a source, delete its stage heads:
   sqlite3 state/state.db "DELETE FROM process_stage_heads WHERE stage = '{stage_name}';"
   sqlite3 state/state.db "DELETE FROM process_stage_status WHERE stage = '{stage_name}';"
   # Then re-run --pipeline for the affected source(s)
   ```
5. **Re-run ALL validation checks** (not just the one that failed — fixes can cascade)
6. **Repeat until all checks pass**

### Common failure patterns and fixes:

| Failure | Likely Cause | Fix |
|---------|-------------|-----|
| "Notices without lots > 0" | `run_extract_lots_stage` didn't run or skipped some | Check source_id filter, verify the stage runs for all 3 sources |
| "Notices without qualification > 0" | `run_prequalify_stage` query didn't match some notices | Check if those notices have `Normalize` stage head (prerequisite) |
| "Lots without qualification > 0" | `run_qualify_lot_stage` missed some lots | Check if parent notice has `prequalify notice` stage head |
| "Orphaned qual_runs" | FK constraint violation passed somehow | Investigate the orphaned rows, delete them, re-run |
| "Only 'unknown' category" | CPV codes not present in raw OCDS data | Check `raw_documents.payload_json` for those notices — ProContract notices may not have OCDS-format CPV codes |
| Stage errors on execution | DB constraint, missing data, code bug | Read the error message, trace to the function, fix |

---

## DELIVERABLES (when all checks pass)

1. **All 7 validation checks pass** with zero failures
2. **Qualification summary:** how many goods / services / works / unknown, how many passed / rejected / review
3. **Key number:** How many contracts survived as viable physical goods? This is the funnel output.
4. **Stage run output** from all 3 `--pipeline` invocations
5. **Any code fixes made** (with explanation of what was wrong and how you fixed it)
6. **`cargo check --all-targets && cargo clippy --all-targets -- -D warnings`** passes (if any code changes)

### MANDATORY: Write Results Manifest

After ALL checks pass, write a results manifest to `docs/pipeline/branch-a-validation-manifest.md`. This document will be reviewed by a follow-up session. Include:

```markdown
# Branch A Validation Manifest

> Generated: {timestamp}
> Status: {GREEN / YELLOW (with caveats) / RED (blockers remain)}

## Pipeline Run Summary

| Source | lots_created | prequalified | lots_qualified | relations |
|--------|-------------|-------------|----------------|-----------|
| procontract | X | X | X | X |
| contracts-finder | X | X | X | X |
| fts | X | X | X | X |

## Qualification Funnel

| Category | Count | Goods Pass | Expiry Pass |
|----------|-------|------------|-------------|
| goods | X | X | X |
| services | X | X | X |
| works | X | X | X |
| unknown | X | X | X |

### Outcome Breakdown
| Outcome | Count |
|---------|-------|
| PASSED: goods | X |
| REJECTED: expired | X |
| REJECTED: services/works | X |
| REVIEW: unknown | X |
| REVIEW: low confidence | X |

### Key Metric
**Viable physical-goods contracts: X of 103 total (X%)**

## Validation Checks

| # | Check | Result | Detail |
|---|-------|--------|--------|
| 1 | Every notice has ≥1 lot | PASS/FAIL | {count} |
| 2 | Every notice has qualification | PASS/FAIL | {count} |
| 3 | Every lot has qualification | PASS/FAIL | {count} |
| 4 | Stage heads exist | PASS/FAIL | {stages found} |
| 5 | Categories sensible | PASS/FAIL | {breakdown} |
| 6 | FK integrity | PASS/FAIL | {orphan counts} |
| 7 | Stage status consistent | PASS/FAIL | {counts} |

## Issues Found & Fixed

{For each issue: what failed, root cause, fix applied, verification}

## Issues Remaining (if any)

{Anything that couldn't be resolved, with explanation}

## Data Snapshot

{Paste the full output of the validation queries here for the reviewer}
```

**This manifest is non-negotiable.** Do not skip it. The follow-up session will use it to decide whether to proceed to Stage 7+ and Branch B.

---

## CONSTRAINTS

1. **Work in a loop until green.** Do NOT report "done" with failing checks. Fix and re-run.
2. **Run for ALL THREE sources:** procontract, contracts-finder, fts.
3. **Do NOT modify qualification logic** (CPV ranges, thresholds) unless it's clearly buggy. Report unexpected results rather than changing the rules.
4. **If the Relate stage runs** (it will, since `--pipeline` calls it), that's fine. Don't worry about validating it — just let it run. Branch B validation is separate.
5. **Rust conventions:** `cargo check --all-targets && cargo clippy --all-targets -- -D warnings` must pass if any code changes are made.

---

## REMINDERS

- ULTRATHINK and be meticulous.
- **The loop is non-negotiable.** Run → validate → fix → repeat. Don't stop at the first error.
- **~97% of notices are ProContract** (HTML-scraped, no CPV codes → will classify as `"unknown"`). Only ~3% are FTS/CF (OCDS API, has CPV). So expect the qualification funnel to show mostly "unknown" with a small number of properly classified contracts. This is expected given our current data, not a bug.
- The key question this branch answers: **How many of our 103 contracts are viable physical-goods opportunities?** This number determines the size of the funnel for Stages 7-12.
- Record the raw numbers. They inform everything we build next.
