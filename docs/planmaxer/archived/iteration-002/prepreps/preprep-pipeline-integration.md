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

<INSERT REVIEWER RESPONSE HERE>

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
