# L5 Expanded Commercial Intelligence — Generate Kickoff + Push Prompts

## First Action

Invoke the `/beads-to-execution` skill.

---

## Your Mission

Generate the **kickoff prompt** (full agent onboarding), **push prompt** (continuation nudge), and **status YAML** for L5 implementation. These are the documents agents will actually USE during implementation. They must be deeply L5-specific — actual file paths, actual ErrorCode names, actual cron expressions, actual external API endpoints.

**CRITICAL: The kickoff and push prompts MUST include the full AgentMail coordination workflow plus br-claim/br-complete wrapper discipline.** Multiple agents will work L5 in parallel. The prompts must leave ZERO ambiguity about:

- Organization model: FLAT — no hierarchy, agents pick next ready bead.
- AgentMail registration (exact MCP calls).
- Inbox check before picking work.
- Bead claim via `bin/br-claim` (atomic: file reservation + Start mail + status update).
- Bead complete via `bin/br-complete` (atomic: verify pushed + Complete mail + close + sync + release).
- File reservation conflict handling.
- Full execution loop (claim → reserve → announce → implement → verify → commit → push → release → loop).
- Session landing protocol (when fully blocked).
- **Bead completion policy** (MUST fully complete every bead — no partial work).
- **Commit workflow** (complete bead fully → verify → stage specific files → commit with `<bead-id>:` prefix → push immediately — work is NOT done until push succeeds).
- **NEVER raw `br update --status=in_progress` or `br close`** (AGENTS.md Rule 2 constitutional).

---

## Source Documents

```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md   — Spec (SOT #1)
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md  — TDD (SOT #2)
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md  — Bootstrap
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md  — Strategy (SOT #3)
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-bead-alias-map.json                      — bead inventory map
```

### Codebase (study for integration surface)
```
AGENTS.md:21-33                                    — Rule 2 (constitutional br wrapper discipline)
AGENTS.md:262-305                                  — authoritative `br` block
AGENTS.md:150-166                                  — Stage Validation Test Gate
AGENTS.md:606-634                                  — Landing the Plane (push to remote)
crates/daemon/src/errors/manifest.rs:20-55       — ErrorCode (Phase 1 target)
crates/daemon/src/sources/registry.rs:570-680      — RegisteredSource (Phase 1 target)
crates/daemon/src/scheduler/cron.rs:16-34          — cron pattern (Phase 1 target)
crates/daemon/src/main.rs:5639-6350+               — landed-cost paths (Phase 6 refactor target)
packages/intelligence/src/                         — Phase 2/3/4/6 target
skills/supplier-price-enrichment/                  — Phase 5 target (existing skills to mature)
infra/supabase/migrations/0001_projection_tables.sql  — L4 projection (auto-flow)
tests/fixtures/calibration/                        — Phase 6 gold-set extension
```

### Quality Bar (MATCH OR EXCEED — 12+ sections, 350+ lines)
```
docs/pipeline/l4-shared-visibility/archived/2026-05-08-l4-shared-visibility/10-generate-kickoff.md  — bar
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-kickoff.md                       — actual L4 output
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-push.md                          — actual L4 output
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-kickoff.md               — 366 lines; secondary
```

---

## Output Files

