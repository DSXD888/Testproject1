# Layer Strategy Optimization — UK Public Procurement Intelligence Platform

> **Role:** You are a fresh-eyes architect specializing in layer decomposition quality assurance. You have NOT seen this strategy before. Your job is to find every flaw, gap, and missed dependency.
>
> **Deliverable:** An improved version of `docs/strategy/layer-strategy.md` with targeted fixes. Not a rewrite — surgical improvements that fix specific problems.
>
> **Scope:** Validation and optimization only. Do not redesign the system — improve the decomposition.

---

## 0. Mandatory Deep Re-Read

**BEFORE ANY VALIDATION**, you must:

1. **Read the ENTIRE source document** (`docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`, ~8,400 lines). Do NOT skim. Cross-cutting dependencies are invisible when skimming.

2. **Read the ENTIRE strategy document** (`docs/strategy/layer-strategy.md`). Build a mental model of the proposed decomposition.

3. **Read `AGENTS.md`** for project constraints and toolchain.

4. **Note divergences.** As you read, note any place where the strategy DIVERGES from the source document — different terminology, missing concepts, simplified models, wrong authority classifications.

---

## 1. Structural Validation (Binary Pass/Fail)

**ANY failure in this section requires a fix.** These are not opinions — they are structural correctness checks.

### 1.1 No Forward References

For each layer, verify that it does NOT reference:
- Tables not introduced in an equal or prior layer
- TypeScript interfaces not defined in an equal or prior layer
- Skill categories not established in an equal or prior layer
- Daemon components not built in an equal or prior layer

**How to check:** For each layer N, list every component/table/interface it uses. Verify each one is introduced in layer ≤ N.

### 1.2 No Invariant Gaps

Verify that:
- The authority matrix (push-owned vs remote-owned vs local-only) is correctly assigned for EVERY table in the layer that introduces it
- No invariant is "temporarily relaxed" between layers — if a rule applies to data in layer N, it must be enforced in layer N
- Integer pence for money is used from the first table that stores money
- Basis points for confidence is used from the first table that stores confidence
- Process-group supervision is active from the first layer that runs agent sessions

**Specific invariants to check:**
- `owner_id` on every push-owned table from its introduction
- `row_version` on every push-owned table from its introduction
- `last_local_seq` on every push-owned table from its introduction
- `stage_run_id` on every pipeline table from its introduction
- `content_hash` on outbox entries from sync engine introduction
- Cleanup verdicts on every agent session from first agent execution
- Error manifests for every pipeline error from first pipeline stage

### 1.3 No Schema Migrations Between Layers

Verify that NO layer requires:
- Adding columns to tables introduced in earlier layers (forward-compatibility columns must exist from introduction)
- Changing column types or constraints on existing tables
- Reclassifying a table's authority (push-owned → remote-owned or vice versa)
- Renaming tables or columns

**Specifically check:**
- `workflow_jobs.parent_job_id` — must exist (nullable) when workflow_jobs is introduced, even if validation comes later
- `workflow_jobs.validation_required` — must exist (default false) when workflow_jobs is introduced
- `workflow_jobs.validation_verdict_id` — must exist (nullable) when workflow_jobs is introduced
- `agent_session_logs.run_manifest_path` — must exist (nullable) when agent_session_logs is introduced
- `agent_session_logs.run_manifest_hash` — must exist (nullable) when agent_session_logs is introduced
- `pipeline_assessments.validation_state` — must exist (default 'unverified') when pipeline_assessments is introduced

### 1.4 Atomic Pairs Never Split

Verify these pairs are NEVER split across layers:

| Pair A | Pair B | Why |
|--------|--------|-----|
| Push-owned table write | `local_outbox` entry | Data written locally but never synced |
| Skill execution | Run manifest (at minimum, fallback) | Forensic data permanently lost |
| Job execution | `process_stage_status` update | Pipeline state machine inconsistent |
| `process_stage_status` write | `process_stage_heads` update | Stage tracking diverges from reality |
| Worker job | `validation_required` field on `workflow_jobs` | Can't add validation later without schema change |

