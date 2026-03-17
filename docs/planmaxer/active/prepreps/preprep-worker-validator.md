# Pre-Prep: Worker-Validator & Run Manifest Architecture

## Step 1: Pre-Prep Research

Read the following documents in this exact order. Do not skip any.

1. **AGENTS.md** — project constraints, toolchain, rules
2. **The research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Read the ENTIRE document, but pay special attention to:
     - §4.10 (Skills and Browser Automation — skill types including category 6 validator skills, validation policy metadata declarations, skill metadata table)
     - §5.2 (Component Designs — Run Manifest Writer, Validation Policy Engine, Validation Executor, Output Quality Gate)
     - §5.6 (Ingestion Data Flow — ingestion steps 12a-12d covering validation sub-job creation, validator invocation, verdict application, and fail-open semantics)
     - §5.11 (Error Handling — VALIDATION_REJECTED, VALIDATION_FLAGGED, VALIDATION_SKIPPED error types and their relationship to the error manifest system)
     - §6.5 (Testing Strategy — validator testing patterns, verdict path coverage, dry-run vs enforcement mode testing)
     - §6.6 (Observability — Layer 4 agent-generated run manifests, validation telemetry metrics)
     - §5.4 (Data Models — validation_verdicts table [local-only], validation_verdict_summaries table [push-only], modified workflow_jobs and agent_session_logs fields)
     - §8.2 (File Reference Map — validator skill files, validation engine files)
     - §11 (Milestones — Milestone 3 = dry-run validation, Milestone 4 = enforcement mode)
3. **The handoff prompt that produced the feedback:** `docs/planmaxer/active/handoffs/handoff-worker-validator.md`

After reading, you should be able to explain:
- The run manifest lifecycle: skill execution produces narrative MD artifact, stored via Artifact Store, referenced by path + hash in agent_session_logs
- The validator skill design: category 6 skills with base + domain overlay structure, consuming run manifests as input
- The validation policy engine: four modes (always, sample, trigger, skip) with per-skill policy declarations
- The validation executor: how it creates sub-jobs (parent_job_id linkage), invokes validator skills, and renders verdicts
- The verdict model: approved/rejected/flagged outcomes, how rejections create error manifests, how flagged items enter operator review
- The fail-open semantics: validation failure (validator skill crashes) does not block the pipeline — creates VALIDATION_SKIPPED error
- The OQG relationship: OQG is first gate (code-based, fast), validator is second gate (agent-based, semantic) — OQG signals become validator inputs, not replacements
- The dry-run vs enforcement mode progression (Milestone 3 → Milestone 4)
- The plug-and-play principle: adding a new skill requires NO daemon code changes, NO schema changes, NO recompilation

**File Ownership:** This pre-prep owns the following sections for editing:
- §4.10 (Skills and Browser Automation) — Validator skill type (category 6), validation policy metadata, skill metadata table entries for validation ONLY
- §5.2 (Component Designs) — Run Manifest Writer, Validation Policy Engine, Validation Executor components ONLY
- §5.6 (Ingestion Data Flow) — Steps 12a-12d (validation sub-flow) ONLY
- §5.11 (Error Handling) — VALIDATION_REJECTED, VALIDATION_FLAGGED, VALIDATION_SKIPPED error types ONLY
- §6.5 (Testing Strategy) — Validator-specific testing subsections ONLY
- §6.6 (Observability) — Layer 4 (run manifest) observability and validation telemetry metrics ONLY

Do NOT edit: §4.9 (daemon orchestration internals), §5.4 (SQL table definitions — note changes for sync-engine domain), §4.7 (database architecture), §5.7 (intelligence pipeline flow), §5.3 (pipeline stages), §11 (milestone structure). The daemon's job runner, session lifecycle, and adapter boundary are settled. Validator skills must work within the existing skill framework, not modify it. If the reviewer suggests changes to daemon internals, the job runner, or SQL schema, note them as deferred with `<!-- CROSS-DOMAIN: ... -->` comments.

---

## Step 2: Feedback Integration

Blunt take: the validator design is promising, but not enforcement-ready as written.

