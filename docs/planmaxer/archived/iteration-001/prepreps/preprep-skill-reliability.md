# Pre-Prep: Skill Authoring & Agent Reliability

## Step 1: Pre-Prep Research

Read the following documents in this exact order. Do not skip any.

1. **AGENTS.md** — project constraints, toolchain, rules
2. **The research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Read the ENTIRE document, but pay special attention to:
     - §4.10 (Skills and Browser Automation — skill types, metadata, lifecycle, design principles)
     - §4.9 (Agent Orchestration — adapter boundary, PTY vs contract mode, session lifecycle)
     - §5.2 (Component Designs — daemon components, skill runner)
     - §5.6 (Ingestion Data Flow — scheduled ingestion with skill execution)
     - §5.7 (Intelligence Pipeline Data Flow — skills used in the pipeline)
     - §5.11 (Error Handling — error taxonomy, error manifest system)
     - §6.5 (Testing Strategy — Agent E2E tests, adapter contract tests)
     - §6.6 (Observability — three-layer model, session forensics)
     - §8.2 (File Reference Map — skills/ directory, daemon source files, test files)
3. **The handoff prompt that produced the feedback:** `docs/planmaxer/active/handoffs/handoff-skill-reliability.md`

After reading, you should be able to explain:
- The skill metadata declaration format (interaction_profile, result_schema_version, auth_requirements, etc.)
- The adapter boundary between PTY and contract mode execution
- How skills emit checkpoints and final result contracts
- The error taxonomy (AGENT_*, DAEMON_*, NETWORK_*, etc.)
- The session lifecycle (7 phases: create → bind → launch → monitor → collect → reap → archive)
- The three-layer observability model (control events, protocol events, forensic bytes)
- How per-session browser profiles work
- The testing strategy: real E2E agent tests, adapter contract tests, deterministic replay

**File Ownership:** This pre-prep owns the following sections for editing:
- §4.10 (Skills and Browser Automation) — FULL ownership
- §5.11 (Error Handling) — Skill-related error patterns and recovery ONLY
- §6.5 (Testing Strategy) — Agent E2E and skill testing subsections ONLY
- §6.6 (Observability) — Skill-level telemetry and session forensics ONLY
- §8.2 — skills/ directory file map ONLY
- §8.3 — Skill-related dependency notes ONLY

Do NOT edit: §4.9 (daemon orchestration internals), §5.2 (daemon component designs), §5.6 (ingestion flow orchestration), §4.7 (database), sync protocol. The daemon's session lifecycle and adapter model are settled — skills must work within these constraints, not modify them. If the reviewer suggests changes to daemon internals, note them as deferred.

---

## Step 2: Feedback Integration

<INSERT REVIEWER RESPONSE HERE>

For each change proposed by the reviewer, evaluate:

### Integration Rules

1. **For skill authoring patterns/guide:** Integrate into §4.10. The current section defines skill metadata and types but lacks authoring guidance. Add a new subsection (e.g., "§4.10.X Skill Authoring Patterns") covering: navigation resilience strategies, extraction validation patterns, checkpoint emission format, error communication patterns. This is new content filling a real gap.

2. **For browser automation reliability patterns:** Integrate into §4.10. Add a subsection on browser-specific reliability: anti-bot handling, auth persistence across sessions, dynamic content loading strategies, geo-restriction considerations. The plan currently has per-session browser profiles but doesn't address these operational concerns.

3. **For skill testing strategy:** Integrate into §6.5. The current section has "Agent E2E Tests" and "Adapter Contract Tests" but lacks skill-specific testing patterns. Add: canary run protocol, golden dataset validation, regression detection (garbage output vs clean failure), skill promotion flow. Stay within the "no mocks" constraint.

4. **For partial failure and recovery design:** Integrate into §5.11 and §4.10. Add concrete checkpoint format specification, granularity recommendations per skill type, and daemon-side resume logic. The current plan has AGENT_PARTIAL_FAILURE as a status but doesn't specify checkpoint mechanics.

5. **For skill versioning and migration:** Integrate into §4.10. Add a subsection on version management: promotion flow, schema migration handling, rollback strategy, cache invalidation on skill update. The current plan mentions version recording but not the operational lifecycle.

