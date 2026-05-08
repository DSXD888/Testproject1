# L4 Shared Visibility — Pipeline Execution Guide

> Generated: 2026-05-06 by pipemaxxer · Pattern B (layer doc → spec validation)
> **Prerequisite:** L3 must be operational and unattended-stable (>=95% skill success over 5 days). Do not start L4 until that gate is met.

---

## Pipeline Entry Point: Batch 1 (Validate Existing Spec)

**Source spec:** `docs/strategy/layers/layer-4-shared-visibility.md` (797 lines, 18 DoD criteria, 14 invariants, 17 test scenarios, 4 test files cumulative-52, 8 Supabase migrations, 1 SQLite sync migration, 1 daemon module, 7 web files, 4 validator overlays).

**Total handoffs:** 15 (01-validate-spec → 15-archive)

---

## Overview Table

| Batch | Step | Skill | Output | Sub-Opt |
|-------|------|-------|--------|---------|
| 1 SPEC | 01 validate-spec | (manual) | spec validated in-place | yes |
| 1 SPEC | 02 optimize-spec | (manual) | spec audited fresh-eyes | yes |
| 2 TDD | 03 create-tdd | `/spec-to-tdd` | `l4-shared-visibility-tdd.md` + `l4-shared-visibility-bootstrap.md` | yes |
| 2 TDD | 04 optimize-tdd | (manual) | TDD audited fresh-eyes | yes |
| 3 STRATEGY | 05 create-strategy | `/spec-to-strategy` | `l4-shared-visibility-strategy.md` | yes |
| 3 STRATEGY | 06 optimize-strategy | (manual) | strategy audited fresh-eyes | yes |
| 4 BEADS | 07 create-beads | `/beads-from-sources` | `l4-*` beads + bulk-import doc | yes |
| 4 BEADS | 08 optimize-beads | (manual) | beads audited fresh-eyes | yes |
| 5 AUDIT | 09 audit-beads | `/bead-plan-audit` | beads audited zero-drift | yes |
| 6 KICKOFF | 10 generate-kickoff | `/beads-to-execution` | `l4-shared-visibility-{kickoff,push,status}.{md,yaml}` | NO (deliverable) |
| 6 KICKOFF | 11 push (use as needed) | (template) | (used during execution) | — |
| POST | 12 completion-audit | `/bead-completion-audit` | audit report + `audit-fix` beads | yes |
| POST | 13 fix-beads-kickoff | (template) | fix beads agent onboarding | NO (deliverable) |
| POST | 14 fix-beads-push | (template) | fix beads continuation | NO (deliverable) |
| POST | 15 archive | (manual) | pipeline archived; status updated | — |

---

## Execution Order

### Batch 1 — SPEC (validate + optimize the existing layer doc)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 1 | `01-validate-spec.md` | manual | — |
| 2 | `02-optimize-spec.md` | manual | session 1 |

**What to do:** L4 doc IS already a structured spec. Session 1 verifies requirement IDs (`L4-FR-X.Y.Z`), ACs, structure, completeness, and resolves the migration-filename mismatch (spec says `sqlite/migrations/0002_sqlite_state_sync.sql`; actual repo convention is flat `migrations/NNNN_*.sql` next-free-0012). Session 2 is a fresh-eyes audit by an independent agent.

**Gate:** Spec has unique requirement IDs, every requirement has explicit ACs, no scope-drift language ("post-MVP", "Phase 2+", "stretch goal", "deferred", "future iteration"), the cross-domain notes are accurate, and the L0 `local_outbox` reservation question is clarified (L4 owns the DDL).

### Enrichment after Batch 1
Read the validated spec. Extract:
- Requirement ID range (e.g., `L4-FR-1.1.1` … `L4-FR-3.7.1`) and total count
- Section count (5 main sections expected)
- DoD criterion count (18 expected; verify same after validation)
- Updated invariant count
- Decision on migration filename and `local_outbox` ownership

