# Stages 1 → 9 Full-Pipeline Validation — Handoff

**For:** a fresh Claude Code agent session with no prior context of this codebase.
**Goal:** run the UK public procurement intelligence pipeline end-to-end, from portal ingestion through to supplier-price enrichment, and demonstrate that every stage produces the rows / artifacts it's designed to produce. Report a measurable "validated" state.
**Authored:** 2026-04-21 by the agent that implemented stages 1-9. Context sources linked inline.

---

## 1. Mission

You are validating that **all nine pipeline stages function end-to-end automatically** on this workspace. The stages, numbered per `AGENTS.md` §Architecture at a Glance:

1. **Ingest** — discover tender notices from a source portal (today: procontract)
2. **Normalize** — project raw source payload into canonical `contract_notices` shape
3. **Relate** — link duplicate / related processes via OCID / title similarity
4. **Qualify** — filter DPS / services / works / expired / services-by-title; mark `goods_qualified`
5. **Acquire Docs** — register interest on the portal, then enrich granted tenders (download attachments)
6. **Extract Lots** — create implicit / explicit lot rows per notice
7. **Extract Products** — generate `product_candidates` from tender-specification PDF seeds
8. **Score Quoteability** — compute `quoteability_score_bps` + `recall_risk_score_bps` per candidate
9. **Price-Signal Match** — project SearchProfile, dispatch per-source enrichment skill (alibaba / 1688 / ebay / amazon)

Stages 1 + 5 are driven by procurement-ingestion agent skills via PTY (Claude + `agent-browser`). Stages 2, 3, 4, 6, 8 live inside `run_deterministic_pipeline` (pure Rust, runs in <1s on 100-notice datasets). Stage 7 is inline during PDF semantic extraction. Stage 9 is dispatched via a dedicated `--enrich-pricing` CLI flag that spawns one PTY session per batch of eligible SearchProfiles.

**You are NOT validating stages 10-15** (gross margin → landed cost → net margin → manifest → operator decision → bid workflow). Those are stubbed. Stop at stage 9.

---

## 2. What's in place (inventory)

### Rust daemon (`crates/daemon/src/main.rs`)
- CLI flags: `--ingest <source>`, `--pipeline <source>`, `--enrich-pricing <source>`, `--enrich-pricing-limit <n>`, `--enrich-pricing-dry-run`
- Stage 1 dispatcher: `run_procontract_discover_live_loop` (line ~5600, via `--ingest procontract`)
- Stage 5 dispatchers:
  - `run_procontract_register_interest_live_loop` (~5720, via `--ingest procontract-interest`)
  - `run_procontract_enrich_tender_live_loop` (~5872, via `--ingest procontract-enrich`)
- Stages 2/3/4/6/8 in `run_deterministic_pipeline` (~9190, via `--pipeline procontract`): invokes `run_extract_lots_stage`, `run_prequalify_stage`, `run_qualify_lot_stage`, `run_build_search_profiles_stage`, `run_relate_stage`
- Stage 7 inline: `persist_tender_spec_domain_writes` (~7733), gated on `qualification_runs.goods_qualified = 1`
- Stage 9 dispatcher: `run_enrich_pricing` (~9246) + `enrichment_skill_path_for` (~9231), 4 source arms (alibaba / 1688 / ebay / amazon)

### Skills (`skills/`)
- Ingestion: `procurement-ingestion/procontract-discover`, `procurement-ingestion/procontract-register-interest`, `procurement-ingestion/procontract-enrich-tender`
- Document extraction: `pdf-extraction/tender-specification` (rich seed contract, mandates per-field slot criticality)
- Auth: `auth-refresh/alibaba`, `auth-refresh/1688` (both mandate CDP + persistent profile + `--headed`)
- Health: `source-health/alibaba` (mandates same attach protocol; canary only)
- Supplier enrichment: `supplier-price-enrichment/{alibaba,1688,ebay,amazon}` — all four skills consume the full SearchProfile JSON, walk 5 query waves, emit `supplier_quotes` via `result.json`

