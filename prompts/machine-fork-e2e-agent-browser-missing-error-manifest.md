# Missing agent-browser Blocks Machine Fork E2E Browser/CDP Validation - QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | integration_mismatch |
| **Severity** | high |
| **Reproducibility** | deterministic in this shell |
| **Affected Component** | machine setup / browser testing tooling / daemon persistent CDP skill preflight |
| **Working Analogue** | Cargo, Bun, Claude, and Codex binaries resolve from `PATH`. |
| **Scope** | `agent-browser` is not available on `PATH`, and the persistent CDP browser is not listening on `localhost:9222`. |
| **Environment** | macOS Darwin 25.3.0 arm64, Bun 1.3.13, Cargo 1.96.0-nightly, repo branch `main` |
| **Git State** | `4d1d1dd docs: persist machine fork e2e helpers`; dirty shared checkout contains unrelated `.DS_Store` and `bin/__pycache__/` |

---

## REPRODUCTION PROTOCOL

Follow these steps exactly.

### Prerequisites

- Repository path: `/Users/zacfarrell/Desktop/Testproject1`
- Current shell: `zsh`
- No source code edits required.

### Step 1: Confirm browser-tool command availability

```bash
cd /Users/zacfarrell/Desktop/Testproject1
which cargo
which bun
which claude || true
which codex || true
which agent-browser || true
bun --version
cargo --version
rustup show active-toolchain
```

Expected: `agent-browser` resolves to an executable path because the QA handoff requires it for browser/CDP validation.

Actual:

```text
/Users/zacfarrell/.cargo/bin/cargo
/Users/zacfarrell/.bun/bin/bun
/Users/zacfarrell/.local/bin/claude
/opt/homebrew/bin/codex
1.3.13
cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)
nightly-aarch64-apple-darwin (overridden by '/Users/zacfarrell/Desktop/Testproject1/rust-toolchain.toml')
```

There is no output for `which agent-browser`.

### Step 2: Run the required CDP browser command

```bash
agent-browser --cdp 9222 open "about:blank"
```

Expected: command opens `about:blank` in the persistent Chrome CDP session.

Actual:

```text
zsh:1: command not found: agent-browser
```

### Step 3: Check CDP listener

```bash
curl -sfS -m 3 http://localhost:9222/json/version
```

Expected: JSON containing Chrome/Protocol version metadata.

Actual:

```text
curl: (7) Failed to connect to localhost port 9222 after 0 ms: Couldn't connect to server
```

### Verification - Confirm Defect Present

```bash
command -v agent-browser || true
zsh -lc 'echo $PATH'
curl -sfS -m 3 http://localhost:9222/json/version
```

Output:

```text
/Library/Frameworks/Python.framework/Versions/3.13/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/System/Cryptexes/App/usr/bin:/usr/bin:/bin:/usr/sbin:/sbin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/local/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/appleinternal/bin:/opt/pmk/env/global/bin:/Users/zacfarrell/.codex/tmp/arg0/codex-arg05aMmUv:/Users/zacfarrell/.bun/bin:/Users/zacfarrell/.npm-global/bin:/Users/zacfarrell/.local/bin:/Library/Frameworks/Python.framework/Versions/3.13/bin:/Users/zacfarrell/.cargo/bin:/Applications/Warp.app/Contents/Resources/bin
curl: (7) Failed to connect to localhost port 9222 after 0 ms: Couldn't connect to server
```

---

## EVIDENCE BUNDLE

### E0 - Environment Evidence

**E0.1: OS and toolchain**

```bash
uname -a
bun --version
cargo --version
rustup show active-toolchain
```

```text
Darwin Mac 25.3.0 Darwin Kernel Version 25.3.0: Wed Jan 28 20:56:34 PST 2026; root:xnu-12377.91.3~2/RELEASE_ARM64_T8122 arm64
1.3.13
cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)
nightly-aarch64-apple-darwin (overridden by '/Users/zacfarrell/Desktop/Testproject1/rust-toolchain.toml')
```

**E0.2: Git state**

```bash
git status --short --branch
git log --oneline -1
```

