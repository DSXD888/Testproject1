---
skill_id: landed-cost/taric-cross-check
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: codex
side_effect_class: none
requires_cdp: true
cdp_default_port: 9222
maxPages: 4
maxScrollCycles: 2
---

# landed-cost/taric-cross-check

## Mission
Cross-check an HMRC-classified commodity code against the EU TARIC database
(ec.europa.eu/taxation_customs/dds2/taric/) for an independent second opinion
on classification validity and structural signals (supplementary unit
requirement, trade-remedy parallel existence, CBAM scope). This is
informational cross-check intelligence for L5 multi-source classification
recall — NEVER an override of HMRC for UK-specific national measures.

The skill produces signal-grade data for `candidateRecallAgreement` populating;
it does NOT produce decision-grade duty figures. UK calculations always use
HMRC's Trade Tariff via `skills/landed-cost/hmrc-customs-classification/`.

## Inputs
The daemon supplies one `job_parameters` object containing:
- `process_id`, `notice_id`, `product_candidate_id`, `pricing_assessment_id`, `selected_quote_id`
- `goods_code` — string, the TARIC-acceptable goods code (typically 10 digits, e.g., `7008002000`); some headings only have measures at CN8 (e.g., `73181500`)
- `origin_country` — ISO 3166-1 alpha-2 (e.g., `CN`); used as TARIC `Area` parameter directly
- `destination_country` — typically `GB` (informational only; TARIC ignores this dimension at the Consultation level)
- `additional_code` — optional, string (e.g., `B999`); usually empty
- `reference_date` — optional, ISO date (default: today)
- optional `simulate_captcha` (bool, daemon test hook)

## L5 Attempt Budget

The daemon kills each TARIC attempt after a 120-second budget (configurable).
Complete the happy path in one pass:

1. Check CDP.
2. Open the direct TARIC `measures.jsp` URL with `Area` + `Taric` + `SimDate` + `Lang=en` pre-filled in the query string. No form interaction required.
3. Run ONE `eval` that scrapes the rendered measures page (or detects the redirect to the search form indicating not_found).
4. Write `result.json`.

The result schema is fully summarized in this SKILL.md. Do not run setup or
discovery commands: no `pwd`, `ls`, `rg`, `find`, `command -v`, schema printing,
browser CLI help output, accessibility snapshots, title checks, broad page
inspection, repository searches, or exploratory schema discovery.

Use at most three shell commands total in the success path:

1. `curl` CDP check.
2. `agent-browser --cdp ${CDP_PORT} tab new "<measures.jsp URL>"` — opens the TARIC measures result directly via GET query params.
3. `agent-browser --cdp ${CDP_PORT} eval '...'` — extract measures from the rendered page.

After command 3, immediately write `result.json`. If a required value is unavailable,
write a structured `blocked` or `not_found` result with `cdp_state` and a blocker/warning code.
Never leave the workdir without `result.json`.

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

# Defensive reachability check. If this fails, follow the "If CDP is
# unreachable" guidance below: write a blocked taric_status: "blocked" result
# with cdp_state: "unreachable", captcha_intercepted: false, and blocker
# CDP_CONNECT_FAILED.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, emit `taric_status: "blocked"`, `cdp_state: "unreachable"`,
`captcha_intercepted: false`, with blocker code `CDP_CONNECT_FAILED`. Do not launch
a fresh browser. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

## Mandatory CDP State + Captcha Reporting

Every result MUST set both `captcha_intercepted` (boolean) and `cdp_state` (one
of `connected`, `unreachable`, `captcha_blocked`, `session_expired`).
`session_expired` is unlikely for TARIC (no login) but the enum is preserved for
daemon-side consistency.

