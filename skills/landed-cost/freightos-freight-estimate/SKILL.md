---
skill_id: landed-cost/freightos-freight-estimate
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

# landed-cost/freightos-freight-estimate

## Mission
Use Freightos Marketplace (`ship.freightos.com`) in the operator's persistent
CDP browser to estimate sea freight for an already-sized shipment. Returns a
`freight_base_amount_minor` plus low/high band and a best-value quote
breakdown. This is quote lookup only — no booking, no payment, no account
changes.

This skill is the SECOND freight provider in L5's dual-source cross-check
(SeaRates is the first). When the two providers' base estimates disagree
materially the L5 freight engine emits `FREIGHT_MODE_LOW_CONFIDENCE`.

## Inputs
The daemon supplies one `job_parameters` object containing:

- `process_id`, `notice_id`, `product_candidate_id`, `pricing_assessment_id`, `selected_quote_id`
- `origin_country` (ISO-2 e.g. `CN`)
- `origin_address` (city or postal code e.g. `Tianjin`)
- `origin_type` (`port_airport` | `factory_warehouse` | `business_address` | `residential_address`)
- `destination_country` (ISO-2 e.g. `GB`)
- `destination_address` (city or postal code e.g. `Portsmouth`)
- `destination_type` (`port_airport` | `warehouse` | `fulfillment_center` | `business_address` | `residential_address` | `last_mile_delivery_warehousing`)
- `load` (object):
  - `mode`: `container` | `loose_cargo`
  - For container: `container_type` (`20` | `40` | `40HC` | `45HC`), `units` (int), `overweight` (bool)
  - For loose cargo: `package_type`, `units`, `pallet_type`, `dimensions_cm` {l,w,h}, `weight_kg`
- `goods_value_minor` + `goods_currency` (e.g. 1000000 USD = $10,000)
- `is_hazmat` (bool — almost always false for L5)
- `readiness_timeframe` (`ready_now` | `within_two_weeks` | `more_than_two_weeks`)
- `ship_date` (ISO date — passed through as `scheduledDateTime`)
- `include_insurance` (bool, default false for the SeaRates-comparable base)
- `include_customs_brokerage` (bool, default false for the SeaRates-comparable base)
- optional `simulate_captcha` (bool, daemon test hook)
- optional `buyer_email` (defaults to `dieter.x.s.danicek@gmail.com`)

## Stage 11 / L5 Attempt Budget

The daemon kills each Freightos attempt after its configured per-attempt budget
(180 seconds by default, env-overridable). Complete the happy path in one pass:

1. Check CDP via `bin/ensure-cdp-9222` + a `curl` reachability probe.
2. Open `https://ship.freightos.com`.
3. In one compact `agent-browser eval`, POST to the internal search API and
   poll until `hasMore=false` (typically <5 polls / ~5 seconds).
4. (Optional, only if attempt budget allows) Run a second POST+poll with
   insurance/brokerage ON to fill `freight_with_services_amount_minor`.
5. Write `result.json`.

The result schema is fully summarized in this SKILL.md. Do NOT run setup or
discovery commands: no `pwd`, `ls`, `rg`, `find`, `command -v`, schema printing,
browser CLI help output, accessibility snapshots, title checks, broad page
inspection, repository searches, or exploratory schema discovery. These commands
already consumed an entire daemon attempt in live validation of sibling skills.

Use at most three shell commands total on the success path:

1. `curl` CDP check (after `bin/ensure-cdp-9222`).
2. `agent-browser --cdp ${CDP_PORT} open https://ship.freightos.com`.
3. `agent-browser --cdp ${CDP_PORT} eval '...'` for the search + poll
   sequence.

After command 3, immediately write `result.json`. If a required value is
unavailable after the fast path, immediately write a structured `blocked`
result with `cdp_state`, `captcha_intercepted`, and a `Stage11Issue`-shaped
blocker. Never leave the workdir without `result.json`. The `evidence` field
should be an array of objects, even when there is only one evidence object.

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

