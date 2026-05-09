# Cleanup Stale Python References in `beads-to-execution` Skill

> **Paste this entire file into a fresh Claude Code / Codex session to onboard the implementation agent.**

---

## Context

During post-L4 forensic analysis (2026-05-08), 4 stale references to the OLD Python implementation of `mcp_agent_mail` were found in the `beads-to-execution` skill — across BOTH the Claude and Codex installations. The Python package was migrated to the Rust `am` binary, so these references would mislead any agent that hits server trouble and consults this guidance. Verify the live binary with `am --version` and `am --help`; do not hardcode an expected version.

**The references are in TROUBLESHOOTING sections only**, so they don't affect the happy path of mail-disciplined bead claiming. But they pollute the recovery flow, and at least one of those file reads happens during routine session warmup.

**This is purely a documentation correction.** No functional code changes. No behavior changes. ~5 minutes of work.

**Project key:** `/Users/zacfarrell/Desktop/Testproject1`
**Branch:** `main` only (no feature branches per AGENTS.md)
**Bead label:** N/A — this is documentation cleanup, not bead-tracked

---

## 1. Mission

Replace the stale Python invocation in 4 files (2 in Claude's skill dir, 2 in Codex's skill dir) with the equivalent Rust binary command. Maintain dual-CLI symmetry (the two skill installations should remain identical content after the edit).

---

## 2. AgentMail Registration

Not required — this work is entirely in `~/.{claude,codex}/skills/`, which is OUTSIDE the project repo. There are no commits to make in `Testproject1`, so the precommit hook never fires. If your skill dirs are themselves in a separate version-controlled repo, export both `AGENT_NAME` and `AGENT_MAIL_NAME` before committing.

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

### Current content of each file (verbatim — verify with your own `grep` before editing)

The stale block lives in a Troubleshooting section near the end of each file. Use `~~~` (tilde) fences below to avoid nested-backtick rendering issues:

~~~markdown
**DO NOT start executing beads. This is SETUP ONLY.**

---

## Troubleshooting

### Mail server not running
```bash
cd apps/mail && uv run python -m mcp_agent_mail.http --port 8765
```

### CLI schema mismatch
If CLI commands fail with "no such column" errors, use SQL instead. See WORKFLOW.md step 1.4.
~~~

WORKFLOW.md has the equivalent block (around line 275) under its own Troubleshooting heading.

**Verify the surrounding context yourself before editing** — run:

```bash
for cli in claude codex; do
  for f in CHECKLIST.md WORKFLOW.md; do
    echo "=== ~/.${cli}/skills/beads-to-execution/${f} ==="
    grep -n -B 2 -A 4 "mcp_agent_mail.http" ~/.${cli}/skills/beads-to-execution/${f}
  done
done
```

This shows you exactly what's around each stale line in all 4 files, so you can confirm the edit target before committing to the change.

### Required edit (apply identically to ALL 4 files)

Replace this block:
```bash
cd apps/mail && uv run python -m mcp_agent_mail.http --port 8765
```

With this block:
```bash
# Restart the Rust agent-mail binary (replaces the obsolete Python `mcp_agent_mail.http`):
pkill am 2>/dev/null || true
am serve-http --port 8765 &
# Verify:
curl -s http://127.0.0.1:8765/healthz
```

Apply the same replacement to all 4 files. Maintain dual-CLI symmetry (Claude/Codex versions identical post-edit).

### Why this exact replacement

- `am serve-http` is the verified Rust binary subcommand (run `am --help` to confirm; it's listed first under `Commands:`).
- `pkill am` defensively kills any stale instance (the user may have a previous server hanging around).
- The `2>/dev/null || true` swallows the "no process found" exit code so the line is safe even when no `am` is currently running.
- The `&` backgrounds the new server (the user can keep using their shell).
- The `curl /healthz` line gives the user immediate feedback that the new server is alive.

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
# A. Verify no stale references remain anywhere
rg "mcp_agent_mail\.http|apps/mail|uv run python -m mcp_agent_mail" ~/.claude/skills/ ~/.codex/skills/
# Expected: zero matches

# B. Verify the new content is present in all 4 files
for cli in claude codex; do
  for f in CHECKLIST.md WORKFLOW.md; do
    echo "--- ~/.${cli}/skills/beads-to-execution/${f} ---"
    grep -A 4 "am serve-http" ~/.${cli}/skills/beads-to-execution/${f} | head -6
  done
done

# C. Cross-CLI symmetry confirmed
diff ~/.claude/skills/beads-to-execution/CHECKLIST.md ~/.codex/skills/beads-to-execution/CHECKLIST.md && echo "CHECKLIST symmetric OK"
diff ~/.claude/skills/beads-to-execution/WORKFLOW.md  ~/.codex/skills/beads-to-execution/WORKFLOW.md  && echo "WORKFLOW symmetric OK"

# D. Project repo not touched
cd /Users/zacfarrell/Desktop/Testproject1
git status --porcelain | grep -v "^?? " || echo "(working tree clean of modified tracked files)"

# E. Defensive paranoia: existing wrappers still work post-edit
#    (Skill edits should NOT affect the wrappers, but verify anyway — costs nothing)
AGENT_NAME=verifier bin/br-claim --help    | head -3 && echo "✓ bin/br-claim still works"
AGENT_NAME=verifier bin/br-complete --help | head -3 && echo "✓ bin/br-complete still works"
```

If any verification fails, fix before reporting.

---

## 7. Commit + Push

This is a single tiny commit. The skills directories live OUTSIDE the project repo, so there's nothing to commit to `main` — the changes are local-only to your machine's skill installation. **No git activity in the project repo.**

Confirm:

```bash
cd /Users/zacfarrell/Desktop/Testproject1
git status
# Expected: no tracked Testproject1 changes caused by this handoff. Other
# agents may have unrelated changes; do not stash, revert, or delete them.
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
5. **No MCP tool calls needed.** This is pure file editing. You don't need to register with AgentMail, send messages, claim beads, or invoke any MCP tools. The whole task is local-disk text edits.
6. **CHECKPOINT after each phase.** After (a) reading the 4 files, (b) editing them, (c) running the verification suite — pause and report progress to the user before moving on. Even for a 5-min task, visible incremental progress is better than a single end-of-task report.
