# Handoff: Skill Authoring & Agent Reliability

## Expert Framing

You are an **LLM agent operations engineer** who has designed, deployed, and maintained production agent systems that run hundreds of sessions per day with browser automation. You've built skill/playbook authoring frameworks, debugged agent reliability at scale, designed error recovery patterns for browser-based agents, and managed the operational lifecycle of agent-powered data pipelines. You draw from experience with systems like OpenAI Codex in production, Claude agent deployments, Playwright/Puppeteer-based scraping at scale, and the operational patterns of companies running agent-based workflows in production (not demos).

You understand that the gap between "agent demo works" and "agent runs reliably every day at 6 AM without human intervention" is enormous, and that most of the reliability work lives in skill design, error recovery, and degraded-mode operations — not in the orchestration layer.

---

## Mission

ULTRATHINK and be meticulous.

The procurement intelligence platform runs its entire data pipeline through AI agent sessions — procurement ingestion, supplier price scraping, document acquisition, product extraction, qualification, and bid automation. All via skill-based agents orchestrated by a Rust daemon through adapter-mediated execution (PTY and contract modes).

The daemon orchestration infrastructure is well-designed (reviewed in iteration 000). What's missing is the **skill layer** — how skills are actually authored for reliability, how they handle real-world browser automation failures, how they recover from partial failures, and how the operational lifecycle of skill maintenance works in practice.

Your job: review the skill design patterns, propose concrete reliability improvements, and identify failure modes that the current architecture doesn't address at the skill level.

**Don't just answer my questions — the questions I DON'T KNOW to ask are more important than the ones I do.**

---

## User's Strong Beliefs (Tensions to Engage With)

1. **"Skills are just SKILL.md files — easy to maintain and update."** The plan positions skills as simple markdown playbooks that agents follow. But real browser automation involves complex state management, authentication flows, anti-bot detection, rate limiting, and session persistence. How much complexity is hidden inside these "simple" playbooks? When does a skill need to be more than a markdown file?

2. **"The daemon handles orchestration; skills handle behavior."** Clean separation in theory. But in practice, skills need to communicate intermediate state back to the daemon (checkpoints, partial results, progress indicators). How does the skill-to-daemon contract actually work for error recovery?

3. **"Skill maintenance is just editing SKILL.md when a website changes."** But how do you detect that a website changed before the skill silently produces garbage data? How do you test skills? How do you version-gate skill changes?

---

## Essential Context

### What Skills Do in This System

Skills are the primary data retrieval mechanism. Every piece of data enters the system through a skill:

**Procurement ingestion skills** (3 skills):
- Navigate to Find a Tender, Contracts Finder, PCS
- Extract notice data using cursor/watermark from previous runs
- Output structured OCDS-normalized notice data

**Supplier price enrichment skills** (4 skills):
- Navigate to eBay, Amazon, Alibaba, 1688
- Search for products using query packs from search profiles
- Extract pricing, availability, seller info, shipping details
- Handle authentication flows (especially Alibaba/1688)

**Intelligence pipeline skills** (4 skills):
- Document acquisition: download spec PDFs, BoQ spreadsheets
- Lot extraction: parse unstructured lot descriptions
- Product extraction: convert lot text to structured candidates
- Preliminary qualification: classify ambiguous mixed contracts

**Workflow skills** (2+ skills):
- Manifest generation: synthesize narrative sections
- Bid portal automation: navigate eSourcing portals, fill forms

### Skill Metadata (from §4.10)

Every skill declares:
- `interaction_profile`: `pty_interactive` or `headless_contract`
- `result_schema_version`: e.g., `1.0`
- `auth_requirements`: `browser_session`, `api_key`, `none`
- `browser_required`: `true` / `false`
- `max_concurrency`: e.g., `1`
- `expected_duration`: e.g., `5m`, `30m`

### Execution Model (from §4.9)

1. Daemon creates adapter-mediated session (PTY or contract mode)
2. Skill SKILL.md loaded into agent context
3. Agent follows playbook, producing checkpoints and final result
4. Daemon validates result contract against declared schema
5. Session reaper cleans up processes
6. Raw session output stored as forensic artifact

### Result Contract (from §5.2)

Skills output validated result contracts. Success requires:
- Valid final result contract
- Clean process exit
- Complete cleanup/reap

Partial results with valid checkpoints before a crash are `AGENT_PARTIAL_FAILURE`, never upgraded to success.

---

## Tensions to Resolve

### Tension 1: Skill Authoring Patterns for Reliability

The plan says skills are SKILL.md files with playbook instructions. But what makes a skill reliable vs fragile?

- **Navigation resilience:** Website layouts change. How should skills describe navigation in a way that's robust to minor layout changes? CSS selectors? Semantic descriptions ("click the search button")? Both?
- **Extraction validation:** How does a skill verify it extracted correct data? Just "check the fields are present" or deeper validation (reasonable value ranges, expected patterns)?
- **State checkpointing:** For long procurement ingestion runs (potentially 5-30 minutes), how should the skill emit intermediate checkpoints? What format? How does the daemon know a checkpoint is valid?
- **Error classification within skills:** When a skill encounters an error, how does it communicate the error type to the daemon? Through the result contract? Through structured output? Through exit codes?

### Tension 2: Browser Automation Failure Modes

Supplier marketplace scraping via browser automation faces real-world challenges:

- **Anti-bot detection:** Alibaba, Amazon, and eBay all have bot detection. How do skills handle CAPTCHAs, rate limits, and IP blocks?
- **Authentication persistence:** Alibaba/1688 require login. How is auth state managed across sessions? Per-session browser profiles prevent cookie sharing, but that means re-authenticating every session.
- **Dynamic content loading:** Many marketplace pages load content dynamically (infinite scroll, lazy loading). How do skills handle "wait for data to load" reliably?
- **Geo-restriction:** Some content is geo-restricted. Does the agent need proxy support? VPN?
- **Session state contamination:** The plan says per-session browser profiles. But what about browser fingerprinting that persists beyond cookies?

### Tension 3: Skill Testing and Validation

How do you know a skill works before running it in production?

- **Unit testing skills:** Can you test a skill without running a full agent session? If not, every skill test costs real agent compute.
- **Regression detection:** When a marketplace changes layout, the skill might produce partial or wrong data instead of failing cleanly. How do you detect "skill is producing garbage" vs "skill failed"?
- **Canary runs:** Should new/updated skills go through a validation phase before being used by the cron scheduler?
- **Golden dataset:** Should there be expected output from known inputs to validate skill correctness?

### Tension 4: Partial Failure and Recovery

The daemon classifies partial failure as `AGENT_PARTIAL_FAILURE` with checkpoint preservation. But:

- What should a skill do when it successfully ingests 45 out of 50 notices and then the website errors?
- How does the daemon resume from a partial failure? Restart the skill from scratch? From the last checkpoint?
- For supplier price scraping, if the agent found prices for 3 out of 5 products before failure, should it keep the 3 or retry all 5?
- What's the checkpoint format? How granular should checkpoints be?

### Tension 5: Skill Versioning and Migration

The plan says skills are versioned in the repo and the daemon records which version was used. But:

- When a skill is updated, does it immediately apply to scheduled runs? Or is there a promotion flow?
- If a skill update changes the result schema, how does the daemon handle the transition? Old sessions used schema v1.0, new sessions use v1.1.
- How do you roll back a skill that's producing worse results than the previous version?
- What about the interaction between skill versions and cached results? If the search profile generation logic changes, should cached supplier results be invalidated?

### Tension 6: Cost Efficiency at the Skill Level

The plan targets $20-60/day in agent compute. Skills directly control cost through:

- Session duration (shorter = cheaper)
- Token usage (prompt size, output verbosity)
- Browser actions (fewer page loads = faster)
- Batching strategy (one session per source per notice vs one session per candidate)

What skill-level design patterns minimize cost while maintaining quality?

---

## What I Don't Want (Anti-Patterns)

- **Don't propose replacing skills with traditional web scrapers.** The skill-based agent approach is a core architectural decision (ADR-001, ADR-005). Work within this paradigm.
- **Don't propose a complex skill DSL or framework.** Skills are SKILL.md files. The power comes from the agent's ability to follow instructions, not from a custom runtime.
- **Don't propose mocking agent sessions for testing.** The project explicitly requires real E2E agent tests (§6.5). Mock-based testing was explicitly rejected.
- **Don't propose generic "add retries" advice.** Be specific about what to retry, when, how many times, and what the backoff strategy should be.

---

## What I Do Want

1. **A skill authoring guide.** Concrete patterns for writing reliable SKILL.md files: navigation resilience, extraction validation, checkpoint emission, error communication, and cost efficiency.

2. **Browser automation reliability patterns.** How to handle anti-bot detection, auth persistence, dynamic content, and session isolation in the context of AI-agent-driven browser automation.

3. **A skill testing strategy.** How to validate skills work correctly and detect regressions, within the "real E2E, no mocks" constraint.

4. **Partial failure recovery design.** Concrete checkpoint format, granularity recommendations, and daemon-side resume logic per skill type.

5. **Skill lifecycle management.** Versioning, promotion, rollback, cache invalidation, and schema migration patterns.

6. **Ideas I haven't asked about.** Skill composition (can skills call sub-skills)? Skill observability (what telemetry should skills emit)? Skill marketplace/sharing patterns? Agent model selection per skill (when to use cheap vs expensive models)?

Provide concrete, implementable recommendations. Where you propose changes to the current plan, frame them as modifications to specific sections.

---

## Context Documents to Attach

When sending this handoff to a reviewer, attach:

1. **The full research document** — the reviewer needs §4.9 (Agent Orchestration), §4.10 (Skills and Browser Automation), §5.2 (component designs), §5.6 (Ingestion Data Flow), §5.7 (Intelligence Pipeline Data Flow), §5.11 (Error Handling), §6.5 (Testing Strategy), §6.6 (Observability).
2. **AGENTS.md** — for project constraints and toolchain.

Focus the reviewer's attention on: §4.10 (skill types, metadata, lifecycle, design principles), §5.6 (scheduled ingestion flow with skill execution), §5.11 (error taxonomy), and §6.5 (testing strategy, especially Agent E2E and adapter contract tests).

# UK Public Procurement Intelligence Platform - Research Document

