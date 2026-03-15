# Beads Creation Handoff — UK Public Procurement Intelligence Platform

> **Role:** You are a manufacturing planner designing a compact documentation production system. Your beads (work orders) will produce the final maintained doc set — and NOTHING else.
>
> **Deliverable:** A bead graph (using `br create`) that, when fully executed, produces exactly the maintained doc set defined in the layer strategy.
>
> **Key constraint:** Extra standalone docs are a planning failure. Compact file count does NOT mean compact content — each layer doc should be execution-grade dense.

---

## 1. Mission

You are designing a manufacturing plan for a compact documentation system. Think of beads as work orders on a factory floor: each one produces or transforms a specific artifact. The layer docs are the product. The bead graph is the manufacturing plan.

**Your goal:** Create a bead graph that transforms the ~8,400-line research document into a compact set of execution-grade layer docs, preserving ALL architectural detail while organizing it for sequential implementation.

**Critical distinction:**
- Beads are WORK ORDERS — they describe what to produce
- Layer docs are the PRODUCT — execution-grade references for implementation
- The research doc is RAW MATERIAL — dense, interconnected source content to be reorganized

**Extra standalone docs are a planning failure.** Every piece of information must land in one of the maintained files. No companion docs, no cross-layer ledgers, no decision files, no "supporting materials."

---

## 2. Final Maintained Doc Set (Non-Negotiable)

{AFTER_STRATEGY: Re-run `/layermaxxer --beads-only` after the strategy defines layer count and layer names. The strategy document will specify the exact maintained doc set. Until then, the structure below is a template.}

### Expected Structure

```
docs/strategy/
├── README.md                           # Overview + navigation
├── cross-reference-index.md            # Cross-layer navigation
├── layer-strategy.md                   # Rollup sequencing + dependency rationale (ALREADY EXISTS)
├── layer-1-{name}.md                   # Execution-grade spec for layer 1
├── layer-2-{name}.md                   # Execution-grade spec for layer 2
├── layer-3-{name}.md                   # Execution-grade spec for layer 3
├── layer-4-{name}.md                   # Execution-grade spec for layer 4
├── layer-5-{name}.md                   # Execution-grade spec for layer 5
└── layer-6-{name}.md                   # Execution-grade spec for layer 6 (if needed)
```

### Authority Split

| File | Owns |
|------|------|
| `layer-strategy.md` | Rollup sequencing, dependency rationale, layer boundary justifications, schema stability matrix, milestone mapping |
| `cross-reference-index.md` | "Where is X?" navigation — maps concepts to layer docs and sections |
| `README.md` | Project overview, document map, quick-start reading order |
| `layer-N-{name}.md` | Execution-grade source of truth for that layer: components, tables, skills, interfaces, tests, acceptance criteria, open questions |

### Path-Ownership Hard Rule

**No bead may own files outside this set.** If a bead produces content that doesn't belong to one of these files, the bead design is wrong. Restructure the bead to merge the content into the appropriate maintained file.

### Explicit Prohibitions

- **No standalone cross-layer ledgers** (e.g., "schema-evolution-plan.md") → absorb into strategy doc
- **No decision files** (e.g., "adr-resolutions.md") → absorb into the layer that resolves each ADR
- **No companion docs** (e.g., "layer-1-testing-companion.md") → absorb into layer doc
- **No "notes" or "scratch" files** → either it's in a maintained file or it doesn't exist

---

## 3. Current Repo State

### Source of Truth Documents

| Path | Lines | Classification |
|------|-------|----------------|
| `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` | ~8,400 | **SOURCE MATERIAL** — raw material to be reorganized into layer docs. Preserved as historical artifact, NOT maintained going forward. |
| `docs/research/2026-03-15-worker-validator-run-manifest-research.md` | ~2,256 | **SOURCE MATERIAL** — already integrated into main research doc. Preserved as historical artifact. |
| `docs/strategy/layer-strategy.md` | TBD | **MAINTAINED** — the strategy doc you're building beads to complement. |

### Planmaxer & Harmonymaxer Artifacts

All files under `docs/planmaxer/` and `docs/harmonymaxer/` are **archived review artifacts**. They informed the research doc but are NOT maintained going forward. Do NOT create beads that target these directories.

