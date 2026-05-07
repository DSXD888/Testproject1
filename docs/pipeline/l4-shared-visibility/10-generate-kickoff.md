# L4 Shared Visibility — 10 · Generate Kickoff + Push Prompts

## First Action

Invoke the `/beads-to-execution` skill.

---

## Your Mission

Generate the **kickoff prompt** (full agent onboarding), **push prompt** (continuation nudge), and **status YAML file** for L4 Shared Visibility. These are the documents agents will actually USE during implementation. They must be deeply feature-specific — reference actual file paths, actual types, actual commands, actual phase names.

**CRITICAL: The kickoff and push prompts MUST include the full AgentMail coordination workflow.** Multiple agents will work in parallel on L4. The prompts must leave ZERO ambiguity about:
- Organization model (FLAT — no hierarchy, agents pick next ready bead)
- AgentMail registration (exact MCP calls)
- Inbox check before picking work (situational awareness)
- Bead claim protocol (update status + reserve files + announce via mail)
- File reservation conflict handling
- Full execution loop (claim → reserve → announce → implement → verify → commit → push → complete → release → loop)
- Session landing protocol (when fully blocked)
- **Bead completion policy** (MUST fully complete every bead — no partial work)
- **Commit workflow** (complete bead fully → verify → stage specific files → commit with bead ID prefix → push immediately — work is NOT done until push succeeds)

---

## Source Documents

```
docs/strategy/layers/layer-4-shared-visibility.md                              — Spec (SOT #1)
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md       — TDD (SOT #2)
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md — Bootstrap (environment setup)
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md  — Strategy (SOT #3)
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md — Bulk-import doc (full bead inventory)
docs/strategy/cross-reference-index.md                                         — every L4 cross-ref
docs/strategy/layer-strategy.md                                                — cross-layer ordering
AGENTS.md                                                                      — project rules (bead CLI, branching, Stage Validation Test Gate, file deletion rules)
```

### Codebase (study for integration surface)
```
crates/daemon/src/db/sqlite.rs                              — 600 lines, where db modules live
crates/daemon/src/db/checkpoints.rs                         — 231 lines, parallel module
crates/daemon/src/validation/policy.rs                      — 959 lines (chokepoint)
crates/daemon/src/validation/executor.rs                    — 1217 lines (chokepoint)
crates/daemon/src/validation/manifest_writer.rs             — 54 lines
crates/daemon/src/jobs/runner.rs                            — 1027 lines (chokepoint, single enqueue boundary)
crates/daemon/src/ipc/handlers/decisions.rs                 — pending_operator_intents writer
crates/daemon/src/errors/manifest.rs                        — 386 lines, error_manifests writer
crates/daemon/src/main.rs                                   — 1.08 MB monolithic init (Phase 6 wiring)
crates/daemon/Cargo.toml                                    — net-new HTTP client (Phase 1)
package.json (root)                                         — workspaces; lint rule banning desktop/tauri imports
migrations/                                                 — flat sequence; L4 owns next NNNN_
infra/supabase/                                             — net-new directory (Phase 1)
apps/web/                                                   — net-new directory (Phase 5)
apps/desktop/src/pages/                                     — templates for apps/web pages
packages/ui/                                                — components apps/web reuses
skills/validation/base/                                     — template for 4 overlays
```

### Quality Bar (MATCH OR EXCEED THIS LEVEL OF DETAIL)
```
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-kickoff.md   — 366 lines, 12 sections — STUDY THE AGENTMAIL SECTION
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-push.md      — 75 lines — STUDY THE COMPACT WORK LOOP
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-status.yaml  — TEMPLATE for the L4 status YAML
```

---

## Output Files

| File | Path | Purpose | Target Lines |
|------|------|---------|--------------|
| Kickoff | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-kickoff.md` | Full agent onboarding | ~400-500 lines (L3 was 366; L4 covers more domains: sync engine + web app + enforcement) |
| Push | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-push.md` | Continuation nudge | ~75-110 lines (L3 was 75) |
| Status | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-status.yaml` | YAML pointer to all SOT docs | ~40-50 lines |

---

## Required Kickoff Sections (ALL mandatory — 15 sections in this order)

### 1. Mission

- Feature description: "Layer 4 — Shared Visibility. Make pipeline data visible to a second user (web viewer) and switch semantic validation from L3 dry-run to enforcement."
- Scope description: "L4 builds: (a) SQLite↔Supabase sync engine; (b) 8 Supabase migrations + RLS for 4 roles; (c) read-only Next.js web portal at `apps/web/` (7 files); (d) provisional-promotion buffer for enforcement-active slices; (e) 4 domain-specific validator overlays; (f) 3-corpora calibration harness."
- Completion milestone: "M4 — Two users view the same pipeline: desktop operator (read-write) + web viewer (read-only). Sync engine operational. Validation enforcement active on calibrated slices. All 18 DoD criteria green."
- Organization: "**FLAT.** No team hierarchy. Pick the next ready bead (`br ready --label layer-4`), implement it fully, move on. Never ask 'what should I work on next?' — `br ready` answers."
- Self-containment: "Beads are SELF-CONTAINED. Each bead has full file paths, type definitions, test IDs, ACs, dependency info. Implement from the bead alone. The spec is only needed as a tiebreaker."

### 2. AgentMail Registration (DO THIS FIRST)

```
**STOP.** Register with AgentMail NOW, before reading anything else. This makes you visible to the swarm and prevents conflicts. Everything else waits until you have your `agent_name`.

