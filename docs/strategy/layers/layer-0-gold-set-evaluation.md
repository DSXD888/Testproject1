# Layer 0 — Gold Set + Evaluation Harness

> **Authority:** This document is the execution-grade specification for Layer 0. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M0
> **Dependencies:** None (greenfield)
> **Exit criteria:** Experiments 1–3 pass on the gold-set corpus.

---

## 1. Overview, Capability & Project Shape

### 1.1 Layer Narrative

Layer 0 proves the intelligence thesis on a curated lot corpus before building any live-source runtime. This is evaluation infrastructure first: gold-set data, harness logic, skill authoring standards, and the screening-grade TypeScript pipeline exercised against curated inputs. No daemon, no desktop, no sync, and no remote authority.

Skills run from the command line for benchmark execution only. Results land in a harness-local SQLite dataset and are verified by the automated evaluation harness and manual inspection. Layer 0 deliberately does **not** count as the first live pipeline layer; it is the measurement system that Layer 1 must satisfy.

**Why this layer exists:** The riskiest assumptions live in the intelligence pipeline — can the system extract usable product candidates, match them to real marketplace listings, and produce margin estimates accurate enough to act on? Infrastructure is well-understood engineering that follows, not leads. Wrong answers at scale are worse than no automation. Building a reliable daemon that produces unreliable intelligence is wasted work. Building credible intelligence first means every infrastructure component has a clear reason to exist.

**If we never build Layer 1:** The gold set and evaluation harness remain valuable as a benchmarking tool and intelligence pipeline prototype. The skills and TypeScript intelligence code can run standalone indefinitely. The gold-set corpus, bilingual lexicon, and evaluation metrics survive as reusable assets.

### 1.2 Capability Delivered

A curated gold-set lot corpus (50–100 lots) runs through extraction, matching, and screening-grade cost estimation, producing per-lot precision/recall metrics against human-labeled ground truth. Three experiments are validated:

| # | Experiment | Pass Criteria | What It Proves |
|---|-----------|--------------|----------------|
| 1 | **Extraction quality** — Can we pull structured lot/product candidates from real procurement documents? | ≥80% of lots in the gold set yield at least one usable product candidate with correct unit, quantity, and specification | The system can read procurement intent |
| 2 | **Matching quality** — Can marketplace search find relevant supplier listings for extracted candidates? | ≥70% of product candidates with a `matchArchetype` produce at least one genuine match (not a false positive from keyword overlap) | The system can connect demand to supply |
| 3 | **Margin accuracy** — Can screening-grade landed cost produce margin bands that distinguish real opportunities from noise? | Margin bands agree with human expert assessment on ≥75% of gold-set lots (within ±5pp for screening-grade) | The system produces actionable commercial signal |

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
| 1 | **Extraction quality** | ≥80% of gold-set lots yield ≥1 usable product candidate with correct unit, quantity, specification | System can read procurement intent | L0 |
| 2 | **Matching quality** | ≥70% of candidates with `matchArchetype` produce ≥1 genuine match | System can connect demand to supply | L0 |
| 3 | **Margin accuracy** | Margin bands agree with human expert on ≥75% of lots (±5pp) | System produces actionable commercial signal | L0 |
| 4 | **Runtime reliability** | 10 consecutive batch runs, ≥90% success rate, 0 orphaned processes | System can operate unattended | L1 |

Stop/go rule: Do not start Layer 1 until Layer 0 passes experiments 1–3 on the gold-set corpus.

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

**Explicit no-list (defer until system proves reliable on whitelist):**

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
| Integer pence for money (`_pence` suffix) | §5.4 | Type system + DDL |
| Integer basis points for confidence/percentage (`_bps` suffix, 0–10000) | §5.4 | Type system + DDL |
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
- `matchArchetype`-driven query planning (6 archetypes: `standard_class_led`, `dimension_material_led`, `model_exact`, `brand_series`, `functional_spec`, `unknown`)
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

