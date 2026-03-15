# Harmonization Handoff — Run 001: User Feedback Integration into Research Document

## Mission Statement

You are a chief systems integrator updating a master architecture research document for a UK public procurement intelligence platform. Your job is to harmonize the owner's architectural feedback into the existing research document so that, after your edits, the research document tells **one coherent story** that reflects the owner's decisions.

**You are editing documentation only. Do not change code.**

**Primary goal:** The research document must fully absorb the feedback below. After harmonization, an engineer reading only the research document gets one complete, consistent architecture with no contradictions and no stale content.

**Secondary goal:** The document must remain implementation-safe — no ambiguity that could produce incompatible implementations from different sections.

**Deliverable:** An updated version of the research document.

---

## Implementation Context

This is a **greenfield project** — there is no code yet. The research document is the foundational architecture brief. All changes are to documentation only.

---

## Mandatory Pre-Read Requirement (CRITICAL)

**Before you touch any documentation, you MUST perform a deep-reading and comparison pass.**

This is not optional. Do not start editing after reading only the feedback summary below.

### Required Steps:

1. **Read the full research document** at `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` (2875 lines). Read every section, every ADR, every data model, every diagram.
2. **Read this entire handoff document** to understand all feedback points and their cascading effects.
3. **Build a private comparison matrix** before editing:
   - What the research document currently says for each affected area
   - What the feedback requires it to say instead
   - Which sections create contradictions if only partially updated
4. **Only THEN begin harmonizing the document.**

You must not skip or collapse this research pass. Cross-section contradictions are only visible after reading everything.

---

## Source Material: Owner Feedback (8 Architectural Changes)

The owner provided feedback across 6 areas. These translate into 8 specific architectural changes that must be integrated:

### Feedback Area 1: Primary Data Strategy — SKILL-FIRST AGENT SCRAPING

**Owner's direction:**
> Core data strategy will be agent-based scraping triggered by our desktop dashboard and executed by our Rust daemon sidecar. The agent will run on my local machine and use an agent browser to agentically scrape information from websites, based on our skill system and prompt system.
>
> We want to constrain ALL scraping and data retrieval to our skill-based agent system running on our local machine. This will simplify the architecture as all data retrieval will be based on browser-enabled agents running on our local machine powered by skills (way easier to maintain, upgrade and change) instead of having to maintain APIs and scrapers for each data source (website).
>
> You can simplify and not worry about any of the API architecture you designed as we are going skill first.

**What this means for the document:**
- The entire API-first connector approach for procurement data (Section 4.1) must be replaced with skill-based agent scraping
- The tiered supplier API approach (Section 4.3) must be replaced with skill-based agent scraping
- ADR-001 ("Use Official Procurement APIs Before Scraping") must be **REPLACED** with a new ADR reflecting skill-first agent scraping
- ADR-005 ("Skills Are Workflow Assets, Not the Workflow Engine") must be **REVISED** — skills are still not the workflow engine (orchestration stays in the daemon/database), but skills are NOW the primary data retrieval mechanism, not just behavioral assets
- The connector package architecture (`packages/connectors/`) in the file reference map must be rethought — data retrieval skills replace API client packages
- Section 4.10 (Skills and Browser Automation) must be significantly expanded as it is now the core data strategy
- The system context diagram (Section 5.1) and component design (Section 5.2) must reflect agent-browser scraping as the primary data path
- The ingestion data flow (Section 5.6) and intelligence pipeline (Section 5.7) must reflect agent-based execution
- The `packages/connectors/` directory in Section 3.4 should be replaced with skills-based structure

**Important nuance:** The owner says "you can simplify and not worry about any of the API architecture." However, for the **cost-estimation engine** (Feedback Area 4), APIs ARE explicitly welcome. So the document must clearly distinguish:
- **Procurement data + supplier pricing** → skill-based agent scraping (no API connectors)
- **Cost estimation data** (customs, freight, FX) → APIs are acceptable and encouraged

### Feedback Area 2: Agent Orchestration — PTY-FIRST FROM DAY 1

