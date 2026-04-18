---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-f7798c8f-8fa1-47ce-8abd-8f81b3052516
session_id: session-semantic-c1e535bacd28dfc72e4d18e941b667a8855a094d7f3fe40c8a6f8cca3977dc2b
execution_mode: contract
started_at: 2026-04-17T19:33:48Z
completed_at: 2026-04-17T19:38:00Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 59
agent_claimed_source_url: https://procontract.due-north.com/RfxResponse/Index?rfxId=28b81c96-0929-f111-813c-005056b64545
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB (document is an English-language draft contract for a UK local authority)
agent_claimed_currency_state: GBP (charges stated exclusive of VAT; implicit GBP; no FX conversion performed)
---

## Summary

Processed one Docling-derived markdown extraction for `Appendix-7-Draft-Contract.pdf`
(document_extraction_id `5829ef4b-37da-41ad-ab69-0b371aef0308`, 59 pages,
64 Docling sections, no tables detected, 112,302 bytes of markdown) against the
`pdf-extraction/tender-specification` overlay. The document is the draft **Agreement
for Ely Library Services — Engage, Design and Fit Out Contract** between
Cambridgeshire County Council and `[SUPPLIER]`, i.e., a contract-form document
(Appendix 7) rather than a standalone technical specification.

The extraction emits a single authoritative `lot` (lot_number "1") matching the
notice-level single-lot structure, four `product_requirement_seeds` (furniture,
signage, carpet, design-engagement service), nine technical standards, and
seventeen shared evidence anchors. Because the operative Schedule 1
(Specification) body is stubbed in this draft and evaluation/scoring criteria and
CPV citations live in other documents in the tender pack, `evaluation_criteria`
and in-document `cpv_codes` are left empty and six explicit warnings are raised.
`extraction_confidence_bps` is set to 6800 to reflect that the draft contract
body is high-fidelity but the underlying specification referenced by
Schedule 1 is not in this payload.

## Steps Taken

1. **Located inputs.** Read the job parameters from the Execution Context block:
   `document_extraction_id=5829ef4b-37da-41ad-ab69-0b371aef0308`,
   `document_set_id=tds-pc-19574d11-0929-f111-813c-005056b64545`,
   `notice_id=pc-19574d11-0929-f111-813c-005056b64545`,
   `process_id=proc-pc-19574d11-0929-f111-813c-005056b64545`,
   `raw_document_id=raw-pc-19574d11-0929-f111-813c-005056b64545-088e...d495ad`,
   `stage_run_id=stage-run-57a5f226-b1b8-4236-9510-0821c0ebd20d-...`,
   `target_domain=lots`, `extraction_scope=full_document`.
2. **Fetched the Docling markdown** from `state.db/document_extractions` for the
   supplied `document_extraction_id`. Confirmed metadata: title
   `Appendix-7-Draft-Contract`, role `technical_specification`, page_count 59,
   section_count 64, table_count 0, extraction_policy_version `2026.04.12`,
   expected_overlays `["pdf-extraction/tender-specification"]`. Wrote the
   markdown to `/tmp/docling_extraction.md` (983 lines, 112,302 bytes) and
   computed `sha256:bc3b944c17736d82c70365e08a5134006e192948a566eaaf555c654e2439b7fa`.
3. **Cross-referenced process and notice metadata** (for cross-reference only):
   process title `25112 Library Services Engage, Design and Fit out Contract -
   Ely Library`, buyer `Cambridgeshire County Council`, notice
   `main_category = "39155000-3 - Library furniture | 79932000-6 - Interior
   design services"`. Existing `notice_lots` row shows a single-lot design-and-
   build contract — consistent with the document's BACKGROUND section.
4. **Walked the document structure.** Reviewed the Contents block listing
   clauses 1–44 and schedules 1–11 with page references, then read the
   BACKGROUND/Parties block, clauses 2–10 (operative scope and delivery),
   clause 11–29 (staffing, disputes, IP, bribery, modern slavery),
   clauses 31–44 (general provisions), and the Schedule 1–11 bodies at the
   end of the document.
