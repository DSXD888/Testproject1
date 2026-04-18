---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-fa18e581-1085-4bdf-89a3-2d873f7f8311
session_id: session-semantic-2ef69e523a7c949cf68b1bce036929f1b8e08f9a04c91dd8848cb485c3340c29
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success
items_processed: 11
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: local_document_extraction:3908eae6-127e-4707-80ca-6749344f9499
agent_claimed_fingerprint_state: not_applicable_contract_mode
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Ran `pdf-extraction/tender-specification@1.0.0` against the Docling markdown for London Borough of Bexley PSQ `6278 – Building Services Framework` (`document_extraction_id=3908eae6-127e-4707-80ca-6749344f9499`, `raw_document_id=raw-pc-dc1daee2-...-cdf5970624cb...`, `source_content_hash=179109ca20e70c6fb170fc93b14745c2d4a1bfeac2b1f42a1ade52bb16770280`, `extraction_policy_version=2026.04.12`). Extraction scope `full_document`; `target_domain=lots` as required by the skill.

Extracted 11 lot records anchored to the explicit `Lot 1` … `Lot 11` headings under the `Summary of requirements` section. Each lot carries a title, description, requirements list, evaluation criteria (shared PSQ-threshold + reference + financial + insurance bases, plus lot-specific accreditation criteria), technical_standards drawn from the Q18 / Q19 / Q21 / Q28 – Q36 "may apply" questions, an evidence_anchor pointing at the lot heading, empty cpv_codes (none present in this PSQ), and lot-specific `product_requirements` only where the document explicitly names procurable products.

Four `product_requirement_seeds` were emitted, exclusively where the source contains unambiguous supply-or-install language: one for Lot 6 ("purchase and supply of all consumables") and three for Lot 11 ("installation of handrails, grab rails, half steps"). No seeds were emitted for Lots 8 or 7 despite mentions of boilers, fire alarms and lift refurbishment, because those items are framed as maintenance targets rather than distinct procurement items.

Outcome is `success`: the 11 expected lots were identified with high confidence (lot boundaries 9500 bps, titles 9500 bps, descriptions 9200 bps); gaps are concentrated in cpv_codes (1000 bps — absent from source) and product seeds (6000 bps — weak in a selection-questionnaire document). Overall `extraction_confidence_bps=7800`.

## Steps Taken

