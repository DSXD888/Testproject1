# L6 Request Pipeline Automation — Step 10: Generate Kickoff + Push + Status

## First Action

Invoke the `/beads-to-execution` skill.

---

## Your Mission

Generate three documents that agents will USE during L6 execution:
1. **Kickoff** (`l6-request-pipeline-automation-kickoff.md`) — Full agent onboarding (~400-700 lines)
2. **Push** (`l6-request-pipeline-automation-push.md`) — Continuation nudge for fresh-context restarts (~120-200 lines)
3. **Status** (`l6-request-pipeline-automation-status.yaml`) — YAML pointer to all SOT docs, bead inventory, milestone state

The kickoff is the most important artifact — it's what an agent reads FIRST when starting L6 work.

**CRITICAL: The kickoff and push MUST include the full AgentMail coordination workflow.** Multiple agents may work in parallel. The prompts must leave ZERO ambiguity about:
- Organization model (FLAT — no hierarchy, agents pick next ready bead)
- AgentMail registration (exact MCP calls + handshake bootstrap)
- Inbox check before picking work (situational awareness)
- Bead claiming via `bin/br-claim` (atomic, file-reservation-first)
- File reservation conflict handling
- Full execution loop (claim → reserve → announce → implement → verify → commit → push → release → loop)
- Session landing protocol (when fully blocked)
- Bead completion policy (MUST fully complete every bead — no partial work, no mid-way commits)
- Commit workflow (complete bead fully → verify → stage specific files → commit with bead ID prefix → push → `bin/br-complete`)

---

## Source Documents

### The 3 Sources of Truth (READ FULLY)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md                                       # SOT #1 (WHAT)
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md      # SOT #2 (HOW TO VERIFY)
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md # Environment + quality gate
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md  # SOT #3 (WHEN)
```

### Manifest + Bead Inventory
```
docs/pipeline/l6-request-pipeline-automation/manifest.yaml                                        # recorded_decisions + existing_files + new_modules
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bulk-import.md  # per-bead detail
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-bead-alias-map.json                     # bead-ID → spec-FR / test-ID mapping
br list --label l6 --json                                                                          # live bead inventory
br ready --label l6 --json                                                                         # ready beads (Phase 1 roots)
```

### Codebase (STUDY FOR INTEGRATION SURFACE — bake these references INTO the kickoff)
```
# AGENTS.md - must reference authoritative `br` block + Rule 2
AGENTS.md:21-34                                                       # Rule 2 (constitutional)
AGENTS.md:279-330                                                     # Authoritative `br` block
AGENTS.md:211-275                                                     # MCP Agent Mail coordination

# Bead CLI wrappers
bin/br-claim                                                          # atomic claim (679 lines, Python 3)
bin/br-complete                                                       # atomic complete (692 lines, Python 3)
bin/br-sync                                                           # br sync + timestamp normalize
bin/ensure-cdp-9222                                                   # CDP launch helper (used by ALL Procontract skills)

# Pipeline integration surface
crates/daemon/src/errors/manifest.rs:19-114                           # ErrorCode (+6 in Phase 1)
crates/daemon/src/main.rs:1513-1517                                   # **+run_bid_workflow dispatch arm goes HERE** (next to existing extract_document_* arms at lines 1513-1517; handler functions at lines 1529 + 1836 are templates). NOT in jobs/runner.rs.
crates/daemon/src/jobs/runner.rs                                      # Shared job-runner types only (1656 lines); not the dispatch site
crates/daemon/src/ipc/handlers/jobs.rs                                # +trigger_bid_workflow (Phase 2)
crates/daemon/src/validation/{policy,executor,manifest_writer}.rs     # validator subsystem (Phase 5)
crates/daemon/src/main.rs:5256-5260,5510-5527,5537-5542,5781-5787     # L5 readiness raw checks
crates/daemon/src/db/sync.rs:24-50                                    # push set (bid_workflow_states at line 39 — no changes)
crates/daemon/src/agents/{adapter,artifacts}.rs                       # PTY + run manifest

# Existing skills L6 builds on
skills/auth-signin/procontract/SKILL.md                               # delegated auth (Phase 4)
skills/procurement-ingestion/procontract-register-interest/SKILL.md   # reference pattern (Phase 4)
skills/validation/overlay-supplier-enrichment/SKILL.md                # frontmatter shape (Phase 5)

# Schema (no changes needed)
migrations/0001_sqlite_state_initial.sql:437-451                      # bid_workflow_states
migrations/0003_sqlite_runtime.sql:49-65                              # error_manifests
infra/supabase/migrations/0002_remote_owned_tables.sql:5-100          # operator_decision_heads + shared_company_profile
```

### Quality Bar (MATCH OR EXCEED — study these for kickoff structure)
```
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-kickoff.md   # L5 kickoff
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-push.md     # L5 push
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-status.yaml # L5 status
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-kickoff.md                                # L4 kickoff (708 lines — gold standard for AgentMail walkthrough)
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-push.md                                   # L4 push (124 lines)
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-status.yaml                               # L4 status
```

---

## Output Files

| File | Path | Purpose | Expected Size |
|------|------|---------|---------------|
| Kickoff | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-kickoff.md` | Full agent onboarding | 400-700 lines |
| Push | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-push.md` | Continuation nudge | 120-200 lines |
| Status | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-status.yaml` | SOT pointer + bead inventory | 60-100 lines |

---

