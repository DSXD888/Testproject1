---
skill_id: procurement-ingestion/procontract-enrich-tender
skill_version: 1.3.0
interaction_profile: pty_interactive
result_schema_version: 1.1.0
checkpoint_schema_version: 1.1.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
source_structure_version: procontract-enrich-2026-04-13
requires_cdp: true
cdp_default_port: 9222
maxPages: 5
maxScrollCycles: 2
---

# procurement-ingestion/procontract-enrich-tender

## Use when
Use this skill when the daemon needs full RFx/tender enrichment from ProContract after interest has already been registered.

## Do not use when
Do not use this skill for listing discovery or for clicking the initial interest-registration action.

## Inputs
- `source_id`: should be `procontract-enrich`
- `download_root`: absolute directory where all attachment files must be saved
- Optional `cursor`
- Optional `awaitingAccessTargets`: daemon-provided durable work-list of
  ProContract tenders that have registered interest but are still waiting for
  the live RFx event. Each item has:
  - `advertId`: ProContract advert/activity identifier
  - `sourceNoticeId`: daemon notice key for the pending tender
  - `processId`: daemon procurement process id

If `awaitingAccessTargets` is missing or empty, run the scan-only flow exactly
as before. If it is present, union these direct targets with the Recently added
scan results and dedupe by `advertId` before enrichment.

## Outputs
- `result.json` containing `enriched_tenders[]`, `not_yet_responded[]`,
  `failed_targets[]`, `directly_visited[]`, and `target_evidence[]`
- Each enriched tender includes the updated deadline and the downloaded document inventory
- Each awaited target must be classified as exactly one of:
  `enriched`, `not_yet_responded`, or `failed`
- `directly_visited[]` records daemon-supplied targets that were reached by
  advert/source id rather than only by the portal scan

## Failure Codes
- AGENT_AUTH_CHALLENGE
- CDP_CONNECT_FAILED
- PAGE_SHAPE_DRIFT
- SOURCE_SOFT_BLOCKED
- AGENT_PARTIAL_FAILURE
- AGENT_STALLED
- SESSION_REFRESH_FAILED

---

## Playbook

