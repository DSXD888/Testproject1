# Cleanup Stale Python References in `beads-to-execution` Skill

> **Paste this entire file into a fresh Claude Code / Codex session to onboard the implementation agent.**

---

## Context

During post-L4 forensic analysis (2026-05-08), 4 stale references to the OLD Python implementation of `mcp_agent_mail` were found in the `beads-to-execution` skill — across BOTH the Claude and Codex installations. The Python package was migrated to a Rust binary (`am 0.2.54`) some time ago, so these references would mislead any agent that hits server trouble and consults this guidance.

**The references are in TROUBLESHOOTING sections only**, so they don't affect the happy path of mail-disciplined bead claiming. But they pollute the recovery flow, and at least one of those file reads happens during routine session warmup.

**This is purely a documentation correction.** No functional code changes. No behavior changes. ~5 minutes of work.

**Project key:** `/Users/alinaaitova/Desktop/Testproject1`
**Branch:** `main` only (no feature branches per AGENTS.md)
**Bead label:** N/A — this is documentation cleanup, not bead-tracked

---

## 1. Mission

Replace the stale Python invocation in 4 files (2 in Claude's skill dir, 2 in Codex's skill dir) with the equivalent Rust binary command. Maintain dual-CLI symmetry (the two skill installations should remain identical content after the edit).

---

## 2. AgentMail Registration

This is documentation cleanup — no bead claim happens. You do NOT need to register with AgentMail. You DO need `AGENT_NAME` set to satisfy the precommit hook on your final commit:

```bash
export AGENT_NAME="<your-name>"
```

Use any string that identifies you (e.g., your git config name). The hook only checks presence, and your commit message uses a `chore(...):` prefix that exempts it from the Start-mail subcheck.

---

## 3. Required Reading

In this order:

1. **`AGENTS.md`** — full read. Confirms current Rule 2 / wrapper conventions you must NOT touch.
2. **The 4 stale files (read all, both CLIs):**
   - `~/.claude/skills/beads-to-execution/CHECKLIST.md` (line ~184)
   - `~/.claude/skills/beads-to-execution/WORKFLOW.md` (line ~275)
   - `~/.codex/skills/beads-to-execution/CHECKLIST.md` (line ~184)
   - `~/.codex/skills/beads-to-execution/WORKFLOW.md` (line ~275)
3. **`am --help`** — verify the Rust binary's actual subcommand for serving HTTP. The expected answer is `am serve-http`. If it differs (e.g., a future `am 0.3.x` renames it), use whatever the binary actually exposes.

After reading, you should be able to answer: *what is the Rust-binary equivalent of `python -m mcp_agent_mail.http --port 8765`?* Verify before writing the patch.

---

## 4. The Edit

The stale line in each file:

```bash
cd apps/mail && uv run python -m mcp_agent_mail.http --port 8765
```

The replacement:

```bash
am serve-http --port 8765
```

Add a brief surrounding comment to make the new context clear:

```bash
# If the server is stale, restart the Rust binary:
pkill am 2>/dev/null || true
am serve-http --port 8765 &
# Verify:
curl -s http://127.0.0.1:8765/healthz
```

The exact replacement is at your discretion as long as:
- The Python invocation is GONE
- The replacement uses `am serve-http`
- A reader can copy-paste the block to actually restart the server
- The same content lands in BOTH CLIs' versions of the file

---

## 5. Acceptance Criteria

- [ ] All 4 files edited
- [ ] No remaining `mcp_agent_mail.http`, `apps/mail`, or `uv run python -m` references in either skill dir:
  ```bash
  rg "mcp_agent_mail\.http|apps/mail|uv run python -m mcp_agent_mail" ~/.claude/skills/ ~/.codex/skills/
  # Should return zero matches
  ```
