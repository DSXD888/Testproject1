# CDP Auto-Signin — Test Report Template

> **Status when this template lands in repo:** UNVERIFIED. The structural lints
> (`crates/daemon/tests/auth_signin_lint.rs`) pass — see "Structural verification"
> below. Per-site empirical verification (Phase A discovery + recovery-from-logout
> end-to-end) is the operator's responsibility before this report can flip to
> READY FOR PRODUCTION. Each row in §"Per-site empirical verification" is left
> blank for the operator to fill in during live testing.

**Date drafted:** 2026-05-17
**Tested by:** (operator name)
**Handoff reference:** `docs/handoff-2026-05-17-cdp-auto-signin.md`
**Depends on:** `docs/handoff-2026-05-17-cdp-9222-auto-launch.md` (executed 2026-05-17)

---

## Verdict
**STATUS: BUILD COMPLETE, EMPIRICAL VERIFICATION PENDING**

Structural deliverables (skill files, production-skill Session Recovery wiring, AGENTS.md update, lint tests) are landed and all 19 relevant tests pass (13 existing `skill_lint` + 6 new `auth_signin_lint`). Live Chrome verification has not yet been performed against eBay / Alibaba / SeaRates / Freightos / ProContract; see "Per-site empirical verification" below.

Flip to **READY FOR PRODUCTION** when every site has a PASS row AND every escalation case has been verified at least once.

---

## Structural verification (automated, complete)

| Check | Result | Evidence |
|---|---|---|
| 6 auth-signin skills exist with SKILL.md + result.schema.json | PASS | `cargo test --test auth_signin_lint ut_asln_001` |
| Each declares `requires_cdp: true` + `cdp_default_port: 9222` | PASS | `ut_asln_002` |
| Each invokes `bin/ensure-cdp-9222` in its body | PASS | `ut_asln_003` |
| 8 production skills reference matching auth-signin path | PASS | `ut_asln_004` |
| Each auth-signin skill declares `side_effect_class: local_state` | PASS | `ut_asln_005` |
| 4 Google OAuth skills reference `google-account-chooser` | PASS | `ut_asln_006` |
| Existing `skill_lint` suite still green (no regressions from production-skill edits) | PASS | `cargo test --test skill_lint` — 13/13 |

Re-run after any change:
```bash
cargo test --test auth_signin_lint --test skill_lint
```

---

## Per-site empirical verification (operator to fill in)

| Site | Sign-out method | auth-signin time | Production-skill recovery time | Verdict |
|---|---|---|---|---|
| eBay UK | Click avatar > Sign out in `ebay.co.uk` | TBD (expected ~8s) | TBD (expected ~12s total) | UNVERIFIED |
| Alibaba | Click avatar > Logout in `alibaba.com` | TBD (expected ~10s) | TBD (expected ~15s) | UNVERIFIED |
| SeaRates | `localStorage.removeItem("s-token")` in DevTools console | TBD (expected ~9s) | TBD (expected ~13s) | UNVERIFIED |
| Freightos | Click "D" avatar > Logout in `ship.freightos.com` | TBD (expected ~10s) | TBD (expected ~14s) | UNVERIFIED |
| ProContract (autofill, no Touch ID) | Browser logout button | TBD (expected ~7s) | TBD (expected ~11s) | UNVERIFIED |
| ProContract (with Touch ID prompt) | Browser logout button | TBD (operator authenticates within 12s budget) | TBD | UNVERIFIED |

### How to run each row

For each site:
1. Confirm Chrome is on port 9222 (`bin/ensure-cdp-9222 && curl -sfS http://localhost:9222/json/version | jq -r '.Browser'` should print `Chrome/...`).
2. Sign OUT of the site via the browser's own UI (or clear the relevant local-storage / cookie as noted in the table).
3. Trigger the production skill that depends on the site:
   - eBay UK: `marketplace-search/ebay-uk` against a known-good candidate fixture.
   - Alibaba: `marketplace-search/alibaba` against a candidate whose listing requires login.
   - SeaRates: `landed-cost/searates-freight-estimate` against the canonical Shanghai→London DGU route.
   - Freightos: `landed-cost/freightos-freight-estimate` (once that operator-built skill exists per the L5 handoff) against the same route.
   - ProContract: `procurement-ingestion/procontract-discover` with `publishedFrom` set to today.
4. Watch the daemon logs: production skill detects logged-out → invokes auth-signin/<site> → click sequence runs → verifies signed-in → exits cleanly → daemon retries → main operation completes.
5. Time from "skill invoked" to "result.json with successful status written" goes in the "Production-skill recovery time" column.
6. Update the Verdict to PASS if the result.json shows the EXPECTED successful status (not `SESSION_REFRESH_FAILED`).