5. **Identified operative scope.** Extracted the contract scope from
   BACKGROUND (A) — "inviting prospective suppliers to submit proposals for the
   provision of design, furniture, signage, and carpet to refresh the public
   library offer in Ely library" — and clauses 4.3–4.7 (design engagement,
   accessibility, RAMS, delivery/installation).
6. **Extracted requirements.** Pulled 19 lot-level requirements anchored to
   clauses 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 5.1, 5.2, 5.4, 6.1, 6.2, 7.9, 8.1,
   8.2, 8.4, 9.4 and 9.5.
7. **Extracted technical standards.** Nine technical standards anchored in the
   operative clauses: Best Industry Practice, dementia-friendly design
   principles, autism-friendly design principles, Authority Health and Safety
   Policy + applicable health and safety Law, site-specific RAMS, Authority's
   Climate Charter, applicable environmental Law (waste/hazardous materials),
   standard for electronic invoicing, and equality Law + equality and
   diversity policy.
8. **Extracted product-requirement seeds.** Created four seeds — furniture,
   signage, carpet (the three product categories named together in BACKGROUND
   and clause 4.6), plus one "design-engagement / interior design service"
   seed covering the design-engagement deliverables (clause 4.3 is the primary
   anchor, with accessibility attributes lifted from clause 4.4).
9. **Suppressed speculative fields.** Left `evaluation_criteria` and in-document
   `cpv_codes` empty because:
     - The document is the draft contract, not the ITT, so no scoring grid is
       present.
     - CPV codes `39155000-3` and `79932000-6` are on the notice metadata
       but are **not** repeated in the contract body; per the skill's
       "do not claim without anchor" rule I left the field empty rather than
       back-fill from the notice.
     - Both choices are explicitly documented in the warnings array
       (`EVALUATION_CRITERIA_NOT_IN_DOCUMENT`, `CPV_CODES_NOT_IN_DOCUMENT`).
10. **Flagged stub Schedule 1.** Emitted `SCHEDULE_SPECIFICATION_STUB` because
    the Schedule 1 Specification body is empty in this draft contract; the
    functional/technical specification lives in the specification pack that
    accompanies this Appendix 7 and should be processed separately.
11. **Flagged warranty requirements in a comment.** Emitted
    `WARRANTY_REQUIREMENTS_IN_COMMENT_ONLY` because the explicit
    "12-month installation workmanship warranty + manufacturer warranties for
    furniture, signage and carpet + warranty pack and care instructions at
    handover" text appears in a tracked-changes comment (`Commented [FB20]`),
    not in the operative text. Those attributes are surfaced on the seeds
    against clause 7.9(a) (the operative handover clause) rather than being
    fabricated as first-class agreed requirements.
12. **Wrote `result.json`.** Materialised the schema-valid payload with
    `target_domain = "lots"`, `extraction_scope = "full_document"`,
    `source_markdown_hash` pinned to the sha256 above, and
    `policy_version = "2026.04.12"`.
13. **Validated the payload.** Loaded
    `skills/pdf-extraction/tender-specification/result.schema.json` and ran a
    `jsonschema.validate` pass against `result.json` → valid.

## Decisions Made

