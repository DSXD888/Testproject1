# Handoff — Area H: Daemon-Driven End-to-End CDP Demo (Option A + Option C) (2026-05-18)

> **Operator (Alina, 2026-05-18):** Both 2026-05-18 handoffs completed — Zac Farrell + Elias contributions are reverted, the CDP stack (helper + 16 skills + daemon integration + Session Recovery on 8 production skills + 6 auth-signin sub-skills) is restored from stash, daemon builds clean, all three lint tests pass. The remaining structural verification is green per `docs/testing/2026-05-18-l5-readiness-validation-report.md`. **But every test so far has been static or isolated — none of them has watched the daemon actually orchestrate a CDP-requiring skill end-to-end.** That's Area H. I want you to do **Option A** (lightweight daemon-driven E2E with `source-health/fts`) to prove the chain works, then **Option C** (Alibaba Session Recovery under a forced logout) to prove the recovery path works. Together these are the headline empirical proof that L5 can invoke CDP skills safely under autonomous cron operation.

> **Mission:** Drive a real daemon process to run two CDP-requiring skills end-to-end against live Chrome, observe the helper auto-launch, observe results landing in SQLite, observe Session Recovery handling a deliberately expired session. Iterate fix-validate-test until both options pass. Then write a TEST_REPORT.md and commit + push.

> **Strict NON-goals:**
> - ❌ Do NOT start L5 step 01 or any L5 pipeline work.
> - ❌ Do NOT modify any of the CDP skills' production logic except to fix bugs Area H discovers (and only via Edit tool — never sed).
> - ❌ Do NOT touch `bin/ensure-cdp-9222` unless §7 triage finds an actual bug in it (the helper is already smoke-tested green per the L5-readiness declaration).
> - ❌ Do NOT bypass the AgentMail guard hooks (`--no-verify`, `AGENT_MAIL_GUARD_MODE=warn`, etc.) — if the hooks block, FIX the underlying cause.
> - ❌ Do NOT advance past a failing test. The validate-test-fix loop in each option is the whole point — iterate until ✅.

> **Authorized operations:** read/write files under `docs/testing/area-h-2026-05-18/`, run `cargo build`/`cargo run -p daemon`, run `bin/ensure-cdp-9222`, query `state/state.db` via `sqlite3`, kill local Chrome processes, edit individual skill SKILL.md files via the Edit tool if §7 triage reveals a bug, commit + push the TEST_REPORT.

---

## 0. Required reading (before any tool call)

1. **This handoff** (§0–§9 in full).
2. `/Users/alinaaitova/Desktop/Testproject1/AGENTS.md` — especially "Compiler Checks (CRITICAL)", "Stage Validation Test Gate", "No Script-Based Changes", "Irreversible Git" sections.
3. `docs/handoff-2026-05-17-cdp-stack-validation-test.md` — the parent test handoff. **Area H lives at lines ~742–820** of that file. Read it for the specific sub-areas H.1/H.2/H.3. Your work here is a focused execution of H.1 (= Option A) and H.2 (= Option C).
4. `docs/testing/2026-05-18-l5-readiness-validation-report.md` — confirms what's been verified structurally, and the "DEFERRED" lines you're closing out (Areas D.3–D.5, E, H).
5. `bin/ensure-cdp-9222` — read it (~135 lines) so you understand the helper's contract: env overrides, diagnostic JSON, exit codes 0/1/2/3.
6. `skills/source-health/fts/SKILL.md` — the Option A target. Lightest CDP skill, no auth, no Session Recovery.
7. `skills/marketplace-search/alibaba/SKILL.md` — the Option C target. Has the full Session Recovery block (~133 lines of additions from the stash pop). Find the session-expired detection and the auth-signin invocation.
8. `skills/auth-signin/alibaba/SKILL.md` + `skills/auth-signin/google-account-chooser/SKILL.md` — the recovery skills Option C will trigger.

Estimated reading: 30 minutes. Worth it — you'll be debugging the chain live, and knowing where the failure can be in advance halves diagnosis time.

---

## 1. Verified context (already gathered for you)

### 1.1 What's on disk that you can rely on

| Artifact | Path | State |
|---|---|---|
| Helper script | `bin/ensure-cdp-9222` | Executable, 135 lines, smoke-tested (Areas A.1–A.4 PASS per L5-readiness report) |
| Daemon integration | `crates/daemon/src/main.rs` | `ensure_cdp_9222()` wired at lines 8070, 12348, 12370; helper-path resolver at 12302 |
| 16 CDP skills | `skills/{source-health,auth-refresh,marketplace-search,procurement-ingestion,landed-cost}/*` | All declare `requires_cdp: true` and invoke the helper |
| 8 production skills with Session Recovery | `skills/marketplace-search/{alibaba,ebay-uk}`, `skills/source-health/alibaba`, `skills/landed-cost/{searates-freight-estimate,shipment-sizing}`, `skills/procurement-ingestion/procontract-{discover,enrich-tender,register-interest}` | All have `auth-signin/` invocation in their bodies |
| 6 auth-signin sub-skills | `skills/auth-signin/{alibaba,ebay,freightos,google-account-chooser,procontract,searates}` | All present, real selectors (no TBD placeholders) |
| Daemon test gate | `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p daemon --bin daemon`, per-integration test | All ✅ as of last L5-readiness check |
| AGENTS.md sections | "CDP Browser Prerequisite (port 9222)" line 741; "Session Recovery for Authenticated CDP Sites" line 784 | Present |

