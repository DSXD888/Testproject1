# Layer Strategy Handoff — UK Public Procurement Intelligence Platform

> **Role:** You are an implementation architect specializing in sequentially-shippable layer decomposition for complex multi-stack systems.
>
> **Deliverable:** A single strategy document (`docs/strategy/layer-strategy.md`) that decomposes the monolithic research plan into sequentially-shippable layers. Each layer must be independently deployable, testable, and deliver incremental user value.
>
> **Scope:** Strategy and sequencing ONLY. You are NOT writing per-layer spec docs — those come later from the beads pipeline.

---

## 1. Source Material

### Primary Source (MANDATORY FULL READ — DO NOT SKIM)

**`docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`** (~8,400 lines)

This is a dense, heavily cross-referenced architecture research document that has been through 3 iterations of expert review (planmaxer) and 2 rounds of feedback integration (harmonymaxer). It covers:

- 15-stage procurement intelligence pipeline
- ~30+ SQL tables with strict authority matrix (local SQLite vs Supabase)
- Hybrid PTY + contract agent execution model
- 6 skill categories including validators
- Desktop control panel (Tauri) + web viewing portal (Next.js)
- Worker-validator system with run manifests
- Customs classification and landed-cost engine
- Sync engine with push/pull protocols

**WHY YOU MUST NOT SKIM:** The architecture has deep cross-cutting dependencies that are invisible when reading individual sections. For example:
- The `stage_run_id` concept (§5.3) threads through job execution, stage tracking, outbox entries, and sync batches — splitting these across layers without understanding the thread creates schema migration debt.
- The authority matrix (§4.7, ADR-002) means every new table must be classified as push-owned, remote-owned, or local-only BEFORE it's created. Getting this wrong in an early layer requires rewriting sync infrastructure.
- The skill system (§4.10, §5.11) defines interaction profiles, metadata declarations, and promotion flows that constrain how skills are authored and deployed. Shipping the daemon without the skill metadata system means retrofitting constraints later.

### Secondary Sources

- **`AGENTS.md`** — Agent guidelines, toolchain (Rust + Bun), code discipline rules
- **`docs/research/2026-03-15-worker-validator-run-manifest-research.md`** (2,256 lines) — Standalone research on worker-validator system (already integrated into main doc, but useful for deeper context)

---

## 2. Structured Research Protocol

**CRITICAL:** Do NOT begin designing layers until you have completed ALL investigations below and synthesized them into unified maps. The quality of your decomposition depends entirely on the depth of your understanding.

Use subagents for parallel investigations where possible. Each investigation produces structured output that feeds into the synthesis phase.

### Investigation A: Data Structure & Schema Cartography

**Sections to read:** §5.4 (TypeScript interfaces + SQL tables), §4.7 (authority matrix), §6.3 (sync infrastructure), §6.4 (pull protocol)

**Produce:**
1. Complete table inventory with authority classification:
   - Push-owned tables (daemon writes, synced to Supabase): list all
   - Remote-owned tables (Supabase writes, pulled by daemon): list all
   - Local-only tables (never synced): list all
   - Projection tables (push-only summaries for web): list all
   - Sync infrastructure tables (SQLite-only): list all
2. Foreign key dependency graph between tables (which tables reference which)
3. For each table: what pipeline stage creates/modifies it
4. Junction tables and their parent relationships
5. Schema fields that exist for forward compatibility (e.g., `stage_run_id`, `schema_epoch`, `row_version`, `content_hash`)

**Key questions:**
- Which tables form atomic groups (must be created together)?
- Which tables can exist with nullable FK references to tables created in later layers?
- What is the minimum table set for a single ingestion cycle?
- Which sync tracking columns (`owner_id`, `row_version`, `last_local_seq`) must be present from day 1 vs can be added later?

### Investigation B: Invariant & Safety Map

**Sections to read:** §4.7 (authority matrix rules), §5.5 (contract state model), §5.3 (stage scoping), §5.11 (error types), ADR-002, ADR-005, ADR-006

**Produce:**
1. Every invariant stated in the document, with:
   - Which layer it MUST be activated in (the layer where the data it protects is first introduced)
   - Whether it's constitutional (must never be relaxed) or operational (can be simplified early)
2. Safety properties:
   - No real-money bids without human approval
   - No mixed-write tables (authority matrix)
   - Fail-open validation (pipeline continues even if validator fails)
   - Conservative defaults for customs (MFN rates, residual duty, standard VAT)
3. Error handling invariants:
   - Every error must produce an error manifest
   - Cleanup verdicts for every agent session (clean/forced/leaked)
   - Run manifests for every skill execution (with machine-authored fallback)

**Key questions:**
- Which invariants span multiple components?
- Which invariants can be enforced with simplified implementations in early layers?
- What happens if an invariant is "temporarily relaxed" — does it create data corruption?

### Investigation C: Pipeline Operation Dependency Map

**Sections to read:** §5.3 (ingestion flow — all 15 stages), §5.5 (contract state model), §5.6 (ingestion flow details), §5.7 (intelligence pipeline)

