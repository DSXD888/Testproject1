---
skill_id: auth-signin/alibaba
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: local_state
requires_cdp: true
cdp_default_port: 9222
maxPages: 3
maxScrollCycles: 1
---

# auth-signin/alibaba

## Use when
Use this skill when a downstream Alibaba-using skill (`marketplace-search/alibaba`, `landed-cost/shipment-sizing`, `source-health/alibaba`) has detected that the operator's persistent Chrome profile is signed OUT of Alibaba (`passport.alibaba.com` redirect, `USER_NOT_LOGIN` API error, Sign In link in header) and needs the session restored before its main operation can complete.

The skill drives the standard "Sign In → Continue with Google → pick `dieter.x.s.danicek@gmail.com` → verify My Alibaba menu present" sequence using already-resident OAuth state. Replaces the previous "operator manually re-signs in via the persistent browser" flow that was the only option before this skill existed (existing `auth-refresh/alibaba` is still the right tool for explicit operator-initiated cookie refresh; THIS skill is for daemon-triggered recovery on session-expired detection).

## Do not use when
Do not invoke when:
- The browser is already signed into Alibaba. The skill no-ops in this case (returns `signin_status: "signed_in"` immediately), but the round-trip wastes time — callers should only invoke after their own logged-out detection.
- The target is `1688.com`. 1688 has no Google OAuth (QR-code / WeChat / Alibaba-credentials only); use the existing `auth-refresh/1688` manual flow.
- The operator's Google account has never been bound to an Alibaba account (first-time Google sign-on triggers a one-time account-binding flow that requires interactive operator input). If this happens, the skill returns `manual_required` with a clear message.

## Inputs
Daemon-injected `job_parameters`:
- `target_email` — Google account to pick. Default: `dieter.x.s.danicek@gmail.com`.

## Outputs
- `result.json` matching `result.schema.json`. Same shape as the chooser sub-skill plus an Alibaba-specific `signed_in_verified_via` field naming which DOM signal confirmed signin (`account_menu_present`, `user_info_present`, or `passport_redirect_absent`).

