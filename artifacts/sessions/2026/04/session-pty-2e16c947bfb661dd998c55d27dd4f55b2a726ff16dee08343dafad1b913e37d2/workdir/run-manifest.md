---
manifest_version: "1.0.0"
skill_id: "landed-cost/hmrc-customs-classification"
skill_version: "1.0.0"
job_id: "job-stage11-landed_cost_customs_classification-236573e24e2a4b93ac0c710c3bbd18dc"
session_id: "session-pty-2e16c947bfb661dd998c55d27dd4f55b2a726ff16dee08343dafad1b913e37d2"
execution_mode: "pty"
started_at: "2026-04-30T21:01:07Z"
completed_at: "2026-04-30T21:03:06Z"
outcome_class: "classified"
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 3
agent_claimed_pages_visited: 3
agent_claimed_source_url: "https://www.trade-tariff.service.gov.uk/uk/api/commodities/7008002000"
agent_claimed_fingerprint_state: "HMRC Trade Tariff API JSON fetched live; no browser fingerprint captured"
agent_claimed_locale_state: "UK Trade Tariff API, destination GB"
agent_claimed_currency_state: "GBP pence from daemon input"
---

## Summary

Classified the selected China-dispatched supplier quote for Low-E double-glazed insulating glass as HMRC declarable commodity code `7008002000`. The result contract was written to `result.json`.

## Steps Taken

- Read the daemon-supplied product and quote context: sealed insulating glass unit, BS EN 1279 context, Alibaba quote title containing "Low E", ship-from country `CN`, no explicit country of origin, and customs value `5160` GBP pence.
- Checked HMRC Trade Tariff API candidates for multiple-walled insulating glass: `7008002000`, `7008008100`, and `7008008900`.
- Inspected import measures on the selected HMRC commodity endpoint, including third-country duty, supplementary unit, VAT, additional duties, and restriction measures.
- Produced `result.json` with structured warnings and no blockers.

## Decisions Made

- Selected `7008002000` because the quote title's "Low E" evidence indicates an absorbent or reflecting layer. The generic sealed two-panel code `7008008100` was preserved as an alternative candidate.
- Used `CN` as the origin proxy because `country_of_origin` was absent and `ship_from_country` was present.
- Estimated basic customs duty at `0` pence because HMRC measure `20008809` shows third-country duty `0.00%` for ERGA OMNES.
- Estimated trade-remedy duty at `0` pence for CN because no China-specific additional-duty or restriction measure was found on the selected commodity. Non-applicable additional-duty branches for BY/RU and restriction branch for UA were preserved in `trade_remedy_cases`.
- Estimated VAT at 20% and included `1032` pence on the provided customs value basis of `5160` pence.

## Warnings

- `ORIGIN_INFERRED_FROM_SHIP_FROM`: country of origin was absent, so ship-from country `CN` was used.
- `CUSTOMS_VALUE_MAY_EXCLUDE_SHIPPING`: quote says shipping is not included and no Incoterm was supplied.
- `LOW_E_BRANCH_SELECTED_FROM_TITLE`: selected branch depends on Low-E evidence in the quote title; if that coating is absent, `7008008100` may be closer.

## Evidence

- Result contract: `result.json`
- HMRC selected commodity endpoint: `https://www.trade-tariff.service.gov.uk/uk/api/commodities/7008002000`
- HMRC selected commodity description observed: "Coloured throughout the mass (body tinted), opacified, flashed or having an absorbent or reflecting layer"
- HMRC selected commodity declarable state observed: `declarable: true`
- HMRC import measures observed for selected code: third-country duty `0.00%`, supplementary unit `square metre (m2)`, VAT `20.00%`, additional duties `35.00%` for BY/RU, and restriction on entry into free circulation for UA.
