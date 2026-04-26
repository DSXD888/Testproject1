# Validate Stage 7 (Extract Products) + Build & Validate Stage 8 (Score Quoteability)

ULTRATHINK and be meticulous.

## FRESH-EYES CURRENT-STATE OVERRIDE (2026-04-26)

This handoff was originally written before Stage 8 was implemented. **Do not rebuild Stage 8 from scratch if the current code still contains `run_score_quoteability_stage()` and the pipeline wiring.** The current code has `run_reclassify_candidates_stage()` and `run_score_quoteability_stage()` in `crates/daemon/src/main.rs`, and `run_deterministic_pipeline()` calls quoteability before gross margin.

Current read-only DB snapshot:

```text
contract_notices=133
notice_lots=179
product_candidates=820
search_profiles=820
search_eligibility_state: eligible=414, marginal=325, ineligible=81
```

Use the body of this handoff as implementation context and validation logic, but the current mission is now: **validate the existing Stage 7/8 outputs against the current DB, fix regressions only if checks fail, and update the manifest rather than duplicating already-landed implementation work.**

## MISSION

Validate and maintain the **Stage 7 → Stage 8 product-candidate/quoteability path** against the current live SQLite state. Stage 7 product candidates and Stage 8 search profiles already exist; your job is to verify data quality, confirm `run_score_quoteability_stage()` still matches the documented scoring contract, and fix only real regressions. If a fresh checkout somehow lacks Stage 8, use the implementation guidance below to rebuild it; otherwise, do not duplicate already-landed work.

After the validation loop is green, the pipeline should have current, scored, searchable product candidates with `search_profiles` ready for Stage 9 marketplace search.

```
┌─────────────────────────────────────────────────────────┐
│                    THE LOOP                               │
│                                                          │
│   1. Validate Stage 7 product candidates                 │
│   2. Fix any Stage 7 data quality issues                 │
│   3. Build Stage 8 (Score Quoteability)                  │
│   4. Run Stage 8 on real data                            │
│   5. Validate Stage 8 output                             │
│   6. Any failures?                                       │
│      ├─ YES → Fix, go back to appropriate step           │
│      └─ NO  → Stages 7+8 GREEN. Write manifest.          │
│                                                          │
│   DO NOT STOP UNTIL STEP 6 = NO                          │
└─────────────────────────────────────────────────────────┘
```

---

## CONTEXT: What Feeds These Stages

```
Branch A (GREEN):
  10 contracts passed as viable physical goods
  63 product_candidates in those 10 contracts (4 contracts have candidates)
  141 total lots, 103 notices qualified

This handoff:
  Stage 7:  Validate 545 existing product_candidates (63 in passed-goods)
  Stage 8:  BUILD + validate Score Quoteability → search_profiles table

Downstream (NOT this handoff):
  Stage 9:  Price-Signal Match (marketplace search skill — needs separate research)
```

---

## KEY CONCEPTS

**Read AGENTS.md** — single source of truth for agent instructions.

**Stage tracking** — see `process_stage_status` and `process_stage_heads` tables. The `persist_stage()` helper writes both atomically. Stages are idempotent — if a head exists, re-running skips that entity.

**The quoteability funnel:** Stage 8 takes product candidates and decides which ones are searchable on a marketplace. It produces a `search_profiles` row for EVERY candidate but marks them with `search_eligibility_state`:
- `eligible` — auto-search with full query waves (score >= 0.65)
- `marginal` — search only top-value candidates (score 0.45-0.64)
- `ineligible` — skip marketplace search, RFQ-only (score < 0.45)

---

## PHASE 1: VALIDATE STAGE 7 (Product Candidates)

### 1A. Research — Understand the existing data

The 545 product_candidates were created by the semantic extraction pipeline (Stage 5c). The `pdf-extraction/tender-specification` skill extracted lot structure and product requirement seeds, and the daemon's `persist_*_domain_writes()` functions translated those seeds into `product_candidates` rows.

Run this assessment:

