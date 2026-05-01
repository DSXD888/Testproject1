---
skill_id: manifest-generation
skill_version: 1.0.0
interaction_profile: headless_contract
result_schema_version: 1.0.0
validation_policy: always
validation_sample_rate: 10000
validation_triggers:
  - fill_rate_drop
  - first_run_after_version_change
  - suspect_output_in_last_five
  - operator_requested
  - source_health_amber_or_red
  - page_fingerprint_unknown
  - continuation_chain_length_gt_one
  - high_value_opportunity
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: none
maxPages: 0
maxScrollCycles: 0
---

# manifest-generation

## Use when
Use this skill when the daemon needs a deterministic, process-scoped opportunity manifest synthesized from local pipeline data only.

## Do not use when
Do not use this skill for browser navigation, live source ingestion, supplier enrichment, or any workflow that depends on external auth or side effects.

## Inputs
- A single `job_parameters` JSON object with `stage: "manifest"` and one `process_id`.
- The daemon supplies process header data, Stage 12 decision summary, per-candidate detail, coverage metrics, deterministic `stoplight`, deterministic `suggested_action`, deterministic `manifest_basis_hash`, and deterministic `clarification_prompts`.
- Treat every deterministic field from the bundle as authoritative. Do not infer, re-rank, re-count, or reclassify candidates.

## Outputs
- Write `result.json` in the working directory.
- After `result.json` is complete, stop. Do not create `run-manifest.md`, do not create cleanup scripts, and do not delete any files.
- The result payload must satisfy `result.schema.json`.
- `manifestMarkdown` must be a full markdown opportunity manifest, including YAML frontmatter followed by sections A-F:
  - `## Header`
  - `## Decision Summary`
  - `## Per-Candidate Detail`
  - `## Operator Input Requests`
  - `## Evidence Trail`
  - `## Coverage Metrics`
- The YAML frontmatter must include:
  - `manifest_version`
  - `process_id`
  - `basis_hash`
  - `stoplight`
  - `suggested_action`
  - `renderer_mode: skill`
  - `clarification_prompts`
- The result fields `process_id`, `decisionBasisHash`, `stoplight`, `suggested_action`, `clarification_prompts`, `candidate_count`, `priced_candidate_count`, and `clarification_prompt_count` must be copied from `job_parameters` unchanged.
- `clarification_prompts` is a strict pass-through field. Copy every object from `job_parameters.clarification_prompts` exactly, including order, `options`, nulls, text, and reason codes. Never emit `[]` unless `job_parameters.clarification_prompts` is empty.
- The count fields must match the copied arrays: `candidate_count = job_parameters.candidate_count`, `priced_candidate_count = job_parameters.priced_candidate_count`, and `clarification_prompt_count = job_parameters.clarification_prompt_count`.
- `decisionBasisHash` is the input bundle's `manifest_basis_hash`.
- `suggested_action_reasoning` is the only creative field. Write 1-2 natural-language sentences explaining why the daemon's stoplight and action are recommended.
- The daemon supplies `job_parameters.required_reasoning_anchor`. The `suggested_action_reasoning` value MUST start with that exact literal token/string followed by a colon, for example: `no_pricing_signal: ...` or `awaiting_rfq_quote: ...`.
- Do not paraphrase, lowercase/uppercase differently, pluralize, or replace `job_parameters.required_reasoning_anchor`. Copy it exactly as the first text in the reasoning field.
- Do not replace reason codes with prose such as "no usable supplier quotes"; the validator checks for exact candidate names or exact reason-code tokens.

## Non-goals
- Live browsing.
- Incremental cursor management.
- Any write outside the local manifest/result contract surfaces.
- Computing stoplights, action labels, prompt mappings, coverage, or candidate counts.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_INTERNAL_ERROR

Return no result if you cannot comply exactly. The daemon will classify schema mismatches or pass-through drift as `AGENT_OUTPUT_CONTRACT_INVALID`. If the reasoning paragraph is generic, hallucinated, or not anchored to a candidate name or reason code from the bundle, the daemon will classify it as `AGENT_SUSPECT_OUTPUT`.

## Rendering Contract
Build the markdown from the supplied bundle only.

The manifest frontmatter must use the deterministic bundle values:

```yaml
---
manifest_version: v1.0-2026-05-01
process_id: <job_parameters.process_id>
basis_hash: <job_parameters.manifest_basis_hash>
stoplight: <job_parameters.stoplight>
suggested_action: <job_parameters.suggested_action>
renderer_mode: skill
clarification_prompts:
  - candidate_id: ...
    candidate_name: ...
    lot_id: ...
    reason_code: ...
    prompt_text: ...
    prompt_unit: ...
    prompt_type: ...
    default_value: null
    rationale: ...
    options: null
---
```

For choice-type clarification prompts, preserve the supplied `options` array exactly. For non-choice prompts, preserve `options: null` when present or omit it if the input omits it.

The markdown body must be comprehensive:

- Header: process_id, notice title, buyer, deadline, contract value, currency, source, notice URL, published and updated timestamps.
- Decision Summary: pipeline_state, review_required, review_reason, verdict counts, Stage 12 decision hash, Stage 13 basis hash, stoplight, suggested action, and reasoning.
- Per-Candidate Detail: every candidate in `job_parameters.candidates`, including priced and no-pricing-signal candidates, verdict, reasons, margin band, selected quote, landed cost, and confidence axes.
- Operator Input Requests: one human-readable entry per `clarification_prompts[]`; if empty, state that no direct operator clarification prompt was generated.
- Evidence Trail: pricing assessment IDs, selected quote IDs and URLs, landed cost IDs, commodity code and exporter basis when present, search attempts, notice URL, and tender document paths if present in the bundle.
- Coverage Metrics: every field from `job_parameters.coverage` plus `candidate_count`, `priced_candidate_count`, and `clarification_prompt_count`.

For unpriced processes, still produce all sections. State in Decision Summary that there are no priced candidates yet and operator manual evaluation is required.

## Result Payload Template
Use this shape exactly:

```json
{
  "process_id": "<same as input>",
  "decisionBasisHash": "<input manifest_basis_hash>",
  "manifestMarkdown": "---\\n...\\n---\\n# ...",
  "stoplight": "<same as input>",
  "suggested_action": "<same as input>",
  "suggested_action_reasoning": "<job_parameters.required_reasoning_anchor>: one or two evidence-anchored sentences.",
  "clarification_prompts": "<copy job_parameters.clarification_prompts exactly>",
  "candidate_count": "<copy job_parameters.candidate_count>",
  "priced_candidate_count": "<copy job_parameters.priced_candidate_count>",
  "clarification_prompt_count": "<copy job_parameters.clarification_prompt_count>"
}
```

Write that JSON object to `result.json` in the current working directory. Do not rely on printing the JSON to stdout as the only result surface.

## Resume Strategy
Resume is not applicable. This skill is single-process and deterministic over local inputs, so the daemon should rerun the whole contract step rather than partial continuation.

## Pagination Bounds
- `maxPages`: 0
- `maxScrollCycles`: 0

## Run Manifest Instructions
Run manifest generation is not part of this skill. The daemon captures the session artifacts and audit rows. Do not write `run-manifest.md`; only write `result.json`.

The daemon-owned run manifest still uses the standard fields `manifest_version`, `skill_id`, `skill_version`, `job_id`, `session_id`, and `execution_mode`, with narrative sections `Summary`, `Steps Taken`, `Decisions Made`, `Warnings`, and `Evidence`. These names are listed here only so the daemon prompt/lint contract stays explicit; the skill must not create the run manifest file.
