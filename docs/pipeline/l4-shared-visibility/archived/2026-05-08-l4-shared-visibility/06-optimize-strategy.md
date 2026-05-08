# L4 Shared Visibility — 06 · Optimize Strategy (Fresh-Eyes Audit)

## First Action

**No skill is invoked.** This is a manual fresh-eyes audit by an independent agent (NOT the agent that ran step 05). Open the strategy doc, work through the 12 audit dimensions below, edit in place to fix issues, then run the follow-up pass.

---

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the L4 strategy doc created in step 05. The strategy is SOT #3 — beads will be grouped by phase and depend on the phase ordering. If the strategy reorders or drops phases, beads break.

**You are reading and improving an existing document — not writing code.**

The strategy's role is to answer "in what order does an agent claim beads, and which beads can run in parallel?" — nothing more. It must NOT duplicate spec or TDD content; references only.

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md   — created in step 05 — READ FULLY
```

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-4-shared-visibility.md                              — Spec (SOT #1)
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md       — TDD (SOT #2)
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md — Bootstrap
```

### Codebase (GROUND TRUTH for dependency ordering)
```
crates/daemon/src/main.rs                           — 1.08 MB monolithic init (single-edit chokepoint)
crates/daemon/src/db/sqlite.rs                      — 600 lines (schema first)
crates/daemon/src/jobs/runner.rs                    — 1027 lines (single enqueue boundary; chokepoint)
crates/daemon/src/validation/policy.rs              — 959 lines (chokepoint; L4 extends)
crates/daemon/src/validation/executor.rs            — 1217 lines (chokepoint; L4 extends with provisional promotion)
crates/daemon/src/ipc/handlers/decisions.rs         — extends with CAS flush
crates/daemon/Cargo.toml                            — net-new HTTP client dep
package.json (root)                                 — net-new apps/web workspace
migrations/                                         — flat sequence; L4 owns next NNNN
infra/supabase/                                     — net-new directory
apps/web/                                           — net-new directory
skills/validation/base/                             — template for 4 overlays
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md  — 478 lines, 12 sections, 6 phases
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**Phases are execution ORDER, not importance tiers.** Every phase ships with the same depth as early phases. The strategy may NOT label a phase as "stretch", "polish", "nice-to-have", or "post-MVP". Phase 6 is a real phase with real gates, not a junk drawer for deferred work.

**Common drift attractors to watch for:**
- "Phase 6: Polish" — usually a junk drawer
- "Optional Phase N+1" — there are no optional phases
- A phase with 1-2 beads while others have 8-12 — investigate; usually scope was silently cut
- Gate criteria like "when ready" or "when complete" — must reference test IDs
- "We can add this later if needed" — never; if it's in spec, it's in scope
- "Simplified for initial implementation" — means the spec features are silently weakened

---

## Think Independently — 12 Audit Dimensions

### 1. Phase Coverage
- Does every spec L4-FR have a phase home? Walk every L4-FR-X.Y.Z and verify it lives in some phase's bead inventory.
- Are the spec's §1.10 components (Database, daemon `db/sync.rs`, Skills × 4 overlays, `apps/web` × 7 files) all phase-owned?
- Are the spec's §5.4 dependencies (Supabase project, 8 migrations, `db/sync.rs`, 4 overlays, web app) all addressed early enough to unblock downstream phases?

### 2. Phase Coverage Equality
- Count beads per phase. Compare distributions.
- A 12-2 ratio is suspicious. Either Phase N has hidden complexity not yet broken down, or it was silently scope-cut.
- L3 had: Phase 1 = 5, Phase 2 = 5, Phase 3 = 5, Phase 4 = 10, Phase 5 = 4, Phase 6 = 7 (total 36 task beads + 6 review-gate epics = 42). L4 should be in similar order of magnitude — likely 30-50 task beads.

### 3. Critical Dependency Ordering
- Phase 1 ships **before** Phase 2 (schema before sync.rs)
- Phase 1 also ships HTTP client dep before any push-engine code
- Phase 1 also ships `apps/web` workspace edit before any Next.js bead
- Within Phase 2: outbox writer → push dispatcher (writer is upstream)
- Within Phase 3: pull cursor → CAS RPC → reconciliation (each consumes prior)
- Phase 4 enforcement assumes provisional buffer from same phase + executor extension is upstream of overlays
- Phase 6 wires `main.rs` startup sequence (pull-before-push, lease, schema_epoch); must be LAST and atomic
- If any of these orderings is violated, that's a critical failure

### 4. Single-Edit Chokepoint Discipline
The following files are single-edit chokepoints. Strategy must NOT have multiple beads in different phases editing them simultaneously:
- `crates/daemon/src/main.rs` — designate ONE integration bead per phase that touches it (or just one phase-6 wiring bead)
- `crates/daemon/src/jobs/runner.rs` — single bead extends with provisional promotion
- `crates/daemon/src/validation/policy.rs` — single bead extends with per-slice readiness
- `crates/daemon/src/validation/executor.rs` — single bead extends with quarantine path
- `package.json` (root) — single bead adds `apps/web` to workspaces + scripts
- `crates/daemon/Cargo.toml` — single bead adds HTTP client

### 5. Parallelism Discipline
- After Phase 1, Phases 2 and 3 should be parallelizable (different DB modules, no shared mutable state in code)
- Phase 5 (web) can start as soon as Phase 2 has populated projection tables; doesn't need full Phase 4
- Phase 4 overlays (4 directories) are pure filesystem — fully parallel within Phase 4
- Phase 6 integration is the serialization point — wait for everything else
- If the strategy doesn't explicitly mark parallelism, mark it now

### 6. Gate Criterion Quality
Every phase needs a gate criterion that:
- References TDD test IDs (not just "tests pass")
- Specifies the invocation command (`cargo test --test sqlite_supabase_sync`, `(cd apps/web && bun run test:e2e)`)
- Is observable and binary (passes or fails, not "looks good")

Examples:
- Bad: "Phase 2 gate: push engine works"
- Good: "Phase 2 gate: `L4-T-sqlite_supabase_sync-001` through `L4-T-sqlite_supabase_sync-{push-engine-end}` pass under `cargo test --test sqlite_supabase_sync` AND `cargo clippy --all-targets -- -D warnings` is clean"

### 7. Phase Sub-Epic Mapping
The existing L4 epic `testproject1-4zq` has zero children. The strategy should suggest one of:
- Keep as a single epic; phases become labels on task beads
- Sub-divide into 4-6 sub-epics (e.g., `l4-schema-foundation`, `l4-push-engine`, `l4-pull-cas`, `l4-validation-enforcement`, `l4-web-portal`, `l4-integration`) — each phase becomes a sub-epic; task beads parent to sub-epic

L3 used the sub-epic approach. Verify the strategy makes the choice and commits.

### 8. Calibration Harness Placement
The calibration harness (3 corpora, 9 mutations, pairwise, Wilson-bound math, rule-of-three math, cross-model audit, override loop) is:
- Phase 4 capstone (depends on validator overlays + per-slice readiness gate)
- Or split between Phase 4 (corpora + harness mechanics) and Phase 6 (override loop wiring + cross-model sampling cadence)

Strategy should make the call and place beads accordingly. If absent, add.

### 9. Two-User Verification
"Two users can view the same pipeline: desktop operator (read-write) + web viewer (read-only)" is DoD criterion #12. This is a manual verification bead in Phase 6. Strategy must call out:
- Setup: one operator on desktop with full write rights; one viewer on web with viewer credentials
- Verification: viewer sees what operator does (within sync latency); viewer cannot mutate; sync state badges match
- Pass criteria

### 10. Schema Hash CI Gate
The spec §2.6 says "CI must apply both migrations from scratch, introspect both, and fail if logical schema hash differs." This is a CI gate. Strategy must place a bead in Phase 1 (after both migrations land) to add this CI rule.

### 11. Logical Consistency Auditor Stub
§5.2 lists "Logical consistency auditor" as a stub for L4 (Future: full implementation). Strategy must verify the stub IS in Phase 6 (minimum stub, not deferred entirely). The stub is "domain invariant checking with conditional auto-repair" — even minimum-stub form must be testable.

### 12. Cross-Layer Touch Points
- L0 cross-domain note: `commit_group_id`, `writer_epoch` columns on `local_outbox` (Phase 1 schema bead)
- L2 cross-domain note: 3-state FreshnessBadge upgrade (Phase 5 web bead, but also touches `packages/ui/freshness-badge.tsx` — flag if a bead simultaneously edits the desktop-side use)
- L6 cross-domain note: remote config staleness classes safety-critical TTL (Phase 4 or Phase 6 wiring bead)

If any cross-layer touch point is missing from the strategy, flag.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
Search the strategy for:
- `post-MVP` / `post mvp`
- `Phase 2+` (in a meta sense — referring to a future cross-pipeline phase, not L4 phase 2)
- `stretch goal` / `stretch-goal`
- `nice to have` / `nice-to-have`
- `optional`
- `if time permits`
- `deferred to L5/L6` (or any phasing that pushes L4 work to a later layer)
- `Polish`, `Cleanup`, `Final touches` (when used as a phase title — these are euphemisms for scope drift)
- `simplified for initial implementation`
- `for now`
- `eventually`

### Coverage Verification

- [ ] Every L4-FR-X.Y.Z has a phase home
- [ ] Every of the 18 DoD criteria has a phase home
- [ ] Every of the 14 invariants has a phase home
- [ ] Every of the 4 validator overlays has a Phase 4 bead
- [ ] Every of the 7 web files has a Phase 5 bead (or named in a multi-file Phase 5 bead)
- [ ] Every of the 8 Supabase migrations has a Phase 1 bead (or grouped into a "Supabase migrations bundle" Phase 1 bead)
- [ ] Cross-domain notes (L0, L2, L6) have phase homes
- [ ] Two-user verification is a Phase 6 bead
- [ ] Schema hash CI gate is a Phase 1 (or 6) bead
- [ ] Calibration harness is a Phase 4 capstone bead

### Drift Checklist

- [ ] No "post-MVP" / "Phase 2+ (meta)" / "stretch goal" / "nice to have" / "deferred" / "future iteration" / "simplified for initial" / "for now" / "eventually" / "optional" / "if time permits" language in load-bearing positions
- [ ] No phase has 1-2 beads while others have 8-12 (unless documented as small)
- [ ] No spec L4-FR is missing a phase home
- [ ] Every phase has a gate criterion referencing TDD test IDs
- [ ] Every chokepoint file has at most one phase-bead concurrently editing it
- [ ] Single-edit chokepoint discipline (`main.rs`, `runner.rs`, `policy.rs`, `executor.rs`, root `package.json`, daemon `Cargo.toml`) is preserved
- [ ] Strategy says: "Phases are EXECUTION ORDER, not importance tiers. Every phase ships." (or equivalent)
- [ ] Strategy says: "Strategy provides ORDERING ONLY. Spec and TDD content is referenced, not duplicated."
- [ ] No spec content is duplicated; no TDD test bodies are inlined

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

ULTRATHINK.

Re-read the fixes you just made. Did they introduce new issues?

1. **Open `crates/daemon/src/main.rs`** (skim only). The strategy must NOT have multiple Phase 2-5 beads editing `main.rs` simultaneously. Walk your fixed strategy: if more than one bead in different phases edits `main.rs`, that's a merge-conflict trap. Designate ONE bead per phase or ONE phase-6 wiring bead.

2. **Open `crates/daemon/src/validation/policy.rs`** (959 lines). L4 extends with per-slice enforcement readiness logic. Verify your strategy has exactly ONE bead extending policy.rs, and it precedes any bead that depends on its outputs (e.g., the executor extension consuming readiness decisions).

3. **Open `crates/daemon/src/validation/executor.rs`** (1217 lines). L4 extends with provisional promotion and quarantine path. Verify your strategy has ONE bead extending executor.rs. Verify it depends on: (a) policy.rs extension (per-slice readiness consumed by executor), (b) provisional buffer staging table from Phase 1, (c) AGENT_SUSPECT_OUTPUT error type from `errors/manifest.rs`.

4. **Open `crates/daemon/src/jobs/runner.rs`** (1027 lines). L4 extends with provisional-write path on enforcement-active slices. Verify your strategy has ONE bead extending runner.rs. Verify it depends on: (a) executor.rs extension, (b) provisional staging mechanism. Verify no Phase 5 web bead touches runner.rs.

5. **Open `apps/desktop/src/pages/ContractList.tsx`** (and ContractDetail, DaemonStatus). These are the templates for the corresponding `apps/web/src/app/...` pages. Verify your strategy's Phase 5 has beads that REUSE `packages/ui` components but do NOT modify them. Verify no Phase 5 bead simultaneously edits both desktop and web pages.

6. **Open `package.json` (root) workspaces array.** Verify your strategy's Phase 1 has a single bead "Add `apps/web` to workspaces + extend `bun run {build,test,lint}` scripts". Verify the lint rule (banning desktop/tauri imports from `packages/ui`) is preserved (the bead must not weaken it).

7. **Walk every phase's gate criterion.** For each, ask: "Could a CI runner machine determine pass/fail from this gate alone?" If the gate says "tests pass" without naming files or test IDs, that's a vague gate.

8. **Walk parallelism statements.** If Phase 2 says "depends on Phase 1" but Phase 3 also says "depends on Phase 1", does the strategy explicitly say Phase 2 and Phase 3 can run concurrently? If not, agents may serialize unnecessarily.

9. **Verify Phase 6 has real content** (not a "wrap-up" / "polish" phase). Phase 6 should be: `main.rs` wiring + RLS policy file + redaction tests + tombstone retention + logical consistency auditor stub + cross-authority provenance + two-user verification + schema hash CI gate + DoD walkthrough. That's 5-8 substantive beads, not "polish".

10. **Re-scan strategy for scope-drift language**:
    - `post-MVP`, `Phase 2+ (meta)`, `nice to have`, `stretch goal`, `deferred`, `future iteration`, `simplified for now`, `can be added later`, `eventually`, `for now`, `optional`, `if time permits`, `Polish`, `Cleanup`, `wrap-up`
    - Hits in load-bearing positions = BLOCKING.

11. **Final sanity check.** The strategy should be reasonably close in length to L3's (478 lines, 12 sections). If it's much shorter (200 lines), it's missing detail. If much longer (1000+ lines), it's duplicating spec/TDD content — strip those.

12. **Verify the existing L4 epic decision is reflected.** `testproject1-4zq` exists with no children. Either the strategy says "keep single epic, phases become labels" or "sub-divide into 6 sub-epics". Both are valid; commit to one.
