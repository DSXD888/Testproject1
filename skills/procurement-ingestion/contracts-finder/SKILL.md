---
skill_id: procurement-ingestion/contracts-finder
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 2.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
requires_cdp: true
cdp_default_port: 9222
maxPages: 10
maxScrollCycles: 2
---

# procurement-ingestion/contracts-finder

## Use when
Use this skill when the daemon needs incremental Contracts Finder notice retrieval for the source-batched ingestion loop using the official OCDS API and cursor-aware continuation semantics.

## Do not use when
Do not use this skill for Find a Tender retrieval, manifest synthesis, supplier enrichment, relation resolution, portal submission flows, or any local-only deterministic transformation.

## Inputs
- Cursor input from `source_cursors` with the current safe watermark.
- Optional `publishedFrom`, `publishedTo`, `stages`, `cursor`, and `limit` job parameters.
- A time budget so the skill can stop at a safe continuation boundary before rate-limit pressure or session drift.

## Outputs
- A batch of structured Contracts Finder notice records matching `result.schema.json`.
- Checkpoints written at committed batch boundaries.
- `continuation_required` when safe remaining work exceeds the current session budget.

## Non-goals
- Private bidder login or downstream submission-portal work.
- Cross-notice relation resolution or process merge decisions.
- Commercial enrichment, manifest rendering, or margin calculation.

## Failure Codes
- SOURCE_CHANGED
- AGENT_PROTOCOL_MISMATCH
- SOURCE_SOFT_BLOCKED
- SOURCE_CAPTCHA_BLOCKED
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- AGENT_STALLED

## Resume Strategy
Resume with a cursor backtrack window and a small overlap on the publication boundary. Re-fetch a short overlap after each committed checkpoint so an in-flight cursor move never skips a notice when Contracts Finder mutates or reorders recent releases.

## Pagination Bounds
- `maxPages`: 10
- `maxScrollCycles`: 2

---

## Playbook

You are a procurement data ingestion agent. Your job is to fetch structured contract notice data from the UK Contracts Finder service and return it as a result contract.

### Mandatory CDP Discipline

Use the operator persistent CDP browser for this public source so the run stays visible on the shared browser surface and preserves locale/session state. Do not launch a fresh browser profile and do not close the operator browser.

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

If CDP is unreachable, write a valid blocked `result.json` with `continuation_required: true`, an empty notice batch, and blocker code `CDP_CONNECT_FAILED`. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

### Step 1: Determine Parameters

Read the job parameters injected by the daemon:
- `publishedFrom` (ISO date, e.g. `2026-04-01`) — fetch notices published on or after this date. If absent, default to 7 days ago.
- `publishedTo` (ISO date) — optional upper bound. If absent, use today.
- `limit` (integer) — maximum number of notices to return. Default: 50. Maximum: 100.
- `stages` (comma-separated) — filter by procurement stage. Default: all stages.
- `cursor` (JSON string) — previous cursor value from `source_cursors`. Contains `cursor_after` (last page fetched) and `cursor_before` (publication date watermark).

### Step 2: Fetch Data via OCDS API Using CDP

The primary data source is the Contracts Finder OCDS API. Use the CDP browser to navigate and extract.

#### 2a. Open the OCDS API endpoint

```
agent-browser --cdp ${CDP_PORT} open "https://www.contractsfinder.service.gov.uk/Published/Notices/OCDS/Search?publishedFrom={publishedFrom}&publishedTo={publishedTo}&size=100&page=1"
```

Wait for the page to load:
```
agent-browser --cdp ${CDP_PORT} wait
```

#### 2b. Extract the JSON response

Get the full page text (the API returns raw JSON):
```
agent-browser --cdp ${CDP_PORT} get text
```

Parse the JSON response. It will contain:
- `hitsCount` — total number of matching notices
- `maxPage` — last page number available
- `releases[]` — array of OCDS release objects

#### 2c. Handle pagination

If `maxPage > 1` and you haven't reached the `limit`, fetch subsequent pages:
```
agent-browser --cdp ${CDP_PORT} open "https://www.contractsfinder.service.gov.uk/Published/Notices/OCDS/Search?publishedFrom={publishedFrom}&publishedTo={publishedTo}&size=100&page={nextPage}"
agent-browser --cdp ${CDP_PORT} wait
agent-browser --cdp ${CDP_PORT} get text
```

Continue until you've either:
- Reached `maxPage`
- Collected `limit` notices
- Hit `maxPages` (10) pagination bound

#### 2d. Handle rate limits

