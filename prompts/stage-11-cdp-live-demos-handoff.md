# Stage 11 CDP Resilience — Live Browser Demos (Scenarios A & B)

ULTRATHINK and be meticulous.

## MISSION

The Stage 11 CDP resilience handoff (`4ba771d`) shipped CODE-COMPLETE: SeaRates skill emits `captcha_intercepted` + `cdp_state` fields, daemon-side retry loop with 60s/15s/45s budget, cached freight fallback within 7-day TTL, operator-notification blockers. Twelve CDP.* unit tests pass + Scenario C (CDP down) ran end-to-end during the original validation.

But Scenarios A (CDP up, no captcha → real freight value persisted) and B (CDP up, simulated captcha → retries → fallback to cached row → `FREIGHT_FROM_CACHED_ESTIMATE` warning) were DEFERRED because no CDP browser was available in the dev environment. This handoff drives both scenarios with a real CDP browser running on `localhost:9222`. The DGU process is the target candidate — its lce row currently has `freight_base_gbp_pence=0` and CDP_CONNECT_FAILED + SEARATES_NO_RESULT blockers. After Scenario A, freight_base should be > 0; after Scenario B, freight should come from cache + warning surfaced.

You will **launch CDP → drive scenario → validate → diagnose any failure → fix or file → repeat** until R.1–R.10 are GREEN.

```
┌────────────────────────────────────────────────────────────┐
│                          THE LOOP                            │
│   1. Pre-flight (§0): start CDP browser                     │
│   2. Scenario A: --ingest landed-cost (no sim)             │
│   3. Validate freight + DGU lce row                         │
│   4. Scenario B: --ingest landed-cost --job-parameters-json │
│      '{"simulate_captcha":true}' (cache from A is target)  │
│   5. Validate retry log + cache hit + warning               │
│   6. Re-confirm Scenario C (CDP down) still works           │
│   7. Quality gates                                           │
│   8. Validation manifest                                     │
└────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## §0 — PRE-FLIGHT

| Claim | Verify |
|---|---|
| HEAD = `4ba771d`; CDP code is committed (search for `STAGE11_SKILL_RETRY_PER_ATTEMPT_SECONDS_DEFAULT` and `SeaRatesCdpState` in main.rs — should hit) | `git log --oneline -1; grep -c "STAGE11_SKILL_RETRY\|SeaRatesCdpState\|lookup_cached_freight\|run_searates_with_retries" crates/daemon/src/main.rs` |
| 12 new CDP.* unit tests pass | `cargo test -p daemon --bin daemon CDP_ 2>&1 \| grep "test result:"` |
| SeaRates skill files modified: `result.schema.json` requires `captcha_intercepted` + `cdp_state`; `SKILL.md` documents these fields + the simulate_captcha test hook | `head -10 skills/landed-cost/searates-freight-estimate/result.schema.json; grep -A2 "captcha_intercepted\|simulate_captcha" skills/landed-cost/searates-freight-estimate/SKILL.md \| head -10` |
| DGU baseline: `freight_base_gbp_pence=0`, blockers contain `CDP_CONNECT_FAILED + SEARATES_NO_RESULT + CUSTOMS_SKILL_NO_RESULT` | `sqlite3 state/state.db "SELECT decision_grade, freight_base_gbp_pence, blockers FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1'"` |
| `agent-browser` available on PATH (used by daemon to drive Codex skills against CDP) | `which agent-browser; agent-browser --version` |
| Codex CLI available (the SeaRates skill's `agent_cli`) | `which codex; codex --version` |
| Operator action required: launch a Chromium-based browser with `--remote-debugging-port=9222`. E.g.: `/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222 --user-data-dir=/tmp/cdp-stage11` | `curl -s http://localhost:9222/json/version` should return browser metadata JSON |
| `agent-mail` healthy; commits need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Operator pre-action**: this handoff CANNOT proceed without a running CDP browser at `localhost:9222`. The agent should pause + prompt the operator before driving Scenario A if `curl http://localhost:9222/json/version` fails.

