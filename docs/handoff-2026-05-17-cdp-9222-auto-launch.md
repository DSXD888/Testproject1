# Handoff: CDP-9222 Auto-Launch Across All Skills (and Daemon)

> **Generated:** 2026-05-17
> **Trigger:** During L5 prep, the operator hit "CDP not listening on 9222" friction; manually launching Chrome each session is a recurring papercut. This handoff makes CDP self-bootstrapping for any skill that needs it.
> **Scope:** All 16 production CDP-requiring skills + the 2 in-progress operator-built skills (Freightos, Taric) + the daemon's pre-flight gate + AGENTS.md + tests.
> **Outcome when done:** A `bin/ensure-cdp-9222` helper (idempotent) is invoked by every CDP-requiring code path; no skill ever fails again with `CDP_CONNECT_FAILED` solely because the operator forgot to open Chrome.

---

## Mission

Provide a single idempotent helper script (`bin/ensure-cdp-9222`) that, when invoked:

1. **Checks** whether CDP is already reachable on `localhost:9222` (~1 second). If yes, exits 0 silently.
2. **If not reachable**, launches Chrome with the operator's persistent `/Users/alinaaitova/.chrome-automation` profile and the canonical flags, in the background.
3. **Polls** for CDP readiness (up to ~15 seconds). On success, exits 0. On failure, prints diagnostic info and exits non-zero with the same `CDP_CONNECT_FAILED` semantics skills already understand.

Then wire that helper into every place CDP is needed:
- **Every CDP-requiring SKILL.md** has its "Mandatory CDP Discipline" section updated to call the helper instead of just `curl`-ing.
- **The daemon** calls the helper before invoking any skill whose `metadata.requires_cdp == true`.
- **AGENTS.md** gets a new "CDP Browser Prerequisite" section so future agents know.
- **A smoke test** verifies the helper's idempotency + launch behaviour.

Why both skill-side AND daemon-side: defense in depth. Daemon catches 99% of cases (so skills usually never see CDP down). Skill-side fallback catches the 1% where the daemon's pre-flight succeeded but Chrome crashed mid-execution.

---

## Audit findings (the exhaustive list — do NOT miss any)

### 16 production skills that REQUIRE CDP (all on port 9222)

Each declares `requires_cdp: true` + `cdp_default_port: 9222` in YAML frontmatter, and each has a "Mandatory CDP Discipline" section that currently does raw `curl` + return-blocker-on-failure.

```
skills/auth-refresh/1688/SKILL.md
skills/auth-refresh/alibaba/SKILL.md
skills/landed-cost/searates-freight-estimate/SKILL.md
skills/landed-cost/shipment-sizing/SKILL.md
skills/marketplace-search/alibaba/SKILL.md
skills/marketplace-search/amazon-uk/SKILL.md
skills/marketplace-search/ebay-uk/SKILL.md
skills/marketplace-search/gem-imports/SKILL.md
skills/marketplace-search/mx-wholesale/SKILL.md
skills/procurement-ingestion/contracts-finder/SKILL.md
skills/procurement-ingestion/fts/SKILL.md
skills/procurement-ingestion/procontract-discover/SKILL.md
skills/procurement-ingestion/procontract-enrich-tender/SKILL.md
skills/procurement-ingestion/procontract-register-interest/SKILL.md
skills/source-health/alibaba/SKILL.md
skills/source-health/fts/SKILL.md
```

Verify the list before starting:
```bash
grep -rl "requires_cdp:.*true" /Users/alinaaitova/Desktop/Testproject1/skills/ \
  | grep -v "golden.assertions.yaml" \
  | grep -v "checkpoint" \
  | sort
# Expected: exactly 16 SKILL.md paths above.
```

### 2 in-progress operator-built skills (will join the list)

```
skills/landed-cost/freightos-freight-estimate/SKILL.md     (built per HANDOFF-freightos-freight-estimate.md)
skills/landed-cost/taric-cross-check/SKILL.md              (built per HANDOFF-taric-cross-check.md)
```

These don't exist YET — they're built by the L5 operator-built-skills handoffs. **This handoff also updates those two build-handoffs** so their SKILL.md templates emit the new helper-aware "Mandatory CDP Discipline" pattern from day one.

### Skills correctly declared as NOT requiring CDP (do not touch)

- `skills/landed-cost/simple-fx-lookup/SKILL.md` (uses Frankfurter HTTP API)
- `skills/landed-cost/hmrc-customs-classification/SKILL.md` (uses HMRC Trade Tariff HTTP API)
- `skills/pdf-extraction/{base,pricing-schedule,tender-specification,bill-of-quantities}/SKILL.md` (file-based)
- `skills/supplier-price-enrichment/{ebay,alibaba,amazon,1688}/SKILL.md` (no `requires_cdp` field — these are wrappers; sanity-check during execution)
- `skills/validation/{base,overlay-*}/SKILL.md` (validation logic on already-collected data)
- `skills/manifest-generation/SKILL.md` (data aggregation)

If sanity-checking reveals any of the "no field" skills DO use `agent-browser --cdp`, that's a metadata bug — fix the frontmatter to declare `requires_cdp: true` and add the skill to the update list. The skill_lint.rs test should catch this.

### Daemon code touchpoints

```
crates/daemon/src/main.rs:799          struct field declaration for `requires_cdp: bool`
crates/daemon/src/main.rs:8064-8066    EXISTING CDP pre-check for shipment-sizing (only Alibaba today)
crates/daemon/src/main.rs:12284-12313  cdp_port_reachable() function + blocked-result factory
crates/daemon/src/main.rs:16067-16071  sets AGENT_BROWSER_CDP_PORT env var when requires_cdp
crates/daemon/src/prompt/renderer.rs:46-47, 75-88   CDP contract block injection into agent prompt
```

