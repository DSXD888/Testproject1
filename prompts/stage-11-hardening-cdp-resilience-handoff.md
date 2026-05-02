# Stage 11 Hardening — CDP Captcha Resilience for the SeaRates Skill

ULTRATHINK and be meticulous.

## MISSION

Stage 11's SeaRates freight skill (`skills/landed-cost/searates-freight-estimate/`) drives a real CDP-controlled Chromium to fetch live freight quotes. When SeaRates serves a captcha, the current behavior **stalls until the bounded skill timeout fires**, then the daemon recovers via `execute_landed_cost_skill_session`'s timeout-recovery path — but the resulting `landed_cost_estimates` row carries `SEARATES_NO_RESULT` and `CDP_CAPTCHA_INTERCEPTION` in its blockers, with stale or missing freight data.

The DGU row in the live DB shows this exact symptom (`blockers=[CDP_CONNECT_FAILED, SEARATES_NO_RESULT]` per the most recent `--ingest landed-cost` runs in environments without a working CDP). Today the operator has no recourse other than spinning up CDP manually, solving the captcha, and re-triggering the skill.

This handoff makes the CDP captcha + connection failure modes **first-class, observable, and recoverable**:
1. The daemon **detects** captcha state explicitly (skill output carries a structured `captcha_intercepted: true` signal — not a silent stall).
2. The daemon **retries** N times with exponential backoff when CDP is reachable but a captcha was served.
3. The daemon **falls back** to a cached previous estimate when one exists and is fresh enough.
4. When all else fails, the daemon **emits a structured operator notification** (a row in a new `freight_estimate_review_queue` table OR an existing manifest blocker that the desktop UI can surface) so the operator is prompted to solve the captcha once and re-run.

You will **build → validate → test → fix in a loop** until every numbered acceptance criterion (R.1–R.13 below) is GREEN. Specifically: the DGU row's freight blockers can be cleared via the new resilience path (cached fallback OR retry success), and the system never silently stalls on a captcha again.