**Parallel-safety**: this handoff drives the daemon + CDP browser. Do NOT run alongside the Stage 14 Phase 2 live validation handoff (which needs the daemon for IPC) or the CF Stages 8-14 walkthrough (which also drives `--ingest landed-cost`). Safe to run alongside docs hygiene + post-merge gate sweep + test flake cleanup.

---

## §1 — DECISIONS ALREADY MADE

### D1 — Scenario order: A → B → C → A again (sanity)

Run in this sequence. The cache target for Scenario B comes from Scenario A's success, so A must run first. Scenario C (CDP down) was validated during the original CDP handoff but re-running confirms no environmental drift.

### D2 — Out-of-band CDP launch is operator-driven

The daemon does NOT launch the CDP browser. The operator must launch it with `--remote-debugging-port=9222`. If this is automated for the agent (e.g., via `mcp__computer-use__open_application`), document the exact invocation in the validation manifest's "Setup" section.

### D3 — Snapshot DB before each scenario, restore between

To validate Scenario B's cache-hit path, you need Scenario A's success row to exist as a cache target. Before Scenario A: snapshot `state/state.db` to `state/state.db.preA`. Run A. Snapshot again as `.preB`. Run B (with simulate_captcha). Validate. If cache hit fails, restore `.preB` and try with adjusted state (e.g., delete the post-A lce row to force re-fetch, but keep the success row in history).

### D4 — Real freight numbers may vary

SeaRates is a real-time service. The freight value Scenario A produces depends on the date, route, container size — it WILL vary. The validation criterion is "value > 0 + reasonable" (say, between £100 and £100,000 for a UK→Asia container), not a specific number.

### D5 — Codex agent_cli — confirm subscription works

The SeaRates skill is `agent_cli: codex`. The Codex CLI must have a working subscription (per MEMORY: "Codex used for code-writing was OpenAI subscription"). Verify with a no-op codex invocation first if uncertain.

---

## §2 — EXECUTION ORDER

### Pre-launch checklist

1. `curl -s http://localhost:9222/json/version` returns 200 + JSON. If not, prompt operator to launch CDP browser.
2. `cargo build -p daemon --bin daemon` completes (the binary is what `--ingest` runs).
3. Snapshot: `cp state/state.db state/state.db.pre-cdp-demos`.

### Scenario A — CDP up, no captcha (happy path)

```bash
# Reset DGU's current lce row (force re-computation), but PRESERVE older rows in case of cache reuse
sqlite3 state/state.db "
DELETE FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1';
DELETE FROM process_stage_heads WHERE stage='landed cost' AND scope_id LIKE '%glass-dgu%';
DELETE FROM process_stage_status WHERE stage='landed cost' AND scope_id LIKE '%glass-dgu%';
"

# Drive ingestion (NO simulate_captcha)
cargo run -p daemon -- --ingest landed-cost 2>&1 | tee /tmp/cdp_demo_a.log

# Validate
sqlite3 -header state/state.db "
SELECT decision_grade, freight_low_gbp_pence, freight_base_gbp_pence, freight_high_gbp_pence, blockers, warnings, created_at
FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1'
ORDER BY created_at DESC LIMIT 1;
"
```

**Expected after Scenario A**:
- `decision_grade='review'` (not `blocked`) — freight succeeded so the row is no longer blocked-by-CDP
- `freight_base_gbp_pence > 0` — real number from SeaRates
- `blockers` no longer contain `CDP_CONNECT_FAILED` or `SEARATES_NO_RESULT`
- `warnings` does NOT contain `FREIGHT_FROM_CACHED_ESTIMATE` (we got fresh data, not cache)
- Snapshot for use in Scenario B: `cp state/state.db state/state.db.preB`

### Scenario B — CDP up, captcha simulated (retry → cache hit)