The daemon's `cdp_port_reachable()` function stays — it's a quick read-only check used in the prompt-time decision logic. The NEW work is: before invoking any CDP-requiring skill, the daemon SPAWNS `bin/ensure-cdp-9222` as a pre-flight, capturing exit code.

### Tests touching CDP

```
crates/daemon/tests/skill_lint.rs:44-61, 373-423     SIGNED_IN_BROWSER_SKILLS + CDP_BROWSER_SKILLS const lists + lint
crates/daemon/tests/agent_contract_session.rs        prompt-render tests with requires_cdp
```

This handoff adds **one new test file**: `crates/daemon/tests/cdp_ensure_helper.rs` — a smoke test that asserts the helper exists, is executable, and behaves correctly (check-only when CDP up, launch when CDP down — the latter is mocked since we can't actually launch Chrome in CI).

### Docs

```
AGENTS.md                                                      add new "CDP Browser Prerequisite" section after line 727 (existing agent-browser section)
docs/pipeline/stage-11-cdp-live-demos-continuation-handoff.md  add 1-line pointer to the new helper for future operators reading this
skills/marketplace-search/alibaba/SKILL.md                     (already in skill-update list) — update its Chrome-136+ workaround section to reference the helper
```

### Pre-existing helpers

```
bin/br-claim       (14KB)  agent mail claim wrapper
bin/br-complete    (14KB)  agent mail complete wrapper
bin/br-sync        (1.8KB) agent mail .beads/ sync wrapper
```

The new helper follows the same convention: executable bash script in `bin/`, single responsibility, idempotent, exit-code based.

---

## Architecture: defense in depth

```
┌──────────────────────────────────────────────────────────────────────┐
│                            DAEMON LAYER                              │
│                                                                      │
│  Before invoking any skill where metadata.requires_cdp == true:      │
│    ┌──────────────────────────────────┐                              │
│    │ spawn bin/ensure-cdp-9222         │── exit 0 → proceed          │
│    │ (idempotent, ~1s when CDP up,     │── exit !=0 → emit blocker   │
│    │  ~3-15s when launching)           │   (do not run skill)         │
│    └──────────────────────────────────┘                              │
└──────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (CDP guaranteed reachable)
┌──────────────────────────────────────────────────────────────────────┐
│                            SKILL LAYER                               │
│                                                                      │
│  Inside SKILL.md "Mandatory CDP Discipline" section:                 │
│    ┌──────────────────────────────────┐                              │
│    │ Step 1: bin/ensure-cdp-9222       │── exit 0 → continue         │
│    │   (catches the 1% case where CDP  │── exit !=0 → return blocker  │
│    │    crashed between daemon         │     with CDP_CONNECT_FAILED  │
│    │    pre-flight and skill start)    │                              │
│    └──────────────────────────────────┘                              │
│              │                                                       │
│              ▼                                                       │
│    Step 2: agent-browser --cdp ${CDP_PORT} ...                       │
└──────────────────────────────────────────────────────────────────────┘
```

The helper is **idempotent and fast on the happy path** (single ~1s TCP check), so calling it from both layers costs nothing in normal operation. Cost is paid only when launch is actually needed.

---

## Implementation

### Step 1 — Create `bin/ensure-cdp-9222` (~20 min)

Create the file:

```bash
touch /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
chmod +x /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
```

Write the helper. Paste the complete script below verbatim — it has been designed for the operator's exact environment (macOS + Chrome + `/Users/alinaaitova/.chrome-automation` profile + the same flags from `docs/pipeline/stage-11-cdp-live-demos-continuation-handoff.md`):

```bash
#!/usr/bin/env bash
#
# bin/ensure-cdp-9222 — idempotent CDP launcher.
#
# Behaviour:
#   - If CDP is already reachable on $CDP_PORT (default 9222), exit 0 silently.
#   - If not reachable, launch Chrome with the operator's persistent profile in
#     the background, poll for CDP readiness, exit 0 on success, non-zero on
#     failure with a JSON-line diagnostic on stderr.
#
# Designed for invocation from both:
#   - The Rust daemon's pre-flight gate (before invoking any requires_cdp skill).
#   - Each SKILL.md's "Mandatory CDP Discipline" section (defense in depth).
#
# Exit codes:
#   0 — CDP reachable on $CDP_PORT (either was already up, or we launched and verified).
#   1 — launch failed or post-launch verification timed out.
#   2 — unsupported OS (only macOS is supported today).
#   3 — Chrome binary not found at canonical path.
#
# Environment overrides:
#   CDP_PORT                       (default: 9222)
#   AGENT_BROWSER_CDP_PORT         (alias for CDP_PORT, matches existing skill convention)
#   CDP_USER_DATA_DIR              (default: $HOME/.chrome-automation)
#   CDP_PROFILE_DIRECTORY          (default: Default)
#   CDP_LAUNCH_TIMEOUT_SECONDS     (default: 15 — total seconds to wait for CDP after launch)
#   CDP_HEALTH_CHECK_TIMEOUT_MS    (default: 1000 — milliseconds per health-check probe)

set -euo pipefail

# --- Configuration ----------------------------------------------------------

CDP_PORT="${AGENT_BROWSER_CDP_PORT:-${CDP_PORT:-9222}}"
CDP_USER_DATA_DIR="${CDP_USER_DATA_DIR:-$HOME/.chrome-automation}"
CDP_PROFILE_DIRECTORY="${CDP_PROFILE_DIRECTORY:-Default}"
CDP_LAUNCH_TIMEOUT_SECONDS="${CDP_LAUNCH_TIMEOUT_SECONDS:-15}"
CDP_HEALTH_CHECK_TIMEOUT_MS="${CDP_HEALTH_CHECK_TIMEOUT_MS:-1000}"

CHROME_BIN_MACOS="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"

# Pure-bash JSON-string escape: handles " and \ and newlines. No python3 dep.
json_escape() {
  local s="$1"
  s="${s//\\/\\\\}"      # backslash → \\
  s="${s//\"/\\\"}"       # double quote → \"
  s="${s//$'\n'/\\n}"     # newline → \n
  s="${s//$'\r'/\\r}"     # carriage return → \r
  s="${s//$'\t'/\\t}"     # tab → \t
  printf '%s' "$s"
}

# Diagnostic JSON to stderr — easy for callers (skill or daemon) to log/grep.
# Format is stable: helper, status, port, message, ts.
emit_diagnostic() {
  local status="$1"
  local message="$2"
  local esc_msg
  esc_msg=$(json_escape "$message")
  printf '{"helper":"ensure-cdp-9222","status":"%s","port":%d,"message":"%s","ts":"%s"}\n' \
    "$status" "$CDP_PORT" "$esc_msg" "$(date -u +%FT%TZ)" >&2
}

# --- Helpers ---------------------------------------------------------------

cdp_reachable() {
  # Returns 0 if CDP /json/version responds with HTTP 200 within the health-check timeout.
  local timeout_seconds
  timeout_seconds=$(awk "BEGIN { printf \"%.3f\", $CDP_HEALTH_CHECK_TIMEOUT_MS / 1000 }")
  curl -sfS -m "$timeout_seconds" "http://localhost:${CDP_PORT}/json/version" >/dev/null 2>&1
}

# --- Step 1: fast path — is it already up? ---------------------------------

if cdp_reachable; then
  exit 0
fi

# --- Step 2: validate environment for launch -------------------------------

if [[ "$OSTYPE" != darwin* ]]; then
  emit_diagnostic "unsupported_os" "ensure-cdp-9222 currently supports macOS only; manual Chrome launch required on $OSTYPE"
  exit 2
fi

if [[ ! -x "$CHROME_BIN_MACOS" ]]; then
  emit_diagnostic "chrome_not_found" "Google Chrome not found at $CHROME_BIN_MACOS"
  exit 3
fi

if [[ ! -d "$CDP_USER_DATA_DIR" ]]; then
  # First-time-ever launch: profile dir doesn't exist. Create it so Chrome doesn't
  # bail. Chrome handles empty user-data-dir gracefully — it creates Local State,
  # Default/, Preferences etc on first launch. But the operator will need to
  # MANUALLY sign in to each site (Freightos, ProContract, Alibaba/1688) once
  # before the relevant skills will work. The operator's existing
  # $HOME/.chrome-automation should already exist with prior sessions; this
  # branch is only for fresh machines.
  mkdir -p "$CDP_USER_DATA_DIR"
  emit_diagnostic "info" "created fresh CDP_USER_DATA_DIR at $CDP_USER_DATA_DIR — operator must sign in to sites manually before CDP-requiring skills will succeed"
fi

# --- Step 3: launch Chrome in background -----------------------------------
#
# Quoting note: --remote-allow-origins=* must be QUOTED for zsh (which globs *).
# Bash with set -u tolerates the literal — both shells safe with the array form.

CHROME_ARGS=(
  "--remote-debugging-port=${CDP_PORT}"
  "--remote-allow-origins=*"
  "--user-data-dir=${CDP_USER_DATA_DIR}"
  "--profile-directory=${CDP_PROFILE_DIRECTORY}"
  "--no-first-run"
  "--no-default-browser-check"
)

# Disown so Chrome survives this script exiting. Use nohup + & for portability.
nohup "$CHROME_BIN_MACOS" "${CHROME_ARGS[@]}" >/dev/null 2>&1 &
CHROME_PID=$!
disown "$CHROME_PID" 2>/dev/null || true

# --- Step 4: poll for CDP readiness ----------------------------------------

START_SECONDS=$SECONDS
while (( SECONDS - START_SECONDS < CDP_LAUNCH_TIMEOUT_SECONDS )); do
  if cdp_reachable; then
    emit_diagnostic "launched" "Chrome launched with pid $CHROME_PID; CDP reachable after $(( SECONDS - START_SECONDS ))s"
    exit 0
  fi
  sleep 0.5
done

# --- Step 5: launch failed or did not come up in time ----------------------

emit_diagnostic "launch_failed" "Chrome process $CHROME_PID did not produce a reachable CDP endpoint on port $CDP_PORT within ${CDP_LAUNCH_TIMEOUT_SECONDS}s"
exit 1
```

Verify it works:

```bash
# Smoke 1: helper succeeds when CDP is already up (should be < 1s)
time /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
echo "exit: $?"   # expected 0

# Smoke 2: helper launches Chrome when not running
pkill -f "remote-debugging-port=9222" 2>/dev/null   # bring CDP down
sleep 2
time /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
echo "exit: $?"   # expected 0 after ~3-5s; Chrome window should appear
curl -sfS http://localhost:9222/json/version | jq -r .Browser   # confirm "Chrome/..."

# Smoke 3: helper short-circuits on unsupported OS
OSTYPE=linux-gnu /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
echo "exit: $?"   # expected 2

# Smoke 4: env override works
CDP_PORT=9333 /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
echo "exit: $?"   # expected 0 after launching a SECOND Chrome on 9333 (or 1 if conflicting)
# Cleanup: pkill -f "remote-debugging-port=9333"
```

### Step 2 — Update all 16 production CDP-requiring SKILL.md files (~60-90 min)

For each skill in the list, locate its existing "Mandatory CDP Discipline" section and replace it with the new helper-aware version below.

**Find current pattern:** roughly this shape (exact wording varies per skill):

```markdown
## Mandatory CDP Discipline
Before navigation:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, emit `<status_field>: "blocked"`, `cdp_state: "unreachable"`, `captcha_intercepted: false`, with blocker code `CDP_CONNECT_FAILED`. Do not launch a fresh browser. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```
```

**Pre-step 2.0 (MUST do before editing): discover each skill's actual status field name.**

The skill subprocess cwd is the per-skill workdir (`launch_spec.cwd`), NOT the project root — so the helper invocation in SKILL.md must use an ABSOLUTE path, and each skill's existing status field name (`freight_status`, `sizing_status`, etc.) must be preserved verbatim. Grep first:

```bash
# Extract every skill's status field name and the absolute path the helper goes at
for f in $(find /Users/alinaaitova/Desktop/Testproject1/skills -name SKILL.md \
  | xargs grep -l "requires_cdp:.*true" \
  | sort); do
  fname=$(echo "$f" | sed 's|.*/skills/||;s|/SKILL.md||')
  field=$(grep -oE '"[a-z_]+_status"' "$f" | sort -u | head -3 | tr '\n' ' ')
  echo "$fname  →  status_field(s): $field"
done
```

Empirically verified field names (cross-check the above against this table; if your grep finds a different name for any skill, the grep wins):

| Skill | status field name (verified) |
|---|---|
| `landed-cost/searates-freight-estimate` | `freight_status` |
| `landed-cost/shipment-sizing` | `sizing_status` |
| `marketplace-search/alibaba` | `search_status` (grep to confirm) |
| `marketplace-search/{amazon-uk,ebay-uk,gem-imports,mx-wholesale}` | `search_status` (grep each) |
| `procurement-ingestion/contracts-finder` | `ingestion_status` (grep to confirm) |
| `procurement-ingestion/fts` | (no `*_status` field — uses `cdp_state` + custom shape; grep to confirm) |
| `procurement-ingestion/procontract-{discover,enrich-tender,register-interest}` | grep each |
| `source-health/{alibaba,fts}` | `health_status` (grep to confirm) |
| `auth-refresh/{1688,alibaba}` | `refresh_status` (grep to confirm) |

**Replace each skill's existing "Mandatory CDP Discipline" block with this verbatim block** (substitute `<STATUS_FIELD>` with the name you grepped — do NOT rename what the skill already uses):

```markdown
## Mandatory CDP Discipline

Before any browser command, ensure CDP is up. The daemon's pre-flight gate
SHOULD have done this already, but defense in depth: call the helper here
as a fast no-op when CDP is up, or to recover if Chrome crashed mid-execution.

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# bin/ensure-cdp-9222 (absolute path — skill cwd is the per-skill workdir,
# not the project root). Idempotent: returns 0 in <1s if CDP is up; launches
# Chrome with the operator's /Users/alinaaitova/.chrome-automation profile if
# not, and polls up to 15s. Returns non-zero only if launch genuinely failed.
if ! /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222; then
  # Helper exited non-zero — CDP is genuinely unavailable. Write blocked
  # result.json IMMEDIATELY (don't launch a browser ourselves — the helper tried).
  cat > result.json <<'BLOCKED_JSON'
{
  "<STATUS_FIELD>": "blocked",
  "cdp_state": "unreachable",
  "captcha_intercepted": false,
  "warnings": [],
  "blockers": [{
    "code": "CDP_CONNECT_FAILED",
    "message": "bin/ensure-cdp-9222 could not establish a reachable CDP endpoint on port 9222. See helper diagnostic JSON on stderr.",
    "severity": "blocked"
  }]
}
BLOCKED_JSON
  exit 0   # exits the SHELL subprocess for this command block; the skill agent
           # has already written result.json above and the daemon's post-run
           # reader will pick it up. Do NOT proceed to any browser command.
fi

# Sanity check (cheap, defensive). If this fails after the helper succeeded,
# something genuinely weird is happening (race, port hijack). Treat as blocked.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version" >/dev/null

# All browser commands from here on:
# agent-browser --cdp ${CDP_PORT} ...
```

If CDP becomes unreachable MID-EXECUTION (captcha challenge, network blip, browser crash mid-eval), emit `<STATUS_FIELD>: "blocked"`, `cdp_state: "unreachable"` (or `captcha_blocked` if it's specifically a captcha challenge), `captcha_intercepted: false` (or `true` for captcha), with blocker code `CDP_CONNECT_FAILED`. Do not launch a fresh browser bypass.
```

**Note on `exit 0` semantics:** the `exit 0` inside the helper-failed branch exits the SHELL SUBPROCESS that was running this command block. The agent (codex CLI) sees the shell exit cleanly with `result.json` already written. The daemon's post-run reader picks up the blocked result. This matches the existing skill convention.

**Special case — `skills/marketplace-search/alibaba/SKILL.md`:** this skill ALSO has a detailed "operator browser launch ceremony" section (around lines 32-66) with the Chrome-136+ workaround for non-default user-data-dir. After updating its "Mandatory CDP Discipline" to use the helper, ALSO add a one-line cross-reference at the top of the launch-ceremony section: *"For routine launches, `bin/ensure-cdp-9222` handles this automatically (and the persistent profile at `/Users/alinaaitova/.chrome-automation` IS already a non-default dir, so the Chrome-136+ constraint is satisfied). The manual ceremony below is for first-time profile creation or recovery from a broken profile state only."*

**Verification grep after editing all 16** (FIXED — uses `find` instead of bash `**`, and filters out `golden.assertions.yaml` which is the one known false-positive):

```bash
# Every CDP-requiring SKILL.md must invoke the helper in its body.
# (Uses `find` because bash `**` requires `shopt -s globstar` and silently
# matches nothing if not enabled — that was a bug in the prior version of
# this command.)
for f in $(find /Users/alinaaitova/Desktop/Testproject1/skills -name SKILL.md -type f \
  | xargs grep -l "requires_cdp:.*true"); do
  if ! grep -q "ensure-cdp-9222" "$f"; then
    echo "MISSING helper invocation: $f"
  fi
done
# Expected output: zero lines. Any line printed = a skill you missed.

# Also verify the count is exactly 16:
find /Users/alinaaitova/Desktop/Testproject1/skills -name SKILL.md -type f \
  | xargs grep -l "requires_cdp:.*true" \
  | wc -l
# Expected: 16
```

### Step 3 — Update the 2 in-progress operator-built skill handoffs (~15 min)

```
docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/HANDOFF-freightos-freight-estimate.md
docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/HANDOFF-taric-cross-check.md
```

**Conditional:** if the Freightos / Taric SKILL.md files don't yet exist on disk (operator hasn't built them yet — they're the deliverables of `HANDOFF-freightos-freight-estimate.md` / `HANDOFF-taric-cross-check.md`), update only those two build-handoff templates instead. When the operator later builds the skills, they'll inherit the helper-aware pattern from the updated templates.

In each handoff (whether updating the template or the produced SKILL.md), find the "Mandatory CDP Discipline" section (around lines 200-280 in each handoff) and replace its current raw-curl pattern with the helper-aware pattern from Step 2. Use status fields `freight_status` (Freightos) and `taric_status` (Taric).

Also update each handoff's Phase C.0.a "verify Chrome ready" checklist to reference the helper:

```markdown
##### C.0.a — Verify CDP is up (auto-launch if not)

```bash
# The helper is idempotent: instant if CDP is up, ~3-15s if it needs to launch Chrome.
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
echo "exit: $?"
# Expected: exit 0. If 1, see helper's diagnostic JSON on stderr.

# Verify Chrome
curl -sfS http://localhost:9222/json/version | jq '{browser: .Browser, userAgent: .["User-Agent"]}'
```

(The previous manual `/Applications/Google Chrome.app/...` launch command in this handoff is now only needed for the operator's first-ever setup of `/Users/alinaaitova/.chrome-automation`, or to launch a fresh Chrome with non-default flags. Routine launches go through the helper.)
```

### Step 4 — Update daemon: pre-flight call before every CDP-requiring skill invocation (~30-45 min)

The daemon currently:
- Sets `AGENT_BROWSER_CDP_PORT` env var when launching a skill with `requires_cdp` (`main.rs:16067`).
- Does a one-off `cdp_port_reachable(9222)` check before Alibaba shipment-sizing (`main.rs:8064`).

Add a centralised pre-flight at the skill-invocation site. The cleanest insertion is in the agent spawner — wherever the daemon converts a skill metadata struct into a process invocation.

**Edit:** `crates/daemon/src/main.rs` (or wherever the agent spawn site is — the audit found references at lines 1235, 6526, 11834, 14201, 16067 — pick the one closest to the actual `Command::new(...)` site).

**IMPORTANT context before writing the Rust code:** verified during audit:

- The daemon does NOT have a `project_root` field. The agent-spawn sites use `launch_spec.cwd` (the per-skill workdir), e.g., `crates/daemon/src/main.rs:6560, 11875, 15827` (`.current_dir(&launch_spec.cwd)`).
- The skill writes its own `result.json` to `launch_spec.cwd`, and the daemon reads it back from `launch_spec.cwd.join("result.json")` (`main.rs:15611, 15871, 15874`).
- There is no unified "skill result" struct — each skill has its own status field name (`freight_status`, `sizing_status`, `search_status`, `taric_status`, `classification_status`, `ingestion_status`, `health_status`, `refresh_status`). The daemon's post-run reader parses JSON generically.

This shapes the design: the daemon's pre-flight gate writes a generic `result.json` directly to `launch_spec.cwd` with ALL possible status fields set to `"blocked"` (downstream readers only consume their own field; extras are ignored), then skips spawning the skill. No new typed-result enum needed.

**Add the helper functions** near `cdp_port_reachable()` (around `main.rs:12284`):

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve the absolute path to `bin/ensure-cdp-9222`.
///
/// Resolution order:
///   1. Env var `CDP_ENSURE_HELPER_PATH` (escape hatch for non-standard
///      operator setups).
///   2. Compile-time workspace-root walk from CARGO_MANIFEST_DIR (works in
///      `cargo test` + `cargo run` from the repo).
///   3. Hardcoded operator default `/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222`
///      (only correct on the current operator's machine; document this).
fn default_ensure_cdp_helper_path() -> PathBuf {
    if let Ok(p) = std::env::var("CDP_ENSURE_HELPER_PATH") {
        return PathBuf::from(p);
    }
    // CARGO_MANIFEST_DIR at compile time = crates/daemon; ../../bin gives project root/bin.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let from_manifest = Path::new(manifest_dir)
        .parent().and_then(|p| p.parent())
        .map(|p| p.join("bin").join("ensure-cdp-9222"));
    if let Some(p) = from_manifest.as_ref() {
        if p.exists() {
            return p.clone();
        }
    }
    // Final fallback — operator-machine default.
    PathBuf::from("/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222")
}

/// Run `bin/ensure-cdp-9222` to guarantee CDP is reachable before invoking a
/// CDP-requiring skill. Returns Ok(()) if the helper exits 0 (CDP up, either
/// because it already was or because the helper successfully launched Chrome).
/// Returns Err(diagnostic_message) if the helper exits non-zero — caller writes
/// a CDP_CONNECT_FAILED blocker `result.json` to `launch_spec.cwd` and skips
/// spawning the skill.
fn ensure_cdp_9222(cdp_port: u16) -> Result<(), String> {
    let helper = default_ensure_cdp_helper_path();
    if !helper.exists() {
        return Err(format!("ensure-cdp-9222 helper not found at {}", helper.display()));
    }
    let output = Command::new(&helper)
        .env("CDP_PORT", cdp_port.to_string())
        .output()
        .map_err(|e| format!("failed to spawn {}: {}", helper.display(), e))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "ensure-cdp-9222 exited with status {:?} — stderr: {}",
            output.status.code(),
            stderr.trim()
        ))
    }
}

