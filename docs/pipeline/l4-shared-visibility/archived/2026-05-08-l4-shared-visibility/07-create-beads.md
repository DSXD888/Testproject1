# L4 Shared Visibility — 07 · Create Beads

## First Action

Invoke the `/beads-from-sources` skill.

---

## Your Mission

Materialize beads for L4 Shared Visibility from the 3 SOTs (spec, TDD, strategy). Beads are **self-contained execution units** — each one carries enough context (file paths, type names, test IDs, ACs) that an agent can implement it from the bead alone. Spec and TDD content is **embedded verbatim** into beads — beads do not paraphrase.

The strategy provides phase ordering only. Beads inherit phase membership and dependency relationships, but bead bodies are sourced from spec L4-FR and TDD L4-T content directly.

---

## Source Documents (Read In This Order)

### Primary Inputs (CREATE FROM THESE)
```
docs/strategy/layers/layer-4-shared-visibility.md                              — Spec (SOT #1)        — READ FULLY
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md       — TDD (SOT #2)         — READ FULLY
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md  — Strategy (SOT #3)    — READ FULLY (for phase ordering only — bead bodies do NOT come from here)
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md — Bootstrap            — READ FULLY (every phase-1 bead must reflect bootstrap prerequisites)
```

### Codebase Context (BEAD BODY GROUNDING — for file paths, type names)
```
crates/daemon/src/db/sqlite.rs                              — 600 lines, where db modules live
crates/daemon/src/db/checkpoints.rs                         — 231 lines, parallel module to where sync.rs will live
crates/daemon/src/validation/policy.rs                      — 959 lines, L3 policy engine (extends in Phase 4)
crates/daemon/src/validation/executor.rs                    — 1217 lines, validator skill spawner (extends in Phase 4)
crates/daemon/src/validation/manifest_writer.rs             — 54 lines
crates/daemon/src/jobs/runner.rs                            — 1027 lines, runner extends in Phase 4
crates/daemon/src/ipc/handlers/decisions.rs                 — pending_operator_intents writer (extends in Phase 3)
crates/daemon/src/errors/manifest.rs                        — 386 lines, AGENT_SUSPECT_OUTPUT added in Phase 4
crates/daemon/src/main.rs                                   — 1.08 MB monolithic init (Phase 6 wiring bead only)
crates/daemon/Cargo.toml                                    — net-new HTTP client (Phase 1 dep-add bead)
package.json (root)                                         — net-new apps/web workspace (Phase 1 wiring bead)
migrations/                                                 — flat sequence; next NNNN_ for L4
infra/supabase/                                             — net-new directory (Phase 1)
apps/web/                                                   — net-new directory (Phase 5)
apps/desktop/src/pages/                                     — templates for apps/web/src/app/* pages
packages/ui/                                                — components apps/web reuses unchanged
skills/validation/base/                                     — template for the 4 overlays (Phase 4)
```

### Quality Bar (MATCH OR EXCEED)
```
.beads/beads.db                                                                         — full bead corpus (skim a few existing l3-* beads to understand the shape)
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bulk-import.md  — L3's bulk-import doc (target)
docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/07-create-beads.md  — L3's analogous handoff
```

