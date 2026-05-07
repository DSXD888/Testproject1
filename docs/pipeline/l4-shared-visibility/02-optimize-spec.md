# L4 Shared Visibility — 02 · Optimize Spec (Fresh-Eyes Audit)

## First Action

**No skill is invoked.** This is a manual fresh-eyes audit by an independent agent (NOT the agent that ran step 01). Open the spec, work through the 10 audit dimensions below, edit the spec in place to fix issues, then run the follow-up pass.

---

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the L4 layer doc — the source-of-truth specification for the Shared Visibility layer of a UK procurement intelligence platform. This is the SECOND pass on the spec, after the structural validation in step 01.

You are reading, analyzing, and IMPROVING an existing document — not writing code.

L4 makes pipeline data visible to a second user (web viewer) and switches semantic validation from L3 dry-run to enforcement. It builds: a SQLite↔Supabase sync engine (push outbox + pull cursors + CAS for operator decisions); 8 Supabase migrations including RLS for 4 roles; a read-only Next.js web portal at `apps/web/`; a provisional-promotion buffer for enforcement-active slices; 4 domain-specific validator overlays; and a 3-corpora calibration harness.

**Authority:** This document is the execution-grade spec. After your audit, an implementation agent should be able to build L4 from this document alone without reopening the research doc.

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/strategy/layers/layer-4-shared-visibility.md   — 797 lines (post-step-01) — READ FULLY
```

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layer-strategy.md                     — cross-layer ordering doc (§6 L4, §12 milestone mapping)
docs/strategy/cross-reference-index.md              — every L4 cross-ref
docs/strategy/layers/layer-3-operational-hardening.md  — what L3 delivers (validation infrastructure, validation_verdicts schema, sync columns on push-owned tables)
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md  — L5 expectations of L4
docs/strategy/layers/layer-6-request-pipeline-automation.md       — L6 expectations of L4 (operator_decision_heads, shared_company_profile, validation enforcement)
docs/strategy/layers/layer-2-single-operator-desktop.md  — desktop side of L4 sync coupling
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/db/sqlite.rs                        — 600 lines, what `open_state_db` does today
crates/daemon/src/db/checkpoints.rs                   — 231 lines, existing checkpoint storage (parallel to where sync.rs will live)
crates/daemon/src/validation/policy.rs                — 959 lines, L3 dry-run policy engine
crates/daemon/src/validation/executor.rs              — 1217 lines, validator skill spawner
crates/daemon/src/validation/manifest_writer.rs       — 54 lines, hybrid manifest writer
crates/daemon/src/errors/manifest.rs                  — 386 lines, error_manifests writer
crates/daemon/src/jobs/runner.rs                      — 1027 lines, single enqueue boundary (L4 must extend this for provisional promotion)
crates/daemon/src/ipc/handlers/decisions.rs           — pending_operator_intents writer (L4 adds CAS flush + conflict event)
crates/daemon/Cargo.toml                              — confirm: NO HTTP client today
migrations/0001_sqlite_state_initial.sql              — 545 lines, L0 schema (no local_outbox)
migrations/0004_l2_pending_operator_intents.sql       — 31 lines, the queue
migrations/0005_l3_operational_hardening.sql          — 119 lines, validation_verdicts richer
packages/ui/                                          — components L4 web portal reuses unchanged
packages/manifests/render-error-manifest.ts           — TS render path L4 web reuses
apps/desktop/src/pages/ContractList.tsx               — desktop's analogous page (target template for apps/web/src/app/contracts/page.tsx)
apps/desktop/src/pages/ContractDetail.tsx             — same, for /contracts/[id]
apps/desktop/src/pages/DaemonStatus.tsx               — same, for /status
skills/validation/base/                               — template the 4 L4 overlays follow
package.json (root)                                   — workspaces + lint rule + bun scripts
AGENTS.md                                             — project rules
```

