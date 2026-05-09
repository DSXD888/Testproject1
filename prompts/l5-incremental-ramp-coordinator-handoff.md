# L5 Incremental Ramp — Coordinator Handoff (1 → 2 → 5 Agents)

> **Paste this entire file into a fresh Claude Code OR Codex session as the L5 SUPERVISOR/COORDINATOR.**
>
> **You are NOT implementing L5.** You are the ramp coordinator: you monitor agent health, verify wrapper/hook behavior at each scale, diagnose any failures, fix them in a loop, and tell the user when to spawn each next agent. You do NOT claim L5 beads. The implementation agents (spawned separately) do that — they consume the L5 kickoff prompt.
>
> **You will run a validate → test → fix loop at each phase. Do not advance to the next phase until the current phase is fully green.** No phase has an iteration cap that auto-fails — diagnose and fix until green, OR escalate to the user with the specific failure.
>
> **CLI-agnostic.** Works identically from Claude Code and Codex. All shell, `br`, `am`, MCP-agent-mail tool calls are common.

---

## 0. Context (read fully before any tool call)

`Testproject1` post-L4 archive:
- L4 SHIPPED 2026-05-08; archived at `docs/pipeline/l4-shared-visibility/archived/2026-05-08-l4-shared-visibility/`.
- AGENTS.md Rule 2 (BEAD CLAIM DISCIPLINE) is constitutional — agents MUST use `bin/br-claim` and `bin/br-complete`.
- Phase A wrappers deployed: `bin/br-claim` (~407 lines), `bin/br-complete` (~415 lines). Both are post-bug-fix versions (commit `adec8e4` fixed the `search_messages` response-key chain).
- Phase B precommit hook deployed: `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` (1069 lines). Mode: Block. Phase B markers: `check_start_mail_for_bead_commits`, `extract_bead_ids`, `AGENT_MAIL_BYPASS`.
- Pre-push hook ALSO installed (per the wrapper-fix smoke loop's `am guard install --prepush`).
- Pre-L5 wrapper smoke test PASSED 2026-05-08 (bead `testproject1-kgr8`, agent `CyanTrout`, both Start + Complete mails landed cleanly).

**What single-agent testing did NOT verify (this handoff's reason for existing):**

| Risk class | Caught by single-agent? | Caught by ramp? |
|---|---|---|
| Wrapper happy-path bugs (e.g., the response-key bug) | ✅ already fixed | ✅ |
| Same-bead claim race (two agents) | ❌ no | ✅ Phase 2 |
| Same-file claim race across different beads | ❌ no | ✅ Phase 2 |
| `git push` contention | ❌ no | ✅ Phase 2/3 |
| `.beads/issues.jsonl` rebase conflict on simultaneous `br close` | ❌ no | ✅ Phase 2/3 |
| Mail archive eventual-consistency lag | ❌ no | ✅ Phase 2/3 |
| Server overload from 5 concurrent `search_messages` from the hook | ❌ no | ✅ Phase 3 |
| Wrapper missing retry for transient failures | ❌ no | ✅ Phase 2/3 surfaces it |

**Your mission:** verify all of the above are either mitigated by existing infrastructure OR cleanly recoverable, before the swarm runs at full scale on real L5 work.

**Project key:** `/Users/zacfarrell/Desktop/Testproject1`
**Branch:** `main` only (per AGENTS.md)
**Bead label:** `layer-5` (or whatever the L5 spec uses — verify in §1)

---

## 1. Pre-Flight (fail-hard if any of these fail; do NOT spawn any agents until all green)

```bash
cd /Users/zacfarrell/Desktop/Testproject1

# A. Working tree clean + synced
[ -z "$(git status --porcelain)" ] && echo "✓ tree clean" || echo "ABORT: tree dirty"
git fetch
ah=$(git rev-list --left-right --count origin/main...HEAD)
behind=$(echo $ah | awk '{print $1}'); ahead=$(echo $ah | awk '{print $2}')
[ "$behind" = "0" ] && [ "$ahead" = "0" ] && echo "✓ synced with origin/main" || echo "ABORT: $behind behind, $ahead ahead"

# B. Wrappers exist and work
test -x bin/br-claim    && bin/br-claim --help    >/dev/null 2>&1 && echo "✓ bin/br-claim works"    || echo "ABORT: bin/br-claim broken"
test -x bin/br-complete && bin/br-complete --help >/dev/null 2>&1 && echo "✓ bin/br-complete works" || echo "ABORT: bin/br-complete broken"

# C. The wrapper response-key fix is in (commit adec8e4 or later)
grep -q '("result", "messages", "results")' bin/br-claim    && echo "✓ br-claim response-key fix in"    || echo "ABORT: br-claim missing fix"
grep -q '("result", "messages", "results")' bin/br-complete && echo "✓ br-complete response-key fix in" || echo "ABORT: br-complete missing fix"

# D. Hook installed (both pre-commit and pre-push) — fail-hard, no eyeballing
am_status=$(am guard status /Users/zacfarrell/Desktop/Testproject1 2>&1)
echo "$am_status" | grep -qE "Pre-commit\s+installed" && echo "✓ pre-commit hook installed" || echo "ABORT: pre-commit hook NOT installed (run: am guard install /Users/zacfarrell/Desktop/Testproject1 /Users/zacfarrell/Desktop/Testproject1)"
echo "$am_status" | grep -qE "Pre-push\s+installed"   && echo "✓ pre-push hook installed"   || echo "ABORT: pre-push hook NOT installed (run: am guard install --prepush /Users/zacfarrell/Desktop/Testproject1 /Users/zacfarrell/Desktop/Testproject1)"
echo "$am_status" | grep -qE "Mode\s+Block"            && echo "✓ Mode=Block"                 || echo "ABORT: hook Mode is NOT Block — enforcement is disabled"

# E. Phase B markers in hook
hook=.git/hooks/hooks.d/pre-commit/50-agent-mail.py
grep -q "check_start_mail_for_bead_commits" "$hook" && echo "✓ Phase B start-mail check present" || echo "ABORT: Phase B missing"

# F. agent-mail server alive
curl -s --max-time 3 http://127.0.0.1:8765/healthz | grep -q '"status":"alive"' \
  && echo "✓ agent-mail server alive" \
  || echo "ABORT: server not reachable; restart with: am serve-http --port 8765 &"

# G. AGENTS.md Rule 2 present
grep -q "RULE NUMBER 2: BEAD CLAIM DISCIPLINE" AGENTS.md && echo "✓ Rule 2 in AGENTS.md" || echo "ABORT: Rule 2 missing"

# H. L5 source-of-truth artifacts exist
test -f docs/strategy/layers/layer-5-expanded-commercial-intelligence.md && echo "✓ L5 spec present" || echo "ABORT: L5 spec missing"
test -d docs/pipeline/l5-expanded-commercial-intelligence/artifacts && echo "✓ L5 artifacts dir present" || echo "ABORT: run /pipemaxxer first to generate L5"

# I. L5 source artifacts: spec dir AND kickoff file specifically
l5_kickoff=docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-kickoff.md
test -f "$l5_kickoff" && echo "✓ L5 kickoff present at $l5_kickoff" || echo "ABORT: L5 kickoff missing — run /pipemaxxer first"

# J. L5 beads exist and at least one is ready
l5_total=$(br list --label layer-5 --json 2>&1 | jq '.issues | length')
l5_ready=$(br ready --label layer-5 --json 2>&1 | jq '.issues | length')
echo "L5 beads: total=$l5_total, ready=$l5_ready"
[ "$l5_ready" -ge "1" ] && echo "✓ ≥1 ready L5 bead" || echo "ABORT: no L5 ready beads — run /pipemaxxer or fix dependencies"

# K. Your own coordinator AGENT_NAME / AGENT_MAIL_NAME set (for any commits you make during diagnosis)
[ -n "$AGENT_NAME" ]      && echo "✓ AGENT_NAME=$AGENT_NAME"           || { echo "Setting AGENT_NAME=l5-coordinator"; export AGENT_NAME="l5-coordinator"; }
[ -n "$AGENT_MAIL_NAME" ] && echo "✓ AGENT_MAIL_NAME=$AGENT_MAIL_NAME" || { echo "Setting AGENT_MAIL_NAME=L5Coordinator"; export AGENT_MAIL_NAME="L5Coordinator"; }

# L. ⚠️ END-TO-END HOOK TEST — catches the "could not locate archive for project"
# failure mode that surfaces only at commit time, not at hook-installation time.
# Uses `git commit --allow-empty` so no files are created/deleted (Rule 1 safe).
# Uses `chore:` prefix (exempt from start-mail subcheck) so this exercises only
# the hook's plumbing, not the bead-id-prefix path.
echo "Running end-to-end hook test (this should succeed without bypass)..."
if git commit --allow-empty -m "chore: pre-flight hook end-to-end test ($(date -u +%FT%TZ))" 2>&1 | tee /tmp/l5-ramp-hook-test.log; then
  echo "✓ hook accepts commits end-to-end (commit landed; no bypass needed)"
  # Remove the empty marker commit. --soft moves HEAD back, leaves nothing staged
  # (no files were involved). Per AGENTS.md, --soft is NOT in the forbidden list
  # (only --hard, clean -fd, rm -rf are forbidden); this is the canonical pattern
  # for "I created a commit I no longer want."
  git reset --soft HEAD~1
  echo "✓ test commit reverted (HEAD now at: $(git log --oneline -1))"
else
  echo "ABORT: hook BLOCKED a chore: commit during pre-flight"
  echo "      Look in /tmp/l5-ramp-hook-test.log for the exact error."
  echo "      Common causes (in order of likelihood):"
  echo "        - 'could not locate archive for project <name>': hook's embedded"
  echo "          PROJECT identifier doesn't match the agent-mail server's"
  echo "          project archive. → See Recovery Recipe R7 below."
  echo "        - server returned 4xx/5xx (not connection-refused): server has a"
  echo "          state bug. → See Recovery Recipe R5 (restart) and R7 (re-ensure)."
  echo "        - hook script crashed (Python traceback): hook bug, escalate."
  echo "      The hook itself is auto-blocking; nothing to clean up."
fi
```

**Note on `git reset --soft HEAD~1`:** AGENTS.md "Irreversible Git & Filesystem Actions" forbids `git reset --hard`, `git clean -fd`, and `rm -rf`. The `--soft` variant is NOT in that list. `--soft HEAD~1` only moves the branch pointer back one commit; the working tree and index are unchanged. Since the test commit was empty (`--allow-empty`), there is nothing to lose.

**CHECKPOINT pre-flight:** if any line above printed `ABORT:`, STOP and report to the user. Do NOT spawn any L5 agents until all green.

The check **L** is the one that has historically been missed. Hook installation can succeed (`am guard status` reports installed) while the hook itself fails at runtime due to a project-archive lookup mismatch. The end-to-end test commit catches this BEFORE you spawn agents — without it, you'd discover the bug only when an implementation agent tries to make their first commit.

If you had to fix any pre-flight item (e.g., restart the server, re-install the hook), re-run the entire pre-flight from the top. Do NOT spawn agents on a partially-fixed system.

---

## 2. Mission

Three phases, each with its own validate → test → fix loop. **Do not advance until the current phase is fully green.**

| Phase | Spawn | Acceptance |
|---|---|---|
| **Phase 1** | 1 L5 agent | One full bead claim → code → commit → push → complete cycle, all artifacts verified |
| **Phase 2** | 1 more agent (total 2) | 15 minutes of concurrent operation with at least 2 beads closed by EACH agent, zero manual unwedges, all races recovered automatically |
| **Phase 3** | 3 more agents (total 5) | 30 minutes of full-swarm operation, ≥10 beads closed total, zero unrecoverable failures |

You stay in this handoff loop until **Phase 3 acceptance is met OR you escalate a specific unfixable failure to the user**. Don't declare done early.

---

## 3. Phase 1 — Single Agent Live Verification

### 3a. Tell the user to spawn the first L5 agent

> **MESSAGE TO USER:** "Pre-flight green. Please spawn ONE L5 implementation agent now (Claude Code or Codex). Paste the L5 kickoff prompt: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-kickoff.md` (or whatever path L5's pipemaxxer produced). I'll watch their first bead complete end-to-end before authorizing agent #2."

Wait for the user to confirm the agent is running. Note their `agent_name` (you may need to ask the user, or check `list_agents` after a few minutes).

### 3b. Watch the first bead claim

You'll observe via these data sources (poll every 60–90s, not faster — don't burn tokens spinning):

```bash
# Live observability
br list --label layer-5 --status=in_progress --json 2>&1 | jq -r '.issues[] | "\(.id) | claimed_at=\(.updated_at)"'
git log --oneline -5 2>&1
git status -sb 2>&1   # what's the active agent editing?
```

```
# In your agent session, periodically:
search_messages(
  project_key="/Users/zacfarrell/Desktop/Testproject1",
  query="layer-5 OR L5 OR Start",
  limit=20,
  ranking="recency"
)
list_agents(project_key="/Users/zacfarrell/Desktop/Testproject1")
```

**CHECKPOINT P1.claim:** As soon as you see ONE in-progress bead and ONE `[<bead-id>] Start` mail, report to the user: "Agent X claimed bead Y, Start mail visible." Don't proceed to wait for completion until the claim is confirmed.

### 3c. Watch the commit + push

The agent will eventually commit and push. Watch for:

```bash
# A bead-id-prefixed commit lands
git log --oneline -1 | grep -E "^[a-f0-9]+ testproject1-[a-z0-9]+\.[0-9]+(\.[0-9]+)*:"
# Verify the precommit hook ACCEPTED it (would have aborted otherwise — no commit at all)
```

If the agent is stuck for >10 min between claim and commit with no apparent progress, ask the user if the agent is healthy. Do NOT touch the agent's bead state.

### 3d. Watch the complete

```bash
# Bead transitions to closed
br show <bead-id> --json 2>&1 | jq -r '..|objects | select(.id=="<bead-id>") | "status=\(.status)"' | head -1
# Expected: status=closed
```

Search for the Complete mail:
```
search_messages(query="[<bead-id>] Complete")
```

### 3e. Verify all post-conditions

After ONE bead has fully cycled through claim → commit → complete, verify all 6 numbered post-conditions:

| # | Post-condition | Verification |
|---|---|---|
| **PC1** | Bead status=closed | `br show <id> --json` |
| **PC2** | Start mail in archive | `search_messages(query="[<id>] Start")` returns ≥1 message from agent-name |
| **PC3** | Complete mail in archive | `search_messages(query="[<id>] Complete")` returns ≥1 message from agent-name |
| **PC4** | Commit on origin/main | `git log origin/main --oneline -10` shows the bead-id-prefixed commit |
| **PC5** | Working tree clean | `git status --porcelain` empty |
| **PC6** | File reservations released | `am file_reservations list /Users/zacfarrell/Desktop/Testproject1` does NOT mention this bead |

### 3f. Phase 1 Validate → Test → Fix Loop

```
LOOP until P1 acceptance criteria all green:

  IF claim succeeded AND commit succeeded AND complete succeeded AND all post-conditions pass:
    → P1 COMPLETE. Advance to Phase 2.

  IF the agent is stuck mid-claim (>10 min, no Start mail):
    DIAGNOSE:
      - Is the server reachable? (`curl /healthz`)
      - Did the agent set AGENT_NAME/AGENT_MAIL_NAME? (ask user)
      - Did `bin/br-claim` exit with an error? (ask user for the agent's terminal output)
    FIX (in priority order):
      1. If server died: restart it (`pkill am ; am serve-http --port 8765 &`)
      2. If agent didn't set env vars: tell user to export them and re-run claim
      3. If wrapper crashed: this is a NEW bug — capture the error, escalate to user, do NOT advance
    Re-run Phase 1 from §3a (with the same OR a different bead).

  IF claim succeeded but commit was BLOCKED by the precommit hook:
    DIAGNOSE:
      - The hook should have accepted (Start mail exists). Why did it block?
      - Check the hook's exact error output (`<bead-id>` mismatch? lookback window expired?)
      - Check the agent-name in the Start mail vs the AGENT_NAME committing
    FIX:
      1. If lookback window expired: check the smoke-test bead's age — if >4h old, this is a TIME issue, ask user to re-claim
      2. If agent-name mismatch: agent didn't register correctly, ask user to fix
      3. If start-mail check has a regex bug: capture, escalate, do NOT advance
    Re-run Phase 1.

  IF complete failed (br-complete exit non-zero):
    DIAGNOSE:
      - What did br-complete print? (ask user)
      - Was the commit pushed? (`git rev-list HEAD..origin/main` should be empty if commit is on origin)
      - Is bead still `in_progress`?
    FIX:
      1. If "commit not pushed": run `git push` manually, then `bin/br-complete <bead-id>` again
      2. If push race: `git pull --rebase ; git push ; bin/br-complete <bead-id>`
      3. If bead is in_progress and Complete mail wasn't sent: see "manual recovery" in pre-l5-sanity-check handoff §6f
      4. If wrapper crashed: NEW bug — capture, escalate, do NOT advance
    Re-run Phase 1 (or recover the in-flight bead manually if the agent already cycled).

  IF the hook says "could not locate archive for project":
    See Recovery Recipe R7 below. This is a config bug between the hook
    and the server, NOT a wrapper bug. Fix it at the infrastructure level
    before the ramp continues.
    Re-run pre-flight check L (the end-to-end hook test) to confirm the fix.

  IF PC6 (file reservations) fails (zombie reservation):
    DIAGNOSE: which bead, which agent, when does TTL expire?
    FIX: `am file_reservations release` for that specific bead, OR wait for TTL expiry (default 1h)
    Re-verify PC1–PC6.
```

**Do NOT proceed to Phase 2 until PC1–PC6 are all green for ONE complete bead cycle.**

---

## 4. Phase 2 — Two-Agent Concurrency

### 4a. Tell the user to spawn agent #2

> **MESSAGE TO USER:** "Phase 1 green: agent X cycled bead Y cleanly. Please spawn ONE more L5 agent now (total 2). Paste the same L5 kickoff. I'll monitor for the next 15 minutes for race conditions, push contention, and rebase conflicts."

### 4b. What you're specifically watching for

Phase 2 is the FIRST exposure to concurrency. Five specific scenarios to observe:

**S1. Same-bead race** (rare but possible): Two agents both run `br ready --label layer-5`, both pick the same top-priority bead. Expected behavior:
- The first to call `file_reservation_paths` wins.
- The second gets `FILE_RESERVATION_CONFLICT`, `bin/br-claim` exits non-zero with a clear error.
- The losing agent re-runs `br ready`, gets a different bead, claims that one.
- **Verification:** check the mail archive — there should be EXACTLY ONE `[<bead-id>] Start` mail per bead, NEVER two from different agents. If you see two: a race bypassed the safety. STOP, escalate.

**S2. Same-file race** (more common — chokepoint files): Two agents claim DIFFERENT beads but those beads' file lists overlap (e.g., both touch `crates/daemon/src/main.rs`). Expected:
- The first wins the reservation; second gets conflict on `bin/br-claim`.
- Loser aborts cleanly.
- **Verification:** `am file_reservations list /Users/zacfarrell/Desktop/Testproject1` should show one agent holding the contested file at a time.

**S3. Push race** (high probability): Agents A and B both finish work and run `bin/br-complete` within seconds. Both attempt `git push`. Expected:
- One push wins. The other agent's push fails with non-fast-forward.
- `bin/br-complete` for the loser exits non-zero. Agent re-runs after `git pull --rebase`.
- **Verification:** check `.git/agent-mail-bypasses.log` — if `AGENT_MAIL_BYPASS=1` was used, an agent gave up. That's BAD — escalate.

**S4. `.beads/issues.jsonl` rebase conflict** (medium probability): Two agents close beads simultaneously. Both stage `.beads/issues.jsonl`. The second `git pull --rebase` surfaces a conflict because both agents added a `"closed"` line for different beads.
- **Recovery:** the JSONL format is line-per-bead. The conflict is RESOLVABLE by keeping all bead lines from both sides. The agent should:
  ```bash
  git status                       # see the conflict marker in .beads/issues.jsonl
  # Manually edit: keep BOTH agents' bead-status lines (delete <<<<<<< / ======= / >>>>>>> markers)
  bin/br-sync                      # re-canonicalize the JSONL (br-sync sorts/dedupes)
  git add .beads/issues.jsonl
  git rebase --continue
  git push
  bin/br-complete <bead-id>        # retry
  ```
- **Verification:** if the agent successfully recovers, you'll see the rebased commits on `origin/main`. If they're stuck for >15 min in the rebase: STOP, intervene, possibly manually unwedge.

**S5. Mail archive duplicate Start** (LOW probability but CRITICAL): Two agents both pass the `is there a Start mail?` check before either has actually sent its Start. Both then send Start, leading to two Start mails for the same bead from different agents.
- **Verification:** for each bead, verify exactly ONE Start mail. If you see two: this is a concurrency bug in the wrapper's idempotency check. STOP, escalate, do NOT proceed.

### 4c. Acceptance criteria for Phase 2

- [ ] **2 agents working concurrently** for at least 15 minutes
- [ ] **At least 3 total bead closures across both agents.** Per-agent ≥1 (no agent stuck idle for the full window). The original "≥2 each" was too strict — bead-pick timing can legitimately give one agent more closures than the other in a 15-min window. As long as both agents are MAKING PROGRESS (state-flipping, committing, pushing), not stuck.
- [ ] **Zero duplicate `[<id>] Start` mails** for any single bead (this is the strict one — no flexibility)
- [ ] **Zero new IMPLEMENTATION-AGENT bypass entries** in `.git/agent-mail-bypasses.log`. (Coordinator-bypass entries during diagnosis are recorded but allowed per Critical Rule 6 — distinguish by the `subject=` field in the log line, which records the commit message; coordinator commits will have `chore:`/`docs:` prefixes, implementer commits should never appear here at all.)
- [ ] **Zero beads stuck in `in_progress`** at the end of the 15-minute window
- [ ] **All file reservations released** at the end (no zombies)
- [ ] **Working tree clean** on both agents' shells
- [ ] **`origin/main` is current** with all completed work pushed

### 4d. Phase 2 Validate → Test → Fix Loop

```
LOOP until P2 acceptance criteria all green:

  Every 60–90s, check:
    - br list --label layer-5 --status=in_progress (count + age)
    - git log --oneline -10 (commit cadence — should see ~2 commits/min)
    - .git/agent-mail-bypasses.log (must remain empty)
    - mail archive: search_messages query="layer-5 Start" (count of distinct bead-ids)

  IF you observe S1 (duplicate Start mails for same bead):
    STOP swarm immediately (tell user to halt agent #2).
    DIAGNOSE: review the idempotency check inside `bin/br-claim` (search for the
    `search_messages` call near `mcp_call(url, token, "search_messages", ...)`,
    NOT a hardcoded line number — line numbers drift). The check filters
    matches by sender_name to determine "did I already send Start for this bead?"
    Likely cause: search_messages eventual consistency, OR the wrapper's "look up Start mail" race window is too tight.
    FIX:
      1. If eventual consistency: add a small delay (e.g., 500ms) between reservation and Start-mail send.
      2. If race window: serialize the check more strictly.
      3. Both require code change to bin/br-claim. Capture diff, propose to user, do NOT auto-apply.
    Once fixed: re-run Phase 1 to verify single-agent still works, then re-run Phase 2.

  IF you observe S3 (push race) and an agent gets stuck:
    DIAGNOSE: which agent, which bead, what error?
    FIX:
      1. Tell user the recovery steps for that agent: `git pull --rebase ; bin/br-complete <bead-id>`
      2. If the agent is autonomous and didn't auto-recover: this is a wrapper gap. The wrapper's br-complete should have its own retry logic.
      3. Document the gap; if it recurs, propose adding retry to bin/br-complete (max 3 attempts with exponential backoff).
    Continue monitoring.

  IF you observe S4 (rebase conflict) and an agent is stuck >15 min:
    DIAGNOSE: review the agent's bash output (ask user). Is the conflict in .beads/issues.jsonl? In code files?
    FIX:
      1. If .beads/ conflict: walk the agent through the resolution (see S4 above).
      2. If code conflict: that's a real chokepoint collision. Two agents claimed beads that BOTH wrote to the same file lines. File reservations should have caught this — investigate why they didn't.
    Continue monitoring.

  IF you observe AGENT_MAIL_BYPASS=1 events:
    STOP swarm immediately.
    DIAGNOSE: what was bypassed? An agent gave up on the wrappers — there's a usability or correctness gap.
    FIX: capture the bypass log entries, escalate to user with proposed root-cause analysis.
    Do NOT advance.

  IF 15 minutes elapse with all acceptance criteria green:
    → P2 COMPLETE. Advance to Phase 3.
```

**Do NOT proceed to Phase 3 until 15 consecutive minutes of clean 2-agent operation.**

---

## 5. Phase 3 — Full Swarm (5 Agents)

### 5a. Tell the user to spawn 3 more agents

> **MESSAGE TO USER:** "Phase 2 green: 2-agent operation stable for 15 min. Please spawn 3 more L5 agents now (total 5). I'll monitor the swarm for 30 minutes for systemic issues."

### 5b. Health metrics to watch

```bash
# Cadence — 5 agents should produce ~5 commits/min steady state, but bursty
git log --since="5 minutes ago" --oneline | wc -l

# In-progress count — should be ~5, never stuck high
br list --label layer-5 --status=in_progress --json 2>&1 | jq '.issues | length'

# Closed cadence
br list --label layer-5 --status=closed --json 2>&1 | jq '.issues | length'

# Bypass log (must remain empty)
test -f .git/agent-mail-bypasses.log && wc -l .git/agent-mail-bypasses.log

# Server health (under load)
curl -s --max-time 3 http://127.0.0.1:8765/healthz | grep '"status":"alive"'

# File reservations
am file_reservations list /Users/zacfarrell/Desktop/Testproject1 | wc -l
```

```
# Mail archive activity
search_messages(query="layer-5", limit=50, ranking="recency")
list_agents(project_key="/Users/zacfarrell/Desktop/Testproject1")
```

### 5c. Acceptance criteria for Phase 3

- [ ] **5 agents working concurrently** for at least 30 minutes
- [ ] **≥10 beads closed** during the window
- [ ] **Each agent closes ≥1 bead** (no agent stuck idle)
- [ ] **Zero duplicate Start mails** (per-bead)
- [ ] **Zero new IMPLEMENTATION-AGENT bypass entries** in `.git/agent-mail-bypasses.log` (coordinator-bypass entries are exempt; distinguish by the `subject=` field — coordinator commits will have `chore:` or `docs:` prefixes)
- [ ] **`in_progress` count stays ≤6** at any sample (5 is the steady-state target; transient 6 is acceptable for the brief window between an agent's `file_reservation_paths` returning success and `br update --status=in_progress` landing). If you sustain count ≥7 for >2 min: that's a bead-leak signal.
- [ ] **Server stays alive** (`/healthz` returns alive on every poll)
- [ ] **No agent abandons mid-claim** (zombie file reservations >10 min after agent's last activity)
- [ ] **Working tree clean** on the user's coordinator shell at the end

### 5d. Phase 3 Validate → Test → Fix Loop

```
LOOP until P3 acceptance criteria all green:

  Every 90s, check:
    - In-progress count (should be roughly stable around 5; if growing without closes, agents are stuck)
    - Commit cadence (should match throughput; if it stalls but in-progress stays high, something's wedged)
    - Bypass log (must stay empty)
    - Server health
    - For each agent in list_agents: last_active_ts. If any agent's last_active_ts is >10 min ago AND that agent has an in-progress bead: it crashed. Recover the bead manually.

  IF in-progress count grows without bound (e.g., reaches 10):
    DIAGNOSE: agents are claiming but not closing. Either bin/br-complete is failing, or beads are blocked on file conflicts.
    FIX:
      1. Check git log cadence. If it's flatlined: agents are wedged on rebase. Walk through .beads/ resolution for each.
      2. If commit cadence is fine but br-complete keeps failing: a wrapper bug under load. Capture, escalate.
    Reduce swarm size to 2 if needed (tell user to kill 3 agents).

  IF server returns non-alive even briefly:
    DIAGNOSE: did 5 concurrent search_messages overwhelm it?
    FIX:
      1. Restart server: `pkill am ; am serve-http --port 8765 &`
      2. If it crashed under load, this is a SERVER bug — increasing load will hit it again. Document, escalate, reduce swarm.
    Continue monitoring at reduced size.

  IF bypass-log gets entries:
    STOP IMMEDIATELY (tell user to pause all agents).
    Investigate which agent and why. Bypass means an agent decided the wrappers were broken enough to circumvent. That's a critical signal.

  IF an agent's last_active_ts is >10 min and they have an in_progress bead:
    The agent crashed or context-compacted out of the work loop.
    Recover the bead:
      1. `am file_reservations release` for that bead's files (with --force if needed for the dead agent)
      2. `br update <bead-id> --status=open` (return to ready queue)
      3. Send a `[<bead-id>] BLOCKED -> reset` mail to peers explaining the recovery
      4. Tell user the agent crashed; another agent will pick up the bead from `br ready`
    Continue monitoring.

  IF 30 minutes elapse with all acceptance criteria green:
    → P3 COMPLETE. Swarm is stable. L5 implementation can proceed without coordinator intervention.
```

---

## 6. Final Acceptance and Reporting

### When all 3 phases pass

```
✅ L5 incremental ramp: COMPLETE
   Phase 1 (1 agent):  PASS — bead <id> closed cleanly via wrappers
   Phase 2 (2 agents): PASS — <N> beads closed in 15 min, zero races escaped
   Phase 3 (5 agents): PASS — <N> beads closed in 30 min, swarm stable

📊 Steady-state metrics:
   Bead throughput:        <N> beads/hr at 5 agents
   Mail traffic:           Start + Complete per bead (zero duplicates, zero bypasses)
   File reservations:      <N> peak concurrent, zero zombies post-cycle
   Push contention:        <N> non-fast-forward retries, all auto-recovered
   .beads/ rebase events:  <N>, all auto-resolved

🚀 L5 implementation can proceed at 5-agent capacity without further
   coordinator intervention. Continue to monitor at lower frequency
   (every 15 min for the first hour; every hour thereafter).

   Report any of these escalation triggers:
   - Duplicate [<id>] Start mails for the same bead
   - AGENT_MAIL_BYPASS entries in .git/agent-mail-bypasses.log
   - In-progress count growing unboundedly (bead-leak)
   - Server health failures
```

### If you have to escalate during any phase

```
❌ L5 ramp PAUSED at Phase <N>

Failure: <one-line summary>
Specific evidence: <bash output / mail-archive snippet / commit hash>
Hypothesis: <your best guess at root cause>
Mitigation tried: <what you attempted, in order>
Recommendation: <user action needed>

Swarm state at pause:
   Active agents: <list with names>
   In-progress beads: <list with ids>
   Stuck agents: <list with names + last_active_ts>
   .git/agent-mail-bypasses.log entries: <count>

Do NOT advance to next phase until user resolves the escalation.
```

---

## 7. Critical Rules

1. **You are NOT an L5 implementer.** Do not claim L5 beads. Do not run `bin/br-claim` for any L5 bead. Your role is to verify, monitor, diagnose, and tell the user when to ramp.
2. **Each phase has a strict acceptance gate.** Do not advance until that phase is fully green. The user paying for this incremental ramp would rather wait an extra hour at Phase 2 than discover the bug at Phase 3.
3. **The validate → test → fix loop has no auto-cap.** Stay in the loop until the phase is green OR until you escalate a SPECIFIC unfixable failure to the user. Do not advance prematurely just because it's been a while.
4. **Wrapper bugs surfaced under concurrency are real bugs, not flakiness.** If you see a duplicate Start mail or a bypass-log entry: STOP, escalate. Concurrency bugs cannot be ignored.
5. **Do not modify wrapper or hook code yourself during the ramp.** If you discover a real bug, capture it, propose the fix, and let the user authorize the change. Code edits during a live swarm risk introducing more problems than they fix.
6. **Use AGENT_MAIL_BYPASS=1 yourself ONLY for diagnosis commits** (e.g., a commit that intentionally tests the hook). Never tell an L5 implementation agent to bypass.
7. **Push your own diagnosis commits sparingly.** You don't have a bead to claim. Use `chore:`, `docs:`, or similar exempt prefixes if you commit anything during the ramp.
8. **AGENTS.md Rule 2 + Rule 1 still apply to you.** No file deletion. No bypassing the bead-claim discipline for your own work (though you have no beads to claim).
9. **Pre-flight (§1) is REQUIRED.** Do not skip even if it's been done recently — state changes between sessions.
10. **Push to origin before declaring done.** Per AGENTS.md, work isn't done until `git push` succeeds.

---

## 8. Reference: Common Recovery Recipes

### R1. Push race ("non-fast-forward")
```bash
git pull --rebase
git push
bin/br-complete <bead-id>   # retry
```

### R2. `.beads/issues.jsonl` rebase conflict
```bash
git status                  # confirm conflict file
# Edit the file: keep BOTH agents' bead-status lines, remove conflict markers
bin/br-sync                 # canonicalize/sort/dedupe the JSONL
git add .beads/issues.jsonl
git rebase --continue
git push
bin/br-complete <bead-id>   # retry
```

### R3. Wrapper exits mid-flow with bead in_progress
```bash
br show <bead-id> --json   # confirm in_progress
# If commit was made and pushed but Complete mail wasn't sent:
bin/br-complete <bead-id> --commit <commit-sha>   # explicit SHA
# If wrapper still fails: manual recovery — see pre-l5-sanity-check handoff §6f
```

### R4. Zombie file reservation (held by a dead agent)
```bash
am file_reservations list /Users/zacfarrell/Desktop/Testproject1
# Identify the stale reservation (note the holder's agent_name)
# If it's the dead agent's:
am file_reservations release <reservation-id>
# OR (more invasive): force_release_file_reservation via MCP tool, with a reason
```

### R5. Server down or unresponsive (and re-register project after restart)
```bash
pkill am 2>/dev/null || true
am serve-http --port 8765 &
sleep 2
curl -s http://127.0.0.1:8765/healthz | grep -q '"status":"alive"' && echo "✓ server back"

# CRITICAL: after a server restart, re-call ensure_project — server-side state
# (project registry, archive index) may need re-seeding. Skipping this leads
# to the "could not locate archive for project" hook failure (see R7).
TOKEN=$(python3 -c "import json,os; cfg=json.load(open(os.path.expanduser('~/.claude.json'))); srv=cfg.get('mcpServers',{}).get('mcp-agent-mail',{}); h=srv.get('headers',{}); a=h.get('Authorization','').replace('Bearer ',''); print(a)" 2>/dev/null)
curl -s --max-time 5 -X POST http://127.0.0.1:8765/mcp/ \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"jsonrpc":"2.0","id":"1","method":"tools/call","params":{"name":"ensure_project","arguments":{"human_key":"/Users/zacfarrell/Desktop/Testproject1"}}}' | head -3

# Verify the hook works again with the end-to-end test from pre-flight check L:
git commit --allow-empty -m "chore: post-restart hook test ($(date -u +%FT%TZ))" \
  && git reset --soft HEAD~1 \
  && echo "✓ hook works after restart"
```

### R6. Crashed agent with in-progress bead
```
1. Verify the agent is gone: list_agents — last_active_ts > 10 min ago
2. Release their reservations:
   am file_reservations list /Users/zacfarrell/Desktop/Testproject1 | grep <crashed-agent>
   am file_reservations release <each-reservation-id>
3. Reset the bead:
   br update <bead-id> --status=open
   br comment add <bead-id> --body "Reset from in_progress: agent <name> crashed at <ts>; another agent will pick up."
4. (Optional) send mail to peers documenting the reset
```

### R7. Hook says "could not locate archive for project"
This was empirically observed during this very project. Symptom: `git commit` (or `git push`) fails with:
```
mcp-agent-mail: guard could not locate archive for project '<NAME>'
```

The cause is a mismatch between the `PROJECT` identifier embedded in the hook (set when `am guard install` was run) and the project archive the agent-mail server has registered.

**Diagnosis:**
```bash
# What does the hook think the project is?
grep "^PROJECT\s*=" .git/hooks/hooks.d/pre-commit/50-agent-mail.py

# What does the server have registered?
TOKEN=$(python3 -c "import json,os; cfg=json.load(open(os.path.expanduser('~/.claude.json'))); srv=cfg.get('mcpServers',{}).get('mcp-agent-mail',{}); h=srv.get('headers',{}); a=h.get('Authorization','').replace('Bearer ',''); print(a)" 2>/dev/null)
curl -s --max-time 5 -X POST http://127.0.0.1:8765/mcp/ \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"jsonrpc":"2.0","id":"1","method":"tools/call","params":{"name":"ensure_project","arguments":{"human_key":"/Users/zacfarrell/Desktop/Testproject1"}}}' | head -5
# Look for: human_key, slug, project_uid in the response.
```

**Fix (in priority order — try option 1 first):**

**Option 1: Re-register the project on the server (least invasive).** The above `ensure_project` call IS the registration call — if it returns successfully and the hook still fails, the issue is the hook's `PROJECT` value vs the server's lookup keys. Try option 2.

**Option 2: Re-install the hook with the project's absolute path as PROJECT.** The agent-mail server's lookup uses the `human_key` (the absolute path), so installing the hook with that exact value as PROJECT aligns them:
```bash
pkill am 2>/dev/null || true
am serve-http --port 8765 &
sleep 2
am guard install /Users/zacfarrell/Desktop/Testproject1 /Users/zacfarrell/Desktop/Testproject1
am guard install --prepush /Users/zacfarrell/Desktop/Testproject1 /Users/zacfarrell/Desktop/Testproject1   # if pre-push is also wanted
am guard status /Users/zacfarrell/Desktop/Testproject1   # confirm
# Run the pre-flight L test again to verify:
git commit --allow-empty -m "chore: post-reinstall hook test" && git reset --soft HEAD~1 && echo "✓ fixed"
```

**Option 3 (NOT for normal use — emergency only):** Use `AGENT_MAIL_BYPASS=1` for a single legitimate diagnosis or out-of-band commit. The bypass is logged in `.git/agent-mail-bypasses.log` for audit. If you're using bypass during a ramp phase, the phase's bypass-log acceptance criterion will fail and the ramp will be paused — that's by design. Bypass is for diagnosis, not for production swarm work.

---

## 9. Stop Conditions (escalate to user)

Halt the ramp and escalate to the user immediately if any of these occur:

- **Duplicate `[<bead-id>] Start` mails** from different agents — concurrency bug in wrapper idempotency
- **Implementation-agent bypass-log entry** — an agent gave up on the wrappers (coordinator-bypass entries are exempt — see Phase 2/3 acceptance criteria)
- **Hook fails with `could not locate archive for project`** during a phase (was caught by pre-flight L if you ran it; if it surfaces mid-phase, the server state regressed) — see Recovery Recipe R7
- **Server crash that doesn't recover** within 2 restart-then-ensure_project attempts (R5)
- **`in_progress` bead count sustained ≥7 for >2 min** — bead-leak indicates wrapper failure under load
- **Same recovery recipe needed >3 times in 5 minutes** — symptom of a real bug, not transient flakiness
- **You yourself can't decide between two diagnoses** — escalate rather than guess
- **Pre-flight L fails (hook end-to-end test)** before any agent has spawned — fix the infrastructure before the ramp begins

When you escalate, use the FAIL template in §6. Be specific: paste the exact output (from `/tmp/l5-ramp-hook-test.log` if it's a hook-test failure), the exact bead IDs, the exact agent names, the exact timestamps. Vague escalations waste the user's time.