It is good enough to run in dry-run and start collecting signal. It is not yet good enough to act as a hard semantic gate across the whole pipeline, because today the validator is mostly judging a self-narrated story plus a result contract, not the run itself. That means it will catch obvious inconsistency, some plausibility failures, and some contextual nonsense. It will miss the hardest class: coherent wrongness.

So the real answer to your key question is:

* **Can it distinguish good output from schema-valid garbage at scale?**
  **Partly.** For anchored, externally checkable semantic defects, yes. For self-consistent-but-wrong runs, not reliably enough yet.

* **Can you know when it can’t?**
  **Not with the current design.** You need explicit blindness/basis metrics, adversarial challenge sets, and anomaly-conditioned health monitoring. Right now a validator can be blind and still look “healthy.”

## Where the OQG-validator boundary actually sits

The clean split you described is too neat for production reality.

The real boundary is not **structure vs semantics**. It is:

1. **Deterministic measurable invariants**
   These are facts derivable from Layers 1–3 without interpretation. Examples: actual domain visited, observed locale, observed currency, page fingerprint state, checkpoint counts, duplicate rates, price/unit histograms, fallback depth.
   This is OQG territory or at least OQG-produced evidence territory.

2. **Anchored interpretive judgment**
   This is where a validator earns its keep: “Given the anchors, do these outputs make sense in context?” Examples: did the agent likely price an accessory instead of the primary product, does a supplier result fit the product family, does the narrative explain the anomalies rather than hand-wave them.

3. **Blind zone**
   These are cases where neither deterministic checks nor the validator can know enough from current inputs. Examples: a wrong page that looks semantically plausible and has no fingerprint baseline, a fabricated narrative with plausible values, a subtle segment-level duplication bug that only appears across the full continuation chain.

That matters because some of your worst errors sit in the blind zone right now. The validator is being asked to infer them from a witness statement.

## The core design flaw

The run manifest is currently treated as a bridge between forensic bytes and semantic review. In practice it is a **self-report from the potentially mistaken actor**.

That makes the current validator closer to a **manifest auditor** than a **semantic validator**.

That is not useless. It is just a narrower thing than the architecture is implicitly claiming.

The fix is not “show the validator raw bytes.” That would be noisy and expensive. The fix is: **give the validator notarized facts derived from raw bytes, not just the worker’s narrative about them.**

## Failure modes the current design does not handle well

### 1. Coherent wrongness

Worker goes to the wrong page, extracts plausible-looking data, narrates confidently, and the result contract looks fine.

Current validator inputs:

* run manifest
* result contract
* OQG signals
* golden assertions
* historical summary

That bundle is enough to catch contradiction. It is weak against **agreement among wrong artifacts**.

This is the single biggest gap.

### 2. Self-attested “verified” fields are unsafe

Fields like `page_fingerprint_matched`, `locale_verified`, and `currency_verified` read like objective truth, but the worker is populating them. That is a semantic trap. It will anchor both operators and validators toward false certainty.

Those fields should never remain ambiguous between:

* “the agent claims it verified this”
* “the daemon observed and verified this”

Right now they blur those together.

### 3. Fail-open will fail open on the hard slice

Fail-open as default is fine for MVP. The missing piece is **fail-open observability by condition**.

The dangerous metric is not overall validator error rate. It is things like:

* `P(validator_error | source_health=amber)`
* `P(validator_error | OQG drift anomaly)`
* `P(validator_error | first run after worker version change)`
* `P(validator_error | page_fingerprint_state=unknown)`

If validator crashes cluster on anomalous runs, fail-open is opening precisely when semantic review is most needed.

### 4. Mass rejection and stale-validator incidents are not operationally closed

A layout change can produce two very different outages:

* Worker is wrong and validator correctly rejects.
* Worker is fine and validator is stale, so validator rejects valid new behavior.

Right now “consecutive rejections” as a promotion/rollback signal is too blunt. It confounds worker regressions, source changes, and validator staleness.

You need incident signatures and triage rules, not just counts.

### 5. Multi-segment validation is incomplete