```text
## main...origin/main
 M .DS_Store
?? bin/__pycache__/
4d1d1dd docs: persist machine fork e2e helpers
```

### E2 - Reproduction Evidence

**E2.1: Required browser CLI is absent**

```bash
which agent-browser || true
```

```text

```

**E2.2: Direct invocation fails**

```bash
agent-browser --cdp 9222 open "about:blank"
```

```text
zsh:1: command not found: agent-browser
```

**E2.3: PATH contains common local binary locations but not a resolving `agent-browser`**

```bash
zsh -lc 'echo $PATH'
```

```text
/Library/Frameworks/Python.framework/Versions/3.13/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/System/Cryptexes/App/usr/bin:/usr/bin:/bin:/usr/sbin:/sbin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/local/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/appleinternal/bin:/opt/pmk/env/global/bin:/Users/zacfarrell/.codex/tmp/arg0/codex-arg05aMmUv:/Users/zacfarrell/.bun/bin:/Users/zacfarrell/.npm-global/bin:/Users/zacfarrell/.local/bin:/Library/Frameworks/Python.framework/Versions/3.13/bin:/Users/zacfarrell/.cargo/bin:/Applications/Warp.app/Contents/Resources/bin
```

**E2.4: Persistent CDP browser is also absent**

```bash
curl -sfS -m 3 http://localhost:9222/json/version
```

```text
curl: (7) Failed to connect to localhost port 9222 after 0 ms: Couldn't connect to server
```

**E2.5: Other required local tools resolve**

```bash
which cargo
which bun
which claude || true
which codex || true
```

```text
/Users/zacfarrell/.cargo/bin/cargo
/Users/zacfarrell/.bun/bin/bun
/Users/zacfarrell/.local/bin/claude
/opt/homebrew/bin/codex
```

---

## CODEPATH COMPARISON

### Working Path: Non-Browser Toolchain

Flow:
1. `cargo`, `bun`, `claude`, and `codex` are present on `PATH`.
2. Rust and TypeScript static/unit gates can be started after product blockers are fixed.

Key evidence: E2.5.

### Broken Path: Browser/CDP Toolchain

Flow:
1. `agent-browser` lookup returns no executable.
2. Direct `agent-browser --cdp 9222 ...` command fails at shell resolution.
3. The Chrome CDP endpoint on port 9222 is not listening.
4. Source login checks and daemon-driven browser skills cannot be validated.

Key evidence: E2.1, E2.2, E2.4.

### Divergence Points

| # | Working Tool Path | Broken Browser Path | Potential Impact |
|---|---|---|---|
| 1 | Cargo/Bun commands resolve. | `agent-browser` does not resolve. | Browser test and CDP preflight cannot run. |
| 2 | Local command versions can be captured. | CDP `/json/version` cannot be captured. | Validation manifest cannot record required CDP browser version. |
| 3 | Daemon can later run non-browser stages after blockers are fixed. | Browser-required skills cannot be preflighted or retried. | Stages 1, 4, 9, 11, desktop/browser smoke are blocked. |

---

## ROOT CAUSE HYPOTHESES

### H1: The agent-browser CLI is not installed on this machine fork - LIKELIHOOD: HIGH

Description: `which agent-browser` returns no path, and shell invocation fails with `command not found`.

Supporting evidence: E2.1, E2.2, E2.3.

Contradicting evidence: none.

Verification steps:
1. Check expected install locations for the project or Codex browser tooling.
2. Install or expose `agent-browser` on `PATH`.
3. Re-run `which agent-browser` and `agent-browser --cdp 9222 open about:blank`.

Fix scope: machine setup, shell PATH, or project tooling install docs.

### H2: The CLI exists outside PATH but was not exported into this Codex shell - LIKELIHOOD: MEDIUM

Description: The command may be installed in a nonstandard location, but the active zsh `PATH` does not include that directory.

Supporting evidence: E2.3 shows the active PATH; E2.1 shows lookup failure.

Contradicting evidence: no candidate path was discovered during preflight.

