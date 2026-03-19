# Strategy Documents — UK Public Procurement Intelligence Platform

## Overview

This directory contains the layer decomposition strategy for building the procurement intelligence platform in 7 sequentially-shippable layers (L0-L6).

## Maintained Doc Set (10 files)

| File | Purpose |
|------|---------|
| `layer-strategy.md` | Cross-layer strategy: design rationale, schema stability matrix, interface contracts, stage scope matrix, testing evolution, milestone mapping, stack sequencing |
| `layers/layer-0-gold-set-evaluation.md` | L0: Gold set, evaluation harness, full SQLite DDL, screening-grade pipeline, skills standard |
| `layers/layer-1-thin-local-runner.md` | L1: Rust daemon, PTY/contract execution, process supervision, run manifests, error manifests |
| `layers/layer-2-single-operator-desktop.md` | L2: Tauri desktop, shared UI, manifest rendering, decision capture, coverage presentation |
| `layers/layer-3-operational-hardening.md` | L3: Cron, resource arbiter, PTY hardening, worker-validator dry-run, skill promotion |
| `layers/layer-4-shared-visibility.md` | L4: Sync engine, Supabase schema/RLS, web portal, validation enforcement |
| `layers/layer-5-expanded-commercial-intelligence.md` | L5: Full customs engine, dual FX, freight APIs, multi-source marketplace search |
| `layers/layer-6-request-pipeline-automation.md` | L6: Bid workflow, portal-specific skills, audit trail |
| `cross-reference-index.md` | Navigation: "if you're looking for X, it's in file Y, section Z" |
| `README.md` | This file |

## Layer Summary

| Layer | Name | Milestone | Key Capability | Exit Criteria |
|-------|------|-----------|----------------|---------------|
| 0 | Gold Set + Evaluation Harness | M0 | Intelligence thesis validation on curated corpus | Experiments 1-3 pass |
| 1 | Thin Local Runner | M1 | Daemon-managed skill execution with process supervision | 10 batch runs, >=90% success, 0 orphans |
| 2 | Single-Operator Desktop MVP | M2 | Desktop control panel with live procurement data | Operator runs daily for 1 week |
| 3 | Operational Hardening | M3 | Cron automation, resource management, dry-run validation | 5 days unattended, >=95% success |
| 4 | Shared Visibility | M4 | Sync engine, web portal, validation enforcement | 2 users view same pipeline |
| 5 | Expanded Commercial Intelligence | M5 | Decision-grade customs, multi-source pricing | >=3 categories, >=2 sources each |
| 6 | Request Pipeline Automation | M6 | Portal-specific bid skills | 1 portal submission end-to-end |

## Reading Order

1. Start with `layer-strategy.md` §1 (Design Rationale) for the decomposition philosophy
2. Read layer docs sequentially (L0-L6) for the build order — each is self-contained
3. Consult `layer-strategy.md` §9-§13 for cross-layer concerns (schema stability, interface contracts, testing evolution)
4. Use `cross-reference-index.md` to find specific topics across layers

## Authority Split

- `layer-strategy.md` owns: layer ordering, schema stability matrix, interface contracts, stage scope matrix, testing evolution, stack sequencing
- Each `layer-N-*.md` owns: layer-local narrative, components, data, invariants, stubs, definition of done, risks, open questions
- `cross-reference-index.md` owns: navigation only (no authoritative content)

## Source Material

The strategy is derived from `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` (~8,400 lines). The research doc is the historical record; the strategy docs are the execution-grade specifications. Implementation agents should work from the layer docs, not the research doc.
