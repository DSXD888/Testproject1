# L6 Request Pipeline Automation — Step 03: Create TDD + Bootstrap

## First Action

Invoke the `/spec-to-tdd` skill.

---

## Your Mission

Produce the **L6 Test-Driven-Development spec** (SOT #2) and the **L6 Bootstrap doc**
(environment prerequisites + validation script + local quality gate sequence).

The TDD enumerates every test that, when green, proves the spec is implemented. The
bootstrap is the implementation agent's pre-flight checklist.

**Quality bar:** L5 TDD (873 lines, 5 package test files × ~57 functions × 98
`L5-T-*` assertion IDs) and L4 TDD (623 lines, 147 L4-FR-* requirements, 158 checks,
4 daemon test files, 8 static CI gates). L6 has 3 planned test files (cumulative
target = 60 per spec §2.2) — but the TDD authors decide the precise file split, test
counts, and IDs.

---

## Source Documents (Read In This Order)

### Primary Input (CREATE FROM THIS)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md           # validated by step 01, with L6-FR-X.Y.Z IDs and §1.0 recorded decisions
```

### Source of Truth Context
```
docs/pipeline/l6-request-pipeline-automation/manifest.yaml            # recorded_decisions + spec_callouts + existing_files + new_modules_ts
```

### Codebase (STUDY FOR PATTERNS)
```
# Existing daemon infrastructure
crates/daemon/src/errors/manifest.rs:19-114                           # ErrorCode enum — add 6 L6 variants
crates/daemon/src/validation/policy.rs:17-25                          # ValidationPolicyMode (Always/Sample/Trigger/Skip)
crates/daemon/src/validation/executor.rs                              # Validator job dispatch + verdict persistence
crates/daemon/src/validation/manifest_writer.rs:23-54                 # write_hybrid_run_manifest()
crates/daemon/src/db/sync.rs:24-50                                    # PUSH_OWNED_RESEED_TABLES (bid_workflow_states at line 39)
crates/daemon/src/main.rs:1513+                                       # **Job-kind dispatch lives HERE in main.rs, NOT in jobs/runner.rs** — see existing "extract_document_structure" + "extract_document_semantics" arms at lines 1513-1517; handler functions at lines 1529 + 1836
crates/daemon/src/jobs/runner.rs                                      # Shared job-runner types (ClaimedJob, EnqueueJobRequest, JobLeaseHeartbeat, SkillTelemetryWindow). NOT where the kind-dispatch lives.
crates/daemon/src/ipc/handlers/jobs.rs:45,95                          # trigger_ingestion at line 45 + trigger_enrichment at line 95 — exact reference patterns for L6's new trigger_bid_workflow handler
crates/daemon/src/agents/adapter.rs:104-108                           # InteractionProfile (PtyRequired/ContractRequired/ContractPreferred)
crates/daemon/src/pty/session.rs                                      # PTY session lifecycle
crates/daemon/src/agents/artifacts.rs                                 # RunManifestFrontmatter, DaemonEvidenceAppendix, screenshot paths
crates/daemon/src/main.rs:5256-5260,5436,5498,5510-5527,5537-5542,5781-5787   # L5 readiness checks the L6 gate wraps

# Existing skills
skills/auth-signin/procontract/SKILL.md                               # delegated auth pattern
skills/procurement-ingestion/procontract-register-interest/SKILL.md   # REFERENCE PATTERN for L6 bid skill
skills/validation/overlay-supplier-enrichment/SKILL.md                # frontmatter shape for L6 validator overlay

# Existing test infrastructure
crates/daemon/tests/support/db.rs (577 lines total)                   # TestStateDb + L4TestStateDb helpers — agents use TestStateDb::new() pattern
crates/daemon/tests/skill_canary_harness.rs:255                       # canary test pattern — extend with e2e_can_010+
crates/daemon/tests/skill_golden_journeys.rs:614                      # promotion lifecycle pattern — extend with l3_t_skill_golden_journeys_009+
crates/daemon/tests/redaction_boundary.rs                             # PII redaction tests — extend for BidScopePayload + company profile fields
crates/daemon/tests/sqlite_schema.rs (1861 lines total)               # schema verification pattern — L6 extends if column-list assertions needed

# Existing schema
migrations/0001_sqlite_state_initial.sql:437-451                      # bid_workflow_states (no CHECK — Rust enforces)
migrations/0001_sqlite_state_initial.sql:272                          # supplier_quotes.price_signal_role (quote-role gate)
migrations/0001_sqlite_state_initial.sql:322-394                      # landed_cost_estimates (L5 blockers + classification_fresh_until)
migrations/0003_sqlite_runtime.sql:49-65                              # error_manifests table
infra/supabase/migrations/0002_remote_owned_tables.sql:5-19           # shared_company_profile (remote-owned)
infra/supabase/migrations/0002_remote_owned_tables.sql:38-100         # operator_decision_heads (remote-owned)

# Test fixture conventions
tests/fixtures/calibration/customs-classification/examination-gloves/  # per-category fixture pattern from L5
tests/fixtures/calibration/customs-classification/{fasteners,packaging-cartons}/   # other L5 categories
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md       # 873 lines — best recent TDD
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md                                     # 623 lines, 147 requirements
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md # 729+ lines — bootstrap reference
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**Every L6-FR-X.Y.Z must have at least one test ID.** No requirement may go untested.

You may improve HOW tests verify a feature (better assertions, better fixtures). You
may NOT drop tests for any requirement under labels like "post-MVP" / "deferred" /
"covered by manual smoke." The TDD is the WHAT — the implementation will follow.

The 3 test files named in spec §2.2 are MINIMUMS, not maximums. If your test
catalog needs additional files (e.g., separate state-machine vs ipc-handler test
files, separate validator-overlay tests), add them. Spec §2.2 says "cumulative 60" —
this is the cumulative project-wide test file count INCLUDING the 3 new L6 files
plus L0-L5's 57. Verify by counting `crates/daemon/tests/*.rs` (Rust integration) +
`packages/**/*.test.ts` (TypeScript unit/integration).

---

## Key Context From Codebase Research

- **`bid_workflow_states` table fully exists** with 7 lifecycle values to be enforced at the Rust layer. The TDD must specify state-transition tests + the Rust enum validation tests (no SQL CHECK).
- **3-6 new ErrorCode variants** (final count is a TDD decision — STRAWMAN list in `manifest.yaml#new_modules_rust`):
  - **Likely new** (no existing code covers): `BID_SUBMISSION_FAILED`, `BID_DUPLICATE_DETECTED`, `BID_SIDE_EFFECT_CAP_EXCEEDED`, `BID_READINESS_GATE_FAILED`
  - **Likely redundant** (existing code may cover — TDD step 03 evaluates): `BID_PORTAL_CHANGED` (vs existing `PAGE_SHAPE_DRIFT`); `BID_PORTAL_AUTH_EXPIRED` (vs existing `AUTH_STATE_STALE` — note auth refresh is daemon-orchestrated retry, not bid-skill-direct call, so a new code may not add value)
  - **Action for TDD author**: open `crates/daemon/src/errors/manifest.rs:19-114`, list every existing variant, grep where each is raised, then decide which L6 failure modes need NEW codes vs which can reuse existing codes with portal-specific `error_context` JSON
  - Each NEW variant needs: enum extension test + `as_str()` round-trip test + clippy-exhaustive-match-propagation test. Per L5 precedent, variant additions propagate to 5-15 sites; each propagation site needs a test (or at least a coverage point).
- **L5→L6 readiness gate is constitutional** — its 5 blockers + quote-role + company-profile + upstream-trust conditions need DEDICATED tests in `bid-readiness-gate.test.ts`, NOT shared with state-machine tests.
- **Side-effect class progression** (READ_ONLY → REMOTE_DRAFT → ... → FUNDS_COMMITTED) must be monotonic and ≤ `sideEffectCap`. Tests must verify the cap-exceeded guard raises `BID_SIDE_EFFECT_CAP_EXCEEDED`.
- **Fail-closed validation** for `bid_submitted`: validator rejection OR outage → blocked. The TDD must cover BOTH paths separately.
- **CDP browser attach pattern**: bid-portal-procontract attaches to operator's persistent Chrome at `/Users/alinaaitova/.chrome-automation:9222`. Tests live-mode is constrained — see `skills/procurement-ingestion/procontract-register-interest/SKILL.md` for the CDP discipline TDD must mirror.
- **No HTTP mocks** — bid portal tests use either real CDP (smoke tier) or cassette replay (fast tier). L5 established this pattern.
- **Calibration corpus** for bid-portal-procontract — per-form-complexity variants (simple/medium/complex), each with `canary.inputs.jsonl` + `golden.assertions.yaml` + `metadata.json` + `manifest.md` in `tests/fixtures/calibration/bid-portal-procontract/`.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)
1. `docs/strategy/layers/layer-6-request-pipeline-automation.md` — **READ FULLY** (post step-01/02)
2. `docs/pipeline/l6-request-pipeline-automation/manifest.yaml` — **READ FULLY**
3. `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md` — **STUDY** (quality bar)
4. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — **STUDY** (alt quality bar; includes RLS testing)
5. `crates/daemon/tests/skill_golden_journeys.rs` — **STUDY** (how skill promotion lifecycle is tested today)
6. `crates/daemon/tests/skill_canary_harness.rs` — **STUDY** (how new skills are canaried today)
7. `crates/daemon/tests/support/db.rs` — **STUDY** (TestStateDb helper)