> Generated by idea-to-research skill on 2026-03-08
> Project: Testproject1
> Current Repo State: Greenfield repository with no application code yet
> Recommended Stack: Rust daemon (SQLite local + Supabase/Postgres sync) + TypeScript frontend + skill-based agent scraping

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Intent & Requirements Analysis](#2-intent--requirements-analysis)
3. [Internal Codebase Analysis](#3-internal-codebase-analysis)
4. [External Technology Research](#4-external-technology-research)
5. [Technical Architecture Blueprint](#5-technical-architecture-blueprint)
6. [Implementation Considerations](#6-implementation-considerations)
7. [Architectural Decisions](#7-architectural-decisions)
8. [File Reference Map](#8-file-reference-map)
9. [Dependencies & Prerequisites](#9-dependencies--prerequisites)
10. [Open Questions](#10-open-questions)
11. [Next Steps](#11-next-steps)
12. [Appendices](#appendices)

---

## 1. Executive Summary

### Feature Overview

The product is a procurement intelligence platform for UK public-sector contracts. It should ingest official contract notice data every day, normalize and store it, qualify it for relevance to physical-goods supply opportunities, enrich qualified notices with supplier pricing plus landed-cost estimates, surface high-margin opportunities in the desktop control panel and web viewing portal, and optionally launch request pipeline automation workflows using agent skills and browser automation (the platform automates preparation and submission steps; it does not place real-money bids without human approval).

This is not one system. It is four tightly connected systems:

1. Procurement ingestion.
2. Intelligence enrichment and margin estimation.
3. User-facing review and decision surfaces.
4. Agent-assisted request pipeline automation.

### Key Recommendations

- **Primary data strategy:** Use skill-based agent scraping via browser automation for all procurement and supplier data retrieval. The agent runs on the local machine, powered by the skill system and triggered by the Rust daemon. APIs are reserved for cost-estimation data (customs, freight, FX) where they are more reliable.
- **System of record:** Authority-matrix database architecture — local SQLite as local operational authority (pipeline data, jobs, logs), Supabase/Postgres as remote shared authority (config, operator decisions) and remote projections (viewer-facing pipeline facts). Mixed-write tables prohibited. Push-only pipeline replication via transactional outbox; pull-only shared config/decisions via cursor-based sync.
- **Desktop/runtime architecture:** Use Tauri v2 with a Rust daemon/sidecar for local agent execution.
- **Frontend strategy:** Desktop (Tauri) is the full control panel — trigger workflows, manage agents, make decisions. Web is a read-only viewing portal for browsing data and pipeline status. Both share React UI packages.
- **Cost-estimation engine:** Build an internal landed-cost engine backed by HMRC Trade Tariff, HMRC FPO API for first-pass HS code classification, Taric Support and HS Ping for validation, HMRC Exchange Rate Service for customs valuation FX, Xe for commercial FX, Freightos and SeaRates for freight-market data, and CurrencyBeacon/OANDA plus optional third-party landed-cost services as secondary comparison sources.
- **Agent orchestration:** Use adapter-mediated agent execution with PTY as a first-class runtime from day 1. The Rust daemon triggers Codex/Claude sessions on cron schedules and manual triggers from the desktop control panel, selecting PTY or contract mode per job. PTY-first remains a design constraint for browser-heavy and interactive flows.
- **Automation policy:** The platform automates the request pipeline, not real-money bid submissions. Actual negotiations and fulfillment are handled manually by humans. An error manifest system flags failures with enough context to debug. Human review remains valuable for commercial decisions but risk is low since no real money changes hands automatically.

### Primary Recommendation in One Sentence

Build a greenfield monorepo around skill-based agent scraping for data retrieval, a Rust daemon with local SQLite and Supabase/Postgres sync, adapter-mediated agent orchestration with PTY-first support, and an expanded API-powered internal cost-estimation engine — with the desktop app as the control panel and the web app as a read-only viewing portal.

### Critical Decisions Required

1. Whether MVP scope is England/Wales/Northern Ireland only or full UK coverage including Scotland-specific procurement flows.
2. ~~Whether the first release should optimize for desktop-first operators or equal desktop/web parity.~~ **RESOLVED:** Desktop = control panel (full read/write/trigger), Web = read-only viewing portal.
3. ~~How much autonomy is acceptable in bidding workflows before manual review is mandatory.~~ **RESOLVED:** Full automation of the request pipeline is acceptable — this is not real-money bidding. Human negotiation handles actual commercial decisions.
4. Whether the margin model should default to a VAT-recoverable importer-of-record assumption.
5. ~~Whether Chinese supplier pricing is allowed to rely on browser automation when official APIs are insufficient.~~ **RESOLVED:** Yes — browser automation via agent skills is the primary data retrieval strategy for all procurement and supplier data.

### Quick Stats

| Metric | Value |
|--------|-------|
| Project Type | Greenfield |
| Existing App Files | 0 meaningful application files |
| New Files | ~131 planned files (104 implementation + 27 tests) |
| Modified Files | 0 required beyond bootstrap |
| New Dependencies | ~20-35 across Rust + TS |
| Estimated Effort | XL |
| Estimated Complexity | Very High |
| Highest-Risk Area | Agent/runtime orchestration reliability + product identity/comparability chain (documents → lots → candidates → quoteability → price-signal matching) + sync integrity |

### Recommended Delivery Phases

1. **Phase A:** Procurement ingestion + normalized storage + operator-surface browsing.
2. **Phase B:** Preliminary AI qualification + supplier price enrichment.
3. **Phase C:** Landed-cost engine with customs/logistics and confidence scoring.
4. **Phase D:** Opportunity manifest generation + operator decision flow.
5. **Phase E:** Portal-specific request-pipeline automation skills.

### What Will Make or Break the Product

- Reliable notice ingestion via agent skills is easy compared with supplier-price normalization via agent skills.
- Supplier-price normalization is easy compared with correct product matching.
- Product matching is easy compared with accurate landed-cost estimation at contract scale.
- Landed-cost estimation is easy compared with robust PTY-based agent orchestration that works every day.
- **But the real bottleneck is the identity chain:** documents/lots → candidate identity → quoteability → comparable listing shortlist → price-signal usability. The current architecture compresses this into concrete stages with deterministic gates, structured search profiles, and dual confidence scoring to prevent fake precision from propagating downstream.

This means the platform must preserve confidence scores, raw evidence, full-fidelity agent execution traces (raw PTY output in PTY mode; raw stdout/stderr plus structured protocol events and result artifacts in contract mode), and an error manifest system from day one.

---

## 2. Intent & Requirements Analysis

### 2.1 Problem Statement

**The Problem:**

You want to identify UK public contracts for physical goods where your business can source from lower-cost suppliers, especially China-linked marketplaces or wholesale suppliers, calculate realistic landed cost in GBP, and decide whether the arbitrage margin justifies pursuing the opportunity.

**Why It Matters:**

Without a structured pipeline:

- high-value opportunities remain hidden in noisy procurement feeds,
- manual screening takes too long,
- expiration windows are missed,
- margin calculations are inconsistent,
- bidding effort gets spent on contracts that are not commercially viable.

**Current State:**

There is no implementation yet. The repo is effectively empty, so the research must define the foundational architecture instead of fitting into existing code.

### 2.2 Deep Intent Summary

The user is not asking for a generic contract-search interface. The real target is a **lead-qualification and arbitrage intelligence engine** with these business goals:

1. Pull in public contract opportunities daily.
2. Filter to contracts that are still actionable.
3. Filter further to physical-goods opportunities.
4. Estimate whether UK/EU sourcing is materially more expensive than Chinese sourcing.
5. Include logistics, customs, and FX to reach a true landed margin.
6. Present only the contracts worth human attention.
7. Let an operator reject, review, or trigger request pipeline automation.

### 2.3 User Stories

```gherkin
As a procurement operator
I want new public contract notices ingested automatically
So that I do not rely on manual searching across multiple portals
```

```gherkin
As a sourcing analyst
I want the platform to estimate supplier prices, shipping, duty, VAT, and net margin
So that I can decide whether a contract is commercially viable
```

```gherkin
As a business user
I want a clear manifest showing why a contract is attractive or unattractive
So that I can review evidence instead of raw system output
```

```gherkin
As an operator
I want to press "bid" and trigger a portal-specific workflow
So that repetitive bidding preparation is partially automated
```

**Primary User Stories (Desktop Operator):**

1. As a desktop operator, I want daily ingestion and manual reruns triggered from the desktop control panel so I can trust pipeline freshness.
2. As an analyst, I want only active goods-based contracts surfaced so I do not waste time on services or near-expiry notices.
3. As a decision-maker, I want landed-cost and net-margin estimates in GBP with evidence so I can decide whether to pursue the opportunity.
4. As a desktop operator, I want clear reject/bid controls and auditable history so contract handling is disciplined.
5. As a desktop operator, I want to see an error manifest when pipeline steps fail so I can debug issues quickly.

**Web Viewer Stories:**

1. As a web viewer, I want to browse opportunities, view manifests, and see pipeline status without needing the desktop app installed.
2. As a web viewer, I want to see error status and pipeline health so I understand the current system state.

**Secondary User Stories:**

1. As an administrator, I want skills, prompts, source credentials, and company profile data centrally managed so local agents run consistent workflows.
2. As a compliance-aware operator, I want confidence scores and human-override points so I know when the system is uncertain.
3. As an operator, I want comprehensive agent tracing logs including full session output so I can trace exactly where errors come from.

### 2.4 Success Criteria

**Functional Criteria:**

- [ ] Daily ingestion pulls all relevant notices from selected official sources with cursor/date tracking.
- [ ] Contracts expiring within 30 days are automatically excluded from active qualified views.
- [ ] Goods-vs-services classification is available for every normalized contract with evidence and confidence.
- [ ] Supplier-price enrichment stores at least one UK/EU and one China-oriented sourcing estimate for qualified goods contracts where feasible.
- [ ] Landed-cost calculation includes FX, customs duty, import VAT logic, logistics estimate, and confidence metadata.
- [ ] Contracts are only highlighted as final qualified opportunities when estimated net margin exceeds 20% and the estimate confidence clears the review threshold.
- [ ] Every final opportunity has a readable Markdown manifest stored as an artifact.
- [ ] Operators can reject, watch, or start a bid workflow from the desktop control panel.

**Non-Functional Criteria:**

- [ ] Freshness: due ingestion jobs are triggered by the daemon's local cron scheduler within 24 hours of publication cadence, and completed within SLA while the daemon is online.
- [ ] Reliability: job execution is idempotent and stored in local SQLite with sync to Supabase.
- [ ] Explainability: every automated decision stores evidence, source URLs, timestamps, and model confidence.
- [ ] Security: secret-bearing actions run in the daemon, not in the browser.
- [ ] Observability: every ingestion, enrichment, and bidding run is traceable.

### 2.5 Scope Definition

**MVP Scope (Must Have):**

- Official-source procurement ingestion for selected UK sources.
- Normalized notice + contract storage.
- Preliminary qualification by expiry and goods relevance.
- Supplier-price enrichment from selected sources.
- Landed-cost engine with shipping/customs/FX.
- Desktop control panel + web viewing portal for browse/search/review.
- Manifest generation.
- Manual and scheduled execution.
- Portal-specific bid workflow framework, with at least one example implementation.

**Phase 2 (Should Have):**

- More procurement source scraping skills.
- More supplier marketplace scraping skills.
- Better product matching using embeddings + structured attributes.
- Freight broker quote escalation when confidence is low.
- Multi-user collaboration and task assignment (via Supabase sync).

**Future Consideration (Could Have):**

- Cloud fallback execution when the local daemon is offline.
- Automated RFQ emails/messages to suppliers.
- Historical win-rate and bid-quality analytics.
- Portfolio optimization across contracts.
- Inventory-aware sourcing recommendations.

### 2.6 Non-Goals & Exclusions

**Explicitly Out of Scope for MVP:**

- Fully autonomous bid submission for all portals.
- Replacing customs brokers or freight forwarders in legally sensitive cases.
- Full ERP/accounting integration.
- Guaranteed exact landed-cost precision for every SKU without human verification.
- Unsupported blanket scraping of marketplaces that prohibit or technically block it.

**Deferred:**

- Invoicing and fulfillment operations after a bid is accepted.
- Full tender drafting beyond structured pre-bid submission support.
- Broad non-UK procurement expansion.

### 2.7 Constraints

- Procurement data is heterogeneous across portals and legal regimes.
- China-oriented supplier marketplaces require skill-based browser scraping; structured data access is limited.
- Customs classification accuracy is business-critical and sometimes legally sensitive.
- Freight estimation depends on shipment mode, Incoterms, density, route, seasonality, and carrier conditions.
- Local daemon execution introduces availability and concurrency concerns.
- Tauri desktop constraints mean server-style Next.js patterns should be avoided in the desktop shell.

### 2.8 Key Ambiguities Identified

| Ambiguity | Why It Matters | Working Assumption |
|-----------|---------------|--------------------|
| Full UK or only non-Scottish procurement in MVP | Source skills and legal variation differ | Start with England/Wales/NI plus optional PCS scraping skill |
| Exact supplier sources in China | Marketplace access and ToS differ sharply | Skill-based agent scraping via browser automation is the primary approach |
| Web and desktop parity | Affects frontend architecture | **RESOLVED:** Desktop = control panel, Web = read-only viewing portal |
| Fully automatic bidding or semi-automatic | Major risk and compliance impact | **RESOLVED:** Full automation of request pipeline; human negotiation handles commercial decisions |
| Use of pure local compute | Affects reliability of scheduled ingestion | **RESOLVED:** Local-first with Rust daemon, SQLite, PTY agent execution. Supabase for shared sync. |

### 2.9 Research Direction

Based on the intent analysis, the most important research areas were:

1. Official UK procurement data access because ingestion quality is foundational.
2. Customs and freight estimation because margin decisions depend on landed cost, not ex-works price.
3. Supplier pricing retrieval and source constraints because China-side sourcing intelligence is the noisiest part of the system and relies on skill-based scraping.
4. Desktop/daemon/shared-database architecture because the user explicitly wants local agent execution with centralized storage.
5. Skills and agent CLI orchestration because this is core to the desired workflow model.

---

## 3. Internal Codebase Analysis

### 3.1 Architecture Overview

**Project Type:** Greenfield repository

**Current Tech Stack:**

- Language: None established
- Framework: None established
- Database: None established
- Build: None established
- Test: None established

**Observed Repository State:**

```text
/Users/alinaaitova/Desktop/Testproject1
├── .git/
└── yolo                # placeholder file with "test"
```

### 3.2 Current Reality

There is no meaningful existing codebase to integrate with. That changes the purpose of this section:

- there are no current patterns to reuse,
- there are no integration points to preserve,
- there is no legacy data model to migrate,
- there are no tests to mirror,
- there is no runtime to extend.

This research document therefore acts as a **foundational architecture brief** for implementation, not as an extension design for an existing application.

### 3.3 Files Present Today

#### File: `yolo`
**Location:** `/Users/alinaaitova/Desktop/Testproject1/yolo`

**Relevance:** None to the target application. Appears to be a placeholder file.

**Recommendation:** Ignore or remove during bootstrap.

### 3.4 Proposed Greenfield Project Shape

Because the repo is empty, the most valuable internal design output is a proposed structure:

```text
Testproject1/
├── apps/
│   ├── desktop/                 # Tauri desktop shell — CONTROL PANEL (full read/write/trigger)
│   ├── web/                     # hosted viewing portal — READ-ONLY
│   └── api/                     # optional thin hosted API for privileged operations
├── crates/
│   ├── daemon/                  # Rust daemon: PTY agent orchestration, SQLite storage,
│   │                            #   cron scheduling, Supabase sync, log capture
│   └── common/                  # Rust shared process/runtime utilities if needed
├── packages/
│   ├── contracts-schema/        # shared TS domain types / zod schemas
│   ├── db/                      # SQL migrations (Supabase), typed queries, generated types
│   ├── intelligence/            # cost-estimation APIs: HMRC tariff, freight, FX integration
│   ├── manifests/               # Markdown manifest generation
│   ├── bidding/                 # bid workflow orchestration definitions
│   ├── ui/                      # shared React components
│   └── config/                  # source configs, env parsing, feature flags
├── skills/
│   ├── procurement-ingestion/   # agent scraping skills for procurement sources
│   │   ├── fts/                 # Find a Tender scraping skill
│   │   ├── contracts-finder/    # Contracts Finder scraping skill
│   │   └── pcs/                 # Public Contracts Scotland scraping skill
│   ├── supplier-price-enrichment/
│   │   ├── ebay/                # eBay UK browser scraping skill
│   │   ├── amazon/              # Amazon UK browser scraping skill
│   │   ├── alibaba/             # Alibaba.com browser scraping skill
│   │   └── 1688/                # 1688 wholesale browser scraping skill
│   ├── preliminary-qualification/
│   ├── manifest-generation/
│   └── bid-portal-*/            # portal-specific bid automation skills
├── docs/
│   ├── research/
│   ├── specs/
│   └── runbooks/
└── infra/
    ├── supabase/                # Supabase config for shared sync target
    ├── monitoring/
    └── ci/
```

The primary implementation path in this document assumes **skill-based agent scraping for data retrieval** (procurement notices and supplier pricing), **API-based integration for cost estimation** (customs, freight, FX), and a **Rust daemon for PTY-based agent orchestration with local SQLite storage and Supabase sync**. The `packages/connectors/` directory from the original design is replaced by the skills system — each data source has a corresponding skill that the agent uses to scrape it via browser automation.

Only procurement/supplier retrieval and workflow playbooks live under `skills/`. The landed-cost engine itself remains in `packages/intelligence` because cost estimation is API-based and deterministic at its core, not an agent-scraping workflow.

### 3.5 Code Patterns to Follow

Because no project code exists, patterns should be created deliberately instead of discovered accidentally.

#### Pattern: Raw payload retention

Every external fetch should store:

- source name,
- endpoint or page URL,
- fetched timestamp,
- raw response payload or storage pointer,
- normalized projection,
- parse confidence and parse version.

This is essential because procurement notices change, supplier prices change, and AI decisions need replayability.

#### Pattern: Confidence-scored enrichment

Every enrichment result should store:

- value,
- method,
- evidence,
- confidence score,
- expiration/TTL,
- manual override fields.

If this is not built into the schema early, the platform will drift into fake precision.

#### Pattern: Orchestration state separate from agent output

Agent skills and manifests should be artifacts. The authoritative record for job status, retries, leases, and workflow transitions should stay in database tables. Name the queue generically, because most jobs are workflow jobs, not agent runs.

#### Pattern: Deterministic fallbacks

Use rule-based gating before AI:

- expiry date rule,
- procurement category rule,
- presence/absence of quantity/unit,
- duty/VAT calculation formulas,
- FX snapshot rules.

Reserve AI primarily for:

- goods vs services classification in ambiguous notices,
- product matching,
- extracting structured line items from messy text,
- generating human-readable manifests,
- browser interaction for bid flows.

### 3.6 Files to Study Before Implementation

There are no current implementation files to study. The practical equivalent is:

#### Must Read (Critical Context)

| File | Purpose | Key Sections |
|------|---------|--------------|
| `/Users/alinaaitova/Desktop/Testproject1/docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` | Foundational system design | All |

#### Should Read (Important Context)

| File | Purpose |
|------|---------|
| `/Users/alinaaitova/.codex/skills/idea-to-research/SKILL.md` | Defines expected research depth and handoff quality |

### 3.7 Files to Modify

#### Definite Modifications

| File | Changes Needed | Impact |
|------|----------------|--------|
| `/Users/alinaaitova/Desktop/Testproject1/yolo` | Ignore during bootstrap; only delete during explicit repo cleanup | Low |

#### Likely Bootstrap Files

| File | Probable Changes | Confidence |
|------|------------------|------------|
| `/Users/alinaaitova/Desktop/Testproject1/package.json` | Workspace/bootstrap setup | High |
| `/Users/alinaaitova/Desktop/Testproject1/bunfig.toml` | Bun workspace/runtime settings | High |
| `/Users/alinaaitova/Desktop/Testproject1/Cargo.toml` | Rust workspace definition | High |

### 3.8 Potential Refactoring Needs

There is no technical debt in the app code because there is no app code yet. The real risk is different:

- if bootstrap is done without a clean package layout, refactoring pain will arrive immediately,
- if Supabase schema and RLS are improvised later, security and agent-write paths will become messy,
- if skills are written before workflow boundaries are modeled, the agent layer will become the architecture by accident.

---

## 4. External Technology Research

Scores in this section are rough decision aids, not statistically calibrated models. Unless otherwise stated, the total shown for each option is a simple equal-weight average of the listed criteria. Small score gaps should not be over-read; differences of a few tenths are directional, not mathematically significant.

### 4.1 Procurement Notice Ingestion Sources

#### Need

Reliable daily access to UK public procurement notices. Data retrieval is performed by skill-based agent scraping via browser automation, running on the local machine and orchestrated by the Rust daemon. The agent navigates to the official procurement sources and extracts notice data using source-specific skills.

#### Option A: Find a Tender OCDS API

**Official evidence:**

- Find a Tender publishes OCDS record and release package APIs.
- Release package endpoint supports `limit`, `cursor`, `updatedFrom`, `updatedTo`, and stage filters.
- Data is available under the Open Government Licence.

**Key references:**

- https://www.find-tender.service.gov.uk/Developer/Documentation
- https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsReleasePackages
- https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsRecordPackages

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Data structure | 5 | OCDS-native |
| Freshness support | 5 | `updatedFrom`/`updatedTo` + cursor |
| Licensing clarity | 5 | Open Government Licence |
| Coverage | 4 | High-value and new-regime notices; not whole UK on its own |
| Operational reliability | 4 | Official API, structured docs |
| **Average Score** | **4.6** | |

**Pros:**

- Official, structured, versioned API.
- Maps notice data to OCDS 1.1.5 with extensions.
- Supports incremental polling.
- Central post-2025 procurement regime source for England, Wales, Northern Ireland high-value flows.

**Cons:**

- Does not fully cover all below-threshold or Scotland-specific notice ecosystems by itself.
- Historical/transitional notices can span legacy platforms.

**Recommendation:** **RECOMMENDED**

Use as a primary ingestion target for the procurement-ingestion agent skill. The agent navigates to the Find a Tender API/portal and extracts notice data using the skill-defined playbook.

#### Option B: Contracts Finder OCDS/Search API

**Official evidence:**

- Contracts Finder exposes API help docs.
- Published/OCDS search supports `publishedFrom`, `publishedTo`, `stages`, `limit`, `cursor`.
- Daily CSV harvester endpoint exists and explicitly mentions rate limiting.
- Contracts Finder uses OCDS in its REST API and publishes data under government open standards.

**Key references:**

- https://www.contractsfinder.service.gov.uk/apidocumentation
- https://www.contractsfinder.service.gov.uk/apidocumentation/Notices/1/GET-Published-Notice-OCDS-Search
- https://www.contractsfinder.service.gov.uk/apidocumentation/Notices/1/GET-Harvester-Notices-Data-CSV-Daily
- https://www.gov.uk/government/publications/open-standards-for-government/open-contracting-data-standard-profile

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Data structure | 4 | OCDS available, plus CSV |
| Freshness support | 4 | date filters + cursor + daily export |
| Licensing clarity | 4 | aligned with open contracting/publication standards |
| Coverage | 4 | lower-value/public opportunities complement FTS |
| Operational reliability | 4 | official docs; rate limits exist |
| **Average Score** | **4.0** | |

**Pros:**

- Strong complement to Find a Tender.
- Supports daily and filtered retrieval.
- Useful for below-threshold and broader discovery.

**Cons:**

- API docs still reflect legacy evolution and mixed versions.
- Need rate-limit-aware polling.

**Recommendation:** **RECOMMENDED**

Use as a primary ingestion target for a dedicated agent skill alongside Find a Tender.

#### Option C: Public Contracts Scotland OCDS API

**Official evidence:**

- PCS exposes an OCDS Web API with notice list and notice family retrieval.
- Query supports output type, notice type, and date windows.

**Key references:**

- https://api.publiccontractsscotland.gov.uk/v1
- https://www.publiccontractsscotland.gov.uk/Default.aspx

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Data structure | 4 | OCDS available |
| Freshness support | 3 | month-based query model is weaker than FTS cursoring |
| Licensing clarity | 3 | official public API, but licensing should be re-confirmed in implementation |
| Coverage | 5 | essential for Scotland coverage |
| Operational reliability | 4 | official endpoint |
| **Average Score** | **3.8** | |

**Pros:**

- Required for true UK coverage.
- Official API rather than HTML scraping.

**Cons:**

- Separate legal/process ecosystem.
- Weaker incremental retrieval ergonomics than FTS.

**Recommendation:** **RECOMMENDED FOR FULL-UK SCOPE**

Add in MVP only if Scotland is in initial scope. Otherwise make it Phase B.

#### Option D: HTML scraping of notice portals

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Data structure | 1 | brittle |
| Freshness support | 2 | possible, but operationally noisy |
| Licensing clarity | 2 | page terms vary |
| Coverage | 3 | helpful only where API is absent |
| Operational reliability | 1 | selectors drift |
| **Average Score** | **1.8** | |

**Recommendation:** **SUBSUMED BY SKILL-BASED APPROACH**

With skill-based agent scraping as the primary architecture, HTML scraping is inherently handled by the agent's browser automation. Each source gets a dedicated skill rather than a generic HTML scraper.

#### Procurement Ingestion Recommendation

**Selected approach: Skill-based agent scraping**

1. `Find a Tender` ingestion skill — agent navigates FTS API/portal via browser automation.
2. `Contracts Finder` ingestion skill — agent navigates CF API/portal via browser automation.
3. Optional `Public Contracts Scotland` ingestion skill if full UK coverage is in scope.

All procurement data retrieval is constrained to the skill-based agent system running on the local machine. This simplifies the architecture: all data retrieval is based on browser-enabled agents running locally, powered by skills (easier to maintain, upgrade, and change) instead of maintaining separate API client packages for each data source.

**Ingestion is not the hard problem.** The official sources already expose structured incremental retrieval: FTS has OCDS release/record packages with `limit`, `cursor`, `updatedFrom`, `updatedTo`, and stage filters under the Open Government Licence; Contracts Finder similarly exposes `publishedFrom`, `publishedTo`, `stages`, `limit`, and `cursor` with documented rate limiting. Ingestion should be source-batched and incremental — one agent session per source per run, not one session per notice. The architecture's real complexity lives downstream in product identity, comparability, and pricing.

**Important architectural consequence:**

Each procurement source has a dedicated skill that defines the agent's scraping playbook. The daemon orchestrates execution via adapter-mediated agent sessions, which are typically PTY-backed for procurement retrieval. The underlying platform still provides:

- source registry (which skills to run and when),
- cursor/watermark tracking (persisted in local SQLite),
- raw payload retention,
- OCDS normalization,
- relation resolution and merge rules across sources.

This retrieval strategy is formalized in **ADR-001** and operationalized through the skill/runtime boundary in **ADR-005**.

### 4.2 Procurement Rules and Notice Semantics

#### What changed recently

Find a Tender states that for procurement processes starting on or after **24 February 2025**, notices specified by the Procurement Act 2023 apply, with transitional treatment for earlier processes and jurisdiction-specific exceptions.

**Official references:**

- https://www.find-tender.service.gov.uk/Home/NoticeTypes
- https://www.gov.uk/government/publications/procurement-act-2023-short-guides/central-digital-platform-factsheet-html
- https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-procure-phase/guidance-central-digital-platform-and-publication-of-information-html

#### Why this matters for the product

- Notice types changed after 24 February 2025.
- Expiry logic cannot depend on one legacy field name.
- Contract lifecycle stages are richer than “tender open” vs “closed”.
- Transitional notices may continue under earlier legislation.

#### Notice Relation Semantics

FTS's notice type documentation shows that linked notices share an OCID, but the relationship between notices is not always “same process → merge.” Specifically:

- **Linked notices (same OCID):** Updates, amendments, and award notices within the same procurement process share an OCID and should be merged into the same `ProcurementProcess` record.
- **Framework awards:** Framework establishment and subsequent call-offs follow different notice sequences. A framework award does not include a UK4 tender notice; the sequence differs from open-tender flows. Call-offs under an established framework are commercially separate opportunities even though they reference the parent framework.
- **Direct award after unsuccessful competition:** FTS explicitly treats this as a separate process with a different OCID. This is not a merge candidate — it is a genuinely new procurement process.
- **Related but separate:** A planning notice and a later tender notice from the same buyer for the same subject may have different OCIDs. These are related but not the same process.

This means the platform needs four persisted relation classifications, not two:

1. **`merged_same_process`** — same OCID or explicit linked notice chain.
2. **`related_not_merged`** — framework/call-off, planning/tender, or same-buyer same-subject but different OCIDs.
3. **`possible_duplicate`** — high-similarity pair held for operator review instead of forced merge.
4. **`separate`** — no meaningful relationship.

See §5.6 for the relation resolution algorithm.

#### Access Route Classification

Not all procurement notices represent actionable opportunities. The platform should classify each notice's access route early — before pricing — to suppress non-bidable opportunities:

| Access Route | Description | Actionability |
|---|---|---|
| `open_tender` | Open competitive procedure; any qualifying supplier can bid | Fully actionable |
| `restricted_tender` | Restricted procedure; requires pre-qualification/shortlisting | Actionable if pre-qualification route is open |
| `framework_call_off` | Call-off under an established framework | Actionable only if you are on the framework |
| `dynamic_market` | Dynamic Purchasing System / Dynamic Market (Procurement Act 2023) | Actionable if you can join the DPS/DM |
| `direct_award` | Direct award without competition | Not actionable |
| `closed_framework_award` | Award under closed framework | Not actionable unless on framework |
| `not_bidable` | Notice type does not support supplier participation | Not actionable |

**Important:** DPS/Dynamic Market notices do not “expire” in the same way as standard tenders — they are open-ended. The 30-day expiry filter should not apply to DPS notices that accept new supplier applications.

#### Lot Structure in Notices

Many UK procurement notices contain multiple lots. A lot — not the notice — is the unit that determines:

- bidability (lots may have different access routes),
- CPV specificity (lots often have more specific CPV codes than the notice-level code),
- value (each lot has its own estimated value),
- deadline (lots may have different submission deadlines),
- technical scope (goods vs services vs works may differ per lot),
- whether goods are separable from services/works.

The platform must extract lots from notices and treat each lot as a separate qualification and pricing unit. If a notice has no explicit lots, create a synthetic whole-notice lot. See §5.4 for the `NoticeLot` entity and §5.7 for lot extraction in the intelligence pipeline.

#### Document Attachments

In UK procurement, the notice body is often the **least useful product document**. The real signal for product extraction lives in:

- specification PDFs,
- pricing schedules,
- lot documents,
- bills of quantities (BoQ) spreadsheets,
- tender appendices.

The platform should attempt to acquire publicly accessible tender documents before product extraction. Without this, the extractor will turn vague notice summaries into garbage product candidates. See §5.7 for the document acquisition stage.

#### Recommendation

Normalize notice lifecycle into internal stages:

- `pipeline`
- `planning`
- `tender_open`
- `award`
- `contract`
- `termination`

Then derive operator-facing eligibility rules from normalized state, not from raw portal-specific labels. Classify access route early in the pipeline to suppress non-actionable notices before pricing.

### 4.3 Supplier Price Intelligence Sources

#### Need

Retrieve or infer comparable market prices for goods in:

- UK/EU-facing marketplaces,
- China-oriented supplier marketplaces,
- using skill-based agent scraping via browser automation on the local machine.

All supplier pricing data retrieval uses the same skill-based agent architecture as procurement ingestion. The agent navigates to marketplace websites using source-specific skills and extracts pricing data. No API client packages are maintained for supplier sources.

#### Category A: eBay Browse API

**Official evidence:**

- Browse API supports keyword, GTIN, category, and item search.
- Item detail responses include price, shipping, seller, availability, and estimated delivery.
- Release notes show ongoing maintenance and added shipping-estimate support in 2025.

**References:**

- https://developer.ebay.com/api-docs/buy/static/api-browse.html
- https://developer.ebay.com/api-docs/buy/browse/static/overview.html
- https://developer.ebay.com/api-docs/buy/browse/resources/item/methods/getItem
- https://developer.ebay.com/api-docs/buy/browse/release-notes.html

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Search quality | 4 | keyword + category + GTIN |
| Structured pricing | 4 | strong item detail support |
| Maintenance | 5 | active release notes |
| UK relevance | 4 | marketplace support is strong |
| Integration risk | 3 | some APIs are partner-gated depending on use case |
| **Average Score** | **4.0** | |

**Pros:**

- Strong structured API surface.
- Good fit for spot price baselines and resale-market checks.
- Rich item metadata.

**Cons:**

- Not ideal for wholesale benchmarking.
- Marketplace/partner approval details must be validated during implementation.

**Recommendation:** **RECOMMENDED**

Use for UK retail/resale pricing baselines via agent skill-based browser scraping.

#### Category B: Amazon Product Advertising API / Creators API Transition

**Official evidence:**

- Amazon states Product Advertising API 5.0 will be deprecated on **30 April 2026**.
- The documentation is explicitly marked outdated.
- New customers are directed to Creators API.
- Amazon updated Associates terms in late 2025 to include Creators API and new agent-related terms.

**References:**

- https://webservices.amazon.com/paapi5/documentation/
- https://webservices.amazon.com/paapi5/documentation/register-for-pa-api.html
- https://affiliate-program.amazon.com/help/operating/compare

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Search quality | 4 | historically strong |
| Structured pricing | 4 | strong catalog detail |
| Maintenance | 2 | PA-API sunsets on 2026-04-30 |
| UK relevance | 5 | very high |
| Integration risk | 2 | migration and policy risk |
| **Average Score** | **3.4** | |

**Pros:**

- Amazon pricing is commercially useful for UK baselines.
- Historically mature API behavior.

**Cons:**

- PA-API is effectively legacy now.
- New integrations should not be designed around a surface that ends on 30 April 2026.
- Access is tied to Amazon affiliate ecosystem rules and changing API terms.

**Recommendation:** **USE WITH TRANSITION PLAN**

Amazon pricing can be collected via agent skill-based browser scraping, avoiding the PA-API sunset risk entirely. If official API access is desirable later, isolate behind a generic skill interface.

#### Category C: Taobao Global Open Platform / Alibaba Open APIs / 1688 APIs

**Official evidence:**

- Taobao Global Open Platform exists and exposes seller/reseller-oriented APIs for products, orders, logistics, and settlements.
- Alibaba.com Open API exists for product/order synchronization.
- 1688-related APIs exist in the Taobao/Alibaba ecosystem, but they are auth-heavy and oriented toward distributor/seller workflows.

**References:**

- https://open.taobao.global/doc/doc.htm
- https://open.taobao.global/doc/api.htm
- https://developer.alibaba.com/docs/doc.htm?articleId=118416&docType=1&treeId=684
- https://open.alitrip.com/docs/api.htm?apiId=29432

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Search quality | 2 | not a clean public catalog search solution for arbitrary procurement matching |
| Structured pricing | 3 | available in some scenarios, but partner/auth complexity is high |
| Maintenance | 4 | official platform docs exist |
| UK relevance | 4 | strong for China sourcing |
| Integration risk | 1 | high auth, partner, language, and workflow complexity |
| **Average Score** | **2.8** | |

**Pros:**

- Official channels exist.
- Better long-term legitimacy than raw scraping.
- Strong fit for supplier-side workflows when access is granted.

**Cons:**

- Not a plug-and-play public price intelligence API.
- Many endpoints assume seller/distributor relationships or credentials.
- Operational complexity is high.

**Recommendation:** **SKILL-BASED SCRAPING**

Chinese sourcing intelligence is collected via agent skill-based browser scraping:

1. Agent navigates to Alibaba/1688/Taobao using dedicated skills.
2. Skills handle authentication flows where needed.
3. All captured data includes confidence scoring and sourcing method.

The skill-based approach eliminates the need to manage complex API auth flows — the agent handles website interactions just as a human would, guided by the skill playbook.

#### Supplier Price Intelligence Recommendation

Use a **skill-based supplier-source model**:

1. UK sources: eBay, Amazon — dedicated browser scraping skills.
2. China sources: Alibaba, 1688 — dedicated browser scraping skills.
3. All collection via agent skills with browser automation on the local machine.

Do not flatten these into one “price” field. Every quote needs:

- source,
- region,
- quote class,
- quoted quantity and unit,
- quantity basis,
- unit assumptions,
- MOQ assumptions,
- shipping included or excluded,
- listing pack quantity and unit (the listing's native basis),
- normalized unit basis (the common basis used for comparison),
- VAT-inclusive flag,
- used/refurbished flag,
- confidence (split: see below),
- validity window.

**Price-signal role classification:** Marketplace listings serve different roles depending on source:

| Source | Typical Price-Signal Role | Implication |
|---|---|---|
| Amazon/eBay UK | Benchmark ceiling — UK retail/resale price, usually VAT-inclusive | Establishes the contract-revenue proxy or upper-bound UK market price |
| Alibaba | Optimistic source-side floor — listed MOQ prices, often ex-works/FOB | Lower bound for supplier cost; actual procurement-scale price will differ |
| 1688 | Domestic China wholesale floor — even more optimistic | Useful for commodity verification but not directly executable from UK |
| RFQ/negotiated | Executable or near-executable quote | Decision-grade pricing |

The system should compute a **margin band** (floor to ceiling) rather than fake precision from one UK listing and one China listing.

**Confidence split:** Every `SupplierQuote` should carry two separate confidence scores:

- `matchConfidence` — how well the marketplace listing matches the product candidate (product identity).
- `quoteUsabilityConfidence` — how commercially usable the observed price is (correct unit basis, relevant MOQ, shipping visibility, listing completeness).

A listing can be a strong product match and still be a weak pricing signal (right product, wrong pack basis), and vice versa (usable market price range, but only a near-match). These should not be averaged for control flow.

**Source routing by candidate type:** Not every product candidate should be searched on every source. Route by `candidateType` (see §5.7):

| Candidate Type | UK Sources | China Sources | Notes |
|---|---|---|---|
| `exact_sku` | eBay, Amazon (primary) | Only if “equivalent” allowed and no OEM lock | SKU/GTIN search |
| `standard_commodity` | eBay, Amazon | Alibaba, 1688 | Full search |
| `regulated_goods` | eBay, Amazon | Alibaba, 1688 only when standards/compliance slots populated | Must verify compliance equivalence |
| `brand_family` | eBay, Amazon | Alibaba only if generic equivalent exists | Brand-specific search |
| `bundle` | Decompose first | Decompose first | Search top 2–3 sub-items only |
| `custom_or_fabricated` | Skip marketplaces | Skip marketplaces | RFQ/manual only |
| `installed_goods` | Skip marketplaces | Skip marketplaces | RFQ/manual only |

This routing stops the search explosion: instead of 4 sources × N candidates, the system only hits sources where results are meaningful.

**Cross-language matching strategy:** Do not translate whole procurement sentences into Chinese and search them. Instead:

1. Normalize the product candidate into structured slots (head noun, critical attributes, standards, numeric specs).
2. Expand English synonyms for marketplace search terms.
3. Translate only the head noun + critical modifiers + standards-relevant descriptors to Chinese.
4. Keep numerics and standards codes unchanged (EN 397, EN 455, dimensions, voltages are universal anchors).
5. Run dual-language queries for China sources: English search first on Alibaba (broad recall), Chinese search on 1688 (where listings are primarily in Chinese).

The bilingual layer is a small curated lexicon per CPV/product family — English marketplace aliases, Chinese marketplace aliases, critical attributes, exclusion vocabulary. This is a search aid, not a taxonomy.

For large tenders, marketplace listings should usually be treated as **indicative signals**, not executable procurement offers. Once an opportunity survives early filtering, the system should escalate toward RFQs, negotiated quotes, or supplier-confirmed volume pricing before treating margin as decision-grade.

### 4.4 Shipping and Logistics Estimation

#### Need

Estimate transport cost from origin to UK with different shipment modes:

- parcel/courier,
- air freight,
- LCL ocean,
- FCL ocean,
- domestic final-mile or pallet delivery.

#### Option A: Freightos APIs / Freightos Terminal

**Official evidence:**

- Freightos API exposes price statistics endpoints for route-level freight data.
- Freightos documentation highlights route, mode, load, and confidence levels.

**References:**

- https://developers.freightos.com/apis
- https://apidocs.freightos.com/reference/get_price-stats
- https://apidocs.freightos.com/reference/getpricestats-1

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Freight relevance | 5 | designed for freight, not just parcel labels |
| Route intelligence | 5 | route-based pricing and confidence |
| Parcel coverage | 2 | not the main use case |
| Bulk-contract usefulness | 5 | very strong |
| Integration risk | 3 | commercial access and quota considerations |
| **Average Score** | **4.2** | |

**Pros:**

- Best fit for large-volume China-to-UK freight estimation.
- Confidence metrics are valuable for downstream margin scoring.
- Route-level freight market data is materially better than parcel-label APIs for big contracts.

**Cons:**

- Not a complete landed-cost engine on its own.
- Requires route and load assumptions.

**Recommendation:** **RECOMMENDED**

Use as the primary freight benchmark source for air/ocean.

#### Option B: Easyship APIs

**Official evidence:**

- Easyship exposes tax-and-duty calculation endpoints.
- Shipping APIs are strong for ecommerce shipping scenarios.

**References:**

- https://developers.easyship.com/reference/taxes_and_duties_calculate

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Freight relevance | 2 | more parcel/ecommerce oriented |
| Parcel coverage | 5 | strong |
| Customs add-on value | 4 | helpful complementary signal |
| Bulk-contract usefulness | 2 | weak for containers/palletized freight |
| Integration risk | 3 | commercial integration still required |
| **Average Score** | **3.2** | |

**Recommendation:** **SUPPORTING SOURCE**

Use for small shipments, sample shipments, and as a secondary customs sanity check, not as the core estimator for large tenders.

#### Option C: Shippo API

**Official evidence:**

- Shippo offers normalized carrier rates, labels, and international shipping workflows.
- International shipping docs emphasize customs declarations and parcel-level shipment creation.

**References:**

- https://docs.goshippo.com/
- https://docs.goshippo.com/shippoapi/public-api
- https://docs.goshippo.com/docs/international_shipping/internationalshipping

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Freight relevance | 2 | parcel-first |
| Parcel coverage | 5 | strong |
| Customs documentation support | 3 | useful |
| Bulk-contract usefulness | 1 | not sufficient for large goods tenders |
| Integration risk | 4 | mature developer surface |
| **Average Score** | **3.0** | |

**Recommendation:** **SUPPORTING SOURCE**

Useful for parcel-mode estimates, samples, and accessory shipments. Not sufficient for containerized landed-cost modeling.

#### Option D: SeaRates Logistics Explorer API

**Official evidence:**

- SeaRates provides a GraphQL API at `rates.searates.com/graphql`.
- Covers FCL, LCL, air freight, and land/road transport across 190+ countries.
- Automatic mode detection based on weight and locations (`Get Auto Rates` endpoint).
- Additional capabilities: container tracking, ship schedules, load calculator, carbon emissions, distance & time, freight index.
- Active development with ongoing updates (Feb 2026 release notes).

**References:**

- https://docs.searates.com/reference/logistics/get-rates

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Freight relevance | 5 | covers FCL, LCL, air, road |
| Route intelligence | 4 | auto mode detection, 190+ countries |
| Parcel coverage | 2 | not the main use case |
| Bulk-contract usefulness | 4 | strong for multimodal and LCL routes |
| Integration risk | 3 | GraphQL API, commercial access |
| **Average Score** | **3.6** | |

**Pros:**

- Complementary coverage to Freightos, especially for LCL and multimodal routes.
- Auto-rate mode detection simplifies mode selection logic.
- Additional tools (load calculator, carbon emissions) add value.

**Cons:**

- GraphQL API has a learning curve compared to REST.
- May overlap with Freightos for FCL/air.

**Recommendation:** **RECOMMENDED AS SECONDARY**

Use alongside Freightos for complementary coverage, especially for LCL and multimodal route estimation.

#### Freightos API Status Warning

Freightos's developer landing page indicates its APIs are being rebuilt (as of 2025) and are currently unsupported, even though the Price Stats documentation remains live and describes route-level USD statistics, rate limits, and native confidence levels based on data-point counts. Treat Freightos as the preferred freight data source where available, but do not treat it as a single operational dependency. A provider-abstracted freight layer is required.

#### HMRC Incidental Expense Simplifications (VAT Valuation Only)

HMRC publishes simplified nationally agreed rates for incidental transport expenses when actual costs are unknown:

- **Air freight:** 40p per air kilo (minimum £100)
- **Ocean/groupage:** £90 per gross-weight tonne plus £80 for groupage/consolidation (minimum £170)
- **Full load:** £550 per full load

These are for **import VAT valuation purposes only** — they determine the VAT base when actual incidental expenses are unknown. They must never be used as commercial freight quotes for margin estimation.

**Reference:** https://www.gov.uk/guidance/working-out-the-vat-value-using-the-customs-value-of-the-imported-goods

#### Shipping Recommendation

Build a **provider-abstracted, mode-aware logistics estimator**:

- `parcel` -> Easyship/Shippo (only for clearly courier-sized consignments, not as the fallback for unknowns)
- `air_freight` -> Freightos + SeaRates (for urgency/high-value/small-consignment cases)
- `lcl_ocean` -> Freightos + SeaRates (SeaRates has strong LCL coverage)
- `fcl_ocean` -> Freightos + SeaRates
- `uk_final_mile` -> internal rate cards / carrier quotes

Never use one API for all logistics cases. Treat Freightos as the preferred freight benchmark where available, and SeaRates as complementary coverage. Both providers expose native confidence metadata (Freightos: data-point counts; SeaRates: indicative rate flags) — use provider-native confidence signals rather than homemade guesses alone.

#### Mode Selection Decision Tree

Before selecting freight mode, resolve the **Incoterm cost boundary** from the supplier quote. If the quote is DDP, UK duty/VAT are already included in supplier cost; if FOB, inland freight to origin port is included but ocean freight is not. Different Incoterms change which cost lines the landed-cost engine must add separately.

**Hard mode locks (override weight/volume rules):**

Apply before any economic mode selection. Hazmat, oversized, fragile, reefer/temperature-controlled, lithium batteries, or installation-sensitive goods must override pure weight/volume routing.

**MVP mode selection heuristics when weight/volume data is sparse:**

1. **Infer packaging state.** Flat-pack vs assembled matters more than raw quantity for furniture and equipment. If packaging state is unknown, run **dual scenarios** (compact-pack and bulky-pack) and gate on the worse landed margin.
2. **Parcel:** Only for clearly courier-sized consignments (single items under ~30 kg, under 1.5 m longest side). Not the default for unknowns.
3. **Air freight:** For urgency, high value-to-weight, or small total consignment cases. Not the default for bulk.
4. **Ocean (LCL vs FCL):** Default for ambiguous bulky goods. Run both LCL and FCL scenarios when volume is uncertain. For furniture (cube-first goods), always default to ocean scenarios.
5. **Mixed mode:** Only when the basket naturally splits — e.g., accessories via parcel, main consignment via ocean.
6. **Always emit `freightLowGbp` / `freightBaseGbp` / `freightHighGbp`** and a separate `modeConfidence` score.

**Stress test — "500 office chairs + 200 desks":** Furniture is cube-first. If pack state is unknown, run flat-pack and assembled scenarios. Default to ocean (LCL + FCL comparison). Gate on the worse landed margin. Do not allow parcel as the default.

#### Freight vs VAT-Base Separation

Separate **commercial freight pricing** (what you actually pay the carrier) from **VAT-base transport estimation** (what HMRC requires for import VAT calculation). HMRC says import VAT value must add incidental expenses to the first UK destination, plus duty and other charges. Where actual incidental expenses are unknown, use HMRC's simplified nationally agreed rates (above) only for the VAT valuation calculation, never as commercial freight estimates.

### 4.5 Customs, Tariff, and Commodity Classification

#### Need

Accurately estimate duty and VAT in the UK, with explainable linkage to commodity code and valuation method.

#### Option A: HMRC Trade Tariff APIs + HMRC valuation guidance

**Official evidence:**

- HMRC Trade Tariff API provides commodity, duty, VAT, quota, and measure data.
- HMRC guidance explains customs value and import VAT valuation methods.
- HMRC provides Advance Tariff Ruling guidance for legally binding classification decisions.

**References:**

- https://hub.trade-tariff.service.gov.uk/
- https://api.trade-tariff.service.gov.uk/getting-started.html
- https://api.trade-tariff.service.gov.uk/the-commodities-api.html
- https://www.gov.uk/trade-tariff
- https://www.gov.uk/guidance/prepare-to-work-out-the-customs-value-of-your-imported-goods
- https://www.gov.uk/guidance/valuing-imported-goods-using-method-1-transaction-value
- https://www.gov.uk/guidance/working-out-the-vat-value-using-the-customs-value-of-the-imported-goods
- https://www.gov.uk/guidance/apply-for-an-advance-tariff-ruling

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 5 | official UK source |
| Explainability | 5 | full trace to code/measures/guidance |
| Coverage | 5 | tariff + VAT + measures |
| Implementation effort | 3 | you must build the engine yourself |
| Accuracy ceiling | 5 | highest available |
| **Average Score** | **4.6** | |

**Recommendation:** **MANDATORY CORE**

This should be the canonical customs engine.

#### Option B: HMRC FPO (Fast Parcel Operator) API

**Official evidence:**

- Official HMRC API that accepts English-language product descriptions and returns candidate commodity codes.
- Free to use for registered organizations.
- Uses ML model trained on tariff data (source: `github.com/trade-tariff/trade-tariff-lambdas-fpo-search`).
- Originally for Windsor Framework GB→NI parcels but the classification logic is broadly applicable.
- Available via HMRC Trade Tariff Developer Portal.

**References:**

- https://hub.trade-tariff.service.gov.uk/
- https://api.trade-tariff.service.gov.uk/fpo.html

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 5 | official HMRC, ML-trained on tariff data |
| Explainability | 4 | returns candidate codes with rationale |
| Coverage | 4 | broad commodity classification from text descriptions |
| Implementation effort | 5 | REST API, free, straightforward |
| Accuracy ceiling | 4 | strong first-pass, needs human review for edge cases |
| **Average Score** | **4.4** | |

**Pros:**

- Official HMRC source with ML-powered classification.
- Accepts plain English product descriptions — ideal for automated first-pass HS code classification.
- Free and accessible.

**Cons:**

- Originally designed for Windsor Framework GB→NI parcels; accuracy on procurement-scale industrial goods (Chapter 84 machinery, Chapter 90 medical instruments, Chapter 94 furniture) may be weaker than on consumer goods.
- Returns only 6-digit HS or 8-digit CN candidates with a match score. The score is retrieval relevance, not legal classification confidence — HMRC's own public repo example shows kitchen towels ranking below textile-towel codes despite eventually matching paper-towel codes.
- Does not return 10-digit declarable commodity codes. The UK tariff's legal duty treatment, additional codes, measure conditions, and specific measurement units live at the 10-digit commodity layer.
- Access may be restricted to UK Carrier-scheme FPOs invited to test. Verify access availability before depending on this as the primary automated classifier.
- Should be treated as a candidate generator, not a final classifier or authority.

**Recommendation:** **STRONGLY RECOMMENDED AS CANDIDATE GENERATOR**

Use as the primary automated first-pass HS code **candidate generator**, not the final classifier. Feed extracted product descriptions into the FPO API to get candidate commodity codes (HS6/CN8), then resolve candidates to 10-digit declarable codes via HMRC Trade Tariff API and validate against Taric Support and HS Ping. The FPO match score indicates retrieval relevance, not classification confidence — downstream resolution and measure scanning determine actual classification quality.

#### Option C: HS Ping

**Official evidence:**

- Commercial HS/HTS code lookup API using official data sources.
- Versioned responses for auditability.

**References:**

- https://hsping.com

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 3 | uses official sources, commercial wrapper |
| Explainability | 3 | code lookup with descriptions |
| Coverage | 4 | HS and HTS codes |
| Implementation effort | 4 | REST API |
| Accuracy ceiling | 3 | validation/cross-check, not primary classification |
| **Average Score** | **3.4** | |

**Recommendation:** **SUPPORTING SOURCE**

Use as a validation/cross-check source against HMRC FPO classifications.

#### Option D: Taric Support API

**Official evidence:**

- All-in-one customs compliance API for UK and EU.
- HS code browsing, validation, description retrieval, suggestions for incomplete codes.
- Real-time tariff data.

**References:**

- https://taricsupport.com/api/

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 4 | real-time tariff data from official sources |
| Explainability | 4 | browsable code hierarchy, validation, suggestions |
| Coverage | 4 | UK and EU tariff data |
| Implementation effort | 4 | REST API |
| Accuracy ceiling | 4 | strong for code validation and completeness checking |
| **Average Score** | **4.0** | |

**Recommendation:** **RECOMMENDED FOR VALIDATION**

Use for code validation, completeness checking, and UK/EU tariff data cross-referencing.

#### Option E: Third-party duty/tax estimation APIs

Easyship and similar services can provide convenience estimates, but they are not authoritative in the same way as HMRC tariff + valuation logic for UK imports.

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 2 | third-party abstraction |
| Explainability | 2 | usually black-box-ish |
| Coverage | 3 | useful convenience layer |
| Implementation effort | 4 | easier |
| Accuracy ceiling | 3 | acceptable for rough estimates, weaker for borderline deals |
| **Average Score** | **2.8** | |

**Recommendation:** **SECONDARY ONLY**

Use only as a comparison or fallback signal.

#### Option F: Third-party Landed Cost Calculator Services

For comparison and validation against the internal engine:

**Dutify**

- AI-powered product classification + landed cost calculator.
- API available for duty and tax calculation in one call.
- Reference: https://docs.dutify.com/docs/landed-cost-calculator

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 2 | commercial wrapper, not legal authority |
| Explainability | 3 | better than most black-box tools, still secondary |
| Coverage | 4 | classification + duty/tax in one service |
| Implementation effort | 4 | easy comparison integration |
| Accuracy ceiling | 3 | useful comparison signal, not canonical |
| **Average Score** | **3.2** | |

**Recommendation:** **USE AS COMPARISON SIGNAL**

**Zonos**

- Cross-border duty/tax calculation API.
- $299+/month, instant setup.
- Reference: https://zonos.com

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 2 | commercial service, not HMRC authority |
| Explainability | 3 | usable for validation, less transparent than internal engine |
| Coverage | 4 | broad cross-border support |
| Implementation effort | 4 | quick to stand up |
| Accuracy ceiling | 3 | strong sanity check, not canonical |
| **Average Score** | **3.2** | |

**Recommendation:** **USE AS SECONDARY VALIDATION**

**Avalara**

- Enterprise compliance solution.
- $16.5K+/year, 2-4 week setup.
- Reference: https://avalara.com

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 3 | enterprise-grade, still third-party |
| Explainability | 4 | stronger audit/compliance posture |
| Coverage | 5 | broadest enterprise feature set |
| Implementation effort | 2 | expensive and heavier onboarding |
| Accuracy ceiling | 4 | high, but overpowered for MVP |
| **Average Score** | **3.6** | |

**Recommendation:** **CONSIDER ONLY AT HIGHER SCALE**

Use third-party services ONLY as secondary validation signals, not as the primary engine. The internal HMRC-based engine must remain the canonical source.

#### AI-Based HS Classification (Future Enhancement)

The academic paper "ATLAS: Benchmarking and Adapting LLMs for Global Trade via Harmonized Tariff Code Classification" (2025) demonstrates fine-tuned LLMs achieving strong HS code classification. This approach could complement the HMRC FPO API as a future enhancement path for complex classification cases.

#### Classification Pipeline Preprocessing

Before any API call, apply preprocessing to improve classification quality:

1. **Description-quality gate.** Reject raw strings that are bundles ("centrifuges, sample preparation devices, and associated consumables"), service descriptions, or generic nouns ("goods", "samples", "parts", "equipment" without qualification). FPO's own documentation warns against generic words.
2. **Bundle splitter.** Decompose multi-item descriptions into separate product candidates before classification. "Centrifuges, sample preparation devices, and associated consumables" must become three separate classification requests. If decomposition fails, mark the basket as mixed/manual and stop automatic net-margin qualification.
3. **Attribute extraction.** Before lookup, extract: head noun, material, principal function, sector, sterility/medical status, power/size, and pack form. These attributes improve both FPO query quality and downstream 10-digit resolution.
4. **Multi-query FPO.** For each product candidate, run multiple normalized description variants: head noun only, noun+material, noun+function, noun+end-use. This reduces dependence on a single description phrasing.

#### 10-Digit Resolution and Measure Scanning

FPO returns only 6-digit HS or 8-digit CN candidates. The UK tariff's legal duty treatment lives at the **10-digit commodity code** layer and can require:

- **Additional 4-character codes** for trade remedies, preferences, quotas, or excise
- **Measure conditions** that change duty or documentary requirements based on price, weight, volume, or certificates
- **Specific measurement units** (e.g., duty per kg, per litre, per unit) that make weight/volume a duty input, not just a freight input
- **Geographic applicability** — different rates for different origin countries

The pipeline must resolve from HS6/CN8 candidates to a declarable 10-digit UK commodity code, then scan import measures for:

- Standard duty rates and any compound/specific duty components
- Trade remedy duties (anti-dumping, countervailing, safeguard) — additional duties that can dwarf base tariff rates, not cancelled by duty suspensions or ATQs
- Preferential tariff eligibility (requires rules-of-origin analysis and proof, not just supplier location)
- Quota/order numbers and remaining quota availability
- Import controls, prohibitions, restrictions, and required certificates/documents
- Additional codes required for declaration
- Measure condition thresholds (price, weight, volume triggers)

**Escalation logic:** Escalate automatically when:
- Top HS6 candidates do not converge (code dispersion)
- Candidates converge at HS6 but diverge materially at 10-digit measure level (measure dispersion)
- Additional codes or measure conditions are present that the engine cannot resolve
- Trade remedy duties are detected
- A positive margin coexists with an unsatisfied control measure (operational blocker)

#### Customs Recommendation

The landed-cost engine classification pipeline should:

1. Apply **description-quality gate and bundle splitter** to decompose and clean product descriptions before API calls.
2. Extract **structured attributes** (head noun, material, function, sector) from cleaned descriptions.
3. Use **HMRC FPO API** as the first-pass **candidate generator** — run multi-query variants and collect HS6/CN8 candidate codes with retrieval scores.
4. **Cluster and rank candidates** at HS6/CN8 level. If top candidates do not converge, flag classification instability.
5. **Resolve to 10-digit declarable commodity codes** via **HMRC Trade Tariff API** commodity hierarchy navigation.
6. Validate and cross-reference via **Taric Support API** for code completeness and hierarchy checking.
7. Cross-check with **HS Ping** for additional confidence signal.
8. **Scan import measures** via HMRC Trade Tariff API for: duty rates (including compound/specific), trade remedies, preferences, quotas, controls, additional codes, measurement units, and condition thresholds.
9. Apply customs valuation rules. **Method 1 (transaction value) is not universal** — no-sale scenarios, free samples, consignment stock, branch-office transfers, hire/lease, and condition-dependent or provisional pricing can require fallback valuation methods 2–6.
10. Note that **invoice price is not customs value** — selling commissions, assists, certain royalties/licence fees, seller's share of resale proceeds, and separately shown post-import deductions can all change duty liability.
11. Compute import VAT using the broadened VAT base: customs value + incidental expenses to first UK destination + duty + other import charges except VAT itself.
12. Determine **import VAT treatment** — postponed VAT accounting (PVA) is available only where the importer is UK VAT-registered, imports for business use, has the right to dispose of the goods, and includes its VAT number on the declaration. Non-VAT-registered importers cannot reclaim. Partially exempt or mixed business/non-business use can reduce recovery rates.
13. Produce per product candidate:
    - declarable 10-digit commodity code (or candidate set if unresolved),
    - additional codes required,
    - classification stability score,
    - duty estimate (base tariff + any trade remedy duties),
    - import VAT estimate with treatment classification,
    - VAT recovery rate estimate,
    - measure warnings (trade remedies, quotas, controls, operational blockers),
    - decision grade: `screening` | `review` | `rfq_required` | `broker_required`.

For repeated high-value product families, formalise the fallback:
- Request **Advance Tariff Rulings (ATRs)** — HMRC replies in roughly 30–120 days; legally binding for three years.
- Request **Advance Valuation Rulings (AVRs)** where the valuation method is complex — HMRC issues within 90 days; binding for three years.
- Cache approved product-family codebooks (HS10 + measures + ruling references) and reuse for matching products.
- Track classification accuracy: chapter accuracy, HS6 accuracy, 10-digit accuracy, duty-outcome accuracy, and review-trigger recall.

**References:**
- Advance Tariff Ruling: https://www.gov.uk/guidance/apply-for-an-advance-tariff-ruling
- Customs valuation Method 1: https://www.gov.uk/guidance/valuing-imported-goods-using-method-1-transaction-value
- PVA eligibility: https://www.gov.uk/guidance/check-when-you-can-account-for-import-vat-on-your-vat-return
- Trade remedies: https://www.gov.uk/government/publications/the-uk-trade-remedies-investigations-process/introduction-to-trade-remedies

#### Special Procedures and Alternative Duty Scenarios

The engine should expose special procedures as explicit **scenario switches**:

- **Inward Processing:** Can suspend customs duty and import VAT on goods imported for processing/repair and re-export.
- **Temporary Admission:** Can remove duty for temporary imports that are re-exported. Partial relief still leaves import VAT payable.
- **Duty suspensions and ATQs:** Can reduce or eliminate base duty for specific goods. ATQs are quantity-limited and do not remove VAT or trade remedy duties.

These can flip the economics of a deal. The engine should flag when a special procedure might be available and what the margin impact would be.

**References:**
- Inward Processing: https://www.gov.uk/guidance/special-procedure-inward-processing/rules-and-processes
- Origin proof: https://www.gov.uk/guidance/get-proof-of-origin-for-your-goods

**Note:** This is the ONE area where APIs are explicitly preferred over agent scraping. Cost-estimation data sources (customs, freight, FX) benefit from API reliability and structured responses. The skill-based agent scraping strategy applies to procurement and supplier data, not to cost-estimation APIs.

This internal-engine choice is formalized in **ADR-006**.

### 4.6 FX Conversion

#### Need

Everything should be evaluated in GBP and re-evaluated as FX changes.

#### Option A: Xe Currency Data API

**Official evidence:**

- Xe’s Currency Data API supports rates against a chosen base currency.
- Xe documentation discusses request-based packaging and update frequency.
- Xe states the API supports large fiat currency coverage and historical/point-in-time use cases.

**References:**

- https://www.xe.com/en-us/xecurrencydata/
- https://help.xe.com/hc/en-gb/articles/4411624652177-What-is-the-Currency-Data-API
- https://help.xe.com/hc/en-gb/articles/4411633342225-How-does-the-Currency-Data-API-work
- https://help.xe.com/hc/en-gb/articles/4414092026769-Currency-Data-API-packages-pricing-and-payment

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Requirement fit | 5 | explicit user preference |
| Historical support | 4 | point-in-time support implied by product docs |
| Commercial reliability | 4 | established provider |
| Implementation risk | 3 | pricing/request limits apply |
| **Average Score** | **4.0** | |

**Recommendation:** **SELECTED FOR COMMERCIAL FX**

Store FX snapshots in the database and re-run margin calculations when rate drift crosses a threshold. Use Xe for commercial margin estimation and real-time FX needs.

#### Option B: HMRC Exchange Rate Service

**Official evidence:**

- Official HMRC monthly exchange rates used for customs valuation calculations.
- Now integrated directly into the Online Trade Tariff service.
- Published penultimate Thursday of every month, applies to following calendar month.
- Available in CSV and XML formats.
- HMRC Developer Hub also provides XML API.

**References:**

- https://trade-tariff.service.gov.uk/exchange_rates/monthly
- https://developer.service.hmrc.gov.uk/api-documentation/docs/api/xml/Exchange%20rates%20from%20HMRC

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 5 | legally authoritative for UK customs duty computation |
| Historical support | 4 | monthly archives available |
| Commercial reliability | 5 | official government service |
| Implementation risk | 5 | CSV/XML, free, straightforward |
| **Average Score** | **4.8** | |

**Recommendation:** **MANDATORY FOR CUSTOMS VALUATION**

This is the legally authoritative rate for UK customs duty computation. The customs engine MUST use HMRC exchange rates for duty calculation. Use Xe for commercial margin estimation separately. The system should clearly distinguish between:
- **HMRC rates** → customs duty calculation (legally required)
- **Xe rates** → commercial margin estimation (real-time market rates)

#### Option C: CurrencyBeacon

**Official evidence:**

- 200+ currencies, 6000+ cryptocurrencies.
- Real-time and historical data.
- 99.9% uptime guarantee, sub-100ms response times.
- Institutional-grade data from ECB, Bank of Canada, etc.

**References:**

- https://currencybeacon.com

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Requirement fit | 4 | broad coverage, good performance |
| Historical support | 4 | historical data available |
| Commercial reliability | 4 | institutional-grade sources |
| Implementation risk | 4 | REST API, competitive pricing |
| **Average Score** | **4.0** | |

**Recommendation:** **BACKUP/COMPARISON**

Useful as a backup FX source or for cross-checking Xe rates.

#### Option D: OANDA Historical Currency Converter

**Official evidence:**

- 31 years of history, 200+ currencies, 38,000 FX pairs.
- Strong for auditing and historical analysis.

**References:**

- https://fxds-hcc.oanda.com

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Requirement fit | 3 | historical focus |
| Historical support | 5 | 31 years of data |
| Commercial reliability | 4 | established provider |
| Implementation risk | 3 | commercial access required |
| **Average Score** | **3.8** | |

**Recommendation:** **SUPPORTING SOURCE**

Useful for historical margin recalculation and audit trails.

#### FX Recommendation

Use a **tiered FX strategy**:

- **Customs valuation:** HMRC Exchange Rate Service (MANDATORY — legally authoritative)
- **Commercial margin estimation:** Xe Currency Data API (SELECTED — real-time market rates)
- **Backup/validation:** CurrencyBeacon (secondary signal)
- **Historical audit:** OANDA (for historical recalculation and audit trails)

Store `customsFxRate` and `commercialFxRate` **separately** in the database with timestamps, source identifiers, and HMRC publication periods. Do not collapse them into a single `fxRate` field — the customs rate is monthly (published penultimate Thursday, applies to the following calendar month) while the commercial rate is near-real-time.

**Basis gap tracking:** Compute and store `fxBasisDeltaPct` — the percentage gap between the HMRC customs rate and the Xe commercial rate at the time of estimation. For example, if HMRC CNY rate is 9.3778/£1 and Xe mid-market is 9.2029/£1, the basis gap is ~1.9%. This gap is a real cost/risk that affects margin accuracy.

**Recalculation trigger:** Trigger margin recalculation when the projected margin **moves materially or crosses a gate threshold**, not when raw FX drift alone exceeds a fixed percentage. A 2% FX move that doesn't change the margin decision is noise; a 0.5% move that flips a marginal opportunity across the 20% net gate is material.

**Operator display:** Present operators with both HMRC customs FX and market FX side by side, with the basis gap clearly shown. Do not hide the dual-rate complexity behind a single number.

### 4.7 Database Architecture — Local SQLite + Supabase/Postgres Sync

#### Need

A dual-database architecture that supports:

- **local-first operation** with the Rust daemon owning its own data store,
- **multi-user visibility** through a shared sync target,
- **corruption resilience** — a local non-corrupted copy protects against shared DB corruption,
- relational querying and auditability,
- storage of structured and unstructured artifacts,
- push-pull synchronization between local and remote.

#### Architecture: Authority Matrix — Local SQLite + Supabase/Postgres

**Design rationale:**

The owner requires multiple users to access the system's data. However, relying solely on a shared remote Postgres creates a single point of corruption risk. The architecture uses a **domain-owned authority model** rather than a singular "source of truth":

- **Local operational authority** (SQLite, bundled with the Rust daemon): Pipeline execution state, job leasing, source cursors, full raw session-output artifacts, full error context, raw documents. The daemon that ran the job is authoritative for its outputs.
- **Remote shared authority** (Supabase/Postgres): Shared configuration (company profile, source configs) and shared human decisions (operator dispositions). These originate remotely and are pulled to local.
- **Remote projections** (Supabase/Postgres): Viewer-facing copies of pipeline facts and status. These are push-only from local to remote and are never authoritative — the local copy can always republish.

**Terminology note:** In this document, prefer **local operational authority**, **remote shared authority**, and **shared sync target**. Historical mentions of "source of truth" appear only when contrasting this model with the superseded singular-authority design.

**Mixed-write tables are prohibited.** Every table has exactly one authority domain. If a concept spans domains (e.g., a pipeline-computed margin AND an operator disposition on the same process), it must be split into separate tables with separate authority.

**MVP constraint:** Exactly one active pipeline daemon per organization. Multi-executor coordination (remote source leases, primary election) is a future extension, not an MVP requirement. The web viewing portal is read-only; shared human decisions come only from other desktop apps if they exist.

**Corruption protection scope:** The dual-database design protects against remote-only failures (bad Supabase migration, accidental admin update, stale restore, RLS policy mistakes). It does NOT protect against application-level logical corruption in the sync/replication layer — a bug in normalization, merge heuristics, or sync mapping will write bad data to both sides. The sync layer is therefore the highest-risk component for data integrity and must be treated accordingly.

#### Authority Split — Per-Table Mapping

| Authority | Tables | Sync Direction |
|-----------|--------|----------------|
| Local-only (no sync) | `raw_documents`, `sources` (local runtime copy), `source_cursors`, `workflow_jobs` (full rows), `agent_session_logs` (full transcript/trace), `error_manifests` (full context), local job leases, local runtime config/secrets | None |
| Push-only (local → remote projections) | `executors` (heartbeat summary), `procurement_processes`, `contract_notices`, `notice_lots`, `qualification_runs`, `product_candidates`, `search_profiles`, `supplier_quotes`, `landed_cost_estimates`, `manifests`, `process_relations`, `pipeline_assessments`, `bid_workflow_states`, `process_stage_status`, `process_stage_heads`, `workflow_job_summaries`, `error_manifest_summaries` | Push only |
| Pull-only (remote → local) | `shared_company_profile`, `shared_source_configs`, `operator_decision_heads` | Pull only |
| Mixed-write | **None — prohibited by design** | N/A |

#### Push Protocol (Local → Remote)

Every local mutation to a push-owned entity writes to both business tables and a **local outbox** in the same SQLite transaction. Crashes cannot create ghost writes or missing replications.

- **Push unit:** Full-row projections (not field patches), batched by `local_seq` range.
- **Ordering:** Monotonic `local_seq` per executor. Remote rejects gaps in `local_seq` — if remote has seq 4810 and receives 4812 first, it fails closed until 4811 arrives.
- **Idempotency:** Remote deduplicates by `(executor_id, batch_id)` and `(executor_id, local_seq)`. Replays are harmless.
- **Row versioning:** For mutable push-owned rows, apply only if incoming `row_version > stored row_version` for the same owner. Equal version with matching `content_hash` is a replay (ignore). Equal version with differing hash is quarantined as a protocol bug.
- **Atomicity:** Remote applies each push batch through a single transactional RPC / stored procedure, not a spray of individual upserts.
- **Stage coherence:** All outputs from a pipeline stage share a `stage_run_id`. The stage's `process_stage_status` is updated and `process_stage_heads` is advanced in the same transaction. Failed stages publish failure status + redacted error summary but do NOT advance the head.
- **Schema compatibility:** Every push batch carries a `schema_epoch`. If epochs are incompatible, sync pauses but local operation continues.
- **Delivery timeout:** A push batch must either ack or fail within 60 seconds. After 3 consecutive failed attempts or 15 minutes in `pending`, the daemon marks sync `degraded`, surfaces an operator alert, and keeps local execution running.

#### Pull Protocol (Remote → Local)

- Pull applies only to remote-owned domains (shared config, operator decisions).
- Remote-owned tables carry a monotonic `remote_change_seq` and `remote_revision`, bumped on every insert/update/delete.
- Local stores a pull cursor per domain (`last_pulled_config_seq`, `last_pulled_decision_seq`).
- Pull query fetches changes where `remote_change_seq > cursor`, ordered by seq, in bounded batches.
- Apply locally in order, transactionally, advance cursor only after successful commit.
- **Operator decisions** (when multiple desktops exist): Written with compare-and-swap — the update includes `base_remote_revision` and succeeds only if current revision matches. On mismatch, return conflict for manual reconciliation. Last-write-wins is NOT used for human commercial decisions.

**Convergence guarantees:** Ordering comes from `local_seq` per executor (push), `remote_change_seq` (pull), and `row_version` / `remote_revision` (row freshness). `updated_at` is for human audit only, never for conflict resolution — clock-based conflict rules create false simplicity.

#### Recovery Model

**Remote corrupted or lost → republish from local:**

The daemon can republish all push-owned projections and stage heads from local state. Two distinct recovery modes:

- **Incremental sync:** Normal outbox replay for recent changes.
- **Full remote rebuild:** Republish current state of all local-owned tables + stage heads. Not just recent outbox entries.

Shared config that originated in Supabase and was only cached locally may be lost if Supabase is the only copy.

**Local SQLite lost → partial rebuild from Supabase:**

From Supabase, the daemon can recover: shared config, shared operator decisions, remote projections of pipeline facts, redacted job/error summaries. The following are **permanently lost**: full raw session-output artifacts (PTY terminal bytes or contract-mode stdout/stderr), full error context, source cursors (daemon will re-ingest from scratch — expensive but recoverable), local job leases, raw documents if stored only on the same disk.

Two recovery tiers:

- **Warm rebuild:** Local DB file damaged but disk/artifact store still available. Run `PRAGMA quick_check` to assess damage, restore from hot backup (Online Backup API or `VACUUM INTO`) if available, then repull remote-owned state and republish local-owned projections.
- **Cold rebuild:** Full machine loss. Requires off-box retention of critical raw procurement artifacts and optionally full transcripts if operationally critical.

**Local corruption detection:** Run `PRAGMA quick_check` on startup; run `PRAGMA integrity_check` on maintenance windows. Use `.recover` only as a salvage path, not the primary recovery plan. Never take backups by copying the main DB file while the process is live — use the Online Backup API or `VACUUM INTO`.

#### Tombstone Policy

Tombstones (`deleted_at`) are used **only** for entities with real shared delete semantics:

- Shared configuration (remote-owned)
- Shared operator decisions (remote-owned)
- Viewer-facing projections where delete must propagate

High-volume pipeline history (quotes, estimates, qualification runs, manifests) is **append-only** and **superseded by stage head pointers**, not tombstoned.

Tombstone garbage collection is safe only when:

- Every active replica has pulled past the delete's `remote_change_seq`, AND
- The tombstone is older than `max_offline_window + safety_margin` (e.g., 30 days + 7 days = 37 days)

Replicas offline past the window become **stale** and must full-bootstrap instead of incremental pull.

#### Local SQLite

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Local-first fit | 5 | bundled with Rust daemon, no network dependency |
| Rust compatibility | 5 | `rusqlite` and `sqlx` with SQLite feature are excellent |
| Corruption resilience | 5 | local copy is independent of shared DB state |
| Multi-user access | 1 | single-user by design |
| Relational power | 4 | full SQL, JSON via json1 extension, but no native arrays |
| **Average Score** | **4.0** | |

**SQLite compatibility notes:**

- No native array types — use TEXT with JSON arrays or junction tables.
- JSON support via json1 extension (built into most SQLite distributions).
- No `gen_random_uuid()` — use application-generated UUIDs.
- No `timestamptz` — store timestamps as TEXT (ISO 8601) or INTEGER (Unix epoch).
- No native `boolean` type — use `INTEGER` (0/1). SQLite accepts `boolean` in DDL but stores as integer.
- No `bigint` distinction — use `INTEGER` (SQLite integers are 64-bit when needed).
- No `numeric` with precision — `REAL` or `INTEGER` (prefer integer pence for monetary values per §5.4).
- No `jsonb` — use `TEXT` with json1 functions. `json_valid()` for check constraints.
- No concurrent write access — single-writer model suits the daemon architecture.
- Use `STRICT` tables where json1 extension is not needed (enforces type affinity).

#### Supabase/Postgres (Shared Sync Target)

**Official evidence:**

- Supabase uses Postgres as its core database.
- RLS is a first-class security model.
- Realtime can stream Postgres changes and supports authorization.
- Self-hosting exists if future control is needed.

**References:**

- https://supabase.com/docs/guides/getting-started/architecture
- https://supabase.com/docs/guides/database/postgres/row-level-security
- https://supabase.com/features/realtime-postgres-changes
- https://supabase.com/docs/guides/realtime/authorization

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Multi-user shared access | 5 | ideal for shared visibility |
| Audit/query power | 5 | excellent for joins, analytics, history |
| Realtime | 4 | strong enough for web viewing portal updates |
| Auth/RLS | 5 | mature and explicit |
| Rust compatibility | 4 | Postgres/sqlx is straightforward |
| **Average Score** | **4.6** | |

**Role in authority matrix:** Supabase/Postgres serves two distinct roles:

- **Remote shared authority:** Shared configuration (company profile, source configs) and shared human decisions (operator dispositions) originate here and are pulled to local.
- **Remote projections:** Viewer-facing copies of pipeline facts, stage status, and summaries are pushed here from local. The web viewing portal reads these projections. Multiple operators can see data from each other's local pipelines.

#### Sync Protocol (Supabase-Specific)

The sync engine implements authority-based replication as described in section 4.7 above. Supabase-specific considerations:

- Push batches are applied through a single transactional RPC / stored procedure (not a spray of individual upserts from the daemon client).
- For web portal live updates, prefer small projection-change or status broadcast events over raw Postgres Changes subscriptions on every hot table. Supabase Broadcast is the recommended path for scalability; Postgres Changes is simpler but less scalable.
- Do not have the web portal query all raw sync tables directly — provide dedicated projection tables keyed to portal use-cases.

#### Database Recommendation

Use **local SQLite** with authority-matrix-based replication to **Supabase/Postgres** for multi-user visibility. Convex and SpacetimeDB are not selected (see original evaluation — good products, wrong fit for this workload).

This authority split is formalized in **ADR-002**.

### 4.8 Desktop Runtime and Frontend Shell

#### Need

A desktop client with:

- ergonomic UI,
- secure local agent execution,
- background daemon/sidecar support,
- optional shared UI with web viewing portal.

#### Option A: Tauri v2 + Next.js static export

**Official evidence:**

- Tauri’s Next.js guide requires static exports.
- Tauri does not natively support server-based solutions inside the desktop shell.

**References:**

- https://v2.tauri.app/start/frontend/nextjs/
- https://v2.tauri.app/start/frontend

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| User preference alignment | 5 | user explicitly likes this path |
| Desktop fit | 3 | works, but only in static/client-heavy mode |
| Shared code potential | 4 | good with web app |
| Complexity | 3 | extra friction if trying to reuse SSR assumptions |
| **Average Score** | **3.8** | |

#### Option B: Tauri v2 + React/Vite desktop + separate Next.js web app

**Official evidence:**

- Tauri docs explicitly recommend Vite for SPA frameworks.
- Tauri supports frontend-agnostic static bundles.

**References:**

- https://v2.tauri.app/start/frontend

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Desktop fit | 5 | simplest and cleanest |
| Operational clarity | 5 | avoids SSR mismatch |
| Shared code potential | 4 | shared UI package still easy |
| User preference alignment | 3 | deviates from stated Next preference |
| **Average Score** | **4.4** | |

#### Option C: Electron + Next.js

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Desktop maturity | 5 | mature |
| Security footprint | 2 | heavier |
| Resource profile | 2 | larger |
| Rust sidecar synergy | 3 | okay, but less attractive |
| **Average Score** | **3.0** | |

#### Desktop Recommendation

**Selected:** Tauri v2.

**Strong recommendation:** Use **shared React UI packages** and avoid binding the whole product architecture to “one Next app inside Tauri”. If you insist on Next for desktop, keep it strictly client-side/static and do not design around SSR/server actions inside the Tauri shell.

This shell/runtime choice is formalized in **ADR-003**.

### 4.9 Local Agent Orchestration Runtime

#### Need

Run Codex/Claude agents locally on user machines with:

- reusable skills,
- dynamic kickoff prompts,
- adapter-mediated agent execution with PTY and contract modes from day 1,
- full raw session-output capture as forensic infrastructure,
- validated result contracts as the authoritative workflow-state record,
- cron-triggered and manual-triggered sessions,
- robust daily operation without interruption,
- process-tree supervision and reliable cleanup.

Agent orchestration is the most critical infrastructure piece. The entire data ingestion, intelligence, and bidding pipeline depends on it working every time. PTY execution is a first-class runtime capability from day 1, but not the mandatory transport for every job — the daemon selects the appropriate execution mode per job via an adapter boundary.

#### Selected Approach: Adapter-Mediated Hybrid Execution

**Design decision:**

The Rust daemon triggers Codex/Claude sessions through an **Agent Adapter** boundary with two execution modes:

1. **PTY mode:** Required for browser automation, interactive auth flows, or any CLI behavior that depends on a controlling terminal. The daemon creates PTY pairs (master/slave), captures full terminal output as an immutable forensic artifact, and monitors session health.
2. **Contract mode:** Preferred for deterministic tasks where the vendor supports structured non-interactive execution (Codex `exec` with `--json` JSONL output; Claude headless with `json`/`stream-json` output and optional `--json-schema` validation). Lower overhead, simpler cleanup, stronger result contracts.

The daemon selects mode per job based on the skill's declared `interaction_profile`. PTY remains a first-class runtime from day 1 — it is not deferred or simplified away.

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Flexibility | 5 | PTY handles all CLI modes; contract mode handles deterministic flows with less overhead |
| Robustness | 5 | adapter boundary isolates vendor changes; mode selection matches job requirements |
| Log capture | 5 | full raw session output captured in both modes, with structured events captured in contract mode |
| Automation fit | 5 | cron + manual triggers, headless operation in both modes |
| Complexity | 3 | adapter + dual-mode is more complex than single-mode, but reduces blast radius |
| **Average Score** | **4.6** | |

**Agent Adapter boundary:**

Each vendor (Claude, Codex) has a dedicated adapter (`agents/codex.rs`, `agents/claude.rs`) that produces a **canonical daemon event stream**. The adapter owns:

- Launch command construction (flags, env vars, working directory).
- Capability probing at startup (CLI version, auth health, structured output viability, PTY viability, browser/tool viability).
- Mode selection (`pty` or `contract`) based on skill `interaction_profile` and probe results.
- Parsing vendor-native JSON/event streams into daemon events.
- Health classification from vendor-specific signals.
- Final result contract validation.

Everything above the adapter speaks only in daemon-native events: `SessionStarted`, `CheckpointWritten`, `AuthChallenge`, `SessionStalled`, `CleanupIncomplete`, `FinalResultValidated`. Unknown CLI versions enter **canary mode** — the adapter runs a smoke probe before accepting production traffic.

**Session isolation:**

Concurrent sessions on the same workstation can contaminate each other through shared CLI config and state. The daemon must:

- Pin `CODEX_HOME` / `.codex` config paths per session or verify isolation.
- Control working directory per run.
- Record CLI version, model, config hash, skill version, and adapter version on every session.
- Use per-session browser profiles for browser-backed flows (prevent cookie/lock contention).

**Session lifecycle (expanded):**

Each agent session has a lifecycle with explicit phases:

```text
created → spawned → active → terminating → exited → reaped → archived
```

A session is only **complete** after all of:

1. Child exit observed.
2. PTY drained to EOF or drain timeout (PTY mode only).
3. Descendant process tree cleaned up (verified, not assumed).
4. Final result contract validated (if present).
5. Cleanup verdict recorded.

- **Cron-triggered execution:** The daemon's built-in cron scheduler creates sessions on schedule, selects the appropriate execution mode via the adapter, injects the kickoff prompt and skill reference, and monitors execution.
- **Manual triggers from desktop:** The Tauri desktop control panel sends trigger commands to the daemon, which creates a session (PTY or contract mode) and streams progress back to the desktop control panel.
- **Full output capture (PTY mode):** Every byte of terminal output is captured and stored as an immutable forensic artifact. This includes agent thinking, tool calls, errors, and results. Terminal output is a forensic channel for debugging — it does not drive workflow state.
- **Full output capture (contract mode):** The daemon captures the raw stdout/stderr stream as an immutable forensic artifact while the adapter parses vendor JSONL/JSON events into daemon-native events. Parsed events drive workflow state; raw bytes remain for debugging and replay.
- **Session capture invariant:** Every agent session must leave behind a complete forensic record appropriate to its mode: raw PTY bytes for PTY mode, or raw stdout/stderr bytes plus structured protocol events and result artifacts for contract mode.
- **Auth challenge detection:** The daemon detects when an agent session fails due to expired credentials, MFA challenges, browser SSO failures, or hidden permission prompts. These are classified as `AGENT_AUTH_CHALLENGE` (distinct from `AGENT_STALLED`).

**Why PTY remains a first-class runtime (not the only runtime):**

PTY mode is required when:

1. The skill uses browser automation (agent needs real terminal semantics for tool interactions).
2. The agent CLI session involves interactive auth flows or re-authentication.
3. The vendor CLI does not support structured non-interactive output for the required task.
4. The skill explicitly declares `interaction_profile: pty_interactive`.

Contract mode is preferred when:

1. The task is deterministic and non-interactive (e.g., API-driven qualification).
2. The vendor supports structured JSON/JSONL output (Codex `exec --json`, Claude headless `json`/`stream-json`).
3. The skill declares `interaction_profile: headless_contract`.
4. Simpler cleanup and stronger result contracts outweigh terminal fidelity.

The adapter chooses mode per job. If a contract-mode session encounters an unexpected interactive prompt or auth challenge, the adapter can escalate the failure rather than silently stalling.

**Process-tree supervision:**

The daemon supervises the **entire descendant tree**, not just the initial CLI process. Agent CLIs may spawn sub-processes (browser instances, tool servers, language runtimes).

The cleanup sequence:

1. Spawn session in its own process group (POSIX) or job object (Windows).
2. Record pid, pgid, PTY id, adapter id at spawn.
3. On soft stop: send in-band interrupt if the adapter supports it.
4. On timeout: SIGTERM the **process group**, not just the main pid.
5. After grace period: SIGKILL the group.
6. Keep draining the PTY master until EOF or drain timeout (PTY mode).
7. Wait/reap explicitly — do not rely on dropping Tokio child handles.
8. Run orphan sweep and record cleanup verdict (`clean`, `forced`, `leaked`).

On Linux, each session can optionally run in a transient `systemd-run --scope` unit for resource control, or the daemon can register as a child subreaper. On macOS, process group signaling via `killpg` is the primary mechanism.

**PTY hardening:**

- **File descriptor hygiene:** Aggressively close inherited fds before exec. macOS Big Sur/Cocoa and GNOME/Mutter can leak fds into child processes, causing intermittent failures.
- **EIO handling:** Treat `EIO` on the PTY master read (after slave closes) as a normal termination path, not an internal error.
- **Signal handling:** Use signals only as wakeups, then drain `waitpid`/wait loops until there is nothing left to reap. Signals may be coalesced by Tokio's Unix signal streams.
- **Terminal geometry:** Fix PTY size for headless runs (e.g., 120x40) so CLI formatting and browser/tool behavior are stable. Only resize when an operator attaches for live viewing.

**Host environment awareness:**

- **Sleep/wake:** If the host suspends during an active session, assume browser-driven PTY sessions are tainted. Use monotonic time for stall detection and emit `HOST_SLEEP_INTERRUPTED` if a suspend gap crosses a configurable threshold.
- **Missed runs:** When the daemon starts after extended downtime, it checks for overdue scheduled jobs and runs a catchup sweep. Jobs are executed in priority order, not all at once.

This runtime model is formalized in **ADR-004**.

### 4.10 Skills and Browser Automation — Core Data Retrieval Architecture

Skills are the **primary data retrieval mechanism** for the platform. All procurement data and supplier pricing is collected via skill-based agent scraping with browser automation. This is the core architectural decision that simplifies the entire system: instead of maintaining API client packages for each data source, the platform maintains skills (way easier to maintain, upgrade, and change).

#### Skills

**Official evidence:**

- OpenAI Codex skills and Claude skills both use directory-based `SKILL.md` packaging and progressive disclosure.

**Terminology note:** In this document, **skill** means the versioned directory asset bound to a job, while **skill playbook** means the human-readable instructions inside that skill's `SKILL.md`.

**References:**

- https://developers.openai.com/codex/skills/
- https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview

#### Browser automation reference

**Reference:**

- https://github.com/vercel-labs/agent-browser/blob/main/skills/agent-browser/SKILL.md

#### Skill Types

The platform requires skills across three categories:

**1. Procurement Ingestion Skills** (agent scrapes procurement sources via browser):

| Skill | Target Source | Approach |
|-------|--------------|----------|
| `procurement-ingestion/fts` | Find a Tender | Navigate to FTS API/portal, extract OCDS notice data |
| `procurement-ingestion/contracts-finder` | Contracts Finder | Navigate to CF API/portal, extract notice data |
| `procurement-ingestion/pcs` | Public Contracts Scotland | Navigate to PCS API/portal, extract notice data |

**2. Supplier Price Enrichment Skills** (agent scrapes marketplace websites via browser):

| Skill | Target Source | Approach |
|-------|--------------|----------|
| `supplier-price-enrichment/ebay` | eBay UK | Search products, extract pricing, seller info, shipping |
| `supplier-price-enrichment/amazon` | Amazon UK | Search products, extract pricing, availability |
| `supplier-price-enrichment/alibaba` | Alibaba.com | Search products, extract pricing, MOQ, shipping |
| `supplier-price-enrichment/1688` | 1688.com | Search products, extract wholesale pricing |

**3. Intelligence Pipeline Skills** (agent performs extraction and classification):

| Skill | Purpose | Approach |
|-------|---------|----------|
| `document-acquisition` | Acquire public tender documents | Navigate to notice source portal, identify and download spec PDFs, pricing schedules, BoQ spreadsheets, lot documents |
| `lot-extraction` | Extract lot structure from documents | Parse unstructured lot descriptions when OCDS lot data is absent |
| `product-extraction` | Extract product candidates from lot text/docs | Identify items, classify candidateType, extract structured slots, decompose bundles |
| `preliminary-qualification` | Classify goods/services for ambiguous cases | Analyze notice text only when deterministic expiry/category/CPV rules leave ambiguity |

**4. Workflow Skills** (agent performs workflow actions via browser):

| Skill | Purpose | Approach |
|-------|---------|----------|
| `bid-portal-*` | Portal-specific bid preparation | Navigate to eSourcing portal, fill forms, capture evidence |
| `manifest-generation` | Generate opportunity manifests | Fill narrative sections for the canonical Markdown manifest produced by the manifest package |

#### Skill Lifecycle

1. **Creation:** Skills are written as `SKILL.md` files with clear playbook instructions for the agent.
2. **Execution:** The daemon triggers an agent session (PTY or contract mode) via the adapter with the skill loaded. The agent follows the skill's playbook.
3. **Maintenance:** When a source website changes layout/flow, only the skill needs updating — no API client code changes.
4. **Versioning:** Skills are versioned in the repo. The daemon records which skill version was used for each execution.
5. **Upgrade:** Updating a skill is as simple as editing the `SKILL.md` — no recompilation, no package upgrades.

#### Skill Design Principles

- Skills define **what data to extract** and **how to navigate** the source.
- Skills include **validation rules** so the agent can verify it extracted correct data.
- Skills include **error handling instructions** so the agent can report failures clearly.
- Skills are **source-specific** — one skill per source, not generic "scrape anything" skills.
- Skills output a validated result contract (JSON payload, artifact file, or NDJSON checkpoint stream) that the daemon validates against a declared schema — workflow state is never driven by transcript scraping.
- **Scheduled jobs bind explicit skill id and version.** Implicit skill discovery is disabled for production cron runs. Discovery may be used for ad hoc operator-triggered sessions.

#### Skill Metadata

Every skill declares metadata that the daemon uses for execution planning:

| Field | Purpose | Example |
|-------|---------|---------|
| `interaction_profile` | Execution mode selection | `pty_interactive` or `headless_contract` |
| `result_schema_version` | Result contract versioning | `1.0` |
| `auth_requirements` | What credentials the agent needs | `browser_session`, `api_key`, `none` |
| `browser_required` | Whether the skill needs browser automation | `true` / `false` |
| `max_concurrency` | Maximum concurrent sessions for this skill's source | `1` |
| `expected_duration` | Typical runtime for resource planning | `5m`, `30m` |

Skills that declare `interaction_profile: headless_contract` must also declare a `result_schema_version` that the daemon uses for result contract validation.

#### Architecture Distinction

Skills are the primary data retrieval mechanism, but they are **still not the workflow engine**. The distinction:

- **Skills define:** How the agent behaves when scraping a source, what data to extract, how to handle errors, and what execution mode they require.
- **Daemon/database defines:** When to run which skill, what job state to track, retry logic, scheduling, sync, and which skill version to bind.
- **Orchestration stays in the daemon:** The daemon creates sessions (PTY or contract mode via the adapter), selects skills by explicit binding, monitors execution, validates result contracts, stores results in SQLite, and syncs to Supabase.
- **Adapter mediates:** The Agent Adapter layer handles vendor-specific launch, structured output parsing, health classification, and capability probing — insulating the daemon from CLI version changes.

These boundaries are formalized in **ADR-005**.

### 4.11 Licensing and Compliance Summary

| Dependency / Data Source | License / Terms Signal | Compatible | Notes |
|--------------------------|------------------------|------------|-------|
| Find a Tender data | Open Government Licence | Yes | official public data, accessed via agent skill scraping |
| Contracts Finder data | government open publication model | Yes | accessed via agent skill scraping |
| PCS API | official public API | Likely | accessed via agent skill scraping |
| HMRC Trade Tariff API | official public government API | Yes | direct API access for cost estimation |
| HMRC FPO API | official public government API | Yes | direct API access for HS code classification |
| HMRC Exchange Rate Service | official public government data | Yes | free, published monthly |
| Xe API | commercial | Yes | paid integration for commercial FX |
| Freightos | commercial | Yes | sales/commercial access required |
| SeaRates | commercial | Yes | commercial API access required |
| Easyship | commercial | Yes | parcel-mode freight and duty estimation (supporting source) |
| Shippo | commercial | Yes | parcel-mode carrier rates and international shipping (supporting source) |
| CurrencyBeacon | commercial | Yes | backup/comparison FX provider (optional) |
| OANDA | commercial | Yes | historical FX audit trail (optional) |
| Taric Support | commercial | Review | commodity-code validation — verify API terms for automated use |
| HS Ping | commercial | Review | HS-code comparison service — verify API terms |
| Dutify / Zonos / Avalara | commercial | Review | optional landed-cost comparison providers — used for validation only |
| eBay (browser scraping) | website terms of use | Review | agent skill scraping — review ToS for automated access |
| Amazon (browser scraping) | website terms of use | Review | agent skill scraping — review ToS for automated access |
| Alibaba/1688 (browser scraping) | website terms of use | Review | agent skill scraping — review ToS for automated access |

**License Summary:**

The core recommended architecture is license-compatible for cost-estimation APIs (HMRC, Xe, Freightos, SeaRates, Easyship, Shippo, CurrencyBeacon, OANDA). For procurement sources, the data is under open government licenses. APIs marked "Review" (Taric Support, HS Ping, Dutify/Zonos/Avalara) require verification of commercial terms for automated pipeline use. The main compliance consideration for the skill-based scraping approach is **website terms of use** for supplier marketplaces (eBay, Amazon, Alibaba). Agent skill scraping should be low-volume and source-specific, with ToS reviewed for each marketplace before implementation.

---

## 5. Technical Architecture Blueprint

### 5.1 System Context Diagram

```text
┌──────────────────────────────────────────────────────────────────────────────────┐
│                           SYSTEM CONTEXT                                         │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   Procurement Sources (scraped via agent skills)                                 │
│   ┌────────────────────┐  ┌────────────────────┐  ┌────────────────────┐        │
│   │ Find a Tender      │  │ Contracts Finder   │  │ Public Contracts   │        │
│   │ (web portal/API)   │  │ (web portal/API)   │  │ Scotland           │        │
│   └─────────┬──────────┘  └─────────┬──────────┘  └─────────┬──────────┘        │
│             └──────────────┬────────┴──────────────┬────────┘                    │
│                            ▼                       ▼                             │
│   Supplier Sources (scraped via agent skills)                                    │
│   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐                       │
│   │ eBay     │  │ Amazon   │  │ Alibaba  │  │ 1688     │                       │
│   └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘                       │
│        └──────┬──────┴──────┬──────┴──────┬──────┘                              │
│               ▼             ▼             ▼                                      │
│   ┌─────────────────────────────────────────────────────────────┐                │
│   │ Rust Daemon (Local Machine)                                 │                │
│   │ ┌─────────────────────────────────────────────────────────┐ │                │
│   │ │ Agent Session Runtime                                   │ │                │
│   │ │ - cron scheduler + manual triggers                      │ │                │
│   │ │ - adapter-mediated sessions (PTY or contract)           │ │                │
│   │ │ - skill loading + kickoff prompt injection              │ │                │
│   │ │ - forensic capture + protocol tracing                   │ │                │
│   │ │ - agent health monitoring                               │ │                │
│   │ └─────────────────────────┬───────────────────────────────┘ │                │
│   │                           ▼                                  │                │
│   │ ┌─────────────────────────────────────────────────────────┐ │                │
│   │ │ Local SQLite (Local Operational Authority)              │ │                │
│   │ │ - contracts, jobs, manifests, artifacts                 │ │                │
│   │ │ - agent session logs, error manifests                   │ │                │
│   │ │ - cursor/watermark state                                │ │                │
│   │ └─────────────────────────┬───────────────────────────────┘ │                │
│   │                           │ authority-based sync             │                │
│   └───────────────────────────┼──────────────────────────────────┘                │
│                               ▼                                                  │
│   ┌─────────────────────────────────────────────────────────────┐                │
│   │ Supabase / PostgreSQL (Shared Sync Target)                  │                │
│   │ - multi-user visibility                                     │                │
│   │ - shared configuration                                      │                │
│   │ - web portal data source                                    │                │
│   └───────────┬───────────────────────────┬─────────────────────┘                │
│               │                           │                                      │
│               ▼                           ▼                                      │
│   ┌───────────────────────┐   ┌────────────────────────────────┐                │
│   │ Desktop (Tauri)       │   │ Web Viewing Portal             │                │
│   │ CONTROL PANEL         │   │ READ-ONLY                      │                │
│   │ - trigger workflows   │   │ - browse opportunities         │                │
│   │ - manage agents       │   │ - view manifests               │                │
│   │ - make decisions      │   │ - view pipeline status         │                │
│   │ - view local data     │   │ - view errors                  │                │
│   │ (reads local SQLite)  │   │ (reads Supabase)               │                │
│   └───────────────────────┘   └────────────────────────────────┘                │
│                                                                                  │
│   Cost-Estimation APIs (direct API access, not agent-scraped)                    │
│   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │
│   │ HMRC     │  │ HMRC FPO │  │ HMRC     │  │Freightos │  │ Xe /     │        │
│   │ Tariff   │  │ (HS code)│  │ FX Rates │  │+ SeaRates│  │ Currency │        │
│   └──────────┘  └──────────┘  └──────────┘  └──────────┘  └──────────┘        │
│                                                                                  │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 High-Level Component Design

#### Component: Source Registry

**Responsibility:** Define procurement and supplier sources, associated skills, schedules, and status. Stored in local SQLite.

**Interface:**

```ts
export interface SourceDefinition {
  id: string;
  kind: "fts" | "contracts_finder" | "pcs" | "ebay" | "amazon" | "alibaba" | "1688";
  category: "procurement" | "supplier";
  enabled: boolean;
  skillPath: string;  // path to the skill directory
  scheduleCron?: string;
  lastRunAt?: string;
  lastCursorValue?: string;
}
```

#### Component: PTY Session Manager

**Responsibility:** Create, monitor, and manage live PTY-backed sessions for agent execution. Captures PTY-mode raw terminal bytes as immutable forensic artifacts and manages the active PTY lifecycle (`created → spawned → active → terminating → exited`). Performs PTY-specific process-group supervision and hardening (fd hygiene, EIO handling, fixed terminal geometry). Final descendant cleanup, orphan sweep, and end-of-life cleanup verdicts are owned by `Session Reaper`.

#### Component: Daemon Cron Scheduler

**Responsibility:** Schedule and enqueue agent work based on cron definitions. Runs locally in the Rust daemon — no hosted scheduler dependency. Binds explicit skill id and version for scheduled jobs (no implicit skill discovery for cron). Determines when a job should exist; it does NOT execute the job payload itself. Performs catchup sweeps for overdue jobs when the daemon starts after extended downtime.

#### Component: Job Runner

**Responsibility:** Claim queued jobs from local SQLite, acquire the required resource budget from the Resource Arbiter, invoke the Agent Adapter in the correct execution mode, and persist stage outputs plus job status. The Job Runner owns execution of already-enqueued work; it does not decide schedules or replication semantics.

#### Component: Skill-Based Ingestion Runner

**Responsibility:** Domain-specific execution logic for procurement-ingestion jobs running under the Job Runner. It selects the procurement skill, invokes the Agent Adapter in the correct mode (PTY or contract per skill), normalizes the retrieved notice data, and writes immutable session artifacts to the artifact store. It is intentionally a job-type handler, not a second scheduler or runtime subsystem.

#### Component: Log/Trace Capture Engine

**Responsibility:** Capture session data across three layers: (1) **control events** from the daemon (spawn, signal, exit, reap, cleanup verdict), (2) **protocol events** from the adapter (vendor JSONL/JSON events parsed into daemon-native events, validated checkpoints), and (3) **forensic bytes** (immutable raw PTY capture in PTY mode, raw stdout/stderr capture in contract mode). Control and protocol events drive workflow state; forensic bytes are for debugging and replay. Structured traces are derived primarily from control/protocol events; transcript parsing is best-effort enrichment only.

#### Component: SQLite Local Store

**Responsibility:** Local operational authority for all daemon-owned data. Stores pipeline data, jobs, manifests, agent session logs, error manifests, cursor state, and the sync outbox. Bundled with the Rust daemon. Split into two databases for operational isolation:

- **`state.db`**: Hot relational state — pipeline entities, jobs, manifests, outbox, sync metadata. WAL mode enabled. STRICT tables where json1 is not required.
- **`artifacts.db`** (or `artifacts/` directory): Large session transcripts (raw PTY bytes or raw stdout/stderr streams), raw document payloads, error context blobs. Separating these from the hot state DB prevents checkpoint and vacuum overhead from large writes.

Operational requirements: keep on local disk only (never NFS/cloud-sync folders), run `PRAGMA quick_check` on startup, run `PRAGMA optimize` on maintenance cadence, use Online Backup API or `VACUUM INTO` for safe hot backups. Store `schema_epoch` in `PRAGMA user_version`.

#### Component: Sync Engine

**Responsibility:** Authority-based replication between local SQLite and Supabase/Postgres. NOT a generic bidirectional sync engine. The SQLite Local Store owns persistence and local transactionality; the Sync Engine owns outbox construction, remote apply, pull cursors, and schema-epoch compatibility. Comprises:

- **Local outbox writer:** Writes outbox entries in the same SQLite transaction as business mutations. Every push-owned entity change produces an outbox row.
- **Push batch builder:** Reads outbox entries by `local_seq` range, constructs full-row projection batches with `schema_epoch`, `content_hash`, and `stage_run_id`.
- **Transactional remote apply RPC:** Submits each push batch to Supabase as a single atomic RPC. Handles dedup, gap rejection, and row-version comparison.
- **Remote pull cursor store:** Maintains per-domain pull cursors (`last_pulled_config_seq`, `last_pulled_decision_seq`). Applies remote-owned changes locally in order.
- **Stage publisher:** On pipeline stage completion, writes all stage outputs with shared `stage_run_id`, updates `process_stage_status` and advances `process_stage_heads` — all in the same transaction.
- **Full republish / remote rebuild mode:** For recovery scenarios — republishes all current local-owned state to remote, or bootstraps local from remote projections.
- **Schema epoch guard:** On startup and before each sync batch, validates `schema_epoch` compatibility between local and remote. Pauses sync (but not local operation) on mismatch.

#### Component: Relation Resolution Engine

**Responsibility:** Classify notice-pair relationships as `merged_same_process`, `related_not_merged`, `possible_duplicate`, or `separate`. Uses OCID matching, explicit notice references, hard blockers, and weighted heuristic scoring. Maintains `ProcessRelation` edges between related-but-unmerged processes to enable downstream cache sharing. See §4.2 for relation semantics and §5.6 for the resolution algorithm.

#### Component: Preliminary Qualification Engine

**Responsibility:** Reject near-expiry, non-goods, and non-bidable opportunities as early as possible using multi-dimensional classification. Evaluates four dimensions per lot:

- `legalCategory` — goods / services / works / mixed / unknown (from CPV codes and notice text).
- `goodsDensityScore` — proportion of the opportunity that is physical goods (0.0–1.0).
- `goodsSeparability` — whether goods can be bid on separately from bundled services/works.
- `accessRoute` — open tender / restricted / framework call-off / DPS / direct award / not-bidable (see §4.2).

Deterministic rules run first (CPV-based heuristics, expiry check, access route check); the `preliminary-qualification` skill is only used for mixed/unknown cases that need AI assistance.

#### Component: Document Acquisition Engine

**Responsibility:** After preliminary qualification, acquire publicly accessible tender documents associated with qualified notices/lots. Targets: specification PDFs, pricing schedules, lot documents, BoQ spreadsheets, tender appendices. The `document-acquisition` skill navigates to the notice source and downloads available public documents. Stores documents as raw artifacts linked to the notice. Without this stage, product extraction operates on vague notice summaries and produces low-quality candidates.

#### Component: Lot Extraction Engine

**Responsibility:** Extract lot structure from normalized notices and acquired documents. Each lot becomes a `NoticeLot` entity with its own CPV codes, estimated value, deadline, and scope description. If a notice has no explicit lots, create a synthetic whole-notice lot. Lot extraction is primarily deterministic (parsing structured OCDS lot data) with AI assist for unstructured lot descriptions in documents.

#### Component: Product Extraction Engine

**Responsibility:** Convert lot-level descriptions and acquired documents into structured `ProductCandidate` entities. Each candidate gets:

- A `candidateType` classification: `exact_sku`, `standard_commodity`, `brand_family`, `bundle`, `custom_or_fabricated`, `installed_goods`, `regulated_goods`, `unknown`.
- Structured slots: `headNoun`, `criticalAttributes`, `numericSpecs`, `standards`, `material`, `brandModel`.
- Separate confidence scores: `identityConfidence` (how well the product is identified), `specConfidence` (how complete the specification is), `quantityConfidence` (how reliable the quantity estimate is).

Bundles are decomposed into constituent candidate families before downstream processing.

#### Component: Quoteability Scorer

**Responsibility:** Determine whether each `ProductCandidate` is worth sending to marketplace search. Uses a deterministic rules-based score:

```text
quoteabilityScore =
  + commodity-like head noun present
  + critical attributes present for the category
  + numeric specs present where category needs them
  + standards/certifications present where category needs them
  + quantity/unit or pack basis present
  + public tender docs available
  + open/bidable access route
  - custom fabrication
  - installation-dominant scope
  - closed framework/call-off
  - regulated goods with missing compliance detail
```

Policy thresholds:

- `>= 0.65` → search automatically
- `0.45–0.64` → search only top-value/top-risk candidates
- `< 0.45` → skip marketplace search; route to watch/RFQ/manual

This gate saves more pipeline cost than most infrastructure optimizations.

#### Component: Supplier Quote Engine

**Responsibility:** Trigger agent skills for supplier price scraping via adapter-mediated sessions, routed by candidate type (see §4.3 source routing table). In practice these runs are usually PTY-backed because marketplace search is browser-driven, but the component consumes the adapter abstraction rather than a PTY-only contract. The agent navigates to marketplace websites using source-specific skills, searches using structured multilingual query packs (not raw notice text), and extracts pricing data. Produces two separate confidence scores per quote: `matchConfidence` (product identity) and `quoteUsabilityConfidence` (commercial usability of the observed price). Applies deterministic hard-conflict filters and shortlisting before any AI adjudication of borderline matches.

#### Component: Landed-Cost Engine

**Responsibility:** Combine ex-works or seller price with freight, duty, VAT, fees, and FX to produce gross and net margin estimates at the **line-item level**, then roll up to process-level estimates. Uses cost-estimation APIs (HMRC Tariff, HMRC FPO, Freightos, SeaRates, Xe, HMRC Exchange Rates) — this is the one area where direct API access is preferred over agent scraping.

The engine operates at two grades:
- **Screening grade:** Automated across the whole market. Uses indicative marketplace pricing, FPO-generated HS6/CN8 candidates, and scenario-based freight estimates. Sufficient for early-stage filtering and gross margin gating.
- **Decision grade:** Automated only for repeated product families where the system has a proven codebook (confirmed 10-digit codes, cached ATRs/AVRs), confirmed supplier economics (RFQ or negotiated quotes), and known freight patterns. Everything else requires operator review or RFQ before final qualification.

Key subsystems:
- **Classification pipeline:** Description preprocessing → FPO candidate generation → 10-digit resolution → measure risk scanning
- **Revenue normalizer:** Convert notice values to expected addressable revenue on an ex-VAT basis, accounting for framework ceilings, dynamic-market structures, and VAT-inclusive notice values
- **Quote normalizer:** Normalize supplier quotes to a common cost boundary based on Incoterm before comparison
- **Freight estimator:** Provider-abstracted, mode-aware, with dual packing scenarios and low/base/high estimates
- **Customs engine:** Duty, trade remedies, VAT treatment, valuation method, special procedures, operational blockers
- **FX engine:** Dual-rate (customs + commercial), basis gap tracking, margin-triggered recalculation

#### Component: Manifest Generator

**Responsibility:** Produce human-readable Markdown summarizing the opportunity, assumptions, evidence, and recommendation. The canonical structure lives in `packages/manifests`; the `manifest-generation` skill can supply narrative synthesis, but it does not replace the deterministic manifest renderer.

#### Component: Bid Workflow Executor

**Responsibility:** Launch portal-specific agent skills via adapter-mediated sessions for request pipeline automation. Portal workflows are normally PTY-backed because they are browser-driven, but the daemon still enters through the adapter boundary so lifecycle, health, and result validation stay consistent with the rest of the runtime. Note: this automates the request pipeline, not real-money bid submissions. Actual negotiations and fulfillment are handled manually by humans.

#### Component: Error Manifest System

**Responsibility:** Track and flag pipeline failures with structured error-manifest records. For each failure, records: which workflow step failed, the error message, the pipeline stage, enough context to debug. Generates error manifest files (Markdown) and surfaces errors in the desktop control panel and web viewing portal. Error manifests include CLI vendor + version, adapter version, model id, config hash, skill version, pid/pgid, cleanup verdict, and last valid checkpoint sequence.

#### Component: Agent Adapter Layer

**Responsibility:** Vendor-specific agent launch, capability probing, auth probing, mode selection (PTY vs. contract), protocol parsing, and result validation for Claude and Codex CLIs. Each adapter produces a canonical daemon event stream (`SessionStarted`, `CheckpointWritten`, `AuthChallenge`, `SessionStalled`, `CleanupIncomplete`, `FinalResultValidated`). Unknown CLI versions enter canary mode until the adapter proves them safe. The adapter isolates the rest of the daemon from vendor CLI changes.

#### Component: Result Contract Validator

**Responsibility:** Validate agent checkpoints and final result artifacts against the skill's declared `result_schema_version`. Enforces the rule: **success = valid final result contract + clean process exit + complete cleanup/reap.** Partial results with valid checkpoints before a crash are classified as `partial_failure`, never upgraded to success. Validates schema version, job identity, checksum, and checkpoint sequencing.

#### Component: Resource Arbiter

**Responsibility:** Admit agent jobs using typed resource budgets instead of a flat session cap. Manages three quota types: **browser slots** (hard cap — each browser-backed session consumes a slot), **agent slots** (soft cap — total concurrent adapter-mediated sessions), and **source/domain lanes** (rate cap — prevent overwhelming a single target). Admission priority is: (1) procurement ingestion, (2) auth refresh / health probes, (3) qualification, (4) supplier enrichment, (5) manifest generation, (6) bid automation.

**MVP operating limits:** Start with `browser_slots = 2`, `agent_slots = 4`, `procurement_source_lane = 1` per source, and `supplier_source_lane = 2` per source. Skill-level `max_concurrency` can lower these ceilings but never raise them.

#### Component: Artifact Store

**Responsibility:** Store immutable compressed raw session-output streams, result contract files, checkpoint artifacts, and large session outputs outside of hot SQLite tables. SQLite stores session metadata, byte counts, tail excerpts, artifact paths, SHA-256 hashes, and derived event summaries. Raw artifacts are chunked, compressed, and stored on the local filesystem at `artifacts/sessions/YYYY/MM/<session_id>/`.

#### Component: Session Reaper

**Responsibility:** Descendant process-tree cleanup after session completion or failure. Performs kill escalation (in-band interrupt → SIGTERM group → SIGKILL group), PTY drain, explicit wait/reap, and orphan sweep. Records cleanup verdict (`clean`, `forced`, `leaked`) as a first-class session attribute. On daemon startup, sweeps for orphaned sessions from prior runs and cleans up leaked processes. On Linux, optionally leverages `systemd-run --scope` for per-session resource control.

#### Architectural Coherence Notes

- **Component purpose / merge guidance:** Every component remains justified, but some are intentionally layered rather than fully standalone. `Skill-Based Ingestion Runner` is best understood as a procurement-specific job handler under `Job Runner`, not a parallel execution stack. `Manifest Generator` remains separate from the `manifest-generation` skill because deterministic rendering is authoritative and the skill contributes optional narrative synthesis only. `Log/Trace Capture Engine` and `Artifact Store` stay separate because capture decides what to record while storage decides where/how to retain it.
- **Qualification vs extraction boundary:** Preliminary Qualification decides whether a notice/lot is commercially worth deeper work using expiry, access route, and goods-shape signals. Product Extraction begins only after a notice survives that gate and documents/lots are available; it does not decide bidability.
- **Scheduler vs runner boundary:** Daemon Cron Scheduler decides when work should be enqueued. Job Runner claims and executes queued work. This keeps schedule policy separate from runtime execution and retry behavior.
- **Store vs sync boundary:** SQLite Local Store is the local persistence authority. Sync Engine is the only component that translates local rows into remote projections or pulls remote-owned state back down.
- **Dependency direction:** Skills are passive assets loaded by the daemon/runtime; the daemon does not depend on skill internals for state management. The web portal depends only on Supabase projections, never on local SQLite. The landed-cost engine depends on upstream candidate/quote outputs, while upstream extraction and qualification do not depend on landed-cost outputs. Downstream review states may consume landed-cost outputs, but landed-cost computation itself does not feed back into extraction or matching logic.
- **Coupling / cycle rule:** The intended graph is acyclic for automatic processing: ingestion → qualification → documents/lots → product extraction → quoteability → supplier matching → landed cost → manifest/review. Human/operator actions may trigger reruns, but no automatic component should depend on a downstream component that already depends on it.
- **Complexity allocation:** The architecture intentionally spends the most component detail on the two hardest problems: PTY/agent orchestration (`PTY Session Manager`, `Agent Adapter Layer`, `Log/Trace Capture Engine`, `Resource Arbiter`, `Session Reaper`) and product/pricing comparability (`Product Extraction Engine`, `Quoteability Scorer`, `SearchProfile`, `Supplier Quote Engine`). The landed-cost engine remains Very High complexity, but it is deliberately kept downstream and API-driven so it does not distort the core orchestration boundaries.

### 5.3 Canonical Workflow

```text
new notice
  -> normalize
  -> relation resolution (merge | related_not_merged | possible_duplicate | separate)
  -> preliminary qualification (per lot)
     -> reject if expiry < 30 days (exception: DPS/dynamic market notices)
     -> classify access route (open_tender / restricted / framework_call_off / dynamic_market / direct_award / closed_framework_award / not_bidable)
     -> reject if not-bidable access route
     -> classify goods dimensions:
        - legalCategory (goods / services / works / mixed / unknown)
        - goodsDensityScore (0.0–1.0)
        - goodsSeparability (true / false / uncertain)
     -> reject if pure services or works with negligible goods density
     -> mixed/unknown -> AI classification via skill
  -> acquire public documents (spec PDFs, pricing schedules, BoQ, lot docs)
  -> extract lots (or create synthetic whole-notice lot)
  -> extract product candidates (per lot)
     -> classify candidateType (exact_sku / standard_commodity / brand_family / bundle / custom_or_fabricated / installed_goods / regulated_goods / unknown)
     -> decompose bundles into constituent candidate families
     -> extract structured slots: headNoun, criticalAttributes, numericSpecs, standards, material, brandModel
  -> score quoteability (deterministic rules-based gate)
     -> >= 0.65: search automatically
     -> 0.45–0.64: search only top-value/top-risk candidates
     -> < 0.45: skip marketplace search; route to watch/RFQ/manual
  -> build structured search profiles (multilingual query packs, not raw text)
  -> price-signal matching (via agent skill scraping, routed by candidate type)
     -> deterministic shortlist: parse results, reject hard conflicts, score top-10
     -> AI adjudication: only top-3 borderline candidates per search
     -> score matchConfidence and quoteUsabilityConfidence separately
     -> classify priceSignalRole (benchmark_ceiling / optimistic_floor / executable)
  -> gross margin estimate (normalized ex-VAT revenue vs normalized supplier cost)
     -> if gross_margin >= 30% and gross_estimate_confidence >= confidence_floor, continue
     -> if gross_estimate_confidence < confidence_floor, route to review
     -> otherwise reject or deprioritize
  -> shipping + customs + FX (landed-cost engine)
  -> net margin band estimate
     -> if netMarginLowPct >= 20% and no critical blockers, qualify
     -> if netMarginBasePct >= 20% but pricing or blocker uncertainty remains, keep as review / RFQ
     -> otherwise keep as non-qualified
  -> generate manifest
  -> operator review
     -> reject
     -> watch
     -> escalate to RFQ (for high-value opportunities with indicative-only pricing)
     -> bid workflow
```

### 5.4 Data Model Design

#### SQLite Compatibility Notes

The primary data store is local SQLite bundled with the Rust daemon. Key differences from Postgres:

- **No native array types:** Use TEXT with JSON arrays (via json1 extension) or junction tables instead of `text[]`.
- **No `gen_random_uuid()`:** Generate UUIDs in the application layer (Rust `uuid` crate).
- **No `timestamptz`:** Store timestamps as TEXT (ISO 8601 format) for readability and portability.
- **No `jsonb`:** Use TEXT with JSON. SQLite's json1 extension provides json_extract(), json_each(), etc.
- **Single-writer model:** SQLite supports one writer at a time — this suits the single-daemon architecture.
- **WAL mode:** Enable WAL (Write-Ahead Logging) for concurrent reads during writes.

The SQL tables below are shown in Postgres syntax for the Supabase sync target. The daemon's SQLite schema mirrors these with appropriate type substitutions.

#### Sync Authority and Change Tracking

The authority matrix (see §4.7) governs which database owns each table. This section specifies the concrete change-tracking columns.

**Push-owned mutable entities** (local → remote projections) carry:

- `owner_executor_id text not null` — which executor produced this row
- `row_version integer not null default 1` — monotonic per row, incremented only by the owner; used for remote apply (NOT `updated_at`)
- `last_local_seq integer` — the outbox sequence that last changed this row (for sync correlation)
- `stage_run_id text` — for stage-produced rows, links all outputs of a single stage execution
- `updated_at text not null` — ISO 8601, for human audit only, never for conflict resolution
- `created_at text not null` — ISO 8601

**Pull-owned entities** (remote → local) carry:

- `remote_change_seq bigint` — monotonic, assigned by remote on every insert/update/delete
- `remote_revision bigint not null default 1` — per-row revision, used for compare-and-swap on shared human decisions
- `deleted_at timestamptz` — tombstone for shared-delete semantics (only on remote-owned entities; see §4.7 Tombstone Policy)

**Local-only entities** (no sync) carry only standard `created_at` / `updated_at` as needed.

**Schema migration coordination:**

- Define a canonical logical schema; generate/map from it into SQLite and Postgres DDL.
- Store `schema_epoch` in SQLite `PRAGMA user_version`, in a Postgres `schema_metadata` table, and in every sync batch envelope.
- On daemon startup, if epochs are incompatible, pause sync but keep local operation.
- Use **expand / migrate / contract** rollouts only — no one-step destructive hot migrations on synced tables.
- In CI: apply both SQLite and Postgres migrations from scratch, introspect both, run a sync round-trip test, fail if logical schema hash differs.

**Monetary values:** Store money as **integer pence** (e.g., `supplier_cost_gbp_pence integer`) and percentages as **basis points** (e.g., `gross_margin_bps integer`) for synced tables. This eliminates floating-point divergence between SQLite and Postgres numeric semantics.

**Array columns:** Use **junction tables** for query-heavy array fields (`cpv_codes`, `candidate_commodity_codes`). Use JSON TEXT only for opaque evidence payloads and raw data where relational querying is not needed.

#### Data Model Notes for Data-Architecture Domain

The following new entities and field changes are required based on the procurement pipeline and product matching review. These should be coordinated with the data-architecture domain:

**New entities required:**

- **`NoticeLot`** — lot-level decomposition of notices. Fields: `id`, `noticeId`, `sourceLotId`, `lotNumber`, `title`, `description`, `cpvCodes`, `estimatedValue`, `deadlineAt`, `isBidableSeparately`, `documents[]`. If no explicit lots, create a synthetic whole-notice lot. `ProductCandidate` should reference `lotId`, not just `noticeId`.
- **`ProcessRelation`** — edges between related-but-unmerged processes. Fields: `id`, `processIdA`, `processIdB`, `relationType` (`merged_same_process` | `related_not_merged` | `possible_duplicate` | `separate`), `confidence`, `evidence`, `createdAt`. Enables cache sharing across related clusters without forced merges.
- **`SearchProfile`** — structured search profile per product candidate. Fields: `id`, `productCandidateId`, `headNoun`, `candidateType`, `criticalAttributesJson`, `standardsJson`, `numericSpecsJson`, `preferredTerms`, `excludedTerms`, `queryPackJson`, `createdAt`. Persisted for debugging and skill improvement.

**Field additions to existing entities:**

- **`ContractNotice`**: add `accessRoute` (`open_tender` | `restricted_tender` | `framework_call_off` | `dynamic_market` | `direct_award` | `closed_framework_award` | `not_bidable`).
- **`ProductCandidate`**: add `lotId`, `candidateType` (`exact_sku` | `standard_commodity` | `brand_family` | `bundle` | `custom_or_fabricated` | `installed_goods` | `regulated_goods` | `unknown`), `headNoun`, `criticalAttributesJson`, `numericSpecsJson`, `standardsJson`, `excludedTermsJson`, `quoteabilityScore`, `identityConfidence`, `specConfidence`, `quantityConfidence`. The existing single `extractionConfidence` should be replaced by the three granular confidence fields.
- **`SupplierQuote`**: add `matchConfidence`, `quoteUsabilityConfidence` (replacing single `confidence`), `priceSignalRole` (`benchmark_ceiling` | `optimistic_floor` | `executable`), `listingPackQty`, `listingPackUnit`, `normalizedUnitBasis`, `isUsedOrRefurbished`, `vatInclusive`, `shippingBasisConfidence`.
  - **Cross-domain note:** The data-architecture domain added `quoteRealismConfidence` (how realistic the observed price is for procurement-volume orders) and `negotiationUpliftPct` (expected uplift from indicative to negotiated price) to the `SupplierQuote` interface. These are **complementary to, not duplicates of**, the procurement pipeline's `quoteUsabilityConfidence` and `priceSignalRole`. The procurement pipeline fields assess the listing's commercial usability (unit basis, MOQ, shipping visibility); the data-architecture fields assess volume-adjusted realism and negotiation gap. Implementers should compute and store all four fields independently.
- **`QualificationResult`**: add `legalCategory`, `goodsDensityScore`, `goodsSeparability` (`true` | `false` | `uncertain`), `accessRoute`. The existing `goodsQualified` boolean should be supplemented (not replaced) by these dimensions.

#### Core Entities

Source notices and canonical opportunities should not be conflated. A single procurement process can have multiple notices across time and, in some cases, across publication surfaces. The source-specific notice model should therefore feed a canonical process/opportunity model.

The TypeScript interfaces below describe the logical application contract. Persisted SQLite/Postgres columns use integer pence and basis points where noted in the SQL schema.

```ts
export interface ProcurementProcess {
  id: string;
  canonicalOcid?: string;
  primaryNoticeId?: string;
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
  estimatedValueOriginal?: number;
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
export interface QualificationResult {
  processId: string;
  noticeId: string;
  expiryQualified: boolean;
  goodsQualified: boolean;
  goodsConfidence: number;
  legalCategory?: "goods" | "services" | "works" | "mixed" | "unknown";
  goodsDensityScore?: number;
  goodsSeparability?: boolean | "uncertain";
  accessRoute?:
    | "open_tender"
    | "restricted_tender"
    | "framework_call_off"
    | "dynamic_market"
    | "direct_award"
    | "closed_framework_award"
    | "not_bidable";
  rationale: string[];
  qualifiedAt: string;
}
```

```ts
export interface ProductCandidate {
  id: string;
  processId: string;
  noticeId: string;
  lotId?: string;
  name: string;
  candidateType?:
    | "exact_sku"
    | "standard_commodity"
    | "brand_family"
    | "bundle"
    | "custom_or_fabricated"
    | "installed_goods"
    | "regulated_goods"
    | "unknown";
  headNoun?: string;
  quantity?: number;
  quantityUnit?: string;
  specificationText?: string;
  estimatedUnitWeightKg?: number;
  estimatedUnitVolumeM3?: number;
  candidateCommodityCodes: string[];
  criticalAttributes?: Record<string, string | number | boolean>;
  numericSpecs?: Record<string, string | number>;
  standards?: string[];
  material?: string;
  brandModel?: string;
  preferredTerms?: string[];
  excludedTerms?: string[];
  quoteabilityScore?: number;
  identityConfidence: number;
  specConfidence: number;
  quantityConfidence: number;
}
```

<!-- COORDINATION NOTE (landed-cost-engine review, 2026-03-12):
     SupplierQuote and LandedCostEstimate fields below were expanded by landed-cost
     domain review. Data-architecture domain should verify SQL table alignment and
     sync-projection design for these additions. -->

<!-- CROSS-DOMAIN NOTE (landed-cost → procurement-pipeline, 2026-03-12):
     The freight mode decision tree (§4.4) depends on ProductCandidate fields that the
     procurement-pipeline domain produces. Specifically, mode selection needs:
     - estimatedUnitWeightKg and estimatedUnitVolumeM3 (already on ProductCandidate)
     - candidateType (from §5.4 data model notes — e.g. bundle triggers decomposition)
     - Packaging state inference (flat-pack vs assembled) — currently derived heuristically
       from candidateType + headNoun, no explicit field
     - Hazmat/oversized/fragile/reefer/battery classification — no explicit field yet
     The landed-cost engine treats missing weight/volume as "unknown" and runs dual
     packing scenarios. If the procurement-pipeline domain adds explicit packingHint
     or handlingRestrictions fields to ProductCandidate, freight mode selection can
     use them directly instead of inferring from text. -->

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
```

```ts
export interface SearchProfile {
  id: string;
  productCandidateId: string;
  headNoun?: string;
  candidateType?: ProductCandidate["candidateType"];
  criticalAttributes?: Record<string, string | number | boolean>;
  standards?: string[];
  numericSpecs?: Record<string, string | number>;
  preferredTerms?: string[];
  excludedTerms?: string[];
  queryPack?: Record<string, string[]>;
  createdAt: string;
}
```

```ts
export interface SupplierQuote {
  id: string;
  processId: string;
  noticeId?: string;
  productCandidateId: string;
  productName?: string;            // denormalized convenience copy for display/debugging
  region: "uk" | "eu" | "china";
  quoteClass: "indicative_marketplace" | "distributor_listing" | "rfq_quote" | "negotiated_quote";
  sourceName: string;
  sourceUrl?: string;
  shipFromCountry?: string;
  countryOfOrigin?: string;           // NEW: actual origin for duty/preference, not dispatch location
  countryOfDispatch?: string;         // NEW: where goods ship from (may differ from origin)
  incoterm?: "EXW" | "FOB" | "CIF" | "DAP" | "DDP" | string;
  quoteCostBoundary?: "EXW" | "FOB" | "CIF" | "CIP" | "DAP" | "DDP" | "UNKNOWN"; // NEW: normalized cost boundary
  quotedQuantity?: number;
  quotedQuantityUnit?: string;
  unitPriceOriginal: number;
  currency: string;
  unitPriceGbp: number;
  minimumOrderQuantity?: number;
  leadTimeDays?: number;
  includesShipping?: boolean;         // legacy hint only; quoteCostBoundary is authoritative
  priceSignalRole?: "benchmark_ceiling" | "optimistic_floor" | "executable";
  listingPackQty?: number;
  listingPackUnit?: string;
  normalizedUnitBasis?: string;
  isUsedOrRefurbished?: boolean;
  vatInclusive?: boolean;
  matchConfidence: number;
  quoteUsabilityConfidence: number;
  shippingBasisConfidence?: number;
  quoteRealismConfidence: number;     // NEW: how realistic is this price for procurement volumes?
  negotiationUpliftPct?: number;      // NEW: expected uplift from indicative to negotiated price
  evidence: string[];
  validUntil?: string;
}
```

```ts
export interface LandedCostEstimate {
  id: string;
  processId: string;
  noticeId?: string;
  productCandidateId?: string;
  supplierQuoteId?: string;                                            // NEW: trace to source quote
  shippingMode: "parcel" | "air_freight" | "lcl_ocean" | "fcl_ocean" | "mixed";
  modeConfidence: number;                                              // NEW: confidence in mode selection
  packingScenario?: "compact" | "bulky" | "unknown";                   // NEW: packing assumption used
  declarableCommodityCode10?: string;                                  // RENAMED: 10-digit declarable code
  candidateCommodityCodes?: CandidateCommodityCode[];                  // NEW: full candidate set
  additionalCodes?: string[];                                          // NEW: 4-char additional codes
  commodityConfidence: number;
  classificationStability: number;                                     // NEW: do candidates converge?
  customsValuationMethod?: "1" | "2" | "3" | "4" | "5" | "6" | "unknown"; // NEW
  customsFxRate: number;                                               // RENAMED: HMRC rate for duty
  commercialFxRate: number;                                            // NEW: Xe rate for margin
  fxBasisDeltaPct: number;                                             // NEW: customs vs commercial gap
  supplierCostGbp: number;
  freightLowGbp: number;                                               // NEW: low scenario
  freightBaseGbp: number;                                              // RENAMED from freightCostGbp
  freightHighGbp: number;                                              // NEW: high scenario
  customsDutyGbp: number;
  tradeRemedyDutyGbp?: number;                                         // NEW: AD/CVD/safeguard
  importVatGbp: number;
  importVatTreatment: "postponed_full" | "postponed_partial"           // NEW: replaces boolean
    | "cash_paid_full" | "cash_paid_partial" | "non_recoverable" | "unknown";
  vatRecoveryRate?: number;                                            // NEW: 0.0-1.0
  ukFinalMileCostGbp: number;
  otherFeesGbp: number;
  economicLandedCostGbp: number;
  cashOutlayGbp: number;
  cashToClearGbp: number;                                              // NEW: total upfront cash
  workingCapitalCostGbp?: number;                                      // NEW: optional
  revenueBasisType: "firm_contract" | "estimated_contract"             // NEW
    | "framework_ceiling" | "dynamic_market" | "concession" | "unknown";
  revenueConfidence: number;                                           // NEW
  netMarginLowPct: number;                                             // NEW: conservative
  netMarginBasePct: number;                                            // NEW: base
  netMarginHighPct: number;                                            // NEW: optimistic
  decisionGrade: "screening" | "review" | "rfq_required" | "broker_required"; // NEW
  confidence: number;
  confidenceBreakdown?: ConfidenceBreakdown;                           // NEW
  warnings: string[];
  blockers: string[];                                                  // NEW: critical blockers
}

export interface CandidateCommodityCode {
  code: string;
  digits: 6 | 8 | 10;
  source: "fpo" | "taric_support" | "hs_ping" | "manual" | "atr";
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
// Pipeline-computed assessment (daemon-owned, push-only)
export interface PipelineAssessment {
  processId: string;
  pipelineState:
    | "discovered"
    | "normalized"
    | "prelim_rejected"
    | "prelim_qualified"
    | "needs_review"
    | "priced"
    | "gross_qualified"
    | "landed_costed"
    | "net_qualified"
    | "expired";
  grossMarginBps?: number;   // basis points (e.g., 1500 = 15.00%)
  netMarginBps?: number;     // basis points
  assessedAt: string;
}
```

```ts
// Human operator disposition (remote-owned, pull-only, compare-and-swap)
export interface OperatorDecisionHead {
  processId: string;
  disposition:
    | "pending"         // no human decision yet
    | "watching"
    | "approved"
    | "rejected";
  decidedBy?: string;
  decidedAt?: string;
  remoteRevision: number;  // for compare-and-swap
}
```

```ts
// Bid workflow state (daemon-owned, push-only)
export interface BidWorkflowState {
  processId: string;
  bidState:
    | "not_started"
    | "ready_to_bid"
    | "bid_in_progress"
    | "bid_submitted"
    | "bid_failed";
  bidJobId?: string;
  startedAt?: string;
  completedAt?: string;
}
```

```ts
// Stage execution tracking (daemon-owned, push-only)
export interface ProcessStageStatus {
  processId: string;
  stage: "ingestion" | "qualification" | "enrichment" | "landed_cost" | "manifest" | "bidding";
  stageRunId: string;
  state: "running" | "succeeded" | "failed";
  startedAt: string;
  completedAt?: string;
  errorManifestId?: string;
}
```

#### Suggested SQL Tables

```sql
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

-- Local runtime copy of source definitions. Populated from shared_source_configs
-- during pull sync. Exists locally so raw_documents, source_cursors, and
-- contract_notices can have local FK references without network dependency.
-- The authoritative version is shared_source_configs (remote-owned, pull-only).
create table sources (
  id text primary key,
  kind text not null,
  enabled boolean not null default true,
  config jsonb not null,
  created_at timestamptz not null default now()
);

create table source_cursors (
  source_id text not null references sources(id),
  cursor_key text not null,
  cursor_value text,
  updated_at timestamptz not null default now(),
  primary key (source_id, cursor_key)
);

-- Single-daemon MVP: one row representing the local daemon.
-- Multi-executor support is a future-consideration extension.
create table executors (
  id uuid primary key default gen_random_uuid(),
  device_name text not null,
  status text not null,
  last_heartbeat_at timestamptz not null default now(),
  -- sync tracking (push-owned: heartbeat summary pushed to remote)
  owner_executor_id text not null,  -- self-referential for consistency
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table procurement_processes (
  id uuid primary key default gen_random_uuid(),
  canonical_ocid text,
  primary_notice_id uuid,
  current_stage text not null,
  buyer_name text,
  title text not null,
  latest_deadline_at timestamptz,
  latest_published_at timestamptz not null,
  latest_updated_at timestamptz not null,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table contract_notices (
  id uuid primary key default gen_random_uuid(),
  process_id uuid references procurement_processes(id),
  source_id text not null references sources(id),
  source_notice_id text not null,
  raw_document_id uuid not null references raw_documents(id),
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
  estimated_value_original_pence integer,  -- integer pence
  estimated_value_currency text,
  estimated_value_gbp_ex_vat_pence integer,  -- integer pence
  estimated_value_vat_inclusive boolean,
  normalized_payload jsonb not null,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now(),
  unique (source_id, source_notice_id)
);
```

```sql
create table qualification_runs (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  expiry_qualified boolean not null,
  goods_qualified boolean not null,
  goods_confidence_bps integer not null,  -- basis points (0-10000)
  legal_category text,            -- from procurement pipeline review
  goods_density_score_bps integer,  -- basis points (0-10000)
  goods_separability text,        -- true, false, uncertain (tri-state classification stored as text)
  access_route text,              -- open_tender, restricted_tender, etc.
  rationale jsonb not null,       -- renamed from 'evidence' to match TS interface
  qualified_at timestamptz not null default now(),
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table product_candidates (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  lot_id uuid references notice_lots(id),
  name text not null,
  candidate_type text,  -- exact_sku, standard_commodity, brand_family, bundle, custom_or_fabricated, installed_goods, regulated_goods, unknown
  head_noun text,
  quantity numeric,
  quantity_unit text,
  specification_text text,
  estimated_unit_weight_kg numeric,
  estimated_unit_volume_m3 numeric,
  -- candidate_commodity_codes: use junction table product_commodity_codes instead of array column
  critical_attributes_json jsonb,
  numeric_specs_json jsonb,
  standards_json jsonb,
  excluded_terms_json jsonb,
  quoteability_score_bps integer,   -- basis points (0-10000)
  identity_confidence_bps integer,  -- basis points (0-10000)
  spec_confidence_bps integer,      -- basis points (0-10000)
  quantity_confidence_bps integer,  -- basis points (0-10000)
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table supplier_quotes (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  product_candidate_id uuid references product_candidates(id),
  product_name text not null,                 -- denormalized convenience copy for display/debugging
  region text not null,
  quote_class text not null,
  source_name text not null,
  source_url text,
  ship_from_country text,
  incoterm text,
  quoted_quantity numeric,
  quoted_quantity_unit text,
  unit_price_original_pence integer not null,  -- integer pence
  currency text not null,
  unit_price_gbp_pence integer not null,       -- integer pence
  minimum_order_quantity numeric,
  lead_time_days integer,
  includes_shipping boolean not null default false,
  -- cross-domain fields (procurement pipeline review)
  price_signal_role text,           -- benchmark_ceiling, optimistic_floor, executable
  listing_pack_qty numeric,
  listing_pack_unit text,
  normalized_unit_basis text,
  is_used_or_refurbished boolean not null default false,
  vat_inclusive boolean,
  match_confidence_bps integer,     -- basis points (0-10000)
  quote_usability_confidence_bps integer,  -- basis points (0-10000)
  shipping_basis_confidence_bps integer,   -- basis points (0-10000)
  -- cross-domain fields (data architecture review)
  quote_realism_confidence_bps integer,  -- basis points (0-10000)
  negotiation_uplift_bps integer,        -- basis points
  evidence jsonb not null,
  valid_until timestamptz,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```


```sql
create table landed_cost_estimates (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid references contract_notices(id),
  product_candidate_id uuid references product_candidates(id),
  supplier_quote_id uuid references supplier_quotes(id),
  shipping_mode text not null,
  mode_confidence_bps integer not null,            -- basis points (0-10000)
  packing_scenario text,
  declarable_commodity_code_10 text,
  additional_codes jsonb,
  commodity_confidence_bps integer not null,        -- basis points (0-10000)
  classification_stability_bps integer not null,   -- basis points (0-10000)
  customs_valuation_method text,
  customs_fx_rate numeric not null,                -- exchange rate (not money — keep numeric)
  commercial_fx_rate numeric not null,             -- exchange rate (not money — keep numeric)
  fx_basis_delta_bps integer not null,             -- basis points
  supplier_cost_gbp_pence integer not null,        -- integer pence
  freight_low_gbp_pence integer not null,          -- integer pence
  freight_base_gbp_pence integer not null,         -- integer pence
  freight_high_gbp_pence integer not null,         -- integer pence
  customs_duty_gbp_pence integer not null,         -- integer pence
  trade_remedy_duty_gbp_pence integer,             -- integer pence
  import_vat_gbp_pence integer not null,           -- integer pence
  import_vat_treatment text not null,              -- postponed_full, postponed_partial, cash_paid_full, cash_paid_partial, non_recoverable, unknown
  vat_recovery_rate_bps integer,                   -- basis points
  uk_final_mile_cost_gbp_pence integer not null,   -- integer pence
  other_fees_gbp_pence integer not null,           -- integer pence
  economic_landed_cost_gbp_pence integer not null,  -- integer pence
  cash_outlay_gbp_pence integer not null,          -- integer pence
  cash_to_clear_gbp_pence integer not null,        -- integer pence
  working_capital_cost_gbp_pence integer,          -- integer pence
  revenue_basis_type text not null,
  revenue_confidence_bps integer not null,          -- basis points (0-10000)
  net_margin_low_bps integer not null,              -- basis points
  net_margin_base_bps integer not null,             -- basis points
  net_margin_high_bps integer not null,             -- basis points
  decision_grade text not null,
  confidence_bps integer not null,                  -- basis points (0-10000)
  confidence_breakdown jsonb,
  warnings jsonb not null,
  blockers jsonb not null default '[]',
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

```sql
create table manifests (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  markdown text not null,
  artifact_path text,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Pipeline-computed assessment (daemon-owned, push-only)
create table pipeline_assessments (
  process_id uuid primary key references procurement_processes(id),
  pipeline_state text not null,  -- discovered, normalized, prelim_rejected, etc.
  gross_margin_bps integer,      -- basis points (1500 = 15.00%)
  net_margin_bps integer,        -- basis points
  assessed_at timestamptz not null default now(),
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Human operator disposition (remote-owned, pull-only, compare-and-swap)
create table operator_decision_heads (
  process_id uuid primary key references procurement_processes(id),
  disposition text not null default 'pending',  -- pending, watching, approved, rejected
  decided_by uuid,
  decided_at timestamptz,
  -- remote-owned tracking
  remote_change_seq bigint,
  remote_revision bigint not null default 1,
  deleted_at timestamptz,  -- tombstone for shared delete semantics
  created_at timestamptz not null default now()
);

-- Bid workflow state (daemon-owned, push-only)
create table bid_workflow_states (
  process_id uuid primary key references procurement_processes(id),
  bid_state text not null default 'not_started',  -- not_started, ready_to_bid, bid_in_progress, bid_submitted, bid_failed
  bid_job_id uuid references workflow_jobs(id),
  started_at timestamptz,
  completed_at timestamptz,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table workflow_jobs (
  id uuid primary key default gen_random_uuid(),
  kind text not null,
  process_id uuid references procurement_processes(id),
  notice_id uuid references contract_notices(id),
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
```

```sql
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
  created_at timestamptz not null default now()
);

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

```sql
-- Remote-facing projection summaries for web visibility.
create table workflow_job_summaries (
  job_id uuid primary key references workflow_jobs(id),
  process_id uuid references procurement_processes(id),
  kind text not null,
  status text not null,
  scheduled_for timestamptz,
  started_at timestamptz,
  completed_at timestamptz,
  last_error text,
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table error_manifest_summaries (
  id uuid primary key references error_manifests(id),
  process_id uuid references procurement_processes(id),
  pipeline_stage text not null,
  workflow_step text not null,
  error_type text not null,
  error_message text not null,
  redacted_context jsonb,
  resolved boolean not null default false,
  resolved_at timestamptz,
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

#### Stage Tracking Tables

```sql
create table process_stage_status (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  stage text not null,
  stage_run_id text not null,
  state text not null,
  started_at timestamptz not null,
  completed_at timestamptz,
  error_manifest_id uuid references error_manifests(id),
  input_revision integer,
  output_revision integer,
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table process_stage_heads (
  process_id uuid not null references procurement_processes(id),
  stage text not null,
  current_stage_run_id text not null,
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  primary key (process_id, stage)
);
```

#### Sync Infrastructure Tables

```sql
-- SQLite-only: local outbox for transactional sync
create table local_outbox (
  local_seq integer primary key autoincrement,
  executor_id text not null,
  entity_kind text not null,
  entity_id text not null,
  op text not null,
  row_version integer not null,
  stage_run_id text,
  schema_epoch integer not null,
  payload_json text not null,
  content_hash text not null,
  created_at text not null
);

-- SQLite-only: tracks push batches
create table sync_batches (
  batch_id text primary key,
  executor_id text not null,
  from_local_seq integer not null,
  to_local_seq integer not null,
  schema_epoch integer not null,
  sent_at text not null,
  ack_at text,
  status text not null default 'pending'
);

-- SQLite-only: pull cursors per remote domain
create table pull_cursors (
  domain text primary key,
  last_pulled_seq bigint not null default 0,
  pulled_at text not null
);

-- Supabase-side: schema version tracking
create table schema_metadata (
  key text primary key,
  value text not null,
  updated_at timestamptz not null default now()
);
```

#### Remote-Owned Tables

```sql
create table shared_company_profile (
  id uuid primary key default gen_random_uuid(),
  profile_data jsonb not null,
  remote_change_seq bigint,
  remote_revision bigint not null default 1,
  deleted_at timestamptz,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table shared_source_configs (
  source_id text primary key,
  kind text not null,
  enabled boolean not null default true,
  config jsonb not null,
  remote_change_seq bigint,
  remote_revision bigint not null default 1,
  deleted_at timestamptz,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

#### Junction Tables

```sql
create table notice_cpv_codes (
  notice_id uuid not null references contract_notices(id),
  cpv_code text not null,
  primary key (notice_id, cpv_code)
);

create table product_commodity_codes (
  product_candidate_id uuid not null references product_candidates(id),
  commodity_code text not null,
  primary key (product_candidate_id, commodity_code)
);
```

#### Cross-Domain Entity Tables

```sql
-- Lot-level decomposition of notices (from procurement pipeline review).
-- If no explicit lots, a synthetic whole-notice lot is created.
create table notice_lots (
  id uuid primary key default gen_random_uuid(),
  notice_id uuid not null references contract_notices(id),
  source_lot_id text,
  lot_number integer,
  title text,
  description text,
  estimated_value_pence integer,  -- integer pence
  deadline_at timestamptz,
  is_bidable_separately boolean not null default true,
  documents jsonb,  -- opaque array of document references
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Junction: lot-level CPV codes
create table lot_cpv_codes (
  lot_id uuid not null references notice_lots(id),
  cpv_code text not null,
  primary key (lot_id, cpv_code)
);

-- Edges between related-but-unmerged processes (from procurement pipeline review).
-- Enables cache sharing across related clusters without forced merges.
create table process_relations (
  id uuid primary key default gen_random_uuid(),
  process_id_a uuid not null references procurement_processes(id),
  process_id_b uuid not null references procurement_processes(id),
  relation_type text not null,  -- merged_same_process, related_not_merged, possible_duplicate, separate
  confidence_bps integer not null,  -- basis points (0-10000)
  evidence jsonb,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Structured search profile per product candidate (from procurement pipeline review).
-- Persisted for debugging and skill improvement.
create table search_profiles (
  id uuid primary key default gen_random_uuid(),
  product_candidate_id uuid not null references product_candidates(id),
  head_noun text,
  candidate_type text,
  critical_attributes_json jsonb,
  standards_json jsonb,
  numeric_specs_json jsonb,
  preferred_terms jsonb,
  excluded_terms jsonb,
  query_pack_json jsonb,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

### 5.5 Contract State Model (Split Authority)

The original single state machine has been decomposed into three separate authority domains to eliminate mixed-write conflicts:

**Pipeline State** (daemon-owned, in `pipeline_assessments`):
```text
discovered
  -> normalized
  -> relation_resolved
  -> prelim_rejected
  -> prelim_qualified
  -> docs_acquired
  -> lots_extracted
  -> products_extracted
  -> quoteability_scored
  -> needs_review
  -> priced
  -> gross_qualified
  -> landed_costed
  -> net_qualified
  -> expired
```

**Operator Disposition** (remote-owned, in `operator_decision_heads`):
```text
pending
  -> watching
  -> approved
  -> rejected
```

**Bid Workflow State** (daemon-owned, in `bid_workflow_states`):
```text
not_started
  -> ready_to_bid
  -> bid_in_progress
  -> bid_submitted
  -> bid_failed
```

**State transition notes:**

- `relation_resolved` is reached after the relation resolution engine classifies the notice's relationship to existing processes (merge / related_not_merged / possible_duplicate / separate).
- `prelim_rejected` is reached when any qualification dimension fails: expiry within 30 days, non-bidable access route, or pure services/works with negligible goods density. The `QualificationResult` records which dimension(s) caused rejection.
- `prelim_qualified` requires passing all four qualification dimensions: expiry, access route, goods classification, and lot-level goods separability. Processes routed to `needs_review` from qualification (e.g., mixed contracts with uncertain separability) remain at `prelim_qualified` with a review flag until the operator resolves them.
- `docs_acquired` → `lots_extracted` → `products_extracted` → `quoteability_scored` form the document-to-candidate pipeline.
- **Below-quoteability path:** Candidates with quoteabilityScore < 0.45 advance to `quoteability_scored` in pipeline state but do NOT enter `priced`. They remain at `quoteability_scored` and the operator disposition defaults to `pending` for operator triage. The operator may then set disposition to `watching`, `rejected`, or escalate to RFQ manually.
- **RFQ escalation** is an operator disposition action (setting disposition to `approved` with a sub-action or note), not a separate pipeline state. The pipeline continues to `priced` if/when executable quotes are obtained.
- `needs_review` is a pipeline state reachable from multiple stages: qualification ambiguity, gross margin below threshold with high uncertainty, or landed-cost blocker (see §5.7 Landed Cost step 15). The `pipeline_assessments` record should include `review_reason` and `review_stage` to distinguish these.
- `expired` is set by a background daemon job that periodically scans active processes and transitions those whose `latestDeadlineAt` has passed. Expiry check runs on daemon startup and on a configurable interval (default: every 6 hours).

### 5.6 Ingestion Data Flow

#### Data Flow: Scheduled Ingestion (Skill-Based Agent Scraping)

1. Daemon's local cron scheduler determines that an ingestion job is due for a source.
2. Scheduler reads the enabled source definition from the Source Registry and enqueues a job with explicit skill id/version binding.
3. Job Runner claims the queued job from local SQLite and asks the Resource Arbiter for the relevant source lane before execution.
4. Agent Adapter selects execution mode (PTY or contract) based on the skill's `interaction_profile` and capability probe results.
5. Adapter spawns the agent session in its own process group, recording pid, pgid, adapter id, CLI version, and config hash.
6. The appropriate procurement ingestion skill is loaded into the agent (e.g., `skills/procurement-ingestion/fts/SKILL.md`).
7. Agent navigates to the source via browser automation, following the skill playbook.
8. Agent extracts notice data from the source, using cursor/watermark from the previous run.
9. Raw session output is captured as an immutable forensic artifact in both modes: PTY bytes in PTY mode, stdout/stderr bytes in contract mode. Structured events are also captured via adapter protocol parsing in contract mode.
10. Agent writes checkpoints and final outputs through a validated result contract (artifact file, JSON payload, or structured stream).
11. Workflow state advances from validated checkpoints/final contracts, not from transcript parsing.
12. Result contract is validated by the Result Contract Validator. Raw artifacts are stored in the artifact store with hashes and paths referenced from SQLite.
13. Notices are normalized into internal schema.
14. Relation resolution engine classifies the notice's relationship to existing processes:
    - **Step 14a: Hard blockers check.** Before scoring, check for hard contradictions that prevent merging:
      - Conflicting exact reference numbers.
      - One notice is framework establishment and the other is a call-off/award under framework.
      - Explicit different OCIDs where source semantics indicate separate processes (e.g., direct award after unsuccessful competition).
      - Incompatible stage chronology.
      - Same buyer and similar title but clearly different lot/value/reference structure.
    - **Step 14b: Exact-match rules.** If exact OCID match → `merged_same_process`. If explicit related-notice reference with plausible stage sequence → `merged_same_process`. If framework/call-off relation detected → `related_not_merged`.
    - **Step 14c: Heuristic scoring** (only for non-blocked, non-exact pairs):
      ```text
      merge_score =
        0.35 * reference_match
        + 0.20 * buyer_match
        + 0.20 * title_similarity
        + 0.10 * cpv_overlap
        + 0.10 * date_proximity
        + 0.05 * value_proximity
      ```
      - `>= 0.95` with no blocker → auto-merge (`merged_same_process`).
      - `0.75–0.94` → `related_not_merged` (or `possible_duplicate` for operator review).
      - `< 0.75` → `separate`.
    - **Step 14d:** For `related_not_merged` pairs, create a `ProcessRelation` edge. This enables sharing search caches, extracted product families, and price signals across the cluster without forcing a false canonical merge.
    - **Step 14e:** Map each notice into a canonical `procurement_processes` record. Merged notices update the existing record; separate notices create new records.
15. Job Runner persists normalized outputs and queues qualification jobs for new or materially updated processes, retaining notice-level provenance.
16. Session Reaper performs descendant cleanup and records cleanup verdict.
17. Stage sync: In the same SQLite transaction that writes stage outputs, the daemon writes all rows with a shared `stage_run_id`, updates `process_stage_status` to `succeeded`, advances `process_stage_heads`, and writes outbox entries. The sync engine then pushes one atomic batch to Supabase. Failed stages publish failure status + redacted error summary but do NOT advance the stage head.

#### Data Flow: Manual Trigger

1. Operator clicks "sync source" in the desktop control panel.
2. Desktop sends trigger command to the daemon.
3. Same flow as scheduled ingestion from step 2 onward.
4. Desktop control panel shows real-time progress via daemon status updates.

#### Error Cases

- Agent stalls (no output) -> daemon detects no PTY/stdout-stderr bytes for configurable timeout, marks `AGENT_STALLED`, kills process group, creates error manifest.
- Agent stalls (no progress) -> adapter detects no validated checkpoint progress despite ongoing output, marks `AGENT_STALLED`, kills process group, creates error manifest.
- Auth challenge -> daemon detects re-login prompt, MFA challenge, browser SSO failure, or hidden permission prompt, marks `AGENT_AUTH_CHALLENGE`, flags for operator.
- Host sleep during session -> daemon detects monotonic time gap exceeding threshold, marks `HOST_SLEEP_INTERRUPTED`, taints browser-backed sessions.
- Source website changed -> skill-based error, marks `AGENT_SKILL_FAILED`, creates error manifest with skill path and step.
- Result contract invalid -> agent output fails schema validation, marks `AGENT_OUTPUT_CONTRACT_INVALID`, preserves transcript + artifact, fails job without transcript-based recovery.
- Partial failure with checkpoint -> valid checkpoint exists before crash, persists checkpoint outputs, marks `AGENT_PARTIAL_FAILURE`, never upgrades to success without final contract + clean exit + reap.
- Agent internal error -> adapter captures error from structured events or terminal output, marks `AGENT_INTERNAL_ERROR`.
- Cleanup failure -> descendant processes leaked after session exit, marks `AGENT_CLEANUP_INCOMPLETE`, records leaked pids for operator attention.
- Resource exhaustion -> host memory/CPU pressure detected, marks `HOST_RESOURCE_EXHAUSTED`, may preempt lower-priority sessions.
- Daemon not running -> jobs remain in local SQLite queue; freshness SLA depends on the daemon being online. On restart, the daemon performs a catchup sweep for overdue jobs.

### 5.7 Intelligence Pipeline Data Flow

#### Data Flow: Preliminary Qualification

1. Read normalized notice and its lot structure (if available from OCDS data).
2. **Expiry check:**
   - Determine deadline and expiry horizon per lot (or notice-level if no lots yet).
   - Reject if deadline is within 30 days.
   - **Exception:** DPS/Dynamic Market notices that accept new supplier applications do not expire in the same way — do not apply the 30-day filter to these.
   - If deadline is missing or cannot be normalized confidently, route to manual review rather than passing automatically.
3. **Access route classification:**
   - Classify the notice's access route: `open_tender`, `restricted_tender`, `framework_call_off`, `dynamic_market`, `direct_award`, `closed_framework_award`, `not_bidable`.
   - Reject `direct_award`, `closed_framework_award`, and `not_bidable` notices immediately.
   - Flag `framework_call_off` and `restricted_tender` for access-route review (bidable only if the operator is on the framework or can pre-qualify).
4. **Multi-dimensional goods classification** (per lot where lots are available, otherwise per notice):
   - **`legalCategory`:** Use CPV-based deterministic lookup first:
     - CPV Divisions 14–44 → `goods`
     - CPV Division 45 → `works`
     - CPV Divisions 50–98 → `services`
     - Special cases: software/licensing CPVs despite goods-like codes → `services`
     - Multiple CPV codes spanning categories → `mixed`
     - No CPV or ambiguous → `unknown`
   - **`goodsDensityScore`** (0.0–1.0): Combine CPV signal with text analysis:
     - Physical noun density in title/description (supply, deliver, provide vs install, maintain, design, manage)
     - Quantity/unit presence
     - Lot titles
     - Attachment filenames (spec sheets, pricing schedules suggest goods)
   - **`goodsSeparability`**: Can the goods component be bid on separately?
     - Explicit separate goods lots → `true`
     - Mixed notice but goods packages identifiable → `uncertain`
     - Goods deeply bundled with services/installation → `false`
   - **`accessRoute`**: Already classified in step 3.
5. **Qualification decision per lot:**
   - **Pass:** Pure goods, or mixed with goods separable and adequate goods density, and open/accessible bid route.
   - **Review:** Mixed/unknown with strong goods density but weak separability, or works notices with identifiable goods packages but unclear award structure.
   - **Reject:** Services only, works with negligible goods density, or closed/non-bidable access route.
6. For mixed contracts with separable goods, price only the goods package — not the whole contract value. Adjust `estimatedValue` to reflect the goods portion.

#### Data Flow: Document Acquisition

1. For each `prelim_qualified` notice, check for publicly accessible tender documents.
2. Daemon creates an adapter-mediated agent session for the `document-acquisition` skill (normally PTY-backed because the flow is browser-driven).
3. Agent navigates to the notice source portal and identifies available documents (spec PDFs, pricing schedules, BoQ spreadsheets, lot documents, tender appendices).
4. Agent downloads publicly accessible documents and outputs structured references.
5. Documents stored as raw artifacts in local SQLite, linked to the notice.
6. If no public documents are available, proceed with notice text only (lower extraction confidence).

#### Data Flow: Lot Extraction

1. Parse OCDS lot data from the normalized notice (if structured lot fields are present).
2. If no structured lots, use the `lot-extraction` skill to extract lot structure from acquired documents.
3. Create `NoticeLot` entities with: lot number, title, description, CPV codes, estimated value, deadline, documents, bidability.
4. If no lots are identifiable, create a synthetic whole-notice lot.
5. Run per-lot qualification if the notice-level qualification was provisional.

#### Data Flow: Product Extraction

1. For each qualified lot, extract `ProductCandidate` entities from lot descriptions and acquired documents.
2. Classify `candidateType`: `exact_sku`, `standard_commodity`, `brand_family`, `bundle`, `custom_or_fabricated`, `installed_goods`, `regulated_goods`, `unknown`.
3. **Decompose bundles** into constituent candidate families before downstream processing.
4. Extract structured slots: headNoun, criticalAttributes, standards, numericSpecs, material, brandModel, preferredTerms, excludedTerms.
5. Use **category-specific critical attribute tables** keyed off CPV family (PPE: standards/material; Electrical: voltage/plug/wattage; Furniture: dimensions/fire-rating; IT: brand/model/specs; Medical: standard/classification/sterile).
6. Assign separate confidence scores: `identityConfidence`, `specConfidence`, `quantityConfidence`.
7. Store candidates in local SQLite linked to their lot.

#### Data Flow: Quoteability Scoring

1. Compute deterministic `quoteabilityScore` per candidate based on: commodity-like head noun, critical attributes, numeric specs, standards, quantity/unit, public docs, open access route; minus penalties for custom fabrication, installation scope, closed framework, missing compliance detail.
2. Thresholds: `>= 0.65` search automatically; `0.45-0.64` search top-value only; `< 0.45` skip marketplace, route to watch/RFQ/manual.
3. Generate structured search profile with multilingual `queryPack` (uk_precise, uk_broad, china_en, china_zh) for passing candidates. Translate only structured product slots, not whole sentences.
4. Store search profiles in local SQLite for debugging.

#### Data Flow: Price-Signal Matching (Skill-Based Agent Scraping)

1. **Source routing** by `candidateType` (see section 4.3). Not every source for every candidate.
2. **Session batching:** Resource Arbiter grants supplier-source lanes; run at most one automated search session per source per notice (multiple candidates batched together).
3. Agent searches using `queryPack` from search profile. Full raw session output is captured.
4. **Deterministic shortlist:** Parse results, reject hard conflicts (wrong brand/model/standard/voltage/material/size, used when not allowed, accessory instead of primary, consumer instead of industrial), score top 10.
5. **AI adjudication (top-3 only):** Agent judges pre-filtered borderline results, does not search.
6. **Dual confidence scoring:** `matchConfidence` (0.40 product_type + 0.30 critical_attribute + 0.20 numeric_spec + 0.10 evidence_quality) and `quoteUsabilityConfidence` (0.35 unit_basis + 0.25 MOQ + 0.20 shipping_visibility + 0.20 completeness). Never average these.
7. **Price-signal role:** `benchmark_ceiling` (UK retail), `optimistic_floor` (China wholesale), `executable` (RFQ/negotiated).
8. Save quotes with dual confidence and price-signal role to local SQLite.
9. **Cache sharing:** Reuse results across related-but-unmerged clusters via `ProcessRelation` edges.
10. Stage sync: write outbox entries in same transaction as quote storage, update stage status/heads, push atomic batch to Supabase.

#### Data Flow: Pipeline Cost Management

Naive design: 1,200+ sessions/day at $0.18-$0.41 = $75-$170/day. Required optimizations:

1. **Batch by source:** One ingestion session per source per run. One supplier session per source per notice (searching multiple candidates).
2. **Search selectively:** Top 2-3 candidates per notice by quoteability/spend share. Skip 1688 unless commodity-type.
3. **Deterministic first:** Hard-conflict filters and shortlisting are deterministic. AI only for top-3 ambiguous.
4. **Cache results:** Normalize query fingerprints, cache 24-72h. Reuse across related process clusters.
5. **Model tiering:** Mini/Haiku for classification/extraction. Premium models only for ambiguous adjudication.
6. **Realistic throughput:** 20-30 priced notices/day after filtering. Target $20-$60/day metered.
7. **Default operating limits (MVP):** start with 2 browser slots, 4 total agent slots, 1 concurrent procurement session per source, 2 concurrent supplier sessions per source, max 25 extracted candidates per notice, max 8 auto-searched candidates per notice, and at most 3 supplier sources searched automatically per product candidate unless an operator forces a wider sweep. Treat marketplace quotes as stale after 72 hours unless `validUntil` is sooner.

#### Data Flow: Landed Cost (API-Based)

**Revenue normalization (before any margin gate):**

1. Determine **revenue basis type**: firm contract, estimated contract value, framework ceiling, dynamic-market admission, or concession. Under Procurement Act guidance, estimated contract value is the maximum value payable and is inclusive of VAT. For frameworks, the estimate is the value of all call-offs and must not be exceeded, but this does not make it guaranteed revenue.
2. Normalize to `expected_addressable_revenue_ex_vat`. Do not use raw notice value — framework ceiling or dynamic-market values are not firm revenue.
3. Assign `revenueConfidence` based on basis type. Firm contracts score higher than framework ceilings.

**Per-line-item estimation (do not force one commodity code onto a mixed basket):**

4. Split mixed notices into separate product candidate line items (bundle splitting should have occurred at the product extraction stage).
5. Select best supplier quote pairs per line item.
6. **Normalize each supplier quote to a common cost boundary based on Incoterm.** Under EXW, buyer pays all transport; under FOB, seller pays to port; under CIF, seller pays freight + insurance; under DDP, seller handles everything including import clearance. Without normalization, the gross-margin gate is mathematically inconsistent across quotes with different Incoterms.
7. **Classification pipeline:** Apply description-quality gate → attribute extraction → multi-query **HMRC FPO API** for HS6/CN8 candidate codes → cluster and rank candidates → resolve to 10-digit declarable codes via **HMRC Trade Tariff API** → validate via **Taric Support API** → cross-check via **HS Ping**.
8. **Measure risk scan:** Query HMRC Trade Tariff API import measures for each resolved 10-digit code. Check for: base duty (including compound/specific), trade remedy duties (anti-dumping, countervailing, safeguard), preferential eligibility, quotas, import controls/prohibitions, additional codes, condition thresholds, and required measurement units.
9. **Freight estimation:** Resolve Incoterm cost boundary → apply hard mode locks (hazmat, oversized, fragile, reefer, batteries) → infer packaging state → select mode(s) → fetch estimates from **Freightos** and **SeaRates** APIs → if packaging state unknown, run dual scenarios → emit `freightLowGbp` / `freightBaseGbp` / `freightHighGbp` with `modeConfidence`.
10. Convert to GBP using:
    - **HMRC Exchange Rate Service** for customs duty calculation (legally authoritative, `customsFxRate`),
    - **Xe** for commercial margin estimation (real-time market rates, `commercialFxRate`).
    - Compute and store `fxBasisDeltaPct` between the two rates.
11. Determine **import VAT treatment**: postponed (full/partial), cash-paid (full/partial), or non-recoverable. Assign `vatRecoveryRate` rather than a binary flag. Record `customsValuationMethod` used (Method 1–6 or unknown).
12. **Estimate remaining cost components:**
    - `ukFinalMileCostGbp` — UK domestic delivery from port/airport to contract delivery address. Use internal rate cards, carrier quotes, or heuristic by weight/volume and distance.
    - `otherFeesGbp` — customs brokerage fees (~£30–150 per entry), port/terminal handling charges (~£150–400 per container), insurance (typically 0.5–2% of CIF value), quality inspection if applicable.
13. **Compute landed cost and margin scenarios** per line item:
    - `economicLandedCostGbp` = supplierCost + freightBase + customsDuty + tradeRemedyDuty + ukFinalMile + otherFees (ex-recoverable-VAT)
    - `netMarginLowPct`, `netMarginBasePct`, `netMarginHighPct` (economic, ex-VAT basis, using freight low/base/high and duty variation from classification uncertainty),
    - `cashToClearGbp` (total upfront cash including non-recoverable or delayed-recovery VAT),
    - `cashOutlayGbp` (total cost including all VAT whether recoverable or not),
    - `workingCapitalCostGbp` (optional — cost of capital tied up between payment and revenue).
14. **Compute confidence breakdown:** Calculate each of the 6 components (`classificationStability`, `quoteRealism`, `commodityCodeConfidence`, `freightModeStability`, `revenueReliability`, `fxFreshness`) and the weighted composite. Assign `decisionGrade`: `screening` | `review` | `rfq_required` | `broker_required`.
15. **Blocker check:** Stop auto-qualification when:
    - Classification is unstable (HS6 candidates diverge or 10-digit code unresolved)
    - Revenue basis is uncertain (framework ceiling, dynamic market, no firm commitment)
    - Freight mode remains unresolved or mode confidence is low
    - Trade remedy duties detected
    - Unsatisfied measure conditions or import controls present (operational blockers)
    - Quote class is only `indicative_marketplace` and margin is within 10 percentage points of gate
16. Roll up line-item estimates to process-level aggregate estimates.
17. Store warnings, confidence breakdown, blockers, and decision grade in local SQLite.
18. Sync results to Supabase.

### 5.8 Control Panel and Viewing Portal Data Flow

**Terminology note:** In this document, **desktop app**, **desktop shell**, and **desktop control panel** all refer to the same writable Tauri client. **Web app**, **web shell**, and **viewing portal** all refer to the same read-only hosted client. **Procurement process** is the canonical internal record; the UI may call it an **opportunity**. A **contract notice** is the source document that feeds a procurement process.

The desktop/web capability split is formalized in **ADR-008**.

#### Data Flow: Desktop Control Panel (Full Read/Write/Trigger)

1. Operator opens the Tauri desktop app.
2. Desktop queries canonical procurement processes by state **from local SQLite** (fast, no network dependency). All desktop writes go through the daemon; desktop SQLite connections are read-oriented.
3. Operator opens opportunity detail.
4. Detail view loads:
   - canonical process summary,
   - normalized source notices,
   - source links,
   - pricing evidence,
   - logistics/customs assumptions,
   - manifest,
   - agent session history and terminal output traces,
   - error manifest entries (if any).
5. Operator chooses:
   - reject,
   - watch,
   - rerun enrichment (triggers the appropriate daemon-managed agent session),
   - start bid workflow (triggers the portal skill via a daemon-managed agent session).
6. Operator can also:
   - trigger manual source syncs,
   - manage agent configurations and skills,
   - view comprehensive agent tracing logs,
   - view and resolve error manifests.

#### Data Flow: Web Viewing Portal (Read-Only)

1. Viewer opens the web viewing portal.
2. Web queries canonical procurement processes by state **from Supabase projection tables** (designed for portal use-cases, not raw sync tables), plus `workflow_job_summaries` and `error_manifest_summaries` for operator-safe status visibility. Displays sync age and last replicated seq for staleness awareness.
3. Viewer opens opportunity detail.
4. Detail view loads:
   - canonical process summary,
   - normalized source notices,
   - source links,
   - pricing evidence,
   - logistics/customs assumptions,
   - manifest,
   - pipeline status,
   - error status.
5. **No write operations.** Portal remains read-only in MVP; do not build generic cross-device edit sync before a second writable desktop exists. The web portal does not support:
   - reject/bid/rerun controls,
   - agent management,
   - configuration changes.

These are desktop-only operations.

### 5.9 Request Pipeline Automation Flow

**Important clarification:** This section describes **request pipeline automation**, not real-money bid submissions. The platform automates the preparation and submission of requests/expressions of interest. Actual negotiations and fulfillment are handled manually by humans using the pipeline output. The cost of pipeline failure is low — no real money changes hands automatically.

This automation-risk posture is formalized in **ADR-007**.

In many cases the notice source and the submission portal are different. `Find a Tender` and `Contracts Finder` are discovery/publication sources, while the actual bidder interaction often happens on downstream eSourcing portals such as Jaggaer, Atamis, Proactis, PCS-Tender, or authority-specific procurement systems. Bid workflows should therefore be modeled around the **underlying submission portal**, not the notice discovery source.

```text
operator clicks "bid" in desktop control panel
  -> create bid job in local SQLite
  -> daemon creates adapter-mediated agent session (normally PTY for portal/browser flows)
  -> portal-specific skill loaded
  -> company profile injected into kickoff prompt
  -> agent navigates to downstream portal via browser automation
  -> agent fills portal forms using skill playbook
  -> draft submission or request-for-bid prepared
  -> evidence/screenshot/log artifacts stored in local SQLite
  -> full raw session output captured for tracing
  -> job marked ready_to_bid, needs_review, or bid_submitted depending on workflow stage
  -> results synced to Supabase for visibility
  -> if failure: error manifest entry created with context for debugging
```

### 5.10 API Design

#### New Endpoints

| Endpoint | Method | Purpose | Auth |
|----------|--------|---------|------|
| `/api/contracts` | GET | Search/browse canonical opportunities | user |
| `/api/contracts/:processId` | GET | Opportunity detail | user |
| `/api/contracts/:processId/manifest` | GET | Read generated manifest | user |
| `/api/contracts/:processId/reject` | POST | Reject an opportunity | user (desktop only) |
| `/api/contracts/:processId/watch` | POST | Move to watch state | user (desktop only) |
| `/api/contracts/:processId/bid` | POST | Trigger request pipeline workflow | user (desktop only) |
| `/api/jobs/sync-source` | POST | Trigger manual source sync | user/admin (desktop only) |
| `/api/jobs/:id` | GET | Job status/log summary | user/admin |
| `/api/company-profile` | GET/PUT | Manage bidder company profile | admin (desktop only) |
| `/api/sources` | GET/PUT | Manage source configs | admin (desktop only) |

#### Request / Response Example

```json
POST /api/contracts/123e4567-e89b-12d3-a456-426614174000/bid
{
  "portalWorkflow": "proactis-v1",
  "notes": "Proceed with standard company profile and attach existing sourcing evidence."
}
```

```json
{
  "jobId": "2ac6d0ac-f7e4-47e6-9a33-d1da267f4e1c",
  "status": "queued",
  "processId": "123e4567-e89b-12d3-a456-426614174000"
}
```

### 5.11 Error Handling Strategy

#### Pipeline Error Types

| Error | Code | Handling |
|-------|------|----------|
| Source website changed | `SOURCE_CHANGED` | flag skill needs update, create error manifest |
| Notice parse failed | `NOTICE_PARSE_FAILED` | store raw payload, mark manual review, create error manifest |
| Goods classification uncertain | `GOODS_CLASSIFICATION_LOW_CONFIDENCE` | keep notice, require review |
| Supplier match weak | `SUPPLIER_MATCH_LOW_CONFIDENCE` | do not auto-qualify |
| Commodity code uncertain | `COMMODITY_CODE_LOW_CONFIDENCE` | route to manual review |
| Freight estimate weak | `LOGISTICS_LOW_CONFIDENCE` | use manual quote/broker escalation |
| FPO API unavailable | `FPO_ACCESS_UNAVAILABLE` | skip FPO, use manual classification or Taric Support lookup |
| HS10 code unresolved | `HS10_UNRESOLVED` | cannot determine declarable 10-digit code; block auto-qualification, escalate to broker |
| Additional code required | `ADDITIONAL_CODE_REQUIRED` | 10-digit code requires additional 4-char codes for declaration; block auto-qualification |
| Measure condition unsatisfied | `MEASURE_CONDITION_UNSATISFIED` | trade tariff measure condition cannot be resolved; operational blocker |
| Trade remedy risk | `TRADE_REMEDY_RISK` | anti-dumping/countervailing/safeguard duty detected; flag for review, add to duty estimate |
| Revenue basis uncertain | `REVENUE_BASIS_UNCERTAIN` | framework ceiling or dynamic market; margin gate uses discounted revenue |
| Incoterm cost overlap | `INCOTERM_COST_OVERLAP` | supplier quote Incoterm includes costs also being added separately; double-counting risk |
| PVA ineligible | `PVA_INELIGIBLE` | importer cannot use postponed VAT accounting; import VAT is real cash cost |
| Freight provider unsupported | `FREIGHT_PROVIDER_UNSUPPORTED` | Freightos/SeaRates cannot quote this route/mode; use manual freight estimate |
| Freight mode low confidence | `FREIGHT_MODE_LOW_CONFIDENCE` | packaging state unknown or weight/volume too uncertain for reliable mode selection |

#### Agent-Specific Error Types

| Error | Code | Handling |
|-------|------|----------|
| Auth challenge | `AGENT_AUTH_CHALLENGE` | Agent needs re-login, MFA, browser SSO, or permission grant. Flag for operator. Desktop control panel shows alert. Distinct from stall — the agent is blocked on auth, not frozen. |
| Agent internal error | `AGENT_INTERNAL_ERROR` | API error inside Claude/Codex. Store full session artifacts. Create error manifest with trace. |
| Agent stalled | `AGENT_STALLED` | Agent stopped producing output or validated progress. Daemon detects via configurable timeout. Kill process group, create error manifest. |
| Agent skill failed | `AGENT_SKILL_FAILED` | Skill execution failed within agent (e.g., website layout changed). Store session artifacts. Create error manifest with skill path, step, and skill version. |
| Agent workflow failed | `AGENT_RUN_FAILED` | General agent failure not classifiable as a more specific type. Retry if safe; store logs/screenshots. Create error manifest. |
| Result contract invalid | `AGENT_OUTPUT_CONTRACT_INVALID` | Agent output failed schema validation against declared `result_schema_version`. Preserve raw transcript + artifact. Fail job without transcript-based recovery. |
| Protocol mismatch | `AGENT_PROTOCOL_MISMATCH` | Adapter detected unexpected CLI output format or unsupported protocol version. Enter canary mode for the vendor. Create error manifest with CLI version details. |
| Partial failure | `AGENT_PARTIAL_FAILURE` | Valid checkpoints exist before crash but no final result contract. Persist checkpoint outputs. Never upgrade to success. |
| Cleanup incomplete | `AGENT_CLEANUP_INCOMPLETE` | Descendant processes leaked after session exit. Record leaked pids/pgids. Operator may need manual intervention. |

#### Host/Runtime Error Types

| Error | Code | Handling |
|-------|------|----------|
| Host sleep interrupted | `HOST_SLEEP_INTERRUPTED` | Host suspended during active session. Detected via monotonic time gap. Taint browser-backed sessions. May require job re-run. |
| Host resource exhausted | `HOST_RESOURCE_EXHAUSTED` | Host memory/CPU pressure detected by resource arbiter. May preempt lower-priority sessions. Create error manifest with resource metrics. |

#### Error Manifest System

Every pipeline and agent failure produces:

1. an entry in the `error_manifests` table, and
2. a local Markdown artifact at `artifacts/error-manifests/YYYY/MM/<error_manifest_id>.md`.

Each error manifest contains:

- **Pipeline stage:** Which stage failed (ingestion, qualification, enrichment, landed_cost, bidding).
- **Workflow step:** The specific step within the stage.
- **Error type and message:** What went wrong.
- **Error context:** Enough information to debug — including skill path, agent session ID, relevant data.
- **Runtime context:** CLI vendor + version, adapter version, model id, config hash, skill version, pid/pgid, cleanup verdict, last valid checkpoint sequence.
- **Markdown artifact:** Human-readable header, pipeline summary, redacted context, linked local session artifact path, and next debugging action.
- **Resolution tracking:** Whether the error has been resolved, by whom, and when.

The desktop control panel provides an error manifest view where operators can:

- See all unresolved errors.
- Filter by pipeline stage, error type, and time.
- Drill into error details including session artifacts (forensic PTY capture or structured event log).
- Mark errors as resolved.

Supabase receives the error-manifest metadata plus redacted context so the web viewing portal can show error status without receiving full raw session transcripts.

#### Error Handling Example

```ts
type ConfidenceGate =
  | "pass"
  | "manual_review"
  | "reject";

function resolveConfidenceGate(score: number, floor = 0.6): ConfidenceGate {
  if (score >= floor) return "pass";
  if (score >= 0.45) return "manual_review";
  return "reject";
}
```

---

## 6. Implementation Considerations

### 6.1 Complexity Analysis

**Overall Complexity:** Very High

**Complexity Breakdown:**

| Component | Complexity | Reason |
|-----------|------------|--------|
| Procurement ingestion | Medium | skill-based scraping of structured official sources; incremental retrieval well-supported |
| Notice normalization | Medium | OCDS helps, but not all sources are identical |
| Relation resolution (merge / related_not_merged / possible_duplicate / separate) | High | OCID matching is straightforward; heuristic scoring for cross-source pairs without shared OCID is harder; framework/call-off detection requires notice-type semantics |
| Goods qualification (multi-dimensional) | Medium-High | CPV heuristics cover most cases; mixed contracts with separable goods require more nuanced analysis; access route classification adds a new dimension |
| Document acquisition | Medium-High | navigating procurement portals to find and download public documents; handling varied document formats |
| Lot extraction | Medium | structured OCDS lot data is straightforward; unstructured lot descriptions in documents require AI assist |
| Product candidate identity extraction | High | noisy natural language, quantities often ambiguous; bundle decomposition; candidate typing |
| Quoteability scoring | Medium | deterministic rules-based; main complexity is tuning the scoring weights per category |
| Marketplace comparability scoring | Very High | bridging procurement text to marketplace listings; cross-language matching; category-specific attribute matching; hard-conflict detection |
| Commercial usability of observed prices | High | pack/unit basis normalization; MOQ compatibility; VAT/shipping inclusion detection; price-signal role classification |
| RFQ escalation strategy | Medium | identifying when indicative pricing is insufficient and executable quotes are needed |
| Landed-cost engine | Very High | logistics + customs + FX + assumptions; expanded API surface; revenue normalization |
| Operator surfaces | Medium | standard CRUD/search/detail flows; desktop control panel + web read-only viewing portal |
| Request pipeline automation | High | portal-specific skills, brittle websites, but low risk since not real money |
| Daemon orchestration (PTY + SQLite + sync) | Very High | PTY session management, SQLite storage, Supabase sync engine, cron scheduling, agent health monitoring, full session-output capture |
| Skill maintenance | Medium | skills need updating when source websites change layout/flow |

**Architecture-shape note:** The component graph is intentionally denser around PTY/runtime orchestration and product/pricing comparability than around generic CRUD or projection concerns. That is deliberate: those two areas dominate failure risk, operational cost, and correctness.

**Risk Areas:**

1. Document/attachment acquisition — without tender documents, product extraction operates on vague summaries and produces garbage candidates. This is the biggest silent failure mode.
2. Product matching between contract descriptions and marketplace listings — now decomposed into identity extraction, comparability scoring, and price-signal usability.
3. Lot blindness — without lot structure, mixed contracts and framework notices contaminate each other.
4. Commodity-code accuracy and duty/VAT correctness.
5. Freight-mode selection and cost assumptions for large contracts.
6. PTY session reliability and agent health monitoring.
7. Skill drift when source websites change — mitigated by the error manifest system.
8. SQLite-to-Supabase sync consistency.

### 6.2 Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Wrong product match inflates margin | High | High | confidence scoring + evidence + manual review |
| Incorrect notice-to-process merging conflates separate opportunities | Medium | High | conservative merge heuristics + merge confidence + manual unmerge tooling |
| HS code is wrong | Medium | Critical | HMRC FPO first-pass + candidate-code ranking + ATR workflow + broker escalation |
| Freight estimate too optimistic | High | Critical | mode-aware engine + Freightos + SeaRates cross-check + manual quote threshold |
| VAT recoverability is modeled incorrectly | Medium | High | store VAT-registration assumptions explicitly and show economic-margin vs cash-outlay views separately |
| Daemon not running for queued work | Medium | Medium | local cron + error manifest flags missed schedules + at least one designated always-on machine |
| Source website layout changes break scraping skills | High | Medium | error manifest flags failures + skill versioning + fast skill update cycle |
| Agent auth challenge blocks a run | High | Medium | daemon health monitoring detects `AGENT_AUTH_CHALLENGE` + operator alert |
| Agent stalls mid-execution | Medium | Medium | daemon timeout detection + auto-kill + error manifest |
| SQLite-to-Supabase sync protocol drift / schema mismatch | Low | Medium | `schema_epoch` guard + transactional outbox ordering + sync round-trip tests in CI |
| Portal UI changes break request pipeline skills | High | Low | portal-specific skills + screenshots + health checks. **Impact downgraded:** this automates the request pipeline, not real-money bids |
| FX volatility changes decision outcome | High | Medium | stored rate snapshots + HMRC official rates for customs + recalculation triggers |
| Attachment blindness — extraction from notice text only | High | Critical | document acquisition stage before product extraction; confidence penalty when no docs available |
| Lot blindness — no lot structure in data model | High | High | NoticeLot entity; synthetic lot for notices without explicit lots; per-lot qualification |
| False bidability on framework call-offs | High | Medium | access route classification early in pipeline; reject closed/non-bidable routes before pricing |
| Compliance-critical false positive (regulatory goods matched without standards verification) | Medium | High | regulated_goods candidate type triggers mandatory standards/compliance slot check before match acceptance |
| Pack-basis normalization error (comparing per-unit vs per-box vs per-pallet) | High | High | listing pack qty/unit extraction; normalized unit basis on every quote; quoteUsabilityConfidence penalizes unclear basis |
| Price-signal false precision (single marketplace listing treated as decision-grade pricing) | High | Medium | margin bands not point estimates; priceSignalRole classification; RFQ escalation for high-value opportunities |
| HMRC FPO API access not secured (restricted to invited UK Carrier scheme organisations) | Medium | High | do not make FPO a hard dependency; fallback to CPV-to-HS mapping heuristics + manual classification |

### 6.3 Security Considerations

**Authentication:**

- Web viewing portal users authenticate through Supabase Auth or compatible OIDC (read-only access).
- Desktop operators authenticate locally; the daemon uses a device-scoped token for Supabase sync.
- Local SQLite requires no authentication (protected by OS-level file permissions).
- Agent CLIs use locally managed credentials, never browser-exposed secrets.

**Authorization:**

- RLS for all user-facing tables in Supabase — push-owned projection tables, remote-owned shared tables, and any portal-facing views.
- Do not distribute the Supabase service-role key to desktop clients.
- Prefer a dedicated `daemon` role exposed through RLS-safe RPCs or a thin hosted broker API for privileged operations. Push batches should be applied through a single transactional RPC, not direct table upserts from the daemon client.
- Distinct roles:
  - `viewer` — read-only access to projection tables via web portal
  - `operator` — read access to projections + write access to `operator_decision_heads` (via compare-and-swap RPC)
  - `admin` — full access to shared config tables
  - `daemon` — push-only access to projection tables via batch RPC, pull-only access to remote-owned tables

**RLS policy specifications (Supabase/Postgres):**

All tables scoped by `org_id` (implicit — single-org MVP, explicit column added when multi-org is needed). Policies below use Supabase `auth.role()` and `auth.uid()`.

| Table Category | Tables | viewer | operator | admin | daemon |
|---|---|---|---|---|---|
| Push-owned projections | `procurement_processes`, `contract_notices`, `notice_lots`, `qualification_runs`, `product_candidates`, `search_profiles`, `supplier_quotes`, `landed_cost_estimates`, `manifests`, `process_relations`, `pipeline_assessments`, `bid_workflow_states`, `process_stage_status`, `process_stage_heads` | SELECT | SELECT | SELECT | INSERT, UPDATE (via batch RPC) |
| Push-owned summaries | `workflow_job_summaries`, `error_manifest_summaries`, `executors` | SELECT | SELECT | SELECT | INSERT, UPDATE (via batch RPC) |
| Remote-owned shared config | `shared_company_profile`, `shared_source_configs` | SELECT | SELECT | SELECT, INSERT, UPDATE, DELETE | SELECT (pull) |
| Remote-owned decisions | `operator_decision_heads` | SELECT | SELECT, UPDATE (via CAS RPC) | SELECT, INSERT, UPDATE, DELETE | SELECT (pull) |
| Sync infrastructure | `schema_metadata` | — | — | SELECT, UPDATE | SELECT, UPDATE |

**Implementation notes:**
- Push-owned tables: daemon writes through a single `apply_push_batch(batch_json)` RPC that validates `schema_epoch`, checks `row_version` ordering, and applies atomically. RLS on the underlying tables grants `INSERT`/`UPDATE` only to `daemon` role.
- Operator decisions: `update_operator_decision(process_id, disposition, base_revision)` RPC enforces compare-and-swap semantics. Direct table `UPDATE` is blocked; only the RPC path is exposed.
- All `SELECT` policies include `WHERE deleted_at IS NULL` for tombstoned tables to exclude soft-deleted rows from portal views.
- `viewer` and `operator` roles cannot see sync infrastructure tables (`local_outbox`, `sync_batches`, `pull_cursors` are SQLite-only; `schema_metadata` is admin/daemon only).

**Sync Security:**

- Every push batch carries `schema_epoch` — remote rejects batches with incompatible epochs.
- Push batches carry `content_hash` per row — replay detection prevents duplicate application.
- Outbox integrity: business writes and outbox entries are in the same SQLite transaction — crashes cannot create orphaned sync state.
- Remote-owned writes (operator decisions) use compare-and-swap with `remote_revision` — prevents silent overwrites of human commercial decisions.
- Remote config must not overwrite machine-local secrets or runtime overrides. Split: remote = org-wide config; local = keychain refs, cookies, CLI paths, concurrency caps, temporary source disable flags.
- Snapshot config revision into each job at enqueue time, so long-running jobs are reproducible even if config changes mid-run.

**Data Protection:**

- Store secrets in desktop OS keychain / Tauri secure storage strategy, not plain-text configs. Local secrets and runtime overrides must never be synced to Supabase.
- Local SQLite file should have restrictive file permissions (0600).
- Encrypt especially sensitive company profile fields if stored centrally in Supabase.
- Redact agent logs before syncing to Supabase — full raw session output may contain sensitive data. Supabase receives only redacted excerpts or null for `terminal_output` and `structured_trace`.
- Agent session-output artifacts stored locally should be access-controlled.

**Security Risks:**

| Risk | Mitigation |
|------|------------|
| Prompt injection via source content | strict extraction schemas + sandboxed agent context |
| Secret leakage to agent logs | scoped env vars + redaction + non-browser secret handling |
| Overpowered Tauri shell execution | explicit Tauri plugin permissions and sidecar allowlists |
| Cross-tenant data exposure | RLS + broker/service-tier isolation + organization scoping |
| Unsafe CLI execution | launch via bounded commands, fixed working dirs, strict envs |

### 6.4 Performance Considerations

**Performance Targets:**

- Incremental ingestion per source should finish in minutes, not hours.
- Opportunity detail page should load in under 2 seconds from warm cache.
- Rerunning enrichment for one contract should begin immediately.
- Job queue should remain responsive even when long agent tasks are running.

**Optimization Strategies:**

- Poll sources incrementally with cursors (stored in local SQLite).
- Keep raw payloads and normalized projections separate.
- Use async worker queues for heavy enrichment.
- Cache FX and tariff lookups for short TTL windows.
- Local SQLite: enable WAL mode for concurrent reads during writes; use appropriate indexes.
- Supabase: precompute viewing-portal summary tables/materialized views for the web portal.

**SQLite Performance Characteristics:**

- Reads are very fast (local disk, no network latency).
- Single-writer model is sufficient for single-daemon architecture.
- WAL mode allows concurrent reads while writing, but long-lived readers can prevent checkpoints from fully draining the WAL. Keep desktop read transactions short.
- Database size should stay manageable (procurement data is relatively small volume).
- **Storage split (design decision):** Store large PTY transcripts and raw document payloads in a separate `artifacts.db` or `artifacts/` directory. The hot `state.db` should contain only relational state, jobs, manifests, and the sync outbox. This prevents checkpoint and vacuum overhead from large blob writes.

**SQLite Operational Guidance:**

- Keep SQLite databases on real local disk only — never NFS, cloud-sync folders (Dropbox, iCloud), or network-mounted storage. WAL requires same-host shared memory.
- Run `PRAGMA quick_check` on startup to detect corruption early. Run `PRAGMA integrity_check` on maintenance windows.
- Run `PRAGMA optimize` on a maintenance cadence (recommended over manually scheduling `ANALYZE`).
- Monitor WAL size, checkpoint latency, and SQLITE_BUSY counts.
- Use the Online Backup API or `VACUUM INTO` for safe hot backups. Never take backups by copying the main DB file while the process is live — missing WAL/journal sidecars can cause corruption.
- Store `schema_epoch` in `PRAGMA user_version` for fast compatibility checks.
- Use STRICT tables where json1 is not required for additional type safety.

**Supabase Performance Guidance:**

- Precompute viewing-portal summary tables/materialized views for the web portal.
- Do not turn on Postgres Changes subscriptions for every hot table. For live web portal updates, prefer small projection-change or status broadcast events (Supabase Broadcast) over raw row-by-row Postgres Changes streams.
- Do not have the web portal query all raw sync tables directly — provide dedicated projection tables keyed to portal use-cases.

**Potential Bottlenecks:**

| Bottleneck | Mitigation |
|------------|------------|
| Supplier search explosion | limit search permutations, use line-item heuristics |
| Agent runtime saturation on user machine | daemon concurrency caps on PTY-backed browser sessions and total adapter-mediated sessions |
| Large manifest generation | generate async and cache artifacts |
| Control-panel and portal search over large JSON payloads | normalized columns + indexes (SQLite for desktop, GIN + FTS for Supabase/web) |
| SQLite-to-Supabase sync volume | outbox-based batch sync, push only changed records via local_seq ordering |
| Terminal output storage growth | store in separate artifacts DB, compress old logs, rotate after configurable retention |
| WAL file growth | monitor checkpoint latency, keep desktop reads short, periodic checkpoint |
| Tombstone accumulation | GC only after all replicas have pulled past delete seq + offline window (37 days default) |

### 6.5 Testing Strategy

**Core principle:** Prioritize **real end-to-end tests** over mocks and stubs. Real E2E agent tests are the ultimate success criterion — they determine whether the pipeline actually works. Avoid creating mocks or stub test functionality.

For hybrid-runtime coverage, "real E2E" means exercising both PTY-backed sessions and contract-mode sessions against real agent CLIs where the workflow supports each mode.

**Test Priority (highest to lowest):**

| Priority | Type | Coverage Target | Tools |
|----------|------|-----------------|-------|
| 1 | **Agent E2E** | Real agent sessions (PTY and contract mode) with sample prompts and skills | cargo test + real agent CLIs |
| 2 | **Integration** | Real database + real agent processes | SQLite + Supabase local stack |
| 3 | **Unit** | Deterministic cost/qualification logic | Vitest, cargo test |
| 4 | **Browser E2E** | Key operator control-panel and viewing-portal flows | Playwright |

**Agent E2E Testing (PRIMARY — determines success):**

The most critical tests verify that the full agent pipeline works:

1. Send real kickoff prompts to real agent sessions (PTY-backed or contract-mode as appropriate for the skill).
2. Load actual skills into the agent session.
3. Verify the agent is working, outputting, and proceeding with its tasks.
4. Verify structured data output from the agent.
5. Verify the daemon correctly captures raw session output and validated protocol events.
6. Verify error detection (stalls, failures, expired sessions).

A real agent E2E test fails if any of these conditions are not met within the test budget: raw session output begins, structured result output is produced, expected job state is reached, and the correct health/error classification is recorded.

```rust
#[tokio::test]
async fn agent_procurement_ingestion_produces_notices() {
    // Create a real PTY session
    let session = DaemonPty::spawn_agent(
        "skills/procurement-ingestion/fts/SKILL.md",
        "Ingest the latest 5 notices from Find a Tender"
    ).await;

    // Verify agent is producing output
    let output = session.wait_for_output(Duration::from_secs(60)).await;
    assert!(!output.is_empty(), "Agent should produce terminal output");

    // Verify structured data was captured
    let notices = session.wait_for_completion().await.unwrap();
    assert!(!notices.is_empty(), "Agent should extract at least one notice");

    // Verify terminal output was captured for tracing
    let trace = session.full_terminal_output();
    assert!(trace.contains("notice"), "Terminal output should show agent working on notices");
}

#[tokio::test]
async fn agent_stall_is_detected() {
    let session = DaemonPty::spawn_agent(
        "skills/test-stall/SKILL.md",
        "This skill intentionally stalls"
    ).await;

    let result = session.wait_for_completion_with_timeout(Duration::from_secs(30)).await;
    assert!(matches!(result, Err(AgentError::Stalled)));
}
```

**Integration Testing (no mocks):**

All integration tests use real databases and real processes:

- Real SQLite databases for daemon storage tests.
- Real Supabase local stack for sync tests.
- Real agent processes for skill execution tests.
- No recorded fixtures, no schema assertions against mocked responses.

**Unit Testing (deterministic logic only):**

Unit tests are reserved for purely deterministic logic that does not depend on external systems:

```ts
describe("net margin gating", () => {
  it("qualifies only when net margin is at least 20%", () => {
    const estimate = {
      revenueGbpExVat: 1000,
      economicLandedCostGbp: 780,
    };

    const netMargin =
      (estimate.revenueGbpExVat - estimate.economicLandedCostGbp) / estimate.revenueGbpExVat;
    expect(netMargin).toBeGreaterThanOrEqual(0.2);
  });
});
```

```rust
#[test]
fn expires_within_30_days_is_rejected() {
    let days_until_expiry = 14_i64;
    assert!(days_until_expiry < 30);
}
```

**Deterministic Replay Testing (parser and health-monitor regression):**

Real vendor E2E remains the acceptance gate, but captured real PTY/JSON sessions are replayed for regression coverage:

- Record real PTY byte streams and contract-mode JSONL from passing E2E runs.
- Replay captured sessions against the adapter parser, health monitor, and result contract validator.
- Detect regressions when parser/validator code changes without needing live agent CLIs.

**Fault-Injection Testing:**

Test daemon resilience against failure modes that real E2E tests rarely trigger:

| Fault | Test Coverage |
|-------|---------------|
| Dropped child handle | Verify orphan sweep detects and reaps leaked processes |
| Orphaned grandchildren | Verify process-group kill reaches descendants spawned by agent |
| PTY EOF/EIO handling | Verify daemon treats EIO after slave close as normal termination |
| Host sleep / monotonic gap | Verify `HOST_SLEEP_INTERRUPTED` classification when time jumps |
| Partial checkpoint then crash | Verify `AGENT_PARTIAL_FAILURE` — valid checkpoints preserved, job not upgraded to success |
| Auth challenge mid-run | Verify `AGENT_AUTH_CHALLENGE` classification distinct from `AGENT_STALLED` |
| Cleanup timeout after main pid exit | Verify `AGENT_CLEANUP_INCOMPLETE` when descendants survive group kill |
| Concurrent session resource pressure | Verify resource arbiter correctly preempts lower-priority sessions |
| Unclassifiable agent exit | Verify `AGENT_RUN_FAILED` classification when exit code is non-zero but no specific error pattern matches |

**Adapter Contract Testing:**

Each vendor adapter is tested for:

- Capability probe correctness (CLI version detection, auth health check, structured output viability).
- Mode selection logic (PTY vs. contract based on skill `interaction_profile` and probe results).
- Canary mode behavior (unknown CLI versions blocked from production until probe passes).
- Structured output parsing (vendor JSONL/JSON parsed into daemon-native events without loss).

### 6.6 Operational Observability

Comprehensive agent tracing and health monitoring is **critical infrastructure**, not a nice-to-have. The entire pipeline runs through agent sessions, so observability of those sessions is essential for debugging and reliability.

#### Three-Layer Observability Model

Observability is structured into three distinct layers, each with a clear role:

**Layer 1: Control Events (daemon-generated)**

Events the daemon itself produces about session lifecycle:

- `session_spawned` (pid, pgid, adapter, mode, skill_id, skill_version, cli_version, config_hash)
- `session_active` (first output/event received)
- `signal_sent` (signal type, target pid/pgid)
- `session_exited` (exit code, exit signal)
- `session_reaped` (reap method, descendants found)
- `cleanup_verdict` (`clean`, `forced`, `leaked`)
- `host_event` (sleep_detected, resource_pressure)

**Layer 2: Protocol Events (adapter-generated)**

Events parsed from vendor-specific structured output by the adapter:

- Vendor JSONL events (Codex: `turn.started`, `turn.completed`, `item.*`, `error`)
- Vendor JSON/stream-json output (Claude headless structured output)
- Validated checkpoints (schema version, job id, skill id, sequence, status, outputs)
- Auth challenges detected from structured signals
- Final result contract validated

**Layer 3: Forensic Bytes (immutable raw capture)**

Raw session-output bytes captured in both execution modes. In PTY mode this includes:

- Agent thinking/reasoning text.
- Tool calls and their results.
- Browser navigation steps.
- Data extraction outputs.
- Error messages and stack traces.
- ANSI escape sequences (stripped for derived summaries, preserved in raw artifact for replay).

In contract mode, Layer 3 additionally includes the raw stdout/stderr stream and any vendor-emitted JSONL/JSON framing before adapter normalization.

**Critical rule:** Only Layer 1 and Layer 2 drive workflow state. Layer 3 is for post-hoc debugging and replay. A transcript can explain a failure; it must never upgrade a job to success.

#### Artifact Storage Model

Raw session-output bytes and large session outputs are stored as immutable compressed artifact files, **not inline in hot SQLite tables**:

- **Raw artifacts:** Chunked, compressed, immutable files at `artifacts/sessions/YYYY/MM/<session_id>/`. PTY mode stores raw terminal bytes; contract mode stores raw stdout/stderr bytes alongside parsed protocol events. Rotate capture chunks every 8 MiB and compress immediately. Preserve full raw capture by default; if a single session exceeds 256 MiB of raw bytes, stop capture only after writing an explicit truncation marker and surfacing an operator warning.
- **SQLite stores:** Session header, byte counts, last 64 KB tail excerpt, redaction markers, artifact path, SHA-256 hash, and derived control/protocol event summaries.
- **Supabase receives:** Redacted summary, event counts, and status only — no raw transcripts.

This preserves full session-output capture as critical infrastructure without turning the operational database into a log sink.

#### Agent Health Detection

The daemon actively monitors every running session for:

| Health Signal | Detection Method | Response |
|---------------|-----------------|----------|
| **No output bytes** | Configurable timeout since last PTY byte or stdout/stderr byte | Mark `AGENT_STALLED`, kill process group, create error manifest |
| **No validated progress** | No checkpoint written within progress timeout (both modes) | Mark `AGENT_STALLED`, kill process group, create error manifest |
| **Auth challenge** | Pattern match on raw output stream or structured event for login/MFA/SSO/permission prompts | Mark `AGENT_AUTH_CHALLENGE`, alert operator via desktop control panel |
| **API error inside agent** | Adapter parses error from structured events or terminal patterns | Mark `AGENT_INTERNAL_ERROR`, capture full context |
| **Skill execution failed** | Agent exits with error or produces error output | Mark `AGENT_SKILL_FAILED`, capture skill path, step, and skill version |
| **Result contract invalid** | Final output fails schema validation | Mark `AGENT_OUTPUT_CONTRACT_INVALID`, preserve artifacts |
| **Host sleep detected** | Monotonic time gap exceeds threshold | Mark `HOST_SLEEP_INTERRUPTED`, taint browser-backed sessions |
| **Host resource pressure** | Resource arbiter detects memory/CPU pressure exceeding threshold | Mark `HOST_RESOURCE_EXHAUSTED`, preempt lower-priority sessions |
| **Protocol mismatch** | Adapter detects unexpected CLI output format or unsupported protocol version during parsing | Mark `AGENT_PROTOCOL_MISMATCH`, enter canary mode for vendor, create error manifest |
| **General run failure** | Agent exits with non-zero code, not classifiable as a more specific error type | Mark `AGENT_RUN_FAILED`, retry if safe, create error manifest |
| **Normal completion** | Valid final result contract + clean exit + complete reap | Mark completed, archive session |

#### Structured Tracing

Structured traces are derived primarily from Layer 1 (control) and Layer 2 (protocol) events, stored in `agent_session_logs.structured_trace`:

```json
{
  "layers": {
    "control": [
      {"type": "session_spawned", "pid": 12345, "pgid": 12345, "mode": "pty", "timestamp": "..."},
      {"type": "session_active", "timestamp": "..."},
      {"type": "session_exited", "exit_code": 0, "timestamp": "..."},
      {"type": "cleanup_verdict", "verdict": "clean", "timestamp": "..."}
    ],
    "protocol": [
      {"type": "navigate", "url": "https://www.find-tender.service.gov.uk/...", "timestamp": "..."},
      {"type": "extract", "count": 12, "entity": "notices", "timestamp": "..."},
      {"type": "checkpoint", "sequence": 1, "status": "partial", "timestamp": "..."},
      {"type": "final_result", "schema_version": "1.0", "status": "success", "timestamp": "..."}
    ]
  },
  "forensic_ref": "artifacts/sessions/2026/03/abc123/raw.zst"
}
```

Transcript-derived enrichment (best-effort parsing of raw PTY output for tool calls, browser steps, etc.) may supplement the structured trace but does not override control/protocol events.

#### Per-Session Metrics

From day one, record per session:

- pid / pgid,
- CLI vendor + version,
- adapter version,
- config hash,
- skill id + skill version,
- session duration,
- bytes captured (raw artifact size),
- checkpoint count,
- cleanup latency (time from exit to reap),
- cleanup verdict,
- auth challenge count,
- stall phase (which timeout triggered, if any),
- redaction count (sensitive data markers),
- peak resource estimate (if available from resource arbiter).

#### Standard Operational Metrics

From day one, record aggregate:

- job start/end with session ID,
- daemon hostname/device id,
- prompt hash / skill version / skill path,
- agent session duration and output volume,
- result contract validation success/failure rate,
- parse failure counts per source,
- source scraping latency,
- quote confidence distribution,
- margin distribution,
- request pipeline success/failure by portal,
- SQLite-to-Supabase sync latency and failure counts,
- error manifest creation rate by type and stage,
- resource arbiter utilization (browser slots, agent slots, source lanes),
- cleanup verdict distribution (clean / forced / leaked).

#### Surface Integration

The desktop control panel provides an observability view:

- **Active agent sessions:** Real-time view of running sessions with live terminal output (PTY mode) or structured event stream (contract mode).
- **Session history:** Browse completed sessions with structured traces and forensic artifact drill-down.
- **Error manifest:** Filterable list of all pipeline errors with drill-down.
- **Health status:** Agent auth status, daemon uptime, sync status, resource arbiter utilization.
- **Metrics:** Charts for job throughput, success rates, latency, and cleanup health.

If this is not instrumented, the platform will be impossible to tune and debug.

---

## 7. Architectural Decisions

### ADR-001: Use Skill-Based Agent Scraping as Primary Data Retrieval

**Status:** Revised (supersedes original "Use Official Procurement APIs Before Scraping")

**Context:**

The platform needs to retrieve data from procurement sources and supplier marketplaces. The original approach was API-first connectors — separate TypeScript API client packages for each source. The owner’s feedback directs that all data retrieval should be constrained to the skill-based agent system running on the local machine, using browser automation. This simplifies the architecture: skills are easier to maintain, upgrade, and change than API client packages.

**Exception:** Cost-estimation data (customs, freight, FX) uses direct API access, where APIs are more reliable and structured.

**Decision:**

All procurement and supplier data retrieval uses skill-based agent scraping via browser automation. The agent runs on the local machine, powered by source-specific skills, and is orchestrated by the Rust daemon through adapter-mediated agent sessions, typically PTY-backed for retrieval flows. No API client packages are maintained for procurement or supplier sources. The official procurement sources (Find a Tender, Contracts Finder, PCS) are still the target data sources — but they are accessed via browser automation, not via TypeScript API clients.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: Skill-based agent scraping (SELECTED) | single retrieval architecture for all sources, easy to maintain/upgrade skills, handles auth-heavy sources naturally | depends on agent reliability, skill maintenance when sites change |
| B: API-first connectors (original) | structured, stable | requires per-source TypeScript packages, harder to maintain, doesn’t handle auth-heavy sources well |
| C: Third-party aggregators | less engineering | vendor dependency, reduced control |

**Rationale:**

Skill-based agent scraping provides a unified data retrieval architecture. Instead of maintaining separate API client packages for each source (with their own auth flows, rate limiting, parsing), the platform maintains skills — markdown playbooks that tell the agent how to navigate each source. When a source changes, only the skill needs updating. This is materially simpler to maintain.

**Consequences:**

- Positive: unified architecture for all data retrieval, easier skill maintenance vs API client packages.
- Negative: depends on agent reliability and browser automation stability. Mitigated by comprehensive tracing and error manifest system.
- Cost management: the skill-based approach requires deliberate session batching (one session per source per notice, not per candidate), quoteability gating to suppress low-value searches, deterministic shortlisting before AI adjudication, and query-fingerprint caching. Without these optimizations, naive enrichment can reach $75–$170/day; with them, $20–$60/day is achievable. See §5.7 (Pipeline Cost Management) for details.

**User Input Required:** No

### ADR-002: Authority-Matrix Database Architecture — Local SQLite + Supabase/Postgres

**Status:** Revised (supersedes original "Use Supabase/Postgres as the Shared Control Plane"; updated to replace singular "source of truth" with authority matrix)

**Context:**

The system needs local-first operation with multi-user visibility. The original approach used Supabase/Postgres as the sole source of truth. The owner's feedback directs a dual-database architecture. Review feedback (planmaxer cycle 1) identified that a singular "source of truth" model hides the real bug class: mixed-authority rows where pipeline-generated data and human decisions coexist.

**Rationale for change:** Multiple users need access to the system's data, but relying solely on a shared remote Postgres creates corruption risk. However, the dual-database design introduces its own dominant risk: application-level logical corruption in the replication layer. The authority matrix model mitigates both by ensuring every table has exactly one authority domain with no mixed writes.

**Decision:**

- **Authority matrix** replaces singular "source of truth" — three authority domains: local operational, remote shared, remote projections.
- **Mixed-write tables are prohibited.** Concepts that span domains are split into separate tables (e.g., `pipeline_assessments` + `operator_decision_heads` + `bid_workflow_states` instead of a single `opportunity_decisions`).
- **Push-only pipeline replication:** Local → remote via transactional outbox with monotonic per-executor ordering and idempotent remote apply.
- **Pull-only shared config/decisions:** Remote → local via cursor-based pull on monotonic `remote_change_seq`. Operator decisions use compare-and-swap (not last-write-wins).
- **MVP constraint:** One active pipeline daemon per org. No generic conflict engine until a second writable desktop exists.
- **Convergence:** Via `local_seq`, `row_version`, `remote_change_seq`, `remote_revision` — never clock-based.
- **Schema coordination:** Canonical logical schema with `schema_epoch` in both databases and all sync envelopes.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: Authority-matrix replication (SELECTED) | local-first resilience, corruption protection, no mixed-write bugs, idempotent sync | sync engine complexity, eventual consistency for web viewers |
| B: Singular source-of-truth local SQLite (previous revision) | simpler mental model | hides mixed-authority bugs, underspecified sync protocol |
| C: Supabase/Postgres only (original) | simpler architecture | network dependency, corruption risk, no local resilience |
| D: Convex | great realtime | weaker fit for SQL-heavy control plane |
| E: SpacetimeDB | exciting realtime model | maturity/fit risks for this workload |

**Rationale:**

The daemon runs locally and writes pipeline data continuously. A local database eliminates network dependency during operation and provides corruption resilience. The authority matrix ensures that the sync layer — now the highest-risk component for data integrity — has clear, testable rules per table. Multi-user visibility is achieved through push-only projections to Supabase.

**Consequences:**

- Positive: local-first resilience, corruption protection, no mixed-write ambiguity, idempotent sync, clear recovery boundaries.
- Negative: must design and maintain authority-based sync engine. Schema must be coordinated across two databases. Web viewers see eventually-consistent data. Recovery from local loss is explicitly lossy (full PTY output, error context permanently lost).

**User Input Required:** No

### ADR-003: Use Tauri v2 With a Rust Daemon

**Status:** Accepted

**Context:**

The user wants local background execution and values Rust robustness.

**Decision:**

Use Tauri v2 for desktop shell and a Rust daemon/sidecar for background work.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: Tauri + Rust daemon | lightweight desktop, strong local execution | two-language stack |
| B: Electron + Node daemon | familiar web tooling | heavier runtime |
| C: Pure web app only | simpler deploy | cannot satisfy local execution requirement |

**Rationale:**

Tauri aligns with the desktop-first ops model and Rust is a good fit for process management and deterministic cost logic.

**Consequences:**

- Positive: lighter desktop footprint, strong systems language for daemon.
- Negative: desktop and web frontends should share UI, not runtime assumptions.

**User Input Required:** No

### ADR-004: Use PTY-Based Agent Execution from Day 1

**Status:** Updated (refined from "PTY for everything" to "PTY as first-class capability with hybrid execution")

**Context:**

The entire data ingestion, intelligence, and bidding pipeline runs through agent sessions triggered by the Rust daemon. PTY execution is a first-class runtime from day 1 for browser automation, interactive auth, and TTY-dependent flows. However, both Codex (`exec` with `--json` JSONL output) and Claude (headless mode with `json`/`stream-json` and `--json-schema`) now support stable structured non-interactive execution. A hybrid model reduces blast radius: PTY for interactive flows, contract mode for deterministic tasks.

**Decision:**

Launch agent jobs through an adapter-mediated execution model with two transport modes from day 1:

1. **PTY mode:** Required for browser automation, interactive auth, or TTY-dependent CLI behavior.
2. **Contract mode:** Preferred for deterministic tasks where the vendor supports structured non-interactive execution.

The daemon selects mode per job based on the skill's declared `interaction_profile` and the adapter's capability probe. Raw session output is captured as a forensic artifact in both modes (PTY bytes in PTY mode, stdout/stderr in contract mode); structured events drive workflow state in both modes.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: Hybrid PTY + contract mode via adapter (SELECTED) | mode matches job requirements, reduces blast radius, stronger result contracts for deterministic work, PTY still available for all interactive flows | adapter boundary adds design work; two code paths to maintain |
| B: PTY-only for everything | consistent single model, handles all CLI modes | pays PTY overhead for deterministic tasks, terminal parsing as control plane is fragile |
| C: Non-interactive only | simplest | misses TTY-dependent flows, cannot handle browser automation or interactive auth |

**Rationale:**

The pipeline must function without interruption every day. PTY provides real terminal semantics needed for browser automation and interactive flows. Contract mode provides stronger result contracts and simpler cleanup for deterministic work. The adapter boundary isolates the daemon from vendor CLI changes. The hybrid model gives the right execution semantics for each job type.

**Consequences:**

- Positive: robust daily operation, full terminal output capture for interactive flows, stronger result contracts for deterministic flows, vendor isolation via adapter boundary, process-tree supervision with explicit cleanup verdicts.
- Negative: two execution paths to maintain. Mitigated by the adapter abstraction producing a unified daemon event stream regardless of mode.

**User Input Required:** No

### ADR-005: Skills Are the Primary Data Retrieval Mechanism and Workflow Assets, Not the Workflow Engine

**Status:** Updated (adds explicit binding and skill metadata requirements)

**Context:**

Skills serve a dual role in the architecture:

1. **Primary data retrieval mechanism:** All procurement and supplier data is collected via skill-based agent scraping. Skills define how the agent navigates sources and extracts data.
2. **Workflow behavioral assets:** Skills define how the agent behaves during qualification, manifest generation, and bid automation.

Skills are NOT the workflow engine — orchestration (scheduling, job state, retries, sync) stays in the daemon and database. Scheduled production jobs bind explicit skill id and version; implicit skill discovery is disabled for cron runs to prevent reliability issues at scale.

**Decision:**

Skills are BOTH the primary data retrieval mechanism AND workflow behavioral assets. The daemon/database owns orchestration: when to run which skill, job state tracking, retry logic, scheduling, sync to Supabase. Skills define: how the agent navigates a source, what data to extract, how to handle errors during execution, and what execution mode they require (`interaction_profile`). Every skill declares metadata (interaction profile, result schema version, auth requirements) that the daemon uses for execution planning.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: skills as data retrieval + assets with explicit binding, DB orchestration (SELECTED) | unified retrieval architecture, reliable orchestration, deterministic cron execution | more skill maintenance work, metadata overhead |
| B: skills as orchestration | fast to prototype | state becomes opaque and fragile |
| C: skills with implicit discovery for all runs | less config | cron reliability depends on ambient discovery behavior — broken by CLI updates |

**Rationale:**

Agent prompts are not a durable job scheduler. But agent skills ARE an excellent mechanism for defining source-specific data retrieval playbooks. The architecture cleanly separates: skills handle the "how" of data retrieval and workflow execution, while the daemon/database handles the "when", "why", and "what happened". Explicit skill binding for scheduled jobs prevents implicit discovery failures from disrupting production pipelines.

**Consequences:**

- Positive: unified data retrieval, replayability, observability, easy skill maintenance, deterministic cron execution via explicit binding.
- Negative: requires explicit workflow modeling, skill versioning, and skill metadata declarations.

**User Input Required:** No

### ADR-006: Build an Internal Landed-Cost Engine

**Status:** Accepted (revised scope clarification)

**Context:**

Third-party shipping/tax APIs are useful, but margin decisions need explainable and auditable numbers. The internal engine uses HMRC Trade Tariff for duty/VAT, HMRC FPO for HS classification candidates, HMRC Exchange Rates for customs FX, and commercial APIs for freight and commercial FX.

**Decision:**

Implement customs, valuation, shipping mode selection, and FX combination as internal logic using authoritative and commercial source inputs. The engine operates at two grades:
- **Screening grade** — automated across the whole market using indicative data
- **Decision grade** — automated only for repeated product families with proven codebooks and confirmed supplier economics

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: internal engine + official data | explainable, auditable | more engineering |
| B: third-party black-box estimator | quick | opaque and weaker for high-stakes decisions |

**Rationale:**

The core business value is the decision engine. That cannot be delegated blindly.

**Consequences:**

- Positive: higher trust, better override behavior, explainable decisions with full audit trail.
- Positive: screening-grade automation filters the whole market; decision-grade grows as product-family codebooks accumulate.
- Negative: implementation complexity is high — 10-digit resolution, measure scanning, dual FX, Incoterm normalization, scenario-based margins.
- Negative: accuracy ceiling depends on classification and freight estimation quality, requiring ongoing calibration.

**User Input Required:** No

### ADR-007: Automate the Request Pipeline, Flag Errors for Debugging

**Status:** Revised (relaxes original "Human Review Is Required for Low-Confidence Commercial Decisions")

**Context:**

The platform automates the **request pipeline**, not real-money bid submissions. Actual negotiations and fulfillment are handled manually by humans using the pipeline output. The cost of pipeline failure is low — no real money changes hands automatically. Human review remains valuable for commercial decisions, but the automation risk framing is significantly relaxed.

**Decision:**

Full automation of the request pipeline is acceptable. When pipeline steps fail, the error manifest system flags: which step failed, the error message, the pipeline stage, and enough context to debug. Human operators handle the actual commercial decisions (negotiations, fulfillment) manually.

Confidence-gated review is still valuable for:
- Commodity code classification (getting HS codes right matters for cost estimation accuracy).
- Supplier matching (ensuring the right products are being compared).

But these are quality gates for better intelligence, not safety gates against financial risk.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: Full pipeline automation + error manifest (SELECTED) | high throughput, low risk since not real money, structured error debugging | may produce some low-quality intelligence |
| B: Confidence-gated human review at every stage (original) | highest quality | too much operator involvement for the actual risk level |

**Rationale:**

Since the platform automates request preparation (not real-money transactions), the cost of errors is low. The error manifest system provides structured debugging when things go wrong, and human operators handle all actual commercial decisions.

**Consequences:**

- Positive: higher throughput, simpler operator workflow, structured error debugging via manifests.
- Negative: some pipeline outputs may be lower quality. Mitigated by confidence scoring and the error manifest system.

**User Input Required:** No

### ADR-008: Desktop is Control Panel, Web is Read-Only Viewing Portal

**Status:** Revised (answers original "Separate Desktop and Web Shells" question)

**Context:**

The desktop app and web app serve fundamentally different roles. The desktop is the control panel; the web is the viewing portal.

**Decision:**

Separate shells with clearly different capabilities:

- **`apps/desktop`** (Tauri + static frontend) — **CONTROL PANEL**: Full read/write/trigger capabilities. Trigger workflows, manage agents, configure skills, make reject/bid/watch decisions, view local data from SQLite, view agent tracing logs, manage error manifests.
- **`apps/web`** (hosted) — **READ-ONLY VIEWING PORTAL**: Browse opportunities, view manifests, view pipeline status, view errors. No write operations. Reads from Supabase (synced data).
- **`packages/ui`** for shared React components (but desktop shell includes additional control components).
- **`packages/contracts-schema`** for shared domain models.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: Split shells with different roles (SELECTED) | clear responsibility split, web is simple, desktop has full power | some duplicated UI shell code |
| B: Single Next app everywhere with equal capabilities | less duplication | desktop constraints leak into product architecture, web gets unnecessary write capabilities |

**Rationale:**

The desktop app runs alongside the daemon and has direct access to local SQLite and agent controls. The web app is for remote viewing only — it reads from Supabase and provides no write operations. This clear split simplifies the web shell significantly.

**Consequences:**

- Positive: web shell is simple and safe (no write operations to protect). Desktop has full local power.
- Negative: some duplicated shell code for shared views.

**User Input Required:** No (resolved by owner feedback)

---

## 8. File Reference Map

### 8.1 Files to Read (Pre-Implementation)

**Critical Reading:**

| Priority | File | Purpose |
|----------|------|---------|
| 1 | `/Users/alinaaitova/Desktop/Testproject1/docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` | Foundational architecture and decisions |
| 2 | `/Users/alinaaitova/.codex/skills/idea-to-research/SKILL.md` | Skill workflow and expected research handoff standard |

### 8.2 Files to Create

#### Workspace Bootstrap

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/package.json` | root workspace scripts | custom |
| `/Users/alinaaitova/Desktop/Testproject1/bunfig.toml` | Bun workspace/runtime settings | custom |
| `/Users/alinaaitova/Desktop/Testproject1/Cargo.toml` | Rust workspace | custom |
| `/Users/alinaaitova/Desktop/Testproject1/.env.example` | environment contract | custom |

#### Database and Infra

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/migrations/0001_sqlite_state_initial.sql` | Local SQLite state.db schema bootstrap — pipeline entities (procurement_processes, contract_notices, notice_lots, qualification_runs, product_candidates, supplier_quotes, landed_cost_estimates, manifests, pipeline_assessments, bid_workflow_states, process_relations, search_profiles, executors), all with sync tracking columns | based on section 5.4 (SQLite syntax) |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/migrations/0002_sqlite_state_sync.sql` | Local outbox, sync batches, pull cursors, stage tracking (process_stage_status, process_stage_heads) | section 5.4 sync infrastructure |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/migrations/0003_sqlite_artifacts.sql` | Artifacts DB schema — agent_session_logs (with orchestration domain columns), raw_documents, error_manifests | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/db/migrations/0001_initial.sql` | Supabase/Postgres projection tables (all push-owned entities including notice_lots, process_relations, search_profiles, workflow_job_summaries, error_manifest_summaries) + remote-owned tables (shared_company_profile, shared_source_configs, operator_decision_heads) | based on section 5.4 |
| `/Users/alinaaitova/Desktop/Testproject1/packages/db/migrations/0002_rls.sql` | RLS policies per authority role (viewer, operator, admin, daemon) — see §6.3 for policy matrix | section 6.3 |
| `/Users/alinaaitova/Desktop/Testproject1/packages/db/migrations/0003_sync_apply.sql` | Transactional sync apply RPC for push batches + operator decision CAS RPC | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/db/migrations/0004_schema_metadata.sql` | Schema metadata table + initial schema_epoch | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/db/migrations/0005_stage_tracking.sql` | process_stage_status, process_stage_heads projection tables | section 5.4 stage tracking |
| `/Users/alinaaitova/Desktop/Testproject1/packages/db/migrations/0006_junction_tables.sql` | notice_cpv_codes, product_commodity_codes, lot_cpv_codes | section 5.4 junction tables |
| `/Users/alinaaitova/Desktop/Testproject1/infra/supabase/config.toml` | local Supabase config | Supabase standard |

#### Shared Domain

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/packages/contracts-schema/src/contracts.ts` | canonical contract types | section 5.4 |
| `/Users/alinaaitova/Desktop/Testproject1/packages/contracts-schema/src/jobs.ts` | job payloads and states | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/contracts-schema/src/confidence.ts` | confidence models | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/contracts-schema/src/zod.ts` | runtime validation | custom |

#### Cost-Estimation Intelligence Engine (API-Based)

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/qualification/expiry.ts` | 30-day expiry filter | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/qualification/access-route.ts` | access-route classification and bidability rules | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/qualification/goods.ts` | goods/services qualification | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/qualification/quoteability.ts` | deterministic quoteability scoring | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/relations/resolve-process-relations.ts` | OCID + heuristic relation resolution | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/extraction/products.ts` | line-item/product extraction | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/search/build-search-profile.ts` | multilingual query-pack construction from structured candidate slots | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/hmrc-tariff.ts` | HMRC Trade Tariff API integration | official HMRC docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/hmrc-fpo.ts` | HMRC FPO candidate code generator API | official HMRC FPO docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/classification-preprocessing.ts` | description-quality gate, bundle splitter, attribute extraction, multi-query FPO dispatch | custom (§4.5) |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/ten-digit-resolver.ts` | 10-digit commodity code resolution from 6/8-digit FPO candidates via Trade Tariff API | custom (§4.5) |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/measure-scanner.ts` | trade remedy detection, preferential eligibility, quota checks, additional code requirements | custom (§4.5) |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/taric-support.ts` | Taric Support API for code validation | Taric Support docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/hs-ping.ts` | HS Ping comparison lookup | HS Ping docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/fx/xe.ts` | Xe commercial FX integration | official Xe docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/fx/hmrc-exchange-rates.ts` | HMRC official exchange rates for customs valuation | HMRC docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/fx/currencybeacon.ts` | CurrencyBeacon backup/comparison FX integration | CurrencyBeacon docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/fx/oanda.ts` | OANDA historical FX integration | OANDA docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/logistics/freightos.ts` | Freightos freight API integration | official Freightos docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/logistics/searates.ts` | SeaRates freight API integration | SeaRates docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/logistics/mode-selection.ts` | freight mode decision tree with hard mode locks and dual packing scenarios | custom (§4.4) |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/logistics/easyship.ts` | Easyship parcel-mode freight and duty estimation | Easyship docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/logistics/shippo.ts` | Shippo parcel-mode carrier rates and international shipping | Shippo docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/validation/dutify.ts` | Dutify landed-cost comparison adapter | Dutify docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/validation/zonos.ts` | Zonos landed-cost comparison adapter | Zonos docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/validation/avalara.ts` | Avalara landed-cost comparison adapter | Avalara docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/margins/revenue-normalizer.ts` | revenue basis classification and expected_addressable_revenue_ex_vat extraction | custom (Appendix A) |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/margins/quote-normalizer.ts` | Incoterm cost boundary normalization (EXW/FOB/CIF/DDP → comparable basis) | custom (Appendix A) |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/margins/margin-scenarios.ts` | low/base/high margin scenario calculation with critical blocker checks | custom (Appendix A) |

#### Manifests

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/packages/manifests/src/render-opportunity.ts` | Markdown manifest generation | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/manifests/src/templates/opportunity.md.ts` | manifest template | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/manifests/src/render-error-manifest.ts` | Markdown error manifest generation | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/manifests/src/templates/error.md.ts` | error manifest template | custom |

#### Bidding

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/packages/bidding/src/workflows/index.ts` | workflow registry | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/bidding/src/workflows/proactis-v1.ts` | initial downstream tender-portal workflow | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/bidding/src/company-profile/schema.ts` | bidder profile contract | custom |

#### Desktop Shell (CONTROL PANEL)

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/package.json` | desktop app package | Tauri bootstrap |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src-tauri/src/main.rs` | Tauri entrypoint | Tauri bootstrap |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src-tauri/capabilities/default.json` | shell/sidecar permissions | Tauri permissions docs |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/lib/daemon.ts` | frontend bridge to daemon | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/pages/contracts.tsx` | contracts list (reads local SQLite via daemon) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/pages/contract-detail.tsx` | opportunity detail with reject/bid/rerun controls | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/pages/agent-sessions.tsx` | agent session monitoring with live terminal output | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/pages/error-manifest.tsx` | error manifest viewer with resolution controls | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/pages/settings.tsx` | source configs, skill management, agent settings | custom |

#### Web Shell (READ-ONLY VIEWING PORTAL)

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/package.json` | web app package | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/src/app/contracts/page.tsx` | contracts list (reads Supabase) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/src/app/contracts/[id]/page.tsx` | opportunity detail (read-only, no controls) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/src/app/status/page.tsx` | pipeline status and error overview (read-only) | custom |

#### Rust Daemon

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/Cargo.toml` | daemon crate definition | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs` | daemon entrypoint | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/db/sqlite.rs` | local SQLite connection and schema management | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/db/sync.rs` | SQLite↔Supabase push-pull sync engine | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/db/checkpoints.rs` | local sync checkpoints and recovery cursors | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/sources/registry.rs` | source registry loading, enablement, and schedule binding | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/scheduler/cron.rs` | local cron scheduler for agent jobs with catchup sweep | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/jobs/claim.rs` | job claiming/leases (local SQLite) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/jobs/runner.rs` | job executor via adapter-mediated sessions | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/pty/session.rs` | PTY session creation, lifecycle, and process-group supervision | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/pty/capture.rs` | raw PTY byte capture to immutable artifact files | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/pty/health.rs` | agent health monitoring (stall, auth challenge, sleep detection) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/pty/reaper.rs` | session reaper — process-group cleanup, kill escalation, orphan sweep | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/pty/hardening.rs` | PTY hardening — fd hygiene, EIO handling, terminal geometry | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/adapter.rs` | agent adapter trait — canonical event stream, mode selection, capability probing | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/codex.rs` | Codex adapter — launch, JSONL parsing, auth probing | local CLI contract |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/claude.rs` | Claude adapter — launch, JSON/stream-json parsing, auth probing | local CLI contract |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/artifacts.rs` | artifact store — compressed immutable session artifacts | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/contracts.rs` | result contract validator — schema validation, checkpoint sequencing | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/arbiter.rs` | resource arbiter — browser slots, agent slots, source lanes, priority scheduling | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/errors/manifest.rs` | error manifest creation and management | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/errors/markdown.rs` | error manifest Markdown artifact writer | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/tracing/structured.rs` | structured trace derivation from control/protocol events | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/tracing/forensic.rs` | forensic transcript enrichment (best-effort PTY output parsing) | custom |

#### Skills (Primary Data Retrieval + Workflow Playbooks)

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/skills/procurement-ingestion/fts/SKILL.md` | Find a Tender agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/procurement-ingestion/contracts-finder/SKILL.md` | Contracts Finder agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/procurement-ingestion/pcs/SKILL.md` | PCS agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/supplier-price-enrichment/ebay/SKILL.md` | eBay pricing agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/supplier-price-enrichment/amazon/SKILL.md` | Amazon pricing agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/supplier-price-enrichment/alibaba/SKILL.md` | Alibaba pricing agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/supplier-price-enrichment/1688/SKILL.md` | 1688 wholesale pricing agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/document-acquisition/SKILL.md` | public tender document download playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/lot-extraction/SKILL.md` | lot structure extraction from unstructured documents | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/product-extraction/SKILL.md` | product candidate extraction with candidateType and structured slots | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/preliminary-qualification/SKILL.md` | expiry + goods reasoning | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/manifest-generation/SKILL.md` | manifest creation workflow | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/bid-portal-proactis/SKILL.md` | first bidding portal workflow | custom |

### 8.3 Test Files Required

#### Agent E2E Tests (HIGHEST PRIORITY)

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/agent_pty_session.rs` | PTY session creation, output capture, full lifecycle including reap and cleanup verdict |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/agent_contract_session.rs` | Contract-mode session creation, structured event capture, result contract validation |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/agent_procurement_ingestion.rs` | Real agent session running procurement ingestion skill, verifying output and result contract |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/agent_supplier_enrichment.rs` | Real agent session running supplier enrichment skill, verifying output and result contract |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/agent_health_detection.rs` | Stall detection, auth challenge detection, sleep interruption, error classification |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/agent_error_manifest.rs` | Error manifest creation on agent failures with full runtime context |

#### Adapter Contract Tests

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/adapter_codex.rs` | Codex adapter capability probe, JSONL parsing, mode selection, canary mode |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/adapter_claude.rs` | Claude adapter capability probe, JSON parsing, mode selection, canary mode, protocol-mismatch detection |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/adapter_result_contract.rs` | Result contract validation, checkpoint sequencing, partial failure classification |

#### Fault-Injection Tests

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/fault_process_cleanup.rs` | Orphaned grandchildren, dropped child handle, cleanup timeout, leaked process detection |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/fault_pty_edge_cases.rs` | PTY EOF/EIO handling, fd leak scenarios, terminal geometry stability |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/fault_host_events.rs` | Host sleep/monotonic gap, resource exhaustion, concurrent session pressure |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/fault_partial_failure.rs` | Partial checkpoint then crash, result contract validation failures, auth challenge mid-run |

#### Deterministic Replay Tests

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/replay_pty_sessions.rs` | Replay captured PTY byte streams against adapter parser and health monitor |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/replay_contract_sessions.rs` | Replay captured JSONL/JSON sessions against adapter parser and result contract validator |

#### Integration Tests (Real Databases, No Mocks)

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/sqlite_schema.rs` | SQLite schema creation and migration |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/sqlite_supabase_sync.rs` | Push-pull sync between SQLite and Supabase |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/job_claim.rs` | Job claiming and lease management in SQLite |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/cron_scheduler.rs` | Cron-based job scheduling |

#### Unit Tests (Deterministic Logic)

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/relations/resolve-process-relations.test.ts` | relation-resolution scoring and hard blockers |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/qualification/goods.test.ts` | goods/services qualification |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/qualification/quoteability.test.ts` | quoteability thresholds and candidate routing |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/hmrc-tariff.test.ts` | tariff parsing |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/hmrc-fpo.test.ts` | FPO HS code classification |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/margins/net-margin.test.ts` | gross and net margin rules |

#### Browser E2E Tests

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/e2e/contracts.spec.ts` | desktop control panel browsing and decision flow |
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/e2e/contracts.spec.ts` | web viewing portal read-only browsing |

---

## 9. Dependencies & Prerequisites

### 9.1 New Dependencies

**Production (TypeScript):**

| Package | Purpose |
|---------|---------|
| `@supabase/supabase-js` | auth, realtime, database access |
| `zod` | runtime validation |
| `react` | UI |
| `next` or `vite` shell dependencies | frontend shell |
| `marked` or equivalent | manifest rendering |
| `playwright` runtime libs where needed | browser automation integration helpers |

**Production (Rust):**

| Package | Purpose |
|---------|---------|
| `tokio` | async runtime |
| `rusqlite` or `sqlx` (SQLite feature) | local SQLite access |
| `sqlx` (Postgres feature) | Supabase/Postgres sync |
| `reqwest` | HTTP clients (cost-estimation APIs) |
| `serde` / `serde_json` | serialization |
| `tracing` | structured logging |
| `chrono` | date/time |
| `anyhow` / `thiserror` | error handling |
| `nix` | POSIX process-group and signal handling |
| `portable-pty` or `pty-process` | PTY session management |
| `cron` | cron expression parsing for local scheduler |
| `sysinfo` | host resource pressure detection for the resource arbiter |
| `uuid` | UUID generation for SQLite (no gen_random_uuid) |
| `sha2` | SHA-256 hashes for artifacts and result contracts |
| `zstd` | compressed transcript and artifact storage |

**Development:**

| Package | Purpose |
|---------|---------|
| `vitest` | unit tests |
| `playwright` | E2E and source workflow tests |
| `typescript` | TS build |
| `turbo` or equivalent | monorepo task orchestration |
| `supabase` CLI | local stack and migrations |

### 9.2 Configuration Changes

| Config File | Changes |
|-------------|---------|
| `.env.example` | Supabase URLs/keys (sync target), Freightos, SeaRates, Xe, Taric Support, optional HS Ping/CurrencyBeacon/OANDA/comparison-provider keys, SQLite path, agent CLI paths |
| `apps/desktop/src-tauri/capabilities/default.json` | allow specific shell/sidecar commands only |
| `apps/desktop/src-tauri/tauri.conf.json` | sidecar and desktop build configuration |
| `infra/supabase/*` | local database and RLS policies |

### 9.3 External Accounts / Access

| Service | Need |
|---------|------|
| Supabase | shared sync target DB/auth/storage |
| HMRC Trade Tariff Developer Portal | tariff API + FPO API access |
| HMRC Exchange Rate Service | customs valuation FX rates (free, no registration needed) |
| Xe | commercial currency API |
| Freightos | freight market pricing access |
| SeaRates | complementary freight API access |
| Easyship | parcel-mode freight and tax/duty estimation (supporting source) |
| Shippo | parcel-mode carrier rates and international shipping (supporting source) |
| Taric Support | commodity-code validation API access |
| HS Ping | optional HS-code comparison access |
| CurrencyBeacon | optional backup FX access |
| OANDA | optional historical FX access |
| Dutify / Zonos / Avalara | optional landed-cost comparison providers |
| Codex/Claude CLI credentials | agent CLI authentication for PTY sessions |
| (No marketplace API accounts needed) | supplier data collected via agent skill scraping, not APIs |

### 9.4 Migration Requirements

**Schema coordination rule:** Both SQLite and Postgres schemas must derive from the same canonical logical schema. CI must apply both migration sets from scratch, introspect both, and fail if the logical schema hash differs.

| Migration | Database | Purpose | Reversible |
|-----------|----------|---------|------------|
| SQLite state initial | SQLite state.db | Pipeline entities (procurement_processes, contract_notices, notice_lots, qualification_runs, product_candidates, supplier_quotes, landed_cost_estimates, manifests, pipeline_assessments, bid_workflow_states, process_relations, search_profiles), sync tracking columns, junction tables | Yes |
| SQLite sync infra | SQLite state.db | local_outbox, sync_batches, pull_cursors, stage tracking (process_stage_status, process_stage_heads) | Yes |
| SQLite artifacts | SQLite artifacts.db | Agent session logs (with orchestration domain columns), raw documents, error manifests | Yes |
| Postgres initial | Supabase | Projection tables (all push-owned entities including notice_lots, process_relations, search_profiles, workflow_job_summaries, error_manifest_summaries), remote-owned tables (shared_company_profile, shared_source_configs, operator_decision_heads) | Yes |
| Postgres RLS | Supabase | Authority-based RLS per role (viewer, operator, admin, daemon) — see §6.3 for policy matrix | Yes |
| Postgres sync apply | Supabase | Transactional RPC for push batch application + operator decision CAS RPC | Yes |
| Postgres schema metadata | Supabase | schema_metadata table with schema_epoch | Yes |
| Postgres stage tracking | Supabase | process_stage_status, process_stage_heads projections | Yes |
| Postgres junction tables | Supabase | notice_cpv_codes, product_commodity_codes, lot_cpv_codes | Yes |

---

## 10. Open Questions

### Requiring User Input

1. **Is Scotland in the MVP ingestion scope?**
   - Context: This determines whether PCS is a first-wave scraping skill or a later phase.
   - Options: England/Wales/NI only, or full UK including Scotland.
   - Recommendation: Include PCS skill in Phase A only if Scotland matters commercially on day one.

2. ~~**Do you want one shared web/desktop shell or separate shells with shared packages?**~~
   - **RESOLVED:** Separate shells with clearly different roles. Desktop = control panel (full read/write/trigger). Web = read-only viewing portal.

3. ~~**What level of autonomy is acceptable for bid workflows in MVP?**~~
   - **RESOLVED:** Full automation of the request pipeline is acceptable. This is not real-money bidding — actual negotiations and fulfillment are handled manually by humans. Error manifest system flags failures for debugging.

4. **Should the margin model assume you are importing through a UK VAT-registered entity that can normally recover import VAT?**
   - Context: This materially changes whether import VAT is treated as an economic cost or primarily a cash-flow item.
   - Recommendation: model both views, but set the default based on your real importer-of-record structure.

5. **Which machine is expected to stay online for scheduled execution and freshness SLAs?**
   - Context: The plan now explicitly assumes one active local daemon per organization and catchup sweeps after downtime, but freshness still depends on at least one machine staying online.
   - Recommendation: identify one designated always-on workstation or mini-server-class host for the MVP.

6. ~~**Are browser-automation-based supplier price captures acceptable for high-value opportunities when official APIs are insufficient?**~~
   - **RESOLVED:** Yes — browser automation via agent skills is the primary data retrieval strategy for ALL procurement and supplier data, not just a fallback. This is a core architectural decision.

### Requiring Further Research

1. **Exact procurement source list by commercial importance**
   - Why unresolved: You described the platform broadly, but not which portals matter most beyond government contract sites.
   - Research needed: rank target sources by contract volume, goods density, and bidability.

2. **Product-family-specific logistics assumptions**
   - Why unresolved: shipping mode depends heavily on item class and packaging density.
   - Research needed: define default freight-mode heuristics for your target product families.

3. **Chinese supplier source strategy**
   - Why unresolved: official APIs exist, but practical access and data quality depend on your commercial relationships.
   - Research needed: validate which supplier platforms you can lawfully and reliably integrate.

### Assumptions Made

1. **Assumption:** UK procurement is the primary geography.
   - Impact if wrong: source layer design changes significantly.
   - Validation: confirm MVP geography.

2. **Validated:** Desktop-local execution is non-negotiable.
   - Evidence: resolved owner feedback and integrated ADR-003/ADR-004 architecture.
   - Remaining concern: confirm what machine is expected to stay online for freshness SLAs.

3. **Assumption:** Margin thresholds are business gates, not just UI labels.
   - Impact if wrong: workflow state design may be too rigid.
   - Validation: confirm whether 30% gross and 20% net are hard filters.

4. **Assumption:** Import VAT is usually recoverable because the operating importer is UK VAT-registered.
   - Impact if wrong: economic-margin modeling could materially overstate opportunity quality.
   - Validation: confirm importer-of-record and VAT-registration structure.

5. **Assumption:** Company profile data can be centrally stored with appropriate protections.
   - Impact if wrong: bid workflows need stronger local-only secret partitioning.
   - Validation: confirm data residency and storage comfort level.

---

## 11. Next Steps

### Immediate Actions

1. Confirm remaining open questions (Scotland scope, VAT entity structure).
2. Bootstrap the monorepo structure with the shell split (desktop control panel + web viewing portal).
3. Build the Rust daemon with PTY session management and local SQLite storage first — this is the critical infrastructure.
4. Build the first procurement ingestion skill (Find a Tender) and verify the full agent E2E flow.
5. Stand up Supabase schema and sync engine.
6. Build the cost-estimation API integrations (HMRC, freight, FX).

### Recommended Delivery Order

#### Milestone 1: Foundation + Daemon Core

- Workspace bootstrap
- Rust daemon with PTY session management
- Local SQLite schema and storage
- Daemon cron scheduler
- Agent health monitoring and log capture
- Error manifest system
- Basic Tauri desktop shell (control panel)
- Basic web shell (read-only viewing portal)

#### Milestone 2: Agent Ingestion Pipeline

- Find a Tender ingestion skill
- Contracts Finder ingestion skill
- Agent E2E tests verifying real PTY sessions
- Normalized notice list/detail UI in desktop
- Supabase schema + sync engine
- Web viewing portal reading from Supabase

#### Milestone 3: Qualification

- Expiry filter
- Goods/services classifier
- Product extraction

#### Milestone 4: Commercial Intelligence

- eBay supplier scraping skill
- Alibaba/1688 supplier scraping skill
- HMRC Trade Tariff API integration
- HMRC FPO API for HS code classification
- HMRC Exchange Rate Service integration
- Xe commercial FX integration
- Freightos + SeaRates freight API integration
- Net-margin view

#### Milestone 5: Operator Workflow

- Manifest generation
- Reject/watch/bid controls in desktop control panel
- Job history and agent session tracing pages
- Error manifest control-panel view

#### Milestone 6: Request Pipeline Automation

- One portal-specific bid skill
- Agent-browser request pipeline flow
- Screenshot/log artifact capture
- Full terminal output tracing for bid workflows

### Recommended Pipeline

```text
This Document
  -> research-to-spec
  -> implementation plan
  -> milestone execution
```

### Pre-requisites for Implementation

- [ ] Scope geography confirmed (Scotland in/out of MVP)
- [x] Desktop/web shell decision made — desktop = control panel, web = read-only viewing portal
- [ ] Initial procurement sources ranked by commercial importance
- [ ] Supabase project created (shared sync target)
- [x] Data retrieval strategy confirmed — skill-based agent scraping for procurement + supplier data
- [x] Cost-estimation API strategy confirmed — HMRC Tariff + FPO + Exchange Rates, Freightos + SeaRates, Xe
- [x] Agent orchestration strategy confirmed — PTY-first from day 1
- [x] Risk policy for automation confirmed — request pipeline automation, not real-money bids
- [ ] Codex/Claude CLI credentials set up for agent PTY sessions
- [ ] VAT entity structure confirmed for margin modeling

---

## Appendices

### A. Suggested Margin Logic

#### Margin Gate Overview

#### Revenue Normalization (Prerequisite)

Before any margin gate, normalize the revenue basis:

```text
revenue_basis_type = classify(notice):
  "firm_contract"       -> direct award or single-winner tender with firm value
  "estimated_contract"  -> tender with estimated (not guaranteed) value
  "framework_ceiling"   -> framework agreement ceiling (not guaranteed revenue)
  "dynamic_market"      -> DPS/dynamic market admission (no volume guarantee)
  "concession"          -> concession contract
  "unknown"             -> insufficient data

# Under Procurement Act guidance, estimated contract value is the maximum value
# payable and is inclusive of VAT. Framework ceilings are not guaranteed revenue.
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

**Reference:** https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-define-phase/guidance-valuation-of-contracts-html

#### Quote Normalization (Prerequisite)

Normalize all supplier quotes to a common cost boundary before comparison:

```text
# Incoterm determines what is already included in the supplier price.
# EXW: buyer pays everything from factory gate
# FOB: seller pays inland transport to port + export clearance
# CIF: seller pays freight + insurance to destination port
# DDP: seller pays everything including import duty + VAT
# Without normalization, comparing EXW to CIF quotes is inconsistent.

normalized_supplier_cost = normalize_to_exw(
  quoted_price, incoterm = quote.quoteCostBoundary,
  estimated_inland_freight, estimated_ocean_freight, estimated_insurance
)
```

**Reference:** https://iccwbo.org/business-solutions/incoterms-rules/

#### Gross Margin Gate

Use normalized ex-VAT revenue and normalized supplier cost:

```text
gross_margin = (expected_addressable_revenue_ex_vat - normalized_supplier_cost)
             / expected_addressable_revenue_ex_vat
gross_estimate_confidence = min(revenue_confidence, quote_realism_confidence)

if gross_margin >= 0.30 and gross_estimate_confidence >= confidence_floor:
  qualify_for_landed_cost_stage
else if gross_estimate_confidence < confidence_floor:
  route_to_manual_review
else:
  reject_or_deprioritize
```

#### Net Margin Gate

Use an ex-VAT economic margin view with **low/base/high scenarios**. Do not blindly subtract recoverable import VAT from economic margin if the importing entity is VAT-registered and can reclaim it.

**Note on duty scenarios:** The `customs_duty_high/base/low` values below are computed at
margin-calculation time from classification uncertainty (candidate codes with different duty
rates) and trade remedy risk. The stored `LandedCostEstimate` entity records `customsDutyGbp`
(base estimate) and `tradeRemedyDutyGbp` separately; the scenarios are derived and stored as
final `netMarginLowPct`/`netMarginBasePct`/`netMarginHighPct` values.

```text
net_margin_low = (
  expected_addressable_revenue_ex_vat
  - normalized_supplier_cost * (1 + negotiation_uplift_pct)
  - freight_high
  - customs_duty_high   # includes trade remedy duties if applicable
  - uk_final_mile - other_fees
) / expected_addressable_revenue_ex_vat

net_margin_base = (
  expected_addressable_revenue_ex_vat
  - normalized_supplier_cost
  - freight_base - customs_duty_base
  - uk_final_mile - other_fees
) / expected_addressable_revenue_ex_vat

net_margin_high = (
  expected_addressable_revenue_ex_vat
  - normalized_supplier_cost * (1 - volume_discount_pct)
  - freight_low - customs_duty_low
  - uk_final_mile - other_fees
) / expected_addressable_revenue_ex_vat

cash_to_clear = (
  normalized_supplier_cost + freight_base + customs_duty_base
  + uk_final_mile + other_fees
  + import_vat * (1 - vat_recovery_rate)    # non-recoverable portion
  + import_vat * vat_recovery_rate           # recoverable but ties up cash
)

# Optional: working-capital-adjusted margin
working_capital_cost = cash_to_clear * cost_of_capital * (days_cash_tied / 365)
wc_adjusted_margin = net_margin_base
  - (working_capital_cost / expected_addressable_revenue_ex_vat)

# Gate logic: use CONSERVATIVE (low) estimate for auto-qualification
if net_margin_low >= 0.20 and no critical_blockers:
  final_qualified
else if net_margin_base >= 0.20 and (quote_class == "indicative_marketplace"
                                     or has_blockers):
  manual_review_required / rfq_required
else:
  not_qualified

# Critical blockers override margin regardless of value:
critical_blockers = [
  classification_unstable,        # HS6 diverges or 10-digit unresolved
  trade_remedy_detected,          # anti-dumping/countervailing/safeguard
  measure_condition_unsatisfied,  # operational blocker
  revenue_basis_uncertain,        # framework ceiling or dynamic market
  pva_ineligible,                 # cannot recover import VAT
]
```

### B. Suggested Confidence Model

Use a weighted confidence score as an internal heuristic only. The weights below are priors for MVP and should be recalibrated against observed operator decisions and realized sourcing outcomes rather than treated as statistically validated probabilities.

```text
final_confidence =
  (0.20 * classification_stability) +
  (0.20 * quote_realism) +
  (0.20 * commodity_code_confidence) +
  (0.15 * freight_mode_stability) +
  (0.15 * revenue_reliability) +
  (0.10 * fx_freshness)
```

This composite applies only after a candidate has already passed the upstream pipeline gates in §5.7 (document acquisition, lot extraction, quoteability, and price-signal matching). Those earlier stages determine whether the system has enough evidence to compute a commercial-confidence score at all; they are not flattened into the final weighted number.

Then apply:

- `>= 0.80` -> high confidence
- `0.60 - 0.79` -> usable with review
- `< 0.60` -> manual review required

**Critical blocker override:** A high composite score MUST NOT hide a red classification, revenue, or trade-remedy blocker. If any critical blocker is present (see Appendix A critical_blockers list), the opportunity cannot auto-qualify regardless of the composite score.

**Operator display:** Do not show operators a single decimal confidence number (e.g., `0.72`). Instead, show:
- **Band:** high / usable / manual review
- **Component panel** with individual scores for:
  - Classification stability (are HS6 candidates converging? is 10-digit resolved?)
  - Quote realism (marketplace indicative vs distributor vs negotiated?)
  - Freight mode stability (is the mode resolved? is packaging state known?)
  - Revenue reliability (firm contract vs framework ceiling vs dynamic market?)
  - FX freshness (how old is the rate data? what is the basis gap?)
- **Blockers:** Any critical blockers that prevent auto-qualification, shown prominently

Keep the simple heuristic internally for pipeline routing, but surface bands, component scores, and blockers in the UI — not fake decimal precision.

For implementation, `gross_estimate_confidence` and `net_estimate_confidence` should be stage-specific projections of the same confidence framework. A reasonable initial `confidence_floor` for automated progression is `0.60`, but this should be calibrated against operator outcomes.

### C. Suggested Job Kinds

| Job Kind | Purpose |
|----------|---------|
| `sync_source` | pull procurement notices (one session per source per run) |
| `normalize_notice` | normalize into internal schema |
| `resolve_relations` | classify notice-to-process relations (`merged_same_process` / `related_not_merged` / `possible_duplicate` / `separate`) |
| `qualify_notice` | expiry + access route + multi-dimensional goods classification |
| `acquire_documents` | download public tender documents (spec PDFs, BoQ, lot docs) |
| `extract_lots` | extract lot structure from notices and documents |
| `extract_products` | per-lot product candidate extraction with candidateType and structured slots |
| `score_quoteability` | deterministic quoteability gate before marketplace search |
| `enrich_supplier_prices` | price-signal matching via marketplace scraping (batched per source per notice) |
| `estimate_landed_cost` | customs/logistics/FX with margin bands |
| `generate_manifest` | Markdown summary |
| `run_bid_workflow` | portal-specific bid assistance |
| `escalate_rfq` | escalate to RFQ for high-value opportunities with indicative-only pricing |

### D. Source References

#### Procurement Data

- Find a Tender data and API docs: https://www.find-tender.service.gov.uk/Developer/Documentation
- Find a Tender OCDS release packages: https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsReleasePackages
- Find a Tender OCDS record packages: https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsRecordPackages
- Find a Tender notice types: https://www.find-tender.service.gov.uk/Home/NoticeTypes
- Central Digital Platform factsheet: https://www.gov.uk/government/publications/procurement-act-2023-short-guides/central-digital-platform-factsheet-html
- Central Digital Platform guidance: https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-procure-phase/guidance-central-digital-platform-and-publication-of-information-html
- Contracts Finder API help: https://www.contractsfinder.service.gov.uk/apidocumentation
- Contracts Finder OCDS search: https://www.contractsfinder.service.gov.uk/apidocumentation/Notices/1/GET-Published-Notice-OCDS-Search
- Contracts Finder daily CSV: https://www.contractsfinder.service.gov.uk/apidocumentation/Notices/1/GET-Harvester-Notices-Data-CSV-Daily
- UK OCDS profile note: https://www.gov.uk/government/publications/open-standards-for-government/open-contracting-data-standard-profile
- PCS API: https://api.publiccontractsscotland.gov.uk/v1

#### Customs / Tariff / VAT

- Trade Tariff developer portal: https://hub.trade-tariff.service.gov.uk/
- Trade Tariff API getting started: https://api.trade-tariff.service.gov.uk/getting-started.html
- Trade Tariff Commodities API: https://api.trade-tariff.service.gov.uk/the-commodities-api.html
- Trade Tariff service: https://www.gov.uk/trade-tariff
- Prepare customs value: https://www.gov.uk/guidance/prepare-to-work-out-the-customs-value-of-your-imported-goods
- Method 1 valuation: https://www.gov.uk/guidance/valuing-imported-goods-using-method-1-transaction-value
- VAT from customs value: https://www.gov.uk/guidance/working-out-the-vat-value-using-the-customs-value-of-the-imported-goods
- Advance Tariff Ruling: https://www.gov.uk/guidance/apply-for-an-advance-tariff-ruling

#### Supplier Pricing

- eBay Browse API: https://developer.ebay.com/api-docs/buy/static/api-browse.html
- eBay Browse overview: https://developer.ebay.com/api-docs/buy/browse/static/overview.html
- eBay getItem: https://developer.ebay.com/api-docs/buy/browse/resources/item/methods/getItem
- eBay release notes: https://developer.ebay.com/api-docs/buy/browse/release-notes.html
- Amazon PA-API docs: https://webservices.amazon.com/paapi5/documentation/
- Amazon PA-API deprecation/register page: https://webservices.amazon.com/paapi5/documentation/register-for-pa-api.html
- Amazon Associates term changes / Creators API transition: https://affiliate-program.amazon.com/help/operating/compare
- Taobao Global Open Platform: https://open.taobao.global/doc/doc.htm
- Taobao Global API reference: https://open.taobao.global/doc/api.htm
- Alibaba Open API intro: https://developer.alibaba.com/docs/doc.htm?articleId=118416&docType=1&treeId=684

#### Freight / FX / Platform

- Freightos APIs: https://developers.freightos.com/apis
- Freightos price stats: https://apidocs.freightos.com/reference/get_price-stats
- Freightos price stats detail: https://apidocs.freightos.com/reference/getpricestats-1
- SeaRates Logistics Explorer API: https://docs.searates.com/reference/logistics/get-rates
- Easyship tax and duty: https://developers.easyship.com/reference/taxes_and_duties_calculate
- Shippo docs: https://docs.goshippo.com/
- Shippo public API: https://docs.goshippo.com/shippoapi/public-api
- Shippo international shipping: https://docs.goshippo.com/docs/international_shipping/internationalshipping
- XE Currency Data API landing page: https://www.xe.com/en-us/xecurrencydata/
- XE API overview: https://help.xe.com/hc/en-gb/articles/4411624652177-What-is-the-Currency-Data-API
- XE API behavior: https://help.xe.com/hc/en-gb/articles/4411633342225-How-does-the-Currency-Data-API-work
- XE API packaging: https://help.xe.com/hc/en-gb/articles/4414092026769-Currency-Data-API-packages-pricing-and-payment
- HMRC Exchange Rate Service: https://trade-tariff.service.gov.uk/exchange_rates/monthly
- HMRC Exchange Rate XML API: https://developer.service.hmrc.gov.uk/api-documentation/docs/api/xml/Exchange%20rates%20from%20HMRC
- CurrencyBeacon: https://currencybeacon.com
- OANDA Historical Currency Converter: https://fxds-hcc.oanda.com

#### HS Code Classification

- HMRC FPO API: https://api.trade-tariff.service.gov.uk/fpo.html
- HMRC FPO source code: https://github.com/trade-tariff/trade-tariff-lambdas-fpo-search
- HS Ping: https://hsping.com
- Taric Support API: https://taricsupport.com/api/

#### Third-Party Landed Cost Services

- Dutify: https://docs.dutify.com/docs/landed-cost-calculator
- Zonos: https://zonos.com
- Avalara: https://avalara.com

#### Customs Valuation / VAT / Trade Remedies / Special Procedures

- PVA eligibility: https://www.gov.uk/guidance/check-when-you-can-account-for-import-vat-on-your-vat-return
- Customs valuation Method 1: https://www.gov.uk/guidance/valuing-imported-goods-using-method-1-transaction-value
- Import VAT value from customs value: https://www.gov.uk/guidance/working-out-the-vat-value-using-the-customs-value-of-the-imported-goods
- Trade remedies introduction: https://www.gov.uk/government/publications/the-uk-trade-remedies-investigations-process/introduction-to-trade-remedies
- Inward Processing rules: https://www.gov.uk/guidance/special-procedure-inward-processing/rules-and-processes
- Origin proof requirements: https://www.gov.uk/guidance/get-proof-of-origin-for-your-goods
- Advance Tariff Ruling: https://www.gov.uk/guidance/apply-for-an-advance-tariff-ruling

#### Procurement Act / Contract Valuation

- Contract valuation guidance: https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-define-phase/guidance-valuation-of-contracts-html

#### Incoterms

- ICC Incoterms rules: https://iccwbo.org/business-solutions/incoterms-rules/

#### Runtime / Database / Skills

- Tauri Next guide: https://v2.tauri.app/start/frontend/nextjs/
- Tauri frontend guidance: https://v2.tauri.app/start/frontend
- Tauri shell plugin: https://v2.tauri.app/plugin/shell
- Tauri sidecar docs: https://v2.tauri.app/develop/sidecar/
- Tauri plugin permissions: https://v2.tauri.app/learn/security/using-plugin-permissions/
- Supabase architecture: https://supabase.com/docs/guides/getting-started/architecture
- Supabase RLS: https://supabase.com/docs/guides/database/postgres/row-level-security
- Supabase realtime changes: https://supabase.com/features/realtime-postgres-changes
- Supabase realtime auth: https://supabase.com/docs/guides/realtime/authorization
- Convex docs: https://docs.convex.dev/home
- Convex realtime: https://docs.convex.dev/realtime
- Convex auth: https://docs.convex.dev/auth
- Convex scheduling: https://docs.convex.dev/scheduling
- SpacetimeDB Maincloud: https://spacetimedb.com/docs/deploying/maincloud/
- SpacetimeDB self-hosting: https://spacetimedb.com/docs/deploying/spacetimedb-standalone
- SpacetimeDB auth: https://spacetimedb.com/docs/spacetimeauth
- OpenAI Codex skills: https://developers.openai.com/codex/skills/
- Claude agent skills: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview
- Agent-browser skill reference: https://github.com/vercel-labs/agent-browser/blob/main/skills/agent-browser/SKILL.md

### E. Change Log

| Date | Author | Change |
|------|--------|--------|
| 2026-03-08 | idea-to-research | Initial research document |
| 2026-03-11 | harmonymaxer (Run 001) | Harmonized owner feedback: skill-first agent scraping, PTY-first execution, local SQLite + Supabase sync, desktop control panel + web viewing portal, expanded cost-estimation sources (HMRC FPO, SeaRates, HMRC Exchange Rates, CurrencyBeacon, OANDA, HS Ping, Taric Support, Dutify, Zonos, Avalara), real E2E testing, comprehensive agent tracing, error manifest system, relaxed automation policy. Revised ADR-001, ADR-002, ADR-004, ADR-005, ADR-007, ADR-008. |
| 2026-03-12 | planmaxer pre-prep (Agent Orchestration) | Integrated expert review: hybrid PTY + contract execution model via adapter boundary, three-layer observability (control/protocol/forensic), process-group supervision with cleanup verdicts, explicit skill binding for cron, skill metadata declarations, artifact store for session bytes outside SQLite, resource arbiter with typed quotas, expanded error taxonomy (AUTH_CHALLENGE, OUTPUT_CONTRACT_INVALID, PROTOCOL_MISMATCH, PARTIAL_FAILURE, CLEANUP_INCOMPLETE, HOST_SLEEP_INTERRUPTED, HOST_RESOURCE_EXHAUSTED), deterministic replay + fault-injection + adapter contract tests. Updated ADR-004, ADR-005. Sections modified: 4.9, 4.10, 5.2, 5.4 (note), 5.6, 5.11, 6.5, 6.6, 8.2, 8.3. |
| 2026-03-12 | planmaxer pre-prep (procurement pipeline) | Integrated expert review feedback on procurement pipeline and product matching. Major changes: (1) Three-way relation resolution replacing binary merge (merged/related/separate) with hard blockers and conservative heuristics in §5.6. (2) Multi-dimensional goods classification (legalCategory, goodsDensityScore, goodsSeparability, accessRoute) replacing single goods/services label in §5.7. (3) New pipeline stages: document acquisition, lot extraction, product extraction with candidateType, quoteability scoring, price-signal matching with source routing in §5.3/§5.7. (4) NoticeLot entity, ProcessRelation edges, SearchProfile entity, expanded ProductCandidate and SupplierQuote fields noted for §5.4 data-architecture domain. (5) Dual confidence scoring (matchConfidence + quoteUsabilityConfidence) and price-signal role classification in §4.3/§5.7. (6) Cross-language matching strategy (translate slots not prose) in §4.3. (7) Pipeline cost management and throughput optimization in §5.7. (8) New risks in §6.2: attachment blindness, lot blindness, false bidability, compliance false positives, pack-basis errors, price-signal false precision, FPO access risk. (9) Updated complexity analysis in §6.1. (10) New skills: document-acquisition, lot-extraction, product-extraction in §4.10. (11) Updated state model in §5.5 and job kinds in Appendix C. Sections modified: 4.1, 4.2, 4.3, 4.10, 5.2, 5.3, 5.4 (notes), 5.5, 5.6, 5.7, 6.1, 6.2, Appendix C, Appendix E. |
| 2026-03-12 | planmaxer (landed-cost review) | Integrated expert review: FPO reframed as candidate generator, 10-digit code resolution + measure scanning, classification preprocessing pipeline, Incoterm normalization, revenue basis normalization, low/base/high margin scenarios with critical blockers, expanded VAT treatment (PVA eligibility), dual FX with basis gap, freight mode decision tree, special procedures, screening/decision grade distinction, confidence model with blocker override, 12 new error codes. Updated 4.4, 4.5, 4.6, 5.2, 5.4 (data model + SQL), 5.7, 5.11, A, B, D, ADR-006. |
| 2026-03-12 | planmaxer steps 3–4 (landed-cost) | Fresh-eyes optimization + sync pass: added missing pipeline steps (ukFinalMileCostGbp, otherFeesGbp, customsValuationMethod, confidenceBreakdown population), updated 5.4 SQL landed_cost_estimates table to match TypeScript interface (27 new/renamed columns), added Easyship and Shippo to 9.3 External Accounts, added cross-domain coordination note for ProductCandidate → freight mode data flow, added 8 new intelligence engine files to 8.2 (classification preprocessing, 10-digit resolution, measure scanning, revenue/quote normalization, margin scenarios, Easyship/Shippo adapters), added missing APIs to 4.11 licensing table. Sections modified: 4.11, 5.4, 5.7, 8.2, 9.3, Appendix D. |
| 2026-03-12 | planmaxer pre-prep (data architecture) | Integrated expert review on data architecture and sync. Major changes: (1) Authority matrix replacing singular "source of truth" — three domains: local operational, remote shared, remote projections. Mixed-write tables prohibited. (2) Split opportunity_decisions into pipeline_assessments (daemon-owned), operator_decision_heads (remote-owned, CAS), bid_workflow_states (daemon-owned). (3) Transactional outbox pattern for push sync. (4) Stage coherence via process_stage_status + process_stage_heads. (5) Push protocol: monotonic local_seq, row_version, content_hash, atomic remote RPC. (6) Pull protocol: remote_change_seq cursors, compare-and-swap for decisions. (7) Schema migration coordination with schema_epoch. (8) Recovery model honesty: warm vs cold rebuild. (9) SQLite operational guidance: state.db/artifacts split. (10) Tombstone policy: only remote-owned entities. (11) Junction tables for cpv_codes and commodity_codes. (12) Integer pence for money, basis points for percentages. (13) MVP: one daemon per org. Sections modified: 4.7, 5.2, 5.4, 5.5, 5.6, 5.7, 5.8, 6.3, 6.4, 8.2, 9.4, ADR-002, Appendix E. |
| 2026-03-12 | planmaxer step 3 (data architecture) | Fresh-eyes optimization: added sync tracking columns to 8 push-owned tables, converted monetary columns to integer pence and confidence to basis points, incorporated cross-domain fields (NoticeLot, ProcessRelation, SearchProfile entities; expanded ProductCandidate, SupplierQuote, QualificationResult, agent_session_logs), removed array columns in favor of junction tables, added RLS policy specifications, updated SQLite compatibility notes, added raw_documents/sources to authority matrix. Sections modified: 4.7 (authority matrix table, SQLite compat notes), 5.4 (all SQL tables, new cross-domain entity tables), 6.3 (RLS policy specs). |
| 2026-03-12 | planmaxer step 4 (data architecture) | Sync pass: updated §8.2 migration file listings and §9.4 migration requirements to reflect new cross-domain entity tables (notice_lots, lot_cpv_codes, process_relations, search_profiles) and expanded column sets. |
| 2026-03-12 | planmaxer step 3+4 (Agent Orchestration) | Fresh-eyes optimization + sync pass: fixed hybrid-model terminology in 4.10 skill lifecycle and 6.5 test priority table, updated error manifest drill-down to reference session artifacts (5.11), added 3 missing health detection rows to 6.6 (HOST_RESOURCE_EXHAUSTED, AGENT_PROTOCOL_MISMATCH, AGENT_RUN_FAILED), added missing fault-injection test for AGENT_RUN_FAILED in 6.5, verified 8.2/8.3 naming patterns and ADR-004/ADR-005 status fields, verified ToC anchors. At that point, Supplier Quote Engine and Bid Workflow Executor still referenced PTY-only wording, and 5.6 still had a step-numbering collision; both were resolved later in the holistic pass. |
| 2026-03-12 | planmaxer steps 3–4 (procurement pipeline) | Fresh-eyes optimization and sync pass. §5.5: resolved rfq_escalated ghost state (clarified as operator disposition action), below-quoteability path, needs_review disambiguation (review_reason + review_stage), expired trigger mechanism (daemon background job), prelim_rejected entry conditions. §5.6: fixed step numbering collision — relation resolution renumbered to steps 13/13a–13e (was 10/10a–10e, colliding with data-arch agent adapter steps 10–12). §5.4: added cross-domain reconciliation note (quoteRealismConfidence from data-arch and quoteUsabilityConfidence from procurement pipeline are complementary, not duplicates). §8.2: added 3 missing intelligence pipeline skill file entries (document-acquisition, lot-extraction, product-extraction). Appendix C job kinds verified complete. §5.3 canonical workflow verified coherent. |
| 2026-03-12 | planmaxer holistic pass | Cross-plan unification pass. Fixed stale PTY-only terminology in the system context and workflow-trigger sections to match the adapter-mediated hybrid runtime; aligned §5.3 margin gates with Appendix A; updated §5.4 TypeScript entities and SQL comments to match the integrated procurement/data-architecture/landed-cost model; added missing `workflow_job_summaries` and `error_manifest_summaries` projection tables to resolve orphaned references in §§4.7/6.3/5.11; added explicit MVP operating limits and quote staleness invariants; corrected Bun workspace file references; extended §8.2/§8.3 and §9.1/§9.4 so file maps, tests, and dependencies cover relation resolution, quoteability, search-profile building, artifact compression, and runtime resource detection. |
| 2026-03-12 | planmaxer holistic pass (step 2 follow-up) | Completed the remaining cross-plan consistency cleanup. Added terminology notes for skill/playbook, process/opportunity, desktop/web surfaces, and shared-sync-target authority language; referenced Source Registry, Resource Arbiter, and summary projections directly in the runtime flows; switched the residual procurement/browser runtime wording from PTY-only to adapter-mediated sessions where appropriate; tightened the session-capture invariant so contract-mode runs also retain raw stdout/stderr forensic output; added sync delivery timeout and transcript chunking/truncation guardrails; made automatic search limits explicit per notice and per candidate; added missing `sha2` dependency and `sources/registry.rs` file mapping; removed the orphaned `operator_decision_events` mention; added the missing TypeScript interfaces for `NoticeLot`, `ProcessRelation`, and `SearchProfile`, and aligned `goodsSeparability` with the tri-state model used elsewhere; clarified that local execution is now a validated assumption and added a new user-input question for the designated always-on execution machine; clarified in Appendix B that the final confidence score applies after upstream qualification gates. |
| 2026-03-12 | planmaxer holistic pass (step 3) | Architectural coherence pass. Added the missing `Job Runner` component and clarified the scheduler-vs-runner split; tightened the SQLite Local Store vs Sync Engine boundary; added explicit dependency-direction notes (skills loaded by daemon, web portal depends only on Supabase projections, landed-cost stays downstream of extraction/matching); documented that complexity is intentionally concentrated in PTY/runtime orchestration and product/pricing comparability; aligned workflow/state/component text with the four-way relation model (`possible_duplicate` included). |
| 2026-03-12 | planmaxer holistic pass (step 4) | Sync pass. Verified the Table of Contents against the actual section structure; checked section/ADR/appendix cross-references and found them consistent; confirmed Appendix D category coverage and 9.3 external-account coverage; updated Quick Stats to reflect the current planned file count and expanded highest-risk area; corrected Appendix C job-kind wording to the four-way relation model. |
| 2026-03-12 | planmaxer holistic pass (step 2 residual) | Final residual consistency cleanup. Normalized remaining PTY-only wording where the runtime now preserves raw output in both PTY and contract modes; clarified recovery, security, and ADR-004 language to refer to full raw session-output artifacts rather than only PTY terminal transcripts; updated legacy `terminal_output_*` SQL comments to match the hybrid runtime contract. |
| 2026-03-12 | planmaxer holistic pass (step 2 residual 2) | Finalized the last hybrid-runtime consistency edits. Updated `Artifact Store` and Layer 3 observability wording to describe raw session-output capture in both PTY and contract modes, and aligned the illustrative `resolveConfidenceGate` example with the document-wide `0.60` starting confidence floor. |
| 2026-03-12 | planmaxer holistic pass (step 2 residual 3) | Closed the last remaining live wording gaps in the current document. Updated supplier-search and bid-workflow flows plus the daemon-complexity row to refer to full raw session output rather than PTY-only output where the runtime is hybrid, and made Appendix C use the canonical four-way relation labels. |
| 2026-03-12 | planmaxer holistic pass (step 3 residual) | Tightened the remaining architecture-language gaps. Added an explicit acyclic coupling rule to the coherence notes, updated the relation-resolution complexity row to the four-way model, and normalized the last user-facing / portal-facing wording to refer to full session output or raw session transcripts rather than PTY-only terminology where the runtime is hybrid. |
| 2026-03-12 | planmaxer holistic pass (step 3 residual 2) | Finalized the last boundary clarifications. Narrowed `PTY Session Manager` to live PTY-backed supervision while leaving final cleanup ownership to `Session Reaper`, reframed `Resource Arbiter` as admission control rather than a second scheduler, and made the complexity-allocation note explicit that the landed-cost engine remains very high complexity while intentionally staying downstream of the orchestration-critical boundaries. |
| 2026-03-12 | planmaxer holistic pass (step 4 residual) | Final sync pass. Rechecked the Table of Contents against the current section layout, verified section/ADR/appendix cross-references still line up, confirmed Appendix D and 9.3 remain complete for the cited services, and corrected the Quick Stats file-count total to the current 131 planned files. |

---

*Generated by idea-to-research skill*

# AGENTS.md — Procurement Intelligence Platform

> Guidelines for AI coding agents working in this codebase.

---

## RULE 0 - THE FUNDAMENTAL OVERRIDE PREROGATIVE

If I tell you to do something, even if it goes against what follows below, YOU MUST LISTEN TO ME. I AM IN CHARGE, NOT YOU.

---

## RULE NUMBER 1: NO FILE DELETION

**YOU ARE NEVER ALLOWED TO DELETE A FILE WITHOUT EXPRESS PERMISSION.** Even a new file that you yourself created, such as a test code file. You have a horrible track record of deleting critically important files or otherwise throwing away tons of expensive work. As a result, you have permanently lost any and all rights to determine that a file or folder should be deleted.

**YOU MUST ALWAYS ASK AND RECEIVE CLEAR, WRITTEN PERMISSION BEFORE EVER DELETING A FILE OR FOLDER OF ANY KIND.**

---

## Irreversible Git & Filesystem Actions — DO NOT EVER BREAK GLASS

1. **Absolutely forbidden commands:** `git reset --hard`, `git clean -fd`, `rm -rf`, or any command that can delete or overwrite code/data must never be run unless the user explicitly provides the exact command and states, in the same message, that they understand and want the irreversible consequences.
2. **No guessing:** If there is any uncertainty about what a command might delete or overwrite, stop immediately and ask the user for specific approval. "I think it's safe" is never acceptable.
3. **Safer alternatives first:** When cleanup or rollbacks are needed, request permission to use non-destructive options (`git status`, `git diff`, `git stash`, copying to backups) before ever considering a destructive command.
4. **Mandatory explicit plan:** Even after explicit user authorization, restate the command verbatim, list exactly what will be affected, and wait for a confirmation that your understanding is correct. Only then may you execute it—if anything remains ambiguous, refuse and escalate.
5. **Document the confirmation:** When running any approved destructive command, record (in the session notes / final response) the exact user text that authorized it, the command actually run, and the execution time. If that record is absent, the operation did not happen.

---

## Git Branch: ONLY Use `main`

**The default branch is `main`.**

- **All work happens on `main`** — commits, PRs, feature branches all merge to `main`
- **Never reference `master` in code or docs** — if you see `master` anywhere, it's a bug that needs fixing

---

## Toolchain

This is a **multi-stack project**. Different layers use different package managers:

| Layer | Stack | Package Manager |
|-------|-------|-----------------|
| **Daemon** (`crates/daemon`, `crates/common`) | Rust | **Cargo** |
| **Desktop** (`apps/desktop`) | Tauri v2 + React/Vite | **Bun** + Cargo |
| **Web** (`apps/web`) | Next.js / React | **Bun** |
| **Packages** (`packages/*`) | TypeScript | **Bun** |

### Rust (Daemon + Common Crates)

- **Package Manager:** Cargo only — NEVER npm/yarn/pnpm for Rust code
- **Edition:** Rust 2024 (nightly required — see `rust-toolchain.toml`)
- **Dependency versions:** Explicit versions for stability
- **Configuration:** Cargo.toml only
- **Unsafe code:** Forbidden (`#![forbid(unsafe_code)]`)

### Bun (Desktop, Web, Packages)

- **Package Manager:** Bun only — NEVER npm/yarn/pnpm for TypeScript layers
- **Lockfile:** `bun.lockb` (binary lockfile)
- **Commands:**
  - `bun install` — Install dependencies
  - `bun run dev` — Development server
  - `bun run build` — Production build
  - `bun run tauri dev` — Tauri development mode
  - `bun run tauri build` — Tauri production build

### Release Profile (Rust)

The release build optimizes for binary size:

```toml
[profile.release]
opt-level = "z"     # Optimize for size (lean binary for distribution)
lto = true          # Link-time optimization
codegen-units = 1   # Single codegen unit for better optimization
panic = "abort"     # Smaller binary, no unwinding overhead
strip = true        # Remove debug symbols
```

---

## Code Editing Discipline

### No Script-Based Changes

**NEVER** run a script that processes/changes code files in this repo. Brittle regex-based transformations create far more problems than they solve.

- **Always make code changes manually**, even when there are many instances
- For many simple changes: use parallel subagents
- For subtle/complex changes: do them methodically yourself

### No File Proliferation

If you want to change something or add a feature, **revise existing code files in place**.

**NEVER** create variations like:
- `mainV2.rs`
- `main_improved.rs`
- `main_enhanced.rs`

New files are reserved for **genuinely new functionality** that makes zero sense to include in any existing file. The bar for creating new files is **incredibly high**.

---

## Backwards Compatibility

We do not care about backwards compatibility—we're in early development with no users. We want to do things the **RIGHT** way with **NO TECH DEBT**.

- Never create "compatibility shims"
- Never create wrapper functions for deprecated APIs
- Just fix the code directly

---

## Compiler Checks (CRITICAL)

**After any substantive code changes, you MUST verify no errors were introduced:**

```bash
# Check for compiler errors and warnings
cargo check --all-targets

# Check for clippy lints (pedantic + nursery are enabled)
cargo clippy --all-targets -- -D warnings

# Verify formatting
cargo fmt --check
```

If you see errors, **carefully understand and resolve each issue**. Read sufficient context to fix them the RIGHT way.

---

## Procurement Intelligence Platform — This Project

**This is the project you're working on.**

A **UK public procurement intelligence platform** that ingests official contract notice data daily, normalizes and stores it, qualifies it for relevance to physical-goods supply opportunities, enriches qualified notices with supplier pricing plus landed-cost estimates, surfaces high-margin opportunities in a desktop control panel and web viewing portal, and optionally launches request pipeline automation workflows using agent skills and browser automation.

**Core idea:** A Rust daemon runs locally, orchestrating AI agent sessions via PTY terminals and structured contract mode. Skill-based agent scraping via browser automation is the primary data retrieval mechanism for all procurement and supplier data. The daemon manages local SQLite storage (operational authority), syncs to Supabase/Postgres (shared authority + projections), runs a cron scheduler for automated pipeline execution, and provides full observability through a three-layer model (control events, protocol events, forensic bytes).

### Architecture at a Glance

1. **Data retrieval:** Skill-based agent scraping via browser automation for procurement notices and supplier pricing. Direct API access for cost-estimation data (HMRC Tariff, freight, FX).
2. **Agent orchestration:** Hybrid PTY + contract execution via adapter boundary. Rust daemon with cron + manual triggers from desktop.
3. **Database:** Authority-matrix architecture — local SQLite (operational authority) + Supabase/Postgres (shared authority + projections). Mixed-write tables prohibited.
4. **Frontend:** Desktop (Tauri) = control panel (full read/write/trigger), Web = read-only viewing portal.
5. **Pipeline:** Ingest → normalize → relate → qualify → acquire docs → extract lots → extract products → score quoteability → price-signal match → gross margin gate → landed cost → net margin gate → manifest → operator decision → bid workflow.

### Key Source of Truth

The foundational architecture is defined in `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` (~4900+ lines). Always consult this document for architectural decisions, data models, component designs, and implementation guidance.

### Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Daemon startup | <500ms | SQLite open + schema check + cron init |
| Incremental ingestion | <5min/source | Per-source agent session |
| Desktop detail page | <2s | From warm local SQLite cache |
| Binary size (release) | <30MB | LTO + strip enabled |
| Idle memory (daemon) | <100MB | No leaks on long sessions |


---

## Third-Party Library Usage

If you aren't 100% sure how to use a third-party library, **SEARCH ONLINE** to find the latest documentation and current best practices. Use EXA MCP for this.

---

## MCP Agent Mail — Multi-Agent Coordination

A mail-like layer that lets coding agents coordinate asynchronously via MCP tools and resources. Provides identities, inbox/outbox, searchable threads, and advisory file reservations with human-auditable artifacts in Git.

### Why It's Useful

- **Prevents conflicts:** Explicit file reservations (leases) for files/globs
- **Token-efficient:** Messages stored in per-project archive, not in context
- **Quick reads:** `resource://inbox/...`, `resource://thread/...`

### Same Repository Workflow

1. **Register identity:**
   ```
   ensure_project(project_key=<abs-path>)
   register_agent(project_key, program, model)
   ```

2. **Reserve files before editing:**
   ```
   file_reservation_paths(project_key, agent_name, ["src/**"], ttl_seconds=3600, exclusive=true)
   ```

3. **Communicate with threads:**
   ```
   send_message(..., thread_id="FEAT-123")
   fetch_inbox(project_key, agent_name)
   acknowledge_message(project_key, agent_name, message_id)
   ```

4. **Quick reads:**
   ```
   resource://inbox/{Agent}?project=<abs-path>&limit=20
   resource://thread/{id}?project=<abs-path>&include_bodies=true
   ```

### Macros vs Granular Tools

- **Prefer macros for speed:** `macro_start_session`, `macro_prepare_thread`, `macro_file_reservation_cycle`, `macro_contact_handshake`
- **Use granular tools for control:** `register_agent`, `file_reservation_paths`, `send_message`, `fetch_inbox`, `acknowledge_message`

### Common Pitfalls

- `"from_agent not registered"`: Always `register_agent` in the correct `project_key` first
- `"FILE_RESERVATION_CONFLICT"`: Adjust patterns, wait for expiry, or use non-exclusive reservation
- **Auth errors:** If JWT+JWKS enabled, include bearer token with matching `kid`

---

## Beads (br) — Dependency-Aware Issue Tracking

Beads provides a lightweight, dependency-aware issue database and CLI (`br` - beads_rust) for selecting "ready work," setting priorities, and tracking status. It complements MCP Agent Mail's messaging and file reservations.

**Important:** `br` is non-invasive—it NEVER runs git commands automatically. You must manually commit changes after `br sync --flush-only`.

### Conventions

- **Single source of truth:** Beads for task status/priority/dependencies; Agent Mail for conversation and audit
- **Shared identifiers:** Use Beads issue ID (e.g., `br-123`) as Mail `thread_id` and prefix subjects with `[br-123]`
- **Reservations:** When starting a task, call `file_reservation_paths()` with the issue ID in `reason`

### Typical Agent Flow

1. **Pick ready work (Beads):**
   ```bash
   br ready --json  # Choose highest priority, no blockers
   ```

2. **Reserve edit surface (Mail):**
   ```
   file_reservation_paths(project_key, agent_name, ["src/**"], ttl_seconds=3600, exclusive=true, reason="br-123")
   ```

3. **Announce start (Mail):**
   ```
   send_message(..., thread_id="br-123", subject="[br-123] Start: <title>", ack_required=true)
   ```

4. **Work and update:** Reply in-thread with progress

5. **Complete and release:**
   ```bash
   br close 123 --reason "Completed"
   br sync --flush-only  # Export to JSONL (no git operations)
   ```
   ```
   release_file_reservations(project_key, agent_name, paths=["src/**"])
   ```
   Final Mail reply: `[br-123] Completed` with summary

### Mapping Cheat Sheet

| Concept | Value |
|---------|-------|
| Mail `thread_id` | `br-###` |
| Mail subject | `[br-###] ...` |
| File reservation `reason` | `br-###` |
| Commit messages | Include `br-###` for traceability |

---

## bv — Graph-Aware Triage Engine

bv is a graph-aware triage engine for Beads projects (`.beads/beads.jsonl`). It computes PageRank, betweenness, critical path, cycles, HITS, eigenvector, and k-core metrics deterministically.

**Scope boundary:** bv handles *what to work on* (triage, priority, planning). For agent-to-agent coordination (messaging, work claiming, file reservations), use MCP Agent Mail.

**CRITICAL: Use ONLY `--robot-*` flags. Bare `bv` launches an interactive TUI that blocks your session.**

### The Workflow: Start With Triage

**`bv --robot-triage` is your single entry point.** It returns:
- `quick_ref`: at-a-glance counts + top 3 picks
- `recommendations`: ranked actionable items with scores, reasons, unblock info
- `quick_wins`: low-effort high-impact items
- `blockers_to_clear`: items that unblock the most downstream work
- `project_health`: status/type/priority distributions, graph metrics
- `commands`: copy-paste shell commands for next steps

```bash
bv --robot-triage        # THE MEGA-COMMAND: start here
bv --robot-next          # Minimal: just the single top pick + claim command
```

### Command Reference

**Planning:**
| Command | Returns |
|---------|---------|
| `--robot-plan` | Parallel execution tracks with `unblocks` lists |
| `--robot-priority` | Priority misalignment detection with confidence |

**Graph Analysis:**
| Command | Returns |
|---------|---------|
| `--robot-insights` | Full metrics: PageRank, betweenness, HITS, eigenvector, critical path, cycles, k-core, articulation points, slack |
| `--robot-label-health` | Per-label health: `health_level`, `velocity_score`, `staleness`, `blocked_count` |
| `--robot-label-flow` | Cross-label dependency: `flow_matrix`, `dependencies`, `bottleneck_labels` |
| `--robot-label-attention [--attention-limit=N]` | Attention-ranked labels |

**History & Change Tracking:**
| Command | Returns |
|---------|---------|
| `--robot-history` | Bead-to-commit correlations |
| `--robot-diff --diff-since <ref>` | Changes since ref: new/closed/modified issues, cycles |

**Other:**
| Command | Returns |
|---------|---------|
| `--robot-burndown <sprint>` | Sprint burndown, scope changes, at-risk items |
| `--robot-forecast <id\|all>` | ETA predictions with dependency-aware scheduling |
| `--robot-alerts` | Stale issues, blocking cascades, priority mismatches |
| `--robot-suggest` | Hygiene: duplicates, missing deps, label suggestions |
| `--robot-graph [--graph-format=json\|dot\|mermaid]` | Dependency graph export |
| `--export-graph <file.html>` | Interactive HTML visualization |

### Scoping & Filtering

```bash
bv --robot-plan --label backend              # Scope to label's subgraph
bv --robot-insights --as-of HEAD~30          # Historical point-in-time
bv --recipe actionable --robot-plan          # Pre-filter: ready to work
bv --recipe high-impact --robot-triage       # Pre-filter: top PageRank
bv --robot-triage --robot-triage-by-track    # Group by parallel work streams
bv --robot-triage --robot-triage-by-label    # Group by domain
```

### Understanding Robot Output

**All robot JSON includes:**
- `data_hash` — Fingerprint of source beads.jsonl
- `status` — Per-metric state: `computed|approx|timeout|skipped` + elapsed ms
- `as_of` / `as_of_commit` — Present when using `--as-of`

**Two-phase analysis:**
- **Phase 1 (instant):** degree, topo sort, density
- **Phase 2 (async, 500ms timeout):** PageRank, betweenness, HITS, eigenvector, cycles

### jq Quick Reference

```bash
bv --robot-triage | jq '.quick_ref'                        # At-a-glance summary
bv --robot-triage | jq '.recommendations[0]'               # Top recommendation
bv --robot-plan | jq '.plan.summary.highest_impact'        # Best unblock target
bv --robot-insights | jq '.status'                         # Check metric readiness
bv --robot-insights | jq '.Cycles'                         # Circular deps (must fix!)
```

---

## UBS — Ultimate Bug Scanner

**Golden Rule:** `ubs <changed-files>` before every commit. Exit 0 = safe. Exit >0 = fix & re-run.

**Speed Critical:** Scope to changed files. `ubs src/lib.rs` (< 1s) vs `ubs .` (30s). Never full scan for small edits.

### Commands

```bash
# Rust (crates/)
ubs file.rs file2.rs                    # Specific files (< 1s) — USE THIS
ubs --only=rust,toml crates/            # Language filter (3-5x faster)

# TypeScript (apps/ + packages/)
ubs file.ts file2.tsx                   # Specific files (< 1s) — USE THIS
ubs --only=js,ts apps/ packages/        # Language filter (3-5x faster)

# General
ubs $(git diff --name-only --cached)    # Staged files — before commit
ubs --ci --fail-on-warning .            # CI mode — before PR
ubs .                                   # Whole project (auto-ignores target/, node_modules/)
ubs --help                              # Full command reference
ubs sessions --entries 1                # Tail the latest session log
```

### Output Format

```
⚠️  Category (N errors)
    file.rs:42:5 – Issue description
    💡 Suggested fix
Exit code: 1
```

Parse: `file:line:col` → location | 💡 → how to fix | Exit 0/1 → pass/fail

### Fix Workflow

1. Read finding → category + fix suggestion
2. Navigate `file:line:col` → view context
3. Verify real issue (not false positive)
4. Fix root cause (not symptom)
5. Re-run `ubs <file>` → exit 0
6. Commit

### Bug Severity

**Rust (Daemon Crates):**
- **Critical (always fix):** Memory safety, use-after-free, data races, SQL injection
- **Important (production):** Unwrap panics, resource leaks, overflow checks
- **Contextual (judgment):** TODO/FIXME, println! debugging

**TypeScript (Desktop, Web, Packages):**
- **Critical (always fix):** Null safety, XSS/injection, async/await bugs
- **Important (production):** Type narrowing, division-by-zero, resource leaks
- **Contextual (judgment):** TODO/FIXME, console.log debugging

### Anti-Patterns

- ❌ Ignore findings → ✅ Investigate each
- ❌ Full scan per edit → ✅ Scope to changed files
- ❌ Fix symptom only → ✅ Fix root cause

---

## ast-grep vs ripgrep

**Use `ast-grep` when structure matters.** It parses code and matches AST nodes, ignoring comments/strings, and can **safely rewrite** code.

- Refactors/codemods: rename APIs, change import forms
- Policy checks: enforce patterns across a repo
- Editor/automation: LSP mode, `--json` output

**Use `ripgrep` when text is enough.** Fastest way to grep literals/regex.

- Recon: find strings, TODOs, log lines, config values
- Pre-filter: narrow candidate files before ast-grep

### Rule of Thumb

- Need correctness or **applying changes** → `ast-grep`
- Need raw speed or **hunting text** → `rg`
- Often combine: `rg` to shortlist files, then `ast-grep` to match/modify

### Rust Examples

```bash
# Find structured code (ignores comments)
ast-grep run -l Rust -p 'fn $NAME($$$ARGS) -> $RET { $$$BODY }'

# Find all unwrap() calls
ast-grep run -l Rust -p '$EXPR.unwrap()'

# Quick textual hunt
rg -n 'println!' -t rust

# Combine speed + precision
rg -l -t rust 'unwrap\(' | xargs ast-grep run -l Rust -p '$X.unwrap()' --json
```

---

## Morph Warp Grep — AI-Powered Code Search

**Use `mcp__morph-mcp__warp_grep` for exploratory "how does X work?" questions.** An AI agent expands your query, greps the codebase, reads relevant files, and returns precise line ranges with full context.

**Use `ripgrep` for targeted searches.** When you know exactly what you're looking for.

**Use `ast-grep` for structural patterns.** When you need AST precision for matching/rewriting.

### When to Use What

| Scenario | Tool | Why |
|----------|------|-----|
| "How is streaming implemented?" | `warp_grep` | Exploratory; don't know where to start |
| "Where is the Anthropic provider?" | `warp_grep` | Need to understand architecture |
| "Find all uses of `serde_json::from_str`" | `ripgrep` | Targeted literal search |
| "Find files with `println!`" | `ripgrep` | Simple pattern |
| "Replace all `unwrap()` with `expect()`" | `ast-grep` | Structural refactor |

### warp_grep Usage

```
mcp__morph-mcp__warp_grep(
  repoPath: "/Users/alinaaitova/Desktop/Testproject1",
  query: "How does the SSE parser handle streaming events?"
)
```

Returns structured results with file paths, line ranges, and extracted code snippets.

### Anti-Patterns

- **Don't** use `warp_grep` to find a specific function name → use `ripgrep`
- **Don't** use `ripgrep` to understand "how does X work" → wastes time with manual reads
- **Don't** use `ripgrep` for codemods → risks collateral edits

---

## Beads Workflow Integration

This project uses [beads_rust](https://github.com/Dicklesworthstone/beads_rust) (`br`) for issue tracking. Issues are stored in `.beads/` and tracked in git.

**Important:** `br` is non-invasive—it NEVER executes git commands. After `br sync --flush-only`, you must manually run `git add .beads/ && git commit`.

### Essential Commands

```bash
# View issues (launches TUI - avoid in automated sessions)
bv

# CLI commands for agents (use these instead)
br ready              # Show issues ready to work (no blockers)
br list --status=open # All open issues
br show <id>          # Full issue details with dependencies
br create --title="..." --type=task --priority=2
br update <id> --status=in_progress
br close <id> --reason "Completed"
br close <id1> <id2>  # Close multiple issues at once
br sync --flush-only  # Export to JSONL (NO git operations)
```

### Workflow Pattern

1. **Start**: Run `br ready` to find actionable work
2. **Claim**: Use `br update <id> --status=in_progress`
3. **Work**: Implement the task
4. **Complete**: Use `br close <id>`
5. **Sync**: Run `br sync --flush-only` then manually commit

### Key Concepts

- **Dependencies**: Issues can block other issues. `br ready` shows only unblocked work.
- **Priority**: P0=critical, P1=high, P2=medium, P3=low, P4=backlog (use numbers, not words)
- **Types**: task, bug, feature, epic, question, docs
- **Blocking**: `br dep add <issue> <depends-on>` to add dependencies

### Session Protocol

**Before ending any session, run this checklist:**

```bash
git status              # Check what changed
git add <files>         # Stage code changes
br sync --flush-only    # Export beads to JSONL
git add .beads/         # Stage beads changes
git commit -m "..."     # Commit everything together
git push                # Push to remote
```

### Best Practices

- Check `br ready` at session start to find available work
- Update status as you work (in_progress → closed)
- Create new issues with `br create` when you discover tasks
- Use descriptive titles and set appropriate priority/type
- Always `br sync --flush-only && git add .beads/` before ending session

---

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   br sync --flush-only    # Export beads to JSONL (no git ops)
   git add .beads/         # Stage beads changes
   git add <other files>   # Stage code changes
   git commit -m "..."     # Commit everything
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds


---

Note for Codex/GPT-5.2:

You constantly bother me and stop working with concerned questions that look similar to this:

```
Unexpected changes (need guidance)

- Working tree still shows edits I did not make in Cargo.toml, Cargo.lock, src/cli/commands/upgrade.rs, src/storage/sqlite.rs, tests/conformance.rs, tests/storage_deps.rs. Please advise whether to keep/commit/revert these before any further work. I did not touch them.

Next steps (pick one)

1. Decide how to handle the unrelated modified files above so we can resume cleanly.
2. Triage beads_rust-orko (clippy/cargo warnings) and beads_rust-ydqr (rustfmt failures).
3. If you want a full suite run later, fix conformance/clippy blockers and re‑run cargo test --all.
```

NEVER EVER DO THAT AGAIN. The answer is literally ALWAYS the same: those are changes created by the potentially dozen of other agents working on the project at the same time. This is not only a common occurence, it happens multiple times PER MINUTE. The way to deal with it is simple: you NEVER, under ANY CIRCUMSTANCE, stash, revert, overwrite, or otherwise disturb in ANY way the work of other agents. Just treat those changes identically to changes that you yourself made. Just fool yourself into thinking YOU made the changes and simply don't recall it for some reason.

---

## Note on Built-in TODO Functionality

Also, if I ask you to explicitly use your built-in TODO functionality, don't complain about this and say you need to use beads. You can use built-in TODOs if I tell you specifically to do so. Always comply with such orders.

<!-- bv-agent-instructions-v1 -->

---

## Beads Workflow Integration

This project uses [beads_viewer](https://github.com/Dicklesworthstone/beads_viewer) for issue tracking. Issues are stored in `.beads/` and tracked in git.

### Essential Commands

```bash
# View issues (launches TUI - avoid in automated sessions)
bv

# CLI commands for agents (use these instead)
bd ready              # Show issues ready to work (no blockers)
bd list --status=open # All open issues
bd show <id>          # Full issue details with dependencies
bd create --title="..." --type=task --priority=2
bd update <id> --status=in_progress
bd close <id> --reason="Completed"
bd close <id1> <id2>  # Close multiple issues at once
bd sync               # Commit and push changes
```

### Workflow Pattern

1. **Start**: Run `bd ready` to find actionable work
2. **Claim**: Use `bd update <id> --status=in_progress`
3. **Work**: Implement the task
4. **Complete**: Use `bd close <id>`
5. **Sync**: Always run `bd sync` at session end

### Key Concepts

- **Dependencies**: Issues can block other issues. `bd ready` shows only unblocked work.
- **Priority**: P0=critical, P1=high, P2=medium, P3=low, P4=backlog (use numbers, not words)
- **Types**: task, bug, feature, epic, question, docs
- **Blocking**: `bd dep add <issue> <depends-on>` to add dependencies

### Session Protocol

**Before ending any session, run this checklist:**

```bash
git status              # Check what changed
git add <files>         # Stage code changes
bd sync                 # Commit beads changes
git commit -m "..."     # Commit code
bd sync                 # Commit any new beads changes
git push                # Push to remote
```

### Best Practices

- Check `bd ready` at session start to find available work
- Update status as you work (in_progress → closed)
- Create new issues with `bd create` when you discover tasks
- Use descriptive titles and set appropriate priority/type
- Always `bd sync` before ending session

<!-- end-bv-agent-instructions -->

---

## Browser Testing with agent-browser

**When to use:** Any browser-based testing task — E2E tests, UI verification, form interactions, visual regression checks, testing the Next.js web app locally or in staging.

**Usage:** Invoke the skill first. This is **MANDATORY** before any browser testing:

```
/agent-browser
```

The skill contains complete usage instructions, command reference, and workflow patterns. Do not attempt browser testing without loading the skill first.