**Owner's direction:**
> We do indeed want to trigger codex/claude sessions based on cron jobs and manually via desktop from our dashboard, powered by our Rust daemon. This being triggered from headless PTY terminals is a design constraint from day 1, as it is critical for robust daily operations for our pipeline to function without interruption.

**What this means for the document:**
- ADR-004 ("Do Not Make PTY Emulation the MVP Orchestrator") must be **REVERSED**. PTY-first is now the selected approach from day 1.
- Section 4.9 (Local Agent Orchestration Runtime) must select PTY-based execution as the primary approach, not defer it
- The daemon architecture (Section 5.2, Section 8.2 daemon files) must reflect PTY-first design
- The complexity analysis (Section 6.1) should acknowledge PTY complexity but frame it as a necessary investment for robustness

**Why this matters:** The owner explicitly states the entire data ingestion, intelligence, and bidding pipeline depends on agent orchestration working every time. This is the most critical infrastructure piece.

### Feedback Area 3: Comprehensive Tracing & Agent Health Monitoring

**Owner's direction:**
> You should probably consider adding comprehensive tracing logs and detection when the agent stops working for any reason so we can easily debug the agent workflow if things go wrong — i.e. we need to re-login to claude/codex or there is an API error inside claude/codex or for whatever other reason. These logs need to include the full terminal output from claude so we can trace exactly what the agent is doing, what it is thinking and trace exactly where the error came from.

**What this means for the document:**
- Section 6.6 (Operational Observability) must be significantly expanded with:
  - Full terminal output capture from agent PTY sessions
  - Agent health detection (stopped working, needs re-login, API errors within agent)
  - Structured tracing of agent thought process and actions
  - Error source tracing (which step, which skill, which agent response caused failure)
- Section 5.11 (Error Handling Strategy) needs new agent-specific error types:
  - `AGENT_SESSION_EXPIRED` — needs re-login
  - `AGENT_INTERNAL_ERROR` — API error inside claude/codex
  - `AGENT_STALLED` — agent stopped producing output
  - `AGENT_SKILL_FAILED` — skill execution failed within agent
- Section 5.2 (Daemon component) and Section 8.2 (daemon files) should reflect log capture architecture

### Feedback Area 4: Cost-Estimation Engine — DEEPER RESEARCH + APIs WELCOME

**Owner's direction:**
> Yes, we want to build an internal cost estimation engine. But I want you to do far more research into each of the data source we can use for every step of this. Use exa and the web.
>
> If we are going to use any APIs for data sources we will most likely use it here, as this is where APIs can actually be very helpful and reliable.

**What this means for the document:**

The cost-estimation engine sections (4.4, 4.5, 4.6) must be expanded with additional data sources discovered through research. APIs are explicitly acceptable here — this is the ONE area where API-based data retrieval is preferred over agent scraping.

**NEW DATA SOURCES TO ADD (from Exa research):**

#### HS Code / Commodity Classification (expand Section 4.5):

1. **HMRC FPO (Fast Parcel Operator) API** — CRITICAL ADDITION
   - Official HMRC API that accepts English-language product descriptions and returns candidate commodity codes
   - Free to use for registered organizations
   - Uses ML model trained on tariff data (source: `github.com/trade-tariff/trade-tariff-lambdas-fpo-search`)
   - Originally for Windsor Framework GB→NI parcels but the classification logic is broadly applicable
   - Endpoint: via HMRC Trade Tariff Developer Portal (`hub.trade-tariff.service.gov.uk`)
   - Reference: `api.trade-tariff.service.gov.uk/fpo.html`
   - **Recommendation:** STRONGLY RECOMMENDED as a first-pass automated HS code classifier before human review

2. **HS Ping** — Commercial HS/HTS code lookup API
   - Official data sources, versioned responses
   - Reference: `hsping.com`
   - Useful as a validation/cross-check source

3. **Taric Support API** — All-in-one customs compliance API for UK and EU
   - HS code browsing, validation, description retrieval, suggestions for incomplete codes
   - Real-time tariff data
   - Reference: `taricsupport.com/api/`
   - Useful for code validation and completeness checking

4. **Dutify** — Landed cost calculator with AI-powered product classification
   - API available for duty and tax calculation
   - Integrates classification + duty + tax in one call
   - Reference: `docs.dutify.com/docs/landed-cost-calculator`
   - Useful as a secondary/comparison signal