### Business policy applied
- DPS / services / works filtering at qualify stage via title heuristics
- Expiry filter at interest-registration **lowered 30 → 14 days** on 2026-04-21 (constant `INTEREST_REGISTRATION_MIN_DEADLINE_DAYS` in Rust; `minimumLeadMs = 14 * 86_400_000` in TypeScript `goods-qualification.ts`)

### Documentation cross-references
- `AGENTS.md` — project-wide agent rules (RULE 0: user is in charge; RULE 1: never delete files; Chrome 136+ CDP constraint)
- `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` — master architecture (≈8000 lines; §4.3 = source routing matrix, §5.4 = SearchProfile shape, §5.7 = price-signal match data flow, §8 = quoteability scoring)
- `docs/stage-9-supplier-price-enrichment-manifest.md` — delivery manifest for stage 9 (what was built, contracts, validation history, open gaps)

---

## 3. Current DB state (as of 2026-04-21 21:30 UTC, pre-handoff)

Query these before you start so you know what "new data" looks like after your run:

```bash
cd /Users/elias/Desktop/govt_scraping_project
sqlite3 state/state.db "
SELECT 'notices_total'       , COUNT(*) FROM contract_notices WHERE source_id='procontract'
UNION ALL SELECT 'interest_completed', COUNT(*) FROM process_stage_status WHERE stage='interest_registration' AND state='completed'
UNION ALL SELECT 'interest_skipped'  , COUNT(*) FROM process_stage_status WHERE stage='interest_registration' AND state='skipped'
UNION ALL SELECT 'enriched_tenders'  , COUNT(DISTINCT notice_id) FROM tender_document_sets
UNION ALL SELECT 'qualification_rows', COUNT(*) FROM qualification_runs WHERE qualification_basis='notice_only'
UNION ALL SELECT 'passed_qualify'    , COUNT(*) FROM qualification_runs WHERE qualification_basis='notice_only' AND goods_qualified=1
UNION ALL SELECT 'product_candidates', COUNT(*) FROM product_candidates
UNION ALL SELECT 'search_profiles'   , COUNT(*) FROM search_profiles
UNION ALL SELECT 'supplier_quotes'   , COUNT(*) FROM supplier_quotes;"
```

**Reference pre-handoff snapshot:**
- `notices_total = 103`, `interest_completed = 16`, `interest_skipped = 80`, `enriched_tenders = 12`, `qualification_rows = 103`, `passed_qualify = 9`, `product_candidates = 545` (all under rejected notices; see stage-9 manifest), `search_profiles = 1` (synthetic seeded stairlift), `supplier_quotes = 0`.

---

## 4. Pre-flight checks (do ALL before step 1)

```bash
cd /Users/elias/Desktop/govt_scraping_project

# 1. Claude CLI present + not rate-limited.
which claude && claude --version   # expect: 2.1.x or newer, path /Users/elias/.local/bin/claude

# 2. agent-browser present + Chrome present.
which agent-browser && agent-browser --version   # expect: 0.26.x
agent-browser doctor 2>&1 | grep -E "CLI version|Google Chrome|Launch test"   # all should be 'pass'

# 3. Daemon builds clean.
cargo check -p daemon 2>&1 | tail -5              # expect: "Finished dev profile"

# 4. NO orphan workflow_jobs (pending/running). Stale jobs will trip claim_next_job.
sqlite3 state/state.db "SELECT status, COUNT(*) FROM workflow_jobs WHERE status IN ('pending','running') GROUP BY status;"
# If any rows come back, run:
#   sqlite3 state/state.db "UPDATE workflow_jobs SET status='cancelled', completed_at=CURRENT_TIMESTAMP WHERE status IN ('pending','running');"

# 5. No stray agent-browser / Chrome.
agent-browser close --all 2>&1 | head -2
pgrep -fl "target/debug/daemon" 2>&1 | head -3   # expect: nothing
pgrep -fl "claude -p" 2>&1 | head -3             # expect: nothing

# 6. Current source cursors (where did last runs leave off).
sqlite3 state/state.db "SELECT source_id, cursor_key, substr(cursor_value,1,80) FROM source_cursors;"

# 7. Rate-limit awareness: Claude CLI enforces a daily/hourly budget.
# During the pre-handoff run (2026-04-21 17:45 UTC) the chain hit:
#   "You've hit your limit · resets 8pm (Europe/Zurich)"
# → every subsequent skill session exited immediately with no result.json,
#   and the daemon reported "did not produce a result contract" for each.
# Before running anything expensive, confirm your time relative to the
# most recent known reset, and sanity-check one cheap claude invocation:
echo "test" | timeout 20 claude -p "reply with OK only" 2>&1 | head -3
```

