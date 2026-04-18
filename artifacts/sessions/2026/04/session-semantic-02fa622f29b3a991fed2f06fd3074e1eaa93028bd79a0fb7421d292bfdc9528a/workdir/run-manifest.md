---
manifest_version: "1.0.0"
skill_id: "pdf-extraction/tender-specification"
skill_version: "1.0.0"
job_id: "job-2dbb4fde-5d02-4b82-ad61-90abcfbb68e1"
session_id: "session-semantic-02fa622f29b3a991fed2f06fd3074e1eaa93028bd79a0fb7421d292bfdc9528a"
execution_mode: "contract"
started_at: "2026-04-17T20:24:00Z"
completed_at: "2026-04-17T21:40:00Z"
outcome_class: "success"
items_processed: 11
items_skipped: 0
items_failed: 0
warnings_count: 8
agent_claimed_pages_visited: 0
agent_claimed_source_url: "https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545"
agent_claimed_fingerprint_state: "not_applicable"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

# Run Manifest - pdf-extraction/tender-specification

## Summary

Full-document semantic extraction of the Procurement Specific Questionnaire (PSQ) for the London Borough of Bexley 6278 Building Services Framework (Lots 1-11). Input was Docling-produced markdown (114,881 chars; SHA-256 8b70398e...8ade6c; source_markdown_hash embedded in result.extraction_metadata). Output written to `result.json` with `target_domain = "lots"` and `extraction_scope = "full_document"`. Eleven authoritative lot records and thirteen product-requirement seeds were emitted; all lot records carry a primary evidence anchor. Overall extraction confidence: 8200 bps.

The document is a selection-stage PSQ rather than a full technical specification, so lot descriptions are summary-level and detailed scope is deferred to the downstream tender spec (flagged in warnings). No CPV codes are cited at lot level in the document text; notice-level CPV context was noted in warnings rather than attributed speculatively.

## Steps Taken

1. Located the structural-extraction payload: `document_extractions.id = 1226a90a-236a-4b5e-8fe4-33fbaf2e5b15` in the daemon state DB; 14 sections, 45 tables, 0 pages (docx source), `extraction_policy_version = 2026.04.12`, `expected_overlays_json = ["pdf-extraction/tender-specification"]`.
2. Verified process context: notice `pc-dc1daee2-772e-f111-813c-005056b64545` titled "6278 - Building Services Framework - Lots 1-11"; buyer London Borough of Bexley; notice `main_category` includes parent CPVs 50700000-2, 50750000-7, 71315000-9. `notice_lots` table was empty for this notice — no prior lot rows to reconcile.
3. Read the full Docling markdown and catalogued the major structural sections: document header, Summary of requirements (Lots 1-11 descriptions), Conditions of Participation, PSQ Explainer, Supplier Incentive Programme, Preliminary Questions (Q1-Q4), Part 1 (Q5), Part 2A (Q6-Q9), Part 2B (Q10-Q11), Part 3 (Q12-Q36), Confirmations.
4. Extracted 11 lot records from the "Summary of requirements" block. Titles and descriptions were taken verbatim (with HTML entity decoding). Lot 8 was correctly captured despite triple-emphasis formatting in the source.
5. Extracted per-lot anticipated values from the Q13 Group 1 / Group 2 lot-grouping table (quantity hints only, flagged as indicative). Noted overlap where Lots 6 and 7 appear in both groupings.
6. Extracted insurance thresholds from Q22, preserving the per-lot variants: Lots 1-9 share a common EL £5m/£10m, PL £5m, PI £2m pattern; Lot 10 has elevated EL £10m / PL £20m / PI £2m; Lot 11 stated as EL £10,000,000 / PL £5,000,000 / PI £2,000,000.
7. Extracted selection/evaluation criteria (PSQ threshold of 16, references, Experian financial assessment, turnover rule) and mapped each to its originating question number.
8. Catalogued accreditations mentioned in Q18-Q36 as `technical_standards`. Universal standards (ISO 9001:2008, ISO 14001:2004, SSIP scheme incl. CHAS / EXOR / ISO 45001) attached to every lot. Domain-specific accreditations (ACOP L8, UKAS/HSE Licensed Contractor, BPCA/RSPH, Gas Safe, NICEIC/NAPIT, BAFE/FIA, DBS, Building Regulations 2010, CSCS cards, Environment Agency Waste Carrier Licence) attached only to lots where the lot description strongly implies applicability; residual ambiguity recorded as `STANDARD_APPLICABILITY_DEFERRED` warning.
9. Emitted 13 product_requirement_seeds: one service seed per lot, plus extra seeds for Lot 6 cleaning-consumables supply and Lot 11 adaptation-fittings supply where the document explicitly names physical items.
10. Validated the output JSON against `result.schema.json` (required keys present; evidence anchors on all standards and lot records; 11 lots; 13 seeds) and wrote `result.json` to the working directory.

