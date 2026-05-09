# Update Agent Mail Skill Templates + Install Bead Wrappers Skill

> Paste this entire file into a fresh Codex or Claude Code session. Execute it carefully and fully.

---

## Mission

Bring the reusable skill templates in this machine back into alignment with the Agent Mail discipline that is already correct in `Testproject1`.

There are five deliverables:

1. Update the Codex `pipemaxxer` templates so generated kickoff/push prompts use `bin/br-claim` and `bin/br-complete`, and never imply an Agent Mail `broadcast` recipient.
2. Update the Codex `layermaxxer` skill/templates so generated execution prompts use the same wrapper discipline and explicit `list_contacts(...)` peer discovery.
3. Update the Codex `beads-to-execution` templates because this is the lower-level generator that still contains the old manual `br update` / `file_reservation_paths` / `send_message` workflow.
4. Mirror the same semantic changes into the matching Claude skill directories so `~/.claude/skills/...` and `~/.codex/skills/...` do not diverge on Agent Mail behavior.
5. Install the missing reusable `install-bead-wrappers` skill under both `~/.codex/skills/install-bead-wrappers/` and `~/.claude/skills/install-bead-wrappers/`.

This handoff is about skill/document maintenance outside normal product code. Do not modify runtime application code.

---

## Current Ground Truth

Project root:

```text
/Users/zacfarrell/Desktop/Testproject1
```

The project `AGENTS.md` is already correct. Do not edit it unless you discover a concrete contradiction with the current repository state and explicitly report it first.

The authoritative Agent Mail pattern is:

- Agent Mail has no `broadcast` command or recipient.
- Discover peers with `list_contacts(project_key, agent_name)`.
- Send messages to an explicit list of agent names returned by `list_contacts(...)`.
- Never use `to="broadcast"` or `to=["broadcast"]`.
- For bead-tracked work, use project wrappers:
  - `bin/br-claim <bead-id>` to claim: reserve files, send `[<id>] Start`, set bead `in_progress`.
  - `bin/br-complete <bead-id>` to complete: verify pushed commit, send `[<id>] Complete`, close bead, sync `.beads/`, release reservations.
- Raw `br update --status=in_progress` and raw `br close` are forbidden for bead-tracked work in this repo because they bypass Agent Mail and file reservations.

The wrappers themselves already implement the right no-broadcast pattern:

```text
/Users/zacfarrell/Desktop/Testproject1/bin/br-claim
/Users/zacfarrell/Desktop/Testproject1/bin/br-complete
```

Both wrappers call `list_contacts(...)`, build an explicit recipient list, and fall back to sending to self only when no peers exist.

---

## Known Current State To Confirm

Confirm this baseline before editing. If it has changed, adapt to the current files but preserve the mission.

```bash
rg -n 'AgentMail|Agent Mail|br update|br close|file_reservation_paths|send_message|broadcast|list_contacts|br-claim|br-complete' \
  /Users/zacfarrell/.codex/skills/pipemaxxer/TEMPLATES.md \
  /Users/zacfarrell/.claude/skills/pipemaxxer/TEMPLATES.md
```

Expected shape at handoff creation:

- Codex `pipemaxxer/TEMPLATES.md` has detailed stale kickoff/fix-bead loops with `file_reservation_paths(...)`, manual Start/Complete `send_message(...)`, and `{bead_cli} complete`.
- Claude `pipemaxxer/TEMPLATES.md` has a shorter older template with the same stale manual claim/reserve/announce flow.
- Codex and Claude `pipemaxxer/TEMPLATES.md` already differ structurally. Do not blindly copy one over the other. Fix the Agent Mail sections in both.

```bash
rg -n 'Coordination|Agent Mail|AgentMail|file reservations|announce|broadcast|list_contacts|br-claim|br-complete|br update|br close' \
  /Users/zacfarrell/.codex/skills/layermaxxer \
  /Users/zacfarrell/.claude/skills/layermaxxer
```

Expected shape at handoff creation:

- `layermaxxer/SKILL.md` contains weak coordination requirements such as "File reservations via Agent Mail if present" and "Close, sync, commit, push, announce, repeat."
- `layermaxxer/references/TEMPLATES.md` contains "Set up Agent Mail, check inbox, reserve files before editing" and "Close bead, sync, commit, push, release reservations, announce completion."
- These need to become wrapper-first instructions when `bin/br-claim` / `bin/br-complete` exist.

