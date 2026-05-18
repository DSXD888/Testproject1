---
skill_id: auth-signin/google-account-chooser
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: local_state
requires_cdp: true
cdp_default_port: 9222
maxPages: 2
maxScrollCycles: 1
---

# auth-signin/google-account-chooser

## Use when
Use this skill when the current Chrome tab has just navigated to Google's OAuth account chooser (`accounts.google.com/o/oauth2/...`, `accounts.google.com/AccountChooser?...`, or `accounts.google.com/signin/oauth/...`) and the caller needs to select `dieter.x.s.danicek@gmail.com` to complete the OAuth handshake back to the originating site.

It is the SHARED sub-skill that every site-specific Google-OAuth `auth-signin/*` skill delegates to. The per-site skills (`auth-signin/{ebay,alibaba,searates,freightos}`) handle the "click Sign In on the site" + "click Continue with Google" half; this skill handles the Google account-picker half + waiting for the redirect back to the parent origin.

## Do not use when
Do not invoke this skill when:
- The browser is NOT on `accounts.google.com` (no chooser to interact with). The caller's job to verify before delegating.
- The caller needs to sign into Google itself for the first time. This skill PICKS from already-signed-in Google accounts; it does NOT enter a Google password. If the chooser shows an email-entry form, the skill returns `blocked_chooser_absent` immediately.
- A non-OAuth signin is needed (e.g. ProContract's password-manager autofill flow uses `auth-signin/procontract` instead).

## Inputs
The skill reads these from `job_parameters` (daemon-injected) or sensible defaults:

- `target_email` — the Google account to select. Default: `dieter.x.s.danicek@gmail.com`. Overridable for future multi-account scenarios.
- `parent_window_origin` — the origin we expect to be redirected to after picking the account. E.g. `https://www.ebay.co.uk`. The skill polls `window.location.origin` and considers the redirect successful when the polled origin matches (substring match — Google sometimes routes via `signin.ebay.co.uk/socialsignin/...` before landing on `ebay.co.uk`).
- `redirect_timeout_ms` — how long to wait for the redirect after clicking the chooser row. Default: `15000` (15s). Cap: `30000`.

## Outputs
- `result.json` in the per-skill workdir, matching `result.schema.json`. The schema is deliberately small (6 fields) — this skill is a single decision point, not a data extractor.

## Non-goals
- Signing into Google with a fresh email + password. Out of scope; would require typing real credentials.
- Solving captchas, 2FA prompts, or "Verify it's you" interstitials. Those are surfaced as `manual_required` and the operator handles them in the persistent Chrome.
- Switching the default Chrome profile or modifying `chrome://chrome-signin` state.
- Closing the operator browser. Leave the final URL visible for the next skill in the chain.

## Failure Codes
- `CDP_CONNECT_FAILED` — `bin/ensure-cdp-9222` exited non-zero OR `localhost:${CDP_PORT}/json/version` did not respond. Skill cannot continue.
- `GOOGLE_CHOOSER_ABSENT` — the page is not on `accounts.google.com` at all, or it's showing the email-entry form (no account currently signed into Chrome's Google session). Operator action required.
- `GOOGLE_TARGET_ACCOUNT_NOT_LISTED` — chooser is present but no row matches `target_email`. The operator's Google has at least one account signed in but NOT `dieter.x.s.danicek@gmail.com`.
- `GOOGLE_INTERACTIVE_CHALLENGE` — captcha, 2FA, or "Verify it's you" interstitial is blocking progress. Operator action required.
- `GOOGLE_REDIRECT_TIMEOUT` — clicked the chooser row, no error visible, but the browser did not redirect back to `parent_window_origin` within the timeout. Caller should treat as `manual_required` and inspect Chrome.
- `AGENT_RUN_FAILED` — generic catch-all for unexpected errors.

## Resume Strategy
Not resumable. The skill is atomic (single click + poll); if it fails, the caller re-invokes from scratch after the operator clears the underlying obstacle.

## Pagination Bounds
- `maxPages`: 2 (chooser page → consent page → redirect target — usually 1 page each, but the consent step is occasionally shown for first-OAuth-with-this-app)
- `maxScrollCycles`: 1

---

## Playbook

You are a Google OAuth account picker. The page is already on `accounts.google.com`. Your job is to identify which of Google's possible page shapes you are on, click the right row, and confirm the browser redirected back to the calling site.

### Mandatory CDP Discipline

This skill MUST attach to the operator's already-running persistent Chrome via CDP. Do not launch a fresh browser, do not solve challenges programmatically, do not log out of any account, do not close the operator browser.

Before any browser command:

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

If CDP is unreachable, write this `result.json` immediately:

```json
{
  "signin_status": "blocked",
  "account_selected": null,
  "redirected_to": null,
  "cdp_state": "unreachable",
  "captcha_intercepted": false,
  "warnings": [],
  "blockers": [{"code": "CDP_CONNECT_FAILED", "message": "bin/ensure-cdp-9222 failed; cannot drive Google chooser", "severity": "blocked"}]
}
```

