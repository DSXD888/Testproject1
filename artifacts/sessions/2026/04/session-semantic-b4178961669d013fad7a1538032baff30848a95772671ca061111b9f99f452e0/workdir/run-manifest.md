---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-e4ee2175-2630-4c01-b177-d1c241a03273
session_id: session-semantic-b4178961669d013fad7a1538032baff30848a95772671ca061111b9f99f452e0
execution_mode: contract
started_at: 2026-04-18T07:28:00Z
completed_at: 2026-04-18T07:40:00Z
outcome_class: success
items_processed: 11
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: /FileUploads/ViewAttachment?attachmentId=fae2dc98-792e-f111-813c-005056b64545&entityTypeId=9b214c2e-2d7d-432b-bced-4df247225fb3&attachmentTypeId=Rfx
agent_claimed_fingerprint_state: not_applicable_contract_mode
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted lot structure, per-lot requirements, evaluation criteria, technical-standard hints,
and product-requirement seeds from a pre-Docling-extracted structural markdown packet for
document_extractions.id = `27750c9f-6fec-4f25-9ab5-0bd2c2ce895b`. The source document is
the PSQ (Procurement Specific Questionnaire) for London Borough of Bexley's framework
reference "6278 – Building Services Framework". Produced one `result.json` with
`target_domain = "lots"` and `extraction_scope = "full_document"` containing 11 lots
(matching the 11 lots named in the PSQ Summary of requirements) and 11 matching
product_requirement_seeds. Schema validation against
`skills/pdf-extraction/tender-specification/result.schema.json` passed.

Overall `extraction_confidence_bps = 7500` reflects:
- High confidence on lot identity (number, title, brief description) — directly quoted from the document.
- Medium confidence on technical standards per lot — the PSQ lists all accreditations (Q28–Q36)
  as "may apply" and defers them to the full specification; per-lot assignments are analytical.
- Low confidence on CPV codes — none appear in the PSQ; empty arrays returned with a dedicated warning.

## Steps Taken

1. Located the extraction packet: read the Docling-derived `extracted_markdown` column for
   `document_extractions.id = 27750c9f-6fec-4f25-9ab5-0bd2c2ce895b` from
   `state/state.db` (311 lines, 115,172 bytes).
2. Confirmed the companion metadata: `page_count = 0` (DOCX, so no paginated anchors),
   `table_count = 45`, `section_count = 14`, `extraction_engine = docling v2.88.0`,
   `source_content_hash = 179109ca…70280`, `policy_version = 2026.04.12`,
   `document_role = technical_specification`.
3. Confirmed the raw document identity: `raw_documents.id =
   raw-pc-dc1daee2-772e-f111-813c-005056b64545-b342d9c6fb7113cc57445623001f054213399aed2ef9d47619ad8b21f93c0dcc`,
   filename `6278 - PSQ - Procurement Specific Questionnaire (Competitive Flexible).docx`.
4. Parsed the "Summary of requirements" section: identified 11 sequentially numbered lots
   (Lot 1 – Lot 11), each with a bolded heading and a one-sentence description.
5. Parsed Question 13 to extract the anticipated annual value and group assignment per lot
   (Group 1 = Lots 1–7, Group 2 = Lots 6–11, with Lots 6 and 7 appearing in both groups).
6. Parsed Question 22 to extract per-lot insurance requirements. Lots 1–9 share a baseline
   (EL £5m / £10m at 100+ employees, PL £5m, PI £2m). Lots 10 and 11 have elevated
   requirements and are recorded verbatim.
7. Parsed Questions 14 / 12 / 27 / 18 / 19 / 20 / 21 for evaluation-criteria evidence
   (references, financial suitability, quality, environmental, equality, health & safety).
8. Parsed Questions 28–36 for technical-standard hints. Because each is introduced
   with "may apply" and deferred to the specification, these were attached to lots
   analytically (e.g. ACOP L8 → Lot 2, UKAS/HSE → Lot 3, BAFE/FIA → Lots 1/4/8,
   Gas Safe → Lots 8/10, NICEIC/NAPIT → Lots 8/10, BPCA/RSPH → Lot 6,
   CSCS → Lots 5/8/9/10/11) with a dedicated `STANDARDS_CONDITIONAL_MAY_APPLY` warning
   so downstream consumers treat the attachment as tentative.
