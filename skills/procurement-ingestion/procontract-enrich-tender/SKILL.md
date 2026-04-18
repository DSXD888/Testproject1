---
skill_id: procurement-ingestion/procontract-enrich-tender
skill_version: 1.1.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
source_structure_version: procontract-enrich-2026-04-13
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
- AGENT_AUTH_CHALLENGE
- PAGE_SHAPE_DRIFT
- SOURCE_SOFT_BLOCKED
- AGENT_PARTIAL_FAILURE
- AGENT_STALLED

---

## Playbook

You are a procurement enrichment agent. Discover tender-ready ProContract activities from the authenticated portal, open the RFx summary for each one, download every attachment, and report the enriched data.

### Credentials

- Username: `sales@sorellon.com`
- Password: `9928fJG3!5nk6nc`

### Step 1: Open the authenticated home page

```bash
agent-browser open "https://procontract.due-north.com/SupplierPostLoginHome#"
agent-browser wait 2000
```

If redirected to `Log In`, authenticate with the supplied credentials.

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

After collecting all activity rows across all pages, process each one through Steps 3-7 below.

### Step 3: Open the activity dashboard

For each candidate row collected from the Recently added list (across all pages):

1. Navigate back to the home page if needed:

```bash
agent-browser open "https://procontract.due-north.com/SupplierPostLoginHome#"
agent-browser wait 2000
```

2. Open the row title link to reach the activity dashboard. If you saved the activity URL or title link from Step 2, navigate directly to it.
3. Confirm the dashboard heading looks like:

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

Download every attachment listed there using `agent-browser download`.

Example:

```bash
agent-browser download @e35 "{download_root}/{notice_id}/107_26-invitation-to-tender-final.pdf"
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

### Step 9: Close the browser

```bash
agent-browser close
```

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