### Quality Bar (MATCH OR EXCEED)
```
docs/strategy/layers/layer-3-operational-hardening.md         — best validated L-spec in the repo
docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/02-optimize-spec.md  — L3's analogous step
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

If it's in the spec post-step-01, it gets implemented. You may IMPROVE clarity, ADD missing technical detail (file paths, function signatures, observable outcomes), STRENGTHEN ACs from vague to precise. You may NOT remove features, defer to "post-MVP" or "Phase 2+", weaken ACs, or introduce a "stretch goal" tier.

**Watch especially for these L4-specific scope-drift attractors** — features sometimes seen as deferable that are NOT:
- Provisional promotion buffer for enforcement slices (§1.3 + §3.4)
- Durable command ledger `operator_decision_commands` (§1.5 + §2.6 / migration 0007)
- Writer fencing token (`writer_epoch`, §1.6 + §2.6 / migration 0006)
- Hi-water fenced pull (§1.4)
- Equal-version retry semantics (§1.3) — all 4 cases must be defined
- Tombstone retention >=14 days (§3.6)
- Cross-authority provenance per pipeline run (§2.7) — `source_config_revision`, `company_profile_revision`, `operator_decision_revision`
- Redaction tests for ALL 6 summary projections (§2.5)
- Epoch-repair reproject minimum path (§5.2 stub) — the spec EXPLICITLY says "cannot remain a future nice-to-have"
- Per-slice enforcement readiness with all 7 prerequisites (§3.4)
- All 9 mutation families + pairwise challenge sets + cross-model audit sampling (§3.6)

---

## Think Independently — 10 Audit Dimensions

Don't just check for obvious errors. Think critically about these:

### 1. Sync Engine Completeness

The push protocol (§1.3) defines: single-in-flight batch, identical-envelope retries, batch boundaries (never cross `schema_epoch`/`stage_run_id`/`commit_group_id`), `row_version` ordering, `content_hash` for idempotency, `writer_epoch` carry, equal-version retry semantics (4 cases), provisional promotion buffer for enforcement slices.

**Audit:**
- Is each of these a separate L4-FR with at least one explicit AC?
- Does the equal-version retry table cover all 4 cases (no-op success on `content_hash` match, hard conflict on `content_hash` mismatch, normal advance on +1, reject on stale/jump)? Are these labeled testably?
- Does §1.3 say what "single transactional Supabase RPC" means concretely — one Postgres function with a `BEGIN`/`COMMIT`, or a single REST RPC call that wraps a transaction? (Implementation needs this to choose between `tokio-postgres` and Supabase REST.)
- Does the spec address: what happens if the push dispatcher crashes mid-batch? (Identical-envelope retry handles this — but is it called out?)

The pull protocol (§1.4) defines: cursor-based pull, hi-water fence, pull-before-push ordering, pending intent reconciliation (latest-per-process coalescing).

**Audit:**
- Are the 3 pull domains (`shared_source_configs`, `shared_company_profile`, `operator_decision_heads`) all explicitly listed with their cursors?
- Is the "latest-per-process" coalescing tiebreaker defined? (See `migrations/0004_l2_pending_operator_intents.sql` — `superseded_by_intent_id` suggests by `created_at` ascending.)
- Does the hi-water fence algorithm specify whether `change_seq` is per-row monotonic on Supabase, or a domain-level sequence? (Critical for the algorithm to be correct.)
- Is "pull-before-push ordering" applied to startup only, or every sync cycle? (It says "On sync startup" — verify this is enough.)

### 2. CAS Correctness

§1.5 defines CAS for operator decisions with `base_remote_revision` check, `command_id` for idempotency, durable command ledger.

**Audit:**
- Does the spec specify the failure mode for "command_id replayed after a later command landed"? The §4.3 test scenario row says "Command ledger returns stored conflict result (not re-evaluated against current state)" — verify this is also an AC under §1.5.
- Is the command ledger GC retention specified (default 30 days per §1.5)? Is GC behavior testable (e.g., a test that exercises post-GC replay → "command unknown, treat as new"?
- Does the CAS RPC signature `update_operator_decision(process_id, disposition, base_revision, command_id)` match the migration 0005 implementation in §2.6?
- Is the Desktop conflict-resolution UX specified (what fields are shown, what actions does the operator take)?
- Does the spec say: "Direct UPDATE on `operator_decision_heads` is blocked at the RLS layer; only the RPC path is exposed"? (§2.5 implies this; verify it's an explicit AC.)

### 3. Writer Fencing & Lease

§1.6 says writer fencing via `writer_epoch` (incremented on lease acquisition), every push batch carries it, remote rejects stale-epoch batches.

**Audit:**
- Is the lease row schema explicit (org_id, holder_executor_id, lease_expires_at, writer_epoch)?
- Does the spec define lease acquisition behavior (CAS-based on `holder_executor_id` IS NULL or expired)?
- Lease heartbeat is via `viewer_sync_status` updates — is the heartbeat interval specified?
- Edge case: daemon crashes without releasing lease — when does it expire? Is `lease_expires_at` enforced?
- Operational rule "one bound machine per org in MVP" is in §1.6 — is there a mechanism to detect and warn the operator if a second machine attempts a push? (RLS rejection is one path; is the desktop UX for "your daemon was rejected" specified?)

### 4. Schema Epoch Guard

§1.7 says `schema_epoch` on every push batch and in `PRAGMA user_version`. Mismatch pauses sync, local operation continues.

**Audit:**
- Is the spec's claim that `schema_epoch` is in `PRAGMA user_version` consistent with `crates/daemon/src/db/sqlite.rs:open_state_db(path, schema_epoch: i64)`? (The current code threads it as a runtime parameter; L4 elevates it to a persistent column. Is this transition spelled out?)
- §5.2 stub for "Epoch-repair reproject" gives a minimum path. Verify this minimum path is **inside L4 scope, not deferred**:
  - Mark `projection_generation`
  - Republish current row images from local authoritative tables
  - Discard stale-epoch outbox entries only after successful reseed
  Is this concrete enough for a bead author to implement?

### 5. Provisional Promotion Buffer

§1.3 + §3.4 define provisional promotion: worker outputs held in local-only staging until validation completes, then promoted (approved) or quarantined (rejected).

**Audit:**
- Does the spec name a specific staging mechanism (table or artifact directory)? If left ambiguous, agents will diverge. The recommendation in step 01 was: SQLite staging table `provisional_worker_outputs`. Verify this is reflected in the spec.
- Does §3.4 specify the AGENT_SUSPECT_OUTPUT error type (in `crates/daemon/src/errors/manifest.rs` or `validation/executor.rs`)?
- Does the spec say what happens in a slice that has enforcement DISABLED (continues direct-write path)? Yes — §1.3 says "For dry-run slices and non-enforcement slices, the current direct-write path continues unchanged." Verify this is an AC.
- Does the spec specify how an operator override of a quarantined result becomes a normal result? (This is a UX + state-transition gap if not specified.)

### 6. Per-Slice Enforcement Readiness

§3.4 lists 7 prerequisites for enabling enforcement on a slice (worker skill × source × worker behavior version × validator behavior version). Auto-demotion triggers are also listed.

**Audit:**
- Are all 7 prerequisites concrete and measurable? (Wilson-bound reject precision lower bound >= 70%; rule-of-three false-pass upper bound <= 5%; >=30 adjudicated cases; >=60 audited approvals; coverage of dominant failure families; no active demotion trigger; validator overlay version compatibility.)
- Are the 4 demotion triggers each measurable (override rate spike, challenge-bank regression, anomaly-conditioned error rise, worker version moves outside compatible range)?
- Is the slice identifier formally defined? (worker_skill_id, source_id, worker_behavior_version, validator_overlay_version → some `enforcement_readiness` table?)
- Where does the per-slice readiness state live? (Implication: a new SQLite table or projection. The L3 spec might already have a reservation — verify.)

### 7. Web Portal Architecture & Security Boundary

(Note: `packages/ui/freshness-badge.tsx` already exists — L4 EXTENDS it to support a 3-state `syncHealth` prop. The spec must not say "create FreshnessBadge"; verify wording is "extend" or "upgrade".)


§3.1-§3.3 define the 7-file Next.js portal, Broadcast invalidation, and 3-state degraded UX.

**Audit:**
- Are all 7 file paths in §3.1's table consistent with `apps/web/src/app/...` (App Router convention)?
- Is the Broadcast topic scheme (§3.2) page-scoped vs row-scoped clear? The spec says "page-level, not row-level" — verify this is testable.
- §3.1: "viewer-scoped credentials only — the browser must not possess operator or admin mutation rights" is a SECURITY BOUNDARY, not a UI convention. Is this enforced via Supabase auth role/JWT scope? Are RLS roles verified by an automated test (`apps/web/e2e/contracts.spec.ts`)?
- §3.3 thresholds: green <90s, amber 90s-5min, red >5min or daemon offline. Are these computed in `apps/web/src/lib/broadcast.ts` (or a hook), or read from `viewer_sync_status` directly? The spec ambiguity here can cause divergent implementations.
- Does the spec address: how does `apps/web` get added to the workspace? Root `package.json` `workspaces` array + `bun run {build,test,lint}` scripts must be updated. Is this an explicit AC?
- The `packages/ui` lint rule (banning imports of `@tauri-apps/api`, `apps/desktop`, `crates/daemon`, `supabase`, `transport`) MUST hold. Is this called out as a constraint that L4 cannot break?

### 8. RLS Policy Matrix Completeness

§2.5 has the policy matrix table. 4 roles (viewer, operator, admin, daemon) × 7 table categories.

**Audit:**
- Is every cell of the matrix non-empty (— or specific permission)?
- Are operator decisions handled correctly: viewer sees `operator_decision_summaries` (push-owned projection); operator can SELECT and UPDATE via CAS RPC on `operator_decision_heads` directly; viewer cannot see `operator_decision_heads`. Verify this is the matrix state.
- Is the daemon-pull tombstone visibility fix (§2.5: "Daemon role SELECT policy does NOT filter on `deleted_at IS NULL` for remote-owned tables") an explicit RLS policy clause?
- Are redaction tests required for all 6 summary projections? (`workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status`, `executors`, `operator_decision_summaries`.) Yes — §4.1 row 18 says so. Is this a single test file (`redaction_boundary.rs` per §4.2) and does each projection get a separate test inside?

### 9. Calibration Harness Specificity

§3.6 requires 3 corpora (clean real 25-30, weird-but-valid 15-20, mutated bad 30-50), 9 mutation families, pairwise challenge sets, cross-model audit sampling, override feedback loop.

**Audit:**
- Is each of the 9 mutation families described with "what it tests"? (Yes per the table.) Are they referenced as L4-FR IDs?
- Is the calibration corpus location specified? (e.g., `tests/fixtures/calibration/{slice_id}/{clean,weird,mutated}/`?)
- Is the challenge-bank update process specified? (§3.6 "Override feedback loop" says every operator override becomes labeled calibration case + incident-cluster member + candidate mutation template. Is this automated or manual? Where does the feedback loop live in code?)
- Are the minimum calibration thresholds testable? (Wilson-bound math, rule-of-three math.) Does the spec specify these are computed via a util module and asserted in tests?

### 10. Cross-Layer Continuity

L4 cross-domain notes at the bottom of §5.5: L0 (`commit_group_id`/`writer_epoch` shape), L2 (degraded-mode UX, `pending_operator_intents`/`operator_decision_heads` transition), L6 (remote config staleness, upstream trust gate).

**Audit:**
- Does L2's spec already declare the 3-state sync indicator support on FreshnessBadge? (If L2 says "L2 ships freshness-2-state, L4 upgrades to 3-state" — is that captured?)
- Does L6's spec depend on `operator_decision_heads` and `shared_company_profile` existing in Supabase? Verify the dependency graph in `layer-strategy.md` lists L4 as a hard prerequisite for L6.
- Does L5 depend on `apps/web` existing? (L5 enrichments must be visible.) Verify this is explicit.
- Does ADR-002 (authority-matrix database, called out in §5.5) get fully realized in L4 or partially? Spec says "Sync/remote-projection half fully operational" — verify "fully operational" is supported by the §4.1 DoD criteria.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

This is the single most important audit step. The original spec was deeply iterated on; your job is to verify the post-step-01 spec preserves 100% of the feature surface and adds no scope-reducing language.

### Red-Flag Language Scan

Search the ENTIRE spec for these phrases. **Every match must be inspected.** Some matches in §5 (Open Questions, Stubs, Forward-Compat, Dependencies) are LEGITIMATE — they describe future-LAYER work or known limitations. Matches inside §1-§4 describing in-scope L4 features are BLOCKING failures.

Phrases:
- `post-MVP` / `post mvp`
- `Phase 2+` / `phase 2 plus`
- `stretch goal` / `stretch-goal`
- `nice to have` / `nice-to-have`
- `deferred` / `defer to`
- `out of scope` / `out-of-scope`
- `future iteration`
- `simplified for initial implementation`
- `simplified for now`
- `can be added later` / `to be added later`
- `eventually`
- `someday`
- `for now` (when describing an L4 feature, not a future layer)
- `we'll do this in L5/L6` (or any phrasing that pushes L4 features to a later layer)

