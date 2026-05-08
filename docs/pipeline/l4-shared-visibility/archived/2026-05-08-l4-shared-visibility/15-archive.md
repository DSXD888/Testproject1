# L4 Shared Visibility — 15 · Archive Pipeline

## Your Mission

The L4 Shared Visibility implementation pipeline is complete. Archive all pipeline artifacts and clean up for the next pipeline run (which will likely be L5 — Expanded Commercial Intelligence).

This is a manual maintenance step — no skill is invoked.

---

## Pre-Archive Verification

Before archiving, verify completion is genuine. Do NOT archive a pipeline that has unfinished work.

### 1. Beads Closed

```bash
br list --label layer-4 --status=open --json     # MUST be empty (or only legitimate-deferral items)
br list --label layer-4 --status=in_progress --json  # MUST be empty
br list --label layer-4 --label audit-fix --status=open --json   # MUST be empty (all fix beads closed)
```

### 2. Completion Audit Verdict

The most recent completion audit (step 12) must have returned **COMPLETE** verdict. If the last audit was GAPS DETECTED or BLOCKING GAPS, do NOT archive — go back to step 13/14 (fix bead execution) until the next audit returns COMPLETE.

### 3. Build + Test Gate Green

Run the full Stage Validation Test Gate one final time:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
bun run test
bun run lint
bun run build
(cd apps/web && bun run test:e2e)
```

All commands must pass with zero warnings.

### 4. DoD Walkthrough

Walk every of the 18 DoD criteria from spec §4.1:

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Push protocol: transactional outbox entries created for all push-owned mutations in same SQLite transaction | ✓ |
| 2 | Push batches applied to Supabase via single transactional RPC with schema_epoch, row_version ordering | ✓ |
| 3 | Pull protocol: cursor-based pull for shared_source_configs, shared_company_profile, operator_decision_heads | ✓ |
| 4 | Operator decisions: queued local intents flush through Supabase CAS RPC with base_remote_revision; conflicts returned (not silently overwritten) | ✓ |
| 5 | Active-writer lease: second daemon instance detected and rejected | ✓ |
| 6 | RLS policies enforced for all 4 roles (viewer, operator, admin, daemon) | ✓ |
| 7 | Web viewing portal displays opportunities, manifests, pipeline status (read-only) | ✓ |
| 8 | Supabase Broadcast invalidation delivers live updates to web portal | ✓ |
| 9 | viewer_sync_status shows sync freshness (green/amber/red) | ✓ |
| 10 | Validation enforcement mode: validator rejections quarantine results as AGENT_SUSPECT_OUTPUT | ✓ |
| 11 | 4 domain-specific validator overlays deployed and producing verdicts | ✓ |
| 12 | Two users can view the same pipeline: desktop (read-write) + web (read-only) | ✓ (manual) |
| 13 | Writer fencing: stale daemon with old `writer_epoch` rejected on push | ✓ |
| 14 | CAS command ledger: retried command returns stored result (idempotent replay) | ✓ |
| 15 | Provisional promotion buffer: enforcement-active slices hold results until validation completes | ✓ |
| 16 | Equal-version retry: identical-envelope retry with matching content_hash = no-op success | ✓ |
| 17 | Web portal uses viewer-scoped credentials only (security boundary, not UI convention) | ✓ |
| 18 | Redaction tests pass: no leakage of operator identity/company profile/raw supplier data in summary projections | ✓ |

If any row is ✗ or ⚠, do NOT archive — return to step 12.

### 5. Two-User Verification (Manual)

DoD #12 is a manual verification:
- Run desktop on machine A with operator credentials → make a decision
- Within ~90s, run web on machine B with viewer credentials → see the decision
- Verify viewer cannot mutate (try to click a decision control — should fail or be absent)
- Kill the daemon on machine A → web shows offline (red badge) within 5min
- Restart daemon → web returns to healthy (green) within sync cycle

Record outcome in archive.

### 6. Git State Clean

```bash
git status              # MUST show "up to date with origin" + "nothing to commit, working tree clean"
git log -5              # Recent commits should reflect L4 closure
```

---

## Archive Steps

### 1. Confirm Pipeline Completion

```bash
echo "L4 Shared Visibility — pipeline completion confirmed: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
br list --label layer-4 --json | jq '[.issues | length, ([.issues[] | select(.status=="closed")] | length), ([.issues[] | select(.status=="open")] | length)] | "Total: \(.[0]), Closed: \(.[1]), Open: \(.[2])"'
```

Expected: total = closed; open = 0.

### 2. Create Archive Directory

```bash
ARCHIVE_DATE=$(date -u +%Y-%m-%d)
ARCHIVE_DIR="docs/pipeline/l4-shared-visibility/archived/${ARCHIVE_DATE}-l4-shared-visibility"
mkdir -p "$ARCHIVE_DIR"
echo "Archive dir: $ARCHIVE_DIR"
```

### 3. Move Pipeline Handoffs to Archive

```bash
# Move all handoff files (01-15)
for f in 01-validate-spec 02-optimize-spec 03-create-tdd 04-optimize-tdd 05-create-strategy 06-optimize-strategy 07-create-beads 08-optimize-beads 09-audit-beads 10-generate-kickoff 11-push 12-completion-audit 13-fix-beads-kickoff 14-fix-beads-push 15-archive; do
  if [ -f "docs/pipeline/l4-shared-visibility/${f}.md" ]; then
    git mv "docs/pipeline/l4-shared-visibility/${f}.md" "$ARCHIVE_DIR/${f}.md"
  fi
