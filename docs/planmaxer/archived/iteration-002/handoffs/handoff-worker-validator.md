# Handoff: Worker-Validator & Semantic Quality Assurance

## Expert Framing

You are an **LLM evaluation and quality assurance engineer** who has built production-grade output validation systems for AI agent pipelines. You have designed and operated LLM output evaluation frameworks (RAGAS, DeepEval, custom evaluation harnesses), agent reliability monitoring systems, and the operational infrastructure required to validate semantic correctness at scale. You have firsthand experience with the gap between "schema-valid output" and "semantically correct output" — which is where most production agent systems fail silently. You have debugged cases where an agent returned perfectly structured JSON containing confidently wrong answers, where evaluation metrics disagreed with human judgment, and where validators themselves became the reliability bottleneck.

You draw from experience with: LLM-as-judge evaluation pipelines, calibration techniques for AI evaluators without large labeled corpora, adversarial robustness testing for generated outputs, cost-efficient validation architectures that balance coverage with budget, and the operational patterns of companies running hundreds of agent evaluations per day in production (not research benchmarks).

You understand that validation is a spectrum problem, not a binary one — and that the hardest part of building a validator is not the validator itself, but knowing when the validator is wrong.

---

## Mission

ULTRATHINK and be meticulous.

The procurement intelligence platform runs its entire data pipeline through AI agent skills — procurement ingestion, supplier price scraping, document acquisition, product extraction, qualification, and bid automation. All via skill-based agents orchestrated by a Rust daemon through adapter-mediated execution (PTY and contract modes).

A new **worker-validator architecture** adds semantic validation: every skill run produces a **run manifest** (a structured Markdown narrative of what the agent did, what it found, what decisions it made, and any warnings), and **validator agent skills** review those manifests to catch contextual errors that are invisible to code-based checks. The Output Quality Gate (OQG) handles structural validation (schema conformance, fill rates, drift detection); the validator handles semantic validation (did the agent actually navigate to the correct page? does the extracted data make sense in context? are the prices plausible for this product category?).

This system was integrated into the architecture on 2026-03-15 and has **never been through expert review**. Your job: stress-test the validator architecture for production viability. Identify failure modes, calibration challenges, cost-benefit boundaries, and maintenance burdens that the current design does not adequately address.

**Key question: can validator agents reliably distinguish good output from schema-valid garbage at scale — and can you know when they can't?**

**Don't just answer my questions — the questions I DON'T KNOW to ask are more important than the ones I do.**

---

## User's Strong Beliefs (Tensions to Engage With)

### 1. "OQG catches structural errors, validators catch semantic errors — clean separation."

The architecture draws a firm line: code-based OQG handles schema conformance, fill-rate thresholds, and statistical drift detection. Agent-based validators handle semantic correctness — did the agent navigate to the right page, is the extracted data contextually plausible, do the prices make sense for this product category.

**But semantic errors ARE often structural in disguise.** A skill that scrapes Alibaba with the wrong locale setting will produce schema-valid data with correct structure — but the prices will be in CNY when the system expects USD, or the product titles will be in Chinese when the downstream pipeline expects English. The fill rate looks healthy. The schema validates. The OQG drift detector might not fire if the source always returns Chinese for this query. Only the validator (or a human) would notice that the prices don't match the expected currency for the declared locale.

Where does the OQG-validator boundary actually sit in practice? Are there categories of error that fall through both gates?

### 2. "Fail-open on validator errors is the right default."

If the validator itself crashes, times out, or produces an invalid verdict schema, the worker results advance as if validation were skipped. The reasoning: workers operated without validators before; validation is an improvement layer, not a hard dependency. Broken validators should never block the pipeline.

**But what if validators are systematically failing on exactly the hard cases they're designed to catch?** If a source starts serving degraded content, the validator might encounter unfamiliar patterns and crash — precisely when its judgment is most needed. Fail-open means the pipeline keeps advancing garbage. The `VALIDATION_SKIPPED_VALIDATOR_ERROR` flag exists, but does anyone act on it before the damage accumulates? What is the operational workflow when fail-open fires repeatedly?

