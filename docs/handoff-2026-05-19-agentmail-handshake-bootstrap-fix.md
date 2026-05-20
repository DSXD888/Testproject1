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

> **DO NOT START THIS WORK WHILE L5 (Layer 5) BEADS ARE STILL OPEN, INCLUDING FIX BEADS.** See §0. The operator's chosen execution path is "complete every L5 bead — main AND fix — first, then land this handoff in the quiet window between L5 closing and any L6 swarm starting."

---

## 0a. Five-line orientation for a cold-arriving agent

If you've never worked in this project before, read this first. Then §0 pre-flight, then §1.

- **The project**: `/Users/alinaaitova/Desktop/Testproject1` — UK public procurement intelligence platform. Multi-stack (Rust daemon + Bun/TypeScript packages + Tauri desktop + Next.js web). Multi-agent — multiple Claude/Codex sessions work in parallel via `bin/br-claim` / `bin/br-complete` + MCP Agent Mail. See `AGENTS.md` (constitutional) and `CLAUDE.md` (points to AGENTS.md).
- **What "L5" means here**: Layer 5 — Expanded Commercial Intelligence. The most recent multi-agent swarm of work in this project, bead ID prefix `testproject1-hn05.*`. L5 introduces decision-grade customs classification + dual FX + real freight + marketplace skill maturations. It is being completed by a swarm of 4 concurrent agents (Claude + Codex CLI sessions).
- **What "fix beads" means**: post-implementation fix beads spawned from L5 work — bugs found during L5, polish, follow-ups. Still labeled `layer-5`. Treated as in-scope for L5 until closed.
- **What you're fixing**: a silent self-addressing bug in `bin/br-claim` and `bin/br-complete` that causes `[<bead-id>] Start` and `Complete` mail to go to `[<self>]` instead of peers whenever `list_contacts(...)` returns `[]` (which is the default on first claim, because `set_contact_policy("open")` does NOT auto-handshake). Operator caught this on 2026-05-19 during L5 Phase 1; applied a manual workaround to the live L5 swarm (see Appendix A for the verbatim message); deferred the durable fix to this handoff.
- **What "universal fix" means**: the bug ships with the wrappers, and the wrappers ship to every new project the operator bootstraps. The fix has to land in this project's wrappers, in `AGENTS.md`, in the kickoff/push templates, in BOTH the Claude (`~/.claude/skills/`) and Codex (`~/.codex/skills/`) skill registries, AND in BOTH copies of the `install-bead-wrappers` skill assets (which is the canonical source from which new projects copy the wrappers). Anything less is partial coverage.

If anything in this handoff conflicts with `AGENTS.md`, AGENTS.md wins.

---

## 0. Pre-flight — verify L5 (including fix beads) is done AND L6 has not started

**Who runs this**: a fresh Claude Code or Codex CLI session in `/Users/alinaaitova/Desktop/Testproject1`, spawned by the operator after L5 closes. The session needs MCP access to the local `mcp-agent-mail` server and the `br` CLI on PATH. If you are a fully autonomous handoff worker, register with AgentMail first (§5.4 of `AGENTS.md`) so your `--solo` smoke tests can prove themselves against a real archive.

**Why timing matters**. This handoff modifies `bin/br-claim`, `bin/br-complete`, and `AGENTS.md`. All three are load-bearing for any in-flight swarm. Touching them mid-swarm risks:

- A half-written wrapper file getting executed by an active claim/complete invocation.
- A merge conflict on `AGENTS.md` or `.beads/` against another agent's commit.
- Test agents created during §13.1 polluting the live swarm's `list_agents` result.

The operator already applied a manual workaround for the live L5 swarm (`macro_contact_handshake` between each pair of L5 agents). The wrapper change is therefore safe to defer; it benefits future swarms, not the current one. **Land this before any L6 swarm spawns.**

**Run all of these checks before any edit. ALL must be clean.**

```bash
# 1. No L5 task beads remain open or in-progress.
#    Allow ONLY epic-level IDs (`testproject1-hn05` root + phase sub-epics
#    `testproject1-hn05.{1..6}`) if the L5 closure plan leaves them open;
#    any deeper bead (e.g. `testproject1-hn05.4.7`) must be closed.
br list --label layer-5 --status=in_progress --json | jq -r '.[] | .id'
br list --label layer-5 --status=open          --json | jq -r '.[] | .id'

# 2. No L6 swarm has begun yet — this fix should ship into the gap.
br list --label layer-6 --status=in_progress --json | jq -r '.[] | .id'
# Expected: empty. If L6 in_progress beads exist AND any holder is still live
# (see check #4), pause: rolling the wrappers mid-claim is unsafe.

# 3. Working tree is clean on main.
git status                       # must show "nothing to commit"
git pull --rebase                # in sync with remote
git rev-parse --abbrev-ref HEAD  # must be `main`

# 4. No live swarm sessions. This check requires you to be AgentMail-registered
#    (run macro_start_session FIRST if you haven't — it's a read-only registration
#    that won't pollute the swarm). Then:
#       list_agents(project_key="/Users/alinaaitova/Desktop/Testproject1")
#    Reject if ANY result has program ∈ {l5-expanded-commercial-intelligence-executor,
#    l6-*-executor} with last_active_ts inside the last 30 minutes.

# 5. AgentMail server is green (the wrapper smoke test in §13.2 needs it):
curl -sfS http://127.0.0.1:8765/health 2>&1 | head -5
# Expect HTTP 200.
```

If any check fails: **stop**, reply in any active handoff thread with `[handoff-block] AgentMail handshake fix paused: <reason>`, and wait. Do not proceed and do not partially apply — every change here is interdependent.

**Specifically — if check #1 fails because fix beads are open**: the operator's chosen path is "let the swarm finish all open `layer-5` work first, using the L5 manual workaround (see Appendix A) the same way L5 main beads did, THEN run this handoff." Do NOT bypass check #1 to land the wrappers mid-fix-bead-swarm — that risks merge conflicts on `bin/br-claim` / `bin/br-complete` / `AGENTS.md` and could leave an active session running a half-loaded wrapper. Instead, reply with `[handoff-block] Waiting on layer-5 fix beads to close before landing handoff` and wait.

Equivalently, if check #2 fails because L6 has already started, the gap has closed and a different decision is needed — escalate to the operator before proceeding. Don't try to thread the needle silently.

---

## 1. TL;DR

The `bin/br-claim` and `bin/br-complete` wrappers silently address every Start/Complete mail to `[<self>]` whenever `list_contacts(...)` returns `[]`. That happens by default on first claim, because `contact_policy: "open"` enables incoming requests but does NOT auto-create handshake links; without an explicit `macro_contact_handshake`, `list_contacts` is empty for every agent. Result: every swarm-generated wrapper mail goes nowhere visible, peers learn of each other only through file-reservation conflicts, and the Phase B precommit hook's "Start mail exists" check passes on a hollow signal.

The fix has four durable parts, applied in order:

1. **Wrapper-level** — replace the `recipients = peers if peers else [agent]` silent self-fallback with a layered discovery strategy (`list_contacts` → `list_agents` liveness filter → loud error). Add a `--solo` opt-in for legitimate single-agent operation.
2. **Documentation-level** — bake the bootstrap handshake into AGENTS.md Rule 2 and into the layermaxxer kickoff/push templates.
3. **Claude skill-level (cross-project lever)** — update `~/.claude/skills/beads-to-execution/TEMPLATES.md` so every future Claude-Code-generated kickoff prompt inherits the handshake step automatically.
4. **Codex skill-level (cross-project lever, MIRROR)** — same edit applied to `~/.codex/skills/beads-to-execution/TEMPLATES.md` so every future Codex-CLI-generated kickoff prompt inherits the same handshake step. Without this mirror, universal coverage breaks the moment the operator generates a kickoff from a Codex session.

The bug is reproducible in 5 steps (predecessor manifest §3). The fix is fully specified in this document — every artifact, every diff, every verification gate. A cold-arriving agent can execute end-to-end without external references beyond AGENTS.md and the predecessor error manifest.

---

## 1A. Why this fix is universal (Claude AND Codex)

The operator's concern: "I want a universal fix — does the handoff cover both Claude Code and Codex CLI?" Answer: yes, and here is the full coverage map. Every coordination surface a Claude or Codex session can touch is addressed by exactly one of the artifacts below:

| Surface | Used by | Covered by which §? | Why it works for both |
|---|---|---|---|
| `bin/br-claim`, `bin/br-complete` (Python wrappers) | Both — they are shelled-out scripts; the CLI identity is irrelevant. The bearer-token resolver at `bin/br-claim:50-87` already handles both the Claude install layout (`~/.claude.json` `mcpServers` entry) AND the Codex install layout (`~/.config/mcp-agent-mail/config.env`). | §6, §7 | Same Python file. Same `discover_peers(...)` helper. Same CLI flags. |
| `AGENTS.md` (project constitution) | Both — Claude Code reads it via `CLAUDE.md:3` (`See AGENTS.md — that is the single source of truth`); Codex CLI's convention is to read `AGENTS.md` directly. | §9 | Single source of truth at the project level. |
| Generated kickoff/push prompts (`docs/pipeline/.../l*-kickoff.md`, `l*-push.md`) | Both — markdown is agent-agnostic. The `program` field in `register_agent(...)` is the only CLI-specific token, and `TEMPLATES.md:160` explicitly comments `# or "codex-cli", "gemini-cli", etc.`. | §10, §11 | Kickoff prompts are deliberately CLI-neutral. |
| Layermaxxer execution-kickoff/push templates (`docs/layermaxxer/active/...`) | Both — these templates are the source for layer-strategy execution prompts; the resulting prompts are consumed by whichever CLI the operator spawns. | §10, §11 | Same as above — markdown, CLI-neutral. |
| `~/.claude/skills/beads-to-execution/TEMPLATES.md` | Claude Code only (the `/beads-to-execution` skill in Claude). | §12 | One half of the cross-CLI mirror. |
| **`~/.codex/skills/beads-to-execution/TEMPLATES.md`** (parallel skill registry) | **Codex CLI only.** Confirmed present on this machine; mirror file structure to the Claude tree. | **§12-bis (NEW)** | Other half of the cross-CLI mirror. Without this, future kickoffs the operator generates from a Codex session would NOT inherit the handshake step. |
| Sibling skills in BOTH trees that emit AgentMail-Setup blocks (12+ files per tree — `bead-completion-audit`, `bead-plan-audit`, `beads-from-sources`, `featuremaxxer`, `layermaxxer`, `pipemaxxer`, `planmaxer`, `install-bead-wrappers`) | Both — each CLI loads from its own tree. | §12.4, §12-bis.3 | Greps cover sibling skill files (TEMPLATES.md / SKILL.md / CHECKLIST.md / references/TEMPLATES.md) in BOTH trees. |
| Live `mcp-agent-mail` server (the broker) | Both — Claude and Codex sessions share one local server at `127.0.0.1:8765`; agents from either CLI co-exist in the same project archive. | (not modified — orthogonal) | The bug is purely client-side; the server's behaviour is correct. |
| Phase B precommit hook (`.git/hooks/hooks.d/pre-commit/50-agent-mail.py`) | Both — runs from `git commit` regardless of CLI. | (not modified — still correct) | The hook only checks "Start mail exists in the thread"; it does not care about recipient list. This fix strengthens the upstream signal without changing the hook's contract. |