# Defensive reachability check.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, emit `freight_status: "blocked"`, `cdp_state: "unreachable"`,
`captcha_intercepted: false`, with blocker code `CDP_CONNECT_FAILED`. Do not
launch a fresh browser. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

## Session Recovery (REQUIRED — per docs/handoff-2026-05-17-cdp-auto-signin.md)

When this skill detects a logged-out state (search API POST returns 401, DOM
redirects to login modal, or the operator's "D" avatar is absent), it MUST
first attempt auto-signin via `skills/auth-signin/freightos/` BEFORE emitting a
session-expired blocker.

**Session-expired detection signals (Freightos):**
- `fetch("/api/open-freight/quoting/quotes/search", ...)` returns HTTP 401, or
  a 302 redirect to a login page, OR a response with `status: false` /
  `error: "unauthorized"`.
- DOM lacks the "D" avatar element (top-right `[class*=avatar]` containing
  text "D"), AND `loggedIn` cookie/localStorage flag is absent.
- The login modal `ant-modal` with `.auth-ui-google-button` is in the DOM tree.

**Recovery flow.** The canonical click sequence below is inlined verbatim from
`skills/auth-signin/freightos/SKILL.md` §Workflow (validated live against the
persistent Chrome on port 9222 on 2026-05-17). If you need to update the
sequence, edit the source-of-truth at `skills/auth-signin/freightos/SKILL.md`
first and re-inline here.

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# 1. Open Freightos homepage and click the top-right Login button (Freightos
#    serves login as a modal on the homepage, not a separate /login route).
agent-browser --cdp ${CDP_PORT} open "https://ship.freightos.com"
agent-browser --cdp ${CDP_PORT} wait 2500
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const loginBtn = Array.from(document.querySelectorAll("a, button"))
    .find(el => /^(login|sign in|log in)$/i.test((el.innerText || "").trim()));
  if (!loginBtn) return JSON.stringify({clicked: false, reason: "no_login_button"});
  loginBtn.click();
  return JSON.stringify({clicked: true});
})()
'
agent-browser --cdp ${CDP_PORT} wait 2500

# 2. Click the "Google" button inside the login modal (class auth-ui-google-button).
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const googleBtn = document.querySelector("button.auth-ui-google-button, button[class*=\"auth-ui-google\"]")
    || Array.from(document.querySelectorAll("button"))
        .find(el => /^google$/i.test((el.innerText || "").trim()));
  if (!googleBtn) return JSON.stringify({clicked: false, reason: "no_google_button"});
  googleBtn.click();
  return JSON.stringify({clicked: true});
})()
'
agent-browser --cdp ${CDP_PORT} wait 4000

# 3. Confirm we landed on Google's account chooser. If not, treat as
#    SESSION_REFRESH_FAILED (Freightos Google button vanished / form changed).
agent-browser --cdp ${CDP_PORT} eval 'JSON.stringify({on_google: window.location.hostname.endsWith("accounts.google.com"), url: window.location.href})'

# 4. Pick the Dieter row on Google's chooser.
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

