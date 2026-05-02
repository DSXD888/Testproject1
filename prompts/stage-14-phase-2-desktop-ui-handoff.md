# Stage 14 Phase 2 — Tauri Desktop UI for Operator Decision

ULTRATHINK and be meticulous.

## MISSION

Stage 14 Phase 1 (committed at `91a1484`) shipped the **CLI capture path** — operator types `--capture-decision`, `--capture-clarification`, etc. and the daemon writes to `pending_operator_intents` and `operator_input_overrides`. Phase 2 wraps that same data plumbing in a **Tauri desktop UI** so the operator works in a graphical control panel instead of a terminal. The data plumbing does NOT change — Phase 2 is presentation + IPC bridge only.

You will **build → validate → test → fix in a loop** until every numbered acceptance criterion (R.1–R.14 below) is GREEN with the operator able to: open the app, see all 7 process manifests, drill into a candidate, capture a clarification, trigger a pipeline re-run, and capture a decision — all without touching a CLI.

```
┌────────────────────────────────────────────────────────────┐
│                          THE LOOP                            │
│   1. Pre-flight (§0)                                         │
│   2. Implement: IPC commands + Manifest Inbox + Process View │
│      + Quick-Decide + Clarification form + drill-downs      │
│   3. Run: cd apps/desktop && bun tauri dev (or release)      │
│   4. Walk the §3 demo flow in the live app                  │
│   5. Validate against R.1..R.14 (queries inline)            │
│   6. Failures? → diagnose, fix, repeat from step 3           │
│   7. Quality gates: cargo fmt/check/clippy + bunx vitest +   │
│      Playwright E2E + per-file integration loop              │
│   8. All green → write the validation manifest               │
└────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## §0 — PRE-FLIGHT

| Claim | Verify |
|---|---|
| Phase 1 CLI is committed at `91a1484` and operational; `pending_operator_intents` has 2 rows (1 superseded, 1 withdrawn from the demo); `operator_input_overrides` has 1 active row for DGU at qty=250 | `git log --oneline HEAD~3..HEAD`; `sqlite3 state/state.db "SELECT COUNT(*) FROM pending_operator_intents"` |
| `apps/desktop/` exists with Tauri scaffolding: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src/App.tsx`, `src/pages/`, `src/hooks/`, `src/stores/`, `src/lib/` | `ls apps/desktop/ apps/desktop/src/ apps/desktop/src-tauri/` |
| **CRITICAL — EXISTING IPC SURFACE**: `apps/desktop/src-tauri/src/main.rs` already has 24 registered Tauri commands wired into `invoke_handler` (lines 63-87), including `get_bootstrap_snapshot`, `list_processes`, `get_process_detail`, **`capture_decision`** (with full impl at line 1569 + parallel `capture_decision_impl` at 1808 + 3 unit tests at 2294/2344/2383), `resolve_error`, `rerun_from_error`, `trigger_ingestion`, `restart_daemon`, `retry_job`, etc. The Phase 2 work must NOT add a duplicate `capture_decision` command — either reuse the existing one (read what `connection.capture_decision()` does at the daemon-bridge layer first) or rename Phase 2 commands to avoid collision. `CaptureDecisionParams` already exists with `process_id`, `disposition`, `reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`, `decision_note`, `idempotency_key` (apps/desktop/src-tauri/src/main.rs:288). | `grep -nE '#\[tauri::command\]|^fn (capture_|list_|get_)' apps/desktop/src-tauri/src/main.rs \| head -40` |
| **EXISTING PAGES**: `apps/desktop/src/pages/` already has `ContractList.tsx`, `ContractDetail.tsx`, `DaemonStatus.tsx`, `Errors.tsx`, `SessionViewer.tsx`, `Sessions.tsx`. The "Manifest Inbox" and "Process Detail View" screens described in D7 BELONG inside `ContractList.tsx` and `ContractDetail.tsx` (which already render contract/process data with `decision: { disposition, isStale, decidedAt }` already in the row shape — see ContractList.tsx:35-42). Extend rather than create new pages. **EXISTING HOOKS**: `use-control-plane.ts`, `use-daemon.ts`. **EXISTING STORES**: `session-buffer.ts`, `session-registry.ts`. Reuse the established patterns. | `ls apps/desktop/src/pages/ apps/desktop/src/hooks/ apps/desktop/src/stores/` |
| **EXISTING SHARED COMPONENT — `ManifestViewer`**: `packages/ui/manifest-viewer.tsx` already exports `ManifestViewer`, and `ContractDetail.tsx:6,492` already imports + renders it. The Stage 13 markdown rendering described in D7 ("Process Detail View renders the full Stage 13 markdown manifest") is LARGELY already implemented in the shared component. Phase 2 work should extend `ManifestViewer` if missing capabilities (Quick-Decide panel, Clarification form) need to live in the shared component, or wire new sibling components in `ContractDetail.tsx`. | `cat packages/ui/manifest-viewer.tsx \| head -30; grep -n ManifestViewer apps/desktop/src/pages/ContractDetail.tsx` |
| **CUSTOM ROUTING**: `App.tsx` does NOT use React Router. It uses a custom `RouteState` with `useState` + `popstate` listener (App.tsx:185-347). When adding the "Process Detail View" route (`/process/:id`), follow the existing pattern in `App.tsx`'s `currentRoute()` function — don't introduce React Router. | `grep -nE 'Route\|popstate\|setRoute' apps/desktop/src/App.tsx` |
| Tauri commands invoke the daemon via a `with_daemon_connection(...)` bridge (NOT subprocess spawn — see e.g. `capture_decision` at line 1569). The bridge calls `connection.capture_decision(&bridge_params, ...)`. Phase 2 must follow this same pattern when adding new commands | `grep -n "with_daemon_connection\|fn connection\|impl DaemonConnection" apps/desktop/src-tauri/src/main.rs \| head -10` |
| Bun is the package manager (`bun install`, `bun tauri dev`); the test runner is **`bunx vitest run`** (NOT `bun test` — verified in `apps/desktop/package.json` `test` script: `"test": "bunx vitest run"`). The Tauri Rust crate is named **`procintel-desktop`** (verified in `apps/desktop/src-tauri/Cargo.toml`). | `head -20 apps/desktop/package.json; grep '^name' apps/desktop/src-tauri/Cargo.toml` |
| 7 manifest rows exist: `proc-pc-19574d11-...`, `218726bd-...`, `3016d0da-...`, `3ba4e6b8-...`, `7046e8ec-...` (DGU), `7e2a6fb2-...`, `d23508ec-...` (Aggregates) | `sqlite3 state/state.db "SELECT process_id, stoplight, json_array_length(clarification_prompts_json) AS prompts FROM manifests"` |
| Phase 1 CLI subcommands invokable: `--capture-decision`, `--capture-clarification`, `--withdraw-clarification`, `--withdraw-decision`, `--list-decisions`. Daemon-internal helpers: `capture_operator_decision(state_db: &mut StateDb, cli: &Cli, process_id: &str)` (main.rs:18877), `capture_operator_clarification(state_db, cli, process_id)` (main.rs:19028), `withdraw_operator_clarification(state_db, cli, process_id)` (main.rs:19168), `withdraw_operator_decision(state_db, idempotency_key)` (main.rs:19228). **CRITICAL signature problem**: the first three take `&Cli` and read `disposition` / `idempotency_key` / `candidate_id` / `reason_code` / `payload` from CLI flag fields. **Tauri-side cannot construct a real `Cli`** without faking flag values. The Phase 2 work MUST refactor these helpers to take typed args directly (e.g. `capture_operator_decision(&mut StateDb, process_id: &str, disposition: &str, idempotency_key: &str, decision_note: Option<&str>) -> Result<Value>`), then have the CLI handler extract from `cli` and call the typed function. Without this refactor, you'll be reimplementing the helpers in Tauri-side (the bug behind the existing `capture_decision_impl` parallel write path). | `sed -n '18877,18895p' crates/daemon/src/main.rs; sed -n '19028,19055p' crates/daemon/src/main.rs` |
| **Existing daemon tests for Stage 14 (do NOT regress)**: `capture_decision_cli` (main.rs:23142), `capture_clarification_cli` (23167), `capture_decision_inserts_pending_operator_intents_row_with_correct_hashes` (23196), `stale_detection_marks_intent_stale_when_manifest_basis_hash_changes` (23646), `stale_detection_does_not_touch_superseded_or_withdrawn_intents` (23677). After refactoring helper signatures, these tests must still pass | `grep -n "fn capture_decision\|fn capture_clarification\|fn stale_detection" crates/daemon/src/main.rs` |
| `manifests` table column for stale detection is `manifest_basis_hash` (column 11), NOT `basis_hash`. The stale-banner UI watches for changes in this column (D9 reference) | `sqlite3 state/state.db "PRAGMA table_info(manifests)" \| grep -i hash` |
| `agent-mail` healthy; commits need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Parallel-safety note**: this handoff touches `apps/desktop/` almost exclusively, plus possibly small additions to `crates/daemon/src/main.rs` for new internal functions if the Tauri commands need them. If running concurrently with the Stage 11 hardening handoffs (which touch `main.rs` Stage 11 code), use a git worktree (`git worktree add ../testproject1-stage14-phase2 main`) for isolation; merge to main with rebase at the end. Stage 14 Phase 2's main.rs additions should be APPENDED (new functions / new IPC handlers) — do not modify existing functions.

