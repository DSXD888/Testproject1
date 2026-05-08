# Pre-L5 Wrapper Fix + Re-Smoke Loop

> **Paste this entire file into a fresh Claude Code OR Codex session.**
> **Do not mark this work complete until the validate→fix→test loop reaches green AND the L5 swarm is safe to spawn.**
>
> **CLI-agnostic.** This handoff works identically from Claude Code (with the `Edit` tool) and Codex (with `apply_patch`). All other tooling — MCP agent-mail calls, bash, Python reproducer, git, `br` — is shared. Skill-template fixes in Appendix A.1 must be applied to BOTH `~/.claude/skills/<skill>/` AND `~/.codex/skills/<skill>/` per the Phase A two-CLI protocol — they are separate file copies, not symlinks.

---

## 0. TL;DR for the operator

`bin/br-claim` and `bin/br-complete` mis-parse the `search_messages` JSON-RPC response. The agent-mail server returns matches under the key `"result"` (singular), but both wrappers look up `"messages"` or `"results"` (plural). Effects:

- **`bin/br-complete` is fully broken on the live path** — it can never find the Start mail it requires before sending Complete, so every L5 agent would succeed at claim and then fail at complete, leaving every bead stuck `in_progress`.
- **`bin/br-claim` idempotency is silently broken** — partial-failure recovery (where `br update` failed after Start mail sent) would send a duplicate Start mail every retry instead of detecting the prior send.

The reference correct pattern already exists in `.git/hooks/hooks.d/pre-commit/50-agent-mail.py:771-781`. We are mirroring that pattern into the two wrappers, then re-running the full smoke test against a fresh throwaway bead, looping on any failure until green.

A prior smoke-test session (2026-05-08, agent `IvoryMaple`, smoke bead `testproject1-l9vh`) discovered this and recovered cleanly. State at handoff time:

- Smoke bead `testproject1-l9vh` is `closed`. **Do NOT delete it** (Rule 1) — it is audit evidence.
- `docs/scratch-smoke-test.md` exists with one historical line. **Do NOT delete or overwrite it** — append only.
- All commits are pushed to `origin/main`. Working tree is clean.
- Wrappers still contain the bug. Hook is correct (it's the reference).

---

## 1. Mission

Three phases, then loop until acceptance criteria are all green.

1. **Validate (Phase V).** Reproduce the bug with a stand-alone Python script. Do not trust the report — verify by independent reproducer. Confirm the wrapper code exactly matches the buggy lines described here.
2. **Fix (Phase F).** Apply identical patches to `bin/br-claim` and `bin/br-complete`. Sanity-check at the unit level by re-running the reproducer and proving the lookup now finds the Start mail.
3. **Test (Phase T).** Run the full smoke test against a **fresh** throwaway bead (DO NOT reuse `testproject1-l9vh`). Includes an explicit idempotency check that exercises the previously-broken code path.

If anything in Phase T fails — even something unrelated to the original bug — diagnose, fix in-band if minor, and **re-run Phase T from the top**. Don't claim done until every acceptance criterion in §10 is true.

**Project key:** `/Users/alinaaitova/Desktop/Testproject1`
**Branch:** `main` only
**Bead label:** `smoke-test` (the new fresh bead reuses this label; it accumulates with the prior closed smoke bead, which is by design — see §11 of the prior pre-l5 sanity-check handoff)

---

## 2. AgentMail Registration (mandatory)

Before any other tool call. The wrappers and the precommit hook both require `AGENT_NAME` to be set:

```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="pre-l5-wrapper-fix",
  model="<your-model>",
  task_description="Fix search_messages response-key parsing in bin/br-claim and bin/br-complete; re-run smoke test until green"
)
# Returns your agent_name. Then in the same shell:
```

```bash
export AGENT_NAME="<your-name>"
export AGENT_MAIL_NAME="<your-name>"
```

> **Reminder:** the Bash tool does not persist shell state between calls. Either source a tiny env script at the start of every command, or prepend `AGENT_NAME=<name> AGENT_MAIL_NAME=<name>` to every command that needs them (`git commit`, `git push`, `bin/br-claim`, `bin/br-complete`). The pre-push hook also requires `AGENT_NAME` — do not omit it on `git push`.

---

## 3. Required Reading (do this before touching any file)

1. **`AGENTS.md`** — full read. You will commit to `main` directly with `fix:` / `beads:` prefixes; both are exempt from the Phase B Start-mail check, so the work itself does not need a separate claim. Rule 1 (no deletion) and Rule 2 (use the wrappers) still apply.
2. **`prompts/pre-l5-mail-discipline-sanity-check-handoff.md`** — the prior smoke-test handoff. You will re-run a near-identical procedure in Phase T. Read so you know what each step proves.
3. **`bin/br-claim` lines 87-148 (`mcp_call`) and 262-292 (idempotency block)** — confirm the buggy lookup is on **line 272** and the unwrap path matches what the reproducer in §6 mimics.
4. **`bin/br-complete` lines 268-298 (Start-mail check)** — confirm the buggy lookup is on **line 276** and the failure-mode is the `die(...)` at lines 293-298.
5. **`.git/hooks/hooks.d/pre-commit/50-agent-mail.py` lines 711-781** — the **reference correct implementation**. Note: it iterates `("result", "messages", "results")` and explicitly checks `isinstance(value, list)`. This is the pattern you are mirroring into the wrappers. **Do NOT modify the hook** — it already works.

After reading, you should be able to answer in one sentence each: *Why does the bug only manifest in `br-complete` during normal operation, even though `br-claim` shares the buggy line?* and *Why is re-running `bin/br-claim <same-bead>` not a valid idempotency test?* (Hint: status check on line 255 dies before the idempotency check fires.)

---

## 4. Pre-flight Checks (carry over from prior handoff — fail-hard on any ABORT)

Do not use `exit 1` inside an inline shell snippet — it terminates your bash session. Just observe and STOP-and-report on any ABORT.

```bash
cd /Users/alinaaitova/Desktop/Testproject1

# A. tree clean
if [ -n "$(git status --porcelain)" ]; then
  echo "ABORT: working tree dirty:"; git status --short
else echo "OK tree clean"; fi

# B. wrappers exist
test -x bin/br-claim    && echo "OK bin/br-claim"    || echo "ABORT: bin/br-claim missing or not executable"
test -x bin/br-complete && echo "OK bin/br-complete" || echo "ABORT: bin/br-complete missing or not executable"

# C. hook unchanged (Phase B marker)
grep -q "check_start_mail_for_bead_commits" .git/hooks/hooks.d/pre-commit/50-agent-mail.py \
  && echo "OK Phase B marker present" \
  || echo "ABORT: Phase B marker missing"

# D. server alive (strict match on alive payload, not just substring)
curl -s --max-time 3 http://127.0.0.1:8765/healthz | grep -q '"status":"alive"' \
  && echo "OK agent-mail server alive" \
  || echo "ABORT: agent-mail server not alive — restart with: am serve-http --port 8765 &"

# E. env vars (must be set in your tool's shell)
if [ -z "$AGENT_NAME" ];      then echo "ABORT: AGENT_NAME not set";      else echo "OK AGENT_NAME=$AGENT_NAME"; fi
if [ -z "$AGENT_MAIL_NAME" ]; then echo "ABORT: AGENT_MAIL_NAME not set"; else echo "OK AGENT_MAIL_NAME=$AGENT_MAIL_NAME"; fi

# F. origin sync
git fetch
ah=$(git rev-list --left-right --count origin/main...HEAD)
behind=$(echo $ah | awk '{print $1}'); ahead=$(echo $ah | awk '{print $2}')
[ "$behind" = "0" ] && [ "$ahead" = "0" ] \
  && echo "OK in sync with origin/main" \
  || echo "ABORT: not in sync (behind=$behind, ahead=$ahead) — pull/push first"
```

**CHECKPOINT pre-flight:** report to the user — if any line printed `ABORT:`, STOP and ask. Otherwise proceed to Phase V.

---

## 5. Confirm the bug location is what the report says

Before reproducing the runtime bug, eyeball the source. The fix is mechanical, so this guards against the case where someone already patched it (in which case Phase F is a no-op and you proceed straight to Phase T to confirm).

```bash
echo "=== br-claim line 272 (idempotency lookup) ==="
sed -n '270,274p' bin/br-claim

echo "=== br-complete line 276 (Start-mail lookup) ==="
sed -n '274,278p' bin/br-complete
```

**Expected pre-fix output (the bug):**

```
=== br-claim line 272 (idempotency lookup) ===
    messages = []
    if isinstance(search_res, dict):
        messages = search_res.get("messages") or search_res.get("results") or []
    elif isinstance(search_res, list):
        messages = search_res
=== br-complete line 276 (Start-mail lookup) ===
    messages = []
    if isinstance(search_res, dict):
        messages = search_res.get("messages") or search_res.get("results") or []
    elif isinstance(search_res, list):
        messages = search_res
```

If you see `("result", "messages", "results")` (with `result` first), the patch is already applied — skip Phase F and go straight to Phase T. Note in your report that the fix was already in place at session start.

If you see something other than either form, STOP and report — the source has drifted in a way this handoff doesn't anticipate.

---

## 6. Phase V — Validate the bug by reproducer

Save and run this stand-alone reproducer. It mirrors the exact unwrap path of `mcp_call` in `bin/br-claim:87-148` and queries `search_messages` for a known-good thread (`testproject1-l9vh` from the prior smoke test, whose Start mail id=213 is in the archive).

```bash
python3 - <<'PYEOF'
import json, urllib.request

URL = "http://127.0.0.1:8765/mcp/"
BEAD = "testproject1-l9vh"  # prior smoke bead — Start mail is in archive

body = json.dumps({
    "jsonrpc": "2.0", "id": 1, "method": "tools/call",
    "params": {"name": "search_messages", "arguments": {
        "project_key": "/Users/alinaaitova/Desktop/Testproject1",
        "query": f"[{BEAD}] Start", "limit": 20,
    }},
}).encode()
req = urllib.request.Request(URL, data=body, method="POST", headers={
    "Content-Type": "application/json",
    "Accept": "application/json, text/event-stream",
})
with urllib.request.urlopen(req, timeout=30) as resp:
    raw = resp.read().decode("utf-8", "replace").strip()
if raw.startswith(("event:", "data:")):
    for line in raw.splitlines():
        if line.startswith("data:"):
            raw = line[5:].strip(); break

envelope = json.loads(raw)
result = envelope.get("result", {}) or {}
content = result.get("content", []) or []
if content and isinstance(content[0], dict) and "text" in content[0]:
    payload = json.loads(content[0]["text"])
else:
    payload = result

print("payload type :", type(payload).__name__)
print("payload keys :", list(payload.keys()) if isinstance(payload, dict) else "(list)")

# Wrapper-style lookup (the bug):
buggy = payload.get("messages") or payload.get("results") or []
print("BUGGY lookup .get('messages') or .get('results'):", "FOUND" if buggy else "EMPTY (BUG REPRODUCED)")

# Correct lookup (the fix):
fixed = []
if isinstance(payload, dict):
    for key in ("result", "messages", "results"):
        v = payload.get(key)
        if isinstance(v, list):
            fixed = v; break
print("FIXED lookup ('result','messages','results'):", "FOUND" if fixed else "EMPTY")
if fixed:
    print("  first match:", fixed[0].get("subject"), "from", fixed[0].get("from"))
PYEOF
```

**Expected if bug is present (pre-fix):**

```
payload type : dict
payload keys : ['result']
BUGGY lookup .get('messages') or .get('results'): EMPTY (BUG REPRODUCED)
FIXED lookup ('result','messages','results'): FOUND
  first match: [testproject1-l9vh] Start: L5 mail-discipline sanity check from <prior-agent-name>
```

If the `BUGGY lookup` line shows `FOUND`, the server response shape has changed since the report was written — STOP and re-investigate before applying the patch. The patch as written assumes the singular-`result` shape.

If the `FIXED lookup` line shows `EMPTY`, the prior smoke bead's Start mail was somehow purged (very unlikely). STOP and report.

If output matches expected: bug confirmed at protocol layer. Proceed to Phase F.

---

## 7. Phase F — Apply the fix

Two files. **Identical change in both.** Apply each patch with your CLI's native file-edit tool — Claude Code's `Edit`, Codex's `apply_patch`, or whatever your harness exposes — never `sed`, regex-replace scripts, or any other scripted code rewriter per AGENTS.md "No Script-Based Changes".

(Optional but recommended) reserve the wrappers so a parallel agent doesn't collide:

```
file_reservation_paths(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  paths=["bin/br-claim", "bin/br-complete"],
  ttl_seconds=1800,
  exclusive=true,
  reason="pre-l5-wrapper-fix"
)
```

### 7a. Patch `bin/br-claim` (the idempotency lookup)

**File:** `bin/br-claim`
**Approximate lines:** 270-274 (in the function `main`, immediately after `search_res = mcp_call(... "search_messages" ...)`).

Replace:

```python
    prior_self_start = False
    prior_other_start = None
    messages = []
    if isinstance(search_res, dict):
        messages = search_res.get("messages") or search_res.get("results") or []
    elif isinstance(search_res, list):
        messages = search_res
```

with:

```python
    prior_self_start = False
    prior_other_start = None
    messages = []
    if isinstance(search_res, dict):
        for key in ("result", "messages", "results"):
            value = search_res.get(key)
            if isinstance(value, list):
                messages = value
                break
    elif isinstance(search_res, list):
        messages = search_res
```

The pattern matches `mcp_search_messages` in `.git/hooks/hooks.d/pre-commit/50-agent-mail.py:771-781` exactly.

### 7b. Patch `bin/br-complete` (the Start-mail-required lookup)

**File:** `bin/br-complete`
**Approximate lines:** 274-278 (function `main`, immediately after `search_res = mcp_call(... "search_messages" ...)`).

Replace:

```python
    messages = []
    if isinstance(search_res, dict):
        messages = search_res.get("messages") or search_res.get("results") or []
    elif isinstance(search_res, list):
        messages = search_res
```

with:

```python
    messages = []
    if isinstance(search_res, dict):
        for key in ("result", "messages", "results"):
            value = search_res.get(key)
            if isinstance(value, list):
                messages = value
                break
    elif isinstance(search_res, list):
        messages = search_res
```

### 7c. Sanity-check the patches

Both wrappers must still parse and `--help` cleanly:

```bash
python3 -c "import ast; ast.parse(open('bin/br-claim').read()); print('OK br-claim parses')"
python3 -c "import ast; ast.parse(open('bin/br-complete').read()); print('OK br-complete parses')"
bin/br-claim --help    >/dev/null && echo "OK br-claim --help"
bin/br-complete --help >/dev/null && echo "OK br-complete --help"
```

Re-run the Phase V reproducer using the **wrapper code path itself** (not just the inline reproducer) — invoke a tiny Python that imports nothing but mirrors the patched lookup against a real response:

```bash
python3 - <<'PYEOF'
# Re-prove the patched logic finds the Start mail.
import json, urllib.request
body = json.dumps({"jsonrpc":"2.0","id":1,"method":"tools/call",
    "params":{"name":"search_messages","arguments":{
        "project_key":"/Users/alinaaitova/Desktop/Testproject1",
        "query":"[testproject1-l9vh] Start","limit":20}}}).encode()
req = urllib.request.Request("http://127.0.0.1:8765/mcp/", data=body, method="POST",
    headers={"Content-Type":"application/json","Accept":"application/json, text/event-stream"})
raw = urllib.request.urlopen(req, timeout=30).read().decode("utf-8","replace").strip()
if raw.startswith(("event:","data:")):
    for line in raw.splitlines():
        if line.startswith("data:"): raw = line[5:].strip(); break
env = json.loads(raw)
result = env.get("result", {}) or {}
content = result.get("content", []) or []
search_res = json.loads(content[0]["text"]) if (content and "text" in content[0]) else result

# patched logic mirrored verbatim:
messages = []
if isinstance(search_res, dict):
    for key in ("result", "messages", "results"):
        v = search_res.get(key)
        if isinstance(v, list): messages = v; break
elif isinstance(search_res, list):
    messages = search_res
print("patched-logic match count:", len(messages))
print("first subject:", messages[0]["subject"] if messages else "(none)")
PYEOF
```

Expect `patched-logic match count: >=1` and the first subject containing `[testproject1-l9vh] Start`. If empty, the patch was applied incorrectly — re-edit and re-run.

### 7d. (Optional) audit for the same bug elsewhere in the repo

This is cheap insurance. Look for any other file that does the same wrong lookup against an mcp `search_messages` response:

```bash
grep -rnE 'get\("messages"\)|get\("results"\)' bin/ scripts/ 2>/dev/null
grep -rnE 'search_res' bin/ scripts/ 2>/dev/null
```

If anything else surfaces, audit it the same way. The hook (already correct) is excluded — do not edit it.

### 7e. Release the wrapper reservations

After both patches are in and verified:

```
release_file_reservations(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  paths=["bin/br-claim", "bin/br-complete"]
)
```

### 7f. Commit the fix (do NOT push yet — the smoke test will push together)

You can either commit now (preferred — atomic fix commit, easy to revert if needed) or stage and commit at the end of Phase T. Recommended: commit now with a `fix:` prefix (exempt from the Phase B Start-mail check):

```bash
git add bin/br-claim bin/br-complete
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> \
git commit -m "fix(wrappers): include 'result' in search_messages response-key chain

bin/br-claim and bin/br-complete were looking up matches via
search_res.get('messages') or search_res.get('results'), but the
agent-mail server returns matches under the singular 'result' key.

Effects of the prior bug:
- bin/br-complete would die with 'no Start mail from <agent>' even
  when the Start mail was present in the archive (verified via
  smoke test on 2026-05-08, smoke bead testproject1-l9vh).
- bin/br-claim idempotency was silently broken — partial-failure
  retries would send duplicate Start mails.

Fix mirrors the canonical pattern already used in
.git/hooks/hooks.d/pre-commit/50-agent-mail.py:771-781 — iterate
('result', 'messages', 'results') and validate isinstance list."
```

> Don't push yet. We'll push after the smoke-test code commit so origin/main moves once with both changes already covered by the smoke test.

---

## 8. Phase T — Re-smoke (full lifecycle, augmented with idempotency)

Carry over the procedure from the prior pre-l5-mail-discipline-sanity-check handoff with three tweaks:

1. **Use a fresh smoke bead.** Do NOT reuse `testproject1-l9vh`. Create a new one.
2. **Use backticks around the file path** in the bead body, so `bin/br-claim`'s path-parser regex (`PATH_BACKTICK_RE`) succeeds without `--paths` override. (Bug 2 from the prior report — see Appendix A. We work around it here, leave fixing for later.)
3. **Inject an idempotency probe** between Phase T steps b and c (a manual `[<bead>] Start` mail injected into a second fresh bead, then `bin/br-claim` against that bead with the prior Start already present, asserting no duplicate).

### 8a. Create a fresh smoke bead (with backticked path)

```bash
SMOKE_BEAD=$(br create \
  --title="Pre-L5 wrapper-fix smoke test" \
  --type=task \
  --priority=4 \
  --description="$(cat <<'EOF'
Throwaway bead for verifying the wrapper response-key fix end-to-end.
Will be claimed, trivially implemented by appending one line to
docs/scratch-smoke-test.md, and closed within minutes.

## Files to Create/Modify
- `docs/scratch-smoke-test.md`

## Acceptance Criteria
- bin/br-claim succeeds without --paths override (backticked path)
- Start mail visible in archive
- File reservation acquired
- Trivial commit lands with bead-id prefix (hook accepts)
- Fake bead-id commit blocked (hook negative test)
- bin/br-complete succeeds (no longer dies on Start-mail lookup)
- Complete mail visible in archive
- Bead transitions open -> in_progress -> closed
- File reservation released
EOF
)" \
  --labels=smoke-test \
  --json | jq -r '.id')

echo "SMOKE_BEAD=$SMOKE_BEAD"
test -n "$SMOKE_BEAD" || echo "ABORT: failed to capture bead ID"
echo "$SMOKE_BEAD" > /tmp/wrapper_fix_smoke_bead.txt

bin/br-sync && git add .beads/
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> \
  git commit -m "beads: create $SMOKE_BEAD pre-l5 wrapper-fix smoke test"
```

### 8b. Step T-a: dry-run claim (no `--paths` override; backticks should be enough)

```bash
SMOKE_BEAD=$(cat /tmp/wrapper_fix_smoke_bead.txt)
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> \
  bin/br-claim "$SMOKE_BEAD" --dry-run
```

**Expected:** `paths = ['docs/scratch-smoke-test.md']` is parsed from the bead body, `DRY-RUN — no changes made.`, exit 0.

If the wrapper still complains "could not determine files to reserve", the path-parser regex didn't match the backticked form — confirm the bead body actually has the backticks (`` ` `` characters), then re-run. If still failing, fall back to `--paths docs/scratch-smoke-test.md` and add a note to your final report (Appendix A bug still bites).

### 8c. Step T-b: real claim

```bash
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> \
  bin/br-claim "$SMOKE_BEAD"
```

**Expected:** wrapper exits 0, prints `✓ reserved 1 path(s)`, `✓ sent Start mail to [...]`, `✓ br update ... --status=in_progress`, summary `DONE  · ...`.

**Verify all four post-conditions** (run all four, do not skip any):

```bash
# 1. status
br show "$SMOKE_BEAD" --json | jq -r '..|objects|select(.id=="'"$SMOKE_BEAD"'")|.status' | head -1
# expect: in_progress

# 2. Start mail in archive (use search_messages MCP tool)
search_messages(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  query="[<SMOKE_BEAD>] Start", ranking="recency", limit=5
)
# expect: 1 result, from=<your-name>, subject contains "[<SMOKE_BEAD>] Start"

# 3. file reservation (artifact on disk)
ls -lt /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/projects/users-alinaaitova-desktop-testproject1/file_reservations/ | head -3
# Then jq the most recent id-N.json — expect agent=<your-name>, reason=<SMOKE_BEAD>

# 4. wrapper exit code (already 0 if you got here)
```

(`am file_reservations active <project>` works only when the agent-mail server is offline — it grabs an exclusive lock that conflicts with the running server. The on-disk JSON is the durable artifact and is what to check while the server runs.)

### 8d. Step T-c (NEW): idempotency probe — manual prior-Start, then claim

This is the test of the previously-broken `bin/br-claim` code path. It only fires when the bead is still `open` AND a prior Start exists from the same agent — a partial-failure shape we synthesize manually here.

```bash
# Create a SECOND fresh smoke bead — keep it open.
IDEM_BEAD=$(br create \
  --title="Pre-L5 wrapper-fix idempotency probe" \
  --type=task --priority=4 \
  --description="$(cat <<'EOF'
Idempotency probe bead. Stays open. We send a Start mail manually,
then run bin/br-claim — wrapper must detect the prior Start and skip
the re-send (proving the response-key fix is wired into idempotency).

## Files to Create/Modify
- `docs/scratch-smoke-test.md`
EOF
)" \
  --labels=smoke-test --json | jq -r '.id')
echo "IDEM_BEAD=$IDEM_BEAD"
echo "$IDEM_BEAD" > /tmp/wrapper_fix_idem_bead.txt
bin/br-sync && git add .beads/
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> \
  git commit -m "beads: create $IDEM_BEAD pre-l5 idempotency probe"
```

Now manually inject a Start mail (DO NOT call `bin/br-claim` yet):

```
send_message(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  sender_name="<your-name>",
  to=["<your-name>"],
  thread_id="<IDEM_BEAD>",
  subject="[<IDEM_BEAD>] Start: Pre-L5 wrapper-fix idempotency probe",
  body_md="Manually-injected Start mail to set up the idempotency-probe scenario.",
  ack_required=false
)
```

Confirm the Start is searchable:

```
search_messages(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  query="[<IDEM_BEAD>] Start", limit=5
)
# expect: count = 1
```

Now run `bin/br-claim` against this bead. The wrapper should:
- See `status=open` (passes the line-255 status check)
- Find the prior Start from us via the **patched** lookup
- Set `prior_self_start = True`
- Skip the `send_message` call (line 340: `if not prior_self_start:`)
- Print `✓ idempotent — Start mail already sent earlier`
- Still acquire the file reservation and run `br update --status=in_progress`
- Exit 0

```bash
IDEM_BEAD=$(cat /tmp/wrapper_fix_idem_bead.txt)
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> \
  bin/br-claim "$IDEM_BEAD"
```

**Expected output contains:**

```
br-claim: ✓ reserved 1 path(s)
br-claim: ✓ idempotent — Start mail already sent earlier
br-claim: ✓ br update <IDEM_BEAD> --status=in_progress
```

**Critical assertion:** count of `[<IDEM_BEAD>] Start` mails after `br-claim` must still be **exactly 1**, not 2.

```
search_messages(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  query="[<IDEM_BEAD>] Start", limit=10
)
# expect: result list length == 1, same id as the manual injection
```

If the count is 2: the patch is incomplete — the wrapper still didn't detect the prior Start, sent a duplicate, and idempotency is still broken. Re-read your patch in `bin/br-claim` lines 270-282 against the spec in §7a and re-apply.

If the wrapper exits non-zero: read the message; the most likely causes are (a) the patch broke another code path (re-read the diff) or (b) `prior_other_start` got set incorrectly (your test sent the manual Start FROM yourself, so `prior_other_start` should remain `None`).

After this assertion passes, immediately close the idempotency-probe bead so it doesn't linger as `in_progress` (we don't run br-complete on it because the test scenario is artificial):