6. **For cost efficiency patterns at skill level:** Integrate into §4.10. Add specific patterns: session duration minimization, token usage optimization, batching strategies, when to use cheap vs expensive models per skill type.

7. **For skill composition (sub-skills) or skill observability:** Integrate into §4.10 and §6.6 respectively. These are likely new concepts — add subsections if the reviewer provides concrete designs.

8. **For changes touching daemon session lifecycle, adapter boundary, or orchestration:** Do NOT integrate. Add `<!-- CROSS-DOMAIN: Reviewer suggested [X] for daemon [component]. Needs coordination with agent-orchestration domain. -->` notes.

### Evaluation Framework

For each proposed change, assess:
- **Agree strongly:** Fills a concrete gap in skill reliability or operational lifecycle. Apply directly.
- **Agree partially:** Good pattern but may add complexity to SKILL.md authoring (which should stay simple). Apply with simplification, noting tradeoffs.
- **Disagree:** Conflicts with settled decisions (skills are SKILL.md files not a DSL, no mocking, agent-based browser automation not traditional scrapers). Note disagreement with rationale.

### Preservation Rules
- Preserve SKILL.md as the authoring format (no custom DSL or framework)
- Preserve the adapter boundary (PTY vs contract mode — skills don't choose, the daemon binds)
- Preserve the "no mocks" testing principle
- Preserve the three-layer observability model
- Preserve per-session browser profile isolation
- Preserve the error taxonomy structure (AGENT_*, DAEMON_*, etc.)
- Preserve skill metadata declarations (interaction_profile, result_schema_version, etc.)
- Use "as-if-always" language

---

## Step 3: Fresh-Eyes Optimization

After completing Step 2, re-read the sections you edited with fresh eyes. Check for:

1. **§4.10 ↔ §4.9 consistency:** Do skill authoring patterns respect the adapter boundary defined in §4.9? Do checkpoint emission patterns align with the session lifecycle phases? Skills should not assume which execution mode they're in — the adapter handles that.

2. **§4.10 ↔ §5.11 consistency:** Do the error communication patterns in skills align with the error taxonomy in §5.11? If new error types were added for skill-level failures, are they properly categorized?

3. **§6.5 ↔ §4.10 consistency:** Does the testing strategy reference the correct skill types and metadata? If canary runs or golden datasets were added, are they referenced in the skill lifecycle?

4. **§6.6 ↔ §4.10 consistency:** If skill-level telemetry was added, does it feed into the three-layer observability model correctly? Control events vs protocol events vs forensic bytes — which layer do skill metrics belong to?

5. **§8.2 file map consistency:** If the reviewer proposed new skill files, templates, or test fixtures, add them to §8.2. Verify all skill files referenced in §4.10 appear in the file map.

6. **Checkpoint format ↔ result contract consistency:** If a concrete checkpoint format was defined, verify it's compatible with the result contract validation described in §5.2.

7. **Terminology:** Ensure consistent use of "skill" vs "playbook", "checkpoint" vs "intermediate result", "session" vs "run" throughout all edited sections.

Fix any inconsistencies found. Make targeted edits, not wholesale rewrites.

---

## Step 4: Sync Pass

1. **Update §1 (Executive Summary):** If skill reliability patterns represent a significant architectural addition (authoring guide, testing strategy, versioning lifecycle), briefly note the maturity of the skill layer in the executive summary.

2. **Update §6.1 (Complexity Analysis):** If the reviewer's feedback changes the complexity assessment for "Skill maintenance" (currently rated "Medium"), update it with rationale.

3. **Update §6.2 (Risk Assessment):** If new skill-related risks were identified (e.g., "skill produces garbage output without failing cleanly", "anti-bot detection blocks agent sessions"), add them with likelihood/impact ratings.

4. **Update §11 (Next Steps):** If the reviewer identified skill authoring prerequisites (e.g., "authoring guide must exist before first skill is written", "golden dataset needed before production deployment"), add them to relevant milestones.

5. **Update Appendix D (Source References):** If the reviewer referenced specific real-world systems, tools, or patterns for skill reliability, add them.

6. **Appendix E (Changelog):** Add a concise entry describing what was integrated in this review cycle.
