# Pre-Prep: Worker-Validator & Run Manifest Architecture

## Step 1: Pre-Prep Research

Read the following documents in this exact order. Do not skip any.

1. **AGENTS.md** — project constraints, toolchain, rules
2. **The research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Read the ENTIRE document, but pay special attention to:
     - §4.10 (Skills and Browser Automation — skill types including category 6 validator skills, validation policy metadata declarations, skill metadata table)
     - §5.2 (Component Designs — Run Manifest Writer, Validation Policy Engine, Validation Executor, Output Quality Gate)
     - §5.6 (Ingestion Data Flow — ingestion steps 12a-12d covering validation sub-job creation, validator invocation, verdict application, and fail-open semantics)
     - §5.11 (Error Handling — VALIDATION_REJECTED, VALIDATION_FLAGGED, VALIDATION_SKIPPED error types and their relationship to the error manifest system)
     - §6.5 (Testing Strategy — validator testing patterns, verdict path coverage, dry-run vs enforcement mode testing)
     - §6.6 (Observability — Layer 4 agent-generated run manifests, validation telemetry metrics)
     - §5.4 (Data Models — validation_verdicts table [local-only], validation_verdict_summaries table [push-only], modified workflow_jobs and agent_session_logs fields)
     - §8.2 (File Reference Map — validator skill files, validation engine files)
     - §11 (Milestones — Milestone 3 = dry-run validation, Milestone 4 = enforcement mode)
3. **The handoff prompt that produced the feedback:** `docs/planmaxer/active/handoffs/handoff-worker-validator.md`

After reading, you should be able to explain:
- The run manifest lifecycle: skill execution produces narrative MD artifact, stored via Artifact Store, referenced by path + hash in agent_session_logs
- The validator skill design: category 6 skills with base + domain overlay structure, consuming run manifests as input
- The validation policy engine: four modes (always, sample, trigger, skip) with per-skill policy declarations
- The validation executor: how it creates sub-jobs (parent_job_id linkage), invokes validator skills, and renders verdicts
- The verdict model: approved/rejected/flagged outcomes, how rejections create error manifests, how flagged items enter operator review
- The fail-open semantics: validation failure (validator skill crashes) does not block the pipeline — creates VALIDATION_SKIPPED error
- The OQG relationship: OQG is first gate (code-based, fast), validator is second gate (agent-based, semantic) — OQG signals become validator inputs, not replacements
- The dry-run vs enforcement mode progression (Milestone 3 → Milestone 4)
- The plug-and-play principle: adding a new skill requires NO daemon code changes, NO schema changes, NO recompilation

**File Ownership:** This pre-prep owns the following sections for editing:
- §4.10 (Skills and Browser Automation) — Validator skill type (category 6), validation policy metadata, skill metadata table entries for validation ONLY
- §5.2 (Component Designs) — Run Manifest Writer, Validation Policy Engine, Validation Executor components ONLY
- §5.6 (Ingestion Data Flow) — Steps 12a-12d (validation sub-flow) ONLY
- §5.11 (Error Handling) — VALIDATION_REJECTED, VALIDATION_FLAGGED, VALIDATION_SKIPPED error types ONLY
- §6.5 (Testing Strategy) — Validator-specific testing subsections ONLY
- §6.6 (Observability) — Layer 4 (run manifest) observability and validation telemetry metrics ONLY

Do NOT edit: §4.9 (daemon orchestration internals), §5.4 (SQL table definitions — note changes for sync-engine domain), §4.7 (database architecture), §5.7 (intelligence pipeline flow), §5.3 (pipeline stages), §11 (milestone structure). The daemon's job runner, session lifecycle, and adapter boundary are settled. Validator skills must work within the existing skill framework, not modify it. If the reviewer suggests changes to daemon internals, the job runner, or SQL schema, note them as deferred with `<!-- CROSS-DOMAIN: ... -->` comments.

---

## Step 2: Feedback Integration

<INSERT REVIEWER RESPONSE HERE>

For each change proposed by the reviewer, evaluate:

### Integration Rules

1. **For run manifest format improvements:** Integrate into §5.2 (Run Manifest Writer component). The current design specifies narrative MD format with structured metadata header. If the reviewer proposes additional structure (sections, required fields, machine-parseable blocks), integrate while preserving the "human-readable narrative" principle. Run manifests are forensic artifacts — they must be readable by operators, not just validators.

2. **For validator skill architecture improvements:** Integrate into §4.10 (category 6 skill type) and §5.2 (Validation Executor). If the reviewer proposes overlay composition patterns, domain-specific validators, or validator chaining, integrate while preserving plug-and-play composability. Adding a new validator must require only a SKILL.md file and policy declaration, not daemon changes.

3. **For validation policy engine refinements:** Integrate into §5.2 (Validation Policy Engine). If the reviewer proposes additional policy modes beyond always/sample/trigger/skip, or proposes policy inheritance rules, or proposes confidence-based sampling, integrate with concrete configuration examples. Policies must be declarative (in skill metadata), not imperative (in daemon code).

4. **For verdict model improvements:** Integrate into §5.2 (Validation Executor) and §5.11 (error types). If the reviewer proposes verdict severity levels, partial approvals, conditional approvals, or verdict expiry, evaluate against the principle that verdicts are binary-ish (approved/rejected/flagged) to keep the pipeline simple. Complex verdict taxonomies risk becoming a second error taxonomy.

5. **For fail-open semantics refinements:** Integrate into §5.6 (steps 12a-12d) and §5.11 (VALIDATION_SKIPPED). If the reviewer proposes configurable fail-open vs fail-closed per skill or per policy, integrate into the policy engine. The default MUST remain fail-open for MVP.

