# Full Pipeline E2E Desktop UI Click-Through Continuation Handoff

ULTRATHINK and be meticulous.

## Mission

Complete the literal desktop UI click-through that the full L1-L3 E2E run could not finish because the validating environment lacked the required machine-control permissions.

The backend, daemon bridge, E2E state root, manifest generation, CLI decision capture, and Quick-Decide-shaped native bridge command have already been validated. What remains is the real Tauri window interaction path:

1. Manifest Inbox loads and shows the Woodhouses spotlight manifest.
2. Click the spotlight card.
3. Process Detail renders the manifest through `ManifestViewer`.
4. Open and submit a clarification from the UI.
5. Click `Re-run pipeline` and observe the progress modal.
6. Capture a decision through the Quick-Decide panel.
7. Capture screenshots for the validation manifest.
8. If practical, validate cross-session staleness from the UI.

This handoff is for an agent/operator with better control of the desktop window than the previous session had.

## Current Context

Validation manifest:

- `docs/pipeline/full-pipeline-e2e-validation-manifest.md`

Screenshots already captured:

- `docs/pipeline/full-pipeline-e2e-screenshots/step-00-default-path-schema-mismatch.png`
- `docs/pipeline/full-pipeline-e2e-screenshots/step-01-e2e-state-inbox-blocked-by-privacy-prompt.png`
- `docs/pipeline/full-pipeline-e2e-screenshots/step-02-final-privacy-prompt-state.png`
- `docs/pipeline/full-pipeline-e2e-screenshots/step-03-full-control-desktop-ready.png`

E2E runtime roots:

- State root: `/Users/alinaaitova/Desktop/Testproject1/state-e2e-20260504T113157Z`
- Runtime dir: `/Users/alinaaitova/Desktop/Testproject1/runtime-e2e-20260504T113157Z`
- Artifact root: `/Users/alinaaitova/Desktop/Testproject1/artifacts-e2e-20260504T113157Z`

Spotlight process:

- Process ID: `proc-pc-956d1aaa-8841-f111-813c-005056b64545`
- Notice title: `4) Woodhouses Voluntary Controlled Primary School - Boilers and BMS Panel Replacement (Four of five school summer shutdown projects)`
- Buyer: Oldham Council
- Manifest artifact: `/Users/alinaaitova/Desktop/Testproject1/artifacts-e2e-20260504T113157Z/manifests/proc-pc-956d1aaa-8841-f111-813c-005056b64545/manifest.md`

## What Was Already Proven

The previous E2E run got the desktop app to `full control` mode using the E2E state root and patched daemon path. A screenshot exists at:

- `docs/pipeline/full-pipeline-e2e-screenshots/step-03-full-control-desktop-ready.png`

Because automated click access was blocked, the E2E validation exercised the same native daemon bridge path behind the UI controls directly. It successfully created:

- CLI decision: `e2e-cli-20260504T145047Z`
- Bridge clarification: `clarification-e2e-ui-bridge-20260504T1557Z`
- Bridge decision: `decision-watch-e2e-ui-bridge-20260504T1616Z`

Those rows prove the IPC command path can write the expected DB state. They do not prove the literal rendered UI click path, which is why this handoff exists.

## Phase 1: Required UI Research

Do not begin the click-through until you understand the UI and Tauri command path below.

### 1A. React Surfaces

Read:

- `apps/desktop/src/App.tsx` - screen routing, process-detail loading, and mutation refresh flow.
- `apps/desktop/src/pages/ManifestInbox.tsx` - inbox card rendering and `list_manifests` invocation.
- `apps/desktop/src/pages/ContractDetail.tsx` - Process Detail layout, `ManifestViewer`, and `OperatorDecisionPanel` placement.
- `apps/desktop/src/pages/OperatorDecisionPanel.tsx` - clarification form, Quick-Decide, withdrawal controls, and pipeline rerun modal.
- `apps/desktop/src/lib/transport.ts` - command transport and dev bridge behavior.

Key frontend types:

```ts
export type ManifestInboxRow = {
  processId: string;
  canonicalProcessId: string;
  title: string | null;
  buyerName: string | null;
  estimatedValuePence: number | null;
  stoplight: string | null;
  suggestedAction: string | null;
  clarificationPromptCount: number;
  manifestBasisHash: string | null;
  generatedAt: string;
  decision: DecisionStatusVm;
};

export type OperatorDecisionPanelProps = {
  processId: string;
  manifestBasisHash: string | null;
  manifestHash: string | null;
  manifestStageRunId: string | null;
  clarificationPromptsJson: string | null;
  latestDecision: {
    disposition: 'approve' | 'reject' | 'watch' | null;
    status?: string | null;
    isStale: boolean;
    decidedAt: string | null;
  };
  onChanged: () => void;
};
```

You must be able to answer:

1. Which UI element invokes `list_manifests`?
2. Which UI element invokes `get_process_detail`?
3. Which UI element invokes `capture_clarification`?
4. Which UI element invokes `trigger_pipeline_rerun`?
5. Which UI element invokes `capture_decision`?

### 1B. Tauri/Rust Bridge

Read:

- `apps/desktop/src-tauri/src/main.rs` - command handlers and `trigger_pipeline_rerun_impl`.
- `apps/desktop/src-tauri/src/query_service.rs` - `ProcessDetailVm`, `ManifestInboxRow`, manifest loading, and decision status calculation.
- `apps/desktop/src-tauri/src/daemon_bridge.rs` - bridge command names and daemon process invocation.

Key command signatures:

```rust
fn list_manifests<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> std::result::Result<Value, String>

fn capture_decision<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: CaptureDecisionParams,
) -> std::result::Result<Value, String>

fn capture_clarification<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: CaptureClarificationFrontendParams,
) -> std::result::Result<Value, String>

fn trigger_pipeline_rerun<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: TriggerPipelineRerunParams,
) -> std::result::Result<Value, String>
```

You must be able to answer:

1. How does `DesktopPaths::from_handle` decide which state DB to open?
2. What env vars force the desktop app onto the E2E state root?
3. What daemon binary does `PROCINTEL_DAEMON_PROGRAM` point at?
4. How do UI mutations become rows in `pending_operator_intents` or `operator_input_overrides`?

### 1C. Screenshot Safety

Do not overwrite existing screenshot files. Before saving a screenshot path, run:

```bash
test -e docs/pipeline/full-pipeline-e2e-screenshots/step-04-inbox-clickthrough.png && echo exists
```

If the target screenshot already exists, use a timestamped suffix, for example:

```text
docs/pipeline/full-pipeline-e2e-screenshots/step-04-inbox-clickthrough-20260504T1725Z.png
```

## Non-Negotiable Constraints

- Do not modify live `state/state.db`.
- Do not delete files.
- Do not run destructive git commands.
- Work on `main`.
- Do not perform Stage 15 or any real-money action.
- The only goal is desktop UI validation against the existing E2E state.

## Preflight

Run:

```bash
cd /Users/alinaaitova/Desktop/Testproject1
git status --short --branch
sqlite3 state-e2e-20260504T113157Z/state.db "PRAGMA integrity_check"
sqlite3 -header -column state-e2e-20260504T113157Z/state.db "
SELECT process_id, stoplight, suggested_action, manifest_basis_hash, artifact_hash
FROM manifests
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545';

SELECT idempotency_key, status, desired_disposition, decision_note, queued_at
FROM pending_operator_intents
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545'
ORDER BY queued_at;

SELECT product_candidate_id, reason_code, payload_json, withdrawn_at
FROM operator_input_overrides
WHERE product_candidate_id IN (
  SELECT id FROM product_candidates
  WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545'
)
ORDER BY provided_at;
"
```

Expected current decision rows include:

- `e2e-cli-20260504T145047Z`
- `decision-watch-e2e-ui-bridge-20260504T1616Z`

Expected current clarification row includes:

- `clarification-e2e-ui-bridge-20260504T1557Z`

The UI click-through should create new rows with new idempotency keys so the result is distinguishable from the bridge validation rows.

## Launch Command

Use the E2E roots explicitly. Do not let the app fall back to its app-support DB or live `state/state.db`.

