# Pre-Prep: L0 Schema Foundation & Gold-Set Evaluation

## Domain: L0 Schema Foundation
## Files This Agent Can Edit

| File | Scope | Notes |
|------|-------|-------|
| `docs/strategy/layers/layer-0-gold-set-evaluation.md` | Full file | Primary target |
| `docs/strategy/layer-strategy.md` | §§8-10 only (schema stability, stage scope) | Cross-layer governance |
| `docs/strategy/cross-reference-index.md` | Full file | Navigation updates |
| `docs/strategy/README.md` | Full file | If overview changes |

**Do NOT edit:** Other layer docs (L1–L6). If changes are needed there, add a cross-domain note at the end of the file you're editing, clearly labeled `<!-- CROSS-DOMAIN NOTE FOR L{N}: ... -->`.

---

## Step 1: Pre-Prep Research

Read these documents in order:

1. **`AGENTS.md`** — Agent guidelines, rules, project overview
2. **`docs/strategy/layers/layer-0-gold-set-evaluation.md`** — THE primary doc. Read cover to cover (~3900 lines). Pay special attention to:
   - DDL definitions (all 43 tables, every column)
   - Forward-compatibility columns (which are nullable, which layers populate them)
   - Evaluation harness design (3 experiments, pass criteria)
   - Pipeline processing logic (15 stages)
   - Gold-set corpus definition
3. **`docs/strategy/layer-strategy.md`** — Read §§8-10 (schema stability matrix, stage scope matrix, interface contracts). Note:
   - Column stability rules
   - Authority classification per table
   - Stage grain and commit semantics
4. **`docs/strategy/cross-reference-index.md`** — Understand navigation structure
5. **`docs/planmaxer/active/handoffs/handoff-l0-schema-foundation.md`** — Read the handoff that produced the feedback you're about to integrate

Build a deep mental model of:
- Every table's authority classification (push/local/remote)
- Every forward-compatibility column and which layer populates it
- The evaluation harness methodology and pass criteria
- The stage scope matrix and commit semantics
- Integer pence / basis points representation rules

---

## Step 2: Feedback Integration

The shape is good. The dangerous parts are not “43 tables” or “SQLite.” The dangerous parts are semantic leakage, a couple of hard schema bugs, and an evaluation gate that is too easy to pass for the wrong reasons.

My verdict:

* **Keep** the layer ordering, the authority-matrix idea, `stage_run_id`, and the “provision future scalar columns early” instinct.
* **Do not freeze L0 as written**. There are a handful of foundation-level issues that will create real pain later.

## What I would fix before calling L0 stable

| Priority | Finding                                                                                                                                                                                                                     | Concrete change                                                                                                                                                                                                     |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0       | **Money columns are too small in Postgres-canonical DDL.** All `_pence` fields are `integer` in Layer 0 §2.5, which caps at **£21,474,836.47**. That is not enough for many framework ceilings or larger notices.           | Change every persisted money column to **`bigint`** in the canonical schema. Also change `last_local_seq`, `from_local_seq`, `to_local_seq`, and preferably `row_version` to `bigint`.                              |
| P0       | **Non-GBP “original” amounts are mis-modeled.** `contract_notices.estimated_value_original_pence` and `supplier_quotes.unit_price_original_pence` are wrong when currency is EUR/USD/CNY. “Pence” only makes sense for GBP. | Rename/replace with **`*_original_minor`** plus currency code. Keep GBP-normalized amounts as `*_gbp_pence`.                                                                                                        |
| P0       | **Sub-penny unit prices will break the quote model.** `supplier_quotes.unit_price_*_pence` cannot represent things like £12.99 / 5000 gloves = **0.2598p each**.                                                            | Add **basis-total price fields**: `offer_price_original_minor`, `offer_price_gbp_pence`, `price_basis_qty`, `price_basis_unit`. If you need normalized per-unit storage, store **micro-pence** separately.          |
| P0       | **Cross-authority foreign keys violate the authority model mechanically.** Push-owned tables point at local-only tables.                                                                                                    | Enforce a rule: **no FK from push-owned or remote-owned tables to local-only tables**. Use local-only link tables or opaque refs instead.                                                                           |
| P0       | **The stage model is inconsistent around qualification.** Layer 0 §3.4 does provisional notice-level qualification before lots exist; layer-strategy §10 says Qualify is lot-grain.                                         | Split into **`prequalify_notice`** and **`qualify_lot_final`**, or explicitly define one stage with two persisted grains. Right now it is fuzzy.                                                                    |
| P0       | **The evaluation gate can pass while commercial reality is still wrong.** Current experiments are global averages and do not test abstention, hard negatives, or category slices.                                           | Keep the 3 headline experiments, but add **slice floors, hard-negative precision, abstention correctness, and decision-agreement metrics**.                                                                         |
| P1       | **Several columns/tables the later layers need are missing or under-specified.**                                                                                                                                            | Add them now: `pending_operator_intents.reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`; richer `product_commodity_codes`; evaluation tables; deterministic scope keys and fingerprints. |
| P1       | **Docs disagree on what exists in L0 vs L1/L4.**                                                                                                                                                                            | Resolve the contradictions now, or implementation agents will fork the architecture in practice.                                                                                                                    |