```bash
br close "$IDEM_BEAD" --reason "Idempotency probe — bead used to verify bin/br-claim no-duplicate-Start behavior; closing manually."
release_file_reservations(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  paths=["docs/scratch-smoke-test.md"]
)
bin/br-sync && git add .beads/
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> \
  git commit -m "beads: close $IDEM_BEAD idempotency probe (manual close)"
```

### 8e. Step T-d: trivial implementation + accepted commit (against the main smoke bead)

```bash
SMOKE_BEAD=$(cat /tmp/wrapper_fix_smoke_bead.txt)
echo "$(date -u +%FT%TZ) — wrapper-fix smoke test marker" >> docs/scratch-smoke-test.md
git add docs/scratch-smoke-test.md
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> \
  git commit -m "$SMOKE_BEAD: wrapper-fix smoke marker"
```

**Expected:** hook ACCEPTS (Start mail is in archive). Commit succeeds.

If hook BLOCKS: the Start mail might not be findable by the hook. The hook itself was confirmed correct in §3, so this would mean the Start mail wasn't actually sent — back up to step T-b and verify.

### 8f. Step T-e: negative test (hook BLOCKS unauthorized commit)

```bash
echo "negative test" >> docs/scratch-smoke-test.md
git add docs/scratch-smoke-test.md
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> \
  git commit -m "testproject1-fake.999.999: deliberately unclaimed"
# Expect: hook BLOCKS with "Pre-commit blocked by Agent Mail discipline (AGENTS.md Rule 2)..."
# exit code is non-zero.
```

