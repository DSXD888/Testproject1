---
title: AgentMail handshake bootstrap — universal wrapper + template fix
date: 2026-05-19
status: planned (execute AFTER L5 closes)
severity: medium-high  # degrades swarm coordination; ships with the wrappers; reused on every future project
component: bin/br-claim, bin/br-complete, AGENTS.md, layermaxxer template, beads-to-execution skill (user-level)
audience: implementing agent picking this up cold AFTER L5 (`testproject1-hn05.*`) is fully closed
predecessor: docs/handoff-2026-05-19-agentmail-wrapper-self-addressing.md  (error manifest — the "why")
operator: xiasshuang@gmail.com
---

# Handoff — AgentMail Handshake Bootstrap (Universal Fix)

> **DO NOT START THIS WORK WHILE L5 BEADS ARE STILL OPEN.** See §0.

---

## 0. Pre-flight — verify L5 is done

This handoff modifies `bin/br-claim`, `bin/br-complete`, and `AGENTS.md`. All three are load-bearing for any in-flight L5 swarm. Touching them mid-swarm risks:

- A half-written wrapper file getting executed by an active L5 agent.
- A merge conflict on `AGENTS.md` or `.beads/` against an L5 commit.
- Test agents created during §9 polluting the live L5 swarm's `list_agents` result.

The operator already applied a manual workaround for the live L5 swarm (`macro_contact_handshake` between each pair of L5 agents). The wrapper change is therefore safe to defer; it benefits future swarms, not the current one.

**Run all of these checks before any edit. ALL must be clean.**

```bash
# 1. No L5 task beads remain open or in-progress (allow only the root epic + phase sub-epics if those are still open).
br list --label layer-5 --status=open --json     | jq '.[] | .id' | grep -v -E '^"testproject1-hn05(\.[1-6])?"$'
br list --label layer-5 --status=in_progress --json | jq '.[] | .id'
# Both commands MUST return empty (or only the epic IDs you intend to leave open per L5 closure plan).

# 2. No L5 file reservations are held in AgentMail.
#    (Spawn the implementing agent's session first, then call:)
#       file_reservation_paths(project_key="/Users/alinaaitova/Desktop/Testproject1",
#                              agent_name=<you>, paths=[], ttl_seconds=1, exclusive=false)
#    is NOT the right tool — use the MCP read tool to list current reservations
#    (varies by mcp-agent-mail version; if no read tool, `am doctor archive-scan`
#    will show recent reservation files in the archive).

# 3. Working tree is clean on main.
git status        # must show "up to date with origin" + "nothing to commit"
git pull --rebase # confirm in sync with remote

# 4. No L5 swarm sessions are still alive — check AgentMail's list_agents for any
#    program="l5-expanded-commercial-intelligence-executor" with last_active_ts
#    inside the last 30 minutes. If yes, wait.
```

If any check fails: **stop**, send a `[handoff-block] AgentMail handshake fix paused: <reason>` mail to the operator, and wait. Do not proceed.

---

## 1. TL;DR

The `bin/br-claim` and `bin/br-complete` wrappers silently address every Start/Complete mail to `[<self>]` whenever `list_contacts(...)` returns `[]`. That happens by default on first claim, because `contact_policy: "open"` enables incoming requests but does NOT auto-create handshake links; without an explicit `macro_contact_handshake`, `list_contacts` is empty for every agent. Result: every swarm-generated wrapper mail goes nowhere visible, peers learn of each other only through file-reservation conflicts, and the Phase B precommit hook's "Start mail exists" check passes on a hollow signal.

The fix has three durable parts, applied in order:

1. **Wrapper-level** — replace the `recipients = peers if peers else [agent]` silent self-fallback with a layered discovery strategy (`list_contacts` → `list_agents` liveness filter → loud error). Add a `--solo` opt-in for legitimate single-agent operation.
2. **Documentation-level** — bake the bootstrap handshake into AGENTS.md Rule 2 and into the layermaxxer kickoff/push templates.
3. **Skill-level (cross-project lever)** — update `~/.claude/skills/beads-to-execution/TEMPLATES.md` so every future project's generated kickoff/push prompts inherit the handshake step automatically.

The bug is reproducible in 5 steps (predecessor manifest §3). The fix is reproducible in this document.

---

## 2. Why this exists (link to evidence)

Open `docs/handoff-2026-05-19-agentmail-wrapper-self-addressing.md` for the full error manifest — 4 affected agents, 16 self-addressed wrapper mails over 40 minutes in the L5 Phase 1 swarm of 2026-05-19, plus a counter-proof that direct `send_message` peer routing works fine. The manifest is dense; you do not need to re-read it to execute this handoff, but you must link to it from any commit message that lands the fix:

```
commit -m "agent-mail wrappers: bootstrap handshake + layered peer discovery (fixes the self-addressing bug documented in docs/handoff-2026-05-19-agentmail-wrapper-self-addressing.md)"
```

The manifest's §5 (Root Cause), §7 (Proposed fix), §10 (Cross-project portability), and §12 (Handoff notes for the implementing agent) are the design source for everything below. If something in this handoff seems contradictory to the manifest, the manifest wins — call out the conflict in your commit message and proceed conservatively.

---

## 3. The fix in one paragraph

A wrapper that has no peers is in a broken state, not a happy edge case. It must NEVER silently self-address. The corrected behaviour is: `list_contacts` first; if empty, `list_agents` filtered by liveness (default cutoff 1 h) and same-program; if still empty, raise `NoPeersError` with a clear diagnostic naming the recovery action (`macro_contact_handshake` or `--solo`). A new `--solo` flag (and `AGENT_MAIL_ALLOW_SOLO=1` env var) is the explicit opt-in for legitimate single-agent operation; in solo mode the wrapper still emits a loud warning rather than acting silently. Meanwhile the kickoff template grows a Step 3 "Handshake every visible peer via `macro_contact_handshake`" between `register_agent` and `fetch_inbox`, and AGENTS.md Rule 2 grows a one-paragraph addendum naming the handshake as a prerequisite for wrapper correctness.

---