### 1.2 Operator credentials (for Option C — Alibaba sign-in via Google OAuth)

- Google account to select in chooser: **`dieter.x.s.danicek@gmail.com`** (display name: Dieter Danicek)
- Persistent Chrome profile: `~/.chrome-automation` — already has this account signed in
- ProContract password autofill: `sales@sorelon.com` (relevant only if Option C extends to ProContract; Touch ID was disabled at `chrome://settings/passwords` on 2026-05-17 per audit doc, so autofill is silent)

### 1.3 Database

- SQLite at `state/state.db` (operational authority per the project's authority-matrix architecture)
- Key table: `agent_session_logs` (verify exact name in `migrations/*.sql` — older migrations may use a different name)
- Result lands in `result_json` column; `status` column transitions running → success | failure | timeout

### 1.4 AgentMail must be green before this handoff starts

The pre-commit + pre-push hooks read MCP. Before any work:

```
mcp__mcp-agent-mail__health_check
# Expected: status: "ok", health_level: "green"
# If "degraded_read_only" / "red": operator must run `am doctor reconstruct --yes`; do NOT proceed
```

If AgentMail is degraded, STOP and surface to operator. Do NOT bypass the guard hooks.

---

## 2. Goal & success criteria

### 2.1 Goal

Two empirical demonstrations against live Chrome, both **observed end-to-end** with evidence captured:

- **Option A (H.1 from parent handoff):** Daemon kicks off a cron tick for `source-health/fts`. The daemon's pre-flight gate invokes `bin/ensure-cdp-9222`. Chrome opens on port 9222. The skill runs in agent-browser, navigates to `find-a-tender.service.gov.uk`, returns a `source_status` result. Daemon writes the result to `agent_session_logs`.
- **Option C (H.2 from parent handoff):** Operator manually logs out of Alibaba mid-run. Daemon triggers `marketplace-search/alibaba`. The skill detects expired session, invokes `skills/auth-signin/alibaba`, which clicks "Sign In" → "Continue with Google" → selects Dieter Danicek in the chooser → returns to alibaba.com logged in. Original skill resumes, completes search, returns listing data.

### 2.2 Success criteria (ALL must be ✅ before §6 sign-off)

**Option A:**
- [ ] Daemon started successfully (no panics, no startup errors in stderr)
- [ ] Daemon log shows `ensure_cdp_9222` invocation BEFORE skill spawn (line ordering matters)
- [ ] Chrome window visibly opens on operator's desktop (`pgrep -fl 9222` after launch returns a match)
- [ ] `curl -sf http://localhost:9222/json/version` returns JSON
- [ ] Daemon log shows `spawning skill: source-health/fts` (or equivalent)
- [ ] Skill's agent-browser session navigates to find-a-tender successfully
- [ ] SQLite row in `agent_session_logs` with `status='success'` and populated `result_json`
- [ ] Daemon log shows graceful shutdown (no orphaned Chrome / agent-browser processes after the run)

**Option C:**
- [ ] All Option A criteria satisfied first (Option A is a prerequisite to Option C)
- [ ] Operator manually logged out of Alibaba (visible: alibaba.com shows "Sign In" button instead of account avatar)
- [ ] Daemon triggers `marketplace-search/alibaba` for a real search query
- [ ] Skill detects expired session (skill log shows a session-expired detection signal: SESSION_REFRESH_TRIGGERED or similar)
- [ ] `skills/auth-signin/alibaba/` is invoked (skill log shows the subskill being dispatched)
- [ ] "Continue with Google" button clicked
- [ ] Google account chooser appears, Dieter Danicek selected (`dieter.x.s.danicek@gmail.com`)
- [ ] Return to alibaba.com fully logged in (avatar visible, no "Sign In" button)
- [ ] Original `marketplace-search/alibaba` resumes WITHOUT requiring a separate daemon retry — Session Recovery is inline
- [ ] SQLite row with `status='success'` AND `result_json` containing real listing data (≥1 product)

**Final artifact:**
- [ ] `docs/testing/area-h-2026-05-18/TEST_REPORT.md` written with verdict per §6 template
- [ ] Single commit `area-h: daemon-driven E2E demo (Options A+C) — verdict <READY|BLOCKED>`
- [ ] Pushed to `origin/main`

---

## 3. Pre-flight (run all; stop if any fails)

```bash
# 3.1 You're on main, in sync with origin
cd /Users/alinaaitova/Desktop/Testproject1
git status --short --branch | head -3                    # expect: ## main...origin/main (no ahead/behind)
git fetch origin
git rev-list HEAD..origin/main --count | grep -q '^0$' || { echo "STOP: behind origin/main"; exit 1; }

# 3.2 AgentMail is green (use MCP tool, not shell)
# Call: mcp__mcp-agent-mail__health_check
# Expected: { "status":"ok", "health_level":"green", ... }
# If red: STOP, ask operator to run `am doctor reconstruct --yes`

# 3.3 Register an agent identity (use MCP tool macro_start_session)
# Call: mcp__mcp-agent-mail__macro_start_session with program=claude-code, model=<your-model-id>,
#       task_description="Area H daemon-driven E2E demo (Options A + C)"
# Record the auto-generated agent name to /tmp/area-h-agent-name.txt
# Export it: export AGENT_NAME=$(cat /tmp/area-h-agent-name.txt)

# 3.4 Daemon builds clean
cargo fmt --check 2>&1 | tail -3
cargo check --all-targets 2>&1 | tail -3
cargo clippy --all-targets -- -D warnings 2>&1 | tail -5
# All three must show no errors. If any fails: STOP — broken baseline, not an Area H bug.

# 3.5 Helper exists, executable, smoke-tested
[ -x bin/ensure-cdp-9222 ] || { echo "STOP: helper missing/non-exec"; exit 1; }
head -1 bin/ensure-cdp-9222 | grep -q "^#!/usr/bin/env bash" || { echo "STOP: wrong shebang"; exit 1; }

# 3.6 No Chrome running on 9222 (you want to see launch)
pkill -f "remote-debugging-port=9222" 2>/dev/null
sleep 2
curl -sf http://localhost:9222/json/version >/dev/null && { echo "STOP: Chrome respawned, investigate"; exit 1; } || echo "✅ Chrome clean"

# 3.7 No daemon running
pkill -f "target/.*daemon" 2>/dev/null
sleep 1
pgrep -fl "target/.*daemon" && { echo "STOP: stale daemon"; exit 1; } || echo "✅ no daemon running"

# 3.8 Evidence directory
mkdir -p docs/testing/area-h-2026-05-18
ls docs/testing/area-h-2026-05-18/                       # confirm exists
```

Record all 3.x outputs in `docs/testing/area-h-2026-05-18/pre-flight.txt`. Every line must be ✅ before §4.

---

## 4. Option A — `source-health/fts` daemon-driven E2E (~10–15 min)

### 4.1 Discover the daemon's one-shot invocation mode

The daemon supports skill-on-demand triggering somewhere; you need to find the mechanism. Three places to check, in order:

```bash
cargo run -p daemon -- --help 2>&1 | head -60          # look for --once, --trigger, --skill, --dry-run flags
```

If `--once --skill <name>` exists, use that. Otherwise:

```bash
grep -rE "fn main|--once|trigger_skill|run_once" crates/daemon/src/main.rs | head -10
# Look for: a CLI arg parser that accepts a skill-name parameter
```

If still no clean one-shot, the fallback is to modify the cron schedule for `source-health/fts` in SQLite so the next-run time is "now":

```bash
# 4.1.fallback — verify the schedule table name first
sqlite3 state/state.db ".schema" | grep -iE "schedule|cron|skill" | head -10
# Once you know the table, e.g. "skill_schedule":
sqlite3 state/state.db "UPDATE skill_schedule SET next_run_at = strftime('%s','now') WHERE skill_name = 'source-health/fts';"
sqlite3 state/state.db "SELECT skill_name, next_run_at FROM skill_schedule WHERE skill_name = 'source-health/fts';"
```

Record which mechanism you used in `evidence/4.1.daemon-trigger-mode.txt`.

### 4.2 Start the daemon with verbose logging

```bash
# Background-run with logs to file
RUST_LOG=daemon=debug,info \
  AGENT_NAME=$(cat /tmp/area-h-agent-name.txt) \
  cargo run -p daemon 2>&1 | tee docs/testing/area-h-2026-05-18/A.daemon.log &
DAEMON_PID=$!
echo "Daemon PID: $DAEMON_PID"
echo "$DAEMON_PID" > docs/testing/area-h-2026-05-18/A.daemon.pid
sleep 3                                                  # let it start
pgrep -fl "target/.*daemon" | head -3                    # confirm it's alive
```

### 4.3 Trigger the skill + watch the chain

If you used a one-shot CLI flag in §4.1: re-run the daemon with that flag and skip ahead. Otherwise the cron tick should fire within the cron interval (often 60s). Tail the log:

```bash
tail -f docs/testing/area-h-2026-05-18/A.daemon.log &
TAIL_PID=$!
# Watch for these checkpoints, in order:
#   (1) "ensure_cdp_9222: probing CDP on port 9222"          ← daemon's pre-flight gate fires
#   (2) "ensure_cdp_9222: launching Chrome"                  ← helper is invoked
#   (3) Chrome window opens (visible on desktop)
#   (4) "spawning skill: source-health/fts"                  ← daemon kicks off the skill
#   (5) "agent-browser: navigating to https://www.find-a-tender.service.gov.uk/..."
#   (6) "skill_result: success" OR "skill_result: failure: <reason>"
#   (7) "result written to agent_session_logs id=N"          ← daemon persists
# When you've seen (7), kill the tail:
kill $TAIL_PID 2>/dev/null
```

Record each checkpoint's actual log line (with timestamp) into `evidence/A.checkpoints.txt`.

### 4.4 Verify SQLite landed the result

```bash
# Verify the row exists with the expected fields
sqlite3 state/state.db <<EOF > docs/testing/area-h-2026-05-18/A.sqlite-row.txt
SELECT id, skill_name, status, started_at, completed_at,
       length(result_json) AS result_size,
       run_manifest_path
FROM agent_session_logs
WHERE skill_name = 'source-health/fts'
ORDER BY id DESC LIMIT 3;
EOF
cat docs/testing/area-h-2026-05-18/A.sqlite-row.txt

# Read the result_json
sqlite3 state/state.db "SELECT result_json FROM agent_session_logs WHERE skill_name='source-health/fts' ORDER BY id DESC LIMIT 1;" \
  | python3 -m json.tool > docs/testing/area-h-2026-05-18/A.result.json
cat docs/testing/area-h-2026-05-18/A.result.json
```

### 4.5 Validate–Test–Fix loop for Option A

**Iterate this loop until §2.2 Option A criteria are ALL ✅. Do NOT advance to §5 with even one ❌ remaining.**

```
LOOP:
  evaluate §2.2 Option A criteria
  if ALL ✅:
    write evidence/A.status.txt = "PASS at $(date -u +%FT%TZ)"
    break LOOP
  else:
    identify the FIRST ❌ criterion
    consult §7 (Triage guide) for that failure symptom
    apply the indicated fix (Edit tool — never sed)
    if the fix touched a skill SKILL.md or daemon code:
      re-run §3.4 (cargo fmt/check/clippy) — must stay green
    kill any leftover daemon/Chrome processes:
      pkill -f "target/.*daemon"; pkill -f "remote-debugging-port=9222"; sleep 2
    re-run §4.2 + §4.3 + §4.4
    goto LOOP
```

There is no iteration cap — keep going until green. Every iteration appends to `evidence/A.iterations.log` with: timestamp, failure observed, fix applied, files changed, re-run outcome.

If after **5 iterations** the same criterion still fails: STOP, write `evidence/A.STUCK.md` summarizing the loop, and surface to operator. Do not pretend to pass.

### 4.6 Cleanup before §5

```bash
# Kill daemon + Chrome cleanly
kill $(cat docs/testing/area-h-2026-05-18/A.daemon.pid) 2>/dev/null
pkill -f "remote-debugging-port=9222" 2>/dev/null
sleep 2
pgrep -fl "target/.*daemon|remote-debugging-port=9222"   # expect: empty
# Confirm clean state for Option C
echo "Option A complete at $(date -u +%FT%TZ)" >> docs/testing/area-h-2026-05-18/A.status.txt
```

---

## 5. Option C — `marketplace-search/alibaba` Session Recovery under forced logout (~30–45 min)

This is the harder, higher-signal test. Requires real operator interaction (manual logout of Alibaba) and watching the full Session Recovery chain fire.

### 5.1 Pre-Option-C state setup

```bash
# 5.1.a Restart Chrome — but this time we want it logged INTO Alibaba first
bin/ensure-cdp-9222
sleep 3
curl -sf http://localhost:9222/json/version | head -1    # confirm CDP up

# 5.1.b Operator action: open the persistent profile's Chrome (already running on 9222)
#       Navigate to https://www.alibaba.com — confirm you see the account avatar (logged in)
#       Take a screenshot to docs/testing/area-h-2026-05-18/C.0a.logged-in.png
echo "Operator: confirm logged-in state on alibaba.com, then proceed"
# (handoff agent: wait for operator confirmation before continuing)
```

**Operator checkpoint:** before proceeding, the operator visually confirms Alibaba shows them as logged in (avatar in top-right, not a "Sign In" button).

### 5.2 Forced logout (operator manual step)

```
Operator action (~30 sec):
1. In the CDP-controlled Chrome window:
   - Click the account avatar (top-right of alibaba.com)
   - Click "Sign Out" / "Log Out" in the dropdown
   - Confirm the page now shows "Sign In" instead of avatar
2. Take a screenshot to docs/testing/area-h-2026-05-18/C.0b.logged-out.png
3. Tell the handoff agent: "Logged out, proceed"
```

The handoff agent **MUST NOT proceed to §5.3 without operator confirmation of logged-out state.** No automated detection — visual confirmation only.

### 5.3 Trigger marketplace-search/alibaba

Same mechanism as §4.1 (one-shot CLI flag if available, else SQLite cron-time edit). The query input the skill expects is in its SKILL.md; pick a real product term that's likely to return results (e.g., "stainless steel bolts M8 carbon steel" or whatever the skill's canary inputs use — read `skills/marketplace-search/alibaba/SKILL.md` `## Inputs` section).

