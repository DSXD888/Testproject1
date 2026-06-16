---
skill_id: marketplace-search/ebay-uk
skill_version: 1.1.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
requires_cdp: true
cdp_default_port: 9222
maxPages: 10
maxScrollCycles: 3
---

# marketplace-search/ebay-uk

## Use when
Use this skill when the daemon needs a UK retail benchmark ceiling from
eBay UK (`https://www.ebay.co.uk/`) for `product_candidates` whose
`search_eligibility_state` is `eligible` or `marginal`.

## Do not use when
Do not use for wholesale, Chinese floor, or ingestion.

## Inputs
- `source` — always `"ebay-uk"`.
- `candidates[]` — see the **Gem Imports SKILL.md** for the full field map.
  Beyond the basic identity fields (`product_candidate_id`,
  `search_profile_id`, `name`, `head_noun`, `specification_text`,
  `query_waves[]`), the daemon also sends structured anchors (`standards[]`,
  `critical_attributes`, `numeric_specs`, `material`, `quantity`,
  `quantity_unit`) plus parent context (`lot_title`, `lot_description`,
  `notice_title`, `notice_summary`, `buyer_name`, `cpv_codes`).

### Using the full candidate context when scoring (non-negotiable)

eBay is heterogeneous — the same head-noun surfaces domestic-DIY,
trade-seller, and retail-refurbish listings. Disambiguate via:
1. **`specification_text`** for size / grade / finish match signals, AND
2. **`standards`** if present — listings citing the same BS/EN/ISO code
   score materially higher, AND
3. **`cpv_codes`** + **`lot_title`** to validate the candidate lot-scope
   against eBay's own category. A listing whose eBay category is
   *"Home, Furniture & DIY > Windows"* is not the same as a tender for
   CPV `44221111-6 - Double-glazed units` (a trade/construction
   classification).

Record these cross-references in each quote's `evidence` field.

## Outputs
- `result.json` matching `result.schema.json`. Every candidate must
  appear in `search_results` at least once.

Prices captured are consumer retail (mixed with some trade sellers).
The daemon applies a configurable wholesale discount (default 30%)
downstream; the skill MUST store the raw listing price unchanged.

## Non-goals
- Logging in, bidding on auctions, messaging sellers.
- Converting retail → estimated wholesale.

## Failure Codes
- SOURCE_CHANGED
- SOURCE_SOFT_BLOCKED
- SOURCE_CAPTCHA_BLOCKED
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- AGENT_STALLED

## Pagination Bounds
- `maxPages`: 10
- `maxScrollCycles`: 3

---

## Playbook

You are a retail-benchmark price-search agent working on eBay UK.
eBay is a mixed marketplace: consumers, dropshippers, and trade
sellers. Your job is to find the **best plausible fixed-price listing**
for each candidate.

### Mandatory CDP Discipline

Use the operator persistent CDP browser for this public source so the run stays visible on the shared browser surface and preserves locale/session state. Do not launch a fresh browser profile and do not close the operator browser.

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

If CDP is unreachable, write a valid blocked `result.json` with `search_failure_mode: "source_soft_blocked"` for every remaining candidate and a warning/blocker code `CDP_CONNECT_FAILED`. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

### Session Recovery (auto-signin on session-expired)

If the main operation below detects that the operator's persistent Chrome profile is signed OUT of eBay UK, attempt auto-signin via `auth-signin/ebay/` BEFORE emitting any session-expired blocker. Only emit `SESSION_REFRESH_FAILED` if auto-signin ALSO failed (which means the case genuinely needs operator intervention — captcha, 2FA, "Verify your account" interstitial, or Google account chooser empty).

This applies to operator-decision setups where the eBay session is expected to be signed in (richer search access, saved-search syndication). It does NOT run when the skill is intentionally configured to operate anonymously — anonymous mode remains the default for this skill.

**Session-expired detection signals (eBay UK):**
- Top-right header shows the `Sign in or register` link (`a[href*="signin.ebay.co.uk"]` is present) instead of the operator's greeting / avatar.
- No `Hi <name>` or `Hello <name>` greeting in `document.body.innerText`.
- No `My eBay` navigation link (`a[href*="/myb/"]` absent).

When any of these conditions hold at navigation time AND the operator wants signed-in access, treat the session as expired.

