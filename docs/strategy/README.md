# Strategy Documents — UK Public Procurement Intelligence Platform

## Overview

This directory contains the layer decomposition strategy for building the procurement intelligence platform in 7 sequentially-shippable layers (L0-L6). As of 2026-05-02, L1-L3 are built and annotated in place; L4-L6 remain planned. **Update 2026-06-04:** the goods-led install/maintenance + UK installation-labour cost extension shipped on top of the L5 landed-cost surface (schema epoch 10); the strategy docs were harmonized to it (see L5 §1.17). **Update 2026-06-12:** autonomous daemon scheduling & ProContract resumption shipped on the L3 runtime (schema epoch **11**, `migrations/0015_procontract_pending_enrichment`) — continuous scheduler-tick, always-on two-lane worker, `commercial_sweep` cron kind, and durable awaiting-access tracking that leaves `interest_registration.state='completed'`; the strategy docs were harmonized to it (see L3 §1.5, §1.11).

## Maintained Doc Set

**Normative planning (8 files):** These are the authoritative execution specifications.

| File | Purpose |
|------|---------|
| `layer-strategy.md` | Cross-layer strategy: design rationale, schema stability matrix, interface contracts, stage scope matrix, testing evolution, milestone mapping, stack sequencing |
| `layers/layer-0-gold-set-evaluation.md` | L0: Gold set, evaluation harness, full SQLite DDL, screening-grade pipeline, skills standard |
| `layers/layer-1-thin-local-runner.md` | L1: Rust daemon (20 files), two-lane adapter, session/ctty/reaper, chunked capture, commit fence, IPC protocol, single-database model, stage tracking, run manifests, error manifests |
| `layers/layer-2-single-operator-desktop.md` | L2: Tauri desktop (18 files: 16 shipped + 2 dev-only), commit fence, ControlPlaneState, 5-state bootstrap, 10 bridge commands, interactive SessionViewer, shared UI (FreshnessBadge, ErrorSummaryList), manifest rendering, decision capture |
| `layers/layer-3-operational-hardening.md` | L3: Cron + autonomous scheduler-tick & two-lane worker (`commercial_sweep`, ProContract awaiting-access resumption), resource arbiter, PTY hardening, worker-validator dry-run, skill promotion, relation resolution with cluster identity, validator health monitoring, trust-state lattice |
| `layers/layer-4-shared-visibility.md` | L4: Sync engine (fenced writer, command ledger, hi-water pull), Supabase schema/RLS, web portal (viewer-scoped security boundary), validation enforcement with provisional promotion buffer |
| `layers/layer-5-expanded-commercial-intelligence.md` | L5: Full customs engine, dual FX, freight APIs, multi-source marketplace search, goods-led install/maintenance + UK installation-labour cost engine (§1.17) |
| `layers/layer-6-request-pipeline-automation.md` | L6: Bid workflow, portal-specific skills, audit trail |

**Navigation aids (no planning authority):**

| File | Purpose |
|------|---------|
| `cross-reference-index.md` | Topic lookup: "if you're looking for X, it's in file Y, section Z" |
| `README.md` | This file — reading order and overview |

## Implementation Status Convention

Each layer doc has a top-of-file **Implementation Status** callout dated to the current annotation pass. L1-L3 also include inline status callouts for load-bearing design decisions:

- `✓ As designed` means the implementation follows the strategy choice.
- `⚠ Deviation` records what was actually built when reality differs from the pre-build spec.
- `✗ Deferred` marks planned work that has not landed.
- `↻ Superseded` marks a historical design replaced by a later decision.

Future readers should treat L1-L3 annotations as the 2026-05-02 implementation snapshot, not as a request to rebuild existing code. L4-L6 intentionally have top-of-file status only because they are not built.

## Layer Summary

| Layer | Name | Status (2026-05-02) | Milestone | Key Capability | Exit Criteria |
|-------|------|---------------------|-----------|----------------|---------------|
| 0 | Gold Set + Evaluation Harness | Partially built | M0 | Intelligence thesis validation on curated corpus (≥80 lots: gating + challenge) | Experiments 1–3b pass (incl. hard-negative correctness) |
| 1 | Thin Local Runner | Built | M1 | Daemon-managed skill execution with process supervision | 10 batch runs, >=90% success, 0 orphans |
| 2 | Single-Operator Desktop MVP | Built | M2 | Desktop control panel with live procurement data | Operator runs daily for 1 week |
| 3 | Operational Hardening | Built (autonomous daemon scheduling added 2026-06-12) | M3 | Cron automation (continuous scheduler-tick + always-on two-lane worker, autonomous `commercial_sweep`, ProContract awaiting-access resumption), resource management, dry-run validation | 5 days unattended, >=95% success |
| 4 | Shared Visibility | Not built | M4 | Sync engine, web portal, validation enforcement | 2 users view same pipeline |
| 5 | Expanded Commercial Intelligence | Not built (goods-led labour-cost extension shipped 2026-06-04) | M5 | Decision-grade customs, multi-source pricing, goods-led install/maintenance + UK installation-labour cost engine | >=3 categories, >=2 sources each |
| 6 | Request Pipeline Automation | Not built; Stage 15 blocked | M6 | Portal-specific bid skills | 1 portal submission end-to-end |

## Reading Order

1. Start with `layer-strategy.md` §1 (Design Rationale) for the decomposition philosophy
2. Read each layer doc's top-of-file implementation status callout before relying on its body text
3. Consult `layer-strategy.md` §9-§13 for cross-layer concerns (schema stability, interface contracts, testing evolution)
4. Use `cross-reference-index.md` to find specific topics across layers

## Authority Split

- `layer-strategy.md` owns: layer ordering, schema stability matrix, interface contracts, stage scope matrix, testing evolution, stack sequencing
- Each `layer-N-*.md` owns: layer-local narrative, components, data, invariants, stubs, definition of done, risks, open questions
- `cross-reference-index.md` owns: navigation only (no authoritative content)

## Source Material

The strategy is derived from `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` (~8,403 lines). The research doc is the historical record; the strategy docs are the execution-grade specifications. Implementation agents should work from the layer docs, not the research doc.
