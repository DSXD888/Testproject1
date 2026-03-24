# Holistic Pass: Cross-Plan Unification

Run this AFTER all 4 domain pre-preps have been completed.

---

## Step 1: Deep Read

Read ALL strategy documents in this order:

1. **`AGENTS.md`** — Agent guidelines
2. **`docs/strategy/README.md`** — Overview and navigation
3. **`docs/strategy/layer-strategy.md`** — Cross-layer authority (~2100 lines). Read every section.
4. **`docs/strategy/cross-reference-index.md`** — Topic navigation
5. **`docs/strategy/layers/layer-0-gold-set-evaluation.md`** (~3900 lines)
6. **`docs/strategy/layers/layer-1-thin-local-runner.md`** (~1300 lines)
7. **`docs/strategy/layers/layer-2-single-operator-desktop.md`** (~700 lines)
8. **`docs/strategy/layers/layer-3-operational-hardening.md`** (~900 lines)
9. **`docs/strategy/layers/layer-4-shared-visibility.md`** (~800 lines)
10. **`docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`** (~600 lines)
11. **`docs/strategy/layers/layer-6-request-pipeline-automation.md`** (~400 lines)

Also read the upstream research document for cross-reference verification:
12. **`docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`** — Skim sections referenced by strategy docs

**Total: ~15,000 lines across 10 strategy files + research doc reference.**

Build a complete mental model of the entire architecture. You need to hold all 7 layers simultaneously.

---

## Step 2: Cross-Plan Improvement

With fresh eyes, look carefully for:

### 2.1 Contradictions Between Docs

- Same concept described differently in different layer docs
- Different numbers for the same metric (e.g., test counts, file counts, table counts)
- Conflicting rules (e.g., L1 says "basic PTY" but L3 says "PTY hardening builds on L1's full implementation")
- Stage scope assignments that differ between a layer doc and layer-strategy.md

### 2.2 Inconsistent Terminology

- Same thing called different names in different docs
- Terms defined in one doc but used without definition in another
- Abbreviations used inconsistently (e.g., "OQG" vs "Output Quality Gate" vs "output quality gate")

### 2.3 Missing Cross-References

- Doc A implies something about Doc B but doesn't say it explicitly
- A component in L3 depends on L1's adapter trait, but L3 doesn't reference the trait definition
- An exit criterion in one layer depends on a deliverable from another layer, but the dependency isn't stated

### 2.4 Invariant Violations

- A rule stated in layer-strategy.md (e.g., "no column renames") that a layer doc implicitly violates
- A schema stability rule that a layer's narrative contradicts
- A non-negotiable constraint that a layer ignores or weakens

### 2.5 Unnecessary Complexity

- Features specified in detail that could be simpler now that the design has matured
- Redundant specifications (same thing specified in two places, slightly differently)
- Over-specified details that should be deferred to implementation

### 2.6 Missing Invariants

- Properties that are assumed everywhere but not formally stated
- Consistency rules that exist implicitly but should be explicit
- Safety properties that multiple layers depend on but nobody owns

### 2.7 Stale Open Questions

- Questions listed as "open" in layer docs that have been resolved by subsequent layer specifications
- Questions in the research doc's §10 that the strategy docs have answered

### 2.8 Orphaned Decisions

- ADRs referenced but not defined
- ADRs defined but not referenced by any layer
- Interface contracts defined in layer-strategy.md but not referenced by any layer doc
- Interface contracts referenced by a layer doc but not defined in layer-strategy.md

Fix each issue with a small, targeted edit. Not one big rewrite.

---

## Step 3: Architectural Coherence

Ask yourself:

### 3.1 Component Purpose
- Does every component across all layers serve a clear purpose?
- Are there components that exist in the specification but have no clear consumer?
- Are there capabilities that are specified but never exercised by any layer?

### 3.2 Boundaries
- Are the boundaries between layers in the right place?
- Are there features that should move earlier (because later layers depend on them implicitly)?
- Are there features that should move later (because they're not needed until later)?

### 3.3 Dependencies
- Is the dependency direction correct everywhere? (Later layers depend on earlier, never the reverse.)
- Are there circular dependencies between layers?
- Are there hidden dependencies (L5 assumes something about L2 that isn't explicitly stated)?

### 3.4 Complexity Allocation
- Is complexity in the right layers?
- Is L0 (gold set) carrying too much complexity by defining all 43 tables?
- Is L3 (hardening) too complex relative to L2 (desktop) — could validation concepts be split differently?
- Is L5 (customs) proportionally complex given its importance?

### 3.5 Testing Coherence
- Do the testing strategies across layers build on each other?
- Are there testing gaps (layers that don't test critical functionality)?
- Is the "no mocks" principle consistently applied?
- Do the cumulative test file counts make sense?

### 3.6 Forward-Compatibility Chain
- Walk through the complete forward-compatibility chain: L0 creates columns → L1 populates some → L2 displays some → L3 uses some → L4 syncs them → L5 populates remaining → L6 consumes them
- Is every link in this chain present?
- Are there columns that are created in L0 but never populated by any layer?

---

## Step 4: Sync

1. **Update cross-reference index:**
   - Verify every topic points to the correct section(s)
   - Add any topics discovered during the coherence review
   - Remove any stale references

2. **Update README:**
   - Verify the maintained doc-set description is accurate
   - Update any line counts that changed
   - Verify the navigation guide is current

3. **Update layer-strategy.md:**
   - Verify all non-negotiable constraints are complete
   - Verify schema stability matrix covers all tables
   - Verify all interface contracts are listed and accurately described
   - Verify testing evolution table matches actual layer test specs
   - Verify performance targets are attributed to the correct owning layers

4. **Verify all doc metadata:**
   - Section numbering consistency
   - Table of contents accuracy (if any docs have one)
   - Cross-reference links not broken

Write all updates using as-if-always language. Do not add changelog entries.