### Existing L4 Epic (RE-USE, DO NOT DUPLICATE)
```
br show testproject1-4zq    — Epic: Layer 4 — Shared Visibility (open, no children)
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**100% spec coverage is MANDATORY.** If any spec L4-FR has zero corresponding beads, this is a BLOCKING failure. If any TDD L4-T test ID is unowned (no bead claims it), that's a BLOCKING failure.

**Beads embed spec + TDD content verbatim.** Do not paraphrase; do not summarize. The bead body should contain the actual L4-FR text + the actual L4-T test names + the actual file paths and type names from the codebase.

**Phases are execution ORDER**, not importance tiers. Late-phase beads get the SAME depth as early-phase beads. Every phase ships.

---

## Key Context From Codebase Research

1. **Bead CLI is `br`** (NOT `bd`). The AGENTS.md duplicated `bd` block at ~L651-708 is OUTDATED. Use `br` exclusively. Use `bin/br-sync` (or `br sync --flush-only` followed by manual `git add .beads/`) — never raw `br sync`.

2. **Bead prefix:** `l4`. Existing epic `testproject1-4zq` describes children `.1` through `.4` plus a review gate `.5` — but the spec has 4 main sections, the strategy may have 6 phases. Decide whether to:
   - Keep epic `testproject1-4zq` and create children directly under it for each phase OR each L4 sub-feature
   - Create 4-6 sub-epics (`l4-schema-foundation`, `l4-push-engine`, `l4-pull-cas`, `l4-validation-enforcement`, `l4-web-portal`, `l4-integration`) parented to `testproject1-4zq`, with task beads under each sub-epic
   The strategy doc made this call. Honor it.

3. **Bead DB location:** `.beads/beads.db`. Bulk-import docs are conventionally at `{handoff_dir}/{feature}-bulk-import.md` mirroring L3's pattern. After `br create` runs, run `bin/br-sync && git add .beads/ && git commit -m "beads: l4 import" && git push`.

4. **Bead sync semantics:** `br sync --flush-only` exports the SQLite DB to JSONL files in `.beads/` without running git. THEN you manually `git add .beads/ && git commit && git push`. Never run `br sync` (without flag) — older versions ran git automatically and the AGENTS.md duplicate block reflects the legacy behavior. Always use `bin/br-sync` (the project wrapper) for safety.

5. **Bead self-containment requirement:** every bead must have:
   - **Title:** `l4-p{N}-{slug}` format (matching L3 convention; e.g., `l4-p1-sqlite-sync-migration`, `l4-p4-validation-executor`)
   - **Description:** embeds spec L4-FR text + TDD L4-T test IDs + file paths + type names + ACs verbatim
   - **Acceptance Criteria:** bullet list, each AC bridges spec → testable assertion → file location
   - **Dependencies:** explicit `br dep add` calls to upstream beads
   - **Labels:** `layer-4`, plus phase label (`l4-phase-1`, ..., `l4-phase-6`)
   - **Priority:** numeric (P0=critical, P1=high, P2=medium, P3=low) — schema bead is P0; main.rs wiring is P0; web tests are P1
   - **Issue type:** `task` (default) or `epic` (sub-epics)

6. **Spec features and their bead landing zones (heuristic — strategy is authoritative):**

   - **Phase 1 — Schema Foundation** (~5-8 beads, P0)
     - `l4-p1-sqlite-sync-migration` — `migrations/0012_l4_sync_state.sql` (creates `local_outbox` with `commit_group_id`/`writer_epoch` columns, `sync_batches`, `pull_cursors`)
     - `l4-p1-supabase-migrations-projection` — `infra/supabase/migrations/0001_projection_tables.sql` (20 push-owned mirrors + 3 junction tables)
     - `l4-p1-supabase-migrations-remote-owned` — `infra/supabase/migrations/0002_remote_owned_tables.sql`
     - `l4-p1-supabase-migrations-summaries` — `infra/supabase/migrations/0003_summary_tables.sql`
     - `l4-p1-supabase-migrations-rls` — `infra/supabase/migrations/0004_rls_policies.sql` (depends on 0001-0003)
     - `l4-p1-supabase-migrations-rpcs` — `infra/supabase/migrations/0005_sync_rpcs.sql`
     - `l4-p1-supabase-migrations-metadata-lease` — `infra/supabase/migrations/0006_schema_metadata.sql`
     - `l4-p1-supabase-migrations-command-ledger` — `infra/supabase/migrations/0007_command_ledger.sql`
     - `l4-p1-supabase-migrations-decision-summaries` — `infra/supabase/migrations/0008_decision_summaries.sql`
     - `l4-p1-daemon-http-client-dep` — add `reqwest` to `crates/daemon/Cargo.toml`
     - `l4-p1-apps-web-workspace-wiring` — add `apps/web` to root `package.json` workspaces + scripts; preserve `packages/ui` lint invariant
     - `l4-p1-schema-hash-ci-gate` — CI rule that applies both migrations from scratch + introspects + hashes

     (Some of these can be grouped into fewer beads — e.g., one "Supabase migrations bundle" bead for 0001-0008 if the work is closely coupled.)

   - **Phase 2 — Push Engine** (~8-12 beads, mostly P0/P1)
     - `l4-p2-sync-rs-skeleton` — create `crates/daemon/src/db/sync.rs` with module structure
     - `l4-p2-outbox-writer` — transactional outbox writer (commit_group_id grouping)
     - `l4-p2-push-dispatcher` — single-in-flight, identical-envelope retries, equal-version retry semantics (4 cases), batch boundary checks
     - `l4-p2-apply-push-batch-rpc` — Postgres function `apply_push_batch(batch_json, writer_epoch)`
     - `l4-p2-writer-fencing` — `writer_epoch` increment on lease acquisition, included in every batch
     - `l4-p2-active-writer-lease` — lease row + acquisition + heartbeat
     - `l4-p2-viewer-sync-status-update` — update on success
     - `l4-p2-schema-epoch-guard` — pause sync on mismatch
     - `l4-p2-epoch-repair-reproject-min-path` — minimum stub: mark `projection_generation`, republish, discard stale-epoch outbox
     - `l4-p2-push-engine-integration-tests` — `crates/daemon/tests/sqlite_supabase_sync.rs` push subset

   - **Phase 3 — Pull Engine + CAS** (~6-8 beads)
     - `l4-p3-pull-cursor-shared-source-configs`
     - `l4-p3-pull-cursor-shared-company-profile`
     - `l4-p3-pull-cursor-operator-decision-heads`
     - `l4-p3-hi-water-fence-algorithm` — implements per-domain fenced pagination
     - `l4-p3-pull-before-push-startup-ordering`
     - `l4-p3-tombstone-aware-daemon-pull` — RLS allows daemon to see tombstones
     - `l4-p3-cas-rpc-update-operator-decision` — Postgres function with command ledger integration
     - `l4-p3-command-ledger-idempotency-replay` — replay returns stored result, not re-evaluation
     - `l4-p3-command-ledger-gc-30-day` — retention + GC
     - `l4-p3-pending-intent-reconciliation` — latest-per-process coalescing in daemon
     - `l4-p3-decision-conflict-ipc-event` — `crates/daemon/src/ipc/handlers/decisions.rs` extension

   - **Phase 4 — Validation Enforcement** (~10-15 beads)
     - `l4-p4-provisional-staging-table` — SQLite staging mechanism (table or directory chosen in spec)
     - `l4-p4-policy-engine-per-slice-readiness` — extend `validation/policy.rs` (chokepoint — single bead)
     - `l4-p4-policy-engine-demotion-triggers` — 4 triggers
     - `l4-p4-executor-quarantine-path` — extend `validation/executor.rs` (chokepoint — single bead)
     - `l4-p4-runner-provisional-write-path` — extend `jobs/runner.rs` (chokepoint — single bead)
     - `l4-p4-error-type-agent-suspect-output` — extend `errors/manifest.rs`
     - `l4-p4-overlay-procurement-ingestion` — `skills/validation/procurement-ingestion/` (6-file template)
     - `l4-p4-overlay-supplier-enrichment` — `skills/validation/supplier-enrichment/`
     - `l4-p4-overlay-document-acquisition` — `skills/validation/document-acquisition/`
     - `l4-p4-overlay-product-extraction` — `skills/validation/product-extraction/`
     - `l4-p4-calibration-corpora-bootstrap` — seed `tests/fixtures/calibration/{slice_id}/{clean,weird,mutated}/`
     - `l4-p4-calibration-mutation-library` — implement 9 mutation families
     - `l4-p4-calibration-pairwise-challenge-sets`
     - `l4-p4-calibration-wilson-bound-math`
     - `l4-p4-calibration-rule-of-three-math`
     - `l4-p4-calibration-cross-model-audit-sampling`
     - `l4-p4-override-feedback-loop` — every override → calibration case + incident-cluster + mutation template

   - **Phase 5 — Web Portal** (~7-10 beads)
     - `l4-p5-apps-web-skeleton` — Next.js scaffold + tsconfig + playwright config + package.json
     - `l4-p5-supabase-client-init` — `apps/web/src/lib/supabase.ts`
     - `l4-p5-broadcast-subscription-manager` — `apps/web/src/lib/broadcast.ts`
     - `l4-p5-layout-and-redirect` — `apps/web/src/app/{layout,page}.tsx`
     - `l4-p5-contracts-list-page` — `apps/web/src/app/contracts/page.tsx`
     - `l4-p5-contracts-detail-page` — `apps/web/src/app/contracts/[id]/page.tsx`
     - `l4-p5-status-page` — `apps/web/src/app/status/page.tsx`
     - `l4-p5-three-state-degraded-mode-ux` — green/amber/red + thresholds
     - `l4-p5-viewer-credentials-rls-boundary-test` — `apps/web/e2e/contracts.spec.ts` RLS boundary verification
     - `l4-p5-broadcast-invalidation-e2e-test` — Playwright test for live updates
     - `l4-p5-freshness-badge-three-state-extension` — extend `packages/ui/freshness-badge.tsx`

   - **Phase 6 — Integration & Hardening** (~6-10 beads)
     - `l4-p6-main-rs-startup-wiring` — pull-before-push, lease acquisition, schema_epoch check
     - `l4-p6-redaction-tests-all-summaries` — `crates/daemon/tests/redaction_boundary.rs` for all 6 projections
     - `l4-p6-tombstone-retention-14-days` — operational constraint + CI test
     - `l4-p6-logical-consistency-auditor-stub` — minimum stub per §5.2
     - `l4-p6-cross-authority-provenance` — record `source_config_revision`, `company_profile_revision`, `operator_decision_revision` per pipeline run
     - `l4-p6-two-user-verification-protocol` — manual verification doc + checklist
     - `l4-p6-full-e2e-push-pull-round-trip` — comprehensive integration test
     - `l4-p6-dod-walkthrough` — verify all 18 DoD criteria
     - `l4-p6-stage-validation-test-gate-update` — extend the AGENTS.md test gate to include web e2e

7. **{ENRICHMENT: spec metadata}** — total L4-FR count + ID range from step 02. Use this to verify coverage matrix.

8. **{ENRICHMENT: tdd metadata}** — total L4-T count + per-file breakdown from step 04. Use this to verify every test ID is owned by a bead.

9. **{ENRICHMENT: strategy metadata}** — phase names + suggested bead counts + gate criteria from step 06. Use this to validate the bead structure aligns with strategy ordering.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY — before anything)

1. Spec, TDD, Strategy, Bootstrap — all 4 — **READ FULLY**
2. `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bulk-import.md` — **STUDY** the bulk-import doc structure (this is the largest L3 bulk import; ~69KB)
3. `br show testproject1-4zq` — **CONFIRM** the existing L4 epic
4. `br list --label layer-4 --json` — **CONFIRM** there are no other L4 beads yet (clean slate)
5. Run `br --version` — **CONFIRM** the version of br matches the AGENTS.md guidance
6. Walk a few existing L3 task beads via `br show l3-p1-deps-migration` (or any l3 bead from `br list --label layer-3 --json`) — **STUDY** the bead body shape (description format, ACs, deps)

### Step 1: Invoke `/beads-from-sources`

**Inputs:**
- Spec: `docs/strategy/layers/layer-4-shared-visibility.md`
- TDD: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md`
- Strategy: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md`
- Bulk import doc output: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md`
- Bead corpus: `.beads/beads.db`

