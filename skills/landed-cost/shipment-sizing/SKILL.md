---
skill_id: landed-cost/shipment-sizing
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: codex
side_effect_class: none
requires_cdp: true
cdp_default_port: 9222
maxPages: 8
maxScrollCycles: 3
---

# landed-cost/shipment-sizing

## Mission
Resolve the shipment basis required before any freight quote: units, total weight in kg, total cubic metres, package count, and LCL/FCL/container plan.

## Inputs
The daemon supplies the same candidate and selected quote context used by the HMRC skill, plus the selected supplier cost basis and `inferred_order_quantity_basis`.

`inferred_order_quantity_basis` is the operational shipment quantity. It has `kind`, `qty`, `unit`, and `source` fields. Size freight from this field, not from `quote.quoted_quantity`, unless `kind == "supplier_capacity"`. Treat `quote.quoted_quantity` as supplier listing tier/capacity evidence unless the daemon explicitly selected it as the order basis.

## Evidence Priority
1. Selected quote evidence fields.
2. Supplier/listing URL if present.
3. Persistent CDP browser inspection for Alibaba or marketplace listings.
4. Candidate specification text and numeric specs.
5. `inferred_order_quantity_basis`.
6. Quote quantity/MOQ/listing pack fields only as supporting evidence.

If CDP is required, verify the persistent browser first:

```bash
CDP_PORT=${AGENT_BROWSER_CDP_PORT:-9222}
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

Use `agent-browser --cdp ${CDP_PORT}` for all browser actions. Do not launch a fresh browser, log in, contact suppliers, or mutate external state.

## Rules
- Do not guess precision. If listing/spec evidence lacks dimensions or weight, block with `SHIPMENT_DIMENSIONS_MISSING`.
- If a quote gives square metres but no thickness/packing/unit mass, inspect source evidence or block.
- For flat glass, sealed insulating glass units, or DGU/IGU products quoted per square metre, a listing/spec thickness is sufficient to create a screening-grade size estimate even when Alibaba's "single item" package fields are ambiguous. Derive glass mass from observed pane thicknesses only, using glass density 2,500 kg/m3; ignore air/argon cavity thickness for weight, but include full unit thickness for volume. Example: `6mm+9A+6mm` means 12 mm of glass for weight and 21 mm total unit thickness for geometric volume.
- When using the flat-glass physics estimate, set `sizing_status: "sized"`, include the formula and source evidence, set confidence below 7000 unless packing/crating evidence maps directly to the quote basis, and warn with `FLAT_GLASS_PHYSICS_ESTIMATE` plus `PACKING_SCHEDULE_MISSING`. Do not use ambiguous "single item" package size/weight as the basis unless the listing explicitly maps it to one square metre or one finished unit.
- For a large square-metre order, create an FCL container plan from total weight/CBM. Use conservative planning assumptions in `container_plan`: 40 ft container payload about 26,000 kg, usable volume about 67 CBM, and add a crating/handling volume uplift when no packing schedule is supplied. Split by weight when weight requires more containers than volume.
- If CDP opens the exact listing URL but the page is captcha-blocked, add warning `CDP_CAPTCHA_INTERCEPTION`. You may continue only when deterministic cached/indexed evidence for the same listing URL is available in the supplied quote/search evidence. In that case add an `evidence` object with `kind: "indexed_listing_evidence"` or `kind: "search_attempt_evidence"`, the exact `source_url`, and `deterministic_captcha_fallback: true`. If no same-URL deterministic fallback exists, return `sizing_status: "blocked"` with blocker `CDP_CAPTCHA_BLOCKED`.
- Choose LCL for genuinely small shipments, FCL when volume/weight requires containers.
- Record load-factor/container heuristics as warnings or in `container_plan.reasoning`.
- If weight exceeds practical payload before volume fills a container, split by weight.

## Output
Write the final JSON object to `result.json` in the current working directory, matching `result.schema.json`. `warnings` and `blockers` must be arrays of objects with `code`, `message`, and `severity`; never emit bare string warning codes or omit severity. Also keep the final response to the daemon as the same raw JSON object; do not wrap it in Markdown.