## Non-goals
- Contacting suppliers, placing inquiries, modifying account settings, changing account region.
- Solving CAPTCHAs (Alibaba's slide-to-verify, image puzzle). Surface as `manual_required`.
- Completing first-time account-binding for a new Google account (the page asking "Bind Google to existing Alibaba account?"). Surface as `manual_required`.
- Switching between Alibaba accounts.
- Closing the operator browser.

## Failure Codes
- `CDP_CONNECT_FAILED` — `bin/ensure-cdp-9222` failed or CDP preflight did not respond.
- `ALIBABA_SIGNIN_PAGE_UNREACHABLE` — opened `login.alibaba.com` but never landed on the expected login form (network error, redirect loop, etc.).
- `ALIBABA_GOOGLE_BUTTON_NOT_FOUND` — landed on Alibaba's login page but could not locate the "Continue with Google" button. Either Alibaba removed Google OAuth in this region or page-shape drift.
- `ALIBABA_SLIDE_CAPTCHA_PRESENT` — Alibaba's slide-to-verify CAPTCHA appeared. Operator action required.
- `ALIBABA_ACCOUNT_BINDING_REQUIRED` — first-time Google → Alibaba account binding flow appeared. Operator action required to complete one-time binding.
- `ALIBABA_REGION_PROMPT_PRESENT` — Alibaba is asking the operator to confirm or change account region. Operator action required.
- `ALIBABA_IDENTITY_VERIFICATION_REQUIRED` — Alibaba routed the post-OAuth flow to `login.alibaba.com/iv/verify_mode.htm` showing "Verify your identity / We've temporarily locked your account / Enter your phone number and we'll send you a code". Triggered when Alibaba's risk engine flags the OAuth attempt (new device, region change, multiple failed attempts). Operator must complete SMS verification manually.
- `ALIBABA_SIGNED_IN_SIGNAL_ABSENT` — completed the OAuth round-trip but cannot confirm signed-in DOM state on `alibaba.com`. Operator inspects manually.
- `GOOGLE_INTERACTIVE_CHALLENGE` / `GOOGLE_TARGET_ACCOUNT_NOT_LISTED` / `GOOGLE_CHOOSER_ABSENT` / `GOOGLE_REDIRECT_TIMEOUT` — propagated from the chooser sub-step.

## Resume Strategy
Not resumable. Atomic.

## Pagination Bounds
- `maxPages`: 3 (login.alibaba.com → Google chooser → alibaba.com redirect)
- `maxScrollCycles`: 1

---

## Playbook

You are an Alibaba auto-signin agent.

### Mandatory CDP Discipline

This skill MUST attach to the operator's persistent Chrome. Do not launch a fresh browser, do not solve CAPTCHAs, do not type credentials, do not close the operator browser.

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

# Defensive reachability check. If this fails, write CDP_CONNECT_FAILED and stop.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, write the blocked result with `CDP_CONNECT_FAILED` and stop. Every browser command MUST use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

### Workflow

#### Step 1 — Open Alibaba login page directly

Direct login URL is faster than homepage → header click:

```bash
agent-browser --cdp ${CDP_PORT} open "https://login.alibaba.com/"
agent-browser --cdp ${CDP_PORT} wait 2500
```

#### Step 2 — Verify NOT already signed in

If Alibaba redirected immediately back to `alibaba.com` (operator was actually still signed in despite the caller's stale logged-out detection), no-op:

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const hostname = window.location.hostname;
  const url = window.location.href;
  // On login page → need to sign in
  if (hostname.includes("login.alibaba.com") || hostname.includes("passport.alibaba.com")) {
    return JSON.stringify({state: "on_login_page", url});
  }
  // Already redirected back to alibaba.com main → check for signed-in signals
  const accountMenu = document.querySelector('"'"'[class*="account-menu"], [class*="my-alibaba"], [class*="user-info"]'"'"');
  if (accountMenu) {
    return JSON.stringify({state: "already_signed_in", url, signal: "account_menu_present"});
  }
  // On alibaba.com but no menu visible — slow render or weak signal
  return JSON.stringify({state: "on_alibaba_unverified", url});
})()
'
```

Decision:
- `state: "already_signed_in"` → write `signin_status: "signed_in"`, `signed_in_verified_via: "account_menu_present"`, stop with success.
- `state: "on_login_page"` → proceed to Step 3.
- `state: "on_alibaba_unverified"` → wait 1500ms and re-evaluate once; if still unverified, proceed to Step 3 (the caller's logged-out signal takes precedence over our slow-render observation).
- Unexpected hostname → write blocked with `ALIBABA_SIGNIN_PAGE_UNREACHABLE`.

#### Step 3 — Check for slide-CAPTCHA on the login page

Alibaba serves a slide-to-verify CAPTCHA after multiple OAuth attempts in short succession (rate-limiting defense). Detect early so the operator knows:

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  // Slide CAPTCHA: iframe with nocaptcha / sufei in src, or visible slide-track DOM
  const sliderIframe = document.querySelector("iframe[src*=nocaptcha], iframe[src*=sufei]");
  const sliderDom = document.querySelector("[id*=nc_], .nc-container, [class*=slider-track]");
  return JSON.stringify({slide_captcha: !!(sliderIframe || sliderDom)});
})()
'
```

If `slide_captcha: true`, write blocked result with `ALIBABA_SLIDE_CAPTCHA_PRESENT` and stop.

#### Step 4 — Click "Continue with Google" on Alibaba's login page

