# Stage 11 SeaRates — Persistent CDP Operator Runbook

> Operator runbook for verifying the persistent Chrome/CDP browser **before** running a live Stage 11 SeaRates freight validation. Satisfies bead `testproject1-z7zz`.

## Why this matters

Stage 11's SeaRates freight estimate (`skills/landed-cost/searates-freight-estimate`, auth `skills/auth-signin/searates`) drives a real browser over CDP. When the persistent browser is absent, the daemon **correctly persists `CDP_CONNECT_FAILED`** rather than silently failing — the `SeaRatesCdpState` enum (`crates/daemon/src/main.rs:5283`) records the connect state on every attempt. So a Stage 11 run with no browser up does not crash; it records a connect failure and the freight quote is unavailable. This runbook ensures the browser is up and reachable first, so a validation run exercises the real SeaRates path instead of recording `CDP_CONNECT_FAILED`.

## Pre-run: start + verify CDP (do this before any live Stage 11 SeaRates validation)

```bash
# 1. Start (or no-op if already running) the persistent CDP browser.
#    Idempotent: <1s no-op if up; up to ~15s on a cold launch.
bin/ensure-cdp-9222
#    Default port 9222 (override with CDP_PORT or AGENT_BROWSER_CDP_PORT).
#    The helper prints a JSON status line: {"helper":"ensure-cdp-9222","status":"...","port":9222,...}

# 2. Health check — confirm CDP is actually reachable.
curl -sfS -m 3 "http://localhost:9222/json/version"
#    EXPECTED: HTTP 200 + JSON containing "Browser":"Chrome/<version>" and a "webSocketDebuggerUrl".
#    FAILURE:  curl exit 7 / "Couldn't connect" → CDP is NOT up. Re-run bin/ensure-cdp-9222
#              and inspect its JSON status line. Do NOT start a fresh throwaway profile —
#              the persistent profile lives at /Users/alinaaitova/.chrome-automation.
```

If step 2 does not return a healthy `Chrome/<version>` payload, the browser is not reachable and a Stage 11 SeaRates run will persist **`CDP_CONNECT_FAILED`**. Resolve CDP before proceeding.

## Failure mode reference

| Symptom | Meaning | Action |
|---|---|---|
| `curl …/json/version` → exit 7 / connection refused | No CDP browser listening on the port | `bin/ensure-cdp-9222`; re-check |
| Stage 11 persists `CDP_CONNECT_FAILED` | The SeaRates skill could not attach to CDP at run time | Confirm CDP health (above) *before* the run; the persisted state is the daemon behaving correctly, not a daemon bug |
| Browser up but SeaRates auth fails | Persistent profile not signed into SeaRates | Complete sign-in once in the persistent profile (`skills/auth-signin/searates`); the session persists in the profile |

## Production validation instructions (link target)

Before a production Stage 11 SeaRates validation run, complete the **Pre-run** section above and confirm the health check returns a healthy `Chrome/<version>` payload. Per-attempt timeout is bounded by `STAGE11_SKILL_RETRY_PER_ATTEMPT_SECONDS` (`crates/daemon/src/main.rs:296`). A run that records `CDP_CONNECT_FAILED` for all attempts indicates the browser was not reachable — re-run the Pre-run steps and retry; it is not a code defect.
