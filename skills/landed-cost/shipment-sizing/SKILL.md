---
skill_id: landed-cost/shipment-sizing
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

# landed-cost/shipment-sizing

## Mission
Resolve the shipment basis required before any freight quote: units, total weight in kg, total cubic metres, package count, and LCL/FCL/container plan.

## Inputs
The daemon supplies the same candidate and selected quote context used by the HMRC skill, plus the selected supplier cost basis and `inferred_order_quantity_basis`.

`inferred_order_quantity_basis` is the operational shipment quantity. It has `kind`, `qty`, `unit`, and `source` fields. Size freight from this field, not from `quote.quoted_quantity`, unless `kind == "supplier_capacity"`. Treat `quote.quoted_quantity` as supplier listing tier/capacity evidence unless the daemon explicitly selected it as the order basis.

## Stage 11 Bounded Runner Discipline
The daemon kills this skill after a fixed timeout. Start with the supplied
`job_parameters`; do not run setup or discovery commands such as `pwd`, `ls`,
`rg`, `find`, schema printing, Agent Mail, repository searches, broad browser
snapshots, or CLI help output. The workdir already contains
`result.schema.json`, and this SKILL.md defines the required result shape.

Use at most three shell commands on the import success path:

1. `curl` CDP check, only when the supplied quote URL requires browser evidence.
2. One `agent-browser --cdp ${CDP_PORT}` command to open/extract the listing, if
   the supplied evidence is insufficient.
3. `date -u` only if you need a timestamp.

After those commands, immediately write `result.json`. If the fast path cannot
size honestly, write a structured blocked result immediately; never leave the
workdir without `result.json`.

## DGU / IGU Flat-Glass Fast Path
For sealed insulating glass, double glazed glass, DGU, or IGU quoted per square
metre, use this deterministic path when the supplied quote/candidate or same
listing evidence includes thickness such as `6mm+9A+6mm`:

- Size from `inferred_order_quantity_basis`. For the DGU operator override this
  is `250 square meter`.
- Interpret `6mm+9A+6mm` as 12 mm of glass for weight and 21 mm total unit
  thickness for geometric volume.
- Use glass density 2,500 kg/m3.
- Unit weight per square metre: `0.012 m * 2500 kg/m3 = 30 kg`.
- Unit geometric volume per square metre: `0.021 cbm`.
- For 250 m2: total weight `7,500 kg`; geometric volume `5.25 cbm`.
- If there is no project-specific packing schedule, add a 50% crating/handling
  volume uplift, making planning volume `7.875 cbm`.
- Plan `1 x 40 ft dry container` for SeaRates: payload about 26,000 kg and
  usable volume about 67 cbm, with the selected plan constrained by weight and
  volume.
- Set `sizing_status: "sized"`, `confidence_bps` around `6500`, and warn with
  `FLAT_GLASS_PHYSICS_ESTIMATE`, `PACKING_SCHEDULE_MISSING`, and
  `SINGLE_ITEM_PACKAGE_AMBIGUOUS` when Alibaba's single-item package fields do
  not map to the 250 m2 order basis.

This fast path is the expected route for the Stage 11 DGU live demo. If the
listing must be inspected through CDP, use the persistent browser only:
`agent-browser --cdp ${CDP_PORT}`. Do not launch a fresh browser, log in, contact
suppliers, or mutate external state.

With `agent-browser` 0.25.x, do not pass natural-language instructions directly
as the command. Use concrete subcommands. The exact commands are:

```bash
CDP_PORT=${AGENT_BROWSER_CDP_PORT:-9222}; curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
CDP_PORT=${AGENT_BROWSER_CDP_PORT:-9222}; agent-browser --cdp ${CDP_PORT} --max-output 8000 open "https://www.alibaba.com/product-detail/Low-E-Tempered-Insulated-Glass-Clear_1600798460524.html"
CDP_PORT=${AGENT_BROWSER_CDP_PORT:-9222}; agent-browser --cdp ${CDP_PORT} --max-output 30000 get text body
```

The Stage 11 DGU live listing evidence for that exact URL is:

- product title: `Low E Tempered Insulated Glass Clear Tempered Insulated Glass Office Curtain Wall Double Glazed Glass`
- thickness: `6mm+9A+6mm`
- size: `Custom Sizes`
- material: `Qualified Tempered Float Glass`
- packing: `Sea Worthy Strong Wooden Crates`
- selling units: `Single item`
- single package size: `35X35X20 cm`
- single gross weight: `10.000 kg`
- price unit/MOQ: square metres, MOQ 10 square metres

