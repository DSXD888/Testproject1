# L1 Daemon Core — Validate Existing Spec for 3-SOT Pipeline

## Your Mission

The spec for Layer 1 (Thin Local Runner) already exists at `docs/strategy/layers/layer-1-thin-local-runner.md` (~1,590 lines). Before it can enter the TDD pipeline, verify it has the structure needed for downstream coverage verification — specifically, that a TDD author could mechanically map every requirement to test IDs, and an implementation agent could know exactly what to build from the spec alone.

**This is NOT research-to-spec.** The spec content is comprehensive and deeply reviewed through 3 planmaxer iterations. You are checking STRUCTURE for pipeline readiness, not rewriting content.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## The Spec to Validate

```
docs/strategy/layers/layer-1-thin-local-runner.md   — READ FULLY (~1,590 lines)
```

### Supporting Context (SKIM for cross-references)

```
docs/strategy/layer-strategy.md                      — Cross-layer contracts, §9 schema rules, §10 interface contracts, §11 testing
docs/strategy/layers/layer-0-gold-set-evaluation.md  — L0 dependency: DDL (§2), skill standard (§4), pipeline stages (§3)
docs/strategy/cross-reference-index.md               — Quick topic lookup across all layer docs
```

---

## Validation Checklist

### 1. Requirement IDs

The Layer 1 doc uses `§N.N` section numbering but does NOT have formal `FR-X.Y.Z` requirement IDs. For TDD coverage verification, every testable requirement needs a stable, greppable ID.

- [ ] Add systematic requirement IDs to every testable capability/behavior
- [ ] Use format: `L1-FR-{subsystem}.{seq}` (e.g., `L1-FR-PTY.1`, `L1-FR-ADAPTER.3`, `L1-FR-RUNNER.5`)
- [ ] Subsystem prefixes should map to the 17-file crate structure:
  - `PTY` — pty/session.rs, pty/capture.rs, pty/health.rs
  - `ADAPTER` — agents/adapter.rs (trait), agents/codex.rs, agents/claude.rs
  - `ARTIFACTS` — agents/artifacts.rs, agents/contracts.rs
  - `RUNNER` — jobs/runner.rs
  - `PROMPT` — prompt/renderer.rs
  - `ERRORS` — errors/manifest.rs, errors/markdown.rs
  - `IPC` — ipc/descriptor.rs, ipc/handshake.rs
  - `REAPER` — sessions/reaper.rs
  - `DB` — db/sqlite.rs
  - `DAEMON` — main.rs (bootstrap, CLI, signal handling)
  - `SKILLS` — skill linting, FTS ingestion, manifest generation
  - `PIPELINE` — stage tracking, ingestion flow, commit semantics
- [ ] Verify IDs are systematic (by section/subsystem, not random)
- [ ] A downstream tool could grep for all `L1-FR-` IDs mechanically

### 2. Acceptance Criteria

- [ ] Every requirement has explicit, bullet-listed acceptance criteria
- [ ] ACs are specific and testable (not "handle appropriately")
- [ ] ACs are not buried in narrative prose — extract into bullet lists
- [ ] Focus areas that likely need AC extraction:
  - §1.6 Ingestion Flow (14 steps — each step needs explicit success/failure criteria)
  - §1.7 Adapter-Mediated Hybrid Execution (mode selection matrix needs AC per row)
  - §1.8 Process-Group Supervision (cleanup sequence needs per-step AC)
  - §2.3 Agent Adapter Trait (trait methods need behavioral AC beyond the type signatures)
  - §2.4 Session Lifecycle (7 phases — each transition needs explicit criteria)
  - §2.5 Artifact Store (chunked capture rules need AC)
  - §2.6 Job Runner (commit fence, stage_run_id scoping need AC)
  - §4.2 Run Manifest System (agent_claimed vs daemon_observed fields need validation AC)
  - §4.3 Error Taxonomy (each error type needs trigger condition + expected action as AC)

### 3. Structure

- [ ] Consistent section numbering (the doc uses §N.N format — verify no gaps)
- [ ] Clear subsystem boundaries (the 6 major sections map well, verify completeness)
- [ ] Success criteria table exists (§5.1 Definition of Done — 13 items, verify each is testable)
- [ ] No TBD sections or hand-waving ("handle appropriately" → must be specific)
- [ ] Verify the Rust code examples in §2.3 (trait definitions) are structurally consistent with the prose descriptions

### 4. Completeness

- [ ] Error paths defined for EVERY happy path:
  - PTY creation failure → what happens?
  - `setsid()` failure → what happens?
  - SQLite migration failure at startup → what happens?
  - Agent CLI not on PATH → what happens?
  - All 12 error types in §1.9 have trigger + action + manifest behavior
