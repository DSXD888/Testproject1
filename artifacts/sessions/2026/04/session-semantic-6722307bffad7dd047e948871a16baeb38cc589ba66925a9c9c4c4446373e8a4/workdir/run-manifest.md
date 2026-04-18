---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-3d0a4928-b13c-4940-92f4-333ea4f8442e
session_id: session-semantic-6722307bffad7dd047e948871a16baeb38cc589ba66925a9c9c4c4446373e8a4
execution_mode: contract
started_at: 2026-04-18T03:48:00Z
completed_at: 2026-04-18T03:51:20Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Processed one tender-specification document (`document_extraction_id=304720e2-6176-4235-a638-8acf907812b3`, `source_content_hash=648d25bde0724fb1deecdee81409f64ae424bfd985ba0f056472f754e4a7ac18`) covering the London Borough of Bexley "Fire Risk Assessment Specification" (Framework 6278, Lot 1). Emitted one authoritative lot record (Lot 1 — Provision of Fire Risk Assessments) with 28 requirements, 5 evaluation-related criteria, 7 product-requirement seeds, and 17 explicit technical standards. No lots other than Lot 1 are described in the document body; the extraction therefore emits only Lot 1 and leaves the remaining notice-level lots (2-11) to their own extractions. `extraction_confidence_bps=8100`. Headline signals the daemon should rely on: five warnings (CPV not in document body, evaluation scoring lives in ITT not spec, property schedule referenced but not embedded, duplicate section heading 3.3 preserved as disambiguated refs, pricing out of scope).

## Steps Taken

1. Loaded the Docling-derived markdown from the daemon's local state store for `document_extractions.id = 304720e2-6176-4235-a638-8acf907812b3` (26,459 bytes, 535 lines, `document_role = technical_specification`, 23 sections, 1 table, 0 images relevant to lot-level semantics).
2. Cross-referenced existing `notice_lots` rows for the parent notice `pc-dc1daee2-772e-f111-813c-005056b64545`. Confirmed 11 notice-level lots exist, but only Lot 1 ("Provision of Fire Risk Assessments") is the subject of *this* document. Other lots (2-11) are sourced from separate specification documents.
3. Walked every heading level 1/2/3 in the markdown to identify lot boundaries, requirement clusters, and evidence anchors. Recorded section labels with disambiguation where the same numeric heading (3.3) appears twice in the Docling output.
4. Extracted every explicit standard, Act, Regulation, PAS, BS and UKAS-accredited certification reference from §1 (Legislative framework), §3.3 (Competence and accreditation), §3.6 (Emergency lighting and alarms), §9.1 (Legislative Framework), §9.5 (Equality and Accessibility), §9.10 (Incident Reporting) and §16 (Mobilisation).
5. Identified priceable FRA service "seeds" (standard FRA, intrusive FRA, PCFRA, FRAEW to PAS 9980, residential duties under FSR 2022, emergency-lighting review, alarm-system review) with one evidence anchor per seed.
6. Captured implicit tender-submission evaluation inputs (sample FRAs, complaints procedure, environmental policy, KPI framework, innovation proposals) as `evaluation_criteria` entries with evidence anchors; flagged that explicit scoring weights / pricing are not present in this document and belong to the ITT / Price Matrix artefacts.
7. Computed `field_confidences` per schema (0-10000 bps) and an overall `extraction_confidence_bps=8100`, lowered by absence of CPV codes and evaluation scoring from the specification body.
8. Wrote `result.json` to the job working directory and validated it against `skills/pdf-extraction/tender-specification/result.schema.json` (`jsonschema.validate` passed).
9. Produced this run manifest for daemon finalisation.

## Decisions Made

