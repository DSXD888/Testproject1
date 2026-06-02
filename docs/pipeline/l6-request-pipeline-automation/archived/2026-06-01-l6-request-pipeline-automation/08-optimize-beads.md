# L6 Request Pipeline Automation — Step 08: Optimize Beads (Fresh-Eyes Audit)

> **Different agent than step 07.** Fresh-eyes review of the L6 bead corpus.
> No skill to invoke. Manual reading + targeted `br` updates.

## Your Mission

Audit every L6 bead created by step 07. Verify self-containment, dependency
correctness, file-list accuracy, AC testability, and ZERO scope drift.

**You are reading and improving an existing bead corpus — not writing code.**

---

## What To Read (Do This First)

### The Bead Corpus to Audit
```bash
# Full bead inventory
br list --label l6 --json | jq '.issues[]'

# Per-bead detail (walk EVERY bead)
br show l6-phase1-errorcode-extension --json
br show l6-phase2-jobs-runner-dispatch --json
# ... continue for every bead
```

### Companion Artifacts
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bulk-import.md
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-bead-alias-map.json
```

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md           # SOT #1 — every FR has a bead
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md       # SOT #2 — every test ID owned 1:1
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md  # SOT #3 — phase + dependency
docs/pipeline/l6-request-pipeline-automation/manifest.yaml            # existing_files + new_modules — every bead path must match
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/errors/manifest.rs:19-114                           # ErrorCode bead must list exact 6 variants
crates/daemon/src/main.rs:1513-1517                                   # **Job-kind dispatch insertion point** — Phase 2 arm goes HERE next to extract_document_* arms; handler functions at lines 1529 + 1836 are templates
crates/daemon/src/jobs/runner.rs                                      # Shared types only (NO match dispatch)
crates/daemon/src/ipc/handlers/jobs.rs                                # IPC handler pattern
crates/daemon/src/main.rs:5256-5260,5510-5527,5537-5542,5781-5787     # L5 readiness raw checks bid-readiness-gate bead must wrap
skills/auth-signin/procontract/SKILL.md                               # auth delegation surface
skills/procurement-ingestion/procontract-register-interest/SKILL.md   # reference bid pattern
skills/validation/overlay-supplier-enrichment/SKILL.md                # overlay frontmatter shape
```

### Quality Bar
```
docs/pipeline/l5-expanded-commercial-intelligence/archived/2026-05-21-l5-expanded-commercial-intelligence/08-optimize-beads.md  # 224 lines
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md  # most detailed L4 bulk-import precedent (4131 lines)
```

---

## Think Independently — 10 Audit Dimensions

### 1. Self-Containment
For each bead, ask: can a fresh agent implement this from the bead alone, without
re-reading the spec or TDD? Specifically:
- Spec FR quoted verbatim?
- TDD test IDs listed?
- File paths exact?
- ACs testable?

If the answer requires "consult the spec," the bead is not self-contained — fix it.

### 2. File List Accuracy
Open each `Files to Create/Modify` path. For "modify" entries: does the file actually
exist? For "create" entries: does the parent directory exist and is the path the right
location?

Specifically:
- Phase 1 ErrorCode bead → `crates/daemon/src/errors/manifest.rs` (exists, line 19-114)
- Phase 2 dispatch bead → `crates/daemon/src/jobs/runner.rs` (exists)
- Phase 2 IPC bead → `crates/daemon/src/ipc/handlers/jobs.rs` (exists)
- Phase 4 portal skill bead → `skills/bid-portal-procontract/SKILL.md` (does NOT exist yet; correctly "create")
- Phase 5 validator overlay bead → `skills/validation/overlay-bid-portal-procontract/SKILL.md` (does NOT exist yet; correctly "create")
- Phase 5 packages/bidding beads → `packages/bidding/{workflow-registry,procontract-workflow,company-profile-schema}.ts` (does NOT exist yet; correctly "create")
- Phase 6 UI beads → `packages/ui/src/{BidWorkflowStatusBadge,BidScopeSummary,DraftSavedReviewPanel,BidWorkflowStartButton}.tsx` (does NOT exist yet; correctly "create")