1. Confirmed workdir state: `storage.sqlite3*` present; no prior `result.json` or `run-manifest.md`.
2. Loaded daemon `state.db` at `/Users/alinaaitova/Desktop/Testproject1/state/state.db` and read `document_extractions.id='3908eae6-127e-4707-80ca-6749344f9499'` to retrieve the extracted markdown, `source_content_hash`, `extraction_policy_version`, `document_role`, `page_count`, and `expected_overlays_json`. Confirmed `expected_overlays_json` contains exactly `pdf-extraction/tender-specification`, matching the skill binding.
3. Read the 311-line markdown to `/tmp/tender_markdown.md` and inspected structure: title row `6278 – Building Services Framework`, PSQ return date `18th May 2026 at noon`, explicit `Summary of requirements` block enumerating `Lot 1` through `Lot 11`, `Conditions of Participation`, `PSQ Explainer`, `Supplier Incentive Programme`, and the `Q1` … `Q36` + `Confirmations` question grid.
4. Cross-referenced existing `notice_lots` rows for `notice_id=pc-dc1daee2-772e-f111-813c-005056b64545` — 11 lots with numbers 1-11 exist, matching the document boundaries. Used this only as a cross-check; all lot content was re-derived from the source markdown per the skill's `Inputs` note that prior lot rows are not authoritative.
5. Loaded `skills/pdf-extraction/tender-specification/result.schema.json` and derived the exact output shape, including the `$defs.evidenceAnchor` required-field set (`page_ref`, `anchor_text`).
6. For each of the 11 lots, extracted the verbatim lot-heading anchor (e.g. `Lot 1: Provision of Fire Risk Assessments`) plus the one-paragraph summary immediately following it. Normalised HTML entities (`&amp;` → `&`, `’` → `'`).
7. Parsed the Q13 grouping table (rendered as a single flattened cell by Docling) to recover per-lot anticipated annual value and Group 1 vs Group 2 membership: `Lot 1 £25k`, `Lot 2 £25k`, `Lot 3 £60k`, `Lot 4 £50k`, `Lot 5 £70k`, `Lot 6 £670k` (both groups), `Lot 7 £40k` (both groups), `Lot 8 £860k`, `Lot 9 £175k`, `Lot 10 £1.8m`, `Lot 11 £375k`.
8. Derived the shared evaluation criteria from the PSQ body: PSQ minimum weighted score of 16, two references per lot per Q14, Experian Business Express financial check per Q12, turnover ≥ 2× lot value per Q27, insurance levels from Q22 (Lots 1-9 uniform; Lot 10 = EL £10m / PL £20m / PI £2m; Lot 11 = EL £10,000,000 / PL £5,000,000 / PI £2,000,000), SSIP H&S pre-qualification per Q21.
9. Mapped "may apply" lot-specific accreditations (Q28–Q36) to the most-relevant lots: Q29 Gas Safe → Lot 8 + Lot 10 (boilers); Q30 NICEIC/NAPIT → Lot 8 + Lot 10; Q31 BAFE/FIA → Lot 1 + Lot 8; Q32 ACOP L8 → Lot 2 + Lot 10 (Legionella); Q33 BPCA/RSPH → Lot 6 (pest control); Q35 UKAS Surveyor / HSE Licensed Contractor → Lot 3 (asbestos); Q34 Waste Carrier Licence → Lot 3 (asbestos waste); Q36 CSCS cards → Lot 8 / Lot 9 / Lot 10 / Lot 11. Q18 (ISO 9001), Q19 (ISO 14001), Q21 (SSIP/ISO 45001/CHAS/EXOR), Q28 (Building Regulations 2010) retained as universal or quasi-universal where the lot's service nature supports it.
10. Generated four product_requirement_seeds (Lot 6 consumables + Lot 11 handrails / grab rails / half steps). Held back speculative seeds for Lot 7 (minor refurbishment parts), Lot 8 (boilers, fire alarms) and Lot 10 (compliance materials) because the source treats those as maintenance targets, not goods to procure. Each seed carries the verbatim anchor_text "purchase and supply of all consumables" or "installation of handrails, grab rails, half steps etc".
11. Set all per-lot `cpv_codes` to `[]` — no CPV codes are present anywhere in the PSQ. Logged `AGENT_MISSING_CPV`.
12. Emitted top-level `evidence_anchors` (20 entries) covering the framework title, Summary of requirements, Conditions of Participation, Q13, Q14, Q18, Q19, Q21, Q22, Q23, Q27, Q28-Q36.
13. Assigned `extraction_confidence_bps=7800` and per-field confidences (lot boundaries / titles 9500 bps, descriptions 9200 bps, values/groups 9000 bps, CPV 1000 bps, product seeds 6000 bps).
14. Validated `result.json` against `result.schema.json` with `jsonschema.validate` — passes.
15. Wrote `run-manifest.md` (this file) into the workdir with the mandated frontmatter + five narrative sections.

## Decisions Made

