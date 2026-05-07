# L4 Shared Visibility — 04 · Optimize TDD + Bootstrap (Fresh-Eyes Audit)

## First Action

**No skill is invoked.** This is a manual fresh-eyes audit by an independent agent (NOT the agent that ran step 03). Open the TDD and Bootstrap, work through the 12 audit dimensions below, edit both docs in place to fix issues, then run the follow-up pass.

---

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the L4 TDD and Bootstrap docs created in step 03. The TDD becomes SOT #2 — beads will reference test IDs (`L4-T-...`) directly. The Bootstrap becomes the environment prerequisite contract — agents trust it to know how to provision Supabase, add the daemon HTTP client, wire `apps/web`, seed calibration corpora.

**You are reading and improving existing documents — not writing code.**

L4 makes pipeline data visible to a second user (web viewer) and switches semantic validation from L3 dry-run to enforcement. The TDD must verify every spec L4-FR with concrete tests; the Bootstrap must enable any fresh agent to set up the environment without reading the spec.

---

## What To Read (Do This First)

### The Documents to Audit (TARGETS)
```
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md         — created in step 03 — READ FULLY
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md   — created in step 03 — READ FULLY
```

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-4-shared-visibility.md   — Spec (SOT #1) post-step-02
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/db/sqlite.rs                      — 600 lines, what `open_state_db` does today
crates/daemon/src/db/checkpoints.rs                 — 231 lines, parallel module pattern
crates/daemon/src/validation/policy.rs              — 959 lines, L3 dry-run policy engine
crates/daemon/src/validation/executor.rs            — 1217 lines, validator skill spawner
crates/daemon/src/validation/manifest_writer.rs     — 54 lines, hybrid manifest writer
crates/daemon/src/errors/manifest.rs                — 386 lines, error_manifests writer
crates/daemon/src/jobs/runner.rs                    — 1027 lines, job runner (provisional promotion path will extend this)
crates/daemon/src/ipc/handlers/decisions.rs         — pending_operator_intents writer
crates/daemon/Cargo.toml                            — confirm: NO http client today
crates/daemon/tests/                                — list every existing integration test file (38 files cumulative pre-L4)
migrations/0001_sqlite_state_initial.sql            — L0 schema (no local_outbox)
migrations/0004_l2_pending_operator_intents.sql     — L2 pending intents
migrations/0005_l3_operational_hardening.sql        — L3 validation_verdicts
packages/ui/                                        — components apps/web reuses
apps/desktop/e2e/playwright.config.ts               — Playwright config L4 web e2e copies
skills/validation/base/                             — template the 4 overlays follow
package.json (root)                                 — workspaces, lint rule, scripts
AGENTS.md                                           — Stage Validation Test Gate
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md         — 557 lines, 11 sections
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md   — 729 lines, 16 sections
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

If a spec feature has no test, that's a coverage gap and a scope-drift signal. You may IMPROVE test rigor; you may NOT silently drop coverage. Every L4-FR must trace to ≥1 L4-T. Every §1.9 invariant must trace to a dedicated invariant test. Every §4.3 scenario must have a corresponding test.

---

## Think Independently — 13 Audit Dimensions

### 1. Coverage Matrix Completeness
- Walk every L4-FR-X.Y.Z in the spec. For each, verify TDD §9 lists ≥1 L4-T test ID.
- Walk every of the 18 DoD criteria. For each, verify ≥1 test maps.
- Walk every of the 14 invariants. For each, verify a dedicated invariant test exists.
- Walk every of the 17 test scenarios in §4.3. For each, verify a TDD test exists with matching semantics.
- Walk every of the 8 risks in §4.4. For each, verify the related test catches the risk's failure mode.

### 2. Push Engine Test Depth
The spec defines 11 push-engine constitutional invariants in §1.9. Each must be tested in `sqlite_supabase_sync.rs`:
- Push batch atomicity (single transactional RPC; partial-apply forbidden)
- Outbox + data in same SQLite transaction (transaction rollback test)
- Active-writer lease with fencing token (stale-epoch reject test, 2-daemon simulation)
- `schema_epoch` on every batch (epoch mismatch pause test)
- Batch boundaries never cross `stage_run_id` (dispatcher reject test)
- Batch boundaries never split `commit_group_id` (dispatcher boundary test)
- Single-in-flight batch (concurrency test — 2 batches queued, only 1 in-flight)
- Equal-version retry semantics — **all 4 cases**:
  - Incoming `row_version` = remote, `content_hash` matches → no-op success
  - Incoming `row_version` = remote, `content_hash` differs → hard conflict
  - Incoming `row_version` = remote + 1 → apply
  - Anything else → reject

If any of these is missing or has only one test for all 4 equal-version cases lumped together, this is BLOCKING.

### 3. Pull Engine Test Depth
- Hi-water fenced pull: paginated pull with concurrent remote writes between fence and cursor advance — verify no skip/duplicate
- Pull-before-push ordering on startup
- 3 pull domains independently cursored: `shared_source_configs`, `shared_company_profile`, `operator_decision_heads`
- Tombstone visibility: daemon role pull SELECT does NOT filter `deleted_at IS NULL` for remote-owned tables (RLS test)
- Pending intent reconciliation: latest-per-process coalescing tests with multiple-unsuperseded-intents fixture

### 4. CAS + Command Ledger Test Depth
- CAS conflict: 2 concurrent operators, second fails with `base_remote_revision` mismatch
- Command idempotency: replay command after later command landed → returns stored conflict result, NOT re-evaluated
- Command ledger GC: post-30-day GC, replay command → "unknown command, treat as new"
- Direct UPDATE blocked at RLS: operator role attempting raw UPDATE on `operator_decision_heads` — fails

### 5. Provisional Promotion Buffer Test Depth
The spec §1.3 + §3.4 names this as constitutional for enforcement slices. Tests must cover:
- Enforcement-active slice + validator approves → outbox entry created, push proceeds
- Enforcement-active slice + validator rejects → NO outbox entry, error manifest created (`AGENT_SUSPECT_OUTPUT`), stage head NOT advanced
- Enforcement-inactive (dry-run) slice + validator rejects → outbox entry STILL created (current path), verdict logged, stage head advances normally
- Operator override of quarantined result → state transition path (the spec doesn't fully specify — flag if missing)

### 6. RLS Policy Test Depth
4 roles × 7 table categories = 28 policy assertions. Tests must verify:
- viewer: SELECT on push-owned projections + summaries; NO access to remote-owned config; NO access to sync infra; NO access to CAS infra
- operator: SELECT + UPDATE on `operator_decision_heads` via CAS RPC only (direct UPDATE blocked); SELECT on push-owned projections; NO access to other config
- admin: full CRUD on remote-owned config
- daemon: INSERT/UPDATE on push-owned via `apply_push_batch` RPC; SELECT on remote-owned (including tombstones); SELECT on push-owned

If any role × category cell lacks a test, that's a gap.

### 7. Redaction Test Depth
6 summary projections × ≥1 redaction scenario each:
- `workflow_job_summaries` — no full job context
- `error_manifest_summaries` — no full context_json or markdown
- `validation_verdict_summaries` — no full reasoning
- `viewer_sync_status` — only org-scoped sync state
- `executors` — daemon heartbeat only
- `operator_decision_summaries` — no operator identity, no command history

If a single test conflates multiple projections, split.

### 8. Web E2E Test Depth (`apps/web/e2e/contracts.spec.ts`)
- Navigate to /contracts, verify reads succeed under viewer credentials
- Verify NO decision controls visible (operator-side UI absent)
- Verify keyset pagination works
- Verify Broadcast invalidation hint received within 3s of remote write
- Verify "N new/updated" pill appears (no destructive reorder)
- Verify /contracts/[id] minor refresh path
- Verify /contracts/[id] major refresh chip path (manifest changed)
- Verify /status shows sync freshness (green/amber/red)
- **Verify viewer credentials FAIL on remote-owned tables** — direct Supabase client query to `shared_company_profile` from the test should fail with 403/RLS error
- Verify offline-mode UX (kill Broadcast → red badge; try a fetch → graceful degradation)

### 9. Calibration Harness Test Depth (§3.6)
- Per skill family: 25-30 clean real, 15-20 weird-but-valid, 30-50 mutated bad
- All 9 mutation families exercised:
  1. Wrong source/domain in URL chain
  2. Wrong page kind (search vs category vs detail)
  3. Locale/currency swap (claims GBP, evidence EUR)
  4. Pack/unit distortion (per-unit vs per-box vs per-pallet)
  5. Accessory-vs-primary confusion
  6. Selective warning omission (says 2, daemon shows 5)
  7. Manifest/result count mismatch
  8. Fabricated evidence references
  9. Cross-segment duplicates/gaps (continuation chains)
- Pairwise challenge sets: A=clean, B=corrupted; validator must rank B less trustworthy
- Wilson-bound precision lower bound >= 70% computation tested
- Rule-of-three false-pass upper bound <= 5% computation tested
- Cross-model audit sampling: periodic alternate-model run; correlation alerts
- Override feedback loop: operator override → labeled calibration case + incident-cluster member + candidate mutation template

### 10. Bootstrap Environment Prerequisites
- Supabase project provisioning steps (CLI version, init, link, migrate)
- Env vars: `SUPABASE_URL`, `SUPABASE_ANON_KEY`, `SUPABASE_SERVICE_ROLE_KEY`, `SUPABASE_PROJECT_ID`
- Daemon HTTP client: exact `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }` line in `crates/daemon/Cargo.toml`
- `apps/web` workspace wiring: root `package.json` workspaces array edit; root `bun run build` script extension to include web build; root `bun run test` includes web; root `bun run lint` includes web
- `apps/web/tsconfig.json` skeleton
- `apps/web/playwright.config.ts` skeleton (mirror `apps/desktop/e2e/playwright.config.ts`)
- `apps/web/package.json` with Next.js + Supabase client + Playwright dev deps
- `infra/supabase/config.toml` skeleton + initial 8-migration scaffold
- Calibration corpus directory: `tests/fixtures/calibration/{slice_id}/{clean,weird,mutated}/`
- Validator overlay scaffolds: `skills/validation/{procurement-ingestion,supplier-enrichment,document-acquisition,product-extraction}/` each with the 6-file template

### 11. Stage Validation Test Gate (Critical AGENTS.md Rule)
Bootstrap §7 Static CI Gates (or equivalent) must include the EXACT command set from AGENTS.md:
```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
bun run test
(cd apps/web && bun run test:e2e)
```
If absent or weakened (e.g., only `cargo test -p daemon --bin daemon`), this is BLOCKING — that command alone silently excludes the integration test files.

### 12. Enforcement Runtime Mechanics (spec ambiguity that the TDD must resolve concretely)

The spec describes the WHAT of per-slice enforcement readiness, demotion triggers, override feedback loop, and cross-model audit sampling — but leaves the runtime MECHANICS implicit. The TDD must pin them down with concrete tests so bead authors implement them consistently:

**Demotion trigger mechanism — WHEN does the readiness check fire?**
- Spec §3.4 says "Demotion is automatic and immediate" but doesn't say what fires the check
- Options: (a) periodic sweep (e.g., every 5 minutes); (b) event-driven (fired after every verdict write); (c) on-demand at policy-engine call sites
- TDD must pick one and add a test. Recommendation: (b) event-driven — after every `validation_verdicts` insert/update, the policy engine recomputes readiness for the affected slice. Test: `L4-T-sqlite_supabase_sync-{N}` — write 30 verdicts that cross the demotion threshold; verify `enforcement_readiness.enforcement_state` flips to `demoted` synchronously within the same transaction.

**Override feedback loop location — WHERE in code does it run?**
- Spec §3.6 says "every operator override becomes (1) labeled calibration case, (2) incident-cluster member, (3) candidate mutation template"
- Where does this fan-out fire? Options: (a) in `crates/daemon/src/ipc/handlers/decisions.rs` where override is captured; (b) in a dedicated `crates/daemon/src/validation/feedback.rs` module; (c) in the `validation/executor.rs` post-verdict hook
- TDD must pick one. Recommendation: (b) — separate module, called from `decisions.rs` IPC handler when override is detected. Test: `L4-T-sqlite_supabase_sync-{N}` — simulate operator override; verify (1) calibration case row written, (2) incident-cluster row written/updated, (3) mutation template entry queued.

**Cross-model audit sampling cadence — WHEN does it run?**
- Spec §3.6 says "Periodically run a sample through a different model family"
- Cadence not specified. Options: (a) cron entry (e.g., daily); (b) every Nth verdict; (c) manual trigger only
- TDD must pick one and define the cron entry or trigger condition. Recommendation: (a) daily cron via the existing `crates/daemon/src/scheduler/cron.rs` infrastructure. Add a cron entry to `Bootstrap §11`. Test: a fixture-driven test that simulates the cron firing and verifies the cross-model run is dispatched.

**Wilson-bound + rule-of-three math — WHERE does the implementation live?**
- Statistical math should be in a util module (likely `crates/daemon/src/validation/calibration.rs` or `validation/stats.rs`)
- TDD must verify the math is correct (compare against known Wilson-bound calculator) and that the policy engine consumes it correctly
- Test: `L4-T-sqlite_supabase_sync-{N}` — input: 26 correct out of 30 adjudicated, expected Wilson-95% lower bound = ~70.4%; assert the readiness check passes the >=70% threshold

If any of these 4 mechanism questions is left ambiguous in the TDD, bead authors will diverge during implementation. The TDD is the binding contract that resolves spec ambiguity.

### 13. Cross-Layer Continuity
- TDD references existing L0/L1/L2/L3 test files where appropriate
- Bootstrap acknowledges `crates/daemon/tests/` already has 38 files; L4 adds 4 → cumulative 52
- Bootstrap acknowledges existing `apps/desktop/e2e/` and treats `apps/web/e2e/` as a parallel structure
- TDD acknowledges existing L3 `validation/policy.rs` tests are not broken by L4 enforcement-readiness extension

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
Search the TDD and Bootstrap for:
- `post-MVP` / `post mvp`
- `Phase 2+` / `phase 2 plus`
- `stretch goal` / `stretch-goal`
- `nice to have` / `nice-to-have`
- `deferred` / `defer to`
- `out of scope` / `out-of-scope`
- `future iteration`
- `simplified for initial implementation` / `simplified for now`
- `can be added later` / `to be added later`
- `eventually`
- `someday`
- `for now`
- `we'll test this in L5/L6` (or any test phasing that pushes L4 coverage to a later layer)

Hits in load-bearing positions = BLOCKING failure.

### Coverage Verification Checklist

- [ ] Every L4-FR-X.Y.Z has ≥1 L4-T in §9 Coverage Matrix
- [ ] Every of the 18 DoD criteria has ≥1 test
- [ ] Every of the 14 invariants has a dedicated invariant test
- [ ] Every of the 17 test scenarios has a corresponding test
- [ ] Every of the 8 risks has a regression test
- [ ] All 11 push-engine invariants tested in `sqlite_supabase_sync.rs`
- [ ] All 4 equal-version retry cases tested separately
- [ ] All 4 RLS roles × 7 categories = 28 policy assertions covered
- [ ] All 6 summary projections have redaction tests
- [ ] All 4 validator overlays have canary + golden tests
- [ ] All 9 mutation families exercised in calibration harness tests
- [ ] Per-slice enforcement readiness tests cover all 7 prerequisites + all 4 demotion triggers
- [ ] Bootstrap §3 covers Supabase env vars + daemon `reqwest` add + `apps/web` workspace wiring + `infra/supabase/` scaffold + calibration corpus directory
- [ ] Bootstrap §7 has the full Stage Validation Test Gate command set
- [ ] No "post-MVP" / "Phase 2+" / "stretch goal" / "nice to have" language anywhere
- [ ] Test ID format `L4-T-{file_stem}-{NNN}` consistent throughout
- [ ] No file path references the wrong migration filename (must be `migrations/0012_l4_sync_state.sql` consistently)

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

ULTRATHINK.

Re-read the fixes you just made. Did they introduce new issues?

1. **Open `crates/daemon/tests/`** and list every test file. Walk through your TDD §6 Planned File Inventory — confirm the 4 new files don't collide with existing names. (Existing examples include things like `pty_*.rs`, `validation_*.rs`, `manifest_*.rs`, `runner_*.rs` etc. The L4 file names — `sqlite_schema.rs`, `sqlite_supabase_sync.rs`, `redaction_boundary.rs` — should be fresh additions.)

2. **Open `apps/desktop/e2e/playwright.config.ts`.** This is the structural template for `apps/web/e2e/playwright.config.ts`. Verify your Bootstrap §3 prerequisites cover this file. Verify your TDD describes Playwright tests using the same project conventions (browsers tested, test directory, web server config).

3. **Open `crates/daemon/Cargo.toml`.** It currently has `tokio`, `rusqlite`, `serde*`, `anyhow`, `thiserror`, `tracing`, `chrono`, `clap`, `cron`, `sha2`, `uuid`, `zstd`, `portable-pty`, `nix`, `sysinfo`, `fs4`, `async-trait`. NO `reqwest`, NO `tokio-postgres`. Verify Bootstrap §5 specifies the exact line(s) to add. If you proposed `tokio-postgres` instead of `reqwest`, justify in the Bootstrap.

4. **Open `package.json` (root).** The current `workspaces` array and the `lint` script (which includes the `rg` ban on `@tauri-apps/api`/`apps/desktop`/`crates/daemon`/`supabase`/`transport` imports from `packages/ui`) are critical. Verify your Bootstrap §5 doesn't break these — `apps/web` is added to workspaces but the lint rule continues to enforce `packages/ui` purity.

5. **Walk every of the 9 mutation families.** For each, verify your TDD §8 Detailed Test Catalog has at least one test ID with concrete assertion semantics ("validator catches X by raising verdict Y because evidence Z conflicts with claim W"). Generic "tests mutation X" is insufficient.

6. **Walk the per-slice enforcement readiness tests.** Verify all 7 prerequisites are tested: validator overlay version compatibility; ≥30 adjudicated cases; ≥60 audited approvals; Wilson-bound precision ≥70%; rule-of-three false-pass ≤5%; coverage of dominant failure families; no active demotion trigger. And all 4 demotion triggers: override rate spike; challenge-bank regression; anomaly-conditioned validator error rise; worker behavior version moves outside compatible range.

7. **Open `migrations/0001_sqlite_state_initial.sql`.** Confirm `local_outbox` is NOT there. Your TDD must specify a `sqlite_schema.rs` test that verifies `local_outbox` is created BY the L4 migration (not assumed pre-existing). Check this is a specific test, not just "migration applies cleanly".

8. **Re-scan the TDD + Bootstrap one final time** for: `post-MVP`, `Phase 2+`, `nice to have`, `deferred`, `stretch goal`, `simplified for now`, `can be added later`, `eventually`, `for now`. Any hits in load-bearing positions = BLOCKING.

9. **Final count consistency check**: 18 DoD ↔ ≥18 tests; 14 invariants ↔ ≥14 invariant tests; 17 scenarios ↔ ≥17 tests; 6 projections ↔ ≥6 redaction tests; 9 mutations ↔ ≥9 mutation tests; 4 overlays ↔ 4 canary+golden test sets; 4 RLS roles × 7 categories ↔ ≥28 RLS assertions. If any count is suspiciously low, investigate.
