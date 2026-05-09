# Phase A — Mail-Discipline Infrastructure Deployment

> **Paste this entire file into a fresh Claude Code / Codex session to onboard the implementation agent.**

---

## Context

L4 implementation finished on 2026-05-08 with 56/56 task beads closed. Empirically, Agent Mail compliance failed: 4 Start mails sent in the first 3 minutes, then 0 mails for the next 10 hours. Beads kept closing via `br update` / `br close` / git, but the swarm had no audit trail. Even resync prompts pasted into 3 of 4 agent sessions were ignored.

**Root cause** (from analysis): Mail discipline is prompting-enforced in the kickoff (which gets compacted), not code-enforced in the bead bodies (which are re-read fresh on every claim) or in the precommit hook (which only checks file reservations). Agents reliably do things with consequences and skip things without — mail had no consequences.

**Phase A's role:** Deploy the infrastructure that puts mail discipline ON the critical path of bead claim/complete, but with **zero workflow risk** for the imminent L4 audit-beads + fix-beads work. Phase B (separate handoff) activates the precommit enforcement AFTER L4 audit/fix completes.

---

## 1. Mission

Deploy 7 deliverables that make Agent Mail compliance the **easiest path** for future agents:
- Wrappers that are shorter to invoke than raw tool calls
- Bead bodies that surface the protocol on every `br show`
- AGENTS.md that treats it as constitutional

**This is purely additive — no existing workflow breaks.**

L4 audit-beads and fix-beads will run AFTER you finish, and they must continue to work exactly as L4 itself did. **Phase B (the precommit hook extension that actually enforces mail) is a SEPARATE handoff that runs AFTER L4 audit/fix completes — DO NOT touch the precommit hook in this handoff.**

**Project key:** `/Users/zacfarrell/Desktop/Testproject1`
**Branch:** `main` only (no feature branches per AGENTS.md)
**Bead label:** N/A — this is infrastructure work, not bead-tracked

---

## 2. AgentMail Registration (optional but recommended)

If you'll be sending mail during testing, register up front:

```
macro_start_session(
  project_key="/Users/zacfarrell/Desktop/Testproject1",
  program="phase-a-infrastructure",
  model="<your-model>",
  task_description="Deploy mail-discipline wrappers, skill template updates, AGENTS.md Rule 2"
)
```

If you're only writing scripts and editing files (no mail tool calls), registration is optional.

---

## 3. Required Reading (do this first, before writing any code)

In this order:

1. **`AGENTS.md`** at project root — full read. You'll be adding a new constitutional section.
2. **`.git/hooks/hooks.d/pre-commit/50-agent-mail.py`** — read the first 100 lines and the bottom 100 lines. You need to understand HOW the existing hook talks to the agent-mail server at `127.0.0.1:8765` (the auth flow, the API client, the message format). Your wrappers MUST use the same auth + endpoint pattern so they share infrastructure. **DO NOT MODIFY THIS FILE in Phase A.**
3. **`bin/br-sync`** — read all ~60 lines. This is the existing wrapper pattern in the repo. Match its style (shebang, error handling, exit codes).
4. **`~/.claude/skills/beads-from-sources/SKILL.md`** + any `TEMPLATES.md` or `WORKFLOW.md` siblings in that directory — read fully. This is one of the three skills you'll modify.
5. **`~/.claude/skills/bead-plan-audit/SKILL.md`** + siblings — skim. Note where in the workflow it creates/edits beads.
6. **`~/.claude/skills/bead-completion-audit/SKILL.md`** + siblings — skim. Note where it creates fix-beads.
7. **`~/.claude/skills/beads-to-execution/SKILL.md`** + any `TEMPLATES.md` siblings — skim. This generates kickoff/push prompts.
8. **`~/.codex/skills/layermaxxer/SKILL.md`** and **`~/.codex/skills/pipemaxxer/SKILL.md`** — skim. These orchestrate the broader pipeline.
9. **One existing bead body**, e.g.
   ```bash
   br show testproject1-4zq.6.8 --json | jq -r '..|objects|select(.id=="testproject1-4zq.6.8").description'
   ```
   See the current shape so you know where the new "Coordination Protocol" section fits.

After reading, you should be able to answer: *Where in the agent-mail server's HTTP API is `send_message` exposed? Where is `file_reservation_paths`? What's the auth header format?* If you can't, re-read item #2 until you can.

---

## 4. Deliverables (7 items)

