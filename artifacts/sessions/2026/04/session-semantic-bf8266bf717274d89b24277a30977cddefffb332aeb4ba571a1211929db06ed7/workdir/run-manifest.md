---
manifest_version: "1.0.0"
skill_id: "pdf-extraction/tender-specification"
skill_version: "1.0.0"
job_id: "job-096317de-47ed-4301-836f-2238999d7fad"
session_id: "session-semantic-bf8266bf717274d89b24277a30977cddefffb332aeb4ba571a1211929db06ed7"
execution_mode: "contract"
started_at: "2026-04-18T05:53:00Z"
completed_at: "2026-04-18T05:58:00Z"
outcome_class: "success"
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 0
agent_claimed_source_url: "https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545"
agent_claimed_fingerprint_state: "not_applicable_offline_markdown"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted a single-lot semantic payload for Lot 10 of the London Borough of Bexley Building Services Framework 6278 ("Planned, Preventative & Responsive Residential Property Maintenance and Compliance Services") from the Docling-extracted markdown of raw document `raw-pc-dc1daee2-772e-f111-813c-005056b64545-0a880f67497c48e109f63adc29f785182265551caac49f0b64701e48b52b7c9a` (source `.docx`, 57,681 characters of markdown). Produced one lot record with 46 requirements, 3 KPI-derived evaluation criteria, 31 technical standards, and 12 product requirement seeds — each anchored to a numbered clause in the specification. Document predominantly describes a services contract (Measured Term Contract under JCT MTC 2024, fixed 4-year term 1 Nov 2026 – 31 Oct 2030, ~230 Council-owned street properties + 120 private leased + 3 Hostels), so seeds skew toward service/inspection/certification classes rather than goods. `extraction_scope = full_document`, `target_domain = lots`, overall confidence 8800 bps.

## Steps Taken

1. Enumerated session workdir — found only the agent-mail SQLite (no pre-staged markdown input). Located the operational state DB at `/Users/alinaaitova/Desktop/Testproject1/state/state.db`.
2. Queried `document_extractions` for the provided `document_extraction_id = 280de666-51ab-4120-872b-33e48dc57013`; confirmed linkage to `raw_document_id` and pulled the `extracted_markdown` column (769 lines) to `/tmp/extracted.md` for analysis.
3. Read the specification end-to-end (§1 Contract Description through §21 TUPE) and catalogued: contract form, scope, priorities/response times, payment terms, statutory compliance clauses, H&S requirements, asbestos/glazing/electrical-specific rules, §10 Gas Safe servicing program, §11 staff suitability, §12 DBS/conduct, §20.2 KPIs.
4. Cross-referenced framework-level metadata: pulled existing `notice_lots` rows (confirmed Lot 10 exists with matching title in notice `pc-dc1daee2-772e-f111-813c-005056b64545`) and framework-level CPV codes from `contract_notices.normalized_payload.cpv_codes` (50700000-2, 50750000-7, 71315000-9). Treated these as advisory context only, not authoritative override.
5. Computed `sha256` of the extracted markdown (`8c42e3b8…2cee4a`) for the `source_markdown_hash` field.
6. Drafted the `lots[0]` entry — one lot record with `lot_number = "10"`, 46 direct-quote-adjacent requirements, 31 technical standards (each individually anchored), 12 product requirements with per-seed evidence anchors, and 3 KPI-derived evaluation criteria from §20.2.
7. Generated mirrored `product_requirement_seeds` entries (one per lot-level product requirement) with `seed_key` stability, per-seed CPV inferences where a reasonable match exists, and linked evidence anchors.
8. Validated `result.json` against `skills/pdf-extraction/tender-specification/result.schema.json` using `jsonschema.validate` — passed.
9. Wrote this run manifest.

## Decisions Made