Then revert the working tree to post-T-d state (preserves the smoke marker, removes the negative-test line):

```bash
git restore --staged docs/scratch-smoke-test.md
git checkout HEAD -- docs/scratch-smoke-test.md

# verify
grep "wrapper-fix smoke test marker" docs/scratch-smoke-test.md && echo "OK marker preserved"
grep "negative test"                  docs/scratch-smoke-test.md && echo "FAIL: negative-test line not reverted" || echo "OK negative line removed"
```

### 8g. Step T-f: push the legitimate commit

```bash
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> git pull --rebase
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> git push
```

If `git pull --rebase` complains about unstaged changes (the `M .beads/issues.jsonl` from `br update` during claim), you can push directly without rebase if `git rev-list --left-right --count origin/main...HEAD` shows `behind: 0`.

### 8h. Step T-g: complete via wrapper (THE PRIMARY FIX TEST)

```bash
SMOKE_BEAD=$(cat /tmp/wrapper_fix_smoke_bead.txt)
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> \
  bin/br-complete "$SMOKE_BEAD"
```

**Expected (the whole point of this handoff):** wrapper exits 0. Visible signals:
- "Found Start mail from <your-name>" or similar (wrapper found the Start via the patched lookup)
- Complete mail sent
- `br close` ran
- `bin/br-sync && git add .beads/ && git commit && git pull --rebase && git push` all succeed (wrapper does this internally)
- Reservations released

