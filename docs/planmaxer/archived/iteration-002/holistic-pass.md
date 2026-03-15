# Holistic Pass: Cross-Plan Unification (Iteration 002)

This prompt runs AFTER all 4 domain pre-preps have been integrated. It performs a full-context pass across the entire research document to fix cross-cutting inconsistencies and improve architectural coherence.

---

## Step 1: Deep Read

Read the following documents in this exact order. Do not skip any.

1. **AGENTS.md** — project constraints, toolchain, rules
2. **The research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Read the ENTIRE document cover to cover. Every section, every subsection, every ADR, every appendix.
   - This document has been through 3 review cycles (iteration 000: procurement pipeline, data architecture, agent orchestration, landed-cost engine; iteration 001: frontend integration, search intelligence, skill reliability, MVP delivery strategy; iteration 002: worker-validator, pipeline integration, customs classification, sync engine) plus 1 harmonymaxer integration (worker-validator research). Multiple independent reviewers have contributed changes across all cycles, creating significant cross-section inconsistency risk.

Build a complete mental model:
- The value proposition (procurement intelligence with landed-cost transparency)
- The full data pipeline (ingestion → normalization → relation → qualification → doc acquisition → lot extraction → product extraction → quoteability scoring → price-signal matching → gross margin gate → landed cost → net margin gate → manifest → validation → operator decision → bid workflow)
- The daemon architecture (adapter boundary, PTY + contract mode, session lifecycle, resource arbiter, process-group supervision)
- The data architecture (authority matrix, SQLite + Supabase, push/pull sync, transactional outbox, stage coherence, schema_epoch)
- The skill layer (SKILL.md authoring, metadata, lifecycle, 6 skill categories, promotion flow, reliability patterns)
- The worker-validator system (run manifests, validator skills, validation policy engine, verdict model, fail-open semantics, OQG as first gate)
- The search intelligence (query waves, match archetypes, four-state evidence model, adaptive shortlisting, zero-result recovery, cross-language matching)
- The customs/landed-cost engine (HS classification, duty resolution, freight estimation, FX conversion, CIF formula, net margin gate)
- The frontend architecture (Tauri desktop, Next.js web, shared packages, daemon communication)
- The delivery strategy (7 milestones 0-6, MVE, validation-first philosophy, stack sequencing)
- All 8 ADRs and their rationale
- The file reference map and milestone assignments
- The testing, observability, and error handling strategies
- The four-layer observability model (control events, protocol events, forensic bytes, agent-generated run manifests)

---

## Step 2: Cross-Plan Improvement

With "fresh eyes," examine the full document for cross-cutting issues. Look carefully for each of the following categories. For each issue found, make a small, targeted edit — not a wholesale rewrite.

### 2.1 Contradictions Between Sections

Look for places where two sections describe the same concept differently:
- Does §4.10 (validator skill type, category 6) match §5.2 (Validation Executor, Validation Policy Engine) on how validators are invoked, what they consume, and what they produce?
- Does §5.3 (pipeline stages) match §5.6 (ingestion flow) and §5.7 (intelligence pipeline) on stage sequencing and handoff points?
- Does §4.5 (customs classification) match §5.7 (landed-cost pipeline steps) on HS lookup inputs, outputs, and fallback behavior?
- Does §4.7 (push/pull protocol) match §5.4 (outbox table, sync tracking columns) on field names and types?
- Does §5.5 (state model) match §5.3 (pipeline stages) on valid state transitions?
- Does §4.3 (supplier price intelligence) match §5.7 (price-signal matching steps) on query waves and shortlisting?
- Does §11 (milestones) match §8.2 (file map) on milestone assignments?
- Does §6.5 (testing strategy) match §4.10 (skill promotion flow) on test requirements for skill promotion?
- Does Appendix A (margin logic) match §5.7 (margin gate descriptions) on thresholds and formulas?
- Does Appendix C (job kinds) match §5.3 (pipeline stages) on the stage-to-job-kind mapping?
- Do ADRs match the sections that implement their decisions?

### 2.2 Inconsistent Terminology