### 3. "Run manifests are the bridge between forensic bytes and human review."

The run manifest is a Layer 4 observability artifact — a structured Markdown narrative written by the agent at the end of every skill run. It fills the gap between Layer 1/2 (too coarse for semantic review) and Layer 3 (too noisy for automated analysis). The manifest is the primary input for validator consumption.

**But the manifest is produced by the same agent that might have made errors.** An agent that navigated to the wrong page could produce a convincing narrative about the right page — the agent doesn't know it's wrong. An agent that extracted prices from a promotional banner instead of the product listing could narrate "extracted primary product price" with full confidence. Can an agent accurately narrate its own mistakes? And if not, what anchors can the validator use to detect self-consistent-but-wrong narratives?

---

## Essential Context

### What the Platform Is

A UK public procurement intelligence platform that:

1. **Ingests** official contract notices daily from Find a Tender, Contracts Finder, and PCS via agent skills (browser automation).
2. **Qualifies** notices for physical-goods supply opportunities using expiry checks, access route classification, and goods-density scoring.
3. **Enriches** qualified notices with supplier pricing from eBay, Amazon, Alibaba, and 1688 via agent skills (browser automation).
4. **Estimates** landed cost using HMRC Tariff, freight APIs, FX APIs, and an internal cost-estimation engine.
5. **Surfaces** high-margin opportunities in a desktop control panel (Tauri) and a read-only web viewing portal.
6. **Automates** request pipeline preparation (not real-money bids) via portal-specific agent skills.

The entire data retrieval pipeline runs through **AI agent skills** — Markdown playbooks (`SKILL.md`) that agents follow to navigate websites, extract data, and produce structured outputs. A **Rust daemon** orchestrates these agents via PTY terminals and structured contract mode, with cron scheduling, health monitoring, and error manifest generation.

### The Worker-Validator System

The worker-validator architecture adds a semantic validation layer on top of the existing pipeline:

**Run Manifest (Layer 4 observability):** After every skill run, the agent writes a human-readable Markdown narrative describing what it did, what it found, what decisions it made, and any warnings. The manifest uses YAML frontmatter with structured metadata (manifest_version, skill_id, skill_version, job_id, session_id, execution_mode, outcome_class, items_processed/skipped/failed, warnings_count, pages_visited, source_url, page_fingerprint_matched, locale_verified, currency_verified) followed by Markdown sections: Summary, Steps Taken, Decisions Made, Warnings, Evidence. Target size: < 50 KB. Stored as an immutable artifact at `artifacts/sessions/YYYY/MM/<session_id>/run-manifest.md`.

**Output Quality Gate (OQG):** The existing code-based first gate. Checks schema conformance, fill rates, statistical drift, and value-range plausibility. OQG is fast, cheap, deterministic, and catches structural errors. OQG is augmented, not replaced.

**Validation Policy Engine:** Determines whether a completed job requires semantic validation based on:
- `always` — every run is validated.
- `sample` — random sample at configured rate.
- `trigger` — validate only when trigger conditions fire (fill_rate_drop, first_run_after_version_change, suspect_output_in_last_5, operator_requested).
- `skip` — no validation (OQG code checks still run).
- Pre-stable skills (draft/candidate/canary/shadow) — **ALWAYS VALIDATE** regardless of declared policy.

**Validation Executor:** Spawns a validator sub-job via the Job Runner. Composes the validator's kickoff prompt with: the run manifest (full text), result contract (full or sampled), `golden.assertions.yaml` from the worker skill directory, OQG semantic and drift signals, and historical run summary (last N outcomes for trend context). The validator does NOT see raw forensic bytes or the worker's internal reasoning.

**Validator Skills (Category 6):** Base validator skill (`skills/validation/base/SKILL.md`) plus domain-specific overlays for procurement-ingestion, supplier-enrichment, document-acquisition, and product-extraction. Validators run in `headless_contract` mode with `structured_extract_low_cost` model profile. Cost: ~$0.01-0.03 per validation.

