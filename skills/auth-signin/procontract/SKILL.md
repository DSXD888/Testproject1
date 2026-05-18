---
skill_id: auth-signin/procontract
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

# auth-signin/procontract

## Use when
Use this skill when one of the `procurement-ingestion/procontract-*` skills (`procontract-discover`, `procontract-enrich-tender`, `procontract-register-interest`) has detected that the operator's persistent Chrome profile is signed OUT of `procontract.due-north.com` (URL contains `/Login` after navigation, OR page text matches `Session expired|Please log in`).

ProContract does NOT use Google OAuth. Instead, the operator's macOS Passwords (synced via iCloud Keychain) holds a saved credential for `sales@sorelon.com` against the ProContract login form. Chrome's password autofill — when enabled at `chrome://settings/passwords` — populates the form fields automatically when the operator (or this skill, via a JS click) focuses the username input. The skill's job is to:

1. Trigger the autofill by clicking the username field.
2. Detect that autofill fired (the field populates without us typing).
3. Click submit.
4. Detect the outcome: signed-in, error, or Touch ID waiting (OS-modal).

## Do not use when
Do not invoke when:
- The operator's macOS Touch ID is the only authentication factor for unlocking the credential AND the operator is not physically present. The skill cannot bypass Touch ID — it surfaces as `manual_required` after the 12-second budget elapses.
- Chrome's password autofill is disabled at `chrome://settings/passwords`. The skill detects this (autofill never fires within 3s) and surfaces as `manual_required` with explicit guidance.
- The saved credential for `sales@sorelon.com` is missing from macOS Passwords. Same handling — surface as `manual_required`.

## Inputs
No `job_parameters` accepted (the autofill credential is identified by the website, not by skill input).

## Outputs
- `result.json` matching `result.schema.json`. Includes `autofill_fired: bool` and `submit_outcome: "signed_in" | "blocked_error" | "touch_id_pending" | "timeout"` for diagnostic clarity.

## Sensitivity warning

The ProContract account `sales@sorelon.com` is the operator's REAL procurement account with bid history and financial implications. This skill MUST NOT:

- Submit any form OTHER than the login form (no tender registration, no profile changes, no clicking "Register interest").
- Modify account settings.
- Cache or log autofilled values to disk.
- Sign out at session end — subsequent skills need the session.
- Type the credential directly — rely on autofill only. If autofill never fires, escalate (do NOT fall back to typing).

The skill's job is narrow: click username field → wait for autofill → click submit → wait for outcome → return.

## Non-goals
- Registering interest in tenders, downloading attachments, modifying profile, sending messages.
- Solving captchas.
- Bypassing Touch ID.
- Typing the credential.

## Failure Codes
- `CDP_CONNECT_FAILED` — `bin/ensure-cdp-9222` failed or CDP preflight did not respond.
- `PROCONTRACT_LOGIN_PAGE_UNREACHABLE` — opened the login URL but never landed on the expected login form.
- `PROCONTRACT_AUTOFILL_DID_NOT_FIRE` — clicked the username field but Chrome's autofill never populated it within 3 seconds. Operator action required: enable autofill OR verify the saved credential exists.
- `PROCONTRACT_TOUCH_ID_PENDING` — submitted login, no progress within 12 seconds, likely macOS Touch ID OS-modal is blocking the page. Operator action required.
- `PROCONTRACT_LOGIN_ERROR_VISIBLE` — submitted login, ProContract displayed an error message (account locked, rate limit, server error). Operator action required.
- `PROCONTRACT_SIGNED_IN_SIGNAL_ABSENT` — completed the submit, 12-second budget elapsed, no error visible but URL still contains `/Login`. Operator inspects.

## Resume Strategy
Not resumable. Atomic.

## Pagination Bounds
- `maxPages`: 3 (ProContract homepage / login URL → potentially a "login here" link → login form → authenticated landing)
- `maxScrollCycles`: 1

---

## Playbook

You are a ProContract auto-signin agent. Unlike the Google OAuth skills, you cannot inspect Chrome's autofill popup directly — it's a browser-UI element, not a DOM element. Detect autofill behaviourally by polling the username field's `value`.

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

#### Step 1 — Open ProContract login URL

```bash
agent-browser --cdp ${CDP_PORT} open "https://procontract.due-north.com/Login"
agent-browser --cdp ${CDP_PORT} wait 2500
```

#### Step 2 — Verify NOT already signed in, and find the "login here" link if shown

