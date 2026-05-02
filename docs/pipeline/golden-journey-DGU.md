# Golden Journey - DGU (Double-Glazed Unit, BS EN 1279)

> Generated: 2026-05-02T13:08:18Z  
> Process: `proc-pc-7046e8ec-2734-ed11-8119-005056b64545`  
> Target notice: `pc-7046e8ec-2734-ed11-8119-005056b64545`  
> Target product candidate: `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279`  
> Scope: trace one real ProContract procurement opportunity through the 14-stage intelligence pipeline without re-running pipeline stages.

## Executive Summary

This document answers the stakeholder question, "what does this product actually do?", by following one real procurement process through the local procurement-intelligence pipeline from source ingestion to operator decision. The target is the DGU process: a ProContract Dynamic Purchasing System (DPS) opportunity for double-glazed units and sheet glass. The current live SQLite data identifies the buyer as `Bristol City Council`, not a university buyer. The commercially interesting thread inside the process is the Lot 1 double-glazed unit candidate: `Double-glazed unit (sealed insulating glass unit, BS EN 1279-compliant, argon-filled, warm-edge spacer)`.

The product ingests a source notice, normalizes it into a stable procurement process, checks whether it relates to other known processes, acquires tender documents, extracts lots and product candidates, scores which products are quoteable, searches supplier markets, chooses a usable price signal, estimates margin, generates an operator-facing manifest, and records operator decisions with reviewed-basis hashes. The DGU process demonstrates that chain on real data: the system found the ProContract notice, acquired document files, extracted two goods lots, narrowed to one priced DGU candidate, found UK marketplace evidence and a China supplier quote, produced a gross-margin and landed-cost assessment, rendered a manifest with a `clarify` stoplight, and captured prior operator intents.

The current decision is not "go bid now." The current manifest says `clarify`: the DGU candidate has pricing evidence and a landed-cost record, but revenue is still `rfq_required`, so the operator needs an RFQ quote before the platform can make a confident commercial call. There is an active operator override at `250 square meter`, which Stage 11 recognizes as the order-quantity basis. There are also data-quality rough edges important for a stakeholder demo: the live DB contains more lots/candidates/quotes than the original handoff assumed; the latest landed-cost row was refreshed after the latest net-margin and manifest rows; and the current opportunity manifest appears stale relative to that latest landed-cost refresh.

The key product proof is still strong: the system can show the whole decision chain with concrete row IDs, artifact paths, stage heads, session logs, and operator decision history. The demo should be framed as a real operational trace with known rough edges, not as a curated "perfect" record.

## Setup

| Item | Value |
|---|---|
| Working tree branch | `main` |
| HEAD at discovery time | `e71f84e1c6114c163abd3446735fdcb9e88e49af` |
| SQLite file inspected | `state/state.db` |
| SQLite open mode | read-only URI: `file:state/state.db?mode=ro` |
| Start DB hash | `a78c02473d83a5b88becf7348ca3f946ed1688f85b95441bd2dcadca15e3a78d` |
| Start DB mtime/size | `May 2 13:06:49 2026`, `160673792` bytes |
| Post-draft DB hash observed during validation | `a60810425b78c377e2e60ad98dd1fc8563e3b801f52371005f01aa578c9cc0c1` |
| Post-draft DB mtime/size observed during validation | `May 2 15:12:28 2026`, `160681984` bytes |
| Pipeline re-runs performed | None |
| Optional desktop screenshot | Skipped; this handoff keeps the DB and app state read-only |

Hash note: this handoff did not re-run any stage or issue any state-mutating DGU command. The file-level DB hash changed while the handoff was being prepared in the shared workspace. The logical DGU pre-flight counts re-checked after the hash change were still `1 notice`, `2 lots`, `3 qualification runs`, `6 candidates`, `5 quotes`, `1 pricing assessment`, `1 landed-cost estimate for the selected quote`, `1 net-margin assessment`, `1 manifest`, `2 operator intents`, and `1 override`. Treat the hash change as shared-environment concurrency evidence rather than a DGU pipeline transition made by this handoff.

Terminology used below:

| Term | Meaning |
|---|---|
| DPS | Dynamic Purchasing System. A public-sector buying route where suppliers are admitted to a panel and later call-offs decide exact orders. |
| DGU | Double-glazed unit. Here, the target candidate is a sealed insulating glass unit with BS EN 1279 evidence. |
| RFQ | Request for quote. The platform needs supplier-specific price, lead-time, MOQ, and quantity basis before a final commercial decision. |
| bps | Basis points. `10000 bps = 100%`; `8500 bps = 85%`. |
| Gross margin gate | Stage 10 check that compares selected supplier price against expected addressable revenue before import/shipping costs. |
| Landed cost | Stage 11 estimate of supplier cost plus freight, customs duty, import VAT, final-mile, and other fees. |
| Net margin gate | Stage 12 check that combines revenue, supplier quote, and landed cost into pass/review/reject guidance. |
| Manifest | Operator-facing Markdown review packet with summary, candidate detail, pricing evidence, risk flags, and recommendation. |

### Pre-flight Snapshot

The original mission text described DGU as having one lot, one product candidate, and one supplier quote. The live DB is richer. It has one notice and one core priced DGU candidate, but the process-level rows include two lots, six product candidates, and five supplier quote rows.

```sql
SELECT 'notice' AS entity, COUNT(*) AS count
FROM contract_notices
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
UNION ALL SELECT 'lot', COUNT(*)
FROM notice_lots
WHERE notice_id IN (
  SELECT id FROM contract_notices
  WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
)
UNION ALL SELECT 'qualification_run', COUNT(*)
FROM qualification_runs
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
UNION ALL SELECT 'candidate', COUNT(*)
FROM product_candidates
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
UNION ALL SELECT 'quote', COUNT(*)
FROM supplier_quotes
WHERE product_candidate_id LIKE 'pc-7046e8ec-%'
UNION ALL SELECT 'pricing_assessment', COUNT(*)
FROM candidate_pricing_assessments
WHERE product_candidate_id LIKE 'pc-7046e8ec-%'
UNION ALL SELECT 'lce', COUNT(*)
FROM landed_cost_estimates
WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1'
UNION ALL SELECT 'net_margin', COUNT(*)
FROM candidate_net_margin_assessments
WHERE product_candidate_id LIKE 'pc-7046e8ec-%'
UNION ALL SELECT 'manifest', COUNT(*)
FROM manifests
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
UNION ALL SELECT 'intents', COUNT(*)
FROM pending_operator_intents
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
UNION ALL SELECT 'overrides', COUNT(*)
FROM operator_input_overrides
WHERE product_candidate_id LIKE 'pc-7046e8ec-%';
```

| Entity | Count |
|---|---:|
| notice | 1 |
| lot | 2 |
| qualification_run | 3 |
| candidate | 6 |
| quote | 5 |
| pricing_assessment | 1 |
| lce | 1 |
| net_margin | 1 |
| manifest | 1 |
| intents | 2 |
| overrides | 1 |

The stage tracker confirms that DGU has rows across the pipeline. Its chronology contains repeats because prior validation runs re-rendered Stage 11, Stage 12, Stage 13, and Stage 14 several times.

| Stage surface | Evidence |
|---|---|
| `process_stage_status` | Rows exist for normalize, tender enrichment, acquire docs, relate, qualify lot, price-signal match, score quoteability, prequalify notice, gross margin gate, net margin gate, manifest, operator decision, and landed cost. |
| `process_stage_heads` | Current heads exist for the same durable stages; latest Stage 11 head is newer than the latest Stage 13 head. |
| `agent_session_logs` | Rows exist for ProContract discovery/enrichment, document semantic extraction, marketplace search, Stage 11 landed-cost skills, and Stage 13 manifest generation. |
| Disk artifacts | The operator manifest exists at `artifacts/manifests/proc-pc-7046e8ec-2734-ed11-8119-005056b64545/manifest.md`; session forensic artifacts exist under `artifacts/sessions/YYYY/MM/<session-id>/...`. |

## Stage-by-stage

### Stage 1: Ingest

#### What this stage does

Ingest is the source-acquisition step. For ProContract, the agent skill visits the source, extracts the notice metadata, and writes a raw source record plus a normalized internal notice row. This is the point where an external portal advert becomes local evidence that downstream stages can replay without revisiting the portal.

#### What ran on DGU at this stage

The initial durable ProContract discovery run for this process is visible as a source-level session rather than a process-scoped job. The key session is `session-pty-8d2add4058ce869c177b8abcaba4138778f9494299d3a9e9444f6b1c80fe80a1`, attached to `skills/procurement-ingestion/procontract-discover/SKILL.md` version `1.0.0`, status `completed`, started `2026-04-13T17:10:22.369207+00:00`, completed `2026-04-13T17:25:52.823568+00:00`. Its forensic stream index is under `artifacts/sessions/2026/04/session-pty-8d2add4058ce869c177b8abcaba4138778f9494299d3a9e9444f6b1c80fe80a1/stream.index.jsonl`.

The resulting raw document row is:

| Field | Value |
|---|---|
| `raw_documents.id` | `raw-pc-7046e8ec-2734-ed11-8119-005056b64545` |
| `source_id` | `procontract` |
| `fetched_at` | `2026-04-13 17:25:52` |
| `content_type` | `application/json` |
| `source_url` | `https://procontract.due-north.com/Advert?advertId=7046e8ec-2734-ed11-8119-005056b64545` |
| `payload_len` | 447 |

#### What ended up in the DB

| Field | Value |
|---|---|
| `contract_notices.id` | `pc-7046e8ec-2734-ed11-8119-005056b64545` |
| `source_id` | `procontract` |
| `source_notice_id` | `7046e8ec-2734-ed11-8119-005056b64545` |
| `title` | `BSH/HRA/Supply of Double Glazed Units and Sheet Glass - DPS - Acceptance Round 5` |
| `buyer_name` | `Bristol City Council` |
| `procurement_stage` | `tender` |
| `access_route` | `portal_authenticated` |
| `deadline_at` | `2026-11-30T20:00:00Z` |
| `published_at` | `2022-09-14T13:30:00Z` |
| `source_updated_at` | `2026-04-15T12:52:00Z` |
| `main_category` | `14820000-5 - Glass | 44221111-6 - Double-glazed units` |

Verification query:

```sql
SELECT id, source_id, source_notice_id, title, buyer_name, procurement_stage,
       access_route, deadline_at, published_at, source_updated_at, main_category
FROM contract_notices
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
```

#### What the operator or downstream stage sees

Downstream stages see a ProContract tender with an authenticated-portal route, a long-running DPS deadline, and glass-related CPV/category signals. The first operator-facing implication is that this is not a simple published-document notice; the system may need portal-authenticated enrichment to acquire the tender documents.

#### Notable observations

The source notice is real and stable, but the handoff's "UK university tender" phrasing does not match the live DB. The live buyer is `Bristol City Council`. For stakeholder demos, use the DB value.

> Strategy doc reference: `docs/strategy/layers/layer-1-thin-local-runner.md` overview and `docs/strategy/layers/layer-2-single-operator-desktop.md` section "New Skill: Contracts Finder Ingestion" for source-skill packaging expectations.

### Stage 2: Normalize

#### What this stage does

Normalize turns a source-specific notice into a stable procurement-process record. A procurement process is the internal identity the rest of the platform uses; it ties notices, lots, candidates, pricing, margins, manifests, and decisions together.

