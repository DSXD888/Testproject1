# Stage 13 — Manifest Generation (Build, Validate, Test, Fix in Loop)

ULTRATHINK and be meticulous.

## MISSION

Build Stage 13 (Manifest Generation) end-to-end against the live DB. Today there are 7 processes with Stage 12 `pipeline_assessments` rows (all `pipeline_state='net_qualified'`) and zero `manifests` rows. Stage 13 turns each of those 7 processes into:

1. A **markdown opportunity manifest** stored as a file artifact AND in the `manifests.markdown` column (Q1 hybrid).
2. **Comprehensive content** spanning header, decision summary, per-candidate detail, operator-input requests, evidence trail, and coverage metrics (Q2 comprehensive).
3. A **deterministic stoplight badge + suggested action label** computed by the daemon, plus a **natural-language reasoning paragraph** rendered by the manifest-generation skill (Q3 + Q5 hybrid: facts deterministic, narrative LLM-rendered).
4. **Always generated** — even un-priced processes get a stub manifest so the operator sees them (Q4 always-generate).
5. **Structured clarification prompts** in the markdown frontmatter AND as a JSON column, so Stage 14's UI can bind directly to them (Q6 hybrid: free-form markdown + structured prompts).

You are not just building it. You will **build → validate → test → fix in a loop** until every numbered acceptance criterion (R.1–R.18 below) is GREEN. Do not stop with any criterion failing. Do not narrow the test gate to make it green.

```
┌──────────────────────────────────────────────────────────────────┐
│                          THE LOOP                                  │
│                                                                    │
│   1. Pre-flight (verify the facts in §0 below)                    │
│   2. Implement: migration + register + result schema + Rust       │
│      + dispatch + skill prompt updates + tests                    │
│   3. Run trigger: cargo run -p daemon -- --ingest manifest        │
│   4. Validate against R.1..R.18 (queries inline below)            │
│   5. Any criterion failing?                                       │
│      ├─ YES → diagnose root cause, fix, clear stage tracking      │
│      │        if needed, go back to step 3                        │
│      └─ NO  → run quality gates (fmt + check + clippy + binary    │
│                unit tests + per-file integration loop). Failures? │
│                ├─ YES → fix, go to step 4                         │
│                └─ NO  → write the validation manifest             │
│                                                                    │
│   DO NOT STOP UNTIL ALL R.* PASS AND ALL QUALITY GATES ARE GREEN. │
└──────────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first. Pay particular attention to `## Compiler Checks (CRITICAL)` and `### Stage Validation Test Gate` — the binary-unit-test-only command is **insufficient**; the per-file integration loop is mandatory.

---

## §0 — PRE-FLIGHT (verify before you start)

These facts were confirmed against the live tree at handoff authoring time. Reconfirm before you write a line of code; if anything has drifted, adjust accordingly and note it in the manifest.

| Claim | Verify command |
|---|---|
| Migration directory is `migrations/` at **repo root**; latest is `0007_net_margin_gate_assessments` (epoch 4) | `ls migrations/` |
| `manifests` table **already exists** in `migrations/0001_sqlite_state_initial.sql` with columns: `id, process_id, markdown, artifact_hash, artifact_locator, owner_id, row_version, last_local_seq, stage_run_id, updated_at, created_at`. **No stoplight / suggested_action / basis_hash columns yet — Stage 13 adds these in 0008** | `sqlite3 state/state.db ".schema manifests"` |
| `manifests` table is **currently empty** (Stage 13 is the first production writer) | `sqlite3 state/state.db "SELECT COUNT(*) FROM manifests"` |
| 7 processes have `pipeline_assessments` rows (Stage 12 rollup) all in `pipeline_state='net_qualified'` | `sqlite3 state/state.db "SELECT pipeline_state, COUNT(*) FROM pipeline_assessments GROUP BY pipeline_state"` |
| `skills/manifest-generation/` exists with `SKILL.md` + `result.schema.json` + canary inputs + golden assertions. Result schema currently requires only `process_id`, `decisionBasisHash`, `manifestMarkdown` — **Stage 13 extends the schema** | `cat skills/manifest-generation/result.schema.json` |
| Existing `ManifestComputation` struct at `crates/daemon/src/main.rs:752` and `build_manifest_from_db` at `main.rs:13223` are the legacy FTS-driven manifest path. They write to `manifests` (deterministic, daemon-only, no skill invocation). **Do NOT modify them** — Stage 13 introduces a new production path alongside | `grep -n "fn build_manifest_from_db\|struct ManifestComputation" crates/daemon/src/main.rs` |
| Existing `INSERT INTO manifests` at `main.rs:12818` (inside `finalize_success`) uses deterministic ID `format!("manifest-{process_id}")`. Stage 13 must follow the same ID convention so the two writers cannot create duplicate rows | `grep -n 'INSERT INTO manifests\|format!("manifest-' crates/daemon/src/main.rs` |
| `execute_skill_session(runtime, skill, job_id, session_id, job_parameters, cli)` at `main.rs:8154` is the canonical skill invocation helper. It returns an `ExecutedSession` and writes to the artifact store. Stage 13 uses this directly | `grep -n "async fn execute_skill_session" crates/daemon/src/main.rs` |
| `validate_result_schema` is **already imported** at `main.rs:138` and used (e.g., line 4017), but it is **shallow** — see `crates/daemon/src/agents/contracts.rs`. It only checks: payload-is-object + required-fields-present + top-level type matches (`type: "string"` etc.). It does **NOT** enforce `enum`, `minLength`/`maxLength`, or `items` constraints inside arrays. Stage 13 must do additional validation in Rust (e.g., deserialize `stoplight` into a `#[derive(Deserialize)]` enum so invalid strings fail; deserialize `clarification_prompts` into `Vec<ClarificationPrompt>` for nested validation; assert `suggested_action_reasoning.len() <= 2000` manually) | `grep -A30 "fn validate_result_schema" crates/daemon/src/agents/contracts.rs` |
| `run_fts_live_loop` is **production code** (not smoke-test scaffolding) — it's dispatched from `--ingest fts` at `main.rs:~1167`. It runs the full FTS live loop including manifest generation via the legacy `build_manifest_from_db` + `finalize_success` path. Currently no FTS data has been ingested (all 133 processes are ProContract), so the legacy manifest path has produced 0 rows in production. Stage 13's new path supersedes the legacy path for all-source manifest generation | `grep -B2 -A2 'run_fts_live_loop' crates/daemon/src/main.rs` |
| `load_skill(skill_root, relative_skill_path)` at `main.rs:13456` returns a `LoadedSkill`. Stage 13 calls it for `skills/manifest-generation` | `grep -n "fn load_skill" crates/daemon/src/main.rs` |
| `pdf_t_schema_activation_009_default_schema_epoch_is_4` exists at `crates/daemon/tests/schema_activation.rs:1108` and asserts `DEFAULT_SCHEMA_EPOCH == 4` AND `user_version == 4`. **This test WILL FAIL when you bump epoch to 5 in §2.2 — you must update or replace it** (see §2.2 step 4) | `grep -n "default_schema_epoch_is_" crates/daemon/tests/schema_activation.rs` |
| `persist_stage(...)` helper at `main.rs:14196` is the canonical stage-tracking writer. Use it for `process_stage_status` + `process_stage_heads` writes | `grep -n "fn persist_stage(" crates/daemon/src/main.rs` |
| `sha2 = "0.10.9"` is in `crates/daemon/Cargo.toml` — sha256 hashing is already available, no new dep needed | `grep "^sha2" crates/daemon/Cargo.toml` |
| `serde_yaml` or `serde_yml` is in dependencies (needed if Stage 13 parses YAML frontmatter from skill output) | `grep -E "serde[_-]y" crates/daemon/Cargo.toml` |
| ProContract data has 4 priced + 13 searched-unpriced candidates across 7 processes (Stage 12 input universe) | inline query in §3 |

If any drift, the rest of this handoff still holds — just update the matching numbers.

---

## §1 — DECISIONS ALREADY MADE (DO NOT RE-OPEN)

The operator has signed off on D1–D6. Implement to spec; do not deviate.

### D1 — Storage: hybrid markdown artifact + DB row

- Markdown file artifact: written to `artifacts/manifests/{process_id}/manifest.md` (per-process directory)
- DB row: `manifests` table, one row per process, `id = format!("manifest-{process_id}")`, `ON CONFLICT(id) DO UPDATE` for re-runs (no per-run history)
- `manifests.markdown` mirrors the file contents; `manifests.artifact_locator` is the relative file path; `manifests.artifact_hash` is the sha256 of the markdown bytes
- File-vs-DB drift: the DB is the source of truth; the file is a human-readable mirror. If they ever drift, regenerate the file from the DB

