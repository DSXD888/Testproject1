# Stage 11 CDP Live Demos Validation Manifest

Date: 2026-05-03

Status: GREEN. Scenarios A, B, and C are complete, and R.1-R.10 are green.

## Setup

- Working tree HEAD during validation: `5dce440 docs: mark post-merge gate sweep green`.
- Original handoff referenced `4ba771d`; current HEAD contains later documentation work.
- CDP browser endpoint: `http://localhost:9222/json/version`.
- Browser metadata: `Chrome/147.0.7727.138`, protocol `1.3`.
- Observed launch command:
  `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome --remote-debugging-port=9222 --remote-allow-origins=* --user-data-dir=/Users/alinaaitova/.chrome-automation --profile-directory=Default --no-first-run --no-default-browser-check`
- `agent-browser`: `/Users/alinaaitova/.nvm/versions/node/v20.19.5/bin/agent-browser`, version `0.25.3`.
- `codex`: `/Users/alinaaitova/.nvm/versions/node/v20.19.5/bin/codex`, version `codex-cli 0.128.0`.
- State backup before Scenario B: `state/state.db.preB-cdp-live-20260503T100301Z.db`.
- Disposable Scenario C state root: `state-cdp-scenario-c-20260503T134400Z`.
- Scenario C runtime/artifact roots: `runtime-cdp-scenario-c`,
  `artifacts-cdp-scenario-c`.
- No files were deleted. DGU recomputation was forced by marking only the DGU
  landed-cost row version metadata stale, preserving the Scenario A freight row
  as the cache source.

## Scenario A: CDP Up, Fresh SeaRates Quote

Command:

```bash
target/debug/daemon --ingest landed-cost 2>&1 | tee /tmp/cdp_demo_a_correct_after_contract_fixes.log
```

Daemon summary:

```text
landed-cost live loop completed: attempted=1 completed=1 blocked=0 domestic=0 import=1 customs_skill_runs=1 sizing_skill_runs=1 searates_skill_runs=1 fx_skill_runs=1
```

SeaRates result contract:

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

Notes:

- Scenario A used one SeaRates skill run.
- No retry or cache fallback log lines appeared.
- `warnings` did not contain `FREIGHT_FROM_CACHED_ESTIMATE`.

## Scenario B: CDP Up, Simulated Captcha, Cache Fallback

Preparation:

```bash
sqlite3 state/state.db ".backup 'state/state.db.preB-cdp-live-20260503T100301Z.db'"
sqlite3 -cmd ".timeout 5000" state/state.db "
UPDATE landed_cost_estimates
SET confidence_breakdown = json_set(
      json_set(COALESCE(confidence_breakdown, '{}'),
               '$.selected_quote_row_version', -1),
      '$.supplier_quote_row_version', -1
    ),
    updated_at = CURRENT_TIMESTAMP
WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1';
"
```

Command:

```bash
STAGE11_SKILL_RETRY_PER_ATTEMPT_SECONDS=60 \
STAGE11_SKILL_RETRY_BACKOFFS_SECONDS=15,45 \
target/debug/daemon --ingest landed-cost \
  --job-parameters-json '{"simulate_captcha":true}' \
  2>&1 | tee /tmp/cdp_demo_b_correct.log
```

Retry and cache log excerpt:

```text
2026-05-03T10:07:41.636446Z WARN Stage 11 SeaRates captcha intercepted; backing off before retry ... attempt=1 backoff_seconds=15
2026-05-03T10:08:41.555915Z WARN Stage 11 SeaRates captcha intercepted; backing off before retry ... attempt=2 backoff_seconds=45
2026-05-03T10:10:25.988808Z WARN Stage 11 SeaRates exhausted bounded retry budget ... attempts=3 captcha_observed=true final_cdp_state="captcha_blocked"
2026-05-03T10:10:25.989813Z INFO Stage 11 SeaRates blocked; substituting cached freight estimate ... cached_at=2026-05-03 09:39:48
landed-cost live loop completed: attempted=1 completed=1 blocked=0 domestic=0 import=1 customs_skill_runs=1 sizing_skill_runs=1 searates_skill_runs=1 fx_skill_runs=0
```

Three SeaRates attempt contracts were produced:

```text
session-pty-f6a6ef9785245031ffb7e6ff9fcf6dfbb97cd702f0a90eff4d58a15339a663f7 blocked captcha_blocked captcha=true
session-pty-246a096884b96faa33f3f89e31b53a63c62ce05734ab392c4eb5c0a145bd5f77 blocked captcha_blocked captcha=true
session-pty-233814bc93b7abee8138d222273ceb09107d7789b20427383a4b694d6753db2f blocked captcha_blocked captcha=true
```

Persisted DGU row after Scenario B:

```text
decision_grade=review
freight_low_gbp_pence=244656
freight_base_gbp_pence=244656
freight_high_gbp_pence=476950
blockers=[]
warnings includes FREIGHT_FROM_CACHED_ESTIMATE
```

Notes:

- The logged `backoff_seconds` values matched the configured `15,45` schedule.
- Wall-clock attempt-to-attempt gaps also include Codex skill process startup
  and result validation time.
- Cache fallback reused the Scenario A GBP freight values.

## Scenario C: CDP Down, No Cache

Status: GREEN.

Operator authorization recorded in the handoff:

```text
Sure, go ahead but make sure you reactivate the chrome process listening at 9222 and that everything is signed in etc.
```

Disposable state preparation:

```bash
scenario_c_root="state-cdp-scenario-c-20260503T134400Z"
mkdir "$scenario_c_root"
sqlite3 state/state.db ".backup '$scenario_c_root/state.db'"
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

Primed copied row:

```text
decision_grade=review
freight_base_gbp_pence=0
blockers=[]
warnings=[]
selected_version=-1
supplier_version=-1
```

Suspend/resume evidence:

```text
9222 listener before suspend: PID 92364
SCENARIO_C_SUSPEND_PID=92364
SCENARIO_C_STOP_UTC=2026-05-03T13:44:53Z
curl while suspended: curl: (28) Operation timed out after 2006 milliseconds with 0 bytes received
SCENARIO_C_SUSPENDED_CURL_STATUS=28
SCENARIO_C_RESUME_PID=92364
SCENARIO_C_RESUME_UTC=2026-05-03T13:49:16Z
post-resume curl: Chrome/147.0.7727.138, protocol 1.3
post-resume agent-browser URL: https://www.searates.com/logistics-explorer/
post-resume agent-browser title: Logistics Explorer App | Freight Calculator to Get Freight Rates Today Online
```

Command:

```bash
target/debug/daemon \
  --state-root "state-cdp-scenario-c-20260503T134400Z" \
  --runtime-dir runtime-cdp-scenario-c \
  --artifact-root artifacts-cdp-scenario-c \
  --ingest landed-cost \
  2>&1 | tee /tmp/cdp_demo_c_correct.log
```

Daemon summary:

```text
2026-05-03T13:49:16.267435Z  WARN stage11.cdp.operator_action_required candidate_id=proc-pc-7046e8ec-2734-ed11-8119-005056b64545 candidate=pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279 final_cdp_state="unreachable" attempts=1
landed-cost live loop completed: attempted=1 completed=0 blocked=1 domestic=0 import=1 customs_skill_runs=1 sizing_skill_runs=1 searates_skill_runs=1 fx_skill_runs=0
```

SeaRates blocked contract evidence:

```json
{
  "freight_status": "blocked",
  "captcha_intercepted": false,
  "cdp_state": "unreachable",
  "blockers": [
    {
      "code": "CDP_CONNECT_FAILED",
      "message": "CDP check to http://localhost:9222/json/version timed out after 3 seconds before navigation; persistent operator browser CDP was unreachable.",
      "severity": "blocked"
    }
  ]
}
```

Persisted copied DGU row after Scenario C:

```text
decision_grade=blocked
freight_base_gbp_pence=0
blockers include CDP_CONNECT_FAILED
blockers include CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED
blockers include FREIGHT_NO_CACHED_FALLBACK
updated_at=2026-05-03 13:49:16
```

No retry/backoff evidence:

```text
rg -n "retry|backoff" /tmp/cdp_demo_c_correct.log
exit code 1; no matches
```

Notes:

- Scenario C ran against the disposable copied state root only. The live
  `state/state.db` DGU row remained the Scenario B cached-freight `review` row
  with `freight_base_gbp_pence=244656`.
- The live daemon path launched one SeaRates skill attempt, which immediately
  produced `cdp_state=unreachable`; it did not enter the retry/backoff loop.

## Invariants

Other LCE rows after Scenario B:

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

## R.1-R.10 Status

| Criterion | Status | Evidence |
|---|---|---|
| R.1 CDP browser launched and reachable | GREEN | `curl http://localhost:9222/json/version` returned Chrome metadata. |
| R.2 Scenario A fresh freight persisted | GREEN | DGU `freight_base_gbp_pence=244656`, `decision_grade=review`, `blockers=[]`. |
| R.3 Scenario A one attempt and no captcha | GREEN | `searates_skill_runs=1`; SeaRates result had `cdp_state=connected`, `captcha_intercepted=false`. |
| R.4 Scenario B retries and cached freight warning | GREEN | Three captcha attempt contracts; row has `FREIGHT_FROM_CACHED_ESTIMATE`. |
| R.5 Scenario B backoff schedule | GREEN | Logs show `backoff_seconds=15` then `backoff_seconds=45`. |
| R.6 Scenario C CDP down sanity | GREEN | Disposable clone row is `blocked`, `freight_base_gbp_pence=0`, blockers include `CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED` and `FREIGHT_NO_CACHED_FALLBACK`; log has `final_cdp_state="unreachable"` and `attempts=1` with no retry/backoff lines. |
| R.7 Other 3 LCE rows unchanged | GREEN | Other rows remain review with zero freight. |
| R.8 Stage 12/13/14 invariants intact | GREEN | 17 review net-margin rows, 7 clarify manifests, DGU intents unchanged. |
| R.9 Stage 12 verdict shift documented | GREEN | No shift observed: DGU remains `review`. |
| R.10 Quality gates | GREEN | See transcript below. |

## Findings

- Scenario A produced a live SeaRates quote of `330500` USD minor units
  converted via FX to `244656` GBP pence.
- Scenario B confirmed daemon retry and cache fallback behavior without touching
  real browser state during the simulated captcha attempts.
- Scenario C confirmed CDP-down/no-cache behavior on a copied state root. Chrome
  PID 92364 was suspended only for the scenario window, resumed immediately, and
  verified through both curl and `agent-browser --cdp 9222`.
- The handoff command `cargo test -p daemon --bin daemon CDP_` filters zero tests
  on current HEAD because the test names are lowercase. The equivalent lowercase
  command, `cargo test -p daemon --bin daemon cdp`, ran 16 tests and passed.

## Quality Gate Transcript

```text
cargo fmt --check
PASS

cargo check --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.72s

cargo clippy --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.33s

cargo test --test skill_lint
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

cargo test -p daemon --bin daemon cdp
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 159 filtered out

cargo test -p daemon --bin daemon
test result: ok. 175 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || break; done
PASS. All per-file integration test targets completed successfully.

Final targeted checks after Scenario C:

cargo fmt --check
PASS

cargo check --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.74s

cargo clippy --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.34s

cargo test -p daemon --bin daemon
test result: ok. 175 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
