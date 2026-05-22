# L6 Request Pipeline Automation — Bead Bulk Import

> **Authority:** This document is a human-readable manifest for the L6 implementation bead inventory. Each task bead's full self-contained description is stored in the beads database; this doc lists IDs + aliases + spec/TDD coverage to enable rapid scanning and audit.
>
> **Source documents:**
> - Spec (SOT #1): `docs/strategy/layers/layer-6-request-pipeline-automation.md` (127 L6-FR-* requirements, §1.0-§2.8)
> - TDD (SOT #2): `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md` (153 stable L6-T-* assertion IDs + 10 CI-L6-* gates)
> - Strategy (SOT #3, ordering only): `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md`
> - Bootstrap: `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md`
> - Manifest: `docs/pipeline/l6-request-pipeline-automation/manifest.yaml`
>
> **Bead totals:** 1 sister implementation epic + 7 phase sub-epics + 63 task beads = **71 beads**.
>
> **Bead CLI:** `br` (NOT `bd`). Sync: `bin/br-sync` then manual `git add .beads/ && git commit && git push`. Claim/complete: `bin/br-claim <bead-id>` / `bin/br-complete <bead-id>` per AGENTS.md Rule 2.
>
> **Parent epic:** `testproject1-3yqf` (Epic: L6 Implementation — Request Pipeline Automation), sister to the closed L6 documentation epic `testproject1-rc0` per Decision 4 recorded 2026-05-21.
>
> **Coverage assertion:** every L6-FR-X.Y.Z appears in at least one bead's `spec_ids` list (verified 127/127). Every numbered L6-T-* test ID has primary ownership in exactly one bead (verified 153/153). Trace artifact: `l6-bead-alias-map.json`.

---

## Root Sister Implementation Epic

### `testproject1-3yqf` — Epic: L6 Implementation — Request Pipeline Automation

- **Type:** epic
- **Priority:** P0
- **Labels:** `implementation`, `l6`, `layer-6`
- **Parent:** (top-level)

Threads all Phase 1-7 implementation beads for L6. The L6 layer doc was produced under `testproject1-rc0` (now closed 2026-05-22 as "Layer doc produced; implementation tracked in sister epic" — Decision 4).

**Phase fan-out:**

| Phase | Epic ID | Name | Task Beads |
|-------|---------|------|------------|
| 1 | `testproject1-3yqf.1` | Foundation: Vocabulary, Types, Registry, Skeleton | 9 |
| 2 | `testproject1-3yqf.2` | Daemon Runtime Entry Points | 7 |
| 3 | `testproject1-3yqf.3` | Bid Workflow State Machine And Readiness Gate | 11 |
| 4 | `testproject1-3yqf.4` | Portal Skill | 10 |
| 5 | `testproject1-3yqf.5` | Validator Overlay And Bidding Package Logic | 10 |
| 6 | `testproject1-3yqf.6` | Desktop And Web Surfaces | 9 |
| 7 | `testproject1-3yqf.7` | Live Demo, Calibration, And Integration Closure | 7 |
| Total | — | — | **63 task beads** + 7 phase sub-epics + 1 sister epic = **71** |

**Operator Decisions (locked 2026-05-21, transcribed in spec §1.0 verbatim):**
1. Proactis === Procontract (naming reconciliation).
2. Reuse `skills/auth-signin/procontract/` via daemon-orchestrated AUTH_STATE_STALE retry.
3. Validator overlay included as L6 deliverable.
4. Sister implementation epic structure (this epic; rc0 closed as doc-only).
5. Procontract access already solved; first live demo uses `sideEffectCap=REMOTE_DRAFT`.

**Exit criteria:** All 7 phase review gates green; CI-L6-001..010 green; 11 DoD rows green; live REMOTE_DRAFT + operator-authorized FORMAL_SUBMISSION evidence; 100% L6-FR coverage (127/127) + 100% numbered L6-T coverage (153/153).

---

## Full L6-FR Coverage Cross-Reference (127 IDs)

This table maps every spec requirement to its primary-owning bead(s). Multi-bead coverage occurs when an FR spans daemon + package + UI surfaces; in that case the FR is covered by the EARLIEST PHASE bead (the foundational implementation) plus integration verification beads in later phases.

### §1.0 Recorded Decisions
All 5 decisions are AC-bearing rows transcribed into spec §1.0 by step 01. No separate bead; the decisions inform Phase 1-7 routing across all beads.

### §1.1 Layer Narrative
- L6-FR-1.1.1 → `4.10` (portal skill capstone), `7.2` (live demo proves), `7.3` (formal submission proves)
- L6-FR-1.1.2 → `4.5` (opportunity lookup verifies separate notice source vs portal)
- L6-FR-1.1.3 → `7.2` (gated by L5 intelligence proof)
- L6-FR-1.1.4 → `3.8` (audit trail in runner body), `3.9` (manifest appendix), `7.2`/`7.3` (live audit)

### §1.2 Capability Delivered
- L6-FR-1.2.1..1.2.8 → primarily Phase 4 (`4.1` through `4.10`) for skill capability + `7.2`/`7.3` for end-to-end evidence

### §1.3 State Machine — 30 IDs
- L6-FR-1.3.1 → `3.1` (lifecycle enum)
- L6-FR-1.3.2..1.3.15 (14 transition rows) → `3.2` (state machine transitions)
- L6-FR-1.3.16 (operator approved) → `3.5` (readiness evaluator)
- L6-FR-1.3.17 (decision fresh) → `3.4` (decision-basis hash), `3.5` (readiness)
- L6-FR-1.3.18 (L5 blockers absent) → `3.5` (readiness evaluator)
- L6-FR-1.3.19 (classification freshness) → `3.5` (readiness)
- L6-FR-1.3.20 (quote-role) → `3.5` (readiness), `6.4` (override UI)
- L6-FR-1.3.21 (company profile current) → `3.5` (readiness), `5.6` (schema)
- L6-FR-1.3.22 (upstream trust) → `3.5` (readiness)
- L6-FR-1.3.23 (validation fail-closed) → `3.2` (state machine final-submit guard), `5.2` (validator failure modes), `6.4` (validation override UI)
- L6-FR-1.3.24..1.3.30 (7 state ACs) → `3.1` (enum), `3.2` (transitions), `5.7` (package test)

### §1.4 BidScopePayload — 12 IDs
- L6-FR-1.4.1..1.4.12 → `1.7` (TS types), `3.3` (Rust validation), `5.7` (package test)

### §1.5 Workflow Flow — 19 IDs
- L6-FR-1.5.1 → `2.4` (IPC handler), `6.3` (desktop Start Bid)
- L6-FR-1.5.2 → `2.4` (IPC), `6.2` (DaemonBridge)
- L6-FR-1.5.3 → `2.1` (dispatch arm), `2.3` (enqueue builder)
- L6-FR-1.5.4 → `2.2` (handler shape), `3.3` (scope validation)
- L6-FR-1.5.5 → `2.1` (dispatch), `2.2` (PTY profile assertion)
- L6-FR-1.5.6 → `2.2` (skill loading)
- L6-FR-1.5.7..1.5.18 → `3.8` (runner body — full flow), `4.3`..`4.7` (skill playbook)
- L6-FR-1.5.19 → `6.6` (web bid-state badge), `6.8` (sync visibility)

### §1.6 Packages/Bidding — 6 IDs
- L6-FR-1.6.1 → `5.4` (workflow-registry.ts)
- L6-FR-1.6.2 → `5.5` (procontract-workflow.ts)
- L6-FR-1.6.3 → `5.6` (company-profile-schema.ts)
- L6-FR-1.6.4 → `5.1` (overlay frontmatter+assertions)
- L6-FR-1.6.5 → `1.4` (module skeleton)
- L6-FR-1.6.6 → `1.1` (enum), `1.2` (exhaustive consumers)

### §1.7 Bid Portal Skill — 20 IDs
- L6-FR-1.7.1..1.7.6 → `4.1` (frontmatter)
- L6-FR-1.7.7..1.7.18 (11 playbook steps) → `4.3` (playbook body), `4.4`..`4.7` (per-step beads)
- L6-FR-1.7.19..1.7.20 (side-effect rules) → `3.6` (runtime cap guard), `5.8` (package test), `4.7` (skill steps)

### §1.8 Audit Trail — 6 IDs
- L6-FR-1.8.1..1.8.6 → `3.8` (runner body — screenshots+PTY+manifest), `3.9` (manifest appendix), `3.10` (redaction)

### §1.9 Data Changes — 3 IDs
- L6-FR-1.9.1 → `1.8` (sync push-set check)
- L6-FR-1.9.2 → `1.8` (no SQL CHECK), `3.1` (Rust enforcement)
- L6-FR-1.9.3 → `6.6` (web badge), `6.7` (RLS grid), `3.10` (daemon redaction)

### §1.10 Invariants — 8 IDs
- L6-FR-1.10.1 (no real-money bids without approval) → `2.4` (IPC pre-check), `3.5` (readiness evaluator)
- L6-FR-1.10.2 (no escalation above cap) → `3.6` (cap guard), `4.7` (skill), `5.8` (package test)
- L6-FR-1.10.3 (explicit scope) → `1.7` (types), `3.3` (validation)
- L6-FR-1.10.4 (full audit trail) → `3.8` (runner), `3.9` (manifest), `4.7` (screenshots), `5.1` (overlay completeness)
- L6-FR-1.10.5 (stale decisions block) → `3.4` (hash), `3.5` (readiness)
- L6-FR-1.10.6 (L5 readiness gate) → `2.5` (precheck wiring), `3.5` (readiness)
- L6-FR-1.10.7 (fail-closed validation) → `3.2` (state machine), `5.2` (validator failure modes), `6.4` (override UI)
- L6-FR-1.10.8 (quote-role quality) → `3.5` (readiness), `6.4` (override UI)

### §2.1 DoD — 11 IDs
- L6-FR-2.1.1..2.1.11 → `7.6` (DoD walkthrough) cites the test IDs that prove each; live demo evidence in `7.2`/`7.3` covers rows 8/9/11

---

## Phase 1: Foundation — 9 Task Beads

**Epic:** `testproject1-3yqf.1` (`l6-p1-foundation-review-gate` is the closing bead).

| Bead | ID | Spec Refs | Test IDs Owned |
|------|----|-----------|----------------|
| `l6-p1-error-vocabulary-enum` | `testproject1-3yqf.1.1` | L6-FR-1.6.6 | L6-T-error_codes-001/002/006-017 (14 IDs — enum + round-trip) |
| `l6-p1-error-exhaustive-consumers` | `testproject1-3yqf.1.2` | L6-FR-1.6.6 | L6-T-error_codes-003/004/005/018-023 (9 IDs — propagation) |
| `l6-p1-model-profile-registration` | `testproject1-3yqf.1.3` | L6-FR-1.7.3 | L6-T-daemon_runner-002 |
| `l6-p1-bid-workflow-module-skeleton` | `testproject1-3yqf.1.4` | L6-FR-1.6.5 | L6-T-daemon_runner-001 |
| `l6-p1-source-registry-entries` | `testproject1-3yqf.1.5` | L6-FR-1.7.1, 1.7.4 | (cross-phase: enables 4.x/5.x) |
| `l6-p1-bidding-package-scaffold` | `testproject1-3yqf.1.6` | L6-FR-1.6.1, 1.6.2, 1.6.3 | (scaffolding — enables CI-L6-006/007/008) |
| `l6-p1-bid-scope-types-ts` | `testproject1-3yqf.1.7` | L6-FR-1.4.1..1.4.12 | (TS types — used by package tests) |
| `l6-p1-sync-push-set-check` | `testproject1-3yqf.1.8` | L6-FR-1.9.1, 1.9.2 | L6-T-sync_propagation-001, L6-T-bid_workflow_states-034 |
| `l6-p1-foundation-review-gate` | `testproject1-3yqf.1.9` | aggregate | CI-L6-001/002/003 |

**Phase 1 dependencies:** all beads have `l6-p1-foundation-review-gate` as a blocking child (gate cannot close until all 8 task beads close). The gate then blocks all Phase 2 beads.

---

## Phase 2: Daemon Runtime Entry Points — 7 Task Beads

**Epic:** `testproject1-3yqf.2`.

| Bead | ID | Spec Refs | Test IDs Owned |
|------|----|-----------|----------------|
| `l6-p2-job-dispatch-arm` | `testproject1-3yqf.2.1` | L6-FR-1.5.3, 1.5.5 | L6-T-daemon_runner-003 |
| `l6-p2-job-handler-shape` | `testproject1-3yqf.2.2` | L6-FR-1.5.4, 1.5.5, 1.5.6 | L6-T-daemon_runner-004 |
| `l6-p2-enqueue-builder` | `testproject1-3yqf.2.3` | L6-FR-1.5.3 | (cross-phase: helper for 2.4) |
| `l6-p2-ipc-trigger-handler` | `testproject1-3yqf.2.4` | L6-FR-1.5.1, 1.5.2, 1.10.3 | L6-T-ipc_trigger_bid_workflow-001/002/003 |
| `l6-p2-readiness-precheck-wiring` | `testproject1-3yqf.2.5` | L6-FR-1.3.16, 1.10.6 | L6-T-ipc_trigger_bid_workflow-004/005 |
| `l6-p2-cli-manual-trigger` | `testproject1-3yqf.2.6` | (manifest) | L6-T-daemon_runner-010 |
| `l6-p2-daemon-runtime-review-gate` | `testproject1-3yqf.2.7` | aggregate | (aggregate gate) |

---

## Phase 3: State Machine And Readiness Gate — 11 Task Beads

**Epic:** `testproject1-3yqf.3`. This is the largest phase — daemon-side runtime closure of state machine + readiness + audit trail.

| Bead | ID | Spec Refs | Test IDs Owned |
|------|----|-----------|----------------|
| `l6-p3-lifecycle-enum-rust` | `testproject1-3yqf.3.1` | L6-FR-1.3.1, 1.3.24-1.3.30, 1.9.2 | L6-T-bid_workflow_states-001 |
| `l6-p3-state-machine-transitions` | `testproject1-3yqf.3.2` | L6-FR-1.3.2..1.3.15 (14 transitions) | L6-T-bid_workflow_states-002..029 (28 IDs daemon runtime) |
| `l6-p3-bid-scope-payload-validation` | `testproject1-3yqf.3.3` | L6-FR-1.4.1..1.4.12, 1.10.3 | L6-T-bid_workflow_states-030/032/033 |
| `l6-p3-decision-basis-hash` | `testproject1-3yqf.3.4` | L6-FR-1.4.9, 1.3.17, 1.10.5 | L6-T-bid_workflow_states-031, L6-T-bid_readiness_gate-002 |
| `l6-p3-readiness-evaluator` | `testproject1-3yqf.3.5` | L6-FR-1.3.16-1.3.22, 1.10.1/.6/.8 | L6-T-bid_readiness_gate-001/003..016 (15 IDs) |
| `l6-p3-side-effect-cap-guard` | `testproject1-3yqf.3.6` | L6-FR-1.7.19, 1.7.20, 1.10.2 | L6-T-portal_side_effects-004/005/006/007 (runtime cap guard) |
| `l6-p3-auth-stale-retry-orchestration` | `testproject1-3yqf.3.7` | L6-FR-1.5.9, 1.7.5/.8 | L6-T-daemon_runner-009 |
| `l6-p3-bid-runner-body` | `testproject1-3yqf.3.8` | L6-FR-1.5.7..1.5.18, 1.8.1/.2/.4, 1.10.4 | L6-T-daemon_runner-005/006/007/011/012 |
| `l6-p3-manifest-appendix-l6-fields` | `testproject1-3yqf.3.9` | L6-FR-1.5.16, 1.8.3, 1.10.4 | L6-T-daemon_runner-008 |
| `l6-p3-redaction-boundary-daemon` | `testproject1-3yqf.3.10` | L6-FR-1.9.3 | L6-T-redaction_boundary-001..006 (6 IDs) |
| `l6-p3-state-machine-review-gate` | `testproject1-3yqf.3.11` | aggregate | (aggregate gate) |

---

## Phase 4: Portal Skill — 10 Task Beads

**Epic:** `testproject1-3yqf.4`. Parallel with Phase 5 after Phase 3 worker contract gate.

| Bead | ID | Spec Refs | Test IDs Owned |
|------|----|-----------|----------------|
| `l6-p4-skill-frontmatter` | `testproject1-3yqf.4.1` | L6-FR-1.7.1..1.7.6 | L6-T-bid_portal_procontract_skill-001 |
| `l6-p4-result-checkpoint-schemas` | `testproject1-3yqf.4.2` | (manifest §1.6) | (cross-phase: enables 4.x + 5.4) |
| `l6-p4-playbook-cdp-discipline` | `testproject1-3yqf.4.3` | L6-FR-1.7.7..1.7.20 | L6-T-bid_portal_procontract_skill-004 |
| `l6-p4-auth-stale-return-path` | `testproject1-3yqf.4.4` | L6-FR-1.7.5, 1.7.8 | L6-T-bid_portal_procontract_skill-005 |
| `l6-p4-opportunity-lookup-verify` | `testproject1-3yqf.4.5` | L6-FR-1.5.10, 1.7.9-1.7.11 | L6-T-bid_portal_procontract_skill-006, L6-T-portal_side_effects-009 |
| `l6-p4-form-fill-document-upload` | `testproject1-3yqf.4.6` | L6-FR-1.5.11, 1.5.12, 1.7.12-1.7.14 | L6-T-bid_portal_procontract_skill-007/008, L6-T-portal_side_effects-012 |
| `l6-p4-draft-final-submit-paths` | `testproject1-3yqf.4.7` | L6-FR-1.5.14, 1.7.16-1.7.18, 1.3.5/.6 | L6-T-bid_portal_procontract_skill-009/010, L6-T-portal_side_effects-002/003 |
| `l6-p4-canary-corpus-shell` | `testproject1-3yqf.4.8` | (TDD §9.4) | L6-T-bid_portal_procontract_skill-003, L6-T-portal_side_effects-001/010/011 |
| `l6-p4-canary-golden-journey-harness` | `testproject1-3yqf.4.9` | (TDD §6.2) | L6-T-bid_portal_procontract_skill-011/012, L6-T-portal_side_effects-008 |
| `l6-p4-portal-skill-review-gate` | `testproject1-3yqf.4.10` | aggregate | (aggregate gate) |

---

## Phase 5: Validator Overlay And Bidding Package — 10 Task Beads

**Epic:** `testproject1-3yqf.5`. Parallel with Phase 4 after Phase 3 worker contract gate.

| Bead | ID | Spec Refs | Test IDs Owned |
|------|----|-----------|----------------|
| `l6-p5-overlay-frontmatter-assertions` | `testproject1-3yqf.5.1` | L6-FR-1.6.4, 1.7.4, 1.10.4 | L6-T-validator_overlay-001..007/011/012 (9 IDs) |
| `l6-p5-overlay-validator-failure-modes` | `testproject1-3yqf.5.2` | L6-FR-1.3.23, 1.10.7 | L6-T-validator_overlay-008/009/010 |
| `l6-p5-overlay-calibration-fixtures` | `testproject1-3yqf.5.3` | (TDD §9.4.2) | (calibration enables 5.1 assertions) |
| `l6-p5-workflow-registry-ts` | `testproject1-3yqf.5.4` | L6-FR-1.6.1 | L6-T-bid_portal_procontract_skill-002 (package side) |
| `l6-p5-procontract-workflow-ts` | `testproject1-3yqf.5.5` | L6-FR-1.6.2 | (TS implementation; package tests cover) |
| `l6-p5-company-profile-schema-ts` | `testproject1-3yqf.5.6` | L6-FR-1.6.3, 1.3.21 | (TS schema; package tests cover) |
| `l6-p5-bid-workflow-states-test-ts` | `testproject1-3yqf.5.7` | L6-FR-1.3.* + 1.4.* (package closure) | L6-T-bid_workflow_states-001..034 (34 IDs — PRIMARY HOME per TDD §6.1) |
| `l6-p5-portal-side-effects-test-ts` | `testproject1-3yqf.5.8` | L6-FR-1.4.10/.11, 1.7.19/.20, 1.10.2 | L6-T-portal_side_effects-001..012 (12 IDs — PRIMARY HOME) |
| `l6-p5-bid-readiness-gate-test-ts` | `testproject1-3yqf.5.9` | L6-FR-1.3.16..1.3.22, 1.10.6/.8 | L6-T-bid_readiness_gate-001..016 (16 IDs — PRIMARY HOME) |
| `l6-p5-validator-bidding-review-gate` | `testproject1-3yqf.5.10` | aggregate | CI-L6-006/007/008 |

**Test ownership note:** Per TDD §6.1, the three Phase 5 package test files (5.7, 5.8, 5.9) are the PRIMARY home for the bid_workflow_states / portal_side_effects / bid_readiness_gate test ID ranges. Phase 3 daemon-side runtime closure (3.1/3.2/3.5/3.6) exercises the SAME logical assertions at the Rust runtime layer via integration tests against the daemon, providing complementary coverage of the same IDs.

---

## Phase 6: Desktop And Web Surfaces — 9 Task Beads

**Epic:** `testproject1-3yqf.6`. Sequential after Phase 2 IPC + Phase 5 package contracts stable.

| Bead | ID | Spec Refs | Test IDs Owned |
|------|----|-----------|----------------|
| `l6-p6-shared-ui-components` | `testproject1-3yqf.6.1` | L6-FR-1.3.27, 1.9.3 | (shared lane — enables 6.x) |
| `l6-p6-desktop-bid-tab` | `testproject1-3yqf.6.2` | L6-FR-1.5.1, 1.8.4 | (tab shell — enables 6.3-6.5) |
| `l6-p6-desktop-start-bid-control` | `testproject1-3yqf.6.3` | L6-FR-1.5.1, 1.3.25, 1.10.1 | L6-T-desktop_ui-001/002 |
| `l6-p6-desktop-override-flows` | `testproject1-3yqf.6.4` | L6-FR-1.3.20, 1.3.23, 1.10.7/.8 | L6-T-desktop_ui-003/005 |
| `l6-p6-desktop-draft-review-session-viewer` | `testproject1-3yqf.6.5` | L6-FR-1.3.5/.9, 1.4.10, 1.5.14, 2.1.9 | L6-T-desktop_ui-004/006/007, CI-L6-009 |
| `l6-p6-web-bid-state-badge` | `testproject1-3yqf.6.6` | L6-FR-1.5.19, 1.9.3 | L6-T-web_ui-001 |
| `l6-p6-web-rls-grid` | `testproject1-3yqf.6.7` | L6-FR-1.9.3 | L6-T-web_ui-002/003/004, L6-T-sync_propagation-004, CI-L6-010 |
| `l6-p6-web-realtime-refresh` | `testproject1-3yqf.6.8` | L6-FR-1.5.19 | L6-T-web_ui-005, L6-T-sync_propagation-002/003/005 |
| `l6-p6-desktop-web-review-gate` | `testproject1-3yqf.6.9` | aggregate | CI-L6-009, CI-L6-010 |

---

## Phase 7: Live Demo, Calibration, And Integration Closure — 7 Task Beads

**Epic:** `testproject1-3yqf.7`. Sequential after Phase 6 review gate.

| Bead | ID | Spec Refs | Test IDs Owned |
|------|----|-----------|----------------|
| `l6-p7-preflight-evidence` | `testproject1-3yqf.7.1` | L6-FR-2.1.8 | L6-T-live_demo-001 |
| `l6-p7-live-demo-remote-draft` | `testproject1-3yqf.7.2` | L6-FR-1.1.1/.3/.4, 1.3.27, 2.1.1-2.1.6, 2.1.9 | L6-T-live_demo-002 |
| `l6-p7-live-demo-formal-submission` | `testproject1-3yqf.7.3` | L6-FR-1.1.1, 1.3.28, 2.1.8 | L6-T-live_demo-003 |
| `l6-p7-calibration-corpus-execution` | `testproject1-3yqf.7.4` | L6-FR-2.1.7 (calibration evidence) | (rerun of 4.x canaries) |
| `l6-p7-static-gates-sweep` | `testproject1-3yqf.7.5` | (TDD §4) | CI-L6-001..010 (all 10) |
| `l6-p7-dod-walkthrough` | `testproject1-3yqf.7.6` | L6-FR-2.1.1..2.1.11 | L6-T-live_demo-004 |
| `l6-p7-final-completion-gate` | `testproject1-3yqf.7.7` | aggregate | (M6 capstone) |

---

## Bead Count Reconciliation

- **1 sister implementation epic** (`testproject1-3yqf`).
- **7 phase sub-epics** (`testproject1-3yqf.1` .. `testproject1-3yqf.7`).
- **63 task beads** (`testproject1-3yqf.N.M` where N=phase, M=position):
  - Phase 1: 9 (3yqf.1.1..3yqf.1.9)
  - Phase 2: 7 (3yqf.2.1..3yqf.2.7)
  - Phase 3: 11 (3yqf.3.1..3yqf.3.11)
  - Phase 4: 10 (3yqf.4.1..3yqf.4.10)
  - Phase 5: 10 (3yqf.5.1..3yqf.5.10)
  - Phase 6: 9 (3yqf.6.1..3yqf.6.9)
  - Phase 7: 7 (3yqf.7.1..3yqf.7.7)
- **TOTAL:** 71 beads tagged `l6` (and `layer-6`).

Strategy projected `9+7+11+10+10+9+7 = 63 task beads` plus 7 phase sub-epics. This bead set matches exactly.

---

## Coverage Verification

- **Spec coverage:** 127/127 L6-FR-* IDs covered via primary-owning beads + cross-phase verification. See per-bead `spec_ids` in `l6-bead-alias-map.json` and the §1.1-§2.1 cross-reference above.
- **TDD coverage:** 153/153 numbered L6-T-* IDs primary-owned by exactly one bead. Cross-phase verification (e.g., Phase 3 daemon runtime exercising the same IDs as Phase 5 package tests) is complementary coverage, not duplicate ownership.
- **CI gates:** all 10 CI-L6-001..010 gates have at least one owning bead (Phase 1/5/6 incremental gates + Phase 7 final sweep).
- **`br lint`** passes with no template warnings (verified 2026-05-22).

---

## Success Criteria

L6 is COMPLETE only when:

- All 7 phase review gates closed via `bin/br-complete` (3yqf.1.9, .2.7, .3.11, .4.10, .5.10, .6.9, .7.7).
- All 10 CI-L6-* static gates green (`l6-p7-static-gates-sweep` evidence retained).
- All 11 DoD rows (L6-FR-2.1.1..L6-FR-2.1.11) green in one walkthrough (`l6-p7-dod-walkthrough`).
- Live REMOTE_DRAFT run evidence captured under `docs/pipeline/l6-request-pipeline-automation/live-demo/first-run-REMOTE_DRAFT-YYYY-MM-DD/`.
- Operator-authorized FORMAL_SUBMISSION run evidence captured under `docs/pipeline/l6-request-pipeline-automation/live-demo/second-run-FORMAL_SUBMISSION-YYYY-MM-DD/` (only if operator authorizes — DoD #8 separation).
- 100% L6-FR coverage (127/127) verified.
- 100% L6-T coverage (153/153) verified.
- All 71 l6-labeled beads in CLOSED status.

This is the M6 capstone. No L7 exists per `cross_layer_context.next_layer_after_l6` — L6 is the final layer.