6. **For validation testing strategy:** Integrate into §6.5. Add validator-specific test patterns: verdict path coverage (all three outcomes), dry-run assertion tests, golden run manifest + expected verdict pairs, validator skill promotion criteria. Stay within the "no mocks" constraint — validator tests use real validator skills against real (or golden) run manifests.

7. **For validation observability:** Integrate into §6.6. If the reviewer proposes additional metrics beyond the five currently specified, or proposes alerting thresholds (e.g., rejection rate spike), or proposes verdict audit trails, integrate into Layer 4 and the validation telemetry subsection.

8. **For changes touching SQL schema, job runner internals, pipeline stage definitions, or milestone structure:** Do NOT integrate directly. Add `<!-- CROSS-DOMAIN: Reviewer suggested [X] for [component]. Needs coordination with [domain]. -->` notes.

### Evaluation Framework

For each proposed change, assess:
- **Agree strongly:** Fills a concrete gap in validation architecture, improves verdict reliability, or strengthens the forensic audit trail. Apply directly.
- **Agree partially:** Good pattern but risks adding complexity to the validator skill authoring experience (which must stay simple — SKILL.md + policy declaration). Apply with simplification, noting tradeoffs.
- **Disagree:** Conflicts with settled decisions (fail-open default, OQG as first gate, plug-and-play composability, narrative MD format for run manifests, no daemon code changes for new skills). Note disagreement with rationale.

### Preservation Rules
- Preserve fail-open as the default validation semantics (validator crash does not block pipeline)
- Preserve OQG as first gate (code-based) and validator as second gate (agent-based) — never merge them
- Preserve run manifest format as narrative MD with structured metadata header
- Preserve plug-and-play composability (new skill = SKILL.md + policy, no daemon changes)
- Preserve the three verdict outcomes (approved/rejected/flagged)
- Preserve parent_job_id linkage for validation sub-jobs
- Preserve the dry-run → enforcement mode progression
- Preserve the error taxonomy structure (VALIDATION_* types under the existing taxonomy)
- Use "as-if-always" language

---

## Step 3: Fresh-Eyes Optimization

After completing Step 2, re-read the sections you edited with fresh eyes. Check for:

1. **§4.10 skill metadata ↔ §5.2 Validation Policy Engine consistency:** Does the validation_policy metadata declared in skill SKILL.md files (§4.10) match what the Validation Policy Engine consumes (§5.2)? Are the four policy modes (always, sample, trigger, skip) referenced identically in both sections? If the reviewer added new policy modes, are they reflected in both places?

2. **§5.2 verdict types ↔ §5.11 error types consistency:** Do the three verdict outcomes (approved/rejected/flagged) in the Validation Executor (§5.2) produce the correct corresponding error types in §5.11? Specifically: rejected → VALIDATION_REJECTED (creates error manifest), flagged → VALIDATION_FLAGGED (enters operator queue), validator crash → VALIDATION_SKIPPED (fail-open). Are there any verdict outcomes that don't map to an error type, or error types that don't map to a verdict?

3. **§6.6 Layer 4 ↔ §5.2 Run Manifest Writer consistency:** Does the observability description of Layer 4 (agent-generated run manifests) in §6.6 match the run manifest format defined in the Run Manifest Writer component (§5.2)? Are the same fields/sections referenced? If the reviewer changed the manifest format, is the observability layer updated?

4. **§6.5 validator testing ↔ verdict paths consistency:** Does the testing strategy in §6.5 cover all three verdict paths (approved, rejected, flagged) plus the fail-open path (VALIDATION_SKIPPED)? Are there golden run manifests specified for each path? Does the validator promotion flow in §4.10 require passing these tests?

5. **§5.6 steps 12a-12d ↔ §5.2 Validation Executor consistency:** Do the ingestion flow steps describing validation (§5.6) match the component design for the Validation Executor (§5.2)? Same sequence, same decision points, same fail-open behavior?

6. **Validation telemetry metrics consistency:** Are the validation-specific metrics in §6.6 consistent with what the Validation Executor (§5.2) and Validation Policy Engine (§5.2) can actually emit? No phantom metrics that nothing produces.

7. **Terminology:** Ensure consistent use of "run manifest" (not "execution log" or "session transcript"), "verdict" (not "result" or "decision" when referring to validation outcomes), "validator skill" (not "validation agent" or "checker"), "policy" (not "rule" or "config" when referring to validation triggering).

Fix any inconsistencies found. Make targeted edits, not wholesale rewrites.

---

## Step 4: Sync Pass

1. **Update §1 (Executive Summary):** If the validation architecture changed materially (new policy modes, changed verdict model, altered fail-open semantics), update the executive summary to reflect the current state of the worker-validator system. The four-layer observability model should still reference Layer 4 correctly.

2. **Update Appendix E (Changelog):** Add a concise entry describing what was integrated in this review cycle. Include: which sections were modified, what the reviewer's key contributions were, and any deferred cross-domain items.

3. **Verify cross-references between validation sections:** Every mention of validation in §4.10 should cross-reference §5.2 components. Every mention of validation errors in §5.11 should cross-reference the verdict model in §5.2. Every mention of validation testing in §6.5 should cross-reference the validator skill type in §4.10. Every mention of run manifests in §6.6 should cross-reference the Run Manifest Writer in §5.2.

4. **Check §10 (Open Questions):** If any open questions about validation have been resolved by this review cycle, mark them as resolved. If new questions emerged, add them.

5. **Verify §8.2 file references:** If the reviewer proposed new validator skill files, validation engine source files, or test fixtures, ensure they appear in §8.2 with correct milestone assignments (validation files should be in Milestone 3 for dry-run, Milestone 4 for enforcement).