### Skill-update protocol (READ FIRST — applies to D3 through D7)

Skills are installed in **two parallel locations** on this machine, one per CLI:

- `~/.claude/skills/<skill>/` — used by Claude Code agents
- `~/.codex/skills/<skill>/` — used by Codex (codex-cli) agents

These are **separate file copies, not symlinks**. They currently have identical content but edits to one location do NOT propagate to the other. The L4 implementation was done entirely by Codex agents (`codex-cli` / gpt-5) — so updating only the Claude side would leave the most likely future execution path unfixed.

**For every skill template you touch in D3 through D7:**

1. Run `ls ~/.claude/skills/<skill>/ ~/.codex/skills/<skill>/ 2>/dev/null` to see which directories exist.
2. If BOTH exist: edit the template in BOTH locations with identical content.
3. If only one exists: edit the one that exists. Note the asymmetry in your final report.
4. After editing, verify with:
   ```bash
   diff ~/.claude/skills/<skill>/<file> ~/.codex/skills/<skill>/<file>
   # Should print nothing (identical)
   ```
5. If the two locations had DIFFERENT pre-edit content (rare but possible), apply the same Coordination Protocol section to both, then run `diff` to confirm post-edit equivalence — and note the divergence in your final report so the user can investigate.

**Why both:** the goal of Phase A is that any future agent — Claude or Codex — generating beads/kickoffs/audits via these skills will produce mail-disciplined output. Skipping either CLI's skill directory leaves a hole.

---

### D1 — `bin/br-claim` wrapper script (foundational, do this first)

**Location:** `bin/br-claim` (new file, executable)
**Language:** Python 3 (matches `50-agent-mail.py`). Shebang `#!/usr/bin/env python3`.
**Dependencies:** stdlib only (`urllib.request`, `subprocess`, `json`, `sys`, `os`, `re`). No `requests` or `httpx` — the existing hook uses stdlib, match it.

**Usage:**

```bash
bin/br-claim <bead-id>                  # claim a bead atomically
bin/br-claim <bead-id> --dry-run        # show what would happen, change nothing
bin/br-claim --help
```

**What it does (exact order — DO NOT REORDER):**

1. **Resolve agent identity.** Read `$AGENT_MAIL_NAME` env var. If unset, exit 1 with message `"AGENT_MAIL_NAME not set. Run macro_start_session first or export the variable."`
2. **Check current bead state.** Call `br show <bead-id> --json` via subprocess. Parse the JSON (it returns an array — the matching object has `.id == <bead-id>`). Exit 1 if:
   - Bead doesn't exist
   - `status == "in_progress"` (someone else has it)
   - `status == "closed"` (already done)