```bash
rg -n 'br update <.*status=in_progress|br close <.*reason|file_reservation_paths|send_message|resource://agents|broadcast|br-claim|br-complete|AgentMail|Agent Mail' \
  /Users/zacfarrell/.codex/skills/beads-to-execution \
  /Users/zacfarrell/.claude/skills/beads-to-execution
```

Expected shape at handoff creation:

- `beads-to-execution/TEMPLATES.md` is the main stale file. It still teaches `br update <bead-id> --status=in_progress`, `file_reservation_paths(...)`, manual claim `send_message(...)`, raw `br close`, and manual completion mail.
- `beads-to-execution/SKILL.md`, `CHECKLIST.md`, and `WORKFLOW.md` mention Agent Mail coordination. Only update those if they actively contradict wrapper discipline. The required edit target is `TEMPLATES.md`.

```bash
test -d /Users/zacfarrell/.codex/skills/install-bead-wrappers || echo "codex install-bead-wrappers missing"
test -d /Users/zacfarrell/.claude/skills/install-bead-wrappers || echo "claude install-bead-wrappers missing"
```

Expected shape at handoff creation: both directories are missing.

Do not chase unrelated uses of the word `broadcast` in skills like `idea-to-research` or `research-to-spec`; those are not Agent Mail recipient examples and are out of scope.

---

## Mandatory Reading

Read these in order before editing:

1. `/Users/zacfarrell/Desktop/Testproject1/AGENTS.md`
   - Focus: Rule 1, Rule 2, irreversible actions, main-only branch, Agent Mail, beads workflow.
2. `/Users/zacfarrell/Desktop/Testproject1/prompts/phase-a-mail-discipline-infrastructure-handoff.md`
   - Focus: D6, verification section, and the intent to move generated prompts from manual mail plumbing to wrapper commands.
3. `/Users/zacfarrell/Desktop/Testproject1/prompts/install-bead-wrappers-skill-handoff.md`
   - Focus: deliverable shape for `install-bead-wrappers`.
   - Adjust all old `/Users/alinaaitova/Desktop/Testproject1` examples to the actual current path `/Users/zacfarrell/Desktop/Testproject1`.
4. `/Users/zacfarrell/Desktop/Testproject1/docs/pipeline/l5-expanded-commercial-intelligence/10-generate-kickoff.md`
   - Focus: sections that mention `bin/br-claim`, `bin/br-complete`, `list_contacts`, and no broadcast.
5. `/Users/zacfarrell/Desktop/Testproject1/docs/pipeline/l5-expanded-commercial-intelligence/11-push.md`
   - Focus: compact correct pattern in sections 3, 4, and 8.
6. Current target skill files listed below.

Useful read commands:

```bash
sed -n '1,260p' AGENTS.md
sed -n '210,255p' prompts/phase-a-mail-discipline-infrastructure-handoff.md
sed -n '335,392p' prompts/phase-a-mail-discipline-infrastructure-handoff.md
sed -n '1,190p' prompts/install-bead-wrappers-skill-handoff.md
sed -n '140,225p' docs/pipeline/l5-expanded-commercial-intelligence/10-generate-kickoff.md
sed -n '60,125p' docs/pipeline/l5-expanded-commercial-intelligence/11-push.md
sed -n '330,390p' bin/br-claim
sed -n '312,358p' bin/br-complete
```

---

## Target Files

Edit these existing skill files:

```text
/Users/zacfarrell/.codex/skills/pipemaxxer/TEMPLATES.md
/Users/zacfarrell/.claude/skills/pipemaxxer/TEMPLATES.md

/Users/zacfarrell/.codex/skills/layermaxxer/SKILL.md
/Users/zacfarrell/.codex/skills/layermaxxer/references/TEMPLATES.md
/Users/zacfarrell/.claude/skills/layermaxxer/SKILL.md
/Users/zacfarrell/.claude/skills/layermaxxer/references/TEMPLATES.md

/Users/zacfarrell/.codex/skills/beads-to-execution/TEMPLATES.md
/Users/zacfarrell/.claude/skills/beads-to-execution/TEMPLATES.md
```

Create these missing skill directories and files:

```text
/Users/zacfarrell/.codex/skills/install-bead-wrappers/
/Users/zacfarrell/.codex/skills/install-bead-wrappers/SKILL.md
/Users/zacfarrell/.codex/skills/install-bead-wrappers/METHODOLOGY.md
/Users/zacfarrell/.codex/skills/install-bead-wrappers/CHECKLIST.md
/Users/zacfarrell/.codex/skills/install-bead-wrappers/assets/br-claim
/Users/zacfarrell/.codex/skills/install-bead-wrappers/assets/br-complete
/Users/zacfarrell/.codex/skills/install-bead-wrappers/assets/agents-md-rule-2.md

/Users/zacfarrell/.claude/skills/install-bead-wrappers/
/Users/zacfarrell/.claude/skills/install-bead-wrappers/SKILL.md
/Users/zacfarrell/.claude/skills/install-bead-wrappers/METHODOLOGY.md
/Users/zacfarrell/.claude/skills/install-bead-wrappers/CHECKLIST.md
/Users/zacfarrell/.claude/skills/install-bead-wrappers/assets/br-claim
/Users/zacfarrell/.claude/skills/install-bead-wrappers/assets/br-complete
/Users/zacfarrell/.claude/skills/install-bead-wrappers/assets/agents-md-rule-2.md
```

Do not delete anything.

---

## Editing Rules

- Make manual, reviewable edits. Do not run regex rewrite scripts over skill files.
- Preserve useful feature-specific guidance. Only replace stale workflow plumbing.
- Do not touch runtime project code.
- Do not change `AGENTS.md`, `.beads/`, `.git/hooks/`, or generated project pipeline docs.
- Existing Claude and Codex skill copies may not be byte-identical before you start. Inspect diffs before editing. After your edits, Agent Mail workflow sections should be semantically identical even if unrelated historical template differences remain.
- If you need to create the new `install-bead-wrappers` directory, create only the files listed in this handoff.
- Do not leave placeholder text in newly created skill files. No `[Embed ...]`, `[Rule 2 content here]`, `{PROJECT_ROOT}`, or similar placeholder may remain in `install-bead-wrappers` deliverables except where the skill intentionally documents user-supplied arguments such as `<PROJECT_ROOT>`.
- No file deletion.

---

## Output Quality Bar

The finished handoff execution should make future generated prompts unambiguous to a new agent:

- Claiming a bead means `bin/br-claim <id>`, not manual `br update` plus manual mail.
- Completing a bead means commit, push, then `bin/br-complete <id>`, not manual `br close` plus manual mail.
- Wrapper-handled Start/Complete mail is not duplicated with raw `send_message(...)`.
- Raw `send_message(...)` remains only for blockers, questions, phase announcements, and session handoffs.
- Every raw `send_message(...)` example includes `to=[...]` populated from `list_contacts(project_key, agent_name)`.
- The phrase `broadcast` appears only in negative guidance such as "no broadcast" or "never use `to=\"broadcast\"`."
- The new `install-bead-wrappers` skill can be invoked in a fresh repo without reading this handoff.

---

## Deliverable 1: Update `pipemaxxer`

### Files

```text
/Users/zacfarrell/.codex/skills/pipemaxxer/TEMPLATES.md
/Users/zacfarrell/.claude/skills/pipemaxxer/TEMPLATES.md
```

### What to Find

Search:

```bash
rg -n 'AgentMail|Agent Mail|br update|br close|file_reservation_paths|send_message|broadcast|list_contacts|br-claim|br-complete' \
  /Users/zacfarrell/.codex/skills/pipemaxxer \
  /Users/zacfarrell/.claude/skills/pipemaxxer
```

Known stale shapes include:

```text
CLAIM: {bead_cli} claim + file_reservation_paths + send_message
br update <id> --status=in_progress
file_reservation_paths(...)
send_message(thread_id="<bead-id>", subject="[<bead-id>] Start: <title>")
{bead_cli} close <id>
send_message(... Complete ...)
release_file_reservations(...)
```

### Required Replacement Pattern

Generated kickoff/push prompts must say:

```markdown
### Agent Mail And Bead Claim Discipline

- Use `bin/br-claim <bead-id>` to claim bead-tracked work. The wrapper reserves files, sends `[<bead-id>] Start` mail to peers discovered via `list_contacts`, and sets the bead `in_progress`.
- Use `bin/br-complete <bead-id>` to complete bead-tracked work. The wrapper verifies the pushed bead commit, sends `[<bead-id>] Complete`, closes the bead, syncs `.beads/`, and releases reservations.
- Do NOT use raw `br update --status=in_progress` or raw `br close <id>` for bead-tracked work.
- Agent Mail has NO broadcast recipient. For blocker/question/handoff messages that are not handled by the wrappers, first call `list_contacts(project_key, agent_name)`, then pass the returned names as the explicit `to=[...]` list.
- Never use `to="broadcast"` or `to=["broadcast"]`.
```

The main work loop should look like this:

```markdown
1. Check inbox (`fetch_inbox`) and acknowledge relevant messages.
2. Run `{bead_cli} ready --json` and pick the highest-priority ready non-epic bead in scope.
3. Read the bead fully and study target files.
4. Claim with `bin/br-claim <bead-id>`; use `--paths <p1> <p2>` only if the bead lacks a parseable file list.
5. Implement the bead fully. No partial work and no mid-way commits of incomplete work.
6. Run all required quality gates.
7. Commit with `git commit -m "<bead-id>: <short description>"`, then `git pull --rebase && git push`.
8. Complete with `bin/br-complete <bead-id>`.
9. Loop back to inbox and ready-work selection.
```

For fix-bead templates, use the same structure with `<fix-bead-id>`.

### Completion Criteria

- `pipemaxxer` no longer instructs agents to claim bead-tracked work with raw `br update` plus manual Start mail.
- `pipemaxxer` no longer instructs agents to complete bead-tracked work with raw `br close` plus manual Complete mail.
- Any manual `send_message(...)` guidance is limited to blockers, questions, session handoffs, or non-bead announcements and includes `list_contacts(...)` first.
- The text explicitly says no Agent Mail broadcast exists.
- Codex and Claude copies may still differ in unrelated template structure, but every Agent Mail / wrapper section must communicate the same behavior.

---

## Deliverable 2: Update `layermaxxer`

### Files

```text
/Users/zacfarrell/.codex/skills/layermaxxer/SKILL.md
/Users/zacfarrell/.codex/skills/layermaxxer/references/TEMPLATES.md
/Users/zacfarrell/.claude/skills/layermaxxer/SKILL.md
/Users/zacfarrell/.claude/skills/layermaxxer/references/TEMPLATES.md
```

### What to Find

Search:

```bash
rg -n 'Coordination|Agent Mail|AgentMail|file reservations|announce|broadcast|list_contacts|br-claim|br-complete|br update|br close' \
  /Users/zacfarrell/.codex/skills/layermaxxer \
  /Users/zacfarrell/.claude/skills/layermaxxer
```

Known weak wording:

```text
File reservations via Agent Mail if present.
Announce claims if other agents are active.
Set up Agent Mail, check inbox, reserve files before editing.
```

### Required Replacement Pattern

Replace vague coordination instructions with:

```markdown
5. COORDINATION (if multi-agent or bead-tracked)
   - Read the project `AGENTS.md` first and follow its bead discipline.
   - If the project has `bin/br-claim` and `bin/br-complete`, use them as the only claim/complete path for bead-tracked work.
   - `bin/br-claim <bead-id>` handles file reservation, Start mail, and status update.
   - `bin/br-complete <bead-id>` handles Complete mail, bead close, `.beads/` sync, push, and reservation release.
   - Do not generate raw `br update --status=in_progress` / raw `br close` instructions when wrappers exist.
   - Agent Mail has NO broadcast recipient. For any manual mail, discover peers with `list_contacts(project_key, agent_name)` and send to the explicit returned names.
```

The execution push section should include the same compact loop as the `pipemaxxer` replacement above, adjusted for compact-doc execution if necessary.

### Completion Criteria

- `layermaxxer` generated prompts no longer leave Agent Mail as vague optional coordination.
- Generated prompts tell agents exactly how to claim/complete with wrappers when wrappers exist.
- Generated prompts explicitly say no broadcast and require `list_contacts(...)` for manual mail.

---

## Deliverable 3: Update `beads-to-execution`

### Files

```text
/Users/zacfarrell/.codex/skills/beads-to-execution/TEMPLATES.md
/Users/zacfarrell/.claude/skills/beads-to-execution/TEMPLATES.md
```