5. **AI-based HS Classification approaches** — The academic paper "ATLAS: Benchmarking and Adapting LLMs for Global Trade via Harmonized Tariff Code Classification" (2025) demonstrates fine-tuned LLMs achieving strong HS code classification. Consider this as a future enhancement path alongside HMRC FPO.

#### Freight & Logistics (expand Section 4.4):

6. **SeaRates Logistics Explorer API** — SIGNIFICANT ADDITION
   - GraphQL API at `rates.searates.com/graphql`
   - Covers FCL, LCL, air freight, land/road transport across 190+ countries
   - Automatic mode detection based on weight and locations (`Get Auto Rates` endpoint)
   - Additional capabilities: container tracking, ship schedules, load calculator, carbon emissions, distance & time, freight index
   - Active development (Feb 2026 release notes show ongoing updates)
   - Reference: `docs.searates.com/reference/logistics/get-rates`
   - **Recommendation:** RECOMMENDED as additional freight benchmark alongside Freightos — provides complementary coverage especially for LCL and multimodal routes

#### FX Conversion (expand Section 4.6):

7. **HMRC Exchange Rate Service** — CRITICAL ADDITION for customs valuation
   - Official HMRC monthly exchange rates used for customs valuation calculations
   - Now integrated directly into the Online Trade Tariff service
   - Published penultimate Thursday of every month, applies to following calendar month
   - Available in CSV and XML formats
   - Reference: `trade-tariff.service.gov.uk/exchange_rates/monthly`
   - HMRC Developer Hub also provides XML API: `developer.service.hmrc.gov.uk/api-documentation/docs/api/xml/Exchange%20rates%20from%20HMRC`
   - **Recommendation:** MANDATORY for customs valuation calculations — this is the legally authoritative rate for UK customs duty computation. Should be used ALONGSIDE Xe (which is better for real-time/commercial FX). The customs engine should use HMRC rates for duty calculation and Xe for commercial margin estimation.

8. **CurrencyBeacon** — Commercial FX API
   - 200+ currencies, 6000+ cryptocurrencies
   - Real-time and historical data
   - 99.9% uptime guarantee, sub-100ms response times
   - Institutional-grade data from ECB, Bank of Canada, etc.
   - Reference: `currencybeacon.com`
   - Useful as a backup/comparison FX source

9. **OANDA Historical Currency Converter** — Historical FX
   - 31 years of history, 200+ currencies, 38,000 FX pairs
   - Strong for auditing and historical analysis
   - Reference: `fxds-hcc.oanda.com`
   - Useful for historical margin recalculation and audit trails

#### Landed Cost Calculator Services (new subsection for comparison):

10. **Zonos** — Cross-border duty/tax calculation API
    - $299+/month, instant setup
    - Reference: `zonos.com`
    - Useful as a comparison/validation signal against internal engine

11. **Avalara** — Enterprise compliance solution
    - $16.5K+/year, 2-4 week setup
    - Comprehensive but expensive
    - Reference: `avalara.com`
    - Consider only if volume justifies cost

**Section structure recommendation:** Add each new source as a properly scored option within the relevant subsection (4.4, 4.5, 4.6), following the existing scoring pattern. Add a new subsection for "Third-party Landed Cost Services" comparing Dutify, Zonos, and Avalara as secondary validation signals.

### Feedback Area 5: Automation Policy — LOWER RISK, ERROR FLAGGING

**Owner's direction:**
> We shouldn't worry too much about partial failures because we're not actually submitting real money bids. We're simply automating the request pipeline. The actual negotiations and the fulfillment will be negotiated manually by the humans using this pipeline so you don't need to worry about the risk there. The cost of failure isn't super high, which is why we can automate this step.
>
> However if there are failures doing the workflows it would be great to have a system to flag where that failed, flag the error, and flag part of the pipeline that it failed so we can debug it in the future. This will also be helpful if we had a dashboard and maybe an error manifest MD file or something.