---

## 4. Mandatory Pre-Bead Research

**BEFORE creating any beads**, you MUST:

### 4.1 Read All Source Documents Fully

1. Read the complete research document (~8,400 lines)
2. Read the strategy document (produced from the strategy handoff)
3. Read `AGENTS.md` for project constraints

### 4.2 Classify Current Files

For every `.md` file in the project:
- **MAINTAIN** — part of the maintained doc set (list exact path)
- **SOURCE** — raw material to be reorganized (preserve as historical artifact)
- **ARCHIVE** — previous pipeline output (preserve, do not modify)
- **DELETE CANDIDATE** — truly obsolete (flag for user, do NOT delete)

### 4.3 Build Private Planning Map

Before writing beads, build a private planning map:

1. **Section inventory:** For each layer doc, list the major sections it needs
2. **Content routing:** For each major section of the research doc, identify which layer doc absorbs it
3. **Coverage check:** Verify every section of the research doc has a destination
4. **Gap check:** Verify no layer doc has sections without source content

This map is your internal planning tool — it does NOT become a separate document.

---

## 5. What the Bead Graph Must Do

### 5.1 Produce the Compact Maintained Set

Every bead in the graph must target one of the maintained files. The graph's execution must result in EXACTLY the maintained doc set and nothing else.

### 5.2 Keep Beads Self-Contained

Each bead must contain enough context that an agent can execute it WITHOUT reopening the research doc. This means:
- Include relevant architectural constraints inline
- Include relevant schema details inline
- Include relevant interface contracts inline
- Include relevant acceptance criteria inline

**"See the research doc for details"** is NEVER acceptable as a substitute for actual content in a bead.

### 5.3 Use Section-Level Ownership Within Files

Multiple beads can target the same file — they just own different sections. For example:
- Bead "Layer 1 — Daemon Components" → `layer-1-{name}.md` § Daemon Components
- Bead "Layer 1 — Database Schema" → `layer-1-{name}.md` § Database Schema
- Bead "Layer 1 — Acceptance Criteria" → `layer-1-{name}.md` § Definition of Done

### 5.4 Consolidate Instead of Proliferate

When research doc content spans multiple subsystems, the bead MERGES it into the owning layer doc. The merge is LOSSLESS — no detail is discarded through summarization. It IS reorganized for the layer's context.

### 5.5 Preserve Full Architectural Detail

Layer docs are execution-grade references. An implementation agent should NOT need to reopen the ~8,400-line research doc to recover missing detail. If a bead's target section covers "landed cost computation," it must include:
- The computation steps
- The input data requirements
- The conservative defaults
- The confidence scoring model
- The blocker conditions

Not just "implements landed cost as described in §5.7."

### 5.6 Keep Open Questions Explicit

The research doc has open questions (§10). Each open question must land in the owning layer doc under an explicit "Open Questions / Blockers" section. Do NOT:
- Silently resolve open questions by picking a default
- Hide open questions in footnotes
- Move open questions to a standalone file

### 5.7 Make Review Gates Obvious

After each layer's beads are complete, there should be a review gate bead that:
- Verifies file-count discipline (no extra files created)
- Verifies density (no lossy summarization)
- Verifies cross-layer consistency (references between layer docs are correct)

---

## 6. Recommended Topology

### 6.1 Structure

```
Root Epic: "Procurement Intelligence Layer Documentation"
├── Epic: "Layer 1 — {name}"
│   ├── Bead: "L1 — Overview & Capability"           → layer-1.md §1
│   ├── Bead: "L1 — Daemon Components"                → layer-1.md §2
│   ├── Bead: "L1 — Database Schema"                  → layer-1.md §3
│   ├── Bead: "L1 — Pipeline Stages"                  → layer-1.md §4
│   ├── Bead: "L1 — Skills & Agent Execution"         → layer-1.md §5
│   ├── Bead: "L1 — Testing & Acceptance Criteria"    → layer-1.md §6
│   ├── Bead: "L1 — Forward Compatibility"            → layer-1.md §7
│   └── Bead: "L1 — Review Gate"                      → audit of layer-1.md
├── Epic: "Layer 2 — {name}"
│   ├── ...
│   └── Bead: "L2 — Review Gate"
├── ...
├── Epic: "Cross-Layer Documentation"
│   ├── Bead: "Cross-Reference Index"                 → cross-reference-index.md
│   ├── Bead: "README"                                → README.md
│   └── Bead: "Strategy Doc Polish"                   → layer-strategy.md (updates)
└── Epic: "Final Audit"
    └── Bead: "Maintained Doc Set Verification"       → full audit
```

