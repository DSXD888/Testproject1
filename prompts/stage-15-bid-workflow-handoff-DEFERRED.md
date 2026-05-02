# Stage 15 — Bid Workflow

> # ⛔ DO NOT RUN THIS HANDOFF UNTIL LAYER 6 IS BUILT ⛔
>
> Stage 15 (bid_workflow) is the final pipeline stage that takes an
> approved operator decision (`pending_operator_intents.status='queued'`,
> `desired_disposition='approve'`) and walks it through bid drafting →
> submission → tracking. This work CANNOT begin until Layer 6
> (Request Pipeline Automation) is built, because the bid drafting
> step depends on L6's commercial-intelligence templates + decision
> policies + portal-automation primitives.
>
> Per project memory (2026-05-02): L4-L6 are not yet built. The
> per-stage prerequisites listed in §0 must ALL be in place before
> any work in this handoff begins.
>
> If you are an agent considering picking this up: confirm with the
> human first that L6 has shipped + the §0 pre-flight items below
> are all GREEN. Otherwise stop and report back.

ULTRATHINK and be meticulous.

---

## MISSION (when L6 ships)

After Stage 14 ships the operator's go/no-go decision, Stage 15 executes the bid workflow:

1. Take an approved intent (`status='queued'`, `desired_disposition='approve'`).
2. Generate bid documents from the manifest evidence + supplier quote + operator's reasoning + L6 commercial templates.
3. Submit (or simulate submission) via the source portal's tender-response interface.
4. Track submission state through the bid lifecycle (drafting → reviewing → submitted → confirmed → won/lost/withdrawn).

**Critical scoping (per project owner)**: Stage 15 is **request pipeline automation**, NOT real-money commitment. The bid drafting + submission steps are reviewable artifacts that the operator confirms before any real submission goes out. The default state machine has a `reviewing` stage between `drafting` and `submitted` — operator must approve the drafted bid before submission. No L6 + L5 + L4 hooks bypass this.

The schema is already provisioned (`bid_workflow_states` table exists from `0008` or earlier migration — verify in §0). The work is: skills + daemon runner + CLI/UI + tests.

You will **build → validate → test → fix in a loop** until every numbered acceptance criterion is GREEN.

```
┌────────────────────────────────────────────────────────────┐
│                          THE LOOP                            │
│   1. Pre-flight (§0): VERIFY L4-L6 ARE BUILT                │
│   2. Implement: bid-workflow skills + daemon runner +        │
│      CLI subcommands + Phase 2-style UI tab                 │
│   3. Drive on a real approved intent (DGU if it has         │
│      a queued+approve intent)                                │
│   4. Validate against R.1..R.N (queries + tests inline)     │
│   5. Failures? → diagnose, fix, repeat from step 3           │
│   6. Quality gates: full test suite per AGENTS.md            │
│   7. All green → write the validation manifest               │
└────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## §0 — PRE-FLIGHT (CRITICAL — DO NOT SKIP)

Before any code is written, ALL of these must be GREEN. If any fails, STOP and ask the human.

| Claim | Verify | Status check |
|---|---|---|
| **L4 (validation pipeline) is built** — validator skills exist + run manifests are emitted as artifacts + validation_verdicts table is populated | `ls skills/validators/ 2>/dev/null \| head; sqlite3 state/state.db "SELECT COUNT(*) FROM validation_verdicts"` | If both empty → L4 not built → STOP |
| **L5 (sync engine) is built** — push protocol writes from local SQLite to Supabase; pull protocol reads remote-owned tables; transactional outbox active | inspection of code in `crates/daemon/src/sync/` (or wherever L5 lives) | If no sync code → L5 not built → STOP |
| **L6 (commercial intelligence) is built** — bid-template skills exist, decision-policy engine active, portal-submission skills wired | `ls skills/bid-templates/ 2>/dev/null; ls skills/portal-submission/ 2>/dev/null` | If neither exists → L6 not built → STOP |
| `bid_workflow_states` table exists with columns: `process_id` (PK FK to procurement_processes), `bid_state` (default 'not_started'), `bid_scope_json`, `review_required`, `bid_job_id`, `started_at`, `completed_at`, `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `updated_at`, `created_at` | `sqlite3 state/state.db ".schema bid_workflow_states"` | Verified existing (2026-05-02) |
| Stage 14 demo state intact: at least 1 `pending_operator_intents` row with `status='queued'` and `desired_disposition='approve'` exists for testing | `sqlite3 state/state.db "SELECT COUNT(*) FROM pending_operator_intents WHERE status='queued' AND desired_disposition='approve'"` | If 0 → drive Stage 14 to create one before starting |
| Operator can verify each portal-submission step in advance — no real-money or commitment-creating action runs without approval | code design choice; document in §1 |
| `agent-mail` healthy; commits need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Parallel-safety**: this handoff modifies `crates/daemon/src/main.rs` heavily (new runner + CLI subcommands), introduces new skill folders, and touches the desktop UI for the bid-workflow tab. Use `git worktree add ../testproject1-stage15 main` for clean isolation. Don't run alongside any other Stage 15 sub-work.