/// Write a generic CDP_CONNECT_FAILED blocker `result.json` to the skill's
/// workdir. Sets EVERY status field name used across CDP-requiring skills, so
/// whichever field the downstream reader looks for, it finds "blocked". The
/// daemon's existing workdir result.json reader picks it up.
fn write_cdp_blocked_result(workdir: &Path, helper_diagnostic: &str) -> std::io::Result<()> {
    let blocked = serde_json::json!({
        // Every status field used across the 16 CDP-requiring skills. Downstream
        // skill-specific readers only consume their own field; extras are ignored.
        "status": "blocked",
        "freight_status": "blocked",
        "sizing_status": "blocked",
        "search_status": "blocked",
        "taric_status": "blocked",
        "classification_status": "blocked",
        "ingestion_status": "blocked",
        "health_status": "blocked",
        "refresh_status": "blocked",
        // Common shape every skill returns:
        "cdp_state": "unreachable",
        "captcha_intercepted": false,
        "warnings": [],
        "blockers": [{
            "code": "CDP_CONNECT_FAILED",
            "message": format!(
                "Daemon pre-flight gate: bin/ensure-cdp-9222 could not establish a reachable CDP endpoint on port {}. Helper diagnostic: {}",
                9222, helper_diagnostic
            ),
            "severity": "blocked"
        }],
        // Audit trail field — distinguishes daemon-written blockers from skill-written ones.
        "blocker_origin": "daemon_pre_flight_ensure_cdp_9222"
    });
    std::fs::write(
        workdir.join("result.json"),
        serde_json::to_string_pretty(&blocked)?,
    )
}
```

**Wire it in at the spawn site.** The audit identified the spawn calls at `crates/daemon/src/main.rs:6560, 11875, 15827` (each one is `Command::new(&launch_spec.program).current_dir(&launch_spec.cwd)...`). Just before each `Command::new(...)`, after `launch_spec` is built but before spawn:

```rust
if skill.metadata.requires_cdp {
    let cdp_port = skill.metadata.cdp_default_port.unwrap_or(9222);
    if let Err(diag) = ensure_cdp_9222(cdp_port) {
        // Pre-flight gate failed. Write a blocked result.json to the workdir
        // and skip spawning the skill subprocess entirely. The daemon's normal
        // post-run reader will pick up the blocker and surface it to the operator
        // exactly as if the skill itself had returned a CDP_CONNECT_FAILED.
        write_cdp_blocked_result(&launch_spec.cwd, &diag).map_err(|e| {
            anyhow::anyhow!("failed to write CDP blocker to {}: {}", launch_spec.cwd.display(), e)
        })?;
        // Return without spawning. Caller's existing logic that reads
        // `launch_spec.cwd.join("result.json")` (e.g., main.rs:15611, 15871) will
        // treat this exactly like a skill-emitted blocked result.
        return Ok(/* whatever the spawn site's success return is */);
    }
}
```

(The exact return value depends on the spawn site's signature — three call sites have three slightly different signatures. The agent doing the work should consult each and return the equivalent of "skill ran, produced a blocked result.json".)

**Update the existing Alibaba pre-flight at `main.rs:8064-8066`** to also use `ensure_cdp_9222(...)`. The existing logic uses `cdp_port_reachable(9222)` for a read-only check at decision time. The decision-time use is fine to keep. The PRE-INVOCATION gate should additionally call `ensure_cdp_9222(...)` so the helper tries to launch Chrome before the decision logic concludes "CDP is down, block this row":

```rust
// BEFORE (read-only check):
let cdp_unreachable_before_sizing = row.source_name == "alibaba-search"
    && row.source_url.is_some()
    && !cdp_port_reachable(9222);

