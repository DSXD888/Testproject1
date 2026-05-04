# Full Pipeline End-to-End RE-VALIDATION — Confirm Fixes Hold + Commercial-Basis State Machine

ULTRATHINK and be meticulous.

## MISSION

The previous full-pipeline E2E handoff (`prompts/full-pipeline-end-to-end-validation-handoff.md`, committed at `4c31791`) was driven on 2026-05-04. It surfaced a stack of real-world bugs under continuous live data that have since been fixed by other agents, and the daemon was robustified with a commercial-basis state machine that closes a class of dangerous landed-cost inferences (the famous "42.39 boilers" case where Stage 11 inferred fractional units from whole-lot value divided by a unit quote).

This handoff **re-drives the entire pipeline end-to-end** to confirm:

1. **Every previously-surfaced bug stays fixed** under live load — none recur.
2. **The new commercial-basis state machine works correctly** when driven against fresh real data — Stage 8 persists explicit product quantities, Stage 9 attempts BOTH UK-side and China-import quote roles, Stage 10 labels its commercial basis honestly (`two_sided_supplier_comparison` / `single_side_with_attempted_alternative` / `missing_required_quote_role_attempt` / fallback variants), Stage 11 refuses unsafe whole-lot quantity derivation when Stage 10 used allocated revenue, Stage 12-13 surface missing imports + quantity ambiguity in operator-readable reasons.
3. **The schema-epoch operational footgun is resolved or routed around** — the previous run hit a desktop/daemon schema mismatch (desktop expected 7, daemon migrated to 8) that required manual `PRAGMA user_version` recovery. This must NOT recur.
4. **The chain still flows end-to-end** from cold-start ingestion through operator decision capture (CLI + UI) on real procurement data.
5. **Any NEW bug surfaced** is fixed inline (small/medium per D6) or filed as a bead (large) — same validate/test/fix loop the previous run pioneered.

You will **drive → validate → diagnose → fix in code → re-drive → continue down the chain** until R.1–R.40 are GREEN.

