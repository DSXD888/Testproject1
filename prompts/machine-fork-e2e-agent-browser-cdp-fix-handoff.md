# Machine Fork E2E agent-browser/CDP Setup Fix Handoff

ULTRATHINK and be meticulous. This is a fix handoff for a deterministic machine-setup blocker, not a product-code bug.

## Mission

Restore the browser validation preflight for `/Users/zacfarrell/Desktop/Testproject1` so machine-fork L0-L3 Stages 1-14 E2E QA can resume.

The blocker has two parts:

1. `agent-browser` is not available on `PATH`.
2. The operator persistent Chrome CDP endpoint is not listening on `localhost:9222`.

The fix is complete only when the same shell can resolve `agent-browser`, query `http://localhost:9222/json/version`, and drive the persistent CDP browser with `agent-browser --cdp 9222`.

## Source Evidence

Primary error manifest:

- `prompts/machine-fork-e2e-agent-browser-missing-error-manifest.md`

Paused validation manifest:

- `docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md`

Relevant preflight contract in the machine-fork E2E handoff:

- `prompts/machine-fork-l0-l3-stages-1-14-e2e-handoff.md`, Environment Preflight / Persistent CDP Browser

Daemon-injected CDP contract:

- `crates/daemon/src/prompt/renderer.rs` says CDP-required skills must use `agent-browser --cdp ${AGENT_BROWSER_CDP_PORT}`, must not close the operator browser, and must write a blocked result if CDP is unreachable.

Official agent-browser docs checked on 2026-05-12:

- Installation: `https://agent-browser.dev/installation`
- CDP mode: `https://agent-browser.dev/cdp-mode`

The official docs list global installation via `npm install -g agent-browser`, Homebrew via `brew install agent-browser`, Cargo via `cargo install agent-browser`, and first-time browser install via `agent-browser install`. The CDP docs confirm `agent-browser --cdp 9222 ...` connects to a local browser exposed through `http://localhost:9222`.

## Current Reproduction Evidence

Captured in `/Users/zacfarrell/Desktop/Testproject1` on 2026-05-12T19:16:34Z.

Environment:

```bash
uname -a
bun --version
cargo --version
rustup show active-toolchain
```

Observed:

```text
Darwin Mac 25.3.0 Darwin Kernel Version 25.3.0: Wed Jan 28 20:56:34 PST 2026; root:xnu-12377.91.3~2/RELEASE_ARM64_T8122 arm64
1.3.13
cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)
nightly-aarch64-apple-darwin (overridden by '/Users/zacfarrell/Desktop/Testproject1/rust-toolchain.toml')
```

Git state:

```bash
git status --short --branch
git log --oneline -1
```

Observed:

```text
## main...origin/main
M  .DS_Store
A  bin/__pycache__/br-claimcpython-313.pyc
A  bin/__pycache__/br-completecpython-313.pyc
2a7d8ec docs: record machine fork e2e pause
```

Do not revert, stash, delete, or otherwise disturb those existing dirty artifacts. Treat them as shared checkout state.

Tool lookup:

```bash
which cargo
which bun
which claude || true
which codex || true
which agent-browser || true
```

Observed:

```text
/Users/zacfarrell/.cargo/bin/cargo
/Users/zacfarrell/.bun/bin/bun
/Users/zacfarrell/.local/bin/claude
/opt/homebrew/bin/codex
agent-browser not found
```

Direct invocation:

```bash
zsh -lc 'agent-browser --cdp 9222 open "about:blank"'
```

Observed:

```text
zsh:1: command not found: agent-browser
```

CDP listener:

```bash
curl -sfS -m 3 http://localhost:9222/json/version
lsof -nP -iTCP:9222 -sTCP:LISTEN || true
```

Observed:

```text
curl: (7) Failed to connect to localhost port 9222 after 0 ms: Couldn't connect to server
```

`lsof` produced no listener rows.

