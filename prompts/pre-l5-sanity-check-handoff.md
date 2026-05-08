# Pre-L5 Mail-Discipline Sanity Check

> **Paste this entire file into a fresh Claude Code / Codex session BEFORE spawning the L5 implementation swarm.**

---

## Context

`Testproject1` is post-L4 close-out. Phase A (wrappers + skill templates + AGENTS.md Rule 2) and Phase B (precommit hook Start-mail enforcement) are deployed. Empirical proof so far:
- Wrappers exist, `--help` works, closed-bead guard works
- Precommit hook blocked a commit lacking `AGENT_NAME` during the Phase A/B deployment session

What has NOT yet been observed:
- An agent running `bin/br-claim <real-bead-id>` end-to-end against an open bead (file reservation, mail send, br update all succeeding atomically)
- The precommit hook's start-mail subcheck firing against a real `<bead-id>:`-prefixed commit
- `bin/br-complete` actually pushing through commit + mail + br close + sync + release in one flow

If a wrapper has a bug that only surfaces on the live path (not `--dry-run`), spawning 4–6 L5 agents simultaneously means that bug is hit by every agent on their first claim — wasting hours.

This handoff is a **30-minute live smoke test** with a throwaway bead, run BEFORE the L5 swarm spawns. It catches any wrapper bug while the blast radius is one agent on one fake bead, not a swarm on real work.