Proceed to §5 only when all 7 checks pass. A failed pre-flight check is cheaper to fix than a 30-min chain aborting mid-way.

---

## 5. Execution plan (run in order, do not parallelize)

Each step produces a PTY session under `artifacts/sessions/YYYY/MM/session-pty-<sha>/`. Capture the session id emitted by the daemon on each step — you'll need it for debugging.

### Step A — Discover (Stage 1): `--ingest procontract`

```bash
cd /Users/elias/Desktop/govt_scraping_project
agent-browser close --all 2>/dev/null
time cargo run --release --bin daemon -- --ingest procontract 2>&1 | tee /tmp/stage-01-discover.log
```

- **Duration:** 5–15 min depending on how many new notices since last cursor (`source_cursors.procontract/notice_cursor`).
- **Success stdout:** `procontract discovery completed: job_id=... notices_ingested=N continuation_required=false`.
- **DB effect:** new rows in `contract_notices` for `source_id='procontract'`; `source_cursors.procontract/notice_cursor` advanced; one row in `agent_session_logs` with `execution_mode='pty' status='completed' exit_code=0`; one row in `workflow_jobs` with `status='completed'`.
- **Validate:**
  ```bash
  sqlite3 state/state.db "SELECT COUNT(*) FROM contract_notices WHERE source_id='procontract';"   # should be > previous snapshot
  sqlite3 state/state.db "SELECT kind, status, substr(created_at,1,19) FROM workflow_jobs WHERE kind='sync_source' AND scope_id='procontract' ORDER BY created_at DESC LIMIT 3;"
  ```

### Step B — Register Interest (Stage 5a): `--ingest procontract-interest`

```bash
agent-browser close --all 2>/dev/null
time cargo run --release --bin daemon -- --ingest procontract-interest 2>&1 | tee /tmp/stage-05a-interest.log
```

- **Duration:** 5–15 min (click N portal "Register interest" buttons, N ≤ number of new qualifying notices).
- **Filter applied:** `candidate_deadline_check` requires `deadline > now + 14 days` (was 30, lowered 2026-04-21) AND `is_likely_goods_contract` on title / category / summary (keyword-based, permissive). Two heuristics, both in `crates/daemon/src/main.rs`.
- **Success stdout:** `procontract interest registration completed: job_id=... evaluations=N persisted=M`.
- **DB effect:** new rows in `process_stage_status` with `stage='interest_registration'`, `state IN ('completed','skipped','failed','already_registered')`.
- **Validate:**
  ```bash
  sqlite3 state/state.db "SELECT state, COUNT(*) FROM process_stage_status WHERE stage='interest_registration' GROUP BY state ORDER BY COUNT(*) DESC;"
  # Look for delta vs your pre-flight snapshot
  ```

### Step C — Enrich Tenders (Stage 5b): `--ingest procontract-enrich`

```bash
agent-browser close --all 2>/dev/null
time cargo run --release --bin daemon -- --ingest procontract-enrich 2>&1 | tee /tmp/stage-05b-enrich.log
```

- **Duration:** 5–20 min (one RFx page fetch + attachment download per activity the buyer has granted access to).
- **Success stdout:** `procontract enrichment completed: job_id=... enriched_tenders=N documents_acquired=M`.
- **DB effect:** new rows in `raw_documents`, `tender_document_sets` (`completeness_state='complete'`); updated `procurement_processes.latest_deadline_at / title / buyer_name` where the RFx page revised them; merged payload in `contract_notices.normalized_payload`.
- **Files on disk:** `artifacts/downloads/procontract/<notice_id>/<filename>` for each download.
- **Validate:**
  ```bash
  sqlite3 state/state.db "SELECT COUNT(DISTINCT notice_id) FROM tender_document_sets;"
  sqlite3 state/state.db "SELECT COUNT(*) FROM raw_documents WHERE source_id='procontract';"
  du -sh artifacts/downloads/procontract/ 2>/dev/null
  ```