**Key arguments/context to provide the skill:**
- Bead prefix: `l4`
- Bead CLI: `br` (NOT `bd`)
- Sync command: `bin/br-sync` (or `br sync --flush-only` + manual git add)
- Existing epic: `testproject1-4zq` (Layer 4 — Shared Visibility)
- Project key (AgentMail): `/Users/alinaaitova/Desktop/Testproject1`
- Phase labels: `l4-phase-1` through `l4-phase-6`
- Spec ID convention: `L4-FR-X.Y.Z` (per step 01)
- TDD ID convention: `L4-T-{file_stem}-{NNN}` (per step 03-04)
- Quality bar: L3 bulk-import doc + existing L3 task bead bodies

### Step 2: Feature-Specific Guidance

While the skill runs, pay attention to:

- **Bead self-containment is non-negotiable.** Every bead body must include: file paths (e.g., `crates/daemon/src/db/sync.rs`), type/function names (e.g., `OutboxEntry`, `apply_push_batch`), spec L4-FR IDs being implemented, TDD L4-T IDs being satisfied, ACs (bullet list with observable outcomes).
- **Embed spec text verbatim.** If a bead implements L4-FR-1.3.5 "Equal-version retry semantics", the bead body should quote the spec's 4-case table verbatim. Do not paraphrase.
- **Embed TDD test bodies verbatim.** If a bead is satisfied by `L4-T-sqlite_supabase_sync-{N}` "writer fencing reject test", the bead body should quote the test's expected behavior verbatim.
- **Single-edit chokepoint discipline:** at most ONE bead per chokepoint file. Verify:
  - Only one bead modifies `crates/daemon/src/main.rs` (Phase 6 wiring bead)
  - Only one bead modifies `crates/daemon/src/jobs/runner.rs` (Phase 4 runner extension)
  - Only one bead modifies `crates/daemon/src/validation/policy.rs` (Phase 4 policy extension)
  - Only one bead modifies `crates/daemon/src/validation/executor.rs` (Phase 4 executor extension)
  - Only one bead modifies `crates/daemon/Cargo.toml` (Phase 1 dep-add)
  - Only one bead modifies root `package.json` workspaces + scripts (Phase 1 wiring)