// AFTER (helper-gated):
let cdp_unreachable_before_sizing = row.source_name == "alibaba-search"
    && row.source_url.is_some()
    && ensure_cdp_9222(9222).is_err();
```

**Update tests** in `crates/daemon/tests/agent_contract_session.rs` to verify the pre-flight hook fires. To allow mocking without launching real Chrome:

- The helper-resolution function honours `CDP_ENSURE_HELPER_PATH` env override. Tests set this env var to a mock script that exits 0 (CDP "up") or non-zero (CDP "down"), e.g.:

```rust
#[test]
fn ut_acs_900_cdp_preflight_blocks_when_helper_fails() {
    let mock_helper = create_mock_helper_that_exits_1();
    std::env::set_var("CDP_ENSURE_HELPER_PATH", &mock_helper);
    // ... build a launch_spec for a requires_cdp skill, call the spawn site,
    // assert workdir result.json contains CDP_CONNECT_FAILED, assert no
    // subprocess was actually spawned.
}
```

### Step 5 — Update AGENTS.md (~15 min)

AGENTS.md currently has **737 lines total**; the "Browser Testing with agent-browser" section runs lines 727-737 and ends at EOF. **Append the new section to the end of the file (immediately after line 737).** Use this content:

```markdown
## CDP Browser Prerequisite (port 9222)