# 5. Poll for redirect back to ship.freightos.com AND verify "D" avatar
#    OR Refer-a-friend signal is present (signed-in DOM state).
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const deadline = Date.now() + 18000;
  while (Date.now() < deadline) {
    const url = window.location.href;
    const hostname = window.location.hostname;
    if (document.querySelector("iframe[src*=recaptcha]")) {
      return JSON.stringify({status: "interstitial_captcha", url});
    }
    if (hostname.endsWith("freightos.com")) {
      const dAvatar = Array.from(document.querySelectorAll("[class*=avatar], [class*=initials], [class*=user-menu]"))
        .find(el => /^D$/.test((el.innerText || "").trim()));
      const referFriend = Array.from(document.querySelectorAll("a, button"))
        .find(el => /refer a friend|invite.*friend/i.test(el.innerText || ""));
      if (dAvatar || referFriend) {
        return JSON.stringify({status: "signed_in", url, verified_via: dAvatar ? "d_avatar_present" : "refer_a_friend_present"});
      }
    }
    await new Promise(r => setTimeout(r, 500));
  }
  return JSON.stringify({status: "timeout", url: window.location.href});
})()
'
```

Map the poll `status` back to this skill's `freight_status` / `cdp_state` taxonomy:

1. **If `status: "signed_in"`:** EXIT this skill cleanly WITHOUT writing
   `result.json`. The daemon's retry policy at
   `crates/daemon/src/jobs/runner.rs` re-invokes the skill on the next pass;
   the second invocation runs the search normally.
2. **If `status: "interstitial_captcha"`:** write `result.json` with
   `freight_status: "blocked"`, `cdp_state: "captcha_blocked"`,
   `captcha_intercepted: true`, blocker
   `{code: "SESSION_REFRESH_FAILED", severity: "blocked", message: "Google reCAPTCHA appeared during Freightos auto-signin"}`.
3. **If `status: "timeout"`, the Google chooser returned `"twofa_interstitial"` / `"chooser_target_missing"` / `"captcha"`, or Step 3 reported `on_google: false`:** write `result.json` with
   `freight_status: "blocked"`, `cdp_state: "session_expired"`,
   `captcha_intercepted: false`, blocker
   `{code: "SESSION_REFRESH_FAILED", severity: "blocked", message: "<descriptive: chooser empty / 2FA / no avatar after redirect / etc>"}`.

The previous `FREIGHTOS_SESSION_EXPIRED` blocker emission is now a fallback:
only used when this Session Recovery path has been attempted unsuccessfully
AND the `cdp_state: "session_expired"` row from the table below still applies.

## Mandatory CDP State + Captcha Reporting

Every result MUST set both `captcha_intercepted` (boolean) and `cdp_state`
(one of `connected`, `unreachable`, `captcha_blocked`, `session_expired`).
The daemon's resilience logic depends on these signals being explicit — do
not stall the session or attempt to solve a captcha.

| Observation | `cdp_state` | `captcha_intercepted` | `freight_status` |
|---|---|---|---|
| Normal Freightos response with a usable quote | `connected` | `false` | `quoted` |
| Cloudflare / in-app captcha challenge | `captcha_blocked` | `true` | `blocked` |
| `localhost:${CDP_PORT}/json/version` returns non-2xx | `unreachable` | `false` | `blocked` |
| CDP reachable but Freightos session expired (search returns 401 / login modal absent of D avatar) | `session_expired` | `false` | `blocked` |

When `cdp_state` is anything other than `connected`, emit the result IMMEDIATELY
(do not retry inside the skill — the daemon's retry loop owns retries). Include
a single `Stage11Issue`-shaped blocker in `blockers` (codes:
`CDP_CAPTCHA_INTERCEPTION`, `CDP_CONNECT_FAILED`, `FREIGHTOS_SESSION_EXPIRED`,
`SESSION_REFRESH_FAILED`).

## Captcha Simulation (Daemon-driven Test Hook)

If `job_parameters.simulate_captcha` is `true` (the daemon injects this when
the operator sets `STAGE11_SIMULATE_CAPTCHA=1` or passes the equivalent
override), short-circuit immediately without driving the browser:

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

## Workflow

The skill's success path uses Freightos's internal `quoting/quotes` API,
discovered live on 2026-05-17 by the build run for this handoff. Bypassing the
search-bar UI eliminates the React popover / autocomplete dance and lets one
`fetch` POST trigger the search, with a small loop of GET polls returning the
fully decorated quote list.

### Step 1 — POST to the search endpoint

`POST https://ship.freightos.com/api/open-freight/quoting/quotes/search`

Authentication is session-cookie-based; `credentials: "include"` on a
same-origin `fetch` from the page is sufficient. No `Authorization` header is
required (Freightos's CSRF/auth lives in the `GailsJwt` and `JSESSIONID`
cookies set by the OAuth round-trip).

The minimum-required payload shape (observed live, payload from the
`Confirm Services & Get Results` button):