9. Built 11 `product_requirement_seeds` (one per lot) keyed `lot-<n>-<slug>` with stable
   `seed_key`s so the daemon can map them to `product_candidates` rows without
   re-inventing identifiers on retry.
10. Populated evidence anchors. Since the underlying extraction has `page_count = 0`
    (DOCX source), anchors use `section-summary` + section header plus
    explicit question numbers (`Q12`, `Q13`, `Q14`, `Q22`, `Q29`, `Q30`, `Q31`, `Q32`,
    `Q33`, `Q35`, `Q36`). This is recorded in a `PAGE_COUNT_ZERO` warning.
11. Populated `extraction_metadata` with `target_domain = "lots"`,
    `extraction_scope = "full_document"`,
    `source_markdown_hash = 179109ca20e70c6fb170fc93b14745c2d4a1bfeac2b1f42a1ade52bb16770280`,
    `policy_version = 2026.04.12`, `document_role = technical_specification`.
12. Wrote `result.json` to the working directory and validated it against
    `skills/pdf-extraction/tender-specification/result.schema.json` with jsonschema
    — passed with no errors.

## Decisions Made

- **Target domain held to `"lots"`.** The skill contract explicitly forbids emitting
  separate rows for `"candidates"` or `"requirements"` target domains; product seeds
  were therefore embedded under `product_requirement_seeds` and per-lot
  `product_requirements`, not as a separate semantic target.
- **Extraction scope recorded as `"full_document"`.** Job parameters request
  `"extraction_scope":"full_document"` and the document fits comfortably in the
  overlay budget (~115 KB), so no chunk plan was produced.
- **CPV codes returned as empty arrays.** The PSQ does not carry CPV codes — these are
  expected to live on the contract notice or in the full technical specification.
  Rather than guess at matching CPVs (e.g. 71317210 for Fire Risk Assessment) with
  no anchor text in-document, all `cpv_codes` arrays are returned empty and a
  `CPV_CODES_NOT_PRESENT_IN_DOCUMENT` warning is emitted so a downstream skill
  (contract-notice ingestion / overlay pass) can attach CPVs.
- **Scoring matrix reference, not reproduction.** Evaluation criteria include
  "PSQ minimum weighted score threshold = 16" (explicit in-document) and "Scoring /
  Weighting Matrix (see PCID)" (explicit deferral to an external document).
  A `SCORING_MATRIX_EXTERNAL` warning flags that numeric weightings are not present
  in this document.
- **Technical standards attached analytically with conditional evidence.** Because
  the PSQ uses generic "may apply" language for Q28–Q36, each standard is attached
  to the lot(s) where its domain is obviously applicable (e.g. BPCA/RSPH → Lot 6
  cleaning with internal pest control) and the evidence anchor cites the question
  number rather than a lot-specific clause. A `STANDARDS_CONDITIONAL_MAY_APPLY`
  warning informs downstream consumers that these are tentative.
- **Group 1 / Group 2 constraint captured as a warning, not a per-lot requirement.**
  The constraint that applicants cannot mix Group 1 and Group 2 lots is a
  cross-lot participation rule (Question 13), not a per-lot requirement, so it is
  emitted as a dedicated `GROUP_CONSTRAINT_CROSS_LOT` warning rather than
  duplicated into each lot's `requirements` array.
- **Stable seed keys.** Seed keys follow the pattern `lot-<n>-<slug>`
  (e.g. `lot-1-fire-risk-assessment-service`) so that any retry, chunk resume, or
  downstream matching uses the same identifier.
- **Document identity as PSQ, not full specification.** A
  `DOCUMENT_IS_PSQ_NOT_FULL_SPECIFICATION` warning flags that detailed per-lot
  scope (volumes, SLAs, full technical standards) lives in separate specification
  documents (e.g. "Schedule 1" / PCID) and that pre-existing `notice_lots` rows
  for this process already contain richer lot-scope detail derived from those
  dedicated specifications.

## Warnings

See `result.json.warnings[]` for full details. Summary:

1. `CPV_CODES_NOT_PRESENT_IN_DOCUMENT` — no CPV codes in PSQ.
2. `SCORING_MATRIX_EXTERNAL` — numeric weightings deferred to PCID.
3. `STANDARDS_CONDITIONAL_MAY_APPLY` — Q28–Q36 accreditations use "may apply" language;
   per-lot attachments are tentative.
4. `PAGE_COUNT_ZERO` — DOCX source yielded `page_count = 0`; anchors use section refs.
5. `DOCUMENT_IS_PSQ_NOT_FULL_SPECIFICATION` — document is selection-stage questionnaire,
   not the full per-lot specification.