| File | Path | Purpose | Approx Size |
|------|------|---------|-------------|
| Kickoff | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-kickoff.md` | Full agent onboarding | 400-550 lines |
| Push | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-push.md` | Continuation nudge | 80-130 lines |
| Status | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-status.yaml` | YAML pointer to all SOT docs | 40-60 lines |

---

## Required Kickoff Sections (ALL mandatory — in this order)

### 1. Mission

- L5 description: "Layer 5 expands customs classification from screening-grade (HS6, single MFN rate, flat freight) to decision-grade (multi-source recall, 10-digit resolution, measure interaction, exporter resolution, classification freshness, dual FX, real freight). Adds 3 marketplace skill maturations (alibaba/amazon/1688) + PCS skill creation (Scotland IN-scope per operator decision 2026-05-08). Evidence-tiered accuracy across 3 goods categories: fasteners + packaging-cartons + examination-gloves. CBAM forward placeholder for 2027-01-01."
- Scope: `packages/intelligence/src/{customs,fx,logistics,validation}/` (19 new files); 3-4 daemon code edits; 1 SQLite migration; 4 cron entries; 4 marketplace skill maturations; ≥3 gold-set categories; CBAM placeholder.
- Completion milestones: 6 phases (Foundation → FX → Freight → Customs → Skills → Integration+Gold-Set+CBAM).
- Statement: "Work from the bead graph. Claim task beads, not epics. Each bead is self-contained — implement from the bead alone."

### 2. Implementation Boundary

**EXISTS (build on top of):**
- L0: full SQLite schema with all 36 decision-grade columns nullable on `landed_cost_estimates`; `supplier_quotes` supporting multiple sources via `source_name`; gold-set + canary calibration corpus at `tests/fixtures/calibration/`.
- L1: daemon runtime, PTY infrastructure, screening-grade landed-cost paths in main.rs (will be refactored in Phase 6).
- L2: desktop control panel.
- L3: validators, error_manifests, ErrorCode enum (44 variants), validation policy + executor.
- L4: SQLite-Supabase sync engine, 8 Supabase migrations, Web portal, RLS policies, CAS+command ledger, validation enforcement infrastructure.
- Existing skills: `skills/supplier-price-enrichment/{ebay,alibaba,amazon,1688}/` at `promotion_stage: candidate`.

**DOES NOT EXIST (you're building):**
- 19 new TS files in `packages/intelligence/src/{customs,fx,logistics,validation}/`.
- 10 new ErrorCode variants (Rust enum extension + clippy-surfaced propagation).
- 4-5 new RegisteredSource entries in registry.rs.
- 4 new CronJobDefinition entries in cron.rs.
- 1 new SQLite migration `migrations/NNNN_l5_product_commodity_codes.sql` (NNNN = next-free).
- Maturation deltas for alibaba/amazon/1688 skills (per spec §1.15 + §2.3, no extra gap-fill — operator confirmed clean QA-stages-1-14 baseline 2026-05-08) and PCS skill creation (Scotland IN-scope).
- ≥3 gold-set category extensions.
- CBAM placeholder logic.

"Do NOT assume unbuilt infrastructure exists. Confirm assumptions against the codebase, not the spec."

### 3. Core Contracts / Non-Negotiables

- **L0 schema is fixed.** No DDL changes to `landed_cost_estimates`. Single new table: `product_commodity_codes` (Phase 1 migration).
- **No mocks for HMRC/Xe/Freightos/SeaRates.** Use fixture cassettes for unit; real HTTP for integration. Project convention.
- **Conservative defaults are constitutional.** When resolution is incomplete: MFN, residual_default, out-of-quota, standard VAT, boundary scenarios, abstain on supplementary unit. Never relax defaults — surface the gap via `unlockable_margin_bps` / `evidence_leverage`.
- **FPO is one source among five, NEVER the primary classifier.** FPO alone is not evidence. Independent agreement requires slot-level compatibility checking.
- **UK-specificity rule.** Taric Support and HS Ping NEVER overrule HMRC on UK national measures, VAT additional codes (DE 6/17), UK quotas, UK trade-remedy implementations.
- **Never greedy descent.** Multi-family expansion to UK 10-digit must consider ALL plausible HS6/CN8 families.
- **Compound/specific duty expressions are preserved.** Never flatten to single `dutyPct`.
- **Customs branch divergence is a hard blocker, not a scenario band.**
- **L5→L6 readiness gate is constitutional.** 5 blocker conditions must be produceable: `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`, classification freshness expired.
- "If code conflicts with this contract, the contract wins."

### 4. Source of Truth Loading (MANDATORY — before starting ANY implementation)

In order:
1. `AGENTS.md` — full read for first-time agents; lines 21-33 (Rule 2 wrappers) and 262-305 (authoritative br block) for returning agents.
2. `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — full read.
3. `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md` — full read.
4. `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md` — full read.
5. `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md` — full read on first session; reference on continuation.
6. `br ready --json` — see ready beads.
7. The bead you claim — read FULLY before touching code.

### 5. AgentMail Setup (Do This ONCE At Session Start)

