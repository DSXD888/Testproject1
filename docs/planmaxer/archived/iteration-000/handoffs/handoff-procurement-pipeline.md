# Handoff: Procurement Pipeline & Product Matching

## Expert Framing

You are a **search relevance / information retrieval engineer** with experience in product matching, catalog reconciliation, or procurement analytics. You have built systems that match unstructured text descriptions to structured product catalogs, dealt with the messiness of real-world procurement data, and understand the difficulty of comparing apples to oranges across different taxonomies.

Draw from: information retrieval techniques (TF-IDF, BM25, embedding-based search), product matching in ecommerce (Amazon catalog matching, Google Shopping), procurement classification standards (CPV, UNSPSC, HS), and the real-world failure modes of automated procurement systems. Also draw from experience with UK public procurement data — OCDS, Find a Tender, Contracts Finder formats and their quirks.

## Mission

ULTRATHINK and be meticulous.

You are reviewing the **procurement pipeline and product matching layer** of a UK procurement intelligence platform. This layer handles: discovering procurement notices, normalizing them, deduplicating/merging into canonical processes, qualifying opportunities (expiry, goods/services), extracting product candidates, and matching them to supplier marketplace listings.

The core value proposition: transform noisy UK procurement feeds into actionable arbitrage opportunities by identifying physical goods contracts and finding cheaper supplier alternatives. The product matching step — connecting messy procurement notice text to actual marketplace products — is the hardest technical problem in the entire system.

**Don't just answer my questions — the questions I DON'T KNOW to ask are more important than the ones I do.**

## User's Strong Beliefs (Engage With These)

1. **"Marketplace prices are indicative signals, not executable procurement offers."** The owner explicitly acknowledges that eBay/Amazon/Alibaba prices are not what you'd actually pay for procurement-scale quantities. Challenge: if the prices are known to be unreliable, how valuable is the matching step? Should the system invest more in getting to RFQ-grade pricing earlier?

2. **"Deterministic rules before AI."** The architecture uses rule-based gating (expiry date, category field, quantity/unit presence) before AI classification. Challenge: where's the right boundary? Are there rule-based approaches to product matching that are being underutilized?

3. **"Conservative merge heuristics with confidence thresholds."** For deduplicating notices into canonical procurement processes, the plan says "only auto-merges when merge confidence clears a threshold." Challenge: how should merge confidence be computed? What happens when over-merging conflates separate opportunities? What happens when under-merging creates duplicates?

## Essential Context

**What the project is:** A procurement intelligence platform that ingests UK public contract notices daily, filters to physical goods opportunities, matches them to cheaper supplier alternatives, and estimates margin.

**Key terms:**
- **Procurement process:** A canonical entity representing a single procurement opportunity, potentially composed of multiple notices across sources.
- **Contract notice:** A source-specific document from Find a Tender, Contracts Finder, or PCS. Multiple notices can map to one procurement process.
- **OCID:** Open Contracting ID — a unique identifier for procurement processes in the OCDS standard.
- **CPV codes:** Common Procurement Vocabulary codes used in EU/UK procurement. Hierarchical classification of goods/services.
- **Product candidate:** An extracted line item from a procurement notice — a named product with estimated quantity, specification text, and candidate commodity codes.
- **Supplier quote:** A price observation from a marketplace (eBay, Amazon, Alibaba, 1688) for a product that matches a product candidate.

**Technical constraints:**
- Data retrieval is via agent skill scraping (browser automation), not API clients
- Procurement notices are often vague, mixed (goods + services in one contract), and inconsistently structured
- Marketplace product catalogs use different taxonomies than procurement CPV codes
- Agent scraping returns noisy, incomplete, and sometimes wrong data
- The system processes notices daily with time-sensitive deadlines

## Current Architecture

### Ingestion Pipeline
```
Agent scrapes procurement source (via skill + PTY session)
  → Raw payload stored in raw_documents
  → Normalized into contract_notices schema
  → Dedupe/merge into procurement_processes using OCID + heuristics
  → Qualification jobs queued for new/updated processes
```