**What this means for the document:**
- ADR-007 should be REVISED: Human review is still valuable for commercial decisions, but the risk framing around bid automation should be significantly relaxed
- Section 5.9 (Bidding Flow) should clarify that "bids" are request pipeline automation, not real-money submissions
- Section 6.2 (Risk Assessment) should downgrade bid automation risks
- ADD: Error manifest system — a structured error log (potentially as markdown files) that flags:
  - Which workflow step failed
  - The error message
  - The pipeline stage where failure occurred
  - Enough context to debug
- ADD: Dashboard integration for error visibility
- The automation policy throughout should reflect that human negotiation handles the actual commercial decisions

### Feedback Area 6: System of Record — LOCAL SQLITE + SUPABASE SYNC

**Owner's direction:**
> We're instead going to use a local SQLite database, which probably should be bundled together with our Rust daemon architecture, and leverage that as the source of truth when we're running data from our machine.
>
> What we will then want to do is have a method to sync to a database on an automatic database — Postgres SQL by Supabase — and then pull from that database as well. We will have that kind of push-pull method.
>
> Why: multiple users using this system. Risk that someone corrupts the shared Postgres. Having a local version of a non-corrupted one could be helpful. We still get the advantage of Postgres having shared state where we can see all the data.

**What this means for the document:**
- ADR-002 must be **FUNDAMENTALLY REVISED**: SQLite bundled with Rust daemon is the local source of truth. Supabase/Postgres is the shared sync target for multi-user visibility, NOT the primary source of truth.
- Section 4.7 (Shared Database) must describe the dual-database architecture:
  - **Local SQLite** (bundled with Rust daemon): source of truth during operation, stores all pipeline data locally
  - **Supabase/Postgres**: shared view, multi-user access, sync target
  - **Push-pull sync mechanism**: local→remote and remote→local sync
  - **Corruption resilience**: local copy protects against shared DB corruption
- Section 5.1 (System Context Diagram) must show SQLite as local store with sync arrows to Supabase
- Section 5.4 (Data Model) must consider SQLite compatibility (no arrays natively, JSON support via json1 extension, etc.)
- Section 5.6 (Ingestion Data Flow) becomes local-first: daemon writes to local SQLite, syncs to Supabase
- Section 5.8 (Dashboard Data Flow): desktop reads from local SQLite, web reads from Supabase
- Section 9.1 (Dependencies): add `rusqlite` or `sqlx` with SQLite feature for Rust
- The job/lease system from Section 5.4 needs adaptation — local daemon manages its own jobs in SQLite, syncs status to Supabase for visibility
- The `Hosted Scheduler` component concept (Section 5.2) changes — scheduling is driven by the Rust daemon locally via cron, not by a hosted scheduler

### Feedback Area 7: Frontend Strategy — DESKTOP = CONTROL PANEL, WEB = VIEWING PORTAL

**Owner's direction:**
> The desktop app is obviously a control panel but the website is not the control panel; it's just the viewing portal where we can view the data.

**What this means for the document:**
- ADR-008 must be updated to clearly state the role split:
  - **Desktop (Tauri)**: Full control panel — trigger workflows, manage agents, configure skills, make bid/reject/watch decisions, view local data
  - **Web**: Read-only viewing portal — browse opportunities, view manifests, view pipeline status, view errors. No write operations.
- Section 5.8 (Dashboard Data Flow) must distinguish the two flows:
  - Desktop: reads from local SQLite, writes decisions, triggers agent workflows
  - Web: reads from Supabase/Postgres (synced), no write operations
- User stories (Section 2.3) should distinguish desktop operator stories from web viewer stories
- Open Question #2 (Section 10) is now answered: separate shells with clearly different capabilities
- The web shell file list (Section 8.2) should be simplified — no reject/bid/rerun controls on web

### Feedback Area 8: Testing Strategy — REAL E2E, NO MOCKS

**Owner's direction:**
> We will need a comprehensive test suite to ensure that all the functionality we build works including agent runtime. Apart from unit tests we will also need full end-to-end agent tests where we actually test sending sample kickoff prompts and skills to the agent PTY instance and verifying that we are seeing the agent working and outputting and proceeding with its tasks. Therefore avoid creating mocks or stub test functionality, and prioritise real end-to-end tests, as those determine success.

