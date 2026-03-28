# L2 Desktop MVP — Test Bootstrap

## 1. Scope

This document defines how Layer 2 tests are bootstrapped and run. It is the executable companion to:

- `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md`
- `docs/strategy/layers/layer-2-single-operator-desktop.md`
- `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md`

The repository currently has the Layer 1 Rust workspace only. It does **not** yet have the Bun workspace, Tauri shell, shared packages, or Playwright harness that Layer 2 requires. This bootstrap therefore covers both:

- environment validation for the existing repo
- scaffold and configuration work required to make the Layer 2 test inventory runnable

## 2. Success Criteria Coverage Summary

| Surface | Covered DoD Items | Primary Tests / Commands |
|---------|-------------------|--------------------------|
| Existing daemon runtime prerequisites | 1, 3, 21 | inherited L1 daemon suites + `cargo test --all-targets` |
| Contracts Finder source skill | 2 | `agent_contracts_finder_ingestion.rs`, skill canary, run-manifest validation |
| Desktop shell bootstrap and runtime modes | 5, 13, 16, 18 | `tauri-integration.spec.ts`, `daemon-status.spec.ts`, `daemon_bridge.rs` |
| Contract list + detail + coverage surfaces | 6, 7, 10 | `contracts.spec.ts`, `coverage-rollup.test.ts`, `render-opportunity.test.ts`, shared UI fixtures |
| Decision capture + staleness | 8, 11, 15 | `decisions.spec.ts`, `commit-fence.spec.ts`, `query_service.rs` |
| Sessions, stop/input/retry, reconnect, gap recovery | 9, 14, 20 | `sessions.spec.ts`, `session-registry.test.ts`, `session-buffer.test.ts`, `agent_health_detection.rs` |
| Error queue and manifest handling | 17 | `errors.spec.ts`, `agent_error_manifest.rs` |
| Security boundary | 19 | `security.spec.ts`, shared UI fixture tests |
| Manual operator validation | 12 | `docs/testing/l2-operator-weekly-validation.md` |

## 3. Environment Prerequisites

| Requirement | Why it is needed | Validation |
|------------|------------------|------------|
| Rust nightly with 2024 edition support | daemon crate and future Tauri Rust bridge target nightly Rust 2024 | `rustup toolchain list | rg nightly` |
| Cargo | Rust build, lint, format, integration tests | `cargo --version` |
| Bun | workspace install, Vite dev server, Vitest, Tauri frontend, Playwright runner | `bun --version` |
| Playwright CLI + Chromium browser | Tier 1 browser E2E | `bunx playwright --version` and `bunx playwright install --dry-run chromium` |
| `rg` | fast fixture discovery and smoke verification | `rg --version` |
| writable local filesystem | temp SQLite DBs, Tauri dev build artifacts, UDS sockets, Playwright output | create and remove a temp file |
| Unix-like host | daemon IPC and current PTY/session harness are Unix-first | `uname -s` in `{Darwin,Linux}` |
| macOS Xcode CLI tools when on macOS | WebKit / native toolchain for Tauri desktop builds | `xcode-select -p` |
| optional network access | live Contracts Finder canary, inherited FTS smoke, external browser downloads | HTTPS probe to source endpoints |

### 3.1 Environment Variables

| Variable | Required | Purpose |
|----------|----------|---------|
| `RUST_LOG` | optional | daemon / Tauri Rust tracing during integration and E2E runs |
| `DAEMON_CLAUDE_PROGRAM` | optional unless live Claude adapter lanes run | override the Claude CLI binary used by daemon-managed skill sessions |
| `DAEMON_CODEX_PROGRAM` | optional unless live Codex adapter lanes run | override the Codex CLI binary used by daemon-managed skill sessions |

If the adapter override variables are absent, live agent E2E must either use the default CLI names available on `PATH` or be skipped explicitly.

## 4. Preflight Validation Script

Run this before scaffolding or executing any Layer 2 tests:

```bash
#!/usr/bin/env bash
set -euo pipefail

pass() { printf '[PASS] %s\n' "$1"; }
warn() { printf '[WARN] %s\n' "$1"; }
fail() { printf '[FAIL] %s\n' "$1"; exit 1; }

rustup toolchain list | rg -q 'nightly' || fail 'Rust nightly toolchain missing'
cargo --version >/dev/null 2>&1 || fail 'cargo missing'
bun --version >/dev/null 2>&1 || fail 'bun missing'
bunx playwright --version >/dev/null 2>&1 || fail 'Playwright CLI missing'
rg --version >/dev/null 2>&1 || fail 'ripgrep missing'

case "$(uname -s)" in
  Darwin|Linux) pass 'Unix-like host detected' ;;
  *) fail 'Layer 2 desktop bootstrap currently assumes a Unix-like host for daemon IPC and PTY semantics' ;;
esac

if [ "$(uname -s)" = "Darwin" ]; then
  xcode-select -p >/dev/null 2>&1 || fail 'Xcode CLI tools missing'
  pass 'macOS build toolchain present'
fi

test -f docs/strategy/layers/layer-2-single-operator-desktop.md || fail 'L2 spec missing'
test -f docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md || fail 'L2 TDD missing'
test -f docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md || fail 'L1 bootstrap missing'
test -f crates/daemon/Cargo.toml || fail 'daemon crate missing'

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
touch "$tmp_root/probe" || fail 'temporary filesystem is not writable'
pass 'temporary filesystem writable'

if curl -IsSf https://www.find-tender.service.gov.uk >/dev/null 2>&1; then
  pass 'FTS endpoint reachable'
else
  warn 'FTS endpoint unreachable: inherited live smoke must be skipped'
fi

if curl -IsSf https://www.contractsfinder.service.gov.uk >/dev/null 2>&1; then
  pass 'Contracts Finder endpoint reachable'
else
  warn 'Contracts Finder endpoint unreachable: CF canary must run against captured evidence only'
fi
```

## 5. Scaffold Plan

### 5.1 Root Bun Workspace

Create the Bun workspace at repo root without disturbing the existing Cargo workspace:

```text
package.json
bunfig.toml
tsconfig.json
apps/
  desktop/
packages/
  ui/
  manifests/
  intelligence/
```

**Root `package.json` minimum:**

```json
{
  "name": "procurement-intelligence-platform",
  "private": true,
  "workspaces": [
    "apps/*",
    "packages/*"
  ]
}
```

**Root `tsconfig.json` minimum:**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "jsx": "react-jsx",
    "strict": true,
    "noEmit": true
  }
}
```

**Why keep the root minimal:** Layer 2 needs the workspace boundary first. Do not hide missing per-package scripts behind complex root wrappers in the first bootstrap pass.

**Version pinning rule:** every new `package.json` and `Cargo.toml` introduced by Layer 2 must use exact versions, not `^` or `~` ranges. The bootstrap is not complete until the generated Bun lockfile and `Cargo.lock` updates are committed.

### 5.1.1 Cargo Workspace Integration

The current root [Cargo.toml](/Users/alinaaitova/Desktop/Testproject1/Cargo.toml) contains only:

```toml
[workspace]
members = ["crates/daemon"]
```

Once `apps/desktop/src-tauri/` exists, the workspace must be extended explicitly:

```toml
[workspace]
members = [
  "crates/daemon",
  "apps/desktop/src-tauri",
]
resolver = "2"
```

The new `apps/desktop/src-tauri/Cargo.toml` must inherit:
- `edition = "2024"`
- workspace clippy lints
- `#![forbid(unsafe_code)]` in all `src-tauri/src/*.rs` entrypoints and tests

Without this step, the root `cargo check --all-targets` / `cargo clippy --all-targets` gates in the TDD are not actually enforcing the desktop Rust surface.

### 5.2 Desktop App Scaffold

Scaffold `apps/desktop` in this order:

1. Create the React/Vite app in `apps/desktop`.
2. Install the Tauri v2 frontend/CLI dependencies into that workspace.
3. Initialize `src-tauri/` so the native shell points at the Vite dev server for development and the Vite build output for production.
4. Add Playwright and Vitest before writing any app code, because the dev bridge and browser-first flow are part of the contract, not a retrofit.

**Minimum desktop scripts:**

```json
{
  "name": "@procintel/desktop",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite --host 127.0.0.1 --port 5173 --strictPort",
    "build": "tsc -b && vite build",
    "lint": "eslint .",
    "test": "vitest run",
    "e2e": "playwright test",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build"
  }
}
```

**Tauri init answers that must be locked in:**
- dev URL: `http://127.0.0.1:5173`
- frontend dist: `../dist`
- app name: desktop-specific, not daemon
- shell model: separate daemon, not embedded backend business logic
- Rust crate policy: `#![forbid(unsafe_code)]` on `src-tauri/src/main.rs`, `query_service.rs`, `daemon_bridge.rs`, and all Rust tests

