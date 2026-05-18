---
skill_id: procurement-ingestion/procontract-register-interest
skill_version: 1.1.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: external_mutation
source_structure_version: procontract-interest-2026-04-13
requires_cdp: true
cdp_default_port: 9222
maxPages: 5
maxScrollCycles: 1
---

# procurement-ingestion/procontract-register-interest

## Use when
Use this skill when the daemon has already pre-filtered ProContract opportunities and needs the browser agent to register interest on the qualifying ones.

## Do not use when
Do not use this skill to discover new opportunities or to extract RFx document packs.

## Inputs
- `source_id`: should be `procontract-interest`
- `contracts_to_register`: array of qualifying contracts the daemon already approved
- `pre_filter_evaluated`: optional full evaluation list including skipped records for reporting

## Outputs
- `result.json` containing a `registrations[]` array and a `summary` object
- One result item per evaluated contract when `pre_filter_evaluated` is supplied

## Side Effects

This skill performs a real external mutation by clicking `Register interest in this opportunity` on a live procurement portal. That action is authorized.

## Failure Codes
- AGENT_AUTH_CHALLENGE
- PAGE_SHAPE_DRIFT
- SOURCE_SOFT_BLOCKED
- AGENT_PARTIAL_FAILURE
- AGENT_STALLED

---

## Playbook

You are a procurement portal action agent. The daemon has already decided which contracts are safe to register. Your job is to carry out the portal action and report what happened.

### Mandatory CDP Discipline

This skill must attach to the operator's already-authenticated persistent browser. Do not launch a fresh browser profile, do not attempt portal login, do not solve captchas, do not log out, and do not close the operator browser.

Before any portal navigation:

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
# rare post-launch race), follow the "If CDP is unreachable" guidance below:
# write the skill-specific blocked result.json and stop.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, write a valid blocked `result.json` immediately:

```json
{
  "cursor": "{}",
  "continuation_required": false,
  "registrations": [],
  "summary": {
    "total_evaluated": 0,
    "registered": 0,
    "already_registered": 0,
    "skipped_service": 0,
    "skipped_expiry": 0,
    "failed": 0
  },
  "status": "blocked",
  "blockers": [
    {
      "code": "CDP_CONNECT_FAILED",
      "message": "ProContract register-interest requires the operator persistent CDP browser on localhost:9222. Start Chrome/Chromium with remote debugging, sign in manually, then rerun.",
      "severity": "blocked"
    }
  ]
}
```

Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

### Session Recovery (auto-signin on session-expired)

This skill performs an EXTERNAL MUTATION (registering interest on the operator's ProContract account). Session Recovery for THIS skill follows the same auto-signin pattern as the other procontract skills, but with one extra-strict guard: after auto-signin returns `signed_in`, the skill MUST re-verify ALL preconditions in the existing precondition block (target contract from `contracts_to_register`, advert checked for already-registered state) BEFORE issuing any mutation click. Auto-signin is for restoring the session — it does NOT pre-authorize any pending mutation.

If any operation lands on a URL containing `/Login` OR the page text matches `Session expired|Please log in`, attempt auto-signin via `auth-signin/procontract/` BEFORE emitting `PROCONTRACT_SESSION_EXPIRED`. Only emit `SESSION_REFRESH_FAILED` if auto-signin ALSO failed.

**Session-expired detection signals (ProContract):**
- URL `pathname` contains `/Login` after a navigation that should have landed on an authenticated page.
- Page text matches `/Session expired|Please log in|Login or register for free/i`.

**Recovery flow:**

The canonical click sequence below was inlined from `skills/auth-signin/procontract/SKILL.md` §Workflow on 2026-05-17. ProContract uses Chrome's password autofill (NOT Google OAuth) and may require macOS Touch ID — the sequence below mirrors the auth-signin skill's behavioural detection of autofill firing and its 12-second submit-outcome budget. Selectors validated live against the persistent Chrome on port 9222 the same day. If you need to update the sequence, edit the source-of-truth at `skills/auth-signin/procontract/SKILL.md` first and re-inline here.

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# 1. Open ProContract login URL
agent-browser --cdp ${CDP_PORT} open "https://procontract.due-north.com/Login"
agent-browser --cdp ${CDP_PORT} wait 2500

# 2. Resolve the page shape: already-signed-in, landing card with "login here"
#    link, the direct login form, or an unknown state. If "landing_card",
#    click "login here" and wait; if "already_signed_in", exit recovery
#    immediately; if "unknown", treat as PROCONTRACT_LOGIN_PAGE_UNREACHABLE.
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const url = window.location.href;
  const pathname = window.location.pathname;
  const bodyText = (document.body && document.body.innerText) || "";
  if (!pathname.includes("/Login") && !/login or register/i.test(bodyText)) {
    return JSON.stringify({state: "already_signed_in", url});
  }
  const loginHereLink = Array.from(document.querySelectorAll("a")).find(a => /login here/i.test(a.innerText || ""));
  if (loginHereLink) {
    return JSON.stringify({state: "landing_card", url, href: loginHereLink.getAttribute("href")});
  }
  const userField = document.querySelector("input[name=username], input[name=Username], input[type=email]");
  if (userField) {
    return JSON.stringify({state: "login_form_visible", url, has_user_field: true});
  }
  return JSON.stringify({state: "unknown", url, body_excerpt: bodyText.slice(0, 200)});
})()
'
# If state == "landing_card", click "login here" and wait:
# agent-browser --cdp ${CDP_PORT} eval '(() => { const a = Array.from(document.querySelectorAll("a")).find(a => /login here/i.test(a.innerText || "")); if (!a) return JSON.stringify({clicked: false}); a.click(); return JSON.stringify({clicked: true}); })()'
# agent-browser --cdp ${CDP_PORT} wait 2000