Alibaba's login form has multiple OAuth providers. The Google button is the most stable to find by anchoring on the Google OAuth href:

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  // PRIMARY (verified live 2026-05-17): Alibaba renders the Google social-signin
  // as an anchor with attr-type="google" and attr-customurl="<sns_auth.htm URL>".
  // Clicking the anchor is wired via a JS handler that reads attr-customurl, so
  // synthetic .click() on the anchor may not navigate. The robust path is to
  // read attr-customurl and navigate via window.location.assign directly.
  const stable = document.querySelector("a[attr-type=google][attr-customurl]");
  if (stable) {
    const target = stable.getAttribute("attr-customurl");
    if (target) {
      window.location.assign(target);
      return JSON.stringify({clicked: true, selector_used: "a[attr-type=google] (navigated via attr-customurl)"});
    }
  }
  // FALLBACK chain — kept in case Alibaba refactors back to standard OAuth href
  // or A/B-test variants are served:
  const candidates = [
    document.querySelector("a[href*=\"accounts.google.com/o/oauth2\"]"),
    document.querySelector("button.google-signin-button, [data-spm*=google i]"),
    ...Array.from(document.querySelectorAll("button, a, [role=button]")).filter(el => /continue with google|sign in with google/i.test(el.innerText || ""))
  ].filter(Boolean);
  const btn = candidates[0];
  if (!btn) return JSON.stringify({clicked: false, reason: "no_google_button_found", current_url: window.location.href});
  btn.click();
  return JSON.stringify({clicked: true, selector_used: "fallback"});
})()
'
agent-browser --cdp ${CDP_PORT} wait 3500
```

If `clicked: false`, write blocked result with `ALIBABA_GOOGLE_BUTTON_NOT_FOUND`.

#### Step 5 — Confirm we landed on Google's account chooser

```bash
agent-browser --cdp ${CDP_PORT} eval 'JSON.stringify({on_google: window.location.hostname.endsWith("accounts.google.com"), url: window.location.href})'
```

If `on_google: false`, write blocked with `ALIBABA_GOOGLE_BUTTON_NOT_FOUND` and include the `url` in the blocker message (helps diagnose whether the button click misfired or Alibaba intercepted the redirect).

#### Step 6 — Delegate to the Google account chooser logic

(See `auth-signin/google-account-chooser/SKILL.md` for the canonical implementation. Inlined here for the same reasons as ebay/SKILL.md Step 6.)

```bash
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
```

Map chooser results: `captcha`/`twofa_interstitial` → `manual_required` + `GOOGLE_INTERACTIVE_CHALLENGE`; `email_entry_form` → blocked + `GOOGLE_CHOOSER_ABSENT`; `chooser_target_missing` → blocked + `GOOGLE_TARGET_ACCOUNT_NOT_LISTED`; `chooser_target_found` → Step 7.

#### Step 7 — Poll for redirect back to alibaba.com, check for binding/region prompts, verify signed-in

After the chooser click, the browser redirects through `passport.alibaba.com/oauth2/...` back to `alibaba.com`. Four things can happen:
1. **Happy path:** lands on `alibaba.com` with My Alibaba menu visible.
2. **Binding prompt:** Alibaba asks the operator to bind this Google account to an existing Alibaba account (first-time only). Operator handles.
3. **Region prompt:** Alibaba asks to confirm/change account region (when IP doesn't match account region). Operator handles.
4. **Identity verification:** Alibaba routes to `login.alibaba.com/iv/verify_mode.htm` showing "Verify your identity / We've temporarily locked your account / Enter your phone number and we'll send you a code". Triggered when risk engine flags the OAuth attempt. Operator must complete SMS verification manually. Verified live 2026-05-17.

```bash
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const deadline = Date.now() + 18000;
  while (Date.now() < deadline) {
    const url = window.location.href;
    const hostname = window.location.hostname;
    const bodyText = (document.body && document.body.innerText) || "";
    // Binding flow
    if (/bind.*alibaba account|link your google account/i.test(bodyText)) {
      return JSON.stringify({status: "binding_required", url});
    }
    // Region prompt
    if (/select your country|change.*region|account region/i.test(bodyText)) {
      return JSON.stringify({status: "region_prompt", url});
    }
    // Identity verification (SMS code) — risk-engine driven
    if (/verify your identity/i.test(bodyText) && /phone number|send.*code/i.test(bodyText)) {
      return JSON.stringify({status: "identity_verification_required", url});
    }
    if (url.includes("/iv/verify_mode.htm")) {
      return JSON.stringify({status: "identity_verification_required", url});
    }
    // Slide CAPTCHA appeared mid-flow
    if (document.querySelector("iframe[src*=nocaptcha], [id*=nc_], .nc-container")) {
      return JSON.stringify({status: "slide_captcha", url});
    }
    // Success: back on alibaba.com main with account menu visible
    const onAlibaba = hostname.endsWith("alibaba.com") && !hostname.startsWith("passport.") && !hostname.startsWith("login.");
    if (onAlibaba) {
      const accountMenu = document.querySelector("[class*=account-menu], [class*=my-alibaba], [class*=user-info]");
      const passportRedirectAbsent = !hostname.startsWith("passport.");
      if (accountMenu) {
        return JSON.stringify({status: "signed_in", url, verified_via: "account_menu_present"});
      }
      if (passportRedirectAbsent) {
        return JSON.stringify({status: "signed_in_weak", url, verified_via: "passport_redirect_absent"});
      }
    }
    await new Promise(r => setTimeout(r, 500));
  }
  return JSON.stringify({status: "timeout", url: window.location.href});
})()
'
```

Map the poll result:

| Poll status | `signin_status` | `signed_in_verified_via` | Blocker |
|---|---|---|---|
| `signed_in` | `signed_in` | from result | (none) |
| `signed_in_weak` | `signed_in` | `passport_redirect_absent` | warning `ALIBABA_SIGNED_IN_WEAK_SIGNAL` |
| `binding_required` | `manual_required` | `null` | `ALIBABA_ACCOUNT_BINDING_REQUIRED` |
| `region_prompt` | `manual_required` | `null` | `ALIBABA_REGION_PROMPT_PRESENT` |
| `identity_verification_required` | `manual_required` | `null` | `ALIBABA_IDENTITY_VERIFICATION_REQUIRED` |
| `slide_captcha` | `manual_required` | `null` | `ALIBABA_SLIDE_CAPTCHA_PRESENT` |
| `timeout` | `manual_required` | `null` | `ALIBABA_SIGNED_IN_SIGNAL_ABSENT` |

#### Step 8 — Leave the browser open

Do NOT close. Downstream skills (`marketplace-search/alibaba`, `landed-cost/shipment-sizing`, `source-health/alibaba`) all share this restored session.

### Phase A discovery (mandatory before first stable run)

1. Confirm `login.alibaba.com` directly serves the login form (vs. redirecting to a region-specific subdomain). If redirected, update Step 1's URL.
2. DevTools-inspect the "Continue with Google" button on the login form. Confirm Step 4's candidate list has at least one match returning exactly 1 element.
3. Trigger the slide CAPTCHA branch (open login.alibaba.com 5-10 times in quick succession). Confirm Step 3's eval detects it.
4. After a successful OAuth round-trip lands on alibaba.com, confirm Step 7's eval returns `status: "signed_in"` (not `signed_in_weak`). If only weak, capture the actual account-menu selector and update Step 7 + Step 2.
5. If you have access to a fresh Google account never bound to Alibaba, trigger the binding flow once and confirm Step 7 detects it. Otherwise document as untested.

Record discovery in `docs/testing/2026-05-17-cdp-auto-signin-test-report.md` under "Phase A discovery — Alibaba".

---

## Canary

Triggered via downstream Alibaba skill end-to-end tests: manually sign out of Alibaba in the persistent Chrome, run `landed-cost/shipment-sizing` against a fixture, observe this skill is invoked, observe the shipment-sizing skill completes successfully on the daemon's retry pass.

## Run Manifest Instructions

Required YAML frontmatter fields:
- manifest_version
- skill_id
- skill_version
- job_id
- session_id
- execution_mode
- started_at
- completed_at
- outcome_class
- items_processed
- items_skipped
- items_failed
- warnings_count
- agent_claimed_pages_visited
- agent_claimed_source_url
- agent_claimed_fingerprint_state
- agent_claimed_locale_state
- agent_claimed_currency_state

Required narrative sections:
- Summary
- Steps Taken
- Decisions Made
- Warnings
- Evidence

`agent_claimed_source_url` is the FINAL URL after the OAuth round-trip. In Decisions Made, name which Google-button selector matched in Step 4, which chooser row pattern matched in Step 6, and which signed-in signal verified success in Step 7. Call out explicitly whether the slide CAPTCHA was encountered (and if so, whether before or after Google delegation).