```
┌────────────────────────────────────────────────────────────┐
│                          THE LOOP                            │
│   1. Pre-flight (§0)                                         │
│   2. Implement: skill captcha-detection signal + daemon-side│
│      retry-with-backoff + cached-freight fallback +          │
│      operator-notification surface                          │
│   3. Diagnose: drive --ingest landed-cost in two modes —    │
│      with CDP up + captcha-free, with CDP up + captcha,      │
│      with CDP down                                          │
│   4. Validate against R.1..R.13 (queries inline)             │
│   5. Failures? → diagnose, fix, repeat from step 3           │
│   6. Quality gates: full test suite per AGENTS.md            │
│   7. All green → write the validation manifest               │
└────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## §0 — PRE-FLIGHT

| Claim | Verify |
|---|---|
| `execute_landed_cost_skill_session` at `crates/daemon/src/main.rs:4497` is the bounded skill runner with timeout-recovery + protocol-parser-recovery added during prior Stage 11 hardening. **This handoff modifies it** to add retry + cached-fallback + structured captcha detection | `grep -nB2 -A30 "fn execute_landed_cost_skill_session" crates/daemon/src/main.rs` |
| `skills/landed-cost/searates-freight-estimate/SKILL.md` declares `interaction_profile: pty_interactive`, **`agent_cli: codex`** (NOT `claude` — the SeaRates skill currently runs under Codex), `model_profile: browse_stable_low_cost`, `requires_cdp: true`, `cdp_default_port: 9222`. The skill prompt drives CDP via the persistent browser at `localhost:9222`. **This handoff modifies the SKILL.md prompt + result.schema.json** to require an explicit `captcha_intercepted` field | `head -20 skills/landed-cost/searates-freight-estimate/SKILL.md` |
| `skills/landed-cost/searates-freight-estimate/result.schema.json` defines the existing freight result contract. **Adding `captcha_intercepted: bool` and `cdp_state: enum` requires schema extension**. The folder currently contains ONLY `SKILL.md` and `result.schema.json` — no `canary.inputs.jsonl` or `golden.assertions.yaml` exist yet. If you add new fields and want canary/golden coverage, you will be CREATING these files (use `skills/manifest-generation/canary.inputs.jsonl` and `skills/source-health/fts/golden.assertions.yaml` as references for shape) | `ls skills/landed-cost/searates-freight-estimate/` |
| The DGU row's existing blockers can be inspected: `[CUSTOMS_SKILL_NO_RESULT, CDP_CONNECT_FAILED, SEARATES_NO_RESULT]` per the live state.db (verified 2026-04-30) | `sqlite3 state/state.db "SELECT blockers FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1'"` |
| `landed_cost_estimates.warnings` and `.blockers` are both JSON arrays of `{code, message, severity}` per the Stage 11 hardening contract | `sqlite3 state/state.db "SELECT warnings, blockers FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1'"` |
| `LANDED_COST_SKILL_TIMEOUT_SECONDS = 300` at `main.rs:256` is a `const`. Retries should NOT invoke the same skill 3× at 300s each (15-minute total stalls); the retry loop must use a smaller per-attempt budget | `grep -n "LANDED_COST_SKILL_TIMEOUT_SECONDS" crates/daemon/src/main.rs` |
| `agent-mail` healthy; commits need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Parallel-safety note**: this handoff modifies `crates/daemon/src/main.rs` near the Stage 11 skill runner (~line 4497), plus `skills/landed-cost/searates-freight-estimate/{SKILL.md, result.schema.json}` (and possibly newly created `canary.inputs.jsonl`/`golden.assertions.yaml`). The order-quantity hardening handoff (running in parallel) modifies `infer_order_quantity_basis` (~line 3821) — **non-overlapping**. Use `git worktree add ../testproject1-stage11-cdp main` for clean isolation. Merge to main with rebase at the end. **Do NOT touch `infer_order_quantity_basis` or `OrderQuantityBasis`** — that's the order-quantity handoff's territory.

---

## §1 — DECISIONS ALREADY MADE

### D1 — Explicit captcha detection in skill output

The SeaRates skill's result contract gets two new fields:

```json
{
  "captcha_intercepted": false,                                 // true iff the agent saw a captcha challenge
  "cdp_state": "connected",                                     // enum: connected | unreachable | captcha_blocked | session_expired
  // ... existing freight fields ...
}
```

Update `result.schema.json` to require both new fields. Update `SKILL.md` to instruct the agent to:
- Set `captcha_intercepted=true` and `cdp_state="captcha_blocked"` immediately if a captcha appears
- Stop interacting (no captcha-solving attempts) and emit the result as soon as the captcha is detected — DO NOT stall the session
- Set `cdp_state="unreachable"` if CDP at `localhost:9222` doesn't respond
- Set `cdp_state="connected"` and `captcha_intercepted=false` on a normal successful run

This converts a silent stall into an **explicit signal** the daemon can act on.

### D2 — Daemon-side retry with exponential backoff

When skill output reports `captcha_intercepted=true` OR `cdp_state="captcha_blocked"`:

- Retry the skill up to **3 times** total (initial + 2 retries)
- Per-attempt timeout: **60 seconds** (default; env-overridable via `STAGE11_SKILL_RETRY_PER_ATTEMPT_SECONDS`)
- Backoff between attempts: **15s → 45s** (only 2 backoffs since 3 attempts have 2 gaps)
- Between retries, the daemon emits a tracing log so the operator can see the loop in real-time
- After 3 failures: stop retrying, fall through to D3 (cached fallback) or D4 (operator notification)

**Budget math** (CRITICAL — must fit within `LANDED_COST_SKILL_TIMEOUT_SECONDS=300`):

| Phase | Time |
|---|---|
| 3 attempts × 60s/attempt (worst-case) | 180s |
| 2 backoffs (15s + 45s) | 60s |
| Cached-freight lookup or operator-notification persistence (D3/D4) | <5s |
| **Total worst-case** | **245s ≤ 300s ✓** |

If you change either the per-attempt timeout or the backoff schedule, **re-do the math** — `attempts × per_attempt + sum(backoffs) + persistence_buffer ≤ 300`.

When `cdp_state="unreachable"`: **do not retry** the skill (CDP isn't going to come back without operator action). Skip to D3 / D4 immediately. Total time in this branch: <5s.

### D3 — Cached freight fallback

When all retries fail (or CDP is unreachable):

1. Look up the most recent successful `landed_cost_estimates` row for this `(process_id, supplier_quote_id)` tuple where `freight_base_gbp_pence > 0` and the row's `created_at` is within **7 days** (configurable via `STAGE11_FREIGHT_CACHE_TTL_DAYS`, default 7).
2. If found: copy `freight_low/base/high_gbp_pence` and `freight_evidence` (or whatever existing freight-detail fields are present) into the current row. Add a warning: `code=FREIGHT_FROM_CACHED_ESTIMATE, message="Captcha/CDP failure; using cached freight from {date}", severity=warning`.
3. The row is persisted as a normal `decision_grade='review'` row with the cached freight + the warning. Stage 12 sees the warning and may downgrade confidence accordingly.

Cache lookup is a single SQL query against `landed_cost_estimates` itself — no new table needed.

### D4 — Operator notification when no fallback applies

When retries fail AND no cached fallback exists:

- Persist the row with `freight_*=0`, `decision_grade='blocked'`, and blockers containing **two** new structured codes:
  - `CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED` — message: "SeaRates served a captcha; please open `localhost:9222` in your browser, solve the captcha manually, then re-run `--ingest landed-cost`"
  - `FREIGHT_NO_CACHED_FALLBACK` — message: "No fresh freight estimate within 7 days; manual operator action needed"
- Emit a structured tracing event `stage11.cdp.operator_action_required` so the desktop UI (Stage 14 Phase 2, when built) can subscribe and show a notification

### D5 — Backwards compatibility

- The DGU row's existing `[CDP_CONNECT_FAILED, SEARATES_NO_RESULT]` blockers should be cleared on the next successful re-run (because the row is replaced atomically by Stage 11). No data migration needed.
- The 3 domestic carpet rows have no freight (domestic), so this handoff has zero effect on them.
- The Aggregates row is also domestic — zero effect.

### D6 — Out of scope

- **Solving the captcha**: not attempted. The retry loop only re-tries the skill (which itself doesn't try to solve captchas). Operator action is required to clear a real captcha.
- **Alternative freight providers** (Freightos etc.): out of scope. Only SeaRates is wired today; adding alternative providers is a separate L5 feature.
- **Persisting cached freight in a separate table**: not needed. The existing `landed_cost_estimates` rows ARE the cache; we just look them up by recency.

---

## §2 — IMPLEMENTATION ORDER

1. Extend `result.schema.json` for SeaRates skill: add required `captcha_intercepted: boolean` + `cdp_state: enum`.
2. Update `SKILL.md` to instruct the agent (Codex, per the SeaRates `agent_cli` setting) to set these fields per D1.
3. Optionally CREATE `skills/landed-cost/searates-freight-estimate/canary.inputs.jsonl` + `golden.assertions.yaml` (these don't exist today — the SeaRates skill folder contains only `SKILL.md` + `result.schema.json`). Use `skills/source-health/fts/canary.inputs.jsonl` and `skills/source-health/fts/golden.assertions.yaml` as shape references. **This is OPTIONAL** — skip if scope feels too large; prioritize the daemon-side retry/cache logic which is the load-bearing part.
4. Update the daemon-side parser (find where `searates-freight-estimate` results are parsed in main.rs) to extract `captcha_intercepted` and `cdp_state`.
5. Wrap the existing skill call in a retry loop with D2 backoff + per-attempt timeout. New constant: `STAGE11_SKILL_RETRY_PER_ATTEMPT_SECONDS_DEFAULT: u64 = 90` + env override.
6. Implement cached-freight lookup helper (D3). New constant: `STAGE11_FREIGHT_CACHE_TTL_DAYS_DEFAULT: u64 = 7` + env override.
7. Implement operator-notification persistence (D4) — extend the blockers/warnings array generation when retries fail.
8. Add unit tests (§4).
9. Drive `--ingest landed-cost` against the live DB in three modes (§3):
   - CDP up, no captcha → normal success path (no retries, no cache)
   - CDP up, simulated captcha → retry → captcha-still-present → cached fallback applied
   - CDP down → no retry → operator-notification blockers persisted
10. Run quality gates.
11. Write the validation manifest.

---

## §3 — DEMO MATRIX

Run these scenarios and capture stdout/log for the validation manifest. **Pre-condition**: ensure a fresh manifest exists (re-run Stages 12+13 after any Stage 11 change).

### Scenario A: CDP up, no captcha (happy path)

```bash
# Pre-check: CDP reachable
curl -sf http://localhost:9222/json/version | head -1
# Expect: a JSON response with browser version

