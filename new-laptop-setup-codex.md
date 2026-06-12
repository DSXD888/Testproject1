# New Laptop Setup - Codex + Procurement Intelligence Platform

Paste this whole file into a fresh Codex session on the new Mac. Tell Codex:

> Set up my Codex environment to match this machine. Walk me through each step, verify after each phase, and stop only when you need a secret, account login, or a destructive action approval.

Treat anything in `<<ANGLE_BRACKETS>>` as a placeholder. Secrets must come from a password manager or a private transfer from the source machine. Do not commit this file if you fill in secrets.

This document is the Codex-side companion to `new-laptop-setup.md`, which was written for Claude Code + Claude Desktop. It intentionally covers different state: `~/.codex`, `.agents` skills, Codex plugins, MCP config, Browser Use, Computer Use, Agent Mail, Beads wrappers, CDP Chrome, and this repo's Bun/Rust/Tauri stack.

---

## Source Machine Snapshot

- macOS, Apple Silicon assumed from the Claude setup guide.
- Codex app installed at `/Applications/Codex.app`.
- PATH `codex`: `codex-cli 0.139.0` from `~/.nvm/versions/node/v20.19.5/lib/node_modules/@openai/codex`.
- App-bundled Codex CLI: `/Applications/Codex.app/Contents/Resources/codex`, currently `0.138.0-alpha.7`.
- Node: `v20.19.5`; npm: `10.8.2`.
- Bun: `1.3.10` (`package.json` pins `packageManager: bun@1.3.10`).
- Rust: nightly via `rust-toolchain.toml`; current cargo is `1.96.0-nightly`.
- Beads: `br 0.1.45`; Beads Viewer/Triage: `bv v0.15.2`.
- UBS: `v5.2.76`.
- Agent Mail: `mcp-agent-mail 0.3.8`.
- User skills:
  - `~/.codex/skills`: 25 top-level dirs, including workflow skills such as `featuremaxxer`, `planmaxer`, `bead-*`, `spec-*`, `research-*`.
  - `~/.agents/skills`: 100 top-level dirs, including `gws-*`, `recipe-*`, `persona-*`, `agent-browser`, `explore`, `code-edit`, `supabase`.
  - Repo skills: 48 `skills/**/SKILL.md` files checked into this project.

Official Codex manual sections checked on 2026-06-10: config basics, advanced config, auth, MCP, skills, plugins, hooks, Computer Use, in-app browser, app, and CLI.

---

## What Codex Setup Actually Is Here

Codex state is not Claude state.

- Codex app + CLI share `~/.codex/config.toml` and auth/session state.
- Codex reads global instructions from `~/.codex/AGENTS.md` or `~/.codex/AGENTS.override.md` if present.
- Codex reads repo instructions from `AGENTS.md` and trusted project `.codex/config.toml` files.
- User skills live mostly in `~/.codex/skills` and `~/.agents/skills`.
- Repo-specific operational skills live in this repo under `skills/`.
- Codex Computer Use is a Codex plugin installed from Codex settings, not Claude Desktop computer-use.
- Browser Use is the Codex in-app browser plugin. It is separate from the project CDP Chrome profile on port `9222`.
- Agent Mail is an external local HTTP MCP server at `http://127.0.0.1:8765/mcp/`.

Do not assume a Claude setup automatically makes Codex ready. The shared pieces are only general OS tooling, Chrome, MCP credentials, and some skill concepts.

---

## Step 1 - Install Base Machine Tooling

Install Xcode Command Line Tools first:

```bash
xcode-select --install
```

Install Homebrew if needed:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Install command-line foundations:

```bash
brew install git jq ripgrep fd direnv
```

Install Node through `nvm` to match this source machine:

```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash
nvm install 20.19.5
nvm use 20.19.5
node --version
npm --version
```

Install Bun:

```bash
curl -fsSL https://bun.sh/install | bash
bun --version    # expect 1.3.10 or newer; this repo pins bun@1.3.10
```