---

## §1 — DECISIONS ALREADY MADE

### D1 — State machine (5 states + 3 terminal outcomes)

```
not_started → drafting → reviewing → submitted → tracking → won | lost | withdrawn
```

- `not_started`: default. No work has begun.
- `drafting`: bid documents being generated. Stage 15 skill `bid-workflow/draft-bid` runs.
- `reviewing`: drafted artifacts available; operator must approve before submission.
- `submitted`: portal submission completed.
- `tracking`: post-submission state monitoring (e.g., portal status checks).
- `won` | `lost` | `withdrawn`: terminal states.

State transitions are explicit (no auto-progression past `reviewing` without operator action).

### D2 — Operator review gate (NON-NEGOTIABLE)

Between `drafting` and `submitted`, the operator MUST approve the drafted artifacts. This is the safeguard against autonomous real-money submissions. The drafted bid is rendered to a reviewable artifact (PDF or markdown) + summary view in the UI. Operator clicks "Submit" or "Cancel + Redraft".

### D3 — Idempotency: bid_workflow_states.process_id is the natural key

One bid workflow per process at a time. If `bid_state` is non-terminal, no new workflow can start. Re-attempts after a `won`/`lost`/`withdrawn` are out of MVP scope (file as bead `BWF-RESUBMIT`).

### D4 — `bid_scope_json` shape (proposed; finalize during build)

```json
{
  "candidate_id": "<product_candidate_id>",
  "supplier_quote_id": "<chosen quote>",
  "manifest_basis_hash": "<hash at time of bid drafting>",
  "operator_intent_id": "<the approving intent>",
  "drafted_artifact_ids": ["<artifact hash>", ...],
  "submission_receipts": [{"portal": "...", "submitted_at": "...", "receipt_id": "..."}, ...]
}
```

The shape may evolve based on L6's templates. Settle on the final shape before writing the migration that makes the column NOT NULL.

### D5 — Skills required (depend on L6)

Names and shapes are placeholders pending L6's spec:

- `bid-workflow/draft-bid` — input: bid_scope_json + manifest content + supplier quote. Output: drafted bid artifacts (PDF or MD or portal-specific form data).
- `bid-workflow/submit-bid` — input: drafted artifacts + portal credentials + operator confirmation. Output: submission receipt.
- `bid-workflow/track-submission` — input: process_id + previous receipt. Output: current portal-side status (e.g., "received", "evaluated", "awarded").

If L6 ships different skill names, follow L6's convention.

### D6 — CLI subcommands

