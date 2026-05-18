---
skill_id: auth-signin/freightos
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

# auth-signin/freightos

## Use when
Use this skill when the in-progress `landed-cost/freightos-freight-estimate` skill (see `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/HANDOFF-freightos-freight-estimate.md`) has detected that the operator's persistent Chrome profile is signed OUT of `ship.freightos.com` (search API POST returned 401, or DOM redirected to login modal, or the operator's "D" avatar is absent from the top-right of `ship.freightos.com`).

The skill walks the standard "Sign in → Continue with Google → pick `dieter.x.s.danicek@gmail.com` → verify D avatar present" sequence. The Freightos account in the operator's setup is referred to as account "D" — this skill expects the same Google email to be bound to that Freightos account.

## Do not use when
Do not invoke when:
- The operator's "D" avatar is visible in `ship.freightos.com`. The Freightos session is fine; the caller's logged-out detection was a false positive.
- The Freightos in-progress skill hasn't been built yet. This skill is forward-compatible with that handoff — invoking it before the caller exists doesn't break anything (it just runs in isolation), but there's no value.
- Freightos's UI moves auth out of `ship.freightos.com` (e.g. to `app.freightos.com`). Re-do Phase A discovery before relying on this skill.

## Inputs
Daemon-injected `job_parameters`:
- `target_email` — Google account to pick. Default: `dieter.x.s.danicek@gmail.com`.

## Outputs
- `result.json` matching `result.schema.json`. Includes `signed_in_verified_via` naming which DOM signal confirmed signin (`d_avatar_present`, `refer_a_friend_present`, or `login_modal_absent`).

## Non-goals
- Booking shipments, contacting forwarders, creating shipment plans.
- Modifying account settings, changing organization.
- Solving captchas (Freightos uses Cloudflare and occasionally serves a challenge for fresh OAuth).
- Switching the Freightos account.

## Failure Codes
- `CDP_CONNECT_FAILED` — `bin/ensure-cdp-9222` failed or CDP preflight did not respond.
- `FREIGHTOS_LOGIN_PAGE_UNREACHABLE` — opened `ship.freightos.com/login` but never landed on a recognisable login form.
- `FREIGHTOS_GOOGLE_BUTTON_NOT_FOUND` — landed on login page but no "Continue with Google" button. Likely page-shape drift or Freightos removed Google OAuth.
- `FREIGHTOS_CLOUDFLARE_CHALLENGE` — Cloudflare interstitial blocking access to login. Operator action required.
- `FREIGHTOS_SIGNED_IN_SIGNAL_ABSENT` — completed OAuth round-trip but cannot confirm signed-in DOM state.
- `GOOGLE_INTERACTIVE_CHALLENGE` / `GOOGLE_TARGET_ACCOUNT_NOT_LISTED` / `GOOGLE_CHOOSER_ABSENT` / `GOOGLE_REDIRECT_TIMEOUT` — propagated.

## Resume Strategy
Not resumable. Atomic.

## Pagination Bounds
- `maxPages`: 3
- `maxScrollCycles`: 1

---

## Playbook

### Mandatory CDP Discipline

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

# Defensive reachability check. If this fails, write CDP_CONNECT_FAILED and stop.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, write blocked result with `CDP_CONNECT_FAILED` and stop. Every browser command MUST use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

### Workflow

#### Step 1 — Open Freightos login

The login URL is unverified at handoff time. Phase A discovery confirms whether it's `/login`, `/auth/signin`, `/account/login`, or similar. Default candidate:

```bash
agent-browser --cdp ${CDP_PORT} open "https://ship.freightos.com/login"
agent-browser --cdp ${CDP_PORT} wait 2500
```

If Freightos redirects to its homepage (`ship.freightos.com/` without `/login`), open the homepage and click the "Sign in" link instead:

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const signinLink = document.querySelector("a[href*=login], a[href*=signin], a[href*=sign-in]");
  if (signinLink) { signinLink.click(); return JSON.stringify({clicked: true}); }
  return JSON.stringify({clicked: false});
})()
'
agent-browser --cdp ${CDP_PORT} wait 2000
```

#### Step 2 — Check for Cloudflare challenge

Freightos uses Cloudflare. Detect challenge interstitial early:

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const cf = document.querySelector("[id^=cf-], #challenge-form, .cf-browser-verification");
  const text = (document.body && document.body.innerText) || "";
  return JSON.stringify({cloudflare: !!cf || /Just a moment|Verifying you are human/.test(text)});
})()
'
```

If `cloudflare: true`, write blocked with `FREIGHTOS_CLOUDFLARE_CHALLENGE` and stop.

