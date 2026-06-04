# Goods-Led Install/Maintenance — Pipeline Execution Guide

**Pipeline Entry Point:** Batch 1 (Create Spec — Pattern A, research doc → full pipeline)
**Total Handoffs:** 15 (01–15) + this guide + `manifest.yaml`
**Feature:** Widen the qualification filter for goods-LED supply-and-install / maintenance contracts + build a modular UK installation/maintenance labour-cost engine. Both halves ship together.
**Source:** `docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md`

---

## Quick Reference

| Item | Value |
|------|-------|
| Spec | `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md` |
| TDD | `.../goods-led-install-maintenance-tdd.md` |
| Strategy | `.../goods-led-install-maintenance-strategy.md` |
| Kickoff / Push | `.../goods-led-install-maintenance-kickoff.md` / `-push.md` |
| Status | `.../goods-led-install-maintenance-status.yaml` |
| Bead CLI | `br` (sync via `bin/br-sync`) |
| Bead label | `goods-led-install` (IDs are `testproject1-*`) |
| AgentMail | enabled — project_key `/Users/alinaaitova/Desktop/Testproject1` |
| Green bar | FULL AGENTS.md stage gate (fmt + check + clippy + bin + EVERY `tests/*.rs` + `bun test`) |

---

## Execution Order

### Batch 1: SPEC
| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 1 | `01-create-spec.md` | `/research-to-spec` | — |
| 2 | `02-optimize-spec.md` | none (fresh-eyes audit) | Session 1 |

→ **ENRICH 03–14** (see below)

### Batch 2: TDD
| 3 | `03-create-tdd.md` | `/spec-to-tdd` | Batch 1 |
| 4 | `04-optimize-tdd.md` | none (audit) | Session 3 |

→ **ENRICH 05–14**

### Batch 3: STRATEGY
| 5 | `05-create-strategy.md` | `/spec-to-strategy` | Batch 2 |
| 6 | `06-optimize-strategy.md` | none (audit) | Session 5 |

→ **ENRICH 07–14**

### Batch 4: BEADS
| 7 | `07-create-beads.md` | `/beads-from-sources` | Batch 3 |
| 8 | `08-optimize-beads.md` | none (audit) | Session 7 |

→ **ENRICH 09–14**

### Batch 5: AUDIT
| 9 | `09-audit-beads.md` | `/bead-plan-audit` | Batch 4 |

### Batch 6: KICKOFF + PUSH
| 10 | `10-generate-kickoff.md` | `/beads-to-execution` | Batch 5 |

⛔ **MANDATORY STOP — Human authorizes execution.**

| 11 | `11-push.md` | none (continuation nudge) | during execution |

⛔ **MANDATORY STOP — Human triggers completion audit.**

### Post-Implementation
| 12 | `12-completion-audit.md` | `/bead-completion-audit` | execution complete |

⛔ **MANDATORY STOP — Human authorizes fix-bead execution.**

| 13 | `13-fix-beads-kickoff.md` | none (generated onboarding) | Batch after step 12 |
| 14 | `14-fix-beads-push.md` | none (continuation nudge) | during fix execution |
| 15 | `15-archive.md` | none | all fix beads complete |

The 12→13→14→(re-audit) loop repeats until the completion audit returns COMPLETE.

---

## Enrichment Between Batches (5 min each)

Later handoffs carry `{ENRICHMENT: ...}` markers and generic references. After each batch, read the new artifact and edit subsequent handoffs to replace generics with specifics.

### After Batch 1 (spec created) → enrich 03–14
Read the spec. Extract: requirement ID ranges (FR-Q/L/R/U/V or whatever the spec used), subsystem count, total requirement count. Edit handoffs 03+ to say "the spec has N requirements across M groups; IDs range FR-…". Add spec-specific verification questions to the 03/04 sub-optimizations.

