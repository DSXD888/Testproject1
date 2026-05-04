# Full Pipeline E2E Commercial Basis + Alibaba Coverage Handoff

ULTRATHINK and be meticulous.

## MISSION

Fix the cross-stage commercial-basis defects surfaced by the Woodhouses full-pipeline E2E run: Stage 8 missed explicit boiler quantities, Stage 9 stopped after a single UK-side eBay quote without durable Alibaba/China-side attempts, Stage 10 treated a one-sided quote as enough for normal gross-margin math, and Stage 11 used an inconsistent quantity/revenue basis that invented `42.39` boilers. The correct primary path is two-sided supplier evidence: at least one UK-side quote signal and at least one China/import-side quote signal must be attempted and persisted before normal benchmarking and margin calculations proceed. The existing 30 percent retail-to-wholesale estimate is allowed only as a clearly labelled fallback/screening path after required source attempts are recorded.

Out of scope: Layer 4 shared visibility, Layer 5 advanced commercial templates, Layer 6 request automation, Stage 15 bid workflow, and any real-money commitment.

---

## OPERATOR INTENT DISTILLED

The product should first answer a supplier-spread question, not just a tender-value question:

```text
Can a China/import-side supplier quote, after landed-cost adjustments, beat a UK-side quote enough to make the opportunity commercially attractive?
```

UK-side eBay/Amazon quotes are valid benchmark evidence. They are not "weak" by default. They become insufficient only when they are the only side of evidence and the China/import side was not attempted or did not yield durable results.

If this handoff is too large for one implementation session, split it into beads in this order:

1. Stage 9 durable Alibaba/import coverage gate.
2. Stage 8 explicit quantity extraction/persistence.
3. Stage 10 two-sided commercial-basis model.
4. Stage 11 quantity/revenue basis alignment.
5. Stage 12/13 reason and manifest surfaces.

Do not close the parent work until all five invariants are satisfied.

---

## PHASE 1: DEEP RESEARCH (mandatory before implementation)

Do not write code until this research phase is complete. This is a cross-stage fix, so a local patch in only Stage 10 is not sufficient.

### 1A. Woodhouses E2E Evidence - understand the exact failure

Read and query:

- `docs/pipeline/full-pipeline-e2e-validation-manifest.md` - official run summary and stage outcomes.
- `state-e2e-20260504T113157Z/state.db` - isolated E2E database; inspect with read-only/immutable SQLite.
- `/tmp/e2e_woodhouses_alibaba_job.json` - prepared Alibaba marketplace input payload, not proof of a completed search.
- `/tmp/e2e_woodhouses_gem_job.json` - prepared GEM marketplace input payload, not proof of a completed search.
- `/tmp/e2e_woodhouses_mx_job.json` - prepared MX marketplace input payload, not proof of a completed search.
- `/tmp/e2e_woodhouses_ebay_job.json` - prepared eBay marketplace input payload.
- `/tmp/e2e_stage9_ebay_woodhouses_manual.log` - eBay run evidence; persisted one GBP quote.
- `/tmp/e2e_stage9_amazon_woodhouses_manual.log` - Amazon run evidence; persisted zero quotes due locale/currency drift.

Run:

```bash
sqlite3 -header -column "file:state-e2e-20260504T113157Z/state.db?mode=ro&immutable=1" "
SELECT cn.id, cn.title, cn.buyer_name, cn.estimated_value_original_minor,
       cn.estimated_value_gbp_ex_vat_pence, cn.deadline_at
FROM contract_notices cn
WHERE cn.process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545';

SELECT nl.id, nl.lot_key, nl.title, nl.estimated_value_pence
FROM notice_lots nl
WHERE nl.notice_id IN (
  SELECT id FROM contract_notices
  WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545'
);

SELECT id, name, candidate_type, head_noun, quote(quantity) AS quantity_raw,
       quantity, quantity_unit, candidate_value_share_bps,
       identity_confidence_bps, spec_confidence_bps,
       quantity_confidence_bps, quoteability_score_bps,
       source_evidence_json
FROM product_candidates
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545';

SELECT source, COUNT(*) AS attempts, SUM(result_count) AS results,
       SUM(shortlisted_count) AS shortlisted,
       GROUP_CONCAT(DISTINCT search_failure_mode) AS failure_modes
FROM search_attempts
WHERE product_candidate_id IN (
  SELECT id FROM product_candidates
  WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545'
)
GROUP BY source;

SELECT source_name, COUNT(*) AS quotes
FROM supplier_quotes
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545'
GROUP BY source_name;
"
```

Expected current evidence:

```text
Notice value: GBP 130,000
Extracted product candidates:
  B01 Ideal Evomax 2 80kW wall-hung gas boiler, quantity NULL
  B02 Ideal Evomax 2 60kW wall-hung gas boiler, quantity NULL

Search attempts:
  amazon-uk: 4 attempts, 0 quotes, locale_or_currency_drift
  ebay-uk:   7 attempts, 1 quote
  alibaba-search: 0 persisted attempts
  gem-imports:    0 persisted attempts
  mx-wholesale:   0 persisted attempts

Supplier quotes:
  ebay-uk: 1 quote
```

Prepared Alibaba job payload excerpt:

```json
{
  "source_id": "alibaba-search",
  "source": "alibaba-search",
  "max_candidates": 2,
  "candidates": [
    {
      "product_candidate_id": "pc-...boiler-b01-ideal-evomax2-80kw",
      "name": "Ideal Evomax 2 80kW Wall-Hung Gas Boiler (B01)",
      "quantity": null,
      "quantity_unit": null
    },
    {
      "product_candidate_id": "pc-...boiler-b02-ideal-evomax2-60kw",
      "name": "Ideal Evomax 2 60kW Wall-Hung Gas Boiler (B02)",
      "quantity": null,
      "quantity_unit": null
    }
  ]
}
```