Edit handoffs **03-15** — replace `{ENRICHMENT: spec metadata}` markers with actuals.

---

### Batch 2 — TDD (test specification + bootstrap)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 3 | `03-create-tdd.md` | `/spec-to-tdd` | Batch 1 |
| 4 | `04-optimize-tdd.md` | manual | session 3 |

**What to do:** Session 3 invokes `/spec-to-tdd` to produce both the TDD spec (test file inventory, test catalog, coverage matrix) and the bootstrap doc (environment prerequisites, validation script, supabase env setup). Session 4 audits both with fresh eyes.

**Gate:** Every L4-FR-X.Y.Z has at least one test ID, every DoD criterion (1-18) maps to a verifying test, the four planned test files (`sqlite_schema.rs`, `sqlite_supabase_sync.rs`, `redaction_boundary.rs`, `apps/web/e2e/contracts.spec.ts`) are concretely scoped with exact assertions, the calibration harness has tests for all 9 mutation families and pairwise challenge sets, RLS redaction tests cover all 6 summary projections, the Bootstrap doc covers the new dependencies (`reqwest` for daemon, Next.js for web, Supabase project creation, `infra/supabase/config.toml`).

### Enrichment after Batch 2
Read the TDD. Extract:
- Test ID range (e.g., `L4-T-sqlite_supabase_sync-001` … `L4-T-contracts_spec-NN`)
- Test counts by file and by type (unit / integration / e2e / E2E migration test / RLS / redaction / canary / calibration)
- CI tier distribution
- Coverage matrix summary (DoD criterion → test IDs)
- Environment prerequisites from Bootstrap (Supabase CLI version, Postgres role/grant SQL, env vars)

Edit handoffs **05-15** — replace `{ENRICHMENT: tdd metadata}` markers with actuals.

---

### Batch 3 — STRATEGY (phase ordering + dependency DAG)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 5 | `05-create-strategy.md` | `/spec-to-strategy` | Batch 2 |
| 6 | `06-optimize-strategy.md` | manual | session 5 |

**What to do:** Session 5 invokes `/spec-to-strategy` to produce phase ordering only (no spec/TDD content duplication). Session 6 audits.

**Gate:** Every L4-FR-X.Y.Z appears in some phase, phases are labeled execution-order (NOT importance tiers), later phases have SAME depth as early phases, the suggested phase ordering respects critical dependencies (SQLite migration → outbox writer → push dispatcher → pull engine → CAS → enforcement → web), no scope-drift language present.