### Coverage Verification

Enumerate every requirement:
- §1.3 (push protocol) — count items
- §1.4 (pull protocol) — count items
- §1.5 (CAS) — count items
- §1.6 (lease) — count items
- §1.7 (epoch guard) — count items
- §1.8 (degraded mode) — count items, including 3 sync states + 4-row staleness class table
- §1.9 (invariants) — must be 14
- §2.1 (push-owned projections) — must be 20 tables
- §2.2 (summary tables) — must be 6 tables
- §2.3 (remote-owned) — must be 3 tables
- §2.4 (junction) — must be 3 tables
- §2.5 (RLS) — count rows × roles
- §2.6 (Supabase migrations) — must be 8
- §2.7 (cross-authority crossings) — must be 4
- §3.1 (web files) — must be 7
- §3.2 (Broadcast topics × pages × payload types)
- §3.3 (3 sync states)
- §3.4 (enforcement transitions, per-slice readiness, demotion triggers)
- §3.5 (must be 4 overlays with concrete domain checks)
- §3.6 (3 corpora, 9 mutations, pairwise, cross-model, override loop)
- §3.7 (`viewer_sync_status` DDL)
- §4.1 (must be 18 DoD)
- §4.2 (must be 4 test files)
- §4.3 (must be 17 scenarios)
- §4.4 (must be 8 risks)
- §5 (open questions, stubs, forward-compat, dependencies, ADRs)

