---
manifest_version: "1.0.0"
skill_id: "procurement-ingestion/procontract-enrich-tender"
skill_version: "1.1.0"
job_id: "job-bab68bfca801462cc5362a78e27ad63be43be8704e5f8e9b79f5dd6762d7fc70"
session_id: "session-pty-7e0c8c10fd5b38655a7fd7856be0a5d1784347e7a6c33c94bc41b158dfbee196"
execution_mode: "pty"
started_at: "2026-04-15T11:53:00Z"
completed_at: "2026-04-15T13:02:45Z"
outcome_class: "success"
items_processed: 11
items_skipped: 16
items_failed: 0
warnings_count: 2
agent_claimed_pages_visited: 32
agent_claimed_source_url: "https://procontract.due-north.com/SupplierPostLoginHome"
agent_claimed_fingerprint_state: "normal"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

# Run Manifest

## Summary

Enriched 11 active tenders from ProContract portal across 27 total activities checked on 3 pages of the Recently added list. Downloaded 66 primary documents (plus extracted archive contents) totalling approximately 45 MB across 11 notice directories. All 3 pages of pagination were visited. Session required 3 re-authentications due to idle timeout during processing.

## Steps Taken

1. **Authenticated** to ProContract portal at `https://procontract.due-north.com` using supplied credentials. Accepted cookie consent dialog.

2. **Navigated to Recently added tab** on the home page Activities panel. Confirmed pagination shows 3 pages (10 + 10 + 7 = 27 activities).

3. **Collected all 27 activity GUIDs** across 3 pages by:
   - Page 1: extracted 10 activity rows via DOM table (refs from snapshot)
   - Page 2: used AJAX endpoint `/SupplierPostLoginHome/GridViewActivities?page=2` to fetch and parse HTML
   - Page 3: same AJAX approach for final 7 rows

4. **Processed each activity dashboard** by navigating to `ActivityDashboard?projectId={guid}`:
   - Identified live tender events ("Not started (Respond by: ...)" labels) vs "Expression of interest accepted" events
   - Clicked the correct "Open" link for each live tender
   - Some activities had the live tender as the first event, others as the second (behind an Advert page)

5. **Extracted RFx metadata** from each RFx page:
   - rfx_id from URL parameter
   - notice_id (ID field), buyer, title, description, deadline from page content
   - Document count from "Activity documentation, files & links" section

6. **Downloaded all documents** using JavaScript `fetch()` with `credentials: 'include'` to maintain session auth, base64-encoded via FileReader, then decoded and saved to disk. This approach was necessary because `agent-browser download` timed out on ViewAttachment URLs.

7. **Extracted ZIP archives** for notices 783700 (3 ZIPs), 783101 (1 ZIP), and 774481 (2 ZIPs) using `unzip` shell command.

8. **Wrote result.json** with all 11 enriched tender records and summary counts.

9. **Closed the browser** session.

## Decisions Made

1. **Skipped 16 activities** that were either expired (deadline in the past), had no respond-by date (likely informational/closed), or were marked "DO NOT USE".

2. **Download method pivot**: Initial attempts using `agent-browser download` command failed on ViewAttachment URLs (hung indefinitely). Pivoted to JavaScript fetch + base64 encode + shell decode approach, which worked reliably for all files.

3. **Activity #14 (6278 Building Services)**: The first "Open" link led to an Advert/View page, not an RFx page. Used the second "Open" link which correctly led to the RFx summary with 5 documents.

4. **Activity #16 (Stockport School)**: Same pattern as #14 - first Open was Advert page, second Open was the RFx with 8 documents.

5. **Activity #27 (YPO School Uniform)**: Deadline 14/04/2026 was borderline (yesterday). Excluded from enrichment as the deadline had passed.

6. **Activity #6 (Bracknell EHC)**: Despite having a future deadline (26/03/2027), the Open link led to an Advert page with no documents and no RFx page available. Recorded as checked but not enriched.

## Warnings

1. **Session timeout**: The ProContract session expired 3 times during processing, requiring re-authentication. This is expected for a run processing 27 activities serially. The session appears to have a ~5-10 minute idle timeout.

2. **ZIP extraction partial failure**: One ZIP file for notice 783700 contained a filename with illegal byte sequences (special characters in "Cleaning Services" lot name). The file was skipped during extraction but the ZIP itself was preserved intact.

## Evidence

- **result.json**: Contains 11 enriched tender records with full metadata and document inventories. `result.json:summary.total_checked` = 27, `result.json:summary.enriched` = 11.
- **Download directories**: 11 directories under `artifacts/downloads/procontract/` named by notice_id (621558, 640042, 774481, 782604, 782746, 783101, 783238, 783700, 783707, 783996, 784078).
- **Total primary documents downloaded**: 66 files across all notices.
- **Pagination coverage**: All 3 pages of Recently added list visited (cursor reflects `pages_checked: 3`).
- **All deadlines verified**: Every enriched tender has a deadline after 2026-04-15 (the execution date).