---

## Escalation cases verified (operator to fill in)

Each row needs at least one positive verification — the goal is to confirm the skill correctly DETECTS the case and emits `signin_status: "manual_required"` (not silently retries forever, and not mis-classifies as another failure).

| Escalation case | Site / skill | How to reproduce | Detected as `manual_required`? | Verdict |
|---|---|---|---|---|
| Touch ID prompt | ProContract / auth-signin/procontract | Run while operator is away from keyboard (Touch ID times out) | TBD (expected: `submit_outcome: "touch_id_pending"`, blocker `PROCONTRACT_TOUCH_ID_PENDING`) | UNVERIFIED |
| Google 2FA prompt | Any Google OAuth site (typically first device or unfamiliar IP) | Force 2FA by signing out of Google + clearing device trust + retry | TBD (expected: `page_shape: "twofa_interstitial"`, blocker `GOOGLE_INTERACTIVE_CHALLENGE`) | UNVERIFIED |
| Google reCAPTCHA on chooser | Any Google OAuth site (typically after many automated attempts) | Repeat the OAuth click sequence 10-15 times in quick succession from an automation profile | TBD (expected: `page_shape: "captcha"`, blocker `GOOGLE_INTERACTIVE_CHALLENGE`) | UNVERIFIED |
| Google account chooser empty | Any Google OAuth site | Sign out of Google entirely in `chrome://chrome-signin` | TBD (expected: `page_shape: "email_entry_form"`, blocker `GOOGLE_CHOOSER_ABSENT`) | UNVERIFIED |
| Chrome password autofill disabled | ProContract | Toggle off at `chrome://settings/passwords` (then re-enable after test) | TBD (expected: `autofill_fired: false`, blocker `PROCONTRACT_AUTOFILL_DID_NOT_FIRE`) | UNVERIFIED |
| Alibaba slide CAPTCHA | Alibaba / auth-signin/alibaba | Repeat OAuth attempts 5-10 times in 60 seconds | TBD (expected: blocker `ALIBABA_SLIDE_CAPTCHA_PRESENT`) | UNVERIFIED |
| Alibaba account binding (first-time Google → Alibaba) | Alibaba | Only reproducible with a fresh Google account never bound to Alibaba | TBD (likely UNTESTED — note in row if not reproducible) | UNVERIFIED |
| Freightos Cloudflare challenge | Freightos | Difficult to force; document if observed naturally | TBD (likely UNTESTED) | UNVERIFIED |
| eBay post-OAuth verification interstitial | eBay UK | Rare — typically only on new-device first signin | TBD (likely UNTESTED) | UNVERIFIED |

---

## Phase A discovery findings (operator to fill in)

Each per-site auth-signin SKILL.md has a "Phase A discovery (mandatory before first stable run)" subsection listing the selectors that need verification against the live site. Record the discovered selectors here so future operators can audit the canonical values.

### Phase A discovery — google-account-chooser

(Reference: `skills/auth-signin/google-account-chooser/SKILL.md` § Phase A discovery)

| Discovery item | Discovered value | Notes |
|---|---|---|
| Chooser target-row selector (priority 1) | TBD | Expected: `[data-identifier="dieter.x.s.danicek@gmail.com"]` |
| Chooser target-row selector (priority 2) | TBD | Expected: `[data-email="dieter.x.s.danicek@gmail.com"]` |
| Chooser target-row selector (priority 3, fallback) | TBD | Expected: `li[role="link"]` with text matching email |
| Email-entry form indicator | TBD | Expected: `input[type="email"][name="identifier"]` or `input#identifierId` |
| reCAPTCHA iframe selector | TBD | Expected: `iframe[src*="recaptcha"]` |
| 2FA interstitial heading regex | TBD | Expected: matches `/Verify it's you|2-Step Verification/` |
| Success-redirect target URL pattern (when chooser URL was `accounts.google.com/AccountChooser?continue=...`) | TBD | Expected: lands on the `continue` URL within ~5s |

### Phase A discovery — eBay UK

(Reference: `skills/auth-signin/ebay/SKILL.md` § Phase A discovery)