### D2 — Content scope: comprehensive everywhere

The manifest contains all six sections, for every process (priced or unpriced):

- **A. Header** — process_id, notice title, buyer, deadline, contract value (gbp_pence + currency), source (FTS / Contracts Finder / ProContract), notice URL, published_at, source_updated_at, freshness timestamps
- **B. Decision summary** — pipeline_state, review_required, review_reason, per-candidate verdict counts (PASS / REVIEW / REJECT), Stage 12's `decision_basis_hash`, Stage 13's `manifest_basis_hash`
- **C. Per-candidate detail** — for every candidate (priced AND no-pricing-signal): candidate name, lot title, candidate_type, verdict, full reasons array, margin band (if priced), selected supplier quote (price, source, region), landed cost breakdown (if computed), confidence axes
- **D. Operator input requests** — one entry per `re_evaluation_eligible=1` candidate, mapped from reason codes (see §2.7 mapping table). Both human-readable in the markdown body AND structured JSON in the frontmatter (per Q6 hybrid). Candidates with only `confidence_*_below_floor` reasons get NO clarification prompt (operator can't manually fix confidence)
- **E. Evidence trail** — IDs and links: `landed_cost_estimate_id` (HMRC commodity code, freight quote URL, exporter rate basis), `selected_quote_id` (source URL, raw price, currency, region), `pricing_assessment_id`, source notice URLs, tender document paths if available
- **F. Coverage metrics** — full set from `pipeline_assessments`: priced/landed/net coverage (count-weighted AND value-weighted), `unresolved_mandatory_lots`, `gross_margin_bps`, `net_margin_bps`

For un-priced processes (4 of 7 today): sections A, B, C (showing all `no_pricing_signal` candidates), D (manual-pricing prompts for each), F (zero coverage), with E showing what we tried (search_attempts evidence). Section B explicitly says "no priced candidates yet — operator manual evaluation required". This is the "stub for unpriced" the operator asked for.

### D3 — Stoplight + suggested action with reasoning

Two computed fields persist alongside the markdown:

- **`stoplight`** ∈ `{'pursue', 'clarify', 'pass', 'review'}` — daemon-computed, deterministic
- **`suggested_action`** — daemon-computed short label (e.g., "Clarify quantity basis for Aggregates")
- **`suggested_action_reasoning`** — skill-rendered natural-language paragraph (1–2 sentences) explaining WHY the stoplight + action are recommended

**Stoplight derivation rules** (apply in order; first match wins):

| Condition | Stoplight |
|---|---|
| ANY candidate `verdict='pass'` AND `priced_coverage_bps >= 5000` | `'pursue'` |
| ANY candidate `re_evaluation_eligible=1` (operator input could change verdict) | `'clarify'` |
| ALL candidates `verdict='reject'` OR (`priced_coverage_bps=0` AND no `re_evaluation_eligible`) | `'pass'` |
| Otherwise | `'review'` |

For today's 7 processes: every process has at least one `re_evaluation_eligible=1` candidate (every Stage 12 row has it set), so all 7 should land at `stoplight='clarify'`.

**Suggested action label rules** (deterministic):

| Stoplight | Label |
|---|---|
| `pursue` | `"Pursue this opportunity"` |
| `clarify` | `format!("Clarify {} candidate(s) before deciding", N)` where N is the count of `re_evaluation_eligible=1` |
| `pass` | `"Pass on this opportunity"` |
| `review` | `"Operator review required"` |

**Reasoning text** is produced by the skill (LLM) given the structured bundle. The daemon does NOT generate the reasoning text.

### D4 — Always generate

Every process with a `pipeline_assessments` row gets a manifest, regardless of coverage. Un-priced processes get a "stub" manifest (per D2 above). The pre-Stage-12 cohort (processes that have not yet been through Stage 12) is **not** processed by Stage 13 — Stage 13's input set is processes with `pipeline_assessments` rows, full stop.

### D5 — Skill role: hybrid (renders markdown + writes reasoning)

The split between daemon and skill:

| Concern | Owner |
|---|---|
| Loading process / candidate / pricing / landed-cost data from DB | Daemon |
| Computing deterministic stoplight per D3 rules | Daemon |
| Computing deterministic suggested_action label per D3 table | Daemon |
| Computing per-candidate clarification_prompts (deterministic mapping per §2.7) | Daemon |
| Computing manifest_basis_hash | Daemon |
| Rendering the markdown body (sections A–F formatted) | **Skill** |
| Writing the natural-language `suggested_action_reasoning` paragraph | **Skill** |
| Validating the skill output against the extended `result.schema.json` | Daemon |
| Persisting the artifact + DB row | Daemon |

Why this split: the skill's only LLM-creative work is the reasoning paragraph. Everything decision-relevant (stoplight, action label, prompts, hash) is deterministic. If the skill's prompt regresses or drifts, the deterministic facts in the DB are still correct — only the natural-language wrapping degrades, and the operator can still act on the structured fields.

### D6 — Operator clarification scaffolding: hybrid

For each `candidate_net_margin_assessments` row with `re_evaluation_eligible=1`, Stage 13 emits a structured `clarification_prompt` object:

```json
{
  "candidate_id": "pc-...",
  "candidate_name": "Double-glazed unit (...)",
  "lot_id": "lot-..." | null,
  "reason_code": "quantity_basis_unaligned",
  "prompt_text": "Provide actual order quantity (Stage 11 inferred basis is misaligned with revenue scope)",
  "prompt_unit": "square meter",
  "prompt_type": "number",
  "default_value": null,
  "rationale": "Selected quote's price_basis_qty=10000 is supplier capacity, not order qty"
}
```

These appear in **both**:
1. The markdown body (a `## Operator Input Requests` section, human-readable)
2. The frontmatter (as `clarification_prompts: [...]` YAML)
3. A new column on `manifests`: `clarification_prompts_json TEXT NOT NULL DEFAULT '[]'`

Stage 14 (later handoff) will read either the frontmatter or the JSON column to render the operator UI. Both sources must be byte-equivalent (the daemon serializes once and writes to both).

---

## §2 — DESIGN

```
                    pipeline_assessments  (Stage 12 rollup, 7 rows)
                              │
                              ▼
                    Stage 13 dispatcher (--ingest manifest)
                              │
        ┌─────────────────────┼─────────────────────────┐
        │  daemon assembles bundle (D5 daemon column)   │
        │  + computes stoplight + action + prompts      │
        └─────────────────────┬─────────────────────────┘
                              │ job_parameters (JSON bundle)
                              ▼
              skills/manifest-generation (renders markdown + writes reasoning)
                              │ result.json
                              ▼
              daemon validates schema + persists
              ├─→ artifacts/manifests/{process_id}/manifest.md  (file)
              ├─→ manifests row (DB)
              ├─→ pipeline_assessments.pipeline_state = 'manifest_ready'
              └─→ process_stage_status / process_stage_heads (scope_kind='process')
```

### §2.1 — New migration: `migrations/0008_manifest_generation_columns.sql`

Extend the existing `manifests` table with new columns. Do NOT recreate the table; use `ALTER TABLE`. **Do NOT include `BEGIN;` / `COMMIT;` in the file** — `apply_sql_file` (in `crates/daemon/src/db/sqlite.rs`) already wraps every migration in `SAVEPOINT state_db_migration`. An inner transaction wrapper conflicts with the savepoint and the migration fails. Stage 12's `migrations/0007_net_margin_gate_assessments.sql` is the canonical pattern — bare DDL statements, no transaction wrapper.

```sql
ALTER TABLE manifests ADD COLUMN manifest_basis_hash TEXT;
ALTER TABLE manifests ADD COLUMN stoplight TEXT;
ALTER TABLE manifests ADD COLUMN suggested_action TEXT;
ALTER TABLE manifests ADD COLUMN suggested_action_reasoning TEXT;
ALTER TABLE manifests ADD COLUMN clarification_prompts_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE manifests ADD COLUMN manifest_template_version TEXT NOT NULL DEFAULT 'v1.0-2026-05-01';
ALTER TABLE manifests ADD COLUMN candidate_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE manifests ADD COLUMN priced_candidate_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE manifests ADD COLUMN clarification_prompt_count INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_manifests_stoplight ON manifests(stoplight);
CREATE INDEX IF NOT EXISTS idx_manifests_basis ON manifests(manifest_basis_hash);
```

All new columns are NULLable OR have safe defaults so the legacy FTS-driven manifest path (`finalize_success` at `main.rs:12771`) continues to work without modification — its rows will have NULL `stoplight`, NULL `manifest_basis_hash`, etc. Production Stage 13 always populates them.

### §2.2 — Migration registration (REQUIRED, easy to forget)

In `crates/daemon/src/db/sqlite.rs`, follow the exact pattern used for `0007`:

1. Add a constant near `L_NET_GATE_MIGRATION_PATH`:
   ```rust
   const L_MANIFEST_MIGRATION_PATH: &str = "migrations/0008_manifest_generation_columns.sql";
   ```
2. Add an entry to the `MIGRATIONS` array (after `0007`):
   ```rust
   ("0008_manifest_generation_columns", 5, L_MANIFEST_MIGRATION_PATH),
   ```
   Schema epoch is `5`. (`0007`=4, so `0008`=5.)
3. Update `pub const DEFAULT_SCHEMA_EPOCH: i64` to `5`.
4. **Update or replace `pdf_t_schema_activation_009_default_schema_epoch_is_4`** in `crates/daemon/tests/schema_activation.rs:1108`. That test hard-codes `DEFAULT_SCHEMA_EPOCH == 4` and `user_version == 4`; once you bump to 5 it will fail. Either rename to `_default_schema_epoch_is_5` and assert `== 5`, OR add a new test for the new value. Do not delete without replacement.

Without these four steps, the migration won't run and/or `cargo test` will go red on an unrelated-looking test (same gotcha as Stage 12).

### §2.3 — Extend `skills/manifest-generation/result.schema.json`

The current schema requires only 3 fields. Extend it to accept the new structured outputs from the skill:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": [
    "process_id",
    "decisionBasisHash",
    "manifestMarkdown",
    "stoplight",
    "suggested_action",
    "suggested_action_reasoning",
    "clarification_prompts",
    "candidate_count"
  ],
  "properties": {
    "process_id": { "type": "string" },
    "decisionBasisHash": { "type": "string" },
    "manifestMarkdown": { "type": "string" },
    "stoplight": { "type": "string", "enum": ["pursue", "clarify", "pass", "review"] },
    "suggested_action": { "type": "string", "minLength": 1 },
    "suggested_action_reasoning": { "type": "string", "minLength": 1, "maxLength": 2000 },
    "clarification_prompts": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["candidate_id", "candidate_name", "reason_code", "prompt_text", "prompt_type"],
        "properties": {
          "candidate_id": { "type": "string" },
          "candidate_name": { "type": "string" },
          "lot_id": { "type": ["string", "null"] },
          "reason_code": { "type": "string" },
          "prompt_text": { "type": "string" },
          "prompt_unit": { "type": ["string", "null"] },
          "prompt_type": { "type": "string", "enum": ["number", "object", "choice"] },
          "default_value": {},
          "rationale": { "type": ["string", "null"] }
        }
      }
    },
    "candidate_count": { "type": "integer", "minimum": 0 },
    "priced_candidate_count": { "type": "integer", "minimum": 0 },
    "clarification_prompt_count": { "type": "integer", "minimum": 0 }
  }
}
```

The `decisionBasisHash` field maps to Stage 13's `manifest_basis_hash` (different from `pipeline_assessments.decision_basis_hash`; the latter is Stage 12's hash, the former is Stage 13's).

### §2.4 — Update `skills/manifest-generation/SKILL.md`, canary input, and golden assertions

The skill receives the structured input bundle (§2.5) as `job_parameters` and must produce the extended result schema. Update SKILL.md to document the new contract:

- Inputs: structured bundle with process header, candidate detail, coverage, deterministic stoplight + action + prompts (passed in by daemon)
- Outputs: extended `result.json` per §2.3, where:
  - `manifestMarkdown` is the full rendered markdown WITH frontmatter (`---\n...\n---\n# Title\n...`)
  - `stoplight`, `suggested_action`, `clarification_prompts`, `candidate_count`, etc. mirror the input bundle (the skill passes them through unchanged — the skill MUST NOT re-derive these)
  - `suggested_action_reasoning` is the only LLM-creative output: a 1–2 sentence natural-language explanation of why the stoplight + action are recommended, anchored to specific evidence from the bundle
