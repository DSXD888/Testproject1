---
manifest_version: "1.0.0"
skill_id: "procurement-ingestion/procontract-enrich-tender"
skill_version: "1.1.0"
job_id: "job-deb1ee44ebfa2bb5b80c7cd87feeaa105424eb87f3e0f4115c2c09d9d5c7824f"
session_id: "session-pty-812f54c9e13b02052b675808236b19c330a8e41708247762c6afe2d6bb4b7d1b"
execution_mode: "pty"
started_at: "2026-04-14T20:24:24Z"
completed_at: "2026-04-14T20:38:39Z"
outcome_class: "COMPLETED"
items_processed: 21
items_skipped: 6
items_failed: 0
warnings_count: 2
agent_claimed_pages_visited: 51
agent_claimed_source_url: "https://procontract.due-north.com"
agent_claimed_fingerprint_state: "normal — standard ProContract V3 portal, Version 26.1"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

# Run Manifest — procontract-enrich-tender v1.1.0

## Summary

Enrichment run completed against the ProContract Due North portal. Checked all 27 activities across 3 pages of the "Recently added" list (the full list — no pagination cap hit). Of these, 21 had live tender events (RFx) and were successfully enriched with metadata extraction and document downloads. 6 activities had no live tender events on their dashboards (only interest-registration events or no OpenStep links) and were skipped. Zero failures. 180 documents totaling ~88 MB were downloaded to disk across 21 activity directories.

## Steps Taken

1. **Authentication** — Navigated to `https://procontract.due-north.com/SupplierPostLoginHome#`. Portal redirected to login page. Dismissed cookie consent dialog. Filled credentials (`sales@sorellon.com`) and submitted. Authenticated successfully — redirected to Supplier Post-Login Home.

2. **Activity collection (Step 2)** — Clicked "Recently added" tab. Fetched all 3 pages via the grid API endpoint (`/SupplierPostLoginHome/GridViewActivities?PageName=recentlyadded&page=N`). Collected 27 activities (10 + 10 + 7). Confirmed no further pages exist (no "Next >" on page 3). Full list captured.

3. **Activity dashboard navigation (Step 3)** — For each of the 27 activities, navigated to the activity dashboard at `/SupplierPostLoginHome/ActivityDashboard?projectId={activityId}&callingPage=Home`. Confirmed the dashboard heading pattern `Activity : {title}`.

4. **Tender event selection (Step 4)** — On each dashboard, extracted all `OpenStep("rfxId", "RFX")` onclick handlers to identify live tender events. Selected the live tender event (the one with "Respond by" in the row text) over interest-confirmation events. For 6 activities (509_27, 191_28, 274_26, 16_26, 301_26, 218_26 — all ESPO), no OpenStep links were present on the dashboard — only interest-registration events existed with no live RFx.

5. **RFx extraction (Step 5)** — For each selected event, called `/RfxResponse/VerifyAccount` (POST) and then navigated to `/RfxResponse/Index?rfxId={rfxId}`. Extracted buyer name, description, and deadline from the Activity Summary page. Deadlines were parsed from UK date formats to ISO 8601 UTC where present.

6. **Document download (Step 6)** — On each RFx page, identified document links from the "Activity documentation, files & links" section. Documents were available as `/FileUploads/ViewAttachment?attachmentId=...` links and `/RFxResponse/ViewTerm?TermID=...` links. Downloaded all available files to `{download_root}/{notice_id}/`. For the Tower Hamlets activity (CO6088), one ZIP archive was downloaded and extracted, yielding additional inner files. Total: 180 files across 21 directories.