ProContract's `/Login` URL sometimes shows a "Login or register for free" landing card with a separate "login here" link, and sometimes shows the login form directly:

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const url = window.location.href;
  const pathname = window.location.pathname;
  const bodyText = (document.body && document.body.innerText) || "";
  // Already signed in? URL would have redirected away from /Login
  if (!pathname.includes("/Login") && !/login or register/i.test(bodyText)) {
    return JSON.stringify({state: "already_signed_in", url});
  }
  // Landing card with separate "login here" link?
  const loginHereLink = Array.from(document.querySelectorAll("a")).find(a => /login here/i.test(a.innerText || ""));
  if (loginHereLink) {
    return JSON.stringify({state: "landing_card", url, href: loginHereLink.getAttribute("href")});
  }
  // Direct login form?
  const userField = document.querySelector("input[name=username], input[name=Username], input[type=email]");
  if (userField) {
    return JSON.stringify({state: "login_form_visible", url, has_user_field: true});
  }
  return JSON.stringify({state: "unknown", url, body_excerpt: bodyText.slice(0, 200)});
})()
'
```

Decision:
- `already_signed_in` → write `signin_status: "signed_in"`, stop with success.
- `landing_card` → click the "login here" link:
  ```bash
  agent-browser --cdp ${CDP_PORT} eval '
  (() => {
    const a = Array.from(document.querySelectorAll("a")).find(a => /login here/i.test(a.innerText || ""));
    if (!a) return JSON.stringify({clicked: false});
    a.click();
    return JSON.stringify({clicked: true});
  })()
  '
  agent-browser --cdp ${CDP_PORT} wait 2000
  ```
  Then proceed to Step 3.
- `login_form_visible` → proceed directly to Step 3.
- `unknown` → write blocked with `PROCONTRACT_LOGIN_PAGE_UNREACHABLE`.

#### Step 3 — Click the username field to trigger Chrome autofill

The CRITICAL behaviour: clicking the username field surfaces Chrome's autofill suggestion popup. If the operator has enabled autofill AND the saved credential exists, Chrome populates both username AND password fields within ~1-2 seconds, fires `input` events on each, and the form is ready to submit. We do NOT control or inspect the popup — we observe the field-value side-effect.

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const userField = document.querySelector("input[name=username], input[name=Username], input[type=email]:not([type=hidden])");
  if (!userField) return JSON.stringify({clicked: false, reason: "username_field_not_found"});
  userField.click();
  userField.focus();
  return JSON.stringify({clicked: true});
})()
'
```

If `clicked: false`, write blocked with `PROCONTRACT_LOGIN_PAGE_UNREACHABLE` (login form shape changed).

#### Step 4 — Behavioural detection of autofill firing

Poll the field value for up to 3 seconds. Use a SINGLE polling eval, not a bash loop with multiple agent-browser calls (subprocess overhead would eat half the budget):

```bash
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const TARGET_USER = "sales@sorelon.com";
  const deadline = Date.now() + 3000;
  while (Date.now() < deadline) {
    const userField = document.querySelector("input[name=username], input[name=Username], input[type=email]:not([type=hidden])");
    const passField = document.querySelector("input[type=password]:not([type=hidden])");
    const userVal = (userField && userField.value) || "";
    const passVal = (passField && passField.value) || "";
    if (userVal === TARGET_USER && passVal.length > 0) {
      return JSON.stringify({autofill_fired: true, user_val: userVal, pass_present: true});
    }
    await new Promise(r => setTimeout(r, 250));
  }
  // After 3s, capture final state for diagnostic
  const userField = document.querySelector("input[name=username], input[name=Username], input[type=email]:not([type=hidden])");
  const passField = document.querySelector("input[type=password]:not([type=hidden])");
  return JSON.stringify({
    autofill_fired: false,
    final_user_val: (userField && userField.value) || "",
    final_user_val_length: ((userField && userField.value) || "").length,
    final_pass_val_length: ((passField && passField.value) || "").length
  });
})()
'
```

If `autofill_fired: false`, write blocked result with `PROCONTRACT_AUTOFILL_DID_NOT_FIRE`. The blocker message MUST be specific so the operator knows what to do:

```text
Chrome password autofill did not populate the username field within 3 seconds. Likely causes (operator must triage):
  (a) Chrome password autofill is disabled — enable it at chrome://settings/passwords.
  (b) The saved credential for sales@sorelon.com is missing from macOS Passwords / Chrome.
  (c) macOS Passwords sync glitch — check the credential is visible in System Settings > Passwords.
Until one of these is resolved, the operator must sign in to ProContract manually in the persistent Chrome browser.
```

If `autofill_fired: true`, proceed to Step 5.

#### Step 5 — Click the submit button