#### What ran on DGU at this stage

Normalization is deterministic. It ran as part of the ingestion persistence path and wrote both the notice row and the process row at `2026-04-13 17:25:52`. The durable stage row is `Normalize | notice | pc-7046e8ec-2734-ed11-8119-005056b64545 | succeeded`.

#### What ended up in the DB

| Field | Value |
|---|---|
| `procurement_processes.id` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` |
| `canonical_ocid` | `pc-7046e8ec-2734-ed11-8119-005056b64545` |
| `primary_notice_id` | `pc-7046e8ec-2734-ed11-8119-005056b64545` |
| `current_stage` | `normalize` |
| `buyer_name` | `Bristol City Council` |
| `title` | `BSH/HRA/Supply of Double Glazed Units and Sheet Glass - DPS - Acceptance Round 5` |
| `latest_deadline_at` | `2026-11-30T20:00:00Z` |
| `latest_published_at` | `2022-09-14T13:30:00Z` |
| `latest_updated_at` | `2026-04-15T12:52:00Z` |

Verification query:

```sql
SELECT id, canonical_ocid, primary_notice_id, canonical_process_id, current_stage,
       buyer_name, title, latest_deadline_at, latest_published_at, latest_updated_at
FROM procurement_processes
WHERE id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
```

#### What the operator or downstream stage sees

The operator does not usually see "Normalize" as a standalone product feature. They see its result: a single detail page can use one process ID to gather notice metadata, lots, product candidates, quotes, costs, manifest, and decisions.

#### Notable observations

`canonical_process_id` is blank in the current row. Layer 3 describes canonical process roots for relation-driven merges; this process has not been merged into another process.

> Strategy doc reference: `docs/strategy/layers/layer-1-thin-local-runner.md` "Capability Delivered" and `crates/daemon/src/jobs/runner.rs` `stage_scope_rule` for the notice-scoped `Normalize` stage.

### Stage 3: Relate

#### What this stage does

Relate checks whether the new process is the same as, connected to, or a possible duplicate of other processes. It exists to avoid presenting fragmented procurement identity to the operator. If the process is separate, the correct persisted evidence can be no relation rows.

#### What ran on DGU at this stage

The current stage tracker has `relate | process | proc-pc-7046e8ec-2734-ed11-8119-005056b64545 | succeeded` at `2026-04-18 15:12:13`, stage run `stage-run-f1185c85-45b8-44da-a202-4c0549d3e347-proc-pc-7046e8ec-2734-ed11-8119-005056b64545`.

#### What ended up in the DB

There are no `process_relations` rows for DGU.

Verification query:

```sql
SELECT *
FROM process_relations
WHERE process_id_a='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
   OR process_id_b='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
```

Result: empty set.

#### What the operator or downstream stage sees

The downstream stages treat DGU as a standalone process. There is no cluster card, alias redirect, duplicate warning, or cache-sharing relation attached to this process.

#### Notable observations

Empty output is the intended evidence for a "separate" process. This is worth calling out in a demo because otherwise a blank relation table can look like a missing stage.

> Strategy doc reference: `docs/strategy/layers/layer-3-operational-hardening.md` section "Relation Resolution Engine".

### Stage 4: Acquire Docs

#### What this stage does

Acquire Docs logs into or navigates the procurement portal and downloads tender documents. For a stakeholder, this is where the platform stops being a notice scraper and starts building an evidence packet from actual tender files.

#### What ran on DGU at this stage

ProContract enrichment ran multiple times. The most relevant document-acquisition sessions are:

| Session | Skill | Version | Started | Completed | Status | Artifact |
|---|---|---:|---|---|---|---|
| `session-pty-bf2f900238b220cc35f4243737b08d97e9029db1550214ee3ad652ac66d99e37` | `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` | `1.1.0` | `2026-04-14T22:08:56.001945+00:00` | `2026-04-14T23:05:38.450649+00:00` | completed | `artifacts/sessions/2026/04/session-pty-bf2f900238b220cc35f4243737b08d97e9029db1550214ee3ad652ac66d99e37/stream.index.jsonl` |
| `session-pty-7e0c8c10fd5b38655a7fd7856be0a5d1784347e7a6c33c94bc41b158dfbee196` | `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` | `1.1.0` | `2026-04-15T11:52:39.007634+00:00` | `2026-04-15T13:03:46.092294+00:00` | completed | `artifacts/sessions/2026/04/session-pty-7e0c8c10fd5b38655a7fd7856be0a5d1784347e7a6c33c94bc41b158dfbee196/stream.index.jsonl` |

#### What ended up in the DB

The current document set row is:

| Field | Value |
|---|---|
| `tender_document_sets.id` | `tds-pc-7046e8ec-2734-ed11-8119-005056b64545` |
| `document_set_hash` | `62c668ad65e13678b899fdab4237453a482f1939e8889d4814e8c75d1c8dc365` |
| `completeness_state` | `complete` |
| `extraction_state` | `partial` |
| `acquired_at` | `2026-04-15T12:52:00Z` |

Downloaded raw document evidence includes:

| Raw document ID suffix | Filename | Role hint | Size |
|---|---|---|---:|
| `...1e509f...` | `Appendix 1 - Evaluation Criteria Sheet Glass.docx` | `response_document` | 71,294 bytes |
| `...6aa08e...` | `Glazing Supplies - DPS Agreement.docx` | `terms_and_conditions` | 128,854 bytes |
| `...4ac5d3...` | `Schedule 7 - Call-Off Contract - Glazing.docx` | `terms_and_conditions` | 43,956 bytes |
| `...fa46de...` | `ITT Glazed Units & Sheet Glass DPS.docx` | `invitation_to_tender` | 135,436 bytes |

Verification query:

```sql
SELECT id, process_id, notice_id, document_set_hash, completeness_state,
       extraction_state, document_roles_json, acquired_at, stage_run_id
FROM tender_document_sets
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
```

#### What the operator or downstream stage sees

Downstream extraction stages can now read the tender pack locally. The operator later sees the effect as lot titles, candidate descriptions, and a manifest evidence trail rather than a folder full of raw `.docx` files.

#### Notable observations

The document set is `complete` for acquisition but only `partial` for extraction. That distinction matters: the platform successfully downloaded the tender pack, while semantic extraction chose or completed only some overlays.

> Strategy doc reference: `docs/strategy/layers/layer-1-thin-local-runner.md` PDF/document extraction sections and `docs/strategy/layers/layer-2-single-operator-desktop.md` "Contract Detail Surfaces the Full Review Packet".

### Stage 5: Extract Lots

#### What this stage does

Extract Lots turns tender-document structure into bid scopes. A lot is a separable piece of the procurement that can be qualified, priced, and surfaced independently.

#### What ran on DGU at this stage

The document extraction pipeline used Docling `2.88.0` with `standard` processing. The extracted documents that matter most to lot creation are:

| Extraction ID | Document title | Current | Sections | Tables | Semantic state |
|---|---|---:|---:|---:|---|
| `ef9b577c-59b9-4696-b5f3-1d251ce6bca1` | `Appendix-1-Evaluation-Criteria-Sheet-Glass` | 1 | 5 | 4 | complete |
| `b2991f39-4d50-4563-b137-b6f8462f602e` | `Glazing-Supplies-DPS-Agreement` | 1 | 65 | 25 | pending |
| `bdfd2938-fba8-48b8-9ab0-bc4deb130a29` | `ITT-Glazed-Units-Sheet-Glass-DPS` | 1 | 41 | 2 | pending |
| `2822160c-13e5-4682-ae78-e0cad8c120bc` | `Schedule-7-Call-Off-Contract-Glazing` | 1 | 144 | 2 | skipped |

#### What ended up in the DB

The current `notice_lots` table contains two lots:

| Lot ID | Lot number | Title | Bidable separately |
|---|---:|---|---:|
| `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1` | 1 | `Supply of Double Glazed Units` | 1 |
| `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-2` | 2 | `Supply of Sheet Glass` | 1 |

Verification query:

```sql
SELECT id, notice_id, source_lot_id, lot_key, lot_number, title,
       estimated_value_pence, deadline_at, is_bidable_separately, stage_run_id
FROM notice_lots
WHERE notice_id='pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY id;
```

#### What the operator or downstream stage sees

The operator sees that DGU is not just one undifferentiated opportunity. It has a double-glazed-units lot and a sheet-glass lot. The golden journey follows Lot 1 because it produced the BS EN 1279 candidate and the selected supplier quote.

#### Notable observations

The handoff expected one lot, but the live extraction contains two. For the demo story, this is not a blocker; it makes the process more realistic. The manifest focuses on the one priced candidate under Lot 1.

> Strategy doc reference: `docs/strategy/layers/layer-1-thin-local-runner.md` document extraction cascade and `docs/strategy/layers/layer-2-single-operator-desktop.md` manifest section "Lot Analysis".

### Stage 6: Qualify Lot

#### What this stage does

Qualify Lot decides whether a lot is relevant to physical-goods supply. It filters out service-only or expired opportunities and keeps goods opportunities that can plausibly be quoted and supplied.

#### What ran on DGU at this stage

The stage tracker has lot-scoped success rows for both extracted lots at `2026-04-18 15:15:29`, plus a later notice-level prequalification row at `2026-04-21 18:30:40`. The live schema uses `qualification_runs`, not the `lot_qualifications` table name used in the handoff prompt.

#### What ended up in the DB

| Qualification ID | Lot | Expiry qualified | Goods qualified | Goods confidence | Basis | Rationale |
|---|---|---:|---:|---:|---|---|
| `qual-lot-...-lot-1` | Lot 1 | 1 | 1 | 8500 | `lot_level` | Inherited from notice: category goods, confidence 8500 bps |
| `qual-lot-...-lot-2` | Lot 2 | 1 | 1 | 8500 | `lot_level` | Inherited from notice: category goods, confidence 8500 bps |
| `qual-pc-7046e8ec-...` | Lot 1 | 1 | 1 | 8500 | `notice_only` | CPV division 14 goods classification |

Verification query:

```sql
SELECT id, lot_id, expiry_qualified, goods_qualified, goods_confidence_bps,
       legal_category, goods_density_score_bps, goods_separability,
       qualification_basis, addressable_value_pence, access_route,
       rationale, qualified_at, stage_run_id