### Qualification Pipeline
```
Read normalized notice
  → Check deadline (reject if < 30 days)
  → Check category:
      goods → pass
      services → fail
      works → fail (unless separable goods delivery)
      mixed/unknown → AI classification via skill
  → If deadline missing/uncertain → manual review
  → Extract product candidates
```

### Product Extraction
```
Messy notice text → ProductCandidate entities:
  - name
  - quantity, quantityUnit
  - specificationText
  - estimatedUnitWeightKg, estimatedUnitVolumeM3
  - candidateCommodityCodes
  - extractionConfidence
```

### Supplier Matching
```
For each product candidate:
  → Generate search profile from title, description, CPV, line items
  → Agent scrapes UK sources (eBay, Amazon) via skills
  → Agent scrapes China sources (Alibaba, 1688) via skills
  → Score comparability
  → Save SupplierQuote with:
      region, quoteClass, unitPriceGbp, confidence, evidence
```

### Notice Deduplication
```
Same process can appear across:
  - Find a Tender AND Contracts Finder (cross-source)
  - Multiple notices within same source (updates/amendments)

Merge uses:
  - OCID matching
  - Source heuristics
  - Only auto-merge when confidence clears threshold
```

### Opportunity State Machine
```
discovered → normalized → prelim_rejected/prelim_qualified →
needs_review → priced → gross_qualified → landed_costed →
net_qualified → operator_rejected/watching/ready_to_bid →
bid_in_progress → bid_submitted → expired
```

## Tensions to Resolve

### Tension 1: Product Matching Accuracy as the Core Bottleneck
**The conflict:** The research document identifies "product matching between contract descriptions and marketplace listings" as the #1 risk area. But the architecture dedicates minimal design space to HOW matching actually works. The `ProductCandidate` extraction and the "generate search profile" for supplier matching are both hand-waved.

**Current resolution:** AI extraction (via skill) producing structured candidates, then marketplace search.

**Why it might be wrong:** The quality of product matching determines everything downstream. A bad match means wrong supplier price, wrong freight estimate, wrong HS code, wrong margin. The plan has detailed schemas for storing matches but almost no specification for how matches are made, scored, or validated.

**Stress test:** A procurement notice reads: "Supply and delivery of personal protective equipment including hi-vis vests, safety boots, hard hats, and ear defenders — estimated quantities to be confirmed." How does the system extract product candidates? What does it search for on Alibaba? How does it know the Alibaba result for "safety helmet" matches the notice's "hard hat"?

### Tension 2: Notice-to-Process Merging Risk
**The conflict:** The same procurement opportunity can appear across multiple sources and as multiple notices. The system must merge them into a single canonical process. But over-merging (combining separate opportunities) and under-merging (creating duplicates) are both costly.

**Current resolution:** "OCID and source heuristics" with "merge confidence threshold."

**Why it might be wrong:** OCID isn't universally present or consistent across sources. Find a Tender and Contracts Finder use different identifiers. A notice amendment may look like a new opportunity. Two genuinely separate frameworks for "office furniture" from the same buyer may look like one.

**Stress test:** Find a Tender publishes a notice for "NHS Hospital Equipment Framework." Contracts Finder publishes "Medical Equipment Supply Agreement" from the same NHS trust, same month. Are these the same procurement process? What if one is a framework notice and the other is a call-off?

### Tension 3: Goods vs. Services Classification Edge Cases
**The conflict:** The qualification step rejects services and works contracts. But many high-value procurement opportunities are mixed (goods + services), and "works" contracts sometimes include significant goods delivery.

**Current resolution:** Explicit category fields first; mixed/unknown goes to AI classification.

**Why it might be wrong:** The biggest procurement opportunities are often frameworks that bundle goods with installation, maintenance, and support services. Rejecting "works" contracts for "building new school — including all furniture, IT equipment, and catering equipment supply" throws away the goods component.

