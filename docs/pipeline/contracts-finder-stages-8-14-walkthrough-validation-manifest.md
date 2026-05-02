# Contracts Finder Stages 8-14 Manual Walkthrough Validation Manifest

Date: 2026-05-02

## Summary

This manifest records a manual proof that a Contracts Finder process can move from completed early pipeline state into the L1-L3 enrichment/decision stages without a Contracts Finder-specific enrichment skill.

Target process:

- Process ID: `proc-ocds-b5fd17-5399000a-8bde-4a4b-aa59-b025c4f5881b`
- Notice ID: `cf-f9abdf99-fe65-40de-83f4-e8c19d3b0f5f-895846`
- Title: `#10 Frances Street Developmnt`
- Source: `contracts-finder`
- Stage in source data: `open`
- Value: GBP 960,000 from `normalized_payload`; `contract_notices.estimated_value_gbp_ex_vat_pence` is null for all open CF notices in this dataset.
- Documents: 40 entries in `tender.documents[]`, including direct `application/pdf` attachment URLs.
- Notice qualification: rejected as `works` at Stage 7 (`CPV division 45`), but the downloaded building-regs PDF contained extractable physical material/product candidates.

The final outcome is a Stage 13 `clarify` manifest and a queued Stage 14 operator intent with `idempotency_key='cf-walkthrough-001'`. Stage 9 produced search attempts but no GBP supplier quotes because Amazon UK rendered EUR prices; therefore Stage 10-11 did not have a valid pricing/LCE input. Stage 12 recorded the no-pricing path.

## Pre-Flight

| Check | Result |
|---|---|
| Branch | `main` |
| HEAD claim | The handoff claimed `4ba771d`; the repo did not match that claim during this session. Initial observed HEAD was `e71f84e`; after concurrent project activity, final observed HEAD before manifest write was `f2fc0ec`. |
| CF notice count | `contracts-finder = 100` |
| ProContract notice count | `procontract = 140` |
| PDF extraction skill layout | `skills/pdf-extraction/` contains `base`, `bill-of-quantities`, `pricing-schedule`, `tender-specification`. There is no `cascade`, `semantic`, or `structural` directory in the current tree. |
| ProContract reference skill | `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` is present and uses the existing download-and-enrich pattern. |
| CF document URL shape | CF documents include the notice landing page and attachment URLs under `https://www.contractsfinder.service.gov.uk/Notice/...`. |
| Agent/browser tooling | `agent-browser`, `codex`, and `claude` are available. |
| Docling | `docling 2.88.0` is available in `$HOME/.docling-venv`. |

## Target Selection

The exact D1 query from the handoff returned no rows because the 100 CF notices use `procurement_stage='open'` for live tenders, not `tender`, and the table-level value column is null for all open CF notices.

Dataset shape:

| Procurement stage | Notices | With documents | With table value |
|---|---:|---:|---:|
| `awarded` | 93 | 0 | 0 |
| `open` | 7 | 7 | 0 |

The live CF notices with documents included:

| Notice | Payload value | Docs | Stage 7 result |
|---|---:|---:|---|
| `#10 Frances Street Developmnt` | 960000 | 40 | works/reject |
| `Supply Chain Notice: ... Replacement of Chiller` | 250000 | 3 | goods/pass |
| `Radford House - External Facade Works` | 175000 | 4 | works/reject |
| `GB-London: Commercial Events and Programme Production Partner` | 100000 | 2 | services/reject |
| `A66 Northern Trans-Pennine (NTP) - Temporary Vehicle Restraint System` | 50000 | 1 | goods/pass |

I selected `#10 Frances Street Developmnt` because it had by far the richest document set and value range, including multiple direct PDFs with building-regs and material details. The chiller notice was a better Stage 7 goods fit but its documents were HTML/XLSX/DOCX, not the PDF path this walkthrough was asked to validate.

This is a meaningful source-shape finding: no CF notice in this 100-row dataset satisfied every D1 criterion simultaneously: open/tender-equivalent, value range, Stage 7 goods-qualified, and rich PDF documents.

## Document Acquisition

Path used: Path B, manual download.

Path A was not used for the actual download because `procontract-enrich-tender` is portal-specific and requires ProContract enrichment state. The current CF notice already exposed attachment URLs in normalized OCDS data, so the narrowest manual proof was to download the CF attachment URLs directly with a browser-compatible request shape.

Command pattern used:

```bash
curl -L \
  -A "Mozilla/5.0" \
  -e "https://www.contractsfinder.service.gov.uk/Notice/f9abdf99-fe65-40de-83f4-e8c19d3b0f5f" \
  -b cf-cookies.txt -c cf-cookies.txt \
  -o artifacts/downloads/contracts-finder/<process_id>/<file>.downloaded.pdf \
  "https://www.contractsfinder.service.gov.uk/Notice/Attachment/<attachment_uuid>"
```

Files retained under `artifacts/downloads/contracts-finder/proc-ocds-b5fd17-5399000a-8bde-4a4b-aa59-b025c4f5881b/`:

| File | Size | SHA-256 | Outcome |
|---|---:|---|---|
| `Frances-Street-building-regs-notes.downloaded.pdf` | 146K | `cab336249ec96fe2703836fe592326c8b7166308dfd69b363566f43f05a3a7bc` | Successful PDF |
| `Cavity-wall-partial-fill-calculation.downloaded.pdf` | 33K | `fea60d043bd0dbfb99e1d39b0ca718c18ae4de9137ed0cdba044e0aa29973591` | Successful PDF |
| `contracts-finder-notice-page.html` | 44K | n/a | Landing-page evidence |
| `cf-cookies.txt` | 226B | n/a | Cookie jar |
| `Frances-Street-building-regs-notes.pdf` | 58B | n/a | Failed first attempt; CF rate-limit message retained |
| `Cavity-wall-partial-fill-calculation.pdf` | 58B | n/a | Failed first attempt; CF rate-limit message retained |

The first unauthenticated/too-fast attempt produced `Rate limit of 12 exceeded. Please retry after 120 seconds.`. A later request with user-agent, referer, and cookie jar downloaded the PDFs successfully.

## Stage Outcomes

| Stage | What ran | DB state | Observation |
|---|---|---|---|
| Stage 8 document acquisition | Manual `raw_documents`, `tender_document_sets`, and extraction jobs for two PDFs | `tender_document_sets.id='tds-cf-f9abdf99-walkthrough'`, `doc_count=2`, `extraction_state='partial'` | `partial` is expected because only one of two PDFs needed semantic overlay for this proof. |
| Stage 8 structural extraction | `cargo run -p daemon -- --runtime-dir runtime-cf-walkthrough --run-next-job` twice | `document_extractions` rows for both PDFs: 2 pages/5475 markdown chars and 1 page/48159 markdown chars | Docling extracted both PDFs. |
| Stage 8 semantic extraction | Existing `pdf-extraction/tender-specification` overlay via queued semantic job `job-5e3475f5-b19c7...` | `document_semantic_extractions.id='semantic-aec80205-4ccd-487a-80f2-cc4bb4018499'`, confidence 8400 bps, 8 written refs | Current repo has no `pdf-extraction/cascade` skill path, so the current Docling + `tender-specification` route was used. |
| Stage 8 candidates | Deterministic scoring via `cargo run -p daemon -- --runtime-dir runtime-cf-walkthrough --pipeline contracts-finder` | 7 `product_candidates` for the target process | Candidates include cavity wall ties, Kooltherm K108 board, blockwork, plaster, mortar, airspace detail, and rendered dense blockwork. |
| Stage 9 search | Manual `marketplace-search/amazon-uk` skill run for `Stainless Steel Cavity Wall Ties` | 4 `search_attempts`, 0 `supplier_quotes` | Amazon UK returned EUR price labels on the UK domain. The skill result recorded `locale_or_currency_drift`, so no GBP quotes were persisted. |
| Stage 10 gross margin | `--pipeline contracts-finder` executed the gross-margin stage | `margins_assessed=0` for this process | No supplier quote existed, so there was no pricing assessment input. |
| Stage 11 landed cost | Not invoked for target after Stage 9 because there were 0 usable GBP quotes and 0 pricing assessments | 0 `landed_cost_estimates` for the target process | Documented blocker: no valid quote input. This satisfies the conditional R.6 path. |
| Stage 12 net margin | `cargo run -p daemon -- --runtime-dir runtime-cf-walkthrough --ingest net-gate` | 1 `candidate_net_margin_assessments` row for wall ties: `verdict='review'`, `reasons_json=["no_pricing_signal"]` | The no-pricing path is represented and re-evaluation is enabled. |
| Stage 13 manifest | `cargo run -p daemon -- --runtime-dir runtime-cf-walkthrough --ingest manifest` | Target manifest: `stoplight='clarify'`, `suggested_action='Clarify 1 candidate(s) before deciding'`, `candidate_count=1`, `priced_candidate_count=0` | Stage 13 successfully summarized a CF process with extracted candidates but no usable quote. |
| Stage 14 decision | `cargo run -p daemon -- --runtime-dir runtime-cf-walkthrough --capture-decision <pid> --disposition watch --idempotency-key cf-walkthrough-001 --reasoning "Manual CF walkthrough validation"` | `pending_operator_intents.idempotency_key='cf-walkthrough-001'`, `status='queued'`, `desired_disposition='watch'` | CLI decision capture succeeded against the CF manifest. |

