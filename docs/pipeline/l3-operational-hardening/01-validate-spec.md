# L3 Operational Hardening — Validate Existing Spec for 3-SOT Pipeline

## Your Mission

The spec for Layer 3 (Operational Hardening) already exists at `docs/strategy/layers/layer-3-operational-hardening.md` (1,150 lines). Before it can enter the TDD pipeline, verify it has the structure needed for downstream coverage verification — requirement IDs, testable acceptance criteria, and no ambiguous "TBD" sections.

**This is NOT research-to-spec.** The spec content is good — you're checking STRUCTURE, not rewriting content.

**Context:** This is a UK public procurement intelligence platform. Layer 3 adds cron scheduling, resource management, PTY hardening, relation resolution, source health monitoring, skill promotion, and the worker-validator system in dry-run mode to the existing Rust daemon. The system must run unattended for 5 consecutive days with >=95% skill success rate.

---

## The Spec to Validate

```
docs/strategy/layers/layer-3-operational-hardening.md    — READ FULLY (1,150 lines)
```

### Supporting Context (SKIM for conventions)

```
docs/strategy/layer-strategy.md                          — Cross-layer strategy — SKIM §5 (L3 section), §9-§13
docs/strategy/cross-reference-index.md                   — Topic lookup — REFERENCE as needed
docs/strategy/layers/layer-1-thin-local-runner.md         — Prior layer spec — SKIM §1-§2 for structure/convention reference
docs/strategy/layers/layer-2-single-operator-desktop.md   — Prior layer spec — SKIM §1 for structure reference
```

### Quality Bar (MATCH THIS STRUCTURE)

```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md — Best existing TDD — STUDY §1-§6 for how requirements map to tests
```

---

## Key Context From Codebase Research

- The spec has **6 major sections**: §1 Overview & Daemon Additions, §2 Worker-Validator System, §3 Skill Promotion/Source Health/Relations, §4 Schema/Telemetry/Observability, §5 Testing & Acceptance, §6 Open Questions/Forward Compatibility/Dependencies
- It defines **12 new Rust daemon modules**, **5 new DB tables** with full DDL, **10 new skills**, **3 new TS modules**, and **18 new test files**
- The Definition of Done (§5.1) has **14 criteria** — these are the closest thing to requirement IDs currently
- Test file listing (§5.2) specifies **18 new test files** organized into 6 categories (fault injection, replay, scheduling, skill lifecycle, validation, relations)
- Test scenarios (§5.3) has **14 scenarios** with expected behaviors
- There are **8 risks** in §5.4 and **3 open questions** in §6.1
- The existing daemon crate has **25 Rust source files** with types like `JobRunner`, `SessionReaper`, `PtySession`, `ArtifactStore`, `ErrorCode` (29 variants), `OutcomeClass`, `ValidationOutcome`
- The L1 TDD (quality bar) has **174 test IDs** mapped to **138 requirement IDs** — L3 needs equivalent traceability

---

## Validation Checklist

### 1. Requirement IDs
- [ ] Every functional requirement has a unique, stable ID (e.g., `L3-FR-1.1`, `L3-FR-2.3.1`)
- [ ] IDs are systematic — by section/subsystem (§1 → L3-FR-1.x, §2 → L3-FR-2.x, etc.)
- [ ] A downstream tool could grep for all requirement IDs mechanically
- [ ] Each of the 12 new daemon components has at least one requirement ID
- [ ] Each of the 5 new DB tables has a requirement ID for schema correctness
- [ ] The worker-validator system (§2) has granular requirement IDs covering: run manifests, policy engine, executor, verdict handling, outcome classes, dry-run mode, fail-open semantics, base validator skill, calibration
- [ ] Resource arbiter quota types (§1.4) each have a requirement ID
- [ ] Skill promotion stages (§3.1) each have a requirement ID
- [ ] Source circuit breaker states (§3.5) have requirement IDs
- [ ] Relation resolution classifications (§3.6) each have a requirement ID
- If missing: ADD IDs systematically without changing content. Follow the pattern `L3-FR-{section}.{subsection}.{item}`

### 2. Acceptance Criteria
- [ ] Every requirement has explicit, bullet-listed acceptance criteria
- [ ] ACs are specific and testable — not "handle appropriately" or "works correctly"
- [ ] The Definition of Done (§5.1) criteria are linked to requirement IDs
- [ ] Each test scenario (§5.3) traces to at least one requirement ID
- [ ] Telemetry metrics (§4.5, §4.6) have concrete threshold values (most already do)
- [ ] Circuit breaker trigger criteria have concrete numbers (N=3 consecutive failures, default 2h cooldown — verify these are in ACs)
- [ ] Validation policy modes (always/sample/trigger/skip) each have testable behavior
- [ ] Risk-stratified sampling factors (§2.3) each have testable conditions
- If buried in prose: EXTRACT into bullet lists under each requirement

### 3. Structure
- [ ] Consistent section numbering — no gaps, no duplicates
- [ ] Clear subsystem boundaries — each module in §1.3 maps to a section
- [ ] The 14 DoD criteria map to identifiable spec sections
- [ ] DDL in §4 is syntactically valid (Postgres + SQLite type substitution notes)
- [ ] Cross-domain notes (HTML comments at end of doc) are still valid
- [ ] Forward-compatibility provisions (§6.3) reference correct Layer 4 features
- [ ] Dependencies (§6.4) list correct crate additions (`cron`, `sysinfo`)