- **Dependency edges are explicit.** After `br create`, run `br dep add <bead-id> <upstream-bead-id>` for every dependency. The push-dispatcher bead depends on the outbox-writer bead. The CAS RPC bead depends on the schema migration. Etc.
- **Phase labels are applied.** Every task bead gets `--label layer-4` AND `--label l4-phase-{N}`. (br accepts multiple `--label` flags.)
- **Sub-epic strategy decision honored.** If the strategy doc said "create 4-6 sub-epics", the agent must `br create --type=epic` for each sub-epic and parent task beads via dependency or epic-membership semantics.
- **Bead priority is informed by phase + chokepoint:** Phase 1 schema migration = P0 (everything blocks on it); Phase 6 main.rs wiring = P0 (final gate); validator overlays = P1 (parallelizable); Playwright e2e = P1; web pages = P1; logical consistency stub = P2.
- **Calibration corpus seeding is a small bead** but non-trivial because it requires real run manifests from L3 dry-run.
- **`apps/web` skeleton bead is heavy** — Next.js + tsconfig + Playwright + package.json + Supabase client deps + integration into root workspaces. Be explicit; do not let this bead be empty.
- **Schema hash CI gate** must be a real bead with a CI workflow file (e.g., `.github/workflows/schema-hash.yml` or extend an existing CI file). Cannot be left vague.

