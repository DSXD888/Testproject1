# L4 Shared Visibility — 09 · Bead Audit (Spec Drift Detection)

## First Action

Invoke the `/bead-plan-audit` skill.

---

## Your Mission

You are performing a **dedicated, systematic audit** of all L4 beads. This is the LAST GATE before execution. Your job is to verify **zero spec drift** — every spec L4-FR is covered, every TDD L4-T test ID is assigned, no silent scope cuts, no scope-drift language anywhere in the bead corpus.

If `/bead-plan-audit` returns the verdict "drift detected", the audit FAILS and beads must be fixed before kickoff (step 10) can run.

---

## Source Documents

### Beads (TARGET — audit EVERY bead)
```bash
br list --label layer-4 --json     # Full L4 inventory
br ready --label layer-4 --json    # Beads with no upstream blockers
br show <bead-id>                  # Read EVERY bead's full description
br dep list <bead-id>              # Verify dependency edges
br lint                            # Static lint
```

Read EVERY bead. Not a sample — every single one.

### Verify Against (SOURCES OF TRUTH)
```
docs/strategy/layers/layer-4-shared-visibility.md                              — Spec (SOT #1) — every L4-FR must have a bead
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md       — TDD (SOT #2) — every L4-T must be claimed
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md  — Strategy (SOT #3) — phase ordering must match
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md — Bootstrap — environment prerequisites
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md — Bulk-import doc (mirror of bead corpus)
```