# Reset DGU's lce row to force re-computation
sqlite3 state/state.db "DELETE FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1';
DELETE FROM process_stage_heads WHERE stage='landed cost' AND scope_id LIKE '%glass-dgu%';
DELETE FROM process_stage_status WHERE stage='landed cost' AND scope_id LIKE '%glass-dgu%';"

cargo run -p daemon -- --ingest landed-cost 2>&1 | tee /tmp/stage11_cdp_demo_a.log

# Expect: DGU row has freight_base_gbp_pence > 0, blockers=[], warnings does NOT contain FREIGHT_FROM_CACHED_ESTIMATE
sqlite3 state/state.db "SELECT freight_base_gbp_pence, blockers, warnings FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1'"
```

### Scenario B: CDP up, captcha present (retry → fallback)

**CRITICAL pre-condition (verified 2026-04-30)**: the live DGU lce row has `freight_base_gbp_pence = 0` (cache is empty). Scenario B will fall through to Scenario C (operator-notification) UNLESS Scenario A is run successfully first AND its successful freight row is preserved as the cache target. Three options:

1. **Best**: run Scenario A first end-to-end, confirm a row with `freight_base > 0` is persisted, THEN run Scenario B. The Scenario A row becomes the cache hit for Scenario B.
2. **Acceptable for testing**: seed a synthetic prior row directly via SQL with a realistic freight value and a `created_at` 1-3 days old (then the `(process_id, supplier_quote_id)` lookup finds it). Document the seed in the validation manifest.
3. **Skip if neither possible**: document in the validation manifest that Scenario B couldn't be exercised because no real cache target exists in this environment, and rely on CDP.6 unit test for cache-hit behavior coverage.

```bash
# Simulate captcha by adding job_parameters to the daemon's skill invocation (the SKILL.md prompt
# instructs the Codex agent to emit captcha_intercepted=true when job_parameters.simulate_captcha=true).
# The daemon-side retry loop honors this signal and runs through its full 3-attempt budget.