## Decisions Made

- **Document role reclassified in output.** The upstream classifier tagged this document `technical_specification`, but the document is clearly a Procurement Specific Questionnaire (selection stage). `extraction_metadata.document_role = "procurement_specific_questionnaire"` was emitted accordingly, with a `DOCUMENT_ROLE_MISMATCH` warning so the daemon can decide whether to update the upstream role field.
- **cpv_codes left empty at lot level.** The PSQ text itself does not cite any CPV codes. Rather than speculatively replicate the notice-level parent CPVs (50700000-2 / 50750000-7 / 71315000-9) on specific lots, `cpv_codes` arrays were left empty and the notice-level context was captured in a `CPV_NOT_IN_DOCUMENT` warning.
- **Evaluation criteria limited to what the document actually states.** The PSQ pass-threshold (16) and reference-count rules are present; the full scoring/weighting matrix is in the external PCID and was not reproduced. Flagged as `SCORING_MATRIX_EXTERNAL`.
- **Domain-specific standards attached conservatively.** E.g. ACOP L8 attached to Lot 2 (water/legionella); UKAS Surveyor + EA Waste Carrier Licence to Lot 3 (asbestos); BPCA/RSPH to Lot 6 (internal pest control in scope); BAFE/FIA to Lots 1 and 8; Gas Safe and NICEIC/NAPIT to Lots 8 and 10; CSCS to site-work lots 5/8/9/10/11; DBS to lots with resident contact (10 and 11). Cross-applicability deferred to the tender spec via `STANDARD_APPLICABILITY_DEFERRED` warning.
- **Service-first seeding.** Because this is a services framework with minimal physical-goods content, the primary seeds are services. Two physical-goods seeds were added where the document explicitly names items (Lot 6 "purchase and supply of all consumables"; Lot 11 "handrails, grab rails, half steps").
- **Page refs use section labels.** Source was a .docx (Docling page_count = 0), so no page numbers exist. `page_ref` fields use section or question-number labels (e.g. "Part 3 Q22") with `anchor_text` quoting the verbatim sentence. Recorded as `PAGE_REF_UNAVAILABLE` warning.
- **Lot grouping preserved as metadata only.** Q13's Group 1 / Group 2 restriction (Group 1 = Lots 1-7, Group 2 = Lots 6-11; cannot bid across groups) was captured in a warning, not as a hard constraint on lot identity, because it governs bid-workflow routing rather than lot scope.

## Warnings

All warnings are emitted in `result.json.warnings`:

1. `DOCUMENT_ROLE_MISMATCH` - Upstream role is `technical_specification` but document is a PSQ (selection stage).
2. `SCORING_MATRIX_EXTERNAL` - Full scoring matrix in the PCID; only the threshold-of-16 pass/fail is in this document.
3. `CPV_NOT_IN_DOCUMENT` - No CPV codes in the text; notice-level parents available but not attributed to specific lots.
4. `STANDARD_APPLICABILITY_DEFERRED` - Q28-Q36 accreditations listed as "may apply" without lot-level assignment.
5. `LOT_GROUPING_RESTRICTION` - Group 1 / Group 2 bid exclusivity from Q13 (Lots 6 and 7 appear in both groupings).
6. `INSURANCE_LOT_10_HIGHER` - Lot 10 insurance thresholds (EL £10m / PL £20m / PI £2m) and Lot 11 (EL £10m / PL £5m / PI £2m) diverge from the common Lot 1-9 pattern.
7. `PAGE_REF_UNAVAILABLE` - .docx source; no page numbers; evidence anchors use section / question-number labels.
8. `LOT_VALUES_IN_Q13_ONLY` - Anticipated lot values are listed only in Q13 and are indicative.