```bash
# Verify cache target exists from Scenario A
sqlite3 state/state.db "SELECT created_at, freight_base_gbp_pence FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1' AND freight_base_gbp_pence > 0 ORDER BY created_at DESC LIMIT 1"
# Should return Scenario A's row.

# Reset CURRENT row but PRESERVE history (the prior successful row)
sqlite3 state/state.db "
DELETE FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1' AND created_at > date('now','-5 minutes');
DELETE FROM process_stage_heads WHERE stage='landed cost' AND scope_id LIKE '%glass-dgu%';
DELETE FROM process_stage_status WHERE stage='landed cost' AND scope_id LIKE '%glass-dgu%';
"

# Drive with simulate_captcha
cargo run -p daemon -- --ingest landed-cost --job-parameters-json '{"simulate_captcha":true}' 2>&1 | tee /tmp/cdp_demo_b.log

# Inspect log for retry messages
grep -E "retry|attempt|captcha|backoff|cached" /tmp/cdp_demo_b.log

# Validate row
sqlite3 -header state/state.db "
SELECT decision_grade, freight_base_gbp_pence, blockers, warnings
FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1'
ORDER BY created_at DESC LIMIT 1;
"
```

**Expected after Scenario B**:
- Log shows 3 retry attempts (each with `captcha_intercepted=true`)
- After 3rd attempt: log shows cache lookup + cache hit (or cache miss + operator-notification blockers)
- `freight_base_gbp_pence > 0` — copied from Scenario A's row
- `warnings` contains structured warning `FREIGHT_FROM_CACHED_ESTIMATE`
- `blockers` does NOT contain `CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED`

### Scenario C — CDP down (sanity re-confirm)

```bash
# Stop CDP (operator action: close the browser OR firewall :9222)

# Reset DGU + clear cache (delete all prior successful rows so cache cannot apply)
sqlite3 state/state.db "
DELETE FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1';
DELETE FROM process_stage_heads WHERE stage='landed cost' AND scope_id LIKE '%glass-dgu%';
DELETE FROM process_stage_status WHERE stage='landed cost' AND scope_id LIKE '%glass-dgu%';
"

cargo run -p daemon -- --ingest landed-cost 2>&1 | tee /tmp/cdp_demo_c.log

# Validate
sqlite3 -header state/state.db "
SELECT decision_grade, freight_base_gbp_pence, blockers
FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1'
ORDER BY created_at DESC LIMIT 1;
"
```

**Expected after Scenario C**:
- Log shows `cdp_state=unreachable` — NO retries (`SeaRatesCdpState::skip_retries` returns true)
- `decision_grade='blocked'`
- `blockers` contains BOTH `CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED` AND `FREIGHT_NO_CACHED_FALLBACK`

### Restore + final state

```bash
# After all 3 scenarios validated: optionally restore pre-demo state
# cp state/state.db.pre-cdp-demos state/state.db
# (or keep the new state — Scenario A's freight number is real production data and useful for the golden journey)
```

---

## §3 — REQUIRED TESTS

This handoff adds NO new unit tests — the 12 existing CDP.* tests cover the logic. If a live demo surfaces a bug NOT caught by unit tests, fix + add a regression test that would have caught it.

---

## §4 — ACCEPTANCE CRITERIA — R.1 to R.10

