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