# (Pre-condition setup per option 1 or 2 above — establish a cached row first.)

# Reset DGU's CURRENT row but PRESERVE older successful row(s) that act as cache
sqlite3 state/state.db "DELETE FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1' AND created_at > date('now','-1 hour');
DELETE FROM process_stage_heads WHERE stage='landed cost' AND scope_id LIKE '%glass-dgu%';
DELETE FROM process_stage_status WHERE stage='landed cost' AND scope_id LIKE '%glass-dgu%';"

# Verify a cache target exists before running:
sqlite3 state/state.db "SELECT created_at, freight_base_gbp_pence FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1' AND freight_base_gbp_pence > 0 ORDER BY created_at DESC LIMIT 1"
# If empty → Scenario B will produce Scenario C-shaped output (operator-notification blockers). Adjust expectation.

# Pass simulate_captcha=1 via job_parameters_json (the daemon-native way, not env var):
cargo run -p daemon -- --ingest landed-cost --job-parameters-json '{"simulate_captcha":true}' 2>&1 | tee /tmp/stage11_cdp_demo_b.log

# Expect IF cache exists: log shows 3 retry attempts; final row has freight_base_gbp_pence > 0 (from cache), warnings contains FREIGHT_FROM_CACHED_ESTIMATE
# Expect IF cache empty: log shows 3 retry attempts; final row has freight_base_gbp_pence=0, blockers contain CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED + FREIGHT_NO_CACHED_FALLBACK (Scenario C shape)
sqlite3 state/state.db "SELECT freight_base_gbp_pence, blockers, warnings FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1' ORDER BY created_at DESC LIMIT 1"
```

### Scenario C: CDP down (no retry → operator notification)

```bash
# Stop CDP / persistent Chromium first (or unset the localhost:9222 endpoint)