`bin/br-claim` parses the Files section to compute reservations. If a path is wrong,
the claim will reserve the wrong files or fail.

### 3. AC Testability
For each AC bullet:
- Is it verifiable by running a command?
- Is the success condition concrete?
- Does it map to a specific L6-T-* test ID?

Bad: "validates correctly"
Good: "given operator_decision_heads.disposition='approved' AND reviewedBasisHash==decisionBasisHash AND no active L5 blockers, when readiness gate is evaluated, then returns ReadinessGateResult::Approved (proven by L6-T-bid_readiness_gate-001)"

### 4. Dependency Correctness
Walk the dependency graph:
- Phase 1 beads → no dependencies (root)
- Phase 2 beads → depend on Phase 1 ErrorCode bead (clippy needs new variants)
- Phase 3 beads → depend on Phase 2 dispatch + IPC handler
- Phase 4 beads (portal skill) → depend on Phase 1 types
- Phase 5 beads (validator overlay + packages/bidding) → depend on Phase 1 types + Phase 4 worker_behavior_version contract
- Phase 6 beads (UI) → depend on Phase 2 IPC handler + Phase 5 validator overlay verdicts
- Phase 7 beads (live demo) → depend on all prior phases

If a bead has a dependency that doesn't exist or is in a later phase, the DAG is broken.

`br lint` should report zero issues. If it reports cycles or broken edges, fix.

### 5. Test ID Ownership (1:1)
Open `l6-bead-alias-map.json`. For every L6-T-* test ID, verify exactly ONE bead owns it.
If a test ID appears in zero beads, it's orphaned — assign to a bead. If it appears in
multiple beads, ownership is unclear — assign to exactly one.

### 6. Spec FR Coverage (100%)
For every L6-FR-X.Y.Z, verify at least one bead implements it. Build a quick coverage
matrix:

| L6-FR-X.Y.Z | Bead(s) |
|-------------|---------|
| L6-FR-1.1.1 | l6-phase4-portal-skill-narrative |
| L6-FR-1.3.1 | l6-phase3-state-machine-not-started |
| ... | ... |

Any FR with zero beads → BLOCKING. Add the bead.

### 7. Phase Coverage Equality
Count beads per phase. If Phase 7 (live demo) has 2 beads and Phase 4 (portal skill)
has 10, investigate. Phase 7 MUST have:
- Bead for first live demo (sideEffectCap=REMOTE_DRAFT → draft_saved)
- Bead for operator-authorized second live demo (FORMAL_SUBMISSION → bid_submitted)
- Bead for calibration corpus validation (canary inputs run against bid-portal-procontract)
- Bead for final quality gate sweep
- Bead for any cross-phase integration verification

If Phase 7 is anomalously thin, it's a scope-drift signal — the live demo was reduced
to "just one quick run."

### 8. Validator Overlay Bead (decision_3)
Phase 5 MUST include a bead for `skills/validation/overlay-bid-portal-procontract/`.
Verify:
- Files section includes `skills/validation/overlay-bid-portal-procontract/SKILL.md` (create)
- ACs reference all the overlay assertions (opportunity ref match, screenshot count, side-effect monotonicity, etc.)
- TDD test IDs for the overlay are owned by this bead

