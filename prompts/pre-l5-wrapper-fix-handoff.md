# Pre-L5 Wrapper Fix + Re-Smoke Loop

> **Paste this entire file into a fresh Claude Code OR Codex session.**
> **Do not mark this work complete until the validate→fix→test loop reaches green AND the L5 swarm is safe to spawn.**
>
> **CLI-agnostic.** This handoff works identically from Claude Code (with the `Edit` tool) and Codex (with `apply_patch`). All other tooling — MCP agent-mail calls, bash, Python reproducer, git, `br` — is shared. Skill-template fixes in Appendix A.1 must be applied to BOTH `~/.claude/skills/<skill>/` AND `~/.codex/skills/<skill>/` per the Phase A two-CLI protocol — they are separate file copies, not symlinks.

---

## 0. TL;DR for the operator

### What L5 is (one paragraph for fresh agents)

This repository is a UK public procurement intelligence platform under construction in 7 layers (L0 through L6, see `docs/strategy/layer-strategy.md`). L4 wrapped on 2026-05-08 with 56 task beads closed, but Agent Mail compliance failed badly — only 4 Start mails were sent in 10 hours of work, leaving the swarm with no audit trail of who did what. Phase A (`prompts/phase-a-mail-discipline-infrastructure-handoff.md`) and Phase B (`prompts/phase-b-precommit-mail-enforcement-handoff.md`) deployed mail-discipline infrastructure: `bin/br-claim` and `bin/br-complete` wrappers, plus a precommit hook that blocks `<bead-id>:`-prefixed commits without a corresponding Start mail. L5 is the next layer — a swarm of 4-6 parallel agents that will use these wrappers to coordinate. **If the wrappers don't work, the L5 swarm will collapse.** This handoff exists to make sure they work before the swarm spawns.

### The bug

`bin/br-claim` and `bin/br-complete` historically mis-parsed the `search_messages` JSON-RPC response. The agent-mail server returns matches under the key `"result"` (singular), but both wrappers were looking up `"messages"` or `"results"` (plural). Effects of the original bug:

- **`bin/br-complete` was fully broken on the live path** — it could never find the Start mail it requires before sending Complete, so every L5 agent would succeed at claim and then fail at complete, leaving every bead stuck `in_progress`.
- **`bin/br-claim` idempotency was silently broken** — partial-failure recovery (where `br update` failed after Start mail sent) would send a duplicate Start mail every retry instead of detecting the prior send.

The canonical correct pattern lives in `.git/hooks/hooks.d/pre-commit/50-agent-mail.py:771-781`. The fix mirrors that pattern into the two wrappers.

### Two possible session-start states

When you start, the wrappers are in **one of two states**. Both are handled by this doc:

- **State B (fix already applied — likely default).** A previous session — or even a parallel session running this same handoff — may have already landed commit `adec8e4` (or an equivalent). The post-fix lookup block spans lines **270-278 of `bin/br-claim`** (with `for key in ("result", "messages", "results"):` at line 272) and lines **274-282 of `bin/br-complete`** (for-loop at line 276). **In State B, Phase F is a no-op — proceed straight to the protocol-level reproducer (Phase V) and the full smoke test (Phase T).** Your job becomes "verify the fix is correct and prove the wrappers work end-to-end," not "apply the fix."
- **State A (fix not yet applied).** Same line ranges, but the lookup is the original 5-line `messages = search_res.get("messages") or search_res.get("results") or []` shape (no for-loop). **Apply the patches in Phase F, then Phase T.**

§5 has the exact source-eyeball you run to determine which state you're in. Don't skip it.

### Carry-over context from the prior smoke-test session

The 2026-05-08 smoke-test session (agent `IvoryMaple`, smoke bead `testproject1-l9vh`, see `prompts/pre-l5-sanity-check-handoff.md` for the original procedure) discovered the bug and recovered cleanly. State you'll inherit:

- Smoke bead `testproject1-l9vh` is `closed`. **Do NOT delete it** (Rule 1) — it is audit evidence and is also used as the §6 reproducer's known-good Start mail.
- `docs/scratch-smoke-test.md` exists with one or more historical lines. **Do NOT delete or overwrite it** — append only.
- The fix commit `adec8e4 fix(wrappers): include 'result' in search_messages response-key chain` may or may not be on `main` depending on which state you're in. Verify with `git log --oneline -5 -- bin/br-claim bin/br-complete`.
- All commits should be pushed to `origin/main`. Working tree should be clean. Pre-flight (§4) verifies this.

---

## 1. Mission

Three phases. The first two are conditional on session-start state (§0); Phase T runs unconditionally.

1. **Validate (Phase V — always run).** Eyeball the source (§5) to determine state. Then run a stand-alone protocol-level reproducer (§6) against the live agent-mail server. The reproducer proves what the buggy lookup vs. the fixed lookup do, regardless of which state the wrappers are in.
2. **Fix (Phase F — conditional).** If §5 shows State A (buggy code present), apply the patches in §7. If §5 shows State B (already fixed), this phase is a no-op — skip to §7c sanity-check, then on to Phase T.
3. **Test (Phase T — always run).** Full smoke test against a **fresh** throwaway bead (DO NOT reuse `testproject1-l9vh`). Includes an explicit idempotency check (§8d) that exercises the previously-broken `bin/br-claim` code path.

