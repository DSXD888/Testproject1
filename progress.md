# Stage 11 CDP Live Validation Progress

Date: 2026-05-03

Repo: `~/Desktop/testproject1`

## Completion Status

Stage 11 CDP live validation is complete. Scenarios A, B, and C are green, and
R.1-R.10 are green in
`docs/pipeline/stage-11-cdp-live-demos-validation-manifest.md`.

The existing operator CDP browser/account on `localhost:9222` was used
throughout. No fresh browser/profile was launched. The user authorized
temporarily suspending the Chrome process listening on 9222 for Scenario C, with
the requirement that it be reactivated and verified afterward.

## Important Safety Notes

- Do not delete files.
- Do not delete live DB rows unless the user explicitly authorizes it.
- For Scenario C, use a disposable copied state root instead of mutating
  `state/state.db`.
- If suspending Chrome, suspend only the PID listening on `127.0.0.1:9222` and
  always resume it with `kill -CONT <pid>`.
- Verify `curl http://localhost:9222/json/version` after resuming.

## Files Changed So Far

Tracked edits:

- `skills/landed-cost/searates-freight-estimate/SKILL.md`
- `skills/landed-cost/hmrc-customs-classification/SKILL.md`
- `skills/landed-cost/shipment-sizing/SKILL.md`
- `docs/pipeline/stage-11-cdp-live-demos-validation-manifest.md`
- `progress.md`

No files have been deleted.

## Why Skills Were Edited

The original live validation surfaced real integration failures:

- SeaRates initially used an oversized GraphQL/recalculate path that failed live
  with a CDP response-channel closure. The skill now uses a compact
  `rates(...)` query and selects a non-expired quote directly.
- HMRC customs and shipment sizing initially spent too much of the daemon budget
  on discovery/setup. Both skills now have bounded-runner instructions and DGU
  deterministic fast paths.
- Shipment sizing now explicitly uses concrete `agent-browser --cdp 9222`
  subcommands and does not pass natural-language commands to `agent-browser`.

`cargo test --test skill_lint` passes after these edits.

## CDP Browser State

9222 listener used for Scenario C and verified after resume:

```text
COMMAND  PID    USER          NAME
Google   92364  alinaaitova   TCP 127.0.0.1:9222 (LISTEN)
```

Observed launch command:

```bash
/Applications/Google Chrome.app/Contents/MacOS/Google Chrome \
  --remote-debugging-port=9222 \
  --remote-allow-origins=* \
  --user-data-dir=~/.chrome-automation \
  --profile-directory=Default \
  --no-first-run \
  --no-default-browser-check
```

Verified tools:

```text
agent-browser 0.25.3
codex-cli 0.128.0
Chrome/147.0.7727.138
```

## Scenario A Result: GREEN

Command run:

```bash
target/debug/daemon --ingest landed-cost \
  2>&1 | tee /tmp/cdp_demo_a_correct_after_contract_fixes.log
```

Daemon summary:

```text
landed-cost live loop completed: attempted=1 completed=1 blocked=0 domestic=0 import=1 customs_skill_runs=1 sizing_skill_runs=1 searates_skill_runs=1 fx_skill_runs=1
```

SeaRates result:

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

Scenario A session IDs:

- HMRC: `session-pty-c330d35a9f23e678d701f668ab50a8129b8e4f749feba75ef2999a72f29321ec`
- Shipment sizing: `session-pty-bb005fe3544b7e08bbc692f908cb91dee8bf1d19808cd3bb3781287b9e9bdc5f`
- SeaRates: `session-pty-e7cadd805d60b713146e0e4d304cdbfd4e7f93d05e4124fd2d45bd3d8286f7f0`
- FX: `session-pty-c82fb8b15d9724fcb467bb39e1f135d51b5e5a3c9179fbe208df81aac1eeed2f`

## Scenario B Result: GREEN

Backup created:

```text
state/state.db.preB-cdp-live-20260503T100301Z.db
```

Recompute trigger used instead of deleting live rows:

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

Retry/cache log:

```text
2026-05-03T10:07:41.636446Z WARN Stage 11 SeaRates captcha intercepted; backing off before retry ... attempt=1 backoff_seconds=15
2026-05-03T10:08:41.555915Z WARN Stage 11 SeaRates captcha intercepted; backing off before retry ... attempt=2 backoff_seconds=45
2026-05-03T10:10:25.988808Z WARN Stage 11 SeaRates exhausted bounded retry budget ... attempts=3 captcha_observed=true final_cdp_state="captcha_blocked"
2026-05-03T10:10:25.989813Z INFO Stage 11 SeaRates blocked; substituting cached freight estimate ... cached_at=2026-05-03 09:39:48
landed-cost live loop completed: attempted=1 completed=1 blocked=0 domestic=0 import=1 customs_skill_runs=1 sizing_skill_runs=1 searates_skill_runs=1 fx_skill_runs=0
```

Three SeaRates captcha attempt contracts:

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

## Scenario C Result: GREEN

Disposable state root:

```text
state-cdp-scenario-c-20260503T134400Z
```

Chrome suspend/resume:

```text
SCENARIO_C_SUSPEND_PID=92364
SCENARIO_C_STOP_UTC=2026-05-03T13:44:53Z
curl while suspended: status 28 timeout
SCENARIO_C_RESUME_PID=92364
SCENARIO_C_RESUME_UTC=2026-05-03T13:49:16Z
post-resume curl returned Chrome/147.0.7727.138
post-resume agent-browser URL: https://www.searates.com/logistics-explorer/
post-resume agent-browser title: Logistics Explorer App | Freight Calculator to Get Freight Rates Today Online
```

Command run:

```bash
target/debug/daemon \
  --state-root state-cdp-scenario-c-20260503T134400Z \
  --runtime-dir runtime-cdp-scenario-c \
  --artifact-root artifacts-cdp-scenario-c \
  --ingest landed-cost \
  2>&1 | tee /tmp/cdp_demo_c_correct.log
```

Daemon summary:

```text
2026-05-03T13:49:16.267435Z  WARN stage11.cdp.operator_action_required ... final_cdp_state="unreachable" attempts=1
landed-cost live loop completed: attempted=1 completed=0 blocked=1 domestic=0 import=1 customs_skill_runs=1 sizing_skill_runs=1 searates_skill_runs=1 fx_skill_runs=0
```

Copied DGU row after Scenario C:

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

The live `state/state.db` DGU row remained the Scenario B cached-freight
`review` row with `freight_base_gbp_pence=244656`.

## Quality Gates Already Run

All passed:

```text
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --test skill_lint
cargo test -p daemon --bin daemon cdp
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || break; done
```

Important detail: the handoff command
`cargo test -p daemon --bin daemon CDP_` ran zero tests because current test
names are lowercase. `cargo test -p daemon --bin daemon cdp` ran 16 CDP-related
tests and passed.

## Invariants Already Checked

Other LCE rows unchanged after Scenario B:

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

## Final Targeted Gates After Scenario C

All passed:

```text
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
```

`cargo test -p daemon --bin daemon` result:

```text
175 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