### 5.2.1 Current Daemon CLI Baseline

The current repository does **not** yet expose a persistent long-running Layer 2 daemon loop. As of the checked-in `crates/daemon/src/main.rs`, the binary supports bounded Layer 1 entrypoints only:

```bash
# Bootstrap and validate runtime artifacts, then exit
cargo run -p daemon -- --dry-run

# Accept exactly one hello/welcome handshake, then exit
cargo run -p daemon -- --serve-once

# Run one bounded FTS ingest session, then exit
cargo run -p daemon -- --ingest fts

# Run one bound skill session, then exit
cargo run -p daemon -- --skill-path skills/procurement-ingestion/fts
```

For deterministic local harness runs, always pass explicit temp paths:

```bash
cargo run -p daemon -- \
  --state-root .tmp/state \
  --runtime-dir .tmp/runtime \
  --artifact-root .tmp/artifacts \
  --serve-once
```

**Bootstrap consequence:** full Playwright and Tauri integration suites require Layer 2 implementation work to add a persistent daemon-serving mode behind the same runtime roots. Until that exists, a fresh agent can validate the current daemon baseline with the commands above, but cannot honestly claim the full browser E2E lane is runnable end-to-end.

**Resulting required files:**

```text
apps/desktop/
  package.json
  vite.config.ts
  vitest.config.ts
  playwright.config.ts
  src/
    App.tsx
    lib/transport.ts
    pages/
    stores/
    hooks/
  src-tauri/
    Cargo.toml
    build.rs
    tauri.conf.json
    capabilities/default.json
    src/
      main.rs
      query_service.rs
      daemon_bridge.rs
```

### 5.3 Vite + Dev Bridge

Layer 2 uses the browser dev bridge as the **primary** E2E surface. The dev bridge must be configured in Vite from day one.

**`apps/desktop/vite.config.ts` minimum:**

```ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { tauriDevBridge } from './vite-plugin-tauri-dev-bridge'

export default defineConfig(({ mode }) => ({
  plugins: [
    react(),
    mode === 'development' ? tauriDevBridge() : undefined,
  ].filter(Boolean),
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
  },
}))
```

**`apps/desktop/vite-plugin-tauri-dev-bridge.ts` must provide:**
- `POST /__tauri_invoke__` for query and mutation proxying
- `ws://127.0.0.1:5173/__ws__` for `SessionFrame` and `BridgeEvent` relay
- loopback-only binding
- no production registration

**Exact bootstrap rule:** the dev bridge must call the same QueryService and DaemonBridge code paths used by the Tauri shell. It must not reimplement business logic in a Vite-only fork.

### 5.4 Runtime Transport Shim

`apps/desktop/src/lib/transport.ts` is the single frontend transport entry point:

```ts
export const invoke = window.__TAURI_INTERNALS__?.invoke
  ?? ((cmd: string, args?: Record<string, unknown>) =>
    fetch('/__tauri_invoke__', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ cmd, args }),
    }).then((r) => r.json()))
```

The same file must also export `subscribe(topic, fromSeq, onEvent)`:
- native path: Tauri `Channel<T>`
- browser path: WebSocket to `/__ws__`

No hook or store may import Tauri APIs directly.

### 5.5 Playwright Configuration

**`apps/desktop/playwright.config.ts` minimum:**

```ts
import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  timeout: 60_000,
  fullyParallel: false,
  use: {
    baseURL: 'http://127.0.0.1:5173',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  webServer: {
    command: 'bun run dev',
    url: 'http://127.0.0.1:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
})
```

**Playwright prerequisites:**
- install the browser once: `bunx playwright install chromium`
- keep browser tests deterministic: bind to `127.0.0.1`, not wildcard hostnames
- do not hide daemon startup inside the Playwright config; the daemon is a separate prerequisite controlled by the test harness
- use an explicit daemon start command in the harness; for the current codebase that means the bounded Layer 1 commands above, and for completed Layer 2 work it means the persistent daemon-serving mode added during implementation
- use role/text/label-driven selectors in specs; avoid CSS-class selectors and raw `waitForTimeout` polling outside deliberate backpressure tests

`bun run dev` starts the Vite frontend plus the dev bridge only. Full daemon access during Playwright runs exists only when the harness has already started the daemon command in a separate process.

### 5.6 Vitest + Shared Package Layout

