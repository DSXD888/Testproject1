# Handoff: End-to-End Pipeline Integration & Data Flow Integrity

## Expert Framing

You are a **data pipeline integration architect** who has built and debugged multi-stage data processing systems with 10+ stages, complex entity cardinality fan-outs, and cross-stage data dependency chains. You've worked on ETL/ELT pipelines at scale, event-driven enrichment systems, and multi-stage data quality platforms where upstream errors compound exponentially downstream. You draw from experience with systems like Airflow/Dagster DAGs with 50+ tasks, real-time enrichment pipelines in adtech or fintech, supply-chain traceability systems, and any architecture where a single record enters at the top and becomes dozens of derived entities by the bottom.

You understand that the hardest bugs in multi-stage pipelines live at stage boundaries, not within stages. A stage that works perfectly in isolation can silently corrupt the next stage by violating an implicit contract. You know that "the data model looks right" and "the pipeline actually preserves data integrity end-to-end" are two completely different statements. You have painful experience with fan-out coherence problems (one input becomes N outputs — what happens when output 7 fails?), retroactive invalidation (upstream data changes after downstream stages have already consumed it), and the silent data corruption that occurs when confidence scores from stage A are consumed by stage B without understanding A's failure modes.

---

## Mission

ULTRATHINK and be meticulous.

This procurement intelligence platform has a 15-stage pipeline:

```
Ingest -> normalize -> relate -> qualify -> acquire docs -> extract lots ->
extract products -> score quoteability -> price-signal match -> gross margin gate ->
landed cost -> net margin gate -> manifest -> operator decision -> bid workflow
```

Each of these stages has been individually designed and reviewed by domain-specific experts across 2 iterations and 8 review domains (procurement pipeline, data architecture, agent orchestration, landed-cost engine, search intelligence, skill reliability, MVP delivery, frontend integration). The individual stages are well-designed. But **nobody has ever reviewed the complete pipeline as an integrated system.**

Your job: find the integration bugs. The places where stage A's output contract doesn't quite match stage B's input expectations. Where failure in stage C silently corrupts stage D's assumptions. Where the data model promises something the pipeline logic doesn't deliver. Where cardinality fan-outs break coherence abstractions. Where retroactive changes to upstream data leave downstream stages operating on stale assumptions.

This is not a review of any single stage. This is a review of the **seams** between stages, the **data flow integrity** across the full pipeline, and the **failure propagation paths** that cross stage boundaries.

**Don't just answer my questions -- the questions I DON'T KNOW to ask are more important than the ones I do.**

---

## User's Strong Beliefs (Tensions to Engage With)

1. **"The identity chain is the hardest problem."** The pipeline transforms entities through a chain: documents -> lots -> candidates -> matches -> quotes -> cost estimates. Each link changes the entity's shape and cardinality. A supplier quote exists because of a specific lot description in a specific tender document. But can you actually trace a `LandedCostEstimate` back through the `SupplierQuote` -> `ProductCandidate` -> `NoticeLot` -> `ContractNotice` -> `ProcurementProcess` chain and reconstruct *why* that cost estimate exists? Are there FK gaps or denormalized copies that break the trace?

2. **"Stage coherence via `process_stage_status` and `stage_run_id`."** The architecture says all outputs from a pipeline stage share a `stage_run_id`, and stage heads are advanced atomically. This works when a stage produces one output row. But some stages fan out: lot extraction produces N `NoticeLot` entities, product extraction produces M `ProductCandidate` entities per lot, price-signal matching produces Q `SupplierQuote` entities per candidate across multiple sources. When cardinality fans from 1 to potentially hundreds, does `stage_run_id` coherence still hold? What happens when extraction succeeds for lot 7 of 12 but fails on lot 8 -- do lots 1-7 get committed or does the whole stage fail?

3. **"The pipeline is deterministic after quoteability scoring."** Quoteability scoring is the gate that decides which candidates enter marketplace search. Everything downstream (search, matching, pricing, margin estimation) depends on the quoteability decision. But quoteability scores are computed from extracted product candidate data, which depends on document acquisition quality, which depends on portal availability on that particular day. What happens when quoteability scores change retroactively -- e.g., a lexicon update changes scoring weights, or a document becomes available that wasn't before? Do downstream stages know their inputs are stale?

---

## Essential Context

### The Platform

A UK public procurement intelligence platform that ingests official contract notices, qualifies them for physical-goods supply opportunities, enriches qualified opportunities with supplier pricing and landed-cost estimates, and surfaces high-margin opportunities for operator review and optional bid automation.

