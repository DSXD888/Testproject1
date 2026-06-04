# Goods-Led Install/Maintenance — Generate Kickoff + Push Prompts

## First Action

Invoke the `/beads-to-execution` skill.

---

## Your Mission

Generate the **kickoff prompt** (full agent onboarding), **push prompt** (continuation nudge), and **status file** for the goods-led install/maintenance feature. These are the documents agents actually USE during implementation — deeply feature-specific, referencing actual file paths, types, and commands.

**CRITICAL: The kickoff and push MUST include the full AgentMail coordination workflow.** Multiple agents may work in parallel. Leave ZERO ambiguity about: FLAT organization (no hierarchy; pick next ready bead), registration, inbox checks, claiming (file reservation + Start mail + status), conflict handling, the full execution loop, session landing, bead completion policy (no partial work), and the commit→push→complete workflow.

> **{ENRICHMENT — after Batch 4}** The kickoff's "Dependency Spine" and "File-Surface Map by Epic" must use the ACTUAL phase epics + bead IDs. If the enrichment pass has run, the implementation epic ID and phase distribution are pinned in `manifest.yaml` → `implementation_epic`. Pull the real epic/sub-epic IDs from `br list --label goods-led-install --json` into §9 and §10 of the generated kickoff — do not leave the spine generic.

---

## Source Documents

```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md       — SOT #1
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md          — SOT #2
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-bootstrap.md     — environment setup
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-strategy.md     — SOT #3
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-architecture.md  — architecture
docs/pipeline/goods-led-install-maintenance/manifest.yaml                                            — paths, epic ID, gates
AGENTS.md                                                                                            — Rule 2 (constitutional), br block lines 279-330, AgentMail section
```

### Codebase (study for the integration surface)
```
packages/intelligence/src/qualification/goods-qualification.ts:264-372   — Phase-1 branch, density, addressable value
packages/intelligence/src/customs/description-quality.ts:23-36,100-113,360 — service-gate split (+ NEW test file)
crates/daemon/src/main.rs:5361-5432,8086,8114,8324,8548,8933,9064,9136-9290 — write struct, both rollups, persistence, live loop
packages/contracts-schema/src/index.ts:36,166,216                          — entities to extend
packages/intelligence/src/margins/coverage-rollup.ts                       — value-share calc
skills/landed-cost/simple-fx-lookup/                                       — http_public skill template
skills/landed-cost/freightos-freight-estimate/                             — result-envelope template
skills/validation/overlay-supplier-enrichment/                             — overlay template
crates/daemon/tests/customs_fx_freight_engine.rs, decision_basis_hash.rs, skill_canary_harness.rs, skill_lint.rs   — REAL test harnesses
```

