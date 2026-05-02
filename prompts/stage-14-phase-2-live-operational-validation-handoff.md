# Stage 14 Phase 2 — Live Operational Validation + Cross-Session Staleness E2E

ULTRATHINK and be meticulous.

## MISSION

The Stage 14 Phase 2 handoff (`be38085`) shipped CODE-COMPLETE: 7 new IPC commands, ManifestInbox + OperatorDecisionPanel React pages, 12 vitest cases passing, 4 RT.* Tauri Rust tests passing. But three R-criteria were explicitly deferred because they require a live desktop session: **R.2** (app launches + Inbox renders 7 cards from live state.db), **R.8** (Re-run progress modal + manifest hash refresh), **R.10** (stale banner appears within 2s of external basis-hash change). This handoff drives those live demos AND adds a cross-session staleness E2E that was implicitly out of scope.

The ONLY way this work happens is by humans (or a desktop-capable agent) launching `bun tauri dev` and walking the operator workflow. Code changes, if any, are bug fixes surfaced by the live demo — NOT new feature work.

You will **drive → screenshot → validate → fix bugs surfaced → repeat** until every numbered acceptance criterion (R.1–R.12) is GREEN.

```
┌────────────────────────────────────────────────────────────┐
│                          THE LOOP                            │
│   1. Pre-flight (§0)                                         │
│   2. Launch bun tauri dev (§2)                               │
│   3. Walk the §3 9-step demo + §4 staleness E2E             │
│   4. Capture screenshots at each step                       │
│   5. For each broken step: file specific bug + fix in code  │
│   6. Re-launch + replay; iterate until all steps green      │
│   7. Quality gates: fmt/check/clippy + 169 daemon tests +    │
│      35 vitest still pass after any fix                     │
│   8. All green → write the validation manifest               │
└────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## §0 — PRE-FLIGHT

| Claim | Verify |
|---|---|
| Phase 2 code committed at `be38085`; CDP resilience at `4ba771d`. main is at HEAD `4ba771d` | `git log --oneline -3` |
| 7 manifests + 4 lce rows + 17 net-margin assessments + 2 demo intents (1 superseded, 1 withdrawn) + 1 active operator override (DGU qty=250) | `sqlite3 state/state.db "SELECT COUNT(*) FROM manifests; SELECT COUNT(*) FROM landed_cost_estimates; SELECT idempotency_key,status FROM pending_operator_intents WHERE idempotency_key LIKE 'dec-2026-05-01-%'"` |
| 7 Tauri IPC commands registered in `apps/desktop/src-tauri/src/main.rs:66-92` invoke_handler block: `list_manifests`, `get_decision_history`, `get_active_overrides`, `capture_clarification`, `withdraw_clarification`, `withdraw_decision`, `trigger_pipeline_rerun`. Plus `capture_decision` (reused, with `decision_note` extension). | `sed -n '66,92p' apps/desktop/src-tauri/src/main.rs` |
| `apps/desktop/package.json` declares `"test": "bunx vitest run"` and `"tauri:dev"` script (or `bun tauri dev` works directly via the @tauri-apps/cli dependency) | `cat apps/desktop/package.json \| grep -E '"(test\|tauri)"'` |
| State-DB UI prerequisites: 7 manifest rows have non-empty `clarification_prompts_json` for at least the `clarify` stoplight rows (DGU + Aggregates + 5 others) | `sqlite3 state/state.db "SELECT process_id, stoplight, json_array_length(clarification_prompts_json) AS prompts FROM manifests"` |
| Bun + Tauri + Rust toolchain installed locally; Mac/Linux desktop GUI available | `bun --version; cargo --version; ls /Applications/ \| head -1` |
| `agent-mail` healthy; commits need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Parallel-safety**: this handoff requires exclusive desktop GUI access (one operator session at a time). Do NOT run alongside `stage-11-cdp-live-demos-handoff.md` or `golden-journey-dgu-handoff.md` if those drive the same daemon. Safe to run alongside the read-only quality-gate sweep, the docs hygiene work, or the test-flake cleanup (different files).

---

## §1 — DECISIONS ALREADY MADE

### D1 — Screenshots are MANDATORY

The Phase 2 manifest's deferred R-criteria specifically called for screenshot capture. Each of the 9 demo steps + the staleness E2E gets at least 1 screenshot, committed to `docs/pipeline/stage-14-phase-2-screenshots/`. Naming: `step-01-inbox.png`, `step-02-detail-DGU.png`, ..., `step-10-stale-banner.png`. PNG (or WebP if file size matters); 1280×800 or larger.

### D2 — Fix bugs surfaced; do NOT defer

If the live demo surfaces a bug (e.g., a Tauri command 500's, a React component crashes, the stale banner doesn't render), fix it in this handoff — that's exactly what the live demo is for. The fix MUST come with: a regression test in vitest or RT.* form, a minimal description in the validation manifest's "Bugs surfaced + fixed" section.

If a bug requires a daemon-side change (e.g., an IPC handler returns wrong shape), the fix is in scope provided it's small (~50 lines). Larger work (e.g., a missing IPC handler) → file as follow-up bead.

### D3 — Cross-session staleness E2E methodology

The cross-session staleness flow:

1. UI: open DGU process → capture decision (any disposition + reasoning).
2. Verify: `sqlite3 state/state.db "SELECT status FROM pending_operator_intents WHERE idempotency_key='<the new key>'"` → `queued`.
3. **External terminal** (NOT the UI): `cargo run -p daemon -- --ingest manifest`. This re-runs Stage 13 manifest generation; if the manifest content changed (e.g., because the operator's clarification flowed into Stage 11→12→13), the basis_hash bumps.
4. Wait 2s. Reload UI (or watch for live update if subscriptions are wired).
5. Verify in UI: stale banner appears on the DGU process card in Inbox AND in the Process Detail View.
6. Verify in DB: `sqlite3 state/state.db "SELECT status FROM pending_operator_intents WHERE idempotency_key='<the new key>'"` → `stale`.

If step 3's manifest re-run produces NO basis_hash change (because nothing actually shifted), the test won't trigger. Either: (a) capture a clarification first that DOES shift downstream state, then re-run manifest; (b) directly bump `manifests.manifest_basis_hash` for DGU via SQL to simulate; document which approach.

### D4 — `bun tauri dev` may need pre-build

The first launch may take 1-2 minutes (cargo build of `procintel-desktop` + Vite dev server). Subsequent launches with hot-reload are fast. Allow time before declaring "doesn't launch."

---

## §2 — IMPLEMENTATION ORDER

1. **Pre-flight** (§0) — verify state-DB has the expected demo data + Tauri toolchain works.
2. **Launch app**: `cd apps/desktop && bun install && bunx tauri dev`. Wait for window to open.
3. **Step 1** of §3 demo (Inbox loads). Screenshot. Validate against R.1.
4. **Step 2** (open DGU). Screenshot. Validate.
5. ... continue through Step 9.
6. **Cross-session staleness E2E** (§4). Screenshots before + after. Validate against R.10.
7. **Bug triage**: any broken step → diagnose → fix in code → re-launch → re-walk that step.
8. **Run quality gates** after any fix: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon && cd apps/desktop && bunx vitest run`.
9. Write validation manifest with screenshots embedded.

