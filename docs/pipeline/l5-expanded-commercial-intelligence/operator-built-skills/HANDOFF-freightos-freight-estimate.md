# Build Handoff: `skills/landed-cost/freightos-freight-estimate/`

> **Audience:** The operator (or a coding agent the operator delegates to) building this skill personally before L5 step 09 bead audit runs.
>
> **Status when done:** Skill directory exists at `skills/landed-cost/freightos-freight-estimate/` with `SKILL.md` + `result.schema.json` (NO `checkpoint.schema.json` — landed-cost family doesn't use them), AND all 8 test-evidence files exist showing PASS for 4 canaries + 4 failure paths, AND TEST_REPORT.md shows verdict READY. Ready for the L5 pipeline's Phase 3 freight engine to invoke.
>
> **L5 manifest reference:** `docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml#recorded_decisions.external_api_account_state.operator_built_prerequisites[0]`
>
> **If you are a coding agent receiving this handoff (delegated by operator) — mandatory read order, no skipping:**
> 1. **READ THE README FIRST (MANDATORY, ~5 min):** `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/README.md`. This explains WHY both prerequisite skills exist, the L5 → L6 readiness chain that depends on them, the step 09 audit gate that will verify your work, and WHY partial-pass is unacceptable. Without this context you may build a skill that technically runs but misses the L5 use case, and you will burn iterations re-discovering the rationale that's already documented. **Do NOT proceed to step 2 until you've read the README in full.**
> 2. **Read this entire handoff** before any tool call. Don't skim. Skipping the Phase A discovery details or the Test/Validate/Fix loop semantics is the #1 cause of broken skills.
> 3. **Read both reference skill files end-to-end:** `skills/landed-cost/searates-freight-estimate/SKILL.md` (296 lines, gold-standard pattern) and `skills/landed-cost/hmrc-customs-classification/SKILL.md` (~181 lines, closer pattern for the result contract style). Do this before opening `agent-browser`.
> 4. **Verify the operator has Chrome ready** per Phase C.0.a checks BEFORE Phase A discovery. If the wrong Chrome is on port 9222, your discoveries will be invalid.
> 5. **The Python validator script + bash test loop scripts in Phase C have been audit-fixed but may still contain bugs you discover empirically.** If you find one, fix it in place AND note the fix in TEST_REPORT.md §"Quirks observed".
> 6. **Do not advance phases out of order.** Phase A → B → C → D is sequential. Phase C iterates internally (C.3 loops per fixture) but doesn't unlock D until C.7 all-checks-green.
>
> **Sibling handoff + assets (also mandatory references during the build):**
> - `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/HANDOFF-taric-cross-check.md` — the other operator-built skill; build either order; they're independent.
> - All Freightos UI screenshots (9 PNGs) are persisted at `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/screenshots/freightos/freightos-step-{01..09}.png` — referenced by step throughout this handoff. Open each screenshot in Preview (or via the Read tool if you're a coding agent without GUI access) when its corresponding step is referenced.

---

## Mission

Build a CDP-browser skill that uses the operator's persistent Chrome (already authenticated to Freightos as account "D") to fetch a real freight quote for a given origin/destination/load/goods specification, and returns a structured `result.json` matching the schema in this handoff. This skill is the **second freight provider** for L5's dual-source cross-check (the first being the existing `skills/landed-cost/searates-freight-estimate/`). When the two providers disagree by a large margin, the L5 freight engine emits `FREIGHT_MODE_LOW_CONFIDENCE`.

**This skill replaces the paid Freightos API.** Per L5 manifest decision 2026-05-08, all freight providers use CDP-browser scraping; no paid API procurement.

---

## Why this matters (context for the builder)

- L5 spec §2.2 mandates **dual-source cross-check** between Freightos and SeaRates. SeaRates is already in place; this skill completes the pair.
- L5 spec §3.5 introduces `FREIGHT_MODE_LOW_CONFIDENCE` as a customs-adjacent error code emitted when the two providers' estimates disagree materially. That code can't be emitted without TWO independent freight quotes — Freightos is the second one.
- The L5 Phase 3 freight engine at `packages/intelligence/src/logistics/freight-engine.ts` (to be built in Phase 3 beads) will invoke this skill in parallel with the SeaRates skill. The Phase 3 bead description will say "invoke `landed-cost/freightos-freight-estimate` and `landed-cost/searates-freight-estimate` in parallel; compare results; emit disagreement warning if delta > X bps."
- Without this skill, L5's Phase 3 beads cannot complete and the L5 → L6 readiness gate is at risk.

---

## Reference skill (READ THIS FIRST — it is the gold-standard pattern)

Read these three files end-to-end before starting:

1. `skills/landed-cost/searates-freight-estimate/SKILL.md` (296 lines) — your primary template. Note especially:
   - YAML frontmatter (`skill_id`, `requires_cdp: true`, `cdp_default_port: 9222`, `maxPages`, `maxScrollCycles`)
   - "Stage 11 Attempt Budget" — 5-step happy path bounded by daemon timeout
   - "Mandatory CDP Discipline" — `curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"` check before any browser command
   - "Mandatory CDP State + Captcha Reporting" table — `cdp_state` enum {connected, unreachable, captcha_blocked, session_expired} + `captcha_intercepted` boolean
   - "Captcha Simulation (Daemon-driven Test Hook)" — short-circuit on `job_parameters.simulate_captcha`
   - GraphQL command shape inside a single `agent-browser --cdp ${CDP_PORT} eval '...'` call
   - Result contract field names and the "freight_status: quoted | blocked" convention (no "estimated")
2. `skills/landed-cost/searates-freight-estimate/result.schema.json` — your output contract template. Has 16 properties + 5 required + the cdp_state enum.
3. `skills/landed-cost/searates-freight-estimate/checkpoint.schema.json` — wait, this file does NOT exist for SeaRates. The landed-cost skill family does not use checkpoints (interaction is bounded to a single attempt; no resume). Your skill follows the same convention: **no checkpoint.schema.json needed**.

Also read `AGENTS.md` lines 21-33 (Rule 2 wrappers) — not directly relevant to this skill's runtime, but it's how the daemon will invoke it.

---

## Site under scrape: `ship.freightos.com`

URL: `https://ship.freightos.com`

Login state: the operator's persistent CDP browser profile lives at `/Users/alinaaitova/.chrome-automation` (verified 2026-05-17, matches the canonical launch command in `docs/pipeline/stage-11-cdp-live-demos-continuation-handoff.md`). This is the profile SeaRates also uses. The profile should already be logged into Freightos as account "D" (top-right "D" avatar + Refer-a-friend button per screenshot freightos-step-01.png).

**Session-expired recovery:** when this skill detects a logged-out state (search API POST returns 401, DOM redirects to login modal, "D" avatar absent), it must FIRST attempt auto-signin via `skills/auth-signin/freightos/` (built as part of `docs/handoff-2026-05-17-cdp-auto-signin.md`, executed 2026-05-17 — the skill directory exists at handoff-build time). Only if auto-signin fails (Google captcha, 2FA, Cloudflare challenge, chooser empty) does the skill emit `FREIGHTOS_SESSION_EXPIRED` / `SESSION_REFRESH_FAILED` as a blocker requiring operator intervention. The reference pattern is identical to the Session Recovery sections recently added to `skills/landed-cost/searates-freight-estimate/SKILL.md` and the procontract-* skills — read one of those for the exact template.

### Phase A.8.1 — Session-expired auto-signin integration

During Phase A discovery (when you discover the session-expired signal per the original Phase A.8 step), ALSO empirically test the auto-signin recovery path:

1. With Freightos signed in normally, run a canary search → verify `freight_status: "quoted"`.
2. Sign OUT of Freightos in the persistent Chrome browser (click "D" avatar → Logout).
3. Re-run the canary search.
4. Confirm the skill: (a) detects the logged-out signal per the discovery in A.8, (b) invokes the auth-signin/freightos recovery flow (whose canonical click sequence is documented in `skills/auth-signin/freightos/SKILL.md` §Workflow), (c) verifies signin succeeded by checking the "D" avatar / Refer-a-friend signals, (d) the daemon retries this skill and the second invocation returns `freight_status: "quoted"`.
5. Document the recovery time in TEST_REPORT.md (expected: ~12-18 seconds for the auth-signin flow + ~10 seconds for the daemon retry pass + ~3-5 seconds for the actual quote fetch).

**Canonical launch command (if Chrome is not already running on 9222):**
```bash
'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' \
  --remote-debugging-port=9222 \
  '--remote-allow-origins=*' \
  --user-data-dir=/Users/alinaaitova/.chrome-automation \
  --profile-directory=Default \
  --no-first-run \
  --no-default-browser-check &
```
(Note the quoted `'--remote-allow-origins=*'` to prevent zsh glob expansion.)

---

## End-to-end UI walk (annotated screenshots in this handoff)

The 9 screenshots are persisted in the project at:
`docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/screenshots/freightos/`

| Step | Screenshot file | What it shows |
|---|---|---|
| 1 | `freightos-step-01.png` | Empty landing page with 4-field search bar |
| 2 | `freightos-step-02.png` | Origin popover — Type/Country/Address |
| 3 | `freightos-step-03.png` | Destination popover — Type/Country/Address |
| 4 | `freightos-step-04.png` | Load popover — Loose Cargo tab |
| 5 | `freightos-step-05.png` | Load popover — Containers tab (20'/40'/40'HC/45'HC) |
| 6 | `freightos-step-06.png` | Goods popover — value + currency + hazmat + readiness |
| 7 | `freightos-step-07.png` | Filled-out search (Tianjin→Portsmouth, 1×40', $10K, "Within 2 weeks") |
| 8 | `freightos-step-08.png` | Recommended Services page — Insurance + Customs brokerage toggles |
| 9 | `freightos-step-09.png` | Results page — Top 28 Quotes, Best value/Quickest/Cheapest/Greenest tabs |

Open them in Preview side-by-side with DevTools during Phase A discovery to identify selectors.

### Step 1 — Landing page (screenshot 1 in this handoff)

Empty 4-field search bar at top of page: **Origin | Destination | Load | Goods**. Each field opens a popover when clicked. URL: `ship.freightos.com`. The footer link "API Developer" exists — do NOT use the API; the CDP browser is the entire mechanism.

### Step 2 — Origin popover (screenshot 2)

When Origin is clicked, a popover opens with:
- **Type** dropdown: Port/Airport, Factory/Warehouse, Business address, Residential address. Default for the L5 use case: **Factory/Warehouse** (matches the procurement-intelligence model — suppliers are usually factories/warehouses).
- **Country** dropdown: country picker with flag (China shown in screenshot). Map from L5's `origin_country` (ISO 3166-1 alpha-2) → Freightos country option. Build the country code → option-text map by scrolling the dropdown ONCE during development and saving the mapping.
- **Address** field: city or zip code (free text). For L5: pass the city name from the supplier's listing data. Example: `Tianjin` (China).
- Right-arrow button (`→`) confirms and closes the popover. Or click outside.

### Step 3 — Destination popover (screenshot 3)

Similar layout to Origin but with MORE type options:
- **Type** dropdown: Port/Airport, Warehouse, Fulfillment center, Business address, Residential address, Last mile delivery warehousing.
  - Default for L5: **Warehouse** when the buyer is a public procurement org (their goods-in is a warehouse). Note this differs from Origin (Factory/Warehouse) — be careful with the type string mapping.
- **Country**: default `United Kingdom` (procurement-intelligence is UK-focused). Map ISO-2 → option.
- **Address**: UK city or postcode. Example: `Portsmouth` (UK).

### Step 4 — Load popover (screenshot 4 & 5)

Two tabs: **Loose Cargo** | **Containers**. The L5 use case overwhelmingly uses **Containers** (40' container is the most common Phase 11 DGU canary). Loose Cargo is also supported but less common.

**Containers tab (screenshot 5):**
- "**i** Containers can be shipped to or from a business address only if there is a loading dock." (Info banner — read but don't act on; just a UX hint.)
- **# of units** spinner (default 1).
- **Container type** segmented buttons: `20'` / `40'` / `40'HC` / `45'HC`. Default for L5 DGU canary: **40'**.
- **Overweight** checkbox (default unchecked).

**Loose Cargo tab (screenshot 4) — for non-container shipments:**
- Calculate by **unit type** or **total shipment** radio.
- **Package type** segmented buttons: Pallets / Boxes/Crates.
- **# of units** spinner.
- **Pallet type** dropdown (defaults to "Pallets (non specified size)").
- **Dimensions** (L × W × H per unit) with unit selector (CM default).
- **Weight** (per unit) with unit selector (KG default).
- **+ Add Another Load** button for multi-pallet shipments.

The "**Confirm**" button (greyed out until inputs are valid) saves the load and closes the popover.

For the skill's first cut: implement **Containers + 40'** as the happy path. Loose Cargo is a follow-up if needed. (Same scoping decision SeaRates made — its skill defaults to `container:ST40` for the Stage 11 DGU canary.)

### Step 5 — Goods popover (screenshot 6 & 7)

- **Goods value** numeric input + currency dropdown (default USD). For L5: pass the candidate's goods value in USD or convert from GBP using the FX skill. Example: `10000` (USD).
- **Personal Shipment or contains hazardous goods** checkbox (default unchecked). For L5 procurement use case: always unchecked (commercial, non-hazmat).
- **Are your goods ready?** dropdown — options visible in screenshot 7:
  - `Yes, my goods are ready`
  - `Will be ready within two weeks` (default for L5; matches procurement lead time)
  - `Will be ready in more than two weeks`
- **Confirm** / **Done** button.

### Step 6 — Trigger search

Once all 4 fields are filled (each shows a green ✓ check), click the magnifying-glass search button on the right edge of the bar. URL changes to `ship.freightos.com/search/services/{uuid}`.

### Step 7 — Recommended Services page (screenshot 8)

URL: `ship.freightos.com/search/services/{uuid}`. Top breadcrumb: **Search ✓ → Recommended Services (active) → Results → Booking → Verification**.

Two toggles:
- **Insurance** — "Yes - (covers the combined value of goods and initial freight costs up to $500k)". **Default for L5: ON** (matches the screenshot example and is the more conservative landed-cost assumption — insurance is part of the realistic landed-cost).
- **Customs brokerage** — "Yes - I need customs brokerage". **Default for L5: ON** (matches the screenshot — UK imports realistically need brokerage).

**IMPORTANT** for cross-check consistency: SeaRates does NOT include insurance/brokerage in its base freight quote. The Freightos skill should:
- Capture the freight quote WITHOUT insurance/brokerage for the apples-to-apples comparison vs SeaRates → return this as `freight_base_amount_minor`.
- Capture the WITH insurance/brokerage figure as additional context → return as a separate field `freight_with_services_amount_minor` for completeness.
- To do this: get a quote with both toggles ON, then a second quote with both toggles OFF, and return both numbers. OR: capture the breakdown by reading the per-line-item charges on the Results / Booking pages.

If only one quote can be obtained per attempt due to time budget, default to **toggles OFF** for the SeaRates-comparable number. Document the decision in the skill.

Click **Confirm Services & Get Results**.

### Step 8 — Results page (screenshot 9)

URL: `ship.freightos.com/results/{base64-id}`. The headline shows "Top 28 Quotes (54 in total)".

**Tabs (top of quote list):**
- **Best value** (default) — shown active in screenshot, "44-52 days · €4,403"
- **Quickest** — "41-53 days · €5,417"
- **Cheapest** — "77-90 days · €4,007"
- **Greenest** — "44-52 days · €4,403"

For L5 cross-check use case: **use the "Best value" tab's top quote** as `freight_base_amount_minor`. This matches SeaRates' "cheapest non-expired" semantics roughly (Best value optimizes price + transit time). Also capture:
- `freight_low_amount_minor` ← Cheapest tab's top quote
- `freight_high_amount_minor` ← Quickest tab's top quote (most expensive)
- `freight_base_amount_minor` ← Best value tab's top quote

**Quote card fields** (each visible in screenshot 9):
- Capacity badge ("Guaranteed capacity") and quality badge ("Top Logistics Provider") when present.
- Mode + transit estimate: "Ocean | Est. 58-71 days (49-56 days port-to-port)" — extract `transit_time_days` as the midpoint or upper bound.
- Estimated arrival + departure dates.
- Route line with port codes: e.g., `300000, Tianjin → CNTSN → GBSOU → PO2 8QD, Portsmouth` — capture the port pair (`CNTSN`, `GBSOU`) and inland origin/destination ZIPs.
- Provider name + star rating (e.g., "Seabay International Freight Forwarding Ltd · ★★★★½ (1881)" or "Think Prime · ★★★★½ (61)").
- Price block (right side): may show strikethrough original price + current price (e.g., ~~€5,024.69~~ → €4,981.69). Use the current (non-strikethrough) price. Currency is shown next to the price symbol (€ here; could be $ or £ depending on operator's Freightos currency setting in top-right corner).
- "Select" button (do NOT click — we just want the quote, not the booking).
- "Rate expires: <date> <time>" — capture as `quote_valid_until` (ISO-8601).

**Filters sidebar** (Price / Expiration date / Booking Type / Modes / Seller / Port to Port Carrier / Origin Port / Destination Port etc.) — NOT used by the skill; just be aware they exist.

---

## Architectural decisions for the skill

### Network strategy: prefer browser-native fetch over UI clicks

Like the SeaRates skill, the **first attempt should try to intercept or replay Freightos' internal API/GraphQL calls** rather than driving the DOM through every popover. The 7 UI steps above can be collapsed into:

1. CDP check (1 curl).
2. `agent-browser --cdp ${PORT} open https://ship.freightos.com` (or the search-URL with state pre-filled if discoverable).
3. ONE `agent-browser --cdp ${PORT} eval '...'` that fills the search via a single network POST to Freightos' search endpoint, polls for results, returns compact JSON.

To discover the endpoint:
- During build, open Chrome DevTools Network tab on `ship.freightos.com`, fill the form manually, and observe the requests. Look for POSTs to `ship.freightos.com/api/...` or `services-api.freightos.com/...` carrying the origin/destination/load/goods payload.
- Capture the request shape (headers including any CSRF/auth token, body JSON). The token is likely a JWT in `Authorization: Bearer ...` header or a session cookie.
- Replicate inside `eval` with `fetch()`, similar to SeaRates' `rates.searates.com/graphql` pattern.

**Fallback (if no clean API exists):** drive the DOM. Use 4 popover clicks → fill → confirm → submit → poll for results page → parse quote cards. Slower but more robust. SeaRates' skill body has a fallback note: "If `rates(...)` times out or returns a network error, write `result.json` immediately with `freight_status:'blocked'`... Do not start fallback UI exploration inside this bounded attempt." — adopt the same conservative bounded-attempt discipline.

### Bounded-attempt discipline (CRITICAL — adopt SeaRates conventions verbatim)

From SeaRates SKILL.md lines 19-46, copy these patterns:

```
The daemon kills each Freightos attempt after its configured per-attempt budget
(180 seconds by default, env-overridable). Complete the happy path in one pass:

1. Check CDP.
2. Open the Freightos search URL.
3. Read or refresh session token / cookies if needed.
4. Run the compact search-API call (or single eval that orchestrates form-fill + submit + result-poll).
5. Write `result.json`.

The result schema is fully summarized in this SKILL.md. Do NOT run setup or
discovery commands: no `pwd`, `ls`, `rg`, `find`, `command -v`, schema printing,
browser CLI help output, accessibility snapshots, title checks, broad page
inspection, repository searches, or exploratory schema discovery. These commands
already consumed an entire daemon attempt in live SeaRates validation.

Use at most three shell commands total in the success path:
1. `curl` CDP check.
2. `agent-browser --cdp ${CDP_PORT} open https://ship.freightos.com/...`.
3. `agent-browser --cdp ${CDP_PORT} eval '...'` for token/cookie discovery + search call.

After command 3, immediately write `result.json`. If a required value is
unavailable after the fast path, immediately write a structured `blocked` result
with `cdp_state`, `captcha_intercepted`, and a Stage11Issue-shaped blocker.
Never leave the workdir without `result.json`.
```

### CDP state + captcha reporting (verbatim from SeaRates pattern)

Every result MUST set both `captcha_intercepted` (boolean) and `cdp_state` (one of `connected`, `unreachable`, `captcha_blocked`, `session_expired`). Map Freightos-specific failures:

| Observation | `cdp_state` | `captcha_intercepted` | `freight_status` |
|---|---|---|---|
| Normal Freightos response with a usable quote | `connected` | `false` | `quoted` |
| Cloudflare/captcha bot wall | `captcha_blocked` | `true` | `blocked` |
| `localhost:${CDP_PORT}/json/version` returns non-2xx | `unreachable` | `false` | `blocked` |
| CDP reachable but Freightos session expired (e.g., 401/302 to login) | `session_expired` | `false` | `blocked` |

Blocker codes specific to Freightos (use these in the `blockers` array):
- `CDP_CONNECT_FAILED` — CDP itself down (same as SeaRates)
- `CDP_CAPTCHA_INTERCEPTION` — Cloudflare or in-app captcha
- `FREIGHTOS_SESSION_EXPIRED` — operator's Freightos session expired; needs manual re-login
- `FREIGHTOS_API_TIMEOUT_OR_ERROR` — internal search API timeout / error
- `FREIGHTOS_NO_USABLE_QUOTE` — search returned 0 non-expired quotes
- `FREIGHTOS_ORIGIN_DESTINATION_UNSUPPORTED` — Freightos refuses the route (e.g., country pair not served)
- `FREIGHTOS_LOAD_INVALID` — load specification rejected (e.g., container can't go to residential address per the info banner)

### Captcha simulation hook (verbatim from SeaRates)

If `job_parameters.simulate_captcha` is true, short-circuit immediately without driving the browser:

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

---

## Target directory structure

```
skills/landed-cost/freightos-freight-estimate/
├── SKILL.md                  # Full skill spec (template below — operator fills in)
├── result.schema.json        # Output contract (template below)
└── (NO checkpoint.schema.json — landed-cost family doesn't use checkpoints)
```

---

## SKILL.md template (start here; modify as needed during build)

```markdown
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
Use Freightos Marketplace (ship.freightos.com) in the operator's persistent CDP
browser to estimate sea/air/express freight for an already-sized shipment.
Returns a `freight_base_amount_minor` plus low/high band. This is quote lookup
only — no booking, no payment, no account changes.

This skill is the SECOND freight provider in L5's dual-source cross-check
(SeaRates is the first). When the two providers' base estimates disagree
materially, the L5 freight engine emits `FREIGHT_MODE_LOW_CONFIDENCE`.

## Inputs
The daemon supplies one object containing:
- `process_id`, `notice_id`, `product_candidate_id`, `pricing_assessment_id`, `selected_quote_id`
- `origin_country` (ISO-2, e.g., `CN`)
- `origin_address` (city or postal code, e.g., `Tianjin`)
- `origin_type` (`port_airport` | `factory_warehouse` | `business_address` | `residential_address`)
- `destination_country` (ISO-2, e.g., `GB`)
- `destination_address` (city or postal code, e.g., `Portsmouth`)
- `destination_type` (`port_airport` | `warehouse` | `fulfillment_center` | `business_address` | `residential_address` | `last_mile_delivery_warehousing`)
- `load` (object):
  - `mode`: `container` | `loose_cargo`
  - For container: `container_type` (`20'` | `40'` | `40HC` | `45HC`), `units` (int), `overweight` (bool)
  - For loose cargo: `package_type`, `units`, `pallet_type`, `dimensions_cm` {l,w,h}, `weight_kg`
- `goods_value_minor` + `goods_currency` (e.g., 1000000 USD pence)
- `is_hazmat` (bool — almost always false for L5)
- `readiness_timeframe` (`ready_now` | `within_two_weeks` | `more_than_two_weeks`)
- `ship_date` (ISO date — informational; Freightos doesn't take this as input)
- `include_insurance` (bool, default false for SeaRates-comparable base)
- `include_customs_brokerage` (bool, default false for SeaRates-comparable base)
- optional `simulate_captcha` (bool, daemon test hook)

## Stage 11 / L5 Attempt Budget

The daemon kills each Freightos attempt after its configured per-attempt budget
(180 seconds by default, env-overridable). Complete the happy path in one pass:

1. Check CDP.
2. Open Freightos search URL (or replay prior search state if available).
3. Read or refresh session token / cookies.
4. Run the compact internal-API call (or single eval that orchestrates
   form-fill + submit + result-poll) for Best Value / Cheapest / Quickest tabs.
5. Write `result.json`.

The result schema is fully summarized in this SKILL.md. Do not run setup or
discovery commands: no `pwd`, `ls`, `rg`, `find`, `command -v`, schema printing,
browser CLI help output, accessibility snapshots, title checks, broad page
inspection, repository searches, or exploratory schema discovery.

Use at most three shell commands total in the success path:
1. `curl` CDP check.
2. `agent-browser --cdp ${CDP_PORT} open https://ship.freightos.com`.
3. `agent-browser --cdp ${CDP_PORT} eval '...'` for token discovery + search API call.

After command 3, immediately write `result.json`. If a required value is
unavailable after the fast path, immediately write a structured `blocked` result
with `cdp_state`, `captcha_intercepted`, and a Stage11Issue-shaped blocker.
Never leave the workdir without `result.json`. The `evidence` field should be an
array of objects, even when there is only one evidence object.

## Mandatory CDP Discipline
Before navigation:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# Ensure CDP is up. The daemon's pre-flight gate SHOULD have done this already,
# but defense in depth: bin/ensure-cdp-9222 is idempotent and fast (<1s no-op
# when CDP is already up; otherwise launches Chrome with the operator's
# persistent profile at /Users/alinaaitova/.chrome-automation and polls up to
# 15s). Returns non-zero only if launch genuinely failed. Absolute path because
# the skill subprocess cwd is the per-skill workdir, not the project root.
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222

# Defensive reachability check. If this fails, follow the "If CDP is
# unreachable" guidance below: write a blocked freight_status: "blocked" result
# with cdp_state: "unreachable", captcha_intercepted: false, and blocker
# CDP_CONNECT_FAILED.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, emit `freight_status: "blocked"`, `cdp_state: "unreachable"`,
`captcha_intercepted: false`, with blocker code `CDP_CONNECT_FAILED`. Do not launch
a fresh browser. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

## Session Recovery (REQUIRED — per docs/handoff-2026-05-17-cdp-auto-signin.md)

After building this SKILL.md per the Freightos handoff Phase B, the agent MUST also add a "Session Recovery" subsection to it, per the pattern in `docs/handoff-2026-05-17-cdp-auto-signin.md` §"Step 4: Wire auto-signin into the 8 affected production skills". When applied to Freightos:

- **Detection signal:** the Freightos search API POST returns HTTP 401 OR redirects to `/login` OR `localStorage` lacks the Freightos auth token (signal discovered during Phase A step 8 — record empirically).
- **Recovery action:** run the inline `agent-browser` click sequence documented in `skills/auth-signin/freightos/SKILL.md` §Workflow (this skill must exist on disk per Phase A step 9 — built per the auto-signin handoff). Copy the click sequence verbatim into this Session Recovery section.
- **Verification:** after the sequence, eval `document.querySelector('<freightos-avatar-D-selector>') !== null` to confirm "D" avatar is visible top-right.
- **On success:** exit cleanly without writing result.json — daemon's existing retry policy re-invokes Freightos skill, second invocation completes the search normally.
- **On failure:** write result.json with `freight_status: "blocked"`, `cdp_state: "session_expired"`, blocker code `SESSION_REFRESH_FAILED` with operator-action guidance (2FA, captcha, empty Google chooser, etc.).

The full template + bash code lives in the auto-signin handoff. Cross-reference there during Freightos build; don't reinvent.

## Mandatory CDP State + Captcha Reporting

Every result MUST set both `captcha_intercepted` (boolean) and `cdp_state` (one
of `connected`, `unreachable`, `captcha_blocked`, `session_expired`).

| Observation | `cdp_state` | `captcha_intercepted` | `freight_status` |
|---|---|---|---|
| Normal Freightos response with a usable quote | `connected` | `false` | `quoted` |
| Cloudflare / in-app captcha | `captcha_blocked` | `true` | `blocked` |
| `localhost:${CDP_PORT}/json/version` returns non-2xx | `unreachable` | `false` | `blocked` |
| Freightos session expired (401/302 to login) | `session_expired` | `false` | `blocked` |

When `cdp_state` is anything other than `connected`, emit IMMEDIATELY (do not
retry inside the skill — the daemon's retry loop owns retries). Include a single
Stage11Issue-shaped blocker in `blockers` (codes: `CDP_CAPTCHA_INTERCEPTION`,
`CDP_CONNECT_FAILED`, `FREIGHTOS_SESSION_EXPIRED`).

## Captcha Simulation (Daemon-driven Test Hook)

If `job_parameters.simulate_captcha` is `true`, short-circuit immediately without
driving the browser:

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

1. Prefer Freightos' internal search API path discovered during build. Do not
   inspect application bundles or run exploratory browser commands before
   trying it.
2. Open `https://ship.freightos.com` through CDP (or directly a saved
   search-state URL if discoverable as a deep link).
3. In a single `agent-browser --cdp ${CDP_PORT} eval '...'` call, read the
   Freightos session token from `localStorage` (key name TBD during build —
   inspect during DevTools discovery; likely `auth_token` or similar JWT) or
   from `document.cookie`. If missing, fetch the platform-token endpoint (TBD
   during build — likely `https://ship.freightos.com/api/auth/...`).
4. In the same eval call, build a compact search request against the discovered
   Freightos internal API endpoint with these parameters:
   - `origin: { type, country, address }`
   - `destination: { type, country, address }`
   - `load: { mode, ...container-or-loose-fields }`
   - `goods: { value_minor, currency, is_hazmat, readiness }`
   - `services: { insurance: false, customs_brokerage: false }` (for the SeaRates-comparable base; capture a second request with both true if attempt budget permits)
5. Parse the response. Extract:
   - Best Value top quote → `freight_base_amount_minor` + currency.
   - Cheapest top quote → `freight_low_amount_minor`.
   - Quickest top quote → `freight_high_amount_minor`.
   - Selected quote's transit time → `transit_time_days` (midpoint of "Est. X-Y days port-to-port").
   - Selected quote's provider name + star rating + review count.
   - Origin/destination port codes (e.g., `CNTSN`, `GBSOU`).
   - Rate expiration date → `quote_valid_until` (ISO-8601).
6. Do NOT click "Select" on any quote (that initiates booking flow). Do NOT
   change Insurance / Customs brokerage toggles after the first capture (it
   re-triggers the search and burns the attempt budget).
7. Capture price, currency, route, mode/container basis, transit time, source
   URL, raw evidence text, caveats.

### Compact API call shape (replace with discovered endpoint during build)

```js
(async () => {
  // 1. Get session token from localStorage / cookie
  let token = localStorage.getItem("auth_token"); // TBD: replace with actual key during build
  if (!token) {
    // TBD: fetch token from auth endpoint discovered during build
    return JSON.stringify({ freightos_error: "FREIGHTOS_SESSION_EXPIRED", message: "no token in localStorage; manual re-login required" });
  }

  // 2. Build search payload from job_parameters
  const payload = {
    origin: {
      type: "factory_warehouse",
      country: "CN",
      address: "Tianjin"
    },
    destination: {
      type: "warehouse",
      country: "GB",
      address: "Portsmouth"
    },
    load: { mode: "container", container_type: "40", units: 1, overweight: false },
    goods: { value_minor: 1000000, currency: "USD", is_hazmat: false, readiness: "within_two_weeks" },
    services: { insurance: false, customs_brokerage: false }
  };

  // 3. POST to Freightos internal search API (endpoint TBD during build)
  const response = await fetch("https://ship.freightos.com/api/search/quotes", {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "authorization": `Bearer ${token}`
    },
    body: JSON.stringify(payload),
    signal: AbortSignal.timeout(15000)
  });
  const result = await response.json();

  // 4. Extract Best Value, Cheapest, Quickest top quotes
  const bestValue = result.tabs?.best_value?.quotes?.[0];
  const cheapest = result.tabs?.cheapest?.quotes?.[0];
  const quickest = result.tabs?.quickest?.quotes?.[0];

  if (!bestValue) {
    return JSON.stringify({
      freightos_error: "FREIGHTOS_NO_USABLE_QUOTE",
      cdp_state: "connected",
      captcha_intercepted: false,
      total_quotes: result.total_quotes || 0
    });
  }

  return JSON.stringify({
    cdp_state: "connected",
    captcha_intercepted: false,
    total_quotes: result.total_quotes,
    best_value: { price: bestValue.price, currency: bestValue.currency, transit_days: bestValue.transit_time_days, provider: bestValue.provider_name, ports: bestValue.ports, expires: bestValue.expires_at },
    cheapest: cheapest ? { price: cheapest.price, currency: cheapest.currency } : null,
    quickest: quickest ? { price: quickest.price, currency: quickest.currency } : null
  });
})()
```

This is a STRAW MAN for build discovery. Replace endpoint path, payload schema,
header names, and field names with the actual Freightos internals discovered
via DevTools Network tab during the build session.

For the final `result.json`, include the selected best-value quote as an object
inside the `evidence` array, including `total_price`, `currency`, `transit_days`,
provider/carrier when present, route/query URL, port codes, and the total quote
count.

If the search API call times out or returns a network error, write `result.json`
immediately with `freight_status:"blocked"`, `cdp_state:"connected"`,
`captcha_intercepted:false`, and blocker code `FREIGHTOS_API_TIMEOUT_OR_ERROR`.
Do not start fallback UI exploration inside this bounded attempt.

When Freightos returns a concrete non-expired amount, the successful result fields
are exactly:

```json
"freight_status": "quoted",
"cdp_state": "connected",
"captcha_intercepted": false
```

Do not emit `freight_status: "estimated"` or any other success status. `blocked`
is the only non-success status accepted by the daemon.

If Freightos shows only expired estimates, set `quote_expired: true`, add warning
`FREIGHTOS_PRICE_EXPIRED`, and do not present the quote as usable freight evidence.

## Safety
Do not log in/out, contact carriers, book shipments, create orders, change
account settings (currency, language), modify the Recommended Services toggles
after the first capture, or write server-side state.

## Output
Write the final JSON object to `result.json` in the current working directory,
matching `result.schema.json`. `evidence`, `warnings`, and `blockers` must be
arrays; `warnings` and `blockers` entries must be objects with `code`, `message`,
and `severity`. Never emit bare string warning codes or omit severity. Also keep
the final response to the daemon as the same raw JSON object; do not wrap it
in Markdown.
```

---

## result.schema.json template (start here)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": [
    "product_candidate_id",
    "selected_quote_id",
    "freight_status",
    "captcha_intercepted",
    "cdp_state",
    "warnings",
    "blockers"
  ],
  "properties": {
    "source": { "type": "string" },
    "product_candidate_id": { "type": "string" },
    "selected_quote_id": { "type": "string" },
    "freight_status": { "type": "string" },
    "captcha_intercepted": { "type": "boolean", "default": false },
    "cdp_state": {
      "type": "string",
      "enum": ["connected", "unreachable", "captcha_blocked", "session_expired"],
      "default": "connected"
    },
    "freight_currency": { "type": ["string", "null"] },
    "freight_base_amount_minor": { "type": ["integer", "null"] },
    "freight_low_amount_minor": { "type": ["integer", "null"] },
    "freight_high_amount_minor": { "type": ["integer", "null"] },
    "freight_with_services_amount_minor": { "type": ["integer", "null"], "description": "Best-value quote WITH insurance + customs brokerage toggles ON, if captured. Optional second capture; may be null if attempt budget didn't permit." },
    "route": { "type": ["object", "null"], "description": "Object with origin_port_code, destination_port_code, inland_origin, inland_destination" },
    "transit_time_days": { "type": ["integer", "null"] },
    "transit_time_days_min": { "type": ["integer", "null"] },
    "transit_time_days_max": { "type": ["integer", "null"] },
    "provider": { "type": ["object", "null"], "description": "Object with name, rating, review_count, badges (top_logistics_provider, guaranteed_capacity)" },
    "quote_valid_until": { "type": ["string", "null"], "description": "ISO-8601 expiration of the captured rate" },
    "quote_expired": { "type": "boolean" },
    "total_quotes_available": { "type": ["integer", "null"] },
    "tab_used": { "type": ["string", "null"], "enum": ["best_value", "cheapest", "quickest", "greenest", null] },
    "source_url": { "type": ["string", "null"] },
    "evidence": { "type": ["array", "object", "null"] },
    "warnings": { "type": "array" },
    "blockers": { "type": "array" }
  }
}
```

---

## Step-by-step build guide

### Phase A: Discovery (~60-90 min, one-off)

1. Open Chrome with the operator's persistent CDP profile that's already logged into Freightos. Open DevTools Network tab; filter for `XHR` and `Fetch`.
2. Manually fill the search form using the example: Tianjin Factory/Warehouse → Portsmouth Warehouse, 1×40' container, $10,000 USD, "Within 2 weeks", Insurance OFF, Customs brokerage OFF.
3. Click search. Observe network requests. Note:
   - The actual search-API endpoint (POST URL).
   - Request headers (especially `Authorization` / cookies).
   - Request body JSON shape.
   - Response JSON shape — how the Best Value / Cheapest / Quickest / Greenest tabs are structured.
4. Note the auth token mechanism. Open `localStorage` in DevTools Application tab — find the Freightos session token's storage key. Also inspect `document.cookie` — note any session cookie names.
5. Note the URL pattern for the recommended-services step and the results step. The results URL has a base64-encoded ID — the API likely returns this ID for direct deep-linking.
6. **Container type input mapping:** the Freightos UI shows `20'`, `40'`, `40'HC`, `45'HC` with apostrophes. The skill's input contract uses unquoted strings (`"20"`, `"40"`, `"40HC"`, `"45HC"`). Document the UI ↔ input mapping in your eval body comments.
7. **Toggles-ON dual-quote decision (do this before C.1):** test the with-services scenario:
   - Re-run the search with Insurance ON, Customs brokerage ON.
   - Time how long the response takes vs. the toggles-OFF baseline.
   - **Decision rule:** if the delta is **< 5 seconds** AND the toggles-ON request returns within the 180s skill budget when combined with the OFF request, code the eval to do BOTH searches in sequence and return `freight_with_services_amount_minor` alongside `freight_base_amount_minor`. Otherwise, code the eval to do ONE search (whose toggle state matches `job_parameters.include_insurance` + `include_customs_brokerage`) and leave the other field null. **Record your empirical timing in TEST_REPORT.md** so the L5 freight engine knows whether dual-capture is reliable.
8. **Session-expiration signal discovery (do this before C.4 fp-03 — it's the empirical answer to "how does the skill detect logged-out"):**
   - Click "Logout" in the Freightos UI (top-right avatar menu).
   - With DevTools Network tab still recording, attempt the same search again.
   - Observe what the search-API endpoint returns: HTTP 401? 302 redirect to `/login`? An error JSON body with a specific code? Empty response?
   - Also check `localStorage` after logout — does the session token key disappear, or does it persist but become invalid?
   - **The signal you discover here becomes the explicit `cdp_state: "session_expired"` detection logic in the eval.** Document it clearly in the eval comments + TEST_REPORT.md. Example: "Logout removes `localStorage.auth_token`; missing token = session_expired" OR "Logout returns HTTP 401 with `{error: 'unauthorized'}` from /api/search/quotes".
   - Log back in before C.1 — OR, after the auto-signin handoff is executed (`docs/handoff-2026-05-17-cdp-auto-signin.md`), trigger `skills/auth-signin/freightos/` to verify automated recovery works.

9. **Sign-in flow discovery for `auth-signin/freightos/` (PREREQUISITE per `docs/handoff-2026-05-17-cdp-auto-signin.md`):**
   - With Chrome logged out of Freightos, manually click "Sign in" / "Login" button. Note its selector (likely top-right header).
   - Observe the resulting auth modal/page. Click "Continue with Google". Note its selector (typically a button labelled "Continue with Google" inside the Freightos auth modal).
   - Observe the Google account chooser at `accounts.google.com/...`. Verify `Dieter Danicek (dieter.x.s.danicek@gmail.com)` appears in the list. Note the DOM selector for that specific account row.
   - Click Dieter Danicek. Observe redirect back to `ship.freightos.com`. Verify avatar "D" returns top-right.
   - **Output:** these three selectors + redirect target become the body of the `skills/auth-signin/freightos/` skill (built per the auto-signin handoff). Document them in BOTH this handoff's TEST_REPORT AND the future auth-signin/freightos SKILL.md.

10. Document the findings inline in the SKILL.md you're building (replace the `TBD during build` placeholders).

### Phase B: Build the skill files (~60-90 min)

1. `mkdir -p skills/landed-cost/freightos-freight-estimate`.
2. Create `SKILL.md` from the template above. Fill in the discovered endpoint paths, auth key names, payload shape, response paths.
3. Create `result.schema.json` from the template above. Adjust property names/types to match your actual extraction.
4. (NO checkpoint.schema.json — landed-cost family doesn't use them.)

### Phase C: Test / Validate / Fix Loop — empirical CDP execution (~2-4 hours iterative)

> **This is THE most important phase.** Building a SKILL.md from a template is the easy part; making the skill actually work end-to-end against a live Freightos site through CDP is where 80% of the real work happens. **Do not skip iterations. Do not ship until 100% of the test matrix below passes.**
>
> **Mindset:** treat the skill like a test-driven feature. Each fixture is a test case. Each failure is a debugging cycle. Each green run is one row of evidence in the final `TEST_REPORT.md`. The loop exits only when every row is green AND the failure-path matrix is fully covered.

#### C.0 — Set up the test environment (one-time, ~15 min)

##### C.0.a — Verify you're using the RIGHT Chrome (do NOT skip)

The skill assumes a SPECIFIC persistent Chrome (the same one SeaRates uses) is running on port 9222, with the operator already logged into Freightos. If a different Chrome (or no Chrome) is on 9222, tests will produce garbage. Verify BEFORE doing anything else:

```bash
# 0. Auto-launch (if needed). Idempotent: <1s no-op when CDP is already up;
#    launches Chrome with the operator's /Users/alinaaitova/.chrome-automation
#    profile and polls up to 15s if not. Exit non-zero only on genuine failure.
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222

# 1. CDP is up
curl -sfS -m 3 "http://localhost:9222/json/version" | jq '{browser: .Browser, userAgent: .["User-Agent"], debuggerUrl: .webSocketDebuggerUrl}'
# Expected: Browser like "Chrome/140.x.x.x" or newer, debuggerUrl starts with ws://localhost:9222
# If empty/error after the helper succeeded: rare port hijack / wrong Chrome on 9222.
# Fix the persistent profile state before proceeding.

# 2. List open tabs — must show at least one Freightos tab
curl -sfS "http://localhost:9222/json/list" | jq '[.[] | select(.url | contains("freightos.com")) | {title, url}]'
# If empty: open https://ship.freightos.com manually in the persistent Chrome before continuing.

# 3. Confirm the operator's login: visually open the Freightos tab and verify the "D" avatar
#    is visible top-right (not a "Login" / "Sign in" button). The skill's session-expired
#    failure-path test will exercise the negative case later — for now we need a logged-in baseline.
```

If any of those three checks fails, **stop** — fix the Chrome/login state before proceeding. Running canaries against a fresh non-persistent Chrome or a logged-out session will produce false failures that look like skill bugs.

##### C.0.b — Working directory + schema validator

```bash
# Working test directory
mkdir -p /tmp/freightos-build-test
cd /tmp/freightos-build-test

# Copy the schema for validation
cp /Users/alinaaitova/Desktop/Testproject1/skills/landed-cost/freightos-freight-estimate/result.schema.json .

# Install a JSON Schema validator (PICK ONE — handoff and TEST_REPORT must be consistent):
# Option A (preferred — Bun-compatible, matches project's bun stack):
bun add -g ajv-cli ajv-formats
ajv --version   # confirm install; record this version in TEST_REPORT.md
# Option B (Python fallback if you don't want a global Bun install):
python3 -m pip install --user jsonschema
python3 -c "import jsonschema; print(jsonschema.__version__)"   # record in TEST_REPORT.md
```

Record in TEST_REPORT.md "Environment" section: bash version, jq version, ajv-cli OR jsonschema version, Chrome version (from curl /json/version), agent-browser version. This makes the test evidence reproducible by anyone re-running C.3 later.

##### C.0.c — Test-evidence directory + eval scratchpad

```bash
# Audit evidence directory
mkdir -p /Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/freightos-freight-estimate

# Local scratchpad for the WIP eval JS body (keep separate from SKILL.md during iteration)
# This is where you iterate on the JS body discovered in Phase A — paste/edit here,
# source from here in C.3 commands, then copy the FINAL working version into SKILL.md.
touch /tmp/freightos-build-test/eval.js
```

**Why a separate eval.js:** during the Test/Validate/Fix loop you'll edit the JS body many times. Keeping it in a file (vs typing into a long bash command each time) means you can re-run with `agent-browser --cdp 9222 eval "$(cat eval.js)"` and just re-edit the file between iterations.

#### C.1 — Define the canary input matrix (one-time, ~15 min)

Write `canary.inputs.jsonl` with **at least these 4 test cases**. Each is one line of JSON (JSONL convention from `tests/fixtures/calibration/`). Save it to:
`docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/freightos-freight-estimate/canary.inputs.jsonl`

```jsonl
{"id":"canary-01-dgu-tianjin-portsmouth","product_candidate_id":"test-dgu-001","selected_quote_id":"q-001","origin_country":"CN","origin_address":"Tianjin","origin_type":"factory_warehouse","destination_country":"GB","destination_address":"Portsmouth","destination_type":"warehouse","load":{"mode":"container","container_type":"40","units":1,"overweight":false},"goods_value_minor":1000000,"goods_currency":"USD","is_hazmat":false,"readiness_timeframe":"within_two_weeks","include_insurance":false,"include_customs_brokerage":false}
{"id":"canary-02-shanghai-felixstowe-20ft","product_candidate_id":"test-fasteners-001","selected_quote_id":"q-002","origin_country":"CN","origin_address":"Shanghai","origin_type":"factory_warehouse","destination_country":"GB","destination_address":"Felixstowe","destination_type":"port_airport","load":{"mode":"container","container_type":"20","units":1,"overweight":false},"goods_value_minor":500000,"goods_currency":"USD","is_hazmat":false,"readiness_timeframe":"ready_now","include_insurance":false,"include_customs_brokerage":false}
{"id":"canary-03-loose-cargo-pallets","product_candidate_id":"test-gloves-001","selected_quote_id":"q-003","origin_country":"CN","origin_address":"Shenzhen","origin_type":"factory_warehouse","destination_country":"GB","destination_address":"London","destination_type":"warehouse","load":{"mode":"loose_cargo","package_type":"pallets","units":2,"pallet_type":"non_specified","dimensions_cm":{"l":120,"w":100,"h":140},"weight_kg":350},"goods_value_minor":250000,"goods_currency":"USD","is_hazmat":false,"readiness_timeframe":"within_two_weeks","include_insurance":false,"include_customs_brokerage":false}
{"id":"canary-04-with-services","product_candidate_id":"test-dgu-002","selected_quote_id":"q-004","origin_country":"CN","origin_address":"Tianjin","origin_type":"factory_warehouse","destination_country":"GB","destination_address":"Portsmouth","destination_type":"warehouse","load":{"mode":"container","container_type":"40","units":1,"overweight":false},"goods_value_minor":1000000,"goods_currency":"USD","is_hazmat":false,"readiness_timeframe":"within_two_weeks","include_insurance":true,"include_customs_brokerage":true}
```

Write `golden.assertions.yaml` — the **expected outcomes** for each canary. Save next to canary.inputs.jsonl:

```yaml
# Golden assertions are tolerance-based — exact freight prices fluctuate.
# These define the SHAPE and BALLPARK ranges that prove the skill works end-to-end.

canary-01-dgu-tianjin-portsmouth:
  freight_status: quoted
  cdp_state: connected
  captcha_intercepted: false
  freight_base_amount_minor:
    min: 200000      # €2,000 floor — Tianjin→Portsmouth 40' should never be below this
    max: 1500000     # €15,000 ceiling — anything above suggests a parsing bug
  freight_currency:
    one_of: ["EUR", "USD", "GBP"]
  transit_time_days:
    min: 30
    max: 90
  route:
    must_contain_port_codes: ["CN", "GB"]   # Origin port starts with CN, destination with GB
  quote_expired: false
  blockers_length: 0

canary-02-shanghai-felixstowe-20ft:
  freight_status: quoted
  cdp_state: connected
  freight_base_amount_minor:
    min: 100000
    max: 1000000     # 20' container is generally cheaper than 40'
  blockers_length: 0

canary-03-loose-cargo-pallets:
  freight_status: quoted
  cdp_state: connected
  blockers_length: 0   # LCL/pallet should also produce a quote

canary-04-with-services:
  freight_status: quoted
  cdp_state: connected
  # When insurance + brokerage are ON, expect a higher number than canary-01
  freight_with_services_amount_minor:
    must_exist: true
  freight_base_amount_minor:
    must_exist: true
  # Sanity: with-services should be GREATER than without
  invariant: freight_with_services_amount_minor > freight_base_amount_minor
```

#### C.2 — Define the failure-path matrix (one-time, ~5 min)

These tests verify the skill's error handling. Save as `failure-paths.jsonl`:

```jsonl
{"id":"fp-01-captcha-sim","scenario":"Pass simulate_captcha=true; verify short-circuit","input_override":{"simulate_captcha":true},"expect":{"freight_status":"blocked","cdp_state":"captcha_blocked","captcha_intercepted":true,"blockers_length":1,"blocker_codes":["CDP_CAPTCHA_INTERCEPTION"]}}
{"id":"fp-02-cdp-down","scenario":"Stop CDP browser before running; verify CDP_CONNECT_FAILED","setup":"Stop Chrome / kill --remote-debugging-port=9222 process","input_override":{},"expect":{"freight_status":"blocked","cdp_state":"unreachable","captcha_intercepted":false,"blockers_length":1,"blocker_codes":["CDP_CONNECT_FAILED"]}}
{"id":"fp-03-session-expired","scenario":"Log out of Freightos in the browser; run skill; verify FREIGHTOS_SESSION_EXPIRED","setup":"Click avatar > Logout in the Freightos browser tab","input_override":{},"expect":{"freight_status":"blocked","cdp_state":"session_expired","captcha_intercepted":false,"blockers_length":1,"blocker_codes":["FREIGHTOS_SESSION_EXPIRED"]}}
{"id":"fp-04-invalid-country","scenario":"Pass origin_country=ZZ (not a valid ISO-2); verify graceful failure","input_override":{"origin_country":"ZZ"},"expect":{"freight_status":"blocked","blocker_codes":["FREIGHTOS_FORM_REJECTED","FREIGHTOS_ORIGIN_DESTINATION_UNSUPPORTED"],"any_blocker_code_matches":true}}
```

#### C.3 — Execute the loop (iterate until 100% pass)

For EACH canary in canary.inputs.jsonl, run this cycle:

```bash
EVID_DIR=/Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/freightos-freight-estimate
CANARY_ID=canary-01-dgu-tianjin-portsmouth   # change per iteration

cd /tmp/freightos-build-test
rm -f result.json   # start fresh

# 1. Extract the canary input
jq -c --arg id "$CANARY_ID" 'select(.id == $id)' "$EVID_DIR/canary.inputs.jsonl" > job_parameters.json
cat job_parameters.json   # sanity-check

# 2. Execute the 3-command happy path from your SKILL.md, in order:

#    Command 1: CDP health check (with auto-launch). The helper makes the
#    happy path a <1s no-op when CDP is up, and recovers if Chrome crashed.
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222 || { echo "CDP DOWN"; exit 1; }
curl -sfS -m 3 "http://localhost:9222/json/version" > /dev/null || { echo "CDP DOWN"; exit 1; }

#    Command 2: open Freightos
agent-browser --cdp 9222 open https://ship.freightos.com

#    Command 3: run your eval (the discovered API call from Phase A)
#    For the FIRST iteration of canary-01, paste the eval body into Chrome
#    DevTools Console manually and confirm it returns the expected JSON shape.
#    Then save the working JS to /tmp/freightos-build-test/eval.js and re-run
#    headlessly:
agent-browser --cdp 9222 eval "$(cat /tmp/freightos-build-test/eval.js)" > result.json

# Note: agent-browser prints the eval's return value to stdout. The eval body
# ends with `return JSON.stringify({...})`, so stdout IS valid JSON. Redirecting
# to result.json gives us the raw JSON file. If you see truncation or extra
# whitespace, inspect with `cat -A result.json` and trim with jq if needed:
#   jq . result.json > result.tmp && mv result.tmp result.json

# After C.3 stabilizes for all 4 canaries, COPY the contents of eval.js verbatim
# into the `### Compact API call shape` block of SKILL.md. SKILL.md is the
# production artifact; eval.js is just the scratchpad.

# 4. SCHEMA validation
ajv validate -s result.schema.json -d result.json --strict=false --all-errors
# If it errors: schema mismatch — fix result.schema.json OR fix the eval output.

# 5. GOLDEN assertions validation
python3 - <<'PYEOF'
import json, yaml, sys
result = json.load(open('result.json'))
golden = yaml.safe_load(open('/Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/freightos-freight-estimate/golden.assertions.yaml'))
canary_id = "canary-01-dgu-tianjin-portsmouth"   # change per iteration
expectations = golden[canary_id]
failures = []
for key, spec in expectations.items():
    if key == 'invariant':
        # Evaluate as Python expression. result's top-level keys become variables.
        # IMPORTANT: invariant strings can only reference top-level keys of result.
        # Nested access like result['route']['port'] must be written as e.g.,
        # `route.get('port')` after binding route — which won't work; restrict
        # invariants to top-level numeric/string comparisons.
        try:
            if not eval(spec, {"__builtins__": {}}, result):
                failures.append(f"invariant failed: {spec}")
        except Exception as e:
            failures.append(f"invariant raised {type(e).__name__}: {e}; expr was {spec!r}")
        continue
    actual = result.get(key)
    if isinstance(spec, dict):
        if 'min' in spec and actual is not None and actual < spec['min']:
            failures.append(f"{key} ({actual}) < min ({spec['min']})")
        if 'max' in spec and actual is not None and actual > spec['max']:
            failures.append(f"{key} ({actual}) > max ({spec['max']})")
        if 'one_of' in spec and actual not in spec['one_of']:
            failures.append(f"{key} ({actual}) not in {spec['one_of']}")
        if spec.get('must_exist') and (actual is None or actual == ''):
            failures.append(f"{key} must exist but is null/empty")
        if 'must_contain' in spec:
            # Substring match (case-insensitive) for string fields
            if not actual or spec['must_contain'].lower() not in str(actual).lower():
                failures.append(f"{key} ({actual!r}) must contain '{spec['must_contain']}'")
        if 'must_contain_port_codes' in spec:
            # Walk a route object/dict looking for port code prefixes in any string value
            route_blob = json.dumps(actual or {})
            for code in spec['must_contain_port_codes']:
                if code not in route_blob:
                    failures.append(f"{key} route missing port code prefix '{code}'")
    elif key == 'blockers_length':
        # Supports integer (exact count) or dict {min, max}
        actual_len = len(result.get('blockers', []))
        if isinstance(spec, int):
            if actual_len != spec:
                failures.append(f"blockers_length ({actual_len}) != {spec}")
        elif isinstance(spec, dict):
            if 'min' in spec and actual_len < spec['min']:
                failures.append(f"blockers_length ({actual_len}) < min ({spec['min']})")
            if 'max' in spec and actual_len > spec['max']:
                failures.append(f"blockers_length ({actual_len}) > max ({spec['max']})")
    elif key == 'warnings_length':
        actual_len = len(result.get('warnings', []))
        if isinstance(spec, int):
            if actual_len != spec:
                failures.append(f"warnings_length ({actual_len}) != {spec}")
        elif isinstance(spec, dict):
            if 'min' in spec and actual_len < spec['min']:
                failures.append(f"warnings_length ({actual_len}) < min ({spec['min']})")
            if 'max' in spec and actual_len > spec['max']:
                failures.append(f"warnings_length ({actual_len}) > max ({spec['max']})")
    else:
        if actual != spec:
            failures.append(f"{key} ({actual!r}) != {spec!r}")
print("FAILURES:" if failures else "ALL PASS")
for f in failures: print(f"  - {f}")
sys.exit(1 if failures else 0)
PYEOF

# 6. Capture evidence
cp job_parameters.json "$EVID_DIR/${CANARY_ID}-input.json"
cp result.json "$EVID_DIR/${CANARY_ID}-result.json"
echo "PASS at $(date -u +%FT%TZ)" > "$EVID_DIR/${CANARY_ID}-status.txt"
```

**The iteration cycle:**

```
┌─ PICK fixture ────────────────────────────────────────┐
│ Choose next un-passed canary from canary.inputs.jsonl │
└───────────────────────────────────────────────────────┘
            │
┌─ EXECUTE ─────────────────────────────────────────────┐
│ Run the 3-command happy path against live CDP browser │
│ Capture eval output as result.json                    │
└───────────────────────────────────────────────────────┘
            │
┌─ VALIDATE ────────────────────────────────────────────┐
│ Schema:   ajv validate -s schema -d result.json        │
│ Golden:   the python script above                      │
└───────────────────────────────────────────────────────┘
            │
       ┌────┴────┐
   PASS│         │FAIL
       │         │
       ▼         ▼
┌─ EVIDENCE ────┐   ┌─ DIAGNOSE + FIX ────────────────────┐
│ Save inputs + │   │ Read the error:                       │
│ result.json + │   │  - Schema mismatch? Fix schema.        │
│ status.txt    │   │  - Eval returned wrong shape? Fix eval │
│ to test-      │   │    in SKILL.md.                        │
│ evidence/     │   │  - Live API changed? Re-run Phase A    │
└───────────────┘   │    DevTools discovery on this fixture. │
       │            │  - Selector broken? Update selectors.  │
       ▼            │  - Timing issue? Add wait inside eval. │
   NEXT fixture     │  - Authentication issue? Re-login,     │
                    │    retry with fresh session token.     │
                    └────────────────────┬───────────────────┘
                                         │
                                         ▼
                                 RE-EXECUTE same fixture
                                 (do NOT skip and move on)
```

**Loop exit gate:** all 4 canaries have a `${CANARY_ID}-status.txt` containing "PASS" in the evidence directory. Partial passes do NOT count.

#### C.4 — Execute the failure-path matrix (~30-45 min)

After all 4 canaries pass, run the 4 failure-path tests from failure-paths.jsonl. Same loop, but the `expect` block is verified instead of golden assertions. Each failure-path test produces:
- `${FP_ID}-result.json` — the skill's blocked output
- `${FP_ID}-status.txt` — "PASS" if `expect` matched

**Important:** failure-path tests MUST be run in dependency-safe order — `fp-02-cdp-down` stops the browser (no further tests possible until restart) and `fp-03-session-expired` requires logout (no further canary work possible until re-login). Order:

1. **fp-01-captcha-sim** (no browser/session changes — safest first)
2. **fp-04-invalid-country** (no browser/session changes)
3. **fp-03-session-expired** (log out via avatar menu in browser):
   - **Setup:** click Freightos top-right avatar → Logout.
   - **Run:** invoke skill with canary-01 input.
   - **Expected behaviour depends on whether the auto-signin handoff has been executed:**
     - **If `skills/auth-signin/freightos/` exists** (auto-signin handoff complete): skill detects session-expired → invokes `auth-signin/freightos` subprocess → Google OAuth click-sequence runs → s-token recovered → skill retries main operation → `result.json` shows `freight_status: "quoted"`. Verify the recovery log line. This is the PRODUCTION-EXPECTED behaviour.
     - **If `skills/auth-signin/freightos/` does NOT yet exist** (auto-signin handoff pending): skill emits `freight_status: "blocked"`, `cdp_state: "session_expired"`, blocker code `FREIGHTOS_SESSION_EXPIRED`. This is the legacy fallback path.
   - **Recovery (REQUIRED before continuing — applies to BOTH paths):**
     - Confirm "D" avatar present top-right (either restored by auto-signin OR by manual re-login).
     - Manual re-login (if auto-signin didn't run): click "Login" / "Sign in" → "Continue with Google" → select `Dieter Danicek (dieter.x.s.danicek@gmail.com)`.
     - Re-run canary-01 to confirm `freight_status: "quoted"` is restored before continuing. If canary-01 fails after re-login, **stop and diagnose** before fp-02 — something else broke.
   - **If `Dieter Danicek` is NOT signed into the persistent Chrome's Google session** (`chrome://chrome-signin`): STOP fp-03 entirely; skip to fp-02; surface in TEST_REPORT.md as "fp-03 untested — Google account not signed into persistent Chrome; auto-signin would have correctly escalated as `manual_required`."
4. **fp-02-cdp-down** (stop browser — this is the LAST test):
   - **Setup:** kill the Chrome process holding port 9222 (`pkill -f "remote-debugging-port=9222"` or close the browser).
   - **Run:** invoke skill with canary-01 input.
   - **Verify:** result.json has `cdp_state: "unreachable"`, blocker code `CDP_CONNECT_FAILED`.
   - **Recovery:** restart the persistent Chrome on port 9222 (use the same launch command the operator used at C.0.a). Re-verify login state via C.0.a checks before any other work.

#### C.5 — Write the TEST_REPORT.md (final artifact, ~15 min)

Once all 4 canaries + 4 failure-paths show "PASS", write a comprehensive report to:
`docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/freightos-freight-estimate/TEST_REPORT.md`

Template:

```markdown
# Freightos CDP Browser Skill — Test Report

**Skill:** `skills/landed-cost/freightos-freight-estimate/`
**Skill version:** 1.0.0
**Tested by:** <operator name or agent name>
**Test date:** YYYY-MM-DD
**CDP port:** 9222
**Chrome profile:** <path or identifier>
**Freightos account:** <account-id e.g. "D">

## Verdict

**STATUS: READY FOR L5 STEP 09 BEAD AUDIT** (or BLOCKED if any test failed)

## Canary results (4/4 must pass)

| Canary | Route | Load | Result | Runtime | freight_base_amount_minor | Notes |
|---|---|---|---|---|---|---|
| canary-01 | Tianjin → Portsmouth | 1×40' | PASS | 12s | 440381 EUR | DGU baseline; matches Phase A example |
| canary-02 | Shanghai → Felixstowe | 1×20' | PASS | 11s | 320145 EUR | port-airport destination type works |
| canary-03 | Shenzhen → London | 2 pallets LCL | PASS | 14s | 178922 EUR | Loose Cargo path works |
| canary-04 | Tianjin → Portsmouth + services | 1×40' incl. insurance + brokerage | PASS | 22s | 440381 base, 510004 w/services | dual-quote capture works |

## Failure-path results (4/4 must pass)

| ID | Scenario | Result | cdp_state | blocker_code | Notes |
|---|---|---|---|---|---|
| fp-01 | Captcha simulation | PASS | captcha_blocked | CDP_CAPTCHA_INTERCEPTION | short-circuit works |
| fp-02 | CDP down | PASS | unreachable | CDP_CONNECT_FAILED | health check correctly fires |
| fp-03 | Session expired | PASS | session_expired | FREIGHTOS_SESSION_EXPIRED | login-prompt detection works |
| fp-04 | Invalid country | PASS | connected | FREIGHTOS_FORM_REJECTED | input validation works |

## Discoveries from Phase A (live API details)

| Item | Value |
|---|---|
| Search API endpoint | `<POST URL discovered>` |
| Auth mechanism | `<Bearer header / cookie name>` |
| Token storage key | `<localStorage key>` |
| Token refresh endpoint | `<URL or "not needed">` |
| Best Value tab response path | `<JSONPath, e.g., $.tabs.best_value.quotes[0]>` |
| Cheapest tab response path | `<JSONPath>` |
| Quickest tab response path | `<JSONPath>` |
| Quote-with-services strategy | `<single request with toggles=true / second request>` |

## Quirks observed

- <e.g., "Freightos backend returns 502 if you query Friday 5pm GMT due to maintenance window">
- <e.g., "currency selector in top-right defaults to operator's profile setting and overrides USD in payload">

## Schema validation

All 8 result.json files validated successfully against `result.schema.json`.

## Sign-off

Operator confirms the Freightos skill is ready for L5 implementation. Skill files committed at SHA `<git sha>`.
```

#### C.6 — Commit the canary fixtures + test evidence (~5 min)

```bash
cd /Users/alinaaitova/Desktop/Testproject1
git add skills/landed-cost/freightos-freight-estimate/
git add docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/freightos-freight-estimate/
git commit -m "freightos skill: initial CDP-browser implementation + test evidence (4/4 canaries pass, 4/4 failure paths pass)"
git pull --rebase && git push
```

#### C.7 — Loop-exit gate (BLOCKING — verify before Phase D)

- [ ] All 4 canaries in test-evidence/ have `*-status.txt` = "PASS".
- [ ] All 4 failure paths in test-evidence/ have `*-status.txt` = "PASS".
- [ ] `TEST_REPORT.md` exists and shows verdict READY.
- [ ] All result.json files validate against result.schema.json (run ajv on all of them as a final sweep).
- [ ] Test evidence pushed to git.

**If ANY checkbox is unchecked, return to C.3 / C.4 and iterate.** Do NOT advance to Phase D.

### Phase D: Integration smoke (~15 min)

1. Verify the skill directory matches the production skill directory standard from L0 S4 (frontmatter, sections, schemas).
2. Stage the directory: `git add skills/landed-cost/freightos-freight-estimate/`.
3. Run `ubs skills/landed-cost/freightos-freight-estimate/SKILL.md` (Ultimate Bug Scanner) if available — fix any flagged issues.
4. Commit (after L5 step 09 bead audit verifies presence; or commit now if confident): `git commit -m "freightos skill: initial CDP-browser implementation for L5 dual-source cross-check"`.
5. Push.

---

## Verification checklist (operator runs before declaring complete)

### Build verification

- [ ] Skill directory exists at `skills/landed-cost/freightos-freight-estimate/` with both files (SKILL.md + result.schema.json).
- [ ] `SKILL.md` has YAML frontmatter with all required fields (skill_id, version, interaction_profile, requires_cdp:true, cdp_default_port, maxPages, maxScrollCycles).
- [ ] SKILL.md describes the 3-command bounded happy path (CDP check, open URL, eval call).
- [ ] SKILL.md includes the captcha simulation hook verbatim.
- [ ] SKILL.md lists all 7+ Freightos-specific blocker codes.
- [ ] `result.schema.json` includes the 7 required fields (product_candidate_id, selected_quote_id, freight_status, captcha_intercepted, cdp_state, warnings, blockers).
- [ ] Schema validates against the Tianjin→Portsmouth test result JSON.
- [ ] No setup/discovery commands in the success path (no `pwd`, `ls`, `rg`, `find`, accessibility snapshot, etc.).
- [ ] Skill never logs in/out, books, or modifies account state.
- [ ] Skill never clicks "Select" or proceeds to Booking/Verification steps.

### Empirical Test/Validate/Fix loop (CDP must be live during this section)

- [ ] `test-evidence/freightos-freight-estimate/canary.inputs.jsonl` exists with ≥4 canaries.
- [ ] `test-evidence/freightos-freight-estimate/golden.assertions.yaml` exists with expectations per canary.
- [ ] `test-evidence/freightos-freight-estimate/failure-paths.jsonl` exists with ≥4 failure scenarios.
- [ ] `test-evidence/freightos-freight-estimate/canary-01-*-result.json` exists and validates against schema.
- [ ] `test-evidence/freightos-freight-estimate/canary-02-*-result.json` exists and validates against schema.
- [ ] `test-evidence/freightos-freight-estimate/canary-03-*-result.json` exists and validates against schema.
- [ ] `test-evidence/freightos-freight-estimate/canary-04-*-result.json` exists and validates against schema.
- [ ] All 4 canary `*-status.txt` files contain "PASS".
- [ ] All 4 failure-path `*-status.txt` files contain "PASS".
- [ ] `TEST_REPORT.md` exists with verdict READY and all rows populated.
- [ ] Skill runs end-to-end in <180s on the happy-path canaries.
- [ ] `freight_with_services_amount_minor > freight_base_amount_minor` invariant holds for canary-04.
- [ ] Phase A discovery findings (API endpoint, auth, response paths) are documented in TEST_REPORT.md.

### Integration verification

- [ ] All test evidence committed and pushed.
- [ ] Skill files committed and pushed.
- [ ] L5 step 09 bead audit can `ls skills/landed-cost/freightos-freight-estimate/` and find SKILL.md + result.schema.json present.

---

## L5 pipeline integration (what happens after this skill ships)

1. Step 07 bead creation: the Phase 3 Freight engine beads will REFERENCE this skill by path (`skills/landed-cost/freightos-freight-estimate/`). Bead `l5-phase-3-freight-engine-dual-source` will say "invoke landed-cost/freightos-freight-estimate AND landed-cost/searates-freight-estimate in parallel; compare freight_base_amount_minor between them; emit FREIGHT_MODE_LOW_CONFIDENCE if the relative delta exceeds threshold (TBD during calibration, suggested 30% as starting point)."
2. Step 09 bead audit: verifies `skills/landed-cost/freightos-freight-estimate/SKILL.md` exists on disk before authorizing step 10 execution. If missing, audit FAILS.
3. Step 12 completion audit (post-execution): verifies the Freightos skill was successfully invoked at least N times by the L5 Phase 3 freight engine during Stage 11 live demo.

---

## Reference materials in this handoff

### Persisted screenshots (open in Preview during build)

All 9 Freightos screenshots are saved in the project (PNG, ~1 MB each):

```
docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/screenshots/freightos/
├── freightos-step-01.png   # Empty homepage with 4-field search bar
├── freightos-step-02.png   # Origin popover (Type/Country/Address)
├── freightos-step-03.png   # Destination popover (Type/Country/Address)
├── freightos-step-04.png   # Load popover Loose Cargo tab (Pallets/Boxes, # units, dimensions, weight)
├── freightos-step-05.png   # Load popover Containers tab (# units, 20'/40'/40'HC/45'HC, overweight)
├── freightos-step-06.png   # Goods popover (value, hazmat, readiness)
├── freightos-step-07.png   # Filled-out form (Tianjin → Portsmouth example) with readiness dropdown
├── freightos-step-08.png   # Recommended Services page (Insurance + Customs brokerage toggles)
└── freightos-step-09.png   # Results page (Top 28 Quotes, Best value/Quickest/Cheapest/Greenest tabs)
```

These are the canonical UI references. Open them in Preview side-by-side with DevTools while doing Phase A discovery — they show field positions, dropdown options, and the exact "Tianjin → Portsmouth, 1×40' container, $10K, Within 2 weeks" example the operator used.

### Reference skills (re-paste from above for convenience)

- `skills/landed-cost/searates-freight-estimate/SKILL.md` — gold-standard template.
- `skills/landed-cost/searates-freight-estimate/result.schema.json` — output contract pattern.

### Time estimate (revised based on empirical CDP testing requirement)

| Phase | Effort | What |
|---|---|---|
| A — Discovery | 60-90 min | DevTools network/DOM exploration |
| B — Build skill files | 60-90 min | Write SKILL.md + result.schema.json from templates |
| **C — Test/Validate/Fix loop** | **2-4 hours** | **Iterate canaries + failure paths against live CDP until 100% pass; write TEST_REPORT.md** |
| D — Integration smoke | 15-30 min | Final commit + push + cross-verify against L5 manifest |
| **Total** | **~4-7 hours** | (vs. earlier estimate of ~2-3 hours — empirical CDP iteration is the actual work) |

---

## Notes for the next agent (if delegating)

If you (operator) delegate this build to a coding agent:

- The agent MUST read `skills/landed-cost/searates-freight-estimate/SKILL.md` end-to-end FIRST.
- The agent MUST do the Phase A discovery in a real Chrome session with DevTools open; do not assume endpoint shapes.
- The agent MUST treat the existing SeaRates skill as the gold-standard template — same frontmatter structure, same bounded-attempt discipline, same CDP discipline, same captcha simulation hook, same `freight_status: "quoted" | "blocked"` enum.
- The agent MUST NOT use mocks or fixtures during discovery — Freightos' actual response shape is the truth.
- The agent MUST commit and push when done so step 09 bead audit can verify.

---

## End of handoff

When the skill is built, tested against the Tianjin→Portsmouth example, and pushed to main, mark this handoff complete and proceed to the Taric skill handoff (`HANDOFF-taric-cross-check.md`).