## 4. Scope of changes

Seven artifacts change. Five live in this repo; one lives at user-level (`~/.claude/`); one is a brand-new test file.

| # | File | Type | Lives in |
|---|------|------|----------|
| 1 | `bin/br-claim` | edit | this repo |
| 2 | `bin/br-complete` | edit | this repo |
| 3 | `bin/tests/test_peer_discovery.py` | **new file** | this repo |
| 4 | `AGENTS.md` | edit (Rule 2 addendum + MCP Agent Mail section addendum) | this repo |
| 5 | `docs/layermaxxer/active/execution-kickoff.md` | edit (§5 AgentMail Coordination) | this repo |
| 6 | `docs/layermaxxer/active/execution-push.md` | edit (§2 AgentMail Coordination) | this repo |
| 7 | `~/.claude/skills/beads-to-execution/TEMPLATES.md` | edit (user-level — propagates to all future projects) | user home |

Out of scope (do **not** touch as part of this handoff):

- `docs/pipeline/l[1-5]-*/artifacts/l*-kickoff.md` and `l*-push.md` — historical, already-consumed artifacts. They drift naturally as projects advance; rewriting them now is churn.
- `bin/br-sync` — no peer-discovery logic; orthogonal.
- The mcp-agent-mail server itself — bug is client-side.
- The Phase B precommit hook at `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` — still works correctly; this handoff strengthens the upstream signal but does not change the hook's contract.

---

## 5. Order of operations (sequenced for safety)

Implement strictly in this order. Each step is committable on its own; do **not** bundle them.

| Step | Bead-equivalent | What lands | Why this order |
|------|-----------------|-----------|---------------|
| 5.1 | `(no bead — direct main)` | New test file `bin/tests/test_peer_discovery.py` + new helper `discover_peers(...)` duplicated in BOTH wrappers, **wired in behind a feature flag (`AGENT_MAIL_USE_LAYERED_DISCOVERY=1`) that defaults OFF** | Tests prove the helper before any wrapper behaviour changes. Old silent-fallback path is still the default. Zero behaviour change for any agent that pulls this commit without setting the flag. |
| 5.2 | `(no bead)` | Flip the wrapper default: helper becomes the primary path, silent-fallback is dropped. `--solo` and `AGENT_MAIL_ALLOW_SOLO=1` become the only ways to address self. Emit a clear `NoPeersError` otherwise. | Once helper is exercised by tests and at least one live swarm validates it (§9.4), drop the flag. |
| 5.3 | `(no bead)` | AGENTS.md Rule 2 addendum + MCP Agent Mail section addendum | Constitutional update — should land in the same commit as 5.2 so the rule and the wrapper behaviour ship together. |
| 5.4 | `(no bead)` | `docs/layermaxxer/active/execution-kickoff.md` §5 + `execution-push.md` §2 — add Step 3 handshake | Future layer kickoffs generated against these templates inherit the fix. |
| 5.5 | `(no bead — user-level skill)` | `~/.claude/skills/beads-to-execution/TEMPLATES.md` — add handshake block to the AgentMail-Setup template | Propagates to every future project the operator uses `beads-to-execution` in. This is the highest-leverage change of the whole handoff. |
| 5.6 | `(no bead)` | Live validation — spawn a 2-agent test swarm against the current project_key, run `bin/br-claim` on a throwaway bead, assert peer-routed Start mail | Final acceptance gate. |

Do NOT touch the existing L5 artifacts (`docs/pipeline/l5-.../artifacts/*`). They are frozen.

---

## 6. Change 1 — `bin/br-claim`

**Current state**: 464 lines. The defective block is at lines 396-419. See `bin/br-claim:396` for the exact slice.

### 6.1 Add the helper (insert near the top, after `mcp_call(...)` definition at line 137-197)

The helper duplicates by design — `bin/br-claim` and `bin/br-complete` already duplicate `mcp_call`, `preflight_health`, `resolve_agent_name`, `resolve_bearer_token`, `resolve_project_key`, and `br_show`. The project's chosen pattern is duplication, not a shared `bin/_br_lib.py`. Respect it. If you want a shared lib, file a separate bead.