**What this means for the document:**
- Section 6.5 (Testing Strategy) must be substantially revised:
  - **Unit tests**: Keep for deterministic cost/qualification logic
  - **E2E agent tests**: NEW priority — real prompts sent to real agent PTY instances, verifying agent output and task progression
  - **Integration tests**: Real database, real agent processes — no mocks or stubs
  - **Contract tests with "recorded fixtures"**: De-emphasize or remove — the owner explicitly says no mocks/stubs
  - **Priority order**: Real E2E tests > unit tests > everything else
- Section 8.3 (Test Files) should add agent E2E test files
- The testing approach should emphasize that agent E2E tests are the ultimate success criterion

---

## What to Read (Exact Paths and Reading Priorities)

### Primary Source
- **This handoff document** — contains all feedback and integration guidance

### Target Document (the document you will edit)
- `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` (2875 lines)
  - Read EVERY section. Focus especially on:
    - Section 4.1 (Procurement Ingestion) — will be rewritten for skill-first
    - Section 4.3 (Supplier Price Intelligence) — will be rewritten for skill-first
    - Section 4.4-4.6 (Shipping, Customs, FX) — will be expanded with new sources
    - Section 4.7 (Database) — will be fundamentally changed to SQLite + Supabase sync
    - Section 4.9 (Agent Orchestration) — will be revised for PTY-first
    - Section 4.10 (Skills) — will be significantly expanded
    - Section 5.1-5.11 (Architecture) — multiple diagram and flow updates
    - ADR-001, 002, 004, 005, 007, 008 — all need revision
    - Section 6.5 (Testing) — rewrite for real E2E
    - Section 6.6 (Observability) — expand for comprehensive tracing
    - Section 8.2-8.3 (File Reference) — update file lists
    - Section 10 (Open Questions) — several are now answered

---

## Core Harmonization Goals (Numbered, Specific)

1. **SKILL-FIRST DATA RETRIEVAL replaces API-FIRST CONNECTORS** — All sections must consistently state that procurement data AND supplier pricing are retrieved via browser-enabled agent skills running on the local machine. The only exception is cost-estimation data (customs, freight, FX) where APIs are preferred. Currently, Sections 4.1, 4.3, 5.1, 5.2, 5.6, 5.7, ADR-001, and ADR-005 all describe an API-first connector approach that must be replaced.

2. **PTY-FIRST AGENT EXECUTION is the day-1 design** — All agent orchestration sections must consistently state that headless PTY terminals are the primary execution model from day 1, not a deferred capability. Currently, Section 4.9 and ADR-004 explicitly recommend against PTY-first. This must be reversed.

3. **LOCAL SQLITE + SUPABASE SYNC is the database architecture** — All sections must consistently describe SQLite (bundled with Rust daemon) as the local source of truth and Supabase/Postgres as the shared sync target. Currently, Sections 4.7, 5.1, 5.2, 5.4, 5.6, 5.8, ADR-002 all describe Supabase as the sole source of truth. This must be changed.

4. **DESKTOP = CONTROL PANEL, WEB = VIEWING PORTAL** — All frontend and dashboard sections must consistently distinguish these roles. Desktop has full read/write/trigger capabilities; web is read-only. Currently, the document implies roughly equal desktop/web capabilities.

5. **COST-ESTIMATION ENGINE is expanded with deeper data source research** — Sections 4.4, 4.5, 4.6 must include the new data sources (HMRC FPO API, SeaRates, HMRC Exchange Rate Service, CurrencyBeacon, OANDA, HS Ping, Taric Support, Dutify, Zonos, Avalara) with proper scoring and recommendations. This is where APIs are explicitly preferred.

6. **COMPREHENSIVE AGENT TRACING** — Section 6.6 and the daemon architecture must describe full terminal output capture, agent health detection, error source tracing. This is not a nice-to-have — it is critical infrastructure.

7. **REAL E2E TESTS, NO MOCKS** — Section 6.5 must prioritize real end-to-end agent tests over mocked/stubbed tests. The testing strategy must reflect that agent E2E tests are the success criterion.

8. **RELAXED AUTOMATION RISK + ERROR MANIFEST** — The automation policy and bid workflow sections must reflect that this automates the request pipeline (not real-money bids) and add an error flagging/manifest system for debugging.