Every browser command MUST use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

### Workflow

The skill is a single triage + click + poll. Do NOT navigate the chooser page yourself — the CALLER (a per-site auth-signin skill) is responsible for landing the browser there. If the current URL is not on `accounts.google.com`, return `GOOGLE_CHOOSER_ABSENT` rather than navigating yourself.

#### Step 1 — Classify the current page in ONE eval call

The Google chooser, the email-entry form, captcha, and 2FA interstitials are mutually exclusive page shapes. One eval call distinguishes them. Inspect the result in priority order:

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const target = "dieter.x.s.danicek@gmail.com";
  const onAccountsGoogle = window.location.hostname.endsWith("accounts.google.com");
  if (!onAccountsGoogle) {
    return JSON.stringify({page_shape: "not_on_google", current_url: window.location.href});
  }
  // Priority 1: captcha (any reCAPTCHA iframe present)
  if (document.querySelector("iframe[src*=recaptcha]")) {
    return JSON.stringify({page_shape: "captcha", current_url: window.location.href});
  }
  // Priority 2: 2FA / "Verify it is you" interstitial
  const bodyText = (document.body && document.body.innerText) || "";
  if (/Verify it.s you|2-Step Verification|Confirm your identity/.test(bodyText)) {
    return JSON.stringify({page_shape: "twofa_interstitial", current_url: window.location.href});
  }
  // Priority 3: email-entry form (means NO Google account is signed in to Chrome at all)
  if (document.querySelector("input[type=email][name=identifier], input#identifierId")) {
    return JSON.stringify({page_shape: "email_entry_form", current_url: window.location.href});
  }
  // Priority 4: account chooser. Try several selector patterns Google has used.
  //   (a) Modern: [data-identifier="<email>"] rows
  //   (b) Older:  [data-email="<email>"] rows
  //   (c) Generic: any li/div with role=link whose text contains the email substring
  const targetByIdentifier = document.querySelector(`[data-identifier="${target}"]`);
  const targetByEmail = document.querySelector(`[data-email="${target}"]`);
  let targetByText = null;
  for (const el of document.querySelectorAll("li[role=link], div[role=link], div[role=button]")) {
    const t = (el.innerText || "").toLowerCase();
    if (t.includes(target.toLowerCase())) { targetByText = el; break; }
  }
  const targetFound = targetByIdentifier || targetByEmail || targetByText;
  // Are ANY chooser rows present (even non-matching)?
  const anyChooserRows = document.querySelectorAll("[data-identifier], [data-email], li[role=link]").length;
  if (targetFound) {
    return JSON.stringify({page_shape: "chooser_target_found", current_url: window.location.href});
  }
  if (anyChooserRows > 0) {
    return JSON.stringify({page_shape: "chooser_target_missing", current_url: window.location.href, rows_present: anyChooserRows});
  }
  return JSON.stringify({page_shape: "unknown", current_url: window.location.href});
})()
'
```

Branch on the `page_shape` field of the JSON returned:

| `page_shape` | Next action |
|---|---|
| `not_on_google` | Write result `signin_status: "blocked"`, `cdp_state: "connected"`, blocker `GOOGLE_CHOOSER_ABSENT`. The caller should not have invoked us. |
| `captcha` | Write result `signin_status: "manual_required"`, blocker `GOOGLE_INTERACTIVE_CHALLENGE`. Operator clears captcha manually. |
| `twofa_interstitial` | Write result `signin_status: "manual_required"`, blocker `GOOGLE_INTERACTIVE_CHALLENGE`. Operator approves 2FA on their phone. |
| `email_entry_form` | Write result `signin_status: "blocked"`, blocker `GOOGLE_CHOOSER_ABSENT`. Operator signs into Google in `chrome://chrome-signin` first. |
| `chooser_target_missing` | Write result `signin_status: "blocked"`, blocker `GOOGLE_TARGET_ACCOUNT_NOT_LISTED`. Operator adds the target account to their Chrome Google profile. |
| `chooser_target_found` | Continue to Step 2. |
| `unknown` | Write result `signin_status: "blocked"`, blocker `AGENT_RUN_FAILED` with a message including the current_url. Surfaces for diagnosis. |

#### Step 2 — Click the target chooser row, then poll for the redirect