Validating each `continuation_required` segment independently is necessary, but not sufficient.

You can approve:

* segment 1
* segment 2
* segment 3

…and still have a broken overall run:

* duplicated items across segments
* missing tail segment
* incompatible assumptions between segments
* changed locale/currency mid-chain
* cumulative counts that do not reconcile

Segment-local validation without chain-close validation is a real blind spot.

### 6. Bulk-run sampling is underdefined

“Full or sampled” result contract is not a safe design choice unless sampling is defined.

Uniform random sampling will miss the exact kinds of sparse corruption validators are supposed to catch:

* one wrong currency cluster
* a small duplicate tail
* one pack-basis outlier
* one accessory mistaken for a primary product

For large result contracts, the sampling strategy itself becomes part of the validator’s reliability envelope.

### 7. `golden.assertions.yaml` in live validator prompts is a hidden liability

This is one of the biggest “you didn’t ask” issues.

Using `golden.assertions.yaml` for offline regression is good. Injecting it into every live validator prompt is risky.

Why:

* it bloats the runtime prompt with stale assumptions
* it encourages validator anchoring to known-good/known-bad patterns
* it makes novelty look suspect
* it mixes **test fixtures** with **live rubric**

Goldens are for calibration and regression. Runtime validators need distilled invariants, not historical example gravity.

### 8. Historical run summaries can anchor the validator into false continuity

Cross-run context helps, but narrative history is dangerous. If a source changes legitimately, a validator primed on the last N “normal” runs can turn novelty into rejection.

The validator should see **structured deltas and incident signatures**, not a nostalgic essay about what the source usually looks like.

### 9. Worker and validator can share the same blind spot

One validator per run is fine for MVP. But don’t ignore correlated failure.

If worker and validator share:

* model family
* prompt priors
* source assumptions
* English-centric biases
* marketplace heuristics

they can fail together.

That does not mean you need consensus per run. It means you need periodic **cross-model audit sampling** during calibration, not as the live path.

### 10. Missing run manifests on the worst runs

If the worker crashes, times out, or emits an invalid contract, the most important narrative artifact may be absent.

The daemon needs a fallback path that synthesizes a **minimum machine-authored run manifest** from Layers 1–3. Otherwise the worst runs leave the thinnest review surface.

## Calibration without a large labeled corpus

You do not need thousands of labels. You do need the right labels.

The minimum viable calibration unit is **failure family × skill family**, not “number of lots.”

### Use three corpora, not one

#### 1. Clean real runs

Real successful runs from the gold lots and live dry-run traffic.

These tell you what healthy variability looks like.

#### 2. Weird-but-valid runs

Runs that are unusual but acceptable:

* source layout changed but extraction is still correct
* localized content appears but correctly normalized
* partial data but honestly narrated
* low-result-count but legitimate searches

Without this corpus, validators overlearn conservatism and reject novelty.

#### 3. Mutated bad runs

Start from real good runs and mutate them into realistic failure cases.

This is the fastest way to bootstrap negatives.

Mutation families worth building immediately:

* wrong source/domain
* wrong page kind
* locale/currency swap
* pack/unit distortion
* accessory-vs-primary confusion
* selective warning omission
* manifest/result count mismatch
* fabricated evidence references
* cross-segment duplicate/gap
* “novel but valid” near-miss reversals

The trick is to mutate **production-looking** runs, not to invent cartoonishly bad examples.

### Minimum useful calibration target

Per skill family or validator overlay, a practical starting bar is:

* **25–30 clean real runs**
* **15–20 weird-but-valid runs**
* **30–50 mutated bad runs**
* **5–10 cases in each dominant failure family**

That is enough to start dry-run with discipline.

Before enabling hard rejection for a given slice, I would want at least:

* **30 adjudicated rejected/flagged cases**
* **60 audited approved cases**
* a live challenge bank with coverage of dominant failure families

Why those numbers?

Because they let you use simple confidence bounds instead of vibes:

* If you get **26 correct rejects out of 30 adjudicated rejects**, your reject precision lower bound is roughly around the 70% mark, which is the start of something usable.
* If you audit **60 approved runs and find zero silent misses**, the “rule of three” gives you an upper bound around 5% on false-pass rate for that slice.