### Why This Matters

`pipemaxxer` delegates kickoff/push generation to `beads-to-execution`. If `pipemaxxer` is fixed but `beads-to-execution` still contains raw claim/complete templates, future generated prompts can regress.

### What to Find

Search:

```bash
rg -n 'br update <.*status=in_progress|br close <.*reason|file_reservation_paths|send_message|resource://agents|broadcast|br-claim|br-complete' \
  /Users/zacfarrell/.codex/skills/beads-to-execution \
  /Users/zacfarrell/.claude/skills/beads-to-execution
```

Known stale sections include:

```text
Step 2c. Claim Your Bead (Announce + Reserve)
br update <bead-id> --status=in_progress
file_reservation_paths(...)
send_message(...)

Step 4. Work Loop
br update <bead-id> --status=in_progress
file_reservation_paths(...)
send_message(...)
br close <bead-id> --reason ...
br sync --flush-only
release_file_reservations(...)
send_message(... Completed ...)
```

### Required Replacement Pattern

Replace "Claim Your Bead (Announce + Reserve)" with wrapper-first text:

````markdown
### 2c. Claim Your Bead

Use the project wrapper:

```bash
bin/br-claim <bead-id>
```

This is the only authorized claim path when the wrapper exists. It reserves the bead's files, sends `[<bead-id>] Start` mail to peers discovered via `list_contacts`, and sets the bead `in_progress`.

If the bead body does not include parseable file paths:

```bash
bin/br-claim <bead-id> --paths <path-1> <path-2>
```

If the wrapper exits non-zero, the bead is not claimed. Fix the underlying cause: file reservation conflict, agent-mail server unavailable, `AGENT_MAIL_NAME` unset, missing bead file list, or another agent already claimed it. Do not bypass with raw `br update`.
````

Replace completion sections with:

````markdown
### Complete The Bead

After implementation and verification:

```bash
git add <specific-files>
git commit -m "<bead-id>: <short description>"
git pull --rebase
git push
bin/br-complete <bead-id>
```

`bin/br-complete` verifies your Start mail and pushed commit, sends `[<bead-id>] Complete`, closes the bead, syncs `.beads/`, pushes bead metadata, and releases reservations.

Do not bypass with raw `br close`.
````

Keep inbox polling guidance. Keep manual `send_message` patterns only for blockers, questions, phase announcements, and session handoffs. Those patterns must include explicit peer discovery:

```markdown
peers = list_contacts(project_key="<project>", agent_name="<your-agent>")
send_message(..., to=peers, ...)
```

Also include:

```markdown
Agent Mail has NO broadcast recipient. Never use `to="broadcast"` or `to=["broadcast"]`.
```

### Completion Criteria

- `beads-to-execution` generated kickoff and push prompts use wrappers for claim/complete.
- Raw `br update --status=in_progress` / raw `br close` remains only as forbidden anti-pattern text, if present at all.
- Manual mail examples discover peers via `list_contacts(...)`.
- No `broadcast` recipient remains.
- The older "first/only agent can skip send_message but reserve files manually" guidance is removed or rewritten: when wrappers exist, the wrapper handles Start mail and self-recipient fallback.

---

## Deliverable 4: Claude/Codex Parity

After the edits above:

1. Compare Codex and Claude copies for each touched skill.
2. Do not blindly overwrite if they had unrelated historical differences.
3. Ensure Agent Mail / wrapper sections are semantically the same.
4. Where files are intended to be identical, make them identical.

Commands:

```bash
diff -u /Users/zacfarrell/.claude/skills/layermaxxer/SKILL.md \
        /Users/zacfarrell/.codex/skills/layermaxxer/SKILL.md

diff -u /Users/zacfarrell/.claude/skills/layermaxxer/references/TEMPLATES.md \
        /Users/zacfarrell/.codex/skills/layermaxxer/references/TEMPLATES.md

diff -u /Users/zacfarrell/.claude/skills/beads-to-execution/TEMPLATES.md \
        /Users/zacfarrell/.codex/skills/beads-to-execution/TEMPLATES.md

diff -u /Users/zacfarrell/.claude/skills/pipemaxxer/TEMPLATES.md \
        /Users/zacfarrell/.codex/skills/pipemaxxer/TEMPLATES.md
```

