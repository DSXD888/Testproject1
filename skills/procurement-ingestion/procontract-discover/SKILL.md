---
skill_id: procurement-ingestion/procontract-discover
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
source_structure_version: procontract-2026-04-13
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

### Credentials

When a detail page redirects to login, authenticate with:

- Username: `sales@sorellon.com`
- Password: `9928fJG3!5nk6nc`

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
agent-browser open "https://procontract.due-north.com/Opportunities/Index?tabName=opportunities&resetFilter=True"
agent-browser wait 2000
agent-browser snapshot -i
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
agent-browser open "https://procontract.due-north.com/Advert?advertId={advertId}"
```

2. If the browser lands on `Log In`, log in with the provided credentials:

```bash
agent-browser fill "User Name" "sales@sorellon.com"
agent-browser fill "Password" "9928fJG3!5nk6nc"
agent-browser click "Continue"
agent-browser wait 2000
```

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

### Step 9: Close the browser

```bash
agent-browser close
```

---

## Canary

Fetch up to three current ProContract opportunities, verify that the public listing is reachable, verify that advert links expose an `advertId`, and verify that at least one authenticated detail page can be opened after login.

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