### The Full 15-Stage Pipeline

```text
Stage 1:  INGEST             ContractNotice created from source portal scraping
Stage 2:  NORMALIZE          ContractNotice normalized to internal OCDS-based schema
Stage 3:  RELATE             ProcurementProcess created/updated via relation resolution
                             (merge | related_not_merged | possible_duplicate | separate)
Stage 4:  QUALIFY            QualificationResult per lot: expiry, access route, goods class
Stage 5:  ACQUIRE DOCS       Tender documents downloaded (spec PDFs, BoQ, pricing schedules)
Stage 6:  EXTRACT LOTS       NoticeLot entities created (or synthetic whole-notice lot)
Stage 7:  EXTRACT PRODUCTS   ProductCandidate entities per lot, with structured slots
Stage 8:  SCORE QUOTEABILITY Deterministic quoteabilityScore + recallRiskScore per candidate
Stage 9:  PRICE-SIGNAL MATCH SupplierQuote entities via marketplace search + shortlisting
Stage 10: GROSS MARGIN GATE  Gross margin check: >= 30% + confidence floor to continue
Stage 11: LANDED COST        LandedCostEstimate via API-based HS classification, freight, FX, duty
Stage 12: NET MARGIN GATE    Net margin check: low >= 20% + no critical blockers to auto-qualify
Stage 13: MANIFEST           Opportunity manifest generation (narrative synthesis)
Stage 14: OPERATOR DECISION  Human review: reject / watch / RFQ escalation / approve
Stage 15: BID WORKFLOW       Portal-specific bid skill execution
```

### Database Architecture

**Authority-matrix split:**
- **Local SQLite** = operational authority (pipeline data, jobs, logs, artifacts). Single-writer (daemon only).
- **Supabase/Postgres** = shared authority (operator decisions, config) + projections (pipeline facts for viewing portal).
- **Push-only** pipeline replication via transactional outbox. **Pull-only** shared config/decisions via cursor-based sync.
- Mixed-write tables are prohibited.

**Stage tracking:**
- `process_stage_status`: per-process, per-stage execution records with `stage_run_id`, state (pending/running/succeeded/failed), timing, error manifest reference.
- `process_stage_heads`: per-process, per-stage pointer to the current `stage_run_id`. Advanced only on success, in the same transaction as stage outputs and outbox entries.

### Key Data Entities and Their Cardinality Chain

```text
ProcurementProcess (1)
  -> ContractNotice (1..N, usually 1-3)
     -> NoticeLot (1..M per notice, 1 synthetic if no explicit lots)
        -> ProductCandidate (0..P per lot, up to 25 per notice)
           -> SearchProfile (1 per candidate that passes quoteability)
              -> SearchAttempt (1..S per source x wave)
              -> SupplierQuote (0..Q per candidate, across sources)
                 -> LandedCostEstimate (0..R per quote or per candidate)
  -> QualificationResult (1 per process at qualification time)
  -> PipelineAssessment (1 per process, mutated as pipeline advances)
  -> OperatorDecisionHead (1 per process, remote-owned)
  -> BidWorkflowState (0..1 per process)
  -> ProcessRelation (0..many edges to related processes)
```

### PipelineAssessment States

```text
discovered -> normalized -> relation_resolved -> prelim_rejected
                                              -> prelim_qualified
  -> docs_acquired -> lots_extracted -> products_extracted -> quoteability_scored
  -> needs_review (reachable from multiple stages)
  -> priced -> gross_qualified -> landed_costed -> net_qualified
  -> expired (background daemon job, any active state)
```

### Validation Gates in the Pipeline

1. **Expiry filter** (stage 4): reject if deadline < 30 days (except DPS/dynamic market)
2. **Access route filter** (stage 4): reject direct_award, closed_framework_award, not_bidable
3. **Goods classification** (stage 4): reject pure services/works with negligible goods density
4. **Quoteability gate** (stage 8): >= 0.65 auto-search, 0.45-0.64 top-value only, < 0.45 skip
5. **Gross margin gate** (stage 10): >= 30% gross margin + confidence floor
6. **Net margin gate** (stage 12): >= 20% net margin (low scenario) + no critical blockers

### Worker-Validator Layer

Between the Output Quality Gate and DB commit (steps 12a-12d in the ingestion flow, also applied to intelligence pipeline skills):
1. Agent writes a **run manifest** (structured MD narrative of what happened).
2. **Validation Policy Engine** checks whether a validator sub-job is needed (always/sample/trigger/skip).
3. **Validator agent** reads run manifest + OQG signals + golden assertions, renders verdict: approved / rejected / flagged.
4. **Verdict processing:** approved = proceed; rejected = quarantine as `AGENT_SUSPECT_OUTPUT`; flagged = proceed with flag. Validator failures are fail-open (worker results advance).