The handoff lands fixes at **six** classes of artifact (wrappers + AGENTS.md + layermaxxer templates + Claude skill templates + Codex skill templates + **install-bead-wrappers asset library in BOTH trees**). The install-bead-wrappers piece (§12-ter) is critical: that skill is THE mechanism by which `bin/br-claim`, `bin/br-complete`, and AGENTS.md Rule 2 propagate to every NEW project. Its assets carry the same bug at the same line numbers; failing to update them means every future project bootstrapped via `install-bead-wrappers` re-introduces the bug from day 1.

**Operator sanity check after implementation**: spawn one Claude Code session and one Codex CLI session in a fresh test project. Have each call `macro_start_session` and run the kickoff's bootstrap handshake step (which will now exist in both Claude- and Codex-generated kickoffs). Run `bin/br-claim` from each. Inspect `search_messages(... query="Start" ...)` — every `to` field must contain the other agent's name. If yes: universal. If only one side works: the missing-tree edit was skipped; finish §12 / §12-bis / §12-ter.

---

## 1B. Universality limits and propagation gaps (honest accounting)

A fix that's "universal in principle" still has propagation gaps in practice. Surface them up-front so the operator sees the full picture:

1. **Existing projects with pre-fix wrappers already cloned.** The wrappers in `bin/br-claim` and `bin/br-complete` are checked into each project's git. Older clones on other machines (or older worktrees on this machine) will keep the buggy wrappers until either (a) a `git pull` brings in the fix commit, OR (b) someone runs the updated `install-bead-wrappers` skill in that project. The handoff's §12-ter step makes (b) work; (a) requires nothing extra. There is no auto-push mechanism — the operator must initiate the refresh per project.