# 3. Click the username field to trigger Chrome password autofill. We do NOT
#    control the popup — we observe the field-value side-effect in Step 4.
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const userField = document.querySelector("input[name=username], input[name=Username], input[type=email]:not([type=hidden])");
  if (!userField) return JSON.stringify({clicked: false, reason: "username_field_not_found"});
  userField.click();
  userField.focus();
  return JSON.stringify({clicked: true});
})()
'

# 4. Behavioural detection of autofill firing (3-second budget). Single
#    polling eval — do NOT loop bash subprocess calls (overhead).
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
# If autofill_fired: false → blocked (autofill disabled or saved credential missing).

# 5. Click the submit button (NOT form.submit() — that bypasses Chrome's
#    autofill commit hooks and sometimes results in an empty post).
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const submitBtn = document.querySelector("button[type=submit], input[type=submit], button[name=login], button[id*=login i]");
  if (!submitBtn) {
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

# 6. Behavioural detection of submit outcome (12-second budget). Three signals:
#    (a) signed_in — URL transitions off /Login; (b) blocked_error — explicit
#    alert visible; (c) timeout — most likely macOS Touch ID OS-modal is
#    blocking the page. 12s is the budget (Touch ID typically completes in 5-8s).
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const deadline = Date.now() + 12000;
  while (Date.now() < deadline) {
    const url = window.location.href;
    const pathname = window.location.pathname;
    if (!pathname.includes("/Login")) {
      return JSON.stringify({outcome: "signed_in", url});
    }
    const errorEl = document.querySelector(".alert-danger, .alert.alert-error, .error-message, [role=alert]");
    const errorText = errorEl ? (errorEl.innerText || "").trim() : "";
    if (errorText && errorText.length > 5) {
      return JSON.stringify({outcome: "blocked_error", url, error_text: errorText.slice(0, 200)});
    }
    await new Promise(r => setTimeout(r, 500));
  }
  return JSON.stringify({outcome: "timeout", url: window.location.href});
})()
'
```

After the recovery sequence completes, map the outcomes back to this skill's `status` / blocker taxonomy. THIS skill performs an EXTERNAL MUTATION, so a successful auto-signin does NOT pre-authorize the pending mutation — the precondition re-verification below is mandatory:

1. **If Step 6's `outcome: "signed_in"` (or Step 2 reported `state: "already_signed_in"`):** RE-VERIFY ALL PRECONDITIONS (per the existing precondition block) before issuing any "Register interest" click. The daemon may have re-invoked the skill in a context where the contract is no longer in `contracts_to_register` OR has been registered by a parallel operator action. If the precondition re-check fails, SKIP the registration and record it in the skill's existing `skipped_*` counters — do NOT proceed with the mutation. If all preconditions still hold, proceed with the registration click as normal.

2. **If Step 6's `outcome: "timeout"` (Touch ID pending), Step 4 returned `autofill_fired: false`, Step 6's `outcome: "blocked_error"`, OR Step 2 returned `state: "unknown"` / Step 3 returned `clicked: false`:** write a blocked `result.json` with `status: "blocked"` and one blocker `{code: "SESSION_REFRESH_FAILED", severity: "blocked", message: "<specific failure from the auto-signin outcome: Touch ID pending, Chrome autofill disabled, saved credential missing, ProContract login error visible, login form shape changed, etc.>"}`. Stop here. NO registration is attempted.

The legacy `PROCONTRACT_SESSION_EXPIRED` blocker is now a fallback: used only when this Session Recovery path has been attempted unsuccessfully AND the operator must restore the session manually. In that case, prefer the `SESSION_REFRESH_FAILED` blocker code.

If the portal shows `Log In`, `Session expired`, captcha, or another auth challenge, run the Session Recovery flow above first. Only emit `PROCONTRACT_SESSION_EXPIRED` or `AGENT_AUTH_CHALLENGE` after auto-signin has failed. Do not click any mutation control when auth state is uncertain — the precondition re-verification after auto-signin is non-negotiable.

Before any external mutation, confirm all preconditions:

- CDP preflight succeeded.
- The current page is authenticated and not an auth challenge.
- The target contract came from `contracts_to_register`.
- The advert has been checked for an already-registered state before clicking.

### Step 1: Read the daemon input

Read:

- `contracts_to_register`: the contracts you must action
- `pre_filter_evaluated`: optional evaluation report for all checked contracts

If `pre_filter_evaluated` is present:

- Any item already marked `action: "skipped"` should be copied through to the final `registrations[]` unchanged.
- Only perform portal actions for the items in `contracts_to_register`.

If `contracts_to_register` is empty:

- Write a valid empty result with copied skipped items, if any.
- Do not attempt portal navigation beyond optional authenticated-session verification.

### Step 2: Open the authorized advert in the persistent browser

For each contract URL:

```bash
agent-browser --cdp ${CDP_PORT} open "{source_url}"
agent-browser --cdp ${CDP_PORT} wait 2000
```

If the page is `Log In`, `Session expired`, captcha, or any auth challenge, stop and write the blocked result described in Mandatory CDP Discipline. Do not click any mutation control when auth state is uncertain.

### Step 3: Detect already-registered state before clicking

The live advert page exposes the already-registered state clearly:

- A section heading `Expression of interest registered`
- Fields such as `Date` and `Workgroup`

If that section already exists before you click:

- Do not click the interest link again
- Emit:
  - `action: "already_registered"`
  - `registered_at`: if visible on the page

### Step 4: Perform the real registration

When the advert still shows `Register interest in this opportunity` and no prior registration section:

```bash
agent-browser --cdp ${CDP_PORT} click "Register interest in this opportunity"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot -i
```

Success is indicated by a modal/dialog containing:

- `Expression of interest successful`
- `What happens next?`
- A link to the `activities centre`

Also verify that the page now contains `Expression of interest registered`.

Emit:

- `action: "registered"`
- `registered_at`: current observed UTC time if the page does not show a timestamp yet

### Step 5: Handle failures

If registration cannot be completed, emit:

- `action: "failed"`
- `failure_reason`: short machine-readable reason such as:
  - `button_not_found`
  - `session_expired`
  - `confirmation_missing`
  - `portal_error`

### Step 6: Build the result contract

Write `result.json` with this top-level shape:

```json
{
  "cursor": "{\"last_processed_at\":\"2026-04-13T15:00:00Z\"}",
  "continuation_required": false,
  "registrations": [],
  "summary": {
    "total_evaluated": 0,
    "registered": 0,
    "already_registered": 0,
    "skipped_service": 0,
    "skipped_expiry": 0,
    "failed": 0
  }
}
```

Per-item fields:

```text
notice_id:             required
process_id:            required if known from input
source_url:            required for acted items
action:                one of registered | already_registered | skipped | failed
skip_reason:           required when action == skipped
failure_reason:        required when action == failed
pre_filter_result:     copy from daemon input when present
registered_at:         ISO 8601 timestamp when action is registered/already_registered
```

### Step 7: Leave the persistent browser open

Do not close or reset the operator browser. Leave the last navigated ProContract page available for operator inspection.

---

## Canary

Verify that a qualifying ProContract advert can be opened through an already-authenticated CDP session, that the `Register interest in this opportunity` control is discoverable, and that the already-registered state can be distinguished from the pre-registration state.

## Run Manifest Instructions

Produce a run manifest with:

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

Narrative sections:

- Summary
- Steps Taken
- Decisions Made
- Warnings
- Evidence