Skills that drive a browser via Chrome DevTools Protocol declare `requires_cdp: true` in their YAML frontmatter. The current set is enumerated in `crates/daemon/tests/skill_lint.rs::CDP_BROWSER_SKILLS`.

**Auto-launch:** The daemon calls `bin/ensure-cdp-9222` before invoking any CDP-requiring skill. If Chrome is not already running on port 9222, the helper launches it with the operator's persistent profile at `/Users/alinaaitova/.chrome-automation`. Skills also call the helper from their "Mandatory CDP Discipline" section as defence in depth (in case Chrome crashes mid-execution).

The helper is idempotent and fast on the happy path (~1s when CDP is up).

**Manual launch (rare — first-time machine setup or non-default flags):**
```bash
'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' \
  --remote-debugging-port=9222 \
  '--remote-allow-origins=*' \
  --user-data-dir=/Users/alinaaitova/.chrome-automation \
  --profile-directory=Default --no-first-run --no-default-browser-check &
```

**Health check:**
```bash
curl -sfS http://localhost:9222/json/version | jq .
```

**Helper environment overrides** (rarely needed):
| Var | Default | Purpose |
|---|---|---|
| `CDP_PORT` / `AGENT_BROWSER_CDP_PORT` | `9222` | Bind a different port (e.g., for an isolated test browser) |
| `CDP_USER_DATA_DIR` | `$HOME/.chrome-automation` | Use a different Chrome profile |
| `CDP_LAUNCH_TIMEOUT_SECONDS` | `15` | How long to wait for CDP after launch before giving up |

