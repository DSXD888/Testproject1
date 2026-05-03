# Persistent CDP Agent-Browser Standardization Handoff

ULTRATHINK and be meticulous.

## MISSION

Bring every browser-driven agent skill up to the same operational standard as the current Alibaba and SeaRates Stage 11 flows:

- Agents must use the `agent-browser` browser automation path.
- Login/session-dependent sources must attach to the operator's already-authenticated persistent CDP browser on `localhost:9222`.
- Agents must not launch fresh browser profiles for signed-in sources.
- Agents must not close the operator's browser.
- CDP/captcha/session failures must become structured result-contract fields or blockers, not stalled terminal sessions.

This is a code + skill contract hardening task. The current SeaRates implementation is mostly the model to preserve; the weaker areas are ProContract, source-health/auth-refresh skills, and the lack of daemon/lint enforcement around `requires_cdp`.

Read `AGENTS.md` first. Follow its no-delete rule, main-branch rule, Cargo/Bun rules, and full quality-gate requirements.

---

## WHY THIS MATTERS

The operator may already be signed in to Alibaba, SeaRates, ProContract, or other sources in a Chrome profile exposed through CDP. If a skill launches a fresh `agent-browser` browser, that browser has no cookies/session state and the agent hits login, captcha, stale locale, or missing account context. That caused prior Alibaba failures and is exactly why the Alibaba skill was moved to CDP.

SeaRates is the shipping-cost lookup path. It is currently in good shape:

- `skills/landed-cost/searates-freight-estimate/SKILL.md` declares `requires_cdp: true` and `cdp_default_port: 9222`.
- The skill opens `https://www.searates.com/logistics-explorer/` via `agent-browser --cdp ${CDP_PORT}`.
- It reads or refreshes SeaRates `s-token` in the persistent browser context.
- It calls `https://rates.searates.com/graphql` for live Logistics Explorer rates.
- It emits `captcha_intercepted` and `cdp_state`, which the daemon uses for retry/cache/operator-action behavior.

Official SeaRates docs still match the current approach:

- Logistics Explorer API uses Bearer-token auth from SeaRates platform-token flow: https://docs.searates.com/reference/logistics/introduction
- Get Rates uses the GraphQL endpoint `https://rates.searates.com/graphql` and supports FCL/LCL/AIR/etc. with coordinate, date, service, container, weight, and volume parameters: https://docs.searates.com/reference/logistics/get-rates

Do not redesign SeaRates unless verification finds drift. Treat it as the reference implementation for CDP discipline.

---

## CURRENT AUDIT SNAPSHOT

### Strong / Already Aligned

`skills/marketplace-search/alibaba/SKILL.md`

- Has `requires_cdp: true`.
- Uses `CDP_PORT=${AGENT_BROWSER_CDP_PORT:-...9222}`.
- All browser commands use `agent-browser --cdp ${CDP_PORT}`.
- Forbids `agent-browser close`.
- Actual run evidence shows a persistent signed-in Alibaba Chrome session was used.

`skills/landed-cost/shipment-sizing/SKILL.md`

- Has `requires_cdp: true`.
- Uses CDP because Alibaba listing inspection depends on the signed-in operator browser.
- Forbids launching a fresh browser, login, supplier contact, or external mutations.

`skills/landed-cost/searates-freight-estimate/SKILL.md`

- Has `requires_cdp: true`.
- Uses SeaRates Logistics Explorer through the operator CDP browser.
- Emits `freight_status`, `cdp_state`, `captcha_intercepted`, warnings, and blockers.
- Has a bounded three-command success path and should stay tight.

Daemon Stage 11 SeaRates handling in `crates/daemon/src/main.rs`

- `SeaRatesFreightResult` includes `captcha_intercepted` and `cdp_state`.
- `SeaRatesCdpState` classifies `connected`, `unreachable`, `captcha_blocked`, and `session_expired`.
- `classify_searates_attempt` retries captcha cases only.
- `run_searates_with_retries` wraps bounded SeaRates attempts.
- `lookup_cached_freight` finds recent successful freight rows for fallback.
- `run_landed_cost_live_loop` adds cached-freight warnings or operator-action blockers when SeaRates/CDP is blocked.