6. `GROUP_CONSTRAINT_CROSS_LOT` — Group 1 / Group 2 participation constraint (Q13)
   is a cross-lot rule, not a per-lot requirement.

## Evidence

All claims in this manifest are grounded in the following observable references
(present in the Docling-derived markdown for `document_extractions.id =
27750c9f-6fec-4f25-9ab5-0bd2c2ce895b`):

- **Document identity.** Header table:
  `Reference number and requirement | 6278 – Building Services Framework`,
  `Procurement Specific Questionnaire (PSQ) return date | 18th May 2026 at noon`.
  Raw-document row in `state.db`:
  `id = raw-pc-dc1daee2-772e-f111-813c-005056b64545-b342d9c6fb7113cc57445623001f054213399aed2ef9d47619ad8b21f93c0dcc`,
  `storage_path = /Users/alinaaitova/Desktop/Testproject1/artifacts/downloads/procontract/afb3b5dc-772e-f111-813c-005056b64545/6278 - PSQ - Procurement Specific Questionnaire (Competitive Flexible).docx`.

- **Lot enumeration (titles + brief descriptions).** Section `Summary of requirements`,
  bolded headings `**Lot 1: Provision of Fire Risk Assessments**` through
  `**Lot 11: Provision of Minor Disability Adaptations ...**`.

- **Anticipated annual values + Group 1 / Group 2 partitioning.** Question 13 table:
  `GROUP 1 ... Lot Number Service Anticipated Lot Value per annum £ ... 1 Provision of Fire
  Risk Assessments £25k per annum ... 7 Provision of Lift Services ... £40k per annum.
  GROUP 2 ... 6 ... 7 ... 8 ... £860k per annum. 9 ... £175k per annum. 10 ... £1.8m per
  annum. 11 ... £375k per annum.`

- **Minimum weighted score threshold.** Conditions of Participation block:
  `For each lot applied for, a minimum weighted score threshold of 16 will be applied
  when assessing PSQ's. Applications not meeting this minimum score threshold will fail.`

- **Scoring matrix deferral.** Scoring/Weighting Matrix block:
  `See the Procurement & Contract information Document (PCID) for the scoring matrix.`

- **References.** Question 14 body lists the Appendix 1 / Appendix 2 reference mechanism,
  the 2 references-per-lot target, the 3-year / 5-year-for-works window, and the
  18th May 2026 5pm return deadline.

- **Financial suitability.** Question 12 body: Experian Business Express red/amber/green
  + Council discretion; Question 27 body: annual turnover ≥ 2× estimated lot/lots value.

- **Insurance (baseline and uplift).** Question 22 rows: baseline EL/PL/PI values for
  Lots 1–9 verbatim; Lot 10: `Employer's (Compulsory) Liability Insurance = £10m ...
  Public Liability Insurance = £20m ... Professional Indemnity Insurance = £2m`;
  Lot 11: `Employer's (Compulsory) Liability Insurance = £10,000,000 ...
  Public Liability Insurance = £5,000,000 ... Professional Indemnity = £2,000,000`.

- **Technical-standard hints.** Questions 29 (Gas Safe), 30 (NICEIC / NAPIT), 31 (BAFE /
  FIA), 32 (ACOP L8 competent person), 33 (BPCA / RSPH), 34 (Waste Carrier Licence),
  35 (UKAS Surveyor / HSE Licensed Contractor), 36 (CSCS cards) — each introduced with
  "may apply" and deferred to the full specification.

- **Structural-extraction metadata.** `document_extractions.id =
  27750c9f-6fec-4f25-9ab5-0bd2c2ce895b`: `source_content_hash =
  179109ca20e70c6fb170fc93b14745c2d4a1bfeac2b1f42a1ade52bb16770280`, `page_count = 0`,
  `table_count = 45`, `section_count = 14`, `extraction_engine = docling`,
  `extraction_engine_version = 2.88.0`, `extraction_policy_version = 2026.04.12`,
  `document_role = technical_specification`.

- **Result artifact.** `result.json` written to
  `artifacts/sessions/2026/04/session-semantic-b4178961669d013fad7a1538032baff30848a95772671ca061111b9f99f452e0/workdir/result.json`
  and validated against
  `skills/pdf-extraction/tender-specification/result.schema.json` (jsonschema 2020-12)
  with no errors.