**Stress test:** A £5M "works" contract for school refurbishment that includes £1.5M of furniture and £500K of IT equipment. The mainCategory is "works." The system rejects it. But the goods component alone is worth pursuing. How should this be handled?

### Tension 4: Search Profile Quality for Cross-Language Matching
**The conflict:** UK procurement notices are in English. Alibaba listings may be in English (with poor translations from Chinese). 1688 listings are primarily in Chinese. The search profile must bridge this gap.

**Current resolution:** Not specified.

**Why it might be wrong:** Direct keyword matching between procurement English and marketplace English/Chinese will have very low recall. "Hi-vis vests" won't match "reflective safety clothing" or "反光安全服." The plan assumes the agent skill handles this, but the skill's search strategy is unspecified.

**Stress test:** Procurement notice requires "Nitrile examination gloves, powder-free, EN 455 compliant." 1688 has the product but listed as "一次性丁腈手套 无粉" — disposable nitrile gloves, powder-free. How does the search profile bridge this? Is the agent expected to translate and search in Chinese?

### Tension 5: Pipeline Throughput vs. Agent Cost
**The conflict:** Each qualification and enrichment step runs through an agent PTY session. For a daily pipeline processing hundreds of notices, that's hundreds of agent invocations — each consuming API tokens, time, and local compute.

**Current resolution:** Not explicitly addressed.

**Why it might be wrong:** If 100 notices are ingested daily, and each needs goods classification, product extraction, UK supplier search, and China supplier search — that's 400+ agent sessions per day. At current Claude/Codex pricing, this could cost $50-200/day in API tokens alone. The plan doesn't discuss cost management or batching.

**Stress test:** A high-volume day: 200 new notices from Find a Tender. After expiry filtering, 120 need goods classification. After that, 60 need product extraction and supplier matching. That's 60 × 4 supplier skills = 240 agent sessions for matching alone. What's the cost? What's the time?

## What I Don't Want (Anti-Patterns)

- **Don't suggest replacing agent-based extraction with traditional NLP/NER pipelines.** The skill-based agent approach is an architectural decision.
- **Don't propose building a proprietary product taxonomy.** The system should work with existing classification standards (CPV, HS, marketplace categories).
- **Don't suggest adding a human-in-the-loop for every product match.** The system needs to scale to daily pipeline volumes.
- **Don't recommend adding embedding search infrastructure (vector DB) for MVP.** Keep it simple first.

## What I Do Want

1. **A concrete product matching strategy** — how should the system convert procurement text to marketplace search queries? What scoring model makes sense for match quality? What's the minimum viable matching approach?

2. **Notice merge/dedupe specification** — beyond "OCID + heuristics," what fields should be compared? What's a reasonable merge confidence function?

3. **Goods/services classification approach for edge cases** — how should mixed contracts and goods-within-works be handled? Is there a CPV-based heuristic that works?

4. **Cross-language matching strategy** — how should the system bridge English procurement text to Chinese marketplace listings?

5. **Pipeline cost and throughput analysis** — what's a realistic daily pipeline cost? How should the system batch or optimize agent invocations?

6. **Ideas I haven't asked about** — product matching approaches I haven't considered, procurement data quality issues that will bite, classification edge cases, marketplace search strategies that work for procurement-scale goods.

7. **Recommendations in git-diff style** — reference specific sections of the research document (sections 4.1-4.3, 5.3, 5.4, 5.6, 5.7, 6.1, 6.2) when proposing changes.

## Context Documents to Attach

When sending this to a reviewer, attach:
- The full research document: `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
- Pay special attention to: Sections 4.1-4.3 (Procurement + Supplier Sources), 5.3 (Canonical Workflow), 5.4 (Data Model — notice/process/product/quote entities), 5.6 (Ingestion Data Flow), 5.7 (Intelligence Pipeline), 6.1 (Complexity Analysis), 6.2 (Risk Assessment)