Install Rust nightly with required components:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly --component rustfmt clippy
rustup show
cargo --version
```

Install Google Chrome. This project's CDP helper expects:

```text
/Applications/Google Chrome.app/Contents/MacOS/Google Chrome
```

Install or restore these external project tools:

```bash
br --version
bv --version
ubs --version
mcp-agent-mail --version
```

If any are missing, install them the same way they were installed on the source machine. The exact installers are not encoded in this repo. Do not substitute `bd` for `br`; this project uses `br` and the wrapper scripts in `bin/`.

---

## Step 2 - Install Codex App and CLI

Install the Codex app:

1. Download the Codex app from the official OpenAI Codex download/docs path.
2. Drag it to `/Applications`.
3. Open Codex and sign in with the same ChatGPT/OpenAI account used on the source machine.
4. Choose this project folder and make sure **Local** mode is selected for local repo work.

Install the Codex CLI. This source machine currently has the PATH CLI from the npm package:

```bash
npm install -g @openai/codex
codex --version
which codex
```

Using `npm` here is only for the global Codex CLI install. Inside this project, use Bun only for JavaScript/TypeScript dependencies.

Verify both CLI paths if the app is installed:

```bash
codex --version
/Applications/Codex.app/Contents/Resources/codex --version
```

The versions may differ because the app bundles its own CLI. For terminal work, make sure `which codex` points to the CLI you intend to use.

---

## Step 3 - Sign In and Handle Auth State

Preferred fresh-machine flow:

```bash
codex login
```

Use ChatGPT login unless you intentionally want API-key billing and reduced feature availability.

If browser login is blocked, use device auth:

```bash
codex login --device-auth
```

Only if you need to transfer an existing file-based session, copy:

```text
~/.codex/auth.json
```

Treat `~/.codex/auth.json` like a password. It can contain access tokens. Do not paste it into prompts, docs, issue trackers, or Git.

Recommended credential-storage setting for the new machine:

```toml
cli_auth_credentials_store = "keyring"
```

Use file-based auth only when you deliberately need a portable `auth.json`.

---

## Step 4 - Recreate `~/.codex/config.toml`

Do not blindly paste the entire source config if it contains tokens. Rebuild it from this shape and fill placeholders privately.

```toml
model = "gpt-5.5"
model_reasoning_effort = "xhigh"
model_context_window = 1000000
# Optional: copy numeric values from the source config if you want exact limits.
# model_auto_compact_token_limit = 900000
# tool_output_token_limit = 10000
approval_policy = "never"
sandbox_mode = "danger-full-access"

notify = [
  "/Users/<<NEW_USER>>/.codex/computer-use/Codex Computer Use.app/Contents/SharedSupport/SkyComputerUseClient.app/Contents/MacOS/SkyComputerUseClient",
  "turn-ended",
]

[tui]
status_line = ["model-with-reasoning", "context-remaining", "current-dir", "context-window-size"]

[features]
goals = true
prevent_idle_sleep = true
js_repl = false

[notice]
hide_rate_limit_model_nudge = true

[projects."/Users/<<NEW_USER>>/Desktop/Testproject1"]
trust_level = "trusted"

[plugins."browser-use@openai-bundled"]
enabled = true

[plugins."computer-use@local"]
enabled = true

[plugins."documents@openai-primary-runtime"]
enabled = true

[plugins."spreadsheets@openai-primary-runtime"]
enabled = true

[plugins."presentations@openai-primary-runtime"]
enabled = true

[mcp_servers.exa]
url = "https://mcp.exa.ai/mcp?exaApiKey=<<EXA_API_KEY>>"

[mcp_servers.morph-mcp]
command = "npx"
args = ["-y", "@morphllm/morphmcp", "--api-key", "<<MORPH_API_KEY>>"]
enabled = true
startup_timeout_sec = 45
env = { MORPH_API_KEY = "<<MORPH_API_KEY>>" }

