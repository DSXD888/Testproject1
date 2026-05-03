# Persistent CDP Agent-Browser Standardization Validation Manifest

## Scope

This manifest validates the browser/CDP contract hardening pass for daemon-driven skills. The pass standardizes browser skills on the operator's persistent Chrome DevTools Protocol browser at `localhost:9222`, blocks instead of launching fresh profiles when CDP/auth is unavailable, and adds daemon/test guardrails for `requires_cdp`.

## Browser Skill Policy

| Skill | requires_cdp | Port | Browser close behavior | Auth/session behavior |
|---|---:|---:|---|---|
| `procurement-ingestion/fts` | true | 9222 | Never closes operator browser | Public source; CDP used for shared observable browser state |
| `procurement-ingestion/contracts-finder` | true | 9222 | Never closes operator browser | Public source; CDP used for shared observable browser state |
| `procurement-ingestion/procontract-discover` | true | 9222 | Never closes operator browser | Requires operator signed-in ProContract CDP session; blocks on auth challenge |
| `procurement-ingestion/procontract-enrich-tender` | true | 9222 | Never closes operator browser | Requires operator signed-in ProContract CDP session; blocks on auth challenge |
| `procurement-ingestion/procontract-register-interest` | true | 9222 | Never closes operator browser | Requires signed-in session and pre-mutation checks before the authorized register-interest click |
| `marketplace-search/alibaba` | true | 9222 | Never closes operator browser | Requires operator signed-in/persistent Alibaba CDP session; blocks on CDP/captcha/auth failures |
| `marketplace-search/amazon-uk` | true | 9222 | Never closes operator browser | Public source; CDP used for shared observable browser state, no login |
| `marketplace-search/ebay-uk` | true | 9222 | Never closes operator browser | Public source; CDP used for shared observable browser state, no login |
| `marketplace-search/gem-imports` | true | 9222 | Never closes operator browser | Public source; CDP used for shared observable browser state, no login |
| `marketplace-search/mx-wholesale` | true | 9222 | Never closes operator browser | Public source; CDP used for shared observable browser state, no account creation |
| `landed-cost/shipment-sizing` | true | 9222 | Never closes operator browser | Uses CDP only to inspect source evidence; no external mutation |
| `landed-cost/searates-freight-estimate` | true | 9222 | Never closes operator browser | Uses persistent SeaRates session token; blocks on CDP/captcha/session expiry |
| `source-health/alibaba` | true | 9222 | Never closes operator browser | Requires operator signed-in Alibaba CDP session; red health result on auth/captcha |
| `source-health/fts` | true | 9222 | Never closes operator browser | Public source-health canary through shared CDP surface |
| `auth-refresh/alibaba` | true | 9222 | Never closes operator browser | Refreshes local auth-state only after operator completes manual browser auth |
| `auth-refresh/1688` | true | 9222 | Never closes operator browser | Refreshes local auth-state only after operator completes manual browser auth |

No public browser skill is CDP-optional in this pass. `landed-cost/simple-fx-lookup` remains `requires_cdp: false` because it is not a browser skill.

## Daemon Guardrails

- `SkillFrontmatter` now deserializes `requires_cdp` and `cdp_default_port`.
- CDP-required skill launches set `AGENT_BROWSER_CDP_PORT` from `cdp_default_port`, defaulting to `9222`.
- Prompt rendering adds a compact persistent-CDP contract only when `requires_cdp=true`.
- Skill binding payloads include the browser policy for telemetry/config-hash visibility.
- Skill lint now fails when a CDP-required skill lacks the localhost CDP preflight, lacks `agent-browser --cdp`, contains bare browser subcommands, or contains browser close/setup/help commands.

## SeaRates Verification

- SeaRates remains `requires_cdp: true` with `cdp_default_port: 9222`.
- The live success path remains bounded to CDP preflight, Logistics Explorer open, and one CDP eval for token plus GraphQL.
- `result.schema.json` still requires `captcha_intercepted` and `cdp_state`.
- Daemon parsing still classifies `connected`, `unreachable`, `captcha_blocked`, and `session_expired`.
- Daemon retry/cache/operator-action paths still use SeaRates `cdp_state` and captcha signals.
- Daemon freight math now treats only positive, non-expired SeaRates quotes as usable freight evidence.
- Official SeaRates docs were checked: the Logistics Explorer docs still describe Bearer-token auth, and Get Rates still documents GraphQL rate fields including shipping type, totals, currency, validity, and expiry.

## Quality Gates

Passed:

- `rg -n "agent-browser close" skills -g 'SKILL.md'`: no hits.
- `rg -n "agent-browser (open|click|fill|snapshot|wait|scroll|get|download|eval|press)" skills -g 'SKILL.md'`: no hits.
- `rg -n "requires_cdp: true|requires_cdp: false|cdp_default_port" skills -g 'SKILL.md'`: all browser skills now expose explicit CDP metadata; `simple-fx-lookup` remains non-browser `requires_cdp: false`.
- `cargo fmt --check`: passed.
- `cargo check --all-targets`: passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo test -p daemon --bin daemon`: passed, 175 tests.
- `cargo test --test skill_lint`: passed, 12 tests.
- Per-file integration loop over `crates/daemon/tests/*.rs`: passed.
- `ubs <changed-files>`: passed after replacing two panic-based test assertions; report covered 11 source files with 0 critical findings.

## Live CDP Limitation

No live ProContract register-interest mutation was performed. Optional live CDP smoke checked `http://localhost:9222/json/version`; no browser was listening in this environment, which is now expected to produce a structured `CDP_CONNECT_FAILED`-style blocker rather than a fresh browser launch.

## Follow-Up

No follow-up issue is currently required for the CDP contract itself. Any future source-specific captcha, portal shape drift, or account recovery issue should be filed against the affected source skill with the observed blocker code and run manifest.