If sync (outbox) is deferred to a later layer than table writes, verify that:
- Sync tracking columns (`owner_id`, `row_version`, `last_local_seq`) EXIST on tables from their introduction
- These columns are POPULATED at write time (not left null until sync ships)
- Only the sync ENGINE (outbox entries, push/pull batches) is deferred, not the sync SCHEMA
- The layer that activates sync introduces the `local_outbox` table AND makes outbox entries atomic with data writes
- The `schema_epoch` field exists on outbox/sync infrastructure from its introduction

### 1.5 Every Layer Has User-Facing Capability

Verify that every layer (except possibly a "Layer 0" gold-set/evaluation-harness) delivers at least one new capability that a user (developer or operator) can observe. "User" includes developers running tests.

### 1.6 Every Layer Has Testable Definition of Done

Verify that every layer has at least 5 concrete acceptance criteria that can be tested WITHOUT requiring later layers.

### 1.7 Decisions Addressed

Verify that every ADR from the source document is addressed in exactly one layer (or explicitly split with justification):
- ADR-001: Skill-based agent scraping → should be in the first pipeline layer (Layer 1)
- ADR-002: Authority-matrix database → schema from Layer 1, sync engine in a later layer — split must be explicit
- ADR-003: Desktop control panel + web viewing portal → typically split: desktop in one layer, web in a later layer
- ADR-004: Hybrid PTY + contract execution → first pipeline layer (Layer 1) at minimum needs PTY; contract mode can come later
- ADR-005: Process-group supervision with cleanup verdicts → must be active from first agent session (Layer 1)
- ADR-006: Conservative bounding engine (customs) → screening-grade in early pipeline layer, full in later layer — split must be explicit
- ADR-007: Error manifest system → from first pipeline error (Layer 1)
- ADR-008: Shared component boundary (frame + slot) → desktop layer

For ADRs that are naturally split (002, 003, 004, 006), verify that the split is documented, that both halves are assigned to specific layers, and that the first half is forward-compatible with the second.

### 1.8 Layer 0 Is Evaluation Infrastructure

Verify that a Layer 0 (gold set + evaluation harness) exists as the foundation. The research doc's M0 milestone defines this as ~25-35 files: curated procurement notices, expected pipeline outputs, evaluation metrics, and a test harness. This is the validation-first philosophy — build the measurement system before the thing being measured.

### 1.9 First Pipeline Layer Completes the Loop

Verify that the first PIPELINE layer (Layer 1, not Layer 0) completes the fundamental operational cycle:

```
Notice ingestion → Pipeline processing → Verifiable output
```

**"Verifiable output" means:** A test assertion, CLI dump, or file-based manifest that proves the complete value chain works. It does NOT require:
- Desktop UI (comes in a later layer)
- Supabase sync (comes in a later layer)
- Operator decisions via CAS (local SQLite write is sufficient)

The complete flow (ingest → qualify → price → margin gate → landed cost → manifest) must work end-to-end, but the "operator sees it" step can be a test or CLI output. This allows the pipeline to be validated against the Layer 0 gold set before any UI is built.

---

## 2. Logical Soundness Validation

### 2.1 Independence Test

For each layer N, ask: **"What if we never build layer N+1?"**

