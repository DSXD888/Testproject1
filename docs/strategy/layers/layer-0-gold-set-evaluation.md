# Layer 0 — Gold Set + Evaluation Harness

> **Authority:** This document is the execution-grade specification for Layer 0. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M0
> **Dependencies:** None (greenfield)
> **Exit criteria:** Experiments 1–3b pass on the gold-set corpus (≥80 lots: gating + challenge).

> **Implementation Status (as of 2026-05-02):** Layer 0 is **PARTIALLY BUILT**. Gold-set fixtures and daemon integration/evaluation harness scaffolding exist in `crates/daemon/tests/`, but the full standalone evaluation pipeline described here is not complete. This layer remains the measurement foundation for L1-L3; no inline implementation markers are added because this pass annotates only built L1-L3 design decisions.
>
> **Updated 2026-06-04 — goods-led labour-cost harmonization.** §1.6 (gold-slices tied to the dominance floor + no-list/bespoke flags), §2.2.1 / §2.5.2 DDL (labour/permit/goods-share columns + new `maintenance_period_estimates` table at schema epoch 10, `migrations/0014`), §3.4 (two-phase goods value-share qualification), §3.11.4 (labour step + Phase-2 dominance gate), and §3.13 (goods value-share + labour coverage metrics) were reconciled to the shipped goods-led install/maintenance extension. The schema epoch is now **10**.

---

## 1. Overview, Capability & Project Shape

### 1.1 Layer Narrative

Layer 0 proves the intelligence thesis on a curated lot corpus before building any live-source runtime. This is evaluation infrastructure first: gold-set data, harness logic, skill authoring standards, and the screening-grade TypeScript pipeline exercised against curated inputs. No daemon, no desktop, no sync, and no remote authority.

Skills run from the command line for benchmark execution only. Results land in a harness-local SQLite dataset and are verified by the automated evaluation harness and manual inspection. Layer 0 deliberately does **not** count as the first live pipeline layer; it is the measurement system that Layer 1 must satisfy.

**Why this layer exists:** The riskiest assumptions live in the intelligence pipeline — can the system extract usable product candidates, match them to real marketplace listings, and produce margin estimates accurate enough to act on? Infrastructure is well-understood engineering that follows, not leads. Wrong answers at scale are worse than no automation. Building a reliable daemon that produces unreliable intelligence is wasted work. Building credible intelligence first means every infrastructure component has a clear reason to exist.

**If we never build Layer 1:** The gold set and evaluation harness remain valuable as a benchmarking tool and intelligence pipeline prototype. The skills and TypeScript intelligence code can run standalone indefinitely. The gold-set corpus, bilingual lexicon, and evaluation metrics survive as reusable assets.

### 1.2 Capability Delivered

A curated gold-set lot corpus (≥80 lots: ≥60 gating + ≥20 challenge) runs through extraction, matching, and screening-grade cost estimation, producing per-lot precision/recall metrics against human-labeled ground truth. Four experiment groups are validated:

| # | Experiment | Pass Criteria | What It Proves |
|---|-----------|--------------|----------------|
| 1 | **Extraction quality** — Can we pull structured lot/product candidates from real procurement documents? | ≥80% of gating-corpus lots yield ≥1 usable product candidate. No in-scope category below 65%. | The system can read procurement intent |
| 2 | **Matching quality** — Can marketplace search find relevant supplier listings for extracted candidates? | ≥70% of candidates with `matchArchetype` produce ≥1 genuine match. Slice floors: no category below 55%, no archetype slice (n≥8) below 50%. | The system can connect demand to supply |
| 3 | **Margin accuracy** — Can screening-grade landed cost produce margin bands that distinguish real opportunities from noise? | Margin bands agree with human expert on ≥75% of gating-corpus lots (±5pp). Decision agreement ≥85%. Auto-qualify precision ≥85%. | The system produces actionable commercial signal |
| 3b | **Hard-negative correctness** — Does the system correctly abstain on hard cases? | False auto-qualify rate ≤5% on challenge corpus. Abstention correctness ≥90%. `matchArchetype=unknown` rate ≤20%. | The system does not fabricate confidence where it should abstain |

Experiment 4 (runtime reliability) is deferred to Layer 1 — it requires the Rust daemon.

### 1.3 Validation-First Delivery Philosophy

Delivery is sequenced by **risk retirement**, not architectural completeness:

1. **Risk retirement, not architectural completeness.** The riskiest assumptions live in the intelligence pipeline. Infrastructure is well-understood engineering that follows, not leads.
2. **Vertical capability slices, not horizontal stack layers.** Each layer delivers an end-to-end flow, not "all Rust components" or "all TypeScript."
3. **Schema from day 1, features incrementally.** Sync tracking columns (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`) are present on all push-owned tables from their first creation in Layer 0. Validation fields (`parent_job_id`, `validation_required`, `validation_verdict_id`) exist as nullable columns on `workflow_jobs` from Layer 0. All columns needed in any future layer exist as nullable from table creation. The sync ENGINE and validation ENFORCEMENT are deferred — the schema provisions are not.

The identity chain is the critical bottleneck: documents/lots -> candidate identity -> quoteability -> comparable listing shortlist -> price-signal usability. The architecture compresses this into concrete stages with deterministic gates, `matchArchetype`-driven query planning, four-state evidence comparison, adaptive diversified shortlisting, compliance evidence tiers, and dual confidence scoring with lexicographic gating to prevent fake precision from propagating downstream.

### 1.4 Minimum Viable Experiment (MVE) Definition

The MVE is ~25–35 files proving experiments 1–3 on a curated lot corpus. No daemon, no cron, no desktop UI, no viewing portal, no Supabase sync. Skills run from CLI. Results land in local SQLite. Evaluation is via test harness and manual inspection.

**MVE scope:**

- Gold-set lot corpus (50–100 curated lots with human labels for product candidates, expected matches, and margin bands)
- Document acquisition for gold-set notices
- Lot extraction with product candidate generation
- `matchArchetype` assignment and slot criticality scoring
- Structured standard normalization (family/part/edition/amendment/class)
- Quoteability scoring with `recallRiskScore`
- One marketplace search skill (eBay) with category-prior search and alias-family query generation
- Search-card screening with four-state evidence comparison
- Detail-page enrichment and compliance evidence tier extraction
- Screening-grade landed-cost bands (duty via HMRC Tariff API, flat-rate freight estimate, single FX rate)
- Evaluation harness: lot-level precision/recall against gold-set labels
- Recall measurement instrumentation (recall@k by archetype x source)
- Local SQLite for intermediate results
- All skills run from CLI (no daemon)

**MVE explicitly excludes:**

- Rust daemon (skills run via CLI)
- Cron scheduling (manual trigger)
- Desktop or web UI (CLI + test harness)
- Supabase sync
- Relation resolution (gold-set lots are pre-curated)
- Multiple procurement sources (gold set is pre-selected)
- Full landed-cost engine (screening-grade only)
- Bid workflow automation
- Worker-validator system (Layer 3)
- Multi-source marketplace search (Layer 5)

### 1.5 Core Validation Order

Four experiments, run in sequence. Each must pass before the next is worth pursuing. Layer 0 proves experiments 1–3; experiment 4 is Layer 1's responsibility.

| # | Experiment | Pass Criteria | What It Proves | Layer |
|---|-----------|--------------|----------------|-------|
| 1 | **Extraction quality** | ≥80% of gating-corpus lots yield ≥1 usable product candidate with correct unit, quantity, specification. No in-scope category below 65%. | System can read procurement intent | L0 |
| 2 | **Matching quality** | ≥70% of candidates with `matchArchetype` produce ≥1 genuine match. No in-scope category below 55%. No active archetype slice (n≥8) below 50%. | System can connect demand to supply | L0 |
| 3 | **Margin accuracy** | Margin bands agree with human expert on ≥75% of gating-corpus lots (±5pp). Decision agreement ≥85%. Auto-qualify precision ≥85%. | System produces actionable commercial signal | L0 |
| 3b | **Hard-negative correctness** | False auto-qualify rate ≤5% on challenge corpus. Abstention correctness ≥90%. `matchArchetype=unknown` rate ≤20%. | System correctly abstains on hard cases | L0 |
| 4 | **Runtime reliability** | 10 consecutive batch runs, ≥90% success rate, 0 orphaned processes | System can operate unattended | L1 |

Stop/go rule: Do not start Layer 1 until Layer 0 passes experiments 1–3b on the gold-set corpus (≥80 lots: ≥60 gating + ≥20 challenge).

### 1.6 Category Whitelist

Start with a narrow goods-category whitelist: commodity-like products with low regulatory burden, high marketplace availability, and straightforward landed-cost estimation.

**Initial whitelist (6 categories):**

| Category | Classification Difficulty | Trade Remedy Exposure | Key Risks |
|----------|--------------------------|----------------------|-----------|
| **Office supplies** | **Easy.** A4 copier paper, envelopes, binders, notebooks are screening-grade candidates if substrate and pack basis are explicit. | Low in current UK measures. | Traps: generic "stationery", labels, adhesive-backed products, paper-vs-textile wipe ambiguity. |
| **Cleaning products** | **Medium-hard.** Buckets and simple brushes are fine. Wipes, cloths, chemically treated consumables, and mixed cleaning kits are not. | Low direct remedy exposure. | Substrate/preparation ambiguity (paper vs nonwoven textile vs cellulose vs chemically impregnated preparations — different tariff chapters). |
| **Basic PPE** | **Hard.** Gloves and helmets are `standard_class_led` products needing facts procurement notices typically omit (material, sterile/non-sterile, examination vs household/industrial, standard met, shell material, end use). | Lower than packaging or metal goods. | Very high **false confidence risk** — engine may resolve a plausible code without enough facts to determine the legally correct one. |
| **Hand tools** | **Medium.** Single non-powered tools are workable. Sets, replacement parts, blades, bits, kits, and anything edging toward powered/electrical are not. | Lower than packaging, but steel-family adjacency matters. | Powered/electrical boundary confusion. |
| **Basic furniture** | **Hard.** Material and construction method matter too much. Wood-only simple items are workable. Metal shelving, office screens, partition systems, modular frames — remedy adjacency jumps sharply. | **High for metal subfamilies.** Organic coated steel: AD rates 0%–26.1%, CVD rates 13.7%–44.7% residual. Aluminium extrusions: 0%–35.1% residual. | Metal shelving/frames have live UK trade remedy measures. |
| **Packaging materials** | **Split: easy subfamilies / dangerous subfamilies.** Corrugated cartons and simple poly mailers are easy. Foil, glass containers, metal can/tin mill families are high-risk. | **Very high for dangerous subfamilies.** Aluminium foil: 14.2%–35.6% residual (to June 2029). Glass containers from China: active investigation AD0087 (opened 5 March 2026). Tin mill products: 27.85% cooperating / 49.98% residual (imposed 12 March 2026). | Cached "no remedy risk" assumptions go stale fast; packaging should be treated as two sub-categories. |

**Goods-led install/maintenance expansion (low-reg, gated):**

These categories may enter production only through labelled gold-set slices. Each slice must satisfy the category-specific gate before live use; until then the category is evaluation-only.

| Slice Label | Category | Admission Notes |
|-------------|----------|-----------------|
| `gold-slice: install_lighting_festive_led` | Festive/LED lighting supply-install and seasonal de-install | Low-reg public-realm goods-led mixed work; permit and access-at-height assumptions route to review when unresolved. |
| `gold-slice: install_signage_low_reg` | Signage supply-install | Low-reg physical goods with fitting labour; excludes wayfinding consultancy-only services. |
| `gold-slice: install_fencing_low_reg` | Fencing supply-install | Low-reg outdoor goods plus installation; excludes civil-engineering-only works packages. |
| `gold-slice: install_playground_outdoor` | Playground/outdoor equipment supply-install-maintain | Goods-led capital event with recurring maintenance kept as visible opex. |
| `gold-slice: install_blinds_low_reg` | Blinds supply-install | Low-reg made-to-size commodity installation; bespoke/custom-manufactured flags still route to review. |
| `gold-slice: install_shelving_wood_plastic` | Shelving/racking (wood/plastic) supply-fit | Admitted as a low-reg furniture-adjacent slice when material evidence is wood or plastic. |
| `gold-slice: install_existing_furniture_ppe` | Install on existing furniture/PPE | Labour can price fitting/assembly for existing goods, but the underlying goods must remain in the L0 whitelist. |
| `gold-slice: metal_shelving_racking_trade_remedy_watchlist` | Metal shelving/racking split-out | Trade-remedy-heavy; not part of the wood/plastic slice and requires separate customs remedy evidence before production. |

**What actually admits or excludes these slices (the economic mechanism):** These slices are governed by the goods-led two-phase value-share model and the UK installation-labour cost engine documented in `layer-5-expanded-commercial-intelligence.md` §1.17 and in §3.4 / §3.11 below. A slice is admitted only when goods dominate by value: Phase-1 qualification applies a goods-density **presence floor** (`GOODS_PRESENCE_FLOOR_BPS = 3500`) plus an early contract-value share short-circuit, and Phase-2 landed-cost synthesis applies the **goods-dominance floor** (`GOODS_DOMINANCE_FLOOR_BPS = 6000`, i.e. goods ≥ 60% of `goods + labour + permits`). The "route to review when unresolved" and "bespoke/custom flags route to review" admission notes above are backed by runtime scope-exclusion flags `out_of_scope_no_list` and `out_of_scope_bespoke` (§3.4). Recurring maintenance (e.g. the playground slice's "visible opex") is held as a separate annualised `maintenance_period_estimates` row and is **not** capitalised into the dominance denominator. The `metal_shelving_racking_trade_remedy_watchlist` split-out exists because the wood/plastic shelving slice (`install_shelving_wood_plastic`) is low-reg only when material evidence is wood or plastic; metal subfamilies carry live trade remedies and need separate customs evidence.

**Explicit no-list (defer until system proves reliable on whitelist):**

- Biocidal products and treated biocidal articles
- Medical devices
- Pharmaceuticals
- IT hardware (complex specifications)
- Vehicles
- Construction materials
- Food/perishables
- Chemicals (regulatory burden)
- Bespoke/custom-manufactured goods

**Expansion rule:** Each new category requires its own gold-set validation before entering production.

### 1.7 Project Shape

The platform is a greenfield monorepo. Layer 0 bootstraps the workspace structure that all subsequent layers build on.

**Full monorepo layout:**

```
Testproject1/
+-- apps/
|   +-- desktop/                 # Tauri desktop shell — CONTROL PANEL (L2)
|   +-- web/                     # Hosted viewing portal — READ-ONLY (L4)
+-- crates/
|   +-- daemon/                  # Rust daemon: PTY agent orchestration, SQLite,
|   |                            #   cron scheduling, Supabase sync (L1+)
+-- packages/
|   +-- contracts-schema/        # Shared TS domain types / Zod schemas (L0)
|   +-- intelligence/            # Cost-estimation, pipeline logic (L0)
|   +-- manifests/               # Markdown manifest generation (L2)
|   +-- bidding/                 # Bid workflow orchestration (L6)
|   +-- ui/                      # Shared React components (L2)
+-- skills/
|   +-- procurement-ingestion/   # Agent scraping skills for procurement sources
|   |   +-- fts/                 # Find a Tender (L1)
|   |   +-- contracts-finder/    # Contracts Finder (L2)
|   |   +-- pcs/                 # Public Contracts Scotland (L5)
|   +-- supplier-price-enrichment/
|   |   +-- ebay/                # eBay UK browser scraping (L0)
|   |   +-- amazon/              # Amazon UK (L5)
|   |   +-- alibaba/             # Alibaba.com (L5)
|   |   +-- 1688/                # 1688 wholesale (L5)
|   +-- document-acquisition/    # PDF/attachment download (L0)
|   +-- lot-extraction/          # Lot/product extraction (L0)
|   +-- product-extraction/      # Product candidate generation (L0)
|   +-- preliminary-qualification/ # Goods qualification (L0)
|   +-- manifest-generation/     # Opportunity manifest (L1)
|   +-- pdf-extraction/          # PDF semantic mapping (L1)
|   |   +-- base/                # Generic document extraction
|   |   +-- tender-specification/ # Lot/requirement extraction
|   |   +-- pricing-schedule/    # Pricing line-item extraction
|   |   +-- bill-of-quantities/  # BoQ itemized extraction
|   +-- bid-portal-*/            # Portal-specific bid automation (L6)
+-- docs/
|   +-- research/
|   +-- strategy/
+-- infra/
    +-- supabase/                # Supabase config (L4)
```

**Layer 0 creates:**

| Path | Purpose |
|------|---------|
| `package.json` | Bun workspace root |
| `bunfig.toml` | Bun runtime configuration |
| `.env.example` | Environment variable template (HMRC API keys, etc.) |
| `packages/contracts-schema/` | Shared TypeScript domain types and Zod schemas |
| `packages/intelligence/` | Intelligence pipeline logic |
| `skills/document-acquisition/` | Document download skill |
| `skills/lot-extraction/` | Lot extraction skill |
| `skills/product-extraction/` | Product candidate generation skill |
| `skills/preliminary-qualification/` | Goods qualification skill |
| `skills/supplier-price-enrichment/ebay/` | eBay marketplace search skill |

**Package boundary rules (constitutional):**

- `packages/contracts-schema` and `packages/ui` are boundary packages — they must remain free of daemon-specific or Supabase-specific implementation details
- They must NOT import `packages/db`, desktop bridge code, daemon modules, or Supabase clients
- Only procurement/supplier retrieval and workflow playbooks live under `skills/`
- The landed-cost engine lives in `packages/intelligence` because cost estimation is API-based and deterministic, not an agent-scraping workflow

### 1.8 Architectural Constraints (Non-Negotiable)

These constraints apply to ALL layers without exception. Layer 0 establishes them:

| Constraint | Source | Enforcement |
|-----------|--------|-------------|
| Authority matrix — every table classified, no mixed writes | ADR-002, §4.7 | Schema DDL enforces classification at table creation |
| Bigint pence for GBP money (`_pence` suffix, BIGINT); original-currency minor units (`_original_minor` + currency) | §5.4, §2.2 rules 1, 16, 20 | Type system + DDL |
| Integer basis points for confidence/percentage (`_bps` suffix); bounded 0–10000 vs signed/unbounded | §5.4, §2.2 rules 2, 17 | Type system + DDL |
| Cross-authority FK discipline — no SQL REFERENCES from push/remote to local-only tables | §2.2 rule 15 | DDL discipline |
| Rounding policy: aggregate first, round last; ROUND_HALF_AWAY_FROM_ZERO | §2.2 rule 18 | Calculation code |
| Skill metadata declarations (SKILL.md + schemas) | §4.10, ADR-005 | Skill authoring guide + linting |
| Conservative customs defaults (MFN, residual, out-of-quota, standard VAT) | ADR-006 | Screening-grade implementation |
| No real-money bids without human approval | ADR-007 | All layers |
| Sync tracking columns on push-owned tables from creation | §5.4 | Schema DDL |
| `pipeline_assessments.validation_state = 'unverified'` from introduction | §5.4 | Schema DDL |
| `lotId` required on `product_candidates` | §5.4 | DDL NOT NULL |
| All columns needed in later layers exist as nullable from table creation | §5.4 | DDL |
| `workflow_jobs.validation_required` default `false` from creation | §5.4 | DDL |
| Process-group supervision with cleanup verdicts | ADR-004 | From Layer 1 |
| Run manifests for every skill execution | §4.10 | From Layer 1 |
| Error manifests for every pipeline failure | §5.11 | From Layer 1 |

### 1.9 What's Stubbed in This Layer

| Feature | Layer 0 Stub | Full Implementation |
|---------|-------------|-------------------|
| Landed-cost engine | Screening-grade: HMRC Tariff API + flat freight + single FX | Layer 5: multi-source recall, 10-digit resolution, measure scanner |
| Marketplace search | Single source (eBay) | Layer 5: Alibaba/1688, Amazon |
| Skills execution | CLI invocation (`bun run skill-name`) | Layer 1: daemon-managed PTY/contract execution |
| Relation resolution | Gold set is pre-curated (no deduplication) | Layer 3: full heuristic resolution |
| Procurement sources | Gold set is pre-selected | Layer 1: first live FTS loop; Layer 2: Contracts Finder |
| Stage tracking | Implicit (no daemon) | Layer 1: explicit `stage_run_id` + stage heads |
| Sync | No sync — local SQLite only | Layer 4: push/pull sync engine |
| Validation | Fields exist (nullable), no validation system | Layer 3: dry-run; Layer 4: enforcement |
| Operator decisions | No operator surface | Layer 2: desktop control panel with local intent queue |
| Run manifests | Skill SKILL.md includes narrative instructions (preparation for L1) | Layer 1: formal daemon-managed manifest system with machine-authored fallback |

### 1.10 What Will Make or Break the Product

The critical bottleneck is the **identity chain**, not infrastructure:

1. Reliable notice ingestion via agent skills is easy compared with supplier-price normalization via agent skills.
2. Supplier-price normalization is easy compared with correct product matching.
3. Product matching is easy compared with accurate landed-cost estimation at contract scale.
4. Landed-cost estimation is easy compared with robust PTY-based agent orchestration that works every day.

**But the real bottleneck is the identity chain:** documents/lots -> candidate identity -> quoteability -> comparable listing shortlist -> price-signal usability. The architecture compresses this into concrete stages with:

- Deterministic gates at each transition
- `matchArchetype`-driven query planning (6 domain archetypes plus `unknown` fallback: `standard_class_led`, `dimension_material_led`, `electrical_spec_led`, `pack_count_led`, `compatibility_led`, `model_exact`, `unknown`)
- Four-state evidence comparison for listing screening
- Adaptive diversified shortlisting
- Compliance evidence tiers
- Dual confidence scoring (`matchConfidence` + `quoteUsabilityConfidence`) with lexicographic gating

The worker-validator architecture (Layer 3+) addresses the most insidious failure mode: schema-valid garbage from skills that navigated to the wrong page, extracted data from the wrong locale, or interpreted the wrong context. Code-based checks (Output Quality Gate) catch structural and statistical anomalies; the semantic validator catches contextual errors invisible to code.

### 1.11 Things NOT Implemented in Layer 0

The following are designed in the research document but NOT active in this layer. They exist only as forward-compatible schema provisions (nullable columns, table shells):

- Rust daemon (Layer 1)
- Process-group supervision (Layer 1)
- Run manifests (Layer 1 — SKILL.md in L0 includes narrative preparation)
- Error manifests (Layer 1)
- Stage tracking with `stage_run_id` (Layer 1 — column exists, not actively managed)
- Cron scheduling (Layer 3)
- Resource arbiter (Layer 3)
- Worker-validator system (Layer 3 dry-run, Layer 4 enforcement)
- Sync engine (Layer 4)
- Multi-source customs recall (Layer 5 — MVP starts screening-grade)
- CBAM carbon border adjustment (forward placeholder only)
- Multi-user / RBAC (not in MVP)
- Multi-executor support (MVP = single daemon per org)

---

## 2. Database Schema (Full DDL)

> **Authority:** This section is the constitutional schema for the entire platform. Every table, every column, every constraint is defined here. Implementation agents copy-paste these statements. No table may be invented later, reclassified, or have columns renamed in later layers. Later layers may first materialize or activate already-specified dormant tables, but they do so using the shapes reserved here.
>
> **Dialect:** SQL is written in Postgres syntax targeting the Supabase remote database. The local SQLite schema mirrors these definitions with type substitutions documented in 2.1. An implementation agent produces both from this single source.

### 2.1 SQLite Compatibility Notes

The platform uses dual databases: local SQLite (operational authority) and Supabase/Postgres (shared authority + projections). The SQL below is Postgres-canonical. SQLite mirrors with these substitutions:

| Postgres | SQLite | Notes |
|----------|--------|-------|
| `uuid` | `TEXT` | Application-generated via Rust `uuid` crate or `crypto.randomUUID()` in TS |
| `gen_random_uuid()` | *(none)* | Application-side UUID generation; no DB default |
| `timestamptz` | `TEXT` | ISO 8601 format (`2026-03-19T14:30:00.000Z`) |
| `jsonb` | `TEXT` | JSON stored as text; `json_extract()` / `json_each()` via json1 extension |
| `boolean` | `INTEGER` | 0 / 1 |
| `numeric` | `REAL` | For exchange rates and quantities only (never money) |
| `bigint` | `INTEGER` | SQLite integers are 64-bit |
| `serial` / `autoincrement` | `INTEGER PRIMARY KEY AUTOINCREMENT` | Only for `local_outbox.local_seq` |
| `now()` | *(none)* | Application-side timestamp; no DB default |
| `CHECK` constraints | Supported | SQLite enforces CHECK constraints |
| `REFERENCES` (FK) | Supported | Requires `PRAGMA foreign_keys = ON` |

**SQLite operational settings:**

```sql
PRAGMA journal_mode = WAL;          -- concurrent reads during writes
PRAGMA foreign_keys = ON;           -- enforce referential integrity
PRAGMA busy_timeout = 5000;         -- 5s wait on lock contention
PRAGMA synchronous = NORMAL;        -- WAL-safe durability
```

**Key implications:**
- No native array types — use junction tables (e.g., `notice_cpv_codes`) or JSON TEXT columns for variable-shape data never queried by DB
- Single-writer model suits single-daemon architecture (one process writes, readers use WAL snapshots)
- Type affinity: TEXT for UUIDs, INTEGER for boolean/pence/bps, TEXT for timestamps
- All money in integer pence (`_pence` suffix, INTEGER type) — never floating point
- All confidence/percentage values in integer basis points (`_bps` suffix, INTEGER 0-10000)

### 2.2 Constitutional Schema Rules

These rules are permanent. No layer may violate them.

| # | Rule | Enforcement |
|---|------|------------|
| 1 | Bigint pence for all GBP money | `_pence` suffix, BIGINT type. Non-GBP originals use `_original_minor` suffix + currency code (rule 16). |
| 2 | Integer basis points for confidence/percentage | `_bps` suffix, INTEGER. **Bounded** (0–10000): confidence, coverage, recovery rates. **Signed/unbounded**: margins, deltas, spreads (rule 17). |
| 3 | Exchange rates use NUMERIC (ratios, not money) | No `_pence` suffix; `numeric` type. All calculations involving FX must use scaled integer arithmetic internally, not floating point. |
| 4 | All push-owned tables carry sync tracking columns | `owner_id`, `row_version`, `last_local_seq`, `stage_run_id` (pipeline tables), `created_at`, `updated_at` — populated from first write |
| 5 | `pipeline_assessments.validation_state` default `'unverified'` | From creation |
| 6 | `workflow_jobs.validation_required` default `false` | From creation |
| 7 | `lot_id` NOT NULL on `product_candidates` | DDL constraint |
| 8 | All future-layer columns exist as nullable from creation | No column adds in later layers |
| 9 | No table reclassification | Authority permanent from introduction |
| 10 | No column renames | Stable across all layers |
| 11 | Junction tables introduced with parent tables | Created together |
| 12 | JSON TEXT only for variable-shape data never queried by DB | Relational for anything filtered/joined/indexed |
| 13 | No mixed-write tables | Every table has exactly one authority owner |
| 14 | Remote-owned tables carry tombstone support | `deleted_at` column for soft-delete semantics |
| 15 | Cross-authority FK discipline | Push-owned and remote-owned tables must NOT use SQL `REFERENCES` to local-only tables. Cross-authority references are opaque IDs. Local-only FK enforcement is optional in SQLite only. |
| 16 | Original-currency amounts use minor units | `*_original_minor` suffix + separate `currency` column. Only GBP-normalized amounts use `*_pence`. |
| 17 | BPS constraint families | **Bounded** (0–10000): confidence, coverage, recovery rates — use CHECK(x >= 0 AND x <= 10000). **Signed/unbounded**: margins, deltas, spreads — may be negative or >10000, no CHECK. |
| 18 | Rounding policy: aggregate first, round last | Intermediate money calculations use full-precision scaled integers. Round to whole pence only at persisted estimate boundaries. `ROUND_HALF_AWAY_FROM_ZERO`. |
| 19 | Junction tables are parent-embedded sync sets | Junction tables sync as part of their parent row's outbox entry, not independently. No standalone sync tracking columns required. |
| 20 | Money columns use BIGINT not INTEGER | All `_pence` and `_minor` columns, plus `row_version`, `last_local_seq`, sequence counters use `bigint` to handle framework ceilings (>£21.4M) and high-volume sequences. |

### 2.2.1 Scope of "All Columns From Creation"

The no-ALTER-TABLE principle (rule 8) eliminates the worst class of migration pain: late column additions to hot synced tables. It does **not** eliminate:

- Index migrations (new query paths require new indices)
- Check constraint additions
- View changes (current-state views must evolve with pipeline logic)
- Backfill operations (populating nullable columns when a layer activates them)
- RLS / RPC / sync protocol changes
- Semantic migrations (a column's meaning can evolve even if its name doesn't)
- Codegen / type wrapper migrations

The correct framing: freeze **logical schema slots** early, but treat **activation, constraints, views, and projections** as first-class migration surfaces.

**Retroactive constitutional additions:** `tender_document_sets.extraction_state` was added to the constitutional shape alongside the PDF extraction toolkit introduction, following the same pattern as `workflow_jobs.validation_required` (forward-compatibility provision included at table creation time).

**Schema evolution beyond L0 (goods-led labour-cost extension, epoch 10):** The no-ALTER principle held through L0–L4, but the goods-led supply-and-install / maintenance expansion (shipped 2026-06-04) performed a genuine **additive** schema migration — `migrations/0014_installation_labour_fields.sql` adds the 9 labour/permit/goods columns to `landed_cost_estimates`, the 6 coverage / value-share columns to `pipeline_assessments`, and creates `maintenance_period_estimates`, bumping the schema epoch 9 → 10 (mirrored to Supabase by `infra/supabase/migrations/0010_installation_labour_projection.sql`). All additions are nullable / default-0 and therefore back-compatible. The DDL above shows these columns/tables inline with `-- added at epoch 10` comments so this file remains the complete schema record; the canonical migration provenance is layer-strategy §9 (Schema Stability Matrix) and L5 §1.17.

### 2.3 Authority Classification

Every table has exactly one authority classification. This is permanent — no reclassification across layers.

**Push-owned pipeline tables** (daemon writes locally, pushes to Supabase):

| # | Table | Scope | Notes |
|---|-------|-------|-------|
| 1 | `procurement_processes` | Process | Canonical process entity |
| 2 | `contract_notices` | Notice | Source-specific notice records |
| 3 | `notice_lots` | Lot | Per-notice lot decomposition |
| 4 | `tender_document_sets` | Process | Acquired document set per notice |
| 5 | `qualification_runs` | Lot | Per-lot qualification results |
| 6 | `product_candidates` | Lot | Extracted product candidates |
| 7 | `search_profiles` | Candidate | Structured search profile per candidate |
| 8 | `search_attempts` | Candidate | Per-wave search execution records |
| 9 | `supplier_quotes` | Candidate | Marketplace/RFQ quotes |
| 10 | `candidate_pricing_assessments` | Candidate | Gross margin gate results |
| 11 | `landed_cost_estimates` | Candidate-quote | Per-candidate-quote cost estimate |
| 12 | `manifests` | Process | Generated opportunity manifests |
| 13 | `pipeline_assessments` | Process | Process-level pipeline rollup |
| 14 | `bid_workflow_states` | Process | Bid automation state |
| 15 | `process_stage_status` | Scoped | Per-stage execution records |
| 16 | `process_stage_heads` | Scoped | Current stage head per scope |
| 17 | `document_extractions` | Notice | Structural extraction output from Docling (L1-introduced via `0006_pdf_extraction_toolkit.sql`) |
| 18 | `document_semantic_extractions` | Notice | Semantic mapping output from pdf-extraction skills (L1-introduced via `0006_pdf_extraction_toolkit.sql`) |
| +1 | `maintenance_period_estimates` | Candidate-quote | Per-period maintenance estimate; **goods-led extension, added at epoch 10 via `migrations/0014`** (the 50th table — see count note below). Push-owned. |

> **Table count:** the original constitutional inventory is **49 tables**. The goods-led labour-cost extension adds one persisted table (`maintenance_period_estimates`) at epoch 10, bringing the operational total to **50**. References elsewhere to "49 tables" describe the pre-extension constitutional inventory.

**Push-owned infrastructure tables:**

| # | Table | Notes |
|---|-------|-------|
| 19 | `executors` | Daemon identity (single row in MVP) |
| 20 | `process_relations` | Inter-process relationship edges |
| 21 | `process_aliases` | Merged-process alias pointers |

**Push-owned junction tables:**

| # | Table | Notes |
|---|-------|-------|
| 22 | `notice_cpv_codes` | Notice-level CPV codes |
| 23 | `product_commodity_codes` | Candidate commodity codes |
| 24 | `lot_cpv_codes` | Lot-level CPV codes |

**Push-owned projection tables** (Supabase-only, for web viewer visibility):

| # | Table | Notes |
|---|-------|-------|
| 25 | `workflow_job_summaries` | Redacted job status for web |
| 26 | `error_manifest_summaries` | Redacted error info for web |
| 27 | `validation_verdict_summaries` | Verdict summary for web |
| 28 | `viewer_sync_status` | Daemon health for web |

**Local-only tables** (SQLite only, never synced):

| # | Table | Notes |
|---|-------|-------|
| 29 | `workflow_jobs` | Job orchestration (summaries pushed separately) |
| 30 | `agent_session_logs` | Full session logs (redacted summaries pushed) |
| 31 | `raw_documents` | Fetched document payloads |
| 32 | `error_manifests` | Full error context (summaries pushed separately) |
| 33 | `source_cursors` | Source pagination cursors |
| 34 | `sources` | Local copy of source definitions |
| 35 | `source_runtime_state` | Circuit breaker / health state |
| 36 | `listing_evidence` | Cached marketplace listing data |
| 37 | `validation_verdicts` | Full validator verdicts |
| 38 | `pending_operator_intents` | Queued operator mutations during degraded sync |
| 39 | `local_outbox` | Transactional outbox for push sync (shape reserved here; L4 materializes the SQLite table) |
| 40 | `sync_batches` | Push batch tracking |
| 41 | `pull_cursors` | Pull cursor per remote domain |
| 42 | `gold_set_cases` | Gold-set corpus cases with human labels |
| 43 | `evaluation_runs` | Evaluation run metadata |
| 44 | `evaluation_case_results` | Per-case results within runs |
| 45 | `evaluation_metric_summaries` | Aggregate metrics per run |

**Remote-owned tables** (Supabase authoritative, daemon pulls):

| # | Table | Layer | Notes |
|---|-------|-------|-------|
| 46 | `operator_decision_heads` | L4 | Human operator dispositions (CAS writes) |
| 47 | `shared_company_profile` | L4 | Company profile data |
| 48 | `shared_source_configs` | L4 | Source configuration (authoritative copy) |

**Supabase-only infrastructure:**

| # | Table | Notes |
|---|-------|-------|
| 49 | `schema_metadata` | Schema epoch tracking |

### 2.4 Sync Tracking Column Patterns

Two patterns exist depending on authority direction.

**Pattern 1 — Push-owned tables** (daemon writes, pushes to Supabase):

```sql
-- Present on ALL push-owned tables
owner_id text not null,              -- executor that owns this row
row_version bigint not null default 1,  -- monotonic version counter, incremented on every write
last_local_seq bigint,              -- outbox sequence number of last push (NULL until first push)
created_at timestamptz not null default now(),
updated_at timestamptz not null default now()

-- Present on push-owned PIPELINE tables only (not infrastructure/projections)
stage_run_id text,                   -- atomic commit cohort for one scoped unit of work
```

**Pattern 2 — Remote-owned tables** (Supabase authoritative, daemon pulls):

```sql
-- Present on ALL remote-owned tables
remote_change_seq bigint,            -- Supabase-assigned change sequence for pull ordering
remote_revision bigint not null default 1,  -- for compare-and-swap (CAS) writes
deleted_at timestamptz,              -- tombstone for soft-delete semantics
created_at timestamptz not null default now()
```

### 2.5 Full CREATE TABLE Statements

All tables grouped by category. SQL is in Postgres syntax. SQLite mirrors with type substitutions from 2.1.

#### 2.5.1 Infrastructure Tables

```sql
-- Local runtime copy of source definitions. Populated from shared_source_configs
-- during pull sync. Exists locally so raw_documents, source_cursors, and
-- contract_notices can have local FK references without network dependency.
-- The authoritative version is shared_source_configs (remote-owned, pull-only).
-- Authority: LOCAL-ONLY
create table sources (
  id text primary key,
  kind text not null,
  enabled boolean not null default true,
  config jsonb not null,
  created_at timestamptz not null default now()
);

-- Authority: LOCAL-ONLY
create table source_cursors (
  source_id text not null references sources(id),
  cursor_key text not null,
  cursor_value text,
  updated_at timestamptz not null default now(),
  primary key (source_id, cursor_key)
);

-- Authority: LOCAL-ONLY
create table source_runtime_state (
  source_id text primary key references sources(id),
  last_health_state text not null default 'unknown',  -- epistemic: not yet assessed, not optimistic
  challenge_event_count integer not null default 0,
  challenge_window_started_at timestamptz,
  circuit_opened_at timestamptz,
  cooldown_until timestamptz,
  last_canary_at timestamptz,
  last_error_code text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Authority: LOCAL-ONLY
create table raw_documents (
  id uuid primary key default gen_random_uuid(),
  source_id text not null references sources(id),
  fetched_at timestamptz not null default now(),
  content_type text,
  source_url text,
  payload_json jsonb,
  storage_path text,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  check (payload_json is not null or storage_path is not null)
);

-- Single-daemon MVP: one row representing the local daemon.
-- Multi-executor support is a future-consideration extension.
-- Authority: PUSH-OWNED
create table executors (
  id uuid primary key default gen_random_uuid(),
  device_name text not null,
  status text not null,
  last_heartbeat_at timestamptz not null default now(),
  -- sync tracking (push-owned: heartbeat summary pushed to remote)
  owner_id text not null,  -- self-referential for consistency
  row_version bigint not null default 1,
  last_local_seq bigint,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

#### 2.5.2 Pipeline Tables

```sql
-- Authority: PUSH-OWNED
create table procurement_processes (
  id uuid primary key default gen_random_uuid(),
  canonical_ocid text,
  primary_notice_id uuid,
  canonical_process_id uuid,  -- cluster identity root; set to own id on first write, updated on merge
  current_stage text not null,
  buyer_name text,
  title text not null,
  latest_deadline_at timestamptz,
  latest_published_at timestamptz not null,
  latest_updated_at timestamptz not null,
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Authority: PUSH-OWNED
create table contract_notices (
  id uuid primary key default gen_random_uuid(),
  process_id uuid references procurement_processes(id),
  source_id text not null,  -- local-only FK: sources (enforced in SQLite only)
  source_notice_id text not null,
  raw_document_id uuid not null,  -- local-only FK: raw_documents
  ocid text,
  title text not null,
  summary text,
  buyer_name text,
  procurement_stage text not null,
  access_route text,  -- open_tender, restricted_tender, framework_call_off, dynamic_market, direct_award, closed_framework_award, not_bidable
  deadline_at timestamptz,
  published_at timestamptz not null,
  source_updated_at timestamptz not null,
  main_category text,
  -- cpv_codes: use junction table notice_cpv_codes instead of array column
  estimated_value_original_minor bigint,   -- original currency minor units (fen, cents, etc.)
  estimated_value_currency text,
  estimated_value_gbp_ex_vat_pence bigint,  -- GBP integer pence
  estimated_value_vat_inclusive boolean,
  normalized_payload jsonb not null,
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now(),
  unique (source_id, source_notice_id)
);

-- Lot-level decomposition of notices.
-- If no explicit lots, a synthetic whole-notice lot is created.
-- Authority: PUSH-OWNED
create table notice_lots (
  id uuid primary key default gen_random_uuid(),
  notice_id uuid not null references contract_notices(id),
  source_lot_id text,
  lot_key text,                       -- deterministic natural key for rerun diffing and gold-set scoring
  lot_number integer,
  title text,
  description text,
  estimated_value_pence bigint,  -- integer pence
  deadline_at timestamptz,
  is_bidable_separately boolean not null default true,
  documents jsonb,  -- opaque array of document references
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Authority: PUSH-OWNED
create table tender_document_sets (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  document_set_hash text not null,
  completeness_state text not null,  -- none, partial, full
  extraction_state text not null default 'none',  -- none, partial, complete, failed (forward-compatibility: populated from L1 when PDF extraction is active)
  document_roles_json jsonb not null,
  documents_json jsonb not null,
  acquired_at timestamptz not null,
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Authority: PUSH-OWNED
create table qualification_runs (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  lot_id uuid not null references notice_lots(id),
  expiry_qualified boolean not null,
  goods_qualified boolean not null,
  goods_confidence_bps integer not null,  -- basis points (0-10000)
  legal_category text,            -- from procurement pipeline review
  goods_density_score_bps integer,  -- basis points (0-10000)
  goods_separability text,        -- true, false, uncertain (tri-state classification stored as text)
  qualification_basis text not null,  -- notice_only, notice_plus_docs, lot_level
  addressable_value_pence bigint,
  access_route text,              -- open_tender, restricted_tender, etc.
  rationale jsonb not null,       -- renamed from 'evidence' to match TS interface
  qualified_at timestamptz not null default now(),
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Authority: PUSH-OWNED
-- Constitutional rule: lot_id NOT NULL
create table product_candidates (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  lot_id uuid not null references notice_lots(id),
  parent_candidate_id uuid references product_candidates(id),
  candidate_key text,                 -- deterministic natural key for rerun diffing and gold-set scoring
  candidate_role text not null default 'standalone',  -- standalone, bundle_parent, bundle_component
  name text not null,
  candidate_type text not null,  -- exact_sku, standard_commodity, brand_family, bundle, custom_or_fabricated, installed_goods, regulated_goods, unknown
  match_archetype text not null, -- standard_class_led, dimension_material_led, electrical_spec_led, pack_count_led, compatibility_led, model_exact, unknown
  head_noun text not null,
  quantity numeric,
  quantity_unit text,
  specification_text text,
  estimated_unit_weight_kg numeric,
  estimated_unit_volume_m3 numeric,
  candidate_value_share_bps integer,
  packing_hint text,              -- flat_pack, assembled, mixed, unknown
  handling_restrictions_json jsonb,
  material text,
  brand_model text,
  -- candidate_commodity_codes: use junction table product_commodity_codes instead of array column
  critical_attributes_json jsonb,
  numeric_specs_json jsonb,
  standards_json jsonb,
  preferred_terms_json jsonb,
  excluded_terms_json jsonb,
  source_evidence_json jsonb,
  quoteability_score_bps integer,   -- basis points (0-10000)
  recall_risk_score_bps integer,    -- basis points (0-10000)
  identity_confidence_bps integer,  -- basis points (0-10000)
  spec_confidence_bps integer,      -- basis points (0-10000)
  quantity_confidence_bps integer,  -- basis points (0-10000)
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Structured search profile per product candidate.
-- Persisted for debugging and skill improvement.
-- Authority: PUSH-OWNED
create table search_profiles (
  id uuid primary key default gen_random_uuid(),
  product_candidate_id uuid not null references product_candidates(id),
  head_noun text not null,
  head_noun_family text,
  candidate_type text not null,
  match_archetype text not null,
  critical_attributes_json jsonb,
  standards_json jsonb,
  numeric_specs_json jsonb,
  noun_aliases_en jsonb,
  noun_aliases_zh jsonb,
  market_frame_en jsonb,
  market_frame_zh jsonb,
  use_context text,
  sector_context text,
  required_anchors jsonb,
  optional_anchors jsonb,
  must_not_have jsonb,
  preferred_terms jsonb,
  excluded_terms jsonb,
  query_waves_json jsonb not null,
  recall_risk_score_bps integer,
  search_eligibility_state text not null,  -- auto_search, manual_only, skip
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Authority: PUSH-OWNED
create table search_attempts (
  id uuid primary key default gen_random_uuid(),
  product_candidate_id uuid not null references product_candidates(id),
  search_profile_id uuid not null references search_profiles(id),
  source text not null,
  locale text not null,
  wave_number integer not null,
  query_text text not null,
  filters_json jsonb,
  is_recovery_wave boolean not null default false,
  result_count integer not null default 0,
  hard_rejected_count integer not null default 0,
  shortlisted_count integer not null default 0,
  search_failure_mode text,
  session_id uuid,  -- local-only FK: agent_session_logs
  executed_at timestamptz not null default now(),
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Authority: PUSH-OWNED
create table supplier_quotes (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  product_candidate_id uuid references product_candidates(id),
  search_attempt_id uuid references search_attempts(id),
  product_name text not null,                 -- denormalized convenience copy for display/debugging
  region text not null,
  quote_class text not null,
  source_name text not null,
  source_url text,
  source_listing_id text,
  ship_from_country text,
  country_of_origin text,
  country_of_dispatch text,
  incoterm text,
  quote_cost_boundary text,
  quoted_quantity numeric,
  quoted_quantity_unit text,
  unit_price_original_minor bigint not null,   -- per-unit price in original currency minor units
  currency text not null,
  unit_price_gbp_pence bigint not null,        -- per-unit GBP pence (may be 0 for sub-penny items)
  offer_total_original_minor bigint,           -- total offer price in original currency minor units
  offer_total_gbp_pence bigint,               -- total offer price in GBP pence
  price_basis_qty numeric,                     -- quantity the offer covers (e.g., 5000 gloves)
  price_basis_unit text,                       -- unit for basis qty (e.g., 'glove')
  minimum_order_quantity numeric,
  lead_time_days integer,
  includes_shipping boolean not null default false,
  -- cross-domain fields (procurement pipeline review)
  price_signal_role text not null,  -- benchmark_ceiling, optimistic_floor, executable
  listing_pack_qty numeric,
  listing_pack_unit text,
  normalized_unit_basis text,
  is_used_or_refurbished boolean not null default false,
  is_ready_to_ship boolean,
  vat_inclusive boolean,
  compliance_evidence_tier text,
  raw_match_score_bps integer,
  match_confidence_bps integer,     -- basis points (0-10000)
  quote_usability_confidence_bps integer,  -- basis points (0-10000)
  shipping_basis_confidence_bps integer,   -- basis points (0-10000)
  -- cross-domain fields (data architecture review)
  quote_realism_confidence_bps integer,  -- basis points (0-10000)
  negotiation_uplift_bps integer,        -- basis points
  evidence_states_json jsonb,
  query_wave_number integer,
  evidence jsonb not null,
  valid_until timestamptz,
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Authority: PUSH-OWNED
create table candidate_pricing_assessments (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  product_candidate_id uuid not null references product_candidates(id),
  selected_quote_id uuid not null references supplier_quotes(id),
  selected_quote_row_version bigint not null,
  quote_cost_boundary text not null,
  revenue_basis_type text not null,
  expected_addressable_revenue_ex_vat_pence bigint not null,
  revenue_confidence_bps integer not null,
  gross_margin_bps integer not null,
  gross_estimate_confidence_bps integer not null,
  identity_gate_passed boolean not null,
  usability_gate_passed boolean not null,
  pricing_input_confidence_bps integer not null,
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Authority: PUSH-OWNED
create table landed_cost_estimates (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  product_candidate_id uuid not null references product_candidates(id),
  supplier_quote_id uuid not null references supplier_quotes(id),
  shipping_mode text not null,
  mode_confidence_bps integer not null,            -- basis points (0-10000)
  packing_scenario text,
  declarable_commodity_code_10 text,
  candidate_commodity_codes_json jsonb,
  additional_codes jsonb,
  additional_code_requirements_json jsonb,
  supplementary_units_required_json jsonb,
  commodity_confidence_bps integer not null,        -- basis points (0-10000)
  classification_stability_bps integer not null,   -- basis points (0-10000)
  customs_valuation_method text,                   -- 1_actual, 1_proxy, 2, 3, 4, 5, 6, unknown
  valuation_mode text,                            -- estimation, declaration
  customs_fx_rate numeric not null,                -- exchange rate (not money -- keep numeric)
  commercial_fx_rate numeric not null,             -- exchange rate (not money -- keep numeric)
  fx_basis_delta_bps integer not null,             -- basis points
  fx_rate_checked_at timestamptz,
  supplier_cost_gbp_pence bigint not null,        -- integer pence
  freight_low_gbp_pence bigint not null,          -- integer pence
  freight_base_gbp_pence bigint not null,         -- integer pence
  freight_high_gbp_pence bigint not null,         -- integer pence
  customs_duty_gbp_pence bigint not null,         -- integer pence
  trade_remedy_duty_gbp_pence bigint,             -- integer pence
  duty_outcome_min_gbp_pence bigint,
  duty_outcome_max_gbp_pence bigint,
  customs_cost_spread_bps integer,
  trade_remedy_state text,
  trade_remedy_cases_json jsonb,
  duty_breakdown_json jsonb,
  measure_interaction_json jsonb,
  exporter_rate_basis text,
  exporter_identity_confidence_bps integer,
  origin_evidence_state text,
  import_vat_gbp_pence bigint not null,           -- integer pence
  vat_rate_state text,
  import_vat_treatment text not null,              -- postponed_full, postponed_partial, cash_paid_full, cash_paid_partial, non_recoverable, unknown
  vat_recovery_rate_bps integer,                   -- basis points
  uk_final_mile_cost_gbp_pence bigint not null,   -- integer pence
  other_fees_gbp_pence bigint not null,           -- integer pence
  economic_landed_cost_gbp_pence bigint not null,  -- integer pence
  cash_outlay_gbp_pence bigint not null,          -- integer pence
  cash_to_clear_gbp_pence bigint not null,        -- integer pence
  working_capital_cost_gbp_pence bigint,          -- integer pence
  revenue_basis_type text not null,
  revenue_confidence_bps integer not null,          -- basis points (0-10000)
  customs_resolution_confidence_bps integer,
  commercial_estimate_confidence_bps integer,
  net_margin_low_bps integer not null,              -- basis points
  net_margin_base_bps integer not null,             -- basis points
  net_margin_high_bps integer not null,             -- basis points
  decision_grade text not null,
  confidence_bps integer not null,                  -- basis points (0-10000)
  confidence_breakdown jsonb,
  warnings jsonb not null,
  blockers jsonb not null default '[]',
  tariff_checked_at timestamptz,
  measures_fresh_until timestamptz,
  classification_fresh_until timestamptz,
  trade_remedy_watch_state text,
  cbam_exposure_flag boolean,
  cbam_estimate_gbp_pence bigint,
  -- goods-led install/maintenance extension (added at epoch 10 by migrations/0014_installation_labour_fields.sql; see §2.2.1 and L5 §1.17)
  goods_landed_cost_gbp_pence bigint,             -- goods-only landed-cost subtotal (makes goods value-share reconstructable from one row)
  installation_labour_cost_gbp_pence bigint,      -- integer pence (base; summed into economic_landed_cost_gbp_pence)
  installation_labour_low_gbp_pence bigint,       -- integer pence (banded low)
  installation_labour_high_gbp_pence bigint,      -- integer pence (banded high)
  installation_labour_confidence_bps integer,     -- basis points (0-10000)
  permit_traffic_mgmt_cost_gbp_pence bigint,      -- integer pence (base; summed into economic_landed_cost_gbp_pence)
  permit_traffic_mgmt_low_gbp_pence bigint,       -- integer pence (banded low)
  permit_traffic_mgmt_high_gbp_pence bigint,      -- integer pence (banded high)
  permit_responsibility text,                     -- who holds permit/TM responsibility; unknown → conservative high band
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Goods-led install/maintenance extension: per-period maintenance estimate.
-- Added at epoch 10 by migrations/0014_installation_labour_fields.sql (see §2.2.1 and L5 §1.17).
-- Recurring maintenance opex is held here as a separate annualised estimate and is
-- NEVER capitalised into the goods value-share dominance denominator.
-- Authority: PUSH-OWNED
create table maintenance_period_estimates (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  product_candidate_id uuid not null references product_candidates(id),
  landed_cost_estimate_id uuid not null references landed_cost_estimates(id),
  supplier_quote_id uuid references supplier_quotes(id),
  visit_scope text,
  visits_per_year integer not null default 0,
  term_years integer not null default 0,
  per_visit_labour_cost_gbp_pence bigint not null default 0,    -- integer pence
  annualised_opex_gbp_pence bigint not null default 0,          -- integer pence
  total_visible_term_opex_gbp_pence bigint not null default 0,  -- integer pence
  maintenance_confidence_bps integer not null default 0,        -- basis points (0-10000)
  margin_signal text,
  warnings jsonb not null default '[]',
  evidence_json jsonb not null default '{}',
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now(),
  unique (product_candidate_id, landed_cost_estimate_id)
);

-- Authority: PUSH-OWNED
create table manifests (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  markdown text not null,
  artifact_hash text,                  -- SHA-256 of manifest artifact for integrity / staleness
  artifact_locator text,               -- portable storage key (not filesystem path)
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Pipeline-computed assessment (daemon-owned, push-only).
-- One row per process — the process-level rollup of all stage outcomes.
-- Authority: PUSH-OWNED
create table pipeline_assessments (
  process_id uuid primary key references procurement_processes(id),
  pipeline_state text not null,  -- discovered, normalized, relation_resolved, prelim_rejected, prelim_qualified, docs_acquired, lots_extracted, products_extracted, quoteability_scored, priced, gross_qualified, landed_costed, net_qualified
  review_required boolean not null default false,
  availability_status text not null default 'active',  -- active, expired, cancelled
  validation_state text not null default 'unverified', -- validated, flagged, unverified
  gross_margin_bps integer,      -- basis points (1500 = 15.00%)
  net_margin_bps integer,        -- basis points
  priced_coverage_bps integer,
  landed_cost_coverage_bps integer,
  net_qualified_coverage_bps integer,
  priced_value_coverage_bps integer,          -- value-weighted: sum of priced lot values / total qualifying value
  landed_cost_value_coverage_bps integer,     -- value-weighted landed-cost coverage
  net_qualified_value_coverage_bps integer,   -- value-weighted net-qualified coverage
  unresolved_mandatory_lots integer not null default 0,
  decision_basis_hash text,
  review_stage text,
  review_reason text,
  -- goods-led install/maintenance extension (added at epoch 10 by migrations/0014_installation_labour_fields.sql; see §2.2.1 and L5 §1.17)
  goods_value_share_bps integer,                       -- goods ÷ (goods + labour + permits), gated by GOODS_DOMINANCE_FLOOR_BPS (6000)
  goods_value_share_basis text,                        -- exact values: cost_proxy | contract_value (contract_value preferred when available)
  installation_labour_coverage_bps integer,            -- lot-count coverage of labour-estimated lots
  installation_labour_value_coverage_bps integer,      -- value-weighted labour coverage
  permit_traffic_mgmt_coverage_bps integer,            -- lot-count coverage of permit/TM-estimated lots
  permit_traffic_mgmt_value_coverage_bps integer,      -- value-weighted permit/TM coverage
  assessed_at timestamptz not null default now(),
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Bid workflow state (daemon-owned, push-only).
-- Authority: PUSH-OWNED
create table bid_workflow_states (
  process_id uuid primary key references procurement_processes(id),
  bid_state text not null default 'not_started',  -- not_started, ready_to_bid, bid_in_progress, draft_saved, bid_submitted, bid_failed, portal_state_unknown
  bid_scope_json jsonb,
  review_required boolean not null default false,
  bid_job_id uuid,  -- local-only FK: workflow_jobs
  started_at timestamptz,
  completed_at timestamptz,
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

#### 2.5.3 Stage Tracking Tables

```sql
-- Per-stage execution records. Each row represents one execution of one stage
-- at one scope grain. stage_run_id is the atomic commit cohort.
-- Authority: PUSH-OWNED
create table process_stage_status (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  stage text not null,              -- ingest, normalize, relate, prequalify_notice, qualify_lot, acquire_docs, extract_lots, extract_products, score_quoteability, price_signal_match, gross_margin_gate, landed_cost, net_margin_gate, generate_manifest, operator_decision, bid_workflow
  scope_kind text not null,         -- process, notice, lot, candidate, candidate_source_pair, candidate_quote_scenario
  scope_id text not null,             -- canonical format defined per scope_kind (see layer-strategy.md §10)
  stage_run_id text not null,
  state text not null,
  freshness_state text not null default 'fresh', -- fresh, soft_stale, hard_invalid
  basis_hash text,
  input_stage_vector_json jsonb,
  started_at timestamptz not null,
  completed_at timestamptz,
  error_manifest_id uuid,  -- local-only FK: error_manifests
  input_revision integer,
  output_revision integer,
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now(),
  unique (process_id, stage, scope_kind, scope_id, stage_run_id)
);

-- Current stage head per (process, stage, scope) tuple.
-- Updated on each stage completion to point to the latest stage_run_id.
-- Authority: PUSH-OWNED
create table process_stage_heads (
  process_id uuid not null references procurement_processes(id),
  stage text not null,
  scope_kind text not null,
  scope_id text not null,
  current_stage_run_id text not null,
  freshness_state text not null default 'fresh',
  basis_hash text,
  input_stage_vector_json jsonb,
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now(),
  primary key (process_id, stage, scope_kind, scope_id)
);
```

#### 2.5.4 Orchestration Tables

```sql
-- Job orchestration row. Local-only — mirrored to workflow_job_summaries for remote visibility.
-- Authority: LOCAL-ONLY
create table workflow_jobs (
  id uuid primary key default gen_random_uuid(),
  kind text not null,            -- sync_source, normalize_notice, resolve_relations, prequalify_notice, qualify_lot, acquire_documents, extract_document_structure, extract_document_semantics, extract_lots, extract_products, score_quoteability, enrich_supplier_prices, compute_gross_gate, estimate_landed_cost, compute_net_gate, generate_manifest, record_operator_decision, run_bid_workflow, escalate_rfq, validation
  process_id uuid references procurement_processes(id),
  notice_id uuid references contract_notices(id),
  scope_kind text,               -- process, notice, lot, candidate, candidate_source_pair, candidate_quote_scenario
  scope_id text,
  parent_job_id uuid references workflow_jobs(id),  -- links validator sub-job to worker job
  continuation_chain_id text,
  chain_closed boolean not null default false,
  validation_required boolean not null default false,  -- set by Validation Policy Engine
  validation_verdict_id uuid,    -- populated after validation completes (references validation_verdicts.id)
  status text not null,
  scheduled_for timestamptz,
  lease_owner uuid references executors(id), -- authoritative in local SQLite; mirrored to Supabase for visibility only
  lease_expires_at timestamptz, -- authoritative in local SQLite; mirrored to Supabase for visibility only
  attempt_count integer not null default 0,
  input_payload jsonb not null,
  output_payload jsonb,
  last_error text,
  started_at timestamptz,
  completed_at timestamptz,
  created_at timestamptz not null default now()
);

-- Full local session log row. Local-only — remote gets summaries/redactions.
-- Authority: LOCAL-ONLY
create table agent_session_logs (
  id uuid primary key default gen_random_uuid(),
  job_id uuid references workflow_jobs(id),
  skill_path text not null,
  skill_version text,
  -- execution identity (from agent orchestration review)
  execution_mode text not null default 'pty',  -- 'pty' or 'contract'
  cli_vendor text,
  cli_version text,
  adapter_version text,
  config_hash text,
  -- process supervision (from agent orchestration review)
  pty_session_id text not null,
  pid integer,
  pgid integer,
  started_at timestamptz not null,
  completed_at timestamptz,
  status text not null,  -- created, spawned, active, terminating, exited, reaped, archived
  -- raw session output (artifact-based; legacy column names retained)
  terminal_output_path text,   -- artifact file path for PTY bytes or contract-mode stdout/stderr (local only; not synced)
  terminal_output_hash text,   -- SHA-256 of raw session output
  terminal_output_bytes integer,
  terminal_output_tail text,   -- last N KB of raw session output for quick inspection (local only; Supabase gets redacted excerpt or null)
  structured_trace jsonb,      -- full parsed trace locally; Supabase stores only redacted summaries
  -- error handling (expanded taxonomy from agent orchestration review)
  error_type text,  -- AGENT_AUTH_CHALLENGE, AGENT_INTERNAL_ERROR, AGENT_STALLED, AGENT_SKILL_FAILED,
                    -- AGENT_OUTPUT_CONTRACT_INVALID, AGENT_PROTOCOL_MISMATCH,
                    -- AGENT_PARTIAL_FAILURE, AGENT_CLEANUP_INCOMPLETE, AGENT_RUN_FAILED,
                    -- HOST_SLEEP_INTERRUPTED, HOST_RESOURCE_EXHAUSTED
  error_message text,
  cleanup_verdict text,        -- clean, forced, leaked
  last_checkpoint_seq integer, -- last valid checkpoint sequence before failure
  -- run manifest (worker-validator system)
  run_manifest_path text,      -- artifact path to the agent-generated run manifest MD file
  run_manifest_hash text,      -- SHA-256 of the run manifest for integrity verification
  created_at timestamptz not null default now()
);

-- Authority: LOCAL-ONLY
create table error_manifests (
  id uuid primary key default gen_random_uuid(),
  job_id uuid references workflow_jobs(id),
  process_id uuid references procurement_processes(id),
  pipeline_stage text not null, -- 'ingestion', 'qualification', 'enrichment', 'landed_cost', 'bidding'
  workflow_step text not null, -- specific step within the stage
  error_type text not null,
  error_message text not null,
  error_context jsonb not null, -- full context locally; redacted context in Supabase
  skill_path text,
  agent_session_id uuid references agent_session_logs(id),
  markdown_path text, -- local Markdown artifact path for human-friendly debugging
  resolved boolean not null default false,
  resolved_at timestamptz,
  resolved_by text,
  created_at timestamptz not null default now()
);
```

#### 2.5.5 Validation Tables

```sql
-- Semantic validation verdicts from validator skills (local-only -- full verdict reasoning stays local).
-- Authority: LOCAL-ONLY
create table validation_verdicts (
  id uuid primary key default gen_random_uuid(),
  worker_job_id uuid not null references workflow_jobs(id),
  worker_session_id uuid references agent_session_logs(id),
  validator_job_id uuid not null references workflow_jobs(id),
  validator_session_id uuid references agent_session_logs(id),
  validator_skill_path text not null,
  validator_skill_version text,
  verdict text not null,                        -- approved, rejected, flagged
  confidence_bps integer,                       -- basis points (0-10000)
  summary text not null,                        -- one-paragraph human-readable summary (max 2000 chars)
  basis_sufficiency text not null default 'medium', -- high, medium, low
  anchor_conflict_count integer not null default 0,
  incident_signature text,
  enforcement_eligible boolean not null default false,
  issues_json jsonb not null default '[]',      -- JSON array of ValidationIssue objects
  quality_assessment_json jsonb,                -- JSON quality scores (manifest completeness, narrative consistency, etc.)
  skill_improvement_suggestions_json jsonb,     -- JSON array of improvement suggestions for the worker skill
  run_manifest_hash text not null,              -- SHA-256 of the run manifest that was validated
  oqg_signals_summary text,                     -- OQG inputs provided to validator
  override_verdict text,                        -- operator override (if any): approved, rejected
  override_reason text,
  override_by text,
  override_at timestamptz,
  verdict_rendered_at timestamptz not null,
  created_at timestamptz not null default now()
);

-- Validation verdict summaries pushed to Supabase for web visibility (push-owned projection).
-- Authority: PUSH-OWNED (projection)
create table validation_verdict_summaries (
  id uuid primary key,                          -- references validation_verdicts(id)
  worker_job_id uuid not null,
  verdict text not null,                        -- approved, rejected, flagged
  confidence_bps integer,
  basis_sufficiency text,
  incident_signature text,
  enforcement_eligible boolean,
  summary text,
  issue_count integer not null default 0,
  critical_issue_count integer not null default 0,
  override_verdict text,
  verdict_rendered_at timestamptz not null,
  -- sync tracking (same pattern as other push-owned projections)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

#### 2.5.6 Projection Tables

```sql
-- Remote-facing projection summaries for web visibility.
-- Authority: PUSH-OWNED (projection)
create table workflow_job_summaries (
  job_id uuid primary key,  -- opaque ref to workflow_jobs.id (local-only; no Supabase FK)
  process_id uuid references procurement_processes(id),
  kind text not null,
  status text not null,
  scheduled_for timestamptz,
  started_at timestamptz,
  completed_at timestamptz,
  last_error text,
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Authority: PUSH-OWNED (projection)
create table error_manifest_summaries (
  id uuid primary key,  -- opaque ref to error_manifests.id (local-only; no Supabase FK)
  process_id uuid references procurement_processes(id),
  pipeline_stage text not null,
  workflow_step text not null,
  error_type text not null,
  error_message text not null,
  redacted_context jsonb,
  resolved boolean not null default false,
  resolved_at timestamptz,
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Authority: PUSH-OWNED (projection)
create table viewer_sync_status (
  executor_id uuid primary key references executors(id),
  daemon_last_seen_at timestamptz not null,
  last_local_seq_acked bigint not null default 0,
  last_success_at timestamptz,
  sync_health text not null,
  schema_epoch integer not null,
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

#### 2.5.7 Remote-Owned Tables

```sql
-- Human operator disposition (remote-owned, pull-only, compare-and-swap).
-- Authority: REMOTE-OWNED
create table operator_decision_heads (
  process_id uuid primary key references procurement_processes(id),
  disposition text not null default 'pending',  -- pending, watching, approved, rejected
  decision_note text,
  decided_by uuid,
  decided_at timestamptz,
  reviewed_basis_hash text,
  reviewed_manifest_hash text,
  reviewed_at_stage_run_id text,
  decision_stale boolean not null default false,
  -- remote-owned tracking
  remote_change_seq bigint,
  remote_revision bigint not null default 1,
  deleted_at timestamptz,  -- tombstone for shared delete semantics
  created_at timestamptz not null default now()
);

-- Authority: REMOTE-OWNED
create table shared_company_profile (
  id uuid primary key default gen_random_uuid(),
  profile_data jsonb not null,
  -- remote-owned tracking
  remote_change_seq bigint,
  remote_revision bigint not null default 1,
  deleted_at timestamptz,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Authority: REMOTE-OWNED
create table shared_source_configs (
  source_id text primary key,
  kind text not null,
  enabled boolean not null default true,
  config jsonb not null,
  -- remote-owned tracking
  remote_change_seq bigint,
  remote_revision bigint not null default 1,
  deleted_at timestamptz,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

#### 2.5.8 Sync Infrastructure Tables

```sql
-- SQLite-only: local outbox for transactional sync.
-- Forward-compat reservation only in L0: migrations/0001_sqlite_state_initial.sql
-- does not pre-create this table. L4 materializes it in
-- migrations/0012_l4_sync_state.sql with this constitutional shape.
-- Every push-owned write appends an outbox entry in the same transaction.
-- The push dispatcher reads from here and sends batches to Supabase.
-- Authority: LOCAL-ONLY (SQLite-only)
create table local_outbox (
  local_seq bigint primary key autoincrement,
  executor_id text not null,
  entity_kind text not null,
  entity_id text not null,
  op text not null,
  row_version bigint not null,
  stage_run_id text,
  schema_epoch integer not null,
  commit_group_id text,
  writer_epoch integer,
  payload_json text not null,
  content_hash text not null,
  created_at text not null
);

-- SQLite-only: tracks push batches for exactly-once delivery.
-- Authority: LOCAL-ONLY (SQLite-only)
create table sync_batches (
  batch_id text primary key,
  executor_id text not null,
  from_local_seq bigint not null,
  to_local_seq bigint not null,
  schema_epoch integer not null,
  sent_at text not null,
  ack_at text,
  status text not null default 'pending'
);

-- SQLite-only: pull cursors per remote domain.
-- Authority: LOCAL-ONLY (SQLite-only)
create table pull_cursors (
  domain text primary key,
  last_pulled_seq bigint not null default 0,
  pulled_at text not null
);

-- SQLite-only: queued remote-authoritative mutations while sync is degraded.
-- Latest-per-process coalescing: only the most recent intent per process matters.
-- Authority: LOCAL-ONLY (SQLite-only)
create table pending_operator_intents (
  id text primary key,
  process_id text not null,
  desired_disposition text not null,
  decision_note text,
  reviewed_basis_hash text,            -- pipeline data hash the operator reviewed
  reviewed_manifest_hash text,         -- manifest hash the operator reviewed
  reviewed_at_stage_run_id text,       -- stage_run_id the operator reviewed against
  reviewed_at text,                    -- when the operator reviewed
  base_remote_revision bigint not null default 0,  -- 0 = no remote head existed when queued (pre-L4)
  idempotency_key text not null unique,
  status text not null default 'queued',  -- queued, flushing, flushed, conflict, superseded
  superseded_by_intent_id text,        -- coalescing: newer intent for same process
  queued_at text not null,
  last_attempt_at text,
  last_error text
);

-- Supabase-side: schema version tracking for epoch coordination.
-- Authority: SUPABASE-ONLY
create table schema_metadata (
  key text primary key,
  value text not null,
  updated_at timestamptz not null default now()
);
```

#### 2.5.9 Junction Tables

```sql
-- Junction: notice-level CPV codes.
-- Created with contract_notices (constitutional rule: junction tables introduced with parent).
-- Authority: PUSH-OWNED
create table notice_cpv_codes (
  notice_id uuid not null references contract_notices(id),
  cpv_code text not null,
  is_primary boolean not null default false,
  primary key (notice_id, cpv_code)
);

-- Junction: candidate-level commodity codes.
-- Created with product_candidates.
-- Authority: PUSH-OWNED
create table product_commodity_codes (
  id uuid primary key default gen_random_uuid(),
  product_candidate_id uuid not null references product_candidates(id),
  commodity_code text not null,
  digits integer not null,            -- 6, 8, or 10
  source text not null,               -- fpo, cpv_prior, rule_hint, tariff_keyword, taric_support, hs_ping, manual, atr
  score_bps integer not null,         -- ranking score (bounded 0-10000)
  rank_ordinal integer,               -- position within source-ranked results
  description text,
  evidence_json jsonb,
  unique (product_candidate_id, commodity_code, source)
);

-- Junction: lot-level CPV codes.
-- Created with notice_lots.
-- Authority: PUSH-OWNED
create table lot_cpv_codes (
  lot_id uuid not null references notice_lots(id),
  cpv_code text not null,
  is_primary boolean not null default false,
  primary key (lot_id, cpv_code)
);
```

#### 2.5.10 Relation Tables

```sql
-- Edges between related-but-unmerged processes.
-- Enables cache sharing across related clusters without forced merges.
-- Authority: PUSH-OWNED
create table process_relations (
  id uuid primary key default gen_random_uuid(),
  process_id_a uuid not null references procurement_processes(id),
  process_id_b uuid not null references procurement_processes(id),
  relation_type text not null,  -- merged_same_process, related_not_merged, possible_duplicate, separate
  confidence_bps integer not null,  -- basis points (0-10000)
  evidence jsonb,
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Alias pointers for merged processes.
-- When two processes are merged, the alias points to the canonical survivor.
-- Authority: PUSH-OWNED
create table process_aliases (
  alias_process_id uuid primary key,
  canonical_process_id uuid not null references procurement_processes(id),
  alias_reason text not null default 'merged_same_process',
  -- sync tracking (push-owned)
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

#### 2.5.11 Cache Tables

```sql
-- Cached marketplace listing evidence for re-scoring across candidates without re-fetching.
-- Authority: LOCAL-ONLY
create table listing_evidence (
  id uuid primary key default gen_random_uuid(),
  source text not null,
  source_listing_id text not null,
  normalized_title text not null,
  structured_attributes_json jsonb,
  parsed_standards_json jsonb,
  parsed_units_json jsonb,
  seller_identifier text,
  minimum_order_quantity numeric,
  shipping_terms text,
  condition text,
  compliance_evidence_tier text not null,
  intended_use_language text,
  is_ready_to_ship boolean,
  fetched_at timestamptz not null,
  detail_page_fetched boolean not null default false,
  trust_state text not null default 'unverified',
  source_job_id uuid,
  created_at timestamptz not null default now(),
  unique (source, source_listing_id)
);
```

#### 2.5.12 Evaluation Tables

```sql
-- Gold-set corpus cases. Each row is one curated lot with human labels.
-- Authority: LOCAL-ONLY
create table gold_set_cases (
  id uuid primary key default gen_random_uuid(),
  corpus_version text not null,
  lot_title text not null,
  lot_description text,
  category text not null,             -- office_supplies, cleaning, ppe, hand_tools, furniture, packaging
  corpus_set text not null,           -- gating (easy categories) or challenge (hard categories / hard negatives)
  source_notice_json jsonb not null,  -- curated OCDS notice data
  source_documents_json jsonb,        -- curated document references
  expected_candidates_json jsonb not null,   -- human-labeled expected product candidates
  expected_matches_json jsonb not null,      -- human-labeled expected supplier matches
  expected_margin_band_json jsonb not null,  -- human-labeled margin bands {lowBps, baseBps, highBps}
  expected_decision text,             -- auto_qualify, review, reject, abstain
  case_as_of_at timestamptz not null,        -- market state date for temporal reproducibility
  label_as_of_at timestamptz not null,       -- when labels were created/verified
  evidence_bundle_hash text,                 -- hash of captured evidence for reproducible benchmark lane
  is_hard_negative boolean not null default false,  -- challenge case for abstention/blocker testing
  hard_negative_reason text,                 -- services_only, custom_fabricated, remedy_adjacent, compliance_gap, etc.
  match_archetype_expected text,             -- expected matchArchetype for unknown-rate monitoring
  is_double_annotated boolean not null default false,  -- flagged for inter-annotator agreement measurement
  created_at timestamptz not null default now()
);

-- Evaluation run metadata for reproducibility and regression tracking.
-- Authority: LOCAL-ONLY
create table evaluation_runs (
  id uuid primary key default gen_random_uuid(),
  corpus_version text not null,
  pipeline_version text not null,
  run_mode text not null,             -- reproducible_benchmark, live_smoke
  started_at timestamptz not null,
  completed_at timestamptz,
  config_json jsonb not null,         -- pipeline config snapshot
  created_at timestamptz not null default now()
);

-- Per-case results within an evaluation run.
-- Authority: LOCAL-ONLY
create table evaluation_case_results (
  id uuid primary key default gen_random_uuid(),
  run_id uuid not null references evaluation_runs(id),
  case_id uuid not null references gold_set_cases(id),
  extraction_passed boolean,
  matching_passed boolean,
  margin_passed boolean,
  decision_agreed boolean,            -- auto-qualify/review/reject agrees with expert label
  abstention_correct boolean,         -- for hard negatives: correctly abstained/blocked?
  candidates_found integer not null default 0,
  genuine_matches_found integer not null default 0,
  margin_error_bps integer,           -- signed: pipeline minus expert (bps)
  match_archetype_assigned text,
  candidate_type_assigned text,
  details_json jsonb not null,        -- full pipeline output for debugging
  created_at timestamptz not null default now(),
  unique (run_id, case_id)
);

-- Aggregate metrics per evaluation run.
-- Authority: LOCAL-ONLY
create table evaluation_metric_summaries (
  id uuid primary key default gen_random_uuid(),
  run_id uuid not null references evaluation_runs(id) unique,
  -- Headline experiment results
  experiment_1_rate_bps integer not null,   -- extraction yield rate (bounded bps)
  experiment_2_rate_bps integer not null,   -- matching yield rate (bounded bps)
  experiment_3_rate_bps integer not null,   -- margin agreement rate (bounded bps)
  -- Slice floors (per-category minimums)
  worst_category_exp1_bps integer,
  worst_category_exp2_bps integer,
  worst_category_exp3_bps integer,
  worst_archetype_exp2_bps integer,
  -- Hard-negative gates
  false_auto_qualify_rate_bps integer,      -- false positives on hard negatives (bounded bps)
  false_match_rate_bps integer,             -- false matches on hard negatives (bounded bps)
  -- Decision-oriented metrics
  decision_agreement_rate_bps integer,      -- auto-qualify/review/reject agreement (bounded bps)
  auto_qualify_precision_bps integer,       -- precision of auto-qualify decisions (bounded bps)
  abstention_correctness_bps integer,       -- correct abstention on challenge cases (bounded bps)
  -- Unknown escape hatch monitoring
  unknown_archetype_rate_bps integer,       -- rate of matchArchetype = unknown (bounded bps)
  unknown_candidate_type_rate_bps integer,  -- rate of candidateType = unknown (bounded bps)
  -- Full breakdown
  metrics_json jsonb not null,              -- per-category, per-archetype, per-corpus-set breakdowns
  created_at timestamptz not null default now()
);
```

### 2.6 Key TypeScript Interfaces

These interfaces define the logical application contract. They are the source of truth for TypeScript code in `packages/contracts-schema/`. Persisted SQLite/Postgres columns use integer pence (`_pence` suffix) and basis points (`_bps` suffix) where noted in the SQL schema above; the TypeScript interfaces use `number` for these fields (the boundary conversion is in the repository layer).

```ts
export interface ProcurementProcess {
  id: string;
  canonicalOcid?: string;
  primaryNoticeId?: string;
  canonicalProcessId?: string;
  currentStage:
    | "pipeline"
    | "planning"
    | "tender_open"
    | "award"
    | "contract"
    | "termination";
  buyerName?: string;
  title: string;
  latestDeadlineAt?: string;
  latestPublishedAt: string;
  latestUpdatedAt: string;
}
```

```ts
export interface ContractNotice {
  id: string;
  processId: string;
  source: "fts" | "contracts_finder" | "pcs" | string;
  sourceNoticeId: string;
  ocid?: string;
  title: string;
  summary?: string;
  buyerName?: string;
  procurementStage:
    | "pipeline"
    | "planning"
    | "tender_open"
    | "award"
    | "contract"
    | "termination";
  accessRoute?:
    | "open_tender"
    | "restricted_tender"
    | "framework_call_off"
    | "dynamic_market"
    | "direct_award"
    | "closed_framework_award"
    | "not_bidable";
  deadlineAt?: string;
  publishedAt: string;
  updatedAt: string;
  mainCategory?: "goods" | "services" | "works" | "mixed" | "unknown";
  cpvCodes: string[];
  estimatedValueOriginalMinor?: number;  // original currency minor units (fen, cents, etc.)
  estimatedValueCurrency?: string;
  estimatedValueGbpExVat?: number;
  estimatedValueVatInclusive?: boolean;
  rawDocumentId: string;
}
```

```ts
export interface NoticeLot {
  id: string;
  noticeId: string;
  sourceLotId?: string;
  lotKey?: string;
  lotNumber?: number;
  title?: string;
  description?: string;
  cpvCodes: string[];
  estimatedValue?: number;
  deadlineAt?: string;
  isBidableSeparately: boolean;
  documents?: string[];
}
```

```ts
export interface TenderDocumentSet {
  id: string;
  processId: string;
  noticeId: string;
  documentSetHash: string;
  completenessState: "none" | "partial" | "full";
  documentRoles: Array<"spec" | "pricing_schedule" | "boq" | "appendix" | "other">;
  documents: Array<{
    rawDocumentId: string;
    role: "spec" | "pricing_schedule" | "boq" | "appendix" | "other";
    sourceUrl?: string;
  }>;
  acquiredAt: string;
}
```

```ts
export interface ProductCandidate {
  id: string;
  processId: string;
  noticeId: string;
  lotId: string;
  parentCandidateId?: string;
  candidateKey?: string;
  candidateRole: "standalone" | "bundle_parent" | "bundle_component";
  name: string;
  candidateType:
    | "exact_sku"
    | "standard_commodity"
    | "brand_family"
    | "bundle"
    | "custom_or_fabricated"
    | "installed_goods"
    | "regulated_goods"
    | "unknown";
  matchArchetype:
    | "standard_class_led"
    | "dimension_material_led"
    | "electrical_spec_led"
    | "pack_count_led"
    | "compatibility_led"
    | "model_exact"
    | "unknown";
  headNoun: string;
  quantity?: number;
  quantityUnit?: string;
  specificationText?: string;
  estimatedUnitWeightKg?: number;
  estimatedUnitVolumeM3?: number;
  candidateValueShareBps?: number;
  packingHint?: "flat_pack" | "assembled" | "mixed" | "unknown";
  handlingRestrictions?: Array<"hazmat" | "oversized" | "fragile" | "reefer" | "lithium_battery">;
  candidateCommodityCodes: string[];
  criticalAttributes?: Record<string, CriticalAttribute>;
  numericSpecs?: Record<string, NumericSpec>;
  standards?: StandardSlot[];
  material?: string;
  brandModel?: string;
  preferredTerms?: string[];
  excludedTerms?: string[];
  sourceEvidence?: Array<{ documentId?: string; pageRef?: string; excerpt?: string }>;
  quoteabilityScore?: number;
  recallRiskScore?: number;
  identityConfidence: number;
  specConfidence: number;
  quantityConfidence: number;
}

export interface CriticalAttribute {
  value: string | number | boolean;
  criticality: "required" | "preferred" | "descriptive" | "exclusion";
}

export interface NumericSpec {
  value: string | number;
  unit?: string;
  toleranceWindow?: { min?: number; max?: number };
  criticality: "required" | "preferred" | "descriptive";
}

export interface StandardSlot {
  raw: string;
  family: string;
  part?: string;
  editionYear?: string;
  amendment?: string;
  classLevel?: string;
  criticality: "required" | "preferred" | "descriptive";
}
```

```ts
export interface ProcessRelation {
  id: string;
  processIdA: string;
  processIdB: string;
  relationType: "merged_same_process" | "related_not_merged" | "possible_duplicate" | "separate";
  confidence: number;
  evidence?: string[];
  createdAt: string;
}

export interface SearchAttempt {
  id: string;
  productCandidateId: string;
  searchProfileId: string;
  source: "ebay" | "amazon" | "alibaba" | "1688";
  locale: "en" | "zh";
  waveNumber: number;
  queryText: string;
  filters?: Record<string, string>;
  isRecoveryWave: boolean;
  resultCount: number;
  hardRejectedCount: number;
  shortlistedCount: number;
  searchFailureMode?: "vocabulary_gap" | "overconstrained_query" | "accessory_swamp"
    | "compliance_not_surfaceable" | "no_marketplace_presence"
    | "low_extraction_quality" | "source_specific_failure";
  executedAt: string;
  sessionId?: string;
}

export interface ListingEvidence {
  id: string;
  source: "ebay" | "amazon" | "alibaba" | "1688";
  sourceListingId: string;
  normalizedTitle: string;
  structuredAttributes?: Record<string, string>;
  parsedStandards?: StandardSlot[];
  parsedUnits?: { packQty?: number; packUnit?: string; normalizedBasis?: string };
  sellerIdentifier?: string;
  minimumOrderQuantity?: number;
  shippingTerms?: string;
  condition?: "new" | "used" | "refurbished" | "unknown";
  complianceEvidenceTier: "structured_attribute" | "title_body_mention" | "absent";
  intendedUseLanguage?: string;
  isReadyToShip?: boolean;
  fetchedAt: string;
  detailPageFetched: boolean;
  trustState?: "verified" | "flagged" | "unverified" | "suspect";
  sourceJobId?: string;
}

export interface SearchProfile {
  id: string;
  productCandidateId: string;
  headNoun: string;
  headNounFamily?: string;
  candidateType: ProductCandidate["candidateType"];
  matchArchetype: ProductCandidate["matchArchetype"];
  criticalAttributes?: Record<string, CriticalAttribute>;
  standards?: StandardSlot[];
  numericSpecs?: Record<string, NumericSpec>;
  nounAliasesEn?: string[];
  nounAliasesZh?: string[];
  marketFrameEn?: string[];
  marketFrameZh?: string[];
  useContext?: string;
  sectorContext?: string;
  requiredAnchors?: string[];
  optionalAnchors?: string[];
  mustNotHave?: string[];
  preferredTerms?: string[];
  excludedTerms?: string[];
  queryWaves: QueryWave[];
  recallRiskScore?: number;
  searchEligibilityState: "auto_search" | "manual_only" | "skip";
  createdAt: string;
}

export interface QueryWave {
  waveNumber: number;
  source: "ebay" | "amazon" | "alibaba" | "1688";
  locale: "en" | "zh";
  queryText: string;
  filters?: Record<string, string>;
  isRecoveryWave: boolean;
}
```

```ts
export interface SupplierQuote {
  id: string;
  processId: string;
  noticeId: string;
  productCandidateId: string;
  searchAttemptId?: string;
  productName?: string;
  region: "uk" | "eu" | "china";
  quoteClass: "indicative_marketplace" | "distributor_listing" | "rfq_quote" | "negotiated_quote";
  sourceName: string;
  sourceUrl?: string;
  sourceListingId?: string;
  shipFromCountry?: string;
  countryOfOrigin?: string;
  countryOfDispatch?: string;
  incoterm?: "EXW" | "FOB" | "CIF" | "DAP" | "DDP" | string;
  quoteCostBoundary?: "EXW" | "FOB" | "CIF" | "CIP" | "DAP" | "DDP" | "UNKNOWN";
  quotedQuantity?: number;
  quotedQuantityUnit?: string;
  unitPriceOriginalMinor: number;  // per-unit in original currency minor units
  currency: string;
  unitPriceGbp: number;
  offerTotalOriginalMinor?: number;  // total offer price in original minor units
  offerTotalGbp?: number;            // total offer price in GBP pence
  priceBasisQty?: number;            // quantity the offer covers
  priceBasisUnit?: string;           // unit for basis qty
  minimumOrderQuantity?: number;
  leadTimeDays?: number;
  includesShipping?: boolean;
  priceSignalRole: "benchmark_ceiling" | "optimistic_floor" | "executable";
  listingPackQty?: number;
  listingPackUnit?: string;
  normalizedUnitBasis?: string;
  isUsedOrRefurbished?: boolean;
  isReadyToShip?: boolean;
  vatInclusive?: boolean;
  complianceEvidenceTier?: "structured_attribute" | "title_body_mention" | "absent";
  rawMatchScore?: number;
  matchConfidence: number;
  quoteUsabilityConfidence: number;
  shippingBasisConfidence?: number;
  quoteRealismConfidence: number;
  negotiationUpliftBps?: number;
  evidenceStates?: Record<string, "match" | "missing" | "ambiguous" | "conflict">;
  queryWaveNumber?: number;
  evidence: string[];
  validUntil?: string;
}
```

```ts
export interface CandidatePricingAssessment {
  id: string;
  processId: string;
  noticeId: string;
  productCandidateId: string;
  selectedQuoteId: string;
  selectedQuoteRowVersion: number;
  quoteCostBoundary: "EXW" | "FOB" | "CIF" | "CIP" | "DAP" | "DDP" | "UNKNOWN";
  revenueBasisType: "firm_contract" | "estimated_contract" | "framework_ceiling" | "dynamic_market" | "concession" | "unknown";
  expectedAddressableRevenueExVatPence: number;
  revenueConfidenceBps: number;
  grossMarginBps: number;
  grossEstimateConfidenceBps: number;
  identityGatePassed: boolean;
  usabilityGatePassed: boolean;
  pricingInputConfidenceBps: number;
  createdAt: string;
}
```

```ts
export interface LandedCostEstimate {
  id: string;
  processId: string;
  noticeId: string;
  productCandidateId: string;
  supplierQuoteId: string;
  shippingMode: "parcel" | "air_freight" | "lcl_ocean" | "fcl_ocean" | "mixed";
  modeConfidence: number;
  packingScenario?: "compact" | "bulky" | "unknown";
  declarableCommodityCode10?: string;
  candidateCommodityCodes?: CandidateCommodityCode[];
  additionalCodes?: string[];
  additionalCodeRequirements?: string[];
  supplementaryUnitsRequired?: string[];
  commodityConfidence: number;
  classificationStability: number;
  customsValuationMethod?: "1_actual" | "1_proxy" | "2" | "3" | "4" | "5" | "6" | "unknown";
  valuationMode?: "estimation" | "declaration";
  customsFxRate: number;
  commercialFxRate: number;
  fxBasisDeltaBps: number;
  fxRateCheckedAt?: string;
  supplierCostGbp: number;
  freightLowGbp: number;
  freightBaseGbp: number;
  freightHighGbp: number;
  customsDutyGbp: number;
  tradeRemedyDutyGbp?: number;
  dutyOutcomeMinGbp?: number;
  dutyOutcomeMaxGbp?: number;
  customsCostSpreadBps?: number;
  tradeRemedyState?: "none" | "investigation_open" | "provisional" | "definitive" | "review_open" | "adjacent_risk";
  tradeRemedyCases?: string[];
  dutyBreakdown?: Record<string, unknown>;
  measureInteraction?: Record<string, unknown>;
  exporterRateBasis?: "named_exporter" | "annex_exporter" | "residual_default";
  exporterIdentityConfidence?: number;
  originEvidenceState?: "proved" | "claimed" | "unknown";
  importVatGbp: number;
  vatRateState?: "standard" | "reduced" | "zero" | "unresolved";
  importVatTreatment: "postponed_full" | "postponed_partial"
    | "cash_paid_full" | "cash_paid_partial" | "non_recoverable" | "unknown";
  vatRecoveryRate?: number;
  ukFinalMileCostGbp: number;
  otherFeesGbp: number;
  economicLandedCostGbp: number;
  cashOutlayGbp: number;
  cashToClearGbp: number;
  workingCapitalCostGbp?: number;
  revenueBasisType: "firm_contract" | "estimated_contract"
    | "framework_ceiling" | "dynamic_market" | "concession" | "unknown";
  revenueConfidence: number;
  customsResolutionConfidence?: number;
  commercialEstimateConfidence?: number;
  netMarginLowBps: number;
  netMarginBaseBps: number;
  netMarginHighBps: number;
  decisionGrade: "screening" | "review" | "rfq_required" | "broker_required";
  confidence: number;
  confidenceBreakdown?: ConfidenceBreakdown;
  warnings: string[];
  blockers: string[];
  tariffCheckedAt?: string;
  measuresFreshUntil?: string;
  classificationFreshUntil?: string;
  tradeRemedyWatchState?: "clear" | "investigation_open" | "provisional_measure" | "review_pending" | "adjacent_family_active";
  cbamExposureFlag?: boolean;
  cbamEstimateGbp?: number;
}

export interface CandidateCommodityCode {
  code: string;
  digits: 6 | 8 | 10;
  source: "fpo" | "cpv_prior" | "rule_hint" | "tariff_keyword" | "taric_support" | "hs_ping" | "manual" | "atr";
  score: number;
  description?: string;
}

export interface ConfidenceBreakdown {
  classificationStability: number;
  quoteRealism: number;
  commodityCodeConfidence: number;
  freightModeStability: number;
  revenueReliability: number;
  fxFreshness: number;
}
```

```ts
export interface PipelineAssessment {
  processId: string;
  pipelineState:
    | "discovered"
    | "normalized"
    | "relation_resolved"
    | "prelim_rejected"
    | "prelim_qualified"
    | "docs_acquired"
    | "lots_extracted"
    | "products_extracted"
    | "quoteability_scored"
    | "priced"
    | "gross_qualified"
    | "landed_costed"
    | "net_qualified";
  reviewRequired: boolean;
  availabilityStatus: "active" | "expired" | "cancelled";
  validationState: "validated" | "flagged" | "unverified";
  grossMarginBps?: number;
  netMarginBps?: number;
  pricedCoverageBps?: number;
  landedCostCoverageBps?: number;
  netQualifiedCoverageBps?: number;
  pricedValueCoverageBps?: number;
  landedCostValueCoverageBps?: number;
  netQualifiedValueCoverageBps?: number;
  unresolvedMandatoryLots?: number;
  decisionBasisHash?: string;
  reviewStage?: "qualification" | "quoteability" | "pricing" | "landed_cost" | "manifest";
  reviewReason?: string;
  assessedAt: string;
}
```

```ts
export interface OperatorDecisionHead {
  processId: string;
  disposition: "pending" | "watching" | "approved" | "rejected";
  decisionNote?: string;
  decidedBy?: string;
  decidedAt?: string;
  reviewedBasisHash?: string;
  reviewedManifestHash?: string;
  reviewedAtStageRunId?: string;
  decisionStale?: boolean;
  remoteRevision: number;
}

export interface BidWorkflowState {
  processId: string;
  bidState:
    | "not_started"
    | "ready_to_bid"
    | "bid_in_progress"
    | "draft_saved"
    | "bid_submitted"
    | "bid_failed"
    | "portal_state_unknown";
  bidScope?: { lotIds: string[]; candidateIds: string[] };
  reviewRequired?: boolean;
  bidJobId?: string;
  startedAt?: string;
  completedAt?: string;
}

export interface WorkflowJob {
  id: string;
  kind:
    | "sync_source"
    | "normalize_notice"
    | "resolve_relations"
    | "prequalify_notice"
    | "qualify_lot"
    | "acquire_documents"
    | "extract_document_structure"
    | "extract_document_semantics"
    | "extract_lots"
    | "extract_products"
    | "score_quoteability"
    | "enrich_supplier_prices"
    | "compute_gross_gate"
    | "estimate_landed_cost"
    | "compute_net_gate"
    | "generate_manifest"
    | "record_operator_decision"
    | "run_bid_workflow"
    | "escalate_rfq"
    | "validation";
  processId?: string;
  noticeId?: string;
  scopeKind?: "process" | "notice" | "lot" | "candidate" | "candidate_source_pair" | "candidate_quote_scenario";
  scopeId?: string;
  parentJobId?: string;
  continuationChainId?: string;
  chainClosed?: boolean;
  validationRequired: boolean;
  validationVerdictId?: string;
  status: string;
  scheduledFor?: string;
  attemptCount: number;
  inputPayload: Record<string, unknown>;
  outputPayload?: Record<string, unknown>;
  lastError?: string;
  startedAt?: string;
  completedAt?: string;
  createdAt: string;
}
```

```ts
export interface AgentSessionLog {
  id: string;
  jobId?: string;
  skillPath: string;
  skillVersion?: string;
  executionMode: "pty" | "contract";
  cliVendor?: string;
  cliVersion?: string;
  adapterVersion?: string;
  configHash?: string;
  ptySessionId: string;
  pid?: number;
  pgid?: number;
  startedAt: string;
  completedAt?: string;
  status: "created" | "spawned" | "active" | "terminating" | "exited" | "reaped" | "archived";
  terminalOutputPath?: string;
  terminalOutputHash?: string;
  terminalOutputBytes?: number;
  terminalOutputTail?: string;
  structuredTrace?: Record<string, unknown>;
  errorType?: string;
  errorMessage?: string;
  cleanupVerdict?: "clean" | "forced" | "leaked";
  lastCheckpointSeq?: number;
  runManifestPath?: string;
  runManifestHash?: string;
}

export interface ProcessStageStatus {
  processId: string;
  stage:
    | "ingest"
    | "normalize"
    | "relate"
    | "prequalify_notice"
    | "qualify_lot"
    | "acquire_docs"
    | "extract_lots"
    | "extract_products"
    | "score_quoteability"
    | "price_signal_match"
    | "gross_margin_gate"
    | "landed_cost"
    | "net_margin_gate"
    | "generate_manifest"
    | "operator_decision"
    | "bid_workflow";
  scopeKind: "process" | "notice" | "lot" | "candidate" | "candidate_source_pair" | "candidate_quote_scenario";
  scopeId: string;
  stageRunId: string;
  state: "running" | "succeeded" | "failed";
  freshnessState?: "fresh" | "soft_stale" | "hard_invalid";
  basisHash?: string;
  inputStageVector?: Record<string, string>;
  startedAt: string;
  completedAt?: string;
  errorManifestId?: string;
}
```

```ts
export interface ValidationVerdict {
  id: string;
  workerJobId: string;
  workerSessionId?: string;
  validatorJobId: string;
  validatorSessionId?: string;
  validatorSkillPath: string;
  validatorSkillVersion?: string;
  verdict: "approved" | "rejected" | "flagged";
  confidenceBps?: number;
  summary: string;
  basisSufficiency: "high" | "medium" | "low";
  anchorConflictCount: number;
  incidentSignature?: string;
  enforcementEligible: boolean;
  issues: ValidationIssue[];
  qualityAssessment?: {
    manifestCompleteness?: "complete" | "partial" | "minimal" | "missing";
    narrativeConsistency?: "consistent" | "minor_inconsistencies" | "contradictory";
    evidencePresent?: boolean;
    fingerprintVerified?: boolean;
    localeVerified?: boolean;
    resultCountMatchesManifest?: boolean;
  };
  skillImprovementSuggestions?: string[];
  runManifestHash: string;
  oqgSignalsSummary?: string;
  overrideVerdict?: "approved" | "rejected";
  overrideReason?: string;
  overrideBy?: string;
  overrideAt?: string;
  verdictRenderedAt: string;
  createdAt: string;
}
```

```ts
export interface ValidationVerdictSummary {
  id: string;
  workerJobId: string;
  verdict: "approved" | "rejected" | "flagged";
  confidenceBps?: number;
  basisSufficiency?: "high" | "medium" | "low";
  incidentSignature?: string;
  enforcementEligible?: boolean;
  summary?: string;
  issueCount: number;
  criticalIssueCount: number;
  overrideVerdict?: "approved" | "rejected";
  verdictRenderedAt: string;
}

export interface ValidationIssue {
  severity: "critical" | "warning" | "info";
  category:
    | "wrong_source"
    | "wrong_locale"
    | "data_fabrication"
    | "data_quality"
    | "missing_data"
    | "semantic_error"
    | "suspicious_pattern"
    | "manifest_inconsistency"
    | "coverage_gap"
    | "policy_violation";
  description: string;
  evidence?: string;
  suggestion?: string;
}
```

```ts
export interface ViewerSyncStatus {
  executorId: string;
  daemonLastSeenAt: string;
  lastLocalSeqAcked: number;
  lastSuccessAt?: string;
  syncHealth: "healthy" | "degraded" | "offline";
  schemaEpoch: number;
}
```

### 2.7 Index Design

Beyond primary keys (implicit in all CREATE TABLE statements above), these indices support the critical query paths.

#### Foreign Key Lookup Indices

```sql
-- Pipeline entity traversal
create index idx_contract_notices_process_id on contract_notices(process_id);
create index idx_notice_lots_notice_id on notice_lots(notice_id);
create index idx_product_candidates_lot_id on product_candidates(lot_id);
create index idx_product_candidates_process_id on product_candidates(process_id);
create index idx_supplier_quotes_candidate_id on supplier_quotes(product_candidate_id);
create index idx_supplier_quotes_process_id on supplier_quotes(process_id);
create index idx_search_profiles_candidate_id on search_profiles(product_candidate_id);
create index idx_search_attempts_profile_id on search_attempts(search_profile_id);
create index idx_search_attempts_candidate_id on search_attempts(product_candidate_id);
create index idx_candidate_pricing_candidate_id on candidate_pricing_assessments(product_candidate_id);
create index idx_landed_cost_candidate_id on landed_cost_estimates(product_candidate_id);
create index idx_landed_cost_quote_id on landed_cost_estimates(supplier_quote_id);
create index idx_tender_docs_process_id on tender_document_sets(process_id);
create index idx_qualification_runs_lot_id on qualification_runs(lot_id);

-- Orchestration traversal
create index idx_workflow_jobs_process_id on workflow_jobs(process_id);
create index idx_workflow_jobs_parent_id on workflow_jobs(parent_job_id);
create index idx_agent_sessions_job_id on agent_session_logs(job_id);
create index idx_error_manifests_job_id on error_manifests(job_id);
create index idx_error_manifests_process_id on error_manifests(process_id);
create index idx_validation_verdicts_worker_job on validation_verdicts(worker_job_id);

-- Relation traversal
create index idx_process_relations_a on process_relations(process_id_a);
create index idx_process_relations_b on process_relations(process_id_b);
create index idx_process_aliases_canonical on process_aliases(canonical_process_id);
```

#### Query Path Indices

```sql
-- Pipeline assessment lookup (most common dashboard query)
create index idx_pipeline_assessments_state on pipeline_assessments(pipeline_state);

-- Stage tracking queries
create index idx_stage_status_process on process_stage_status(process_id, stage);
create index idx_stage_status_run on process_stage_status(stage_run_id);

-- Job queue queries (scheduler hot path)
create index idx_workflow_jobs_status on workflow_jobs(status, scheduled_for);
create index idx_workflow_jobs_kind_status on workflow_jobs(kind, status);

-- Source cursor lookup
create index idx_raw_documents_source on raw_documents(source_id);
```

#### Keyset Pagination Indices

```sql
-- For list-view pagination in desktop/web UI (created_at, id) pairs
create index idx_procurement_processes_pagination on procurement_processes(created_at, id);
create index idx_contract_notices_pagination on contract_notices(created_at, id);
create index idx_supplier_quotes_pagination on supplier_quotes(created_at, id);
create index idx_workflow_jobs_pagination on workflow_jobs(created_at, id);
```

#### Unique Constraints (declared in DDL, restated for clarity)

```sql
-- Already declared in CREATE TABLE statements above:
-- contract_notices: unique (source_id, source_notice_id)
-- listing_evidence: unique (source, source_listing_id)
-- source_cursors: primary key (source_id, cursor_key)
-- notice_cpv_codes: primary key (notice_id, cpv_code)
-- product_commodity_codes: primary key (id), unique (product_candidate_id, commodity_code, source)
-- lot_cpv_codes: primary key (lot_id, cpv_code)
-- process_stage_heads: primary key (process_id, stage, scope_kind, scope_id)
-- pending_operator_intents: unique (idempotency_key)
```

### 2.8 Forward-Compatibility Columns

These columns exist from Layer 0 table creation and are either populated immediately or become meaningful in later layers. They are listed here to prevent "where did this column come from?" confusion during implementation.

| Table | Column(s) | First Populated | Purpose |
|-------|-----------|----------------|---------|
| `procurement_processes` | `canonical_process_id` | L0 | Stable cluster identity root; initialized to self on first write, updated by L3 relation merges |
| `workflow_jobs` | `parent_job_id` | L3 | Links validator sub-job to worker job |
| `workflow_jobs` | `validation_required` | L3 | Set by Validation Policy Engine (default `false` from creation) |
| `workflow_jobs` | `validation_verdict_id` | L3 | Populated after validation completes |
| `workflow_jobs` | `continuation_chain_id`, `chain_closed` | L1 | Continuation chain tracking for multi-session skills |
| `workflow_jobs` | `lease_owner`, `lease_expires_at` | L1 | Job leasing for daemon scheduler |
| `workflow_jobs` | `scope_kind`, `scope_id` | L1 | Stage scope grain (process/notice/lot/candidate) |
| `pipeline_assessments` | `validation_state` | L3 | Default `'unverified'`; set to `'validated'`/`'flagged'` by validator |
| `pipeline_assessments` | `priced_coverage_bps` | L2 | Coverage metric display |
| `pipeline_assessments` | `landed_cost_coverage_bps` | L2 | Coverage metric display |
| `pipeline_assessments` | `net_qualified_coverage_bps` | L2 | Coverage metric display |
| `pipeline_assessments` | `decision_basis_hash` | L2 | Decision staleness tracking |
| `agent_session_logs` | `run_manifest_path`, `run_manifest_hash` | L1 | Formal run manifest system |
| `agent_session_logs` | `cleanup_verdict` | L1 | Process-group supervision |
| `process_stage_status` | `freshness_state` | L1 | Staleness model (default `'fresh'`) |
| `process_stage_status` | `input_revision`, `output_revision` | L1 | Revision tracking for invalidation |
| `process_stage_heads` | `freshness_state`, `basis_hash` | L1 | Staleness propagation |
| `listing_evidence` | `trust_state`, `source_job_id` | L3 | Carry reusable marketplace evidence forward with trust propagation and producing-job lineage |
| `landed_cost_estimates` | Decision-grade fields (all non-screening columns) | L5 | Full customs resolution |
| `landed_cost_estimates` | `duty_breakdown_json`, `measure_interaction_json` | L5 | Preserve full duty expression and measure-family interaction outcomes |
| `landed_cost_estimates` | `measures_fresh_until`, `fx_rate_checked_at` | L5 | Track measure freshness and FX verification as independent invalidation vectors |
| `landed_cost_estimates` | `cbam_exposure_flag`, `cbam_estimate_gbp_pence` | Future | CBAM carbon border adjustment |
| `operator_decision_heads` | All columns | L4 | Remote-owned, pulled via sync engine |
| `shared_company_profile` | All columns | L4 | Remote-owned, pulled via sync engine |
| `shared_source_configs` | All columns | L4 | Remote-owned, pulled via sync engine |
| `local_outbox` | All columns, including `commit_group_id` and `writer_epoch` | L4 | Sync engine transactional outbox; L0 reserves shape, L4 materializes table |
| `sync_batches` | All columns | L4 | Sync engine batch tracking |
| `pull_cursors` | All columns | L4 | Sync engine pull cursors |
| `pending_operator_intents` | All columns | L2 | Local intent queue for operator actions |
| `validation_verdicts` | All columns | L3 | Worker-validator system |
| `validation_verdict_summaries` | All columns | L3 | Worker-validator projections |
| `bid_workflow_states` | All columns | L6 | Bid automation |
| `viewer_sync_status` | All columns | L4 | Sync health monitoring |

**Tables that are fully active in Layer 0** (all columns populated during gold-set evaluation):

- `sources`, `source_cursors`, `raw_documents` (infrastructure)
- `procurement_processes`, `contract_notices`, `notice_lots` (pipeline core)
- `qualification_runs`, `product_candidates`, `search_profiles` (extraction/matching)
- `search_attempts`, `supplier_quotes`, `listing_evidence` (search execution)
- `candidate_pricing_assessments` (gross gate)
- `landed_cost_estimates` (screening-grade subset of columns)
- `pipeline_assessments` (pipeline state tracking)
- `notice_cpv_codes`, `product_commodity_codes`, `lot_cpv_codes` (junction tables)
- `workflow_jobs` (job tracking, even in CLI mode)
- `agent_session_logs` (skill execution records)

## 3. Intelligence Pipeline (Screening-Grade)

> **Ownership:** This section owns the **processing logic** of each pipeline stage — what it does, what it produces, what gates it applies. Stage **tracking infrastructure** (stage_run_id, scope grain table, commit semantics) is owned by Layer 1 §3. Coverage metric **presentation** is owned by Layer 2 §3.

The screening-grade pipeline is the core of Layer 0. It runs extraction, matching, and cost estimation on curated gold-set data. The pipeline has 15 rerunnable stages. In Layer 0, stages run from CLI against pre-curated inputs. In Layer 1, they run live under daemon management.

### 3.1 Stage 1 — Ingest

Source-batched fetch via procurement ingestion skills. One agent session per source per run, not one per notice.

**FTS (Find a Tender):** OCDS release package API with `limit`, `cursor`, `updatedFrom`, `updatedTo`, and stage filters. Data under Open Government Licence. The agent navigates the FTS API/portal via browser automation using the `procurement-ingestion/fts` skill.

**Contracts Finder:** OCDS/search API with `publishedFrom`, `publishedTo`, `stages`, `limit`, `cursor`. Daily CSV harvester endpoint with rate limiting. Dedicated `procurement-ingestion/contracts-finder` skill.

**PCS (Public Contracts Scotland):** OCDS Web API with notice list and notice family retrieval. Month-based query model. **Deferred to Layer 5** unless Scotland is confirmed in MVP scope (see Open Questions §7).

**Output:** Raw notice records in OCDS format.

**Layer 0 stub:** Gold-set notices are pre-curated. No live source interaction. Cursor management activates in Layer 1.

### 3.2 Stage 2 — Normalize

Parse OCDS release packages into canonical internal entities: `ProcurementProcess` + `ContractNotice` + `NoticeLot`.

**Field mapping:**
- `title`, `description`, `publishedDate`, `deadlineDate`, `estimatedValue`, `procedureType`
- Notice lifecycle stages: `pipeline` → `planning` → `tender_open` → `award` → `contract` → `termination`

**Multi-notice handling:**
- Amendments update existing processes (same OCID)
- New releases create new processes
- Operator-facing eligibility derived from normalized state, not raw portal labels

**Procurement Act 2023 changes:** For processes starting on or after 24 February 2025, new notice types apply. The normalizer must handle both legacy and new-regime notices with transitional treatment.

**Output:** Canonical `ProcurementProcess`, `ContractNotice`, and `NoticeLot` entities.

### 3.3 Stage 3 — Relate

Three-way relation resolution: notice ↔ tender ↔ process.

Four persisted relation classifications:

| Classification | Meaning |
|---------------|---------|
| `merged_same_process` | Same OCID or explicit linked notice chain |
| `related_not_merged` | Framework/call-off, planning/tender, same buyer + subject but different OCIDs |
| `possible_duplicate` | High-similarity pair held for operator review instead of forced merge |
| `separate` | No meaningful relationship |

**Key distinctions:**
- Framework call-offs are commercially separate from framework establishment
- Direct award after unsuccessful competition gets a different OCID — genuinely new process
- Planning notice and later tender may share subject but not OCID — related but not merged

**Layer 0 stub:** Gold set is pre-curated with no deduplication needed. Full relation resolution engine in Layer 3.

**Output:** Linked process graph.

### 3.4 Stages 4a/4b — Prequalify Notice + Qualify Lot

Qualification runs in **two phases** at different scope grains:

- **Phase A — `prequalify_notice`** (notice-grain): Runs before lot extraction. Uses notice-level text, CPV codes, and access route to make a provisional goods/services determination. If clearly non-goods or non-bidable → reject early, skip document acquisition and lot extraction. If qualified or ambiguous → proceed to document acquisition (stage 5) and lot extraction (stage 6).
- **Phase B — `qualify_lot`** (lot-grain): Runs after lot extraction. Per-lot qualification with document support, lot-level CPV codes, and addressable-value computation. Upgrades or downgrades the provisional notice-level qualification with lot-specific evidence.

Both phases use the same multi-dimensional goods classification logic below, but differ in scope grain and evidence quality (`qualificationBasis`).

**Step 1 — Expiry check:**
- Reject if deadline < 30 days from now
- **Exception:** DPS/Dynamic Market notices that accept new supplier applications do not expire in the same way — do not apply the 30-day filter
- Missing/unnormalizable deadline → route to manual review, not auto-pass

**Step 2 — Access route classification:**

| Access Route | Description | Actionability |
|-------------|-------------|---------------|
| `open_tender` | Open competitive procedure | Fully actionable |
| `restricted_tender` | Requires pre-qualification | Actionable if pre-qual open |
| `framework_call_off` | Under established framework | Only if on framework |
| `dynamic_market` | DPS / Dynamic Market (Procurement Act 2023) | If can join DPS/DM |
| `direct_award` | Without competition | Not actionable |
| `closed_framework_award` | Under closed framework | Not actionable unless on framework |
| `not_bidable` | Does not support participation | Not actionable |

Reject `direct_award`, `closed_framework_award`, `not_bidable` immediately. Flag `framework_call_off` and `restricted_tender` for access-route review.

**Step 3 — Multi-dimensional goods classification** (per lot, or per notice if no lots yet):

| Dimension | Values | Source |
|-----------|--------|--------|
| `legalCategory` | `goods` / `services` / `works` / `mixed` / `unknown` | CPV-based deterministic lookup first; AI for ambiguous |
| `goodsDensityScore` | 0.0–1.0 | CPV signal + text analysis (physical noun density, quantity/unit presence, attachment filenames) |
| `goodsSeparability` | `true` / `uncertain` / `false` | Lot structure analysis |
| `accessRoute` | See table above | Classified in step 2 |

**CPV deterministic lookup:**
- Divisions 14–44 → `goods`
- Division 45 → `works`
- Divisions 50–98 → `services`
- Software/licensing CPVs despite goods-like codes → `services`
- Multiple CPV codes spanning categories → `mixed`
- No CPV or ambiguous → `unknown`

**Step 4 — Qualification decision per lot:**
- **Pass:** Pure goods or mixed with separable goods + adequate density + accessible bid route
- **Review:** Mixed/unknown with strong goods density but weak separability, or works with identifiable goods packages but unclear award structure
- **Reject:** Services only, works with negligible goods density, or closed/non-bidable access route

**Step 5 — Addressable value:** For mixed contracts with separable goods, compute `addressableValuePence` at lot level reflecting the goods portion only. Do not use the whole contract value for margin computation.

**Step 6 — Qualification basis tracking:**

| Basis | Meaning |
|-------|---------|
| `notice_only` | From notice text and CPV codes only (provisional — may upgrade after doc acquisition) |
| `notice_plus_docs` | After tender documents available (stronger basis) |
| `lot_level` | At individual lot granularity from structured lot data |

Provisional `notice_only` qualification can still trigger document acquisition for ambiguous cases. Documents arriving later trigger re-qualification with `notice_plus_docs`.

**Output:**
- **Phase A (`prequalify_notice`):** `QualificationResult` per notice with `expiryQualified`, `goodsQualified`, `accessRoute`, `qualificationBasis = notice_only`. No `lotId` yet.
- **Phase B (`qualify_lot`):** `QualificationResult` per lot with `lotId`, `expiryQualified`, `goodsQualified`, `accessRoute`, `qualificationBasis = notice_plus_docs | lot_level`, `addressableValuePence`.

**Goods-led mixed work (install/maintenance) — two-phase value-share gate.** Shipped 2026-06-04 as part of the goods-led expansion (`packages/intelligence/src/qualification/goods-qualification.ts`). This is the qualification half of a two-phase model whose second phase runs at landed-cost synthesis (§3.11). The decision logic in `determineQualificationDecision` does three things:

- **Separable-mixed early value-share short-circuit.** For `legalCategory === "mixed" && goodsSeparability === true`, the engine computes the early **contract-value** goods share via `computeValueShareBps(addressableValuePence, totalValuePence)` and returns `pass` iff that share `>= GOODS_DOMINANCE_FLOOR_BPS` (`review` on a restricted/framework route), **else `review`** — deferring the borderline case to the Phase-2 landed-cost dominance gate. There is no unconditional density-only `pass`.
- **Phase-1 provisional-admit.** For goods-bearing `(mixed | works)` lots with `goodsSeparability !== false` and `Math.round(goodsDensityScore * 10000) >= GOODS_PRESENCE_FLOOR_BPS`, the lot is admitted to `review`. `GOODS_PRESENCE_FLOOR_BPS = 3500` is compared against `goodsDensityScore` scaled to bps (≈ a 0.35 presence floor) — it is a concrete value for the presence floor, not a new signal.
- **No-list / bespoke scope-exclusion flag** (`detectScopeExclusionFlags`): a no-list CPV family sets `out_of_scope_no_list` (reject only on a high-confidence CPV match, else review); a bespoke / made-to-measure keyword sets `out_of_scope_bespoke` → review. These back the §1.6 "route to review" admission notes.

The service-description gate (`packages/intelligence/src/customs/description-quality.ts`, in the L5 customs package) was split so a description carrying a goods head-noun **and** a supply signal **and** install/maintenance terms returns the `goods_led_mixed` verdict (continue) — `{ kind: "goods_led_mixed"; installationTerms: string[]; goodsHeadNoun: string; supplySignal: string }` — instead of `reject_service`; pure-service patterns still return `reject_service`. For the canonical examples: `supply and installation of steel shelving` has a goods head-noun, a supply signal, and install terms, so it continues as `goods_led_mixed` into the Phase-1 / Phase-2 dominance checks (with metal-shelving customs remedy evidence still required by §1.6); `grounds maintenance services` lacks the goods-led supply signal and stays `reject_service`.

**New operator strategy knobs:** `GOODS_PRESENCE_FLOOR_BPS = 3500` (Phase-1 goods-density presence floor) and `GOODS_DOMINANCE_FLOOR_BPS = 6000` (Phase-1 separable-mixed short-circuit and the Phase-2 dominance gate; env-overridable in the daemon via `GOODS_DOMINANCE_FLOOR_BPS`). Both are stated identically in §1.6, §3.11, and L5 §1.17.

### 3.5 Stage 5 — Acquire Documents

Download publicly accessible tender documents. In UK procurement, the notice body is often the **least useful** product document. The real signal lives in:

- Specification PDFs
- Pricing schedules
- Lot documents
- Bills of quantities (BoQ) spreadsheets
- Tender appendices

**Process:**
1. For each `prelim_qualified` notice, check for publicly accessible documents
2. Agent session (PTY-backed) navigates to source portal and identifies available documents
3. Agent downloads documents and outputs structured references
4. Documents stored as raw artifacts in local SQLite

**Sub-capabilities within Stage 5 (from L1):**
- **5a — Document download:** Raw document acquisition as described above
- **5b — Structural extraction:** Docling subprocess extracts markdown, sections, and tables from PDFs. Output stored in `document_extractions`. Cache-aware: identical content hashes reuse prior extraction results.
- **5c — Semantic extraction:** Claude pdf-extraction overlay skills (headless_contract mode) map structural output to domain-specific structured data. Output stored in `document_semantic_extractions`. Results enrich downstream Stages 6 (Extract Lots) and 7 (Extract Products) with document-derived evidence.

These are not new top-level pipeline stages but internal capabilities that enrich the document set before lot and product extraction.

**Document set output contract:** Produces a first-class `TenderDocumentSet` output:
- Which documents found and their roles: `spec` | `pricing_schedule` | `boq` | `appendix` | `other`
- Completeness state: `none` | `partial` | `full`
- `documentSetHash` computed from acquired document set — this is the invalidation key: if documents are re-acquired and hash changes, downstream stages (6–8) are hard-invalidated

If no public documents available → proceed with notice text only (lower extraction confidence, `qualificationBasis = notice_only`).

**Layer 0 stub:** Gold-set documents are pre-curated. Full implementation in Layer 1 (live source interaction).

### 3.6 Stage 6 — Extract Lots

Parse tender documents and OCDS lot data into structured lot records.

**Evidence inputs:** Notice metadata, `TenderDocumentSet`, and (from L1) `document_extractions` + `document_semantic_extractions` from the PDF extraction toolkit. Semantic extraction outputs for the `lots` target domain provide pre-extracted lot structures and requirements that enrich or validate OCDS-derived data.

**Process:**
1. Parse OCDS lot data from normalized notice (if structured lot fields present)
2. If no structured lots, use `lot-extraction` skill on acquired documents (augmented by semantic extraction output when available)
3. Create `NoticeLot` entities with: lot number, title, description, CPV codes, estimated value, deadline, documents, bidability
4. If no lots identifiable → create synthetic whole-notice lot
5. Run per-lot qualification if notice-level qualification was provisional

**Commit semantics:** **Set-replacement** — a notice's lot set is one coherent decomposition. Partial results (lots 1–7 of 12) stored as checkpoint artifacts but do NOT advance stage head. Must succeed for complete lot set or fail entirely.

### 3.7 Stage 7 — Extract Products

Per-lot product candidate extraction.

**Step 1 — Classify `candidateType`:**

| Candidate Type | Description | Marketplace Routing |
|---------------|-------------|-------------------|
| `exact_sku` | Known brand/model/GTIN | eBay, Amazon (primary); China only if equivalent allowed |
| `standard_commodity` | Generic goods identifiable by standards/specs | Full search all routed sources |
| `brand_family` | Brand-specific but not model-exact | eBay, Amazon; Alibaba if generic equivalent exists |
| `bundle` | Mixed set of products | Decompose first, search top 2–3 sub-items |
| `custom_or_fabricated` | Bespoke/custom-manufactured | Skip marketplaces — RFQ/manual only |
| `installed_goods` | Goods + installation inseparable | Skip marketplaces — RFQ/manual only |
| `regulated_goods` | Requires specific standards/certifications | Full search but compliance verification mandatory |
| `unknown` | Insufficient data to classify | Generic matching with wider budgets |

**Step 2 — Classify `matchArchetype`:** Orthogonal to `candidateType` — describes how product identity is recovered on marketplaces.

| Match Archetype | Identity Recovery Strategy | Example Products |
|----------------|--------------------------|------------------|
| `standard_class_led` | Standards family + class + compliance evidence | PPE, medical consumables, workwear, fire safety |
| `dimension_material_led` | Physical dimensions + material + fire/load rating | Desks, lockers, bins, shelving, racking |
| `electrical_spec_led` | Voltage, wattage, plug type, IP rating, connector | Work lights, PSUs, chargers, switchgear |
| `pack_count_led` | Unit/pack basis + material + disposability | Gloves, wipes, bags, cable ties, bin liners |
| `compatibility_led` | Parent product reference + part number | Filters, toner, printer parts, replacement blades |
| `model_exact` | Brand + model number + generation/revision | IT equipment, branded power tools, instruments |

If extraction cannot determine archetype → default to `unknown` (triggers generic matching with wider shortlist budgets).

**Step 3 — Extract structured slots:**
- `headNoun` — primary product noun
- `criticalAttributes` — key distinguishing attributes
- `standards` — normalized standard references (family/part/edition/amendment/class)
- `numericSpecs` — dimensions, weights, electrical specs
- `material` — primary material
- `brandModel` — brand and model if applicable
- `preferredTerms` / `excludedTerms` — from tender language

**Step 4 — Assign slot criticality per extracted attribute:**

| Role | Meaning | Matching Behavior |
|------|---------|-------------------|
| `required` | Tender mandates ("must", "shall", "required") | Hard reject on conflict; review-flag on absence |
| `preferred` | Tender prefers ("preferred", "desirable") | Confidence penalty on conflict; no penalty on absence |
| `descriptive` | Informational only | No matching impact; stored for evidence |
| `exclusion` | Explicitly excluded ("not", "excluding") | Hard reject on presence |

When ambiguous → default to `preferred`. `matchArchetype` provides category-level defaults: for `standard_class_led`, standards slots default to `required`; for `dimension_material_led`, key dimension slots default to `required`.

**Step 5 — Normalize standard references:** Parse into structured components: family (EN 397), part, edition year, amendment (A1:2012), class/level. Enables precise comparison during hard-conflict filtering.

**Step 6 — Bundle decomposition:** Decompose bundles into constituent candidate families. Record `parentCandidateId` and `candidateRole` (`standalone` | `bundle_parent` | `bundle_component`). Persist `candidateValueShareBps` for top-value search selection and coverage rollups.

**Step 7 — Confidence scores:** Assign `identityConfidence`, `specConfidence`, `quantityConfidence` separately.

**Step 8 — Provenance:** Store `sourceEvidence` (document/page/span) so downstream reviewers can trace fields back to tender evidence.

**Constraint:** `lotId` is **required** — every candidate must trace to a specific lot (or synthetic whole-notice lot).

### 3.8 Stage 8 — Score Quoteability

Deterministic rules-based gate per candidate.

**Quoteability score formula:** Combines:
- Commodity-like head noun
- Critical attributes completeness
- Numeric specs presence
- Standards presence
- Quantity/unit clarity
- Public docs available
- Open access route

Minus penalties for: custom fabrication, installation scope, closed framework, missing compliance detail.

**Thresholds:**

| Score Range | Action |
|-------------|--------|
| ≥ 0.65 | Search automatically — full query waves generated |
| 0.45–0.64 | Search top-value candidates only (by `candidateValueShareBps`). Query waves generated. |
| < 0.45 | Skip marketplace search. Query waves generated for RFQ reference. Operator can trigger manual search. |

**Recall risk score** (`recallRiskScore`): Per candidate, from alias drift for the `matchArchetype`, compliance criticality, accessory contamination risk, spec sparsity, source noisiness (1688 > Alibaba > Amazon > eBay), language mismatch risk. Drives adaptive shortlist budgets in stage 9.

**SearchProfile generation:** For **ALL candidates** (not just auto-searched):
- Head noun family, `matchArchetype`
- Noun aliases (EN/ZH), market framing terms (EN/ZH)
- Required/optional anchors, exclusion terms
- Use context, sector context
- `searchEligibilityState`: `auto_search` | `manual_only` | `skip`

Creating SearchProfiles for all candidates preserves search intent for below-threshold candidates, enabling RFQ re-entry in Layer 2.

### 3.9 Stage 9 — Price-Signal Match (Query-Wave Search)

Marketplace search via agent skill-based browser scraping. The durable business unit is the **candidate-source pair**.

#### 3.9.1 Source Routing

Route by `candidateType` — not every source for every candidate:

| Candidate Type | UK Sources | China Sources | Notes |
|---------------|-----------|--------------|-------|
| `exact_sku` | eBay, Amazon | Only if equivalent allowed | SKU/GTIN search |
| `standard_commodity` | eBay, Amazon | Alibaba, 1688 | Full search |
| `regulated_goods` | eBay, Amazon | Alibaba, 1688 only when compliance slots populated | Must verify compliance equivalence |
| `brand_family` | eBay, Amazon | Alibaba if generic equivalent exists | Brand-specific |
| `bundle` | Decompose first | Decompose first | Top 2–3 sub-items only |
| `custom_or_fabricated` | Skip | Skip | RFQ/manual only |
| `installed_goods` | Skip | Skip | RFQ/manual only |

**Price-signal roles:**

| Source | Typical Role | Implication |
|--------|-------------|-------------|
| Amazon/eBay UK | Benchmark ceiling — UK retail/resale, usually VAT-inclusive | Upper-bound UK market price |
| Alibaba | Optimistic source-side floor — MOQ prices, often ex-works/FOB | Lower bound for supplier cost |
| 1688 | Domestic China wholesale floor — most optimistic | Commodity existence signal |
| RFQ/negotiated | Executable or near-executable quote | Decision-grade pricing |

The system computes a **margin band** (floor to ceiling), not fake precision from one UK and one China listing.

#### 3.9.2 Query-Wave Planning

For each candidate × source, the query planner generates ordered waves from the search profile:

| Wave | Strategy | Precision/Recall |
|------|----------|-----------------|
| **0 — Exact anchor** | Head noun alias + strongest anchor (model number, standard, key spec) | High precision, low recall |
| **1 — Alias swap** | Change noun phrase, keep anchor. Tests marketplace vocabulary variants. | Medium |
| **2 — Market framing** | Add use-context/industry terms from `marketFrameEn`/`marketFrameZh` | Medium |
| **3 — Broad existence** | Drop soft modifiers. Keep only head noun family + one anchor. | Low precision, high recall |
| **4 — Compliance-off** | (Regulated goods only) Remove standard token, keep as post-filter. Many compliant listings omit standards from titles. | Recovery |

Each wave records: query text, source, locale, wave number (for debugging and recall measurement).

**Source-specific search behavior:**
- **eBay:** Category-prior search (head noun → eBay category suggestion). Fixed-price listings preferred. Short title-like phrases (3–7 tokens).
- **Amazon:** Alias families — multiple short noun-phrase variants. Rewards synonyms and abbreviations over verbose descriptions.
- **Alibaba:** Filter-heavy and sourcing-oriented. Category, supplier type, MOQ range, shipping option, certifications, price range, supplier location as first-class parameters. "Ready to Ship" listings get `quoteUsabilityConfidence` bump.
- **1688:** Chinese-keyword-only. Short keyword clusters (2–4 terms). Primarily commodity existence signals and domestic wholesale floor prices.

#### 3.9.3 Search-Card Screening

Parse results and apply hard-conflict filters using **four evidence states** per required slot:

| State | Meaning | Action |
|-------|---------|--------|
| `match` | Listing explicitly satisfies slot requirement | Pass |
| `missing` | No evidence either way | Confidence penalty (not disqualification) |
| `ambiguous` | Present but unclear/partially matching | Confidence penalty |
| `conflict` | Explicitly contradicts requirement | **Hard rejection** |

**Hard-conflict matrix:**

| Conflict Type | Hard Reject When | Penalty/Review When |
|--------------|------------------|---------------------|
| Brand/model | Different brand/model on `exact_sku` or locked brand | Brand/model absent |
| Standard/certification | Wrong family; wrong part; class below required | Same family, edition/amendment absent |
| Voltage/frequency/plug | Incompatible mandatory electrical spec | Missing electrical spec |
| Material | Contradiction on mandatory material (nitrile vs latex) | Material missing or non-mandatory |
| Dimensions/size | Incompatible fixed size or range | Size missing; adjustable size |
| Condition | Used/refurbished when not allowed | Condition unclear |
| Accessory vs primary | Accessory/liner/refill/replacement/shell-only listing | Bundle or unclear title |
| Consumer vs industrial | Sports/toy/fashion/domestic variant conflicting with industrial use | Weak industrial signal only |
| Pack basis | Wrong product form (single vs pair, consumable vs kit) | Unclear pack/unit basis |

**Standards normalization for comparison:** Parse into family/part/edition/amendment/class. Rules: wrong family or part = hard reject; same family with amendment absent = soft conflict; edition absent = penalty; class below required = hard reject; class absent = review/penalty.

**Consumer vs industrial detection:** Do NOT use price as primary signal. Use: category path, standard family (EN 1078 cycling vs EN 397 industrial), title vocabulary (bike/climbing/kids = hard reject for industrial PPE), seller assortment, detail-page intended-use language, structured spec attributes.

#### 3.9.4 Detail-Page Enrichment

For results surviving search-card screening as plausible matches, fetch detail page to extract:
- Structured spec table / product attributes
- Standards/certification mentions not visible on search card
- Seller information and assortment context
- Pack/unit details, shipping and MOQ specifics
- Intended-use language

**Compliance evidence tier determination:**

| Tier | Evidence Source | Matching Implication |
|------|---------------|---------------------|
| `structured_attribute` | Spec table or attribute fields | Strong — eligible for automatic match |
| `title_body_mention` | Title or body text only | Moderate — confidence penalty |
| `absent` | No compliance evidence | No auto-match for `regulated_goods`; confidence cap for others |

For `regulated_goods`, missing compliance on search card alone does **not** disqualify — detail-page check is mandatory before rejection.

#### 3.9.5 Adaptive Diversified Shortlist

After hard-conflict filtering and detail-page enrichment, build shortlist from de-duplicated union across all waves. Budget is adaptive per `recallRiskScore`:

| Recall Risk | Deterministic Shortlist | AI Adjudication Budget |
|-------------|------------------------|----------------------|
| Low | Top 8 | 0–2 borderline |
| Medium | Top 12 | 3 |
| High | Top 20 | 5–8 |

**Diversity enforcement rules** (before rank truncation):
- Max 2 results per seller
- Max 2 results per query wave
- Max 1 result per normalized title family (near-identical titles after brand/seller removal)

Without diversity enforcement, shortlists for noisy categories (PPE, lighting, accessories) fill with near-duplicates that waste AI adjudication budget.

#### 3.9.6 AI Adjudication (Borderline Only)

Agent judges pre-filtered borderline results from deterministic shortlist. AI does NOT perform new searches — it evaluates identity and usability of candidates the deterministic scorer couldn't confidently classify. Budget scales with `recallRiskScore`.

#### 3.9.7 Dual Confidence Scoring

Per result:

```
matchConfidence = 0.40 * product_type_match
               + 0.30 * critical_attribute_match
               + 0.20 * numeric_spec_match
               + 0.10 * evidence_quality
```

Weights are MVP priors. Archetype-specific profiles adjust them (e.g., `model_exact` shifts toward product_type_match; `dimension_material_led` shifts toward numeric_spec_match).

```
quoteUsabilityConfidence = 0.35 * unit_basis_clarity
                        + 0.25 * MOQ_compatibility
                        + 0.20 * shipping_visibility
                        + 0.20 * completeness
```

**Never average these.** Apply lexicographic gating: identity gate first (`matchConfidence` floor by archetype), then usability gate, then price-signal role assignment.

**Lexicographic gating table:**

| matchConfidence | quoteUsabilityConfidence | priceSignalRole | Action |
|----------------|------------------------|----------------|--------|
| High (≥ 0.75) | High (≥ 0.70) | any | Landed-cost candidate |
| High (≥ 0.75) | Medium (0.45–0.69) | `benchmark_ceiling` / `optimistic_floor` | Screening-only price signal |
| High (≥ 0.75) | Low (< 0.45) | any | Identity evidence only, not pricing |
| Medium (0.50–0.74) | High (≥ 0.70) | any | AI/manual review of identity |
| Low (< 0.50) | any | any | Reject |

**Regulated override:** For `regulated_goods`, automatic match acceptance additionally requires `complianceEvidenceTier` of `structured_attribute` or `title_body_mention`, regardless of confidence scores.

**Archetype-specific confidence floors:**

| Match Archetype | Identity Floor | Key Requirement |
|----------------|---------------|-----------------|
| `model_exact` | ≥ 0.85 | Exact brand/model match |
| `standard_class_led` | ≥ 0.75 | Explicit compliance evidence |
| `compatibility_led` | ≥ 0.75 | Parent product reference resolves |
| `electrical_spec_led` | ≥ 0.70 | Mandatory electrical specs match |
| `dimension_material_led` | ≥ 0.65 | Key dimensions within tolerance |
| `pack_count_led` | ≥ 0.55 | Higher pack/unit usability requirements |

#### 3.9.8 Zero-Result Recovery Ladder

When search returns zero usable results for a candidate × source:

| Round | Strategy |
|-------|----------|
| 1 | Drop weakest modifier. Keep head noun + one hard anchor. |
| 2 | Alias swap: next alias from search profile. Keep hard anchor. |
| 3 | Market framing: add use-context/industry terms from `marketFrameEn`/`marketFrameZh`. |
| 4 | Compliance-off: remove standard token, keep as post-filter (regulated goods). |
| 5 | China language escalation: Alibaba EN → Alibaba ZH → 1688 ZH. |
| 6 | Broad existence: category-family term only (head noun family + source). |

**Stop conditions:**
- Max 2 recovery rounds per source beyond initial waves
- Per-candidate query budget ceiling (higher for high-value/high-risk)
- Stop immediately if broad queries find plausible items — problem is specific query, not market

**Search failure classification:**

| Failure Mode | Diagnosis | Implication |
|-------------|-----------|-------------|
| `vocabulary_gap` | Broad queries find products, specific miss | Alias expansion needed |
| `overconstrained_query` | Too many anchors for source search quality | Relax constraints |
| `accessory_swamp` | Source returns mostly accessories/parts | Add negative terms; adjust routing |
| `compliance_not_surfaceable` | Product exists but compliance not searchable | Detail-page-only check; longer budget |
| `no_marketplace_presence` | All broad queries across all sources return nothing | Genuine absence — RFQ/manual |
| `low_extraction_quality` | Candidate description too vague | Upstream issue — confidence penalty |
| `source_specific_failure` | Website error, rate limit, layout change | Retry later; error manifest |

**Key distinction:** Broad queries finding products but specific queries missing = `vocabulary_gap` (query problem). All broad queries returning nothing = `no_marketplace_presence` (market absence). Blocked empty shells, soft blocks, challenge pages never count as `no_marketplace_presence` — they stay in source-failure path.

#### 3.9.9 Output

Save quotes with dual confidence, compliance evidence tier, price-signal role, and `SearchAttempt` metadata to local SQLite. Persist each wave execution in `search_attempts`. A listing can be a valid match without becoming a `SupplierQuote` — only screened, normalized, commercially usable results are persisted as quote rows.

### 3.10 Stage 10 — Gross Margin Gate

Three-gate lexicographic confidence check before margin computation.

#### 3.10.1 Revenue Normalization (Prerequisite)

Classify the revenue basis and compute expected addressable revenue:

```
revenue_basis_type = classify(notice):
  "firm_contract"       -> direct award or single-winner tender with firm value
  "estimated_contract"  -> tender with estimated (not guaranteed) value
  "framework_ceiling"   -> framework agreement ceiling (not guaranteed revenue)
  "dynamic_market"      -> DPS/dynamic market admission (no volume guarantee)
  "concession"          -> concession contract
  "unknown"             -> insufficient data

expected_addressable_revenue_ex_vat = normalize(
  raw_notice_value,
  vat_inclusive = notice.estimatedValueVatInclusive,
  basis_type = revenue_basis_type
)

revenue_confidence = {
  "firm_contract": 0.90, "estimated_contract": 0.70,
  "framework_ceiling": 0.40, "dynamic_market": 0.30,
  "concession": 0.50, "unknown": 0.20
}[revenue_basis_type]
```

For mixed contracts with separable goods, use the **goods-addressable value** from qualification (stage 4), not the whole notice value.

Revenue normalization is computed **once** in the gross margin gate and persisted for reuse by landed cost and net margin stages.

**Reference:** [Contract valuation guidance](https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-define-phase/guidance-valuation-of-contracts-html)

#### 3.10.2 Quote Normalization (Prerequisite)

Normalize all supplier quotes to a common EXW cost boundary:

```
normalized_supplier_cost = normalize_to_exw(
  quoted_price,
  incoterm = quote.quoteCostBoundary,
  estimated_inland_freight, estimated_ocean_freight, estimated_insurance
)
```

Incoterm determines what's included: EXW (buyer pays everything from factory), FOB (seller pays to port + export clearance), CIF (seller pays freight + insurance to destination), DDP (seller pays everything incl. duty + VAT). Without normalization, comparing EXW to CIF is inconsistent.

#### 3.10.3 Quote Selection

Select best supplier quote(s) per candidate via lexicographic confidence gating:

1. **Identity floor first:** `matchConfidence >= archetype floor` (see §3.9.7)
2. **Usability floor second:** `quoteUsabilityConfidence >= 0.45`
3. **Realism third:** A quote can be realistic but still wrong product or wrong pack basis

A realistic-looking price for the WRONG product must not pass.

#### 3.10.4 Gross Margin Computation

```
gross_margin = (expected_addressable_revenue_ex_vat - normalized_supplier_cost)
             / expected_addressable_revenue_ex_vat

pricing_input_confidence = min(
  match_confidence,
  quote_usability_confidence,
  quote_realism_confidence
)
gross_estimate_confidence = min(revenue_confidence, pricing_input_confidence)

if gross_margin >= 0.30 and gross_estimate_confidence >= confidence_floor:
  qualify_for_landed_cost_stage
else if gross_estimate_confidence < confidence_floor:
  route_to_manual_review
else:
  reject_or_deprioritize
```

Initial `confidence_floor` for automated progression: `0.60` (calibrate against operator outcomes).

**Output:** `CandidatePricingAssessment` entity with `selectedQuoteId`, `selectedQuoteRowVersion`, `revenueBasisType`, `expectedAddressableRevenueExVatPence`, `grossMarginBps`, and confidence-gate decisions.

### 3.11 Stage 11 — Landed Cost (Screening-Grade)

Screening-grade: HMRC Tariff API + flat freight + single FX rate. Decision-grade engine with multi-source recall, 10-digit resolution, and measure interaction engine is Layer 5.

**Landed-cost estimates are scoped to candidate + selected quote** — each estimate anchored to one commercial scenario. The `supplierQuoteId` field is **required**.

#### 3.11.1 Screening-Grade Classification (Layer 0)

1. Read persisted stage 10 output: selected quote, normalized supplier cost, revenue basis
2. Query HMRC Trade Tariff API for HS6-level duty rate (MFN)
3. Apply flat freight assumption (single mode)
4. Use single FX rate for screening

**Conservative defaults (constitutional — these are non-negotiable):**

| Scenario | Default | Rationale |
|----------|---------|-----------|
| No origin proof | MFN rate (not preferential) | Safety |
| No exporter proof | Residual remedy rate | Worst-case trade remedy exposure |
| No quota allocation | Out-of-quota rate | Prevents false optimism |
| Unresolved VAT status | Standard VAT rate | Conservative |
| Unresolved Incoterm | Boundary scenarios (not point normalization) | Avoids false precision |
| Unresolved supplementary units | Abstain on duty calculation | Cannot compute without units |

#### 3.11.2 Decision-Grade Classification (Layer 5 — Not Active)

The full pipeline (documented here for completeness — NOT active in Layer 0):

1. Description-quality gate with archetype-keyed missing-facts request on rejection
2. Attribute extraction
3. **Multi-source candidate recall:** HMRC FPO API, CPV→chapter priors, rule-based hints, tariff keyword search, ATR/codebook reuse
4. Cluster and rank across sources with `candidateRecallAgreement`
5. **Expand all plausible families to all declarable UK 10-digit descendants** via HMRC Trade Tariff API — never greedy descent of top candidate only
6. Rerank descendants by slot + measure compatibility
7. Validate via Taric Support API, cross-check via HS Ping
8. Check `tariffCheckedAt` and `tradeRemedyWatchState` for classification freshness

#### 3.11.3 Landed Cost Computation

```
economicLandedCostGbp = supplierCost + freightBase + customsDuty
  + tradeRemedyDuty + ukFinalMile + otherFees    (ex-recoverable-VAT)
```

**Remaining cost components:**
- `ukFinalMileCostGbp` — UK domestic delivery (port/airport to contract delivery address)
- `otherFeesGbp` — customs brokerage (~£30–150/entry), port/terminal handling (~£150–400/container), insurance (0.5–2% CIF), quality inspection if applicable

**FX conversion:**
- **HMRC Exchange Rate Service** for customs duty calculation (`customsFxRate`, legally authoritative, monthly rolling)
- **Xe** for commercial margin estimation (`commercialFxRate`, real-time)
- Compute and store `fxBasisDeltaBps` between the two rates

**VAT determination:**
- `vatRateState`: `standard` | `reduced` | `zero` | `unresolved` (default to standard when unresolved)
- Import VAT recovery treatment: postponed (full/partial), cash-paid (full/partial), non-recoverable
- `vatRecoveryRate` — proportion recoverable
- `valuationMode`: `estimation` | `declaration` — distinguishes marketplace-derived proxy values from actual transaction prices

#### 3.11.4 Goods-Led Installation Labour & Phase-2 Dominance Gate (Goods-Led Extension)

Shipped 2026-06-04 with the goods-led supply-and-install / maintenance expansion. This is the second phase of the two-phase goods value-share model whose first phase runs in qualification (§3.4). It is documented in full in `layer-5-expanded-commercial-intelligence.md` §1.17.

**Installation-labour estimation step** (after FX, before decision synthesis): for goods-led mixed work, the daemon invokes the `http_public` worker skill `skills/landed-cost/uk-installation-labour-estimate/` (transient-loaded in `run_landed_cost_live_loop` via the `landed_cost_uk_installation_labour` job kind; not registered in `sources/registry.rs`, not chained via `next_job_kind`). The skill reads curated skill-local rate tables (region modelled as a multiplier on a Midlands baseline) plus bounded LLM archetype inference — **not** Bills of Quantities, ASHE/ONS wage APIs, or a CDP/browser skill. It returns labour and permit/traffic-management cost as base + banded (`_low`/`_high`) values with a confidence and a `permit_responsibility` (default `unknown` → conservative high band). Labour and permit base costs are summed via `.saturating_add` into `economic_landed_cost_gbp_pence` at both `build_domestic_landed_cost_estimate` and `build_import_landed_cost_estimate`. The extended computation:

```
economicLandedCostGbp = supplierCost + freightBase + customsDuty
  + tradeRemedyDuty + ukFinalMile + otherFees
  + installationLabourBase + permitTrafficMgmtBase   (goods-led extension)
  (ex-recoverable-VAT)
```

Labour/permit line items are carried as a JSON breakdown (`labour_line_items`), not normalized tables; only `maintenance_period_estimates` is a persisted entity.

**Phase-2 goods-dominance gate:** the daemon applies `GOODS_DOMINANCE_FLOOR_BPS` (default 6000 / 60%, env-overridable) after landed-cost synthesis. The persisted share carries an explicit `goods_value_share_basis` using exact stored values: `contract_value` when addressable/total value evidence is available, otherwise `cost_proxy` computed as `goods ÷ (goods + labour + permits)` (`computeGoodsValueShareBps` in `packages/intelligence/src/margins/coverage-rollup.ts`, null if any component is unknown). The goods-only subtotal `goods_landed_cost_gbp_pence` is persisted so the cost-proxy ratio is reconstructable from one row. A `blocked` labour outcome degrades the opportunity to review rather than crashing or fabricating a number. Recurring maintenance is held as a separate annualised `maintenance_period_estimates` row and is never capitalised into the dominance denominator.

### 3.12 Stage 12 — Net Margin Gate

Net margin with **low/base/high scenarios**:

```
net_margin_low = (revenue_ex_vat - supplier_cost*(1+negotiation_uplift)
  - freight_high - customs_duty_high - uk_final_mile - other_fees)
  / revenue_ex_vat

net_margin_base = (revenue_ex_vat - supplier_cost
  - freight_base - customs_duty_base - uk_final_mile - other_fees)
  / revenue_ex_vat

net_margin_high = (revenue_ex_vat - supplier_cost*(1-volume_discount)
  - freight_low - customs_duty_low - uk_final_mile - other_fees)
  / revenue_ex_vat

cash_to_clear = supplier_cost + freight_base + customs_duty_base
  + uk_final_mile + other_fees
  + import_vat * (1 - vat_recovery_rate)     # non-recoverable
  + import_vat * vat_recovery_rate            # ties up cash
```

**Gate logic:** Use CONSERVATIVE (low) estimate for auto-qualification:

```
if net_margin_low >= 0.20 and no critical_blockers
   and net_qualified_coverage_bps >= process_coverage_floor_bps:
  final_qualified
else if net_margin_base >= 0.20 and (indicative_only or has_blockers):
  manual_review_required / rfq_required
else:
  not_qualified
```

**Critical blockers** (override margin regardless of value):

| Blocker | Trigger |
|---------|---------|
| `classification_unstable` | HS6 candidates diverge or 10-digit unresolved |
| `trade_remedy_detected` | Anti-dumping/countervailing/safeguard (any tier) |
| `measure_condition_unsatisfied` | Operational blocker from unsatisfied import conditions |
| `revenue_basis_uncertain` | Framework ceiling or dynamic market — no volume guarantee |
| `pva_ineligible` | Cannot recover import VAT |
| `customs_branch_divergent` | Plausible UK10 branches split on remedy/no-remedy, specific/ad-valorem, VAT family, or supplementary units |
| `exporter_identity_unresolved` | Active remedy but no exporter proof → residual assumed |
| `classification_stale` | Tariff data expired or TRA watch state changed |

**Customs hard rule:** If plausible branches introduce remedy/no-remedy split, unresolved specific duty, quota/safeguard interaction, or different VAT family → suppress decision-grade margin. A 2.7%–48.5% customs branch spread is a **blocker**, not a scenario band.

### 3.13 Coverage Rollup (Inline Computation, Not a Tracked Stage)

Process-level business rollup computed from candidate-level results. **This is not a separately tracked pipeline stage** — it runs inline as part of `compute_net_gate` and `generate_manifest` job execution. It has no entry in `process_stage_status`, `workflow_jobs.kind`, or the Stage Scope Matrix. The §3 numbering is retained for reference continuity only.

**These are the authoritative coverage metric definitions:**

| Metric | Formula | Meaning |
|--------|---------|---------|
| `pricedCoverageBps` | (lots with ≥1 priced candidate / total qualifying lots) × 10000 | Pricing breadth (lot-count) |
| `landedCostCoverageBps` | (lots with landed cost / total qualifying lots) × 10000 | Landed-cost breadth (lot-count) |
| `netQualifiedCoverageBps` | (lots passing net gate / total qualifying lots) × 10000 | Net-qualified breadth (lot-count) |
| `pricedValueCoverageBps` | (sum of priced lot values / total qualifying lot value) × 10000 | Pricing breadth (value-weighted) |
| `landedCostValueCoverageBps` | (sum of landed-cost lot values / total qualifying lot value) × 10000 | Landed-cost breadth (value-weighted) |
| `netQualifiedValueCoverageBps` | (sum of net-qualified lot values / total qualifying lot value) × 10000 | Net-qualified breadth (value-weighted) |
| `unresolvedMandatoryLots` | Count of mandatory lots without resolution | Gap count |
| `decisionBasisHash` | SHA-256 of pipeline data the operator would review | Staleness detection |

**Goods-led extension coverage metrics** (persisted on `pipeline_assessments`, added at epoch 10 by `migrations/0014`):

| Metric | Formula / Meaning |
|--------|-------------------|
| `goodsValueShareBps` | `goods ÷ (goods + labour + permits)` × 10000 (`computeGoodsValueShareBps`); null when any component is unknown. Gated by `GOODS_DOMINANCE_FLOOR_BPS` (6000). |
| `goodsValueShareBasis` | Basis enum for the share above — exact stored values `cost_proxy` vs `contract_value` (`contract_value` preferred when available). |
| `installationLabourCoverageBps` / `installationLabourValueCoverageBps` | Lot-count and value-weighted coverage of lots that received an installation-labour estimate. |
| `permitTrafficMgmtCoverageBps` / `permitTrafficMgmtValueCoverageBps` | Lot-count and value-weighted coverage of lots with a permit / traffic-management estimate. |

**Auto-qualification coverage floor:** Uses **value-weighted** coverage: `netQualifiedValueCoverageBps >= 5000` (50% of qualifying value). Lot-count coverage is displayed but value coverage gates. A process with 9/10 lots qualified but the unresolved lot representing 80% of value cannot auto-qualify.

### 3.14 Stage 14 — Manifest Generation

Per-process opportunity manifest from pipeline results.

**Layer 0:** Minimal local output (evaluation harness report).
**Layer 1:** Local markdown artifact.
**Layer 2:** Operator-facing rendered manifest via `packages/manifests` renderer.

### 3.15 Stage 15 — Operator Decision

Approve/reject/watch disposition per process.

**Layer 0:** No operator surface (evaluation harness only).
**Layer 2:** Local intent queue in `pending_operator_intents`.
**Layer 4:** Remote-owned `operator_decision_heads` with Supabase CAS.

### 3.16 Confidence Model

Weighted confidence score for internal pipeline routing (not for operator display):

```
final_confidence =
  (0.20 * classification_stability) +
  (0.20 * quote_realism) +
  (0.20 * commodity_code_confidence) +
  (0.15 * freight_mode_stability) +
  (0.15 * revenue_reliability) +
  (0.10 * fx_freshness)
```

This composite applies **only after** upstream pipeline gates (qualification, extraction, matching). Weights are MVP priors — recalibrate against observed operator decisions.

**Confidence bands:**
- `>= 0.80` → high confidence
- `0.60–0.79` → usable with review
- `< 0.60` → manual review required

**Critical blocker override:** A high composite score MUST NOT hide a red classification, revenue, or trade-remedy blocker. Any critical blocker present (see §3.12 blockers table) → cannot auto-qualify regardless of score.

**Operator display:** Show bands + component panel + blockers, NOT a single decimal number.

### 3.17 Bilingual Lexicon

The cross-language matching layer — a curated lexicon, NOT a grand taxonomy.

**Scope:** Top 30–50 spend-heavy searchable product families. Per family:
- 5–15 EN noun aliases (marketplace vocabulary, not procurement vocabulary)
- 5–15 ZH noun aliases / market-frame aliases
- 5–20 negative terms
- 3–10 unit/pack aliases
- A few standard/class patterns

Total: **hundreds of curated entries**, not tens of thousands.

**Market framing slots on each SearchProfile:**
- `marketFrameEn` — English marketplace framing (e.g., "industrial", "commercial", "heavy duty")
- `marketFrameZh` — Chinese marketplace framing (e.g., `劳保` for PPE, `工地` for construction, `工业` for industrial)
- `useContext` — end-use context disambiguating otherwise identical products
- `sectorContext` — industry sector affecting terminology and compliance expectations

**Cross-language matching rules:**
1. Normalize candidate into structured slots
2. Expand English synonyms for marketplace terms (not procurement vocabulary)
3. Translate head noun + critical modifiers + market framing context + standards descriptors to Chinese
4. Keep numerics and standards codes unchanged (EN 397, EN 455, dimensions, voltages are universal)
5. Do NOT translate prose. Do NOT rely on machine-translated paragraphs.
6. Run dual-language queries for China sources: English first on Alibaba (broad recall), Chinese on 1688
7. **Chinese retry on Alibaba:** When English returns zero/only accessories/only consumer items → retry with Chinese queries

**Bilingual unit dictionary:** For quantity and MOQ handling:
- `件 / 个 / 只 / 双 / 套 / 箱 / 包 / 卷 / 瓶 / 桶 / 台`
- Normalize Chinese marketplace units to the candidate's procurement unit

**Lexicon maintenance model:**
1. Seed manually from procurement category knowledge
2. Bootstrap from operational data: accepted matches → confirmed aliases; rejected false positives → negative terms; zero-result cases → vocabulary gaps; operator edits → refined framing
3. Alias promoted to production lexicon only when operator-confirmed or repeatedly associated with accepted matches

### 3.18 Pipeline Cost Management

**Problem:** Naive design = 1,200+ sessions/day at $0.18–$0.41 = $75–$170/day.

**Optimizations:**

| Strategy | Detail |
|----------|--------|
| **Batch by source** | One ingestion session per source per run. One supplier session per source per notice. |
| **Search selectively** | Top 2–3 candidates per notice by quoteability/spend share. Hard cap: 8 auto-searched per notice. Skip 1688 unless commodity-type. |
| **Deterministic first** | Hard-conflict filters and shortlisting are deterministic. AI adjudication scales with `recallRiskScore` (0–2 low risk, up to 5–8 high risk). |
| **Two-tier cache** | Separate search-result cache from listing-evidence cache (see below). |
| **Model tiering** | Mini/Haiku for classification/extraction. Premium only for ambiguous adjudication. |
| **Target throughput** | 20–30 priced notices/day after filtering. Target $20–$60/day metered. |

**Default operating limits (MVP):**
- 2 browser slots, 4 total agent slots
- 1 concurrent procurement session per source
- 2 concurrent supplier sessions per source
- Max 25 extracted candidates per notice
- Max 8 auto-searched candidates per notice
- Max 3 supplier sources per candidate unless operator forces wider sweep
- Max 2 extra recovery rounds per source beyond initial waves

**Two-tier cache:**

| Layer | Stores | Invalidation | Reuse |
|-------|--------|-------------|-------|
| **Search-result cache** | Per query: source, locale, text, wave number, listing IDs, rank, count, timestamp | Hard fingerprint change | Same hard fingerprint only |
| **Listing-evidence cache** | Per listing ID: normalized title, structured attributes, seller, MOQ, shipping, standards, units, compliance tier | Source-specific TTL | Cross-candidate, cross-process via `ProcessRelation` edges |

Source-specific freshness TTLs:

| Source | Search-Result TTL | Listing-Evidence TTL |
|--------|------------------|---------------------|
| eBay UK | 24h | 48h |
| Amazon UK | 36h | 48h |
| Alibaba | 48h | 72h |
| 1688 | 72h | 96h |
| Ready-to-Ship (Alibaba) | 36h | 48h |

Cached results carry **decayed commercial confidence**: identity evidence is stable, but price/shipping/stock freshness decays with age relative to TTL.

**Query fingerprint design:**
- **Hard fingerprint** (identity-defining): source, locale, `headNounFamily`, `matchArchetype`, required anchors, compliance-required flag, negative family. Change → full cache miss.
- **Soft fingerprint** (optional modifiers): optional anchors, market framing terms, cosmetic descriptors. Change → re-rank cached results only.

### 3.19 Recall Measurement

For every operator-accepted match, log: source, wave number, raw rank, reranked position, AI adjudication pool membership, `matchArchetype`, `candidateType`.

Compute weekly/monthly:
- recall@3 (AI pool), recall@10 (deterministic), recall@20 (deterministic) — by `matchArchetype × source`
- Operator override rate (matches accepted that system would have rejected)
- False-positive reason mix
- Score histogram and precision by score decile
- Reliability curve (predicted confidence vs actual acceptance rate)

**Calibration approach (MVP):** Keep deterministic rules with explicit feature logging. When 200+ labeled decisions per archetype × source accumulate, fit simple calibration layer (isotonic regression or Platt scaling) per source × `matchArchetype`, backed off to pooled calibrator when sparse. Store `rawMatchScore` alongside calibrated `matchConfidence` for auditability.

### 3.20 Stage Boundary Contracts (Summary)

These input/output contracts between stages are established in Layer 0 and must hold through all layers:

| Boundary | Required Input | Required Output |
|----------|---------------|----------------|
| prequalify_notice → acquire_docs | `QualificationResult` per notice: expiryQualified, goodsQualified, accessRoute, qualificationBasis=notice_only | Per-notice provisional pass/review/reject |
| extract_lots → qualify_lot | `NoticeLot` entities, document context | `QualificationResult` per lot with lotId, qualificationBasis=lot_level, addressableValuePence |
| acquire_docs → extract_lots | Document set completeness, document roles, structural extraction output (`document_extractions`), semantic extraction output (`document_semantic_extractions`) | `TenderDocumentSet` with `documentSetHash` + extraction evidence |
| extract_lots → extract_products | `NoticeLot` with title/description, lotNumber | At least one lot per qualified notice |
| extract_products → score_quoteability | `ProductCandidate` with lotId, headNoun, identityConfidence, candidateType, matchArchetype | At least one candidate per qualified lot |
| score_quoteability → price_match | quoteabilityScore, recallRiskScore, SearchProfile with queryWaves | quoteabilityScore ≥ 0.45 for auto-search |
| price_match → gross_gate | `SupplierQuote` with matchConfidence, quoteUsabilityConfidence, priceSignalRole | At least one quote per searched pair |
| gross_gate → landed_cost | CandidatePricingAssessment: selected quote, revenue basis, confidence gates | grossMarginBps, selected quote |
| landed_cost → net_gate | `LandedCostEstimate` anchored to candidate + quote | netMarginLowBps/BaseBps/HighBps, blockers |
| net_gate → manifest | Process-level rollup with coverage metrics | Coverage-aware qualification |

---

## 4. Skills & Skill Authoring Standard

### 4.1 Architecture Role

Skills are the **primary data retrieval mechanism** for the platform. All procurement data and supplier pricing is collected via skill-based agent scraping with browser automation. Instead of maintaining API client packages for each data source, the platform maintains skills — versioned directory assets that are easier to maintain, upgrade, and change than traditional API clients.

**Architecture distinction:**

- **Skills define:** How the agent behaves when scraping a source, what data to extract, how to handle errors, and what execution mode they require.
- **Daemon defines:** When to run which skill, what job state to track, retry logic, scheduling, sync, and which skill version to bind.
- **Orchestration stays in the daemon:** The daemon creates sessions (PTY or contract mode via the adapter), selects skills by explicit binding, monitors execution, validates result contracts, stores results in SQLite, and syncs to Supabase.
- **Adapter mediates:** The Agent Adapter layer handles vendor-specific launch, structured output parsing, health classification, and capability probing — insulating the daemon from CLI version changes.

These boundaries are formalized in **ADR-005**.

**Terminology:** In this document, **skill** means the versioned directory asset bound to a job, while **skill playbook** means the human-readable instructions inside that skill's `SKILL.md`.

### 4.2 Production Skill Directory Standard

A production skill is not "just `SKILL.md`." The maintained unit is the **skill directory**: `SKILL.md` plus supporting artifacts that operationalize the skill as a contract.

**Required files for production-bound skills:**

| File | Purpose | Format |
|------|---------|--------|
| `SKILL.md` | Human-readable playbook — the primary authoring artifact. Contains trigger boundaries, navigation strategy, extraction validation, per-item evidence rules, checkpoint unit, and outcome classes. | Markdown |
| `result.schema.json` | JSON Schema for the final result contract. The daemon validates every skill output against this. | JSON Schema draft-07 |
| `checkpoint.schema.json` | JSON Schema for checkpoint payloads. Checkpoints align to idempotency units (notice batch, candidate-source pair, document, lot), not browser state. | JSON Schema draft-07 |
| `canary.inputs.jsonl` | Small set of known inputs for smoke testing. Used by the CI canary gate and the live smoke canary harness. | JSONL — one input per line |
| `golden.assertions.yaml` | Invariant-based assertions for golden journey tests. Each assertion specifies a condition that must hold on the skill's output for the corresponding canary input. | YAML |

**Optional supporting files:**

| File | Purpose |
|------|---------|
| `scripts/` | Deterministic helper code for validation, normalization, field extraction, query fingerprinting. Rule: **if the same input should always produce the same intermediate check or normalization, that part belongs in deterministic helper code, not in model prose.** |
| `references/` | Source notes, screenshots of expected page layouts, challenge fingerprints |
| `validator-rubric.yaml` | Distilled runtime rubric consumed by the Validation Executor at runtime (Layer 3+) |
| `validator-overlay.md` | Co-located override for domain-specific validation criteria when a shared overlay package under `skills/validation/*` is not sufficient |

**When markdown-only is enough:** Instruction-only skills suffice for short, non-authenticated, non-resumable tasks where correctness is obvious from the final artifact (e.g., manifest narrative synthesis, simple qualification checks).

**When deterministic helpers are needed:** The moment a skill requires any of the following, prose instructions alone are insufficient:

- Auth/session state reuse
- Resumability from checkpoints
- Deterministic field validation or normalization
- Stable query fingerprinting
- Challenge classification
- Field-level evidence extraction
- Canary assertions that must be bit-exact

### 4.3 SKILL.md Structure & Metadata

Every skill declares metadata that the daemon uses for execution planning:

| Field | Purpose | Values |
|-------|---------|--------|
| `interaction_profile` | Execution mode selection | `pty_interactive` (browser automation, interactive flows) or `headless_contract` (structured JSON/JSONL I/O, no browser) |
| `result_schema_version` | Result contract versioning | Semantic version (e.g., `1.0`) |
| `auth_requirements` | What credentials the agent needs | `browser_session`, `api_key`, `none` |
| `browser_required` | Whether the skill needs browser automation | `true` / `false` |
| `max_concurrency` | Maximum concurrent sessions for this skill's source | Integer (e.g., `1`) |
| `expected_duration` | Typical runtime for resource planning | Duration string (e.g., `5m`, `30m`) |
| `model_profile` | Model capability tier for daemon-side model selection (see §4.7) | `browse_stable_low_cost`, `structured_extract_low_cost`, `ambiguous_match_high_reasoning`, `portal_form_high_reliability` |
| `egress_profile` | Expected network egress for geo-sensitive skills | `uk_residential`, `cn_exit`, `default` |
| `validation_policy` | When semantic validation runs for this skill (Layer 3+) | `always`, `sample`, `trigger`, `skip` |
| `validation_sample_rate` | Base sample rate when policy is `sample` (risk factors increase effective rate) | Float (e.g., `0.1` = 10%) |
| `validation_triggers` | Conditions that force validation regardless of policy | `fill_rate_drop > 0.2`, `first_run_after_version_change`, `suspect_output_in_last_5`, `source_health_amber_or_red`, `page_fingerprint_unknown`, `continuation_chain_length > 1`, `high_value_opportunity` |
| `validator_overlay_compatible_versions` | Worker behavior-version range the current validator overlay was authored against | Version range (e.g., `>= 2026.03.12-1, < 2026.04.01`) |

Skills that declare `interaction_profile: headless_contract` must also declare a `result_schema_version` that the daemon uses for result contract validation.

**Validation policy** is declared per-skill in skill metadata, not in daemon config. This preserves plug-and-play composability: adding validation to a skill requires only setting the `validation_policy` field and binding a compatible validator overlay package — either a shared overlay under `skills/validation/*` or a co-located `validator-overlay.md` for source-specific exceptions. No daemon code changes, schema changes, or recompilation required. Skills in draft/candidate/canary/shadow stages always validate regardless of declared policy.

**Validator compatibility:** Worker skills binding a validator overlay must declare `validator_overlay_compatible_versions`. The Validation Policy Engine checks compatibility before allowing enforcement mode for a given slice (worker skill x source x worker version x validator overlay version). If the worker's behavior version is outside the overlay's compatible range, the engine demotes that slice to dry-run or flag-only mode. No compatibility declaration = no enforcement (dry-run only).

**Independent version dimensions:**

| Dimension | What It Tracks | When It Bumps |
|-----------|---------------|---------------|
| `skill_version` | Playbook instructions and behavior | Any change to SKILL.md or scripts/ |
| `result_schema_version` | Final result contract shape | Contract-breaking changes to result.schema.json |
| `checkpoint_schema_version` | Checkpoint payload shape | Changes to checkpoint.schema.json |
| `validator_version` | Output quality validation logic | Changes to golden.assertions.yaml or validation scripts |
| `normalizer_version` | Deterministic data cleanup/normalization | Changes to normalization helpers in scripts/ |

A playbook update does not force a schema version bump unless the contract shape changes. This separation enables identifying whether a regression came from the playbook, the schema, or the validator.

### 4.4 Skill Authoring Guide (8 Questions)

Every production skill answers eight questions explicitly in its `SKILL.md`. Vague trigger boundaries are the single largest source of skill unreliability.

**1. Trigger Boundary**

Every `SKILL.md` includes:

- **Use when:** conditions under which this skill is appropriate
- **Do not use when:** explicit exclusions and non-goals
- **Inputs expected:** schema reference, required context, cursor/resume data
- **Outputs promised:** result contract shape, artifact types
- **Known non-goals:** what this skill explicitly does not attempt

**2. Page Fingerprint**

Before extracting, the skill proves the page is the page it expects:

| Check | Purpose | Example |
|-------|---------|---------|
| Required anchors | Prove correct page loaded | Search box present, results container, at least one result card |
| Forbidden anchors | Detect blocks or wrong page | Login form, CAPTCHA text, "verify you are human," "access denied" |
| Expected locale | Prevent locale drift | `en-GB` |
| Expected currency | Prevent currency contamination | `GBP` |
| Expected persona/auth state | Verify correct account | Logged in as expected, or anonymous where expected |
| Expected page kind | Distinguish result types | Search results vs category page vs detail page |

This stops "clean run, wrong page" failures — the most insidious failure mode because the output looks schema-valid.

**3. Navigation Strategy**

Tiered locator strategy (Playwright best-practice model — prefer user-facing locators with auto-waiting/actionability checks; raw CSS/XPath only as last fallback):

1. **Semantic locator** (role, label, accessible name) — preferred
2. **Anchored relative locator** (relative to a stable landmark)
3. **Text/content fallback** (visible text matching)
4. **CSS/XPath escape hatch** (fragile, last resort)

Every navigation step specifies: **Goal** (what it accomplishes), **Proof** (verification the step succeeded), **Fallback** (alternative if primary locator fails), **Abort condition** (when to stop trying and classify the failure), **Checkpoint rule** (whether completing this step is a valid checkpoint boundary).

**4. Overlay and Interrupt Handling**

- **Predictable overlays** (cookie consent, login prompts on known pages): explicit step in the normal flow — handle as part of expected page state, not as exception
- **Unpredictable overlays** (surveys, promotional interstitials, A/B test popups): handled via known overlay handler map. Each handler specifies detection pattern and dismissal action
- Every handler invocation increments a metric
- Repeated handler invocations in one run (>3 same type) raise a suspicion signal

**5. Extraction Validation** (three code-based layers)

| Layer | What It Checks | Examples |
|-------|---------------|---------|
| **Syntactic** | Required fields present, types valid, URLs parse to expected domains, schema matches `result.schema.json` | Missing fields, wrong types, invalid URLs |
| **Semantic** | Business invariants hold | Deadline after publication, currency consistent with locale, source IDs unique in batch, price/unit plausible, lot values sum correctly, quantity x unit price within range |
| **Behavioral/drift** | Statistical envelope checks against historical norms | Null-rate spike (extraction breakage), duplicate rate spike (pagination loop), result count collapse, page fingerprint hash divergence, locator fallback depth increase (DOM structure change) |

These three code-based layers are enforced by the **Output Quality Gate** (OQG). Contextual errors invisible to code (wrong page with valid-looking data, Scottish notices from England-only source, consumer vs industrial pricing) are caught by the semantic validator (Layer 3+). OQG signals feed INTO the validator as inputs (via daemon evidence appendix), not compete with it.

**6. Per-Item Evidence**

Every extracted business item (notice, quote, document reference, product candidate) carries evidence:

| Field | Purpose |
|-------|---------|
| `sourceUrl` | Exact URL the data was extracted from |
| `rawTextFragments` | Key text snippets used for extraction (not full page HTML) |
| `captureTimestampUtc` | When the extraction occurred |
| `pageFingerprint` | Hash of the page fingerprint at extraction time |
| `normalizationVersion` | Version of the normalization logic applied |
| `evidenceSnippet` | Human-readable excerpt for debugging |
| `artifactPointer` | Reference to forensic session artifact if deeper inspection needed |
| `itemDigest` | Content hash of the normalized record for deduplication and change detection |

**7. Checkpoint Unit**

Checkpoints align to the **idempotency unit** of the business operation, not browser state:

| Skill Type | Durable Unit | Checkpoint Cadence | Resume Token |
|------------|-------------|-------------------|--------------|
| Procurement ingestion | Notice batch / source page | Every page or every 25 notices | `last_source_notice_id`, `cursor_value` |
| Supplier enrichment | Candidate-source pair | After shortlist complete, then after normalized quotes | `candidate_id`, `query_fingerprint`, `after_listing_id` |
| Document acquisition | Document | Per downloaded document | `document_checksum`, `document_url` |
| Lot/product extraction | Lot or document | Per lot/document processed | `lot_id`, `document_id` |
| Bid portal automation | Verified save boundary | After each server-verified save/upload | `last_verified_portal_section`, `form_checkpoint_id` |

**Bad checkpoint units** (browser state, non-resumable): DOM element index, scroll pixel count, current tab number, "I clicked something."

Resume tokens are business-level identifiers. On resume, the daemon injects the resume token into the kickoff prompt so the agent can skip already-committed work.

**Concrete checkpoint format:**

```json
{
  "schema_version": "1.0",
  "job_id": "job-123",
  "skill_id": "supplier-price-enrichment/alibaba",
  "skill_version": "2026.03.12-4",
  "seq": 12,
  "kind": "candidate_source_batch",
  "stage": "shortlist_complete",
  "resume_from": {
    "candidate_id": "cand-7",
    "query_fingerprint": "qf-abc",
    "after_listing_id": "listing-443"
  },
  "progress": { "units_total": 5, "units_done": 3 },
  "payload_ref": "artifact://session/.../cp-12.json",
  "payload_digest": "sha256:..."
}
```

Daemon acceptance rules: schema valid, monotonic `seq`, skill/job identity match, digest valid, resume token present when required, idempotency keys valid, payload stored before checkpoint accepted.

**Resume logic examples:**

- **45/50 notices ingested, then website errors:** Keep the 45 if each batch was durably checkpointed. Resume from last committed cursor with small overlap/backtrack window. Do not restart from scratch unless cursor semantics are broken.
- **3/5 product prices found before failure:** Keep the 3 if durable unit is candidate-source pair. Retry only the 2 missing units. Do not discard committed quotes because a later unit failed.
- **Bid portal failed after upload but before submit:** Resume only if last safe boundary was server-verified. If any chance the submit boundary was crossed, enter **verify-only** mode first.

**8. Outcome Classes**

Skills return one of six standard outcome classes (not binary success/failure):

| Status | Meaning | Daemon Action |
|--------|---------|--------------|
| `completed` | Full scope finished, all validation passed | Accept results, advance stage |
| `degraded_completed` | Partial but usable results with quality warnings | Accept with degradation flag, surface warnings |
| `continuation_required` | Clean exit at safe boundary with remaining work | Accept committed results, enqueue continuation segment |
| `blocked_human_action` | Human intervention required (CAPTCHA, MFA, manual approval) | Emit appropriate error code, alert operator |
| `suspect_output` | Schema-valid output that failed semantic or drift validation | Quarantine results, do not promote to pipeline, surface for review |
| `failed` | Unrecoverable error with specific failure code | Create error manifest, apply retry/cooldown policy |

The `failed` status carries structured failure metadata:

```json
{
  "status": "failed",
  "failure_code": "PAGE_SHAPE_DRIFT | AUTH_STATE_STALE | SOURCE_CAPTCHA_BLOCKED | SOURCE_SOFT_BLOCKED | LOCALE_OR_CURRENCY_DRIFT | AGENT_OUTPUT_CONTRACT_INVALID | ...",
  "retry_advice": "step_retry | session_retry | cooldown | resume | manual | none",
  "cooldown_seconds": 0,
  "resume_from": {},
  "warnings": []
}
```

The critical distinction: `continuation_required` means a long-running skill hit its time budget after 45 notices — not a failure. It emits a valid final contract with continuation data, and the Job Runner automatically enqueues the next segment using the emitted resume token. Crashes with only prior checkpoints remain `AGENT_PARTIAL_FAILURE`. Clean exits with continuation contract are a different, healthy class.

### 4.5 Skill Categories

The platform requires skills across six categories:

**Category 1 — Procurement Ingestion Skills** (agent scrapes procurement sources via browser):

| Skill | Target | Layer | `interaction_profile` | `model_profile` |
|-------|--------|-------|-----------------------|-----------------|
| `procurement-ingestion/fts` | Find a Tender | L1 | `pty_interactive` | `browse_stable_low_cost` |
| `procurement-ingestion/contracts-finder` | Contracts Finder | L2 | `pty_interactive` | `browse_stable_low_cost` |
| `procurement-ingestion/pcs` | Public Contracts Scotland | L5 | `pty_interactive` | `browse_stable_low_cost` |

**Category 2 — Supplier Price Enrichment Skills** (agent scrapes marketplaces via browser):

| Skill | Target | Layer | `interaction_profile` | `model_profile` |
|-------|--------|-------|-----------------------|-----------------|
| `supplier-price-enrichment/ebay` | eBay UK | L0 | `pty_interactive` | `browse_stable_low_cost` |
| `supplier-price-enrichment/amazon` | Amazon UK | L5 | `pty_interactive` | `browse_stable_low_cost` |
| `supplier-price-enrichment/alibaba` | Alibaba.com | L5 | `pty_interactive` | `browse_stable_low_cost` |
| `supplier-price-enrichment/1688` | 1688.com | L5 | `pty_interactive` | `browse_stable_low_cost` |

**Category 3 — Intelligence Pipeline Skills** (agent performs extraction and classification):

| Skill | Purpose | Layer | `interaction_profile` | `model_profile` |
|-------|---------|-------|-----------------------|-----------------|
| `document-acquisition` | Download tender documents from notice source portals | L0 | `pty_interactive` | `browse_stable_low_cost` |
| `lot-extraction` | Extract lot structure from unstructured documents | L0 | `headless_contract` | `structured_extract_low_cost` |
| `product-extraction` | Extract product candidates with candidateType and structured slots | L0 | `headless_contract` | `structured_extract_low_cost` |
| `preliminary-qualification` | Classify goods/services/works/mixed for ambiguous cases | L0 | `headless_contract` | `ambiguous_match_high_reasoning` |

**Category 4 — Workflow Skills** (agent performs workflow actions):

| Skill | Purpose | Layer | `interaction_profile` | `model_profile` |
|-------|---------|-------|-----------------------|-----------------|
| `manifest-generation` | Fill narrative sections for opportunity manifests | L1 | `headless_contract` | `structured_extract_low_cost` |
| `bid-portal-*` | Portal-specific bid preparation and submission | L6 | `pty_interactive` | `portal_form_high_reliability` |

**Category 5 — Operational Skills** (maintain source health and authentication):

| Skill | Purpose | Layer |
|-------|---------|-------|
| `auth-refresh/alibaba` | Establish or refresh Alibaba authenticated session state, publish auth-state handle | L3 |
| `auth-refresh/1688` | Establish or refresh 1688 authenticated session state, publish auth-state handle | L3 |
| `source-health/<source>` | Pre-flight canary: verify reachability, geo-correctness, auth state, page fingerprint, challenge rate | L3 |

Auth-refresh skills do one thing: establish or refresh authenticated state and publish a new auth-state handle. They isolate login complexity from extraction complexity. Source-health canary skills run before expensive scheduled jobs to detect whether a source is reachable, geo-correct, authenticated (if needed), and not actively challenging — if health is red, the daemon skips the big job.

**Category 6 — Validator Skills** (agent reviews run manifests for semantic quality):

| Skill | Purpose | Layer |
|-------|---------|-------|
| `validation/base` | Generic run manifest quality checks: completeness, consistency, cross-check agent-claimed fields against daemon-observed evidence appendix, assess `basis_sufficiency` | L3 |
| `validation/procurement-ingestion` | Procurement-specific overlay: verify OCID structure, date ranges, notice types, detect cross-source contamination | L4 |
| `validation/supplier-enrichment` | Supplier-specific overlay: verify pricing plausibility, currency consistency, locale correctness, detect wrong-market data | L4 |
| `validation/document-acquisition` | Document acquisition overlay: verify document types, non-empty content, correct format | L4 |
| `validation/product-extraction` | Product extraction overlay: verify candidate plausibility given lot descriptions, detect fabricated structured data | L4 |

Validator skills read the worker's hybrid-authored run manifest (agent narrative + daemon evidence appendix with machine-observed facts) and render a verdict (`approved`, `rejected`, `flagged`) with verdict metadata including `basis_sufficiency` and `enforcement_eligible`. They use `headless_contract` mode with `structured_extract_low_cost` model profile. Base validator criteria apply to all runs; domain-specific criteria come from the bound validator overlay.

**Boundary note:** There is intentionally no `validation/landed-cost` overlay because the landed-cost engine is deterministic and API-based. If it becomes skill-executed in the future, it adopts the same overlay pattern.

**OQG vs semantic validation:** The Output Quality Gate performs three code-based validation layers (syntactic, semantic, behavioral/drift). Validator skills perform a fourth layer: agent-based semantic validation — catching contextual errors invisible to code. OQG signals feed INTO the validator as inputs (via daemon evidence appendix and anomaly-stratified result contract sample), not compete with it.

### 4.6 Layer 0 Skills (5 Initial Skills)

These 5 skills are authored in Layer 0 with complete production directory structure. Each contains all 5 required files: `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`.

#### 4.6.1 `skills/document-acquisition/`

| Field | Value |
|-------|-------|
| **Purpose** | Navigate to notice source portal, identify and download tender specification PDFs, pricing schedules, BoQ spreadsheets, and lot documents |
| **`interaction_profile`** | `pty_interactive` |
| **`model_profile`** | `browse_stable_low_cost` |
| **`auth_requirements`** | `none` (FTS/CF are public portals) |
| **Inputs** | Notice URL, expected document types |
| **Outputs** | Downloaded documents stored as artifacts, `TenderDocumentSet` with document roles and completeness |
| **Checkpoint unit** | Per downloaded document (`document_checksum`, `document_url`) |
| **L0 scope** | **STUBBED** — gold-set documents are pre-curated. No live source interaction until Layer 1. |

#### 4.6.2 `skills/lot-extraction/`

| Field | Value |
|-------|-------|
| **Purpose** | Parse unstructured lot descriptions from tender documents when OCDS lot data is absent. Extract lot number, title, description, estimated value, CPV codes. |
| **`interaction_profile`** | `headless_contract` |
| **`model_profile`** | `structured_extract_low_cost` |
| **`auth_requirements`** | `none` |
| **Inputs** | Tender document text/parsed content, notice metadata |
| **Outputs** | Structured `NoticeLot` entities with `lot_cpv_codes` junction records |
| **Checkpoint unit** | Per lot/document processed (`lot_id`, `document_id`) |

#### 4.6.3 `skills/product-extraction/`

| Field | Value |
|-------|-------|
| **Purpose** | Extract product candidates from lot text/documents. Identify items, classify `candidateType`, extract structured slots (headNoun, unit, quantity, specification), decompose bundles, assign `matchArchetype`. |
| **`interaction_profile`** | `headless_contract` |
| **`model_profile`** | `structured_extract_low_cost` |
| **`auth_requirements`** | `none` |
| **Inputs** | Lot descriptions, document context |
| **Outputs** | `ProductCandidate` entities per lot with `lotId` (required NOT NULL), `headNoun`, `identityConfidenceBps`, `candidateType`, `matchArchetype`, `candidateValueShareBps` |
| **Checkpoint unit** | Per lot/document processed (`lot_id`, `document_id`) |

#### 4.6.4 `skills/preliminary-qualification/`

| Field | Value |
|-------|-------|
| **Purpose** | Classify goods/services/works/mixed for notices where deterministic CPV/expiry/category rules leave ambiguity. Analyze notice text only when needed. |
| **`interaction_profile`** | `headless_contract` |
| **`model_profile`** | `ambiguous_match_high_reasoning` |
| **`auth_requirements`** | `none` |
| **Inputs** | Notice text, CPV codes, prior classification signals |
| **Outputs** | `QualificationResult` with goods/services/works/mixed classification, reasoning, confidence |
| **Checkpoint unit** | Per notice |

#### 4.6.5 `skills/supplier-price-enrichment/ebay/`

| Field | Value |
|-------|-------|
| **Purpose** | Search eBay UK for matching supplier listings. Extract pricing, seller info, shipping, condition, specification fields. |
| **`interaction_profile`** | `pty_interactive` |
| **`model_profile`** | `browse_stable_low_cost` |
| **`auth_requirements`** | `none` (eBay browsing is public) |
| **`egress_profile`** | `uk_residential` |
| **Inputs** | `SearchProfile` with query waves, `ProductCandidate` identity fields |
| **Outputs** | `SupplierQuote` entities per candidate-source pair with `matchConfidenceBps`, `quoteUsabilityConfidenceBps`, `priceSignalRole` |
| **Checkpoint unit** | Candidate-source pair (`candidate_id`, `query_fingerprint`, `after_listing_id`) |
| **Budget declarations** | `maxPages: 5`, `maxScrollCycles: 3`, `maxDetailPageVisits: 10`, `maxQueryVariants: 3`, `targetConfidentHits: 3`, `timeBudgetMinutes: 10` |

### 4.7 Model Profile Abstraction

Skills declare a `model_profile` instead of a vendor-specific model string. The daemon maps the profile to the current best-fit model. This allows model upgrades without touching skill files, enables A/B testing of model assignments, and concentrates cost optimization in one place.

| Profile | Use Case | Optimization |
|---------|----------|-------------|
| `browse_stable_low_cost` | Procurement ingestion, document acquisition, simple navigation | Cheapest model that can follow browser playbooks reliably |
| `structured_extract_low_cost` | Lot extraction, product extraction, field normalization, validator skills | Cheapest model with strong structured output |
| `ambiguous_match_high_reasoning` | Borderline product matching, qualification of mixed contracts | Premium model for reasoning-heavy adjudication |
| `portal_form_high_reliability` | Bid portal form filling, submission preparation | Premium model where errors have operational consequences |

### 4.8 Skill Composition Rules

Skills can compose conceptually, but a production skill must not recursively invoke other skills inside one opaque session. Uncontrolled sub-skill recursion breaks cost attribution, resume logic, and debugging.

Two composition paths:

1. **Intra-skill composition** — Shared references/scripts within the skill directory: deterministic helper scripts in `scripts/` and reference documents in `references/` loaded into the agent's context.
2. **Inter-skill composition** — Daemon-orchestrated sub-jobs: when a workflow requires multiple skill invocations (e.g., auth-refresh before enrichment, document acquisition before product extraction), the daemon orchestrates them as separate jobs with independent result contracts and observability.

The daemon owns composition sequencing. Skills are leaf-level execution units.

### 4.9 Browser Automation Reliability Patterns

#### Anti-Bot, CAPTCHA, and Rate Limit Classification

Skills classify blocks and apply the appropriate response — they do not "try harder":

| Condition | Immediate Action | Auto Retry |
|-----------|-----------------|------------|
| Navigation timeout before page fingerprint | Retry same step | 2 times: 5s then 20s backoff |
| Repeated timeout with no challenge markers | Restart clean session | 1 restart, then fail |
| Soft block / challenge page / empty shell | Capture artifacts, report `SOURCE_SOFT_BLOCKED` | No retry, set source cooldown |
| Hard CAPTCHA / MFA | Emit `blocked_human_action` with artifacts | None — requires human |
| Auth expired | One rehydrate attempt from stored auth state | 1 attempt, then `AUTH_STATE_STALE` |
| Geo / locale mismatch | Fail fast with `LOCALE_OR_CURRENCY_DRIFT` | None — requires different egress |
| Empty results with valid page fingerprint | Record as legitimate empty result set | None — valid business outcome |

The daemon backs this with a **source circuit breaker**: three challenge-class events in a rolling 30-minute window open the circuit for that source, set cooldown in `source_runtime_state`, and suppress normal extraction jobs. During cooldown, only `auth-refresh/*` and `source-health/*` can run until a canary clears the source.

#### Authentication Persistence

Per-session browser isolation is the correct baseline. Re-authenticating from scratch every session is not.

1. Maintain **source-scoped auth state** per persona/account, managed by dedicated `auth-refresh/<source>` skills
2. Inject stored auth state (cookies, local storage, IndexedDB snapshot) into fresh isolated browser context at session start
3. Verify logged-in identity before extraction — if persona wrong or auth stale, fail fast with `AUTH_STATE_STALE`
4. On auth expiry mid-run, attempt one rehydrate. If rehydrate fails, emit `blocked_human_action`
5. Auth-refresh skills publish new auth-state handles that enrichment skills consume — isolating login complexity from extraction complexity

For multi-account sources (e.g., multiple Alibaba accounts), concurrent runs use unique accounts to avoid session interference.

#### Dynamic Content and Infinite Scroll

Never use "wait N seconds" as primary readiness strategy. Use a readiness algorithm:

1. Page fingerprint passes (required anchors present, forbidden absent)
2. First result card visible and content-populated (not skeleton loader)
3. Capture visible item IDs/titles
4. Wait/scroll once
5. Compare visible item IDs with previous capture
6. Stop when no new IDs across two consecutive scroll cycles, or max scroll budget hit

Skills declare: `maxScrollCycles` (hard cap), `maxDetailPageVisits` (drill-down cap), `stopCondition` ("no new IDs twice" or equivalent), `skeletonTimeout` (skeleton loaders with no content change -> `PAGE_SHAPE_DRIFT`).

#### Geo-Restriction and Locale Drift

Every browser skill asserts at run start: expected country, expected currency, expected language, expected signed-in persona (if authenticated), expected seller/market variant. If any assertion fails, fail fast with `LOCALE_OR_CURRENCY_DRIFT`.

The `egress_profile` in source config controls egress routing. Skills **detect and report** geo mismatch; the daemon/source config **decides** egress routing.

#### Session Contamination Prevention

- Isolated browser context per session (no shared live profiles across concurrent sessions)
- Isolated downloads directory per session
- Persona verification at run start (not assumed from profile contents)
- Cache keys include persona + locale + currency + egress profile (not just source + query)
- Stable fingerprints preferred over randomized — for production automation, boring is safer than stealthy

### 4.10 Cost Efficiency Patterns

**Budget declarations** — every browser skill declares:

| Budget | Purpose | Example |
|--------|---------|---------|
| `maxPages` | Hard cap on paginated navigation | `10` |
| `maxScrollCycles` | Hard cap on infinite scroll iterations | `5` |
| `maxDetailPageVisits` | Cap on detail-page drill-downs | `15` |
| `maxQueryVariants` | Cap on search query attempts | `3` (exact -> broad -> stop) |
| `targetConfidentHits` | Early-stop threshold on strong matches | `3` |
| `timeBudgetMinutes` | Session time budget (triggers `continuation_required`) | `15` |
| `degradeThreshold` | When to accept degraded results and stop | Fill-rate < 20% after half budget |

**Concrete cost patterns:**

- Stop supplier search after 3 strong comparable hits with high `matchConfidence`, not after exhausting 30 weak candidates
- Only visit detail pages for shortlisted result cards — list-page data is often sufficient for screening-grade quotes
- Try exact query wave first, then one broader fallback, then stop
- If shipping information is behind login/interaction, record screening-grade quote and move on
- Segment long jobs with `continuation_required` rather than letting sessions bloat past time budget
- For procurement ingestion, batch by source: one session per source per run, not one session per notice

**Cost observability:** The meaningful metric is **cost per committed useful record**: cost per committed notice, per accepted quote, per qualified opportunity. Tracked per skill, per source, and trended over time to detect efficiency regressions after skill updates.

### 4.11 Skill Failure Modes

Beyond the standard error taxonomy, skills detect and report these production failure modes:

| Failure Mode | Detection | Why It's Dangerous |
|-------------|-----------|-------------------|
| Schema-valid garbage | Semantic + drift validation | Clean exit, valid JSON, but wrong page or bad extraction |
| Soft blocks disguised as empty results | Page fingerprint + challenge marker check | HTTP 200, no useful data, not technically an error |
| Auth half-expired state | Persona verification + privileged data presence check | Session loads but privileged data missing or persona changed |
| Locale/currency/persona drift | Pre-extraction assertion of expected market state | Same site, different market mode, silently poisoning comparisons |
| Pagination loops / duplicate scroll churn | Business-item dedup across scroll cycles | Skill keeps "working" but not discovering new items |
| Wrong artifact acquisition | Post-download content-type and structure validation | File is wrong format, corrupted, login HTML, or empty PDF |
| Unsafe resume after external side effect | Side-effect boundary tracking in portal skills | Especially dangerous in bid portal automation |
| Cache poisoning across versions/personas | Cache key includes all relevant dimensions | Old cached results reused under different normalization assumptions |

### 4.12 Skill Lifecycle & Promotion Flow

**Lifecycle stages:**

1. **Creation** — Skills authored as directories with `SKILL.md` plus supporting artifacts. Markdown-first authoring preserved.
2. **Promotion** — New/updated skills follow: draft -> candidate -> canary -> shadow -> stable. Rollback rebinds to prior stable version. Validator approval is a promotion gate alongside quality metrics (Layer 3+).
3. **Execution** — Daemon triggers agent session (PTY or contract via adapter). Scheduled jobs bind explicit `stable_version` + optional `canary_version`/`shadow_version`. Agent follows playbook, writes run manifest narrative. Daemon appends evidence appendix.
4. **Manual mode** — Draft skills explored without scheduler. Kickoff Prompt Renderer emits exact prompt for copy-paste into fresh agent session. Does not commit pipeline state.
5. **Dry-run mode** — Full worker + validator path against scratch workspace at `artifacts/scratch/<run_id>/`. Records manifests and verdicts for review but does not advance authoritative rows, outbox, or stage heads. Default calibration path before enforcement.
6. **Validation** — After OQG, Validation Policy Engine applies risk-stratified sampling. Validator sub-job reads hybrid manifest, result contract sample, rubric, OQG signals, drift deltas. Renders verdict (Layer 3+).
7. **Maintenance** — Source website changes only require skill update, not API client changes. Source-health canaries detect drift before expensive extraction runs.
8. **Versioning** — Skills version-tracked in repo. Daemon records which skill version was used for each execution. Behavior, schema, checkpoint, validator, and normalizer versions tracked independently.
9. **Upgrade** — Editing SKILL.md + supporting files. No recompilation, no package upgrades. Schema migrations use dual-accept windows: daemon accepts both old and new schema versions during transition, shadow comparison validates quality parity.

**Promotion flow:**

| Stage | Gate | Scope |
|-------|------|-------|
| **Draft** | Dev-only | Manual runs only, not scheduler-eligible |
| **Candidate** | CI lint + smoke canaries pass | CI-validated, not production-eligible |
| **Canary** | Smoke canaries pass against live source | Small percentage or single source shard |
| **Shadow** | Quality comparison against stable on real workload | Full production inputs, results not authoritative |
| **Stable** | Shadow metrics meet promotion thresholds | Scheduler default for all production runs |
| **Rollback** | Any time | Rebind source to prior stable version immediately |

Rollback is "change source binding," not "edit markdown again" — instantaneous and safe.

**Version binding at enqueue time:**

| Binding | Purpose |
|---------|---------|
| `stable_version` | Scheduler default — runs authoritative extraction |
| `canary_version` | Optional — small percentage or single shard for validation |
| `shadow_version` | Optional — parallel with stable for quality comparison, no authoritative writes |

"Latest on disk" never determines production behavior. Scheduled jobs bind explicit versions.

**Schema migration:** For result schema transitions (v1.0 -> v1.1): daemon accepts both versions during transition window, shadow comparison validates quality parity, dual-read/dual-validate if downstream consumers need adaptation time, promote when all consumers accept new version, retire old version after promotion window closes.

**Cache key dimensions** (for supplier result caching):

| Key Component | Why It Matters |
|--------------|---------------|
| Source | Different sources return different data |
| Query fingerprint | Different queries return different results |
| Candidate normalization version | Changed normalization invalidates cached matches |
| Skill major version | Major skill changes may extract different fields |
| Validator version | Changed validation may reclassify cached results |
| Persona | Different accounts may see different pricing/availability |
| Locale / currency | Market variant affects all pricing data |
| Egress profile | Different exit nodes may trigger different geo behavior |

For procurement normalization changes (not source changes), prefer **reprocessing from stored raw artifacts** over re-scraping. Raw artifact retention enables replay, not just forensics.

### 4.13 Run Manifest Preparation (Layer 0)

In Layer 0, skills run from the CLI without a daemon. There is no formal run manifest system yet (that arrives in Layer 1). However, each skill's `SKILL.md` includes instructions for the agent to write a structured narrative of what it did as part of its output:

1. Every skill output includes a narrative section describing: what pages were visited, what data was extracted, what decisions were made, and any warnings or anomalies encountered
2. The narrative format follows a consistent structure that the Layer 1 daemon can later consume as the agent-authored portion of the hybrid run manifest
3. Skills practice the "what I did, what I found, what I decided" discipline from their first execution

In Layer 1, the daemon formalizes this into the hybrid-authored run manifest system:
- The agent writes narrative sections ("what I did")
- The daemon appends a machine-authored evidence appendix with independently observed facts (machine-timed events, resource usage, checkpoint sequence, OQG results)
- If the agent fails to narrate, the daemon produces a fallback machine-authored manifest
- The combined manifest is stored as an immutable artifact referenced by `run_manifest_path` and `run_manifest_hash` in `agent_session_logs`

### 4.14 Licensing & Compliance Notes

| Source | License / Terms | Status | Notes |
|--------|----------------|--------|-------|
| **Find a Tender** data | Open Government Licence | Compatible | Official public data, accessed via agent skill scraping |
| **Contracts Finder** data | Government open publication model | Compatible | Accessed via agent skill scraping |
| **PCS** | Official public API | Likely compatible | Accessed via agent skill scraping |
| **HMRC Trade Tariff API** | Official public government API | Compatible | Direct API access for cost estimation |
| **HMRC FPO API** | Official public government API | Compatible | Direct API access for HS code classification |
| **HMRC Exchange Rate Service** | Official public government data | Compatible | Free, published monthly |
| **eBay** (browser scraping) | Website terms of use | **Review required** | Agent skill scraping — review ToS for automated access before implementation. Low-volume, source-specific. |
| **Amazon** (browser scraping) | Website terms of use | **Review required** | Agent skill scraping — review ToS before implementation (Layer 5) |
| **Alibaba/1688** (browser scraping) | Website terms of use | **Review required** | Agent skill scraping — review ToS before implementation (Layer 5) |
| **Xe API** | Commercial | Compatible | Paid integration for commercial FX (Layer 5) |
| **Freightos** | Commercial | Compatible | Sales/commercial access required (Layer 5) |
| **SeaRates** | Commercial | Compatible | Commercial API access required (Layer 5) |
| **Easyship** | Commercial | Compatible | Parcel-mode freight and duty estimation (Layer 5) |
| **Shippo** | Commercial | Compatible | Parcel-mode carrier rates (Layer 5) |
| **Taric Support, HS Ping** | Commercial | **Review required** | Verify API terms for automated pipeline use |
| **Dutify, Zonos, Avalara** | Commercial | **Review required** | Optional landed-cost comparison providers — validation use only |

**General rule:** APIs marked "Review required" require verification of commercial terms for automated pipeline use. The main compliance consideration for skill-based scraping is **website terms of use** for supplier marketplaces. Agent skill scraping should be low-volume and source-specific, with ToS reviewed for each marketplace before implementation.

---

## 5. TypeScript Packages & Evaluation Harness

### 5.1 packages/contracts-schema

4 files providing the canonical TypeScript domain types shared across all layers.

> **Ownership rule:** §2 (Schema) owns the full TypeScript interface definitions inline with DDL. This section owns the *package structure* — which interfaces go in which file. Reference §2 for interface bodies.

**contracts.ts** — All pipeline entity interfaces:
- `ProcurementProcess`, `ContractNotice`, `NoticeLot`, `TenderDocumentSet`
- `QualificationResult`
- `ProductCandidate` (with `CriticalAttribute`, `NumericSpec`, `StandardSlot` supporting types)
- `ProcessRelation`, `SearchProfile` (with `QueryWave`), `SearchAttempt`, `ListingEvidence`
- `SupplierQuote`
- `CandidatePricingAssessment`
- `LandedCostEstimate` (with `CandidateCommodityCode`, `ConfidenceBreakdown`)
- `PipelineAssessment`
- `OperatorDecisionHead` (remote-owned)
- `BidWorkflowState`
- `ProcessStageStatus`, `ViewerSyncStatus`

**jobs.ts** — Job payload types:
- `WorkflowJob` interface with `kind` discriminator
- 18 + 2 job kinds mapping (from Appendix C):

| Job Kind | Pipeline Stage | Scope Grain |
|----------|---------------|-------------|
| `sync_source` | 1. Ingest | per source per run |
| `normalize_notice` | 2. Normalize | per notice |
| `resolve_relations` | 3. Relate | per process cluster |
| `prequalify_notice` | 4a. Prequalify notice | per notice |
| `qualify_lot` | 4b. Qualify lot | per lot |
| `acquire_documents` | 5. Acquire docs | per notice |
| `extract_document_structure` | 5b. Structural extraction | per document (runner-native, Docling subprocess) |
| `extract_document_semantics` | 5c. Semantic extraction | per document × overlay (agent adapter, headless_contract) |
| `extract_lots` | 6. Extract lots | per notice + doc set |
| `extract_products` | 7. Extract products | per lot |
| `score_quoteability` | 8. Score quoteability | per candidate |
| `enrich_supplier_prices` | 9. Price-signal match | per candidate-source pair |
| `compute_gross_gate` | 10. Gross margin gate | per candidate |
| `estimate_landed_cost` | 11. Landed cost | per candidate-quote scenario |
| `compute_net_gate` | 12. Net margin gate | per candidate + process rollup |
| `generate_manifest` | 13. Manifest | per process |
| `record_operator_decision` | 14. Operator decision | per process |
| `run_bid_workflow` | 15. Bid workflow | per process (with scope) |
| `escalate_rfq` | (re-entry) | per candidate |
| `validation` | (cross-cutting) | per worker job |

- `AgentSessionLog` interface
- `ValidationVerdict`, `ValidationVerdictSummary` interfaces
- `ValidationIssue` interface

**confidence.ts** — Confidence scoring types:
- `ConfidenceBreakdown` (6 components: `classificationStability`, `quoteRealism`, `commodityCodeConfidence`, `freightModeStability`, `revenueReliability`, `fxFreshness`)
- Confidence bands: >= 0.80 high, 0.60–0.79 usable, < 0.60 manual review
- `CandidateCommodityCode` with source enum
- Dual confidence model types (`matchConfidence` + `quoteUsabilityConfidence`)
- Lexicographic gating types

**zod.ts** — Runtime validation schemas:
- Zod schemas for all entity interfaces in contracts.ts
- Used for skill result contract validation and data boundary checks
- Must stay in sync with TypeScript interfaces

### 5.2 packages/intelligence

The intelligence pipeline logic. All screening-grade computation, deterministic gates, and evaluation harness.

**Package boundary rule (constitutional):** `packages/intelligence` must NOT import `packages/db`, daemon modules, desktop bridge, or Supabase clients. It is a pure TypeScript library consumed by the daemon (via Bun), the evaluation harness, and tests.

#### qualification/ (4 files)

| File | Responsibility |
|------|---------------|
| `expiry.ts` | 30-day expiry filter with DPS/Dynamic Market exception |
| `access-route.ts` | Access route classification and bidability rules (7 access routes) |
| `goods.ts` | Multi-dimensional goods classification (`legalCategory`, `goodsDensityScore`, `goodsSeparability`) + CPV deterministic lookup |
| `quoteability.ts` | Deterministic quoteability scoring formula, `recallRiskScore` computation, `SearchProfile` generation for all candidates |

#### extraction/ (1 file)

| File | Responsibility |
|------|---------------|
| `products.ts` | Product candidate extraction: `candidateType` classification, `matchArchetype` assignment, structured slot extraction, slot criticality annotation, standard normalization, bundle decomposition |

#### search/ (1 file)

| File | Responsibility |
|------|---------------|
| `build-search-profile.ts` | Multilingual query-wave construction from structured candidate slots. Generates `QueryWave` sequences per candidate x source. Consumes bilingual lexicon. |

#### customs/ (L0: 1 file; L5: 6 additional files)

| File | Layer | Responsibility |
|------|-------|---------------|
| `hmrc-tariff.ts` | L0 | HMRC Trade Tariff API integration — HS6 duty lookup, MFN rate retrieval (screening-grade) |
| `hmrc-fpo.ts` | L5 | HMRC FPO API for HS code candidate generation |
| `classification-preprocessing.ts` | L5 | Description-quality gate, attribute extraction, multi-source recall dispatch |
| `ten-digit-resolver.ts` | L5 | 10-digit UK commodity code resolution from 6/8-digit candidates |
| `measure-scanner.ts` | L5 | Trade remedy detection, preferential eligibility, quota checks, measure interactions |
| `taric-support.ts` | L5 | Taric Support API for code validation |
| `hs-ping.ts` | L5 | HS Ping comparison lookup |

#### fx/ (L0: 0 files; L5: 4 files)

| File | Layer | Responsibility |
|------|-------|---------------|
| `hmrc-exchange-rates.ts` | L5 | HMRC official exchange rates (customs valuation, monthly rolling) |
| `xe.ts` | L5 | Xe commercial FX (real-time spot rates) |
| `currencybeacon.ts` | L5 | CurrencyBeacon backup/comparison FX |
| `oanda.ts` | L5 | OANDA historical FX |

L0 uses a single hardcoded/seeded FX rate for screening-grade. Full dual-rate FX in Layer 5.

#### logistics/ (L0: 0 files; L5: 5 files)

| File | Layer | Responsibility |
|------|-------|---------------|
| `mode-selection.ts` | L5 | Freight mode decision tree with hard locks (hazmat, oversized, fragile, reefer, batteries) |
| `freightos.ts` | L5 | Freightos freight API |
| `searates.ts` | L5 | SeaRates freight API |
| `easyship.ts` | L5 | Easyship parcel-mode estimation |
| `shippo.ts` | L5 | Shippo parcel-mode carrier rates |

L0 uses flat freight assumption. Full freight engine in Layer 5.

#### margins/ (5 files)

| File | Responsibility |
|------|---------------|
| `revenue-normalizer.ts` | Revenue basis classification (`firm_contract` / `estimated_contract` / `framework_ceiling` / `dynamic_market` / `concession` / `unknown`) and `expected_addressable_revenue_ex_vat` computation |
| `quote-normalizer.ts` | Incoterm cost boundary normalization (EXW/FOB/CIF/DDP to comparable basis) |
| `gross-gate.ts` | Gross margin gate: identity floor, usability floor, realism gate, margin computation. Produces `CandidatePricingAssessment`. |
| `margin-scenarios.ts` | Low/base/high net margin scenario calculation with critical blocker checks |
| `coverage-rollup.ts` | Process-level coverage: `pricedCoverageBps`, `landedCostCoverageBps`, `netQualifiedCoverageBps`, `unresolvedMandatoryLots`, `decisionBasisHash` |

#### validation/ (L0: 0 files; L5: 3 files)

| File | Layer | Responsibility |
|------|-------|---------------|
| `dutify.ts` | L5 | Dutify landed-cost comparison adapter |
| `zonos.ts` | L5 | Zonos landed-cost comparison adapter |
| `avalara.ts` | L5 | Avalara landed-cost comparison adapter |

#### relations/ (L0: 0 files; L3: 1 file)

| File | Layer | Responsibility |
|------|-------|---------------|
| `resolve-process-relations.ts` | L3 | OCID + heuristic relation resolution engine |

#### evaluation/ (2 files)

| File | Responsibility |
|------|---------------|
| `harness.ts` | Gold-set pipeline runner — feeds curated lots through extraction, matching, and cost pipeline, collects per-lot results |
| `metrics.ts` | Lot-level precision/recall computation, archetype-stratified reporting, pass/fail against experiment thresholds |

### 5.3 Evaluation Harness Design

The evaluation harness runs the gold-set lot corpus through the full screening-grade pipeline and produces verifiable metrics. Results are persisted in the evaluation tables (§2.5.12) for regression analysis.

#### 5.3.1 Gold-Set Corpus Structure

The corpus is split into two sets:

**Gating corpus** (≥60 lots) — categories where L0 screening-grade intelligence is expected to succeed:
- Office supplies
- Simple cleaning hardware (buckets, brushes — not chemically treated consumables)
- Hand tools (single non-powered tools — not sets/kits/powered)
- Simple packaging subfamilies (corrugated cartons, poly mailers)

**Challenge corpus** (≥20 lots) — categories where the correct answer is often "abstain" or "blocker":
- Basic PPE (glove/helmet/eyewear families requiring standard compliance evidence)
- Metal or remedy-adjacent furniture (steel shelving, aluminium extrusions)
- Remedy-adjacent packaging families (aluminium foil, glass containers, tin mill)
- Services-only contracts (hard negatives — should be rejected)
- Mixed contracts with inseparable goods+services (hard negatives)
- Custom fabrication lots (hard negatives)
- Compliance-heavy regulated goods (should trigger abstention/review)

**Minimum totals:** 80+ lots for the real go/no-go gate. 50 lots is a smoke floor only. Include at least 10–20% double-annotated cases to measure label noise.

**Temporal specification:** Every gold-set case carries:
- `case_as_of_at` — the market state date the case represents
- `label_as_of_at` — when human labels were created/verified
- `evidence_bundle_hash` — hash of captured marketplace evidence and tariff data

This enables two test lanes:
1. **Reproducible benchmark lane** — uses captured evidence snapshots; deterministic, regression-safe
2. **Live smoke lane** — hits current external sources; validates operational viability but not reproducible

#### 5.3.2 Workflow

1. **Load** — Read gold-set corpus from `gold_set_cases` table. Filter by `corpus_set` (gating or challenge or both).
2. **Run** — Execute full pipeline per lot: extraction → matching → screening-grade cost estimation. In reproducible mode, use captured evidence bundles.
3. **Measure** — Compare pipeline output against human labels. Compute per-lot precision/recall. Write results to `evaluation_case_results`.
4. **Report** — Compute aggregate metrics, write to `evaluation_metric_summaries`. Produce structured report with:
   - Per-lot pass/fail against experiment thresholds
   - Archetype-stratified metrics (recall@k by `matchArchetype` × source)
   - Per-category slice results
   - Hard-negative false-positive rates
   - Decision agreement and abstention correctness
   - Unknown escape-hatch rates
   - Failure mode breakdown (which stages fail, why)

#### 5.3.3 Experiment Thresholds

**Headline experiments (global, on gating corpus):**

| # | Experiment | Pass Criteria |
|---|-----------|--------------|
| 1 | Extraction quality | ≥80% of gating-corpus lots yield ≥1 usable candidate |
| 2 | Matching quality | ≥70% of candidates with `matchArchetype` produce ≥1 genuine match |
| 3 | Margin accuracy | Margin bands agree with human expert on ≥75% of gating-corpus lots (±5pp) |

**Slice floors (prevent easy categories from washing out hard ones):**

| Gate | Threshold | Scope |
|------|-----------|-------|
| Per-category experiment 1 | ≥65% | No in-scope category below this |
| Per-category experiment 2 | ≥55% | No in-scope category below this |
| Per-category experiment 3 | ≥60% | No in-scope category below this |
| Per-archetype experiment 2 | ≥50% | No active `matchArchetype` slice with n≥8 below this |

**Hard-negative gates (on challenge corpus):**

| Gate | Threshold | What It Prevents |
|------|-----------|-----------------|
| False auto-qualify rate | ≤5% | System auto-qualifying services-only or custom fabrication |
| False genuine-match rate | ≤10% | System claiming matches on wrong-standard near-misses |
| Abstention correctness | ≥90% | System must correctly abstain/block on compliance gaps and remedy adjacency |

**Decision-oriented gates (on combined corpus):**

| Gate | Threshold | What It Proves |
|------|-----------|---------------|
| Decision agreement | ≥85% | Pipeline auto-qualify/review/reject agrees with expert |
| Auto-qualify precision | ≥85% | When system says "auto-qualify," expert agrees |

**Unknown escape-hatch caps:**

| Gate | Threshold | What It Prevents |
|------|-----------|-----------------|
| `matchArchetype = unknown` rate | ≤20% of gating corpus | System avoiding hard classification |
| `candidateType = unknown` rate | ≤15% of gating corpus | System avoiding hard classification |

**Cost:** ~$5–15 per benchmark batch run (model calls for extraction + matching adjudication on 80+ lots).

**Test lanes:**
- **Fast inner loop** (per-save, near-zero cost): extraction parsers, margin math, scoring functions, data model validation
- **Benchmark batch** (per-milestone, medium cost): full gold-set corpus through pipeline

### 5.3.4 Harness Database Isolation

**Critical:** The evaluation harness database MUST be a separate SQLite file from the live operational database. Since L0 and later layers share table names and row shapes, reusing the harness database as the L1 operational DB would mix gold-set rows with live data. The harness creates its own SQLite file, runs the pipeline against it, and produces metrics. The L1 daemon creates a fresh operational database.

### 5.3.5 Branded Scalar Types (Implementation Note)

The TypeScript `packages/contracts-schema` should use branded types (or equivalent nominal typing) to prevent accidental mixing of money and score representations:

- `Pence` (bigint GBP pence)
- `MinorUnits` (bigint original-currency minor units)
- `BpsBounded` (0–10000 confidence/coverage)
- `BpsSigned` (margins, deltas — may be negative or >10000)
- `ScopeId` (canonical scope identifier string)
- `StageRunId` (atomic commit cohort identifier)

These prevent a lot of "wrong type at boundary" bugs without runtime overhead.

### 5.4 Package Boundary Rules

Constitutional rules that apply from Layer 0 forward:

1. `packages/contracts-schema` and `packages/ui` are **boundary packages** — they must remain free of daemon-specific or Supabase-specific implementation
2. They must NOT import `packages/db`, desktop bridge code, daemon modules, or Supabase clients
3. Only procurement/supplier retrieval and workflow playbooks live under `skills/`
4. The landed-cost engine lives in `packages/intelligence` because cost estimation is API-based and deterministic, not agent-scraping

### 5.5 L0 File Count Summary

| Package | L0 Files | Purpose |
|---------|----------|---------|
| `packages/contracts-schema` | 4 | Domain types + Zod schemas |
| `packages/intelligence` | 14 | Qualification (4), extraction (1), search (1), customs (1), margins (5), evaluation (2) |
| Workspace bootstrap | 3 | `package.json`, `bunfig.toml`, `.env.example` |
| **Total L0 TypeScript** | **21** | |

This does NOT count skills (5 skill directories) or database migrations (owned by §2).

---

## 6. Testing & Acceptance

### 6.1 Definition of Done

All items must pass before Layer 0 is complete. Experiment 4 (runtime reliability) is deferred to Layer 1.

| # | Criterion | Verification |
|---|----------|-------------|
| 1 | **Gold-set lot corpus** contains ≥80 curated lots: ≥60 gating corpus (office supplies, simple cleaning, hand tools, simple packaging) + ≥20 challenge corpus (PPE, remedy-adjacent furniture/packaging, services-only hard negatives, custom fabrication). Human labels for product candidates, expected matches, margin bands, and expected decisions (auto-qualify/review/reject/abstain). ≥10% double-annotated. Each case has `case_as_of_at` and `evidence_bundle_hash`. | Manual: corpus file count, label coverage audit, double-annotation rate check |
| 2 | **Bilingual lexicon** seeded for top 30–50 product families with EN/ZH noun aliases, market framing terms, negative terms, unit/pack aliases, and standard/class patterns | Manual: lexicon file review; ≥30 families covered |
| 3 | **All 5 skills** have complete `SKILL.md` + `result.schema.json` + `checkpoint.schema.json` + `canary.inputs.jsonl` + `golden.assertions.yaml` | Automated: skill linting CI gate validates directory structure and metadata |
| 4 | **Evaluation harness** runs full gold-set corpus through extraction → matching → screening-grade cost, produces per-lot precision/recall report | Automated: `bun run harness` completes without error and emits metrics report |
| 5 | **Experiment 1 passes:** ≥80% of gating-corpus lots yield at least one usable product candidate. No in-scope category below 65%. | Automated: harness metrics check |
| 6 | **Experiment 2 passes:** ≥70% of candidates with `matchArchetype` produce at least one genuine match. No in-scope category below 55%. No active archetype slice (n≥8) below 50%. | Automated: harness metrics check |
| 7 | **Experiment 3 passes:** Margin bands agree with human expert on ≥75% of gating-corpus lots (±5pp). Decision agreement ≥85%. Auto-qualify precision ≥85%. | Semi-automated: harness computes bands, human expert reviews a sample |
| 7b | **Experiment 3b passes (hard-negative correctness):** False auto-qualify rate ≤5% on challenge corpus. Abstention correctness ≥90% on challenge cases. `matchArchetype=unknown` rate ≤20% on gating corpus. `candidateType=unknown` rate ≤15%. | Automated: harness metrics check |
| 8 | **All 4 unit test suites pass:** `goods-qualification.test.ts`, `quoteability.test.ts`, `tariff-parsing.test.ts`, `margin-rules.test.ts` | Automated: `bun test` |
| 9 | **SQLite schema validates:** all push-owned tables have sync tracking columns (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`), all monetary fields are integer pence (`_pence` suffix), all confidence/percentage fields are integer basis points (`_bps` suffix, 0–10000) | Automated: schema introspection test |
| 10 | **Each skill's SKILL.md** includes instructions for the agent to write a structured narrative of what it did (a "run manifest") as part of its output — prepares for the formal daemon-managed manifest system in Layer 1 | Manual: review each SKILL.md for narrative instruction block |

### 6.2 Test Scenarios

#### Fast Inner Loop (per-save, near-zero cost)

Deterministic logic tests that run on every save:

| Suite | File | What It Tests |
|-------|------|--------------|
| Goods qualification | `packages/intelligence/src/qualification/goods-qualification.test.ts` | CPV-to-legalCategory lookup (divisions 14–44 → goods, 45 → works, 50–98 → services), goodsDensityScore computation, goodsSeparability detection, access route classification, expiry check (30-day rule with DPS exception) |
| Quoteability scoring | `packages/intelligence/src/search/quoteability.test.ts` | quoteabilityScore formula (positive/negative signal weighting), recallRiskScore computation, searchEligibilityState thresholds (≥0.65 auto, 0.45–0.64 manual, <0.45 skip), SearchProfile generation for all candidates including below-threshold |
| Tariff parsing | `packages/intelligence/src/customs/tariff-parsing.test.ts` | HMRC Tariff API response parsing, HS6-to-duty-rate extraction, MFN rate identification, compound/specific duty expression handling, conservative default selection (MFN, residual, out-of-quota, standard VAT) |
| Margin rules | `packages/intelligence/src/margins/margin-rules.test.ts` | Revenue normalization by basis type (firm/estimated/framework/dynamic/concession/unknown), quote normalization to EXW basis, gross margin gate (≥30% threshold + confidence floor), net margin gate (≥20% conservative + blockers + coverage floor), critical blocker list enforcement, integer pence arithmetic throughout |

#### Benchmark Batch (per-milestone, ~$2–10 per run)

Full gold-set corpus through the pipeline:

1. Load all gold-set lots with human labels
2. Run full pipeline: extraction → matching → screening-grade cost estimation
3. Compare pipeline output against human labels per lot
4. Produce metrics report: extraction precision/recall, matching precision/recall by archetype, margin accuracy per category
5. Validate experiments 1–3 against thresholds

#### Specific Edge Cases

| Scenario | Expected Behavior |
|----------|------------------|
| Gold-set lot with 12 lots where 11 are irrelevant (services/works) | System correctly identifies the 1 qualifying goods lot; irrelevant lots rejected with correct `legalCategory` |
| Mixed contract with separable goods | `addressableValuePence` reflects goods portion only, not whole contract value; margin computed against goods-addressable revenue |
| Candidate with `matchArchetype = unknown` | System does NOT fabricate an archetype; uses generic matching with wider shortlist budgets; `identityConfidence` penalty applied |
| Screening-grade landed cost | Uses conservative defaults throughout: MFN rate (not preferential), residual duty (not named-exporter), out-of-quota, standard VAT |
| DPS/Dynamic Market notice | 30-day expiry filter NOT applied; notice remains active |
| Pure services notice with goods-like CPV codes (software) | Correctly classified as `services` despite CPV division placement |
| Notice with missing deadline | Routed to manual review, NOT auto-passed |
| Framework call-off notice | Flagged for access-route review, NOT auto-passed as fully actionable |
| Bundle candidate | Decomposed into constituent candidate families before downstream processing; `parentCandidateId` and `candidateRole` recorded |
| Regulated goods candidate without compliance evidence | `complianceEvidenceTier = absent` prevents automatic match acceptance regardless of confidence scores |
| Pack-basis mismatch (per-unit vs per-box) | `quoteUsabilityConfidence` penalized; not treated as valid pricing signal |
| Query returns only accessories | Classified as `accessory_swamp` failure mode; not counted as genuine marketplace presence |
| **Hard-negative: services-only notice** | Correctly rejected at prequalify_notice phase; NOT auto-qualified |
| **Hard-negative: custom fabrication lot** | Correctly classified as `custom_or_fabricated`; marketplace search skipped; NOT auto-qualified |
| **Hard-negative: wrong-standard near-match** (EN 1078 cycling helmet vs EN 397 industrial) | Hard rejected by standards normalization; NOT counted as genuine match |
| **Hard-negative: remedy-adjacent packaging** (aluminium foil, glass containers) | `customs_branch_divergent` or `trade_remedy_detected` blocker raised; NOT auto-qualified |
| **Abstention case: PPE without compliance evidence** | `complianceEvidenceTier = absent` prevents auto-match; system correctly routes to review/abstain |
| **Abstention case: metal furniture with steel shelving** | Trade remedy exposure flagged; system correctly routes to review or blocker |
| **Sub-penny unit price** (£12.99 / 5000 gloves) | `offerTotalGbpPence = 1299`, `priceBasisQty = 5000` preserves precision; margin computed from offer-total, not truncated per-unit |

### 6.3 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Gold-set too small for statistical significance | Medium | Medium | Start with 50 lots, expand per category; lot-level measurement (not notice-level); per-archetype and per-category metrics to detect weak spots |
| Screening-grade landed cost too inaccurate for margin estimation | Medium | High | Conservative defaults ensure overestimation of costs, not underestimation — system rejects on the safe side; Layer 5 delivers decision-grade accuracy |
| eBay listings don't represent real supplier economics | High | Medium | eBay is first validation source only; Layer 5 adds Alibaba/1688 for wholesale pricing; eBay prices treated as `benchmark_ceiling`, not `executable` |
| Wrong product match inflates margin | High | High | Dual confidence scoring with lexicographic gating + archetype-specific identity floors + four-state evidence comparison + compliance evidence tiers + operator review |
| Attachment blindness — extraction from notice text only | High | Critical | Gold set includes pre-acquired documents; Layer 1 adds live document acquisition; `qualificationBasis` tracks whether extraction had document support |
| Pack-basis normalization error | High | High | Listing pack qty/unit extraction; normalized unit basis on every quote; `quoteUsabilityConfidence` penalizes unclear basis |
| Price-signal false precision | High | Medium | Margin bands (not point estimates); `priceSignalRole` classification; RFQ escalation for high-value opportunities |
| Compliance-critical false positive | Medium | High | `regulated_goods` candidateType + `complianceEvidenceTier` + detail-page enrichment before rejection + standard normalization |
| Missing evidence treated as conflict | High | High | Four-state evidence model; only `conflict` triggers hard rejection; `missing` and `ambiguous` are confidence penalties |
| Static shortlist truncation drops valid matches | High | Medium | Adaptive shortlist budget driven by `recallRiskScore`; diversity enforcement; recall@k measurement |
| Query vocabulary mismatch | High | Medium | matchArchetype-driven query templates; market framing slots; query-wave pattern with staged recovery |
| Cross-language market framing gap | Medium | Medium | `marketFrameEn`/`marketFrameZh` slots; curated lexicon; Chinese retry on Alibaba |
| Infrastructure-first delivery delays thesis validation | High | High | Intelligence pipeline validated first (L0 experiments); no daemon/desktop/sync investment until intelligence thesis proven |
| Premature category breadth | High | High | Narrow whitelist (6 categories); expand only after system proves reliable |
| No labeled benchmark corpus for calibration | High | Medium | Gold-set lot corpus with human labels is the first deliverable of L0 |
| HMRC FPO API access not secured | Medium | High | FPO not a hard dependency; fallback to CPV-to-HS mapping; HS6-level screening in L0 |

---

## 7. Open Questions, Forward Compatibility & Dependencies

### 7.1 Open Questions

These questions are routed from the research document §10. They are **explicitly unresolved** — do NOT silently resolve them. Each must be answered before or during implementation.

#### Requiring User Input

**§10 User Input #1: Is Scotland in the MVP ingestion scope?**
- Context: Determines whether PCS is a first-wave scraping skill or a later phase.
- Options: England/Wales/NI only, or full UK including Scotland.
- Recommendation: Include PCS skill in the first live procurement wave only if Scotland matters commercially on day one.
- **Layer 0 impact:** Affects gold-set source selection — should gold-set include PCS-sourced notices?

**§10 User Input #4: Should the margin model assume VAT-recoverable importer?**
- Context: Materially changes whether import VAT is treated as an economic cost or primarily a cash-flow item.
- Recommendation: Model both views, but set the default based on real importer-of-record structure.
- **Layer 0 impact:** Affects margin-rules test expectations and screening-grade landed cost defaults.

**§10 User Input #5: Which machine is expected to stay online for scheduled execution?**
- Context: One active local daemon per org assumed; freshness depends on at least one machine staying online.
- Recommendation: Identify one designated always-on workstation or mini-server-class host.
- **Layer 0 impact:** None directly — daemon is L1. But gold-set evaluation harness environment should match expected production host.

#### Requiring Further Research

**§10 Further Research #1: Exact procurement source list by commercial importance**
- Why unresolved: Broad platform scope described, but which portals matter most beyond government sites is unclear.
- Research needed: Rank target sources by contract volume, goods density, and bidability.
- **Layer 0 impact:** Informs gold-set source selection and notice variety.

**§10 Further Research #5: Auto-qualification coverage floor calibration**
- What is the correct `process_coverage_floor_bps` for auto-qualification? Default is 5000 (50%).
- May be too aggressive for mixed contracts with many service lots.
- **Layer 0 impact:** Affects coverage rollup logic in the evaluation harness.

**§10 Further Research #8: Customs cost spread materiality threshold**
- What `customsCostSpreadBps` value should trigger the `customs_branch_divergent` blocker?
- A 2.7%–4.7% spread is arguably tolerable; 2.7%–48.5% is clearly a blocker. Threshold needs calibration against whitelist categories.
- **Layer 0 impact:** Affects screening-grade blocker logic.

**§10 Further Research #9: Packaging materials sub-category reclassification**
- Packaging contains both easy subfamilies (corrugated cartons, poly mailers) and dangerous subfamilies (aluminium foil with 14.2%–35.6% residual AD rates, glass containers under active investigation, tin mill products with 27.85%–49.98% duties).
- Should the whitelist split packaging into separate sub-categories with different pipeline treatment?
- **Layer 0 impact:** Directly affects gold-set lot selection and category-specific test expectations.

#### Assumptions Requiring Validation

**§10 Assumption #1: UK procurement is the primary geography.**
- Impact if wrong: Source layer design changes significantly.
- **Layer 0 impact:** Gold-set lot selection assumes UK sources.

**§10 Assumption #3: Margin thresholds are business gates, not just UI labels.**
- Impact if wrong: Workflow state design may be too rigid.
- Validation: Confirm whether 30% gross and 20% net are hard filters or soft indicators.
- **Layer 0 impact:** Directly affects gross/net gate implementation in margin-rules tests.

**§10 Assumption #4: Import VAT is usually recoverable (UK VAT-registered importer).**
- Impact if wrong: Economic-margin modeling could materially overstate opportunity quality.
- Validation: Confirm importer-of-record and VAT-registration structure.
- **Layer 0 impact:** Affects screening-grade landed cost defaults and margin test expectations.

### 7.2 Forward-Compatibility Provisions

All provisions below are schema or structural decisions made in Layer 0 that exist to support later layers. They are NOT implemented features — they are nullable columns, structural patterns, or design decisions that prevent breaking changes later.

| Provision | What Exists in L0 | Target Layer | Why It Matters |
|-----------|-------------------|-------------|----------------|
| Sync tracking columns | `owner_id`, `row_version`, `last_local_seq`, `stage_run_id` on ALL push-owned tables from creation | Layer 4 (sync engine) | Avoids schema migration when sync activates; columns populated on first write even before outbox exists |
| Validation columns on `workflow_jobs` | `parent_job_id` (nullable), `validation_required` (default `false`), `validation_verdict_id` (nullable) from creation | Layer 3 (dry-run validation) | Avoids adding columns to a hot table later |
| `stage_run_id` on pipeline tables | Column present from creation; not actively managed by daemon in L0 | Layer 1 (stage tracking) | Enables atomic commit cohort tracking without schema change |
| Skill directory structure | Follows production standard: SKILL.md + schemas + canary + golden | Layer 1 (skill linting CI gate) | Skills authored in L0 pass L1 linting without restructuring |
| SearchProfile for ALL candidates | Created even for below-threshold candidates (`searchEligibilityState = skip`) | Layer 2 (RFQ re-entry) | Operator can trigger manual search or RFQ using existing search profile |
| `pending_operator_intents` table | Exists from L0 schema with CAS payload structure | Layer 2 (desktop decisions), Layer 4 (remote CAS flush) | Local decision capture without authority matrix violation; no table reclassification needed |
| `pipeline_assessments.validation_state` | Column exists with default `'unverified'` from creation | Layer 3 (validation system) | Every pipeline assessment already carries validation state |
| All future-layer columns nullable from creation | Decision-grade fields on `landed_cost_estimates`, coverage fields on `pipeline_assessments`, etc. | Layers 2–5 | Later layers populate existing columns; never add new columns to existing tables |
| Run manifest preparation in SKILL.md | Each skill includes instructions for agent to write structured narrative | Layer 1 (daemon-managed manifests) | Skills already produce narrative output; L1 formalizes it |
| `lotId` required on `product_candidates` | NOT NULL constraint from creation | All layers | Prevents orphaned candidates that can't trace to lot-level qualification |
| Qualification two-phase model | `prequalify_notice` (notice-grain) and `qualify_lot` (lot-grain) as separate job kinds and stage enum values | Layer 1 (Job Runner handles both) | Avoids forced lot extraction before early rejection of non-goods notices |
| `pending_operator_intents` reviewed-basis fields | `reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`, `reviewed_at`, `superseded_by_intent_id` present from creation | Layer 2 (desktop decision capture) | Desktop writes reviewed-basis metadata from first operator use; L4 CAS flush uses it |
| Value-weighted coverage metrics | `priced_value_coverage_bps`, `landed_cost_value_coverage_bps`, `net_qualified_value_coverage_bps` on `pipeline_assessments` | Layer 2 (desktop display), Layer 4 (sync projection) | Auto-qualify gate uses value coverage, not lot-count coverage |
| `product_commodity_codes` as full entity | `id`, `digits`, `source`, `score_bps`, `rank_ordinal`, `description`, `evidence_json` from creation | Layer 5 (multi-source recall and ranking) | L5 customs engine writes multi-source candidate codes with provenance; thin junction table would force schema change |
| Natural keys for rerun diffing | `notice_lots.lot_key`, `product_candidates.candidate_key` from creation | Layer 1 (stage tracking), Layer 0 (gold-set scoring) | Deterministic keys enable stage-run diffing and evaluation harness comparison |
| Basis-total pricing on `supplier_quotes` | `offer_total_original_minor`, `offer_total_gbp_pence`, `price_basis_qty`, `price_basis_unit` from creation | Layer 0 (screening-grade), Layer 5 (decision-grade) | Preserves sub-penny precision for bulk-quantity items; margin computed from offer totals, not truncated per-unit |
| Evaluation tables | `gold_set_cases`, `evaluation_runs`, `evaluation_case_results`, `evaluation_metric_summaries` local-only from creation | Layer 0 (harness only) | Persisted evaluation results enable regression tracking across pipeline iterations |

### 7.3 Dependencies

**Layer 0 has no upstream layer dependencies** — it is greenfield.

#### TypeScript Production Dependencies

| Package | Purpose | Layer 0 Scope |
|---------|---------|--------------|
| `zod` | Runtime validation of pipeline entities and skill result contracts | Full — used for all entity validation |
| `marked` (or equivalent) | Markdown manifest rendering | Minimal — used for gold-set evaluation report output |

#### TypeScript Development Dependencies

| Package | Purpose |
|---------|---------|
| `vitest` | Unit test runner for intelligence pipeline tests |
| `playwright` | Browser automation for skill E2E testing (gold-set evaluation uses CLI, but skill canary tests need browser) |
| `typescript` | TypeScript build toolchain |
| `turbo` (or equivalent) | Monorepo task orchestration |

#### Rust Dependencies (Layer 0 scope: none)

No Rust dependencies in Layer 0. Rust daemon is introduced in Layer 1. The `Cargo.toml` workspace root is not created until Layer 1.

#### Configuration

| Config File | Layer 0 Entries |
|-------------|----------------|
| `.env.example` | `SQLITE_PATH` — path to local SQLite database; `HMRC_TARIFF_API_BASE` — HMRC Trade Tariff API base URL (public, no key); `AGENT_CLI_PATH` — path to agent CLI binary (Codex/Claude); `GOLD_SET_PATH` — path to gold-set lot corpus |

### 7.4 ADRs Addressed

| ADR | Decision | Layer 0 Action |
|-----|----------|---------------|
| **ADR-001:** Skills as primary data retrieval | Skill-based agent scraping via browser automation for all procurement and supplier data | First 5 skills authored with complete directory standard |
| **ADR-005:** Skill directory standard | SKILL.md + schemas + canary inputs + golden assertions per skill | All 5 skills follow production standard; 8-question authoring guide established |
| **ADR-006:** Internal landed-cost engine | Build internal engine instead of third-party API | Screening-grade subset implemented: HMRC Tariff API for duty, flat freight, single FX, conservative defaults |

---

## Cross-Domain Notes

These notes flag L0 decisions that require awareness or action in other layers. They are informational — the owning layer doc is authoritative for its own implementation.

<!-- CROSS-DOMAIN NOTE FOR L1: Qualification is two separate job kinds (`prequalify_notice` at notice-grain and `qualify_lot` at lot-grain). The Job Runner must handle both in the pipeline flow: prequalify_notice → acquire_docs → extract_lots → qualify_lot → extract_products. The `prequalify_notice` result has no lotId. The `qualify_lot` result has lotId and addressableValuePence. -->

<!-- CROSS-DOMAIN NOTE FOR L1: `process_stage_heads` has `created_at timestamptz not null default now()` from L0. L1 populates this on first head creation. `bid_workflow_states` has `stage_run_id text` from L0 for pipeline tracking consistency. -->

<!-- CROSS-DOMAIN NOTE FOR L1: Money columns across all push-owned tables use `bigint` (not `integer`) to support framework ceiling values exceeding £21.4M. `row_version` and `last_local_seq` also use `bigint`. The L1 daemon must use 64-bit integers for all pence arithmetic and sequence counters. -->

<!-- CROSS-DOMAIN NOTE FOR L1: `supplier_quotes` has basis-total pricing fields (`offer_total_original_minor`, `offer_total_gbp_pence`, `price_basis_qty`, `price_basis_unit`) for sub-penny unit-price accuracy. Margin calculations should use offer-total values, not per-unit values, to avoid integer truncation on bulk-quantity items. -->

<!-- CROSS-DOMAIN NOTE FOR L1: Natural keys `notice_lots.lot_key` and `product_candidates.candidate_key` exist from L0. L1 must populate these deterministically so stage-run diffing and evaluation harness comparison work correctly. -->

<!-- CROSS-DOMAIN NOTE FOR L2: `pending_operator_intents` has expanded reviewed-basis fields (`reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`, `reviewed_at`, `superseded_by_intent_id`). L2 desktop writes these on every operator decision. The `base_remote_revision` defaults to 0 for pre-L4 local-only operation. Latest-per-process coalescing uses `superseded_by_intent_id`. -->

<!-- CROSS-DOMAIN NOTE FOR L2: Auto-qualification coverage floor uses **value-weighted** coverage (`netQualifiedValueCoverageBps >= 5000`), not lot-count coverage. L2 desktop displays both lot-count and value-weighted coverage but the gate logic uses value. -->

<!-- CROSS-DOMAIN NOTE FOR L3: Qualification split means the Validation Policy Engine sees `prequalify_notice` and `qualify_lot` as separate skill/job kinds. Validator overlays may differ per phase. -->

<!-- CROSS-DOMAIN NOTE FOR L4: `product_commodity_codes` is a full entity table (with `id`, `digits`, `source`, `score_bps`, `rank_ordinal`, `description`, `evidence_json`) rather than a thin junction table. L4 sync outbox protocol handles it as a parent-embedded set of `product_candidates` (per constitutional rule 19). No standalone sync tracking columns — synced atomically with the parent candidate. -->

<!-- CROSS-DOMAIN NOTE FOR L4: Push-owned tables do NOT have SQL REFERENCES to local-only tables (constitutional rule 15). Cross-authority references are opaque IDs. The Supabase DDL must not add REFERENCES for columns that point to local-only tables (e.g., `contract_notices.source_id`, `search_attempts.session_id`, `process_stage_status.error_manifest_id`). -->

<!-- CROSS-DOMAIN NOTE FOR L4: `local_outbox` is not pre-created by the actual L0 SQLite migration. The constitutional column shape (`commit_group_id`, `writer_epoch`) on `local_outbox` is reserved here as a forward-compat note for when L4 first materializes the table in `migrations/0012_l4_sync_state.sql`; `commit_group_id` is text NULL and `writer_epoch` is integer NULL at first creation. -->

<!-- CROSS-DOMAIN NOTE FOR L5: `product_commodity_codes` expanded schema supports multi-source recall: each commodity code row carries `source` (fpo, cpv_prior, rule_hint, tariff_keyword, etc.), `digits` (6/8/10), `score_bps`, and `rank_ordinal`. L5 customs engine writes multiple rows per candidate from independent recall sources. Set-replacement semantics per candidate per `stage_run_id`. -->