FROM qualification_runs
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY qualified_at;
```

#### What the operator or downstream stage sees

Both lots remain in the goods lane. The platform can keep searching and pricing rather than stopping the process as irrelevant.

#### Notable observations

The stale handoff query name is a documentation drift finding: the current DB table is `qualification_runs`. Also, the extra notice-level qualification row should be explained in demos as historical prequalification evidence, not a third lot.

> Strategy doc reference: `docs/strategy/layers/layer-2-single-operator-desktop.md` "Coverage Metrics" and Stage 6 goods qualification expectations.

### Stage 7: Reclassify Candidates / Extract Products

#### What this stage does

Product extraction translates lot text and tender-document semantics into concrete things the system can search for. Reclassification then cleans up whether each item is a product, service, broad category, or SKU-like candidate.

#### What ran on DGU at this stage

Three document-semantic extraction sessions wrote lot and candidate references:

| Session | Extraction ID | Confidence | Written refs |
|---|---|---:|---|
| `session-semantic-86c6a4cb6cca140186156f8a8d7bfad2ce1ec4750f85b57bfcfa010ae4ed8b39` | `semantic-00914d37-f6e6-4684-a56d-548bcf53d37d` | 8200 | Lot 1, BS EN 1279 DGU candidate, Stippolytte candidate |
| `session-semantic-ebec889cb8a89687c2b39f66caf564e72411272eee89ac7fa8a6389009446242` | `semantic-583f6750-9bfa-471f-9d8c-2ef8d9edcc2a` | 7200 | Lot 1 broad DGU candidate, Lot 2 broad sheet-glass candidate |
| `session-semantic-ef096cf9468384f89c90818786bed9f992845ae8c43024af46a29434abdb82ac` | `semantic-d76fe327-e6fa-4d00-bde1-82e098cd9dc1` | 7800 | Lot 1 DPS DGU candidate, Lot 2 DPS sheet-glass candidate |

#### What ended up in the DB

The process has six product candidates. The target one is:

| Field | Value |
|---|---|
| `product_candidates.id` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279` |
| `lot_id` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1` |
| `name` | `Double-glazed unit (sealed insulating glass unit, BS EN 1279-compliant, argon-filled, warm-edge spacer)` |
| `candidate_type` | `sku` |
| `match_archetype` | `direct_match` |
| `head_noun` | `double glazed unit` |
| `standards_json` | `["BS EN 1279"]` |

Verification query:

```sql
SELECT id, lot_id, candidate_key, candidate_role, name, candidate_type,
       match_archetype, head_noun, standards_json, created_at, updated_at
FROM product_candidates
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY lot_id, id;
```

#### What the operator or downstream stage sees

The operator eventually sees a concrete DGU line item rather than just "glass." Downstream search gets a standards-aware query target, which is why Stage 9 can search for a double-glazed unit instead of the whole DPS.

#### Notable observations

The candidate table contains duplicate-ish broad seeds and DPS category candidates. The manifest narrows to one priced candidate, but a stakeholder may ask why six candidates exist. The answer is that extraction preserves candidate evidence from multiple tender documents; later stages only carry forward the quoteable/priced candidate into the decision packet.

> Strategy doc reference: `docs/strategy/layers/layer-1-thin-local-runner.md` document semantic extraction and `docs/strategy/layers/layer-2-single-operator-desktop.md` manifest section "Product Candidates".

### Stage 8: Score Quoteability

#### What this stage does

Quoteability scoring estimates whether a product candidate is specific enough to search and price. High quoteability means the system has enough identity and specification signal to ask supplier-market questions without drowning in irrelevant results.

#### What ran on DGU at this stage

Stage 8 is deterministic. The current stage head for the target candidate is `stage-run-62950487-0d76-43df-a31e-53e41677b798-pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279`, updated `2026-04-21 14:32:53`.

#### What ended up in the DB

For the target DGU candidate:

| Field | Value |
|---|---:|
| `quoteability_score_bps` | 8000 |
| `recall_risk_score_bps` | 2000 |
| `identity_confidence_bps` | 8500 |
| `spec_confidence_bps` | 9200 |
| `quantity_confidence_bps` | 9200 |

The search profile produced four query waves:

| Wave | Query | Strategy |
|---:|---|---|
| 1 | `double glazed unit Flat hermetically sealed` | `exact_noun` |
| 2 | `double glazed unit BS EN 1279` | `noun_with_spec` |
| 3 | `double glazed` | `broader_noun` |
| 4 | `double glazed unit` | `existence_check` |

Verification query:

```sql
SELECT pc.id, pc.name, pc.candidate_type, pc.quoteability_score_bps,
       pc.recall_risk_score_bps, pc.identity_confidence_bps,
       pc.spec_confidence_bps, pc.quantity_confidence_bps,
       sp.query_waves_json, sp.search_eligibility_state
FROM product_candidates pc
LEFT JOIN search_profiles sp ON sp.product_candidate_id=pc.id
WHERE pc.id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279';
```

#### What the operator or downstream stage sees

The operator does not need to inspect the score directly, but the score explains why the system attempted supplier search for this candidate. It was specific enough to move from tender evidence into market evidence.

#### Notable observations

The broad `double glazed` wave intentionally trades precision for recall. Stage 9 must then reject irrelevant listings and keep only usable evidence.

> Strategy doc reference: `docs/pipeline/stages-7-8-validation-manifest.md` and `docs/strategy/layers/layer-2-single-operator-desktop.md` "Product Candidates" manifest section.

### Stage 9: Search / Price-Signal Match

#### What this stage does

Search sends candidate-specific queries to supplier or marketplace sources. Price-signal matching converts listings into quote rows with source identity, unit price, region, confidence, and usability metadata.

#### What ran on DGU at this stage

The target DGU candidate was searched across several sources. Notable sessions include:

| Source | Session | Skill | Started | Completed | Result |
|---|---|---|---|---|---|
| `gem-imports` | `session-pty-64a53420788f33fe8cdb0c3940b7788dea7b7585c6c11491791a5ba01fd57961` | `skills/marketplace-search/gem-imports/SKILL.md` | `2026-04-19T19:05:37.874219+00:00` | `2026-04-19T19:16:46.002343+00:00` | no shortlisted quotes |
| `ebay-uk` | `session-pty-4ac4d2b61bcf2f762730a0b9365eae686eaf0c3a309e8372f9758eb7eafa5501` | `skills/marketplace-search/ebay-uk/SKILL.md` | `2026-04-19T19:29:15.376951+00:00` | `2026-04-19T19:39:08.717856+00:00` | 3 UK retail benchmark quotes |
| `amazon-uk` | `session-pty-c9f69e3124e00bcaea36992a997299030409bc7fe468d9e4045c4221fad9489d` | `skills/marketplace-search/amazon-uk/SKILL.md` | `2026-04-19T19:42:33.532605+00:00` | `2026-04-19T19:52:07.282067+00:00` | no shortlisted quotes |
| `mx-wholesale` | `session-pty-c98e4add331d10f94a46d710c4fbd2cbfc1c1c69ee27d52b1631a3dda70c81b7` | `skills/marketplace-search/mx-wholesale/SKILL.md` | `2026-04-19T19:53:01.721396+00:00` | `2026-04-19T20:01:45.669926+00:00` | no shortlisted quotes |
| `alibaba-search` | `session-pty-a8768f66e8a61216d4d44819daed1d337d15cd8c34b1db87e7848d92629a9d55` | `skills/marketplace-search/alibaba/SKILL.md` | `2026-04-21T13:12:46.772202+00:00` | `2026-04-21T13:33:18.799856+00:00` | 2 China supplier-floor quotes |

#### What ended up in the DB

The target candidate has five quote rows: three UK retail benchmark eBay rows and two Alibaba trade rows. The selected quote downstream is:

| Field | Value |
|---|---|
| `supplier_quotes.id` | `sq-b22fcf00b7a545d699c3438c42b9027c-1` |
| `source_name` | `alibaba-search` |
| `region` | `CN` |
| `quote_class` | `trade` |
| `product_name` | `Low E Tempered Insulated Glass Clear Tempered Insulated Glass Office Curtain Wall Double Glazed Glass` |
| `source_url` | `https://www.alibaba.com/product-detail/Low-E-Tempered-Insulated-Glass-Clear_1600798460524.html` |
| `unit_price_gbp_pence` | 516 |
| `quoted_quantity` | 10000 |
| `quoted_quantity_unit` | `square meter` |
| `minimum_order_quantity` | 10 |
| `price_signal_role` | `cn_supplier_floor` |
| `raw_match_score_bps` | 6500 |

Verification query:

```sql
SELECT id, product_candidate_id, source_name, source_url, region, quote_class,
       product_name, quoted_quantity, quoted_quantity_unit, unit_price_gbp_pence,
       currency, minimum_order_quantity, price_signal_role, raw_match_score_bps
FROM supplier_quotes
WHERE product_candidate_id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279'
ORDER BY id;
```

#### What the operator or downstream stage sees

The operator sees evidence that the product can be sourced: UK market references exist, and Alibaba produced a low supplier-floor price. Downstream margin stages use the selected Alibaba quote because it is the most relevant import-floor signal for a high-margin supply opportunity.

#### Notable observations

The platform uses a mixture of successful and empty searches as evidence. "No results" from several UK/wholesale sources is not wasted work; it documents search coverage and helps justify why the Alibaba quote became the selected commercial signal.

> Strategy doc reference: `docs/pipeline/stage-9-live-execution-manifest.md` and `docs/strategy/layers/layer-2-single-operator-desktop.md` manifest section "Pricing Evidence".

### Stage 10: Gross Margin Gate

#### What this stage does

Gross margin gate checks whether the selected supplier price is commercially interesting before landed-cost complexity is added. It compares expected revenue with the selected quote and records whether identity/usability gates passed.

#### What ran on DGU at this stage

Stage 10 is deterministic. The current target stage row is `gross margin gate | candidate | pc-7046e8ec-...-glass-dgu-bs-en-1279 | succeeded`, updated `2026-04-25 13:40:52`.

#### What ended up in the DB

| Field | Value |
|---|---|
| `candidate_pricing_assessments.id` | `pricing-pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279` |
| `selected_quote_id` | `sq-b22fcf00b7a545d699c3438c42b9027c-1` |
| `quote_cost_boundary` | `unit` |
| `revenue_basis_type` | `rfq_required` |
| `expected_addressable_revenue_ex_vat_pence` | 0 |
| `revenue_confidence_bps` | 1000 |
| `gross_margin_bps` | 0 |
| `gross_estimate_confidence_bps` | 1000 |
| `identity_gate_passed` | 1 |
| `usability_gate_passed` | 1 |

Verification query:

```sql
SELECT id, product_candidate_id, selected_quote_id, quote_cost_boundary,
       revenue_basis_type, expected_addressable_revenue_ex_vat_pence,
       revenue_confidence_bps, gross_margin_bps, gross_estimate_confidence_bps,
       identity_gate_passed, usability_gate_passed, pricing_input_confidence_bps
FROM candidate_pricing_assessments
WHERE product_candidate_id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279';
```

#### What the operator or downstream stage sees

The operator sees a candidate with usable supply-side pricing but no validated buyer-side revenue basis. That is why the journey does not become an automatic "pursue"; it needs RFQ clarification.

#### Notable observations

Gross margin is zero not because supplier cost is necessarily unattractive, but because the revenue basis is unknown. That distinction should be explicit in demos.

> Strategy doc reference: `docs/strategy/layers/layer-2-single-operator-desktop.md` coverage and margin display sections.

### Stage 11: Landed Cost

#### What this stage does

Landed Cost estimates what the supplier quote would cost once imported and delivered into the UK. It includes supplier cost, freight, duty, import VAT, final-mile, and confidence/blocker evidence. For DGU, it also has to respect the operator's quantity override.

#### What ran on DGU at this stage

The current latest Stage 11 head is newer than the latest manifest:

| Item | Value |
|---|---|
| Current Stage 11 head | `stage-run-6f1c6abc-d2de-46bb-9805-8b9fe39da4be-...` |
| Updated at | `2026-05-02 11:06:49` |
| Scope | `candidate_quote` |
| Scope ID | `pc-7046e8ec-...-glass-dgu-bs-en-1279::sq-b22fcf00b7a545d699c3438c42b9027c-1` |
| Latest associated skill job | `job-stage11-landed_cost_customs_classification-2d66d4f15ed2404795527270085a8ae9` |
| Latest associated session | `session-pty-1a8a33346f28f6eb373f0be402ad0f15c48e8f3ae55dd7e6cee17851cc0557d5` referenced inside `confidence_breakdown.skill_session_ids.hmrc_customs` |

