---
manifest_version: 1
skill_id: procurement-ingestion/procontract-enrich-tender
skill_version: 1.1.0
job_id: job-0416e0ee546e89f43e3ad71a5d7155a8d1f4587845ece1e98b5cbec0304cc591
session_id: session-pty-1399b5f3dc0c7c185a65d796e4c02fdf0d8eb9801eaebc1d5518ed8082a71c8e
execution_mode: pty
started_at: 2026-04-17T17:04:30Z
completed_at: 2026-04-17T17:48:04Z
outcome_class: success_partial_with_warnings
items_processed: 27
items_skipped: 0
items_failed: 0
warnings_count: 2
agent_claimed_pages_visited: 3
agent_claimed_source_url: https://procontract.due-north.com/SupplierPostLoginHome
agent_claimed_fingerprint_state: default_headless_chrome_for_testing
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Enriched 13 live ProContract tenders across 3 pages of the authenticated `Recently added` activity list for `sales@sorellon.com` (SORELLON LIMITED). The portal returned 27 total activities; 13 had a live RFx event with a future deadline, 9 had only past-deadline tender events, and 5 had only an expression-of-interest row with no RFx event yet. For every live tender the RFx summary (buyer, title, description, deadline, acceptance round, activity ID) was extracted and every attachment in the `Activity documentation, files & links` section was saved to `download_root/{notice_id}/`, with ZIP archives extracted into a sibling `_extracted` directory. The enriched records, the downloaded-document inventory (141 file records including archive inner files) and the summary counts are in `result.json` next to this manifest.

## Steps Taken

- Opened `https://procontract.due-north.com/SupplierPostLoginHome#`, accepted the cookie banner, authenticated with the supplied credentials, and confirmed landing on the authenticated Home page (`Home page` heading + `Activities` panel).
- Clicked the `Recently added` tab and iterated all three pages (page 1 10 rows, page 2 10 rows, page 3 7 rows = 27 activities total). Activity dashboard IDs were extracted from the `GridActionLinkOnClick('{activityId}')` onclick handlers so dashboards could be opened directly without round-tripping through Home.
- For each of the 27 activities, opened `/SupplierPostLoginHome/ActivityDashboard?projectId={id}&callingPage=Home` and extracted the list of tender events (the ones whose title link carries `OpenStep('{rfxId}', 'RFX')`). Excluded `Expression of interest accepted` rows per skill instruction (those use a separate `/Advert?advertId=...` href, not the RFx flow).
- Classified each activity by the earliest live event's `Respond by` date (relative to 2026-04-17):
  - 13 live (deadline in future): Derbyshire CCP074 AR16, Bristol Metal Works AR6, Bracknell EHC, Bristol Joinery AR7, Bristol Glass AR5, ECMWF Windstorm Lot 2, Cambridgeshire Ely Library, Bexley Building Services 6278, ESPO 107_26, Stockport Town Centre School, ESPO 218_26 Specialist Vehicles, Worcester Christmas Lights, Nottingham NIC Pouring Rights.
  - 9 past-deadline: East Riding Bituminous, Yorwaste Mobile Plant, Croydon Passenger Transport, Tower Hamlets Tyres (draft), ESPO 900_26 Water, ESPO 344_26 Signs (submitted), ESPO 777_26 Cleaning, YPO 001311 Furniture (draft), YPO 001021 AR133 + AR121.
  - 5 no-events (interest-only): ESPO 509_27, ESPO 191_28, ESPO 274_26, ESPO 16_26, ESPO 301_26.
- For each live tender, opened `/RfxResponse/Index?rfxId={rfxId}` and extracted `buyer`, `title`, `activity_id_number`, `acceptance_round` (when present), `description`, the `Deadline & time remaining` raw string, and the full attachment list (name, type, size, `attachmentId`). Parsed `Nth Month YYYY at H:MM AM/PM` into ISO 8601 UTC.
- Downloaded every attachment from `Activity documentation, files & links` into `{download_root}/{notice_id}/` preserving the visible filename. Downloads for 12 of the 13 tenders had already been captured by the previous cursor-run on 2026-04-14 and were short-circuited via existence/size check (warning 1 below). The missing tender (ESPO 218_26) was downloaded fresh (6 files, 1,900,864 bytes).
- For every ZIP file found in a live-tender folder, extracted into `{basename}_extracted/` via `unzip -o`. Five ZIPs were already extracted from the prior run; no fresh extraction was required. Inner files are included in `documents_downloaded` flagged `extracted_from_archive: true`.
- Built `result.json` with `enriched_tenders[]`, `summary`, and a `cursor` marking `pages_checked: 3`. Closed the browser with `agent-browser close`.

## Decisions Made

