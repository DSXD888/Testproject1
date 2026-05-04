# Full Pipeline End-to-End Validation — L1-L3, Stages 1-14, All Built Surfaces

ULTRATHINK and be meticulous.

## MISSION

Drive a single procurement opportunity from **first source-side discovery** all the way to **operator decision capture**, exercising every stage and every code path the L1-L3 product has shipped. This is the canonical "does the whole thing actually work?" test. Every commit since `91a1484` (Stage 14 Phase 1 baseline) plus every prior validated stage must hold up under one continuous, real-data flow.

Concretely, walk the chain:

1. **Find contracts** — drive `--ingest procontract` (live CDP browser) AND `--ingest contracts-finder` (live OCDS API).
2. **Express interest** — drive `--ingest procontract-interest` for newly-discovered processes that pass pre-filter (REAL portal mutation; carefully scoped — see D4).
3. **Wait for the buyer's reply** — bookmark the run, optionally pause until the portal grants document access. If the test cannot wait, fall back to processes that registered interest in earlier sessions (April 21) and have presumably been granted by now.
4. **Get the information** — drive `--ingest procontract-enrich` and PDF extraction skills. Acquire and parse spec sheets / pricing schedules / BoQs.
5. **Find the quotes** — drive `--ingest <marketplace>` across `amazon-uk`, `gem-imports`, `mx-wholesale`, `ebay-uk`, and `alibaba-search` until at least one Stage 9 produces real GBP supplier quotes.
6. **Calculate logistics, customs, FX** — drive `--ingest landed-cost` (the four-skill chain: HMRC customs + shipment-sizing + SeaRates freight + simple-FX), with CDP browser running.
7. **Net margin gate + manifest + operator decision** — drive `--ingest net-gate && --ingest manifest`, then capture a decision via CLI AND via the Tauri desktop UI.
8. **Operate everything from the desktop UI** — launch `bunx tauri dev`, walk the Manifest Inbox + Process Detail flow live, validate cross-session staleness, capture screenshots.
9. **Operational subsystems** — verify cron scheduler enqueues jobs, source-health/validation/resource-arbiter snapshots return non-empty payloads, the workflow_jobs queue drains.

You will **drive → validate → diagnose → fix bugs in code → re-drive → continue down the chain** in a strict sequential loop. **The whole point is to surface real-world bugs that unit tests + isolated stage validations missed.** Bugs found here MUST be fixed in this handoff (small fixes inline; larger ones get a regression test + bead).

```
┌──────────────────────────────────────────────────────────────────────┐
│                       THE END-TO-END LOOP                             │
│                                                                       │
│   1. Pre-flight (§0): every prerequisite + baseline snapshot         │
│   2. For each Stage N in [1, 2, 3, 4a, 4-wait, 4b, 5, 6, 7, 8,       │
│      9, 10, 11, 12, 13, 14a-CLI, 14b-UI]:                            │
│      a. Drive the stage (§3 protocol)                                │
│      b. Validate against Stage N's expected output (§3 + §7 queries) │
│      c. If broken: diagnose root cause; fix code; add regression     │
│         test if non-trivial; re-drive Stage N from a clean state     │
│      d. Move to Stage N+1                                             │
│   3. Run the Desktop UI flow (§4) once Stage 13 is green             │
│   4. Run operational-subsystem checks (§5)                            │
│   5. Quality gates: full daemon binary suite + per-file integration  │
│      loop + Tauri Rust tests + vitest                                │
│   6. All criteria GREEN → write the validation manifest               │
└──────────────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first. This handoff is comprehensive (~800 lines) by design; treat each section as the spec for that part of the test.

---

## SCOPE

### IN scope

- **Layer 1**: daemon runner, PTY skill execution, IPC bridge, cron scheduler, source registry, resource arbiter, validation policy/executor (skeleton).
- **Layer 2**: Tauri desktop app, daemon supervision, IPC commands, ContractList/ContractDetail/ManifestInbox/OperatorDecisionPanel pages.
- **Layer 3**: pipeline Stages 1-14, multi-source ingestion (procontract + contracts-finder + fts fixture), Stage 11 hardening (order-quantity + CDP resilience), Stage 13 manifest skill + daemon fallback, Stage 14 operator decision (CLI + UI).

### OUT of scope (DO NOT attempt)

- **Layer 4** — Shared Visibility (web viewing portal + sync engine + push/pull to Supabase + cross-org coordination). NOT BUILT.
- **Layer 5** — Expanded Commercial Intelligence (decision-grade templates, advanced policies). NOT BUILT.
- **Layer 6** — Request Pipeline Automation. NOT BUILT.
- **Stage 15** (`bid_workflow`) — L6-blocked. The deferred handoff `prompts/stage-15-bid-workflow-handoff-DEFERRED.md` covers it; do not run.
- **Real-money or commitment actions**: this test must NOT submit a bid, sign a contract, or send a purchase order. The only authorized real-portal mutation is `procontract-register-interest` (which clicks a "register interest" button — see D4 for safety rules).
- **Web viewing portal**: `apps/web/` does not exist. The Tauri desktop is the only frontend.

---

## §0 — PRE-FLIGHT

| Claim | Verify |
|---|---|
| HEAD on `main` is at the latest commit (verify with `git log --oneline -1`); working tree clean (`git status --short` empty); synced 0/0 with `origin/main` | `git log --oneline -1; git status --short; git rev-list --left-right --count origin/main...main` |
| Daemon binary tests baseline: **180 / 180** passing on current main (was 169 before Phase 2 live + flake cleanup landed); per-file integration loop **39 / 39 PASS**; vitest 43 / 43; Tauri Rust tests query_service 25/25, daemon_bridge 14/14, native_shell 22/22 | `cargo test -p daemon --bin daemon 2>&1 \| grep "test result:"; cd apps/desktop && bunx vitest run; cd ../..; cargo test -p procintel-desktop` |
| Daemon binary builds: `target/debug/daemon` exists | `ls -la target/debug/daemon` |
| All 5 ProContract/CF/FTS source CLI invocations are wired in the dispatcher (verify with grep — exact lines drift as main.rs grows; current ranges: fts ~1273, contracts-finder ~1277, procontract ~1278) | `grep -n '"fts" =>\|"contracts-finder" =>\|"procontract" =>\|"manifest" =>\|"net-gate" =>\|"landed-cost" =>' crates/daemon/src/main.rs \| head -10` |
| Stage 11 dispatcher: `landed-cost` → `run_landed_cost_live_loop` | same grep — currently line ~1285 |
| Stage 12 dispatcher: `net-gate` → `run_net_margin_gate_live_loop` | same grep — currently line ~1286 |
| Stage 13 dispatcher: `manifest` → `run_manifest_generation_live_loop` | same grep — currently line ~1287 |
| Marketplace-search dispatcher: 5 IDs (`gem-imports`, `mx-wholesale`, `amazon-uk`, `ebay-uk`, `alibaba-search`) → `run_marketplace_search_live_loop` | `grep -n 'run_marketplace_search_live_loop' crates/daemon/src/main.rs \| head -3` |
| Stage 2/3/5/6/7/8 deterministic chain: `--pipeline <source>` → `run_deterministic_pipeline` (line 17790+; verify with grep) | `grep -n 'fn run_deterministic_pipeline' crates/daemon/src/main.rs` |
| Stage 14 CLI: `--capture-decision`, `--capture-clarification`, `--withdraw-clarification`, `--withdraw-decision`, `--list-decisions` (cli flags around line ~480 onward; verify with grep) | `grep -n 'capture_decision: Option\|capture_clarification: Option\|withdraw_clarification: Option\|withdraw_decision: Option' crates/daemon/src/main.rs \| head -6` |
| Tauri IPC commands include 7 Phase 2 commands wired into `invoke_handler` block (currently lines ~84-116; verify with grep): `list_manifests`, `get_decision_history`, `get_active_overrides`, `capture_clarification`, `withdraw_clarification`, `withdraw_decision`, `trigger_pipeline_rerun` (plus the existing 17 from before Phase 2) | `grep -n 'invoke_handler(tauri::generate_handler' apps/desktop/src-tauri/src/main.rs; sed -n '84,116p' apps/desktop/src-tauri/src/main.rs` |
| `bid_workflow_states` table EXISTS but unused (Stage 15 is L6-blocked) — do NOT exercise it | `sqlite3 state/state.db "SELECT COUNT(*) FROM bid_workflow_states"` (expected 0) |
| Validation pipeline (L4) skeleton: `crates/daemon/src/validation/{policy,executor}.rs` exist but no validator skills configured. The `validation_verdicts` table may be empty | `sqlite3 state/state.db "SELECT COUNT(*) FROM validation_verdicts"` |
| 24 skills inventoried — 5 procurement-ingestion, 4 PDF-extraction, 5 marketplace-search, 4 landed-cost, 1 manifest-generation, 2 source-health, 2 auth-refresh, 1 validation/base | `find skills -name SKILL.md \| wc -l` (expected 24) |
| Required CLIs on PATH: **claude** (most ingestion/UI skills), **codex** (HMRC + sizing + SeaRates), **agent-browser** (CDP-driven browser interactions), **docling** (PDF structural extraction; usually in `$HOME/.docling-venv`) | `which claude codex agent-browser; ls $HOME/.docling-venv/bin/docling 2>/dev/null` |
| **CDP BROWSER REQUIREMENT**: Chrome (or Chromium) running with `--remote-debugging-port=9222`. Many skills are gated on `requires_cdp: true` and will fail with `CDP_CONNECT_FAILED` without it. Operator must launch this BEFORE this handoff begins | `curl -sf http://localhost:9222/json/version \| head -1` (expected: JSON with browser version) |
| **PORTAL AUTHENTICATION**: the operator's CDP browser session must already be SIGNED IN to ProContract. The `procontract-discover` and `procontract-interest` skills assume an active session. If signed out, agent will fail with `AGENT_AUTH_CHALLENGE` | operator-confirmed |
| Live state.db baseline (read-only check; do NOT modify): 240 contract_notices (140 procontract + 100 contracts-finder), 239 procurement_processes, 286 notice_lots, 820 product_candidates, 7 manifests, 4 landed_cost_estimates, 17 review net-margin verdicts, 9 pending_operator_intents, 11 operator_input_overrides, source_cursors at procontract page 4 last_published 2026-05-01 | `sqlite3 state/state.db "<the count queries from earlier validations>"` |
| `agent-mail` healthy; commits need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Parallel-safety**: this handoff runs the full pipeline end-to-end and writes to state.db extensively. **DO NOT run this concurrently with any other handoff.** It will conflict with the post-merge gate sweep, the CDP live demos, the Stage 14 Phase 2 live validation, and the CF Stages 8-14 walkthrough. If those are still in-flight, wait. If running in parallel-safe mode is required, use a worktree + isolated state root (D1 covers this).