[mcp_servers.mcp_agent_mail]
url = "http://127.0.0.1:8765/mcp/"
startup_timeout_sec = 30
http_headers = { Authorization = "Bearer <<AGENT_MAIL_BEARER_TOKEN>>" }

[mcp_servers.openaiDeveloperDocs]
url = "https://developers.openai.com/mcp"
startup_timeout_sec = 30
```

The source config also has a `node_repl` MCP block pointing into `/Applications/Codex.app/Contents/Resources/cua_node/...`. Prefer letting the Codex app/runtime create that on the new machine. If Browser Use or JS REPL tooling is missing after plugin install, copy the `node_repl` block from the source machine and update paths only if the app path differs.

Useful MCP setup commands if starting from a blank config:

```bash
codex mcp add exa --url "https://mcp.exa.ai/mcp?exaApiKey=<<EXA_API_KEY>>"

codex mcp add morph-mcp \
  --env MORPH_API_KEY="<<MORPH_API_KEY>>" \
  -- npx -y @morphllm/morphmcp --api-key "<<MORPH_API_KEY>>"

codex mcp add mcp_agent_mail \
  --url "http://127.0.0.1:8765/mcp/" \
  --bearer-token-env-var AGENT_MAIL_BEARER_TOKEN

codex mcp add openaiDeveloperDocs --url "https://developers.openai.com/mcp"
```

Then verify inside Codex:

```text
/mcp
```

Expected: `exa`, `morph-mcp`, `mcp_agent_mail`, `openaiDeveloperDocs`, and plugin-provided servers are available or clearly explain why they are disabled.

---

## Step 5 - Install Codex Plugins

Install/enable these from Codex app or CLI `/plugins`:

- Browser Use / `browser-use@openai-bundled`
- Computer Use / `computer-use@local`
- Documents / `documents@openai-primary-runtime`
- Spreadsheets / `spreadsheets@openai-primary-runtime`
- Presentations / `presentations@openai-primary-runtime`

Computer Use setup:

1. Open Codex settings -> Computer Use.
2. Install the Computer Use plugin.
3. Grant macOS permissions:
   - System Settings -> Privacy & Security -> Screen Recording -> Codex
   - System Settings -> Privacy & Security -> Accessibility -> Codex
4. Approve app-level prompts when Codex asks to control a specific app.

Expected local files after Computer Use install:

```text
~/.codex/plugins/computer-use/.codex-plugin/plugin.json
~/.codex/plugins/computer-use/.mcp.json
~/.codex/computer-use/Codex Computer Use.app
```

Browser Use setup:

- Enable Browser Use in `/plugins`.
- Use it for unauthenticated local app previews, `localhost`, `127.0.0.1`, `file://`, screenshots, and UI verification.
- It uses the Codex in-app browser, not your signed-in Chrome profile.

For signed-in websites, browser extensions, or saved Chrome credentials, use normal Chrome/CDP flows from this project instead of Browser Use.

---

## Step 6 - Restore Skills

Codex discovers skills from multiple roots:

- System/bundled skills: installed with Codex.
- User skills: `~/.codex/skills` and `~/.agents/skills`.
- Repo skills: `.agents/skills` and `skills/` when applicable.

On the source machine, create archives:

```bash
tar czf ~/codex-skills.tgz -C ~/.codex skills
tar czf ~/agents-skills.tgz -C ~/.agents skills
```

On the new laptop:

```bash
mkdir -p ~/.codex ~/.agents
tar xzf ~/codex-skills.tgz -C ~/.codex
tar xzf ~/agents-skills.tgz -C ~/.agents
```

If you want a cleaner restore, skip generated/system plugin skill caches and let Codex reinstall them:

```text
~/.codex/skills/.system
~/.codex/skills/codex-primary-runtime
```

Do keep the custom workflow skills and Google Workspace/persona/recipe skills if you use them.