**Recovery flow:**

The canonical click sequence below was inlined from `skills/auth-signin/ebay/SKILL.md` §Workflow on 2026-05-17. Phase A discovery of the selectors is complete. If you need to update the sequence, edit the source-of-truth at `skills/auth-signin/ebay/SKILL.md` first and re-inline here.

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# 1. Click the Sign In link (selector: canonical href on the eBay homepage)
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const a = document.querySelector("a[href*=\"signin.ebay.co.uk\"]");
  if (!a) return JSON.stringify({clicked: false, reason: "signin_link_disappeared"});
  a.click();
  return JSON.stringify({clicked: true});
})()
'
agent-browser --cdp ${CDP_PORT} wait 2500

# 2. Click "Continue with Google" on the eBay signin page
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const candidates = [
    document.querySelector("button[aria-label*=Google i]"),
    document.querySelector("[data-social=google]"),
    document.querySelector("button[data-test-id*=google i]"),
    ...Array.from(document.querySelectorAll("button, a")).filter(el => /continue with google/i.test(el.innerText || ""))
  ].filter(Boolean);
  const btn = candidates[0];
  if (!btn) return JSON.stringify({clicked: false, reason: "no_google_button_found", current_url: window.location.href});
  btn.click();
  return JSON.stringify({clicked: true});
})()
'
agent-browser --cdp ${CDP_PORT} wait 3000

# 3. Confirm we landed on the Google chooser
agent-browser --cdp ${CDP_PORT} eval 'JSON.stringify({on_google: window.location.hostname.endsWith("accounts.google.com"), url: window.location.href})'

# 4. Pick the Dieter row on the Google chooser (mirror of auth-signin/google-account-chooser)
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

# 5. Poll for redirect back to ebay.co.uk and verify signed-in
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const deadline = Date.now() + 15000;
  while (Date.now() < deadline) {
    const url = window.location.href;
    if (document.querySelector("iframe[src*=recaptcha], iframe[title*=captcha i]")) return JSON.stringify({status: "interstitial_captcha", url});
    if (/Verify your account|Please confirm your identity|unusual activity/i.test((document.body && document.body.innerText) || "")) return JSON.stringify({status: "interstitial_verification", url});
    const onEbay = window.location.hostname.endsWith("ebay.co.uk") && !window.location.hostname.startsWith("signin.");
    if (onEbay) {
      const signinLink = document.querySelector("a[href*=\"signin.ebay.co.uk\"]");
      const greetingPresent = /\bHi |\bHello /.test((document.body && document.body.innerText) || "");
      const myEbayLink = document.querySelector("a[href*=\"/myb/\"], a[href*=\"my.ebay.co.uk\"]");
      if (!signinLink && (greetingPresent || myEbayLink)) return JSON.stringify({status: "signed_in", url, verified_via: greetingPresent ? "greeting_text_present" : "my_ebay_link_present"});
      if (!signinLink) return JSON.stringify({status: "signed_in_weak", url, verified_via: "signin_link_absent"});
    }
    await new Promise(r => setTimeout(r, 500));
  }
  return JSON.stringify({status: "timeout", url: window.location.href});
})()
'
```

After the recovery sequence completes, verify signed-in by re-running this skill's own logged-out detection eval (signin link absent + greeting / My eBay link present). Then:

1. **If signed in:** EXIT this skill cleanly WITHOUT writing `result.json`. The daemon's existing retry policy at `crates/daemon/src/jobs/runner.rs` will re-invoke this skill on its next retry pass; the second invocation sees the restored session and completes the main search operation normally.

2. **If NOT signed in after the signin sequence:** write a blocked `result.json` with one `search_results` entry per remaining candidate having `search_failure_mode: "source_soft_blocked"` and a single blocker `{code: "SESSION_REFRESH_FAILED", severity: "blocked", message: "<descriptive message naming the failure mode: captcha, 2FA, account chooser empty, etc.>"}`. Stop here — the daemon's retry loop owns retries.

This auto-signin recovery is ONLY invoked on session-expired detection. It does NOT run preemptively on every invocation — that would waste time on the happy path AND risk login-storm rate limits (Google flags excessive automated OAuth attempts).

### Step 0: Read Job Parameters

Parse `candidates` and `query_waves`.

### Step 1: Open eBay UK

```
agent-browser --cdp ${CDP_PORT} open "https://www.ebay.co.uk/"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