---

## §3 — DEMO SEQUENCE (9 steps from Phase 2 handoff §3)

Drive these IN ORDER. Capture a screenshot at each step. Keep the dev tools console open and grep for any errors.

1. Launch app. Manifest Inbox loads with 7 process cards. **Validation**: each card shows process title, buyer, contract value, stoplight badge with color, suggested action, prompt count, last-decided status (DGU should show `withdrawn` per Phase 1 demo state).
2. Click DGU card. Process Detail View opens. Markdown body renders (sections A–F via `ManifestViewer` shared component). Quick-Decide panel visible. Decision history shows 2 prior intents (`dec-2026-05-01-dgu-001` superseded, `dec-2026-05-01-dgu-002` withdrawn).
3. Open Clarification form. **Validation**: 2 prompts visible — `quantity_basis_unaligned` (with current value 250 m²) and `awaiting_rfq_quote`. Each prompt has the right input control per `prompt_type`.
4. Update `quantity_basis_unaligned` to **150 m²**. Click submit. UI shows success. Verify DB: `sqlite3 state/state.db "SELECT json_extract(payload_json,'$.qty') FROM operator_input_overrides WHERE product_candidate_id LIKE '%glass-dgu%' AND withdrawn_at IS NULL ORDER BY provided_at DESC LIMIT 1"` → `150.0` (the new override supersedes the prior 250).
5. Click "Re-run pipeline". Progress modal appears. Cycles through `landed-cost` → `net-gate` → `manifest`. Closes when done. **Validation**: `manifest_basis_hash` for DGU has changed; the page re-renders.
6. In Quick-Decide panel: select `approve`, type reasoning *"DGU at 150 m² is the most realistic spec; freight covered by Stage 11 estimate; pursue."*, click Capture. **Validation**: UI shows "Decision captured" + decision ID. Decision history refreshes with new intent at `status='queued'`. DB: `sqlite3 state/state.db "SELECT status, decision_note FROM pending_operator_intents WHERE process_id='proc-pc-7046e8ec-...' AND status='queued' ORDER BY queued_at DESC LIMIT 1"` → returns `queued` + the reasoning text.
7. Open Aggregates process (`proc-pc-d23508ec-...`). Capture a clarification with `reason_code=quantity_basis_undetermined` and `qty=500 kg`. Re-run pipeline. Decide `watch`.
8. **External terminal**: `cargo run -p daemon -- --ingest manifest`. Wait. Reload Inbox. **Validation**: see §4 staleness E2E.
9. Withdraw a decision via UI (the just-captured Aggregates `watch` is a good target). **Validation**: decision history shows withdrawal; DB: that intent's `status='withdrawn'`.