This excerpt proves two things:

1. The daemon or validation harness had enough candidate/search-profile data to prepare an Alibaba run.
2. The prepared input still had `quantity: null`, so the quantity extraction defect flowed into supplier search as well as margin math.

You must be able to answer:

1. Why is `/tmp/e2e_woodhouses_alibaba_job.json` not durable Stage 9 evidence?
2. Which persisted tables prove that Alibaba/GEM/MX were not actually completed?
3. Which DB fields show that B01/B02 quantity was missed?
4. Why did Stage 10 allocate GBP 65,000 to B02?
5. Why did Stage 11 infer `42.39` boilers?

### 1B. Tender extraction path - quantity should have been captured

Read:

- `skills/pdf-extraction/tender-specification/SKILL.md` - current semantic extraction prompt explicitly says not to do BoQ quantity extraction, but its schema includes `quantity_hint`.
- `skills/pdf-extraction/tender-specification/result.schema.json` - `product_requirement_seeds[].quantity_hint` exists and should be used for explicit schedule/spec quantity signals.
- `crates/daemon/src/main.rs` - Stage 8 semantic extraction persistence from `product_requirement_seeds` into `notice_lots` and `product_candidates`.
- `migrations/0006_pdf_extraction_toolkit.sql` - document extraction tables.

Known source evidence from the Woodhouses documents:

```text
Boiler schedule:
  B01: Ideal Evomax 2, Output 80kW
  B02: Ideal Evomax 2, Output 60kW

Large mechanical specification:
  "The Contractor shall supply and install 2no. gas fired heating boilers."
  "The Contractor shall supply and install 2no. wall-mounted gas fired heating boilers..."
  "The 2no. new boilers shall be complete with 1no. common flue system..."
  "The controls system is to be adapted to include for 2no new boilers..."
```

Current persisted candidate state:

```text
product_candidates.quantity = NULL
product_candidates.quantity_unit = NULL
product_candidates.quantity_confidence_bps = 9200
```

Important nuance from the existing semantic extraction:

```text
document_semantic_extractions.warnings_json includes:
  QUANTITY_INFERRED_FROM_SCHEDULE_REFERENCE
  "Schedule lists each boiler (B01, B02) as a single referenced unit;
   quantity=1 per reference is inferred from the one-row-per-reference
   convention rather than an explicit quantity field. Total project
   boiler count = 2."
```

So the semantic skill already noticed a quantity signal, but daemon-side persistence did not materialize it into `product_candidates.quantity`. The fix should decide whether that warning should become an explicit structured quantity field, a parsed `quantity_hint`, or a lower-confidence review prompt. It must not leave `quantity_confidence_bps` high while `quantity` remains `NULL` without an explanatory basis.

Important current skill text:

```markdown
## Non-goals
- Parsing layout or table structure that should already be present in Docling output.
- Emitting separate semantic rows for `"candidates"` or `"requirements"` target domains.
- Performing pricing normalization or BoQ quantity extraction.
- Guessing lot boundaries, standards, or product requirements without citing evidence anchors.
```

Important current schema field:

```json
"quantity_hint": {
  "type": "string"
}
```

You must be able to answer:

1. Where does the daemon convert `quantity_hint` into `product_candidates.quantity` today, if anywhere?
2. Does Stage 8 distinguish explicit tender quantities from BoQ pricing quantities?
3. How should `2no` total boilers be represented when B01 and B02 are separate schedule rows?
4. How should the system behave if the document says total quantity but candidate-level allocation is ambiguous?

### 1C. Marketplace source roles - required quote coverage

Read:

- `crates/daemon/src/sources/registry.rs` - source definitions and `price_signal_role`.
- `crates/daemon/src/main.rs` - Stage 9 live loop and marketplace result persistence.
- `skills/marketplace-search/amazon-uk/SKILL.md`
- `skills/marketplace-search/ebay-uk/SKILL.md`
- `skills/marketplace-search/alibaba/SKILL.md`
- `skills/marketplace-search/gem-imports/SKILL.md`
- `skills/marketplace-search/mx-wholesale/SKILL.md`

Current source roles embedded in `crates/daemon/src/sources/registry.rs`:

```rust
"gem-imports" => {
    "price_signal_role": "uk_wholesale",
    "region": "GB",
    "quote_class": "wholesale",
}

"mx-wholesale" => {
    "price_signal_role": "uk_wholesale",
    "region": "GB",
    "quote_class": "wholesale",
}

"amazon-uk" => {
    "price_signal_role": "uk_retail_benchmark",
    "region": "GB",
    "quote_class": "spot",
}

"ebay-uk" => {
    "price_signal_role": "uk_retail_benchmark",
    "region": "GB",
    "quote_class": "spot",
}

"alibaba-search" => {
    "price_signal_role": "cn_supplier_floor",
    "region": "CN",
    "quote_class": "trade",
}
```

Current Stage 9 persistence contract:

```rust
fn persist_marketplace_search_results(
    state_db: &mut StateDb,
    source_id: &str,
    result_json: &Value,
) -> Result<(usize, usize)>
```

Current Stage 9 live-loop entry point:

```rust
async fn run_marketplace_search_live_loop(
    runtime: &mut RuntimeBootstrap,
    source_id: &str,
    cli: &Cli,
) -> Result<LiveLoopReport>
```