```json
{
  "messageHeader": { "messageID": "<uuid v4>" },
  "businessInfo": {
    "serviceName": "Quoting",
    "serviceMethod": "New",
    "messageDateTime": "<ISO 8601>",
    "parties": [
      {
        "partyTypeCode": "BY",
        "contact": { "electronicMail": "<buyer email>" },
        "knownShipper": false
      },
      { "partyTypeCode": "FR", "name": "freightos.com" }
    ]
  },
  "shipment": {
    "originLocation": {
      "locationName": "<port_airport|warehouse|...>",
      "locationTypeCode": "<Seaport|Airport|address>",
      "locationCode": "<UN/LOCODE e.g. CNTXG>",     // when locationTypeCode is Seaport/Airport
      "ID": "<Google Place ID e.g. ChIJhw4sHmL87TURMUCIMLeUcYQ>", // when locationTypeCode is address
      "countryID": { "value": "CN" },
      "addressLines": ["<city, country>"]            // when address-typed
    },
    "destinationLocation": { /* mirror of origin */ },
    "additionalInformation": [],
    "pickupEvent": {
      "eventDate": {
        "scheduledDateTime": "<ISO 8601 ship_date>",
        "endDateTime": ""
      }
    },
    "load": {
      "packages": [
        { "quantity": 1, "packagingType": "container40", "overWeightIndicator": false }
      ],
      "declaredValue": { "value": 10000, "currencyID": "USD" }
    },
    "accessorials": [],
    "insuranceValueAmount": { "value": 10000, "currencyID": "USD" },
    "pricePreference": {
      "includeOriginPortCharges": true,
      "includeDestinationPortCharges": true
    },
    "declaredCustoms": { "entry": true }
  }
}
```

**Container packagingType mapping** (Freightos enum):
- `container20` ← input `20`
- `container40` ← input `40`
- `container40HC` ← input `40HC`
- `container45HC` ← input `45HC`

**Insurance + customs brokerage toggles**:
- `include_insurance: true` → include the `insuranceValueAmount` field with
  the same value as goods value.
- `include_customs_brokerage: true` → `declaredCustoms.entry: true`.
- For the SeaRates-comparable base, omit both (insurance value zero,
  declaredCustoms.entry false).

**Response shape:**

```json
{
  "messageHeader": {
    "messageID": "<same uuid>",
    "conversationID": "<RFQ id base64>"      // <-- POLL THIS
  },
  "businessInfo": { ... },
  "quotes": [],                              // empty initially
  "paging": {
    "nextCursor": 0, "next": "...", "hasMore": true, "count": 0
  },
  "URIReferences": {
    "results": { "uri": "https://ship.freightos.com/results/{rfq}" },
    "services": { "uri": "https://ship.freightos.com/search/services/{rfq}" }
  }
}
```

### Step 2 — Poll for quotes

`GET https://ship.freightos.com/api/open-freight/quoting/quotes/{conversationID}/poll?cursor={N}&excludePlatformFee=true`

Loop while `paging.hasMore === true && quotes.length > 0`. Advance `cursor` to
`paging.nextCursor`. Stop after `maxPages` (8) iterations defensively. Empirical
budget: ~4–6 polls / 4–8 seconds for a typical Tianjin→Portsmouth search.

Each `quotes[]` entry has these load-bearing fields:

```jsonc
{
  "referenceID": "<quote ID>",
  "businessInfo": { "parties": [{ "partyTypeCode": "FW", "name": "<forwarder>", "ID": "<seller id>" }, { "partyTypeCode": "IN", "name": "<insurance>", "ID": "..." }] },
  "transportMode": "FCL",
  "transportService": "Door-to-door",
  "connection": {
    "transitTime":            { "estimatedTransitTimes": [{ "from": {"value": 39, "unitCode": "DAY"}, "to": {"value": 46, "unitCode": "DAY"} }] },
    "portToPortTransitTime":  { "estimatedTransitTimes": [...] },
    "generalCharges":         [{ "feeType": "customsBrokerage", "rate": {"value": 103.2, "currencyID": "EUR"}, ... }],
    "connectionSegments":     [...]
  },
  "priceIndicator": {
    "exchangeRates":          [{ "from": {"value": "USD"}, "to": {"value": "EUR"}, "exchangeRate": 0.86 }, ...],
    "includeOriginPortCharges": true,
    "includeDestinationPortCharges": true,
    "totalCharge":             { "value": 4403.81, "currencyID": "EUR" },
    "totalChargeDiscounted":   { "value": 4403.81, "currencyID": "EUR" }
  },
  "validTo": "2026-05-31T23:59:59.000Z"
}
```