Verification steps:
1. Search known local tool roots manually or consult the project setup docs.
2. Add the correct directory to PATH for this shell.
3. Re-run the preflight commands.

Fix scope: shell profile, Codex tool bootstrap, or QA runner environment.

### H3: Persistent Chrome CDP was never launched for this machine fork - LIKELIHOOD: HIGH

Description: Even after the CLI is available, the required Chrome profile must be launched on port 9222.

Supporting evidence: E2.4.

Contradicting evidence: none.

Verification steps:
1. Launch Chrome with the canonical remote debugging command.
2. Run `curl -sfS -m 3 http://localhost:9222/json/version`.
3. Run `agent-browser --cdp 9222 open about:blank`.

Fix scope: operator machine setup, Chrome launch/profile.

---

## INVESTIGATION ROADMAP

### Step 1: Restore `agent-browser`

Run:

```bash
command -v agent-browser || true
```

If no path is returned, install or expose the CLI. If a path is returned, continue to Step 2.

### Step 2: Restore persistent Chrome CDP

Run:

```bash
open -na "Google Chrome" --args \
  "--remote-debugging-port=9222" \
  "--remote-allow-origins=*" \
  "--user-data-dir=$HOME/.chrome-automation" \
  "--profile-directory=Default" \
  "--no-first-run" \
  "--no-default-browser-check"
curl -sfS -m 3 http://localhost:9222/json/version
```

If the curl command fails, repair Chrome/CDP setup before resuming QA.

### Step 3: Validate CLI to CDP connection

Run:

```bash
agent-browser --cdp 9222 open "about:blank"
agent-browser --cdp 9222 get title
```

If these fail after `agent-browser` is installed and CDP is listening, inspect CLI/CDP compatibility.

---

## FIX CONSTRAINTS

1. Do not close or replace the operator's persistent CDP profile during QA.
2. Do not run `agent-browser close` or `agent-browser close --all`.
3. Do not ask for credentials in chat.
4. Do not automate captcha solving.
5. Do not continue source skills until the same CDP browser state used by the daemon has passed preflight.

---

## ACCEPTANCE CRITERIA

1. Required browser CLI resolves:

```bash
which agent-browser
```

2. Persistent CDP listener returns browser metadata:

```bash
curl -sfS -m 3 http://localhost:9222/json/version
```

3. `agent-browser` can control the persistent CDP browser:

```bash
agent-browser --cdp 9222 open "about:blank"
agent-browser --cdp 9222 get title
```

4. Login checks can resume:

```bash
agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"
agent-browser --cdp 9222 snapshot
agent-browser --cdp 9222 open "https://www.alibaba.com/"
agent-browser --cdp 9222 snapshot
agent-browser --cdp 9222 open "https://www.searates.com/logistics-explorer/"
agent-browser --cdp 9222 snapshot
agent-browser --cdp 9222 open "https://www.ebay.co.uk/"
agent-browser --cdp 9222 snapshot
```

5. Machine-fork QA can resume from Environment Preflight and then Isolated Run Setup.

---

## FILE MAP

| File/Command | Why It Matters |
|---|---|
| `agent-browser` | Required CLI for persistent CDP browser preflight and daemon-driven browser skills. |
| `http://localhost:9222/json/version` | Required CDP endpoint for Chrome browser metadata. |
| `skills/procurement-ingestion/*/SKILL.md` | In-scope procurement skills require persistent CDP. |
| `skills/marketplace-search/*/SKILL.md` | In-scope marketplace skills require persistent CDP. |
| `skills/landed-cost/shipment-sizing/SKILL.md` | Stage 11 listing inspection can require persistent CDP. |
| `skills/landed-cost/searates-freight-estimate/SKILL.md` | Stage 11 freight estimate requires persistent CDP. |

---

## SELF-VERIFICATION

- Reproduction protocol uses exact commands and captured outputs.
- Evidence items E0.1 through E2.5 back the defect claim.
- The first missing CDP listener was classified as operator setup pause, while missing `agent-browser` was documented as a machine setup blocker.
- Hypotheses are ranked and tied to evidence.
- Acceptance criteria use the same commands that failed in preflight.