If anything in Phase T fails — even something unrelated to the original bug — diagnose, fix in-band if minor, and **re-run from the failed step downward** per the §9 loop policy. Don't claim done until every acceptance criterion in §10 is true.

**Project key:** `/Users/zacfarrell/Desktop/Testproject1`
**Branch:** `main` only
**Bead label:** `smoke-test` (the new fresh beads reuse this label; they accumulate with the prior closed smoke bead `testproject1-l9vh`, by design — see §11 of `prompts/pre-l5-sanity-check-handoff.md`)

### First action checklist

If you're trying to figure out where to start, do these in order. Each step is one or two tool calls:

1. §2 — call `macro_start_session`, capture your agent_name, set `AGENT_NAME` and `AGENT_MAIL_NAME`.
2. §3 — read the listed files / line ranges.
3. §4 — run the pre-flight checks. STOP on any ABORT.
4. §5 — eyeball the wrapper source. Determine State A vs State B. Branch accordingly.
5. §6 — run the protocol-level reproducer. Always do this regardless of state.
6. §7 (conditional) — apply patches if State A. Sanity-check (§7c) regardless.
7. §8 — full smoke test. Loop on failures per §9.
8. §10 — confirm every acceptance criterion is green.
9. §13 — write the PASS or BLOCKED report.

---

## 2. AgentMail Registration (mandatory)

Before any other tool call. The wrappers and the precommit/pre-push hooks all require `AGENT_NAME` to be set.

### Notation convention used throughout this handoff

When you see a function-call style block like `macro_start_session(...)` or `search_messages(...)`, that means **invoke the corresponding MCP tool through your agent harness** — Claude Code uses the `mcp__mcp-agent-mail__<tool>` namespacing; Codex uses the same tool names without the namespace prefix; both end up calling the same agent-mail server. The argument shape shown is the actual MCP argument schema. These notation blocks are not Python — don't run them through `python3`.

### If MCP agent-mail tools are deferred in your harness