### Suggested phase shape (heuristic — validate against critical-path analysis in Step 5)
1. **Schema Foundation** — Add `local_outbox` / `sync_batches` / `pull_cursors` to SQLite (`migrations/0012_l4_sync_state.sql` or successor). Bootstrap `infra/supabase/{config.toml, migrations/0001-0008}`. Add `reqwest` to daemon. Wire `apps/web` skeleton. **Root of DAG.**
2. **Push Engine** — Outbox writer in same SQLite transaction; push dispatcher with single-in-flight, identical-envelope retries, equal-version retry semantics; `apply_push_batch` RPC; writer fencing token (`writer_epoch`); active-writer lease; `commit_group_id`/`stage_run_id` boundary checks; viewer_sync_status update on success.
3. **Pull Engine + CAS** — Hi-water fenced cursor pull for 3 domains; pull-before-push ordering at sync startup; `update_operator_decision` CAS RPC; `operator_decision_commands` durable command ledger; pending operator intent reconciliation (latest-per-process coalescing).
4. **Validation Enforcement** — Provisional promotion buffer (local-only staging table or artifact dir); per-slice enforcement readiness gating (Wilson-bound precision, rule-of-three false-pass, calibration thresholds); 4 domain validator overlays; auto-demotion triggers; AGENT_SUSPECT_OUTPUT quarantine flow; calibration harness with 3 corpora + mutation bank + pairwise challenge sets; cross-model audit sampling.
5. **Web Portal** — `apps/web/src/app/{layout,page,contracts,contracts/[id],status}` Next.js routes; `lib/{supabase.ts, broadcast.ts}`; viewer-credentials-only enforcement; three-state degraded-mode UX (healthy/amber/red); Broadcast invalidation hints (NOT row payloads); per-page topic subscription lifecycle; Playwright e2e covering RLS boundary.
6. **Integration & Hardening** — Schema epoch guard + minimum epoch-repair reproject path (NOT deferred — Issue #696 in §5.1); RLS policies for all 4 roles + redaction tests for all 6 summary projections; logical consistency auditor stub; tombstone retention (>=14 days); two-user manual verification; full E2E push-pull round-trip; final DoD walkthrough.

### Enrichment after Batch 3
Read the strategy. Extract: phase names, total phases, tasks/beads per phase, gate criteria. Edit handoffs **07-15** with actuals.

---

### Batch 4 — BEADS (executable units)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 7 | `07-create-beads.md` | `/beads-from-sources` | Batch 3 |
| 8 | `08-optimize-beads.md` | manual | session 7 |

**What to do:** Session 7 invokes `/beads-from-sources` to materialize beads from the 3 SOTs. Session 8 audits coverage and uniqueness, runs `br lint`, and verifies bead self-containment.

**Gate:** Bead prefix is `l4`. Every L4-FR-X.Y.Z has a bead. Every L4-T-* test ID is owned by exactly one bead. No bead is empty (every bead has full file paths, type definitions, test specs, ACs). The existing epic `testproject1-4zq` is hooked up as parent or replaced by a richer epic structure (4 sub-epics per spec or 6 sub-epics per phase). Run `bin/br-sync && git add .beads/ && git commit && git push`.

### Enrichment after Batch 4
Run `br list --json | grep l4`. Extract: total bead count, ready-bead count, distribution per phase. Edit handoffs **09-15** with actuals.

---

### Batch 5 — AUDIT (last gate before execution)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 9 | `09-audit-beads.md` | `/bead-plan-audit` | Batch 4 |

**What to do:** Systematic spec-drift audit against beads. Read every bead, walk every spec L4-FR, verify zero coverage gap, scan for scope-reduction language.

**Gate:** Coverage matrix shows 100% L4-FR → bead mapping. Zero "post-MVP", "Phase 2+", "deferred", "stretch goal" language anywhere in the bead corpus. Phase coverage is balanced (no anomalously thin late phase).

---

### Batch 6 — KICKOFF (generate execution prompts)

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 10 | `10-generate-kickoff.md` | `/beads-to-execution` | Batch 5 |
| (11) | `11-push.md` | (template; used as needed) | session 10 |

**What to do:** Session 10 produces the kickoff (~350+ lines, 12 sections matching L3 quality bar), push (~75 lines), and status YAML. The kickoff and push are the documents agents use during implementation. Session 11 is the push prompt itself — reused continuously by continuation agents.

⛔ **MANDATORY STOP — Human Authorizes Execution**

Before running `10`, the human must:
1. Read the kickoff/push (review against L3 quality bar)
2. Verify AgentMail registration block is correct for `/Users/alinaaitova/Desktop/Testproject1`
3. Verify `br ready` shows the L4 bead graph in expected order
4. Verify quality gates list correct Rust + TS + UBS commands
5. Authorize parallel agent execution

---

### Execution Phase

After authorization, agents claim L4 beads and execute. The push prompt (11) is a fresh-context restart for any continuation agent. The kickoff (10) is for agents who haven't seen this work yet.

⛔ **MANDATORY STOP — Human Triggers Completion Audit**

When `br list --status=open --label=layer-4` returns empty (or only `audit-fix` candidates), the human triggers Batch 12.

---

### POST — Completion Audit + Fix Loop

| Session | File | Skill | Depends On |
|---------|------|-------|-----------|
| 12 | `12-completion-audit.md` | `/bead-completion-audit` | execution complete |
| 13 | `13-fix-beads-kickoff.md` | (template) | session 12 |
| 14 | `14-fix-beads-push.md` | (template) | execution started |
| 15 | `15-archive.md` | manual | all complete |

**What to do:** Session 12 walks every L4-FR-X.Y.Z against the codebase + tests; gaps become `audit-fix` beads. Sessions 13-14 onboard agents to those fix beads. Session 15 archives the pipeline.

**Gate:** Final DoD walkthrough confirms all 18 acceptance criteria are met. Two-user manual verification (desktop + web on same dataset) passes. Sync engine green. Validation enforcement active on calibrated slices. RLS boundaries enforce security at the database, not the UI.

---

## Quick Reference

| | |
|---|---|
| Spec | `docs/strategy/layers/layer-4-shared-visibility.md` |
| TDD | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` |
| Bootstrap | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md` |
| Strategy | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` |
| Kickoff | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-kickoff.md` |
| Push | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-push.md` |
| Status | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-status.yaml` |
| Handoff dir | `docs/pipeline/l4-shared-visibility/` |
| Manifest | `docs/pipeline/l4-shared-visibility/manifest.yaml` |
| Bead prefix | `l4` |
| Bead CLI | `br` (NEVER `bd`) |
| Bead sync | `bin/br-sync` (or `br sync --flush-only` + manual `git add .beads/`) |
| Existing L4 epic | `testproject1-4zq` (children to be created in Batch 4) |
| Build | `cargo check --all-targets && bun run build` |
| Test (Rust full) | `cargo test -p daemon --bin daemon && for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" \|\| exit 1; done` |
| Test (TS) | `bun run test` |
| Lint | `cargo clippy --all-targets -- -D warnings && cargo fmt --check && bun run lint` |
| UBS | `ubs $(git diff --name-only --cached)` |
| AgentMail key | `/Users/alinaaitova/Desktop/Testproject1` |

---

## Mandatory Stops Recap

1. **Before Batch 6 step 10:** human reviews kickoff + push, authorizes execution
2. **After parallel execution closes all task beads:** human triggers Batch 12 completion audit
3. **After fix beads close:** human verifies two-user E2E and authorizes archive (15)

---

## Timeline Estimate

| Batch | Approx | Notes |
|-------|--------|------|
| 1 SPEC | 1-3 hours | Validation pass + optimize fresh-eyes |
| 2 TDD | 3-6 hours | TDD generation is large (4 test files but each spans many ID ranges); bootstrap is independent and can parallelize |
| 3 STRATEGY | 1-3 hours | Phase ordering only |
| 4 BEADS | 3-8 hours | 25-50 beads expected (push engine alone is ~6-10 beads) |
| 5 AUDIT | 1-3 hours | Systematic walk |
| 6 KICKOFF | 1-2 hours | Single session output |
| Execution | days-week+ | Multi-agent parallel; depends on team size and provisional-promotion-buffer complexity |
| POST | 1-3 days | Audit → fix beads → re-verify loop |

---

## Notes for Pipemaxxer Re-Runs

- This pipeline targets L4 only. **Do not** mix L5 or L6 work into these handoffs.
- If something in the L4 spec proves impossible during execution, prefer **fix beads + spec amendment** over silent scope reduction. Scope reduction = blocking failure.
- `infra/supabase/` and `apps/web/` are net-new directories — agents must create them, then update root `package.json` workspaces and `bun run {build,test,lint}` scripts to include them. The bootstrap doc owns this wiring.
- The migration-filename mismatch (`sqlite/migrations/0002_sqlite_state_sync.sql` in spec vs `migrations/0012_l4_sync_state.sql` in repo convention) MUST be resolved during step 01 — record the chosen path back into the spec.
