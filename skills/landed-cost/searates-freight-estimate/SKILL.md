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
4. Run the GraphQL query.
5. Write `result.json`.

The result schema is fully summarized in this SKILL.md. Do not run setup or discovery commands: no `pwd`, `ls`, `rg`, `find`, `command -v`, schema printing, browser CLI help output, accessibility snapshots, title checks, broad page inspection, repository searches, or exploratory schema discovery. These commands already consumed an entire daemon attempt in live validation.

Use at most three shell commands total in the success path:

1. `curl` CDP check.
2. `agent-browser --cdp ${CDP_PORT} open https://www.searates.com/logistics-explorer/`.
3. `agent-browser --cdp ${CDP_PORT} eval '...'` for token + GraphQL.

After command 3, immediately write `result.json`. If a required value is unavailable after the fast path, immediately write a structured `blocked` result with `cdp_state`, `captcha_intercepted`, and a Stage11Issue-shaped blocker. Never leave the workdir without `result.json`. The `evidence` field should be an array of objects, even when there is only one evidence object.

## Mandatory CDP Discipline
Before navigation:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, emit `freight_status: "blocked"`, `cdp_state: "unreachable"`, `captcha_intercepted: false`, with blocker code `CDP_CONNECT_FAILED`. Do not launch a fresh browser. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

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
4. In the same eval call, build a SeaRates `rates(...)` GraphQL query against `https://rates.searates.com/graphql` with the documented coordinate arguments:
   - `shippingType:FCL`
   - `includedServices:d2d`
   - `portFromFees:false`
   - `portToFees:false`
   - `date:"${job_parameters.ship_date}"` in `YYYY-MM-DD`
   - `container:ST40` for a 40-foot plan, otherwise map the visible plan to the closest documented container enum.
   - `coordinatesFrom` / `coordinatesTo`.
5. For the canonical Stage 11 DGU route, use coordinates `coordinatesFrom:[31.2304,121.4737]` for Shanghai, China and `coordinatesTo:[51.5072,-0.1276]` for London, GB. These are only routing seeds; SeaRates resolves them to the actual port legs in its response.
6. If the sizing payload has `shipment.container_plan.containers_required` or `shipment.container_plan.container_count`, use that exact value as `containerAmount` when calling `recalculateRate(shipmentId:<id>, containerAmount:<count>)` for each non-expired candidate rate before choosing the freight amount. Do not hard-code a container count; the current DGU operator-override sizing commonly produces 1 x 40-foot container.
7. Select the cheapest non-expired recalculated SeaRates rate as `freight_base_amount_minor`. Use the minimum non-expired recalculated amount as `freight_low_amount_minor` and maximum as `freight_high_amount_minor`. Currency minor units are cents for USD and pence for GBP.
8. Capture price, currency, route, mode/container basis, transit time if shown, source URL, raw evidence text, and caveats.

The coordinate GraphQL path is still live SeaRates data: it uses the operator's CDP browser session token and SeaRates' public Logistics Explorer endpoint. If GraphQL returns a validation error, record that exact response in evidence and then use the visible UI workflow as fallback.

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
  return await response.text();
})()
```

Then recalculate the chosen `general.shipmentId` for the planned container count:

```js
(async () => {
  const token = localStorage.getItem("s-token");
  const query = `{ recalculateRate(shipmentId: 54530266, containerAmount: 12) { points { id rateId location { id name country lat lng code inaccessible pointType } shippingType provider providerLogo loads { id unit amount shortCode type } routeTariff { name abbr price currency terms } lumpsumTariff { price currency } transitTime { rate port route } distance totalPrice totalCurrency pointTotal routeTotal } general { shipmentId validityFrom validityTo individual totalPrice totalCurrency totalTransitTime expired spot standard indicative rateOwner queryShippingType promotionObligations interpolationInfo } request { date shipping_type container_type } } }`;
  const response = await fetch("https://rates.searates.com/graphql", {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "accept": "application/json",
      "authorization": `Bearer ${token}`,
    },
    body: JSON.stringify({ query }),
  });
  return await response.text();
})()
```

For the final `result.json`, include the selected recalculated rate as an object inside the `evidence` array, including `shipmentId`, `totalPrice`, `totalCurrency`, `totalTransitTime`, provider/carrier, port legs, and the route/query URL.

If the 60-second budget is tight, skip recalculate and emit the cheapest concrete non-expired `rates(...)` candidate directly. The daemon acceptance criterion is a fresh positive freight amount with explicit CDP/captcha fields; recalculate is a refinement, not a reason to miss the contract deadline.

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