Current important behavior:

```text
run_marketplace_search_live_loop(source_id) only runs the one requested source.
It writes a workflow_jobs row and, if the skill produces result.json, persists
search_attempts and supplier_quotes.

The E2E harness stopped after eBay produced at least one GBP quote, so
alibaba-search/gem-imports/mx-wholesale were never persisted.
```

You must be able to answer:

1. How does the daemon decide which candidates need a source-specific search?
2. What durable state proves a marketplace source was attempted?
3. What durable state proves a marketplace source failed?
4. What durable state proves a marketplace quote role is available?
5. Where should the system enforce "do not advance to normal Stage 10 basis until required source roles were attempted"?

### 1D. Stage 10 gross margin gate - current one-sided math

Read:

- `crates/daemon/src/main.rs` around `run_gross_margin_gate_stage`.
- `crates/daemon/src/main.rs` around `effective_quote_cost_pence`.
- `crates/daemon/src/main.rs` around `allocate_gross_margin_revenue`.
- `migrations/0001_sqlite_state_initial.sql` and later migrations that define or alter `candidate_pricing_assessments`.

Current key signatures:

```rust
fn effective_quote_cost_pence(quote: &GrossMarginQuoteRow) -> i64

fn quote_identity_gate_passed(quote: &GrossMarginQuoteRow) -> bool

fn quote_usability_gate_passed(quote: &GrossMarginQuoteRow) -> bool

fn allocate_gross_margin_revenue(
    tx: &Transaction<'_>,
    candidate: &GrossMarginCandidateRow,
) -> Result<RevenueAllocation>

fn run_gross_margin_gate_stage(state_db: &mut StateDb) -> Result<usize>
```

Current one-sided formula:

```rust
fn effective_quote_cost_pence(quote: &GrossMarginQuoteRow) -> i64 {
    if quote.price_signal_role == "uk_retail_benchmark" {
        quote.unit_price_gbp_pence * (10_000 - DEFAULT_UK_RETAIL_WHOLESALE_DISCOUNT_BPS) / 10_000
    } else {
        quote.unit_price_gbp_pence
    }
}

let revenue = allocate_gross_margin_revenue(tx, candidate)?;
let effective_cost = effective_quote_cost_pence(selected_quote);
let gross_margin_bps = if revenue.revenue_pence == 0 {
    0
} else {
    (revenue.revenue_pence - effective_cost) * 10_000 / revenue.revenue_pence
};
```

Current Woodhouses Stage 10 result:

```text
Selected quote: eBay UK quote for B02
Raw price: GBP 3,066.70
price_signal_role: uk_retail_benchmark
Wholesale discount: 30 percent
Effective cost used by Stage 10: GBP 2,146.69
Revenue basis: expected_value
Expected addressable revenue: GBP 65,000
Gross margin: 9669 bps
identity_gate_passed: 1
usability_gate_passed: 0
pricing_input_confidence_bps: 6000
```

You must be able to answer:

1. Why is this one-sided calculation not enough for normal commercial readiness?
2. How should UK quote and China/import quote be compared before falling back to estimated wholesale discount?
3. Which table should record the basis kind: two-sided comparison vs fallback single-side estimate?
4. How should Stage 10 behave when Alibaba was prepared but not durably attempted?

### 1E. Stage 11 landed cost and Stage 12 net gate - basis alignment

Read:

- `crates/daemon/src/main.rs` around `infer_order_quantity_basis`.
- `crates/daemon/src/main.rs` around `compute_supplier_cost_basis`.
- `crates/daemon/src/main.rs` around `build_domestic_landed_cost_estimate`.
- `crates/daemon/src/main.rs` around `evaluate_net_margin_decision`.
- `migrations/0007_net_margin_gate_assessments.sql`.

Current key signatures:

```rust
fn infer_order_quantity_basis(row: &LandedCostAssessmentRow) -> OrderQuantityBasis

fn compute_supplier_cost_basis(row: &LandedCostAssessmentRow) -> SupplierCostBasis

fn build_domestic_landed_cost_estimate(
    row: &LandedCostAssessmentRow,
    supplier_cost: &SupplierCostBasis,
    stage_run_id: &str,
    order_quantity_basis: &OrderQuantityBasis,
    skill_versions: &Value,
) -> Result<LandedCostEstimateWrite>

const fn net_margin_bps(revenue_pence: i64, landed_cost_pence: i64) -> i64

fn evaluate_net_margin_decision(input: &NetMarginDecisionInput<'_>) -> NetMarginDecision
```

Current Stage 11 quantity inference ladder includes:

```rust
// Priority 3a: candidate-level quantity (pc.quantity).
if let Some(qty) = positive_finite_quantity(row.candidate_quantity) {
    return OrderQuantityBasis::FromLotQuantity {
        qty,
        basis_unit: row.candidate_quantity_unit.clone().or(preferred_unit),
    };
}

// Priority 3b: derive from notice_lots.estimated_value_pence when plausible.
if let Some(qty) = lot_value_derived_quantity(row) {
    return OrderQuantityBasis::FromLotQuantity {
        qty,
        basis_unit: row.candidate_quantity_unit.clone().or(preferred_unit),
    };
}
```

Current Woodhouses Stage 11 result:

```text
Stage 10 revenue for B02: GBP 65,000
Raw eBay unit quote: GBP 3,066.70
Candidate quantity: NULL
Lot value: GBP 130,000
Stage 11 derived quantity: GBP 130,000 / GBP 3,066.70 = 42.3908435777872
Stage 11 supplier cost: 42.3908435777872 * GBP 3,066.70 = GBP 130,000
Stage 12 net margin: (GBP 65,000 - GBP 130,000) / GBP 65,000 = -10000 bps
```

This is not a document-derived quantity. It is an unsafe fallback caused by missing quantity and inconsistent revenue/lot basis.

You must be able to answer:

1. Why did Stage 11 use whole lot value after Stage 10 had allocated half the lot value to B02?
2. What should Stage 11 do when candidate quantity is missing and Stage 10 used allocated candidate revenue?
3. How should an extracted `2no` total boiler signal flow into Stage 11?
4. When should Stage 12 review/block instead of reject/pass because basis is not aligned?

---

## PHASE 2: ANALYSIS & DESIGN

The receiving session should design this as a commercial-basis state-machine improvement across Stages 8-12.

### Problem 1: Prepared Alibaba job files were not durable Stage 9 evidence

**Observed**: `/tmp/e2e_woodhouses_alibaba_job.json`, `/tmp/e2e_woodhouses_gem_job.json`, and `/tmp/e2e_woodhouses_mx_job.json` existed. They included source IDs, candidate IDs, search profile IDs, and query waves for the Woodhouses B01/B02 candidates. But the DB had zero `search_attempts` and zero `supplier_quotes` for those sources.

**Meaning**: A temp job payload is only input preparation. It does not mean the skill ran, completed, failed, or produced no results. Durable evidence is only in the DB: `workflow_jobs`, `agent_session_logs`, `search_attempts`, `supplier_quotes`, and `process_stage_status`.

**Correct behavior**:

- If a required source role is prepared, it must either be run or recorded as skipped/failed with a durable reason.
- No acceptance check may count a `/tmp/*.json` job payload as source coverage.
- Stage 9 should have a required-source-role coverage concept for each candidate or candidate set.

### Problem 2: Stage 9 must attempt Alibaba/China-side search before normal margin readiness

**Primary policy**:

For a priceable physical product candidate, the normal path requires two-sided quote coverage:

```text
UK-side quote role:
  uk_retail_benchmark from amazon-uk or ebay-uk
  OR uk_wholesale from gem-imports or mx-wholesale

China/import-side quote role:
  cn_supplier_floor from alibaba-search
  OR a future configured CN/import source with equivalent role
```

The system may still run more sources. But it must not treat one UK-side quote as enough for normal commercial basis.

**Failure policy**:

If Alibaba is blocked by CAPTCHA, auth, no results, timeout, or source layout drift, persist a durable attempted state:

```text
search_attempts.source = 'alibaba-search'
search_failure_mode = 'SOURCE_CAPTCHA_BLOCKED' | 'AGENT_AUTH_CHALLENGE' |
                      'no_results_all_waves' | 'SOURCE_LAYOUT_DRIFT' |
                      'AGENT_RUN_FAILED' | ...
```

Then Stage 10 may use fallback logic, but it must label that fallback explicitly.

### Problem 3: Stage 10 should compare quotes first, not start from tender value

The corrected primary math should be quote-spread first:

```text
Given:
  q = extracted candidate quantity
  p_uk = UK-side quote unit price or normalized total
  p_cn = China/import supplier unit price or normalized total
  landed_cn = p_cn plus freight/customs/FX/final-mile as applicable

Primary spread:
  uk_benchmark_total = p_uk * q
  cn_landed_total = landed_cn
  supplier_spread_bps = (uk_benchmark_total - cn_landed_total) * 10000 / uk_benchmark_total

Tender viability:
  tender_revenue_total = candidate-level tender revenue allocation if available
  tender_margin_bps = (tender_revenue_total - selected_supply_total) * 10000 / tender_revenue_total
```

The quote-spread basis answers the operator's first commercial question:

```text
Can one supplier side, especially China/import, produce a margin versus the UK-side market quote?
```

Tender expected value may still be used as a revenue cap or opportunity sizing signal, but it should not be the first and only basis for gross margin when two supplier-side quotes are available.

### Problem 4: Single quote plus 30 percent discount is fallback, not normal basis

The current default 30 percent retail-to-wholesale discount is still useful when the system has no import/China quote after real attempts. But it must be labelled and surfaced as fallback/screening:

```text
commercial_basis_kind = single_side_retail_discount_fallback
normal_quote_coverage = false
required_source_roles_attempted = true
missing_quote_role = cn_supplier_floor
fallback_discount_bps = 3000
```

Do not mark this as normal gross-margin readiness. Do not hide it behind a high `gross_margin_bps`.

### Problem 5: Quantity extraction and propagation must be fixed

Woodhouses proves that explicit quantity signals can live outside the schedule row:

```text
Schedule identifies B01 and B02.
Spec prose says 2no gas-fired boilers.
Commercial inference: total boiler quantity = 2, likely 1 x B01 and 1 x B02.
```

The fix needs a careful representation:

```text
If schedule rows identify two unique boiler references and spec says 2no boilers:
  B01.quantity = 1
  B01.quantity_unit = 'unit'
  B02.quantity = 1
  B02.quantity_unit = 'unit'
  source evidence cites both schedule row and "2no" spec prose

If total quantity exists but allocation is ambiguous:
  do not invent per-candidate quantities
  persist a quantity-basis warning/clarification prompt
  block normal Stage 11 quantity inference
```

### Problem 6: Stage 10 and Stage 11 must use the same basis

Stage 10 used candidate revenue `GBP 65,000`. Stage 11 used whole lot value `GBP 130,000` to infer quantity. That should not be possible in normal mode.

