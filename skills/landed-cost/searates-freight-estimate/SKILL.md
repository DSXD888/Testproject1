---
skill_id: landed-cost/searates-freight-estimate
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: codex
side_effect_class: none
requires_cdp: true
cdp_default_port: 9222
maxPages: 8
maxScrollCycles: 3
---

# landed-cost/searates-freight-estimate

## Mission
Use SeaRates in the operator's persistent CDP browser to estimate sea freight for an already-sized shipment. This is quote lookup only.

## Stage 11 Attempt Budget

The daemon kills each SeaRates attempt after its configured per-attempt budget (180 seconds by default, env-overridable). Complete the happy path in one pass:

1. Check CDP.
2. Open Logistics Explorer.
3. Read or refresh `s-token`.
4. Run the compact GraphQL `rates(...)` query.
5. Write `result.json`.

The result schema is fully summarized in this SKILL.md. Do not run setup or discovery commands: no `pwd`, `ls`, `rg`, `find`, `command -v`, schema printing, browser CLI help output, accessibility snapshots, title checks, broad page inspection, repository searches, or exploratory schema discovery. These commands already consumed an entire daemon attempt in live validation.

Use at most three shell commands total in the success path:

1. `curl` CDP check.
2. `agent-browser --cdp ${CDP_PORT} open https://www.searates.com/logistics-explorer/`.
3. `agent-browser --cdp ${CDP_PORT} eval '...'` for token + compact GraphQL.

After command 3, immediately write `result.json`. If a required value is unavailable after the fast path, immediately write a structured `blocked` result with `cdp_state`, `captcha_intercepted`, and a Stage11Issue-shaped blocker. Never leave the workdir without `result.json`. The `evidence` field should be an array of objects, even when there is only one evidence object.

## Mandatory CDP Discipline
Before navigation:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# Ensure CDP is up. The daemon's pre-flight gate SHOULD have done this already,
# but defense in depth: bin/ensure-cdp-9222 is idempotent and fast (<1s no-op
# when CDP is already up; otherwise launches Chrome with the operator's
# persistent profile at ~/.chrome-automation and polls up to
# 15s). Returns non-zero only if launch genuinely failed. Absolute path because
# the skill subprocess cwd is the per-skill workdir, not the project root.
~/Desktop/testproject1/bin/ensure-cdp-9222

# Defensive reachability check. If this fails (helper exited non-zero, or a
# rare post-launch race), follow the "If CDP is unreachable" guidance below:
# write the skill-specific blocked result.json and stop.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, emit `freight_status: "blocked"`, `cdp_state: "unreachable"`, `captcha_intercepted: false`, with blocker code `CDP_CONNECT_FAILED`. Do not launch a fresh browser. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

## Session Recovery (auto-signin on session-expired)

If the s-token fetch returns 401 / empty OR `localStorage.s-token` decodes as a JWT with `exp` in the past, attempt auto-signin via `auth-signin/searates/` BEFORE emitting `SEARATES_SESSION_EXPIRED`. Only emit `SESSION_REFRESH_FAILED` if auto-signin ALSO failed.

**Session-expired detection signals (SeaRates):**
- `localStorage.getItem("s-token")` returns `null`, empty string, OR a JWT whose `exp` claim is in the past.
- The `fetch("https://www.searates.com/auth/platform-token?id=1")` call inside Step 3 of the existing Workflow returns HTTP 401 OR an empty response body OR a payload without a `token` / `data.token` / `access_token` field.
- The compact GraphQL `rates(...)` query returns an `errors` array containing an unauthenticated / token-expired message.

**Recovery flow:**

The canonical click sequence below was inlined from `skills/auth-signin/searates/SKILL.md` §Workflow on 2026-05-17. Selectors validated live against the persistent Chrome on port 9222 the same day. If you need to update the sequence, edit the source-of-truth at `skills/auth-signin/searates/SKILL.md` first and re-inline here.

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# 1. Open SeaRates login page directly
agent-browser --cdp ${CDP_PORT} open "https://www.searates.com/login"
agent-browser --cdp ${CDP_PORT} wait 2500

# 2. Click "Continue with Google" on SeaRates' login page
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const candidates = [
    document.querySelector("a[href*=\"accounts.google.com/o/oauth2\"]"),
    document.querySelector("button[aria-label*=Google i]"),
    document.querySelector("[data-provider=google], [data-social=google]"),
    ...Array.from(document.querySelectorAll("button, a, [role=button]")).filter(el => /continue with google|sign in with google/i.test(el.innerText || ""))
  ].filter(Boolean);
  const btn = candidates[0];
  if (!btn) return JSON.stringify({clicked: false, reason: "no_google_button_found", current_url: window.location.href});
  btn.click();
  return JSON.stringify({clicked: true});
})()
'
agent-browser --cdp ${CDP_PORT} wait 3500