You are a procurement enrichment agent. Discover tender-ready ProContract activities from the authenticated portal, open the RFx summary for each one, download every attachment, and report the enriched data.

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
  "enriched_tenders": [],
  "not_yet_responded": [],
  "failed_targets": [],
  "directly_visited": [],
  "target_evidence": [],
  "summary": {
    "total_checked": 0,
    "enriched": 0,
    "not_yet_responded": 0,
    "failed": 0,
    "direct_targets_received": 0,
    "direct_targets_visited": 0,
    "directly_visited": 0
  },
  "status": "blocked",
  "blockers": [
    {
      "code": "CDP_CONNECT_FAILED",
      "message": "ProContract enrichment requires the operator persistent CDP browser on localhost:9222. Start Chrome/Chromium with remote debugging, sign in manually, then rerun.",
      "severity": "blocked"
    }
  ]
}
```

If `awaitingAccessTargets` was supplied and CDP or session recovery blocks the
run before target inspection, every supplied target remains daemon-owned
`awaiting_access`. Do not mark it expired. Emit one `failed_targets[]` and one
`target_evidence[]` entry per supplied target with the blocking code
(`CDP_CONNECT_FAILED` or `SESSION_REFRESH_FAILED`) so the daemon can retain the
row and retry later.

Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

### Session Recovery (auto-signin on session-expired)

If any enrich operation lands on a URL containing `/Login` OR the page text matches `Session expired|Please log in`, attempt auto-signin via `auth-signin/procontract/` BEFORE emitting `PROCONTRACT_SESSION_EXPIRED`. Only emit `SESSION_REFRESH_FAILED` if auto-signin ALSO failed (Touch ID not satisfied, password autofill disabled, saved credential missing, ProContract login error visible).

**Session-expired detection signals (ProContract):**
- URL `pathname` contains `/Login` after a navigation that should have landed on the authenticated home page or tender detail page.
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

After the recovery sequence completes, map the outcomes back to this skill's `status` / blocker taxonomy:

1. **If Step 6's `outcome: "signed_in"` (or Step 2 reported `state: "already_signed_in"`):** EXIT this skill cleanly WITHOUT writing `result.json`. The daemon's retry policy at `crates/daemon/src/jobs/runner.rs` re-invokes the skill on the next pass; the second invocation reaches the authenticated tender detail page and completes enrichment normally.

2. **If Step 6's `outcome: "timeout"` (Touch ID pending), Step 4 returned `autofill_fired: false`, Step 6's `outcome: "blocked_error"`, OR Step 2 returned `state: "unknown"` / Step 3 returned `clicked: false`:** write a blocked `result.json` with `status: "blocked"` and one blocker `{code: "SESSION_REFRESH_FAILED", severity: "blocked", message: "<specific failure from the auto-signin outcome: Touch ID pending, Chrome autofill disabled, saved credential missing, ProContract login error visible, login form shape changed, etc.>"}`. Stop here.

The legacy `PROCONTRACT_SESSION_EXPIRED` blocker is now a fallback: used only when this Session Recovery path has been attempted unsuccessfully AND the operator must restore the session manually. In that case, prefer the `SESSION_REFRESH_FAILED` blocker code (not the legacy code), telling the operator the autonomous recovery already tried and what specifically blocked it.

If the portal shows `Log In`, `Session expired`, captcha, or another auth challenge, run the Session Recovery flow above first. Only emit `PROCONTRACT_SESSION_EXPIRED` or `AGENT_AUTH_CHALLENGE` after auto-signin has failed.

### Step 1: Open the authenticated home page

```bash
agent-browser --cdp ${CDP_PORT} open "https://procontract.due-north.com/SupplierPostLoginHome#"
agent-browser --cdp ${CDP_PORT} wait 2000
```

If redirected to `Log In`, stop and write the blocked result described in Mandatory CDP Discipline.

### Step 2: Go to the Activities area and paginate through ALL pages

On the live home page:

- The `Activities` panel contains `Active`, `Recently added`, and `Last viewed`
- Each row shows `Buyer`, `Title`, and `Added`
- The table has **pagination** at the bottom (page numbers and `Next >`)

Preferred path:

1. Click `Recently added`
2. Collect every activity row on the current page
3. **Check for pagination**: look for `Next >` or numbered page links below the table
4. If more pages exist, click `Next >` (or the next page number) and collect rows from that page too
5. Repeat until there are no more pages or you reach `maxPages` (5)

**You MUST process ALL pages of the Recently added list, not just the first page.** The portal shows ~10 rows per page and there are often 2-3 pages. Stopping at page 1 misses activities.

If the `Recently added` tab does not visibly switch but the grid already contains newly added items, continue with the visible activity list and record that the tab interaction was brittle.

After collecting all activity rows across all pages, merge in
`awaitingAccessTargets`:

1. Normalize each scanned row and each awaiting target to a common key set:
   `advert_id`, `source_notice_id`, `process_id`, `activity_url`, `title`, and
   `discovery_method`
2. Dedupe by `advert_id` when present, otherwise by `source_notice_id`
3. Prefer scanned row metadata for titles and activity URLs, but retain daemon
   ids from `awaitingAccessTargets`
4. For awaiting targets not present in the scan, create a direct target and
   visit it by advert/source id in Step 3

After building this scan/direct union, process each candidate through Steps 3-7
below.

### Step 3: Open the activity dashboard

For each candidate row collected from the Recently added list (across all pages)
or supplied by `awaitingAccessTargets`:

1. Navigate back to the home page if needed:

```bash
agent-browser --cdp ${CDP_PORT} open "https://procontract.due-north.com/SupplierPostLoginHome#"
agent-browser --cdp ${CDP_PORT} wait 2000
```

2. Open the row title link to reach the activity dashboard. If you saved the activity URL or title link from Step 2, navigate directly to it.
3. If this is a direct awaiting target that was not found in the scan, use the
   `advertId` / `sourceNoticeId` to search the activity grid or construct the
   ProContract activity URL when the portal exposes it. Record
   `direct_visit_attempted: true` in `target_evidence[]` whether the visit
   succeeds or fails.
4. Confirm the dashboard heading looks like:

```text
Activity : {title}
```

### Step 4: Choose the correct event

This step is critical.

The activity dashboard can contain multiple event rows, including:

- `Expression of interest accepted`
- A live tender event such as `Not started (Respond by: 15/05/2026)`

Always open the live tender event, not the interest-confirmation event.

On the live page observed during validation:

- `Open` beside `Not started (Respond by: ...)` led to the RFx summary page
- `Open` beside `Expression of interest accepted` is not the correct enrichment path

If a target only shows `Expression of interest accepted`, no live tender event,
or copy equivalent to "not yet responded", classify it as `not_yet_responded`
and do not fail it. The daemon keeps that row in `awaiting_access` and retries
on a later scheduled enrichment run.

If a target cannot be reached because the portal shape changed, the direct
activity URL is unavailable, or the page blocks inspection, classify it in
`failed_targets[]` with the precise failure code and include the same target in
`target_evidence[]`. A failed inspection is not an expiry decision for daemon
awaiting-access state.

### Step 5: Extract the RFx summary

After opening the correct tender event, you should land on an RFx page such as:

```text
https://procontract.due-north.com/RfxResponse/Index?rfxId={rfxId}
```

Extract:

- `rfx_id` from the page URL
- Title
- Buyer
- Full description
- Exact response deadline from the `Deadline & time remaining` section
- Buyer contact details when present

### Step 6: Download every document

The RFx page exposes an `Activity documentation, files & links` section with filename, type, and size.

Download every attachment listed there through the CDP browser.

Example:

```bash
agent-browser --cdp ${CDP_PORT} download @e35 "{download_root}/{notice_id}/107_26-invitation-to-tender-final.pdf"
```

Rules:

- Create a dedicated directory per notice under `download_root`
- Download every file listed, including `pdf`, `docx`, `xlsx`, `zip`, and any other served types
- Preserve the extension
- Record the final saved `local_path`

If a downloaded file is a ZIP archive:

1. Save the ZIP itself
2. Extract it into a sibling directory with shell tooling such as:

```bash
unzip -o "{zip_path}" -d "{extract_dir}"
```

3. Include the ZIP file in `documents_downloaded`
4. Include extracted inner files in `documents_downloaded` too, marking them with `extracted_from_archive`

### Step 7: Build the enriched tender record

Per tender, emit:

```text
notice_id:                 required
process_id:                required if known
advert_id:                 required if known
source_notice_id:          required if known
rfx_id:                    required
rfx_url:                   current RFx page URL
updated_title:             RFx title
updated_description:       RFx description
updated_deadline_at:       ISO 8601 UTC
buyer_name:                buyer organisation
buyer_contact:             object with name/email/organisation when visible
documents_downloaded:      array of downloaded file objects
documents_count:           integer
enriched_at:               ISO 8601 UTC
discovery_method:          scan, direct, or scan_and_direct
direct_visit_attempted:    true when sourced from awaitingAccessTargets
activity_dashboard_url:    dashboard URL when known
```

Per downloaded document:

```text
filename:                  original visible filename
content_type:              infer from extension if the browser does not expose it
size_bytes:                read from the downloaded file on disk when possible
local_path:                absolute local file path
download_url:              optional if discoverable
document_role_hint:        infer from filename, e.g. invitation_to_tender, technical_specification,
                           pricing_schedule, terms_and_conditions, response_document, archive
