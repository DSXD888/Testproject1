---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-10f7fddb-61ce-4edc-b582-1542e3a505d7
session_id: session-semantic-02fcff2e1d0c954e46f7dd38cc509dcfbc3bfd5a5d36a43a1f62c8a49d1e82af
execution_mode: contract
started_at: 2026-04-18T02:31:00Z
completed_at: 2026-04-18T02:36:44Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: []
agent_claimed_source_url: "https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Summary

Extracted one authoritative lot (Lot 11 — Measured Term Contract for Provision of Minor Disability Adaptations / Minor Building Works) from the Docling-produced markdown of `6278 - Draft Specification - Lot 11.doc` for the London Borough of Bexley Building Services Framework 6278. The document is a contract specification to be read alongside a Pricing Schedule; it defines administrative/H&S obligations and 44 specification codes (1.1 – 8.7) that were each emitted as a `product_requirement_seed`. Twenty-six named technical standards and statutory regulations were extracted, each with evidence anchors. No evaluation/award criteria are present in this document (flagged as a warning). CPV codes are notice-level only and were intentionally left empty to avoid lot-level fabrication.

# Steps Taken

1. Located the job's `document_extraction_id = c313316c-59af-4b36-bb53-7763aa283f50` in `/Users/alinaaitova/Desktop/Testproject1/state/state.db` (`document_extractions` table).
2. Read `extracted_markdown` (69,679 bytes, 915 lines), `extracted_sections_json` (empty `[]`), `extracted_tables_json` (empty `[]`) and `extracted_metadata_json` (Docling structure JSON, no pagebreak info).
3. Joined to `contract_notices` (`pc-dc1daee2-772e-f111-813c-005056b64545`) and `notice_lots` to confirm the document is scoped to Lot 11 only; other lots (1–10) are captured on separate extractions.
4. Computed the source markdown SHA-256: `4099c275a37939274b2edd260aea8b6277974f8ee9dc2150c798e92ee4c8d8d9`.
5. Identified sections by structural reading of the markdown: title block; General / Timescales / Invoices / VAT / DBS / TUPE / No Smoking / General Expectations / Regulations / Damage and Re-instatement / Execution of Works / Premises Protocols / Staff Suitability / Environment and Energy / Health and Safety (inc. Asbestos, CDM 2015, Working at Height, COSHH, PPE, Glazing, RIDDOR) / Quotation Visits / Pricing / Warranties / Additional Work Reports / Period of Contract / Confidentiality / Performance and Administration / Guidance Notes for Specification of Work (codes 1.1–8.7).
6. Built `lots[0]` with authoritative title (from notice_lots.lot_11 = "Provision of Minor Disability Adaptations (Minor Building Works) in Privately Owned Properties"), 28 requirement bullets drawn verbatim / paraphrased with evidence.
7. Emitted one `product_requirement` per specification code (40 codes × some numbered variants = 40 product_requirements on the lot; 44 `product_requirement_seeds` at top level including the 7.3 variant split for sliding-gear, Duraflex frame replacement and UPVC front/rear door).
8. Collected 26 `technical_standards` entries (BS 7671:2008 IET 18th Ed., Gas Safety 1998, JCT MTC 2024 and 2016, Approved Document K, BS EN 12600, BS 952-1:1995, BS 6262-4:2018, BS EN ISO 16321 (2022), PPE Regulations 2022, H&SAWA 1974, SSIP schemes, CDM 2015, CAR 2012, COSHH 2002, Working at Height 2005, Electricity at Work 1989, PUWER 1998, EPA 1990, COPA 1974, RIDDOR 2013, First Aid 1981, GDPR, TUPE 2006, Part P, Regulation 14 Workplace HSW 1992).
9. Validated the result against `skills/pdf-extraction/tender-specification/result.schema.json` (jsonschema draft 2020-12). Schema validation passed.
10. Wrote `result.json` and this run manifest to the session workdir.

# Decisions Made

- **Single-lot emission.** Only Lot 11 is described by this document (title, filename and body are Lot-11-specific). Emitting speculative entries for Lots 1–10 would violate "Do not claim a lot split ... without citing evidence anchors."
- **Empty `cpv_codes`.** Notice-level CPVs (50700000, 50750000, 71315000) describe the full framework across Lots 1–11. None of them is reaffirmed inside the Lot 11 document, so lot-level CPV classification is deferred rather than fabricated.
- **Empty `evaluation_criteria`.** This is a contract specification document, not an ITT / award-criteria document. A warning documents the absence.
- **`product_requirement_seeds` keyed by `lot11-<code>`.** This gives stable seed keys that survive resume / rerun and allows the daemon to align `product_candidate` writes with specific spec codes in the Pricing Schedule counterpart.
- **Generic `pg-1` page_ref.** Docling produced `page_count = 0` for this `.doc` conversion (no stable page boundaries). Using section / code anchors was selected as a meaningful substitute; a warning flags the limitation.
- **Contract-term conflict surfaced as a warning, not silently reconciled.** Header says 2026–2030; "PERIOD OF CONTRACT" clause says 2026–2028 with up to two one-year extensions. Reported both and used the body text for the requirement bullet.
- **`BS 7671:2008 ... 18th Edition` cited verbatim and flagged.** The specification's edition number is internally inconsistent (BS 7671:2008 is the 17th edition; the current 18th edition is BS 7671:2018+A2:2022). A warning records the verbatim citation without silent correction.
- **Outcome class `success`.** All required schema fields produced, all specification codes captured, 7 non-blocking warnings documenting source ambiguities.