```
--draft-bid <process_id>                                    # transitions not_started → drafting
--list-bid-workflows [--state <state>]                      # status check
--approve-bid <process_id> --idempotency-key <key>          # transitions reviewing → submitted (after operator review)
--reject-bid <process_id> --reason <text>                   # transitions reviewing → drafting (redraft) or → withdrawn
--track-bid <process_id>                                    # poll source portal for current state
```

### D7 — UI: new "Bid Workflow" tab in OperatorDecisionPanel

After a decision is captured + approved, a new tab appears showing:

- Current bid state (state machine visualization)
- Drafted artifact preview (rendered MD/PDF)
- Approve / Reject buttons (only enabled in `reviewing` state)
- Submission receipts (post-submitted)
- Tracking history (post-tracking)

Reuses Phase 2's IPC patterns. New IPC commands: `start_bid_workflow`, `list_bid_workflows`, `approve_bid_submission`, `reject_bid_submission`, `track_bid_status`.

### D8 — Test environments (real portals are dangerous)

Stage 15 testing uses **simulated portals** by default. A `STAGE15_PORTAL_MODE` env var: `simulate` (default — write to a local fixture instead of real portal) | `staging` (real portal staging API if available) | `production` (NEVER default; requires `--i-understand-this-is-real` CLI flag and operator confirmation).

Live submissions to real portals during Stage 15 development are explicitly OUT OF SCOPE — that's Layer 6 production rollout, not Stage 15 build.

---

## §2 — IMPLEMENTATION ORDER

(Executed only after §0 confirms L4-L6 built.)

1. Verify §0 + create or locate an approved intent (Stage 14 capture).
2. **Schema**: confirm `bid_workflow_states` schema is current; add columns if D4 finalization changed the shape (would need a new migration `0010` or higher).
3. **Skills**: implement `bid-workflow/{draft-bid, submit-bid, track-submission}` per D5. Each: `SKILL.md` + `result.schema.json` + canary inputs + golden assertions.
4. **Daemon runner**: `run_bid_workflow_advance_loop(state_db, cli)` — pulls one workflow from `bid_workflow_states`, picks the next state per the state machine, invokes the appropriate skill or operator-gated action.
5. **CLI subcommands** per D6.
6. **IPC commands** per D7.
7. **React tab** in `OperatorDecisionPanel.tsx` (or sibling component).
8. **Tests**: state machine transitions (unit), skill round-trips (canary), simulate-mode E2E, real-portal-staging mode validation if available.
9. Drive on the approved intent end-to-end through `simulate` mode.
10. Quality gates.
11. Validation manifest.

---

## §3 — REQUIRED TESTS

| # | Test | Verifies |
|---|---|---|
| BWF.1 | `bid_workflow_state_machine_only_allows_legal_transitions` | not_started→drafting OK; drafting→submitted ILLEGAL (must go via reviewing) |
| BWF.2 | `bid_workflow_idempotency_one_active_per_process` | Second `--draft-bid` for the same process while non-terminal returns the existing workflow ID + an error |
| BWF.3 | `draft_bid_skill_round_trips_via_canary_inputs` | Skill canary fixtures produce expected drafted artifact shape |
| BWF.4 | `submit_bid_blocked_until_operator_approves` | Calling `--approve-bid` on a workflow in state ≠ `reviewing` returns an error |
| BWF.5 | `submit_bid_simulate_mode_writes_local_fixture_not_real_portal` | When `STAGE15_PORTAL_MODE=simulate`, the submission writes to `artifacts/bid-submissions/simulate/<process_id>/...` not a real portal |
| BWF.6 | `submit_bid_production_mode_requires_safety_flag` | Calling submit in `production` mode without `--i-understand-this-is-real` returns an error |
| BWF.7 | `track_submission_polls_portal_idempotently` | Multiple `--track-bid` invocations don't duplicate receipts in `bid_scope_json.submission_receipts[]` |
| BWF.8 | `bid_workflow_terminal_states_immutable` | Workflows in `won`/`lost`/`withdrawn` reject all transition attempts |
| BWF.9 | `bid_workflow_basis_hash_stale_warning` | If `manifest_basis_hash` changes between drafting and submission, the workflow surfaces a warning (operator can choose to redraft or proceed) |
| BWF.10 | `start_bid_workflow_requires_approved_intent` | Cannot start a workflow without a queued+approve intent for the process |

