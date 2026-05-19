# Error Manifest — AgentMail Wrapper Self-Addressing

**Date:** 2026-05-19
**Reporter:** L5 swarm monitor session (claude-opus-4-7)
**Severity:** MEDIUM-HIGH — degrades swarm coordination, not currently catastrophic, compounds with parallelism
**Affected:**
- `bin/br-claim` (lines 396-443, esp. 397-419)
- `bin/br-complete` (lines 374-412, esp. 375-395)
- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-kickoff.md` §5 (AgentMail Setup)
- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-push.md` §1 (re-orient block)

**Cross-project relevance:** HIGH. The wrappers are the project's constitutional Rule 2 mechanism (AGENTS.md:21-33). They are intended to be reused unchanged on future machines and projects. The bug ships with them.

---

## 1. Summary

When ≥2 agents register with AgentMail in quick succession and then run `bin/br-claim` / `bin/br-complete`, every Start and Complete mail is silently addressed `to=[<self>]` instead of `to=[<peer-names>]`. Peers never see each other's claims or completions through the mail archive. Collision protection still works (file reservations are DB-backed and independent of mail), but the swarm becomes "blind" via the mail channel — the audit trail it relies on for blocker visibility, handoff signalling, and the Phase B precommit hook is hollow.

The bug is **deterministic**, **persistent across an agent's full lifecycle** (does not self-heal between beads), and **silent** — wrappers print `br-claim: ✓ sent Start mail to ['<self>']` as if everything succeeded.

---

## 2. Symptom — observed evidence from the live L5 swarm

Project: `/Users/alinaaitova/Desktop/Testproject1`
Spawn window: 2026-05-19 20:01:10 – 20:01:23 UTC (13-second spread across 4 agents)

Four agents registered (`list_agents` excerpt):

| Name | Inception | contact_policy |
|---|---|---|
| OliveCanyon | 20:01:10.641 | open |
| NavyStone | 20:01:10.778 | open |
| AmberPike | 20:01:15.542 | open |
| BeigeTrout | 20:01:23.477 | open |

`search_messages` excerpt — every wrapper-generated mail across the next 40 minutes (16 mails) was self-addressed:

```
id=5   20:02:40  AmberPike   → AmberPike       [hn05.1.1] Start
id=6   20:03:05  BeigeTrout  → BeigeTrout      [hn05.1.2] Start
id=7   20:03:49  OliveCanyon → OliveCanyon     [hn05.1.4] Start
id=8   20:04:18  NavyStone   → NavyStone       [hn05.1.6] Start
id=10  20:16:10  OliveCanyon → OliveCanyon     [hn05.1.4] Complete
id=14  20:18:53  OliveCanyon → OliveCanyon     [hn05.1.5] Start          ← second claim, still self
id=15  20:19:54  AmberPike   → AmberPike       [hn05.1.1] Complete
id=16  20:20:34  BeigeTrout  → BeigeTrout      [hn05.1.2] Complete
id=17  20:20:39  NavyStone   → NavyStone       [hn05.1.6] Complete
id=20  20:22:59  BeigeTrout  → BeigeTrout      [hn05.1.3] Start          ← second claim, still self
id=25  20:29:50  OliveCanyon → OliveCanyon     [hn05.1.5] Complete
id=30  20:36:14  BeigeTrout  → BeigeTrout      [hn05.1.3] Complete
id=31  20:36:38  BeigeTrout  → BeigeTrout      [hn05.1.7] Start          ← third claim, still self
id=32  20:40:57  BeigeTrout  → BeigeTrout      [hn05.1.7] Complete
id=33  20:41:25  BeigeTrout  → BeigeTrout      [hn05.1.8] Start          ← fourth claim, still self
```

Counter-evidence — when an agent calls `send_message` directly (NOT through a wrapper), peer routing works correctly. Three messages prove this:

```
id=13  20:18:32  BeigeTrout  → ["AmberPike", "NavyStone"]   [hn05.1.2] Landing blocked by dirty tracked Phase 1 files
id=19  20:21:00  NavyStone   → ["BeigeTrout"]               Re: [hn05.1.2] Landing blocked …
id=26  20:30:47  AmberPike   → ["BeigeTrout"]               [l5] Blocked check: only 1.3 remains
```

So the agents have peer names available when they need them (likely via `list_agents`). The gap is exclusively in the wrappers' contact-discovery path.

**Did the bug self-heal?** No. After 16 wrapper invocations across all four agents, zero peer-addressed wrapper mails occurred. There is no automatic recovery.

---

## 3. Reproduction

Minimal reproduction in 5 steps:

1. Start AgentMail (`http://127.0.0.1:8765` per AGENTS.md).
2. Open ≥2 concurrent Claude/Codex sessions in the same project directory.
3. In each session, run `macro_start_session(project_key=<abs path>, program=<same program>, model=<...>, task_description=<...>)` (or the kickoff §5 granular equivalent: `ensure_project` + `register_agent` + `set_contact_policy(policy="open")`).
4. Each session claims a bead via `bin/br-claim <bead-id>`.
5. Inspect: `search_messages(project_key, query="Start", limit=20, ranking="recency")`.

**Observed:** every Start mail has `"to": ["<self>"]`. **Expected:** `"to": ["<peer-a>", "<peer-b>", …]` (every other registered live peer).

Reproduction does not require Phase 1 work specifically — any bead-claim flow exhibits it. The trigger is "no explicit contact handshake ever happened between these agents."

---

## 4. Impact

### Phase 1 (observed, 2026-05-19, ~40 min wall-clock)

- ✅ Zero collisions. File reservations (DB-backed at AgentMail layer) caught all overlapping claim attempts.
- ✅ Throughput excellent. 11 beads closed in 40 min.
- ⚠️ Three real coordination events happened *outside* the wrapper channel (agents sent direct `send_message` calls with peer routing manually constructed). Peers DID find each other when they wanted to.
- ❌ The 16 wrapper mails carried no peer-visible signal. The mail-archive audit trail of "who claimed what when" exists, but no peer ever received it as inbox content.

### Phase 2 onward (predicted)

Risk compounds with parallelism and file-surface overlap:

- **Phase 2/3/5 (parallel after gate 1.8):** ~6 active claims at once. Disjoint file surfaces (`packages/intelligence/src/fx/`, `.../logistics/`, `skills/`). Self-addressing degrades visibility but file reservations still bound risk.
- **Phase 4 (13 beads, 3–4 parallel sub-lanes):** heavy `packages/intelligence/src/customs/` overlap. Agents need to see `[<bead-id>] Start` mails to know which sibling bead is closing what file next. Self-addressing means an agent only learns of contention via `FILE_RESERVATION_CONFLICT`, AFTER they've already invested time picking a bead.
- **Phase 6 (`main.rs` cutover sequence):** strictly serial. The 3-bead sequence (`6.8 → 6.9 → 6.10`) requires explicit Complete-mail visibility so the next agent knows the prior bead's commit hash before starting. Self-addressing breaks the trust chain to the precommit hook.

### Phase B precommit hook (AGENTS.md:28)

> "the Phase B extension to that hook (when active) will additionally block any commit with a `<bead-id>:` message prefix that has no corresponding `[<bead-id>] Start` mail in the archive."

The hook checks for *any* Start mail in the thread, not for peer routing. So self-addressed Start mail still satisfies the hook. **The bug does not break the hook directly**, but it removes the second-order signal (peer inbox notifications) that the hook is supposed to layer on top of.

---

## 5. Root Cause

### 5.1 The wrapper fallback

`bin/br-claim` line 397-419 (verbatim):

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
    recipients = peers if peers else [agent]   # ← the silent self-fallback