### Step D — Deterministic Pipeline (Stages 2/3/4/6/8): `--pipeline procontract`

```bash
time cargo run --release --bin daemon -- --pipeline procontract 2>&1 | tee /tmp/stage-2-3-4-6-8-pipeline.log
```

- **Duration:** < 1s usually; < 30s even on 1000+ notices. Deterministic, no agent.
- **Success stdout:** `deterministic pipeline complete: lots_created=X prequalified=Y lots_qualified=Z search_profiles=W relations=V`.
- **DB effect:**
  - `notice_lots` — new rows for notices without explicit lots (one implicit lot each)
  - `qualification_runs` — new rows for notices that passed `Normalize` stage head; one `notice_only` row per notice + one `lot_level` row per lot
  - `search_profiles` — new rows for `product_candidates` belonging to qualify-passed notices, with 5-wave query ladder + quoteability + recall-risk + eligibility (auto_search / manual_only / skip)
  - Rationale format on qualify rows: `result=<pass|review|reject>; reason=<code>; access_route=<...>; legal_category=<goods|services|works|unknown|mixed>; cpv_div=<n|unknown>; goods_density=<bps>; expiry_qualified=<true|false>`
- **Validate:**
  ```bash
  # qualify-result histogram
  sqlite3 state/state.db "
  SELECT CASE
    WHEN rationale LIKE '%reason=dynamic_purchasing_system%' THEN 'DPS'
    WHEN rationale LIKE '%reason=services_category%' THEN 'services_cpv'
    WHEN rationale LIKE '%reason=services_title_heuristic%' THEN 'services_title'
    WHEN rationale LIKE '%reason=works_category%' THEN 'works_cpv'
    WHEN rationale LIKE '%reason=works_title_heuristic%' THEN 'works_title'
    WHEN rationale LIKE '%reason=expired%' THEN 'expired'
    WHEN rationale LIKE '%reason=-%' THEN 'passed/review'
    ELSE 'other' END AS bucket, COUNT(*)
  FROM qualification_runs WHERE qualification_basis='notice_only'
  GROUP BY bucket ORDER BY COUNT(*) DESC;"

  # search_profile eligibility distribution
  sqlite3 state/state.db "SELECT search_eligibility_state, COUNT(*), AVG(recall_risk_score_bps) FROM search_profiles GROUP BY search_eligibility_state;"
  ```

### Step E — Supplier Price Enrichment (Stage 9): `--enrich-pricing alibaba`

Alibaba is the first source implemented. 1688 / eBay / Amazon are registered in the router but their auth / canary cycles are not yet proven (see §8 open gaps in the stage-9 manifest).

```bash
# First validate routing + payload shape without spending tokens
cargo run --release --bin daemon -- --enrich-pricing alibaba --enrich-pricing-dry-run --enrich-pricing-limit 2 2>&1 | tee /tmp/stage-09-alibaba-dryrun.log
# Inspect the dry-run output — job_parameters should include full search_profile with head_noun, candidate_type, critical_attributes_json, numeric_specs_json, query_waves_json
grep -E "head_noun|critical_attributes|numeric_specs|query_waves" /tmp/stage-09-alibaba-dryrun.log | head -20

# Then the real run (spawns Claude + agent-browser, 5-10 min per profile)
agent-browser close --all 2>/dev/null
time cargo run --release --bin daemon -- --enrich-pricing alibaba --enrich-pricing-limit 2 2>&1 | tee /tmp/stage-09-alibaba.log
```

