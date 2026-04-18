# Harmonization Optimization — Worker-Validator & Run Manifest System

> **Run:** 002
> **Purpose:** Fresh-eyes verification and consistency audit after harmonization handoff execution
> **Source material:** `docs/research/2026-03-15-worker-validator-run-manifest-research.md`
> **Target doc:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
> **Generated:** 2026-03-15

---

## Mission

You are a fresh-eyes architecture auditor. The main research document has just been updated to absorb the worker-validator and run manifest system from a standalone research doc. Your job is to verify that the integration is flawless — no contradictions, no stale terminology, no silent drops, no overreach.

**You are editing documentation only. Do not change code.**

The standard is "boringly consistent" — every section that mentions validation, manifests, OQG, skill execution, or job flow should tell the same story.

---

## 1. MANDATORY Deep Re-Read

Before any audit, you MUST:

1. **Re-read the source material in full:** `docs/research/2026-03-15-worker-validator-run-manifest-research.md` (all ~2256 lines). Build a checklist of every concept it contains.

2. **Re-read the entire target document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` (all lines — it's now ~7500+ lines). Not just the modified sections — ALL of it. Cross-cutting dependencies are only visible after reading everything.

3. **Note any remaining divergences** between the source and target.

---

## 2. Contradiction Hunt

For each core concept from the source material, search every section of the target doc for statements about that concept. Verify all statements are mutually consistent. Flag any remaining stale or conflicting language.

### Target terms and concepts to hunt:

| Term / Concept | Search Pattern | What to verify |
|---|---|---|
| **"run manifest"** | `run manifest`, `run_manifest`, `runManifest` | Always full term. Never confused with "error manifest" or "opportunity manifest". Storage location consistent (artifact store). Hash and path columns in agent_session_logs. |
| **"manifest"** (unqualified) | `manifest` | Every unqualified use must clearly refer to either "opportunity manifest" (Manifest Generator, manifests table) or "error manifest" (Error Manifest System). No ambiguous uses that could mean "run manifest". |
| **"Output Quality Gate" / "OQG"** | `Output Quality Gate`, `OQG`, `quality gate` | OQG role is augmented (signals feed validator), not replaced. OQG still performs syntactic/semantic/behavioral-drift checks. Validator is a separate, optional gate AFTER OQG. |
| **"AGENT_SUSPECT_OUTPUT"** | `AGENT_SUSPECT_OUTPUT`, `suspect_output`, `suspect output` | Both OQG failures AND validator rejections can trigger this status. The error manifest must indicate which gate triggered quarantine. |
| **"validation"** | `validation`, `validate`, `validator` | No pre-existing use of "validation" conflicts with the new validation layer. The existing three validation layers (syntactic/semantic/behavioral-drift) in §4.10 are code-based checks BY the skill. The new validator is an AGENT-based check OF the skill's output. These must not be confused. |
| **"workflow_jobs"** | `workflow_jobs`, `WorkflowJob` | parent_job_id and validation_required fields present in TS interface AND SQL table. |
| **"agent_session_logs"** | `agent_session_logs`, `AgentSessionLog` | run_manifest_path and run_manifest_hash fields present in TS interface AND SQL table. |
| **"promotion"** | `promotion`, `promote`, `promotion flow` | Validation pass rates mentioned as optional promotion gate (canary → shadow → stable). |
| **"plug-and-play"** | `plug-and-play`, `composab`, `no daemon code`, `no schema change`, `no recompilation` | Validation design preserves plug-and-play composability. Adding a validator overlay to a skill directory requires NO daemon code changes. |
| **"validation_verdicts"** | `validation_verdicts`, `ValidationVerdict` | Table exists in SQL, TS interface exists, appears in authority split table as local-only. |
| **"validation_verdict_summaries"** | `validation_verdict_summaries` | Table exists in SQL, appears in authority split table as push-owned projection. |
| **"Validation Policy Engine"** | `Validation Policy`, `validation_policy` | Component described in §5.2. Metadata field in skill metadata table. Policy modes (always/sample/trigger/skip) documented. |
| **"Validation Executor"** | `Validation Executor` | Component described in §5.2. Spawns validator sub-jobs via Job Runner. |
| **"validator skill"** | `validator skill`, `validator overlay`, `base validator` | Category 6 in skill types table. Uses headless_contract mode. Follows same directory standard. |
| **"dry-run"** | `dry-run`, `dry_run`, `dryRun` | Described consistently — verdicts logged but not enforced. Default initial deployment mode. |
| **"authority split" / "authority matrix"** | `authority`, `push-owned`, `local-only` | New tables correctly classified in §4.7 table. |

---

## 3. Doc Boundary Sanity

Verify concept ownership hasn't drifted:

- [ ] Run manifests are described in §4.10 (what they are), §5.2 (who writes/stores them), §5.4 (DB columns), §6.6 (artifact storage). No section contradicts another.
- [ ] Validation policy is owned by skill metadata (§4.10) and the Validation Policy Engine component (§5.2). Not duplicated in a different section with different rules.
- [ ] Validator skills are listed in the skill types table (§4.10) and in the files-to-create list (§8.2). Both lists agree.
- [ ] Validation verdicts are described in §5.4 (data model), §4.7 (authority split), and §5.11 (error handling). All descriptions are compatible.
- [ ] The Job Runner description (§5.2) mentions spawning validator sub-jobs. The ingestion flow (§5.6) and intelligence flow (§5.7) show the validation step in the right place. These are consistent.
- [ ] §6.2 risks, §6.5 testing, §6.6 observability, and Appendix E all cover validation-related concerns. No risk is mentioned in one place and contradicted in another.

---

## 4. Implementation Readiness

Ask: could an engineer implement the worker-validator system from only the harmonized research doc?

- [ ] Are the integrated validation concepts clear enough to implement without reading the source doc?
- [ ] Is the boundary between OQG and semantic validation unmistakable?
- [ ] Could engineers reading §5.2 (components) and §5.6 (data flow) derive compatible implementations?
- [ ] Is the run manifest format described well enough? (MD artifact, human-readable narrative, what sections to include)
- [ ] Is the validation verdict schema described well enough? (verdict enum, issues array, severity levels, categories)
- [ ] Is the validation policy engine's decision logic described well enough? (always/sample/trigger/skip modes, trigger conditions)
- [ ] Is the relationship between worker job and validator job clear? (parent_job_id, validation_required, how verdict affects job advancement)
- [ ] Is the dry-run vs enforcement mode distinction clear? (when verdicts block vs when they only log)
- [ ] Is the fail-open behavior clear? (validator itself failing does not block worker results)
- [ ] Could the engineer still misread any integrated concept?

---

## 5. Overreach Check

Verify the harmonization didn't:

- [ ] Add implementation sequencing or task planning ("first build X, then Y")
- [ ] Include bead/issue tracking instructions
- [ ] Blur into execution guidance
- [ ] Over-specify things that are still design choices (e.g., exact trigger condition formulas for the validation policy engine)
- [ ] Modify sections unrelated to the worker-validator system
- [ ] Change existing skill outcome classes (they should remain exactly 6)
- [ ] Replace or weaken the existing OQG description
- [ ] Add unnecessary complexity to sections that were fine before

---

## 6. Absorption Completeness

Verify the source material is FULLY absorbed:

- [ ] **Run manifests** — described in the target doc? Format, storage, when generated, by whom?
- [ ] **Validator skills** — described in the target doc? Category, mode, base + overlay pattern?
- [ ] **Validation policy engine** — described? Modes, trigger conditions, metadata?
- [ ] **Dry-run mode** — described? What it does, when to use it?
- [ ] **Manual mode** — described? Kickoff prompt rendering for human-initiated validation?
- [ ] **New DB entities** — validation_verdicts, validation_verdict_summaries?
- [ ] **Modified DB entities** — workflow_jobs (parent_job_id, validation_required), agent_session_logs (run_manifest_path, run_manifest_hash)?
- [ ] **Integration with Job Runner** — described? Sub-job spawning, verdict processing?
- [ ] **Integration with OQG** — described? OQG signals as validator inputs?
- [ ] **Integration with promotion flow** — described? Validation as promotion gate?
- [ ] **Integration with error manifests** — described? Rejections create error manifests?
- [ ] **Plug-and-play composability** — described? No daemon code changes for new validators?
- [ ] **New files** — listed in §8.2, §8.3?
- [ ] **Milestone assignment** — files assigned in §8.4?
- [ ] **Risks** — listed in §6.2 and Appendix E?
- [ ] **Testing** — validator testing described in §6.5?
- [ ] **Observability** — validation metrics described in §6.6?

For any concept NOT found in the target doc, flag it as a silent drop and integrate it.

The source doc (`docs/research/2026-03-15-worker-validator-run-manifest-research.md`) should now be genuinely redundant — an engineer does NOT need to read it to get the full picture.

---

## 7. Terminology Consistency

Verify all sections use identical terminology for:

- **"run manifest"** — never "run log", "execution manifest", "session manifest", "skill manifest", or unqualified "manifest"
- **"validation verdict"** — never "validation result", "validation outcome", or "validation decision" (these overlap with existing terms)
- **"validator skill"** — never "validation skill", "checker skill", "audit skill"
- **"validation policy"** — never "validation config", "validation setting", "validation mode" (the MODES are always/sample/trigger/skip; the POLICY is the overall concept)
- **"approved" / "rejected" / "flagged"** — these are the three verdict values, used consistently everywhere
- **"VALIDATION_REJECTED" / "VALIDATION_FLAGGED"** — these are the error codes, distinct from the verdict values
- **"dry-run mode"** — never "test mode", "passive mode", "non-blocking mode"
- **"validator overlay"** — never "validation template", "validation rules", "domain validator" (the overlay IS domain-specific rules, but the term is "overlay")

---

## 8. Fix and Final Fresh-Eyes Pass

Fix any issues found above with targeted edits, not wholesale rewrites.

After all fixes, re-read everything ONE MORE TIME:

**"Imagine you are the implementation engineer and the docs are the only thing you get. You need to implement the worker-validator system. Could you still misread anything about:**

1. **When validation runs?** (Policy engine decides based on skill metadata)
2. **Who writes the run manifest?** (The agent, as part of its skill execution)
3. **Who reads the run manifest?** (The validator agent, spawned as a sub-job)
4. **What happens on approval?** (Worker job advances normally)
5. **What happens on rejection?** (Results quarantined as AGENT_SUSPECT_OUTPUT, error manifest created, operator alerted)
6. **What happens on flagging?** (Results advance with flag for operator review)
7. **What happens if the validator itself fails?** (Fail-open — worker results not blocked)
8. **What's the difference between OQG and semantic validation?** (OQG = code-based structural/statistical; validator = agent-based contextual/semantic)
9. **Where are run manifests stored?** (Artifact store, path in agent_session_logs)
10. **Where are validation verdicts stored?** (validation_verdicts table locally, summaries pushed to Supabase)
11. **How does this preserve plug-and-play?** (Validation policy in skill metadata, overlay in skill directory — no daemon code changes)
12. **What's the difference between the three kinds of 'manifest'?** (Opportunity manifest, error manifest, run manifest — all different things)**"

Fix anything that fails this test.

---

## 9. Required Output Format

When complete, provide:

1. **Docs changed:** List every file and section modified.
2. **Contradictions removed:** List specific contradictions that were fixed.
3. **Ambiguities tightened:** List specific ambiguities that were resolved.
4. **Remaining concerns:** List any issues that could not be resolved or require user input.

---

## 10. Follow-Up Fresh-Eyes Pass

> **Send this as a follow-up message in the same session after the optimization is complete.**

One final pass. Re-read:
1. §1 (Executive Summary) — does it mention worker-validator?
2. §4.10 (Skills) — is the validator skill category present? Is run manifest generation in the lifecycle?
3. §5.2 (Components) — are all new components present? Is Job Runner updated?
4. §5.4 (Data Model) — are all new/modified entities present in BOTH TS and SQL?
5. §5.6 (Ingestion Flow) — is the validation step present after OQG?
6. §5.11 (Error Handling) — are validation error types present?
7. §8.2 + §8.3 + §8.4 — are all files listed and assigned to milestones?

For each section, ask: "If I only read this section, would I know about the worker-validator system?" For §1, §4.10, §5.2, §5.4, §5.6, §5.11 — the answer must be yes. For other sections, it depends on relevance.

Fix anything missing.