If any count differs from the original spec without an explicit additive justification, it's a BLOCKING failure (silent scope cut).

### Drift Checklist

- [ ] Every §4.1 DoD row is represented somewhere in §1-§3 with concrete ACs
- [ ] Every §1.9 invariant has a corresponding test in §4.2
- [ ] No §1-§4 content was silently dropped or weakened
- [ ] No assumptions made that aren't in the spec
- [ ] **No "post-MVP" / "Phase 2+" / "stretch goal" / "nice to have" language inside §1-§4**
- [ ] §5.2 stubs each have explicit "what's in L4" vs "what's future" boundaries
- [ ] Later sections (§3.4-§3.7) have SAME depth as early sections (§1.3-§1.6)
- [ ] Cross-domain notes are accurate (L0, L2, L6 references)
- [ ] All 4 validator overlays in §3.5 have concrete domain checks (not "TBD")
- [ ] All 9 mutation families in §3.6 have explicit "what it tests" semantics
- [ ] Provisional promotion buffer mechanism is named (table or directory) — not ambiguous
- [ ] Equal-version retry has all 4 cases defined
- [ ] Migration filename mismatch resolved consistently throughout (no mixing of `sqlite/migrations/0002_*` and `migrations/0012_*`)
- [ ] `local_outbox` ownership clarified (L4 owns DDL; L0 reserves column shape)
- [ ] HTTP client dependency called out in §5.4 prerequisites
- [ ] `apps/web` workspace integration called out as L4 work
- [ ] Bead CLI references (if added) say `br`, not `bd`

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

