# L6 Request Pipeline Automation — Step 15: Archive Pipeline

## Your Mission

The L6 implementation pipeline is COMPLETE. Archive all pipeline artifacts, update
status, update project memory, and report deliverables.

**L6 is the FINAL layer per strategy.** There is no L7. This is the M6 capstone
completion. The next `/pipemaxxer` invocation will target a different scope (or
none — the procurement intelligence platform's layered architecture is complete).

---

## Pre-Archive Verification (ALL must be GREEN before archiving)

### Verification Checklist

1. **All L6 beads closed:**
   ```bash
   br list --label l6 --status open --json | jq '.issues | length'
   # Expect: 0 (or only the implementation epic itself)
   ```

2. **All fix beads closed (if any were created):**
   ```bash
   br list --label l6-audit-fix --status open --json | jq '.issues | length'
   # Expect: 0
   ```

3. **Completion audit verdict = COMPLETE:**
   ```bash
   # Open the latest l6-completion-audit-YYYY-MM-DD.md and verify the verdict line
   grep -i "^.*verdict.*COMPLETE" /Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l6-request-pipeline-automation/artifacts/l6-completion-audit-*.md
   ```

4. **Full quality gate sweep is green:**
   ```bash
   cd /Users/alinaaitova/Desktop/Testproject1
   cargo fmt --check
   cargo check --all-targets
   cargo clippy --all-targets -- -D warnings
   cargo test -p daemon --bin daemon
   for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
   bun run build
   bun run lint
   bun run test
   (cd apps/desktop && bun run test:e2e)
   (cd apps/web && bun run test:e2e)
   # ALL must exit 0
   ```

5. **DoD walkthrough — all 11 criteria GREEN** (from completion audit):
   - [ ] DoD #1: Bid-portal skill navigates Procontract via browser automation
   - [ ] DoD #2: Company profile injected from `shared_company_profile`
   - [ ] DoD #3: Form filling + document upload with side-effect class tracking
   - [ ] DoD #4: Submission with screenshot at each stage
   - [ ] DoD #5: Full terminal output tracing
   - [ ] DoD #6: Run manifest generated with audit trail
   - [ ] DoD #7: Bid failure → error manifest with portal-specific context
   - [ ] **DoD #8: Live demo passed** (first run REMOTE_DRAFT → draft_saved; operator-authorized second run FORMAL_SUBMISSION → bid_submitted)
   - [ ] DoD #9: draft_saved path works when reviewRequired=true
   - [ ] DoD #10: L5 readiness gate blocks ready_to_bid with active L5 blockers
   - [ ] DoD #11: Validator rejection or outage blocks bid_submitted (fail-closed)

6. **L6 capability deliverable verification (live, not just tests):**
   ```bash
   # Verify the live demo actually happened
   sqlite3 /Users/alinaaitova/Desktop/Testproject1/state.db "SELECT process_id, bid_state, started_at, completed_at FROM bid_workflow_states WHERE bid_state IN ('draft_saved', 'bid_submitted') ORDER BY completed_at DESC LIMIT 5;"
   # Expect at least one row (the live demo's draft_saved + optionally bid_submitted)

   # Verify Supabase visibility (web viewer can see the demo's bid_state)
   # — operator may need to manually confirm via web portal
   ```

7. **Git state clean:**
   ```bash
   git status
   # Expect: nothing to commit, working tree clean

   git log --oneline -10
   # Last commits should be L6 work + perhaps the audit verdict commit

   git rev-parse HEAD
   git rev-parse @{u}
   # Both must match (pushed to remote)
   ```

8. **Cross-layer interface contracts unchanged:**
   - L5→L6 readiness gate (5 blockers + quote-role + freshness + upstream-trust) still works correctly
   - L4 sync engine still pushes `bid_workflow_states` to Supabase (no regression)
   - L3 validator subsystem still handles `validation_policy: always` for bid runs
   - L2 desktop control panel still operational (no regression in pipeline tabs)
   - L1 daemon still operational (PTY/cron/scheduler functional)
   - L0 schema unchanged (bid_workflow_states still has its L0-era columns)

If ANY checklist item is unchecked, STOP. Do not archive.

---

## Archive Steps

### 1. Create Archive Folder + Auxiliary Subdirectories
```bash
ARCHIVE_DATE=$(date +%Y-%m-%d)
ARCHIVE_DIR="/Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l6-request-pipeline-automation/archived/${ARCHIVE_DATE}-l6-request-pipeline-automation"
mkdir -p "${ARCHIVE_DIR}"

# L5 pipeline established these auxiliary subdirs that survive across pipeline cycles
# (not moved into archive — they persist alongside artifacts/):
mkdir -p /Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l6-request-pipeline-automation/live-demo
# (populated by step 6c below)

# Optional, mirror L5 if operator-built skill prerequisites end up being added for L6:
# mkdir -p /Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l6-request-pipeline-automation/operator-built-skills
# (per decision_5, L6 needed NO operator-built skill prerequisites; this dir typically stays empty for L6)
```

### 2. Move Handoff Files to Archive
```bash
cd /Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l6-request-pipeline-automation

# Move all 15 handoff files + EXECUTION-GUIDE + manifest to archive
mv 01-validate-spec.md "${ARCHIVE_DIR}/"
mv 02-optimize-spec.md "${ARCHIVE_DIR}/"
mv 03-create-tdd.md "${ARCHIVE_DIR}/"
mv 04-optimize-tdd.md "${ARCHIVE_DIR}/"
mv 05-create-strategy.md "${ARCHIVE_DIR}/"
mv 06-optimize-strategy.md "${ARCHIVE_DIR}/"
mv 07-create-beads.md "${ARCHIVE_DIR}/"
mv 08-optimize-beads.md "${ARCHIVE_DIR}/"
mv 09-audit-beads.md "${ARCHIVE_DIR}/"
mv 10-generate-kickoff.md "${ARCHIVE_DIR}/"
mv 11-push.md "${ARCHIVE_DIR}/"
mv 12-completion-audit.md "${ARCHIVE_DIR}/"
mv 13-fix-beads-kickoff.md "${ARCHIVE_DIR}/"
mv 14-fix-beads-push.md "${ARCHIVE_DIR}/"
mv 15-archive.md "${ARCHIVE_DIR}/"
mv EXECUTION-GUIDE.md "${ARCHIVE_DIR}/"
mv manifest.yaml "${ARCHIVE_DIR}/"

# Copy (don't move) the completion audit + permanent artifacts so they remain at canonical paths
cp artifacts/l6-completion-audit-*.md "${ARCHIVE_DIR}/"
```

**DO NOT** move the `artifacts/` directory itself. The permanent artifacts (spec is
already at `docs/strategy/layers/`; TDD/bootstrap/strategy/kickoff/push/status remain
at `docs/pipeline/l6-request-pipeline-automation/artifacts/`) stay in place for
future reference.

### 3. Update Status YAML

```bash
# Edit docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-status.yaml
# Update fields:
```

```yaml
feature: l6-request-pipeline-automation
status: complete
completed: "YYYY-MM-DD"                        # today's date
spec: docs/strategy/layers/layer-6-request-pipeline-automation.md
tdd: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md
bootstrap: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md
strategy: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md
kickoff: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-kickoff.md
push: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-push.md
completion_audit: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-completion-audit-YYYY-MM-DD.md
bead_prefix: l6
bead_label: layer-6
total_beads: <count>                            # final br list count
task_beads: <count>
review_gate_beads: <count>
fix_beads: <count>                              # 0 if audit was COMPLETE first time
audit_iterations: <count>                       # 1 if first audit COMPLETE; 2+ if fix-bead loop
existing_layer_doc_epic: testproject1-rc0        # CLOSED in step 07
existing_implementation_epic: <epic-id>          # from step 07
agentmail_key: /Users/alinaaitova/Desktop/Testproject1
branch: main
target_milestone: M6
exit_criteria_met: true
ready_for_next_layer: N/A                       # L6 is final layer
live_demo_first_run_state: draft_saved          # confirms decision_5 implementation
live_demo_second_run_state: bid_submitted        # confirms full DoD #8

m6_capstone_completion:
  procurement_intelligence_platform: COMPLETE
  layers_built: ["L0", "L1", "L2", "L3", "L4", "L5", "L6"]
  layers_planned: []
  next_layer: N/A
```

### 4. Update Project Memory

Append a brief entry to `/Users/alinaaitova/.claude/projects/-Users-alinaaitova-Desktop-Testproject1/memory/MEMORY.md`:

```markdown
## L6 Implementation Complete (YYYY-MM-DD)

- L6 Request Pipeline Automation: COMPLETE — l6-completion-audit-YYYY-MM-DD.md verdict COMPLETE
- All 11 DoD criteria green; live demo passed (REMOTE_DRAFT → draft_saved; FORMAL_SUBMISSION → bid_submitted)
- N L6 beads closed; M fix beads (N audit iterations)
- Sister epic (created step 07, closed step 15)
- Naming reconciliation Proactis=Procontract baked in across all artifacts
- Reused existing skills/auth-signin/procontract/ (decision_2)
- Validator overlay skills/validation/overlay-bid-portal-procontract/ created (decision_3)
- N ErrorCode variants added (fill in actual count from TDD/implementation): likely 3-6 from the TDD-determined set after auditing existing PAGE_SHAPE_DRIFT / AUTH_STATE_STALE / AGENT_SKILL_FAILED for L6 coverage
- Archive: docs/pipeline/l6-request-pipeline-automation/archived/YYYY-MM-DD-l6-request-pipeline-automation/

## M6 Capstone

The UK public procurement intelligence platform is now layer-COMPLETE:
- L0 Gold-set + evaluation harness (built)
- L1 Thin local runner (built)
- L2 Single-operator desktop MVP (built)
- L3 Operational hardening (built)
- L4 Shared visibility (built 2026-05-08)
- L5 Expanded commercial intelligence (built 2026-05-21)
- L6 Request pipeline automation (built YYYY-MM-DD)
- L7+: NONE — platform layered architecture complete
```

### 5. Update L6 Spec's Implementation Status Callout

Edit `docs/strategy/layers/layer-6-request-pipeline-automation.md`'s top callout:

```markdown
> **Implementation Status (as of YYYY-MM-DD):** Layer 6 is **BUILT — production**.
> All 11 DoD criteria green per `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-completion-audit-YYYY-MM-DD.md`.
> Live demo completed: first run REMOTE_DRAFT → draft_saved; operator-authorized second
> run FORMAL_SUBMISSION → bid_submitted. Implementation epic (sister of testproject1-rc0)
> closed. Pipeline archive at `docs/pipeline/l6-request-pipeline-automation/archived/YYYY-MM-DD-l6-request-pipeline-automation/`.
> Stage 15 (`bid_workflow`) deferred handoff at `prompts/stage-15-bid-workflow-handoff-DEFERRED.md`
> is now SUPERSEDED by L6 — flag for cleanup or rewrite if any historical reference is needed.
```

### 6. Update Strategy Docs (high-level layer-strategy.md)

Edit `docs/strategy/layer-strategy.md` to reflect L6 BUILT status:
- Line 16 ("Layer 6 (Request Pipeline Automation): NOT BUILT. Stage 15 (`bid_workflow`) blocked on this layer.") → "Layer 6 (Request Pipeline Automation): BUILT (YYYY-MM-DD). Stage 15 (`bid_workflow`) superseded by L6 implementation."
- Update L6 row in the layer-status table to reflect built status

Edit `docs/strategy/README.md` "Layer Summary" table:
- Layer 6 row → Status: Built; Stage 15 (`bid_workflow`) superseded

### 6a. Update L5 status (cross-layer link)

L5's status file currently records `ready_for_l6: true` and `l5_to_l6_gate_producible: true`.
With L6 BUILT, add a confirmation entry noting the L5→L6 gate was verified live:

```bash
# Append to docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-status.yaml
# (or edit the file to add at the bottom):
```

```yaml
l5_to_l6_gate_verified_live:
  verified_at: "YYYY-MM-DD"          # today
  via: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-completion-audit-YYYY-MM-DD.md
  verdict: "L5→L6 readiness gate (5 blockers + quote-role + freshness + upstream-trust) operational in production. Bid workflow respects all L5 hard blockers; operator-overrideable gates (quote-role, validation) recorded audit-trail entries on every override during live demo."
  l6_status: complete
  l6_built_date: "YYYY-MM-DD"
```

### 6b. Add inline status callouts to L6 spec (per project README convention)

The project README (`docs/strategy/README.md`) states:
> L1-L3 also include inline status callouts for load-bearing design decisions:
> ✓ As designed / ⚠ Deviation / ✗ Deferred / ↻ Superseded.

With L6 BUILT, walk the L6 spec (`docs/strategy/layers/layer-6-request-pipeline-automation.md`) and annotate load-bearing design decisions with inline status markers based on what actually shipped:
- **✓ As designed** — implementation matches the spec choice exactly
- **⚠ Deviation** — what was actually built differs from pre-build spec (record what + why)
- **✗ Deferred** — planned work that did NOT land (should be zero per spec §2.8 "Nothing — final layer"; if non-zero, document as fix-bead candidate)
- **↻ Superseded** — historical design replaced by a later decision (e.g., the per-step side-effect class tracking — was it implemented per spec or differently?)

Key sections to annotate:
- §1.3 transition table (each of 14 transitions: ✓ or ⚠)
- §1.4 BidScopePayload fields (each field: ✓ or ⚠)
- §1.6 packages/bidding files (each file: ✓ As designed with rename to `procontract-workflow.ts` per decision_1)
- §1.7 skill playbook (each of 11 steps: ✓ or ⚠)
- §1.10 invariants (each of 8: ✓ enforced)
- §2.1 DoD criteria (each of 11: ✓ green)

### 6c. Live-demo subdirectory (mirror L5 pattern)

L5's pipeline directory has a `live-demo/` subdirectory capturing the live-demo run artifacts
(screenshots, transcripts, bid_workflow_states rows for that run). L6 should preserve the
same pattern:

```bash
mkdir -p /Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l6-request-pipeline-automation/live-demo

# Capture the live demo run materials (run IDs, screenshot paths, final state):
cat > docs/pipeline/l6-request-pipeline-automation/live-demo/first-run-REMOTE_DRAFT.md <<EOF
# L6 Live Demo — First Run (REMOTE_DRAFT)

**Date:** YYYY-MM-DD
**Process ID:** <process_id>
**Portal Opportunity Ref:** <portalOpportunityRef>
**Side Effect Cap:** REMOTE_DRAFT
**Final State:** draft_saved
**Stage Run ID:** <stage_run_id>
**Run Manifest:** artifacts/sessions/.../run-manifest.md
**Screenshots:** artifacts/sessions/.../step-{01..11}.webp (11 captured)
**Validator Verdict:** approved (or override per decision_5)
**Supabase Visibility:** Web viewer can see bid_state=draft_saved badge; BidScopePayload + run manifest hidden by RLS
EOF

cat > docs/pipeline/l6-request-pipeline-automation/live-demo/second-run-FORMAL_SUBMISSION.md <<EOF
# L6 Live Demo — Second Run (FORMAL_SUBMISSION, operator-authorized)

**Date:** YYYY-MM-DD
**Process ID:** <process_id>
**Portal Opportunity Ref:** <portalOpportunityRef>
**Side Effect Cap:** FORMAL_SUBMISSION (operator explicitly authorized)
**Operator Authorization:** recorded in operator_decision_heads override field
**Final State:** bid_submitted
**Portal Confirmation Ref:** <portal-generated reference>
**Stage Run ID:** <stage_run_id>
**Run Manifest:** artifacts/sessions/.../run-manifest.md
**Validator Verdict:** approved
**Supabase Visibility:** Web viewer can see bid_state=bid_submitted badge
EOF

# Also write a bid-run diary capturing both runs (analogous to L4's two_user_smoke_diary):
cat > docs/pipeline/l6-request-pipeline-automation/live-demo/l6-bid-run-diary.md <<EOF
# L6 Bid Run Diary

Records both live-demo runs end-to-end with observations, deviations, and operator feedback.
Format mirrors docs/pipeline/l4-shared-visibility/artifacts/l4-two-user-smoke-diary.md.

## Run 1 — REMOTE_DRAFT (draft_saved)
[narrative of what happened, what went smoothly, what was surprising]

## Run 2 — FORMAL_SUBMISSION (bid_submitted)
[narrative, including operator's authorization moment + portal confirmation receipt]

## Observations
[deviations from spec, any L6-FR-X.Y.Z that needed clarification mid-run]

## Operator Feedback
[any UX issues with desktop BidWorkflow tab, any validator overlay false-positives, etc.]
EOF
```

### 7. Commit + Push

```bash
cd /Users/alinaaitova/Desktop/Testproject1
git add docs/pipeline/l6-request-pipeline-automation/archived/
git add docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-status.yaml
git add docs/pipeline/l6-request-pipeline-automation/live-demo/
git add docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-status.yaml  # 6a edit
git add docs/strategy/layers/layer-6-request-pipeline-automation.md  # 6b inline callouts
git add docs/strategy/layer-strategy.md
git add docs/strategy/README.md
git add /Users/alinaaitova/.claude/projects/-Users-alinaaitova-Desktop-Testproject1/memory/MEMORY.md

git commit -m "l6-archive: complete pipeline archive for L6 Request Pipeline Automation"
git push
```

### 8. Bead Hygiene

```bash
# Verify L6 epic is closed (it should be — happened in step 07 for rc0 + step 15 for sister)
br list --label l6 --status open --json | jq '.issues | length'
# Expect: 0

# Close the implementation sister epic if not already closed
br close <SISTER_EPIC_ID> --reason "L6 implementation complete; live demo passed; archive YYYY-MM-DD"

# Final sync
bin/br-sync
git add .beads/issues.jsonl
git commit -m "l6-archive: close implementation epic; final bead sync"
git push
```

### 9. Report Deliverables

Final report to user:

```
✅ L6 Request Pipeline Automation — COMPLETE

📦 Deliverables:
  - Spec: docs/strategy/layers/layer-6-request-pipeline-automation.md
  - TDD: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md (~NNN lines)
  - Bootstrap: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md (~NNN lines)
  - Strategy: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md (~NNN lines)
  - Kickoff: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-kickoff.md (~NNN lines)
  - Push: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-push.md (~NNN lines)
  - Status: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-status.yaml
  - Completion Audit: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-completion-audit-YYYY-MM-DD.md
  - Implementation: N beads executed (X task + Y review gate + Z fix), N audit iterations

📁 Archived: docs/pipeline/l6-request-pipeline-automation/archived/YYYY-MM-DD-l6-request-pipeline-automation/

🎯 M6 Capstone Achieved:
  - L6 = final layer
  - Procurement intelligence platform layered architecture COMPLETE
  - Live demo passed: prepared and submitted one request on Procontract end-to-end
  - All 8 invariants enforced
  - All 11 DoD criteria green
  - L5→L6 readiness gate (5 blockers + quote-role + freshness + upstream-trust) operational
  - Validator overlay (skills/validation/overlay-bid-portal-procontract/) enforces fail-closed for portal submission

🔄 Next:
  - No L7 — platform layered architecture is complete
  - Operational L6 work is now ongoing (production bid workflows on operator-chosen opportunities)
  - Pipeline ready for next feature (if any) via /pipemaxxer
```

---

## Optional Clean-Up

- Remove any temporary working files in `docs/pipeline/l6-request-pipeline-automation/` (only artifacts/ + archived/ should remain after archive move)
- Verify `.beads/` is clean (no orphan beads, no in-progress beads)
- Verify file reservations are all released:
  ```bash
  curl -s http://127.0.0.1:8765/file-reservations | jq '.[] | select(.reason | startswith("l6-"))'
  # Expect: empty array
  ```

---

## Next Pipeline Run

L6 is the final layer. There is no L7. If the operator wants to begin a new
feature pipeline:

```
/pipemaxxer <new-feature-source-doc>
```

The pipeline is stateless — each run generates fresh handoffs. L6 archive
remains for reference. Status YAML stays at canonical path.

---

## Key Reminders

- **DO NOT archive** if any pre-archive checklist item is unchecked
- **DoD #8 (live demo) is the M6 capstone** — both first AND second run must have happened
- **Update MEMORY.md** so future pipemaxxer runs know L6 is built
- **Update layer-strategy.md + README.md** so the layer status table is current
- **Supersede the old Stage 15 deferred handoff** (`prompts/stage-15-bid-workflow-handoff-DEFERRED.md`) — flag for cleanup
- **Use `bin/br-sync`** for final bead export (NEVER raw `br sync`)
- L6 = final layer — no `ready_for_l7` field, no L7 readiness gate
- The procurement intelligence platform layered architecture is COMPLETE at M6
