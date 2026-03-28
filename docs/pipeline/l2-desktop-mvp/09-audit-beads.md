# L2 Desktop MVP — Bead Audit (Spec Drift Detection)

## First Action

Invoke the `/bead-plan-audit` skill.

---

## Your Mission

You are performing a **dedicated, systematic audit** of all beads for L2 Desktop MVP. This is the LAST GATE before execution. Your job is to verify **zero spec drift** — every spec requirement is covered, every TDD test is assigned, no silent scope cuts.

---

## Source Documents

### Beads (TARGET — audit EVERY bead)
```bash
br list --json               # Full inventory
br show <id> --json          # Individual bead detail
```
Read EVERY bead. Not a sample — every single one.

### Verify Against
```
docs/strategy/layers/layer-2-single-operator-desktop.md                — Spec (SOT #1) — Every L2-FR-* must have a bead
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md          — TDD (SOT #2) — Every test ID must be assigned
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md      — Strategy (SOT #3) — Phase ordering must match
```

### Codebase
```
crates/daemon/src/ipc/handshake.rs         — IPC protocol beads reference
crates/daemon/src/agents/adapter.rs        — Type definitions beads reference
crates/daemon/src/db/sqlite.rs             — commit_clock structure beads reference
crates/daemon/src/jobs/runner.rs           — Job Runner beads reference
migrations/0001_sqlite_state_initial.sql   — Schema tables beads reference
skills/procurement-ingestion/fts/          — Skill structure beads reference
```

---

## Audit Methodology

Invoke `/bead-plan-audit` which handles the detailed methodology.

### Feature-Specific Checks

**L2-Specific Drift Risks:**

1. **DaemonBridge command coverage:** The spec lists 10 commands (§1.5). Verify all 10 appear in beads. Common drift: `restart_daemon` and `rerun_from_error` get silently dropped because they seem "optional."

2. **Channel<T> streaming completeness:** The spec defines 6 frame types (§1.6), 2 subscription levels, gap recovery, live→historical handoff, and chunked replay. Verify ALL are in beads, not just "implement streaming."

3. **Bootstrap state machine coverage:** 5 runtime modes (§1.7), 8-step daemon crash recovery, 9-step startup sequence. Verify the implementation bead covers all modes and transitions, not just "full control" and "browse-only."

4. **Shared UI component completeness:** 5+1 components in packages/ui. Verify each has a bead. Common drift: `ErrorSummaryList` gets forgotten because it's listed in §1.11 but detailed separately.

5. **Security sanitization is explicitly assigned:** Markdown sanitization, dangerous link stripping, and ANSI escape sanitization must appear in specific beads — not assumed to be "part of the component."

6. **Coverage rollup module exists:** `packages/intelligence/src/margins/coverage-rollup.ts` is a separate file from the desktop. Verify it has its own bead (often merged into a larger bead and forgotten).

7. **Decision capture metadata completeness:** §3.2 requires 8 specific fields on `pending_operator_intents`. Verify the decision bead lists all 8, not just "disposition."

8. **Accessibility requirements assigned:** §1.16 has 5 specific accessibility rules. Verify they appear in relevant component beads.

9. **Test bead pairing:** Every implementation bead should include its tests. Verify there are no orphan test files with no implementation bead.

10. **Dev bridge bead exists:** `vite-plugin-tauri-dev-bridge.ts` and `src/lib/transport.ts` are critical for testing but easy to forget in beads.

---

## Follow-Up Pass

> After the audit is complete and all fixes applied:

1. Re-read your own fixes. Did fixing one bead break another's dependencies?
2. Re-run the coverage matrix. Is it still 100%?
3. Verify bead count still matches strategy phase task count.
4. Are all 10 DaemonBridge commands accounted for?
5. Are all 6 SessionFrame variants accounted for?
6. Are all 5 runtime modes accounted for?
7. Are all 5+1 shared UI components accounted for?
8. Run `br list --status=open --json | jq 'length'` — confirm total matches expectations.