- Failure modes: AGENT_OUTPUT_CONTRACT_INVALID (schema mismatch), AGENT_SUSPECT_OUTPUT (reasoning is generic / hallucinated / not anchored)

**Also update `skills/manifest-generation/canary.inputs.jsonl`** — current content is the stub `{"process_id":"process-123","candidate_count":2}` (49 bytes). With the extended contract, the canary needs a representative full bundle so the skill linter / canary harness can verify the skill produces a schema-compliant output. Replace with at least one complete bundle that covers the most-common case (a process with at least 1 priced candidate + 1 actionable reason → at least 1 clarification prompt).

**Also update `skills/manifest-generation/golden.assertions.yaml`** — current assertions cover the agent's run-manifest sections (`Summary, Steps Taken, Decisions Made, Warnings, Evidence`). Add NEW assertions for the result-payload contract:

```yaml
required_result_fields:
  - process_id
  - decisionBasisHash
  - manifestMarkdown
  - stoplight
  - suggested_action
  - suggested_action_reasoning
  - clarification_prompts
  - candidate_count
required_markdown_sections:
  - "## Header"
  - "## Decision Summary"
  - "## Per-Candidate Detail"
  - "## Operator Input Requests"
  - "## Evidence Trail"
  - "## Coverage Metrics"
required_frontmatter_fields:
  - manifest_version
  - process_id
  - basis_hash
  - stoplight
  - suggested_action
  - clarification_prompts
```

The golden harness (`crates/daemon/tests/skill_golden_journeys.rs`) reads these assertions; if you add assertions that exceed what the skill outputs, the test fails — keep the assertions in lockstep with the skill output.

### §2.5 — Skill input bundle (`job_parameters` shape)

Daemon assembles this for each process and passes it via `job_parameters` to `execute_skill_session`:

```json
{
  "stage": "manifest",
  "process_id": "proc-pc-...",
  "manifest_template_version": "v1.0-2026-05-01",
  "manifest_basis_hash": "<sha256>",
  "stoplight": "clarify",
  "suggested_action": "Clarify 1 candidate(s) before deciding",
  "header": {
    "notice_id": "pc-...",
    "title": "...",
    "buyer_name": "...",
    "deadline_at": "2026-...",
    "contract_value_pence": 167222200,
    "currency": "GBP",
    "source_id": "procontract",
    "source_notice_id": "...",
    "published_at": "...",
    "source_updated_at": "..."
  },
  "decision_summary": {
    "pipeline_state": "net_qualified",
    "review_required": 1,
    "review_reason": "quantity_basis_undetermined",
    "stage12_decision_basis_hash": "...",
    "verdict_counts": { "pass": 0, "review": 1, "reject": 0 }
  },
  "candidates": [
    {
      "candidate_id": "pc-d23508ec-...",
      "candidate_key": "...",
      "candidate_name": "Aggregates (general...)",
      "candidate_type": "sku",
      "lot_id": "lot-...",
      "lot_title": "...",
      "verdict": "review",
      "reasons": ["quantity_basis_undetermined"],
      "re_evaluation_eligible": 1,
      "margin_band": { "low_bps": 9999, "base_bps": 9999, "high_bps": 9999 },
      "revenue_used_pence": 16722222,
      "revenue_basis_type": "expected_value",
      "economic_landed_cost_pence": 1540,
      "confidence": {
        "revenue_bps": 6500,
        "pricing_input_bps": 5250,
        "customs_resolution_bps": 10000,
        "commercial_estimate_bps": 4000
      },
      "selected_quote": {
        "supplier_quote_id": "sq-...",
        "source_name": "amazon-uk",
        "region": "GB",
        "unit_price_gbp_pence": 88,
        "price_basis_qty": 17.5,
        "price_basis_unit": "kg",
        "source_url": "..."
      },
      /* effective_cost is derived (not a stored column on supplier_quotes):
         - For a priced candidate, use landed_cost_estimates.supplier_cost_gbp_pence
           (which Stage 11 computed from unit_price × quantity_basis × discount).
         - For unpriced candidates, all selected_quote fields are null. */
      "landed_cost": {
        "landed_cost_estimate_id": "landed-...",
        "shipping_mode": "domestic",
        "supplier_cost_gbp_pence": 1540,
        "freight_base_gbp_pence": 0,
        "customs_duty_gbp_pence": 0,
        "import_vat_gbp_pence": 0,
        "economic_landed_cost_gbp_pence": 1540,
        "declarable_commodity_code_10": null
      }
    }
    /* ... and any no_pricing_signal candidates with margin/cost/quote = null */
  ],
  "coverage": {
    "priced_coverage_bps": 5000,
    "landed_cost_coverage_bps": 5000,
    "net_qualified_coverage_bps": 0,
    "priced_value_coverage_bps": 5000,
    "landed_cost_value_coverage_bps": 5000,
    "net_qualified_value_coverage_bps": 0,
    "unresolved_mandatory_lots": 1,
    "gross_margin_bps": 9999,
    "net_margin_bps": 9999
  },
  "clarification_prompts": [
    {
      "candidate_id": "pc-d23508ec-...",
      "candidate_name": "Aggregates (general...)",
      "lot_id": "lot-...",
      "reason_code": "quantity_basis_undetermined",
      "prompt_text": "Provide framework annual usage estimate",
      "prompt_unit": "kg",
      "prompt_type": "number",
      "default_value": null,
      "rationale": "Revenue is framework total (£167,222) but supplier_cost is per-pack (17.5 kg). Cannot compute meaningful net margin without quantity scaling."
    }
  ]
}
```

