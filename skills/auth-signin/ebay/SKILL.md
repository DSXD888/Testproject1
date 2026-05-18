---
skill_id: auth-signin/ebay
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

# auth-signin/ebay

## Use when
Use this skill when a downstream eBay UK skill (currently only `marketplace-search/ebay-uk`, though more may follow) has detected that the operator's persistent Chrome profile is signed OUT of `ebay.co.uk` and needs to be signed back in before its main operation can complete.

The skill walks the standard "Sign in → Continue with Google → pick `dieter.x.s.danicek@gmail.com` → verify avatar present" sequence using already-resident OAuth state. It does NOT type a password and does NOT solve captchas — those are surfaced as `manual_required` for the operator to handle.

## Do not use when
Do not invoke when:
- The browser is already signed into eBay UK. The skill no-ops in this case (returns `signin_status: "signed_in"` immediately), but the round-trip is wasted — callers should only invoke after their own logged-out detection.
- The target site is `ebay.com`, `ebay.de`, or another locale. eBay's signin redirect domains differ per TLD; this skill is hardcoded for `ebay.co.uk` / `signin.ebay.co.uk`.
- A non-Google credential is needed. The operator's setup uses `dieter.x.s.danicek@gmail.com` via Google OAuth on eBay; if eBay refuses Google for this account (rare — typically only on suspicious-login lockouts), the skill returns `manual_required`.

## Inputs
Daemon-injected `job_parameters`:
- `target_email` — Google account to pick. Default: `dieter.x.s.danicek@gmail.com`.

## Outputs
- `result.json` matching `result.schema.json`. Same shape as `auth-signin/google-account-chooser` plus an eBay-specific `signed_in_verified_via` field naming which DOM signal the skill checked to confirm signin (`signin_link_absent`, `greeting_text_present`, or `my_ebay_link_present`).

## Non-goals
- Bidding, watching, messaging sellers, modifying account settings, placing orders.
- Switching the eBay account.
- Closing the operator browser. Leave the final eBay page visible.
- Solving captchas, 2FA prompts, "Verify your account" interstitials.

## Failure Codes
- `CDP_CONNECT_FAILED` — `bin/ensure-cdp-9222` failed or `localhost:${CDP_PORT}/json/version` did not respond.
- `EBAY_SIGNIN_BUTTON_NOT_FOUND` — landed on the eBay UK homepage but could not locate the "Sign in" link. Likely page-shape drift.
- `EBAY_GOOGLE_BUTTON_NOT_FOUND` — landed on the eBay signin page but could not locate the "Continue with Google" button. Either eBay removed Google OAuth or page-shape drift.
- `EBAY_SOCIAL_REDIRECT_TIMEOUT` — clicked Continue with Google but the browser never navigated to `accounts.google.com`.
- `EBAY_POST_OAUTH_VERIFICATION_INTERSTITIAL` — eBay served a captcha or "Verify your account" page after Google returned. Operator action required.
- `EBAY_SIGNED_IN_SIGNAL_ABSENT` — completed the OAuth round-trip but cannot confirm signed-in DOM state on `ebay.co.uk`. Operator inspects manually.
- `GOOGLE_INTERACTIVE_CHALLENGE` / `GOOGLE_TARGET_ACCOUNT_NOT_LISTED` / `GOOGLE_CHOOSER_ABSENT` / `GOOGLE_REDIRECT_TIMEOUT` — propagated from the chooser sub-step.

## Resume Strategy
Not resumable. Atomic: if any step fails, the caller re-invokes from scratch after the operator clears the obstacle.

## Pagination Bounds
- `maxPages`: 3 (eBay homepage → signin page → Google chooser → eBay redirect)
- `maxScrollCycles`: 1

---

## Playbook

You are an eBay UK auto-signin agent. The eBay-side click sequence is short — most of the work is detecting the right page shapes and delegating the Google account picker to the shared chooser logic.

### Mandatory CDP Discipline

This skill MUST attach to the operator's persistent Chrome. Do not launch a fresh browser, do not solve captchas, do not type credentials, do not close the operator browser.

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

# Defensive reachability check. If this fails (helper exited non-zero, or a
# rare post-launch race), write the CDP_CONNECT_FAILED result and stop.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, write the blocked result with blocker code `CDP_CONNECT_FAILED` and stop. Every browser command MUST use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

### Workflow