| # | Criterion |
|---|---|
| R.1 | CDP browser launched + reachable at `localhost:9222` (operator-confirmed) |
| R.2 | Scenario A succeeds: DGU's lce row gains `freight_base_gbp_pence > 0`, blockers cleared of CDP/SeaRates codes, `decision_grade='review'` (not `blocked`) |
| R.3 | Scenario A log shows 1 attempt only (no retries needed) and `captcha_intercepted=false` in skill output |
| R.4 | Scenario B succeeds: log shows 3 retry attempts (each with captcha_intercepted=true), final row has `freight_base > 0` from cache, `warnings` contains `FREIGHT_FROM_CACHED_ESTIMATE` |
| R.5 | Scenario B's retry intervals roughly match the 15s + 45s backoff schedule (logged timestamps) |
| R.6 | Scenario C succeeds: log shows `cdp_state=unreachable`, NO retries, blockers include both `CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED` and `FREIGHT_NO_CACHED_FALLBACK` |
| R.7 | Other 3 lce rows (Aggregates + 2 carpets) UNCHANGED across all scenarios (this handoff only touches DGU) |
| R.8 | Stage 12/13/14 invariants intact: 17 net-margin verdicts, 7 manifests, demo intents (`-001` superseded, `-002` withdrawn), DGU operator override (qty=250 OR new value if Phase 2 live demo modified) |
| R.9 | If Scenario A's freight value caused Stage 12 verdict to change for DGU (e.g., review → pursue), document this — it's an interesting positive outcome |
| R.10 | Quality gates: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings`; 169 daemon tests pass; per-file integration loop unchanged |

---

## §5 — VALIDATION QUERIES + LOG INSPECTION

```bash
# After each scenario:
sqlite3 -header -column state/state.db "
SELECT '=== DGU lce post-scenario ===';
SELECT decision_grade, freight_low_gbp_pence AS f_low, freight_base_gbp_pence AS f_base, freight_high_gbp_pence AS f_high,
       blockers, warnings, created_at
FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1' ORDER BY created_at DESC LIMIT 3;

SELECT '=== Other lce rows unchanged ===';
SELECT pc.name, lce.decision_grade, lce.freight_base_gbp_pence
FROM landed_cost_estimates lce JOIN product_candidates pc ON pc.id=lce.product_candidate_id
WHERE lce.supplier_quote_id != 'sq-b22fcf00b7a545d699c3438c42b9027c-1' ORDER BY pc.name;

SELECT '=== Structured codes count ===';
SELECT
  SUM(CASE WHEN warnings LIKE '%FREIGHT_FROM_CACHED_ESTIMATE%' THEN 1 ELSE 0 END) AS cached_warning_rows,
  SUM(CASE WHEN blockers LIKE '%CDP_CAPTCHA_BLOCKED_OPERATOR_ACTION_REQUIRED%' THEN 1 ELSE 0 END) AS operator_blocker_rows,
  SUM(CASE WHEN blockers LIKE '%FREIGHT_NO_CACHED_FALLBACK%' THEN 1 ELSE 0 END) AS no_cache_blocker_rows
FROM landed_cost_estimates;

SELECT '=== Stage 12/13/14 invariants ===';
SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY verdict;
SELECT stoplight, COUNT(*) FROM manifests GROUP BY stoplight;
SELECT idempotency_key, status FROM pending_operator_intents WHERE idempotency_key LIKE 'dec-2026-05-01-dgu-%';
"