done

# Move EXECUTION-GUIDE and manifest
git mv "docs/pipeline/l4-shared-visibility/EXECUTION-GUIDE.md" "$ARCHIVE_DIR/EXECUTION-GUIDE.md"
git mv "docs/pipeline/l4-shared-visibility/manifest.yaml" "$ARCHIVE_DIR/manifest.yaml"

# Move completion audit report (if exists)
for report in docs/pipeline/l4-shared-visibility/completion-audit-report-*.md; do
  if [ -f "$report" ]; then
    git mv "$report" "$ARCHIVE_DIR/"
  fi
done
```

**The `artifacts/` directory STAYS in place.** It contains the source-of-truth documents (spec, TDD, strategy, kickoff, push, status) that future runs and downstream layers reference. Do NOT move artifacts.

### 4. Update Status File

The status file at `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-status.yaml` stays in place but updates from `in-progress` to `complete`:

```yaml
feature: l4-shared-visibility
status: complete
started: "<date>"     # populate from actual start
completed: "<date>"   # set to today
spec: docs/strategy/layers/layer-4-shared-visibility.md
tdd: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md
strategy: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md
bootstrap: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md
bulk_import: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md
bead_prefix: l4
bead_cli: br
bead_sync_cmd: bin/br-sync
existing_epic: testproject1-4zq
total_beads: <N>           # populate from final br list count
task_beads: <N>
fix_beads: <N>
audit_iterations: <N>      # how many step 12 cycles to reach COMPLETE
phases: 6
agentmail_key: /Users/alinaaitova/Desktop/Testproject1

source_of_truth:
  spec:
    path: docs/strategy/layers/layer-4-shared-visibility.md
    role: "SOT #1 — WHAT to build (L4-FR-* requirement IDs, 18 DoD criteria, 14 invariants)"
  tdd:
    path: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md
    role: "SOT #2 — HOW to verify (L4-T-* test IDs, 4 test files, ~52 cumulative)"
  strategy:
    path: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md
    role: "SOT #3 — WHAT ORDER (6 phases, dependency DAG, gate criteria)"
  bootstrap:
    path: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md
    role: "Environment prerequisites (Supabase, daemon HTTP client, apps/web wiring, calibration corpus seeding)"

execution_prompts:
  kickoff:
    path: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-kickoff.md
    role: "Full agent onboarding"
  push:
    path: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-push.md
    role: "Continuation nudge"

milestone: M4
exit_criterion_met: true
exit_criterion: "Two users view the same pipeline: desktop operator (read-write) + web viewer (read-only). Sync engine operational. Validation enforcement active."

archived: docs/pipeline/l4-shared-visibility/archived/<DATE>-l4-shared-visibility/
```

### 5. Commit and Push

```bash
git add docs/pipeline/l4-shared-visibility/
git commit -m "$(cat <<'EOF'
pipeline: archive L4 shared visibility

L4 Shared Visibility implementation complete. All 18 DoD criteria met.
Sync engine operational, web portal live, validation enforcement active.
Two-user verification passed.

Total beads: <N> (task: <N>, audit-fix: <N>, audit iterations: <N>).
Handoffs moved to archived/<DATE>-l4-shared-visibility/.
Artifacts (spec/TDD/strategy/kickoff/push/status) remain in place for downstream layers.