# Warnings

Seven warnings are emitted inline in `result.json` and summarized here:

1. `LOT_SCOPE_SINGLE` — document only covers Lot 11; other lots intentionally not inferred.
2. `EVALUATION_CRITERIA_ABSENT` — specification does not contain an award-criteria section.
3. `CPV_NOT_DECLARED_IN_DOCUMENT` — CPVs left empty at lot level; notice-level overlay may apply.
4. `PAGE_NUMBERS_UNKNOWN` — Docling produced no page boundaries; `pg-1` + section refs used.
5. `CONTRACT_TERM_INCONSISTENCY` — 2030 header vs 2028 + 2-year-extension body clause.
6. `ELECTRICAL_STANDARD_CITED_UK_VARIANT` — "BS 7671:2008 ... 18th Edition" captured verbatim despite internal year/edition mismatch.
7. `PRICING_CODES_REFERENCE_EXTERNAL_SCHEDULE` — quantities / rates live in the Pricing Schedule companion document.

# Evidence

Major claims are anchored to observable text in the source markdown (hash `sha256:4099c275a37939274b2edd260aea8b6277974f8ee9dc2150c798e92ee4c8d8d9`). Representative anchors:

- **Lot identity / title block.** `INSTALLATION OF EQUIPMENT AND MINOR WORKS AT PROPERTIES FOR PEOPLE WITH DISABILITIES` — title block at top of document; matched to `notice_lots.lot_key=11` title `Measured Term Contract – Provision of Minor Disability Adaptations (minor building works)`.
- **Contract period.** `The contract will cover the period 1 st November 2026 to 31 st October 2028` — PERIOD OF CONTRACT clause.
- **Response-time tiers.** `Standard work (Code D) is to be completed within 15 working days`, `Urgent work (Code C) is to be completed within 5 working days`, `Emergency work (Code B) is to be completed within 48 hours`, `Works relating to discharge from hospital (Code A) to be completed within 24 hours`.
- **DBS.** `all operatives visiting clients in their homes ... must have been subjected to an enhanced DBS check`.
- **TUPE.** `Transfer of Undertakings (Protection of Employment) Regulations 2006`.
- **Electrical regulations.** `Gas Safety (Installation and Use) Regulations 1998 and BS 7671:2008 Requirement for Electrical Installations - IET Wiring Regulations Eighteenth Edition`.
- **SSIP / CHAS / EXOR / OHSAS 18001.** `the pre-qualification schemes that form the SSIP (Safety Schemes in Procurement) Forum as the minimum standards of Health and Safety competence`.
- **CDM 2015.** `the Construction (Design and Management) Regulations 2015 apply`.
- **Asbestos.** `The Control of Asbestos Regulations 2012 (CAR 2012)`.
- **Glazing.** `All safety glazing must be clearly and permanently marked in accordance with BS EN 12600`; `BS 952-1:1995 Parts 1 & 2`; `BS 6262-4:2018`.
- **PPE / eye protection.** `BS EN ISO 16321 (2022)`; `The Personal Protective Equipment at Work (Amendment) Regulations 2022`.
- **Environmental / waste.** `The Environmental Protection Act 1990 (EPA1990)`.
- **Data protection.** `The EU's General Data Protection Regulation (GDPR) will apply`.
- **Warranty period.** `Equipment defects and/or breakdowns occurring within twelve months of repair or installation shall be rectified by the contractor at no extra cost`.
- **Product models (cited verbatim in markdown).** `AKW 4000 series 04130P (Blue) tip up padded shower seat`; `Extra wide AKW 4000 series 04240P (grey)`; `Mira Advance Wireless 9kw electric shower unit`; `AKW 1800 series fold-up double rail with support leg`; `AKW Raised Height WC suite (650mm projection ref 23162)`; `Ring Doorbell 3`; `Master Lock water resistant combination key safe`; `kee clamp' or similar galvanized '33.7mm (size 6) or 42.4mm (size 7)'`; `48.3mm diameter tube, welded and bent to shape`; `Catnic insulated steel or P.C.C lintel`; `'Duraflex' or similar level wheelchair accessible threshold`.
- **Guidance-notes header.** `GUIDANCE NOTES FOR SPECIFICATION OF WORK TO BE READ IN CONJUNCTION WITH THE PRICING SCHEDULE` — confirms pricing codes 1.1–8.7 cross-reference the Pricing Schedule.

Artifact refs:
- Source markdown hash: `sha256:4099c275a37939274b2edd260aea8b6277974f8ee9dc2150c798e92ee4c8d8d9` (stored in `document_extractions.extracted_markdown` row `c313316c-59af-4b36-bb53-7763aa283f50`).
- Source URL on the daemon-observed raw document: `https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545` (rfxId differs from parent notice because the Lot 11 draft spec sits under a sub-RfX within advert `dc1daee2-772e-f111-813c-005056b64545`).
- Result contract: `result.json` in session workdir, validated against `skills/pdf-extraction/tender-specification/result.schema.json`.