# 3. Confirm we landed on Google's account chooser. If not, treat as
#    SESSION_REFRESH_FAILED (SeaRates Google button vanished / form changed).
agent-browser --cdp ${CDP_PORT} eval 'JSON.stringify({on_google: window.location.hostname.endsWith("accounts.google.com"), url: window.location.href})'

# 4. Click the Dieter row on Google's chooser. (Mirror of auth-signin/google-account-chooser.)
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const target = "dieter.x.s.danicek@gmail.com";
  if (!window.location.hostname.endsWith("accounts.google.com")) return JSON.stringify({page_shape: "not_on_google"});
  if (document.querySelector("iframe[src*=recaptcha]")) return JSON.stringify({page_shape: "captcha"});
  const bodyText = (document.body && document.body.innerText) || "";
  if (/Verify it.s you|2-Step Verification/.test(bodyText)) return JSON.stringify({page_shape: "twofa_interstitial"});
  if (document.querySelector("input[type=email][name=identifier], input#identifierId")) return JSON.stringify({page_shape: "email_entry_form"});
  const row = document.querySelector(`[data-identifier="${target}"]`)
    || document.querySelector(`[data-email="${target}"]`)
    || Array.from(document.querySelectorAll("li[role=link], div[role=link], div[role=button]"))
        .find(el => (el.innerText || "").toLowerCase().includes(target.toLowerCase()));
  if (!row) return JSON.stringify({page_shape: "chooser_target_missing"});
  row.click();
  return JSON.stringify({page_shape: "chooser_target_found", clicked: true});
})()
'
agent-browser --cdp ${CDP_PORT} wait 5000