Search for variant names for the same concept:
- "run manifest" vs "execution log" vs "session log" — run manifest is the agent-generated narrative; session log is the daemon-captured record
- "verdict" vs "validation result" vs "validation decision" — verdict is the standard term for validator output
- "authority" vs "source of truth" vs "ownership" — authority is the standard term from the authority matrix
- "push" vs "sync" vs "replicate" — push is local→remote, pull is remote→local, sync is the overall system
- "desktop" vs "control panel" vs "Tauri app" — pick one primary term
- "web portal" vs "viewing portal" vs "web shell" — pick one primary term
- "skill" vs "playbook" vs "SKILL.md" — when is each appropriate?
- "query wave" vs "query pack" vs "search profile" — are these synonyms or distinct concepts?
- "checkpoint" vs "intermediate result" vs "partial result"
- "match" vs "quote" vs "supplier quote" — clarify the lifecycle stages
- "notice" vs "opportunity" vs "contract" vs "process" — clarify the entity hierarchy
- "pipeline stage" vs "step" vs "phase" — pipeline stage is the standard term
- "margin gate" vs "margin check" vs "margin filter" — margin gate is the standard term
- "job kind" vs "job type" — job kind is the standard term from Appendix C
- "HS code" vs "tariff code" vs "customs code" — HS code is the standard term
- "outbox" vs "queue" vs "buffer" — outbox is the standard term for the transactional outbox
- "projection" vs "replica" vs "mirror" — projection is the standard term for Supabase copies

For each inconsistency found, standardize on one term and update all occurrences. Add a terminology note in §1 or the relevant section if the distinction is meaningful.

### 2.3 Missing Cross-References

Check that sections reference each other properly:
- §4.10 (validator skills) should reference §5.2 (Validation Executor, Validation Policy Engine)
- §5.2 (Run Manifest Writer) should reference §6.6 (Layer 4 observability)
- §5.6 (validation steps 12a-12d) should reference §5.2 (Validation Executor)
- §5.11 (VALIDATION_* errors) should reference §5.2 (verdict model)
- §4.5 (customs classification) should reference §5.7 (landed-cost pipeline steps) and Appendix A (margin formula)
- §4.7 (push protocol) should reference §5.4 (outbox table) and §5.2 (Sync Engine)
- §6.3 (RLS policies) should reference §4.7 (authority matrix)
- §5.3 (pipeline stages) should reference §5.5 (state model) and Appendix C (job kinds)
- §5.7 should reference §4.3 for source routing rules and §4.5 for classification pipeline
- §11 should reference §6.1 and §6.2 for complexity/risk-driven delivery decisions
- §6.5 should reference §4.10 for skill testing and validator testing patterns
- Component designs in §5.2 should reference the data models they consume/produce in §5.4
- §4.6 (FX strategy) should reference Appendix A (margin formula FX component)

### 2.4 Invariant Violations

Check for rules stated in one section that are violated by another:
- Authority matrix (§4.7, ADR-002): Is every table correctly categorized? Does any section imply writing to a table from the wrong authority?
- Mixed-write prohibition (ADR-002): Does any section imply both daemon and web write to the same table?
- Adapter boundary (§4.9, ADR-005): Do skills in §4.10 ever assume a specific execution mode?
- Fail-open semantics: Does any section outside §5.6 steps 12a-12d imply that validation failure blocks the pipeline?
- OQG-first principle: Does any section suggest the validator replaces or runs before the OQG?
- Dual confidence separation (§4.3): Is matchConfidence ever averaged with quoteUsabilityConfidence anywhere?
- "No mocks" testing (§6.5): Does any section reference mock-based testing?
- Integer pence for money (§5.4): Does any section use floating-point for monetary values?
- Plug-and-play composability: Does any section imply that adding a new skill requires daemon code changes?
- Conservative estimation bias: Does any landed-cost section prefer optimistic estimates over conservative?

### 2.5 Unnecessary Complexity

After 3 review cycles plus a harmonymaxer integration, some sections may have grown redundant or over-specified:
- Are there concepts explained in multiple places that could be explained once and cross-referenced?
- Are there subsections that repeat information from other subsections?
- Are there overly detailed specifications for components that haven't been validated yet?
- Are there open questions (§10) that have been resolved by the review cycles but not marked as resolved?
- Has the validation architecture (worker-validator) introduced any redundancy with the error manifest system?
- Has the sync engine specification grown beyond what's needed for the one-daemon-per-org MVP?