### 9. Live Demo Bead Operator Gates (decision_5)
Phase 7 live demo bead must list operator-action gates as ACs:
- [ ] Persistent CDP profile at `/Users/alinaaitova/.chrome-automation` has saved sales@sorelon.com credential
- [ ] Touch ID requirement disabled in Chrome's password autofill (operator confirmed 2026-05-21)
- [ ] At least one Procontract opportunity ref identified (won't generate buyer-visible legal commitment for first run)
- [ ] `bin/ensure-cdp-9222` returns success
- [ ] AgentMail healthy
- [ ] sideEffectCap=REMOTE_DRAFT for first run
- [ ] Operator EXPLICITLY authorizes a separate run with sideEffectCap allowing FORMAL_SUBMISSION

### 10. Scope-Drift Scan
Grep the bulk-import doc + `br list` output for:
- "post-MVP" / "post-L6" / "Phase 8+"
- "stretch goal" / "nice to have"
- "out of scope" / "deferred"
- "future iteration" / "later"
- "simplified for initial implementation"
- "if time permits"
- "could add"
- "optional"

Zero hits expected.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Coverage Verification
- [ ] Every L6-FR has ≥1 bead (open bead-alias-map.json, walk every FR)
- [ ] Every L6-T-* test ID has exactly 1 owning bead
- [ ] Every DoD criterion (11) is satisfied by ≥1 bead
- [ ] All **14** state transitions have implementing beads (spec §1.3 has 14 distinct From→To rows — verify by counting yourself; do not undercount as 11)
- [ ] All 6 SideEffectClass values are tested by some bead
- [ ] All 5 L5 readiness blockers are wrapped by the readiness-gate bead
- [ ] Validator overlay bead exists in Phase 5
- [ ] Live demo bead exists in Phase 7 with operator-action gates

### Phase Coverage Equality
- [ ] Each phase has 3+ beads (Phase 1 may be smaller)
- [ ] No phase is anomalously thin
- [ ] Phase 7 has at least 4 beads (first demo + second demo + calibration validation + final gate sweep)

### Red-Flag Language Scan
- [ ] Zero "post-MVP" / "stretch" / "nice to have" / "deferred"
- [ ] Zero "TODO" / "placeholder" in bead bodies (other than AC checkboxes)
- [ ] Zero "and related files" in Files sections (must be exact)

---

## Follow-Up Pass (Run After Fixes)

> ULTRATHINK. Re-read YOUR fixes with completely fresh eyes.

1. **Did fixing one bead break another's dependencies?** If you split a bead, the new sub-beads need their own dependency edges. `br lint` to verify.

2. **Did you add a test to a bead but forget to update the alias map?** Open `l6-bead-alias-map.json` and verify your edits are reflected.

3. **Did you change a Files section?** `bin/br-claim` reads it — verify the new file list is parseable (one path per backticked entry).

4. **Walk every Phase 7 bead.** Are operator-action gates explicit? Are sideEffectCap defaults explicit?

5. **Re-run `br lint` + `br list --label l6 --json`.** Verify no broken dependencies, no orphan beads.

6. **Re-walk every spec invariant (§1.10, 8 of them).** For each, identify the bead that enforces it. If any invariant has no enforcing bead, add one.

7. **Re-run scope-drift grep. Zero hits.**

8. **Count phases + beads per phase.** Report:
   ```
   Phase 1 — Foundation: N beads
   Phase 2 — Daemon Runtime: N beads
   Phase 3 — State Machine: N beads
   Phase 4 — Portal Skill: N beads
   Phase 5 — Validator Overlay + packages/bidding: N beads
   Phase 6 — Desktop + Web UI: N beads
   Phase 7 — Live Demo + Integration: N beads
   Total: N task beads + N review-gate beads + 1 implementation epic = N total
   ```

9. **Verify bin/br-sync was run after edits.** If you used `br update` directly, the JSONL export is stale until `bin/br-sync` runs.

---

## Post-Audit Commit

After all edits + verification:

```bash
bin/br-sync
git add .beads/issues.jsonl
git add docs/pipeline/l6-request-pipeline-automation/artifacts/
git commit -m "l6-pipeline: optimize beads after step 08 fresh-eyes audit"
git push
```

---

## Output

- Bead corpus (updated via `br update` for each fixed bead)
- Updated bulk-import doc (regenerated or hand-edited)
- Updated alias map (regenerated or hand-edited)
- No new files produced (other than re-exports)
- Bead activity: bead updates only (no claim/complete — step 08 is doc work, not bead-tracked implementation)

---

## Key Reminders

- Fresh eyes — assume step 07 missed things
- Cross-reference everything against codebase ground truth
- `bin/br-claim` parses Files sections — be exact
- Validator overlay (decision_3) MUST have a bead
- Live demo (decision_5 + DoD #8) MUST have operator-action gates
- Phase 7 has at least 4 beads (no thin "just one demo" phase)
- ZERO SCOPE DRIFT
- Match L5 optimize-beads quality (224 lines)
