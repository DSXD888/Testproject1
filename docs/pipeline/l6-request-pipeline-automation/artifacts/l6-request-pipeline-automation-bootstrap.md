# L6 Request Pipeline Automation - Bootstrap

## 1. Scope

This bootstrap is the pre-flight checklist for agents implementing Layer 6 from:

- `docs/strategy/layers/layer-6-request-pipeline-automation.md`
- `docs/pipeline/l6-request-pipeline-automation/manifest.yaml`
- `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md`

It describes the local environment, validation script, fixture expectations, and
quality gate sequence required before claiming L6 work complete.

Layer 6 is skill-first. It does not require new paid commercial accounts or new
paid APIs. The only external account dependency is the already-provisioned
Procontract account `sales@sorelon.com` in the persistent Chrome profile.

## 2. Success Criteria Coverage Summary

| Success criterion | Bootstrap evidence | TDD IDs |
|---|---|---|
| Procontract bid skill can attach to persistent CDP | `bin/ensure-cdp-9222` plus `/json/version` probe | `L6-T-bid_portal_procontract_skill-004`, `L6-T-live_demo-001` |
| BidScopePayload and company profile are available | package fixtures and daemon injection checks | `L6-T-bid_workflow_states-030`, `L6-T-daemon_runner-005` |
| Readiness gate is constitutional | local package tests and IPC pre-check tests | `L6-T-bid_readiness_gate-001` through `016` |
| Side-effect cap prevents unsafe portal mutation | side-effect package tests and live cap evidence | `L6-T-portal_side_effects-001` through `012` |
| Validator fail-closed behavior is implemented | daemon validation executor tests | `L6-T-validator_overlay-008`, `009`, `010` |
| Desktop operator workflow is usable | desktop E2E | `L6-T-desktop_ui-001` through `007` |
| Web viewer remains read-only and redacted | web E2E and sync tests | `L6-T-web_ui-001` through `005`, `L6-T-sync_propagation-003`, `004` |
| Live demo reaches draft saved safely | manual smoke diary | `L6-T-live_demo-002`, `004` |

## 3. Environment Prerequisites

Run these checks from `/Users/alinaaitova/Desktop/Testproject1`.