Required invariant:

```text
If Stage 10 selected/allocated candidate-level revenue, Stage 11 may not use whole-lot value to infer candidate order quantity unless it can prove the candidate covers the whole lot.
```

For Woodhouses, the right quantity basis should be document-derived:

```text
B02.quantity = 1 unit
supplier cost = selected quote unit price * 1
```

The current `42.39` quantity should become impossible except in an explicit fallback state that is reviewed/blocked and never presented as document fact.

---

## PHASE 3: IMPLEMENTATION GUIDANCE

### Approach

Implement in this order:

1. Add durable quote-coverage semantics to Stage 9.
2. Update marketplace orchestration so required source roles are attempted before Stage 10 normal path.
3. Update Stage 8 quantity extraction/persistence for explicit `2no`-style quantity signals.
4. Redesign Stage 10 assessment basis to prefer two-sided quote comparison.
5. Align Stage 11 quantity/revenue basis with Stage 10 and extracted quantity.
6. Update Stage 12/13 verdict and manifest language so fallback/screening basis is visible.
7. Add Woodhouses regression coverage.

### 3A. Stage 9: Required Source Role Coverage

Add a durable way to answer, per candidate:

```text
Did we attempt UK-side quote coverage?
Did we attempt China/import-side quote coverage?
Did we get usable UK-side quote(s)?
Did we get usable China/import-side quote(s)?
If not, why not?
```

Implementation options:

1. Add a new table, e.g. `candidate_quote_coverage`, via a new migration.
2. Or derive coverage from existing `search_attempts` + `supplier_quotes` every time Stage 10 runs.

The preferred implementation is a small durable table because it makes UI/manifest/debugging easier, but deriving from existing rows is acceptable if it is clean and well-tested.

Suggested table shape if adding one:

```sql
CREATE TABLE candidate_quote_coverage (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL,
  notice_id TEXT NOT NULL,
  product_candidate_id TEXT NOT NULL,
  required_role TEXT NOT NULL,
  attempted INTEGER NOT NULL DEFAULT 0,
  usable_quote_count INTEGER NOT NULL DEFAULT 0,
  latest_source TEXT,
  latest_failure_mode TEXT,
  coverage_state TEXT NOT NULL,
  evidence_json TEXT NOT NULL DEFAULT '{}',
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(product_candidate_id, required_role)
);
```

Suggested `required_role` values:

```text
uk_side_quote
china_import_quote
```

Suggested `coverage_state` values:

```text
not_attempted
attempted_no_quote
attempted_failed
usable_quote_available
blocked_captcha
blocked_auth
```

### 3B. Stage 9: Orchestration Must Attempt Alibaba

Current CLI triggers are source-specific:

```bash
target/debug/daemon --ingest amazon-uk
target/debug/daemon --ingest ebay-uk
target/debug/daemon --ingest alibaba-search
```

The E2E harness stopped after eBay produced one quote. That allowed `alibaba-search` to be skipped.

Fix one of these paths:

1. Add a new commercial-coverage ingestion mode, e.g. `--ingest marketplace-coverage`, that drives required source roles in order:
   - UK side: amazon-uk and/or ebay-uk, plus GEM/MX if configured.
   - China/import side: alibaba-search.
   - Persist coverage for each role.
2. Or update the Stage 9 E2E/manifest acceptance flow and Stage 10 gate so `--ingest net-gate` or `--ingest landed-cost` refuses normal basis unless required role attempts exist.

The best product behavior is both:

- Stage 9 has an orchestration path that tries all required roles.
- Stage 10 refuses to silently proceed as normal when required roles are missing.

Required Alibaba behavior:

```text
For every candidate that reaches Stage 9 and has UK-side quote evidence,
the system must attempt alibaba-search or record a durable skip/failure reason.
```

No future run should produce only `/tmp/e2e_woodhouses_alibaba_job.json` without any DB evidence of Alibaba attempt.

### 3C. Stage 8: Quantity Extraction

Revise `skills/pdf-extraction/tender-specification/SKILL.md` carefully. The skill should still avoid full BoQ pricing extraction, but it must extract explicit product quantity evidence when it is a technical requirement signal.

Change the non-goal from broad "do not perform BoQ quantity extraction" to something like:

```text
Do not perform pricing normalization or full bill-of-quantities costing.
Do extract explicit product quantity requirements when the tender text or
schedule states counts such as "2no boilers", "1No. cylinder", or "3 units",
and include them as evidence-backed quantity_hint values.
```

Then update daemon-side persistence so `quantity_hint` can populate:

```text
product_candidates.quantity
product_candidates.quantity_unit
product_candidates.quantity_confidence_bps
source_evidence_json
```

Rules:

- Only parse explicit counts with evidence anchors.
- Do not parse money or BoQ prices as quantity.
- If total quantity maps cleanly to unique schedule rows, allocate deterministically.
- If allocation is ambiguous, do not invent; persist a clarification prompt or low-confidence quantity basis.

Woodhouses target:

```text
B01.quantity = 1
B01.quantity_unit = 'unit'
B02.quantity = 1
B02.quantity_unit = 'unit'
quantity evidence cites schedule row + "2no gas fired heating boilers"
```

### 3D. Stage 10: Two-Sided Quote Comparison

Refactor `run_gross_margin_gate_stage` so quote selection understands source roles.

Current behavior:

```text
select cheapest fully passing quote;
if none fully pass, select best available quote;
compare selected quote against allocated revenue;
```

Required behavior:

```text
1. Load all quotes for candidate.
2. Group quotes by role:
   - uk_side_quote: uk_retail_benchmark, uk_wholesale
   - china_import_quote: cn_supplier_floor
3. Load durable role-coverage attempts.
4. If UK + China/import usable quotes exist:
   - choose best comparable UK-side quote
   - choose best China/import quote
   - mark basis as two_sided_supplier_comparison
   - compute quote spread
   - pass China/import quote to landed-cost path for import/customs/freight if needed
5. If required role attempts happened but China/import quote missing:
   - allow single_side_retail_discount_fallback only if policy permits
   - mark normal coverage false
   - keep confidence lower and reasons explicit
6. If required role attempts did not happen:
   - do not persist normal gross-margin assessment
   - either skip/review with reason missing_required_quote_role_attempt
   - or enqueue/trigger missing marketplace source
```

Suggested basis values:

```text
two_sided_supplier_comparison
single_side_retail_discount_fallback
single_side_wholesale_fallback
rfq_required_no_quote_coverage
missing_required_quote_role_attempt
```

Add fields to `candidate_pricing_assessments` or a linked basis table:

```text
commercial_basis_kind
normal_quote_coverage
uk_quote_id
china_import_quote_id
benchmark_quote_id
selected_supply_quote_id
supplier_spread_bps
fallback_discount_bps
missing_required_roles_json
coverage_evidence_json
```

Do not overload `gross_margin_bps` so the UI cannot tell whether it came from quote-spread or fallback revenue math.

### 3E. Stage 11: Landed Cost Must Use Selected Supply Quote and Extracted Quantity

When two-sided comparison exists, Stage 11 should run landed cost on the selected supply quote, especially China/import quote:

```text
if selected supply quote ship_from_country != GB:
  HMRC customs + shipment sizing + SeaRates + FX
else:
  domestic landed-cost path
```

Quantity basis rules:

```text
1. Operator override quantity wins.
2. Extracted product candidate quantity wins.
3. Supplier quote MOQ/quoted quantity may support screening only.
4. Never use whole lot value / unit price for a candidate when Stage 10 used candidate-allocated revenue, unless candidate_value_share_bps or evidence proves the candidate is the whole lot.
5. If no trustworthy quantity exists, mark basis_undetermined and review/clarify.
```

Woodhouses target:

```text
B02 quantity basis = document_extracted_quantity
B02 quantity = 1 unit
No derived 42.3908435777872 quantity
```

### 3F. Stage 12/13: Verdict and Manifest Must Surface Basis

Net margin and manifests must clearly distinguish:

```text
PASS/REVIEW/REJECT based on two-sided supplier comparison
REVIEW because China/import quote missing after attempted sources
REVIEW because Alibaba not attempted
REVIEW because quantity basis is ambiguous
FALLBACK screening only using 30 percent retail-to-wholesale discount
```

For Woodhouses after the fix, an eBay-only run should not look like a normal high-margin opportunity. It should say something like:

```text
UK-side quote found: eBay UK GBP 3,066.70
China/import-side quote: not attempted or no durable result
Commercial basis: incomplete quote coverage
Action: run Alibaba/import sourcing before margin decision
Quantity: documents indicate 2 boilers total; B02 likely 1 unit
```

---

## DATA MODEL GUIDANCE

The current project uses SQLite migrations in `migrations/` and registration in `crates/daemon/src/db/sqlite.rs`.

Current migration list is in:

```rust
const L0_MIGRATION_PATH: &str = "migrations/0001_sqlite_state_initial.sql";
const L1_MIGRATION_PATH: &str = "migrations/0003_sqlite_runtime.sql";
const L2_MIGRATION_PATH: &str = "migrations/0004_l2_pending_operator_intents.sql";
const L3_MIGRATION_PATH: &str = "migrations/0005_l3_operational_hardening.sql";
const L_PDF_MIGRATION_PATH: &str = "migrations/0006_pdf_extraction_toolkit.sql";
const L_NET_GATE_MIGRATION_PATH: &str = "migrations/0007_net_margin_gate_assessments.sql";
const L_MANIFEST_MIGRATION_PATH: &str = "migrations/0008_manifest_generation_columns.sql";
const L_OPERATOR_OVERRIDES_MIGRATION_PATH: &str = "migrations/0009_operator_input_overrides.sql";
const L_OPERATOR_OVERRIDE_AUDIT_MIGRATION_PATH: &str =
    "migrations/0010_operator_override_audit_rows.sql";

pub const DEFAULT_SCHEMA_EPOCH: i64 = 7;
```

If adding schema, create the next migration rather than mutating old migrations. Update:

- `crates/daemon/src/db/sqlite.rs`
- `migrations/0011_*.sql`
- schema verification if needed
- tests that create/open fresh state DBs

Use existing migration style. Do not create compatibility shims.

---

## REPRODUCTION SCRIPT FOR WOODHOUSES

Use the isolated E2E state. Do not modify live `state/state.db`.

