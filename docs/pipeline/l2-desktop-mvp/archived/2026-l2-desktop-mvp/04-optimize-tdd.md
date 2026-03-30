# L2 Desktop MVP — Optimize TDD (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the Layer 2 TDD and Bootstrap documents. The TDD defines HOW to verify every `L2-FR-*` requirement, and the Bootstrap defines HOW to set up the development environment. Both must be rigorous enough that an implementation agent can write tests directly from the TDD and set up a working dev environment from the Bootstrap alone.

**You are reading, analyzing, and improving existing documents — not writing code.**

---

## What To Read (Do This First)

### The Documents to Audit (TARGET)
```
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md          — TDD (created in previous step)
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md     — Bootstrap (created in previous step)
```
Read both fully. These are the primary audit targets.

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-2-single-operator-desktop.md                — Spec (SOT #1) — Every L2-FR-* must have tests
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/ipc/handshake.rs         — Verify DaemonBridge tests reference real protocol
crates/daemon/src/db/sqlite.rs             — Verify commit fence tests use real commit_clock structure
crates/daemon/src/agents/adapter.rs        — Verify streaming tests cover all ControlEvent/OutputFrame variants
crates/daemon/src/pty/session.rs           — Verify session lifecycle tests cover all SessionPhase transitions
crates/daemon/tests/agent_pty_session.rs   — L1 test quality bar — are L2 Tier 3 tests at least this rigorous?
migrations/0001_sqlite_state_initial.sql   — Verify all table/column assertions match actual DDL
Cargo.toml                                 — Verify Rust workspace config matches bootstrap
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md        — L1 TDD (150 test IDs)
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md  — L1 Bootstrap
```

---

## Think Independently

### 1. Coverage Matrix Completeness
Walk the coverage matrix row by row. For EVERY `L2-FR-*` requirement:
- Is there at least one test ID?
- Is the test specific enough to catch a real regression?
- Would a "return true" implementation pass the test? If yes, the assertion is too weak.

### 2. Commit Fence Test Rigor
The commit fence (§1.4) is the most subtle contract. Verify:
- Tests assert on `snapshotSeq` values, not just "data appeared"
- Tests cover the ghost-state scenario: mutation committed but query returns stale data
- Tests cover concurrent mutations: two mutations → two `visibleAfterSeq` values → queries reflect correct ordering
- Tests exercise the `data_version` vs `commit_seq` distinction (QueryService uses `data_version` for cache invalidation, `commit_seq` for visibility fence)

### 3. Streaming Test Infrastructure
Channel<T> tests require daemon-side infrastructure. Verify:
- The bootstrap defines how to set up a test daemon that produces controlled streams
- Gap recovery tests have a mechanism to simulate slow consumers
- Live-to-historical handoff tests have a mechanism to verify `archiveSeq` consistency
- The WebSocket relay (dev bridge) is tested independently of the Tauri Channel<T>

### 4. Playwright Test Realism
Tier 1 tests run in a real browser. Verify:
- Tests navigate to pages (not just render components in isolation)
- Tests use Playwright selectors that a human could understand (text content, roles, not CSS classes)
- Tests wait for data to appear (commit fence aware), not just `waitForTimeout`
- Tests verify accessibility attributes (not just visual rendering)
- The test setup starts a real daemon + dev server (not mocked)

### 5. Security Test Adversarial Quality
Verify the security test fixtures include:
- Real XSS payloads: `<img src=x onerror=alert(1)>`, `<svg onload=alert(1)>`
- Link injection: `[click](javascript:void(0))`, `[click](data:text/html,<script>alert(1)</script>)`
- ANSI escape attacks: OSC 8 hyperlinks, clipboard-related sequences (OSC 52)
- Large payloads: 10MB manifest, 100K-line session output
- Unicode edge cases: RTL override characters, zero-width joiners in agent output

### 6. Bootstrap Executability
Could a fresh agent run the bootstrap and have a working environment?
- Are all dependency versions pinned?
- Are all environment variables documented?
- Does the bootstrap handle the "no daemon running" case?
- Is the Tauri v2 project scaffold complete (Cargo.toml for `src-tauri`, tauri.conf.json, capabilities)?
- Is the Playwright configuration complete (browsers to install, test directories)?

### 7. Tier Assignment Correctness
Verify each test is in the right tier:
- **Tier 1 (Playwright):** UI rendering, navigation, data display, user interaction — anything testable in a browser
- **Tier 2 (Tauri):** Desktop-only: daemon auto-launch, window lifecycle, browse-only fallback, instanceId detection
- **Tier 3 (Agent E2E):** Skill runtime: PTY spawn, notice extraction, result contract validation
- Tests should NOT be in a higher tier than necessary — if it can be tested in Playwright, don't require a Tauri build

### 8. Test ID Naming Convention
L1 used a structured naming convention. Verify L2 does too:
- Format: `L2-T-{SUBSYSTEM}-{NNN}` or similar
- No collisions with L1 test IDs (`L1-T-*`)
- Grouped by subsystem for easy filtering

---

## Drift Checklist

- [ ] Every `L2-FR-*` requirement ID has at least one test ID in the coverage matrix
- [ ] No test was silently weakened (assertions that always pass)
- [ ] Bootstrap produces a working dev environment for all three test runners
- [ ] Tier assignments are correct and minimal (no over-tiering)
- [ ] CI gate commands match: `cargo check`, `cargo clippy`, `cargo fmt`, `cargo test`, `bun run build`, `bun run lint`, `bun run test`, `bunx playwright test`
- [ ] The TDD doesn't assume features from L3+ (no validation, no cron, no sync)
- [ ] Test fixtures for security tests are adversarial, not hello-world
- [ ] The commit fence test section is the most thoroughly specified section

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message.

Re-read the fixes you just made. Did they introduce new issues?

1. If you strengthened assertions, are they still realistic? (e.g., don't assert exact timestamps — assert ordering)
2. If you added test infrastructure requirements to the bootstrap, is the bootstrap still executable in < 10 minutes?
3. Count test IDs by tier. What's the distribution? Expect: ~60% Tier 1 (Playwright), ~15% Tier 2 (Tauri), ~25% Tier 3 (Agent E2E). If Tier 1 is below 40%, UI coverage is likely weak.
4. Check the coverage matrix one more time — no `L2-FR-*` requirement should be orphaned.
5. Verify the bootstrap section for dev bridge setup — can a test start `bun run dev` and have Playwright navigate to `localhost:5173` with full daemon access?
