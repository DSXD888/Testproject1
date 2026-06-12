# Autonomous Daemon Scheduling — Step 15: Archive Pipeline

## Your Mission

The autonomous daemon scheduling implementation pipeline is complete. Archive all pipeline artifacts and clean up for the next run.

---

## Steps

### 1. Verify Completion
- [ ] All beads closed: `br list --label autonomous-scheduling --status closed --json` (0 open, 0 in_progress)
- [ ] Completion audit passed (COMPLETE verdict, step 12)
- [ ] All fix beads (`--label audit-fix`) executed and verified
- [ ] The FULL Stage Validation Test Gate is green:
  ```bash
  cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings
  cargo test -p daemon --bin daemon
  for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
  ```
- [ ] If Q4=table: epoch is 11 across ALL constants (manifest `schema_epoch_constants`) + `migrations/0015_*` applied + `bun run build` green (desktop reads work)
- [ ] If Q7 taken: `bun run build && bun run test && bun run lint` green

### 2. Archive Pipeline Handoffs
```bash
mkdir -p docs/pipeline/autonomous-daemon-scheduling/archived/2026-06-DD-autonomous-daemon-scheduling
# Move the numbered handoff prompts (NOT the artifacts/ SOT docs — those stay as the record)
mv docs/pipeline/autonomous-daemon-scheduling/0*.md docs/pipeline/autonomous-daemon-scheduling/1*.md \
   docs/pipeline/autonomous-daemon-scheduling/archived/2026-06-DD-autonomous-daemon-scheduling/ 2>/dev/null
# Keep manifest.yaml + EXECUTION-GUIDE.md alongside the archive (copy for the record)
cp docs/pipeline/autonomous-daemon-scheduling/manifest.yaml \
   docs/pipeline/autonomous-daemon-scheduling/archived/2026-06-DD-autonomous-daemon-scheduling/
```
> Per AGENTS.md RULE 1 (no file deletion): MOVE, never delete. The `artifacts/` SOT docs (spec/tdd/strategy/kickoff/etc.) remain in place as the permanent record (matches the goods-led archive shape).

### 3. Update Status File
Update `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-status.yaml`:
```yaml
feature: autonomous-daemon-scheduling
status: complete
completed: "2026-06-DD"
spec: docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md
tdd: docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md
strategy: docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-strategy.md
architecture: docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-architecture.md
bootstrap: docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-bootstrap.md
completion_audit: docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-completion-audit-2026-06-DD.md
implementation_epic: "<epic-id from step 07>"
bead_label: autonomous-scheduling
total_beads: <N>
fix_beads: <N>
audit_iterations: <N>
schema_epoch: <10 or 11 — 11 only if Q4 picked the dedicated procontract_pending_enrichment table>
resolved_open_questions:   # record how Q1/Q4/Q5/Q6/Q7/Q8/Q9 were decided at spec time
  Q1_worker_exec_context: "<decision>"
  Q4_awaiting_access_modeling: "<table | state-string>"
  Q5_staleness_policy: "<decision>"
  Q6_shadow_flag: "<decision>"
  Q7_desktop_ux: "<taken | not taken>"
  Q8_chained_nudge: "<decision>"
  Q9_registry_reload: "<generation-counter | poll-updated_at>"
gates:
  beads: "<N> closed under label; 0 open, 0 in_progress"
  completion_audit: "COMPLETE"
  stage_gate: "green — fmt/check/clippy + --bin daemon + all integration targets (incl. scheduler_tick.rs / worker_pump.rs / procontract_resumption.rs)"
  autonomy_e2e: "M1 (G1+G2), M2 (G6, no daemon stop), M3 (G4 lifecycle), M4 (manual-path + live button), M5 (concurrency soak) all green"
archived_to: docs/pipeline/autonomous-daemon-scheduling/archived/2026-06-DD-autonomous-daemon-scheduling/
```

### 4. Update Project Memory
Add/refresh a memory note: autonomous daemon scheduling COMPLETE — daemon now fires schedules autonomously (tick thread), drains cron kinds via the two-lane worker, ProContract resumption hardened, commercial stages auto-advance. Update the existing `project_autonomous_scheduling.md` memory (research → implemented). Note final schema epoch and whether Q4 added migration 0015.

### 5. Report Deliverables
```
✅ Autonomous Daemon Scheduling & ProContract Resumption — COMPLETE

📦 Deliverables:
  - Spec / TDD / Strategy / Architecture / Bootstrap (docs/pipeline/autonomous-daemon-scheduling/artifacts/)
  - Implementation: <N> beads executed (label autonomous-scheduling)
  - Fix beads: <N> created and resolved
  - Audit: <N> iterations to 100%
  - Tests: scheduler_tick.rs, worker_pump.rs, procontract_resumption.rs (net-new) + M1–M5 E2E
  - Schema epoch: <10/11>

📁 Archived: docs/pipeline/autonomous-daemon-scheduling/archived/2026-06-DD-autonomous-daemon-scheduling/
🔄 Ready for next pipeline run.
```

### 6. Clean Up
- Verify git is clean (`git status`) and `.beads/` is synced (`bin/br-sync`)
- Remove any temporary working files (MOVE to archive if unsure — never delete per Rule 1)

---

## Next Pipeline Run
```
/pipemaxxer <next_source_doc_path>
```
The pipeline is stateless — each run generates fresh handoffs. Prior artifacts are archived, not overwritten.
