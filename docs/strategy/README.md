# Strategy Documents — UK Public Procurement Intelligence Platform

## Overview

This directory contains the layer decomposition strategy for building the procurement intelligence platform in sequentially-shippable layers.

## Documents

| File | Purpose |
|------|---------|
| `layer-strategy.md` | **Primary document.** Full layer decomposition with per-layer specifications, cross-layer contracts, schema stability matrix, testing evolution, and milestone mapping. |
| `cross-reference-index.md` | Navigation index: "if you're looking for X, it's in Layer N §Y." |
| `README.md` | This file. |

## Layer Summary

| Layer | Name | Milestone | Key Capability |
|-------|------|-----------|----------------|
| 0 | Gold Set + Evaluation Harness | M0 | Intelligence thesis validation on curated corpus |
| 1 | Thin Local Runner | M1 | Daemon-managed skill execution with process supervision |
| 2 | Single-Operator Desktop MVP | M2 | Desktop control panel with live procurement data |
| 3 | Operational Hardening | M3 | Cron automation, resource management, dry-run validation |
| 4 | Shared Visibility | M4 | Sync engine, web portal, validation enforcement |
| 5 | Expanded Commercial Intelligence | M5 | Decision-grade customs, multi-source pricing |
| 6 | Request Pipeline Automation | M6 | Portal-specific bid skills |

## Reading Order

1. Start with `layer-strategy.md` §1 (Design Rationale) for the decomposition philosophy
2. Read layers sequentially (§2–§8) for the build order
3. Consult cross-layer sections (§9–§13) for schema stability, interface contracts, and testing evolution
4. Use `cross-reference-index.md` to find specific topics across layers

## Source Material

The strategy is derived from `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` (~8,400 lines), which remains the authoritative source for architectural details not covered in the strategy.