---

## Expected Document Changes

### HIGHEST PRIORITY — Sections Requiring Fundamental Rewrite/Major Changes:

**Section 4.1 (Procurement Notice Ingestion Sources):**
- Reframe: Procurement ingestion now happens via agent skills with browser automation, not API connectors
- The official APIs (Find a Tender, Contracts Finder, PCS) are still the DATA SOURCES being scraped — they just aren't accessed via TypeScript API clients anymore; the agent browser navigates to them
- Keep the source quality analysis (data structure scores etc.) as it's still useful for understanding which sources to target
- Change the "Procurement Ingestion Recommendation" to reflect skill-first approach
- The "connector platform" framing should become a "skill-based scraping platform"

**Section 4.3 (Supplier Price Intelligence Sources):**
- Same reframe: supplier pricing is now collected via agent skills with browser automation
- The source analysis (eBay, Amazon, Alibaba) is still useful for understanding where to look
- Remove the API-integration-specific details and replace with skill-based collection approach
- The tiered model becomes: skill-based scraping of UK sources, skill-based scraping of China sources, with confidence scoring

**Section 4.7 (Shared Database / Backend Platform):**
- Fundamental rewrite: dual-database architecture
- SQLite as local source of truth bundled with Rust daemon
- Supabase/Postgres as shared sync target
- Push-pull sync mechanism
- Corruption resilience rationale
- Keep the Supabase evaluation but reframe its role
- Remove Convex and SpacetimeDB sections or significantly condense (decision already made)

**Section 4.9 (Local Agent Orchestration Runtime):**
- Reverse the recommendation: PTY-first is now selected
- Option B becomes the primary approach
- Significantly expand the PTY design considerations
- Include headless PTY design for cron-triggered and dashboard-triggered sessions

**ADR-001:** Replace with "Use Skill-Based Agent Scraping as Primary Data Retrieval" — skills navigate to official sources via browser automation. APIs are reserved for cost-estimation data.

**ADR-002:** Revise to "Use Local SQLite as Source of Truth with Supabase/Postgres Sync" — describe dual-database architecture and push-pull sync rationale.

**ADR-004:** Reverse to "Use PTY-Based Agent Execution from Day 1" — headless PTY terminals are a day-1 design constraint.

**Section 6.5 (Testing Strategy):**
- Substantially revise to prioritize real E2E agent tests
- Remove or de-emphasize "recorded fixtures + schema assertions"
- Add agent E2E test patterns (send real prompts, verify real agent output)
- Keep unit tests for deterministic logic

### SECONDARY PRIORITY — Sections Requiring Significant Updates:

**ADR-005:** Revise to clarify that skills are BOTH workflow assets AND the primary data retrieval mechanism (but still not the workflow engine — orchestration remains in daemon/database).

**ADR-007:** Revise risk framing — automation automates the request pipeline, not real money. Add error manifest system concept.

**ADR-008:** Update to explicitly state desktop = control panel, web = viewing portal.

**Section 4.4 (Shipping and Logistics):**
- Add SeaRates Logistics Explorer API as a new option with scoring
- Update the shipping recommendation to include SeaRates

**Section 4.5 (Customs, Tariff, and Commodity Classification):**
- Add HMRC FPO API as a new option with scoring (STRONGLY RECOMMENDED)
- Add HS Ping, Taric Support API as supporting options
- Add a brief mention of AI-based classification approaches (ATLAS research)
- Add a new subsection for third-party landed cost services (Dutify, Zonos, Avalara)

**Section 4.6 (FX Conversion):**
- Add HMRC Exchange Rate Service as MANDATORY for customs valuation
- Add CurrencyBeacon and OANDA as supporting options
- Clarify the distinction: HMRC rates for customs duty calculation, Xe for commercial margin estimation

**Section 4.10 (Skills and Browser Automation):**
- Significantly expand — this is now the core data retrieval architecture
- Detail the skill types needed for procurement scraping, supplier scraping, bidding
- Describe the skill lifecycle and how skills are maintained/upgraded
- Reference agent-browser patterns