**Project key:** `/Users/alinaaitova/Desktop/Testproject1`

**Option A — Macro (preferred):**
```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l4-shared-visibility-executor",
  model="<your-model>",
  task_description="L4 Shared Visibility implementation"
)
```

**Option B — Granular:**
```
ensure_project(human_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l4-shared-visibility-executor",
  model="<your-model>",
  task_description="L4 Shared Visibility implementation"
)
set_contact_policy(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  policy="open"
)
```

After registration, you receive your `agent_name`. Use it for all subsequent calls. Then immediately:
```
fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="<your-name>", limit=20)
```
Read and acknowledge any pending messages before starting work. If another agent left a blocker notice, address it first.
```

### 3. Source-of-Truth Reading List (MANDATORY — before any code)

```
Read in this order:
1. `AGENTS.md` — agent rules, coordination, branching, Stage Validation Test Gate (mandatory full-test invocation pattern)
2. `docs/strategy/layers/layer-4-shared-visibility.md` — Spec (SOT #1, ~800 lines, 18 DoD criteria, 14 invariants, 4 sections)
3. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` — Strategy (SOT #3, phase ordering)
4. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — TDD (SOT #2, all L4-T test IDs)
5. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md` — environment prerequisites (Supabase, daemon HTTP client, apps/web wiring)
6. `br list --label layer-4 --json` — bead inventory (your work orders)
7. Codebase familiarity:
   - `crates/daemon/src/db/sqlite.rs` (600 lines) — what `open_state_db` does today
   - `crates/daemon/src/validation/policy.rs` (959 lines) — L3 dry-run policy (Phase 4 extends)
   - `crates/daemon/src/validation/executor.rs` (1217 lines) — validator skill spawner (Phase 4 extends with provisional promotion)
   - `crates/daemon/src/jobs/runner.rs` (1027 lines) — single enqueue boundary (Phase 4 extends with provisional path)
   - `crates/daemon/src/main.rs` (1.08 MB) — startup wiring (Phase 6 only)
   - `migrations/0001_sqlite_state_initial.sql` (545 lines) — L0 schema (no `local_outbox`!)
   - `migrations/0004_l2_pending_operator_intents.sql` (31 lines) — pending intents queue
   - `migrations/0005_l3_operational_hardening.sql` (119 lines) — L3 validation_verdicts
   - `apps/desktop/src/pages/{ContractList,ContractDetail,DaemonStatus}.tsx` — templates for apps/web pages
   - `packages/ui/{contract-card-frame,manifest-viewer,freshness-badge,pipeline-badge,error-summary-list}.tsx` — components apps/web reuses
   - `skills/validation/base/` — template for 4 L4 overlays
```

### 4. Implementation Boundary

```
**EXISTS (build on top of):**
- L0/L1/L2/L3 are fully built. All 22 SQLite tables, validation policy/executor/manifest-writer, error manifest system, IPC, PTY runtime, scheduler/cron, sources registry, validation_verdicts table, pending_operator_intents queue, all process-stage-status / heads tables, all push-owned column shapes (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`).
- Validator base skill in `skills/validation/base/` with full 6-file template.
- AgentMail at `http://127.0.0.1:8765` for coordination.
- Bead CLI `br` (NEVER `bd`) at `~/.local/bin/br`.

**DOES NOT EXIST (you're building):**
- `crates/daemon/src/db/sync.rs` — the entire sync engine module
- `infra/supabase/` directory — config + 8 migrations
- `apps/web/` directory — entire Next.js app + 7 source files
- SQLite tables: `local_outbox`, `sync_batches`, `pull_cursors` (NOT pre-reserved in L0 despite spec wording — L4 owns these)
- Supabase tables: 20 push-owned projections + 3 junction + 6 summary + 3 remote-owned + lease + command ledger + decision summaries
- HTTP client dependency (`reqwest`) in `crates/daemon/Cargo.toml` — daemon currently has NO HTTP client
- `apps/web` workspace integration in root `package.json`
- 4 validator overlay skill directories: `skills/validation/{procurement-ingestion,supplier-enrichment,document-acquisition,product-extraction}/`
- Calibration corpora at `tests/fixtures/calibration/{slice_id}/{clean,weird,mutated}/`
- Provisional staging mechanism (table or directory — chosen in spec)
- 4 new test files: `crates/daemon/tests/sqlite_schema.rs`, `sqlite_supabase_sync.rs`, `redaction_boundary.rs`, `apps/web/e2e/contracts.spec.ts`

Do NOT assume unbuilt infrastructure exists. The bead body lists what exists vs what to create.
```

### 5. Core Contracts / Non-Negotiables

```
1. **Push protocol is transactional + atomic.** Single `apply_push_batch(batch_json, writer_epoch)` RPC; partial-apply forbidden. Outbox + data in same SQLite transaction (commit_group_id grouping).
2. **Equal-version retry semantics — all 4 cases:** (a) `row_version` = remote + content_hash matches → no-op success; (b) `row_version` = remote + content_hash differs → hard conflict; (c) `row_version` = remote + 1 → apply; (d) anything else → reject.
3. **Pull-before-push at startup.** Pull remote-owned state FIRST, classify/coalesce pending intents, THEN resume push.
4. **CAS for operator decisions** with `base_remote_revision` check + durable `operator_decision_commands` ledger for idempotent retries.
5. **Writer fencing token (`writer_epoch`)** on every push batch. Stale-epoch batches rejected.
6. **Schema epoch guard** (`schema_epoch`) — mismatch pauses sync; local operation continues; minimum epoch-repair reproject path lands in L4.
7. **Provisional promotion buffer** for enforcement-active slices: worker outputs go to local-only staging, NOT directly to push-owned tables. Validator approves → promote. Validator rejects → quarantine as `AGENT_SUSPECT_OUTPUT`.
8. **Per-slice enforcement readiness** with all 7 prerequisites (Wilson-bound precision, rule-of-three false-pass, calibration thresholds, demotion triggers).
9. **Web read-only is a SECURITY BOUNDARY.** Viewer-scoped credentials only. RLS enforced at Supabase. `apps/web/src/lib/supabase.ts` is the SOLE Supabase boundary in the web app.
10. **`packages/ui` lint invariant:** no imports of `@tauri-apps/api`, `apps/desktop`, `crates/daemon`, `supabase`, `transport`. Web reuses UI components unchanged.
11. **No file deletion.** EVER. (AGENTS.md Rule 1.)
12. **Single-edit chokepoints:** `main.rs`, `runner.rs`, `policy.rs`, `executor.rs`, `decisions.rs`, `errors/manifest.rs`, daemon `Cargo.toml`, root `package.json`, `packages/ui/freshness-badge.tsx` — at most ONE bead per chokepoint at a time.
13. **Stage Validation Test Gate** (AGENTS.md): `cargo test -p daemon --bin daemon` is INSUFFICIENT. Iterate every `crates/daemon/tests/*.rs` file. (See Quality Gates below.)

If code conflicts with these contracts, the contract wins.
```

### 6. Bead Completion Policy (CRITICAL — verbatim)

```
**You MUST fully complete every bead you claim.** No partial completion. No "I'll finish this later." No moving on to another bead while one is in progress.

A bead is COMPLETE when:
1. All code changes described in the bead are implemented
2. All tests described in the bead pass (the specific L4-T-* tests it claims)
3. All build/lint/test verification commands pass with zero warnings
4. Code is committed with proper bead ID prefix
5. Bead is marked complete (`br close <bead-id> --reason "Completed"`)
6. Beads synced (`bin/br-sync && git add .beads/ && git commit && git push`)
7. Completion announced via AgentMail
8. File reservations released

If you hit a blocker during a bead:
- DO NOT abandon the bead and move to another
- DO NOT mark it complete with missing pieces
- DO NOT do mid-way commits of incomplete work
- Diagnose the blocker. Read the error. Understand the root cause.
- Fix the blocker yourself — you have the full codebase available
- Only if the blocker is genuinely impossible to resolve (missing infrastructure not in your bead, another agent's bead in-flight that you depend on), announce the specific issue in AgentMail with file path, line number, and what's wrong, then wait for resolution

**Work is NOT complete until `git push` succeeds.** NEVER stop before pushing — that leaves work stranded locally.
```

### 7. Work Loop (Full ASCII Diagram)

```
ONE-TIME SETUP:
  1. AgentMail registration (above)
  2. Read AGENTS.md + spec + strategy + TDD + bootstrap
  3. `br ready --label layer-4` → understand the bead graph

MAIN LOOP (repeat until all L4 beads complete):

  ┌─ PICK ──────────────────────────────────────────────┐
  │ br ready --label layer-4 --json                     │
  │ Pick the highest-priority ready bead.               │
  │ Read the bead description FULLY before any code.    │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ VERIFY AVAILABLE ─────────────────────────────────┐
  │ # GUARD: prevent duplicate claims                  │
  │ br show <bead-id> --json                           │
  │ # → if status=in_progress, SKIP this bead          │
  │ search_messages(                                   │
  │   project_key="/Users/alinaaitova/Desktop/Testproject1",│
  │   agent_name=<your-name>,                          │
  │   query="[<bead-id>] Start"                        │
  │ )                                                  │
  │ # → if claim message found, SKIP this bead         │
  └────────────────────────────────────────────────────┘
           │
  ┌─ CLAIM + ANNOUNCE ──────────────────────────────────┐
  │ # Discover peers (refresh between beads)            │
  │ list_contacts(                                      │
  │   project_key="/Users/alinaaitova/Desktop/Testproject1",│
  │   agent_name=<your-name>                            │
  │ )  # → e.g. ["BlueLake", "RedPeak"]                 │
  │                                                      │
  │ br update <bead-id> --status=in_progress             │
  │                                                      │
  │ file_reservation_paths(                              │
  │   project_key="/Users/alinaaitova/Desktop/Testproject1",│
  │   agent_name=<your-name>,                            │
  │   paths=[<files this bead touches>],                 │
  │   ttl_seconds=3600,                                  │
  │   exclusive=true,                                    │
  │   reason="<bead-id>"                                 │
  │ )                                                    │
  │                                                      │
  │ send_message(                                        │
  │   project_key="/Users/alinaaitova/Desktop/Testproject1",│
  │   sender_name=<your-name>,                           │
  │   to=["<peers>"],   # NEVER "broadcast"              │
  │   thread_id="<bead-id>",                             │
  │   subject="[<bead-id>] Start: <title>",              │
  │   body_md="Claiming bead. Files: <list>."            │
  │ )                                                    │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ IMPLEMENT (TDD) ──────────────────────────────────┐
  │ 1. Study target files — understand current state    │
  │ 2. Read spec L4-FR section referenced in bead       │
  │ 3. Write/extend tests FIRST (the L4-T-* IDs claimed)│
  │ 4. Implement code to make tests pass                │
  │ 5. Verify ALL ACs from the bead body                │
  │                                                      │
  │ DO NOT cut corners. DO NOT skip tests.               │
  │ DO NOT leave TODOs. DO NOT do mid-way commits.       │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ VERIFY ────────────────────────────────────────────┐
  │ # Stage Validation Test Gate (AGENTS.md)            │
  │ cargo fmt --check                                    │
  │ cargo check --all-targets                            │
  │ cargo clippy --all-targets -- -D warnings            │
  │ cargo test -p daemon --bin daemon                    │
  │ for t in crates/daemon/tests/*.rs; do                │
  │   cargo test --test "$(basename "$t" .rs)" || exit 1 │
  │ done                                                 │
  │ bun run test                                         │
  │ # If your bead touches apps/web:                     │
  │ (cd apps/web && bun run test:e2e)                    │
  │                                                      │
  │ Fix ALL failures before proceeding.                  │
  │ Use debug builds for fast iteration.                 │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ COMMIT ────────────────────────────────────────────┐
  │ ubs $(git diff --name-only --cached)  # pre-commit  │
  │ git add <specific-files-only>                        │
  │ git commit -m "<bead-id>: <short description>"       │
  │ git pull --rebase                                    │
  │ git push                                             │
  │ # Work is NOT done until push succeeds.              │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ COMPLETE + RELEASE ────────────────────────────────┐
  │ br close <bead-id> --reason "Completed"              │
  │ bin/br-sync                                          │
  │ git add .beads/                                      │
  │ git commit -m "beads: close <bead-id>"               │
  │ git pull --rebase && git push                        │
  │                                                      │
  │ send_message(                                        │
  │   project_key="/Users/alinaaitova/Desktop/Testproject1",│
  │   sender_name=<your-name>,                           │
  │   to=["<peers>"],   # NEVER "broadcast"              │
  │   thread_id="<bead-id>",                             │
  │   subject="[<bead-id>] Complete: <title>",           │
  │   body_md="Commit: <hash>. All checks pass."         │
  │ )                                                    │
  │                                                      │
  │ release_file_reservations(                           │
  │   project_key="/Users/alinaaitova/Desktop/Testproject1",│
  │   agent_name=<your-name>                             │
  │ )                                                    │
  └──────────────────────────────────────────────────────┘
           │
        LOOP → back to PICK
```

### 8. Bead Inventory (by Phase)

`{ENRICHMENT: bead metadata}` — after step 08 completes, populate this section with:
- Phase 1 — Schema Foundation (~5-8 beads, P0)
- Phase 2 — Push Engine (~8-12 beads, P0/P1)
- Phase 3 — Pull Engine + CAS (~6-8 beads)
- Phase 4 — Validation Enforcement (~10-15 beads)
- Phase 5 — Web Portal (~7-10 beads)
- Phase 6 — Integration & Hardening (~6-10 beads)
- Total: ~40-65 task beads

For each phase, list a 3-column table (ID / Title / Depends On) plus an ASCII DAG showing parallelism.

**Enrichment fallback (if step 08 enrichment was skipped):** if the `{ENRICHMENT: bead metadata}` marker is still present at step 10 invocation time, populate it from the live bead corpus directly:

```bash
# Capture the inventory snapshot
br list --label layer-4 --json | jq -r '
  .issues
  | group_by(.labels[] | select(startswith("l4-phase-")))
  | map({
      phase: (.[0].labels[] | select(startswith("l4-phase-"))),
      count: length,
      beads: [.[] | {id: .id, title: .title, priority: .priority, status: .status}]
    })
' > /tmp/l4-bead-inventory.json

# Read counts per phase
jq '[.[] | {phase: .phase, count: .count}]' /tmp/l4-bead-inventory.json
```

Then walk the JSON output and produce the per-phase tables in §8 of the kickoff. The kickoff MUST contain the actual current bead IDs and titles — do not leave the section as a placeholder when the bead corpus exists.

### 9. Quality Gates (Actual Commands)

```bash
# Rust workspace
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done

# TypeScript packages
bun run test
bun run lint
bun run build

# apps/web (if your bead touches it)
(cd apps/web && bun run test:e2e)

# Pre-commit (UBS)
ubs $(git diff --name-only --cached)
```

Constitutional constraints:
- `#![forbid(unsafe_code)]` (Rust)
- No `unwrap()` in production code paths
- Rust 2024 nightly toolchain (`rust-toolchain.toml`)
- Bun only for TypeScript layers (NEVER npm/yarn/pnpm)
- `packages/ui` lint rule: NO imports of `@tauri-apps/api`, `apps/desktop`, `crates/daemon`, `supabase`, `transport`

### 10. File-Surface Map by Phase

For each phase, list which specific files are in scope. Prevents unnecessary exploration.

| Phase | Key Files |
|-------|-----------|
| 1 Schema Foundation | `migrations/0012_l4_sync_state.sql` (or chosen NNNN), `infra/supabase/{config.toml, migrations/0001-0008}`, `crates/daemon/Cargo.toml` (`reqwest` add), `package.json` (root, workspaces edit) |
| 2 Push Engine | `crates/daemon/src/db/sync.rs` (push half), Supabase function `apply_push_batch`, `crates/daemon/tests/sqlite_supabase_sync.rs` (push subset) |
| 3 Pull Engine + CAS | `crates/daemon/src/db/sync.rs` (pull half), Supabase function `update_operator_decision`, `crates/daemon/src/ipc/handlers/decisions.rs` (CAS conflict event extension), `crates/daemon/tests/sqlite_supabase_sync.rs` (pull/CAS subset) |
| 4 Validation Enforcement | `crates/daemon/src/validation/policy.rs` (per-slice readiness), `crates/daemon/src/validation/executor.rs` (quarantine path), `crates/daemon/src/jobs/runner.rs` (provisional write path), `crates/daemon/src/errors/manifest.rs` (AGENT_SUSPECT_OUTPUT), `skills/validation/{4 overlays}/` (6 files each), `tests/fixtures/calibration/...` |
| 5 Web Portal | `apps/web/src/{app,lib}/*` (7 files), `apps/web/e2e/contracts.spec.ts`, `apps/web/{tsconfig.json,playwright.config.ts,package.json}`, `packages/ui/freshness-badge.tsx` (3-state extension) |
| 6 Integration & Hardening | `crates/daemon/src/main.rs` (wiring), `crates/daemon/tests/redaction_boundary.rs`, `crates/daemon/tests/sqlite_schema.rs` (final), `infra/supabase/migrations/0004_rls_policies.sql` (final RLS audit) |

### 11. Dependency Spine

```
Phase 1 (Schema) ──┬── Phase 2 (Push) ──┐
                   ├── Phase 3 (Pull/CAS) ──┐
                   └── Phase 5 (Web skeleton) ──┐ → Phase 6 (Integration) → DoD
                                  ↑              │
                            Phase 4 (Enforcement)─┘
```

Trust `br ready --label layer-4`, but use this spine as your mental model.

### 12. AgentMail Quick Reference

```
**CRITICAL — `<peers>` IS A PLACEHOLDER, NOT A LITERAL.** Agent Mail has NO broadcast. `"broadcast"` is NOT a valid recipient. The `<peers>` token in every example below is a placeholder for the actual list returned by `list_contacts(project_key, agent_name)` — substitute the literal list before sending. Example:

  ```
  # WRONG (sends to a peer literally named "<peers>" which doesn't exist):
  send_message(..., to=["<peers>"], ...)

  # RIGHT (substitute actual peer names from list_contacts):
  peers = list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="<your-name>")
  # → returns e.g. ["BlueLake", "RedPeak"]
  send_message(..., to=["BlueLake", "RedPeak"], ...)
  ```

Refresh `<peers>` between beads (other agents may register or leave). NEVER use `to="broadcast"` — Agent Mail will reject it.

# Discover peers
list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)

# Check inbox (between beads)
fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>, limit=10)

# Reserve files
file_reservation_paths(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name=<your-name>,
  paths=[<files>],
  ttl_seconds=3600, exclusive=true, reason="<bead-id>"
)

# Claim (to=<peers> MANDATORY; never "broadcast")
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<bead-id>", subject="[<bead-id>] Start: <title>")

# Complete
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<bead-id>", subject="[<bead-id>] Complete: <title>", body_md="Commit: <hash>. All checks pass.")

# Blocker
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<bead-id>", subject="[<bead-id>] BLOCKED: <issue>",
  body_md="File: <path>, Line: <N>. Issue: <desc>. Needed: <what>")

# Release
release_file_reservations(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)
```

### 13. Session Landing Protocol (when fully blocked)

This section MUST be present in the kickoff (L3's analogous §10, ~39 lines). Use this format:

```markdown
## 13. Session Landing Protocol

If you are FULLY BLOCKED — every ready bead conflicts with another agent's reservation, or every remaining bead in your filter has unresolved upstream blockers — DO NOT idle, DO NOT spin retries. Land the plane:

1. **Stop claiming new work.** Do not pick another bead.
2. **Push any in-progress changes.**
   ```bash
   git add <files-touched-this-session>
   git commit -m "wip: <bead-id-or-context> — handoff (incomplete)"
   git pull --rebase
   git push
   ```
3. **Sync beads.**
   ```bash
   bin/br-sync
   git add .beads/
   git commit -m "beads: handoff sync"
   git pull --rebase && git push
   ```
4. **Update bead status to reflect reality.** If you partially implemented a bead, set it back to `open` with a note describing the blocker:
   ```bash
   br update <bead-id> --status=open
   br comment add <bead-id> --body "Handoff: blocked on <reason>. Files: <list>. Next step: <action>."
   ```
5. **Release file reservations.**
   ```
   release_file_reservations(
     project_key="/Users/alinaaitova/Desktop/Testproject1",
     agent_name=<your-name>
   )
   ```
6. **Announce handoff via AgentMail** (NOT "broadcast"; use peers from `list_contacts`):
   ```
   send_message(
     project_key="/Users/alinaaitova/Desktop/Testproject1",
     sender_name=<your-name>,
     to=["<peers>"],
     thread_id="l4-handoff-<date>",
     subject="[l4] Session handoff: <N> beads done, blocked on <reason>",
     body_md="Done: <bead-ids>. In-progress: <bead-id> at <state>. Blocked on: <specific-reason>. Files needing follow-up: <list>."
   )
   ```
7. **Verify git is clean.**
   ```bash
   git status   # MUST show "up to date with origin" + "nothing to commit"
   ```
8. **Stop.** Another agent will pick up from here. Do NOT continue.

If `git push` is rejected (rebase conflict from another agent's push), `git pull --rebase` and resolve, then retry. Never `git reset --hard` or `git push --force`.
```

### 14. Critical Rules (numbered list)

```
1. Beads are SELF-CONTAINED — implement from the bead alone; spec is tiebreaker only
2. NEVER delete files (AGENTS.md Rule 1) — even ones you created
3. NEVER stash other agents' work — concurrent agents' commits are normal
4. Use `br` (NOT `bd`); use `bin/br-sync` (NOT raw `br sync`)
5. EXTEND-don't-fork: `validation/policy.rs`, `validation/executor.rs`, `jobs/runner.rs`, `ipc/handlers/decisions.rs`, `errors/manifest.rs` — extend in place
6. Schema first: `migrations/0012_l4_sync_state.sql` (or chosen NNNN) is the root of the dependency DAG
7. `packages/ui` lint invariant: NO imports of `@tauri-apps/api`, `apps/desktop`, `crates/daemon`, `supabase`, `transport`
8. `apps/web/src/lib/supabase.ts` is the SOLE Supabase boundary in the web app
9. Web is read-only — viewer-scoped credentials only — security boundary, not UI convention
10. Provisional promotion buffer for enforcement slices is constitutional, NOT optional
11. `main.rs` edits are Phase-6-only integration beads (single-edit chokepoint)
12. Stage Validation Test Gate from AGENTS.md is the minimum gate — iterate every `crates/daemon/tests/*.rs`
13. Do NOT ask what to work on — `br ready --label layer-4` answers
14. Work is NOT done until `git push` succeeds
15. After every bead: `bin/br-sync && git add .beads/ && git commit && git push`
```

### 15. Quick Reference (footer)

| | |
|---|---|
| Spec | `docs/strategy/layers/layer-4-shared-visibility.md` |
| TDD | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` |
| Strategy | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` |
| Bootstrap | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md` |
| Bulk import | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md` |
| Bead prefix | `l4` |
| Bead CLI | `br` |
| Bead sync | `bin/br-sync` |
| AgentMail key | `/Users/alinaaitova/Desktop/Testproject1` |
| Branch | `main` (no feature branches) |

---

## Required Push Sections (ALL mandatory)

The generated push prompt at `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-push.md` MUST include these sections (~75-100 lines total):

### 1. Re-Orient
- Re-read AGENTS.md
- Feature: L4 Shared Visibility — sync engine + web portal + validation enforcement
- Spec/strategy/TDD paths
- "Your work orders are beads: `br ready --label layer-4 --json`"
- AgentMail registration (compact: project key + macro_start_session)

### 2. Core Contracts (compact reminder)
- Push protocol transactional + atomic (single RPC, partial-apply forbidden)
- Equal-version retry — 4 cases
- Pull-before-push at startup
- CAS with command ledger
- Writer fencing (`writer_epoch`)
- Schema epoch guard
- Provisional promotion for enforcement slices
- Web read-only is a security boundary
- Single-edit chokepoints (`main.rs`, `runner.rs`, `policy.rs`, `executor.rs`, daemon `Cargo.toml`, root `package.json`)

### 3. Bead Completion Policy (compact, firm)
- Every bead fully completed before moving on
- Complete = code + tests + checks + committed + announced + reservations released + bead closed + .beads/ pushed
- If blocked: diagnose, fix yourself, escalate via AgentMail only if blocker is in another agent's code

### 4. Work Loop (compact)
```
1. Check inbox (fetch_inbox) → acknowledge messages
2. br ready --label layer-4 --json → pick highest-priority ready bead
3. Read bead FULLY → study target files
4. CLAIM: br update <id> --status=in_progress + file_reservation_paths(...) + send_message(to=["<peers>"])
   # <peers> from list_contacts — NEVER "broadcast"
5. IMPLEMENT: TDD — extend tests first, then code. Implement ALL of it. No mid-way commits.
6. VERIFY: cargo {fmt,check,clippy,test (full gate)} + bun run {test,lint} + Playwright if web
7. COMMIT: ubs + git add <files> + git commit -m "<bead-id>: <desc>" + git pull --rebase && git push
8. COMPLETE: br close <id> + bin/br-sync + git add .beads/ + git commit + git push + send_message(to=["<peers>"]) + release_file_reservations
9. LOOP → step 1
Do not stop to ask what to work on next. Pick the next ready bead.
```

### 5. Verification Commands (compact, per stack)
```bash
# Rust full gate
cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon && for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
# TS
bun run test && bun run lint && bun run build
# Web e2e (if touched)
(cd apps/web && bun run test:e2e)
# Pre-commit
ubs $(git diff --name-only --cached)
```

### 6. Dependency Spine (one-line per phase)
- Phase 1 → Phase 2 (push) | Phase 3 (pull/CAS) | Phase 5 (web) — parallel after Phase 1
- Phase 4 (enforcement) depends on Phase 1 + extends `runner.rs`/`policy.rs`/`executor.rs`
- Phase 6 (integration) wires `main.rs` startup; depends on most prior phases

### 7. File Surfaces (compact table)
Same as kickoff §10 but condensed.

### 8. AgentMail Quick Reference (compact)
Claim/complete/blocker/release patterns. NEVER "broadcast".

### 9. Session Landing Protocol (when fully blocked)
- Push all work
- Release all reservations
- Announce handoff via AgentMail with status of what's done and what's blocked
- Update any bead status to reflect actual state

---

## Required Status YAML

`docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-status.yaml`:

```yaml
feature: l4-shared-visibility
status: in-progress     # or 'complete' after archive
started: "2026-05-XX"
completed: null         # set on archive
spec: docs/strategy/layers/layer-4-shared-visibility.md
tdd: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md
strategy: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md
bootstrap: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md
bulk_import: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md
bead_prefix: l4
bead_cli: br
bead_sync_cmd: bin/br-sync
existing_epic: testproject1-4zq
total_beads_planned: ~50         # populate from {ENRICHMENT}
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
    role: "Full agent onboarding (~350 lines, 14 sections)"
  push:
    path: docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-push.md
    role: "Continuation nudge (~75-100 lines, 9 sections)"

archived: null    # set on archive
```

---

## NO sub-optimization — these ARE the deliverables.

The kickoff/push/status are the actual documents agents will use during implementation. They are the deliverable, not artifacts of self-improvement. After this step generates them, a human MUST review them before authorizing execution (see EXECUTION-GUIDE.md "Mandatory Stops").

---

## Step-by-Step Workflow for the Generator Agent (You)

### Step 0: Read Source Documents (MANDATORY)

1. All 5 SOT docs (spec, TDD, strategy, bootstrap, bulk-import) — **READ FULLY**
2. AGENTS.md — **READ FULLY** (for Stage Validation Test Gate, file deletion rules, branching, beads CLI)
3. L3 kickoff at `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-kickoff.md` — **STUDY THE STRUCTURE** (366 lines, 12 sections)
4. L3 push at `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-push.md` — **STUDY THE STRUCTURE** (75 lines)
5. L3 status YAML at `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-status.yaml` — **STUDY THE FORMAT**
6. `br list --label layer-4 --json` — get the actual bead inventory for §8 enrichment

### Step 1: Invoke `/beads-to-execution`

Inputs:
- Spec, TDD, Strategy, Bootstrap, Bulk-import — all 5 paths
- Bead inventory: `br list --label layer-4 --json`
- Output kickoff: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-kickoff.md`
- Output push: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-push.md`
- Output status: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-status.yaml`
- Project key: `/Users/alinaaitova/Desktop/Testproject1`
- Bead prefix: `l4`
- Bead CLI: `br`
- Quality bar: L3 kickoff/push/status

### Step 2: Verify Output

After the skill completes, verify:

- [ ] Kickoff exists, ~350-400 lines, 14 sections present
- [ ] Push exists, ~75-100 lines, 9 sections present
- [ ] Status YAML exists, ~40-50 lines, all SOT paths correct
- [ ] AgentMail registration block uses exact project key `/Users/alinaaitova/Desktop/Testproject1`
- [ ] Work loop ASCII diagram includes the duplicate-claim guard (search_messages + br show)
- [ ] Bead completion policy is verbatim from §6 above
- [ ] Quality gates list the FULL Stage Validation Test Gate (cargo full gate + iterate test files + bun + Playwright + UBS)
- [ ] §8 Bead Inventory enriched from actual `br list --label layer-4` output
- [ ] §10 File-Surface Map is concrete (specific file paths per phase)
- [ ] §13 Session Landing Protocol has full 8-step landing procedure
- [ ] §14 Critical Rules has all 15 rules
- [ ] §15 Quick Reference footer present
- [ ] AgentMail Quick Reference uses project key explicitly in every call (no placeholders)
- [ ] No "broadcast" recipient anywhere — all `to=["<peers>"]` with peers from list_contacts
- [ ] Sub-optimization absent — these are deliverables, not improved artifacts

### Step 3: Submit for Human Review

After generation, the EXECUTION-GUIDE has a MANDATORY STOP. The human reviews:
- The kickoff is reasonable for this codebase
- The work loop is unambiguous
- The bead inventory is accurate
- The file-surface map is correct
- The verification commands match the actual repo

Once authorized, agents claim L4 beads and execute. The push prompt is reused continuously by continuation agents. The kickoff is for fresh agents who haven't seen this work.

---

## Key Reminders

- **NO sub-optimization.** These are deliverables, not improved artifacts.
- **AgentMail project key is `/Users/alinaaitova/Desktop/Testproject1`** — embed verbatim, no placeholders.
- **Bead CLI is `br`** — never `bd`. Sync via `bin/br-sync`.
- **The work loop must include the duplicate-claim guard** (search_messages + br show).
- **The Stage Validation Test Gate** from AGENTS.md is the minimum quality gate — iterate every `crates/daemon/tests/*.rs` file.
- **All 14 critical rules** from kickoff §13 must be present.
- **`packages/ui` lint invariant** must be called out (no imports of `@tauri-apps/api`, `apps/desktop`, `crates/daemon`, `supabase`, `transport`).
- **`apps/web/src/lib/supabase.ts` is the SOLE Supabase boundary** in the web app.
- **Web read-only is a security boundary**, not UI convention — viewer-scoped credentials only.
- **Phase 6 is an integration phase, NOT a polish phase.**
- **Existing epic `testproject1-4zq`** must be referenced; sub-epic structure (if used) must be reflected in §8.
