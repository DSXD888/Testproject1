# L2 Desktop MVP — Validate Existing Spec for 3-SOT Pipeline

## Your Mission

The spec for Layer 2 (Single-Operator Desktop MVP) already exists at `docs/strategy/layers/layer-2-single-operator-desktop.md` (959 lines, execution-grade). Before it enters the TDD pipeline, verify it has the structure needed for downstream coverage verification — requirement IDs, testable acceptance criteria, and complete error paths.

**This is NOT research-to-spec.** The spec content is authoritative — you're checking STRUCTURE, not rewriting content.

---

## The Spec to Validate

```
docs/strategy/layers/layer-2-single-operator-desktop.md    — READ FULLY (959 lines)
```

## Supporting Context (SKIM for cross-references)

```
docs/strategy/layer-strategy.md                            — Lines 490-609 (L2 section) — SKIM
docs/strategy/cross-reference-index.md                     — Topic lookup — REFERENCE
docs/strategy/layers/layer-1-thin-local-runner.md          — L1 integration surface — REFERENCE
docs/strategy/layers/layer-0-gold-set-evaluation.md        — L0 schema + pipeline — REFERENCE
```

## Quality Bar (MATCH OR EXCEED)

```
docs/strategy/layers/layer-1-thin-local-runner.md          — L1 spec had 138 L1-FR-* requirement IDs
```

The L1 spec used `L1-FR-{SUBSYSTEM}.{N}` format (e.g., `L1-FR-PTY.1`, `L1-FR-PIPELINE.3`). The L2 spec should use `L2-FR-{SUBSYSTEM}.{N}` following the same convention.

---

## Key Context From Codebase Research

- **L2 is the first user-facing layer** — a mixed Rust + TypeScript layer creating the entire frontend stack from scratch
- **The spec covers 5 major sections:** §1 (Overview + Desktop Architecture, 16 subsections), §2 (Shared UI Components), §3 (Pipeline Surface + Coverage + Decisions), §4 (Testing + Acceptance, including 16 Definition of Done criteria and 15 test scenarios), §5 (Open Questions + Dependencies)
- **Current codebase has NO `packages/` or `apps/` directories** — L2 creates all TypeScript/React infrastructure
- **L1 integration surface is live:** daemon crate at `crates/daemon/` with IPC, PTY, jobs, artifacts, sessions modules
- **The spec explicitly lists 18 desktop files (§1.9), 5 shared UI components (§1.11), 2 manifest files (§1.12), and 1 new skill (§1.13)** — these should all have requirement IDs

---

## Validation Checklist

### 1. Requirement IDs
- [ ] Every functional requirement has a unique, stable ID in `L2-FR-{SUBSYSTEM}.{N}` format
- [ ] IDs are systematic by section/subsystem, not random
- [ ] A downstream tool could grep for all `L2-FR-*` IDs mechanically
- If missing: ADD IDs systematically without changing content

**Suggested subsystem prefixes (derive from spec structure):**

| Subsystem | Prefix | Spec Section |
|-----------|--------|-------------|
| QueryService | `L2-FR-QS` | §1.4 |
| DaemonBridge | `L2-FR-DB` | §1.5 |
| Channel/Streaming | `L2-FR-CH` | §1.6 |
| Bootstrap | `L2-FR-BOOT` | §1.7 |
| Dev Bridge | `L2-FR-DEV` | §1.8 |
| Desktop Pages | `L2-FR-PAGE` | §1.9, ContractList/Detail/Sessions/etc. |
| React State | `L2-FR-STATE` | §1.10 |
| Shared UI | `L2-FR-UI` | §1.11, §2 |
| Manifests | `L2-FR-MAN` | §1.12, §2.2 |
| Contracts Finder Skill | `L2-FR-CF` | §1.13 |
| Coverage Metrics | `L2-FR-COV` | §3.1, §3.4 |
| Decision Capture | `L2-FR-DEC` | §3.2, §3.3 |
| Pagination | `L2-FR-PAG` | §3.5 |
| Accessibility | `L2-FR-A11Y` | §1.16 |
| Security | `L2-FR-SEC` | §2.3 (rule 7) |