---

## §1 — DECISIONS ALREADY MADE

### D1 — State isolation strategy: **fresh state root, preserve live demo state**

This handoff exercises real ingestion and must NOT clobber the Stage 11/12/13/14 demo state in `state/state.db` (DGU's basis_undetermined fix, the 2 demo intents, the 250-m² operator override — all of which validation manifests reference). Use a new isolated runtime:

```bash
export E2E_STATE_ROOT="state-e2e-$(date -u +%Y%m%dT%H%M%SZ)"
export E2E_RUNTIME_DIR="runtime-e2e-$(date -u +%Y%m%dT%H%M%SZ)"
export E2E_ARTIFACT_ROOT="artifacts-e2e-$(date -u +%Y%m%dT%H%M%SZ)"

mkdir "$E2E_STATE_ROOT" "$E2E_RUNTIME_DIR" "$E2E_ARTIFACT_ROOT"

# All daemon invocations in this handoff use these flags:
target/debug/daemon \
  --state-root "$E2E_STATE_ROOT" \
  --runtime-dir "$E2E_RUNTIME_DIR" \
  --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest <whatever>
```

This means EVERY daemon invocation in §3 needs the three `--state-root / --runtime-dir / --artifact-root` flags. Failing to pass them will modify the live `state/state.db`. Be careful.

When done (after Stage 14 captures a decision), the `$E2E_STATE_ROOT/state.db` becomes the test artifact; the live `state/state.db` is untouched.

### D2 — Target process selection

This handoff intentionally drives the chain on a **freshly discovered** ProContract opportunity, not a process from the live DB. This stresses every stage with new data, not just deterministic re-walks. After the discover step, identify ONE specific process to follow end-to-end (the "spotlight process"). Other processes ingested in the same batch will get partial coverage incidentally, but the spotlight is what the validation manifest tracks.

Spotlight selection criteria (apply after Stage 1 completes):

```sql
-- Pick the freshest non-construction goods notice with reasonable value + a UK buyer
-- (construction notices typically need site visits + bid bonds and aren't a clean
--  L1-L3 demo; goods are easier to walk through marketplace search + landed cost)
SELECT cn.id AS notice_id, p.id AS process_id, cn.title, cn.published_at, cn.deadline_at,
       cn.estimated_value_gbp_ex_vat_pence, cn.main_category
FROM contract_notices cn
JOIN procurement_processes p ON p.primary_notice_id = cn.id
WHERE cn.source_id='procontract'
  AND cn.published_at >= datetime('now','-7 days')
  AND lower(cn.main_category) NOT LIKE '%construction%'
  AND lower(cn.main_category) NOT LIKE '%works%'
  AND cn.estimated_value_gbp_ex_vat_pence BETWEEN 500000 AND 100000000   -- £5k to £1M
ORDER BY cn.published_at DESC
LIMIT 10;
```

Pick the candidate that looks most "physical-goods-like" (e.g., furniture, ICT, lab equipment, catering supplies — anything with a real product specification). Document the pick + reasoning in the validation manifest. If no fresh good fits, fall back to one already-discovered process from the live state (cite that you fell back; the test still proves the chain).

### D3 — The wait period

The ProContract chain is asynchronous: register-interest → wait for buyer-side approval → enrich-tender (downloads docs once granted). Approval can take seconds, hours, or days depending on the buyer.

Three handling strategies, in order of preference:

1. **Pick an "Open" notice with auto-grant**. Many ProContract opportunities grant document access immediately upon registering interest (no manual approval). The skill output's `registrations[]` array tells you whether docs are immediately available. If the spotlight notice is auto-grant: proceed straight to enrich after register.

2. **Use a previously-registered process from earlier sessions**. The current `state/state.db` shows 24 completed `interest_registration` actions from April 21. ANY of those processes that haven't been enriched yet are excellent candidates — interest was registered 13+ days ago, so buyer approval has had ample time. Query:

   ```sql
   SELECT cn.id, cn.title, cn.deadline_at
   FROM contract_notices cn
   WHERE cn.source_id='procontract'
     AND EXISTS (SELECT 1 FROM process_stage_status pss
                 WHERE pss.process_id = cn.process_id
                   AND pss.stage = 'interest_registration'
                   AND pss.state = 'succeeded')
     AND NOT EXISTS (SELECT 1 FROM process_stage_status pss
                     WHERE pss.process_id = cn.process_id
                       AND pss.stage = 'tender_enrichment'
                       AND pss.state = 'completed')
     AND cn.deadline_at >= datetime('now')
   ORDER BY cn.published_at DESC LIMIT 10;
   ```

   These processes registered interest but were never enriched. Drive `--ingest procontract-enrich` against them to walk Stage 4b. Use one of these as the spotlight if no fresh discover happens to grant immediately.

3. **Real-time wait**. If neither (1) nor (2) is available, the handoff includes a documented pause: register interest → record the timestamp + idempotency key → SLEEP for an operator-acceptable interval (say, 4 hours during business hours). Then resume with enrich-tender. If running automated, the agent should checkpoint and document "resuming N hours later" in the manifest.

The handoff agent should pick strategy (1) if possible, fall back to (2) if not, and only resort to (3) if both fail.

### D4 — REAL EXTERNAL MUTATION SAFETY (`procontract-register-interest`)

This skill clicks the "Register interest in this opportunity" button on a real procurement portal. That action is reviewable but not commitment-creating. Per `skills/procurement-ingestion/procontract-register-interest/SKILL.md`:

- `side_effect_class: external_mutation`
- The operator has authorized this action (per project memory: "Stage 15 is request pipeline automation, not real-money commitment" — register-interest is a request-pipeline action and is authorized).

Safety rules for THIS handoff:

- Limit to **at most 3 register-interest actions** per spotlight + supplementary candidates. Do not mass-register.
- If the daemon's pre-filter (the gate before the skill) rejects a notice, do NOT override the rejection — that's the skill respecting category whitelists / value bands.
- The notice must be picked per D2 criteria (goods-like, reasonable value, fresh).
- Capture the registered notice IDs in the validation manifest — these are the buyer-visible mutations this handoff produced.
- DO NOT register interest on construction or services notices unless the spotlight choice deliberately requires it (and even then, document why).

The skill does NOT send messages, place bids, or commit to anything. It clicks "express interest" — equivalent to a prospective bidder asking for the document pack. This is the safe scope.

### D5 — Parallel paths (multi-source breadth)

While the spotlight ProContract process walks the full chain, also drive contracts-finder and FTS in parallel time, even though they aren't the spotlight:

- **Contracts Finder**: drive `--ingest contracts-finder` to pull fresh OCDS notices since 2026-05-02. Walk Stages 1-7 (deterministic) on the new CF batch via `--pipeline contracts-finder`. Stages 8-14 won't auto-progress for CF (per bead `MS-ENRICH-1`); document the gap.

- **FTS**: drive `--ingest fts --fixture-path <real-data-fixture>` if a fixture is available. If not, document FTS as deferred (per bead `MS-FTS-LIVE-1`).

This lets the validation manifest claim breadth ("3 sources exercised") without requiring all 3 to walk all 14 stages.

### D6 — Bug fix scope: in or out

When a stage breaks during E2E driving, the fix scope is bounded:

| Symptom | Action |
|---|---|
| Trivial bug (typo, missing field default, wrong column name): fix in <30 min | Fix in place + regression test + commit |
| Medium bug (logic gap, missing error path, schema mismatch): fix in <2 hours | Fix in place + regression test + commit; document in validation manifest |
| Significant bug (state machine flaw, architectural gap, requires new skill): fix would take >2 hours OR require L4-L6 features | DO NOT fix here. File as bead. Document the gap in the validation manifest. The chain MAY still be walked past this point IF possible (e.g., manually populate the missing data), or the test stops at the broken stage and the manifest documents how far it got. |

The handoff value comes from BOTH "proved end-to-end works" AND "found N bugs along the way and fixed them". Both are useful outcomes.

### D7 — Marketplace search safety

All 5 marketplace-search skills are `side_effect_class: none`. They drive a CDP browser to search Amazon/eBay/Alibaba/etc. and read prices. They do NOT add to cart, place orders, or sign in to anything stateful (the Alibaba skill DOES require an authenticated session if specific access is needed, but most use anonymous browsing).

Safe to drive without further confirmation. Just be aware:

- Amazon UK has been observed to render EUR prices in CF walkthrough — the skill correctly flags `LOCALE_OR_CURRENCY_DRIFT`. Expected behavior, not a bug.
- Alibaba may serve captchas. If the skill returns `SOURCE_CAPTCHA_BLOCKED`, retry once; if persistent, skip that marketplace for the spotlight and try another.

### D8 — UI walkthrough timing

The desktop UI walkthrough (§4) runs AFTER Stage 13 has produced a manifest for the spotlight process. Don't try to walk the UI mid-pipeline; the inbox shows manifests, and there is none until Stage 13 completes.

---

## §2 — EXECUTION ORDER (high-level sequence)

1. Pre-flight (§0) — verify ALL prerequisites, including CDP browser running.
2. Initialize isolated state root (D1).
3. **Stage 1 — Discover** (`--ingest procontract` + `--ingest contracts-finder`).
4. Pick **spotlight process** (D2 query).
5. **Stage 2-3 — Normalize + Relate** (auto-runs as part of `--pipeline procontract`).
6. **Stage 4a — Register interest** (`--ingest procontract-interest`) on the spotlight (D4 safety).
7. **Stage 4-wait** — handle the wait per D3.
8. **Stage 4b — Enrich tender** (`--ingest procontract-enrich`).
9. **Stages 5, 6, 7 — Extract lots, qualify, reclassify** (already complete via `--pipeline procontract`; verify).
10. **Stage 8 — Extract products** (PDF semantic extraction; manually drive if not auto-triggered).
11. **Stage 9 — Marketplace search** (`--ingest amazon-uk`, etc.; iterate marketplaces until at least 1 GBP quote exists).
12. **Stage 10-11 — Gross margin gate + Landed cost** (`--ingest landed-cost`; CDP browser required).
13. **Stage 12 — Net margin gate** (`--ingest net-gate`).
14. **Stage 13 — Manifest** (`--ingest manifest`).
15. **Stage 14a — Capture decision via CLI** (`--capture-decision`).
16. **§4 Desktop UI flow** — launch tauri dev, walk operator workflow, capture screenshots.
17. **Stage 14b — Capture decision via UI** (cross-validate that UI does the same as CLI).
18. **§5 Operational subsystem checks**.
19. Quality gates.
20. Validation manifest.

If any stage fails, follow the loop in the diagram at top: diagnose, fix, re-drive, continue.

---

## §3 — STAGE-BY-STAGE PROTOCOL

For each stage, the following template applies:

- **Trigger**: the exact command(s) to run.
- **Expected outcome**: what success looks like.
- **Validation queries**: SQL or filesystem checks.
- **Failure modes**: common breakages + diagnosis.

### Stage 1 — Discover (Ingest)

**Trigger** (run all three; first two in parallel windows; FTS only if fixture exists):

```bash
target/debug/daemon \
  --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest procontract 2>&1 | tee /tmp/e2e_stage1_procontract.log

target/debug/daemon \
  --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest contracts-finder 2>&1 | tee /tmp/e2e_stage1_cf.log

# FTS only if a fixture exists (search for one):
find . -name '*.json' -path '*fts*fixture*' 2>/dev/null | head -3
# If found:
# target/debug/daemon --state-root ... --ingest fts --fixture-path <path> 2>&1 | tee /tmp/e2e_stage1_fts.log
```

**Expected outcome**: ProContract live loop returns ≥10 new notices (depending on cursor + day's activity). CF returns ≥10 OCDS notices. Each writes via `persist_ingestion_from_agent_output` into `contract_notices`.

**Validation queries**:

```sql
-- Run against $E2E_STATE_ROOT/state.db (NOT live state.db)
SELECT source_id, COUNT(*) FROM contract_notices GROUP BY source_id;
-- Expect: procontract>=10, contracts-finder>=10, (fts if fixture run)

-- Verify Stage 1 stage_status rows
SELECT stage, scope_kind, state, COUNT(*) FROM process_stage_status GROUP BY 1,2,3;
-- Expect: 'Normalize' | 'notice' | 'succeeded' | matches notice count
```

**Failure modes**:

| Symptom | Diagnosis |
|---|---|
| `AGENT_AUTH_CHALLENGE` from procontract | CDP browser is signed out of ProContract. Operator must sign in. |
| `CDP_CONNECT_FAILED` | CDP browser not running on :9222. Launch Chrome with `--remote-debugging-port=9222 --user-data-dir=$HOME/.chrome-automation`. |
| `LOCALE_OR_CURRENCY_DRIFT` on CF | CF returned non-GBP value bands. The skill should still persist the records; investigate the skill prompt if persistence is blocked. |
| Skill timeout | Daemon kills skills after `--state-root`-derived timeout (Stage 1 inherits the default ~10 min for live discovery). If it stalls, inspect `agent_session_logs.failure_reason`. |

### Stage 2 — Normalize

**Trigger**: auto-runs as part of `persist_ingestion_from_agent_output` (synchronous after Stage 1 skill returns).

**Expected outcome**: each new `contract_notice` produces a `procurement_process` row (1:1 unless multiple notices share an OCID — then 1 process per OCID). `process_stage_status` rows for stage='Normalize' scope='notice' state='succeeded'.

**Validation queries**:

```sql
SELECT 'notices', COUNT(*) FROM contract_notices;
SELECT 'processes', COUNT(*) FROM procurement_processes;
SELECT 'normalize succeeded', COUNT(*) FROM process_stage_status WHERE stage='Normalize' AND state='succeeded';
-- Expect: processes count >= notices count - intra-source duplicates (typically 0 or 1)
```

### Stage 3 — Relate

**Trigger**: auto-runs as part of `--pipeline procontract` and `--pipeline contracts-finder`. Run those now:

```bash
target/debug/daemon --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --pipeline procontract 2>&1 | tee /tmp/e2e_stage2-7_procontract.log
target/debug/daemon --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --pipeline contracts-finder 2>&1 | tee /tmp/e2e_stage2-7_cf.log
```

**Expected outcome**: deterministic chain emits `extract-lots`, `prequalify-notice`, `qualify-lot`, `reclassify-candidates`, `score-quoteability`, `gross-margin-gate`, `relate` log lines. `process_relations` count grows IFF cross-source canonical_ocid matches exist (per the multi-source manifest's bead `MS-DEDUP-1`, the procontract `pc-<uuid>` and CF `ocds-b5fd17-<uuid>` namespaces are disjoint, so this is typically 0).

**Validation queries**:

```sql
SELECT relation_type, COUNT(*) FROM process_relations GROUP BY relation_type;
SELECT stage, COUNT(*) FROM process_stage_status WHERE stage IN ('relate','extract lots','prequalify notice','qualify lot') AND state='succeeded' GROUP BY 1;
```

### Pick the SPOTLIGHT process NOW

Use the D2 query against `$E2E_STATE_ROOT/state.db`. Document the choice + reasoning in the validation manifest. Save to env var:

```bash
export SPOTLIGHT_PID="proc-pc-<uuid>"
echo "Spotlight: $SPOTLIGHT_PID" | tee /tmp/e2e_spotlight.txt
```

### Stage 4a — Register interest (REAL portal mutation)

**Trigger**:

```bash
# Re-confirm safety: D4 limits this to ≤3 actions
# The skill applies its own pre-filter; we just trigger the source loop and
# the daemon decides which notices qualify.
target/debug/daemon --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest procontract-interest 2>&1 | tee /tmp/e2e_stage4a_interest.log
```

**Expected outcome**: skill emits `registrations[]` array. For each notice the daemon pre-filter approved, the agent clicks "Register interest" and reports either `registered`, `already_registered`, or a failure code. `interest_registration` stage_status row created.

**Validation queries**:

```sql
SELECT 'interest_registration succeeded', COUNT(*) FROM process_stage_status WHERE stage='interest_registration' AND state IN ('succeeded','completed','skipped');

-- Did the spotlight register?
SELECT pss.state, asl.failure_reason
FROM process_stage_status pss
LEFT JOIN agent_session_logs asl ON asl.session_id IN (
  SELECT session_id FROM agent_session_jobs WHERE job_parameters_json LIKE '%' || ? || '%'
)
WHERE pss.process_id = '$SPOTLIGHT_PID' AND pss.stage='interest_registration';
```

**Failure modes**:

| Symptom | Action |
|---|---|
| `AGENT_AUTH_CHALLENGE` | Sign in to ProContract in the CDP browser. Re-run. |
| Pre-filter rejected the spotlight | Pick a different spotlight that passes. The pre-filter is the safety boundary; don't override it. |
| `SOURCE_SOFT_BLOCKED` | ProContract may be rate-limiting. Wait 10 min and retry. |
| Skill registered interest but `state='skipped'` for spotlight | The agent reported success but the state isn't reflecting it. Inspect `agent_session_logs` for the session — likely a parser bug. |

### Stage 4-wait — Wait for buyer-side approval

Per D3:

- If the skill's `registrations[]` for the spotlight reports `documents_immediately_available: true` (or equivalent — verify the skill's result.schema.json), proceed immediately to Stage 4b.
- Else if a previously-registered process from earlier April 21 sessions can serve as the spotlight, switch to that one (document the swap).
- Else if real-time wait is required: record the registration timestamp, write a checkpoint markdown to `$E2E_ARTIFACT_ROOT/wait-checkpoint.md` capturing state, and resume after the operator deems wait sufficient (typically 4-24 hours).

The validation manifest documents the wait strategy used.

### Stage 4b — Enrich tender (download docs)

**Trigger**:

```bash
target/debug/daemon --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest procontract-enrich 2>&1 | tee /tmp/e2e_stage4b_enrich.log
```

**Expected outcome**: skill emits `enriched_tenders[]`. For each enriched process, downloads PDFs/spec sheets to `$E2E_ARTIFACT_ROOT/downloads/procontract/<process_id>/`. `tender_enrichment` stage_status row created.

**Validation queries**:

```sql
SELECT process_id, state FROM process_stage_status WHERE stage='tender_enrichment' AND process_id='$SPOTLIGHT_PID';
SELECT COUNT(*) AS doc_count FROM raw_documents WHERE process_id='$SPOTLIGHT_PID';
```

Filesystem:

```bash
ls $E2E_ARTIFACT_ROOT/downloads/procontract/$SPOTLIGHT_PID/ | head -10
```

**Failure modes**: same as Stage 4a + new ones for download (file write errors, document URL changes, etc.). If enrich completes but `raw_documents` count is 0, the parser probably didn't capture the file paths. Inspect `agent_session_logs` for the enrich session.

### Stage 5 — Extract lots

Already auto-runs in `--pipeline procontract`. After Stage 4b enriches the spotlight, re-run pipeline to pick up new lots:

```bash
target/debug/daemon --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --pipeline procontract 2>&1 | tee /tmp/e2e_stage5-7_procontract.log
```

**Validation queries**:

```sql
SELECT id, lot_key, title, estimated_value_pence FROM notice_lots
WHERE notice_id IN (SELECT id FROM contract_notices WHERE process_id='$SPOTLIGHT_PID');
```

Expect ≥1 lot row (often "implicit" if the notice doesn't break out lots).

### Stage 6 — Qualify lot

**Validation queries**:

```sql
SELECT id, lot_id, qualification_state, qualification_reasons_json
FROM lot_qualifications WHERE lot_id IN (
  SELECT id FROM notice_lots WHERE notice_id IN
    (SELECT id FROM contract_notices WHERE process_id='$SPOTLIGHT_PID')
);
```

Expect ≥1 row with `qualification_state` of `goods` or `services` or `works`. The handoff cares about goods primarily.

### Stage 7 — Reclassify candidates

This stage backfills `head_noun` and `candidate_type` for any candidates that earlier extraction got wrong. Auto-runs in `--pipeline procontract`. Validation: no specific query — just confirm it appears in the pipeline log without errors.

### Stage 8 — Extract products (PDF semantic extraction)

This stage does NOT auto-trigger from Stage 4b enrichment in the current code — it requires manual invocation OR a job-queue trigger. Drive it manually:

```bash
# Identify spotlight's downloaded PDFs
ls $E2E_ARTIFACT_ROOT/downloads/procontract/$SPOTLIGHT_PID/*.pdf

# For each PDF, queue a structural extraction job:
target/debug/daemon --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --skill-path skills/pdf-extraction/tender-specification --job-parameters-json '{
  "process_id": "'$SPOTLIGHT_PID'",
  "document_paths": ["<absolute path to PDF>"]
}' 2>&1 | tee /tmp/e2e_stage8_pdf.log

# OR: drive --run-next-job to process queued PDF jobs
target/debug/daemon --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --run-next-job 2>&1 | tee /tmp/e2e_stage8_runner.log
```

**Validation queries**:

```sql
SELECT id, name, candidate_kind, identity_confidence_bps, quoteability_score_bps
FROM product_candidates WHERE process_id='$SPOTLIGHT_PID';
SELECT COUNT(*) FROM document_extractions WHERE process_id='$SPOTLIGHT_PID';
SELECT COUNT(*) FROM document_semantic_extractions WHERE process_id='$SPOTLIGHT_PID';
```

Expect ≥1 product candidate emitted per qualifying document.

**Failure modes**: PDF extraction is the most fragile stage. Common issues: docling can't parse encrypted PDFs (skip), semantic extraction returns no candidates (PDF is mostly boilerplate, not spec content), wrong candidate_kind (re-run Stage 7 reclassify).

### Stage 9 — Marketplace search

**Trigger**: drive each marketplace until at least 1 GBP quote exists for the spotlight:

```bash
for marketplace in amazon-uk gem-imports mx-wholesale ebay-uk alibaba-search; do
  echo "=== Driving $marketplace ==="
  target/debug/daemon --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest "$marketplace" 2>&1 | tee /tmp/e2e_stage9_$marketplace.log

  # Check for GBP quotes after each
  count=$(sqlite3 $E2E_STATE_ROOT/state.db "SELECT COUNT(*) FROM supplier_quotes WHERE product_candidate_id IN (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID') AND currency='GBP'")
  echo "GBP quotes for spotlight after $marketplace: $count"
  if [ "$count" -gt 0 ]; then
    echo "Got GBP quote, stopping marketplace iteration"
    break
  fi
done
```

**Expected outcome**: at least 1 GBP supplier quote exists for at least 1 spotlight candidate after iterating marketplaces. If NONE produce GBP quotes (all Amazon EUR-rendered, Alibaba captcha-blocked, etc.), document the failure and proceed to Stage 11 with screening-only state (Stage 11 will produce `0` lce rows; that's the expected zero-quote path per the CF walkthrough manifest).

**Validation queries**:

```sql
SELECT id, source_label, listing_url, unit_price_gbp_pence, currency, listing_pack_qty
FROM supplier_quotes WHERE product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID');

SELECT COUNT(*), AVG(unit_price_gbp_pence) AS avg_pence FROM supplier_quotes
WHERE product_candidate_id IN (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID')
  AND currency='GBP';
```

### Stage 10 — Gross margin gate

Auto-runs as part of `--ingest landed-cost`. Don't drive separately. Validation happens after Stage 11 below.

### Stage 11 — Landed cost

**Trigger**:

```bash
target/debug/daemon --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest landed-cost 2>&1 | tee /tmp/e2e_stage11_landed.log
```

**Expected outcome**: for each (candidate, supplier_quote) pair with gross margin gate=pursue/review, the chain runs HMRC customs → shipment-sizing → SeaRates freight → simple-FX → persists `landed_cost_estimates`. Domestic candidates skip freight/customs.

**Validation queries**:

```sql
SELECT lce.id, lce.decision_grade, lce.economic_landed_cost_gbp_pence,
       lce.commercial_estimate_confidence_bps,
       cpa.gross_margin_bps,
       json_extract(lce.confidence_breakdown, '$.order_quantity_basis.kind') AS basis_kind,
       lce.blockers, lce.warnings
FROM landed_cost_estimates lce
JOIN candidate_pricing_assessments cpa ON cpa.product_candidate_id=lce.product_candidate_id AND cpa.selected_quote_id=lce.supplier_quote_id
WHERE lce.product_candidate_id IN (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID');
```

**Failure modes**: the Stage 11 hardening manifests have already documented every CDP-related failure mode + retry/cache-fallback path. If `CDP_CONNECT_FAILED` appears, restart the CDP browser. If `SEARATES_NO_RESULT`, that means the Codex agent didn't drive SeaRates correctly — inspect the session log.

### Stage 12 — Net margin gate

**Trigger**:

```bash
target/debug/daemon --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest net-gate 2>&1 | tee /tmp/e2e_stage12_netgate.log
```

**Expected outcome**: per-candidate `candidate_net_margin_assessments` rows + per-process rollups. `verdict` is one of `pursue`, `review`, `pass`, or `reject`.

**Validation queries**:

```sql
SELECT product_candidate_id, verdict, reasons_json, basis_hash
FROM candidate_net_margin_assessments WHERE product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID');
```

### Stage 13 — Manifest generation

**Trigger**:

```bash
target/debug/daemon --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest manifest 2>&1 | tee /tmp/e2e_stage13_manifest.log
```

**Expected outcome**: `manifests` row for the spotlight with `stoplight` ∈ {pursue, clarify, pass, review} and a rendered manifest artifact at `$E2E_ARTIFACT_ROOT/manifests/$SPOTLIGHT_PID/manifest.md`.

**Validation queries**:

```sql
SELECT process_id, stoplight, suggested_action, manifest_basis_hash, artifact_hash
FROM manifests WHERE process_id='$SPOTLIGHT_PID';
```

```bash
# Inspect the rendered manifest
cat $E2E_ARTIFACT_ROOT/manifests/$SPOTLIGHT_PID/manifest.md | head -50
```

The manifest should be human-readable, include sections for the lot/candidate/quote/landed-cost evidence, and surface clarification prompts if any.

### Stage 14a — Capture decision via CLI

**Trigger**:

```bash
target/debug/daemon --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --capture-decision $SPOTLIGHT_PID \
  --disposition watch \
  --idempotency-key e2e-cli-$(date -u +%Y%m%dT%H%M%SZ) \
  --reasoning "End-to-end validation handoff: walked $SPOTLIGHT_PID from discovery to decision through every L1-L3 stage." 2>&1 | tee /tmp/e2e_stage14a_cli.log
```

**Validation queries**:

```sql
SELECT idempotency_key, status, desired_disposition, decision_note, queued_at
FROM pending_operator_intents WHERE process_id='$SPOTLIGHT_PID';
```

Expect 1 row with `status='queued'`.

---

## §4 — DESKTOP UI VALIDATION

After Stage 13 produces the spotlight's manifest, exercise the desktop UI. The UI should be pointed at `$E2E_STATE_ROOT/state.db` (NOT live state.db). Two paths:

**Path A — Run UI against E2E state**: requires environment variable or config override pointing the desktop bootstrap at the E2E state root. Check `apps/desktop/src-tauri/src/main.rs` `DesktopPaths::from_handle` + how it resolves `state_db_path()`. If it's not configurable, use Path B.

**Path B — Snapshot E2E state into the desktop's expected location** for the UI walkthrough (then restore live state after):

```bash
# Snapshot live state, swap in E2E state for UI validation
cp state/state.db state/state.db.live-snapshot-pre-ui
cp $E2E_STATE_ROOT/state.db state/state.db
```

(If using Path B, REMEMBER to restore live state at the end of UI validation: `cp state/state.db.live-snapshot-pre-ui state/state.db`.)

### UI flow

```bash
cd apps/desktop && bunx tauri dev
```

Walk:

1. Manifest Inbox loads (default route `/inbox`). Verify spotlight process card visible with stoplight + suggested action.
2. Click spotlight card → Process Detail View opens. Markdown renders via `ManifestViewer`.
3. Decision history empty (no prior decisions captured via UI yet).
4. Open Clarification form (if any prompts). Submit a test clarification (NOT for real; pick a non-load-bearing reason_code).
5. Click "Re-run pipeline". Progress modal cycles. Manifest hash refreshes.
6. Capture a decision (`approve`, `reject`, or `watch`) via Quick-Decide panel + reasoning textarea.

### Stage 14b — Capture decision via UI

The UI walkthrough's Quick-Decide IS Stage 14b. Verify in DB:

```sql
SELECT idempotency_key, status, desired_disposition, decision_note, queued_at
FROM pending_operator_intents WHERE process_id='$SPOTLIGHT_PID' ORDER BY queued_at DESC;
```

Expect ≥2 rows now (one from CLI Stage 14a, one from UI Stage 14b).

### Cross-session staleness E2E (bonus)

While desktop is open with spotlight loaded:

1. **External terminal**: `target/debug/daemon --state-root state --ingest manifest` (re-render manifests; if anything shifted, basis_hash bumps).
2. Reload UI Inbox. Stale banner should appear on spotlight card.
3. Open spotlight detail. Banner above Quick-Decide panel.

### Screenshots

Capture screenshots at every UI step (spotlight inbox, detail view, clarification form, decision capture, stale banner). Commit to `docs/pipeline/full-pipeline-e2e-screenshots/`.

### Cleanup

If using Path B, restore live state.db now: `cp state/state.db.live-snapshot-pre-ui state/state.db`.

---

## §5 — OPERATIONAL SUBSYSTEM CHECKS

Quick verifications that the L1 operational substrate works:

| Subsystem | Trigger | Expected |
|---|---|---|
| Cron scheduler | Inspect `cron_definitions` table OR daemon logs for cron tick lines | At minimum, source-health crons should be defined for each registered source |
| Source health | Trigger via daemon logs OR check `source_health_states` table | Each source has a recent health-check entry; circuit-breaker state visible |
| Resource arbiter | Inspect daemon logs during high-concurrency multi-marketplace runs | Browser slot limits + agent slot limits respected; no panic |
| Validation pipeline (skeleton) | `validation_verdicts` table | EXPECTED EMPTY today (L4 not built; no validator skills configured). Document as such — this is the L4 gap |
| Workflow jobs queue | `workflow_jobs` table | After Stage 8 PDF queueing, expect rows transitioning pending → claimed → completed |
| IPC bridge | Tauri desktop's bootstrap snapshot | Returns `daemon_state=Ready` and `instance_id` matches the daemon's |

```sql
-- Quick subsystem snapshot
SELECT 'cron_defs', COUNT(*) FROM cron_definitions;
SELECT 'workflow_jobs by status', status, COUNT(*) FROM workflow_jobs GROUP BY status;
SELECT 'validation_verdicts', COUNT(*) FROM validation_verdicts;
SELECT 'agent_session_logs total', COUNT(*) FROM agent_session_logs;
```

---

## §6 — ACCEPTANCE CRITERIA — R.1 to R.30

| # | Criterion |
|---|---|
| R.1 | Pre-flight (§0) all green — CDP browser running, all CLIs available, isolated state root created |
| R.2 | Stage 1: `--ingest procontract` returned ≥10 new notices into $E2E_STATE_ROOT/state.db |
| R.3 | Stage 1: `--ingest contracts-finder` returned ≥10 new notices |
| R.4 | Stage 1: FTS path either ran with a fixture OR documented as deferred (per bead `MS-FTS-LIVE-1`) |
| R.5 | Stage 2-3: deterministic chain via `--pipeline procontract` + `--pipeline contracts-finder` ran cleanly; `process_relations` count grew IFF cross-source OCID matches existed |
| R.6 | Spotlight process selected per D2 query; document choice + reasoning |
| R.7 | Stage 4a: `--ingest procontract-interest` ran with safety bounds (≤3 actions); spotlight registered (or pre-filter rejected with reason documented) |
| R.8 | Stage 4-wait: handled per D3 strategy (1, 2, or 3); strategy + outcome documented |
| R.9 | Stage 4b: `--ingest procontract-enrich` downloaded docs to `$E2E_ARTIFACT_ROOT/downloads/procontract/$SPOTLIGHT_PID/`; ≥1 PDF present |
| R.10 | Stage 5-7: `--pipeline procontract` re-ran after enrichment; spotlight has ≥1 lot + qualification |
| R.11 | Stage 8: PDF extraction (structural + semantic) produced ≥1 product_candidate for spotlight |
| R.12 | Stage 9: at least one marketplace produced ≥1 GBP supplier_quote for spotlight, OR all 5 documented why no quote (e.g., locale_or_currency_drift, captcha) |
| R.13 | Stage 10-11: `--ingest landed-cost` produced ≥1 lce row for spotlight (or documented why 0 — e.g., zero quotes from Stage 9) |
| R.14 | Stage 12: `--ingest net-gate` produced ≥1 net-margin assessment for spotlight |
| R.15 | Stage 13: `--ingest manifest` produced 1 manifest row for spotlight; rendered `manifest.md` artifact exists |
| R.16 | Stage 14a (CLI): captured decision; `pending_operator_intents` row has the test idempotency key + status='queued' |
| R.17 | §4 Desktop UI: `bunx tauri dev` launched cleanly; spotlight visible in Manifest Inbox |
| R.18 | §4 UI: clicked spotlight → Process Detail View renders manifest |
| R.19 | §4 UI: re-run pipeline button cycles modal + refreshes manifest hash |
| R.20 | §4 UI: capture decision via Quick-Decide panel produces a second row in `pending_operator_intents` |
| R.21 | §4 UI: cross-session staleness E2E: external `--ingest manifest` → stale banner appears in UI within 2s of reload |
| R.22 | §4 Screenshots committed to `docs/pipeline/full-pipeline-e2e-screenshots/` |
| R.23 | §5: cron_definitions has ≥N rows (whatever the registered-source baseline is); workflow_jobs queue transitioned at least one job through pending → completed during this run |
| R.24 | §5: validation_verdicts is empty (L4 gap documented, not a bug) |
| R.25 | Live state.db NOT MODIFIED — pre/post hash match (use `sha256sum state/state.db` before + after; if Path B used in §4, before is snapshot from after-restore) |
| R.26 | Bugs surfaced + fixed: every bug fix in scope per D6 has a regression test + commit; bugs out-of-scope filed as beads with reproduction steps |
| R.27 | Daemon binary tests still 180 / 180 (or 180+N if regression tests added); per-file integration loop still 39 / 39; vitest still 43 / 43; Tauri Rust tests unchanged |
| R.28 | Quality gates: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings` clean |
| R.29 | Validation manifest written to `docs/pipeline/full-pipeline-e2e-validation-manifest.md` covering every stage's outcome + bugs + screenshots |
| R.30 | Final commit + push to origin if any code changed |

---

## §7 — VALIDATION QUERY PACK

Save this script to `/tmp/e2e_validate.sh` and source it after each stage:

```bash
#!/usr/bin/env bash
DB="${1:-$E2E_STATE_ROOT/state.db}"
SPOTLIGHT="${2:-$SPOTLIGHT_PID}"

echo "=== Stage 1-3 (ingest/normalize/relate) ==="
sqlite3 -header -column "$DB" "
SELECT source_id, COUNT(*) AS notices FROM contract_notices GROUP BY source_id;
SELECT 'processes total', COUNT(*) FROM procurement_processes;
SELECT 'process_relations', COUNT(*) FROM process_relations;
"

echo "=== Stage 4 (interest/enrich) for spotlight ==="
sqlite3 -header "$DB" "
SELECT stage, state, COUNT(*) FROM process_stage_status WHERE process_id='$SPOTLIGHT' GROUP BY 1,2;
"

echo "=== Stages 5-7 (lots/qualify/reclassify) ==="
sqlite3 -header "$DB" "
SELECT 'lots', COUNT(*) FROM notice_lots WHERE notice_id IN (SELECT id FROM contract_notices WHERE process_id='$SPOTLIGHT');
SELECT 'lot_qualifications', COUNT(*) FROM lot_qualifications WHERE lot_id IN (
  SELECT id FROM notice_lots WHERE notice_id IN (SELECT id FROM contract_notices WHERE process_id='$SPOTLIGHT'));
"

echo "=== Stage 8 (product_candidates) ==="
sqlite3 -header "$DB" "
SELECT id, name, candidate_kind, identity_confidence_bps, quoteability_score_bps
FROM product_candidates WHERE process_id='$SPOTLIGHT' LIMIT 10;
"

echo "=== Stage 9 (supplier_quotes) ==="
sqlite3 -header "$DB" "
SELECT id, source_label, currency, unit_price_gbp_pence, listing_pack_qty
FROM supplier_quotes WHERE product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT') LIMIT 10;
"

echo "=== Stage 10-11 (gross + landed cost) ==="
sqlite3 -header "$DB" "
SELECT lce.id, lce.decision_grade, lce.economic_landed_cost_gbp_pence,
       lce.commercial_estimate_confidence_bps, cpa.gross_margin_bps,
       json_extract(lce.confidence_breakdown, '\$.order_quantity_basis.kind') AS basis_kind
FROM landed_cost_estimates lce
JOIN candidate_pricing_assessments cpa ON cpa.product_candidate_id=lce.product_candidate_id AND cpa.selected_quote_id=lce.supplier_quote_id
WHERE lce.product_candidate_id IN (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT');
"

echo "=== Stage 12 (net margin) ==="
sqlite3 -header "$DB" "
SELECT product_candidate_id, verdict, reasons_json
FROM candidate_net_margin_assessments WHERE product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT');
"

echo "=== Stage 13 (manifest) ==="
sqlite3 -header "$DB" "
SELECT process_id, stoplight, suggested_action, manifest_basis_hash, artifact_hash
FROM manifests WHERE process_id='$SPOTLIGHT';
"

echo "=== Stage 14 (decisions + overrides) ==="
sqlite3 -header "$DB" "
SELECT idempotency_key, status, desired_disposition, decision_note, queued_at
FROM pending_operator_intents WHERE process_id='$SPOTLIGHT' ORDER BY queued_at;
SELECT product_candidate_id, reason_code, payload_json, withdrawn_at
FROM operator_input_overrides WHERE product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT') ORDER BY provided_at;
"

echo "=== Stage tracking summary for spotlight ==="
sqlite3 -header -column "$DB" "
SELECT stage, scope_kind, state, updated_at
FROM process_stage_status WHERE process_id='$SPOTLIGHT' ORDER BY updated_at;
"
```

---

## §8 — FAILURE-MODE PLAYBOOK (broad)

| Symptom (any stage) | Diagnosis path |
|---|---|
| Skill timeout | Inspect `agent_session_logs.failure_reason` + `artifacts/sessions/...` for the session's transcript. If timeout was real, the per-stage env var (e.g., `LANDED_COST_SKILL_TIMEOUT_SECONDS=300`) may need adjusting OR the skill prompt is genuinely stuck. |
| Skill returned but data not persisted | `persist_*_from_agent_output` parser bug. Inspect the daemon logs for parse warnings. The result.schema.json may have drifted. |
| Pipeline stage skipped silently | Check `process_stage_status` for `state='skipped'` rows + the stage's gating logic (typically prerequisite stages must succeed first). |
| `CDP_CONNECT_FAILED` recurs | Restart Chrome with `--remote-debugging-port=9222 --user-data-dir=$HOME/.chrome-automation`. Verify `curl http://localhost:9222/json/version`. If still failing, the persistent profile may be corrupted; create a fresh `--user-data-dir`. |
| `AGENT_AUTH_CHALLENGE` | Sign back into the relevant portal in the CDP browser. Persistent re-auth probably means the auth-refresh skill didn't run; manually refresh. |
| `LOCALE_OR_CURRENCY_DRIFT` | Marketplace rendered non-GBP prices. Skill correctly rejected. Try a different marketplace or document the gap. |
| Daemon panics / segfaults | This is a real bug. Capture stack trace, file as bead, attempt narrow fix. Don't deferred — daemon stability is critical. |
| State.db corruption | Run `sqlite3 $E2E_STATE_ROOT/state.db "PRAGMA integrity_check"`. If broken, the test must restart from a fresh state root. |
| Tauri UI doesn't launch | Inspect Tauri dev logs (vite + cargo). Often a port conflict (5173 in use) or daemon-supervision failure. |
| Tauri UI launches but Inbox empty | The UI is reading the wrong state.db. Verify Path A or Path B per §4. |
| Per-file integration loop newly fails one of the 39 files | Regression introduced by an in-handoff fix. Identify via `git diff` what changed; bisect if needed. |
| `cargo clippy` newly fails | A fix introduced lint debt. Address before commit. |

---

## §9 — CONSTRAINTS

1. **Live `state/state.db` MUST NOT be modified** by stage execution. Use isolated state root (D1). If §4 UI walkthrough requires Path B (snapshot swap), restore the live snapshot at the end and verify with sha256sum (R.25).
2. **Real portal mutations limited to ≤3 register-interest actions** per D4. Document each one (process_id + buyer name + notice title) in the validation manifest.
3. **No real-money / no real-bid actions.** Stage 15 is L6-blocked; the bid_workflow_states table exists but must remain empty after this handoff.
4. **CDP browser launch is operator-driven.** Agent must NOT auto-launch Chrome unless explicitly authorized. If CDP isn't running at start, prompt the operator.
5. **Stage 14 demo state in live DB MUST remain intact** (R.25 verifies via hash; if you accidentally hit live state, restore from a backup before declaring done).
6. **Bug fixes per D6 scope** — small/medium fixes inline with regression tests; large fixes filed as beads.
7. **Quality gates per R.27/R.28 mandatory** — no silent test skipping.
8. **Don't re-run successful stages** unless the validation reveals an issue. The chain is designed to flow forward; reverse moves are last resort.
9. **The wait period (Stage 4) is acceptable** — if the operator/agent cannot wait, swap spotlight to a previously-registered process per D3 strategy 2.

---

## §10 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/full-pipeline-e2e-validation-manifest.md`. Template:

```markdown
# Full Pipeline End-to-End Validation Manifest

> Generated: <ISO timestamp>
> Status: GREEN / RED / YELLOW (with caveats)
> HEAD: <commit>
> E2E state root: <path>
> Spotlight process: <id> ("<title>")

## Summary

<2-3 paragraphs: what was the spotlight, did the chain complete end-to-end,
how many bugs surfaced + fixed, what stages were green vs yellow vs red,
final operator decision captured>

## Setup

| Item | Value |
|---|---|
| Branch | main |
| HEAD at start | <commit> |
| HEAD at end | <commit> (if changed) |
| Isolated state root | $E2E_STATE_ROOT |
| CDP browser | <command + version> |
| Operator portal sign-in | confirmed/yes/n/a |
| Wait strategy used | D3.1 / D3.2 / D3.3 |

## Spotlight selection (D2)

| Field | Value |
|---|---|
| process_id | proc-pc-... |
| Notice title | ... |
| Buyer | ... |
| Source | procontract |
| Published | ... |
| Deadline | ... |
| Estimated value | £... |
| Category | goods/services/works |
| Reason chosen | ... |

## Stage-by-stage outcomes

| Stage | Result | Evidence |
|---|---|---|
| 1 — Discover (procontract) | PASS / FAIL | N notices ingested; log /tmp/... |
| 1 — Discover (CF) | PASS / FAIL | N notices ingested |
| 1 — Discover (FTS) | PASS / DEFERRED | Reason ... |
| 2 — Normalize | PASS | N processes |
| 3 — Relate | PASS | N relations (typically 0) |
| 4a — Register interest | PASS / SKIPPED | N notices registered, M skipped |
| 4-wait | Strategy D3.X used | ... |
| 4b — Enrich | PASS / FAIL | N PDFs downloaded |
| 5-7 — Lots/qualify/reclassify | PASS | ... |
| 8 — Product extraction | PASS / FAIL | N candidates emitted |
| 9 — Marketplace search | PASS / SCREENING-ONLY | N quotes from M marketplaces |
| 10-11 — Landed cost | PASS / FAIL | N lce rows |
| 12 — Net margin | PASS | verdict=... |
| 13 — Manifest | PASS | stoplight=... |
| 14a — CLI decision | PASS | idempotency_key=... |
| 14b — UI decision | PASS | idempotency_key=... |

## Real portal mutations (per D4)

| Notice ID | Buyer | Title | Action | Result |
|---|---|---|---|---|
| ... | ... | ... | register_interest | registered/already_registered |

## Bugs surfaced + fixed

| File | Bug | Fix | Regression test |

## Bugs surfaced + filed (out of scope per D6)

| Bead ID | Title | Reproduction | Why deferred |

## Desktop UI walkthrough

| Step | Screenshot | Result |
| 1 — Inbox loads | step-01-... | PASS |
| ... |

## Operational subsystem checks

| Subsystem | Status | Evidence |

## Live state.db integrity check (R.25)

| Check | Result |
| sha256 before | ... |
| sha256 after | ... |
| Match | YES / NO |

## R.1–R.30 status

| # | Criterion | Status | Evidence |

## Quality Gates

| Gate | Result |

## Stages NOT exercised (out-of-scope reminders)

- Stage 15 (bid_workflow): L6-blocked
- Web viewing portal: L4 not built
- Sync to Supabase: L4 not built
- Validator skills: L4 not built
- Cross-org coordination: L4 not built
```

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon              # 180 / 180 (or +N if regressions added)
cargo test -p procintel-desktop --test query_service     # 25 / 25
cargo test -p procintel-desktop --test daemon_bridge     # 14 / 14
cargo test -p procintel-desktop --test native_shell      # 22 / 22
cd apps/desktop && bunx vitest run               # 43 / 43
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done   # 39 / 39
```

---

## §11 — REMINDERS

- ULTRATHINK and be meticulous.
- This is the **canonical end-to-end test of the L1-L3 product**. The point is not to demo a polished flow — it's to find every break the unit tests + isolated stage validations missed.
- **Bugs surfaced are EXPECTED**. The Stage 14 Phase 2 live validation found 8. This handoff likely will find more. Fix what you can; file what you can't.
- The wait period (Stage 4) is not a test failure — it's a real-world async dependency. Pick a strategy per D3 and document.
- The CDP browser is the operator's responsibility. Pre-flight prompts; agent does not auto-launch unless explicitly authorized.
- Real portal mutations are limited (D4). Three is the cap; document each.
- Live state.db is sacred. Use isolated state root (D1). Verify integrity with sha256sum at the end (R.25).
- The manifest is the deliverable. Without it, the test results can't be reviewed. Take the time to write it well.
- Stage 15, web portal, sync, validator skills — all out of scope. They live in L4-L6 and are correctly deferred.
- After this handoff lands GREEN, the L1-L3 product can credibly claim "validated end-to-end on real procurement data, with operator able to discover → register → enrich → quote → cost → decide via both CLI and desktop UI." That's the threshold for stakeholder demo + L4 build start.