Chrome is installed and running, but the observed Chrome process list did not include `--remote-debugging-port=9222`. `/Applications/Google Chrome.app` exists.

Known local binary roots checked:

```bash
find "$HOME/.local/bin" "$HOME/.npm-global/bin" "$HOME/.bun/bin" "$HOME/.cargo/bin" "/opt/homebrew/bin" -maxdepth 1 \( -type f -o -type l \) -name 'agent-browser*' 2>/dev/null
npm list -g --depth=0 2>/dev/null | rg 'agent-browser|vercel' || true
```

Observed: no `agent-browser` binary or global npm package was found.

## Root Cause Hypotheses

### H1 - CLI missing from this machine fork

Likelihood: high.

Evidence: `which agent-browser` fails, direct invocation exits 127, common bin roots contain no binary, and global npm package listing does not show `agent-browser`.

Fix scope: machine setup only. Prefer a global install or Homebrew install that lands in an existing `PATH` directory. Avoid repo package churn unless deliberately choosing to pin `agent-browser` as a project dependency.

### H2 - CLI installed outside active PATH

Likelihood: medium.

Evidence: active `PATH` includes common roots but no command resolves. It is still possible another install root exists outside those checked paths.

Fix scope: expose the existing install location to the shell, or replace it with a documented global install. Do not change project source just to paper over a shell profile issue.

### H3 - CDP browser not launched with remote debugging

Likelihood: high.

Evidence: Chrome exists and is running, but port 9222 has no listener and `/json/version` refuses connection.

Fix scope: launch a separate operator automation Chrome instance with `--remote-debugging-port=9222` and a non-default user-data-dir, or repair the existing operator automation profile. Do not close the operator's normal Chrome windows unless the operator explicitly authorizes it.

## Fix Constraints

1. Do not delete files. Do not run `rm`, `git clean`, `git reset --hard`, or any destructive cleanup.
2. Do not revert/stash shared checkout changes.
3. Do not run `agent-browser close`, `agent-browser close --all`, or any command that closes the operator's browser during validation.
4. Do not continue source skills or daemon browser validation until the CDP preflight passes.
5. Do not ask for credentials in chat. If a site requires login or captcha, pause and ask the operator to complete it in the CDP browser.
6. If you install `agent-browser`, record the exact install method and resulting version in the validation manifest.

## Fix Loop

Run this loop until all acceptance commands pass.

### Loop 0 - Reproduce

```bash
cd /Users/zacfarrell/Desktop/Testproject1
command -v agent-browser || true
zsh -lc 'agent-browser --cdp 9222 open "about:blank"'
curl -sfS -m 3 http://localhost:9222/json/version
```

Expected before fix: no command path, `command not found`, and curl connection failure.

### Loop 1 - Restore agent-browser CLI

First check whether the command exists outside `PATH`:

```bash
find "$HOME" "/opt/homebrew" -maxdepth 4 \( -type f -o -type l \) -name 'agent-browser*' 2>/dev/null | sed -n '1,40p'
```

If a valid binary exists, expose its directory to this shell and document the path.

If no valid binary exists, install using one official method. On this macOS machine, prefer a method that places the binary in an existing `PATH` directory:

```bash
brew install agent-browser
```

If Homebrew does not have the formula or fails for environment reasons, use the repo-documented npm path:

```bash
npm install -g agent-browser
```

Then install or verify the browser payload if the CLI requires it:

```bash
agent-browser install
agent-browser --version
agent-browser doctor --offline --quick
```

If `agent-browser doctor --offline --quick` is unavailable in the installed version, record that and continue with `agent-browser --version` plus the CDP validation below.

### Loop 2 - Restore persistent CDP browser

Use the project canonical launch shape:

```bash
open -na "Google Chrome" --args \
  "--remote-debugging-port=9222" \
  "--remote-allow-origins=*" \
  "--user-data-dir=$HOME/.chrome-automation" \
  "--profile-directory=Default" \
  "--no-first-run" \
  "--no-default-browser-check"
```

