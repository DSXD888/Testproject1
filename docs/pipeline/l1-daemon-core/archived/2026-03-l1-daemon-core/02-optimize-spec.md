# L1 Daemon Core — Optimize Spec (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the Layer 1 spec after requirement IDs and acceptance criteria were added in Step 01. Your job is to find gaps, inconsistencies, and ambiguities that would cause implementation or TDD problems downstream.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)

```
docs/strategy/layers/layer-1-thin-local-runner.md   — READ FULLY (validated spec from Step 01)
```

### Verify Against (SOURCE OF TRUTH)

```
docs/strategy/layer-strategy.md                      — Cross-layer contracts (§9-§11)
docs/strategy/layers/layer-0-gold-set-evaluation.md  — L0 DDL (§2), skill standard (§4), pipeline (§3)
```

### Quality Bar

```
docs/strategy/layers/layer-0-gold-set-evaluation.md  — Best existing layer spec (~4,170 lines, deeply structured)
```

---

## Think Independently

Don't just check for obvious errors. Think critically about these 8 areas:

### 1. Adapter Trait Completeness

The `AgentAdapter` trait (§2.3) defines 4 methods + `AdapterParser` sub-trait. Does the trait design account for:
- Adapter lifecycle (initialization, teardown) — or is stateless assumed?
- Concurrent usage — can one adapter instance serve multiple sessions simultaneously?
- Version migration — what happens when a CLI updates mid-daemon-lifetime?
- Error propagation from `probe()` — does the runner handle probe failures gracefully?
- `parser()` ownership model — `Box<dyn AdapterParser>` is per-session. Is creation cheap?

### 2. PTY/Contract Mode Boundary

The two-lane event model splits `OutputFrame` and `ControlEvent`. Verify:
- In PTY mode, who derives `ControlEvent` from raw PTY bytes? (Answer should be: the `AdapterParser`)
- In contract mode with `--json` output, does structured JSON go through `Stdout(Bytes)` AND get parsed into `ControlEvent`? Or only one lane?
- What happens when contract mode produces stderr alongside structured stdout?
- Is there a requirement for event ordering between the two lanes?

### 3. Session Phase Machine Gaps

The 7-phase lifecycle (§2.4) has a transition table. Audit for:
- Can a session transition from `spawned` directly to `exited` (agent binary crashes immediately)?
- Can a session transition from `created` directly to `archived` (spawn failure)?
- Is there a timeout on the `terminating` → `exited` transition?
- What's the maximum time a session can spend in `exited` before moving to `reaped`?
- What happens if the reaper crashes during `reaped` → `archived`?

### 4. Commit Fence Correctness

The `commit_seq` / `visibleAfterSeq` model (§2.6) provides read-your-writes. Verify:
- Is `commit_seq` a global monotonic counter across ALL write transactions, or per-table?
- What happens if two write transactions commit in rapid succession — is ordering guaranteed?
- Does `commit_clock` table access become a contention bottleneck under concurrent writes? (L1 is single-threaded, but L3 adds concurrency)
- Is there a requirement for what `visibleAfterSeq` returns on a failed write?

### 5. Artifact Store Crash Safety

Chunked capture (§2.5) uses temp-file-then-rename for crash safety. Verify:
- What happens if the daemon crashes mid-chunk? Is there a recovery procedure?
- Is `stream.index.jsonl` updated atomically with the chunk file? (Append is NOT atomic on crash)
- What happens to in-flight chunks if the PTY session exits abnormally?
- Is there a maximum chunk count per session? What if a session runs for hours?

### 6. Error Type Completeness

§1.9 and §4.3 define error types. Cross-reference:
- Are all 12 error types from §1.9 represented in §4.3?
- Are the 5 source/skill errors from §4.3 represented in §1.9?
- Is there an error type for "skill directory invalid" (detected by skill linting)?
- Is there an error type for "SQLite write failure" (disk full, corruption)?
- Is there an error type for "daemon startup failure" (lock contention, schema mismatch)?

### 7. Stage Tracking Atomicity

§3.3 describes stage tracking with `stage_run_id`. Verify:
- For set-replacement stages, is "rollback on partial failure" specified? (The spec says don't advance head, but are partial rows deleted or left as orphans?)
- For additive stages, is there a mechanism to know which scope units succeeded and which failed within one agent session?
- Is `stage_run_id` a UUID generated before the session starts, or after it completes?
- What happens to `process_stage_status` rows when a stage is re-run? (New row with new `stage_run_id`, or update existing?)

### 8. Forward-Compatibility Tensions

The spec has extensive forward-compatibility provisions (§6.3). Verify these don't create implementation confusion:
- Are nullable columns that "exist but are unused" clearly marked so L1 implementers don't accidentally populate them?
- Is the `parent_job_id` field documented well enough that L1 implementers know to ignore it?
- Is the `validation_required = false` default documented as a constitutional invariant for L1?

---

## Drift Checklist

- [ ] Every section in the original spec is still present (no accidental deletion during Step 01)
- [ ] Requirement IDs are consistent with the subsystem they reference
- [ ] No acceptance criteria contradict each other
- [ ] No acceptance criteria contradict the layer-strategy.md contracts
- [ ] All 13 Definition of Done criteria (§5.1) map to at least one requirement ID
- [ ] All 10 test files (§5.2) map to the requirements they cover
- [ ] The Rust trait code in §2.3 is consistent with the prose requirements around it
- [ ] Stubs (§1.10, §6.2) don't accidentally describe L3+ behavior as L1 requirements

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

ULTRATHINK and re-read the fixes you just made:

1. Did adding error paths for edge cases accidentally expand L1's scope beyond "thin runner"?
2. Count the requirement IDs — are there more than 120? If so, some may be too granular (implementation detail, not requirement).
3. Open §2.3 — did your audit introduce any new trait methods? If so, verify they don't break the `LaunchSpec`-return design principle.
4. Open §3.3 — did your stage tracking audit introduce any new scope_kind values beyond the 6 canonical ones?
5. Read the Definition of Done (§5.1) — after your changes, are all 13 items still achievable in L1 scope?
