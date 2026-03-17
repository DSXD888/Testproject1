# Pre-Prep: Pipeline Integration & End-to-End Flow

## Step 1: Pre-Prep Research

Read the following documents in this exact order. Do not skip any.

1. **AGENTS.md** — project constraints, toolchain, rules
2. **The research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Read the ENTIRE document, but pay special attention to:
     - §5.3 (Pipeline Flow — full stage sequence from ingestion through bid workflow, stage transitions, pipeline orchestration)
     - §5.5 (State Model — PipelineAssessment states, stage transitions, operator decision states, bid workflow states)
     - §5.6 (Ingestion Data Flow — scheduled ingestion steps, skill invocation, normalization, sync trigger points)
     - §5.7 (Intelligence Pipeline Data Flow — price-signal matching, shortlisting, landed cost, net margin gate, manifest generation)
     - §5.4 (Data Models — all entities and tables, especially process_stage_status, pipeline_assessments, workflow_jobs, NoticeLot, ProductCandidate, SupplierQuote, LandedCostEstimate)
     - §4.2 (Procurement Intelligence Pipeline — the value chain from notice to bid)
     - §4.3 (Supplier Price Intelligence — source routing, query waves, matching)
     - Appendix A (Margin Logic — gross margin gate, net margin gate, landed cost formula)
     - Appendix C (Job Kinds — job type registry, input/output contracts per job kind)
3. **The handoff prompt that produced the feedback:** `docs/planmaxer/active/handoffs/handoff-pipeline-integration.md`

After reading, you should be able to explain:
- The full pipeline stage sequence: ingest → normalize → relate → qualify → acquire docs → extract lots → extract products → score quoteability → price-signal match → gross margin gate → landed cost → net margin gate → manifest → operator decision → bid workflow
- The state model: how PipelineAssessment tracks overall process state, how process_stage_status tracks per-stage completion, how stage heads enforce ordering
- The ingestion data flow: scheduled triggers, skill execution, document parsing, normalization, deduplication, and sync
- The intelligence pipeline: how qualified processes flow through matching, pricing, margin analysis, and manifest generation
- The job runner's role: how workflow_jobs drive pipeline stages, parent_job_id for sub-jobs, validation sub-jobs
- Appendix A margin arithmetic: revenue normalization, quote normalization to common cost boundary, gross margin gate (>= 30% + confidence floor), net margin gate (low >= 20% + no critical blockers), cash_to_clear computation
- Appendix C job kinds: how each pipeline stage maps to a job kind with typed input/output contracts
- The cardinality chain: ProcurementProcess (1) → ContractNotice (1..N) → NoticeLot (1..M) → ProductCandidate (0..P) → SearchProfile (1) → SupplierQuote (0..Q) → LandedCostEstimate (0..R)
- The process_stage_status / process_stage_heads atomic advancement pattern: all outputs written with shared stage_run_id in the same SQLite transaction as stage head advancement and outbox entries
- The two separate granularities: ProcessStageStatus.stage is coarse (ingestion, qualification, enrichment, landed_cost, manifest, bidding) while PipelineAssessment.pipelineState is fine-grained (15 business states)
- The gross margin gate sequencing question: revenue normalization is described in §5.7 landed cost steps 1-3, but the gross margin gate (stage 10) fires BEFORE landed cost (stage 11) — how does the gross margin gate access revenue normalization data?

**File Ownership:** This pre-prep owns the following sections for editing:
- §5.3 (Pipeline Flow) — FULL ownership
- §5.5 (State Model) — FULL ownership
- §5.6 (Ingestion Data Flow) — FULL ownership (except validation steps 12a-12d, owned by worker-validator domain)
- §5.7 (Intelligence Pipeline Data Flow) — FULL ownership (except landed-cost subsection internals, owned by customs-classification domain)
- Appendix A (Margin Logic) — Margin gate thresholds and formulas ONLY (not customs rate lookups)
- Appendix C (Job Kinds) — FULL ownership