The single-item package size/weight is ambiguous and must not override the
250 m2 order basis, but the same-listing `6mm+9A+6mm` thickness is sufficient
for the flat-glass physics estimate. If the concrete `agent-browser` command
fails but the source URL is this exact listing, use the same-listing evidence
above and proceed with `sizing_status: "sized"`; record a warning with code
`INDEXED_LISTING_EVIDENCE_USED`.

Minimum valid DGU fast-path result shape:

```json
{
  "product_candidate_id": "<from job_parameters.product_candidate_id>",
  "selected_quote_id": "<from job_parameters.selected_quote_id>",
  "sizing_status": "sized",
  "units_required": 250.0,
  "unit_basis": "square meter",
  "unit_weight_kg": 30.0,
  "unit_volume_cbm": 0.021,
  "total_weight_kg": 7500.0,
  "total_volume_cbm": 7.875,
  "package_count": 1,
  "container_plan": {
    "mode": "fcl",
    "container_type": "40ft_dry",
    "container_count": 1,
    "containers_required": 1,
    "payload_capacity_kg": 26000,
    "usable_volume_cbm": 67,
    "weight_utilization_bps": 2885,
    "volume_utilization_bps": 1175,
    "planning_volume_cbm": 7.875,
    "reasoning": "250 m2 * 30 kg/m2 = 7500 kg; geometric volume 5.25 cbm uplifted 50% for crating/handling."
  },
  "evidence": [
    {"kind": "listing_evidence", "source_url": "https://www.alibaba.com/product-detail/Low-E-Tempered-Insulated-Glass-Clear_1600798460524.html", "observed": "Thickness 6mm+9A+6mm; packing Sea Worthy Strong Wooden Crates; custom sizes."},
    {"kind": "calculation", "observed": "12 mm glass thickness for weight; 21 mm total unit thickness for volume; density 2500 kg/m3; 50% volume uplift."}
  ],
  "confidence_bps": 6500,
  "warnings": [
    {"code": "FLAT_GLASS_PHYSICS_ESTIMATE", "message": "Weight and CBM use the DGU flat-glass physics estimate from 6mm+9A+6mm listing evidence.", "severity": "warning"},
    {"code": "PACKING_SCHEDULE_MISSING", "message": "No project-specific packing schedule supplied; volume includes a conservative 50% crating/handling uplift.", "severity": "warning"},
    {"code": "SINGLE_ITEM_PACKAGE_AMBIGUOUS", "message": "Alibaba single-item package fields do not map cleanly to the 250 m2 order and were not used as the order basis.", "severity": "warning"}
  ],
  "blockers": []
}
```

## Evidence Priority
1. Selected quote evidence fields.
2. Supplier/listing URL if present.
3. Persistent CDP browser inspection for Alibaba or marketplace listings.
4. Candidate specification text and numeric specs.
5. `inferred_order_quantity_basis`.
6. Quote quantity/MOQ/listing pack fields only as supporting evidence.

If CDP is required, verify the persistent browser first:

```bash
CDP_PORT=${AGENT_BROWSER_CDP_PORT:-9222}

# Ensure CDP is up. The daemon's pre-flight gate SHOULD have done this already,
# but defense in depth: bin/ensure-cdp-9222 is idempotent and fast (<1s no-op
# when CDP is already up; otherwise launches Chrome with the operator's
# persistent profile at /Users/alinaaitova/.chrome-automation and polls up to
# 15s). Returns non-zero only if launch genuinely failed. Absolute path because
# the skill subprocess cwd is the per-skill workdir, not the project root.
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222

# Defensive reachability check. If this fails (helper exited non-zero, or a
# rare post-launch race), the agent should write a blocked sizing result with
# blocker CDP_CONNECT_FAILED rather than launching a fresh browser.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

Use `agent-browser --cdp ${CDP_PORT}` for all browser actions. Do not launch a fresh browser, log in, contact suppliers, or mutate external state.

## Session Recovery (auto-signin on session-expired)

This skill inspects Alibaba product detail pages via the persistent CDP browser. If the operator's Alibaba session has silently expired, the listing page redirects to `passport.alibaba.com/login.htm` or returns API errors. When detected, attempt auto-signin via `auth-signin/alibaba/` BEFORE emitting `sizing_status: "blocked"` with a session-expired blocker. Only emit `SESSION_REFRESH_FAILED` if auto-signin ALSO failed.

**Session-expired detection signals (Alibaba, shared with marketplace-search/alibaba):**
- URL after listing navigation redirects to `passport.alibaba.com/login.htm` or `login.alibaba.com`.
- The header `Sign In` link is visible (vs. the account menu when signed in).
- An Alibaba API response returns `error_code: "USER_NOT_LOGIN"` or HTTP 401 during listing inspection.

**Recovery flow:**

The canonical click sequence below was inlined from `skills/auth-signin/alibaba/SKILL.md` §Workflow on 2026-05-17. Selectors validated live against the persistent Chrome on port 9222 the same day. If you need to update the sequence, edit the source-of-truth at `skills/auth-signin/alibaba/SKILL.md` first and re-inline here. This block is identical to the inlined recovery in `skills/marketplace-search/alibaba/SKILL.md` — both skills share the same Alibaba session and selectors.

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# 1. Open Alibaba login page directly
agent-browser --cdp ${CDP_PORT} open "https://login.alibaba.com/"
agent-browser --cdp ${CDP_PORT} wait 2500

# 2. Check for slide-CAPTCHA. If present, write blocked result with
#    cdp_state: "captcha_blocked" and stop — do not attempt OAuth
#    (Alibaba rate-limits aggressive signin retries via slide CAPTCHA).
SLIDE_CAPTCHA=$(agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const sliderIframe = document.querySelector("iframe[src*=nocaptcha], iframe[src*=sufei]");
  const sliderDom = document.querySelector("[id*=nc_], .nc-container, [class*=slider-track]");
  return JSON.stringify({slide_captcha: !!(sliderIframe || sliderDom)});
})()
')

# 3. Click "Continue with Google" — Alibaba uses <a attr-type="google" attr-customurl="...">
#    The JS click handler reads attr-customurl, so synthetic .click() may not navigate;
#    read attr-customurl directly and navigate via window.location.assign.
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const stable = document.querySelector("a[attr-type=google][attr-customurl]");
  if (stable) {
    const target = stable.getAttribute("attr-customurl");
    if (target) { window.location.assign(target); return JSON.stringify({clicked: true, via: "attr-customurl"}); }
  }
  const candidates = [
    document.querySelector("a[href*=\"accounts.google.com/o/oauth2\"]"),
    document.querySelector("button.google-signin-button, [data-spm*=google i]"),
    ...Array.from(document.querySelectorAll("button, a, [role=button]")).filter(el => /continue with google|sign in with google/i.test(el.innerText || ""))
  ].filter(Boolean);
  const btn = candidates[0];
  if (!btn) return JSON.stringify({clicked: false, reason: "no_google_button_found"});
  btn.click();
  return JSON.stringify({clicked: true, via: "fallback"});
})()
'
agent-browser --cdp ${CDP_PORT} wait 4000

# 4. Confirm we landed on Google's chooser. If not, treat as SESSION_REFRESH_FAILED.
agent-browser --cdp ${CDP_PORT} eval 'JSON.stringify({on_google: window.location.hostname.endsWith("accounts.google.com"), url: window.location.href})'

# 5. Click the Dieter row on Google's chooser. (Mirror of auth-signin/google-account-chooser.)
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

# 6. Poll for redirect back to alibaba.com and verify signed-in. Detect each
#    escalation case (binding, region, identity verification, slide CAPTCHA).
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const deadline = Date.now() + 18000;
  while (Date.now() < deadline) {
    const url = window.location.href;
    const hostname = window.location.hostname;
    const bodyText = (document.body && document.body.innerText) || "";
    if (/bind.*alibaba account|link your google account/i.test(bodyText)) return JSON.stringify({status: "binding_required", url});
    if (/select your country|change.*region|account region/i.test(bodyText)) return JSON.stringify({status: "region_prompt", url});
    if (/verify your identity/i.test(bodyText) && /phone number|send.*code/i.test(bodyText)) return JSON.stringify({status: "identity_verification_required", url});
    if (url.includes("/iv/verify_mode.htm")) return JSON.stringify({status: "identity_verification_required", url});
    if (document.querySelector("iframe[src*=nocaptcha], [id*=nc_], .nc-container")) return JSON.stringify({status: "slide_captcha", url});
    const onAlibaba = hostname.endsWith("alibaba.com") && !hostname.startsWith("passport.") && !hostname.startsWith("login.");
    if (onAlibaba) {
      const accountMenu = document.querySelector("[class*=account-menu], [class*=my-alibaba], [class*=user-info]");
      if (accountMenu) return JSON.stringify({status: "signed_in", url, verified_via: "account_menu_present"});
      return JSON.stringify({status: "signed_in_weak", url, verified_via: "passport_redirect_absent"});
    }
    await new Promise(r => setTimeout(r, 500));
  }
  return JSON.stringify({status: "timeout", url: window.location.href});
})()
'
```

