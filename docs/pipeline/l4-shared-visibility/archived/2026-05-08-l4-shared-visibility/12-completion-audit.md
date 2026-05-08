# L4 Shared Visibility — 12 · Completion Audit

## First Action

Invoke the `/bead-completion-audit` skill.

---

## Your Mission

Deep-read the L4 spec, TDD, and the actual codebase after L4 implementation is "complete". Find EVERY gap between what the spec requires and what's actually implemented. Create `audit-fix` beads for gaps. Do NOT execute the fixes — just create and scope them.

This audit is the post-execution gate. The trigger is: `br list --status=open --label layer-4` returns empty (or only contains items already deferred for legitimate reasons recorded in earlier audits).

**CRITICAL: The spec is the source of truth.** The spec was deeply iterated on through 6 prior steps before execution started. EVERY feature in the spec must be implemented in the codebase. If something is missing, it's a gap — not a "nice to have" that was intentionally skipped. Create a fix bead for it.

**Especially watch for:**
- Features that were in the spec but no implementation exists (silent scope drop during execution)
- Features partially implemented (e.g., push protocol exists but writer fencing was skipped)
- Tests in the TDD that don't exist in the codebase (silent test drop)
- ACs that weren't met fully (e.g., AC said "all 4 equal-version cases", code only handles 3)
- Cross-domain notes that weren't acted on (e.g., L2 FreshnessBadge wasn't extended to 3-state)
- Stubs that were left as TODOs instead of minimum implementations (especially the §5.2 epoch-repair reproject — it was explicitly marked "cannot remain a future nice-to-have")

---

## Source Documents

```
docs/strategy/layers/layer-4-shared-visibility.md                              — Spec (SOT #1)        — WALK EVERY L4-FR
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md       — TDD (SOT #2)         — VERIFY EVERY L4-T EXISTS
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md — Bootstrap            — VERIFY ENVIRONMENT MATCHES
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md — Bulk-import doc    — VERIFY EVERY BEAD CLOSED
```

