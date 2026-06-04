# Goods-Led Install/Maintenance — Completion Audit (2026-06-03)

> Skill: `bead-completion-audit` v4.0 (audit-only mode — fix beads created, NOT executed, per the step-12 mandatory stop).
> Spec (SOT #1): `goods-led-install-maintenance-spec.md` · TDD (SOT #2): `goods-led-install-maintenance-tdd.md`
> Implementation epic: `testproject1-qoa5` (CLOSED 2026-06-03, 8 phase epics + 74 task beads).

## Verdict

**SUBSTANTIALLY COMPLETE — 2 Tier-A gaps found, 2 fix beads created (`testproject1-qoa5.9`, `testproject1-qoa5.10`).**

- 32/32 Tier-A functional requirements implemented; 30 fully spec-compliant, **2 with spec drift** (FR-L.6-c, FR-R.2-c domestic).
- 3/3 Tier-B research spikes delivered (docs + closed beads) — correctly research-only, not full implementation.
- Full test gate GREEN: 145 TS tests; daemon integration targets `installation_labour_engine` (40), `skill_canary_harness` (31), `skill_lint` (26), `decision_basis_hash` (38), `customs_fx_freight_engine` (41) all pass.
- Both gaps are masked by green CI **because no test pins the specific spec requirement** — exactly the drift class this audit exists to catch.

Per the step-12 mandatory stop, the two fix beads are **created and scoped but NOT executed**. Awaiting human authorization (step 13).

## Method

Read spec + TDD cover-to-cover (35 FRs, 182 ACs, ~60 named test IDs). Ran the full mixed-stack test gate. Fanned out six read-only deep-read agents (one per implementation surface), each verifying ACs against actual code/test bodies — not test names. Personally verified the two candidate gaps the agents surfaced.

## Per-area results

| Surface | Requirements | Result |
|---|---|---|
| Qualification + service gate (TS) | FR-Q.1–Q.6, TS-QUAL-*, TS-DQ-001..008 | ✅ PASS — Phase-1 branch placement, config floors, full 8-row service-gate truth table all correct |
| Coverage rollup + contracts-schema (TS) | FR-R.4, FR-R.5, TS-COV-007/008/009, TS-SCHEMA-001..005 | ✅ PASS — share literals 6944/8065 exact; `MaintenancePeriodEstimateRow` first-class; maintenance opex excluded from denominator |
| Daemon rollup / live loop / hash (Rust) | FR-R.1, R.2, R.3, R.6, R.7 | ⚠️ 1 GAP — labour summed at BOTH sites ✅; live-loop transient-load + job-kind ✅; decision-basis hash ✅; **domestic net-margin bands collapse (R.2-c)** |
| Labour skill + overlay + rate-refresh | FR-L.2–L.5, L.7, L.9–L.12, SKL-* | ⚠️ 1 GAP — frontmatter/sections/8 warning codes/4 reference tables/overlay all ✅; **labour_line_item schema contradicts FR-L.6-c** |
| E2E engine + canary fixtures (Rust) | FR-V.1–V.3, FR-L.1/L.6/L.8 | ✅ PASS — every E2E-IL-001..011 pence/bps literal matches TDD exactly; Scenario-1 band 210000–265000; 7-case fixture; all 8 warning codes pinned; real SQLite seeding, no mocks |
| UI + L0 doc + Tier-B spikes | FR-U.1/U.2, FR-V.4, FR-Z.1–Z.3 | ✅ PASS — labour + permit rows + `labourCoverageBps` + 4-reason review list rendered (SSR tests); L0 §1.6 expansion + metal split; 3 spike docs + closed beads |

Schema epoch confirmed = 10 across all four constants (`sqlite.rs:89`, desktop `main.rs:51`, `query_service.rs:14`, `daemon_bridge.rs:16`); SQLite migration `0014_installation_labour_fields.sql` present & registered.

## Gap 1 — `testproject1-qoa5.9` (FR-L.6-c) · Tier-A · P2

`result.schema.json` `$defs.labour_line_item` requires `trade, hours, crew_size, hourly_rate_gbp_pence, region_multiplier_bps, base_gbp_pence, basis` with `additionalProperties: false` — which **forbids** the FR-L.6-c-mandated fields (`labour_type, quantity_hours, rate_gbp_per_hour_pence, subtotal_gbp_pence, confidence_bps, rate_source`). SKILL.md:88, `golden.assertions.yaml:32-38`, and the daemon E2E fixture (`main.rs:32222`) already use the spec names, so the schema is the lone outlier; a spec-shaped line item would be rejected. The daemon live-loop builder (`main.rs:8185-8224`) emits a third partial naming. No test validates worker output against the schema's line-item shape, so CI stayed green. Fix: reconcile schema (and builder) to the spec field list and add a contract test.

## Gap 2 — `testproject1-qoa5.10` (AC-FR-R.2-c, domestic) · Tier-A · P2

`build_domestic_landed_cost_estimate` (`main.rs:9365-9388`) computes `net_margin_low/base/high_bps` all from the single `economic_landed_cost_gbp_pence` (base-only). The labour/permit LOW/HIGH bands (base ±20% per FR-L.3-c) are never used, so the three domestic margin bands are always identical — the screening band is invisible for domestic install opportunities. The import path does this correctly (RS-ROLLUP-002: 1755/2700/3370). No domestic RS-ROLLUP-003 band test exists. Fix: build low/base/high landed costs from labour/permit low/base/high (mirroring import), order bands inversely to cost, preserve `calibrate_zero_margin` and Scenario-8 non-regression; add the domestic band test.

## Non-gaps explicitly cleared (high-risk per research §6.6)

- Service over-admission (#1 failure mode): all 8 truth-table rows correct — "Supply of maintenance kits" → `accept`, "Installation services" → `reject_service`.
- Both rollup sites sum labour + permit (saturating).
- Scenario 8 non-regression: economic landed cost unchanged, labour/permit 0/null.
- `goods_value_share_bps` reconstructable from a single persisted row (asserted every E2E scenario).
- `MaintenancePeriodEstimate` distinct entity, NOT capitalised into the dominance denominator.
- All 8 warning codes emitted/asserted by the canary corpus.

## Follow-up pass (2026-06-03)

### Fix-bead quality re-review
Both fix beads re-read and judged implementable from the bead alone:
- **`testproject1-qoa5.9`** — cites spec §2.2 FR-L.6 / AC-FR-L.6-c verbatim; TDD §7.7 E2E-IL-001 + §7.6 CAN-LAB-001; exact file:line (`result.schema.json $defs.labour_line_item`, `SKILL.md:88`, `golden.assertions.yaml:32-38`, `main.rs:8185-8224` builder + `:32222-32229` fixture); concrete ACs incl. the missing contract test to add.
- **`testproject1-qoa5.10`** — cites spec §2.3 FR-R.2 / AC-FR-R.2-c verbatim; TDD §7.5 RS-ROLLUP-003; exact file:line (`main.rs:9365-9388` defect, `~9808-9937` import reference); concrete change (build low/base/high landed cost from labour/permit bands, order margins inversely), non-regression AC (Scenario 8), and the missing domestic band test to add.

Each AC is concrete ("rewrite `$defs.labour_line_item` required to …", "derive low/base/high_landed_cost from labour/permit low/base/high"), not "fix this".

### Final coverage count — 100%
35/35 spec requirements walked, none skipped:
- 32 Tier-A = 30 fully spec-complete + 2 fix beads (FR-L.6 → qoa5.9, FR-R.2-c → qoa5.10).
- 3 Tier-B = 3 research spikes delivered (docs + closed beads qoa5.8.1/.8.3/.8.5).
Coverage = (30 complete + 2 fix-bead-tracked + 3 spike) / 35 = **100% accounted for**.

### Full AGENTS.md stage gate — ✅ ALL GREEN (re-run 2026-06-03)
| Gate | Command | Result |
|---|---|---|
| CI-GIM-001 | `cargo fmt --check` | ✅ pass (exit 0) |
| CI-GIM-002 | `cargo check --all-targets` | ✅ pass (exit 0) |
| CI-GIM-003 | `cargo clippy --all-targets -- -D warnings` | ✅ pass (exit 0) |
| CI-GIM-004 | `cargo test -p daemon --bin daemon` | ✅ 264 passed / 0 failed |
| CI-GIM-005 | every `crates/daemon/tests/*.rs` target | ✅ all 60 targets pass (incl. installation_labour_engine 40, skill_canary_harness 31, skill_lint 26, decision_basis_hash 38, customs_fx_freight_engine 41, sqlite_supabase_sync 112, sqlite_schema 53) |
| CI-GIM-006/007 | `bun run test` | ✅ 68 passed / 0 failed (5 files); intelligence pkg 145 passed separately |
| build | `bun run build` | ✅ pass (exit 0) |

The two gaps are real spec drift but do **not** break the build — they are masked by the gate precisely because no test pins the drifted requirement (FR-L.6-c field names; domestic RS-ROLLUP-003 band). Each fix bead adds the missing test.

**Confirming re-run (second pass, 2026-06-03):** the entire gate was executed a second time against the unchanged code surface (the audit modified zero code files — only `.beads/issues.jsonl` for the two new beads and these doc/memory files). Identical result: fmt/check/clippy exit 0; daemon bin **264 passed**; **61/61 integration targets pass**; `bun run test` **68 passed**; `bun run build` exit 0. The green bar is stable and reproducible.

## Next step

⛔ **MANDATORY STOP** — Human authorizes fix-bead execution (manifest step 13 / `13-fix-beads-kickoff.md`) before any fix bead is claimed. Both beads are self-contained (spec AC verbatim, exact file:line, ACs, required test additions). No fix bead has been claimed or executed.