Some Claude Code sessions surface MCP tools as **deferred** (the names appear in the deferred-tools list but the schemas aren't loaded; calling them returns `InputValidationError`). If that's your situation, load the agent-mail toolkit in bulk before §4 — one call gets the entire server's tools:

```
ToolSearch(query="mcp-agent-mail", max_results=30)
```

Codex and other harnesses typically auto-load all configured MCP tools at session start, so this step is usually unnecessary outside Claude Code. If your `macro_start_session` call already worked above, you're loaded — proceed.

### Invocation

```
macro_start_session(
  project_key="/Users/zacfarrell/Desktop/Testproject1",
  program="pre-l5-wrapper-fix",
  model="<your-model>",
  task_description="Fix search_messages response-key parsing in bin/br-claim and bin/br-complete; re-run smoke test until green"
)
```

The response is a structured object with shape `{"project": {...}, "agent": {"id": <int>, "name": "<name>", ...}, "file_reservations": {...}, "inbox": [...]}`. Your assigned name is at `response.agent.name` (e.g. `IvoryMaple`, `PlumMouse`, `BlueLake`). Capture it, then in the same shell:

```bash
export AGENT_NAME="<your-name>"
export AGENT_MAIL_NAME="<your-name>"
```

> **Reminder:** the Bash tool does not persist shell state between calls. Either source a tiny env script at the start of every command, or prepend `AGENT_NAME=<name> AGENT_MAIL_NAME=<name>` to every command that needs them (`git commit`, `git push`, `bin/br-claim`, `bin/br-complete`). The pre-push hook also requires `AGENT_NAME` — do not omit it on `git push`.

---

## 3. Required Reading (do this before touching any file)

These files are necessary context. The estimated read time is ~15-20 minutes; you can skim sections labeled "skim" rather than read them line-by-line.

1. **`AGENTS.md`** — read **Rule 1 (no deletion), Rule 2 (use the wrappers), Irreversible Git & Filesystem Actions, and the "Note for Codex/GPT-5.2"** sections in full (~150 lines combined). Skim the rest. You will commit to `main` directly with `fix:` / `beads:` prefixes; both are exempt from the Phase B Start-mail check, so the work itself does not need a separate claim.
2. **`prompts/pre-l5-sanity-check-handoff.md`** — skim sections §1, §6 (the lifecycle steps), and §10 (the failure-mode example). You will re-run a near-identical procedure in Phase T; reading the original lets you cross-reference if §8 is ambiguous. Note the actual filename — there is no `mail-discipline` infix, despite the document's heading.
3. **`bin/br-claim` lines 87-148 (`mcp_call`) and 262-296 (idempotency block + `prior_other_start` guard)** — confirm the lookup is at **line 272** (`for key in ...` post-fix, or `messages = search_res.get("messages") ...` pre-fix) and the unwrap path matches what the reproducer in §6 mimics. The `prior_self_start` consumer is at line 344 (`if not prior_self_start:`); the idempotent-print is at line 390. Skim the rest of the file.
4. **`bin/br-complete` lines 268-302 (Start-mail check)** — confirm the lookup is at **line 276** and the failure-mode is the `die(...)` at lines **297-302** (`if not self_start_found: die(f"no \`[{bead_id}] Start\` mail from {agent!r} ...")`). The sender comparison is at line 294. Skim the rest of the file.
5. **`.git/hooks/hooks.d/pre-commit/50-agent-mail.py` lines 711-781** — read in full (only ~70 lines). This is the **reference correct implementation**. Note: it iterates `("result", "messages", "results")` and explicitly checks `isinstance(value, list)`. This is the pattern you are mirroring into the wrappers. **Do NOT modify the hook** — it already works.

After reading, you should be able to answer in one sentence each: *Why does the bug only manifest in `br-complete` during normal operation, even though `br-claim` shares the buggy line?* and *Why is re-running `bin/br-claim <same-bead>` not a valid idempotency test?* (Hint: status check on line 255 dies before the idempotency check fires.)

### Optional supplementary reading (only if specific symptoms surface)

- `prompts/phase-a-mail-discipline-infrastructure-handoff.md` — the original wrapper deployment handoff. Read if you suspect a wrapper-design issue rather than a parsing issue.
- `prompts/phase-b-precommit-mail-enforcement-handoff.md` — the hook deployment handoff. Read if Step T-d (hook accepts) or T-e (hook blocks) misbehaves.
- `prompts/mcp-agent-mail-immediate-read-after-write-fix-handoff.md` — read if you suspect a `send_message` → `search_messages` indexing-lag race in §8d. The historical fix may not cover all edge cases.

---

## 4. Pre-flight Checks (carry over from prior handoff — fail-hard on any ABORT)

Do not use `exit 1` inside an inline shell snippet — it terminates your bash session. Just observe and STOP-and-report on any ABORT.

```bash
cd /Users/zacfarrell/Desktop/Testproject1

# A. tree clean
if [ -n "$(git status --porcelain)" ]; then
  echo "ABORT: working tree dirty:"; git status --short
else echo "OK tree clean"; fi

# B. wrappers exist
test -x bin/br-claim    && echo "OK bin/br-claim"    || echo "ABORT: bin/br-claim missing or not executable"
test -x bin/br-complete && echo "OK bin/br-complete" || echo "ABORT: bin/br-complete missing or not executable"

# B2. br (beads_rust) CLI on PATH — wrappers shell out to it
command -v br >/dev/null \
  && echo "OK br $(br --version 2>&1 | head -1)" \
  || echo "ABORT: br (beads_rust) not on PATH — wrappers will fail. Install per https://github.com/Dicklesworthstone/beads_rust"

# C. hook unchanged (Phase B marker)
grep -q "check_start_mail_for_bead_commits" .git/hooks/hooks.d/pre-commit/50-agent-mail.py \
  && echo "OK Phase B marker present" \
  || echo "ABORT: Phase B marker missing"

# C2. jq present (used heavily by §8 and §12 verification)
command -v jq >/dev/null \
  && echo "OK jq $(jq --version)" \
  || echo "ABORT: jq not installed — install (brew install jq on macOS, apt install jq on Linux) before continuing"

# C3. Detect orphaned smoke-test state from prior abandoned runs
ORPHANS=$(br list --label=smoke-test --json 2>&1 | jq -r '.issues[]? | select(.status == "in_progress") | .id' 2>/dev/null)
if [ -n "$ORPHANS" ]; then
  echo "WARN: orphaned in_progress smoke-test beads from prior runs (do NOT touch unless asked):"
  echo "$ORPHANS" | sed 's/^/  /'
  echo "  → Note these in your final report. They may be from a parallel agent (still running) or"
  echo "    an abandoned run (e.g., br-complete failed and recovery was never finished). Do not"
  echo "    'clean them up' — leave them for the user to decide. Your fresh smoke beads in §8a/§8d"
  echo "    will be DIFFERENT IDs, so this won't interfere with your run."
else
  echo "OK no orphaned smoke-test in_progress beads"
fi

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

## 5. Determine session-start state (State A vs State B)

This decides whether Phase F runs. Eyeball the source AND check git log so you know how the codebase reached its current state — that gives you context if Phase T later fails in an unexpected way.

```bash
echo "=== br-claim — idempotency lookup region ==="
sed -n '270,282p' bin/br-claim

echo "=== br-complete — Start-mail lookup region ==="
sed -n '274,286p' bin/br-complete

echo "=== git history of these files (last 5 commits) ==="
git log --oneline -5 -- bin/br-claim bin/br-complete
```

### State A — original bug present (Phase F is required)

You'll see this in BOTH wrappers:

```python
    messages = []
    if isinstance(search_res, dict):
        messages = search_res.get("messages") or search_res.get("results") or []
    elif isinstance(search_res, list):
        messages = search_res
```

→ Proceed: §6 (reproducer) → §7 (apply patches) → §7c (sanity) → §8 (smoke test).

### State B — fix already applied (Phase F is a no-op)

You'll see this in BOTH wrappers (this is the most likely state in a fresh session — commit `adec8e4` or equivalent already landed):

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

→ Proceed: §6 (reproducer — still useful, proves the protocol-layer behavior) → SKIP §7a/§7b → §7c (sanity, run anyway) → §8 (smoke test). Note in your final report: "Fix already in place at session start (commit `<sha-from-git-log>`); only verification + smoke ran."

### State C — neither pattern matches (drift)

If the source contains some other shape (someone refactored, the file was renamed, etc.), STOP and report. Don't guess. The patch in §7 is calibrated to the two patterns above; applying it blindly to drifted source could break working code.

Read the surrounding 30-50 lines, identify why the lookup region differs from both A and B, capture that in your report, and ask the user before continuing.

---

## 6. Phase V — Validate at the protocol layer

Always run this regardless of State A vs B. The reproducer hits the live agent-mail server and shows what the buggy lookup vs. the fixed lookup returns. In State A this proves the bug exists; in State B this proves the agent-mail server still emits the singular-`result` shape (and therefore the fix is still load-bearing).

It mirrors the exact unwrap path of `mcp_call` in `bin/br-claim:87-148` and queries `search_messages` for a known-good thread (`testproject1-l9vh` from the prior smoke test, whose Start mail id=213 is in the archive). If that thread's Start mail is gone — see "Fallback if testproject1-l9vh has been purged" below.

**Auth note:** if the local agent-mail server is configured to require a bearer token, the reproducer below will return HTTP 401/403. In that case, set `AGENT_MAIL_BEARER_TOKEN` and add `req.add_header("Authorization", f"Bearer {os.environ['AGENT_MAIL_BEARER_TOKEN']}")` after the `Request(...)` line. Token sources, in order of precedence:
- `AGENT_MAIL_BEARER_TOKEN` env var (if set)
- Claude Code: `~/.claude.json` → `mcpServers.mcp-agent-mail.headers.Authorization` (strip the `Bearer ` prefix)
- Codex: `~/.codex/config.toml` → `[mcp_servers.mcp-agent-mail]` → `env.AUTH_TOKEN` or `headers."Authorization"`, depending on how it's configured. May also be in `~/.codex/auth.json`.
- Or: read it directly from how the running `am serve-http` process was started (check parent process arguments / env)

The reproducer as written assumes localhost-no-auth, which matches the server's current config.

```bash
python3 - <<'PYEOF'
import json, urllib.request

URL = "http://127.0.0.1:8765/mcp/"
BEAD = "testproject1-l9vh"  # prior smoke bead — Start mail is in archive

body = json.dumps({
    "jsonrpc": "2.0", "id": 1, "method": "tools/call",
    "params": {"name": "search_messages", "arguments": {
        "project_key": "/Users/zacfarrell/Desktop/Testproject1",
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

> **Note on match counts:** the search query `[testproject1-l9vh] Start` is a phrase search over both subject and body. The Start mail (id=213) matches via subject. Other mails that reference this phrase in their body (e.g., the recovery Complete mail id=214 from the original smoke test, which mentions `[testproject1-l9vh] Start` in prose) will also match. Don't be alarmed if the search returns 2+ results — the wrappers' inner subject filter (`if f"[{bead_id}] Start" not in subj: continue`) keeps only true Start mails. For the §6 reproducer, what matters is `FOUND` vs `EMPTY`, not the exact count.

### Interpreting the output

| `BUGGY lookup` | `FIXED lookup` | What it means | What to do |
|---|---|---|---|
| EMPTY | FOUND | Server emits singular `result`; the fix in §7 is correctly calibrated | If §5 said State A → proceed to Phase F. If §5 said State B → skip to §7c sanity, then Phase T. |
| FOUND | FOUND | Server response shape has additional keys, or has changed | STOP and re-investigate. The patch as written keeps `result` first; if `messages` or `results` is also populated, the patch still works — but verify before proceeding. |
| EMPTY | EMPTY | Either the `testproject1-l9vh` Start mail was purged, OR the server returns no matches at all | See "Fallback" below. |
| (HTTP error) | (HTTP error) | Server isn't reachable or auth changed | Re-run pre-flight (§4 D), set `AGENT_MAIL_BEARER_TOKEN` if needed (§6 auth note). |

### Fallback if `testproject1-l9vh` has been purged

If both lookups returned EMPTY and you've ruled out HTTP errors, the prior smoke-bead's Start mail may have been archived/purged. Two options:

**Option 1 — query against any Start mail in the archive.** Edit the reproducer's `params.arguments.query` line to drop the bracket prefix entirely. Replace `"query": f"[{BEAD}] Start", "limit": 20,` with `"query": "Start", "limit": 20,` AND remove the `BEAD = "testproject1-l9vh"` line (it's no longer used). Re-run.

**Option 2 — inject a synthetic Start mail and query for it.** From your agent session, call `send_message` with `thread_id="testproject1-recheck.0.0"`, `subject="[testproject1-recheck.0.0] Start: reproducer probe"`, `body_md="probe"`. Then change `BEAD = "testproject1-l9vh"` to `BEAD = "testproject1-recheck.0.0"` in the reproducer (no other line changes needed) and re-run.

If output matches the first row of the table: bug behavior confirmed at protocol layer. Proceed per the "What to do" column.

---

## 7. Phase F — Apply the fix (skip §7a/§7b if §5 said State B)

Two files. **Identical change in both.** Apply each patch with your CLI's native file-edit tool — Claude Code's `Edit`, Codex's `apply_patch`, or whatever your harness exposes — never `sed`, regex-replace scripts, or any other scripted code rewriter per AGENTS.md "No Script-Based Changes".

> **Skip-condition:** if §5 reported **State B** (the post-fix pattern is already in both files), §7a and §7b are no-ops. Do NOT re-edit files just to "make sure." Jump directly to §7c (sanity-check), then §8 (smoke test). The §7d audit and §7e reservation-release are optional regardless.

(Optional but recommended for State A) reserve the wrappers so a parallel agent doesn't collide:

```
file_reservation_paths(
  project_key="/Users/zacfarrell/Desktop/Testproject1",
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
        "project_key":"/Users/zacfarrell/Desktop/Testproject1",
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
  project_key="/Users/zacfarrell/Desktop/Testproject1",
  agent_name="<your-name>",
  paths=["bin/br-claim", "bin/br-complete"]
)
```

### 7f. Commit the fix (do NOT push yet — the smoke test will push together)

> **Skip-condition (State B):** if §5 said State B, you didn't apply a patch in §7a/§7b — there's nothing new to commit. `git status --porcelain` should show no changes to `bin/br-claim` or `bin/br-complete`. Skip §7f entirely and proceed to §8.

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

Carry over the procedure from `prompts/pre-l5-sanity-check-handoff.md` with three tweaks:

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
  project_key="/Users/zacfarrell/Desktop/Testproject1",
  query="[<SMOKE_BEAD>] Start", ranking="recency", limit=5
)
# expect: 1 result, from=<your-name>, subject contains "[<SMOKE_BEAD>] Start"

# 3. file reservation (artifact on disk)
ls -lt /Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/projects/users-zacfarrell-desktop-testproject1/file_reservations/ | head -3
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
  project_key="/Users/zacfarrell/Desktop/Testproject1",
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
  project_key="/Users/zacfarrell/Desktop/Testproject1",
  query="[<IDEM_BEAD>] Start", limit=5
)
# expect: count = 1
```

Now run `bin/br-claim` against this bead. The wrapper should:
- See `status=open` (passes the status check at line 255)
- Find the prior Start from us via the **patched** lookup at line 272
- Set `prior_self_start = True` (line 290)
- Print an early info line: `br-claim: idempotency: prior Start from <your-name> found → mail will be SKIPPED` (line 315)
- Acquire the file reservation
- Skip the `send_message` call (the `if not prior_self_start:` guard at line 344)
- Print `✓ idempotent — Start mail already sent earlier` (line 390)
- Run `br update --status=in_progress`
- Exit 0

> **Indexing-lag note:** if you just sent the manual Start mail and immediately invoke `bin/br-claim`, the agent-mail Search V3 index may not yet show the message even though it's committed to the database. If `bin/br-claim` sends a duplicate (count goes to 2), wait 2-3 seconds and re-test on a fresh `IDEM_BEAD2` — the prior `mcp-agent-mail-immediate-read-after-write-fix-handoff.md` work made this rare, but the race window is non-zero. Don't immediately conclude the patch is broken; rule out the race first.

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
  project_key="/Users/zacfarrell/Desktop/Testproject1",
  query="[<IDEM_BEAD>] Start", limit=10
)
# expect: result list length == 1, same id as the manual injection
```

If the count is 2: the patch is incomplete — the wrapper still didn't detect the prior Start, sent a duplicate, and idempotency is still broken. Re-read your patch in `bin/br-claim` lines 270-282 against the spec in §7a and re-apply.

If the wrapper exits non-zero: read the message; the most likely causes are (a) the patch broke another code path (re-read the diff) or (b) `prior_other_start` got set incorrectly (your test sent the manual Start FROM yourself, so `prior_other_start` should remain `None`).

After this assertion passes, immediately close the idempotency-probe bead so it doesn't linger as `in_progress` (we don't run br-complete on it because the test scenario is artificial):

```bash
br close "$IDEM_BEAD" --reason "Idempotency probe — bead used to verify bin/br-claim no-duplicate-Start behavior; closing manually."
```

**Release ONLY the idempotency-probe's reservation, not the main smoke bead's.** Subtle but critical: §8c's `bin/br-claim` already acquired an exclusive reservation on `docs/scratch-smoke-test.md` for `<SMOKE_BEAD>` BEFORE §8d ran, and §8d's claim acquired a SECOND reservation on the same path for `<IDEM_BEAD>`. Same-agent same-path reservations don't conflict (the conflict check only fires for different agents), so both coexist. If you release by `paths=["docs/scratch-smoke-test.md"]` or by `agent_name` alone, you'll release BOTH — and §8h's `bin/br-complete` will fail because the main bead's reservation is gone before it tries to release on its own.

The safe pattern is ID-targeted release. Find the idempotency-probe reservation ID via the on-disk JSON, then release by ID:

```bash
IDEM_BEAD=$(cat /tmp/wrapper_fix_idem_bead.txt)
RES_ID=$(grep -l "\"reason\":[[:space:]]*\"$IDEM_BEAD\"" \
  /Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/projects/users-zacfarrell-desktop-testproject1/file_reservations/id-*.json 2>/dev/null \
  | xargs -I{} jq -r 'select(has("released_ts") | not) | .id' {} \
  | head -1)
echo "RES_ID=$RES_ID"
test -n "$RES_ID" || echo "WARN: no active reservation found for $IDEM_BEAD — already released?"
```

Then in your agent session (substitute the integer):

```
release_file_reservations(
  project_key="/Users/zacfarrell/Desktop/Testproject1",
  agent_name="<your-name>",
  file_reservation_ids=[<RES_ID>]
)
```

If `RES_ID` is empty (no active reservation found), skip the release call — the idempotency probe's reservation may have already been released somehow, or `br-claim` didn't acquire one (in which case re-read the §8d output to confirm `✓ reserved 1 path(s)` actually printed).

Then commit the bead-state change:

```bash
bin/br-sync && git add .beads/
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> \
  git commit -m "beads: close $IDEM_BEAD idempotency probe (manual close)"
```

The main smoke bead's reservation stays held by you until §8h's `bin/br-complete` releases it — that's correct.

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

`git pull --rebase` will FAIL at this point with `cannot pull with rebase: You have unstaged changes` because of the leftover `M .beads/issues.jsonl` from the `br update --status=in_progress` performed during the claim. Don't pull-rebase; check origin sync first and push directly when behind=0:

```bash
git fetch
ah=$(git rev-list --left-right --count origin/main...HEAD)
behind=$(echo $ah | awk '{print $1}'); ahead=$(echo $ah | awk '{print $2}')
echo "behind=$behind ahead=$ahead"

if [ "$behind" = "0" ]; then
  AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> git push
else
  echo "Concurrent commits on origin — work needs special handling. STOP and report."
fi
```

For a single-agent smoke test, `behind=0 ahead=1` is the normal post-T-d state and `git push` lands cleanly. If `behind > 0`, that means another agent pushed to `main` while you were working — that's a multi-agent scenario this single-agent handoff doesn't cover; report and ask the user.

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
  project_key="/Users/zacfarrell/Desktop/Testproject1",
  query="[<SMOKE_BEAD>] Complete", limit=5
)
# expect: at least one result, from=<your-name>, body mentions the commit hash
```

```bash
# reservations released — server-running-friendly check
grep -l '"agent":[[:space:]]*"<your-name>"' \
  /Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/projects/users-zacfarrell-desktop-testproject1/file_reservations/id-*.json 2>/dev/null \
  | xargs -I{} jq -r 'select(has("released_ts") | not) | "STILL ACTIVE: id=\(.id) path=\(.path_pattern) reason=\(.reason)"' {}