That is not perfection. It is enough to justify moving one slice from dry-run to enforcement.

### Use pairwise challenge sets

Absolute judgment is hard. Pairwise judgment is easier.

Offline, build cases like:

* run A = clean
* run B = one realistic corruption

Then measure whether the validator ranks B as less trustworthy or identifies the specific defect.

This gives you a lot more signal per labeled case than raw pass/fail scoring.

## Cost-benefit boundary

You should decide validation intensity with utility, not ideology.

Use this as the working equation:

```text
utility per run
= reject_rate * (precision * avoided_bad_output_cost - (1 - precision) * false_quarantine_review_cost)
- validation_cost
```

Rearranged, the break-even precision threshold is:

```text
precision > (review_cost + validation_cost / reject_rate) / (avoided_bad_output_cost + review_cost)
```

What that means in practice:

* **Low-reject slices** need either very high reject precision or very high avoided-loss per true catch.
* **High-reject slices** can justify lower precision only if the caught failures are genuinely expensive.

So “always validate” should not be a universal moral stance. It should be reserved for:

* pre-stable skills
* high-value opportunity slices
* version changes
* source-health amber/red
* low-anchor/basis-thin runs
* domains with severe downstream damage from false approve

For mature, boring slices, use:

* a small unconditional random floor for surveillance
* plus risk-triggered oversampling

Pure Bernoulli random sampling is not enough.

## What makes a validator healthy, useless, or dangerous

Approval/rejection rates do not tell you much.

A healthy validator shows:

* high-severity catches concentrated in risky slices
* low operator override rate on rejects
* low anomaly-conditioned crash/skip rate
* challenge-bank stability
* reasonable basis sufficiency on most runs

A useless validator shows:

* near-universal approval
* generic issue text
* no incremental catches over OQG
* little change in behavior during source drift

A dangerous validator shows:

* high-confidence rejects that operators override
* rejection spikes right after source or worker changes
* validator errors concentrated on triggered/anomalous runs
* high rates of `basis_sufficiency=low` but confident decisions anyway

Track that explicitly.

## Make the run manifest validator-friendly without making it ugly

The clean move is to make the run manifest **hybrid-authored**.

### Keep the top readable

Agent-authored:

* Summary
* Steps Taken
* Decisions Made
* Warnings
* Evidence narrative

### Add a concise machine appendix or sidecar

Daemon-authored, immutable, compact:

* observed URL chain / domain set
* page-kind sequence
* fingerprint state: matched / mismatched / unknown
* observed locale and currency
* auth/challenge events
* fallback depth
* checkpoint counts
* result counts from contract
* anomaly summary from OQG
* representative item sample IDs and outlier IDs
* artifact refs / hashes

The validator should consume both.

That preserves human readability while giving the validator anchors.

### Add claim/evidence references

Every important narrative claim should be able to point to:

* result contract fields
* machine appendix anchors
* artifact refs

Not every sentence. Just major claims.

### Rename misleading fields

Do not keep booleans that sound objective when they are self-reported.

Change fields like:

* `page_fingerprint_matched`
* `locale_verified`
* `currency_verified`

to either:

* `agent_claimed_*`
  or, better,
* `page_fingerprint_state_machine`
* `locale_state_machine`
* `currency_state_machine`
  with values like `matched | mismatched | unknown`

That one change alone will remove a lot of fake certainty.

## Validation and skill promotion should be tied, but carefully

### Make enforcement per slice, not system-wide

The right unit is:

**worker skill × source × worker version × validator overlay version**

Not “the whole system.”

A supplier-enrichment validator can be enforcement-ready while a document-acquisition validator stays dry-run.

### Add validator compatibility metadata

When a worker skill changes, the system needs to know whether the validator overlay is stale.

Add explicit compatibility ranges between:

* worker skill version / behavior version
* result schema version
* manifest evidence schema version
* validator overlay version

No compatibility declaration, no promotion to hard-reject mode.

### Do not use consecutive rejects alone as rollback proof