The operator override in force at this stage is:

| Field | Value |
|---|---|
| `operator_input_overrides.id` | `override-30a31047-48e4b773-quantity_basis_unaligned` |
| `reason_code` | `quantity_basis_unaligned` |
| `payload_json` | `{"qty":250.0,"unit":"square meter"}` |
| `provided_at` | `2026-05-01 20:23:29` |
| `basis_hash_when_provided` | `c51aba0875297fc95625aa020c2d46bb78a74849c338d8f8086e64e8fdf4e93c` |

#### What ended up in the DB

The current landed-cost row is:

| Field | Value |
|---|---|
| `landed_cost_estimates.id` | `landed-pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279-sq-b22fcf00b7a545d699c3438c42b9027c-1` |
| `supplier_quote_id` | `sq-b22fcf00b7a545d699c3438c42b9027c-1` |
| `shipping_mode` | `unknown_import` |
| `supplier_cost_gbp_pence` | 129000 |
| `freight_base_gbp_pence` | 0 |
| `customs_duty_gbp_pence` | 0 |
| `import_vat_gbp_pence` | 25800 |
| `economic_landed_cost_gbp_pence` | 154800 |
| `cash_outlay_gbp_pence` | 154800 |
| `cash_to_clear_gbp_pence` | 25800 |
| `revenue_basis_type` | `rfq_required` |
| `decision_grade` | `blocked` |
| `confidence_bps` | 0 |
| `order_quantity_basis` | `from_operator_override`, `qty=250.0`, `unit=square meter` |
| `blockers` | `CUSTOMS_SKILL_NO_RESULT`, `CDP_CONNECT_FAILED`, `SEARATES_NO_RESULT` |

Verification query:

```sql
SELECT id, product_candidate_id, supplier_quote_id, shipping_mode,
       supplier_cost_gbp_pence, freight_base_gbp_pence, customs_duty_gbp_pence,
       import_vat_gbp_pence, economic_landed_cost_gbp_pence, revenue_basis_type,
       decision_grade, confidence_bps, confidence_breakdown, warnings, blockers,
       updated_at
FROM landed_cost_estimates
WHERE product_candidate_id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279';
```

#### What the operator or downstream stage sees

The operator sees that the override has been accepted as the quantity basis, but the current landed-cost row is blocked. The platform can show a cost estimate and cash/VAT components, but it cannot honestly claim a clean decision-grade import calculation in the current snapshot.

#### Notable observations

This is the most important demo-readiness issue. The manifest currently says `Commodity Code: 7008002000`, but the latest landed-cost row was updated after that manifest and currently has customs blockers. This suggests the visible manifest is stale relative to the latest Stage 11 state and should be explained before any stakeholder walkthrough.

> Strategy doc reference: `docs/pipeline/stage-11-real-data-validation-manifest.md`, `docs/pipeline/stage-11-hardening-order-quantity-validation-manifest.md`, and `docs/strategy/layers/layer-2-single-operator-desktop.md` "Landed cost" stub notes.

### Stage 12: Net Margin Gate

#### What this stage does

Net Margin Gate combines gross pricing, landed-cost estimate, and confidence thresholds into a verdict. Its output is the immediate basis for manifest generation and operator review.

#### What ran on DGU at this stage

The latest Stage 12 run for the process completed at `2026-05-02 10:38:13`, before the latest Stage 11 landed-cost head at `2026-05-02 11:06:49`.

#### What ended up in the DB

| Field | Value |
|---|---|
| `candidate_net_margin_assessments.id` | `net-margin-pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279` |
| `verdict` | `review` |
| `reasons_json` | `["awaiting_rfq_quote","confidence_commercial_estimate_below_floor","confidence_pricing_input_below_floor","confidence_revenue_below_floor"]` |
| `re_evaluation_eligible` | 1 |
| `net_margin_low_bps` | 0 |
| `net_margin_base_bps` | 0 |
| `net_margin_high_bps` | 0 |
| `revenue_used_pence` | 0 |
| `revenue_basis_type` | `rfq_required` |
| `economic_landed_cost_used_pence` | 154800 |
| `basis_hash` | `a5e236c4c350d3fd7de5a3cfdcb5e4fdda4ab96f0c2eeb79b7ff34efd4654f0f` |

The process rollup is:

| Field | Value |
|---|---|
| `pipeline_state` | `manifest_ready` |
| `review_required` | 1 |
| `validation_state` | `unverified` |
| `priced_coverage_bps` | 5000 |
| `landed_cost_coverage_bps` | 5000 |
| `net_qualified_coverage_bps` | 0 |
| `unresolved_mandatory_lots` | 1 |
| `decision_basis_hash` | `ecb31264ab603280c5205117e15ef8d53090646185e7a12a38c4631b43e8a965` |
| `review_stage` | `net_margin_gate` |
| `review_reason` | `awaiting_rfq_quote` |

Verification query:

```sql
SELECT product_candidate_id, verdict, reasons_json, re_evaluation_eligible,
       net_margin_low_bps, net_margin_base_bps, net_margin_high_bps,
       revenue_used_pence, revenue_basis_type, economic_landed_cost_used_pence,
       basis_hash, stage_run_id, updated_at
FROM candidate_net_margin_assessments
WHERE product_candidate_id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279';
```

#### What the operator or downstream stage sees

The operator sees "review" rather than "pass." The specific action is to obtain RFQ-level commercial details so the margin can be re-evaluated with revenue confidence.

#### Notable observations

Stage 12 currently predates the latest Stage 11 landed-cost write. Its verdict still makes business sense because the RFQ blocker remains, but the stage-head ordering means the latest manifest is not fully synchronized with the latest landed-cost attempt.

> Strategy doc reference: `docs/pipeline/stage-12-validation-manifest.md` and `docs/strategy/layers/layer-2-single-operator-desktop.md` "Coverage Metrics".

### Stage 13: Manifest Generation

#### What this stage does

Manifest Generation renders a stakeholder/operator review packet from the pipeline state. It brings together notice metadata, lots, candidates, quote evidence, landed cost, coverage, blockers, and the recommended next action.

#### What ran on DGU at this stage

The latest manifest-generation session for DGU is:

| Field | Value |
|---|---|
| `agent_session_logs.id` | `session-pty-1b442fe2da306e77eeb3768c6f4b712da4d78b31ca5e313dce89ac98853e7a70` |
| `job_id` | `job-stage13-manifest-0c625b180e784ecca7e017b88caf2b73` |
| `skill_path` | `skills/manifest-generation/SKILL.md` |
| `execution_mode` | `contract` |
| `cli_vendor` | `claude` |
| `started_at` | `2026-05-02T10:38:14.234677+00:00` |
| `completed_at` | `2026-05-02T10:38:49.367452+00:00` |
| `status` | `succeeded` |
| `exit_code` | 143 |
| `terminal_output_path` | `artifacts/sessions/2026/05/session-pty-1b442fe2da306e77eeb3768c6f4b712da4d78b31ca5e313dce89ac98853e7a70/terminal-output.txt` |

#### What ended up in the DB and on disk

| Field | Value |
|---|---|
| `manifests.id` | `manifest-proc-pc-7046e8ec-2734-ed11-8119-005056b64545` |
| `stoplight` | `clarify` |
| `suggested_action` | `Clarify 1 candidate(s) before deciding` |
| `manifest_basis_hash` | `b0fe9d06a54eb28b2e50aca20215631dd09a024362c5a1d8fe6b437b22560a10` |
| `artifact_hash` | `d7ac86248d95b1d52318ee6dc7aadaa50e181b038bfac8911ebcd45108e4cbf5` |
| `artifact_locator` | `artifacts/manifests/proc-pc-7046e8ec-2734-ed11-8119-005056b64545/manifest.md` |
| `candidate_count` | 1 |
| `priced_candidate_count` | 1 |
| `clarification_prompt_count` | 1 |

Key manifest excerpt:

```markdown
stoplight: clarify
suggested_action: Clarify 1 candidate(s) before deciding

**Review Reason:** awaiting_rfq_quote

**Reasoning:** awaiting_rfq_quote: The single candidate (Double-glazed unit,
BS EN 1279-compliant) has an identified Alibaba source quote (516 GBP/m2)
but the revenue basis type is rfq_required, meaning an RFQ from the actual
supplier is needed before commercial pricing can be validated.
```

Verification query:

```sql
SELECT id, stoplight, suggested_action, suggested_action_reasoning,
       manifest_basis_hash, artifact_hash, artifact_locator,
       candidate_count, priced_candidate_count, clarification_prompt_count,
       stage_run_id, updated_at
FROM manifests
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
```

#### What the operator or downstream stage sees

The operator sees a concise `clarify` packet. It names the DGU candidate, the Alibaba quote, the landed-cost estimate, the zero net-margin band, and the exact missing input: supplier RFQ details.

#### Notable observations

The DB manifest is useful but stale relative to the latest Stage 11 head. Also, `agent_session_logs.run_manifest_path` is blank for the Stage 13 session even though the opportunity artifact exists in `manifests.artifact_locator`. Layer 3's observability spec expects run-manifest metadata to be actively populated for manifest-producing sessions, so this is a traceability rough edge.

> Strategy doc reference: `docs/strategy/layers/layer-2-single-operator-desktop.md` section "Manifest Rendering" and `docs/strategy/layers/layer-3-operational-hardening.md` "Four-Layer Observability Model".

### Stage 14: Operator Decision

#### What this stage does

Operator Decision records the human's reviewed intent with hashes of the pipeline basis and manifest they saw. It preserves auditability: changing a decision creates a new row or status transition rather than erasing history.

#### What ran on DGU at this stage

Stage 14 was exercised by prior CLI/UI validation. The current status history includes operator decision stage rows at `2026-05-01 20:16:02`, `20:23:09`, `20:23:21`, `20:23:29`, and `20:34:44`.

#### What ended up in the DB

Decision intents:

| Intent ID | Disposition | Status | Idempotency key | Reviewed basis hash | Decision note |
|---|---|---|---|---|---|
| `intent-5045f346fb0246b7a23d9eda308ff073` | `approve` | `superseded` | `dec-2026-05-01-dgu-001` | `4e7916264b5a95ff443fbae18ca9f2ae87de26ec940723302c9c62e70f6ce380` | DGU sized at 100m2; pursue once blockers fixed |
| `intent-a33a4a3f23df45838c1d5d640e440db4` | `watch` | `withdrawn` | `dec-2026-05-01-dgu-002` | `4e7916264b5a95ff443fbae18ca9f2ae87de26ec940723302c9c62e70f6ce380` | Holding while getting a second freight quote |

Active operator override:

| Override ID | Reason | Payload | Status |
|---|---|---|---|
| `override-30a31047-48e4b773-quantity_basis_unaligned` | `quantity_basis_unaligned` | `{"qty":250.0,"unit":"square meter"}` | active; no `withdrawn_at`, no `superseded_by_override_id` |

Verification query:

```sql
SELECT id, desired_disposition, reviewed_basis_hash, reviewed_manifest_hash,
       reviewed_at_stage_run_id, reviewed_at, idempotency_key, status,
       superseded_by_intent_id, queued_at, decision_note, last_attempt_at, last_error
FROM pending_operator_intents
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY queued_at;

SELECT id, product_candidate_id, reason_code, payload_json, notes,
       basis_hash_when_provided, provided_at, withdrawn_at, superseded_by_override_id
FROM operator_input_overrides
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY provided_at;
```