#### Step 3 — Verify NOT already signed in

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  // Logged-in signals: "D" avatar (top-right circular avatar with letter D), or "Refer a friend" link
  // visible only to logged-in operators
  const dAvatar = Array.from(document.querySelectorAll("[class*=avatar], [class*=initials], [class*=user-menu]"))
    .find(el => /^D$/.test((el.innerText || "").trim()));
  const referFriend = Array.from(document.querySelectorAll("a, button"))
    .find(el => /refer a friend|invite.*friend/i.test(el.innerText || ""));
  return JSON.stringify({
    already_signed_in: !!(dAvatar || referFriend),
    signal: dAvatar ? "d_avatar_present" : (referFriend ? "refer_a_friend_present" : null)
  });
})()
'
```

If `already_signed_in: true`, write `signin_status: "signed_in"`, `signed_in_verified_via: <signal>`, stop.

#### Step 4 — Click "Continue with Google"

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const candidates = [
    document.querySelector("a[href*=\"accounts.google.com/o/oauth2\"]"),
    document.querySelector("button[aria-label*=Google i]"),
    document.querySelector("[data-provider=google], [data-test-id*=google i]"),
    ...Array.from(document.querySelectorAll("button, a, [role=button]")).filter(el => /continue with google|sign in with google/i.test(el.innerText || ""))
  ].filter(Boolean);
  const btn = candidates[0];
  if (!btn) return JSON.stringify({clicked: false, reason: "no_google_button_found", current_url: window.location.href});
  btn.click();
  return JSON.stringify({clicked: true});
})()
'
agent-browser --cdp ${CDP_PORT} wait 3500
```

If `clicked: false`, write blocked with `FREIGHTOS_GOOGLE_BUTTON_NOT_FOUND`.

#### Step 5 — Confirm we landed on Google's chooser

```bash
agent-browser --cdp ${CDP_PORT} eval 'JSON.stringify({on_google: window.location.hostname.endsWith("accounts.google.com"), url: window.location.href})'
```

If `on_google: false`, write blocked with `FREIGHTOS_GOOGLE_BUTTON_NOT_FOUND` + url.

#### Step 6 — Delegate to Google account chooser logic

(See `skills/auth-signin/google-account-chooser/SKILL.md` for the canonical implementation. Inlined here for the same reasons as `skills/auth-signin/ebay/SKILL.md` Step 6.)

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

Map chooser results the same way as in `auth-signin/ebay/SKILL.md` Step 6.

#### Step 7 — Poll for redirect back to ship.freightos.com and verify signed-in

```bash
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const deadline = Date.now() + 18000;
  while (Date.now() < deadline) {
    const url = window.location.href;
    const hostname = window.location.hostname;
    // Cloudflare interstitial that may have appeared during redirect
    if (document.querySelector("[id^=cf-], #challenge-form, .cf-browser-verification")) {
      return JSON.stringify({status: "cloudflare", url});
    }
    if (hostname.endsWith("freightos.com")) {
      const dAvatar = Array.from(document.querySelectorAll("[class*=avatar], [class*=initials], [class*=user-menu]"))
        .find(el => /^D$/.test((el.innerText || "").trim()));
      const referFriend = Array.from(document.querySelectorAll("a, button"))
        .find(el => /refer a friend|invite.*friend/i.test(el.innerText || ""));
      const loginModal = document.querySelector("form[action*=login], [class*=login-modal]");
      if (dAvatar) return JSON.stringify({status: "signed_in", url, verified_via: "d_avatar_present"});
      if (referFriend) return JSON.stringify({status: "signed_in", url, verified_via: "refer_a_friend_present"});
      if (!loginModal && hostname === "ship.freightos.com") {
        // Weak signal: on the right host, no login modal visible
        return JSON.stringify({status: "signed_in_weak", url, verified_via: "login_modal_absent"});
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
| `signed_in_weak` | `signed_in` | `login_modal_absent` | warning `FREIGHTOS_SIGNED_IN_WEAK_SIGNAL` |
| `cloudflare` | `manual_required` | `null` | `FREIGHTOS_CLOUDFLARE_CHALLENGE` |
| `timeout` | `manual_required` | `null` | `FREIGHTOS_SIGNED_IN_SIGNAL_ABSENT` |

#### Step 8 — Leave the browser open

Do NOT close. `landed-cost/freightos-freight-estimate` (once built) will be re-invoked by the daemon's retry policy; the session persists.

### Phase A discovery (mandatory before first stable run)

1. Confirm `ship.freightos.com/login` is today's login URL. If it redirects, capture the actual login URL and update Step 1.
2. Confirm the "D" avatar selector. Inspect with DevTools when logged in; if it's not literally a circular element with "D" text, capture the right selector (e.g. `img[alt*="account D" i]`).
3. Confirm "Refer a friend" link/button selector (per the operator's screenshot reference `screenshots/freightos/freightos-step-01.png`).
4. Trigger Cloudflare challenge (open `ship.freightos.com` 10-15 times in rapid succession from a fresh-IP perspective). Confirm Step 2 detects it.
5. Complete one full OAuth round-trip and confirm Step 7 returns `status: "signed_in"`. If only `signed_in_weak`, refine the avatar/refer-a-friend selectors.

Record discovery in `docs/testing/2026-05-17-cdp-auto-signin-test-report.md` under "Phase A discovery — Freightos".

---

## Canary

Triggered as part of `landed-cost/freightos-freight-estimate` end-to-end tests (once that skill exists per the L5 operator-built handoff): manually sign out of Freightos in the persistent Chrome, run the Freightos skill against a fixture, observe this skill is invoked + Freightos skill completes successfully on retry.

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

`agent_claimed_source_url` is the FINAL URL after the OAuth round-trip. In Decisions Made, name the Google-button selector matched in Step 4, the chooser row pattern in Step 6, which signed-in signal verified success in Step 7, and whether Cloudflare interstitial was encountered.