```bash
sqlite3 state/state.db "
SELECT '=== TOTAL CANDIDATES ===';
SELECT COUNT(*) FROM product_candidates;
SELECT '';
SELECT '=== IN PASSED-GOODS CONTRACTS (the ones that matter) ===';
SELECT COUNT(*) FROM product_candidates pc
  JOIN qualification_runs qr ON qr.process_id = pc.process_id
  WHERE qr.qualification_basis = 'notice_only' AND qr.goods_qualified = 1 AND qr.expiry_qualified = 1;
SELECT '';
SELECT '=== FIELD COMPLETENESS (all 545) ===';
SELECT
  COUNT(*) as total,
  SUM(CASE WHEN head_noun IS NOT NULL AND head_noun != '' THEN 1 ELSE 0 END) as has_head_noun,
  SUM(CASE WHEN name IS NOT NULL AND name != '' THEN 1 ELSE 0 END) as has_name,
  SUM(CASE WHEN candidate_type IS NOT NULL THEN 1 ELSE 0 END) as has_type,
  SUM(CASE WHEN match_archetype IS NOT NULL THEN 1 ELSE 0 END) as has_archetype,
  SUM(CASE WHEN specification_text IS NOT NULL AND specification_text != '' THEN 1 ELSE 0 END) as has_spec,
  SUM(CASE WHEN quantity IS NOT NULL THEN 1 ELSE 0 END) as has_quantity,
  SUM(CASE WHEN material IS NOT NULL AND material != '' THEN 1 ELSE 0 END) as has_material,
  SUM(CASE WHEN standards_json IS NOT NULL AND standards_json NOT IN ('', '[]', 'null') THEN 1 ELSE 0 END) as has_standards,
  SUM(CASE WHEN critical_attributes_json IS NOT NULL AND critical_attributes_json NOT IN ('', '[]', 'null') THEN 1 ELSE 0 END) as has_critical_attrs,
  SUM(CASE WHEN numeric_specs_json IS NOT NULL AND numeric_specs_json NOT IN ('', '[]', 'null') THEN 1 ELSE 0 END) as has_numeric_specs,
  SUM(CASE WHEN quoteability_score_bps IS NOT NULL THEN 1 ELSE 0 END) as has_quoteability,
  SUM(CASE WHEN identity_confidence_bps IS NOT NULL THEN 1 ELSE 0 END) as has_identity_conf
FROM product_candidates;
SELECT '';
SELECT '=== SAMPLE CANDIDATES (passed-goods, first 15) ===';
SELECT pc.id, pc.name, pc.head_noun, pc.candidate_type, pc.match_archetype,
       pc.quantity, pc.quantity_unit, pc.material,
       SUBSTR(pc.specification_text, 1, 80) as spec_preview
FROM product_candidates pc
JOIN qualification_runs qr ON qr.process_id = pc.process_id
WHERE qr.qualification_basis = 'notice_only' AND qr.goods_qualified = 1 AND qr.expiry_qualified = 1
LIMIT 15;
SELECT '';
SELECT '=== CANDIDATES PER PASSED-GOODS CONTRACT ===';
SELECT pc.process_id, cn.title, COUNT(*) as candidates
FROM product_candidates pc
JOIN contract_notices cn ON cn.process_id = pc.process_id
JOIN qualification_runs qr ON qr.process_id = pc.process_id
WHERE qr.qualification_basis = 'notice_only' AND qr.goods_qualified = 1 AND qr.expiry_qualified = 1
GROUP BY pc.process_id ORDER BY candidates DESC;
"
```

### 1B. Known Data Quality Issues

From prior investigation, the 545 candidates have these characteristics:
- **ALL 545** have: `name`, `head_noun`, `candidate_type`, `match_archetype`, `specification_text`
- **335/545** have `standards_json` populated
- **0/545** have: `quoteability_score_bps`, `identity_confidence_bps`, `recall_risk_score_bps` — **these are Stage 8's job to compute**
- **0/545** have: `critical_attributes_json`, `numeric_specs_json`, `material` — structured extraction gaps
- **1/545** has `quantity` populated — almost entirely missing
- `candidate_type` is uniformly `"sku"` and `match_archetype` is uniformly `"direct_match"` across all 545 — these appear to be defaults, not properly classified

### 1C. Stage 7 Validation Checks