- Is the system still useful? (If not, consider merging with the next layer)
- Is it independently deployable? (If not, it's not a real layer boundary)
- Does it deliver value that justifies the development effort? (If not, consider merging)

### 2.2 Complexity Reasonableness

For each layer, verify:
- The scope is achievable in a focused effort (not so large it should be split)
- The scope is substantial enough to warrant being a separate layer (not so small it should be merged)
- Component count is roughly balanced across layers (no "kitchen sink" layer)

### 2.3 Dependency Graph Linearity

Verify:
- Are there genuine circular dependencies between layers? (If so, the layer order is wrong)
- Are there opportunities for parallelization the strategy doesn't acknowledge?
- Are dependencies between layers truly necessary or overly conservative?

### 2.4 User Value Delivery Timing

Verify:
- When does the first verifiable end-to-end pipeline result appear? (Should be Layer 1 — tested against gold set)
- When does the desktop UI appear? (Should not be deferred past layer 3-4 — visual validation of pipeline output is high value)
- When does sync + web portal appear? (Can be later, but should not be the very last layer — sync bugs found early are cheaper)
- When does validation appear? (Enhancement, not foundation — can be later, but dry-run should precede enforcement)
- Does the ordering follow the research doc's recommended stack sequencing? (Skills → TS types → Rust daemon → Desktop → Web)

### 2.5 Governance Transition

Verify:
- How are source configs provided in the first layer? (Hardcoded? Seed file? Supabase?)
- How does company profile data get populated before the web portal exists?
- How does the operator make decisions before sync is implemented? (Local-only mode?)
- Is the transition from bootstrapped config to permanent config explicit in the strategy?

---

## 3. Layering Quality Validation (Hard Questions)

### 3.1 Layer Boundary Swap Test

For each pair of adjacent layers (N, N+1):
- **What would CONCRETELY break if you swapped their order?**
- If nothing would break, consider whether the layer boundary is in the right place.
- If many things would break, the boundary is likely correct.

### 3.2 Tight/Loose Coupling Check

- Are there tightly-coupled subsystems split across layers?
  - Example: Job Runner and Process Supervisor are tightly coupled — splitting them would create integration issues
  - Example: Run Manifest Writer and Artifact Store are tightly coupled — manifests need somewhere to live
- Are there loosely-coupled subsystems bundled unnecessarily?
  - Example: Web portal and Desktop app are loosely coupled — they can be in different layers
  - Example: Customs classification and freight estimation are loosely coupled — they can be phased in separately

### 3.3 Size Balance Check

- Is any layer so large it should be split?
  - A layer with 8+ new tables, 4+ new daemon components, AND new skills is probably too large
- Is any layer so small it should be merged?
  - A layer that only adds one column to one table is too small
- Does each layer take roughly similar effort? (No 1-week layer followed by a 3-month layer)

### 3.4 Stubbing Completeness

For each layer's "what's stubbed" section:
- Is there a concrete "enhanced in layer N" reference for every stub?
- Are the stubs viable? (Can you actually implement a screening-grade landed cost, or does it require the full customs classification pipeline?)
- Are stub interfaces forward-compatible? (Will the enhanced version be a drop-in replacement, or will callers need to change?)

### 3.5 Interface Contract Explicitness

For each layer boundary:
- Are the interface contracts explicit enough for independent teams to work on adjacent layers?
- Are there implicit contracts that should be documented?
- Would a team building layer N+1 know EXACTLY what layer N provides?

---

## 4. Project-Specific Validation

### 4.1 Authority Matrix Correctness

For EVERY table in the strategy, verify its authority classification against the source document:

- Push-owned: daemon writes, synced to Supabase
- Remote-owned: Supabase writes, pulled by daemon
- Local-only: never synced
- Projection: push-only summary for web visibility

**Pay special attention to:**
- `operator_decision_heads` — MUST be remote-owned (not push-owned)
- `shared_source_configs` — MUST be remote-owned
- `shared_company_profile` — MUST be remote-owned
- `validation_verdicts` — MUST be local-only
- `validation_verdict_summaries` — MUST be push-only projection
- `local_outbox`, `sync_batches`, `pull_cursors` — MUST be SQLite-only

### 4.2 Pipeline Stage Scoping

Verify that the strategy acknowledges the scope grain per stage by checking against the source document's `ProcessStageStatus.scopeKind` definition (§5.4). The 6 possible scope kinds are: `process`, `notice`, `lot`, `candidate`, `candidate_source_pair`, `candidate_quote_scenario`. Each of the 15 pipeline stages operates at a specific grain — verify the strategy's assignments match the research doc.

This matters because layers that introduce stages at a finer grain must also introduce the entities at that grain. For example: lot-level stages require `notice_lots`, candidate-level stages require `product_candidates`, candidate-source-pair stages require `supplier_quotes`.

### 4.3 Customs Classification Complexity Spectrum

Verify the strategy handles the complexity spectrum:
- Screening-grade: single-path FPO recommendation, simplified duty lookup
- Full: multi-source recall, multi-family expansion, measure interaction engine, exporter resolution, trade remedy monitoring

The strategy MUST NOT put full customs in the first layer. It should start screening-grade and enhance.

### 4.4 Skill Promotion Flow

Verify the strategy accounts for the 6-stage skill promotion flow (draft → candidate → canary → shadow → stable → rollback). This doesn't need to be fully implemented in the first layer, but the metadata structure must exist.

### 4.5 Observability Evolution

Verify the strategy addresses the 4-layer observability model across layers:
- Layer 1 (control events): daemon-emitted structured events → from first daemon layer
- Layer 2 (protocol events): adapter-level events → from first skill execution
- Layer 3 (forensic bytes): raw session capture → from first PTY session
- Layer 4 (run manifests): agent-generated narratives → from first skill run

### 4.6 Sync Tracking Column Presence

For EVERY push-owned table, verify that `owner_id`, `row_version`, and `last_local_seq` are specified in the layer that INTRODUCES the table — NOT deferred to the layer that implements sync.

---

## 5. Execution Artifact Policy Validation

### 5.1 Maintained Doc Set

Verify the strategy defines:
- Exact file count for the maintained doc set
- File paths for every maintained file
- Authority split between strategy doc and layer docs
- No "companion docs" or "decision files" outside the maintained set

### 5.2 Cross-Layer Material

Verify that all cross-cutting concerns are embedded in appropriate files:
- Schema stability matrix → in strategy doc
- Interface contracts → in strategy doc
- Testing evolution → in strategy doc
- Open questions → in the owning layer doc, not a separate file

---

## 6. Fix Protocol

For each issue found:

1. **Classify:** Structural failure (must fix) vs logical concern (should fix) vs quality improvement (nice to fix)
2. **Explain:** Why this is a problem, with specific reference to the source document section
3. **Fix:** Make a targeted edit to the strategy document. Do NOT rewrite large sections unless structurally necessary.
4. **Verify:** After the fix, verify it doesn't break other layer boundaries.

**Priority order for fixes:**
1. Structural failures (§1) — fix first, these are binary pass/fail
2. Loop completeness (§1.8) — critical for the strategy's foundation
3. Forward-compatibility gaps (§1.3) — create schema migration debt
4. Authority matrix errors (§4.1) — create sync nightmares
5. Everything else

---

## 7. Follow-Up Fresh-Eyes Pass

After ALL fixes are applied:

1. Re-read the entire strategy document one more time with completely fresh eyes.
2. Ask yourself: **"If I gave this to an implementation team tomorrow, would they know exactly what to build first, second, third — with no ambiguity about layer boundaries?"**
3. Ask yourself: **"Are there any layers where a developer would need to open the 8,400-line research doc to figure out what to build? If so, the strategy doc needs more detail."**
4. Ask yourself: **"Does the layer count feel right? Not too few (layers too large to be manageable), not too many (layers too thin to be meaningful)?"**
5. Make any final adjustments.

---

## 8. Output

When complete, provide:

1. **Summary of changes:** Bulleted list of what was fixed and why
2. **Structural validation results:** Pass/fail for each check in §1
3. **Remaining concerns:** Any issues that were noted but not fixed (with rationale)
4. **Confidence assessment:** Your confidence that this strategy is ready for beads creation (high/medium/low, with reasons)