**Section 5.1 (System Context Diagram):**
- Replace "Ingestion Connectors" with "Agent Skills / Browser Automation"
- Show SQLite as local store with sync to Supabase
- Show desktop as control panel, web as viewing portal

**Section 5.2 (High-Level Component Design):**
- Replace "Source Registry" and "Ingestion Runner" with skill-based execution model
- Replace "Hosted Scheduler" with daemon-local cron scheduling
- Add PTY session manager component
- Add log/trace capture component
- Add SQLite local store component
- Add sync engine component

**Section 5.4 (Data Model Design):**
- Consider SQLite compatibility notes (TEXT arrays instead of native arrays, JSON via json1)
- Add error_manifest table
- Add agent_session_logs table

**Section 5.6 (Ingestion Data Flow):**
- Rewrite for skill-based execution: daemon triggers agent skill via PTY → agent scrapes source → data stored locally in SQLite → synced to Supabase

**Section 5.7 (Intelligence Pipeline Data Flow):**
- Supplier enrichment now happens via agent skills, not API calls

**Section 5.8 (Dashboard Data Flow):**
- Split into two flows: desktop (local SQLite, full control) and web (Supabase, read-only)

**Section 5.11 (Error Handling Strategy):**
- Add agent-specific error types
- Add error manifest concept

**Section 6.6 (Operational Observability):**
- Significantly expand with comprehensive agent tracing, full terminal output capture, health detection

### LIGHT TOUCH — Sections Requiring Minor Updates:

**Section 3.4 (Proposed Greenfield Project Shape):**
- Replace `packages/connectors/` with skill-based structure
- Add SQLite storage to daemon crate

**Section 5.3 (Canonical Workflow):**
- Minor terminology updates (skill-based enrichment instead of API-based)

**Section 5.5 (Contract State Model):**
- Unchanged — the state model still works

**Section 6.1 (Complexity Analysis):**
- Update daemon complexity to reflect PTY + SQLite + sync
- Note that skill maintenance replaces connector maintenance

**Section 6.2 (Risk Assessment):**
- Downgrade bid automation risk per relaxed automation policy
- Add skill maintenance/drift risk

**Section 6.3 (Security Considerations):**
- Minor updates for SQLite local storage security

**Section 6.4 (Performance):**
- Add SQLite performance characteristics

**Section 8.2 (Files to Create):**
- Update file lists to reflect new architecture (skills instead of connectors, SQLite, sync engine)

**Section 8.3 (Test Files):**
- Add agent E2E test files

**Section 9.1-9.3 (Dependencies):**
- Add SQLite deps, remove some TS connector deps
- Add PTY-related Rust deps

**Section 10 (Open Questions):**
- Question 2 (desktop/web parity) is answered: desktop = control, web = view-only
- Question 3 (autonomy for bid workflows) is answered: automate fully, it's just request pipeline
- Question 5 (browser automation acceptable) is answered: yes, it's the primary strategy

**Section 11 (Next Steps):**
- Update to reflect new architecture priorities

---

## Integration Rules

### What Must Be True After Harmonization:

- No section describes API-first connectors as the primary data retrieval strategy (except for cost-estimation APIs)
- No section describes Supabase as the sole/primary source of truth
- No section recommends deferring PTY-based execution
- No section implies desktop and web have equal write capabilities
- All ADRs are internally consistent with each other and with the body of the document
- The system context diagram, component design, and all data flows reflect the updated architecture
- Cost-estimation sections are expanded with all new data sources, properly scored
- Testing strategy prioritizes real E2E over mocks
- Observability section includes comprehensive agent tracing

### What Must Stay OUT:

- Implementation task planning / bead sequencing / "first implement X then Y" beyond the existing milestone structure
- Tool-specific dev instructions
- Design iteration workflow notes
- This harmonization handoff document's content copied verbatim — absorb the intent, don't paste the instructions

### What Must Be ADDED Where Existing Docs Are Too Vague:

- Explicit skill-based data retrieval architecture (currently absent)
- PTY session management architecture (currently deferred)
- SQLite + Supabase sync mechanism design (currently absent)
- Error manifest system (currently absent)
- Agent E2E testing patterns (currently absent)
- HMRC FPO API, SeaRates, HMRC Exchange Rates, and other new data sources (currently absent)
- Desktop vs web role distinction (currently implicit)