Reasons for putting BOTH stoplight AND clarification_prompts in the input bundle (not just having the skill compute them):
- The deterministic policy lives in the daemon (one source of truth)
- The skill's job is rendering, not deciding
- If you add T.* tests for the daemon's stoplight/prompt logic (per §4), they don't need to mock the LLM

### §2.6 — Daemon-side mapping for `clarification_prompts`

Per §1 D6, deterministic mapping from reason code → prompt:

| Reason code | prompt_text | prompt_unit | prompt_type | rationale source |
|---|---|---|---|---|
| `quantity_basis_undetermined` | `"Provide framework annual usage estimate"` | from `supplier_quotes.price_basis_unit` | `"number"` | Revenue scope and cost scope mismatch; provide quantity to scale cost |
| `quantity_basis_unaligned` | `"Provide actual order quantity (Stage 11 inferred basis is misaligned)"` | from `supplier_quotes.price_basis_unit` or `landed_cost_estimates.confidence_breakdown.order_quantity_basis.unit` | `"number"` | Stage 11's inferred quantity basis is `rfq_required` or `supplier_capacity` — not the buyer's order |
| `awaiting_rfq_quote` | `"Provide RFQ quote: per-unit price (GBP), lead time (days), MOQ"` | NULL | `"object"` | revenue_basis_type='rfq_required' — Stage 10 has no addressable revenue without an RFQ |
| `no_pricing_signal` | `"Manually price candidate or flag as un-pursueable"` | NULL | `"choice"` (with options: `manual_price`, `un_pursueable`, `defer`) | Stage 9 produced no quote; operator must price by hand or defer |
| `confidence_revenue_below_floor` | NO PROMPT | — | — | Operator can't manually fix confidence — informational only |
| `confidence_pricing_input_below_floor` | NO PROMPT | — | — | same |
| `confidence_customs_resolution_below_floor` | NO PROMPT | — | — | same |
| `confidence_commercial_estimate_below_floor` | NO PROMPT | NO PROMPT | — | same |

A candidate with multiple reasons yields multiple prompts (one per actionable reason). A candidate with only `confidence_*_below_floor` reasons yields ZERO prompts — even though it's `re_evaluation_eligible=1`, there's nothing for the operator to clarify directly.

For DGU today: 4 reasons (`quantity_basis_unaligned`, `awaiting_rfq_quote`, `confidence_pricing_input_below_floor`, `confidence_revenue_below_floor`) → 2 prompts (the first two).

### §2.7 — Re-runnability via `manifest_basis_hash`

```
manifest_basis_hash = sha256(
  process_id || ':' ||
  pipeline_assessments.decision_basis_hash || ':' ||
  manifest_template_version
)
```

`pipeline_assessments.decision_basis_hash` is composed by Stage 12 from all child candidate basis_hashes — so any change to upstream Stage 10/11/12 cascades up. If Stage 13's freshly-computed `manifest_basis_hash` matches what's already persisted on `manifests`, **skip** (idempotent no-op). Otherwise upsert (and rewrite the markdown artifact).

**Skill-output drift detection** (during the upsert path): after `validate_result_schema` (which only checks required fields exist), the daemon must verify the skill **echoed the bundle's pass-through fields unchanged**:

| Field | Source | Drift check |
|---|---|---|
| `stoplight` | Daemon-computed in bundle | `assert_eq!(skill_output.stoplight, bundle.stoplight)` else fail with `AGENT_OUTPUT_CONTRACT_INVALID` reason `stoplight_drifted_from_bundle` |
| `suggested_action` | Daemon-computed | `assert_eq!` |
| `clarification_prompts` | Daemon-computed (canonical JSON) | `assert_eq!` after sorting both by `candidate_id` then `reason_code` for stable comparison |
| `candidate_count`, `priced_candidate_count`, `clarification_prompt_count` | Daemon-computed | `assert_eq!` |
| `process_id`, `decisionBasisHash` (= manifest_basis_hash) | Daemon-computed | `assert_eq!` |
| `manifestMarkdown` | Skill-rendered | NO drift check — but parse the YAML frontmatter and assert that `frontmatter.stoplight`, `frontmatter.basis_hash`, `frontmatter.clarification_prompts` match the bundle (else `AGENT_OUTPUT_CONTRACT_INVALID` reason `frontmatter_drifted_from_bundle`) |
| `suggested_action_reasoning` | Skill-creative (only LLM output) | Length check (1..=2000); non-empty; SHOULD reference at least one specific reason code OR candidate name from the bundle (soft check, T.5 enforces) |

Persistence convention: for pass-through fields, persist from the **bundle** (canonical source), not the skill output, even though they should be byte-equivalent after the drift check passes. For `markdown` and `suggested_action_reasoning`, persist the skill output. This way the DB always reflects the deterministic facts even if a future skill-prompt change introduces drift before tests catch it.

### §2.8 — Stage tracking

- `process_stage_status` / `process_stage_heads`:
  - stage = `'manifest'`, scope_kind = `'process'`, scope_id = `procurement_processes.id` — one per process processed

State is `'succeeded'` on a normal run. Use `persist_stage(...)` helper (§0 pre-flight).

### §2.8a — `pipeline_assessments` UPDATE scope (minimization)

Stage 13 must transition `pipeline_assessments.pipeline_state` from `'net_qualified'` to `'manifest_ready'`, but **must NOT touch any other column**. Stage 12 owns those columns; if Stage 13 re-writes them with stale or wrong values, Stage 12's authoritative state is corrupted.

Use a tightly-scoped UPDATE (preferred over UPSERT for this case, since the row already exists from Stage 12):

```rust
tx.execute(
    "UPDATE pipeline_assessments
     SET pipeline_state = 'manifest_ready',
         updated_at = CURRENT_TIMESTAMP
     WHERE process_id = ?1
       AND pipeline_state IN ('net_qualified', 'manifest_ready')",
    params![process_id],
)?;
```

The `pipeline_state IN (...)` predicate prevents accidentally regressing a state that's already past `'manifest_ready'` (e.g., a future Stage 14 might set `'operator_decided'` — Stage 13 should NOT downgrade that). For today's Stage-12-output rows, `IN ('net_qualified', 'manifest_ready')` matches.

**Note on `'manifest_ready'` value**: this enum value is NOT in L0's documented `pipeline_state` enum (`docs/strategy/layers/layer-0-gold-set-evaluation.md:959`), but is the **established convention** used by the existing `finalize_success` legacy writer at `main.rs:12840`. The column is `TEXT NOT NULL` with no CHECK constraint, so the value is accepted. Stage 13 follows the existing convention; updating L0 docs is out of scope.

### §2.8b — Markdown frontmatter parsing (no existing helper)

There is no existing helper in `crates/daemon/src/` for extracting YAML frontmatter from a markdown string. Stage 13 must build a small parser. Minimal shape:

```rust
fn extract_frontmatter(markdown: &str) -> Result<serde_yaml::Value> {
    let trimmed = markdown.trim_start();
    let after_open = trimmed
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("manifest markdown does not start with `---\\n` frontmatter"))?;
    let close_index = after_open
        .find("\n---\n")
        .ok_or_else(|| anyhow!("manifest markdown frontmatter is not closed with `\\n---\\n`"))?;
    let yaml_text = &after_open[..close_index];
    serde_yaml::from_str(yaml_text)
        .with_context(|| format!("parsing manifest frontmatter:\n{yaml_text}"))
}
```

Use this for the §2.7 frontmatter-vs-bundle drift check. T.21 covers it.

### §2.9 — Dispatch

Add `--ingest manifest` (mirrors `--ingest landed-cost`, `--ingest net-gate`).

```rust
// in the existing match block at crates/daemon/src/main.rs:1161:
"manifest" => run_manifest_generation_live_loop(&mut runtime, &cli).await?,
```

Implement an async loop function:

```rust
async fn run_manifest_generation_live_loop(
    runtime: &mut RuntimeBootstrap,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    let stage_run_id = allocate_stage_run_id();
    let manifest_skill = load_skill(
        &repo_root()?.join("skills/manifest-generation"),
        "skills/manifest-generation/SKILL.md",
    )?;
    // 1. Load input set: all processes with pipeline_assessments rows
    // 2. For each process:
    //    a. Compute manifest_basis_hash; skip if matches persisted
    //    b. Build input bundle (§2.5) — load candidates, quotes, landed cost, coverage
    //    c. Compute stoplight + suggested_action label (§1 D3)
    //    d. Compute clarification_prompts (§2.6)
    //    e. Invoke skill via execute_skill_session(..., job_parameters: bundle)
    //    f. Validate skill output (3 layers per §0 + §2.7):
    //         (i)   validate_result_schema (shallow — required fields + top-level types)
    //         (ii)  Rust-typed deserialization (stoplight enum, Vec<ClarificationPrompt>,
    //               length-bound on suggested_action_reasoning)
    //         (iii) drift detection vs bundle (per §2.7 table — stoplight, suggested_action,
    //               clarification_prompts, candidate_count, manifest_basis_hash)
    //         (iv)  frontmatter-vs-bundle drift via extract_frontmatter (§2.8b)
    //         Any failure → AGENT_OUTPUT_CONTRACT_INVALID with specific reason code.
    //    g. UPSERT manifests with id = format!("manifest-{process_id}") + ON CONFLICT DO UPDATE
    //    h. Write markdown artifact to artifacts/manifests/{process_id}/manifest.md
    //    i. UPDATE pipeline_assessments — TIGHT SCOPE per §2.8a (only pipeline_state +
    //       updated_at; do NOT clobber Stage 12's columns)
    //    j. persist_stage(...) for scope_kind='process'
    // 3. Return summary line
}
```

Summary line shape (matches Stage 12):

```
manifest live loop completed: attempted=N processes_rendered=M
  stoplight_pursue=P stoplight_clarify=C stoplight_pass=X stoplight_review=R
  skipped_unchanged=S
```

**Wiring `--ingest manifest` into `run_deterministic_pipeline` is OUT OF SCOPE for this handoff.** Standalone trigger only.

**Do NOT modify `run_fts_live_loop` or `finalize_success` or `build_manifest_from_db` or `ManifestComputation`** — those are the legacy FTS-driven manifest path. Stage 13 introduces a parallel production path.

---

## §3 — EXPECTED RESULTS AGAINST CURRENT DATA

After a clean run today, the DB should look like this. Use as your reference for verifying R.1–R.18:

**Per-process candidate distribution (verified against live DB at handoff time):**

| Process | Total candidates | Priced | Unpriced | Actionable reasons (= prompts) | Stoplight | Suggested action |
|---|---:|---:|---:|---:|---|---|
| `proc-pc-19574d11-...` (Ely Library carpets) | 5 | 2 | 3 | **5** (2 priced × `awaiting_rfq_quote` + 3 unpriced × `no_pricing_signal`) | `clarify` | `"Clarify 5 candidate(s) before deciding"` |
| `proc-pc-218726bd-...` | 1 | 0 | 1 | **1** (1 unpriced × `no_pricing_signal`) | `clarify` | `"Clarify 1 candidate(s) before deciding"` |
| `proc-pc-3016d0da-...` | 5 | 0 | 5 | **5** (5 unpriced × `no_pricing_signal`) | `clarify` | `"Clarify 5 candidate(s) before deciding"` |
| `proc-pc-3ba4e6b8-...` | 2 | 0 | 2 | **2** (2 unpriced × `no_pricing_signal`) | `clarify` | `"Clarify 2 candidate(s) before deciding"` |
| `proc-pc-7046e8ec-...` (DGU) | 1 | 1 | 0 | **2** (DGU has `quantity_basis_unaligned + awaiting_rfq_quote` actionable; 2 confidence reasons are not actionable per §2.6) | `clarify` | `"Clarify 1 candidate(s) before deciding"` |
| `proc-pc-7e2a6fb2-...` | 1 | 0 | 1 | **1** (1 unpriced × `no_pricing_signal`) | `clarify` | `"Clarify 1 candidate(s) before deciding"` |
| `proc-pc-d23508ec-...` (Aggregates) | 2 | 1 | 1 | **2** (Aggregates priced × `quantity_basis_undetermined` + 1 unpriced × `no_pricing_signal`) | `clarify` | `"Clarify 2 candidate(s) before deciding"` |
| **TOTAL** | **17** | **4** | **13** | **18 prompts** | — | — |

**Important nuance**: A process can have a mix of priced AND unpriced candidates (e.g., Ely Library carpets has 2 priced + 3 unpriced; Aggregates has 1 priced "Aggregates" + 1 unpriced "Asphalts"). The manifest must show both kinds in section C and emit one clarification prompt per actionable reason per candidate.

**Important second nuance**: "Total candidates" in this table is the **Stage 12 input set** (priced + searched-unpriced). The underlying `product_candidates` table can have many more rows per process — e.g. the Aggregates process actually has 10 product_candidates rows (Aggregates, Asphalts, Roof tiles, Bricks, Cement, Slabs, etc.) but only 2 made it into Stage 12 because the others have no `search_attempts` yet. The 8 not-yet-searched candidates are NOT in Stage 13's per-candidate detail (D2 section C); they're surfaced at lot/coverage level via D6 metrics. Stage 13 does NOT query `product_candidates` directly — only `candidate_net_margin_assessments` (which is the Stage 12 universe).

**Important — `suggested_action` count is `re_evaluation_eligible` candidates per process, NOT prompts**. For DGU, 1 candidate has 2 prompts but the action label says "Clarify 1 candidate(s)" because there's only 1 candidate to clarify. For Aggregates: 2 re_evaluation_eligible candidates → "Clarify 2 candidate(s)" but 2 prompts (one each).

**Totals to verify in R.1, R.3, R.6**:
- `manifests` row count = **7** (one per `pipeline_assessments` row)
- All 7 → `stoplight='clarify'` (no PASS, no REJECT, every process has re_evaluation_eligible candidates)
- Total markdown artifacts: **7** at `artifacts/manifests/{process_id}/manifest.md`
- `pipeline_assessments.pipeline_state`: all 7 transition `'net_qualified'` → `'manifest_ready'`
- Total `clarification_prompts` summed across all manifests: **18** (per §2.6 mapping; verified by query)
- Per-process `clarification_prompt_count` column: as listed in the table above

This is the expected GREEN outcome. **0 PURSUE / 7 CLARIFY / 0 PASS / 0 REVIEW** with **18 prompts** is correct for today's data — every process needs operator clarification before it can auto-pursue.

---

## §4 — REQUIRED NEW UNIT TESTS

Add 21 tests to `crates/daemon/src/main.rs` (`mod tests`). Every test name listed must exist with `#[test]` or `#[tokio::test]` and pass. **Total: 21 new tests** (T.1–T.21), bringing the binary unit test count from 99 (current) to 120.