---

## §1 — DECISIONS ALREADY MADE

D1–D6 from the Stage 14 Phase 1 handoff carry forward unchanged. The Phase 2 additions:

### D7 — UI architecture: Manifest Inbox + Process Detail View

Two main screens:

| Screen | Purpose |
|---|---|
| **Manifest Inbox** (default route) | List of all 7 processes. Each card shows: process title, buyer, contract value, stoplight badge (color-coded), suggested action, prompt count, last-decided status (queued/superseded/stale/withdrawn/none), updated-at timestamp |
| **Process Detail View** (route `/process/:id`) | The full Stage 13 manifest rendered as readable HTML. Sticky **Quick-Decide panel** on the right with disposition buttons + reasoning textarea + idempotency-key auto-generation. **Clarification form** (collapsible) listing each `clarification_prompt` from the manifest frontmatter with input controls per prompt_type (number / object / choice). **Drill-downs** for per-candidate detail, evidence trail (linked landed-cost row, supplier quotes), and decision history (audit chain) |

### D8 — IPC commands (Tauri Rust ↔ React TS)

**CRITICAL**: `capture_decision` already exists as a registered Tauri command (apps/desktop/src-tauri/src/main.rs:1569). The new Phase 2 commands MUST NOT collide with existing names. **Read the existing `invoke_handler` block (lines 63-87) and the existing impls before adding anything**. If the existing `capture_decision` already meets Stage 14 contract needs (it takes `CaptureDecisionParams` with `process_id`, `disposition`, `reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`, `decision_note`, `idempotency_key`), reuse it — don't add a parallel command.

