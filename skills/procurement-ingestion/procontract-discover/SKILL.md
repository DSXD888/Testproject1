---
skill_id: procurement-ingestion/procontract-discover
skill_version: 1.1.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
source_structure_version: procontract-2026-04-13
requires_cdp: true
cdp_default_port: 9222
maxPages: 10
maxScrollCycles: 3
---

# procurement-ingestion/procontract-discover

## Use when
Use this skill when the daemon needs a fresh or incremental batch of ProContract opportunities and must convert them into the standard `notices[]` ingestion contract.

## Do not use when
Do not use this skill for interest registration, RFx response extraction, bid submission, or any buyer-facing mutation.

## Inputs
- Optional `cursor` string from `source_cursors.notice_cursor`.
- Optional `publishedFrom` / `publishedTo` in ISO `YYYY-MM-DD` format for narrowing the listing.
- Optional `limit` integer.
- Optional `startPage` integer when resuming a continuation run.

## Outputs
- Standard procurement ingestion contract with top-level `cursor`, `continuation_required`, and `notices`.
- Each notice must set `ocid` to the same `pc-{advertId}` value as `notice_id`.
- `result.json` written in the current working directory.

## Failure Codes
- AGENT_AUTH_CHALLENGE
- PAGE_SHAPE_DRIFT
- SOURCE_SOFT_BLOCKED
- AGENT_STALLED
- AGENT_PARTIAL_FAILURE

## Resume Strategy
- Resume from the saved page number in `cursor.page`.
- Re-apply the same published-date lower bound.
- Re-fetch the current page with a one-page overlap instead of assuming stable ordering.

## Pagination Bounds
- `maxPages`: 10
- `maxScrollCycles`: 3

---

## Playbook

You are a procurement data ingestion agent. Retrieve ProContract opportunities, open every advert detail page that you include in the result, and return a valid notice contract.

### Mandatory CDP Discipline

This skill must attach to the operator's already-authenticated persistent browser. Do not launch a fresh browser profile, do not attempt portal login, do not solve captchas, do not log out, and do not close the operator browser.

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

If CDP is unreachable, write a valid blocked `result.json` immediately:

```json
{
  "cursor": "{}",
  "continuation_required": false,
  "notices": [],
  "status": "blocked",
  "blockers": [
    {
      "code": "CDP_CONNECT_FAILED",
      "message": "ProContract discovery requires the operator persistent CDP browser on localhost:9222. Start Chrome/Chromium with remote debugging, sign in manually, then rerun.",
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

If a detail page redirects to `/Login` OR the page text matches `Session expired|Please log in`, attempt auto-signin via `auth-signin/procontract/` BEFORE emitting `PROCONTRACT_SESSION_EXPIRED`. Only emit `SESSION_REFRESH_FAILED` if auto-signin ALSO failed (Touch ID not satisfied, password autofill disabled, saved credential missing, ProContract login error visible).

**Session-expired detection signals (ProContract):**
- URL `pathname` contains `/Login` after a navigation that should have landed on an authenticated page (e.g., `/Opportunities/Index` or `/Supplier/Advert/View`).
- Page text matches `/Session expired|Please log in|Login or register for free/i`.
- The "login here" link from the unauthenticated landing card is visible.

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

After the recovery sequence completes, map the outcomes back to this skill's blocker taxonomy:

1. **If Step 6's `outcome: "signed_in"` (or Step 2 reported `state: "already_signed_in"`):** EXIT this skill cleanly WITHOUT writing `result.json`. The daemon's retry policy at `crates/daemon/src/jobs/runner.rs` re-invokes the skill on the next pass; the second invocation reaches the authenticated detail page and completes normally.

2. **If Step 6's `outcome: "timeout"` (Touch ID pending):** write a blocked `result.json` with one blocker `{code: "SESSION_REFRESH_FAILED", severity: "blocked", message: "ProContract auto-signin invoked Chrome's password autofill and clicked submit, but received no progress within 12 seconds. Most likely macOS Touch ID prompt is waiting at the OS level — the operator must physically authenticate at the Touch ID prompt, then re-trigger the job."}`. Stop here.

3. **If Step 4 returned `autofill_fired: false`, Step 6's `outcome: "blocked_error"`, OR Step 2 returned `state: "unknown"` / Step 3 returned `clicked: false`:** write a blocked `result.json` with one blocker `{code: "SESSION_REFRESH_FAILED", severity: "blocked", message: "<specific failure: Chrome autofill disabled, saved credential missing, ProContract login error visible, or login form shape changed>"}`. Stop here.

The legacy `PROCONTRACT_SESSION_EXPIRED` blocker is now a fallback: used when this Session Recovery path has been attempted unsuccessfully AND the operator must restore the session manually in the persistent CDP browser.

If a detail page shows `Log In`, `Session expired`, captcha, or another auth challenge, run the Session Recovery flow above first. Only emit `PROCONTRACT_SESSION_EXPIRED` or `AGENT_AUTH_CHALLENGE` after auto-signin has failed — and in that case, the blocker code is `SESSION_REFRESH_FAILED` (not the legacy codes), telling the operator the autonomous recovery already tried and what specifically blocked it.

### Step 1: Read the job parameters

Read the injected daemon parameters:

- `source_id`: should be `procontract`
- `cursor`: optional JSON string like `{"page":2,"last_published":"2026-04-13"}`
- `publishedFrom`: optional lower bound date, `YYYY-MM-DD`
- `publishedTo`: optional upper bound date, `YYYY-MM-DD`
- `limit`: maximum notices to emit, default `25`, hard cap `100`
- `startPage`: optional starting page, default `1`

If no `publishedFrom` is supplied, prefer the `cursor.last_published` value. If neither exists, leave the listing unfiltered.

### Step 2: Open the opportunities listing

Open the public opportunities listing:

```bash
agent-browser --cdp ${CDP_PORT} open "https://procontract.due-north.com/Opportunities/Index?tabName=opportunities&resetFilter=True"
agent-browser --cdp ${CDP_PORT} wait 2000
agent-browser --cdp ${CDP_PORT} snapshot -i
```

Important observations from the live page:

- The listing is public.
- The table columns are `Title`, `Buyer`, `Expression Start`, `Expression End`, and `Estimated value`.
- The left rail includes `Portals`, `Organisations`, `Categories`, `Regions`, `Keywords`, `Include closed`, `Expression date`, and `Published date`.
- Public listing links use `/Advert?advertId=...`.
- Authenticated detail pages resolve to `/Supplier/Advert/View?advertId=...`.

### Step 3: Apply filters when requested

If `publishedFrom` or `publishedTo` is present:

1. Fill the `Published date` start/end fields using `dd/mm/yyyy`.
2. Click `Update`.
3. Wait for the table to refresh.

If the page shape drifts and you cannot reliably identify those controls, continue without the filter and record a warning in the run manifest.

### Step 4: Parse the current listing page

For each row on the listing page, capture:

- Listing title text
- Buyer name
- Expression start date
- Expression end date
- Estimated value text
- The advert link href containing `advertId=...`

Extract `advertId` from the row link href. Public rows normally look like:

```text
/Advert?advertId=f0f30f94-e22c-f111-813c-005056b64545
```

### Step 5: Open every advert detail page before emitting it

For each advert you plan to include:

1. Open the advert URL. Prefer the public form first:

```bash
agent-browser --cdp ${CDP_PORT} open "https://procontract.due-north.com/Advert?advertId={advertId}"
```

2. If the browser lands on `Log In`, `Session expired`, captcha, or any auth challenge, stop and write the blocked result described in Mandatory CDP Discipline. The operator must restore the session manually in the persistent CDP browser.

3. Confirm you reached the authenticated advert page. The live detail page contains:
- `Main contract details`
- `Key dates`
- `Expression of interest window`
- `Contact details`
- Optional `Attachments`
- A `Register interest in this opportunity` link

4. Extract all high-level fields from the detail page, not from the listing row only.

### Step 6: Field extraction and mapping

Map the advert detail page into a notice object:

```text
notice_id:                    "pc-{advertId}"
source_notice_id:             "{advertId}"
source_url:                   current authenticated advert URL
ocid:                         "pc-{advertId}"
title:                        page heading / Title field
summary:                      full Description text
buyer_name:                   Buyer field under Contact details
procurement_stage:            map page state:
                              - if detail page shows register-interest window and no award language -> "tender"
                              - if title/description clearly says awarded/closed -> "awarded" or "closed"
                              - if clearly pipeline/future -> "future_opportunity"