### Codebase (THE IMPLEMENTATION — read deeply)
```
crates/daemon/src/db/sync.rs                                — SHOULD EXIST after L4; if absent, BLOCKING gap
crates/daemon/src/db/sqlite.rs                              — extended for L4? schema_epoch persisted?
crates/daemon/src/validation/policy.rs                      — extended with per-slice readiness?
crates/daemon/src/validation/executor.rs                    — extended with provisional promotion + quarantine?
crates/daemon/src/validation/manifest_writer.rs             — unchanged?
crates/daemon/src/jobs/runner.rs                            — extended with provisional write path?
crates/daemon/src/ipc/handlers/decisions.rs                 — extended with CAS conflict event?
crates/daemon/src/errors/manifest.rs                        — extended with AGENT_SUSPECT_OUTPUT?
crates/daemon/src/main.rs                                   — extended with pull-before-push, lease, schema_epoch wiring?
crates/daemon/Cargo.toml                                    — has reqwest (or chosen alternative)?
crates/daemon/tests/sqlite_schema.rs                        — SHOULD EXIST
crates/daemon/tests/sqlite_supabase_sync.rs                 — SHOULD EXIST
crates/daemon/tests/redaction_boundary.rs                   — SHOULD EXIST
migrations/0012_l4_sync_state.sql (or chosen NNNN)         — SHOULD EXIST; creates local_outbox/sync_batches/pull_cursors
infra/supabase/                                             — SHOULD EXIST: config.toml + 8 migrations
apps/web/                                                   — SHOULD EXIST: 7 source files + tsconfig + playwright + package.json
apps/web/e2e/contracts.spec.ts                              — SHOULD EXIST
package.json (root)                                         — apps/web added to workspaces? bun run scripts extended?
packages/ui/freshness-badge.tsx                             — extended to 3-state (healthy/amber/red)?
skills/validation/procurement-ingestion/                    — SHOULD EXIST (6 files)
skills/validation/supplier-enrichment/                      — SHOULD EXIST (6 files)
skills/validation/document-acquisition/                     — SHOULD EXIST (6 files)
skills/validation/product-extraction/                       — SHOULD EXIST (6 files)
tests/fixtures/calibration/                                 — SHOULD EXIST with 3 corpora seeded
.github/workflows/ (or equivalent)                          — schema hash CI gate added?
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**Do NOT accept "this was intentionally deferred" as an excuse.** If it's in the spec and not in the codebase, it's a gap. Create a fix bead for it. The pipeline started with the cardinal rule "if it's in the research doc, it gets implemented" and that rule applies through this audit.

The only legitimate exceptions are:
- Items explicitly listed in the spec §5.2 Stubs as "Future" (verify the L4 minimum stub IS present, e.g., epoch-repair reproject minimum path)
- Items the spec EXPLICITLY scopes to L5/L6 with rationale (verify the rationale is in the spec, not invented post-hoc)
- Items the audit step 09 explicitly accepted as warnings with documented reasoning

If the gap doesn't fall into one of these three categories, create a fix bead.

---

## Audit Methodology

Walk the spec section by section. For EACH:

### 1. §1 Overview & Sync Engine

For each subsection (§1.3 push, §1.4 pull, §1.5 CAS, §1.6 lease, §1.7 epoch guard, §1.8 degraded mode, §1.9 invariants):
- Find the corresponding implementation in `crates/daemon/src/db/sync.rs` (and supporting files)
- Verify it matches the spec's ACs
- Find the corresponding TDD test(s) in `crates/daemon/tests/sqlite_supabase_sync.rs`
- Run `cargo test --test sqlite_supabase_sync` and verify the test exists and passes
- For each invariant in §1.9 (14 total), verify a dedicated test exists
- For each of the 4 equal-version retry cases in §1.3, verify a separate test exists
- Verify epoch-repair reproject MINIMUM path exists (§5.2 stub — not deferred entirely)

If any subsection has no implementation OR partial implementation OR missing tests → fix bead.

### 2. §2 Supabase Schema, RLS & Remote-Owned Tables

- Verify all 8 migrations exist in `infra/supabase/migrations/`
- Apply them from scratch in a test container; verify schema is as expected
- Verify all 20 push-owned projection tables exist
- Verify all 6 summary tables exist
- Verify all 3 remote-owned tables exist (`shared_company_profile`, `shared_source_configs`, `operator_decision_heads`)
- Verify all 3 junction tables exist
- Verify RLS policies for all 4 roles × 7 categories (28 policy assertions) — `crates/daemon/tests/redaction_boundary.rs` and Playwright test
- Verify daemon-pull tombstone visibility (RLS does NOT filter on `deleted_at IS NULL` for daemon role)
- Verify CAS RPC `update_operator_decision` exists and uses command ledger
- Verify `apply_push_batch` RPC exists with `writer_epoch` validation
- Verify `operator_decision_commands` (command ledger) exists
- Verify `viewer_sync_status` exists with all expected columns
- Verify cross-authority provenance recording (`source_config_revision`, `company_profile_revision`, `operator_decision_revision`) — code path?
- Verify schema hash CI gate exists (apply both migrations from scratch + introspect + hash)

If any of these is missing → fix bead.

### 3. §3 Web Portal & Validation Enforcement

- Verify `apps/web/` directory exists with all 7 source files
- Verify root `package.json` has `apps/web` in workspaces and `bun run {build,test,lint}` includes web
- Verify `apps/web/src/lib/supabase.ts` is the SOLE Supabase boundary
- Verify `packages/ui` lint invariant still holds (no imports of `@tauri-apps/api`/`apps/desktop`/`crates/daemon`/`supabase`/`transport` from `packages/ui`)
- Verify `apps/web/e2e/contracts.spec.ts` exists and tests:
  - Read-only access for viewer role
  - Keyset pagination
  - Sync freshness indicator (3 states)
  - Broadcast invalidation hints
  - RLS boundary (viewer credentials FAIL on remote-owned config tables)
- Verify `packages/ui/freshness-badge.tsx` extended to 3-state (healthy/amber/red) with thresholds 90s, 5min
- Verify three-state degraded-mode UX in desktop too (cross-domain note from §5.5)
- Verify validation enforcement mode active:
  - `validation/policy.rs` extended with per-slice readiness (Wilson-bound, rule-of-three, all 7 prerequisites)
  - `validation/executor.rs` extended with provisional promotion + quarantine path
  - `errors/manifest.rs` extended with AGENT_SUSPECT_OUTPUT type
  - `jobs/runner.rs` extended with provisional write path
  - All 4 demotion triggers exist
- Verify all 4 validator overlays exist:
  - `skills/validation/procurement-ingestion/` (6 files)
  - `skills/validation/supplier-enrichment/` (6 files)
  - `skills/validation/document-acquisition/` (6 files)
  - `skills/validation/product-extraction/` (6 files)
- Verify calibration harness:
  - 3 corpora seeded at `tests/fixtures/calibration/{slice_id}/{clean,weird,mutated}/`
  - All 9 mutation families implemented
  - Pairwise challenge sets implemented
  - Wilson-bound math + rule-of-three math
  - Cross-model audit sampling
  - Override feedback loop (every override → calibration case + incident-cluster + mutation template)

If any of these is missing → fix bead.

### 4. §4 Testing & Acceptance — DoD Walkthrough

Walk every of the 18 DoD criteria in §4.1. For each:
- Find the implementation
- Find the test that verifies it
- Run the test; verify it passes
- If implementation is partial OR test is missing OR test fails → fix bead

For the two-user verification (DoD #12), this is a manual verification:
- Run desktop on machine A with operator credentials
- Run web on machine B with viewer credentials
- Verify the same data appears
- Verify viewer cannot mutate
- Verify sync state badges match

If two-user verification was not performed → fix bead requiring it.

### 5. §5 Open Questions, Stubs, Forward-Compat, Dependencies

Verify the §5.2 stubs each have minimum implementation (NOT deferred entirely):
- Schema epoch transitions: initial epoch only (acceptable; no transitions needed yet)
- Multi-org: single org hardcoded (acceptable; future scoping)
- **Epoch-repair reproject:** MINIMUM path — mark `projection_generation`, republish current row images, discard stale-epoch outbox after reseed. Spec EXPLICITLY says "cannot remain a future nice-to-have". If this minimum path is missing → BLOCKING gap → fix bead.
- Logical consistency auditor: minimum stub — domain invariant checking, no auto-repair. If absent → fix bead.

Verify the §5.4 dependencies are all met:
- Supabase project provisioned
- 8 Supabase migrations applied
- `crates/daemon/src/db/sync.rs` exists
- 4 validator overlays exist
- `apps/web/` exists with 7 files

If any dependency unmet → fix bead.

### 6. Cross-Domain Notes (§5.5)

- L0 cross-domain note: `commit_group_id` (text, nullable) and `writer_epoch` (integer, nullable) on `local_outbox`. Verify the columns exist in `migrations/0012_l4_sync_state.sql`.
- L2 cross-domain note: 3-state FreshnessBadge upgrade. Verify `packages/ui/freshness-badge.tsx` accepts the 3-state prop.
- L2 cross-domain note: desktop decision capture transition. Verify desktop's decision capture flow still writes to `pending_operator_intents` AND L4 added daemon-side CAS flush + conflict resolution UI.
- L6 cross-domain note: remote config staleness classes safety-critical TTL. Verify §1.8's 4-row table is implemented (config classes get different stale behaviors).

If any cross-domain note unmet → fix bead.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)

1. Spec, TDD, Bootstrap, Bulk-import — all 4 — **READ FULLY**
2. `br list --label layer-4 --status=open --json` — verify it's empty (or only has legitimate-deferral items)
3. `br list --label layer-4 --status=closed --json` — confirm bead closure pattern matches the planned set

### Step 1: Invoke `/bead-completion-audit`

**Inputs:**
- Spec: `docs/strategy/layers/layer-4-shared-visibility.md`
- TDD: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md`
- Bootstrap: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md`
- Bulk-import: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md`
- Codebase: full repo state