**Produce:**
1. Complete 15-stage pipeline with:
   - Input data requirements per stage
   - Output data produced per stage
   - Which tables are read/written per stage
   - Scope grain per stage (process/notice/lot/candidate/candidate_source_pair)
   - Which stages require agent skills vs pure daemon computation
2. Stage dependency graph (which stages must complete before which)
3. Stage scoping model: which stages operate at process level, lot level, candidate level
4. Validation insertion points (steps 12a-12d)
5. Gates and their pass criteria (quoteability gate, gross margin gate, net margin gate)

**Key questions:**
- What is the minimum number of stages for a useful end-to-end cycle?
- Which stages are purely computational (daemon-only) vs require skill execution?
- Which stages produce data that downstream stages depend on critically?
- Can any stages be stubbed (returning hardcoded results) for early layers?

### Investigation D: Agent Orchestration & Skill System Map

**Sections to read:** §4.9 (agent orchestration), §4.10 (skill system), §5.2 (daemon components), §5.11 (skill directory), ADR-004, ADR-005

**Produce:**
1. Complete skill category inventory:
   - Category 1: Source skills (which ones, what they scrape)
   - Category 2: Intelligence skills (qualification, enrichment, matching)
   - Category 3: Action skills (bid workflow)
   - Category 4: Operational skills (auth-refresh, source-health)
   - Category 5: Search/matching skills
   - Category 6: Validator skills
2. Daemon component dependency graph:
   - Job Runner, Scheduler, Agent Adapter, Process Supervisor, Resource Arbiter
   - How they interact: who creates jobs, who executes them, who monitors
3. PTY vs Contract mode: which skill categories use which mode
4. Skill metadata requirements (interaction_profile, result_schema_version, auth_requirements)
5. Skill promotion flow (draft → candidate → canary → shadow → stable → rollback)
6. Session lifecycle phases (7 phases)

**Key questions:**
- What is the minimum daemon infrastructure to run a single skill?
- Can the daemon start with PTY-only and add contract mode later, or vice versa?
- Which daemon components are required from day 1 vs can be added incrementally?
- How does the resource arbiter interact with the job runner?

### Investigation E: Sync Engine & Data Flow Map

**Sections to read:** §4.7, §5.4 (sync interfaces), §6.3 (push protocol), §6.4 (pull protocol), §9.4 (sync testing), ADR-002

**Produce:**
1. Push protocol details:
   - Transactional outbox pattern
   - Monotonic `local_seq` + `row_version` + `content_hash`
   - Batch boundary rules (never cross epoch/stage_run_id)
   - Single-in-flight batch constraint
   - Identical-envelope retries
2. Pull protocol details:
   - `remote_change_seq` serialization
   - Transaction-scoped advisory locks per domain
   - CAS for shared decisions
   - Pending operator intent handling
3. Schema epoch protocol (expand/drain/upgrade/contract)
4. Recovery model (warm/cold rebuild)
5. Cross-authority consistency classification (display/control/automation)

**Key questions:**
- Can sync be deferred entirely until a later layer, or must sync tracking columns exist from day 1?
- Can the desktop app work without sync (local-only mode)?
- What is the minimum sync implementation for MVP?
- Can push and pull be implemented independently in different layers?

### Investigation F: Frontend & Presentation Map

**Sections to read:** §5.8 (shared React components), §5.10 (desktop/web architecture), ADR-003, ADR-008

**Produce:**
1. Desktop control panel capabilities:
   - Read: what data views
   - Write: what mutations (approve/reject/watch, trigger workflows)
   - Trigger: what daemon operations
2. Web viewing portal capabilities:
   - Read-only views
   - Supabase projection dependencies
   - Live update mechanism (Broadcast invalidation)
3. Shared component architecture:
   - Frame + slot composition model
   - What is shared (presentational) vs what is shell-specific
   - Package structure (`packages/ui`, `packages/contracts-schema`)
4. Degraded-mode UX (three sync states: healthy, degraded, offline)

**Key questions:**
- Can the desktop app be useful without the full pipeline?
- Can the web portal be useful with partial sync?
- What is the minimum desktop view for an MVP?

### Investigation G: Gold Set, Evaluation Harness & MVE Definition

**Sections to read:** §1 (executive summary — MVE definition), §6.1 (complexity analysis — MVE-critical tier), §6.2 (risks — delivery risks), §11 (next steps — milestone 0), §8.2 (file reference — evaluation harness files)

**Produce:**
1. MVE (Minimum Viable Experiment) definition:
   - What is the gold set? (Curated procurement notices with known-correct pipeline outputs)
   - What evaluation metrics are defined? (precision, recall, margin accuracy)
   - What files comprise the evaluation harness? (harness.ts, metrics.ts, gold set data)
   - What is the ~25-35 file scope of M0?
2. Core validation order (4 experiments with pass criteria from §11):
   - What must each experiment prove?
   - What pass criteria gate progression to the next experiment?