## Evidence

| Claim | Source anchor | Section |
|------|---------------|---------|
| 11 lots exist (identity and ordering) | "Lot 1: Provision of Fire Risk Assessments"..."Lot 11: Provision of Minor Disability Adaptations" | Summary of requirements |
| PSQ pass threshold = 16 | "a minimum weighted score threshold of 16 will be applied when assessing PSQ's" | Summary of requirements / Conditions of Participation |
| Full scoring matrix external | "See the Procurement & Contract information Document (PCID) for the scoring matrix" | Summary of requirements |
| Reference rule (2 sought, min 1 received) | "Complete Part A of the reference form provided at Appendix 1 for 2 different clients, for each lot which you have applied"; "At least one reference must be received for each lot applied for" | Part 3 - Q14 Relevant experience |
| Financial assessment via Experian | "The Council will make a financial assessment of your company using the Experian Business Express platform" | Part 3 - Q12 |
| Turnover rule (2x lot value) | "Please self-certify that your company's annual turnover is at least twice the estimated value of the Lot or total value of Lot's for your which you have applied" | Part 3 - Q27 Financial Suitability |
| Group 1 / Group 2 bid exclusivity | "COMPANIES APPLYING FOR LOTS IN GROUP 1 CANNOT APPLY FOR LOTS IN GROUP 2 AND VICE VERSA" | Part 3 - Q13 |
| Per-annum indicative lot values | Q13 table cells listing "£25k per annum" through "£1.8m per annum" | Part 3 - Q13 |
| Insurance per lot (1-9 common) | "Employer's (Compulsory) Liability Insurance = of not less than £5 million pounds... Public Liability Insurance = for not less than £5 million pounds... Professional Indemnity Insurance = of not less than £2 million pounds" | Part 3 - Q22 (Lots 1-9) |
| Insurance Lot 10 (elevated) | "Employer's (Compulsory) Liability Insurance = £10m  Public Liability Insurance = £20m Professional Indemnity Insurance = £2m" | Part 3 - Q22 (Lot 10) |
| Insurance Lot 11 | "Employer's (Compulsory) Liability Insurance = £10,000,000  Public Liability Insurance = £5,000,000 Professional Indemnity = £2,000,000" | Part 3 - Q22 (Lot 11) |
| ISO 9001 standard | "Does your organisation have ISO 9001:2008 or equivalent certification for works or services relating to this contract?" | Part 3 - Q18 Quality Assurance |
| ISO 14001 standard | "Does your organisation have ISO 14001:2004 or equivalent certification for works/services relating to this contract?" | Part 3 - Q19 Environmental Qualification |
| SSIP scheme (CHAS / EXOR / ISO 45001) | "the most common standards used are; CHAS - EXOR Health and Safety - ISO 45001" | Part 3 - Q21 Health and Safety at Work |
| DBS requirement | "operatives assigned to work on the Contract on behalf of the Contractor or any sub contractors must be cleared under the Disclosure and Barring Service Scheme" | Part 3 - Q23 |
| "May apply" accreditations (Q28-Q36) | Building Regulations 2010, Gas Safe, NICEIC/NAPIT, BAFE/FIA, ACOP L8, BPCA/RSPH, EA Waste Carrier, UKAS/HSE Licensed, CSCS — all quoted verbatim from their respective question rows | Part 3 - Q28 through Q36 |
| Lot 6 consumables supply in scope | "purchase and supply of all consumables" | Summary of requirements - Lot 6 |
| Lot 11 named fittings | "installation of handrails, grab rails, half steps etc." | Summary of requirements - Lot 11 |
| Lot 10 portfolio (~360 props) | "The portfolio currently comprises approximately 230 Council-owned street properties, 2 x Hostels and 130 privately leased properties" | Summary of requirements - Lot 10 |

Source markdown integrity: SHA-256 `8b70398e1d2dba0ac5adad64b538b451f371358e824ae5de3b94c807a58ade6c` recorded in `extraction_metadata.source_markdown_hash`.