```bash
# Re-export agent name + start daemon
export AGENT_NAME=$(cat /tmp/area-h-agent-name.txt)
RUST_LOG=daemon=debug,info,skill=debug \
  cargo run -p daemon 2>&1 | tee docs/testing/area-h-2026-05-18/C.daemon.log &
DAEMON_PID=$!
echo "$DAEMON_PID" > docs/testing/area-h-2026-05-18/C.daemon.pid
sleep 3
```

### 5.4 Watch the Session Recovery chain (the headline observable)

Tail the log and watch for THIS specific sequence of events. Missing any one of them = ❌ for §2.2:

```
Expected log sequence (timestamps will vary):

(1) "ensure_cdp_9222: CDP already reachable, no-op"
    ← Chrome was already running from §5.1; helper exits 0 fast

(2) "spawning skill: marketplace-search/alibaba"
    ← daemon kicks off the production skill

(3) "agent-browser: navigating to https://www.alibaba.com/trade/search?..."
    ← skill begins normal flow

(4) Detection signal (one of these, depending on how the skill body detects logout):
    - "session-expired: detected 'Sign In' button on logged-in URL"
    - "SESSION_REFRESH_TRIGGERED"
    - "auth_state: expired"
    ← this is the moment the recovery path engages

(5) "invoking skill: auth-signin/alibaba"
    ← production skill delegates to recovery sub-skill

(6) auth-signin/alibaba log:
    - "agent-browser: clicking 'Sign In'"
    - "agent-browser: clicking 'Continue with Google'"
    - "agent-browser: chooser detected"

(7) auth-signin/google-account-chooser log:
    - "agent-browser: selecting account dieter.x.s.danicek@gmail.com"
    - "agent-browser: OAuth complete, returned to alibaba.com"

(8) production skill resumes:
    - "auth-signin successful, resuming original flow"
    - "agent-browser: navigating to https://www.alibaba.com/trade/search?..." (retry)

(9) "skill_result: success" with non-empty listing data

(10) "result written to agent_session_logs id=N"
```