## Required Kickoff Sections (ALL mandatory — in this order)

### 1. Mission

- Feature description: portal-specific bid skill (Procontract) for filling forms + uploading documents + submitting expressions of interest with full audit trail
- Scope: builds on L0-L5 infrastructure; greenfield `packages/bidding/` + `skills/bid-portal-procontract/` + `skills/validation/overlay-bid-portal-procontract/`
- Completion milestones (epic-level): Phase 1-7 from strategy
- "Work from the bead graph. Claim task beads, not epics. Each bead is self-contained — implement from the bead alone."
- Spec is `docs/strategy/layers/layer-6-request-pipeline-automation.md`
- M6 capstone: live demo proves bid workflow end-to-end

### 2. Implementation Boundary

**EXISTS (build on top of):**
- `bid_workflow_states` table (L0 schema, lines 437-451 of `migrations/0001_sqlite_state_initial.sql`) — fully columned, dormant, awaiting L6 activation
- `bid_workflow_states` already in `PUSH_OWNED_RESEED_TABLES` (sync.rs line 39) — sync engine no-op
- `error_manifests` table (L3) — write target for bid failures
- `operator_decision_heads` + `shared_company_profile` (L4 remote-owned) — read targets
- L5 readiness checks (CLASSIFICATION_AMBIGUOUS / TARIFF_BRANCH_DIVERGENCE / SUPPLEMENTARY_UNIT_MISSING / decision_grade=broker_required / classification_fresh_until_expired) — wrap, don't reimplement
- `skills/auth-signin/procontract/` — delegated portal login via DAEMON-ORCHESTRATED RETRY (the bid skill returns `AUTH_STATE_STALE` on session expiry; the daemon runs auth-signin/procontract as a sub-job; daemon re-runs the original bid job — bid skill NEVER invokes auth-signin directly)
- `skills/procurement-ingestion/procontract-register-interest/` — REFERENCE PATTERN for the new bid skill (CDP attach pattern, side_effect_class declarations, schema layout)
- Validator subsystem (`crates/daemon/src/validation/*`) — bid runs enqueue validator jobs; no new validator code
- Run manifest writer (`crates/daemon/src/validation/manifest_writer.rs:23-54`) — bid runs produce hybrid manifests with screenshot artifact refs
- PTY infrastructure + artifact store — bid runs reuse without changes
- Persistent CDP browser at `/Users/alinaaitova/.chrome-automation` (port 9222) — attached by all Procontract skills via `bin/ensure-cdp-9222`

