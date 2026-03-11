# Harmonization Optimization — Run 001: Verification & Consistency Audit

## Mission Statement

You are a fresh-eyes architecture auditor. The research document for a UK public procurement intelligence platform was recently updated to incorporate owner feedback across 8 architectural areas. Your job is to verify that the harmonization is flawless — no contradictions between sections, no stale terminology, no silent omissions, and no ambiguity that could produce incompatible implementations.

**You are auditing and fixing documentation only. Do not change code.**

**Standard:** "Boringly consistent" — every section tells the same story about every concept.

---

## Mandatory Deep Re-Read (Before Any Audit)

1. **Read the harmonization handoff** at `docs/harmonymaxer/active/harmonization-handoff.md` — this describes what was supposed to change and why. It contains the complete list of feedback areas and integration requirements.

2. **Read the ENTIRE research document** at `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` — every section, every ADR, every diagram, every data model, every file reference. Do NOT skim.

3. **Build a fresh mental model** of the architecture as described in the current document. Note any remaining divergences or ambiguities.

4. **Only THEN begin the audit below.**

---

## Audit Dimension 1: Contradiction Hunt

For each of the 8 core concepts below, search EVERY section of the research document for statements about this concept and verify all statements are mutually consistent.

### Concept 1: Data Retrieval Strategy
**Expected state:** Skill-based agent scraping via browser automation is the primary data retrieval method for procurement data AND supplier pricing. APIs are used ONLY for cost-estimation data (customs, freight, FX).

**Hunt for:**
- Any remaining references to "API-first connectors" as the primary data strategy
- Any section still describing `packages/connectors/` TypeScript API clients as the main data path
- Any section recommending API integration for procurement or supplier data (vs. cost estimation)
- ADR-001 — does it reflect skill-first, or still say "Use Official Procurement APIs Before Scraping"?
- The connector package in Section 3.4 / Section 8.2 — has it been replaced with skills architecture?
- Section 5.6 and 5.7 — do ingestion and enrichment flows describe agent execution or API calls?

**Specific terms to flag if they appear in the wrong context:**
- "connector" (acceptable only when referring to cost-estimation APIs or as historical context)
- "API-first" (acceptable only for cost-estimation)
- "client.ts" in packages/connectors/ (should be replaced or reframed)

### Concept 2: Agent Orchestration Model
**Expected state:** Headless PTY terminals are the day-1 execution model for codex/claude agents. NOT deferred.

**Hunt for:**
- Any remaining text recommending non-interactive child process as the primary/selected approach
- ADR-004 — does it say "PTY from day 1" or still say "defer PTY"?
- Section 4.9 — is Option A (non-interactive) still marked as "SELECTED"?
- Any mention of "add PTY support only for workflows that genuinely require it" or similar deferral language

**Specific terms to flag:**
- "defer" or "later" in relation to PTY
- "non-interactive" as the "selected" or "primary" approach (should be secondary/fallback at most)

### Concept 3: Database Architecture
**Expected state:** Local SQLite bundled with Rust daemon is the source of truth during operation. Supabase/Postgres is the shared sync target for multi-user visibility. Push-pull sync mechanism between them.

**Hunt for:**
- Any remaining references to Supabase as "the source of truth" or "the control plane"
- ADR-002 — does it describe the dual-database architecture or still recommend Supabase-only?
- Section 5.1 system context diagram — does it show SQLite locally + Supabase as sync target?
- Section 5.4 data model — does it note SQLite compatibility considerations?
- Section 5.6 ingestion flow — does data flow to local SQLite first, then sync?
- Section 5.8 dashboard flow — does desktop read from SQLite and web from Supabase?
- "Hosted Scheduler" component — has it been replaced with daemon-local cron?
- Job leasing mechanism — does it still assume Supabase-only or reflect local SQLite?

**Specific terms to flag:**
- "source of truth" applied to Supabase (should say "shared sync target" or similar)
- "control plane" for Supabase (acceptable if clearly framed as shared visibility layer, not primary authority)
- "pg_cron" or hosted scheduling concepts (should be daemon-local cron)

### Concept 4: Frontend Role Split
**Expected state:** Desktop (Tauri) = full control panel (read, write, trigger workflows, manage agents). Web = read-only viewing portal.

**Hunt for:**
- Any section implying web has write capabilities (reject, bid, trigger, rerun)
- Section 5.8 — does it distinguish desktop and web data flows?
- ADR-008 — does it explicitly state the control/view split?
- User stories (2.3) — are operator write actions scoped to desktop only?
- Section 8.2 web shell files — does the web app still have reject/bid controls?
- Open Question #2 (Section 10) — is it answered?

**Specific terms to flag:**
- "bid" or "reject" or "trigger" in web app context (should be desktop-only)

### Concept 5: Cost-Estimation Engine Data Sources
**Expected state:** Sections 4.4, 4.5, 4.6 are expanded with new data sources, each properly scored.

