# Build the `install-bead-wrappers` Skill

> **Paste this entire file into a fresh Claude Code / Codex session to onboard the implementation agent.**

---

## Context

`Testproject1` has bead-claim/complete wrappers (`bin/br-claim`, `bin/br-complete`), AGENTS.md Rule 2, and the precommit hook (`.git/hooks/hooks.d/pre-commit/50-agent-mail.py` extended for Start-mail enforcement). All three were deployed via Phase A and Phase B handoffs.

When you start L5 in this same repo, none of that is needed again — it's already in place. But when you start a NEW project (a future `Testproject2`, or any other repo that wants the same mail discipline), you'd need to re-do all three installs by hand.

This handoff builds a **reusable skill** — `install-bead-wrappers` — that any project can invoke to bootstrap the mail-discipline infrastructure in one command. Idempotent. Cross-CLI. The skill is the canonical source of the wrappers; updates to the wrappers happen in the skill, then re-running the skill in any project pulls the latest version.

**Why this matters strategically:** without this skill, the mail-discipline pattern is locked to `Testproject1`. With this skill, it's a portable workflow that travels with the user.

**Project key:** `/Users/alinaaitova/Desktop/Testproject1` (source of canonical wrappers; the skill itself lives outside this repo)
**Branch:** `main` only (no feature branches per AGENTS.md)
**Bead label:** N/A — skill creation, not bead-tracked
**Skill name:** `install-bead-wrappers`

---

## 1. Mission

Create a new skill at BOTH `~/.claude/skills/install-bead-wrappers/` AND `~/.codex/skills/install-bead-wrappers/`, with:

1. The canonical wrapper sources bundled as skill assets
2. The AGENTS.md Rule 2 snippet bundled as a template
3. A SKILL.md that defines a paste-ready workflow for installation in any new project
4. Supporting docs (METHODOLOGY.md, CHECKLIST.md)

When a future user runs `/install-bead-wrappers` in a fresh git repo (Claude or Codex agent), the skill instructs the agent to:
1. Validate prerequisites (git repo, am binary, server reachable)
2. Copy the wrappers from the skill's assets to the project's `bin/`
3. Append AGENTS.md Rule 2 (or create AGENTS.md if missing)
4. Invoke `am install-guard --project <auto-detected-key>` to install the precommit hook
5. Smoke-test the install
6. Report success

**The skill itself is dual-CLI symmetric** — same content in both `~/.claude/skills/` and `~/.codex/skills/`. Both Claude Code and Codex agents can invoke it identically, in any project on the user's machine.

---

## 2. AgentMail Registration

This is skill creation — no bead claim happens. You do NOT need to register with AgentMail. You DO need `AGENT_NAME` set if you intend to commit anywhere (most of this work is on the skill dirs which are typically outside the repo, so commits may not be required):

```bash
export AGENT_NAME="<your-name>"
```

---

## 3. Required Reading

In this order:

1. **`AGENTS.md`** at `/Users/alinaaitova/Desktop/Testproject1/AGENTS.md` — full read. Rule 2 lives here; you'll extract the section as a template asset.

2. **The canonical wrappers** (these become skill assets, copied verbatim):
   - `/Users/alinaaitova/Desktop/Testproject1/bin/br-claim` (~407 lines)
   - `/Users/alinaaitova/Desktop/Testproject1/bin/br-complete` (~415 lines)

3. **The Phase A handoff** (`/Users/alinaaitova/Desktop/Testproject1/prompts/phase-a-mail-discipline-infrastructure-handoff.md`) — the spec these wrappers were built against. Reference for what Rule 2 should look like and how dual-CLI parity is maintained.

4. **The Phase B handoff** (`/Users/alinaaitova/Desktop/Testproject1/prompts/phase-b-precommit-mail-enforcement-handoff.md`) — explains the precommit hook. The skill will delegate hook installation to `am install-guard` (one of the Rust binary's subcommands), so you do NOT need to ship the hook content yourself.

5. **An existing skill for structural reference** — pick any well-formed one from `~/.claude/skills/`, e.g. `~/.claude/skills/beads-from-sources/SKILL.md`. Note the YAML frontmatter format (name + description), the section headings, and how SKILL.md delegates to TEMPLATES.md / CHECKLIST.md / WORKFLOW.md / METHODOLOGY.md siblings. **Match the existing skill conventions exactly** — your new skill should look like it was created by the same author.

6. **The Rust binary's subcommand surface** — run:
   ```bash
   am --help
   am install-guard --help
   ```
   Confirm `install-guard` exists and what flags it takes. If a flag's spelling or semantics is different from what this handoff says, use what the binary actually supports (it's the source of truth).