| Requirement | Required state | Check |
|---|---|---|
| Rust nightly | Toolchain pinned by `rust-toolchain.toml` and available locally. | `rustup show active-toolchain` and `cargo --version` |
| Bun | Bun 1.3.10, as pinned for desktop/web/packages. | `bun --version` |
| Git branch | Work happens on `main`. | `git branch --show-current` |
| AgentMail | Server reachable at `http://127.0.0.1:8765`. | `curl -s http://127.0.0.1:8765/health/liveness` |
| CDP helper | `bin/ensure-cdp-9222` exists and can start or find Chrome on port 9222. | `bin/ensure-cdp-9222` |
| Persistent Chrome profile | `/Users/alinaaitova/.chrome-automation` exists and is the profile used by port 9222. | `test -d /Users/alinaaitova/.chrome-automation` |
| Procontract credential | `sales@sorelon.com` is saved in Chrome Passwords for Procontract. Touch ID is not required. | Manual check at `chrome://settings/passwords` in the CDP profile. |
| Live demo opportunity | Operator selects at least one Procontract opportunity reference that permits a draft-only smoke and, with separate written authorization, a formal-submission smoke. | Record reference, deadline, portal status, duplicate-state check, and authorization scope in the live-demo diary before the run. |
| L5 completion | L5 status confirms `ready_for_l6: true`. | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-status.yaml` |
| Local Supabase stack | Available for sync/RLS/web E2E when those tests run. | `test -f infra/supabase/config.toml && test -d infra/supabase/migrations && bash -lc ':</dev/tcp/127.0.0.1/54321'` |

### 3.1 Six Prerequisite Categories

The implementation preflight must cover these six categories. A green check in
only one category is not sufficient for L6 work.

| Category | Required evidence | Blocking failure |
|---|---|---|
| Toolchain | Rust nightly from `rust-toolchain.toml`, Cargo, Bun 1.3.10, Git on `main`, `rg`, `curl`, and shell tools. | Any missing command or wrong Bun version. |
| Prior layers | L5 status file has `ready_for_l6: true`; L4/L5 sync and RLS infrastructure are present. | L5 not complete or source documents missing. |
| Local services | AgentMail liveness on `127.0.0.1:8765`; local Supabase stack config, migrations, and API port `54321` available for sync/RLS/web E2E. | AgentMail down for coordinated work or Supabase unavailable for RLS tests. |
| Browser automation | `bin/ensure-cdp-9222` succeeds; `/json/version` responds; profile path is `/Users/alinaaitova/.chrome-automation`. | CDP unreachable or wrong profile. |
| Credentials and live target | `sales@sorelon.com` saved in Chrome Passwords; operator-selected Procontract opportunity reference recorded with deadline, status, duplicate-state check, and authorization scope. | Credential missing or no safe draft-only target. |
| Scope controls | Zero new paid accounts; no paid API key variables required; no buyer-visible legal commitment under default cap. | A bead or script requires paid procurement, supplier, freight, FX, tariff, or bid-writing services. |

### 3.2 Manual Checks Not Automatable

The validation script can prove toolchain, files, local services, CDP reachability,
and environment variables. It cannot safely prove the following:

- The saved Chrome password is for `sales@sorelon.com`.
- The selected Procontract opportunity is appropriate for draft-save evidence and any separately authorized formal submission.
- The operator has authorized any formal submission.
- The visible browser session is in a state the operator is comfortable using.

Record these checks in the live-demo diary before T4 smoke.

## 4. External Account Negative List

L6 requires zero new paid commercial accounts.

Do not create, request, or block on:

- Paid procurement-data APIs.
- Paid bid-writing services.
- Paid supplier-pricing APIs.
- Paid freight, FX, tariff, or landed-cost accounts.
- New Procontract credentials.

Allowed external dependency:

- Existing Procontract account `sales@sorelon.com`, already saved in the
  persistent Chrome profile at `/Users/alinaaitova/.chrome-automation`.

If an implementation bead claims it needs a new paid account, file a blocking
spec-drift issue instead of adding the dependency.

## 5. CDP Smoke Prerequisite

Every CDP-backed bid skill smoke starts with the idempotent helper:

```bash
bin/ensure-cdp-9222
curl -sfS -m 3 http://localhost:9222/json/version
```

Expected properties:

- The browser uses `/Users/alinaaitova/.chrome-automation`.
- The browser is not closed by the test or skill.
- The skill attaches to the existing browser and does not launch an isolated
  profile.
- Every browser command uses `agent-browser --cdp ${CDP_PORT}`.
- The bid skill does not attempt portal login outside `auth-signin/procontract`.
- The bid skill does not solve captchas.
- The bid skill does not log out.
- The operator remains signed in as `sales@sorelon.com` or receives
  `BID_PORTAL_AUTH_EXPIRED` for daemon-orchestrated refresh.
- Live smoke under default cap reaches at most `REMOTE_DRAFT`.

Manual credential confirmation:

1. Open the CDP Chrome profile.
2. Visit `chrome://settings/passwords`.
3. Confirm the Procontract credential for `sales@sorelon.com` is saved.
4. Confirm Touch ID is not required for the saved credential path.

## 6. AgentMail Registration Walkthrough

Use the same project key for every L6 implementation agent:

```text
/Users/alinaaitova/Desktop/Testproject1
```

Session start:

```text
ensure_project(human_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="<agent-program>",
  model="<model>",
  task_description="L6 request pipeline automation"
)
set_contact_policy(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<you>",
  policy="open"
)
```

Set the shell identity used by the bead wrappers in the same terminal:

```bash
export AGENT_MAIL_NAME="<you>"
export AGENT_MAIL_PROJECT_KEY="/Users/alinaaitova/Desktop/Testproject1"
```

