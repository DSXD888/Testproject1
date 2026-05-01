# Stage 13 Skill Invocation Restoration (Option B) — Investigate, Fix, Validate

ULTRATHINK and be meticulous.

## MISSION

Stage 13 (Manifest Generation) shipped on 2026-05-01 with a **fallback to a daemon-local Rust renderer** because the manifest-generation skill stalled on live invocation attempts (per `docs/pipeline/stage-13-validation-manifest.md`'s Issues Found section). The result: 7 manifests are deterministic and correct, but the LLM-creative `suggested_action_reasoning` paragraph the operator signed off on (Q5 hybrid) is currently templated by Rust, not written by the skill. The skill's drift-detection logic (§2.7 of the original Stage 13 handoff) is technically running but tautologically (daemon validates its own output).

Your job: **investigate the stall, fix the root cause, and route Stage 13 through actual skill invocation**. After this work, the `manifest-generation` skill is invoked once per process via `execute_skill_session`, the LLM writes the reasoning paragraph, and drift detection becomes meaningful (because there's a real LLM to drift).

You are not just fixing it. You will **investigate → patch → validate → test → fix in a loop** until every numbered acceptance criterion (R.1–R.16 below) is GREEN with **skill_session_ids non-NULL on all 7 manifest rows**. Do not stop with the daemon-local renderer still in the production path. Do not silently weaken the test gate to make it green.

```
┌──────────────────────────────────────────────────────────────────┐
│                          THE LOOP                                  │
│                                                                    │
│   1. Pre-flight (verify the facts in §0 below)                    │
│   2. Add a SKILL_INVOCATION feature flag (§2)                     │
│   3. Run `--ingest manifest` with flag ON, smallest process first │
│      (capture diagnostics per §3)                                 │
│   4. Diagnose: which leg stalled? (timeout, payload, parser, ...) │
│   5. Apply the matching fix from §4                               │
│   6. Re-run on small process. Then medium. Then large.            │
│   7. Validate against R.1..R.16                                   │
│   8. Any criterion failing?                                       │
│      ├─ YES → diagnose, apply next fix candidate, go to step 6    │
│      └─ NO  → run quality gates (fmt + check + clippy + binary    │
│                unit tests + per-file integration loop). Failures? │
│                ├─ YES → fix, go to step 7                         │
│                └─ NO  → write the validation manifest             │
│                                                                    │
│   DO NOT STOP UNTIL ALL R.* PASS WITH SKILL INVOCATION ENABLED.   │
└──────────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first. Pay particular attention to `## Compiler Checks (CRITICAL)` and `### Stage Validation Test Gate`.

---

## §0 — PRE-FLIGHT (verify before you start)

These facts were confirmed against the live tree at handoff authoring time.

| Claim | Verify command |
|---|---|
| Stage 13 has 7 `manifests` rows, all `stoplight='clarify'`, 18 total clarification_prompts | `sqlite3 state/state.db "SELECT COUNT(*), SUM(json_array_length(clarification_prompts_json)) FROM manifests"` |
| The current Stage 13 path uses a **daemon-local renderer** — no skill is invoked. Existing manifest rows have NULL or synthetic skill_session_ids | `grep -n "daemon.local manifest contract renderer\|local renderer\|fn .*manifest_render" crates/daemon/src/main.rs` |
| `run_manifest_generation_live_loop` lives at `main.rs:8114` (per Stage 13 manifest) | `grep -n "fn run_manifest_generation_live_loop" crates/daemon/src/main.rs` |
| `execute_skill_session(runtime, skill, job_id, session_id, job_parameters, cli)` at `main.rs:8154` is the canonical helper Stage 11 uses for skill invocation; Stage 13's restored path will reuse it | `grep -n "async fn execute_skill_session" crates/daemon/src/main.rs` |
| Stage 11's bounded-skill helper `execute_landed_cost_skill_session` at `main.rs:4070` has protocol-parser recovery + timeout-recovery logic added during Stage 11 hardening. Worth re-reading if Stage 13's stalls look protocol-shaped | `grep -n "async fn execute_landed_cost_skill_session" crates/daemon/src/main.rs` |
| Stage 11's `run_landed_cost_skill` at `main.rs:~3741` is the canonical "build job_parameters → workflow_jobs row → execute_skill_session → read result.json → parse" pattern. Stage 13's restored skill path should mirror this structure | `grep -n "async fn run_landed_cost_skill\b" crates/daemon/src/main.rs` |
| **Stage 13's daemon-local renderer is ALREADY extracted** as `render_manifest_skill_output_from_bundle(&bundle)` at `main.rs:7808`, plus `validate_manifest_skill_output(schema, bundle, payload)` at `main.rs:7860`, plus `persist_manifest_generation_output(...)` at `main.rs:7979`. **The agent's job is to ADD a skill-mode path that produces a `Value`, then feed it into the existing `validate_manifest_skill_output`** — the validate + persist code is reused unchanged | `grep -n "fn render_manifest_skill_output_from_bundle\|fn validate_manifest_skill_output\|fn persist_manifest_generation_output" crates/daemon/src/main.rs` |
| **`validate_manifest_skill_output` ALREADY contains the FULL drift-detection suite** with these exact reason codes: `schema_validation_failed`, `typed_deserialization_failed`, `process_id_drifted_from_bundle`, `decision_basis_hash_drifted_from_bundle`, `stoplight_drifted_from_bundle`, `suggested_action_drifted_from_bundle`, `candidate_count_drifted_from_bundle`, `priced_candidate_count_drifted_from_bundle`, `clarification_prompt_count_drifted_from_bundle`, `clarification_prompts_drifted`, `suggested_action_reasoning_length_invalid`, `suggested_action_reasoning_not_anchored`. Plus the helper `manifest_reasoning_is_anchored(bundle, reasoning)` is in place. Drift checks are tautological in daemon-only mode but become **meaningful as soon as the skill is invoked**. No new validation code needed for Option B — just route the skill output through the existing validator | `grep -A 90 "fn validate_manifest_skill_output" crates/daemon/src/main.rs` |
| **`stoplight` is typed as `ManifestStoplight` enum** (not String) in `ManifestSkillOutput`. Invalid stoplight values fail at typed-deserialization (`typed_deserialization_failed` reason), BEFORE drift detection runs. Round-3's worry about `validate_result_schema` not enforcing enums is moot here — the Rust type system enforces it via serde | `grep -B2 -A10 "struct ManifestSkillOutput" crates/daemon/src/main.rs` |
| **JSON key vs Rust field naming** — `ManifestSkillOutput` uses `#[serde(rename = "decisionBasisHash")]` and `#[serde(rename = "manifestMarkdown")]`. The skill's `result.json` MUST emit camelCase: `decisionBasisHash`, `manifestMarkdown`. All other fields are snake_case. **`SKILL.md` already documents this correctly** (look at the "Result Payload Template" section) — Stage 13's build updated it even though they didn't end up invoking the skill. No SKILL.md update needed for camelCase | `grep -A 12 "struct ManifestSkillOutput" crates/daemon/src/main.rs` and `cat skills/manifest-generation/SKILL.md` |
| **Current frontmatter does NOT include `renderer_mode`** — the daemon-local renderer didn't add it. For Option B, both render paths need to add `renderer_mode: <value>` to the YAML frontmatter so R.1's verification grep can read it. The skill-mode path: extend the SKILL.md frontmatter template to require `renderer_mode: skill`. The daemon-local path: extend `render_manifest_skill_output_from_bundle` to emit `renderer_mode: daemon` (or `auto_fallback_to_daemon` when called from the auto branch). The drift check should NOT compare `renderer_mode` (it's the only intentionally-different field between bundle and output) | `sqlite3 state/state.db "SELECT markdown FROM manifests LIMIT 1" \| head -10` |
| **`SKILL.md` "Rendering Contract" lists clarification_prompt fields**: candidate_id, candidate_name, lot_id, reason_code, prompt_text, prompt_unit, prompt_type, default_value, rationale. **But choice-type prompts (for `no_pricing_signal`) ALSO carry an `options:` array** (e.g., `[manual_price, un_pursueable, defer]`) which SKILL.md's template doesn't show. If the bundle's `ClarificationPrompt` struct includes `options` for choice-type prompts, the skill must echo it; otherwise the canonical prompts-JSON drift check fires (`clarification_prompts_drifted`). Either: extend SKILL.md to document the `options:` field, OR confirm the prompt struct doesn't carry it (in which case the daemon adds it post-skill, before persistence) | `sqlite3 state/state.db "SELECT json_extract(clarification_prompts_json, '\$[0]') FROM manifests WHERE clarification_prompts_json LIKE '%no_pricing_signal%' LIMIT 1"` |
| **`execute_skill_session` does NOT take a timeout parameter**. Wrapping with `tokio::time::timeout(Duration::from_secs(N), execute_skill_session(...))` cancels the await but does NOT kill the spawned child process — the session may keep running in the background, leaking work and possibly subscription quota. Stage 11's `execute_landed_cost_skill_session` (line 4070) handles this via explicit child-process termination + result-recovery. Mirror that pattern for Stage 13's bounded skill invocation | `grep -B1 -A10 "async fn execute_skill_session\|async fn execute_landed_cost_skill_session" crates/daemon/src/main.rs` |
| **`Cli` struct does NOT derive `Clone`** (only `Debug + Parser`). My F.2a concurrency example uses `cli.clone()` — that **won't compile**. Either add `#[derive(Clone)]` to `Cli` (safe, low-impact change) or refactor to share `&Cli` via Arc, OR stay with serial F.2b execution which only needs `&Cli` references | `grep -B5 "^struct Cli\b" crates/daemon/src/main.rs` |
| **`extract_frontmatter` helper does NOT exist in the codebase** — Stage 13's actual build skipped it (the daemon-local renderer doesn't need to parse YAML from a string; it constructs the markdown WITH frontmatter directly). For Option B's frontmatter-vs-bundle drift check (and for R.1 verification), the agent must build it. Code shape provided below in §3.2.5 | `grep -n "fn extract_frontmatter" crates/daemon/src/main.rs` (should be empty) |
| `agent_session_logs` schema has columns: `id`, `job_id`, **`skill_path`** (NOT `skill_id`), `skill_version`, `execution_mode`, `pty_session_id` (NOT `session_id`), `started_at`, `completed_at`, **`status`** (NOT `outcome_class`), `terminal_output_path`. Stage 13 verification queries use `skill_path LIKE '%manifest-generation%'` and `status='succeeded'` | `sqlite3 state/state.db ".schema agent_session_logs"` |
| `manifests` table has **NO `confidence_breakdown` column** — that's a Stage 11 (`landed_cost_estimates`) thing. Stage 13's "where do we record skill_invocation_outcome" answer is: **markdown frontmatter** (`renderer_mode: skill / daemon / auto_fallback_to_daemon`), cross-checked against `agent_session_logs` rows. No migration 0009 needed | `sqlite3 state/state.db ".schema manifests" \| grep confidence` (should be empty) |
| `futures` crate is **NOT in `crates/daemon/Cargo.toml`** — F.2 concurrency cannot use `futures::stream::buffer_unordered`. Use `tokio::task::JoinSet` (already available) or serial execution. Caveat: `runtime: &mut RuntimeBootstrap` is borrowed mutably through the loop, so spawning concurrent tasks may require refactoring shared state into `Arc<Mutex<>>` — possibly more complex than worth | `grep -E "^futures\b\|futures =" crates/daemon/Cargo.toml` |
| `insert_executed_session_log(conn, skill, job_id, executed_session)` at `main.rs:11217` is the canonical helper Stage 11 uses to record the skill audit row. Stage 13's restored skill path **must** call this after every skill invocation so R.1 verification has data | `grep -n "fn insert_executed_session_log" crates/daemon/src/main.rs` |
| `LANDED_COST_SKILL_TIMEOUT_SECONDS = 300` at `main.rs:251` is Stage 11's bounded ceiling. Stage 13 may have reused this constant or hardcoded a tighter one — find and document the current Stage 13 timeout | `grep -nE "TIMEOUT_SECONDS|tokio::time::timeout|Duration::from_secs" crates/daemon/src/main.rs` (look near the run_manifest_generation_live_loop body) |
| `skills/manifest-generation/SKILL.md` declares `agent_cli: claude` + `interaction_profile: headless_contract` + `model_profile: structured_extract_low_cost` | `head -20 skills/manifest-generation/SKILL.md` |
| `skills/manifest-generation/result.schema.json` has 8 required fields (process_id, decisionBasisHash, manifestMarkdown, stoplight, suggested_action, suggested_action_reasoning, clarification_prompts, candidate_count) | `cat skills/manifest-generation/result.schema.json` |
| The skill is **not** wired into the contracts-finder or other live loops — only the daemon-local renderer in `run_manifest_generation_live_loop` produces production manifests today | `grep -n "manifest-generation\|manifest_skill" crates/daemon/src/main.rs` |
| `claude` CLI is available on PATH and authenticated against the operator's subscription (the headless contract path leverages this — no Anthropic API key billing) | `which claude && claude --version` |
| 120 binary unit tests pass; 39 integration test files green; pdf_t_schema_activation_009 currently asserts epoch=5 | `cargo test -p daemon --bin daemon 2>&1 \| grep "test result"` |
| Stage 11/12 data invariants: DGU `landed_cost_estimates` row `review\|5160\|17692\|0\|4570\|27422`; 17 `candidate_net_margin_assessments` rows verdict='review'; 7 `pipeline_assessments` rows pipeline_state='manifest_ready' (after Stage 13's daemon-local run) | inline query in §6 |

If any drift, the rest of this handoff still holds — just update the matching numbers in the validation queries.

---

## §1 — DECISIONS ALREADY MADE (DO NOT RE-OPEN)

The operator has signed off on D1–D5. Implement to spec; do not deviate.

### D1 — Target architecture: full skill invocation (Q5 hybrid restored)

The original Stage 13 design split daemon-vs-skill responsibility (per the Stage 13 handoff §1 D5). Restore that split:

| Concern | Owner |
|---|---|
| Compute deterministic stoplight, suggested_action label, clarification_prompts, manifest_basis_hash | Daemon (unchanged from current) |
| Build the input bundle | Daemon (unchanged) |
| **Render the markdown body (sections A–F)** | **Skill** (was: daemon; restoring) |
| **Write the natural-language `suggested_action_reasoning` paragraph (1–2 sentences, evidence-anchored)** | **Skill** (was: daemon-templated; restoring) |
| Validate skill output (schema + Rust types + drift + frontmatter) | Daemon (unchanged) |
| Persist artifact + DB row | Daemon (unchanged) |

The daemon-local renderer **stays in the codebase** as the fallback path behind a feature flag (D2). It is no longer the production path once Option B is GREEN.

### D2 — Feature-flag gating

Introduce a runtime mode selector for the manifest path:

- Constant: `const STAGE13_RENDERER_MODE_ENV: &str = "STAGE13_RENDERER_MODE";`
- Values: `"skill"` (production target after Option B) | `"daemon"` (legacy fallback) | `"auto"` (default — try skill first, fall back to daemon if skill fails)
- Default during your investigation: `"daemon"` (preserves current behavior)
- Default after Option B is GREEN: `"skill"` (full LLM path)
- Operator override: `STAGE13_RENDERER_MODE=daemon cargo run -p daemon -- --ingest manifest` for emergency rollback

`"auto"` mode is your **production safety net**: if the skill stalls or returns invalid output, the row falls back to daemon-local rendering and the row is persisted with `confidence_breakdown.skill_invocation_outcome = 'fallback_to_daemon'`. This way a skill regression does not wedge Stage 13 in production.

### D3 — Diagnostic-first methodology

Do NOT immediately speculate about the stall. **Reproduce it under controlled conditions and capture artifacts**:

1. Pick the smallest process by candidate_count (probably `proc-pc-218726bd-9dda-f011-813c-005056b64545`, 1 candidate, 1 prompt) for the first invocation
2. Set the timeout to 900s (3× Stage 11's ceiling) so you don't conflate "timeout" with "real stall"
3. Capture full session artifacts: terminal-output.txt, structured_trace, all events, stdout/stderr
4. Record: wall-clock time to first token, wall-clock time to result.json appearance, total wall-clock, exit code

§3 details the diagnostic capture procedure. §4 maps diagnostic findings to fix strategies.

### D4 — Determinism scope after restoration

With the LLM in the loop, some manifest fields stop being byte-deterministic across runs:

- **Deterministic** (must remain bit-stable across runs with identical inputs): `stoplight`, `suggested_action` (label), `clarification_prompts_json`, `candidate_count`, `priced_candidate_count`, `clarification_prompt_count`, `manifest_basis_hash`
- **Non-deterministic** (LLM-rendered, may vary): `markdown` body (the prose-shaped narrative sections), `suggested_action_reasoning`, `artifact_hash` (hash of markdown bytes — varies because markdown varies)

This means **R.16 (re-runnability)** must be split: deterministic fields hash-stable across runs; markdown + reasoning may differ but must still pass schema + drift + anchoring checks. Tests T.16 (deterministic ordering of clarification_prompts) and the new T.22 below cover this.

Idempotency (R.17) is **unchanged**: when basis_hash matches the persisted row, skip — no skill call, no rewrite, hashes stay the same. Determinism only matters when basis_hash CHANGES; in that case, deterministic fields stay stable but the LLM-rendered prose updates.

### D5 — Failure budget and stop criteria

If, after applying fix candidates F.1–F.6 (§4) on real data, the skill still stalls reproducibly on the smallest process, **stop and write a RED manifest**. Do not ship a "skill mode default with auto fallback" if `auto` mode silently falls back 100% of the time — that's just `daemon` mode with extra latency. Specifically:

- **Hard stop**: if smallest-process invocation stalls for >900s after applying F.1+F.2 (timeout raise + concurrent dispatch). Document findings; user re-evaluates B-vs-A.
- **Soft warning**: if largest process needs aggressive bundle trimming (F.4) to fit, document the trimmed fields and ask the user before shipping.
- **Acceptable**: end-to-end skill mode works for all 7 processes within reasonable wall time (<5 min per `--ingest manifest`), even if individual processes take 30–90s each.

---

## §2 — ARCHITECTURE: FEATURE-FLAG SWITCH

Good news: the existing Stage 13 build is well-decomposed. The current loop body is already:

```rust
// Existing flow in run_manifest_generation_live_loop (line 8114):
let bundle = load_manifest_skill_bundle(conn, process_id)?;            // loads ManifestSkillBundle
let rendered_output = render_manifest_skill_output_from_bundle(&bundle)?; // daemon-local render → ManifestSkillOutput struct
let result_json = serde_json::to_value(&rendered_output)?;             // struct → JSON Value
let output = validate_manifest_skill_output(&skill.result_schema, &bundle, &result_json)?; // schema + drift + typed deser
persist_manifest_generation_output(...)?;                              // writes manifest row + artifact
```

**Option B's task is small**: replace the `render_manifest_skill_output_from_bundle → serde_json::to_value` step with a mode-dispatched call that produces a `Value` directly (either by calling the daemon-local renderer + serializing, OR by invoking the skill and reading its `result.json`). Validation and persistence stay unchanged. Drift detection inside `validate_manifest_skill_output` becomes meaningful the moment skill mode is used.

The `Stage13RenderOutput` struct I sketched in earlier drafts is unnecessary — the existing `ManifestSkillOutput` struct (returned by `validate_manifest_skill_output`) is what persistence consumes. The mode-dispatched code just produces the JSON `Value` that feeds the validator.

**Where does `renderer_mode` go?** Embed it as a frontmatter line in the manifest's `manifestMarkdown` (since manifestMarkdown is a string field that already carries frontmatter). The validator's typed deserialization can be extended to accept an optional `renderer_mode` field, OR the daemon can grep/parse the markdown post-validation to extract it. **Pick frontmatter** — no schema migration needed. Add a `renderer_mode: <value>` line to the YAML block of the markdown the daemon writes (skill mode: skill writes it as part of its prompt instruction; daemon-local mode: daemon writes it directly).

Then introduce the mode dispatch:

```rust
enum Stage13RendererMode { Skill, Daemon, Auto }

fn read_stage13_renderer_mode() -> Stage13RendererMode {
    match std::env::var("STAGE13_RENDERER_MODE").as_deref() {
        Ok("skill") => Stage13RendererMode::Skill,
        Ok("daemon") | Err(_) => Stage13RendererMode::Daemon, // default until §7 declares Option B GREEN
        Ok("auto") => Stage13RendererMode::Auto,
        Ok(other) => {
            tracing::warn!(mode = %other, "unknown STAGE13_RENDERER_MODE; defaulting to daemon");
            Stage13RendererMode::Daemon
        }
    }
}

// New helper the agent writes — mirrors Stage 11's run_landed_cost_skill pattern but for headless contract
// mode (no PTY interaction profile). Returns the parsed result.json as a Value, ready for validate_manifest_skill_output.
async fn execute_manifest_skill_session(
    runtime: &mut RuntimeBootstrap,
    skill: &LoadedSkill,
    bundle: &ManifestSkillBundle,
    cli: &Cli,
    timeout_seconds: u64,
) -> Result<Value> {
    // 1. Build job_parameters from bundle (already shaped — bundle is the contract)
    // 2. INSERT INTO workflow_jobs row (status='pending', kind='generate_manifest', scope_kind='process')
    // 3. claim_job_by_id (executor) so the row gets owner+lease
    // 4. Bounded skill execution. CAUTION: tokio::time::timeout(timeout_seconds, execute_skill_session(...))
    //    only cancels the await — the spawned child process keeps running in the background.
    //    Stage 11's execute_landed_cost_skill_session at main.rs:4070 has the proper pattern:
    //      - explicit child handle tracking
    //      - SIGTERM on timeout, SIGKILL on grace-period expiry
    //      - result-recovery (if result.json was written before timeout, accept it)
    //    For Option B's first try: borrow execute_landed_cost_skill_session and adapt for headless contract
    //    (manifest skill is not pty_interactive). If you build a thinner wrapper instead, document the
    //    leak surface (orphan child processes after stalls) in the validation manifest.
    // 5. Read result.json from session workdir (path: <artifact_root>/sessions/<YYYY>/<MM>/<session_id>/workdir/result.json)
    // 6. Parse as serde_json::Value
    // 7. insert_executed_session_log(conn, skill, &job_id, &executed_session) so R.1 verification has data
    //    (see main.rs:11217 for the helper signature; Stage 11 calls this at lines 1667, 4003, 9629, etc.)
    // 8. Return the parsed Value (NOT validated — validate_manifest_skill_output is the next step in the loop)
    todo!()
}

// In the per-process loop, replace the existing render → serialize step:
let mode = read_stage13_renderer_mode();
let result_json: Value = match mode {
    Stage13RendererMode::Skill => {
        execute_manifest_skill_session(runtime, &manifest_skill, &bundle, cli,
                                       stage13_skill_timeout_seconds).await?
    }
    Stage13RendererMode::Daemon => {
        let rendered = render_manifest_skill_output_from_bundle(&bundle)?;
        serde_json::to_value(&rendered)?
    }
    Stage13RendererMode::Auto => {
        match execute_manifest_skill_session(runtime, &manifest_skill, &bundle, cli,
                                             stage13_skill_timeout_seconds).await {
            Ok(json) => json,
            Err(skill_error) => {
                tracing::warn!(error = ?skill_error, process_id = %bundle.process_id,
                               "skill invocation failed; falling back to daemon-local renderer");
                let rendered = render_manifest_skill_output_from_bundle(&bundle)?;
                serde_json::to_value(&rendered)?
            }
        }
    }
};

// Validation + drift detection runs IDENTICALLY for both paths — drift detection is now meaningful
// when skill produced the JSON, tautological when daemon did:
let output = validate_manifest_skill_output(&manifest_skill.result_schema, &bundle, &result_json)?;

// Embed renderer_mode into the markdown frontmatter post-validation (the validator's typed
// deserialization gives you `output.manifest_markdown` to edit, OR add a `renderer_mode` field
// to ManifestSkillOutput and have render_manifest_skill_output_from_bundle + the skill prompt
// both populate it). Either way, the final persisted manifest must carry `renderer_mode: <value>`
// so R.1 verification can read it from the file.

// Persist as today (no changes):
persist_manifest_generation_output(/* ... */)?;
```

Rules:
- Both render paths produce the SAME `Stage13RenderOutput` shape and BOTH return `Result<Stage13RenderOutput>` (the daemon path can technically be infallible, but wrap in `Ok(...)` for symmetry).
- Drift detection (§2.7 of the original handoff) runs in BOTH modes — in `Daemon`, it's a tautology; in `Skill`/`Auto-success`, it's meaningful.
- After your investigation declares Option B GREEN, change the **default** in `read_stage13_renderer_mode()` from `Daemon` to `Skill`. Until then, `Daemon` remains default so production stays where it is.
- **Persistence of `renderer_mode`**: encoded in the **markdown frontmatter** (no migration). Verifiable by parsing the frontmatter in `manifests.markdown`. The agent extracts it during validation queries via `extract_frontmatter()` (the helper added in Stage 13's §2.8b) or via shell `grep`. **Do not add a `renderer_mode` column to `manifests`** — would require migration 0009 + epoch bump + another `pdf_t_schema_activation_009` rename. Out of scope.
- A second source of truth for "did the skill actually run" is `agent_session_logs` — every successful skill invocation creates a row via `insert_executed_session_log` (`main.rs:11217`). Stage 13's restored skill path **must** call this. R.1 verification cross-checks the frontmatter against the agent_session_logs row count.

---

## §3 — INVESTIGATION PHASE (capture diagnostics)

Goal: turn "the skill stalled" into a concrete root cause from one of the F.1–F.6 categories below.

### §3.1 — Setup

```bash
# Reset the smallest process so it gets re-rendered:
sqlite3 state/state.db "
DELETE FROM manifests WHERE process_id = 'proc-pc-218726bd-9dda-f011-813c-005056b64545';
DELETE FROM process_stage_heads WHERE stage='manifest' AND scope_id='proc-pc-218726bd-9dda-f011-813c-005056b64545';
DELETE FROM process_stage_status WHERE stage='manifest' AND scope_id='proc-pc-218726bd-9dda-f011-813c-005056b64545';
UPDATE pipeline_assessments SET pipeline_state='net_qualified' WHERE process_id='proc-pc-218726bd-9dda-f011-813c-005056b64545';
"
rm -rf artifacts/manifests/proc-pc-218726bd-9dda-f011-813c-005056b64545
```

Run with skill mode + extended timeout + verbose logging:

```bash
RUST_LOG=info,daemon=debug \
STAGE13_RENDERER_MODE=skill \
STAGE13_SKILL_TIMEOUT_SECONDS=900 \
cargo run -p daemon -- --ingest manifest 2>&1 | tee /tmp/stage13_diag_smallest.log
```

If you have not yet introduced `STAGE13_SKILL_TIMEOUT_SECONDS`, add it as a const+env override in `run_manifest_generation_live_loop` so timeouts are tunable without recompilation.

### §3.2 — Capture artifacts

After the run (success or stall), collect:

| Artifact | Path | What you're looking for |
|---|---|---|
| Daemon log | `/tmp/stage13_diag_smallest.log` | tracing events: skill load, session spawn, prompt write, output read, parser events, timeout fired |
| Session dir | `artifacts/sessions/{YYYY}/{MM}/session-pty-{HASH}/` | terminal-output.txt (the agent's stream-json output), stdout, stderr, structured_trace.json |
| Result contract | `<session_dir>/workdir/result.json` | did the skill ever write a result? if so, is it well-formed? if not, where did it stop? |
| `agent_session_logs` row | `sqlite3 state/state.db "SELECT * FROM agent_session_logs WHERE skill_path LIKE '%manifest-generation%' ORDER BY started_at DESC LIMIT 1"` | started_at, completed_at, status, terminal_output_path, pty_session_id |

Time-stamp every step. The breakdown that matters:

- T0 = session spawn
- T1 = first stream-json event received (proves the LLM started)
- T2 = result.json written (proves the LLM finished output)
- T3 = exit code received

If T1 is small but T2 never arrives: **output buffering / parser issue** → §4 F.5
If T1 is large (>30s): **model latency / cold start** → §4 F.3 or F.4
If T0–T1 is large (>60s): **CLI startup / authentication** → §4 F.6
If exit code is non-zero with timeout: **need higher ceiling AND likely a real bug** → §4 F.1 + investigate

### §3.2.5 — `extract_frontmatter` helper (the agent must build this)

Stage 13's actual build skipped the frontmatter parser (the daemon-local renderer constructs the markdown WITH frontmatter directly, no need to round-trip). Option B's R.1 verification + drift checks need it. Code shape (no new dependency — `serde_yaml` is already available):

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

Add to `crates/daemon/src/main.rs`. Used by the new T.21 (round-trip drift check on the markdown body) and by the validation queries (read `renderer_mode` field).

### §3.3 — Categorize the stall

Based on §3.2 measurements, assign one of:

| Category | Typical signature | Most likely fix |
|---|---|---|
| **Timeout-only** | Skill produces output but slow; result.json appears just past the timeout | F.1 raise timeout |
| **Payload-too-large** | Slow on largest process, fast on smallest | F.4 trim bundle |
| **Output-too-large** | T1 fast, T2 never arrives, terminal-output.txt is multi-MB | F.5 protocol parser hardening (re-use Stage 11's pattern) |
| **Model latency** | Slow start (T0–T1) on every call; no protocol issues | F.3 use smaller model |
| **CLI/auth issue** | Spawn succeeds but no events ever; or auth-prompted | F.6 fix CLI auth/config |
| **Concurrent rate-limit** | First 1–2 processes succeed, then stalls | F.2 reduce concurrency / serialize |

Document the category in the validation manifest. If multiple apply, fix the dominant one first.

---

## §4 — FIX CANDIDATES (apply in order, smallest-impact first)

Each fix below is independently testable. Apply one, re-run §3.1, see if it resolves. If not, layer the next.

### F.1 — Raise timeout (smallest fix)

`std::env::var(...)` is **runtime**, not const-evaluable, so `const` won't compile. Use a runtime read at the top of `run_manifest_generation_live_loop`:

```rust
let stage13_skill_timeout_seconds: u64 = std::env::var("STAGE13_SKILL_TIMEOUT_SECONDS")
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(600);
```

Pass `stage13_skill_timeout_seconds` into `render_via_skill`. Default: 600s. Stage 11's `LANDED_COST_SKILL_TIMEOUT_SECONDS = 300` (at `main.rs:254`) is a hard `const`; Stage 13's takes the env-overridable shape because the diagnostic phase needs to vary it without recompilation. After Option B is GREEN with a stable working timeout, you may convert to a `const` for hygiene (or leave the env override as a permanent escape valve).

### F.2 — Concurrent skill invocation

`futures` is **not** in `Cargo.toml`. `futures::stream::buffer_unordered` is not available without adding the dep. Two viable options:

**Option F.2a (only if F.2b's wall time is unacceptable): tokio::task::JoinSet**

```rust
use tokio::task::JoinSet;
let mut tasks = JoinSet::new();
for bundle in bundles {
    let skill = manifest_skill.clone();          // LoadedSkill derives Clone — verified
    let cli = cli.clone();                       // Cli does NOT derive Clone today — see caveat
    let timeout_seconds = stage13_skill_timeout_seconds;
    // runtime is &mut and cannot be moved; see caveat below
    tasks.spawn(async move {
        render_one_with_skill(skill, bundle, cli, timeout_seconds).await
    });
}
let mut outputs = Vec::new();
while let Some(result) = tasks.join_next().await {
    outputs.push(result??);
}
```

**Caveat 1 — `Cli` is NOT `Clone` today**: `struct Cli` derives only `Debug + Parser` (verified). `cli.clone()` won't compile until you add `#[derive(Clone)]` to `Cli`. This is a safe one-line addition (clap-derive structs are usually Clone-safe), but it's a change. Alternative: pull only the bits of Cli needed (`adapter_kind`, `state_root`, etc.) into a small new owned struct and clone that.

**Caveat 2 — runtime borrow**: `runtime: &mut RuntimeBootstrap` cannot be moved into a `tokio::spawn` task without an `'static` bound. Two ways to handle this:

- **Refactor `render_one_with_skill` to NOT need the full runtime** — pass only what's needed (`artifact_root`, `state_db_path`). The skill invocation opens its own connection per task. More work but cleaner.
- **Stay serial (F.2b — strongly preferred for MVP)** — accept higher wall time. With 7 processes × ~30–60s = 3.5–7 minutes, this is still in R.14's <5 min budget if individual calls are fast. F.1 (raised timeout) alone may be enough.

**Option F.2b: serial with raised timeout** — preferred if F.2a's refactor is invasive. Total wall time = sum of per-process latencies. Acceptable if each <60s.

**Caveat**: if the stall was caused by a single-process issue (payload size, model latency on a specific input), concurrency doesn't help — it just makes the same N stalls happen in parallel.

Recommendation: try F.2b (serial + F.1 timeout raise) first. Move to F.2a only if total wall time exceeds the R.14 budget after F.1 succeeds.

### F.3 — Pin a faster model for this skill

The skill declares `model_profile: structured_extract_low_cost` which (per `crates/daemon/src/main.rs` model resolution) maps to a Haiku-class model. Verify this at runtime — if it's accidentally resolving to Sonnet, that explains the latency.

```bash
grep -A 5 "structured_extract_low_cost" crates/daemon/src/main.rs | head -20
# Look for the actual model name being passed to the CLI
```

If it's resolving to a slow model, override in SKILL.md with a more explicit profile, OR add a Stage-13-specific model override in the skill prompt.

### F.4 — Trim the bundle

The skill needs enough data to render markdown sections A–F. Some bundle fields are evidence-trail IDs the skill renders verbatim; others are computed metrics. Trim:

| Field | Necessary? | Risk if removed |
|---|---|---|
| `header.notice_id`, `process_id`, etc. | YES — section A | breaks header |
| `decision_summary.*` | YES — section B | breaks summary |
| `candidates[].confidence.*` | YES — operator wants to see confidences | breaks per-candidate detail |
| `candidates[].selected_quote.source_url` | YES — evidence trail | breaks evidence section |
| `candidates[].landed_cost.*` | YES — evidence | breaks evidence section |
| `coverage.*` | YES — section F | breaks coverage |
| `clarification_prompts.[].rationale` | maybe — extra context for operator | mild — operator may want the why |
| **Anything that's deterministic and ALSO present in `clarification_prompts_json` column** | NO — drop from bundle, daemon re-derives post-render | none, since these are pass-through |

Realistically: the bundle is compact. Trimming saves 10–30% input tokens but unlikely to fix a hard stall. Use only if F.1 + F.2 don't.

### F.5 — Protocol-parser hardening (re-use Stage 11 pattern)

If output buffering or parser races are the issue (T1 fast, T2 never arrives), borrow the Stage 11 pattern from `execute_landed_cost_skill_session` at `main.rs:3878`:

- **Result-recovery after timeout**: if the timeout fires but `result.json` was already written to the workdir, accept the recovered contract instead of failing. (Stage 11 hardening implemented this; reuse the logic.)
- **Result-recovery after protocol parse errors**: if stream-json parsing fails but `result.json` exists, accept it.
- **Tighter result extraction**: only accept agent_message envelopes with the expected payload structure (Stage 11 already does this for landed-cost; mirror for manifest).

This is the most code-heavy fix. Don't apply it unless §3 diagnostics actually show output-buffering symptoms.

### F.6 — CLI/auth diagnostics

If T0–T1 is large (CLI startup), check:

```bash
# Verify claude CLI is on PATH and authenticated:
which claude
claude --version
echo '{"name":"test"}' | claude --output-format stream-json --verbose 'Echo back the input as JSON' 2>&1 | head -20
# Should produce stream-json events. If it hangs or prompts for login, that's F.6.

# Verify subscription quota isn't depleted (Pro/Max accounts have monthly limits):
# (no programmatic check; check the CLAUDE_CONFIG_DIR for any session metadata or just open the desktop app and check usage)
```

If CLI startup is the issue, this is operator-environment, not a code fix. Document in the manifest.

### F.7 — (Last resort) Fall back to Option C

If F.1–F.6 don't fix it, route only the `suggested_action_reasoning` field through the skill (small payload, small output) and keep daemon-local for the markdown body. This is Option C from the prior conversation, not Option B. **If you reach this point, STOP and consult the user — they explicitly chose Option B.** Document findings in the validation manifest's "Issues Remaining" and ask whether to switch to C or stay on the daemon fallback (A).

---

## §5 — IMPLEMENTATION ORDER

1. **Add the feature flag** (§2). Default `Daemon`. Confirm production behavior unchanged.
2. **Add `STAGE13_SKILL_TIMEOUT_SECONDS`** (§4 F.1). Default 600s. Tunable.
3. **Run §3 diagnostic capture** with `STAGE13_RENDERER_MODE=skill` on the smallest process. Categorize per §3.3.
4. **Apply the matching fix from §4**. Re-run on smallest process. Confirm it succeeds with skill_session_id populated.
5. **Run on the largest process** (5 candidates). If it succeeds: full skill mode is viable. If it stalls: layer the next fix.
6. **Run on all 7 processes** (full `--ingest manifest`). Confirm wall time and outcome.
7. **Flip default in `read_stage13_renderer_mode()`** to `Skill`.
8. **Add the new tests** (§7).
9. **Run full quality gate** (broad test + per-file integration loop).
10. **Write the validation manifest**.

---

## §6 — EXPECTED RESULTS

After Option B is GREEN with `STAGE13_RENDERER_MODE=skill`:

| Field | Expected |
|---|---|
| `manifests` row count | 7 (unchanged) |
| `stoplight` distribution | clarify=7 (unchanged — daemon-computed) |
| `suggested_action` labels | unchanged from Stage 13 manifest (daemon-computed) |
| `clarification_prompts_json` | byte-equivalent to current (daemon-computed; pass-through verified by drift detection) |
| `markdown` body | **Different** from current — LLM-rendered prose for sections A–F. Likely longer, more narrative-shaped |
| `suggested_action_reasoning` | **Different** from current — 1–2 sentence LLM paragraph anchored to specific candidate names + reason codes |
| `manifest_basis_hash` | Unchanged from current (composition didn't change) |
| `artifact_hash` | **Different** — sha256 of new markdown bytes |
| Markdown frontmatter `renderer_mode` | `skill` for all 7 rows (was: not present, or `daemon` after Stage 13's daemon-local fallback) |
| `agent_session_logs` rows | 7 new rows with `skill_path LIKE '%manifest-generation%'`, `status='succeeded'`, `started_at` after the run start time |
| `confidence_breakdown.skill_invocation_outcome` (or frontmatter) | `'skill'` for all 7 if pure skill mode; `'fallback_to_daemon'` for any that hit the auto-fallback |

Pipeline 1→13 invariants must remain intact (R.15, R.16 of original Stage 13).

---

## §7 — REQUIRED NEW UNIT TESTS

Add to `crates/daemon/src/main.rs` (`mod tests`). These are NEW; T.1–T.21 from the original Stage 13 handoff are already in place.

| # | Test name | What it verifies |
|---|---|---|
| T.22 | `stage13_renderer_mode_defaults_to_daemon_when_env_unset` | env unset → mode = Daemon |
| T.23 | `stage13_renderer_mode_skill_routes_through_skill_path` | env=skill → invokes execute_skill_session (mocked) |
| T.24 | `stage13_renderer_mode_auto_falls_back_when_skill_errors` | env=auto + skill mock fails → renders via daemon, sets skill_invocation_outcome='fallback_to_daemon' |
| T.25 | `stage13_renderer_mode_unknown_value_logs_warning_and_defaults_to_daemon` | env=garbage → warns + Daemon |
| T.26 | `skill_invocation_outcome_persisted_in_frontmatter` | a synthetic skill render → frontmatter has `renderer_mode: skill` |
| T.27 | `auto_fallback_outcome_persisted_in_frontmatter` | auto-mode + skill error → frontmatter has `renderer_mode: auto_fallback_to_daemon` |
| T.28 | `deterministic_fields_byte_stable_across_skill_renders` | Same bundle through skill twice → stoplight, suggested_action, clarification_prompts identical (LLM may give different prose, but pass-through fields don't drift) |
| T.29 | `skill_render_drift_detection_rejects_diverging_stoplight` | mocked skill output with stoplight=pursue, bundle has clarify → `validate_manifest_skill_output` errors with reason `stoplight_drifted_from_bundle` (existing logic; this test confirms it fires under the new skill path). The existing validator emits 9+ specific drift reason codes — see §0; T.29 covers stoplight specifically; T.20 covers clarification_prompts (`clarification_prompts_drifted` reason code) |
| T.30 | `skill_render_reasoning_anchored_to_candidate_names_or_reason_codes` | mocked skill output with reasoning="generic boilerplate" → `validate_manifest_skill_output` errors with `suggested_action_reasoning_not_anchored` (existing helper `manifest_reasoning_is_anchored(bundle, reasoning)` already implements this; T.30 confirms it fires under the new skill path) |

Total: 9 new tests. Combined with T.1–T.21: 30 named tests in main.rs's mod tests for Stage 13. After this work: **120 prior + 9 = 129 binary unit tests**.

T.22–T.27 are pure-function or env-var tests (fast). T.28–T.30 require a mockable skill invocation interface — refactor `render_via_skill` to take a trait (`SkillInvoker`) so tests can inject a fake.

---

## §8 — ACCEPTANCE CRITERIA — R.1 to R.16

Each must PASS before declaring done. Validation queries inline in §9.

| # | Criterion |
|---|---|
| R.1 | Run `STAGE13_RENDERER_MODE=skill cargo run -p daemon -- --ingest manifest`. All 7 manifests rendered via skill, verified by **two cross-checked signals**: (a) markdown frontmatter has `renderer_mode: skill` (parse the YAML block from `manifests.markdown`); (b) `agent_session_logs` has 7 NEW rows with `skill_path LIKE '%manifest-generation%'` AND `started_at > {run_start_iso}`. Both must be true |
| R.2 | `agent_session_logs` rows from R.1 all have `status='succeeded'` and distinct `pty_session_id` values |
| R.3 | Stoplight distribution unchanged: clarify=7 (the daemon-computed deterministic fields are unaffected by the path switch) |
| R.4 | Per-process `clarification_prompt_count` matches the original Stage 13 §3 table (5/1/5/2/2/1/2 = 18 total) |
| R.5 | `suggested_action_reasoning` non-empty for all 7, each <=2000 chars, **each contains at least one candidate name OR one reason_code from the bundle** (anchoring check, T.30 enforces) |
| R.6 | Drift detection passes for all 7 (skill echoes pass-through fields unchanged) |
| R.7 | Frontmatter consistency: every manifest's frontmatter `stoplight` and `clarification_prompts` matches the bundle |
| R.8 | Markdown artifact files at `artifacts/manifests/{process_id}/manifest.md` for all 7; byte-equivalent to `manifests.markdown` column; sha256 = `artifacts.artifact_hash` |
| R.9 | `pipeline_assessments.pipeline_state` for all 7 still `'manifest_ready'`; ALL other Stage 12-owned columns unchanged from pre-Option-B snapshot (snapshot before the run, diff after) |
| R.10 | Stage 11/12 untouched: 4 `landed_cost_estimates` rows still `decision_grade='review'`; DGU row `5160\|17692\|0\|4570\|27422`; 17 `candidate_net_margin_assessments` rows still `verdict='review'` |
| R.11 | Idempotency: a second `--ingest manifest` (skill mode) produces zero new rows AND zero artifact-file rewrites (manifest_basis_hash matches → skip). Log line reports `skipped_unchanged=7` |
| R.12 | Re-runnability after delete: deleting all manifests rows + artifact files + re-running produces 7 manifests with **same stoplights, same clarification_prompts** (deterministic). Markdown body and reasoning text **may differ** from original due to LLM non-determinism — that's acceptable per D4. Fail R.12 only if deterministic fields differ |
| R.13 | `auto` mode behavior: kill the skill mid-run (e.g., force a timeout to 1s temporarily); verify `STAGE13_RENDERER_MODE=auto` falls back to daemon; verify the resulting manifest has `skill_invocation_outcome='fallback_to_daemon'` in frontmatter |
| R.14 | Wall time per `--ingest manifest`: **<5 minutes total** for all 7 processes (target). If serial mode + 600s timeout pushes past this, F.2 concurrency must be applied |
| R.15 | Default `STAGE13_RENDERER_MODE` (env unset) is `Skill` AFTER the work completes (was `Daemon` before). Verified by T.22 plus a manual check of the source |
| R.16 | Quality gates pass: `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p daemon --bin daemon` (T.22–T.30 included → expect 120 prior + 9 = **129 binary unit tests pass**), per-file integration loop green across all **39 files** |

---

## §9 — VALIDATION QUERIES

Run after Option B's first successful skill-mode `--ingest manifest`:

Capture run start before invoking, then verify against agent_session_logs:

```bash
RUN_START=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
STAGE13_RENDERER_MODE=skill cargo run -p daemon -- --ingest manifest

sqlite3 -header -column state/state.db "
SELECT '=== R.1 (b) agent_session_logs has 7 new rows for manifest-generation ===';
SELECT skill_path, status, COUNT(*) AS n FROM agent_session_logs
WHERE skill_path LIKE '%manifest-generation%' AND started_at >= '$RUN_START'
GROUP BY skill_path, status;

SELECT '=== R.2 distinct pty_session_id values + status=succeeded ===';
SELECT COUNT(DISTINCT pty_session_id) AS distinct_sessions,
       SUM(CASE WHEN status='succeeded' THEN 1 ELSE 0 END) AS succeeded,
       COUNT(*) AS total
FROM agent_session_logs
WHERE skill_path LIKE '%manifest-generation%' AND started_at >= '$RUN_START';
"

# R.1 (a) frontmatter renderer_mode check (parse YAML in markdown column)
echo "=== R.1 (a) renderer_mode in markdown frontmatter ==="
sqlite3 -separator $'\t' state/state.db "SELECT process_id, artifact_locator FROM manifests" | while IFS=$'\t' read -r pid loc; do
  mode=$(grep -E "^renderer_mode:" "$loc" | head -1 | awk '{print $2}')
  echo "$pid: ${mode:-MISSING}"
done
# Expect every line ending in 'skill' (or 'auto_fallback_to_daemon' if auto mode hit fallback)

sqlite3 -header -column state/state.db "

SELECT '=== R.3 stoplight unchanged ===';
SELECT stoplight, COUNT(*) FROM manifests GROUP BY stoplight;

SELECT '=== R.4 prompt counts unchanged ===';
SELECT process_id, json_array_length(clarification_prompts_json) FROM manifests ORDER BY process_id;

SELECT '=== R.5 reasoning anchoring (manual eyeball each) ===';
SELECT process_id, suggested_action_reasoning FROM manifests ORDER BY process_id;
-- Each row should reference at least one candidate name fragment OR reason_code

SELECT '=== R.9/R.10 Stage 11/12 untouched ===';
SELECT decision_grade || '|' || supplier_cost_gbp_pence || '|' || freight_base_gbp_pence
       || '|' || customs_duty_gbp_pence || '|' || import_vat_gbp_pence
       || '|' || economic_landed_cost_gbp_pence
FROM landed_cost_estimates WHERE supplier_quote_id = 'sq-b22fcf00b7a545d699c3438c42b9027c-1';
SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY verdict;
SELECT pipeline_state, COUNT(*) FROM pipeline_assessments GROUP BY pipeline_state;
"
```

R.11 idempotency:

```bash
ROW_COUNT_BEFORE=$(sqlite3 state/state.db "SELECT COUNT(*) FROM manifests")
HASH_BEFORE=$(sqlite3 state/state.db "SELECT artifact_hash FROM manifests ORDER BY process_id" | shasum -a 256)
STAGE13_RENDERER_MODE=skill cargo run -p daemon -- --ingest manifest
ROW_COUNT_AFTER=$(sqlite3 state/state.db "SELECT COUNT(*) FROM manifests")
HASH_AFTER=$(sqlite3 state/state.db "SELECT artifact_hash FROM manifests ORDER BY process_id" | shasum -a 256)
echo "rows: $ROW_COUNT_BEFORE -> $ROW_COUNT_AFTER (must equal)"
echo "hashes: $HASH_BEFORE -> $HASH_AFTER (must equal — basis_hash skip means no LLM call)"
```

R.13 auto-fallback (use FULL process id, not abbreviated):

```bash
PID='proc-pc-218726bd-9dda-f011-813c-005056b64545'

# Snapshot current state
sqlite3 state/state.db "SELECT process_id, artifact_hash FROM manifests ORDER BY process_id" > /tmp/r13_before.tsv

# Reset one process and force a skill failure (1s timeout = guaranteed stall)
sqlite3 state/state.db "
DELETE FROM manifests WHERE process_id = '$PID';
DELETE FROM process_stage_heads WHERE stage='manifest' AND scope_id='$PID';
DELETE FROM process_stage_status WHERE stage='manifest' AND scope_id='$PID';
UPDATE pipeline_assessments SET pipeline_state='net_qualified' WHERE process_id = '$PID';
"
rm -rf "artifacts/manifests/$PID"

STAGE13_RENDERER_MODE=auto STAGE13_SKILL_TIMEOUT_SECONDS=1 cargo run -p daemon -- --ingest manifest

# Verify fallback was used (renderer_mode in frontmatter)
LOC=$(sqlite3 state/state.db "SELECT artifact_locator FROM manifests WHERE process_id = '$PID'")
echo "renderer_mode for $PID:"
grep -E "^renderer_mode:" "$LOC" | head -1
# Expect: renderer_mode: auto_fallback_to_daemon
```

R.14 wall time:

```bash
time STAGE13_RENDERER_MODE=skill cargo run -p daemon -- --ingest manifest
# Total real time should be <5 minutes for 7 processes
```

---

## §10 — FAILURE-MODE PLAYBOOK

| Symptom | Likely cause | Remediation |
|---|---|---|
| Smallest process (1 candidate) stalls past 900s | Fundamental issue (CLI auth, prompt malformed, agent stuck in tool-use loop) | F.6 CLI diagnostics. Check `claude --version`, run a smoke-test invocation manually, examine session_dir for stuck patterns. If reproducible: stop and consult user (per D5) |
| Largest process (5 candidates) stalls but smallest succeeds | Payload-size-dependent | F.4 trim bundle, OR F.2 concurrency-with-shorter-individual-timeout |
| All processes succeed but wall time >10min | Just slow, not stalled | F.2 concurrency (3-way buffer cuts time by ~3x), AND/OR F.3 verify model isn't accidentally Sonnet-class |
| Skill output schema validation fails | SKILL.md prompt drifted; LLM emitting old shape | Re-read `skills/manifest-generation/SKILL.md` — does it explicitly require all 8 result fields? Update prompt. Re-run |
| Drift detection fires (stoplight mismatch) | LLM is re-deriving instead of passing through | Update SKILL.md to explicitly state: "These fields are provided in `job_parameters` and must be echoed back unchanged: stoplight, suggested_action, clarification_prompts, candidate_count, manifest_basis_hash. Do NOT recompute or modify them." |
| `suggested_action_reasoning` is generic ("This opportunity needs review") | Skill prompt isn't requiring anchoring | SKILL.md must require: "reasoning must reference at least one specific candidate name OR specific reason_code from the bundle. Generic phrases like 'this opportunity needs review' are rejected by anchoring check (T.30)" |
| `auto` mode falls back 100% of the time | Skill errors on every attempt | This is just `daemon` mode with extra latency. Don't ship. Investigate why skill always fails; likely missed §3 diagnostic step |
| Idempotency broken: re-run rewrites all 7 | basis_hash includes a non-stable input (e.g., timestamp) | basis_hash composition must NOT include `Utc::now()` or anything non-deterministic. Verify §2.7 of original Stage 13 handoff |
| `cargo test` count off (not 129) | Forgot to add T.22–T.30, OR existing tests broken | Re-read §7. `grep -c '#\[test\]\|#\[tokio::test\]' crates/daemon/src/main.rs` to count |
| Modified anything outside Stage 13 path / skill files | Out of scope | Stop. Stage 11, 12, and Stage 13's daemon-local renderer all stay |

---

## §11 — CONSTRAINTS

1. **Do not modify Stage 11 or Stage 12 code.** Stage 13 changes are scoped to: `run_manifest_generation_live_loop` (and its helpers), `skills/manifest-generation/*`, the new tests, and the validation manifest.
2. **Keep the daemon-local renderer.** It's the auto-fallback path AND the emergency rollback path (`STAGE13_RENDERER_MODE=daemon`). Do not delete it.
3. **Default mode flips to `Skill` only after Option B is GREEN.** Until then, default is `Daemon` so production stays where it is.
4. **No silent test exclusion.** Per AGENTS.md `### Stage Validation Test Gate`: full binary unit tests + per-file integration loop are mandatory.
5. **No breaking changes to the result.schema.json or SKILL.md fields.** Both were updated for Stage 13; Option B reuses the contract.
6. **No new dependencies.** `serde_yaml`, `sha2`, `futures` are already available.
7. **Drift detection cannot be weakened to make the skill pass.** If the LLM re-derives instead of echoing, fix the SKILL.md prompt — don't relax the drift check.
8. **No Anthropic API keys.** This project uses the operator's logged-in Claude subscription via the headless CLI. Do not introduce direct API calls.
9. **No PII or sensitive data in skill prompts beyond what's already in the bundle.** Stage 13 bundles never include credentials, tokens, or auth artifacts. If you need to add a field to the bundle, double-check it.
10. **Validation manifest is mandatory.** Even if Option B works on the first try, write the manifest. Future debugging needs the diagnostic record.

---

## §12 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/stage-13-skill-invocation-restoration-manifest.md`. Use this template **verbatim**:

```markdown
# Stage 13 Skill Invocation Restoration (Option B) Validation Manifest

> Generated: {ISO timestamp}
> Status: {GREEN | YELLOW | RED}

## Summary
{1 paragraph: did Option B succeed? what was the stall root cause? what fixes were applied? what's the new wall time? are all 7 manifests now skill-rendered?}

## Investigation Findings
{Diagnostic data from §3.2 + categorization from §3.3:
 - Stall category: timeout-only / payload-too-large / output-too-large / model-latency / cli-auth / concurrent-rate-limit / other
 - Smallest-process T0/T1/T2/T3 timestamps
 - Largest-process T0/T1/T2/T3 timestamps
 - Total wall time for full --ingest manifest (skill mode)
 - Logs path (/tmp/stage13_diag_*.log)}

## Fixes Applied
| Fix | Applied? | Effect |
|---|---|---|
| F.1 raise timeout | YES/NO | {observed effect} |
| F.2 concurrency | YES/NO | {observed effect} |
| F.3 model pin | YES/NO | {observed effect} |
| F.4 bundle trim | YES/NO | {observed effect} |
| F.5 protocol parser hardening | YES/NO | {observed effect} |
| F.6 CLI/auth | YES/NO | {observed effect} |

## Per-Process Skill Invocation Result
| Process | session_id | wall_clock_s | outcome |
|---|---|---|---|

## R.1–R.16 Acceptance Criteria
| # | Criterion | Result | Detail |

## Quality Gates
- cargo fmt --check: {PASS/FAIL}
- cargo check --all-targets: {PASS/FAIL}
- cargo clippy --all-targets -- -D warnings: {PASS/FAIL}
- cargo test -p daemon --bin daemon: {PASS/FAIL — N tests, includes T.22–T.30 (target: 129 total)}
- per-file integration loop: {PASS/FAIL — 39 files}

## Code Changes
| File | Change |

## Default Mode After This Work
- `read_stage13_renderer_mode()` default: {Skill / Daemon / Auto}
- Reasoning: {why this default makes sense given findings}

## Issues Remaining
{Anything observed but not fixed; e.g., wall time still >5min, intermittent fallbacks, etc.}

## Out-of-Scope (Documented, Not Fixed Here)
- Operator override mechanism + UI binding (Stage 14)
- Wiring `--ingest manifest` into `run_deterministic_pipeline`
- Manifest staleness TTL / scheduled re-render
```

### Quality gates (the broader gate, mandatory per AGENTS.md)

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon                       # expect 129 tests
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

If any single integration test target fails, fix the root cause. Do **not** weaken or `#[ignore]` to bypass.

---

## §13 — REMINDERS

- ULTRATHINK and be meticulous.
- The goal is **skill_session_ids non-NULL on all 7 manifests**. If you finish with the daemon-local renderer still in the production path, the work isn't done — even if all R criteria appear green.
- **Diagnose before fixing.** §3 is not optional. The Stage 13 manifest's "Issues Found" was vague ("stalled"). Your manifest must categorize the stall concretely.
- The drift-detection logic that was tautological in daemon-only mode becomes **genuinely meaningful** with the LLM in the loop. T.29 specifically tests that the skill cannot drift from the bundle without the daemon catching it.
- The `auto` mode is a production safety net. **It must work** (R.13) but must not be the default-path-by-design. If Option B's `auto` mode is silently falling back 100%, that's the same as Option A but slower.
- T.30 reasoning anchoring is the fuzzy check — the LLM must reference something specific. If it produces "this opportunity needs review" generically, reject. The prompt has to demand specificity.
- **If the skill genuinely cannot be made reliable** (stalls reproducibly on the smallest process even after F.1–F.6), STOP and write a RED manifest. Do not ship "auto mode that always falls back" pretending Option B works. Per D5: that's a hard stop.
- The cost is wall-time, not dollars (your subscription, not API). Tune timeouts and concurrency to fit the operator's tolerance for `--ingest manifest` latency.
- Once GREEN: commit. Stage 14 (Operator Decision) is the next stage; it's a separate handoff and depends on `manifests` being skill-rendered (or at least skill-render-capable) for prose quality at the operator UI.