---

## §4 — CROSS-SESSION STALENESS E2E

After step 6 (DGU `approve` captured at qty=150):

| # | Action | Expected |
|---|---|---|
| S.1 | UI: confirm DGU's latest intent is at `status='queued'` (decision history) | Visible queued pill |
| S.2 | DB: `sqlite3 state/state.db "SELECT status FROM pending_operator_intents WHERE idempotency_key='<step-6-key>'"` | `queued` |
| S.3 | External terminal: capture a NEW clarification on DGU via CLI (NOT through the UI), e.g., `cargo run -p daemon -- --capture-clarification <process-id> --candidate-id pc-7046e8ec-...-glass-dgu-bs-en-1279 --reason-code quantity_basis_unaligned --payload '{"qty":300,"unit":"square meter"}'`. This shifts the override from 150 → 300 (out-of-band). | Override row updated |
| S.4 | External terminal: `cargo run -p daemon -- --ingest landed-cost && cargo run -p daemon -- --ingest net-gate && cargo run -p daemon -- --ingest manifest`. The Stage 11→12→13 chain re-runs; manifest_basis_hash bumps. | New manifest row, new basis hash |
| S.5 | Wait 2s. Reload UI Inbox (manual refresh button or browser reload). | DGU card shows yellow stale indicator |
| S.6 | Open DGU detail. | Stale banner visible above Quick-Decide panel: *"This decision was made against an earlier manifest version. Re-confirm or withdraw."* |
| S.7 | DB: `sqlite3 state/state.db "SELECT status FROM pending_operator_intents WHERE idempotency_key='<step-6-key>'"` | `stale` |
| S.8 | Click "Withdraw decision" via UI. Confirm withdrawal. | Status flips to `withdrawn`; banner disappears; decision history shows withdrawn entry |

Capture screenshots at S.5 (Inbox stale indicator), S.6 (banner in Detail), S.8 (post-withdrawal).

If steps S.5/S.6 don't show the banner BUT S.7 confirms `status='stale'` in DB, that's a UI bug (data is right; render is wrong) — fix the React component.

If S.7 shows `queued` instead of `stale`, that's a daemon-side bug in the `stale_detection_*` logic — investigate `crates/daemon/src/main.rs` around the `'stale'` references at line 18951 / 19369.

---

## §5 — REQUIRED TESTS

This handoff adds NO new tests UNLESS a bug is fixed. If a bug is fixed:

- UI bug → vitest test in `apps/desktop/src/pages/<page>.test.tsx`
- Tauri Rust bug → RT.* test in `apps/desktop/src-tauri/tests/query_service.rs` or `daemon_bridge.rs`
- Daemon bug → unit test in `crates/daemon/src/main.rs` `mod tests` OR integration test in `crates/daemon/tests/`

Naming: `bug_<issue>_regression_<description>` to make the regression intent explicit.

---

## §6 — ACCEPTANCE CRITERIA — R.1 to R.12

