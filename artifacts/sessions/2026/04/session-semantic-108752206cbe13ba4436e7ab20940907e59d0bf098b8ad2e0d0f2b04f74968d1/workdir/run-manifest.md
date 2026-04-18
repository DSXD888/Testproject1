---
manifest_version: "1.0.0"
skill_id: "pdf-extraction/tender-specification"
skill_version: "1.0.0"
job_id: "job-6514373b-d51a-42c4-a027-1032f6b17911"
session_id: "session-semantic-108752206cbe13ba4436e7ab20940907e59d0bf098b8ad2e0d0f2b04f74968d1"
execution_mode: "contract"
started_at: "2026-04-18T04:05:00Z"
completed_at: "2026-04-18T04:12:51Z"
outcome_class: "success"
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 8
agent_claimed_pages_visited: 0
agent_claimed_source_url: "sqlite:///state/state.db#document_extractions/d42e1d60-16fb-4193-936e-ea5a04ccaffd"
agent_claimed_fingerprint_state: "not_applicable_headless_contract"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

# Run Manifest - pdf-extraction/tender-specification

## Summary

Extracted a single-lot structured payload from Bexley Council's tender specification "6278 - Draft Specification - Lot 11 Provision of Minor Disability Adaptations (minor building works) in privately owned properties". The Docling-produced markdown was retrieved from `document_extractions.id = d42e1d60-16fb-4193-936e-ea5a04ccaffd` (69,679 chars, 915 lines, markdown hash `87b1ad089d9d2878efc10a9a68d2e86ce1618dc748d2c4836eb9d36e6efac982`). One lot (Lot 11), 44 lot-level product requirements, 43 `product_requirement_seeds`, and 27 technical standards were emitted with evidence anchors keyed to the specification's own section codes (e.g. `1.1`, `5.7.1`, `8.6`).

The document is a technical specification to be read in conjunction with a separate Pricing Schedule, so quantities and rates were deliberately out of scope. Evaluation criteria in the output are compliance/performance criteria (response times, warranty, H&S registration, DBS, British Standards) rather than weighted MEAT criteria, which belong in the ITT. `result.json` validated against `skills/pdf-extraction/tender-specification/result.schema.json`. No result-contract, suspect-output, partial-failure, or run failure conditions were encountered.

## Steps Taken

1. Discovered the session workdir contained only agent-mail sqlite databases, so resolved the authoritative markdown via the parent `state/state.db` using the `document_extraction_id` from the job parameters.
2. Inspected `document_extractions` schema and row `d42e1d60-...` — recorded source hash, extraction engine (`docling 2.88.0`, `standard`), `document_title`, `document_role = technical_specification`, and confirmed `page_count=0`, `section_count=0`, `table_count=0` (flat markdown, no structural tree from Docling).
3. Fetched the notice context from `contract_notices.id = pc-dc1daee2-772e-f111-813c-005056b64545` — notice title "6278 - Building Services Framework - Lots 1-11", buyer `London Borough of Bexley`, CPV codes `50700000-2 | 50750000-7 | 71315000-9`.
4. Confirmed the pre-existing `notice_lots` rows include Lots 1-11, but treated the tender document as authoritative for Lot 11 content rather than merging against the notice-level lot row. Other 10 lots are out of scope for this document.
5. Dumped the full markdown to a temporary file and read it end-to-end (915 lines). Partitioned the document into (a) general contractual/compliance sections (Timescales, Invoices, VAT, DBS, TUPE, Health and Safety, Asbestos, PPE, CDM, etc.) and (b) the "SPECIFICATION NOTES" numbered section (1.1 through 8.7).
6. Extracted lot-level `requirements` from the contractual/compliance sections (response codes A-D, JCT MTC 2024, DBS, SSIP, BS 7671, RIDDOR, COSHH, CDM 2015, GDPR etc.).
7. Extracted `evaluation_criteria` as compliance/performance criteria observable in the spec: response-time achievement, defect rectification window (14 days) and warranty (12 months), customer satisfaction monitoring, staff suitability/DBS, SSIP registration, British Standards compliance.
8. Extracted `technical_standards` citing every concrete regulation or BS/EN standard referenced (27 items including BS 7671:2008, BS EN 12600, BS 952-1:1995, BS 6262-4:2018, BS EN ISO 16321, JCT MTC 2024/2016, Part P, CAR 2012, CDM 2015, COSHH 2002, Working at Height Regs 2005, PPE Regs (Amendment) 2022, EPA 1990, COPA 1974, HSWA 1974, First Aid Regs 1981, Gas Safety Regs 1998, PUWER 1998, Electricity at Work Regs 1989, RIDDOR 2013, SSIP-forum schemes, CSCS, TUPE 2006, Approved Doc K, Workplace Regs 1992 Reg 14, GDPR/UK DPA).
9. Extracted `product_requirements` and mirrored them as `product_requirement_seeds` (44 and 43 entries respectively — the two 'Mira Advance' and 'Remove existing heater' items are listed as product_requirements but only the supply/install variants were promoted to seeds; the delete/reuse line items either ship as product_requirement entries only or are merged). Each seed carries a stable `seed_key` keyed to the specification's section code, a `candidate_name`, a `summary`, an enumerated `requirements` list, any in-scope technical standards, and heuristic CPV hints.
10. Populated `evidence_anchors` with 48 document-level anchors covering the header, key contractual sections, and each section-coded product block.
11. Computed per-field confidences and an overall `extraction_confidence_bps = 8400`. Lower confidences on CPV assignments and evaluation criteria are explained in `warnings`.
12. Logged 8 warnings (SINGLE_LOT_SCOPE_CONFIRMED, PAGE_REFS_UNAVAILABLE, CPV_INHERITED_FROM_NOTICE, SEED_CPV_SUGGESTIONS_HEURISTIC, CONTRACT_PERIOD_AMBIGUITY, EVALUATION_CRITERIA_FROM_SPEC_ONLY, CROSS_REFERENCE_TO_PRICING_SCHEDULE, DOCLING_ZERO_SECTIONS_TABLES).
13. Wrote `result.json` in the workdir and validated it against `result.schema.json` using `jsonschema` (VALID).