---

## Tensions to Resolve

### Tension 1: Fan-Out Coherence — When `stage_run_id` Meets Cardinality Explosion

A single procurement process can produce 12 lots, 25 product candidates across those lots, and potentially 50+ supplier quotes across candidates and sources. All outputs from a single pipeline stage execution share a `stage_run_id`, and the stage head is advanced in the same SQLite transaction.

But consider:

- **Lot extraction** (stage 6): The skill extracts lots from documents. If it successfully extracts lots 1-7 but fails parsing lot 8, what happens? The skill outcome is `AGENT_PARTIAL_FAILURE` (checkpoint exists but no final contract). Do lots 1-7 get committed? The architecture says partial failures are "never upgraded to success" -- does that mean 7 good lots are thrown away because lot 8 failed?

- **Product extraction** (stage 7): Runs per qualified lot. But is it one skill session per lot, or one session for all lots in a notice? If one session, a failure on lot 3's candidates means lot 1-2's candidates are also lost. If separate sessions, the `stage_run_id` semantics are unclear -- does product extraction get one `stage_run_id` per lot or one per notice?

- **Price-signal matching** (stage 9): The architecture says "one supplier session per source per notice (multiple candidates batched together)." So a single session searches for 8 candidates on eBay. If candidates 1-5 get results but the session fails during candidate 6's search, what happens to candidates 1-5's quotes? The doc says "durable business unit is the candidate-source pair" but `stage_run_id` is per-stage, not per-pair.

- At what cardinality does `stage_run_id` become meaningless? If 50 rows across 3 entity types all share the same `stage_run_id`, what useful invariant does that provide beyond "they were produced in the same execution"?

### Tension 2: Retroactive Invalidation — The Stale Downstream Problem

The pipeline computes derived data at each stage based on upstream outputs. But upstream data can become stale:

- **FX rates** change daily. A landed-cost estimate computed with yesterday's GBP/CNY rate is stale today. The architecture stores `customsFxRate` and `commercialFxRate` on `LandedCostEstimate`, but there is no mechanism to flag existing estimates as stale when rates move significantly.

- **Quoteability scoring weights** may be tuned. If the scoring formula changes (e.g., penalty weights for missing standards are adjusted), existing `quoteabilityScore` values are stale. Candidates that were below 0.45 might now be above 0.65, and vice versa. But there is no concept of "quoteability version" tracked on the score.

- **Lexicon updates** for search profiles. If the bilingual lexicon adds new noun aliases, existing `SearchProfile` query waves are stale -- they were generated without the new aliases. Better searches might exist.

