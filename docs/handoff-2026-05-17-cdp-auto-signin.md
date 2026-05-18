# Handoff: CDP Auto-Signin (`skills/auth-signin/{site}/`)

> **Generated:** 2026-05-17
> **Trigger:** Sites with auto-logout (Freightos, ProContract, eBay, Alibaba, SeaRates) silently expire the operator's persistent-profile session over time. Currently the affected skills emit a `*_SESSION_EXPIRED` blocker and the operator must manually re-sign-in in the browser before the job can retry. This handoff adds a new skill family that performs the sign-in automatically (Google OAuth for most sites, password-manager flow for ProContract), and wires every affected production skill to call it on session-expired detection before giving up.
> **Scope:** Net-new `skills/auth-signin/{site}/` family with 5 skills (eBay, Alibaba, SeaRates, Freightos, ProContract) + 1 shared sub-skill (Google account chooser) + edits to 8 affected production skills + the 2 in-progress operator-built skill handoffs.
> **Depends on:** `docs/handoff-2026-05-17-cdp-9222-auto-launch.md` (the `bin/ensure-cdp-9222` helper). That handoff must be executed FIRST so this one can assume CDP is always reachable.
> **Outcome when done:** When any covered skill detects a logged-out state, it invokes the matching `auth-signin` skill, which performs the Google OAuth click sequence (or ProContract password-manager flow), retries the original operation ONCE, and only emits a `SESSION_REFRESH_FAILED` blocker if auto-signin genuinely couldn't recover (e.g., fingerprint required, Google captcha challenge, 2FA prompt). Net effect: ~95% reduction in "operator forgot the site logged them out" interruptions.

---

## Mission

Add an `auth-signin/{site}/` skill family with 5 sites + 1 shared Google account chooser. Wire every CDP-using production skill whose site requires auth to:

1. Detect session-expired state from response signals (401, redirect to /login, missing avatar element, etc. — discovered empirically per site).
2. Invoke the matching `auth-signin` skill as a subprocess.
3. Retry the original operation ONCE on signin success.
4. Emit `SESSION_REFRESH_FAILED` only if auto-signin also failed (which means the case genuinely needs the operator).

The five operator-supplied flows (verbatim from your guidance):

| Site | Flow | Notes |
|---|---|---|
| eBay UK | click `Sign In` → `Continue with Google` → select Dieter Danicek (`dieter.x.s.danicek@gmail.com`) | Standard pattern |
| Alibaba | "same concept as eBay" | Standard pattern |
| SeaRates | "same concept as eBay" | Standard pattern — recovers s-token via Google sign-in |
| Freightos | "same concept as eBay" | Standard pattern — uses persistent profile account "D" |
| ProContract | click `login here` → click username field → password manager popup → select `sales@sorelon.com` → autofill → `Continue`. **Sometimes requires fingerprint** — if so, escalate. | Password-manager flow (NOT Google OAuth) |

The common Google flow has THREE steps in the DOM: (a) click `Sign In` button on the site, (b) click `Continue with Google` in the resulting auth modal, (c) navigate the Google account chooser to select Dieter Danicek. Step (c) is identical across all four Google sites — encapsulate it as a shared sub-skill (`auth-signin/google-account-chooser/`) that the per-site skills delegate to.

---

## Audit findings (the empirical site coverage list)

Verified by reading all 16 CDP-requiring `SKILL.md` files. Coverage:

| Skill | Site | Login state today | Auto-signin needed? | Sign-in mechanism |
|---|---|---|---|---|
| `marketplace-search/ebay-uk` | ebay.co.uk | currently anonymous | YES (operator decision — richer access) | Google OAuth |
| `marketplace-search/alibaba` | alibaba.com | pre-auth in profile | YES | Google OAuth |
| `marketplace-search/amazon-uk` | amazon.co.uk | anonymous | NO (operator default — anonymous is fine) | — |
| `marketplace-search/gem-imports` | gemimports.co.uk | anonymous | NO | — |
| `marketplace-search/mx-wholesale` | mxwholesale.co.uk | anonymous (some prices login-gated; skill skips) | NO (operator default) | — |
| `procurement-ingestion/contracts-finder` | contractsfinder.service.gov.uk | public OCDS API | NO | — |
| `procurement-ingestion/fts` | find-tender.service.gov.uk | public | NO | — |
| `procurement-ingestion/procontract-discover` | procontract.due-north.com | pre-auth in profile | YES | Password manager (sales@sorelon.com) |
| `procurement-ingestion/procontract-enrich-tender` | procontract.due-north.com | pre-auth in profile | YES | Password manager |
| `procurement-ingestion/procontract-register-interest` | procontract.due-north.com | pre-auth in profile | YES | Password manager |
| `landed-cost/searates-freight-estimate` | searates.com | s-token in localStorage | YES (Google OAuth recovers token) | Google OAuth |
| `landed-cost/shipment-sizing` | alibaba.com (listing inspect) | shares alibaba session | YES (via auth-signin/alibaba) | Google OAuth |
| `source-health/alibaba` | alibaba.com | shares alibaba session | YES (via auth-signin/alibaba) | Google OAuth |
| `source-health/fts` | find-tender.service.gov.uk | public | NO | — |
| `auth-refresh/alibaba` | alibaba.com | MANUAL refresh skill | N/A — existing skill is for explicit operator refresh | — |
| `auth-refresh/1688` | 1688.com | MANUAL refresh skill | N/A — 1688 doesn't support Google OAuth (QR code / WeChat only) | — |

In-progress L5 operator-built handoffs:

| Handoff | Site | Auto-signin? |
|---|---|---|
| `HANDOFF-freightos-freight-estimate.md` (template for skill not yet built) | ship.freightos.com | YES (Google OAuth) — handoff updated to integrate from day one |
| `HANDOFF-taric-cross-check.md` (template for skill not yet built) | ec.europa.eu (TARIC) | NO (free public EU data, no login) — handoff gets explicit N/A note |

**Net auto-signin coverage: 8 production skills + 1 in-progress operator-built skill, across 5 site-specific signin flows.**

---

## Architecture

### New skill family layout

```
skills/auth-signin/
├── google-account-chooser/        # Shared sub-skill: select Dieter Danicek on any Google OAuth chooser page
│   ├── SKILL.md
│   ├── result.schema.json
│   └── (no checkpoint — atomic short skill)
├── ebay/
│   ├── SKILL.md                   # Flow: click Sign In → click Continue with Google → delegate to chooser
│   └── result.schema.json
├── alibaba/
│   ├── SKILL.md
│   └── result.schema.json
├── searates/
│   ├── SKILL.md
│   └── result.schema.json
├── freightos/
│   ├── SKILL.md
│   └── result.schema.json
└── procontract/
    ├── SKILL.md                   # Different flow: password-manager popup
    └── result.schema.json
```

