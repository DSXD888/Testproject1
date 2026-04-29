# Validate Branch B: Relation Resolution (Stage 3 — Relate)

ULTRATHINK and be meticulous.

## FRESH-EYES CURRENT-STATE OVERRIDE (2026-04-26)

This validation handoff predates the full rerun. The earlier manifest validated 103 processes; current DB has 133 processes, 133 `relate` heads, and 0 `process_relations`.

Use this handoff as the relation-resolution validation checklist, but update counts from the live DB before judging pass/fail. With ProContract-only data and unique `canonical_ocid` values, zero relation rows can still be the correct result.

## MISSION

Validate that **Stage 3: Relate** (relation resolution / cross-source deduplication) is working correctly against the current live DB. This stage has already been executed by `run_deterministic_pipeline()`, which calls `run_relate_stage()` as its final step. Your job is to validate current results, verify that zero relations is still correct for ProContract-only unique OCIDs, fix only real defects, and produce or update a manifest.

If any validation check fails, diagnose, fix, and re-run until green.

```
┌─────────────────────────────────────────────────────────┐
│                    THE LOOP                               │
│                                                          │
│   1. Validate existing results (queries below)           │
│   2. Any failures?                                       │
│      ├─ YES → Diagnose, fix, re-run relate stage,       │
│      │        go back to step 1                          │
│      └─ NO  → Branch B is GREEN. Write manifest.         │
│                                                          │
│   DO NOT STOP UNTIL STEP 2 = NO                          │
└─────────────────────────────────────────────────────────┘
```

---

## WHAT STAGE 3 (RELATE) DOES

Relation resolution detects duplicate or related procurement processes across data sources. The same tender can appear on multiple portals (FTS, Contracts Finder, ProContract). Without dedup, downstream stages would analyze the same opportunity twice.

### Current Implementation (OCID matching)

The daemon's `run_relate_stage()` in `crates/daemon/src/main.rs`:

1. Queries all processes WITHOUT a `relate` stage head (no source filter — runs globally)
2. Groups processes by `canonical_ocid`
3. If 2+ processes share the same OCID → creates `process_relations` entry:
   - `relation_type = 'merged_same_process'`
   - `confidence_bps = 10000` (100% confidence)
   - `evidence = {"rule": "same_ocid", "ocid": "..."}`
4. Marks ALL processes (including singletons with unique OCIDs) as `relate` stage completed

```bash
# Find the function:
grep -n "fn run_relate_stage" crates/daemon/src/main.rs
```

### What It Does NOT Do (L3 full heuristic — future work)

The TypeScript engine at `packages/intelligence/src/relations/resolve-process-relations.ts` has a richer resolution engine that is NOT yet wired into the daemon:

- **Exact rules:** same OCID, linked notice chains, framework call-offs, planning→tender pairs
- **6 heuristic features** with weighted scoring:
  - Reference match (0.35 weight)
  - Buyer match (0.2)
  - Title similarity (0.2)
  - CPV overlap (0.1)
  - Date proximity (0.1)
  - Value proximity (0.05)
- **Thresholds:** >=0.95 → merge, 0.75-0.94 → related, <0.75 → separate
- **Hard blockers:** conflicting references, buyer mismatch, incompatible chronology

Wiring this full engine into the daemon is a future task. For now, OCID-only matching is the implemented behavior.

### Expected Results for Our Current Data

**All 103 notices are from ProContract only** (zero FTS, zero Contracts Finder). Each ProContract notice has a unique `canonical_ocid` of the form `pc-{advertId}`. With a single source and unique IDs, there should be **zero duplicates found**. This is correct — there's nothing to deduplicate.

Branch B becomes meaningful when multiple sources ingest overlapping procurement data.

---

## KEY CONCEPTS