**Project key:** `/Users/alinaaitova/Desktop/Testproject1`
**Branch:** `main` only
**Bead label:** `smoke-test` (a fresh label that won't collide with `layer-4`, `layer-5`, etc.)

---

## 1. Mission

Run the full `bin/br-claim` → trivial commit → `bin/br-complete` lifecycle against a throwaway test bead, verifying every artifact at each step. If everything passes, report green and L5 is safe to spawn. If anything fails, do NOT spawn L5 — fix the wrapper or hook first, then re-run this handoff.

---

## 2. AgentMail Registration (mandatory — this is bead-tracked work)

Register before any other tool call. The wrappers require `AGENT_NAME` to be set; mail tools require registration:

```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="pre-l5-sanity-check",
  model="<your-model>",
  task_description="Pre-L5 mail-discipline sanity check"
)
# Returns your agent_name. Then in the same shell:
```

```bash
export AGENT_NAME="<your-name>"
# Required by the precommit hook AND by bin/br-claim/bin/br-complete
```

Set `AGENT_MAIL_NAME` too (br-claim's preferred env var per its docstring; falls back to AGENT_NAME if unset, but be explicit):

```bash
export AGENT_MAIL_NAME="<your-name>"
```

---

## 3. Required Reading

In this order:

1. **`AGENTS.md`** — full read. Rule 2 governs what you're testing.
2. **`prompts/phase-a-mail-discipline-infrastructure-handoff.md`** — what each wrapper claims to do.
3. **`prompts/phase-b-precommit-mail-enforcement-handoff.md`** — what the hook claims to do.
4. **`bin/br-claim --help` and `bin/br-complete --help`** — confirm the wrappers' flags are what this handoff expects.
5. **The first 30 lines of `bin/br-claim`** — read the docstring; confirm the order is "file-reservation FIRST, then mail, then br update."
6. **The first 30 lines of `.git/hooks/hooks.d/pre-commit/50-agent-mail.py`** — confirm Phase B's `check_start_mail_for_bead_commits` is wired in.

---

## 4. Pre-Flight (fail-hard if any of these fail)

```bash
cd /Users/alinaaitova/Desktop/Testproject1

# A. Working tree must be CLEAN
git status --porcelain | grep -q . && { echo "ABORT: working tree dirty"; exit 1; } || echo "tree clean OK"

# B. Wrappers exist + executable
test -x bin/br-claim    && echo "br-claim OK"
test -x bin/br-complete && echo "br-complete OK"

# C. Hook present and Phase B markers in it
grep -q "check_start_mail_for_bead_commits" .git/hooks/hooks.d/pre-commit/50-agent-mail.py && echo "Phase B marker OK"

# D. Server reachable
curl -s --max-time 3 http://127.0.0.1:8765/healthz | grep -q alive && echo "server OK"

# E. Env vars set
[ -n "$AGENT_NAME" ] && echo "AGENT_NAME=$AGENT_NAME OK"
[ -n "$AGENT_MAIL_NAME" ] && echo "AGENT_MAIL_NAME=$AGENT_MAIL_NAME OK"

# F. Git remote sync
git fetch && git status -sb
# Expected: "## main...origin/main" with no ahead/behind
```

If any line fails, STOP. Report which check failed and the exit code/message.

---

## 5. Create the Throwaway Test Bead

We need a real bead in the `br` store to test against. Use a unique label so it can't collide with real work.

```bash
br create \
  --title="L5 mail-discipline sanity check" \
  --type=task \
  --priority=4 \
  --description="$(cat <<'EOF'
Throwaway bead for verifying bin/br-claim and bin/br-complete work end-to-end
before spawning the L5 swarm. Will be claimed, trivially "implemented" by
appending one line to docs/scratch-smoke-test.md, and closed within minutes.

## Files to Create/Modify
- docs/scratch-smoke-test.md (create or append)

## Acceptance Criteria
- bin/br-claim succeeds with no errors
- Start mail visible in archive
- File reservation acquired
- Trivial commit lands with bead-id prefix
- bin/br-complete succeeds with no errors
- Complete mail visible in archive
- bead transitions open → in_progress → closed
- File reservation released
EOF
)" \
  --labels=smoke-test
```

Capture the bead ID returned (e.g. `testproject1-xyzN`). Save it for the rest of the procedure:

```bash
SMOKE_BEAD=<the-id-returned>
echo "Smoke test bead: $SMOKE_BEAD"
```

`bin/br-sync && git add .beads/ && git commit -m "beads: create $SMOKE_BEAD smoke-test" && git pull --rebase && git push`

(This commit doesn't have a bead-id-prefix, it has the `beads:` exempt prefix, so the precommit hook won't demand a Start mail for it.)

---

## 6. Live Test Procedure

### Step 6a — Dry-run claim

```bash
bin/br-claim "$SMOKE_BEAD" --dry-run
```

**Expected:** Wrapper prints intentions (would reserve docs/scratch-smoke-test.md, would send Start mail, would set status=in_progress) and exits 0. NO actual changes. Verify:

```bash
# After dry-run, bead should still be open (no real claim happened)
br show "$SMOKE_BEAD" --json | grep -E '"status"' | head -1
# Expected: "status":"open"
```

If dry-run fails or makes real changes: **STOP, do not proceed.** Report the error, do not spawn L5.

### Step 6b — Real claim

```bash
bin/br-claim "$SMOKE_BEAD"
```

**Expected:** Wrapper exits 0 with summary line like `Claimed: $SMOKE_BEAD · Files reserved: 1 · Mail sent · br=in_progress`.

Verify all four post-conditions:

```bash
# 1. Bead status is now in_progress
br show "$SMOKE_BEAD" --json | grep -E '"status"' | head -1
# Expected: "status":"in_progress"

# 2. File reservation acquired (tooling-dependent — try the MCP layer)
# Use mcp__mcp-agent-mail__file_reservation_paths or list_active_reservations,
# OR check the agent-mail server's reservation log
```

In your agent session, also call:
```
search_messages(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  query="[<SMOKE_BEAD>] Start"
)
```

**Expected:** at least one message returned, sender = your agent_name, subject `[<SMOKE_BEAD>] Start: L5 mail-discipline sanity check` (or similar), created within the last 60 seconds.

If any of these fail: **STOP, do not proceed.** The wrappers have a bug. Report findings.

### Step 6c — Trivial implementation

```bash
echo "$(date -u +%FT%TZ) — smoke test marker" >> docs/scratch-smoke-test.md
git add docs/scratch-smoke-test.md
```

Then attempt the commit with the bead-id prefix. **The precommit hook should ACCEPT this** because the Start mail exists:

```bash
git commit -m "$SMOKE_BEAD: smoke-test marker"
```

**Expected:** commit succeeds (exit 0). Hook may print informational lines but does not block.

If the hook BLOCKS this commit despite the Start mail being in the archive: **STOP, do not proceed.** The Phase B start-mail check has a bug (lookback window too tight, agent-name mismatch, etc.). Report what the hook said.

### Step 6d — Negative test (the hook BLOCKS without a Start mail)

In a different scratch file, attempt a commit using a DIFFERENT bead-id you haven't claimed:

```bash
echo "negative test" >> docs/scratch-smoke-test.md
git add docs/scratch-smoke-test.md
git commit -m "testproject1-fake.999.999: deliberately unclaimed"
```

**Expected:** the precommit hook BLOCKS this with the Phase B error message ("Pre-commit blocked by Agent Mail discipline (AGENTS.md Rule 2)..."). Exit code non-zero.

Then unstage the negative test:

```bash
git restore --staged docs/scratch-smoke-test.md
git checkout docs/scratch-smoke-test.md  # remove the "negative test" line; keep the date marker
```

If the hook does NOT block this: **STOP, do not proceed.** Phase B's Start-mail check is not firing correctly. Report what happened.

### Step 6e — Push the legitimate commit

```bash
git pull --rebase
git push
```

**Expected:** Push succeeds; commit lands on origin/main.

### Step 6f — Complete via wrapper

```bash
bin/br-complete "$SMOKE_BEAD"
```

**Expected:** Wrapper exits 0. Verify all five post-conditions:

```bash
# 1. Bead status is closed
br show "$SMOKE_BEAD" --json | grep -E '"status"' | head -1
# Expected: "status":"closed"

# 2. Working tree clean
git status --porcelain | wc -l   # expect 0

# 3. main pushed to origin
git rev-list --count origin/main..HEAD   # expect 0

# 4. Complete mail in archive
```

In your agent session:
```
search_messages(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  query="[<SMOKE_BEAD>] Complete"
)
```

**Expected:** at least one message, sender = your agent_name, subject `[<SMOKE_BEAD>] Complete: ...`, body mentioning the commit hash.

```
# 5. File reservations released — check via mail tool
list_contacts / list_active_reservations
```

If `bin/br-complete` fails or any verification fails: **STOP**, but in this case the bead is already in flight so do NOT just abandon it. Instead:
- Run `git push` manually if not yet pushed
- Run `br close $SMOKE_BEAD --reason "Smoke test complete — wrapper had issue at <step>"` manually
- Manually send the Complete mail (the agent-mail tools)
- Release reservations manually
- Report findings

---

## 7. Acceptance Criteria

The full smoke test PASSES when ALL of these are true:

- [ ] Pre-flight (§4) all green
- [ ] Smoke bead created and visible in `br show`
- [ ] `bin/br-claim --dry-run` exits 0 with no side effects
- [ ] `bin/br-claim` (real) exits 0; status flips to `in_progress`; Start mail visible in archive; file reservation acquired
- [ ] Trivial commit with `$SMOKE_BEAD:` prefix is ACCEPTED by the precommit hook
- [ ] Commit with a fake bead-id `testproject1-fake.999.999:` is BLOCKED by the precommit hook
- [ ] `git push` succeeds
- [ ] `bin/br-complete` exits 0; status flips to `closed`; Complete mail visible; reservations released
- [ ] Working tree ends clean; main is in sync with origin/main
- [ ] No errors, no Python tracebacks, no "command not found"

If ANY criterion fails: REPORT the specific failure point with the exact error output. **Do not spawn the L5 swarm.** The user will decide whether to fix-and-retest or escalate.

---

## 8. Cleanup (always run, even if test passed)

The smoke bead should be left in its closed state (per AGENTS.md Rule 1, no deletions). The scratch file `docs/scratch-smoke-test.md` will be a 1-line file that's part of the commit history — that's fine; it serves as historical evidence the smoke test ran.

If you want to keep things tidy:
- Confirm the smoke bead is `closed` and labeled `smoke-test`
- Confirm the scratch file is committed and pushed (no orphan changes in working tree)
- Optionally: add a `br comment` to the smoke bead linking to this handoff for future reference

---

## 9. Verification Snapshot (run before reporting)

```bash
# Smoke bead final state
br show "$SMOKE_BEAD" --json | jq -r '..|objects|select(.id=="'"$SMOKE_BEAD"'")|"id=\(.id) status=\(.status) updated=\(.updated_at)"' | head -1

# Working tree clean
git status -sb

# Origin sync
git rev-list --left-right --count origin/main...HEAD | awk '{print "behind:",$1," ahead:",$2}'

# Mail count for the smoke bead
# (in agent session: search_messages query="[<SMOKE_BEAD>]" — should return at least 2 messages: Start and Complete)
```

---

## 10. Report Format When Done

### If smoke test PASSED

```
Pre-L5 mail-discipline sanity check: PASS

Smoke bead: <SMOKE_BEAD>
Status: closed
Mails sent: 2 (Start, Complete) — both visible via search_messages
Hook behavior:
  Acceptable commit (with Start mail in archive): ACCEPTED ✓
  Unauthorized commit (fake bead-id, no Start mail): BLOCKED ✓
File reservations: acquired and released cleanly
Working tree: clean
Origin sync: 0 ahead / 0 behind

Wrapper end-to-end flow verified.
L5 swarm is safe to spawn.

Recommended next step: paste the L5 kickoff prompt into the first agent
session and observe its first bead claim. If that succeeds, proceed to
spawn the rest of the swarm.
```

### If smoke test FAILED

```
Pre-L5 mail-discipline sanity check: FAIL at step <6a/6b/6c/6d/6e/6f>

Smoke bead: <SMOKE_BEAD>
Failure point: <which step>
Exact output:
<paste the wrapper or hook output verbatim>

Hypothesis (your best guess at root cause): <e.g., wrapper bug in mail-send,
Phase B regex matched too eagerly, server returned unexpected JSON shape>

State of artifacts:
  Bead: <status>
  Working tree: <clean / dirty — list>
  Mails: <Start present? Complete present?>
  Reservations: <held / released>

Recommended action: <e.g., "fix the bug in bin/br-claim line N", "verify
the Phase B regex against this commit subject pattern", "check the agent-
mail server logs">

L5 swarm is NOT safe to spawn until this is resolved.
```

Stop after reporting.

---

## 11. Critical Rules

1. **Do NOT spawn the L5 swarm if anything in this smoke test fails.** A wrapper bug at single-agent scale becomes a swarm-wide outage.
2. **The smoke bead is a real bead** — it lives in `br ready`, gets file reservations, sends real mail. Treat it like any other bead. Don't try to delete it (Rule 1 forbids deletion); just close it cleanly.
3. **`AGENT_MAIL_BYPASS=1` is OFF-LIMITS for this handoff.** The whole point is to verify the hook fires correctly. Bypassing defeats the test.
4. **If you hit a wrapper bug mid-flow, recover manually.** Do not abandon the smoke bead in `in_progress` state. Either complete the lifecycle manually (br close + mail + release) or escalate via report with the bead in a known recoverable state.
5. **NEVER stash other agents' work** — concurrent commits on `main` are normal; `git pull --rebase`.
6. **NEVER `git reset --hard` or `git push --force`** — see AGENTS.md "Irreversible Git Actions".
7. **Single-agent procedure.** Do not spawn parallel agents to "test concurrency" in this handoff — that's a bigger smoke test that should be a separate handoff if you want it. This handoff verifies the single-agent path; multi-agent collision testing is out of scope.
8. **Push to origin before declaring done.** Per AGENTS.md, work isn't done until `git push` succeeds.