3. Category whitelist:
   - Which product categories are in-scope for MVP?
   - Which are explicitly excluded?
   - How does the no-list work?
4. Validation-first philosophy:
   - Why is the evaluation harness built BEFORE pipeline code?
   - How does the gold set anchor all subsequent pipeline development?

**Key questions:**
- What is the minimum gold set size for meaningful evaluation?
- Can the evaluation harness be layer 0, or must it co-exist with pipeline code?
- How does the gold set evolve as layers add capabilities?
- Does the evaluation harness have ANY runtime dependencies, or is it purely test infrastructure?

### Investigation H: Customs, Landed Cost & Trade Compliance Map

**Sections to read:** §4.5 (customs classification), §4.6 (FX), §5.7 (landed-cost pipeline steps), ADR-006, Appendix A

**Produce:**
1. Customs classification pipeline:
   - Multi-source recall (CPV priors, FPO, rule-based hints, tariff keyword search)
   - Multi-family expansion to UK 10-digit
   - Exporter/manufacturer resolution
   - Measure interaction engine (AD/CVD/safeguard/quota)
2. Landed cost computation:
   - 11 computation steps
   - Input data requirements per step
   - Conservative default selections
   - Confidence scoring
3. Trade remedy monitoring (daily TRA watcher)
4. Classification freshness system
5. Category whitelist with per-category difficulty ratings

**Key questions:**
- Can landed cost be stubbed in early layers (e.g., return "screening-grade" estimates)?
- What is the minimum customs implementation for the quoteability gate?
- Which API integrations (HMRC Tariff, freight, FX) are required from day 1?
- Can customs classification run as a simplified single-path before the full multi-source recall?

---

## 3. Synthesis Phase

**ONLY AFTER completing all investigations**, produce these unified maps:

### Synthesis 1: Component Dependency Graph

Draw the complete dependency graph between ALL major components. Include:
- Daemon components → which other daemon components they depend on
- Skills → which daemon components they need
- Frontend shells → which data they need
- Sync engine → which tables it operates on

### Synthesis 2: Activation Sequence

For each component/capability, determine its earliest possible activation layer:
- What MUST exist before it can work?
- What can be STUBBED for it to work in a simplified way?
- What ENHANCES it later but isn't required initially?

### Synthesis 3: Data Introduction Sequence

Map the order in which tables/entities must be introduced:
- Foundation tables (executors, sources, workflow_jobs)
- Pipeline tables (raw_documents through landed_cost_estimates)
- Decision tables (operator_decision_heads, bid_workflow_states)
- Sync tables (local_outbox, sync_batches, pull_cursors)
- Projection tables (all `*_summaries` tables)

### Synthesis 4: The Layer Boundary Candidates

Based on the above, identify natural boundary points in the dependency graph where:
- A meaningful end-to-end capability is demonstrated
- The system is independently testable
- Forward compatibility with later layers is maintained

**Starting hypothesis:** The research doc defines 7 milestones (M0-M6 in §11):
- M0: Gold Set + Evaluation Harness (~25-35 files)
- M1: Daemon + single source ingestion
- M2: Full intelligence pipeline (qualify → manifest)
- M3: Dry-run validation (worker-validator)
- M4: Enforcement-mode validation
- M5: Desktop control panel
- M6: Web viewing portal + full sync

These milestones are a reasonable STARTING POINT for layer boundaries. You may refine, merge, split, or reorder them based on your synthesis — but you should explicitly justify any deviation. Pay particular attention to whether M3 and M4 (validation milestones) should remain separate layers or be merged, and whether M5 (desktop) should come earlier to enable visual validation of pipeline output.

Do NOT artificially constrain yourself to a fixed count. The right number of layers is the number that emerges from the dependency graph — typically 5-8 for a system of this complexity.

---

## 4. Critical Context

### What This Project Is

This is a UK public procurement intelligence platform that:

1. **Ingests** official contract notice data (Contracts Finder, Find a Tender) daily using AI agent skills that scrape via browser automation
2. **Qualifies** notices for physical-goods supply relevance using LLM-based analysis
3. **Enriches** qualified notices with supplier pricing (Alibaba, RS Components, etc.) and landed-cost estimates (customs duty, freight, VAT, trade remedies)
4. **Surfaces** high-margin opportunities in a desktop control panel (Tauri) and read-only web portal (Next.js)
5. **Optionally automates** request pipeline workflows (preparation and submission steps — never places real-money bids without human approval)

### Technology Stack

| Layer | Stack |
|-------|-------|
| Daemon | Rust (2024 edition, nightly) |
| Desktop | Tauri v2 + React/Vite |
| Web | Next.js / React |
| Shared packages | TypeScript (Bun) |
| Local DB | SQLite (WAL mode) |
| Remote DB | Supabase (Postgres + PostgREST + RLS) |
| Agent runtime | PTY + Contract mode via adapter boundary |

### Component Architecture