Verify in a new Codex session:

```text
/skills
```

Spot-check that these are visible:

- `explore`
- `code-edit`
- `agent-browser`
- `featuremaxxer`
- `planmaxer`
- `bead-plan-audit`
- `bead-completion-audit`
- `supabase`
- `gws-gmail`

---

## Step 7 - Restore Global Instructions, Rules, and Optional State

Copy if present and wanted:

```text
~/.codex/AGENTS.md
~/.codex/rules/
~/.codex/config.toml
~/.codex/hooks.json
~/.codex/prompts/
```

This source machine has no custom prompt files under `~/.codex/prompts` or `~/.agents/prompts` at the time this guide was written.

Optional state:

- `~/.codex/memories_*.sqlite`: useful if you want Codex app memories to follow you.
- `~/.codex/goals_*.sqlite`: useful if you rely on Codex goal tracking.
- `~/.codex/models_cache.json`: not important; regenerates.

Do not copy by default:

- `~/.codex/sessions/`
- `~/.codex/session_index.jsonl`
- `~/.codex/logs_*.sqlite`
- `~/.codex/state_*.sqlite`
- `~/.codex/cache/`
- `~/.codex/generated_images/`
- old `config.toml.*.bak` files unless you need forensic history.

---

## Step 8 - Project Checkout and Dependencies

Clone or copy the repo to the new machine, then use `main`:

```bash
NEW_USER="<<NEW_USER>>"
REPO_URL="<<REPO_URL>>"

cd "/Users/$NEW_USER/Desktop"
git clone "$REPO_URL" Testproject1
cd Testproject1
git checkout main
git pull --ff-only
```

Install JavaScript dependencies with Bun only:

```bash
bun install
```

Do not run `npm install`, `yarn`, or `pnpm` inside this repo.

Rust will use `rust-toolchain.toml`:

```bash
rustup show
cargo check --all-targets
```

If you use `direnv`, copy defaults only when overrides are needed:

```bash
cp .envrc.example .envrc
direnv allow
```

The defaults in `.envrc.example` cover public/free endpoints:

- HMRC Trade Tariff
- HMRC exchange rates
- Frankfurter FX
- `CDP_ENSURE_HELPER_PATH=bin/ensure-cdp-9222`
- L5 cron switches defaulting to `false`

---

## Step 9 - Restore Project Codex Config

This repo has a project-local Codex config:

```text
.codex/config.toml
```

It currently configures the Agent Mail MCP server with a static bearer token. Treat it as sensitive.

Options:

1. Copy the file privately from the source machine if the token remains valid.
2. Regenerate the Agent Mail bearer token and update `.codex/config.toml`.
3. Prefer an environment-backed user config on the new machine and remove project-local secrets later in a dedicated cleanup.

Verification:

```bash
NEW_USER="<<NEW_USER>>"
codex --cd "/Users/$NEW_USER/Desktop/Testproject1" "Summarize active instruction and config sources."
```

Expected:

- `AGENTS.md` is loaded.
- `/Users/<<NEW_USER>>/Desktop/Testproject1` is trusted.
- Project `.codex/config.toml` is not ignored.
- The Agent Mail MCP server is available.

---

## Step 10 - Agent Mail and Beads

Agent Mail must be reachable before bead-tracked work:

```bash
mcp-agent-mail serve
```

The default local MCP endpoint is:

```text
http://127.0.0.1:8765/mcp/
```

Config file to restore or recreate:

```text
~/.config/mcp-agent-mail/config.env
```

Critical values:

```bash
export AGENT_MAIL_BEARER_TOKEN="<<AGENT_MAIL_BEARER_TOKEN>>"
export AGENT_MAIL_PROJECT_KEY="/Users/<<NEW_USER>>/Desktop/Testproject1"
export AGENT_MAIL_NAME="<<CURRENT_AGENT_NAME>>"
```

In a Codex session, perform the normal Agent Mail bootstrap:

```text
ensure_project(human_key="/Users/<<NEW_USER>>/Desktop/Testproject1")
register_agent(project_key="/Users/<<NEW_USER>>/Desktop/Testproject1", program="codex-cli", model="gpt-5")
set_contact_policy(project_key="/Users/<<NEW_USER>>/Desktop/Testproject1", agent_name=<you>, policy="open")
macro_contact_handshake(...) for visible live peers
```

For bead-tracked work, never bypass the wrappers:

```bash
bin/br-claim <bead-id>
# work, test, commit, push as required
bin/br-complete <bead-id>
```

Do not use raw `br update --status=in_progress` or raw `br close` for tracked work. The wrappers reserve files, send mail, update Beads, sync `.beads/`, and release reservations.

Verify wrapper readiness:

```bash
test -x bin/br-claim
test -x bin/br-complete
bin/br-claim --help
bin/br-complete --help
br ready --json
bv --robot-triage | jq '.quick_ref'
```

---

## Step 11 - CDP Chrome for Project Skills

This repo has many live browser skills that attach to persistent Chrome over CDP. This is separate from Codex Browser Use.

Expected defaults:

```text
CDP port: 9222
CDP host: 127.0.0.1
Chrome profile: ~/.chrome-automation
Helper: bin/ensure-cdp-9222
```

Start or verify CDP:

```bash
bin/ensure-cdp-9222
curl -sf http://127.0.0.1:9222/json/version
```

If this is a live operations machine, consider privately migrating:

```text
~/.chrome-automation
```

That profile may contain signed-in sessions and saved credentials for ProContract, Alibaba/1688, Freightos, SeaRates, and related portals. Treat it as sensitive.

If you do not copy it, manually sign into required websites on the new laptop through the CDP Chrome profile before running CDP-dependent skills.

Do not launch random fresh Chrome profiles for project skills. Use `bin/ensure-cdp-9222` so the daemon and skills share the expected profile and port.

---

## Step 12 - Project Data Continuity Decision

The Claude setup guide says not to carry over:

```text
state/state.db
runtime/daemon.info.json
artifacts/downloads/**
```

That is safe for a fresh development machine. It may be wrong if this laptop is taking over real operations.

This project's architecture treats local SQLite as operational authority for some flows. Before wiping or ignoring source-machine runtime state, decide which mode you want:

### Fresh Dev Machine

Do not copy runtime state. Let the daemon rebuild local state:

```text
state/state.db
runtime/
artifacts/downloads/
target/
apps/web/.next/
```

### Operational Takeover

Make an explicit backup/migration plan before the old machine is retired:

```bash
sqlite3 state/state.db "PRAGMA integrity_check;"
cp state/state.db ~/Desktop/testproject1-state-backup.sqlite
```

Also decide whether to migrate:

```text
artifacts/downloads/
runtime/
~/.chrome-automation
Supabase project env vars and service credentials
Agent Mail archive/index state
```

Do not delete or overwrite runtime state casually. If unsure, keep a private backup and ask before destructive cleanup.

---

## Step 13 - Verification Commands

Run from the repo root:

```bash
git status --short --branch
bun --version
cargo --version
rustup show
codex --version
br --version
bv --version
ubs --version
mcp-agent-mail --version
```

Install/build checks:

```bash
bun install
bun run build
bun run lint
bun run test
```