### Weak / Needs Fix

`skills/procurement-ingestion/procontract-discover/SKILL.md`

- Uses plain `agent-browser open/fill/click`.
- Contains a credentials section.
- Logs in itself when redirected.
- Calls `agent-browser close`.
- Needs persistent CDP and signed-in-session handling.

`skills/procurement-ingestion/procontract-enrich-tender/SKILL.md`

- Uses plain `agent-browser`.
- Contains a credentials section.
- Logs in itself when redirected.
- Calls `agent-browser close`.
- Needs persistent CDP and signed-in-session handling.

`skills/procurement-ingestion/procontract-register-interest/SKILL.md`

- Uses plain `agent-browser`.
- Contains a credentials section.
- Performs a real portal mutation.
- Calls `agent-browser close`.
- Needs persistent CDP, explicit signed-in precondition, and structured session-blocked results before any mutation.

`skills/procurement-ingestion/contracts-finder/SKILL.md`

- Public source, no login expected.
- Uses plain `agent-browser`.
- Calls `agent-browser close`.
- Decide whether to make all browser skills CDP-only for uniformity, or explicitly mark this public source as `requires_cdp: false` with a documented reason.

`skills/procurement-ingestion/fts/SKILL.md`

- PTY/browser-oriented but has no concrete `agent-browser` CDP playbook.
- Needs either a concrete CDP playbook or an explicit non-CDP justification.

`skills/marketplace-search/amazon-uk/SKILL.md`, `ebay-uk`, `gem-imports`, `mx-wholesale`

- Public/no-login sources.
- Use plain `agent-browser`.
- Call `agent-browser close`.
- For a consistent operator browser model, update to CDP unless there is a written source-specific reason not to.

`skills/source-health/alibaba/SKILL.md`, `skills/auth-refresh/alibaba/SKILL.md`

- PTY/browser-like, login/session-sensitive, but do not contain the concrete CDP command discipline present in `marketplace-search/alibaba`.
- Needs `requires_cdp: true`, CDP preflight, `agent-browser --cdp`, no browser close, and structured auth/session/captcha outcomes.

`skills/source-health/fts/SKILL.md`

- Public source-health canary. Needs either CDP standardization or explicit non-CDP rationale.

`skills/auth-refresh/1688/SKILL.md`

- Login/session-sensitive. Needs the same CDP discipline as Alibaba auth-refresh.

Daemon/lint gap

- `requires_cdp` and `cdp_default_port` exist in some SKILL.md frontmatter, but `crates/daemon/src/main.rs::SkillFrontmatter` does not currently deserialize/store them.
- `crates/daemon/tests/support/skills.rs` also does not lint those metadata fields.
- The prompt renderer simply embeds SKILL.md; it does not add a daemon-level CDP browser contract.
- There is no lint that prevents `agent-browser close` or bare `agent-browser open` in `requires_cdp: true` skills.

---

## TARGET BEHAVIOR

### Required CDP Browser Contract

For every skill that depends on signed-in browser state:

```yaml
requires_cdp: true
cdp_default_port: 9222
```

The SKILL.md playbook must include a mandatory preflight:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable:

- Do not launch a fresh browser.
- Do not attempt login.
- Write a valid `result.json`.
- Mark the result blocked with a source-specific code such as `CDP_CONNECT_FAILED`.
- Include an operator-action message: start Chrome/Chromium with remote debugging, sign in manually, then rerun.

Every browser command in `requires_cdp: true` skills must be:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

Forbidden in `requires_cdp: true` skills:

```bash
agent-browser open ...
agent-browser close
agent-browser install
agent-browser --help
```

Also forbidden for signed-in sources:

- Fresh browser launch.
- Logging in with credentials from the skill.
- Logging out.
- Solving captcha automatically.
- Mutating account settings.

### Public Source Policy

For public/no-login sources, choose one policy and enforce it consistently:

Preferred for this handoff: use persistent CDP for all browser skills, even public ones, so the operator has one observable browser surface and uniform forensic behavior.

Allowed exception: set `requires_cdp: false` only when all of these are true:

- The source is public and never needs signed-in state.
- The skill has no source-specific locale/currency/session dependency.
- The SKILL.md explicitly says fresh browser fallback is allowed.
- The skill still does not close the operator's persistent CDP browser.

### SeaRates Special Rule

Do not add expensive discovery or help-doc commands to SeaRates' live success path. `skills/landed-cost/searates-freight-estimate/SKILL.md` intentionally limits itself to:

1. `curl` CDP preflight.
2. `agent-browser --cdp ${CDP_PORT} open https://www.searates.com/logistics-explorer/`.
3. `agent-browser --cdp ${CDP_PORT} eval '...'` for token + GraphQL.

If you add a generic "use the agent-browser skill docs" instruction, exempt SeaRates from extra runtime documentation commands. Use the official docs and local skill text while editing, not during a bounded daemon attempt.

---

## IMPLEMENTATION PLAN

### 1. Add CDP metadata support in daemon structs

Update `crates/daemon/src/main.rs::SkillFrontmatter`:

```rust
#[serde(default)]
requires_cdp: bool,
#[serde(default)]
cdp_default_port: Option<u16>,
```

Then thread this metadata where useful:

- `skill_config_hash` should include it automatically if it hashes full markdown; verify this rather than assuming.
- Prompt rendering should get enough metadata to include a short daemon-level CDP contract when `requires_cdp=true`.
- Spawn env should include `AGENT_BROWSER_CDP_PORT` for `requires_cdp=true`; use `cdp_default_port.unwrap_or(9222)`.

Do not break existing skills without the fields.

### 2. Add prompt-level CDP contract

Update `crates/daemon/src/prompt/renderer.rs` with an optional browser/CDP contract block. Keep it small and authoritative:

- "This skill requires the operator persistent CDP browser."
- "Use `agent-browser --cdp ${AGENT_BROWSER_CDP_PORT}` for all browser commands."
- "Do not use `agent-browser close`."
- "If CDP is unreachable, emit a blocked result; do not launch a browser."

Add tests around `PromptRenderer::render` proving the block appears only when requested.

Avoid duplicating large SKILL.md content in code. The daemon block is a safety rail; the SKILL.md remains source-specific.

### 3. Add skill lint enforcement

Update `crates/daemon/tests/support/skills.rs`:

- Parse `requires_cdp` and `cdp_default_port`.
- Expose them on `LintedSkillMetadata`.

Update `crates/daemon/tests/skill_lint.rs`:

- Add a test that any `requires_cdp: true` SKILL.md:
  - Contains `agent-browser --cdp`.
  - Contains `http://localhost:${CDP_PORT}/json/version` or equivalent CDP preflight.
  - Does not contain bare `agent-browser open`, `agent-browser click`, `agent-browser fill`, `agent-browser snapshot`, `agent-browser wait`, `agent-browser scroll`, `agent-browser get`, `agent-browser download`, or `agent-browser eval` commands without `--cdp`.
  - Does not contain `agent-browser close`.
- Add a test that signed-in source skills must set `requires_cdp: true`.

Signed-in source set for this pass:

- `procurement-ingestion/procontract-discover`
- `procurement-ingestion/procontract-enrich-tender`
- `procurement-ingestion/procontract-register-interest`
- `marketplace-search/alibaba`
- `landed-cost/shipment-sizing`
- `landed-cost/searates-freight-estimate`
- `source-health/alibaba`
- `auth-refresh/alibaba`
- `auth-refresh/1688`

Decide and document whether public skills are also `requires_cdp: true`.

