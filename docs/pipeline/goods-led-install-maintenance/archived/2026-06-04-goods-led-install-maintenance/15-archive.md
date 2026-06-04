# Goods-Led Install/Maintenance — Archive Pipeline

## Your Mission

The goods-led install/maintenance implementation pipeline is complete. Archive all pipeline artifacts and clean up for the next run. This matches the project's established archival pattern (see `docs/pipeline/l6-request-pipeline-automation/archived/`).

---

## Steps

### 1. Verify Completion
- [ ] All `goods-led-install` beads are closed (`br list --label goods-led-install --status closed --json`); none left open or in_progress. (The 3 Tier-B `research-spike` beads count as closed once their research/design output is committed — even if that output's conclusion is "implement via a separate future pipeline." Their being in-scope here means the *research* is done, not that full behaviour shipped. Do NOT block archive waiting for Tier-B implementation, and do NOT leave spike beads open.)
- [ ] Completion audit passed (COMPLETE verdict in the audit report).
- [ ] All `audit-fix` fix beads executed and verified.
- [ ] Full AGENTS.md stage gate green:
  ```bash
  cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings \
    && cargo test -p daemon --bin daemon \
    && for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done \
    && bun test
  ```
- [ ] The Scenario-1 NW band hard-assert passes (`installation_labour_engine.rs`).
- [ ] Schema-epoch is consistently 10 across all 4 constants + Supabase; desktop reads return no `{code,message}` envelopes.

### 2. Archive Pipeline Handoffs
```bash
DATE=$(date +%Y-%m-%d)   # or pass the date explicitly
mkdir -p docs/pipeline/goods-led-install-maintenance/archived/${DATE}-goods-led-install-maintenance
mv docs/pipeline/goods-led-install-maintenance/[0-9][0-9]-*.md \
   docs/pipeline/goods-led-install-maintenance/archived/${DATE}-goods-led-install-maintenance/
mv docs/pipeline/goods-led-install-maintenance/EXECUTION-GUIDE.md \
   docs/pipeline/goods-led-install-maintenance/archived/${DATE}-goods-led-install-maintenance/
cp docs/pipeline/goods-led-install-maintenance/manifest.yaml \
   docs/pipeline/goods-led-install-maintenance/archived/${DATE}-goods-led-install-maintenance/
```
Keep the `artifacts/` directory in place (spec/TDD/strategy/kickoff/push/status remain the live reference, mirroring how L6's artifacts stayed put).

### 3. Update Status
Update `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-status.yaml`:
```yaml
feature: goods-led-install-maintenance
status: complete
completed: "<DATE>"
spec: docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md
tdd: docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md
strategy: docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-strategy.md
implementation_epic: "<epic-id>"
bead_label: goods-led-install
total_beads: <N>
fix_beads: <N>
audit_iterations: <N>
schema_epoch: 10
```
Then `bin/br-sync` and commit the `.beads/` + status changes.

### 4. Report Deliverables
```
✅ Goods-Led Install/Maintenance Expansion + UK Labour-Cost Engine — COMPLETE

📦 Deliverables:
  - Spec / TDD / Strategy in docs/pipeline/goods-led-install-maintenance/artifacts/
  - Filter widening: two-phase qualification + goods_led_mixed service gate + no-list/bespoke flag
  - UK labour-cost engine: uk-installation-labour-estimate skill + 4 reference tables + overlay-labour-cost
  - Rollup: labour/permit fields summed at both sites; goods_value_share_bps dominance gate
  - Recurring maintenance: MaintenancePeriodEstimate per-period margin check
  - Validation: 11 gold-set scenarios + 11 E2E manifests + 7-case canary corpus
  - Schema epoch 9 -> 10 across all 4 constants + Supabase projection
  - Implementation: <N> beads; <N> fix beads; <N> audit iterations to 100%

📁 Archived: docs/pipeline/goods-led-install-maintenance/archived/<DATE>-goods-led-install-maintenance/
🔄 Ready for next pipeline run.
```

### 5. Clean Up
- Verify git is clean (`git status`).
- Update project memory if any durable decisions were made during execution (e.g. final `GOODS_DOMINANCE_FLOOR_BPS` value the operator settled on).

---

## Next Pipeline Run
```
/pipemaxxer <next_source_doc_path>
```
The pipeline is stateless — each run generates fresh handoffs. Prior artifacts are archived, not overwritten.
