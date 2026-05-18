# Build Handoff: `skills/landed-cost/taric-cross-check/`

> **Audience:** The operator (or a coding agent the operator delegates to) building this skill personally before L5 step 09 bead audit runs.
>
> **Status when done:** Skill directory exists at `skills/landed-cost/taric-cross-check/` with `SKILL.md` + `result.schema.json` (NO `checkpoint.schema.json` — landed-cost family doesn't use them), AND all 9 test-evidence files exist showing PASS for 5 canaries + 4 failure paths, AND TEST_REPORT.md shows verdict READY with HMRC cross-agreement section populated empirically (not placeholders). Ready for the L5 pipeline's Phase 4 customs multi-source-recall engine to invoke as a cross-check.
>
> **L5 manifest reference:** `docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml#recorded_decisions.external_api_account_state.operator_built_prerequisites[1]`
>
> **If you are a coding agent receiving this handoff (delegated by operator) — mandatory read order, no skipping:**
> 1. **READ THE README FIRST (MANDATORY, ~5 min):** `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/README.md`. This explains WHY both prerequisite skills exist, the L5 → L6 readiness chain, the step 09 audit gate that will verify your work, and WHY partial-pass is unacceptable. Without this context you may build a skill that technically scrapes TARIC but produces signals the L5 multi-source-recall engine can't use, and you will burn iterations re-discovering the rationale that's already documented. **Do NOT proceed to step 2 until you've read the README in full.**
> 2. **Read this entire handoff** before any tool call. The Phase A discovery + Phase C Test/Validate/Fix loop (especially the C.5 HMRC cross-agreement scorer) are the bulk of the work; skipping context here breaks the skill.
> 3. **Read three reference files end-to-end before opening agent-browser:** `skills/landed-cost/searates-freight-estimate/SKILL.md` (CDP discipline template), `skills/landed-cost/hmrc-customs-classification/SKILL.md` (result-contract pattern AND the skill this one cross-checks), and `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/reference/Taric_user_guide.pdf` (107 pages of TARIC concept reference — skim the table of contents and read sections relevant to "Search by goods code and geographical area" + measure types taxonomy).
> 4. **The Phase A discovery happens against the LIVE EU TARIC site** in the operator's persistent Chrome. The PDF is OLD UI — the new UI selectors must be discovered empirically. Verify Chrome readiness per C.0.a BEFORE Phase A.
> 5. **The Python HMRC cross-agreement scorer in Phase C.5 is non-trivial.** It walks the HMRC Trade Tariff API's `included` array looking for measure_type / measure_component IDs. The IDs (103=MFN, 105=AD, 305=supp unit, 552=safeguard) are documented inline. If the HMRC API response shape differs from what the scorer expects, fix the extractor in place AND note in TEST_REPORT.md §"Quirks observed".
> 6. **UK-Specificity Rule is constitutional.** This skill produces INFORMATIONAL signal, never an override of HMRC for UK customs calculations. If during your discovery you're tempted to write the skill to "return TARIC's MFN as authoritative", STOP — that violates the L5 spec §1.4 contract.
>
> **Sibling handoff + assets (also mandatory references during the build):**
> - `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/HANDOFF-freightos-freight-estimate.md` — the other operator-built skill; build either order; they're independent.
> - TARIC user guide PDF (107 pages, OLD UI but authoritative concepts) at `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/reference/Taric_user_guide.pdf` — read it after the README + this handoff, as part of step 3 above.
> - Current-UI screenshot at `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/screenshots/taric/taric-consultation-new-ui-2026-05-17.png` — open in Preview (or via Read tool if you're a coding agent without GUI access) during Phase A selector discovery.

---

## Mission

Build a CDP-browser skill that uses the operator's persistent Chrome to query the **EU TARIC Consultation database** (`ec.europa.eu/taxation_customs/dds2/taric/taric_consultation.jsp`) for measures associated with a given goods code + origin/destination, and returns a structured `result.json` matching the schema in this handoff.

The skill provides an **independent cross-check** for L5's multi-source HS classification recall (spec §1.4). It is NOT a primary classifier and NEVER overrules HMRC on UK-specific national measures, VAT additional codes (DE 6/17), UK quotas, or UK trade-remedy implementations. It produces a "second opinion" signal that feeds into the `candidateRecallAgreement` field on `landed_cost_estimates`.

**This skill replaces the paid Taric Support vendor API.** Per L5 manifest decision 2026-05-08, the underlying EU TARIC database is free public EU data — we scrape it directly. No vendor subscription needed.

---

## Why this matters (context for the builder)

- L5 spec §1.4 mandates **5 parallel HS classification recall sources** with the rule "FPO alone is NOT evidence — independent agreement requires slot-level compatibility checking." Without an independent source like Taric, the engine cannot validate HMRC's classifications.
- L5 spec §1.4 UK-specificity rule: "Taric Support and HS Ping are useful cross-checks but must NEVER overrule HMRC on UK-specific national measures." This skill produces an INFORMATIONAL cross-check, not a decision-grade override.
- The L5 Phase 4 customs engine at `packages/intelligence/src/customs/multi-source-recall.ts` (to be built in Phase 4 beads) will invoke this skill in parallel with the HMRC skill. The Phase 4 bead description will say "invoke `landed-cost/hmrc-customs-classification` for primary classification, then `landed-cost/taric-cross-check` for EU TARIC cross-check; populate `candidateRecallAgreement` with both source signals."
- Per spec §1.4 critical rule: "Correlated agreement between FPO, Taric Support, and HS Ping is not independent evidence — all three may return the same plausible-but-wrong family for vague input." So this skill's role is to provide AN OPINION, not THE truth. Calibration weighs its signal alongside others.
- Without this skill, L5's Phase 4 multi-source-recall bead cannot complete and the L5 → L6 readiness gate is at risk (specifically `CLASSIFICATION_AMBIGUOUS` emission depends on having 2+ source opinions to compare).

---

## Reference skills (READ THESE FIRST — gold-standard patterns)

Read these files end-to-end before starting:

1. **`skills/landed-cost/searates-freight-estimate/SKILL.md`** (296 lines) — primary template for CDP discipline + captcha discipline + bounded-attempt discipline. Same patterns apply to this skill. Key sections:
   - YAML frontmatter (`requires_cdp: true`, `cdp_default_port: 9222`, `maxPages`, `maxScrollCycles`)
   - "Stage 11 Attempt Budget" — bounded happy path
   - "Mandatory CDP Discipline" — curl health check before browser commands
   - "Mandatory CDP State + Captcha Reporting" table
   - "Captcha Simulation (Daemon-driven Test Hook)"
2. **`skills/landed-cost/hmrc-customs-classification/SKILL.md`** (181 lines) — your closer template since both skills deal with customs classification (vs SeaRates' freight). Key sections:
   - "Stage 11 Bounded Runner Discipline" — 3 commands max
   - "Required Sources" — the HMRC Trade Tariff API URL pattern
   - "Domestic Path" / "Import Rules" — show how to handle edge cases like no-classification-possible
   - Result contract structure with `candidate_commodity_codes`, `commodity_confidence_bps`, `classification_status` enum
3. **`skills/landed-cost/hmrc-customs-classification/result.schema.json`** — your output contract template basis (46 lines, classification-oriented).

Also read `AGENTS.md` lines 21-33 (Rule 2 wrappers) — not directly relevant to skill runtime, but it's how the daemon will invoke it.

**Critical difference from SeaRates:** TARIC requires NO login (free public EU data). There is no session token, no `localStorage` auth, no Bearer header. The skill is purely an unauthenticated CDP browser query. CDP is still required because the page is a JSP form-driven app with AJAX behavior, not a static HTML page; running it through CDP gives reliable form-fill + result extraction in the operator's standard browser context.

---

## Site under scrape: `ec.europa.eu/taxation_customs/dds2/taric/taric_consultation.jsp`

URL: `https://ec.europa.eu/taxation_customs/dds2/taric/taric_consultation.jsp?Lang=en`

Login state: NONE required. EU TARIC is free public data.

Access: should work from any geography (no region lock for TARIC Consultation, though Display-Database-Services like national-additional-codes vary by member state).

**The "old UI" vs "new UI" point from the operator:** The PDF user guide at `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/reference/Taric_user_guide.pdf` (107 pages, persisted in the project) documents the OLD UI. The screenshot at `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/screenshots/taric/taric-consultation-new-ui-2026-05-17.png` shows the CURRENT (new) UI. Concepts are identical; layout differs. Field names and semantics in the PDF are still authoritative — just look at the screenshot to see where the same fields live now.

---

## What the screenshot tells us (the current UI)

The new-UI screenshot is persisted at:
`docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/screenshots/taric/taric-consultation-new-ui-2026-05-17.png`

Open it in Preview alongside DevTools during Phase A discovery — it shows the current form layout that you will scrape.

From the operator-provided screenshot of `taric_consultation.jsp`:

- **Page title:** "TARIC Consultation"
- **Top-right metadata:** "Last TARIC update: 15-05-2026", with links to "TARIC Daily updates" and "TARIC Full database"
- **Top tabs:** **Measures** (default active) | **Geographical areas** | **Regulations**
- **Active panel heading:** "Search by goods code and geographical area"
- **Inputs:**
  - **Goods code** field (numeric, ~10 digit Combined Nomenclature) with helper link `[Browse the Nomenclature]`
  - **Additional code** field (optional, for additional duty codes)
  - **Origin/destination** dropdown (country selector)
  - **[Advanced search]** link (more filters)
- **Action:** **Retrieve Measures** button
- **Reference date:** "The current reference date for the query is `17-05-2026`" with calendar picker (defaults to today)

**Workflow** (deduced from the form):
1. Enter a 10-digit goods code (e.g., `7008002000` for sealed insulating glass).
2. Optionally enter an additional code (rarely used; leave blank by default).
3. Select an origin/destination from the dropdown (e.g., `CN` for China).
4. Optionally adjust reference date (default today is fine).
5. Click "Retrieve Measures".
6. The page reloads/renders with a measures table showing duties, geographical-area-specific rates, anti-dumping/countervailing measures, supplementary unit requirements, footnotes referencing legal acts.
7. Optionally switch to "Geographical areas" or "Regulations" tabs for related context.

---

## What measures TARIC returns (for L5 cross-check use)

When you query a goods code + origin, TARIC's Measures view typically shows:

| Measure type | What it is | L5 cross-check signal |
|---|---|---|
| Third-country duty (MFN) | Base import duty rate for non-preferential trade | Compare to HMRC's MFN rate — should match closely (UK Tariff is largely derived from EU MFN baseline post-Brexit) |
| Tariff preferences | Lower rates under FTAs | UK has DIFFERENT FTA portfolio post-Brexit; agreement only signals "non-empty FTA list exists" |
| Tariff quotas | Limited-volume duty-free or reduced-rate quotas | UK has DIFFERENT quotas; informational only |
| Anti-dumping duty | Trade remedy against specific origins | If EU AD exists for CN steel and UK AD exists for CN steel → strong agreement; if EU has and UK doesn't (or vice versa), divergence signal |
| Countervailing duty | Like AD but for subsidies | Same logic as AD |
| Safeguard measures | Time-limited protections | Same logic as AD |
| Suspensions | Temporary duty suspensions for industrial inputs | UK has different suspension list; informational |
| VAT | Standard/reduced VAT applicable | NEVER use TARIC VAT for UK calc (per UK-specificity rule); HMRC's UK VAT is authoritative |
| Additional duties | Surcharges (e.g., emergency measures) | Informational |
| Excise duties | Alcohol, tobacco, fuel | Informational |
| Supplementary units | Unit of measure required for declarations (e.g., kg net, square metres) | Cross-check vs HMRC's supp unit requirement — STRONG agreement signal (the underlying CN code structure is identical EU/UK) |

**The single most valuable cross-check signal** from this skill is whether **EU TARIC and HMRC agree on the supplementary unit requirement**, because the supp unit is structural (it follows the CN code's nature, not jurisdiction-specific policy). Disagreement here means either the HMRC skill picked the wrong code OR TARIC's view differs (rare but possible for newer codes).

**The second most valuable signal** is whether **EU AD/CVD measures exist for this code+origin pair**. UK and EU often have parallel trade remedies; this is a useful "does a trade remedy exist somewhere" cross-check even though we never USE the EU rate for UK calculation.

---

## Architectural decisions for the skill

### Network strategy: form POST or page scrape

TARIC Consultation is a JSP page. Investigate during build:
- **Option A (preferred if it works):** capture the AJAX/fetch call that "Retrieve Measures" triggers (likely a POST to a backend with goods code + origin + date as form data). Replay via `fetch` in a single `eval` call. Returns structured JSON or HTML fragment.
- **Option B (fallback):** drive the DOM — fill 3 inputs, click button, scrape the rendered measures table from `document` after the result panel appears.

The TARIC application is known to be a server-rendered JSP that posts back to itself with a full page reload (no AJAX in the old UI; the new UI may or may not have changed this). If it's still server-rendered, Option B (DOM scrape) is the realistic path. The PDF user guide has screenshots of the OLD result page layout; the structure is consistent enough that the new UI's HTML should be parseable with the same logical fields.

### Bounded-attempt discipline (adopt SeaRates pattern verbatim)

```
The daemon kills each Taric attempt after its configured per-attempt budget
(120 seconds by default — shorter than SeaRates because TARIC is fast).
Complete the happy path in one pass:

1. Check CDP.
2. Open TARIC Consultation URL with goods code + origin pre-filled in query string (if supported), OR open the bare URL and fill via eval.
3. Run ONE eval that fills the form (if not pre-filled), submits, waits for result panel, scrapes measures table, returns compact JSON.
4. Write `result.json`.

Use at most three shell commands total in the success path:
1. `curl` CDP check.
2. `agent-browser --cdp ${CDP_PORT} open https://ec.europa.eu/taxation_customs/dds2/taric/taric_consultation.jsp?Lang=en`.
3. `agent-browser --cdp ${CDP_PORT} eval '...'` for form-fill + submit + scrape.

After command 3, immediately write `result.json`. If a required value is unavailable,
write a structured `blocked` result with `cdp_state` and a `TARIC_*` blocker code.
Never leave the workdir without `result.json`.
```

### CDP state + captcha reporting (adopted)

Same enums + table as SeaRates skill. TARIC is unauthenticated so `session_expired` is not really a possible state, but keep the enum unchanged for daemon-side consistency:

| Observation | `cdp_state` | `captcha_intercepted` | `taric_status` |
|---|---|---|---|
| Normal TARIC response with measures | `connected` | `false` | `confirmed` |
| Goods code unknown in TARIC | `connected` | `false` | `not_found` |
| Cloudflare / WAF challenge | `captcha_blocked` | `true` | `blocked` |
| `localhost:${CDP_PORT}/json/version` returns non-2xx | `unreachable` | `false` | `blocked` |
| TARIC service down (5xx, page never loads) | `connected` | `false` | `blocked` (with `TARIC_SERVICE_DOWN`) |

Blocker codes specific to TARIC:
- `CDP_CONNECT_FAILED` — CDP itself down (same as SeaRates)
- `CDP_CAPTCHA_INTERCEPTION` — WAF / Cloudflare wall (rare for ec.europa.eu but possible during high-traffic periods)
- `TARIC_SERVICE_DOWN` — 5xx error from TARIC backend
- `TARIC_GOODS_CODE_NOT_FOUND` — TARIC returns "no measures found" or equivalent for the queried code (treat as a successful `not_found` outcome, NOT a blocker — see status below)
- `TARIC_FORM_REJECTED` — form submission rejected (bad goods code format, invalid country, etc.)
- `TARIC_PARSE_FAILED` — page rendered but the measures table couldn't be extracted (UI change broke selector)

**Note:** `TARIC_GOODS_CODE_NOT_FOUND` is interesting — it's not really a blocker, it's a SIGNAL ("EU TARIC doesn't know this code"). Treat as `taric_status: "not_found"` with status code in `warnings`, not in `blockers`. This is meaningful cross-check evidence — if HMRC has the code but EU TARIC doesn't, that's an agreement-disagreement signal.

### Captcha simulation hook (verbatim from SeaRates)

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

---

## Target directory structure

```
skills/landed-cost/taric-cross-check/
├── SKILL.md                  # Full skill spec (template below — operator fills in)
├── result.schema.json        # Output contract (template below)
└── (NO checkpoint.schema.json — landed-cost family doesn't use checkpoints)
```

---

## SKILL.md template (start here; modify as needed during build)

```markdown
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
(ec.europa.eu/taxation_customs/dds2/taric/taric_consultation.jsp) for an
independent second opinion on classification validity and structural signals
(supplementary unit requirement, trade-remedy parallel existence). This is
informational cross-check intelligence for L5 multi-source classification recall —
NEVER an override of HMRC for UK-specific national measures.

The skill produces signal-grade data for `candidateRecallAgreement` populating;
it does NOT produce decision-grade duty figures. UK calculations always use
HMRC's Trade Tariff via skills/landed-cost/hmrc-customs-classification.

## Inputs
The daemon supplies one object containing:
- `process_id`, `notice_id`, `product_candidate_id`, `pricing_assessment_id`, `selected_quote_id`
- `goods_code` — string, the 10-digit Combined Nomenclature code (e.g., `7008002000`)
- `origin_country` — ISO 3166-1 alpha-2 (e.g., `CN`)
- `destination_country` — typically `GB` (or any EU country for testing); informational
- `additional_code` — optional, string (e.g., `B999`), usually empty
- `reference_date` — optional, ISO date (default: today)
- optional `simulate_captcha` (bool, daemon test hook)

## L5 Attempt Budget

The daemon kills each TARIC attempt after a 120-second budget (configurable).
Complete the happy path in one pass:

1. Check CDP.
2. Open TARIC Consultation URL.
3. Fill goods_code + origin in form, submit, scrape measures.
4. Write `result.json`.

The result schema is fully summarized in this SKILL.md. Do not run setup or
discovery commands: no `pwd`, `ls`, `rg`, `find`, `command -v`, schema printing,
browser CLI help output, accessibility snapshots, title checks, broad page
inspection, repository searches, or exploratory schema discovery.

Use at most three shell commands total in the success path:
1. `curl` CDP check.
2. `agent-browser --cdp ${CDP_PORT} open https://ec.europa.eu/taxation_customs/dds2/taric/taric_consultation.jsp?Lang=en`.
3. `agent-browser --cdp ${CDP_PORT} eval '...'` for form-fill + submit + scrape.

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
# persistent profile at /Users/alinaaitova/.chrome-automation and polls up to
# 15s). Returns non-zero only if launch genuinely failed. Absolute path because
# the skill subprocess cwd is the per-skill workdir, not the project root.
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222

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
`session_expired` is unlikely for TARIC (no login) but keep the enum for daemon-
side consistency.

| Observation | `cdp_state` | `captcha_intercepted` | `taric_status` |
|---|---|---|---|
| TARIC returned measures for queried code+origin | `connected` | `false` | `confirmed` |
| TARIC reports "no measures found" for code+origin | `connected` | `false` | `not_found` |
| Cloudflare / WAF challenge | `captcha_blocked` | `true` | `blocked` |
| `localhost:${CDP_PORT}/json/version` returns non-2xx | `unreachable` | `false` | `blocked` |
| TARIC backend 5xx | `connected` | `false` | `blocked` (with TARIC_SERVICE_DOWN) |

## Captcha Simulation (Daemon-driven Test Hook)

If `job_parameters.simulate_captcha` is `true`, short-circuit:

```json
{
  "product_candidate_id": "...",
  "queried_goods_code": "...",
  "taric_status": "blocked",
  "captcha_intercepted": true,
  "cdp_state": "captcha_blocked",
  "warnings": [],
  "blockers": [
    { "code": "CDP_CAPTCHA_INTERCEPTION", "message": "Captcha simulation requested...", "severity": "blocked" }
  ]
}
```

## Workflow

1. Open `https://ec.europa.eu/taxation_customs/dds2/taric/taric_consultation.jsp?Lang=en`
   through CDP. Wait for the form to render (Goods code input visible).
2. In a single `agent-browser --cdp ${CDP_PORT} eval '...'` call:
   - Fill the Goods code input with `job_parameters.goods_code` (10-digit, no spaces).
   - Open the Origin/destination dropdown and select the option matching `job_parameters.origin_country`. Build a country-ISO → option-text mapping during Phase A discovery and store it inline in the skill. (Common L5 origins: CN=China, IN=India, US=United States, DE=Germany, IT=Italy, TR=Turkey.)
   - Leave Additional code empty unless `job_parameters.additional_code` is provided.
   - If `reference_date` is provided and != today, fill the calendar picker.
   - Submit by clicking the "Retrieve Measures" button.
   - Wait for the result panel to render (selector TBD during Phase A discovery — likely a `#measureResult` div or similar).
   - Scrape the measures table. Extract:
     - Each row's measure type (e.g., "Third country duty", "Anti-dumping duty", "Tariff preference")
     - Geographical area code (origin-specific or "ERGA OMNES")
     - Rate (numeric, format varies: percent, EUR/kg, EUR/litre, compound)
     - Validity dates
     - Conditions / additional codes (if any)
     - Footnote references
   - Detect "no measures found" / "goods code does not exist" → return `taric_status: "not_found"` with warning.
   - Detect supplementary unit requirement (often shown as a row or annotation, e.g., "supplementary unit: 100 m²").
3. Return compact JSON inside the eval result (selected measures summary, plus the raw HTML of the measures panel as evidence).
4. Outside the eval, write the final `result.json`.

### Scrape command shape (replace selectors during Phase A discovery)

```js
(async () => {
  // 1. Verify form is present
  const goodsCodeInput = document.querySelector("input[name='Taric'], input[name='goods_code'], #taricNomGoodsCode"); // TBD during build
  if (!goodsCodeInput) {
    return JSON.stringify({ taric_error: "TARIC_FORM_NOT_FOUND", page_title: document.title });
  }

  // 2. Fill goods code
  const goodsCode = "7008002000"; // from job_parameters
  goodsCodeInput.value = goodsCode;
  goodsCodeInput.dispatchEvent(new Event("input", { bubbles: true }));
  goodsCodeInput.dispatchEvent(new Event("change", { bubbles: true }));

  // 3. Set origin dropdown (selector TBD during build)
  const originSelect = document.querySelector("select[name='origin'], #origin"); // TBD
  if (originSelect) {
    const optionValue = "CN"; // mapped from job_parameters.origin_country
    originSelect.value = optionValue;
    originSelect.dispatchEvent(new Event("change", { bubbles: true }));
  }

  // 4. Click "Retrieve Measures" button (selector TBD during build)
  const submitBtn = document.querySelector("button[type='submit'], input[value='Retrieve Measures']"); // TBD
  if (!submitBtn) {
    return JSON.stringify({ taric_error: "TARIC_SUBMIT_BUTTON_NOT_FOUND" });
  }
  submitBtn.click();

  // 5. Wait for result panel (replace with the actual loaded-event signal)
  await new Promise((resolve) => {
    const start = Date.now();
    const tick = () => {
      const panel = document.querySelector("#measureResult, .measure-result, table.measures"); // TBD
      if (panel && panel.querySelector("tr")) return resolve();
      if (Date.now() - start > 30000) return resolve(); // 30s max wait
      setTimeout(tick, 250);
    };
    tick();
  });

  // 6. Detect "not found"
  const bodyText = document.body.innerText;
  if (/no measures found|does not exist|no data found/i.test(bodyText)) {
    return JSON.stringify({
      taric_status: "not_found",
      queried_goods_code: goodsCode,
      cdp_state: "connected",
      captcha_intercepted: false,
      reason: "TARIC reports no measures for this goods code + origin combination"
    });
  }

  // 7. Scrape measures table
  const rows = Array.from(document.querySelectorAll("table.measures tr, .measure-row")); // TBD
  const measures = rows.map((row) => {
    const cells = Array.from(row.querySelectorAll("td")).map((c) => c.innerText.trim());
    return {
      measure_type: cells[0] || null,
      geographical_area: cells[1] || null,
      rate: cells[2] || null,
      conditions: cells[3] || null,
      footnotes: cells[4] || null
    };
  }).filter((m) => m.measure_type);

  // 8. Detect supplementary unit (often in a separate row or header annotation)
  const supplementaryUnitMatch = bodyText.match(/supplementary unit[:\s]+([^\n]+)/i);
  const supplementary_unit = supplementaryUnitMatch ? supplementaryUnitMatch[1].trim() : null;

  // 9. Detect AD/CVD presence
  const has_anti_dumping = measures.some((m) => /anti.?dumping/i.test(m.measure_type || ""));
  const has_countervailing = measures.some((m) => /countervail/i.test(m.measure_type || ""));
  const has_safeguard = measures.some((m) => /safeguard/i.test(m.measure_type || ""));

  // 10. Extract MFN third-country duty rate (the headline number)
  const mfnRow = measures.find((m) => /third country duty/i.test(m.measure_type || ""));
  const mfn_rate_text = mfnRow ? mfnRow.rate : null;

  return JSON.stringify({
    taric_status: "confirmed",
    queried_goods_code: goodsCode,
    cdp_state: "connected",
    captcha_intercepted: false,
    measures_count: measures.length,
    mfn_rate_text,
    supplementary_unit,
    has_anti_dumping,
    has_countervailing,
    has_safeguard,
    measures, // full array for evidence
    raw_evidence_text: bodyText.substring(0, 4000) // capped for size
  });
})()
```

This is a STRAW MAN for build discovery. Replace selectors and the result-panel
detection with the actual TARIC DOM structure observed during Phase A. The TARIC
old-UI selectors (in the PDF user guide) DIFFER from the new-UI selectors (in the
attached screenshot) — discover the new ones empirically.

### What to do with the data (cross-check semantics)

For the L5 multi-source-recall engine, the most useful fields are:

| Field | Cross-check use |
|---|---|
| `taric_status` | `confirmed` = EU TARIC knows the code; `not_found` = EU TARIC doesn't = disagreement signal vs HMRC |
| `measures_count` | 0 with `confirmed` is suspicious; expected >= 1 (third-country duty at minimum) |
| `supplementary_unit` | Cross-check vs HMRC's supplementary unit requirement — STRUCTURAL agreement signal |
| `has_anti_dumping` / `has_countervailing` / `has_safeguard` | Cross-check vs HMRC's `trade_remedy_state` — parallel-trade-remedy signal |
| `mfn_rate_text` | Cross-check vs HMRC's basic_duty_rate_bps — informational only (UK MFN can differ from EU MFN post-Brexit) |

The orchestrator code in `packages/intelligence/src/customs/multi-source-recall.ts`
will read this skill's output and populate the `candidateRecallAgreement` field on
`landed_cost_estimates` with the agreement/disagreement signals.

## UK-Specificity Rule (CONSTITUTIONAL — per L5 spec §1.4)

This skill returns EU TARIC data. EU TARIC is INFORMATIONAL for UK importation
purposes. NEVER use this skill's `mfn_rate_text`, `vat`, or any measure rate to
override HMRC's view in `landed_cost_estimates`. UK-authoritative measures
(DE 6/17 national additional codes, UK quotas, UK trade-remedy implementations)
are HMRC-only.

The orchestrator code treats this skill's output as a SECOND OPINION signal,
weighted in `commodity_confidence_bps` calibration but never overriding
HMRC-sourced numeric outputs.

## Auto-signin: NOT APPLICABLE for this skill

TARIC (`ec.europa.eu/taxation_customs/dds2/taric/`) is **free public EU customs
data** — no login mechanism exists, no session cookies are required, no API
key is needed. The `auth-signin` skill family (per
`docs/handoff-2026-05-17-cdp-auto-signin.md`) does NOT include a
`skills/auth-signin/taric/` skill, and this production skill does NOT include
a "Session Recovery" section. There is no logged-out state to detect or recover
from.

The CDP browser is still required (because the TARIC page is a JSP
server-rendered form, not a static page), but no authentication is performed.

If the skill ever encounters an unexpected login wall (which would indicate
either a TARIC infrastructure change or the operator's Chrome being WAF-blocked
for some other reason), emit `TARIC_UNEXPECTED_LOGIN_WALL` as a defensive
blocker — but this is purely speculative; TARIC has been a free public EU
resource since 1992 and is unlikely to ever gate access.

## Safety

Do not log in (there's no login). Do not change reference date casually — TARIC
historical lookups are part of the legitimate use, but consistency with HMRC's
`tariff_checked_at` matters; default to today unless `job_parameters.reference_date`
is explicitly set. Do not write server-side state. Do not bookmark or download
the TARIC Full database (multi-GB).

## Output
Write the final JSON object to `result.json` in the current working directory,
matching `result.schema.json`. `evidence`, `warnings`, and `blockers` must be
arrays; `warnings` and `blockers` entries must be objects with `code`, `message`,
and `severity`. Never emit bare string warning codes or omit severity. Also keep
the final response to the daemon as the same raw JSON object; do not wrap it in
Markdown.
```

---

## result.schema.json template (start here)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": [
    "product_candidate_id",
    "queried_goods_code",
    "taric_status",
    "captcha_intercepted",
    "cdp_state",
    "warnings",
    "blockers"
  ],
  "properties": {
    "source": { "type": "string" },
    "product_candidate_id": { "type": "string" },
    "queried_goods_code": { "type": "string", "description": "The 10-digit Combined Nomenclature code that was queried" },
    "queried_origin_country": { "type": ["string", "null"], "description": "ISO 3166-1 alpha-2" },
    "queried_destination_country": { "type": ["string", "null"] },
    "queried_additional_code": { "type": ["string", "null"] },
    "reference_date": { "type": ["string", "null"], "description": "ISO date used for the TARIC consultation" },
    "taric_last_update": { "type": ["string", "null"], "description": "From the page metadata 'Last TARIC update'" },
    "taric_status": {
      "type": "string",
      "enum": ["confirmed", "not_found", "blocked"],
      "description": "confirmed = code+origin returned measures; not_found = TARIC has no measures for this pair; blocked = couldn't complete the lookup"
    },
    "captcha_intercepted": { "type": "boolean", "default": false },
    "cdp_state": {
      "type": "string",
      "enum": ["connected", "unreachable", "captcha_blocked", "session_expired"],
      "default": "connected"
    },
    "measures_count": { "type": ["integer", "null"], "minimum": 0 },
    "mfn_rate_text": { "type": ["string", "null"], "description": "Third-country duty text exactly as TARIC displays it (e.g., '6.5 %' or '50 EUR / 100 kg net')" },
    "mfn_rate_bps": { "type": ["integer", "null"], "minimum": 0, "description": "Parsed ad valorem rate in basis points when mfn_rate_text is a simple percent; null when compound/specific" },
    "supplementary_unit": { "type": ["string", "null"], "description": "Supplementary unit description (e.g., 'm²', 'kg net', 'p/st')" },
    "supplementary_unit_required": { "type": ["boolean", "null"] },
    "has_anti_dumping": { "type": "boolean", "default": false },
    "has_countervailing": { "type": "boolean", "default": false },
    "has_safeguard": { "type": "boolean", "default": false },
    "has_tariff_quota": { "type": "boolean", "default": false },
    "has_tariff_preference": { "type": "boolean", "default": false },
    "has_suspension": { "type": "boolean", "default": false },
    "measures": {
      "type": "array",
      "description": "Full extracted measures table; each row has measure_type, geographical_area, rate, conditions, footnotes",
      "items": { "type": "object" }
    },
    "source_url": { "type": ["string", "null"], "description": "Direct deep-link to the TARIC consultation result if discoverable" },
    "evidence": { "type": ["array", "object", "null"] },
    "warnings": { "type": "array" },
    "blockers": { "type": "array" }
  }
}
```

---

## Step-by-step build guide

### Phase A: Discovery (~30-45 min, one-off)

1. Open Chrome with the operator's persistent CDP profile on port 9222. Navigate to `https://ec.europa.eu/taxation_customs/dds2/taric/taric_consultation.jsp?Lang=en`. Open DevTools Network tab + Elements tab.
2. Manually fill the form: Goods code `7008002000`, Origin/destination `CN`, leave others default. Click "Retrieve Measures". Observe:
   - Is the result delivered via full page reload (POST → server-render) or via AJAX (POST → JSON → client-render)?
   - What is the URL after submission? Is there a stable deep-link pattern (e.g., `?goods_code=7008002000&origin=CN`)?
   - Inspect the DOM of the rendered measures table. Note element selectors:
     - Form input names/IDs (Goods code input, Origin dropdown, Submit button).
     - Result panel container selector.
     - Per-measure-row selector.
     - Per-cell selectors (measure type, geographical area, rate, conditions, footnotes).
     - "No measures found" message (force the not-found path by querying a fake code like `9999999999`).
3. Note any session/cookie behavior. TARIC may set a `JSESSIONID` cookie — the persistent CDP browser handles this automatically.
4. Capture the "Last TARIC update" date from the top-right of the page.
5. Test the Geographical areas tab and Regulations tab briefly to see if anything useful (probably not for L5 skill v1 — leave for follow-up).
6. Test multiple known codes to confirm selectors are stable:
   - `7008002000` + `CN` (DGU canary — should return measures).
   - `7214200020` + `CN` (steel rebar — should return AD/CVD measures + supplementary unit `kg net`).
   - `1234567890` + `CN` (invalid code — should return "not found").
7. Build a country ISO-2 → TARIC option value mapping. Most ISO-2 codes match TARIC values directly; verify.
8. Document findings inline in the SKILL.md you're building (replace `TBD during build` placeholders with actual selectors).

### Phase B: Build the skill files (~45-60 min)

1. `mkdir -p skills/landed-cost/taric-cross-check`.
2. Create `SKILL.md` from the template above. Fill in:
   - Actual form-input selectors discovered in Phase A.
   - Actual result-panel selector + per-row selector.
   - Actual "not found" detection pattern.
   - Country ISO → TARIC value mapping (inline as a JS object literal in the eval).
   - Sample known-good outputs for `7008002000+CN` and `7214200020+CN` in the doc.
3. Create `result.schema.json` from the template above. Adjust property names/types to match your actual extraction.
4. (NO checkpoint.schema.json.)

### Phase C: Test / Validate / Fix Loop — empirical CDP execution (~1.5-3 hours iterative)

> **This is THE most important phase.** Building a SKILL.md from a template is the easy part; making the skill actually scrape live TARIC pages reliably is where most real work happens. **Do not skip iterations. Do not ship until 100% of the test matrix below passes.**
>
> **Mindset:** treat the skill like a test-driven feature. Each fixture is a test case. Each failure is a debugging cycle. Each green run is one row of evidence in the final `TEST_REPORT.md`. The loop exits only when every row is green AND the failure-path matrix is fully covered, AND the HMRC cross-agreement signal is empirically verified.

#### C.0 — Set up the test environment (~15 min)

##### C.0.a — Verify you're using the RIGHT Chrome (do NOT skip)

The skill assumes a SPECIFIC persistent Chrome is running on port 9222. Even though TARIC needs no login, the persistent Chrome carries the right user-agent, geolocation, and timezone that EU TARIC's WAF expects from a normal user. A fresh headless Chrome on 9222 may get rate-limited or challenged. Verify BEFORE doing anything else:

```bash
# 0. Auto-launch (if needed). Idempotent: <1s no-op when CDP is already up;
#    launches Chrome with the operator's /Users/alinaaitova/.chrome-automation
#    profile and polls up to 15s if not. Exit non-zero only on genuine failure.
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222

# 1. CDP is up
curl -sfS -m 3 "http://localhost:9222/json/version" | jq '{browser: .Browser, userAgent: .["User-Agent"], debuggerUrl: .webSocketDebuggerUrl}'
# Expected: Browser like "Chrome/140.x.x.x" or newer. The helper above auto-launches
# the persistent Chrome with the canonical flags. Manual launch is below for reference
# (first-time profile creation / recovery from a broken profile state):
#   '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' \
#     --remote-debugging-port=9222 \
#     '--remote-allow-origins=*' \
#     --user-data-dir=/Users/alinaaitova/.chrome-automation \
#     --profile-directory=Default --no-first-run --no-default-browser-check &
# (Note the quoted '--remote-allow-origins=*' to prevent zsh glob expansion.)

# 2. Confirm the Chrome can reach ec.europa.eu (no corporate proxy blocking)
curl -s -o /dev/null -w "%{http_code}\n" "https://ec.europa.eu/taxation_customs/dds2/taric/taric_consultation.jsp?Lang=en"
# Expected: 200. If 403/503: WAF challenge or geolocation block — investigate before continuing.

# (TARIC has NO login — skip the avatar/session check that Freightos requires.)
```

If any check fails, **stop** and fix Chrome state before proceeding.

##### C.0.b — Working directory + schema validator

```bash
# Working test directory
mkdir -p /tmp/taric-build-test
cd /tmp/taric-build-test

# Copy the schema for validation
cp /Users/alinaaitova/Desktop/Testproject1/skills/landed-cost/taric-cross-check/result.schema.json .

# Install JSON Schema validator (PICK ONE — must match what the Freightos handoff used
# for consistency across L5 operator-built skills):
bun add -g ajv-cli ajv-formats
ajv --version   # record in TEST_REPORT.md
# OR
python3 -m pip install --user jsonschema
python3 -c "import jsonschema; print(jsonschema.__version__)"   # record in TEST_REPORT.md
```

Record in TEST_REPORT.md "Environment" section: bash version, jq version, ajv-cli OR jsonschema version, Chrome version (from curl /json/version), agent-browser version. This makes the test evidence reproducible.

##### C.0.c — Test-evidence directory + eval scratchpad

```bash
mkdir -p /Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/taric-cross-check

# Local scratchpad for the WIP eval JS body during iteration
touch /tmp/taric-build-test/eval.js
```

The eval body is edited many times during the loop. Source it as `agent-browser --cdp 9222 eval "$(cat /tmp/taric-build-test/eval.js)"` and re-edit the file between iterations. Copy the FINAL working version into SKILL.md when C.3 stabilizes.

#### C.1 — Define the canary input matrix (~10 min)

Save `canary.inputs.jsonl` to `test-evidence/taric-cross-check/canary.inputs.jsonl`. **At least these 5 cases** — each line is one canary covering a distinct cross-check signal:

```jsonl
{"id":"canary-01-dgu-confirmed","product_candidate_id":"test-dgu-001","goods_code":"7008002000","origin_country":"CN","destination_country":"GB","additional_code":null,"reference_date":null}
{"id":"canary-02-steel-rebar-ad","product_candidate_id":"test-rebar-001","goods_code":"7214200020","origin_country":"CN","destination_country":"GB","additional_code":null,"reference_date":null}
{"id":"canary-03-fasteners-cn","product_candidate_id":"test-fasteners-001","goods_code":"7318159081","origin_country":"CN","destination_country":"GB","additional_code":null,"reference_date":null}
{"id":"canary-04-gloves-medical","product_candidate_id":"test-gloves-001","goods_code":"4015120000","origin_country":"MY","destination_country":"GB","additional_code":null,"reference_date":null}
{"id":"canary-05-not-found","product_candidate_id":"test-invalid-001","goods_code":"9999999990","origin_country":"CN","destination_country":"GB","additional_code":null,"reference_date":null}
```

Why these 5:
- **canary-01** (`7008002000+CN`): the L5 DGU canary that HMRC already classifies; confirms basic happy path + agreement with HMRC's 0% MFN expectation.
- **canary-02** (`7214200020+CN`): steel rebar from China; **must** detect EU anti-dumping measures + supplementary unit `kg net`. This is the highest-signal cross-check fixture.
- **canary-03** (`7318159081+CN`): screws/bolts from China; another remedy-active commodity (often has AD measures). Validates fastener gold-set category for L5.
- **canary-04** (`4015120000+MY`): examination gloves from Malaysia (CN8 4015 = rubber gloves); validates regulated-goods gold-set category, non-CN origin path.
- **canary-05** (`9999999990+CN`): invalid code; verifies the `not_found` path is a SIGNAL not a blocker.

Save `golden.assertions.yaml` to the same directory:

```yaml
# Golden assertions are signal-shape based — TARIC measure rates can change quarterly.
# These verify the skill captures the RIGHT KINDS of signals reliably.

canary-01-dgu-confirmed:
  taric_status: confirmed
  cdp_state: connected
  captcha_intercepted: false
  measures_count:
    min: 1
  supplementary_unit:
    must_exist: true   # 7008 codes generally require m²
  blockers_length: 0

canary-02-steel-rebar-ad:
  taric_status: confirmed
  cdp_state: connected
  has_anti_dumping: true              # MUST be true — Chinese steel rebar has EU AD
  supplementary_unit:
    must_exist: true
    must_contain: "kg"                # supplementary unit should mention kg
  measures_count:
    min: 2                            # third-country duty + AD at minimum
  blockers_length: 0

canary-03-fasteners-cn:
  taric_status: confirmed
  cdp_state: connected
  measures_count:
    min: 1
  blockers_length: 0
  # AD/CVD presence is variable for fasteners; record but don't assert.

canary-04-gloves-medical:
  taric_status: confirmed
  cdp_state: connected
  measures_count:
    min: 1
  blockers_length: 0

canary-05-not-found:
  taric_status: not_found
  cdp_state: connected
  captcha_intercepted: false
  blockers_length: 0                  # not_found is a WARNING, not a blocker
  warnings_length:
    min: 1                            # should warn about the missing code
```

#### C.2 — Define the failure-path matrix (~5 min)

Save `failure-paths.jsonl`:

```jsonl
{"id":"fp-01-captcha-sim","scenario":"Pass simulate_captcha=true; verify short-circuit","input_override":{"simulate_captcha":true},"expect":{"taric_status":"blocked","cdp_state":"captcha_blocked","captcha_intercepted":true,"blockers_length":1,"blocker_codes":["CDP_CAPTCHA_INTERCEPTION"]}}
{"id":"fp-02-cdp-down","scenario":"Stop CDP browser before running; verify CDP_CONNECT_FAILED","setup":"Stop Chrome / kill --remote-debugging-port=9222 process","input_override":{},"expect":{"taric_status":"blocked","cdp_state":"unreachable","captcha_intercepted":false,"blockers_length":1,"blocker_codes":["CDP_CONNECT_FAILED"]}}
{"id":"fp-03-malformed-code","scenario":"Pass non-numeric goods_code; verify graceful TARIC_FORM_REJECTED","input_override":{"goods_code":"NOT-A-CODE"},"expect":{"taric_status":"blocked","blocker_codes":["TARIC_FORM_REJECTED"],"any_blocker_code_matches":true}}
{"id":"fp-04-historical-date","scenario":"Query with reference_date=2019-01-01 (pre-Brexit baseline); verify TARIC still responds with that historical view","input_override":{"reference_date":"2019-01-01","goods_code":"7008002000"},"expect":{"taric_status":"confirmed","cdp_state":"connected","reference_date":"2019-01-01"}}
```

Note: fp-04 is not a true failure path — it's a date-handling regression test to confirm the reference-date input works (TARIC supports historical queries).

#### C.3 — Execute the loop (iterate until 100% pass)

Per-canary cycle:

```bash
EVID_DIR=/Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/taric-cross-check
CANARY_ID=canary-01-dgu-confirmed   # change per iteration

cd /tmp/taric-build-test
rm -f result.json

# 1. Extract canary input
jq -c --arg id "$CANARY_ID" 'select(.id == $id)' "$EVID_DIR/canary.inputs.jsonl" > job_parameters.json
cat job_parameters.json

# 2. Execute the 3-command happy path from SKILL.md:
# Auto-launch + health check (helper is idempotent, <1s no-op when CDP up).
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222 || { echo "CDP DOWN"; exit 1; }
curl -sfS -m 3 "http://localhost:9222/json/version" > /dev/null || { echo "CDP DOWN"; exit 1; }
agent-browser --cdp 9222 open "https://ec.europa.eu/taxation_customs/dds2/taric/taric_consultation.jsp?Lang=en"

# The eval body lives in /tmp/taric-build-test/eval.js during iteration.
# It must read the goods_code/origin_country values from canary input (you can hardcode
# during iteration, then templatize once stable). The eval returns JSON.stringify({...}).
agent-browser --cdp 9222 eval "$(cat /tmp/taric-build-test/eval.js)" > result.json

# If the output isn't valid JSON (e.g., has stray whitespace or shell-injected text),
# clean it up: jq . result.json > result.tmp && mv result.tmp result.json

# 4. Schema validation
ajv validate -s result.schema.json -d result.json --strict=false --all-errors

# 5. Golden assertions (same python pattern as Freightos handoff C.3; adjusted for taric fields)
python3 - <<'PYEOF'
import json, yaml, sys
result = json.load(open('result.json'))
golden = yaml.safe_load(open('/Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/taric-cross-check/golden.assertions.yaml'))
canary_id = "canary-01-dgu-confirmed"   # change per iteration
expectations = golden[canary_id]
failures = []
for key, spec in expectations.items():
    actual = result.get(key)
    if isinstance(spec, dict):
        if 'min' in spec and (actual is None or actual < spec['min']):
            failures.append(f"{key} ({actual}) < min ({spec['min']})")
        if 'max' in spec and actual is not None and actual > spec['max']:
            failures.append(f"{key} ({actual}) > max ({spec['max']})")
        if spec.get('must_exist') and not actual:
            failures.append(f"{key} must exist but is null/empty")
        if 'must_contain' in spec and (not actual or spec['must_contain'].lower() not in str(actual).lower()):
            failures.append(f"{key} ({actual!r}) must contain '{spec['must_contain']}'")
    elif key == 'blockers_length':
        if len(result.get('blockers', [])) != spec:
            failures.append(f"blockers_length ({len(result.get('blockers', []))}) != {spec}")
    elif key == 'warnings_length':
        if isinstance(spec, dict) and 'min' in spec:
            if len(result.get('warnings', [])) < spec['min']:
                failures.append(f"warnings_length ({len(result.get('warnings', []))}) < min ({spec['min']})")
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

**Iteration cycle is the same shape as Freightos C.3:**

```
PICK fixture → EXECUTE → VALIDATE → [PASS: save evidence + next] / [FAIL: diagnose + fix SKILL.md + re-execute SAME fixture]
```

**Loop exit gate:** all 5 canaries have `*-status.txt` = "PASS" in the evidence directory.

#### C.4 — Execute the failure-path matrix (~20 min)

Same pattern as Freightos handoff C.4. Run in order:
1. fp-01-captcha-sim (no browser changes)
2. fp-03-malformed-code (no browser changes)
3. fp-04-historical-date (no browser changes; recovery test)
4. fp-02-cdp-down (stop browser)
5. (Restart browser if you want to do anything else after)

Each failure-path test produces `*-result.json` + `*-status.txt`.

#### C.5 — Cross-agreement test with HMRC (~30-45 min) — CRITICAL for L5

This step verifies the SEMANTIC purpose of the skill (cross-checking HMRC) actually works. The L5 multi-source-recall engine relies on this agreement signal — if the skill returns garbage that never agrees with HMRC, the agreement matrix is useless.

Run for each of canary-01 (`7008002000+CN`) and canary-02 (`7214200020+CN`) — these are the two highest-signal cases. canary-03/04/05 are documented as informational only.

```bash
mkdir -p /tmp/hmrc-cross-check && cd /tmp/hmrc-cross-check
cp /Users/alinaaitova/Desktop/Testproject1/skills/landed-cost/hmrc-customs-classification/result.schema.json .

# 1. Fetch HMRC raw response for canary-01 (DGU from CN)
curl -g -fsSL 'https://www.trade-tariff.service.gov.uk/uk/api/commodities/7008002000?filter[geographical_area_id]=CN' | jq . > hmrc_canary_01.json

# 2. Fetch HMRC raw response for canary-02 (steel rebar from CN)
curl -g -fsSL 'https://www.trade-tariff.service.gov.uk/uk/api/commodities/7214200020?filter[geographical_area_id]=CN' | jq . > hmrc_canary_02.json

# 3. Run the agreement scorer (writes cross-agreement-report.json)
python3 - <<'PYEOF'
import json, re

EVID_DIR = '/Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/taric-cross-check'

# ---- HMRC extractors (work against the public Trade Tariff API response shape) ----

def extract_hmrc_signals(hmrc_raw):
    """
    Walk hmrc_raw['included'] for measure_type / measure_component / measure entries
    and surface the third-country (MFN) duty rate, supplementary unit code, and
    presence of anti-dumping (AD), countervailing (CVD), safeguard measures.

    Trade Tariff measure_type IDs of interest:
      - 103 = Third-country duty (MFN)
      - 109 = Tariff preference
      - 105 = Anti-dumping duty (ADD)
      - 305 = Supplementary unit
      - 552 = Safeguard
    """
    description = hmrc_raw.get('data', {}).get('attributes', {}).get('description_plain', '')
    included = hmrc_raw.get('included', [])

    # Index measure_components by id, and measure_types by id, so we can resolve refs
    components = {it['id']: it for it in included if it.get('type') == 'measure_component'}
    measure_types = {it['id']: it for it in included if it.get('type') == 'measure_type'}
    measures = [it for it in included if it.get('type') == 'measure']

    mfn_rate_text = None
    supp_unit_code = None
    has_ad = False
    has_cvd = False
    has_safeguard = False

    for m in measures:
        mt_id = (m.get('relationships', {})
                  .get('measure_type', {})
                  .get('data', {})
                  .get('id'))
        mt = measure_types.get(mt_id) or {}
        mt_desc = (mt.get('attributes', {}).get('description') or '').lower()

        if mt_id == '103':  # MFN
            mfn_rate_text = m.get('attributes', {}).get('duty_expression', {}).get('base') \
                or m.get('attributes', {}).get('duty_expression_str')
        if mt_id == '305' or 'supplementary' in mt_desc:
            # Walk this measure's components for the unit
            comp_ids = [c['id'] for c in m.get('relationships', {}).get('measure_components', {}).get('data', [])]
            for cid in comp_ids:
                c_attrs = (components.get(cid) or {}).get('attributes', {}) or {}
                if c_attrs.get('measurement_unit_code'):
                    supp_unit_code = c_attrs['measurement_unit_code']
                    break
        if 'anti-dumping' in mt_desc or mt_id == '105':
            has_ad = True
        if 'countervail' in mt_desc:
            has_cvd = True
        if 'safeguard' in mt_desc:
            has_safeguard = True

    return {
        'description': description,
        'mfn_rate_text': mfn_rate_text,
        'supplementary_unit_code': supp_unit_code,
        'has_anti_dumping': has_ad,
        'has_countervailing': has_cvd,
        'has_safeguard': has_safeguard,
    }

# ---- Agreement scoring rules (these become the L5 orchestrator's logic too) ----

UNIT_ALIASES = {
    # HMRC code => TARIC display string equivalences. Extend as discovered.
    'MTK': ['m²', 'm2', 'square metre', 'square meter'],
    'KGM': ['kg', 'kilogram', 'kg net'],
    'LTR': ['l', 'litre', 'liter'],
    'NAR': ['p/st', 'number'],
}

def units_agree(hmrc_code, taric_text):
    if not hmrc_code or not taric_text:
        return None  # cannot score
    aliases = UNIT_ALIASES.get(hmrc_code.upper(), [hmrc_code.lower()])
    return any(a in taric_text.lower() for a in aliases)

def mfn_pct_extract(text):
    """Extract a numeric percent from 'X.X %' / 'X %' / 'X.X%' / '0.00%'."""
    if not text: return None
    m = re.search(r'([0-9]+(?:\.[0-9]+)?)\s*%', str(text))
    return float(m.group(1)) if m else None

def mfn_agree(hmrc_text, taric_text, tolerance_pp=0.5):
    """Both must be percent expressions; agree if within tolerance_pp percentage points."""
    h = mfn_pct_extract(hmrc_text)
    t = mfn_pct_extract(taric_text)
    if h is None or t is None: return None
    return abs(h - t) <= tolerance_pp

def remedy_agree(hmrc_flag, taric_flag):
    if hmrc_flag is None or taric_flag is None: return None
    return bool(hmrc_flag) == bool(taric_flag)

def score_overall(supp_unit_ok, mfn_ok, remedy_ok):
    signals = [s for s in (supp_unit_ok, mfn_ok, remedy_ok) if s is not None]
    if not signals: return 'UNSCORABLE'
    agrees = sum(1 for s in signals if s)
    if agrees == len(signals): return 'STRONG_AGREEMENT'
    if agrees >= len(signals) / 2: return 'PARTIAL_AGREEMENT'
    return 'DISAGREEMENT'

# ---- Run for both canaries ----

results = []
for canary_id, hmrc_file in [
    ('canary-01-dgu-confirmed', 'hmrc_canary_01.json'),
    ('canary-02-steel-rebar-ad', 'hmrc_canary_02.json'),
]:
    hmrc_raw = json.load(open(hmrc_file))
    taric = json.load(open(f'{EVID_DIR}/{canary_id}-result.json'))
    hmrc = extract_hmrc_signals(hmrc_raw)

    supp_ok = units_agree(hmrc.get('supplementary_unit_code'), taric.get('supplementary_unit'))
    mfn_ok = mfn_agree(hmrc.get('mfn_rate_text'), taric.get('mfn_rate_text'))
    ad_ok = remedy_agree(hmrc.get('has_anti_dumping'), taric.get('has_anti_dumping'))
    verdict = score_overall(supp_ok, mfn_ok, ad_ok)

    row = {
        'canary': canary_id,
        'hmrc': hmrc,
        'taric': {k: taric.get(k) for k in ['taric_status','mfn_rate_text','supplementary_unit','has_anti_dumping','has_countervailing','has_safeguard']},
        'agreement': {
            'supp_unit': supp_ok,        # True / False / None
            'mfn_duty':  mfn_ok,
            'trade_remedy_presence': ad_ok,
            'overall_verdict': verdict,  # STRONG_AGREEMENT / PARTIAL_AGREEMENT / DISAGREEMENT / UNSCORABLE
        },
    }
    results.append(row)
    print(f"\n=== {canary_id} ===")
    print(json.dumps(row, indent=2, default=str))

# 4. Write a machine-readable report
json.dump(results, open(f'{EVID_DIR}/hmrc-cross-agreement.json', 'w'), indent=2, default=str)
print(f"\nWrote {EVID_DIR}/hmrc-cross-agreement.json")
PYEOF
```

##### C.5 Agreement scoring rules (codify so step 09 audit can read TEST_REPORT objectively)

| Signal | Rule | Score values |
|---|---|---|
| **Supplementary unit** | HMRC code (MTK, KGM, LTR, NAR…) matches TARIC display string via `UNIT_ALIASES` dict | True (exact code↔string match) / False (mismatch) / None (one side absent) |
| **MFN duty** | Both extractable as percent; agree if within ±0.5 percentage points | True / False / None |
| **Trade-remedy presence** | HMRC `has_anti_dumping` boolean == TARIC `has_anti_dumping` boolean | True / False / None |
| **Overall verdict** | All 3 signals True → STRONG_AGREEMENT; majority True → PARTIAL; else DISAGREEMENT; all None → UNSCORABLE | enum |

**Expected results given the L5 use case:**
- canary-01 (7008002000+CN, sealed insulating glass): expect STRONG_AGREEMENT — both sources should agree on MTK/m², 0% MFN, no AD.
- canary-02 (7214200020+CN, steel rebar): expect at minimum PARTIAL_AGREEMENT — both should agree on KGM/kg and AD presence; MFN may differ (UK and EU steel-product rates diverged post-Brexit).

**If the actual result is DISAGREEMENT for canary-01:** something is wrong with the TARIC skill's parsing. Loop back to C.3, re-diagnose. Do NOT proceed to C.6 with a broken cross-agreement signal — the L5 orchestrator will produce garbage `candidateRecallAgreement` data.

Record the `hmrc-cross-agreement.json` artifact in TEST_REPORT.md §"HMRC cross-agreement" with the per-canary scores AND the overall verdict. Surface any DISAGREEMENTs as warnings.

#### C.6 — Write the TEST_REPORT.md (~15 min)

Save to `test-evidence/taric-cross-check/TEST_REPORT.md`:

```markdown
# TARIC Cross-Check CDP Browser Skill — Test Report

**Skill:** `skills/landed-cost/taric-cross-check/`
**Skill version:** 1.0.0
**Tested by:** <operator name or agent name>
**Test date:** YYYY-MM-DD
**CDP port:** 9222
**TARIC last update at time of test:** <from page header, e.g., 15-05-2026>

## Verdict

**STATUS: READY FOR L5 STEP 09 BEAD AUDIT** (or BLOCKED if any test failed)

## Canary results (5/5 must pass)

| Canary | Code + Origin | Expected signal | Result | Runtime | measures_count | Notes |
|---|---|---|---|---|---|---|
| canary-01 | 7008002000+CN | confirmed + supp unit m² | PASS | 7s | 4 | matches HMRC's DGU classification |
| canary-02 | 7214200020+CN | has_anti_dumping=true + supp unit kg | PASS | 8s | 6 | EU AD on Chinese steel confirmed |
| canary-03 | 7318159081+CN | confirmed | PASS | 7s | 3 | fasteners; AD signal: <yes/no> |
| canary-04 | 4015120000+MY | confirmed | PASS | 7s | 2 | gloves from MY (non-CN origin works) |
| canary-05 | 9999999990+CN | taric_status=not_found | PASS | 6s | 0 | not_found is a SIGNAL, not blocker |

## Failure-path results (4/4 must pass)

| ID | Scenario | Result | cdp_state | blocker_code | Notes |
|---|---|---|---|---|---|
| fp-01 | Captcha simulation | PASS | captcha_blocked | CDP_CAPTCHA_INTERCEPTION | short-circuit works |
| fp-02 | CDP down | PASS | unreachable | CDP_CONNECT_FAILED | health check correctly fires |
| fp-03 | Malformed code (non-numeric) | PASS | connected | TARIC_FORM_REJECTED | form validation works |
| fp-04 | Historical reference_date | PASS | connected | (none) | reference_date plumbing works |

## HMRC cross-agreement (semantic verification — the WHY of this skill)

Populated empirically from `test-evidence/taric-cross-check/hmrc-cross-agreement.json` (produced by the C.5 scorer script):

| Canary | HMRC supp unit | TARIC supp unit | supp_unit agree? | HMRC MFN | TARIC MFN | mfn_duty agree? | HMRC AD | TARIC AD | remedy agree? | Overall verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| canary-01 (7008002000+CN) | MTK | m² | ✓ True | 0.00% | 0% | ✓ True | False | False | ✓ True | **STRONG_AGREEMENT** |
| canary-02 (7214200020+CN) | KGM | kg net | ✓ True | (varies) | (varies) | (per scorer) | True | True | ✓ True | **STRONG_AGREEMENT** or PARTIAL |

(Replace the example values with the actual scorer output. If any row shows DISAGREEMENT, surface it as a warning and document the suspected cause.)

The L5 orchestrator at `packages/intelligence/src/customs/multi-source-recall.ts` will use these agreement signals to populate `candidateRecallAgreement` on `landed_cost_estimates`. A skill that produces DISAGREEMENT on canary-01 (where the underlying data is identical between EU and UK) is BROKEN and must be re-diagnosed before shipping.

## Discoveries from Phase A (live selectors)

| Item | Value |
|---|---|
| Form submission mechanism | <full page reload / AJAX / partial reload> |
| Goods code input selector | `<actual CSS or XPath>` |
| Origin/destination select selector | `<actual CSS or XPath>` |
| Submit button selector | `<actual CSS or XPath>` |
| Result panel selector | `<actual CSS or XPath>` |
| Per-measure-row selector | `<actual CSS or XPath>` |
| Measure-type cell index/selector | `<actual>` |
| Supplementary unit detection pattern | `<regex or DOM path>` |
| "Not found" detection pattern | `<regex matched on body text>` |
| Country code → option value mapping | (inline as JS object) `{ "CN": "...", "MY": "...", "GB": "...", "DE": "...", ... }` |

## Quirks observed

- <e.g., "TARIC backend redirects to ?Lang=en if Accept-Language header is non-EU">
- <e.g., "additional_code field is auto-filled to '0000' on submit; clear before extraction">

## Schema validation

All 9 result.json files (5 canaries + 4 failure-paths) validated successfully against `result.schema.json`.

## Sign-off

Operator confirms the TARIC cross-check skill is ready for L5 implementation. UK-Specificity rule honored — skill never recommends overriding HMRC for UK calculations. Skill files committed at SHA `<git sha>`.
```

#### C.7 — Commit + push (~5 min)

```bash
cd /Users/alinaaitova/Desktop/Testproject1
git add skills/landed-cost/taric-cross-check/
git add docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/taric-cross-check/
git commit -m "taric cross-check skill: initial CDP-browser implementation + test evidence (5/5 canaries pass, 4/4 failure paths pass, HMRC cross-agreement verified)"
git pull --rebase && git push
```

#### C.8 — Loop-exit gate (BLOCKING — verify before Phase D)

- [ ] All 5 canaries have `*-status.txt` = "PASS".
- [ ] All 4 failure paths have `*-status.txt` = "PASS".
- [ ] HMRC cross-agreement section in TEST_REPORT.md is populated (not left as `<...>`).
- [ ] `TEST_REPORT.md` exists and shows verdict READY.
- [ ] All 9 result.json files validate against result.schema.json.
- [ ] Test evidence pushed to git.

**If ANY checkbox is unchecked, return to C.3 / C.4 / C.5 and iterate.** Do NOT advance to Phase D.

### Phase D: Integration smoke (~15 min)

1. Verify the skill directory matches production standard.
2. Stage: `git add skills/landed-cost/taric-cross-check/`.
3. Run `ubs skills/landed-cost/taric-cross-check/SKILL.md` if available — fix any flagged issues.
4. Commit: `git commit -m "taric cross-check skill: initial CDP-browser implementation for L5 multi-source recall"`.
5. Push.

---

## Verification checklist (operator runs before declaring complete)

### Build verification

- [ ] Skill directory exists at `skills/landed-cost/taric-cross-check/` with both files (SKILL.md + result.schema.json).
- [ ] `SKILL.md` has YAML frontmatter with all required fields.
- [ ] SKILL.md describes the 3-command bounded happy path (CDP check, open URL, eval call).
- [ ] SKILL.md includes the captcha simulation hook verbatim.
- [ ] SKILL.md lists all TARIC-specific blocker/status codes.
- [ ] SKILL.md includes the "UK-Specificity Rule (CONSTITUTIONAL)" section warning the orchestrator never to override HMRC with TARIC data.
- [ ] `result.schema.json` includes the 7 required fields.
- [ ] No setup/discovery commands in the success path.
- [ ] Skill never writes server-side state, never logs in (TARIC has no login), never bookmarks.

### Empirical Test/Validate/Fix loop (CDP must be live during this section)

- [ ] `test-evidence/taric-cross-check/canary.inputs.jsonl` exists with ≥5 canaries.
- [ ] `test-evidence/taric-cross-check/golden.assertions.yaml` exists with expectations per canary.
- [ ] `test-evidence/taric-cross-check/failure-paths.jsonl` exists with ≥4 failure scenarios.
- [ ] All 5 canary `*-result.json` files exist and validate against schema.
- [ ] All 5 canary `*-status.txt` files contain "PASS".
- [ ] All 4 failure-path `*-status.txt` files contain "PASS".
- [ ] Skill captures `supplementary_unit` correctly for codes that require it (canary-02 with steel rebar).
- [ ] Skill correctly detects AD/CVD for Chinese steel rebar (canary-02 `has_anti_dumping=true`).
- [ ] Skill returns `taric_status: "not_found"` with warning (NOT blocker) for canary-05 (invalid code).
- [ ] HMRC cross-agreement section in TEST_REPORT.md is populated with empirical comparisons (not placeholders).
- [ ] `TEST_REPORT.md` exists with verdict READY and all rows populated.
- [ ] Skill runs end-to-end in <120s on the happy-path canaries.
- [ ] Phase A discovery findings (selectors, country-code mapping) are documented in TEST_REPORT.md.

### Integration verification

- [ ] All test evidence committed and pushed.
- [ ] Skill files committed and pushed.
- [ ] L5 step 09 bead audit can `ls skills/landed-cost/taric-cross-check/` and find SKILL.md + result.schema.json present.

---

## L5 pipeline integration (what happens after this skill ships)

1. Step 07 bead creation: Phase 4 Customs engine beads will REFERENCE this skill by path (`skills/landed-cost/taric-cross-check/`). Specifically:
   - Bead `l5-phase-4-customs-multi-source-recall` will say "for each candidate commodity code from HMRC, invoke landed-cost/taric-cross-check; populate `candidateRecallAgreement` with the {hmrc, taric, cpv_priors, rule_based_hints, atr_codebook} agreement matrix; if Taric agrees on supplementary_unit and trade-remedy presence, increment commodity_confidence_bps."
   - Bead `l5-phase-4-customs-classification-ambiguous-emission` will say "if Taric returns `not_found` for a code HMRC classified, this is a divergence signal — incorporate into the CLASSIFICATION_AMBIGUOUS emission decision."
2. Step 09 bead audit: verifies `skills/landed-cost/taric-cross-check/SKILL.md` exists on disk before authorizing step 10 execution. If missing, audit FAILS.
3. Step 12 completion audit: verifies the Taric skill was successfully invoked at least N times by the L5 Phase 4 customs engine during Stage 11 live demo against at least 2 of the 3 gold-set categories (fasteners, packaging-cartons, examination-gloves).

---

## Reference materials in this handoff

### TARIC user guide (PDF, 107 pages, old UI but authoritative on concepts)

Persisted in the project at:
`docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/reference/Taric_user_guide.pdf`

(Originally provided by the operator at `/Users/alinaaitova/Desktop/Taric_user_guide.pdf` — copied into the project so the build agent has stable access regardless of Desktop state.)

UI layout differs from current production but the concepts are authoritative. Reference it for:
- The full taxonomy of measure types
- How geographical areas are coded (e.g., ERGA OMNES, country groups)
- Footnote / legal-act reference conventions
- Tariff quota and additional code semantics
- Advanced search options (most not needed for v1)

### Persisted new-UI screenshot

`docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/screenshots/taric/taric-consultation-new-ui-2026-05-17.png` — shows the current production UI: Measures/Geographical areas/Regulations tabs, Goods code + Additional code + Origin/destination inputs, Retrieve Measures button, reference date picker, "Last TARIC update" header.

Open it in Preview alongside DevTools during Phase A discovery — it's the canonical reference for current selectors.

### Reference skills (re-paste for convenience)

- `skills/landed-cost/searates-freight-estimate/` (CDP discipline template)
- `skills/landed-cost/hmrc-customs-classification/` (classification-result template, and the skill this one cross-checks)

### Time estimate (revised based on empirical CDP testing requirement)

| Phase | Effort | What |
|---|---|---|
| A — Discovery | 30-60 min | DevTools network/DOM exploration; build country-code mapping |
| B — Build skill files | 45-60 min | Write SKILL.md + result.schema.json from templates |
| **C — Test/Validate/Fix loop (incl. HMRC cross-agreement)** | **1.5-3 hours** | **Iterate 5 canaries + 4 failure paths + HMRC cross-check; write TEST_REPORT.md** |
| D — Integration smoke | 15 min | Final commit + push |
| **Total** | **~3-5 hours** | (vs. earlier estimate of ~2-3 hours — empirical CDP iteration + HMRC cross-agreement verification is the actual work) |

---

## Notes for the next agent (if delegating)

If you (operator) delegate this build to a coding agent:

- The agent MUST read both `skills/landed-cost/searates-freight-estimate/SKILL.md` and `skills/landed-cost/hmrc-customs-classification/SKILL.md` end-to-end FIRST.
- The agent MUST do the Phase A discovery in a real Chrome session with DevTools open against the LIVE current TARIC UI — the PDF user guide reflects the OLD UI and selectors WILL differ.
- The agent MUST treat the UK-Specificity Rule as constitutional — never recommend the skill's output be used to override HMRC for UK calculations.
- The agent MUST test against the 3 known codes listed in Phase C (DGU `7008002000`, steel rebar `7214200020`, invalid `9999999999`).
- The agent MUST NOT use mocks — the real TARIC database is the truth.
- The agent MUST commit and push when done so step 09 bead audit can verify.

---

## End of handoff

When the skill is built, tested against the 3 known codes, cross-checked against HMRC for agreement signals, and pushed to main, mark this handoff complete. Both this and the Freightos handoff must be complete before L5 step 09 bead audit can pass.