```python
class NoPeersError(RuntimeError):
    """Raised when neither list_contacts nor list_agents yields any peer.

    Carries the diagnostic the user needs to recover (run macro_contact_handshake,
    confirm a second agent is registered, or pass --solo).
    """


def _parse_iso_ts(value):
    """Parse an ISO-8601 timestamp; tolerate trailing 'Z' and missing tz."""
    from datetime import datetime, timezone
    if not isinstance(value, str) or not value:
        return None
    text = value.strip()
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    try:
        dt = datetime.fromisoformat(text)
    except ValueError:
        return None
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt


def _normalize_peer_list(payload, *, exclude):
    """Flatten a list/dict response from list_contacts or list_agents into a
    deduped list of agent names, excluding `exclude` (self)."""
    out = []
    seen = set()
    def _add(name):
        if not name or not isinstance(name, str):
            return
        if name == exclude or name in seen:
            return
        seen.add(name)
        out.append(name)
    items = []
    if isinstance(payload, list):
        items = payload
    elif isinstance(payload, dict):
        for key in ("contacts", "agents", "result", "results", "items"):
            value = payload.get(key)
            if isinstance(value, list):
                items = value
                break
    for entry in items:
        if isinstance(entry, str):
            _add(entry)
        elif isinstance(entry, dict):
            _add(entry.get("name") or entry.get("agent_name"))
    return out


def discover_peers(
    url, token, project_key, agent,
    *,
    max_idle_seconds=3600,
    same_program=None,
    allow_solo=False,
    tool_label="br-claim",
):
    """Resolve recipients for Start/Complete mail.

    Strategy:
      1. list_contacts — preferred (explicit handshake links).
      2. list_agents — fallback, filtered by liveness (last_active_ts within
         max_idle_seconds) and optionally by same program.
      3. If both empty: raise NoPeersError unless allow_solo=True, in which
         case return ([agent], "solo") with a warning printed by the caller.

    Returns (recipients: list[str], source: str). `source` is one of
    "contacts", "list_agents", or "solo".
    """
    # 1. contacts (explicit handshake links)
    contacts_payload = mcp_call(url, token, "list_contacts", {
        "project_key": project_key, "agent_name": agent,
    })
    contacts = _normalize_peer_list(contacts_payload, exclude=agent)
    if contacts:
        return contacts, "contacts"

    # 2. fallback to list_agents, filtered by liveness + same program
    agents_payload = mcp_call(url, token, "list_agents", {
        "project_key": project_key,
    })
    candidates = []
    if isinstance(agents_payload, list):
        candidates = agents_payload
    elif isinstance(agents_payload, dict):
        for key in ("agents", "result", "results", "items"):
            value = agents_payload.get(key)
            if isinstance(value, list):
                candidates = value
                break
    from datetime import datetime, timedelta, timezone
    cutoff = datetime.now(timezone.utc) - timedelta(seconds=max_idle_seconds)
    live = []
    seen = set()
    for entry in candidates:
        if not isinstance(entry, dict):
            continue
        name = entry.get("name") or entry.get("agent_name")
        if not name or name == agent or name in seen:
            continue
        if same_program:
            prog = entry.get("program")
            if prog and prog != same_program:
                continue
        last = entry.get("last_active_ts") or entry.get("inception_ts")
        ts = _parse_iso_ts(last) if last else None
        # If the server doesn't surface a timestamp, treat the agent as live
        # rather than dropping it — better to over-include than to silently
        # ghost a peer.
        if ts is None or ts >= cutoff:
            seen.add(name)
            live.append(name)
    if live:
        return live, "list_agents"

    # 3. no peers anywhere
    if allow_solo:
        return [agent], "solo"
    raise NoPeersError(
        f"{tool_label}: no peers discoverable.\n"
        f"  list_contacts returned [] AND list_agents returned no live peers "
        f"(idle cutoff: {max_idle_seconds}s, program filter: {same_program!r}).\n"
        f"  Recovery options:\n"
        f"    1. Handshake an existing peer:\n"
        f"         macro_contact_handshake(project_key={project_key!r},\n"
        f"                                 agent_name={agent!r}, peer_name=<peer>)\n"
        f"    2. Confirm a second agent is registered in this project:\n"
        f"         list_agents(project_key={project_key!r})\n"
        f"    3. If this really is a swarm of one, opt in explicitly:\n"
        f"         {tool_label} <bead-id> --solo\n"
        f"         (or export AGENT_MAIL_ALLOW_SOLO=1)"
    )
```

### 6.2 Add CLI flags (modify `parse_args()` at line 261-287)

Add the following after the existing `--dry-run` argument:

```python
    p.add_argument(
        "--solo", action="store_true",
        help=(
            "Allow self-addressing when no peers are discoverable. "
            "Use only for genuine single-agent operation. "
            "Equivalent to AGENT_MAIL_ALLOW_SOLO=1."
        ),
    )
    p.add_argument(
        "--peer-max-idle-seconds", type=int, default=3600,
        help=(
            "When falling back to list_agents (because list_contacts is empty), "
            "treat agents idle longer than this as not-live. Default: 3600 (1h)."
        ),
    )
    p.add_argument(
        "--peer-same-program", default=None,
        help=(
            "When falling back to list_agents, only include peers whose program "
            "matches this string. Default: no program filter."
        ),
    )
```

### 6.3 Replace the defective block (lines 396-419 verbatim)

Old (DELETE — these are the 24 lines that ship the bug):

```python
    # 6. SEND START MAIL (unless idempotency says skip).
    if not prior_self_start:
        # Discover peers (never use "broadcast").
        peers_res = mcp_call(url, token, "list_contacts", {
            "project_key": project_key, "agent_name": agent,
        })
        peers = []
        if isinstance(peers_res, list):
            for p in peers_res:
                if isinstance(p, str) and p and p != agent:
                    peers.append(p)
                elif isinstance(p, dict):
                    n = p.get("agent_name") or p.get("name")
                    if n and n != agent:
                        peers.append(n)
        elif isinstance(peers_res, dict):
            for p in peers_res.get("contacts", []) or []:
                if isinstance(p, dict):
                    n = p.get("agent_name") or p.get("name")
                    if n and n != agent:
                        peers.append(n)
                elif isinstance(p, str) and p != agent:
                    peers.append(p)
        recipients = peers if peers else [agent]
```

New (WRITE — uses the layered helper, fails loud on no peers, accepts `--solo`):

```python
    # 6. SEND START MAIL (unless idempotency says skip).
    if not prior_self_start:
        allow_solo = bool(args.solo) or os.environ.get("AGENT_MAIL_ALLOW_SOLO", "").strip() in ("1", "true", "yes")
        try:
            recipients, source = discover_peers(
                url, token, project_key, agent,
                max_idle_seconds=args.peer_max_idle_seconds,
                same_program=args.peer_same_program,
                allow_solo=allow_solo,
                tool_label="br-claim",
            )
        except NoPeersError as e:
            die(str(e))
        if source == "solo":
            warn(
                "no peers found — proceeding in solo mode. "
                "Start mail will be self-addressed (audit-only)."
            )
        elif source == "list_agents":
            warn(
                "no contacts found; falling back to list_agents liveness filter. "
                f"Recipients: {recipients!r}. "
                "Run macro_contact_handshake to make this deterministic."
            )
```

The rest of the block (building `body_md`, calling `send_message`, printing the result) stays unchanged. Line 438's success log already reads `print(f"br-claim: ✓ sent Start mail to {recipients}")` — keep it as-is; it now reports the correct list.

### 6.4 Update the top-of-file docstring

The order doc-comment at line 4-12 currently says:

```python
"""br-claim — atomically claim a bead.

Order (do not reorder):
  1. resolve agent identity ($AGENT_MAIL_NAME, fallback $AGENT_NAME)
  ...
  5. list_contacts → peers; send_message Start (to peers + self)
  6. `br update <id> --status=in_progress`
"""
```

Change step 5 to:

```
  5. discover_peers (list_contacts, then list_agents liveness fallback;
     loud error if neither yields peers and --solo not set);
     send_message Start to those peers (NEVER silently self)
```

---

## 7. Change 2 — `bin/br-complete`

**Current state**: 476 lines. The defective block is at lines 374-395.

Apply the same three edits in the same order.

### 7.1 Add the helper

Paste the entire `class NoPeersError`, `_parse_iso_ts`, `_normalize_peer_list`, and `discover_peers` block from §6.1 above into `bin/br-complete`, after the `mcp_call(...)` definition (around line 190). Change the default `tool_label="br-claim"` to `tool_label="br-complete"`.

### 7.2 Add CLI flags

Paste the three `add_argument` calls from §6.2 into `parse_args()` at line 278-299.

### 7.3 Replace the defective block (lines 374-395)

Old (DELETE):

```python
    # 4. SEND COMPLETE MAIL.
    peers_res = mcp_call(url, token, "list_contacts", {
        "project_key": project_key, "agent_name": agent,
    })
    peers = []
    if isinstance(peers_res, list):
        for p in peers_res:
            if isinstance(p, str) and p and p != agent:
                peers.append(p)
            elif isinstance(p, dict):
                n = p.get("agent_name") or p.get("name")
                if n and n != agent:
                    peers.append(n)
    elif isinstance(peers_res, dict):
        for p in peers_res.get("contacts", []) or []:
            if isinstance(p, dict):
                n = p.get("agent_name") or p.get("name")
                if n and n != agent:
                    peers.append(n)
            elif isinstance(p, str) and p != agent:
                peers.append(p)
    recipients = peers if peers else [agent]
```

New (WRITE):

```python
    # 4. SEND COMPLETE MAIL.
    allow_solo = bool(args.solo) or os.environ.get("AGENT_MAIL_ALLOW_SOLO", "").strip() in ("1", "true", "yes")
    try:
        recipients, source = discover_peers(
            url, token, project_key, agent,
            max_idle_seconds=args.peer_max_idle_seconds,
            same_program=args.peer_same_program,
            allow_solo=allow_solo,
            tool_label="br-complete",
        )
    except NoPeersError as e:
        die(str(e))
    if source == "solo":
        warn(
            "no peers found — proceeding in solo mode. "
            "Complete mail will be self-addressed (audit-only)."
        )
    elif source == "list_agents":
        warn(
            "no contacts found; falling back to list_agents liveness filter. "
            f"Recipients: {recipients!r}. "
            "Run macro_contact_handshake to make this deterministic."
        )
```

### 7.4 Update top-of-file docstring

Change line 11 from:

```
  6. send_message [<id>] Complete (to peers + self)
```

to:

```
  6. discover_peers (list_contacts, then list_agents liveness fallback;
     loud error if neither yields peers and --solo not set);
     send_message [<id>] Complete to those peers (NEVER silently self)
```

---

## 8. Change 3 — `bin/tests/test_peer_discovery.py` (NEW FILE)

`bin/tests/` does not exist yet. Create it.

```bash
mkdir -p bin/tests
```

Then create the test file. The wrappers shell out to a live `mcp-agent-mail` server via `mcp_call(...)`; do NOT mock that — write the tests against the real MCP server using a throwaway `project_key`. This matches the project's no-mocks convention (AGENTS.md → MCP Agent Mail section) and the predecessor manifest §12 ("write the wrapper test fixtures to use AgentMail's actual MCP surface (a test-only project_key + ephemeral DB), not in-process mocks").