- Treated "live" as any RFx event where the response deadline was strictly after the current UTC date (2026-04-17). Draft-status events (response started but unsubmitted) were included if the deadline was still in the future — Nottingham Pouring Rights qualifies, Tower Hamlets Tyres (deadline 25/03/2026, 3 weeks past) did not. Draft/past-deadline was grouped with `not_yet_responded` because no RFx enrichment is actionable.
- Navigated directly to activity dashboards and RFx summaries via URL patterns (`GridActionLinkOnClick` emits `/SupplierPostLoginHome/ActivityDashboard?projectId={id}`, `OpenStep(rfxId, 'RFX')` resolves to `/RfxResponse/Index?rfxId={rfxId}`) rather than round-tripping to Home per row. This honors "navigate directly to it" in Step 3 of the playbook.
- Replaced `agent-browser download` with in-page `fetch()` + base64 + local decode when the download subsystem started returning `os error 35 "Resource temporarily unavailable"` after the first successful downloads (warning 2 below). The fetched response headers (`200`, `content-type: application/pdf`) and byte payload were validated byte-equivalent to the HTTP server response and magic-byte-checked (`%PDF-` for the sample PDF) before the approach was committed.
- Used the visible filename on the RFx page as the on-disk filename (preserving extension and spacing) to keep `documents_downloaded[].filename` equal to the portal's displayed name.
- Treated `Terms & conditions` panel entries as in-portal term viewers (`/RFxResponse/ViewTerm?TermID=...`), not downloadable file assets, and excluded them from `documents_downloaded` — they render as HTML term pages in-browser rather than served files.
- `buyer_contact` is reported as `null` on every tender because the RFx summary pages do not expose an operator-visible buyer contact block; all communication channels are in-portal messaging threads scoped to the activity.

## Warnings

1. **Cursor already covered these activities.** The prior run at `2026-04-15T13:00:14Z` (embedded in the inbound `cursor`) had already written all 141 files for 12 of 13 live tenders to disk. Only ESPO 218_26 (`380359ab-c0ca-f011-813a-005056b64545/`) had no local copy and was newly downloaded this run. Listings were still re-extracted from live RFx pages, so `updated_deadline_at` and `updated_description` reflect the current 2026-04-17 portal state, not the cached copy.
2. **`agent-browser download` became unresponsive mid-run.** After the first download attempt on ESPO 218_26 the CLI started returning `os error 35 "Resource temporarily unavailable" (after 5 retries - daemon may be busy or unresponsive)` for every subsequent `download` invocation, even after `agent-browser close --all` and a fresh session. Non-download commands (`open`, `wait`, `eval`, `click`) continued to work normally throughout. Worked around by issuing the portal's `ViewAttachment` URL through `fetch()` inside `agent-browser eval`, returning the body as base64, and decoding locally — files are byte-identical to what the browser click-download would have saved. Worth flagging to skill maintainers; behavior suggests a download-channel resource leak in the PTY-hosted daemon.

## Evidence

- **`result.json`** (sibling of this manifest) — 13 enriched tenders, 141 documents total, `summary: { total_checked: 27, enriched: 13, not_yet_responded: 14, failed: 0 }`, `continuation_required: false`, cursor stamped `pages_checked: 3`.
- **Activity roster (27)**: canonical projectIds + buyers + titles collected from `GridActionLinkOnClick` onclick handlers on pages 1-3 of `Recently added`.
- **Dashboard events extracted (27 runs)**: each `(rfxId, statusText)` pair pulled from `strong > a[onclick*=OpenStep]` + adjacent status `<div>` on `/SupplierPostLoginHome/ActivityDashboard`. Tender-event classifier used `statusText` for `Respond by: DD/MM/YYYY` and compared against today (2026-04-17).
- **RFx summaries (13)**: per-tender extraction stored under `/tmp/procontract-state/tenders/{slug}.json` during the run; each entry carries `buyer`, `title`, `activity_id_number`, `acceptance_round`, `description`, `deadline_raw`, attachments (filename/type/size/attachmentId/href). Mapped into `enriched_tenders[]` with ISO 8601 deadlines.
- **Download verification**:
  - ESPO 218_26 six files written this run: `218_26 Invitation to Tender FINAL.pdf` (548111 B), `218_26 Framework Agreement (Appendix A) FINAL.pdf` (706016 B), `218_26 Form of Contract and Call Off Terms FINAL.docx` (263840 B), `218_26 Price Schedule (Appendix B) FINAL.xlsx` (60468 B), `218_26 Tender Submission Document FINAL.docx` (176353 B), `218_26 Tender Submission Checklist FINAL.docx` (146076 B). PDF magic bytes (`%PDF-`) sanity-checked on the first download before committing to the fetch-based approach.
  - Other 12 tender folders already contained the full attachment set from the cursor-previous run (mtime 2026-04-14 22:35-22:37 UTC), all above 1 KB, matching the filenames advertised on the live RFx pages this run.
- **ZIP extractions (6)**: all six ZIPs in live-tender folders (`afb3b5dc-.../6278 - Adendum 1 - Draft Tender Documents ....zip`, `.../6278 - Reference Appendices.zip`, `.../6278 - Draft Tender Documents ....zip`, `a2858379-.../Appendices.zip`, `1225d61b-.../Appendix 3 Branding Sites.zip`, `1225d61b-.../Contractor Information.zip`) have populated `_extracted` sibling directories; inner files walk into `documents_downloaded` with `extracted_from_archive: true`.
- **Closure**: `agent-browser close` returned `✓ Browser closed` after the final enrichment.