```bash
export E2E_STATE_ROOT=state-e2e-20260504T113157Z
export E2E_RUNTIME_DIR=runtime-e2e-20260504T113157Z
export E2E_ARTIFACT_ROOT=artifacts-e2e-20260504T113157Z
export SPOTLIGHT_PID=proc-pc-956d1aaa-8841-f111-813c-005056b64545

sqlite3 -header -column "file:$E2E_STATE_ROOT/state.db?mode=ro&immutable=1" "
SELECT source, COUNT(*) AS attempts, SUM(result_count) AS results,
       SUM(shortlisted_count) AS shortlisted,
       GROUP_CONCAT(DISTINCT search_failure_mode) AS failure_modes
FROM search_attempts
WHERE product_candidate_id IN (
  SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID'
)
GROUP BY source;

SELECT id, source_name, price_signal_role, source_url, unit_price_gbp_pence,
       currency, ship_from_country, country_of_origin
FROM supplier_quotes
WHERE process_id='$SPOTLIGHT_PID';

SELECT id, name, quantity, quantity_unit, quantity_confidence_bps,
       source_evidence_json
FROM product_candidates
WHERE process_id='$SPOTLIGHT_PID';

SELECT product_candidate_id, selected_quote_id,
       expected_addressable_revenue_ex_vat_pence, gross_margin_bps,
       identity_gate_passed, usability_gate_passed,
       pricing_input_confidence_bps
FROM candidate_pricing_assessments
WHERE process_id='$SPOTLIGHT_PID';

SELECT product_candidate_id, supplier_quote_id,
       json_extract(confidence_breakdown, '$.order_quantity_basis.kind') AS qty_kind,
       json_extract(confidence_breakdown, '$.order_quantity_basis.qty') AS qty,
       json_extract(confidence_breakdown, '$.order_quantity_basis.source') AS qty_source,
       economic_landed_cost_gbp_pence, net_margin_base_bps, decision_grade,
       warnings
FROM landed_cost_estimates
WHERE process_id='$SPOTLIGHT_PID';
"
```

Current bad output to eliminate:

```text
alibaba-search has no search_attempt rows.
B01/B02 quantity is NULL.
Stage 10 B02 expected_addressable_revenue_ex_vat_pence = 6500000.
Stage 11 B02 order_quantity_basis.qty = 42.3908435777872.
Stage 11 B02 economic_landed_cost_gbp_pence = 13000000.
```

---

## ACCEPTANCE CRITERIA

### A. Stage 8 Quantity Extraction

1. Woodhouses B01/B02 no longer have `quantity = NULL` when explicit `2no boilers` evidence is available.
2. B01/B02 quantity evidence references both the schedule row and the mechanical spec quantity sentence, or records an honest ambiguity state if allocation cannot be proven.
3. Quantity extraction does not become full BoQ price extraction.
4. Regression tests cover:
   - `2no gas fired heating boilers` + B01/B02 schedule rows.
   - ambiguous total quantity that cannot be safely allocated.
   - no quantity signal.

### B. Stage 9 Required Marketplace Coverage

1. For a priceable physical candidate, Stage 9 attempts/persists UK-side and China/import-side source roles before normal Stage 10 readiness.
2. `alibaba-search` is attempted for Woodhouses B01/B02 or durably records a failure/skip reason.
3. Temp job JSON files do not count as coverage unless corresponding durable DB rows exist.
4. Regression tests prove that prepared job payload without `search_attempts` is treated as missing coverage.
5. Regression tests prove that eBay-only quote coverage remains incomplete until Alibaba/import is attempted.

### C. Stage 10 Commercial Basis

1. Stage 10 primary path uses two-sided supplier comparison when UK-side and China/import-side quotes are available.
2. Stage 10 records basis kind and missing role information.
3. Stage 10 does not silently use single eBay/Amazon quote as normal gross-margin readiness.
4. The 30 percent retail-to-wholesale discount remains available only as fallback/screening after required role attempts are durably recorded.
5. The UI/manifest can distinguish two-sided comparison from fallback estimate.

### D. Stage 11 Quantity and Landed Cost Basis

1. Stage 11 uses extracted candidate quantity before fallback inference.
2. Stage 11 does not derive `42.39` boilers from whole lot value for Woodhouses.
3. Stage 11 uses the selected supply quote from Stage 10, especially China/import quote when available, for landed-cost chain.
4. If no reliable quantity exists, Stage 11 marks basis as undetermined and forces review/clarification.

### E. Stage 12/13 Decision Surface

1. Net gate reasons include missing required quote role, missing Alibaba/import quote, fallback screening, or quantity ambiguity when applicable.
2. Manifest clearly shows:
   - UK-side quote evidence.
   - China/import-side quote evidence or failed attempt.
   - Commercial basis kind.
   - Quantity basis and evidence.
   - Whether result is normal comparison or fallback.
3. Woodhouses eBay-only state does not present as a normal high-margin pass.

---

## REGRESSION TEST PLAN

Add tests in existing daemon test files where possible. Do not create a new test file unless existing files are truly the wrong home.

Suggested tests:

```text
woodhouses_boiler_quantity_hint_persists_candidate_quantities
marketplace_coverage_requires_alibaba_attempt_for_normal_basis
prepared_marketplace_job_without_search_attempts_is_not_coverage
gross_margin_prefers_two_sided_supplier_comparison
gross_margin_single_uk_quote_is_fallback_after_import_attempt_failure
gross_margin_blocks_normal_basis_when_import_source_not_attempted
stage11_uses_extracted_candidate_quantity_before_lot_value_fallback
stage11_rejects_lot_value_quantity_when_stage10_used_allocated_revenue
manifest_surfaces_commercial_basis_and_missing_quote_roles
```

Minimum Woodhouses fixture shape for tests:

```text
notice value: 13000000 pence
implicit lot value: 13000000 pence
candidate B01: Ideal Evomax 2 80kW, quantity 1 unit
candidate B02: Ideal Evomax 2 60kW, quantity 1 unit
UK quote: ebay-uk, uk_retail_benchmark, GBP 306670 pence, ship_from GB
China quote: alibaba-search, cn_supplier_floor, USD or GBP normalized quote, ship_from CN
```