# Reset DGU and clear the cache (delete previous successful rows so cached-fallback can't apply)
sqlite3 state/state.db "DELETE FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1';
DELETE FROM process_stage_heads WHERE stage='landed cost' AND scope_id LIKE '%glass-dgu%';
DELETE FROM process_stage_status WHERE stage='landed cost' AND scope_id LIKE '%glass-dgu%';"

cargo run -p daemon -- --ingest landed-cost 2>&1 | tee /tmp/stage11_cdp_demo_c.log

# Expect: log shows cdp_state=unreachable, no retries (skipped); row has decision_grade='blocked', blockers contains CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED + FREIGHT_NO_CACHED_FALLBACK
sqlite3 state/state.db "SELECT decision_grade, blockers FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1' ORDER BY created_at DESC LIMIT 1"
```

---

## §4 — REQUIRED UNIT TESTS

Add to `crates/daemon/src/main.rs` (`mod tests`):

| # | Test | Verifies |
|---|---|---|
| CDP.1 | `searates_skill_output_parser_extracts_captcha_intercepted_field` | Mock skill output JSON with `captcha_intercepted=true` → parser sets the field on the typed struct |
| CDP.2 | `searates_skill_output_parser_extracts_cdp_state_enum` | Each cdp_state value (connected / unreachable / captcha_blocked / session_expired) deserializes correctly |
| CDP.3 | `retry_loop_attempts_3_times_when_captcha_intercepted` | Mock invoker returns captcha_intercepted=true for all 3 calls → daemon attempts exactly 3 times, then bails |
| CDP.4 | `retry_loop_succeeds_on_second_attempt_clears_captcha` | Mock invoker returns captcha=true once then captcha=false → final result is the second attempt's freight data |
| CDP.5 | `retry_loop_does_not_retry_when_cdp_state_unreachable` | Mock invoker returns cdp_state=unreachable → daemon does NOT retry; bails immediately |
| CDP.6 | `cached_freight_fallback_uses_recent_successful_row` | Synthetic DB with a prior successful row 3 days old → cached fallback applies, warning emitted |
| CDP.7 | `cached_freight_fallback_skips_rows_older_than_ttl` | Synthetic DB with a prior row 10 days old, TTL=7 → fallback does NOT apply; operator notification persisted |
| CDP.8 | `cached_freight_fallback_skips_rows_with_zero_freight` | Synthetic DB with prior rows having freight_base=0 → fallback does NOT apply; operator notification persisted |
| CDP.9 | `operator_notification_blockers_contain_both_required_codes` | When fallback doesn't apply, persisted blockers include both CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED and FREIGHT_NO_CACHED_FALLBACK |
| CDP.10 | `retry_per_attempt_timeout_honored` | A mock invoker that sleeps past the per-attempt timeout (60s default; can override via env in test setup) → daemon kills the attempt and counts it toward the retry budget |
| CDP.11 | `retry_loop_uses_exponential_backoff_intervals` | Inspect the time deltas between retry attempts (using a mockable clock) → ~15s after attempt 1, ~45s after attempt 2 (no backoff after attempt 3 — bail through to D3/D4) |
| CDP.12 | `existing_4_landed_cost_rows_unaffected_by_handoff_when_no_captcha` | After running through the new code path with CDP up + no captcha → DGU row matches the prior successful state (or improves), other 3 domestic rows unchanged |

**Test count expectation**: 12 new unit tests. Current binary unit count: 147. After: **159**. Plus all existing 147 must still pass.

---

## §5 — ACCEPTANCE CRITERIA — R.1 to R.13

| # | Criterion |
|---|---|
| R.1 | `searates-freight-estimate/result.schema.json` requires `captcha_intercepted: boolean` and `cdp_state: enum (connected/unreachable/captcha_blocked/session_expired)`; SKILL.md documents the new contract |
| R.2 | Daemon-side parser extracts both new fields into a typed struct; CDP.1 + CDP.2 pass |
| R.3 | Retry loop fires up to 3 times when captcha_intercepted=true; CDP.3 + CDP.4 pass |
| R.4 | Retry loop does NOT fire when cdp_state=unreachable; CDP.5 passes |
| R.5 | Per-attempt timeout = 60s default (env-overridable via `STAGE11_SKILL_RETRY_PER_ATTEMPT_SECONDS`); total retry budget (`attempts × per_attempt + sum(backoffs)`) ≤ 240s, leaving ≥60s headroom inside the outer 300s `LANDED_COST_SKILL_TIMEOUT_SECONDS`; CDP.10 passes |
| R.6 | Backoff intervals: 15s → 45s (2 backoffs across 3 attempts); CDP.11 passes |
| R.7 | Cached freight fallback uses the most recent `landed_cost_estimates` row for the same `(process_id, supplier_quote_id)` with `freight_base_gbp_pence > 0` and within 7 days (env-overridable via `STAGE11_FREIGHT_CACHE_TTL_DAYS`); CDP.6 + CDP.7 + CDP.8 pass |
| R.8 | When fallback applies, `landed_cost_estimates.warnings` contains structured warning `FREIGHT_FROM_CACHED_ESTIMATE` |
| R.9 | When fallback does not apply, `landed_cost_estimates.blockers` contains both `CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED` and `FREIGHT_NO_CACHED_FALLBACK`; decision_grade='blocked'; CDP.9 passes |
| R.10 | All 3 demo scenarios in §3 walk-through cleanly: A succeeds with fresh data, B retries-then-falls-back-to-cache with warning, C operator-notification path |
| R.11 | Stage 12/13/14 invariants intact: 17 net-margin assessments still review (or better — DGU may improve if Scenario A succeeds with real freight), 7 manifests still manifest_ready, pending_operator_intents from Stage 14 demo unchanged |
| R.12 | CDP.12 passes: existing 4 landed_cost_estimates rows are not regressed; only DGU's row may change (and only in the freight columns + warnings/blockers — other Stage 11 fields like supplier_cost, customs_duty, import_vat are unchanged unless freight changed economic_landed_cost downstream) |
| R.13 | Quality gates: `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p daemon --bin daemon` (159 binary tests), per-file integration loop green across 39 files |

---

## §6 — VALIDATION QUERIES

Run these after each demo scenario in §3:

```bash
sqlite3 -header -column state/state.db "
SELECT '=== R.10 / DGU lce post-run ===';
SELECT
  decision_grade,
  freight_low_gbp_pence AS f_low, freight_base_gbp_pence AS f_base, freight_high_gbp_pence AS f_high,
  blockers,
  warnings,
  created_at