Define these new `#[tauri::command]` functions in the Tauri Rust side, each invoking the daemon's existing internal helpers via the `with_daemon_connection(...)` bridge (NOT subprocess spawn):

| Command | Status | Wraps |
|---|---|---|
| `list_manifests()` | NEW | Reads `manifests` table; returns array of process rows with stoplight, suggested_action, prompt_count, manifest_basis_hash, latest decision status |
| `get_manifest(process_id)` | NEW (extends existing `get_process_detail` if shape overlaps — verify first) | Reads one manifest row; returns markdown body + parsed frontmatter (clarification_prompts) + linked Stage 12 / Stage 11 / Stage 10 evidence |
| `get_decision_history(process_id)` | NEW | Reads `pending_operator_intents` for the process, ordered by queued_at DESC. Includes superseded + withdrawn for audit chain |
| `get_active_overrides(process_id)` | NEW | Reads `operator_input_overrides` WHERE withdrawn_at IS NULL AND superseded_by_override_id IS NULL |
| `capture_decision(...)` | **EXISTS — REUSE** at apps/desktop/src-tauri/src/main.rs:1569; verify its signature already accepts Stage 14's contract; if so, no Tauri-side change needed (only frontend wiring). If the existing `capture_decision_impl` (line 1808) duplicates the daemon-internal `capture_operator_decision`, refactor BOTH to share one source of truth (recommended: route through daemon-internal `pub(crate)` helper) | (existing) |
| `capture_clarification(process_id, candidate_id, reason_code, payload, note)` | NEW | Calls `capture_operator_clarification()` (`main.rs:19028`) |
| `withdraw_clarification(process_id, candidate_id, reason_code)` | NEW | Calls `withdraw_operator_clarification()` (`main.rs:19168`) |
| `withdraw_decision(idempotency_key)` | NEW | Calls `withdraw_operator_decision()` (`main.rs:19228`) |
| `trigger_pipeline_rerun(process_id)` | NEW (consider whether `trigger_ingestion` at line ~1366 already covers part of this) | Invokes `run_landed_cost_live_loop` → `run_net_margin_gate_live_loop` → `run_manifest_generation_live_loop` for the specific process. Returns a stream of progress events the UI can render |