**Check 7.1:** Every passed-goods contract has at least 1 product candidate
```bash
sqlite3 state/state.db "
SELECT 'Passed-goods contracts WITHOUT candidates: ' || COUNT(*) FROM (
  SELECT DISTINCT qr.process_id FROM qualification_runs qr
  WHERE qr.qualification_basis = 'notice_only' AND qr.goods_qualified = 1 AND qr.expiry_qualified = 1
) passed
WHERE NOT EXISTS (SELECT 1 FROM product_candidates pc WHERE pc.process_id = passed.process_id);
"
```
**PASS condition:** 0 is ideal. If > 0, document WHICH contracts have no candidates and WHY (no tender documents enriched, no semantic extraction ran). This is a coverage gap, not a blocking failure — Stage 8 can still score the candidates that DO exist. Record the gap in the manifest.

**Check 7.2:** Every candidate has required fields for Stage 8
```bash
sqlite3 state/state.db "
SELECT 'Candidates missing name:       ' || COUNT(*) FROM product_candidates WHERE name IS NULL OR name = '';
SELECT 'Candidates missing head_noun:  ' || COUNT(*) FROM product_candidates WHERE head_noun IS NULL OR head_noun = '';
SELECT 'Candidates missing type:       ' || COUNT(*) FROM product_candidates WHERE candidate_type IS NULL;
SELECT 'Candidates missing archetype:  ' || COUNT(*) FROM product_candidates WHERE match_archetype IS NULL;
SELECT 'Candidates missing spec_text:  ' || COUNT(*) FROM product_candidates WHERE specification_text IS NULL OR specification_text = '';
"
```
**PASS condition:** All counts = 0 for the 63 passed-goods candidates.

**Check 7.3:** FK integrity
```bash
sqlite3 state/state.db "
SELECT 'Orphaned candidates (lot missing):    ' || COUNT(*) FROM product_candidates pc
  LEFT JOIN notice_lots nl ON nl.id = pc.lot_id WHERE nl.id IS NULL;
SELECT 'Orphaned candidates (notice missing):  ' || COUNT(*) FROM product_candidates pc
  LEFT JOIN contract_notices cn ON cn.id = pc.notice_id WHERE cn.id IS NULL;
SELECT 'Orphaned candidates (process missing): ' || COUNT(*) FROM product_candidates pc
  LEFT JOIN procurement_processes pp ON pp.id = pc.process_id WHERE pp.id IS NULL;
"
```
**PASS condition:** All 0.

### 1D. Stage 7 Assessment Decision

After the checks, determine:
- **If the 63 passed-goods candidates have name + head_noun + specification_text:** they're usable for Stage 8 scoring. The missing fields (quoteability, identity_confidence, recall_risk) are Stage 8's job. The missing structured fields (critical_attributes, numeric_specs, material) reduce scoring precision but don't block it.
- **If passed-goods contracts have zero candidates:** document which ones and why (no tender docs enriched). These will have no searchable products — that's a coverage gap, not a bug.

---

## PHASE 2: BUILD STAGE 8 (Score Quoteability)

### 2A. Research — What Stage 8 Must Do

Stage 8 is a **deterministic code stage** (no LLM, no agent skill). It reads product candidates, scores their searchability, and generates `search_profiles` with query wave plans.

**Read these files:**

1. **`crates/daemon/src/main.rs`** — search for `run_deterministic_pipeline` to see where Stage 8 fits. Currently Stage 8 is ONLY in the fixture materialization path (`persist_pipeline_materialization`), not in the real pipeline. You need to add a real `run_score_quoteability_stage()` function.

2. **`run_prequalify_stage()`** — YOUR REFERENCE PATTERN. Search for it in main.rs and study the structure. Every deterministic stage function follows this exact skeleton:

```rust
fn run_STAGE_NAME_stage(state_db: &mut StateDb) -> Result<usize> {
    // 1. QUERY: find unprocessed entities (WHERE NOT EXISTS stage head)
    let rows: Vec<(...)> = {
        let conn = state_db.connection();
        let mut stmt = conn.prepare("SELECT ... WHERE NOT EXISTS (SELECT 1 FROM process_stage_heads ...)")?;
        stmt.query_map(...)?.collect::<Result<Vec<_>, _>>()?
    };
    if rows.is_empty() { return Ok(0); }

    // 2. COMPUTE + WRITE in a single transaction
    let stage_run_id = allocate_stage_run_id();
    let (count, _) = state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        let mut processed = 0_usize;
        for row in &rows {
            // ... compute scoring / classification ...
            // ... INSERT into output table (ON CONFLICT DO ... for idempotency) ...
            // ... persist_stage(tx, process_id, "Stage name", scope_kind, scope_id, &stage_run_id) ...
            processed += 1;
        }
        Ok(processed)
    })?;
    Ok(count)
}
```

