# Harmonization Handoff — Worker-Validator & Run Manifest System Integration

> **Run:** 002
> **Source material:** `docs/research/2026-03-15-worker-validator-run-manifest-research.md` (2256 lines, RESEARCH DOC MODE)
> **Target doc:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` (~7268 lines)
> **Integration type:** FEATURE ADDITION + ARCHITECTURE EVOLUTION (hybrid)
> **Generated:** 2026-03-15

---

## 1. Mission Statement

You are a chief systems integrator updating the master architecture document for a UK public procurement intelligence platform. Your deliverable: an updated research document that fully absorbs the worker-validator and run manifest system from the standalone research doc, so that the standalone doc becomes genuinely redundant.

**You are editing documentation only. Do not change code.**

Primary goal: one coherent architecture model — no contradictions between the existing content and the new worker-validator material, no stale content, no ambiguity that could produce incompatible implementations.

Secondary goal: implementation-safe docs — an engineer reading only the main research doc gets the complete picture of how worker-validator operates within the existing Job Runner → Adapter → Result Contract Validator → OQG pipeline.

---

## 2. Implementation Context

**This is a greenfield project with NO code yet.** The research document is the foundational architecture brief. There is no implementation state to track. All content describes intended architecture.

---

## 3. MANDATORY Pre-Read Requirement

**CRITICAL — Before you touch any documentation, you MUST perform a deep-reading and comparison pass.**

This is not optional. Do not start editing after reading only the source material.

### Required Steps:

1. **Read the source material in full:** `docs/research/2026-03-15-worker-validator-run-manifest-research.md` (all 2256 lines). Understand every section — the run manifest lifecycle, validator skill anatomy, validation policy engine, dry-run mode, manual mode, DB entities, integration points, and the plug-and-play composability principle.

2. **Read the target document in full:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` (all ~7268 lines). Pay special attention to:
   - §4.9 (Agent Orchestration — hybrid execution model, session lifecycle)
   - §4.10 (Skills — skill types, lifecycle, design principles, production directory standard, outcome classes, promotion flow, versioning, browser automation reliability, model policy)
   - §5.2 (Components — Job Runner, Result Contract Validator, Output Quality Gate, Artifact Store, Error Manifest System, Resource Arbiter)
   - §5.4 (Data Model — workflow_jobs, agent_session_logs, all entity interfaces and SQL tables)
   - §5.5 (Contract State Model)
   - §5.6 (Ingestion Data Flow — steps 10-17, error cases)
   - §5.7 (Intelligence Pipeline Data Flow)
   - §5.11 (Error Handling Strategy — error types, outcome classes, error manifest system)
   - §6.1 (Complexity Analysis)
   - §6.2 (Risk Assessment)
   - §6.5 (Testing Strategy — all test categories)
   - §6.6 (Operational Observability — three-layer model, metrics, artifact storage)
   - §8.2 (Files to Create — daemon files, skill files)
   - §8.3 (Test Files Required)
   - §8.4 (Milestone Assignment Map)
   - §11 (Next Steps — milestone definitions)
   - Appendix E (Risk Register)
   - ADR-005 (Skills boundary)

3. **Read AGENTS.md** for project rules (especially: no file deletion, only `main` branch, Rust 2024 nightly, Bun for TS).

4. **Build a private comparison matrix:**
   - What the source material adds (run manifests, validator skills, validation policy, new DB entities)
   - What the source material changes (Job Runner, workflow_jobs, agent_session_logs, OQG role, promotion flow)
   - What currently exists in the target doc for each affected area
   - Where terminology might conflict ("manifest" is already used for opportunity manifests and error manifests)

5. **Only THEN begin harmonizing the document.**

You must not skip or collapse this research pass.

---

## 4. What to Read

### Primary Source (integrate FROM):

| Path | Lines | Focus |
|------|-------|-------|
| `docs/research/2026-03-15-worker-validator-run-manifest-research.md` | ~2256 | Everything — this is the material being absorbed |