**Hunt for:**
- Is HMRC FPO API present in Section 4.5 with proper evaluation?
- Is SeaRates Logistics Explorer present in Section 4.4 with proper evaluation?
- Is HMRC Exchange Rate Service present in Section 4.6 as MANDATORY for customs valuation?
- Are CurrencyBeacon and OANDA present as supporting FX options?
- Are HS Ping, Taric Support, Dutify present in Section 4.5?
- Is there a new subsection for third-party landed cost services (Zonos, Avalara)?
- Are all new sources scored using the existing evaluation pattern?
- Is the distinction clear: HMRC rates for customs duty, Xe for commercial FX?
- Were any of the new sources silently dropped?

### Concept 6: Agent Tracing & Observability
**Expected state:** Section 6.6 describes comprehensive agent tracing including full terminal output capture, agent health detection, and error source tracing.

**Hunt for:**
- Does Section 6.6 include full terminal output capture?
- Does it include agent health detection (session expired, stalled, internal error)?
- Does Section 5.11 include agent-specific error types?
- Does the daemon architecture reference log capture?
- Is the tracing comprehensive enough for an engineer to implement?

### Concept 7: Testing Strategy
**Expected state:** Real E2E agent tests are the priority. No emphasis on mocks or stubs.

**Hunt for:**
- Does Section 6.5 prioritize real E2E agent tests?
- Are "recorded fixtures" or "schema assertions" still emphasized for agent testing?
- Are agent E2E test files listed in Section 8.3?
- Is the phrase "mock" or "stub" used positively in the testing strategy?
- Is the success criterion for testing clearly "real agent execution works"?

### Concept 8: Automation Policy & Error Manifest
**Expected state:** Bid automation risk is downgraded. Error manifest system is described for debugging.

**Hunt for:**
- Does ADR-007 reflect relaxed risk framing?
- Does Section 5.9 clarify that "bids" are request pipeline automation, not real-money?
- Is the error manifest system described (structured error log, MD files, dashboard integration)?
- Does Section 6.2 have appropriate risk levels for bid automation?

---

## Audit Dimension 2: Document Boundary Sanity

Verify that the document's internal structure is coherent:

- **Section 4 (External Research)** describes data sources and technology options
- **Section 5 (Architecture)** describes how those options are assembled into a system
- **Section 7 (ADRs)** captures the key decisions from Sections 4-5
- **Section 8 (File Reference)** maps the architecture to concrete file paths
- **Section 9 (Dependencies)** lists what's needed to build the architecture

Check:
- Does every ADR in Section 7 align with the corresponding Section 4/5 analysis?
- Does the file reference map in Section 8 reflect the architecture in Section 5?
- Do the dependencies in Section 9 match what the architecture actually needs?
- Is there any concept defined in one place but contradicted in another?

---

## Audit Dimension 3: Implementation Readiness

Ask: **Could an engineer implement the full system from only this document?**

For each major architecture component, verify:

1. **Skill-based scraping system** — Is it clear HOW skills are triggered, what the agent does in the browser, how data is captured and stored? Or is it hand-wavy?

2. **PTY-based agent execution** — Is it clear HOW the daemon launches PTY sessions, captures output, detects errors, and manages lifecycle? Or just "use PTY"?

3. **SQLite + Supabase sync** — Is it clear WHAT the sync mechanism looks like (conflict resolution, sync frequency, what data syncs, corruption recovery)? Or just "push-pull"?

4. **Error manifest system** — Is it clear WHERE errors are stored, in what FORMAT, how they appear in the dashboard?

5. **Agent E2E testing** — Is it clear WHAT a test looks like (what prompt is sent, what is verified, how long tests take, what constitutes pass/fail)?

6. **Cost-estimation engine** — Are the data source options clear enough to choose between them? Is the recommended composition clear (HMRC FPO for classification + HMRC Tariff for duty + HMRC Exchange Rates for customs FX + Xe for commercial FX + Freightos for ocean/air + SeaRates as complement)?

If any area is too vague, add clarifying detail — enough for implementation, not so much that it becomes implementation planning.

---

## Audit Dimension 4: Overreach Check

Verify the harmonization didn't accidentally:

- Add implementation sequencing or task planning ("first build X, then Y" beyond the existing milestone structure)
- Include tool-specific dev instructions
- Blur into execution guidance
- Over-specify things that are still design choices
- Add content that duplicates or contradicts itself
- Introduce premature abstraction or unnecessary complexity
- Add excessive caveats that dilute clear decisions

---

## Audit Dimension 5: Absorption Completeness

Verify EVERY concept from the owner's feedback has a home in the document:

| Feedback Concept | Expected Location | Present? | Consistent? |
|---|---|---|---|
| Skill-first data retrieval | Sections 4.1, 4.3, 4.10, 5.1, 5.2, 5.6, ADR-001 | ? | ? |
| Browser-enabled agent scraping | Section 4.10, 5.2 | ? | ? |
| Skills easier to maintain than APIs | Section 4.1, 4.3, ADR-001 rationale | ? | ? |
| PTY from day 1 | Section 4.9, 5.2, ADR-004 | ? | ? |
| Cron + manual trigger via dashboard | Section 5.2, 5.6 | ? | ? |
| Comprehensive tracing logs | Section 6.6 | ? | ? |
| Full terminal output from agent | Section 6.6, 5.2 | ? | ? |
| Agent health detection (re-login, errors) | Section 6.6, 5.11 | ? | ? |
| Real E2E agent tests, no mocks | Section 6.5, 8.3 | ? | ? |
| Send real prompts, verify real output | Section 6.5 | ? | ? |
| Cost estimation APIs welcome | Sections 4.4-4.6 | ? | ? |
| HMRC FPO API for HS classification | Section 4.5 | ? | ? |
| SeaRates for freight | Section 4.4 | ? | ? |
| HMRC Exchange Rates for customs FX | Section 4.6 | ? | ? |
| Relaxed bid automation risk | ADR-007, 5.9, 6.2 | ? | ? |
| Request pipeline, not real money bids | Section 5.9 | ? | ? |
| Error flagging system | Section 5.11, 6.6 | ? | ? |
| Error manifest MD file | Section 5.11 or 5.4 | ? | ? |
| Dashboard error display | Section 5.8 | ? | ? |
| Local SQLite with Rust daemon | Section 4.7, 5.2, ADR-002 | ? | ? |
| Supabase as shared sync target | Section 4.7, ADR-002 | ? | ? |
| Push-pull sync mechanism | Section 4.7, 5.2 | ? | ? |
| Corruption resilience rationale | Section 4.7, ADR-002 | ? | ? |
| Desktop = control panel | ADR-008, 5.8 | ? | ? |
| Web = viewing portal only | ADR-008, 5.8 | ? | ? |

If any concept is missing or inconsistent, fix it.

---

## Audit Dimension 6: Terminology Consistency

Verify all sections use identical terminology for:

- **Data retrieval**: "skill-based agent scraping" / "agent browser automation" (not "API connectors" or "connector platform" for procurement/supplier data)
- **Database local**: "local SQLite" (not "local database" sometimes and "SQLite" other times inconsistently)
- **Database shared**: "Supabase/Postgres sync target" or "shared Postgres" (not "source of truth" or "control plane" for Supabase)
- **Agent execution**: "headless PTY" or "PTY-based execution" (not "non-interactive child process" as the primary model)
- **Desktop role**: "control panel" (not "dashboard" used interchangeably for both desktop and web)
- **Web role**: "viewing portal" (not "dashboard" implying write access)
- **Cost data sources**: "cost-estimation APIs" (clearly distinguished from the skill-based approach used for procurement/supplier data)
- **Error tracking**: "error manifest" (consistent term throughout)
- **Testing**: "real E2E agent tests" (not "contract tests" or "recorded fixtures" as primary)

---

## Audit Dimension 7: Fix and Final Fresh-Eyes Pass

Fix any issues found above with targeted edits, not wholesale rewrites.

After all fixes, re-read the ENTIRE document ONE MORE TIME.

**The Implementation Engineer Test:**

Imagine you are the implementation engineer and this document is the ONLY thing you get. Ask yourself:

1. "If I only read Section 5.1, would I know the data retrieval strategy is skill-based agent scraping?" → If not, fix.
2. "If I only read the ADRs, would I get a consistent picture of the architecture?" → If not, fix.
3. "If I read Section 4.7 about the database, would I understand it's SQLite local + Supabase sync?" → If not, fix.
4. "If I read Section 6.5 about testing, would I know to write real E2E tests, not mocks?" → If not, fix.
5. "If I read Section 4.1 about procurement data, would I understand I'm building agent skills, not API clients?" → If not, fix.
6. "If I read Section 4.5 about customs classification, would I know about HMRC FPO API?" → If not, fix.

Fix anything that fails this test.

---

## Required Output

When finished, provide:

1. **Docs changed:** List of sections modified during this audit
2. **Contradictions removed:** Specific contradictions found and resolved
3. **Ambiguities tightened:** Areas where vague language was made concrete
4. **Remaining concerns:** Anything that couldn't be fully resolved or needs owner input
5. **Confidence assessment:** How confident are you (1-10) that the document is now internally consistent and implementation-ready?

---

## Follow-Up Fresh-Eyes Pass

> **Send this as a follow-up message in the same session after completing the audit above.**

One final pass. Re-read the executive summary (Section 1) and the key recommendations. Do they match the body of the document? Specifically:

1. Does the "Primary data strategy" recommendation reflect skill-based agent scraping?
2. Does the "System of record" recommendation reflect SQLite + Supabase sync?
3. Does the "Agent orchestration" recommendation reflect PTY from day 1?
4. Does the "Frontend strategy" recommendation reflect desktop = control, web = view only?
5. Does the "Cost-estimation engine" recommendation reference the expanded data sources?
6. Does the "Automation policy" recommendation reflect the relaxed risk model?
7. Does the "Primary Recommendation in One Sentence" need updating?
8. Do the "Critical Decisions Required" reflect which are now answered?
9. Do the "Quick Stats" still make sense?

Fix any executive summary inconsistencies. The executive summary is often the only thing decision-makers read — it MUST match the body.