Next pipeline: L5 (Expanded Commercial Intelligence) or L6 (Request Pipeline Automation).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
git pull --rebase
git push
git status     # MUST show "up to date with origin"
```

### 6. Update Strategy Index (cross-reference)

Update `docs/strategy/cross-reference-index.md` if it tracks layer completion status. Mark L4 as complete with implementation status note pointing to the archive.

Update `docs/strategy/layer-strategy.md` if it tracks layer completion. Same.

(These updates are optional but recommended — they help future agents and humans understand the layer completion timeline.)

### 7. Notify L5/L6 Pipeline Start (if applicable)

If L5 or L6 is the next planned pipeline run, the human user can now invoke:

```
/pipemaxxer docs/strategy/layers/layer-5-expanded-commercial-intelligence.md
```

or

```
/pipemaxxer docs/strategy/layers/layer-6-request-pipeline-automation.md
```

L4's completion unblocks L5 (sync + projection tables operational) and L6 (operator_decision_heads + shared_company_profile + validation enforcement available).

### 8. Report Deliverables

```
✅ L4 Shared Visibility — COMPLETE

📦 Deliverables:
  - Spec: docs/strategy/layers/layer-4-shared-visibility.md
  - TDD: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md
  - Strategy: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md
  - Bootstrap: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md
  - Kickoff: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-kickoff.md
  - Push: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-push.md
  - Status: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-status.yaml
  - Implementation: <N> beads executed (task: <N>, audit-fix: <N>)
  - Audit iterations: <N> to reach COMPLETE verdict

📊 Capabilities Delivered:
  - SQLite ↔ Supabase sync engine (push outbox + pull cursors + CAS)
  - Read-only Next.js web portal at apps/web/ (7 source files)
  - 8 Supabase migrations with RLS for 4 roles
  - Validation enforcement mode active on calibrated slices
  - 4 domain-specific validator overlays
  - 3-corpora calibration harness
  - Two-user verification passed

📁 Archived: docs/pipeline/l4-shared-visibility/archived/<DATE>-l4-shared-visibility/
🔄 Ready for next pipeline run: L5 or L6.
```

---

## Cleanup (Optional)

- Remove any temporary working files from earlier in the pipeline (calibration scratch, test fixtures used for iteration)
- Verify `git log` shows the L4 commit history is clean and tells a coherent story
- Verify no `.bak`, `.tmp`, `.swp` files were accidentally committed

## Memory Update (Recommended)

The user maintains an auto-memory at `/Users/alinaaitova/.claude/projects/-Users-alinaaitova-Desktop-Testproject1/memory/MEMORY.md`. Adding a brief L4 completion entry helps future conversations understand the pipeline timeline. Keep entries to one line under ~150 chars (the index is space-constrained — see existing MEMORY.md for the format).

Suggested entry (move detail into a topic file if needed):

```markdown
- [Pipemaxxer Run L4](pipemaxxer_l4.md) — L4 Shared Visibility shipped <DATE>; sync engine + web portal + enforcement; <N> beads, <N> audit iterations
```

If the index is already over the size limit warning, skip this step or trim older entries. Never add memory entries that contain information already derivable from `git log` or the spec/TDD/strategy artifacts.

---

## Next Pipeline Run

To start L5 or L6:

```
/pipemaxxer docs/strategy/layers/layer-5-expanded-commercial-intelligence.md
```

or

```
/pipemaxxer docs/strategy/layers/layer-6-request-pipeline-automation.md
```

The pipeline is stateless — each run generates fresh handoffs. Prior pipeline artifacts are archived at `docs/pipeline/l4-shared-visibility/archived/...`, not overwritten. The `artifacts/` directory at the top of the L4 pipeline dir remains in place as reference for downstream layers.

---

## Notes

- **L4 archived ≠ L4 frozen.** If a future bug requires changing L4 behavior, the spec at `docs/strategy/layers/layer-4-shared-visibility.md` is still editable. A new fix bead under `--label layer-4-followup` (not `audit-fix`) can be created and executed.
- **The artifacts directory is shared** — L5 and L6 implementations will reference L4's spec, kickoff, and push. Do not delete artifacts; do not rename without updating cross-references.
- **The completion audit report** is the historical record of what was found in step 12. Keep it in the archive for traceability.
- **Memory update (optional):** the user may want to update `MEMORY.md` to record L4 completion timeline, audit iterations, and any surprising findings.

---

## Critical Rules

1. **Do NOT archive prematurely.** Pre-archive verification (above) MUST pass.
2. **Do NOT move the `artifacts/` directory.** It stays in place for downstream reference.
3. **Do NOT delete handoff files** during archive. Use `git mv` to preserve history.
4. **Do NOT skip the DoD walkthrough.** All 18 criteria must be ✓.
5. **Do NOT skip two-user verification.** This is DoD #12 and is manual.
6. **Update the status YAML** to reflect completion (not just by archiving — the YAML is the canonical pointer).
7. **Commit + push.** Work is NOT done until `git push` succeeds (per AGENTS.md).