# Log inspection per scenario:
grep -E "captcha_intercepted|cdp_state|retry|attempt|backoff|cache.*hit|cache.*miss|FREIGHT_FROM_CACHED" /tmp/cdp_demo_a.log /tmp/cdp_demo_b.log /tmp/cdp_demo_c.log
```

---

## §6 — FAILURE-MODE PLAYBOOK

| Symptom | Cause | Fix |
|---|---|---|
| `curl http://localhost:9222/json/version` fails | Browser not running with `--remote-debugging-port=9222` | Operator action: launch Chrome with the flag. Document the exact command in the validation manifest |
| Scenario A finishes but `freight_base=0` | Codex agent didn't drive the SeaRates page successfully (timeout, navigation failed, blocked by anti-bot) | Inspect the agent_session_logs row for `landed-cost/searates-freight-estimate`. If transcript shows the agent navigated but couldn't extract freight, that's a SeaRates UI change — the skill prompt may need updating. Out of scope for this handoff but file as bead |
| Scenario A finishes but `cdp_state=connected` is missing in skill output | Skill prompt doesn't enforce the new field per CDP handoff D1 | Verify the SKILL.md has the "Mandatory CDP State + Captcha Reporting" section. If missing, this would be a CDP commit regression — investigate |
| Scenario B doesn't retry | The classify_searates_attempt logic isn't seeing captcha_intercepted=true | Inspect `/tmp/cdp_demo_b.log` for the parsed result struct. If `captcha_intercepted=false`, the skill didn't honor `simulate_captcha`. Check SKILL.md "Captcha Simulation (Daemon-driven Test Hook)" section + the daemon's `STAGE11_SIMULATE_CAPTCHA` env var → job_parameters propagation |
| Scenario B retries but cache lookup returns nothing | Either (a) the prior successful row was deleted, (b) `lookup_cached_freight` query has wrong filter, (c) TTL too narrow | Check that Scenario A's row still exists with `freight_base > 0`. Verify the lookup query filters by `(process_id, supplier_quote_id)` AND `freight_base > 0` AND within `STAGE11_FREIGHT_CACHE_TTL_DAYS`. If query is right, ensure created_at is within the TTL window (default 7 days) |
| Scenario B's retry intervals are wrong | Backoff env var misread or hardcoded | Verify `STAGE11_SKILL_RETRY_BACKOFFS_SECONDS` (default "15,45") parsing. Inspect the timestamps between log entries for retry attempts |
| Scenario C produces `decision_grade='review'` instead of `blocked` | Operator-notification blocker logic isn't gated on retry-and-fallback exhaustion | Trace the live loop integration in `crates/daemon/src/main.rs` — find where `lookup_cached_freight` returns None + retries exhausted → both blockers must be appended + decision_grade set to blocked |
| Other 3 lce rows changed during demo | Domestic rows shouldn't touch SeaRates path; if they regressed, something in the loop is over-running | Inspect log for non-DGU candidates. Domestic candidates skip freight entirely (per CDP handoff D5). Investigate |

---

## §7 — CONSTRAINTS

1. **No new migrations.**
2. **No changes to `infer_order_quantity_basis`, `OrderQuantityBasis`, or any Stage 14 code paths.**
3. **No changes to non-DGU lce rows.** This handoff exclusively exercises DGU's freight path.
4. **CDP browser launch is operator-driven** — agent must NOT auto-launch unless explicitly authorized.
5. **Snapshot state.db before starting** to enable rollback if cache demo gets confused.
6. **Real money / real bookings**: SeaRates is a price-lookup service, NOT a booking. This demo MUST NOT execute any booking action (the skill is read-only — verified by `side_effect_class: none` in SKILL.md).
7. **No silent test exclusion** — quality gates per R.10 mandatory.

---

## §8 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/stage-11-cdp-live-demos-validation-manifest.md`. Include:

- Setup section: exact CDP browser launch command, agent_cli version (codex), agent-browser version
- Per-scenario block: command run, log excerpt (retry attempts, cache hits), DB state before/after, screenshot of `chrome://inspect/#devices` showing CDP browser if helpful
- R.1–R.10 status table
- Findings (e.g., real freight value for DGU; whether Stage 12 verdict shifted)
- Quality gate transcript

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon          # 169 / 169
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || break; done   # baseline (38/39 + pre-existing fault_process_cleanup flake)
```

---

## §9 — REMINDERS

- ULTRATHINK and be meticulous.
- This handoff CANNOT proceed without operator action (launch CDP browser). Pause and prompt if needed.
- Scenarios A → B → C in order. Scenario A's success is Scenario B's cache target.
- Real freight values vary — validate "reasonable" not "specific".
- The CDP code (`4ba771d`) was validated via 12 unit tests + Scenario C; this handoff is the LIVE confirmation that it works against a real browser. If a unit-tested code path fails live, that's a high-value finding (probably an integration bug between skill prompt + daemon parser + cache lookup that the unit tests didn't cover).
- DGU may improve from `blocked` → `review` after Scenario A's success. That's a positive outcome — record in golden journey if both handoffs are running together.
- Cleanup: optionally restore state.db post-demo, OR keep Scenario A's real freight (useful for stakeholder demos showing "live data in the system").