### 2. Acceptance Criteria
- [ ] Every requirement has explicit, bullet-listed acceptance criteria
- [ ] ACs are specific and testable (not "handle appropriately")
- [ ] ACs are not buried in narrative prose
- If buried: EXTRACT into bullet lists under each requirement

**Pay special attention to:**
- §1.4 QueryService: Does each read model (list_processes, get_process_detail, list_sessions, list_errors) have explicit return type + pagination contract?
- §1.5 DaemonBridge: Does each command (10 commands listed in the table) have explicit success/failure ACs?
- §1.6 Channel streaming: Are the gap recovery, live→historical handoff, and chunked replay contracts testable?
- §1.7 Bootstrap: Are the 5 runtime mode transitions testable with explicit conditions?
- §3.2 Decision capture: Are the metadata fields and coalescing rules testable?
- §3.3 Staleness: Is the staleness detection trigger list exhaustive and testable?

### 3. Structure
- [ ] Consistent section numbering (§1.1 through §5.4)
- [ ] Clear subsystem/feature boundaries
- [ ] Success criteria / Definition of Done table (§4.2 — already has 16 criteria)
- [ ] No TBD sections, no hand-waving

### 4. Completeness
- [ ] Error paths defined (not just happy paths)
  - What happens when daemon is unreachable during a command?
  - What happens when QueryService returns empty results?
  - What happens when a Channel<T> subscription fails?
  - What happens when `file_reservation_paths` returns a conflict?
- [ ] Edge cases called out
  - Zero-lot processes
  - Processes with no candidates
  - Sessions that never reach Active phase
  - Auth challenges with no PTY input path
- [ ] Integration points specified (L1 IPC, L0 schema, L0 skills)
- [ ] No "to be determined later" items

### 5. L2-Specific Structural Checks
- [ ] Every file listed in §1.9 (18 desktop files) maps to at least one requirement ID
- [ ] Every command listed in §1.5 (10 DaemonBridge commands) maps to at least one requirement ID
- [ ] Every shared component in §1.11/§2.1 (5+1 components) maps to at least one requirement ID
- [ ] Every Definition of Done criterion in §4.2 (16 criteria) is traceable to requirement IDs
- [ ] Every test scenario in §4.4 (15 scenarios) references testable requirements
- [ ] The "Things NOT Implemented" list (§1.15) is consistent with the "What's Stubbed" list (§1.14)
- [ ] Cross-domain notes (HTML comments at end of spec) are consistent with the spec body

---

## Output

The same spec file at `docs/strategy/layers/layer-2-single-operator-desktop.md`, validated and structured. If requirement IDs are added, expect the file to grow by 100-200 lines.

---

## Follow-Up Pass (Sub-Optimization)

After making your changes, re-read the spec with fresh eyes and answer these questions:

1. **Would a test engineer know exactly what to test?** Open §4.4 and for each test scenario, can you trace it to a specific `L2-FR-*` requirement ID? If not, the requirement ID is missing.

2. **Would a developer know exactly what to build?** Pick any 3 files from §1.9. Can you derive the complete public API (function signatures, types, error handling) from the spec alone? If you'd need to guess, the spec is incomplete.

3. **Are the DaemonBridge commands fully specified?** Each of the 10 commands in §1.5 should have: input parameters, return type, error conditions, idempotency behavior, and which daemon-side handler processes it. Cross-check against `crates/daemon/src/ipc/handshake.rs` to verify the IPC protocol can accommodate these commands.

4. **Is the Channel<T> streaming contract testable?** Can you write a test that verifies gap recovery from `seq` 100 to `seq` 150? The spec should define: what triggers a gap frame, how the client requests missing chunks, and what happens if chunks are no longer available.

5. **Are the shared component props fully typed?** Every `packages/ui` component has TypeScript interface definitions in the spec. Verify they match the actual usage described in the page components. For example, does `ContractCardFrameProps.metrics.pipelineState` cover all states the QueryService can return?

6. **Count the requirement IDs you added.** The L1 spec had 138 `L1-FR-*` IDs for 17 production files. L2 has ~28+ files with a broader surface (UI + data + streaming + skill). Expect 100-180 `L2-FR-*` IDs. If you have fewer than 80, you likely missed requirements buried in prose.
