# Beads Optimization — UK Public Procurement Intelligence Platform

> **Role:** You are a meticulous QA auditor for bead graphs. Your job is to examine every bead against 6 audit dimensions and fix any issues.
>
> **Deliverable:** An optimized bead graph with all issues resolved and a detailed audit report.
>
> **Scope:** Audit and fix only. Do not redesign the bead topology — improve individual beads.

---

## 0. Mandatory Pre-Audit Read

**BEFORE auditing any beads**, read:

1. **`docs/strategy/layer-strategy.md`** — the strategy that defines layer boundaries
2. **`AGENTS.md`** — project constraints
3. **The bead graph** — all beads via `br list --status=open` and `br show <id>` for each

Build a mental model of the INTENDED final state: the maintained doc set, the authority split, and how beads map to files and sections.

---

## 1. Compact-Artifact Audit

For EVERY non-epic bead:

### 1.1 Target File Validation

- [ ] Does the bead specify an exact target file path?
- [ ] Is that file one of the allowed maintained files?
- [ ] Does the bead specify which section(s) of the target file it owns?

### 1.2 No Extra Files

- [ ] Does the bead avoid mentioning any path outside the maintained set?
- [ ] Does the bead avoid creating "companion" or "supporting" documents?
- [ ] Does the bead avoid referencing future standalone docs (e.g., "this will be expanded in a separate testing guide")?

### 1.3 Failures to Flag

- Bead targets a file not in the maintained set → **FIX: redirect content to appropriate maintained file**
- Bead mentions creating a new file → **FIX: merge content into existing maintained file**
- Bead references a "cross-layer ledger" or "companion doc" → **FIX: absorb into strategy doc or owning layer doc**

---

## 2. Consolidation Audit

For EVERY non-epic bead:

### 2.1 Source Material Identification

- [ ] Does the bead identify what source material (research doc sections) it consolidates?
- [ ] Does it specify the source sections by number (e.g., "§4.5, §5.7 steps 3-11")?

### 2.2 Lossless Preservation

- [ ] Does the bead preserve source detail (not just summarize)?
- [ ] Are SQL table definitions included in full (not just table names)?
- [ ] Are TypeScript interfaces included in full (not just type names)?
- [ ] Are computation algorithms included (not just "computes X")?
- [ ] Are invariants stated specifically (not just "enforces constraints")?

### 2.3 Source Artifact Management

- [ ] Does the bead prevent source docs from surviving as unnecessary artifacts?
- [ ] Is it clear that the research doc is a historical artifact, not a maintained reference?

### 2.4 Failures to Flag

- Bead says "see §5.7 for details" without including the details → **FIX: inline the relevant content**
- Bead produces a 50-line section from a 500-line research section → **FIX: verify no detail was lost, expand if needed**
- Bead leaves the research doc as the authoritative source for some information → **FIX: ensure layer doc is self-contained**

---

## 3. Self-Containment Audit

For EVERY non-epic bead:

### 3.1 Execution Independence

- [ ] Could an agent execute this bead without opening the research doc?
- [ ] Does the bead include all necessary context about what to write?
- [ ] Does the bead include the architectural constraints that apply to its content?
- [ ] Does the bead include the acceptance criteria for its section?

### 3.2 Dependency Context

- [ ] Does the bead reference its dependencies by bead ID?
- [ ] Does it explain what it needs from those dependencies (not just "depends on bead X")?

### 3.3 Target Specificity

- [ ] Does the bead name the exact target file?
- [ ] Does it name the exact section within the file?
- [ ] Would two agents executing the same bead produce substantially similar output?

### 3.4 Failures to Flag

- Bead description is vague ("write the daemon section") → **FIX: specify exact content expectations**
- Bead lacks architectural constraints → **FIX: inline the relevant constraints**
- Bead has no acceptance criteria → **FIX: add 3-5 testable criteria**

---

## 4. Dependency Audit

### 4.1 Sequencing Correctness

- [ ] Is the dependency order correct? (No bead depends on information from a bead that should run later)
- [ ] Are all layer N beads dependent on the layer N-1 review gate?
- [ ] Are cross-layer beads dependent on all layer review gates?

### 4.2 Parallelization Opportunities

- [ ] Can beads within a layer run in parallel where they target different sections?
- [ ] Are there beads marked as sequential that could actually run in parallel?
- [ ] Are there beads marked as parallel that actually have hidden dependencies?

### 4.3 Missing Dependencies

- [ ] Are there beads that reference content from other beads without declaring a dependency?
- [ ] Are there interface contracts that create implicit dependencies between layers?

### 4.4 Over-Conservative Dependencies