### 6.2 Dependency Rules

- **Within a layer:** Beads targeting different sections of the same layer doc CAN run in parallel (they own non-overlapping sections). The review gate bead depends on ALL content beads for that layer.
- **Across layers:** Layer N content beads depend on the specific Layer N-1 beads they reference — NOT the entire Layer N-1 review gate. This allows parallelization: a Layer 2 "daemon components" bead can start once Layer 1's "daemon components" bead is done, without waiting for Layer 1's "testing" bead.
- **Exception:** If a Layer N bead needs to understand the FULL context of Layer N-1 (e.g., the review gate itself, or a bead that references cross-cutting concerns from the prior layer), it should depend on the Layer N-1 review gate.
- Cross-layer documentation depends on ALL layer review gates
- Final audit depends on everything

### 6.3 Canonical-Layer Density Rule

**Layer docs are execution-grade references, NOT summaries.**

An implementation agent reading `layer-3-{name}.md` should be able to build that layer's components without EVER opening the research doc. This means:
- SQL table definitions: include the full CREATE TABLE statement, not just the table name
- TypeScript interfaces: include the full interface definition, not just the type name
- Pipeline logic: include the computation steps, not just "computes gross margin"
- Invariants: include the specific rules, not just "enforces authority matrix"

If a bead produces a section that requires the reader to open the research doc, the bead has failed.

---

## 7. Bead Quality Standards

### 7.1 Minimum Contents Per Bead

Every non-epic bead must include:

1. **Background:** Why this section exists and what it contributes to the layer
2. **Current-state context:** What source material is being reorganized
3. **Target behavior:** What the section should contain when complete
4. **Architectural detail:** Specific tables, interfaces, components, algorithms
5. **Dependencies:** What beads must complete first
6. **Verification:** How to verify the section is complete and correct
7. **Acceptance criteria:** 3-5 specific, testable criteria

### 7.2 Critical Rules

- **NEVER** say "see source doc" as a substitute for content. Include the content.
- **NEVER** normalize extra docs. If you feel the urge to create a companion file, the layer doc needs a new section instead.
- **NEVER** turn open questions into hidden assumptions. If the research doc says "open question: should X be Y or Z?", the layer doc must preserve that question explicitly.
- **NEVER** create beads that target files outside the maintained set.
- **ALWAYS** specify the exact target file AND section for each bead.

---

## 8. Final Audit Summary

When the bead graph is complete, produce a final audit report:

### 8.1 File Inventory

List EVERY file path referenced in the bead graph. Verify:
- Every path belongs to the maintained doc set
- Every maintained file has at least one bead targeting it
- No bead targets a path outside the maintained set

### 8.2 Coverage Report

For each major section of the research doc, identify:
- Which layer doc absorbs it
- Which bead is responsible
- Confidence that no detail is lost (high/medium/low)

### 8.3 Bead Statistics

- Total beads created
- Beads per layer
- Review gate beads
- Average dependencies per bead
- Maximum dependency chain length

### 8.4 Risk Assessment

- Sections at risk of lossy compression (where dense research content must be reorganized)
- Cross-layer concerns that might create inconsistencies
- Beads that feel "too large" and might need splitting

---

## 9. Follow-Up Fresh-Eyes Pass

After creating all beads:

1. **Re-read the bead graph** with fresh eyes
2. **Check for doc sprawl:** Does any bead create or imply files outside the maintained set?
3. **Check for lossy compression:** Does any bead's target section feel thin compared to the research doc source?
4. **Check for split-doc drift:** Does any bead reference content that another bead also owns? (Ownership must be unambiguous)
5. **Fix** any issues found

Ask yourself: **"If this graph executes perfectly, does the repo end with exactly the maintained doc set and nothing else?"** If the answer is no, fix the graph.
