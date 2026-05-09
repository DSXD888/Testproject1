# Stage 11 CDP Live Demos Continuation Handoff

Date: 2026-05-03

Repository: `/Users/zacfarrell/Desktop/Testproject1`

Purpose: allow a fresh agent to resume exactly where this session stopped and
complete the Stage 11 CDP resilience live validation end to end.

## Current State

Scenarios A and B are green. Quality gates are green. Scenario C is the only
remaining live scenario.

The operator CDP browser is currently up and reachable on `localhost:9222`.
Do not launch a new Chrome profile. Use the existing operator browser/account.

Current listener:

```text
COMMAND  PID    USER          NAME
Google   92364  zacfarrell   TCP 127.0.0.1:9222 (LISTEN)
```

Current browser metadata:

```json
{
  "Browser": "Chrome/147.0.7727.138",
  "Protocol-Version": "1.3",
  "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Safari/537.36"
}
```

Observed launch command:

```bash
/Applications/Google Chrome.app/Contents/MacOS/Google Chrome \
  --remote-debugging-port=9222 \
  --remote-allow-origins=* \
  --user-data-dir=/Users/zacfarrell/.chrome-automation \
  --profile-directory=Default \
  --no-first-run \
  --no-default-browser-check
```

Tool versions:

```text
agent-browser 0.25.3
codex-cli 0.128.0
```

User authorization for Scenario C process suspend:

```text
Sure, go ahead but make sure you reactivate the chrome process listening at 9222 and that everything is signed in etc.
```

No suspend has been performed yet in this session as of this handoff document.

## Safety Rules For Continuation

- Do not delete files or folders.
- Do not delete live DB rows.
- Do not mutate unrelated work from other agents.
- Do not kill or quit Chrome permanently.
- For Scenario C, suspend only the process currently listening on 9222, then
  resume that same process immediately after the Scenario C command returns.
- Always verify `localhost:9222` after resuming Chrome.
- Run Scenario C against a disposable copied state root, not live
  `state/state.db`.

## Working Tree

Current known modified/untracked files:

```text
 M skills/landed-cost/hmrc-customs-classification/SKILL.md
 M skills/landed-cost/searates-freight-estimate/SKILL.md
 M skills/landed-cost/shipment-sizing/SKILL.md
?? docs/pipeline/stage-11-cdp-live-demos-validation-manifest.md
?? docs/pipeline/stage-11-cdp-live-demos-continuation-handoff.md
?? progress.md
```

Do not revert these. They are the live-validation fixes and documentation.

## Why The Skill Files Changed

The original live validation exposed integration problems that were not visible
from unit tests alone:

- `searates-freight-estimate/SKILL.md`: the initial live SeaRates flow attempted
  a larger GraphQL/recalculate path and hit a CDP response-channel closure. The
  skill now uses a compact `rates(...)` query, chooses a non-expired rate
  directly, and preserves the required `captcha_intercepted` and `cdp_state`
  fields.
- `hmrc-customs-classification/SKILL.md`: the skill initially spent too much of
  its daemon budget on discovery/setup and produced an incomplete result once.
  It now has bounded-runner discipline and a DGU fast path that emits the full
  required result shape.
- `shipment-sizing/SKILL.md`: the skill initially used an invalid natural
  language `agent-browser` invocation. It now uses concrete
  `agent-browser --cdp 9222` commands and a deterministic DGU/IGU flat-glass
  fast path.

`cargo test --test skill_lint` passed after these edits.

## Scenario A: GREEN

Command run:

```bash
target/debug/daemon --ingest landed-cost \
  2>&1 | tee /tmp/cdp_demo_a_correct_after_contract_fixes.log
```

Daemon summary:

```text
landed-cost live loop completed: attempted=1 completed=1 blocked=0 domestic=0 import=1 customs_skill_runs=1 sizing_skill_runs=1 searates_skill_runs=1 fx_skill_runs=1
```

SeaRates contract result:

```json
{
  "freight_status": "quoted",
  "cdp_state": "connected",
  "captcha_intercepted": false,
  "freight_base_amount_minor": 330500,
  "freight_low_amount_minor": 330500,
  "freight_high_amount_minor": 644300,
  "freight_currency": "USD"
}
```

Persisted DGU row after Scenario A:

```text
decision_grade=review
freight_low_gbp_pence=244656
freight_base_gbp_pence=244656
freight_high_gbp_pence=476950
declarable_commodity_code_10=7008002000
blockers=[]
```

Relevant session IDs:

```text
HMRC:             session-pty-c330d35a9f23e678d701f668ab50a8129b8e4f749feba75ef2999a72f29321ec
Shipment sizing: session-pty-bb005fe3544b7e08bbc692f908cb91dee8bf1d19808cd3bb3781287b9e9bdc5f
SeaRates:        session-pty-e7cadd805d60b713146e0e4d304cdbfd4e7f93d05e4124fd2d45bd3d8286f7f0
FX:              session-pty-c82fb8b15d9724fcb467bb39e1f135d51b5e5a3c9179fbe208df81aac1eeed2f
```

## Scenario B: GREEN

Backup created before B:

```text
state/state.db.preB-cdp-live-20260503T100301Z.db
```

Recompute was forced without deleting the DGU row by marking only DGU version
metadata stale:

```sql
UPDATE landed_cost_estimates
SET confidence_breakdown = json_set(
      json_set(COALESCE(confidence_breakdown, '{}'),
               '$.selected_quote_row_version', -1),
      '$.supplier_quote_row_version', -1
    ),
    updated_at = CURRENT_TIMESTAMP
WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1';
```

Command run:

```bash
STAGE11_SKILL_RETRY_PER_ATTEMPT_SECONDS=60 \
STAGE11_SKILL_RETRY_BACKOFFS_SECONDS=15,45 \
target/debug/daemon --ingest landed-cost \
  --job-parameters-json '{"simulate_captcha":true}' \
  2>&1 | tee /tmp/cdp_demo_b_correct.log
```

Retry and cache log evidence:

```text
2026-05-03T10:07:41.636446Z WARN Stage 11 SeaRates captcha intercepted; backing off before retry ... attempt=1 backoff_seconds=15
2026-05-03T10:08:41.555915Z WARN Stage 11 SeaRates captcha intercepted; backing off before retry ... attempt=2 backoff_seconds=45
2026-05-03T10:10:25.988808Z WARN Stage 11 SeaRates exhausted bounded retry budget ... attempts=3 captcha_observed=true final_cdp_state="captcha_blocked"
2026-05-03T10:10:25.989813Z INFO Stage 11 SeaRates blocked; substituting cached freight estimate ... cached_at=2026-05-03 09:39:48
landed-cost live loop completed: attempted=1 completed=1 blocked=0 domestic=0 import=1 customs_skill_runs=1 sizing_skill_runs=1 searates_skill_runs=1 fx_skill_runs=0
```

Three SeaRates attempts produced captcha-blocked contracts:

```text
session-pty-f6a6ef9785245031ffb7e6ff9fcf6dfbb97cd702f0a90eff4d58a15339a663f7 blocked captcha_blocked captcha=true
session-pty-246a096884b96faa33f3f89e31b53a63c62ce05734ab392c4eb5c0a145bd5f77 blocked captcha_blocked captcha=true
session-pty-233814bc93b7abee8138d222273ceb09107d7789b20427383a4b694d6753db2f blocked captcha_blocked captcha=true
```

Current live DGU row after Scenario B:

```text
decision_grade=review
freight_low_gbp_pence=244656
freight_base_gbp_pence=244656
freight_high_gbp_pence=476950
blockers=[]
warnings includes FREIGHT_FROM_CACHED_ESTIMATE
created_at=2026-05-03 09:39:48
updated_at=2026-05-03 10:10:25
```

## Already-Passed Quality Gates