- **Single-lot model.** The source document is scoped to Lot 10 only (confirmed by filename "6278 - Draft Specification - Lot 10" and internal scope language). Other lots (Lots 1–11) exist in the framework but are covered by sibling specification documents. Emitted exactly one `lots[]` entry rather than fabricating placeholders for sibling lots.
- **KPIs as evaluation_criteria.** The document does not contain bid award criteria or price/quality weightings. It does contain three explicit contractual KPIs (§20.2). Mapped these to `evaluation_criteria` as the closest semantic fit and emitted warning `NO_AWARD_CRITERIA_WEIGHTS` so downstream consumers do not mistake them for tender scoring weights.
- **CPV code provenance.** The specification does not cite CPV codes inline. Lot-level `cpv_codes` inherits 50700000-2 (Repair and maintenance services of building installations) and 71315000-9 (Building services) from the notice-level framework as the best-evidenced matches to this Lot's residential maintenance scope; excluded 50750000-7 (Lift maintenance) as out of scope. Per-seed CPVs are analyst-inferred from cited standards and activities (e.g., 50531200-8 for gas appliance maintenance, 71317100-4 for fire/safety inspection, 90650000-8 for asbestos). Emitted warning `CPV_NOT_DIRECTLY_CITED` and lowered `cpv_codes` field confidence to 5500 bps.
- **Consolidated BS 7671 standard entry.** Document cites BS 7671 at two editions (Seventeenth in §5.1 — with an apparent "BS 7671:200" typo — and Eighteenth in §7.2). Emitted a single technical_standards record carrying both citations in the context field, plus warning `WIRING_REGULATIONS_EDITION_MISMATCH`.
- **Evidence anchors use section references, not page numbers.** Docling reported `page_count = 0` for this `.docx` source, so `page_ref = "full_document"` is used across all anchors and the numbered clause (`§10.1`, `§20.2`, etc.) is carried in `section_ref`. Warning `DOCUMENT_LACKS_PAGE_NUMBERS` flags this for downstream readers who need true page numbers.
- **Product seeds scoped to seeds only.** Per skill contract, the skill emits seeds — not authoritative `product_candidates` rows. Each seed carries stable `seed_key`, `lot_number = "10"`, and evidence anchor for the daemon to persist.

## Warnings

- `CPV_NOT_DIRECTLY_CITED` — the specification does not cite CPV codes inline; lot-level codes inherited from the framework notice, seed-level codes are analyst-inferred.
- `NO_AWARD_CRITERIA_WEIGHTS` — no bid award criteria / weightings are present; evaluation_criteria captures performance KPIs from §20.2.
- `SINGLE_LOT_DOCUMENT` — this document covers only Lot 10; sibling lots live in separate specifications.
- `WIRING_REGULATIONS_EDITION_MISMATCH` — BS 7671 edition labelling is inconsistent across §5.1 (17th Ed, with likely typo "BS 7671:200") and §7.2 (18th Ed).
- `DOCUMENT_LACKS_PAGE_NUMBERS` — Docling reported `page_count = 0`; all evidence anchors use `page_ref = "full_document"` with section-level granularity.

## Evidence

All major claims are backed by anchors already embedded in `result.json` (`evidence_anchors[]` plus per-lot / per-criterion / per-seed / per-standard anchors). Key cross-references:

- **Lot identity and scope** — "CONTRACT SPECIFICATION — Measured Term Contract – Planned, Preventative & Responsive Residential Property Maintenance and Compliance Services" (document title) + §1 Contract Description + §2.2 property counts.
- **Contract form** — §2.1 "This Contract shall be governed by the JCT Measured Term Contract 2024 (MTC 2024)".
- **Term (4 years, no extension)** — §18 "The contract will be awarded on a fixed term basis for four years from 1st November 2026 to 31st October 2030 with no option to extend or price uplifts."
- **Response and completion times** — §3.3 priority/response table (Standard/Urgent/Emergency/Small Void/Large Void).
- **KPIs (evaluation_criteria)** — §20.2 three KPI blocks with 90%/100%/90% targets.
- **Gas servicing programme and LGSR** — §10.1 Servicing Program; §10.2 Gas Safety Inspections; §10.3 Void Gas Safety Inspections; §10.4 Breakdowns; §10.5 System Boiler Replacements.
- **Statutory compliance umbrella** — §5.1 lists Building Regulations 2010, Building Safety Act 2022, CDM 2015, Gas Safety Regs, BS 7671, Fire Safety Order 2005, EPA 1990, GDPR/DPA 2018.
- **Health & safety, CDM, asbestos, glazing, electrical, PPE, hot works, working-at-height, COSHH, RIDDOR** — §6 (body) and interleaved §§7.2, 7.4, 8, 9.
- **Staff suitability and ID cards** — §11 (ID card fields) and §12.1 (CSCS / DBS / photo ID requirements).
- **Environmental obligations** — §13 Environment & Sustainability (EPA 1990, waste carrier registration, spill kits).
- **TUPE** — §21 Transfer of Undertakings (Protection of Employment) Regulations 2006.

Observable facts carried in daemon-visible form:
- Source markdown length: 57,681 chars (daemon-observable via `document_extractions.length(extracted_markdown)`).
- `source_markdown_hash = 8c42e3b8d65dc697c7c6193c553f16a08800e8065d97d065d31a4a796f2cee4a` (sha256 of extracted markdown).
- Docling `page_count = 0`; document title on extraction record = "6278 - Draft Specification - Lot 10"; document_role = "technical_specification".
- Raw document source URL = `https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545`.
- Notice-level framework CPVs available for cross-reference: 50700000-2, 50750000-7, 71315000-9 (latter two either inherited or excluded as documented under Decisions Made).