main_category:                Categories text joined with " | "
deadline_at:                  Expression of interest end timestamp in ISO 8601 UTC
published_at:                 listing Expression Start date at 00:00:00Z unless a better published date is visible
source_updated_at:            published_at unless the page shows a fresher date
estimated_value_low_pence:    listing/detail estimated value converted from GBP to pence, else null
estimated_value_currency:     "GBP" when a value exists, otherwise "GBP"
access_route:                 "portal_authenticated"
raw_release_json:             stringified JSON object of all raw scraped fields
```

Date rules:

- Convert `dd/mm/yyyy` to `YYYY-MM-DDT00:00:00Z` when time is unknown.
- Convert `From dd/mm/yyyy HH:MM to dd/mm/yyyy HH:MM` expression windows using the end timestamp for `deadline_at`.

Value rules:

- `£10,000,000.00` -> `1000000000`
- `N/A` -> `null`

### Step 7: Pagination and continuation

After finishing the current page:

- Stop if you have emitted `limit` notices.
- Stop if you reached `maxPages`.
- If another page exists and you still have budget, click `Next >` or open the next page number directly and continue.

Set:

- `continuation_required: true` when more pages remain beyond the current batch limit or page budget.
- `cursor` to a JSON string with:
  - `page`: the last page fully processed
  - `last_published`: the latest publication-related date you relied on

Example:

```json
{"page":2,"last_published":"2026-04-13"}
```

### Step 8: Write `result.json`

Write a final JSON object shaped exactly like `result.schema.json` to `result.json` in the current working directory.

If no notices are found, still write a valid empty result:

```json
{
  "cursor": "{\"page\":1}",
  "continuation_required": false,
  "notices": []
}
```

### Step 9: Leave the persistent browser open

Do not close or reset the operator browser. Leave the last navigated ProContract page available for operator inspection.

---

## Canary

Fetch up to three current ProContract opportunities, verify that the public listing is reachable, verify that advert links expose an `advertId`, and verify that at least one authenticated detail page can be opened when the operator session is already signed in through CDP.

## Run Manifest Instructions

Produce a run manifest with YAML frontmatter containing:

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

Narrative sections required:

- Summary
- Steps Taken
- Decisions Made
- Warnings
- Evidence