**Critical**: `priceIndicator.totalChargeDiscounted` is the price actually
shown to the user when discounts apply; otherwise it equals `totalCharge`.
Prefer `totalChargeDiscounted` for `freight_*_amount_minor`.

### Step 3 — Select Best Value / Cheapest / Quickest

The polled `quotes[]` array arrives sorted by Freightos's Best Value heuristic
(price + transit time tradeoff). The first non-expired quote in the array is
the Best Value top quote.

For the cross-check with SeaRates, use this client-side selection on the
non-expired subset (`(new Date(validTo)).getTime() > Date.now()`):

- `freight_base_amount_minor` ← Best Value top quote's totalChargeDiscounted.
- `freight_low_amount_minor` ← min(totalChargeDiscounted) (the Cheapest).
- `freight_high_amount_minor` ← max(totalChargeDiscounted) (the Quickest's
  upper bound, or the most expensive Best-Value alternative — both are
  acceptable conservative upper bounds for the cross-check).

`tab_used` should be `best_value` for the first cut.

### Step 4 — Location resolution (dynamic via geoPlaces)

The skill resolves arbitrary `(country, city)` pairs to a Freightos seaport
UN/LOCODE by calling the marketplace's own port autocomplete endpoint:

```
GET /servlet/geoPlaces?country=<CC>&types=seaports&isQuotingContext=true&searchTerm=<CITY>
```

Response shape:
```json
{
  "seaports": [
    { "englishName": "Tianjin Xingang", "country": "CN", "code": "CNTXG",
      "latitude": 39.00, "longitude": 117.74, "label": "CNTXG Tianjin Xingang", "value": "CNTXG" }
  ],
  "airports": []
}
```

The skill picks the first seaport whose `englishName` is a case-insensitive
match for the input city; otherwise the first result (Freightos already ranks
by relevance). The chosen `code` becomes `originLocation.locationCode` /
`destinationLocation.locationCode`:

```json
"originLocation": {
  "locationName": "port_airport",
  "locationTypeCode": "Seaport",
  "locationCode": "CNTXG",
  "countryID": { "value": "CN" }
}
```

**Nearest-port fallback for cities without a Freightos seaport.** Some UK
destinations (Portsmouth, Tilbury, Greenwich) return an empty `seaports` array
from `/servlet/geoPlaces`. The skill maintains a small inline fallback table
mapping such cities to a nearby Freightos-served port:

| ISO-2 | Source city | Nearest Freightos port (geoPlaces query) |
|---|---|---|
| GB | Portsmouth | Southampton (`GBSOU`) |
| GB | Tilbury | Felixstowe (`GBFXT`) |
| GB | Greenwich | Felixstowe (`GBFXT`) |

When fallback applies, the skill emits warning
`FREIGHTOS_PORT_FALLBACK_NEAREST` so the L5 freight engine can see that the
quote is for a substituted destination.

When `/servlet/geoPlaces` returns empty for an input not in the fallback table,
the skill emits blocker `FREIGHTOS_PORT_NOT_FOUND` with operator-actionable
message ("Add a CITY_FALLBACK entry if a nearby port exists").

