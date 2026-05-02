# Multi-Source Ingestion Validation — Drive FTS + Contracts Finder + Stage 3 Dedup

ULTRATHINK and be meticulous.

## MISSION

Today's live DB has **133 procurement processes, all from ProContract, zero from FTS, zero from Contracts Finder, and zero `process_relations` rows** — meaning Stage 3 (Relate / cross-source dedup) has never had multi-source data to test against. The full pipeline (Stages 1–14) is built and works, but everything we've validated has run on a single-source corpus.

This handoff drives the **other two procurement-source skills** (`procurement-ingestion/fts` and `procurement-ingestion/contracts-finder`) against real data, then validates that:
1. New processes flow cleanly through Stages 1 → 13 (no regressions).
2. **Stage 3 actually detects cross-source duplicates** when the same opportunity appears in multiple sources, and creates `process_relations` rows.
3. Stages 11 (landed cost) and 12 (net margin gate) handle the new variety (different HS families, different origins, possibly non-GBP supplier quotes if Stage 9 finds any).
4. Stage 13 manifests render correctly for the new processes; Stage 14 CLI can decide on them.

This is more of a **drive-and-observe + fix-as-needed** handoff than a build-from-scratch one. The skills already exist; the daemon already dispatches them. The work is exercising them on real data, surfacing whatever breaks, and fixing it.

You will **drive → validate → fix → repeat in a loop** until every numbered acceptance criterion (R.1–R.13 below) is GREEN.

