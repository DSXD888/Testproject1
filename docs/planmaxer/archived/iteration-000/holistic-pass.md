# Holistic Pass: Cross-Plan Unification

Run this prompt AFTER all four domain pre-preps have been integrated into the research document. This is a full-context pass to catch cross-cutting inconsistencies and improve architectural coherence.

---

## Step 1: Deep Read

Read the following documents in order, cover to cover:

1. **The full research document** — `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Read every section, every table, every code block
   - Build a mental model of the complete architecture as it now stands post-integration

2. **The four handoff prompts** (for context on what was reviewed):
   - `docs/planmaxer/active/handoffs/handoff-agent-orchestration.md`
   - `docs/planmaxer/active/handoffs/handoff-landed-cost-engine.md`
   - `docs/planmaxer/active/handoffs/handoff-data-architecture.md`
   - `docs/planmaxer/active/handoffs/handoff-procurement-pipeline.md`

3. **Any reviewer responses** in `docs/planmaxer/active/reviews/` (if available)

---

## Step 2: Cross-Plan Improvement

With "fresh eyes," examine the research document for each of the following issues. Fix each with a small, targeted edit — not a rewrite.

### 2.1 Cross-Section Contradictions

Look for places where information added in one domain's integration contradicts another:

- Does the PTY session lifecycle in 4.9 still match the ingestion flow in 5.6?
- Do the data model entities in 5.4 match the TypeScript interfaces earlier in the same section?
- Does the canonical workflow in 5.3 match the intelligence pipeline in 5.7?
- Do the margin gates in 5.3 match Appendix A?
- Does the confidence model in Appendix B account for all the pipeline stages in 5.7?
- Do the error types in 5.11 match the health detection in 6.6?
- Do the file listings in 8.2 match the components described in 5.2?

### 2.2 Terminology Consistency

Check for the same concept called different names:

- "Agent session" vs "PTY session" vs "agent run" — are these used consistently?
- "Skill" vs "skill playbook" vs "SKILL.md" — consistent?
- "Procurement process" vs "opportunity" vs "contract" — clearly differentiated?
- "Source of truth" vs "primary database" vs "local store" — consistent?
- "Sync target" vs "shared database" vs "remote database" — consistent?
- "Error manifest" vs "error log" vs "error entry" — consistent?
- "Control panel" vs "desktop app" vs "desktop shell" — consistent?
- "Viewing portal" vs "web app" vs "web shell" — consistent?

### 2.3 Missing Cross-References

Look for:

- A component in 5.2 that's mentioned but never appears in the data flows (5.6, 5.7, 5.8, 5.9)
- A data model entity that's defined in 5.4 but never referenced in a flow
- An error type in 5.11 that doesn't have a corresponding test in 6.5
- A skill mentioned in 4.10 that doesn't appear in the file listings (8.2)
- An API mentioned in 4.4/4.5/4.6 that doesn't appear in external accounts (9.3)
- A dependency mentioned in text but missing from 9.1

### 2.4 Invariant Violations

Check that these rules are respected everywhere:

- Cost-estimation APIs are ALWAYS API-based, never agent-scraped (sections 4.4, 4.5, 4.6, 5.7)
- Desktop = control panel (read/write/trigger), Web = read-only viewing portal (section 5.8, ADR-008)
- Local SQLite wins for pipeline data, Supabase wins for shared config (section 4.7, ADR-002)
- HMRC rates used for customs duty, Xe for commercial margin (section 4.6, 5.7)
- No real money changes hands automatically (sections 5.9, ADR-007)
- Full terminal output is captured for every agent session (section 6.6)
- Real E2E tests, no mocks/stubs (section 6.5)

### 2.5 Unnecessary Complexity

After all integrations, look for:

- Components that were added but overlap with existing ones
- Error types that are too fine-grained (could be merged)
- Data model fields that are redundant
- Workflow states that can never be reached
- Over-specified confidence models that add false precision
- Sections that grew too long and need restructuring

### 2.6 Missing Invariants

Properties that are assumed but not stated:

- Is there a maximum number of concurrent PTY sessions? Should there be?
- Is there a maximum age for supplier quotes before they're considered stale?
- Is there a maximum number of product candidates per notice?
- Should there be a maximum number of supplier sources searched per product?
- Is there a sync timeout after which the daemon should flag a sync failure?
- Is there a maximum size for terminal output before it's truncated/compressed?

### 2.7 Stale Open Questions

Check section 10 (Open Questions):

- Are any "Requiring User Input" questions now answered by the integrations?
- Are any "Requiring Further Research" questions now resolved?
- Are any assumptions (section 10) now validated or invalidated?
- Should new open questions be added based on the integrations?

### 2.8 Orphaned References

Check:

- All ADRs referenced in text exist in section 7
- All ADRs in section 7 are referenced somewhere in the document
- All appendices referenced in text exist
- All file paths in section 8 are internally consistent

---

## Step 3: Architectural Coherence

Step back and evaluate the whole architecture:

1. **Does every component serve a clear purpose?** Are there components that could be merged or removed?

2. **Are the boundaries between components in the right place?** For example:
   - Is the boundary between "qualification engine" and "product extraction engine" clear?
   - Is the boundary between "daemon cron scheduler" and "job runner" clear?
   - Is the boundary between "sync engine" and "SQLite local store" clear?

3. **Is the dependency direction correct everywhere?** Skills depend on the daemon, not the other way around. The web portal depends on Supabase, not local SQLite.

4. **Are there circular dependencies or unnecessary coupling?** For example, does the landed-cost engine depend on something that depends on the landed-cost engine?

5. **Is complexity allocated to the right places?** The plan identifies PTY orchestration and product matching as the hardest problems. Does the architecture give them proportional design attention?

---

## Step 4: Sync

1. Update the Change Log in Appendix E with a holistic-pass entry.

2. Verify ALL cross-references between sections are valid.

3. Verify the Table of Contents matches the actual section structure.

4. Verify Appendix D (Source References) is complete and organized.

5. Verify section 9.3 (External Accounts) includes all services mentioned in the document.

6. Verify the "Quick Stats" table in section 1 is still accurate (file counts, complexity rating, risk areas).