Each new Tauri command returns a typed JSON shape; React side consumes via `@tauri-apps/api/tauri`'s `invoke()`.

**Net new IPC commands**: 7 (or 6 if `get_manifest` is folded into `get_process_detail`). Plus existing `capture_decision` reused.

### D9 — UI state model

- **Stale signal surfacing**: `pending_operator_intents.status='stale'` is set by `stale_detection_*` logic (main.rs:18951, 19369) when the latest `manifests.manifest_basis_hash` for a process diverges from the intent's `reviewed_basis_hash`. The Process Detail View shows a banner "This decision was made against an earlier manifest version. Re-confirm or withdraw." The decision card in the Inbox shows a yellow indicator. (`ContractList.tsx:35-42` row shape already includes `decision: { isStale: boolean }` — surface this field.)
- **Re-run progress**: `trigger_pipeline_rerun` streams progress events (landed-cost done → net-gate done → manifest done). UI shows a progress modal; dismisses when manifest hash updates.
- **Optimistic vs pessimistic UI**: capture-decision is **pessimistic** (wait for IPC success before updating the visible status). Clarification capture is also pessimistic (small payloads, fast).

### D10 — Frontend stack

Use what the existing `apps/desktop/src/` scaffolding establishes. Don't introduce new heavyweight dependencies (no Redux unless already there; use `apps/desktop/src/stores/` patterns). Markdown rendering: pick a small library compatible with the existing scaffolding (e.g. `react-markdown`) OR sanitize-and-render via a minimal helper.

---

## §2 — IMPLEMENTATION ORDER

1. **Audit existing IPC surface** (READ-ONLY). Enumerate every existing `#[tauri::command]` in `apps/desktop/src-tauri/src/main.rs`, what `connection.<verb>(...)` it calls on the daemon-bridge side, and what its `*Params` struct looks like. Decide for each new Phase 2 command whether to (a) add new, (b) extend existing params, or (c) reuse as-is. This is where the `capture_decision` reconciliation question gets answered.
2. **IPC commands** (Rust side). Add the net-new commands as `#[tauri::command]` in `apps/desktop/src-tauri/src/main.rs`. Wire them into the `invoke_handler!` macro at line 63. Each command: use the `with_daemon_connection(...)` pattern (line 1582 et al for reference). `cargo check --all-targets` must pass.
3. **Decide page strategy**: extend existing `ContractList.tsx` and `ContractDetail.tsx` to render the new manifest data, OR add new pages and route between them. Reuse `use-control-plane.ts` / `use-daemon.ts` hooks rather than building parallel data fetching.
4. **Manifest Inbox screen** (likely an extension of `ContractList.tsx`). List view with cards. Verify against live data (7 cards expected).
5. **Process Detail View** (likely an extension of `ContractDetail.tsx`). Markdown rendering + Quick-Decide panel + decision history.
6. **Clarification form**. Maps `clarification_prompts` from frontmatter to input controls. Submits via `capture_clarification` IPC.
7. **Pipeline re-run trigger** + progress UI.
8. **Stale banner** + decision-history audit chain visualisation.
9. **Tests** (§4).
10. **Quality gates** + validation manifest.

---

## §3 — END-TO-END DEMO (the validation manifest must record this)

