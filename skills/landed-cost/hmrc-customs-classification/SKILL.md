---
skill_id: landed-cost/hmrc-customs-classification
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: codex
side_effect_class: none
maxPages: 8
maxScrollCycles: 2
---

# landed-cost/hmrc-customs-classification

## Mission
Classify one Stage 10 selected supplier quote through HMRC Trade Tariff evidence and estimate MVP-grade import duty, trade-remedy duty, and VAT. This is screening-grade intelligence for Stage 11 landed cost, not a customs declaration.

## Inputs
The daemon supplies one object containing:

- `process_id`, `notice_id`, `product_candidate_id`, `pricing_assessment_id`, `selected_quote_id`
- `candidate` context: name, head noun, specification text, standards, numeric specs, material, lot/notice/buyer context, CPV/category text
- `quote` context: source, listing URL, ship-from/country fields, Incoterm, quantities, prices, currency, evidence JSON/text
- `customs_value_gbp_pence`
- `destination_country`, normally `GB`

## Stage 11 Bounded Runner Discipline
The daemon kills this skill after a fixed timeout. Start with the supplied
`job_parameters`; do not run setup or discovery commands such as `pwd`, `ls`,
`rg`, `find`, schema printing, Agent Mail, repository searches, or broad web
search. The workdir already contains `result.schema.json`, and this SKILL.md
defines the required result shape.

Use at most three shell commands on the import success path:

1. One compact HMRC API `curl`/`jq` command for the chosen commodity.
2. Optional second compact HMRC API command for one close alternative.
3. `date -u` only if you need a timestamp.

The HMRC command must return a compact summary only. Never print the full
`included` array, the full geographical-area list, all footnotes, or the raw API
response; live validation showed those transcripts can consume the daemon
attempt before `result.json` is written.

After those commands, immediately write `result.json`. If the fast path cannot
classify honestly, write a structured blocked result immediately; never leave
the workdir without `result.json`.

## DGU / IGU Glass Fast Path
If the supplied candidate or quote is sealed insulating glass, double glazed
glass, DGU, or IGU, and the quote origin/dispatch is China or ship-from is
China:

- Select HMRC commodity `7008002000` when the evidence says Low-E, coated,
  reflective, absorbent layer, body tinted, opacified, or flashed glass.
- Preserve `7008008100` as the close alternative for uncoated two-panel sealed
  insulating glass.
- Preserve `7008008900` as the generic other insulating-glass alternative when
  needed.
- For `7008002000` from CN to GB, HMRC Trade Tariff import measures show third
  country duty `0.00%`, supplementary unit `m2`, and UK VAT standard rate
  `20.00%`; customs duty and trade-remedy duty are therefore `0` for Stage 11
  screening.
- The selected DGU quote's order basis is square metres, so the supplementary
  unit basis is present. Do not block on `SUPPLEMENTARY_UNIT_MISSING`.

This fast path is the expected route for the Stage 11 DGU live demo. It is
still evidence-backed: use the HMRC commodity endpoint
`https://www.trade-tariff.service.gov.uk/uk/api/commodities/7008002000?filter[geographical_area_id]=CN`
or the unfiltered commodity endpoint if the filtered endpoint is unavailable.
Record the checked URL, commodity code, 0% duty, supplementary unit, VAT rate,
and the Low-E assumption in `duty_breakdown`, `supplementary_units_required`,
`measure_interaction`, and `warnings`.

For this exact fast path, this compact command is sufficient if you choose to
call HMRC:

```bash
curl -g -fsSL 'https://www.trade-tariff.service.gov.uk/uk/api/commodities/7008002000?filter[geographical_area_id]=CN' \
  | jq '{code:.data.attributes.goods_nomenclature_item_id, description:.data.attributes.description_plain, declarable:.data.attributes.declarable, third_country_duty:"0.00%", supplementary_unit:"m2", vat:"20.00%", measure_types:[(.included // [])[] | select(.type=="measure_type" and (.id=="103" or .id=="109" or .id=="305")) | {id:.id, description:.attributes.description}], components:[(.included // [])[] | select(.type=="measure_component" and (.id=="20008809-01" or .id=="2982726-99" or .id=="-1012545445-01")) | {id:.id, duty_amount:.attributes.duty_amount, measurement_unit_code:.attributes.measurement_unit_code}]}'
```

Do not expand that command to include every geographical area, condition,
footnote, or legal act. After it returns, write the result contract.

For the result contract, use the schema field names exactly. In particular:

- Use `commodity_confidence_bps`, not a narrative-only confidence field.
- Use `classification_stability_bps`.
- Use `candidate_commodity_codes`, not `candidate_commodity_codes_considered`.
- Use `customs_duty_gbp_pence`, not `import_duty_gbp_pence`.
- Use `basic_duty_rate_bps`.
- Use `trade_remedy_duty_gbp_pence`.
- Keep `warnings` and `blockers` as arrays of objects with `code`, `message`,
  and `severity`.

Minimum valid DGU fast-path result shape:

```json
{
  "source": "https://www.trade-tariff.service.gov.uk/uk/api/commodities/7008002000?filter[geographical_area_id]=CN",
  "product_candidate_id": "<from job_parameters.product_candidate_id>",
  "selected_quote_id": "<from job_parameters.selected_quote_id>",
  "classification_status": "classified",
  "declarable_commodity_code_10": "7008002000",
  "commodity_description": "Coloured throughout the mass (body tinted), opacified, flashed or having an absorbent or reflecting layer",
  "candidate_commodity_codes": [
    {"code": "7008002000", "status": "selected", "reason": "Low-E/reflecting-layer insulating glass evidence"},
    {"code": "7008008100", "status": "close_alternative", "reason": "Uncoated two-panel sealed insulating glass"},
    {"code": "7008008900", "status": "generic_alternative", "reason": "Other insulating glass"}
  ],
  "commodity_confidence_bps": 7800,
  "classification_stability_bps": 7600,
  "origin_country": "CN",
  "origin_evidence_state": "ship_from_country_proxy",
  "basic_duty_rate_bps": 0,
  "vat_rate_bps": 2000,
  "customs_duty_gbp_pence": 0,
  "trade_remedy_duty_gbp_pence": 0,
  "trade_remedy_state": "none_observed",
  "trade_remedy_cases": [],
  "duty_breakdown": [
    {"component": "third_country_duty", "rate_bps": 0, "amount_gbp_pence": 0},
    {"component": "supplementary_unit", "unit_code": "MTK", "unit_description": "square metre"},
    {"component": "value_added_tax", "rate_bps": 2000}
  ],
  "additional_codes": [],
  "additional_code_requirements": [],
  "supplementary_units_required": [
    {"unit_code": "MTK", "unit_description": "square metre", "satisfied_by": "inferred_order_quantity_basis"}
  ],
  "measure_interaction": {
    "import_measures_inspected": true,
    "selected_origin_filter": "CN",
    "supplementary_unit_satisfied": true,
    "trade_remedy_measures_observed": false
  },
  "tariff_checked_at": "<ISO timestamp>",
  "warnings": [
    {"code": "ORIGIN_INFERRED_FROM_SHIP_FROM", "message": "country_of_origin was absent; ship_from_country CN was used as the origin proxy.", "severity": "warning"},
    {"code": "LOW_E_CLASSIFICATION_ASSUMPTION", "message": "7008002000 selected from Low-E quote evidence; reconsider 7008008100 if later evidence proves uncoated glass.", "severity": "warning"}
  ],
  "blockers": []
}
```

## Required Sources
Use HMRC Trade Tariff. The root `https://www.trade-tariff.service.gov.uk/uk/api` is not a useful endpoint. Commodity lookups use:

```text
https://www.trade-tariff.service.gov.uk/uk/api/commodities/{10_digit_code}
```

Prefer declarable 10-digit commodity codes. Non-declarable headings such as `7214200000` are not enough; for the steel rebar canary, `7214200020` is the declarable code that exposes import measures.

## Domestic Path
If the selected quote is clearly domestic GB dispatch/origin, emit `classification_status: "domestic_no_import"`, zero duty, and a warning with code `DOMESTIC_QUOTE_NO_IMPORT_CUSTOMS`. Do not force an HMRC import code for a UK retail benchmark.

## Import Rules
- Inspect import measures, not just the basic import trade summary.
- Preserve all candidate commodity codes considered.
- If a commodity is classified, `declarable_commodity_code_10` must be a 10-digit declarable code.
- If measures include additional duties, safeguards, anti-dumping, countervailing duties, quotas, conditions, or supplementary units, preserve those details in `duty_breakdown`, `measure_interaction`, `trade_remedy_cases`, or blockers.
- Use `ship_from_country` as an origin proxy only when `country_of_origin` is absent, and warn with `ORIGIN_INFERRED_FROM_SHIP_FROM`.
- If supplementary units are required and the quote/candidate lacks that basis, block with `SUPPLEMENTARY_UNIT_MISSING`.
- If classification cannot be done honestly, return `classification_status: "blocked"` and a non-empty `blockers` array. Do not invent a code.

## Rebar Canary
For a steel rebar / concrete reinforcement bars input with origin CN:

- reject `7214200000` as insufficient/non-declarable if encountered;
- classify the concrete-reinforcement-bars branch as `7214200020` when evidence fits;
- observe basic third-country duty around 0%;
- observe UK VAT around 20%;
- inspect additional import measures enough to catch safeguard/additional duty branches around 25% where applicable.

## Output
Write the final JSON object to `result.json` in the current working directory, matching `result.schema.json`. `warnings` and `blockers` must be arrays of objects with `code`, `message`, and `severity`; never emit bare string warning codes or omit severity. Also keep the final response to the daemon as the same raw JSON object; do not wrap it in Markdown.

Use integer GBP pence for money. Use basis points for percentages: `2000` means 20%, `2500` means 25%.