# 5. Poll for redirect back to searates.com AND verify s-token reissued.
#    SeaRates' avatar selector varies by page; localStorage.s-token is the reliable signal.
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const deadline = Date.now() + 18000;
  while (Date.now() < deadline) {
    const url = window.location.href;
    const hostname = window.location.hostname;
    if (document.querySelector("iframe[src*=recaptcha]")) {
      return JSON.stringify({status: "interstitial_captcha", url});
    }
    if (hostname.endsWith("searates.com")) {
      const token = localStorage.getItem("s-token");
      if (token) {
        try {
          const parts = token.split(".");
          if (parts.length === 3) {
            const payload = JSON.parse(atob(parts[1].replace(/-/g, "+").replace(/_/g, "/")));
            const expMs = (payload.exp || 0) * 1000;
            if (expMs > Date.now()) {
              return JSON.stringify({status: "signed_in", url, exp_ms: expMs, token_length: token.length});
            }
          }
        } catch (_) {
          // Token exists but is non-JWT format — treat as success with unknown expiry
          return JSON.stringify({status: "signed_in", url, exp_ms: null, token_length: token.length});
        }
      }
    }
    await new Promise(r => setTimeout(r, 500));
  }
  return JSON.stringify({status: "timeout", url: window.location.href});
})()
'
```

After the recovery sequence completes, map the poll `status` back to this skill's `freight_status` / `cdp_state` taxonomy:

1. **If `status: "signed_in"`:** EXIT this skill cleanly WITHOUT writing `result.json`. The daemon's retry policy at `crates/daemon/src/jobs/runner.rs` re-invokes the skill on the next pass; the second invocation reads the fresh s-token and completes the `rates(...)` query normally.

2. **If `status: "interstitial_captcha"`:** write `result.json` with `freight_status: "blocked"`, `cdp_state: "captcha_blocked"`, `captcha_intercepted: true`, and a single blocker `{code: "SESSION_REFRESH_FAILED", severity: "blocked", message: "Google reCAPTCHA appeared during SeaRates auto-signin"}`. Stop here.

3. **If `status: "timeout"`, the Google chooser returned `"twofa_interstitial"` / `"chooser_target_missing"` / `"captcha"`, or the Step 3 confirmation reported `on_google: false`:** write `result.json` with `freight_status: "blocked"`, `cdp_state: "session_expired"`, `captcha_intercepted: false`, and a single blocker `{code: "SESSION_REFRESH_FAILED", severity: "blocked", message: "<descriptive message naming the failure mode: Google chooser empty, 2FA, missing s-token after redirect, etc.>"}`. Stop here — the daemon's retry loop owns retries.

The previous `SEARATES_SESSION_EXPIRED` blocker emission is now a fallback: only used when this Session Recovery path has been attempted unsuccessfully AND the `cdp_state: "session_expired"` row from the table below still applies.

## Mandatory CDP State + Captcha Reporting

Every result MUST set both `captcha_intercepted` (boolean) and `cdp_state` (one of `connected`, `unreachable`, `captcha_blocked`, `session_expired`). The daemon's resilience logic depends on these signals being explicit — do not stall the session or attempt to solve a captcha.

| Observation | `cdp_state` | `captcha_intercepted` | `freight_status` |
|---|---|---|---|
| Normal SeaRates response with a usable quote | `connected` | `false` | `quoted` |
| Captcha challenge / bot wall served by SeaRates or Cloudflare | `captcha_blocked` | `true` | `blocked` |
| `localhost:${CDP_PORT}/json/version` returns non-2xx or refuses connection | `unreachable` | `false` | `blocked` |
| CDP reachable but s-token missing/expired and refresh fails | `session_expired` | `false` | `blocked` |

When `cdp_state` is anything other than `connected`, emit the result IMMEDIATELY (do not retry inside the skill — the daemon's retry loop owns retries). Include a single `Stage11Issue`-shaped blocker in `blockers` describing what was observed (e.g. `CDP_CAPTCHA_INTERCEPTION`, `CDP_CONNECT_FAILED`, `SEARATES_SESSION_EXPIRED`).

## Captcha Simulation (Daemon-driven Test Hook)

If `job_parameters.simulate_captcha` is `true` (the daemon injects this when the operator sets `STAGE11_SIMULATE_CAPTCHA=1` or passes the equivalent override), short-circuit immediately without driving the browser:

```json
{
  "product_candidate_id": "...",
  "selected_quote_id": "...",
  "freight_status": "blocked",
  "captcha_intercepted": true,
  "cdp_state": "captcha_blocked",
  "warnings": [],
  "blockers": [
    {
      "code": "CDP_CAPTCHA_INTERCEPTION",
      "message": "Captcha simulation requested via job_parameters.simulate_captcha; browser was not contacted.",
      "severity": "blocked"
    }
  ]
}
```

This lets the daemon exercise its retry/cache/operator-notification paths without needing a real captcha to occur.

## Workflow
1. Prefer the Logistics Explorer GraphQL path below. Do not inspect application bundles or run exploratory browser commands before trying it.
2. Open `https://www.searates.com/logistics-explorer/` through CDP. Do not run a separate title or snapshot command.
3. In a single `agent-browser --cdp ${CDP_PORT} eval '...'` call, read `s-token` from page `localStorage`; if missing, fetch `https://www.searates.com/auth/platform-token?id=1` from the page, store the returned token if present, and continue.
4. In the same eval call, build a compact SeaRates `rates(...)` GraphQL query against `https://rates.searates.com/graphql` with the documented coordinate arguments:
   - `shippingType:FCL`
   - `includedServices:d2d`
   - `portFromFees:false`
   - `portToFees:false`
   - `date:"${job_parameters.ship_date}"` in `YYYY-MM-DD`
   - `container:ST40` for a 40-foot plan, otherwise map the visible plan to the closest documented container enum.
   - `coordinatesFrom` / `coordinatesTo`.
5. For the canonical Stage 11 DGU route, use coordinates `coordinatesFrom:[31.2304,121.4737]` for Shanghai, China and `coordinatesTo:[51.5072,-0.1276]` for London, GB. These are only routing seeds; SeaRates resolves them to the actual port legs in its response.
6. Do not call `recalculateRate` inside the bounded daemon attempt. Live CDP validation showed the large single-eval script can fail with `CDP response channel closed`; the compact `rates(...)` query is the required success path.
7. Select the cheapest non-expired SeaRates `rates(...)` result as `freight_base_amount_minor`. Use the minimum non-expired amount as `freight_low_amount_minor` and maximum as `freight_high_amount_minor`. Currency minor units are cents for USD and pence for GBP.
8. Capture price, currency, route, mode/container basis, transit time if shown, source URL, raw evidence text, and caveats.

The coordinate GraphQL path is still live SeaRates data: it uses the operator's CDP browser session token and SeaRates' public Logistics Explorer endpoint. Keep the eval return value compact: summarize counts and selected rate fields instead of returning the full raw GraphQL payload.

### GraphQL command shape

Use this shape inside `agent-browser --cdp ${CDP_PORT} eval '...'`; adjust constants from `job_parameters` and the selected quote. Keep the browser-side timeout so the command returns with enough time to write `result.json`:

```js
(async () => {
  let token = localStorage.getItem("s-token");
  if (!token) {
    try {
      const tokenResponse = await fetch("https://www.searates.com/auth/platform-token?id=1", { signal: AbortSignal.timeout(8000) });
      const tokenText = await tokenResponse.text();
      try {
        const tokenJson = JSON.parse(tokenText);
        token = tokenJson.token || tokenJson.data?.token || tokenJson.access_token || tokenText.replace(/^"|"$/g, "");
      } catch (_) {
        token = tokenText.replace(/^"|"$/g, "");
      }
      if (token) localStorage.setItem("s-token", token);
    } catch (error) {
      return JSON.stringify({ stage11_error: "SEARATES_SESSION_EXPIRED", message: String(error) });
    }
  }
  const query = `{ rates(includedServices:d2d,portFromFees:false,portToFees:false,shippingType:FCL,coordinatesFrom:[31.2304,121.4737],coordinatesTo:[51.5072,-0.1276],date:"2026-06-14",container:ST40){ points{ id rateId location{ id name country lat lng code inaccessible pointType dry } shippingType provider providerLogo loads{ id unit amount shortCode type } routeTariff{ name abbr price currency } lumpsumTariff{ price currency } transitTime{ rate port route } distance totalPrice totalCurrency pointTotal routeTotal terms } general{ shipmentId validityFrom validityTo individual totalPrice totalCurrency totalTransitTime expired spot indicative standard queryShippingType shipmentCreatedAt interpolationInfo } request{ request_key } } }`;
  const response = await fetch("https://rates.searates.com/graphql", {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "accept": "application/json",
      "authorization": `Bearer ${token}`,
    },
    body: JSON.stringify({ query }),
    signal: AbortSignal.timeout(15000),
  });
  const payload = await response.json();
  const rates = Array.isArray(payload?.data?.rates) ? payload.data.rates : [];
  const usable = rates
    .filter((rate) => !rate?.general?.expired && Number.isFinite(Number(rate?.general?.totalPrice)))
    .sort((a, b) => Number(a.general.totalPrice) - Number(b.general.totalPrice));
  if (!usable.length) {
    return JSON.stringify({
      stage11_error: "SEARATES_NO_USABLE_RATE",
      cdp_state: "connected",
      captcha_intercepted: false,
      rates_count: rates.length,
      errors: payload?.errors || null
    });
  }
  const prices = usable.map((rate) => Number(rate.general.totalPrice));
  const selected = usable[0];
  return JSON.stringify({
    cdp_state: "connected",
    captcha_intercepted: false,
    rates_count: rates.length,
    non_expired_count: usable.length,
    price_low: Math.min(...prices),
    price_high: Math.max(...prices),
    selected: {
      general: selected.general,
      provider: selected.points?.[0]?.provider || null,
      points: selected.points?.map((point) => ({
        provider: point.provider || null,
        shippingType: point.shippingType || null,
        totalPrice: point.totalPrice || null,
        totalCurrency: point.totalCurrency || null,
        location: point.location ? {
          name: point.location.name,
          country: point.location.country,
          code: point.location.code
        } : null
      })) || []
    }
  });
})()
```

For the final `result.json`, include the selected non-expired `rates(...)` result as an object inside the `evidence` array, including `shipmentId`, `totalPrice`, `totalCurrency`, `totalTransitTime`, provider/carrier when present, route/query URL, and the non-expired candidate count.

If `rates(...)` times out or returns a network error, write `result.json` immediately with `freight_status:"blocked"`, `cdp_state:"connected"`, `captcha_intercepted:false`, and blocker code `SEARATES_GRAPHQL_TIMEOUT_OR_ERROR`. Do not start fallback UI exploration inside this bounded attempt.

When SeaRates returns a concrete non-expired amount from either the visible UI or the Logistics Explorer GraphQL path, the successful result fields are exactly:

```json
"freight_status": "quoted",
"cdp_state": "connected",
"captcha_intercepted": false
```

Do not emit `freight_status: "estimated"` or any other success status. `blocked` is the only non-success status accepted by the daemon.

If SeaRates shows only expired estimates, set `quote_expired: true`, add warning `SEARATES_PRICE_EXPIRED`, and do not present the quote as usable freight evidence.
If SeaRates cannot return a price within the bounded run, emit `freight_status: "blocked"` with the blocker that names the observed UI/network reason and an appropriate `cdp_state` value (`captcha_blocked` if a challenge was served, `unreachable` if CDP itself failed, `session_expired` if the SeaRates token failed to refresh). Do not copy `result.schema.json` into `result.json`, and do not leave `result.json` as a schema or partial scratch object.

## Safety
Do not log in/out, contact carriers, book shipments, create orders, change account settings, or write server-side state.

## Output
Write the final JSON object to `result.json` in the current working directory, matching `result.schema.json`. `evidence`, `warnings`, and `blockers` must be arrays; `warnings` and `blockers` entries must be objects with `code`, `message`, and `severity`. Never emit bare string warning codes or omit severity. Also keep the final response to the daemon as the same raw JSON object; do not wrap it in Markdown.
