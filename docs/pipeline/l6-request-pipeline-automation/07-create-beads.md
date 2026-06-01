# L6 Request Pipeline Automation — Step 07: Create Beads

## First Action

Invoke the `/beads-from-sources` skill.

---

## Your Mission

Materialize the L6 implementation work as **self-contained beads** (br issues). Every
bead carries the spec FR text it implements, the TDD tests it owns, the strategy phase
it belongs to, the exact files it touches, and its dependency edges.

**Three Sources of Truth feed every bead:**
- **Spec** (SOT #1) — WHAT to build (FR text)
- **TDD** (SOT #2) — HOW to verify (test IDs + assertions)
- **Strategy** (SOT #3) — WHAT ORDER (phase + dependency graph)

Per decision_4: BEFORE creating beads, **close the existing layer-doc epic `testproject1-rc0`** and **create a sister implementation epic** (matches L4 + L5 pattern).

---

## Source Documents (Read In This Order)

### Three Sources of Truth (READ FULLY)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md           # SOT #1 (WHAT)
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md       # SOT #2 (HOW TO VERIFY)
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md # Environment + quality gate
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md  # SOT #3 (WHEN)
```

### Manifest (existing infrastructure + new modules — every bead must accurately reference)
```
docs/pipeline/l6-request-pipeline-automation/manifest.yaml            # existing_files + new_modules_ts + new_modules_rust + new_skills + recorded_decisions
```

### Codebase (REFERENCE for bead file lists)
```
# Existing infrastructure
crates/daemon/src/errors/manifest.rs:19-114                           # ErrorCode (Phase 1 extends)
crates/daemon/src/main.rs:1513-1517                                   # **Job-kind dispatch lives HERE** — Phase 2 adds "run_bid_workflow" arm next to existing extract_document_* arms; handler function follows run_extract_document_structure_job pattern at line 1529
crates/daemon/src/jobs/runner.rs (1656 lines)                         # Shared types (ClaimedJob, EnqueueJobRequest, JobLeaseHeartbeat) — NOT where match dispatch lives
crates/daemon/src/ipc/handlers/jobs.rs                                # IPC handlers (Phase 2 extends)
crates/daemon/src/validation/{policy,executor,manifest_writer,calibration,feedback,provisional}.rs    # validation surface
crates/daemon/src/agents/adapter.rs:104-108                           # InteractionProfile
crates/daemon/src/db/sync.rs:24-50                                    # push set (no-op for L6; reference only)
crates/daemon/src/main.rs:5256-5260,5510-5527,5537-5542,5781-5787     # L5 readiness checks (Phase 3 references)
crates/daemon/src/sources/registry.rs (1338 lines total)              # source registry — Phase 1 extends (adds RegisteredSource entries for bid-portal-procontract + overlay-bid-portal-procontract)
crates/daemon/tests/skill_canary_harness.rs:255                       # canary tests (Phase 4 extends with e2e_can_010+)
crates/daemon/tests/skill_golden_journeys.rs:614                      # promotion lifecycle (Phase 4 extends)
crates/daemon/tests/redaction_boundary.rs                             # PII redaction (Phase 3 extends)
crates/daemon/tests/sqlite_schema.rs (1861 lines total)               # schema verification (Phase 1 extends if column-list assertions needed)

# Existing skills
skills/auth-signin/procontract/SKILL.md                               # delegate (Phase 4 references)
skills/procurement-ingestion/procontract-register-interest/SKILL.md   # reference pattern (Phase 4)
skills/validation/overlay-supplier-enrichment/SKILL.md                # overlay frontmatter shape (Phase 5)

# Existing schema
migrations/0001_sqlite_state_initial.sql:437-451                      # bid_workflow_states (no DDL changes)
migrations/0003_sqlite_runtime.sql:49-65                              # error_manifests (Phase 3 reads, doesn't write)

# Existing TS surface
packages/contracts-schema/src/index.ts:463                            # type re-exports
packages/bidding/                                                     # DOES NOT EXIST — Phase 5 creates
packages/ui/                                                           # extends with bid workflow components (Phase 6)
apps/desktop/                                                          # integrates BidWorkflow tab (Phase 6)
apps/web/                                                              # integrates read-only bid_state badge (Phase 6)
```

### Quality Bar
```
docs/pipeline/l5-expanded-commercial-intelligence/archived/2026-05-21-l5-expanded-commercial-intelligence/07-create-beads.md  # 253 lines
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md  # 4131 lines — L4 bulk-import (most detailed precedent)
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-bead-alias-map.json  # bead inventory JSON pattern
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**Every spec FR has a bead. Every TDD test ID is owned by exactly one bead.**

You may NOT:
- Create a bead labeled "post-MVP" or "stretch" or "Phase 8"
- Drop FRs because they look hard
- Lump multiple distinct FRs under "investigation needed" — each FR is a concrete deliverable
- Defer validator overlay (decision_3) — it's L6 deliverable
- Defer live demo (DoD #8 = M6 capstone)

If `br lint` reports ANY FR with zero corresponding beads, fix before declaring step 07 complete.

---

## Key Context From Codebase Research

- **No DDL changes** — `bid_workflow_states` already exists from L0 with all needed columns
- **No cron entries** — L6 is operator-triggered
- **No sync engine changes** — `bid_workflow_states` already in push set
- **3-6 NEW ErrorCode variants** (TDD-determined after auditing existing codes for redundancy) with clippy-surfaced exhaustive matches across 5-15 sites
- **`packages/bidding/` is greenfield** — creates package.json, tsconfig.json, vitest.config.ts + 4 source files + 3 test files
- **Validator overlay** has its own skill directory + frontmatter following overlay-supplier-enrichment precedent
- **CDP Chrome profile** at `/Users/alinaaitova/.chrome-automation` (port 9222) is shared with all existing procontract skills
- **Existing sister-epic pattern** (L4: testproject1-4zq; L5: separately-tagged sister epic) — L6 follows

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)
1. All 4 source-of-truth docs (spec / TDD / bootstrap / strategy) — **READ FULLY**
2. `manifest.yaml` — **READ FULLY**
3. `docs/pipeline/l5-expanded-commercial-intelligence/archived/2026-05-21-l5-expanded-commercial-intelligence/07-create-beads.md` — **STUDY** for quality bar

### Step 1: Pre-Conditions for Bead Creation (verify recorded in spec)

All FIVE step 01 decisions MUST be present in spec §1.0 and resolved:
- Decision 1 — Proactis = Procontract naming reconciliation
- Decision 2 — Reuse skills/auth-signin/procontract/
- Decision 3 — Validator overlay (skills/validation/overlay-bid-portal-procontract/) included
- Decision 4 — Sister implementation epic
- Decision 5 — Procontract access already solved (live demo feasible)

If ANY decision is missing or marked UNDECIDED, step 07 STOPS and escalates to step 01.

### Step 2: Sister Epic Creation (per decision_4)

Before creating any task beads:

```bash
# 1. Verify rc0 is doc-only (its target was the layer doc, which exists)
br show testproject1-rc0 --json | jq '.title, .description, .status'
# Expected: title contains "Epic: Layer 6", description references "Produce docs/strategy/layers/layer-6-request-pipeline-automation.md", status open

# 2. Close rc0 with reason
br close testproject1-rc0 --reason "Layer doc produced; implementation tracked in sister epic"

# 3. Create the sister implementation epic
# NOTE: `br create` uses `--labels` (plural, comma-separated value); singular `--label` is rejected by clap.
# `br list` / `br ready` use `--label` (singular, repeatable) — different flag semantics.
br create --type=epic --priority=P0 \
  --title="Epic: L6 Implementation — Request Pipeline Automation" \
  --labels l6 \
  --description "Implementation tracking epic for Layer 6 (Request Pipeline Automation) per docs/strategy/layers/layer-6-request-pipeline-automation.md. Children are Phase 1-N task beads created by pipemaxxer step 07. Exit criteria: all 11 DoD criteria green (spec §2.1) + live demo passes (DoD #8)."

# 4. Capture the new epic ID
NEW_EPIC_ID=$(br list --label l6 --type epic --json | jq -r '.issues[] | select(.title | contains("Implementation")) | .id')
echo "New implementation epic: $NEW_EPIC_ID"

# 5. Write to manifest (not status.yaml yet — that's step 10)
# Append to docs/pipeline/l6-request-pipeline-automation/manifest.yaml under:
#   existing_implementation_epic: <NEW_EPIC_ID>

# 6. Bulk-sync (use the wrapper, NOT raw `br sync`)
bin/br-sync && git add .beads/ && git commit -m "l6-epic: open implementation sister epic ${NEW_EPIC_ID}; close rc0 doc epic" && git push
```

### Step 3: Invoke `/beads-from-sources`

Run `/beads-from-sources`.

**Inputs:**
- Spec: `docs/strategy/layers/layer-6-request-pipeline-automation.md`
- TDD: `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md`
- Strategy: `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md`
- Bootstrap: `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md`
- Parent epic: `<NEW_EPIC_ID>` (from Step 2)
- Bead prefix: `l6`
- Bead label: `layer-6`

**Output artifacts:**
- Beads in `.beads/` (via `br create` for each task)
- `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bulk-import.md` — human-readable bulk import doc
- `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-bead-alias-map.json` — bead-ID → spec-FR mapping + test-ID mapping + phase assignment

### Step 4: Feature-Specific Bead Authoring Guidance

For every bead:

1. **Title format:** `l6-{phase}-{slug}` (e.g., `l6-phase1-errorcode-extension`)
2. **Body sections (verbatim from skill output, no improvisation):**
   - `### Spec Reference` — quote the FR text from the spec (verbatim, not paraphrased)
   - `### TDD Tests` — list every L6-T-* ID this bead's implementation makes pass
   - `### Files to Create/Modify` — every path the bead touches (absolute or repo-relative). **CRITICAL: bin/br-claim parses this section to compute file reservations. If a bead lacks this section, claim will fail or require --paths override.**
   - `### Acceptance Criteria` — bullet list, every criterion testable
   - `### Dependencies` — list of bead IDs this bead depends on (use `br dep add`)
   - `### Phase` — phase number from strategy
3. **Each bead is self-contained** — an agent can implement from the bead alone, without re-reading the spec
4. **File reservation predictability** — every `Files to Create/Modify` path is exact, not "and related files"
5. **Test ID ownership is 1:1** — exactly one bead owns each L6-T-* ID (no shared ownership; no orphan tests)

### Step 5: Phase Distribution (expected ranges per strategy)

Based on the recommended 7-phase shape:

| Phase | Name | Expected bead count |
|-------|------|---------------------|
| 1 | Foundation (ErrorCode + `packages/bidding/bid-scope-types.ts` + contracts-schema additions) — types are Phase 1 because Phase 4 portal skill + Phase 2 IPC handler consume them | 3-6 |
| 2 | Daemon Runtime (jobs/runner dispatch + ipc/handler + main.rs CLI) | 4-7 |
| 3 | Bid Workflow State Machine (Rust enforcement + invariants + readiness gate) | 5-8 |
| 4 | Portal Skill (skills/bid-portal-procontract/ + canary harness + golden journey) | 6-10 |
| 5 | Validator Overlay + packages/bidding workflow files (overlay-bid-portal-procontract + workflow-registry + procontract-workflow + company-profile-schema + 3 test files + package config; bid-scope-types.ts already landed in Phase 1) | 6-10 |
| 6 | Desktop + Web UI (BidWorkflow tab, status badges, review panels, RLS-bounded web view, e2e tests) | 6-10 |
| 7 | Live Demo + Integration + Calibration Validation (first demo REMOTE_DRAFT → draft_saved; second demo FORMAL_SUBMISSION; calibration corpus runs; final quality gate sweep) | 4-7 |

**Total expected: 34-58 beads** + 6-8 phase review-gate beads + the implementation epic = ~40-67 beads. Comparable to L4 (56) and L5 (61).

If your final count is < 30, beads are too coarse — split. If > 70, too fine — merge sub-tasks.

### Step 6: Bulk-Import Doc Authoring

The bulk-import doc (`l6-request-pipeline-automation-bulk-import.md`) is the human-readable
companion. Every bead in it is identical to the `br show <id> --json` content but
formatted as Markdown sections. Pattern:

```markdown
## l6-phase1-errorcode-extension
**Type:** task
**Priority:** P0
**Parent:** <NEW_EPIC_ID>
**Phase:** 1 — Foundation
**Labels:** layer-6, l6, phase-1

### Spec Reference (L6-FR-3.X.Y from spec)
> [quoted FR text]

### TDD Tests Owned
- L6-T-error_codes-001
- L6-T-error_codes-002
- L6-T-error_codes-003
- ... (one bullet per L6-T-* ID this bead makes pass)

### Files to Create/Modify
- crates/daemon/src/errors/manifest.rs  (extend ErrorCode enum + as_str() arm)
- crates/daemon/src/errors/markdown.rs  (clippy-surfaced exhaustive match propagation)
- crates/daemon/src/validation/policy.rs  (clippy-surfaced exhaustive match propagation)
- crates/daemon/src/validation/executor.rs  (clippy-surfaced exhaustive match propagation)
- crates/daemon/src/tracing/structured.rs  (clippy-surfaced exhaustive match propagation)
- crates/daemon/src/ipc/handlers/*.rs  (clippy-surfaced exhaustive match propagation, ~3-5 files)

### Acceptance Criteria
- [ ] All TDD-specified NEW ErrorCode variants present (count is TDD-determined — typically 3-6 after auditing existing codes for redundancy; see TDD's ErrorCode-extension test plan)
- [ ] as_str() arm returns the snake-case version (e.g., "BID_PORTAL_CHANGED")
- [ ] cargo clippy --all-targets -- -D warnings passes
- [ ] Round-trip test (parse_error_code(variant.as_str()) == Ok(variant)) passes for every NEW variant (count per TDD)
- [ ] All three sites updated: enum body (lines 20-64), as_str() impl (lines 65-115), parse_error_code() (line ~365)
- [ ] No existing tests broken

### Dependencies
- (none — phase 1 root)
```

### Step 7: Bead Sync + Commit

After all beads are created:

```bash
# Export to JSONL + normalize timestamps
bin/br-sync

# Stage + commit
git add .beads/issues.jsonl
git add docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bulk-import.md
git add docs/pipeline/l6-request-pipeline-automation/artifacts/l6-bead-alias-map.json
git commit -m "l6-pipeline: create L6 implementation beads from 3 SOT (spec + TDD + strategy)"
git push
```

### Step 8: Verify Output

After `/beads-from-sources` completes + commit pushed:
- [ ] Bead inventory: `br list --label l6 --json | jq '.issues | length'` matches expected total (~40-67 including epic + review gates)
- [ ] Every L6-FR-X.Y.Z appears in `l6-bead-alias-map.json` → at least one bead
- [ ] Every L6-T-* test ID is owned by exactly one bead in `l6-bead-alias-map.json`
- [ ] Every bead's `Files to Create/Modify` section is non-empty (bin/br-claim will fail otherwise)
- [ ] Every bead has explicit dependencies (or is a phase root with no dependencies)
- [ ] No bead is empty / TODO / placeholder
- [ ] `br lint` passes (no broken dependencies, no orphan beads)
- [ ] Sister implementation epic exists and is the parent for all task beads
- [ ] rc0 doc epic is closed

### Step 9: Follow-Up Pass (Sub-Optimization)

> ULTRATHINK. After bead creation, re-read with fresh eyes.

1. **Open the bulk-import doc.** Walk every bead. For each:
   - Is `Files to Create/Modify` exact? No "and related files"?
   - Does the file path actually exist (for "modify" beads) or correctly belong in a new directory (for "create" beads)?
   - Is the AC bullet list testable, not vague?
   - Does the bead reference the right phase (matches strategy)?

2. **Open `l6-bead-alias-map.json`.** Verify:
   - Spec FR → bead mapping at 100% (every FR has at least one bead)
   - Test ID → bead mapping at 100%, 1:1 (every test ID has exactly one owner)
   - Phase assignment matches strategy

3. **Open `crates/daemon/src/errors/manifest.rs`.** Precise boundaries (verified 2026-05-21): enum body lines 20-64 (~45 variants); `impl ErrorCode { fn as_str() ... }` lines 65-115; `parse_error_code()` at line ~365. Verify the ErrorCode extension bead lists the TDD-determined NEW variant set (typically 3-6 after auditing existing codes for redundancy — see TDD's plan), with snake_case names, and updates ALL THREE locations (enum body + as_str() + parse_error_code).

4. **Open `crates/daemon/src/main.rs` around lines 1513-1517 and find the job-kind dispatch** (`"extract_document_structure" => run_extract_document_structure_job(...)` and `"extract_document_semantics" => run_extract_document_semantics_job(...)`). The dispatch is in **main.rs**, NOT `jobs/runner.rs`. Verify the Phase 2 dispatch-arm bead specifies the exact insertion point in main.rs and notes the existing handler function patterns at lines 1529 + 1836 as templates.

5. **Open `crates/daemon/src/ipc/handlers/jobs.rs` and read `trigger_ingestion` at line 45 + `trigger_enrichment` at line 95.** These are the exact reference patterns. Verify the Phase 2 `trigger_bid_workflow` bead specifies the readiness-gate pre-check happens BEFORE job enqueue and references both existing handlers as templates.

6. **Open `skills/procurement-ingestion/procontract-register-interest/SKILL.md`.** Verify the Phase 4 bid-portal-procontract skill bead references this as the REFERENCE PATTERN.

7. **Open `skills/validation/overlay-supplier-enrichment/SKILL.md` frontmatter.** Verify the Phase 5 validator overlay bead references this frontmatter shape.

8. **Open `packages/contracts-schema/src/index.ts`.** Verify the Phase 1 or Phase 5 bead correctly attributes type ownership (BidScopePayload / SideEffectClass / BidWorkflow live in `packages/bidding/`, NOT contracts-schema; re-export only if needed).

9. **Walk Phase 6 UI beads.** Are the component file paths in `packages/ui/src/` exact? Are the desktop tab integration files in `apps/desktop/src/` exact? Are the web RLS verification tests in `apps/web/e2e/` exact?

10. **Walk Phase 7 live demo beads.** Are the operator-action gates explicit (CDP profile + opportunity ref + sideEffectCap=REMOTE_DRAFT for first run + separate FORMAL_SUBMISSION authorization)?

11. **Run `br lint`.** Verify zero issues.

12. **Scope-drift scan:** grep the bulk-import doc for "post-MVP", "stretch", "nice to have", "deferred". Zero hits.

13. **Phase coverage equality:** count beads per phase. If Phase 7 has 2 beads vs Phase 4's 10, investigate — Phase 7 must have the same depth (it's the M6 capstone). If Phase 1 has 1 bead, the ErrorCode propagation work was lumped — split.

14. **Sister epic verification:** `br show <NEW_EPIC_ID> --json` shows all task beads as children.

### Step 10: Scope-Preservation Check (MANDATORY — BLOCKING)

- [ ] rc0 epic closed (decision_4)
- [ ] Sister implementation epic created with new ID
- [ ] Every L6-FR has ≥1 bead
- [ ] Every L6-T has exactly 1 owning bead
- [ ] Every bead has Files to Create/Modify section (non-empty, exact paths)
- [ ] Every bead has testable ACs
- [ ] Every bead has correct dependencies
- [ ] No bead labeled "post-MVP" / "stretch" / "nice to have"
- [ ] Validator overlay bead exists (Phase 5)
- [ ] Live demo bead exists (Phase 7)
- [ ] Phase distribution is balanced (no phase has < 3 beads except possibly Phase 1 which CAN be small)
- [ ] Total bead count: 30-70 inclusive
- [ ] `br lint` passes
- [ ] bin/br-sync + commit + push completed

---

## Output

| File | Path |
|------|------|
| Beads | `.beads/issues.jsonl` (via br) |
| Bulk-import doc | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bulk-import.md` |
| Bead alias map | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-bead-alias-map.json` |

---

## Key Reminders

- **rc0 → sister epic** (per decision_4) — first action before any task beads
- **Every bead's Files section is parsed by bin/br-claim** — be exact
- **Every bead is self-contained** — implementable from bead alone
- **3-6 NEW ErrorCode variants** (TDD-determined count) drive a single Phase 1 bead (not N beads per variant)
- **Validator overlay** is a Phase 5 bead (decision_3 made it L6 deliverable)
- **Live demo** is a Phase 7 bead with explicit operator-action gates
- **NO scope drift** — every spec FR has at least one bead
- **Use `bin/br-sync`** (NEVER raw `br sync`)
- Match L5/L4 bead density (~40-60 leaf beads)