Do NOT edit: §4.2 (pipeline overview — that's the conceptual design, not the implementation flow), §4.7 (database architecture), §4.9 (daemon orchestration internals), §5.2 (component designs), §5.4 (SQL table definitions — note changes for sync-engine domain), §4.10 (skill types). The pipeline flow describes HOW stages execute, not WHAT the daemon architecture looks like. If the reviewer suggests changes to daemon internals, component designs, or SQL schema, note them as deferred with `<!-- CROSS-DOMAIN: ... -->` comments.

---

## Step 2: Feedback Integration

You’re right: the stage designs are mostly sane. The integrated system is not.

The core problem is **grain drift**.

The pipeline’s real unit of work changes as it moves:

**process → notice → lot-set → lot → candidate → candidate-source pair → quote → landed-cost scenario → process rollup**

But the control model in §5.4/§5.5 stays mostly **process-level** and **single-headed**. That is where the corruption will happen.

If this ships as written, you will get clean-looking wrongness:

* `net_qualified` processes with only one small candidate actually priced
* landed-cost estimates that cannot be tied back to the exact quote revision and doc set that produced them
* re-acquired documents that never invalidate stale candidates
* quoteability/search/margin logic that looks deterministic but is actually conditional on hidden upstream state

## Bottom line

The biggest integration bugs are these:

1. **Stage 4 is defined as per-lot, but persisted as per-process/per-notice.**
2. **The 15-stage pipeline is tracked with stage metadata that is too coarse to rerun or invalidate correctly.**
3. **Stages 7–12 are candidate/quote scoped, but `PipelineAssessment` is process scoped, so rollups lie.**
4. **Stage 5 has no first-class output object.** Docs are acquired, but there is no durable `document_set` contract for downstream stages to depend on.
5. **The identity chain exists only if a bunch of nullable FKs happen to be populated.**
6. **Stage 9 produces three important pricing truths (`matchConfidence`, `quoteUsabilityConfidence`, `quoteRealismConfidence`), then Appendix A mostly uses one and drops the other two.**
7. **There is no retroactive invalidation model.**
8. **`needs_review` and `expired` are modeled as pipeline states when they are really overlays.**
9. **`stage_run_id` is overloaded.** It is being asked to mean “session”, “atomic commit cohort”, and “freshness/version anchor”. It should mean one thing.

---

## 1) Concrete trace: one procurement process through all 15 stages

Example process:

**ProcurementProcess P-9001**
“Supply of PPE and ward consumables to Northshire NHS Trust”

Realistic structure:

* 2 source notices
* 3 lots
* 5 extracted candidates
* 4 search profiles actually executed
* ~8 search attempts across sources/waves
* ~7 supplier quotes
* ~3 landed-cost scenarios
* 1 manifest
* 1 operator decision
* optional bid workflow

### Stage 1 — INGEST

Created:

* `ContractNotice N-FTS-1` — original tender notice
* `ContractNotice N-FTS-2` — amendment / updated deadline notice
* `raw_documents RD-1`, `RD-2` — raw scrape payloads

Fields that matter next:

* `sourceNoticeId`
* `ocid`
* `publishedAt`
* `sourceUpdatedAt`
* `deadlineAt`
* `cpvCodes`
* `estimatedValueOriginal`
* `rawDocumentId`

Seam problem:

* there is no explicit **material content hash** for the notice payload that downstream invalidation can key off

### Stage 2 — NORMALIZE

Updated:

* `contract_notices.normalized_payload`
* `notice_cpv_codes`

Fields that matter next:

* normalized stage
* normalized CPVs
* normalized deadline / value / category hints

Seam problem:

* normalization updates the notice row, but there is no durable “this normalized version is what relation resolution / qualification consumed” basis snapshot

### Stage 3 — RELATE

Created/updated:

* `ProcurementProcess P-9001`
* maybe `process_relations` if there are related-but-not-merged notices
* `primaryNoticeId = N-FTS-2`

Fields that matter next:

* `processId`
* `canonicalOcid`
* relation classification

Seam problem:

* if a late-arriving notice later changes merge semantics, `process_aliases` only helps deep links; it does not solve downstream row reconciliation

### Stage 4 — QUALIFY

Intended business outcome:

* Lot 1 qualifies
* Lot 2 qualifies
* Lot 3 should be rejected or review-only because it is service-dominant

Actual persisted shape in §5.4:

* `QualificationResult` / `qualification_runs` has `processId` and `noticeId`
* **no `lotId`**

This is the first hard contract break.

Fields that matter next:

* expiry decision
* access route
* goods density / separability
* addressable goods value

Seam problem:

* the stage is described in §5.3/§5.7 as **per lot**
* the data model stores it **per process / per notice**
* lot-specific qualification truth cannot survive

### Stage 5 — ACQUIRE DOCS

Created:

* tender spec PDF
* pricing schedule spreadsheet
* appendix for service lot

Example artifacts:

* `TD-1 spec.pdf`
* `TD-2 pricing_schedule.xlsx`
* `TD-3 service_appendix.pdf`

Fields that matter next:

* document roles
* document completeness
* document-set hash
* acquisition quality

Current model:

* generic `raw_documents`
* no first-class `document_set`

Seam problem:

* downstream stages cannot tell **notice-only**, **partial docs**, and **full docs**
* this kills invalidation and confidence gating

### Stage 6 — EXTRACT LOTS

Created:

* `NoticeLot L-1` — nitrile gloves + aprons
* `NoticeLot L-2` — masks + visors
* `NoticeLot L-3` — managed vending / stock-control service

Fields that matter next:

* lot title / description
* lot value
* lot deadline
* lot CPV
* lot document refs

Seam problem:

* lot extraction is a **set-replacement** stage
* if extraction succeeds for lots 1–2 and fails on 3, a partial authoritative commit is unsafe
* current `stage_run_id` model does not say that clearly

### Stage 7 — EXTRACT PRODUCTS

Created:

* `ProductCandidate C-1` — nitrile examination gloves EN 455
* `ProductCandidate C-2` — disposable plastic apron
* `ProductCandidate C-3` — type IIR surgical mask EN 14683
* `ProductCandidate C-4` — face visor
* `ProductCandidate C-5` — possible garbage candidate from service appendix if extraction quality is poor

Fields that matter next:

* `candidateType`
* `matchArchetype`
* `headNoun`
* standards
* quantity/unit
* identity/spec/quantity confidence

Seam problems:

* `lotId` is optional in the interface
* there is no parent/child lineage for bundle decomposition
* there is no structured “extracted from document/page/span” provenance

### Stage 8 — SCORE QUOTEABILITY

Updated / created:

* `quoteabilityScore`
* `recallRiskScore`
* `SearchProfile SP-1`..`SP-4` for passing candidates

Example:

* C-1 = 0.87
* C-2 = 0.71
* C-3 = 0.83
* C-4 = 0.62
* C-5 = 0.29

Seam problems:

* the policy says **0.45–0.64 => search only top-value/top-risk candidates**
* but `ProductCandidate` has no durable **candidate value share**
* “top-value” is undefined in the persisted model
* below-threshold candidates become a dead-end because search intent is not persisted for all candidates

### Stage 9 — PRICE-SIGNAL MATCH

Created:

* `SearchAttempt SA-*`
* `SupplierQuote Q-*`
* local-only `ListingEvidence LE-*`

Example:

* `Q-1` Amazon UK gloves, box of 100
* `Q-2` Alibaba gloves, carton/MOQ listing
* `Q-3` eBay aprons pack
* `Q-4` Amazon masks
* etc.

Fields that matter next:

* `matchConfidence`
* `quoteUsabilityConfidence`
* `quoteRealismConfidence`
* `priceSignalRole`
* `normalizedUnitBasis`
* `quoteCostBoundary`
* `searchFailureMode`

Seam problems:

* the durable business unit is explicitly said to be **candidate-source pair**
* but stage tracking is still process-level
* `SupplierQuote -> ListingEvidence` is only implicit via `(source, sourceListingId)`
* persisted quotes have no first-class freshness contract

### Stage 10 — GROSS MARGIN GATE

Business outcome:

* choose which quote(s) are commercially usable enough to continue
* compare normalized supplier cost vs normalized addressable revenue

What must exist here:

* selected quote id(s)
* quote row version(s)
* selected revenue basis
* gross low/base/high or at least selected scenario
* selection reason

What exists in §5.4:

* **nothing durable for this stage**
* only process-level `PipelineAssessment.grossMarginBps`

This is the second hard contract break.

### Stage 11 — LANDED COST

Created:

* `LandedCostEstimate LC-1` for C-1/Q-2
* `LC-2` for C-2/Q-3
* `LC-3` maybe for C-3/Q-4

Fields that matter next:

* selected quote
* HS code candidate set
* chosen 10-digit code
* tariff snapshot
* FX snapshot(s)
* freight snapshot
* blockers
* low/base/high net margin

Seam problems:

* `LandedCostEstimate` can be “per quote or per candidate” in the prose; that ambiguity is fatal
* `supplierQuoteId` is optional
* there is no persisted `basedOnSupplierQuoteRowVersion`
* no first-class snapshot refs for tariff / FX / freight

### Stage 12 — NET MARGIN GATE

Business outcome:

* candidate-level pass/review/fail using low scenario + blockers

Actual process-level state:

* `PipelineAssessment.pipelineState = net_qualified`

Seam problem:

* the process can now look qualified even if only one candidate out of five is actually qualified
* coverage is not modeled

### Stage 13 — MANIFEST

Created:

* `Manifest M-1`

Seam problem:

* manifest is process-level and mutable
* there is no `basis_hash` tying it to the exact candidates/quotes/estimates it narrates

### Stage 14 — OPERATOR DECISION

Created/updated:

* `OperatorDecisionHead O-1`

Seam problem:

* decision is process-level
* there is no snapshot of the exact basis the operator reviewed
* if upstream data later changes, decision staleness is invisible

### Stage 15 — BID WORKFLOW

Created/updated:

* `BidWorkflowState`

Seam problem:

* bid scope is process-level
* no explicit lot/candidate subset is stored
* that is unsafe for mixed-lot or partially qualified processes

---

## Identity-chain audit

This is the chain you care about most.

Current strength:

| Link                                                 | Current state | Why it is weak                                               |
| ---------------------------------------------------- | ------------- | ------------------------------------------------------------ |
| `ProcurementProcess -> ContractNotice`               | medium        | FK exists, but late merge/split can change canonical meaning |
| `ContractNotice -> NoticeLot`                        | medium        | FK exists                                                    |
| `NoticeLot -> ProductCandidate`                      | weak          | `lotId` is optional                                          |
| `ProductCandidate -> SupplierQuote`                  | weak          | `productCandidateId` is nullable in the SQL                  |
| `SupplierQuote -> LandedCostEstimate`                | weak          | `supplierQuoteId` is optional, no quote row-version pin      |
| `LandedCostEstimate -> Manifest / Operator decision` | absent        | no basis hash / selected input snapshot                      |

So: **the FK chain is not actually guaranteed end-to-end**, and the “why” chain is weaker than the FK chain.

---

## 2) Specific integration bugs at stage boundaries

### 1. Stage 4 says “per lot”; §5.4 stores it per process/notice

**Refs:** §5.3 Stage 4, §5.7 Preliminary Qualification, §5.4 `QualificationResult` / `qualification_runs`

This is a real bug, not a modeling preference.

Stage 4 decisions are supposed to vary by lot:

* deadline
* access route
* goods density
* separability
* reject/review/pass

But the persisted entity has no `lotId`. That means stage 5/6/7 cannot know which lots were qualified, only that the process/notice was.

**Fix:** make qualification results lot-scoped, with a provisional mode when lots are not yet explicit.

---

### 2. Goods-only addressable value is required by the workflow, but not persisted

**Refs:** §5.7 qualification step 6, Appendix A revenue normalization, §5.4 data model

The doc explicitly says mixed contracts with separable goods should be priced on the **goods package**, not the whole contract.

But there is no durable field for:

* `goods_addressable_value_pence`
* `candidate_value_share_bps`
* `lot_addressable_value_pence`

So stage 10 can easily fall back to whole-notice value, which overstates margin.

**Fix:** persist addressable value at lot level, and candidate value share inside the lot. If candidate value share is unknown, process-level auto-qualification must stop.

---

### 3. Qualification is upstream of document acquisition, but many qualification decisions need documents

**Refs:** §5.3, §5.7 Preliminary Qualification vs Document Acquisition

This is a causal inversion.

For mixed or messy notices, you often cannot know:

* goods separability
* actual lot boundaries
* whether the goods are bidable separately

until you inspect the docs.

But docs acquisition only happens after `prelim_qualified`.

**Fix:** allow **provisional qualification** that can still trigger stage 5 for ambiguous cases. No new stage needed; just add `qualificationBasis = notice_only | notice_plus_docs | lot_level`.

---

### 4. Stage 5 has no first-class output contract

**Refs:** §5.7 Document Acquisition, §5.4 `raw_documents`

Stage 5 is not “download some files”. It produces a semantic input to later stages:

* which docs were found
* what roles they have
* whether the set is partial/full
* the hash of the input set used by extraction

Right now it only lands in generic artifacts.

That means stages 6–8 cannot depend on a stable document basis, and you cannot invalidate downstream outputs when the doc set changes.

**Fix:** introduce a first-class `TenderDocumentSet` / document-acquisition result entity.

---

### 5. Lot extraction is a set-replacement stage, but the commit model doesn’t say that

**Refs:** §5.6, §5.7, `stage_run_id` / `process_stage_heads`

Lot extraction is not additive. A notice’s lot set is one coherent decomposition of scope.

So if lots 1–7 succeed and lot 8 fails, authoritative commit of 1–7 is unsafe. You do not yet have the lot set.

**Fix:** treat stage 6 as **atomic per notice + document set hash**. Partial outputs may be stored as checkpoint/debug artifacts, but must not become authoritative heads.

---

### 6. Product extraction contract is missing lineage

**Refs:** §5.7 Product Extraction, §5.4 `ProductCandidate`

Stage 7 promises:

* bundle decomposition
* candidate typing
* archetype classification
* structured slots

But the entity is missing:

* `parentCandidateId` / `bundleGroupId`
* required `lotId`
* provenance back to document/page/span
* candidate value share

So stage 8 and later stages receive candidates with too little lineage.

**Fix:** require lot linkage, add candidate lineage/provenance, and store value-share when decomposition occurs.

---

### 7. The below-quoteability path is under-modeled

**Refs:** §5.5 below-quoteability path, §5.4 `SearchProfile`, `OperatorDecisionHead`

Today:

* `< 0.45` skips search
* operator can watch/reject/RFQ manually

But:

* no pricing data exists
* no structured search intent is guaranteed to exist
* RFQ re-entry is not modeled cleanly

This becomes an information dead-end.

**Fix:**

* create `SearchProfile` for **all** candidates, not just auto-searched ones
* add `searchEligibilityState = auto_search | manual_only | skip`
* RFQ re-entry should normalize into `SupplierQuote(quoteClass = rfq_quote | negotiated_quote)` and re-enter at stage 10

---

### 8. Stage 9 splits pricing confidence correctly; Appendix A throws two-thirds of it away

**Refs:** §4.3 dual confidence model, §5.7 Stage 9, Appendix A gross margin logic, Appendix B

This is one of the nastiest seam bugs.

Stage 9 carefully distinguishes:

* `matchConfidence`
* `quoteUsabilityConfidence`
* `quoteRealismConfidence`

But Appendix A gross gate uses:

```text
gross_estimate_confidence = min(revenue_confidence, quote_realism_confidence)
```

So a quote can be:

* a weak product identity match
* on a fuzzy pack basis
* but still gross-pass because `quoteRealismConfidence` is high

That is exactly how a 100x pack-basis error survives.

**Fix:** stage 10 must gate on **identity floor first**, then **usability floor**, then realism.

---

### 9. Stage 10 has no persisted output object

**Refs:** §5.3 Stage 10, Appendix A, §5.4

Stage 10 must make a selection:

* which quote(s) continue
* on what revenue basis
* with what normalization
* with what confidence

That selection is not persisted.

So stage 11 can easily use a different quote/basis than stage 10 did.

**Fix:** add a candidate-scoped pricing assessment object between stages 9 and 11. No new stage, just a real output contract.

---

### 10. Stage 11 grain is ambiguous: “per quote or per candidate”

**Refs:** user’s cardinality chain, §5.4 `LandedCostEstimate`, §5.7 Landed Cost

That “or” is dangerous.

A landed-cost estimate should be anchored to a single commercial scenario. In practice that means:

* candidate + selected quote
* plus tariff/FX/freight snapshots

If sometimes it is per quote and sometimes per candidate, stage 12 cannot reason consistently.

**Fix:** make landed-cost estimates **candidate-quote scenario scoped**. Candidate rollups happen after that.

---

### 11. Process-level `PipelineAssessment` loses candidate truth

**Refs:** §5.4 `PipelineAssessment`, §5.5 state model

Stages 7–12 are not process-grain. They are lot/candidate/quote-grain.

A process-level enum without coverage semantics allows false positives:

* one easy glove candidate can make the whole process look `priced` or `net_qualified`
* the hard 60% of value can remain unresolved and invisible

**Fix:** keep process state as a rollup over scoped heads, with coverage metrics.

---

### 12. `needs_review` and `expired` should not be primary pipeline states

**Refs:** §5.5 state notes

Both are overlays:

* `needs_review` can happen from multiple stages
* `expired` can happen from any active state

Making them primary states destroys progression context.

**Fix:** keep:

* `pipelineState` = actual processing frontier
* `reviewRequired` + `reviewStage`
* `availabilityStatus = active | expired | cancelled`

---

### 13. Stage tracking is too coarse for a 15-stage pipeline

**Refs:** §5.3, §5.4 `ProcessStageStatus`, `ProcessStageHeads`, `WorkflowJob.kind`

This is the orchestration/control-plane version of the grain bug.

You defined 15 stages in prose, but stage heads/jobs are mostly tracked as:

* ingestion
* qualification
* enrichment
* landed_cost
* manifest
* bidding

That is too blunt to do:

* targeted reruns
* targeted invalidation
* targeted coverage
* partial fan-out recovery

**Fix:** the tracked stage set must align with the rerunnable boundaries, not a simplified UI grouping.

---

### 14. Late merge/split after downstream work is under-specified

**Refs:** §5.6 relation resolution, §5.4 `process_relations`, `process_aliases`

`process_aliases` fixes routing. It does not fix data integrity.

If two processes are enriched separately, then later merged, what happens to:

* quotes
* landed-cost estimates
* manifests
* operator decisions
* stage heads

You need a stable lineage key / cluster key.

**Fix:** introduce an immutable `processClusterId` (or equivalent lineage key). `processId` can remain the current canonical shell, but rollups must be cluster-aware.

---

### 15. Operator decisions and bid workflow have no basis/scope snapshot

**Refs:** §5.5 OperatorDecisionHead, §5.9 bid workflow

The operator decision is not pinned to:

* manifest hash
* basis hash
* selected candidate ids
* selected lot ids

That means:

* a decision can become stale invisibly
* bid workflow scope is ambiguous in multi-lot / partially qualified cases

**Fix:** decisions must record the reviewed basis hash, and bid jobs must carry explicit lot/candidate scope.

---

## 3) Retroactive invalidation / staleness propagation

You do not need a new workflow engine for this.
You need **basis vectors**, **scope-aware heads**, and an **invalidation queue** in SQLite.

## The model

Every successful scoped stage commit should persist:

* `stage_run_id`
* `scope_kind`
* `scope_id`
* `input_stage_vector_json`
* `rule_version_json`
* `external_snapshot_json`
* `basis_hash`
* `freshness_state`
* `supersedes_stage_run_id?`

### Example basis vector

For a landed-cost run on candidate C-1 / quote Q-2:

```json
{
  "extract_products": "run_pr_118",
  "score_quoteability": "run_qs_52",
  "price_match": "run_pm_c1_alibaba_7",
  "selected_quote_id": "Q-2",
  "selected_quote_row_version": 3,
  "tariff_snapshot": "hmrc_tariff_2026-03-15",
  "customs_fx_snapshot": "hmrc_fx_2026-03",
  "commercial_fx_snapshot": "xe_2026-03-15T09:00:00Z",
  "freight_snapshot": "freightos_2026-03-15T09:05:12Z",
  "classifier_version": "hs_v4",
  "freight_model_version": "fm_v2"
}
```

That is what the estimate is *based on*. Right now the doc stores some values, but not the basis.

## Hard invalidation vs soft staleness

### Hard invalidation

The semantic input changed. Downstream outputs are no longer authoritative.

Examples:

* new document set acquired
* lot set changed
* product extraction changed
* relation resolution merged/split a process cluster
* quoteability formula or lexicon changed **and** score band / query fingerprint changed

Action:

* mark downstream scoped heads `hard_invalid`
* keep old rows for audit
* exclude them from current rollup
* enqueue rerun from the earliest invalid stage

### Soft staleness

The semantics are still valid, but time-sensitive inputs have aged.

Examples:

* quote TTL expired
* FX snapshots aged out
* freight quote aged out
* tariff snapshot changed without product semantics changing

Action:

* mark rows `soft_stale`
* keep them viewable
* block auto-qualification if the soft-stale input is decision-critical
* rerun only the affected later stages

## Cascade matrix

### Re-run triggers that should cascade

* **Notice material hash changed**
  Re-run: `normalize -> relate -> qualify`
  Cascade further only if normalized fields materially changed.

* **Document set hash changed**
  Re-run: `extract_lots -> extract_products -> score_quoteability -> price_match -> gross_gate -> landed_cost -> net_gate -> manifest`

* **Lot set changed**
  Re-run: `extract_products` onward for affected notice/lots

* **Product candidate lineage/archetype changed**
  Re-run: `score_quoteability` onward for affected candidates

* **Quoteability formula version changed**
  Re-run `score_quoteability` for affected candidates
  Cascade to `price_match` only if:

  * the candidate crossed a band boundary, or
  * the query fingerprint changed

* **Lexicon/query planner version changed**
  Re-run `price_match` only where `query_profile_hash` changes

* **SupplierQuote expired / refreshed**
  Re-run `gross_gate -> landed_cost -> net_gate`
  Re-run `price_match` only if no fresh quote remains

* **FX move / freight refresh / tariff refresh**
  Re-run `landed_cost -> net_gate` only

* **Manual RFQ quote arrives**
  Insert new `SupplierQuote(quoteClass=rfq_quote)`
  Re-run `gross_gate -> landed_cost -> net_gate` for that candidate only

## Scheduler rule

Every job should carry `expected_input_basis_hash` (or expected upstream stage vector).
At claim/start time, the Job Runner compares it to current scope heads.

If mismatched:

* do **not** run stale work
* mark job obsolete / rebase it
* enqueue a new job from the correct earliest invalid stage

That is your guard against out-of-order execution and stale retries.

---

## 4) Error propagation: what fails loudly, what fails silently, what is ambiguous

This is the short version by stage.

### 1. INGEST

* **Loud:** scrape failure, source block, invalid result contract
* **Silent:** replayed old notice version treated as current; cursor/backfill duplication
* **Ambiguous:** partial payload from source
* **Downstream actually checks:** mostly uniqueness and timestamps

### 2. NORMALIZE

* **Loud:** schema/parse failure
* **Silent:** wrong deadline normalization, wrong CPV/category projection
* **Ambiguous:** mixed/unknown categorization
* **Downstream actually checks:** normalized fields, not normalization confidence

### 3. RELATE

* **Loud:** almost nothing; relation errors are usually not explicit failures
* **Silent:** wrong merge or wrong separation
* **Ambiguous:** `possible_duplicate` with no stable cluster semantics
* **Downstream actually checks:** `processId` becomes fact

### 4. QUALIFY

* **Loud:** explicit reject, missing deadline, non-bidable route
* **Silent:** lot-varying notice treated as process-level truth; goods-only addressable value lost
* **Ambiguous:** mixed/uncertain goods separability
* **Downstream actually checks:** bools and access route; `goodsConfidence` is mostly parked

### 5. ACQUIRE DOCS

* **Loud:** download/auth failure
* **Silent:** wrong docs, partial docs, login HTML saved as document
* **Ambiguous:** no docs available vs docs not found yet
* **Downstream actually checks:** mostly docs-present vs not; not completeness quality

### 6. EXTRACT LOTS

* **Loud:** extraction crash
* **Silent:** incomplete or wrong lot partition
* **Ambiguous:** synthetic whole-notice lot created when true lots exist
* **Downstream actually checks:** lot rows as if authoritative; no lot-set confidence

### 7. EXTRACT PRODUCTS

* **Loud:** extraction crash, invalid contract
* **Silent:** garbage candidates with valid schema, wrong bundle decomposition, wrong archetype
* **Ambiguous:** low identity/spec/quantity confidence
* **Downstream actually checks:** `candidateType`, `matchArchetype`, slots; the three granular confidences are not strongly gating

### 8. SCORE QUOTEABILITY

* **Loud:** scoring failure
* **Silent:** stale formula / stale lexicon / stale doc basis
* **Ambiguous:** mid-band candidates with undefined “top-value” semantics
* **Downstream actually checks:** `quoteabilityScore`, `recallRiskScore`; no formula-version basis

### 9. PRICE-SIGNAL MATCH

* **Loud:** source failure, source block, invalid result contract
* **Silent:** wrong accessory, wrong pack basis, stale listing price, consumer/commercial mismatch
* **Ambiguous:** medium-confidence identity or medium usability
* **Downstream actually checks:** stage 9 itself uses `matchConfidence` and `quoteUsabilityConfidence`; Appendix A later mostly drops them

### 10. GROSS MARGIN GATE

* **Loud:** no usable quote / explicit review path
* **Silent:** wrong revenue basis, wrong quote selection, wrong cost boundary
* **Ambiguous:** positive gross margin on screening-only quote
* **Downstream actually checks:** not enough, because selected inputs are not persisted

### 11. LANDED COST

* **Loud:** HS unresolved, API failure, trade remedy hit, measure blocker
* **Silent:** wrong HS convergence, wrong freight mode, wrong VAT recovery assumption
* **Ambiguous:** decision grade `review` / `rfq_required` / `broker_required`
* **Downstream actually checks:** blockers and margin; several confidence components are parked

### 12. NET MARGIN GATE

* **Loud:** blocker / fail
* **Silent:** process rollup ignores coverage and unresolved candidates
* **Ambiguous:** one candidate qualifies, most value unresolved
* **Downstream actually checks:** process state, not candidate coverage

### 13. MANIFEST

* **Loud:** render failure
* **Silent:** stale or overconfident narrative
* **Ambiguous:** partial-coverage cases narrated as whole-process truth
* **Downstream actually checks:** operator reads the narrative; no basis hash

### 14. OPERATOR DECISION

* **Loud:** CAS conflict
* **Silent:** decision on stale data
* **Ambiguous:** watch/pending on below-quoteability candidates with no re-entry contract
* **Downstream actually checks:** process-level approval only

### 15. BID WORKFLOW

* **Loud:** portal failure
* **Silent:** wrong lot subset or wrong scope submitted
* **Ambiguous:** `ready_to_bid` for a process whose qualifying basis is stale
* **Downstream actually checks:** process approval + bid state, not reviewed scope

---

## Confidence scores: actually consumed vs mostly parked

### Actually consumed in control flow

* `quoteabilityScore`
* `recallRiskScore`
* `matchConfidence`
* `quoteUsabilityConfidence`
* `priceSignalRole`
* `quoteRealismConfidence` (Appendix A gross confidence)
* `blockers`
* `netMarginLowPct`

### Mostly stored but not decisive enough

* `goodsConfidence`
* `identityConfidence`
* `specConfidence`
* `quantityConfidence`
* `shippingBasisConfidence`
* `modeConfidence`
* `classificationStability` as an independent gate
* `revenueConfidence` as an independent gate
* `decisionGrade` as a hard gate
* `validation_skipped_validator_error` / degraded result class

### The most important drop

The pipeline does a lot of good work to distinguish:

* identity truth
* price usability truth
* quote realism truth

Then the later commercial gate logic mostly behaves as if realism truth is enough.
It isn’t.

A price can be realistic and still be:

* the wrong product
* the wrong pack basis
* the wrong commercial boundary

---

## 5) Fan-out coherence model

This is the clean model.

## First rule

**`stage_run_id` should mean atomic commit cohort for one scoped business unit.**

Not:

* whole session
* whole process
* whole stage bucket

Introduce a separate id for the agent run/session:

* `session_execution_id` = one job attempt / one agent session
* `stage_run_id` = one authoritative commit of one scoped unit

A single stage-9 eBay session for one notice may produce:

* 1 `session_execution_id`
* 8 committed `stage_run_id`s
  (one per candidate-source pair)

That is fine.

## Second rule

Split stages into **set-replacement** vs **additive**.

### Set-replacement stages

Partial commit is unsafe.

* `normalize` — per notice version
* `relate` — per process/cluster relation resolution
* `acquire_docs` — per notice document set
* `extract_lots` — per notice lot set
* `manifest` — per process manifest basis

### Additive / independently retryable stages

Partial commit is safe at the correct scope.

* `extract_products` — per lot
* `score_quoteability` — per candidate
* `price_match` — per candidate-source pair
* `landed_cost` — per candidate-quote scenario

### Selection / gate stages

Commit the selection decision as its own scoped assessment.

* `gross_gate` — per candidate
* `net_gate` — per candidate
* process rollup derived after that

## Concrete answers to your scenarios

### If lot extraction succeeds for lots 1–7 and fails on 8

**Authoritative commit: no.**

Reason:

* lot extraction is a set-replacement stage
* a partial lot set is semantically unsafe

What to keep:

* checkpoint/debug artifacts
* maybe provisional scratch rows
* no head advance

### If product extraction succeeds for lots 1–2 and fails on lot 3

**Authoritative commit: yes, per lot.**

Reason:

* lots are independent at this stage
* product extraction is additive per lot

### If eBay search succeeds for candidates 1–5 and fails on 6

**Authoritative commit: yes, per candidate-source pair.**

Reason:

* the durable unit is candidate-source pair
* completed pairs should survive
* remaining pairs stay pending/failed

## What to change in the stage tables

You need scope-aware heads.

At minimum:

* `scopeKind = process | notice | lot | candidate | candidate_source | candidate_quote`
* `scopeId`
* `completenessState = complete | partial | degraded | blocked`
* `freshnessState = fresh | soft_stale | hard_invalid | superseded`
* `basisHash`
* `inputStageVectorJson`

Then process-level stage/head rows should be **rollups**, not the only truth.

## When does `stage_run_id` become meaningless?

The moment it spans outputs that can be independently retried or independently become stale.

In this pipeline:

* it starts getting shaky at **Stage 6**
* it is fully broken by **Stage 9** if kept process-level

---

## 6) Things you didn’t ask that still matter

### A. Late merge after downstream enrichment

This will bite you.

If two processes are enriched separately, then later merged, you need a stable cluster identity. `process_aliases` is not enough.

**Recommendation:** add immutable `processClusterId`.

---

### B. Candidate value coverage is a first-class control variable

Without candidate value share, you cannot implement:

* “top-value” quoteability search
* meaningful process-level coverage
* honest manifests

**Recommendation:** persist `candidateValueShareBps` or `estimatedCandidateValuePence`.
If unknown, auto-qualification must stay conservative.

---

### C. Decision staleness must be explicit

Operator decisions should not auto-revoke, but they absolutely can become stale.

**Recommendation:** when operator approves/rejects/watches, store:

* `reviewedBasisHash`
* `reviewedManifestHash`
* `reviewedAtStageRunId`

If upstream hard-invalidates later:

* keep the decision
* mark `decision_stale = true`
* block bid workflow until re-affirmed

---

### D. Fail-open validator errors should still propagate review pressure

The doc says validator failures are fail-open. Operationally that’s fine.

But if `validation_skipped_validator_error` is invisible downstream, suspect outputs can still auto-qualify.

**Recommendation:** fail-open for pipeline progress, but propagate:

* `validationState = skipped_validator_error`
* set `reviewRequired = true` on affected scope/process rollup

---

### E. Bid workflow needs explicit scope

A process can have:

* qualifying lot 1
* review lot 2
* rejected lot 3

A process-level bid state is too blunt.

**Recommendation:** add `bidScopeJson` (lot ids / candidate ids) to bid workflow state or job payload.

---

### F. Contract tests at the seams

This architecture needs stage-boundary tests, not just unit tests inside stages.

The must-have seam tests are:

1. **Synthetic whole-notice lot later splits into real lots**
2. **Documents arrive after notice-only product extraction**
3. **Candidate-source session dies after some pairs commit**
4. **Pack basis unresolved quote tries to pass gross gate**
5. **Quoteability formula changes without band crossing**
6. **Late process merge after pricing**
7. **Operator decision becomes stale after upstream invalidation**

---

## 7) Changes I would make to the document

No stage-16 nonsense. Just tighten the existing stages.

### Diff 1 — make stage tracking scope-aware and align it to the actual 15-stage pipeline

**§5.4 / §5.6 / §5.7**

```diff
-export interface ProcessStageStatus {
-  processId: string;
-  stage: "ingestion" | "qualification" | "enrichment" | "landed_cost" | "manifest" | "bidding";
-  stageRunId: string;
-  state: "running" | "succeeded" | "failed";
-  startedAt: string;
-  completedAt?: string;
-  errorManifestId?: string;
-}
+export interface StageScopeStatus {
+  processId: string;
+  stage:
+    | "ingest"
+    | "normalize"
+    | "relate"
+    | "qualify"
+    | "acquire_docs"
+    | "extract_lots"
+    | "extract_products"
+    | "score_quoteability"
+    | "price_match"
+    | "gross_gate"
+    | "landed_cost"
+    | "net_gate"
+    | "manifest"
+    | "operator_decision"
+    | "bid_workflow";
+  scopeKind: "process" | "notice" | "lot" | "candidate" | "candidate_source" | "candidate_quote";
+  scopeId: string;
+  stageRunId: string;
+  sessionExecutionId?: string;
+  state: "running" | "succeeded" | "failed";
+  resultClass: "complete" | "partial" | "degraded" | "blocked" | "skipped";
+  freshnessState: "fresh" | "soft_stale" | "hard_invalid" | "superseded";
+  inputStageVectorJson: Record<string, string>;
+  basisHash: string;
+  startedAt: string;
+  completedAt?: string;
+  errorManifestId?: string;
+}
```

Also align `WorkflowJob.kind` to the real rerunnable boundaries, not the coarse 6-kind version.

---

### Diff 2 — add the missing stage-5 output and repair the identity chain

**§5.4 / §5.7**

```diff
+export interface TenderDocumentSet {
+  id: string;
+  processId: string;
+  noticeId: string;
+  documentSetHash: string;
+  completenessState: "none" | "partial" | "full";
+  documentRoles: Record<string, "spec" | "pricing_schedule" | "boq" | "appendix" | "other">;
+  acquiredAt: string;
+}

 export interface QualificationResult {
   processId: string;
   noticeId: string;
+  lotId: string;
+  qualificationBasis: "notice_only" | "notice_plus_docs" | "lot_level";
+  addressableValuePence?: number;
   expiryQualified: boolean;
   goodsQualified: boolean;
   goodsConfidence: number;
   ...
 }

 export interface ProductCandidate {
   id: string;
   processId: string;
   noticeId: string;
-  lotId?: string;
+  lotId: string;
+  lineageKey: string;
+  parentCandidateId?: string;
+  candidateRole: "standalone" | "bundle_parent" | "bundle_component";
+  candidateValueShareBps?: number;
+  sourceEvidence: Array<{
+    documentSetId: string;
+    documentId: string;
+    pageRef?: string;
+    textSpanHash: string;
+  }>;
   ...
 }
```

This fixes:

* lot-scoped qualification
* document basis traceability
* bundle lineage
* top-value candidate selection
* the optional-FK hole in the identity chain

---

### Diff 3 — stop using `PipelineAssessment` as if one process-state can tell the whole truth

**§5.5 / §5.4**

```diff
 export interface PipelineAssessment {
   processId: string;
   pipelineState:
     | "discovered"
     | "normalized"
     | "relation_resolved"
     | "prelim_rejected"
     | "prelim_qualified"
     | "docs_acquired"
     | "lots_extracted"
     | "products_extracted"
     | "quoteability_scored"
-    | "needs_review"
     | "priced"
     | "gross_qualified"
     | "landed_costed"
-    | "net_qualified"
-    | "expired";
-  grossMarginBps?: number;
-  netMarginBps?: number;
-  reviewStage?: "qualification" | "quoteability" | "pricing" | "landed_cost" | "manifest";
-  reviewReason?: string;
+    | "net_qualified";
+  availabilityStatus: "active" | "expired" | "cancelled";
+  reviewRequired: boolean;
+  reviewStage?: "qualification" | "quoteability" | "pricing" | "landed_cost" | "manifest";
+  reviewReason?: string;
+  pricedCoverageBps?: number;
+  landedCostCoverageBps?: number;
+  netQualifiedCoverageBps?: number;
+  unresolvedMandatoryLots?: number;
+  decisionBasisHash?: string;
   assessedAt: string;
 }
```

Process rollup should be **derived from scoped stage heads + coverage**, not manually advanced as if the process were the only real unit.

---

### Diff 4 — make stage 10 and 12 consume the right pricing truths

**Appendix A / Appendix B**

```diff
-gross_estimate_confidence = min(revenue_confidence, quote_realism_confidence)
+pricing_input_confidence = min(
+  match_confidence,
+  quote_usability_confidence,
+  quote_realism_confidence
+)
+gross_estimate_confidence = min(revenue_confidence, pricing_input_confidence)

-if gross_margin >= 0.30 and gross_estimate_confidence >= confidence_floor:
+if match_confidence < archetype_identity_floor:
+  route_to_manual_review
+else if quote_usability_confidence < 0.45:
+  route_to_manual_review
+else if gross_margin >= 0.30 and gross_estimate_confidence >= confidence_floor:
   qualify_for_landed_cost_stage
```

And for net gate:

```diff
-if net_margin_low >= 0.20 and no critical_blockers:
+if net_margin_low >= 0.20
+   and no critical_blockers
+   and decision_grade == "screening"
+   and net_qualified_coverage_bps >= process_coverage_floor_bps:
   final_qualified
```

That closes the biggest silent-corruption path in the commercial chain.

---

### Diff 5 — pin operator decisions to what was actually reviewed

**§5.4 / §5.5 / §5.9**

```diff
 export interface OperatorDecisionHead {
   processId: string;
   disposition: "pending" | "watching" | "approved" | "rejected";
   decisionNote?: string;
   decidedBy?: string;
   decidedAt?: string;
+  reviewedBasisHash?: string;
+  reviewedManifestHash?: string;
+  reviewedAtStageRunId?: string;
   remoteRevision: number;
 }
```

And bid workflow jobs should carry:

```diff
+  bidScopeJson?: {
+    lotIds?: string[];
+    candidateIds?: string[];
+  };
```

---

## 8) The three fixes I would do first

If you only do three things before implementation, do these:

### 1. Make execution and freshness **scope-aware**

That means:

* candidate-source heads
* lot heads
* candidate heads
* basis vectors
* hard-invalid vs soft-stale

Without this, retries and reruns will lie.

### 2. Repair the lineage chain

That means:

* first-class document set
* lot-scoped qualification
* required `lotId`
* bundle lineage
* quote row-version pin
* basis hash on decisions/manifests

Without this, you cannot explain why an estimate exists.

### 3. Add coverage-aware rollups

That means:

* candidate value share
* priced coverage
* landed-cost coverage
* net-qualified coverage
* mandatory-lot unresolved count

Without this, `net_qualified` will overstate reality.

---

The harsh version:

The current design is good at describing **what each stage does**.
It is not yet good at describing **what the pipeline currently knows, how complete that knowledge is, and whether it is still valid**.

That is the seam problem.
Fix that, and the rest of the architecture gets a lot more honest.


For each change proposed by the reviewer, evaluate:

### Integration Rules

1. **For fan-out coherence improvements (stage_run_id semantics at cardinality explosion):** Integrate into §5.3 (pipeline flow description) and §5.5 (state model). The current design says all outputs from a single stage execution share a `stage_run_id` and the stage head is advanced atomically. If the reviewer proposes sub-stage-run tracking, per-entity-type `stage_run_id` semantics, or partial-commit policies for fan-out stages (lot extraction producing N lots, product extraction producing M candidates, price-signal matching producing Q quotes), integrate while preserving the atomic transaction invariant: stage head advancement + all output rows + outbox entries must land in the same SQLite transaction. If partial-commit semantics are proposed for fan-out stages, they must define a clear partial-failure policy: are 7/12 successfully extracted lots committed and the stage marked as partial, or does the whole stage fail? Whatever the policy, it must be explicit in §5.3 and §5.5, not left to implementation. Consider these specific fan-out cases:
   - **Lot extraction** (stage 6): skill extracts lots from documents. Partial success = some lots parsed, others failed. Current AGENT_PARTIAL_FAILURE says "never upgraded to success."
   - **Product extraction** (stage 7): runs per qualified lot. Is it one skill session per lot or one for all lots? If one session, failure on lot 3 loses lots 1-2. If separate sessions, what is the stage_run_id scope?
   - **Price-signal matching** (stage 9): one session per source per notice, multiple candidates batched. Checkpoint per candidate-source pair, but stage_run_id is per-stage.

2. **For retroactive invalidation / staleness propagation proposals:** Integrate into §5.5 (state model) and a new subsection of §5.3. The current design has no concept of "upstream staleness" — each process gets one forward pass through the pipeline. If the reviewer proposes an invalidation watermark, a stage dependency graph with version tracking, or re-run trigger conditions, integrate with concrete semantics. Consider these specific staleness vectors:
   - **FX rate drift**: landed-cost estimates computed with yesterday's GBP/CNY rate. No mechanism to flag stale estimates.
   - **Quoteability formula changes**: updated scoring weights change which candidates pass the 0.45/0.65 thresholds.
   - **Lexicon updates**: new noun aliases make existing SearchProfile query waves stale.
   - **Document acquisition success on retry**: documents now available that weren't before. Existing ProductCandidates were extracted from vague summaries.
   - **Marketplace price changes**: SupplierQuote rows in SQLite have no TTL or staleness marker.
   Key question: which upstream changes should cascade re-runs and which should not? The proposal must work with the existing `process_stage_status` / `process_stage_heads` tables — do not introduce a separate invalidation tracking system without coordinating with the sync-engine domain via `<!-- CROSS-DOMAIN -->` comments.

3. **For cross-stage error propagation analysis (silent corruption paths):** Integrate into §5.7 (intelligence pipeline flow) as inline annotations at each stage boundary. The current design defines error types per stage (§5.11) and confidence scores on entities, but does not explicitly map which confidence scores are consumed by downstream gates and which are stored but never checked. The reviewer was asked to identify three types of failure per stage: what fails loudly (triggers error manifest), what fails silently (produces valid-schema garbage), and what fails ambiguously (low confidence that nobody checks). Specific paths to check:
   - **"Garbage candidates" from document-less extraction**: valid ProductCandidate shape, low identityConfidence, passes quoteability scoring with low-but-nonzero score, might even get marketplace results. How does low identityConfidence actually gate downstream behavior?
   - **Pack-basis normalization errors**: a single extraction error at stage 9 (listing "box of 100 gloves" read as "1 glove") corrupts stages 10-12 (100x wrong margin). The unitPriceGbp is 100x wrong. This flows into gross margin, landed cost, net margin. But quoteUsabilityConfidence doesn't track "unit basis parsing quality" as a separate dimension.
   - **HS code misclassification compounding**: wrong HS code cascades across duty rates, freight mode, trade remedy risk, and import controls. classificationStability only tells you whether candidates converge, not whether they converge on the RIGHT code.
   If the reviewer identifies concrete silent failure paths, add explicit confidence gate checks or routing-to-review at the consumption point in §5.7. Do NOT add new error types — those belong to §5.11 (not owned by this domain).

4. **For pipeline resumability improvements:** Integrate into §5.5 (state model) and §5.6/§5.7 (data flow descriptions). The current design tracks state per stage but does not specify resume-after-partial-failure semantics. Consider these specific scenarios:
   - **Resume after partial stage failure**: process at `products_extracted`, price-signal matching fails mid-run. Candidates 1-5 got quotes but 6-8 didn't. Does retry re-search all 8 or just 6-8?
   - **Re-running an upstream stage**: document acquisition re-run (new documents found) but pipeline state is already at `quoteability_scored`. Does the daemon know to re-run stages 6-8? No mechanism for "invalidate all stages after stage X."
   - **stage_run_id on retry**: does a retry get a new stage_run_id or reuse the failed one? If new, the old partial outputs have a different stage_run_id than the new complete outputs. If reused, the partial outputs are mixed with new outputs.
   Resume semantics must answer: does retry re-run the entire stage or pick up from failure point? How are already-committed outputs from the failed run handled — preserved or replaced?

5. **For below-quoteability path improvements:** Integrate into §5.5 (state model) and §5.7 (quoteability scoring data flow). The current design says candidates with `quoteabilityScore < 0.45` don't enter marketplace search and the operator sees them at `quoteability_scored` with `disposition: pending`. The handoff identified these specific gaps:
   - **Operator has no pricing data**: without marketplace search, no SupplierQuote rows, no margin estimates, no landed-cost numbers. Operator sees a product candidate description and must decide based on personal market knowledge.
   - **RFQ escalation goes where?**: the doc says RFQ escalation is "an operator disposition action" but doesn't describe what happens mechanically. No RFQ template generation, no routing.
   - **Process-level vs candidate-level state**: PipelineAssessment.pipelineState is per-process, not per-candidate. If a process has 5 candidates where 3 pass quoteability and 2 don't, the process advances to `priced` on the passing candidates. Below-quoteability candidates are invisible in process-level state.
   - **Net-qualified with orphaned candidates**: process reaches `net_qualified` on strength of best candidates, but below-quoteability candidates representing 60% of contract value were never evaluated.
   If the reviewer proposes enriching this path, fold improvements into the existing flow. Do NOT create a parallel pipeline path.

6. **For pipeline idempotency proposals:** Integrate into §5.6 (ingestion flow) and §5.7 (intelligence pipeline flow). The current design uses cursor/watermark tracking for ingestion idempotency, but downstream stages lack explicit idempotency guarantees. Key question: is idempotency per `stage_run_id` (same run produces same outputs) or per `process_id × stage` (re-running the same stage for the same process produces compatible outputs)?

7. **For data lineage tracking proposals (beyond FK chains):** Integrate into §5.3 as cross-domain notes. FK chains already provide structural lineage (SupplierQuote → ProductCandidate → NoticeLot → ContractNotice → ProcurementProcess). If the reviewer proposes richer provenance (e.g., "this LandedCostEstimate traces back through SupplierQuote → ProductCandidate → NoticeLot → ContractNotice → ProcurementProcess and the full FK chain is intact"), verify the chain is gap-free. If they identify FK gaps or denormalized copies that break the trace, integrate fixes into the relevant data flow section and note schema changes via `<!-- CROSS-DOMAIN: ... -->`.

8. **For stage contract testing proposals:** Integrate into §5.3 as a new "Stage Boundary Contracts" subsection. If the reviewer proposes formalizing the input/output contract at each stage boundary (e.g., "product extraction requires lots with at least title OR description populated and outputs candidates with at least headNoun and identityConfidence"), this belongs in §5.3 alongside the pipeline flow. These contracts should be concrete enough to be testable: required fields, minimum population rates, and what happens when the contract is violated (fail stage vs degrade vs skip entity).

9. **For Appendix A margin logic improvements:** Integrate directly. The reviewer was specifically asked about the revenue normalization sequencing: revenue normalization is described in §5.7 landed cost steps 1-3, but the gross margin gate needs it too. If the reviewer proposes moving revenue normalization earlier in the pipeline (before the gross margin gate) or clarifying how the gross margin gate accesses it, integrate that fix. Do not change the gate thresholds (30% gross, 20% net) without noting that these are calibratable parameters. If the reviewer proposes more nuanced scenarios (volume-discount curves, lead-time-adjusted margins), integrate into the existing Appendix A structure.

10. **For Appendix C job kind improvements:** Integrate directly. Currently Appendix C lists 13 job kinds for 15 pipeline stages. If the reviewer proposes splitting existing job kinds, adding missing ones (e.g., separate `gross_margin_gate` from `estimate_landed_cost`, separate `net_margin_gate`), or specifying typed input/output contracts per job kind, integrate into Appendix C. Each job kind should specify: trigger condition, input entities, output entities, success criteria, failure handling, and which pipeline state transition it drives.

11. **For concurrent stage execution guards:** Integrate into §5.5 (state model). If the reviewer identifies race conditions where two jobs for the same process could execute out of order (e.g., a qualification job and a document acquisition job both in the queue), integrate ordering guards into the state model. The guard mechanism should use `process_stage_heads` — a stage can only execute if the prerequisite stage head exists and is current. Do NOT propose database-level locking beyond SQLite's single-writer model.

12. **For expiry check race condition resolution:** Integrate into §5.5. The current design says expiry is set by a background daemon job every 6 hours, and `PipelineAssessment.pipelineState` is a single enum that can't be both `priced` and `expired`. If the reviewer proposes a resolution (expiry as a separate boolean flag, or expiry interrupting the pipeline by transitioning to `expired` regardless of current state), integrate it. The proposal must define what happens to in-flight jobs when a process expires.

13. **For pipeline cost management improvements:** Integrate into §5.7 (Pipeline Cost Management subsection). If the reviewer proposes better budget allocation per stage, cost tracking per stage for attribution, or cost-aware stage skipping, apply while preserving the $20-60/day target and the existing operating limits (2 browser slots, 4 agent slots, max 25 candidates per notice, max 8 auto-searched per notice).

14. **For changes touching SQL schema, daemon internals, component designs, skill types, or milestone structure:** Do NOT integrate directly. Add `<!-- CROSS-DOMAIN: Reviewer suggested [X] for [component]. Needs coordination with [domain]. -->` notes at the relevant location in the owned sections.

### Evaluation Framework

For each proposed change, assess:
- **Agree strongly:** Fills a concrete gap in pipeline integration — identifies a real stage-boundary bug, resolves an ambiguity in partial-failure handling, maps a silent corruption path through to its downstream consequence, or provides a missing contract specification. Apply directly.
- **Agree partially:** Good analysis but the proposed solution introduces complexity beyond what's needed for MVP (e.g., full DAG-based invalidation when a simple "re-run from stage X" mechanism suffices, or per-entity staleness tracking when per-stage staleness is adequate). Apply a simplified version, noting the full version as a future enhancement.
- **Disagree:** Conflicts with settled decisions (daemon-orchestrated job queues per ADR-005, local SQLite as operational authority per ADR-002, atomic stage head advancement, single-writer model, no external workflow engine per ADR-005, no generic event bus). Note disagreement with rationale.

### Preservation Rules
- Preserve the 15-stage pipeline sequence — do not add stages, though you may add sub-steps within stages
- Preserve atomic stage head advancement (all outputs + stage status + outbox entries in one SQLite transaction)
- Preserve the `process_stage_status` / `process_stage_heads` tracking model
- Preserve the coarse-grained `ProcessStageStatus.stage` vs fine-grained `PipelineAssessment.pipelineState` distinction
- Preserve the authority matrix: pipeline state is daemon-owned/push-only, operator disposition is remote-owned/pull-only
- Preserve the single-writer SQLite model (no concurrent writes)
- Preserve the job-queue-driven pipeline execution model (workflow_jobs table, Job Runner claims and executes)
- Preserve the gross margin gate → landed cost → net margin gate sequence (two-gate margin analysis)
- Preserve the margin gate thresholds as calibratable parameters (30% gross, 20% net are defaults, not axioms)
- Preserve the below-quoteability path: candidates < 0.45 don't enter marketplace search, operator triages them
- Preserve the worker-validator integration: steps 12a-12d in §5.6 are owned by the worker-validator domain
- Preserve landed-cost subsection internals in §5.7: the customs classification pipeline (steps 7-11 in landed cost flow) is owned by the customs-classification domain
- Preserve pipeline cost target ($20-60/day) and existing operating limits
- Preserve the distinction between ingestion flow (§5.6) and intelligence pipeline (§5.7)
- Preserve skill-based execution for data retrieval stages
- Use "as-if-always" language

---

## Step 3: Fresh-Eyes Optimization

After completing Step 2, re-read the sections you edited with fresh eyes. Check for:

1. **§5.3 pipeline stages ↔ §5.5 state machine consistency:** Does every stage in the §5.3 pipeline flow map to at least one `PipelineAssessment.pipelineState` value in §5.5? Are there pipeline states that no stage produces? Are there stages that don't advance the pipeline state? If the reviewer added sub-steps or modified stage boundaries, verify the state machine still covers the full pipeline. Specifically check: does `quoteability_scored` correctly serve as BOTH the terminal state for below-quoteability candidates AND the gateway state for marketplace search candidates?

2. **§5.5 state transitions ↔ §5.6/§5.7 data flow consistency:** Does each state transition described in §5.5 correspond to a concrete step in either §5.6 (ingestion) or §5.7 (intelligence pipeline)? Walk the full path: `discovered` → `normalized` should map to §5.6 step 13; `normalized` → `relation_resolved` should map to §5.6 step 14; `relation_resolved` → `prelim_qualified` or `prelim_rejected` should map to §5.7 qualification flow; `prelim_qualified` → `docs_acquired` should map to §5.7 document acquisition; `docs_acquired` → `lots_extracted` → `products_extracted` → `quoteability_scored` should map to §5.7 extraction flows; `quoteability_scored` → `priced` should map to §5.7 price-signal matching; `priced` → `gross_qualified` → `landed_costed` → `net_qualified` should map to §5.7 landed cost + Appendix A gates. Any state transition without a corresponding data flow step is a documentation gap.

3. **§5.7 stage boundary field contracts ↔ §5.4 entity fields consistency:** At each stage boundary in §5.7, check that the fields consumed by the downstream stage actually exist on the entity produced by the upstream stage. Specific checks:
   - Does product extraction (§5.7) populate `identityConfidence`, `specConfidence`, `quantityConfidence` that quoteability scoring requires?
   - Does quoteability scoring populate `quoteabilityScore` and `recallRiskScore` that search profile generation requires?
   - Does price-signal matching populate `matchConfidence`, `quoteUsabilityConfidence`, `priceSignalRole` that the gross margin gate requires?
   - **Revenue normalization sequencing**: Does the gross margin gate (stage 10) have access to `expected_addressable_revenue_ex_vat`? Revenue normalization is currently described in §5.7 landed cost steps 1-3, which is stage 11. But the gross margin gate fires at stage 10 and needs revenue data. Is there a separate revenue normalization that happens before the gross margin gate? If the reviewer fixed this, verify the fix is consistent across §5.3, §5.7, and Appendix A.
   - Does `LandedCostEstimate` carry `netMarginLowPct` / `netMarginBasePct` / `netMarginHighPct` that the net margin gate requires?
   - Does `PipelineAssessment` get updated with `grossMarginBps` from the gross margin gate and `netMarginBps` from the net margin gate?

4. **Appendix C job kinds ↔ §5.3 pipeline stages consistency:** Does every pipeline stage have a corresponding job kind in Appendix C? Currently Appendix C lists 13 job kinds for 15 pipeline stages. Check: is `gross_margin_gate` a separate job kind or folded into `estimate_landed_cost`? Is `net_margin_gate` a separate job kind or folded into landed cost? Is manifest generation listed? Are there pipeline stages that share a job kind, and if so, is that intentional and documented?

5. **Appendix A margin formulas ↔ §5.7 gate logic consistency:** Do the margin formulas in Appendix A reference the same field names and computation steps as §5.7's landed cost data flow? Specifically:
   - Does the gross margin gate formula use `normalized_supplier_cost` (Incoterm-normalized per Appendix A) and `expected_addressable_revenue_ex_vat`?
   - Does the net margin gate formula reference `freight_high/base/low` and `customs_duty_high/base/low`? Are these the same as `freightHighGbp/freightBaseGbp/freightLowGbp` and `customsDutyGbp + tradeRemedyDutyGbp` on LandedCostEstimate?
   - Is `gross_estimate_confidence = min(revenue_confidence, quote_realism_confidence)` computed anywhere in §5.7, or is it only in Appendix A? If only in Appendix A, add the computation step to §5.7.

6. **Fan-out coherence invariants consistency:** If the reviewer proposed partial-commit semantics for fan-out stages, verify they're applied consistently across all fan-out stages: lot extraction (1 notice → N lots), product extraction (1 lot → M candidates), price-signal matching (1 candidate → Q quotes across sources), and landed cost (1 quote → R estimates). The partial-commit policy should be the same across all fan-out stages unless there's a documented reason for different policies.

7. **Expiry mechanism ↔ in-flight job handling consistency:** If the reviewer proposed changes to the expiry mechanism (§5.5), verify the proposal handles all in-flight scenarios: process mid-qualification, process mid-enrichment, process with active supplier search sessions, process with pending validation sub-job. The expiry behavior should be explicit for each case.

8. **Staleness propagation ↔ process_stage_heads interaction:** If the reviewer added a staleness or invalidation mechanism, verify it interacts correctly with process_stage_heads. Specifically: if stage 5 outputs are invalidated, does the stage head for stage 5 get rolled back? If so, what happens to stage heads for stages 6-8 that were advanced based on the now-invalidated stage 5 outputs? If not, how does the pipeline know to re-run stages 6-8?

9. **Terminology:** Ensure consistent use of:
   - "pipeline state" (not "workflow state" or "process state") for `PipelineAssessment.pipelineState`
   - "stage" (not "step" or "phase") for the 15-pipeline-stage sequence
   - "step" for numbered sub-operations within a stage (e.g., "landed cost step 7")
   - "stage_run_id" (not "execution_id" or "run_id") for stage execution tracking
   - "stage head" (not "stage pointer" or "stage cursor") for `process_stage_heads`
   - "fan-out" (not "explosion" or "multiplication") for cardinality increase at stage boundaries
   - "gate" (not "check" or "filter") for margin gates specifically (gross margin gate, net margin gate)
   - "qualification" (not "filtering" or "screening") for the preliminary qualification stage
   - "below-quoteability" (not "unquoteable" or "non-quoteable") for candidates below the 0.45 threshold

Fix any inconsistencies found. Make targeted edits, not wholesale rewrites.

---

## Step 4: Sync Pass

1. **Update §1 (Executive Summary):** If the pipeline integration changes materially affected the stage sequence, state model, or margin gate logic, update the executive summary pipeline description. The "15-stage pipeline" summary and "What Will Make or Break the Product" section should reflect any structural changes. If fan-out coherence or staleness propagation mechanisms were added, mention them in the "confidence scores, raw evidence" preservation paragraph.

2. **Update Appendix E (Changelog):** Add a concise entry describing what was integrated in this review cycle. Include: which sections were modified, what the reviewer's key contributions were (stage boundary bugs found, partial-failure semantics clarified, staleness propagation mechanism, silent corruption paths mapped, revenue normalization sequencing fixed, etc.), and any deferred cross-domain items.

3. **Verify cross-references between pipeline sections:** Every mention of pipeline stages in §5.3 should be traceable to a data flow step in §5.6 or §5.7. Every state transition in §5.5 should reference the stage that causes it. Every job kind in Appendix C should reference the pipeline stage it serves. Every margin formula in Appendix A should be computable from the fields defined in §5.4 entities.

4. **Check §10 (Open Questions):** If any open questions about pipeline flow, stage ordering, partial failure handling, or margin computation have been resolved by this review cycle, mark them as resolved. If new questions emerged (e.g., "should expiry be a separate boolean or a pipeline state?", "how should re-running an upstream stage invalidate downstream stages?"), add them.

5. **Verify §8.2 file references:** If the reviewer proposed new pipeline-related source files (stage contract validators, pipeline cost attribution modules, staleness tracker), ensure they appear in §8.2 with correct milestone assignments. Core pipeline integration fixes should be in Milestone 1-2, not deferred.

6. **Check §6.1 and §6.2 for new risks:** If the reviewer identified new risk areas (fan-out coherence failures, retroactive invalidation gaps, silent corruption paths not previously catalogued, revenue normalization sequencing bug), add them to §6.2 with likelihood/impact ratings. If the reviewer changed complexity assessments for pipeline-related components, update §6.1.

7. **Verify §11 milestone assignments:** If the reviewer's changes affect which pipeline capabilities are needed at each milestone, verify the milestone plan in §11 is consistent. Specifically: does MVE (Milestone 0) require any of the newly added pipeline integration mechanisms (e.g., stage boundary contracts), or can they be deferred to Milestone 1+? Fan-out coherence and basic resumability should be in Milestone 1 (thin local runner). Staleness propagation can likely be deferred to Milestone 3+.

8. **Update §4.2 cross-references:** If pipeline stages in §5.3 diverged from the conceptual pipeline described in §4.2, add a cross-reference note. Do not modify §4.2 directly — it's the conceptual design and should remain stable.