| # | Criterion |
|---|---|
| R.1 | `bunx tauri dev` launches and the app window opens within 3 minutes (first launch); subsequent launches within 30s |
| R.2 | Manifest Inbox renders all 7 process cards from live state.db. Stoplight colors per Phase 2 R.3 (clarify=yellow, pursue=green, pass=red, review=gray) |
| R.3 | Process Detail View opens for DGU with markdown sections A–F + Quick-Decide panel + Clarification form + decision history (showing 2 prior intents) |
| R.4 | Clarification form submits successfully → DB has new override row (qty=150 active, qty=250 superseded) |
| R.5 | "Re-run pipeline" button cycles progress modal through landed-cost → net-gate → manifest → closes; manifest_basis_hash bumps |
| R.6 | Decision capture writes a `queued` row to `pending_operator_intents` with the entered reasoning text in `decision_note` |
| R.7 | Aggregates process can be opened, clarified (qty=500 kg), re-run, decided `watch`, all visible in DB |
| R.8 | Stale banner appears within 2 seconds of an out-of-band basis_hash change (cross-session staleness §4 S.5/S.6) |
| R.9 | Withdraw via UI flips intent's status to `withdrawn` (§4 S.8) |
| R.10 | All 9 demo screenshots + ≥3 staleness screenshots committed to `docs/pipeline/stage-14-phase-2-screenshots/` |
| R.11 | Stage 11/12/13/14 invariants intact: 17 net-margin verdicts still review; 7 manifests still present (the 2 modified by this demo are the same rows, with newer manifest_basis_hash); pre-existing demo intents (`-001`, `-002`) untouched |
| R.12 | Quality gates: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings`; 169 daemon binary tests + 35 vitest + 21 query_service + 11 daemon_bridge all pass; any new regression tests pass; per-file integration loop unchanged from baseline |

---

## §7 — VALIDATION QUERIES + COMMANDS

```bash
# Inbox baseline
sqlite3 -header -column state/state.db "
SELECT process_id, stoplight, json_array_length(clarification_prompts_json) AS prompts, manifest_basis_hash
FROM manifests ORDER BY process_id;
"

# After step 4 (clarification submitted)
sqlite3 state/state.db "
SELECT json_extract(payload_json, '\$.qty') AS qty, withdrawn_at, superseded_by_override_id
FROM operator_input_overrides
WHERE product_candidate_id LIKE '%glass-dgu%' ORDER BY provided_at DESC LIMIT 5;
"

# After step 6 (decision captured)
sqlite3 state/state.db "
SELECT idempotency_key, status, desired_disposition, decision_note, queued_at
FROM pending_operator_intents WHERE process_id LIKE 'proc-pc-7046e8ec-%' ORDER BY queued_at DESC LIMIT 5;
"

# Staleness check (§4 S.7)
sqlite3 state/state.db "
SELECT idempotency_key, status, reviewed_basis_hash,
       (SELECT manifest_basis_hash FROM manifests m WHERE m.process_id = p.process_id) AS current_basis_hash
FROM pending_operator_intents p WHERE process_id LIKE 'proc-pc-7046e8ec-%' ORDER BY queued_at DESC LIMIT 3;
"