#### Step 1 — Open eBay UK homepage

```bash
agent-browser --cdp ${CDP_PORT} open "https://www.ebay.co.uk/"
agent-browser --cdp ${CDP_PORT} wait 2500
```

#### Step 2 — Verify NOT already signed in

eBay's header shape changes based on signin state. Detect logged-out via a single eval:

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  // Logged-out signal: top-right shows "Sign in or register" link to signin.ebay.co.uk
  const signinLink = document.querySelector("a[href*=\"signin.ebay.co.uk\"]");
  // Logged-in signals: greeting text containing "Hi <name>" OR a My eBay nav link
  const greetingPresent = /\bHi |\bHello /.test((document.body && document.body.innerText) || "");
  const myEbayLink = document.querySelector("a[href*=\"/myb/\"], a[href*=\"my.ebay.co.uk\"]");
  return JSON.stringify({
    signin_link_present: !!signinLink,
    greeting_present: greetingPresent,
    my_ebay_link_present: !!myEbayLink,
    href: signinLink ? signinLink.getAttribute("href") : null
  });
})()
'
```

Decision:
- If `signin_link_present` is `false` AND (`greeting_present` is `true` OR `my_ebay_link_present` is `true`): already signed in. Write `signin_status: "signed_in"`, `signed_in_verified_via: "<whichever logged-in signal matched>"`, and stop with success.
- Otherwise: proceed to Step 3.

#### Step 3 — Click the Sign In link

The selector to use is the `href` returned from Step 2's eval (it's the canonical link with current campaign tracking params; selecting it by exact href avoids tracking-param drift):

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const a = document.querySelector("a[href*=\"signin.ebay.co.uk\"]");
  if (!a) return JSON.stringify({clicked: false, reason: "signin_link_disappeared"});
  a.click();
  return JSON.stringify({clicked: true});
})()
'
agent-browser --cdp ${CDP_PORT} wait 2500
```

If `clicked: false`, write the blocked result with `EBAY_SIGNIN_BUTTON_NOT_FOUND`.

#### Step 4 — Click "Continue with Google" on the signin page