- **Only Lot 1 emitted.** The document title, legislative framing and entire body describe the Fire Risk Assessment scope. Emitting a Lot 2-11 from this payload would violate the "do not treat pre-existing lot rows as authoritative truth if the tender document contradicts them" instruction in the SKILL — those lots are not described here, so emitting them would be manufactured content. The daemon retains its existing `notice_lots` rows for lots 2-11 sourced from other extractions.
- **`lot_number = "1"`.** The document is captioned "Lot 1" via its filename ("6278 - Draft Specification - Lot 1 Provision of Fire Risk Assessments") and aligns with `notice_lots.lot_number = 1` for the parent notice. The specification body itself does not restate the lot number, but both the document title and the contract reference Framework 6278 Lot 1.
- **CPV codes left empty.** No CPV codes are cited in the specification body. The skill prompt notes CPV signals already present on the *process* may be pre-existing; carrying them into the lot extraction would duplicate authority. The daemon can resolve CPV at the notice layer. A `CPV_CODES_NOT_PRESENT_IN_SPECIFICATION` warning is emitted.
- **Evaluation scoring not fabricated.** The document is a Service Specification (Schedule 1-class), not an ITT scoring matrix. Explicit `weighting_text` was not invented; `evaluation_criteria` entries describe the *submission obligations* the specification imposes on tenderers (sample FRAs, complaints procedure, environmental policy, KPI framework, innovation proposals). An `EVALUATION_SCORING_NOT_IN_DOCUMENT` warning flags the boundary.
- **Pricing declared out of scope.** Appendix 1 (Price Matrix) and Appendix 1.2 (Property Schedule) are referenced but not embedded in the markdown. Pricing extraction belongs to `pdf-extraction/pricing-schedule`; BoQ extraction is not applicable. Warned accordingly.
- **Duplicate 3.3 heading disambiguated.** The Docling output contains two sections labelled 3.3 ("The Property Schedule" and "Competence and accreditation"). Preserved both using disambiguated `section_ref` values (`section-3.3-property-schedule`, `section-3.3-competence`) to keep evidence anchors round-trippable. Warning `DUPLICATE_SECTION_HEADING_3.3` emitted.
- **Standards list kept maximally evidenced.** Every standard in `technical_standards` has a verbatim `anchor_text` drawn from the markdown. BAFE SP205 and the IFE Assessor Register are included as standards-equivalent certifications because they act as gating accreditations for the Supplier and individual assessors.
- **Service-oriented product seeds.** This is a services contract; `product_requirement_seeds` describe the priceable FRA activities (Type 1-4, PCFRA, FRAEW, residential duties, emergency-lighting / alarm reviews) rather than physical goods. This matches the SKILL's intent that seeds feed `product_candidates` writes without mutating authoritative rows.
- **`page_ref` uses section tokens.** The Docling markdown does not preserve PDF page numbers inline. Section-based `page_ref` values (e.g., `section-3.4`, `document-header`) were used — schema requires the field as a non-empty string, not a numeric page. Downstream review tooling can still navigate by section heading, and the daemon appendix holds the original content hash for forensic re-rendering.
- **Locale/currency.** Document is en-GB; all monetary provisions refer to the Council paying the Supplier, implying GBP. Recorded in frontmatter.

## Warnings

| Code | Summary | Evidence |
|------|---------|----------|
| `CPV_CODES_NOT_PRESENT_IN_SPECIFICATION` | No CPV codes in spec body; resolve from notice/process metadata. | §1 Legislative framework; §3.1 Scope. |
| `EVALUATION_SCORING_NOT_IN_DOCUMENT` | Specification does not contain scored evaluation weights or pricing. | §2.3; §3.3 Property Schedule. |
| `PROPERTY_SCHEDULE_REFERENCED_NOT_EMBEDDED` | Appendix 1.2 property schedule is referenced but its contents are not present in this extraction. | §3.3 Property Schedule. |
| `DUPLICATE_SECTION_HEADING_3.3` | Two sections labelled 3.3; disambiguated section_refs used. | `section-3.3-property-schedule` and `section-3.3-competence`. |
| `PRICING_SCHEDULE_OUT_OF_SCOPE` | Pricing to be extracted by `pdf-extraction/pricing-schedule`; not handled here. | §2.3 Appendix 1 reference. |

No blocking errors. `outcome_class = success`.

## Evidence

All major claims cite the markdown directly. Anchor map (selected):