#### What the operator or downstream stage sees

There is no current approved decision. The prior approval was superseded; the later watch intent was withdrawn. The active piece of operator input is the quantity override, which Stage 11 uses to avoid sizing the import estimate from the Alibaba listing's supplier capacity.

#### Notable observations

This is an effective audit story: the operator changed their mind and the system preserved the chain. It is not yet an "approved opportunity" story. The current decision packet still needs RFQ clarification.

> Strategy doc reference: `docs/strategy/layers/layer-2-single-operator-desktop.md` section "Decision Capture - pending_operator_intents" and `docs/pipeline/stage-14-validation-manifest.md`.

## Findings

| # | Finding | Severity | Disposition |
|---:|---|---|---|
| 1 | Live buyer is `Bristol City Council`, while the handoff text says "UK university tender." | Demo correctness | Use the DB value in stakeholder demos. |
| 2 | Pre-flight counts differ from the handoff assumption: 2 lots, 6 candidates, and 5 quote rows exist. | Demo narrative | Tell the richer story, then narrow to the one priced BS EN 1279 DGU candidate. |
| 3 | The handoff query references `lot_qualifications`, but the live schema uses `qualification_runs`. | Documentation drift | This document uses `qualification_runs`; no code change made. |
| 4 | Latest Stage 11 landed-cost head is newer than latest Stage 12 and Stage 13 heads. | Demo readiness | Explain that current manifest may not reflect the latest landed-cost retry. |
| 5 | Current manifest claims a commodity-code story that does not match the latest landed-cost row's blockers. | Demo readiness | Re-run Stage 12/13 before a live stakeholder demo, or frame this as a known stale artifact. |
| 6 | `agent_session_logs.run_manifest_path` is blank for manifest-producing Stage 13 sessions, while the opportunity artifact is stored via `manifests.artifact_locator`. | Traceability rough edge | Documented here; file follow-up if strict L3 observability compliance is required. |
| 7 | Several Stage 11 rows and jobs show environmental/browser blockers such as `CDP_CONNECT_FAILED` and `SEARATES_NO_RESULT`. | Environment | Not fixed here; current handoff is read-only. |
| 8 | Some `process_stage_status.scope_id` values for net margin use `glass-dgu-bs-en-1279` rather than the full candidate ID. | Trace readability | Not blocking; worth normalizing in future stage-tracking cleanup. |
| 9 | The file-level `state/state.db` hash changed during handoff validation in this shared workspace. | Audit caveat | No stage re-run or write command was issued for this handoff. Logical DGU counts stayed stable after re-check; record both hashes when reviewing R.10. |

## Cross-reference index

| Stage | Strategy doc | Section |
|---:|---|---|
| 1 | `docs/strategy/layers/layer-1-thin-local-runner.md` | Live pipeline runner, skill-based ingestion, artifact capture |
| 2 | `docs/strategy/layers/layer-1-thin-local-runner.md` | Pipeline persistence and stage scope rules |
| 3 | `docs/strategy/layers/layer-3-operational-hardening.md` | Relation Resolution Engine |
| 4 | `docs/strategy/layers/layer-1-thin-local-runner.md` | Document acquisition and extraction cascade |
| 5 | `docs/strategy/layers/layer-1-thin-local-runner.md` | Structural extraction and lot/product refresh |
| 6 | `docs/strategy/layers/layer-2-single-operator-desktop.md` | Coverage metrics and goods qualification presentation |
| 7 | `docs/strategy/layers/layer-1-thin-local-runner.md` | Document semantic extraction |
| 8 | `docs/pipeline/stages-7-8-validation-manifest.md` | Stage 8 quoteability validation |
| 9 | `docs/pipeline/stage-9-live-execution-manifest.md` | Marketplace search live execution |
| 10 | `docs/strategy/layers/layer-2-single-operator-desktop.md` | Margin display and operator review data |
| 11 | `docs/pipeline/stage-11-real-data-validation-manifest.md` | DGU landed-cost real-data validation |
| 12 | `docs/pipeline/stage-12-validation-manifest.md` | Net margin gate validation |
| 13 | `docs/strategy/layers/layer-2-single-operator-desktop.md` | Manifest Rendering |
| 14 | `docs/strategy/layers/layer-2-single-operator-desktop.md` | Decision Capture - `pending_operator_intents` |

## Appendix A: Operator's view today

No live desktop screenshot was captured for this handoff. The handoff deliberately avoided launching or re-running the desktop app because the required deliverable is a read-only trace document, and the current DB already shows a post-manifest Stage 11 refresh that should be resolved before using a fresh UI screenshot in a stakeholder demo.

## Appendix B: Stage Status Ledger

This appendix gives reviewers a compact chronology of the DGU stage tracker. It is intentionally redundant with the narrative sections: the body tells the story, while this ledger shows the durable stage rows that make the story auditable.

Query:

```sql
SELECT stage, scope_kind, scope_id, state, updated_at
FROM process_stage_status
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY updated_at, stage;
```

Result:

| Stage | Scope kind | Scope ID | State | Updated at |
|---|---|---|---|---|
| `Normalize` | `notice` | `pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-04-13 17:25:52` |
| `interest_registration` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `completed` | `2026-04-13 17:41:49` |
| `tender_enrichment` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `completed` | `2026-04-14 23:05:38` |
| `tender_enrichment` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `completed` | `2026-04-15 13:03:46` |
| `acquire docs` | `notice` | `pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-04-17 18:00:24` |
| `acquire docs` | `notice` | `pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-04-17 23:04:59` |
| `relate` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-04-18 15:12:13` |
| `qualify lot` | `lot` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1` | `succeeded` | `2026-04-18 15:15:29` |
| `qualify lot` | `lot` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-2` | `succeeded` | `2026-04-18 15:15:29` |
| `price_signal_match` | `candidate_source_pair` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279::alibaba-search` | `succeeded` | `2026-04-19 19:28:04` |
| `price_signal_match` | `candidate_source_pair` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279::ebay-uk` | `succeeded` | `2026-04-19 19:39:08` |
| `price_signal_match` | `candidate_source_pair` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279::amazon-uk` | `succeeded` | `2026-04-19 19:52:07` |
| `price_signal_match` | `candidate_source_pair` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279::mx-wholesale` | `succeeded` | `2026-04-19 20:01:45` |
| `price_signal_match` | `candidate_source_pair` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279::alibaba-search` | `succeeded` | `2026-04-21 13:33:18` |
| `Score quoteability` | `candidate` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279` | `succeeded` | `2026-04-21 14:32:53` |
| `Score quoteability` | `candidate` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-stippolytte-6-4mm` | `succeeded` | `2026-04-21 14:32:53` |
| `Score quoteability` | `candidate` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-lot1-double-glazed-units` | `succeeded` | `2026-04-21 14:32:53` |
| `Score quoteability` | `candidate` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-2-candidate-lot2-sheet-glass` | `succeeded` | `2026-04-21 14:32:53` |
| `Score quoteability` | `candidate` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-lot1-double-glazed-units-dps` | `succeeded` | `2026-04-21 14:32:53` |
| `Score quoteability` | `candidate` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-2-candidate-lot2-sheet-glass-dps` | `succeeded` | `2026-04-21 14:32:53` |
| `prequalify notice` | `notice` | `pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-04-21 18:30:40` |
| `gross margin gate` | `candidate` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279` | `succeeded` | `2026-04-25 13:40:52` |
| `net margin gate` | `candidate` | `glass-dgu-bs-en-1279` | `succeeded` | `2026-05-01 10:40:58` |
| `net margin gate` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-01 10:40:58` |
| `manifest` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-01 18:00:12` |
| `operator decision` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-01 20:16:02` |
| `net margin gate` | `candidate` | `glass-dgu-bs-en-1279` | `succeeded` | `2026-05-01 20:21:41` |
| `net margin gate` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-01 20:21:41` |
| `manifest` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-01 20:22:57` |
| `operator decision` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-01 20:23:09` |
| `operator decision` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-01 20:23:21` |
| `operator decision` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-01 20:23:29` |
| `net margin gate` | `candidate` | `glass-dgu-bs-en-1279` | `succeeded` | `2026-05-01 20:34:01` |
| `net margin gate` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-01 20:34:01` |
| `manifest` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-01 20:34:35` |
| `operator decision` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-01 20:34:44` |
| `net margin gate` | `candidate` | `glass-dgu-bs-en-1279` | `succeeded` | `2026-05-02 10:38:13` |
| `net margin gate` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-02 10:38:13` |
| `manifest` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `succeeded` | `2026-05-02 10:38:49` |
| `landed cost` | `candidate_quote` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279::sq-b22fcf00b7a545d699c3438c42b9027c-1` | `succeeded` | `2026-05-02 11:06:49` |

Current stage heads:

```sql
SELECT stage, scope_kind, scope_id, current_stage_run_id, freshness_state, updated_at
FROM process_stage_heads
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY updated_at, stage;
```