**Sites that require manual sign-in (helper auto-launch does NOT log you in):**
- ProContract (procurement-ingestion/procontract-*) — sign in once via the persistent Chrome browser UI; the session cookie persists in the profile.
- Alibaba / 1688 (auth-refresh/* skills exist specifically to prompt manual re-signin when cookies expire).
- Freightos (the L5 operator-built freightos-freight-estimate skill).

Auto-launch brings up the SAME persistent profile, so any prior signin state is preserved. If cookies have expired, the relevant skill will surface a session-expired blocker — re-signin manually in the browser, then re-run.
```

### Step 6 — Update `docs/pipeline/stage-11-cdp-live-demos-continuation-handoff.md` (~5 min)

This is the canonical doc-source of the manual launch command. Add a one-line note at the top of the launch section pointing to the helper:

```markdown
> **2026-05-17 update:** Routine CDP launches now go through `bin/ensure-cdp-9222` (idempotent helper). The manual command below remains the canonical reference for the flags + profile path; the helper just automates invoking it when CDP is not already up.
```

### Step 7 — Add helper smoke test (~30 min)

Create `crates/daemon/tests/cdp_ensure_helper.rs`:

```rust
//! ut_ehlpr_001..099 — smoke tests for bin/ensure-cdp-9222.
//!
//! These tests assert the helper's structural properties (exists, executable,
//! handles unsupported OS, honours env overrides). They do NOT launch real
//! Chrome — that's exercised by the integration tests that already run the
//! shipment-sizing flow against a live CDP browser.

use std::path::PathBuf;
use std::process::Command;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn helper_path() -> PathBuf {
    project_root().join("bin").join("ensure-cdp-9222")
}

#[test]
fn ut_ehlpr_001_helper_exists() {
    assert!(
        helper_path().exists(),
        "ensure-cdp-9222 helper missing at {}",
        helper_path().display()
    );
}

#[test]
fn ut_ehlpr_002_helper_is_executable() {
    use std::os::unix::fs::PermissionsExt;
    let metadata = std::fs::metadata(helper_path()).unwrap();
    let mode = metadata.permissions().mode();
    assert!(
        mode & 0o111 != 0,
        "ensure-cdp-9222 helper is not executable (mode = {:o})",
        mode
    );
}

#[test]
fn ut_ehlpr_003_helper_short_circuits_unsupported_os() {
    // Force the OSTYPE env override; helper should exit 2.
    let output = Command::new(helper_path())
        .env("OSTYPE", "linux-gnu")
        .env("CDP_PORT", "65530") // unused port so the curl pre-check fails fast
        .output()
        .unwrap();
    // Helper exits 2 for unsupported OS — see exit code reference in helper header.
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit 2 for unsupported OS; got {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn ut_ehlpr_004_helper_emits_diagnostic_json_on_failure() {
    let output = Command::new(helper_path())
        .env("OSTYPE", "linux-gnu")
        .env("CDP_PORT", "65531")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Diagnostic JSON should have helper, status, port, message, ts keys.
    assert!(stderr.contains(r#""helper":"ensure-cdp-9222""#), "missing helper key: {}", stderr);
    assert!(stderr.contains(r#""status":"unsupported_os""#), "missing status key: {}", stderr);
}

// ut_ehlpr_005 — when CDP IS up, helper exits 0 in <5s. This requires a real
// CDP browser running — gated by `#[ignore]` so CI doesn't fail without one.
// Run locally: `cargo test --test cdp_ensure_helper -- --ignored`.
// The 5s budget allows for slow machines / loaded CI runners; fast path is
// typically <1s when CDP is already up.
#[test]
#[ignore = "requires live CDP browser on localhost:9222"]
fn ut_ehlpr_005_helper_fast_path_when_cdp_up() {
    use std::time::Instant;
    let start = Instant::now();
    let output = Command::new(helper_path()).output().unwrap();
    let elapsed = start.elapsed();
    assert!(output.status.success(), "helper failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(elapsed.as_secs() < 5, "helper too slow on fast path: {:?}", elapsed);
}
```

Add the new test file to `Cargo.toml` if needed (most projects auto-discover `tests/*.rs`).

### Step 8 — Update `crates/daemon/tests/skill_lint.rs` (~10 min)

The lint already asserts every CDP-requiring skill declares the right frontmatter. Extend it to also assert each skill's body invokes the helper:

```rust
#[test]
fn ut_skl_302_cdp_skills_invoke_ensure_helper() {
    for skill_id in CDP_BROWSER_SKILLS.iter() {
        let path = skills_root().join(skill_id).join("SKILL.md");
        let body = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("could not read {}: {}", path.display(), e));
        assert!(
            body.contains("bin/ensure-cdp-9222"),
            "{} declares requires_cdp:true but does not invoke bin/ensure-cdp-9222 in its body",
            skill_id
        );
    }
}
```

This test BLOCKS the build if anyone adds a new CDP-requiring skill in the future without wiring the helper. Self-policing.

---

## Verification checklist (the operator runs this end-to-end)

After all 8 steps are done:

### A. Helper itself

- [ ] `bin/ensure-cdp-9222` exists, executable bit set.
- [ ] All 4 smoke commands in Step 1 pass.
- [ ] `crates/daemon/tests/cdp_ensure_helper.rs` smoke tests pass: `cargo test --test cdp_ensure_helper`.

### B. Skill coverage

- [ ] All 16 production CDP-requiring SKILL.md files invoke `ensure-cdp-9222`. Run (uses `find` because bash `**` requires `shopt -s globstar` and silently matches nothing without it):
  ```bash
  for f in $(find /Users/alinaaitova/Desktop/Testproject1/skills -name SKILL.md -type f \
    | xargs grep -l "requires_cdp:.*true"); do
    if ! grep -q "ensure-cdp-9222" "$f"; then echo "MISSING: $f"; fi
  done
  # Expected: zero output. Also verify count:
  find /Users/alinaaitova/Desktop/Testproject1/skills -name SKILL.md -type f \
    | xargs grep -l "requires_cdp:.*true" | wc -l    # expected: 16
  ```
- [ ] Both in-progress L5 operator-built handoffs (Freightos + Taric) reference the helper in their SKILL.md templates AND in their Phase C.0.a setup.
- [ ] `cargo test --test skill_lint -- ut_skl_302_cdp_skills_invoke_ensure_helper` passes (new lint).

### C. Daemon integration

- [ ] `ensure_cdp_9222(...)` function exists in `crates/daemon/src/main.rs`.
- [ ] At the agent-spawn site, when `skill.metadata.requires_cdp == true`, `ensure_cdp_9222(...)` is called first.
- [ ] On failure, the daemon emits a `CDP_CONNECT_FAILED` blocker instead of invoking the skill.
- [ ] The existing Alibaba `cdp_port_reachable(9222)` pre-flight at `main.rs:8064` is updated to use the new helper.
- [ ] `cargo test -p daemon --bin daemon` passes (no regressions in the agent-spawn paths).
- [ ] Full Rust integration test suite passes: `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done`.

### D. Documentation

- [ ] AGENTS.md has the new "CDP Browser Prerequisite (port 9222)" section.
- [ ] `docs/pipeline/stage-11-cdp-live-demos-continuation-handoff.md` has the 1-line update pointing to the helper.

### E. End-to-end behaviour

Kill Chrome to prove auto-launch works in real use:

```bash
# 1. Verify Chrome is up (or up it via helper)
bin/ensure-cdp-9222 && echo "CDP up"

# 2. Stop Chrome
pkill -f "remote-debugging-port=9222"
sleep 2
curl -sfS -m 1 http://localhost:9222/json/version || echo "CDP confirmed down"

# 3. Invoke a CDP-requiring skill via the daemon (e.g., trigger an Alibaba shipment-sizing job).
#    Expectation: the daemon's pre-flight calls bin/ensure-cdp-9222, which launches Chrome,
#    skill runs successfully. The previous behaviour (skill returns CDP_CONNECT_FAILED) is gone.

# 4. Watch the daemon logs for the helper's diagnostic JSON on stderr — should see
#    {"helper":"ensure-cdp-9222","status":"launched",...}.
```

- [ ] End-to-end test passes: triggering a skill after killing Chrome auto-launches Chrome and the skill completes.

### F. Test evidence (commit a TEST_REPORT)

Write a brief test report to `docs/testing/2026-05-17-cdp-9222-auto-launch-test-report.md`:

```markdown
# CDP-9222 Auto-Launch — Test Report

**Date:** YYYY-MM-DD
**Helper version:** bin/ensure-cdp-9222 SHA <git sha>
**Tested by:** <operator or agent>

## Verdict
**STATUS: READY FOR PRODUCTION** (or BLOCKED if any check failed)

## Coverage
- 16/16 production SKILL.md files invoke the helper
- 2/2 in-progress L5 operator-built handoffs updated
- Daemon pre-flight calls helper at agent-spawn site
- AGENTS.md updated
- 5 new smoke tests in crates/daemon/tests/cdp_ensure_helper.rs (4 passing, 1 ignored-by-default)
- skill_lint ut_skl_302 passing (CDP skills must invoke helper)

## End-to-end verification
- Killed Chrome → invoked Alibaba shipment-sizing → daemon's pre-flight auto-launched → skill completed successfully ✓
- Diagnostic JSON in daemon logs as expected ✓

## Quirks observed
- <e.g., "Chrome takes ~4s to come up on first launch with a large profile">
```

---

## Rollback plan (if something goes catastrophically wrong)

The helper is additive. To roll back:

1. **Revert daemon changes:** `git revert <sha>` for the `ensure_cdp_9222` integration commit. Daemon goes back to assuming operator has pre-launched.
2. **Revert SKILL.md edits:** each skill's Mandatory CDP Discipline goes back to raw curl. Restore with `git restore $(find skills -name SKILL.md -type f)` (avoid `git restore skills/**/SKILL.md` — bash `**` glob is off by default and would silently restore nothing).
3. **Leave the helper script in place:** it's harmless when not invoked. Operators can still run `bin/ensure-cdp-9222` manually.
4. **Leave AGENTS.md edits:** documentation only, no code impact.

Total rollback time: < 10 min.

---

## Notes for the next agent

If you (operator) delegate this to a coding agent:

- The agent MUST execute the steps in order. Skipping Step 1 (helper creation) makes Steps 2-4 meaningless.
- The agent MUST verify the 16-skill list before editing — run the grep at the top of Step 2.
- The agent MUST NOT invent new `requires_cdp` skills during this work — scope is auto-launch only, not adding capabilities.
- The agent MUST run the verification checklist Section A-D before claiming complete. Section E (end-to-end) requires a live operator-driven trigger and is optional for the agent (operator confirms post-build).
- For the daemon integration in Step 4: if the agent struggles to find the exact agent-spawn site in `main.rs`, fall back to placing the call in the most-upstream code path that reads `metadata.requires_cdp` (currently around line 16067 — the env-var-setting site). It's better to gate too early than not at all.

---

## Outcome when this handoff is complete

- The operator (and future operators) NEVER have to manually launch Chrome again for routine skill work.
- Skill failures with reason `CDP_CONNECT_FAILED` become a true signal (the helper genuinely couldn't bring CDP up — e.g., Chrome binary missing) rather than the noise of "operator forgot to open browser".
- New CDP-requiring skills added in the future are required by the lint to invoke the helper — no possibility of regression.
- The L5 freightos + taric build sessions inherit auto-launch from day one.