## Stage 8 Product Candidates

| Candidate | Type | Identity bps | Quoteability bps |
|---|---:|---:|---:|
| `Inner Leaf Blockwork (100mm, k-value 0.49 W/mK)` | `sku` | 5300 | 5000 |
| `Kingspan Kooltherm K108 Cavity Board (100mm)` | `sku` | 6000 | 5800 |
| `Mortar (Cavity Wall Bridging)` | `sku` | 5300 | 5300 |
| `Rendered Dense Blockwork Outer Leaf (100mm + 10mm Polymer Render)` | `sku` | 6000 | 5800 |
| `Lightweight Plaster (Internal Finish, 13mm)` | `sku` | 5300 | 5300 |
| `Low-E Cavity Airspace Detail (50mm Residual)` | `sku` | 7000 | 6500 |
| `Stainless Steel Cavity Wall Ties (5.46mm dia, 3 per m2)` | `sku` | 6000 | 5800 |

## Validation Queries

Key query results:

| Check | Result |
|---|---|
| Target product candidates | 7 rows |
| Target search attempts | 4 rows, all `locale_or_currency_drift` |
| Target supplier quotes | 0 rows |
| Target landed-cost estimates | 0 rows |
| Target net-margin assessments | 1 row, `review`, `["no_pricing_signal"]` |
| Target manifest | 1 row, `clarify`, `Clarify 1 candidate(s) before deciding` |
| Target operator intent | 1 row, `cf-walkthrough-001`, `queued`, `watch` |
| Other CF product candidates | 0 rows |
| Source counts | `contracts-finder=100`, `procontract=140` |
| ProContract manifests | 7 |
| ProContract landed-cost estimates | 4 |
| ProContract net-margin assessments | 17 |
| DGU demo intents | `dec-2026-05-01-dgu-001=superseded`, `dec-2026-05-01-dgu-002=withdrawn` |
| DGU operator override | Active `qty=250.0` |

Agent-session evidence:

| Job | Skill | Mode | Status | Evidence |
|---|---|---|---|---|
| `job-5e3475f5-b19c-427b-a45e-71ed0350b25b` | `pdf-extraction/tender-specification` | contract | completed | `artifacts/sessions/2026/05/session-semantic-c3fd57d4101de964e9ecfc9bfa24baee5b20bc68be8d262795747bbf514f0497/contract-output.index.json` |
| `job-add8e7c35676d9265b600e11f2275c829597bad46b717d248241be1e34398604` | `skills/marketplace-search/amazon-uk/SKILL.md` | pty | completed | `artifacts/sessions/2026/05/session-manual-2e184bdebac785c2fb3c0537c4b35034342f8b2cdbc33746ebabd0ad75c4ec07/stream.index.jsonl` |

## Findings

1. Contracts Finder open tenders are represented as `procurement_stage='open'`, not `tender`. The D1 SQL from the handoff was too narrow for this dataset.
2. CF table-level estimated value fields are null for the 7 open notices; value is present in `normalized_payload.tender.value`.
3. The current repo does not contain the requested `skills/pdf-extraction/cascade` path. The working route is Docling structural extraction plus the existing `pdf-extraction/tender-specification` semantic overlay.
4. The selected CF notice is Stage 7 rejected as `works`, but its technical PDF contains source-agnostic physical product/material candidates. This proves Stage 8 extraction and later decision stages can operate on CF documents, while also showing that the marketplace auto-trigger gate remains notice-qualification-bound.
5. Normal `--ingest amazon-uk` would not select this process because the notice-level qualification run has `goods_qualified=0`. Manual Stage 9 skill invocation was therefore required for the proof.
6. Amazon UK rendered EUR price labels in this session. The marketplace skill correctly treated this as locale/currency drift and did not persist unusable non-GBP quotes.
7. The Amazon skill result used alias field names (`wave`, `query`, `results_count_estimate`) rather than the canonical schema names (`wave_number`, `query_used`, `result_count`). The daemon still persisted search attempts, but the persisted attempts have blank `query_text` and `wave_number=0`. This is a result-contract quality issue, not a blocker for the CF walkthrough.

