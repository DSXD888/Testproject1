# Pre-Prep: MVP Delivery Strategy & Vertical Slicing

## Step 1: Pre-Prep Research

Read the following documents in this exact order. Do not skip any.

1. **AGENTS.md** — project constraints, toolchain, rules
2. **The research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Read the ENTIRE document, but pay special attention to:
     - §1 (Executive Summary — Quick Stats, Delivery Phases, what will make or break the product)
     - §6.1 (Complexity Analysis — all 15 components rated)
     - §6.2 (Risk Assessment — all risks with likelihood/impact)
     - §8.2 (File Reference Map — all 131 planned files across 6 milestones)
     - §11 (Next Steps — milestones, prerequisites, and delivery order)
     - §4.9 (Agent Orchestration — daemon complexity)
     - §4.2 (Procurement Intelligence Pipeline — the value chain)
     - §4.3 (Supplier Price Intelligence — the hardest matching problem)
     - §4.4–4.6 (Shipping, Customs, FX — landed cost components)
     - §5.7 (Intelligence Pipeline Data Flow — Pipeline Cost Management)
     - §6.5 (Testing Strategy — real E2E agent tests, cost per test)
3. **The handoff prompt that produced the feedback:** `docs/planmaxer/active/handoffs/handoff-mvp-delivery.md`

After reading, you should be able to explain:
- The 6 current milestones and their prerequisites
- The 131 planned files and which milestone each belongs to
- The complexity and risk landscape (which components are Very High complexity, which risks are Critical impact)
- The technology stack split (Rust daemon, TypeScript desktop/web/packages, Skills)
- The testing cost model (real E2E agent tests, $0.20-$0.50 per test run)
- The pipeline cost target ($20-60/day in agent compute)
- What exists today (zero application code)
- The core product thesis (identify procurement opportunities where Chinese sourcing creates profitable arbitrage)

**File Ownership:** This pre-prep owns the following sections for editing:
- §1 (Executive Summary) — Delivery-related content (Quick Stats, Delivery Phases) ONLY
- §6.1 (Complexity Analysis) — FULL ownership (may reorder or add delivery-relevant annotations)
- §6.2 (Risk Assessment) — Delivery-risk entries ONLY (not technical risk details)
- §8.2 (File Reference Map) — Milestone assignments and file groupings ONLY (not file descriptions)
- §11 (Next Steps) — FULL ownership (milestones, prerequisites, delivery order, validation experiments)

Do NOT edit: §4.x (technical architecture sections), §5.x (component designs and data flows), ADRs, §6.3-§6.6, §8.3, §9.x, §10, Appendices. The delivery strategy works WITH the architecture, not against it. If the reviewer suggests architectural changes (e.g., "simplify the daemon for MVP"), note the suggestion in §11 as a delivery-phase scoping decision rather than modifying the architectural spec.

---

## Step 2: Feedback Integration

<INSERT REVIEWER RESPONSE HERE>

For each change proposed by the reviewer, evaluate:

### Integration Rules

1. **For risk-ordered validation plan:** Integrate into §11. Replace or restructure the current milestone ordering if the reviewer proposes a different validation sequence. The key output is: which assumptions to validate first, with what experiments, and what success criteria.

2. **For minimum viable vertical slice definition:** Integrate into §11 as a new subsection (e.g., "§11.X Minimum Viable Vertical Slice"). List the specific files/components needed, what's built for real vs stubbed/hardcoded, and what success looks like. Cross-reference the §8.2 file map to identify which of the 131 files are in the MVP slice.

3. **For delivery sequencing changes:** Integrate into §11. If the reviewer proposes reordering, splitting, or combining milestones, restructure §11 accordingly. Update §8.2 milestone assignments if files move between milestones.

4. **For testing strategy during development:** Integrate into §11 as development-phase guidance. Do NOT modify §6.5 (Testing Strategy) which covers the long-term testing architecture. Instead, frame this as "MVP testing approach" — what testing looks like during rapid iteration vs what the full testing strategy covers.

5. **For stack sequencing (Rust vs TypeScript vs Skills first):** Integrate into §11. Add a clear recommendation on which stack to start with and why, with concrete rationale tied to risk validation.

