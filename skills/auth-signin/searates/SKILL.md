---
skill_id: auth-signin/searates
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

# auth-signin/searates

## Use when
Use this skill when `landed-cost/searates-freight-estimate` has detected that `localStorage.s-token` is null/expired AND the SeaRates `/auth/platform-token?id=1` refresh returned 401 or no token, indicating the operator's SeaRates session is fully expired. The skill drives the standard "Sign in → Continue with Google → pick `dieter.x.s.danicek@gmail.com` → wait for s-token to repopulate" sequence using already-resident OAuth state.

SeaRates' s-token is a JWT issued post-OAuth and stored in `localStorage`. The skill considers signin successful when the token reappears in localStorage with a future expiry, regardless of the visible UI state.

## Do not use when
Do not invoke when:
- `localStorage.s-token` exists and decodes to a non-expired JWT. The caller's freshness check already handles this; invoking us is wasted overhead.
- The target SeaRates account is not the Google-OAuth-bound one (the operator's setup has SeaRates linked to `dieter.x.s.danicek@gmail.com`; other accounts are out of scope).
- A direct token-refresh path is available. The caller's first attempt SHOULD be `fetch("/auth/platform-token?id=1")` from inside the SeaRates page; only invoke us if that returns 401 / empty.

## Inputs
Daemon-injected `job_parameters`:
- `target_email` — Google account to pick. Default: `dieter.x.s.danicek@gmail.com`.

## Outputs
- `result.json` matching `result.schema.json`. Includes `s_token_reissued: bool` and `s_token_expires_at: iso8601_string | null` to give the caller direct confirmation the token is usable.

## Non-goals
- Booking shipments, contacting carriers, creating quotes, modifying account settings.
- Solving captchas (rare on SeaRates but documented for completeness).
- Switching the SeaRates account.

## Failure Codes
- `CDP_CONNECT_FAILED` — `bin/ensure-cdp-9222` failed or CDP preflight did not respond.
- `SEARATES_LOGIN_PAGE_UNREACHABLE` — opened `searates.com/login` but never landed on a recognisable login form.
- `SEARATES_GOOGLE_BUTTON_NOT_FOUND` — landed on login page but could not locate the "Continue with Google" button.
- `SEARATES_S_TOKEN_NOT_REISSUED` — completed the OAuth round-trip but `localStorage.s-token` is still null/empty after the wait window. Operator inspects manually.
- `GOOGLE_INTERACTIVE_CHALLENGE` / `GOOGLE_TARGET_ACCOUNT_NOT_LISTED` / `GOOGLE_CHOOSER_ABSENT` / `GOOGLE_REDIRECT_TIMEOUT` — propagated from the chooser sub-step.

## Resume Strategy
Not resumable. Atomic.

## Pagination Bounds
- `maxPages`: 3 (SeaRates login page → Google chooser → SeaRates redirect)
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

#### Step 1 — Open SeaRates login

```bash
agent-browser --cdp ${CDP_PORT} open "https://www.searates.com/login"
agent-browser --cdp ${CDP_PORT} wait 2500
```

If SeaRates' login URL has changed (Phase A discovery confirms today's path), update this step.

#### Step 2 — Verify s-token is actually missing