| Discovery item | Discovered value | Notes |
|---|---|---|
| Logged-out top-right link selector | TBD | Expected: `a[href*="signin.ebay.co.uk"]` |
| Sign In button selector (eBay homepage) | TBD | Tested in Phase A |
| Continue with Google button selector (eBay signin page) | TBD | One of: `button[aria-label*="Google" i]`, `[data-social="google"]`, button with text "Continue with Google" |
| Signed-in greeting/avatar selector | TBD | Operator confirms which signal (greeting / My eBay link / both) is the most reliable |
| Post-OAuth interstitial selector (if encountered) | TBD | Document if "Verify your account" page seen |

### Phase A discovery — Alibaba

(Reference: `skills/auth-signin/alibaba/SKILL.md` § Phase A discovery)

| Discovery item | Discovered value | Notes |
|---|---|---|
| Login URL behavior (direct `login.alibaba.com` vs. redirect) | TBD | If redirected, capture the actual URL |
| Continue with Google button selector | TBD | One of: anchor with Google OAuth href, `button.google-signin-button`, `[data-spm*="google" i]` |
| Slide CAPTCHA detection selector | TBD | Expected: `iframe[src*=nocaptcha]`, `[id*=nc_]`, `.nc-container` |
| My Alibaba / account menu selector (signed-in signal) | TBD | One of: `[class*=account-menu]`, `[class*=my-alibaba]`, `[class*=user-info]` |
| Binding flow text regex (for first-time Google→Alibaba) | TBD | Expected: matches `/bind.*alibaba account|link your google account/i` |
| Region prompt text regex | TBD | Expected: matches `/select your country|change.*region|account region/i` |

### Phase A discovery — SeaRates

(Reference: `skills/auth-signin/searates/SKILL.md` § Phase A discovery)

| Discovery item | Discovered value | Notes |
|---|---|---|
| Login URL | TBD | Expected: `https://www.searates.com/login` |
| Continue with Google button selector | TBD | One of: anchor with Google OAuth href, `button[aria-label*="Google" i]`, `[data-provider="google"]` |
| s-token storage location | TBD | Expected: `localStorage.getItem("s-token")` returns JWT |
| s-token format (JWT vs. opaque) | TBD | If JWT, has `exp` claim; if opaque, skill falls back to "any non-empty value" |
| s-token TTL observed | TBD | Days / weeks — informs how often auto-signin is invoked |

### Phase A discovery — Freightos

(Reference: `skills/auth-signin/freightos/SKILL.md` § Phase A discovery)

| Discovery item | Discovered value | Notes |
|---|---|---|
| Login URL | TBD | Default candidate: `https://ship.freightos.com/login` — confirm whether redirected |
| "D" avatar selector (logged-in signal) | TBD | Expected: circular element with text "D" |
| "Refer a friend" link selector | TBD | Per screenshot `screenshots/freightos/freightos-step-01.png` |
| Continue with Google button selector | TBD | One of: anchor with Google OAuth href, `button[aria-label*="Google" i]`, `[data-provider="google"]` |
| Cloudflare interstitial detection | TBD | Expected: `[id^="cf-"]`, `#challenge-form`, `.cf-browser-verification`, or text "Just a moment" |

### Phase A discovery — ProContract

(Reference: `skills/auth-signin/procontract/SKILL.md` § Phase A discovery)

| Discovery item | Discovered value | Notes |
|---|---|---|
| Login URL | TBD | Default: `https://procontract.due-north.com/Login` |
| Landing card vs. direct login form | TBD | Capture which state the unauthenticated `/Login` URL serves |
| Username field selector | TBD | Expected: `input[name="username"]` or `input[name="Username"]` |
| Password field selector | TBD | Expected: `input[type="password"]:not([type="hidden"])` |
| Submit button selector | TBD | Expected: `button[type="submit"]` |
| Chrome autofill firing time observed | TBD | Expected: 1-2 seconds after username click |
| Touch ID required? (yes/no, time to satisfy) | TBD | Varies by macOS version + Chrome autofill settings |
| Error alert selector (for wrong-credentials test) | TBD | Expected: `.alert-danger`, `.alert.alert-error`, `[role="alert"]` |

---

## Quirks observed during Phase A (operator to fill in)

(Free-form section — record anything surprising, e.g., "Google chooser DOM changed on YYYY-MM-DD, updated selector from X to Y in google-account-chooser SKILL.md", "Alibaba's slide CAPTCHA appeared after 3 OAuth attempts not 5", "ProContract /Login now serves direct form, no landing card", etc.)

---

## Sign-off

Operator: ____________________
Date: ____________________

Once every row above is PASS or explicitly marked UNTESTED-with-rationale, flip the top-line verdict from "BUILD COMPLETE, EMPIRICAL VERIFICATION PENDING" to "READY FOR PRODUCTION" and note the date here.