```python
#!/usr/bin/env python3
"""Tests for discover_peers(...) in bin/br-claim and bin/br-complete.

Runs against the live mcp-agent-mail HTTP server on 127.0.0.1:8765. Each test
uses a unique throwaway project_key (UUID-suffixed) so it does not pollute any
real project's archive. Cleanup at end-of-module via release_file_reservations
and (where supported by the server build) a project teardown call.

Skipped if mcp-agent-mail is not reachable.

Run:
    python3 -m unittest bin.tests.test_peer_discovery
"""

import importlib.util
import os
import socket
import sys
import unittest
import uuid


REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
BR_CLAIM = os.path.join(REPO_ROOT, "bin", "br-claim")
MCP_URL = os.environ.get("AGENT_MAIL_MCP_URL", "http://127.0.0.1:8765/mcp/")


def _load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def _agentmail_reachable():
    """Probe TCP 8765 on localhost. Returns True if open, False otherwise."""
    try:
        with socket.create_connection(("127.0.0.1", 8765), timeout=1):
            return True
    except OSError:
        return False


@unittest.skipUnless(_agentmail_reachable(),
                     "mcp-agent-mail not reachable on 127.0.0.1:8765")
class DiscoverPeersTests(unittest.TestCase):

    @classmethod
    def setUpClass(cls):
        cls.br_claim = _load_module(BR_CLAIM, "br_claim_under_test")
        cls.token = cls.br_claim.resolve_bearer_token()
        # Unique project_key per test run so we never collide with real work.
        cls.project_key = f"/tmp/test-peer-discovery-{uuid.uuid4().hex[:12]}"
        cls.br_claim.mcp_call(MCP_URL, cls.token, "ensure_project", {
            "human_key": cls.project_key,
        })

    @classmethod
    def tearDownClass(cls):
        # Best-effort cleanup. The test project_key is unique so leftover state
        # is benign; if the server build supports a project teardown tool, call
        # it here.
        pass

    def _register(self, suffix):
        """Register a new agent in the test project, return its name."""
        res = self.br_claim.mcp_call(MCP_URL, self.token, "register_agent", {
            "project_key": self.project_key,
            "program": "peer-discovery-test",
            "model": "test",
            "task_description": f"peer-discovery test agent {suffix}",
        })
        name = res.get("name") if isinstance(res, dict) else None
        self.assertIsNotNone(name, f"register_agent returned no name: {res!r}")
        self.br_claim.mcp_call(MCP_URL, self.token, "set_contact_policy", {
            "project_key": self.project_key,
            "agent_name": name,
            "policy": "open",
        })
        return name

    # ---- T1 — list_contacts returns peers; prefer those ---------------------

    def test_T1_contacts_present_returns_contacts(self):
        a = self._register("T1a")
        b = self._register("T1b")
        self.br_claim.mcp_call(MCP_URL, self.token, "macro_contact_handshake", {
            "project_key": self.project_key,
            "agent_name": a,
            "peer_name": b,
        })
        peers, source = self.br_claim.discover_peers(
            MCP_URL, self.token, self.project_key, a,
        )
        self.assertEqual(source, "contacts")
        self.assertIn(b, peers)
        self.assertNotIn(a, peers)

    # ---- T2 — no contacts, list_agents has live peers -----------------------

    def test_T2_no_contacts_list_agents_live(self):
        a = self._register("T2a")
        b = self._register("T2b")
        # Deliberately do NOT handshake.
        peers, source = self.br_claim.discover_peers(
            MCP_URL, self.token, self.project_key, a,
            max_idle_seconds=3600,
        )
        self.assertEqual(source, "list_agents")
        self.assertIn(b, peers)
        self.assertNotIn(a, peers)

    # ---- T3 — no contacts, list_agents has only stale peers -----------------
    # Requires the server to expose last_active_ts; if it doesn't, the helper
    # treats agents as live (over-include rather than ghost). In that case this
    # test reduces to T2; document the limitation rather than gaming it.

    def test_T3_stale_only_raises_or_returns_live(self):
        a = self._register("T3a")
        b = self._register("T3b")
        try:
            peers, source = self.br_claim.discover_peers(
                MCP_URL, self.token, self.project_key, a,
                max_idle_seconds=0,  # cutoff is "now", everything is stale
            )
            # If the server surfaces no timestamp the helper falls through to
            # "include anyway"; assert that we at least did NOT silently
            # self-address.
            self.assertNotEqual(peers, [a])
            self.assertEqual(source, "list_agents")
        except self.br_claim.NoPeersError:
            pass  # acceptable: server provided timestamps and they were stale

    # ---- T4 — no contacts, no other agents → NoPeersError -------------------

    def test_T4_solo_raises(self):
        a = self._register("T4a")
        with self.assertRaises(self.br_claim.NoPeersError) as cm:
            self.br_claim.discover_peers(
                MCP_URL, self.token, self.project_key, a,
            )
        msg = str(cm.exception)
        self.assertIn("macro_contact_handshake", msg)
        self.assertIn("--solo", msg)

    # ---- T5 — allow_solo=True returns [self], source="solo" -----------------

    def test_T5_allow_solo_returns_self(self):
        a = self._register("T5a")
        peers, source = self.br_claim.discover_peers(
            MCP_URL, self.token, self.project_key, a,
            allow_solo=True,
        )
        self.assertEqual(peers, [a])
        self.assertEqual(source, "solo")

    # ---- T6 — dedup across list_contacts and list_agents shapes -------------

    def test_T6_dedup_and_self_filter(self):
        a = self._register("T6a")
        b = self._register("T6b")
        self.br_claim.mcp_call(MCP_URL, self.token, "macro_contact_handshake", {
            "project_key": self.project_key,
            "agent_name": a,
            "peer_name": b,
        })
        peers, _ = self.br_claim.discover_peers(
            MCP_URL, self.token, self.project_key, a,
        )
        # b appears exactly once; a (self) never appears.
        self.assertEqual(peers.count(b), 1)
        self.assertNotIn(a, peers)


if __name__ == "__main__":
    unittest.main()
```

`bin/br-claim` is not a `.py` file (no extension), so the test loads it via `importlib.util.spec_from_file_location` — that's why it's not a normal import. The same loader trick works against `bin/br-complete`.

Run:

```bash
python3 -m unittest bin.tests.test_peer_discovery -v
```

(Or invoke the file directly: `python3 bin/tests/test_peer_discovery.py`.)

If `mcp-agent-mail` is not running, the entire test class is skipped — that's by design; we don't want CI to fail on a missing daemon. Document this in `bin/tests/README.md` if you want, but it's not required.

---

## 9. Change 4 — `AGENTS.md`

Two surgical edits. Both belong in the same commit as the wrapper changes (§6 + §7).

### 9.1 Rule 2 addendum (around line 28)

Find this paragraph (currently `AGENTS.md:28`):

```
You MUST NOT use raw `br update --status=in_progress` or `br close <id>` directly — they bypass Agent Mail and file reservations, which leaves the swarm unable to detect collisions. The pre-commit hook at `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` enforces file-reservation conflicts today; the Phase B extension to that hook (when active) will additionally block any commit with a `<bead-id>:` message prefix that has no corresponding `[<bead-id>] Start` mail in the archive.
```

Insert a NEW paragraph immediately after it:

```
**Prerequisite — bootstrap contact handshake.** `bin/br-claim` and `bin/br-complete` send Start/Complete mail to the peers returned by `list_contacts(...)`. `contact_policy: "open"` enables incoming requests but does NOT auto-create handshake links — without an explicit `macro_contact_handshake(...)`, `list_contacts` is empty and the wrappers fall back to a liveness-filtered `list_agents` (or raise `NoPeersError` if neither yields peers). For deterministic peer routing, run `macro_contact_handshake` against every visible peer at session start (see the "MCP Agent Mail" section below) and refresh after any swarm-growth event. The wrappers also accept `--solo` (or `AGENT_MAIL_ALLOW_SOLO=1`) for legitimate single-agent operation, which still emits a loud warning rather than acting silently.
```

### 9.2 MCP Agent Mail section addendum (around line 222, inside "Same Repository Workflow")

Find this block (currently `AGENTS.md:221-223`):

```
1. **Register identity:**
   ```
   ensure_project(project_key=<abs-path>)
   register_agent(project_key, program, model)
   ```
```

Replace with:

```
1. **Register identity, set policy, AND handshake peers:**
   ```
   ensure_project(project_key=<abs-path>)
   register_agent(project_key, program, model)
   set_contact_policy(project_key, agent_name=<you>, policy="open")

   # CRITICAL: handshake every live peer. contact_policy="open" only
   # accepts incoming contact requests; it does not establish bilateral
   # links. Without these handshakes, list_contacts(<you>) is empty
   # and bin/br-claim / bin/br-complete fall back to list_agents
   # (or raise NoPeersError).
   for peer in list_agents(project_key):
       if peer["name"] != <you>:
           macro_contact_handshake(
               project_key=project_key,
               agent_name=<you>,
               peer_name=peer["name"],
           )
   ```
```

### 9.3 Common Pitfalls addendum (around line 256)

Add a fourth bullet:

```
- **Empty `list_contacts` after `set_contact_policy(policy="open")`**: expected. Policy enables incoming requests; it does not auto-handshake. Run `macro_contact_handshake` (see Same Repository Workflow step 1) before relying on `list_contacts(...)` for routing. `bin/br-claim` / `bin/br-complete` will refuse to silently self-address — they fall back to `list_agents` and, failing that, raise `NoPeersError`.
```

---

## 10. Change 5 — `docs/layermaxxer/active/execution-kickoff.md`

The layermaxxer template at `docs/layermaxxer/active/execution-kickoff.md` is the source for every layermaxxer execution prompt. Currently §5.1 (around line 175) registers via `macro_start_session` and fetches inbox but does NOT handshake.

Find §5.1 "Session Startup (Do This Once at Session Start)" and add a sub-step between registration and the inbox check:

```
### 5.1 Session Startup (Do This Once at Session Start)

Register your identity and check for existing activity. Use the macro for speed:

```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="layermaxxer-executor",
  model="your-model"
)
```

This registers your agent, fetches your inbox, and shows active file reservations in one call.

**THEN — bootstrap contact handshakes (REQUIRED before any `bin/br-claim`).**

`macro_start_session` registers and may set `contact_policy="open"`, but it does NOT establish bilateral contact links. Without an explicit handshake, `list_contacts(<you>)` is empty and the `bin/br-claim` / `bin/br-complete` wrappers fall back to a liveness-filtered `list_agents`, or — if that is also empty — raise `NoPeersError`. For deterministic peer routing:

```
peers = list_agents(project_key="/Users/alinaaitova/Desktop/Testproject1")
for peer in peers:
    if peer["name"] != <your-name>:
        macro_contact_handshake(
            project_key="/Users/alinaaitova/Desktop/Testproject1",
            agent_name=<your-name>,
            peer_name=peer["name"],
        )
```

Refresh this handshake loop whenever you notice a new peer in `list_agents` that is not in your `list_contacts` — i.e., on every fresh inbox check, before each new `bin/br-claim`.

If `macro_start_session` is unavailable, do it manually:
...
```

(The remainder of §5.1 — the manual fallback — is unchanged.)

---

## 11. Change 6 — `docs/layermaxxer/active/execution-push.md`

§2.1 (around line 40-50) currently re-registers and fetches inbox. Add the same handshake sub-step.

Find §2.1 "Re-Register and Check Inbox" and immediately after the `macro_start_session(...)` block, insert:

```
**Refresh contact handshakes.** Other agents may have joined the swarm between your sessions. Without an explicit handshake, the `bin/br-claim` / `bin/br-complete` wrappers cannot peer-route Start/Complete mail to them:

```
peers = list_agents(project_key="/Users/alinaaitova/Desktop/Testproject1")
contacts = list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)
contact_names = {c["name"] if isinstance(c, dict) else c for c in (contacts or [])}
for peer in peers:
    if peer["name"] != <your-name> and peer["name"] not in contact_names:
        macro_contact_handshake(
            project_key="/Users/alinaaitova/Desktop/Testproject1",
            agent_name=<your-name>,
            peer_name=peer["name"],
        )
```

Do this on every session resume and after any "swarm grows" event.
```

---

## 12. Change 7 — `~/.claude/skills/beads-to-execution/TEMPLATES.md` (USER-LEVEL)

**This is the highest-leverage change of the entire handoff.** `~/.claude/skills/beads-to-execution/TEMPLATES.md` is the template file the `/beads-to-execution` skill consumes to generate kickoff/push prompts for every future project. The L5 `10-generate-kickoff.md` already invokes this skill (its very first instruction is "Invoke the `/beads-to-execution` skill"). Updating this template = every future project's kickoff prompt automatically includes the handshake step, with no per-project work.

### 12.1 Locate the AgentMail Setup block

Open `~/.claude/skills/beads-to-execution/TEMPLATES.md`. Search for the verbatim AgentMail registration block — it is the canonical template the skill emits for "Section 5 — AgentMail Setup (Do This ONCE At Session Start)" in generated kickoff prompts. It currently contains some variant of:

```
ensure_project(project_key="<project>")
register_agent(project_key, program=<program>, model=<model>)
set_contact_policy(project_key, agent_name, policy="open")
fetch_inbox(project_key, agent_name, limit=20)
```