- [ ] Edge cases from §5.3 are traceable to requirements (each edge case should reference an `L1-FR-*` ID)
- [ ] Integration points specified:
  - L0 skill directory standard → L1 skill linting gate
  - L0 TypeScript pipeline → L1 daemon invocation
  - L0 SQLite schema → L1 migration compatibility
  - L1 IPC socket → L2 desktop bridge (forward reference)
- [ ] Stubs documented (§1.10, §6.2) — each stub references the target layer

### 5. Cross-Layer Consistency

- [ ] Verify §3.3 Stage Tracking Model is consistent with `layer-strategy.md` §10 Stage Scope Matrix
- [ ] Verify §2.2 Rust Crate Dependencies matches what's needed for the described functionality
- [ ] Verify the 10 test files in §5.2 cover all 13 Definition of Done criteria
- [ ] Verify §4.3 Error Taxonomy base types are consistent with `layer-strategy.md` error classification

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)

1. `docs/strategy/layers/layer-1-thin-local-runner.md` — **READ FULLY** (the validation target)
2. `docs/strategy/layer-strategy.md` §9-§11 — **SKIM** (cross-layer contracts for consistency checks)
3. `docs/strategy/layers/layer-0-gold-set-evaluation.md` §2, §4 — **REFERENCE** (L0 DDL and skill standard)
4. `docs/strategy/cross-reference-index.md` — **REFERENCE** (topic lookup)

### Step 1: Add Requirement IDs

Systematically walk each section and add `L1-FR-{subsystem}.{seq}` IDs to every testable behavior. The IDs should be inline markers, not a separate table — they must live next to the requirement they identify.

**Target count:** The spec describes ~17 daemon files + 2 skills + 1 migration + 10 test files + 13 DoD criteria + 12 error types + 7 session phases + 14 ingestion steps. Expect 80-120 requirement IDs.

### Step 2: Extract Acceptance Criteria

For each requirement ID, ensure there are explicit bullet-listed acceptance criteria. Convert narrative descriptions into testable bullets.

Example transformation:
```
BEFORE (narrative):
"The daemon creates PTY pairs, spawns in own session, records identifiers."

AFTER (structured):
**L1-FR-PTY.1** — PTY Session Creation
- AC1: PTY master/slave pair created via openpty() or equivalent
- AC2: Child process calls setsid() to create new session
- AC3: Controlling terminal established via login_tty() or TIOCSCTTY
- AC4: Master fd marked O_CLOEXEC before child exec
- AC5: sid, pid, pgid recorded in session record
- AC6: Default terminal geometry 120x40 set at creation
```

### Step 3: Verify Completeness

Walk the validation checklist above. For each unchecked item, either:
- Add the missing content, OR
- Add a clear note explaining why it's intentionally absent

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete.

Re-read the spec with fresh eyes and ULTRATHINK about these verification questions:

1. Open `docs/strategy/layers/layer-1-thin-local-runner.md` §2.3 — the `AgentAdapter` trait has 4 methods. Are there any implicit behaviors NOT captured as requirements? (e.g., thread safety, cancellation, timeout behavior)
2. Open §2.4 Session Lifecycle — 7 phases with a transition table. Is every transition's failure mode specified? What happens on `spawned -> ??? -> exited` (skip `active`)?
3. Open §2.6 Job Runner — the `stage_run_id` scoping rules describe set-replacement vs additive. Are the boundary conditions explicit? What if a set-replacement stage partially succeeds?
4. Open §4.2 Run Manifest — agent_claimed vs daemon_observed fields. Is there a requirement for what happens when they DISAGREE?
5. Count the error types in §1.9 and §4.3 — are they the same list or have some been added in one but not the other?
6. The ingestion flow (§1.6) has 14 steps — does each step specify what database writes happen? Map each step to the table it writes.
7. The IPC handshake (§2.8) specifies hello/welcome fields — is there a requirement for what happens when `expectedSchemaEpoch` doesn't match `schemaEpoch`?

---

## Key Reminders

- This is structure validation, not content rewrite — the planmaxer-reviewed content is authoritative
- Requirement IDs must be greppable (`L1-FR-*`) for mechanical TDD coverage verification
- Every AC must be testable by a test engineer who hasn't read the research doc
- The spec's authority statement says "An implementation agent should be able to build this layer from this document alone" — validate that this is true after your changes
- Do NOT remove existing content — only add structure (IDs, ACs, missing error paths)
- The 17-file crate structure is load-bearing — requirement IDs must map to it