| # | Test name | What it verifies |
|---|---|---|
| T.1 | `stoplight_pursue_when_any_pass_and_priced_coverage_above_5000` | synthetic process: 1 PASS candidate + priced_coverage_bps=6000 → stoplight='pursue' |
| T.2 | `stoplight_clarify_when_re_evaluation_eligible_and_no_pass` | candidate REVIEW with re_eval_eligible=1, no PASS → stoplight='clarify' |
| T.3 | `stoplight_pass_when_all_reject` | all candidates REJECT → stoplight='pass' |
| T.4 | `stoplight_pass_when_zero_coverage_no_re_eval_eligible` | priced_coverage_bps=0 AND no re_eval_eligible → stoplight='pass' |
| T.5 | `stoplight_review_otherwise` | mixed verdicts not matching any rule → stoplight='review' |
| T.6 | `suggested_action_label_includes_re_eval_count_for_clarify` | 3 re_eval candidates → label="Clarify 3 candidate(s) before deciding" |
| T.7 | `clarification_prompt_for_quantity_basis_undetermined_includes_unit_from_quote` | candidate with `quantity_basis_undetermined` reason and supplier_quote.price_basis_unit='kg' → prompt has unit='kg' |
| T.8 | `clarification_prompt_for_awaiting_rfq_quote_has_object_type` | reason `awaiting_rfq_quote` → prompt_type='object' |
| T.9 | `clarification_prompt_for_no_pricing_signal_has_choice_type` | reason `no_pricing_signal` → prompt_type='choice' |
| T.10 | `confidence_below_floor_alone_yields_no_prompt` | candidate with ONLY `confidence_*_below_floor` reasons → 0 prompts emitted (even though re_eval_eligible=1) |
| T.11 | `multiple_actionable_reasons_yield_multiple_prompts_per_candidate` | DGU-shape: 2 actionable + 2 informational reasons → exactly 2 prompts |
| T.12 | `manifest_basis_hash_stable_across_runs_with_identical_inputs` | identical inputs twice → identical hash |
| T.13 | `manifest_basis_hash_changes_when_pipeline_assessments_decision_basis_hash_changes` | bump Stage 12's hash → Stage 13's hash differs |
| T.14 | `idempotent_no_op_when_manifest_basis_hash_matches_persisted` | run Stage 13 twice → second call upserts 0 rows |
| T.15 | `unpriced_process_yields_stub_manifest_with_zero_priced_count` | process with all `no_pricing_signal` candidates → manifest has `priced_candidate_count=0`, prompts exist (one per candidate) |
| T.16 | `clarification_prompts_json_serializes_alphabetically_by_candidate_id` | deterministic ordering for diff stability |
| T.17 | `pipeline_state_transitions_to_manifest_ready_after_stage13` | UPSERT changes pipeline_state from 'net_qualified' → 'manifest_ready' |
| T.18 | `markdown_artifact_path_uses_per_process_subdir` | manifest written to `artifacts/manifests/{process_id}/manifest.md` (not flat layout) |
| T.19 | `stoplight_drift_in_skill_output_rejects_with_specific_reason` | synthetic skill output with `stoplight='pursue'` while bundle has `stoplight='clarify'` → daemon rejects with `AGENT_OUTPUT_CONTRACT_INVALID` and reason `stoplight_drifted_from_bundle` (per §2.7 drift table) |
| T.20 | `clarification_prompts_drift_in_skill_output_rejects_with_specific_reason` | synthetic skill output with one extra prompt (or one removed) compared to bundle → rejection with reason containing `clarification_prompts_drifted` |
| T.21 | `frontmatter_drift_in_markdown_rejects_with_specific_reason` | skill output's `manifestMarkdown` has frontmatter `stoplight: pursue` while bundle (and skill's structured `stoplight` field) has `clarify` → rejection with reason `frontmatter_drifted_from_bundle` |

T.1–T.16 are pure-function tests (no DB, no skill). T.17–T.18 may need a DB fixture; refactor the persist logic into a pure function if needed so the test stays fast and deterministic. T.19–T.21 are pure-function tests over the daemon's validation pipeline (synthetic skill output → drift detector → reject/accept).

---

## §5 — REQUIRED SCHEMA-ACTIVATION TEST

Add to `crates/daemon/tests/schema_activation.rs` mirroring `ngg_t_schema_activation_007_upgrade_path_applies_0007_cleanly`:

```rust
#[test]
fn mfg_t_schema_activation_008_upgrade_path_applies_0008_cleanly() -> Result<()> {
    // Apply 0001..0007 manually; seed schema_migrations to epoch 4;
    // open StateDb (which should apply 0008 + bump epoch to 5);
    // assert manifests table now has columns: stoplight, suggested_action,
    //   suggested_action_reasoning, clarification_prompts_json, manifest_basis_hash,
    //   manifest_template_version, candidate_count, priced_candidate_count,
    //   clarification_prompt_count;
    // assert schema_migrations row for '0008_manifest_generation_columns' exists.
}
```

---

## §6 — ACCEPTANCE CRITERIA — R.1 to R.18

| # | Criterion |
|---|---|
| R.1 | `manifests` row count = number of `pipeline_assessments` rows (today: 7) |
| R.2 | All `manifests` rows have non-NULL `stoplight`, `suggested_action`, `suggested_action_reasoning`, `manifest_basis_hash`, `markdown`, `artifact_locator`, `artifact_hash` |
| R.3 | Stoplight distribution matches expected: today `pursue=0, clarify=7, pass=0, review=0` |
| R.4 | All 7 `suggested_action` strings match the per-stoplight format from §1 D3 (e.g. "Clarify N candidate(s) before deciding") |
| R.5 | All 7 `suggested_action_reasoning` strings are non-empty, each <= 2000 chars, each references at least one specific Stage 12 reason code or candidate name (anchored to evidence) |
| R.6 | Clarification prompts: total count across all manifests equals the sum of actionable reasons across all candidates, per §2.6 mapping. **Today: 18 prompts total** (verified by query, breakdown in §3 table). Per-process `clarification_prompt_count` column matches the §3 per-process figures |
| R.7 | No clarification prompt has `reason_code` starting with `confidence_` (those are not actionable per §2.6) |
| R.8 | Each clarification prompt has all required fields populated (candidate_id, candidate_name, reason_code, prompt_text, prompt_type) |
| R.9 | `clarification_prompts_json` column on `manifests` exactly equals the frontmatter `clarification_prompts` parsed from the markdown body (byte-equivalent after canonical serialization) |
| R.10 | Markdown artifact files exist at `artifacts/manifests/{process_id}/manifest.md` for all 7 processes |
| R.11 | `manifests.markdown` column equals the file contents (re-parse the file, compare) |
| R.12 | `manifests.artifact_hash` equals sha256 of the markdown bytes |
| R.13 | `pipeline_assessments.pipeline_state` for all 7 Stage 13 processes is now `'manifest_ready'` (was `'net_qualified'` after Stage 12). All other `pipeline_assessments` columns set by Stage 12 (`gross_margin_bps`, `net_margin_bps`, `priced_coverage_bps`, `landed_cost_coverage_bps`, `net_qualified_coverage_bps`, `unresolved_mandatory_lots`, `decision_basis_hash`, `review_required`, `review_stage`, `review_reason`) are **unchanged** — Stage 13's UPDATE only sets `pipeline_state` and `updated_at` (per §2.8a) |
| R.14 | `process_stage_status` rows: 7 with stage='manifest', scope_kind='process', state='succeeded'; `process_stage_heads`: 7 likewise |
| R.15 | FK integrity: 0 orphans `manifests` → `procurement_processes`; 0 manifests files without DB rows or vice versa |
| R.16 | Re-runnability: deleting all manifests rows + artifact files + re-running `--ingest manifest` reproduces the same row count, same stoplights, same clarification_prompts (deterministic) |
| R.17 | Idempotency: a second `--ingest manifest` invocation produces zero new rows AND zero artifact-file rewrites (manifest_basis_hash matches → skip). Log line reports `skipped_unchanged=R.1_count` |
| R.18 | Stage 11 + Stage 12 untouched: 4 `landed_cost_estimates` still `decision_grade='review'`, DGU row unchanged (`5160\|17692\|0\|4570\|27422`); 17 `candidate_net_margin_assessments` rows still all `verdict='review'` with same reasons; ALL `pipeline_assessments` columns OTHER than `pipeline_state`+`updated_at` are bit-for-bit identical to their pre-Stage-13 values (snapshot before Stage 13 run, compare after — see §2.8a); quality gates pass: `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p daemon --bin daemon` (T.1–T.21 included → expect 99 prior + 21 = **120 binary unit tests pass**), per-file integration loop green across all **39 files** (file count unchanged; `schema_activation.rs` grows from 19 to 20 tests with §5 addition) |

---

## §7 — VALIDATION QUERIES