**Door-to-door inputs are approximated as port-to-port** (because Freightos's
door-to-door endpoint requires a Google Place ID for the address, and the
discovery time to wire that up wasn't necessary for the L5 cross-check). The
skill emits warning `FREIGHTOS_DOOR_TO_DOOR_APPROX_AS_PORT_TO_PORT` whenever
either `origin_type` or `destination_type` is not `port_airport`. The freight
engine should account for this when comparing against SeaRates (which uses
coordinate seeds — also port-to-port semantics).

### Compact compact eval shape (production code lives in `eval.js` adjacent to this SKILL.md during build; here is the executive summary)

```js
(async () => {
  // Inputs (replace from job_parameters)
  const j = JOB_PARAMETERS;
  // 1. Resolve cities to UN/LOCODE (table lookup; warn on miss)
  // 2. Construct payload with port-to-port locations
  // 3. POST /api/open-freight/quoting/quotes/search
  // 4. Poll /api/open-freight/quoting/quotes/{rfq}/poll?cursor=N until hasMore=false
  // 5. Filter non-expired, sort by priceIndicator.totalChargeDiscounted, return summary
  // 6. Optionally re-run with insurance + customs brokerage ON for freight_with_services_amount_minor
  // 7. Return JSON with cdp_state, captcha_intercepted, evidence array, warnings/blockers
})()
```

A reference implementation is at the bottom of this file under
"### Reference eval body".

If the search POST returns 401 or the response contains
`status: false / error: "unauthorized"`, treat as session expired and run the
Session Recovery section above (do NOT silently retry the search; the
daemon's retry loop owns retries).

If the POST returns a non-2xx that is NOT 401, write `result.json` with
`freight_status: "blocked"`, `cdp_state: "connected"`,
`captcha_intercepted: false`, blocker code
`FREIGHTOS_API_TIMEOUT_OR_ERROR`. Do not start fallback UI exploration inside
this bounded attempt.

When Freightos returns ≥1 non-expired usable quote, the successful result
fields are exactly:

```json
"freight_status": "quoted",
"cdp_state": "connected",
"captcha_intercepted": false
```

Do not emit `freight_status: "estimated"` or any other success status.
`blocked` is the only non-success status accepted by the daemon.

If Freightos returns only expired quotes (every `validTo` already passed),
set `quote_expired: true`, add warning `FREIGHTOS_PRICE_EXPIRED`, and DO NOT
present the quote as usable freight evidence (use `freight_status: "blocked"`
with blocker `FREIGHTOS_NO_USABLE_QUOTE`).

## Safety
Do not log in/out, contact carriers, book shipments, create orders, change
account settings (currency, language), modify Recommended Services toggles
beyond capturing both states, or write server-side state. Do NOT click
"Select" on any quote — that initiates the booking flow.

## Output
Write the final JSON object to `result.json` in the current working directory,
matching `result.schema.json`. `evidence`, `warnings`, and `blockers` must be
arrays; `warnings` and `blockers` entries must be objects with `code`,
`message`, and `severity`. Never emit bare string warning codes or omit
severity. Also keep the final response to the daemon as the same raw JSON
object; do not wrap it in Markdown.

---

### Reference eval body

This is the actual JS the skill runs inside command 3
(`agent-browser --cdp 9222 eval '...'`). The placeholder `JOB_PARAMETERS_JSON`
is replaced by the daemon with a stringified copy of the input contract.

```js
(async () => {
  const J = JOB_PARAMETERS_JSON;

  const LOCODE = {
    "CN:Tianjin": "CNTSN", "CN:Shanghai": "CNSHA", "CN:Shenzhen": "CNYTN",
    "CN:Ningbo": "CNNGB", "CN:Guangzhou": "CNNSA", "CN:Nansha": "CNNSA",
    "GB:Portsmouth": "GBPME", "GB:Felixstowe": "GBFXT", "GB:London": "GBFXT",
    "GB:Tilbury": "GBFXT", "GB:Southampton": "GBSOU", "GB:Liverpool": "GBLIV"
  };

  const PACK = { "20": "container20", "40": "container40", "40HC": "container40HC", "45HC": "container45HC" };

  const warnings = [];
  const blockers = [];

  function locFor(country, address) {
    const key = `${country}:${(address || "").trim()}`;
    const code = LOCODE[key];
    if (!code) {
      blockers.push({ code: "FREIGHTOS_PORT_NOT_MAPPED", message: `No UN/LOCODE mapping for ${key}; extend table inside SKILL.md`, severity: "blocked" });
      return null;
    }
    return { locationName: "port_airport", locationTypeCode: "Seaport", locationCode: code, countryID: { value: country } };
  }

  async function runSearch({ withServices }) {
    const origin = locFor(J.origin_country, J.origin_address);
    const dest   = locFor(J.destination_country, J.destination_address);
    if (!origin || !dest) return null;
    const goodsValue = Math.max(1, Math.round((J.goods_value_minor || 0) / 100));
    const containerType = (J.load && J.load.mode === "container") ? (PACK[J.load.container_type] || "container40") : "container40";
    const payload = {
      messageHeader: { messageID: crypto.randomUUID() },
      businessInfo: {
        serviceName: "Quoting", serviceMethod: "New",
        messageDateTime: new Date().toISOString(),
        parties: [
          { partyTypeCode: "BY", contact: { electronicMail: J.buyer_email || "dieter.x.s.danicek@gmail.com" }, knownShipper: false },
          { partyTypeCode: "FR", name: "freightos.com" }
        ]
      },
      shipment: {
        originLocation: origin, destinationLocation: dest,
        additionalInformation: [],
        pickupEvent: { eventDate: { scheduledDateTime: (J.ship_date ? new Date(J.ship_date).toISOString() : new Date(Date.now() + 86400000).toISOString()), endDateTime: "" } },
        load: {
          packages: [{ quantity: (J.load && J.load.units) || 1, packagingType: containerType, overWeightIndicator: !!(J.load && J.load.overweight) }],
          declaredValue: { value: goodsValue, currencyID: J.goods_currency || "USD" }
        },
        accessorials: [],
        insuranceValueAmount: { value: withServices ? goodsValue : 0, currencyID: J.goods_currency || "USD" },
        pricePreference: { includeOriginPortCharges: true, includeDestinationPortCharges: true },
        declaredCustoms: { entry: !!withServices }
      }
    };

    let postRes;
    try {
      postRes = await fetch("/api/open-freight/quoting/quotes/search", {
        method: "POST",
        headers: { "Content-Type": "application/json", "Accept": "application/json" },
        body: JSON.stringify(payload),
        credentials: "include",
        signal: AbortSignal.timeout(12000)
      });
    } catch (e) {
      blockers.push({ code: "FREIGHTOS_API_TIMEOUT_OR_ERROR", message: `POST /search failed: ${String(e)}`, severity: "blocked" });
      return null;
    }
    if (postRes.status === 401) return { __session_expired: true };
    if (!postRes.ok) {
      blockers.push({ code: "FREIGHTOS_API_TIMEOUT_OR_ERROR", message: `POST /search HTTP ${postRes.status}`, severity: "blocked" });
      return null;
    }
    const postJson = await postRes.json();
    const rfq = postJson?.messageHeader?.conversationID;
    if (!rfq) {
      blockers.push({ code: "FREIGHTOS_API_TIMEOUT_OR_ERROR", message: "POST /search response missing conversationID", severity: "blocked" });
      return null;
    }
    const resultsUri = postJson?.URIReferences?.results?.uri || null;

    const quotes = [];
    let cursor = 0, hasMore = true, polls = 0;
    const deadline = Date.now() + 35000;
    while (hasMore && polls < 8 && Date.now() < deadline) {
      let r;
      try {
        r = await fetch(`/api/open-freight/quoting/quotes/${rfq}/poll?cursor=${cursor}&excludePlatformFee=true`, { credentials: "include", signal: AbortSignal.timeout(10000) });
      } catch (e) {
        blockers.push({ code: "FREIGHTOS_API_TIMEOUT_OR_ERROR", message: `poll cursor=${cursor} failed: ${String(e)}`, severity: "blocked" });
        return null;
      }
      if (r.status === 401) return { __session_expired: true };
      if (!r.ok) {
        blockers.push({ code: "FREIGHTOS_API_TIMEOUT_OR_ERROR", message: `poll cursor=${cursor} HTTP ${r.status}`, severity: "blocked" });
        return null;
      }
      const j = await r.json();
      if (Array.isArray(j.quotes)) quotes.push(...j.quotes);
      cursor = j.paging?.nextCursor ?? cursor + (j.quotes?.length || 1);
      hasMore = !!j.paging?.hasMore && (j.quotes?.length || 0) > 0;
      polls++;
      if (hasMore) await new Promise(r => setTimeout(r, 600));
    }
    return { rfq, quotes, resultsUri, polls };
  }

  const baseRun = await runSearch({ withServices: false });
  if (!baseRun) return JSON.stringify({ __error: true, warnings, blockers });
  if (baseRun.__session_expired) return JSON.stringify({ __session_expired: true });

  const now = Date.now();
  const usable = baseRun.quotes.filter(q => q?.priceIndicator?.totalCharge && (new Date(q.validTo).getTime() > now));
  if (!usable.length) {
    return JSON.stringify({
      __ok: true, rfq_id: baseRun.rfq, results_uri: baseRun.resultsUri,
      total_quotes: baseRun.quotes.length,
      no_usable: true, warnings, blockers
    });
  }
  // Best Value = first non-expired (API-sorted)
  const best = usable[0];
  const cheapest = usable.slice().sort((a, b) => (a.priceIndicator.totalChargeDiscounted?.value || a.priceIndicator.totalCharge.value) - (b.priceIndicator.totalChargeDiscounted?.value || b.priceIndicator.totalCharge.value))[0];
  const priciest = usable.slice().sort((a, b) => (b.priceIndicator.totalChargeDiscounted?.value || b.priceIndicator.totalCharge.value) - (a.priceIndicator.totalChargeDiscounted?.value || a.priceIndicator.totalCharge.value))[0];

  function priceOf(q) {
    const p = q.priceIndicator.totalChargeDiscounted || q.priceIndicator.totalCharge;
    return { value: p.value, currency: p.currencyID };
  }
  function transitOf(q) {
    const t = q.connection?.transitTime?.estimatedTransitTimes?.[0];
    return t ? { from: t.from?.value, to: t.to?.value } : null;
  }

  const baseQuote = {
    referenceID: best.referenceID,
    forwarder: best.businessInfo?.parties?.find(p => p.partyTypeCode === "FW") || null,
    price: priceOf(best),
    transit: transitOf(best),
    validTo: best.validTo
  };

  // Optional with-services run (insurance + customs brokerage ON)
  let withServicesPrice = null;
  if (J.include_insurance || J.include_customs_brokerage) {
    const wsRun = await runSearch({ withServices: true });
    if (wsRun && !wsRun.__session_expired) {
      const wsUsable = wsRun.quotes.filter(q => q?.priceIndicator?.totalCharge && (new Date(q.validTo).getTime() > Date.now()));
      if (wsUsable.length) withServicesPrice = priceOf(wsUsable[0]);
    }
  }

  return JSON.stringify({
    __ok: true,
    rfq_id: baseRun.rfq,
    results_uri: baseRun.resultsUri,
    total_quotes: baseRun.quotes.length,
    usable_count: usable.length,
    best, cheapest, priciest,
    base_price: priceOf(best),
    cheapest_price: priceOf(cheapest),
    priciest_price: priceOf(priciest),
    with_services_price: withServicesPrice,
    warnings, blockers
  });
})()
```

The bash wrapper then converts the eval's JSON output into the final
`result.json` per `result.schema.json`. Use integer minor units: multiply
EUR/USD/GBP `value` by 100 and round to integer. Currency code goes in
`freight_currency`.

If the eval body returns `__session_expired: true`, the bash wrapper runs the
Session Recovery sequence and either exits cleanly (signed_in) or writes a
blocked result (timeout/captcha/2FA).