Also test the current bad fixture:

```text
B02 has only eBay quote.
No alibaba-search search_attempt exists.
Expected: Stage 10 does not mark normal quote coverage.
Expected: manifest/net gate asks for Alibaba/import quote attempt.
```

---

## QUALITY GATES

For any code/schema/skill changes, run:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

If desktop-visible fields or IPC DTOs change, also run:

```bash
cargo test -p procintel-desktop --test query_service
cargo test -p procintel-desktop --test daemon_bridge
cargo test -p procintel-desktop --test native_shell
cd apps/desktop && bunx vitest run
```

If only skills/prompts are edited, still run focused daemon tests that validate parsing/persistence plus `git diff --check`.

---

## DELIVERABLES

1. Durable quote coverage policy for UK-side and China/import-side marketplace evidence.
2. Stage 9 orchestration or gate enforcement that always attempts `alibaba-search` for priceable candidates before normal Stage 10 basis.
3. Durable failure/skip recording when Alibaba/import search cannot produce quotes.
4. Stage 8 quantity extraction/persistence for explicit `2no`-style tender quantity signals.
5. Stage 10 commercial-basis refactor that prefers two-sided supplier comparison and labels fallback single-quote estimates.
6. Stage 11 basis alignment so extracted quantity and Stage 10 candidate-level basis are used consistently.
7. Stage 12/13 reason/manifest updates exposing quote coverage, missing roles, fallback basis, and quantity basis.
8. Regression tests covering Woodhouses and the no-Alibaba-attempt gap.
9. Updated validation manifest or a new follow-up manifest documenting the re-drive.
10. Commit and push to `origin/main`.

---

## CONSTRAINTS

1. Do not modify live `state/state.db` while reproducing; use isolated roots or copied test DBs.
2. Do not delete files.
3. Do not run destructive git commands.
4. Work on `main`.
5. Do not perform real-money or bid-submission actions.
6. Do not count temp `/tmp/*job.json` payloads as marketplace coverage.
7. Do not treat eBay/Amazon as weak by default; they are valid UK-side quote signals. They become weak only when they are the sole side of evidence and the China/import side is missing.
8. Do not use the 30 percent discount as the primary path when two-sided supplier comparison is available or still unattempted.
9. Do not invent quantities from contract value divided by unit price when documents contain explicit quantities or when basis is ambiguous.
10. Do not claim GREEN unless Alibaba/import-side attempt coverage is durable in DB rows.

---

## FINAL VERIFICATION QUERIES

After implementation and re-drive, these queries should show fixed behavior:

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, name, quantity, quantity_unit, quantity_confidence_bps
FROM product_candidates
WHERE process_id='$SPOTLIGHT_PID';

SELECT source, COUNT(*) AS attempts, SUM(result_count) AS results,
       SUM(shortlisted_count) AS shortlisted,
       GROUP_CONCAT(DISTINCT search_failure_mode) AS failure_modes
FROM search_attempts
WHERE product_candidate_id IN (
  SELECT id FROM product_candidates WHERE process_id='$SPOTLIGHT_PID'
)
GROUP BY source;

SELECT source_name, price_signal_role, region, quote_class, COUNT(*) AS quotes
FROM supplier_quotes
WHERE process_id='$SPOTLIGHT_PID'
GROUP BY source_name, price_signal_role, region, quote_class;

SELECT product_candidate_id, selected_quote_id,
       expected_addressable_revenue_ex_vat_pence,
       gross_margin_bps, identity_gate_passed, usability_gate_passed,
       pricing_input_confidence_bps
FROM candidate_pricing_assessments
WHERE process_id='$SPOTLIGHT_PID';

SELECT product_candidate_id, supplier_quote_id,
       json_extract(confidence_breakdown, '$.order_quantity_basis.kind') AS qty_kind,
       json_extract(confidence_breakdown, '$.order_quantity_basis.qty') AS qty,
       json_extract(confidence_breakdown, '$.supplier_cost_basis.basis') AS cost_basis,
       economic_landed_cost_gbp_pence,
       net_margin_base_bps,
       decision_grade,
       warnings
FROM landed_cost_estimates
WHERE process_id='$SPOTLIGHT_PID';

SELECT product_candidate_id, verdict, reasons_json,
       revenue_used_pence, economic_landed_cost_used_pence,
       net_margin_base_bps
FROM candidate_net_margin_assessments
WHERE process_id='$SPOTLIGHT_PID';
"
```

Expected fixed Woodhouses shape:

```text
B01/B02 quantities are not NULL if explicit 2no evidence is allocated.
alibaba-search has durable search_attempt rows or a durable failure reason.
No normal commercial basis exists from eBay-only evidence.
No landed-cost row contains quantity 42.3908435777872 for B02.
Manifest explains UK-side quote, China/import quote status, commercial basis, and quantity basis.
```

---

## REMINDERS

- ULTRATHINK and take your time. This is a cross-stage policy/data-flow fix.
- The purpose is not to make Woodhouses look profitable; the purpose is to make the pipeline tell the commercial truth.
- Alibaba/import-side coverage is a must for normal quote comparison. Missing Alibaba must be visible, durable, and actionable.
- The 30 percent discount is still allowed, but only as a fallback after required quote attempts.
- Explicit tender quantities must beat inferred quantities.
- A high gross margin from one retail quote can be misleading; the manifest and net gate must reveal the evidence basis.
- Keep changes scoped to L1-L3. Stage 15 and L4-L6 are out of scope.