```bash
sqlite3 -header -column state/state.db "
SELECT '=== R.1 row count ===';
SELECT COUNT(*) AS manifests_rows FROM manifests;
SELECT COUNT(*) AS pipeline_assessments_rows FROM pipeline_assessments;

SELECT '=== R.2 required fields populated ===';
SELECT 'rows with NULL critical fields: ' || COUNT(*) FROM manifests
WHERE stoplight IS NULL OR suggested_action IS NULL
   OR suggested_action_reasoning IS NULL OR manifest_basis_hash IS NULL
   OR markdown IS NULL OR artifact_locator IS NULL OR artifact_hash IS NULL;

SELECT '=== R.3 stoplight distribution ===';
SELECT stoplight, COUNT(*) FROM manifests GROUP BY stoplight;

SELECT '=== R.4 suggested_action format ===';
SELECT stoplight, suggested_action FROM manifests ORDER BY stoplight, process_id;

SELECT '=== R.5 reasoning length + non-empty ===';
SELECT process_id, length(suggested_action_reasoning) AS chars FROM manifests
WHERE suggested_action_reasoning IS NULL OR length(suggested_action_reasoning) = 0
   OR length(suggested_action_reasoning) > 2000;

SELECT '=== R.6 clarification prompt total count ===';
SELECT SUM(json_array_length(clarification_prompts_json)) AS total_prompts FROM manifests;
SELECT 'expected: 18';

SELECT '=== R.6 per-process prompt counts (must match §3 table) ===';
SELECT process_id, json_array_length(clarification_prompts_json) AS prompts FROM manifests ORDER BY process_id;

SELECT '=== R.7 no confidence_* prompts ===';
SELECT 'rows with confidence_ prompt: ' || COUNT(*) FROM manifests m
CROSS JOIN json_each(m.clarification_prompts_json) AS je
WHERE json_extract(je.value, '\$.reason_code') LIKE 'confidence_%';

SELECT '=== R.8 all prompt fields populated ===';
SELECT 'prompts missing required field: ' || COUNT(*) FROM manifests m
CROSS JOIN json_each(m.clarification_prompts_json) AS je
WHERE json_extract(je.value, '\$.candidate_id') IS NULL
   OR json_extract(je.value, '\$.candidate_name') IS NULL
   OR json_extract(je.value, '\$.reason_code') IS NULL
   OR json_extract(je.value, '\$.prompt_text') IS NULL
   OR json_extract(je.value, '\$.prompt_type') IS NULL;

SELECT '=== R.13 pipeline_state transitions ===';
SELECT pipeline_state, COUNT(*) FROM pipeline_assessments GROUP BY pipeline_state;
-- Expected: all 7 = 'manifest_ready'

SELECT '=== R.14 stage tracking ===';
SELECT stage, scope_kind, state, COUNT(*) FROM process_stage_status WHERE stage='manifest' GROUP BY 1,2,3;
SELECT stage, scope_kind, COUNT(*) FROM process_stage_heads WHERE stage='manifest' GROUP BY 1,2;

SELECT '=== R.15 FK integrity ===';
SELECT 'orphan process: ' || COUNT(*) FROM manifests m
LEFT JOIN procurement_processes p ON p.id=m.process_id WHERE p.id IS NULL;

SELECT '=== R.18 Stage 11/12 untouched ===';
SELECT decision_grade || '|' || supplier_cost_gbp_pence || '|' || freight_base_gbp_pence
       || '|' || customs_duty_gbp_pence || '|' || import_vat_gbp_pence
       || '|' || economic_landed_cost_gbp_pence
FROM landed_cost_estimates WHERE supplier_quote_id = 'sq-b22fcf00b7a545d699c3438c42b9027c-1';
SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY verdict;
"
```

R.10–R.12 (file ↔ DB integrity) — hash-based comparison so multi-line markdown doesn't break shell quoting:

```bash
echo "=== R.10 file existence ==="
sqlite3 -separator $'\t' state/state.db "SELECT process_id, artifact_locator FROM manifests" | while IFS=$'\t' read -r pid loc; do
  if [ -f "$loc" ]; then echo "OK $pid"; else echo "MISSING $pid -> $loc"; fi
done

echo "=== R.11/R.12 hash invariant: file_sha256 == db.artifact_hash AND db.markdown_sha256 == db.artifact_hash ==="
sqlite3 -separator $'\t' state/state.db "SELECT process_id, artifact_locator, artifact_hash FROM manifests" | while IFS=$'\t' read -r pid loc stored_hash; do
  file_hash=$(shasum -a 256 "$loc" 2>/dev/null | awk '{print $1}')
  db_md_hash=$(sqlite3 state/state.db "SELECT markdown FROM manifests WHERE process_id='$pid'" | shasum -a 256 | awk '{print $1}')
  if [ "$stored_hash" = "$file_hash" ] && [ "$stored_hash" = "$db_md_hash" ]; then
    echo "OK $pid (hash $stored_hash)"
  else
    echo "DRIFT $pid stored=$stored_hash file=$file_hash db_md=$db_md_hash"
  fi
done
```

Note: `sqlite3` may emit a trailing newline that affects `shasum`. The agent may need to trim a trailing `\n` from the `markdown` column before hashing, OR the daemon should canonicalize markdown content (no trailing whitespace) when computing `artifact_hash` so the comparison is stable across stdout-capture boundaries. Document the chosen convention in §10's Issues Found section if drift is detected.

R.16 (re-runnability — destructive, only after R.1–R.15 are green):

```bash
sqlite3 state/state.db "
DELETE FROM manifests;
DELETE FROM process_stage_heads WHERE stage = 'manifest';
DELETE FROM process_stage_status WHERE stage = 'manifest';
UPDATE pipeline_assessments SET pipeline_state = 'net_qualified' WHERE pipeline_state = 'manifest_ready';
"
rm -rf artifacts/manifests/*
cargo run -p daemon -- --ingest manifest
# Re-run R.1–R.6 and confirm identical counts and stoplights.
```

R.17 (idempotency):

```bash
cargo run -p daemon -- --ingest manifest  # populates
ROW_COUNT_BEFORE=$(sqlite3 state/state.db "SELECT COUNT(*) FROM manifests;")
HASH_BEFORE=$(sqlite3 state/state.db "SELECT artifact_hash FROM manifests ORDER BY process_id" | shasum -a 256)
cargo run -p daemon -- --ingest manifest  # should be no-op
ROW_COUNT_AFTER=$(sqlite3 state/state.db "SELECT COUNT(*) FROM manifests;")
HASH_AFTER=$(sqlite3 state/state.db "SELECT artifact_hash FROM manifests ORDER BY process_id" | shasum -a 256)
echo "rows: before=$ROW_COUNT_BEFORE after=$ROW_COUNT_AFTER (must equal)"
echo "hashes: before=$HASH_BEFORE after=$HASH_AFTER (must equal)"
# Log line should report skipped_unchanged=7
```

---

## §8 — FAILURE-MODE PLAYBOOK

| Symptom | Likely cause | Remediation |
|---|---|---|
| All 7 manifests have `stoplight='review'` | Stoplight derivation order wrong; the `clarify` rule isn't matching `re_evaluation_eligible=1` | Re-read §1 D3 — `clarify` rule is row 2, must check `re_evaluation_eligible=1` before falling through to `'review'` |
| Aggregates manifest has 0 clarification prompts | §2.6 mapping not applied for `quantity_basis_undetermined` reason | The mapping table is the only source of truth for prompts. Aggregates' candidate has reason `quantity_basis_undetermined` → 1 prompt with unit from supplier_quote.price_basis_unit='kg' |
| DGU manifest has 4 prompts (one per reason) | Confidence reasons not being filtered out | Per §2.6, `confidence_*_below_floor` reasons emit ZERO prompts. DGU has 4 reasons but only 2 actionable (`quantity_basis_unaligned`, `awaiting_rfq_quote`) |
| Skill output rejected: schema mismatch | Skill emitted `manifestMarkdown` but missing `stoplight` / `clarification_prompts` / etc. | Skill must echo back the deterministic fields the daemon passed in. Re-read §2.4 — skill MUST NOT re-derive these; pass them through unchanged |
| `suggested_action_reasoning` is generic ("This opportunity needs review") | Skill prompt isn't requiring evidence anchoring | Update SKILL.md to require reasoning to reference specific candidate names AND specific reason codes from the bundle |
| Migration 0008 not applied → first INSERT fails with "no such column" | Forgot §2.2 registration steps (constant + MIGRATIONS array + DEFAULT_SCHEMA_EPOCH bump) | Re-read §2.2. All four steps required including the test update (step 4) |
| `pdf_t_schema_activation_009_default_schema_epoch_is_4` test failed | Schema epoch bumped without updating the test | Per §2.2 step 4: rename to `_is_5` and assert `== 5`, OR add a new test |
| Markdown file ≠ DB column | Daemon wrote one path but not the other | Persist artifact + DB row in a single transaction-ordered sequence. Compute markdown ONCE, write to file, hash file bytes, write DB row with that hash. Don't re-render |
| `clarification_prompts_json` column ≠ frontmatter `clarification_prompts` | Daemon serialized differently for column vs frontmatter | Serialize ONCE (canonical JSON, sorted by candidate_id), write the same bytes to both column and frontmatter. The skill reads the bundle's prompts and must emit them unchanged in the frontmatter |
| Re-running mutates manifests non-deterministically | manifest_template_version changes per run, OR basis_hash composition order varies | manifest_template_version must be a constant. basis_hash inputs must be in the exact order shown in §2.7, with `:` separators |
| Idempotent run still rewrites artifact files | Skip-on-match check missing OR doesn't touch the file system | The skip check returns early from the per-process loop iteration BEFORE invoking the skill or writing files. Log line increments `skipped_unchanged` |
| `pipeline_state` regresses from `'manifest_ready'` to `'net_qualified'` after a Stage 12 re-run | Expected behavior — Stage 12 unconditionally sets `'net_qualified'` (per Stage 12 D6) | Re-run Stage 13 after Stage 12 to advance to `'manifest_ready'` again |
| Quality gates fail in `pdf_structural_extraction` | Docling not installed locally | These tests auto-skip per cleanup commit `7b77e94`. Don't bypass |
| Modified `run_fts_live_loop`, `finalize_success`, `build_manifest_from_db`, or `ManifestComputation` | Out of scope — those are legacy FTS-driven manifest path | Stop. Build a NEW production path. The legacy path continues to write `manifests` rows during FTS smoke tests (with NULL stoplight etc.) — which is fine, they're separate concerns |