Naming distinguishes the new `auth-signin/` family from the existing `auth-refresh/` family:
- **`auth-refresh/`** (existing) = the operator manually refreshes cookies via the browser GUI; skill detects + reports state.
- **`auth-signin/`** (NEW) = skill performs the click sequence automatically using already-resident OAuth state / password manager.

### Integration pattern (every affected production skill)

Each affected skill's "Mandatory CDP Discipline" section gets a new "Session Recovery" subsection. The pattern runs INLINE within the skill (not as a separate subprocess) using `agent-browser` subcommands — matching the existing skill convention (verified: `marketplace-search/{ebay-uk,alibaba}/SKILL.md` use `agent-browser open/click/wait/snapshot`, not custom subprocess spawning).

```
1. ensure-cdp-9222              (auto-launch Chrome if needed — from prior handoff)
2. Attempt main operation        (e.g., fetch SeaRates GraphQL, navigate Freightos search)
3. IF session-expired signal     (per-site detection — discovered empirically):
   a. Run signin click sequence INLINE  (agent-browser open/click/wait — sequence copied
      verbatim from skills/auth-signin/<SITE>/SKILL.md §Workflow)
   b. Verify signed-in via per-site logged-in signal (agent-browser eval)
   c. IF signed-in: EXIT skill cleanly without writing result.json.
                    Daemon's existing retry policy re-invokes the skill;
                    the second invocation sees the restored session and completes
                    the main operation normally.
   d. IF NOT signed-in after the click sequence: write result.json with
      SESSION_REFRESH_FAILED blocker (operator action required: 2FA, captcha,
      empty Google chooser, password autofill disabled, etc.).
4. (Normal flow continues — only reached if no session-expired signal in step 3.)
```

**Why daemon-retry instead of in-skill retry:** the skill's main operation may be a multi-step sequence (open → click → eval → parse). Re-executing it inline would require wrapping it in a bash function (fragile across skills with different main-op shapes) or using `exec` to restart the skill (loses captured state). The daemon's existing retry policy at `crates/daemon/src/jobs/runner.rs` already retries failed/missing-result.json skills with backoff — leveraging it keeps the new integration shallow.

The trade-off: one extra retry round-trip when signin happens (typical: signin takes ~5-10s, daemon retry kicks in ~10-30s later, second invocation completes in normal time). Acceptable vs. the alternative complexity.

**Crucially**: auto-signin is only triggered ON session-expired detection. It does NOT run preemptively on every skill invocation — that would waste time on the happy path AND risk login-storm rate limits (Google flags excessive automated OAuth attempts).

### Compatibility with the prior auto-launch handoff's lint