- **`target_domain="lots"` only**: Per the `Outputs` and `Non-goals` sections of SKILL.md, I did not emit separate `requirements` or `candidates` semantic rows. Product seeds are nested inside the `lots` payload and mirrored at the top-level `product_requirement_seeds` array, exactly as schema-required.
- **`extraction_scope="full_document"`**: Job parameters set `extraction_scope=full_document`, and the Docling markdown covers the full 45-table / 14-section PSQ artefact. No chunking was required; the document fits within single-pass budget, so no `chunk_merged:<plan_hash>` scope key was used.
- **Treat the PSQ as tender_spec but flag it**: `document_role=technical_specification` came through from the daemon. The content is structurally a Procurement Specific Questionnaire (selection stage), not a full works specification. I preserved the daemon-assigned role but logged `AGENT_PSQ_NOT_FULL_SPEC` so downstream validators don't expect measurable per-lot technical performance criteria. Per-lot technical_standards are therefore drawn from the "may apply" accreditation questions, which is the strongest standards signal the document actually carries.
- **Group-mutex as a warning, not a lot property**: The Q13 rule that "Group 1 applicants cannot apply for Group 2 and vice versa" is a bid-assembly constraint, not a lot attribute. Captured in the `AGENT_GROUP_CONSTRAINT` warning and mentioned in each affected lot's description field so that downstream consumers see it without polluting the schema.
- **Insurance level differentiation**: Preserved Lot 10's elevated `EL £10m / PL £20m` and Lot 11's `EL £10,000,000 / PL £5,000,000 / PI £2,000,000` as distinct `evaluation_criteria.criterion="Insurance levels (elevated ...)"` entries rather than folding them into the universal boilerplate.
- **Conservative product seed floor**: Lot 11 yields the clearest product seeds (supply and install handrails/grab rails/half steps) because the source uses installation-of-product language. Lot 6 yields a single combined "consumables" seed because the source says "purchase and supply" explicitly. I did NOT emit product seeds for Lot 7 ("minor refurbishment"), Lot 8 ("boilers, fire alarms") or Lot 9 ("gutters, boarding up"), because the source frames them as maintenance assets or repair actions rather than procurable goods. This matches the skill's rule "Do not claim … a product seed … unless the evidence section cites the relevant anchor."
- **CPV codes left empty rather than guessed**: The PSQ contains no CPV strings at all. Rather than infer plausible CPVs from lot titles (e.g. 71317200 for Fire Risk Assessments, 90900000 for Cleaning), I left `cpv_codes=[]` and logged `AGENT_MISSING_CPV`. CPV inference can be done downstream from `contract_notices` or the linked PCID, which is a separate document.
- **Evidence anchors use section/table references, not page numbers**: `document_extractions.page_count=0` for this row, so numeric `page_ref` values would be fabricated. I used `page_ref="section:…"` and `page_ref="table:…"` strings matching the document's own section/table hierarchy, and noted this in the `AGENT_PAGE_REFS_UNAVAILABLE` warning.
- **HTML entity normalisation**: Collapsed `&amp;` to `&` and `’` to `'` in stored descriptions so the output is human-clean; preserved the original wording otherwise.
- **Cross-reference only, never authoritative**: Existing `notice_lots` rows (prepopulated for this notice) matched the 11-lot structure, but all lot data was re-derived from the source markdown per SKILL.md guidance. No writes to `notice_lots` occurred from this agent; the daemon owns persistence.

## Warnings

1. `AGENT_MISSING_CPV` — No CPV codes present in the document. `cpv_codes` arrays are all empty; downstream must infer from notice context or the linked PCID.
2. `AGENT_CROSS_DOC_REFERENCE` — Scoring/weighting matrix is deferred to the Procurement & Contract Information Document (PCID) which is not part of this extraction; per-lot weighted scoring breakdowns beyond the `16 minimum` threshold are not present.
3. `AGENT_PSQ_NOT_FULL_SPEC` — Document is a Procurement Specific Questionnaire (selection stage) rather than a technical specification. Per-lot performance criteria, quantities and measurable targets are not present. Technical standards were derived from Q18, Q19, Q21 and the "may apply" Q28–Q36 questions.
4. `AGENT_GROUP_CONSTRAINT` — Q13 enforces a mutex: Group 1 lots (1-5, plus 6 and 7) cannot be bid alongside Group 2 lots (6-11, plus the shared 6 and 7). Surfaced as a warning for downstream assembly logic.
5. `AGENT_FLATTENED_TABLE` — Docling rendered the Q13 lot-grouping table as a single concatenated cell. Per-lot anticipated annual values were recovered textually from that cell; verify against the source PDF if exact values matter.
6. `AGENT_PAGE_REFS_UNAVAILABLE` — `document_extractions.page_count=0` for this row. All evidence anchors use section/table references rather than numeric page numbers; downstream consumers that expect `page_ref` to be a page integer should fall back to `section_ref`.

No `AGENT_OUTPUT_CONTRACT_INVALID`, `AGENT_SUSPECT_OUTPUT`, `AGENT_SKILL_FAILED`, or `AGENT_PARTIAL_FAILURE` conditions were encountered.

## Evidence