Shared packages must exist before the first desktop page test, because the spec makes `packages/ui`, `packages/manifests`, and `packages/intelligence` part of the Layer 2 contract.

```text
packages/ui/
  package.json
  index.ts
  fixtures/
packages/manifests/
  package.json
  render-opportunity.ts
  opportunity.md.ts
packages/intelligence/
  package.json
  src/
    qualification/
    margins/
```

**Required package test intent:**
- `packages/ui`: fixture-driven render tests only
- `packages/manifests`: deterministic renderer tests only
- `packages/intelligence`: coverage and classification logic only

Do not bootstrap data-access clients or daemon IPC code into any shared package.

### 5.7 Contracts Finder Skill Scaffold

Layer 2 adds a new skill package:

```text
skills/procurement-ingestion/contracts-finder/
  SKILL.md
  checkpoint.schema.json
  result.schema.json
  canary.inputs.jsonl
  golden.assertions.yaml
```

The bootstrap must mirror the FTS structure exactly:
- same top-level sections in `SKILL.md`
- same canary + golden assertion pairing
- same run-manifest frontmatter contract
- explicit cursor overlap/backtrack documentation

## 6. Test Harness Layout

| Path | Purpose |
|------|---------|
| `apps/desktop/e2e/fixtures/` | browser fixtures, route payloads, seeded process/session/error states |
| `apps/desktop/e2e/support/` | Playwright helpers, seeded daemon controls, polling helpers for commit-fence assertions |
| `apps/desktop/src-tauri/tests/support/` | Rust-side harnesses for QueryService, bridge, temp DB, descriptor paths, and artifact gaps |
| `packages/ui/fixtures/` | stable view-model fixtures for shared UI tests |
| `packages/manifests/fixtures/` | deterministic manifest inputs and expected heading order |
| `packages/intelligence/fixtures/` | classification and coverage math inputs |
| `crates/daemon/tests/support/` | existing L1 support extended for CF, enrichment, and error-manifest E2E |
| `docs/testing/` | manual operator diary / weekly validation checklist |

`apps/desktop/e2e/support/` must include, at minimum:
- a daemon launcher that starts the real daemon command for the current lane
- a slow-consumer helper that forces `gap` emission without `waitForTimeout`-driven flakiness
- a replay verifier that asserts exact `seq` continuity and no duplicate frames
- a selector policy helper that centralizes `getByRole` / visible-text / route-param selectors

## 7. Post-Scaffold Validation Script

Run this after the desktop/package files exist:

```bash
#!/usr/bin/env bash
set -euo pipefail

pass() { printf '[PASS] %s\n' "$1"; }
fail() { printf '[FAIL] %s\n' "$1"; exit 1; }

required_files=(
  package.json
  tsconfig.json
  apps/desktop/package.json
  apps/desktop/vite.config.ts
  apps/desktop/playwright.config.ts
  apps/desktop/src/lib/transport.ts
  apps/desktop/vite-plugin-tauri-dev-bridge.ts
  apps/desktop/src-tauri/tauri.conf.json
  apps/desktop/src-tauri/src/main.rs
  apps/desktop/src-tauri/src/query_service.rs
  apps/desktop/src-tauri/src/daemon_bridge.rs
  packages/ui/index.ts
  packages/manifests/render-opportunity.ts
  packages/intelligence/src/margins/coverage-rollup.ts
  skills/procurement-ingestion/contracts-finder/SKILL.md
  skills/procurement-ingestion/contracts-finder/canary.inputs.jsonl
  skills/procurement-ingestion/contracts-finder/golden.assertions.yaml
)

for path in "${required_files[@]}"; do
  test -f "$path" || fail "missing $path"
done

pass 'required Layer 2 scaffold files exist'

bun install
pass 'bun install completed'

(cd apps/desktop && bun run build)
pass 'desktop build passes'

(cd apps/desktop && bun run test)
pass 'desktop Vitest suite passes'

(cd apps/desktop && bunx playwright test --list)
pass 'desktop Playwright suite resolves'
```

## 8. Runner Commands

Run from repo root unless otherwise noted.

### 8.1 Rust Gates

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test --all-targets
```

### 8.2 Bun + Desktop Gates

```bash
bun install
cd apps/desktop && bun run lint
cd apps/desktop && bun run build
cd apps/desktop && bun run test
cd apps/desktop && bunx playwright test
```

### 8.3 Focused Execution Lanes

```bash
# Current daemon baseline smoke
cargo run -p daemon -- --dry-run
cargo run -p daemon -- --serve-once --state-root .tmp/state --runtime-dir .tmp/runtime --artifact-root .tmp/artifacts