For each event, record the actual timestamped log line to `evidence/C.sequence.txt`. **Missing or out-of-order events = failure; consult §7 triage.**

**Operator checkpoint at (7):** while the chooser is open, the operator should visually confirm:
- Two or more Google accounts are listed
- Dieter Danicek (`dieter.x.s.danicek@gmail.com`) is the one selected automatically
- No Touch ID / 2FA prompt appears (account is already signed in to Chrome profile)

Take a screenshot of the chooser to `docs/testing/area-h-2026-05-18/C.5.chooser-selection.png` if possible (operator action — agent can't screenshot the Chrome UI from within the daemon process).

### 5.5 Verify SQLite landed the recovery result

```bash
sqlite3 state/state.db <<EOF > docs/testing/area-h-2026-05-18/C.sqlite-row.txt
SELECT id, skill_name, status, started_at, completed_at,
       length(result_json) AS result_size,
       run_manifest_path
FROM agent_session_logs
WHERE skill_name = 'marketplace-search/alibaba'
ORDER BY id DESC LIMIT 3;
EOF

# Decode the result — must contain real listings, not just metadata
sqlite3 state/state.db "SELECT result_json FROM agent_session_logs WHERE skill_name='marketplace-search/alibaba' ORDER BY id DESC LIMIT 1;" \
  | python3 -m json.tool > docs/testing/area-h-2026-05-18/C.result.json

# Sanity-check the result has listings
python3 -c "
import json
with open('docs/testing/area-h-2026-05-18/C.result.json') as f:
    r = json.load(f)
listings = r.get('listings') or r.get('items') or r.get('results') or []
print(f'Listings count: {len(listings)}')
print(f'First listing keys: {list(listings[0].keys()) if listings else \"(none)\"}')
assert len(listings) > 0, 'No listings returned — Session Recovery resumed but data extraction failed'
print('✅ Listings present')
"
```

### 5.6 Validate–Test–Fix loop for Option C

**Same loop semantics as §4.5 but for Option C criteria.** Iterate until ALL ✅.

```
LOOP:
  evaluate §2.2 Option C criteria
  if ALL ✅:
    write evidence/C.status.txt = "PASS at $(date -u +%FT%TZ)"
    break LOOP
  else:
    identify FIRST ❌
    consult §7 triage for that symptom
    apply fix (Edit tool only)
    if fix touched any skill SKILL.md or daemon code:
      re-run §3.4 (must stay green)
    cleanup: kill daemon + Chrome, sleep 2, re-verify clean state
    repeat §5.1 (operator re-confirms logged-in)
    repeat §5.2 (operator re-logs-out)
    re-run §5.3 + §5.4 + §5.5
    goto LOOP
```

**Iteration budget for Option C: 4 iterations max** before STOP and `evidence/C.STUCK.md`. (Lower than Option A because each iteration requires operator manual logout — costly. If 4 attempts didn't reveal the fix, the bug is structural and needs operator design input.)

### 5.7 Cleanup

```bash
kill $(cat docs/testing/area-h-2026-05-18/C.daemon.pid) 2>/dev/null
pkill -f "remote-debugging-port=9222" 2>/dev/null
sleep 2
pgrep -fl "target/.*daemon|remote-debugging-port=9222"   # expect: empty
echo "Option C complete at $(date -u +%FT%TZ)" >> docs/testing/area-h-2026-05-18/C.status.txt
```

---

## 6. TEST_REPORT.md template

Write `docs/testing/area-h-2026-05-18/TEST_REPORT.md` with this exact structure (fill in the bracketed values):

```markdown
# Area H — Daemon-Driven E2E Demo TEST_REPORT

**Date:** 2026-05-18
**Operator:** Alina Aitova
**Handoff agent:** <your-model-id>
**Agent identity:** [agent-name from §3.3]
**Pre-flight:** ✅ all 8 checks green (see pre-flight.txt)

---

## Option A — source-health/fts daemon-driven E2E

**Verdict:** [READY | BLOCKED]

| Sub-criterion | Result | Evidence |
|---|---|---|
| Daemon started without panics | ✅ / ❌ | A.daemon.log |
| ensure_cdp_9222 invoked before skill spawn | ✅ / ❌ | A.checkpoints.txt:line N |
| Chrome opened visibly | ✅ / ❌ | operator-confirmed |
| Curl /json/version returned JSON | ✅ / ❌ | A.checkpoints.txt |
| skill_lint shows skill running | ✅ / ❌ | A.daemon.log |
| Agent-browser navigated to find-a-tender | ✅ / ❌ | A.daemon.log |
| Result landed in agent_session_logs | ✅ / ❌ | A.sqlite-row.txt |
| Result JSON is well-formed | ✅ / ❌ | A.result.json |
| No orphaned processes | ✅ / ❌ | post-cleanup pgrep |

**Iterations to PASS:** [N]
**Fixes applied during iteration (if any):**
- [file path]: [one-line description of the fix and why]
- ...

**Total duration:** [start-time] → [end-time] ([X minutes])

---

## Option C — marketplace-search/alibaba Session Recovery

**Verdict:** [READY | BLOCKED]

| Sub-criterion | Result | Evidence |
|---|---|---|
| Pre-Option-C: Alibaba logged in | ✅ / ❌ | C.0a.logged-in.png |
| Operator manual logout completed | ✅ / ❌ | C.0b.logged-out.png |
| Daemon triggered marketplace-search/alibaba | ✅ / ❌ | C.daemon.log |
| Session-expired detection fired | ✅ / ❌ | C.sequence.txt event (4) |
| auth-signin/alibaba invoked | ✅ / ❌ | C.sequence.txt event (5) |
| "Continue with Google" clicked | ✅ / ❌ | C.sequence.txt event (6) |
| Dieter Danicek auto-selected in chooser | ✅ / ❌ | operator-confirmed, C.5.chooser-selection.png if available |
| Returned to alibaba.com logged in | ✅ / ❌ | C.sequence.txt event (7) |
| Production skill resumed inline (no daemon retry) | ✅ / ❌ | C.sequence.txt event (8) — single skill_id throughout |
| Listing data returned with ≥1 product | ✅ / ❌ | C.result.json |
| Result landed in agent_session_logs | ✅ / ❌ | C.sqlite-row.txt |

**Iterations to PASS:** [N] (max 4)
**Fixes applied:**
- ...

**Total duration:** [X minutes]

---

## Discoveries / quirks / selector drift noted

[Free-form: anything you learned about Alibaba's UI, eBay's chooser, the daemon's behaviour, that future agents would benefit from knowing. Particularly: which selectors held up, which drifted, any timing-sensitive moments, any edge cases the production skills should be hardened against.]

---

## Overall verdict

**[READY FOR L5 STEP 01 | BLOCKED — see §X]**

If READY: Operator may run `docs/pipeline/l5-expanded-commercial-intelligence/01-validate-spec.md` immediately.

If BLOCKED: the blocking criteria are [list ❌ criteria]. Operator action required: [specific recommendation].

---

## Commit

This report is committed as: `area-h: daemon-driven E2E demo (Options A+C) — verdict <X>`
SHA: [<sha after commit>]
Pushed to origin/main: [✅ at <time> | ❌ <reason>]
```

---

## 7. Triage guide — fix recipes by failure symptom

For each ❌ criterion you observe, find the matching row and apply the indicated fix. **All file edits must use the Edit tool (never sed) per AGENTS.md.**

### 7.A — Daemon won't start

| Symptom | Likely cause | Fix |
|---|---|---|
| `panic` in stderr | Code error introduced by stash pop | `cargo check --all-targets`; read the panic; if it references a function that doesn't exist, Handoff 1's audit doc has a `## Build-required Zack contributions` section — that file may need to be re-restored |
| "Address already in use" on daemon port | Stale daemon process | `pkill -f "target/.*daemon"`; if you don't know the port, search `main.rs` for `bind(` |
| AgentMail rejection | health check returned red | STOP — operator runs `am doctor reconstruct --yes` |
| Hooks block startup (commit-msg or similar) | Missing AGENT_NAME env var | Re-run §3.3 to register identity, re-export AGENT_NAME |

### 7.B — Chrome doesn't auto-launch

| Symptom | Likely cause | Fix |
|---|---|---|
| Daemon log has no "ensure_cdp_9222" line at all | Daemon doesn't actually call the helper | Read `crates/daemon/src/main.rs` lines 8070, 12348, 12370 — does `ensure_cdp_9222()` invocation precede skill spawn? If lines moved, find new line numbers via `grep -n "ensure_cdp_9222" crates/daemon/src/main.rs` |
| "ensure_cdp_9222: probing" appears but no "launching" | Helper found Chrome already up | Verify §3.6 actually killed Chrome (`pgrep -fl 9222`) and re-run from §4.2 |
| "ensure_cdp_9222: launching" but Chrome doesn't open | Helper exits non-zero / Chrome binary path wrong | Run `bin/ensure-cdp-9222` standalone, read its stderr JSON; verify `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome` exists |
| Chrome opens but on a different port | `CDP_PORT` env var leak | `env \| grep CDP_PORT`; unset stale env |

### 7.C — Skill errors with CDP_UNAVAILABLE

| Symptom | Likely cause | Fix |
|---|---|---|
| Skill log: "agent-browser: CDP not reachable" | Helper succeeded but skill probes wrong port | Check skill SKILL.md frontmatter `cdp_default_port: 9222` |
| Skill log: "DEVTOOLS_REMOTE_TARGETS_EMPTY" | Chrome up but no targets (rare) | Restart Chrome via helper; check `bin/ensure-cdp-9222` polled `/json/version` before returning |
| Skill log: "navigation timeout" | Site is slow / blocked | Check that you can `curl` the target URL from the same machine; if upstream is genuinely down, mark as `DEFERRED — upstream` not blocker |

### 7.D — Skill runs but no SQLite row

| Symptom | Likely cause | Fix |
|---|---|---|
| Daemon log shows "skill_result: success" but `agent_session_logs` empty | Result-writing path broken | `grep -nE "insert.*agent_session_logs\|write_result" crates/daemon/src/main.rs`; verify the function fires; check for transaction rollback in log |
| Row exists but `result_json` is NULL | Skill returned empty/malformed result | Read the run_manifest_path if populated — daemon writes forensic bytes there even if result_json is empty; diagnose from manifest |
| Row exists with `status='failure'` | Skill itself errored (separate from CDP / daemon) | Read `result_json.error`; consult skill's own SKILL.md for the failure mode |

### 7.E — Option C: Session-expired detection doesn't fire

| Symptom | Likely cause | Fix |
|---|---|---|
| Skill log shows "navigated to alibaba.com" and then "results: empty" or "results: 0" — no SESSION_REFRESH event | Detection regex/selector in skill body doesn't match the actual logged-out page DOM | Read `skills/marketplace-search/alibaba/SKILL.md` — find the "Session Recovery" or detection block; the agent-browser snapshot it expects to see may have drifted. Open Alibaba logged-out in CDP manually, run `agent-browser snapshot http://localhost:9222 > /tmp/alibaba-logged-out.html`; find the actual "Sign In" element; update the detection selector via Edit tool |
| Skill log shows "detection check skipped — fast path" | Skill optimization that bypasses detection when it shouldn't | Check skill body for early-return; the fast path should only fire when listings are present, not when redirected to logged-out page |
| Skill log shows detection fired but immediately retried in-place without auth-signin | The `if (session_expired) { invoke auth-signin }` branch is broken | Edit-tool fix the invocation line in skill body |

### 7.F — Option C: auth-signin invocation fails

| Symptom | Likely cause | Fix |
|---|---|---|
| Log: "auth-signin/alibaba: skill not found" | Path mismatch | `ls skills/auth-signin/alibaba/SKILL.md` — confirm it exists. If skill name format differs (with/without slash), check production skill's invocation line and align |
| Log: "auth-signin/alibaba: timeout" | Sub-skill hangs on a UI element | Sub-skill is waiting for a selector that's not appearing. Snapshot the current page, find the actual element, Edit-fix the sub-skill |
| Log: "Continue with Google" not found | Alibaba changed their signin page layout | Use `agent-browser snapshot` to capture current signin page HTML; find the actual "Continue with Google" button selector; Edit-fix `skills/auth-signin/alibaba/SKILL.md` |

### 7.G — Option C: Google chooser shows wrong account

| Symptom | Likely cause | Fix |
|---|---|---|
| Chooser opens but selects a non-Dieter account | `dieter.x.s.danicek@gmail.com` selector drifted in `skills/auth-signin/google-account-chooser/SKILL.md` | `agent-browser snapshot` of the chooser; find the actual `attr-customurl` or `data-email` for Dieter's row; Edit-fix the selector |
| Chooser appears empty | Chrome profile doesn't have any Google accounts | Operator: open `chrome://settings/syncSetup`, ensure Dieter is signed in; restart Chrome via helper |
| 2FA / Touch ID prompt appears | Account has 2FA enabled OR Chrome's "use phone to confirm" setting is on | Operator: `chrome://settings/passwords` — disable "Use phone or tablet to fill in passwords"; for 2FA, sign Dieter in fresh in the profile via Chrome UI |

### 7.H — Cleanup leaves orphans

| Symptom | Fix |
|---|---|
| `pgrep -fl daemon` shows old daemon after kill | `kill -9 $(pgrep -f "target/.*daemon")`; check for zombie processes (`ps -ef \| grep defunct`); reboot Chrome with `pkill -9 -f 9222` |
| Chrome stuck on a profile lock | `~/.chrome-automation/SingletonLock` file — operator removes it (sentinel file, safe to delete) |

---

## 8. What you must NOT do (AGENTS.md compliance)

- ❌ `git reset --hard`, `git clean -fd`, `rm -rf` — absolutely forbidden per AGENTS.md "Irreversible Git" rules.
- ❌ `git push --force` or any history rewrite.
- ❌ `git commit --no-verify` or `AGENT_MAIL_GUARD_MODE=warn` — these defeat the AgentMail discipline. If hooks block, FIX the cause.
- ❌ `sed -i` or any script-based file mutation — use the Edit tool. The Triage fixes in §7 all assume Edit-tool changes.
- ❌ Modify the daemon's CDP integration (`ensure_cdp_9222` calls) UNLESS §7.B reveals an actual bug — those lines are load-bearing for L5; treat them as read-only by default.
- ❌ Skip operator checkpoints in Option C (§5.1 logged-in confirmation, §5.2 logout confirmation, §5.4 (7) chooser confirmation). The whole point of Option C is observer-confirmed Session Recovery.
- ❌ Cap iterations at "good enough". Run until ✅ or §4.5/§5.6 STUCK markers fire.
- ❌ Start L5 step 01 or any L5 pipeline work after this completes. Sign off, push, stop.

### 8.1 Bead discipline

This work is **NOT bead-tracked**. AGENTS.md RULE 2 applies only to bead-tracked work. No `bin/br-claim` / `bin/br-complete` needed. Pre-commit + pre-push AgentMail hooks still apply (they enforce file-reservation discipline) — keep AGENT_NAME exported from §3.3 throughout.

---

## 9. Sign-off + push

After both options PASS and TEST_REPORT.md is complete:

```bash
# 9.1 Stage exactly the evidence directory
git add docs/testing/area-h-2026-05-18/
git add docs/handoff-2026-05-18-area-h-daemon-e2e-demo.md   # this handoff doc itself (currently untracked)
git status --short | head -10                                # verify scope

# 9.2 Commit
git commit -m "$(cat <<'EOF'
area-h: daemon-driven E2E demo (Options A+C) — verdict READY

Option A (source-health/fts): daemon → ensure_cdp_9222 → Chrome launch → CDP skill → result in agent_session_logs.
Verdict: READY. [N] iterations, [Y] fixes applied during loop.

Option C (marketplace-search/alibaba): forced logout → Session Recovery → auth-signin/alibaba → Google chooser → Dieter Danicek selected → inline resume → listings returned.
Verdict: READY. [N] iterations, [Y] fixes applied during loop.

Evidence + TEST_REPORT.md at docs/testing/area-h-2026-05-18/.

Closes deferred items D.3, E, H from L5-readiness validation report.
L5 step 01 may be run at operator's discretion.

Co-Authored-By: <your-model-id> <noreply@anthropic.com>
EOF
)"

# 9.3 Push (re-verify AgentMail green right before)
# Call mcp__mcp-agent-mail__health_check, confirm health_level=green
git fetch origin
git rev-list HEAD..origin/main --count | grep -q '^0$' || { echo "STOP: origin moved"; exit 1; }
git push origin main
git log origin/main..HEAD                                    # expect: empty
echo "Pushed at $(date -u +%FT%TZ)"
```

If push is rejected: do NOT --force. `git pull --rebase`, re-run §3.4 (test gate), re-push.

### 9.4 Final report block

Print this to the operator:

```
AREA H COMPLETE.

Option A (source-health/fts E2E):     [READY | BLOCKED]  ([N] iters, [Y] fixes)
Option C (Alibaba Session Recovery):  [READY | BLOCKED]  ([N] iters, [Y] fixes)

Selector drift discoveries: [list, or "none"]
Skill files modified (if any): [list]
Daemon code modified (if any): [list, expected empty]

Commit: [<sha>]
Pushed to origin/main: ✅ at 2026-05-18T[time]

L5 status: [READY TO START / BLOCKED — see report]
Evidence: docs/testing/area-h-2026-05-18/TEST_REPORT.md
```

---

## 10. Estimated time

| Phase | Time |
|---|---|
| Required reading (§0) | 30 min |
| Pre-flight (§3) including agent registration | 10 min |
| Option A first attempt (§4) | 15 min |
| Option A validate-test-fix loop (likely 1–2 iters) | 0–30 min |
| Option C setup + logout (§5.1–5.2) | 5 min (operator interaction) |
| Option C first attempt (§5.3–5.5) | 15 min |
| Option C validate-test-fix loop (likely 1–3 iters, costly per iter) | 0–90 min |
| TEST_REPORT.md write | 15 min |
| Commit + push (§9) | 5 min |
| **Total (optimistic)** | **~95 min** |
| **Total (pessimistic, with multiple Option C iterations)** | **~3 hours** |

---

## 11. Why Area H matters for L5

L5 is the commercial-intelligence engine. Its step 09 (bead audit) and step 10 (kickoff) will trigger autonomous cron execution of CDP-requiring skills hundreds of times during the pipeline. Every L5 bead that depends on `marketplace-search/*`, `landed-cost/*`, `procurement-ingestion/procontract-*`, or any other CDP skill assumes:

1. The daemon can detect "skill requires CDP" and invoke `bin/ensure-cdp-9222` (tested by Area H Option A).
2. Skills can detect expired session and self-heal via `auth-signin/{site}/` (tested by Area H Option C).
3. Results land durably in SQLite without daemon-level retry (tested by both options).

All three are wired structurally per the L5-readiness validation report. **Area H proves they work behaviourally.** If any one of them is broken under real cron execution, L5 step 09 fails its bead audit, which blocks step 10 (kickoff), which blocks all 50+ L5 beads.

The cost of running Area H now (~95–180 min) is one hour against the cost of a half-run L5 that has to be unblocked mid-flight (often 10+ hours). The validate-test-fix loop ensures any bug discovered here is *fixed in place* before L5 starts — exactly the discipline that turned Freightos + Taric from PARTIAL to READY in their build phases.

You're not allowed to declare L5 ready until Area H signs off. That's the whole point.
