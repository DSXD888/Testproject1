# L6 Live Demo Preflight - 2026-05-23

## Verdict

BLOCKED. Do not run the REMOTE_DRAFT live smoke yet.

This pass captured the automatable preflight evidence for `testproject1-3yqf.7.1`, but the live-demo target and bidder profile prerequisites are not present locally. The missing inputs are operator-owned and cannot be fabricated by an agent.

## Captured Evidence

### Toolchain

- `rustup show active-toolchain`: `nightly-aarch64-apple-darwin` from `rust-toolchain.toml`.
- `cargo --version`: `cargo 1.96.0-nightly (90ed291a5 2026-03-05)`.
- `bun --version`: `1.3.10`.
- `git branch --show-current`: `main`.
- Persistent Chrome profile directory: `/Users/alinaaitova/.chrome-automation` exists.
- L5 readiness source: `docs/pipeline/l5-expanded-commercial-intelligence/archived/2026-05-21-l5-expanded-commercial-intelligence/15-archive.md:161` records `ready_for_l6: true`.

### AgentMail

- MCP `health_check`: `status=ok`, `health_level=green`.
- Liveness endpoint: `curl -sfS -m 3 http://127.0.0.1:8765/health/liveness` returned `{"status":"alive"}`.
- Agent identity for this pass: `GrayMouse`.
- Bead: `testproject1-3yqf.7.1`.

### CDP Health

Command:

```bash
bin/ensure-cdp-9222 && curl -sfS -m 3 http://localhost:9222/json/version
```

Observed output:

```json
{"helper":"ensure-cdp-9222","status":"launched","port":9222,"message":"Chrome launched with pid 26928; CDP reachable after 2s","ts":"2026-05-23T14:13:12Z"}
{
  "Browser": "Chrome/148.0.7778.179",
  "Protocol-Version": "1.3",
  "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.0.0 Safari/537.36",
  "V8-Version": "14.8.178.22",
  "WebKit-Version": "537.36 (@4687a88676f95bfbce9ec62314791faae363fea6)",
  "webSocketDebuggerUrl": "ws://localhost:9222/devtools/browser/01fa57fb-f60d-4df4-8ece-04a6cc32d423"
}
```

Follow-up note: a later `/json/list` probe could not reconnect to `localhost:9222` after the helper command completed. Before any live portal run, rerun the CDP health probe and confirm the endpoint remains reachable after the command exits.

### L5 Readiness Fixture

Command:

```bash
cargo test --test bid_readiness_gate
```

Result: 45 passed, 0 failed. The passing tests include the constitutional blockers:

- `l6_t_bid_readiness_gate_003_classification_ambiguous_blocks`
- `l6_t_bid_readiness_gate_004_tariff_branch_divergence_blocks`
- `l6_t_bid_readiness_gate_005_supplementary_unit_missing_blocks`
- `l6_t_bid_readiness_gate_006_broker_required_blocks`
- `l6_t_bid_readiness_gate_007_expired_classification_freshness_blocks`
- `l6_t_bid_readiness_gate_015_reports_multiple_blockers_together`
- `l6_t_bid_readiness_gate_016_non_overrideable_attempt_is_rejected`

### Local Services And Data

- `infra/supabase/config.toml` exists.
- `infra/supabase/migrations/` exists.
- `127.0.0.1:54321` was unreachable during this pass.
- `state/state.db`, `runtime/state.db`, `./state.db`, and `state/state.db.preB-cdp-live-20260503T100301Z.db` did not expose populated `shared_company_profile` or `operator_decision_heads` tables for the live preflight.
- `state/state.db` does include `bid_workflow_states`, `operator_input_overrides`, `pending_operator_intent_receipts`, and `pending_operator_intents`.

## Blocking Items

1. Operator must select a real Procontract opportunity reference for the live demo.
   Required fields: opportunity reference, tender title, deadline, portal status, duplicate-state result, and confirmation that a `REMOTE_DRAFT` run creates no buyer-visible legal commitment.

2. Operator/admin must populate `shared_company_profile` before the live run.
   Required fields include company name, registration number, address, primary contact, current certifications, current insurance policies, and the profile version used by the bid scope.

3. Local Supabase/PostgREST must be reachable or an equivalent remote verification path must be documented.
   The preflight expected the local API on `127.0.0.1:54321`; it was unreachable in this pass.

4. The Chrome saved credential and Touch ID state still require manual confirmation in the persistent profile.
   The locked L6 decision records that `sales@sorelon.com` and Touch-ID-removed credentials were operator-confirmed on 2026-05-21, but this pass did not verify Chrome Passwords directly.

5. CDP persistence must be rechecked immediately before live smoke.
   The helper reached `/json/version`, but a later `/json/list` probe failed after the helper exited.

## Next Step

After the operator supplies the opportunity reference and confirms/populates the company profile, rerun this preflight, convert the blocking sections above to GREEN evidence, then update `docs/pipeline/l6-request-pipeline-automation/manifest.yaml` with the final preflight artifact path.