```
┌────────────────────────────────────────────────────────────────────────┐
│                  THE END-TO-END RE-VALIDATION LOOP                       │
│                                                                         │
│   1. Pre-flight (§0): full prerequisites + SCHEMA EPOCH CHECK           │
│   2. For each Stage in [1, 2, 3, 4a, 4-wait, 4b, 5, 6, 7, 8, 9         │
│      (marketplace-coverage), 10 (gross-margin), 11, 12, 13, 14a, 14b]: │
│      a. Drive (per §3 protocol)                                        │
│      b. Validate functional output AGAINST §3 expected shape           │
│      c. Validate FIX-VERIFICATION criteria for that stage (§3 callouts)│
│      d. Validate COMMERCIAL-BASIS criteria for stages 8-13 (§3 + §6)   │
│      e. If broken: diagnose root cause; fix code; add regression test; │
│         re-drive Stage from clean state                                │
│      f. Move to Stage N+1                                               │
│   3. Run Desktop UI flow (§4) once Stage 13 is green                   │
│   4. Run operational-subsystem checks (§5)                              │
│   5. Quality gates (R.27 expected: 202 daemon binary tests)            │
│   6. All criteria GREEN → write the validation manifest                 │
└────────────────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first. This handoff is comprehensive (~1000 lines) by design.

---

## SCOPE

### IN scope

- **Layer 1**: daemon runner, PTY skill execution (with the new `supervise_pty_session` bounded supervisor), contract-mode skill execution (with `CONTRACT_SKILL_TIMEOUT_SECONDS`), IPC bridge, cron scheduler, source registry, resource arbiter, validation policy/executor (skeleton).
- **Layer 2**: Tauri desktop app, daemon supervision, IPC commands, all pages including the choice-clarification reason field fix.
- **Layer 3**: pipeline Stages 1-14 with commercial-basis state machine, multi-source ingestion (procontract + contracts-finder + fts fixture), Stage 11 hardening (order-quantity + CDP resilience + commercial-basis quantity safety), Stage 13 manifest skill + daemon fallback, Stage 14 operator decision (CLI + UI + new clarification reason field).

### OUT of scope (DO NOT attempt)

- L4 (web portal, sync engine, validator skills) — NOT BUILT.
- L5 (commercial intelligence beyond the basis state machine) — NOT BUILT.
- L6 (request pipeline automation) — NOT BUILT.
- Stage 15 (`bid_workflow`) — L6-blocked. The deferred handoff `prompts/stage-15-bid-workflow-handoff-DEFERRED.md` covers it; do not run.
- Real-money / commitment actions. The only authorized real-portal mutation remains `procontract-register-interest` (D4 safety still applies, capped at ≤3 actions).
- `apps/web/` does not exist; the Tauri desktop is the only frontend.

---

## §0 — PRE-FLIGHT (extra rigorous because of last run's footgun)

| Claim | Verify |
|---|---|
| HEAD on `main` is at the latest commit; working tree clean; synced 0/0 with `origin/main` | `git log --oneline -1; git status --short; git rev-list --left-right --count origin/main...main` |
| Daemon binary tests baseline: **202 / 202** passing on current main (was 180 before commercial-basis work added 22 regression tests) | `cargo test -p daemon --bin daemon 2>&1 \| grep "test result:"` |
| Per-file integration loop: **39 / 39 PASS** | `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" \|\| break; done \| tail -3` |
| Tauri Rust tests: query_service 25/25, daemon_bridge 14/14, native_shell 22/22 | `cargo test -p procintel-desktop --test query_service; cargo test -p procintel-desktop --test daemon_bridge; cargo test -p procintel-desktop --test native_shell` |
| vitest (apps/desktop): **44 / 44** (was 43 before; +1 for choice-clarification reason field test) | `cd apps/desktop && bunx vitest run` |
| Daemon binary builds: `target/debug/daemon` exists and is up to date | `cargo build -p daemon --bin daemon; ls -la target/debug/daemon` |
| `crates/daemon/src/db/sqlite.rs:73` declares `pub const DEFAULT_SCHEMA_EPOCH: i64 = 8` | `grep -n "pub const DEFAULT_SCHEMA_EPOCH" crates/daemon/src/db/sqlite.rs` (expected: 8) |
| **CRITICAL — SCHEMA EPOCH ALIGNMENT CHECK**: `apps/desktop/src-tauri/src/main.rs:51` declares `const EXPECTED_SCHEMA_EPOCH: i64 = N`. **It MUST match the daemon's value (8)**. If desktop is at 7 and daemon at 8, the UI bridge will refuse with `schema epoch mismatch: database has 8, daemon expects 7`. The previous E2E run hit this footgun mid-flight when a concurrent rebuild bumped the daemon to 8 while desktop was still at 7. | `grep -n "EXPECTED_SCHEMA_EPOCH:" apps/desktop/src-tauri/src/main.rs apps/desktop/src-tauri/src/daemon_bridge.rs` |
| **IF SCHEMA EPOCHS ARE MISMATCHED**: this is the FIRST FIX of the handoff. Bump `EXPECTED_SCHEMA_EPOCH` (and the matching constant in `apps/desktop/src-tauri/src/daemon_bridge.rs:16`) to match the daemon's `DEFAULT_SCHEMA_EPOCH`. Re-run vitest + Tauri Rust tests. Commit before proceeding. | inline fix |
| Migration `0011_commercial_basis_coverage.sql` exists; declares `candidate_quote_coverage` table + adds 10 columns to `candidate_pricing_assessments` (`commercial_basis_kind`, `normal_quote_coverage`, `uk_quote_id`, `china_import_quote_id`, `benchmark_quote_id`, `selected_supply_quote_id`, `supplier_spread_bps`, `fallback_discount_bps`, `missing_required_roles_json`, `coverage_evidence_json`) | `cat migrations/0011_commercial_basis_coverage.sql` |
| New CLI dispatchers wired: `--ingest marketplace-coverage` and `--ingest gross-margin` | `grep -n '"marketplace-coverage"\|"gross-margin"' crates/daemon/src/main.rs \| head -5` |
| Stage 1-14 dispatchers all wired (verify with grep — exact lines drift; current ranges fts ~1273+, contracts-finder ~1277, procontract ~1278, marketplace-coverage ~1319+, landed-cost/net-gate/manifest in same block) | `grep -n '"fts" =>\|"contracts-finder" =>\|"procontract" =>\|"manifest" =>\|"net-gate" =>\|"landed-cost" =>\|"marketplace-coverage" =>\|"gross-margin" =>' crates/daemon/src/main.rs \| head -15` |
| Stage 14 CLI flags: `--capture-decision`, `--capture-clarification`, `--withdraw-clarification`, `--withdraw-decision`, `--list-decisions` (around line ~485 onward) | `grep -n 'capture_decision: Option\|capture_clarification: Option\|withdraw_clarification: Option\|withdraw_decision: Option' crates/daemon/src/main.rs \| head -6` |
| Tauri IPC: 7 Phase 2 commands wired into `invoke_handler` block (currently lines ~84-116; verify with grep) | `grep -n 'invoke_handler(tauri::generate_handler' apps/desktop/src-tauri/src/main.rs; sed -n '84,116p' apps/desktop/src-tauri/src/main.rs` |
| 24 skills inventoried (5 procurement-ingestion, 4 PDF-extraction, 5 marketplace-search, 4 landed-cost, 1 manifest-generation, 2 source-health, 2 auth-refresh, 1 validation/base) | `find skills -name SKILL.md \| wc -l` (expected 24) |
| Required CLIs on PATH: **claude**, **codex**, **agent-browser**, **docling** | `which claude codex agent-browser; ls $HOME/.docling-venv/bin/docling 2>/dev/null` |
| **CDP BROWSER REQUIREMENT**: Chrome with `--remote-debugging-port=9222`, signed into ProContract. Same constraint as previous E2E. | `curl -sf http://localhost:9222/json/version \| head -1` |
| Live `state/state.db` baseline (READ-ONLY check; do NOT modify): 240 contract_notices (140 procontract + 100 contracts-finder), 239 procurement_processes, 7 manifests, 4 lce, 17 review verdicts, 9 intents, 11 overrides. Capture sha256 for R.25 integrity check | `sha256sum state/state.db; sqlite3 state/state.db "SELECT source_id, COUNT(*) FROM contract_notices GROUP BY source_id"` |
| Previous E2E state root may still exist at `state-e2e-20260504T113157Z` (per the 2026-05-04 manifest). It already has migration 0011 applied + Woodhouses spotlight history. **Do NOT re-use it for this handoff** — start fresh per D1 below. The previous root is reference history, not a working state | `ls -d state-e2e-20260504T113157Z 2>/dev/null && echo "exists - leave alone"` |
| `agent-mail` healthy; commits need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Parallel-safety**: this handoff runs the full pipeline end-to-end and writes extensively to a NEW isolated state.db. **Do NOT run this concurrently with any other handoff.** Especially do not run alongside any handoff that rebuilds the daemon binary mid-flight (that's how the schema epoch footgun manifested last time).

---

## §1 — DECISIONS ALREADY MADE

### D1 — Fresh isolated state root (NOT the previous one)

```bash
export E2E_TS="$(date -u +%Y%m%dT%H%M%SZ)"
export E2E_STATE_ROOT="state-e2e-$E2E_TS"
export E2E_RUNTIME_DIR="runtime-e2e-$E2E_TS"
export E2E_ARTIFACT_ROOT="artifacts-e2e-$E2E_TS"
export E2E_TARGET_DIR="/tmp/testproject1-target-e2e-$E2E_TS"

mkdir "$E2E_STATE_ROOT" "$E2E_RUNTIME_DIR" "$E2E_ARTIFACT_ROOT"

# Build daemon to an isolated target dir so concurrent edits to main don't
# accidentally swap the binary mid-run (this was the previous footgun).
CARGO_TARGET_DIR="$E2E_TARGET_DIR" cargo build -p daemon --bin daemon

# Use this exact binary for every daemon invocation in this handoff:
export E2E_DAEMON="$E2E_TARGET_DIR/debug/daemon"

# All daemon invocations in §3 use:
$E2E_DAEMON \
  --state-root "$E2E_STATE_ROOT" \
  --runtime-dir "$E2E_RUNTIME_DIR" \
  --artifact-root "$E2E_ARTIFACT_ROOT" \
  <subcommand>
```

The previous run's E2E state root (`state-e2e-20260504T113157Z`) is reference history with the Woodhouses spotlight + commercial-basis fix evidence. Do NOT re-use it; do NOT delete it; treat as forensic artifact.

### D2 — Spotlight selection (same query as previous run, but expect a different process)

ProContract publishes new notices daily. The previous spotlight (Woodhouses boilers) was published 2026-04-26 and registered interest on 2026-05-04. Today's discovery will return different candidates. Apply the D2 query from the previous handoff:

```sql
SELECT cn.id AS notice_id, p.id AS process_id, cn.title, cn.published_at, cn.deadline_at,
       cn.estimated_value_gbp_ex_vat_pence, cn.main_category
FROM contract_notices cn
JOIN procurement_processes p ON p.primary_notice_id = cn.id
WHERE cn.source_id='procontract'
  AND cn.published_at >= datetime('now','-7 days')
  AND lower(cn.main_category) NOT LIKE '%construction%'
  AND lower(cn.main_category) NOT LIKE '%works%'
  AND cn.estimated_value_gbp_ex_vat_pence BETWEEN 500000 AND 100000000
ORDER BY cn.published_at DESC LIMIT 10;
```

Pick the most physical-goods-like candidate. **The Woodhouses boiler shape is an excellent target type — boilers, equipment, ICT, lab supplies, catering — anything with a clear product specification that exercises Stage 9 marketplace search non-trivially.** Document the choice + reasoning.

If the day's batch is thin on goods, fall back to a category that includes some physical line items (e.g., "Not Elsewhere Classified" caught Woodhouses last time despite being mechanical). Avoid pure construction/services/professional fees.

### D3 — Wait strategies (unchanged from previous; D3.1 worked last time)

1. **Auto-grant** (D3.1, used in previous run): pick a notice that grants document access immediately upon registering interest. The skill output's `registrations[]` indicates this.
2. **Carryover from earlier sessions**: if no fresh auto-grant, use a process from the live state.db that was registered earlier and is now eligible for enrich.
3. **Real-time pause**: as last resort, register interest, checkpoint, wait hours, resume.

### D4 — REAL EXTERNAL MUTATION SAFETY (unchanged; cap remains 3)

Same rules as previous handoff. **Cap = 3 register-interest actions per run.** Document each one. The previous run's `MAX_PROCONTRACT_INTEREST_REGISTRATIONS_PER_RUN=3` cap is now in code and enforced; this handoff verifies the cap holds.

### D5 — Multi-source breadth

Same as previous: drive procontract + contracts-finder live; FTS only if a fixture exists.

### D6 — Bug-fix scope (unchanged)

| Symptom | Action |
|---|---|
| Trivial bug (typo, missing default, wrong column) | Fix in place + regression test + commit |
| Medium bug (logic gap, schema mismatch, missing error path) | Fix in place + regression test + commit |
| Significant bug (state machine flaw, requires new skill, requires L4-L6 features) | DO NOT fix; file as bead with reproduction steps |

### D7 — Marketplace-search safety + new commercial-basis orchestration

The new `--ingest marketplace-coverage` orchestrates Amazon UK → eBay UK → GEM → MX → Alibaba in sequence per candidate. **Use it instead of driving each marketplace individually** for Stage 9 (it's the new daemon-blessed path). Iterating individual marketplaces is still possible for failure isolation but is not the default.

All marketplace skills remain `side_effect_class: none`; safe to drive freely.

### D8 — UI walkthrough timing (unchanged)

Run §4 only after Stage 13 produces the spotlight's manifest.

### D9 — Fix-verification gates (NEW)

After each stage's functional validation succeeds, also verify the fixes that landed since the previous E2E run. The fix table in §3 lists what to check per stage. A fix-verification failure (e.g., interest-registration cap not enforced; PTY child hangs >20 min on a hung marketplace skill) is treated as a regression and triggers the validate/test/fix loop.

### D10 — Commercial-basis state machine validation (NEW)

The new state machine introduces specific behaviors at Stages 8/9/10/11/12/13 that didn't exist in the previous run. R.31-R.40 cover these. Every stage protocol includes a "commercial-basis check" callout where applicable.

### D11 — Schema epoch alignment is a HARD pre-flight gate (NEW)

If §0's schema epoch check fails, this handoff CANNOT proceed cleanly. Either:

- Fix it inline by bumping `EXPECTED_SCHEMA_EPOCH` (and `daemon_bridge.rs`'s mirror) and committing BEFORE starting; OR
- Document the mismatch as a follow-up bead and skip the UI walkthrough (§4 will fail with bridge mismatch).

The first option is strongly preferred because the UI walkthrough is a key R-criterion.

### D12 — Carry forward known-fixed bugs (NEW)

The previous E2E run found and fixed (or filed-then-fixed) these issues. This re-drive must NOT see any of them recur:

| Previous bug | Fix (commit) | Re-drive validation |
|---|---|---|
| ProContract interest over-selection beyond cap | a80d2e1 added `MAX_PROCONTRACT_INTEREST_REGISTRATIONS_PER_RUN=3` | Stage 4a: verify ≤3 registrations occurred (R.7) |
| ProContract enrich result ID drift (portal numeric vs local process ID) | a80d2e1 added resolution from hint/notice/title/buyer | Stage 4b: verify spotlight enriched cleanly without FK errors |
| Stale extraction jobs blocking new ones | a80d2e1 narrowed claim selector to extraction jobs only | Stage 8: verify queue drains without manual intervention |
| Semantic extraction missing prepared Docling artifacts | a80d2e1 added input preparation in `pdf-extraction-inputs` | Stage 8: verify semantic jobs receive the focused/full markdown |
| Contract-mode child timeout hangs | a80d2e1 added `CONTRACT_SKILL_TIMEOUT_SECONDS` + process-group launch + drain threads | Stage 11 (HMRC/SeaRates/sizing/FX): verify no hung children >timeout |
| Large semantic spec timeout | a80d2e1 added `build_focused_semantic_markdown` chunking | Stage 8: verify large mechanical-spec PDFs (~40+ pages) complete via focused subset |
| PTY marketplace child hangs after writing result.json | a80d2e1 added `supervise_pty_session` bounded supervisor with `cleanup_verdict='terminated_after_result_contract'` | Stage 9: verify marketplace skills terminate within `PTY_SKILL_TIMEOUT_SECONDS` (default 1200s) even if the agent doesn't exit |
| Choice clarification missing reason field | a80d2e1 patched `OperatorDecisionPanel.tsx` | §4 UI: verify clarification submit succeeds for `un_pursueable` / `defer` variants |
| Stage 8 missing product quantities for explicit hints | 3845b1f added quantity persistence | Stage 8: verify product_candidates.quantity populated when source had explicit hint |
| Stage 11 inferring fractional units from whole-lot value (the "42.39 boilers" bug) | 3845b1f rejects whole-lot quantity derivation when allocated revenue used | Stage 11: verify spotlight's lce confidence_breakdown does NOT contain order_quantity_basis.qty=non-integer-whole-units |
| Stage 9 stops after one UK-side quote without import attempt | 3845b1f added `--ingest marketplace-coverage` + `candidate_quote_coverage` table | Stage 9: verify spotlight has BOTH uk_side_quote AND china_import_quote attempts in coverage table |
| Stage 10 treats single-sided quote as normal | 3845b1f added `commercial_basis_kind` discrimination | Stage 10: verify spotlight's pricing_assessment has correct `commercial_basis_kind` (one of `two_sided_supplier_comparison` / `single_side_with_attempted_alternative` / `missing_required_quote_role_attempt` / fallback variants) |
| Stage 12-13 doesn't surface missing imports / quantity ambiguity | 3845b1f added reasons | Stage 13: verify manifest text mentions UK quote evidence, China/import quote state, commercial basis kind, quantity basis |
| Schema epoch operational footgun (desktop expected 7 while daemon at 8 mid-rerun) | NOT fully resolved as of this handoff start | §0 hard gate: bump desktop constants if mismatched |

---

## §2 — EXECUTION ORDER (high-level sequence)

1. Pre-flight (§0) — **including the SCHEMA EPOCH check**. If mismatched, fix + commit BEFORE starting.
2. Initialize fresh isolated state root + dedicated target dir + binary (D1).
3. **Stage 1 — Discover** (`--ingest procontract` + `--ingest contracts-finder`).
4. Pick **spotlight process** (D2 query against E2E state).
5. **Stages 2-3** — auto via `--pipeline procontract` + `--pipeline contracts-finder`.
6. **Stage 4a — Register interest** (`--ingest procontract-interest`, D4 cap of 3).
7. **Stage 4-wait** — D3 strategy (prefer D3.1 immediate auto-grant).
8. **Stage 4b — Enrich tender** (`--ingest procontract-enrich`).
9. **Stages 5, 6, 7** — re-run `--pipeline procontract` to pick up new lots/qualifications/reclassifications.
10. **Stage 8 — Extract products** (PDF cascade — structural via docling + semantic via tender-spec skill; verify focused-markdown chunking on any large spec; verify quantities persisted).
11. **Stage 9 — Marketplace coverage** (`--ingest marketplace-coverage` — new orchestrated path; verify both uk_side and china_import attempted).
12. **Stage 10 — Gross margin** (`--ingest gross-margin` — new role-coverage-aware path; verify `commercial_basis_kind` correctly set).
13. **Stage 11 — Landed cost** (`--ingest landed-cost`; verify NO unsafe quantity inference).
14. **Stage 12 — Net margin gate** (`--ingest net-gate`; verify reasons surface coverage gaps).
15. **Stage 13 — Manifest** (`--ingest manifest`; verify text surfaces commercial basis + quantity basis + coverage state).
16. **Stage 14a — Capture decision via CLI**.
17. **§4 Desktop UI flow** — launch tauri dev pointed at E2E state, walk operator workflow, capture screenshots.
18. **Stage 14b — Capture decision via UI**.
19. **§5 Operational subsystem checks**.
20. Quality gates.
21. Validation manifest.

If any stage fails, follow the validate/test/fix loop. **Re-running a stage from clean state means deleting just that stage's output rows and re-driving — NOT re-driving the whole chain from Stage 1.**

---

## §3 — STAGE-BY-STAGE PROTOCOL

For each stage:

- **Trigger**: command(s) to run.
- **Expected outcome**: success shape.
- **Validation queries**: SQL/filesystem checks.
- **Fix-verification check**: which previously-fixed bug to re-confirm.
- **Commercial-basis check** (where applicable): new state machine behavior to verify.
- **Failure modes**: common breakages.

### Stage 1 — Discover (Ingest)

**Trigger**:

```bash
$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest procontract 2>&1 | tee /tmp/e2e_v2_stage1_procontract.log

$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest contracts-finder 2>&1 | tee /tmp/e2e_v2_stage1_cf.log

# FTS only if fixture found
find . -name '*.json' -path '*fts*fixture*' 2>/dev/null | head -3
```

**Expected**: ProContract returns ≥10 new notices for the day's batch; CF returns ≥10. Both write via `persist_ingestion_from_agent_output`.

**Validation queries**:

```sql
SELECT source_id, COUNT(*) FROM contract_notices GROUP BY source_id;  -- ≥10 each
SELECT stage, COUNT(*) FROM process_stage_status WHERE stage='Normalize' AND state='succeeded' GROUP BY 1;
```

**Fix-verification**: same as before (CDP browser stable, claude agent driving cleanly).

### Stages 2-3 — Normalize + Relate

**Trigger**:

```bash
$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --pipeline procontract 2>&1 | tee /tmp/e2e_v2_stage2-7_procontract.log

$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --pipeline contracts-finder 2>&1 | tee /tmp/e2e_v2_stage2-7_cf.log
```

**Expected**: extract-lots, prequalify-notice, qualify-lot, reclassify-candidates, score-quoteability, gross-margin-gate, relate log lines. `process_relations` typically 0 (procontract `pc-...` and CF `ocds-b5fd17-...` namespaces disjoint).

### PICK SPOTLIGHT NOW

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "<the D2 query>"
export SPOTLIGHT_PID="proc-pc-<uuid>"
echo "$SPOTLIGHT_PID" > /tmp/e2e_v2_spotlight.txt
```

Document choice in the validation manifest: notice ID, title, buyer, value, category, why chosen.

### Stage 4a — Register interest (REAL portal mutation, capped)

**Trigger**:

```bash
$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest procontract-interest 2>&1 | tee /tmp/e2e_v2_stage4a_interest.log
```

**Expected**: ≤3 register-interest skill actions; spotlight registered (or pre-filter rejected with reason).

**Fix-verification (R.31)**: count distinct registrations via:

```sql
SELECT COUNT(DISTINCT process_id) FROM process_stage_status
WHERE stage='interest_registration' AND state IN ('succeeded','completed')
  AND updated_at >= '<run start ISO timestamp>';
```

**Must be ≤3**. If >3, the cap regressed — bug.

### Stage 4-wait

Per D3. Document strategy used.

### Stage 4b — Enrich tender

**Trigger**:

```bash
$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest procontract-enrich 2>&1 | tee /tmp/e2e_v2_stage4b_enrich.log
```

**Expected**: spotlight enriched cleanly; PDFs downloaded to `$E2E_ARTIFACT_ROOT/downloads/procontract/<process_id>/`; tender_enrichment stage_status row.

**Fix-verification (R.32)**: enrich must persist correctly without FK errors. Inspect agent_session_logs for the enrich session — `failure_reason` should be NULL. If you see `FOREIGN KEY constraint failed` in logs, the ID-resolution fix regressed.

### Stages 5-7 — Lots / qualify / reclassify

Re-run pipeline to pick up new spotlight lots:

```bash
$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --pipeline procontract 2>&1 | tee /tmp/e2e_v2_stage5-7_procontract_after_enrich.log
```

**Validation queries**:

```sql
SELECT * FROM notice_lots WHERE notice_id IN (SELECT id FROM contract_notices WHERE process_id='$SPOTLIGHT_PID');
SELECT * FROM lot_qualifications WHERE lot_id IN
  (SELECT id FROM notice_lots WHERE notice_id IN (SELECT id FROM contract_notices WHERE process_id='$SPOTLIGHT_PID'));
```

### Stage 8 — Extract products (PDF cascade)

**Trigger**: queue PDF jobs via skill-path or run-next-job:

```bash
# Identify spotlight PDFs
ls $E2E_ARTIFACT_ROOT/downloads/procontract/*/*.pdf | head -10

# Drive structural extraction (docling) per PDF if not auto-queued, then semantic
# Semantic extraction is the test target for the "focused markdown" chunking fix.
# For each PDF, queue:
$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --skill-path skills/pdf-extraction/tender-specification \
  --job-parameters-json '{"process_id":"'$SPOTLIGHT_PID'", "document_paths":["<absolute pdf path>"]}' \
  2>&1 | tee /tmp/e2e_v2_stage8_pdf.log

# OR drain the queued jobs:
while $E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
        --run-next-job 2>&1 | tee -a /tmp/e2e_v2_stage8_runner.log | grep -q "completed"; do :; done
```

**Validation queries** (Stage 8 + commercial-basis quantity persistence):

```sql
-- Stage 8 candidates exist
SELECT id, name, candidate_kind, identity_confidence_bps, quoteability_score_bps,
       quantity, quantity_unit, quantity_confidence_bps
FROM product_candidates WHERE process_id='$SPOTLIGHT_PID';

-- Document extractions
SELECT id, semantic_extraction_state, structural_extraction_state, page_count
FROM document_extractions WHERE process_id='$SPOTLIGHT_PID';

-- Semantic extractions
SELECT id, confidence_bps, json_extract(extraction_json, '$.products') AS products
FROM document_semantic_extractions WHERE process_id='$SPOTLIGHT_PID' LIMIT 5;
```

**Fix-verification (R.33)**: large mechanical specs must complete (no timeout-pending forever). If a PDF is >40 pages and the semantic skill takes the full markdown, it would timeout. Verify by checking `agent_session_logs` for the semantic skill — should NOT show `AGENT_STALLED` or run >300s. The focused-markdown chunking should have written `docling-markdown.md` (focused) + `docling-markdown-full.md` (preserved) to the job workdir.

**Commercial-basis check (R.34)**: if spotlight had explicit quantity hints in source documents (e.g., "2no boilers", "qty: 5"), `product_candidates.quantity` should be populated. Verify with the validation query above. NULL is acceptable if the document had no explicit quantity.

### Stage 9 — Marketplace coverage (NEW orchestrated path)

**Trigger** (use the NEW orchestrated path):

```bash
$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest marketplace-coverage 2>&1 | tee /tmp/e2e_v2_stage9_coverage.log
```

This orchestrates Amazon UK → eBay UK → GEM → MX → Alibaba per candidate, persisting durable rows in BOTH `search_attempts` AND `candidate_quote_coverage`.

**Validation queries** (commercial-basis coverage):

```sql
-- Coverage state per spotlight candidate
SELECT product_candidate_id, required_role, attempted, usable_quote_count, coverage_state, latest_source, latest_failure_mode
FROM candidate_quote_coverage WHERE product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID');

-- Search attempts
SELECT product_candidate_id, search_failure_mode, COUNT(*)
FROM search_attempts WHERE product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID')
GROUP BY 1, 2;

-- Supplier quotes that ended up persisted
SELECT id, source_label, currency, unit_price_gbp_pence
FROM supplier_quotes WHERE product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID');
```

**Fix-verification (R.35)**: coverage table MUST have rows for BOTH `uk_side_quote` AND `china_import_quote` required_role values for the spotlight. If only `uk_side_quote` exists, the orchestration regressed. Failed Alibaba attempts should persist as `coverage_state='attempted_failed'` with explicit `latest_failure_mode`.

**Fix-verification (R.36)**: PTY supervisor must terminate hung marketplace skills. Check `agent_session_logs.cleanup_verdict` for marketplace sessions — should be `clean` or `terminated_after_result_contract`. If `forced` after the per-attempt budget elapsed, that's the supervisor doing its job (acceptable; means a child genuinely hung and was reaped).

### Stage 10 — Gross margin (NEW role-coverage-aware path)

**Trigger** (use the NEW orchestrated path):

```bash
$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest gross-margin 2>&1 | tee /tmp/e2e_v2_stage10_gross.log
```

**Validation queries** (commercial basis labelling):

```sql
SELECT cpa.id, cpa.product_candidate_id, cpa.gross_margin_bps, cpa.gross_margin_verdict,
       cpa.commercial_basis_kind, cpa.normal_quote_coverage,
       cpa.uk_quote_id, cpa.china_import_quote_id, cpa.benchmark_quote_id,
       cpa.selected_supply_quote_id, cpa.supplier_spread_bps, cpa.fallback_discount_bps,
       cpa.missing_required_roles_json, cpa.coverage_evidence_json
FROM candidate_pricing_assessments cpa
WHERE cpa.product_candidate_id IN (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID');
```

**Commercial-basis check (R.37)**: `commercial_basis_kind` MUST be one of:

- `two_sided_supplier_comparison` (best — both UK + China quote attempted + at least one of each usable)
- `single_side_with_attempted_alternative` (acceptable — UK quote present; Alibaba attempted but no usable result)
- `missing_required_quote_role_attempt` (degraded — UK quote present but Alibaba NOT attempted; this should NOT happen if Stage 9 ran cleanly)
- A labelled fallback variant (e.g., `fallback_after_import_failure`)

Whichever value it is, it must be HONEST about what happened. The previous bug treated single-sided quotes as normal — verify that doesn't recur.

### Stage 11 — Landed cost

**Trigger**:

```bash
$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest landed-cost 2>&1 | tee /tmp/e2e_v2_stage11_landed.log
```

**Validation queries**:

```sql
SELECT lce.id, lce.decision_grade, lce.economic_landed_cost_gbp_pence,
       lce.commercial_estimate_confidence_bps,
       json_extract(lce.confidence_breakdown, '$.order_quantity_basis.kind') AS basis_kind,
       json_extract(lce.confidence_breakdown, '$.order_quantity_basis.qty') AS basis_qty,
       json_extract(lce.confidence_breakdown, '$.commercial_basis.kind') AS commercial_basis,
       lce.blockers, lce.warnings
FROM landed_cost_estimates lce
WHERE lce.product_candidate_id IN (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID');
```

**Fix-verification (R.38) — THE "42.39 BOILERS" CHECK**: `basis_qty` must NOT be a non-integer fractional value derived from whole-lot value / unit-quote when Stage 10 used allocated revenue. Specifically:

- If basis_kind is `from_lot_quantity` AND Stage 10 commercial_basis_kind was anything other than two-sided, this is the regression. Stage 11 should have refused and returned `basis_undetermined` instead.
- If basis_qty is a non-integer like `42.39`, that's the dangerous fractional inference — bug.
- Acceptable values: integer counts (1, 2, 5, 100, etc.) OR `basis_undetermined` with an explanatory `reason`.

**Commercial-basis check (R.39)**: `confidence_breakdown.commercial_basis.kind` should mirror Stage 10's `commercial_basis_kind`. If they disagree, the Stage 11 hand-off reads stale Stage 10 state.

**Fix-verification (R.40)**: contract-mode skills (HMRC customs, SeaRates freight, sizing, FX) must NOT hang. Check agent_session_logs for these sessions — `cleanup_verdict` should be `clean` or `terminated_after_timeout` (if a skill genuinely hung). No session should run >`CONTRACT_SKILL_TIMEOUT_SECONDS` without termination.

### Stage 12 — Net margin gate

**Trigger**:

```bash
$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest net-gate 2>&1 | tee /tmp/e2e_v2_stage12_netgate.log
```

**Validation queries**:

```sql
SELECT product_candidate_id, verdict, reasons_json
FROM candidate_net_margin_assessments WHERE product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID');
```

**Commercial-basis check**: `reasons_json` should explicitly reference the new commercial-basis state when relevant — e.g., `missing_required_quote_role_attempt`, `missing_china_import_quote_after_attempt`, `fallback_screening_basis`, `incomplete_quote_coverage`, `quantity_basis_undetermined`. The previous E2E manifest noted Stage 12 didn't surface these; the fix should now make them visible.

### Stage 13 — Manifest

**Trigger**:

```bash
$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest manifest 2>&1 | tee /tmp/e2e_v2_stage13_manifest.log
```

**Validation queries**:

```sql
SELECT process_id, stoplight, suggested_action, manifest_basis_hash, artifact_hash
FROM manifests WHERE process_id='$SPOTLIGHT_PID';
```

```bash
cat $E2E_ARTIFACT_ROOT/manifests/$SPOTLIGHT_PID/manifest.md | head -100
```

**Commercial-basis check**: manifest text MUST surface:

- UK quote evidence (which marketplace, which price)
- China/import quote state (attempted? usable? skipped?)
- Commercial basis kind (one of the labels from Stage 10)
- Quantity basis (kind + value or "undetermined" with reason)
- Fallback / screening status if applicable

If any of these are absent from a process where they should appear, Stage 13's manifest skill regressed.

### Stage 14a — Capture decision via CLI

**Trigger**:

```bash
$E2E_DAEMON --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" \
  --capture-decision $SPOTLIGHT_PID \
  --disposition watch \
  --idempotency-key e2e-v2-cli-$E2E_TS \
  --reasoning "End-to-end re-validation: spotlight $SPOTLIGHT_PID walked through all L1-L3 stages with new commercial-basis state machine." \
  2>&1 | tee /tmp/e2e_v2_stage14a_cli.log
```

**Validation queries**:

```sql
SELECT idempotency_key, status, desired_disposition, decision_note, queued_at
FROM pending_operator_intents WHERE process_id='$SPOTLIGHT_PID' ORDER BY queued_at DESC LIMIT 3;
```

---

## §4 — DESKTOP UI VALIDATION

After Stage 13, exercise the desktop UI pointed at the E2E state. Use Path A (env overrides) — Path B (snapshot swap) is a fallback if Path A is broken.

### Path A — Env overrides

```bash
PROCINTEL_DESKTOP_STATE_ROOT="$E2E_STATE_ROOT" \
PROCINTEL_DESKTOP_RUNTIME_DIR="$E2E_RUNTIME_DIR" \
PROCINTEL_DESKTOP_ARTIFACT_ROOT="$E2E_ARTIFACT_ROOT" \
PROCINTEL_DAEMON_PROGRAM="$E2E_DAEMON" \
  ( cd apps/desktop && bunx tauri dev )
```

The `PROCINTEL_DAEMON_PROGRAM` override is ESSENTIAL — without it, the desktop's third-fallback launcher rebuilds from cargo workspace, which is the schema-epoch footgun pathway from the previous run. Pinning to `$E2E_DAEMON` (built in §0/D1 at a known schema epoch) eliminates that risk.

### UI walkthrough

1. Manifest Inbox loads (route `/inbox`). Spotlight card visible with stoplight + suggested action.
2. Click spotlight → Process Detail. Markdown via `ManifestViewer`. Decision history empty initially.
3. Open Clarification form. **Verify the choice prompt now has BOTH a variant select AND a Reason textarea** (the previous run's bug fix). Submit a clarification with `reason_code=no_pricing_signal`, variant=`defer`, reason="UI re-validation".
4. Click "Re-run pipeline". Progress modal cycles `landed-cost` → `net-gate` → `manifest`. Modal closes. Page refreshes.
5. Capture decision via Quick-Decide panel: select `watch`, type reasoning, click Watch button. Decision history shows new entry.

### Stage 14b — Decision via UI

The UI Quick-Decide IS Stage 14b. Verify in DB:

```sql
SELECT idempotency_key, status, desired_disposition, decision_note FROM pending_operator_intents
WHERE process_id='$SPOTLIGHT_PID' ORDER BY queued_at DESC LIMIT 3;
```

Expect ≥2 rows now (Stage 14a CLI + Stage 14b UI).

### Cross-session staleness E2E

While desktop is open with spotlight loaded:

1. **External terminal**: drive `--ingest manifest` against $E2E_STATE_ROOT.
2. Reload UI inbox.
3. Stale banner should appear IF the basis hash changed.

If basis hash didn't change (the previous run's case — clarification didn't shift inputs), document honestly. Don't force a synthetic stale signal.

### Screenshots

Capture at each step. Commit to `docs/pipeline/full-pipeline-e2e-revalidation-screenshots/`.

---

## §5 — OPERATIONAL SUBSYSTEM CHECKS

Same as previous E2E:

| Subsystem | Check |
|---|---|
| Cron scheduler | Inspect daemon logs for cron tick lines; verify cron_definitions / source_runtime_state tables align with registered sources |
| Source health | `source_runtime_state` shows 5 registered sources; circuit_state for each |
| Resource arbiter | No panics during multi-source/multi-marketplace runs |
| Validation pipeline | `validation_verdicts` empty (L4 gap, expected) |
| Workflow jobs queue | Drained — completed >> pending after all stages |
| IPC bridge | Tauri bootstrap reaches full-control mode |

```sql
SELECT 'cron_defs', COUNT(*) FROM cron_definitions;
SELECT 'workflow_jobs by status', status, COUNT(*) FROM workflow_jobs GROUP BY status;
SELECT 'validation_verdicts', COUNT(*) FROM validation_verdicts;
SELECT 'sources health', source_id, circuit_state FROM source_runtime_state;
```

---

## §6 — ACCEPTANCE CRITERIA — R.1 to R.40

R.1-R.30 carry forward from the previous handoff (with refreshed test counts):

| # | Criterion |
|---|---|
| R.1 | Pre-flight all green; SCHEMA EPOCHS aligned (daemon DEFAULT_SCHEMA_EPOCH = desktop EXPECTED_SCHEMA_EPOCH); CDP browser running; isolated state root + dedicated target dir created |
| R.2 | Stage 1: `--ingest procontract` returned ≥10 new notices into $E2E_STATE_ROOT |
| R.3 | Stage 1: `--ingest contracts-finder` returned ≥10 new notices |
| R.4 | Stage 1: FTS ran with fixture OR documented as deferred |
| R.5 | Stage 2-3: deterministic chains ran cleanly |
| R.6 | Spotlight selected per D2; choice + reasoning documented |
| R.7 | Stage 4a: ≤3 register-interest actions (cap enforced); spotlight registered or rejected with reason |
| R.8 | Stage 4-wait: D3 strategy documented |
| R.9 | Stage 4b: spotlight enriched; ≥1 PDF in $E2E_ARTIFACT_ROOT/downloads |
| R.10 | Stages 5-7: `--pipeline procontract` re-ran after enrichment |
| R.11 | Stage 8: ≥1 product_candidate for spotlight |
| R.12 | Stage 9: at least one marketplace produced ≥1 GBP supplier_quote OR all 5 documented why no quote |
| R.13 | Stage 10-11: ≥1 lce row for spotlight (or documented why 0) |
| R.14 | Stage 12: ≥1 net-margin assessment for spotlight |
| R.15 | Stage 13: 1 manifest row + rendered manifest.md |
| R.16 | Stage 14a CLI: queued intent row with test idempotency key |
| R.17 | §4 UI: tauri dev launches; spotlight visible in Inbox |
| R.18 | §4 UI: spotlight detail view renders manifest |
| R.19 | §4 UI: re-run pipeline button cycles modal + refreshes manifest hash (or basis stable; documented honestly) |
| R.20 | §4 UI: Quick-Decide produces a second `pending_operator_intents` row |
| R.21 | §4 UI: cross-session staleness E2E (if basis shifted) OR documented as basis-stable |
| R.22 | §4 Screenshots committed to docs/pipeline/full-pipeline-e2e-revalidation-screenshots/ |
| R.23 | §5: operational subsystems healthy |
| R.24 | §5: validation_verdicts empty (L4 gap documented) |
| R.25 | Live state.db NOT MODIFIED — sha256 hash matches before/after |
| R.26 | Bugs surfaced + fixed: every fix has a regression test + commit; bugs out of scope filed as beads |
| R.27 | Daemon binary tests still **202 / 202** (or +N if regressions added); per-file integration loop 39/39; vitest 44/44; Tauri Rust tests query_service 25/25 + daemon_bridge 14/14 + native_shell 22/22 |
| R.28 | Quality gates: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings` clean |
| R.29 | Validation manifest written to `docs/pipeline/full-pipeline-e2e-revalidation-validation-manifest.md` |
| R.30 | Final commit + push to origin |

R.31-R.40 are NEW for this re-drive — verify every previously-fixed bug + commercial-basis behavior:

| # | Criterion |
|---|---|
| R.31 | **Interest cap fix holds**: ≤3 distinct register-interest registrations during this run (Stage 4a) — verifies `MAX_PROCONTRACT_INTEREST_REGISTRATIONS_PER_RUN=3` from a80d2e1 |
| R.32 | **Enrich ID resolution fix holds**: spotlight enrich session has no FK errors / no `process_id` mismatch failures (Stage 4b agent_session_logs.failure_reason IS NULL) |
| R.33 | **Semantic chunking fix holds**: any spotlight PDF >40 pages has `docling-markdown.md` (focused) AND `docling-markdown-full.md` (preserved) in its job workdir; semantic skill completed within timeout |
| R.34 | **Stage 8 quantity persistence holds**: if any spotlight source document has explicit quantity hints, `product_candidates.quantity` populated (NULL acceptable only when document had no explicit hint) |
| R.35 | **Marketplace coverage orchestration holds**: spotlight has rows in `candidate_quote_coverage` for BOTH `uk_side_quote` AND `china_import_quote` required_role values (each row has `attempted=1`) |
| R.36 | **PTY supervisor holds**: marketplace skill sessions in agent_session_logs have `cleanup_verdict` ∈ {`clean`, `terminated_after_result_contract`}. None in `forced` after >`PTY_SKILL_TIMEOUT_SECONDS` (default 1200s). |
| R.37 | **Stage 10 commercial_basis_kind correctly labelled**: spotlight's `candidate_pricing_assessments.commercial_basis_kind` is one of the legitimate values (not `legacy_single_quote` unless intentional) |
| R.38 | **Stage 11 quantity safety holds**: spotlight's lce confidence_breakdown does NOT contain non-integer `order_quantity_basis.qty` derived from whole-lot value when Stage 10 used allocated revenue. The "42.39 boilers" pattern must NOT recur. |
| R.39 | **Stage 11/10 commercial basis consistency**: lce `confidence_breakdown.commercial_basis.kind` matches Stage 10's `commercial_basis_kind` for the same candidate |
| R.40 | **Contract-mode skill timeout holds**: HMRC/SeaRates/sizing/FX sessions in agent_session_logs do not exceed `CONTRACT_SKILL_TIMEOUT_SECONDS` without termination; `cleanup_verdict` set appropriately |

---

## §7 — VALIDATION QUERY PACK

Save to `/tmp/e2e_v2_validate.sh`:

```bash
#!/usr/bin/env bash
DB="${1:-$E2E_STATE_ROOT/state.db}"
SPOTLIGHT="${2:-$SPOTLIGHT_PID}"

echo "=== Stage 1-3 ==="
sqlite3 -header -column "$DB" "
SELECT source_id, COUNT(*) AS notices FROM contract_notices GROUP BY source_id;
SELECT 'processes total', COUNT(*) FROM procurement_processes;
SELECT 'process_relations', COUNT(*) FROM process_relations;
"

echo "=== Stage 4 (spotlight) ==="
sqlite3 -header "$DB" "
SELECT stage, state, COUNT(*) FROM process_stage_status WHERE process_id='$SPOTLIGHT' GROUP BY 1,2;
SELECT 'interest_registration count this run', COUNT(DISTINCT process_id)
FROM process_stage_status WHERE stage='interest_registration' AND state IN ('succeeded','completed');
"

echo "=== Stage 5-7 ==="
sqlite3 -header "$DB" "
SELECT 'lots', COUNT(*) FROM notice_lots WHERE notice_id IN
  (SELECT id FROM contract_notices WHERE process_id='$SPOTLIGHT');
SELECT 'lot_qualifications', COUNT(*) FROM lot_qualifications WHERE lot_id IN
  (SELECT id FROM notice_lots WHERE notice_id IN (SELECT id FROM contract_notices WHERE process_id='$SPOTLIGHT'));
"

echo "=== Stage 8 (with quantity persistence) ==="
sqlite3 -header "$DB" "
SELECT id, name, candidate_kind, quantity, quantity_unit, quantity_confidence_bps,
       identity_confidence_bps, quoteability_score_bps
FROM product_candidates WHERE process_id='$SPOTLIGHT';
"

echo "=== Stage 9 (with coverage orchestration) ==="
sqlite3 -header "$DB" "
SELECT product_candidate_id, required_role, attempted, usable_quote_count,
       coverage_state, latest_source, latest_failure_mode
FROM candidate_quote_coverage WHERE product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT');
SELECT id, source_label, currency, unit_price_gbp_pence FROM supplier_quotes
WHERE product_candidate_id IN (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT');
"

echo "=== Stage 10 (commercial basis) ==="
sqlite3 -header "$DB" "
SELECT cpa.product_candidate_id, cpa.gross_margin_bps, cpa.gross_margin_verdict,
       cpa.commercial_basis_kind, cpa.normal_quote_coverage,
       cpa.uk_quote_id, cpa.china_import_quote_id, cpa.benchmark_quote_id,
       cpa.selected_supply_quote_id, cpa.supplier_spread_bps,
       cpa.missing_required_roles_json
FROM candidate_pricing_assessments cpa WHERE cpa.product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT');
"

echo "=== Stage 11 (quantity safety + commercial basis consistency) ==="
sqlite3 -header "$DB" "
SELECT lce.id, lce.decision_grade, lce.economic_landed_cost_gbp_pence,
       lce.commercial_estimate_confidence_bps,
       json_extract(lce.confidence_breakdown, '\$.order_quantity_basis.kind') AS basis_kind,
       json_extract(lce.confidence_breakdown, '\$.order_quantity_basis.qty') AS basis_qty,
       json_extract(lce.confidence_breakdown, '\$.commercial_basis.kind') AS commercial_basis,
       lce.blockers, lce.warnings
FROM landed_cost_estimates lce WHERE lce.product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT');
"

echo "=== Stage 12 ==="
sqlite3 -header "$DB" "
SELECT product_candidate_id, verdict, reasons_json
FROM candidate_net_margin_assessments WHERE product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT');
"

echo "=== Stage 13 ==="
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
  (SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT');
"

echo "=== Fix-verification: agent_session_logs cleanup verdicts ==="
sqlite3 -header "$DB" "
SELECT skill_path, status, cleanup_verdict, COUNT(*) FROM agent_session_logs
WHERE started_at >= '<run start>' GROUP BY 1, 2, 3 ORDER BY 1;
"
```

---

## §8 — FAILURE-MODE PLAYBOOK

Carry forward from previous handoff + new patterns:

| Symptom | Diagnosis |
|---|---|
| Schema epoch mismatch error from desktop bridge | Pre-flight should have caught this. Bump desktop's `EXPECTED_SCHEMA_EPOCH` constant + `daemon_bridge.rs` mirror to match daemon's `DEFAULT_SCHEMA_EPOCH`. Rebuild + commit. |
| `--ingest marketplace-coverage` not recognized | This subcommand was added in 3845b1f. If not recognized, the daemon binary is older than the migration. Rebuild daemon: `cargo build -p daemon --bin daemon` AND ensure `$E2E_DAEMON` points at the freshly-built binary (D1). |
| Stage 9 produces only `uk_side_quote` rows | Marketplace-coverage orchestration didn't reach Alibaba. Inspect the orchestration log; verify Alibaba marketplace is in the chain. May indicate Alibaba session captcha-blocked or skipped. |
| Stage 11 lce shows `basis_qty=42.39` (or any non-integer fractional) | THE REGRESSION. Stage 11 should reject this. Inspect `crates/daemon/src/main.rs` around the quantity-derivation logic; verify the `Stage10 used allocated revenue` check is intact. |
| Stage 10 shows `commercial_basis_kind='legacy_single_quote'` for spotlight | Either (a) the spotlight legitimately has only a UK quote and Alibaba was never attempted (D7 violation; Stage 9 incomplete), or (b) the labelling code regressed. Inspect Stage 9's coverage table first. |
| Choice clarification submit fails with `'<variant>' override missing string 'reason'` | The `OperatorDecisionPanel.tsx` reason field fix from a80d2e1 regressed. Verify `apps/desktop/src/pages/OperatorDecisionPanel.tsx` still renders the Reason textarea + seeds default reason. |
| PTY marketplace skill hangs >20 min | The supervisor fix from a80d2e1 should reap. Inspect agent_session_logs.cleanup_verdict + the skill's session transcript. If supervisor isn't terminating, the bound supervisor regressed. |
| Daemon panic / segfault | Real bug. Capture stack trace. File as bead. Attempt narrow fix. |
| State.db corruption | Run `PRAGMA integrity_check`. Restart from fresh state root if broken. |

(Other failure modes from the previous handoff still apply.)

---

## §9 — CONSTRAINTS

1. **Live `state/state.db` MUST NOT be modified.** Use isolated state root (D1). sha256sum check at end (R.25).
2. **Real portal mutations limited to ≤3 register-interest actions** per D4. Document each.
3. **No real-money / no real-bid actions.** Stage 15 stays L6-blocked.
4. **CDP browser launch is operator-driven.**
5. **DAEMON BINARY PINNED** — use `$E2E_DAEMON` for every daemon invocation. Do NOT let the desktop's third-fallback launcher rebuild from cargo workspace mid-run (that's how the schema epoch footgun manifested last time). This is enforced by `PROCINTEL_DAEMON_PROGRAM=$E2E_DAEMON` in §4.
6. **Stage 14 demo state in live DB MUST remain intact** — sha256 hash check.
7. **Bug fixes per D6 scope**.
8. **Quality gates per R.27/R.28 mandatory** — daemon binary tests at 202 (or 202+N if regressions added).
9. **Don't re-run successful stages** unless validation reveals an issue.
10. **The wait period (Stage 4) is acceptable** — D3 strategy documented.

---

## §10 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/full-pipeline-e2e-revalidation-validation-manifest.md`. Template:

```markdown
# Full Pipeline End-to-End RE-VALIDATION Manifest

> Generated: <ISO timestamp>
> Status: GREEN / RED / YELLOW (with caveats)
> HEAD: <commit>
> E2E state root: <path>
> Spotlight process: <id> ("<title>")
> Previous run reference: state-e2e-20260504T113157Z (Woodhouses boilers)

## Summary

<2-3 paragraphs: what was the spotlight, did the chain complete end-to-end,
how many bugs surfaced + fixed (vs how many fixes from previous run held),
final operator decision captured, commercial-basis state machine outcomes>

## Pre-flight schema epoch alignment

| Constant | Value | Status |
|---|---|---|
| daemon DEFAULT_SCHEMA_EPOCH | 8 | ✓ |
| desktop EXPECTED_SCHEMA_EPOCH | 8 | ✓ (bumped from 7 in pre-flight if needed) |
| desktop daemon_bridge.rs mirror | 8 | ✓ |
| live state/state.db user_version | 7 (untouched per R.25) | ✓ |
| E2E state.db user_version after migration 0011 | 8 | ✓ |

## Setup

| Item | Value |
|---|---|
| Branch | main |
| HEAD at start | <commit> |
| HEAD at end | <commit> (if changed) |
| Isolated state root | $E2E_STATE_ROOT |
| Isolated daemon binary | $E2E_DAEMON |
| CDP browser | <command + version> |
| Wait strategy used | D3.1 / D3.2 / D3.3 |

## Spotlight selection (D2)

| Field | Value |

## Stage-by-stage outcomes

| Stage | Result | Evidence |
| ... |

## Fix-verification table (R.31-R.40)

| # | Fix | Re-validated? | Evidence |
| R.31 | Interest cap (≤3) | YES/NO | <count> registrations |
| R.32 | Enrich ID resolution | YES/NO | <session FK error count> |
| R.33 | Semantic chunking | YES/NO | <focused vs full markdown sizes> |
| R.34 | Stage 8 quantity persistence | YES/NO | <quantity nullity for spotlight> |
| R.35 | Marketplace coverage orchestration | YES/NO | <coverage row count per role> |
| R.36 | PTY supervisor | YES/NO | <cleanup_verdict distribution> |
| R.37 | Stage 10 commercial_basis_kind | YES/NO | <kind value> |
| R.38 | Stage 11 quantity safety | YES/NO | <basis_qty value> |
| R.39 | Stage 11/10 commercial basis consistency | YES/NO | <comparison> |
| R.40 | Contract-mode timeout | YES/NO | <session durations> |

## Real portal mutations (per D4)

| Notice ID | Buyer | Title | Action | Result |

## NEW bugs surfaced + fixed (this run)

| File | Bug | Fix | Regression test |

## NEW bugs surfaced + filed (deferred)

| Bead ID | Title | Reproduction | Why deferred |

## Desktop UI walkthrough

| Step | Screenshot | Result |

## Operational subsystem checks

| Subsystem | Status | Evidence |

## Live state.db integrity check (R.25)

| Check | Result |

## R.1–R.40 status

| # | Criterion | Status | Evidence |

## Quality Gates

| Gate | Result |

## Stages NOT exercised

- Stage 15: L6-blocked
- Web portal / sync / validators / commercial intelligence: L4-L6 not built
```

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon              # 202 / 202 (or +N if regressions added)
cargo test -p procintel-desktop --test query_service     # 25 / 25
cargo test -p procintel-desktop --test daemon_bridge     # 14 / 14
cargo test -p procintel-desktop --test native_shell      # 22 / 22
cd apps/desktop && bunx vitest run               # 44 / 44
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done   # 39 / 39
```

---

## §11 — REMINDERS

- ULTRATHINK and be meticulous.
- This is a **CONFIRMATION run**, not a build run. The point is to verify all the fixes from the previous E2E iteration land correctly under live load. Bugs surfacing here would be REGRESSIONS — treat them seriously.
- The schema epoch alignment is the hardest pre-flight requirement. Get it right BEFORE starting; fix-and-commit if mismatched; don't try to work around mid-flight.
- Pin the daemon binary via `$E2E_DAEMON` and `PROCINTEL_DAEMON_PROGRAM` so the desktop never rebuilds it during the run. This eliminates the previous footgun by construction.
- The Woodhouses boiler spotlight from the previous run remains the canonical "shape" for what this re-drive validates. Today's spotlight will be different but should exercise the same characteristics: physical goods, marketplace-priceable, tender-stage with documents.
- The new commercial-basis state machine is the most consequential change since the previous run. R.31-R.40 are how we verify it works honestly — a passing R.37 says "Stage 10 told the truth about what coverage it had", which is exactly the kind of operational integrity the platform relies on.
- The "42.39 boilers" check (R.38) is the single most important regression check. If basis_qty comes back as a non-integer fractional value derived from whole-lot value, the fix regressed and the operator could plan a bid based on garbage quantity inference. Catch this if it recurs.
- After this handoff lands GREEN, the L1-L3 product can credibly claim "validated end-to-end on real procurement data, with the new commercial-basis state machine demonstrating honest commercial-readiness labelling and quantity safety, AND with every previously-found bug confirmed-not-regressed under continuous live load." That's the threshold for stakeholder demo + L4 build start (whichever comes first).
- The previous run's manifest (`docs/pipeline/full-pipeline-e2e-validation-manifest.md`) is the reference for what the pipeline did before fixes; this run's manifest is the reference for what it does after fixes. Both should be reviewable side-by-side as the "before / after" of the commercial-basis hardening.