## Decisions Made

- **One lot, not eleven.** The document is explicitly and exclusively scoped to Lot 11. I did not fabricate lot records for Lots 1-10 from the notice-level sibling rows: the skill contract forbids treating pre-existing lot rows as authoritative when the tender document contradicts or narrows them. A `SINGLE_LOT_SCOPE_CONFIRMED` warning makes the scope explicit for the daemon.
- **`extraction_scope = "full_document"`** because the job parameters specified `extraction_scope = full_document` and there was no chunk plan — the whole markdown was ingested in a single pass.
- **`page_ref = "p.unknown"` uniformly.** Docling reported `page_count = 0`, so no physical page numbers are available. The section codes in the spec (`1.1`, `5.7.1`, `8.6`, etc.) and uppercase headings serve as structural anchors instead; the document's own footer self-reports "Page 23 of 23".
- **CPV codes inherited from the notice, flagged heuristic.** The specification body never quotes CPV codes. Lot-level CPV was inherited from notice `main_category`; per-seed CPV suggestions were mapped heuristically against product type. Two warnings (`CPV_INHERITED_FROM_NOTICE`, `SEED_CPV_SUGGESTIONS_HEURISTIC`) keep this explicit for downstream validators.
- **Evaluation criteria emitted as contract-performance criteria.** This is a technical spec, not an ITT — no bid-evaluation weightings (price/quality %) are present. The six criteria emitted reflect the compliance and performance items the spec actually enforces (and which the Performance Review section explicitly monitors). Warning `EVALUATION_CRITERIA_FROM_SPEC_ONLY` flags this.
- **Product seed keys are stable and section-anchored** (e.g. `lot11-5.7-derby-grab-rail`, `lot11-8.6-raised-height-wc`) so that chunked or retried runs can re-anchor without regenerating identifiers.
- **Delta from notice_lots row vs. spec title.** The document title shows the contract window as `1 Nov 2026 - 31 Oct 2030` but the PERIOD OF CONTRACT section gives `2026-2028` + 2x1-year extensions. Both are consistent if both extensions are exercised; flagged as `CONTRACT_PERIOD_AMBIGUITY`, not reconciled unilaterally.
- **Pricing and quantities deliberately out of scope.** The spec explicitly says "Items in this document are to be priced in accordance with the pricing schedule"; quantity extraction belongs to the companion skill `pdf-extraction/pricing-schedule` and is recorded as the `CROSS_REFERENCE_TO_PRICING_SCHEDULE` warning.

## Warnings