```

`bin/br-complete` line 375-395 has the identical logic.

The fallback was placed defensively (avoid empty `to=[]` which AgentMail would reject), but it masks the real failure: when `list_contacts` returns `[]`, the system is in a broken state — it does not know who its peers are. Self-addressing is not a sensible default; **erroring or rerouting through `list_agents` is.**

### 5.2 Why `list_contacts` returns empty — contact_policy semantics

AgentMail distinguishes between:

| Concept | What it means | How it's established |
|---|---|---|
| **Registration** (`register_agent`) | "I exist in this project." | `register_agent(project_key, program, model, task_description)` |
| **Contact policy** (`set_contact_policy`) | "Whether I accept incoming contact requests at all." Values: `open` (accept anyone), `auto` (default — auto-accept from same project), `restricted` (manual approval). | `set_contact_policy(...)` |
| **Contact link** | An explicit bilateral relationship: agent A and agent B have exchanged a handshake; `list_contacts(A)` includes B and vice versa. | `request_contact` + `respond_contact`, OR `macro_contact_handshake` |

**`contact_policy: "open"` enables handshakes; it does NOT auto-create links.** Without a handshake, `list_contacts(<self>)` returns `[]` — even when other agents are visibly registered in `list_agents`.

The kickoff §5 instructs agents to call `set_contact_policy(policy="open")` after registration, but **never** to handshake. The wrappers then fail open-loop, because no contacts ever exist for them to find.

This is the entire bug.

### 5.3 Race window — why all 4 agents are simultaneously in the broken state

Even if the kickoff did handshake, the protocol would need careful sequencing in a swarm spawn window. AgentMail's `list_agents(project_key)` returns whoever is currently registered. If 4 agents register inside a 13-second window:

- Agent 1's `list_agents` at t=0 returns `[A1]` only.
- Agent 2's at t=3 returns `[A1, A2]`.
- Agent 3's at t=6 returns `[A1, A2, A3]`.
- Agent 4's at t=12 returns `[A1, A2, A3, A4]`.

If each agent only handshakes the peers it sees at registration time, A1 never handshakes anyone (it was alone). A2 handshakes A1. A3 handshakes A1, A2. A4 handshakes everyone. Even handshake-on-registration produces an *asymmetric* contact graph unless every agent **re-checks `list_agents` on every claim** (or on an explicit refresh).

The kickoff §11 notes "Refresh `<peers>` between beads (other agents may register or leave)" — but the *wrappers* do not honor that guidance. They check once per call (good) but check `list_contacts` (wrong source) instead of `list_agents` (correct source).

---

## 6. Why prior fixes didn't catch this

`git log -- bin/br-claim bin/br-complete` shows three relevant commits:

| Commit | Subject | Relevance |
|---|---|---|
| `53a6adf` | `chore: add bin/br-claim and bin/br-complete wrappers for atomic bead claim/complete` | Initial creation. The self-addressing fallback at the end of the peer-discovery block (`recipients = peers if peers else [agent]`) shipped on day 1. |
| `adec8e4` | `fix(wrappers): include 'result' in search_messages response-key chain` | Fixed a separate bug in the duplicate-claim guard's call to `search_messages`. Did not touch `list_contacts`. |
| `e64d8de` | `chore: agent-mail SQLite drift defense — wrapper preflight + handoff docs` | Added a preflight that detects AgentMail DB corruption (see `docs/handoff-2026-05-05-mcp-agent-mail-db-recovery.md`). Did not touch peer discovery. |

No prior fix addressed contact discovery. The user's recollection that "we had a skill that specifically fixed this" is most likely a memory of `adec8e4` (which *was* a wrapper fix and *did* touch contact-related response parsing) being conflated with this bug — but `adec8e4` is for a different code path (the duplicate-claim guard reads `search_messages` for prior Start mail; it does not read `list_contacts`).

Inspection of `.claude/skills/` reveals no agent-mail-specific skill exists. The "skill" mechanism for this project is the wrapper scripts themselves plus the embedded protocols in `AGENTS.md` and the kickoff prompt. **The wrappers ARE the skill.** Any fix lives in those two files.

---

## 7. Proposed fix

Apply **all three** parts. Each is independently valuable; together they form the durable solution.

### 7.A — Wrappers (highest priority, ships with the codebase)

Replace the `list_contacts` → `peers if peers else [agent]` block in both `bin/br-claim:397-419` and `bin/br-complete:375-395` with a layered discovery strategy:

```python
def discover_peers(url, token, project_key, agent, *, max_idle_seconds=3600):
    """
    Resolve recipients for Start/Complete mail.

    Priority order:
      1. list_contacts (explicit handshake links)            — preferred
      2. list_agents   (everyone in the project)              — fallback
            • filter out self
            • filter to agents with last_active_ts within max_idle_seconds
            • optionally filter to same program (caller passes program)
      3. error out — never self-address.

    Never returns [] or [self]; if no peers exist, raises so the caller
    can surface the diagnostic to the operator.
    """
    # Step 1 — try contacts
    peers = _normalize_agent_list(
        mcp_call(url, token, "list_contacts",
                 {"project_key": project_key, "agent_name": agent}),
        exclude=agent,
    )
    if peers:
        return peers, "contacts"

    # Step 2 — fall back to list_agents, filtered by liveness
    agents_res = mcp_call(url, token, "list_agents", {"project_key": project_key})
    live = []
    cutoff = _now_minus(max_idle_seconds)
    for a in agents_res or []:
        if not isinstance(a, dict): continue
        name = a.get("name") or a.get("agent_name")
        if not name or name == agent: continue
        last = a.get("last_active_ts") or a.get("inception_ts")
        if last and _parse_iso(last) >= cutoff:
            live.append(name)
    if live:
        return live, "list_agents"

    # Step 3 — no peers anywhere. Do NOT silently self-address.
    raise NoPeersError(
        "No peers discoverable via list_contacts OR list_agents. "
        "Either run macro_contact_handshake with a peer, or confirm at least "
        "one other live agent is registered in this project. "
        f"agent={agent!r} project_key={project_key!r}"
    )
