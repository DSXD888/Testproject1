# Phase B — Activate Precommit Mail Enforcement

> **Paste this entire file into a fresh Claude Code / Codex session to onboard the implementation agent.**
>
> **PREREQUISITE:** Phase A must already be deployed (`prompts/phase-a-mail-discipline-infrastructure-handoff.md`) AND L4 audit-beads + fix-beads workflows must be completely finished. If audit/fix is still in flight, STOP and wait.

---

## Context

Phase A deployed the infrastructure: `bin/br-claim`, `bin/br-complete`, embedded mail protocol in bead body templates, and AGENTS.md Rule 2. None of those have teeth — they're documentation and convenience. Agents who choose to skip them suffer no consequence.

Phase B activates the teeth. Extend the existing precommit hook so commits with a `<bead-id>:` message prefix require a corresponding `[<bead-id>] Start` mail in the agent-mail archive. This is the same enforcement model that already works for file reservations — we're just extending it to mail.

Why now and not earlier: this hook will block any agent that tries to commit without a Start mail. If we'd activated it during L4 audit/fix, the audit and fix workflows (which were already working under L4-style loose-mail discipline) would have been disrupted. By gating Phase B on "L4 audit/fix complete," L5 starts under enforcement and L4 finishes under the looser regime it began with.

---

## 1. Mission

Extend `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` with a new check function that:

1. Inspects the staged commit message
2. Detects bead-id prefix patterns (e.g. `testproject1-4zq.5.3:`)
3. For each detected bead ID, queries the agent-mail server for a `[<bead-id>] Start` message in the archive
4. **Blocks the commit** if no Start message exists from any registered agent in the last 4 hours
5. Provides an escape hatch (`AGENT_MAIL_BYPASS=1`) for emergency commits
6. Falls back to a non-blocking warning if the agent-mail server is unreachable (don't brick the repo on server downtime)

This is the only deliverable. **Do not modify the wrappers, the skills, AGENTS.md, or any other file.**

**Project key:** `/Users/zacfarrell/Desktop/Testproject1`
**Branch:** `main` only
**Bead label:** N/A — infrastructure work

---

## 2. Pre-flight Checks (do FIRST — abort if any fail)

```bash
cd /Users/zacfarrell/Desktop/Testproject1

# A. Phase A is deployed
test -x bin/br-claim && echo "br-claim OK" || { echo "ABORT: bin/br-claim missing — Phase A not deployed"; exit 1; }
test -x bin/br-complete && echo "br-complete OK" || { echo "ABORT: bin/br-complete missing — Phase A not deployed"; exit 1; }
grep -q "RULE NUMBER 2" AGENTS.md && echo "AGENTS.md OK" || { echo "ABORT: AGENTS.md Rule 2 missing"; exit 1; }

# B. L4 audit/fix is complete (no open beads, no in-progress beads, working tree clean)
br list --label layer-4 --status=in_progress --json 2>&1 | jq '.issues | length' | xargs -I{} test {} -eq 0 || { echo "ABORT: L4 has in-progress beads"; exit 1; }
git status --porcelain | grep -q . && { echo "ABORT: working tree dirty — finish or stash before deploying"; exit 1; } || echo "tree clean OK"

# C. agent-mail server reachable
curl -s --max-time 2 http://127.0.0.1:8765/healthz > /dev/null 2>&1 && echo "agent-mail OK" || echo "WARNING: agent-mail server not reachable — testing harder cases will require it"

# D. Existing hook is the version Phase A left untouched
sha256sum .git/hooks/hooks.d/pre-commit/50-agent-mail.py
# Note this hash; you'll diff against it post-edit to verify only your additions changed.
```

If any of A or B fail, STOP and report to the user. Do not proceed.

---

## 3. AgentMail Registration

Register up front — you'll need to send and search test mails:

```
macro_start_session(
  project_key="/Users/zacfarrell/Desktop/Testproject1",
  program="phase-b-precommit-enforcement",
  model="<your-model>",
  task_description="Extend precommit hook with Start-mail check"
)
```

---

## 4. Required Reading

1. **Existing hook:** `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` — full read (~677 lines). Understand:
   - The HTTP client (how it talks to `127.0.0.1:8765`)
   - The auth flow (any bearer tokens, JWT/JWKS handling)
   - The existing `check_conflicts(paths, reservations)` function — your new check will be its sibling
   - The main `if __name__ == "__main__"` block — how checks are sequenced and how blocks are reported
   - Exit code conventions (0 = pass, non-zero = block)

2. **`bin/br-claim`** (deployed by Phase A) — to understand the exact mail format, subject regex, and `search_messages` API call shape. Your hook check must match the same patterns.

3. **AGENTS.md "RULE NUMBER 2"** — your block message will reference this rule and the wrapper paths.

4. **A handful of recent commit messages** to inform your regex:
   ```bash
   git log --since="2026-05-07" --pretty=format:"%s" | head -50
   ```
   Note which messages have bead-id prefixes (subject to check) vs `beads:`/`wip:`/`chore:` prefixes (exempt).

---

## 5. Implementation Spec

### 5a. The new check function

Add a function near the existing `check_conflicts` (similar style, similar location):

```python
def check_start_mail_for_bead_commits(commit_message: str, server_client) -> List[str]:
    """
    Returns a list of error strings (empty = pass).

    Behavior:
    1. Parse commit_message for any line matching ^([\w-]+-[\w]+\.[0-9]+(\.[0-9]+)*):
       (the bead-id prefix pattern; verify against actual bead IDs in the repo).
       Exclude lines starting with 'beads:'/'wip:'/'chore:'/etc. (those are sync/wip/chore commits, not work commits).
    2. For each detected bead_id:
       a. Query server_client.search_messages(query=f'[{bead_id}] Start', limit=10).
       b. Filter to messages created within the last 4 hours.
       c. If zero matches: append a block error to the result list.
    3. If $AGENT_MAIL_BYPASS=1 in environ: return [] (skip all checks; log to bypass log).
    4. If server is unreachable: return [WARNING-but-don't-block] — log to stderr but don't add to error list.
    """
```

### 5b. The bead-id detection regex

The regex must match commit messages like:

- `testproject1-4zq.6.8: record L4 stage gate walkthrough` ✓
- `testproject1-4zq.5.3: implement web broadcast client` ✓
- `proj-abc.10.2.1: nested numbering` ✓
- `beads: close testproject1-4zq.5.3` ✗ (sync commit, not subject to check)
- `wip: testproject1-4zq.4.3 — handoff` ✗ (wip commit, exempt)
- `chore: add bin/br-claim` ✗ (no bead id)
- `Merge branch 'main'` ✗

Suggested regex (verify against the actual L4 git log to make sure):

```python
import re
BEAD_COMMIT_RE = re.compile(r'^([a-z][a-z0-9-]*-[a-z0-9]+(?:\.[0-9]+)+):\s', re.MULTILINE)
EXEMPT_PREFIXES_RE = re.compile(r'^(beads|wip|chore|docs|fix|feat|test|refactor):', re.MULTILINE)

def extract_bead_ids(commit_message: str) -> List[str]:
    # Skip if first non-blank line is an exempt prefix
    first_line = next((line for line in commit_message.splitlines() if line.strip()), "")
    if EXEMPT_PREFIXES_RE.match(first_line):
        return []
    return BEAD_COMMIT_RE.findall(commit_message)
```

Test against the actual L4 commit log (see §4 item 4). Make sure: bead-prefix commits (e.g. `testproject1-4zq.X.Y: ...`) match; `beads: ...` and `wip: ...` and `chore: ...` do not. Tune regex if needed.

### 5c. The search_messages call

Use the same HTTP client the existing hook uses. The query must be exact-string-match for `[<bead-id>] Start` since that's what `bin/br-claim` produces.

Time-window filter: messages with `created_ts >= now - 4 hours`. Use ISO-8601 timestamps; do not rely on the server's pagination defaults.

### 5d. Block message

When blocking, the hook output should be unmistakable:

```
ERROR: Pre-commit blocked by Agent Mail discipline (AGENTS.md Rule 2).

Commit message references bead `<bead-id>` but no `[<bead-id>] Start` mail was found
in the archive within the last 4 hours.

To fix:
  1. Run: bin/br-claim <bead-id>
  2. Re-attempt the commit.

Emergency bypass (use sparingly, audit later):
  AGENT_MAIL_BYPASS=1 git commit ...

If you believe this is a false positive (server downtime, archive sync lag, etc.),
contact the repo owner.

Detected bead IDs: <comma-separated list>
```

Use the same formatting (color codes, indent, etc.) as the existing `check_conflicts` block message — match the in-repo style.

### 5e. Wire it into main

Find the existing main entry point in `50-agent-mail.py`. After the existing `check_conflicts` call, add your new call:

```python
# After existing reservation check
errors_mail = check_start_mail_for_bead_commits(commit_msg, client)
errors.extend(errors_mail)

if errors:
    print('\n\n'.join(errors), file=sys.stderr)
    sys.exit(1)
```

Preserve the existing exit-code convention (0 = pass, 1 = block).

### 5f. Bypass mechanism

Honor `AGENT_MAIL_BYPASS=1`. Implementation:

```python
if os.environ.get("AGENT_MAIL_BYPASS") == "1":
    print("WARNING: AGENT_MAIL_BYPASS=1 — skipping mail check. This commit is logged.", file=sys.stderr)
    # Log the bypass event to a local file for audit
    log_bypass(commit_msg)
    return []
```

The bypass should be loud (visible warning) but not blocking. Log all bypasses to `.git/agent-mail-bypasses.log` (gitignored — add to `.gitignore` if not already) so the user can audit them later.

### 5g. Server-unreachable fallback

If the HTTP call to the agent-mail server raises a connection error:

```python
except (ConnectionError, urllib.error.URLError, socket.timeout) as e:
    print(f"WARNING: agent-mail server unreachable ({e}). Mail check skipped for this commit.", file=sys.stderr)
    print("WARNING: Consider checking server health: curl http://127.0.0.1:8765/healthz", file=sys.stderr)
    return []  # Do NOT block. Server downtime should not brick the repo.
```

Server-up-but-broken (4xx/5xx that's not connection error) — that's still a block, since it might mean the search_messages API changed.

---

## 6. Test Plan (write a small test script alongside the hook edit)

Create `.git/hooks/hooks.d/pre-commit/test_50-agent-mail.py` (a sibling test file). It does NOT execute on commit — it's a manual run for verification.

Test cases (each must pass before declaring done):

| # | Setup | Expected |
|---|---|---|
| T1 | Commit message: `chore: add foo` | PASS (no bead-id) |
| T2 | Commit message: `beads: close testproject1-4zq.5.3` | PASS (sync commit, exempt) |
| T3 | Commit message: `wip: testproject1-4zq.5.3 — handoff` | PASS (wip commit, exempt) |
| T4 | Commit message: `testproject1-4zq.5.3: do thing`, no Start mail in archive | **BLOCK** with the §5d message |
| T5 | Same as T4, but Start mail exists in archive (created < 4h ago, any agent) | PASS |
| T6 | Same as T4, but Start mail exists from > 4h ago | **BLOCK** (too old) |
| T7 | Same as T4, but `AGENT_MAIL_BYPASS=1` | PASS with warning |
| T8 | Same as T4, but agent-mail server down (kill it temporarily) | PASS with warning (server-unreachable fallback) |
| T9 | Multi-bead commit: `testproject1-4zq.5.3 + 5.4: combined`. 5.3 has Start, 5.4 doesn't. | **BLOCK** (one missing) |

Run each test by:

```bash
echo "<commit message>" | python3 .git/hooks/hooks.d/pre-commit/50-agent-mail.py
echo "exit: $?"
```

Document the test results in your final report.

---

## 7. Acceptance Criteria

- [ ] All 9 test cases (T1–T9) produce expected outcomes
- [ ] The hook still passes the existing file-reservation check tests (whatever those are — preserve the `check_conflicts` codepath unchanged)
- [ ] Hook diff is bounded: `git diff .git/hooks/` shows ONLY additions (a new function, a wire-in line in main, an import or two), no deletions to the existing logic
- [ ] `.gitignore` includes `.git/agent-mail-bypasses.log` (the bypass audit log)
- [ ] A real test commit on a non-trunk branch, then revert, succeeds:
  ```bash
  git checkout -b phase-b-test
  echo "test" > /tmp/foo
  git add /tmp/foo  # or any tracked file change
  git commit -m "testproject1-4zq.test-fake.1: this should block"
  # Expect block message
  AGENT_MAIL_BYPASS=1 git commit -m "testproject1-4zq.test-fake.1: this should pass with bypass"
  # Expect pass with warning
  git checkout main && git branch -D phase-b-test
  ```
- [ ] No L4 commits or beads or other non-hook files modified

---

## 8. Rollback Plan

If after deployment commits start failing unexpectedly:

```bash
# Quick disable (for the active session):
export AGENT_MAIL_BYPASS=1
# OR add to ~/.zshrc / ~/.bashrc temporarily

# Full rollback to pre-Phase-B state:
cd /Users/zacfarrell/Desktop/Testproject1
git log --oneline -5  # Find the Phase B commit hash
git revert <phase-b-commit-sha>
git push
```

Document this rollback procedure in your final report so the user knows what to do if anything breaks.

---

## 9. Commit + Push

One commit:

```
chore(hooks): extend precommit hook to enforce [<bead-id>] Start mail

Per AGENTS.md Rule 2, every commit with a bead-id message prefix must have
a corresponding Start mail in the agent-mail archive. This block kicks in
NOW that L4 audit/fix is complete and Phase A wrappers are deployed.

Bypass: AGENT_MAIL_BYPASS=1 (logged to .git/agent-mail-bypasses.log).
Fallback: server-unreachable does not block (warning only).
```

Files in the commit:

- `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` (extended)
- `.gitignore` (added `.git/agent-mail-bypasses.log` if not present)
- `.git/hooks/hooks.d/pre-commit/test_50-agent-mail.py` (new test file)

Then `git pull --rebase && git push`.

---

## 10. Report Format When Done

```
Phase B complete.

Hook extended: .git/hooks/hooks.d/pre-commit/50-agent-mail.py
  +N lines (new check function), +M lines (test file)

All 9 test cases pass:
  T1 PASS, T2 PASS, T3 PASS, T4 BLOCK, T5 PASS, T6 BLOCK,
  T7 PASS-with-warning, T8 PASS-with-warning, T9 BLOCK

Bypass mechanism: AGENT_MAIL_BYPASS=1, logged to .git/agent-mail-bypasses.log
Server-unreachable fallback: tested, falls through to warning

Commits pushed: <sha>

Mail discipline is now enforced. L5 may begin.

Rollback procedure documented in §8 of the handoff.
```

Stop after reporting.

---

## 11. Critical Rules

1. **DO NOT modify anything other than `.git/hooks/hooks.d/pre-commit/50-agent-mail.py`, `.gitignore`, and the new sibling test file.** Wrappers, skills, AGENTS.md are all Phase A territory and frozen for this session.
2. **DO NOT delete any existing logic** in the precommit hook. Your changes are purely additive — a new function, a wire-in line, optional imports.
3. **DO test all 9 cases manually** before declaring done. The hook will affect every future commit; a buggy hook bricks the repo for everyone.
4. **DO ensure both escape hatches work** — `AGENT_MAIL_BYPASS=1` AND server-unreachable. Both are non-negotiable safety measures.
5. **NEVER stash other agents' work** — concurrent commits on `main` are normal; `git pull --rebase`.
6. **NEVER use `git reset --hard` or `git push --force`** — see AGENTS.md "Irreversible Git Actions".
7. **Work is NOT done until `git push` succeeds** — see AGENTS.md "Landing the Plane".