### 4. Completeness
- [ ] Error paths defined — not just happy paths. Check:
  - What happens when cron fires but resource arbiter denies admission?
  - What happens when a validator sub-job itself fails?
  - What happens when relation resolution encounters contradictory evidence?
  - What happens when source-health canary times out (vs fails vs succeeds)?
- [ ] Edge cases called out — verify:
  - Concurrent job claiming race conditions
  - PTY EIO during active extraction (not just after slave closes)
  - Host sleep during validation sub-job
  - Late merge where both processes have in-flight jobs
- [ ] Integration points specified — verify:
  - Cron scheduler → Job Runner → Resource Arbiter flow is clear
  - Source health → Circuit breaker → Scheduler suppression chain is explicit
  - Run manifest writer → Artifact store → Validation executor pipeline is defined
  - Validation executor → Job Runner (sub-job) → Verdict → Job Runner (verdict handling) loop is clear
- [ ] No "to be determined later" items — §6.1 open questions are acceptable IF they don't block implementation
- [ ] All 10 new skills have sufficient specification for implementation (directory structure, purpose, interaction profile)

---

## Step-by-Step Workflow

### Step 0: Read the Spec (MANDATORY)
Read `docs/strategy/layers/layer-3-operational-hardening.md` fully. Take notes on sections that lack explicit requirement IDs or testable ACs.

### Step 1: Scan for Requirement ID Coverage
Walk through every subsection. For each functional behavior described, check if there's an explicit ID. Create a mapping table: `Section → Requirement IDs → Count`.

### Step 2: Add Missing Requirement IDs
Where IDs are missing, add them following the pattern `L3-FR-{section}.{subsection}.{item}`. Do NOT change any spec content — only add IDs.

**Expected ID ranges:**
- §1.3-1.7: Component requirements (L3-FR-1.x) — ~15-20 IDs for 12 daemon components + host awareness
- §2.1-2.9: Worker-validator (L3-FR-2.x) — ~25-35 IDs for manifests, policy, executor, verdicts, dry-run, fail-open, base validator, calibration
- §3.1-3.8: Skill promotion, source health, relations (L3-FR-3.x) — ~20-25 IDs
- §4.1-4.6: Schema, telemetry, observability (L3-FR-4.x) — ~10-15 IDs
- §5: Testing (no FR IDs needed — these are verification, not requirements)
- §6: Open questions (no FR IDs needed)

### Step 3: Extract Buried Acceptance Criteria
For each requirement, ensure ACs are bullet-listed and testable. Convert narrative descriptions into explicit criteria where needed.

### Step 4: Verify Completeness
Walk the completeness checklist above. For each item, either confirm it's addressed or add a note about what's missing.

### Step 5: Verify Structure
Run through the structure checklist. Fix numbering gaps, ensure cross-references are valid.

---

## Output

The same spec file at `docs/strategy/layers/layer-3-operational-hardening.md`, validated and structured. The file should gain:
- Systematic requirement IDs (L3-FR-x.y.z)
- Extracted acceptance criteria bullets where they were buried in prose
- No content changes — only structural additions

---

## Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main validation work is complete.

ULTRATHINK. Re-read the validated spec with fresh eyes. Specifically:

1. **Open `crates/daemon/src/agents/artifacts.rs`** — does every field in `RunManifestFrontmatter` and `DaemonEvidenceAppendix` have a corresponding requirement in §2.1-2.2? Are there fields in the code that the spec doesn't mention, or vice versa?

2. **Open `crates/daemon/src/errors/manifest.rs`** — the spec defines new error codes in §3.4. Does the `ErrorCode` enum (29 variants currently) already include them? Does the spec account for ALL existing error codes or only new ones?

3. **Open `crates/daemon/src/jobs/runner.rs`** — does the spec's §1.5 cron scheduler and §2.4 validation executor account for the existing `EnqueueJobRequest` fields (`parent_job_id`, `continuation_chain_id`, `validation_required`)? Are the interaction patterns with `JobRunner::claim_next_job` explicitly specified?

4. **Open `crates/daemon/src/agents/contracts.rs`** — does §2.4's verdict handling integrate with the existing `ValidationOutcome` and `ValidationDisposition` types? Is there a clear mapping from validator verdicts to existing contract validation outcomes?

5. **Count**: Would a test engineer reading this spec know EXACTLY how many tests to write and what each one checks? Can you produce a requirement count and estimate the test count? Compare against §5.2's claim of 18 new test files.

6. **Open `crates/daemon/src/pty/session.rs`** — does §1.6 PTY hardening account for the existing `SessionPhase` lifecycle (Created → Spawned → Active → Terminating → Exited → Reaped → Archived)? Does the EIO handling integrate with the existing `PtySessionError` variants?

7. **Verify forward references**: §6.3 claims 9 forward-compatibility provisions for Layer 4. Cross-check each against `docs/strategy/layers/layer-4-shared-visibility.md` §1-§3 (skim) — do the stubs match what L4 expects to consume?

Report any gaps found and fix them in the spec.

---

## Key Reminders

- This spec is the **single source of truth (SOT #1)** — downstream TDD and strategy will be generated from it
- Every requirement that lacks an ID will be invisible to the TDD coverage matrix
- Every AC that's buried in prose will produce vague tests
- The L1 TDD has 174 tests for 138 requirements — L3 should aim for comparable density
- Do NOT change architectural decisions or domain content — structural validation only
- The cross-domain notes (HTML comments at EOF) are communication to other layer docs — preserve them
- The existing 25 daemon files and their types are the integration surface — the spec must account for them