Then verify:

```bash
curl -sfS -m 3 http://localhost:9222/json/version
lsof -nP -iTCP:9222 -sTCP:LISTEN
```

If curl still fails, inspect whether Chrome reused an existing normal profile instead of launching the automation profile. Use process inspection only:

```bash
ps -axo pid,command | rg 'Google Chrome|remote-debugging-port=9222|chrome-automation' | rg -v 'rg '
```

Do not kill Chrome processes without explicit operator authorization.

### Loop 3 - Validate agent-browser controls CDP

```bash
agent-browser --cdp 9222 open "about:blank"
agent-browser --cdp 9222 get title
agent-browser --cdp 9222 snapshot
```

Expected: commands complete without shell-resolution failure or CDP connection failure.

If the CLI is present and curl returns JSON but `agent-browser --cdp 9222 ...` fails, inspect CLI/CDP compatibility:

```bash
agent-browser --version
agent-browser --help | sed -n '1,80p'
agent-browser --cdp 9222 snapshot --debug
```

Record the exact error and update the error manifest if it reveals a different defect.

### Loop 4 - Resume login checks

Only after Loop 3 passes:

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

If any page requires login, MFA, or captcha, pause and ask the operator to complete it in the visible CDP browser. Retry the same command after the operator says it is complete.

## Validate Test

Create one machine preflight validation command block and run it after every fix attempt:

```bash
set -euo pipefail
cd /Users/zacfarrell/Desktop/Testproject1

echo "== cli =="
which agent-browser
agent-browser --version

echo "== cdp =="
curl -sfS -m 3 http://localhost:9222/json/version

echo "== control =="
agent-browser --cdp 9222 open "about:blank"
agent-browser --cdp 9222 get title
agent-browser --cdp 9222 snapshot

echo "AGENT_BROWSER_CDP_PREFLIGHT_GREEN"
```

Pass condition:

- command exits 0
- `which agent-browser` prints a path
- `agent-browser --version` prints a version
- curl prints Chrome/Protocol metadata JSON
- `agent-browser --cdp 9222` commands complete
- terminal prints `AGENT_BROWSER_CDP_PREFLIGHT_GREEN`

Fail condition:

- any command exits non-zero
- shell says `command not found`
- curl cannot connect to port 9222
- `agent-browser` reports a CDP connection or protocol error

When the validate test fails, return to the smallest relevant loop:

- `command not found` -> Loop 1
- curl fails -> Loop 2
- curl passes but `agent-browser --cdp` fails -> Loop 3
- source login/captcha appears -> Loop 4

## Acceptance Criteria

All of these must be true before handing back to the E2E runner:

- `which agent-browser` resolves to a real executable path.
- `agent-browser --version` returns a version.
- `curl -sfS -m 3 http://localhost:9222/json/version` returns Chrome/Protocol metadata.
- `agent-browser --cdp 9222 open "about:blank"` succeeds.
- `agent-browser --cdp 9222 get title` succeeds.
- The login-check command set in Loop 4 has been attempted, and any operator-required login/captcha state is documented.
- `docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md` is updated with the fix method, versions, CDP browser metadata, and the next resume point.

## Resume Point

After this handoff is green, return to `prompts/machine-fork-l0-l3-stages-1-14-e2e-handoff.md` at:

1. Environment Preflight -> Toolchain
2. Persistent CDP Browser
3. Login Checks
4. Isolated Run Setup

Do not skip the separate schema-epoch blocker recorded in `prompts/machine-fork-e2e-schema-epoch-mismatch-error-manifest.md`; it is independent from this browser/CDP setup defect.

## Final Reporting Requirements

The fixing agent's final note must include:

- exact `agent-browser` install/exposure method
- `agent-browser --version` output
- `/json/version` output summary, including Chrome and Protocol versions
- whether the CDP browser was launched with `$HOME/.chrome-automation`
- validate test result
- any remaining login/captcha/operator-action pauses