- [ ] Both CLI installations still have IDENTICAL content for the edited files (per the dual-CLI symmetry rule from Phase A):
  ```bash
  diff ~/.claude/skills/beads-to-execution/CHECKLIST.md ~/.codex/skills/beads-to-execution/CHECKLIST.md
  diff ~/.claude/skills/beads-to-execution/WORKFLOW.md  ~/.codex/skills/beads-to-execution/WORKFLOW.md
  # Both should return empty (identical) — unless pre-edit asymmetry already existed; in that case
  # leave the asymmetry alone but ensure your edit is applied to BOTH files identically.
  ```
- [ ] `~/.claude/skills/` AND `~/.codex/skills/` are otherwise untouched (no incidental changes)
- [ ] Project repo is untouched — this work happens entirely in `~/.{claude,codex}/skills/`
- [ ] No new beads created
- [ ] No new files in the project repo

---

## 6. Verification (run before reporting done)

```bash
# Verify no stale references remain anywhere
rg "mcp_agent_mail\.http|apps/mail|uv run python -m mcp_agent_mail" ~/.claude/skills/ ~/.codex/skills/

# Verify the new content is present in all 4 files
for cli in claude codex; do
  for f in CHECKLIST.md WORKFLOW.md; do
    echo "--- ~/.${cli}/skills/beads-to-execution/${f} ---"
    grep -A 2 "am serve-http" ~/.${cli}/skills/beads-to-execution/${f} | head -4
  done
done

# Cross-CLI symmetry confirmed
diff ~/.claude/skills/beads-to-execution/CHECKLIST.md ~/.codex/skills/beads-to-execution/CHECKLIST.md && echo "CHECKLIST OK"
diff ~/.claude/skills/beads-to-execution/WORKFLOW.md  ~/.codex/skills/beads-to-execution/WORKFLOW.md  && echo "WORKFLOW OK"

# Project repo not touched
cd /Users/alinaaitova/Desktop/Testproject1
git status --porcelain | grep -v "^?? " || echo "(working tree clean of modified tracked files)"
```

If any verification fails, fix before reporting.

---

## 7. Commit + Push

This is a single tiny commit. The skills directories live OUTSIDE the project repo, so there's nothing to commit to `main` — the changes are local-only to your machine's skill installation. **No git activity in the project repo.**

Confirm:

```bash
cd /Users/alinaaitova/Desktop/Testproject1
git status
# Should still be clean — skill dirs are not part of this repo
```

If your skill dirs are version-controlled in some OTHER repo (some users keep `~/.claude/skills/` in a personal dotfiles repo), follow that repo's convention. Most users do not — in which case there's nothing to commit.

---

## 8. Report Format When Done

```
Stale Python references cleanup complete.

Files edited (4 total):
  ~/.claude/skills/beads-to-execution/CHECKLIST.md (line N)
  ~/.claude/skills/beads-to-execution/WORKFLOW.md  (line N)
  ~/.codex/skills/beads-to-execution/CHECKLIST.md  (line N)
  ~/.codex/skills/beads-to-execution/WORKFLOW.md   (line N)

Replacement applied: `am serve-http --port 8765`
Verification: zero remaining `mcp_agent_mail.http` / `apps/mail` references.
Cross-CLI symmetry: identical content across Claude/Codex versions.
Project repo: untouched.

Next handoff queued: prompts/install-bead-wrappers-skill-handoff.md
```

---

## 9. Critical Rules

1. **DO NOT modify any project file.** This work is entirely in `~/.{claude,codex}/skills/`.
2. **DO NOT create new files.** Edit in place.
3. **DO maintain dual-CLI symmetry.** Both CLI dirs must end with the same content for the edited files.
4. **DO NOT scope-creep.** If you find OTHER stale references (e.g., to `mcp_agent_mail` in some unrelated context), note them in your report but do not fix them in this handoff. Each fix gets its own scoped change.
5. **NEVER stash other agents' work** — if any of your skill files were modified by another agent during this session, treat their changes as legitimate; merge gracefully or ask before overwriting.
6. **NEVER use `git reset --hard` or `git push --force`** — see AGENTS.md "Irreversible Git Actions".