Known note: `pipemaxxer/TEMPLATES.md` may already differ between Claude and Codex before this task. If so, preserve unrelated differences but make the Agent Mail sections equivalent.

---

## Deliverable 5: Install `install-bead-wrappers` Skill

### Create Both Skill Trees

Create:

```text
/Users/zacfarrell/.codex/skills/install-bead-wrappers/
/Users/zacfarrell/.claude/skills/install-bead-wrappers/
```

Each must contain:

```text
SKILL.md
METHODOLOGY.md
CHECKLIST.md
assets/br-claim
assets/br-complete
assets/agents-md-rule-2.md
```

### Asset Sources

Copy these wrapper assets verbatim:

```text
/Users/zacfarrell/Desktop/Testproject1/bin/br-claim
/Users/zacfarrell/Desktop/Testproject1/bin/br-complete
```

The skill assets should be byte-identical to the project wrappers.

Extract Rule 2 from:

```text
/Users/zacfarrell/Desktop/Testproject1/AGENTS.md
```

Wrap the extracted Rule 2 asset in markers:

```markdown
<!-- BEGIN: AGENTS.md Rule 2 (managed by install-bead-wrappers skill) -->
## RULE NUMBER 2: BEAD CLAIM DISCIPLINE — USE THE WRAPPERS

For any bead-tracked work in this repo, you MUST use the project wrappers to claim and complete beads:

- **`bin/br-claim <bead-id>`** to start work — atomically reserves files, sends `[<bead-id>] Start` mail to peers, and sets the bead `in_progress`.
- **`bin/br-complete <bead-id>`** to finish work — atomically sends `[<bead-id>] Complete` mail, closes the bead, syncs `.beads/`, and releases file reservations.

You MUST NOT use raw `br update --status=in_progress` or `br close <id>` directly — they bypass Agent Mail and file reservations, which leaves the swarm unable to detect collisions. The pre-commit hook at `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` enforces file-reservation conflicts today; the Phase B extension to that hook (when active) will additionally block any commit with a `<bead-id>:` message prefix that has no corresponding `[<bead-id>] Start` mail in the archive.

If a wrapper errors, fix the underlying problem (agent-mail server down, file reservation conflict, missing `AGENT_MAIL_NAME` env var). Never bypass.

This is a constitutional rule, equal in force to Rule 1 (no file deletion). It applies to every layer (L0 through L6 and beyond) and every future stage.
<!-- END: AGENTS.md Rule 2 (managed by install-bead-wrappers skill) -->
```

Do not include Rule 1 or the irreversible-actions section in this asset.

### `SKILL.md` Required Frontmatter

```yaml
---
name: install-bead-wrappers
description: Bootstrap mail-discipline infrastructure in any git project. Copies bin/br-claim and bin/br-complete wrappers, appends AGENTS.md Rule 2, installs the agent-mail precommit hook via `am guard install`. Idempotent and safe to re-run. Use when the user says install bead wrappers, bootstrap mail discipline, or set up agent-mail enforcement.
---
```

### `SKILL.md` Required Sections

Include these sections:

```markdown
# Install Bead Wrappers

## When To Use
## Prerequisites
## What This Installs
## Workflow
## Idempotency
## Troubleshooting
## Mandatory Stop
```

The workflow must instruct the invoking agent to:

1. Find the target repo root with `git rev-parse --show-toplevel`.
2. Verify `am` exists and inspect `am guard install --help`.
3. Verify the agent-mail HTTP server health endpoint if available.
4. Create `<repo>/bin/` if needed.
5. Copy bundled `assets/br-claim` and `assets/br-complete` into `<repo>/bin/`.
6. `chmod +x <repo>/bin/br-claim <repo>/bin/br-complete`.
7. Create or update `<repo>/AGENTS.md` by appending `assets/agents-md-rule-2.md` only if the marker is absent.
8. Install the hook with:

```bash
am guard install <PROJECT_NAME> <PROJECT_ROOT>
```

Use the actual project name and root path. Do not use `am install-guard`; that command is not the verified surface.

9. Smoke-test:

```bash
<PROJECT_ROOT>/bin/br-claim --help
<PROJECT_ROOT>/bin/br-complete --help
am guard status
```

10. Report exactly what was installed and stop. Do not claim beads or edit product code.