**Verdict Types:**
- `approved` — worker results committed normally.
- `rejected` — results quarantined as `AGENT_SUSPECT_OUTPUT`; error manifest created with validator's reasoning and suggested fixes; operator alerted.
- `flagged` — results accepted with flag for operator review; non-critical issues recorded.

**Dry-run mode (Milestone 3):** Verdicts logged but not enforced. Rejected results still advance. Calibration phase.

**Enforcement mode (Milestone 4):** Rejections quarantine results. Domain-specific overlays introduced. Validation trend dashboard. Auto-rollback on consecutive rejections as a promotion gate.

**Fail-open principle:** Validator failure (crash, timeout, invalid verdict) is treated as a validator error, not a worker error. Worker results advance with `validation_skipped_validator_error` flag.

**Multi-segment validation:** When a worker exits with `continuation_required`, each segment is validated independently. Rejected segments are quarantined; prior committed segments remain.

### Key Terms

- **Run manifest:** Structured MD narrative produced by the worker agent at the end of every skill run. Primary input for validators.
- **OQG (Output Quality Gate):** Code-based first gate. Schema, fill rates, drift. Deterministic.
- **Validation verdict:** The validator's assessment of the worker's output: approved / rejected / flagged.
- **Validator overlay:** Domain-specific validation instructions layered on top of the base validator skill. Lives in each worker skill's directory as `validator-overlay.md`.
- **Dry-run mode:** Verdicts logged but not enforced. For calibration before enforcement.
- **Enforcement mode:** Validator rejections quarantine results.
- **Fail-open:** Validator failures do not block the pipeline.
- **Skill promotion stages:** draft -> candidate -> canary -> shadow -> stable -> rollback. Pre-stable skills always require validation regardless of policy.
- **golden.assertions.yaml:** Per-skill file containing known-good and known-bad assertions for regression testing.

---

## Current Architecture (Sections to Focus On)

### Run Manifest Writer (SS5.2)

After a skill completes, the agent writes a human-readable Markdown narrative describing what it did. The run manifest is written BY the agent as part of the skill's output — embedded in the kickoff prompt as a standardized instruction block. The daemon stores the manifest as an immutable artifact and records its path and SHA-256 hash in `agent_session_logs`.

Key design choices:
- Manifest is < 50 KB — focused narrative, not a log dump.
- For bulk operations (hundreds of items), the manifest summarizes patterns and highlights anomalies rather than enumerating every item.
- YAML frontmatter has structured metadata for automated parsing; Markdown body is for human/validator consumption.
- Three disambiguation: "opportunity manifest" (for operators), "error manifest" (for failures), "run manifest" (this component). Always use full term.

### Validation Policy Engine (SS5.2)

Decision flow:
1. Schema validation already failed? -> No validator (already quarantined).
2. Skill in pre-stable promotion stage? -> ALWAYS VALIDATE.
3. `always` -> validate.
4. `sample` -> random roll against sample rate.
5. `trigger` -> check trigger conditions.
6. `skip` -> skip (OQG still runs).
7. Any trigger condition met regardless of policy? -> VALIDATE.

Composability: Policy reads from skill metadata. No daemon code changes needed to adjust validation for any skill.

### Validation Executor (SS5.2)

Kickoff prompt composition: run manifest + result contract + golden assertions + OQG signals + historical run summary. Validator does NOT see raw forensic bytes or worker's internal reasoning (prevents anchoring bias).

Verdict processing: approved -> commit, rejected -> quarantine + error manifest + alert, flagged -> commit with flag.

Validation is non-recursive: validators do not have their own validators. Unreliable validators are fixed through the same skill promotion flow as workers.

### Ingestion Data Flow with Validation (SS5.6, steps 12a-12d)

After result contract validation (step 12) and OQG checking:
- 12a: Agent writes run manifest.
- 12b: Validation Policy Engine checks skill's `validation_policy` metadata.
- 12c: Validator reads run manifest, OQG signals, result contract summary, golden assertions. Renders verdict.
- 12d: Verdict processing — approved proceeds, rejected quarantines, flagged proceeds with flag. Dry-run mode: verdicts logged without enforcement.

### Validation Error Types (SS5.11)