Rust gates:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
```

Full stage-validation gate when writing or validating `docs/pipeline/stage-*-manifest.md`:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

Desktop checks:

```bash
cd apps/desktop
bun run test
bun run build
bun run dev
# for native desktop:
bun run tauri:dev
```

Web checks:

```bash
cd apps/web
bun run test
bun run lint
bun run build
bun run dev
```

CDP check:

```bash
bin/ensure-cdp-9222
curl -sf http://127.0.0.1:9222/json/version
```

Codex checks inside a new Codex session:

```text
/mcp
/plugins
/skills
```

Ask Codex:

```text
Use Browser Use to open http://127.0.0.1:5173 after I start the desktop Vite server and verify the page renders.
```

Ask Codex:

```text
Use Computer Use to take a screenshot of Finder.
```

Expected: macOS permission prompts appear if not already granted, then Codex returns a screenshot or a clear permission error.

---

## Step 14 - Secrets Checklist

Move these through a password manager, private encrypted transfer, or direct local copy. Never paste filled values into Git, tickets, or shared docs.

- `~/.codex/auth.json`, if using file-based Codex auth.
- `OPENAI_API_KEY`, if using API-key auth.
- `CODEX_ACCESS_TOKEN`, if using enterprise access-token auth.
- `EXA_API_KEY`.
- `MORPH_API_KEY`.
- `AGENT_MAIL_BEARER_TOKEN`.
- `~/.config/mcp-agent-mail/config.env`.
- Static bearer token in project `.codex/config.toml`, if kept.
- Supabase URL/anon/service-role keys and any project-specific `.env*`.
- Google Workspace OAuth/token state used by `gws-*` skills.
- Chrome CDP profile `~/.chrome-automation`.
- Any credentials in source-machine shell profiles, password-manager exports, or MCP `http_headers`.

Before sharing this doc, search it for filled secrets:

```bash
rg -n "sk-|Bearer |TOKEN|KEY|SECRET|PASSWORD|SUPABASE|MORPH|EXA" new-laptop-setup-codex.md
```

The placeholders are expected. Real secret values are not.

---

## Things to Not Carry Over by Default

Do not copy these unless you have a specific reason:

- `target/`
- `node_modules/`
- `apps/web/.next/`
- `.DS_Store`
- `~/.codex/sessions/`
- `~/.codex/session_index.jsonl`
- `~/.codex/logs_*.sqlite`
- `~/.codex/state_*.sqlite`
- `~/.codex/cache/`
- `~/.codex/generated_images/`
- `~/.codex/shell_snapshots/`
- `~/.codex/config.toml.*.bak`
- `runtime/daemon.info.json`
- `artifacts/downloads/**`, unless doing an operational takeover
- `state/state.db`, unless doing an operational takeover

Keep or deliberately migrate these:

- `~/.codex/config.toml` after redacting/updating machine-specific paths.
- `~/.codex/AGENTS.md` or `AGENTS.override.md`, if present.
- `~/.codex/rules/`.
- `~/.codex/skills/` custom skill dirs.
- `~/.agents/skills/`.
- `~/.config/mcp-agent-mail/config.env`, privately.
- `~/.chrome-automation`, privately if live browser sessions matter.

---

## What Was Missing from the Claude-Oriented Guide

The Claude guide is useful but does not cover these Codex-specific items:

- Codex user config at `~/.codex/config.toml`.
- Codex project config at `.codex/config.toml`.
- Codex app/plugin state under `~/.codex/plugins`.
- Codex Computer Use plugin and `~/.codex/computer-use/Codex Computer Use.app`.
- Browser Use plugin and in-app browser behavior.
- Node REPL/browser-use runtime wiring from the Codex app.
- User skills split between `~/.codex/skills` and `~/.agents/skills`.
- OpenAI Docs MCP (`openaiDeveloperDocs`) for current Codex/OpenAI docs.
- Current PATH Codex CLI source and version mismatch with the app-bundled CLI.
- Agent Mail bearer-token handling for Codex (`~/.config/mcp-agent-mail/config.env` and project `.codex/config.toml`).
- Explicit Beads wrapper verification (`bin/br-claim`, `bin/br-complete`, `bin/br-sync`).
- CDP Chrome helper details and the sensitive `~/.chrome-automation` profile.
- Data-continuity decision for `state/state.db` instead of a blanket "do not copy" rule.

Use this document as the Codex migration guide; use `new-laptop-setup.md` only for Claude Code/Claude Desktop setup.