```text
ensure_project(project_key="/Users/zacfarrell/Desktop/Testproject1")
register_agent(project_key, program="claude-code", model="opus")
set_contact_policy(project_key, agent_name, policy="open")
fetch_inbox(project_key, agent_name, limit=20)
```

"Read and acknowledge any pending messages before starting work. If another agent left a blocker notice, address it first."

### 6. Bead Completion Policy (CRITICAL — verbatim)

(Use the exact verbatim block from `pipemaxxer/TEMPLATES.md` "Bead Completion Policy", adjusted for L5: bead prefix `l5`, sync command `bin/br-sync`.)

Key elements:
- Use `bin/br-claim <bead-id>` and `bin/br-complete <bead-id>` — only authorized path.
- AGENTS.md Rule 2 (constitutional) forbids raw `br update`/`br close`.
- Phase B precommit hook (when active) blocks any commit with a `<bead-id>:` subject that has no Start mail in the archive.
- Bead complete = code + tests + checks + committed AND pushed + br-complete ran clean.
- Blocked: diagnose, fix yourself; only escalate via AgentMail (reply in bead's thread) if the blocker is in another agent's code.
- Do NOT release reservations or close while blocked.

### 7. Work Loop (Full Visual — claim through release)

Use the ASCII diagram from `pipemaxxer/TEMPLATES.md` "Required Kickoff Sections — 7. Work Loop", adjusted for L5:
- Bead CLI is `br` (not `bd`).
- Sync command is `bin/br-sync`.
- Specific verification commands per stack surface (Rust + TS).
- Project key: `/Users/zacfarrell/Desktop/Testproject1`.

### 8. Verification Commands (per stack surface)

```bash
# Rust (daemon — Phase 1 + 6 surface)
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
# AGENTS.md:150-166 — must also iterate every integration test:
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done

# TypeScript (packages/intelligence — Phase 2-6 surface)
bun run build
bun run lint
bun run test

# UBS (Ultimate Bug Scanner — pre-commit)
ubs $(git diff --name-only --cached)
```

"Use debug builds for fast iteration. Never proceed with failing checks. Match L4's discipline."

### 9. File-Surface Map by Phase

For each L5 phase, list the specific files in scope. Prevents unnecessary exploration.

| Phase | Scope | Key Files |
|-------|-------|-----------|
| 1 Foundation | Schema + daemon glue | `migrations/NNNN_l5_product_commodity_codes.sql`; `crates/daemon/src/errors/manifest.rs:20-55+` (ErrorCode enum + as_str() matcher; expect 5-15 clippy-surfaced propagation sites in `validation/policy.rs`, `validation/executor.rs`, `errors/markdown.rs`, `tracing/structured.rs`, `ipc/handlers/*`); `crates/daemon/src/sources/registry.rs:570-680`; `crates/daemon/src/scheduler/cron.rs:16-34` (constants 16-23, CronJobDefinition struct 26-34); `crates/daemon/src/db/sync.rs:24-49` (PUSH_OWNED_RESEED_TABLES — `landed_cost_estimates` at line 35; add `product_commodity_codes`) |
| 2 FX | FX engine | `packages/intelligence/src/fx/{hmrc-exchange-rates,xe-commercial-rates,fx-engine,fx-types}.ts` |
| 3 Freight | Freight engine | `packages/intelligence/src/logistics/{mode-selector,freightos-client,searates-client,freight-engine,freight-types}.ts` |
| 4 Customs | Customs engine | `packages/intelligence/src/customs/{description-quality-gate,multi-source-recall,ten-digit-resolution,measure-scanner,exporter-resolution,classification-freshness}.ts` |
| 5 Skills | Marketplace + validator overlay | `skills/supplier-price-enrichment/{alibaba,amazon,1688}/SKILL.md` + schemas (mature candidate→stable); `skills/procurement-ingestion/pcs/` (full skill directory — Scotland IN-scope per operator decision 2026-05-08); `skills/validation/overlay-customs-classification/` (recommended) |
| 6 Integration | Gold-set + CBAM + accuracy + comparison + main.rs refactor | `packages/intelligence/src/customs/accuracy.ts`; `packages/intelligence/src/validation/{dutify,zonos,avalara}-adapter.ts`; `tests/fixtures/calibration/customs-classification/{category}/`; `crates/daemon/src/main.rs:5639-6350+` (refactor) |

### 10. Dependency Spine (visual mental model)

```
Phase 1 (Foundation)
   │
   ├─→ Phase 2 (FX)        ─┐
   │                         │
   ├─→ Phase 3 (Freight)    ─┼─→ Phase 6 (Integration + Gold-Set + CBAM + Refactor)
   │                         │
   ├─→ Phase 5 (Skills)     ─┤
   │                         │
   └─→ Phase 4 (Customs) ←──┘
            (Phase 4 depends on Phase 1 + 2 — FX needed for valuation)
```

"Trust `br ready`, but use this spine as your mental model. Phase 1 is the long pole; Phase 4 is the heaviest content; Phase 6 closes everything."

### 11. AgentMail Quick Reference

(Use the exact verbatim block from `pipemaxxer/TEMPLATES.md` "Required Kickoff Sections — 11. AgentMail Quick Reference", with project key `/Users/zacfarrell/Desktop/Testproject1`.)

Key reminders:
- NO broadcast. `"broadcast"` is NOT a valid recipient.
- `bin/br-claim` and `bin/br-complete` wrappers handle Start/Complete mail automatically.
- Use `send_message` directly only for non-claim/complete events: blocker reports, mid-implementation questions, hand-offs.
- Always discover peers via `list_contacts(project_key, agent_name)` first.
- Refresh contacts between beads.

### 12. L5-Specific Pitfalls

- **ErrorCode propagation:** adding 10 variants to a Rust enum surfaces 5-15 exhaustive match sites. `cargo clippy --all-targets -- -D warnings` will surface them. Fix all in the Phase 1 ErrorCode bead.
- **Multi-family expansion is computationally cheap when classification lattice is separated from commercial scenario lattice.** Recall → local expansion → local scoring → shortlist → external validation only on shortlisted candidates. Do NOT cartesian-product 200 codes × 3 quotes × 2 packings × 3 Incoterms × 3 exporter bases × 2 quota states × 2 FX snapshots — that's 43,200 tuples. Shortlist FIRST.
- **HMRC FPO endpoint name unverified.** Spec §1.4 explicit: "verify the correct API endpoint name during implementation". The Phase 4 multi-source-recall bead has this as an explicit prerequisite. Resolve before integrating FPO.
- **PCS Scotland scope:** IN-SCOPE per operator decision 2026-05-08 (manifest `recorded_decisions.decision_2_pcs_scotland_scope`). PCS skill creation is mandatory in Phase 5; daemon registry entry required; bootstrap covers PCS OCDS Web API.
- **CBAM is placeholder-only in L5.** Rate-bearing logic awaits secondary legislation. Don't ship rate logic.
- **Stage 11 live demo gate (Phase 6):** must run against a real Contracts Finder lot before L5 closes.

---

## Required Push Sections (ALL mandatory)

The push prompt is short (80-130 lines) — for continuation agents who already know the project.

### 1. Re-Orient
- Re-read `AGENTS.md`.
- L5 description: 1 line.
- Spec / TDD / strategy paths.
- Bead command: `br ready --json` for ready beads.

### 2. Core Contracts (compact)
- Conservative defaults are constitutional.
- FPO never primary; UK-specificity rule.
- Compound/specific duty preserved.
- L5→L6 gate: 5 blocker conditions.

### 3. Bead Completion Policy (compact but firm)

(Use the exact verbatim compact block from `pipemaxxer/TEMPLATES.md` "Required Push Sections — 3. Bead Completion Policy".)

### 4. Work Loop (compact)

(Use the exact verbatim compact block from `pipemaxxer/TEMPLATES.md` "Required Push Sections — 4. Work Loop", with bead-cli=br, bead-prefix=l5, project-key=/Users/zacfarrell/Desktop/Testproject1.)

### 5. Verification Commands (compact, per stack surface)

(Per stack surface — Rust + TS — same commands as kickoff §8.)

### 6. Dependency Spine (compact, one-liner per phase)

```
Phase 1: schema + daemon glue (root)
Phase 2: FX engine (parallel after 1)
Phase 3: Freight engine (parallel after 1)
Phase 4: Customs engine (after 1 + 2)
Phase 5: Skills (parallel after 1)
Phase 6: Integration + gold-set + CBAM + main.rs refactor (after 4 + 3 + 5)
```

### 7. File Surfaces (compact table)

| Phase | Key Files |
|-------|-----------|
| 1 | migrations/NNNN_l5_*.sql; daemon validation/policy.rs, sources/registry.rs, scheduler/cron.rs, db/sync.rs |
| 2 | packages/intelligence/src/fx/*.ts |
| 3 | packages/intelligence/src/logistics/*.ts |
| 4 | packages/intelligence/src/customs/*.ts |
| 5 | skills/supplier-price-enrichment/{alibaba,amazon,1688}/; skills/procurement-ingestion/pcs/ (if scoped); skills/validation/overlay-customs-classification/ (optional) |
| 6 | packages/intelligence/src/customs/accuracy.ts; packages/intelligence/src/validation/*.ts; tests/fixtures/calibration/customs-classification/{category}/; daemon main.rs refactor |

### 8. AgentMail Quick Reference (compact)

(Compact form: discover peers, fetch_inbox, send_message for blockers, release_file_reservations.)

---

## Status YAML

```yaml
feature: l5-expanded-commercial-intelligence
milestone: M5
spec: docs/strategy/layers/layer-5-expanded-commercial-intelligence.md
tdd: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md
bootstrap: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md
strategy: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md
kickoff: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-kickoff.md
push: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-push.md
bead_prefix: l5
bead_cli: br
bead_sync_cmd: bin/br-sync
agentmail_project_key: /Users/zacfarrell/Desktop/Testproject1
phases:
  1: { name: Foundation, beads: <count>, status: pending }
  2: { name: FX Engine, beads: <count>, status: pending }
  3: { name: Freight Engine, beads: <count>, status: pending }
  4: { name: Customs Engine, beads: <count>, status: pending }
  5: { name: Skills (Marketplace + Validator Overlay), beads: <count>, status: pending }
  6: { name: Integration + Gold-Set + CBAM + main.rs refactor, beads: <count>, status: pending }
exit_criterion: |
  System handles >=3 goods categories with decision-grade landed cost and >=2 supplier sources
  per category. All 12 DoD criteria pass. L5→L6 readiness gate (5 blocker conditions) is producible.
```

---

## Step-by-Step Workflow

### Step 0: Read Source Documents

1. All L5 SOTs (spec, TDD, bootstrap, strategy).
2. The L4 kickoff + push (this is your bar).
3. AGENTS.md sections 21-33, 150-166, 262-305, 606-634.

### Step 1: Invoke `/beads-to-execution`

Inputs:
- All SOT docs.
- Bead inventory (`br list --json`).
- Project key, bead prefix, bead CLI.

### Step 2: Verify Output

- [ ] Kickoff exists, ≥400 lines.
- [ ] Kickoff has all 12 required sections.
- [ ] Push exists, ≥80 lines.
- [ ] Push has all 8 required sections.
- [ ] Status YAML exists, has all SOT pointers.
- [ ] Bead completion policy verbatim from template.
- [ ] AgentMail quick reference verbatim with correct project key.
- [ ] Verification commands match L4 conventions.

### Step 3: NO Sub-Optimization

These ARE the deliverables. They are user-facing. They are not iteratively self-improved. Generate, verify, ship.

---

## ⛔ MANDATORY STOP — Human Authorizes Execution

After step 10, generation is complete. **Before agents begin executing L5 implementation, the human operator must:**

1. Read kickoff + push (review against L4 quality bar).
2. Verify AgentMail registration block uses `/Users/zacfarrell/Desktop/Testproject1`.
3. Verify `br ready --json` shows the L5 bead graph.
4. Verify env vars from bootstrap are set (or scoped delegation explicit).
5. Authorize parallel agent execution.

DO NOT proceed to step 11/execution without human authorization.

---

## Key Reminders

- These docs are user-facing — no TODO, no [placeholder], no enrichment markers.
- Bead CLI is `br`. Sync is `bin/br-sync`. AGENTS.md Rule 2 is constitutional.
- L5→L6 gate is constitutional — every blocker condition must be produceable by L5's end.
- ZERO SCOPE DRIFT.