Handshake every visible peer before using bead wrappers:

```text
for peer in list_agents(project_key="/Users/alinaaitova/Desktop/Testproject1"):
  if peer.name != "<you>":
    macro_contact_handshake(
      project_key="/Users/alinaaitova/Desktop/Testproject1",
      requester="<you>",
      target=peer.name
    )
```

Before editing implementation files:

```text
file_reservation_paths(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<you>",
  paths=["<minimal path or glob>"],
  ttl_seconds=3600,
  exclusive=true,
  reason="<bead-id>"
)
```

For bead-tracked work, use `bin/br-claim <bead-id>`,
`bin/br-complete <bead-id>`, and `bin/br-sync`. Do not bypass the wrappers with
raw status updates.

## 7. Beads Authority

AGENTS.md Rule 2 makes the wrappers mandatory. The authoritative
issue-tracking block is the `br` block in `AGENTS.md` lines 279-330. Do not use
the archived viewer-era issue-tracking block.

Required commands:

```bash
br ready --json
bin/br-claim <bead-id>
bin/br-complete <bead-id>
bin/br-sync
```

Rules for L6 implementation agents:

- Use `bin/br-claim <bead-id>` before changing bead-tracked files.
- Use `bin/br-complete <bead-id>` after the bead's tests and evidence are green.
- Do not use raw `br update --status=in_progress`.
- Do not use raw `br close <id>`.
- Do not use archived viewer-era commands from the stale block.
- Keep `bin/br-sync` separate from Git staging and commits.
- Use the bead ID in reservation reasons, mail subjects, and commit messages.

Minimal bead-tracked closeout:

```bash
bin/br-complete <bead-id>
bin/br-sync
git add .beads/
git add <changed-files>
git commit -m "<bead-id>: <summary>"
git push
```

The wrappers send AgentMail start/complete messages and manage file reservations.
If a wrapper fails, fix AgentMail identity, peer handshake, reservation conflict,
or server liveness. Do not bypass the wrapper.

## 8. Validation Script

The following script checks the L6 pre-flight in one pass and exits non-zero with
actionable errors. It is intentionally local-only and does not create accounts,
write project files, or run destructive commands.