Key source concepts that MUST survive integration:
- **Run manifests** — every skill run produces an MD narrative artifact describing what happened, what was found, how the agent interprets its results. Human-readable workflows, not code.
- **Validator skills** — separate agent skills that review run manifests and render verdicts (approved/rejected/flagged). Base validator + domain-specific overlays.
- **Validation policy engine** — controls when validation runs based on skill category, decision impact, and historical reliability. Modes: always, sample, trigger, skip.
- **Dry-run mode** — validation infrastructure without enforcement (MVP).
- **Manual mode** — kickoff prompt renderer for human-initiated validation.
- **New DB entities** — `validation_verdicts`, `validation_verdict_summaries` (Supabase projection).
- **Modified DB entities** — `workflow_jobs` (parent_job_id, validation_required), `agent_session_logs` (run_manifest_path, run_manifest_hash).
- **Plug-and-play composability** — adding a new skill requires NO daemon code changes, NO schema changes, NO recompilation. The validator skill overlay is loaded from the skill directory, not hardcoded.
- **Skills are human-readable workflows, not code** — validation must be semantic (agent-based), not syntactic (code-based). The validator reads the run manifest as a human would.

### Primary Target (integrate INTO):

| Path | Lines | Focus |
|------|-------|-------|
| `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` | ~7268 | The single monolithic research/architecture document |

**Priority sections to modify (PRIMARY — substantial changes):**

| Section | What changes |
|---------|-------------|
| §1 (Executive Summary) | Add worker-validator to "key recommendations" — semantic validation layer. Update "what will make or break" to mention validator catching schema-valid garbage |
| §4.10 (Skills and Browser Automation) | Add "Validator Skills" as category 6 to Skill Types table. Add run manifest generation to skill execution lifecycle. Add validation policy to skill metadata. Integrate validator into promotion flow. Add validator skill directory standard. |
| §5.2 (High-Level Component Design) | Add new component: "Validation Executor" (or integrate into Job Runner description). Add new component: "Validation Policy Engine". Modify Job Runner to describe validation sub-job spawning. Modify Result Contract Validator to clarify its relationship with the new semantic validator. Modify Output Quality Gate to explain that OQG signals become validator inputs. |
| §5.4 (Data Model Design) | Add `ValidationVerdict` TS interface and SQL table. Add `validation_verdict_summaries` SQL table. Modify `workflow_jobs` interface/SQL to add `parent_job_id` and `validation_required`. Modify `agent_session_logs` interface/SQL to add `run_manifest_path` and `run_manifest_hash`. Update authority split table (§4.7). |
| §5.6 (Ingestion Data Flow) | Insert validation step after step 12 (Result Contract Validator / OQG) — daemon checks validation policy, optionally spawns validator sub-job, processes verdict. Add validation to error cases. |
| §5.11 (Error Handling Strategy) | Add validation-specific error types (VALIDATION_REJECTED, VALIDATION_FLAGGED). Add validator outcome to skill outcome classes or create separate section. Clarify that validator rejections create error manifests. |
| §6.1 (Complexity Analysis) | Add "Worker-validator semantic validation" complexity entry |
| §6.2 (Risk Assessment) | Add validation-related risks (validator disagrees with OQG, validator itself produces wrong verdicts, validation cost overhead) |
| §6.5 (Testing Strategy) | Add validator testing section — validator skill canaries, golden journey tests for validator skills, dry-run mode testing |
| §6.6 (Operational Observability) | Add validation metrics — approval rate, rejection rate, flagging rate, false positive/negative rates, validator cost per job |
| §8.2 (Files to Create) | Add validator daemon files, validator skill files, validation policy files |
| §8.3 (Test Files Required) | Add validator test files |
| §8.4 (Milestone Assignment Map) | Assign validator files to milestones (likely Milestone 3 or new milestone) |
| §11 (Next Steps) | Integrate validation into milestone descriptions |
| Appendix E (Risks) | Add validator-related risks |

**SECONDARY targets (consistency alignment):**