### 2.6 Missing Content

After 3 review cycles, some gaps should be visible:
- Are there sections that reference concepts not defined anywhere?
- Are there data flow diagrams that skip steps?
- Are there component designs that don't specify their inputs/outputs?
- Are there ADRs referenced but not defined, or defined but never referenced?
- Are there files in §8.2 that don't correspond to any component or data flow description?
- Are there pipeline stages in §5.3 without corresponding error types in §5.11?
- Are there validator test patterns in §6.5 without corresponding golden run manifests in §8.2?
- Are there customs/tariff edge cases mentioned in §4.5 without corresponding error handling in §5.11?

---

## Step 3: Architectural Coherence

Step back from the details and assess the architecture as a whole:

### 3.1 Purpose Clarity
- Does every component serve a clear purpose? Is there anything that exists "just in case"?
- Can you trace every component back to the value proposition (identifying profitable procurement opportunities)?
- Are there components that could be removed without affecting the core value chain?
- Does the worker-validator system earn its complexity? (It adds a second quality gate — is the first gate (OQG) insufficient?)

### 3.2 Boundary Placement
- Are the boundaries between components in the right place?
  - Desktop <-> Daemon: Is the API surface right-sized? Too many endpoints? Too few?
  - Daemon <-> Skills: Is the adapter boundary at the right abstraction level?
  - OQG <-> Validator: Is the boundary between code-based and agent-based quality gates clear?
  - SQLite <-> Supabase: Is the authority matrix correctly allocated?
  - Pipeline stages: Are the handoff points between stages well-defined?
  - Classification <-> Landed Cost: Is the boundary between HS lookup and cost calculation clear?

### 3.3 Dependency Direction
- Do dependencies flow in one direction (no circular dependencies)?
- Does the file map reflect the dependency structure?
- Are shared packages (`packages/ui`, `packages/contracts-schema`) properly isolated from implementation details?
- Does the validation system depend on the pipeline, or does the pipeline depend on the validation system? (It should be the former — validation is opt-in, not load-bearing.)

### 3.4 Complexity Allocation
- Is complexity concentrated in the right places (the hard problems: matching, landed cost, daemon orchestration, validation)?
- Are simple problems kept simple (ingestion, UI, sync for MVP)?
- Is the testing strategy proportional to complexity (more testing for higher-complexity components)?
- Is the customs classification pipeline appropriately simple for the China-to-UK corridor, or has it been over-generalized?

### 3.5 Delivery Coherence
- Does §11 (delivery strategy) align with §6.1 (complexity) and §6.2 (risk)?
- Does the MVP slice validate the riskiest assumptions?
- Is the milestone ordering consistent with the dependency graph?
- Are Milestones 3 (dry-run validation) and 4 (enforcement mode) correctly sequenced relative to the pipeline milestones?
- Does the validation-first delivery philosophy still hold after all iterations?

---

## Step 4: Sync

1. **Update §1 (Executive Summary):** Ensure it accurately summarizes the current state of the architecture after 3 review cycles plus harmonymaxer integrations. Quick Stats, Delivery Phases, Key Risks should all reflect the latest content. The worker-validator system, customs classification refinements, sync engine maturity, and pipeline integration improvements should all be reflected.

2. **Update §10 (Open Questions):** Mark any questions that have been resolved by the review cycles. Add any new questions that emerged from this holistic pass.

3. **Update §8.2 (File Reference Map):** Verify all files are accounted for, milestone assignments are consistent with §11, and no orphaned files exist. Validator skill files, customs classification modules, sync engine files, and pipeline integration files should all be present.

4. **Update ADRs:** Verify each ADR's rationale still matches the architecture. If review cycles added nuance (especially ADR-002 for sync, ADR-005 for adapter boundary), update the rationale.

5. **Update Appendix E (Changelog):** Add a concise entry for this holistic pass.

6. **Terminology glossary:** If significant terminology inconsistencies were found and fixed in Step 2.2, consider adding a brief terminology section to §1 or §2 to prevent future drift. After 3 review cycles, terminology drift is a real risk.