```bash
#!/usr/bin/env bash
set -u

failures=0

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  failures=$((failures + 1))
}

warn() {
  printf 'WARN: %s\n' "$1" >&2
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing command: $1"
}

ROOT="/Users/alinaaitova/Desktop/Testproject1"

if [ "$(pwd)" != "$ROOT" ]; then
  fail "run from $ROOT"
fi

for cmd in cargo rustc rustup bun git curl rg find wc bash sqlite3; do
  require_cmd "$cmd"
done

if [ -f rust-toolchain.toml ]; then
  rustup show active-toolchain >/dev/null 2>&1 || fail "Rust toolchain is not installed"
else
  fail "rust-toolchain.toml is missing"
fi

if command -v bun >/dev/null 2>&1; then
  bun_version="$(bun --version 2>/dev/null || true)"
  if [ "$bun_version" != "1.3.10" ]; then
    fail "expected Bun 1.3.10, got ${bun_version:-unknown}"
  fi
fi

branch="$(git branch --show-current 2>/dev/null || true)"
if [ "$branch" != "main" ]; then
  fail "expected git branch main, got ${branch:-unknown}"
fi

for path in \
  docs/strategy/layers/layer-6-request-pipeline-automation.md \
  docs/pipeline/l6-request-pipeline-automation/manifest.yaml \
  docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md \
  docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-status.yaml \
  infra/supabase/config.toml \
  infra/supabase/migrations \
  crates/daemon/src/main.rs \
  crates/daemon/src/errors/manifest.rs \
  crates/daemon/tests/skill_canary_harness.rs \
  crates/daemon/tests/skill_golden_journeys.rs
do
  [ -e "$path" ] || fail "missing required path: $path"
done

if ! rg -q "ready_for_l6: true" docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-status.yaml; then
  fail "L5 status does not confirm ready_for_l6: true"
fi

if ! curl -sfS -m 3 http://127.0.0.1:8765/health/liveness >/dev/null; then
  fail "AgentMail liveness probe failed at http://127.0.0.1:8765/health/liveness"
fi

if ! bash -lc ':</dev/tcp/127.0.0.1/54321' >/dev/null 2>&1; then
  fail "local Supabase API port 54321 is not reachable"
fi

if [ ! -x bin/ensure-cdp-9222 ]; then
  fail "bin/ensure-cdp-9222 is missing or not executable"
else
  if ! bin/ensure-cdp-9222 >/dev/null 2>&1; then
    fail "bin/ensure-cdp-9222 failed"
  fi
fi

if ! curl -sfS -m 3 http://localhost:9222/json/version >/dev/null; then
  fail "CDP probe failed at http://localhost:9222/json/version"
fi

if [ ! -d /Users/alinaaitova/.chrome-automation ]; then
  fail "persistent Chrome profile /Users/alinaaitova/.chrome-automation is missing"
fi

rust_tests="$(find crates/daemon/tests -maxdepth 1 -name '*.rs' | wc -l | tr -d ' ')"
pkg_tests="$(find packages -path '*/node_modules' -prune -o -name '*.test.ts' -print | wc -l | tr -d ' ')"
printf 'INFO: Rust integration tests: %s\n' "$rust_tests"
printf 'INFO: package *.test.ts files: %s\n' "$pkg_tests"
printf 'INFO: after the 3 L6 package test files exist, cumulative target is >=60\n'

for paid_key in \
  XE_API_KEY \
  OANDA_API_KEY \
  FREIGHTOS_API_KEY \
  SEARATES_API_KEY \
  DUTIFY_API_KEY \
  ZONOS_API_KEY \
  AVALARA_API_KEY \
  SHIPPO_API_KEY
do
  if printenv "$paid_key" >/dev/null 2>&1; then
    fail "paid commercial API key variable is set but L6 requires zero new paid accounts: $paid_key"
  fi
done

warn "Manual check still required: confirm sales@sorelon.com is saved in chrome://settings/passwords for the CDP profile."
warn "Manual check still required: record an operator-selected Procontract opportunity reference before T4 smoke."

if [ "$failures" -ne 0 ]; then
  printf 'L6 preflight failed with %s issue(s).\n' "$failures" >&2
  exit 1
fi

printf 'L6 preflight passed. Complete the two manual checks before live smoke.\n'
```

Recommended storage path if the implementation team wants an executable copy:
`scripts/l6-preflight.sh`. Keep the source of truth in this bootstrap and avoid
adding vendor-account procurement logic.

## 9. Fixture And Corpus Bootstrap

Create the bid portal calibration corpus before promoting the skill beyond
candidate status:

```text
tests/fixtures/calibration/bid-portal-procontract/
  simple/
    canary.inputs.jsonl
    golden.assertions.yaml
    validator-rubric.yaml
    metadata.json
    manifest.md
  medium/
    canary.inputs.jsonl
    golden.assertions.yaml
    validator-rubric.yaml
    metadata.json
    manifest.md
  complex/
    canary.inputs.jsonl
    golden.assertions.yaml
    validator-rubric.yaml
    metadata.json
    manifest.md
```

Each variant must include:

- A portal form complexity label.
- Captured DOM or cassette material sufficient for fast deterministic replay.
- Screenshot expectations for lookup, fill, upload, draft save, and final submit
  where applicable.
- Expected side-effect progression.
- Expected redaction sentinels.
- The Procontract `source_structure_version` used when the capture was made.

The fast tier may replay captured material. The smoke tier uses real CDP. Do not
replace portal behavior with generic HTTP mocks.

### 9.1 Per-Variant Canary Counts