3. **Idempotency check.** Query `search_messages` for `query="[<bead-id>] Start"`. If a message from `<your-agent-name>` already exists in the last 4 hours, skip step 5 (don't re-send Start). If a message from a DIFFERENT agent exists, exit 1 with `"Bead claimed by <other-agent>; abort."`
4. **Acquire file reservations.** Parse the bead description for the section "## Files to Create/Modify" (regex-extract the file paths under it). If no such section, fall back to a wildcard-warning message and let the agent confirm. Call `file_reservation_paths(paths=<list>, ttl_seconds=3600, exclusive=true, reason=<bead-id>)` via the agent-mail HTTP API. **Exit 1 with the conflict message if any reservation conflicts** — this is the primary collision detector.
5. **Send Start mail.** Call `list_contacts` to get peers. Call `send_message(to=<peers list, NEVER "broadcast">, thread_id=<bead-id>, subject="[<bead-id>] Start: <title>", body_md="Claiming bead. Files: <list>.", ack_required=true)`.
6. **Update br state.** `br update <bead-id> --status=in_progress` via subprocess. If this fails after mail was sent, log a warning but exit 0 (mail-state can be reconciled later by re-running; br is the trailing source of truth).
7. **Exit 0** with summary: `Claimed: <bead-id> · Files reserved: N · Mail sent · br=in_progress`.

**Why this order:** file-reservation BEFORE Start mail is critical. File reservation is the only step that can detect a collision atomically. If mail goes first, two agents can both successfully send Start mail for the same bead and you get a soft race. Doing file reservation first means the loser of the race never sends mail.

**Error handling:**
- All HTTP errors must report the exact endpoint and status code.
- Server unreachable (connect refused on 127.0.0.1:8765) should exit 2 with `"agent-mail server not running. Start it before claiming beads."` — do NOT fall through to br update.
- Use `--dry-run` to print intentions and skip all side effects.

### D2 — `bin/br-complete` wrapper script

**Location:** `bin/br-complete` (new file, executable)
**Same conventions as D1.**

**Usage:**

```bash
bin/br-complete <bead-id>                       # complete a claimed bead
bin/br-complete <bead-id> --commit <sha>        # explicit commit hash for the Complete mail body
bin/br-complete <bead-id> --dry-run
bin/br-complete --help
```

**What it does (exact order — DO NOT REORDER):**

1. Resolve `$AGENT_MAIL_NAME` (same as D1).
2. **Verify bead is claimed by me.** `br show <bead-id> --json` → status MUST be `in_progress`. `search_messages query="[<bead-id>] Start"` → there must be a Start mail from `$AGENT_MAIL_NAME` (otherwise the bead was never properly claimed; refuse to complete it without a corresponding Start). Exit 1 if either fails.
3. **Resolve commit hash.** If `--commit` provided, use it. Else `git log -1 --format=%H --grep="^<bead-id>:"` and use the most recent matching commit. Error if none found.
4. **Verify code is pushed.** `git rev-parse HEAD` and `git rev-parse @{u}` must match — i.e. local is up-to-date with origin AND the commit exists upstream. Exit 1 with `"Commit not pushed. Run git push first."` if not.
5. **Send Complete mail.** `send_message(to=<peers>, thread_id=<bead-id>, subject="[<bead-id>] Complete: <title>", body_md="Commit: <sha>. All checks pass.", ack_required=false)`. The "All checks pass" claim is the agent's responsibility to verify before invoking — document this.
6. **Close bead.** `br close <bead-id> --reason "Completed"` via subprocess.
7. **Sync beads.** `bin/br-sync` then `git add .beads/ && git commit -m "beads: close <bead-id>" && git pull --rebase && git push`. If any step fails, exit 1 with the failure detail; don't release reservations yet (the user might re-run after fix).
8. **Release file reservations.** `release_file_reservations(agent_name=<your-name>)` (releases all of yours; safe because you're done with this bead).
9. Exit 0 with summary.

**Error handling:** same conventions as D1.

### D3 — Update `beads-from-sources` skill template (BOTH `~/.claude/skills/` and `~/.codex/skills/`)

**Goal:** Every newly-generated bead body includes a top-level "## Coordination Protocol" section that points the executing agent to the wrappers.

**Locations to edit (per the Skill-update protocol above):**
- `~/.claude/skills/beads-from-sources/<template-file>`
- `~/.codex/skills/beads-from-sources/<template-file>`

From your reading in §3, identify whether the template lives in `SKILL.md` (inline template), `TEMPLATES.md`, or a sibling. The file may have a different layout in one CLI's tree vs the other — inspect both. **Edit each file in place.** Do not create new files.

**Section to insert** (place near the top of the template, immediately after the bead identity / SOURCE OF TRUTH section, before "## Context"):

````markdown
## Coordination Protocol (MANDATORY — read before writing code)

This bead is coordinated through Agent Mail + file reservations + br state. Use the project wrappers to claim and complete it atomically — do not call `br update` or `br close` directly:

```bash
# Before writing any code:
bin/br-claim {{BEAD_ID}}
# Atomically: reserves files, sends [{{BEAD_ID}}] Start mail, sets status=in_progress.

# After code is committed AND pushed AND tests pass:
bin/br-complete {{BEAD_ID}}
# Atomically: sends [{{BEAD_ID}}] Complete mail, closes bead, syncs .beads/, releases reservations.
```

If a wrapper errors, fix the underlying issue (server down, file conflict, missing env var) — do not bypass. The Phase B precommit hook (when active) will block any commit referencing this bead without a Start mail in the archive.
````

The `{{BEAD_ID}}` token must match whatever placeholder the rest of the template uses for the bead's full ID (e.g. `testproject1-4zq.5.3`). Inspect the existing template for the convention and conform.

### D4 — Update `bead-completion-audit` skill template (BOTH CLIs)

**Goal:** When this skill creates fix-beads (during the audit/fix loop), each fix-bead body must also include the Coordination Protocol section.

**Locations to edit:**
- `~/.claude/skills/bead-completion-audit/<template-file>`
- `~/.codex/skills/bead-completion-audit/<template-file>`

Find the file in each that defines the fix-bead body shape. May be inline in `SKILL.md`, may be a separate template file. Edit each in place.

**Section to insert:** identical content to D3. Same `{{BEAD_ID}}` substitution rule.

**Important:** if `bead-completion-audit` doesn't have its own bead-body template (i.e., it just says "use beads-from-sources"), then no edit is needed here — D3 covers it. Verify before assuming, in BOTH CLI directories, and note your finding in the final report.

### D5 — Update `bead-plan-audit` skill template (BOTH CLIs)

**Same as D4** but for the pre-execution audit skill. Edit both:
- `~/.claude/skills/bead-plan-audit/<template-file>`
- `~/.codex/skills/bead-plan-audit/<template-file>`

If it adds or edits beads, those beads must include the Coordination Protocol. If it only edits existing beads, ensure the editing logic preserves the section if it's already there and doesn't strip it.

### D6 — Update `beads-to-execution` skill templates (BOTH CLIs)

**Goal:** Future kickoff and push prompts (generated by this skill) should reference `bin/br-claim` and `bin/br-complete` as the canonical work-loop commands, replacing the current verbose Agent Mail prose in the kickoff §6 and §7.

**Locations to edit:**
- `~/.claude/skills/beads-to-execution/<template-file>` (likely `TEMPLATES.md`)
- `~/.codex/skills/beads-to-execution/<template-file>`

Edit each in place.

**Specific edits to the kickoff template:**

- §6 "Bead Completion Policy" — REPLACE the long checklist with:
  ```
  Use the project wrappers to claim and complete every bead:
  - `bin/br-claim <bead-id>` — claims atomically (mail + reservations + br update)
  - `bin/br-complete <bead-id>` — completes atomically (mail + br close + sync + release)

  Do NOT use raw `br update --status=in_progress` or `br close` — they bypass mail and file reservations. The Phase B precommit hook blocks commits without proper claims.
  ```

- §7 "Work Loop" — REPLACE the CLAIM + ANNOUNCE block with:
  ```
  bin/br-claim <bead-id>
  ```

  And REPLACE the COMPLETE + RELEASE block with:
  ```
  git commit -m "<bead-id>: <description>"
  git pull --rebase && git push
  bin/br-complete <bead-id>
  ```

**Specific edits to the push template:**

- §3 "Bead Completion Policy" — REPLACE the verbose mail block with the same wrapper-based copy as above.
- §4 "Work Loop" — same compaction.

Keep the spec/TDD/strategy reading list, file surfaces, dependency spine, and verification (Stage Validation Test Gate) sections **unchanged** — those are valuable. Only the workflow plumbing changes.

### D7 — Update `layermaxxer` and `pipemaxxer` orchestrator skills (BOTH CLIs)

**Goal:** Both orchestrators currently produce kickoff prompts via `beads-to-execution`. Confirm the wrapper-based kickoffs flow through. If either skill has its own template instead of inheriting, update it the same way as D6.

**Locations to inspect (and edit if needed):**
- `~/.claude/skills/layermaxxer/SKILL.md`
- `~/.codex/skills/layermaxxer/SKILL.md`
- `~/.claude/skills/pipemaxxer/SKILL.md`
- `~/.codex/skills/pipemaxxer/SKILL.md`

**Action:** read all four SKILL.md files. Look for any inline kickoff/push prose. If you find it, update it to match D6. If they delegate to `beads-to-execution`, no change needed — note this per file in your final report.

**Heads-up:** if you find an inline template in only ONE of the two CLI copies of a skill (i.e., asymmetric divergence), apply the same Coordination Protocol fix to both copies AND flag the divergence in your report so the user can decide whether to consolidate them.

### D8 — `AGENTS.md` constitutional rule

**Location:** `AGENTS.md` at project root.

**Where to insert:** as a new top-level section, immediately AFTER the existing "## RULE NUMBER 1: NO FILE DELETION" section and BEFORE "## Irreversible Git & Filesystem Actions". This places it as a peer of the no-deletion rule — both are constitutional.

**Section content:**

````markdown
## RULE NUMBER 2: BEAD CLAIM DISCIPLINE — USE THE WRAPPERS

For any bead-tracked work in this repo, you MUST use the project wrappers to claim and complete beads:

- **`bin/br-claim <bead-id>`** to start work — atomically reserves files, sends `[<bead-id>] Start` mail to peers, and sets the bead `in_progress`.
- **`bin/br-complete <bead-id>`** to finish work — atomically sends `[<bead-id>] Complete` mail, closes the bead, syncs `.beads/`, and releases file reservations.

You MUST NOT use raw `br update --status=in_progress` or `br close <id>` directly — they bypass Agent Mail and file reservations, which leaves the swarm unable to detect collisions. The pre-commit hook at `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` enforces this: commits with a `<bead-id>:` message prefix require a corresponding Start mail in the archive.

If a wrapper errors, fix the underlying problem (agent-mail server down, file reservation conflict, missing `AGENT_MAIL_NAME` env var). Never bypass.

This is a constitutional rule, equal in force to Rule 1 (no file deletion). It applies to every layer (L0 through L6 and beyond) and every future stage.
````

Adjust prose lightly to match the existing AGENTS.md voice if needed, but keep all the specifics (paths, command names, the rule-2 framing).

---

## 5. Order of Operations

Do these in this order to avoid forward references:

1. **D1** (`bin/br-claim`) — foundational, blocks everything else.
2. **D2** (`bin/br-complete`) — pairs with D1.
3. **D8** (AGENTS.md) — gives all subsequent edits a constitutional anchor to reference.
4. **D3, D4, D5, D6, D7** — can be done in parallel after D1/D2/D8 land. They all reference `bin/br-claim` / `bin/br-complete` paths and the AGENTS.md rule.

After D1 and D2 are written:

```bash
chmod +x bin/br-claim bin/br-complete
```

---

## 6. Acceptance Criteria

Each must be true before declaring Phase A complete:

### Wrappers

- [ ] `bin/br-claim --help` prints sane usage; exit 0
- [ ] `bin/br-complete --help` prints sane usage; exit 0
- [ ] `bin/br-claim <fake-bead-id>` exits 1 with a clear error (bead not found)
- [ ] `bin/br-claim <real-bead-id-already-closed>` exits 1 with a clear error (already closed)
- [ ] `bin/br-claim <real-bead-id> --dry-run` prints intentions, makes ZERO changes (verify with `git status`, `br show`, `search_messages`)
- [ ] Both scripts use stdlib only (no pip dependencies)
- [ ] Both scripts read `AGENT_MAIL_NAME` env var; error sanely if unset
- [ ] Connection error to `127.0.0.1:8765` produces exit code 2 with a clear message, NOT a Python traceback
- [ ] Order verified: file_reservation BEFORE Start mail BEFORE `br update` (in br-claim)

### Skill template updates (BOTH CLIs — Claude AND Codex)

For every check below, verify in BOTH `~/.claude/skills/` AND `~/.codex/skills/`:

- [ ] `beads-from-sources` template includes the Coordination Protocol section
- [ ] `bead-completion-audit` template includes it (or doc'd as inheriting from beads-from-sources)
- [ ] `bead-plan-audit` template includes it
- [ ] `beads-to-execution` kickoff and push templates use wrapper commands (not the long mail-prose blocks)
- [ ] `layermaxxer` and `pipemaxxer` either inherit (verified) or are updated
- [ ] All edited files have IDENTICAL content across the two CLI directories (`diff` returns empty), unless pre-edit divergence was documented in the report

Single-line verification:
```bash
for skill in beads-from-sources bead-completion-audit bead-plan-audit beads-to-execution; do
  echo "=== $skill ==="
  grep -l 'bin/br-claim' ~/.claude/skills/$skill/ 2>/dev/null
  grep -l 'bin/br-claim' ~/.codex/skills/$skill/ 2>/dev/null
done
```
Both lines under each `===` header should print at least one matching file path.

### AGENTS.md

- [ ] New "RULE NUMBER 2" section exists between Rule 1 and the Irreversible Git Actions section
- [ ] References both wrapper paths verbatim
- [ ] References the precommit hook by its actual path (`.git/hooks/hooks.d/pre-commit/50-agent-mail.py`)

### What MUST NOT have changed

- [ ] `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` — UNCHANGED. `git diff .git/hooks/` must be empty.
- [ ] Existing L4 kickoff/push prompts under `docs/pipeline/l4-shared-visibility/artifacts/` — UNCHANGED.
- [ ] Existing L4 bead bodies in `.beads/issues.jsonl` — UNCHANGED. (`git diff .beads/`)
- [ ] No new beads created.

---

## 7. Verification (run before reporting done)

```bash
# Working tree should show only the new wrapper files + AGENTS.md + skill files
git status

# Wrappers exist and are executable
test -x bin/br-claim && echo OK || echo MISSING
test -x bin/br-complete && echo OK || echo MISSING

# Wrappers don't crash
bin/br-claim --help | head -5
bin/br-complete --help | head -5

# AGENTS.md has the new rule
grep -A 2 "RULE NUMBER 2" AGENTS.md

# Skill template was actually updated in BOTH CLI directories (do for each of D3-D7)
for skill in beads-from-sources bead-completion-audit bead-plan-audit beads-to-execution layermaxxer pipemaxxer; do
  echo "--- $skill ---"
  grep -rn 'bin/br-claim' ~/.claude/skills/$skill/ 2>/dev/null | head -1
  grep -rn 'bin/br-claim' ~/.codex/skills/$skill/ 2>/dev/null | head -1
done

# Verify both CLI copies are in sync after edits (where the skill exists in both)
for skill in beads-from-sources bead-completion-audit bead-plan-audit beads-to-execution; do
  if [ -d ~/.claude/skills/$skill ] && [ -d ~/.codex/skills/$skill ]; then
    diff -r ~/.claude/skills/$skill/ ~/.codex/skills/$skill/ > /tmp/skill-diff-$skill 2>&1
    if [ -s /tmp/skill-diff-$skill ]; then
      echo "WARNING: $skill differs between Claude and Codex installations:"
      cat /tmp/skill-diff-$skill | head -20
    else
      echo "OK: $skill identical across CLIs"
    fi
  fi
done

# Existing infrastructure NOT touched
git diff .git/hooks/                                     # MUST be empty
git diff docs/pipeline/l4-shared-visibility/artifacts/   # MUST be empty
git diff .beads/                                         # MUST be empty
```

If any verification fails, fix before reporting.

---

## 8. Commit + Push

Do NOT batch all 7 deliverables in one commit. Use these commits in order:

1. `chore: add bin/br-claim and bin/br-complete wrappers for atomic bead claim/complete` — files: `bin/br-claim`, `bin/br-complete`
2. `docs(agents): add Rule 2 — bead claim discipline via wrappers` — files: `AGENTS.md`
3. `chore(skills): embed coordination protocol in bead body templates` — files: skill templates touched in D3-D7

After each: `git pull --rebase && git push`.

---

## 9. Report Format When Done

```
Phase A complete.

Wrappers:
  - bin/br-claim (NNN lines)
  - bin/br-complete (NNN lines)

Skill templates updated (BOTH CLIs unless noted):
  - beads-from-sources/<file>           Claude: ✓  Codex: ✓
  - bead-completion-audit/<file>        Claude: ✓  Codex: ✓  (or 'inherits from beads-from-sources')
  - bead-plan-audit/<file>              Claude: ✓  Codex: ✓
  - beads-to-execution/<file>           Claude: ✓  Codex: ✓
  - layermaxxer/<file>                  Claude: ✓ or 'inherits'  Codex: ✓ or 'inherits'
  - pipemaxxer/<file>                   Claude: ✓ or 'inherits'  Codex: ✓ or 'inherits'

Cross-CLI sync verification (diff -r per skill): all identical / divergences flagged below
  <list any unexpected divergences if found>

AGENTS.md: Rule 2 added at line N.

Untouched: .git/hooks/, L4 prompts, .beads/issues.jsonl.

Commits pushed: <sha1>, <sha2>, <sha3>.

Phase B handoff (prompts/phase-b-precommit-mail-enforcement-handoff.md) is now safe to execute (after L4 audit/fix completes).
```

Do NOT execute Phase B. Do NOT touch the precommit hook. Stop after reporting.

---

## 10. Critical Rules

1. **DO NOT modify `.git/hooks/hooks.d/pre-commit/50-agent-mail.py`** — that's Phase B's job.
2. **DO NOT modify existing L4 artifacts** — kickoff, push, bead bodies, smoke diary, etc. are frozen.
3. **DO NOT create new beads** — this is infrastructure, not bead-tracked work.
4. **DO follow the dependency order** in §5 — wrappers exist before they're referenced in templates.
5. **DO use stdlib only** in the wrappers — match the existing precommit hook's stack.
6. **DO test the wrappers manually** before declaring done — broken wrappers will block all of Phase B.
7. **NEVER stash other agents' work** — concurrent commits on `main` are normal; `git pull --rebase`.
8. **NEVER use `git reset --hard` or `git push --force`** — see AGENTS.md "Irreversible Git Actions".
9. **Work is NOT done until `git push` succeeds** — see AGENTS.md "Landing the Plane".