- `VALIDATION_REJECTED` — Critical semantic issues found. Results quarantined.
- `VALIDATION_FLAGGED` — Non-critical issues found. Results accepted with flag.
- `VALIDATION_SKIPPED_VALIDATOR_ERROR` — Validator itself failed. Worker results advance. Fail-open.

Key distinction: `AGENT_SUSPECT_OUTPUT` can now be set by either OQG or a validator rejection. Error manifests record which gate triggered quarantine.

### DB Entities

**New tables:**
- `validation_verdicts` (local-only): verdict, issues array, reasoning, quality assessment per validation run.
- `validation_verdict_summaries` (push-only to Supabase): aggregate validation metrics for the viewing portal.

**Modified tables:**
- `workflow_jobs`: +`parent_job_id` (validator sub-jobs reference parent worker job), +`validation_required`, +`validation_verdict_id`.
- `agent_session_logs`: +`run_manifest_path`, +`run_manifest_hash`.

### Four-Layer Observability (SS6.6)

- Layer 1: Control events (daemon-generated). Session lifecycle.
- Layer 2: Protocol events (adapter-generated). Vendor JSONL, checkpoints, result contracts.
- Layer 3: Forensic bytes (immutable raw capture). Too large for automated review.
- Layer 4: Narrative (agent-generated run manifest). Structured summary for human and validator consumption.

Critical rule: Only Layers 1 and 2 drive workflow state. Layer 4 informs the validator's semantic judgment, which influences state indirectly via verdicts, but the run manifest itself never drives state directly.

### Milestone Sequence

- **Milestone 3 (Operational Hardening):** Validation in dry-run mode. Base validator skill, validation policy engine, run manifest generation. Calibrate false positive rate.
- **Milestone 4 (Shared Visibility):** Enforcement mode. Domain-specific validator overlays. Validation trend dashboard. Auto-rollback on consecutive rejections.

### Planned Files

**Rust daemon:** `src/validation/policy.rs`, `src/validation/executor.rs`, `src/validation/manifest_writer.rs`

**Skills:**
- `skills/validation/base/SKILL.md` + `result.schema.json` + `checkpoint.schema.json` + `canary.inputs.jsonl` + `golden.assertions.yaml`
- `skills/validation/procurement-ingestion/SKILL.md`
- `skills/validation/supplier-enrichment/SKILL.md`
- `skills/validation/document-acquisition/SKILL.md`
- `skills/validation/product-extraction/SKILL.md`

**Tests:**
- `crates/daemon/tests/validation_policy.rs` — policy engine decision logic
- `crates/daemon/tests/validation_executor.rs` — sub-job spawning, prompt composition, verdict processing
- `crates/daemon/tests/validation_dry_run.rs` — dry-run mode behavior
- `crates/daemon/tests/validation_golden_verdicts.rs` — known-good/bad manifests -> verify correct verdicts

---

## Tensions to Resolve

### Tension 1: Validator Calibration Without a Large Labeled Corpus

The architecture assumes validators can be calibrated during Milestone 3 dry-run mode. But calibration requires knowing whether the validator's verdicts are correct — which requires ground truth.

The gold set (50-100 lots from Milestone 0) provides some ground truth for the end-to-end pipeline, but:
- 50-100 lots produce maybe 200-400 total skill runs across all skill types. Of these, only a fraction will have ambiguous or erroneous outputs worth validating.
- The gold set is curated for pipeline accuracy, not for validator calibration. It contains "known-good inputs" — but validators need "known-bad inputs" to calibrate rejection sensitivity.
- The `golden.assertions.yaml` per skill provides some known-good/bad run manifests for regression testing, but who creates and maintains these? How many are enough?

**The calibration chicken-and-egg:** You need a validator to catch bad outputs, but you need bad outputs to calibrate the validator. During dry-run mode, how do you systematically generate or discover bad outputs to test the validator against? What's the minimum corpus size for meaningful validator calibration per skill type?

### Tension 2: Adversarial Robustness of Self-Narrated Manifests

The run manifest is produced by the worker agent — the same agent that might have made errors. The architecture acknowledges this in Layer 4 design but does not describe specific mitigations.