| Stage | Scope kind | Scope ID | Current stage run | Freshness | Updated at |
|---|---|---|---|---|---|
| `Normalize` | `notice` | `pc-7046e8ec-2734-ed11-8119-005056b64545` | `stage-run-60741368-d5c7-4b12-abbc-ea825bf98651-...` | `fresh` | `2026-04-13 17:25:52` |
| `interest_registration` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `stage-run-8c81f5b4-4007-4c09-ad48-3488cd0e44d2-...` | `fresh` | `2026-04-13 17:41:49` |
| `tender_enrichment` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `stage-run-57a5f226-b1b8-4236-9510-0821c0ebd20d-...` | `fresh` | `2026-04-15 13:03:46` |
| `acquire docs` | `notice` | `pc-7046e8ec-2734-ed11-8119-005056b64545` | `stage-run-57a5f226-b1b8-4236-9510-0821c0ebd20d-...-pdf-extraction-tender-specification` | `fresh` | `2026-04-17 23:04:59` |
| `relate` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `stage-run-f1185c85-45b8-44da-a202-4c0549d3e347-...` | `fresh` | `2026-04-18 15:12:13` |
| `qualify lot` | `lot` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1` | `stage-run-2775bee1-d5bf-4925-a647-af7fc6c42ac9-...-lot-1` | `fresh` | `2026-04-18 15:15:29` |
| `qualify lot` | `lot` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-2` | `stage-run-2775bee1-d5bf-4925-a647-af7fc6c42ac9-...-lot-2` | `fresh` | `2026-04-18 15:15:29` |
| `price_signal_match` | `candidate_source_pair` | `...glass-dgu-bs-en-1279::ebay-uk` | `stage-run-9546da05-3ec1-45a9-bebc-3b89f636f043-...` | `fresh` | `2026-04-19 19:39:08` |
| `price_signal_match` | `candidate_source_pair` | `...glass-dgu-bs-en-1279::amazon-uk` | `stage-run-35aa3484-9651-4724-a31b-52ff2776acfc-...` | `fresh` | `2026-04-19 19:52:07` |
| `price_signal_match` | `candidate_source_pair` | `...glass-dgu-bs-en-1279::mx-wholesale` | `stage-run-482ad2bf-5755-4b74-9675-aa95a26ecccd-...` | `fresh` | `2026-04-19 20:01:45` |
| `price_signal_match` | `candidate_source_pair` | `...glass-dgu-bs-en-1279::alibaba-search` | `stage-run-5677a7dc-7ddd-4d8e-93fb-b48cd51c02c9-...` | `fresh` | `2026-04-21 13:33:18` |
| `Score quoteability` | `candidate` | `...glass-dgu-bs-en-1279` | `stage-run-62950487-0d76-43df-a31e-53e41677b798-...` | `fresh` | `2026-04-21 14:32:53` |
| `Score quoteability` | `candidate` | `...glass-stippolytte-6-4mm` | `stage-run-62950487-0d76-43df-a31e-53e41677b798-...` | `fresh` | `2026-04-21 14:32:53` |
| `Score quoteability` | `candidate` | `...lot1-double-glazed-units` | `stage-run-62950487-0d76-43df-a31e-53e41677b798-...` | `fresh` | `2026-04-21 14:32:53` |
| `Score quoteability` | `candidate` | `...lot1-double-glazed-units-dps` | `stage-run-62950487-0d76-43df-a31e-53e41677b798-...` | `fresh` | `2026-04-21 14:32:53` |
| `Score quoteability` | `candidate` | `...lot2-sheet-glass` | `stage-run-62950487-0d76-43df-a31e-53e41677b798-...` | `fresh` | `2026-04-21 14:32:53` |
| `Score quoteability` | `candidate` | `...lot2-sheet-glass-dps` | `stage-run-62950487-0d76-43df-a31e-53e41677b798-...` | `fresh` | `2026-04-21 14:32:53` |
| `prequalify notice` | `notice` | `pc-7046e8ec-2734-ed11-8119-005056b64545` | `stage-run-bf21f143-4b6c-467a-aaca-de7d3206bfe1-...` | `fresh` | `2026-04-21 18:30:40` |
| `gross margin gate` | `candidate` | `...glass-dgu-bs-en-1279` | `stage-run-5804aec1-ffc3-4246-a4f7-db4498ed1205-...` | `fresh` | `2026-04-25 13:40:52` |
| `operator decision` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `stage-run-6545894a-6140-4c4d-aeda-29865c509ae9-...` | `fresh` | `2026-05-01 20:34:44` |
| `net margin gate` | `candidate` | `glass-dgu-bs-en-1279` | `stage-run-9899aefd-d666-433d-b756-cf8d1a35022e-...` | `fresh` | `2026-05-02 10:38:13` |
| `net margin gate` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `stage-run-9899aefd-d666-433d-b756-cf8d1a35022e-...` | `fresh` | `2026-05-02 10:38:13` |
| `manifest` | `process` | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `stage-run-eb41a5d6-ebbc-4035-b606-e05f49324a69-...` | `fresh` | `2026-05-02 10:38:49` |
| `landed cost` | `candidate_quote` | `...glass-dgu-bs-en-1279::sq-b22fcf00b7a545d699c3438c42b9027c-1` | `stage-run-6f1c6abc-d2de-46bb-9805-8b9fe39da4be-...` | `fresh` | `2026-05-02 11:06:49` |

The stage-head ledger is the clearest evidence that the current DGU process is not a linear one-shot run. Stage 13 is ready from `2026-05-02 10:38:49`, then Stage 11 landed cost was refreshed at `2026-05-02 11:06:49`. That is why this document calls out manifest staleness as a demo-readiness finding.

## Appendix C: Agent Session Evidence

Agent sessions are the forensic trail for source navigation, marketplace search, landed-cost skill calls, and manifest generation. The table below keeps the row IDs needed to find terminal output without flooding the narrative body.

Query:

```sql
SELECT id, job_id, skill_path, skill_version, execution_mode, cli_vendor,
       status, exit_code, started_at, completed_at, terminal_output_path,
       run_manifest_path
FROM agent_session_logs
WHERE skill_path LIKE '%procontract%'
   OR skill_path LIKE '%marketplace%'
   OR skill_path LIKE '%landed-cost%'
   OR skill_path LIKE '%manifest%'
ORDER BY started_at;
```

Curated result rows used by this journey:

| Stage | Session | Skill | Status | Exit | Started | Completed |
|---:|---|---|---|---:|---|---|
| 1 | `session-pty-8d2add4058ce869c177b8abcaba4138778f9494299d3a9e9444f6b1c80fe80a1` | `skills/procurement-ingestion/procontract-discover/SKILL.md` | `completed` | 0 | `2026-04-13T17:10:22.369207+00:00` | `2026-04-13T17:25:52.823568+00:00` |
| 4 | `session-pty-bf2f900238b220cc35f4243737b08d97e9029db1550214ee3ad652ac66d99e37` | `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` | `completed` | 0 | `2026-04-14T22:08:56.001945+00:00` | `2026-04-14T23:05:38.450649+00:00` |
| 4 | `session-pty-7e0c8c10fd5b38655a7fd7856be0a5d1784347e7a6c33c94bc41b158dfbee196` | `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` | `completed` | 0 | `2026-04-15T11:52:39.007634+00:00` | `2026-04-15T13:03:46.092294+00:00` |
| 9 | `session-pty-64a53420788f33fe8cdb0c3940b7788dea7b7585c6c11491791a5ba01fd57961` | `skills/marketplace-search/gem-imports/SKILL.md` | `completed` | 0 | `2026-04-19T19:05:37.874219+00:00` | `2026-04-19T19:16:46.002343+00:00` |
| 9 | `session-pty-2fdc8665ce089485a9f6e1edeedb1a35ba0c2bf6a77c69a4910d920377c03bc6` | `skills/marketplace-search/alibaba/SKILL.md` | `completed` | 0 | `2026-04-19T19:19:32.912950+00:00` | `2026-04-19T19:28:04.668183+00:00` |
| 9 | `session-pty-4ac4d2b61bcf2f762730a0b9365eae686eaf0c3a309e8372f9758eb7eafa5501` | `skills/marketplace-search/ebay-uk/SKILL.md` | `completed` | 0 | `2026-04-19T19:29:15.376951+00:00` | `2026-04-19T19:39:08.717856+00:00` |
| 9 | `session-pty-c9f69e3124e00bcaea36992a997299030409bc7fe468d9e4045c4221fad9489d` | `skills/marketplace-search/amazon-uk/SKILL.md` | `completed` | 0 | `2026-04-19T19:42:33.532605+00:00` | `2026-04-19T19:52:07.282067+00:00` |
| 9 | `session-pty-c98e4add331d10f94a46d710c4fbd2cbfc1c1c69ee27d52b1631a3dda70c81b7` | `skills/marketplace-search/mx-wholesale/SKILL.md` | `completed` | 0 | `2026-04-19T19:53:01.721396+00:00` | `2026-04-19T20:01:45.669926+00:00` |
| 9 | `session-pty-a8768f66e8a61216d4d44819daed1d337d15cd8c34b1db87e7848d92629a9d55` | `skills/marketplace-search/alibaba/SKILL.md` | `completed` | 0 | `2026-04-21T13:12:46.772202+00:00` | `2026-04-21T13:33:18.799856+00:00` |
| 11 | `session-pty-75db70be734eb948d69a4062453ba701894b7416edd7944d954eea30d97e7969` | `skills/landed-cost/hmrc-customs-classification/SKILL.md` | `completed` | 0 | `2026-04-30T18:46:22.016599+00:00` | `2026-04-30T18:51:22.994189+00:00` |
| 11 | `session-pty-eb73ce1eafcfdd80e4b3da1a21196a7ed60566a49479ecb5af140fc9bcd65f60` | `skills/landed-cost/shipment-sizing/SKILL.md` | `completed` | 0 | `2026-04-30T18:51:23.185894+00:00` | `2026-04-30T18:54:51.595115+00:00` |
| 11 | `session-pty-935210d533476f7e6a930020254a58dd5f41669d9724e385528896ddc5217bc5` | `skills/landed-cost/searates-freight-estimate/SKILL.md` | `completed` | 0 | `2026-04-30T18:54:51.809203+00:00` | `2026-04-30T18:59:52.856720+00:00` |
| 11 | `session-pty-ab02eeddd7cd3ced586bcf6f84b91612374975cda5e5f561fa462cd1aacfb2b0` | `skills/landed-cost/simple-fx-lookup/SKILL.md` | `completed` | 0 | `2026-04-30T19:56:14.307406+00:00` | `2026-04-30T19:58:32.951788+00:00` |
| 11 | `session-pty-07cd2312cb9fdacb916924a6708a0f0f176ebdf07fb87ef4470f1f16375854f0` | `skills/landed-cost/hmrc-customs-classification/SKILL.md` | `completed` | 0 | `2026-05-01T20:16:07.533622+00:00` | `2026-05-01T20:21:08.474615+00:00` |
| 11 | `session-pty-f7391d915758a260095e84444dd93eed5e0a67d87003909aff69fb7debbb990c` | `skills/landed-cost/hmrc-customs-classification/SKILL.md` | `completed` | 0 | `2026-05-02T10:33:11.529327+00:00` | `2026-05-02T10:38:12.525427+00:00` |
| 13 | `session-pty-60bf8d4598b9a89af09ca898b1d8a890a1950dafffcf6483523696de02b9c777` | `skills/manifest-generation/SKILL.md` | `succeeded` | 143 | `2026-05-01T17:59:16.261783+00:00` | `2026-05-01T18:00:12.457906+00:00` |
| 13 | `session-pty-949f04f3a302577f7122f30ff6ade611e4ad8774d2136560cfe7941e7ef0c80d` | `skills/manifest-generation/SKILL.md` | `succeeded` | 143 | `2026-05-01T20:21:43.330266+00:00` | `2026-05-01T20:22:57.449066+00:00` |
| 13 | `session-pty-faa1993d7efb7357ce9364ee647a74674aa78b5755923f4c3f4566b532bb7da5` | `skills/manifest-generation/SKILL.md` | `succeeded` | 143 | `2026-05-01T20:34:02.293686+00:00` | `2026-05-01T20:34:35.307344+00:00` |
| 13 | `session-pty-1b442fe2da306e77eeb3768c6f4b712da4d78b31ca5e313dce89ac98853e7a70` | `skills/manifest-generation/SKILL.md` | `succeeded` | 143 | `2026-05-02T10:38:14.234677+00:00` | `2026-05-02T10:38:49.367452+00:00` |

Artifact lookup examples:

| Session | Terminal output or stream artifact |
|---|---|
| `session-pty-8d2add4058ce869c177b8abcaba4138778f9494299d3a9e9444f6b1c80fe80a1` | `artifacts/sessions/2026/04/session-pty-8d2add4058ce869c177b8abcaba4138778f9494299d3a9e9444f6b1c80fe80a1/stream.index.jsonl` |
| `session-pty-bf2f900238b220cc35f4243737b08d97e9029db1550214ee3ad652ac66d99e37` | `artifacts/sessions/2026/04/session-pty-bf2f900238b220cc35f4243737b08d97e9029db1550214ee3ad652ac66d99e37/stream.index.jsonl` |
| `session-pty-a8768f66e8a61216d4d44819daed1d337d15cd8c34b1db87e7848d92629a9d55` | `artifacts/sessions/2026/04/session-pty-a8768f66e8a61216d4d44819daed1d337d15cd8c34b1db87e7848d92629a9d55/stream.index.jsonl` |
| `session-pty-1b442fe2da306e77eeb3768c6f4b712da4d78b31ca5e313dce89ac98853e7a70` | `artifacts/sessions/2026/05/session-pty-1b442fe2da306e77eeb3768c6f4b712da4d78b31ca5e313dce89ac98853e7a70/terminal-output.txt` |