- **Lot boundaries**: 11 distinct `**Lot N:**` headings found verbatim under the `**Summary of requirements**` heading (markdown lines 9, 13, 17, 21, 25, 29, 33, 37 (italicised), 41, 45, 49). Each `lots[i].evidence_anchor.anchor_text` copies the exact heading string.
- **PSQ identity**: `**Reference number and requirement | 6278 – Building Services Framework**` (line 3). PSQ return date `**18th May 2026 at noon**` (line 5).
- **Conditions of Participation**: Six-point list (lines 55-60) gives pass/fail basis; the bold paragraph "For each lot applied for, a **minimum weighted score threshold of 16** will be applied when assessing PSQ's" (line 62) anchors the minimum-score evaluation criterion on every lot.
- **Q13 grouping and anticipated values**: Single flattened row (line 148) contains `GROUP 1 … Lot Number Service Anticipated Lot Value per annum £ … 1 Provision of Fire Risk Assessments £25k per annum. … 11 Provision of Minor Disability Adaptations (minor building works) in privately owned properties £375k per annum.` The mutex text `COMPANIES APPLYING FOR LOTS IN GROUP 1 CANNOT APPLY FOR LOTS IN GROUP 2 AND VICE VERSA` anchors the `AGENT_GROUP_CONSTRAINT` warning. Lots 6 and 7 appear in both Group 1 and Group 2 sections of the same row.
- **Q14 references**: `Complete Part A of the reference form provided at Appendix 1 for 2 different clients, for each lot which you have applied` (line 158); `References must be for current contracts or contracts completed within the last 3 years. (5 years for works contracts)`; `At least one reference must be received for each lot applied for to ensure your application is considered.`
- **Q18 / Q19 / Q21 universal standards**: ISO 9001:2008 (line 173), ISO 14001:2004 (line 184), SSIP schemes CHAS / EXOR / ISO 45001 (line 202).
- **Q22 insurance — Lots 1-9 uniform**: `Employer's (Compulsory) Liability Insurance = of not less than £5 million … Public Liability Insurance = for not less than £5 million … Professional Indemnity Insurance = of not less than £2 million`. Lot 10 (line 217): `EL £10m ; PL £20m ; PI £2m`. Lot 11 (line 218): `EL £10,000,000 ; PL £5,000,000 ; PI £2,000,000`.
- **Q23 DBS**: `operatives assigned to work on the Contract on behalf of the Contractor or any sub contractors must be cleared under the Disclosure and Barring Service Scheme` (line 224). Mapped to Lot 10 (residential) and Lot 11 (private residential adaptations).
- **Q27 turnover**: `Please self-certify that your company's annual turnover is at least twice the estimated value of the Lot or total value of Lot's for your which you have applied` (line 250).
- **"May apply" lot-specific accreditations**: Q28 Building Regulations 2010 (line 256), Q29 Gas Safe (line 262), Q30 NICEIC/NAPIT (line 268), Q31 BAFE/FIA (line 274), Q32 ACOP L8 (line 280), Q33 BPCA/RSPH (line 286), Q34 Waste Carrier Licence (line 292), Q35 UKAS Surveyor / HSE Licensed Contractor (line 298), Q36 CSCS cards (line 304). All appear as "may apply" clauses referring back to the tender specification for triggering conditions.
- **Product seeds**: Lot 6 `purchase and supply of all consumables` (line 31). Lot 11 `installation of handrails, grab rails, half steps etc` (line 51). These are the only two sites in the document that use "purchase / supply / install" language around named products.
- **No CPV strings**: A full-text scan for `CPV`, digits patterns like `\d{8}`, and the word "classification" finds nothing matching a CPV code anywhere in the 311-line markdown. The only cross-document reference for classifications is "See the Procurement & Contract information Document (PCID) for the scoring matrix" (line 66), which is a distinct document not provided to this run.
- **Result contract**: `result.json` written to `/Users/alinaaitova/Desktop/Testproject1/artifacts/sessions/2026/04/session-semantic-2ef69e523a7c949cf68b1bce036929f1b8e08f9a04c91dd8848cb485c3340c29/workdir/result.json`. Schema validated against `skills/pdf-extraction/tender-specification/result.schema.json` — passes. Top-level keys present: `lots` (11 items), `product_requirement_seeds` (4 items), `field_confidences` (13 keys), `extraction_confidence_bps=7800`, `warnings` (6 items), `evidence_anchors` (20 items), `extraction_metadata { target_domain=lots, extraction_scope=full_document, source_markdown_hash=179109ca…, policy_version=2026.04.12, document_role=technical_specification }`.