### Step 1: Invoke the Skill
Run `/spec-to-tdd`.

**Inputs:**
- Spec: `docs/strategy/layers/layer-6-request-pipeline-automation.md`
- Output TDD: `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md`
- Output Bootstrap: `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md`

**Key arguments / context to provide the skill:**
- Test ID scheme: `L6-T-{domain}-{ordinal}` where domain ∈ {`bid_workflow_states`, `portal_side_effects`, `bid_readiness_gate`, `bid_portal_procontract_skill`, `validator_overlay`, `daemon_runner`, `ipc_trigger_bid_workflow`, `error_codes`, `sync_propagation`, `desktop_ui`, `web_ui`, `live_demo`}
- Existing infrastructure tier list (from `manifest.yaml#existing_files`)
- Net-new tier list (from `manifest.yaml#new_modules_ts` + `new_modules_rust` + `new_skills`)
- L5→L6 readiness gate constitutionality (cannot be weakened by any test or implementation choice)
- Validation policy fail-closed exception (bid submission is the ONLY current fail-closed surface; all other Always-policy skills are fail-open per L3)
- Test infrastructure patterns: TestStateDb + cassettes + persistent CDP for smoke + Playwright for desktop/web E2E

### Step 2: Feature-Specific Guidance for the Skill

While `/spec-to-tdd` runs, ensure the TDD covers:

1. **State machine — all 14 transitions × happy/sad** (~28-40 tests in `bid-workflow-states.test.ts`) — spec §1.3 transition table has **14** distinct From→To rows (NOT 11; re-count yourself before authoring tests)
2. **L5 readiness gate — 5 blockers + quote-role + freshness + company profile + upstream trust** (~10-18 tests in `bid-readiness-gate.test.ts`). **Include operator-override coverage**: per `manifest.yaml#cross_layer_context.operator_override_mechanisms`, the quote-role and validation gates are OVERRIDEABLE with logged acknowledgment; the 5 L5 hard blockers + operator-approval + staleness + upstream-trust are NON-OVERRIDEABLE. Every overrideable gate needs both "no override → blocked" AND "override present → allowed (with audit-trail entry)" test cases.
3. **Side-effect class — 6 values × progression rules + cap enforcement** (~8-15 tests in `portal-side-effects.test.ts`)
4. **decisionBasisHash — all 5 input vectors + staleness detection** (covered in `bid-workflow-states.test.ts` or split)
5. **Bid portal skill — 11 playbook steps × canary inputs** (in `skill_canary_harness.rs` extensions starting at `e2e_can_010` — current highest is `e2e_can_009` per direct verification 2026-05-21). Skill frontmatter MUST include `source_structure_version: procontract-bid-portal-YYYY-MM-DD` (Procontract convention — all 3 existing procontract skills carry this field for HTML/selector drift tracking). Skill frontmatter `model_profile: portal_form_high_reliability` is NEW (existing skills use `browse_stable_low_cost` or `structured_extract_low_cost`) — TDD must specify a Phase 1 daemon-code task to add the new match arm in `crates/daemon/src/main.rs`'s `model_profile.as_str()` block, OR explicitly decide to reuse an existing profile.
6. **Validator overlay — fail-closed rejection + fail-closed outage + operator override path** (in `validation_executor.rs` extensions OR new `validation_overlay_bid_portal.rs`). The override path test: validator returns rejected verdict → operator submits "Override and submit" with structured override_reason → bid transitions to bid_submitted AND the override is recorded in run manifest metadata (validator overlay then flags any future runs that submit without citing the override pattern as anomaly).
7. **IPC handler — `trigger_bid_workflow` with readiness gate pre-check** (new test file or extend existing IPC tests)
8. **ErrorCode propagation — 3-6 new variants (TDD-determined, NOT 6 by default) pass clippy + as_str() round-trip** (in `crates/daemon/src/errors/manifest.rs` #[cfg(test)] OR new integration test). Author MUST audit existing variants first to avoid duplication.
9. **PII redaction — BidScopePayload + CompanyProfile fields scrubbed from error manifests + run manifest narrative + Supabase projection** (extend `redaction_boundary.rs`)
10. **Sync propagation — bid_workflow_states mutations reach Supabase via existing push set** (extend `sqlite_supabase_sync.rs` if it exists, OR add a focused test)
11. **Desktop E2E — Start Bid button visible when ready, blocked with actionable error otherwise, draft_saved review panel renders** (in `apps/desktop/e2e/`)
12. **Web E2E — bid_state badge visible to viewer role, BidScopePayload + run manifest NOT visible to viewer role** (in `apps/web/e2e/`)
13. **Live demo — operator-driven, sideEffectCap = REMOTE_DRAFT, reaches draft_saved with screenshot trail + run manifest + Supabase visibility** (manual smoke; documented in TDD; verified in step 12 completion audit)

### Step 3: Bootstrap Doc Authoring

The bootstrap (`l6-request-pipeline-automation-bootstrap.md`) must cover:

1. **Environment prerequisites:**
   - Rust nightly toolchain (already pinned via `rust-toolchain.toml`)
   - Bun 1.3.10 (already pinned)
   - Persistent CDP profile at `/Users/alinaaitova/.chrome-automation` with `sales@sorelon.com` Procontract credential saved in Chrome's Passwords (Touch ID requirement removed; confirm via `chrome://settings/passwords`)
   - At least one identified Procontract opportunity reference for live-demo testing (operator-selected, won't generate buyer-visible legal commitment)
   - AgentMail running at `http://127.0.0.1:8765` (health check `curl -s http://127.0.0.1:8765/health/liveness`)
   - L5 complete and `ready_for_l6: true` confirmed in L5 status file
2. **Quality gate sequence (verbatim from L5's bootstrap — local-only, no CI workflow):**
   ```bash
   cargo fmt --check
   cargo check --all-targets
   cargo clippy --all-targets -- -D warnings
   cargo test -p daemon --bin daemon
   for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
   bun run build
   bun run lint
   bun run test
   (cd apps/desktop && bun run test:e2e)
   (cd apps/web && bun run test:e2e)
   ```
3. **Validation script:** A short shell script that checks all prerequisites in one pass and exits non-zero with actionable error on missing items.
4. **External account negative list:** L6 requires ZERO paid commercial accounts (same skill-first philosophy as L5). The only external dependency is the Procontract account `sales@sorelon.com` (already provisioned).
5. **Per-CDP-skill smoke prerequisite:** `bin/ensure-cdp-9222` is idempotent; bootstrap documents the launch command + check `curl -sfS -m 3 http://localhost:9222/json/version`.
6. **AgentMail registration walkthrough:** identical to L5's bootstrap (same project_key `/Users/alinaaitova/Desktop/Testproject1`).

### Step 4: Verify Output

After `/spec-to-tdd` completes, verify:
- [ ] TDD file exists at `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md`
- [ ] Bootstrap file exists at `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md`
- [ ] TDD has a coverage matrix mapping every `L6-FR-X.Y.Z` → ≥1 `L6-T-*` test ID
- [ ] TDD covers all 13 areas listed in Step 2
- [ ] TDD includes static CI gate list (likely ~8 gates per L4/L5 precedent)
- [ ] TDD specifies test pyramid (T1 unit / T2 integration / T3 E2E / T4 manual smoke)
- [ ] Bootstrap covers all 6 environment prerequisite categories
- [ ] Bootstrap has the local quality gate sequence verbatim
- [ ] Bootstrap notes ZERO new paid commercial accounts (skill-first)
- [ ] Bootstrap includes the CDP smoke prerequisite

### Step 5: Follow-Up Pass (Sub-Optimization)

> ULTRATHINK. After `/spec-to-tdd` finishes, re-read both outputs with fresh eyes.

Specifically check:

1. **Open `docs/strategy/layers/layer-6-request-pipeline-automation.md` and enumerate every `L6-FR-X.Y.Z`.** Open the TDD and search for each ID. Does each appear in the coverage matrix? If any ID has ZERO mapped tests, the TDD has a gap — fix it.

2. **Open `crates/daemon/src/errors/manifest.rs` lines 19-114.** Count the existing variants. The TDD should specify the exact 6 new variant names + their snake_case `as_str()` values + the propagation site count test target (5-15 sites per L5 precedent). Verify.

3. **Open `crates/daemon/src/validation/policy.rs` lines 17-25.** Confirm the TDD's fail-closed-exception tests target the bid-workflow STATE MACHINE layer (not a new policy mode). The validator job uses `ValidationPolicyMode::Always` exactly as L5's customs validators do.

4. **Open `crates/daemon/src/main.rs` around line 1513-1517 and find the existing job-kind dispatch** (match arms for `"extract_document_structure"` and `"extract_document_semantics"`). The dispatch is in **main.rs**, NOT in `jobs/runner.rs` (verify by grepping for `extract_document_structure` — you'll see the arm in main.rs). Confirm the TDD specifies a test that the new `"run_bid_workflow"` arm is dispatched correctly and reaches the skill execution path (the handler function mirrors `run_extract_document_structure_job` at line 1529).

5. **Open `crates/daemon/src/ipc/handlers/jobs.rs` and read `trigger_ingestion` at line 45 + `trigger_enrichment` at line 95** — these are the exact reference patterns. Confirm the TDD's `trigger_bid_workflow` test specifies:
   - The readiness-gate pre-check happens BEFORE job enqueue (not after)
   - Failed gate returns a structured rejection with the specific blocker code(s) — the rejection shape mirrors how existing handlers return errors
   - Successful gate creates a workflow_job with kind='run_bid_workflow' via EnqueueJobRequest and returns the continuation_job_id

6. **Open `skills/procurement-ingestion/procontract-register-interest/SKILL.md` and study the "Mandatory CDP Discipline" section.** Confirm the TDD's canary test for `bid-portal-procontract` enforces the same discipline (attach to persistent profile, no fresh browser, no portal login attempt outside auth-signin skill, no captcha solving, no logout, no closing the operator's browser).

7. **Open the L5 TDD and count its assertion IDs (`L5-T-*`).** L5 has 98 IDs across 57 functions. L6 should have a comparable density (~80-120 IDs across ~40-80 functions given L6 is smaller surface).

8. **Open the L4 TDD and look at its RLS test matrix.** L6's web E2E tests should follow the same matrix-cell pattern for the bid workflow RLS boundary (viewer can see `bid_state` badge; viewer CANNOT see `bid_scope_json` or run manifest).

9. **Re-walk every DoD criterion (§2.1, 11 of them).** For each, name the `L6-T-*` test IDs that, when green, prove the DoD criterion is met. If any DoD has zero mapped tests, fix.

10. **Verify the calibration corpus structure** for `tests/fixtures/calibration/bid-portal-procontract/`. Per-form-complexity variants (simple/medium/complex), each with full L5-style fixture layout. The TDD must specify the canary input count per variant.

11. **Verify the bootstrap doc references the AUTHORITATIVE `br` block** at AGENTS.md lines 279-330, NOT the stale `bd` block at lines 683-738.

12. **Confirm zero scope-drift language** in both TDD and bootstrap. Grep for "post-MVP", "stretch goal", "nice to have", "deferred", "later".

### Step 6: Scope-Preservation Check (MANDATORY — BLOCKING)

- [ ] Every L6-FR-X.Y.Z appears in the TDD coverage matrix with ≥1 mapped L6-T-* ID
- [ ] Every DoD criterion (11) is satisfied by ≥1 test
- [ ] All **14** state-machine transitions have explicit tests (one happy + one sad per transition where applicable)
- [ ] All 8 invariants have explicit tests (or are statically guaranteed by Rust types + a test that proves the type guarantee)
- [ ] All 5 L5 readiness-gate blockers have explicit tests
- [ ] All 6 ErrorCode additions have round-trip tests
- [ ] All 6 SideEffectClass values appear in side-effect-progression tests
- [ ] Validator overlay (overlay-bid-portal-procontract) has dedicated tests
- [ ] Desktop E2E has Start Bid + draft_saved review tests
- [ ] Web E2E has RLS boundary tests
- [ ] Calibration corpus structure specified
- [ ] Bootstrap covers all 6 prerequisite categories
- [ ] Zero scope-drift language found
- [ ] Bootstrap quality gate sequence is verbatim from L5

If any checkbox is `[ ]`, fix before declaring step 03 complete.

---

## Output

| File | Path |
|------|------|
| TDD | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md` |
| Bootstrap | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md` |

Expected sizes: TDD 700-900 lines (mirrors L4/L5 density); bootstrap 600-900 lines.

---

## Key Reminders

- The 3 test files in spec §2.2 are MINIMUMS — add more if test coverage requires
- 3-6 NEW ErrorCode variants (TDD-determined; audit existing codes first) need propagation tests (clippy is the first signal)
- The L5→L6 readiness gate is constitutional — NO test may weaken it
- Validation fail-closed for `bid_submitted` is the ONLY current fail-closed surface — the TDD must clearly distinguish from the L3 fail-open default
- No mocks for HTTP — cassettes or live CDP only (per L5 policy)
- The bid-portal-procontract canary tests inherit the auth-signin/procontract CDP discipline
- ZERO SCOPE DRIFT: every requirement has at least one test
- Quality bar: L5 TDD (873 lines, 98 assertion IDs) and L4 TDD (623 lines, 158 checks)