Also include these implementation details:

- Export both `AGENT_NAME` and `AGENT_MAIL_NAME` when smoke-testing wrapper behavior. The wrappers prefer `AGENT_MAIL_NAME` and fall back to `AGENT_NAME`.
- The skill should use the target repository root discovered at invocation time. Do not hardcode `/Users/zacfarrell/Desktop/Testproject1` into the install workflow except in `METHODOLOGY.md` as the snapshot source for bundled assets.
- The skill should be safe to run repeatedly in the same target repo. It may overwrite wrapper files from its bundled assets, but it must not append Rule 2 more than once.
- The skill must explicitly stop after infrastructure installation. It must not claim beads, run project quality gates, or perform unrelated cleanup.

### `METHODOLOGY.md` Required Content

Explain:

- Why the wrappers exist: atomic claim/complete, reservations, Start/Complete mail, no raw bead status transitions.
- Why Agent Mail has no broadcast and how wrappers use `list_contacts(...)`.
- Why `am guard install` is used rather than bundling hook source.
- How wrapper assets are snapshotted from `Testproject1`.
- How to refresh the skill later: update project wrappers first, then copy new wrapper assets into both skill trees and update any snapshot notes.

Include a snapshot stamp:

```bash
git -C /Users/zacfarrell/Desktop/Testproject1 log -1 --format='%h %ci' -- bin/br-claim bin/br-complete
```

Record the returned commit/date in `METHODOLOGY.md` so a future refresh can tell whether the skill assets are stale.

### `CHECKLIST.md` Required Content

Include a checklist covering:

- Skill exists in both Claude and Codex locations.
- Both skill trees contain the six required files.
- Wrapper assets are byte-identical to `Testproject1/bin/br-claim` and `Testproject1/bin/br-complete`.
- `agents-md-rule-2.md` contains BEGIN and END markers.
- `SKILL.md` frontmatter has `name` and `description`.
- `am guard install --help` is documented, not `am install-guard`.
- Install workflow is idempotent.
- Skill stops after installation.
- No placeholder text remains in any deliverable.

### Parity Requirement

The `install-bead-wrappers` trees should be byte-identical across Claude and Codex:

```bash
diff -r /Users/zacfarrell/.claude/skills/install-bead-wrappers \
        /Users/zacfarrell/.codex/skills/install-bead-wrappers
```

This diff should be empty.

---

## Verification

Run these after edits:

```bash
# 1. No stale Agent Mail broadcast recipient in touched skills
if rg -n "to=\"broadcast\"|to=\\[\"broadcast\"\\]|to=\\['broadcast'\\]" \
  /Users/zacfarrell/.codex/skills/pipemaxxer \
  /Users/zacfarrell/.codex/skills/layermaxxer \
  /Users/zacfarrell/.codex/skills/beads-to-execution \
  /Users/zacfarrell/.claude/skills/pipemaxxer \
  /Users/zacfarrell/.claude/skills/layermaxxer \
  /Users/zacfarrell/.claude/skills/beads-to-execution; then
  echo "FAIL: stale broadcast recipient remains"
  exit 1
else
  echo "OK: no broadcast recipient examples remain"
fi
```

```bash
# 2. Wrapper commands appear in all relevant generators
for skill in pipemaxxer layermaxxer beads-to-execution install-bead-wrappers; do
  echo "=== $skill ==="
  rg -n 'bin/br-claim|bin/br-complete|list_contacts|NO broadcast|no broadcast|Never use `to="broadcast"`|NEVER `to="broadcast"`' \
    /Users/zacfarrell/.codex/skills/$skill \
    /Users/zacfarrell/.claude/skills/$skill
done
```

```bash
# 3. Raw claim/close patterns are gone or only appear as forbidden anti-patterns
rg -n 'br update <.*status=in_progress|br close <.*reason|file_reservation_paths|send_message\(' \
  /Users/zacfarrell/.codex/skills/pipemaxxer \
  /Users/zacfarrell/.codex/skills/layermaxxer \
  /Users/zacfarrell/.codex/skills/beads-to-execution \
  /Users/zacfarrell/.claude/skills/pipemaxxer \
  /Users/zacfarrell/.claude/skills/layermaxxer \
  /Users/zacfarrell/.claude/skills/beads-to-execution
```