# expect: no output (all your reservations have released_ts set)
```

If `bin/br-complete` does its own internal git push, you may need to re-fetch before checking origin sync. After this, the smoke bead is closed and pushed; you're done with the primary test.

### 8i. Self-recovery if `bin/br-complete` fails mid-flight (read this BEFORE running §8h)

`bin/br-complete` is the most likely place the smoke test fails, because it does the most things sequentially: search Start mail → resolve commit hash → verify pushed → send Complete mail → `br close` → `bin/br-sync` → `git add .beads/` → `git commit` → `git pull --rebase` → `git push` → `release_file_reservations`. Any of those can fail. If it dies mid-sequence, the bead may be in a partial state — Start mail in archive, possibly Complete mail sent, possibly bead closed locally but not synced, possibly reservations still held.

**Crucial:** if you can't recover cleanly, **report BLOCKED per §13 with the full state inventory — DO NOT just walk away.** PlumMouse (Codex) ran an earlier version of this handoff on 2026-05-08 17:56 and abandoned smoke bead `testproject1-k9bg` mid-recovery; an audit-trail Complete mail (id=217) was filed manually after the fact. Avoid being the next PlumMouse — finish what you start, even if "finishing" means a clean BLOCKED report.

The recovery procedure below is the same template I (the previous IvoryMaple session) used to recover from my own `bin/br-complete` failure on `testproject1-l9vh` and to clean up PlumMouse's orphan. Run them **in this exact order**:

```bash
SMOKE_BEAD=$(cat /tmp/wrapper_fix_smoke_bead.txt)