| Variant | Minimum `canary.inputs.jsonl` rows | Required cases |
|---|---:|---|
| `simple` | 6 | Happy draft, missing company profile, stale auth, duplicate draft, cap breach, portal reference not found. |
| `medium` | 8 | Happy draft, upload success, upload hash mismatch, quote override, validation rejection, selector drift, resume draft, duplicate submission. |
| `complex` | 10 | Multi-lot draft, multi-document upload, conditional declarations, legal declaration boundary, final submit authorized, final submit blocked, validator outage, validation override, remote draft changed externally, unknown portal state reconciliation. |

### 9.2 Fixture File Contracts

`metadata.json` must include:

```json
{
  "fixture_id": "bid-portal-procontract-simple",
  "form_complexity": "simple",
  "source_structure_version": "procontract-bid-portal-YYYY-MM-DD",
  "captured_at": "YYYY-MM-DDTHH:MM:SSZ",
  "capture_method": "cdp",
  "portal_host": "procontract.due-north.com",
  "redaction_sentinels": ["..."],
  "side_effect_cap": "REMOTE_DRAFT"
}
```

`manifest.md` must record:

- What page or form shape was captured.
- Which DOM or cassette artifacts belong to the fixture.
- Which screenshots are expected.
- Which sentinel values must be absent from public outputs.
- Which skill version and model profile generated the capture.

`golden.assertions.yaml` must assert:

- Expected terminal state.
- Expected side-effect progression.
- Expected error code, if any.
- Expected screenshot checkpoint names.
- Expected validator overlay verdict.
- Expected redaction result.

`validator-rubric.yaml` must define:

- Evidence anchors required for opportunity ref, process ID, lot IDs, candidate
  IDs, quote IDs, company-profile version, screenshot checkpoints, side-effect
  progression, and confirmation or draft evidence.
- Rejection categories for wrong opportunity, missing screenshots, non-monotonic
  side effects, malformed confirmation reference, missing narrative citations,
  PII leakage, and validator execution outage.
- Approval thresholds for clean simple, medium, and complex run manifests.
- Rubric version emitted into the validator verdict and daemon appendix.

`canary.inputs.jsonl` must be line-delimited JSON. Every row must include:

- `case_id`
- `portal_opportunity_ref`
- `process_id`
- `lot_ids`
- `candidate_ids`
- `selected_quote_ids`
- `company_profile_version`
- `side_effect_cap`
- `expected_terminal_state`
- `expected_error_code`
- `cassette_ref`
- `screenshot_expectations`
- `validator_rubric_ref`

The canary harness must fail before running individual cases if the row count is
below the required count for a variant.

### 9.3 No HTTP Mock Rule

Allowed fast-tier evidence:

- Captured DOM snapshots.
- Captured screenshots.
- Cassettes produced from real CDP portal sessions.
- Local JSON fixtures derived from a captured Procontract page.

Disallowed evidence:

- Generic hand-authored HTTP responses.
- Test-only portal endpoints.
- Mock login success.
- Mock captcha bypass.
- Synthetic final submission confirmation references.

## 10. Package And Daemon Bootstrap Order

Implement in this order to keep failures local:

1. `packages/bidding` types and pure tests:
   - `package.json` with `build`, `lint`, and `test` scripts
   - `tsconfig.json`
   - `vitest.config.ts`
   - `workflow-registry.ts`
   - `procontract-workflow.ts`
   - `company-profile-schema.ts`
   - `bid-scope-types.ts`
   - `tests/bid-workflow-states.test.ts`
   - `tests/portal-side-effects.test.ts`
   - `tests/bid-readiness-gate.test.ts`
2. Daemon Rust state, readiness, IPC, error-code, and dispatch tests.
3. Skill directory and schema files:
   - `skills/bid-portal-procontract/SKILL.md`
   - `skills/bid-portal-procontract/result.schema.json`
   - `skills/bid-portal-procontract/checkpoint.schema.json`
   - `skills/bid-portal-procontract/canary.inputs.jsonl`
   - `skills/bid-portal-procontract/golden.assertions.yaml`
   - `skills/bid-portal-procontract/validator-rubric.yaml`
   - `skills/validation/overlay-bid-portal-procontract/SKILL.md`