If you receive an HTTP 403 response or the page text contains an error indicating rate limiting:
1. Wait at least 5 minutes before retrying
2. Record a warning in the run manifest
3. If the rate limit persists after 2 retries, stop with `continuation_required: true`

### Step 3: Verify with HTML Site (Optional Enrichment)

For verification, you may spot-check a few notices against the HTML search:
```
agent-browser --cdp ${CDP_PORT} open "https://www.contractsfinder.service.gov.uk/Search/Results"
agent-browser --cdp ${CDP_PORT} wait
agent-browser --cdp ${CDP_PORT} snapshot -i
```

This step is optional and should only be used to verify data quality, not as a primary extraction path.

### Step 4: Transform Each Release into a Notice Record

For each OCDS release in the response, extract and map fields as follows:

```
notice_id:                    "cf-" + release.id (or release.ocid if id is absent)
source_notice_id:             release.id (the original OCDS release ID)
source_url:                   "https://www.contractsfinder.service.gov.uk/notice/" + extract UUID from release.id
ocid:                         release.ocid
title:                        release.tender.title
summary:                      release.tender.description (truncate to 2000 chars if longer)
buyer_name:                   release.buyer.name OR first party with role "buyer"
procurement_stage:            map from release.tag[] and release.tender.status:
                              - "tender" + status "active" -> "open"
                              - "award" -> "awarded"
                              - "planning" -> "future_opportunity"
                              - "tenderAmendment" -> "open"
                              - default -> release.tender.status
main_category:                release.tender.mainProcurementCategory (works/goods/services)
deadline_at:                  release.tender.tenderPeriod.endDate (ISO 8601)
published_at:                 release.tender.datePublished OR release.date
source_updated_at:            release.date
estimated_value_low_pence:    release.tender.minValue.amount * 100 (convert to pence)
estimated_value_high_pence:   release.tender.value.amount * 100 (convert to pence)
estimated_value_currency:     release.tender.value.currency (expect "GBP")
access_route:                 "public_web"
procurement_method:           release.tender.procurementMethod
raw_release_json:             JSON.stringify(release) — the full raw OCDS release
```

#### Lots extraction
If `release.tender.lots` exists (Contracts Finder often uses items, not lots):
```
lot_id:                       "cf-lot-" + lot.id
source_lot_id:                lot.id
title:                        lot.title
description:                  lot.description
estimated_value_pence:        lot.value.amount * 100
deadline_at:                  lot.tenderPeriod.endDate
```

If no explicit lots exist but `release.tender.items[]` exists, treat the entire tender as a single implicit lot.

#### Buyer extraction
```
buyer.name:                   release.buyer.name OR buyer party name
buyer.identifier_scheme:      buyer party identifier.scheme (e.g. "GB-GOR")
buyer.identifier_id:          buyer party identifier.id
buyer.contact_name:           buyer party contactPoint.name
buyer.contact_email:          buyer party contactPoint.email
buyer.contact_telephone:      buyer party contactPoint.telephone
buyer.address_locality:       buyer party address.locality
buyer.address_region:         buyer party address.region
buyer.address_postal_code:    buyer party address.postalCode
```

#### Documents extraction
From `release.tender.documents[]` and `release.awards[].documents[]`:
```
id:                           document.id
document_type:                document.documentType
url:                          document.url
description:                  document.description
```

#### CPV codes extraction
```
cpv_codes:                    [release.tender.classification.id] +
                              release.tender.additionalClassifications[].id
                              (filter to scheme == "CPV")
```

### Step 5: Build and Write the Result Contract

Assemble the result JSON matching `result.schema.json` and **write it to `result.json` in the current working directory**.

```json
{
  "cursor": "{\"page\": <last_page_fetched>, \"publishedFrom\": \"<publishedFrom>\", \"publishedTo\": \"<latest_release_date>\"}",
  "continuation_required": <true if more pages remain beyond limit>,
  "notices": [ ... array of notice objects ... ]
}
```

Write this JSON to the file `result.json` in the current directory. The daemon will read this file after the session completes.

### Step 6: Leave Browser Open

Do not close or reset the operator browser. Leave the last page visible for inspection.

---

## Canary
Fetch the five most recent notices from the official Contracts Finder OCDS search endpoint, verify release-package shape, verify `language = en-GB`, verify `GBP` contract-value expectations when value data is present, and treat HTTP `403` as a documented rate-limit condition that requires at least a five-minute backoff before retry.

## Run Manifest Instructions

You must produce a run manifest that the daemon can finalize into the durable `run-manifest.md` artifact.

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

Major claims must be anchored to observable evidence such as result-contract fields, daemon appendix fields, or artifact refs. Do not rely on operator memory or hidden context.
