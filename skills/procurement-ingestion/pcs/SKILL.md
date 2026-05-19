---
skill_id: procurement-ingestion/pcs
skill_version: 1.0.0
promotion_stage: candidate
interaction_profile: http_public
auth_requirements: none
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
model_profile: deterministic_http_low_cost
side_effect_class: none
requires_cdp: false
endpoints:
  notice_list: "https://api.publiccontractsscotland.gov.uk/v1/Notices"
  notice_family: "https://api.publiccontractsscotland.gov.uk/v1/Notice?id={ocid}"
source_account_state: public_no_account_required
validator_overlay: validation/overlay-customs-classification
worker_behavior_version: procurement-ingestion/pcs@1.0.0
validator_behavior_version: validation/overlay-customs-classification@1.0.0+1.0
---

# procurement-ingestion/pcs

## Use when
Use this skill when the daemon needs Public Contracts Scotland (PCS) notice-list or notice-family retrieval from the official PCS OCDS Web API for Scotland procurement coverage.

## Do not use when
Do not use this skill for Contracts Finder, Find a Tender, ProContract, supplier-price enrichment, landed-cost calculation, bid submission, or authenticated PCS-Tender document retrieval.

## Inputs
- `mode`: `notice_list`, `notice_detail`, or `notice_list_replay`.
- `dateFrom`: optional PCS month filter in `mm-yyyy` format for `notice_list`. Defaults to the current month.
- `noticeType`: optional PCS notice type. Defaults to `2` for contract notices.
- `outputType`: optional PCS output type. Defaults to `0` for OCDS.
- `ocid`: required for `notice_detail`.
- `limit`: optional maximum notices to return from a list response.
- `cassette_path`: optional checked-in replay artifact for deterministic stale-cassette coverage.

## Outputs
- A result contract matching `result.schema.json`.
- A checkpoint matching `checkpoint.schema.json` after each committed batch.
- `continuation_required: true` only when the run stops before a safe cursor boundary.

## Non-goals
- PCS-Tender login or document download.
- Scotland scope decisions; PCS is in scope per the 2026-05-08 operator decision.
- Product classification, tariff lookup, freight, FX, or supplier quote scoring.

## Failure Codes
- SOURCE_CHANGED
- SOURCE_SOFT_BLOCKED
- PAGE_SHAPE_DRIFT
- AGENT_PROTOCOL_MISMATCH
- PCS_OCDS_EMPTY_RESULT
- PCS_OCDS_SCHEMA_DRIFT
- PCS_OCDS_ENDPOINT_UNAVAILABLE

## Resume Strategy
Persist a checkpoint only after a full list page or notice-family response has been schema-validated and transformed. Resume `notice_list` from the last committed `dateFrom`, `noticeType`, `outputType`, and release offset; resume `notice_detail` by retrying the same `ocid` because the endpoint returns a single notice family. If a replay cassette is stale or malformed, return a blocked result with the stale-cassette warning and do not advance the checkpoint.

## Run Manifest Instructions
Every run manifest must include `manifest_version`, `skill_id`, `skill_version`, `job_id`, `session_id`, and `execution_mode` in frontmatter or equivalent structured fields. The narrative must include `Summary`, `Steps Taken`, `Decisions Made`, `Warnings`, and `Evidence`. Evidence should name the PCS endpoint, query parameters, response family (`releases` or `records`), canary id when applicable, and any checkpoint boundary that was committed.

## Public Endpoint Contract
The PCS OCDS Web API is public, unauthenticated, and free to use:

- Notice list: `https://api.publiccontractsscotland.gov.uk/v1/Notices`
- Notice family: `https://api.publiccontractsscotland.gov.uk/v1/Notice?id={ocid}`
- Help page: `https://api.publiccontractsscotland.gov.uk/`

The live probes for this skill were verified against:

- List: `https://api.publiccontractsscotland.gov.uk/v1/Notices?dateFrom=05-2026&noticeType=2&outputType=0`
- Family: `https://api.publiccontractsscotland.gov.uk/v1/Notice?id=ocds-r6ebe6-0000832398`

## Playbook

### Step 1: Build The Request

For `notice_list`, call:

```text
GET https://api.publiccontractsscotland.gov.uk/v1/Notices?dateFrom={dateFrom}&noticeType={noticeType}&outputType={outputType}
```

Use `dateFrom` in `mm-yyyy` format. A full calendar month of releases is returned. Keep `outputType=0` so the response remains OCDS.

For `notice_detail`, call:

```text
GET https://api.publiccontractsscotland.gov.uk/v1/Notice?id={ocid}
```

The `ocid` must retain the `ocds-r6ebe6-` prefix.

### Step 2: Validate Response Shape

List responses must contain:

- `uri`
- `version`
- `publishedDate`
- `releases`
- `publisher.name`

Notice-family responses must contain:

- `uri`
- `version`
- `publishedDate`
- `records`
- `publisher.name`

If a successful HTTP response omits the expected array, return a blocked result with `PCS_OCDS_SCHEMA_DRIFT`. If the array is present but empty for a deliberate zero-result canary, return `notices: []`, `continuation_required: false`, and add warning `PCS_OCDS_EMPTY_RESULT`.

### Step 3: Transform Releases

For every release, map:

```text
notice_id: "pcs-" + release.id
source_notice_id: release.id
source_url: first tender/planning document URL, else first canonical link href
ocid: release.ocid
publication_date: release.date[0..10]
title: release.tender.title or release.planning.documents[0].title
buyer_name: release.buyer.name or first party with role "buyer"
buyer_country: "GB-SCT"
cpv_codes: tender.classification.id plus item additionalClassifications[].id
lot_count: release.tender.lots.length, or 1 when no explicit lots exist
tender_documents: release.tender.documents plus release.planning.documents
raw_release_json: JSON.stringify(release)
```

Preserve all source text and identifiers exactly. Do not infer English/Scottish scope from buyer names; the skill output marks `buyer_country` as `GB-SCT` because PCS is the Scotland source.

### Step 4: Canary Evidence

The committed canaries cover:

- Current notice-list retrieval.
- Fasteners/ironmongery family retrieval.
- Packaging materials family retrieval.
- Examination gloves family retrieval.
- Deliberate zero-result recovery.
- Stale cassette replay.

All canary cases must pass schema validation before this source can be promoted beyond `candidate`.
