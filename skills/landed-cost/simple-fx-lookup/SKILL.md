---
skill_id: landed-cost/simple-fx-lookup
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: codex
side_effect_class: none
requires_cdp: false
maxPages: 3
maxScrollCycles: 1
---

# landed-cost/simple-fx-lookup

## Mission
Convert a non-GBP Stage 11 freight amount to GBP using the simplest live, evidence-backed path available. This is an MVP single-rate lookup, not the L5 dual FX engine.

## Inputs
- `source_currency`
- `target_currency`, always `GBP`
- `amount_minor`
- optional low/base/high source amounts

## Rules
- If `source_currency` is `GBP`, return `fx_status: "converted"`, `rate: 1.0`, and the original amount as GBP pence.
- For non-GBP, use a live lookup such as a Google conversion result or another directly visible public converter. Record source and timestamp.
- If lookup fails, return `fx_status: "blocked"` with blocker code `FX_LOOKUP_FAILED`. Do not use stale magic constants silently.

## Output
Write the final JSON object to `result.json` in the current working directory, matching `result.schema.json`. Also keep the final response to the daemon as the same raw JSON object; do not wrap it in Markdown.