### Step 3: Verify Output

After the skill completes, verify:

- [ ] `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md` exists
- [ ] `br list --label layer-4` returns ≥30 beads (typical 30-50)
- [ ] `br ready --label layer-4` returns the Phase 1 root beads (no upstream blockers)
- [ ] Every spec L4-FR-X.Y.Z appears in at least one bead description (grep through `br list --json`)
- [ ] Every TDD L4-T-* test ID is owned by exactly one bead
- [ ] Bead prefix is `l4` consistently
- [ ] Phase labels `l4-phase-1` through `l4-phase-6` applied
- [ ] Single-edit chokepoint discipline preserved (at most one bead per chokepoint file across phases)
- [ ] Dependency edges added (`br dep list <id>` shows expected upstream)
- [ ] No bead body has "post-MVP" / "Phase 2+" / "stretch goal" / "deferred" / "future iteration" / "nice to have" / "simplified for now" language
- [ ] No bead is empty (every bead has full description, ACs, file paths)
- [ ] Bulk-import doc has the same density as L3's
- [ ] After `bin/br-sync && git add .beads/`, `git status` shows .beads/ changes ready to commit
- [ ] `br lint` passes (no broken dependency cycles, no orphaned beads)

### Step 3.5: Concrete `br create` Pattern (Reference Example)

For each bead the skill creates, the underlying invocation pattern looks like this. Use this as a mental model when verifying the skill's output, and as a manual fallback if any bead needs to be re-created by hand:

```bash
# 1. Create the bead — capture stdout for the new ID
NEW_ID=$(br create \
  --title="l4-p1-sqlite-sync-migration" \
  --type=task \
  --priority=0 \
  --description="$(cat <<'EOF'
# l4-p1-sqlite-sync-migration

## Spec L4-FR References
- L4-FR-1.3.X (outbox column shape)
- L4-FR-1.10.X (Components Touched: Database Create)

## Files Created
- migrations/{NEXT_NNNN}_l4_sync_state.sql
  Creates: local_outbox (with commit_group_id text nullable, writer_epoch integer nullable),
           sync_batches, pull_cursors

## TDD Test IDs Satisfied
- L4-T-sqlite_schema-001 (table exists)
- L4-T-sqlite_schema-002 (commit_group_id column shape)
- L4-T-sqlite_schema-003 (writer_epoch column shape)
- L4-T-sqlite_supabase_sync-001 (outbox row written transactionally with data row)

## Acceptance Criteria
- [ ] migrations/{NEXT_NNNN}_l4_sync_state.sql exists with the 3 tables
- [ ] commit_group_id is `text NULL` per L0 cross-domain note
- [ ] writer_epoch is `integer NULL` per L0 cross-domain note
- [ ] crates/daemon/src/db/sqlite.rs:open_state_db applies the migration cleanly on a fresh DB
- [ ] L4-T-sqlite_schema-001..003 pass under `cargo test --test sqlite_schema`

## Cross-Layer Notes
- L0: this bead implements the constitutional `commit_group_id`/`writer_epoch` column shape reserved as a forward-compat note in L0 §5.5.
EOF
)" --json | jq -r '.id')

# 2. Apply labels
br label add "$NEW_ID" layer-4
br label add "$NEW_ID" l4-phase-1

# 3. Add dependency to existing L4 epic
br dep add "$NEW_ID" testproject1-4zq

# 4. (Repeat for next bead with appropriate upstream deps)
# Example: l4-p2-outbox-writer depends on l4-p1-sqlite-sync-migration
# br dep add <l4-p2-outbox-writer-id> $NEW_ID
```

Notes on this pattern:
- `--priority=0` for P0 (root of DAG); `--priority=1` for P1 (most beads); `--priority=2` for P2 (docs / minor cleanup)
- `--type=task` (default); `--type=epic` only for the optional sub-epics
- The HEREDOC description must include: spec L4-FR references, files created/modified, TDD test IDs satisfied, ACs as a checkbox list, and any cross-layer notes
- After creating ALL beads + adding ALL deps + ALL labels:
  ```bash
  bin/br-sync
  git add .beads/
  git commit -m "beads: l4 shared visibility — initial import (NN beads)"
  git pull --rebase && git push
  ```

### Step 4: Sync Beads to Repo

```bash
bin/br-sync                      # exports to JSONL files in .beads/ (NO git ops)
git add .beads/
git commit -m "beads: l4 shared visibility — initial import (NN beads)"
git pull --rebase
git push
git status                       # MUST show "up to date with origin"
```

### Step 5: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete.

ULTRATHINK.

Re-read the beads with fresh eyes and answer:

1. **Open `crates/daemon/src/main.rs`** (skim only — 1.08 MB monolith). Walk every bead's "files this bead modifies" annotation. Confirm: only ONE bead lists `main.rs` (the Phase 6 startup-wiring bead). If two or more beads list `main.rs`, this is a parallel-execution merge-conflict trap. Consolidate.

2. **Open `crates/daemon/src/jobs/runner.rs` (1027 lines).** Confirm only one bead extends it (Phase 4 runner-provisional-write-path). If split, consolidate.

3. **Open `crates/daemon/src/validation/policy.rs` (959 lines)** and `executor.rs` (1217 lines). Confirm one bead per file. Confirm executor depends on policy (`br dep add`).

4. **Open `migrations/`** and confirm next-NNNN sequence. Walk every Phase 1 migration bead — verify each names a distinct file with the correct NNNN. If any two beads target the same migration filename, that's a collision.

5. **Walk every of the 4 validator overlays.** For each, verify the bead body lists the 6 standard files (`SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`). The bead body should also reference `skills/validation/base/SKILL.md` as the template.