Click via a JS dispatch rather than `agent-browser click "@..."` — the chooser rows often have non-stable accessibility refs, and the JS click runs the same handler Google's UI runs:

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const target = "dieter.x.s.danicek@gmail.com";
  const row = document.querySelector(`[data-identifier="${target}"]`)
    || document.querySelector(`[data-email="${target}"]`)
    || Array.from(document.querySelectorAll("li[role=link], div[role=link], div[role=button]"))
        .find(el => (el.innerText || "").toLowerCase().includes(target.toLowerCase()));
  if (!row) return JSON.stringify({clicked: false, reason: "row_disappeared"});
  row.click();
  return JSON.stringify({clicked: true});
})()
'
```

If `clicked: false`, write `signin_status: "blocked"`, blocker `AGENT_RUN_FAILED` with reason from the eval, and stop.

Then poll for the redirect away from `accounts.google.com` to the `parent_window_origin`. Use a SINGLE polling eval (NOT a bash loop with multiple agent-browser calls — that wastes the attempt budget on subprocess overhead):

```bash
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const parentOrigin = "<PARENT_WINDOW_ORIGIN — substituted from job_parameters>";
  const deadline = Date.now() + Number("<REDIRECT_TIMEOUT_MS — default 15000>");
  while (Date.now() < deadline) {
    const url = window.location.href;
    const origin = window.location.origin;
    // Success: we have redirected to the parent origin (substring match —
    // Google may route via signin.<site>.com/socialsignin/... before landing
    // on the canonical origin).
    if (origin.includes(new URL(parentOrigin).hostname.replace(/^www\./, ""))
        || url.includes(parentOrigin)) {
      return JSON.stringify({redirected: true, final_url: url});
    }
    // Detect interstitial that may have appeared after the click (e.g. consent screen,
    // sudden captcha). Return early with the shape so the caller can branch.
    if (document.querySelector("iframe[src*=recaptcha]")) {
      return JSON.stringify({redirected: false, final_url: url, interstitial: "captcha"});
    }
    if (/Verify it.s you|2-Step Verification/.test((document.body && document.body.innerText) || "")) {
      return JSON.stringify({redirected: false, final_url: url, interstitial: "twofa"});
    }
    await new Promise(r => setTimeout(r, 500));
  }
  return JSON.stringify({redirected: false, final_url: window.location.href, interstitial: null, timeout: true});
})()
'
```

Map the polling result to `result.json`:

| Polling result | `signin_status` | `cdp_state` | Blocker |
|---|---|---|---|
| `redirected: true` | `signed_in` | `connected` | (none) |
| `redirected: false, interstitial: "captcha"` | `manual_required` | `connected` | `GOOGLE_INTERACTIVE_CHALLENGE` |
| `redirected: false, interstitial: "twofa"` | `manual_required` | `connected` | `GOOGLE_INTERACTIVE_CHALLENGE` |
| `redirected: false, timeout: true` | `manual_required` | `connected` | `GOOGLE_REDIRECT_TIMEOUT` |

In ALL paths, populate `account_selected` (the target email, when known) and `redirected_to` (the final URL from the polling eval) on the way out — they're useful for run-manifest auditing even on failure.

#### Step 3 — Leave the browser open

Do NOT navigate away, do NOT close the tab. The CALLER (per-site auth-signin skill) needs the redirected page to verify site-specific logged-in signals (avatar, account menu, restored s-token, etc.).

### Phase A discovery (mandatory before this skill is considered stable)

Selectors in this SKILL.md are based on Google's account-chooser shape as of 2026-05-17. Google revises the chooser DOM periodically. Re-validate every ~3 months by:

1. With Chrome on port 9222 and `dieter.x.s.danicek@gmail.com` signed into Chrome's Google session (visible at `chrome://chrome-signin`), open `https://accounts.google.com/AccountChooser?continue=https://myaccount.google.com/` in a tab.
2. DevTools Elements panel → find the row for the target email. Try each selector in the priority order above. Confirm `document.querySelectorAll('[data-identifier="dieter.x.s.danicek@gmail.com"]').length === 1`.
3. Test the click handler: `document.querySelector('[data-identifier="dieter.x.s.danicek@gmail.com"]').click()` → URL should change to `myaccount.google.com` within ~5s.
4. Sign out of Google in Chrome (`chrome://chrome-signin` → Sign out). Re-open the AccountChooser URL. Confirm the `email_entry_form` page shape now triggers (`input#identifierId` present, no chooser rows).
5. Sign back in. Update this SKILL.md with any selector deltas observed.

Record discovery results in `docs/testing/2026-05-17-cdp-auto-signin-test-report.md` under the "Phase A discovery — google-account-chooser" section.

---

## Canary

The canary is implicit: any successful invocation of a per-site `auth-signin/*` skill exercises this sub-skill end-to-end. There is no standalone canary input because the skill cannot run usefully without an upstream caller having navigated to `accounts.google.com`.

## Run Manifest Instructions

You must produce a run manifest the daemon can finalize into the durable `run-manifest.md` artifact.

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

`agent_claimed_source_url` is the FINAL URL after redirect (the `redirected_to` field in result.json). In Decisions Made, name the `page_shape` you detected in Step 1, the selector pattern that matched the target row (`data-identifier` vs `data-email` vs text fallback), and the redirect outcome.

Major claims must be anchored to observable evidence such as result-contract fields, daemon appendix fields, or artifact refs. Do not rely on operator memory or hidden context.
