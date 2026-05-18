# Handoff — Scout, Isolate, Revert, Test & Push: Remove Zack + Elias Contributions (2026-05-18)

> **Operator note (Alina, 2026-05-18):** Two non-owner contributors appear in this repo's history. **Zac Farrell** (`zfarrell54@gmail.com`) made 43 commits between 2026-05-09 and 2026-05-17 including production code; his authorization has been **revoked**. An even earlier reference to **Elias** (`/Users/elias/Desktop/govt_scraping_project`) appears in one prompt — possibly the project's original name before rename, possibly another person. **Mission: surgically remove every line and every file that Zack or Elias touched, restore each affected file to its last Alina-authored state (or pre-collaborator state where Alina never touched it), verify the codebase still builds and tests pass, then commit and push to `origin`.** Reverting these commits should also auto-fix the AgentMail `project_key` in `manifest.yaml` (Zack edited it) and other zacfarrell-path artifacts as a side effect.

> **Strict NON-goals:** Do NOT start L5 work. Do NOT pop `stash@{0}` (that is Handoff 2's job — but if `stash@{0}` is itself authored by Zack, see §3.4). Do NOT touch `skills/auth-signin/` (Handoff 2 owns it). Do NOT create `bin/ensure-cdp-9222` (Handoff 2 owns it). Do NOT `git revert` Zack's commits as separate "undo" commits — use file-level restoration so the resulting history is a single clean cleanup commit, not 43 noisy reverts.

> **Authorized destructive operations (with verbatim operator authorization — see §0.1):** You ARE authorized to (a) `git checkout <SHA> -- <file>` to restore files to prior states, (b) `git rm <file>` to delete files Zack created (per RULE 1 of AGENTS.md, this requires express permission — that permission is granted verbatim in §0.1 below), (c) `git push origin main` after all verifications pass. You are NOT authorized to `git reset --hard`, `git clean -fd`, `rm -rf` (these are absolutely forbidden in AGENTS.md), `git push --force`, or rewrite history with rebase/filter-branch — operator wants a forward-only fix.

---

## 0. Required context

Read these in order before touching anything:

1. This handoff (you're here, including §0.1 deletion authorization below)
2. `/Users/alinaaitova/Desktop/Testproject1/AGENTS.md` — 739 lines, **the single source of truth** for all agent rules in this repo (RULE 0 = operator override, RULE 1 = no file deletion without permission, RULE 2 = bead claim discipline, forbidden commands, code-editing discipline). `CLAUDE.md` is a one-liner that points here.
3. `git log --all --author="Zac Farrell" --format='%h | %ad | %s' --date=short` — full list of his 43 commits
4. `docs/handoff-2026-05-17-cdp-stack-validation-test.md` (~900 lines) — the existing CDP validation test handoff, which Handoff 2 will run after you. Includes a "Rollback" section that informs your backup strategy.

You are not allowed to start work until you have read item 2 (AGENTS.md) in full. Note: there is NO `MEMORY.md` file in this project — the operator's persistent memory lives in their Claude Code session, not in the repo. Do not look for it.

### 0.1 Verbatim operator authorization for file deletions (REQUIRED — quotes AGENTS.md RULE 1 satisfaction)

AGENTS.md RULE 1 forbids you from deleting any file "without express permission". The operator (Alina Aitova) provided that permission in writing on 2026-05-18, verbatim:

> *"For the first hand off, I would like you to just revert everything any other user has touched, be it Zach or Elias. I want you to review everything, find and trace every piece of code, every line, every single item they touched, and revert it to the state it was before they touched it."*

This authorizes:
- `git checkout <SHA> -- <file>` to restore Alina-touched or pre-collaborator file states (R1, R2).
- `git rm <file>` to delete files where the only author was Zack or Elias (R3).

This does NOT authorize:
- Deletion of any file Alina ever authored (even partially).
- Deletion of files outside the Zack/Elias contribution scope.
- Use of `rm -rf` or other forbidden commands.
- Deletion based on guesses about authorship — every R3 file must have its sole-author status verified in §3.2.

If you find a file you believe is R3 but you have any doubt, surface it as REVIEW in the audit doc and ask the operator before deletion.

---

## 1. Verified pre-work state (already gathered for you)

### 1.1 Git authors in this repo

```
Alina Aitova <alinaaitova@Mac.fritz.box>
Alina Aitova <alinaaitova@Mac.mynet>
Alina Aitova <alinaaitova@MacBook-Air-Alina.local>
Zac Farrell <zfarrell54@gmail.com>            ← non-owner, authorization revoked 2026-05-17
```

### 1.2 Revert classes — what to do per Zack-touched file

Zack touched 43 files (see `git log --all --author="Zac Farrell" --name-only --format='' | sort -u`). For each file, decide its **revert class** by looking at the git log:

| Class | Condition | Action |
|---|---|---|
| **R1: Alina-restored** | Alina ALSO touched the file (in commits other than Zack's) | `git checkout <last-Alina-commit-on-file> -- <file>` — restores to the most recent Alina-authored state; all of Alina's work on this file is preserved, all of Zack's edits are removed |
| **R2: Pre-Zack-restored** | Only Zack touched the file AFTER creation, but the file existed BEFORE Zack's first commit | `git checkout <commit-before-Zack-first-touched-it> -- <file>` — restores to the version that existed before Zack got involved |
| **R3: Delete** | Zack CREATED the file (no commits by anyone touch it before Zack) AND it is not a forensic/incident record | `git rm <file>` — deletes it entirely (authorized per §0.1) |
| **R4: Forensic carve-out — KEEP with operator decision** | Zack-created, but the file CONTENT documents a real incident (error manifest, blocker pause record, agent-mail database recovery log, an outage timeline). Examples: `prompts/machine-fork-e2e-*-error-manifest.md`, `prompts/machine-fork-e2e-*-blocker-*.md`, anything titled "record [X] blocker" / "pause" / "timeout". | **Do NOT delete by default.** Instead, surface ALL R4 candidates as a single decision list in the audit doc under `## R4 forensic-doc candidates — operator decision required`. Operator reviews and chooses keep/delete per file. **You wait for that decision before touching any R4 file.** |

How to decide R3 vs R4 for a Zack-created docs file:
- If the file is a **handoff** (prospective instructions for future agent work) — **R3, delete**.
- If the file is an **error manifest** documenting what failed on a specific date — **R4, surface for operator decision**.
- If unclear, default to R4 (safer — operator decides).

You produce a `## Revert plan` table in `docs/audit-2026-05-18-zack-elias-provenance.md` with one row per Zack-touched file listing its class and the SHA being restored (or `—` for R3, or `OPERATOR DECISION PENDING` for R4). Operator reviews the plan BEFORE you execute (see §3.4 final paragraph for the explicit STOP-and-await-operator-review gate).

**Side-effect to expect:** since Zack edited `docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml` (which contains the AgentMail `project_key`), the R1 restoration will likely revert line 29 from `/Users/zacfarrell/Desktop/Testproject1` to `/Users/alinaaitova/Desktop/Testproject1` automatically. Verify in §5 that this happened; if not, fix it via the **Edit tool** (manual edit, NOT sed — per AGENTS.md "No Script-Based Changes" rule, §5.6 below).

### 1.3 Elias scope

Only **one** file currently mentions Elias: `prompts/stages-1-to-9-full-pipeline-validation-handoff.md` (4 references: lines 64, 85, 88, 131; path `/Users/elias/Desktop/govt_scraping_project` — note the project name `govt_scraping_project` differs from `Testproject1`, suggesting a predecessor project or another machine's working dir). But the operator's directive is **scout EVERY change Elias touched**, not just path strings. So during scout (§3.3) you will also:

- Check `git log --all --author='Elias' --author='elias'` for any commits attributed to him (the email/name may vary).
- Check `git log -- prompts/stages-1-to-9-full-pipeline-validation-handoff.md` for who created and modified that file.
- Apply the same R1/R2/R3 revert class logic to that file: if Alina authored most of it and Elias's contribution is just the 4 path lines, treat as R1 (restore last Alina version) — but if Elias created the file, treat as R3 (delete).
- If the file is R3-deletable, confirm no other file references it (e.g., via `grep -rl "stages-1-to-9-full-pipeline-validation-handoff"` in docs/prompts/) before deletion.

### 1.4 Stash status — already verified Alina's; do NOT consume

`stash@{0}` exists ("freightos-build-temp-stash"). It is verified **clean of zacfarrell paths** in its diff. Handoff 2 will pop it. **You will not consume the stash and you will not drop it.**

Provenance facts (already verified — DO NOT re-check via `git log stash@{0}` because `git stash` always records "git stash" as the commit author, so that check is uninformative):

- **Parent commit**: `ed82602` — "freightos test report: record commit SHA + final inventory" — authored by Alina Aitova.
- **Stash created at**: 2026-05-17 23:07:26 +0200 (timestamp on the stash commit, after Zack's last commit on the same day).
- **Stash name**: "freightos-build-temp-stash" — named by Alina during the Freightos skill build session (per project context — Freightos was Alina's work, not Zack's).
- **Content**: 1573 lines spanning AGENTS.md (CDP sections), crates/daemon/src/main.rs (ensure_cdp_9222 daemon integration), crates/daemon/tests/skill_lint.rs + cdp_ensure_helper.rs + auth_signin_lint.rs (3 daemon test files), 16 SKILL.md updates wiring `bin/ensure-cdp-9222`, L5 pipeline doc updates, AND untracked items including the helper script itself (`bin/ensure-cdp-9222` blob a6b6b04e, mode 100755 executable) plus the two missing CDP handoff docs (`docs/handoff-2026-05-17-cdp-9222-auto-launch.md`, `docs/handoff-2026-05-17-cdp-auto-signin.md`).
- **Verdict**: stash is **Alina's CDP-stack work**. Leave it untouched for Handoff 2.

Record the fact "Stash provenance: confirmed Alina's CDP-stack work per pre-verified facts in §1.4 — preserved for Handoff 2" in the audit doc under `## Stash provenance`. No new investigation needed.

### 1.5 Git config

`git config --get user.name` returned blank in interactive shell, but recent commits show `Alina Aitova <alinaaitova@*>` — config is likely set at `--local` or a per-commit author override. **Verify and fix before committing**: see §4.

### 1.6 Remote

`origin = https://github.com/DSXD888/Testproject1.git`. Zack pushed here. You ARE authorized to push your cleanup commit (see §7) — but only AFTER §6 verification passes 100% green.

---

## 2. Goal & success criteria

### 2.1 Goal

Every line and every file Zack or Elias touched is removed from the working tree (preserving Alina's other work on the same files via R1 restoration). The codebase still builds, all existing tests pass, and the only `/Users/zacfarrell/` or `/Users/elias/` references that remain in tracked files are in **forensic records that explicitly document those contributors' actions** (e.g., `docs/handoff-2026-05-05-mcp-agent-mail-db-recovery.md` — incident log of Zack's mcp-agent-mail bug). The cleanup commit is pushed to `origin/main`. The L5 pipeline manifest's AgentMail `project_key` is back to `/Users/alinaaitova/Desktop/Testproject1` as an automatic side-effect of the R1 restoration.

### 2.2 Success criteria (ALL must be green before push)

- [ ] **Revert plan committed to audit doc:** `docs/audit-2026-05-18-zack-elias-provenance.md` contains a `## Revert plan` table with one row per Zack-touched file (file path, class R1/R2/R3, restore SHA, post-restore status). Operator reviewed.
- [ ] **Per-commit audit committed to audit doc:** same doc contains a `## Zack commit log` table with all 43 commits tagged `OK` / `REVIEW` / `SUSPICIOUS`. Any SUSPICIOUS commits triggered an immediate STOP (see §3.1) — handoff does not proceed without operator authorization.
- [ ] **Elias scope resolved:** audit doc has `## Elias finding` section recording authorship, file disposition, and post-revert state of `prompts/stages-1-to-9-full-pipeline-validation-handoff.md`.
- [ ] **Stash provenance recorded:** audit doc has `## Stash provenance` section; if Zack-authored, stash dropped; if Alina-authored, stash preserved for Handoff 2.
- [ ] **Path purity:** `git grep -nE '/Users/zacfarrell|/Users/elias' | grep -vF "$(cat docs/audit-2026-05-18-zack-elias-provenance.md | grep -oE '[a-z0-9/_-]+\.md' | sort -u)"` returns **0 lines outside the documented forensic-exception list**.
- [ ] **Build green:** `cargo build --workspace` exits 0.
- [ ] **Tests green:** `cargo test --workspace` exits 0. Any pre-existing test failure unrelated to the revert is identified in the audit doc as a known-failure baseline (not a revert-caused regression).
- [ ] **Daemon-specific tests green:** `cargo test -p daemon` exits 0.
- [ ] **AgentMail health check passing:** invoke MCP tool `mcp__mcp-agent-mail__health_check` and ALSO run `am guard status /Users/alinaaitova/Desktop/Testproject1` if the `am` binary is on PATH; record both outputs in audit doc `## AgentMail health` section. Both must report healthy.
- [ ] **Single cleanup commit:** one commit titled `cleanup: revert all Zac Farrell + Elias contributions` (plus optionally a second commit for the audit doc itself). Author = `Alina Aitova <alinaaitova@...>`. No `--force`. No `--no-verify`. No history rewrite.
- [ ] **Pushed to origin/main:** `git push origin main` succeeded (non-force). `git status` shows clean tree synced with `origin/main`.

---

## 3. Scout phase (build the revert plan + commit audit, BEFORE touching any file)

### 3.1 Per-commit content audit — 43 commits

For each commit Zack authored, classify into one of three tags:

- **OK** — pure docs/handoffs/error manifests; reasonable content; no concerning patterns.
- **REVIEW** — touches production code (`crates/`, `apps/`, `skills/`) OR introduces test changes. Operator should re-read these before relying on them.
- **SUSPICIOUS** — anything that looks like a backdoor, credential exfiltration, weakening of security/auth, disabling of safety checks, embedded URLs to external endpoints, hard-coded API keys, modifications to `.gitignore`/`.git*` that hide files, or any change to wallet/financial/auth-related code.

How to do it efficiently:

```bash
# Step 1: get the full commit list
git log --all --author="Zac Farrell" --format='%h|%ad|%s' --date=short > /tmp/zack-commits.txt

# Step 2: for each commit, capture a one-line digest of files + insertion magnitude
# (note: --shortstat can show "0 insertions" or omit the field entirely for some commits — defaults to 0)
while IFS='|' read -r sha date subject; do
  files=$(git show --name-only --format='' "$sha" | grep -v '^$' | tr '\n' ',' | sed 's/,$//')
  shortstat=$(git show --shortstat --format='' "$sha" | tail -1)
  ins=$(echo "$shortstat" | grep -oE '[0-9]+ insertion' | grep -oE '[0-9]+' || echo 0)
  del=$(echo "$shortstat" | grep -oE '[0-9]+ deletion'  | grep -oE '[0-9]+' || echo 0)
  echo "$sha | $date | +${ins:-0}/-${del:-0} | $subject"
  echo "    files: $files"
done < /tmp/zack-commits.txt > /tmp/zack-digest.txt
```

For each commit, decide its tag by reading the diff:

```bash
git show <sha> | less    # read the actual diff before tagging
```

**Tagging rules** (commit a tag, not a vibe):

- Touches only `docs/`, `prompts/`, `pipemaxxer/`, or `harmonymaxer/` → **OK** (default), unless content has suspicious patterns.
- Touches `crates/daemon/src/main.rs` or `apps/desktop/src-tauri/src/*` → **REVIEW** by default. Read the diff. If it's a clean fix matching the commit message, keep REVIEW. If anything is off, escalate to SUSPICIOUS.
- Touches `.gitignore` to add patterns that hide directories → **SUSPICIOUS**.
- Touches authentication, credentials, secrets handling, environment variable readers → **SUSPICIOUS**.
- Introduces network calls to unknown hosts → **SUSPICIOUS**.

Write each row to `docs/audit-2026-05-18-zack-elias-provenance.md` under a `## Zack commit log` heading. Format:

```markdown
| SHA | Date | Lines | Tag | One-line note |
|---|---|---|---|---|
| 37de717 | 2026-05-17 | +X/-Y | OK | docs only — stage 9 fix handoff |
| 28b6298 | 2026-05-17 | +X/-Y | REVIEW | crates/daemon/src/main.rs +X — semantic lot canonicalization fix |
| ... |
```

If ANY commit lands on **SUSPICIOUS**, stop, do NOT commit your work, and post a `### Suspicious commit findings` section in the audit doc with the diff snippet + your reasoning, then return to the operator.

### 3.2 Cross-author file-overlap audit

Run:

```bash
# For each file Zack ever touched, who else touched it?
git log --all --author="Zac Farrell" --name-only --format='' | sort -u | grep -v '^$' > /tmp/zack-files.txt
while read -r f; do
  authors=$(git log --all --format='%an' -- "$f" | sort -u | tr '\n' ',' | sed 's/,$//')
  echo "$f :: $authors"
done < /tmp/zack-files.txt > /tmp/file-authors.txt
```

Any file Zack is the **sole author** of is **R3 — delete**. Sub-classify for the operator's awareness:
- (a) Documentation/handoffs he authored solo → R3 delete (Zack's docs are not load-bearing for Alina's work; deletion removes his footprint cleanly).
- (b) Production code he authored solo → R3 delete (this is the high-risk case — if Alina relied on the code, the §5b build will fail and you'll need to either repair OR add to "Build-required Zack contributions" exceptions). Flag every (b) row in the audit under `## Solo-Zack-authored production files`.

Add the full file/author/disposition analysis to the audit doc as a `## Cross-author file overlap` section.

### 3.3 Elias scope + revert plan

Run:

```bash
git log --all --author='Elias' --author='elias' --format='%h | %an <%ae> | %ad | %s' --date=short
git log --format='%h | %an <%ae> | %ad | %s' --date=short -- prompts/stages-1-to-9-full-pipeline-validation-handoff.md
git grep -lE 'govt_scraping_project|/Users/elias'
git log --all --oneline | tail -20    # earliest commits — does project predate "Testproject1" name?
```

Decision table for `prompts/stages-1-to-9-full-pipeline-validation-handoff.md`:

| Authorship pattern | Action |
|---|---|
| Alina created, Elias never committed (his name only appears in path strings inside the file's content) | **R1** — restore to last Alina commit on the file (drops the elias path references) |
| Alina created AND modified, Elias also has commits on it | **R1** — restore to last Alina-non-Elias commit |
| Elias created the file | **R3** — `git rm prompts/stages-1-to-9-full-pipeline-validation-handoff.md` (unless another file references it; check with `grep -rl "stages-1-to-9-full-pipeline-validation-handoff" docs/ prompts/`) |

Record decision and reasoning under `## Elias finding` in the audit doc.

### 3.4 Build the per-file Revert Plan

From §3.2's file-authors data, classify every Zack-touched file into R1 / R2 / R3 / R4 and identify the restore SHA for R1/R2 files. **Run all three classification checks per file** to avoid mistakes:

```bash
# Step 1 — is the file's FIRST-EVER commit by Zack? (decides R3 vs R1/R2)
first_author=$(git log --reverse --format='%an' -- <file> | head -1)
echo "$file first author: $first_author"

# Step 2 — has Alina ever authored a commit on the file (non-Zack)?
alina_touched=$(git log --not --author='Zac Farrell' --format='%h' -- <file> | head -1)
echo "$file last non-Zack commit: ${alina_touched:-NONE}"

# Classification decision tree (apply in order):
#   first_author == "Zac Farrell" AND file looks like a handoff/instruction → R3 (delete)
#   first_author == "Zac Farrell" AND file is an error-manifest / incident record → R4 (operator decision)
#   first_author != "Zac Farrell" AND alina_touched != NONE → R1 (restore to alina_touched SHA)
#   first_author != "Zac Farrell" AND alina_touched == NONE → R2 (restore to pre-Zack SHA below)
#
# For R1, the restore SHA = the alina_touched commit:
git log --not --author='Zac Farrell' --format='%h | %an | %ad' --date=short -- <file> | head -1

# For R2, find the commit just before Zack's first touch:
zack_first=$(git log --reverse --author='Zac Farrell' --format='%h' -- <file> | head -1)
git log --format='%h | %an | %ad' --date=short "${zack_first}^" -- <file> | head -1

# For R3, no restore SHA needed. But VERIFY the "looks like handoff" heuristic by reading the file:
#   - Filename pattern *-handoff.md → R3 (prospective instructions)
#   - Filename pattern *-error-manifest.md / *-blocker-*.md / *-pause-*.md → R4 (forensic — operator decides)
#   - Other docs created solo by Zack → R4 by default (safer; operator decides)

# For R4, no restore SHA. Add to operator-decision list.
```

Write the full plan to `docs/audit-2026-05-18-zack-elias-provenance.md` under `## Revert plan`:

```markdown
| File | Class | Restore SHA (or — for R3, OPERATOR DECISION for R4) | Expected post-revert state |
|---|---|---|---|
| AGENTS.md | R1 | <SHA> | Alina's pre-Zack AGENTS.md content |
| crates/daemon/src/main.rs | R1 | <SHA> | Alina's pre-Zack main.rs (NOTE: §5b.1 build may break if Alina later code depends on Zack's fix) |
| docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml | R1 | <SHA> | side-effect: AgentMail project_key reverts to /Users/alinaaitova/ |
| prompts/contracts-finder-skill-build-handoff.md | R3 | — | deleted (Zack created, looks like prospective handoff) |
| prompts/machine-fork-e2e-procontract-enrich-timeout-error-manifest.md | R4 | OPERATOR DECISION | candidate for deletion; operator decides whether to preserve as incident record |
| ... |
```

In a separate section `## R4 forensic-doc candidates — operator decision required`, list each R4 file with a one-line summary of what incident it documents, so the operator can decide keep-vs-delete efficiently:

```markdown
| R4 file | Incident it records | Date | Suggested action |
|---|---|---|---|
| prompts/machine-fork-e2e-procontract-enrich-timeout-error-manifest.md | ProContract enrichment timeout during machine-fork E2E run | 2026-05-12 | keep (real outage, useful for debug) |
| prompts/machine-fork-e2e-stage-9-no-candidates-after-cdp-auth-error-manifest.md | Stage 9 no-candidates after CDP auth | 2026-05-17 | keep or delete? (operator) |
| ... |
```

**STOP and request operator review** of this plan before proceeding to §5. The operator MUST:
1. Approve the R1/R2 restore SHAs (or override them).
2. Decide keep-vs-delete for every R4 file.
3. Approve the R3 deletion list (per §0.1 authorization — but operator can override per-file).
4. Identify any "build-required Zack contributions" they want pre-kept (skips the §5b.1 break-and-repair loop).

Record their decisions inline in the Revert plan table (one decision column per row). Do not proceed to §5 until you have written confirmation from the operator.

---

## 4. Git config sanity check

Run:

```bash
git config --get user.name
git config --get user.email
git config --local --get user.name
git config --local --get user.email
git config --global --get user.name
git config --global --get user.email
```

Expected: `Alina Aitova` and `alinaaitova@...` somewhere (the recent commits show this works). If `git config --get user.email` returns blank in your shell, something is misconfigured. Do not commit until:

```bash
git config user.name 'Alina Aitova'
git config user.email 'alinaaitova@MacBook-Air-Alina.local'   # match the most recent commit's email
```

is set at `--local` scope. Record the before/after state in the audit doc under `## Git config sanity`.

---

## 5. Revert execution (after operator approves the §3.4 plan)

### 5.0 Create backup branch + tag BEFORE any destructive operation (MANDATORY)

This is the rollback safety net. Required by AGENTS.md "Mandatory explicit plan" + "Document the confirmation" sections of the Irreversible Git rules. Without this, there is no clean undo path if §5b reveals the revert went wrong.

```bash
# 5.0.a Backup branch — non-destructive snapshot of current main
git branch backup-2026-05-18-pre-revert main
git log -1 backup-2026-05-18-pre-revert --format='%h %s'   # confirm it points to current main HEAD

# 5.0.b Annotated tag with the operator's authorization quoted (per AGENTS.md "Document the confirmation")
git tag -a backup-2026-05-18-pre-revert -m "$(cat <<'EOF'
Backup snapshot before Zac Farrell + Elias revert (Handoff 1, 2026-05-18).

Operator authorization (verbatim, from §0.1 of docs/handoff-2026-05-18-zack-elias-cleanup.md):
"For the first hand off, I would like you to just revert everything any other user has touched,
be it Zach or Elias. I want you to review everything, find and trace every piece of code,
every line, every single item they touched, and revert it to the state it was before they touched it."

Rollback command (if §5b reveals the revert was wrong):
  git checkout backup-2026-05-18-pre-revert -- .
  (Restores ALL files to this snapshot. Requires operator authorization to execute per AGENTS.md.)
EOF
)"

# 5.0.c Verify both exist
git branch | grep backup-2026-05-18-pre-revert
git tag    | grep backup-2026-05-18-pre-revert

# 5.0.d (Optional but recommended) push the backup branch to origin so a hardware failure can't lose it
git push origin backup-2026-05-18-pre-revert
git push origin backup-2026-05-18-pre-revert --tags     # if your origin accepts tag pushes
```

Record both the branch and tag SHAs in the audit doc under `## Backup snapshot`. Operator can `git checkout backup-2026-05-18-pre-revert` at any time to recover.

> **Worktree option (recommended for first-time attempts):** if you want extra safety beyond the backup branch, do the revert work in a fresh git worktree so the main checkout stays intact even if you crash the process:
> ```bash
> git worktree add ../Testproject1.revert main
> cd ../Testproject1.revert
> ```
> Skip the worktree if you've already done a dry-run of the plan and are confident. The backup branch + tag from §5.0 is the primary safety net; the worktree is belt-and-suspenders.

### 5.1 Execute R1 restorations (file-by-file, one at a time)

For each R1 row in your plan, restore the file from the recorded SHA:

```bash
git checkout <restore-sha> -- <file>
# Spot-check: is this what you expected?
git diff HEAD -- <file> | head -40
```

Do NOT batch R1 restorations into a single command. The point of one-at-a-time is so that if one file's restoration breaks the build, you know exactly which one.

### 5.2 Execute R2 restorations (same pattern)

```bash
git checkout <restore-sha> -- <file>
git diff HEAD -- <file> | head -40
```

### 5.3 Execute R3 deletions (per §0.1 operator authorization)

R3 deletions are the only AGENTS.md RULE-1-relevant operation here. Re-read §0.1 before this step. The list of R3 files MUST exactly match the operator-approved list in the audit doc — not any superset, not any inferred list.

```bash
# 5.3.a — Compile the EXACT operator-approved R3 list (copy verbatim from audit doc's Revert plan rows where Class=R3 AND operator-decision=DELETE)
cat > /tmp/r3-approved.txt <<'EOF'
prompts/contracts-finder-skill-build-handoff.md
prompts/<other-approved-r3-files-from-audit-doc...>
EOF
wc -l /tmp/r3-approved.txt    # sanity: matches the R3 count in your audit doc

# 5.3.b — For EACH file: confirm it exists, confirm nothing else references it, then delete
while read -r f; do
  [ -f "$f" ] || { echo "SKIP: $f does not exist"; continue; }
  refs=$(grep -rl "$(basename "$f" .md)" docs/ prompts/ 2>/dev/null | grep -v "^$f$")
  if [ -n "$refs" ]; then
    echo "WARN: $f is referenced by: $refs — confirm operator still wants deletion"
    # If references found, re-confirm with operator before proceeding for THIS file.
  fi
  echo "Deleting: $f"
  git rm "$f"
done < /tmp/r3-approved.txt

# 5.3.c — Verify deletions are staged
git status --short | grep "^D " | wc -l   # should match operator-approved R3 count

# 5.3.d — Record each deletion in audit doc under `## R3 deletions executed` with timestamp
```

**R4 files: do NOT delete in §5.3.** For each R4 file the operator marked KEEP: do nothing (file stays as is, even though Zack authored it; this is per the operator's per-file decision). For each R4 file the operator marked DELETE: add it to the §5.3.a list and run it through the same gates.

### 5.4 Stash handling (if §1.4 found stash@{0} is Zack-authored)

```bash
git stash drop stash@{0}
git stash list                                # confirm empty
```

If Alina-authored: do nothing — leave the stash for Handoff 2.

### 5.5 Path-purity sanity check

After all restorations and deletions:

```bash
# What zacfarrell / elias references remain? Should be ONLY documented forensic files.
git grep -nE '/Users/zacfarrell|/Users/elias' | tee /tmp/remaining-refs.txt

# Compare against the exceptions list in your audit doc:
# - docs/handoff-2026-05-05-mcp-agent-mail-db-recovery.md (forensic incident record)
# - docs/session-log-2026-05-05.md (forensic session log)
# - docs/planmaxer/archived/* (frozen historical iterations — only if Alina-authored)
# Any reference NOT on that list is a missed file. Fix by adding it to the revert plan and re-restoring.
```

### 5.6 manifest.yaml line 29 (operator-flagged side-effect check)

After R1 restoration of `docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml`:

```bash
grep -n "project_key" docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml
# Expected: project_key: /Users/alinaaitova/Desktop/Testproject1
```

If it still shows `/Users/zacfarrell/`, the restore SHA you picked was wrong (it pre-dated when Alina set the path correctly, OR Alina herself set it wrong somewhere).

**Fix via the Edit tool — DO NOT use `sed`.** AGENTS.md "No Script-Based Changes" forbids script-based transformations: *"NEVER run a script that processes/changes code files in this repo. Brittle regex-based transformations create far more problems than they solve."*

Process:
1. `Read` the manifest.yaml file (find the bad line).
2. Use `Edit` tool with `old_string=<line containing /Users/zacfarrell/...>` and `new_string=<same line with /Users/alinaaitova/...>`. One occurrence at a time.
3. Repeat for each occurrence (likely just line 29 — the AgentMail `project_key`; possibly also the slug `users-zacfarrell-desktop-testproject1` if present).
4. Re-verify:
   ```bash
   grep -nE "zacfarrell|users-zacfarrell" docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml
   # Expected: no matches
   ```

Record in audit doc under `## manifest.yaml fix` whether this explicit fix was needed and which lines were Edit-tool'd.

---

## 5b. Test phase (BEFORE commit, BEFORE push — revert may have broken the build)

This is the gate the user explicitly asked for. The codebase MUST still work after the revert. If Alina built features on top of Zack's bug fixes, restoring her files to pre-Zack state could break things. You must verify and either repair or roll back.

### 5b.1 Build + lint + format (AGENTS.md "Compiler Checks" — MANDATORY)

Per AGENTS.md "Compiler Checks (CRITICAL)": *"After any substantive code changes, you MUST verify no errors were introduced."* Run all four in order:

```bash
# 5b.1.a Formatting (fast — must pass before anything else)
cargo fmt --check 2>&1 | tee /tmp/cargo-fmt.log
[ "${PIPESTATUS[0]}" = "0" ] && echo "✅ cargo fmt --check" || { echo "❌ cargo fmt --check FAILED"; head -40 /tmp/cargo-fmt.log; }

# 5b.1.b Compiler check (all targets, not just bin)
cargo check --all-targets 2>&1 | tee /tmp/cargo-check.log
grep -q "Finished" /tmp/cargo-check.log && echo "✅ cargo check" || { echo "❌ cargo check FAILED"; tail -40 /tmp/cargo-check.log; }

# 5b.1.c Clippy (pedantic + nursery; -D warnings means warnings count as errors)
cargo clippy --all-targets -- -D warnings 2>&1 | tee /tmp/cargo-clippy.log
grep -q "warning:" /tmp/cargo-clippy.log && echo "❌ clippy warnings present (treat as errors)" || echo "✅ cargo clippy"

# 5b.1.d Build the daemon binary
cargo build -p daemon 2>&1 | tee /tmp/cargo-build.log | tail -10
grep -q "Finished" /tmp/cargo-build.log && echo "✅ cargo build -p daemon" || { echo "❌ build FAILED"; tail -40 /tmp/cargo-build.log; }
```

If ANY of these fails:
- Read the error/warning carefully — what symbol/function/style is the problem?
- Was the offending code introduced by Zack? Check via pickaxe: `git log --all -S'<missing-symbol-text>' --format='%h %an %s'`.
- Two repair options (pick ONE per failure and document):
  - (a) **Keep Zack's contribution** as a documented exception. Run `git checkout HEAD -- <file>` to re-apply just that file. Record in audit doc under `## Build-required Zack contributions` with: file path, missing symbol, the Zack commit SHA, the reason Alina's code depends on it, and a **follow-up TODO for Alina to re-implement Zack-free later**.
  - (b) **Find an Alina re-implementation** in a later commit (rare — only if Alina happened to redo the same fix). Cherry-pick over.
- Re-run all four gates until green.

### 5b.2 Test (AGENTS.md "Stage Validation Test Gate" — MANDATORY)

Per AGENTS.md: *"`cargo test -p daemon --bin daemon` is **insufficient**. It only runs the binary unit tests and silently excludes the 39 integration test files in `crates/daemon/tests/*.rs`. Two prior Stage 11 manifests claimed GREEN under this narrow command while 13 integration tests were failing."*

Run the full gate (binary unit tests + every integration test file individually):

```bash
# 5b.2.a Binary unit tests
cargo test -p daemon --bin daemon 2>&1 | tee /tmp/test-bin.log | tail -10
grep -q "test result: ok" /tmp/test-bin.log && echo "✅ daemon bin tests" || { echo "❌ daemon bin tests FAILED"; tail -30 /tmp/test-bin.log; }

# 5b.2.b Integration test files — one per test target (per AGENTS.md)
fail_count=0
for t in crates/daemon/tests/*.rs; do
  name=$(basename "$t" .rs)
  # Skip env-dependent files that probe + auto-skip (per AGENTS.md they should self-skip, but tolerate it)
  cargo test --test "$name" 2>&1 > /tmp/test-$name.log
  if grep -q "test result: ok" /tmp/test-$name.log; then
    echo "✅ test/$name"
  else
    echo "❌ test/$name"
    fail_count=$((fail_count+1))
    tail -20 /tmp/test-$name.log
  fi
done
echo "Integration tests failed: $fail_count"
```

Triage failures the same way as §5b.1. Record kept-Zack files in `## Build-required Zack contributions`, pre-existing failures in `## Pre-existing test failures (not revert-caused)`. **Both lists must be empty for full green; non-empty lists block §6 push unless operator explicitly overrides each.**

### 5b.3 AgentMail health check (operator explicitly requested)

```bash
# 1. MCP tool — invoke directly via the MCP toolset (you have access to mcp__mcp-agent-mail__health_check)
# Capture the JSON output verbatim into the audit doc under "## AgentMail health"

# 2. CLI tool if available
which am 2>/dev/null && am guard status /Users/alinaaitova/Desktop/Testproject1 2>&1 | tee /tmp/am-status.txt
# Record the result. If `am` not on PATH: note that in audit doc, don't fail.

# 3. Wrapper sanity (project skill the user built specifically for this discipline)
ls -la bin/br-claim bin/br-complete bin/br-sync
# All three must exist and be executable. If missing after revert (e.g., Zack created them?):
# git log --all --follow -- bin/br-claim | head -5     # find the creating commit
# git checkout <SHA> -- bin/br-claim                    # restore if it was Alina's
```

Both health checks (MCP + CLI if available) must report healthy before commit. If either fails:
- The MCP server may be stuck on a stale project_key — having restored the manifest to `/Users/alinaaitova/` may have triggered a re-registration; check the MCP server logs.
- If `am guard status` reports the hook is not installed in this repo, that's a Handoff 2 problem (not a blocker here) — log it in `## Handoff 2 prerequisites`.

---

## 6. Commit + push

### 6.1 Stage your changes (DO NOT use `git add -A` or `git add .`)

```bash
# Stage only tracked files you modified or deleted in the revert (R1/R2/R3).
# `-u` updates tracked files only — it does NOT pull in untracked files, which is critical
# because Handoff 2's untracked items (skills/auth-signin/, docs/handoff-2026-05-17-*, docs/testing/,
# docs/handoff-2026-05-18-*.md, operator-built-skills/) MUST remain untracked for Handoff 2 to commit them later.
git add -u

# Also stage R3 deletions explicitly (`-u` covers this on most git versions, but be explicit):
# (no-op if no R3 deletes; harmless to run)
for f in <list-of-R3-deletions-from-your-plan>; do
  git rm "$f" 2>/dev/null || true
done

git status --short                          # verify: only Zack-touched files modified; untracked items still listed as ??
git diff --cached --stat | tail -20         # sanity-check the magnitude
```

If `git status` shows anything STAGED that you didn't expect (e.g., a modified file that wasn't in your revert plan, or any untracked item that snuck in), STOP and investigate — `git restore --staged <file>` to unstage.

Specifically confirm that **none** of these are staged (they belong to Handoff 2):
- `skills/auth-signin/`
- `docs/handoff-2026-05-17-cdp-*.md`
- `docs/handoff-2026-05-18-*.md`
- `docs/testing/`
- `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/`
- `bin/ensure-cdp-9222` (if it exists)

```bash
for f in skills/auth-signin docs/handoff-2026-05-17-cdp docs/handoff-2026-05-18 docs/testing docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills bin/ensure-cdp-9222; do
  git diff --cached --name-only | grep -q "^$f" && { echo "STOP: $f is staged but belongs to Handoff 2"; exit 1; }
done
echo "✅ Handoff 2 scope is untouched"
```

### 6.2 Commit (one commit for the revert + one for the audit doc — total of 2)

```bash
# Commit A: the revert itself
git commit -m "$(cat <<'EOF'
cleanup: revert all Zac Farrell + Elias contributions

Restores N files to their last Alina-authored state (class R1),
restores M files to pre-Zack state (class R2), deletes K files Zack created (class R3).
Side effect: AgentMail project_key in manifest.yaml back to /Users/alinaaitova/Desktop/Testproject1.

Per-file revert plan: see docs/audit-2026-05-18-zack-elias-provenance.md §"Revert plan".
Build verified: cargo build --workspace + cargo test --workspace + cargo test -p daemon all green.
AgentMail verified: mcp__mcp-agent-mail__health_check healthy.

Authorization for Zac Farrell <zfarrell54@gmail.com> revoked 2026-05-17 by operator.

Co-Authored-By: <your-model-id> <noreply@anthropic.com>
EOF
)"

# Commit B: the audit doc itself (separate so the revert commit is small/reviewable)
git add docs/audit-2026-05-18-zack-elias-provenance.md
git commit -m "$(cat <<'EOF'
docs: add Zac Farrell + Elias provenance audit (2026-05-18)

Records per-commit tags for all 43 Zack commits, revert plan with restore SHAs,
build/test/AgentMail health-check results, Elias finding, and any kept-Zack
file exceptions (with justifications).

Co-Authored-By: <your-model-id> <noreply@anthropic.com>
EOF
)"

git log --oneline -3
```

### 6.3 Push to origin (you ARE authorized)

```bash
# Sanity: confirm you're on main and up-to-date
git status
git rev-parse --abbrev-ref HEAD                            # expect: main
git fetch origin
git status                                                 # any "Your branch is behind origin/main" means STOP and rebase first

# Push (non-force — if origin has moved, rebase and re-verify build/test before pushing)
git push origin main
git log origin/main..HEAD                                  # expect: empty (push succeeded)
```

If push is rejected:
- It means someone else pushed to `origin/main` since you started. STOP. Do NOT `--force`. Pull, rebase, re-run §5b tests, then push again.

### 6.4 Final status block

Append to the audit doc:

```markdown
## Handoff 1 completion status
- Revert: ✅ N files restored (R1) + M files restored (R2) + K files deleted (R3)
- Build green: ✅ cargo build/test/test -p daemon all 0 exit
- AgentMail health: ✅ MCP healthy + `am guard status` healthy (or N/A)
- Suspicious Zack commits flagged: [count] (list below if any)
- Commit SHAs: <SHA-A revert>, <SHA-B audit doc>
- Pushed to origin/main: ✅ 2026-05-18T<time>
- Ready for: Handoff 2 (verify the 3 "missing" items, finalize CDP stack, push)
```

---

## 7. What you must NOT do

### 7.1 AGENTS.md absolutely-forbidden commands (these have NO authorization):

- ❌ **`git reset --hard`** — would discard Alina's intervening work too (§1.6 of AGENTS.md "Irreversible Git" rules).
- ❌ **`git clean -fd`** — same risk (per AGENTS.md "Absolutely forbidden commands").
- ❌ **`rm -rf`** anything — per AGENTS.md (use `git rm` only for R3-deletion files authorized in §0.1).
- ❌ **`git push --force`** or any history rewrite (rebase --interactive that drops commits, filter-branch, filter-repo).

### 7.2 Out-of-scope for this handoff (Handoff 2 territory):

- ❌ **Consume `stash@{0}`** — it's Alina's (§1.4 verified facts), it stays for Handoff 2.
- ❌ **Touch `skills/auth-signin/` content** — Handoff 2 will commit it. However, IF §3.2 cross-author analysis finds the auth-signin tree was actually Zack-created (unlikely — it's uncommitted on disk, no git history), treat as R3 with operator-decision per §3.4.
- ❌ **Create `bin/ensure-cdp-9222`** — Handoff 2's stash pop produces it (verified: it's blob a6b6b04e in the stash).
- ❌ **Start L5 work** or run any L5 pipeline step.

### 7.3 Process violations:

- ❌ **`git revert` Zack's commits as 43 separate undo commits.** Use file-level R1/R2/R3 restoration so the result is one clean cleanup commit per §6.2.
- ❌ **`git commit --no-verify`** or skip pre-commit hooks (per AGENTS.md). If the AgentMail guard hook (`.git/hooks/hooks.d/pre-commit/50-agent-mail.py`) fails, FIX the underlying cause (likely manifest project_key wrong, see §5.6).
- ❌ **`sed -i`** or other script-based file transformations (per AGENTS.md "No Script-Based Changes" — use Edit tool).
- ❌ **`git add -A` or `git add .`** — see §6.1 for the precise staging approach.
- ❌ **Skip §5b test phase.** A clean build/test pass is a hard gate before push.
- ❌ **Delete an R4 file without operator decision.** Per §0.1 the authorization for deletion is conditional on the operator's per-file judgment for R4 candidates.

### 7.4 Bead-claim discipline (AGENTS.md RULE 2 clarification):

This work is **NOT bead-tracked**. AGENTS.md RULE 2 says *"For any bead-tracked work in this repo, you MUST use the project wrappers"* — this handoff is a one-off operator-authorized cleanup, not a bead. So you do NOT run `bin/br-claim` / `bin/br-complete`. The pre-commit AgentMail guard hook still applies (it runs on every commit) — if it blocks you, surface the error.

If you discover during work that this *should* have been bead-tracked, STOP and ask operator before proceeding.

---

## 8. Failure modes & recovery

| Failure | Recovery |
|---|---|
| Audit finds **ANY** SUSPICIOUS commit | STOP. Do not execute revert. Write a `### SUSPICIOUS commit findings` section in the audit doc with the diff snippet + your reasoning + recommended action, then return to operator. Operator authorizes before you continue. |
| `cargo build` fails after revert because Alina depended on Zack's code | Identify the load-bearing Zack file via `git log --all -S<missing-symbol>`. Either (a) restore that single file with `git checkout HEAD -- <file>` and log as a kept-Zack exception in the audit, OR (b) ask operator if Alina re-implemented it in a later commit you can cherry-pick over. Do NOT mix: pick one path and document. |
| `cargo test` shows a NEW failure (i.e., wasn't failing before you started) | Same triage as above — load-bearing Zack contribution. Repair OR keep-with-exception. |
| `cargo test` shows a PRE-EXISTING failure unrelated to the revert | Run `git stash; git checkout main; cargo test -p daemon <failing-test>` to confirm it was failing before; record as known baseline in audit `## Pre-existing test failures`. Not a blocker for push. |
| MCP `mcp__mcp-agent-mail__health_check` fails after revert | Likely the MCP server cached the old `/Users/zacfarrell/` project_key. Restart the MCP server (or just retry after a few seconds — it may auto-recover when daemon re-registers). If still failing: check the MCP server log for the actual error before continuing. |
| `am guard status` reports hook not installed | Not a blocker for Handoff 1 — log in `## Handoff 2 prerequisites`. Handoff 2 can install the hook via `am guard install`. |
| Git config blank everywhere | Set `--local` config in this repo (see §4) before committing. |
| `git push origin main` rejected (non-fast-forward) | Origin moved since you started. `git fetch origin; git rebase origin/main`. Re-run §5b (build/test/AgentMail). Then push. Do NOT `--force`. |
| Worktree (if used) ends with conflicts or weird state | `cd` back to main checkout, `git worktree remove --force ../Testproject1.revert`, start over in main checkout (less safe but viable). |
| You realize you missed a file (path-purity check at §5.5 finds an undocumented reference) | Add it to the revert plan, decide R1/R2/R3, execute, re-verify. Update the audit doc. |
| You modify a file by accident outside your scope | `git checkout -- <file>` immediately, log in audit doc. |

---

## 9. Estimated time

| Phase | Time |
|---|---|
| Read context (§0) | 15 min |
| Per-commit audit (43 commits × ~1 min to read diff) | 45–60 min |
| Cross-author file analysis + Revert Plan build (§3.4) | 30 min |
| Elias investigation | 10 min |
| Operator review of Revert Plan (waiting time) | 15–30 min |
| Revert execution (file-by-file) | 30 min |
| Test phase (§5b — build/test + AgentMail) | 20 min + any repair loop time |
| Commit + push | 10 min |
| Audit doc finalization | 10 min |
| **Total** | **~3–4 hours** |

---

## 10. When done

Print exactly this block to the operator (filling in the bracketed numbers):

```
HANDOFF 1 COMPLETE.

Revert: [N] files R1-restored + [M] files R2-restored + [K] files R3-deleted + [F] R4 files kept-per-operator.
Backup snapshot: branch=backup-2026-05-18-pre-revert tag=backup-2026-05-18-pre-revert (pushed to origin: [yes | no])
Side-effect: manifest.yaml project_key reverted to /Users/alinaaitova/[ + confirmed automatically | NOT confirmed — Edit-tool fix applied ].
Zack commit audit: 43 commits tagged. OK=[X], REVIEW=[Y], SUSPICIOUS=[Z].
Elias finding: [R1-restored | R3-deleted | not present in history] — see audit §"Elias finding".
Stash provenance: confirmed Alina's CDP-stack work (per §1.4 verified facts) — preserved for Handoff 2.
Build gate (AGENTS.md "Stage Validation Test Gate"):
  cargo fmt --check:        [✅ | ❌]
  cargo check --all-targets: [✅ | ❌]
  cargo clippy -D warnings:  [✅ | ❌]
  cargo test --bin daemon:   [✅ | ❌]
  per-integration tests:     [✅ all green | ❌ [P] failed]
Kept-Zack exceptions (build-required): [Q] files, listed in audit § "Build-required Zack contributions" with TODOs for Alina to re-implement.
AgentMail health: MCP [✅ healthy] + am guard status [✅ healthy | N/A — am not on PATH] + bin/br-* wrappers [✅ present | ❌ missing].
Git config: verified [OK | repaired].
Commit SHAs: [<SHA-A revert>] + [<SHA-B audit-doc>]
Pushed to origin/main: ✅ at 2026-05-18T[time]
Backup branch pushed: [✅ origin/backup-2026-05-18-pre-revert | NO — local only]

Handoff 2 (/goal docs/handoff-2026-05-18-goal-cdp-l5-ready.md) is now unblocked.
Audit doc for operator review: docs/audit-2026-05-18-zack-elias-provenance.md
```