Trigger form submission via the submit button (NOT `form.submit()` — that bypasses Chrome's autofill commit hooks and sometimes results in an empty post):

```bash
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const submitBtn = document.querySelector("button[type=submit], input[type=submit], button[name=login], button[id*=login i]");
  if (!submitBtn) {
    // Fallback: any visible button whose text matches /^(log[ ]?in|continue|sign[ ]?in)$/i
    const fallback = Array.from(document.querySelectorAll("button, input[type=button]"))
      .find(b => /^(log[ ]?in|continue|sign[ ]?in)$/i.test((b.innerText || b.value || "").trim()));
    if (!fallback) return JSON.stringify({submitted: false, reason: "no_submit_button_found"});
    fallback.click();
    return JSON.stringify({submitted: true, via: "text_fallback"});
  }
  submitBtn.click();
  return JSON.stringify({submitted: true, via: "type_submit"});
})()
'
```

If `submitted: false`, write blocked with `PROCONTRACT_LOGIN_PAGE_UNREACHABLE` (form shape changed mid-flow).

#### Step 6 — Behavioural detection of submit outcome (12-second budget)

Three possible outcomes:
1. **Signed in:** URL transitions away from `/Login`.
2. **Error:** ProContract displays an error alert (`.alert-danger`, `.error`, `[role=alert]`).
3. **Touch ID waiting:** No progress at all — most likely macOS Touch ID OS-modal is blocking the page (the operator has to physically authenticate). After 12 seconds with no signal, surface as such.

12 seconds is the budget. Touch ID typically completes within 5-8s when the operator is present and ready. Sleeping 30s would burn the daemon's attempt budget; 12s balances responsiveness with allowing a present operator to authenticate.

```bash
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const deadline = Date.now() + 12000;
  while (Date.now() < deadline) {
    const url = window.location.href;
    const pathname = window.location.pathname;
    // Signal A: signed in (URL no longer contains /Login)
    if (!pathname.includes("/Login")) {
      return JSON.stringify({outcome: "signed_in", url});
    }
    // Signal B: explicit error message
    const errorEl = document.querySelector(".alert-danger, .alert.alert-error, .error-message, [role=alert]");
    const errorText = errorEl ? (errorEl.innerText || "").trim() : "";
    if (errorText && errorText.length > 5) {
      return JSON.stringify({outcome: "blocked_error", url, error_text: errorText.slice(0, 200)});
    }
    await new Promise(r => setTimeout(r, 500));
  }
  // After 12s, no URL change and no error — likely Touch ID OS-modal waiting,
  // or network slow, or ProContract rate-limiting.
  return JSON.stringify({outcome: "timeout", url: window.location.href});
})()
'
```

Map the outcome to result.json:

| Outcome | `signin_status` | `submit_outcome` | Blocker |
|---|---|---|---|
| `signed_in` | `signed_in` | `signed_in` | (none) |
| `blocked_error` | `blocked` | `blocked_error` | `PROCONTRACT_LOGIN_ERROR_VISIBLE` with `error_text` from poll |
| `timeout` | `manual_required` | `touch_id_pending` | `PROCONTRACT_TOUCH_ID_PENDING` |

For the `timeout` case, the blocker message MUST be specific:

```text
ProContract login submit returned no progress within 12 seconds (no URL change, no error message). Most likely macOS Touch ID prompt is waiting at the OS level — the operator must physically authenticate at the Touch ID prompt, then re-trigger the job. Less likely: network is slow OR ProContract is rate-limiting. Operator action: check for a pending Touch ID prompt; if none, retry once.
```

#### Step 7 — Leave the browser open

Do NOT close, do NOT navigate away, do NOT sign out. Subsequent `procurement-ingestion/procontract-*` skills share this session.

### Phase A discovery (mandatory before first stable run)

1. With Chrome on port 9222 and macOS Passwords containing the saved credential for `sales@sorelon.com` against `procontract.due-north.com`, navigate manually to `https://procontract.due-north.com/Login`. Confirm:
   - The form's username field uses `name="username"` (or capture the actual name).
   - The password field is `input[type="password"]` and visible (not hidden).
   - The submit button is `button[type="submit"]` (or capture actual selector).
2. Sign OUT of ProContract. Re-open `/Login`. Confirm Step 2's eval returns `state: "login_form_visible"` OR `state: "landing_card"` (capture which).
3. Manually click the username field. Within 1-3 seconds, observe Chrome's autofill suggestion popup → click the `sales@sorelon.com` suggestion → confirm both fields populate. This is the operator-visible analog of Step 4's behavioural detection.
4. Click submit. If Touch ID prompts, time how long it takes to satisfy (typically 3-5s). If autofill silently commits without Touch ID (Chrome's "use a saved password" setting differs by macOS version), document this and the skill's success rate increases.
5. Disable autofill at `chrome://settings/passwords`. Re-run the full flow. Confirm Step 4 returns `autofill_fired: false` with the expected diagnostic values.
6. Re-enable autofill. Confirm Step 4 returns `autofill_fired: true`.
7. Test error path: enter wrong credentials manually, then run the skill (the wrong-credentials state should resolve to `blocked_error` with the visible ProContract error message). Then restore correct credentials.

Record discovery in `docs/testing/2026-05-17-cdp-auto-signin-test-report.md` under "Phase A discovery — ProContract".

---

## Canary

Triggered as part of `procurement-ingestion/procontract-discover` end-to-end tests: manually sign out of ProContract in the persistent Chrome, run procontract-discover against a fresh batch, observe this skill is invoked + procontract-discover completes successfully on the daemon's retry pass. The Touch ID case is operator-witnessed (skill cannot self-verify Touch ID was actually displayed).

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

`agent_claimed_source_url` is the FINAL URL after the submit. In Decisions Made, name the `state` returned by Step 2 (landing card vs. direct form), the `autofill_fired` value from Step 4 (with timing if available), and the submit outcome from Step 6. If `outcome: "timeout"`, explicitly call out that Touch ID is the most likely explanation but cannot be confirmed from DOM observation alone.