---

## 1. Schema design review

### What is strong

These decisions are right:

* **Authority split** is the right shape.
* **`pending_operator_intents` before `operator_decision_heads`** is the right pattern.
* **`stage_run_id` from day 1** is right.
* **JSON for evolving customs detail** is the right hedge for L5 (`duty_breakdown_json`, `measure_interaction_json`, etc.).
* **SQLite local-first** is fine for this workload.

### Where the current design will bite

### A. “All columns from creation” is good as a **logical schema** rule, not as a full migration cure

`layer-strategy.md` §9 is directionally right, but the belief is overstated.

It eliminates one class of pain: **late `ALTER TABLE` on hot synced tables**.

It does **not** eliminate:

* index migrations
* check-constraint changes
* view changes
* backfills
* RLS / RPC / sync protocol changes
* semantic migrations
* codegen / wrapper migrations

So I would keep the principle, but I would stop describing it as “eliminates migration complexity entirely.” It doesn’t.

The real rule should be:

> Freeze the **logical schema slots** early, but treat **activation, constraints, views, and projections** as first-class migration surfaces.

### B. Your logical schema and physical DDL are currently conflated

Layer 0 §2.1 says the SQL is “Postgres-canonical” while the operational authority is SQLite. That is backwards for this system.

For this architecture, you need:

1. **One logical schema registry**: table names, columns, semantic contracts, authority class.
2. **One SQLite physical DDL**: local-only + push-owned + pulled caches that actually exist locally.
3. **One Supabase physical DDL**: push projections + remote-owned tables that actually exist remotely.

Without that split, you get impossible constructs like remote schemas that would need local-only FKs.

### C. The authority matrix is conceptually correct, but mechanically violated

These are the worst offenders from Layer 0 §2.5:

* `contract_notices.source_id -> sources(id)` where `sources` is local-only.
* `contract_notices.raw_document_id -> raw_documents(id)` where `raw_documents` is local-only.
* `search_attempts.session_id -> agent_session_logs(id)` where `agent_session_logs` is local-only.
* `process_stage_status.error_manifest_id -> error_manifests(id)` where `error_manifests` is local-only.
* `bid_workflow_states.bid_job_id -> workflow_jobs(id)` where `workflow_jobs` is local-only.
* `workflow_job_summaries.job_id -> workflow_jobs(id)` where the source table is local-only.
* `error_manifest_summaries.id -> error_manifests(id)` same problem.
* `validation_verdict_summaries.id -> validation_verdicts(id)` same problem.

That is the exact opposite of clean authority boundaries.

**Rule I would adopt now:**

* local-only tables may FK to local-only or push-owned rows present locally
* push-owned tables may FK only to tables that also exist in the push domain
* remote-owned tables may FK only within the remote domain
* cross-authority references are **opaque IDs or local-only link tables**, not FKs

### D. `pending_operator_intents` is missing the fields the later layers already assume

Prompt summary + Layer 2 narrative say this table carries reviewed-basis metadata. Layer 0 §2.5.8 does not.

Add now:

* `decision_note`
* `reviewed_basis_hash`
* `reviewed_manifest_hash`
* `reviewed_at_stage_run_id`
* `reviewed_at`

Also, `base_remote_revision bigint not null` is wrong for pre-L4 local-only operation. In Layer 2 there may be **no remote head yet**.

Make it either:

* nullable, or
* `default 0` with explicit meaning “no remote revision existed when queued”

Also add a coalescing mechanism. The comment says “latest-per-process coalescing,” but the schema does not enforce it. Add either:

* a partial unique index on active queued intents per `process_id`, or
* `supersedes_intent_id` / `superseded_by_intent_id`

### E. `product_commodity_codes` is too thin for Layer 5

Layer 0 §2.6 defines `CandidateCommodityCode` with:

* `code`
* `digits`
* `source`
* `score`
* `description`

But Layer 0 §2.5.9 `product_commodity_codes` stores only:

* `product_candidate_id`
* `commodity_code`

That will not support L5 multi-source recall and ranking.

I would expand it now to include at least:

* `id`
* `commodity_code`
* `digits`
* `source`
* `score_bps`
* `rank_ordinal`
* `description`
* `evidence_json`
* sync columns if it remains push-owned

Same story, smaller version, for `notice_cpv_codes` and `lot_cpv_codes`: add `is_primary`.

### F. Push-owned junction tables are missing sync tracking columns

Layer 0 §2.2 rule 4 / layer-strategy §9 rule 3 say all push-owned tables carry sync tracking columns. But:

* `notice_cpv_codes`
* `product_commodity_codes`
* `lot_cpv_codes`

do not.

Either:

* make them true push-owned rows with sync columns, or
* explicitly define them as parent-embedded sets in the outbox protocol

Right now they are in between.

### G. You are missing value-weighted coverage

Layer 0 §3.13 coverage rollup is lot-count based. That is not enough.

A process can pass a 50% lot-count floor while leaving 80% of value unresolved.

Add now on `pipeline_assessments`:

* `priced_value_coverage_bps`
* `landed_cost_value_coverage_bps`
* `net_qualified_value_coverage_bps`

And make the auto-qualify gate use **value coverage**, not just lot coverage.

### H. A few columns should not exist in their current form

Not because they are “too early,” but because the semantics are wrong.

The main ones:

* `*_original_pence` for non-GBP amounts
* `manifests.artifact_path` on a push-owned logical row
* local-only diagnostic FKs embedded in push-owned tables

`artifact_path` is a local filesystem concern. If manifests must be visible remotely, the portable fields are:

* `artifact_hash`
* `artifact_bytes`
* `artifact_content_type`
* `artifact_locator` or storage key

---

## 2. Integer arithmetic analysis

Your instinct is right for persisted money and bounded probabilities. The current implementation detail is not.

### The hard problems

| Scenario                             | Current design outcome                                 | Why it is wrong                                                   | Fix                                                  |
| ------------------------------------ | ------------------------------------------------------ | ----------------------------------------------------------------- | ---------------------------------------------------- |
| **£50m framework ceiling**           | `integer` `_pence` overflows in Postgres-canonical DDL | 5,000,000,000 pence > 2,147,483,647                               | Use `bigint` for all money fields                    |
| **CNY 138.00 carton price**          | Stored in `unit_price_original_pence`                  | Fen are not pence                                                 | Store original amounts in **minor units**, not pence |
| **£12.99 / 5000 gloves**             | per-unit integer pence becomes 0p                      | sub-penny units disappear                                         | Store **basis-total price + basis qty/unit**         |
| **500 line items × 5p at 3.5% duty** | per-line truncation gives 0p duty each, total 0p       | aggregate duty should be 87.5p                                    | **Aggregate first, round last**                      |
| **Revenue 1000p, cost 1200p**        | margin = -2000 bps                                     | negative margin is valid, but any 0..10000 blanket rule breaks it | distinguish **bounded** vs **signed** bps fields     |

### Specific recommendations

1. **All GBP-normalized money fields:** use `bigint`.
2. **All non-GBP original amounts:** use integer **minor units** with currency code.
3. **All calculations:** use integer scaled math or decimal math internally, not SQLite `REAL`.
4. **FX rates:** do **not** rely on SQLite `REAL` (`Layer 0 §2.1` maps numeric -> REAL). Use a scaled integer like `*_rate_e8 bigint`.
5. **Quantities that drive duty/freight math:** keep original decimal string and, where used in calculations, a normalized scaled integer.
6. **Rounding policy:** define it explicitly now.

My recommended policy for L0/L1 screening:

* carry intermediate money in **micro-pence** or equivalent scaled integers
* **aggregate before rounding**
* round to whole pence **only at persisted estimate boundaries**
* use **round half away from zero**
* when L5 introduces declaration-grade logic, allow authority-specific rounding rules there if needed

### Bps needs two classes, not one

Right now the docs talk like all `_bps` are 0–10,000. That is only true for:

* confidence
* coverage
* bounded heuristic scores
* recovery rates

It is **not** true for:

* `gross_margin_bps`
* `net_margin_*_bps`
* `fx_basis_delta_bps`
* `customs_cost_spread_bps`
* `negotiation_uplift_bps`

So define two constraint families now:

* **bounded bps**: `0 <= x <= 10000`
* **signed/unbounded bps**: integer, possibly negative, possibly > 10000

---

## 3. Evaluation harness critique

The current 3 experiments prove **something useful**, but not the whole claim you are implying.

They prove:

* can the system recover product intent from curated procurement material
* can it find external comparable price signals
* can it generate coarse screening signal on the evaluated strata

They do **not** prove:

* decision-grade landed cost
* robustness on hard categories
* correct abstention on customs/compliance ambiguity
* runtime economics
* temporal stability

### The biggest blind spots

### A. No hard-negative gate

Layer 0 §6 has edge cases, but the headline experiments are almost entirely positive-case oriented.

You need a formal negative corpus: services-only, mixed contracts, accessory swamps, wrong-standard near matches, non-quoteable custom fabrication.

Add a gate such as:

* **false auto-qualify rate on hard negatives <= 5%**
* **false genuine-match rate on hard negatives <= 10%**

### B. Global averages are too easy to game

Easy office-supplies lots can wash out hard PPE or packaging cases.

Keep the global 80/70/75, but add slice floors. My recommended starting point:

* no in-scope category below **65 / 55 / 60**
* no active `matchArchetype` slice with `n >= 8` below **60 / 50 / 55**

### C. The “unknown” escape hatch can inflate scores

If the system calls too many hard cases `matchArchetype = unknown`, experiment 2 can improve artificially.

Add a cap:

* `matchArchetype = unknown` rate on the gating corpus must stay below a threshold, e.g. **20%**

Same logic applies to `candidateType = unknown`.

### D. Margin agreement should be decision-oriented, not just point-error-oriented

“Within ±5pp” is not enough. A system can be within 5 points and still be on the wrong side of the business threshold.

Add:

* **decision agreement** on `auto-qualify / review / reject`
* **auto-qualify precision**
* **abstention correctness** on challenge cases

I would target:

* decision agreement >= **85%**
* auto-qualify precision >= **85%**
* abstention correctness on challenge corpus >= **90%**

### E. Your whitelist mixes L0-decisionable categories with L5-dependent categories

Layer 0 §1.6 itself says:

* basic PPE = hard
* basic furniture = hard
* packaging = split, with dangerous subfamilies

That means the current whitelist is mixing “good screening candidates” with “abstain-or-blocker candidates.”

Split the corpus into two sets:

**Gating corpus** for L0 margin screening:

* office supplies
* simple cleaning hardware
* hand tools
* simple packaging subfamilies

**Challenge corpus** for abstention/blocker correctness:

* PPE
* metal or remedy-adjacent furniture
* remedy-adjacent packaging families
* compliance-heavy goods

That fixes the screening-grade vs decision-grade tension cleanly.

### F. The corpus is temporally under-specified

This is a major one.

If the gold set contains historical notices, and the harness runs supplier enrichment with **today’s** marketplace prices and tariff data, the label and the evidence are from different market states.

You need an **as-of date** per gold-set case.

Otherwise:

* expiry logic rejects everything historical
* margin labels drift with the market
* reruns become non-reproducible

Add at least:

* `case_as_of_at`
* `label_as_of_at`
* captured evidence bundle hash

And separate two test lanes:

1. **Reproducible benchmark lane** using captured real evidence snapshots
2. **Live smoke lane** hitting current external sources

That is not “using mocks.” It is preserving the observed market state that the label was created against.

### G. 50 lots is a pilot, not a serious gate

With 50 lots, one or two cases move the percentages materially.

My recommendation:

* **50** = smoke floor
* **80+** = real go/no-go gate

And include at least:

* 20 hard negatives / mixed cases
* enough category/archetype representation to avoid washout
* 10–20% double-annotated cases to measure label noise

### H. The harness schema is missing

For a layer called “Gold Set + Evaluation Harness,” the runtime tables are richly specified, but evaluation persistence is not.

I would add four local-only tables now:

* `gold_set_cases`
* `evaluation_runs`
* `evaluation_case_results`
* `evaluation_metric_summaries`

