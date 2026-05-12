---
skill_id: procurement-ingestion/procontract-enrich-tender
skill_version: 1.2.1
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
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

## Outputs
- `result.json` containing `enriched_tenders[]`
- Each enriched tender includes the updated deadline and the downloaded document inventory

## Failure Codes
- CDP_CONNECT_FAILED
- PROCONTRACT_SESSION_EXPIRED
- AGENT_AUTH_CHALLENGE
- PAGE_SHAPE_DRIFT
- SOURCE_SOFT_BLOCKED
- AGENT_PARTIAL_FAILURE
- AGENT_STALLED

---

## Playbook

You are a procurement enrichment agent. Discover tender-ready ProContract activities from the authenticated portal, open the RFx summary for each one, download every attachment, and report the enriched data.

### Mandatory CDP Discipline

This skill must attach to the operator's already-authenticated persistent browser. Do not launch a fresh browser profile, do not attempt portal login, do not solve captchas, do not log out, and do not close the operator browser.

Before navigation:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, write a valid blocked `result.json` immediately:

```json
{
  "cursor": "{}",
  "continuation_required": false,
  "enriched_tenders": [],
  "summary": {
    "total_checked": 0,
    "enriched": 0,
    "not_yet_responded": 0,
    "failed": 0
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

Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

If the portal shows `Log In`, `Session expired`, captcha, or another auth challenge, emit a valid blocked result with blocker code `PROCONTRACT_SESSION_EXPIRED` or `AGENT_AUTH_CHALLENGE`. The operator must restore the session manually in the persistent CDP browser.

### Mandatory Bounded-Stall Contract

Before portal work, record `started_at` and an internal deadline that is comfortably below the daemon PTY wall-clock timeout. Use 15 minutes as the maximum internal deadline unless the prompt gives a smaller limit. Do not continue browser navigation past that internal deadline.

Every page transition must have a bounded retry budget:

- Open or return to the authenticated home page: at most 2 attempts, 10 seconds each.
- Switch to `Recently added` or page the activity grid: at most 2 attempts per transition, 8 seconds each.
- Open an activity dashboard: at most 2 attempts, 12 seconds each.
- Open the live tender event from the activity dashboard: at most 2 attempts, 12 seconds each.
- Load the RFx summary and document section: at most 2 attempts, 15 seconds each.
- Download each document: at most 2 attempts, 30 seconds each.

Each activity has a bounded processing budget of 3 minutes, including dashboard navigation, RFx summary extraction, and document download attempts. If that budget is exceeded, stop processing that activity and write the final `result.json` as a blocked contract.

If a required page shape, live tender event, RFx summary, or document section is not reachable within budget, write a valid blocked `result.json` with:

- `status`: `blocked`
- `continuation_required`: `false`
- `enriched_tenders`: every tender fully enriched before the blocker, or `[]`
- `summary.total_checked`: activities attempted before blocking
- `summary.enriched`: count of fully enriched tenders in `enriched_tenders`
- `summary.failed`: at least `1`
- `blockers[]`: one or more entries with `code`, `message`, `severity`, and current `url` and `title` when available
- `cursor`: a JSON string containing at least `blocked_at`, `started_at`, and the last page/activity reached when known

Use blocker codes from the Failure Codes list. For page shape drift use `PAGE_SHAPE_DRIFT`; for auth/session problems use `AGENT_AUTH_CHALLENGE`; for source throttling or portal denial use `SOURCE_SOFT_BLOCKED`; for no progress within the bounded navigation/processing budget use `AGENT_STALLED`.

Do not write partial `result.json` checkpoints during normal successful processing. The daemon supervisor terminates the PTY child shortly after it sees a valid `result.json`. Only write `result.json` when the run is complete or blocked.

### Step 1: Open the authenticated home page

```bash
agent-browser --cdp ${CDP_PORT} open "https://procontract.due-north.com/SupplierPostLoginHome#"
agent-browser --cdp ${CDP_PORT} wait 2000
```

If redirected to `Log In`, stop and write the blocked result described in Mandatory CDP Discipline. If the authenticated home page or expected activity grid cannot be confirmed within the bounded retry budget, write a blocked result with `PAGE_SHAPE_DRIFT` or `AGENT_STALLED` and include the current URL/title.

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

If the activity grid, pagination controls, or visible activity rows cannot be read within budget, write a blocked result with `PAGE_SHAPE_DRIFT` or `AGENT_STALLED`. Do not keep clicking or waiting until the daemon timeout.

After collecting all activity rows across all pages, process each one through Steps 3-7 below.

### Step 3: Open the activity dashboard

For each candidate row collected from the Recently added list (across all pages):

1. Navigate back to the home page if needed:

```bash
agent-browser --cdp ${CDP_PORT} open "https://procontract.due-north.com/SupplierPostLoginHome#"
agent-browser --cdp ${CDP_PORT} wait 2000
```

2. Open the row title link to reach the activity dashboard. If you saved the activity URL or title link from Step 2, navigate directly to it.
3. Confirm the dashboard heading looks like:

```text
Activity : {title}
```

If the dashboard heading or activity event list is not reachable within the bounded retry budget, write a blocked result with `AGENT_STALLED` and include current URL/title plus the candidate title.

### Step 4: Choose the correct event

This step is critical.

The activity dashboard can contain multiple event rows, including:

- `Expression of interest accepted`
- A live tender event such as `Not started (Respond by: 15/05/2026)`

Always open the live tender event, not the interest-confirmation event.

On the live page observed during validation:

- `Open` beside `Not started (Respond by: ...)` led to the RFx summary page
- `Open` beside `Expression of interest accepted` is not the correct enrichment path

If no live tender event is visible, or opening it fails within the bounded retry budget, write a blocked result with `PAGE_SHAPE_DRIFT` or `AGENT_STALLED`. If some tenders were enriched before this blocker, keep them in `enriched_tenders[]` and increment `summary.failed`.

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

If the RFx page loads but the required summary fields or `Activity documentation, files & links` section cannot be located within the bounded retry budget, write a blocked result with `PAGE_SHAPE_DRIFT`. Include the current RFx URL/title and the missing section in the blocker message.

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

If a document download repeatedly fails, record the failed filename in a blocker and write a blocked result with `AGENT_STALLED` or `SOURCE_SOFT_BLOCKED`. Preserve any fully downloaded documents and enriched tenders completed before the blocker.

### Step 7: Build the enriched tender record

Per tender, emit:

```text
notice_id:                 required
process_id:                required if known
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
  "summary": {
    "total_checked": 0,
    "enriched": 0,
    "not_yet_responded": 0,
    "failed": 0
  }
}
```

Set `continuation_required` to `false` when you have checked ALL pages of the Recently added list. Only set it to `true` if you hit the `maxPages` limit before reaching the end.

The `total_checked` count must reflect activities checked across ALL pages, not just one page.

If no tender-ready RFx pages are found, return an empty `enriched_tenders` array with a valid summary.

For a blocked run, include the blocked fields described in Mandatory Bounded-Stall Contract. Example:

```json
{
  "cursor": "{\"blocked_at\":\"2026-05-12T22:00:00Z\",\"started_at\":\"2026-05-12T21:50:00Z\",\"last_activity\":\"Activity title\"}",
  "continuation_required": false,
  "enriched_tenders": [],
  "summary": {
    "total_checked": 1,
    "enriched": 0,
    "not_yet_responded": 0,
    "failed": 1
  },
  "status": "blocked",
  "blockers": [
    {
      "code": "AGENT_STALLED",
      "message": "Timed out opening the live RFx event from the activity dashboard.",
      "severity": "blocked",
      "url": "https://procontract.due-north.com/...",
      "title": "Activity dashboard"
    }
  ]
}
```

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

Narrative sections:

- Summary
- Steps Taken
- Decisions Made
- Warnings
- Evidence