Consecutive rejects may mean:

* worker broke
* validator broke
* source changed
* manifest evidence got thinner
* sampling got unlucky

Rollback needs:

* repeated incident signature
* rapid human triage on first few cases
* override pattern
* challenge-bank sanity

### Every override is gold

An operator override should immediately become:

* a labeled calibration case
* an incident-cluster member
* a candidate mutation template

If overrides are not feeding the validator harness, you are wasting the only trustworthy labels you have.

## Concrete changes I would make now

Here are the specific doc-level diffs I’d make.

### In **SS5.2 Run Manifest Writer**

Change the run manifest from fully agent-authored to **hybrid-authored**.

Add:

* agent narrative section
* daemon-authored evidence appendix or sidecar
* claim/evidence references for major claims
* `agent_narrative_missing=true` fallback mode when worker fails to emit a manifest

Rename or split current self-reported metadata so “verified” fields are clearly machine-observed or clearly agent-claimed.

### In **SS5.2 Validation Executor**

Stop injecting raw `golden.assertions.yaml` into live validator prompts.

Replace it with a distilled runtime rubric such as `validator-rubric.yaml` or an invariant summary generated from the skill package.

Keep `golden.assertions.yaml` for offline regression and challenge-bank tests.

Also replace free-form historical run summary with **structured drift deltas and recent incident signatures**, not narrative history.

### In **SS5.2 Validation Executor**

Add verdict metadata:

* `basis_sufficiency`: `high | medium | low`
* `anchor_conflict_count`
* `incident_signature`
* `enforcement_eligible: boolean`

Rule: when `basis_sufficiency=low`, the validator should not hard-reject unless there is a hard machine-observed contradiction. Otherwise it should `flag` with a `coverage_gap` issue.

That is how you know when the validator cannot know.

### In **SS5.2 Validation Policy Engine**

Replace pure random sampling with **risk-stratified sampling plus a small random floor**.

Add trigger inputs for:

* source health amber/red
* page fingerprint state unknown/missing
* validator error spike on same slice
* continuation chain length > 1
* unusually high fallback depth
* low basis sufficiency in recent runs
* high-value opportunities

### In **SS5.6 step 12c / SS5.7 validation note**

Define sampled result-contract input as **anomaly-stratified**, not “sampled.”

Always include:

* OQG outliers
* duplicate cluster reps
* first/last processed items
* extreme prices or quantities
* a few median/boring rows
* segment boundary rows for continuation chains

### In **SS5.6 / SS5.2**

Keep per-segment validation for `continuation_required`, but add **chain-close aggregate validation** before final stage-head advancement or final remote projection.

Intermediate segments can commit locally, but the chain should carry `stage_closed=false` or equivalent until aggregate validation passes.

### In **SS5.11**

Add explicit handling for validator blindness, either as:

* a dedicated `VALIDATION_FLAGGED_INSUFFICIENT_BASIS` code
  or
* a required `coverage_gap` flagged issue with downstream “unverified” semantics

Also make `VALIDATION_SKIPPED_VALIDATOR_ERROR` propagate to downstream trust state, not just sit as a log flag.

### In **SS6.5**

Add a formal validator calibration harness:

* real clean corpus
* weird-valid corpus
* mutation bank by failure family
* pairwise challenge sets
* Wilson-bound precision gates for rejects
* audited-approval false-pass sampling
* mandatory incorporation of operator overrides into the labeled set

Also add daily or per-release validator canary runs against known challenge cases.

### In **SS6.6**

Expand validator health monitoring to include:

* severity-weighted reject precision
* sampled false-approve rate
* `P(validator_error | anomaly)`
* basis sufficiency distribution
* high-confidence wrong rate
* operator override rate by issue category
* true catches per 100 validations
* utility per dollar

That is the difference between “validator is busy” and “validator is good.”

### In **SS8.4 / SS11**

Make the dry-run → enforcement transition **per slice**, not system-wide.

And add an automatic demotion rule:

* if operator override rate spikes
* or challenge-bank performance regresses
* or anomaly-conditioned validator error rate rises