If wrapper exits 1 with `no [<SMOKE_BEAD>] Start mail from '<your-name>' in the archive`: the patch did not stick. Re-read `bin/br-complete` lines 274-282 — the change must be byte-identical to the spec in §7b. Then re-run from §8h.

If wrapper exits 1 with anything else: read the message, diagnose. Most likely candidates: working tree dirty (the `.beads/issues.jsonl` left over from claim — wrapper handles its own internal sync, so don't pre-commit it), commit not yet pushed, etc.

**Verify all five post-conditions:**

```bash
SMOKE_BEAD=$(cat /tmp/wrapper_fix_smoke_bead.txt)
br show "$SMOKE_BEAD" --json | jq -r '..|objects|select(.id=="'"$SMOKE_BEAD"'")|.status' | head -1
# expect: closed

git status --porcelain | wc -l
# expect: 0

git rev-list --count origin/main..HEAD
# expect: 0
```

```
search_messages(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  query="[<SMOKE_BEAD>] Complete", limit=5
)
# expect: at least one result, from=<your-name>, body mentions the commit hash
```

```bash
# reservations released — server-running-friendly check
grep -l '"agent":[[:space:]]*"<your-name>"' \
  /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/projects/users-alinaaitova-desktop-testproject1/file_reservations/*.json 2>/dev/null \
  | xargs -I{} jq -r 'select(has("released_ts") | not) | "STILL ACTIVE: id=\(.id) path=\(.path_pattern) reason=\(.reason)"' {}
# expect: no output (all your reservations have released_ts set)
```

If `bin/br-complete` does its own internal git push, you may need to re-fetch before checking origin sync. After this, the smoke bead is closed and pushed; you're done with the primary test.

---

## 9. Validate→Fix→Test Loop policy

You will iterate until every acceptance criterion in §10 is true. Use this loop discipline:

### Cycle structure

1. **Identify the failing step.** Be exact: "step T-c manual idempotency probe — duplicate Start mail sent (count=2 instead of 1)."
2. **State a single hypothesis.** "The patch in `bin/br-claim` lines 271-279 doesn't include `result` in the key list."
3. **Apply ONE targeted change.** Don't rewrite multiple things at once — that obscures cause/effect.
4. **Re-run the **same** failing step plus the next downstream step.** Don't skip ahead. Don't re-run earlier passing steps unnecessarily, but DO re-run if your change touches their code path.
5. **If pass → continue down the smoke test.** If fail → return to (1) with the new diagnosis.

### Bounded retry

- **Same step, same error, three distinct fix attempts:** STOP. The diagnosis is wrong. Read the relevant code top-to-bottom in full, write your understanding into the report, and escalate to the user.
- **More than 5 commits during this work without convergence:** STOP. Something is structurally off — perhaps the agent-mail server response shape changed mid-session or another agent is editing the wrappers in parallel. Escalate.
- **More than 3 distinct smoke beads created (the main, the idempotency probe, plus one):** STOP. You should not need a fourth. Investigate what is causing repeated rebuilds.

### What "fix in band" means vs. "escalate"

| Failure | In-band action |
|---|---|
| You forgot to set `AGENT_NAME` for `git push` | Set it, retry. Not a bug. |
| Pre-commit hook blocks because Start mail wasn't actually sent | Re-run `bin/br-claim` (which is now idempotent). Not a bug. |
| Patch syntax error | Re-edit, re-run. |
| New bug discovered in a wrapper code path you didn't change | Read the code, decide if minor (one-line obvious fix) or significant. If minor, fix it; if significant, escalate with a written report. |
| `am serve-http` died mid-session | Restart it (`am serve-http --port 8765 &` — but verify with the user first if you don't recognize the original startup). |
| The agent-mail SQLite is locked / mailbox repo is in a degraded state | Escalate. Do not "force unlock" or delete lock files. |
| You discover the precommit hook is wrong | Escalate. The hook was confirmed correct in this handoff. If it's broken now, something else changed — don't paper over it. |

### Things you must NEVER do during the loop

- **Never `git reset --hard`, `git clean -fd`, or any destructive cleanup.** AGENTS.md "Irreversible Git Actions" applies.
- **Never delete the smoke beads** (Rule 1). Close them, leave them.
- **Never delete `docs/scratch-smoke-test.md`** or the historical lines in it. Append-only.
- **Never use `AGENT_MAIL_BYPASS=1` to skip the hook.** The whole purpose is to verify the hook fires correctly.
- **Never amend a pushed commit.** New commits only.
- **Never spawn the L5 swarm.** Even if the smoke test goes green, your job ends at "report PASS to the user." The user spawns L5.

### When you're allowed to declare done

Only when **all** of §10's acceptance criteria are simultaneously true on the **same** test run (not "this passed yesterday, this passed today"). One end-to-end clean pass. If you patch something between criteria, restart the relevant downstream criteria.

---

## 10. Acceptance Criteria (the green state)

The handoff is complete when, on a single uninterrupted Phase T run after the patches in Phase F, ALL of these are true:

- [ ] **A1 Patches present.** `grep -E '\("result", "messages", "results"\)' bin/br-claim bin/br-complete` returns hits in both files.
- [ ] **A2 Wrappers parse and load.** `python3 -c "import ast; ast.parse(open('bin/br-claim').read())"` and same for `br-complete` exit 0. `bin/br-claim --help` and `bin/br-complete --help` exit 0.
- [ ] **A3 Reproducer post-fix.** The §7c reproducer prints `patched-logic match count: >=1` against `testproject1-l9vh`.
- [ ] **A4 Fresh smoke bead created.** New ID captured (NOT `testproject1-l9vh`), label `smoke-test`, status starts `open`.
- [ ] **A5 Dry-run claim works without `--paths`.** `bin/br-claim <fresh-bead> --dry-run` parses paths from the backticked bead body, exits 0, no side effects.
- [ ] **A6 Real claim works.** Status flips to `in_progress`, Start mail in archive, file reservation acquired, wrapper exits 0.
- [ ] **A7 Idempotency probe.** Against a second fresh bead with a manually-injected Start mail, `bin/br-claim` prints `✓ idempotent — Start mail already sent earlier`, exits 0, and the `[<bead>] Start` count in the archive remains exactly 1.
- [ ] **A8 Hook accepts legitimate commit.** `<bead-id>:`-prefixed commit on the main smoke bead lands cleanly.
- [ ] **A9 Hook blocks unauthorized commit.** `testproject1-fake.999.999:`-prefixed attempt is rejected with the AGENTS.md Rule 2 error message.
- [ ] **A10 Push works.** `git push` lands on origin/main with AGENT_NAME set.
- [ ] **A11 br-complete works (THE primary fix).** `bin/br-complete <fresh-bead>` exits 0, status flips to `closed`, Complete mail in archive, reservations released, wrapper's internal sync push lands.
- [ ] **A12 Final state clean.** Working tree clean, in sync with origin/main, no orphaned `in_progress` beads owned by you, no active reservations under your agent name. Smoke bead is `closed` and labeled `smoke-test`. Idempotency-probe bead is `closed`.

If any criterion is false: loop (§9).

---

## 11. Cleanup

- **The two new smoke beads** (`<SMOKE_BEAD>` and `<IDEM_BEAD>`) join `testproject1-l9vh` as closed `smoke-test`-labeled beads. They accumulate by design.
- `docs/scratch-smoke-test.md` now has at least 2 historical lines. Don't trim — it's evidence.
- All commits should be pushed by the time you reach §12.
- Add a short `br comment` on the main smoke bead linking to this handoff if you like (optional, low value):
  ```bash
  br comment $SMOKE_BEAD "Smoke test for prompts/pre-l5-wrapper-fix-handoff.md — search_messages response-key fix verified."
  ```

If you accidentally left an active file reservation:

```
release_file_reservations(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>"
)
```

(This releases ALL active reservations held by you; safe because by the end of Phase T you should have none in flight.)

---

## 12. Verification Snapshot (run before reporting)

```bash
SMOKE_BEAD=$(cat /tmp/wrapper_fix_smoke_bead.txt)
IDEM_BEAD=$(cat /tmp/wrapper_fix_idem_bead.txt)

echo "=== smoke beads final state ==="
br show "$SMOKE_BEAD" --json | jq -r '..|objects|select(.id=="'"$SMOKE_BEAD"'")|"id=\(.id) status=\(.status)"' | head -1
br show "$IDEM_BEAD"  --json | jq -r '..|objects|select(.id=="'"$IDEM_BEAD"'")|"id=\(.id) status=\(.status)"'  | head -1

echo "=== patches present ==="
grep -nE '\("result", "messages", "results"\)' bin/br-claim bin/br-complete

echo "=== working tree ==="
git status -sb

echo "=== origin sync ==="
git rev-list --left-right --count origin/main...HEAD | awk '{print "behind:",$1," ahead:",$2}'

echo "=== still-active reservations under your name ==="
grep -l '"agent":[[:space:]]*"<your-name>"' \
  /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/projects/users-alinaaitova-desktop-testproject1/file_reservations/*.json 2>/dev/null \
  | xargs -I{} jq -r 'select(has("released_ts") | not) | "STILL ACTIVE: id=\(.id) path=\(.path_pattern)"' {}
# expect: no output
```

In your agent session, also confirm mail counts:

```
search_messages(query="[<SMOKE_BEAD>]", limit=10)   # expect Start (1) + Complete (1)
search_messages(query="[<IDEM_BEAD>] Start", limit=10)   # expect exactly 1 (proves idempotency)
```

---

## 13. Report Format

### If everything passes

```
Pre-L5 wrapper fix + re-smoke: PASS

Patches applied:
  bin/br-claim (idempotency lookup)    — line ~272 → ('result','messages','results') iterator
  bin/br-complete (Start-mail lookup)  — line ~276 → ('result','messages','results') iterator

Smoke beads (closed, label smoke-test):
  <SMOKE_BEAD>  — main lifecycle test
  <IDEM_BEAD>   — idempotency probe

Acceptance criteria: A1–A12 all green on a single end-to-end run.

Wrapper end-to-end flow verified, including the previously-broken
br-complete path and br-claim idempotency.
L5 swarm is safe to spawn.

Recommended next step: paste the L5 kickoff prompt into the first
agent session and observe its first bead claim. If that succeeds,
proceed to spawn the rest of the swarm.

Optional follow-ups (NOT blockers — see Appendix A):
  - Bug 2 (path-parser regex requires backticks)
  - Bug 3 (pre-push hook AGENT_NAME requirement is undocumented)
```

### If the loop hits a hard stop

```
Pre-L5 wrapper fix + re-smoke: BLOCKED at <step>

Loop iteration count: <N>
Distinct fixes attempted: <list>
Distinct smoke beads created: <list>

Last failing step: <step letter and description>
Last error verbatim:
<paste>

Hypothesis at time of escalation: <best guess>

State of artifacts:
  Wrappers patches present: <yes/no, with grep evidence>
  Smoke beads:
    <id> — status=<...>
  Working tree: <clean/dirty — list>
  Origin sync: behind=<N> ahead=<N>
  Active reservations under your agent: <list or none>

What I have not tried yet but might be worth: <one or two ideas>

L5 swarm is NOT safe to spawn until this is resolved.
```

After reporting, STOP. Do not attempt further fixes without user input.

---

## 14. Critical Rules

1. **Do NOT spawn the L5 swarm.** Even after PASS. The user spawns it.
2. **Do NOT delete files.** Smoke beads and scratch files stay (Rule 1).
3. **Do NOT modify the precommit hook.** It's the reference correct implementation; modifying it risks regressing what already works.
4. **Do NOT use `AGENT_MAIL_BYPASS=1`.** It defeats the test.
5. **Do NOT amend pushed commits.** New commits only.
6. **Do NOT use destructive git** (`reset --hard`, `clean -fd`, `push --force`). See AGENTS.md "Irreversible Git Actions".
7. **Always set `AGENT_NAME` (and `AGENT_MAIL_NAME`) for every git operation**, including `git push`. The pre-push hook also requires it.
8. **Treat unrelated working-tree changes as parallel-agent activity.** Never stash or revert work you did not author. Per AGENTS.md "Note for Codex/GPT-5.2".
9. **Single agent procedure.** Do not spawn parallel sub-agents for "concurrency testing" — that's a separate handoff if you want it.
10. **Push to origin before declaring done.** Per AGENTS.md "Landing the Plane".

---

## Appendix A — Optional follow-ups (NOT blockers for L5)

These are real but lower-priority issues uncovered during the prior smoke test. Document them in your final report; do not block on them.

### A.1 — Path-parser regex requires backticks

`bin/br-claim`'s `parse_files_from_bead` uses `PATH_BACKTICK_RE = re.compile(r"`([^`\n]+\.[A-Za-z0-9_]{1,8})`")` — backticks are mandatory. Bead bodies that list paths inline without backticks (e.g. `- docs/foo.md (create)`) trigger `could not determine files to reserve`, forcing a `--paths` override.

**Workaround used in this handoff:** the smoke-test bead body uses backticks — `` - `docs/scratch-smoke-test.md` ``.

**Recommended fix (not in this handoff):** loosen the parser to accept unbacked paths under `### Files to Create/Modify`, OR enforce backticks in the `beads-from-sources` and `beads-from-strategy` skill templates so all generated bead bodies match the parser's expectations. Apply to both `~/.claude/skills/beads-from-sources/` and `~/.codex/skills/beads-from-sources/` per the Phase A two-CLI protocol.

### A.2 — Pre-push hook AGENT_NAME requirement is undocumented

`git push` from this repo invokes a pre-push hook that also requires `AGENT_NAME` to be set. The original Phase A/B handoffs only mention this for `git commit`. Effect: agents who only set `AGENT_NAME` for commits hit a confusing error on push:

```
mcp-agent-mail: AGENT_NAME environment variable is required.
error: failed to push some refs to '...'
```

**Recommended:** add a one-liner to the L5 kickoff and the `beads-to-execution` skill template noting that `AGENT_NAME` must be set for ALL git operations (commit, push, pull-rebase). Trivial documentation tweak, no code change needed.

### A.3 — `am file_reservations active <project>` requires exclusive mailbox lock

The CLI command from §9 of the prior handoff (`am file_reservations active <project>`) competes with the running `am serve-http` for an exclusive mailbox-activity lock and fails with "Resource is temporarily busy" while the server is up. Workaround used: read the on-disk JSON artifacts directly under `~/.mcp_agent_mail_git_mailbox_repo/projects/<project>/file_reservations/`. This is a CLI ergonomics issue, not a correctness bug — note in passing.

---

## Appendix B — Quick code-level reference

### Reference correct implementation (the hook)

`.git/hooks/hooks.d/pre-commit/50-agent-mail.py:771-781`:

```python
# Server returns {result: [...], ...} but some shapes use 'messages' / 'results'.
messages = []
if isinstance(payload, dict):
    for key in ("result", "messages", "results"):
        value = payload.get(key)
        if isinstance(value, list):
            messages = value
            break
elif isinstance(payload, list):
    messages = payload
return messages
```

### Pre-fix wrapper (the bug, both files)

```python
messages = []
if isinstance(search_res, dict):
    messages = search_res.get("messages") or search_res.get("results") or []
elif isinstance(search_res, list):
    messages = search_res
```

### Post-fix wrapper (mirror of hook pattern, both files)

```python
messages = []
if isinstance(search_res, dict):
    for key in ("result", "messages", "results"):
        value = search_res.get(key)
        if isinstance(value, list):
            messages = value
            break
elif isinstance(search_res, list):
    messages = search_res
```

### Effect on the wrapper-internal flow

- **`bin/br-claim`:** with the fix, the `prior_self_start` flag is correctly set when our own Start exists in the archive. Line 340 (`if not prior_self_start:`) then skips the redundant `send_message` call. Without the fix, a partial-failure retry sends a duplicate Start mail every time.
- **`bin/br-complete`:** with the fix, line 290 (`if sender == agent: self_start_found = True`) fires correctly, the wrapper proceeds past the existence check, and the rest of the complete lifecycle (close → sync → push → release) runs. Without the fix, the wrapper dies at lines 293-298 ("no Start mail from <agent>") regardless of archive contents.

---

## Appendix C — What the prior smoke test proved (so you don't re-prove it)

These were already verified on 2026-05-08 by agent `IvoryMaple`, smoke bead `testproject1-l9vh`. You don't need to re-test these specific points if the wrappers/hook are unchanged from then:

- `bin/br-claim --dry-run` exits 0 with no side effects.
- `bin/br-claim` (real) creates a file reservation, sends a Start mail to peers (or self if no peers), and flips `br update --status=in_progress` atomically.
- The precommit hook ACCEPTS a `<bead-id>:`-prefixed commit when a Start mail exists in the archive within the lookback window.
- The precommit hook BLOCKS a `<bead-id>:`-prefixed commit when no Start mail exists, with an actionable error message that names the missing bead and references AGENTS.md Rule 2.
- The pre-push hook works (with AGENT_NAME set).

The thing that was NOT proven and that this handoff is testing: **`bin/br-complete` running cleanly end-to-end against a freshly-claimed bead** (it died on the response-key bug). Plus the new idempotency probe.