These have already passed in this session:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --test skill_lint
cargo test -p daemon --bin daemon cdp
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || break; done
```

Important test-filter note:

```text
cargo test -p daemon --bin daemon CDP_
```

ran zero tests on current HEAD because test names are lowercase. The meaningful
filter is:

```text
cargo test -p daemon --bin daemon cdp
```

That command ran 16 CDP-related tests and passed.

## Already-Checked Invariants

Other LCE rows were unchanged after Scenario B:

```text
Aggregates (general, framework-wide, detailed grades in ITT)  review  0
Ely Library carpet / flooring package                         review  0
Library carpet / floor covering (Ely Library refresh)         review  0
```

Stage 12/13/14 invariants:

```text
candidate_net_margin_assessments: review=17
manifests: clarify=7
dec-2026-05-01-dgu-001: superseded
dec-2026-05-01-dgu-002: withdrawn
DGU operator override: {"qty":250.0,"unit":"square meter"}
```

Structured-code counts after Scenario B:

```text
cached_warning_rows=1
operator_blocker_rows=0
no_cache_blocker_rows=0
```

## Remaining Work: Scenario C

Scenario C must prove CDP-down behavior:

- `cdp_state=unreachable`
- no SeaRates retries
- final DGU row is `blocked`
- blockers include `CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED`
- blockers include `FREIGHT_NO_CACHED_FALLBACK`

Use a disposable state root so live `state/state.db` is not damaged.

### 1. Prepare Disposable State Root

```bash
scenario_c_root="state-cdp-scenario-c-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir "$scenario_c_root"
sqlite3 state/state.db ".backup '$scenario_c_root/state.db'"
```

In the copied DB only, make DGU recomputable and prevent the copied Scenario A/B
row from qualifying as cache by zeroing copied freight:

```bash
sqlite3 -cmd ".timeout 5000" "$scenario_c_root/state.db" "
UPDATE landed_cost_estimates
SET freight_low_gbp_pence = 0,
    freight_base_gbp_pence = 0,
    freight_high_gbp_pence = 0,
    warnings = '[]',
    blockers = '[]',
    confidence_breakdown = json_set(
      json_set(COALESCE(confidence_breakdown, '{}'),
               '$.selected_quote_row_version', -1),
      '$.supplier_quote_row_version', -1
    ),
    updated_at = CURRENT_TIMESTAMP
WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1';
"
```

Confirm the copied DB is primed:

```bash
sqlite3 -header -column "$scenario_c_root/state.db" "
SELECT decision_grade, freight_base_gbp_pence, blockers, warnings,
       json_extract(confidence_breakdown, '$.selected_quote_row_version') AS selected_version,
       json_extract(confidence_breakdown, '$.supplier_quote_row_version') AS supplier_version
FROM landed_cost_estimates
WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1';
"
```

Expected primed copied row:

```text
freight_base_gbp_pence=0
blockers=[]
warnings=[]
selected_version=-1
supplier_version=-1
```

### 2. Suspend Only The 9222 Chrome Process

Re-read the listener immediately before suspending; do not assume PID 92364 if
it changed:

```bash
lsof -nP -iTCP:9222 -sTCP:LISTEN
```

If it still shows PID `92364`, suspend it:

```bash
kill -STOP 92364
```

Verify CDP is unreachable:

```bash
curl -sfS -m 2 http://localhost:9222/json/version
```

Expected: command fails or times out. If it still succeeds, Scenario C is not
valid; do not proceed until 9222 is actually unreachable.

### 3. Run Scenario C Against The Clone

```bash
target/debug/daemon \
  --state-root "$scenario_c_root" \
  --runtime-dir runtime-cdp-scenario-c \
  --artifact-root artifacts-cdp-scenario-c \
  --ingest landed-cost \
  2>&1 | tee /tmp/cdp_demo_c_correct.log