| Observation | `cdp_state` | `captcha_intercepted` | `taric_status` |
|---|---|---|---|
| `document.title === "TARIC Measure Information"` AND measures extracted | `connected` | `false` | `confirmed` |
| `document.title === "TARIC Measure Information"` AND no measures (sub-tree heading only — code exists but has no direct measures at this level) | `connected` | `false` | `not_found` (with warning `TARIC_CODE_EXISTS_BUT_NO_DIRECT_MEASURES`) |
| `document.title === "TARIC Consultation"` (the measures.jsp request bounced back to the search form — invalid/unknown code) | `connected` | `false` | `not_found` |
| Cloudflare / WAF challenge | `captcha_blocked` | `true` | `blocked` |
| `localhost:${CDP_PORT}/json/version` returns non-2xx | `unreachable` | `false` | `blocked` |
| TARIC backend 5xx | `connected` | `false` | `blocked` (with `TARIC_SERVICE_DOWN`) |

When `cdp_state` is anything other than `connected`, emit the result IMMEDIATELY
(do not retry inside the skill — the daemon's retry loop owns retries). Include
a single `Stage11Issue`-shaped blocker in `blockers` describing what was
observed (e.g. `CDP_CAPTCHA_INTERCEPTION`, `CDP_CONNECT_FAILED`,
`TARIC_SERVICE_DOWN`).

### Blocker / warning code reference (TARIC-specific)

- `CDP_CONNECT_FAILED` — CDP itself down.
- `CDP_CAPTCHA_INTERCEPTION` — WAF / Cloudflare wall (rare for ec.europa.eu but possible).
- `TARIC_SERVICE_DOWN` — 5xx error from TARIC backend or page never loaded.
- `TARIC_GOODS_CODE_NOT_FOUND` — TARIC redirected back to the search form indicating the queried code does not exist or has no measures (treat as `taric_status: "not_found"` with warning, NOT a blocker).
- `TARIC_CODE_EXISTS_BUT_NO_DIRECT_MEASURES` — measure page loaded but only sub-tree headings rendered, no actual measure rows (warning, not blocker; emit `taric_status: "not_found"`).
- `TARIC_FORM_REJECTED` — server rejected the query (bad goods code format like non-numeric).
- `TARIC_PARSE_FAILED` — page rendered but the measures region couldn't be located (UI change broke selectors).

## Captcha Simulation (Daemon-driven Test Hook)

If `job_parameters.simulate_captcha` is `true`, short-circuit immediately
without driving the browser:

```json
{
  "product_candidate_id": "...",
  "queried_goods_code": "...",
  "taric_status": "blocked",
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

This lets the daemon exercise its retry/cache/operator-notification paths
without needing a real captcha to occur.

## Workflow

The TARIC Consultation form is a server-rendered JSP whose JS submit handler
ultimately navigates to `measures.jsp` with the search parameters in the query
string. Skip the form interaction entirely — GET the result URL directly.

1. Build the TARIC measures URL:
   `https://ec.europa.eu/taxation_customs/dds2/taric/measures.jsp?Lang=en&SimDate=${YYYYMMDD}&Area=${ORIGIN}&Taric=${GOODS_CODE}`
   - `SimDate` format is `YYYYMMDD` (e.g., `20260517` for 2026-05-17). If `job_parameters.reference_date` is provided, transform `YYYY-MM-DD` → `YYYYMMDD`. Otherwise use today (UTC).
   - `Area` is the ISO 3166-1 alpha-2 code — TARIC's dropdown uses ISO-2 directly (`CN`, `MY`, `DE`, `GB`, etc.).
   - `Taric` is the goods code, digits only. Strip any spaces. Leave as-is whether 8 or 10 digits — TARIC honours both.
2. Open the URL via `agent-browser --cdp ${CDP_PORT} tab new "<url>"`. Wait ~2 seconds for render (`tab new` returns when DOM is ready in most cases, but the JS that builds measure tables is fast).
3. Run a single `eval` (script body below) that:
   - Reads `document.title`.
   - If title is `"TARIC Consultation"`, the request redirected to the search form — the code or area is invalid. Return `taric_status: "not_found"`, warning `TARIC_GOODS_CODE_NOT_FOUND`, no blockers.
   - If title is `"TARIC Measure Information"`, slice the body text from `"Goods nomenclature code"` up to the last `"Top\n"` (which marks the start of footnote/regulation appendices). That is the measures region.
   - Capture `Last TARIC update: DD-MM-YYYY` from the body for `taric_last_update`.
   - From the measures region, extract:
     - Third country duty rate (MFN) via regex `Third country duty[^\n]*?:\s*([\d.,]+\s*%|[\d.,]+\s*EUR[^\n]*)`.
     - Supplementary unit via regex `Supplementary unit[^\n]*?:\s*(\S+)`.
     - Boolean presence flags for: Anti-dumping, Countervailing, Safeguard, Suspension, Tariff preference, Tariff quota, Additional duties, Carbon Border Adjustment Mechanism (CBAM).
     - Per-row measure summaries: each line of the region matching the recognised measure-type whitelist, with the rate text and any geographical-area annotation following it.
   - If the title says `"TARIC Measure Information"` but the regex finds NO MFN line AND no other measure rows (the page is just sub-tree headings under the queried code), emit `taric_status: "not_found"` with warning `TARIC_CODE_EXISTS_BUT_NO_DIRECT_MEASURES` — the goods code is structural, not declarable at this level.
4. Write the final `result.json`.

### Scrape eval (USE THIS SHAPE — selectors empirically validated against the new UI 2026-05-17)

```js
(() => {
  const title = document.title;
  const url = location.href;

  // Detect redirect-to-form (= invalid goods code / unknown area combination)
  if (title === "TARIC Consultation") {
    return JSON.stringify({
      taric_status: "not_found",
      cdp_state: "connected",
      captcha_intercepted: false,
      title, url,
      reason: "Request bounced back to search form — goods code/area unknown to TARIC."
    });
  }

  if (title !== "TARIC Measure Information") {
    return JSON.stringify({
      taric_status: "blocked",
      cdp_state: "connected",
      captcha_intercepted: false,
      blocker_code: "TARIC_PARSE_FAILED",
      title, url,
      reason: "Unexpected page title — TARIC layout may have changed."
    });
  }

  const all = document.body.innerText || "";

  // Capture "Last TARIC update: DD-MM-YYYY" from the page metadata.
  const lastUpdateMatch = all.match(/Last TARIC update:\s*([\d-]+)/);
  const taricLastUpdate = lastUpdateMatch ? lastUpdateMatch[1] : null;

  // Trim to measures region — from "Goods nomenclature code" through the last
  // "Top\n" marker (footnote/regulation appendices begin after that marker).
  let region = all;
  const topIdx = region.lastIndexOf("Top\n");
  if (topIdx > 0) region = region.substring(0, topIdx);
  const startIdx = region.indexOf("Goods nomenclature code");
  if (startIdx > 0) region = region.substring(startIdx);

  // Extract MFN duty (third country duty).
  const mfnMatch = region.match(/Third country duty[^\n]*?:\s*([\d.,]+\s*%|[\d.,]+\s*EUR[^\n]*)/i);
  const mfn_rate_text = mfnMatch ? mfnMatch[1].trim() : null;

  // Parse simple percent to bps (e.g., "3.00 %" -> 300; "3.70 %" -> 370).
  let mfn_rate_bps = null;
  if (mfn_rate_text) {
    const pct = mfn_rate_text.match(/^([\d.]+)\s*%$/);
    if (pct) mfn_rate_bps = Math.round(parseFloat(pct[1]) * 100);
  }

  // Extract supplementary unit. Null is valid (many codes do not declare one).
  const suppMatch = region.match(/Supplementary unit[^\n]*?:\s*(\S+)/i);
  const supplementary_unit = suppMatch ? suppMatch[1].trim() : null;

  // Boolean presence flags.
  const has_anti_dumping = /Anti.?dumping/i.test(region);
  const has_countervailing = /Countervailing/i.test(region);
  const has_safeguard = /Safeguard/i.test(region);
  const has_suspension = /\bSuspension\b/i.test(region);
  const has_tariff_preference = /Tariff preference/i.test(region);
  const has_tariff_quota = /Tariff quota/i.test(region);
  const has_additional_duties = /Additional duties/i.test(region);
  const has_cbam = /Carbon Border Adjustment Mechanism/i.test(region);

  // Per-row measure summaries (lines whose first token matches the whitelist).
  const measureLineRegex = /\b(Third country duty|Anti-dumping|Countervailing|Safeguard|Tariff preference|Tariff quota|Suspension|Additional duties|Supplementary unit|Excise|Carbon Border Adjustment Mechanism|Import control|Non preferential tariff quota|Airworthiness)/i;
  const measureLines = region
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0 && measureLineRegex.test(l));
  const measures = measureLines.map((line) => {
    const m = line.match(/^([^(]+?)(?:\s*\(([^)]+)\))?\s*:\s*(.*)$/);
    return m
      ? {
          measure_type: m[1].trim(),
          period: m[2] ? m[2].trim() : null,
          rate_text: m[3] ? m[3].trim() : null,
        }
      : { measure_type: line.substring(0, 60), period: null, rate_text: null };
  });

  // Detect "page rendered but no actual measure rows" — code exists structurally
  // but has only sub-tree headings (not declarable at this level).
  const has_any_substantive_measure =
    !!mfn_rate_text || !!supplementary_unit ||
    has_anti_dumping || has_countervailing || has_safeguard ||
    has_suspension || has_tariff_preference || has_tariff_quota ||
    has_additional_duties || has_cbam;

  if (!has_any_substantive_measure) {
    return JSON.stringify({
      taric_status: "not_found",
      cdp_state: "connected",
      captcha_intercepted: false,
      title,
      url,
      taric_last_update: taricLastUpdate,
      warning_code: "TARIC_CODE_EXISTS_BUT_NO_DIRECT_MEASURES",
      reason: "Page rendered but contains only sub-tree headings; goods code is structural, not declarable at this level."
    });
  }

  return JSON.stringify({
    taric_status: "confirmed",
    cdp_state: "connected",
    captcha_intercepted: false,
    title,
    url,
    taric_last_update: taricLastUpdate,
    mfn_rate_text,
    mfn_rate_bps,
    supplementary_unit,
    has_anti_dumping,
    has_countervailing,
    has_safeguard,
    has_suspension,
    has_tariff_preference,
    has_tariff_quota,
    has_additional_duties,
    has_cbam,
    measures_count: measures.length,
    measures,
    raw_evidence_text: region.substring(0, 4000)
  });
})()
```

### Country / area mapping

TARIC's `Area` query parameter accepts ISO 3166-1 alpha-2 codes directly.
The dropdown values were empirically verified for canonical L5 origins:

| ISO | TARIC `Area` | Label |
|---|---|---|
| CN | `CN` | China - CN |
| MY | `MY` | Malaysia - MY |
| IN | `IN` | India - IN |
| US | `US` | United States - US |
| DE | `DE` | Germany - DE |
| IT | `IT` | Italy - IT |
| TR | `TR` | Türkiye - TR |
| RU | `RU` | Russian Federation - RU |
| GB | `GB` | United Kingdom - GB |

Pass the input `origin_country` directly as the `Area` value with no
translation. TARIC also accepts geographical-area codes (e.g., `XS` for
Serbia) and country groups (e.g., `1010` for ERGA OMNES) but L5 supplies
country ISO-2 only.

### Reference-date handling

If `job_parameters.reference_date` is provided (`YYYY-MM-DD`), transform to
`SimDate=YYYYMMDD` in the URL. Otherwise use today UTC. TARIC supports
historical queries — `reference_date: "2019-01-01"` returns the pre-Brexit
EU view, which is occasionally useful for cross-period customs research.

### What to do with the data (cross-check semantics)

For the L5 multi-source-recall engine, the most useful fields are:

| Field | Cross-check use |
|---|---|
| `taric_status` | `confirmed` = EU TARIC knows this code+area; `not_found` = disagreement signal vs HMRC |
| `measures_count` | 0 with `confirmed` is suspicious; expected >= 1 (third-country duty at minimum) |
| `supplementary_unit` | Cross-check vs HMRC `measurement_unit_code` — STRUCTURAL agreement signal (CN code structure is identical EU/UK) |
| `has_anti_dumping` / `has_countervailing` / `has_safeguard` | Cross-check vs HMRC `trade_remedy_state` — parallel-trade-remedy signal |
| `has_cbam` | Cross-check vs HMRC CBAM scope — material composition signal for iron/steel/aluminium/etc. |
| `mfn_rate_text` / `mfn_rate_bps` | Cross-check vs HMRC `basic_duty_rate_bps` — informational only (UK MFN can differ from EU MFN post-Brexit; treat as agreement signal not authority) |

The orchestrator code in `packages/intelligence/src/customs/multi-source-recall.ts`
will read this skill's output and populate the `candidateRecallAgreement` field
on `landed_cost_estimates` with the agreement/disagreement signals.

## UK-Specificity Rule (CONSTITUTIONAL — per L5 spec §1.4)

This skill returns EU TARIC data. EU TARIC is INFORMATIONAL for UK importation
purposes. NEVER use this skill's `mfn_rate_text`, `vat`, or any measure rate to
override HMRC's view in `landed_cost_estimates`. UK-authoritative measures
(DE 6/17 national additional codes, UK quotas, UK trade-remedy implementations)
are HMRC-only.

The orchestrator code treats this skill's output as a SECOND OPINION signal,
weighted in `commodity_confidence_bps` calibration but never overriding
HMRC-sourced numeric outputs. A divergence between EU MFN and UK MFN is a
LEGITIMATE signal (post-Brexit divergence happens), not a bug.

## Auto-signin: NOT APPLICABLE for this skill

TARIC (`ec.europa.eu/taxation_customs/dds2/taric/`) is **free public EU customs
data** — no login mechanism exists, no session cookies are required, no API
key is needed. The `auth-signin` skill family does NOT include a
`skills/auth-signin/taric/` skill, and this production skill does NOT include
a "Session Recovery" section. There is no logged-out state to detect or
recover from.

The CDP browser is still required (because the TARIC page is a JSP
server-rendered form, not a static page, and the operator's persistent Chrome
carries the right user-agent / geolocation expected by EU TARIC's WAF), but no
authentication is performed.

If the skill ever encounters an unexpected login wall (which would indicate
either a TARIC infrastructure change or the operator's Chrome being WAF-blocked
for some other reason), emit `TARIC_UNEXPECTED_LOGIN_WALL` as a defensive
blocker — but this is purely speculative; TARIC has been a free public EU
resource since 1992 and is unlikely to ever gate access.

## Safety

Do not log in (there is no login). Do not change reference date casually —
TARIC historical lookups are part of the legitimate use, but consistency with
HMRC's `tariff_checked_at` matters; default to today unless
`job_parameters.reference_date` is explicitly set. Do not write server-side
state. Do not bookmark or download the TARIC Full database (multi-GB).

## Output

Write the final JSON object to `result.json` in the current working directory,
matching `result.schema.json`. `evidence`, `warnings`, and `blockers` must be
arrays; `warnings` and `blockers` entries must be objects with `code`, `message`,
and `severity`. Never emit bare string warning codes or omit severity. Also keep
the final response to the daemon as the same raw JSON object; do not wrap it in
Markdown.