4. Skill canary and golden-journey daemon tests.
5. Desktop and web E2E.
6. T4 live smoke evidence.

## 11. Local Quality Gate Sequence

Run this exact local-only sequence before declaring L6 green:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
bun run build
(cd packages/bidding && bun run build)
bun run lint
(cd packages/bidding && bun run lint)
bun run test
(cd packages/bidding && bun run test)
(cd apps/desktop && bun run e2e)
(cd apps/web && bun run test:e2e)
```

Do not substitute `cargo test -p daemon --bin daemon` for the integration loop.
That command excludes the daemon integration tests and is not a valid L6 gate by
itself. Do not rely on the root Bun scripts alone until `packages/bidding` is
explicitly included in the root `build`, `lint`, and `test` scripts; the package
commands above are the direct guard that the new TS tests run.

### 11.1 Gate Evidence To Retain

Completion audit needs concrete evidence, not a prose claim that the gates were
run. Preserve the following in the L6 completion audit or linked run log:

| Gate | Evidence |
|---|---|
| Rust formatting | Command, exit code, timestamp. |
| Rust compile | Command, exit code, final `Finished` line or failing diagnostics. |
| Clippy | Command, exit code, confirmation `-D warnings` was present. |
| Daemon binary tests | Command, exit code, test summary. |
| Daemon integration loop | List of every `crates/daemon/tests/*.rs` target executed and summary per target. |
| Bun build | Command, exit code, workspace target summary. |
| Bun lint | Command, exit code, lint summary. |
| Bun tests | Command, exit code, package test summary. |
| Desktop E2E | Command, exit code, Playwright report path or trace path on failure. |
| Web E2E | Command, exit code, Playwright report path or trace path on failure. |

### 11.2 Expected New Test Inventory

After L6 implementation, these files or extensions must exist:

- `packages/bidding/tests/bid-workflow-states.test.ts`
- `packages/bidding/tests/portal-side-effects.test.ts`
- `packages/bidding/tests/bid-readiness-gate.test.ts`
- `crates/daemon/tests/skill_canary_harness.rs` with `e2e_can_010+`
- `crates/daemon/tests/skill_golden_journeys.rs` with `l3_t_skill_golden_journeys_009+`
- `crates/daemon/tests/redaction_boundary.rs` bid payload/company-profile sentinels
- `crates/daemon/tests/sqlite_supabase_sync.rs` bid workflow projection checks
- `apps/desktop/e2e/bid-workflow.spec.ts`
- `apps/web/e2e/bid-visibility.spec.ts`

The cumulative Rust integration plus package `*.test.ts` count must be at least
60 after the three L6 package test files are added.

## 12. Live Demo Runbook

Default live demo cap:

```text
sideEffectCap = REMOTE_DRAFT
```

Pre-run evidence to record:

- Operator-selected Procontract opportunity reference.
- Portal deadline, current portal status, and tender title copied from the
  authenticated Procontract page.
- Confirmation that the target run will not create buyer-visible legal
  commitment under `REMOTE_DRAFT`.
- Duplicate-state check showing no existing draft/submission for the same
  opportunity, or a recorded operator reconciliation if one exists.
- Company profile version.
- Process ID, lot IDs, candidate IDs, selected quote IDs.
- `decisionBasisHash`.
- CDP `/json/version` response timestamp.
- AgentMail identity and bead ID, if bead-tracked.

Opportunity reference selection rules:

- Prefer an open Procontract opportunity with a visible reference, title,
  response deadline, and response/draft controls reachable after sign-in.
- Prefer a tender where saving a draft does not notify the buyer, lock the
  account, accept legal declarations, or commit funds.
- Avoid opportunities with a same-account draft or submission unless the
  operator explicitly chooses that target for reconciliation testing.
- Avoid opportunities whose deadline is inside the current test window, whose
  portal status is closed/awarded/withdrawn, or whose documents require paid
  tender fees, bonds, or external accounts.
- Record the selected reference in the diary before running `trigger_bid_workflow`.

Run expectations:

- Start from desktop Start Bid control or equivalent local trigger.
- Attach to existing CDP profile.
- Verify opportunity reference and duplicate state.
- Fill form and upload documents.
- Save draft.
- Stop before final submission.
- Persist `draft_saved`.
- Write hybrid run manifest with screenshot trail.
- Push bid-state summary to Supabase.
- Show viewer-role bid-state badge on web without exposing BidScopePayload or
  run manifest body.

Draft-state SQL verification:

```bash
sqlite3 state/state.db "SELECT process_id,bid_state,bid_job_id,stage_run_id,updated_at FROM bid_workflow_states WHERE process_id = '<process-id>';"
```

Expected `bid_state`: `draft_saved`.

Formal-submission smoke:

- Requires separate written operator authorization.
- Requires `sideEffectCap >= FORMAL_SUBMISSION`.
- Requires fail-closed validator accepted verdict or structured validation
  override.
- Requires confirmation reference evidence.
- Requires SQL verification that `bid_workflow_states` reaches `bid_submitted`
  for the authorized run and remains linked to the same process/scope.

Formal-submit SQL verification:

```bash
sqlite3 state/state.db "SELECT process_id,bid_state,bid_job_id,stage_run_id,completed_at FROM bid_workflow_states WHERE process_id = '<process-id>';"
```

Expected `bid_state`: `bid_submitted`.

## 13. False Positive Prevention

| Risk | Prevention |
|---|---|
| Tests pass while skipping hard readiness blockers | `L6-T-bid_readiness_gate-015` requires multi-blocker reporting and `016` rejects override misuse. |
| Portal tests fake behavior through HTTP mocks | Bootstrap requires cassette replay or real CDP only. |
| Auth handling drifts into bid skill | `L6-T-bid_portal_procontract_skill-005` and `L6-T-daemon_runner-009` enforce daemon-orchestrated retry. |
| Final submission fails open during validator outage | `L6-T-validator_overlay-009` blocks submission on outage. |
| Remote draft retry creates duplicate buyer-visible state | `L6-T-portal_side_effects-008`, `009`, and `010` force verification and unknown-state handling. |
| New ErrorCode variants are incomplete | `L6-T-error_codes-001` through `023` require six exact bid variants, per-variant enum/as_str/parser checks, and propagation-site coverage. |
| Web viewer leaks sensitive bid scope | `L6-T-web_ui-002` and `L6-T-sync_propagation-004` test both UI and data boundaries. |
| Desktop allows formal submit under default cap | `L6-T-desktop_ui-007` and `L6-T-portal_side_effects-005` enforce cap. |

## 14. Handoff Checklist

Before an implementation agent starts:

- [ ] Read the L6 spec, manifest, TDD, and this bootstrap.
- [ ] Run the validation script or equivalent checks.
- [ ] Confirm AgentMail identity and peer handshakes.
- [ ] Claim the bead with `bin/br-claim <bead-id>` if bead-tracked.
- [ ] Reserve only the files needed for the bead.
- [ ] Confirm CDP port 9222 responds before any skill smoke.
- [ ] Confirm `sales@sorelon.com` credential is saved manually before live smoke.
- [ ] Confirm no new paid accounts or paid API keys are required.

Before closing L6:

- [ ] All 153 `L6-T-*` requirement-bearing checks are implemented or explicitly
      represented by the named automated/manual surface.
- [ ] Coverage matrix still maps all 127 `L6-FR-*` IDs.
- [ ] Static gates `CI-L6-001` through `CI-L6-010` pass.
- [ ] Live `REMOTE_DRAFT` smoke evidence exists, and the separately authorized
      formal-submission smoke reaches `bid_submitted` with confirmation evidence.
- [ ] No sensitive BidScopePayload, company profile, portal artifact, or manifest
      body leaks to viewer-role surfaces.
- [ ] `git status` and bead status reflect the intended changes only.