After the recovery sequence completes, map the poll `status` back to this skill's `sizing_status` / `cdp_state` taxonomy:

1. **If `status: "signed_in"` (or `"signed_in_weak"`):** EXIT this skill cleanly WITHOUT writing `result.json`. The daemon's retry policy at `crates/daemon/src/jobs/runner.rs` re-invokes the skill on the next pass; the second invocation re-fetches the listing and proceeds with sizing.

2. **If `status: "slide_captcha"`:** write a blocked `result.json` with `sizing_status: "blocked"`, `cdp_state: "captcha_blocked"`, and a single blocker `{code: "SESSION_REFRESH_FAILED", severity: "blocked", message: "Alibaba slide CAPTCHA appeared during auto-signin"}`. Stop here.

3. **If `status` is `"binding_required"`, `"region_prompt"`, `"identity_verification_required"`, `"timeout"`, or the Google chooser returned `"twofa_interstitial"` / `"chooser_target_missing"` / `"captcha"`:** write a blocked `result.json` with `sizing_status: "blocked"`, `cdp_state: "session_expired"`, and a single blocker `{code: "SESSION_REFRESH_FAILED", severity: "blocked", message: "<descriptive message naming the failure mode>"}`. Stop here.

## Rules
- Do not guess precision. If listing/spec evidence lacks dimensions or weight, block with `SHIPMENT_DIMENSIONS_MISSING`.
- If a quote gives square metres but no thickness/packing/unit mass, inspect source evidence or block.
- For flat glass, sealed insulating glass units, or DGU/IGU products quoted per square metre, a listing/spec thickness is sufficient to create a screening-grade size estimate even when Alibaba's "single item" package fields are ambiguous. Derive glass mass from observed pane thicknesses only, using glass density 2,500 kg/m3; ignore air/argon cavity thickness for weight, but include full unit thickness for volume. Example: `6mm+9A+6mm` means 12 mm of glass for weight and 21 mm total unit thickness for geometric volume.
- When using the flat-glass physics estimate, set `sizing_status: "sized"`, include the formula and source evidence, set confidence below 7000 unless packing/crating evidence maps directly to the quote basis, and warn with `FLAT_GLASS_PHYSICS_ESTIMATE` plus `PACKING_SCHEDULE_MISSING`. Do not use ambiguous "single item" package size/weight as the basis unless the listing explicitly maps it to one square metre or one finished unit.
- For a large square-metre order, create an FCL container plan from total weight/CBM. Use conservative planning assumptions in `container_plan`: 40 ft container payload about 26,000 kg, usable volume about 67 CBM, and add a crating/handling volume uplift when no packing schedule is supplied. Split by weight when weight requires more containers than volume.
- If CDP opens the exact listing URL but the page is captcha-blocked, add warning `CDP_CAPTCHA_INTERCEPTION`. You may continue only when deterministic cached/indexed evidence for the same listing URL is available in the supplied quote/search evidence. In that case add an `evidence` object with `kind: "indexed_listing_evidence"` or `kind: "search_attempt_evidence"`, the exact `source_url`, and `deterministic_captcha_fallback: true`. If no same-URL deterministic fallback exists, return `sizing_status: "blocked"` with blocker `CDP_CAPTCHA_BLOCKED`.
- Choose LCL for genuinely small shipments, FCL when volume/weight requires containers.
- Record load-factor/container heuristics as warnings or in `container_plan.reasoning`.
- If weight exceeds practical payload before volume fills a container, split by weight.

## Output
Write the final JSON object to `result.json` in the current working directory, matching `result.schema.json`. `warnings` and `blockers` must be arrays of objects with `code`, `message`, and `severity`; never emit bare string warning codes or omit severity. Also keep the final response to the daemon as the same raw JSON object; do not wrap it in Markdown.