- **Document acquisition** succeeds on retry. Initially, no tender documents were available, so product extraction ran on "vague notice summaries" (the doc's own words) with low confidence. Later, documents become available. The existing product candidates are now based on inferior inputs, but there is no signal that says "these candidates were extracted without documents and should be re-extracted."

- **Marketplace price changes.** The two-tier cache has source-specific TTLs (24h-96h). But the `SupplierQuote` rows persisted in SQLite have no TTL or staleness marker. A quote from 2 weeks ago still looks current in the pipeline.

**The core question:** Is there a concept of "upstream staleness" that propagates downstream? If stage 5's output changes (documents now available), does the pipeline know to re-run stages 6-12? Or does each process get exactly one pass through the pipeline and whatever comes out is final?

### Tension 3: Cross-Stage Error Propagation — Silent Corruption Paths

The error handling strategy (section 5.11) defines error types per stage. But some errors don't crash the pipeline -- they silently degrade downstream quality:

- **"Garbage candidates" from document-less extraction.** The doc says "without tender documents, product extraction operates on vague summaries and produces garbage candidates" (section 6.1 Risk Area #1). But a "garbage candidate" still has a valid `ProductCandidate` schema shape. It passes quoteability scoring (probably with a low but non-zero score). It might even get marketplace results that look plausible. The `identityConfidence` should be low, but how does this low confidence actually gate downstream behavior? The quoteability formula uses it as an input, but `identityConfidence` of 0.3 doesn't mean "this candidate is probably wrong" -- it just means "we're not sure." The pipeline proceeds.

- **Pack-basis normalization cascading.** The doc identifies pack-basis errors as high-likelihood, high-impact (section 6.2). If a listing says "box of 100 gloves" but the extraction reads it as "1 glove," the `unitPriceGbp` is 100x wrong. This flows into gross margin (100x wrong), landed cost (100x wrong quantity for freight), and net margin (100x wrong). A single extraction error at stage 9 corrupts stages 10-12. But the confidence scores don't specifically track "unit basis parsing quality" as a separate dimension -- it is folded into `quoteUsabilityConfidence`.

- **HS code misclassification compounding.** A wrong HS code at stage 11 doesn't just give wrong duty rates. It can change the freight mode (different HS codes trigger different handling requirements), the trade remedy risk (some codes have anti-dumping duties), and even the import controls (some codes require licenses). A single classification error cascades across 4+ sub-calculations within the landed-cost stage. The `classificationStability` metric exists but only tells you whether candidate codes converge -- it doesn't tell you whether they converge on the RIGHT code.

- **What fails silently vs what fails loudly?** The error manifest system captures explicit failures. But "skill completed successfully with garbage data" is the failure mode that never triggers an error manifest. When does a low confidence score become an error?

### Tension 4: Pipeline Resumability — Partial Runs and Ordering Dependencies

The pipeline has implicit ordering dependencies: can't extract products before lots, can't price before quoteability, can't compute landed cost before pricing. The architecture describes per-stage checkpoints and `process_stage_status` tracking.

But consider:

- **Resume after partial failure.** A process reaches `products_extracted` (stage 7) but price-signal matching (stage 9) fails. When the daemon retries, does it re-run stage 9 from scratch, or does it pick up from where the failed session left off? If stage 9 was batching multiple candidates in one session, and candidates 1-5 got quotes but 6-8 didn't, does the retry re-search all 8 or just 6-8?

- **Re-running an upstream stage.** If document acquisition (stage 5) is re-run (new documents found), the pipeline state is already at `quoteability_scored` (stage 8). Does the daemon know to re-run stages 6-8? Or does it just update the documents and leave the stale lot/candidate data in place? The `process_stage_heads` table tracks the current `stage_run_id` per stage, but there's no mechanism for "invalidate all stages after stage X."

- **Expiry check race condition.** The background daemon job expires processes whose `latestDeadlineAt` has passed (every 6 hours). But a process might be mid-pipeline (say, at `priced`) when it expires. Does expiry interrupt the pipeline? Or can a process be `expired` and `priced` simultaneously? The `PipelineAssessment.pipelineState` is a single enum -- it can't be both `priced` and `expired`.

- **Concurrent stage execution.** Can two stages for the same process run concurrently? The pipeline is sequential, but the daemon uses job queues. If a qualification job and a document acquisition job for the same process are both in the queue, is there a guard that prevents out-of-order execution?

### Tension 5: Below-Quoteability Path — Dead End or Deferred Decision?

Candidates with `quoteabilityScore < 0.45` don't enter marketplace search. They advance to `quoteability_scored` in pipeline state, and the operator disposition defaults to `pending`. The doc says the operator can "set disposition to `watching`, `rejected`, or escalate to RFQ manually."

But:

- **The operator has no pricing data.** Without marketplace search, there are no `SupplierQuote` rows, no margin estimates, no landed-cost numbers. The operator sees a product candidate description and... must decide based on what? Their personal market knowledge? This seems like an information-dead-end.

- **RFQ escalation goes where?** The doc says RFQ escalation is "an operator disposition action" but doesn't describe what happens mechanically. Does the system generate an RFQ template? Does it route to a manual process? If the operator escalates to RFQ and an executable quote comes back, how does that quote re-enter the pipeline at stage 9?

- **Process-level vs candidate-level state.** `PipelineAssessment.pipelineState` is per-process, not per-candidate. If a process has 5 candidates where 3 pass quoteability and 2 don't, the process advances to `priced` based on the passing candidates. But the 2 below-quoteability candidates are invisible in the process-level state. Where does the operator see them? How does the operator know they exist and need attention?

- **The process can be `net_qualified` with orphaned candidates.** A process reaches `net_qualified` on the strength of its best candidates. But below-quoteability candidates from the same process were never evaluated. If those candidates represent 60% of the contract value (e.g., the "hard" items in a mixed lot), the operator is making a decision based on partial information.

---

## What I Don't Want (Anti-Patterns)

- **Don't propose a generic event-driven architecture or message bus.** The pipeline is daemon-orchestrated via job queues in local SQLite. That's decided (ADR-005). The Rust daemon manages all pipeline execution.
- **Don't suggest adding a workflow engine (Temporal, Prefect, Airflow).** The Rust daemon IS the workflow engine. Proposing an external orchestrator contradicts the local-first architecture.
- **Don't propose breaking the monolithic research document into separate specs.** That restructuring comes later. Work within the current doc structure.
- **Don't suggest adding more pipeline stages.** 15 is already a lot. If you identify a missing concern, propose folding it into an existing stage, not creating stage 16.
- **Don't propose generic "add more logging" advice.** The architecture already has three-layer observability (control events, protocol events, forensic bytes), run manifests, error manifests, and stage tracking. Be specific about what integration-level observability is missing.

---

## What I Do Want

1. **A concrete trace of one procurement process through all 15 stages.** Pick a realistic example (e.g., "Supply of PPE to NHS Trust, 3 lots, mixed goods/services"). Trace exactly which entities are created at each stage, which fields flow forward to the next stage, and where data is lost, transformed, or assumed. Show the cardinality at each stage. Identify every point where a FK reference or denormalized field is the only link between stages.

2. **Specific integration bugs at stage boundaries.** Not "there might be issues" -- concrete cases where stage A's output contract and stage B's input expectations are misaligned. For example: Does product extraction (stage 7) always populate the fields that quoteability scoring (stage 8) needs? Does the gross margin gate (stage 10) have access to the revenue normalization data it needs, or does it assume it from the notice value?

3. **A proposal for retroactive invalidation / staleness propagation.** How should the pipeline handle re-runs of upstream stages? Propose a concrete mechanism (e.g., "invalidation watermark," "stage dependency graph with version tracking," "re-run trigger conditions"). Be specific about which stage re-runs should cascade and which should not.

4. **Analysis of error propagation paths.** For each stage, identify: what fails loudly (triggers error manifest), what fails silently (produces valid-schema garbage), and what fails ambiguously (low confidence that nobody checks). Map the silent failure paths through the full pipeline. Identify which confidence scores are actually consumed by downstream gates and which are stored but never checked.

5. **Fan-out coherence model.** Propose how `stage_run_id` semantics should work when a stage produces N entities across M sub-executions. Address the partial-failure question: when 7 of 12 lots extract successfully, what should happen?

6. **Ideas I haven't asked about.** Pipeline idempotency? Data lineage tracking beyond FK chains? Stage contract testing (validating that stage B can actually consume stage A's output)? Pipeline cost attribution (which stage is consuming the most agent compute and why)? Monotonicity violations (can a process move backward in pipeline state)?

Provide concrete, implementable recommendations. Reference specific doc sections (sections 5.3-5.7, section 5.4 data models, section 5.5 state model, section 5.11 error handling, Appendix A margin logic). Frame changes to the current plan as modifications to specific sections with git-diff style output where helpful.

---

## Context Documents to Attach

When sending this handoff to a reviewer, attach:

1. **The full research document** (`docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`) -- the reviewer needs comprehensive access, but should focus especially on:
   - **Section 5.3** (Canonical Workflow) -- the 15-stage pipeline definition
   - **Section 5.4** (Data Model Design) -- all entity interfaces, FK relationships, `stage_run_id` columns, sync tracking columns
   - **Section 5.5** (Contract State Model) -- pipeline state machine, below-quoteability path, `needs_review` transitions
   - **Section 5.6** (Ingestion Data Flow) -- steps 1-17 including worker-validator steps 12a-12d, error cases, stage sync step 17
   - **Section 5.7** (Intelligence Pipeline Data Flow) -- qualification, document acquisition, lot/product extraction, quoteability, price-signal matching, zero-result recovery, landed cost steps 1-18, pipeline cost management, two-tier cache
   - **Section 5.11** (Error Handling Strategy) -- all error type tables, skill outcome classes, error manifest system
   - **Section 5.5 state transition notes** -- especially below-quoteability path, `needs_review` reachability, expiry mechanism
   - **Appendix A** (Margin Logic) -- revenue normalization, quote normalization, gross/net margin gates, critical blockers
   - **Appendix B** (Confidence Model) -- weighted composite, critical blocker override
   - **Section 6.1** (Complexity Analysis) -- risk areas, delivery-priority tiers
   - **Section 6.2** (Risk Assessment) -- all 20+ identified risks with likelihood/impact

Focus the reviewer's attention on: the seams between stages in sections 5.6 and 5.7, the entity cardinality chain in section 5.4, the state machine in section 5.5, and the gap between the error types defined in section 5.11 and the silent failure modes that produce valid-schema garbage.