```

Critical invariants:

1. **No silent self-fallback.** If peers cannot be found, the wrapper raises and refuses to send mail. The operator sees a clear diagnostic instead of a phantom Start mail.
2. **`list_agents` filter must check liveness.** Old/abandoned agents in the project archive (this project has 19 historical agents from prior work) must not become spurious recipients. Default 1-hour idle cutoff is a reasonable starting point; expose `--peer-max-idle-seconds` if needed.
3. **Same-program filter is optional** but recommended when the program field is set consistently — it prevents L5 executors from spamming an unrelated doc-audit agent that happens to be live in the same project.
4. **Keep the wrapper idempotent.** The duplicate-claim guard already prevents re-sending a Start mail. The peer-discovery layer must not change that.

### 7.B — Kickoff §5 (AgentMail Setup) — bootstrap handshake step

After `register_agent` + `set_contact_policy`, add a third explicit step:

```
3. Bootstrap peer contacts (REQUIRED):

   peers = list_agents(project_key="...")
   # Filter: exclude self, exclude inactive (last_active_ts older than 1h)
   for peer in peers:
       macro_contact_handshake(
           project_key="...",
           agent_name=<your-name>,
           peer_name=peer,
       )
```

Wording for the kickoff:

> **Step 3 — Handshake every live peer.** `contact_policy: "open"` lets others *contact* you, but `list_contacts(...)` returns explicit handshake links — without this step, every wrapper-generated Start/Complete mail will silently address you instead of your peers. Run `list_agents(project_key=...)`, filter to peers active within the last hour, and call `macro_contact_handshake(project_key, agent_name=<you>, peer_name=<peer>)` once per peer. Re-run on every new session and after any "swarm grows" event.

Add to the kickoff §11 (AgentMail Quick Reference):

> **Refresh handshakes on swarm growth.** If `list_agents` shows a new peer not yet in your `list_contacts`, handshake them before your next `bin/br-claim`. The wrappers depend on contacts being current.

### 7.C — Push prompt §1 — re-orient handshake check

Add a third blocking step after AGENTS.md re-read and AgentMail registration confirmation:

> 3. **Verify handshake graph.** Call `list_contacts(project_key, agent_name=<you>)`. If empty AND `list_agents(project_key)` shows ≥1 other live peer, you are in the self-addressing failure state. Run `macro_contact_handshake(...)` for each peer before continuing.

---

## 8. Test plan

### 8.1 Wrapper unit tests (new — currently no wrapper tests exist)

Create `bin/tests/test_peer_discovery.py` (or extend an existing test surface) exercising `discover_peers(...)`:

| Test | Setup | Expected |
|---|---|---|
| T1 | `list_contacts` returns 2 peers | returns those 2, source = "contacts" |
| T2 | `list_contacts` empty, `list_agents` returns 2 live peers | returns those 2, source = "list_agents" |
| T3 | `list_contacts` empty, `list_agents` returns only stale peers (last_active 2h ago) | raises `NoPeersError` |
| T4 | `list_contacts` empty, `list_agents` returns only self | raises `NoPeersError` |
| T5 | Both calls return same peer plus self | self filtered out; peer returned once |
| T6 | `list_contacts` returns peer with `agent_name` key; `list_agents` returns same peer with `name` key | dedup by name; returned once |

### 8.2 Integration test — swarm bootstrap

`tests/agent-mail-coordination/test_bootstrap_handshake.sh`:

1. Start AgentMail server (clean DB).
2. Spawn 3 mock agents in parallel; each runs registration + bootstrap-handshake.
3. Assert: every agent's `list_contacts` shows the other two.
4. Each agent runs `bin/br-claim <fake-bead-id>` (with `--paths /tmp/test-paths`).
5. Assert: every Start mail has `to` containing both other peers, not self.

### 8.3 Regression test — no peers case

`tests/agent-mail-coordination/test_no_peers_errors.sh`:

1. Single agent registers, no others.
2. Run `bin/br-claim <id>`.
3. Assert: command exits non-zero with diagnostic mentioning `NoPeersError` and "swarm of one".
4. Assert: no Start mail was archived.

(In single-agent operation, the operator must explicitly pass `--solo` or set `AGENT_MAIL_ALLOW_SOLO=1`.)

### 8.4 Live swarm validation

After fix lands, re-spawn the L5 swarm setup (or any 2-agent swarm):

1. Both agents run kickoff §5 including the new Step 3 handshake.
2. Both run `bin/br-claim <id>`.
3. Assert via `search_messages(query="Start", ranking="recency")`: `to` contains the peer name, not self.

---

## 9. Acceptance criteria

- [ ] `bin/br-claim` and `bin/br-complete` no longer silent-fallback to `[<self>]` — they either find live peers via the layered strategy or raise.
- [ ] Kickoff §5 documents the explicit handshake step.
- [ ] Push prompt §1 adds the handshake verification step.
- [ ] Unit tests T1–T6 pass.
- [ ] Integration tests 8.2 and 8.3 pass.
- [ ] Live re-spawn (8.4) shows peer-addressed Start/Complete mail.
- [ ] AGENTS.md Rule 2 block is updated to reference the bootstrap handshake as a prerequisite for wrapper behavior.

---

## 10. Cross-project portability — why this matters beyond Testproject1

The wrappers (`bin/br-claim`, `bin/br-complete`, `bin/br-sync`) are the project's portable Rule 2 mechanism. They:

- ship with the repo (copied to each new machine via `git clone`),
- run identically across macOS / Linux,
- depend only on `python3`, `curl`, and a running AgentMail server,
- encode the project's bead-and-mail coordination protocol as a single artifact.

**This is the right level of abstraction.** When the user moves to a new machine or starts a new project that adopts this coordination pattern, the wrappers travel intact. The bug as it stands today would travel with them — every future swarm would silently self-address until someone notices and re-handshakes.

A correct fix must:

1. **Be encoded inside the wrappers** (not in per-project skill prompts) so it travels with the code.
2. **Be encoded in the kickoff prompt template** so future kickoff prompts (L6, future layers, future projects) inherit the handshake step.
3. **Be encoded in `AGENTS.md`** as a constitutional addendum to Rule 2, so future agents reading AGENTS.md learn the handshake-before-claim rule without needing to discover it via a postmortem.

Reusable skill artifacts to consider extracting (one-time uplift, then ships everywhere):

- `bin/br-bootstrap-handshake` — a single-shot script that an agent runs at session start, after registration. Wraps `list_agents` + dedup + `macro_contact_handshake` calls. Idempotent. Safe to re-run.
- `bin/br-claim --bootstrap` — flag that triggers handshake-then-claim atomically. Better DX than a separate command.
- Project-template `.claude/agents/agent-mail-bootstrap.md` — a project-template skill file the kickoff prompt can reference (`/agent-mail-bootstrap` if Claude Code skills are wired) that encapsulates the handshake step. Future projects copy the template.

These extractions are nice-to-have, not blocking. The minimum fix is 7.A + 7.B + 7.C.

---

## 11. References

### Code
- `bin/br-claim:397-419` — self-addressing fallback in claim path
- `bin/br-complete:375-395` — self-addressing fallback in complete path
- `bin/br-sync` — unrelated; included for context, no peer-discovery logic

### Project docs
- `AGENTS.md:21-33` — Rule 2 (constitutional wrapper discipline)
- `AGENTS.md:209-258` — MCP Agent Mail section (registration, file reservations, broadcast prohibition)
- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-kickoff.md` §5 (AgentMail Setup — currently missing the handshake step)
- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-kickoff.md` §11 (AgentMail Quick Reference)
- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-push.md` §1 (re-orient block)
- `docs/handoff-2026-05-05-mcp-agent-mail-db-recovery.md` — predecessor incident (DB corruption); demonstrates the wrappers' preflight pattern this fix should match