then that slice falls back to dry-run or flag-only behavior without disabling validation everywhere.

## Final call

I would approve this architecture for **Milestone 3 dry-run with strong instrumentation**.

I would **not** approve it for blanket Milestone 4 hard enforcement as-is.

I would approve **per-skill, per-source enforcement** only after:

* the run manifest becomes hybrid-authored
* validator blindness becomes first-class
* live runtime prompts stop depending on raw goldens/history
* bulk sampling is defined
* chain-close validation exists for continuation runs
* reject precision and false-pass audits clear explicit thresholds on that slice

The architecture is close. The missing step is to stop treating the validator like a mind-reader and start treating it like an evaluator that needs evidence, calibration, and a way to say “I don’t actually know.”


For each change proposed by the reviewer, evaluate:

### Integration Rules

1. **For run manifest format improvements:** Integrate into §5.2 (Run Manifest Writer component). The current design specifies narrative MD format with structured metadata header. If the reviewer proposes additional structure (sections, required fields, machine-parseable blocks), integrate while preserving the "human-readable narrative" principle. Run manifests are forensic artifacts — they must be readable by operators, not just validators.

2. **For validator skill architecture improvements:** Integrate into §4.10 (category 6 skill type) and §5.2 (Validation Executor). If the reviewer proposes overlay composition patterns, domain-specific validators, or validator chaining, integrate while preserving plug-and-play composability. Adding a new validator must require only a SKILL.md file and policy declaration, not daemon changes.

3. **For validation policy engine refinements:** Integrate into §5.2 (Validation Policy Engine). If the reviewer proposes additional policy modes beyond always/sample/trigger/skip, or proposes policy inheritance rules, or proposes confidence-based sampling, integrate with concrete configuration examples. Policies must be declarative (in skill metadata), not imperative (in daemon code).

4. **For verdict model improvements:** Integrate into §5.2 (Validation Executor) and §5.11 (error types). If the reviewer proposes verdict severity levels, partial approvals, conditional approvals, or verdict expiry, evaluate against the principle that verdicts are binary-ish (approved/rejected/flagged) to keep the pipeline simple. Complex verdict taxonomies risk becoming a second error taxonomy.

5. **For fail-open semantics refinements:** Integrate into §5.6 (steps 12a-12d) and §5.11 (VALIDATION_SKIPPED). If the reviewer proposes configurable fail-open vs fail-closed per skill or per policy, integrate into the policy engine. The default MUST remain fail-open for MVP.

6. **For validation testing strategy:** Integrate into §6.5. Add validator-specific test patterns: verdict path coverage (all three outcomes), dry-run assertion tests, golden run manifest + expected verdict pairs, validator skill promotion criteria. Stay within the "no mocks" constraint — validator tests use real validator skills against real (or golden) run manifests.

7. **For validation observability:** Integrate into §6.6. If the reviewer proposes additional metrics beyond the five currently specified, or proposes alerting thresholds (e.g., rejection rate spike), or proposes verdict audit trails, integrate into Layer 4 and the validation telemetry subsection.

8. **For changes touching SQL schema, job runner internals, pipeline stage definitions, or milestone structure:** Do NOT integrate directly. Add `<!-- CROSS-DOMAIN: Reviewer suggested [X] for [component]. Needs coordination with [domain]. -->` notes.

### Evaluation Framework

For each proposed change, assess:
- **Agree strongly:** Fills a concrete gap in validation architecture, improves verdict reliability, or strengthens the forensic audit trail. Apply directly.
- **Agree partially:** Good pattern but risks adding complexity to the validator skill authoring experience (which must stay simple — SKILL.md + policy declaration). Apply with simplification, noting tradeoffs.
- **Disagree:** Conflicts with settled decisions (fail-open default, OQG as first gate, plug-and-play composability, narrative MD format for run manifests, no daemon code changes for new skills). Note disagreement with rationale.