```
┌────────────────────────────────────────────────────────────┐
│                          THE LOOP                            │
│   1. Pre-flight (§0)                                         │
│   2. Drive: --ingest fts (then --ingest contracts-finder)   │
│   3. Validate against R.1..R.13 (queries inline)             │
│   4. Failures? → diagnose, fix code or skill prompt, repeat  │
│      from step 2 (with appropriate state reset)              │
│   5. Quality gates: full test suite per AGENTS.md            │
│   6. All green → write the validation manifest               │
└────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## §0 — PRE-FLIGHT

| Claim | Verify |
|---|---|
| `run_fts_live_loop` exists at `crates/daemon/src/main.rs:13786` (production code; dispatched from `--ingest fts`); `run_contracts_finder_live_loop` exists at `main.rs:13022` (dispatched from `--ingest contracts-finder`) | `grep -n "fn run_fts_live_loop\|fn run_contracts_finder_live_loop" crates/daemon/src/main.rs` |
| **CRITICAL**: `--ingest fts` REQUIRES `--fixture-path <path>` (the dispatcher at `main.rs:1248-1251` calls `load_fixture(cli.fixture_path.as_deref())?` and passes it into `run_fts_live_loop(runtime, fixture, cli)`). The contracts-finder path has its own cursor model (no fixture needed — it pulls from `source_cursors` table). Verify the FTS fixture format from existing fixtures or the `LiveLoopFixture` struct shape | `grep -n "fn load_fixture\|struct LiveLoopFixture" crates/daemon/src/main.rs; find . -name '*.json' -path '*fixture*' 2>/dev/null \| head -5` |
| `skills/procurement-ingestion/fts/SKILL.md` and `skills/procurement-ingestion/contracts-finder/SKILL.md` both exist; verify their actual `agent_cli` and `interaction_profile` (the SeaRates skill turned out to be Codex, not Claude — don't assume) | `head -20 skills/procurement-ingestion/fts/SKILL.md; head -20 skills/procurement-ingestion/contracts-finder/SKILL.md` |
| Currently 133 processes, all ProContract; 0 FTS, 0 contracts-finder; 0 process_relations | `sqlite3 state/state.db "SELECT source_id, COUNT(*) FROM contract_notices GROUP BY source_id; SELECT COUNT(*) FROM process_relations"` |
| Stage 3 (`run_relate_stage`) at `crates/daemon/src/main.rs:18381` groups processes; the only `relation_type` value emitted today is `'merged_same_process'` (constant at `main.rs:18440`); confidence_bps = 10000 | `grep -n "fn run_relate_stage\|merged_same_process" crates/daemon/src/main.rs \| head -5` |
| The 7 existing `pipeline_assessments` rows + 7 manifests are all from the ProContract slice; new FTS/Contracts Finder data will increase these counts unless Stage 8/9 reject new candidates | `sqlite3 state/state.db "SELECT COUNT(*) FROM pipeline_assessments; SELECT COUNT(*) FROM manifests"` |
| The `sources` table has 5 source IDs: `procontract`, `procontract-enrich`, `procontract-interest`, `fts`, `contracts-finder`. Joining via `source_id` is straightforward — there's no `sources` JOIN-required surface, the source_id text is the identifier | `sqlite3 state/state.db "SELECT id FROM sources"` |
| `agent-mail` healthy; commits need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Parallel-safety note**: this handoff modifies `crates/daemon/src/main.rs` near `run_fts_live_loop` (~line 13786) and possibly the procurement-ingestion skill files. The Stage 11 hardening handoffs touch lines ~3821 and ~4497 — non-overlapping. Stage 14 Phase 2 touches `apps/desktop/`. Use `git worktree add ../testproject1-multisource main` for clean isolation. Merge to main with rebase at the end. **Do NOT touch Stage 11 / Stage 14 code paths** — drive the existing pipeline; if downstream stages break on the new data, that's a finding to document, not necessarily a fix.

---

## §1 — DECISIONS ALREADY MADE

### D1 — Drive against REAL data, but acknowledge the FTS asymmetry

**Verified (2026-04-30) source-path asymmetry in the codebase**:
- **procontract**: `run_procontract_discover_live_loop(runtime, cli)` — NO fixture. Live agent driving real ProContract. This is the path that populated 133 notices.
- **contracts-finder**: `run_contracts_finder_live_loop(runtime, cli)` — NO fixture. Cursor-driven from `source_cursors` table. Live ingestion path.
- **fts**: `run_fts_live_loop(runtime, fixture, cli)` — **REQUIRES `LiveLoopFixture`**, which carries `pty_transcript_lines: Vec<String>` and other replay fields. This path is **fixture-replay-driven today**, not real-API-driven. There is no "drive FTS against the live FTS API" path in the daemon as of 2026-04-30.

**Decision tree for the agent**:

1. **Easy wins** (pure drive-and-observe):
   - Drive `--ingest contracts-finder` → real cursor-based ingestion of recent notices. Validate Stages 1–13 cleanly.
   - Drive `--ingest procontract` again → adds new ProContract notices since the last sync. Validate no regressions.
2. **FTS path** (harder):
   - Either find / construct a `LiveLoopFixture` that carries realistic FTS transcript lines (not greenfield work — search for any existing FTS-related fixture in `crates/daemon/tests/` or similar), drive it, see what flows through.
   - OR document the FTS-live-API gap as a finding and **DO NOT add a real-API path in this handoff** (out of scope — that's a significant feature). File a follow-up bead.
3. **Cross-source dedup verification** can still be valid if contracts-finder + procontract overlap on `canonical_ocid` (some Contracts Finder notices syndicate from ProContract; check if the actual data has any matches).

**Bottom line**: do NOT block the handoff on getting live FTS working. The contracts-finder ingestion alone gets you a multi-source state.db; the dedup verification can be done with the data that exists. Document the FTS-live gap as a deferred bead.

### D2 — Scope the ingestion to a manageable batch

Don't pull every notice that's ever existed. Scope to:
- **FTS**: notices published in the last 30 days, OR a specific small subset (e.g., a known UK government framework category like construction or office supplies)
- **Contracts Finder**: similarly scoped

The goal is variety, not volume. **Aim for ~10–30 new notices per source**, enough to:
- Exercise Stage 3 dedup if any of them happen to be cross-published
- Add diversity (different buyers, different value bands, different categories)
- Not overwhelm the operator's manual decision queue

### D3 — Don't fix downstream regressions silently

If a new notice from FTS breaks Stage 8 (extract products), Stage 9 (search), Stage 11 (landed cost), or any later stage:
1. **Document the failure** with concrete reproduction (which notice, which stage, what error)
2. **Categorize the bug**: is it a data-shape variant the daemon doesn't handle? a skill prompt that needs to handle a new case? a real bug in stage logic?
3. **Fix only if scoped** (a quick-fix that's clearly correct)
4. **Otherwise file a follow-up bead** with the failure recorded; continue ingesting other notices that don't hit the bug
5. The validation manifest documents every such finding

### D4 — Stage 3 cross-source dedup must be verified

The current Relate stage uses `canonical_ocid` matching. UK FTS notices and Contracts Finder notices may share OCIDs for the same opportunity (the OCDS standard explicitly supports this). If you find any such pair in the ingested data:
- A `process_relations` row should exist with `relation_type='merged_same_process'`, `confidence_bps=10000`
- If no such pair exists in the actual ingested data → that's not a bug, just means none of the notices in this batch were cross-published

If cross-source pairs DO exist but `process_relations` rows DON'T appear → Stage 3 has a real bug. Fix it.

### D5 — No code changes for non-regression scenarios

If FTS + Contracts Finder ingestion produces new processes that flow cleanly through Stages 1–13 with no errors, this handoff requires **zero code changes** — just driving the pipeline + observing + writing the validation manifest. That's a successful outcome.

---

## §2 — IMPLEMENTATION ORDER

1. Verify pre-flight (§0).
2. Identify the real-data path for FTS ingestion (live API or fixture-from-real-data). Set up auth / rate-limit / fixture file as needed.
3. Drive `--ingest fts` once with a small batch (~10 notices). Capture log to `/tmp/multisource_fts.log`.
4. Inspect: how many new processes / notices created? Any errors?
5. Drive downstream stages: `--ingest manifest` (which transitively pulls Stage 12 + Stage 11 + Stage 13 if needed) — actually, the live loop should auto-cascade. Verify.
6. Validate: do the new FTS processes have manifests? What stoplights?
7. Repeat for Contracts Finder: identify real-data path, drive `--ingest contracts-finder`, validate.
8. Verify Stage 3: any new `process_relations` rows? If both FTS and Contracts Finder ingested, check for cross-source pairs.
9. Walk a sample of new processes through Stage 14 CLI: clarify, decide. Confirm the operator workflow handles non-ProContract data identically.
10. Document every failure / observation in the validation manifest.
11. Quality gates.
12. Write validation manifest.

---

## §3 — DEMO SEQUENCE

```bash
# Snapshot baseline counts
sqlite3 state/state.db "
SELECT 'baseline processes: ' || COUNT(*) FROM procurement_processes;
SELECT 'baseline notices by source: ' || source_id || '=' || COUNT(*) FROM contract_notices GROUP BY source_id;
SELECT 'baseline relations: ' || COUNT(*) FROM process_relations;
SELECT 'baseline pipeline_assessments: ' || COUNT(*) FROM pipeline_assessments;
SELECT 'baseline manifests: ' || COUNT(*) FROM manifests;
" | tee /tmp/multisource_baseline.log