### Codebase (for verifying bead file/type references are accurate)
```
crates/daemon/src/db/sqlite.rs                              — confirm sync.rs path
crates/daemon/src/db/checkpoints.rs                         — pattern for db modules
crates/daemon/src/validation/policy.rs                      — 959 lines (chokepoint)
crates/daemon/src/validation/executor.rs                    — 1217 lines (chokepoint)
crates/daemon/src/jobs/runner.rs                            — 1027 lines (chokepoint)
crates/daemon/src/main.rs                                   — 1.08 MB (chokepoint, Phase 6 only)
crates/daemon/Cargo.toml                                    — net-new HTTP client
package.json (root)                                         — workspaces edit
migrations/                                                 — flat sequence
infra/supabase/                                             — net-new
apps/web/                                                   — net-new
skills/validation/base/                                     — template
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**This is the BLOCKING audit.** If any of the following are true, the audit fails and the pipeline cannot proceed to kickoff (step 10):
- Any spec L4-FR has zero corresponding beads
- Any TDD L4-T test ID is unowned
- Any DoD criterion has zero beads delivering it
- Any of the 14 invariants has zero beads establishing it
- Any bead body contains red-flag scope-drift language in a load-bearing position
- Phase coverage is unbalanced beyond reasonable explanation
- A chokepoint file has >1 bead simultaneously editing it
- Dependency edges are missing or wrong
- `br lint` reports errors

---

## Audit Methodology

The `/bead-plan-audit` skill handles the detailed methodology. Add these L4-specific checks:

### Feature-Specific Audit Targets

#### 1. Sync Engine Coverage (Push + Pull)
- All 11 push-engine constitutional invariants (§1.9) covered
- All 4 equal-version retry cases covered as separate beads or AC items
- Hi-water fenced pull tested across all 3 pull domains
- Pull-before-push ordering explicit
- Tombstone visibility for daemon role
- Writer fencing token tested with 2-daemon scenario
- Schema epoch guard + minimum epoch-repair reproject path

#### 2. CAS + Command Ledger Coverage
- CAS RPC bead exists with 4 cases (apply, conflict, idempotent replay, command-unknown-after-GC)
- Command ledger durability separate from `operator_decision_heads.command_id` (§1.5)
- Pending intent reconciliation with latest-per-process coalescing
- Decision conflict IPC event (§5.5 cross-domain note → L2 expectation)

#### 3. Validation Enforcement Coverage
- Provisional promotion buffer mechanism named (table or directory) and bead exists
- Per-slice enforcement readiness gate with all 7 prerequisites
- All 4 demotion triggers as testable beads or AC items
- AGENT_SUSPECT_OUTPUT error type in `errors/manifest.rs`
- Quarantine flow: rejected results stay local, error manifest, stage head NOT advanced
- All 4 validator overlays exist (procurement-ingestion, supplier-enrichment, document-acquisition, product-extraction)
- Each overlay has 6 standard files (SKILL.md + 4 schemas + 1 references)

#### 4. Calibration Harness Coverage
- 3 corpora (clean real, weird-but-valid, mutated bad) with sample-size targets
- All 9 mutation families exercised
- Pairwise challenge sets (A=clean, B=corrupted)
- Wilson-bound precision math
- Rule-of-three false-pass math
- Cross-model audit sampling
- Override feedback loop (every override → calibration case + incident-cluster + mutation template)

#### 5. Web Portal Coverage
- All 7 files have beads (or are grouped in multi-file beads)
- 3 Broadcast topic schemes per page
- Three-state degraded-mode UX (green/amber/red with thresholds 90s, 5min)
- Viewer-credentials-only enforcement test
- Playwright e2e covers RLS boundary
- `apps/web/` workspace integration into root `package.json`
- `packages/ui` lint invariant preserved (no desktop/tauri imports)

#### 6. RLS + Redaction Coverage
- 4 roles × 7 categories = 28 RLS policy assertions
- All 6 summary projections have redaction tests in `crates/daemon/tests/redaction_boundary.rs`
- Daemon-pull tombstone visibility (NO `deleted_at IS NULL` filter for daemon role on remote-owned tables)
- Direct UPDATE on `operator_decision_heads` blocked at RLS for operator role

#### 7. Cross-Layer Touch Points
- L0 cross-domain note: `commit_group_id`, `writer_epoch` columns reflected in Phase 1 schema bead
- L2 cross-domain note: 3-state FreshnessBadge upgrade in Phase 5
- L6 cross-domain note: remote config staleness classes safety-critical TTL in Phase 4 or Phase 6

### Scope Drift Detection (MANDATORY — BLOCKING DIMENSION)

**This is the single most important audit dimension.**

#### Red-Flag Language Scan

Run:
```bash
br list --label layer-4 --json > /tmp/l4-beads-audit.json
grep -inE 'post[ -]?mvp|phase 2\+|stretch goal|nice[ -]to[ -]have|deferred|out[ -]of[ -]scope|future iteration|simplified for now|simplified for initial|can be added later|to be added later|eventually|someday|for now|optional|if time permits|we may|we could|might add' /tmp/l4-beads-audit.json
```

Every match must be inspected. Hits in bead descriptions / ACs that describe in-scope L4 features = BLOCKING failure. Edit the bead and re-sync.

#### Coverage Verification

For EVERY feature/subsystem in the spec:
- Identify which bead(s) implement it
- If ANY spec feature has ZERO corresponding beads → BLOCKING failure
- Create missing beads before proceeding

Build the coverage matrix:

| Spec Feature | Bead(s) | Phase | Status |
|--------------|---------|------|--------|
| §1.3 Push protocol | l4-p2-push-dispatcher, l4-p2-outbox-writer, ... | 2 | ✓ |
| §1.4 Pull protocol | l4-p3-pull-cursor-*, l4-p3-hi-water-fence | 3 | ✓ |
| §1.5 CAS | l4-p3-cas-rpc, l4-p3-command-ledger-* | 3 | ✓ |
| §1.6 Active-writer lease | l4-p2-active-writer-lease, l4-p2-writer-fencing | 2 | ✓ |
| §1.7 Schema epoch guard | l4-p2-schema-epoch-guard | 2 | ✓ |
| §1.8 Degraded mode | l4-p5-three-state-degraded-mode-ux + others | 5 | ✓ |
| §1.9 Invariants (each) | (verify each invariant individually) | varies | (must check) |
| §2.1 Push-owned projections (20) | l4-p1-supabase-migrations-projection | 1 | ✓ |
| §2.2 Summary tables (6) | l4-p1-supabase-migrations-summaries + l4-p1-supabase-migrations-decision-summaries | 1 | ✓ |
| §2.3 Remote-owned (3) | l4-p1-supabase-migrations-remote-owned | 1 | ✓ |
| §2.5 RLS matrix (4 roles × 7 categories) | l4-p1-supabase-migrations-rls + Phase 6 redaction tests | 1+6 | ✓ |
| §2.6 Migrations (8) | 8 separate beads | 1 | ✓ |
| §2.7 Cross-authority crossings (4) | l4-p6-cross-authority-provenance | 6 | ✓ |
| §3.1 Web files (7) | 7 Phase 5 beads | 5 | ✓ |
| §3.2 Broadcast invalidation | l4-p5-broadcast-subscription-manager | 5 | ✓ |
| §3.3 3-state degraded UX | l4-p5-three-state-degraded-mode-ux | 5 | ✓ |
| §3.4 Enforcement transitions | l4-p4-executor-quarantine-path, l4-p4-policy-engine-per-slice-readiness | 4 | ✓ |
| §3.5 Validator overlays (4) | 4 beads | 4 | ✓ |
| §3.6 Calibration harness | ~7 beads | 4 | ✓ |
| §3.7 viewer_sync_status | l4-p1-supabase-migrations-summaries (DDL) + l4-p2-viewer-sync-status-update (write path) | 1+2 | ✓ |
| §4.1 DoD (18) | walk every row | varies | (must check) |
| §5.2 Stub: epoch-repair reproject | l4-p2-epoch-repair-reproject-min-path | 2 | ✓ |
| §5.2 Stub: logical consistency auditor | l4-p6-logical-consistency-auditor-stub | 6 | ✓ |

If any cell is empty / `?` / "no bead found", that's a BLOCKING gap.

#### Phase Coverage Equality

Count beads per phase. L3 baseline: 5/5/5/10/4/7. L4 expected: 5-8/8-12/6-8/10-15/7-10/6-10. If any phase is anomalously thin, investigate. Phase 6 should NOT be a junk drawer; it should have substantive beads (main.rs wiring, redaction tests, two-user verification, DoD walkthrough, cross-authority provenance, logical consistency stub).

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)

1. `docs/strategy/layers/layer-4-shared-visibility.md` — **READ FULLY**
2. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — **READ FULLY**
3. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` — **READ FULLY**
4. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md` — **READ FULLY**
5. `br list --label layer-4 --json` — **STORE** to `/tmp/l4-beads-audit.json`
6. Walk EVERY bead via `br show <id>` — read descriptions, ACs, deps, labels

### Step 1: Invoke `/bead-plan-audit`

**Inputs:**
- Beads: `br list --label layer-4 --json`
- Spec: `docs/strategy/layers/layer-4-shared-visibility.md`
- TDD: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md`
- Strategy: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md`

**Key arguments/context:**
- Bead prefix: `l4`
- Phase labels: `l4-phase-1` through `l4-phase-6`
- Chokepoint files (no parallel edits): `main.rs`, `runner.rs`, `policy.rs`, `executor.rs`, `decisions.rs`, `errors/manifest.rs`, daemon `Cargo.toml`, root `package.json`, `packages/ui/freshness-badge.tsx`
- Net-new files L4 owns: `crates/daemon/src/db/sync.rs`, `migrations/0012_l4_sync_state.sql` (or chosen NNNN), `infra/supabase/{config.toml, migrations/0001-0008}`, `apps/web/{...}` (7 files), `skills/validation/{4 overlays}/` (24 files)
- Quality bar: L3 audit pass

### Step 2: Feature-Specific Checks

While the audit runs:

- Verify the audit reports "drift detected" if ANY scope-drift language is found in any bead
- Verify the audit reports a coverage matrix (spec L4-FR → bead) with no empty cells
- Verify the audit reports a TDD coverage matrix (L4-T → bead) with every test ID claimed
- Verify the audit reports phase coverage counts and flags imbalance
- Verify the audit checks dependency edges — no cycles, no orphans, no missing parents
- Verify the audit confirms `br lint` is clean
- Verify the audit confirms chokepoint discipline (1 bead per chokepoint file across all phases)

### Step 3: Apply Fixes (if audit fails)

If the audit reports failures:
1. **Coverage gaps:** create missing beads via `br create`. Add deps via `br dep add`. Apply labels via `br label add`. Re-sync via `bin/br-sync`.
2. **Scope-drift language:** edit affected beads via `br update <id>` or `br edit <id>`. Remove the language. Re-sync.
3. **Chokepoint violations:** consolidate beads via merging descriptions; close redundant ones via `br close <id> --reason "Merged into <other-id>"`. Re-sync.
4. **Dependency violations:** add/remove edges via `br dep add` / `br dep rm`. Re-sync.

After every fix:
```bash
bin/br-sync
git add .beads/
git commit -m "beads: l4 audit fixes — <description>"
git pull --rebase
git push
```

Re-run the audit until it returns CLEAN.

### Step 4: Verify Audit Output

After the audit completes (CLEAN verdict):

- [ ] Audit report saved (file path or in-session output)
- [ ] Coverage matrix shows 100% L4-FR → bead mapping
- [ ] TDD test ID coverage shows 100% L4-T → bead mapping
- [ ] Phase coverage is balanced
- [ ] No scope-drift language detected
- [ ] No chokepoint violations
- [ ] No dependency violations
- [ ] `br lint` clean

---

## Follow-Up Pass (Run After Audit Clean)

> After the audit is complete and all fixes applied:

ULTRATHINK.

Re-read your own fixes. Did fixing one bead break another's dependencies?

1. **Re-run the coverage matrix.** Is it still 100% after your fixes?

2. **Verify bead count matches strategy phase task count.** If the strategy said "Phase 4 has ~12 beads" and you ended up with 6, did you silently scope-cut, or was the strategy estimate too high?

3. **Verify ZERO scope-drift language remains in any bead.** Re-grep:
   ```bash
   br list --label layer-4 --json | jq '[.issues[] | .description + " " + (.title // "")] | join("\n")' | grep -iE 'post[ -]?mvp|phase 2\+|stretch|nice[ -]to[ -]have|deferred|future iteration|simplified for now|can be added later|eventually|for now'
   ```
   If the grep returns nothing, you're clean.

4. **Verify every spec feature has at least one bead.** Build the coverage diff:
   ```bash
   br list --label layer-4 --json | jq '[.issues[] | .description] | join(" ")' > /tmp/l4-bead-bodies.txt
   grep -oE 'L4-FR-[0-9]+\.[0-9]+(\.[0-9]+)?' /tmp/l4-bead-bodies.txt | sort -u > /tmp/l4-fr-in-beads.txt
   grep -oE 'L4-FR-[0-9]+\.[0-9]+(\.[0-9]+)?' docs/strategy/layers/layer-4-shared-visibility.md | sort -u > /tmp/l4-fr-in-spec.txt
   diff /tmp/l4-fr-in-spec.txt /tmp/l4-fr-in-beads.txt
   ```
   Empty diff = clean.

5. **Verify every TDD test ID is owned by exactly one bead.** Same exercise for L4-T IDs.

6. **Walk the chokepoint discipline check.** For `main.rs`, `runner.rs`, `policy.rs`, `executor.rs`, `decisions.rs`, `errors/manifest.rs`, daemon `Cargo.toml`, root `package.json`, `packages/ui/freshness-badge.tsx` — verify exactly one bead lists each as a target.

7. **Run `br lint`** one final time. Address all warnings.

8. **Run `bin/br-sync && git status`.** Confirm `.beads/` changes are staged and pushed.

9. **Verify the existing epic `testproject1-4zq`** is properly hooked up. `br show testproject1-4zq` and `br dep list testproject1-4zq` — children should be visible.

10. **Final readiness check:** `br ready --label layer-4 --json` returns a non-empty list of Phase 1 beads (or Phase 1+2+3 beads if parallelism is enabled). These are the beads agents will claim first in step 10 kickoff.

---

## Audit Verdicts

The audit returns one of these verdicts:

- **CLEAN** — all coverage matrices 100%; no scope-drift language; chokepoint discipline holds; deps clean. Proceed to step 10 (kickoff generation).
- **DRIFT DETECTED** — at least one of: spec feature with no bead; test ID unclaimed; scope-drift language found; chokepoint collision; broken dep. **BLOCKING.** Apply fixes; re-run.
- **WARNINGS** — phase coverage imbalanced or anomalies that may not be drift. Investigate, document if acceptable, then proceed.
- **SPEC ERROR DETECTED** (rare escape hatch) — the spec itself contains a contradiction, impossibility, or factual error discovered only during audit (e.g., a §1.X requirement is in conflict with a §3.X requirement; or a §2.X schema clause references a column that no migration creates; or the migration filename in §1.10 conflicts with the one in §2.6 after step 01 had already resolved it). Fix beads CANNOT fix the spec. **Path:** stop the audit; do NOT proceed to step 10. Return to step 01 (validate-spec) with a documented "spec-error" exception list. Apply spec edits. Then regenerate downstream artifacts in order: re-run step 02 fresh-eyes audit, then step 04 if TDD assumptions changed, then step 06 if strategy ordering changed, then step 08 if beads must change. Then re-run this step (09). This is rare but the path must exist — the alternative (silently accepting a contradictory spec) violates the cardinal rule.

If `DRIFT DETECTED`, the pipeline does NOT proceed to kickoff until the audit returns CLEAN.
If `SPEC ERROR DETECTED`, the pipeline returns to step 01.