```
┌────────────────────────────────────────────────────────────┐
│                    RUST DAEMON                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐             │
│  │ Scheduler │→│ Job      │→│ Agent Adapter │             │
│  │ (cron)    │  │ Runner   │  │ (PTY/Contract)│             │
│  └──────────┘  └────┬─────┘  └──────┬───────┘             │
│                     │               │                       │
│  ┌──────────┐  ┌────┴─────┐  ┌──────┴───────┐             │
│  │ Resource  │  │ Process  │  │ Artifact     │             │
│  │ Arbiter   │  │ Supervisor│  │ Store        │             │
│  └──────────┘  └──────────┘  └──────────────┘             │
│                                                             │
│  ┌──────────────┐  ┌────────────┐  ┌──────────────┐       │
│  │ Run Manifest  │  │ Validation │  │ Output       │       │
│  │ Writer        │  │ Policy Eng │  │ Quality Gate │       │
│  └──────────────┘  └────────────┘  └──────────────┘       │
│                                                             │
│  ┌──────────────┐  ┌────────────┐                          │
│  │ Sync Engine   │  │ Error      │                          │
│  │ (Push + Pull) │  │ Manifest   │                          │
│  └──────────────┘  └────────────┘                          │
└────────────┬───────────────┬───────────────────────────────┘
             │               │
     ┌───────┴──────┐  ┌────┴─────────┐
     │ SQLite       │  │ Supabase     │
     │ (state.db +  │  │ (Postgres +  │
     │  artifacts)  │  │  PostgREST)  │
     └──────────────┘  └──────────────┘
             │                │
     ┌───────┴──────┐  ┌────┴─────────┐
     │ Tauri Desktop │  │ Next.js Web  │
     │ (Control)     │  │ (View-only)  │
     └──────────────┘  └──────────────┘
```

### Pipeline Flow (15 Stages)

```
ingest → normalize → relate → qualify → acquire_docs → extract_lots
  → extract_products → score_quoteability → price_signal_match
  → gross_margin_gate → landed_cost → net_margin_gate
  → generate_manifest → operator_decision → bid_workflow
```

Each stage has a defined scope grain (process/notice/lot/candidate/candidate_source_pair), a `stage_run_id` as atomic commit cohort, and writes to specific tables.

### Non-Negotiable Architectural Constraints

1. **Authority matrix:** Every table must be classified as push-owned, remote-owned, or local-only. Mixed-write tables are prohibited. This classification MUST be correct from day 1 — reclassifying a table later requires rewriting its sync logic.

2. **Integer pence for money:** All monetary values stored as integer pence (`_pence` suffix). No floating point. No exceptions.

3. **Basis points for confidence:** All confidence/percentage values stored as integer basis points (`_bps` suffix, 0-10000). No floating point.

4. **Skill-first architecture:** Data retrieval happens through agent skills (browser automation), not API connectors. Exception: cost-estimation APIs (HMRC Tariff, freight, FX) use direct API access.

5. **No real-money bids:** The platform automates preparation and submission steps. It does NOT place real-money bids without explicit human approval via operator decision.

6. **Process-group supervision:** Every agent session runs in a process group with mandatory cleanup verdicts (clean/forced/leaked). No orphaned processes.

7. **Run manifests for every skill run:** Every skill execution produces a hybrid-authored run manifest (agent narrative + daemon evidence appendix). If the agent fails to narrate, the daemon synthesizes a fallback machine-authored manifest.

8. **Conservative customs defaults:** When in doubt, use MFN rates, residual duty, out-of-quota, standard VAT, boundary scenarios. Never silently optimize.

### Complete Dependency Graph

```
Skills depend on:
  → Agent Adapter (PTY or Contract mode)
  → Job Runner (job lifecycle management)
  → Process Supervisor (process-group management)
  → Resource Arbiter (slot allocation)
  → Artifact Store (session bytes, run manifests)

Job Runner depends on:
  → SQLite (workflow_jobs table)
  → Scheduler (cron triggers)
  → Resource Arbiter (resource availability)

Sync Engine depends on:
  → SQLite (local_outbox, sync_batches, pull_cursors)
  → Supabase (remote tables)
  → All push-owned tables (reads outbox entries)
  → Schema epoch tracking

Desktop App depends on:
  → SQLite (direct reads via QueryService)
  → Daemon (Tauri commands for triggering operations)
  → Shared UI package (packages/ui)
  → Contracts schema package (packages/contracts-schema)

Web App depends on:
  → Supabase (projection tables via PostgREST + RLS)
  → Shared UI package (packages/ui)
  → Contracts schema package (packages/contracts-schema)
  → Sync Engine having pushed data

Validation depends on:
  → Run Manifest Writer (producing manifests to validate)
  → Validator Skills (category 6)
  → Validation Policy Engine (deciding what to validate)
  → Job Runner (validation sub-jobs)
  → validation_verdicts table (local-only)
```

---

## 5. The Loop Anchor

### The System's Fundamental Operational Cycle

The platform's core value loop is: **Notice arrives → Pipeline processes it → Opportunity surfaced → Operator decides.**

The MINIMUM steps for one complete cycle:

```
1. INGEST: Scrape a procurement source (e.g., Contracts Finder)
   → Creates: raw_documents, contract_notices, procurement_processes
   → Requires: Source skill + PTY adapter + daemon job runner

2. NORMALIZE: Parse raw HTML/JSON into canonical schema
   → Modifies: contract_notices (normalized_payload)
   → Can be inline with ingestion skill or separate step

3. RELATE: Deduplicate and link to procurement processes
   → Creates: process_relations, process_aliases
   → Modifies: contract_notices (process_id)

4. QUALIFY: Determine goods relevance
   → Creates: qualification_runs, notice_lots
   → Requires: LLM-based analysis skill

5. ACQUIRE DOCS: Download tender documents (simplified: can be deferred or
   stubbed as "no documents available" in early layers)
   → Creates: tender_document_sets
   → Requires: Document scraping skill OR can be skipped initially

6. EXTRACT LOTS: Parse lots from notices/documents
   → Creates/modifies: notice_lots (lot details, CPV codes)
   → Can be merged with qualification in early layers (qualification
     already creates synthetic whole-notice lots)

7. EXTRACT PRODUCTS: Identify product candidates from qualified lots
   → Creates: product_candidates
   → Requires: LLM analysis skill

8. SCORE QUOTEABILITY: Assess how quotable each candidate is
   → Modifies: product_candidates (quoteability_score_bps)
   → Can be merged with product extraction in early layers

9. PRICE MATCH: Find supplier pricing
   → Creates: search_profiles, search_attempts, supplier_quotes
   → Requires: Source scraping skills (Alibaba, RS Components, etc.)

10. GROSS MARGIN GATE: Compute gross margins
    → Creates: candidate_pricing_assessments
    → Modifies: pipeline_assessments
    → Daemon computation (no skill needed)

11. LANDED COST: Estimate full import costs
    → Creates: landed_cost_estimates
    → Requires: API calls (HMRC Tariff, freight, FX) + daemon computation

12. NET MARGIN GATE: Compute net margins after landed costs
    → Modifies: pipeline_assessments
    → Daemon computation

13. MANIFEST: Generate opportunity summary
    → Creates: manifests
    → Can be daemon computation or skill

14. PRESENT: Operator sees the opportunity
    → FIRST LAYER: CLI output, test assertion, or file-based manifest is
      ACCEPTABLE. Desktop UI is NOT required for loop completion.
    → LATER LAYERS: Desktop app reads from local SQLite.
    → Desktop UI is an ENHANCEMENT, not a loop prerequisite.

15. DECIDE: Operator approves/rejects/watches
    → FIRST LAYER: Local SQLite record (e.g., a row in
      operator_decision_heads with local-only writes) is ACCEPTABLE.
      Supabase CAS sync is NOT required for loop completion.
    → LATER LAYERS: Desktop writes to Supabase via sync, daemon pulls.
    → Full operator decision flow is an ENHANCEMENT, not a loop prerequisite.
```

**YOUR FIRST USER-FACING LAYER MUST COMPLETE AT LEAST ONE FULL CYCLE.**

This does NOT mean every step must be at production quality. Steps can be simplified or merged:
- Steps 5-6 (acquire docs, extract lots) can be deferred or merged with qualification
- Step 8 (quoteability scoring) can be merged with product extraction
- Landed cost can start as "screening-grade" (simplified, not full multi-source recall)
- Search can start with a single source
- Step 14 (present): CLI/file output is acceptable — desktop UI is NOT required
- Step 15 (decide): Local SQLite write is acceptable — Supabase sync is NOT required
- Validation can be dry-run only or deferred entirely

The pipeline must flow from ingestion through to a verifiable output that proves the complete value chain works — but "verifiable output" means a test, a CLI dump, or a file, NOT necessarily a desktop UI.

**IMPORTANT: The research doc defines Milestone 0 (M0) as "Gold Set + Evaluation Harness" — a validation-first approach where the evaluation infrastructure (gold set of known-good procurement notices, evaluation metrics, test harness) is built BEFORE any pipeline code.** Your Layer 0 should likely correspond to M0. The first PIPELINE layer (Layer 1) then builds on this evaluation infrastructure.

---

## 6. Things That Will Break the Strategy If Missed

### Atomic Operation Pairs (NEVER SPLIT)

1. **Skill execution + run manifest:** Every skill run produces a run manifest. The daemon must capture session bytes AND produce at minimum a fallback manifest. Shipping skills without run manifest infrastructure means losing forensic data.

2. **Job execution + stage status:** Every job that advances the pipeline must update `process_stage_status` and `process_stage_heads`. Shipping jobs without stage tracking means the pipeline state machine becomes inconsistent.

3. **Worker job + validation sub-job:** Linked via `parent_job_id`. The validation policy engine decides whether validation is needed, then the job runner creates a sub-job. Shipping workers without validation infrastructure is fine (fail-open), but the `validation_required` field must exist on `workflow_jobs` from day 1.