Drive the app in `bun tauri dev` mode and walk this flow. Take screenshots at each step (commit them to `docs/pipeline/stage-14-phase-2-screenshots/`).

1. Launch app. Manifest Inbox loads with 7 process cards. DGU card shows: stoplight=clarify, prompts=2, last decision=withdrawn (per Phase 1 demo state).
2. Click DGU card. Process Detail View opens. Markdown body renders sections A–F. Quick-Decide panel visible. Decision history shows 2 prior intents (dec-001 superseded, dec-002 withdrawn).
3. Open the Clarification form. See 2 prompts: `quantity_basis_unaligned` (current value: 250 m²) and `awaiting_rfq_quote`.
4. Update `quantity_basis_unaligned` to 150 m². Submit. UI shows success, override updated.
5. Click "Re-run pipeline". Progress modal cycles through landed-cost → net-gate → manifest. Modal closes. Manifest hash badge updates. The page re-renders with new freight estimates (or, in environments without CDP, the manifest still re-renders but freight columns may be unchanged — that's environmental).
6. In Quick-Decide panel: select `approve`, type reasoning "DGU at 150 m² is the most realistic spec; freight covered by Stage 11 estimate; pursue.", click Capture. UI shows "Decision captured" + decision ID. Decision history refreshes with the new intent at `status='queued'`.
7. Open another process (e.g., `proc-pc-d23508ec-...` Aggregates). Capture a `quantity_basis_undetermined` clarification (qty=500 kg). Re-run. Decide `watch`. Verify both processes' decisions visible in the Inbox cards.
8. Trigger an external state change: from a separate terminal, `cargo run -p daemon -- --ingest manifest`. Wait. Reload Inbox. Stale banner should appear on processes whose basis changed.
9. Withdraw a decision via UI. Verify decision history reflects withdrawal.

---

## §4 — REQUIRED TESTS

### Frontend (React TS, `apps/desktop/src/`)

| # | Test | Verifies |
|---|---|---|
| FT.1 | `manifest-inbox.test.tsx` — renders 7 cards from mocked `list_manifests` IPC | Inbox component renders correctly given known data |
| FT.2 | `manifest-inbox.test.tsx` — stoplight badge color maps correctly (clarify=yellow, pursue=green, pass=red, review=gray) | Visual contract |
| FT.3 | `process-detail.test.tsx` — renders manifest markdown + decision history | Detail page integration |
| FT.4 | `quick-decide.test.tsx` — disposition button click → invokes `capture_decision` IPC with correct args | UI ↔ IPC contract |
| FT.5 | `clarification-form.test.tsx` — number-type prompt → number input; choice-type prompt → select; object-type prompt → multi-field form | Prompt-to-control mapping |
| FT.6 | `clarification-form.test.tsx` — submit invokes `capture_clarification` IPC with correct payload shape per reason_code | Payload validation contract |
| FT.7 | `stale-banner.test.tsx` — when latest intent status='stale', banner renders | State-derived UI |
| FT.8 | `decision-history.test.tsx` — audit chain visualization (superseded → newer; withdrawn marked) | History rendering |

### Tauri Rust commands (`apps/desktop/src-tauri/src/`)

| # | Test | Verifies |
|---|---|---|
| RT.1 | `list_manifests_returns_all_manifest_rows` | Reads from a synthetic StateDb fixture, returns expected count |
| RT.2 | `get_manifest_returns_parsed_frontmatter` | Frontmatter `clarification_prompts` correctly parsed |
| RT.3 | `capture_decision_calls_daemon_internal_helper` | IPC command invokes `capture_operator_decision` (not subprocess) |
| RT.4 | `trigger_pipeline_rerun_streams_progress_events` | Events emitted in landed-cost → net-gate → manifest order |

### Playwright E2E (if scaffolding exists, otherwise document as deferred)

| # | Test | Verifies |
|---|---|---|
| ET.1 | Open app, click DGU card, capture clarification, re-run, capture decision — full §3 demo flow | End-to-end happy path |

**Test-count expectations**: 8 frontend tests + 4 Tauri Rust tests = 12 new. Plus existing tests must still pass.

---

## §5 — ACCEPTANCE CRITERIA — R.1 to R.14

| # | Criterion |
|---|---|
| R.1 | All NET-NEW Tauri IPC commands (7 if `get_manifest` is folded into existing `get_process_detail`; 8 otherwise) defined and registered in the Tauri builder; existing `capture_decision` reused (or refactored to share daemon-internal source of truth with `capture_operator_decision`); `cargo check -p procintel-desktop` passes; no duplicate Tauri command symbols |
| R.2 | `bun tauri dev` launches; Manifest Inbox renders 7 process cards from live state.db |
| R.3 | Stoplight badge colors correct: clarify=yellow, pursue=green, pass=red, review=gray |
| R.4 | Process Detail View renders the full Stage 13 markdown manifest with sections A–F visible |
| R.5 | Decision history visible per process: shows superseded + withdrawn intents in chronological order with status badges |
| R.6 | Clarification form renders one input per `clarification_prompt` in the manifest frontmatter; each input type matches `prompt_type` (number/object/choice) |
| R.7 | Submit clarification → UI shows success → DB has a new active row in `operator_input_overrides` with the entered payload |
| R.8 | Trigger Re-run → progress modal cycles → manifest hash updates → page re-renders with new content |
| R.9 | Capture Decision → DB has new `pending_operator_intents` row with `status='queued'`, `desired_disposition` matching button click, `decision_note` matching textarea content |
| R.10 | Stale banner appears within 2s of the manifest's basis_hash changing (test by triggering external re-run) |
| R.11 | Withdraw Decision via UI → `pending_operator_intents.status='withdrawn'` for the targeted intent |
| R.12 | All FT.1–FT.8 + RT.1–RT.4 tests pass; Playwright ET.1 passes if scaffolding is available, otherwise documented as deferred with reason |
| R.13 | Stage 11/12/13/14 Phase 1 invariants intact: 17 net-margin assessments still review; 7 manifests still manifest_ready; pending_operator_intents from Phase 1 demo intact (1 superseded, 1 withdrawn); operator_input_overrides DGU row intact |
| R.14 | Quality gates: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings`; `bunx vitest run` in `apps/desktop/`; `cargo test -p procintel-desktop` (Tauri-side Rust tests including new RT.1–RT.4); `cargo test -p daemon --bin daemon` (147 binary tests still pass — Phase 2 must not regress); per-file integration loop green across 39 daemon test files |

---

## §6 — VALIDATION QUERIES + COMMANDS

Live DB checks (run after demo):

```bash
sqlite3 -header -column state/state.db "
SELECT '=== R.7 latest active override for DGU ===';
SELECT product_candidate_id, reason_code, payload_json, provided_at
FROM operator_input_overrides
WHERE product_candidate_id LIKE '%glass-dgu%'
  AND withdrawn_at IS NULL AND superseded_by_override_id IS NULL;

SELECT '=== R.9 queued decisions captured via UI ===';
SELECT idempotency_key, desired_disposition, status, decision_note, queued_at
FROM pending_operator_intents
WHERE status = 'queued' ORDER BY queued_at DESC LIMIT 5;

SELECT '=== R.13 invariants ===';
SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY 1;
SELECT pipeline_state, COUNT(*) FROM pipeline_assessments GROUP BY 1;
SELECT stoplight, COUNT(*) FROM manifests GROUP BY 1;
"
```

Build / test:

```bash
cd apps/desktop
bun install
bunx vitest run                       # FT.1–FT.8 (the package.json `test` script wraps this)
# Smoke build (Tauri build is heavy; consider `bunx tsc --noEmit` from package.json `build` script first)
bunx tsc --noEmit -p tsconfig.json
cd ../..
cargo check -p procintel-desktop      # Tauri Rust crate type check
cargo test -p procintel-desktop       # RT.1–RT.4 (Tauri-side Rust tests; existing tests at apps/desktop/src-tauri/src/main.rs:2293+)
cargo test -p daemon --bin daemon     # 147 binary tests must still pass
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

---

## §7 — FAILURE-MODE PLAYBOOK

| Symptom | Cause | Fix |
|---|---|---|
| Tauri command panics on `state_db.connection()` | StateDb not opened or path wrong | Initialize StateDb in Tauri `setup` hook with the same `state_root` the CLI uses (default `./state`) |
| `cargo check -p procintel-desktop` errors on missing daemon helper | Tauri Rust side imports an internal helper from `daemon` crate, but the function is module-private | Either expose the helper as `pub(crate)` in daemon's main.rs, OR re-implement minimally in Tauri side, OR route the call through the existing `with_daemon_connection(...)` IPC bridge (the established pattern — see `apps/desktop/src-tauri/src/daemon_bridge.rs:562` for the existing `capture_decision` bridge implementation) |
| Markdown body renders with raw HTML or broken | Sanitiser too aggressive OR markdown parser doesn't handle YAML frontmatter | Strip the `---\n...\n---\n` header before passing to the markdown renderer; pass parsed frontmatter as a separate prop |
| Re-run progress modal hangs | `trigger_pipeline_rerun` doesn't return promptly OR errors aren't propagated | Wrap each pipeline step in a Tauri `emit` call so React can listen via `listen('pipeline-progress', ...)`. Errors must emit a `pipeline-error` event |
| Stale banner doesn't appear | UI doesn't poll OR doesn't subscribe to manifest changes | After `trigger_pipeline_rerun` returns, refetch `list_manifests` + `get_decision_history`; OR subscribe to a Tauri event the daemon emits on manifest writes |
| Modified Stage 11 / Stage 12 / Stage 13 code | Out of scope for Phase 2 | Stop. Phase 2 is UI + IPC only. If a daemon-side function is missing or broken, file a follow-up bead |
| `bunx vitest run` can't find a test file | New test file outside vitest's discovery glob | Check `apps/desktop/vitest.config.ts` (or the `test` field of `vite.config.ts`) for the include pattern. Existing tests like `apps/desktop/src/App.test.tsx` and `apps/desktop/src/hooks/use-control-plane.test.tsx` follow `*.test.tsx` co-located naming — match this convention |

---

## §8 — CONSTRAINTS

1. **Do not modify Stage 1–14 Phase 1 daemon code paths** beyond exposing internal helpers as `pub(crate)` for Tauri to call. No new business logic in Phase 2.
2. **`apps/desktop/` is the primary work directory.** Anything in `crates/daemon/` is at most visibility tweaks (`pub(crate)` exposures).
3. **No new daemon migrations.** All schema is in place from Phases 1–14.
4. **Use existing project conventions** for state management, routing, styling. Don't introduce Redux, MobX, etc. if they're not already present.
5. **Worktree isolation** if running parallel to the Stage 11 hardening handoffs: `git worktree add ../testproject1-stage14-phase2 main`.
6. **No silent test exclusion.** All test gates must pass: `bunx vitest run` (frontend), `cargo test -p procintel-desktop` (Tauri-side Rust), `cargo test -p daemon --bin daemon` (147 daemon binary tests), and the per-file integration loop across 39 daemon test files.
7. **No new direct DB writes from the React side.** All DB access goes through Tauri commands. The TS side is a thin client.

---

## §9 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/stage-14-phase-2-validation-manifest.md`. Same template shape as Phase 1 manifest. Add a `## Screenshots` section pointing to `docs/pipeline/stage-14-phase-2-screenshots/{step}.png`.

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon          # 147 tests must still pass
cargo test -p procintel-desktop            # RT.1–RT.4 + existing Tauri tests
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
cd apps/desktop && bun install && bunx vitest run
```

---

## §10 — REMINDERS

- ULTRATHINK and be meticulous.
- Phase 2 is **presentation + IPC bridge only**. The data model, state machine, and side-effect contracts are all set by Phase 1.
- The §3 demo is the highest-fidelity success signal — walk it manually, take screenshots, attach to the validation manifest.
- If Tauri scaffolding has surprises (existing routing, state stores, layout shells), respect them. Don't rebuild what's there.
- If parallel agents are working on Stage 11 hardening at the same time, use a worktree. Merge to main with rebase at the end.
- Ship Phase 2 once R.1–R.14 are green. Stage 14 is then operationally complete (CLI + UI). Stage 15 (bid_workflow) remains L6-blocked.