The prior `docs/handoff-2026-05-17-cdp-9222-auto-launch.md` added a lint test `ut_skl_302_cdp_skills_invoke_ensure_helper` that requires every skill with `requires_cdp: true` to invoke `bin/ensure-cdp-9222` somewhere in its body. **The new auth-signin/* skills declare `requires_cdp: true`** (Step 8 lint asserts this). **Therefore each auth-signin SKILL.md MUST include a `bin/ensure-cdp-9222` invocation in its "Mandatory CDP Discipline" block** to pass the existing lint. The Step 1-3 SKILL.md templates include this; don't strip it.

### Detection signals per site (discovered empirically during build)

Each site has different signals for "you're logged out". Per-site SKILL.md documents the canonical signals:

| Site | Detection signal (suggested — discover empirically during Phase A) |
|---|---|
| eBay | "Sign in" button visible top-right (selector: `[data-test-id="user-sign-in"]` or similar) — avatar element absent |
| Alibaba | redirect to `passport.alibaba.com/login.htm`, OR "Sign In" link visible in header, OR API response with `error_code: "USER_NOT_LOGIN"` |
| SeaRates | `localStorage.getItem("s-token")` returns null/expired, OR `/auth/platform-token?id=1` fetch returns 401 |
| Freightos | search API POST returns HTTP 401 / 302 to `/login`, OR DOM contains login modal selector |
| ProContract | URL contains `/Login` after navigation, OR page text contains "Session expired" / "Please log in" |

**These signals are discovered in each per-site auth-signin SKILL.md's Phase A** (DevTools session-recording while manually logging out). The production-skill SKILL.md references the same signals for its "step 3 IF session-expired signal" check.

### Escalation cases (operator manual intervention required)

Auto-signin will NOT succeed in these cases. The auth-signin skill returns `signin_status: "manual_required"` and the production skill emits `SESSION_REFRESH_FAILED` with a clear message telling the operator what to do:

| Case | Trigger | Operator action |
|---|---|---|
| **ProContract fingerprint** | After password autofill, Touch ID prompt appears | Operator physically authenticates with Touch ID |
| **Google reCAPTCHA on chooser** | Google detects automation; shows "I'm not a robot" | Operator clicks captcha manually |
| **Google 2FA prompt** | First-time device or suspicious-login detection | Operator approves 2FA on phone |
| **Google account chooser empty** | Operator's Google account isn't signed into the persistent Chrome profile at all | Operator signs into Google in `chrome://chrome-signin` |
| **Site-side captcha after Continue with Google** | eBay / Alibaba defense-in-depth | Operator solves captcha |
| **Password manager popup absent** (ProContract) | macOS Passwords app disabled, or autofill turned off in Chrome | Operator enables Chrome password autofill OR fills credentials manually |

Each is a legitimate escalation — no skill can solve them silently. The skill's job is to DETECT them quickly and emit a clear blocker, not to hang trying.

---

## Implementation

### Step 1 — Create `skills/auth-signin/google-account-chooser/` (~45-60 min)

This is the shared sub-skill all four Google-OAuth-using sites delegate to.

**Mission:** Given that the current Chrome tab has just navigated to Google's OAuth account chooser (URL like `accounts.google.com/o/oauth2/...` or `accounts.google.com/AccountChooser?...`), find the row for `dieter.x.s.danicek@gmail.com`, click it, wait for the consent/redirect, and report success.

**Inputs:**
- `target_email`: defaults to `dieter.x.s.danicek@gmail.com` (overridable for future multi-account scenarios)
- `parent_window_origin`: e.g., `https://www.ebay.co.uk` (so the skill knows what redirect to wait for)

**Workflow (3-command happy path, like other CDP skills):**
1. CDP health check.
2. `agent-browser --cdp 9222 eval` discovery: read the current page; look for Google's account chooser DOM. Common selectors:
   - `[data-email="dieter.x.s.danicek@gmail.com"]`
   - `[aria-label*="dieter.x.s.danicek"]`
   - Generic: row containing `<div>` with text matching the email substring
3. Click the matching row via `agent-browser eval` (issue native click). Then poll for redirect away from `accounts.google.com` back to `parent_window_origin`. Up to 30 seconds.

**Output (`result.schema.json`):**
```json
{
  "signin_status": "signed_in",     // or "manual_required" / "blocked"
  "account_selected": "dieter.x.s.danicek@gmail.com",
  "redirected_to": "https://www.ebay.co.uk/...",
  "cdp_state": "connected",
  "captcha_intercepted": false,
  "warnings": [],
  "blockers": []
}
```

Possible `signin_status` values:
- `"signed_in"`: chooser navigated, target email matched a chooser row, click succeeded, redirect to `parent_window_origin` observed within timeout.
- `"manual_required"`: chooser visible but blocked by interactive challenge (2FA / captcha / "Verify it's you" interstitial) — skill returns immediately without interacting.
- `"blocked_no_account"`: chooser visible, BUT no row matches `target_email`. Means the operator's Chrome doesn't have `dieter.x.s.danicek@gmail.com` signed in to Google itself. Operator must visit `chrome://chrome-signin` and add the account before this skill can work.
- `"blocked_chooser_absent"`: navigated to expected chooser URL but the chooser DOM never appeared (URL doesn't reach `accounts.google.com/...`, or page is a Google login form asking for password). Means NO Google account is signed into the persistent Chrome profile at all — operator must first complete Google signin in `chrome://chrome-signin`.
- `"blocked"`: generic fallback (network error, timeout without specific signal, etc.).

**Detection signals — ordered by priority (check earliest matches first):**

| Signal | Page contains | Returns |
|---|---|---|
| Google itself wants signin | `<input type="email" name="identifier">` (Google's email-entry form) | `"blocked_chooser_absent"` — operator must sign in to Google first |
| Recaptcha challenge | iframe `[src*="recaptcha"]` | `"manual_required"` |
| 2FA / "Verify it's you" | heading text matches `Verify it's you` OR `2-Step Verification` | `"manual_required"` |
| Account chooser present | container `[data-identifier]` with multiple `[data-identifier]` rows | proceed to row matching |
| Row matches target_email | row where `[data-identifier="dieter.x.s.danicek@gmail.com"]` OR row text contains substring `dieter.x.s.danicek` | click row, poll for redirect |
| No row matches | chooser present, no matching row | `"blocked_no_account"` |

**Empirical Phase A discovery (mandatory before writing the SKILL.md):**

1. Run `bin/ensure-cdp-9222`. Confirm Chrome is up with `dieter.x.s.danicek@gmail.com` signed in (visible at `chrome://chrome-signin`).
2. Navigate Chrome manually to a known Google OAuth flow — for example `https://accounts.google.com/AccountChooser?continue=https://myaccount.google.com/`. This is a stable Google URL that always shows the chooser when ≥1 account is signed in.
3. Open DevTools Elements. Find the account row for `dieter.x.s.danicek@gmail.com`. **Record the exact selector** — likely one of:
   - `[data-identifier="dieter.x.s.danicek@gmail.com"]` (most stable)
   - `[data-email="dieter.x.s.danicek@gmail.com"]` (older)
   - `li[role="link"][aria-label*="dieter"]` (semantic fallback)
4. **Test each selector via Chrome console**: `document.querySelectorAll('<your selector>').length` should return `1`. If it returns 0, the selector is wrong. If it returns >1, narrow it.
5. **Test the click behaviour**: in console, run `document.querySelector('<selector>').click()`. Observe whether URL changes to `myaccount.google.com` within ~5s. If yes, your selector + click work.
6. Now log out of Google in Chrome (`chrome://chrome-signin` → Sign out). Repeat step 2. Observe that the chooser is REPLACED by Google's email-entry form (`input[name="identifier"]` present). This is the `"blocked_chooser_absent"` signal.
7. Sign back into Google with `dieter.x.s.danicek@gmail.com`.
8. Document all 6 selectors discovered (chooser-container, target-row, email-entry-form, recaptcha-iframe, 2fa-heading, success-redirect-url) inline in the SKILL.md you build for `auth-signin/google-account-chooser/`. Update the table above with the empirical values.

### Step 2 — Create per-site `auth-signin/{ebay,alibaba,searates,freightos}/` (~45-60 min each, 3-4 hr total)

Each is a thin skill that:
1. Navigates to the site's homepage.
2. Detects logged-out state (verifies the operator really IS logged out — guards against unnecessary auth attempts).
3. Clicks the site-specific "Sign In" button.
4. Clicks "Continue with Google" in the resulting modal.
5. Delegates to `auth-signin/google-account-chooser/` for the account selection + redirect.
6. Verifies signin success by checking for logged-in state signal (avatar, account menu, etc.).
7. Returns `signin_status: "signed_in"` on success.

**Per-site click-sequence specification** (verbatim from operator's 2026-05-17 instructions). Each skill follows the same template: `agent-browser open → wait → click → wait → click → wait → eval-to-verify`. The placeholder selectors get filled in during Phase A discovery (see "Empirical Phase A discovery" instructions at the end of this section).

#### `auth-signin/ebay/` — PRIORITY SITE per operator

**Pre-build context — read these files FIRST:**
- `skills/marketplace-search/ebay-uk/SKILL.md` lines 109-170 — shows how the existing ebay-uk skill drives the page (uses `agent-browser open`, `wait`, `click`, `snapshot`, no custom eval). Match this pattern.
- The existing ebay-uk skill works ANONYMOUSLY today — it never had to deal with the Sign In button. Adding auto-signin is genuinely new territory for this skill family. Phase A discovery is essential.

**Workflow:**
1. `agent-browser --cdp ${CDP_PORT} open "https://www.ebay.co.uk/"` — wait 3s.
2. **Verify NOT already logged in** — eBay shows different headers depending on signin state:
   - Signed in: top-right shows `<span>Hi <name>!</span>` with avatar.
   - Signed out: top-right shows `Sign in or register` link.
   - Detection eval: `document.querySelector('a[href*="signin.ebay.co.uk"]') !== null` returns `true` when signed out.
3. Click the Sign In link/button. Selectors to discover in Phase A (likely candidates — verify each):
   - `a[href*="signin.ebay.co.uk"]` (the top-right signin link)
   - `a#gh-ug-flex` (eBay's user-greeting flex anchor)
   - `[data-test-id="topnav-signin"]` (if exists)
4. After signin page loads (URL is `signin.ebay.co.uk/...`), find and click "Continue with Google":
   - eBay's signin page uses a social-login section with multiple OAuth provider buttons.
   - Likely selector: `button[aria-label*="Google"]` OR `[data-social="google"]` OR a button whose innerText contains "Google".
   - Discover empirically in Phase A.
5. **After clicking, the browser navigates to `accounts.google.com/o/oauth2/...`** — the Google account chooser. Delegate to the `google-account-chooser` shared logic (copy its click sequence verbatim into this skill's body — `agent-browser click <chooser-row-selector>`).
6. After chooser click, browser redirects back to `ebay.co.uk` (might pass through `signin.ebay.co.uk/socialsignin/...` first). Wait up to 10s for the redirect.
7. **Verify signed in:** detection eval — `document.querySelector('a[href*="signin.ebay.co.uk"]') === null` (signin link is GONE) AND `/Hi |Hello |\\bMy eBay\\b/i.test(document.body.innerText)` is `true`.

**Failure cases specific to eBay:**
- eBay shows a captcha after Continue with Google (defense-in-depth) → `manual_required`.
- eBay redirects to a "Verify your account" page → `manual_required`.

#### `auth-signin/alibaba/` — PRIORITY SITE per operator

**Pre-build context — read these files FIRST:**
- `skills/marketplace-search/alibaba/SKILL.md` lines 32-300 — the most detailed marketplace skill in the codebase. Has its OWN section on launching Chrome (lines 32-66) and detecting auth state. Read carefully. Its Mandatory CDP Discipline assumes `passport.alibaba.com` login already happened.
- The existing alibaba skill DOES require login (per the audit table). It currently assumes pre-auth from the persistent profile. After this handoff, it gets auto-signin recovery.

**Workflow:**
1. `agent-browser --cdp ${CDP_PORT} open "https://login.alibaba.com/"` — direct login URL is faster than the homepage.
   - Wait 2-3s.
   - Alternative if direct login URL doesn't work: open `https://www.alibaba.com/`, then click the Sign In link in the header.
2. **Verify on login page (not already signed in):** `window.location.hostname.includes("login.alibaba.com") || window.location.hostname.includes("passport.alibaba.com")` returns `true`. If false (page already redirected back to alibaba.com), already signed in — skip the rest, return `signed_in`.
3. Find and click "Continue with Google" on Alibaba's login page. Alibaba's login form has multiple OAuth providers. Likely selectors:
   - `button.google-signin-button`
   - `[data-spm*="google"]` (Alibaba uses `data-spm` for analytics tracking)
   - Button whose innerText contains "Google"
   - Or an `<a>` tag with `href` containing `accounts.google.com/o/oauth2`
   - Discover in Phase A.
4. Browser navigates to Google account chooser. Click the Dieter Danicek row (selector from `google-account-chooser` discovery).
5. Wait for redirect back to `alibaba.com` (passes through `passport.alibaba.com/oauth2/...`).
6. **Verify signed in:** `document.querySelector('[class*="account-menu"], [class*="user-info"]') !== null` (My Alibaba menu visible) AND URL hostname is `alibaba.com` (not passport).

**Failure cases specific to Alibaba:**
- Alibaba shows a "Slide to verify" CAPTCHA on the login page (very common after multiple OAuth attempts in short succession) → `manual_required`.
- Alibaba's account-binding flow appears (if this Google account hasn't been linked to an Alibaba account before) — page asks "Bind Google to existing Alibaba account?" → `manual_required` (operator must complete one-time binding manually).
- Alibaba shows a country/region prompt (operator's IP doesn't match account region) → `manual_required`.

#### `auth-signin/searates/`
1. `agent-browser --cdp ${CDP_PORT} open "https://www.searates.com/login"` (direct URL).
2. Verify NOT logged in: `agent-browser eval 'localStorage.getItem("s-token")' returns empty/null`.
3. Click "Continue with Google" (SeaRates' social-login row). Phase A discovery: open the login page and inspect; selectors are unknown until verified.
4. Click Dieter Danicek row in Google chooser.
5. Wait for redirect back to `searates.com`.
6. **Verify signed in:** `agent-browser eval 'localStorage.getItem("s-token")'` returns a non-empty string.

#### `auth-signin/freightos/` (also referenced from `HANDOFF-freightos-freight-estimate.md`)
1. `agent-browser --cdp ${CDP_PORT} open "https://ship.freightos.com/login"` (verify URL during Phase A — Freightos may use `/auth/signin` or similar).
2. Verify NOT logged in: navigate to `ship.freightos.com`, check for "D" avatar absence.
3. Click "Continue with Google".
4. Click Dieter Danicek row in Google chooser.
5. Wait for redirect back to `ship.freightos.com`.
6. **Verify signed in:** "D" avatar visible top-right (matches the existing Freightos build handoff's `screenshots/freightos/freightos-step-01.png` reference).

**Each per-site SKILL.md follows the standard CDP-skill template** (frontmatter with `requires_cdp: true`, `cdp_default_port: 9222`, "Mandatory CDP Discipline" section calling `bin/ensure-cdp-9222`, bounded ~60-second attempt budget, captcha-simulation hook, result schema). Use the SeaRates SKILL.md as the template per the previous auto-launch handoff.

### Step 3 — Create `auth-signin/procontract/` (~60-90 min — more complex)

ProContract does NOT use Google OAuth. The flow uses macOS / Chrome's password autofill against a saved credential for `sales@sorelon.com`.

**KEY ARCHITECTURAL CAVEAT:** the password manager popup is a **Chrome UI element**, NOT a DOM element. The skill cannot detect it via `document.querySelector`. The Touch ID prompt is a **macOS OS-modal**, similarly invisible to the DOM. Both must be detected **behaviourally** (poll for downstream DOM signals like field population or URL change) rather than directly.

**Click sequence:**

1. `agent-browser --cdp ${CDP_PORT} open "https://procontract.due-north.com/Login"`.
2. Verify NOT already logged in: `agent-browser eval` returns `true` for the JS expression `window.location.pathname.includes("/Login") || document.body.innerText.includes("Login or register for free")`.
3. Find and click the "login here" link inside the "Login or register for free" section. Selector discovered in Phase A; typical: `a[href*="Login"]` within the relevant card.
4. Click the username input field. (Selector: `input[name="username"]` or `input[type="email"]` within the login form — discover in Phase A.) **This triggers Chrome's autofill suggestion popup AT THE BROWSER UI LEVEL — not visible to DOM queries.** The skill cannot inspect or click the popup directly.
5. **Behavioural detection that autofill happened:**

   ```bash
   # Poll the username field's value for ~3 seconds. If autofill fires, the field
   # will populate with sales@sorelon.com without the skill typing anything.
   for i in 1 2 3 4 5 6; do
     USERNAME_VALUE=$(agent-browser --cdp ${CDP_PORT} eval 'document.querySelector("input[name=username]")?.value || ""' 2>/dev/null | tr -d '"' | tr -d '[:space:]')
     if [[ "$USERNAME_VALUE" == "sales@sorelon.com" ]]; then
       AUTOFILL_FIRED=true
       break
     fi
     sleep 0.5
   done

   if [[ "${AUTOFILL_FIRED:-false}" != "true" ]]; then
     # No autofill within 3 seconds. Likely causes:
     # (a) Chrome password manager disabled at chrome://settings/passwords.
     # (b) The saved credential for sales@sorelon.com has been deleted.
     # (c) macOS Passwords sync glitch.
     # Either way, the skill cannot proceed silently — escalate.
     # Return signin_status: "manual_required" with explicit operator guidance.
     return_manual_required "Chrome password autofill did not populate the username field within 3 seconds. Either Chrome password autofill is disabled (enable at chrome://settings/passwords), the saved credential for sales@sorelon.com is missing, OR the operator must sign in manually in the persistent browser tab."
   fi
   ```

6. Once autofill fires (both username AND password populated), click the "Continue" / "Log In" submit button.
7. **Behavioural detection of post-submit outcomes** — poll for ONE of three signals within a 12-second budget (more aggressive than the previous 30s — Touch ID is the slowest legitimate path and typically completes in 5-8s if the operator is present):

   ```bash
   for i in $(seq 1 24); do   # 24 × 0.5s = 12 seconds
     CURRENT_URL=$(agent-browser --cdp ${CDP_PORT} eval 'window.location.href' 2>/dev/null | tr -d '"')
     ERROR_TEXT=$(agent-browser --cdp ${CDP_PORT} eval 'document.querySelector(".alert-danger, .error, [role=alert]")?.innerText || ""' 2>/dev/null | tr -d '"')

     # Signal A: signed in successfully (URL no longer contains /Login)
     if [[ "$CURRENT_URL" != *"/Login"* ]]; then
       OUTCOME="signed_in"
       break
     fi

     # Signal B: explicit error message shown
     if [[ -n "$ERROR_TEXT" ]]; then
       OUTCOME="blocked"
       break
     fi

     # Otherwise: still on /Login, no error. Could be Touch ID waiting, network slow, or
     # ProContract server rate-limiting. Keep polling.
     sleep 0.5
   done

   # After 12 seconds, if still on /Login with no error → likely Touch ID prompt is
   # OS-modal-blocking the page. Escalate with clear guidance.
   if [[ -z "${OUTCOME:-}" ]]; then
     OUTCOME="manual_required"
     ESCALATION_MSG="ProContract login submit returned no progress within 12 seconds (no URL change, no error message). Most likely macOS Touch ID prompt is waiting at the OS level — the operator must physically authenticate at the Touch ID prompt, then re-trigger the job. Less likely: network is slow OR ProContract is rate-limiting. Operator action: check for a pending Touch ID prompt; if none, retry once."
   fi
   ```

8. Based on `OUTCOME`, write the appropriate `result.json` (signed_in / blocked / manual_required).

**12-second timeout rationale:** Touch ID typically completes within 5-8 seconds when the operator is present and ready. If the operator is away from the machine, Touch ID times out after ~30s — but waiting 30s burns the skill's daemon budget. 12s is a pragmatic middle ground: fast enough to keep the skill responsive, long enough to allow a present operator to authenticate.

**Sensitivity warning (read this carefully):** the ProContract account `sales@sorelon.com` is the operator's REAL procurement account with bid history and financial implications. The skill MUST NOT:
- Submit forms beyond the login form (no tender registration, no profile changes).
- Modify any account settings.
- Type the password directly — rely on autofill only. (If autofill never fires, escalate — DON'T fall back to typing.)
- Log out at session end — subsequent skills need the session.
- Cache or log the autofilled values to disk.

The skill's job is narrow: click → wait for autofill → click submit → wait for outcome → return. Nothing else.

**Sensitivity note:** the ProContract account `sales@sorelon.com` is the operator's actual procurement account with bid history. The skill MUST NOT:
- Attempt to register interest in tenders.
- Modify account settings.
- Submit forms beyond the login form.
- Log out at the end (subsequent skills need the session).

Only login → wait for redirect → return. Nothing else.

### Step 4 — Wire auto-signin into the 8 affected production skills (~2-3 hr)

For each of these production skills, add a "Session Recovery" subsection AFTER the existing "Mandatory CDP Discipline" section:

```
marketplace-search/ebay-uk                           (NEW — currently anonymous; optional auto-signin)
marketplace-search/alibaba                           (REPLACE existing pre-auth-only model)
landed-cost/searates-freight-estimate                (REPLACE existing SEARATES_SESSION_EXPIRED blocker emission)
landed-cost/shipment-sizing                          (delegate to auth-signin/alibaba via shared profile)
source-health/alibaba                                (delegate to auth-signin/alibaba)
procurement-ingestion/procontract-discover           (REPLACE existing PROCONTRACT_SESSION_EXPIRED behavior)
procurement-ingestion/procontract-enrich-tender      (same)
procurement-ingestion/procontract-register-interest  (same)
```

**Replacement pattern** — insert after the "Mandatory CDP Discipline" block. **IMPORTANT design note (verified against existing skills):** existing CDP skills (`marketplace-search/ebay-uk`, `marketplace-search/alibaba`) drive the browser using `agent-browser` subcommands (`open`, `click`, `wait`, `snapshot`) — NOT `agent-browser eval`. The Session Recovery pattern matches that convention: it inlines the per-site click sequence using `agent-browser` subcommands. The matching `skills/auth-signin/<SITE>/SKILL.md` is the CANONICAL documentation of those click sequences (selectors discovered during Phase A); production skills copy the sequence verbatim from there.

This deliberately does NOT spawn a separate `agent-browser eval` subprocess or read a separate `eval-body.js` file (neither convention exists today). Inline `agent-browser` subcommands match existing skill DNA.

```markdown
## Session Recovery (NEW 2026-05-17)

If the operation detects a logged-out signal (per-site signals listed below),
run the per-site signin sequence INLINE using `agent-browser` subcommands. The
canonical sequence is documented in `skills/auth-signin/<SITE>/SKILL.md` §Workflow;
copy it verbatim into this section, then verify success and either succeed-and-continue
or escalate.

### Site-specific session-expired detection

Per-site signals (discovered empirically during Phase A; update if discovery finds different selectors):
- (e.g., for eBay) URL after main navigation contains `/signin/`, OR `agent-browser eval` returns `false` for `document.querySelector("[data-test-id=user-account]")`.
- (e.g., for SeaRates) `agent-browser eval` returns `null` for `localStorage.getItem("s-token")`.
- (e.g., for ProContract) URL contains `/Login` after navigation, OR page text matches `Session expired|Please log in`.

### Recovery flow

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# 1. Ensure CDP is up (defense in depth — daemon's pre-flight should already have done this).
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222 || {
  cat > result.json <<'BLOCKED'
{ "<STATUS_FIELD>": "blocked", "cdp_state": "unreachable", "captcha_intercepted": false,
  "warnings": [], "blockers": [{"code": "CDP_CONNECT_FAILED", "message": "ensure-cdp-9222 failed", "severity": "blocked"}] }
BLOCKED
  exit 0
}

# 2. Run the per-site signin sequence (COPY VERBATIM from
#    skills/auth-signin/<SITE>/SKILL.md §Workflow — discovered in Phase A).
#    Example for Alibaba (selectors are PLACEHOLDERS until Phase A discovery completes;
#    the auth-signin/alibaba SKILL.md is the source of truth):
agent-browser --cdp ${CDP_PORT} open "https://login.alibaba.com/"
agent-browser --cdp ${CDP_PORT} wait 2000
agent-browser --cdp ${CDP_PORT} click "<CSS selector for 'Continue with Google' button — from Phase A>"
agent-browser --cdp ${CDP_PORT} wait 3000

# Now on Google account chooser. Click Dieter Danicek's row.
# (auth-signin/google-account-chooser/SKILL.md documents the selector pattern.)
agent-browser --cdp ${CDP_PORT} click "<CSS selector matching dieter.x.s.danicek@gmail.com row>"
agent-browser --cdp ${CDP_PORT} wait 5000

# 3. Verify signed in (per-site logged-in signal — discovered in Phase A).
SIGNED_IN=$(agent-browser --cdp ${CDP_PORT} eval '<JS expression returning true if avatar/menu visible>' 2>/dev/null | tr -d '[:space:]')

if [[ "$SIGNED_IN" != "true" ]]; then
  cat > result.json <<'BLOCKED'
{
  "<STATUS_FIELD>": "blocked",
  "cdp_state": "session_expired",
  "captcha_intercepted": false,
  "warnings": [],
  "blockers": [{
    "code": "SESSION_REFRESH_FAILED",
    "message": "Auto-signin via Google OAuth attempted but logged-in signal not detected after click sequence. Likely causes: (a) Google chooser empty (no Google account signed into Chrome — operator action: sign in at chrome://chrome-signin); (b) 2FA / captcha / 'Verify it's you' interstitial (operator action: complete the challenge manually in the persistent browser); (c) site-side captcha after redirect (operator action: solve in browser). Inspect Chrome's current tab to determine the cause.",
    "severity": "blocked"
  }]
}
BLOCKED
  exit 0
fi

# 4. Signed in successfully. EXIT CLEANLY with no result.json written.
#    The daemon's existing retry loop will re-invoke this skill from scratch on
#    the next retry pass — the second invocation will see the restored session
#    and complete the main operation normally.
#
#    Why daemon-retry instead of in-skill retry: re-running the entire main eval
#    inline would require either (a) wrapping main op in a bash function and
#    calling it twice (fragile across skills with different main-op shapes), or
#    (b) using exec to restart the skill (loses captured state). Daemon retry is
#    the existing convention — see crates/daemon/src/jobs/runner.rs retry policy.
#
#    The daemon will mark this attempt as "no result.json written" → retry-able.
#    Daemon retries automatically (with backoff per existing policy). Operator
#    sees one extra retry attempt in logs; net result: signin succeeded, main op
#    succeeds on retry.
exit 0
```

**Replace `<STATUS_FIELD>`** with the skill's existing field name (e.g., `freight_status`, `search_status`, `ingestion_status` — grep the skill's existing body to find it; don't rename).

**Replace selector placeholders `<...>`** with the actual CSS selectors discovered during Phase A of building `skills/auth-signin/<SITE>/`. Until Phase A is done, leave the placeholders + a `// TBD` comment.
```

**Per-skill specifics:**

| Production skill | auth-signin to call | Detection signal | Status field |
|---|---|---|---|
| `marketplace-search/ebay-uk` | `auth-signin/ebay` | Sign In button visible | `search_status` |
| `marketplace-search/alibaba` | `auth-signin/alibaba` | URL redirect to passport.alibaba.com OR `error_code: USER_NOT_LOGIN` | `search_status` |
| `landed-cost/searates-freight-estimate` | `auth-signin/searates` | `localStorage` s-token null/expired | `freight_status` |
| `landed-cost/shipment-sizing` | `auth-signin/alibaba` (shared profile) | Same as marketplace-search/alibaba | `sizing_status` |
| `source-health/alibaba` | `auth-signin/alibaba` | Same as marketplace-search/alibaba | `health_status` |
| `procurement-ingestion/procontract-discover` | `auth-signin/procontract` | URL contains `/Login` after navigation | `ingestion_status` |
| `procurement-ingestion/procontract-enrich-tender` | same | same | same |
| `procurement-ingestion/procontract-register-interest` | same | same | same |

### Step 5 — Update Freightos in-progress handoff (~15 min)

`docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/HANDOFF-freightos-freight-estimate.md`

Add a new "Phase A.8.1 — Session-expired auto-signin integration" sub-step in the Discovery section. Specify:

- Detection signal: after the Freightos search API POST, if HTTP 401 / 302 to `/login`, treat as session-expired.
- Recovery flow: invoke `auth-signin/freightos/` subprocess, retry POST once, emit `FREIGHTOS_SESSION_EXPIRED` only if second attempt also fails.
- Update the existing FAILURE-PATH test `fp-03-session-expired` to verify recovery: after logout, skill should call auth-signin/freightos, succeed, and return `freight_status: "quoted"` (not `blocked`).

Also note in the handoff: building `auth-signin/freightos/` is now a PREREQUISITE for Freightos skill build (alongside the existing pre-reqs).

### Step 6 — Update Taric handoff (~5 min)

`docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/HANDOFF-taric-cross-check.md`

Add a one-paragraph "Session Recovery: N/A" note in the architectural-decisions section:

```markdown
### Auto-signin: NOT APPLICABLE for this skill

TARIC is free public EU customs data — no login mechanism exists. The
auth-signin skill family (per `docs/handoff-2026-05-17-cdp-auto-signin.md`)
does not have a `auth-signin/taric/` skill, and the production skill does
not include a Session Recovery section. If the skill ever encounters an
unexpected login wall (would indicate a TARIC infrastructure change), emit
`TARIC_UNEXPECTED_LOGIN_WALL` as a blocker — but this is purely defensive;
TARIC has been a public EU resource since 1992.
```

### Step 7 — Update AGENTS.md (~10 min)

After the "CDP Browser Prerequisite (port 9222)" section (added by the prior auto-launch handoff), append:

```markdown
## Session Recovery for Authenticated CDP Sites

Sites that require login (eBay, Alibaba, SeaRates, Freightos, ProContract) silently log the operator's persistent profile out over time. Skills targeting these sites detect the logged-out state and invoke the matching `auth-signin/{site}/` skill to automatically restore the session.

**Sign-in mechanisms:**
- Google OAuth (`Continue with Google` → select `dieter.x.s.danicek@gmail.com`): eBay, Alibaba, SeaRates, Freightos.
- Password manager autofill (`sales@sorelon.com`): ProContract.
- Manual operator-only (cookie refresh): 1688, Alibaba's `auth-refresh/` skills (no Google OAuth support).

**Escalation cases (operator must intervene):**
- ProContract Touch ID prompt → operator physically authenticates.
- Google 2FA / captcha → operator approves or solves.
- Password autofill disabled → operator enables in `chrome://settings/passwords`.
- Operator's Google account not signed into Chrome → operator signs in via `chrome://chrome-signin`.

When auto-signin escalates, the production skill emits `SESSION_REFRESH_FAILED` with a message naming the specific operator action needed.

**Skills with NO auto-signin (intentional):**
- `marketplace-search/{amazon-uk,gem-imports,mx-wholesale}` — anonymous access is sufficient.
- `procurement-ingestion/{contracts-finder,fts}` — public OCDS APIs.
- `source-health/fts` — public.
- `landed-cost/{hmrc-customs-classification,simple-fx-lookup}` — public APIs (HMRC, Frankfurter).
- `auth-refresh/{alibaba,1688}` — these ARE the manual-refresh skills.

See `docs/handoff-2026-05-17-cdp-auto-signin.md` for the full design.
```

### Step 8 — Tests (~60 min)

Create `crates/daemon/tests/auth_signin_lint.rs`:

```rust
//! ut_asln_001..099 — lint asserting auth-signin skill structure.

const AUTH_SIGNIN_SKILLS: &[&str] = &[
    "auth-signin/google-account-chooser",
    "auth-signin/ebay",
    "auth-signin/alibaba",
    "auth-signin/searates",
    "auth-signin/freightos",
    "auth-signin/procontract",
];

#[test]
fn ut_asln_001_auth_signin_skills_exist() {
    for skill_id in AUTH_SIGNIN_SKILLS {
        let path = skills_root().join(skill_id).join("SKILL.md");
        assert!(path.exists(), "missing auth-signin skill: {}", skill_id);
    }
}

#[test]
fn ut_asln_002_auth_signin_skills_declare_requires_cdp() {
    for skill_id in AUTH_SIGNIN_SKILLS {
        let body = std::fs::read_to_string(skills_root().join(skill_id).join("SKILL.md")).unwrap();
        assert!(body.contains("requires_cdp: true"), "{} missing requires_cdp:true", skill_id);
        assert!(body.contains("cdp_default_port: 9222"), "{} missing cdp_default_port:9222", skill_id);
    }
}

#[test]
fn ut_asln_003_auth_signin_skills_invoke_ensure_helper() {
    // Inherits the rule from the prior handoff: every CDP-requiring skill
    // must invoke bin/ensure-cdp-9222 in its body.
    for skill_id in AUTH_SIGNIN_SKILLS {
        let body = std::fs::read_to_string(skills_root().join(skill_id).join("SKILL.md")).unwrap();
        assert!(body.contains("ensure-cdp-9222"), "{} does not invoke ensure-cdp-9222", skill_id);
    }
}

#[test]
fn ut_asln_004_production_skills_invoke_auth_signin() {
    // Skills that need auto-signin must invoke the matching auth-signin in their body.
    let mapping: &[(&str, &str)] = &[
        ("marketplace-search/ebay-uk", "auth-signin/ebay"),
        ("marketplace-search/alibaba", "auth-signin/alibaba"),
        ("landed-cost/searates-freight-estimate", "auth-signin/searates"),
        ("landed-cost/shipment-sizing", "auth-signin/alibaba"),
        ("source-health/alibaba", "auth-signin/alibaba"),
        ("procurement-ingestion/procontract-discover", "auth-signin/procontract"),
        ("procurement-ingestion/procontract-enrich-tender", "auth-signin/procontract"),
        ("procurement-ingestion/procontract-register-interest", "auth-signin/procontract"),
    ];
    for (prod, signin) in mapping {
        let body = std::fs::read_to_string(skills_root().join(prod).join("SKILL.md")).unwrap();
        assert!(body.contains(signin), "{} does not invoke {}", prod, signin);
    }
}
```

### Step 9 — Verification (~30 min)

#### A. Skill structure

- [ ] `skills/auth-signin/` directory exists with 6 sub-skills (`google-account-chooser`, `ebay`, `alibaba`, `searates`, `freightos`, `procontract`).
- [ ] Each has SKILL.md + result.schema.json + (no checkpoint.schema.json — short atomic skills).
- [ ] Each SKILL.md has `requires_cdp: true` + `cdp_default_port: 9222`.
- [ ] Each invokes `bin/ensure-cdp-9222` per the auto-launch handoff convention.
- [ ] `cargo test --test auth_signin_lint` passes (4 lints above).

#### B. Production skill integration

For each of the 8 affected production skills:
- [ ] Has a "Session Recovery" section after "Mandatory CDP Discipline".
- [ ] Invokes the correct `auth-signin/{site}` skill.
- [ ] Has per-site session-expired detection signals documented.
- [ ] Retries the main operation ONCE on signin success (not infinite loop).
- [ ] Emits `SESSION_REFRESH_FAILED` only on signin `manual_required` outcome.

#### C. End-to-end empirical verification (per site, ~15 min each)

For each Google-OAuth site (eBay, Alibaba, SeaRates, Freightos):
1. With Chrome on port 9222, manually sign OUT of the site in the browser.
2. Trigger the affected production skill (e.g., shipment-sizing for Alibaba).
3. Watch logs: skill detects logged-out → invokes auth-signin/{site} → auth-signin opens site / clicks Sign In / clicks Continue with Google / picks Dieter Danicek / verifies redirect → skill retries main operation → succeeds.
4. Confirm `result.json` shows successful main operation (NOT `SESSION_REFRESH_FAILED`).

For ProContract:
5. Sign out of ProContract in browser.
6. Trigger procontract-discover.
7. Watch: skill detects logged-out → invokes auth-signin/procontract → password manager popup → autofill → submit → IF Touch ID prompts: pause, operator authenticates → skill retries → succeeds.
8. If Touch ID does NOT prompt (Chrome+macOS magic): skill completes without operator action.

### Step 10 — Write TEST_REPORT.md (~15 min)

`docs/testing/2026-05-17-cdp-auto-signin-test-report.md`:

```markdown
# CDP Auto-Signin — Test Report

**Date:** YYYY-MM-DD
**Tested by:** <operator>
**Skills built:** 6 auth-signin sub-skills + 8 production skill updates

## Verdict
**STATUS: READY FOR PRODUCTION** (or BLOCKED if any check failed)

## Per-site empirical results (recovery from manual logout)

| Site | Sign-out method | auth-signin time | Production skill recovery time | Verdict |
|---|---|---|---|---|
| eBay | Click avatar > Sign out | ~8s | ~12s total | PASS |
| Alibaba | Click avatar > Logout | ~10s | ~15s | PASS |
| SeaRates | Clear localStorage.s-token | ~9s | ~13s | PASS |
| Freightos | Click avatar > Logout | ~10s | ~14s | PASS |
| ProContract (no fingerprint) | Browser logout button | ~7s | ~11s | PASS |
| ProContract (with fingerprint) | Browser logout button | escalates after ~10s | emits SESSION_REFRESH_FAILED | PASS (correct escalation) |

## Escalation cases verified

- [x] Touch ID prompt → manual_required (operator informed) ✓
- [x] Google 2FA prompt → manual_required ✓
- [x] Google captcha → manual_required ✓
- [x] Password manager disabled → manual_required with clear message ✓

## Discoveries (per site)

(record selectors discovered in Phase A for each auth-signin skill)
```

---

## Rollback plan

The new auth-signin skills are additive. To roll back:

1. **Revert production-skill SKILL.md edits:** restore Mandatory CDP Discipline back to its prior shape (no Session Recovery section). `git restore $(find skills -name SKILL.md -type f)`.
2. **Revert daemon test additions:** `git rm crates/daemon/tests/auth_signin_lint.rs` if desired.
3. **Leave the `skills/auth-signin/` directory in place:** harmless when not invoked. Operator can still invoke `auth-signin/{site}` manually as a one-off recovery skill.
4. **Leave AGENTS.md additions:** documentation only.

Total rollback time: < 15 min.

---

## Open questions / things the operator should confirm before delegation

The handoff makes these assumptions. If wrong, flag and adjust:

1. **Amazon UK stays anonymous?** Currently the skill works fine without login; adding auto-signin is unnecessary unless the operator wants to access account-bound features (saved searches, Subscribe & Save pricing). Default: skip.
2. **1688 stays manual?** 1688 has no Google OAuth (QR code / WeChat / Alibaba password only). Default: keep existing `auth-refresh/1688` manual flow.
3. **Gem Imports / MX Wholesale stay anonymous?** Both work for logged-out visitors today. MX Wholesale has some login-gated prices but the skill already skips those. Default: skip auto-signin.
4. **Source-health/alibaba auto-signs in if logged-out?** Currently it just reports red on auth failure. The handoff proposes auto-signin so the health check can complete successfully even after expiry. Default: yes, auto-signin.
5. **SeaRates: Google OAuth replaces s-token flow OR augments it?** The existing skill uses an s-token. The handoff proposes Google OAuth as the RECOVERY path when s-token is null/expired (auto-signin via Google → site reissues s-token → skill caches new token). Original s-token fast-path stays.
6. **Order of execution:** this handoff DEPENDS on the prior auto-launch handoff (`docs/handoff-2026-05-17-cdp-9222-auto-launch.md`). Auto-launch must be executed FIRST.
7. **Build sequence:** Step 1 (google-account-chooser) before Step 2 (per-site skills) because Step 2 delegates to Step 1. Step 3 (ProContract) is independent. Step 4 (production skill updates) requires all auth-signin skills to exist first.

---

## Notes for the next agent

If you (operator) delegate this build:

- Read `docs/handoff-2026-05-17-cdp-9222-auto-launch.md` FIRST and confirm `bin/ensure-cdp-9222` exists before starting any auth-signin work.
- Read the audit findings table at the top of this handoff and confirm site coverage. Do NOT invent new auth-signin skills not in the table.
- For each auth-signin skill, do Phase A discovery (DevTools selector inspection) AGAINST THE LIVE SITE before writing SKILL.md. Sign-in button selectors change.
- Google OAuth flow specifics (account chooser DOM) change quarterly. If chooser logic stops working in a few months, re-discover in Phase A.
- Never invent credentials. Use only:
  - Google account: `dieter.x.s.danicek@gmail.com`
  - ProContract: `sales@sorelon.com` (via password manager autofill — never type the password directly).
- If you discover that any site has 2FA / mandatory captcha / Touch ID, do NOT try to bypass — that's an escalation case, return `manual_required`.

---

## Outcome when complete

- The operator never has to manually sign back into a site after auto-logout (except true escalation cases: 2FA, captcha, Touch ID, password-autofill-disabled).
- Skills that previously emitted `*_SESSION_EXPIRED` blockers now self-recover and complete successfully.
- ProContract Touch ID stays a manual escalation — by design, not a regression.
- 1688 stays a manual escalation — by design, no Google OAuth available.
- Daemon retry loops dramatically reduce "stuck on session expired" failure modes.