### Preservation Rules
- Preserve fail-open as the default validation semantics (validator crash does not block pipeline)
- Preserve OQG as first gate (code-based) and validator as second gate (agent-based) — never merge them
- Preserve run manifest format as narrative MD with structured metadata header
- Preserve plug-and-play composability (new skill = SKILL.md + policy, no daemon changes)
- Preserve the three verdict outcomes (approved/rejected/flagged)
- Preserve parent_job_id linkage for validation sub-jobs
- Preserve the dry-run → enforcement mode progression
- Preserve the error taxonomy structure (VALIDATION_* types under the existing taxonomy)
- Use "as-if-always" language

---

## Step 3: Fresh-Eyes Optimization

After completing Step 2, re-read the sections you edited with fresh eyes. Check for:

1. **§4.10 skill metadata ↔ §5.2 Validation Policy Engine consistency:** Does the validation_policy metadata declared in skill SKILL.md files (§4.10) match what the Validation Policy Engine consumes (§5.2)? Are the four policy modes (always, sample, trigger, skip) referenced identically in both sections? If the reviewer added new policy modes, are they reflected in both places?

2. **§5.2 verdict types ↔ §5.11 error types consistency:** Do the three verdict outcomes (approved/rejected/flagged) in the Validation Executor (§5.2) produce the correct corresponding error types in §5.11? Specifically: rejected → VALIDATION_REJECTED (creates error manifest), flagged → VALIDATION_FLAGGED (enters operator queue), validator crash → VALIDATION_SKIPPED (fail-open). Are there any verdict outcomes that don't map to an error type, or error types that don't map to a verdict?

3. **§6.6 Layer 4 ↔ §5.2 Run Manifest Writer consistency:** Does the observability description of Layer 4 (agent-generated run manifests) in §6.6 match the run manifest format defined in the Run Manifest Writer component (§5.2)? Are the same fields/sections referenced? If the reviewer changed the manifest format, is the observability layer updated?

4. **§6.5 validator testing ↔ verdict paths consistency:** Does the testing strategy in §6.5 cover all three verdict paths (approved, rejected, flagged) plus the fail-open path (VALIDATION_SKIPPED)? Are there golden run manifests specified for each path? Does the validator promotion flow in §4.10 require passing these tests?

5. **§5.6 steps 12a-12d ↔ §5.2 Validation Executor consistency:** Do the ingestion flow steps describing validation (§5.6) match the component design for the Validation Executor (§5.2)? Same sequence, same decision points, same fail-open behavior?

6. **Validation telemetry metrics consistency:** Are the validation-specific metrics in §6.6 consistent with what the Validation Executor (§5.2) and Validation Policy Engine (§5.2) can actually emit? No phantom metrics that nothing produces.

7. **Terminology:** Ensure consistent use of "run manifest" (not "execution log" or "session transcript"), "verdict" (not "result" or "decision" when referring to validation outcomes), "validator skill" (not "validation agent" or "checker"), "policy" (not "rule" or "config" when referring to validation triggering).

Fix any inconsistencies found. Make targeted edits, not wholesale rewrites.

---

## Step 4: Sync Pass

1. **Update §1 (Executive Summary):** If the validation architecture changed materially (new policy modes, changed verdict model, altered fail-open semantics), update the executive summary to reflect the current state of the worker-validator system. The four-layer observability model should still reference Layer 4 correctly.

2. **Update Appendix E (Changelog):** Add a concise entry describing what was integrated in this review cycle. Include: which sections were modified, what the reviewer's key contributions were, and any deferred cross-domain items.

3. **Verify cross-references between validation sections:** Every mention of validation in §4.10 should cross-reference §5.2 components. Every mention of validation errors in §5.11 should cross-reference the verdict model in §5.2. Every mention of validation testing in §6.5 should cross-reference the validator skill type in §4.10. Every mention of run manifests in §6.6 should cross-reference the Run Manifest Writer in §5.2.

4. **Check §10 (Open Questions):** If any open questions about validation have been resolved by this review cycle, mark them as resolved. If new questions emerged, add them.

5. **Verify §8.2 file references:** If the reviewer proposed new validator skill files, validation engine source files, or test fixtures, ensure they appear in §8.2 with correct milestone assignments (validation files should be in Milestone 3 for dry-run, Milestone 4 for enforcement).