The caller has reported the token is missing, but verify before driving the UI (avoids racing with a concurrent refresh from another tab):

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const token = localStorage.getItem("s-token");
  if (!token) return JSON.stringify({token_present: false});
  // Decode JWT exp claim
  try {
    const parts = token.split(".");
    if (parts.length === 3) {
      const payload = JSON.parse(atob(parts[1].replace(/-/g, "+").replace(/_/g, "/")));
      const expMs = (payload.exp || 0) * 1000;
      return JSON.stringify({token_present: true, exp_ms: expMs, expired: expMs < Date.now()});
    }
  } catch (_) { /* fall through */ }
  return JSON.stringify({token_present: true, exp_ms: null, expired: null});
})()
'
```

Decision:
- `token_present: true` AND (`expired: false` OR `expired: null`) → operator is already signed in / token already refreshed; write `signin_status: "signed_in"`, `s_token_reissued: false`, populate `s_token_expires_at` from `exp_ms`, stop with success.
- Otherwise: proceed to Step 3.

#### Step 3 — Click "Continue with Google" on SeaRates' login page

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const candidates = [
    document.querySelector("a[href*=\"accounts.google.com/o/oauth2\"]"),
    document.querySelector("button[aria-label*=Google i]"),
    document.querySelector("[data-provider=google], [data-social=google]"),
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

If `clicked: false`, write blocked with `SEARATES_GOOGLE_BUTTON_NOT_FOUND`.

#### Step 4 — Confirm we landed on Google's account chooser

```bash
agent-browser --cdp ${CDP_PORT} eval 'JSON.stringify({on_google: window.location.hostname.endsWith("accounts.google.com"), url: window.location.href})'
```

If `on_google: false`, write blocked with `SEARATES_GOOGLE_BUTTON_NOT_FOUND` + url in message.

#### Step 5 — Delegate to Google account chooser logic

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

#### Step 6 — Poll for redirect back to searates.com AND verify s-token reissued

SeaRates' visible UI signal is weak (the avatar appears in the header after login but the selector varies by page). The reliable signal is `localStorage.s-token` repopulating. Poll both:

```bash
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const deadline = Date.now() + 18000;
  while (Date.now() < deadline) {
    const url = window.location.href;
    const hostname = window.location.hostname;
    // Check for interstitial
    if (document.querySelector("iframe[src*=recaptcha]")) {
      return JSON.stringify({status: "interstitial_captcha", url});
    }
    // On searates.com? Check the token
    if (hostname.endsWith("searates.com")) {
      const token = localStorage.getItem("s-token");
      if (token) {
        try {
          const parts = token.split(".");
          if (parts.length === 3) {
            const payload = JSON.parse(atob(parts[1].replace(/-/g, "+").replace(/_/g, "/")));
            const expMs = (payload.exp || 0) * 1000;
            if (expMs > Date.now()) {
              return JSON.stringify({status: "signed_in", url, exp_ms: expMs, token_length: token.length});
            }
          }
        } catch (_) {
          // Token exists but is non-JWT format — treat as success with unknown expiry
          return JSON.stringify({status: "signed_in", url, exp_ms: null, token_length: token.length});
        }
      }
    }
    await new Promise(r => setTimeout(r, 500));
  }
  return JSON.stringify({status: "timeout", url: window.location.href});
})()
'
```

Map the poll result:

| Poll status | `signin_status` | Blocker | `s_token_reissued` |
|---|---|---|---|
| `signed_in` | `signed_in` | (none) | `true`, `s_token_expires_at` from `exp_ms` |
| `interstitial_captcha` | `manual_required` | `GOOGLE_INTERACTIVE_CHALLENGE` | `false` |
| `timeout` | `manual_required` | `SEARATES_S_TOKEN_NOT_REISSUED` | `false` |

#### Step 7 — Leave the browser open

Do NOT close. `landed-cost/searates-freight-estimate` will be re-invoked by the daemon's retry policy; the new s-token persists in localStorage.

### Phase A discovery (mandatory before first stable run)

1. Confirm `https://www.searates.com/login` is today's login URL.
2. With a logged-OUT SeaRates session (clear `localStorage.s-token`), DevTools-inspect the social login section. Update Step 3's candidate list if needed.
3. Complete one full OAuth round-trip manually. After landing back on searates.com, confirm `localStorage.s-token` is a valid 3-part JWT and decodes with an `exp` claim. If not (SeaRates moved to opaque tokens), adapt Step 2/6 to fall back to "any non-empty s-token after redirect".
4. Document the s-token TTL observed (typically days-weeks for SeaRates) — this informs how often this skill will get invoked.

Record discovery in `docs/testing/2026-05-17-cdp-auto-signin-test-report.md` under "Phase A discovery — SeaRates".

---

## Canary

Triggered as part of `landed-cost/searates-freight-estimate` end-to-end tests: clear `localStorage.s-token` in the persistent Chrome, run the SeaRates skill against a fixture route, observe this skill is invoked + token reissued + SeaRates skill completes on retry.

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

`agent_claimed_source_url` is the FINAL URL after the OAuth round-trip. In Decisions Made, name which Google-button selector matched in Step 3, the chooser row pattern in Step 5, and whether the s-token decoded as a JWT with an `exp` claim or fell back to opaque-token success.