**Key arguments/context:**
- Bead prefix: `l4`
- Fix bead label: `audit-fix` (combined with `layer-4`)
- Audit focus: walk every L4-FR-X.Y.Z and L4-T-* exhaustively; do NOT sample
- Cardinal rule: spec is source of truth; gaps = fix beads, never "deferred"
- Quality bar: L3 completion audit (verify L3 audit doc structure if exists)

### Step 2: Apply Audit

The skill walks every spec section + codebase. For each gap, it should:
1. Identify the spec L4-FR or L4-T that's not satisfied
2. Identify what's missing in the codebase
3. Create an `audit-fix` bead with:
   - Title: `l4-fix-{slug}` (e.g., `l4-fix-equal-version-retry-case-3`)
   - Labels: `layer-4`, `audit-fix`, `l4-phase-{N}` (the phase the gap belongs to)
   - Priority: P0 if blocking DoD; P1 if completing partial work; P2 if minor
   - Description: spec L4-FR text + codebase gap description + ACs
   - Dependencies: link to original-implementation bead if relevant

After all fix beads created:
```bash
bin/br-sync
git add .beads/
git commit -m "beads: l4 completion audit — N fix beads created"
git pull --rebase
git push
```

### Step 3: Verify Audit Output

After the skill completes, verify:

- [ ] Audit report exists (file path or in-session output)
- [ ] Every spec L4-FR was checked
- [ ] Every TDD L4-T was checked (does the test exist? does it pass?)
- [ ] Every of the 18 DoD criteria was walked
- [ ] Every of the 14 invariants was verified
- [ ] Every of the 4 validator overlays was checked (6 files each)
- [ ] All 8 Supabase migrations applied cleanly in test container
- [ ] All 7 web files exist with expected content
- [ ] All 3 cross-domain notes were verified
- [ ] §5.2 stubs (especially epoch-repair reproject minimum) are present, not deferred
- [ ] Fix beads created for every gap with full ACs
- [ ] No "post-MVP" / "Phase 2+" / "stretch goal" / "deferred" justifications accepted (all gaps become fix beads)

### Step 4: Determine Verdict

The audit returns one of:

- **COMPLETE** — Zero gaps. All spec L4-FR satisfied. All tests exist and pass. Two-user verification passed. Proceed to step 13 only if there are zero fix beads. Otherwise proceed to fix-bead execution.

- **GAPS DETECTED** — At least one spec L4-FR is unimplemented or partially implemented. Fix beads created. Proceed to step 13 (fix-bead execution kickoff).

- **BLOCKING GAPS** — Critical gaps that prevent the layer from being usable (e.g., sync engine doesn't push at all, web portal doesn't render). Fix beads at P0; these MUST be executed before archive.