6. **For what to cut from MVP:** Integrate into §11. Create a clear "Deferred from MVP" list with rationale for each deferral. Update §8.2 milestone assignments if files are moved to later milestones.

7. **For ideas the reviewer proposed that we didn't ask about** (feature flags, shadow mode, metrics, etc.): Evaluate each against project constraints. If applicable, add to §11 as validation techniques or operational patterns.

8. **For suggestions that change the architectural spec** (e.g., "remove Supabase for MVP", "start without the daemon"): Do NOT modify the architectural sections. Instead, frame in §11 as "Phase 0 simplifications" — what the MVP can skip while staying compatible with the full architecture. For example: "MVP can run agents via CLI without the full daemon, but should use the same result contract format for forward compatibility."

### Evaluation Framework

For each proposed change, assess:
- **Agree strongly:** Reduces time-to-first-validation without compromising the architecture. Apply directly.
- **Agree partially:** Good principle but needs adaptation to the specific constraints (zero code today, solo developer, real E2E testing mandate). Apply with modifications.
- **Disagree:** Proposes cutting something that's actually on the critical path, or proposes a 6-month waterfall plan, or proposes abandoning a core requirement. Note disagreement with rationale.

### Preservation Rules
- Preserve the Rust daemon as a core requirement (but its initial scope can be much simpler)
- Preserve the "real E2E, no mocks" principle (but testing scope for MVP can be smaller)
- Preserve the local-first + sync architecture (but sync can be deferred)
- Preserve the full architecture as the destination — the MVP is a subset, not a different system
- Do NOT delete planned files from §8.2 — instead, assign them to later milestones or mark as "post-MVP"
- Use "as-if-always" language

---

## Step 3: Fresh-Eyes Optimization

After completing Step 2, re-read the sections you edited with fresh eyes. Check for:

1. **§11 ↔ §8.2 consistency:** Does every file listed in the MVP vertical slice appear in §8.2? Are milestone assignments consistent between §11's prose and §8.2's file map? If files were moved between milestones, are all references updated?

2. **§11 ↔ §6.1 consistency:** Does the delivery order address the highest-complexity components (Very High: marketplace comparability, landed-cost engine, daemon orchestration) early enough for risk validation?

3. **§11 ↔ §6.2 consistency:** Does the validation plan cover the highest-impact risks (wrong product match, wrong HS code, freight estimate too optimistic, attachment blindness)? Each Critical-impact risk should have a corresponding validation experiment.

4. **§1 ↔ §11 consistency:** Does the Executive Summary's delivery phases overview match the restructured milestones in §11?

5. **Internal §11 consistency:** Do milestone prerequisites form a valid DAG (no circular dependencies)? Is the critical path clearly identifiable? Are success criteria specific and measurable?

6. **Scope creep check:** Did you inadvertently add more work to the MVP than what existed before? The reviewer's job is to REDUCE time-to-first-validation, not add more things to build.

Fix any inconsistencies found. Make targeted edits, not wholesale rewrites.

---

## Step 4: Sync Pass

1. **Update §1 (Executive Summary):** Refresh the Quick Stats (if file counts per milestone changed), Delivery Phases overview, and the "what will make or break the product" framing to reflect the new delivery strategy.

2. **Update §8.2 (File Reference Map):** Ensure milestone assignments match §11. If the reviewer proposed a "Phase 0" or "MVP slice," annotate affected files.

3. **Update §6.1 (Complexity Analysis):** If the reviewer provided delivery-relevant annotations (e.g., "this Very High complexity component should be tackled in week 2, not week 8"), add them as delivery notes alongside the complexity ratings.

4. **Update §6.2 (Risk Assessment):** If new delivery-specific risks were identified (e.g., "building infrastructure for 3 months before validating product thesis"), add them. If existing risks now have mitigation strategies via the validation plan, note them.

5. **Cross-reference check:** Search for references to specific milestones ("Milestone 1", "Milestone 2", etc.) throughout the document. If milestone numbering changed, update all references.

6. **Appendix E (Changelog):** Add a concise entry describing what was integrated in this review cycle.