- **Duration:** 5–15 min per profile.
- **Expected outcomes per profile** (depending on Alibaba profile state):
  - **Green path** (operator has logged into `~/.local/share/govt-scraping/chrome-profiles/alibaba` and pinned shipping country to GB): `result.json` has populated `supplier_quotes[]` with `match_confidence_bps`, `quote_usability_confidence_bps`, `unit_price_gbp_pence`, etc.
  - **Red path** (never logged in): `result.json` has `supplier_quotes: []` and `extraction_metadata.failure_code = SOURCE_AUTH_STATE_INVALID`. **This is a valid outcome — the skill is designed to short-circuit rather than search anonymously.** It tells the operator: do the one-time login, then re-run.
- **DB effect today:** NONE in `supplier_quotes` — `run_bound_skill` doesn't have a `supplier-price-enrichment/*` persistence branch yet (this is open gap §8.1 in the stage-9 manifest). The `result.json` IS written correctly to the session workdir; promoting it into the authoritative table is follow-up work.
- **Validate:**
  ```bash
  LATEST=$(ls -td artifacts/sessions/2026/04/session-manual-* | head -1)
  echo "session=$LATEST"
  cat "$LATEST/workdir/result.json"
  # you're looking for: a non-null extraction_metadata.failure_code OR a populated supplier_quotes[] array
  ```

---

## 6. Known failure modes + what to do

Each of these was observed during the 2026-04-21 implementation & validation. If you hit one, the fix is deterministic.

### F1. `Error: claimed unexpected workflow job`

**Root cause:** `claim_next_job` ordered by created_at picked up a stale `pending`/`running` row from a previously-interrupted run (you killed it / it hit rate-limit / Chrome crashed without updating status).

**Fix:**
```bash
sqlite3 state/state.db "SELECT id, kind, scope_id, status, substr(created_at,1,19) FROM workflow_jobs WHERE status IN ('pending','running') ORDER BY created_at;"
sqlite3 state/state.db "UPDATE workflow_jobs SET status='cancelled', completed_at=CURRENT_TIMESTAMP WHERE status IN ('pending','running');"
```

Then re-run the step that failed.

### F2. `Error: <skill> did not produce a result contract`

**Root cause (in 90% of cases):** Claude CLI hit its API rate-limit and exited without writing `result.json` to the workdir.

**Diagnose:**
```bash
LATEST_PTY=$(ls -td artifacts/sessions/2026/04/session-pty-* | head -1)
ls "$LATEST_PTY/workdir/"   # if no result.json + no run-manifest.md → Claude aborted
for chunk in "$LATEST_PTY/stream/"*.zst; do zstd -d --stdout "$chunk" 2>/dev/null | tail -10; done
# Look for messages like "You've hit your limit · resets <time>"
```

**Fix:** wait for the reset window (typically 8pm Europe/Zurich daily for personal-tier accounts) and retry. You can do other non-Claude work in the meantime — the deterministic pipeline (`--pipeline`) does not consume Claude tokens.

**Other possible causes of same error:** Claude produced `result.json` in the wrong directory (check `storage.sqlite3` ≠ `result.json`), or the PDF extraction skill emitted an empty result (inspect `run-manifest.md`).

### F3. ProContract login fails / CAPTCHA

**Symptoms:** step A/B/C session takes ~1 min then exits non-zero; session stream shows login page or CAPTCHA.

**Current credentials** (per skill files, hardcoded in `skills/procurement-ingestion/procontract-*/SKILL.md`): `sales@sorellon.com` / `9928fJG3!5nk6nc`. **If these no longer work, the tenant has reset the password — contact ops for the new credentials and update the skill files.** Do NOT commit secrets to git without moving them to env vars first.

### F4. Alibaba session anonymous

**Symptoms:** step E returns `failure_code: SOURCE_AUTH_STATE_INVALID`; `extraction_metadata.observed_locale` will show `en-CH` or similar drift.

**Fix:** the one-time operator login workflow documented in `skills/auth-refresh/alibaba/SKILL.md`:
```bash
agent-browser --profile "$HOME/.local/share/govt-scraping/chrome-profiles/alibaba" \
  --headed open https://login.alibaba.com/
# user logs in via the visible Chrome window, completes CAPTCHA, pins shipping country to GB in My Alibaba
agent-browser close --all
```
Then rerun step E.