FROM landed_cost_estimates
WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1'
ORDER BY created_at DESC LIMIT 3;

SELECT '=== R.8 / R.9 structured codes present ===';
SELECT
  SUM(CASE WHEN warnings LIKE '%FREIGHT_FROM_CACHED_ESTIMATE%' THEN 1 ELSE 0 END) AS rows_with_cached_warning,
  SUM(CASE WHEN blockers LIKE '%CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED%' THEN 1 ELSE 0 END) AS rows_with_operator_blocker,
  SUM(CASE WHEN blockers LIKE '%FREIGHT_NO_CACHED_FALLBACK%' THEN 1 ELSE 0 END) AS rows_with_no_cache_blocker
FROM landed_cost_estimates;

SELECT '=== R.11 invariants ===';
SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY verdict;
SELECT pipeline_state, COUNT(*) FROM pipeline_assessments GROUP BY pipeline_state;
SELECT stoplight, COUNT(*) FROM manifests GROUP BY stoplight;
SELECT idempotency_key, status FROM pending_operator_intents WHERE idempotency_key LIKE 'dec-2026-05-01-dgu-%';
"
```

---

## §7 — FAILURE-MODE PLAYBOOK

| Symptom | Cause | Fix |
|---|---|---|
| Skill output schema validation fails because `captcha_intercepted` is missing | Skill prompt not updated OR result.schema.json marks it required but old runs predate it | Update SKILL.md to instruct the Codex agent to always emit the field; if you created canary/golden files in step 3, update them too; for backwards-compat with already-recorded runs, consider making the field `default: false` in the schema rather than strictly required |
| Retry loop runs forever / hits the outer 300s timeout | Retry not honoring per-attempt timeout OR not counting attempts | Wrap each attempt in `tokio::time::timeout(Duration::from_secs(per_attempt_seconds), ...)`. Track an `attempt_count` counter; bail when it reaches `MAX_ATTEMPTS=3` |
| Cached-freight fallback returns rows from a different process | Lookup query missing `process_id` filter | The lookup must filter by BOTH `process_id` AND `supplier_quote_id` to ensure the cached freight is for the right route + buyer context |
| Cached fallback silently overwrites fresh freight | Order-of-operations bug — fallback applied even when retry succeeded | Apply fallback ONLY when retry budget is exhausted AND the final attempt's `captcha_intercepted=true` OR `cdp_state` is non-`connected` |
| Operator-notification blockers appear even when freight succeeded | Blocker emission not gated on retry-and-fallback exhaustion | Blockers fire only after the full chain (retry → cache → notify) completes without a usable freight number |
| Modified `infer_order_quantity_basis` or `OrderQuantityBasis` | Out of scope — that's the order-quantity hardening handoff | Stop. Revert. This handoff is `execute_landed_cost_skill_session` + SeaRates skill files only |
| Schema epoch mismatch errors | Spurious migration added | This handoff adds NO migrations. If `cargo test` complains about epoch, you've changed something you shouldn't have — revert |
| `STAGE11_SIMULATE_CAPTCHA=1` doesn't propagate to the skill | The skill prompt must honor the env var by reading it from the agent context OR the daemon must pass it as a `job_parameters` field | Recommended: when `STAGE11_SIMULATE_CAPTCHA=1` is set, the daemon adds `simulate_captcha: true` to the SeaRates skill's job_parameters; SKILL.md instructs the agent to honor it by emitting `captcha_intercepted=true` without driving the browser |

---

## §8 — CONSTRAINTS

1. **No new migrations.** This handoff is logic + skill-prompt only.
2. **No changes to `infer_order_quantity_basis` or `OrderQuantityBasis`** — that's the order-quantity hardening handoff.
3. **No changes to Stage 12, Stage 13, or Stage 14 code paths.**
4. **Stage 12 verdict for DGU may improve** (if Scenario A succeeds with real freight) but must not regress in the other 3 rows.
5. **Skill schema extension is BACKWARDS-INCOMPATIBLE** for the SeaRates skill — once `captcha_intercepted` is required in the result schema, every invocation must emit it. Coordinate the skill prompt + canary input + daemon parser changes in the same commit.
6. **Worktree** if running concurrently with the order-quantity handoff: `git worktree add ../testproject1-stage11-cdp main`. Merge to main with rebase at the end.
7. **No silent test exclusion.** Per AGENTS.md `### Stage Validation Test Gate`, the per-file integration loop is mandatory.

---

## §9 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/stage-11-hardening-cdp-resilience-validation-manifest.md`. Same template shape as the Stage 11 hardening manifest. Include the 3 demo scenarios' before/after blocker + warning + freight columns for the DGU row, plus the §6 query output.

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon          # 159 tests
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
```

---

## §10 — REMINDERS

- ULTRATHINK and be meticulous.
- The goal is **observable resilience**, not auto-solving captchas. The daemon detects, retries, falls back, and notifies the operator. The operator solves the captcha (out-of-band).
- The DGU row's existing blockers (`CDP_CONNECT_FAILED`, `SEARATES_NO_RESULT`) should be replaced with the new structured codes after this handoff lands. The new codes are operator-actionable.
- The §3 demo matrix is the primary success signal — three scenarios, three captured logs, three distinct outcomes.
- If you find yourself touching `infer_order_quantity_basis`, `OrderQuantityBasis`, the desktop UI, or anything outside Stage 11's skill runner — stop. Wrong handoff.
- Worktree for parallel safety. Rebase to main at the end.