### 3.4 Stage 4 — Qualify (Preliminary)

Per-lot qualification with multi-dimensional goods classification.

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

**Output:** `QualificationResult` per lot with `lotId`, `expiryQualified`, `goodsQualified`, `accessRoute`, `qualificationBasis`, `addressableValuePence`.

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

**Document set output contract:** Produces a first-class `TenderDocumentSet` output:
- Which documents found and their roles: `spec` | `pricing_schedule` | `boq` | `appendix` | `other`
- Completeness state: `none` | `partial` | `full`
- `documentSetHash` computed from acquired document set — this is the invalidation key: if documents are re-acquired and hash changes, downstream stages (6–8) are hard-invalidated

If no public documents available → proceed with notice text only (lower extraction confidence, `qualificationBasis = notice_only`).

**Layer 0 stub:** Gold-set documents are pre-curated. Full implementation in Layer 1 (live source interaction).

### 3.6 Stage 6 — Extract Lots

Parse tender documents and OCDS lot data into structured lot records.

**Process:**
1. Parse OCDS lot data from normalized notice (if structured lot fields present)
2. If no structured lots, use `lot-extraction` skill on acquired documents
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
- Compute and store `fxBasisDeltaPct` between the two rates

**VAT determination:**
- `vatRateState`: `standard` | `reduced` | `zero` | `unresolved` (default to standard when unresolved)
- Import VAT recovery treatment: postponed (full/partial), cash-paid (full/partial), non-recoverable
- `vatRecoveryRate` — proportion recoverable
- `valuationMode`: `estimation` | `declaration` — distinguishes marketplace-derived proxy values from actual transaction prices

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

### 3.13 Stage 13 — Coverage Rollup

Process-level business rollup computed from candidate-level results. **These are the authoritative coverage metric definitions:**

| Metric | Formula | Meaning |
|--------|---------|---------|
| `pricedCoverageBps` | (lots with ≥1 priced candidate / total qualifying lots) × 10000 | Pricing breadth |
| `landedCostCoverageBps` | (lots with landed cost / total qualifying lots) × 10000 | Landed-cost breadth |
| `netQualifiedCoverageBps` | (lots passing net gate / total qualifying lots) × 10000 | Net-qualified breadth |
| `unresolvedMandatoryLots` | Count of mandatory lots without resolution | Gap count |
| `decisionBasisHash` | SHA-256 of pipeline data the operator would review | Staleness detection |

**Auto-qualification coverage floor:** `netQualifiedCoverageBps >= 5000` (50%). If below-quoteability candidates represent >50% of process value and remain unresolved, the process cannot auto-qualify regardless of net margin on resolved candidates.

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
| qualify → acquire_docs | `QualificationResult` with lotId, expiryQualified, goodsQualified, accessRoute | Per-lot pass/review/reject |
| acquire_docs → extract_lots | Document set completeness, document roles | `TenderDocumentSet` with `documentSetHash` |
| extract_lots → extract_products | `NoticeLot` with title/description, lotNumber | At least one lot per qualified notice |
| extract_products → score_quoteability | `ProductCandidate` with lotId, headNoun, identityConfidence, candidateType, matchArchetype | At least one candidate per qualified lot |
| score_quoteability → price_match | quoteabilityScore, recallRiskScore, SearchProfile with queryWaves | quoteabilityScore ≥ 0.45 for auto-search |
| price_match → gross_gate | `SupplierQuote` with matchConfidence, quoteUsabilityConfidence, priceSignalRole | At least one quote per searched pair |
| gross_gate → landed_cost | CandidatePricingAssessment: selected quote, revenue basis, confidence gates | grossMarginBps, selected quote |
| landed_cost → net_gate | `LandedCostEstimate` anchored to candidate + quote | netMarginLowPct/BasePct/HighPct, blockers |
| net_gate → manifest | Process-level rollup with coverage metrics | Coverage-aware qualification |