- **SPEC ERROR DETECTED** (rare escape hatch) — During execution, an agent discovered the spec itself has a contradiction, impossibility, or factual error that fix beads cannot resolve (e.g., a §1.X invariant cannot coexist with a §3.X requirement under any implementation; a referenced Postgres feature doesn't exist in the version pinned by `infra/supabase/config.toml`; a cross-domain note on L0 is structurally impossible). Fix beads cannot fix the spec.
  **Path:** stop the audit. Do NOT proceed to fix beads. Return to step 01 (validate-spec) with a documented "spec-error" exception list (file: `docs/pipeline/l4-shared-visibility/spec-errors-{date}.md`). Apply spec edits with full re-validation. Then regenerate downstream artifacts in order (re-run 02 → 04 → 06 → 08 if affected). Then return to this step.
  This is rare. Per AGENTS.md Rule 0, if a human-overridable judgment is needed, escalate via AgentMail with the proposed spec edit before unilateral spec changes.

If GAPS DETECTED or BLOCKING, the pipeline does NOT proceed to archive (step 15) until all fix beads are closed and a re-audit returns COMPLETE.
If SPEC ERROR DETECTED, the pipeline returns to step 01.

---

## Follow-Up Pass (Sub-Optimization)

> After creating all fix beads:

ULTRATHINK.

Re-read the fix beads you created. Did fixing them surface new issues?

1. **Are they specific enough?** Each fix bead body should reference:
   - The exact spec L4-FR-X.Y.Z that's not satisfied
   - The exact file/line where implementation is missing or incomplete
   - The exact L4-T test ID that fails or doesn't exist
   - The exact AC the bead must meet (concrete, testable)

   If a fix bead says "fix the validator", that's too vague. Re-write it to "Add `validator_overlay_compatible_versions` field to `skills/validation/procurement-ingestion/SKILL.md` frontmatter to satisfy spec §3.5 / L4-FR-3.5.1; verified by `L4-T-canary-procurement-001` which currently fails because the field is absent."

2. **Did you cover EVERY spec requirement, or did you stop early?** Walk every L4-FR-X.Y.Z one final time. For each, confirm: (a) it's implemented in the codebase, (b) tests exist and pass, OR (c) a fix bead exists addressing it. If any L4-FR has none of these three, you missed a gap.

3. **Final count check:** spec L4-FR count = beads delivering them + fix beads = 100%? Run:
   ```bash
   grep -oE 'L4-FR-[0-9]+\.[0-9]+(\.[0-9]+)?' docs/strategy/layers/layer-4-shared-visibility.md | sort -u | wc -l
   br list --label layer-4 --json | jq '[.issues[] | .description] | join(" ")' | grep -oE 'L4-FR-[0-9]+\.[0-9]+(\.[0-9]+)?' | sort -u | wc -l
   ```
   The two numbers should match. If beads have fewer L4-FR coverage, fix beads are needed.

4. **Did your fix beads have correct phase labels?** A fix bead for missing `apps/web/src/lib/broadcast.ts` Broadcast topic logic should be `l4-phase-5` (web phase). Don't lump all fix beads into one phase.

5. **Re-scan fix beads for scope-drift language.** They MUST NOT contain `post-MVP`, `Phase 2+`, `nice to have`, `deferred`, etc. — fix beads are by definition NOT deferred (they're fixes, not new features).

6. **Could an agent implement each fix from the fix bead description alone?** The acceptance criteria should be concrete — not "fix this" but "add field X of type Y to struct Z; pass test L4-T-..."

7. **Verify the fix beads cover BLOCKING items from §5.2 stubs.** Especially the epoch-repair reproject minimum path. If §5.2 explicitly said "cannot remain a future nice-to-have" and the implementation skipped it, a P0 fix bead is mandatory.

8. **Re-verify cross-domain notes.** L0 (`commit_group_id`/`writer_epoch` on `local_outbox`), L2 (3-state FreshnessBadge), L2 (decision capture + CAS flush), L6 (remote config staleness classes) — each must have either implementation or a fix bead.

---

## Audit Output

Save the audit report to `docs/pipeline/l4-shared-visibility/completion-audit-report-{date}.md` with:
- Overall verdict (COMPLETE / GAPS DETECTED / BLOCKING GAPS)
- Summary of gaps found by section
- Total fix beads created and their IDs
- Walkthrough of every of the 18 DoD criteria with status (✓ pass, ⚠ partial, ✗ fail)
- Test execution results (which tests exist, which pass, which fail)
- Cross-domain note verification status

This report becomes part of the archived pipeline in step 15.

---

## Key Reminders

- **Spec is source of truth.** Gaps = fix beads, never "deferred". Cardinal rule from research-doc through audit.
- **Every L4-FR-X.Y.Z** must be verified against codebase + tests
- **Every L4-T-** must exist and pass
- **§5.2 stubs** must have minimum implementations (especially epoch-repair reproject)
- **Cross-domain notes** must be verified (L0, L2 × 2, L6)
- **Two-user verification** is a manual gate (DoD #12) — must be performed
- **Fix beads** are P0/P1 by default — they're NOT deferred work, they're missing implementation
- **AgentMail project key:** `/Users/alinaaitova/Desktop/Testproject1`
- **Bead CLI:** `br`. **Sync:** `bin/br-sync`.
- **Don't skip fix beads even for "minor" gaps.** A minor gap not in the spec is OK; a minor gap that IS in the spec is a fix bead.