### Quality Bar (MATCH THIS LEVEL OF DETAIL)
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-kickoff.md   — best existing kickoff (study the AgentMail + work-loop sections)
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-push.md       — best existing push
```

---

## Output Files

| File | Path | Purpose |
|------|------|---------|
| Kickoff | `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-kickoff.md` | Full agent onboarding (~250-400 lines) |
| Push | `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-push.md` | Continuation nudge (~80-120 lines) |
| Status | `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-status.yaml` | YAML pointer to all SOT docs + epic ID |

---

## Required Kickoff Sections (ALL mandatory — in this order)

### 1. Mission
- WHAT: widen the filter for goods-LED install/maintenance contracts + build a UK labour-cost engine (both halves ship together).
- WHY: the funnel is too thin on pure-supply; a large slice of public goods spend is supply-and-install.
- Scope: `packages/intelligence` (qualification + customs + margins), `crates/daemon/src/main.rs` (rollup + live loop), `packages/contracts-schema`, `packages/ui`, `skills/landed-cost/uk-installation-labour-estimate`, `skills/validation/overlay-labour-cost`, migrations + Supabase projection, `docs/strategy/layers/layer-0-gold-set-evaluation.md`.
- "Work from the bead graph. Claim task beads, not epics. Each bead is self-contained — implement from the bead alone."

### 2. Implementation Boundary
- **EXISTS (build on top of):** the landed-cost live loop + `run_landed_cost_skill` (retry/result-contract/timeout); freight/customs/FX skills (the transient-load pattern); `determineQualificationDecision` + `computeAddressableValue` + `computeGoodsDensityScore`; the `DescriptionQualityVerdict` union; both rollup functions; the calibration-fixture/canary harness; `skill_lint.rs`; the unused `uk_final_mile_cost_gbp_pence` field.
- **DOES NOT EXIST (you're building):** the `uk-installation-labour-estimate` skill + 4 reference JSONs; `overlay-labour-cost`; the labour/permit fields + migration; the Phase-1 branch + Phase-2 gate + `goods_led_mixed` variant + no-list/bespoke flag; `MaintenancePeriodEstimate`; `description-quality.test.ts` (NEW); `installation_labour_engine.rs` (NEW). **No `registry.rs` edit. No `next_job_kind` chaining.**

### 3. Core Contracts / Non-Negotiables
- Money = integer pence; ratios/confidence = bps (10000=100%); saturating arithmetic.
- Estimates are **screening-grade ±20%**, never firm quotes; carry VAT + "what's excluded" flags.
- **Fail-soft:** a `blocked` labour result degrades confidence + routes to `review`; it never crashes the pipeline (mirror freight `blocked`).
- The labour skill is `http_public`, no browser; must NOT join `CDP_BROWSER_SKILLS`/`SIGNED_IN_BROWSER_SKILLS`.
- Labour summed at BOTH rollup sites; new columns nullable/default-0; labour added ONLY when an `InstallationRequirement` exists (Scenario 8 stays labour=0).
- Near-floor value-share cases route to `review`, never auto-reject; prefer the contract-value basis when available.
- "If code conflicts with this contract, the contract wins."

### 4. Source of Truth Loading (MANDATORY before any implementation)
Ordered list with READ/SKIM annotations: AGENTS.md → spec → TDD → strategy → bead graph (`br ready --label goods-led-install --json`) → the key codebase files above.

### 5. AgentMail Setup (Do This ONCE At Session Start)
```
ensure_project(project_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(project_key, program="claude-code", model="opus")  # export AGENT_MAIL_NAME
set_contact_policy(project_key, agent_name, policy="open")
# Bootstrap contact handshakes BEFORE any bin/br-claim (Rule 2 prerequisite):
#   list_agents(project_key) → same-program live peers →
#   macro_contact_handshake(project_key, requester=agent_name, target=peer["name"])
#   verify each target in list_contacts(project_key, agent_name)
fetch_inbox(project_key, agent_name, limit=20)
```
"Read and acknowledge any pending messages before starting work."

### 6. Bead Completion Policy (CRITICAL — include verbatim per TEMPLATES.md)
Use `bin/br-claim <id>` / `bin/br-complete <id>` — the ONLY authorized path (AGENTS.md Rule 2, constitutional). Never raw `br update`/`br close`. Fully complete every bead (all code + all tests + fmt/clippy/test green + committed AND pushed + `bin/br-complete` ran clean). No partial work, no mid-way commits, no "finish later." If blocked: diagnose, fix yourself; only escalate via AgentMail (reply in the bead's thread) if the blocker is in another agent's code — do NOT release reservations while blocked.

**Research-spike carve-out (the 3 Tier-B beads — `research-spike` label):** these produce a research/design OUTPUT, not code — so their "complete" definition differs. A `research-spike` bead is COMPLETE when its acceptance (a committed design note / decision on the §10 question / a defined extension point, per the bead body) is met, committed, and pushed — there is no test suite to pass. The generated kickoff MUST state this carve-out explicitly so an agent claiming a research-spike bead doesn't block on a nonexistent test gate. All other beads follow the standard code-completion policy above.

### 7. Work Loop (Full Visual — claim through release)
Generate the ASCII work loop (PICK → CLAIM via `bin/br-claim` → IMPLEMENT TDD → VERIFY → COMMIT+PUSH → COMPLETE via `bin/br-complete` → LOOP). Use the project's exact wrapper semantics (file reservation FIRST, Start mail, status; `br-complete` verifies pushed then mail+close+sync+release).

### 8. Verification Commands (the FULL AGENTS.md stage gate)
```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done   # incl. the NEW installation_labour_engine
bun test                                                                                          # all packages/intelligence *.test.ts
```
"A passing `cargo test -p daemon --bin daemon` alone is NOT green — every integration target must run."

### 9. File-Surface Map by Epic/Lane
For each phase epic: the specific files in scope (qualification: `goods-qualification.ts` + `description-quality.ts` + tests; rollup: `main.rs` both sites + write struct; schema: migration + 4 epoch constants + Supabase; skill: `skills/landed-cost/uk-installation-labour-estimate/*` + reference JSONs; etc.).

### 10. Dependency Spine
schema/types → reference data + labour skill → rollup integration → qualification/Phase-2 gate → recurring maintenance → surfacing → overlay/gold-set/rate-refresh → Tier-B research spikes. "Trust `br ready`, but use this spine as your mental model. The Phase-2 gate cannot ship before the rollup produces labour (the two halves ship together). The three Tier-B `research-spike` beads (live enrichment lane, installer-quote ingestion, detailed per-zone permit) are research/design deliverables — don't expect full implementation."

### 11. AgentMail Quick Reference
NO broadcast. Wrappers send Start/Complete mail (discover peers via `list_contacts` first). `send_message` directly only for blockers/questions/hand-offs.

---

## Required Push Sections (ALL mandatory)
1. Re-Orient (re-read AGENTS.md; feature 1-liner; spec/strategy paths; `br ready --label goods-led-install --json`)
2. Core Contracts (compact: integer pence/bps; screening-grade ±20%; fail-soft; labour both sites + nullable; near-floor → review; http_public no-CDP)
3. Bead Completion Policy (compact but firm — wrappers only; complete = code+tests+checks+pushed+br-complete)
4. Work Loop (compact 9-step)
5. Verification Commands (the full AGENTS.md stage gate)
6. Dependency Spine (one line per phase)
7. File Surfaces (compact table)
8. AgentMail Quick Reference (compact)

---

## NO sub-optimization — these ARE the deliverables.

After generating, write the status file and STOP.

⛔ **MANDATORY STOP** — Human authorizes execution before any bead is claimed.