Dismiss cookie banners. Do NOT log in.

### Step 2: For Each Candidate, Walk Query Waves

For each candidate, iterate `query_waves` and stop at the first wave
that produces ≥1 shortlisted quote with `match_confidence_bps >= 4000`.

#### 2a. Execute the search — filtered for Buy-It-Now + New

eBay supports URL-filtered searches. Always append these filters:

- `LH_BIN=1` (Buy It Now only — **exclude auctions**)
- `LH_ItemCondition=1000|1500` (New or New Other only — exclude used)

```
agent-browser --cdp ${CDP_PORT} open "https://www.ebay.co.uk/sch/i.html?_nkw=<URL-encoded query>&LH_BIN=1&LH_ItemCondition=1000%7C1500&_sop=15"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

`_sop=15` sorts by "Price + postage: lowest first", which aligns with
finding the best per-unit deal.

#### 2b. Shortlist up to 3 listings — prefer multi-item & trade sellers

Scan results and prefer:

- Multi-item listings ("10 pack", "Lot of 50", "Bulk 100") — divide
  total price by item count for the per-unit price.
- Listings with "Business seller" badge.
- Listings with high "items sold" counts — a weak signal of legitimate
  trade presence.

Reject:

- Auction-style listings (should already be filtered, but verify).
- Used / refurbished (verify the New filter held).
- Listings whose seller feedback is below 98% (hover to read the
  seller rating; skip if visibly low).
- Listings whose title clearly mismatches.

#### 2c. Open each shortlisted detail page

```
agent-browser --cdp ${CDP_PORT} click "@<ref of listing title>"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

Scroll to reveal variation selector / quantity picker:

```
agent-browser --cdp ${CDP_PORT} scroll down 600
agent-browser --cdp ${CDP_PORT} snapshot
```

If the listing has **variations** (e.g. "Colour: White / Size: A4")
that change the price, stay on the base/default variation — do not
click-through variations.

#### 2d. Extract raw fields

| Field | Notes |
|-------|-------|
| `product_name` | listing title |
| `source_url` | current URL (strip query params except item id) |
| `source_listing_id` | item id (e.g. `/itm/395123456789` → `395123456789`) |
| `unit_price_minor` | per-unit pence; for a listing of N items at £P total, `(P × 100) / N` rounded down |
| `currency` | `"GBP"` |
| `vat_inclusive` | typically `true` for UK retail; `null` if ambiguous |
| `quantity` | items bundled in the single listing, or 1 |
| `quantity_unit` | `"pack"`, `"lot"`, `"unit"` |
| `minimum_order_quantity` | `null` (eBay has no MOQ) |
| `price_tiers` | single-tier array — eBay typically lists one price |
| `ship_from_country` | from the "Item location" string on the listing (`GB` if in UK) |
| `includes_shipping` | `true` if postage is `£0.00 / Free`; else `false`; add postage to total if needed |
| `is_used_or_refurbished` | `false` (filter held) |
| `seller_identifier` | seller username |
| `match_confidence_bps` | rubric from Gem Imports, minus 500 bps retail bias, minus further 500 bps if seller is new/low-feedback |
| `evidence` | 1–3 sentences citing title, pack/lot size, and per-unit math |

### Step 3: Handle Zero-Result and Blocked Waves

- Zero results across all waves → one entry with
  `search_failure_mode: "no_results_all_waves"`, empty `quotes`.
- Soft block → `search_failure_mode: "source_soft_blocked"`.

### Step 4: Emit Result Contract

Write to `result.json` per the schema.

### Step 5: Leave Browser Open

Do not close or reset the operator browser. Leave the last page visible for inspection.

---

## Canary
Search three representative goods. Verify every quote has the BIN + New
filter applied (no auction items, no used items). Verify `currency = GBP`
and `ship_from_country = GB` for the chosen quotes.

## Run Manifest Instructions
See Gem Imports SKILL.md. `agent_claimed_source_url` is
`https://www.ebay.co.uk/`. In Decisions Made, call out which listings
you rejected for auction / used / low seller feedback, and which you
preferred for multi-item bundles.