4. **Local data mutation + outbox entry (FROM THE LAYER THAT ACTIVATES SYNC):** Once the sync engine is active, every push-owned table write must create a `local_outbox` entry in the same SQLite transaction. **IMPORTANT DISTINCTION:** Before the sync engine layer, tables must still have sync tracking COLUMNS (`owner_id`, `row_version`, `last_local_seq`) populated at write time. Only the outbox ENTRY and the sync ENGINE are deferred — never the schema. This allows local-only operation in early layers without losing the ability to sync later.

### Accumulator Patterns (MUST EXIST FROM FIRST OPERATION)

1. **`row_version` on every push-owned table:** Used by sync to detect conflicts. Must be incremented on every update. Cannot be retrofitted without a schema migration.

2. **`owner_id` on every push-owned table:** Identifies the executor that created the row. Used for push ownership validation. Must exist from first row creation.

3. **`stage_run_id` on pipeline tables:** The atomic commit cohort identifier. Must be populated when pipeline stages write data. Cannot be retrofitted without losing stage coherence.

4. **`last_local_seq` on push-owned tables:** Back-reference to the outbox entry. If populated at write time, sync knows exactly which outbox entry corresponds to which row change.

### Day-1 Safety Requirements

1. **Authority matrix classification:** Every table must be correctly classified before it's created. Changing classification later requires rewriting sync logic for that table.

2. **Integer pence for money:** Introducing floating-point money in an early layer and converting later creates precision issues.

3. **Process-group supervision:** Agent sessions without process-group management risk orphaned browser instances consuming resources indefinitely.

4. **Error manifest system:** Pipeline failures without error manifests are invisible to operators.

### Forward Compatibility Requirements

1. **Schema epoch:** The `schema_epoch` field in the outbox and sync batches allows coordinated schema evolution. Must exist from day 1, even if the epoch never changes in early layers.

2. **Skill metadata:** The `SKILL.md` + schema + canary system must be designed from the first skill, not retrofitted. Otherwise early skills become tech debt.

3. **Sync tracking columns:** All push-owned tables must include `owner_id`, `row_version`, `last_local_seq` from creation, even if sync isn't implemented yet.

4. **Validation fields on workflow_jobs:** `parent_job_id`, `validation_required`, `validation_verdict_id` must exist as nullable columns from day 1, even if validation is disabled.

### Governance Bootstrapping

- MVP is single daemon per org
- Company profile is remote-owned (Supabase) — must be seed-able for first run
- Source configs are remote-owned — must be seed-able or have local fallback
- Operator decisions are remote-owned — desktop writes to Supabase, daemon pulls
- No multi-user, no RBAC in MVP

### Concrete Bad Decomposition Example

**WRONG approach — "One stack per layer":**

```
Layer 1: Daemon scaffold + SQLite schema
Layer 2: Source ingestion skills
Layer 3: Intelligence pipeline skills
Layer 4: Desktop app
Layer 5: Sync engine + Web app
```

**Why this fails:**

- **Layer 1** has no testable behavior — it's pure infrastructure with no user-facing capability
- **Layer 2** produces raw data but there's no way to see if it's correct (no UI, no pipeline processing)
- **Layer 3** depends on Layer 2's output format being stable, but Layer 2 was never validated end-to-end
- **Layer 4** arrives too late — by the time the desktop is built, the daemon's data model may have drifted from what the UI expects
- **Layer 5** discovers sync incompatibilities because tables were designed without sync tracking

**The core problem:** Each layer is "horizontally sliced" — all components at one depth. But the system's value flows VERTICALLY: data enters at the top (ingestion), flows through the middle (pipeline), and exits at the bottom (UI). Horizontal slices never produce a complete vertical flow.

---

## 7. What "Shippable Layer" Means

A shippable layer satisfies ALL of these criteria:

1. **Compiles and runs:** All code compiles, all tests pass, binary is startable
2. **Complete happy path:** At least one end-to-end flow works (not just individual components)
3. **Independently testable:** Can be tested with automated tests that don't require later layers
4. **Maintains all applicable invariants:** Authority matrix, integer pence, basis points, process-group supervision — no invariant is "temporarily relaxed"
5. **Defines stable interfaces:** Later layers extend but don't modify the interfaces introduced in this layer
6. **Clear definition of done:** Concrete, testable acceptance criteria
7. **Produces observable output:** An operator or developer can verify the layer's behavior through some interface (CLI, desktop UI, test output, log inspection)
8. **Forward-compatible schema:** All sync tracking columns present, all nullable forward-reference columns exist

### What a Layer MAY Include

- Simplified implementations (screening-grade landed cost instead of full multi-source recall)
- Single-source support (just Contracts Finder, not all sources)
- Dry-run modes (validation without enforcement)
- CLI-only interfaces (before desktop UI exists)
- Local-only operation (before sync is implemented)

### What a Layer MUST NOT Include

- "Temporary" schema designs that will require migration later
- Relaxed invariants ("we'll enforce authority matrix in the next layer")
- Split atomic pairs (outbox entries without corresponding data writes)
- Placeholder tables with wrong authority classification
- Skills without metadata declarations
- Agent sessions without process-group supervision
- Pipeline stages without stage status tracking