# Step 1 — make sure your code commit (smoke marker) is on origin/main.
# If not, push first; if behind, see §8g concurrent-commits note.
git fetch
git rev-list --left-right --count origin/main...HEAD | awk '{print "behind:",$1," ahead:",$2}'
# If "ahead: >0", push:
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> git push

# Step 2 — close the bead manually with a reason that names this recovery.
br close "$SMOKE_BEAD" --reason "Smoke test bin/br-complete failed at <step>; closed manually by <your-name> for cleanup. See manual Complete mail in thread $SMOKE_BEAD."

# Step 3 — sync and commit the .beads/ change (exempt prefix, no Start mail needed).
bin/br-sync && git add .beads/
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> \
  git commit -m "beads: close $SMOKE_BEAD smoke-test (manual close after br-complete failure)"
AGENT_NAME=<your-name> AGENT_MAIL_NAME=<your-name> git push
```

Step 4 — send a manual Complete mail that explicitly says "this is recovery, not a successful smoke test." Use this template (substitute the bead-id and the failure-step):

```
send_message(
  project_key="/Users/zacfarrell/Desktop/Testproject1",
  sender_name="<your-name>",
  to=["<your-name>"],
  thread_id="<SMOKE_BEAD>",
  subject="[<SMOKE_BEAD>] Complete: manual recovery (br-complete failed)",
  body_md="**This is NOT a successful smoke-test completion.** bin/br-complete failed at step <X> with error:\n\n<paste exact error>\n\nManual recovery actions taken: bead closed, .beads/ change committed (commit <sha>) and pushed, this Complete mail sent, reservations released next. State will be reported in the BLOCKED report per §13.",
  ack_required=false,
  importance="normal"
)
```

Step 5 — release ALL your reservations (broad release is safe here because you're done with this run):

```
release_file_reservations(
  project_key="/Users/zacfarrell/Desktop/Testproject1",
  agent_name="<your-name>"
)
```

Step 6 — verify final state:

```bash
br show "$SMOKE_BEAD" --json | jq -r '..|objects|select(.id=="'"$SMOKE_BEAD"'")|.status'   # expect: closed
git status --porcelain | wc -l                                                              # expect: 0
git rev-list --count origin/main..HEAD                                                       # expect: 0
```

In your agent session:

```
search_messages(query="[<SMOKE_BEAD>]", limit=10)   # expect Start (1) + manual Complete (1)
```

Step 7 — if you also created an `<IDEM_BEAD>` and it's still `in_progress`, repeat steps 2-6 against `IDEM_BEAD` (you don't need to send a separate Complete mail for it since the idempotency probe never claimed to be a full lifecycle test; just close + manual mail noting "idempotency probe — closed manually").

Step 8 — write the BLOCKED report per §13 with full state. **Do not omit this** — the report is the only signal to the user that something went wrong and that L5 should NOT spawn yet.

> **Reference precedent.** A working example of this exact recovery exists in the agent-mail archive: thread `testproject1-l9vh` has a "[testproject1-l9vh] Complete: smoke test (manual recovery)" mail (id=214) from the original 2026-05-08 IvoryMaple session, and thread `testproject1-k9bg` has a "[testproject1-k9bg] Complete: orphan recovery (PlumMouse run abandoned)" mail (id=217). Read either if you want a template.

---

## 9. Validate→Fix→Test Loop policy

You will iterate until every acceptance criterion in §10 is true. Use this loop discipline:

### Cycle structure

1. **Identify the failing step.** Be exact: "step T-c manual idempotency probe — duplicate Start mail sent (count=2 instead of 1)."
2. **State a single hypothesis.** "The patch in `bin/br-claim` lines 270-278 doesn't include `result` in the key list."
3. **Apply ONE targeted change.** Don't rewrite multiple things at once — that obscures cause/effect.
4. **Re-run the **same** failing step plus the next downstream step.** Don't skip ahead. Don't re-run earlier passing steps unnecessarily, but DO re-run if your change touches their code path.
5. **If pass → continue down the smoke test.** If fail → return to (1) with the new diagnosis.

### Bounded retry

A clean Phase T run produces ~6 commits even on the happy path (create smoke bead, create idem bead, close idem bead, smoke marker, internal br-complete closure, plus optional fix commit). Use the bounds below as "something is structurally wrong" tripwires, not happy-path budgets.

- **Same step, same error, three distinct fix attempts:** STOP. The diagnosis is wrong. Read the relevant code top-to-bottom in full, write your understanding into the report, and escalate to the user.
- **More than 10 commits during this work without convergence:** STOP. Something is structurally off — perhaps the agent-mail server response shape changed mid-session, or another agent is editing the wrappers in parallel. Escalate with `git log --oneline -15` in the report.
- **More than 3 distinct smoke beads created (the main, the idempotency probe, plus one re-creation):** STOP. You should not need a fourth. Investigate what is causing repeated rebuilds.

### Debug methodology when stuck

When you hit a failure that doesn't match any row in the "fix in band vs escalate" table below, work this checklist top-to-bottom (cheapest first):

1. **Read the actual error message verbatim.** Don't paraphrase to yourself; copy-paste it. The message often names the failing function and the specific assertion.
2. **Check env vars.** `echo "AGENT_NAME=$AGENT_NAME AGENT_MAIL_NAME=$AGENT_MAIL_NAME"` — both should be your agent name. If empty, you're in a sub-shell that didn't inherit them.
3. **Re-confirm State A vs State B.** Re-run §5's `sed` blocks. A parallel agent may have changed the source.
4. **Run the §6 reproducer again.** If `BUGGY` is now `FOUND` or `FIXED` is now `EMPTY`, the server response shape has changed.
5. **Read the wrapper file end-to-end** for the failing wrapper. The bug might be in a code path you didn't anticipate (e.g., `prior_other_start` triggering on a stale mail from a different agent).
6. **Check the on-disk artifacts.** Mail at `~/.mcp_agent_mail_git_mailbox_repo/projects/users-zacfarrell-desktop-testproject1/messages/threads/<bead>.md`; reservations at `.../file_reservations/id-N.json`. The state-of-record on disk often clarifies what the wrapper saw.
7. **Search prior handoffs/reports.** `prompts/pre-l5-sanity-check-handoff.md` and the original smoke test report (in this conversation history) document the wrapper internals you may need.
8. **If still stuck after items 1-7, escalate** per the bounded-retry rule.

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
- [ ] **A4 Two fresh beads created.** Main smoke bead (`<SMOKE_BEAD>`, §8a) and idempotency-probe bead (`<IDEM_BEAD>`, §8d). Both have label `smoke-test`. Both start `open`. Neither is `testproject1-l9vh`.
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
  project_key="/Users/zacfarrell/Desktop/Testproject1",
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
  /Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/projects/users-zacfarrell-desktop-testproject1/file_reservations/id-*.json 2>/dev/null \
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

Session-start state: <State A | State B>
Fix commit on origin/main: <sha — e.g. adec8e4 or your new commit if you applied the patch>
Pre-fix verification (§6 reproducer): BUGGY=EMPTY, FIXED=FOUND
Post-fix verification (§7c reproducer): patched-logic match count >= 1

Patches present at end of session:
  bin/br-claim    lines 270-278 → for-loop iterates ("result", "messages", "results"); for-loop start at line 272
  bin/br-complete lines 274-282 → same pattern; for-loop start at line 276

Smoke beads (closed, label smoke-test):
  <SMOKE_BEAD>  — main lifecycle test (claim → commit → push → complete)
  <IDEM_BEAD>   — idempotency probe (manual prior-Start, then claim)

Mail counts:
  [<SMOKE_BEAD>]: Start=1 + Complete=1
  [<IDEM_BEAD>] Start: exactly 1 (proves idempotency — no duplicate sent)

Acceptance criteria: A1–A12 all green on a single end-to-end run.

Wrapper end-to-end flow verified — the previously-broken bin/br-complete
path now succeeds, and bin/br-claim idempotency is wired correctly.
L5 swarm is safe to spawn.

Recommended next step: paste the L5 kickoff prompt into the first
agent session and observe its first bead claim. If that succeeds,
proceed to spawn the rest of the swarm.

Optional follow-ups (NOT blockers — see Appendix A):
  - A.1 (path-parser regex requires backticks)
  - A.2 (pre-push hook AGENT_NAME requirement is undocumented)
  - A.3 (am file_reservations CLI competes with running server)
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

All line numbers below refer to the post-fix (current) file. The fix expanded the lookup block by +4 lines, so subtract 4 from each to get the pre-fix line if you need to read git history before commit `adec8e4`.

- **`bin/br-claim`:** with the fix, the lookup at line 272 finds the Start mail and `prior_self_start = True` is set at line 290. The early info-print at line 315 (`idempotency: prior Start from <agent> found → mail will be SKIPPED`) shows the wrapper recognized the case. Line 344 (`if not prior_self_start:`) then skips the redundant `send_message` call, and line 390 prints `✓ idempotent — Start mail already sent earlier`. Without the fix, the lookup returns empty, both flags stay `False`, and a partial-failure retry sends a duplicate Start mail every time.
- **`bin/br-complete`:** with the fix, the lookup at line 276 finds the Start mail. Line 294 (`if sender == agent: self_start_found = True`) fires correctly, line 297's `if not self_start_found:` falls through, and the rest of the complete lifecycle (close → sync → push → release) runs. Without the fix, the wrapper dies at lines 297-302 (`die(f"no \`[{bead_id}] Start\` mail from {agent!r} in the archive...")`) regardless of archive contents.

---

## Appendix C — What the prior smoke test proved (so you don't re-prove it)

These were already verified on 2026-05-08 by agent `IvoryMaple`, smoke bead `testproject1-l9vh`. You don't need to re-test these specific points if the wrappers/hook are unchanged from then:

- `bin/br-claim --dry-run` exits 0 with no side effects.
- `bin/br-claim` (real) creates a file reservation, sends a Start mail to peers (or self if no peers), and flips `br update --status=in_progress` atomically.
- The precommit hook ACCEPTS a `<bead-id>:`-prefixed commit when a Start mail exists in the archive within the lookback window.
- The precommit hook BLOCKS a `<bead-id>:`-prefixed commit when no Start mail exists, with an actionable error message that names the missing bead and references AGENTS.md Rule 2.
- The pre-push hook works (with AGENT_NAME set).

The thing that was NOT proven by the prior smoke test and that THIS handoff is testing: **`bin/br-complete` running cleanly end-to-end against a freshly-claimed bead** (it died on the response-key bug). Plus the new idempotency probe (§8d).

Phase T must independently re-prove the items in the bullet list above, not skip them — those proofs were against the pre-fix wrappers, and the patch could in principle have regressed an unrelated code path. You are re-testing the entire `br-claim` → commit → hook → push → `br-complete` chain against the post-fix wrappers. Don't shortcut.