**DOES NOT EXIST (you're building):**
- `packages/bidding/` directory (4 source files + 3 test files + package.json + tsconfig.json + vitest.config.ts)
- `skills/bid-portal-procontract/` (SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml)
- `skills/validation/overlay-bid-portal-procontract/` (SKILL.md)
- `tests/fixtures/calibration/bid-portal-procontract/` (per-variant fixtures: simple/medium/complex)
- 3-6 NEW ErrorCode variants (TDD-determined count after redundancy audit)
- `crates/daemon/src/main.rs` extension for `run_bid_workflow` dispatch arm at line 1513+ (next to existing extract_document_* arms) + the `run_bid_workflow_job(...)` handler function (inline in main.rs OR extracted to `crates/daemon/src/bid_workflow/` — Phase 3 decides). NOT in `crates/daemon/src/jobs/runner.rs` (which holds shared types only).
- `crates/daemon/src/ipc/handlers/jobs.rs` extension for `trigger_bid_workflow` handler following `trigger_ingestion` at line 45 + `trigger_enrichment` at line 95 patterns (with L5 readiness gate pre-check)
- `crates/daemon/src/main.rs` extension for `--run-bid-workflow` CLI subcommand
- Optional `crates/daemon/src/bid_workflow/` module (Phase 3 may create if state-machine logic warrants)
- `packages/ui` extensions (BidWorkflowStatusBadge, BidScopeSummary, DraftSavedReviewPanel, BidWorkflowStartButton)
- `apps/desktop` BidWorkflow tab integration
- `apps/web` read-only bid_state badge + RLS verification

"Do NOT assume unbuilt infrastructure exists. Do NOT reimplement existing auth or sync logic."

### 3. Core Contracts / Non-Negotiables (8 constitutional invariants + override semantics)

Quote from spec §1.10 verbatim:
1. No real-money bids without human approval (operator `approved` disposition required)
2. No externally visible portal mutation beyond approved side-effect class (`sideEffectCap` enforcement)
3. Bid workflow carries explicit scope (`bidScopeJson` payload with lot/candidate IDs — no blanket bids)
4. Full audit trail for submissions (screenshots, terminal output, run manifest per step)
5. Stale decisions block bid (`reviewedBasisHash != decisionBasisHash` → blocked)
6. L5 readiness gate (5 blockers + quote-role + freshness + upstream trust)
7. Validation fail-closed for portal submission (validator rejection or outage → `bid_submitted` blocked; `draft_saved` permitted without)
8. Quote-role quality gate (`domestic_wholesale_floor` / `optimistic_floor`-only evidence requires explicit operator override)

Plus naming reconciliation (decision_1): **Proactis === Procontract**. New files use `procontract` naming.

**Operator override semantics (`manifest.yaml#operator_override_mechanisms` for full detail):**
- **OVERRIDEABLE gates** (with logged acknowledgment): quote-role quality gate (floor-only evidence); validation gate at final submission (validator rejection or outage)
- **NON-OVERRIDEABLE gates** (constitutional, no bypass): all 5 L5 hard blockers (CLASSIFICATION_AMBIGUOUS / TARIFF_BRANCH_DIVERGENCE / SUPPLEMENTARY_UNIT_MISSING / decision_grade=broker_required / classification_fresh_until_expired); operator decision NOT approved; operator decision stale; upstream pipeline trust gate (validation_state ≠ validated without operator trust-debt acceptance)
- Every override generates an audit-trail entry in the run manifest narrative — validator overlay flags missing override-citations as anomalies

"If code conflicts with this contract, the contract wins."

### 4. Source of Truth Loading (MANDATORY — before starting ANY implementation)

Ordered read list with READ/SKIM/REFERENCE annotations:

1. **AGENTS.md** — **READ FULLY** (especially Rule 2 at lines 21-34, authoritative `br` block at lines 279-330, AgentMail coordination at lines 211-275)
2. **docs/strategy/layers/layer-6-request-pipeline-automation.md** — **READ FULLY** (the spec — SOT #1)
3. **docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md** — **READ FULLY** (SOT #2)
4. **docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md** — **READ FULLY** (env + quality gate)
5. **docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md** — **READ FULLY** (SOT #3 — phase ordering)
6. **docs/pipeline/l6-request-pipeline-automation/manifest.yaml** — **REFERENCE** (decisions + existing_files; bookmark)
7. **docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bulk-import.md** — **SKIM** (bead inventory; use `br show <id>` for the bead you claim)
8. **skills/auth-signin/procontract/SKILL.md** — **READ** when claiming any Phase 4 bead
9. **skills/procurement-ingestion/procontract-register-interest/SKILL.md** — **STUDY** when claiming any Phase 4 bead (reference pattern)
10. **skills/validation/overlay-supplier-enrichment/SKILL.md** — **READ** when claiming Phase 5 validator overlay bead (frontmatter shape)
11. **L4 + L5 kickoffs + completion audits** — **REFERENCE** for quality bar

### 5. AgentMail Setup (Do This ONCE At Session Start)

Exact MCP calls — no ambiguity. CRITICAL: handshake bootstrap is required (per AGENTS.md Rule 2 prerequisite block) — without it, `bin/br-claim` falls back to `list_agents` liveness or raises `NoPeersError`.

```
# 1. Project + identity
ensure_project(project_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(project_key, program="claude-code", model="opus")
set_contact_policy(project_key, agent_name, policy="open")

# 2. Bootstrap Contact Handshakes (MANDATORY before any bin/br-claim)
peers = list_agents(project_key)
for peer in peers:
    if peer['program'] == 'claude-code' and peer['active_in_last_hour'] and peer['name'] != agent_name:
        macro_contact_handshake(project_key, requester=agent_name, target=peer['name'])

# 3. Verify handshake landed
contacts = list_contacts(project_key, agent_name)
# Expect at least the peers you handshook to appear here. If empty AND you're solo,
# pass --solo to bin/br-claim. If empty AND there are live peers, retry handshake.

# 4. Check inbox for messages from prior sessions or other agents
fetch_inbox(project_key, agent_name, limit=20)
```

"Read and acknowledge any pending messages before starting work. If another agent left a blocker notice, address it first."

### 6. Bead Completion Policy (CRITICAL — verbatim, non-negotiable)

**Use the project wrappers to claim and complete every bead — they are the only authorized path:**

- `bin/br-claim <bead-id>` — claims atomically (file reservation FIRST as the collision detector, then `[<bead-id>] Start` mail to peers, then `br update --status=in_progress`)
- `bin/br-complete <bead-id>` — completes atomically (verifies commit is pushed to remote, sends `[<bead-id>] Complete` mail, closes bead, syncs `.beads/`, releases file reservations)

**Do NOT use raw `br update --status=in_progress` or `br close`** — they bypass mail and file reservations and break the swarm's collision detection. AGENTS.md Rule 2 (constitutional, lines 21-34) forbids it; the Phase B precommit hook blocks any commit with a `<bead-id>:` subject that has no Start mail in the archive.

**You MUST fully complete every bead you claim.** No partial completion. No "I'll finish this later." No moving on to another bead while one is in progress.

A bead is COMPLETE when:
1. All code changes described in the bead are implemented
2. All tests described in the bead pass
3. All build/lint/test verification commands pass (zero warnings on `cargo clippy --all-targets -- -D warnings`)
4. Code is committed with `<bead-id>:` subject prefix and pushed to the remote
5. `bin/br-complete <bead-id>` has run successfully (mail + close + sync + release)

If you hit a blocker during a bead:
- DO NOT abandon the bead and move to another track
- DO NOT mark it complete with missing pieces
- DO NOT do mid-way commits of incomplete work
- Diagnose the blocker. Read the error. Understand the root cause.
- Fix the blocker yourself — you have the full codebase available
- Only if the blocker is genuinely impossible to resolve (missing infrastructure not in your bead), reply in the bead's mail thread with file path, line number, and what's wrong, then wait for resolution. Do NOT release reservations or close the bead while blocked.

### 7. Work Loop (Full Visual)

```
ONE-TIME SETUP:
  1. AgentMail registration (see §5 — export AGENT_MAIL_NAME)
  2. Read SOT docs (see §4)
  3. br ready --label l6 --json → understand the bead graph

MAIN LOOP (repeat until all L6 beads complete):

  ┌─ PICK ──────────────────────────────────────────────┐
  │ br ready --label l6 --json                           │
  │ Pick the highest-priority ready bead.                │
  │ Read the bead description FULLY before touching code.│
  │ Verify the bead's strategy phase matches what you    │
  │ expect (e.g., Phase 1 beads have no deps).           │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ CLAIM (atomic via wrapper) ────────────────────────┐
  │ bin/br-claim <bead-id>                               │
  │ # Wrapper does, in order:                            │
  │ #   1. duplicate-claim guard (status + Start search) │
  │ #   2. file reservation (collision detector — FIRST) │
  │ #   3. [<bead-id>] Start mail to peers (no broadcast)│
  │ #   4. br update --status=in_progress                │
  │ # If FILE_RESERVATION_CONFLICT: pick a different bead│
  │ # Override file list with --paths if bead lacks a    │
  │ # `### Files to Create/Modify` section.              │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ IMPLEMENT (TDD) ──────────────────────────────────┐
  │ 1. Study target files — understand current state    │
  │ 2. For NEW files: scaffold per spec §1.6/§1.7 +     │
  │    reference skills/procurement-ingestion/          │
  │    procontract-register-interest/SKILL.md pattern   │
  │ 3. Write tests FIRST (the L6-T-* IDs the bead owns) │
  │ 4. Implement code until ALL tests pass              │
  │ 5. Verify against bead's AC bullets one-by-one      │
  │                                                      │
  │ DO NOT cut corners. DO NOT skip tests.              │
  │ DO NOT leave TODOs. DO NOT do mid-way commits.      │
  │                                                      │
  │ CDP-related beads: invoke bin/ensure-cdp-9222       │
  │ before any portal navigation. NEVER launch a fresh  │
  │ browser profile. ATTACH to operator's persistent    │
  │ profile at /Users/alinaaitova/.chrome-automation.   │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ VERIFY ────────────────────────────────────────────┐
  │ cargo fmt --check                                    │
  │ cargo check --all-targets                            │
  │ cargo clippy --all-targets -- -D warnings            │
  │ cargo test --test <relevant-test>                    │
  │ bun run build                                        │
  │ bun run lint                                         │
  │ bun run test                                         │
  │ # Plus desktop / web E2E if Phase 6                  │
  │ (cd apps/desktop && bun run test:e2e)                │
  │ (cd apps/web && bun run test:e2e)                    │
  │                                                      │
  │ Fix ALL failures before proceeding.                  │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ COMMIT + PUSH (must precede br-complete) ──────────┐
  │ git add <specific-files-only>                        │
  │ git commit -m "<bead-id>: <short description>"       │
  │ git pull --rebase && git push                        │
  │ # br-complete refuses to run until HEAD == @{u}.     │
  │ One commit per bead (multiple OK if large, but each  │
  │ must pass all checks independently).                 │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ COMPLETE (atomic via wrapper) ─────────────────────┐
  │ bin/br-complete <bead-id>                            │
  │ # Wrapper does, in order:                            │
  │ #   1. verify status=in_progress + Start mail by you │
  │ #   2. resolve commit SHA + verify pushed            │
  │ #   3. [<bead-id>] Complete mail to peers            │
  │ #   4. br close <bead-id>                            │
  │ #   5. bin/br-sync + git add .beads/ + commit + push │
  │ #   6. release_file_reservations(agent_name=self)    │
  └──────────────────────────────────────────────────────┘
           │
        LOOP → back to PICK
```

### 8. Verification Commands (Per Stack Surface)

```bash
# Rust (daemon)
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done

# TypeScript (packages + apps)
bun run build
bun run lint
bun run test

# E2E (Phase 6 + Phase 7)
(cd apps/desktop && bun run test:e2e)
(cd apps/web && bun run test:e2e)

# UBS (on staged files)
ubs $(git diff --name-only --cached)
```

"Use debug builds (`cargo check`, not `cargo build --release`) for fast iteration."

### 9. File-Surface Map by Phase

For each phase, the files in scope:

**Phase 1 — Foundation:**
- `crates/daemon/src/errors/manifest.rs` (extend ErrorCode with the TDD-determined NEW variant set, typically 3-6; add `as_str()` arms)
- `crates/daemon/src/errors/markdown.rs` + `validation/policy.rs` + `validation/executor.rs` + `tracing/structured.rs` + `ipc/handlers/*.rs` (clippy-surfaced exhaustive matches)
- `crates/daemon/src/bid_workflow/mod.rs` (NEW — module root) + `types.rs` (Rust mirror of bid-scope-types.ts) + `runner.rs` (stub `run_bid_workflow_job(...) { todo!("Phase 3") }`) + `readiness_gate.rs` (stub) + `state_machine.rs` (stub). Wire `mod bid_workflow;` into main.rs so Phase 2 dispatch can `use crate::bid_workflow::...`. See `manifest.yaml#spec_callouts.bid_workflow_runner_module_placement` for the full rationale.
- `crates/daemon/src/main.rs` model_profile registration: add a new match arm for `"portal_form_high_reliability"` in the `match skill.metadata.model_profile.as_str() { ... }` block (existing arms: `browse_stable_low_cost` + `structured_extract_low_cost`). This is the L6 introduction per spec §1.7. If TDD step 03 chose to reuse an existing profile instead, drop this work.
- `packages/bidding/bid-scope-types.ts` (NEW — BidScopePayload, SideEffectClass union, BidWorkflow, BidDocument, SubmissionResult, DraftSaveResult, PortalStateVerification; types MUST land in Phase 1 so Phase 4's portal skill `result.schema.json` and Phase 2's IPC handler can reference them)
- `packages/bidding/package.json` + `tsconfig.json` (minimal scaffolding for the types to compile; full workflow files land in Phase 5)

**Phase 2 — Daemon Runtime:**
- `crates/daemon/src/main.rs:1513+` (add `"run_bid_workflow" => bid_workflow::runner::run_bid_workflow_job(...).await` arm next to existing extract_document_* arms — call wires to the Phase 1 module skeleton stub; the stub `todo!()` panics at runtime until Phase 3 fills it out)
- `crates/daemon/src/ipc/handlers/jobs.rs` (add `trigger_bid_workflow` handler at line ~150+ following `trigger_ingestion` at line 45 + `trigger_enrichment` at line 95 patterns; readiness-gate pre-check BEFORE job enqueue)
- `crates/daemon/src/main.rs` (add `--run-bid-workflow` CLI subcommand)
- Optional: `crates/daemon/src/bid_workflow/mod.rs` (if state-machine logic warrants extraction)

**Phase 3 — State Machine + Readiness Gate (fills out Phase 1 stubs):**
- `crates/daemon/src/bid_workflow/runner.rs` — fill body: load `skills/bid-portal-procontract/`, inject `BidScopePayload` + `CompanyProfile` from `operator_decision_heads` + `shared_company_profile`, spawn PTY session, capture screenshots per step, persist `bid_workflow_states` transitions atomically with run manifest write (extends `validation/manifest_writer.rs` write_hybrid_run_manifest)
- `crates/daemon/src/bid_workflow/state_machine.rs` — fill body: all **14** transitions (per spec §1.3 table) with prerequisite checks; self-loop guard; staleness detection on transitions
- `crates/daemon/src/bid_workflow/readiness_gate.rs` — fill body: wrap existing L5 checks at `crates/daemon/src/main.rs:5256-5260,5510-5527,5537-5542,5781-5787` + quote-role + freshness + company-profile + upstream-trust + operator-override read; return structured `ReadinessGateResult { approved, blockers: Vec<BlockerCode>, override_required: Option<OverrideKind> }`
- `crates/daemon/src/bid_workflow/decision_basis_hash.rs` (NEW — or in state_machine.rs) — SHA-256 input vectors: pipeline_assessments hash + selected supplier_quotes versions + shared_company_profile version + per-candidate landed_cost_estimates.classification_fresh_until + L5 freshness vectors (tariff/measures/remedies/FX hashes); ANY input change → reviewedBasisHash != current → bid blocked
- `crates/daemon/tests/redaction_boundary.rs` (extend for BidScopePayload + CompanyProfile fields)

**Phase 4 — Portal Skill:**
- `skills/bid-portal-procontract/SKILL.md` (NEW — 11-step playbook; frontmatter MUST include `source_structure_version: procontract-bid-portal-<date>` + `validation_policy: always` + `requires_cdp: true` + `cdp_default_port: 9222` + `worker_behavior_version: bid-portal/procontract@<v>+<schema-v>`; `model_profile: portal_form_high_reliability` is NEW — paired with a Phase 1 daemon bead that adds the match arm to `main.rs`'s `model_profile.as_str()` block, OR fall back to `browse_stable_low_cost`)
- `skills/bid-portal-procontract/result.schema.json` (mirror `skills/procurement-ingestion/procontract-register-interest/result.schema.json` shape; reference SubmissionResult/DraftSaveResult from Phase 1 `packages/bidding/bid-scope-types.ts`)
- `skills/bid-portal-procontract/checkpoint.schema.json` (mirror procontract-register-interest's checkpoint format; per-step resume state)
- `skills/bid-portal-procontract/canary.inputs.jsonl` (≥3 canary inputs covering small/medium/complex form complexity; format mirrors existing procontract skills)
- `skills/bid-portal-procontract/golden.assertions.yaml` (per-canary expected outputs)
- `tests/fixtures/calibration/bid-portal-procontract/{simple,medium,complex}/` (NEW)
- `crates/daemon/src/sources/registry.rs` (add RegisteredSource entry)
- `crates/daemon/tests/skill_canary_harness.rs` (add e2e_can_010+)
- `crates/daemon/tests/skill_golden_journeys.rs` (add l3_t_skill_golden_journeys_009+)

**Phase 5 — Validator Overlay + packages/bidding workflow:**
- `skills/validation/overlay-bid-portal-procontract/SKILL.md` (NEW)
- `packages/bidding/workflow-registry.ts` (NEW — depends on bid-scope-types.ts from Phase 1)
- `packages/bidding/procontract-workflow.ts` (NEW — implements BidWorkflow interface from Phase 1's bid-scope-types.ts)
- `packages/bidding/company-profile-schema.ts` (NEW)
- `packages/bidding/tests/bid-workflow-states.test.ts` (NEW)
- `packages/bidding/tests/portal-side-effects.test.ts` (NEW)
- `packages/bidding/tests/bid-readiness-gate.test.ts` (NEW)
- `packages/bidding/vitest.config.ts` (NEW; package.json + tsconfig.json already landed in Phase 1 for the types to compile)

**Phase 6 — Desktop + Web UI:**
- `packages/ui/src/BidWorkflowStatusBadge.tsx` (NEW)
- `packages/ui/src/BidScopeSummary.tsx` (NEW)
- `packages/ui/src/DraftSavedReviewPanel.tsx` (NEW)
- `packages/ui/src/BidWorkflowStartButton.tsx` (NEW)
- `apps/desktop/src/` (NEW BidWorkflow tab integration)
- `apps/desktop/e2e/bid-workflow.spec.ts` (NEW)
- `apps/web/src/` (read-only bid_state badge + RLS-bounded view)
- `apps/web/e2e/bid-workflow-viewer.spec.ts` (NEW — verifies viewer role sees badge but NOT bid_scope_json or run manifest)

**Phase 7 — Live Demo + Integration + Calibration Validation:**
- Manual smoke (operator-driven via desktop)
- `tests/fixtures/calibration/bid-portal-procontract/` calibration runs
- Final quality gate sweep (all phases)

### 10. Dependency Spine (Mental Model)

```
Phase 1 (Foundation — ErrorCode + bid-scope-types.ts)
   ↓
Phase 2 (Daemon Runtime — jobs/runner + ipc/handler + CLI)
   ↓
Phase 3 (State Machine + Readiness Gate)
   ↓
Phase 4 (Portal Skill — bid-portal-procontract)  ∥  Phase 5 (Validator Overlay + packages/bidding workflow)
                                  ↓                                  ↓
                                  └──────────┬───────────────────────┘
                                             ↓
                              Phase 6 (Desktop + Web UI — HARD-depends on Phase 2 IPC handler)
                                             ↓
                              Phase 7 (Live Demo + Integration)
```

Critical edges:
- Phase 1's `packages/bidding/bid-scope-types.ts` is a hard prereq for Phases 2 (IPC handler signature), 4 (portal skill result.schema.json), and 5 (workflow files implement these interfaces).
- Phase 6's Start Bid button HARD-depends on Phase 2's `trigger_bid_workflow` IPC handler — the button cannot work without it.
- Phase 7's live demo requires ALL prior phases closed + operator action gates.

"Trust `br ready --label l6`, but use this spine as your mental model. If `br ready` returns a bead that the spine says shouldn't be ready yet, something is broken — investigate before claiming."

### 11. AgentMail Quick Reference (ALL agents)

**CRITICAL:** Agent Mail has NO broadcast. `"broadcast"` is NOT a valid recipient. The `bin/br-claim` and `bin/br-complete` wrappers handle peer discovery via `list_contacts` and send Start/Complete mail for you — never send those by hand. Use `send_message` directly only for non-claim/complete events: blocker reports, mid-implementation questions, hand-offs, etc. Always discover peers via `list_contacts(project_key, agent_name)` first.

```
# Check inbox (do this between beads)
fetch_inbox(project_key, agent_name, limit=10)

# Claim/Complete — use the wrappers (handles mail + reservations atomically):
bin/br-claim <bead-id>
bin/br-complete <bead-id>

# Discover peers — refresh between beads (used by wrappers; also useful for ad-hoc messages)
list_contacts(project_key, agent_name)  # → e.g. ["BlueLake", "RedPeak"]

# Ad-hoc messages (NOT for Start/Complete — wrappers send those):
# Report blocker
send_message(project_key, sender_name=<your-name>, to=["<peers>"],
  thread_id="<bead-id>", subject="[<bead-id>] BLOCKED: <issue>",
  body_md="File: <path>, Line: <N>. Issue: <desc>. Needed: <what>")
# Mid-implementation question
send_message(project_key, sender_name=<your-name>, to=["<peers>"],
  thread_id="<bead-id>", subject="[<bead-id>] Question: <topic>")
```

### 12. CDP Browser Discipline (Phase 4 + Phase 7 critical; informational for Phase 1-3, 5, 6)

ALL Procontract skills (auth-signin, procurement-ingestion/procontract-*, and the new bid-portal-procontract) attach to the operator's persistent Chrome at `/Users/alinaaitova/.chrome-automation` on port 9222.

```bash
# Defense in depth: idempotent helper. <1s no-op if CDP up; up to 15s if cold launch needed.
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222

# Reachability check
curl -sfS -m 3 "http://localhost:9222/json/version"
```

**MUST NOT:**
- Launch a fresh browser profile
- Attempt portal login outside the auth-signin/procontract skill
- Solve captchas
- Log out
- Close the operator's browser

**Sensitivity:** `sales@sorelon.com` is the operator's REAL Procontract account with bid history and financial implications. The bid skill MUST NOT mutate anything outside the scope payload's lots/candidates.

### 13. Live Demo Constraints (Phase 7 ONLY — DoD #8)

> **Phase 1-6 agents: this section does NOT apply to your bead.** Only Phase 7 (live demo + integration + calibration validation) beads have operator-action gates around real Procontract opportunity interactions. Skip this section if your bead's phase < 7.

Per decision_5:
- **First live demo:** sideEffectCap = REMOTE_DRAFT. Reaches `draft_saved`. Screenshot trail present. Run manifest written. Supabase visible.
- **Operator-authorized second live demo:** sideEffectCap allows FORMAL_SUBMISSION. Reaches `bid_submitted`. Portal confirmation ref captured.

The second run requires explicit operator authorization (separate operator interaction). DO NOT escalate to FORMAL_SUBMISSION unless the bead's AC explicitly says so AND operator has confirmed via desktop UI.

---

## Required Push Sections (ALL mandatory)

The push prompt is for continuation agents. Compact, no walkthrough fluff.

### 1. Re-Orient
- Re-read `AGENTS.md`
- L6 = Request Pipeline Automation; spec at `docs/strategy/layers/layer-6-request-pipeline-automation.md`
- Strategy at `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md`
- Bead graph: `br ready --label l6 --json`
- Implementation epic: <NEW_EPIC_ID> (from manifest after step 07)

### 2. Core Contracts (compact)
- 8 invariants from spec §1.10
- Proactis === Procontract (decision_1)
- Reuse auth-signin/procontract (decision_2)
- Validator overlay is deliverable (decision_3)
- Sister epic structure (decision_4)
- Live demo gated on operator action (decision_5)

### 3. Bead Completion Policy (compact)
"Use `bin/br-claim <id>` and `bin/br-complete <id>` — never raw `br update`/`br close`. AGENTS.md Rule 2 (constitutional, lines 21-34) makes the wrappers the only authorized path. Every bead you claim MUST be fully completed. No partial work. No TODOs. No 'finish later.' Complete means: all code + all tests + all checks + committed AND pushed + br-complete ran clean. If blocked: diagnose, fix yourself, only escalate via AgentMail (reply in the bead's thread) if the blocker is in another agent's code. Do NOT release reservations while blocked."

### 4. Work Loop (compact)
```
1. fetch_inbox(project_key, agent_name, limit=10) → acknowledge any messages
2. br ready --label l6 --json → pick highest-priority ready bead
3. Read bead FULLY → study target files
4. CLAIM: bin/br-claim <bead-id>
   # Wrapper handles reservation + Start mail + br update atomically.
5. IMPLEMENT: TDD — test first, then code. ALL of it. No mid-way commits.
6. VERIFY: cargo fmt/check/clippy/test + bun build/lint/test (ALL pass)
7. COMMIT + PUSH: git add <specific-files> + git commit -m "<bead-id>: <desc>" + git pull --rebase && git push
8. COMPLETE: bin/br-complete <bead-id>
9. LOOP → step 1
Do not stop to ask what to work on next. Pick the next ready bead.
```

### 5. Verification Commands (compact)
Same as kickoff §8.

### 6. Dependency Spine (compact)
Same as kickoff §10.

### 7. File Surfaces (compact)
Phase × Key Files table (condensed version of kickoff §9).

### 8. AgentMail Quick Reference (compact)
Same as kickoff §11.

### 9. CDP Browser Discipline (compact)
Same as kickoff §12.

---

## Required Status YAML Content

Match L4's status structure (see `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-status.yaml` for the gold-standard format) with L5's simpler bead_prefix scheme:

```yaml
# L6 Request Pipeline Automation — Pipeline Status

feature: l6-request-pipeline-automation
status: in_progress                          # → "complete" after step 15 archive
started: "2026-MM-DD"                         # date of step 10 generation
completed: null                                # set by step 15
spec: docs/strategy/layers/layer-6-request-pipeline-automation.md
tdd: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md
strategy: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md
bootstrap: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md
bulk_import: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bulk-import.md
bead_label: layer-6
bead_prefix: l6
bead_cli: br
bead_sync_cmd: bin/br-sync
bead_id_pattern: "l6-*"
existing_layer_doc_epic: testproject1-rc0     # CLOSED in step 07 (decision_4)
existing_implementation_epic: <NEW_EPIC_ID>   # CREATED in step 07 (decision_4)
phases: 7                                     # may be 6-8 depending on strategy step 05 outcome
total_beads: <count_from_br_list>
task_beads: <count>
review_gate_beads: <count>
fix_beads: 0                                  # populated by step 12 if gaps found
audit_iterations: 0                           # incremented by step 12 re-runs
agentmail_key: /Users/alinaaitova/Desktop/Testproject1
branch: main
target_milestone: M6
exit_criteria_met: false                      # → true after step 12 verdict COMPLETE + live demo
exit_criterion: "System can prepare and submit one request on Procontract end-to-end with full audit trail. DoD #1-11 green. Live demo first run (sideEffectCap=REMOTE_DRAFT) reached draft_saved; operator-authorized second run (FORMAL_SUBMISSION) reached bid_submitted with portal confirmation reference."
ready_for_next_layer: "N/A — L6 is final layer; no L7 exists; M6 capstone completion"

source_of_truth:
  spec:
    path: docs/strategy/layers/layer-6-request-pipeline-automation.md
    role: "SOT #1 — WHAT to build (~NN L6-FR-X.Y.Z requirement IDs, 11 DoD criteria, 8 invariants, 14 state-machine transitions, 11-step playbook with 10a/10b branching)"
  tdd:
    path: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md
    role: "SOT #2 — HOW to verify (NN L6-T-* test IDs across 3 spec'd TS files + Rust integration extensions; NN static CI gates)"
  strategy:
    path: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md
    role: "SOT #3 — WHAT ORDER (7 phases, dependency DAG, gate criteria, Phase 7 = M6 capstone live demo)"
  bootstrap:
    path: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md
    role: "Environment setup — Rust nightly, Bun 1.3.10, persistent CDP profile, AgentMail, Procontract opportunity ref for live demo, local quality gate sequence"

execution_prompts:
  kickoff:
    path: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-kickoff.md
    role: "Full agent onboarding (~400-700 lines, 13 sections incl. CDP discipline + live demo constraints)"
  push:
    path: docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-push.md
    role: "Continuation nudge (~120-200 lines, 9 sections — compact reminder for fresh-context restarts)"

completion_evidence:                          # populated by step 12 + step 15
  audit_report: null                          # → docs/pipeline/l6-request-pipeline-automation/artifacts/l6-completion-audit-YYYY-MM-DD.md
  audit_verdict: null                         # → "COMPLETE — zero gaps" or "INCOMPLETE (N fix beads)"
  dod_criteria_passed: 0
  dod_criteria_total: 11
  invariants_passed: 0
  invariants_total: 8
  state_machine_transitions_tested: 0
  state_machine_transitions_total: 14
  live_demo_first_run_state: null             # → "draft_saved" after live demo
  live_demo_second_run_state: null            # → "bid_submitted" after operator-authorized second demo
  validator_overlay_built: false              # → true (decision_3)
  packages_bidding_files_landed: 0            # → N (3 source + 3 test + 3 config)
  bid_portal_procontract_skill_landed: false  # → true
  desktop_bid_workflow_tab_landed: false      # → true
  web_bid_state_badge_landed: false           # → true (read-only viewer, RLS-bounded)
  new_error_code_variants: 0                  # → N (3-6, TDD-determined)

recorded_decisions:                           # snapshot from manifest.yaml#recorded_decisions
  decision_1: "Proactis === Procontract — same portal (procontract.due-north.com)"
  decision_2: "REUSE skills/auth-signin/procontract/ (Touch ID removed)"
  decision_3: "Validator overlay skills/validation/overlay-bid-portal-procontract/ is L6 deliverable"
  decision_4: "Sister implementation epic (rc0 closed, new epic created in step 07)"
  decision_5: "Procontract access already solved (live demo feasible day-one)"

m6_capstone:
  procurement_intelligence_platform_status: in_progress    # → complete after step 15
  layers_built: ["L0", "L1", "L2", "L3", "L4", "L5"]      # → +"L6" after step 15
  layers_planned: ["L6"]                                    # → [] after step 15
  next_layer: "L6"                                          # → "N/A — final layer" after step 15

archived: null                                # → docs/pipeline/l6-request-pipeline-automation/archived/YYYY-MM-DD-l6-request-pipeline-automation/
```

**NOTE on bead_id_pattern**: L4 used `testproject1-4zq.*` (children of the doc-authoring epic). L5 switched to `l5-*` prefix (sister-epic children). L6 follows L5's more recent convention with `l6-*`.

**NOTE on phases**: defaults to 7 per the suggested strategy shape; if strategy step 05 chose 6 or 8, update this field and the enrichment block warnings in EXECUTION-GUIDE.

---

## Feature-Specific Guidance for Skill

While `/beads-to-execution` runs, ensure it:

1. **Bakes in the 5 recorded decisions** (kickoff §3 Core Contracts + status YAML)
2. **Uses procontract naming throughout** (NEVER proactis in file paths or skill IDs)
3. **References specific line numbers** from existing files (NOT generic "find the file")
4. **Includes the CDP discipline section** (§12) — bid-portal skill inherits the same rules as the existing procontract skills
5. **Includes live demo constraints** (§13) — first run REMOTE_DRAFT, second run requires operator authorization
6. **Includes the handshake bootstrap** in AgentMail setup (§5) — without it, br-claim peer discovery fails
7. **References `bin/br-sync`** (NEVER raw `br sync`)
8. **Uses authoritative `br` block** (AGENTS.md lines 279-330), NEVER the stale `bd` block at lines 683-738
9. **Names the existing implementation epic ID** (from manifest, populated by step 07)

---

## Verification (after `/beads-to-execution` completes)

- [ ] Kickoff file exists, 400-700 lines, 13 sections per requirements above
- [ ] Push file exists, 120-200 lines, 9 sections per requirements above
- [ ] Status YAML exists with all fields populated
- [ ] All 5 recorded decisions appear in kickoff Core Contracts
- [ ] All Procontract references (in new file paths) use "procontract" not "proactis"
- [ ] CDP discipline section present
- [ ] Live demo constraints section present
- [ ] AgentMail setup includes handshake bootstrap
- [ ] `bin/br-claim` + `bin/br-complete` + `bin/br-sync` referenced (NEVER raw `br` mutation commands)
- [ ] Authoritative `br` block referenced (NEVER `bd`)
- [ ] Implementation epic ID populated

---

## ⛔ MANDATORY STOP — Human Authorizes Execution

After this step, **STOP**. The human reviews the kickoff + push + status before agents begin executing L6 implementation. Specifically:

1. Read the kickoff (compare against L4/L5 kickoff quality bar)
2. Verify AgentMail registration block uses `/Users/alinaaitova/Desktop/Testproject1`
3. Verify `br ready --label l6` shows the L6 bead graph in expected order (Phase 1 roots ready, downstream blocked)
4. Verify quality gates list correct Rust + TS + UBS commands
5. Authorize parallel agent execution

Do not proceed to execution / step 11 / step 12 without explicit human authorization.

---

## NO sub-optimization — these ARE the deliverables.

The kickoff + push + status ARE the artifacts agents use. They are the highest-stakes
documents in the pipeline. Get them right the first time. Match L4/L5 kickoff rigor
(708 / similar lines, deeply project-specific).