Concrete failure scenarios:
- **Wrong-page narration:** An agent navigates to a 404 page or a different product category but produces a narrative saying "navigated to the target category page, found 12 listings." The page_fingerprint_matched field might catch this — but what if the source has no registered fingerprint yet (new source, first run)?
- **Locale blindness:** An agent on Alibaba gets served Chinese-language content instead of English, extracts product names in Chinese, but narrates "extracted product names in English" because the prompt told it to expect English. The manifest's `locale_verified` field is self-reported.
- **Selective omission:** An agent encounters 5 errors during extraction but only narrates 2 in the Warnings section. The manifest says `warnings_count: 2` which matches the narrative but not reality.
- **Confabulated evidence:** An agent fails to find matching products, fabricates plausible-sounding results, and produces a narrative consistent with the fabrication.

How robust is the validator against self-consistent-but-wrong manifests? What external anchors (beyond the run manifest) does the validator have access to? The architecture says the validator sees OQG signals and golden assertions — is that enough?

### Tension 3: Validation Cost-Benefit at Scale

At $0.01-0.03 per validation, "always validate" adds 5-15% overhead. The architecture positions this as acceptable. But the value depends on precision:

- If validators reject 0.1% of runs and 80% of those rejections are correct, that's 0.08% true catches at a 5-15% overhead cost. Is this worth it?
- If validators reject 2% of runs and 50% are false positives, that's 1% true catches but also 1% false quarantines that require operator intervention to release.
- The `sample` policy mode exists to reduce cost. But sampling at 10% means 90% of runs go unvalidated — and the runs most likely to be bad (novel sources, edge cases) are also the ones least likely to match historical patterns that trigger-based validation would catch.

What's the minimum useful precision for the validator to justify its cost? What metrics should the system track during dry-run mode to make the enforcement decision data-driven? Is there a way to dynamically adjust validation intensity based on measured validator utility?

### Tension 4: Cascading Validator Failures Under Source Degradation

If a source degrades (e.g., Alibaba starts serving CAPTCHAs, a procurement portal changes layout), the worker skills will produce unusual outputs. The validators will see unusual run manifests.

Two scenarios:
- **Mass rejection:** Validators correctly identify that outputs are wrong, reject all runs from the degraded source. The pipeline accumulates a backlog of quarantined results. The source circuit breaker (existing infrastructure) should eventually open — but the circuit breaker triggers on worker-level failures, not on validator rejections. Does a wave of validator rejections feed back into the circuit breaker? Or does the source keep sending workers, and the validators keep rejecting, burning compute on both sides?
- **Validator confusion:** The source change is subtle enough that workers produce "different but not wrong" outputs. Validators trained on old patterns reject valid new outputs. False positive rate spikes. If this happens during enforcement mode, the pipeline stops advancing for that source — effectively a validator-caused outage.

Does the architecture handle mass rejection gracefully? Is there a validator-level circuit breaker? What happens when validator false positive rate exceeds a threshold?

### Tension 5: Validator Skill Maintenance Burden

Validators must understand what "correct" looks like for each source. The base validator handles generic quality checks (completeness, consistency, cross-check against result contract). Domain-specific overlays add source-specific validation criteria.

When sources change:
- The worker skill needs updating (new selectors, new navigation, new data format).
- The worker's `golden.assertions.yaml` needs updating.
- The domain-specific validator overlay needs updating (new "correct" patterns).
- The validator's `canary.inputs.jsonl` needs updating (new known-good/bad examples).

This is effectively a doubled maintenance surface. Every source change requires coordinated updates to worker skill, worker golden assertions, validator overlay, and validator test fixtures.

Questions:
- Can validator overlays be auto-generated or derived from worker skill metadata, or must they be hand-authored?
- When a worker skill is updated, how does the system detect that the validator overlay is now stale? Is there a version coupling between worker skills and their validator overlays?
- The skill promotion flow (draft -> stable) gates on canary/golden/shadow tests. Does this flow also validate that the validator overlay is compatible with the new worker skill version?

---

## What I Don't Want (Anti-Patterns)