### AgentMail MCP tools (all accessible via the registered AgentMail MCP server)
- `register_agent` / `ensure_project` / `macro_start_session` — registration
- `set_contact_policy` — `open` / `auto` / `restricted`
- `list_contacts` — explicit handshake links (current wrapper input — wrong source for first-claim)
- `list_agents` — all registered agents in project (correct fallback source)
- `request_contact` + `respond_contact` — manual handshake
- `macro_contact_handshake` — single-call handshake (preferred for bootstrap)
- `send_message` — peer-addressed mail (works fine when `to` is correctly populated)
- `file_reservation_paths` — DB-level collision detection (still works; orthogonal to this bug)

### Live evidence snapshot (this incident)
- AgentMail health: green at 2026-05-19 20:42 UTC
- Affected agents: AmberPike, BeigeTrout, NavyStone, OliveCanyon (all program=`l5-expanded-commercial-intelligence-executor`)
- 16 wrapper mails self-addressed; 3 direct messages correctly peer-addressed
- Snapshot data persists in AgentMail archive at `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/`

---

## 12. Handoff notes for the implementing agent

- **Do not change wrapper exit semantics.** `bin/br-claim` must still set `br status=in_progress` exactly once after mail+reservation succeed; the order in §6 of the kickoff is constitutional.
- **Do not introduce new `mcp_call` retry behavior** — the existing helper is intentionally simple. If peers are needed and unavailable, fail loudly. Idempotency is the safety net for partial retries.
- **The fix is portable**, so write the wrapper test fixtures to use AgentMail's actual MCP surface (a test-only project_key + ephemeral DB), not in-process mocks. The wrappers shell out to `mcp_call`; mocking them would defeat the purpose.
- **Update `MEMORY.md` reference index entry** for this manifest after the fix lands, so future sessions know the workaround (`macro_contact_handshake` once at session start) until the wrapper change is everywhere.
- **Do not delete the existing `recipients = peers if peers else [agent]` line until the new `discover_peers` helper is exercised by tests.** Replace, don't ablate-and-rebuild.
- **Backwards-compatibility note:** future agents working on machines that haven't synced this fix will continue self-addressing. The fix lands when it's pulled. No data-format migration is needed; existing self-addressed mail in the archive is harmless and remains valid as a (degraded) Start/Complete record.
