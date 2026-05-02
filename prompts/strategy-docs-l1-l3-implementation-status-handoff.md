# Strategy Docs — L1-L3 Implementation Status Annotation

ULTRATHINK and be meticulous.

## MISSION

The 7 strategy docs in `docs/strategy/layers/` (layer-0 through layer-6) were drafted before Stages 11-14 + Phase 2 + multi-source landed. Each layer doc claims authority ("An implementation agent should be able to build this layer from this document alone"), but L1-L3 are now BUILT — and reality has diverged from the spec in places (e.g., the IPC handler module added in Stage 14 Phase 2 isn't documented as a design choice in L2; the OrderQuantityBasis::BasisUndetermined variant added in Stage 11 hardening isn't reflected in L3's landed-cost section).

This handoff annotates each L1/L2/L3 strategy doc with **implementation status callouts** — for every load-bearing design decision, mark "✓ Implemented as designed", "⚠ Implemented with deviation", "✗ Deferred", or "↻ Superseded". Where divergence happened, document the actual implementation. L4-L6 docs remain "planned" (they ARE planned — nothing built yet).

This prevents the situation where a future implementation agent reads `layer-2-single-operator-desktop.md`, treats it as authoritative, and rebuilds something that already exists.

You will **read each layer doc → cross-reference code → annotate → review → refine** in a loop until R.1–R.10 are GREEN.

```
┌────────────────────────────────────────────────────────────┐
│                          THE LOOP                            │
│   1. Pre-flight (§0)                                         │
│   2. For each of L1, L2, L3:                                 │
│      a. Read top-to-bottom                                  │
│      b. For each design decision: annotate with status     │
│      c. Where deviation: write 1-paragraph "actual impl"   │
│   3. Update layer-strategy.md to reflect L1-L3 = built     │
│   4. Update cross-reference-index.md                        │
│   5. Update README.md                                        │
│   6. Read whole strategy directory end-to-end → does it    │
│      reflect today's reality?                                │
│   7. All green → write the validation manifest               │
└────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## §0 — PRE-FLIGHT

| Claim | Verify |
|---|---|
| Strategy docs exist at `docs/strategy/{README.md, cross-reference-index.md, layer-strategy.md, layers/layer-0..6-*.md}` (10 files total) | `ls docs/strategy/ docs/strategy/layers/` |
| Each layer doc opens with "**Authority:** This document is the execution-grade specification for Layer N..." | `for f in docs/strategy/layers/*.md; do head -5 "$f"; done` |
| Implementation status (per MEMORY.md): L1+L2+L3 implemented; L4-L6 not started; Stage 15 L6-blocked | (cross-reference) |
| Per-stage validation manifests document specific implementation choices (e.g., `docs/pipeline/stage-14-phase-2-validation-manifest.md` documents the IPC handler module) | `ls docs/pipeline/*validation*manifest*.md \| head -10` |
| Major code surfaces to cross-reference: `crates/daemon/src/main.rs` (~24k lines), `crates/daemon/src/ipc/`, `apps/desktop/src-tauri/src/`, `apps/desktop/src/`, `skills/` | high-level browse |
| `agent-mail` healthy; commits need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Parallel-safety**: this handoff modifies docs only (`docs/strategy/*.md`). Safe to run alongside any other handoff including the post-merge gate sweep, golden journey, CDP demos, and test flake cleanup. The only file collision risk is with handoffs that also modify docs/strategy/ — none of the other 7 handoffs do.

---

## §1 — DECISIONS ALREADY MADE

### D1 — Annotation format: top-of-file callout + inline status markers

**Top-of-file callout** (after the existing Authority block):

```markdown
> **Implementation Status (as of 2026-05-02):** Layer N is **<state>**. See `docs/pipeline/<manifests>` for per-stage validation evidence. This doc has been annotated to reflect actual implementation choices — search for `[IMPL]` markers. Where actual implementation diverged from the spec, the rationale is in a `> **[IMPL] Deviation:**` callout.
```

**Inline status markers** — for each load-bearing design decision in the doc, prepend or append a callout:

- `> **[IMPL] ✓ As designed** — <commit ref or file path>` (built as the doc described)
- `> **[IMPL] ⚠ Deviation** — <what actually happened + why>` (built differently)
- `> **[IMPL] ✗ Deferred** — <reason>` (planned but not built; usually L4-L6 dependencies)
- `> **[IMPL] ↻ Superseded** — <link to newer decision>` (the design was replaced by a later one)

### D2 — Granularity: load-bearing decisions only

Don't annotate every paragraph — only design CHOICES. A "load-bearing decision" is one where:

- A future agent needs to know whether to follow the doc or the code
- The choice constrains other layers
- A reasonable reader could implement it differently and produce wrong behavior

Examples of load-bearing decisions in L1-L3:

- L1: PTY-based agent execution? Yes/no. Skill metadata schema. Job runner state machine. Cron scheduling.
- L2: Tauri Rust ↔ React TS IPC method (typed JSON over Unix socket). Daemon connection pattern (with_daemon_connection). Specific command names + their CRUD/RPC semantics.
- L3: 14-stage pipeline definition. Skill vs deterministic per stage. Operator decision schema (`pending_operator_intents`). Override schema (`operator_input_overrides`). Manifest skill vs daemon-fallback.

Examples of NOT load-bearing (skip annotation):

- General prose ("This layer focuses on...")
- Aspirational goals ("We aim to...")
- Risk tables (pre-implementation thinking)

### D3 — Where to look for ground truth

| Topic | Where to verify |
|---|---|
| Skill execution model | `crates/daemon/src/main.rs` `run_*_live_loop` functions; `crates/daemon/src/agent_session.rs`; `skills/*/SKILL.md` |
| IPC commands | `apps/desktop/src-tauri/src/main.rs` invoke_handler block; `apps/desktop/src-tauri/src/daemon_bridge.rs` |
| Pipeline stages | `crates/daemon/src/main.rs` line ~1246 (`--ingest` dispatcher) + each stage's `run_*_live_loop` |
| Database schema | `migrations/000{1..9}.sql`; `sqlite3 state/state.db ".schema"` |
| Operator decision flow | `crates/daemon/src/main.rs` lines 18877+ (capture_operator_decision etc.) + Stage 14 Phase 1+2 manifests |
| Validation manifests | `docs/pipeline/*validation-manifest.md` (commit 91a1484, be38085, 4ba771d, f769b23) |

### D4 — L4-L6: keep as "planned"

L4 (validation pipeline / sync), L5 (commercial intelligence), L6 (request pipeline automation) docs remain unchanged in their core spec. Add only the top-of-file callout: *"Implementation Status (as of 2026-05-02): Layer N is **PLANNED — not yet built**. No production code exists for this layer."* Do NOT add `[IMPL]` markers throughout (nothing to annotate against).

For Stage 15 specifically, mention in L6: *"Stage 15 (bid_workflow) is L6-blocked — see `prompts/stage-15-bid-workflow-handoff-DEFERRED.md` for the planned approach once L6 is built."*

### D5 — `layer-strategy.md` updates

The cross-layer strategy doc currently treats all 7 layers as planned. Add at the top:

```markdown
> **Build status (2026-05-02):**
> - Layer 0 (Gold Set + Evaluation): partially built (gold-set fixtures exist; evaluation harness scaffolding in `crates/daemon/tests/`).
> - Layer 1 (Thin Local Runner): **BUILT**. Daemon binary, PTY-based skill execution, cron scheduler, agent error manifest.
> - Layer 2 (Single-Operator Desktop): **BUILT**. Tauri desktop app, IPC bridge, ContractList/ContractDetail/ManifestInbox/OperatorDecisionPanel pages.
> - Layer 3 (Operational Hardening): **BUILT**. Stages 1-14 of the pipeline, multi-source ingestion, Stage 11 hardening, Stage 14 Phase 1+2.
> - Layer 4 (Shared Visibility): NOT BUILT.
> - Layer 5 (Expanded Commercial Intelligence): NOT BUILT.
> - Layer 6 (Request Pipeline Automation): NOT BUILT. Stage 15 (bid_workflow) blocked on this layer.
```

### D6 — `cross-reference-index.md` updates

Audit the existing cross-reference index. For each entry referencing L1-L3 designs, add an `Impl:` column or callout pointing at the implementing file/commit. For L4-L6 entries, leave as is (no impl exists).

### D7 — `README.md` of strategy folder

Update README.md to mention the implementation status convention (D1) and direct readers to the per-layer top-of-file callouts.

---

## §2 — EXECUTION ORDER

1. **Pre-flight** (§0).
2. **L1 (`layer-1-thin-local-runner.md`)** — read end-to-end, identify load-bearing decisions per D2, annotate per D1, cross-reference per D3. Note: this is the largest layer; expect 30-50 annotations.
3. **L2 (`layer-2-single-operator-desktop.md`)** — annotate. Note Phase 2's IPC handler module + ManifestInbox/OperatorDecisionPanel as deviations from any pre-Phase-2 design choices.
4. **L3 (`layer-3-operational-hardening.md`)** — annotate. The 14-stage pipeline definition, OrderQuantityBasis::BasisUndetermined, Stage 14 operator decision flow, all the Stage 11 hardening — significant deviation density.
5. **L4 (`layer-4-shared-visibility.md`)** — top-of-file callout only (D4). No `[IMPL]` markers.
6. **L5 (`layer-5-expanded-commercial-intelligence.md`)** — top-of-file callout only.
7. **L6 (`layer-6-request-pipeline-automation.md`)** — top-of-file callout + Stage 15 mention (D4).
8. **L0 (`layer-0-gold-set-evaluation.md`)** — top-of-file callout indicating partial implementation (gold-set fixtures + harness scaffolding exist; full evaluation pipeline doesn't).
9. **`layer-strategy.md`** — add the Build status callout (D5).
10. **`cross-reference-index.md`** — annotate per D6.
11. **`README.md`** — update per D7.
12. Read the whole `docs/strategy/` directory end-to-end. Does it accurately reflect 2026-05-02 reality?
13. Quality gates (minimal — docs only).
14. Validation manifest.
15. Commit.

---

## §3 — REQUIRED TESTS

This handoff modifies docs only. **No new tests.** Quality gate is "the doc accurately reflects code reality" — best validated by review, not automation.

If a doc has a code reference that's outdated (e.g., points to a function that no longer exists), update it. Use `grep` to verify each `crates/daemon/src/main.rs:LINE` reference resolves.

---

## §4 — ACCEPTANCE CRITERIA — R.1 to R.10

| # | Criterion |
|---|---|
| R.1 | L1, L2, L3 docs each have the top-of-file Implementation Status callout per D1 |
| R.2 | L1, L2, L3 docs have ≥10 inline `[IMPL]` markers each (proportional to the doc's design density) |
| R.3 | Each `[IMPL]` marker uses one of the 4 D1 statuses (✓ As designed / ⚠ Deviation / ✗ Deferred / ↻ Superseded) |
| R.4 | Each ⚠ Deviation marker has a 1-paragraph "actual impl" callout linking to a code file or commit |
| R.5 | L0, L4, L5, L6 docs each have a top-of-file "Implementation Status" callout (no inline markers) |
| R.6 | L6 callout mentions Stage 15 + the deferred handoff path |
| R.7 | `layer-strategy.md` has the Build status block per D5 at the top |
| R.8 | `cross-reference-index.md` reflects implementation status per D6 |
| R.9 | `README.md` mentions the implementation status convention |
| R.10 | Quality gates: no code touched, but `cargo check --all-targets` and `cargo test -p daemon --bin daemon` still pass (sanity); markdown renders cleanly (preview in IDE or via `mdcat` / `pandoc`) |

---

## §5 — VALIDATION QUERIES

```bash
# Spot-check that each annotation references something real
# E.g., for an [IMPL] marker pointing at "main.rs:18877":
sed -n '18875,18880p' crates/daemon/src/main.rs
# Should show the cited function definition

# Verify each layer doc has the top-of-file callout
for f in docs/strategy/layers/layer-{0..6}-*.md; do
  echo -n "$(basename $f): "
  grep -c "Implementation Status" "$f" || echo "MISSING"
done

# Count inline markers per layer
for f in docs/strategy/layers/layer-{1..3}-*.md; do
  echo -n "$(basename $f) — [IMPL] markers: "
  grep -c "\[IMPL\]" "$f"
done

# Render check (if mdcat installed)
mdcat docs/strategy/layers/layer-1-thin-local-runner.md | head -50
```

---

## §6 — FAILURE-MODE PLAYBOOK

| Symptom | Cause | Action |
|---|---|---|
| Layer doc references a function that doesn't exist | Code refactored since doc was written | Update the reference to current code path. If the design has been entirely superseded, mark `[IMPL] ↻ Superseded` and link to the replacement |
| Layer doc describes a design that wasn't implemented at all | Decision was reversed during implementation, or just ignored | Mark `[IMPL] ✗ Deferred` if it might still happen. Mark `[IMPL] ↻ Superseded` if a different choice replaced it. Either way, document the actual implementation |
| Hard to tell whether a design was "as designed" or "deviated" | The doc is too vague to compare | Mark `[IMPL] ⚠ Deviation` and document what was actually built. Future readers will appreciate the honesty |
| Cross-reference between docs is broken | One doc renamed or moved | Fix the link in place. Don't mass-rename docs in this handoff |
| Markdown linter complains | Style nit (trailing whitespace, list indentation) | Fix per project convention if obvious; otherwise ignore — this isn't a style refactor handoff |
| Tempted to rewrite a layer doc from scratch | Doc is so divergent from reality that annotation feels insufficient | Don't rewrite. The original doc has historical value. Annotate heavily; if the rewrite is genuinely needed, file as bead `STRATEGY-LN-REWRITE` for a future handoff |

---

## §7 — CONSTRAINTS

1. **Docs only.** No code changes.
2. **No layer doc rewrites.** Annotation pattern per D1, in place.
3. **L4-L6 stay as planned designs.** Top-of-file callout only; no `[IMPL]` markers.
4. **Date-stamp the annotations** ("as of 2026-05-02") — reality moves; future annotations should layer over these, not replace them.
5. **Cross-references must resolve.** A `[IMPL] ✓ As designed — main.rs:18877` is only useful if line 18877 actually shows the cited code at the time of annotation.
6. **Commit message specifies the cutoff date** (2026-05-02) so future readers can locate the snapshot.
7. **No silent test exclusion** — quality gates per R.10 mandatory (minimal because no code touched).

---

## §8 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/strategy-docs-l1-l3-implementation-status-validation-manifest.md`. Include:

- Cutoff date (2026-05-02) + main HEAD
- Per-doc annotation count: L1 has X markers, L2 has Y, L3 has Z, L0/L4/L5/L6 have callouts only
- Major deviations identified (1 sentence each, link to the inline marker for detail)
- Cross-reference verification: each `main.rs:LINE` cited at least once verified to point at the right code
- R.1–R.10 status table

### Quality gates

```
cargo fmt --check                        # sanity
cargo check --all-targets                # sanity (no code touched)
cargo test -p daemon --bin daemon        # 169 / 169 (sanity)
# Visual review of rendered markdown encouraged but not gated
```

---

## §9 — REMINDERS

- ULTRATHINK and be meticulous.
- The audience is a future implementation agent who will read these docs to understand "what to build". The annotations exist so that agent doesn't rebuild what's already built.
- Don't annotate every paragraph — only load-bearing design decisions (D2).
- L4-L6 are still "planned"; resist annotating them with imagined implementation status.
- Where reality diverged from the spec, be HONEST about it. The doc isn't "wrong" — it described the intent at the time. Annotation says "and here's what we actually built."
- Cross-reference verification (D5 → R.10) is the single most important quality bar. A wrong line number is worse than no line number.
- Date-stamp everything (2026-05-02). Future passes layer over, not replace.