### F5. Chrome 136+ refuses CDP on the default profile

**Symptoms:** Chrome stderr prints `DevTools remote debugging requires a non-default data directory.`

**Root cause:** Chrome 136 (2025 security hardening) explicitly blocks `--remote-debugging-port` on its default `user-data-dir`. Every source skill in this repo pins a dedicated profile path under `~/.local/share/govt-scraping/chrome-profiles/<source>/`. If you see this error you've regressed the skill to use `--profile Default` or `--auto-connect` — both of which are explicitly forbidden in every supplier-price-enrichment and auth-refresh SKILL.md.

### F6. Workdir contains `storage.sqlite3*` files

**Not a bug.** Claude CLI writes its own prompt-cache DB into its CWD. Cosmetic, non-load-bearing. The run-manifest.md + result.json you care about are sibling files.

---

## 7. Success criteria — what "validated" means

You've validated stages 1-9 when ALL of the following are true in the DB after your run:

- [ ] `contract_notices` for `source_id='procontract'` increased by ≥ 0 rows (0 allowed if no new tenders since last cursor)
- [ ] `workflow_jobs` has ≥ 3 new rows with `kind='sync_source'`, all with `status='completed'`
- [ ] `process_stage_status` has ≥ 0 new `interest_registration` rows (0 allowed if nothing qualified; report the evaluation summary)
- [ ] `tender_document_sets` has ≥ 0 new `completeness_state='complete'` rows (0 allowed if no buyer has granted access yet)
- [ ] After `--pipeline`:
  - `qualification_runs` has coverage — every qualify-eligible notice has at least one `notice_only` row
  - `search_profiles` is consistent — every qualify-passed notice with ≥1 product_candidate has ≥1 search_profile