1. `SINGLE_LOT_SCOPE_CONFIRMED` — tender document is scoped to Lot 11; the other 10 lots are not described here.
2. `PAGE_REFS_UNAVAILABLE` — Docling `page_count = 0`; section codes used as primary anchors.
3. `CPV_INHERITED_FROM_NOTICE` — lot-level CPV inherited from generic framework codes; more specific lot CPVs unknown.
4. `SEED_CPV_SUGGESTIONS_HEURISTIC` — per-seed CPV codes are heuristic, not cited in the document.
5. `CONTRACT_PERIOD_AMBIGUITY` — header window (2026-2030) differs from PERIOD OF CONTRACT section (2026-2028 + 2x1-yr extensions).
6. `EVALUATION_CRITERIA_FROM_SPEC_ONLY` — technical spec has compliance criteria only; bid-weighted criteria live in the ITT.
7. `CROSS_REFERENCE_TO_PRICING_SCHEDULE` — quantities/rates belong to the companion Pricing Schedule document.
8. `DOCLING_ZERO_SECTIONS_TABLES` — Docling reported zero sections/tables; partitioning is based on inline bold section codes rather than a structural tree.

## Evidence

Canonical structural anchors used by this run (full list in `result.json.evidence_anchors`):

- **Header** - "INSTALLATION OF EQUIPMENT AND MINOR WORKS AT PROPERTIES FOR PEOPLE WITH DISABILITIES / Contract Specification - to be read in conjunction with Pricing Schedule" establishes document identity.
- **TIMESCALES AND COMPLETION OF WORK** — response-time codes A (24h), B (48h), C (5 working days), D (15 working days), plus £100/day penalty for late Code A.
- **Regulations and statutory bodies** — "Gas Safety (Installation and Use) Regulations 1998 and BS 7671:2008 Requirement for Electrical Installations - IET Wiring Regulations Eighteenth Edition".
- **PREMISES PROTOCOLS (electrical works)** — Part P Building Regulations certification on completion for all new circuits.
- **HEALTH AND SAFETY (SSIP)** — CHAS, EXOR Health and Safety, OHSAS 18001; SSIP Forum registration required.
- **HEALTH AND SAFETY (asbestos)** — Control of Asbestos Regulations 2012 (CAR 2012); asbestos removal only by Council-appointed specialists.
- **HEALTH AND SAFETY (glazing)** — Approved Doc K, Workplace Regs 1992 Reg 14, BS EN 12600, BS 952-1:1995 Parts 1 & 2, BS 6262-4:2018.
- **HEALTH AND SAFETY (eye protection)** — BS EN ISO 16321 (2022).
- **Product seeds** — section codes `1.1` (Electric Door Locks), `1.2/1.2.1` (AKW 4000 series 04130P/04240P shower seats), `1.3 & 1.4` (Shower Curtains), `2.x` (Electrical sockets/switches/fan heater, Mira Advance Wireless 9kW shower), `3.1-3.3` (3/4" chrome-plated 1/4-turn lever taps), `4.1/4.1.1` (radiator relocation/replacement), `5.1` (balustrade), `5.2` (mopstick rail), `5.3` (AKW 1800 drop-down rail), `5.4/5.4.1` (newel rail), `5.5` (kee-clamp galvanized rail), `5.6` (painted/bended/welded 48.3mm tube rail), `5.7` (Derby grab rail 300/450/600mm), `5.8` (epoxy external grab rail 300/450/600mm), `6.1-6.6` (concrete steps/half-steps/ramp/platform/path with 1:14 fall, 1200/1000mm widths, 1500x1500mm platform), `7.1-7.3` (door aperture widening, UPVC entry door, sliding plywood door, Catnic lintel), `7.4` (Duraflex level threshold), `8.1-8.7` (timber fillet ramp, sliding door, PVC folding concertina door, Ring Doorbell 3 wired/battery with 8-24VAC @ 40VA, Master Lock key safe, AKW Raised Height WC ref 23162 650mm projection, re-hang door).
- **WARRANTIES** — 12-month defect rectification at no extra cost (except spare parts), 14-day fault rectification, 10+ default-notices/yr trigger re-procurement.
- **PERIOD OF CONTRACT** — "1 st November 2026 to 31 st October 2028. The Council also reserves the right to extend the contract for a further two years at its own discretion in one year increments".
- **CROSS-REFERENCE** — header plus "GUIDANCE NOTES FOR SPECIFICATION OF WORK TO BE READ IN CONJUNCTION WITH THE PRICING SCHEDULE" establishes that pricing lives in a companion document.