- **Don't propose replacing agent-based validation with more code-based checks.** The whole point of the validator architecture is catching what code-based OQG cannot see. The semantic gap between "schema-valid" and "contextually correct" is the problem space. Expanding OQG is not the answer.
- **Don't propose LLM-as-judge benchmarking frameworks that require massive labeled datasets.** We need practical calibration techniques that work with 50-100 gold lots and incrementally grow. Academic evaluation frameworks that assume thousands of labeled examples are not applicable here.
- **Don't suggest removing fail-open semantics entirely.** Validator reliability is not yet proven. Fail-open is the correct conservative default until enforcement mode is validated. The question is how to make fail-open observable and actionable, not whether to remove it.
- **Don't propose heavyweight consensus mechanisms (multiple validators per run) for MVP.** Multi-validator consensus is interesting for later, but MVP needs one validator per run with high confidence. The cost and latency budget does not support ensembling.

---

## What I Do Want

1. **Concrete proposals for validator calibration without a large labeled corpus.** How do you bootstrap validator accuracy with 50-100 gold lots? What synthetic bad-output generation techniques can expand the calibration set? How do you measure validator precision and recall during dry-run mode with limited ground truth?

2. **Specific failure modes the current architecture doesn't handle.** What happens when validators are confidently wrong? What happens when the run manifest is self-consistent but factually incorrect? What happens when validator performance degrades silently? What failure modes exist at the boundary between OQG and validator responsibilities?

3. **Ideas for making run manifests more validator-friendly without making them harder for humans to read.** The manifest serves dual audiences: human operators (who want narrative context) and validator agents (who need structured anchors for judgment). Are there structural additions to the manifest format that help validators without hurting readability?

4. **Proposals for validator health monitoring beyond simple approval/rejection rates.** Approval rate alone doesn't tell you if validators are catching real problems or rubber-stamping everything. What metrics distinguish "healthy validator" from "useless validator" from "dangerous validator"?

5. **Ideas for the relationship between validation and skill promotion.** The architecture says pre-stable skills are always validated. But how do validation verdicts feed back into promotion decisions? Should consecutive rejections block promotion? Should validator accuracy be a promotion gate for the VALIDATOR skill itself? How do you promote a validator skill if you don't have ground truth for its verdicts?

6. **Ideas I haven't asked about.** Especially around: validator drift detection (validators becoming stale as sources evolve), the interaction between validation and the continuation/multi-segment model, whether validators should see cross-run context (last N manifests for the same skill/source), and whether the dry-run -> enforcement transition should be per-skill rather than system-wide.

Provide concrete, implementable recommendations. Where you propose changes to the current plan, frame them as git-diff-style modifications to specific document sections (e.g., "In SS5.2 Validation Executor, add..." or "In SS5.6 step 12c, modify...").

---

## Context Documents to Attach

When sending this handoff to a reviewer, attach:

1. **The full research document** (`docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`) — the reviewer needs the complete document (~7600 lines) but should focus attention on:
   - **SS4.10** — Skills and browser automation, skill lifecycle, production skill directory standard, skill authoring guide
   - **SS5.2** — Component designs, especially Run Manifest Writer, Validation Policy Engine, Validation Executor, and the Architectural Coherence Notes
   - **SS5.6** — Ingestion Data Flow, especially steps 12a-12d for the validation sub-flow
   - **SS5.11** — Error handling strategy, especially Validation-Specific Error Types and Skill Outcome Classes
   - **SS6.1** — Complexity analysis
   - **SS6.2** — Risk assessment
   - **SS6.5** — Testing strategy, especially Validator Tests and Skill Testing sections
   - **SS6.6** — Operational observability, especially the Four-Layer Observability Model (Layer 4 is the run manifest)
   - **SS8.2** — File reference map, especially the Skills section (validator skills) and Rust Daemon section (validation modules)
   - **SS8.4** — Milestone assignment map, especially Milestones 3 (dry-run) and 4 (enforcement)
   - **SS11** — Next steps, especially Milestones 3 and 4 descriptions

2. **AGENTS.md** — for project constraints and toolchain.