# R.11 invariants
sqlite3 -header state/state.db "
SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY verdict;
SELECT idempotency_key, status FROM pending_operator_intents WHERE idempotency_key IN ('dec-2026-05-01-dgu-001','dec-2026-05-01-dgu-002');
"
```

---

## §8 — FAILURE-MODE PLAYBOOK

| Symptom | Cause | Fix |
|---|---|---|
| `bunx tauri dev` fails with "command not found" | tauri CLI not installed | `cd apps/desktop && bun install` first; if still missing, `bun add -d @tauri-apps/cli` |
| App window opens but Inbox shows "0 manifests" | `list_manifests` IPC returns empty OR daemon isn't running OR state.db path mismatch | Check Tauri logs (Cmd+Opt+I → Console). Check daemon status. Confirm `EXPECTED_SCHEMA_EPOCH = 2` matches the daemon's `DEFAULT_SCHEMA_EPOCH`. The daemon should be auto-spawned by the desktop bootstrap. |
| Stoplight colors wrong | Tone map regression in `ManifestInbox.tsx` | The colors are asserted by `ManifestInbox.test.tsx › FT.2` — re-run that test. If it passes locally, the issue is CSS load order; check that `styles.css` is imported. |
| Stale banner doesn't appear (R.8 fails) BUT DB shows `status='stale'` | UI not subscribing to changes, OR not refetching after `trigger_pipeline_rerun` | Either (a) add a refetch on focus, (b) subscribe to a Tauri event the daemon emits on manifest writes, OR (c) document as known limitation requiring manual reload (downgrade R.8 to "manual reload required") |
| Stale banner doesn't appear AND DB shows `status='queued'` | Daemon `stale_detection_*` logic didn't run or didn't detect a basis_hash diff | Check `crates/daemon/src/main.rs` around lines 18951 / 19369. Verify `reviewed_basis_hash` differs from current `manifest_basis_hash` for that process. |
| Re-run pipeline modal hangs | `trigger_pipeline_rerun` Tauri command doesn't return | Per Phase 2 architecture note, this command aliases `trigger_enrichment`. Check daemon logs. May need to add a timeout to the command + show error in UI. |
| Decision capture saves but `decision_note` is NULL in DB | `CaptureDecisionParams.decision_note` field added in Phase 2 but the capture path doesn't propagate it | Verify `bridge_capture_decision_params` forwarder in `apps/desktop/src-tauri/src/main.rs` includes `decision_note`. Verify `decisions.rs` IPC handler INSERTs it. Both should be in `be38085`. |
| `bunx vitest run` from repo root sweeps Playwright .spec.ts files | Wrong working directory | `cd apps/desktop` first; the vitest config has `root: desktopRoot` |

---

## §9 — CONSTRAINTS

1. **No new IPC commands.** Phase 2 introduced 7; this handoff doesn't add to that surface.
2. **No new React pages.** Use the existing ManifestInbox + OperatorDecisionPanel + ContractDetail.
3. **Daemon-side fixes allowed only for clear UI-surfaced bugs.** Bigger architectural work → follow-up bead.
4. **Screenshots are MANDATORY** (R.10). Without them, R-criteria for Phase 2 R.2/R.8/R.10 cannot be marked GREEN.
5. **Stage 11/12/13/14 invariants intact** (R.11). The demo modifies DGU's intent + override state, but other rows + the 2 pre-existing demo intents (`-001`, `-002`) must not regress.
6. **No silent test exclusion.** Quality gates per §6 R.12 are mandatory.
7. **State.db pre-condition**: this handoff modifies state.db (new override row, new intent row). If you want to re-run from scratch, snapshot state.db first, run, then restore.

---

## §10 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/stage-14-phase-2-live-operational-validation-manifest.md`. Template:

```markdown
# Stage 14 Phase 2 — Live Operational Validation Manifest

> Generated: <ISO timestamp>
> Status: GREEN / RED

## Summary
<1 paragraph: were all 9 demo steps + staleness E2E walked? Were any bugs surfaced + fixed? Final state of state.db>

## Step-by-step screenshots
| Step | Screenshot | Result |
|---|---|---|
| 1 — Inbox loads | docs/pipeline/stage-14-phase-2-screenshots/step-01-inbox.png | PASS / FAIL |
| 2 — DGU detail | docs/pipeline/stage-14-phase-2-screenshots/step-02-detail-DGU.png | PASS / FAIL |
| ... | | |
| Staleness S.5 | step-10-stale-inbox.png | PASS / FAIL |
| Staleness S.6 | step-11-stale-detail.png | PASS / FAIL |

## Bugs surfaced + fixed (if any)
| File | Bug | Fix | Regression test |

## R.1–R.12 status
| # | Criterion | Status | Evidence |

## Quality Gates
| Gate | Result |

## Out-of-scope items (filed as beads)
```

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon                # 169 / 169 (or +N if regression tests added)
cargo test -p procintel-desktop --test query_service   # 21 / 21
cargo test -p procintel-desktop --test daemon_bridge   # 11 / 11
cd apps/desktop && bunx vitest run                # 35 / 35
```

---

## §11 — REMINDERS

- ULTRATHINK and be meticulous.
- The DEFERRED R-criteria from Phase 2 (R.2, R.8, R.10) are the primary deliverable here. Without screenshots + a documented walk-through, this handoff is incomplete.
- The cross-session staleness E2E (§4) is the highest-value scenario — it exercises the daemon's stale_detection_* logic, the UI's data refresh, AND the visual stale banner — all in one.
- Bugs surfaced by the live demo are EXPECTED. Fix them; don't hide them.
- Stage 14 demo state IS modified by this handoff (new intents + overrides). That's fine — it's adding to history, not overwriting.
- When in doubt about whether a Tauri command did what you expected, query the DB. The DB is ground truth; the UI is presentation.