Expected: either no stale claim/complete workflow remains, or any remaining mentions are clearly framed as forbidden anti-patterns or non-claim manual mail for blockers/questions/handoffs with `list_contacts(...)`.

```bash
# 4. install-bead-wrappers is installed in both CLIs
test -f /Users/zacfarrell/.codex/skills/install-bead-wrappers/SKILL.md
test -f /Users/zacfarrell/.claude/skills/install-bead-wrappers/SKILL.md
test -f /Users/zacfarrell/.codex/skills/install-bead-wrappers/assets/br-claim
test -f /Users/zacfarrell/.claude/skills/install-bead-wrappers/assets/br-claim
```

```bash
# 4b. install-bead-wrappers has no unreplaced placeholders
if rg -n '\[Embed|\[Rule 2 content here\]|\{PROJECT_ROOT\}|\{PROJECT_NAME\}' \
  /Users/zacfarrell/.codex/skills/install-bead-wrappers \
  /Users/zacfarrell/.claude/skills/install-bead-wrappers; then
  echo "FAIL: placeholder text remains in install-bead-wrappers"
  exit 1
else
  echo "OK: no placeholder text remains in install-bead-wrappers"
fi
```

```bash
# 5. Wrapper assets match project wrappers
diff /Users/zacfarrell/.codex/skills/install-bead-wrappers/assets/br-claim \
     /Users/zacfarrell/Desktop/Testproject1/bin/br-claim
diff /Users/zacfarrell/.codex/skills/install-bead-wrappers/assets/br-complete \
     /Users/zacfarrell/Desktop/Testproject1/bin/br-complete
diff /Users/zacfarrell/.claude/skills/install-bead-wrappers/assets/br-claim \
     /Users/zacfarrell/Desktop/Testproject1/bin/br-claim
diff /Users/zacfarrell/.claude/skills/install-bead-wrappers/assets/br-complete \
     /Users/zacfarrell/Desktop/Testproject1/bin/br-complete
```

```bash
# 6. install-bead-wrappers dual-CLI parity
diff -r /Users/zacfarrell/.claude/skills/install-bead-wrappers \
        /Users/zacfarrell/.codex/skills/install-bead-wrappers
```

```bash
# 7. Skill frontmatter sanity
sed -n '1,12p' /Users/zacfarrell/.codex/skills/install-bead-wrappers/SKILL.md
sed -n '1,12p' /Users/zacfarrell/.claude/skills/install-bead-wrappers/SKILL.md
```

---

## Final Report Template

Use this exact structure:

```markdown
## Summary

Updated Agent Mail skill-template discipline and installed `install-bead-wrappers` for both Codex and Claude.

## Files Changed

- `~/.codex/skills/pipemaxxer/TEMPLATES.md`
- `~/.claude/skills/pipemaxxer/TEMPLATES.md`
- `~/.codex/skills/layermaxxer/SKILL.md`
- `~/.codex/skills/layermaxxer/references/TEMPLATES.md`
- `~/.claude/skills/layermaxxer/SKILL.md`
- `~/.claude/skills/layermaxxer/references/TEMPLATES.md`
- `~/.codex/skills/beads-to-execution/TEMPLATES.md`
- `~/.claude/skills/beads-to-execution/TEMPLATES.md`
- `~/.codex/skills/install-bead-wrappers/...`
- `~/.claude/skills/install-bead-wrappers/...`

## Verification

- No `to="broadcast"` / `to=["broadcast"]` remains in touched skill templates.
- Wrapper commands are present in pipemaxxer, layermaxxer, beads-to-execution, and install-bead-wrappers.
- Raw `br update` / `br close` claim-complete flows are gone or only appear as forbidden anti-patterns.
- `install-bead-wrappers` exists in both Codex and Claude skill roots.
- Wrapper assets match `Testproject1/bin/br-claim` and `Testproject1/bin/br-complete`.
- `install-bead-wrappers` Claude/Codex trees are byte-identical.

## Notes

- `AGENTS.md`, project docs, runtime code, `.beads/`, and `.git/hooks/` were not modified.
- Any pre-existing unrelated Claude/Codex template divergence was preserved unless it concerned Agent Mail discipline.
```

---

## Stop Condition

Stop after the final report. Do not continue into bead work, feature implementation, or project cleanup unless the user explicitly asks.