Without these, reproducibility and regression analysis will end up in ad hoc files.

---

## 4. Stage scope matrix validation

This is where I would make the biggest conceptual change.

### The current model is inconsistent on qualification

`layer-strategy.md` §10 says **Qualify** is a lot-grain stage.

`layer-0-gold-set-evaluation.md` §3.4 describes a provisional notice-level qualification **before lot extraction**.

Both cannot be the persisted truth at once.

### What I would change

| Stage              | Current doc stance                                                         | What I would do                                                                                                      |
| ------------------ | -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Qualify            | lot-grain in cross-layer strategy; provisional notice-level in L0 pipeline | Split into **`prequalify_notice`** and **`qualify_lot_final`**                                                       |
| Extract products   | additive                                                                   | **Set-replacement current candidate set** per lot                                                                    |
| Score quoteability | additive by implication                                                    | **Set-replacement current scoring/profile** per candidate version                                                    |
| Price-signal match | additive                                                                   | Append-only attempts are fine, but **current shortlist semantics must be set-replacement** per candidate-source pair |
| Gross margin gate  | additive                                                                   | **Set-replacement current assessment** per candidate                                                                 |
| Landed cost        | additive                                                                   | Append-only scenarios are fine, but define **current scenario set** per candidate+quote+basis                        |
| Net margin gate    | additive                                                                   | **Set-replacement current decision** per candidate                                                                   |

The key idea:

* **audit history can be append-only**
* **decision surface cannot be purely additive**

If you leave extract/match/gate outputs as “just additive,” you will accumulate zombie candidates, stale cheap quotes, and duplicate assessments.

### Use stage-head-filtered current views everywhere

Once L1 exists, no application query should hit additive base tables directly.

Create and standardize views such as:

* `current_notice_lots`
* `current_product_candidates`
* `current_search_profiles`
* `current_supplier_quotes`
* `current_candidate_pricing_assessments`
* `current_landed_cost_estimates`

If you do not do that, every developer will eventually forget a stage-head join and you will get subtle stale-data bugs.

### Define `scope_id` canonically now

`process_stage_status.scope_id text` is too loose.

Define it in `layer-strategy.md` §10:

* notice: stable notice id
* lot: deterministic `lot_key`
* candidate: deterministic `candidate_key`
* candidate_source_pair: `hash(candidate_key + source + search_profile_hash)`
* candidate_quote_scenario: `hash(candidate_key + quote_key + calculation_basis_hash)`

Then add:

* `unique(process_id, stage, scope_kind, scope_id, stage_run_id)` on `process_stage_status`
* CHECK constraints on `stage` and `scope_kind`

You also need stable natural keys:

* `notice_lots.lot_key`
* `product_candidates.candidate_key`
* `search_attempts.query_fingerprint`

Without them, rerun diffing and gold-set scoring stay messy.

---

## 5. Ideas you did not ask about but should care about

### A. `product_commodity_codes` is the canary for unknown future pain

This table tells me where the design still pretends later semantics are settled when they are not. Your L5 customs engine already wants ranked, sourced candidate codes. The current table cannot hold that.

That is the exact category of field that deserves richer structure **now**, not later.

### B. Defaults should be epistemic, not optimistic

A few defaults are risky.

For example, `source_runtime_state.last_health_state default 'green'` is too optimistic before first canary. `unknown` is safer.

Same principle anywhere a default can look like a fact rather than “not yet assessed.”

### C. The harness DB should stay separate from the live state DB

Because L0 and later layers share table names and row shapes, it will be very easy to accidentally mix gold-set rows with live operational rows.

Do not reuse the harness database file as the L1 operational DB.

### D. Your codegen/types need to understand layer activation

Documentation alone will not stop an L2 developer from writing L5 columns.

Add a machine-readable activation registry with, at minimum:

* table
* column
* authority class
* first write layer
* first read layer
* null semantics

Then generate per-layer write DTOs / SQL lint rules from it.

### E. This is the right place for branded scalar types

You are using raw SQL with typed wrappers. Good.

Use that leverage:

* `Pence`
* `MinorUnits`
* `BpsBounded`
* `BpsSigned`
* `ScopeId`
* `StageRunId`

That will prevent a lot of accidental nonsense.

---

## Direct answers to the three strong beliefs

### 1. “All columns from creation prevents migration pain.”

**Partly true.** It prevents some of the worst sync-era DDL pain.

It does **not** remove migration pain. It moves a lot of it into:

* semantics
* constraints
* projections
* current-state views
* backfills
* activation discipline

So I would **keep the principle**, but only with:

* logical-vs-physical schema separation
* activation metadata
* same-authority FK discipline
* stronger check constraints
* current-state views

Without those, it becomes “we avoided `ALTER TABLE`, but created semantic drift instead.”

### 2. “Integer pence and basis points everywhere.”

**Correct for persisted GBP amounts and bounded probabilities. Incorrect if applied naively to original-currency amounts, unit prices, FX, quantities, and signed margins/spreads.**

The corrected rule should be:

* GBP-normalized money: **bigint pence**
* original-currency money: **bigint minor units + currency code**
* FX: **scaled integer fixed-point**
* bounded scores/confidence/coverage: **0..10000 bps**
* margins/deltas/spreads: **signed bps**

### 3. “Gold set proves thesis before runtime.”

**It proves the right first thesis only if you narrow the claim.**

It can prove:

* the identity chain is viable
* screening-grade external price-signal discovery is viable
* coarse commercial triage is viable on gated categories

It does **not** prove:

* decision-grade customs intelligence
* category expansion
* runtime reliability
* live-source stability
* commercial viability of hard categories

That is still valuable. It just needs to be stated honestly and gated properly.

---

If I had to cut this down to six concrete moves: fix money types, fix original-currency fields, remove cross-authority FKs, split qualification, add hard-negative/abstention evaluation gates, and enrich `product_commodity_codes` plus `pending_operator_intents`. Those six changes would remove most of the avoidable future pain.


### Integration Rules

For each proposed change from the reviewer, evaluate:

1. **Agree strongly** → Apply directly to the appropriate file
2. **Agree partially** → Apply the useful parts, note what you modified and why
3. **Disagree** → Do NOT apply. Note the disagreement and reasoning at the end of your response

### Specific Guidance

- **Schema changes (new columns, modified columns):** Acceptable IF they follow the authority-matrix rules and don't violate "no column renames." New columns must be nullable (forward-compatibility principle). Add them to L0 DDL.
- **Evaluation harness changes (new experiments, modified pass criteria):** Acceptable. Update the experiments section.
- **Stage scope changes (different grain or commit semantics):** Requires careful cross-layer analysis. If you change a grain assignment, verify it doesn't break L1-L6 assumptions. Add cross-domain notes for affected layers.
- **Integer arithmetic edge cases:** If the reviewer identifies real precision issues, add explicit rounding rules. Document the rounding policy near the integer-pence/bps rule.
- **New tables:** High bar. Must be justified by a domain entity that doesn't fit existing tables. Add to L0 DDL with full column spec, authority classification, and sync tracking columns.

### Preservation Rules

- Preserve all ADR numbers and references
- Preserve table names exactly (no renames per schema stability)
- Preserve authority classifications (push/local/remote per table)
- Preserve integer pence / bps representation
- Update line counts in any references if the doc grows substantially

---

## Step 3: Fresh-Eyes Optimization

After completing the edits, re-read the entire L0 doc with fresh eyes. Check for:

1. **DDL consistency:** Do all tables have consistent column patterns? (sync tracking columns on all push-owned tables? created_at/updated_at everywhere?)
2. **Forward-compatibility completeness:** For every column referenced in L1-L6 layer docs, verify it exists in L0 DDL as nullable.
3. **Evaluation harness → pipeline alignment:** Do the 3 experiments actually test the stages they claim to test? Are the stages in the pipeline logic consistent with the stage scope matrix?
4. **Integer representation consistency:** Are ALL monetary values in pence? Are ALL percentages/confidence/coverage in bps? No exceptions?
5. **Authority classification consistency:** Cross-check every table's authority (push/local/remote) against layer-strategy.md §8.
6. **Gold-set definition completeness:** Is the gold-set corpus definition sufficient for all 3 experiments? Are there experiments that need data the gold set doesn't provide?

---

## Step 4: Sync Pass

1. Update `docs/strategy/cross-reference-index.md` if any new tables, entities, or concepts were added
2. Update `docs/strategy/README.md` if the L0 scope description changed
3. Verify layer-strategy.md §§8-10 are consistent with any changes made to L0
4. Add cross-domain notes for any changes that affect L1-L6 (e.g., new columns that L1 must populate, changed stage scope that L3 must handle)

Write all updates using as-if-always language (not changelog style).
