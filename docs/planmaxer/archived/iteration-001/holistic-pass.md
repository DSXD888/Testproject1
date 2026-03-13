# Holistic Pass: Cross-Plan Unification (Iteration 001)

This prompt runs AFTER all 4 domain pre-preps have been integrated. It performs a full-context pass across the entire research document to fix cross-cutting inconsistencies and improve architectural coherence.

---

## Step 1: Deep Read

Read the following documents in this exact order. Do not skip any.

1. **AGENTS.md** — project constraints, toolchain, rules
2. **The research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Read the ENTIRE document cover to cover. Every section, every subsection, every ADR, every appendix.
   - This document has been through 2 review cycles (iteration 000: procurement pipeline, data architecture, agent orchestration, landed-cost engine; iteration 001: frontend integration, search intelligence, skill reliability, MVP delivery strategy). Multiple independent reviewers have contributed changes, creating cross-section inconsistency risk.

Build a complete mental model:
- The value proposition (procurement intelligence with landed-cost transparency)
- The full data pipeline (ingestion → normalization → qualification → extraction → matching → pricing → landed cost → manifest)
- The daemon architecture (adapter boundary, PTY + contract mode, session lifecycle, resource arbiter)
- The data architecture (authority matrix, SQLite + Supabase, push/pull sync, stage coherence)
- The skill layer (SKILL.md authoring, metadata, lifecycle, reliability patterns)
- The frontend architecture (Tauri desktop, Next.js web, shared packages, daemon communication)
- The search intelligence (query generation, shortlisting, dual confidence, cross-language matching)
- The delivery strategy (milestones, MVP slice, validation plan, stack sequencing)
- All 8 ADRs and their rationale
- The file reference map (131+ files across milestones)
- The testing, observability, and error handling strategies

---

## Step 2: Cross-Plan Improvement

With "fresh eyes," examine the full document for cross-cutting issues. Look carefully for each of the following categories. For each issue found, make a small, targeted edit — not a wholesale rewrite.

### 2.1 Contradictions Between Sections

Look for places where two sections describe the same concept differently:
- Does §5.8 (control panel data flow) match §5.10 (API design)? Are the same endpoints described consistently?
- Does §4.10 (skills) match §5.6 (ingestion data flow) on how skills are invoked and what they produce?
- Does §4.3 (supplier price intelligence) match §5.7 (intelligence pipeline) on query generation and shortlisting?
- Does §5.4 (data models) match §5.2 (component designs) on field names and types?
- Does §11 (milestones) match §8.2 (file map) on milestone assignments?
- Do ADRs match the sections that implement their decisions?

### 2.2 Inconsistent Terminology

Search for variant names for the same concept:
- "desktop" vs "control panel" vs "Tauri app" — pick one primary term
- "web portal" vs "viewing portal" vs "web shell" — pick one primary term
- "skill" vs "playbook" vs "SKILL.md" — when is each appropriate?
- "search profile" vs "query pack" — are these synonyms or distinct concepts?
- "checkpoint" vs "intermediate result" vs "partial result"
- "match" vs "quote" vs "supplier quote" — clarify the lifecycle stages
- "notice" vs "opportunity" vs "contract" vs "process" — clarify the entity hierarchy
- "daemon API" vs "internal API" vs "localhost API"
- "result contract" vs "output contract" vs "final result"

For each inconsistency found, standardize on one term and update all occurrences. Add a terminology note in §1 or the relevant section if the distinction is meaningful.

### 2.3 Missing Cross-References

Check that sections reference each other properly:
- §5.8 should reference §5.10 for API endpoint details
- §4.10 should reference §5.11 for error taxonomy
- §4.3 should reference §5.4 for data model definitions
- §5.7 should reference §4.3 for source routing rules
- §11 should reference §6.1 and §6.2 for complexity/risk-driven delivery decisions
- §6.5 should reference §4.10 for skill testing patterns
- Component designs in §5.2 should reference the data models they consume/produce in §5.4