- [ ] Stage 9 `--enrich-pricing alibaba` run completes exit 0 AND writes a schema-conformant `result.json` to the session workdir. The json may have `supplier_quotes: []` + a populated `failure_code` (that's a valid short-circuit); what it MUST NOT have is the file absent entirely.
- [ ] `agent_session_logs` has new rows with `status='completed'` and non-null `run_manifest_path` for each PTY session you triggered

None of these require a specific COUNT — what matters is that each stage produces the right SHAPE of output. A day with zero new qualifying tenders is a valid validated day; the filter just told you so.

---

## 8. Reporting template — how to write this up when done

Post-run, produce a single markdown block with this structure (so the operator can diff across days):

```markdown
# Stages 1-9 Validation Run — <YYYY-MM-DD HH:MM UTC>

## Pre-flight
- claude version: <x.y.z>
- agent-browser version: <x.y.z>
- daemon cargo check: green / <error>
- orphan workflow_jobs cleared: <n rows cancelled>
- claude rate-limit test: <OK / limited-until X>

## Stage-by-stage

| # | Stage | Command | Exit | Walltime | Rows added | Notes |
|---|---|---|---|---|---|---|
| 1 | Ingest (procontract-discover) | `--ingest procontract` | 0 / err | Xm Ys | +N notices | |
| 5a | Register interest | `--ingest procontract-interest` | 0 / err | Xm Ys | +N interest rows (M completed, K skipped) | |
| 5b | Enrich tender | `--ingest procontract-enrich` | 0 / err | Xm Ys | +N tender_document_sets, +M raw_documents | |
| 2-4/6/8 | Deterministic pipeline | `--pipeline procontract` | 0 / err | Xs | +L lots, +Q qual runs, +P search_profiles | |
| 9 | Enrich alibaba | `--enrich-pricing alibaba` | 0 / err | Xm Ys | session_id=<sha> failure_code=<code or null> | |

## DB delta

<before vs after counts — paste the output of the query in §3>

## Artifacts created

- session-pty-<sha1> @ artifacts/sessions/2026/MM/... (discover)
- session-pty-<sha2> @ ... (interest)
- session-pty-<sha3> @ ... (enrich)
- session-manual-<sha4> @ ... (alibaba)

## Issues encountered + resolution

<F-codes from §6 hit + how you resolved them; or "none">

## Open gaps surfaced during validation (if any)

<anything new you found that the stage-9 manifest §8 didn't already call out>

## Verdict

[GREEN | YELLOW | RED] — one-line reason.
```

---

## 9. What this handoff is NOT

- **Not** an implementation task. All nine stages are written and `cargo check -p daemon` is clean. You are validating behaviour, not writing new code.
- **Not** a stage-10-15 task. Those stages are stubs (`candidate_pricing_assessments`, `landed_cost_estimates`, `bid_workflow_states` tables exist but no runners). Respect that boundary.
- **Not** an occasion to refactor or reorganize. If you find a gap, add it to the stage-9 manifest's §8 open-gaps list and flag it in your §8 reporting — do not attempt to close it in the same run.
- **Not** a manifest persistence task. Stage 9's `result.json` does NOT get promoted into `supplier_quotes` yet (open gap §8.1 in the stage-9 manifest). Writing a `persist_supplier_quotes_from_agent_output` function is a separate ~30 LOC follow-up not in scope here.

---

## 10. Appendix — file index

```
crates/daemon/src/main.rs
  line  354       CLI flag --skill-path
  line  396-409   CLI flags --enrich-pricing, --enrich-pricing-limit, --enrich-pricing-dry-run
  line 3015       const INTEREST_REGISTRATION_MIN_DEADLINE_DAYS (14)
  line 3017       fn candidate_deadline_check
  line ~5600      run_procontract_discover_live_loop
  line ~5720      run_procontract_register_interest_live_loop
  line ~5872      run_procontract_enrich_tender_live_loop
  line 9190       run_deterministic_pipeline
  line 9231       enrichment_skill_path_for (alibaba/1688/ebay/amazon router)
  line 9246       run_enrich_pricing (Stage 9 dispatcher)
  line 9448       supplier_sources_for_candidate_type (routing matrix per research §4.3; advisory, not yet wired)
  line 9496       run_prequalify_stage (Stage 4 — DPS/services/works title heuristics)
  line 9729       run_qualify_lot_stage (Stage 4 per-lot inheritance)
  line 9843       run_relate_stage
  line 10137+     stage 8.5 helpers — build_query_waves_json, compute_quoteability_score_bps, compute_recall_risk_bps
  line 10428      run_build_search_profiles_stage
  line 7733       persist_tender_spec_domain_writes (Stage 7 inline, gated on qualify)

packages/intelligence/src/qualification/goods-qualification.ts
  line  201       evaluateExpiry (DPS exemption + 14-day minimum)
  line  221-226   minimumLeadMs = 14 * 24 * 60 * 60 * 1000

skills/procurement-ingestion/
  procontract-discover/          (Stage 1)
  procontract-register-interest/ (Stage 5a)
  procontract-enrich-tender/     (Stage 5b)
  contracts-finder/              (alt Stage 1 — OCDS API; less exercised)

skills/pdf-extraction/tender-specification/
  SKILL.md                       (Stage 7 extraction spec — "product seed specificity" section is the contract)
  result.schema.json             (rich product_requirement_seeds definitions)

skills/auth-refresh/
  alibaba/SKILL.md               (CDP + persistent profile — canonical attach protocol)
  1688/SKILL.md                  (CDP mirror, Chinese auth markers)

skills/source-health/alibaba/SKILL.md   (Stage 1/health canary; mandates same attach protocol)

skills/supplier-price-enrichment/
  alibaba/                       (Stage 9 — canonical)
  1688/                          (Stage 9 — Chinese wholesale sibling)
  ebay/                          (Stage 9 — UK retail ceiling)
  amazon/                        (Stage 9 — UK retail, most strict anti-bot)

docs/research/2026-03-08-public-procurement-intelligence-platform-research.md
  §4.3  source routing matrix (lines ~900-1000)
  §5.4  SearchProfile shape + lexicon (lines ~3300)
  §5.7  price-signal matching data flow (lines ~5200)
  §8    quoteability scoring (lines ~2860)

docs/stage-9-supplier-price-enrichment-manifest.md   (dedicated delivery manifest for stage 9 — read this FIRST before running §5)
```

End of handoff.