ULTRATHINK.

Re-read the fixes you just made. Did they introduce new issues?

1. **Did fixing the migration-filename mismatch** in §1.10 / §2.6 / §5.4 introduce inconsistency anywhere else? Grep the full file for `0002`, `0012`, `sqlite/migrations`, `migrations/`. Every reference should point to the same path.

2. **Did adding ACs to §1.5 CAS** preserve §4.3 test scenarios? Re-read §4.3 row "CAS command retried after later command landed" — verify the test scenario maps to a §1.5 AC you added.

3. **Open `crates/daemon/src/validation/policy.rs`.** Does the L4 spec correctly distinguish what stays in `policy.rs` (slice readiness, sampling decisions) from what is new in L4 (per-slice enforcement gating, Wilson-bound thresholds, demotion triggers)? Verify §3.4 doesn't tell agents to "rewrite" or "replace" the policy engine — it should "extend" it. (L3 already shipped 959 lines of policy.rs; L4 extends.)

4. **Open `migrations/0001_sqlite_state_initial.sql`** lines around 20 (the early table definitions). Confirm none of the 22 tables there have `commit_group_id` or `writer_epoch` columns — meaning L4's SQLite migration must add those columns to push-owned tables (or to `local_outbox` only?). Re-read §1.3 outbox entry structure: `commit_group_id` is on `local_outbox` (an L4 table). So the L0 cross-domain note at the bottom of §5.5 is about `local_outbox`'s columns, not about pre-existing tables. Verify the spec wording reflects this.