- **`process_relations`** — stores edges between related processes. Each row has `process_id_a`, `process_id_b`, `relation_type` (merged_same_process / related_not_merged / possible_duplicate / separate), `confidence_bps`, and `evidence` (JSON with the matching rule).
- **`process_stage_heads`** with `stage = 'relate'` — one row per process, marks it as having been through relation resolution. Stage is idempotent — re-running skips processes that already have a head.
- **`process_aliases`** — would store canonical process ID redirects for merged duplicates. Not populated by the current OCID-only implementation (it creates relation edges but doesn't merge process records).

**Read AGENTS.md** — single source of truth for agent instructions.

---

## PHASE 1: VALIDATE EXISTING RESULTS

The relate stage already ran during Branch A. Run these checks:

### Check 1: Every process has a relate stage head

```bash
sqlite3 state/state.db "
SELECT 'Processes with relate head:    ' || COUNT(*) FROM process_stage_heads
  WHERE stage = 'relate' AND scope_kind = 'process';
SELECT 'Processes without relate head: ' || COUNT(*) FROM procurement_processes p
  WHERE NOT EXISTS (SELECT 1 FROM process_stage_heads h
    WHERE h.process_id = p.id AND h.stage = 'relate' AND h.scope_kind = 'process');
SELECT 'Total processes:               ' || COUNT(*) FROM procurement_processes;
"
```

**PASS condition:** Processes without relate head = 0. Processes with relate head = total processes.

### Check 2: Relation results are consistent

```bash
sqlite3 state/state.db "
SELECT '=== RELATION SUMMARY ===';
SELECT COUNT(*) as total_relations FROM process_relations;
SELECT relation_type, COUNT(*) FROM process_relations GROUP BY relation_type;
SELECT '';
SELECT '=== DUPLICATE GROUPS (if any) ===';
SELECT process_id_a, process_id_b, confidence_bps, evidence
FROM process_relations WHERE relation_type = 'merged_same_process' LIMIT 10;
"
```

**PASS condition:** For our current single-source data, `total_relations = 0` is correct. If relations DO exist, verify they make sense (same OCID, 10000 confidence).

### Check 3: Stage status consistency

```bash
sqlite3 state/state.db "
SELECT stage, state, COUNT(*) FROM process_stage_status
  WHERE stage = 'relate' GROUP BY stage, state;
"
```

**PASS condition:** `relate | succeeded | {count matching total processes}`.

### Check 4: FK integrity

```bash
sqlite3 state/state.db "
SELECT 'Orphaned relations (process_a missing): ' || COUNT(*) FROM process_relations pr
  LEFT JOIN procurement_processes pa ON pa.id = pr.process_id_a WHERE pa.id IS NULL;
SELECT 'Orphaned relations (process_b missing): ' || COUNT(*) FROM process_relations pr
  LEFT JOIN procurement_processes pb ON pb.id = pr.process_id_b WHERE pb.id IS NULL;
"
```

**PASS condition:** Both counts = 0.

### Check 5: OCID uniqueness verification

```bash
sqlite3 state/state.db "
SELECT '=== OCID DISTRIBUTION ===';
SELECT canonical_ocid, COUNT(*) as cnt FROM procurement_processes
  GROUP BY canonical_ocid HAVING cnt > 1 ORDER BY cnt DESC LIMIT 10;
SELECT '';
SELECT 'Unique OCIDs: ' || COUNT(DISTINCT canonical_ocid) FROM procurement_processes;
SELECT 'Total processes: ' || COUNT(*) FROM procurement_processes;
SELECT 'NULL OCIDs: ' || COUNT(*) FROM procurement_processes WHERE canonical_ocid IS NULL OR canonical_ocid = '';
"
```

**PASS condition:** No OCID groups with cnt > 1 (confirms zero duplicates is correct). If duplicates ARE found, verify they have corresponding `process_relations` entries.

---

## PHASE 2: FIX LOOP (if any check fails)

If any check fails:
1. Diagnose whether it's a code bug, data issue, or missing stage execution
2. To re-run the relate stage:
   ```bash
   sqlite3 state/state.db "
   DELETE FROM process_stage_heads WHERE stage = 'relate';
   DELETE FROM process_stage_status WHERE stage = 'relate';
   DELETE FROM process_relations;
   "
   cargo run --release --bin daemon -- --pipeline procontract \
     --state-root state --runtime-dir runtime --artifact-root artifacts
   ```
3. Re-validate all checks
4. Repeat until green

---

## DELIVERABLES

### MANDATORY: Write Results Manifest

After ALL checks pass, write a results manifest to `docs/pipeline/branch-b-validation-manifest.md`:

```markdown
# Branch B Validation Manifest

> Generated: {timestamp}
> Status: {GREEN / YELLOW / RED}

## Relate Stage Summary

| Metric | Value |
|--------|-------|
| Total processes | X |
| Processes with relate head | X |
| Processes without relate head | X |
| Total process_relations | X |
| Duplicate groups found | X |
| Unique OCIDs | X |
| NULL/empty OCIDs | X |

## Validation Checks

| # | Check | Result | Detail |
|---|-------|--------|--------|
| 1 | Every process has relate head | PASS/FAIL | {counts} |
| 2 | Relation results consistent | PASS/FAIL | {relation count + types} |
| 3 | Stage status consistent | PASS/FAIL | {state counts} |
| 4 | FK integrity | PASS/FAIL | {orphan counts} |
| 5 | OCID uniqueness correct | PASS/FAIL | {duplicate groups} |

## Current Limitations

- Only OCID-exact matching is implemented (daemon-side)
- Full heuristic engine (title similarity, buyer match, CPV overlap, etc.) exists in TypeScript (`packages/intelligence/src/relations/resolve-process-relations.ts`) but is NOT wired into the daemon
- With single-source data (ProContract only), zero duplicates is the expected correct result
- Branch B becomes meaningful when FTS and Contracts Finder data is also present

## Issues Found & Fixed

{Any issues, or "None — results correct on first validation"}

## Data Snapshot

{Paste full output of all 5 validation queries}
```

---

## CONSTRAINTS

1. **Work in a loop until green.** Do NOT report "done" with failing checks.
2. **Do NOT modify the relate logic.** Validate the existing OCID-matching behavior as-is.
3. **Zero duplicates is the expected correct result** for single-source ProContract data. Do not flag this as a failure.
4. **Rust conventions:** `cargo check --all-targets && cargo clippy --all-targets -- -D warnings` if any code changes.

---

## REMINDERS

- ULTRATHINK and be meticulous.
- The relate stage ALREADY RAN during Branch A. You are validating, not running from scratch.
- With 103 ProContract-only notices and unique `pc-{advertId}` OCIDs, zero relations is correct.
- The key question: **Are all processes accounted for in the relate stage, and are there any unexpected duplicates?**
- If you find duplicate OCIDs that DON'T have corresponding `process_relations` entries, that's a bug in `run_relate_stage`.