# QueryService / bridge contracts
cargo test -p daemon --test query_service
cargo test -p daemon --test daemon_bridge

# Browser-first UI iteration
# Requires the Layer 2 persistent daemon-serving mode; the current Layer 1 CLI is bounded and exits after one action.
cd apps/desktop && bun run dev
cd apps/desktop && bunx playwright test e2e/contracts.spec.ts

# Native-shell verification
# Also requires the Layer 2 persistent daemon-serving mode plus the Tauri shell scaffold.
cd apps/desktop && bun run tauri:dev
cd apps/desktop && bunx playwright test e2e/tauri-integration.spec.ts

# Shared packages
cd packages/ui && bun test
cd packages/manifests && bun test
cd packages/intelligence && bun test

# Agent E2E
cargo test -p daemon --test agent_contracts_finder_ingestion
cargo test -p daemon --test agent_supplier_enrichment
cargo test -p daemon --test agent_health_detection
cargo test -p daemon --test agent_error_manifest
```

## 9. False Positive Prevention

| Risk | Prevention Rule |
|------|------------------|
| Commit-fence tests pass without proving stale pre-fence state | Every mutation-flow test must assert old UI state at `snapshotSeq < visibleAfterSeq` before asserting the visible state |
| Gap-recovery tests pass while silently duplicating frames | Replay tests must assert exact `seq` continuity and zero duplicate `seq` values after merge |
| Staleness tests pass without real basis change | Decision tests must record hash A, cause a rerun, and assert hash B differs before expecting `isStale = true` |
| Security tests pass with benign content only | All manifest/error/session sanitization tests must use hostile fixtures from `SecurityFixtureFactory` |
| Browser E2E passes with weak selectors or timeout sleeps | Specs must use role/text selectors and observable state transitions; `waitForTimeout` is forbidden outside explicit backpressure harness cases |
| Tauri tests accidentally exercise the browser dev bridge | Native-shell tests must assert real Tauri window launch and daemon PID/descriptor behavior |
| Shared-package tests leak backend dependencies | `packages/ui`, `packages/manifests`, and `packages/intelligence` tests must be fixture-only and import-graph clean |

## 10. Artifact Checklist

Before Layer 2 implementation is considered bootstrapped, these artifacts must exist:

- `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md`
- `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md`
- `package.json`
- `tsconfig.json`
- `apps/desktop/package.json`
- `apps/desktop/vite.config.ts`
- `apps/desktop/playwright.config.ts`
- `apps/desktop/vite-plugin-tauri-dev-bridge.ts`
- `apps/desktop/src/lib/transport.ts`
- `apps/desktop/src-tauri/tauri.conf.json`
- `apps/desktop/src-tauri/src/main.rs`
- `apps/desktop/src-tauri/src/query_service.rs`
- `apps/desktop/src-tauri/src/daemon_bridge.rs`
- `packages/ui/index.ts`
- `packages/manifests/render-opportunity.ts`
- `packages/manifests/opportunity.md.ts`
- `packages/intelligence/src/margins/coverage-rollup.ts`
- `skills/procurement-ingestion/contracts-finder/SKILL.md`
- `skills/procurement-ingestion/contracts-finder/checkpoint.schema.json`
- `skills/procurement-ingestion/contracts-finder/result.schema.json`
- `skills/procurement-ingestion/contracts-finder/canary.inputs.jsonl`
- `skills/procurement-ingestion/contracts-finder/golden.assertions.yaml`
- `docs/testing/l2-operator-weekly-validation.md`

## 11. Bootstrap Exit Criteria

Bootstrap is complete when all of the following are true:

- Preflight validation passes.
- The Bun workspace resolves and the desktop scaffold exists on disk.
- The dev bridge is wired into Vite and the transport shim is the only frontend transport entry point.
- Playwright can reach `http://127.0.0.1:5173` and list the browser specs.
- Native Tauri shell can launch against the same frontend.
- Rust + Bun + Playwright commands listed above all execute successfully.
- The Contracts Finder skill scaffold exists with the FTS-compatible canary/golden structure.

**Timing expectation:** on a warm machine with Bun dependencies and the Chromium browser already installed, rerunning the bootstrap validation should fit comfortably inside ten minutes. A cold machine may exceed ten minutes because first-time browser downloads and frontend dependency installation are network-bound.