- **Single lot.** Retained lot_number `1` and matched the notice-level
  single-lot structure because the draft contract scope is unitary (one
  integrated engage-design-and-fit-out service); no sub-lots or tranches are
  referenced. The existing `notice_lots` row was used for cross-reference only
  (per the skill's guidance) but is consistent with the document text, so no
  contradiction exists. Flagged with `SINGLE_LOT_INFERRED`.
- **Four product-requirement seeds rather than three.** Added an explicit
  `design_service` seed alongside the three physical goods (furniture,
  signage, carpet). Rationale: clause 4.3 makes design engagement a distinct
  deliverable with its own acceptance gate (clause 4.5 written design approval)
  and its own milestone (clause 9.4(a) "Design approval"), so from a product-
  candidate sourcing perspective it is a separate candidate class. The three
  physical-goods seeds share the same evidence anchor (clause 4.6) because the
  contract treats them as a single delivery obligation.
- **Empty `evaluation_criteria` / `cpv_codes`.** Deliberately empty with
  explicit warnings rather than back-filled from notice metadata. Reason:
  the skill's non-goals rule "Do not claim a lot split, product seed, or
  standard reference unless the evidence section cites the relevant anchor"
  is read to cover classification codes as well — the CPV codes live on the
  notice, not in the contract text, and downstream consumers know where to
  find them.
- **Warranty attributes attached to operative anchor.** The 12-month
  installation-workmanship warranty text is in a tracked-changes comment
  (`Commented [FB20]`), so I did not hoist it into a requirement line.
  Instead I anchored the weaker "manufacturer warranties documented at
  handover" statement to clause 7.9(a) (which is operative) and raised a
  warning so the daemon / reviewer knows the quantitative 12-month figure is
  proposed, not agreed.
- **Confidence = 6800 bps.** High confidence on lot scope, requirements and
  standards (these are fully supported by operative text and anchored), but
  dragged down by (a) the Schedule 1 spec body being absent, (b) template
  placeholders littering the document, and (c) evaluation-criteria / CPV
  fields empty. Per-field confidences in `field_confidences` reflect the
  split — 8500 for requirements, 2000 for evaluation_criteria and cpv_codes.
- **Evidence-anchor strategy.** Used Docling section headings (`clause N.M`,
  `Schedule N`) as `section_ref`. Populated `page_ref` from the page numbers
  the document's own Table of Contents assigns to each clause / schedule
  (Docling did not embed per-line page markers in the markdown; the TOC is
  the highest-fidelity in-document page mapping available). Where a
  page could not be resolved precisely from the TOC (e.g. for anchors inside
  long clauses), I used the TOC start page of the enclosing clause.
- **`extraction_scope = "full_document"`.** Single pass over the entire
  markdown; no chunking was required (payload fit comfortably inside the
  overlay budget, matching the SKILL.md "prefer a single-pass run" guidance).

## Warnings

Six warnings were emitted in `result.json`; each is repeated here for the
audit appendix:

1. `SCHEDULE_SPECIFICATION_STUB` — Schedule 1 (Specification) body is a stub
   in this draft contract; the real specification lives in a sibling document.
2. `EVALUATION_CRITERIA_NOT_IN_DOCUMENT` — the ITT / award grid is not in
   this document; `evaluation_criteria` is empty.
3. `CPV_CODES_NOT_IN_DOCUMENT` — CPV codes are on the notice metadata
   (`39155000-3`, `79932000-6`) but not restated in the contract body;
   `cpv_codes` is empty.
4. `TEMPLATE_PLACEHOLDERS_PRESENT` — many `[INSERT DATE]`, `[SUPPLIER]`,
   `[FIND A TENDER NUMBER]`, `[NUMBER]`, `[INSERT DETAILS]` placeholders
   remain; they are pending contractualisation, not extracted facts.
5. `WARRANTY_REQUIREMENTS_IN_COMMENT_ONLY` — the explicit 12-month
   installation warranty is in tracked-changes comment `[FB20]`, not the
   operative body.
6. `SINGLE_LOT_INFERRED` — single-lot structure inferred from the document's
   unitary scope; consistent with the existing `notice_lots` row.

No warnings of category `AGENT_OUTPUT_CONTRACT_INVALID`, `AGENT_SUSPECT_OUTPUT`,
`AGENT_SKILL_FAILED`, `AGENT_RUN_FAILED` or `AGENT_PARTIAL_FAILURE` were
raised; `outcome_class` is `success_with_warnings`.

## Evidence

All major claims in this manifest are anchored to observable evidence in the
Docling-extracted markdown (`source_markdown_hash:
sha256:bc3b944c17736d82c70365e08a5134006e192948a566eaaf555c654e2439b7fa`):

- **Lot scope (single lot, design-engage-and-fit-out):**
  - Front-matter title (page 1): `AGREEMENT RELATING TO ELY LIBRARY SERVICES -
    ENGAGE, DESIGN AND FIT OUT CONTRACT`.
  - `BACKGROUND (A)` (page 1): `inviting prospective suppliers to submit
    proposals for the provision of design, furniture, signage, and carpet to
    refresh the public library offer in Ely library`.
- **Design-engagement deliverables:**
  - Clause 4.3 (page 13): `undertake all design engagement activities …
    a) attending a site visit at Ely Library and carrying out stakeholder
    consultation; b) preparing initial design proposals …; c) incorporating
    one round of feedback from the Authority into revised design proposals`.
  - Clause 4.4 (page 13): `all design proposals comply with applicable
    accessibility standards, including dementia friendly and autism friendly
    design principles`.
  - Clause 4.5 (page 13): `must not commence manufacture, delivery or
    installation of any items until the Authority has provided written
    approval of the final design`.
- **Physical-goods delivery (furniture / signage / carpet):**
  - Clause 4.6 (page 13): `deliver and install all furniture, signage, carpet
    and any associated items … in accordance with the approved design`.
  - Clause 4.2(b) (page 13): `all goods, materials, standards and techniques
    used in providing the Services are of the best quality and are free from
    defects in workmanship, installation and design`.
- **Site-specific RAMS / first-floor access constraint:**
  - Clause 4.7 (page 13): `prepare and submit Risk Assessments and Method
    Statements (RAMS) specific to Ely Library, taking into account first-floor
    access constraints, delivery windows and any other site-specific
    requirements`.
- **Health & safety, environmental, equality standards:**
  - Clause 5.2 (page 14): `all applicable Law regarding health and safety …
    the Health and Safety Policy whilst at the Authority Premises`.
  - Clause 5.4 (page 14): `all applicable equality law (whether in relation
    to race, sex, gender reassignment, age, disability …)`.
  - Clause 6.1(a) (page 15): `comply with all applicable environmental Law,
    including in relation to waste disposal and the handling of hazardous and
    toxic materials`.
  - Clause 6.2(b) (page 15): `carbon-reduction measures consistent with the
    Authority's Climate Charter`.
- **Handover / warranty documentation (operative anchor for warranty
  attributes):**
  - Clause 7.9(a) (page 17): `full documentation of all materials,
    components, equipment and associated warranties, including manufacturer
    warranties, maintenance instructions, compliance certificates`.
- **Disaster recovery cadence:**
  - Clause 8.2 (page 18): `test the Disaster Recovery Plan on a regular basis
    (and, in any event no less than once every 12 months period from the
    Commencement Date)`.
  - Clause 8.4 (page 18): `implement the Disaster Recovery Plan if the
    Services are not available for more than 10 days`.
- **Invoicing / payment milestones:**
  - Clause 9.4 (page 19): `invoice the Authority only upon completion of the
    following milestones … a) Design approval; b) Delivery of goods to site;
    c) Completion of installation and handover`.
  - Clause 9.5 (page 19): `accept and process for payment any electronic
    invoice … in a form that complies with the standard for electronic
    invoicing`.
- **Schedule-1 stub (warning basis):**
  - Schedule 1 Specification header (page 45) with no operative body;
    subsequent `Commented [FB20]` lists 12-month installation warranty +
    manufacturer warranties + warranty pack / care instructions as a
    *proposed* amendment, not agreed text.
- **Notice-metadata cross-reference (used for context, not as primary
  evidence):**
  - `contract_notices.main_category = "39155000-3 - Library furniture |
    79932000-6 - Interior design services"` (from `state.db`, used only to
    confirm the process scope matches the extracted lot scope; these CPV
    codes are deliberately *not* injected into `lots[0].cpv_codes` because
    they are absent from the document body).