7. **Result construction (Steps 7-8)** — Built enriched tender records per the result schema. Titles were sourced from the activity grid (the RFx page's title field was unreliable — it showed only "ID:" due to page structure). Wrote `result.json` with 21 enriched tenders.

8. **Browser closed (Step 9)** — Playwright browser instance closed cleanly.

## Decisions Made

1. **Activity URL pattern discovery** — The initial URL pattern `/SupplierActivity/Index?activityId=GUID` returned 404 for all activities. Investigated the `GridActionLinkOnClick` JS function and discovered the correct pattern: `/SupplierPostLoginHome/ActivityDashboard?projectId=GUID&callingPage=Home`. All 27 dashboards loaded successfully after this correction.

2. **Event selection logic** — The `OpenStep` function was discovered via `page.evaluate` to navigate to `/RfxResponse/Index?rfxId=GUID`. Used this to identify and open live tender events, preferring events with "Respond by" in the row text over interest-confirmation events. For YPO School Uniform (activity 27), two RFx events existed (acceptance rounds 133 and 121) — selected the latest (round 133) as the most current.

3. **Title fallback** — The RFx page `Title:` field consistently returned "ID:" (a fragment of the title row's formatting). Corrected this by using the authoritative title from the activity grid's "Recently added" list.

4. **Deadline parsing** — Two UK date formats were encountered: "13th January 2022 at 3:43 PM" and "15/05/2026 12:00". Both were parsed to ISO 8601 UTC. 15 out of 21 tenders had no parseable deadline in the RFx text (the deadline section text was present but in a non-standard narrative format). These were set to `null`.

5. **Skipped activities** — 6 ESPO activities (509_27, 191_28, 274_26, 16_26, 301_26, 218_26) showed no `OpenStep` links on their dashboards, meaning no live tender event was available. These were classified as `not_yet_responded` (interest registered but no RFx round open).

6. **ViewTerm document handling** — Two document types were found: direct FileUploads attachments and ViewTerm pages. ViewTerm pages (terms and conditions) were navigated to in separate tabs, and download links were extracted from within. Where no download link was available, the term content was saved as plain text.

## Warnings

1. **Title extraction unreliable** — The RFx page `Title:` row field consistently returned "ID:" instead of the full title. This appears to be a ProContract V3 rendering issue where the title label and ID badge share the same container. Titles were corrected using the activity grid as source of truth.

2. **Deadline extraction partial** — 15 of 21 enriched tenders have `null` for `updated_deadline_at`. The deadline text on these RFx pages was embedded in narrative sentences rather than structured fields (e.g., "has passed for our own checks to be completed on this"). The parser handled explicit date formats but could not extract dates from unstructured prose.

## Evidence

- **result.json** — Written to `workdir/result.json`. Contains 21 enriched tenders, 0 failures, `continuation_required: false`.
- **Downloaded documents** — 180 files totaling ~88 MB in `artifacts/downloads/procontract/` across 21 subdirectories named by `activityId`.
- **Authentication state** — Saved to `/tmp/pw-auth-state.json` (session cookies for procontract.due-north.com).
- **Page screenshots** — Saved to `/tmp/pw-rfx-{N}.png` for each of the 27 activities.
- **Run metadata** — Saved to `workdir/run-meta.json` with full page visit log, activity list, and timing data.
- **Source structure** — ProContract V3 portal, Version 26.1. Grid endpoint: `/SupplierPostLoginHome/GridViewActivities`. Dashboard endpoint: `/SupplierPostLoginHome/ActivityDashboard`. RFx endpoint: `/RfxResponse/Index`. Documents: `/FileUploads/ViewAttachment` and `/RFxResponse/ViewTerm`.
- **Pages checked** — 3 of 3 total pages (all pages of Recently added list). No continuation required.
- **Activities per page** — Page 1: 10, Page 2: 10, Page 3: 7. Total: 27.
- **Buyers represented** — East Riding of Yorkshire, Yorwaste Ltd, Derbyshire County Council, Bristol City Council, Croydon Council, Bracknell Forest Borough Council, ESPO, ECMWF, Cambridgeshire County Council, London Borough of Bexley, Stockport Metropolitan Borough Council, Worcester City Council, Tower Hamlets, Nottingham City Council, YPO.