| Section | What needs alignment |
|---------|---------------------|
| §4.7 (Database Architecture) | Authority split table needs `validation_verdicts` (push-owned) and `validation_verdict_summaries` (push-owned projection) |
| §4.9 (Agent Orchestration) | Session lifecycle may need to note that validator sessions follow the same lifecycle as worker sessions |
| §5.3 (Canonical Workflow) | May need a "validate" step in the pipeline flow diagram |
| §5.5 (Contract State Model) | Pipeline state may need new states or notes about validation hold |
| §5.7 (Intelligence Pipeline Data Flow) | Same validation step insertion pattern as §5.6 |
| §5.9 (Request Pipeline Automation) | Note that bid automation skills should always-validate given their operational consequence |
| ADR-005 | May need update to mention validator skills as a new skill category that follows the same boundary rules |

**LIGHT TOUCH (only if contradictions exist):**

| Section | Check for |
|---------|-----------|
| §4.1–4.8 (other external tech) | No changes expected |
| §5.1 (System Context Diagram) | Probably fine as-is — validation is internal |
| §5.8 (Frontend) | Desktop might need to surface validation verdicts — mention only if source doc requires it |
| §5.10 (API Design) | May need validation verdict endpoints |
| §6.3–6.4 (Security, Performance) | Light check for validation impact |
| §9 (Dependencies) | No new external dependencies expected |

---

## 5. Core Harmonization Goals

### Goal 1: Clear Delineation Between OQG and Semantic Validation

The existing Output Quality Gate (§4.10, §5.2) performs three validation layers: syntactic (schema), semantic (business invariants), and behavioral/drift (statistical envelope). The new worker-validator system performs a FOURTH layer: agent-based semantic validation — an agent reads the run manifest as a human would and judges whether the output makes sense.

**What must be consistently stated:** OQG catches structural and statistical anomalies (wrong fields, null-rate spikes, duplicate rate changes). The semantic validator catches contextual errors that are invisible to code-based checks (navigated to wrong page but extracted valid-looking data, Scottish notices from an England-only source, prices from consumer listings when industrial was required). OQG signals FEED INTO the validator as inputs, not compete with it.

**Affected sections:** §4.10 (skill validation), §5.2 (OQG component, new Validation Executor component), §5.6 (ingestion flow step 12), §5.11 (error handling)

### Goal 2: Run Manifest as Forensic Artifact

Run manifests must be described consistently with the existing artifact storage model (§6.6). They are stored in the artifact store alongside raw session output, not inline in SQLite.

**What must be consistently stated:** Run manifests are immutable compressed MD artifacts stored at `artifacts/manifests/YYYY/MM/<job_id>.md`. SQLite stores: `run_manifest_path`, `run_manifest_hash`. The agent_session_logs table gets these two new columns.

**Affected sections:** §5.4 (data model), §6.6 (artifact storage model), §5.2 (Artifact Store component)

### Goal 3: Validator Skills Follow Existing Skill Patterns

Validator skills must follow the same production skill directory standard as all other skills (SKILL.md + schemas + canary inputs + golden assertions). They are a new category in the skill type table, not a special class.