### 2.4 Invariant Violations

Check for rules stated in one section that are violated by another:
- Authority matrix (§4.7, ADR-002): Is every table correctly categorized? Does any section imply writing to a table from the wrong authority?
- Adapter boundary (§4.9, ADR-005): Do skills in §4.10 ever assume a specific execution mode?
- Dual confidence separation (§4.3): Is matchConfidence ever averaged with quoteUsabilityConfidence anywhere?
- "No mocks" testing (§6.5): Does any section reference mock-based testing?
- Integer pence for money (§5.4): Does any section use floating-point for monetary values?
- Mixed-write prohibition (ADR-002): Does any section imply both daemon and web write to the same table?

### 2.5 Unnecessary Complexity

After 2 review cycles, some sections may have grown redundant or over-specified:
- Are there concepts explained in multiple places that could be explained once and cross-referenced?
- Are there subsections that repeat information from other subsections?
- Are there overly detailed specifications for components that haven't been validated yet?
- Are there open questions (§10) that have been resolved by the review cycles but not marked as resolved?

### 2.6 Missing Content

After 2 review cycles, some gaps should be visible:
- Are there sections that reference concepts not defined anywhere?
- Are there data flow diagrams that skip steps?
- Are there component designs that don't specify their inputs/outputs?
- Are there ADRs referenced but not defined, or defined but never referenced?
- Are there files in §8.2 that don't correspond to any component or data flow description?

---

## Step 3: Architectural Coherence

Step back from the details and assess the architecture as a whole:

### 3.1 Purpose Clarity
- Does every component serve a clear purpose? Is there anything that exists "just in case"?
- Can you trace every component back to the value proposition (identifying profitable procurement opportunities)?
- Are there components that could be removed without affecting the core value chain?

### 3.2 Boundary Placement
- Are the boundaries between components in the right place?
  - Desktop ↔ Daemon: Is the API surface right-sized? Too many endpoints? Too few?
  - Daemon ↔ Skills: Is the adapter boundary at the right abstraction level?
  - SQLite ↔ Supabase: Is the authority matrix correctly allocated?
  - Pipeline stages: Are the handoff points between stages well-defined?

### 3.3 Dependency Direction
- Do dependencies flow in one direction (no circular dependencies)?
- Does the file map reflect the dependency structure?
- Are shared packages (`packages/ui`, `packages/contracts-schema`) properly isolated from implementation details?

### 3.4 Complexity Allocation
- Is complexity concentrated in the right places (the hard problems: matching, landed cost, daemon orchestration)?
- Are simple problems kept simple (ingestion, UI, sync)?
- Is the testing strategy proportional to complexity (more testing for higher-complexity components)?

### 3.5 Delivery Coherence
- Does §11 (delivery strategy) align with §6.1 (complexity) and §6.2 (risk)?
- Does the MVP slice validate the riskiest assumptions?
- Is the milestone ordering consistent with the dependency graph?

---

## Step 4: Sync

1. **Update §1 (Executive Summary):** Ensure it accurately summarizes the current state of the architecture after 2 review cycles. Quick Stats, Delivery Phases, Key Risks should all reflect the latest content.

2. **Update §10 (Open Questions):** Mark any questions that have been resolved by the review cycles. Add any new questions that emerged from this holistic pass.

3. **Update §8.2 (File Reference Map):** Verify all files are accounted for, milestone assignments are consistent with §11, and no orphaned files exist.

4. **Update ADRs:** Verify each ADR's rationale still matches the architecture. If review cycles added nuance, update the rationale.

5. **Update Appendix E (Changelog):** Add a concise entry for this holistic pass.

6. **Terminology glossary:** If significant terminology inconsistencies were found and fixed in Step 2.2, consider adding a brief terminology section to §1 or §2 to prevent future drift.