extracted_from_archive:    true for inner files extracted from ZIP archives
```

### Step 8: Write `result.json`

Write a top-level JSON object:

```json
{
  "cursor": "{\"last_checked_at\":\"2026-04-13T16:00:00Z\",\"pages_checked\":3}",
  "continuation_required": false,
  "enriched_tenders": [],
  "not_yet_responded": [],
  "failed_targets": [],
  "directly_visited": [],
  "target_evidence": [],
  "summary": {
    "total_checked": 0,
    "enriched": 0,
    "not_yet_responded": 0,
    "failed": 0,
    "direct_targets_received": 0,
    "direct_targets_visited": 0,
    "directly_visited": 0,
    "scan_rows_checked": 0
  }
}
```

Set `continuation_required` to `false` when you have checked ALL pages of the Recently added list. Only set it to `true` if you hit the `maxPages` limit before reaching the end.

The `total_checked` count must reflect the deduped scan/direct union checked
across ALL pages, not just one page. `scan_rows_checked` is the raw count of
Recently added rows inspected before union. `direct_targets_received` is the
input `awaitingAccessTargets` length. `direct_targets_visited` is the count of
awaiting targets for which a direct visit was attempted.

If no tender-ready RFx pages are found, return an empty `enriched_tenders` array with a valid summary.

For every supplied awaiting target:

- Emit an `enriched_tenders[]` entry when a live RFx page was opened and
  documents were inspected
- Emit a `not_yet_responded[]` entry when the target is reachable but still has
  no live tender event
- Emit a `failed_targets[]` entry only for inspection failures
- Emit a `directly_visited[]` entry when a direct visit was attempted
- Emit a `target_evidence[]` entry in all cases, including blocked CDP/session
  outcomes

### Step 9: Leave the persistent browser open

Do not close or reset the operator browser. Leave the last navigated ProContract page available for operator inspection.

---

## Canary

Verify that the authenticated home page shows the activity grid, verify that an activity dashboard contains both an interest-confirmation event and a live tender event, and verify that at least one RFx document link can be downloaded to disk.

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
- awaiting_access_targets_received
- scan_rows_checked
- scan_direct_union_count
- direct_targets_visited
- directly_visited_targets
- enriched_targets
- not_yet_responded_targets
- failed_targets
- direct_visit_evidence

Narrative sections:

- Summary
- Steps Taken
- Decisions Made
- Warnings
- Evidence
