# L4 Shared Visibility — 08 · Optimize Beads (Fresh-Eyes Audit)

## First Action

**No skill is invoked.** This is a manual fresh-eyes audit by an independent agent (NOT the agent that ran step 07). Walk every L4 bead with `br show <id>`, work through the 14 audit dimensions below, edit beads in place via `br update` / `br dep add` / `br label add` / `br close`, re-sync via `bin/br-sync`, then run the follow-up pass.

The dedicated `/bead-plan-audit` skill runs in step 09 — this step (08) is the lighter fresh-eyes pass that catches the obvious gaps before the formal audit.

---

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the L4 bead corpus created in step 07. Beads are the execution units agents will claim — they must be self-contained, well-scoped, and 100% covering the spec. After this step, beads enter the dedicated audit pass (`/bead-plan-audit` in step 09).

**You are reading and improving existing beads — not writing code.**

---

## What To Read (Do This First)

### The Beads to Audit (TARGETS)
```bash
br list --label layer-4 --json     # Full L4 inventory
br ready --label layer-4 --json    # Beads with no upstream blockers
br show <bead-id>                  # Individual bead detail (read EVERY bead)
br lint                            # Static lint over the corpus
```

Walk EVERY L4 bead. Do not sample.

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-4-shared-visibility.md                              — Spec (SOT #1)
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md       — TDD (SOT #2)
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md  — Strategy (SOT #3)
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md — Bootstrap
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md — Bulk import doc (created in step 07)
```

### Codebase (GROUND TRUTH for verifying file/type references in beads)
```
crates/daemon/src/db/sqlite.rs                              — confirm sync.rs path
crates/daemon/src/validation/policy.rs                      — 959 lines (chokepoint)
crates/daemon/src/validation/executor.rs                    — 1217 lines (chokepoint)
crates/daemon/src/jobs/runner.rs                            — 1027 lines (chokepoint)
crates/daemon/src/main.rs                                   — 1.08 MB (chokepoint, Phase 6 only)
crates/daemon/Cargo.toml                                    — net-new HTTP client
package.json (root)                                         — workspaces; lint rule banning desktop/tauri imports from packages/ui
migrations/                                                 — flat sequence
infra/supabase/                                             — net-new
apps/web/                                                   — net-new
skills/validation/base/                                     — template (6 files)
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bulk-import.md   — L3 bulk import (~69KB)
.beads/beads.db                                                                            — existing l3-* beads as a quality reference
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**100% coverage is mandatory.** Every spec L4-FR has ≥1 bead. Every TDD L4-T is owned by exactly one bead. Every DoD criterion has ≥1 bead delivering it. Every invariant has ≥1 bead establishing it.

**Phase coverage equality:** later phases get the same depth as early phases. A phase with 1-2 beads while others have 8-12 is a scope-drift signal.

---

## Think Independently — 14 Audit Dimensions

### 1. Spec L4-FR Coverage
Walk every L4-FR-X.Y.Z in the spec. For each, identify which bead(s) implement it. Make a table:

| L4-FR | Title | Bead(s) | Phase |
|------|-------|--------|------|
| L4-FR-1.3.1 | Single-in-flight push batch | l4-p2-push-dispatcher | 2 |
| L4-FR-1.3.2 | Identical-envelope retries | l4-p2-push-dispatcher | 2 |
| ... | ... | ... | ... |

If any L4-FR has no bead, BLOCKING. Add it.

### 2. TDD L4-T Coverage
Walk every L4-T test ID in the TDD. For each, identify which bead claims it. Build the inverse:

| L4-T | Test File | Owning Bead | Phase |
|------|-----------|-------------|------|
| L4-T-sqlite_supabase_sync-001 | sqlite_supabase_sync.rs | l4-p2-outbox-writer | 2 |
| ... | ... | ... | ... |

If any L4-T is unowned (or claimed by multiple beads), BLOCKING. Reassign.

### 3. Bead Self-Containment
Spot-check 10 random beads. For each, ask:
- Does the bead body contain file paths (e.g., `crates/daemon/src/db/sync.rs`)?
- Does it contain type/function names (e.g., `OutboxEntry`, `apply_push_batch`)?
- Does it embed the spec L4-FR text VERBATIM (not paraphrased)?
- Does it list TDD L4-T test IDs that satisfy it?
- Does it have observable AC bullets (state changes, RPC responses, UI badges)?
- Could a fresh agent implement this bead from the bead body alone, without re-reading the spec?

If any bead fails, mark for revision.

### 4. Single-Edit Chokepoint Discipline
For each chokepoint file, count how many beads list it as a target:
- `crates/daemon/src/main.rs` — must be 1 (Phase 6 wiring)
- `crates/daemon/src/jobs/runner.rs` — must be 1 (Phase 4 runner extension)
- `crates/daemon/src/validation/policy.rs` — must be 1 (Phase 4 policy extension)
- `crates/daemon/src/validation/executor.rs` — must be 1 (Phase 4 executor extension)
- `crates/daemon/src/ipc/handlers/decisions.rs` — must be 1 (Phase 3 CAS conflict event)
- `crates/daemon/src/errors/manifest.rs` — must be 1 (Phase 4 AGENT_SUSPECT_OUTPUT extension)
- `crates/daemon/Cargo.toml` — must be 1 (Phase 1 dep-add)
- `package.json` (root) — must be 1 (Phase 1 workspaces edit)
- `packages/ui/freshness-badge.tsx` — must be 1 (Phase 5 three-state extension)

Counts > 1 = parallel-execution merge-conflict trap. Consolidate.

### 5. Dependency Edge Sanity
- Run `br lint` — must pass
- Walk Phase 1 beads — must have NO upstream deps (or only depend on epic)
- Walk Phase 2 beads — must depend on Phase 1 schema bead
- Walk Phase 3 beads — must depend on Phase 1 schema bead (parallel to Phase 2 OK)
- Walk Phase 4 beads — typically depend on Phase 1 + sometimes Phase 2/3 (executor depends on policy bead, runner depends on executor)
- Walk Phase 5 beads — depend on Phase 1 + apps/web workspace bead
- Walk Phase 6 beads — depend on most prior phases (it's the integration phase)

Look for surprising edges: if a Phase 2 bead depends on a Phase 5 bead, that's wrong (reverse direction). Investigate.

### 6. Phase Coverage Equality
Count beads per phase. Compare to L3 (5/5/5/10/4/7 = 36 task beads). For L4, expect 5-8/8-12/6-8/10-15/7-10/6-10 (40-65 beads total).
- If a phase has 1-2 beads while neighbors have 8-12, that phase was scope-cut. Investigate.
- If a phase has 25+ beads while neighbors have 5-8, beads in that phase may be too granular. Investigate.

### 7. Bead Title Convention
Every bead title should be `l4-p{N}-{slug}` (e.g., `l4-p1-sqlite-sync-migration`). Verify:
- All have `l4-` prefix
- All have phase number in title (helps `br ready` ordering)
- Slugs are descriptive
- No bead title is just `l4-p4-overlay` — must be `l4-p4-overlay-procurement-ingestion` etc.

### 8. Label Hygiene
Every L4 bead must have:
- `--label layer-4`
- `--label l4-phase-{N}`

Optional but recommended:
- `--label chokepoint` for beads editing single-edit chokepoint files
- `--label net-new-file` for beads creating new files (vs extending existing)
- `--label external-dep` for beads requiring external services (Supabase project provisioning, calibration corpus seeding)

### 9. Priority Distribution
- P0 beads: schema migration, HTTP client dep, apps/web workspace wiring, push dispatcher core, CAS RPC, executor quarantine path, main.rs wiring, redaction tests
- P1 beads: validator overlays, web pages, calibration harness, schema_epoch guard, lease, command ledger
- P2 beads: docs, optional polish (logical consistency stub if minimum, two-user verification doc)

If everything is P0, agents will collide. If nothing is P0, the schema bead might not be claimed first.

### 10. Acceptance Criteria Specificity
Walk 10 random beads' ACs. Each AC must be:
- Observable (state change visible somewhere)
- Testable (can be asserted in code or manual verification)
- Bridged to a TDD test ID

Bad ACs: "Push engine works correctly", "Outbox writer behaves as expected"
Good ACs: "Calling `db.write_with_outbox(entry)` creates exactly one row in `local_outbox` with same `commit_group_id` as the data write, verified by `L4-T-sqlite_supabase_sync-{N}`"

### 11. Sub-Epic / Epic Structure
Verify the existing epic `testproject1-4zq` is reflected. Either:
- All task beads parent (via dep) to `testproject1-4zq`; phases are labels
- 4-6 sub-epics exist (`l4-schema-foundation`, `l4-push-engine`, ..., `l4-integration`); task beads parent to sub-epics; sub-epics parent to `testproject1-4zq`

Whichever choice was made in steps 06-07 must be reflected. Confirm.

### 12. Bulk Import Doc Quality
The bulk-import doc at `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md` should:
- Have the same density as L3's (~69KB)
- List every bead with title, description, ACs, deps, labels
- Be a faithful representation of `br list --json`
- Be readable as a standalone doc (so a future archive run has the full inventory)

If the doc is much shorter than L3's, beads may be sparse.

### 13. Scope Drift Scan (BLOCKING)
For every bead, search the description and ACs for:
- `post-MVP` / `post mvp`
- `Phase 2+` (when used as scope-defer phrase, not as L4 phase 2 reference)
- `stretch goal` / `stretch-goal`
- `nice to have` / `nice-to-have`
- `optional`
- `deferred to L5/L6` (or any phasing that pushes L4 work to a later layer)
- `future iteration`
- `simplified for initial implementation`
- `can be added later` / `to be added later`
- `eventually`
- `for now`
- `we may`
- `if time permits`

Every hit in a load-bearing position = BLOCKING failure. Edit the bead and re-sync.

### 14. CalibrationHarness, Web Portal, RLS Beads
These three areas are most prone to under-scoping. Verify:

**Calibration harness (~7 beads expected):**
- 3 corpora seeding bead
- 9 mutation families bead (or split into individual mutation beads)
- Pairwise challenge sets bead
- Wilson-bound math bead
- Rule-of-three math bead
- Cross-model audit sampling bead
- Override feedback loop bead

**Web portal (~7-10 beads expected):**
- `apps/web/` skeleton + workspace integration bead
- Supabase client init bead
- Broadcast subscription manager bead
- 4 page beads (layout+redirect, contracts list, contracts detail, status)
- Three-state degraded-mode UX bead
- Playwright e2e bead

**RLS / Redaction (~3-5 beads expected):**
- `0004_rls_policies.sql` bead (Phase 1)
- Redaction tests bead (Phase 6 — covers all 6 summary projections)
- Daemon-pull tombstone visibility bead (Phase 3)
- Viewer-credentials boundary test in Playwright bead (Phase 5)

If any of these three areas has fewer beads than expected, that's a coverage gap.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
Run:
```bash
br list --json > /tmp/l4-beads.json
grep -iE 'post[ -]?mvp|phase 2\+|stretch goal|nice[ -]to[ -]have|deferred|future iteration|simplified for now|can be added later|eventually|for now|optional|if time permits|we may' /tmp/l4-beads.json
```

Every match must be inspected. Hits in bead descriptions / ACs that describe in-scope L4 features = BLOCKING. Edit the bead and re-sync.

### Coverage Verification

- [ ] Every L4-FR-X.Y.Z is owned by ≥1 bead (built table in dimension #1)
- [ ] Every L4-T-* is owned by exactly one bead (built inverse table in dimension #2)
- [ ] Every of the 18 DoD criteria has ≥1 bead delivering it
- [ ] Every of the 14 invariants has ≥1 bead establishing it
- [ ] Every of the 4 validator overlays has a bead
- [ ] Every of the 7 web files has a bead (or is grouped in a multi-file bead)
- [ ] Every of the 8 Supabase migrations has a bead
- [ ] All 9 mutation families covered (one or more beads)
- [ ] Calibration harness covered (~7 beads)
- [ ] RLS + redaction covered (~3-5 beads)
- [ ] Two-user verification bead exists (Phase 6)
- [ ] Schema hash CI gate bead exists (Phase 1 or 6)
- [ ] Logical consistency auditor stub bead exists (Phase 6, minimum stub per §5.2)

### Drift Checklist

- [ ] No bead has scope-drift language in description / ACs
- [ ] No phase has 1-2 beads while neighbors have 8-12
- [ ] No chokepoint file has >1 bead concurrently editing it
- [ ] Every bead has explicit dependency edges to upstream beads
- [ ] Every bead has labels: `layer-4` + `l4-phase-{N}`
- [ ] Bead corpus passes `br lint`
- [ ] Bulk-import doc density matches L3 quality bar
- [ ] Existing epic `testproject1-4zq` properly reflected

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

ULTRATHINK.

Re-read the beads after your fixes. Did fixing one bead break another?

1. **Did your edits to chokepoint beads** (e.g., consolidating two `main.rs` editing beads into one) introduce new dependency edges? Walk `br dep list <consolidated-bead>` — verify all upstream deps are correct.

2. **Did your additions of missing-coverage beads** create circular dependencies? Run `br lint` again.

3. **Did your scope-drift language removal** weaken AC specificity? Re-read the affected beads — ACs should still be testable and observable.

4. **Open `crates/daemon/src/main.rs`.** The Phase 6 wiring bead must list: pull-before-push startup ordering, lease acquisition, schema_epoch check, viewer_sync_status update wiring, pull cursor advancement, push dispatcher invocation. Walk the bead body — does it cover all? If not, add to the bead.

5. **Open `crates/daemon/src/validation/executor.rs` (1217 lines).** The Phase 4 executor extension bead must list: provisional staging write path, AGENT_SUSPECT_OUTPUT emission on rejection, dry-run vs enforcement-active branch logic, depend on policy.rs extension. Walk the bead body — does it cover all?

6. **Open `apps/desktop/src/pages/ContractList.tsx`** (and ContractDetail, DaemonStatus). The Phase 5 web pages should reuse `packages/ui` components that already exist. Walk a Phase 5 web page bead — does it explicitly say "reuse `ContractCardFrame` from `packages/ui`" or does it reinvent the wheel? If reinvention, that's a problem; the bead should explicitly require reuse.

7. **Open `package.json` (root).** Confirm the lint script's `rg` rule still bans `@tauri-apps/api`/`apps/desktop`/`crates/daemon`/`supabase`/`transport` imports from `packages/ui`. The Phase 1 workspace bead must NOT weaken this rule. Walk that bead — does its body explicitly preserve the lint invariant?

8. **Walk every bead's "files modified" annotation.** Build a histogram: which files have the most beads pointing at them?
   - `crates/daemon/src/db/sync.rs` — likely 8-12 beads (it's a new file with many features)
   - `infra/supabase/migrations/*` — 8 beads (one per migration)
   - `skills/validation/{4 overlays}/*` — 24 beads (4 × 6 files)
   - `apps/web/src/{app,lib}/*` — 7-10 beads
   - `crates/daemon/tests/*` — 4 beads (one per test file)

   Anomalies (e.g., 20 beads pointing at `runner.rs`) signal over-decomposition.

9. **Re-scan for scope-drift language** one final time. Search bead descriptions and ACs (extract from `br list --json` and grep). Any hits in load-bearing positions = BLOCKING.

10. **Run `br lint`** one final time. Address all warnings.

11. **Run `bin/br-sync && git status`.** Confirm `.beads/` changes are staged. Then `git add .beads/ && git commit -m "beads: l4 audit fixes" && git pull --rebase && git push`.

12. **Verify the epic structure once more.** `br show testproject1-4zq` — does it reference the children created? `br dep list testproject1-4zq` — are sub-epics or task beads parented? If empty, the parent edges may not have been added; fix.

13. **Final coverage sanity check.** Run:
    ```bash
    br list --label layer-4 --json | jq '[.issues[] | .description] | join("\n")' > /tmp/l4-descs.md
    grep -oE 'L4-FR-[0-9]+\.[0-9]+(\.[0-9]+)?' /tmp/l4-descs.md | sort -u > /tmp/l4-fr-in-beads.txt
    grep -oE 'L4-FR-[0-9]+\.[0-9]+(\.[0-9]+)?' docs/strategy/layers/layer-4-shared-visibility.md | sort -u > /tmp/l4-fr-in-spec.txt
    diff /tmp/l4-fr-in-spec.txt /tmp/l4-fr-in-beads.txt
    ```
    If the diff shows L4-FR IDs in the spec but not in any bead, those are unimplemented features. BLOCKING.

    Same exercise for L4-T:
    ```bash
    grep -oE 'L4-T-[a-z_]+-[0-9]+' /tmp/l4-descs.md | sort -u > /tmp/l4-t-in-beads.txt
    grep -oE 'L4-T-[a-z_]+-[0-9]+' docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md | sort -u > /tmp/l4-t-in-tdd.txt
    diff /tmp/l4-t-in-tdd.txt /tmp/l4-t-in-beads.txt
    ```
    Any TDD test ID not claimed by a bead = BLOCKING.