Key helpers used by all stage functions:
- `allocate_stage_run_id()` — generates a unique UUID-based run ID
- `next_commit_seq(tx)` — increments the commit clock for ordering
- `persist_stage(tx, process_id, stage, scope_kind, scope_id, &stage_run_id)` — writes `process_stage_status` + advances `process_stage_heads` atomically

You must be able to answer before coding:
1. How does a stage function find entities that haven't been processed yet? (Answer: `NOT EXISTS` on `process_stage_heads`)
2. What does `allocate_stage_run_id()` return? (Answer: a unique string that groups all writes from one execution batch)
3. What happens if `persist_stage()` is called for an entity that already has a head? (Answer: `ON CONFLICT DO UPDATE` — advances to the new run)

**The `search_profiles` table schema** (the output table you'll write to):
```sql
CREATE TABLE search_profiles (
  id TEXT PRIMARY KEY,
  product_candidate_id TEXT NOT NULL REFERENCES product_candidates(id),
  head_noun TEXT NOT NULL,
  head_noun_family TEXT,
  candidate_type TEXT NOT NULL,
  match_archetype TEXT NOT NULL,
  critical_attributes_json TEXT,
  standards_json TEXT,
  numeric_specs_json TEXT,
  noun_aliases_en TEXT,
  noun_aliases_zh TEXT,
  market_frame_en TEXT,
  market_frame_zh TEXT,
  use_context TEXT,
  sector_context TEXT,
  required_anchors TEXT,
  optional_anchors TEXT,
  must_not_have TEXT,
  preferred_terms TEXT,
  excluded_terms TEXT,
  query_waves_json TEXT NOT NULL,
  recall_risk_score_bps INTEGER,
  search_eligibility_state TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**The `product_candidates` table** is the input. Key fields for scoring: `head_noun`, `name`, `candidate_type`, `match_archetype`, `specification_text`, `standards_json`, `quantity`, `quantity_unit`, `critical_attributes_json`, `numeric_specs_json`.

### 2B. What to Build

Add `run_score_quoteability_stage()` to `crates/daemon/src/main.rs`. Follow the pattern of `run_prequalify_stage`:

**Input query:** Find product candidates whose lot has passed qualification (`qualify lot` stage head exists) but the candidate does NOT have a `Score quoteability` stage head.

**Scoring logic (deterministic rules):**

```
score = 0.0

// Head noun quality (is it a real commodity noun?)
if head_noun length >= 3 and not a generic word:  +0.20
if head_noun is a known commodity family:          +0.10  (bonus)

// Specification completeness
if specification_text is present and length > 20:  +0.15
if critical_attributes_json has entries:           +0.10
if numeric_specs_json has entries:                 +0.10
if standards_json has entries:                     +0.05

// Quantity clarity
if quantity is not null and > 0:                   +0.10
if quantity_unit is present:                       +0.05

// Product type suitability
if candidate_type in (sku, standard_commodity):    +0.15
if candidate_type in (brand_family):               +0.10
if candidate_type in (bundle, custom):             +0.05

// Total: max ~1.0 (10000 bps)
quoteability_score_bps = (score * 10000) as integer
```

**Search eligibility classification:**
```
if quoteability_score_bps >= 6500:  search_eligibility_state = "eligible"
if quoteability_score_bps >= 4500:  search_eligibility_state = "marginal"
else:                                search_eligibility_state = "ineligible"
```

**IMPORTANT — scoring reality check with current data:**
Given that 0/545 candidates have `quantity`, `critical_attributes_json`, `numeric_specs_json`, or `material`, the maximum achievable score with the above logic is:
```
head_noun >= 3 chars:  +0.20
spec_text > 20 chars:  +0.15
standards_json exists: +0.05
candidate_type = sku:  +0.15
TOTAL MAX:              0.55 → 5500 bps → "marginal"
```
With these thresholds, ZERO candidates would be "eligible" (all would be marginal or ineligible). You MUST adjust either the scoring weights or the eligibility thresholds so the distribution is meaningful. Options:
- Lower the "eligible" threshold to ~5000 bps
- Increase the weight of `specification_text` quality (length, keyword richness)
- Add a signal for `head_noun` specificity (multi-word nouns like "double glazed units" score higher than single-word like "print")
- Use your judgment — the goal is that well-specified physical products (timber, glass, metal) score meaningfully higher than vague items

**Recall risk scoring:**
```
recall_risk_score_bps = 10000 - quoteability_score_bps
// Higher risk = harder to find matches
```

**Query waves generation:**
For each candidate, generate a `query_waves_json` array with progressively broader search terms. Build the query text from the candidate's actual fields. Example for a candidate with `name="Supply of Double Glazed Units"`, `head_noun="double glazed units"`, `specification_text="4mm toughened sealed units, various sizes"`:

```json
[
  {"wave": 1, "strategy": "exact_noun", "query": "double glazed units 4mm toughened sealed"},
  {"wave": 2, "strategy": "noun_with_spec", "query": "double glazed units toughened glass"},
  {"wave": 3, "strategy": "broader_noun", "query": "sealed glazing units"},
  {"wave": 4, "strategy": "existence_check", "query": "double glazed units"}
]
```

Build each wave from available data:
- Wave 1: `head_noun` + first meaningful terms from `specification_text` (skip filler words)
- Wave 2: `head_noun` + any material or standard references
- Wave 3: broader phrasing (remove adjectives, use simpler noun)
- Wave 4: just `head_noun` alone (widest net)

If `specification_text` is short or empty, waves 1-3 may collapse to similar text. That's fine — the agent skill in Stage 9 will decide which waves to actually execute.

**Database writes per candidate:**

1. **UPDATE `product_candidates`** — set `quoteability_score_bps`, `recall_risk_score_bps`, `identity_confidence_bps` (based on name clarity and spec completeness)

2. **INSERT `search_profiles`** — one row per candidate with:
   - `id`: `"sp-{product_candidate_id}"`
   - Copy `head_noun`, `candidate_type`, `match_archetype` from candidate
   - Copy `standards_json`, `critical_attributes_json`, `numeric_specs_json` from candidate
   - Generate `query_waves_json` based on available data
   - Set `search_eligibility_state` based on score threshold

3. **`persist_stage()`** — record `"Score quoteability"` with scope_kind `"candidate"`

**Wire into the pipeline:**

Add the call to `run_deterministic_pipeline()` after `run_qualify_lot_stage()` and before `run_relate_stage()`:
```rust
let quoteability_scored = run_score_quoteability_stage(state_db)?;
::tracing::info!(quoteability_scored, "score-quoteability stage complete");
```

### 2C. Implementation Constraints

- **Rust edition 2024**, Clippy pedantic+nursery, unsafe forbidden
- Follow the exact pattern of `run_prequalify_stage` — query → write transaction → persist_stage → return count
- The function should be global (no source_id filter) — score ALL candidates whose lots are qualified
- Only score candidates in lots that have `qualify lot` stage heads
- Only score candidates that DON'T already have `Score quoteability` stage heads (idempotent)
- `cargo check --all-targets && cargo clippy --all-targets -- -D warnings` must pass

---

## PHASE 3: RUN & VALIDATE STAGE 8

### Execution

```bash
cargo build --release --bin daemon
cargo run --release --bin daemon -- --pipeline procontract \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

This re-runs the full deterministic pipeline. **Stages 6, 4a, 4b, and 3 will be skipped automatically** — they already have stage heads, and each function's `NOT EXISTS (... process_stage_heads ...)` query returns zero rows. Only Stage 8 (your new code) will process candidates. This is by design — all stages are idempotent.

### Check 8.1: Search profiles created

```bash
sqlite3 state/state.db "
SELECT 'Total search_profiles: ' || COUNT(*) FROM search_profiles;
SELECT 'Candidates in passed-goods with profiles: ' || COUNT(*) FROM search_profiles sp
  JOIN product_candidates pc ON pc.id = sp.product_candidate_id
  JOIN qualification_runs qr ON qr.process_id = pc.process_id
  WHERE qr.qualification_basis = 'notice_only' AND qr.goods_qualified = 1 AND qr.expiry_qualified = 1;
"
```
**PASS condition:** search_profiles count > 0. Should match the number of candidates scored.

### Check 8.2: Quoteability scores populated on candidates

```bash
sqlite3 state/state.db "
SELECT 'Candidates with quoteability score: ' || COUNT(*) FROM product_candidates WHERE quoteability_score_bps IS NOT NULL;
SELECT 'Candidates with recall_risk score:  ' || COUNT(*) FROM product_candidates WHERE recall_risk_score_bps IS NOT NULL;
"
```
**PASS condition:** Matches the number of candidates processed by Stage 8.

### Check 8.3: Search eligibility distribution

```bash
sqlite3 state/state.db "
SELECT search_eligibility_state, COUNT(*) FROM search_profiles GROUP BY search_eligibility_state;
"
```
**PASS condition:** At least 2 states present. Given the data quality (missing structured attributes, quantities), expect many `marginal` or `ineligible`. Some `eligible` candidates should exist for well-specified products.

### Check 8.4: Query waves exist

```bash
sqlite3 state/state.db "
SELECT 'Profiles with query_waves: ' || COUNT(*) FROM search_profiles WHERE query_waves_json IS NOT NULL AND query_waves_json != '[]';
SELECT 'Profiles without waves:    ' || COUNT(*) FROM search_profiles WHERE query_waves_json IS NULL OR query_waves_json = '[]';
"
```
**PASS condition:** Profiles without waves = 0. Every profile must have at least 1 query wave.

### Check 8.5: Stage heads exist

```bash
sqlite3 state/state.db "
SELECT stage, scope_kind, COUNT(*) FROM process_stage_heads
  WHERE stage = 'Score quoteability'
  GROUP BY stage, scope_kind;
SELECT '';
SELECT stage, state, COUNT(*) FROM process_stage_status
  WHERE stage = 'Score quoteability'
  GROUP BY stage, state;
"
```
**PASS condition:** Stage heads exist with scope_kind = `candidate`. State = `succeeded`.

### Check 8.6: FK integrity

```bash
sqlite3 state/state.db "
SELECT 'Orphaned search_profiles (candidate missing): ' || COUNT(*) FROM search_profiles sp
  LEFT JOIN product_candidates pc ON pc.id = sp.product_candidate_id WHERE pc.id IS NULL;
"
```
**PASS condition:** 0.

### Check 8.7: Eligibility breakdown for passed-goods candidates

```bash
sqlite3 state/state.db "
SELECT '=== PASSED-GOODS ELIGIBILITY ===';
SELECT sp.search_eligibility_state, COUNT(*), AVG(pc.quoteability_score_bps) as avg_score
FROM search_profiles sp
JOIN product_candidates pc ON pc.id = sp.product_candidate_id
JOIN qualification_runs qr ON qr.process_id = pc.process_id
WHERE qr.qualification_basis = 'notice_only' AND qr.goods_qualified = 1 AND qr.expiry_qualified = 1
GROUP BY sp.search_eligibility_state;
"
```
**PASS condition:** Results make sense — physical goods candidates (timber, glass, metal work) should score higher than generic items.

---

## PHASE 4: FIX LOOP

If any check fails:
1. Diagnose root cause (code bug, data issue, scoring logic too strict/lenient)
2. Fix the issue
3. To re-run Stage 8:
   ```bash
   sqlite3 state/state.db "
   DELETE FROM process_stage_heads WHERE stage = 'Score quoteability';
   DELETE FROM process_stage_status WHERE stage = 'Score quoteability';
   DELETE FROM search_profiles;
   UPDATE product_candidates SET quoteability_score_bps = NULL, recall_risk_score_bps = NULL, identity_confidence_bps = NULL WHERE quoteability_score_bps IS NOT NULL;
   "
   cargo run --release --bin daemon -- --pipeline procontract \
     --state-root state --runtime-dir runtime --artifact-root artifacts
   ```
4. Re-run ALL validation checks
5. Repeat until green

---

## DELIVERABLES

1. **Stage 7 validation passes** — existing product candidates are usable
2. **Stage 8 implementation** — `run_score_quoteability_stage()` function in main.rs
3. **Stage 8 wired into `run_deterministic_pipeline()`**
4. **All validation checks pass** (7.1-7.3 + 8.1-8.7)
5. **`cargo check --all-targets && cargo clippy --all-targets -- -D warnings`** clean
6. **Any fixes documented** in the manifest

### MANDATORY: Write Results Manifest

After ALL checks pass, write to `docs/pipeline/stages-7-8-validation-manifest.md`:

```markdown
# Stages 7-8 Validation Manifest

> Generated: {timestamp}
> Status: {GREEN / YELLOW / RED}

## Stage 7: Product Candidates Assessment

| Metric | Value |
|--------|-------|
| Total product_candidates | X |
| In passed-goods contracts | X |
| Passed-goods contracts with candidates | X of 10 |
| Passed-goods contracts WITHOUT candidates | X (list them) |
| Field completeness: name | X% |
| Field completeness: specification_text | X% |
| Field completeness: quantity | X% |

## Stage 8: Score Quoteability Results

| Metric | Value |
|--------|-------|
| Total search_profiles created | X |
| Candidates scored (quoteability_score_bps set) | X |

### Eligibility Distribution (all candidates)
| State | Count | Avg Score |
|-------|-------|-----------|
| eligible | X | X |
| marginal | X | X |
| ineligible | X | X |

### Eligibility Distribution (passed-goods only)
| State | Count | Avg Score |
|-------|-------|-----------|
| eligible | X | X |
| marginal | X | X |
| ineligible | X | X |

### Key Metric
**Searchable candidates (eligible + marginal): X of 63 passed-goods candidates (X%)**

## Validation Checks

| # | Check | Result | Detail |
|---|-------|--------|--------|
| 7.1 | Passed-goods have candidates | PASS/FAIL | X |
| 7.2 | Required fields present | PASS/FAIL | X |
| 7.3 | FK integrity (candidates) | PASS/FAIL | X |
| 8.1 | Search profiles created | PASS/FAIL | X |
| 8.2 | Quoteability scores populated | PASS/FAIL | X |
| 8.3 | Eligibility distribution sensible | PASS/FAIL | X |
| 8.4 | Query waves exist | PASS/FAIL | X |
| 8.5 | Stage heads exist | PASS/FAIL | X |
| 8.6 | FK integrity (profiles) | PASS/FAIL | X |
| 8.7 | Passed-goods eligibility sensible | PASS/FAIL | X |

## Code Changes

| File | Change |
|---|---|
| `crates/daemon/src/main.rs` | Added `run_score_quoteability_stage()`, wired into `run_deterministic_pipeline()` |

## Issues Found & Fixed

{For each: what failed, root cause, fix, verification}

## Issues Remaining

{Any gaps, or "None"}

## Data Snapshot

{Full output of all validation queries}
```

---

## CONSTRAINTS

1. **Work in a loop until green.** Do NOT report "done" with failing checks.
2. **Stage 8 is deterministic code only.** No LLM, no agent skill, no browser automation. Pure Rust computation.
3. **Follow existing patterns.** Study `run_prequalify_stage()` and `run_qualify_lot_stage()` — your function should look structurally identical.
4. **Score ALL candidates** whose lots have passed qualification, not just the 63 in passed-goods contracts. The scoring should run on all 545 candidates globally (some may become relevant if qualification criteria change later).
5. **Rust conventions:** `cargo check --all-targets && cargo clippy --all-targets -- -D warnings` must pass.
6. **Do NOT build Stage 9.** Stage 8 ends at `search_profiles` + scored candidates. Marketplace search is a separate handoff requiring its own research.

---

## REMINDERS

- ULTRATHINK and be meticulous.
- **The loop is non-negotiable.** Run → validate → fix → repeat until green.
- The scoring logic above is a starting point. Adjust thresholds if the results are degenerate (all eligible or all ineligible). The goal is a reasonable distribution that reflects actual data quality differences between candidates.
- **Known data quirk:** ALL 545 candidates have `candidate_type = "sku"` and `match_archetype = "direct_match"` — these appear to be defaults from the semantic extraction, not proper classifications. The "product type suitability" scoring rule will give +0.15 to everyone. Consider reducing the weight of this signal or keying the bonus off `specification_text` quality instead.
- Many candidates have weak structured data (no quantity, no material, no critical_attributes). The scoring should reflect this — candidates with only a name and spec text should score lower than candidates with full structured attributes. The DIFFERENTIATING signals are: `specification_text` length/quality, `standards_json` presence, and `head_noun` specificity.
- The `query_waves_json` field is critical for Stage 9. Each wave must be a concrete search strategy with actual query text, not placeholders.
- The stage name in `persist_stage()` must be exactly `"Score quoteability"` (capital S, lowercase q) to match the existing fixture pattern.