5. **Open `package.json` (root) workspaces array.** Currently the workspaces include `packages/*`, `apps/desktop`. After L4, it must include `apps/web`. Also `bun run build` script currently runs `bunx tsc -p apps/desktop/tsconfig.json --noEmit` — after L4 it must also run a `tsc -p apps/web/tsconfig.json --noEmit` or `next build` step. Is this in the spec? It's a Bootstrap-doc concern but the spec should at least mention the integration.

6. **Verify the §3.5 validator overlays section is concrete enough.** Each overlay should specify (a) directory path under `skills/validation/`, (b) the 6 standard files (per `skills/validation/base/`), (c) at least 4 domain-specific checks. Open `skills/validation/base/SKILL.md` to confirm the template. If any overlay lacks 4 concrete checks, flag it.

7. **Walk every §4.1 DoD row.** For each, ask: "Does the spec body (§1-§3) define this requirement with at least one observable AC? Does §4.2 list a test file that exercises it? Is there a clear path from `cargo test` / `bun run test:e2e` to verifying this DoD row?" If any of those three is missing for any DoD row, it's a gap.

8. **Re-scan for scope-drift language one final time** — same list as the Red-Flag scan. Some hits in §5 are legitimate; hits anywhere in §1-§4 that describe in-scope L4 features are BLOCKING.

9. **Final count consistency check.** Re-count: 18 DoD, 14 invariants, 4 test files, 8 Supabase migrations, 4 overlays, 7 web files, 20 push-owned tables, 6 summary tables, 3 remote-owned tables, 3 junction tables, 4 RLS roles, 4 cross-authority classifications, 9 mutation families, 3 calibration corpora, 17 test scenarios, 8 risks, 3 open questions, 4 stubs, 3 forward-compat provisions. If any count drifted, investigate.
