# L6 Request Pipeline Automation — Step 11: Push Prompt (Continuation Nudge)

> This step is a **documentation note**, not a separate handoff for an agent to execute.
> The push prompt (`l6-request-pipeline-automation-push.md`) is generated AS PART OF
> step 10's `/beads-to-execution` output. This file documents WHEN and HOW the push
> prompt is used by agents during L6 execution.

---

## When to Use the Push Prompt

The **kickoff** (from step 10) is for agents starting L6 work FRESH (first session,
no prior context). It's long (~400-700 lines) and onboards from zero.

The **push** is for agents who are CONTINUING L6 work (fresh shell, fresh context
window, but L6 has been in progress for a while). It's compact (~120-200 lines) and
re-orients without re-explaining the project.

Use cases:
- A previous L6 session was interrupted (context compaction, agent crash, operator
  stopped the session) and a new agent must pick up where it left off
- An agent is finishing one bead and needs a fresh prompt before claiming the next
- A new agent joins the swarm mid-execution and needs to ramp up quickly without the
  full kickoff
- Parallel agents need fresh-start prompts every time the bead they just completed
  pushed them into a new phase

---

## How to Use the Push Prompt

For any agent continuing L6 work:

1. Open `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-push.md`
2. Read it FULLY (compact — should take 5 minutes)
3. Run AgentMail setup if not already registered in this shell:
   ```
   ensure_project(project_key="/Users/alinaaitova/Desktop/Testproject1")
   register_agent(project_key, program="claude-code", model="opus")
   set_contact_policy(project_key, agent_name, policy="open")
   # Bootstrap handshakes (per AGENTS.md Rule 2)
   peers = list_agents(project_key)
   for peer in peers:
       if peer['program'] == 'claude-code' and peer['active_in_last_hour'] and peer['name'] != agent_name:
           macro_contact_handshake(project_key, requester=agent_name, target=peer['name'])
   fetch_inbox(project_key, agent_name, limit=10)
   ```
4. Run `br ready --label l6 --json` to see ready beads
5. Pick highest-priority ready bead
6. `bin/br-claim <bead-id>`
7. Implement → verify → commit + push → `bin/br-complete <bead-id>` → loop

The push prompt is short because the work loop is the same as kickoff §7 — just
restate it without the project context.

---

## What the Push Prompt Contains (from step 10's `/beads-to-execution` output)

The push prompt at `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-push.md` has 9 sections (per step 10 requirements):

1. **Re-Orient** — Project name, spec/strategy paths, bead graph command
2. **Core Contracts** — 8 invariants + 5 recorded decisions (compact)
3. **Bead Completion Policy** — Use wrappers, fully complete, escalate via mail only
4. **Work Loop** — Compact 9-step loop (check inbox → ready → claim → implement → verify → commit → push → complete → loop)
5. **Verification Commands** — Full cargo + bun + e2e sequence
6. **Dependency Spine** — Phase 1 → 2 → 3 → (4 ∥ 5) → 6 → 7
7. **File Surfaces** — Phase × Key Files (compact table)
8. **AgentMail Quick Reference** — claim/complete via wrappers; ad-hoc via send_message with explicit peer list (NEVER "broadcast")
9. **CDP Browser Discipline** — Attach to persistent profile; never fresh; never log out

---

## Push Prompt Quality Reference

```
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-push.md      # 124 lines — gold standard for compact push
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-push.md  # L5 push reference
```

---

## Common Push Prompt Pitfalls (the kickoff/push generator MUST avoid)

1. **Re-explaining the project narrative** — push is for agents who already know L6 exists and what it does. Don't repeat the layer narrative; just re-orient with paths.
2. **Listing every bead** — `br ready --label l6 --json` is the dynamic source. The push references the COMMAND, not a hardcoded bead list.
3. **Repeating the AgentMail setup verbatim** — push includes a COMPACT version that assumes the agent has done this kind of setup before. The kickoff has the full walkthrough.
4. **Forgetting CDP discipline** — agents continuing into Phase 4/7 work MUST respect persistent-profile + no-logout rules. Push includes this.
5. **Forgetting the operator-action gate for Phase 7** — if a continuation agent reaches Phase 7 live demo, they MUST NOT escalate to FORMAL_SUBMISSION without operator authorization. Push explicitly says so.

---

## Push Prompt NOT For

- New agents who have never seen this project → use the kickoff
- Operators reviewing pipeline state → use the status YAML
- Pipeline orchestrators (pipemaxxer, planmaxer) → use the strategy + manifest

---

## Output

No new files produced by this step. The push prompt is the deliverable from step 10.

This file (11-push.md) exists in the pipeline directory to:
1. Document the usage pattern
2. Provide context for execution agents who wonder "why is there a step 11 if step 10 already produced the push?"
3. Serve as the reference for any agent needing to understand the kickoff vs push distinction

---

## ⛔ MANDATORY STOP — Human Triggers Completion Audit

Step 11 (push usage during execution) ends when ALL L6 task beads have status=closed
(or only `audit-fix` candidates remain). At that point, the human triggers Batch 12
(completion audit). Do NOT auto-trigger.

Check by running:
```bash
br list --label l6 --status open --json | jq '.issues | length'
# When this returns 0 (or only the implementation epic itself), L6 task execution is complete.
```

Then the human invokes `/bead-completion-audit` per step 12.