If the file is structured by named sections (likely — it's a templates file), find the section that emits this block. If the file is structured as a flat list of verbatim chunks, search for `set_contact_policy`.

### 12.2 Insert the handshake step

Between `set_contact_policy` and `fetch_inbox`, add:

```

# Bootstrap contact handshakes — REQUIRED before any bin/br-claim.
# contact_policy="open" enables incoming requests but does NOT auto-create
# handshake links. Without these calls, list_contacts(<you>) is empty and
# the bin/br-claim / bin/br-complete wrappers fall back to list_agents
# (or raise NoPeersError if no live peers exist).
peers = list_agents(project_key="<project>")
for peer in peers:
    if peer["name"] != <agent-name>:
        macro_contact_handshake(
            project_key="<project>",
            agent_name=<agent-name>,
            peer_name=peer["name"],
        )
```

### 12.3 Add the "refresh handshakes" guidance to the Quick Reference template

In the same `TEMPLATES.md`, find the "AgentMail Quick Reference" template block (the one that becomes §11 of generated kickoffs and §8 of generated push prompts). Add a note:

```
**Refresh handshakes on swarm growth.** If `list_agents` shows a new peer not yet in your `list_contacts`, call `macro_contact_handshake(...)` for that peer before your next `bin/br-claim`. The wrappers' layered peer-discovery (list_contacts → list_agents → loud error) covers you in the meantime, but explicit handshakes make peer routing deterministic.
```

### 12.4 Same edit for sibling skills (only if they emit AgentMail-setup blocks)

Some other skills under `~/.claude/skills/` may also emit AgentMail setup blocks. Grep:

```bash
grep -l 'set_contact_policy' ~/.claude/skills/*/TEMPLATES.md ~/.claude/skills/*/references/TEMPLATES.md 2>/dev/null
```

Likely candidates: `pipemaxxer/TEMPLATES.md`, `layermaxxer/references/TEMPLATES.md`. Apply the same handshake-insertion edit to each match. Do NOT touch skills that don't emit AgentMail setup blocks (e.g., `spec-to-tdd`, `codebase-to-spec`).

### 12.5 Commit message at user-level

`~/.claude/` is the operator's home; it is not under this project's git. If the operator versions their `.claude/` directory separately, follow that convention. Otherwise the edits are in-place — note the change in the operator handoff back-channel (see §15).

---

## 13. Tests + verification

### 13.1 Unit tests (the new file from §8)

```bash
# With mcp-agent-mail running on 127.0.0.1:8765:
python3 -m unittest bin.tests.test_peer_discovery -v

# Expected: 6 tests pass (T1, T2, T3, T4, T5, T6).
# Without mcp-agent-mail: 6 tests skipped, exit 0.
```

### 13.2 Wrapper smoke test (no real bead)

```bash
# Pick a non-existent bead-id and --dry-run. The wrapper should fail at the
# br_show step ("bead not found"), NOT at the peer-discovery step.
bin/br-claim nonexistent-bead-id --dry-run --paths /tmp/test
# Expected exit: non-zero, error message about bead not found.

# With AGENT_MAIL_ALLOW_SOLO=1 and a real bead in a single-agent project:
AGENT_MAIL_ALLOW_SOLO=1 bin/br-claim <real-bead-id> --dry-run
# Expected: prints solo warning, completes dry-run.
```

### 13.3 No-peers regression

Single-agent scenario: register yourself only, run `bin/br-claim`. Expected:

```
br-claim: br-claim: no peers discoverable.
  list_contacts returned [] AND list_agents returned no live peers (idle cutoff: 3600s, program filter: None).
  Recovery options:
    1. Handshake an existing peer:
         macro_contact_handshake(...)
    2. Confirm a second agent is registered in this project:
         list_agents(...)
    3. If this really is a swarm of one, opt in explicitly:
         br-claim <bead-id> --solo
         (or export AGENT_MAIL_ALLOW_SOLO=1)
```

Exit code non-zero. No Start mail archived.

### 13.4 Live swarm validation (the acceptance gate)

After §6–§12 have all landed, spawn a 2-agent test swarm:

1. Open two terminal panes; in each, register against the live project:
   ```
   macro_start_session(project_key="/Users/alinaaitova/Desktop/Testproject1",
                       program="handshake-fix-validation",
                       model="<your-model>",
                       task_description="post-fix validation")
   ```
2. In **each** session, run the new handshake loop from §10.
3. Create a throwaway bead in `br` and run `bin/br-claim <throwaway>` in one of the panes (or use a real bead that genuinely needs claiming).
4. Inspect:
   ```
   search_messages(project_key="/Users/alinaaitova/Desktop/Testproject1",
                   query="Start", limit=5, ranking="recency")
   ```
   The Start mail's `to` field must contain the **other** pane's agent name, NOT the sender.
5. Repeat for `bin/br-complete`.
6. Tear down: `release_file_reservations` for both agents.

If step 4's assertion holds: acceptance gate passed.

---

## 14. Acceptance criteria

All of the following must hold before this handoff is considered complete:

- [ ] `bin/br-claim:396-419` no longer contains `recipients = peers if peers else [agent]`.
- [ ] `bin/br-complete:374-395` no longer contains `recipients = peers if peers else [agent]`.
- [ ] Both wrappers contain the `discover_peers(...)` helper (duplicated, per §6.1 / §7.1).
- [ ] Both wrappers expose `--solo`, `--peer-max-idle-seconds`, `--peer-same-program` CLI flags and honour `AGENT_MAIL_ALLOW_SOLO=1`.
- [ ] `bin/tests/test_peer_discovery.py` exists; T1–T6 pass against a live `mcp-agent-mail` server; the entire class is skipped (not failed) if the server is not reachable.
- [ ] `AGENTS.md` Rule 2 (line ~28-30) has the bootstrap-handshake addendum.
- [ ] `AGENTS.md` MCP Agent Mail section step 1 includes the handshake loop.
- [ ] `AGENTS.md` Common Pitfalls section has the "empty list_contacts after open policy" bullet.
- [ ] `docs/layermaxxer/active/execution-kickoff.md` §5.1 has the bootstrap handshake sub-step.
- [ ] `docs/layermaxxer/active/execution-push.md` §2.1 has the refresh-handshake sub-step.
- [ ] `~/.claude/skills/beads-to-execution/TEMPLATES.md` AgentMail-Setup block includes the handshake loop between `set_contact_policy` and `fetch_inbox`.
- [ ] `~/.claude/skills/beads-to-execution/TEMPLATES.md` Quick Reference block includes the "refresh handshakes on swarm growth" guidance.
- [ ] Live swarm validation (§13.4) shows peer-addressed Start/Complete mail.
- [ ] One commit lands all repo-side changes with a message referencing both this handoff and the predecessor error manifest.
- [ ] `MEMORY.md` index entry for the error manifest is updated to note "fix landed YYYY-MM-DD — see docs/handoff-2026-05-19-agentmail-handshake-bootstrap-fix.md".

---

## 15. Risk / rollback

### 15.1 Risk: a future single-agent session breaks under the new "loud error" behaviour

Mitigation: `--solo` and `AGENT_MAIL_ALLOW_SOLO=1` are documented in `--help` output AND in the `NoPeersError` diagnostic text. Solo mode still works; it just demands explicit opt-in. Document this in the commit message.

### 15.2 Risk: the wrapper-level helper diverges between `bin/br-claim` and `bin/br-complete` over time

Mitigation: this handoff lands the helper as a verbatim duplicate (matching the project's existing pattern for `mcp_call`, `preflight_health`, etc.). If divergence becomes a real problem (rather than a theoretical one), file a separate bead to factor out `bin/_br_lib.py`. Do NOT pre-extract — premature abstraction has cost and the duplication is currently small.

### 15.3 Risk: the global `~/.claude/skills/beads-to-execution/TEMPLATES.md` edit affects other projects mid-iteration

Mitigation: the edit is purely additive — it inserts a handshake step between two existing steps. Generated kickoffs for projects that don't have multi-agent swarms simply do an extra `list_agents` call that returns `[]` and a no-op loop. Cost: one extra MCP call per session start. Benefit: bug is impossible by construction in every future project.

### 15.4 Rollback

If any wrapper change destabilises a future swarm:

```bash
git revert <commit-sha>     # reverts the wrappers and AGENTS.md addendum
git push
```

The kickoff/push template edits (`docs/layermaxxer/...`) are documentation-only and harmless to keep. The `~/.claude/skills/...` edit is operator-side; revert manually by re-deleting the handshake block from the template.

The error manifest (`docs/handoff-2026-05-19-agentmail-wrapper-self-addressing.md`) and this handoff document remain in `docs/` regardless — they are evidence, not code.

---

## 16. Out-of-scope deferred items

These are mentioned in the predecessor manifest §10 as "nice to have, not blocking". Do not pursue as part of this handoff; file separate beads if you want them:

- `bin/br-bootstrap-handshake` — a single-shot script wrapping the handshake loop. Better DX than embedding the loop in every kickoff. Idempotent. Worth doing once the dust settles.
- `bin/br-claim --bootstrap` — handshake-then-claim atomic flag.
- `.claude/agents/agent-mail-bootstrap.md` — project-template skill file the kickoff prompt can reference. Future projects copy the template.
- Factoring `bin/br-claim`/`bin/br-complete` shared helpers into `bin/_br_lib.py` — only do this once divergence is a real problem.
- A "swarm topology" view in the `mcp-agent-mail` MCP server itself, surfacing handshake-graph completeness — server-side, separate project.

---

## 17. References

### This repo
- `bin/br-claim:396-419` — current self-addressing bug site (claim path)
- `bin/br-complete:374-395` — current self-addressing bug site (complete path)
- `bin/br-claim:137-197` — `mcp_call(...)` helper (where `discover_peers` insertion goes)
- `bin/br-complete:136-190` — same
- `AGENTS.md:21-33` — Rule 2 (constitutional)
- `AGENTS.md:209-258` — MCP Agent Mail section
- `docs/handoff-2026-05-19-agentmail-wrapper-self-addressing.md` — error manifest (the "why")
- `docs/handoff-2026-05-05-mcp-agent-mail-db-recovery.md` — predecessor incident (SQLite drift); demonstrates the wrappers' preflight pattern this fix should match in tone
- `docs/layermaxxer/active/execution-kickoff.md:171-208` — current AgentMail §5.1 (target for §10 edit)
- `docs/layermaxxer/active/execution-push.md:36-60` — current AgentMail §2.1 (target for §11 edit)
- `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` — Phase B precommit hook (still works correctly; this fix strengthens its upstream signal but does not change its contract)

### User-level
- `~/.claude/skills/beads-to-execution/TEMPLATES.md` — universal template (§12 edit)
- `~/.claude/skills/beads-to-execution/SKILL.md` — the skill definition that consumes TEMPLATES.md
- `~/.claude/skills/pipemaxxer/TEMPLATES.md` — sibling skill (check via §12.4 grep)
- `~/.claude/skills/layermaxxer/references/TEMPLATES.md` — sibling skill (check via §12.4 grep)

### AgentMail MCP tools (all already available)
- `register_agent`, `ensure_project`, `macro_start_session` — registration
- `set_contact_policy` — `open` / `auto` / `restricted`
- `list_contacts` — explicit handshake links (preferred peer source)
- `list_agents` — all registered agents in a project (fallback peer source)
- `macro_contact_handshake` — single-call handshake (preferred for bootstrap)
- `request_contact` + `respond_contact` — manual handshake (granular alternative)
- `send_message` — peer-addressed mail (works correctly when `to` is populated)
- `file_reservation_paths` — DB-level collision detection (orthogonal; unchanged)

### Memory-side
- `MEMORY.md` index entry for the error manifest — update after fix lands per §14.

---

## 18. Handoff notes for the implementing agent

- Read §0 first. If L5 isn't done, stop.
- The predecessor error manifest (`docs/handoff-2026-05-19-agentmail-wrapper-self-addressing.md`) is dense; you do not need to re-read it cover-to-cover, but you must cite it in your commit message and you must accept its design decisions (especially §7's no-silent-self-addressing rule). If you disagree with the design, write a counter-proposal as a new handoff document; do NOT silently diverge.
- Land steps 5.1 → 5.6 in order. Do not skip the feature-flag step (5.1) — it is the only safety net if the helper is wrong.
- The wrappers' existing duplication pattern is intentional (per `mcp_call`, `preflight_health`, etc.). Do not pre-extract a shared lib.
- The wrappers' existing tone (`die`, `warn`, plain stderr) is intentional. Match it.
- `AGENTS.md` is constitutional. Do not casually re-format it; restrict your edits to the named insertions in §9.
- The `~/.claude/` edit is operator-side. The operator may keep their `.claude/` directory under separate VCS; check `~/.claude/.git` before deciding how to commit it. If unversioned, simply edit in-place and note the change in your wrap-up message.
- After landing, send a `[handoff-done] agent-mail handshake bootstrap fix landed` mail to the operator's session (or whoever's current `program=l6-*-executor` is most recently active) so they know the wrapper protocol changed and to refresh their session.
- Update `MEMORY.md` per §14 — index entry only, one line under ~200 chars.