After reading, you should be able to answer:
- *Where does Rule 2 begin and end in AGENTS.md?* (You'll extract it.)
- *What's the YAML frontmatter shape for a SKILL.md?* (Two fields: `name`, `description`.)
- *What command installs the precommit hook?* (Should be `am install-guard --project <key> --hook pre-commit` or similar — verify.)
- *Where do skill assets live conventionally?* (Inspect peer skills.)

---

## 4. Deliverables (per CLI; produce both)

For BOTH `~/.claude/skills/install-bead-wrappers/` AND `~/.codex/skills/install-bead-wrappers/`, create the following 6 files. Total = 12 files.

### D1 — `SKILL.md` (entry point)

YAML frontmatter:

```yaml
---
name: install-bead-wrappers
description: Bootstrap mail-discipline infrastructure in any git project. Copies bin/br-claim and bin/br-complete wrappers, appends AGENTS.md Rule 2, installs the agent-mail precommit hook via `am install-guard`. Idempotent — safe to re-run. Run once per new project. Works for both Claude Code and Codex agents.
---
```

Body sections (match the structure of existing skills like `beads-from-sources/SKILL.md`):

- **Mission** — one paragraph: what the skill installs and why (it's the per-project half of the AGENTS.md Rule 2 + Phase A + Phase B stack).
- **When to Use** — bullet list of triggers ("starting a new git project that will use bead-tracked work", "cloned an existing project that lacks the wrappers", "wrappers got out of date and you want to refresh", etc.)
- **Prerequisites** — `am` binary in PATH, git repo initialized, agent-mail server reachable on `127.0.0.1:8765`, `AGENT_NAME` env var set in the user's shell.
- **What This Installs** — the three artifacts: `<project>/bin/br-claim`, `<project>/bin/br-complete`, AGENTS.md Rule 2 section, `<project>/.git/hooks/hooks.d/pre-commit/50-agent-mail.py` (via `am install-guard`).
- **Workflow** — step-by-step procedure (see §5 of this handoff for the full procedure to embed).
- **Idempotency** — note that running twice is safe: wrappers overwrite (refreshing to skill's bundled version), Rule 2 append is guarded by a marker check, hook install is owned by `am install-guard` which is itself idempotent.
- **Troubleshooting** — common failure modes (server down, am missing, git not initialized, AGENTS.md write conflict).
- **MANDATORY STOP — DO NOT EXECUTE** — match the pattern from `beads-to-execution/SKILL.md`. After install completes, agent reports + waits for user authorization before any further work.

### D2 — `METHODOLOGY.md`

A longer-form document explaining the rationale, the dual-CLI pattern, the choice to delegate hook install to `am install-guard`, and how the skill keeps in sync with the canonical Testproject1 wrappers (i.e. it doesn't — they're snapshotted; updates require re-running this skill-creation handoff in the future).

### D3 — `CHECKLIST.md`

Pre-flight checklist (5–10 items) and post-install verification checklist (5–10 items). Match the format used by `~/.claude/skills/beads-from-sources/CHECKLIST.md`.

### D4 — `assets/br-claim`

Copied **verbatim** from `/Users/alinaaitova/Desktop/Testproject1/bin/br-claim`. No modifications. Copy preserves the shebang and executable bit (note: skill assets aren't typically chmod +x in the skill dir, since the install procedure does the chmod when copying into the target project's `bin/`).

### D5 — `assets/br-complete`

Copied **verbatim** from `/Users/alinaaitova/Desktop/Testproject1/bin/br-complete`. Same handling as D4.

### D6 — `assets/agents-md-rule-2.md`

Extract the content of "RULE NUMBER 2: BEAD CLAIM DISCIPLINE" from `/Users/alinaaitova/Desktop/Testproject1/AGENTS.md` (lines starting at the `## RULE NUMBER 2` header, ending at the next `##` header or `---` divider — read carefully to get the full section). Save as the standalone snippet that the install procedure appends to a target project's AGENTS.md.

Add a unique marker at the very top so the install procedure can detect "Rule 2 already present" idempotency:

```markdown
<!-- BEGIN: AGENTS.md Rule 2 (managed by install-bead-wrappers skill) -->

## RULE NUMBER 2: BEAD CLAIM DISCIPLINE — USE THE WRAPPERS

[...full content from Testproject1's AGENTS.md...]

<!-- END: AGENTS.md Rule 2 -->
```

The two HTML-comment markers let future re-runs of the skill detect the section without ambiguity.

---

## 5. The Install Procedure (embed this in SKILL.md "Workflow" section)

The skill's SKILL.md should describe — in language an agent (Claude or Codex) can execute step-by-step — the following procedure. Use exact paths and bash where helpful; let the agent know which steps fail-soft vs fail-hard.

```
PROJECT_ROOT = $(git rev-parse --show-toplevel)   # fail hard if not a git repo
SKILL_DIR    = <determined by which CLI is running this — see Implementation Notes>

Step 1 — Pre-flight (fail-hard if any of these fail)
  - Verify am binary: `command -v am`
  - Verify git repo: `git rev-parse --show-toplevel` (works if in repo)
  - Verify server: `curl -s --max-time 3 http://127.0.0.1:8765/healthz` returns alive
  - Verify env: $AGENT_NAME is set (export if not)

Step 2 — Install wrappers
  - mkdir -p $PROJECT_ROOT/bin
  - cp $SKILL_DIR/assets/br-claim     $PROJECT_ROOT/bin/br-claim
  - cp $SKILL_DIR/assets/br-complete  $PROJECT_ROOT/bin/br-complete
  - chmod +x $PROJECT_ROOT/bin/br-claim $PROJECT_ROOT/bin/br-complete
  - Verify: `$PROJECT_ROOT/bin/br-claim --help` exits 0

Step 3 — Install AGENTS.md Rule 2 (idempotent)
  - If $PROJECT_ROOT/AGENTS.md doesn't exist: create with a minimal header
    ("# AGENTS.md\n\n> Guidelines for AI coding agents...\n")
  - Check if "BEGIN: AGENTS.md Rule 2 (managed by install-bead-wrappers skill)" already
    appears in the file:
      - If yes: skip (idempotent)
      - If no: append the contents of $SKILL_DIR/assets/agents-md-rule-2.md
        (with two leading newlines for spacing) to the END of the file

Step 4 — Install precommit hook (delegate to am)
  - PROJECT_KEY=$(realpath $PROJECT_ROOT)
  - am install-guard --project "$PROJECT_KEY" --hook pre-commit
    (or whatever subcommand `am install-guard --help` actually documents — verify
     during skill-creation reading; if the binary on the user's machine has a
     different exact spelling, USE WHAT THE BINARY ACTUALLY SUPPORTS)

Step 5 — Smoke test
  - cd $PROJECT_ROOT
  - bin/br-claim --help    | head -3   # verify no crash
  - bin/br-complete --help | head -3   # verify no crash
  - test -x .git/hooks/hooks.d/pre-commit/50-agent-mail.py  # hook installed

Step 6 — Report (then STOP)
  - Print summary:
      Wrappers:   $PROJECT_ROOT/bin/{br-claim,br-complete}
      AGENTS.md:  $PROJECT_ROOT/AGENTS.md (Rule 2 present)
      Hook:       $PROJECT_ROOT/.git/hooks/hooks.d/pre-commit/50-agent-mail.py (installed via am)
      Smoke test: PASS
  - Wait for user authorization before any further action
```

The exact phrasing of these steps is yours — the SKILL.md should be readable and executable by a future agent who has not seen this handoff.

---

## 6. Implementation Notes (gotchas the agent must handle)

### 6a. SKILL_DIR discovery — CLI-aware

The skill needs to know its own asset path. Two CLI cases:
- Claude: skill dir is `~/.claude/skills/install-bead-wrappers/`
- Codex: skill dir is `~/.codex/skills/install-bead-wrappers/`

The SKILL.md should NOT hardcode either path. Instead, instruct the executing agent to detect via:

```bash
# Detect skill dir from this skill's own path (the agent knows its CLI's skill root):
SKILL_DIR=$(dirname "$(realpath ~/.claude/skills/install-bead-wrappers/SKILL.md 2>/dev/null || \
                       realpath ~/.codex/skills/install-bead-wrappers/SKILL.md)")
```

Or pass the assets via a known env var the CLI sets when invoking skills. Match whatever convention the existing skills use (inspect peer skills' SKILL.md to confirm).

### 6b. Wrappers may have evolved between snapshot and use

The wrappers in `Testproject1/bin/` are the canonical version at the moment this skill is created. If they evolve later (bug fixes, new features), the skill assets become stale. Two mitigations:
- The skill's METHODOLOGY.md should clearly state the snapshot version (e.g., "wrappers snapshot from Testproject1 commit `<sha>` on `<date>`").
- Future skill updates can re-run this handoff: re-copy the canonical wrappers into the skill's assets, bumping the snapshot pointer.

This is acceptable because wrappers don't change often, and any project using the skill will have a working snapshot — they just may not have the latest fixes until the skill is refreshed.

### 6c. AGENTS.md Rule 2 idempotency — the marker matters

The HTML-comment marker (`<!-- BEGIN: AGENTS.md Rule 2 (managed by install-bead-wrappers skill) -->`) is the ONLY signal the install procedure has for "is Rule 2 already there?" If the user manually copies Rule 2 (without the marker) and then runs the skill, the skill will append a SECOND copy — pollution. The METHODOLOGY.md should warn about this, and recommend always letting the skill manage Rule 2 (or manually adding the markers when copy-pasting).

### 6d. `am install-guard` may write outside the user's expectation

The `am` binary has its own conventions for where to install the hook (probably `.git/hooks/hooks.d/pre-commit/`). Inspect what `am install-guard --help` says, and document the install path in METHODOLOGY.md. The hook gets project-specific config baked in (the `PROJECT = "Testproject1"` line we saw earlier comes from `am install-guard`). This means the hook will correctly point to the new project's name automatically — no manual edits.

### 6e. Dual-CLI symmetry

After creating, verify the two installations are byte-identical:

```bash
diff -r ~/.claude/skills/install-bead-wrappers/ ~/.codex/skills/install-bead-wrappers/
# Should print nothing
```

If anything differs, either the asset copies were imperfect or the SKILL.md / METHODOLOGY.md content drifted. Sync them up.

---

## 7. Acceptance Criteria

### Skill structure (both CLIs)

- [ ] `~/.claude/skills/install-bead-wrappers/SKILL.md` exists with proper YAML frontmatter
- [ ] `~/.claude/skills/install-bead-wrappers/METHODOLOGY.md` exists
- [ ] `~/.claude/skills/install-bead-wrappers/CHECKLIST.md` exists
- [ ] `~/.claude/skills/install-bead-wrappers/assets/br-claim` exists, is byte-identical to `Testproject1/bin/br-claim`
- [ ] `~/.claude/skills/install-bead-wrappers/assets/br-complete` exists, is byte-identical to `Testproject1/bin/br-complete`
- [ ] `~/.claude/skills/install-bead-wrappers/assets/agents-md-rule-2.md` exists with HTML-comment markers
- [ ] All 6 above ALSO exist in `~/.codex/skills/install-bead-wrappers/`
- [ ] `diff -r ~/.claude/skills/install-bead-wrappers/ ~/.codex/skills/install-bead-wrappers/` is empty

### Behavioral test (in a throwaway test repo)

- [ ] `mkdir /tmp/install-bead-wrappers-test && cd /tmp/install-bead-wrappers-test && git init`
- [ ] Invoke the skill (paste the SKILL.md workflow into a new agent session pointing at this temp repo)
- [ ] After install: `bin/br-claim --help` runs (exit 0)
- [ ] After install: `bin/br-complete --help` runs (exit 0)
- [ ] After install: `AGENTS.md` contains the Rule 2 markers + content
- [ ] After install: `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` exists, is executable
- [ ] **Idempotency:** invoking the skill a SECOND time on the same temp repo:
  - Skips Rule 2 append (no duplicate)
  - Re-copies wrappers (overwrite is fine)
  - `am install-guard` no-ops or refreshes (depends on its semantics — verify)
- [ ] Cleanup: `rm -rf /tmp/install-bead-wrappers-test` after test

### Project repo (Testproject1) NOT touched

- [ ] `git status` in `/Users/alinaaitova/Desktop/Testproject1` shows clean working tree
- [ ] No new commits in Testproject1 from this handoff (the skill creation is entirely in `~/.{claude,codex}/skills/`)

---

## 8. Verification (run before reporting done)

```bash
# Asset byte-identity with canonical sources
diff ~/.claude/skills/install-bead-wrappers/assets/br-claim    /Users/alinaaitova/Desktop/Testproject1/bin/br-claim    && echo "br-claim OK"
diff ~/.claude/skills/install-bead-wrappers/assets/br-complete /Users/alinaaitova/Desktop/Testproject1/bin/br-complete && echo "br-complete OK"

# Dual-CLI symmetry
diff -r ~/.claude/skills/install-bead-wrappers/ ~/.codex/skills/install-bead-wrappers/

# Rule 2 markers present
grep -q "BEGIN: AGENTS.md Rule 2" ~/.claude/skills/install-bead-wrappers/assets/agents-md-rule-2.md && echo "BEGIN marker OK"
grep -q "END: AGENTS.md Rule 2"   ~/.claude/skills/install-bead-wrappers/assets/agents-md-rule-2.md && echo "END marker OK"

# YAML frontmatter on SKILL.md
head -5 ~/.claude/skills/install-bead-wrappers/SKILL.md   # should show frontmatter

# Throwaway test repo round-trip (only run if you're set up to do this)
cd /tmp && rm -rf install-bead-wrappers-test
mkdir install-bead-wrappers-test && cd install-bead-wrappers-test && git init
# ... follow the SKILL.md workflow manually ...
test -x bin/br-claim && echo "wrapper executable OK"
test -x bin/br-complete && echo "wrapper executable OK"
grep -q "RULE NUMBER 2" AGENTS.md && echo "Rule 2 OK"
test -f .git/hooks/hooks.d/pre-commit/50-agent-mail.py && echo "hook OK"
cd /tmp && rm -rf install-bead-wrappers-test
```

If the throwaway test step is too involved for this session, document it as "tested manually post-create" or queue it for the pre-L5 sanity-check handoff.

---

## 9. Commit + Push

The skill dirs (`~/.{claude,codex}/skills/`) are typically NOT version-controlled within the project. They're per-machine. So this handoff produces no commits to the Testproject1 repo. **Verify the working tree stays clean:**

```bash
cd /Users/alinaaitova/Desktop/Testproject1
git status
# Expected: "On branch main / Your branch is up to date / nothing to commit"
```

If your skill dirs are version-controlled in some OTHER repo (a personal dotfiles repo), follow that repo's commit conventions. Most users don't, so usually nothing further is needed.

---

## 10. Report Format When Done

```
install-bead-wrappers skill creation complete.

Skill installed in BOTH CLI directories:
  ~/.claude/skills/install-bead-wrappers/  (6 files)
  ~/.codex/skills/install-bead-wrappers/   (6 files)

File-by-file:
  SKILL.md                       — entry point with YAML frontmatter
  METHODOLOGY.md                 — rationale + dual-CLI design notes
  CHECKLIST.md                   — pre-flight + post-install checklist
  assets/br-claim                — verbatim copy of Testproject1/bin/br-claim
  assets/br-complete             — verbatim copy of Testproject1/bin/br-complete
  assets/agents-md-rule-2.md     — extracted Rule 2 with HTML-comment markers

Wrapper snapshot version:
  Testproject1 commit <sha> as of <date>

Dual-CLI symmetry verified:
  diff -r ~/.claude/skills/install-bead-wrappers/ ~/.codex/skills/install-bead-wrappers/ → empty

Throwaway test repo round-trip:
  PASS / NOT PERFORMED — <reason if not performed>

Idempotency:
  PASS / NOT VERIFIED — <reason>

Project repo (Testproject1):
  Untouched. git status clean.
```

Stop after reporting.

---

## 11. Critical Rules

1. **Project repo (Testproject1) MUST NOT be modified.** All work is in `~/.{claude,codex}/skills/install-bead-wrappers/`.
2. **Dual-CLI symmetry is mandatory.** Whatever lands in `~/.claude/skills/install-bead-wrappers/` MUST also land in `~/.codex/skills/install-bead-wrappers/` with byte-identical content.
3. **Wrapper assets are byte-for-byte copies of Testproject1/bin/{br-claim,br-complete}.** No edits, no "improvements." If the wrappers have a bug, fix them in `Testproject1/bin/` first (separate handoff), THEN re-snapshot into the skill assets.
4. **Match existing skill conventions.** YAML frontmatter, section structure, MANDATORY STOP pattern, file naming — all should look indistinguishable from peer skills like `beads-from-sources`, `bead-completion-audit`. Inspect peers; do not reinvent.
5. **Delegate hook install to `am install-guard`.** Don't ship hook content; let the binary handle it. The binary's behavior is canonical.
6. **The skill's install procedure is described in SKILL.md, not implemented as code.** This is consistent with how skills work — they instruct the agent in natural language; the agent executes the steps via shell commands.
7. **Idempotency is a contract.** The skill must be safe to invoke multiple times in the same project. Use the HTML-comment marker for Rule 2 detection; let `am install-guard` handle hook idempotency; wrappers can be overwritten safely.
8. **NEVER stash other agents' work.** If skill files were created by another agent during this session, treat as legitimate; merge or ask before overwriting.
9. **NEVER use `git reset --hard` or `git push --force`.** Standard AGENTS.md "Irreversible Git Actions" rules apply.
10. **Report and stop.** Do not chain into another action without user authorization (matches the `MANDATORY STOP` pattern from peer skills like `beads-to-execution`).