### 4. Update ProContract skills in place

Files:

- `skills/procurement-ingestion/procontract-discover/SKILL.md`
- `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md`
- `skills/procurement-ingestion/procontract-register-interest/SKILL.md`

Required changes:

- Add `requires_cdp: true` and `cdp_default_port: 9222`.
- Bump `skill_version` because behavior changes.
- Remove credential-driven login instructions from the playbook. Do not copy credential values into replacement text.
- Add a "Mandatory CDP Discipline" section modeled after Alibaba/SeaRates.
- Replace every command with `agent-browser --cdp ${CDP_PORT} ...`.
- Replace "If redirected to Log In, authenticate..." with:
  - Detect login/session-expired page.
  - Emit valid blocked result.
  - Use blocker code `AGENT_AUTH_CHALLENGE` or `PROCONTRACT_SESSION_EXPIRED`.
  - Ask operator to sign into the persistent CDP browser manually, then rerun.
- Remove `agent-browser close`.
- For `register-interest`, preserve external mutation authorization, but require pre-mutation checks:
  - CDP connected.
  - Current page is authenticated.
  - Input contract is in `contracts_to_register`.
  - Already-registered state checked before clicking.

Also update canary/golden files if they assert skill version or browser command text.

### 5. Update Alibaba/1688 operational skills

Files:

- `skills/source-health/alibaba/SKILL.md`
- `skills/auth-refresh/alibaba/SKILL.md`
- `skills/auth-refresh/1688/SKILL.md`

Required changes:

- Add `requires_cdp: true` and `cdp_default_port: 9222`.
- Add exact CDP preflight.
- Use `agent-browser --cdp ${CDP_PORT}` for all browser actions.
- Do not launch a fresh browser.
- Do not close browser.
- Do not solve captchas automatically.
- For auth-refresh, be careful: it should refresh local auth-state metadata only when the operator has already completed interactive login in the CDP browser. If human action is required, emit `blocked_human_action` / source-specific blocker instead of trying to type credentials.

### 6. Update public browser skills

Files to inspect/update:

- `skills/procurement-ingestion/fts/SKILL.md`
- `skills/procurement-ingestion/contracts-finder/SKILL.md`
- `skills/marketplace-search/amazon-uk/SKILL.md`
- `skills/marketplace-search/ebay-uk/SKILL.md`
- `skills/marketplace-search/gem-imports/SKILL.md`
- `skills/marketplace-search/mx-wholesale/SKILL.md`
- `skills/source-health/fts/SKILL.md`

Preferred implementation:

- Add `requires_cdp: true`, `cdp_default_port: 9222`, and CDP discipline to every browser skill.
- Replace bare `agent-browser` commands with `agent-browser --cdp ${CDP_PORT}`.
- Remove `agent-browser close`.

If you choose not to require CDP for a public skill, add:

```yaml
requires_cdp: false
```

Then add a short SKILL.md rationale:

- source is public;
- no authenticated state;
- no operator session needed;
- fresh browser fallback is intentional.

Do not leave browser policy implicit.

### 7. SeaRates verification pass

Do not rewrite SeaRates unless this pass finds a concrete drift.

Verify:

- `skills/landed-cost/searates-freight-estimate/SKILL.md` still has `requires_cdp: true`.
- All `agent-browser` examples include `--cdp`.
- No `agent-browser close`.
- `result.schema.json` still requires `captcha_intercepted` and `cdp_state`.
- `crates/daemon/src/main.rs` still parses and retries based on `SeaRatesFreightResult`.
- `freight_scenario_from_searates` still treats only positive, non-expired quote amounts as usable freight.

Possible small fix:

- If setting `AGENT_BROWSER_CDP_PORT` in the daemon, simplify SeaRates' shell snippet to `CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"` to avoid relying on pseudo-shell syntax for `job_parameters.cdp_port`.

### 8. Update documentation

Add or update a concise validation manifest under `docs/pipeline/`, likely:

- `docs/pipeline/persistent-cdp-agent-browser-standardization-validation-manifest.md`

Include:

- Scope.
- Which skills are now CDP-required.
- Which public skills are intentionally CDP-optional, if any.
- SeaRates verification result.
- Quality gate output.
- Any live CDP limitations if the environment cannot start Chrome on `localhost:9222`.

---

## VALIDATION

Minimum static checks:

```bash
rg -n "agent-browser close" skills
rg -n "agent-browser (open|click|fill|snapshot|wait|scroll|get|download|eval)" skills
rg -n "requires_cdp: true|requires_cdp: false|cdp_default_port" skills -g 'SKILL.md'
```

The first command should have no hits in `requires_cdp: true` skills. The second command should show `--cdp` on every browser action for CDP-required skills.

Daemon/unit checks:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
cargo test --test skill_lint
```

Because this touches Stage 11 and skill/runtime contracts, run the full integration loop before declaring green:

```bash
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

Run UBS on changed files before commit:

```bash
ubs <changed-files>
```

Optional live CDP smoke if a browser is available:

```bash
curl -sfS -m 3 http://localhost:9222/json/version
agent-browser --cdp 9222 open https://www.searates.com/logistics-explorer/
```

Do not perform a live ProContract register-interest mutation unless the daemon input explicitly authorizes a specific contract.

---

## ACCEPTANCE CRITERIA

R.1 Every signed-in browser skill declares `requires_cdp: true` and `cdp_default_port: 9222`.

R.2 Every `requires_cdp: true` skill has a CDP preflight and uses only `agent-browser --cdp ${CDP_PORT}` browser commands.

R.3 No `requires_cdp: true` skill contains `agent-browser close`.

R.4 ProContract skills no longer instruct agents to log in with credentials; they rely on the operator's persistent signed-in CDP browser and emit structured blocked results when auth is missing.

R.5 ProContract register-interest still performs the approved external mutation when, and only when, input contracts authorize it and the authenticated page state is confirmed.

R.6 Alibaba/1688 auth-refresh and source-health skills use the same CDP/captcha/session discipline as Alibaba marketplace search.

R.7 Public browser skills either use CDP too or explicitly declare/document `requires_cdp: false`.

R.8 Daemon metadata parsing understands `requires_cdp` and `cdp_default_port`.

R.9 Prompt rendering and/or launch environment gives CDP-required agents a clear, daemon-level browser contract and an `AGENT_BROWSER_CDP_PORT`.

R.10 Skill lint fails if a CDP-required skill regresses to bare `agent-browser` commands or `agent-browser close`.

R.11 SeaRates remains working and remains the reference Stage 11 freight path: CDP token, GraphQL rates, explicit `cdp_state`, explicit `captcha_intercepted`, daemon retry/cache/operator-action behavior intact.

R.12 Full required quality gates pass, including the per-file integration test loop.

---

## RISKS AND NON-GOALS

Do not delete files.

Do not create alternate skill copies like `SKILL_v2.md`.

Do not move credentials into a new file or document. The preferred fix is to stop skill-side login and require operator-authenticated CDP state.

Do not automate captcha solving.

Do not book freight, contact carriers, contact suppliers, submit bids, or mutate procurement portals except the already-authorized ProContract register-interest action.

Do not broaden SeaRates beyond quote lookup. Booking and account modifications are out of scope.

Do not turn this into the Layer 5 full freight engine. Freightos/SeaRates dual-source decision-grade freight remains a future L5 concern; this handoff is about making current browser/CDP behavior consistent and reliable.

---

## FINAL HANDOFF OUTPUT EXPECTED

When complete, provide:

- Summary of changed files.
- Table of every browser skill and its final browser policy: `requires_cdp=true/false`, CDP port, close behavior, and auth behavior.
- SeaRates verification note.
- Quality gate results.
- Any remaining blockers with concrete follow-up beads/issues.

Commit and push per `AGENTS.md` session-completion rules.