## Code Fixes And Tests

No daemon code changes were made.

One minimal desktop test fixture fix was applied after the required quality gate exposed a stale Stage 14 disposition value:

- `apps/desktop/src-tauri/tests/query_service.rs`: changed two fixture/operator-decision expectations from `pursue` to `approve`, because `pending_operator_intents.desired_disposition` is constrained to `approve`, `reject`, or `watch`. `pursue` remains valid as a manifest stoplight, not as an operator disposition.

No daemon regression tests were added because the walkthrough did not require a daemon code fix. The observed CF walkthrough gaps are operational/data-shape issues:

- CF-specific auto document acquisition remains out of scope and is tracked by `MS-ENRICH-1`.
- Marketplace result-contract alias tolerance could be handled as a future hardening bead if desired, but it was not required to complete this proof.

Quality gates run after the fixture correction:

| Gate | Result |
|---|---|
| `ubs apps/desktop/src-tauri/tests/query_service.rs docs/pipeline/contracts-finder-stages-8-14-walkthrough-validation-manifest.md` | Exit 0; warnings were existing test panic/assert inventory. |
| `cargo fmt --check` | PASS |
| `cargo check --all-targets` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test -p daemon --bin daemon` | PASS, 169/169 |
| `cargo test -p procintel-desktop --test query_service` | PASS, 22/22 in the current tree |
| `cd apps/desktop && bunx vitest run` | PASS, 8 files and 36 tests |
| `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || break; done` | PASS, all 39 integration targets completed |

## Acceptance Criteria

| Criterion | Status | Evidence |
|---|---|---|
| R.1 CF process selected per D1 criteria; choice and reasoning in manifest | GREEN with documented D1 compromise | No row satisfied every D1 criterion. Selected the richest value-range PDF target and documented why. |
| R.2 Doc acquisition path documented with commands and file count | GREEN | Path B documented; 2 successful PDFs plus landing/cookie evidence; 2 failed rate-limit placeholders retained. |
| R.3 PDF extraction succeeded with agent-session evidence | GREEN with current skill-layout note | One PDF completed semantic extraction via existing `pdf-extraction/tender-specification`; both PDFs completed structural extraction. |
| R.4 Stage 8 product candidates exist | GREEN | 7 `product_candidates` rows for target process. |
| R.5 Stage 9 quote or documented no quote | GREEN | 4 Amazon search attempts, 0 quotes, all blocked by locale/currency drift. |
| R.6 Stage 10-11 LCE if quotes exist, or documented blocker | GREEN | No GBP supplier quote existed; 0 pricing assessments and 0 LCE rows for target are expected. |
| R.7 Stage 12 net-margin assessment exists | GREEN | 1 row: `review`, `["no_pricing_signal"]`. |
| R.8 Stage 13 manifest exists | GREEN | 1 row: `stoplight='clarify'`, suggested action present. |
| R.9 Stage 14 pending operator intent exists | GREEN | `cf-walkthrough-001`, `queued`, `watch`. |
| R.10 Other 99 CF processes unchanged | GREEN | Other CF `product_candidates` query returned 0 rows. |
| R.11 ProContract universe unchanged | GREEN | 7 manifests, 4 LCE rows, 17 net-margin verdicts, DGU demo intents and override intact. |
| R.12 Code fixes minimal and tested | GREEN | One test-fixture-only desktop correction; gates rerun and passing. |
| R.13 Quality gates | GREEN | `fmt`, `check`, `clippy`, daemon bin tests, desktop query-service test, Vitest, and all 39 daemon integration targets passed. |

## Deferred Items

- Build a Contracts Finder auto-enrichment skill that navigates/downstreams CF documents automatically: `MS-ENRICH-1`.
- Tighten marketplace result contract validation or alias normalization for `wave`/`query`/`results_count_estimate`.
- Revisit notice qualification for works notices that contain separable material/product candidates, if the product strategy wants those to enter Stage 9 automatically.