Plus integration tests (one per state transition) + a happy-path E2E in `simulate` mode walking not_started → drafting → reviewing → submitted → tracking → won.

---

## §4 — ACCEPTANCE CRITERIA — R.1 to R.15

| # | Criterion |
|---|---|
| R.1 | §0 pre-flight all GREEN (L4, L5, L6 built; bid_workflow_states schema present; approved intent exists) |
| R.2 | `bid-workflow/{draft-bid, submit-bid, track-submission}` skills implemented per D5 with full SKILL.md + canary + golden coverage |
| R.3 | `run_bid_workflow_advance_loop` daemon function implemented; state machine per D1 |
| R.4 | `--draft-bid`, `--list-bid-workflows`, `--approve-bid`, `--reject-bid`, `--track-bid` CLI subcommands work |
| R.5 | New IPC commands per D7 wired through DaemonBridge socket |
| R.6 | UI tab in OperatorDecisionPanel renders bid state + artifact preview + approve/reject + receipts + tracking |
| R.7 | Operator review gate (D2) enforced: cannot transition reviewing → submitted without explicit operator action |
| R.8 | `STAGE15_PORTAL_MODE=simulate` writes to local fixture; `production` requires `--i-understand-this-is-real` (D8) |
| R.9 | All BWF.1–BWF.10 unit tests pass; integration test for happy-path E2E (simulate mode) passes |
| R.10 | One real approved intent (DGU or whichever exists) walked end-to-end through simulate mode; bid_workflow_states row transitions through all states |
| R.11 | Stale basis_hash warning surfaces when manifest changes between drafting and submission (BWF.9) |
| R.12 | Stage 1-14 invariants intact: 7 manifests, 4 lce rows (or current count), 17 net-margin verdicts, prior demo intents preserved |
| R.13 | No real-money / real-portal submission was made during testing (verifiable via STAGE15_PORTAL_MODE=simulate logs) |
| R.14 | Quality gates: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings`; daemon binary tests pass; per-file integration loop unchanged + new BWF integration tests added; vitest + Tauri Rust tests + new bid-workflow UI tests pass |
| R.15 | Validation manifest documents the simulate-mode walkthrough with command excerpts + DB snapshots at each transition |

---

## §5 — VALIDATION QUERIES

```bash
TARGET_PID="<process_id of the approved intent — likely DGU>"

# Pre-launch: verify approved intent exists
sqlite3 state/state.db "SELECT idempotency_key, desired_disposition, status FROM pending_operator_intents WHERE process_id='$TARGET_PID' AND status='queued' AND desired_disposition='approve' ORDER BY queued_at DESC LIMIT 1"

# After --draft-bid
sqlite3 state/state.db "SELECT process_id, bid_state, bid_scope_json, started_at FROM bid_workflow_states WHERE process_id='$TARGET_PID'"
ls artifacts/bid-drafts/$TARGET_PID/   # drafted artifacts

# After --approve-bid
sqlite3 state/state.db "SELECT bid_state, json_extract(bid_scope_json, '\$.submission_receipts') FROM bid_workflow_states WHERE process_id='$TARGET_PID'"

# After --track-bid
sqlite3 state/state.db "SELECT bid_state, json_extract(bid_scope_json, '\$.submission_receipts'), completed_at FROM bid_workflow_states WHERE process_id='$TARGET_PID'"