**What must be consistently stated:** Validator skills are category 6 in the skill type table. They declare `interaction_profile: headless_contract` (they read manifests, not browse websites). They have base validators (generic quality checks) and domain-specific overlays (loaded from the worker skill's directory). They follow the same promotion flow as all skills.

**Affected sections:** §4.10 (skill types table, skill lifecycle, promotion flow), §8.2 (skill files)

### Goal 4: Plug-and-Play Composability Preserved

The source material emphasizes that adding a new skill should require NO daemon code changes, NO schema changes, NO recompilation. The validation policy and validator overlays must be loadable from the skill directory structure.

**What must be consistently stated:** Validation policy is declared in skill metadata (not daemon config). Validator overlays live in the worker skill's directory (e.g., `skills/procurement-ingestion/fts/validator-overlay.md`). The daemon discovers and loads these at runtime without code changes.

**Affected sections:** §4.10 (skill metadata, skill design principles), §5.2 (Validation Policy Engine component), ADR-005

### Goal 5: Validation Integrates into Existing Job Flow

The validation step must be described as a natural extension of the existing Job Runner → Adapter → Result Contract Validator → OQG pipeline, not a parallel system.

**What must be consistently stated:** After OQG passes, the Job Runner checks the validation policy. If validation is required, it spawns a validator sub-job (parent_job_id links back). The validator agent reads the run manifest, renders a verdict. The Job Runner processes the verdict: approved → advance stage; rejected → quarantine as AGENT_SUSPECT_OUTPUT + create error manifest; flagged → hold for operator review.

**Affected sections:** §5.2 (Job Runner), §5.4 (workflow_jobs), §5.6 (ingestion flow), §5.7 (intelligence flow)

### Goal 6: Terminology Clarity

Three things are called "manifest" in this architecture:
1. **Opportunity manifest** — the Markdown summary of an opportunity (existing, §5.2 Manifest Generator)
2. **Error manifest** — the structured error record (existing, §5.2 Error Manifest System)
3. **Run manifest** — the MD narrative of what a skill run did (NEW)

**What must be consistently stated:** Always use the full term "run manifest" when referring to the new concept. Never just "manifest" when the run manifest is meant. The run manifest is stored in the artifact store; the opportunity manifest is stored in the manifests table; the error manifest is stored in the error_manifests table.

**Affected sections:** All sections that mention any kind of manifest

### Goal 7: DB Schema Follows Authority Matrix

New tables must follow the established authority matrix pattern (§4.7).

**What must be consistently stated:** `validation_verdicts` is local-only (no sync to Supabase — contains full validator reasoning). `validation_verdict_summaries` is push-owned (local → remote projection, contains verdict + issue count + summary). These follow the same pattern as `agent_session_logs` (full locally) / redacted summaries pushed to Supabase.

**Affected sections:** §4.7 (authority split table), §5.4 (data model), SQL tables

### Goal 8: Milestone Placement

The worker-validator system should be assigned to an appropriate milestone. Given that it catches semantic errors that OQG misses, it's valuable early but not MVE-blocking.

**What must be consistently stated:** Dry-run mode (manifests generated, validation runs but doesn't block) is appropriate for Milestone 2 or 3. Enforcement mode (validation can quarantine results) is appropriate for Milestone 3. The evaluation harness (§8.4 Milestone 0) should include run manifest generation for gold-set validation.

**Affected sections:** §8.4, §11

---

## 6. Expected Document Changes

### Highest Priority (PRIMARY — substantial edits):

**§1 Executive Summary:**
- In the "Key Recommendations" bullet about skills, add a sentence about run manifests and semantic validation. Something like: "Every skill run produces a human-readable run manifest; validator skills review these manifests to catch semantic errors invisible to code-based checks."
- In "What Will Make or Break the Product" — add that the worker-validator catches the most insidious failure mode: schema-valid garbage from skills that navigated to the wrong page or extracted data from the wrong context.

**§4.10 Skills and Browser Automation:**
- **Skill Types table:** Add category "6. Validator Skills" with entries for base validator and domain-specific overlay skills.
- **Skill Lifecycle:** Add run manifest generation as a step in skill execution (between "output result contract" and "daemon validates result contract"). Note: the agent writes the run manifest BEFORE the result contract is validated.
- **Skill Metadata:** Add `validation_policy` field (always/sample/trigger/skip) and `validator_overlay_path` to the metadata table.
- **Promotion Flow:** Add that promotion from canary → shadow → stable can require validation pass rates meeting thresholds.
- **Production Skill Directory Standard:** Add optional `validator-overlay.md` to the skill directory structure.

**§5.2 High-Level Component Design:**
- **New Component: Run Manifest Writer.** Responsibility: After a skill completes, the agent writes a human-readable MD narrative describing what it did, what it found, and how it interprets its results. Stored as an immutable artifact. The manifest is written BY the agent (it's part of the skill's output), not by the daemon.
- **New Component: Validation Policy Engine.** Responsibility: Determine whether a completed job requires validation based on the skill's `validation_policy` metadata, the job's decision impact, and historical reliability signals. Returns: validate/skip/sample.
- **New Component: Validation Executor.** Responsibility: Spawn validator sub-jobs via the Job Runner. Load the base validator skill + any domain-specific overlay from the worker skill's directory. Inject the run manifest, result contract summary, and OQG signals into the validator's kickoff prompt. Process the verdict.
- **Modify: Job Runner.** Add that after OQG passes, Job Runner consults the Validation Policy Engine. If validation required, it spawns a validator sub-job with `parent_job_id` linking back to the worker job. Job Runner holds the worker job's final stage advancement until the verdict is rendered (or validation policy says skip).
- **Modify: Output Quality Gate.** Clarify that OQG signals (syntactic/semantic/behavioral-drift check results) are packaged as inputs to the validator when validation runs. OQG remains the first gate; the validator is the second.
- **Modify: Artifact Store.** Add run manifests as a stored artifact type alongside raw session output.

**§5.4 Data Model Design:**
- **Add `ValidationVerdict` TS interface:**
  ```ts
  export interface ValidationVerdict {
    id: string;
    workerJobId: string;        // the job being validated
    validatorJobId: string;     // the validator agent session
    validatorSkillPath: string;
    validatorSkillVersion: string;
    verdict: "approved" | "rejected" | "flagged";
    issues: ValidationIssue[];
    summary: string;            // one-paragraph human-readable summary
    runManifestHash: string;    // hash of the manifest that was validated
    oqgSignalsSummary?: string; // OQG inputs provided to validator
    verdictRenderedAt: string;
  }

  export interface ValidationIssue {
    severity: "critical" | "warning" | "info";
    category: string;           // e.g., "wrong_source", "data_contamination", "missing_expected_data"
    description: string;
    suggestion?: string;
  }
  ```
- **Add SQL table `validation_verdicts`** (local-only):
  ```sql
  create table validation_verdicts (
    id uuid primary key,
    worker_job_id uuid not null references workflow_jobs(id),
    validator_job_id uuid not null references workflow_jobs(id),
    validator_skill_path text not null,
    validator_skill_version text,
    verdict text not null,  -- approved, rejected, flagged
    issues_json jsonb not null default '[]',
    summary text not null,
    run_manifest_hash text not null,
    oqg_signals_summary text,
    verdict_rendered_at timestamptz not null,
    created_at timestamptz not null default now()
  );
  ```
- **Add SQL table `validation_verdict_summaries`** (push-owned projection):
  ```sql
  create table validation_verdict_summaries (
    id uuid primary key references validation_verdicts(id),
    worker_job_id uuid not null,
    verdict text not null,
    issue_count integer not null default 0,
    summary text not null,
    verdict_rendered_at timestamptz not null,
    owner_executor_id text not null,
    row_version integer not null default 1,
    last_local_seq integer,
    updated_at timestamptz not null default now(),
    created_at timestamptz not null default now()
  );
  ```
- **Modify `workflow_jobs` interface:** Add `parentJobId?: string` and `validationRequired?: boolean`.
- **Modify `workflow_jobs` SQL:** Add `parent_job_id uuid references workflow_jobs(id)` and `validation_required boolean not null default false`.
- **Modify `agent_session_logs` interface:** Add `runManifestPath?: string` and `runManifestHash?: string`.
- **Modify `agent_session_logs` SQL:** Add `run_manifest_path text` and `run_manifest_hash text`.

**§4.7 Authority Split table:**
- Add `validation_verdicts` to Local-only (no sync) row.
- Add `validation_verdict_summaries` to Push-only (local → remote projections) row.

**§5.6 Ingestion Data Flow:**
- After step 12 (Result contract validated by OQG), insert new steps:
  - 12a. Agent writes run manifest — human-readable MD narrative of what happened during the ingestion run. Stored as immutable artifact.
  - 12b. Validation Policy Engine checks the skill's `validation_policy`. If `skip`, proceed directly to step 13. If `always` or `trigger` (and trigger conditions met), spawn validator sub-job.
  - 12c. Validator agent reads run manifest + OQG signals + result contract summary. Renders verdict.
  - 12d. Verdict processing: `approved` → proceed to step 13. `rejected` → quarantine results as `AGENT_SUSPECT_OUTPUT`, create error manifest with validator's reasoning, alert operator. `flagged` → proceed to step 13 with flag for operator review.
  - Note: In dry-run mode, steps 12b-12d run but verdicts are logged without enforcement — rejected results still advance.
- Add to Error Cases: "Validator rejects output → results quarantined, error manifest created with validator's reasoning and suggested fixes."

**§5.11 Error Handling Strategy:**
- Add to Agent-Specific Error Types table:
  - `VALIDATION_REJECTED` — Validator agent found critical semantic issues in run manifest. Results quarantined. Error manifest created with validator's issues and suggestions.
  - `VALIDATION_FLAGGED` — Validator agent found non-critical issues. Results accepted with flag for operator review.
- Add to Skill Outcome Classes or create new subsection: Validator verdicts are NOT skill outcomes — they are a separate verdict layer. A validator skill itself uses the standard outcome classes (completed/failed/etc). The verdict (approved/rejected/flagged) is the validator's assessment of the WORKER's output, carried in the result contract.
- Clarify: AGENT_SUSPECT_OUTPUT can now be set by either OQG (existing) or validator rejection (new). Both paths create error manifests. The error manifest includes which gate triggered the quarantine.

**§6.1 Complexity Analysis:**
- Add row: "Worker-validator semantic validation | Medium-High | Run manifest generation is straightforward (agent writes narrative); validation policy is config-driven; the hard part is writing good validator skills that catch real issues without false positives"
- Update "Delivery-priority annotation" to note that dry-run validation is Operational (build third) tier.

**§6.2 Risk Assessment:**
- Add risks:
  - "Validator produces false rejections, blocking good data" | Medium | Medium | Dry-run mode first; validator approval/rejection rates monitored; operator override for rejected results
  - "Validator itself fails or is unreliable" | Medium | Low | Validator skills follow same promotion flow; validator failures don't block worker results (fail-open by default)
  - "Run manifest quality too low for meaningful validation" | Medium | Medium | Run manifest is part of the skill contract — skill promotion requires run manifests that validators can meaningfully review
  - "Validation cost overhead per job" | Low | Low | Validator skills use headless_contract mode with cheap models; validation policy can sample instead of always-validate

**§6.5 Testing Strategy:**
- Add Validator Testing subsection:
  - Validator canary tests: Feed known-good and known-bad run manifests to validators, verify correct verdicts
  - Validator golden journeys: Run full worker → validator pipeline on gold-set inputs
  - Dry-run validation tests: Verify that dry-run mode logs verdicts without enforcement
  - False positive monitoring: Track validator rejection rate against operator overrides

**§6.6 Operational Observability:**
- Add to skill-level telemetry table:
  - Validation approval rate | Validation executor | Validator quality — low approval rate signals overly strict validators or degrading worker quality
  - Validation rejection rate | Validation executor | Worker quality issues — rising rejection rate signals worker skill regression
  - Validation flag rate | Validation executor | Borderline case frequency
  - Validator cost per job | Post-run calculation | Validation overhead efficiency
  - Operator override rate (of rejections) | Operator actions | False positive rate — high override rate signals validator needs tuning

**§8.2 Files to Create:**
- Add to Rust Daemon section:
  - `crates/daemon/src/validation/policy.rs` — Validation policy engine: reads skill metadata, evaluates trigger conditions, returns validate/skip/sample
  - `crates/daemon/src/validation/executor.rs` — Validation executor: spawns validator sub-jobs, injects run manifest + OQG signals, processes verdicts
  - `crates/daemon/src/validation/manifest_writer.rs` — Run manifest artifact writer: stores agent-produced MD narratives as compressed immutable artifacts
- Add to Skills section:
  - `skills/validators/base/SKILL.md` — Base validator skill: generic quality checks applicable to all skill outputs
  - `skills/validators/base/result.schema.json` — Validator result contract schema (verdict + issues)
- Add optional files to existing skill directories:
  - Note that any production skill directory MAY include `validator-overlay.md` for domain-specific validation checks

**§8.3 Test Files Required:**
- Add Validator Tests section:
  - `crates/daemon/tests/validation_policy.rs` — Policy engine decision logic
  - `crates/daemon/tests/validation_executor.rs` — Validator sub-job spawning and verdict processing
  - `crates/daemon/tests/validation_dry_run.rs` — Dry-run mode: verdicts logged but not enforced
  - `crates/daemon/tests/validation_golden_verdicts.rs` — Known-good and known-bad run manifests → correct verdicts

**§8.4 Milestone Assignment Map:**
- **Milestone 0 (Gold Set):** Add run manifest generation to evaluation harness skills. The evaluation harness should produce run manifests so validation infrastructure can be developed against real examples.
- **Milestone 2 or 3:** Add validation daemon files (policy.rs, executor.rs, manifest_writer.rs), base validator skill, dry-run mode tests. Validation runs in dry-run mode — verdicts logged but don't block.
- **Milestone 3:** Add enforcement mode. Validator rejections quarantine results. Add golden verdict tests.
- **Milestone 4:** Add `validation_verdict_summaries` to Supabase projection tables.

**§11 Next Steps:**
- In the appropriate milestone description, add: "Worker-validator validation layer — run manifest generation in all skills, validator skills for semantic quality checking, validation policy engine, dry-run mode for initial deployment."

**Appendix E (Risks):**
- Add the same risks listed in §6.2 additions above.

### Secondary (consistency alignment):

**§4.9 (Agent Orchestration):** Add a note that validator agent sessions follow the same lifecycle and adapter-mediated execution as all other sessions. They use `headless_contract` mode exclusively (no browser needed — they read manifests).

**§5.3 (Canonical Workflow):** After the "generate manifest" step, consider adding: "-> validate (run manifest reviewed by validator skill, if policy requires)". Or add a note that validation can occur at any stage where the policy engine requires it.

**§5.5 (Contract State Model):** Consider whether `pipeline_state` needs a `validation_pending` state, or whether validation is handled within the existing stage without a new state. The source doc suggests validation holds the job's final advancement, not a new pipeline state — be consistent with this.

**§5.7 (Intelligence Pipeline Data Flow):** Same validation step insertion pattern as §5.6 — after OQG, check validation policy, optionally spawn validator.

**§5.9 (Request Pipeline Automation):** Note that bid automation skills should default to `validation_policy: always` given their operational consequence (portal form filling, even though not real money).

**ADR-005:** Consider adding a note that validator skills are a new skill category that follows the same boundary rules — the daemon orchestrates validator sessions just like worker sessions; validator skills are passive assets loaded by the daemon.

---

## 7. Integration Rules

### What MUST be true after harmonization:

- No contradictions between any section about how validation works
- No stale terminology — "run manifest" is always the full term, never just "manifest" when this concept is meant
- No implicit assumptions that contradict the source material
- The existing OQG description is NOT weakened — it's augmented by the new validator layer
- The existing skill outcome classes are NOT changed — validator verdicts are a separate concept
- The authority matrix is NOT violated — every new table has exactly one authority domain
- The plug-and-play composability principle is NOT violated — adding validation to a skill requires only adding metadata + optional overlay file, not daemon code changes

### What MUST stay OUT:

- Implementation task planning / bead sequencing
- "First implement X, then Y" language
- Tool-specific dev instructions
- Design iteration notes
- Anything suggesting the worker-validator is a standalone subsystem separate from the main pipeline

### What MUST be ADDED where existing docs are too vague:

- Explicit statement that OQG and semantic validation serve different purposes
- Explicit statement about fail-open vs fail-closed behavior (validator failure should not block worker results by default)
- Explicit statement about dry-run mode as the default initial deployment strategy
- Explicit statement about run manifest format and storage location
- Explicit DB authority classification for new tables

### Style Rules:

- Preserve the existing document's section structure and style
- Integrate into existing sections — do NOT dump as appendices
- If a concept is missing entirely, add a properly placed subsection
- Use the same TS interface + SQL table pattern established in §5.4
- Use the same component description pattern established in §5.2
- Use the same risk table format established in §6.2

---

## 8. What a Great Outcome Looks Like

- An engineer reading only the harmonized research doc gets one coherent architecture model that includes worker-validator as a natural part of the pipeline — not bolted on.
- The docs clearly distinguish between OQG (code-based structural/statistical checks) and semantic validation (agent-based contextual review).
- The docs clearly distinguish between "run manifest" (what a skill run did), "opportunity manifest" (opportunity summary), and "error manifest" (pipeline failure record).
- The run manifest is described using the same artifact storage patterns as raw session output.
- Validator skills follow the same production directory standard as all other skills.
- The validation policy engine is config-driven and respects plug-and-play composability.
- The standalone worker-validator research doc is genuinely redundant — an implementer NEVER needs to read it.

---

## 9. Deliverable

1. Updated `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` with all changes described above.
2. Concise change summary: which sections changed, what was resolved, what remains open.
3. At minimum expect to modify: §1, §4.7, §4.10, §5.2, §5.3, §5.4, §5.5, §5.6, §5.7, §5.11, §6.1, §6.2, §6.5, §6.6, §8.2, §8.3, §8.4, §11, Appendix E, and potentially ADR-005.

---

## 10. Final Verification Checklist

Before stopping, verify:

- [ ] Every modified section was re-read after editing
- [ ] No modified section contradicts the source material
- [ ] No modified section contradicts another section in the target doc
- [ ] No modified section gained implementation sequencing content
- [ ] "Run manifest" is always used as the full term (never just "manifest" when this concept is meant)
- [ ] OQG description is augmented, not weakened or replaced
- [ ] Skill outcome classes are NOT modified — validator verdicts are separate
- [ ] Authority matrix is consistent — new tables classified correctly
- [ ] Plug-and-play composability is preserved — no daemon code changes needed for new validators
- [ ] `workflow_jobs` changes (parent_job_id, validation_required) appear in BOTH the TS interface AND the SQL table
- [ ] `agent_session_logs` changes (run_manifest_path, run_manifest_hash) appear in BOTH the TS interface AND the SQL table
- [ ] New tables appear in the §4.7 authority split table
- [ ] New files appear in §8.2, §8.3, §8.4
- [ ] Milestone assignments are realistic (dry-run first, enforcement later)
- [ ] Appendix E has the new risks

---

## 11. Follow-Up Fresh-Eyes Pass

> **Send this as a follow-up message in the same session after the main harmonization is complete.**

Re-read the source material (`docs/research/2026-03-15-worker-validator-run-manifest-research.md`) and every modified section of the target doc with completely fresh eyes.

**Specific contradiction hunt targets:**

1. Search for every occurrence of "manifest" — verify that "run manifest", "error manifest", and "opportunity manifest" are never confused.
2. Search for every occurrence of "OQG" or "Output Quality Gate" — verify that the new validator doesn't contradict or duplicate OQG's described role.
3. Search for every occurrence of "AGENT_SUSPECT_OUTPUT" — verify that both OQG and validator rejection paths are described as sources of this status.
4. Search for every occurrence of "validation" — verify no pre-existing use of this term conflicts with the new validation layer.
5. Search for every occurrence of "workflow_jobs" — verify parent_job_id and validation_required are present everywhere the table is described.
6. Search for every occurrence of "agent_session_logs" — verify run_manifest_path and run_manifest_hash are present.
7. Search for every occurrence of "promotion" or "promotion flow" — verify that validation pass rates are mentioned as a promotion gate where appropriate.
8. Search for "plug-and-play" or "composab" — verify that the validation design doesn't violate this principle.

**Verify no stale wording remains:**
- The existing text about "three validation layers (syntactic, semantic, behavioral/drift)" in §4.10 should now mention a fourth layer or be updated to reflect the new semantic validator.
- Any text that implies the OQG is the final quality gate should be updated to note the optional validator gate after OQG.

**Verify no implementation planning leaked in.**

**Fix anything that fails these checks.**