### After Batch 2 (TDD created) → enrich 05–14
Read the TDD. Extract: test ID ranges (TS-QUAL-*, TS-COV-*, the 8 truth-table rows, 11 `installation_labour_engine.rs` manifests, 7 canary cases), counts by tier, the coverage-matrix summary. Edit handoffs 05+ with actual counts; update the strategy gate-criteria references to real test IDs.

### After Batch 3 (strategy created) → enrich 07–14
Read the strategy. Extract: phase names, phase count, expected beads per phase, gate criteria. Edit handoffs 07+ with phase→epic mapping and expected bead count.

### After Batch 4 (beads created) → enrich 09–14
Read the bead inventory (`br list --label goods-led-install --json`). Extract: implementation epic ID, total bead count, ready count, per-phase distribution. Write the epic ID into `manifest.yaml` + the status file. Edit handoffs 09+ with the epic ID and expected coverage numbers.

---

## ZERO SCOPE DRIFT — The Cardinal Rule

The research doc has been through two researchmaxer passes and is verified against codebase reality. It IS the contract. **Nothing in it is silently dropped.** Scope is two-tiered (operator-decided 2026-06-02 — see `manifest.yaml` SCOPE TIERS):

- **TIER A — committed, full depth.** MVP (§2.4) + the §2.4 "Phase 2 (Should Have)" items (recurring maintenance, access-equipment auto-selection, quarterly rate-refresh skill, overlay-labour-cost). These get full spec ACs + named tests + full beads, at equal depth. Every optimize/audit step scans Tier-A content for red-flag language ("post-MVP," "Phase 2+," "stretch goal," "deferred," "out of scope," etc.) and verifies coverage.
- **TIER B — bounded in-scope + research spike.** The three §2.4 "Future (Could Have)" items that §10 ALSO flags as "requiring further research": live per-estimate enrichment lane; installer-quote ingestion (`quote_class = installer_quote`); the detailed per-authority/per-zone permit sub-model. Each is NAMED + BOUNDED in the spec (scope boundary + extension point + §10 citation), gets one `research-spike` bead (research/design gate, not a code test), and is tracked through every audit — but is NOT given fabricated full ACs or expanded into a delivery epic. The research/later framing on these is *correct*, not drift.

The only outright exclusions are the research doc's own §2.5 exclusions: service-only contracts, heavily-regulated goods, firm pricing, and doing the installation.

Track the full surface in `manifest.yaml` → `feature_surface` (Tier A) + `future_tier_b` (Tier B). At every audit step, re-verify each item is covered at its tier.

---

## Feature-Specific Watch-List (carry through every step)

1. **Both rollup sites** — labour summed at `main.rs:8114` (domestic) AND `:8548` (import).
2. **Migration nullability** — new columns nullable/default-0 or legacy direct-INSERT tests break.
3. **Schema-epoch 9→10** — all 4 constants (daemon sqlite.rs, desktop main.rs=9, daemon_bridge.rs=8, query_service.rs=8) + Supabase `schema_metadata` + projection migration.
4. **Service gate** — add `goods_led_mixed` to the EXISTING `DescriptionQualityVerdict` union; create the NEW `description-quality.test.ts` (zero coverage today — the #1 silent-failure risk).
5. **Two-phase split** — Phase-1 early (goods-presence, no cost share); Phase-2 post-cost dominance gate; prefer the contract-value basis when available; near-floor → `review`.
6. **Labour skill** — `http_public`, no CDP; 4 skill-local reference JSONs; result-envelope mirrors freight but NOT its profile.
7. **Scenario-1 hard assert** — labour in 210000–265000 pence (NW festive lights).
8. **Recurring maintenance** — `MaintenancePeriodEstimate`, capital-event-vs-opex split (not capitalised into the dominance denominator).
9. **Fail-soft** — `blocked` labour → `review`, never a crash.
10. **No `registry.rs` edit, no `next_job_kind` chaining** — transient `load_skill` in `run_landed_cost_live_loop` only.
