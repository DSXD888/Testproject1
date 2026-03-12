# Pre-Prep: Agent Orchestration & PTY Runtime

## Domain Ownership

This pre-prep owns modifications to these sections of the research document:
- **Section 4.9** — Local Agent Orchestration Runtime
- **Section 4.10** — Skills and Browser Automation
- **Section 5.2** — High-Level Component Design (PTY Session Manager, Daemon Cron Scheduler, Skill-Based Ingestion Runner, Log/Trace Capture Engine components)
- **Section 5.6** — Ingestion Data Flow (daemon/PTY execution steps, error cases)
- **Section 5.11** — Error Handling Strategy (agent-specific error types)
- **Section 6.5** — Testing Strategy (agent E2E tests)
- **Section 6.6** — Operational Observability (terminal output capture, health detection, structured tracing)
- **ADR-004** — PTY-Based Agent Execution
- **ADR-005** — Skills as Data Retrieval + Workflow Assets

**Shared with other domains:** Sections 5.2, 5.4, 6.1, 6.2 may be touched by other domains. If your changes affect data model fields in 5.4, note them but don't restructure the tables — the data-architecture domain owns those.

---

## Step 1: Pre-Prep Research

Before integrating the reviewer's feedback, read and deeply understand:

1. **The full research document** — `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Focus heavily on sections 4.9, 4.10, 5.2, 5.6, 5.11, 6.5, 6.6
   - Understand the PTY session lifecycle, health detection matrix, and structured trace model
   - Understand the skill lifecycle and skill design principles
   - Understand the current error type taxonomy (AGENT_STALLED, AGENT_SESSION_EXPIRED, AGENT_INTERNAL_ERROR, AGENT_SKILL_FAILED, AGENT_RUN_FAILED)

2. **The handoff prompt** — `docs/planmaxer/active/handoffs/handoff-agent-orchestration.md`
   - Understand the 5 tensions that were posed to the reviewer
   - Note the anti-patterns (what NOT to change) and the user's strong beliefs

3. **Pay attention to:**
   - How the daemon-agent interface is currently specified (what's concrete vs. hand-waved)
   - The relationship between skills, PTY sessions, and job state
   - The testing strategy for agent E2E tests
   - Any places where the document contradicts itself or leaves gaps

---

## Step 2: Feedback Integration

<INSERT REVIEWER RESPONSE HERE>

### Integration Rules

For each proposed change from the reviewer:

1. **Agree strongly** — Integrate directly into the research document. Edit the relevant section with the improvement.

2. **Partially agree** — Integrate the core insight but adapt framing to match the document's existing conventions. Add nuance where the reviewer's suggestion needs context.

3. **Disagree** — Do NOT integrate. Note why in a brief comment at the end of this step (but don't add the comment to the research doc).

### Integration Checklist

For each change you make:
- [ ] Preserve existing ADR numbers and IDs (ADR-004, ADR-005)
- [ ] Use the document's existing style (same heading levels, same table formats, same code block conventions)
- [ ] If adding new error types, follow the existing `AGENT_*` naming pattern
- [ ] If modifying the PTY session lifecycle, update ALL places it's referenced (5.2, 5.6, 6.6)
- [ ] If adding new components, add them to section 5.2 following the existing component template
- [ ] If modifying test strategy, keep the priority ordering (Agent E2E > Integration > Unit > Browser E2E)
- [ ] Update the Change Log in Appendix E with a summary of this integration

### What You Can Edit Directly
- Sections 4.9, 4.10, 5.6, 5.11, 6.5, 6.6 — full ownership
- Section 5.2 components: PTY Session Manager, Daemon Cron Scheduler, Skill-Based Ingestion Runner, Log/Trace Capture Engine — full ownership
- ADR-004, ADR-005 — can add "Updated" status and add context
- Section 8.2 (Files to Create) — Rust Daemon and Skills sections only
- Section 8.3 (Test Files) — Agent E2E and Integration test sections

### What Requires Coordination
- Section 5.4 (Data Model) — note proposed changes but don't restructure tables
- Section 5.2 (other components) — don't modify components owned by other domains
- Section 6.1, 6.2 (Complexity/Risk) — can add risk items but don't remove existing ones

---

## Step 3: Fresh-Eyes Optimization

After integrating feedback, re-read the sections you modified with fresh eyes:

1. **Consistency check:** Does the PTY lifecycle described in 4.9 match 5.2 match 5.6 match 6.6? Are there any contradictions?

2. **Terminology check:** Are PTY-related terms used consistently? (session, master/slave pair, terminal output, structured trace, etc.)

3. **Completeness check:** Does every error type in 5.11 have a detection method in 6.6 and a test case in 6.5?

4. **Cross-domain check:** Do your changes imply data model changes that section 5.4 needs to reflect? Note these for the holistic pass.

5. **Feasibility check:** Are the agent E2E test examples in 6.5 still realistic given any architecture changes?

---

## Step 4: Sync Pass

1. Update the Change Log in Appendix E with a dated entry summarizing all changes from this integration.

2. If you added new files to section 8.2 or 8.3, verify they follow the existing naming patterns.

3. If you modified ADRs, ensure the Status field and rationale are updated.

4. Check that the Table of Contents references still point to valid section headers.