# Drive Contracts Finder ingestion FIRST (this is the live-API path; no fixture needed)
cargo run -p daemon -- --ingest contracts-finder 2>&1 | tee /tmp/multisource_cf.log
# Inspect counts:
sqlite3 state/state.db "SELECT source_id, COUNT(*) FROM contract_notices GROUP BY source_id"

# Drive FTS ingestion — NOTE: --ingest fts REQUIRES --fixture-path AND is fixture-replay-driven
# (NOT a real-API path today — see D1). Skip if no realistic fixture available; document as deferred.
# If attempting:
#   - Read crates/daemon/src/main.rs:644 (LiveLoopFixture struct) to understand the fixture shape
#   - Look for any existing FTS fixture in crates/daemon/tests/ first
#   - DO NOT build a real-API FTS path in this handoff (out of scope)
# cargo run -p daemon -- --ingest fts --fixture-path <path/to/fts-fixture.json> 2>&1 | tee /tmp/multisource_fts.log

# Re-drive procontract for any new notices since last sync (no fixture; live)
cargo run -p daemon -- --ingest procontract 2>&1 | tee /tmp/multisource_procontract_resync.log

# Walk the deterministic pipeline forward MANUALLY — verified: each --ingest <X> runs ONLY its named stage (no auto-cascade).
# Stage 3 (Relate) is gated on process_stage_heads.stage='relate' for each new process; check whether the
# ingest-skill auto-runs Stage 3 or whether you need an explicit trigger (read run_fts_live_loop / run_contracts_finder_live_loop).
# Stages 4-9 are agent-driven (extract docs, lots, products, search). They run as enrichment skills triggered by Stage 1's pipeline scheduling.
# Stages 10-13 are deterministic and have explicit --ingest flags:
cargo run -p daemon -- --ingest landed-cost 2>&1 | tee /tmp/multisource_landed.log
cargo run -p daemon -- --ingest net-gate 2>&1 | tee /tmp/multisource_netgate.log
cargo run -p daemon -- --ingest manifest 2>&1 | tee /tmp/multisource_manifest.log
# If new processes don't progress through Stages 4-9 (extract → search), that's the agent-orchestration boundary —
# you may need to invoke the enrichment skills explicitly OR document as a finding (skill scheduling depends on
# pipeline_assessments rows, which depend on prior stages completing).