2. **Active sessions don't get the fix until they re-load.** A Claude or Codex session that's already running when the fix lands will continue invoking the old wrappers (the wrappers are re-loaded per shell invocation, so each new `bin/br-claim` actually picks up the fixed version — but in-memory state about handshakes is per-session and won't auto-refresh). Operator should restart any active session after the fix lands.

3. **The `mcp-agent-mail` server itself isn't changed.** This handoff is purely client-side. If a future server release changes the shape of `list_agents` / `list_contacts` responses, the `discover_peers` helper might need a re-look. The `_normalize_peer_list` helper is defensive about multiple response shapes (list / `{"agents": [...]}` / `{"contacts": [...]}` / `{"result": [...]}`), so most server changes will be transparent.

4. **Pre-existing self-addressed mail in the archive remains.** The 16 L5 self-addressed mails from 2026-05-19 stay in the archive as a (degraded) Start/Complete record. They are harmless — the Phase B precommit hook checks for "any Start mail in the thread", not for peer routing. Do not attempt to "fix" historical mail; it's evidence.

5. **The `set_contact_policy("open")` semantic is unchanged.** The fix does not lobby for AgentMail to auto-handshake when policy="open" — that would be a server-side feature request. If the operator wants to file that upstream (mcp-agent-mail issue tracker), the predecessor manifest's §5.2 has the rationale. Until then, explicit handshake at session start is the load-bearing step.

6. **Non-Claude, non-Codex CLIs (Gemini CLI, etc.) are not covered by skill-template edits.** They are covered by AGENTS.md (project-level, CLI-agnostic) and by the wrappers themselves (Python, CLI-agnostic). But if Gemini-CLI ever ships a `~/.gemini/skills/beads-to-execution/TEMPLATES.md`-equivalent, that would need the same §12-style edit. The current handoff focuses on Claude + Codex because those are the two CLIs the operator actually uses today.

7. **Skill assets vs live wrappers can diverge between updates.** `install-bead-wrappers/METHODOLOGY.md` documents the manual sync command (`cp /Users/alinaaitova/Desktop/Testproject1/bin/br-claim ~/.claude/skills/install-bead-wrappers/assets/br-claim`). Until someone files a bead to automate this (auto-publish on commit to `bin/br-*`), divergence is possible. §12-ter pins the canonical sync command at fix time.

Bottom line: post-implementation, every NEW project (via `install-bead-wrappers`) and every NEW kickoff (via `beads-to-execution`) is universally fixed. Existing projects on existing machines need a manual refresh — operator-initiated, per project, low effort.

---

## 2. Why this exists (link to evidence)

Open `docs/handoff-2026-05-19-agentmail-wrapper-self-addressing.md` for the full error manifest — 4 affected agents, 16 self-addressed wrapper mails over 40 minutes in the L5 Phase 1 swarm of 2026-05-19, plus a counter-proof that direct `send_message` peer routing works fine. The manifest is dense; you do not need to re-read it to execute this handoff, but you must link to BOTH this handoff and the error manifest from your commit message:

```bash
git commit -m "$(cat <<'EOF'
agent-mail wrappers: bootstrap handshake + layered peer discovery

Replaces the silent `recipients = peers if peers else [agent]` self-fallback
in bin/br-claim and bin/br-complete with a layered discovery strategy
(list_contacts → list_agents liveness/program filter → loud NoPeersError),
plus a --solo opt-in for legitimate single-agent operation. Adds the
bootstrap-handshake step to AGENTS.md Rule 2 and the layermaxxer templates.

Refs:
  - docs/handoff-2026-05-19-agentmail-handshake-bootstrap-fix.md  (this handoff — plan)
  - docs/handoff-2026-05-19-agentmail-wrapper-self-addressing.md  (error manifest — why)

Operator-side mirror edits (NOT in this commit, applied in same session):
  ~/.claude/skills/beads-to-execution/TEMPLATES.md           — §12
  ~/.codex/skills/beads-to-execution/TEMPLATES.md            — §12-bis
  ~/.claude/skills/install-bead-wrappers/assets/{br-claim,br-complete,agents-md-rule-2.md}
  ~/.codex/skills/install-bead-wrappers/assets/{br-claim,br-complete,agents-md-rule-2.md}
EOF
)"
```

The manifest's §5 (Root Cause), §7 (Proposed fix), §10 (Cross-project portability), and §12 (Handoff notes for the implementing agent) are the design source for everything below. If something in this handoff seems contradictory to the manifest, the manifest wins — call out the conflict in your commit message and proceed conservatively.

---

## 3. The fix in one paragraph

A wrapper that has no peers is in a broken state, not a happy edge case. It must NEVER silently self-address. The corrected behaviour is: `list_contacts` first; if empty, `list_agents` filtered by liveness (default cutoff 1 h) and same-program; if still empty, raise `NoPeersError` with a clear diagnostic naming the recovery action (`macro_contact_handshake` or `--solo`). A new `--solo` flag (and `AGENT_MAIL_ALLOW_SOLO=1` env var) is the explicit opt-in for legitimate single-agent operation; in solo mode the wrapper still emits a loud warning rather than acting silently. Meanwhile the kickoff template grows a Step 3 "Handshake every visible peer via `macro_contact_handshake`" between `register_agent` and `fetch_inbox`, and AGENTS.md Rule 2 grows a one-paragraph addendum naming the handshake as a prerequisite for wrapper correctness.

---

## 4. Scope of changes

**Fifteen+ artifacts** change, in five physical locations: this repo, the new test directory, the Claude user-skill tree, the Codex user-skill tree, and the install-bead-wrappers asset library mirrored across both user trees. The artifact count climbs above twelve once the §12.4/§12-bis.3 sibling-skill sweep returns its full hit list (12+ files per tree on this machine).

| # | File | Type | Lives in |
|---|------|------|----------|
| 1 | `bin/br-claim` | edit | this repo |
| 2 | `bin/br-complete` | edit | this repo |
| 3 | `bin/tests/test_peer_discovery.py` | **new file** | this repo |
| 4 | `AGENTS.md` | edit (Rule 2 addendum + MCP Agent Mail section addendum + Common Pitfalls bullet) | this repo |
| 5 | `docs/layermaxxer/active/execution-kickoff.md` | edit (§5.1 Session Startup) | this repo |
| 6 | `docs/layermaxxer/active/execution-push.md` | edit (§2.1 Re-Register and Check Inbox) | this repo |
| 7 | `~/.claude/skills/beads-to-execution/TEMPLATES.md` | edit (§2a → new §2a-bis) — propagates to every Claude-Code-generated kickoff in every future project | Claude user home |
| 8 | **`~/.codex/skills/beads-to-execution/TEMPLATES.md`** | edit (mirror of #7) — propagates to every Codex-CLI-generated kickoff in every future project | Codex user home |
| 9 | Sibling skill files in `~/.claude/skills/` that emit AgentMail-Setup blocks (12+ files — full list from §12.4 sweep) | edit (mirror of #7) | Claude user home |
| 10 | Sibling skill files in `~/.codex/skills/` (same 12+ files mirrored) | edit (mirror of #8) | Codex user home |
| 11 | **`~/.claude/skills/install-bead-wrappers/assets/br-claim`** | **byte-for-byte copy of fixed #1** — this is the canonical source from which every future `install-bead-wrappers` invocation copies the wrapper into new projects | Claude user home |
| 12 | **`~/.claude/skills/install-bead-wrappers/assets/br-complete`** | **byte-for-byte copy of fixed #2** | Claude user home |
| 13 | **`~/.claude/skills/install-bead-wrappers/assets/agents-md-rule-2.md`** | **edit (append the same Rule 2 addendum as #4)** — every future project's appended Rule 2 inherits the handshake addendum | Claude user home |
| 14 | **`~/.codex/skills/install-bead-wrappers/assets/br-claim`** | **mirror of #11** (file is byte-identical between the two trees today; keep it that way) | Codex user home |
| 15 | **`~/.codex/skills/install-bead-wrappers/assets/br-complete`** | **mirror of #12** | Codex user home |
| 16 | **`~/.codex/skills/install-bead-wrappers/assets/agents-md-rule-2.md`** | **mirror of #13** | Codex user home |

**Why install-bead-wrappers is load-bearing**: that skill is the operator's standard recipe for adding multi-agent claim/complete discipline to a new git project (the skill's own description says "Copies bin/br-claim and bin/br-complete wrappers, appends AGENTS.md Rule 2, installs the agent-mail precommit hook"). Its assets are the canonical source for new projects. Failing to update them means every future project bootstrapped via this skill ships with the bug from day 1, even though every other artifact in this handoff is fixed.

Out of scope (do **not** touch as part of this handoff):

- `docs/pipeline/l[1-5]-*/artifacts/l*-kickoff.md` and `l*-push.md` — historical, already-consumed artifacts. They drift naturally as projects advance; rewriting them now is churn.
- `bin/br-sync` — no peer-discovery logic; orthogonal.
- The mcp-agent-mail server itself — bug is client-side.
- The Phase B precommit hook at `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` — still works correctly; this handoff strengthens the upstream signal but does not change the hook's contract.

---

## 5. Order of operations (sequenced for safety)

Implement strictly in this order. Each step is committable on its own; do **not** bundle them.

No feature flag, no two-step landing. The entire premise of this handoff is "run after L5 closes" (§0), so there is no active swarm to protect from a one-shot wrapper rewrite. The §13.2 static greps + §13.5 cross-CLI live test catch any helper bug deterministically before the change ever touches a real swarm. Land each step, run its verification gate, then proceed.

| Step | What lands | Why this order / gate |
|------|-----------|----------------------|
| 5.1 | New test file `bin/tests/test_peer_discovery.py` (§8). Run T1–T6 against a live mcp-agent-mail. | Tests are written BEFORE the wrappers change so the helper's contract is locked. T4 in particular pins the "raise on no peers" invariant. |
| 5.2 | `bin/br-claim` + `bin/br-complete` — drop the old `recipients = peers if peers else [agent]` line and land the `discover_peers(...)` helper + CLI flags (§6 + §7). Single commit. **Gate: §13.2 grep checks must all pass before commit.** | One atomic replacement, not parallel paths. After this commit, no wrapper invocation in this repo can silently self-address. |
| 5.3 | `AGENTS.md` — Rule 2 addendum + MCP Agent Mail section step-1 rewrite + Common Pitfalls bullet (§9). Same commit as 5.2 so rule and wrapper behaviour ship together. | Constitutional change pinned to the behaviour change. |
| 5.4 | `docs/layermaxxer/active/execution-kickoff.md` §5.1 + `execution-push.md` §2.1 — add bootstrap-handshake sub-step (§10 + §11). Same commit as 5.2/5.3. | Future layer kickoffs generated against these templates inherit the fix. |
| 5.5 | **`~/.claude/skills/beads-to-execution/TEMPLATES.md`** — insert new sub-step §2a-bis (Bootstrap contact handshakes) between §2a "Boot Your Session" and §2b "Check What's Happening" (§12). Plus §12.4 sibling-skill sweep. | Propagates to every future Claude-Code-generated kickoff. Operator-side edit (not in the repo commit). |
| 5.6 | **`~/.codex/skills/beads-to-execution/TEMPLATES.md`** — same edit, mirrored (§12-bis). Plus §12-bis.3 sibling-skill sweep. **Gate: §12-bis.5 cross-CLI sanity grep must return `1` for both tree's TEMPLATES.md.** | **Without this, every kickoff generated from a Codex session ships the bug.** Universal coverage hinges on this. Operator-side edit. |
| 5.7 | **`~/.claude/skills/install-bead-wrappers/assets/{br-claim, br-complete, agents-md-rule-2.md}`** + the same trio in `~/.codex/skills/install-bead-wrappers/assets/` — `cp` the fixed wrappers, append the Rule 2 addendum to both copies of `agents-md-rule-2.md` (§12-ter). **Gate: §12-ter.2 final `diff -q` chain returns no output.** | **Without this, every NEW project bootstrapped via `install-bead-wrappers` re-introduces the bug from day 1.** §12-ter is the canonical pin. |
| 5.8 | Live validation — spawn one Claude session and one Codex session, run the kickoff bootstrap from each, claim a throwaway bead, assert peer-routed Start mail in BOTH directions (§13.5). | Final acceptance gate; cross-CLI proof. |
| 5.9 | **End-to-end portability proof** — `git init` a `/tmp/...` throwaway repo, run `install-bead-wrappers` from either CLI, confirm the freshly-copied `bin/br-claim` contains `discover_peers` and does NOT contain `peers if peers else [agent]`, and the freshly-appended Rule 2 includes the new handshake addendum (§12-ter.5). | Proves the install-bead-wrappers assets actually carry the fix to new projects. |

**Commit strategy**: steps 5.2–5.4 are one repo-side commit (wrappers + AGENTS.md + layermaxxer templates), so the constitutional rule and the behaviour change land together. Steps 5.5–5.7 are operator-side edits — they don't go through git. Steps 5.1, 5.8, 5.9 are validation gates that produce no artifacts.

Do NOT touch the existing L5 artifacts (`docs/pipeline/l5-.../artifacts/*`). They are frozen.

---

## 6. Change 1 — `bin/br-claim`

**Current state**: 464 lines. The defective block is at lines 396-419. See `bin/br-claim:396` for the exact slice.

### 6.1 Add the helper (insert near the top, after `mcp_call(...)` definition at line 137-197)

The helper duplicates by design — `bin/br-claim` and `bin/br-complete` already duplicate `mcp_call`, `preflight_health`, `resolve_agent_name`, `resolve_bearer_token`, `resolve_project_key`, and `br_show`. The project's chosen pattern is duplication, not a shared `bin/_br_lib.py`. Respect it. If you want a shared lib, file a separate bead.

Also add the following imports to the top of the file (alongside the existing `import argparse`, `import json`, etc.) — keep `datetime` at module top, not lazy-imported inside the function:

```python
from datetime import datetime, timedelta, timezone
```

Then add the helper:

```python
class NoPeersError(RuntimeError):
    """Raised when neither list_contacts nor list_agents yields any peer.

    Carries the diagnostic the user needs to recover (run macro_contact_handshake,
    confirm a second agent is registered, or pass --solo).
    """


def _parse_iso_ts(value):
    """Parse an ISO-8601 timestamp; tolerate trailing 'Z' and missing tz."""
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


# Sentinel for "auto-derive same_program from self's list_agents entry".
# Distinct from None (which here means "no filter") and "" (also "no filter").
_AUTO_PROGRAM = object()


def discover_peers(
    url, token, project_key, agent,
    *,
    max_idle_seconds=3600,
    same_program=_AUTO_PROGRAM,
    allow_solo=False,
    tool_label="br-claim",
):
    """Resolve recipients for Start/Complete mail.

    Strategy:
      1. list_contacts — preferred (explicit handshake links).
      2. list_agents — fallback, filtered by liveness (last_active_ts within
         max_idle_seconds) and by same program. Same-program defaults to
         "auto-derive from the agent's own list_agents entry"; this matches
         the operator's manual L5 workaround semantics ("peers whose program
         matches yours"). Pass same_program="" or None to disable the filter
         entirely; pass a literal string to force it.
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

    # If caller didn't explicitly pin a program, auto-derive from self's entry.
    # This matches the operator's manual L5 workaround ("peers whose program
    # matches yours"). Treat self-not-found as "no filter" (don't ghost peers
    # if we can't find ourselves).
    if same_program is _AUTO_PROGRAM:
        same_program = ""
        for entry in candidates:
            if not isinstance(entry, dict):
                continue
            name = entry.get("name") or entry.get("agent_name")
            if name == agent:
                same_program = entry.get("program") or ""
                break

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
    # Note: do NOT prefix this message with `{tool_label}:` — the caller wraps
    # `discover_peers` in `die(str(e))`, and `die()` already adds the wrapper
    # name as a prefix. Including it here would produce a double prefix
    # ("br-claim: br-claim: no peers ..."). The tool_label is still used in
    # the recovery-options block to suggest the right --solo invocation.
    raise NoPeersError(
        "no peers discoverable.\n"
        f"  list_contacts returned [] AND list_agents returned no live peers "
        f"(idle cutoff: {max_idle_seconds}s, program filter: {same_program!r}).\n"
        f"  Recovery options:\n"
        f"    1. Handshake an existing peer:\n"
        f"         macro_contact_handshake(project_key={project_key!r},\n"
        f"                                 requester={agent!r}, target=<peer>)\n"
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
            "matches this string. Default: auto-derived from your own program in "
            "list_agents (matches the operator's L5 workaround semantics). "
            "Pass '*' to disable the filter (include peers regardless of program). "
            "Pass a literal program name to force-pin (rarely needed)."
        ),
    )
```

### 6.3 Replace the defective block (was lines 396-419 pre-edit; will have shifted after §6.1 + §6.2)

**Line-number caveat**: §6.1 inserts ~110 lines of helper code BEFORE the original line 396, and §6.2 inserts ~25 lines of CLI flags into `parse_args()`. So by the time you reach this edit, the original line 396 is now at roughly line 530. **Do not chase line numbers — use Edit's exact-string match against the unique text below.** The old block starts with `# 6. SEND START MAIL (unless idempotency says skip).` and ends with `recipients = peers if peers else [agent]` — both phrases are unique in the file.

Old (DELETE — these are the 23 lines that ship the bug):

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
        # CLI semantics for --peer-same-program:
        #   not passed (None) → auto-derive from self's list_agents entry (default)
        #   '*'              → disable filter (include peers across all programs)
        #   any other string → force-pin to that literal
        if args.peer_same_program is None:
            program_arg = _AUTO_PROGRAM
        elif args.peer_same_program == "*":
            program_arg = ""
        else:
            program_arg = args.peer_same_program
        try:
            recipients, source = discover_peers(
                url, token, project_key, agent,
                max_idle_seconds=args.peer_max_idle_seconds,
                same_program=program_arg,
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

### 7.3 Replace the defective block (was lines 374-395 pre-edit; will have shifted after §7.1 + §7.2)

**Line-number caveat (same as §6.3)**: §7.1's helper insertion and §7.2's CLI flags push the original line 374 to roughly line 510. Use the unique block text to locate, not line numbers. The block starts with `# 4. SEND COMPLETE MAIL.` and ends with `recipients = peers if peers else [agent]`.

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
    # CLI semantics for --peer-same-program — see br-claim §6.3 for the contract.
    if args.peer_same_program is None:
        program_arg = _AUTO_PROGRAM
    elif args.peer_same_program == "*":
        program_arg = ""
    else:
        program_arg = args.peer_same_program
    try:
        recipients, source = discover_peers(
            url, token, project_key, agent,
            max_idle_seconds=args.peer_max_idle_seconds,
            same_program=program_arg,
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
            "requester": a,
            "target": b,
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
            "requester": a,
            "target": b,
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

~~~
1. **Register identity:**
   ```
   ensure_project(project_key=<abs-path>)
   register_agent(project_key, program, model)
   ```
~~~

Replace with:

~~~
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
               requester=<you>,
               target=peer["name"],
           )
   ```
~~~

### 9.3 Common Pitfalls addendum (around line 256)

Add a fourth bullet:

```
- **Empty `list_contacts` after `set_contact_policy(policy="open")`**: expected. Policy enables incoming requests; it does not auto-handshake. Run `macro_contact_handshake` (see Same Repository Workflow step 1) before relying on `list_contacts(...)` for routing. `bin/br-claim` / `bin/br-complete` will refuse to silently self-address — they fall back to `list_agents` and, failing that, raise `NoPeersError`.
```

---

## 10. Change 5 — `docs/layermaxxer/active/execution-kickoff.md`

The layermaxxer template at `docs/layermaxxer/active/execution-kickoff.md` is the source for every layermaxxer execution prompt. Currently §5.1 (around line 175) registers via `macro_start_session` and fetches inbox but does NOT handshake.

Find §5.1 "Session Startup (Do This Once at Session Start)" and add a sub-step between registration and the inbox check. Headings inside the `~~~` block below are content of the layermaxxer template, NOT headings inside this handoff doc:

~~~
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
# 1. Discover live peers in YOUR program.
#    _within_last_hour is a 3-line helper:
#      def _within_last_hour(ts): return ts and (now_utc() - parse_iso(ts)) < timedelta(hours=1)
peers = list_agents(project_key="/Users/alinaaitova/Desktop/Testproject1")
my_program = next((p["program"] for p in peers if p["name"] == <your-name>), None)
candidates = [p for p in peers
              if p["name"] != <your-name>
              and p.get("program") == my_program
              and _within_last_hour(p.get("last_active_ts"))]

# 2. Handshake each.
for peer in candidates:
    macro_contact_handshake(
        project_key="/Users/alinaaitova/Desktop/Testproject1",
        requester=<your-name>,
        target=peer["name"],
    )

# 3. VERIFY — every candidate must now appear in list_contacts.
verified = list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1",
                         agent_name=<your-name>)
verified_names = {(c["name"] if isinstance(c, dict) else c) for c in (verified or [])}
missing = [p["name"] for p in candidates if p["name"] not in verified_names]
assert not missing, \
    f"Handshake verification failed — these peers did NOT land in list_contacts: {missing}. " \
    f"Investigate (rate limit, server error, race) before claiming any bead."
```

Refresh this handshake loop whenever you notice a new peer in `list_agents` that is not in your `list_contacts` — i.e., on every fresh inbox check, before each new `bin/br-claim`.

If `macro_start_session` is unavailable, do it manually:
...
~~~

(The remainder of §5.1 — the manual fallback — is unchanged.)

---

## 11. Change 6 — `docs/layermaxxer/active/execution-push.md`

§2.1 (around line 40-50) currently re-registers and fetches inbox. Add the same handshake sub-step.

Find §2.1 "Re-Register and Check Inbox" and immediately after the `macro_start_session(...)` block, insert the following. The `~~~` outer fence prevents the inner ``` from terminating prematurely; everything inside is content to paste into the template:

~~~
**Refresh contact handshakes.** Other agents may have joined the swarm between your sessions. Without an explicit handshake, the `bin/br-claim` / `bin/br-complete` wrappers cannot peer-route Start/Complete mail to them:

```
# _within_last_hour is a 3-line helper:
#   def _within_last_hour(ts): return ts and (now_utc() - parse_iso(ts)) < timedelta(hours=1)
peers = list_agents(project_key="/Users/alinaaitova/Desktop/Testproject1")
contacts = list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)
contact_names = {(c["name"] if isinstance(c, dict) else c) for c in (contacts or [])}
my_program = next((p["program"] for p in peers if p["name"] == <your-name>), None)
new_handshakes = []
for peer in peers:
    if (peer["name"] != <your-name>
        and peer["name"] not in contact_names
        and peer.get("program") == my_program
        and _within_last_hour(peer.get("last_active_ts"))):
        macro_contact_handshake(
            project_key="/Users/alinaaitova/Desktop/Testproject1",
            requester=<your-name>,
            target=peer["name"],
        )
        new_handshakes.append(peer["name"])

# Verify — list_contacts must now include every peer you just handshook.
contacts_after = list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1",
                               agent_name=<your-name>)
after_names = {(c["name"] if isinstance(c, dict) else c) for c in (contacts_after or [])}
missing = [n for n in new_handshakes if n not in after_names]
assert not missing, \
    f"Refresh handshake failed for: {missing}. Investigate before next bin/br-claim."
```

Do this on every session resume and after any "swarm grows" event.
~~~

---

## 12. Change 7 — `~/.claude/skills/beads-to-execution/TEMPLATES.md` (Claude-side user-level skill)

**This is one of the two highest-leverage changes of the entire handoff.** `~/.claude/skills/beads-to-execution/TEMPLATES.md` is the template the `/beads-to-execution` skill consumes to generate kickoff/push prompts for every future project. The L5 `10-generate-kickoff.md` invokes this skill on its very first instruction ("Invoke the `/beads-to-execution` skill"). Updating this template = every future Claude-Code-generated kickoff inherits the handshake step automatically, in every future project.

The Codex CLI mirror lives at `~/.codex/skills/beads-to-execution/TEMPLATES.md` (§12-bis) — apply BOTH edits or universal coverage breaks.

### 12.1 Locate the insertion point

The template's AgentMail section is `## Step 2: Register with AgentMail (BEFORE ANY CODE)` at `TEMPLATES.md:145`. Its sub-structure today (verified against the file as of 2026-05-19) is:

- `### 2a. Boot Your Session` — `TEMPLATES.md:151-175` — emits `ensure_project` + `register_agent` OR `macro_start_session`. **Does NOT call `set_contact_policy` at all** (so don't search for that — it isn't there).
- `### 2b. Check What's Happening (Situational Awareness)` — `TEMPLATES.md:177-194` — `fetch_inbox`, `search_messages`, `br ready`.
- `### 2c. Claim Your Bead (use the wrapper — atomic)` — `TEMPLATES.md:196+` — `bin/br-claim` discipline.

The insertion is a **new sub-section `### 2a-bis. Bootstrap Contact Handshakes (REQUIRED)`** between §2a (line 175) and §2b (line 177). This keeps the existing §2a verbatim (don't add `set_contact_policy` inline — that's a separate decision the operator may want to leave to per-project kickoff customization).

### 12.2 The verbatim insertion

Apply this Edit. Old (the current boundary between §2a and §2b at `~/.claude/skills/beads-to-execution/TEMPLATES.md:174-178`) — the `### 2b.` heading below is content of the TEMPLATES.md file, not a heading inside this handoff doc; the `~~~` fence makes that explicit:

~~~
macro_start_session(
  human_key="{absolute_project_path}",
  program="claude-code",
  model="<your-model>",
  task_description="{Feature Name} — {scope description}"
)
```

### 2b. Check What's Happening (Situational Awareness)
~~~

New (inserts a §2a-bis sub-section without touching §2a or §2b headings — again, the `### 2a-bis.` and `### 2b.` below are TEMPLATES.md content, not handoff-doc headings):

~~~
macro_start_session(
  human_key="{absolute_project_path}",
  program="claude-code",
  model="<your-model>",
  task_description="{Feature Name} — {scope description}"
)
```

### 2a-bis. Bootstrap Contact Handshakes (REQUIRED before any `bin/br-claim`)

`register_agent` / `macro_start_session` records your existence. It does NOT establish bilateral contact links with other agents. Without explicit handshakes, `list_contacts(<you>)` is empty and the `bin/br-claim` / `bin/br-complete` wrappers fall back to a liveness-filtered `list_agents` — or, if that is also empty, raise `NoPeersError`. For deterministic peer routing, handshake every visible peer at session start:

```
# 1. Discover peers whose program matches yours and who are active in the last hour.
#    _within_last_hour is a 3-line inline helper:
#      def _within_last_hour(ts): return ts and (now_utc() - parse_iso(ts)) < timedelta(hours=1)
peers = list_agents(project_key="{absolute_project_path}")
my_program = next((p["program"] for p in peers if p["name"] == "<your-agent-name>"), None)
candidates = [p for p in peers
              if p["name"] != "<your-agent-name>"
              and p.get("program") == my_program
              and _within_last_hour(p.get("last_active_ts"))]

# 2. Handshake each.
for peer in candidates:
    macro_contact_handshake(
        project_key="{absolute_project_path}",
        requester="<your-agent-name>",
        target=peer["name"],
    )

# 3. VERIFY — every candidate must now appear in list_contacts.
verified = list_contacts(project_key="{absolute_project_path}",
                         agent_name="<your-agent-name>")
verified_names = {(c["name"] if isinstance(c, dict) else c) for c in (verified or [])}
missing = [p["name"] for p in candidates if p["name"] not in verified_names]
assert not missing, \
    f"Handshake verification failed for: {missing}. Investigate (rate limit, " \
    f"server error, race) before claiming any bead."
```

Refresh this loop whenever `list_agents` shows a new peer not yet in your `list_contacts` — i.e. before every `bin/br-claim` if the swarm is growing. The wrappers' `list_agents` fallback (auto-derives your program and filters by same-program-only) covers you between refreshes, but explicit handshakes are deterministic.

### 2b. Check What's Happening (Situational Awareness)
~~~

### 12.3 Quick-reference addendum (also in TEMPLATES.md)

The same `TEMPLATES.md` will contain (or be extended by sibling skills to contain) an "AgentMail Quick Reference" block that becomes §11 of generated kickoffs and §8 of generated push prompts. Find it via `grep -n 'Quick Reference\|AgentMail Quick' ~/.claude/skills/beads-to-execution/TEMPLATES.md` — if a Quick Reference block exists, add this paragraph at the top:

```
**Refresh handshakes on swarm growth.** If `list_agents` shows a new peer not yet in your `list_contacts`, call `macro_contact_handshake(...)` for that peer before your next `bin/br-claim`. The wrappers' layered peer-discovery (list_contacts → list_agents → loud error) covers you in the meantime, but explicit handshakes make peer routing deterministic.
```

If the Quick Reference block lives in a sibling skill (`pipemaxxer/TEMPLATES.md`, `layermaxxer/references/TEMPLATES.md`), add it there instead. The §12.4 grep below covers this.

### 12.4 Sibling-skill sweep (Claude tree)

Other skills under `~/.claude/skills/` may emit their own AgentMail-Setup blocks; the same handshake gap exists wherever they do. Grep:

```bash
grep -l -E 'register_agent|macro_start_session|set_contact_policy' \
  ~/.claude/skills/*/TEMPLATES.md \
  ~/.claude/skills/*/references/TEMPLATES.md \
  ~/.claude/skills/TEMPLATES.md \
  2>/dev/null
```

Full hit list on the current machine (verified 2026-05-19 — re-grep before editing in case paths drift):

- `~/.claude/skills/bead-completion-audit/TEMPLATES.md`
- `~/.claude/skills/bead-plan-audit/TEMPLATES.md`
- `~/.claude/skills/beads-from-sources/TEMPLATES.md`
- `~/.claude/skills/beads-to-execution/CHECKLIST.md`  ← additional to TEMPLATES.md
- `~/.claude/skills/beads-to-execution/TEMPLATES.md`  ← §12.1-12.3 target
- `~/.claude/skills/featuremaxxer/SKILL.md`
- `~/.claude/skills/featuremaxxer/references/TEMPLATES.md`
- `~/.claude/skills/install-bead-wrappers/SKILL.md`  ← documents wrapper installation (covered by §12-ter)
- `~/.claude/skills/install-bead-wrappers/CHECKLIST.md`
- `~/.claude/skills/layermaxxer/SKILL.md`
- `~/.claude/skills/layermaxxer/references/TEMPLATES.md`
- `~/.claude/skills/pipemaxxer/TEMPLATES.md`
- `~/.claude/skills/planmaxer/references/TEMPLATES.md`

For each match, locate the AgentMail-Setup block (search for `register_agent` or `macro_start_session`) and either (a) add the same `### 2a-bis. Bootstrap Contact Handshakes` sub-section after registration, OR (b) if the block is shorter/inline, insert the verbatim handshake loop (the one in §12.2 with `_within_last_hour` filtering and the list_contacts read-back verification) immediately after the `register_agent` / `macro_start_session` call. Do NOT touch skills that don't emit AgentMail blocks (e.g. `spec-to-tdd`, `codebase-to-spec`, `error-manifest`, `idea-to-research`).

If a hit is documentation-only (e.g. `bead-completion-audit/TEMPLATES.md` may just MENTION AgentMail without emitting a setup block), inspect first — only edit if the file actually emits setup steps that a generated kickoff/audit prompt will inherit.

### 12.5 Versioning / commit

`~/.claude/` may or may not be under separate VCS — check `~/.claude/.git` (likely absent on this machine). If unversioned, the edit is in-place and survives via filesystem alone; note it in the operator handoff back-channel (see §15) and copy the edited file into a personal backup if the operator wants belt-and-suspenders. The change is idempotent — adding a handshake loop block twice produces an obviously-redundant template; the operator will notice and remove the duplicate.

---

## 12-bis. Change 8 — `~/.codex/skills/beads-to-execution/TEMPLATES.md` (Codex-side user-level skill, MIRROR)

**This is the other highest-leverage change.** Codex CLI maintains a parallel skill registry at `~/.codex/skills/` with the SAME skill names as `~/.claude/skills/` (verified on the current machine — `~/.codex/skills/{beads-to-execution,pipemaxxer,layermaxxer,...}` all exist). When the operator generates a kickoff/push prompt from a Codex CLI session, the Codex `/beads-to-execution` skill loads from `~/.codex/skills/beads-to-execution/TEMPLATES.md`. Without this mirror, every kickoff generated from a Codex session would ship the bug.

### 12-bis.1 Confirm the Codex tree exists and mirrors the Claude tree

```bash
# Required: both should print a non-empty TEMPLATES.md path.
ls -la ~/.codex/skills/beads-to-execution/TEMPLATES.md
ls -la ~/.claude/skills/beads-to-execution/TEMPLATES.md

# Recommended: diff the two to see whether they are byte-identical, near-identical, or diverged.
diff -u ~/.claude/skills/beads-to-execution/TEMPLATES.md \
        ~/.codex/skills/beads-to-execution/TEMPLATES.md | head -100
```

Two outcomes:

- **Byte-identical** (or near-identical): apply the §12.2 edit verbatim to the Codex file. Done.
- **Diverged** (different section numbering, or Codex tree predates a Claude tree refresh): the Codex template may not have a `### 2a` / `### 2b` split. Search for the AgentMail registration block via `grep -n 'register_agent\|macro_start_session' ~/.codex/skills/beads-to-execution/TEMPLATES.md`. Insert the same handshake block (the verbatim "### 2a-bis. Bootstrap Contact Handshakes ..." chunk from §12.2) immediately after the registration call, regardless of surrounding section numbering. Adjust the heading level (`###` vs `##`) to match the surrounding Codex template structure.

### 12-bis.2 Mirror the Quick-Reference addendum

Apply the §12.3 quick-reference paragraph to the Codex tree as well. Grep:

```bash
grep -n 'Quick Reference\|AgentMail Quick' ~/.codex/skills/beads-to-execution/TEMPLATES.md
```

### 12-bis.3 Codex sibling-skill sweep

Same as §12.4 but against the Codex tree:

```bash
grep -l -E 'register_agent|macro_start_session|set_contact_policy' \
  ~/.codex/skills/*/TEMPLATES.md \
  ~/.codex/skills/*/references/TEMPLATES.md \
  ~/.codex/vendor_imports/skills/**/TEMPLATES.md \
  2>/dev/null
```

Full hit list on the Codex tree (verified 2026-05-19 — same 12 paths as the Claude tree, since the two trees mirror each other):

- `~/.codex/skills/bead-completion-audit/TEMPLATES.md`
- `~/.codex/skills/bead-plan-audit/TEMPLATES.md`
- `~/.codex/skills/beads-from-sources/TEMPLATES.md`
- `~/.codex/skills/beads-to-execution/CHECKLIST.md`
- `~/.codex/skills/beads-to-execution/TEMPLATES.md`  ← §12-bis.1-12-bis.2 target
- `~/.codex/skills/featuremaxxer/SKILL.md`
- `~/.codex/skills/featuremaxxer/references/TEMPLATES.md`
- `~/.codex/skills/install-bead-wrappers/SKILL.md`  ← covered by §12-ter
- `~/.codex/skills/install-bead-wrappers/CHECKLIST.md`
- `~/.codex/skills/layermaxxer/SKILL.md`
- `~/.codex/skills/layermaxxer/references/TEMPLATES.md`
- `~/.codex/skills/pipemaxxer/TEMPLATES.md`
- `~/.codex/skills/planmaxer/references/TEMPLATES.md`

For each match that emits an AgentMail-Setup block, mirror the handshake insertion. Same content as the Claude-side edit — the Codex tree is designed to stay byte-identical to Claude where it makes sense.

### 12-bis.4 Versioning / commit (Codex side)

Codex CLI does not version-control `~/.codex/` either (the directory contains config.toml, sessions, sqlite, etc. — all local state). The edit is in-place. Note it in the operator handoff back-channel alongside the §12 note.

### 12-bis.5 Cross-CLI sanity check (do this immediately after §12 and §12-bis both land)

```bash
# Confirm both trees have the same canonical phrase in their AgentMail-Setup block.
# The phrase is a unique substring of the inserted handshake comment.
grep -c 'Bootstrap Contact Handshakes' \
  ~/.claude/skills/beads-to-execution/TEMPLATES.md \
  ~/.codex/skills/beads-to-execution/TEMPLATES.md
# Expected output: 1 and 1 (one match per file). If either is 0, that side
# is unpatched — universal coverage broken.
```

If the cross-CLI sanity check fails: do NOT proceed to §13.5 (live validation); finish the missing edit first.

---

## 12-ter. Change 9-13 — `install-bead-wrappers` asset library (BOTH user trees)

**Without this, every NEW project bootstrapped via `install-bead-wrappers` from a fresh `git init` re-introduces the bug from day 1**, even after every other artifact in this handoff is fixed. The `install-bead-wrappers` skill is the operator's standard recipe for adding multi-agent claim/complete discipline to a new project (per its own SKILL.md description: "Copies bin/br-claim and bin/br-complete wrappers, appends AGENTS.md Rule 2, installs the agent-mail precommit hook"). Its asset library is the canonical source.

### 12-ter.1 Confirm the state

```bash
# Both trees have an assets directory with the same three files. Both currently
# contain the bug at the SAME line numbers as the live wrappers (verified by the
# author on 2026-05-19).
ls -la ~/.claude/skills/install-bead-wrappers/assets/
ls -la ~/.codex/skills/install-bead-wrappers/assets/

# Quick proof the bug lives here too:
grep -n 'peers if peers else \[agent\]' \
  ~/.claude/skills/install-bead-wrappers/assets/br-claim \
  ~/.claude/skills/install-bead-wrappers/assets/br-complete \
  ~/.codex/skills/install-bead-wrappers/assets/br-claim \
  ~/.codex/skills/install-bead-wrappers/assets/br-complete
# Expected before fix: 4 matches (line 419 in br-claim, line 395 in br-complete, in both trees).
# Expected after fix:  0 matches.

# Confirm both trees' assets are byte-identical today.
diff -q ~/.claude/skills/install-bead-wrappers/assets/ \
        ~/.codex/skills/install-bead-wrappers/assets/
# Expected: no output (identical). If they diverged, flag to operator but proceed.
```

### 12-ter.2 Sync the fixed wrappers into BOTH asset trees

After the live `bin/br-claim` and `bin/br-complete` have landed (§6 + §7 + §13.1-13.4 passed), copy them verbatim into both asset libraries. The skill's `METHODOLOGY.md:28-30` documents this exact procedure:

```bash
SRC_ROOT=/Users/alinaaitova/Desktop/Testproject1

# Mirror into Claude tree.
cp "$SRC_ROOT/bin/br-claim"    ~/.claude/skills/install-bead-wrappers/assets/br-claim
cp "$SRC_ROOT/bin/br-complete" ~/.claude/skills/install-bead-wrappers/assets/br-complete
chmod +x ~/.claude/skills/install-bead-wrappers/assets/br-claim \
         ~/.claude/skills/install-bead-wrappers/assets/br-complete

# Mirror into Codex tree.
cp "$SRC_ROOT/bin/br-claim"    ~/.codex/skills/install-bead-wrappers/assets/br-claim
cp "$SRC_ROOT/bin/br-complete" ~/.codex/skills/install-bead-wrappers/assets/br-complete
chmod +x ~/.codex/skills/install-bead-wrappers/assets/br-claim \
         ~/.codex/skills/install-bead-wrappers/assets/br-complete

# Verify the bug is gone from both asset trees.
grep -c 'peers if peers else \[agent\]' \
  ~/.claude/skills/install-bead-wrappers/assets/br-claim \
  ~/.claude/skills/install-bead-wrappers/assets/br-complete \
  ~/.codex/skills/install-bead-wrappers/assets/br-claim \
  ~/.codex/skills/install-bead-wrappers/assets/br-complete
# Expected: 0 for each file.

# Verify the fixed helper is present.
grep -c '^def discover_peers' \
  ~/.claude/skills/install-bead-wrappers/assets/br-claim \
  ~/.claude/skills/install-bead-wrappers/assets/br-complete \
  ~/.codex/skills/install-bead-wrappers/assets/br-claim \
  ~/.codex/skills/install-bead-wrappers/assets/br-complete
# Expected: 1 for each file.

# Final assertion: assets byte-identical with the live wrappers AND with each other.
diff -q "$SRC_ROOT/bin/br-claim"    ~/.claude/skills/install-bead-wrappers/assets/br-claim
diff -q "$SRC_ROOT/bin/br-complete" ~/.claude/skills/install-bead-wrappers/assets/br-complete
diff -q ~/.claude/skills/install-bead-wrappers/assets/ \
        ~/.codex/skills/install-bead-wrappers/assets/
# All three should produce no output.
```

### 12-ter.3 Append the Rule 2 addendum to the AGENTS.md asset (BOTH trees)

`~/.claude/skills/install-bead-wrappers/assets/agents-md-rule-2.md` is the canonical block the skill appends to every new project's `AGENTS.md`. It contains the same Rule 2 content as this project's `AGENTS.md:21-33` plus the BEGIN/END HTML-comment markers for idempotent re-append. Edit it to include the SAME bootstrap-handshake addendum paragraph you applied in §9.1, immediately before the `<!-- END: AGENTS.md Rule 2 -->` marker.

Old (the last paragraph before the END marker — verbatim from the asset file):

```
This is a constitutional rule, equal in force to Rule 1 (no file deletion). It applies to every layer (L0 through L6 and beyond) and every future stage.

<!-- END: AGENTS.md Rule 2 -->
```

New (insert the addendum paragraph between the existing last line and the END marker):

```
This is a constitutional rule, equal in force to Rule 1 (no file deletion). It applies to every layer (L0 through L6 and beyond) and every future stage.

**Prerequisite — bootstrap contact handshake.** `bin/br-claim` and `bin/br-complete` send Start/Complete mail to the peers returned by `list_contacts(...)`. `contact_policy: "open"` enables incoming requests but does NOT auto-create handshake links — without an explicit `macro_contact_handshake(...)`, `list_contacts` is empty and the wrappers fall back to a liveness-filtered `list_agents` (or raise `NoPeersError` if neither yields peers). For deterministic peer routing, run `macro_contact_handshake` against every visible peer (filtered to your own program, active within the last hour) at session start, and refresh after any swarm-growth event. The wrappers also accept `--solo` (or `AGENT_MAIL_ALLOW_SOLO=1`) for legitimate single-agent operation, which still emits a loud warning rather than acting silently.

<!-- END: AGENTS.md Rule 2 -->
```

Then mirror the same edit to `~/.codex/skills/install-bead-wrappers/assets/agents-md-rule-2.md`. Verify both files are byte-identical post-edit:

```bash
diff -q ~/.claude/skills/install-bead-wrappers/assets/agents-md-rule-2.md \
        ~/.codex/skills/install-bead-wrappers/assets/agents-md-rule-2.md
# Expected: no output (identical).

grep -c 'bootstrap contact handshake' \
  ~/.claude/skills/install-bead-wrappers/assets/agents-md-rule-2.md \
  ~/.codex/skills/install-bead-wrappers/assets/agents-md-rule-2.md
# Expected: 1 for each file.
```

### 12-ter.4 Re-validate METHODOLOGY.md is still accurate

`METHODOLOGY.md` documents the manual sync command and reference line counts (e.g., "`bin/br-claim` (~470 lines)"). After §6 + §7, the wrappers grow by roughly +100 lines each (helper + CLI flags). Update both copies:

```bash
# Find the line-count references in both METHODOLOGY.md copies.
grep -n 'Source paths' ~/.claude/skills/install-bead-wrappers/METHODOLOGY.md \
                       ~/.codex/skills/install-bead-wrappers/METHODOLOGY.md
```

Apply the same text edit (replacing the numbers) to both METHODOLOGY.md files. Housekeeping, not load-bearing.

### 12-ter.5 End-to-end portability proof

Once §12-ter.1 through §12-ter.4 are all green, prove the fix actually propagates to a freshly-bootstrapped project:

```bash
# 1. Create a throwaway test project.
TEST_PROJECT=$(mktemp -d /tmp/install-bead-wrappers-postfix-test.XXXXXX)
cd "$TEST_PROJECT"
git init -q

# 2. From inside this session (Claude OR Codex), invoke the install-bead-wrappers
#    skill — the user-facing trigger is typing `/install-bead-wrappers` in a
#    Claude session, or equivalent in Codex. The skill auto-discovers its assets
#    from ~/.claude/skills/.../assets/ (Claude) or ~/.codex/skills/.../assets/
#    (Codex) per its Step 0 logic, and cp's them into bin/ + appends Rule 2 to
#    AGENTS.md + installs the precommit hook via `am guard install`.
#
#    If you cannot invoke the slash command from your environment, you can
#    simulate it by directly running:
#       cp ~/.claude/skills/install-bead-wrappers/assets/br-claim    bin/br-claim
#       cp ~/.claude/skills/install-bead-wrappers/assets/br-complete bin/br-complete
#       chmod +x bin/br-claim bin/br-complete
#       touch AGENTS.md
#       cat ~/.claude/skills/install-bead-wrappers/assets/agents-md-rule-2.md >> AGENTS.md
#    The skill does additional safety checks (preflight, hook install) but the
#    portability assertions in step 3 below only need the cp + append.

# 3. Confirm the freshly-copied wrappers are the FIXED version.
grep -c '^def discover_peers' bin/br-claim bin/br-complete       # Expected: 1, 1
grep -c 'peers if peers else \[agent\]' bin/br-claim bin/br-complete   # Expected: 0, 0
grep -c '"--solo"' bin/br-claim bin/br-complete                  # Expected: 1, 1

# 4. Confirm the freshly-appended Rule 2 contains the handshake addendum.
grep -c 'bootstrap contact handshake' AGENTS.md                   # Expected: 1

# 5. Tear down.
cd /tmp && rm -rf "$TEST_PROJECT"
```

If all assertions pass: the install-bead-wrappers fix is durable. Every future project bootstrapped from either tree will ship the corrected wrappers and Rule 2 from day one.

### 12-ter.6 Notify the operator

§15 / §18 wrap-up should explicitly list the install-bead-wrappers asset versions as updated, with timestamp. Operator may want to back up previous asset versions before overwriting — ask first if there's any chance the assets were customized in a way that would be lost.

---

## 13. Tests + verification

### 13.1 Unit tests (the new file from §8)

```bash
# With mcp-agent-mail running on 127.0.0.1:8765:
python3 -m unittest bin.tests.test_peer_discovery -v

# Expected: 6 tests pass (T1, T2, T3, T4, T5, T6).
# Without mcp-agent-mail: 6 tests skipped, exit 0.
```

Note on coverage: the tests directly exercise `bin/br-claim`'s `discover_peers` only. `bin/br-complete`'s helper is intentionally a verbatim copy (per the project's duplication pattern — see §6.1), so the contract is identical. The §13.2 grep checks plus the §13.5 live round-trip from the Codex pane (which uses `bin/br-complete`) catch any drift between the two.

### 13.2 Static cross-file consistency greps (run after §6 and §7 both land)

Before any wrapper smoke test, confirm the helper landed in BOTH wrappers and the silent self-fallback is gone from BOTH. Each grep MUST return the expected line count:

```bash
# Helper definition present in both wrappers (1 each = 2 total):
grep -c '^def discover_peers' bin/br-claim bin/br-complete
# Expected: bin/br-claim:1   bin/br-complete:1

# NoPeersError class defined in both wrappers (1 each):
grep -c '^class NoPeersError' bin/br-claim bin/br-complete
# Expected: bin/br-claim:1   bin/br-complete:1

# Helper invocation present in both wrappers (1 each):
grep -c 'discover_peers(' bin/br-claim bin/br-complete
# Expected: bin/br-claim:>=2   bin/br-complete:>=2   (1 def + ≥1 call each)

# Silent self-fallback GONE from both wrappers (0 each):
grep -c 'peers if peers else \[agent\]' bin/br-claim bin/br-complete
# Expected: bin/br-claim:0   bin/br-complete:0

# --solo flag present in both wrappers (1 each):
grep -c '"--solo"' bin/br-claim bin/br-complete
# Expected: bin/br-claim:1   bin/br-complete:1
```

Any deviation: stop and fix the missing edit before continuing.

### 13.3 Wrapper smoke test (no real bead)

Run from inside Testproject1 (the wrapper resolves `project_key` via `git rev-parse --show-toplevel`):

```bash
# Pick a non-existent bead-id and --dry-run. The wrapper should fail at the
# br_show step ("bead not found"), NOT at the peer-discovery step.
./bin/br-claim nonexistent-bead-id-12345 --dry-run --paths /tmp/test
# Expected exit: non-zero, error message about bead not found.
# This proves: the wrapper loads, parses args, resolves agent/token/project_key,
# and reaches br_show before peer discovery. It does NOT prove peer discovery works.

# To prove peer discovery: spin up a temporary throwaway project_key in /tmp
# (so you don't pollute the real Testproject1 archive with test agents).
# Either:
#   (a) use --solo to prove the solo-mode path:
AGENT_MAIL_ALLOW_SOLO=1 AGENT_MAIL_PROJECT_KEY=/tmp/test-smoke-$(uuidgen) \
  ./bin/br-claim <real-or-fake-bead-id> --dry-run
# Expected: prints solo warning, completes dry-run (or fails at br_show if the
# bead doesn't exist — which is fine; the peer-discovery path already ran).
#   (b) or rely on §13.5 to prove the multi-peer path in a real cross-CLI swarm.
```

### 13.4 No-peers regression

Single-agent scenario, in a clean throwaway project_key (NOT the live Testproject1, to avoid polluting the real archive): register yourself only, run `bin/br-claim`. Expected:

```
br-claim: no peers discoverable.
  list_contacts returned [] AND list_agents returned no live peers (idle cutoff: 3600s, program filter: '').
  Recovery options:
    1. Handshake an existing peer:
         macro_contact_handshake(project_key='/tmp/...', requester='...', target=<peer>)
    2. Confirm a second agent is registered in this project:
         list_agents(project_key='/tmp/...')
    3. If this really is a swarm of one, opt in explicitly:
         br-claim <bead-id> --solo
         (or export AGENT_MAIL_ALLOW_SOLO=1)
```

Exit code non-zero. No Start mail archived. Single "br-claim:" prefix (the §6.1 `NoPeersError` body omits the prefix because `die()` already adds it).

### 13.5 Cross-CLI live swarm validation (THE universal-coverage acceptance gate)

This is the load-bearing test for the operator's "I want a universal fix" requirement. It MUST be run with ONE Claude Code session and ONE Codex CLI session, not two Claude sessions. After §6–§12-bis have all landed:

1. **Pane A — Claude Code session.** Bring up a fresh Claude session in `/Users/alinaaitova/Desktop/Testproject1`. Have it run the generated kickoff's `### 5. AgentMail Setup` step (which, post-§10/§12, now includes the bootstrap handshake loop). Capture its assigned agent name as `${CLAUDE_AGENT}`.

2. **Pane B — Codex CLI session.** Bring up a fresh Codex session in the same directory. Run the kickoff (post-§12-bis, it should also include the bootstrap handshake loop — verify by reading the generated prompt OR by manually invoking `list_agents` + the handshake loop). Capture its agent name as `${CODEX_AGENT}`.

3. **Cross-check the handshake actually fired**. From either pane:
   ```
   list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1",
                 agent_name="${CLAUDE_AGENT}")
   # Expected: contains ${CODEX_AGENT}.

   list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1",
                 agent_name="${CODEX_AGENT}")
   # Expected: contains ${CLAUDE_AGENT}.
   ```

4. **Wrapper round-trip — claim from the Claude pane**. Create a throwaway bead in `br` (e.g. `br create --title "handshake-fix-validation" --type=task --priority=4`), then from the Claude pane: `bin/br-claim <throwaway> --paths /tmp/handshake-validation`. Inspect:
   ```
   search_messages(project_key="/Users/alinaaitova/Desktop/Testproject1",
                   query="Start", limit=5, ranking="recency")
   ```
   The most recent Start mail's `from` field MUST be `${CLAUDE_AGENT}` and its `to` field MUST contain `${CODEX_AGENT}` (NOT `${CLAUDE_AGENT}`).

5. **Wrapper round-trip — complete from the Codex pane**. From the Codex pane, claim a different throwaway bead and run `bin/br-claim` then (after a token commit + push) `bin/br-complete`. Same assertion: most recent Complete mail's `from` is `${CODEX_AGENT}` and `to` contains `${CLAUDE_AGENT}`.

6. **Tear down**. `release_file_reservations` from both panes; close both throwaway beads with `br close`.

If steps 4 and 5 both pass: universal coverage is proven and the handoff is accepted. If only one direction works: the unpatched skill template is the cause — re-check §12.5 sanity grep, then re-check whichever of §12 / §12-bis was incomplete.

**If you only have one CLI available** (Claude OR Codex, but not both): spawn two sessions of whichever you have, AND manually walk through the Codex (or Claude) skill template by `cat`'ing the generated kickoff and confirming the handshake block is present. Document the partial validation in your wrap-up message and request the operator complete the cross-CLI test before declaring the handoff accepted.

---

## 14. Acceptance criteria

All of the following must hold before this handoff is considered complete:

**Wrappers (repo-side):**
- [ ] `bin/br-claim` no longer contains `recipients = peers if peers else [agent]` (was at ~line 419).
- [ ] `bin/br-complete` no longer contains `recipients = peers if peers else [agent]` (was at ~line 395).
- [ ] Both wrappers contain a top-of-file `from datetime import datetime, timedelta, timezone` import.
- [ ] Both wrappers contain the `class NoPeersError(RuntimeError)` declaration and the `discover_peers(...)` helper.
- [ ] Both wrappers expose `--solo`, `--peer-max-idle-seconds`, `--peer-same-program` CLI flags and honour `AGENT_MAIL_ALLOW_SOLO=1`.
- [ ] Both wrappers' top-of-file docstrings reference `discover_peers` in step 5 (br-claim) / step 6 (br-complete) instead of the old `list_contacts → peers + self` wording.
- [ ] All §13.2 grep counts match expected values.

**Tests:**
- [ ] `bin/tests/test_peer_discovery.py` exists; T1–T6 pass against a live `mcp-agent-mail` server; the entire class is skipped (not failed) if the server is not reachable.

**Constitutional (repo-side):**
- [ ] `AGENTS.md` Rule 2 (immediately after the line beginning "You MUST NOT use raw `br update`...") has the bootstrap-handshake addendum paragraph.
- [ ] `AGENTS.md` MCP Agent Mail "Same Repository Workflow" step 1 is rewritten to include `set_contact_policy` + the handshake loop.
- [ ] `AGENTS.md` "Common Pitfalls" has the new "Empty `list_contacts` after `set_contact_policy(policy=\"open\")`" bullet.

**Documentation (repo-side):**
- [ ] `docs/layermaxxer/active/execution-kickoff.md` §5.1 has the bootstrap handshake sub-step inserted between the `macro_start_session` block (ends ~line 191) and the existing "Check your inbox" line (~line 193).
- [ ] `docs/layermaxxer/active/execution-push.md` §2.1 has the refresh-handshake sub-step inserted after the `macro_start_session(...)` block.

**Claude-side user skill (Change 7):**
- [ ] `~/.claude/skills/beads-to-execution/TEMPLATES.md` has a new `### 2a-bis. Bootstrap Contact Handshakes (REQUIRED before any \`bin/br-claim\`)` sub-section inserted between §2a "Boot Your Session" and §2b "Check What's Happening".
- [ ] If a Quick Reference block exists in the Claude tree (§12.3): it contains the "Refresh handshakes on swarm growth" paragraph.
- [ ] §12.4 sibling-skill grep returned, and every matching sibling skill in `~/.claude/skills/` had the handshake block mirrored.

**Codex-side user skill (Change 8 — universal-coverage gate):**
- [ ] `~/.codex/skills/beads-to-execution/TEMPLATES.md` has the same `### 2a-bis. Bootstrap Contact Handshakes ...` insertion (§12-bis.1, §12-bis.2).
- [ ] §12-bis.3 sibling-skill grep returned, and every matching sibling skill in `~/.codex/skills/` had the handshake block mirrored.
- [ ] §12-bis.5 cross-CLI sanity check (`grep -c 'Bootstrap Contact Handshakes' ~/.claude/skills/.../TEMPLATES.md ~/.codex/skills/.../TEMPLATES.md`) returned `1` for BOTH files.

**install-bead-wrappers asset propagation (Changes 11-16 — cross-project propagation gate):**
- [ ] `~/.claude/skills/install-bead-wrappers/assets/br-claim` byte-identical with the fixed `bin/br-claim` (§12-ter.2 final diff).
- [ ] `~/.claude/skills/install-bead-wrappers/assets/br-complete` byte-identical with the fixed `bin/br-complete`.
- [ ] `~/.codex/skills/install-bead-wrappers/assets/br-claim` byte-identical with the Claude-tree copy AND with the fixed `bin/br-claim`.
- [ ] `~/.codex/skills/install-bead-wrappers/assets/br-complete` byte-identical with the Claude-tree copy AND with the fixed `bin/br-complete`.
- [ ] Both copies of `agents-md-rule-2.md` contain the new bootstrap-handshake addendum (`grep -c 'bootstrap contact handshake' ...` returns 1 for each).
- [ ] Both copies of `METHODOLOGY.md` updated to reflect new wrapper line counts (housekeeping).
- [ ] §12-ter.5 end-to-end portability test passed: a fresh `git init` + `install-bead-wrappers` produces wrappers and Rule 2 carrying the fix.

**Live validation:**
- [ ] §13.5 cross-CLI swarm validation passed: a Claude session's `bin/br-claim` produced a Start mail addressed `to: [<codex-agent>]`, AND a Codex session's `bin/br-complete` produced a Complete mail addressed `to: [<claude-agent>]`. Both directions, not just one.

**Wrap-up:**
- [ ] One repo-side commit lands all six repo-side artifacts (bin/br-claim, bin/br-complete, bin/tests/test_peer_discovery.py, AGENTS.md, docs/layermaxxer/active/execution-kickoff.md, docs/layermaxxer/active/execution-push.md) with the §2 commit message format (references both this handoff AND the predecessor error manifest).
- [ ] `MEMORY.md` index entry under "Open Handoffs (post-L5)" is updated to "**LANDED YYYY-MM-DD**" with the commit SHA.
- [ ] Operator notified via AgentMail (reply in any handoff thread, or fresh mail to the operator's current session) that the wrapper protocol changed and existing sessions should refresh.

---

## 15. Risk / rollback

### 15.1 Risk: a future single-agent session breaks under the new "loud error" behaviour

Mitigation: `--solo` and `AGENT_MAIL_ALLOW_SOLO=1` are documented in `--help` output AND in the `NoPeersError` diagnostic text. Solo mode still works; it just demands explicit opt-in. Document this in the commit message.

### 15.2 Risk: the wrapper-level helper diverges between `bin/br-claim` and `bin/br-complete` over time

Mitigation: this handoff lands the helper as a verbatim duplicate (matching the project's existing pattern for `mcp_call`, `preflight_health`, etc.). If divergence becomes a real problem (rather than a theoretical one), file a separate bead to factor out `bin/_br_lib.py`. Do NOT pre-extract — premature abstraction has cost and the duplication is currently small.

### 15.3 Risk: the global `~/.claude/skills/...` AND `~/.codex/skills/...` edits affect other projects mid-iteration

Mitigation: BOTH edits are purely additive — they insert a handshake step between two existing steps. Generated kickoffs for projects that don't have multi-agent swarms simply do an extra `list_agents` call that returns `[]` and a no-op loop. Cost: one extra MCP call per session start. Benefit: bug is impossible by construction in every future project, regardless of whether the operator spawns a Claude or Codex session.

### 15.4 Rollback

If any wrapper change destabilises a future swarm:

```bash
git revert <commit-sha>     # reverts the wrappers and AGENTS.md addendum
git push
```

The kickoff/push template edits (`docs/layermaxxer/...`) are documentation-only and harmless to keep. The `~/.claude/skills/...` AND `~/.codex/skills/...` edits are operator-side; revert manually by re-deleting the handshake block from each template. Revert BOTH together — leaving one CLI patched and the other unpatched gives the false impression that universal coverage holds when it does not.

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

### User-level (Claude tree)
- `~/.claude/skills/beads-to-execution/TEMPLATES.md` — Claude-side template (§12 edit; insertion at §2a/§2b boundary, line ~175)
- `~/.claude/skills/beads-to-execution/SKILL.md` — the skill definition that consumes TEMPLATES.md
- 12 sibling skill files in `~/.claude/skills/` carrying AgentMail patterns — see the full list in §12.4 (covers bead-completion-audit, bead-plan-audit, beads-from-sources, featuremaxxer, layermaxxer, pipemaxxer, planmaxer, install-bead-wrappers)
- **`~/.claude/skills/install-bead-wrappers/assets/{br-claim, br-complete, agents-md-rule-2.md}`** — the canonical wrapper + Rule-2 source used to bootstrap every new project (§12-ter)
- `~/.claude/skills/install-bead-wrappers/METHODOLOGY.md` — documents the manual sync command (§12-ter.4 housekeeping target)

### User-level (Codex tree — universal-coverage mirror)
- `~/.codex/skills/beads-to-execution/TEMPLATES.md` — Codex-side template (§12-bis edit; mirror of §12)
- 12 sibling skill files in `~/.codex/skills/` mirroring the Claude tree — see the full list in §12-bis.3
- **`~/.codex/skills/install-bead-wrappers/assets/{br-claim, br-complete, agents-md-rule-2.md}`** — mirror of §12-ter Claude-side targets; byte-identical with the Claude tree's copies today and post-fix
- `~/.codex/skills/install-bead-wrappers/METHODOLOGY.md` — mirror of §12-ter.4 housekeeping
- `~/.codex/vendor_imports/skills/` — vendor-imported skills; check via §12-bis.3 grep
- `~/.codex/AGENTS.md` — empty on this machine; Codex CLI reads the **project-level** `AGENTS.md`, so §9's edits cover both CLIs

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
- Land steps 5.1 → 5.9 in order. Do not skip the feature-flag step (5.1) — it is the only safety net if the helper is wrong. Do not skip step 5.7 (install-bead-wrappers assets) — it is the only mechanism that propagates the fix to NEW projects.
- The wrappers' existing duplication pattern is intentional (per `mcp_call`, `preflight_health`, etc.). Do not pre-extract a shared lib.
- The wrappers' existing tone (`die`, `warn`, plain stderr) is intentional. Match it.
- `AGENTS.md` is constitutional. Do not casually re-format it; restrict your edits to the named insertions in §9.
- The `~/.claude/` AND `~/.codex/` edits are both operator-side. Neither directory is typically under git on this machine; check `~/.claude/.git` and `~/.codex/.git` before deciding how to commit. If unversioned (likely), edit in-place and note the change in your wrap-up message. **You MUST apply both — the §12 (Claude) edit alone gives partial coverage, the §12-bis (Codex) edit alone gives partial coverage, the §12-ter (install-bead-wrappers assets) edit covers cross-project propagation, and any combination shy of all three leaves a known coordination liability.** A partial fix is worse than no fix because it gives the false impression of universal coverage; future swarms that mix CLIs (or future projects bootstrapped from `install-bead-wrappers`) will silently revert to self-addressing whenever the unpatched path spawns first.
- After landing, send a `[handoff-done] agent-mail handshake bootstrap fix landed` mail to the operator's session (or whoever's current `program=l6-*-executor` is most recently active) so they know the wrapper protocol changed and to refresh their session.
- Update `MEMORY.md` per §14 — index entry only, one line under ~200 chars.

### If you get stuck

This handoff was designed to be self-contained, but reality has more edges than any plan. Concrete escalation paths in priority order:

1. **Conflict between this handoff and the predecessor manifest** (`docs/handoff-2026-05-19-agentmail-wrapper-self-addressing.md`): the manifest wins. Call out the conflict in your commit message and proceed conservatively.
2. **Conflict between this handoff and `AGENTS.md`**: AGENTS.md wins. The constitutional rules (Rule 1 no file deletion; Rule 2 wrapper discipline) override anything else.
3. **A §13 verification gate fails AND you can't identify why**: do NOT push the wrapper change. Send `[handoff-block] §13.<N> failed: <symptom>` to the operator's inbox via AgentMail, with a copy of the failing command + output. Wait for guidance.
4. **An MCP tool returns a shape you don't expect**: `_normalize_peer_list` (§6.1) is defensive about multiple response shapes — extend it with another `("agents", "result", ...)` key if needed. Add a comment naming the new shape and which server version surfaced it.
5. **`install-bead-wrappers` slash command not available in your environment**: use the manual `cp + cat >>` fallback documented in §12-ter.5 step 2.
6. **Genuine ambiguity or unforeseen edge case**: do NOT silently guess. Send `[handoff-question] <topic>` to the operator's inbox, propose two paths with tradeoffs, and wait. The operator answers fast; silent guesses produce silent bugs.
7. **You discover the wrappers were ALREADY partially fixed** (e.g. a prior session landed §6 but not §7): finish what's missing rather than reverting. Use the §13.2 grep checks to identify exactly what's in place vs. missing. The fix is idempotent — applying a missing piece on top of an already-fixed wrapper is safe.

---

## Appendix A — Operator's L5 manual workaround (verbatim, 2026-05-19)

For reference and as a fallback recipe if any incident requires manual onboarding of an active swarm before the wrapper fix has propagated to every machine, the operator's exact text sent to the L5 swarm on 2026-05-19:

> Handshake bootstrap (one-time, per session start): Agent Mail's `list_contacts(...)` returns explicit handshake links — without them, `bin/br-claim` and `bin/br-complete` silently address Start/Complete mail to yourself instead of your peers. Before claiming your first bead:
>
> 1. Call `list_agents(project_key="/Users/alinaaitova/Desktop/Testproject1")` and note the peers whose program matches yours and whose `last_active_ts` is within the last hour.
> 2. For each peer, call `macro_contact_handshake(project_key="/Users/alinaaitova/Desktop/Testproject1", requester=<your-name>, target=<peer-name>)`.
>
> Confirm by calling `list_contacts(project_key=..., agent_name=<your-name>)` — it should now return each peer. Then proceed with your normal kickoff work loop.

Notable design signals encoded by this message that the wrapper helper (§6.1 `discover_peers`) inherits as defaults:

- **Same-program filter is the default**, not an opt-in. The wrapper auto-derives the caller's program from their own `list_agents` entry and applies the filter automatically (`same_program=_AUTO_PROGRAM` sentinel; pass `--peer-same-program=*` to disable).
- **1-hour idle cutoff is the default** (`max_idle_seconds=3600`).
- **Verification via `list_contacts` read-back** is now part of every generated kickoff's handshake block (§10, §11, §12.2, §12-bis), matching the operator's "Confirm by calling list_contacts" instruction.

If a future incident requires a similar emergency manual onboarding before the wrapper fix is in place on a given machine, this verbatim text is the recommended message to send — it's known-good, operator-vetted, and aligns with the wrapper's automatic semantics so the manual workaround and the automatic behaviour are interchangeable.