## Appendix D: Core Row Evidence

This appendix records the compact row snapshots behind the stage narrative. These tables are meant for reviewers who want to verify that the document did not silently collapse multiple live rows into a cleaner demo story.

### D.1 Process and Notice Identity

```sql
SELECT p.id, p.canonical_ocid, p.primary_notice_id, p.current_stage,
       p.buyer_name, p.title, p.latest_deadline_at, p.latest_published_at,
       n.source_id, n.source_notice_id, n.procurement_stage, n.access_route
FROM procurement_processes p
JOIN contract_notices n ON n.id=p.primary_notice_id
WHERE p.id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
```

| Field | Value |
|---|---|
| Process ID | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` |
| Canonical OCID | `pc-7046e8ec-2734-ed11-8119-005056b64545` |
| Primary notice ID | `pc-7046e8ec-2734-ed11-8119-005056b64545` |
| Current process stage | `normalize` |
| Buyer | `Bristol City Council` |
| Title | `BSH/HRA/Supply of Double Glazed Units and Sheet Glass - DPS - Acceptance Round 5` |
| Deadline | `2026-11-30T20:00:00Z` |
| Published | `2022-09-14T13:30:00Z` |
| Source | `procontract` |
| Source notice ID | `7046e8ec-2734-ed11-8119-005056b64545` |
| Procurement stage | `tender` |
| Access route | `portal_authenticated` |

### D.2 Lots

```sql
SELECT id, lot_number, title, is_bidable_separately, stage_run_id
FROM notice_lots
WHERE notice_id='pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY lot_number;
```

| Lot ID | Lot | Title | Separately bidable |
|---|---:|---|---:|
| `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1` | 1 | `Supply of Double Glazed Units` | 1 |
| `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-2` | 2 | `Supply of Sheet Glass` | 1 |

Lot interpretation:

| Lot | Why it matters |
|---:|---|
| 1 | Contains the BS EN 1279 double-glazed-unit candidate that became the priced demo thread. |
| 2 | Keeps the source tender honest: the DPS also covers sheet glass, but this golden journey does not follow it because it does not have the same rich landed-cost and decision trail. |

### D.3 Qualification Runs

```sql
SELECT id, lot_id, expiry_qualified, goods_qualified, goods_confidence_bps,
       qualification_basis, rationale, qualified_at
FROM qualification_runs
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY qualified_at;
```

| Qualification ID | Lot ID | Expiry | Goods | Confidence | Basis | Qualified at |
|---|---|---:|---:|---:|---|---|
| `qual-lot-pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1` | 1 | 1 | 8500 | `lot_level` | `2026-04-18 15:15:29` |
| `qual-lot-pc-7046e8ec-2734-ed11-8119-005056b64545-lot-2` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-2` | 1 | 1 | 8500 | `lot_level` | `2026-04-18 15:15:29` |
| `qual-pc-7046e8ec-2734-ed11-8119-005056b64545` | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1` | 1 | 1 | 8500 | `notice_only` | `2026-04-21 18:30:40` |

Qualification interpretation:

| Evidence | Reading |
|---|---|
| Lot 1 and Lot 2 both goods-qualified | The DPS stays inside the physical-goods opportunity lane. |
| Notice-level row also present | Historical or broader prequalification evidence, not a third lot. |
| Confidence 8500 bps | The platform considers the goods signal strong enough to keep processing. |

### D.4 Product Candidates

```sql
SELECT id, lot_id, candidate_role, candidate_type, quoteability_score_bps,
       identity_confidence_bps, spec_confidence_bps, quantity_confidence_bps, name
FROM product_candidates
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY lot_id, id;
```

| Candidate ID | Lot | Quoteability | Identity | Spec | Quantity | Name |
|---|---|---:|---:|---:|---:|---|
| `...candidate-glass-dgu-bs-en-1279` | Lot 1 | 8000 | 8500 | 9200 | 9200 | `Double-glazed unit (sealed insulating glass unit, BS EN 1279-compliant, argon-filled, warm-edge spacer)` |
| `...candidate-glass-stippolytte-6-4mm` | Lot 1 | 5300 | 5300 | 9200 | 9200 | `6.4mm laminated Stippolytte single glazing` |
| `...candidate-lot1-double-glazed-units` | Lot 1 | 6500 | 7000 | 9200 | 9200 | `Double glazed units (insulating glass units)` |
| `...candidate-lot1-double-glazed-units-dps` | Lot 1 | 6000 | 6300 | 9200 | 9200 | `Double-glazed unit (DPS supply category)` |
| `...candidate-lot2-sheet-glass` | Lot 2 | 6000 | 6300 | 9200 | 9200 | `Sheet glass (flat glass)` |
| `...candidate-lot2-sheet-glass-dps` | Lot 2 | 6000 | 6300 | 9200 | 9200 | `Sheet glass (DPS supply category)` |

Candidate interpretation:

| Evidence | Reading |
|---|---|
| Six candidate rows | Product extraction preserved multiple document-derived product interpretations. |
| Target DGU has the highest quoteability | This is why the journey follows that row through marketplace search and margin gates. |
| Lot 2 candidates exist but are not priced in the story | They prove the process has broader scope; they are not the richest demo path. |

### D.5 Supplier Quotes

```sql
SELECT id, source_name, region, quote_class, unit_price_gbp_pence, currency,
       quoted_quantity, quoted_quantity_unit, minimum_order_quantity,
       price_signal_role
FROM supplier_quotes
WHERE product_candidate_id LIKE 'pc-7046e8ec-%'
ORDER BY product_candidate_id, id;
```

| Quote ID | Source | Region | Class | Unit price pence | Quantity | Unit | MOQ | Role |
|---|---|---|---|---:|---:|---|---:|---|
| `sq-6260275402fd43849b430a9cdc6f44d5-0` | `ebay-uk` | `GB` | `spot` | 1850 | 1 | `unit` | | `uk_retail_benchmark` |
| `sq-6260275402fd43849b430a9cdc6f44d5-1` | `ebay-uk` | `GB` | `spot` | 1900 | 1 | `unit` | | `uk_retail_benchmark` |
| `sq-6260275402fd43849b430a9cdc6f44d5-2` | `ebay-uk` | `GB` | `spot` | 44500 | 1 | `unit` | | `uk_retail_benchmark` |
| `sq-b22fcf00b7a545d699c3438c42b9027c-0` | `alibaba-search` | `CN` | `trade` | 579 | 5000 | `piece` | 100 | `cn_supplier_floor` |
| `sq-b22fcf00b7a545d699c3438c42b9027c-1` | `alibaba-search` | `CN` | `trade` | 516 | 10000 | `square meter` | 10 | `cn_supplier_floor` |

Quote interpretation:

| Evidence | Reading |
|---|---|
| UK eBay rows | Useful market sanity checks, but retail spot listings are not the selected supply basis. |
| Alibaba rows | Trade/import price signals; the square-meter row is selected for downstream cost because it matches the operator quantity unit. |
| `sq-b22...-1` | The core quote ID that ties Stage 9 to Stage 10 and Stage 11. |

### D.6 Gross, Landed, and Net Margin Rows

```sql
SELECT cpa.id, cpa.selected_quote_id, cpa.revenue_basis_type,
       cpa.expected_addressable_revenue_ex_vat_pence, cpa.gross_margin_bps,
       lce.id AS lce_id, lce.decision_grade, lce.economic_landed_cost_gbp_pence,
       lce.confidence_bps, cnma.verdict
FROM candidate_pricing_assessments cpa
LEFT JOIN landed_cost_estimates lce
  ON lce.product_candidate_id=cpa.product_candidate_id
 AND lce.supplier_quote_id=cpa.selected_quote_id
LEFT JOIN candidate_net_margin_assessments cnma
  ON cnma.product_candidate_id=cpa.product_candidate_id
WHERE cpa.product_candidate_id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279';
```

| Layer | ID or value |
|---|---|
| Gross pricing assessment | `pricing-pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279` |
| Selected quote | `sq-b22fcf00b7a545d699c3438c42b9027c-1` |
| Revenue basis | `rfq_required` |
| Expected revenue | `0` pence |
| Gross margin | `0` bps |
| Landed-cost row | `landed-pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279-sq-b22fcf00b7a545d699c3438c42b9027c-1` |
| Landed decision grade | `blocked` |
| Economic landed cost | `154800` pence |
| Landed confidence | `0` bps |
| Net-margin verdict | `review` |

Margin interpretation:

| Evidence | Reading |
|---|---|
| `rfq_required` revenue basis | The platform cannot infer the buyer-side value confidently enough from the DPS documents. |
| Selected supplier quote exists | The supply-side cost signal is concrete enough to continue. |
| Landed cost is blocked | Latest customs/freight/browser-dependent skill evidence did not produce a clean import result. |
| Net verdict is `review` | The system refuses to overstate the opportunity and asks for clarification. |

### D.7 Pipeline Assessment

```sql
SELECT process_id, pipeline_state, review_required, validation_state,
       priced_coverage_bps, landed_cost_coverage_bps, net_qualified_coverage_bps,
       unresolved_mandatory_lots, decision_basis_hash, review_stage,
       review_reason, updated_at
FROM pipeline_assessments
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
```

| Field | Value |
|---|---|
| Pipeline state | `manifest_ready` |
| Review required | 1 |
| Validation state | `unverified` |
| Priced coverage | 5000 bps |
| Landed-cost coverage | 5000 bps |
| Net-qualified coverage | 0 bps |
| Unresolved mandatory lots | 1 |
| Decision basis hash | `ecb31264ab603280c5205117e15ef8d53090646185e7a12a38c4631b43e8a965` |
| Review stage | `net_margin_gate` |
| Review reason | `awaiting_rfq_quote` |
| Updated at | `2026-05-02 10:38:49` |

Pipeline interpretation:

| Evidence | Reading |
|---|---|
| `manifest_ready` | The operator can review a packet. |
| `review_required=1` | The system deliberately stops short of automatic approval. |
| `unresolved_mandatory_lots=1` | The process still has at least one unresolved required commercial input. |
| `awaiting_rfq_quote` | The main next action is commercial clarification, not more source scraping. |

### D.8 Manifest and Decision Rows

```sql
SELECT id, stoplight, suggested_action, manifest_basis_hash,
       artifact_hash, artifact_locator, candidate_count,
       priced_candidate_count, clarification_prompt_count, updated_at
FROM manifests
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
```

| Field | Value |
|---|---|
| Manifest ID | `manifest-proc-pc-7046e8ec-2734-ed11-8119-005056b64545` |
| Stoplight | `clarify` |
| Suggested action | `Clarify 1 candidate(s) before deciding` |
| Manifest basis hash | `b0fe9d06a54eb28b2e50aca20215631dd09a024362c5a1d8fe6b437b22560a10` |
| Artifact hash | `d7ac86248d95b1d52318ee6dc7aadaa50e181b038bfac8911ebcd45108e4cbf5` |
| Artifact locator | `artifacts/manifests/proc-pc-7046e8ec-2734-ed11-8119-005056b64545/manifest.md` |
| Candidate count | 1 |
| Priced candidate count | 1 |
| Clarification prompts | 1 |
| Updated at | `2026-05-02 10:38:49` |

```sql
SELECT id, desired_disposition, reviewed_basis_hash, reviewed_manifest_hash,
       idempotency_key, status, superseded_by_intent_id, queued_at, decision_note