eBay's signin page has a social-login section. The Google button's selector is not perfectly stable, so probe several candidates and pick the first match:

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const candidates = [
    document.querySelector("button[aria-label*=Google i]"),
    document.querySelector("[data-social=google]"),
    document.querySelector("button[data-test-id*=google i]"),
    // Generic text-based fallback — last because it is the weakest signal
    ...Array.from(document.querySelectorAll("button, a")).filter(el => /continue with google/i.test(el.innerText || ""))
  ].filter(Boolean);
  const btn = candidates[0];
  if (!btn) return JSON.stringify({clicked: false, reason: "no_google_button_found", current_url: window.location.href});
  btn.click();
  return JSON.stringify({clicked: true, selector_used: btn.outerHTML.slice(0, 120)});
})()
'
agent-browser --cdp ${CDP_PORT} wait 3000
```

If `clicked: false`, write blocked result with `EBAY_GOOGLE_BUTTON_NOT_FOUND` and include `current_url` in the blocker message.

#### Step 5 — Confirm we landed on Google's account chooser

```bash
agent-browser --cdp ${CDP_PORT} eval 'JSON.stringify({on_google: window.location.hostname.endsWith("accounts.google.com"), url: window.location.href})'
```

If `on_google` is `false`: the Continue with Google button didn't redirect. Write blocked result with `EBAY_SOCIAL_REDIRECT_TIMEOUT`.

#### Step 6 — Delegate to the Google account chooser logic

The shared `auth-signin/google-account-chooser` SKILL.md documents the canonical chooser interaction. Inline the chooser eval here (rather than spawning the chooser skill as a subprocess — auth-signin skills are short and inline composition keeps the attempt budget tight):

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const target = "dieter.x.s.danicek@gmail.com";
  const onAccountsGoogle = window.location.hostname.endsWith("accounts.google.com");
  if (!onAccountsGoogle) return JSON.stringify({page_shape: "not_on_google", current_url: window.location.href});
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

Map the chooser result to the eBay result fields:
- `page_shape: "captcha"` or `"twofa_interstitial"` → `signin_status: "manual_required"`, blocker `GOOGLE_INTERACTIVE_CHALLENGE`.
- `page_shape: "email_entry_form"` → `signin_status: "blocked"`, blocker `GOOGLE_CHOOSER_ABSENT`.
- `page_shape: "chooser_target_missing"` → `signin_status: "blocked"`, blocker `GOOGLE_TARGET_ACCOUNT_NOT_LISTED`.
- `page_shape: "chooser_target_found"` → proceed to Step 7.

#### Step 7 — Poll for redirect back to ebay.co.uk and verify signed-in

After picking the chooser row, the browser redirects through `signin.ebay.co.uk/socialsignin/...` → `ebay.co.uk/...`. Poll for the eBay header signal:

```bash
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const deadline = Date.now() + 15000;
  while (Date.now() < deadline) {
    const url = window.location.href;
    // Detect interstitial that may have appeared between Google + eBay
    if (document.querySelector("iframe[src*=recaptcha], iframe[title*=captcha i]")) {
      return JSON.stringify({status: "interstitial_captcha", url});
    }
    if (/Verify your account|Please confirm your identity|unusual activity/i.test((document.body && document.body.innerText) || "")) {
      return JSON.stringify({status: "interstitial_verification", url});
    }
    // Success: back on ebay.co.uk and signin link is GONE
    const onEbay = window.location.hostname.endsWith("ebay.co.uk") && !window.location.hostname.startsWith("signin.");
    if (onEbay) {
      const signinLink = document.querySelector("a[href*=\"signin.ebay.co.uk\"]");
      const greetingPresent = /\bHi |\bHello /.test((document.body && document.body.innerText) || "");
      const myEbayLink = document.querySelector("a[href*=\"/myb/\"], a[href*=\"my.ebay.co.uk\"]");
      if (!signinLink && (greetingPresent || myEbayLink)) {
        return JSON.stringify({
          status: "signed_in",
          url,
          verified_via: greetingPresent ? "greeting_text_present" : "my_ebay_link_present"
        });
      }
      if (!signinLink) {
        return JSON.stringify({status: "signed_in_weak", url, verified_via: "signin_link_absent"});
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
| `signed_in_weak` | `signed_in` | `signin_link_absent` | warning `EBAY_SIGNED_IN_WEAK_SIGNAL` |
| `interstitial_captcha` | `manual_required` | `null` | `EBAY_POST_OAUTH_VERIFICATION_INTERSTITIAL` |
| `interstitial_verification` | `manual_required` | `null` | `EBAY_POST_OAUTH_VERIFICATION_INTERSTITIAL` |
| `timeout` | `manual_required` | `null` | `EBAY_SIGNED_IN_SIGNAL_ABSENT` |

#### Step 8 — Leave the browser open

Do NOT close the tab. The downstream `marketplace-search/ebay-uk` skill (or whichever caller triggered the recovery) needs the restored session.

### Phase A discovery (mandatory before first stable run)

Selectors in Steps 3-7 are placeholders informed by the operator's verbal description (2026-05-17). Before this skill is considered production-stable:

1. With Chrome on port 9222 and a logged-OUT eBay UK tab open, run DevTools and confirm `document.querySelector('a[href*="signin.ebay.co.uk"]')` returns the right element.
2. Click the link manually. On the signin page, DevTools-inspect the "Continue with Google" button. Update Step 4's selector list if `button[aria-label*=Google i]` doesn't return exactly 1 element.
3. After the full OAuth round-trip lands you back on ebay.co.uk signed in, confirm the eval in Step 7 returns `status: "signed_in"` (not `signed_in_weak`). If only weak, capture the actual greeting/avatar selector and update Step 2 + Step 7.
4. Test the captcha branch: trigger eBay's "Verify your account" interstitial (rare — typically only on new-device first-login). If you cannot trigger it naturally, document that the interstitial branch is untested and operator-watched only.

Record discovery results in `docs/testing/2026-05-17-cdp-auto-signin-test-report.md` under "Phase A discovery — eBay UK".

---

## Canary

Triggered as part of the `marketplace-search/ebay-uk` end-to-end test path: manually sign out of eBay in the persistent Chrome, run `marketplace-search/ebay-uk` against a known-good fixture, observe that this skill is invoked and that the eBay skill completes successfully on the daemon's retry pass.

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

`agent_claimed_source_url` is the FINAL URL after the OAuth round-trip (the `url` field in Step 7's eval). In Decisions Made, name which Google-button selector matched in Step 4, which chooser row pattern matched in Step 6, and which signed-in signal verified success in Step 7.