- [ ] Are there dependencies that are too conservative? (e.g., bead A depends on bead B, but they target different files and don't share information)
- [ ] Can the dependency graph be relaxed to enable more parallelization?

### 4.5 Open Question Placement

- [ ] Are open questions (from research doc §10) embedded in the correct layer doc?
- [ ] Are they in explicit "Open Questions / Blockers" sections?
- [ ] Are they NOT in a standalone file?

---

## 5. Architecture-Coverage Audit

### 5.1 Layer Completeness

For each layer defined in the strategy:
- [ ] Are all components listed in the strategy covered by at least one bead?
- [ ] Are all tables listed in the strategy covered by at least one bead?
- [ ] Are all skills listed in the strategy covered by at least one bead?
- [ ] Are all interface contracts listed in the strategy covered?

### 5.2 Data Structure Coverage

- [ ] Are ALL ~30+ SQL tables covered by beads?
- [ ] Are ALL TypeScript interfaces covered?
- [ ] Are junction tables explicitly addressed?
- [ ] Are sync infrastructure tables covered?
- [ ] Are remote-owned tables covered with correct authority classification?
- [ ] Are projection tables covered?

### 5.3 Pipeline Coverage

- [ ] Are all 15 pipeline stages covered?
- [ ] Are the stage scoping models documented per stage?
- [ ] Are the gates (quoteability, gross margin, net margin) covered with their pass criteria?
- [ ] Are validation insertion points (steps 12a-12d) covered?

### 5.4 System Coverage

- [ ] Is the agent adapter (PTY + contract mode) covered?
- [ ] Is the process supervisor covered?
- [ ] Is the resource arbiter covered?
- [ ] Is the sync engine (push + pull protocols) covered?
- [ ] Is the error manifest system covered?
- [ ] Is the run manifest system covered?
- [ ] Is the validation system covered?
- [ ] Is the skill metadata system (SKILL.md, promotion flow) covered?
- [ ] Is the observability model (4 layers) covered?
- [ ] Is customs classification covered with complexity spectrum?
- [ ] Is the frontend architecture (frame + slot, shell-specific) covered?
- [ ] Is the sync degraded-mode UX covered?

### 5.5 Decision Coverage

- [ ] Is every ADR (001-008) addressed in at least one bead?
- [ ] Are the non-negotiable constraints embedded in relevant layer docs?

### 5.6 Project-Specific Coverage

- [ ] Is the authority matrix documented per table in each layer that introduces tables?
- [ ] Is the milestone mapping (M0-M6 → layers) documented?
- [ ] Is the stack sequencing (Skills → TS → Rust → Desktop → Web) addressed?
- [ ] Is the category whitelist for customs addressed?
- [ ] Is the evaluation harness / gold set addressed?
- [ ] Are the performance targets documented?

---

## 6. Final-Shape Sanity Audit

### 6.1 File Path Enumeration

List EVERY markdown path that appears in the bead graph:

```
Expected:
  docs/strategy/README.md
  docs/strategy/cross-reference-index.md
  docs/strategy/layer-strategy.md
  docs/strategy/layer-1-{name}.md
  docs/strategy/layer-2-{name}.md
  ...
```

**Exact match?** If the bead graph references ANY file not in this list → FIX.
**Full coverage?** If ANY file in this list has NO bead targeting it → FIX.

### 6.2 End-State Visualization

If this graph executes perfectly:
1. How many files are in `docs/strategy/`?
2. Does that match the maintained doc set exactly?
3. Are there any files the graph would create outside `docs/strategy/`?
4. Are there any files in the maintained set that no bead would produce content for?

---

## 7. Required Output

When the audit is complete, produce:

### 7.1 Audit Summary

```
Total beads audited: N
  Passed (no changes): N
  Enriched (content added): N
  Fixed (structural issues): N
  Created (missing beads): N
  Flagged (needs user decision): N
```

### 7.2 Full File Path Set

```
Files in bead graph:
  1. docs/strategy/README.md (beads: X, Y)
  2. docs/strategy/cross-reference-index.md (beads: X)
  3. docs/strategy/layer-strategy.md (beads: X)
  4. docs/strategy/layer-1-{name}.md (beads: X, Y, Z, ...)
  ...

Match with maintained set: EXACT / MISMATCH (details)
```

### 7.3 Coverage Gaps

```
Uncovered research doc sections:
  § X.Y - {section name} - RISK: {what would be lost}

Uncovered system components:
  {component} - {which layer should cover it}
```

### 7.4 Remaining Concerns

Any beads that still implying extra docs, have thin coverage, or need user input.

---

## 8. Follow-Up Fresh-Eyes Pass

After ALL fixes:

1. Re-read the entire bead graph with completely fresh eyes
2. Fix any bead that would:
   - Recreate doc sprawl (extra files beyond maintained set)
   - Lose detail through lossy compression
   - Leave the research doc as the authoritative source for anything
3. Verify the graph still correctly builds exactly the maintained doc set

**Final question:** "If I execute every bead in dependency order, will the result be a compact, execution-grade documentation set where an implementation agent can build any layer by reading only its layer doc + the strategy doc?"

If yes → audit is complete.
If no → identify and fix the gaps.