FROM pending_operator_intents
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY queued_at;
```

| Intent | Disposition | Status | Idempotency | Queued | Note |
|---|---|---|---|---|---|
| `intent-5045f346fb0246b7a23d9eda308ff073` | `approve` | `superseded` | `dec-2026-05-01-dgu-001` | `2026-05-01 20:16:02` | DGU sized at 100m2; pursue once blockers fixed |
| `intent-a33a4a3f23df45838c1d5d640e440db4` | `watch` | `withdrawn` | `dec-2026-05-01-dgu-002` | `2026-05-01 20:23:09` | Holding while getting a second freight quote |

Decision interpretation:

| Evidence | Reading |
|---|---|
| Manifest stoplight is `clarify` | The product is asking for operator clarification, not approving automatically. |
| Earlier approval intent is superseded | Prior operator action remains audit-visible but is no longer current. |
| Later watch intent is withdrawn | The operator can reverse or pause decisions without deleting history. |
| Active quantity override exists | The durable operator input is the quantity basis, not a current approval. |

## Appendix E: Validation Query Pack

The following query pack is the minimal reproducible path a reviewer can run against the same DB to validate the main claims. Use `sqlite3 -readonly state/state.db` when possible. Avoid `PRAGMA wal_checkpoint` in read-only review because it attempts a write in WAL mode.

### E.1 Pre-flight Counts

```sql
SELECT 'notice' AS entity, COUNT(*) AS count
FROM contract_notices
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
UNION ALL SELECT 'lot', COUNT(*)
FROM notice_lots
WHERE notice_id IN (
  SELECT id FROM contract_notices
  WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
)
UNION ALL SELECT 'qualification_run', COUNT(*)
FROM qualification_runs
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
UNION ALL SELECT 'candidate', COUNT(*)
FROM product_candidates
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
UNION ALL SELECT 'quote', COUNT(*)
FROM supplier_quotes
WHERE product_candidate_id LIKE 'pc-7046e8ec-%'
UNION ALL SELECT 'pricing_assessment', COUNT(*)
FROM candidate_pricing_assessments
WHERE product_candidate_id LIKE 'pc-7046e8ec-%'
UNION ALL SELECT 'lce', COUNT(*)
FROM landed_cost_estimates
WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1'
UNION ALL SELECT 'net_margin', COUNT(*)
FROM candidate_net_margin_assessments
WHERE product_candidate_id LIKE 'pc-7046e8ec-%'
UNION ALL SELECT 'manifest', COUNT(*)
FROM manifests
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
UNION ALL SELECT 'intents', COUNT(*)
FROM pending_operator_intents
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
UNION ALL SELECT 'overrides', COUNT(*)
FROM operator_input_overrides
WHERE product_candidate_id LIKE 'pc-7046e8ec-%';
```

Expected logical counts in this snapshot:

| Entity | Count |
|---|---:|
| `notice` | 1 |
| `lot` | 2 |
| `qualification_run` | 3 |
| `candidate` | 6 |
| `quote` | 5 |
| `pricing_assessment` | 1 |
| `lce` | 1 |
| `net_margin` | 1 |
| `manifest` | 1 |
| `intents` | 2 |
| `overrides` | 1 |

### E.2 Stage 1 and Stage 2

```sql
SELECT id, source_id, source_notice_id, title, buyer_name, procurement_stage,
       access_route, deadline_at, published_at, source_updated_at
FROM contract_notices
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';

SELECT id, canonical_ocid, primary_notice_id, current_stage, buyer_name,
       title, latest_deadline_at, latest_published_at, latest_updated_at
FROM procurement_processes
WHERE id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
```

### E.3 Stage 3

```sql
SELECT *
FROM process_relations
WHERE process_id_a='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
   OR process_id_b='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
```

Expected result: empty set.

### E.4 Stage 4 and Stage 5

```sql
SELECT id, process_id, notice_id, document_set_hash, completeness_state,
       extraction_state, acquired_at, stage_run_id
FROM tender_document_sets
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';

SELECT id, document_set_id, document_title, is_current, section_count,
       table_count, semantic_extraction_state
FROM document_extractions
WHERE document_set_id='tds-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY document_title;

SELECT id, notice_id, lot_number, title, is_bidable_separately, stage_run_id
FROM notice_lots
WHERE notice_id='pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY lot_number;
```

### E.5 Stage 6

```sql
SELECT id, lot_id, expiry_qualified, goods_qualified, goods_confidence_bps,
       legal_category, qualification_basis, rationale, qualified_at, stage_run_id
FROM qualification_runs
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY qualified_at;
```

### E.6 Stage 7 and Stage 8

```sql
SELECT id, lot_id, candidate_key, candidate_role, name, candidate_type,
       match_archetype, head_noun, standards_json, quoteability_score_bps,
       recall_risk_score_bps, identity_confidence_bps, spec_confidence_bps,
       quantity_confidence_bps
FROM product_candidates
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY lot_id, id;

SELECT product_candidate_id, search_eligibility_state, query_waves_json
FROM search_profiles
WHERE product_candidate_id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279';
```

### E.7 Stage 9

```sql
SELECT id, product_candidate_id, source_name, source_url, region, quote_class,
       product_name, quoted_quantity, quoted_quantity_unit, unit_price_gbp_pence,
       currency, minimum_order_quantity, price_signal_role, raw_match_score_bps
FROM supplier_quotes
WHERE product_candidate_id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279'
ORDER BY id;

SELECT id, source_id, status, searched_at, selected_listing_id, stage_run_id
FROM search_attempts
WHERE product_candidate_id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279'
ORDER BY searched_at;
```

### E.8 Stage 10 and Stage 11

```sql
SELECT id, product_candidate_id, selected_quote_id, quote_cost_boundary,
       revenue_basis_type, expected_addressable_revenue_ex_vat_pence,
       revenue_confidence_bps, gross_margin_bps, gross_estimate_confidence_bps,
       identity_gate_passed, usability_gate_passed, pricing_input_confidence_bps
FROM candidate_pricing_assessments
WHERE product_candidate_id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279';

SELECT id, product_candidate_id, supplier_quote_id, shipping_mode,
       supplier_cost_gbp_pence, freight_base_gbp_pence, customs_duty_gbp_pence,
       import_vat_gbp_pence, economic_landed_cost_gbp_pence, revenue_basis_type,
       decision_grade, confidence_bps, confidence_breakdown, warnings, blockers,
       updated_at
FROM landed_cost_estimates
WHERE product_candidate_id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279';

SELECT id, product_candidate_id, reason_code, payload_json, notes,
       basis_hash_when_provided, provided_at, withdrawn_at, superseded_by_override_id
FROM operator_input_overrides
WHERE product_candidate_id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279'
ORDER BY provided_at;
```

### E.9 Stage 12 and Stage 13

```sql
SELECT product_candidate_id, verdict, reasons_json, re_evaluation_eligible,
       net_margin_low_bps, net_margin_base_bps, net_margin_high_bps,
       revenue_used_pence, revenue_basis_type, economic_landed_cost_used_pence,
       basis_hash, stage_run_id, updated_at
FROM candidate_net_margin_assessments
WHERE product_candidate_id='pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279';

SELECT process_id, pipeline_state, review_required, validation_state,
       priced_coverage_bps, landed_cost_coverage_bps, net_qualified_coverage_bps,
       unresolved_mandatory_lots, decision_basis_hash, review_stage,
       review_reason, updated_at
FROM pipeline_assessments
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';

SELECT id, stoplight, suggested_action, suggested_action_reasoning,
       manifest_basis_hash, artifact_hash, artifact_locator,
       candidate_count, priced_candidate_count, clarification_prompt_count,
       stage_run_id, updated_at
FROM manifests
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
```

### E.10 Stage 14

```sql
SELECT id, desired_disposition, reviewed_basis_hash, reviewed_manifest_hash,
       reviewed_at_stage_run_id, reviewed_at, idempotency_key, status,
       superseded_by_intent_id, queued_at, decision_note, last_attempt_at, last_error
FROM pending_operator_intents
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY queued_at;

SELECT id, product_candidate_id, reason_code, payload_json, notes,
       basis_hash_when_provided, provided_at, withdrawn_at, superseded_by_override_id
FROM operator_input_overrides
WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY provided_at;
```

### E.11 Artifacts

```bash
find artifacts/ -path '*7046e8ec*' | sort | head -50
wc -l artifacts/manifests/proc-pc-7046e8ec-2734-ed11-8119-005056b64545/manifest.md
sed -n '1,80p' artifacts/manifests/proc-pc-7046e8ec-2734-ed11-8119-005056b64545/manifest.md
```

The manifest file inspected during this handoff was:

| Artifact | Evidence |
|---|---|
| `artifacts/manifests/proc-pc-7046e8ec-2734-ed11-8119-005056b64545/manifest.md` | 186 lines |
| `artifact_hash` | `d7ac86248d95b1d52318ee6dc7aadaa50e181b038bfac8911ebcd45108e4cbf5` |
| `stoplight` | `clarify` |
| `suggested_action` | `Clarify 1 candidate(s) before deciding` |

### E.12 Read-only Hash Check

```bash
sha256sum state/state.db
stat -f '%Sm %z %N' state/state.db
```

Observed after the document draft:

| File | Hash / size evidence |
|---|---|
| `state/state.db` | `a60810425b78c377e2e60ad98dd1fc8563e3b801f52371005f01aa578c9cc0c1` |
| `state/state.db` mtime/size | `May 2 15:12:28 2026`, `160681984` bytes |

The logical DGU counts after this observation still matched Appendix E.1. WAL and SHM companion files are intentionally not treated as stable audit artifacts because SQLite can update them during read coordination.

## Appendix F: Acceptance Criteria Trace

| Criterion | Status | Evidence |
|---|---|---|
| R.1 | Done | This file exists at `docs/pipeline/golden-journey-DGU.md`. |
| R.2 | Done | Stages 1-14 each have a section with the requested template headings. |
| R.3 | Done | Each stage section includes at least one DB row ID, artifact path, or session ID. |
| R.4 | Done | Each stage section includes a strategy or validation-doc reference. |
| R.5 | Done with correction | Executive summary traces DGU through the 14-stage pipeline and corrects the live buyer identity. |
| R.6 | Done | Findings section lists data-quality and demo-readiness rough edges. |
| R.7 | Done | Cross-reference index included. |
| R.8 | Done | Domain terms are defined in Setup; stage prose is written for a technical stakeholder unfamiliar with the codebase. |
| R.9 | Done | Each stage includes verification SQL for DB-state claims. |
| R.10 | Caveated | This handoff did not re-run stages or intentionally write state. The file-level DB hash changed in the shared workspace during validation; logical DGU counts were re-checked and stayed stable. |
| R.11 | Skipped by design | Appendix A documents why no screenshot was captured. |
| R.12 | Caveated | `cargo check --all-targets` passed. `cargo test -p daemon --bin daemon` passed 169/169. `cargo fmt --check` failed on pre-existing formatting in `apps/desktop/src-tauri/src/main.rs:1889`, outside this documentation handoff. |