```

Expected daemon behavior:

- HMRC may run.
- Shipment sizing should be skipped via pre-freight CDP-down blocker.
- SeaRates should be skipped.
- No retry/backoff lines should appear.
- Daemon should append the operator-action and no-cache fallback blockers.

### 4. Always Resume Chrome Immediately

Even if Scenario C fails, resume Chrome before diagnosing:

```bash
kill -CONT 92364
```

Verify the browser is reachable again:

```bash
curl -sfS -m 5 http://localhost:9222/json/version
agent-browser --cdp 9222 get url title
```

The user specifically asked to make sure the Chrome process is reactivated and
that the session remains signed in. At minimum, confirm the CDP endpoint is back
and `agent-browser --cdp 9222` can inspect the active tab. If sign-in status is
material to the active website, navigate only if needed and avoid logging out or
changing external state.

### 5. Validate Scenario C Copied DB

```bash
sqlite3 -header -column "$scenario_c_root/state.db" "
SELECT decision_grade, freight_base_gbp_pence, blockers, warnings, updated_at
FROM landed_cost_estimates
WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1'
ORDER BY updated_at DESC LIMIT 1;
"
```

Expected:

```text
decision_grade=blocked
freight_base_gbp_pence=0
blockers contains CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED
blockers contains FREIGHT_NO_CACHED_FALLBACK
```

Log inspection:

```bash
grep -E "cdp_state|retry|attempt|backoff|unreachable|operator_action|FREIGHT_NO_CACHED|landed-cost live loop completed" /tmp/cdp_demo_c_correct.log
```

Expected:

- log includes `cdp_state=unreachable`
- no retry/backoff entries
- final summary should show DGU blocked

### 6. Update Manifest

Update:

```text
docs/pipeline/stage-11-cdp-live-demos-validation-manifest.md
```

Set Scenario C and R.6 to GREEN if the copied DB/log validation passes. Add:

- disposable state root path used
- exact suspend/resume PID
- curl failure while suspended
- curl success after resume
- Scenario C daemon summary
- Scenario C DB row
- log excerpt proving no retries

### 7. Final Checks After Scenario C

Because Scenario C should not require code changes, rerun at least:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
```

If any code or skill files are edited after this handoff, rerun:

```bash
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || break
done
```

The full integration loop already passed before this handoff.

## R.1-R.10 Current Status

| Criterion | Status | Evidence |
|---|---|---|
| R.1 CDP browser launched/reachable | GREEN | 9222 returns Chrome metadata. |
| R.2 Scenario A fresh freight persisted | GREEN | DGU `freight_base_gbp_pence=244656`, `decision_grade=review`, `blockers=[]`. |
| R.3 Scenario A one attempt/no captcha | GREEN | `searates_skill_runs=1`; SeaRates `cdp_state=connected`, `captcha_intercepted=false`. |
| R.4 Scenario B retries/cache warning | GREEN | Three captcha contracts; `FREIGHT_FROM_CACHED_ESTIMATE` persisted. |
| R.5 Scenario B 15/45 backoff | GREEN | Logs show `backoff_seconds=15` then `45`. |
| R.6 Scenario C CDP down | PENDING | Run remaining procedure above. |
| R.7 Other 3 LCE rows unchanged | GREEN | Other rows remain `review`, freight `0`. |
| R.8 Stage 12/13/14 invariants | GREEN | 17 review verdicts, 7 clarify manifests, DGU intents unchanged. |
| R.9 Stage 12 verdict shift documented | GREEN | No shift observed; DGU remains `review`. |
| R.10 Quality gates | GREEN | fmt/check/clippy/bin tests/full integration loop passed. |

## Useful Files And Logs

```text
/tmp/cdp_demo_a_correct_after_contract_fixes.log
/tmp/cdp_demo_b_correct.log
/tmp/cdp_unit_tests_lowercase.log
/tmp/cdp_daemon_bin_tests.log
/tmp/cdp_integration_loop.log
docs/pipeline/stage-11-cdp-live-demos-validation-manifest.md
progress.md
```

## Final Completion Checklist

1. Run Scenario C on a disposable copied state root.
2. Resume Chrome and verify `localhost:9222` works.
3. Update the validation manifest with Scenario C evidence.
4. Rerun final targeted gates.
5. Confirm `git status --short`.
6. Report exactly what is green and any remaining risk.