6. **Walk every of the 7 web files.** For each, verify a bead exists OR is grouped into a multi-file bead (`l4-p5-contracts-pages` for both list + detail). The bead body should reference the corresponding `apps/desktop/src/pages/` source as the structural template.

7. **Walk every of the 8 Supabase migrations.** For each, verify a bead exists with the exact filename and target tables. Verify dep edges: 0004 (RLS) depends on 0001-0003 (tables); 0005 (RPCs) depends on 0001-0004; etc.

8. **Walk the §1.9 invariants table (14 invariants).** For each, identify which bead(s) establish the invariant. If any invariant has zero beads, that's a coverage gap.

9. **Walk the §4.1 DoD table (18 criteria).** For each, identify which bead(s) deliver it. If any DoD has zero beads, that's a coverage gap.

10. **Walk the calibration harness §3.6.** Verify beads exist for: 3 corpora seeding, 9 mutation families, pairwise challenge sets, Wilson-bound math, rule-of-three math, cross-model audit sampling, override feedback loop. That's ~7 beads. If split differently or consolidated, verify the bead bodies cover all 7 elements.

11. **Walk every L4-T test ID.** For each, identify which bead claims it. If any L4-T is unclaimed, this is a BLOCKING coverage gap.

12. **Re-scan beads for scope-drift language.** `br list --json` and search every bead description for: `post-MVP`, `Phase 2+`, `stretch goal`, `nice to have`, `deferred`, `future iteration`, `simplified for now`, `can be added later`, `eventually`, `for now`, `optional`, `if time permits`, `we may`. Hits in load-bearing positions = BLOCKING failure. Re-write those bead bodies to remove the language.

13. **Run `br lint`**. Address any warnings (orphaned beads, dependency cycles, missing labels).

14. **Verify the existing epic `testproject1-4zq`** is properly reflected. If you sub-divided into 4-6 sub-epics, the sub-epics should have parent dependency on `testproject1-4zq`. If you kept the single epic, the task beads should depend on it.

### Step 6: Scope-Preservation Check (MANDATORY — BLOCKING)

After the sub-optimization, perform this check:

1. **Enumerate** every L4-FR from the spec. For each, verify ≥1 bead exists with that ID in its body.
2. **Enumerate** every L4-T from the TDD. For each, verify exactly one bead claims it.
3. **Enumerate** every of the 18 DoD criteria. For each, verify ≥1 bead delivers it.
4. **Enumerate** every of the 14 invariants. For each, verify ≥1 bead establishes it.
5. **Scan** all bead bodies for red-flag language. Any hit = BLOCKING.
6. **If any spec feature has zero beads** — BLOCKING. Add beads before proceeding.
7. **If any test ID is unclaimed** — BLOCKING. Reassign or create new bead.
8. **If any red-flag language is found** — BLOCKING. Edit the bead body and re-sync.

---

## Key Reminders

- **Bead CLI is `br`.** Never `bd`. The duplicated AGENTS.md `bd` block at ~L651-708 is OUTDATED.
- **Sync command is `bin/br-sync`** (or `br sync --flush-only` + manual `git add .beads/`). Never raw `br sync`.
- **Beads are SELF-CONTAINED.** Each bead embeds spec L4-FR text + TDD L4-T test bodies + file paths + ACs. Implementation agents should never need to reopen the spec.
- **Bead title format:** `l4-p{N}-{slug}` (e.g., `l4-p1-sqlite-sync-migration`, `l4-p4-overlay-procurement-ingestion`).
- **Phase labels:** `layer-4` AND `l4-phase-{N}` on every task bead.
- **Dependency edges are explicit.** Use `br dep add <bead> <upstream>`.
- **Single-edit chokepoint discipline:** at most ONE bead per chokepoint file.
- **Existing epic** `testproject1-4zq` is the parent (or use sub-epics per strategy decision).
- **Sync after creation:** `bin/br-sync && git add .beads/ && git commit -m "beads: l4 ... " && git push`.
- **AgentMail project key:** `/Users/alinaaitova/Desktop/Testproject1` (referenced in beads' operational guidance only).
- **ZERO SCOPE DRIFT: 100% spec coverage, 100% test ID coverage, no red-flag language.**