- **Lot identity / Lot 1 scope.** Document header ("London Borough of Bexley - Fire Risk Assessment - Specification - Contract Commencement April 2026"); §1 "This Service Specification sets out the requirements for Fire Risk Assessments (FRAs) across the Council's operational estate."; §3.1 "The Supplier shall deliver all fire safety services in strict accordance with the Regulatory Reform (Fire Safety) Order 2005..."
- **Contract term.** §1 "The contract will be for a period of 2 years with an option to extend for a further period of up to 12 months to a maximum term of 3 years".
- **Statutory framework.** §1 Legislative framework list (RRFSO 2005 as amended by s.156 BSA 2022; Fire Safety Act 2021; Fire Safety (England) Regulations 2022; PAS 9980:2022; BS 9991:2024; BS EN 1838:2024; BS 8674:2025; BS 9792:2025).
- **FRA types (Type 1-4 + PCFRA).** §3.2 "align the survey type to risk using the established Type 1-4 approach ... and Person-Centred FRAs (PCFRA) for residents requiring support".
- **FRAEW.** §3.2 "determine whether a PAS 9980:2022 FRAEW is required and coordinate any specialist inputs."
- **Residential duties.** §3.5 "fire door checking regimes (quarterly common parts; annual flat doors), and, for high-rise buildings, arrangements for information boxes, floor plans, lift checks and wayfinding signage."
- **Emergency lighting / alarms.** §3.6 "Emergency lighting design/maintenance evidence should reference BS EN 1838:2024 ... fire detection and alarm systems should reference BS 5839 (relevant parts). Where strategies rely on evacuation lifts or EVC, align with BS 9991:2024 recommendations."
- **Competence / accreditation.** §3.3 Competence "Individual assessors must hold recognised Fire Risk Assessment qualifications and be listed on the IFE Assessor Register; Company must hold BAFE SP205 certification or equivalent UKAS-accredited scheme; Compliance with BS ISO 19706:2007"; "Associate Membership of IFE alone is not sufficient."
- **Report content & SLA.** §3.4 "Reports shall be issued within 10 working days of site completion ... Format must be compatible with TF Cloud."
- **Sample-FRA submission (evaluation input).** §3.4 "As part of the tender response the supplier should submit anonymised fire risk assessments for both a standard FRA and intrusive FRA with explanatory notes to explain their process".
- **Complaints procedure submission.** §15.1 "The Supplier shall implement a formal complaints procedure to ensure effective resolution. This procedure must be submitted with the tender response."
- **Environmental policy submission.** §11.1 "The Supplier is invited to submit a copy of their Company Environmental Policy, including: Practical examples of how the policy has been implemented; A summary of environmental, financial, and social benefits achieved; Alignment with the Council's Social Value priorities".
- **KPI framework.** §7 "Quarterly contract performance partnership meetings to review: Key Performance Indicators (KPIs); Health & Safety compliance; Overall contract performance".
- **Mobilisation & BS 9792 alignment.** §16 "During mobilisation agree: report templates (including alignment to BS 9792 where applicable), resource plan, programme, KPIs, data fields for TF Cloud, and information exchange protocols (including Section 156 FSO record-keeping and information-sharing duties)".
- **Innovation / continuous improvement.** §17 "Propose technologies and methods that improve value, compliance assurance, and safety (e.g., digital door registers, QR-coded assets, photo-verified remedials)."
- **Compartmentation surveys.** §2.6 "Where instructed by the Council, the Supplier shall support fire compartmentation surveys, which may involve intrusive building fabric works. ... Ensure access to roof spaces and concealed areas for assessment."
- **Asbestos / CAR 2012.** §2.5 "No Asbestos Containing Material (ACM) shall be disturbed or removed without prior written agreement from the Council and in full compliance with the Control of Asbestos Regulations 2012."
- **H&S framework.** §9.1 legislative list (HSWA 1974, Fire Safety Act 2021, Building Safety Act 2022, CAR 2012, Equality Act 2010).
- **RIDDOR.** §9.10 "Maintain a full understanding of RIDDOR 2013 requirements; Ensure timely and accurate reporting of all reportable incidents".
- **Authorised Person / DRA.** §10.1-§10.2 "This includes fulfilling the duties of the 'Authorised Person' ... completion of a Dynamic Risk Assessment is mandatory and forms a core compliance requirement under this Contract."
- **TF Cloud / CAFM integration.** §4 "The Council operates TF Cloud for instructions, updates, document upload and invoicing."; §5 "Use TF Cloud for recording completions, dates/times and uploading reports. Applications for payment must be submitted via the system on a monthly cycle; only completed jobs with required documentation will be authorised for payment."
- **Invoice routing.** §8 "All invoices must be sent electronically to: fmcu.buildingservices@bexley.gov.uk (and not to Central Payments)."

The full set of evidence anchors (18 records) is carried in `result.json#/evidence_anchors` and referenced per-field in each lot and seed record.