---

## §9 — CONSTRAINTS

1. **Do not modify Stage 11 or Stage 12 code.** No changes under `run_landed_cost_*` or `run_net_margin_gate_*`. Stage 13 is a pure consumer of Stage 12 output.
2. **Do not modify the legacy FTS-driven manifest path.** `run_fts_live_loop` (production code, dispatched from `--ingest fts`), `finalize_success`, `build_manifest_from_db`, `ManifestComputation` continue to exist unchanged. Stage 13 introduces a parallel production path under `--ingest manifest` that supersedes them for all-source manifest generation. The two paths are intentionally orthogonal during the transition: legacy fires only during FTS ingestion (currently 0 rows), Stage 13 fires for everything in `pipeline_assessments`.
3. **Do not consume the operator-override mechanism.** Stage 14 will introduce it. Stage 13 v1 only marks `re_evaluation_eligible` candidates with clarification_prompts; it does NOT read overrides.
4. **Do not wire `--ingest manifest` into `run_deterministic_pipeline`.** Standalone trigger only.
5. **Skill is the only LLM-creative writer.** The skill produces `suggested_action_reasoning`. All other output fields are passed through from the bundle. Test T.5 (reasoning evidence anchoring) is mandatory.
6. **No silent test exclusion.** `cargo test -p daemon --bin daemon` is necessary but not sufficient (per AGENTS.md `### Stage Validation Test Gate`). The per-file integration loop is mandatory.
7. **Migration is `IF NOT EXISTS` for indexes; `ALTER TABLE` for columns is irreversible — get it right the first time.** Test the migration on a copy of state.db before committing.
8. **`clarification_prompts_json` is JSON `object[]` not `string[]`.** Each object has the structure documented in §1 D6 / §2.6. Closed reason_code enum (R.7).
9. **Markdown artifact and DB column must be byte-equivalent.** Single source: render once, write to both, hash from the bytes that hit the disk.
10. **Skill output validation is non-negotiable AND has two layers** (per §2.7 drift detection): (1) `validate_result_schema` for shallow shape (required fields, top-level types — but it does NOT enforce enum/minLength/items per §0); (2) Rust-typed deserialization of `stoplight` (enum), `clarification_prompts` (Vec<ClarificationPrompt>), and length bounds for `suggested_action_reasoning`; (3) drift comparison of pass-through fields against the bundle. All three layers reject with `AGENT_OUTPUT_CONTRACT_INVALID` and a specific reason code.
11. **`row_version` does NOT auto-increment on UPSERT.** Stage 11's existing pipeline_assessments UPSERT does not set row_version in the SET clause (it stays at whatever it was). For consistency with existing convention, Stage 13's UPSERT should also leave `row_version` untouched on conflict (it stays at 1 from the INSERT path). Sync (L4) will revisit this; out of scope for v1.

---

## §10 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/stage-13-validation-manifest.md`. Use the template below **verbatim** (fill in the placeholders); do not invent your own format. Mirrors `docs/pipeline/stage-12-validation-manifest.md`.

```markdown
# Stage 13 Manifest Generation Validation Manifest

> Generated: {ISO timestamp}
> Status: {GREEN | RED}

## Summary
{1 paragraph: what was built, what the live DB looks like after,
stoplight distribution, clarification_prompt counts, observations.}

## Build Summary
- Migration: `migrations/0008_manifest_generation_columns.sql`
- Migration registered: `crates/daemon/src/db/sqlite.rs` MIGRATIONS array (epoch 5)
- Skill schema extension: `skills/manifest-generation/result.schema.json` (8 required fields)
- SKILL.md updates: `skills/manifest-generation/SKILL.md` (new contract)
- Code: `run_manifest_generation_live_loop` at `main.rs:LINE`
- Dispatch: `--ingest manifest` at `main.rs:LINE`
- New unit tests: T.1–T.18 (all in `mod tests`)
- New schema test: `mfg_t_schema_activation_008_*` in `crates/daemon/tests/schema_activation.rs`
- Updated test: `pdf_t_schema_activation_009_*` renamed to `_is_5`

## Live Run Result
{Final daemon log line in fenced block}

## Per-Process Manifest Distribution
{Table: process_id, stoplight, suggested_action, candidate_count, prompt_count}

## Stoplight Distribution
{Table: stoplight, count}

## R.1–R.18 Acceptance Criteria
| # | Criterion | Result | Detail |

## Quality Gates
- cargo fmt --check: {PASS/FAIL}
- cargo check --all-targets: {PASS/FAIL}
- cargo clippy --all-targets -- -D warnings: {PASS/FAIL}
- cargo test -p daemon --bin daemon: {PASS/FAIL — N tests, includes T.1–T.18}
- per-file integration loop: {PASS/FAIL — 39 files}

## Issues Found & Fixed
{Each: symptom → root cause → fix → verification}

## Code Changes
| File | Change |

## Out-of-Scope (Documented, Not Fixed Here)
- Operator override mechanism + UI binding (Stage 14)
- Wiring `--ingest manifest` into `run_deterministic_pipeline`
- Manifest staleness TTL / scheduled re-render (currently only basis_hash-driven)
```

### Quality gates (the broader gate, mandatory per AGENTS.md)

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

If any single integration test target fails, fix the root cause. Do **not** weaken or `#[ignore]` to bypass.

---

## §11 — REMINDERS

- ULTRATHINK and be meticulous.
- The expected outcome on today's data is **0 PURSUE / 7 CLARIFY / 0 PASS / 0 REVIEW** with **21 clarification prompts** total. That is correct, not a bug. Every process needs operator clarification before it can auto-pursue. **Do not engineer a PURSUE to feel good.**
- The skill's ONLY creative output is `suggested_action_reasoning` (1–2 sentences, evidence-anchored). Everything else is deterministic and passes through unchanged. If you find yourself making the skill compute stoplight or prompts, stop.
- The legacy FTS-driven manifest path (`run_fts_live_loop`, `finalize_success`, `build_manifest_from_db`, `ManifestComputation`) continues to write to `manifests` with NULL on the new columns. This is fine. Stage 13's production path is separate and always populates them.
- DGU is the most interesting test case again: 4 reasons but only 2 prompts (the 2 confidence reasons are not actionable). If your implementation emits 4 prompts for DGU, you've missed §2.6.
- T.1–T.18 are not optional. Stage 12 hardening's "5 untested code paths" lesson applies — every new policy decision must be regression-tested.
- If you find a design corner that D1–D6 don't cover, **ask before guessing**.
- Once the manifest is GREEN, stop. Stage 14 (Operator Decision) is the next stage; it is a separate handoff. Stage 14 will read `manifests.clarification_prompts_json` AND the markdown frontmatter to render the operator UI.
