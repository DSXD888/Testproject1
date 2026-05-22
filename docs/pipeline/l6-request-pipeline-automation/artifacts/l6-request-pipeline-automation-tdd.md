# L6 Request Pipeline Automation - TDD Specification

## 1. Scope

This is Source of Truth 2 for Layer 6 request pipeline automation. It translates
`docs/strategy/layers/layer-6-request-pipeline-automation.md` into the test set
that must pass before the layer can be declared implemented.

Primary inputs:

- `docs/strategy/layers/layer-6-request-pipeline-automation.md`
- `docs/pipeline/l6-request-pipeline-automation/manifest.yaml`
- Existing daemon validation, job runner, PTY, sync, IPC, error-manifest, and
  skill canary infrastructure listed in the manifest.

Reference quality bars:

- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md`
- `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md`

Spec inventory:

- 127 unique `L6-FR-*` requirement IDs.
- 14 distinct `bid_workflow_states` transition rows.
- 11 bid portal playbook steps, with 10a and 10b mutually exclusive by
  `sideEffectCap`.
- 6 side-effect classes.
- 11 definition-of-done requirements.
- 47 existing Rust daemon integration test files plus 10 existing package
  `*.test.ts` files at TDD authoring time. L6 adds 3 package test files, bringing
  the cumulative Rust-plus-package test-file count to 60.

### 1.1 TDD Decisions

| Decision | TDD position | Test consequence |
|---|---|---|
| L6 module placement | Use a dedicated `crates/daemon/src/bid_workflow/` module tree and wire it from `main.rs`; job-kind dispatch remains in `main.rs`. | `L6-T-daemon_runner-001`, `003` fail if dispatch is moved to `jobs/runner.rs` or if bid logic is buried in unrelated modules. |
| Model profile | Add `portal_form_high_reliability` and test the `model_profile.as_str()` match arm in `crates/daemon/src/main.rs`. | `L6-T-daemon_runner-002` and `L6-T-bid_portal_procontract_skill-001` require the new profile. |
| Error codes | Add four new variants: `BID_SUBMISSION_FAILED`, `BID_DUPLICATE_DETECTED`, `BID_SIDE_EFFECT_CAP_EXCEEDED`, `BID_READINESS_GATE_FAILED`. Reuse `PAGE_SHAPE_DRIFT` for portal structure drift and `AUTH_STATE_STALE` for auth expiry. | `L6-T-error_codes-001` through `005` lock the decision and prevent redundant `BID_PORTAL_CHANGED` / `BID_PORTAL_AUTH_EXPIRED` drift. |
| SQL state enforcement | Do not add a SQL `CHECK` to `bid_workflow_states.bid_state`; enforce seven lifecycle states in Rust. | `L6-T-bid_workflow_states-001`, `034` require Rust enum validation and schema stability. |
| Validation policy | `bid_submitted` is the only current fail-closed validation surface. `draft_saved` may be fail-open under Always-policy outage, but final submission may not. | `L6-T-validator_overlay-008`, `009`, `010` and `L6-T-bid_workflow_states-010`, `016` split rejection, outage, and override. |
| Upstream trust override | Quote-role and validation gates are overrideable with structured operator acknowledgment. The five L5 hard blockers, operator approval, staleness, classification freshness, company-profile currency, and upstream trust are non-overrideable. | `L6-T-bid_readiness_gate-009`, `010`, `011`, `016` keep override boundaries explicit. |

## 2. Non-Negotiable Test Principles

1. Every `L6-FR-*` has at least one stable `L6-T-*` test ID in the coverage matrix.
2. No requirement is deferred to manual smoke, "post-MVP", or untracked inspection.
3. Bid portal tests use cassette replay for fast tiers or a real persistent CDP
   browser for smoke tiers. There are no HTTP mocks for portal behavior.
4. The bid skill never calls the auth skill directly. It reports
   `AUTH_STATE_STALE`; the daemon performs the delegated retry.
5. The readiness gate is constitutional. A test may improve diagnostics, but it
   may not weaken any blocker.
6. Side-effect progression is monotonic and bounded by `sideEffectCap`.
7. Final submission is fail-closed on validator rejection and validator outage.
8. `sideEffectCap=REMOTE_DRAFT` is the default live-demo cap. Any formal
   submission requires separate operator authorization.
9. PII, company-profile values, BidScopePayload contents, and portal artifacts
   are redacted from public error paths and read-only web projections.
10. Desktop and web tests must verify real user-visible affordances and absence
    of mutation paths, not just component rendering.

## 3. Test Pyramid

| Tier | Purpose | Primary surfaces | Runtime expectation |
|---|---|---|---|
| T1 Unit | Pure state, scope, hash, readiness, side-effect, enum, and schema logic. | `packages/bidding/**/*.test.ts`, daemon unit tests inside touched Rust modules. | Fast local and pre-commit. |
| T2 Integration | SQLite, daemon job dispatch, IPC handlers, sync, validation executor, canary/golden harness, redaction. | `crates/daemon/tests/*.rs`. | Full local daemon integration loop. |
| T3 E2E | Desktop and web user workflows against local services, replayed portal cassettes where possible. | `apps/desktop/e2e`, `apps/web/e2e`, Playwright. | Local gate, no buyer-visible mutation. |
| T4 Manual smoke | Operator-driven Procontract run through persistent CDP. | `skills/bid-portal-procontract/`, `bin/ensure-cdp-9222`, live Procontract account. | Step 12 audit evidence, default cap `REMOTE_DRAFT`. |

## 4. CI And Local Gate Inventory

The layer has ten static gates. The bootstrap document gives the local command
sequence verbatim.

| Gate ID | Gate | Proves |
|---|---|---|
| `CI-L6-001` | `cargo fmt --check` | Rust formatting is stable. |
| `CI-L6-002` | `cargo check --all-targets` | Rust code and test targets compile. |
| `CI-L6-003` | `cargo clippy --all-targets -- -D warnings` | Exhaustive matches, new enum propagation, and lint policies pass. |
| `CI-L6-004` | `cargo test -p daemon --bin daemon` | Daemon binary unit tests pass. |
| `CI-L6-005` | `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done` | All daemon integration targets, including L6 extensions, pass. |
| `CI-L6-006` | `bun run build` | Workspace TypeScript packages and apps build. |
| `CI-L6-007` | `bun run lint` | TypeScript lint and no hidden `any` escapes pass. |
| `CI-L6-008` | `bun run test` | Package-level unit and integration tests pass. |
| `CI-L6-009` | `(cd apps/desktop && bun run test:e2e)` | Desktop bid workflow E2E passes. |
| `CI-L6-010` | `(cd apps/web && bun run test:e2e)` | Web viewer visibility and read-only E2E passes. |

## 5. Shared Test Infrastructure

| Harness or fixture | Owner surface | Requirements |
|---|---|---|
| `BidWorkflowStateMachineHarness` | `packages/bidding/tests/bid-workflow-states.test.ts` | Enumerates seven states, fourteen transitions, retry semantics, Rust-compatible JSON payloads, and no SQL `CHECK` assumptions. |
| `BidScopeFixture` | `packages/bidding`, daemon IPC tests | Builds valid and invalid BidScopePayload instances with process, lot, candidate, quote, company-profile, portal reference, hash, and cap vectors. |
| `BidReadinessGateFixture` | `packages/bidding/tests/bid-readiness-gate.test.ts` | Seeds operator decisions, L5 landed-cost blockers, supplier quote roles, company profile freshness, classification freshness, and upstream trust states. |
| `PortalSideEffectFixture` | `packages/bidding/tests/portal-side-effects.test.ts` | Defines the six ordered side-effect classes and maps every bid skill playbook step to a class. |
| `ProcontractCassetteHarness` | skill canary and golden journey tests | Replays simple, medium, and complex Procontract form captures. It may not emulate arbitrary HTTP not observed in the cassette. |
| `PersistentCdpSmokeHarness` | T4 manual smoke | Attaches to `/Users/alinaaitova/.chrome-automation:9222` and forbids closing the browser, signing out, or creating a fresh profile. |
| `BidValidatorOverlayHarness` | daemon validation executor tests | Runs `skills/validation/overlay-bid-portal-procontract/` and records accepted, rejected, outage, override, and anomaly verdicts. |
| `BidErrorCodeHarness` | daemon error manifest tests | Checks enum, `as_str()`, parsing, markdown rendering, structured tracing, and redaction context for the four new bid codes. |
| `BidSyncHarness` | SQLite to Supabase sync tests | Confirms `bid_workflow_states` mutations are push-owned and remote projections hide sensitive payloads. |
| `BidRedactionSentinelFixture` | `redaction_boundary.rs` | Uses unique company-profile and BidScopePayload sentinel values that must not leak into manifests, projections, logs, or web snapshots. |
| `DesktopBidWorkflowHarness` | desktop E2E | Drives Start Bid, blocked diagnostics, override acknowledgments, draft review, and validation override UI. |
| `WebBidVisibilityHarness` | web E2E | Drives viewer-role bid-state badges and absence of mutation or sensitive artifact access. |
| `LiveDemoRunbookHarness` | manual smoke evidence | Records operator-selected opportunity reference, cap, screenshot trail, run manifest path, sync evidence, and final state. |

## 6. Planned Test File Inventory

### 6.1 New Package Tests

| File | Target count | Primary IDs |
|---|---:|---|
| `packages/bidding/tests/bid-workflow-states.test.ts` | 34 checks | `L6-T-bid_workflow_states-001` through `034` |
| `packages/bidding/tests/portal-side-effects.test.ts` | 12 checks | `L6-T-portal_side_effects-001` through `012` |
| `packages/bidding/tests/bid-readiness-gate.test.ts` | 16 checks | `L6-T-bid_readiness_gate-001` through `016` |

These three files are the minimum named by the spec. They bring the current
47 daemon integration files plus 10 package test files to 60 Rust-plus-package
test files.

### 6.2 Daemon Test Extensions

| File | L6 addition |
|---|---|
| `crates/daemon/tests/skill_canary_harness.rs` | Add `e2e_can_010+` coverage for bid portal canaries and validator overlay. |
| `crates/daemon/tests/skill_golden_journeys.rs` | Add `l3_t_skill_golden_journeys_009+` promotion lifecycle coverage for the bid skill. |
| `crates/daemon/tests/redaction_boundary.rs` | Extend PII redaction coverage for BidScopePayload and company-profile sentinel fields. |
| `crates/daemon/tests/sqlite_schema.rs` | Assert no new table or SQL `CHECK` is introduced for `bid_workflow_states`. |
| `crates/daemon/tests/sqlite_supabase_sync.rs` | Extend push-owned sync coverage for `bid_workflow_states` projection behavior. |
| `crates/daemon/tests/ipc_*` or a new focused IPC test | Cover `trigger_bid_workflow` request validation and readiness pre-check. |
| `crates/daemon/src/errors/manifest.rs` tests or focused integration | Cover new bid error-code enum variants and string round-trips. |

### 6.3 E2E And Smoke Tests

| File or evidence | L6 addition |
|---|---|
| `apps/desktop/e2e/bid-workflow.spec.ts` | Start Bid, blocked diagnostics, overrides, draft review, validation override, SessionViewer. |
| `apps/web/e2e/bid-visibility.spec.ts` | Viewer badge, redacted payloads, no mutation controls, live refresh. |
| Step 12 manual smoke evidence | Procontract REMOTE_DRAFT run with screenshot trail, manifest, and Supabase visibility. |

## 7. Detailed Test Catalog

### 7.1 Bid Workflow State Machine

| Test ID | Tier | Test |
|---|---|---|
| `L6-T-bid_workflow_states-001` | T1/T2 | The lifecycle enum accepts exactly `not_started`, `ready_to_bid`, `bid_in_progress`, `draft_saved`, `bid_submitted`, `bid_failed`, and `portal_state_unknown`; unknown values fail Rust validation before persistence. |
| `L6-T-bid_workflow_states-002` | T1 | `not_started -> ready_to_bid` succeeds only when every readiness gate passes and no BidScopePayload is required yet. |
| `L6-T-bid_workflow_states-003` | T1 | `not_started -> ready_to_bid` is rejected with `BID_READINESS_GATE_FAILED` when any readiness blocker exists. |
| `L6-T-bid_workflow_states-004` | T1 | `ready_to_bid -> bid_in_progress` succeeds for manual operator start and daemon continuation with a valid BidScopePayload. |
| `L6-T-bid_workflow_states-005` | T1 | `ready_to_bid -> bid_in_progress` is rejected when BidScopePayload is missing, malformed, stale, or references unknown entities. |
| `L6-T-bid_workflow_states-006` | T1 | `bid_in_progress -> ready_to_bid` supports safe pre-mutation cancellation or readiness regression and clears transient runner state. |
| `L6-T-bid_workflow_states-007` | T1 | `bid_in_progress -> ready_to_bid` is rejected after any remote side effect above `READ_ONLY`. |
| `L6-T-bid_workflow_states-008` | T1/T2 | `bid_in_progress -> draft_saved` succeeds when the skill saves a remote draft within cap and records draft evidence. |
| `L6-T-bid_workflow_states-009` | T1/T2 | `bid_in_progress -> draft_saved` is rejected when draft evidence, screenshot trail, or manifest linkage is absent. |
| `L6-T-bid_workflow_states-010` | T1/T2 | `bid_in_progress -> bid_submitted` succeeds only when cap allows formal submission and validator verdict is accepted or structured override is present. |
| `L6-T-bid_workflow_states-011` | T1/T2 | `bid_in_progress -> bid_submitted` is rejected on validator rejection, validator outage, stale decision basis, or cap breach. |
| `L6-T-bid_workflow_states-012` | T1/T2 | `bid_in_progress -> bid_failed` records deterministic failure before remote mutation and emits the selected error code. |
| `L6-T-bid_workflow_states-013` | T1/T2 | `bid_in_progress -> bid_failed` is rejected for failures after a remote draft exists; those cases must move to `portal_state_unknown`. |
| `L6-T-bid_workflow_states-014` | T1/T2 | `bid_in_progress -> portal_state_unknown` succeeds when remote state cannot be reconciled after mutation, crash, duplicate detection, or portal ambiguity. |
| `L6-T-bid_workflow_states-015` | T1 | `bid_in_progress -> portal_state_unknown` is rejected when no remote side effect or portal ambiguity has occurred. |
| `L6-T-bid_workflow_states-016` | T1/T2 | `draft_saved -> bid_submitted` succeeds only after fresh readiness, cap, and fail-closed validation checks. |
| `L6-T-bid_workflow_states-017` | T1/T2 | `draft_saved -> bid_submitted` is rejected on stale decision basis, validator rejection, validator outage without override, or operator cap mismatch. |
| `L6-T-bid_workflow_states-018` | T1/T2 | `draft_saved -> bid_in_progress` resumes editing only after verifying the existing remote draft state. |
| `L6-T-bid_workflow_states-019` | T1/T2 | `draft_saved -> bid_in_progress` is rejected when the remote draft was changed outside the daemon or cannot be verified. |
| `L6-T-bid_workflow_states-020` | T1 | `draft_saved -> bid_failed` records operator abandonment without treating the remote draft as safely gone. |
| `L6-T-bid_workflow_states-021` | T1 | `draft_saved -> bid_failed` is rejected for automated retry failures that cannot prove remote state is harmless. |
| `L6-T-bid_workflow_states-022` | T1/T2 | `portal_state_unknown -> bid_in_progress` succeeds only after explicit operator reconciliation and portal re-verification. |
| `L6-T-bid_workflow_states-023` | T1/T2 | `portal_state_unknown -> bid_in_progress` is rejected without reconciliation evidence. |
| `L6-T-bid_workflow_states-024` | T1/T2 | `portal_state_unknown -> bid_submitted` succeeds only on operator-confirmed portal submission with confirmation reference. |
| `L6-T-bid_workflow_states-025` | T1/T2 | `portal_state_unknown -> bid_submitted` is rejected if the confirmation reference is missing or conflicts with the run manifest. |
| `L6-T-bid_workflow_states-026` | T1/T2 | `portal_state_unknown -> bid_failed` succeeds when operator reconciliation proves no buyer-visible submission exists. |
| `L6-T-bid_workflow_states-027` | T1/T2 | `portal_state_unknown -> bid_failed` is rejected when portal evidence is ambiguous. |
| `L6-T-bid_workflow_states-028` | T1/T2 | `bid_failed -> ready_to_bid` supports retry only after readiness passes and previous failure context is preserved. |
| `L6-T-bid_workflow_states-029` | T1/T2 | `bid_failed -> ready_to_bid` is rejected when the previous failure indicates unresolved remote mutation or stale inputs. |
| `L6-T-bid_workflow_states-030` | T1/T2 | BidScopePayload validates all required references: process, lots, candidates, selected quotes, company profile version, portal type, and portal opportunity reference. |
| `L6-T-bid_workflow_states-031` | T1/T2 | `decisionBasisHash` is a 64-character SHA-256 over the five required input vectors and changes when any vector changes. |
| `L6-T-bid_workflow_states-032` | T1 | `reviewRequired` defaults to true and `sideEffectCap` defaults to `REMOTE_DRAFT` for operator-started workflows. |
| `L6-T-bid_workflow_states-033` | T2 | `bid_scope_json` is persisted for active and terminal bid workflows but is never exposed through read-only projection payloads. |
| `L6-T-bid_workflow_states-034` | T2 | SQLite schema for `bid_workflow_states` remains table-compatible with existing migrations and has no SQL lifecycle `CHECK`. |

### 7.2 Bid Readiness Gate

| Test ID | Tier | Test |
|---|---|---|
| `L6-T-bid_readiness_gate-001` | T1/T2 | Pending operator intents are ignored; only approved `operator_decision_heads` can make a process eligible. |
| `L6-T-bid_readiness_gate-002` | T1/T2 | A stale `reviewedBasisHash` blocks start, resume, and final submit with an actionable stale-decision diagnostic. |
| `L6-T-bid_readiness_gate-003` | T1/T2 | `CLASSIFICATION_AMBIGUOUS` blocks and cannot be operator-overridden. |
| `L6-T-bid_readiness_gate-004` | T1/T2 | `TARIFF_BRANCH_DIVERGENCE` blocks and cannot be operator-overridden. |
| `L6-T-bid_readiness_gate-005` | T1/T2 | `SUPPLEMENTARY_UNIT_MISSING` blocks and cannot be operator-overridden. |
| `L6-T-bid_readiness_gate-006` | T1/T2 | `decision_grade = broker_required` blocks and cannot be operator-overridden. |
| `L6-T-bid_readiness_gate-007` | T1/T2 | Expired classification freshness blocks and cannot be operator-overridden. |
| `L6-T-bid_readiness_gate-008` | T1/T2 | Missing, expired, or mismatched company-profile version blocks and cannot be operator-overridden. |
| `L6-T-bid_readiness_gate-009` | T1/T2 | Supplier quote roles `domestic_wholesale_floor` and `optimistic_floor` block when no structured override exists. |
| `L6-T-bid_readiness_gate-010` | T1/T2 | Quote-role override allows progress only with structured acknowledgment and an audit-trail entry cited in the run manifest. |
| `L6-T-bid_readiness_gate-011` | T1/T2 | Upstream trust state other than `validated` blocks and cannot be operator-overridden. |
| `L6-T-bid_readiness_gate-012` | T1/T2 | A fully green candidate becomes eligible and can transition to `ready_to_bid`. |
| `L6-T-bid_readiness_gate-013` | T1/T2 | Readiness is recomputed at start, resume, draft review, and final submit; stale cached gate decisions are rejected. |
| `L6-T-bid_readiness_gate-014` | T1/T2 | Missing or ambiguous `portalOpportunityRef` blocks before any portal side effect. |
| `L6-T-bid_readiness_gate-015` | T1/T2 | Multiple blockers are reported together without short-circuit hiding. |
| `L6-T-bid_readiness_gate-016` | T1/T2 | Override attempts cannot clear non-overrideable gates and are logged as rejected acknowledgments. |

### 7.3 Portal Side Effects

| Test ID | Tier | Test |
|---|---|---|
| `L6-T-portal_side_effects-001` | T1 | SideEffectClass ordering is exactly `READ_ONLY < REMOTE_DRAFT < NOTIFICATION_SENT < LEGAL_ACKNOWLEDGED < FORMAL_SUBMISSION < FUNDS_COMMITTED`. |
| `L6-T-portal_side_effects-002` | T1/T2 | Each bid skill playbook step declares the expected side-effect class, including the mutually exclusive 10a draft and 10b final submit paths. |
| `L6-T-portal_side_effects-003` | T1/T2 | Observed side effects are monotonic; regression or skipped-class claims fail validation. |
| `L6-T-portal_side_effects-004` | T1/T2 | `sideEffectCap=READ_ONLY` halts before remote form mutation and records no buyer-visible change. |
| `L6-T-portal_side_effects-005` | T1/T2 | `sideEffectCap=REMOTE_DRAFT` allows draft save, blocks final submission, and emits `BID_SIDE_EFFECT_CAP_EXCEEDED` on escalation. |
| `L6-T-portal_side_effects-006` | T1/T2 | `sideEffectCap=FORMAL_SUBMISSION` allows final submit but blocks any later `FUNDS_COMMITTED` side effect. |
| `L6-T-portal_side_effects-007` | T1/T2 | Unexpected `NOTIFICATION_SENT`, `LEGAL_ACKNOWLEDGED`, or `FUNDS_COMMITTED` evidence moves state to `portal_state_unknown`. |
| `L6-T-portal_side_effects-008` | T1/T2 | Any retry after `REMOTE_DRAFT` or above must call portal verification before editing. |
| `L6-T-portal_side_effects-009` | T1/T2 | Duplicate remote draft or submission evidence raises `BID_DUPLICATE_DETECTED` and blocks blind retry. |
| `L6-T-portal_side_effects-010` | T1/T2 | Crash before remote draft can safely retry; crash after remote draft moves to `portal_state_unknown`. |
| `L6-T-portal_side_effects-011` | T2 | Screenshot trail is complete for every side-effect checkpoint and missing screenshots reject promotion. |
| `L6-T-portal_side_effects-012` | T1/T2 | Uploaded document hashes are checked; mismatch or failed upload raises `BID_SUBMISSION_FAILED`. |

### 7.4 Bid Portal Procontract Skill

| Test ID | Tier | Test |
|---|---|---|
| `L6-T-bid_portal_procontract_skill-001` | T2 | `skills/bid-portal-procontract/SKILL.md` frontmatter declares `pty_interactive`, `validation: always`, `requires_cdp: true`, `model_profile: portal_form_high_reliability`, and `source_structure_version: procontract-bid-portal-YYYY-MM-DD`. |
| `L6-T-bid_portal_procontract_skill-002` | T1/T2 | `packages/bidding/workflow-registry.ts` maps `bidPortalType="procontract"` to the Procontract workflow, rejects unknown portal types, and exports schemas compatible with the skill result and checkpoint contracts. |
| `L6-T-bid_portal_procontract_skill-003` | T2 | Calibration corpus exists for simple, medium, and complex Procontract forms, each with `canary.inputs.jsonl`, `golden.assertions.yaml`, `metadata.json`, and `manifest.md`. |
| `L6-T-bid_portal_procontract_skill-004` | T2/T4 | CDP attach discipline uses `bin/ensure-cdp-9222`, attaches to `/Users/alinaaitova/.chrome-automation:9222`, and never launches a fresh profile, closes the browser, or signs out. |
| `L6-T-bid_portal_procontract_skill-005` | T2 | Expired Procontract auth returns `AUTH_STATE_STALE` with portal context; the bid skill does not invoke the auth skill directly. |
| `L6-T-bid_portal_procontract_skill-006` | T2/T4 | Opportunity lookup verifies portal reference, title, status, and duplicate state before mutation; portal structure drift reuses `PAGE_SHAPE_DRIFT`. |
| `L6-T-bid_portal_procontract_skill-007` | T2/T4 | Form fill uses only BidScopePayload, selected quotes, bid documents, and the versioned company profile. |
| `L6-T-bid_portal_procontract_skill-008` | T2/T4 | Document upload records filename, content hash, portal acknowledgement, and screenshot evidence. |
| `L6-T-bid_portal_procontract_skill-009` | T2/T4 | Draft save path returns `draft_saved`, remote draft reference, screenshot trail, and no final-submission confirmation. |
| `L6-T-bid_portal_procontract_skill-010` | T2/T4 | Final submit path returns `bid_submitted`, confirmation reference, and validator-ready manifest evidence. |
| `L6-T-bid_portal_procontract_skill-011` | T2 | `skill_canary_harness.rs` extends from current `e2e_can_009` with `e2e_can_010+` bid portal canaries. |
| `L6-T-bid_portal_procontract_skill-012` | T2 | `skill_golden_journeys.rs` extends from current `l3_t_skill_golden_journeys_008` with `l3_t_skill_golden_journeys_009+` promotion lifecycle tests. |

### 7.5 Validator Overlay

| Test ID | Tier | Test |
|---|---|---|
| `L6-T-validator_overlay-001` | T2 | `skills/validation/overlay-bid-portal-procontract/SKILL.md` declares the overlay role, worker skill ID, validation mode, evidence inputs, and rubric version. |
| `L6-T-validator_overlay-002` | T2 | Overlay verifies portal opportunity reference, process ID, lot IDs, and candidate IDs against the run manifest. |
| `L6-T-validator_overlay-003` | T2 | Overlay verifies screenshot coverage for lookup, form fill, upload, draft save, and final submit checkpoints. |
| `L6-T-validator_overlay-004` | T2 | Overlay rejects non-monotonic side-effect evidence or side-effect evidence above cap. |
| `L6-T-validator_overlay-005` | T2 | Overlay verifies final confirmation reference before accepting `bid_submitted`. |
| `L6-T-validator_overlay-006` | T2 | Overlay verifies narrative citations for lot, candidate, quote, company-profile version, and override IDs. |
| `L6-T-validator_overlay-007` | T2 | Overlay output and manifest appendix redact BidScopePayload and company-profile sentinel values. |
| `L6-T-validator_overlay-008` | T2 | Validator rejection blocks `bid_submitted` and records fail-closed status. |
| `L6-T-validator_overlay-009` | T2 | Validator outage blocks `bid_submitted` and records fail-closed outage diagnostics. |
| `L6-T-validator_overlay-010` | T2/T3 | Structured validation override allows final submit only with operator reason recorded in bid metadata and run manifest. |
| `L6-T-validator_overlay-011` | T2 | Future runs that submit without citing an established override pattern are flagged as anomalies. |

### 7.6 Daemon Runner

| Test ID | Tier | Test |
|---|---|---|
| `L6-T-daemon_runner-001` | T2 | Dedicated `bid_workflow` module exports state, scope, readiness, side-effect, and result types without moving job-kind dispatch out of `main.rs`. |
| `L6-T-daemon_runner-002` | T2 | `main.rs` model-profile match arm resolves `portal_form_high_reliability` to the intended provider, budget, and interaction profile. |
| `L6-T-daemon_runner-003` | T2 | `main.rs` job-kind dispatch adds `run_bid_workflow` beside existing extraction arms and routes to the L6 handler. |
| `L6-T-daemon_runner-004` | T2 | Runner selects `InteractionProfile::PtyRequired`, loads the bid skill, and rejects contract-only execution. |
| `L6-T-daemon_runner-005` | T2 | Runner injects CompanyProfile, BidDocuments, BidScopePayload, selected quote summaries, and portal opportunity reference into the skill input. |
| `L6-T-daemon_runner-006` | T2 | `workflow_jobs` and `bid_workflow_states` changes are atomic and idempotent on retry. |
| `L6-T-daemon_runner-007` | T2 | Raw PTY bytes, screenshot paths, and forensic artifacts are captured and redacted according to existing artifact rules. |
| `L6-T-daemon_runner-008` | T2 | `write_hybrid_run_manifest()` appends L6 side-effect progression, readiness evidence, validation verdict, and override metadata. |
| `L6-T-daemon_runner-009` | T2 | `AUTH_STATE_STALE` causes daemon-orchestrated auth refresh and one bounded retry without the bid skill calling auth directly. |
| `L6-T-daemon_runner-010` | T2 | CLI/manual trigger path supports `--run-bid-workflow` or equivalent local trigger without bypassing readiness gates. |
| `L6-T-daemon_runner-011` | T2 | `AdmissionPriority::BidAutomation` and browser-resource serialization prevent concurrent portal mutations for the same opportunity. |

### 7.7 IPC Trigger Bid Workflow

| Test ID | Tier | Test |
|---|---|---|
| `L6-T-ipc_trigger_bid_workflow-001` | T2/T3 | New IPC route and TypeScript bridge method validate the `trigger_bid_workflow` request and response schema. |
| `L6-T-ipc_trigger_bid_workflow-002` | T2/T3 | IPC handler runs readiness pre-check and returns actionable blocker payloads without enqueueing a job when blocked. |
| `L6-T-ipc_trigger_bid_workflow-003` | T2/T3 | Eligible request enqueues exactly one workflow job with bid scope, cap, and continuation metadata. |
| `L6-T-ipc_trigger_bid_workflow-004` | T2 | Invalid BidScopePayload is rejected before database mutation. |
| `L6-T-ipc_trigger_bid_workflow-005` | T2/T3 | Duplicate trigger request returns the same continuation acknowledgement and does not create duplicate portal work. |

### 7.8 Error Codes

| Test ID | Tier | Test |
|---|---|---|
| `L6-T-error_codes-001` | T1/T2 | Audit proves four new bid codes are required and that portal-changed/auth-expired reuse `PAGE_SHAPE_DRIFT` and `AUTH_STATE_STALE`. |
| `L6-T-error_codes-002` | T1 | New enum variants, `as_str()`, and parser round-trip for `BID_SUBMISSION_FAILED`, `BID_DUPLICATE_DETECTED`, `BID_SIDE_EFFECT_CAP_EXCEEDED`, and `BID_READINESS_GATE_FAILED`. |
| `L6-T-error_codes-003` | T1/T2 | Exhaustive matches, markdown rendering, tracing categorization, and static inventories include the four new variants. |
| `L6-T-error_codes-004` | T2 | Error manifests for each new code include portal-safe structured context and redact BidScopePayload/company-profile sentinel values. |
| `L6-T-error_codes-005` | T1/T2 | Tests fail if redundant `BID_PORTAL_CHANGED` or `BID_PORTAL_AUTH_EXPIRED` variants are added without revising this TDD. |

### 7.9 Sync Propagation

| Test ID | Tier | Test |
|---|---|---|
| `L6-T-sync_propagation-001` | T2 | `bid_workflow_states` remains in `PUSH_OWNED_RESEED_TABLES`; no new mixed-write table is introduced. |
| `L6-T-sync_propagation-002` | T2 | Local bid state mutation creates the expected push-owned sync event and projects state, timestamps, and non-sensitive summary fields. |
| `L6-T-sync_propagation-003` | T2/T3 | Supabase projection exposes bid-state summary to viewer role after sync. |
| `L6-T-sync_propagation-004` | T2/T3 | Viewer role cannot read BidScopePayload, run manifest body, company-profile details, or raw portal artifact paths. |
| `L6-T-sync_propagation-005` | T2 | Remote-owned company profile and operator decision heads are pulled before readiness evaluation and never written by L6. |

### 7.10 Desktop UI

| Test ID | Tier | Test |
|---|---|---|
| `L6-T-desktop_ui-001` | T3 | Start Bid control is visible only for eligible `ready_to_bid` opportunities. |
| `L6-T-desktop_ui-002` | T3 | Blocked Start Bid path renders all readiness blockers with actionable messages and no job enqueue. |
| `L6-T-desktop_ui-003` | T3 | Quote-role override UI requires structured acknowledgment and writes an audit entry. |
| `L6-T-desktop_ui-004` | T3 | `draft_saved` review panel renders draft reference, screenshots, manifest link, and next allowed actions. |
| `L6-T-desktop_ui-005` | T3 | Validation override path requires a non-empty structured reason and records it before final submission. |
| `L6-T-desktop_ui-006` | T3 | SessionViewer opens the bid workflow PTY/manifest evidence without leaking redacted payloads. |
| `L6-T-desktop_ui-007` | T3 | Formal submission controls remain disabled unless sideEffectCap and operator authorization explicitly allow them. |

### 7.11 Web UI

| Test ID | Tier | Test |
|---|---|---|
| `L6-T-web_ui-001` | T3 | Viewer role sees bid-state badge and timestamp for synced opportunities. |
| `L6-T-web_ui-002` | T3 | Viewer role cannot see BidScopePayload, company-profile fields, run manifest body, or portal artifact paths. |
| `L6-T-web_ui-003` | T3 | Viewer role has no bid mutation controls and direct mutation requests are rejected. |
| `L6-T-web_ui-004` | T3 | `bid_failed` summary displays redacted code and actionable category without PII leakage. |
| `L6-T-web_ui-005` | T3 | Realtime or refresh path updates bid-state badge after local-to-remote sync. |

### 7.12 Live Demo

| Test ID | Tier | Test |
|---|---|---|
| `L6-T-live_demo-001` | T4 | Operator selects a Procontract opportunity reference, confirms it will not create buyer-visible legal commitment under `REMOTE_DRAFT`, and records profile/version prerequisites. |
| `L6-T-live_demo-002` | T4 | Live run with `sideEffectCap=REMOTE_DRAFT` reaches `draft_saved`, records screenshot trail, run manifest, remote draft reference, and Supabase viewer badge. |
| `L6-T-live_demo-003` | T4 | Optional formal-submission smoke runs only with separate written operator authorization and reaches `bid_submitted` with confirmation reference. |
| `L6-T-live_demo-004` | T4 | Demo diary records commands, CDP health, opportunity reference, manifest path, validator verdict, sync evidence, and final state. |

## 8. Audit Tables

### 8.1 Error-Code Decision Table

| Failure mode | Code | Reason |
|---|---|---|
| Final submission failure after portal interaction | `BID_SUBMISSION_FAILED` | L6-specific side-effecting workflow failure with bid context. |
| Remote draft/submission duplicate detected | `BID_DUPLICATE_DETECTED` | Requires special retry safety and state reconciliation. |
| Skill attempts side effect above cap | `BID_SIDE_EFFECT_CAP_EXCEEDED` | Constitutional cap guard; existing generic skill failures are too broad. |
| Readiness gate blocks L6 start/resume/submit | `BID_READINESS_GATE_FAILED` | Operator-facing gate failure, not source drift or skill crash. |
| Portal HTML/selector drift | `PAGE_SHAPE_DRIFT` | Existing code already models page shape drift with source context. |
| Procontract auth expired | `AUTH_STATE_STALE` | Existing auth-staleness code is correct; daemon performs retry. |

### 8.2 Readiness Gate Override Matrix

| Gate | Overrideable | Test IDs |
|---|---|---|
| Approved operator decision exists | No | `L6-T-bid_readiness_gate-001`, `016` |
| Fresh `decisionBasisHash` | No | `L6-T-bid_readiness_gate-002`, `013`, `016` |
| `CLASSIFICATION_AMBIGUOUS` absent | No | `L6-T-bid_readiness_gate-003`, `016` |
| `TARIFF_BRANCH_DIVERGENCE` absent | No | `L6-T-bid_readiness_gate-004`, `016` |
| `SUPPLEMENTARY_UNIT_MISSING` absent | No | `L6-T-bid_readiness_gate-005`, `016` |
| `broker_required` absent | No | `L6-T-bid_readiness_gate-006`, `016` |
| Classification freshness current | No | `L6-T-bid_readiness_gate-007`, `016` |
| Company profile current | No | `L6-T-bid_readiness_gate-008`, `016` |
| Quote-role quality gate | Yes | `L6-T-bid_readiness_gate-009`, `010` |
| Upstream trust validated | No | `L6-T-bid_readiness_gate-011`, `016` |
| Validator rejected/outage before final submit | Yes, with structured final-submit override | `L6-T-validator_overlay-008`, `009`, `010`, `011` |

### 8.3 Side-Effect Class Map

| Class | Representative playbook steps | Cap behavior |
|---|---|---|
| `READ_ONLY` | Attach CDP, authenticate check, search opportunity, read form. | Allowed by every cap. |
| `REMOTE_DRAFT` | Fill form, upload documents, save draft. | Default live-demo maximum. |
| `NOTIFICATION_SENT` | Any portal action that sends buyer/supplier notification. | Above default cap, must be explicit. |
| `LEGAL_ACKNOWLEDGED` | Accept terms or legal declarations. | Above default cap, must be explicit. |
| `FORMAL_SUBMISSION` | Click final submit and receive confirmation reference. | Requires operator authorization and fail-closed validator. |
| `FUNDS_COMMITTED` | Any paid tender, bond, or fee commitment. | Out of scope unless future spec authorizes it. |

### 8.4 Calibration Corpus Contract

Each directory under `tests/fixtures/calibration/bid-portal-procontract/` must contain:

- `canary.inputs.jsonl`
- `golden.assertions.yaml`
- `metadata.json`
- `manifest.md`

Required variants:

- `simple`
- `medium`
- `complex`

The corpus must include at least one cassette or captured DOM/screenshot bundle per
variant. The fast tier replays the captured material. The smoke tier attaches to
the persistent CDP browser.

### 8.5 Redaction Contract

The following sentinel classes must be injected by tests and absent from public
or read-only surfaces:

- BidScopePayload process IDs, lot IDs, candidate IDs, quote IDs, and portal refs.
- Company registration number, address, directors, email, phone, certificate IDs,
  bank/tax identifiers, and signer names.
- Raw portal form field values and uploaded document paths.
- Run manifest body and forensic PTY byte paths for viewer role.

They may appear only in authorized local daemon artifacts that are already covered
by artifact storage and redaction policy.

## 9. Coverage Matrix

| Requirement | Test IDs |
|---|---|
| `L6-FR-1.1.1` | `L6-T-bid_portal_procontract_skill-001`, `L6-T-live_demo-002` |
| `L6-FR-1.1.2` | `L6-T-bid_portal_procontract_skill-006`, `L6-T-daemon_runner-005` |
| `L6-FR-1.1.3` | `L6-T-bid_readiness_gate-012`, `L6-T-bid_workflow_states-002` |
| `L6-FR-1.1.4` | `L6-T-daemon_runner-007`, `L6-T-daemon_runner-008`, `L6-T-validator_overlay-003` |
| `L6-FR-1.2.1` | `L6-T-bid_portal_procontract_skill-004`, `L6-T-bid_portal_procontract_skill-006`, `L6-T-daemon_runner-004` |
| `L6-FR-1.2.2` | `L6-T-bid_readiness_gate-008`, `L6-T-bid_portal_procontract_skill-007`, `L6-T-daemon_runner-005` |
| `L6-FR-1.2.3` | `L6-T-bid_portal_procontract_skill-007`, `L6-T-portal_side_effects-002` |
| `L6-FR-1.2.4` | `L6-T-bid_portal_procontract_skill-008`, `L6-T-portal_side_effects-012` |
| `L6-FR-1.2.5` | `L6-T-validator_overlay-003`, `L6-T-bid_portal_procontract_skill-011`, `L6-T-daemon_runner-007` |
| `L6-FR-1.2.6` | `L6-T-daemon_runner-007`, `L6-T-validator_overlay-003` |
| `L6-FR-1.2.7` | `L6-T-daemon_runner-008`, `L6-T-validator_overlay-004` |
| `L6-FR-1.2.8` | `L6-T-error_codes-004`, `L6-T-portal_side_effects-012` |
| `L6-FR-1.3.1` | `L6-T-bid_workflow_states-001`, `L6-T-bid_workflow_states-034` |
| `L6-FR-1.3.2` | `L6-T-bid_workflow_states-002`, `L6-T-bid_workflow_states-003` |
| `L6-FR-1.3.3` | `L6-T-bid_workflow_states-004`, `L6-T-bid_workflow_states-005` |
| `L6-FR-1.3.4` | `L6-T-bid_workflow_states-006`, `L6-T-bid_workflow_states-007` |
| `L6-FR-1.3.5` | `L6-T-bid_workflow_states-008`, `L6-T-bid_workflow_states-009` |
| `L6-FR-1.3.6` | `L6-T-bid_workflow_states-010`, `L6-T-bid_workflow_states-011` |
| `L6-FR-1.3.7` | `L6-T-bid_workflow_states-012`, `L6-T-bid_workflow_states-013` |
| `L6-FR-1.3.8` | `L6-T-bid_workflow_states-014`, `L6-T-bid_workflow_states-015` |
| `L6-FR-1.3.9` | `L6-T-bid_workflow_states-016`, `L6-T-bid_workflow_states-017` |
| `L6-FR-1.3.10` | `L6-T-bid_workflow_states-018`, `L6-T-bid_workflow_states-019` |
| `L6-FR-1.3.11` | `L6-T-bid_workflow_states-020`, `L6-T-bid_workflow_states-021` |
| `L6-FR-1.3.12` | `L6-T-bid_workflow_states-022`, `L6-T-bid_workflow_states-023` |
| `L6-FR-1.3.13` | `L6-T-bid_workflow_states-024`, `L6-T-bid_workflow_states-025` |
| `L6-FR-1.3.14` | `L6-T-bid_workflow_states-026`, `L6-T-bid_workflow_states-027` |
| `L6-FR-1.3.15` | `L6-T-bid_workflow_states-028`, `L6-T-bid_workflow_states-029` |
| `L6-FR-1.3.16` | `L6-T-bid_readiness_gate-001`, `L6-T-ipc_trigger_bid_workflow-002` |
| `L6-FR-1.3.17` | `L6-T-bid_readiness_gate-002`, `L6-T-bid_workflow_states-031` |
| `L6-FR-1.3.18` | `L6-T-bid_readiness_gate-003`, `L6-T-bid_readiness_gate-004`, `L6-T-bid_readiness_gate-005`, `L6-T-bid_readiness_gate-006` |
| `L6-FR-1.3.19` | `L6-T-bid_readiness_gate-007` |
| `L6-FR-1.3.20` | `L6-T-bid_readiness_gate-009`, `L6-T-bid_readiness_gate-010` |
| `L6-FR-1.3.21` | `L6-T-bid_readiness_gate-008` |
| `L6-FR-1.3.22` | `L6-T-bid_readiness_gate-011` |
| `L6-FR-1.3.23` | `L6-T-validator_overlay-008`, `L6-T-validator_overlay-009`, `L6-T-validator_overlay-010`, `L6-T-bid_workflow_states-010` |
| `L6-FR-1.3.24` | `L6-T-bid_workflow_states-001` |
| `L6-FR-1.3.25` | `L6-T-bid_workflow_states-002`, `L6-T-bid_workflow_states-003`, `L6-T-bid_readiness_gate-012` |
| `L6-FR-1.3.26` | `L6-T-bid_workflow_states-004`, `L6-T-bid_workflow_states-030` |
| `L6-FR-1.3.27` | `L6-T-bid_workflow_states-008`, `L6-T-portal_side_effects-005`, `L6-T-desktop_ui-004` |
| `L6-FR-1.3.28` | `L6-T-bid_workflow_states-010`, `L6-T-bid_workflow_states-024`, `L6-T-validator_overlay-010` |
| `L6-FR-1.3.29` | `L6-T-bid_workflow_states-012`, `L6-T-bid_workflow_states-020`, `L6-T-bid_workflow_states-026` |
| `L6-FR-1.3.30` | `L6-T-bid_workflow_states-014`, `L6-T-bid_workflow_states-022`, `L6-T-bid_workflow_states-024`, `L6-T-bid_workflow_states-026` |
| `L6-FR-1.4.1` | `L6-T-bid_workflow_states-030`, `L6-T-bid_readiness_gate-001` |
| `L6-FR-1.4.2` | `L6-T-bid_workflow_states-030` |
| `L6-FR-1.4.3` | `L6-T-bid_workflow_states-030` |
| `L6-FR-1.4.4` | `L6-T-bid_workflow_states-030`, `L6-T-bid_readiness_gate-009` |
| `L6-FR-1.4.5` | `L6-T-bid_readiness_gate-008`, `L6-T-bid_portal_procontract_skill-007` |
| `L6-FR-1.4.6` | `L6-T-bid_portal_procontract_skill-001`, `L6-T-daemon_runner-005` |
| `L6-FR-1.4.7` | `L6-T-bid_readiness_gate-014`, `L6-T-bid_portal_procontract_skill-006` |
| `L6-FR-1.4.8` | `L6-T-bid_workflow_states-032`, `L6-T-desktop_ui-007`, `L6-T-live_demo-002` |
| `L6-FR-1.4.9` | `L6-T-bid_workflow_states-031` |
| `L6-FR-1.4.10` | `L6-T-bid_workflow_states-032`, `L6-T-portal_side_effects-004`, `L6-T-portal_side_effects-005`, `L6-T-portal_side_effects-006` |
| `L6-FR-1.4.11` | `L6-T-portal_side_effects-001` |
| `L6-FR-1.4.12` | `L6-T-bid_workflow_states-033`, `L6-T-sync_propagation-002` |
| `L6-FR-1.5.1` | `L6-T-desktop_ui-001`, `L6-T-ipc_trigger_bid_workflow-002` |
| `L6-FR-1.5.2` | `L6-T-ipc_trigger_bid_workflow-001`, `L6-T-desktop_ui-001` |
| `L6-FR-1.5.3` | `L6-T-ipc_trigger_bid_workflow-003`, `L6-T-daemon_runner-006` |
| `L6-FR-1.5.4` | `L6-T-daemon_runner-006`, `L6-T-bid_workflow_states-030` |
| `L6-FR-1.5.5` | `L6-T-daemon_runner-004` |
| `L6-FR-1.5.6` | `L6-T-daemon_runner-004`, `L6-T-bid_portal_procontract_skill-001` |
| `L6-FR-1.5.7` | `L6-T-daemon_runner-005`, `L6-T-bid_portal_procontract_skill-007` |
| `L6-FR-1.5.8` | `L6-T-bid_portal_procontract_skill-006` |
| `L6-FR-1.5.9` | `L6-T-bid_portal_procontract_skill-005`, `L6-T-daemon_runner-009` |
| `L6-FR-1.5.10` | `L6-T-bid_portal_procontract_skill-006` |
| `L6-FR-1.5.11` | `L6-T-bid_portal_procontract_skill-007`, `L6-T-portal_side_effects-002` |
| `L6-FR-1.5.12` | `L6-T-bid_portal_procontract_skill-008`, `L6-T-portal_side_effects-012` |
| `L6-FR-1.5.13` | `L6-T-bid_portal_procontract_skill-011`, `L6-T-validator_overlay-003` |
| `L6-FR-1.5.14` | `L6-T-portal_side_effects-005`, `L6-T-portal_side_effects-006`, `L6-T-bid_portal_procontract_skill-009`, `L6-T-bid_portal_procontract_skill-010` |
| `L6-FR-1.5.15` | `L6-T-daemon_runner-007` |
| `L6-FR-1.5.16` | `L6-T-daemon_runner-008` |
| `L6-FR-1.5.17` | `L6-T-bid_workflow_states-010`, `L6-T-validator_overlay-008`, `L6-T-validator_overlay-010` |
| `L6-FR-1.5.18` | `L6-T-error_codes-004`, `L6-T-portal_side_effects-012`, `L6-T-bid_workflow_states-012` |
| `L6-FR-1.5.19` | `L6-T-sync_propagation-001`, `L6-T-sync_propagation-002`, `L6-T-sync_propagation-003` |
| `L6-FR-1.6.1` | `L6-T-bid_portal_procontract_skill-002` |
| `L6-FR-1.6.2` | `L6-T-bid_portal_procontract_skill-006`, `L6-T-bid_portal_procontract_skill-007`, `L6-T-bid_portal_procontract_skill-008`, `L6-T-bid_portal_procontract_skill-009`, `L6-T-bid_portal_procontract_skill-010` |
| `L6-FR-1.6.3` | `L6-T-bid_readiness_gate-008`, `L6-T-bid_portal_procontract_skill-007` |
| `L6-FR-1.6.4` | `L6-T-validator_overlay-001`, `L6-T-validator_overlay-002`, `L6-T-validator_overlay-003`, `L6-T-validator_overlay-004`, `L6-T-validator_overlay-005`, `L6-T-validator_overlay-006`, `L6-T-validator_overlay-007` |
| `L6-FR-1.6.5` | `L6-T-daemon_runner-001` |
| `L6-FR-1.6.6` | `L6-T-error_codes-001`, `L6-T-error_codes-002`, `L6-T-error_codes-003`, `L6-T-error_codes-004` |
| `L6-FR-1.7.1` | `L6-T-bid_portal_procontract_skill-001`, `L6-T-bid_portal_procontract_skill-004` |
| `L6-FR-1.7.2` | `L6-T-daemon_runner-004`, `L6-T-bid_portal_procontract_skill-004` |
| `L6-FR-1.7.3` | `L6-T-daemon_runner-002`, `L6-T-bid_portal_procontract_skill-001` |
| `L6-FR-1.7.4` | `L6-T-validator_overlay-008`, `L6-T-validator_overlay-009` |
| `L6-FR-1.7.5` | `L6-T-bid_portal_procontract_skill-005`, `L6-T-daemon_runner-009` |
| `L6-FR-1.7.6` | `L6-T-portal_side_effects-008`, `L6-T-portal_side_effects-010` |
| `L6-FR-1.7.7` | `L6-T-bid_portal_procontract_skill-004`, `L6-T-portal_side_effects-002` |
| `L6-FR-1.7.8` | `L6-T-bid_portal_procontract_skill-005`, `L6-T-daemon_runner-009` |
| `L6-FR-1.7.9` | `L6-T-bid_portal_procontract_skill-006` |
| `L6-FR-1.7.10` | `L6-T-bid_portal_procontract_skill-006`, `L6-T-portal_side_effects-009` |
| `L6-FR-1.7.11` | `L6-T-bid_portal_procontract_skill-006`, `L6-T-portal_side_effects-002` |
| `L6-FR-1.7.12` | `L6-T-bid_portal_procontract_skill-007`, `L6-T-portal_side_effects-002` |
| `L6-FR-1.7.13` | `L6-T-bid_portal_procontract_skill-007` |
| `L6-FR-1.7.14` | `L6-T-bid_portal_procontract_skill-008`, `L6-T-portal_side_effects-012` |
| `L6-FR-1.7.15` | `L6-T-bid_portal_procontract_skill-009` |
| `L6-FR-1.7.16` | `L6-T-bid_portal_procontract_skill-009`, `L6-T-portal_side_effects-005` |
| `L6-FR-1.7.17` | `L6-T-bid_portal_procontract_skill-010`, `L6-T-portal_side_effects-006`, `L6-T-validator_overlay-008`, `L6-T-validator_overlay-010` |
| `L6-FR-1.7.18` | `L6-T-bid_portal_procontract_skill-010`, `L6-T-validator_overlay-005` |
| `L6-FR-1.7.19` | `L6-T-portal_side_effects-002`, `L6-T-portal_side_effects-003` |
| `L6-FR-1.7.20` | `L6-T-portal_side_effects-004`, `L6-T-portal_side_effects-005`, `L6-T-error_codes-004` |
| `L6-FR-1.8.1` | `L6-T-validator_overlay-003`, `L6-T-daemon_runner-007` |
| `L6-FR-1.8.2` | `L6-T-daemon_runner-007` |
| `L6-FR-1.8.3` | `L6-T-daemon_runner-008` |
| `L6-FR-1.8.4` | `L6-T-sync_propagation-002`, `L6-T-bid_workflow_states-033` |
| `L6-FR-1.8.5` | `L6-T-error_codes-004`, `L6-T-portal_side_effects-012` |
| `L6-FR-1.8.6` | `L6-T-validator_overlay-005`, `L6-T-bid_workflow_states-024` |
| `L6-FR-1.9.1` | `L6-T-sync_propagation-001`, `L6-T-sync_propagation-002` |
| `L6-FR-1.9.2` | `L6-T-bid_workflow_states-034` |
| `L6-FR-1.9.3` | `L6-T-web_ui-001`, `L6-T-web_ui-002`, `L6-T-web_ui-003`, `L6-T-sync_propagation-003` |
| `L6-FR-1.10.1` | `L6-T-bid_readiness_gate-001`, `L6-T-ipc_trigger_bid_workflow-002` |
| `L6-FR-1.10.2` | `L6-T-portal_side_effects-004`, `L6-T-portal_side_effects-005`, `L6-T-portal_side_effects-006` |
| `L6-FR-1.10.3` | `L6-T-bid_workflow_states-030`, `L6-T-ipc_trigger_bid_workflow-004` |
| `L6-FR-1.10.4` | `L6-T-daemon_runner-007`, `L6-T-daemon_runner-008`, `L6-T-validator_overlay-003` |
| `L6-FR-1.10.5` | `L6-T-bid_workflow_states-031`, `L6-T-bid_readiness_gate-002` |
| `L6-FR-1.10.6` | `L6-T-bid_readiness_gate-003`, `L6-T-bid_readiness_gate-004`, `L6-T-bid_readiness_gate-005`, `L6-T-bid_readiness_gate-006`, `L6-T-bid_readiness_gate-007` |
| `L6-FR-1.10.7` | `L6-T-validator_overlay-008`, `L6-T-validator_overlay-009`, `L6-T-validator_overlay-010` |
| `L6-FR-1.10.8` | `L6-T-bid_readiness_gate-009`, `L6-T-bid_readiness_gate-010`, `L6-T-validator_overlay-011` |
| `L6-FR-2.1.1` | `L6-T-live_demo-002`, `L6-T-bid_portal_procontract_skill-004`, `L6-T-bid_portal_procontract_skill-006` |
| `L6-FR-2.1.2` | `L6-T-bid_portal_procontract_skill-007`, `L6-T-daemon_runner-005` |
| `L6-FR-2.1.3` | `L6-T-bid_portal_procontract_skill-007`, `L6-T-bid_portal_procontract_skill-008`, `L6-T-portal_side_effects-002` |
| `L6-FR-2.1.4` | `L6-T-bid_portal_procontract_skill-010`, `L6-T-validator_overlay-003` |
| `L6-FR-2.1.5` | `L6-T-daemon_runner-007` |
| `L6-FR-2.1.6` | `L6-T-daemon_runner-008` |
| `L6-FR-2.1.7` | `L6-T-error_codes-004`, `L6-T-portal_side_effects-012` |
| `L6-FR-2.1.8` | `L6-T-live_demo-002`, `L6-T-live_demo-003` |
| `L6-FR-2.1.9` | `L6-T-desktop_ui-004`, `L6-T-bid_workflow_states-008`, `L6-T-bid_workflow_states-016`, `L6-T-live_demo-002` |
| `L6-FR-2.1.10` | `L6-T-bid_readiness_gate-003`, `L6-T-bid_readiness_gate-004`, `L6-T-bid_readiness_gate-005`, `L6-T-bid_readiness_gate-006`, `L6-T-bid_readiness_gate-007` |
| `L6-FR-2.1.11` | `L6-T-validator_overlay-008`, `L6-T-validator_overlay-009` |

## 10. Coverage Summary

| Metric | Count |
|---|---:|
| Unique spec requirements | 127 |
| Requirements mapped to at least one test | 127 |
| Unmapped requirements | 0 |
| Requirement-bearing `L6-T-*` IDs | 127 |
| Static CI gates | 10 |
| New package test files | 3 |
| Cumulative Rust integration plus package test files after L6 package tests | 60 |

## 11. Notes For Strategy And Beads

1. The implementation layer should create the three package test files first, then
   the daemon extensions, then desktop/web E2E, then smoke evidence.
2. Error-code changes are intentionally limited to four new variants. Any future
   fifth or sixth variant must update `L6-T-error_codes-001` and this TDD.
3. The validator overlay override path is narrow: it applies only to final-submit
   validator rejection/outage with structured operator reason. It does not weaken
   readiness gates.
4. The live demo is a proof of the skill-first pipeline under `REMOTE_DRAFT`; it
   is not authority to run formal submission.
5. Completion audit must rerun the full daemon integration loop, not just
   `cargo test -p daemon --bin daemon`.