# Snapshot post-run counts
sqlite3 state/state.db "
SELECT 'post processes: ' || COUNT(*) FROM procurement_processes;
SELECT 'post notices by source: ' || source_id || '=' || COUNT(*) FROM contract_notices GROUP BY source_id;
SELECT 'post relations: ' || COUNT(*) FROM process_relations;
SELECT 'post pipeline_assessments: ' || COUNT(*) FROM pipeline_assessments;
SELECT 'post manifests: ' || COUNT(*) FROM manifests;
" | tee /tmp/multisource_post.log

# Inspect any cross-source process_relations
# (Stage 3 keys on procurement_processes.canonical_ocid — verified at main.rs:18381+)
sqlite3 state/state.db "
SELECT pr.relation_type, pr.confidence_bps,
       cn1.source_id AS source_a, cn2.source_id AS source_b,
       pp_a.canonical_ocid AS shared_canonical_ocid,
       cn1.title AS title_a
FROM process_relations pr
JOIN contract_notices cn1 ON cn1.process_id = pr.process_id_a
JOIN contract_notices cn2 ON cn2.process_id = pr.process_id_b
JOIN procurement_processes pp_a ON pp_a.id = pr.process_id_a
WHERE cn1.source_id != cn2.source_id
LIMIT 20;
"