# Invariants (R.12)
sqlite3 -header state/state.db "
SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY verdict;
SELECT stoplight, COUNT(*) FROM manifests GROUP BY stoplight;
SELECT idempotency_key, status FROM pending_operator_intents WHERE idempotency_key LIKE 'dec-2026-05-01-dgu-%';
"
```

---

## §6 — FAILURE-MODE PLAYBOOK

| Symptom | Cause | Fix |
|---|---|---|
| L6 templates skill not found | L6 didn't ship the bid-template skill yet | Stop. The §0 prerequisite isn't met. Defer until L6 supplies the template skill. |
| Drafted bid artifact shape doesn't match L6 template output | L6 changed its output shape | Reconcile with L6's actual schema. If L6 is the authority, conform the bid-workflow skills |
| `submit_bid` accidentally hits a real portal in simulate mode | Mode wiring bug or default value | URGENT investigation: verify `STAGE15_PORTAL_MODE` defaults to `simulate`. Add a CI test asserting the default. Roll back any real submission |
| Operator review gate skipped | State machine bug — direct drafting → submitted transition allowed | BWF.1 unit test should catch this; if it didn't, add a new test for that exact transition |
| Stale basis hash warning doesn't fire | manifest_basis_hash comparison logic missing or wrong | Verify the workflow stores `manifest_basis_hash` at drafting time + compares at submission time |
| Workflow stuck in `reviewing` indefinitely | Operator hasn't acted; or the UI's approve button is broken | Surface operator-pending state in the manifest inbox / dashboard. Consider a TTL after which the workflow auto-warns |

---

## §7 — CONSTRAINTS

1. **NO real-money or real-portal submissions during build/test.** Default mode must be `simulate`.
2. **Operator review gate is non-negotiable** (D2). Drafting → submission MUST go via `reviewing` with explicit operator action.
3. **Schema changes (if any) require a new migration** (`0010`+). DO NOT modify existing migrations.
4. **L6 templates are authoritative** for bid content shape. Conform; don't invent.
5. **Stage 14 idempotency contract intact** — bid workflow's input is the existing operator intent, not a parallel structure.
6. **Worktree** for isolation: `git worktree add ../testproject1-stage15 main`.
7. **No silent test exclusion** — all gates per R.14 mandatory.

---

## §8 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/stage-15-bid-workflow-validation-manifest.md`. Include:

- Which pre-flight items in §0 were green at start
- Real approved intent walked end-to-end (process_id, intent_id, idempotency_key)
- Per-transition snapshot: `bid_workflow_states` row at each state + artifacts produced
- Operator review evidence: screenshot of UI approve dialog or CLI confirmation prompt
- All R.* statuses
- Quality gate transcript
- Out-of-scope items (real-portal staging, BWF-RESUBMIT for terminal-state retries, etc.)

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon            # baseline + 10 BWF.* tests
cargo test -p procintel-desktop              # baseline + new bid-workflow tab tests
cd apps/desktop && bunx vitest run            # baseline + new tests
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || break; done   # all PASS, including new bid-workflow integration test
```

---

## §9 — REMINDERS

- ULTRATHINK and be meticulous.
- **DO NOT START THIS HANDOFF UNTIL §0 PRE-FLIGHT CONFIRMS L4-L6 ARE BUILT.** This is the most important reminder.
- Stage 15 is the riskiest stage in the pipeline because it's the only one that can affect the outside world (real portal submissions). The operator review gate (D2) and `STAGE15_PORTAL_MODE=simulate` default (D8) are the safeguards that keep that risk bounded.
- The `bid_workflow_states` schema already exists; the work is the runner + skills + UI + tests.
- L6's bid templates are the authoritative source for bid content shape — don't invent template formats here.
- One real approved intent walked end-to-end in simulate mode is the demonstration deliverable. Save the artifacts; they prove the pipeline works without committing the operator to anything real.
- When this handoff ships, the L1-L3 product is feature-complete: ingestion → intelligence → operator decision → bid execution. Worth marking the moment.
