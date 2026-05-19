# HMRC FPO Endpoint Decision

Bead: `testproject1-hn05.4.1` / `l5-p4-hmrc-fpo-taric-discovery`  
Recorded: `2026-05-19T21:56:17Z`  
Agent: `NavyStone`

## Decision

The HMRC Fast Parcel Operator API is not a public/free L5 source. It is documented as free to use only for UK Carrier scheme registered organisations and restricted to invited Fast Parcel Operators that receive an `X-Api-Key`.

L5 therefore records the FPO endpoint as `FPO_ACCESS_UNAVAILABLE` unless an operator-provided prerequisite later supplies authorised public use evidence. No Developer Portal account, API-key procurement, or credential request is created by this bead.

## Endpoint Evidence

- FPO base URL: `https://search.trade-tariff.service.gov.uk`
- FPO operation: `POST /fpo-code-search`
- Authentication: `X-Api-Key` header required
- Unauthenticated probe: `403 {"message":"Forbidden"}`
- Public replacement selected for this bead: `GET https://www.trade-tariff.service.gov.uk/api/v2/search_references?query%5Bletter%5D=s`
- Legacy service paths probed: `/api/search` and `/api/search_suggestions` returned `404 Not Found`
- Successful public replacement cassette: `tests/fixtures/cassettes/l5/hmrc/fpo-search-reference-medical-gloves.json`
- Restricted FPO cassette: `tests/fixtures/cassettes/l5/hmrc/fpo-restricted-403.json`

The public search-reference endpoint is used only as recall/search relevance. It is not legal classification evidence and cannot auto-classify by itself.

## Fallback Behavior

If the public replacement is rate-limited, unavailable, or schema-drifted, the adapter emits `FPO_ACCESS_UNAVAILABLE`, marks the classification path stale when appropriate, and continues with the remaining recall families. Four non-FPO recall families plus HMRC public tariff search remain sufficient for green recall-source readiness when FPO is unavailable.

## TARIC Boundary

The prerequisite `skills/landed-cost/taric-cross-check/` is present and its test evidence reports READY/PASS. TARIC remains advisory only: disagreements are retained as evidence divergence, while HMRC remains authoritative for UK national measures, VAT additional codes, UK quotas, and UK trade remedies.

## Guardrails

- FPO failures reuse the daemon vocabulary code `FPO_ACCESS_UNAVAILABLE`.
- No second FPO-specific unavailable code is introduced.
- FPO/search suggestions are retrieval-only and never legal evidence.
- HS Ping remains declined; it is not introduced as a source or credential path.