### Style Rules:

- Preserve the document's existing section structure and numbering
- Integrate changes INTO existing sections — don't dump new content as appendices
- Follow the existing scoring/evaluation pattern for new data source options
- Keep ADR format consistent (Status, Context, Decision, Options, Rationale, Consequences)
- Maintain the document's technical depth and evidence-based tone
- When revising ADRs, clearly mark the new status (e.g., "Status: Revised" or "Status: Superseded")

---

## What a Great Outcome Looks Like

- An engineer reading only the harmonized research document gets ONE coherent architecture: skill-based agent scraping for data retrieval, PTY-first daemon execution, local SQLite with Supabase sync, desktop control panel with web viewing portal, and an API-powered cost-estimation engine.
- The document no longer contains any reference to API-first connectors being the primary data strategy (except for cost estimation).
- ADR-004 no longer says "defer PTY" — it says "PTY from day 1."
- ADR-002 no longer says "Supabase is the source of truth" — it describes the dual-database architecture.
- The cost-estimation sections are significantly richer with HMRC FPO API, SeaRates, HMRC Exchange Rates, and other new sources properly evaluated and scored.
- The testing strategy is clearly E2E-first with no emphasis on mocks or stubs.
- The observability section is comprehensive enough that an engineer knows exactly what to log from agent sessions.
- An engineer NEVER needs to read the original feedback to understand the architecture.
- Edit until this standard is met, not until you've touched the obvious sections.

---

## Deliverable

1. Updated `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
2. Concise change summary: which sections changed, what was resolved, what (if anything) remains open

At minimum, expect to modify sections: 3.4, 4.1, 4.3, 4.4, 4.5, 4.6, 4.7, 4.9, 4.10, 5.1, 5.2, 5.4, 5.6, 5.7, 5.8, 5.11, 6.1, 6.2, 6.5, 6.6, 7 (ADRs 001-008), 8.2, 8.3, 9.1-9.3, 10, 11.

---

## Final Verification Checklist

Before stopping, verify:

- [ ] Every modified section was re-read after editing
- [ ] No section still describes API connectors as the primary data retrieval strategy (except cost estimation)
- [ ] No section still describes Supabase as the sole source of truth
- [ ] No section still recommends deferring PTY execution
- [ ] ADR-001, ADR-002, ADR-004, ADR-005, ADR-007, ADR-008 are all revised and internally consistent
- [ ] All new data sources (HMRC FPO, SeaRates, HMRC Exchange Rates, CurrencyBeacon, OANDA, HS Ping, Taric Support, Dutify, Zonos, Avalara) are integrated with scoring
- [ ] Testing strategy prioritizes real E2E and does not emphasize mocks/stubs
- [ ] Observability section includes full terminal output capture and agent health monitoring
- [ ] System context diagram reflects skill-based scraping, local SQLite, and Supabase sync
- [ ] Desktop vs web role distinction is explicit in all relevant sections
- [ ] Error manifest system is described
- [ ] No implementation sequencing content was accidentally added
- [ ] The document tells one coherent story from end to end

---

## Follow-Up Fresh-Eyes Pass

> **Send this as a follow-up message in the same session after completing the harmonization above.**

Re-read the entire research document with completely fresh eyes. Pretend you have never seen the feedback or this handoff. Then:

1. **Contradiction hunt:** Search every section for any remaining mention of:
   - "API-first" or "connector" as the primary data strategy (should only appear for cost estimation)
   - "Supabase" as "source of truth" (should say "shared sync target" or similar)
   - "defer PTY" or "non-interactive first" (should say PTY from day 1)
   - "recorded fixtures" or "contract tests" emphasized over E2E (should be de-emphasized)

2. **Consistency check:** Verify that the system context diagram, component design, data flows, ADRs, and file reference map all tell the same story.

3. **Absorption check:** Could an engineer implement the full architecture from ONLY this document? Is anything ambiguous or contradictory?

4. **Stale wording check:** Are there any phrases that belong to the pre-feedback architecture that survived editing?

5. Fix any issues found with targeted edits.