---

## 8. Execution Artifact Policy

### Maintained Doc Set

The maintained end-state is intentionally small. When the strategy is complete, define:

- **One README** (`docs/strategy/README.md`) — overview and navigation
- **One strategy doc** (`docs/strategy/layer-strategy.md`) — rollout order, dependency rationale, sequencing
- **One index doc** (`docs/strategy/cross-reference-index.md`) — cross-layer navigation
- **Exactly N canonical layer docs** (`docs/strategy/layer-{N}-{name}.md`) — execution-grade source of truth per layer

Total: likely 8-12 files.

### Authority Split

- **Strategy doc:** OWNS rollup sequencing, dependency rationale, milestone mapping, layer boundary justifications
- **Layer docs:** OWN execution-grade detail for their layer — components, tables, skills, tests, acceptance criteria
- **Cross-reference index:** OWNS navigation between layers — "if you're looking for X, it's in Layer N §Y"

### Hard Rules

- No standalone cross-layer ledgers (absorb into strategy doc or layer docs)
- No companion docs (absorb into owning layer doc)
- No decision files (decisions live in the layer that addresses them)
- Open questions live inside the owning layer doc under explicit blocker sections

---

## 9. Required Document Structure

### Per-Layer Section Structure

For each layer in the strategy document, include:

1. **Name and narrative:** What this layer IS, in 2-3 sentences
2. **Capability delivered:** What new user-facing capability this layer enables
3. **Components touched:** Daemon components, skills, packages modified/created
4. **Data introduced:** New tables, new columns on existing tables
5. **Authority classification:** Push-owned / remote-owned / local-only for each new table
6. **Shared library additions:** New types in `packages/contracts-schema`, new components in `packages/ui`
7. **Invariants enforced:** Which invariants become active in this layer
8. **What's stubbed:** Simplified implementations that will be enhanced in later layers
9. **Definition of done:** 5-10 concrete, testable acceptance criteria
10. **Test scenarios:** End-to-end tests that prove this layer works
11. **Dependencies:** What must exist from earlier layers
12. **Forward-compatibility provisions:** Schema fields, interface stubs for later layers
13. **Risks:** Layer-specific risks and mitigations
14. **Decisions addressed:** Which ADRs or design decisions are resolved in this layer

### Cross-Layer Sections

After the per-layer sections, include:

1. **Schema stability matrix:** For each table, which layer introduces it, which layers modify it
2. **Interface contracts:** Boundaries between layers that later layers MUST NOT break
3. **Testing evolution:** How testing strategy evolves across layers
4. **Milestone mapping:** How layers map to the existing milestone structure (M0-M6 from §11)
5. **Stack sequencing:** Order of technology introduction (Skills → TypeScript → Rust → Desktop → Web)

---

## 10. Quality Criteria

### Good If

- [ ] A Layer 0 (gold set + evaluation harness) exists as the foundation before pipeline code
- [ ] First pipeline layer (Layer 1) completes the full operational loop with verifiable output (CLI/test/file — desktop UI NOT required)
- [ ] Each layer can answer "what if we never build layer N+1?" and the answer is "the system is still useful"
- [ ] Layer boundaries explicitly map to (or justify deviations from) the research doc's M0-M6 milestones
- [ ] No schema migrations between layers — only forward-compatible additions
- [ ] Authority matrix is correctly assigned from the first table creation
- [ ] Sync tracking columns present on all push-owned tables from day 1
- [ ] Skills have metadata declarations (SKILL.md) from their first layer
- [ ] Validation fields exist on workflow_jobs even before validation is active
- [ ] Landed cost starts as screening-grade and evolves to full multi-source recall
- [ ] Desktop app arrives early enough to validate pipeline output
- [ ] Each layer has at least 5 concrete, testable acceptance criteria
- [ ] Layer boundaries align with natural capability boundaries, not component boundaries
- [ ] The customs classification complexity spectrum is acknowledged (simple → full multi-family expansion)
- [ ] Stack sequencing follows the research doc's recommendation: Skills → TS → Rust → Desktop → Web

### Bad If

- [ ] First pipeline layer is pure infrastructure with no verifiable end-to-end flow
- [ ] Any layer requires modifying a table's authority classification from a previous layer
- [ ] Atomic pairs are split across layers (jobs without stage status, skills without run manifests)
- [ ] More than 2 pipeline layers before the first verifiable end-to-end result
- [ ] Desktop UI is required in the first pipeline layer (CLI/test output is sufficient for loop proof)
- [ ] Supabase sync is required in the first pipeline layer (local-only operation is sufficient initially)
- [ ] Sync engine is Layer 1 (over-engineering — desktop can read SQLite directly)
- [ ] Worker-validator system is Layer 1 (validation is enhancement, not foundation)
- [ ] Customs classification starts at full complexity instead of screening-grade
- [ ] Layers are sliced horizontally by stack (all Rust, then all TypeScript) instead of vertically by capability
- [ ] Layer strategy produces more than 12 documents
- [ ] Any layer has fewer than 3 concrete acceptance criteria
- [ ] M0 (gold set + evaluation harness) is skipped — validation-first philosophy requires it as foundation