# Walk Stage 14 on a sample new process (substitute a real FTS-ingested process_id)
NEW_PID=$(sqlite3 state/state.db "SELECT process_id FROM contract_notices WHERE source_id='fts' ORDER BY published_at DESC LIMIT 1")
cargo run -p daemon -- --list-decisions --process-id "$NEW_PID"
# (operator clarification + decision capture for at least one new process to prove end-to-end flow)
```

---

## §4 — REQUIRED TESTS

This handoff is mostly drive-and-observe; tests are minimal additions if any code changes were needed:

| # | Test | Verifies |
|---|---|---|
| MS.1 | (only if a regression was fixed) `<descriptive_name>` | Reproduction of the fixed bug |
| MS.2 | (only if Stage 3 dedup was fixed) `relate_stage_creates_relation_row_when_two_sources_share_canonical_ocid` | Stage 3 cross-source dedup |

If no code changes are needed (D5 case), no new tests are mandatory. **The validation manifest is the primary deliverable.**

---

## §5 — ACCEPTANCE CRITERIA — R.1 to R.13

| # | Criterion |
|---|---|
| R.1 | `--ingest contracts-finder` invocation completes; at least 1 new Contracts Finder notice persisted (source_id='contracts-finder'); summary in `/tmp/multisource_cf.log`. THIS IS THE PRIMARY VALIDATION TARGET (live-API path). |
| R.2 | EITHER `--ingest fts --fixture-path <fixture>` runs cleanly with a realistic fixture (transcript replay) AND at least 1 FTS notice persisted, OR the FTS-live-API gap is documented in the validation manifest as a deferred bead with reasoning (live FTS path doesn't exist in the daemon today; building it is out of scope). Either outcome is acceptable. |
| R.3 | New processes from FTS + Contracts Finder flow through Stages 1–7 (Normalize, Relate, Prequalify, Acquire docs, Extract lots, Qualify lot) without errors. Per-stage process_stage_status counts increase commensurate with the new data |
| R.4 | Stage 8 (extract products) processes the new lots; Stage 9 (score quoteability + price-signal match) at least attempts the new candidates (search_attempts may or may not produce quotes — that's source-dependent) |
| R.5 | Stage 10 (gross margin gate) and Stage 11 (landed cost) handle the new candidates without crashing. The `landed_cost_estimates` table grows IFF Stage 10 produces priced assessments for the new candidates (which depends on Stage 9 finding quotes). If Stage 9 finds zero quotes for the new data → Stage 10/11 produce zero new rows; that's expected, not a bug |
| R.6 | Stage 12 (net margin gate) processes the new universe. `candidate_net_margin_assessments` row count = old(17) + new(N) where N matches the count of new candidates with EITHER a pricing_assessment OR a search_attempt |
| R.7 | Stage 13 (manifest generation) produces manifests for the new processes. `manifests` row count = old(7) + new(M) where M matches the new pipeline_assessments count |
| R.8 | Stage 3 (Relate): if any cross-source `procurement_processes.canonical_ocid` matches exist in the ingested data, `process_relations` rows are created with `relation_type='merged_same_process'` and `confidence_bps=10000` (the only relation_type emitted today — schema CHECK also allows `'related_not_merged'` and `'possible_duplicate'` but those code paths are not implemented yet). If no matches exist, that's an acceptable observation (just means the batch had no cross-published notices) |
| R.9 | At least 1 sample new process is walked through Stage 14 CLI (clarify + decide); resulting `pending_operator_intents` row exists with the new process_id |
| R.10 | Existing 7 ProContract processes remain unchanged: 7 manifests still present, 7 pipeline_assessments still present, the DGU row still has `from_operator_override` qty=250 (Stage 14 demo state preserved), the 2 demo intents (1 superseded, 1 withdrawn) still in `pending_operator_intents` |
| R.11 | Every observed failure during ingestion is documented in the validation manifest with: which notice triggered it, which stage, error message / log excerpt, categorization (data-shape gap / skill prompt issue / real bug), and disposition (fixed-here / follow-up-bead-filed / acceptable-known-limitation) |
| R.12 | If any code changes WERE made during this handoff (D3 quick-fixes), they are minimal, scoped, and accompanied by tests. No drive-by refactors |
| R.13 | Quality gates: `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p daemon --bin daemon` (147 binary tests still pass + any new MS.* tests), per-file integration loop green across 39 files |

---

## §6 — VALIDATION QUERIES

```bash
sqlite3 -header -column state/state.db "
SELECT '=== R.1 / R.2 source breakdown ===';
SELECT source_id, COUNT(*) AS notice_count FROM contract_notices GROUP BY source_id ORDER BY source_id;

SELECT '=== R.3 stage tracking — counts grew commensurately ===';
SELECT stage, scope_kind, state, COUNT(*) FROM process_stage_status GROUP BY 1,2,3 ORDER BY 1,2;