```bash
cd /Users/alinaaitova/Desktop/Testproject1/apps/desktop

PROCINTEL_DAEMON_PROGRAM=/tmp/testproject1-target-e2e/debug/daemon \
PROCINTEL_DESKTOP_STATE_ROOT=/Users/alinaaitova/Desktop/Testproject1/state-e2e-20260504T113157Z \
PROCINTEL_DESKTOP_RUNTIME_DIR=/Users/alinaaitova/Desktop/Testproject1/runtime-e2e-20260504T113157Z \
PROCINTEL_DESKTOP_ARTIFACT_ROOT=/Users/alinaaitova/Desktop/Testproject1/artifacts-e2e-20260504T113157Z \
CARGO_TARGET_DIR=/tmp/testproject1-tauri-e2e-target \
CARGO_INCREMENTAL=0 \
bunx tauri dev
```

If `/tmp/testproject1-target-e2e/debug/daemon` is missing, rebuild the daemon without touching live state:

```bash
cd /Users/alinaaitova/Desktop/Testproject1
CARGO_TARGET_DIR=/tmp/testproject1-target-e2e cargo build -p daemon --bin daemon
```

Then re-run the Tauri command above.

## UI Flow

### Step 1: Manifest Inbox

Expected:

- App reaches full-control mode.
- Default route is Manifest Inbox or navigates to `/inbox`.
- Woodhouses spotlight card is visible.
- Card shows stoplight `clarify` or equivalent suggested action state.

Capture screenshot:

- `docs/pipeline/full-pipeline-e2e-screenshots/step-04-inbox-clickthrough.png`

If this file already exists, use a timestamped suffix. Do not overwrite existing screenshot evidence.

### Step 2: Process Detail

Click the Woodhouses spotlight card.

Expected:

- Process Detail View opens.
- Manifest markdown renders through `ManifestViewer`.
- Candidate evidence, quote evidence, landed cost, and clarification prompts are visible.

Capture screenshot:

- `docs/pipeline/full-pipeline-e2e-screenshots/step-05-detail-manifest-clickthrough.png`

If this file already exists, use a timestamped suffix. Do not overwrite existing screenshot evidence.

### Step 3: Clarification Form

Open the clarification form.

Submit a non-load-bearing test clarification. Use a note that clearly marks it as UI validation. Do not send anything externally.

Suggested values:

- Reason code: `ui_clickthrough_validation`
- Payload/note: `E2E desktop UI click-through validation: confirming the clarification form writes through the Tauri command path.`

If the UI only exposes predefined clarification reasons, choose the safest non-commercial option and write the note above in the reasoning/details field.

Capture screenshot before or after submission:

- `docs/pipeline/full-pipeline-e2e-screenshots/step-06-clarification-form-clickthrough.png`

If this file already exists, use a timestamped suffix. Do not overwrite existing screenshot evidence.

Validate:

```bash
sqlite3 -header -column /Users/alinaaitova/Desktop/Testproject1/state-e2e-20260504T113157Z/state.db "
SELECT idempotency_key, reason_code, payload_json, withdrawn_at, provided_at
FROM operator_input_overrides
WHERE product_candidate_id IN (
  SELECT id FROM product_candidates
  WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545'
)
ORDER BY provided_at DESC
LIMIT 5;
"
```

### Step 4: Re-run Pipeline

Click `Re-run pipeline`.

Expected:

- Progress modal opens.
- Pipeline steps cycle through landed cost, net gate, manifest, or whatever the current UI exposes.
- Modal resolves successfully.
- Manifest basis hash either stays stable because inputs did not change, or refreshes if the clarification changed the basis.

Capture screenshot:

- `docs/pipeline/full-pipeline-e2e-screenshots/step-07-rerun-modal-clickthrough.png`

If this file already exists, use a timestamped suffix. Do not overwrite existing screenshot evidence.

Validate:

```bash
sqlite3 -header -column /Users/alinaaitova/Desktop/Testproject1/state-e2e-20260504T113157Z/state.db "
SELECT process_id, stoplight, suggested_action, manifest_basis_hash, artifact_hash, updated_at
FROM manifests
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545';
"
```

### Step 5: Quick-Decide

Use the Quick-Decide panel to capture a new decision.

Use disposition:

- `watch`

Suggested reasoning:

```text
E2E desktop UI click-through validation: verified Manifest Inbox, Process Detail, rerun, and Quick-Decide against isolated E2E state.
```

Capture screenshot:

- `docs/pipeline/full-pipeline-e2e-screenshots/step-08-quick-decide-clickthrough.png`

If this file already exists, use a timestamped suffix. Do not overwrite existing screenshot evidence.

Validate:

```bash
sqlite3 -header -column /Users/alinaaitova/Desktop/Testproject1/state-e2e-20260504T113157Z/state.db "
SELECT idempotency_key, status, desired_disposition, decision_note, queued_at
FROM pending_operator_intents
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545'
ORDER BY queued_at DESC
LIMIT 5;
"
```

Expected:

- A new row exists, distinct from `e2e-cli-20260504T145047Z` and `decision-watch-e2e-ui-bridge-20260504T1616Z`.
- Status is `queued`.
- Disposition is `watch`.

### Step 6: Cross-Session Staleness

If practical, validate staleness.

With the UI open on the detail view, trigger an external manifest rerun against the same E2E roots:

```bash
cd /Users/alinaaitova/Desktop/Testproject1
target/debug/daemon \
  --state-root state-e2e-20260504T113157Z \
  --runtime-dir runtime-e2e-20260504T113157Z \
  --artifact-root artifacts-e2e-20260504T113157Z \
  --ingest manifest
```

Reload or refocus the UI as needed.

Expected:

- Stale banner appears on the card or detail view if the manifest basis/hash changed while the UI had an older snapshot.

Capture screenshot if it appears:

- `docs/pipeline/full-pipeline-e2e-screenshots/step-09-stale-banner-clickthrough.png`

If this file already exists, use a timestamped suffix. Do not overwrite existing screenshot evidence.

If the hash does not change and the stale banner cannot be triggered honestly, document that as `NOT OBSERVED - basis stable`, not as a failure.

## Troubleshooting

### App reads wrong DB

Symptom:

- Inbox empty or schema mismatch unrelated to E2E state.

Fix:

- Relaunch with all three `PROCINTEL_DESKTOP_*` env vars and `PROCINTEL_DAEMON_PROGRAM`.
- Check the Tauri logs for the resolved state DB path.

### Browse-only mode

Symptom:

- App opens but says it is in browse-only mode or cannot control daemon.

Fix:

- Ensure `PROCINTEL_DAEMON_PROGRAM=/tmp/testproject1-target-e2e/debug/daemon`.
- Ensure the binary exists and is executable.
- Relaunch from `apps/desktop`.

### Privacy prompt blocks the app

Symptom:

- macOS screen/audio or accessibility prompt prevents clicking.

Fix:

- This is why this handoff needs a machine-control-capable agent/operator. Approve the prompt or use a tool with sufficient desktop permissions.

### Computer Use cannot see the app

Symptom:

- Computer Use lists no actionable Tauri app, or `osascript` reports assistive access denial.

Fix:

- Use direct operator click-through, a CUA agent with proper permissions, or approve assistive access. Do not fake the click-through by only calling the backend bridge again.

## Acceptance Criteria

This continuation is green when:

- Tauri launches cleanly against `/Users/alinaaitova/Desktop/Testproject1/state-e2e-20260504T113157Z/state.db`.
- Manifest Inbox visibly shows the Woodhouses spotlight card.
- Clicking the card opens Process Detail.
- Manifest markdown renders in the detail view.
- UI clarification submission writes a new override row.
- UI rerun button shows progress and completes.
- UI Quick-Decide writes a new `pending_operator_intents` row distinct from the previous CLI and bridge rows.
- Screenshots for inbox, detail, clarification, rerun modal, and Quick-Decide are saved under `docs/pipeline/full-pipeline-e2e-screenshots/`.
- `docs/pipeline/full-pipeline-e2e-validation-manifest.md` is updated from YELLOW caveat to the accurate final status for the UI click-through.

## Suggested Manifest Update

Update these sections:

- Summary
- Stage-by-stage Outcomes, Stage 14b row
- Desktop UI Walkthrough table
- R.17 through R.22 status rows
- Quality Gates if code changed

Do not claim the cross-session stale banner passed unless it was actually observed.

## Quality Gates

If this is docs/screenshots only:

```bash
git diff --check
```

If code changes are made to the desktop app or Tauri bridge:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p procintel-desktop --test query_service
cargo test -p procintel-desktop --test daemon_bridge
cargo test -p procintel-desktop --test native_shell
cd apps/desktop && bunx vitest run
```

## Deliverables

- New screenshots under `docs/pipeline/full-pipeline-e2e-screenshots/`.
- Updated validation manifest.
- Any code fixes required for the UI click-through, with tests.
- Commit and push to `origin/main`.