---

## 11. Process Instructions

Follow this process strictly:

### Step 1: Research (Parallel Investigations A-H)

Launch subagents for each investigation. Each produces structured output per the investigation specifications above. Do NOT skip any investigation. Investigation G (Gold Set / Evaluation Harness) is particularly important — it defines Layer 0.

### Step 2: Synthesize

Build the 4 unified maps (component dependency graph, activation sequence, data introduction sequence, layer boundary candidates). These maps are your PRIMARY tools for layer design.

### Step 3: Design Layer 0 (Gold Set + Evaluation Harness)

From Investigation G, define Layer 0: the evaluation infrastructure that precedes all pipeline code. This should correspond to the research doc's M0 milestone. Layer 0 is pure test infrastructure — it has no runtime dependencies on the daemon.

### Step 4: Identify the Minimum Loop (Layer 1)

From Synthesis 4, identify the minimum set of components needed for one complete operational cycle: notice ingestion → pipeline processing → verifiable output (test assertion or CLI dump — desktop UI NOT required).

This becomes the core of Layer 1. Everything in this set MUST be in Layer 1. Everything NOT in this set is a candidate for later layers.

### Step 5: Work Outward (Layers 2+)

From Layer 1's minimum loop, extend outward to define subsequent layers. Priority order:
1. **Correctness:** What additional components make the pipeline output MORE CORRECT? (full customs, better matching, multi-source search)
2. **Value:** What capabilities deliver the most user value? (desktop UI, operator decisions, visual validation)
3. **Operational maturity:** What capabilities make the system production-ready? (validation, sync, web portal, monitoring)

Use the research doc's M0-M6 milestones as a starting hypothesis. Justify deviations.

### Step 6: Validate All Layer Boundaries

For each proposed layer boundary, verify:
- No forward references to data not yet introduced
- No invariant gaps (no invariant is "temporarily relaxed")
- No schema migrations needed from previous layers
- Atomic pairs are never split
- Every layer has testable acceptance criteria
- Forward-compatibility provisions are explicit
- Each layer can survive "what if we never build layer N+1?"
- Layer boundaries map to (or explicitly deviate from) M0-M6

### Step 7: Write the Strategy Document

Write `docs/strategy/layer-strategy.md` following the required document structure from §9 above. Include all per-layer sections and all cross-layer sections.

Remember: this document is the single source of truth for the decomposition. It must be comprehensive enough that someone reading only this document (not the 8,400-line research doc) can understand the rationale for every layer boundary.

### Step 8: Fresh-Eyes Pass

After writing the complete strategy document, set it aside mentally and re-read it as if you have never seen the research doc or any prior analysis. Ask yourself these questions and fix any issue you find:

1. **Layer ordering:** Read the layers in sequence from 0 to N. Does each layer build cleanly on top of the previous one with no gaps? Could an implementation team ship Layer 0, then Layer 1, then Layer 2 — each time producing a working system that is strictly more capable than the last?

2. **Forward reference check:** For each layer, scan every component, table, skill, and interface it mentions. Is every one of them either introduced in this layer or already present from a prior layer? If you find a forward reference to something introduced later, either move it earlier or remove the reference.

3. **Schema migration check:** For each table introduced in an earlier layer, verify that no later layer adds columns, renames fields, or changes authority classification. All forward-compatibility columns must exist (nullable or with defaults) from the layer that introduces the table.

4. **Atomic pair check:** Verify that skills always ship with run manifest infrastructure, jobs always ship with stage status tracking, and sync tracking columns always ship with their tables — even if the sync engine itself comes later.

5. **Independence test:** For each layer N, ask: "What if we stop here and never build layer N+1?" The system should still be useful, testable, and stable. If stopping at any layer would leave the system in a broken or meaningless state, that layer boundary is wrong — merge it with the next layer.

6. **Scope balance:** Are any layers disproportionately large or small compared to the others? A layer with 15 new tables, 5 new daemon components, and 4 new skills is probably too large. A layer that only adds a single flag is too small. Rebalance if needed.

7. **Value delivery cadence:** Does user-visible value arrive steadily across layers, or is there a long stretch of infrastructure before anything visible happens? If more than 2 layers pass without something an operator or developer can observe, restructure.

8. **Completeness:** Does every major section of the research doc (§1–§11 + appendices) have a home in at least one layer? Are there orphaned concepts — things mentioned in the research doc that no layer claims? If so, assign them.

9. **Readability:** Could someone who has NOT read the 8,400-line research doc pick up this strategy document and understand what to build first, second, third — with no ambiguity about layer boundaries, no undefined terms, and no missing context? If any section requires the reader to "just know" something from the research doc, add the missing context inline.

Fix every issue you find during this pass. Then read the document one final time to verify your fixes didn't introduce new problems.