SELECT '=== R.5 / R.6 / R.7 row counts post-ingestion ===';
SELECT 'landed_cost_estimates: ' || COUNT(*) FROM landed_cost_estimates;
SELECT 'candidate_net_margin_assessments: ' || COUNT(*) FROM candidate_net_margin_assessments;
SELECT 'pipeline_assessments: ' || COUNT(*) FROM pipeline_assessments;
SELECT 'manifests: ' || COUNT(*) FROM manifests;

SELECT '=== R.8 cross-source dedup ===';
SELECT pr.relation_type, COUNT(*) AS relation_count FROM process_relations pr GROUP BY pr.relation_type;
-- Stage 3 keys on procurement_processes.canonical_ocid (NOT contract_notices.ocid)
SELECT pr.process_id_a, pr.process_id_b,
       cn1.source_id AS source_a, cn2.source_id AS source_b,
       pp_a.canonical_ocid AS shared_canonical_ocid
FROM process_relations pr
JOIN contract_notices cn1 ON cn1.process_id = pr.process_id_a
JOIN contract_notices cn2 ON cn2.process_id = pr.process_id_b
JOIN procurement_processes pp_a ON pp_a.id = pr.process_id_a
WHERE cn1.source_id != cn2.source_id LIMIT 10;

SELECT '=== R.10 ProContract invariants intact ===';
SELECT COUNT(*) AS procontract_processes_unchanged FROM contract_notices WHERE source_id='procontract';
SELECT decision_grade, supplier_cost_gbp_pence, freight_base_gbp_pence
FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1';
SELECT idempotency_key, status FROM pending_operator_intents WHERE idempotency_key LIKE 'dec-2026-05-01-dgu-%';
SELECT product_candidate_id, json_extract(payload_json, '\$.qty')
FROM operator_input_overrides WHERE withdrawn_at IS NULL AND superseded_by_override_id IS NULL;

SELECT '=== R.9 Stage 14 walked on a new process ===';
SELECT process_id, desired_disposition, status, decision_note FROM pending_operator_intents
WHERE process_id NOT IN (SELECT process_id FROM contract_notices WHERE source_id='procontract')
ORDER BY queued_at DESC LIMIT 5;
"
```

---

## §7 — FAILURE-MODE PLAYBOOK

| Symptom | Likely cause | Disposition |
|---|---|---|
| `--ingest fts` exits with auth error | FTS API key not set OR fixture file missing | Set up auth / locate fixture per pre-flight; document the setup in the validation manifest's "Setup" section |
| FTS skill produces 0 notices | Date filter too narrow OR API rate-limited OR no notices match the query | Widen the date window; check the agent's stdout for the search query used; document the empty-set finding |
| Stage 8 fails on a new lot from FTS with "missing field" error | FTS data shape has a field the parser doesn't expect | Document the failing notice; if the fix is a one-liner (default the field), apply + test; otherwise file a follow-up bead and skip the notice |
| Stage 11 panics on a candidate with currency != GBP | Stage 11 may not handle non-GBP supplier quotes uniformly | Document; this is the "non-GBP FX path" caveat that's been deferred multiple times; do NOT fix here, file a bead |
| Cross-source duplicate exists by OCID match but no `process_relations` row created | Stage 3 hasn't run after the new ingestion (verified: `--ingest manifest` runs ONLY `run_manifest_generation_live_loop`, NOT a cascade) OR `procurement_processes.canonical_ocid` is NULL/empty for the new notices | Look at how the existing pipeline invokes Stage 3 — Stage 3 is gated on `process_stage_heads.stage='relate'` not having a row for the process. Either re-trigger by deleting from `process_stage_heads WHERE stage='relate'` for the new processes, OR find the right `--ingest <stage>` flag (`--ingest landed-cost`, `--ingest net-gate`, `--ingest manifest` are deterministic stages but only run their named stage). If `canonical_ocid` is NULL, that's a Stage 1 (normalize) bug — different domain |
| ProContract data changes (R.10 fails) | Some upstream stage re-processed existing rows because of a basis_hash drift | Determine which stage. If the change is benign (e.g., basis_hash recomputed but values identical), document. If values shifted, that's a real regression — investigate |
| Modified Stage 11 / Stage 14 code paths | Out of scope — those are separate hardening handoffs | Stop. Revert. This handoff drives existing skills + observes; bug fixes should be small + ingestion-related |

---

## §8 — CONSTRAINTS

1. **No new migrations.**
2. **No changes to Stage 11/Stage 12/Stage 13/Stage 14 code paths** beyond ingestion-related fixes (Stages 1–3 + skill prompts only).
3. **No deletions of existing data.** ProContract rows + 7 manifests + 2 demo intents + 1 active override must all survive.
4. **Document every observation**, even non-failures. The validation manifest is the audit trail of "what happens when we ingest from real FTS + Contracts Finder data".
5. **Worktree isolation** if running concurrently with other handoffs: `git worktree add ../testproject1-multisource main`. Merge to main with rebase at the end.
6. **No silent test exclusion.** Quality gate is mandatory.

---

## §9 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/multi-source-ingestion-validation-manifest.md`. Should include:

```markdown
# Multi-Source Ingestion Validation Manifest

> Generated: {ISO timestamp}
> Status: {GREEN | RED}

## Summary
{1-paragraph: how many notices ingested per source, how many cross-source dedup matches found, what broke + what was fixed, what stayed broken + filed as beads}

## Setup
{Auth / fixture / data-source URLs used. Reproducible recipe.}

## Counts before vs after

| Metric | Before | After | Delta |
|---|---|---|---|
| Total processes | 133 | ? | ? |
| FTS notices | 0 | ? | ? |
| Contracts Finder notices | 0 | ? | ? |
| ProContract notices | 133 | 133 | 0 (must be unchanged per R.10) |
| process_relations | 0 | ? | ? |
| pipeline_assessments | 7 | ? | ? |
| manifests | 7 | ? | ? |

## Stage-by-stage outcomes

| Stage | New processes processed | Failures | Disposition |
|---|---|---|---|

## Cross-source dedup findings

{Per R.8: any cross-source canonical_ocid matches? List them with relation_type + confidence_bps + the matching notices}

## Failures observed

| Notice / process_id | Stage | Error | Categorization | Disposition |
|---|---|---|---|---|

## Sample new-process Stage 14 walk-through

{Captured: clarification + decision for at least one new process. Show CLI output and resulting DB row}

## Code changes (if any)

| File | Change | Justification |
|---|---|---|

## Out-of-scope (filed as follow-up beads)

{Each: bead ID, title, why deferred}

## Quality Gates

- cargo fmt --check: PASS/FAIL
- cargo check --all-targets: PASS/FAIL
- cargo clippy --all-targets -- -D warnings: PASS/FAIL
- cargo test -p daemon --bin daemon: PASS/FAIL — N tests
- per-file integration loop: PASS/FAIL — 39 files
```

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
```

---

## §10 — REMINDERS

- ULTRATHINK and be meticulous.
- This handoff is **drive + observe + document**, not "build a feature". The most valuable output is the validation manifest's Failures Observed + Disposition table.
- A successful run with **zero code changes** is a positive outcome, not a sign of doing nothing. It means the L1-L3 pipeline is robust to multi-source variety.
- If you find yourself rewriting Stage 11 freight, Stage 14 CLI, or anything outside Stage 1–3 + ingestion skills — stop. Wrong handoff.
- Worktree for parallel safety. Rebase to main at the end.
- The DGU row's existing CDP_CONNECT_FAILED blockers are an environmental artifact (no CDP browser running in your env). The captcha-resilience handoff (running in parallel) addresses these. Do NOT try to fix them in this handoff.
