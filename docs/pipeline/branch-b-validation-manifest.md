# Branch B Validation Manifest

> Generated: 2026-04-26 (post full-pipeline rerun)
> Status: **GREEN**

## Relate Stage Summary

| Metric | Value |
|--------|------:|
| Total processes | 133 |
| Processes with relate head | 133 |
| Processes without relate head | 0 |
| Total process_relations | 0 |
| Duplicate groups found | 0 |
| Unique OCIDs | 133 |
| NULL/empty OCIDs | 0 |

All 133 ProContract processes have unique `pc-{advertId}` OCIDs, so zero relations is the **expected correct result**. The OCID-only matcher in `run_relate_stage()` had nothing to deduplicate.

## Validation Checks

| # | Check | Result | Detail |
|---|-------|--------|--------|
| 1 | Every process has relate head | **PASS** | heads=133, missing=0, total=133 |
| 2 | Relation results consistent | **PASS** | total_relations=0 (no OCID duplicates exist to merge) |
| 3 | Stage status consistent | **PASS** | `relate \| succeeded \| 133` |
| 4 | FK integrity | **PASS** | 0 orphaned process_a, 0 orphaned process_b |
| 5 | OCID uniqueness correct | **PASS** | 133 unique OCIDs / 133 processes, 0 NULL, 0 duplicate groups |

## Current Limitations

- Only **OCID-exact matching** is implemented in the daemon (`crates/daemon/src/main.rs:11351` — `run_relate_stage`). Processes sharing a `canonical_ocid` are merged with `relation_type='merged_same_process'` and `confidence_bps=10000`.
- The **full heuristic engine** (reference match, buyer match, title similarity, CPV overlap, date proximity, value proximity, weighted 0.95 / 0.75 thresholds, hard blockers) lives in TypeScript at `packages/intelligence/src/relations/resolve-process-relations.ts` but is **NOT wired into the daemon**. Wiring it in is future work.
- `process_aliases` is not populated by the current implementation — the OCID matcher only creates edges in `process_relations`, it does not merge process records.
- With **single-source data (ProContract only)**, zero duplicates is the expected correct result. Branch B becomes meaningful when FTS and Contracts Finder data is also ingested and overlapping procurement processes appear across sources.

## Issues Found & Fixed

None — results correct on re-validation against the post-rerun DB (133 processes). No code changes required. Counts updated from the previous validation (103 processes) to reflect the current live state.

## Data Snapshot

### Check 1 — Relate stage head coverage

```
Processes with relate head:    133
Processes without relate head: 0
Total processes:               133
```

### Check 2 — Relation summary

```
=== RELATION SUMMARY ===
total_relations: 0

=== DUPLICATE GROUPS (if any) ===
(none)
```

### Check 3 — Stage status

```
relate | succeeded | 133
```

### Check 4 — FK integrity

```
Orphaned relations (process_a missing): 0
Orphaned relations (process_b missing): 0
```

### Check 5 — OCID distribution

```
=== OCID DUPLICATES ===
(none — no canonical_ocid has COUNT(*) > 1)

Unique OCIDs:    133
Total processes: 133
NULL/empty OCIDs: 0
```
