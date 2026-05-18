# Handoff — `/goal` Verify, Fix, Test & Push: CDP Stack + L5 Ready (2026-05-18)

> **Invoke this with:** `/goal docs/handoff-2026-05-18-goal-cdp-l5-ready.md`
>
> **Sequencing:** This handoff runs **AFTER** `docs/handoff-2026-05-18-zack-elias-cleanup.md` has been committed AND pushed. If `git log -1` does not show a commit titled `cleanup: revert all Zac Farrell + Elias contributions`, STOP and run Handoff 1 first.

> **Operator (Alina, 2026-05-18):** Yesterday's CDP-stack work (helper-invocation wiring + Session Recovery for 8 production skills + daemon integration + AGENTS.md sections + lint test) is sitting in `stash@{0}` ("freightos-build-temp-stash") and was never popped back. I previously thought three other things were "missing" (the helper script `bin/ensure-cdp-9222`, the AgentMail path in manifest.yaml, and the `skills/auth-signin/` tree's git-tracking). **Now that Handoff 1 has done a full revert of Zack's commits, two of those three may already be resolved as side-effects** — the manifest path almost certainly is, and the helper might exist if it was in a pre-Zack commit you can recover. **Your first job after pre-flight is to RE-VERIFY each "missing" item rather than blindly assume it's missing.** Then: pop the stash if Handoff 1 didn't already drop it, fill any gaps that genuinely remain, validate the CDP stack end-to-end against live Chrome, fix anything broken in a tight `/goal` loop, run the AgentMail health check, commit in clean logical chunks, **push to `origin/main`**, and declare L5 ready to start. **Do not start L5.**

> **Hard non-goals:**
> - ❌ Do NOT execute L5 step 01 or any L5 pipeline work.
> - ❌ Do NOT touch the Freightos or Taric skills (`skills/landed-cost/freightos-freight-estimate/`, `skills/landed-cost/taric-cross-check/`) — they're already committed and passing.
> - ❌ Do NOT redo any of Handoff 1's revert work.
> - ❌ Do NOT modify `docs/audit-2026-05-18-zack-elias-provenance.md` (Handoff 1's output).
> - ❌ Do NOT `git reset --hard`, `git push --force`, or rewrite history.
> - ❌ Do NOT skip the §3 RE-VERIFICATION step. If you assume an item is missing without checking, you may waste hours recreating something that already exists.

> **Authorized operations (operator's directive, verbatim):**
>
> *"the first hand off, just going back to the previous commits before the other two collaborators pushed their commits, might fix the other two things, being the agent mail path as well as other stuff. The fact that neither handoff pushes to origin is wrong, so the first hand off should just look for Scout, isolate, revert, test, and make sure it's all good, and then just push. The second hand off should also just make sure everything is green and push."*
>
> Therefore you are authorized to: `git stash pop` (consume Handoff 1's verified-Alina stash), `git commit` (3 logical chunks per §10), `git push origin main` (non-force, after §9 + §10.4 gates pass). NOT authorized: `git reset --hard`, `git clean -fd`, `rm -rf`, `git push --force`, `git commit --no-verify`, `sed -i`, or any history rewrite — see §13.5 for the full forbidden-commands list.

---

## 0. The `/goal`

Stated as a single sentence the agent loops against:

> **All CDP-stack work is committed and pushed to `origin/main`, `cargo build --workspace` is green, `cargo test -p daemon skill_lint` is green, `bin/ensure-cdp-9222` reliably starts Chrome on port 9222 when not running and is a no-op when it is, all 8 production skills with Session Recovery survive a forced session expiry in live Chrome, the AgentMail health check passes via both MCP (`mcp__mcp-agent-mail__health_check`) and CLI (`am guard status` if available), and the L5 readiness checklist in §11 is 100% green — without any L5 pipeline step being run.**

The loop is: do the next step → verify it → if it fails, diagnose and fix → re-verify → only advance when verified. You may iterate any step as many times as needed.

---

## 1. Required reading (before any tool call)

**Critical disclosure**: two of the documents below live INSIDE `stash@{0}` (Alina's "freightos-build-temp-stash"), not on disk. You can read them via `git show <blob>` BEFORE popping the stash, or wait until §3 pop and read them on disk. Both options shown.

In this exact order:

1. **This handoff** (§0–§13).
2. **`docs/audit-2026-05-18-zack-elias-provenance.md`** — Handoff 1's output. Tells you: which files were reverted, any kept-Zack exceptions, the §"Stash provenance" confirmation that the stash is Alina's, and any pre-existing test failures noted as baseline. This is your starting-state reference.
3. **`/Users/alinaaitova/Desktop/Testproject1/AGENTS.md`** (739 lines, authoritative) — Especially: RULE 1 (no file deletion without express permission), RULE 2 (bead claim discipline — this handoff is NOT bead-tracked, see §13.6), "Irreversible Git" section (absolutely forbidden commands), "Code Editing Discipline / No Script-Based Changes" (no sed for file mutations), "Compiler Checks (CRITICAL)" + "Stage Validation Test Gate" (the test gate §6 uses).
4. **`docs/handoff-2026-05-17-cdp-stack-validation-test.md`** (953 lines, ON DISK already) — has 8 test areas A–H you'll run after stash pop. Has its own "Rollback" section (~line 900) you can fall back to if §6 reveals catastrophic bugs.
5. **`docs/handoff-2026-05-17-cdp-9222-auto-launch.md`** — **IN STASH** as blob `12e5f189` (file appears at this path after `git stash pop` in §3.3). Specifies the full design of `bin/ensure-cdp-9222` (which also appears from stash). Read BEFORE pop via: `git show 12e5f189 | less` — this is the helper script's spec/audit. After pop, read it normally on disk.
6. **`docs/handoff-2026-05-17-cdp-auto-signin.md`** — **IN STASH** as blob `af2fc720` (also appears after `git stash pop`). Specifies the auth-signin skill family (which lives on disk uncommitted). Read BEFORE pop via: `git show af2fc720 | less`.
7. **`prompts/install-bead-wrappers-skill-handoff.md`** — the operator-referenced "agent-mail skill" context (the portable bootstrap skill at `~/.claude/skills/install-bead-wrappers/` outside this repo); explains `am guard install` / `am guard status` and the `bin/br-*` wrapper discipline. AgentMail health check in §9.11 uses this context.

There is NO `MEMORY.md` in this project (operator's persistent memory lives in their Claude Code session). Do not look for it.

Estimated reading time: ~45 minutes (longer than usual because the auto-launch + auto-signin handoffs are non-trivial specs). Worth every minute — you will save 2× that in avoided rework, especially because Handoff 1's revert changed the starting state.

---

## 2. Pre-flight (verify state before doing anything)

Run all of these and stop if any check fails:

```bash
# 2.1 Handoff 1 committed AND pushed?
git log --oneline -5 | grep -qE "cleanup: revert all Zac Farrell|cleanup: revert all Zack Farrell" || { echo "STOP: Handoff 1 revert commit not found"; exit 1; }
git fetch origin
git rev-list HEAD..origin/main --count | grep -q '^0$' || { echo "STOP: HEAD is behind origin/main — rebase first"; exit 1; }
git rev-list origin/main..HEAD --count | grep -q '^0$' || { echo "STOP: HEAD is ahead of origin/main — Handoff 1 didn't push; have operator push first"; exit 1; }

# 2.2 Stash still present? (Handoff 1 may have dropped it if it was Zack-authored — check audit doc to know which case)
git stash list
# If Handoff 1's audit doc §"Stash provenance" says Alina-authored: stash@{0} MUST still be here.
# If Handoff 1's audit doc says Zack-authored: stash should be GONE; you'll skip §4 entirely.
# If neither matches reality, STOP and reconcile.

# 2.3 Working tree clean except for the expected untracked items?
git status --short
# EXPECTED untracked items (13) — verified ground truth as of 2026-05-18 pre-Handoff-1:
#   ?? docs/handoff-2026-05-17-cdp-stack-validation-test.md
#   ?? docs/handoff-2026-05-18-goal-cdp-l5-ready.md
#   ?? docs/handoff-2026-05-18-zack-elias-cleanup.md
#   ?? docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/HANDOFF-freightos-freight-estimate.md
#   ?? docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/HANDOFF-taric-cross-check.md
#   ?? docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/README.md
#   ?? docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/reference/
#   ?? docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/screenshots/
#   ?? docs/testing/2026-05-17-cdp-9222-auto-launch-test-report.md
#   ?? docs/testing/2026-05-17-cdp-auto-signin-test-report.md
#   ?? docs/testing/2026-05-17-cdp-stack-validation-test-report.md
#   ?? docs/testing/cdp-stack-validation-2026-05-17/
#   ?? skills/auth-signin/
#
# Items NOT in this list — they will appear AFTER §3 stash pop:
#   ?? bin/ensure-cdp-9222                                          (blob a6b6b04e, mode 100755 — in stash's untracked component)
#   ?? docs/handoff-2026-05-17-cdp-9222-auto-launch.md              (blob 12e5f189 — in stash's untracked component)
#   ?? docs/handoff-2026-05-17-cdp-auto-signin.md                   (blob af2fc720 — in stash's untracked component)
#   ?? crates/daemon/tests/cdp_ensure_helper.rs                     (blob a015a64d — in stash's untracked component; new daemon test)
#   ?? crates/daemon/tests/auth_signin_lint.rs                      (blob 38d1d748 — in stash's untracked component; new daemon test)
#
# DO NOT confuse "untracked on disk now" with "will appear after pop". The five items above are NOT on disk yet —
# they appear from the stash's untracked-files component (bfa127f). They DO NOT need to be created from spec
# in §4 — §4 verifies their post-pop presence and runs smoke tests.
#
# Differences vs the verified ground truth could mean:
#   - Handoff 1's revert deleted (R3) something that was in this list (consistent with §0.1 deletion authorization) — check audit doc § Revert plan
#   - A new file appeared from somewhere else (concerning — STOP and investigate)
# If you see anything UNEXPECTED, STOP and investigate before continuing.

# 2.4 Daemon currently builds?
cargo check -p daemon 2>&1 | tail -5
# Must show "Finished" with no errors. If errors, STOP — the revert may have left a broken baseline that Handoff 1
# should have caught in its §5b test phase. Check `docs/audit-2026-05-18-zack-elias-provenance.md` § "Build-required Zack contributions"
# — if Handoff 1 documented an exception you weren't told about, reconcile with operator before continuing.

# 2.5 Chrome state (will matter for §7)
pgrep -fl "remote-debugging-port=9222" || echo "(Chrome not running on 9222 — fine, we'll test launching)"

# 2.6 AgentMail health (the skill the operator referenced — verify it's healthy BEFORE pop/build)
# Invoke MCP tool: mcp__mcp-agent-mail__health_check  (use it directly — you have access)
# Record JSON result.
# Also if `am` CLI is on PATH:
which am 2>/dev/null && am guard status /Users/alinaaitova/Desktop/Testproject1 2>&1 | head -10
# Wrappers present?
ls -la bin/br-claim bin/br-complete bin/br-sync
# All three must exist and be executable. If any missing, Handoff 1's revert may have eaten them — STOP and reconcile.
```

Record the pre-flight state in a `## Pre-flight` section of your final report (see §12).

---

## 3. Stash pop (the big one) — but FIRST re-verify what's actually missing

This is where the bulk of yesterday's work comes back. The stash contains 1573 insertions across 28 files. Handle conflicts methodically — and **before you pop**, do §3.0 to confirm what gap actually remains. After Handoff 1's revert, the situation may differ from what the operator originally described.

### 3.0 RE-VERIFY THE THREE "MISSING" ITEMS — pre-pop state (verified facts + per-item plan)

The operator originally listed three things as "missing": (1) `bin/ensure-cdp-9222`, (2) the AgentMail `project_key` in manifest.yaml, (3) `skills/auth-signin/` not git-tracked. They later said: "I think they exist and they're fine — verify before assuming." The actual situation (already verified pre-Handoff-1 + post-Handoff-1) is:

**Item 1 — `bin/ensure-cdp-9222`** (verified pre-revert):
- Does NOT exist on disk in working tree.
- Does NOT exist in any committed branch.
- DOES exist in `stash@{0}` as blob `a6b6b04e`, mode `100755` (executable), pure-bash 60-line script with json_escape, env overrides, and diagnostic JSON.
- → After §3.3 stash pop, the file appears at `bin/ensure-cdp-9222` automatically. **No §4 "create from spec" needed.** §4 only verifies post-pop and runs smoke tests.

**Item 2 — manifest.yaml `project_key`** (post-revert side-effect):
- Pre-revert it was `/Users/zacfarrell/Desktop/Testproject1` (Zack edited it).
- Handoff 1's R1 restoration of manifest.yaml should have reverted it to `/Users/alinaaitova/Desktop/Testproject1`.
- → After Handoff 1, this should be ALREADY FIXED. Verify in §3.0.b below; if Handoff 1 missed it, do a **manual Edit-tool fix** (NOT sed — AGENTS.md "No Script-Based Changes").

**Item 3 — `skills/auth-signin/` git-tracking**:
- Tree exists on disk (6 sub-skills: alibaba, ebay, freightos, google-account-chooser, procontract, searates) with real selectors.
- Not yet committed (no git history for the tree).
- → §5 commits it.

```bash
# 3.0.a — bin/ensure-cdp-9222: confirm pre-pop state matches expectation
if [ -f bin/ensure-cdp-9222 ]; then
  echo "❓ UNEXPECTED: helper exists pre-pop. Recheck audit doc §Revert plan — did Handoff 1 restore it from somewhere?"
  ls -la bin/ensure-cdp-9222
else
  echo "✅ Expected: helper not on disk pre-pop; will appear from stash in §3.3"
fi

# 3.0.b — AgentMail project_key in manifest.yaml (Handoff 1 should have fixed this)
grep -n "project_key" docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml
# Expected after Handoff 1: project_key: /Users/alinaaitova/Desktop/Testproject1
# If still shows /Users/zacfarrell/:
#   1. Read the file with the Read tool to find the exact line.
#   2. Use the Edit tool: old_string=<line with /Users/zacfarrell/...>, new_string=<line with /Users/alinaaitova/...>.
#   3. DO NOT use sed (AGENTS.md "No Script-Based Changes" forbids it).
#   4. Flag in commit C message + audit-doc as "Handoff 1 missed manifest project_key fix; Handoff 2 corrected via Edit tool".

# 3.0.c — skills/auth-signin/ tracking
ls skills/auth-signin/ 2>&1 | head -10                  # confirm 6 sub-skills present (alibaba, ebay, freightos, google-account-chooser, procontract, searates)
git ls-files skills/auth-signin/ 2>&1 | head -5         # if empty: not yet tracked (the expected case — §5 will add)
# If git ls-files shows content but git status doesn't show ?? skills/auth-signin/:
#   it's already tracked and committed (unusual — verify what commit it landed in via `git log -- skills/auth-signin/`)

# Record findings in /tmp/reverify.txt for use in §12 final report
{
  echo "3.0.a bin/ensure-cdp-9222 (pre-pop): $(test -x bin/ensure-cdp-9222 && echo UNEXPECTED_EXISTS || echo EXPECTED_MISSING_WILL_APPEAR_FROM_STASH)"
  echo "3.0.b manifest project_key: $(grep -E '^\s*project_key:' docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml | head -1)"
  echo "3.0.c skills/auth-signin tracked: $(test -z "$(git ls-files skills/auth-signin/)" && echo NO_UNTRACKED_OK || echo YES_ALREADY_TRACKED)"
} > /tmp/reverify.txt
cat /tmp/reverify.txt
```

Branching for remaining work:
- (3.0.a) helper not on disk → §4 will verify it appears AFTER §3.3 stash pop, then smoke-test.
- (3.0.a edge case) helper already on disk pre-pop → §4 just smoke-tests; flag in audit doc.
- (3.0.b) manifest correct after Handoff 1 → no action; pass.
- (3.0.b edge case) manifest still wrong → Edit-tool fix per above; flag in commit C message.
- (3.0.c) auth-signin untracked → §5 commits it.
- (3.0.c edge case) auth-signin already tracked → §5 becomes a no-op for that path.

### 3.1 Inspect before popping

```bash
git stash show stash@{0} --stat                                                  # full list of touched files
git stash show stash@{0} -p > /tmp/stash-preview.patch                           # save patch for reference
git stash show stash@{0} --name-only | sort > /tmp/stash-files.txt              # sorted file list
```

### 3.2 Identify conflict-prone files (HIGH CONFLICT RISK after Handoff 1's revert)

The stash was created BEFORE Handoff 1 ran. Its hunks were authored against a baseline that included Zack's edits. Handoff 1's revert dropped those edits, so the stash's context lines may no longer match — `git stash pop` is **likely** to report conflicts on any file Handoff 1 reverted.

Files that are most conflict-prone:

```
docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml             (stash +91, Handoff 1 R1-restored)
docs/pipeline/l5-expanded-commercial-intelligence/01-validate-spec.md       (stash +39, Handoff 1 R1-restored)
docs/pipeline/l5-expanded-commercial-intelligence/EXECUTION-GUIDE.md        (stash +6,  Handoff 1 R1-restored)
docs/pipeline/l5-expanded-commercial-intelligence/03-create-tdd.md          (stash +13, possibly R1-restored)
docs/pipeline/l5-expanded-commercial-intelligence/05-create-strategy.md     (stash +22, possibly R1-restored)
docs/pipeline/l5-expanded-commercial-intelligence/07-create-beads.md        (stash +32, possibly R1-restored)
docs/pipeline/l5-expanded-commercial-intelligence/09-audit-beads.md         (stash +29, possibly R1-restored)
docs/pipeline/l5-expanded-commercial-intelligence/12-completion-audit.md    (stash +5,  possibly R1-restored)
docs/pipeline/stage-11-cdp-live-demos-continuation-handoff.md               (stash +5,  possibly R1-restored)
```

Files that should be LOW conflict (stash adds wholly-new content; not edited by Zack):

```
AGENTS.md                                      (Zack touched it but stash hunks add NEW sections)
crates/daemon/src/main.rs                      (Zack touched it but stash hunks add NEW functions)
crates/daemon/tests/skill_lint.rs              (probably new test methods)
skills/{...}/SKILL.md  (16 of them)            (stash hunks add helper invocation; may conflict if Zack edited the same SKILL.md sections)
```

Untracked files (from the stash's untracked component `bfa127f`) — ZERO conflict risk; they just appear on disk after pop:

```
bin/ensure-cdp-9222                                (blob a6b6b04e, executable — the auto-launch helper)
crates/daemon/tests/cdp_ensure_helper.rs           (blob a015a64d — new test target ut_ehlpr_001..)
crates/daemon/tests/auth_signin_lint.rs            (blob 38d1d748 — new test target ut_asln_001..)
docs/handoff-2026-05-17-cdp-9222-auto-launch.md    (blob 12e5f189 — the helper's design doc)
docs/handoff-2026-05-17-cdp-auto-signin.md         (blob af2fc720 — auth-signin family design)
```

These five items DO NOT need to be created in §4/§5; they restore from the stash. Verify their post-pop presence in §3.4 and treat their commit as part of the §10 staging plan.

**Conflict resolution policy (use this for EVERY conflict):**
- The stash's hunks represent the CDP-stack work — they are valid intent.
- If a conflict marker shows stash's hunk vs. Handoff 1's reverted baseline: **keep the stash version** (the CDP-stack work supersedes pre-Zack state in those file regions).
- If a conflict marker shows stash content that re-introduces a `/Users/zacfarrell/` path: that's the rare exception — keep the Alina-path version (Handoff 1's baseline) and patch the stash hunk to use alinaaitova.
- After EVERY hand-resolved file, re-grep for zacfarrell paths to confirm none crept back in.

### 3.3 Pop

```bash
git stash pop stash@{0}
```

**If pop reports conflicts:** (expected — see §3.2; resolve per the policy there)

```bash
git status                                                                       # lists CONFLICT files
# For each conflict file:
git diff <file>                                                                  # see the markers
# Resolution policy (also stated in §3.2):
#   - Stash hunks ADDING content (new CDP-stack functions, helper invocations, Session Recovery): KEEP THEM.
#   - Handoff 1 baseline (post-revert content, no zacfarrell paths): KEEP IT for any path strings.
#   - If a stash hunk also adds a new mention of /Users/zacfarrell/ (it shouldn't, but check):
#       reject the stash version and use the alinaaitova path.
# After hand-resolving (use plain grep since file isn't staged yet):
grep -E '/Users/zacfarrell' <file> && { echo "STOP: conflict resolution reintroduced zacfarrell path"; exit 1; }
git add <file>
```

**If pop reports NO conflicts:** double-check anyway:

```bash
git grep -nE '/Users/zacfarrell|/Users/elias' -- docs/pipeline/l5-expanded-commercial-intelligence/
# Expected: 0 matches (or only in any forensic record under that tree — there shouldn't be any)
```

### 3.4 Verify stash applied correctly

```bash
git status --short | head -40

# 3.4.a Spot-check the TRACKED-FILE modifications from stash:
git diff HEAD -- AGENTS.md | head -30                                            # should show new CDP Prerequisite + Session Recovery sections
git diff HEAD -- crates/daemon/src/main.rs | head -30                            # should show ensure_cdp_9222 function additions
git diff HEAD -- crates/daemon/tests/skill_lint.rs | head -20                    # should show ut_skl_302 test
git diff HEAD -- skills/marketplace-search/alibaba/SKILL.md | wc -l              # should show ~133 lines of diff
git diff HEAD -- skills/marketplace-search/ebay-uk/SKILL.md | wc -l              # ~114
git diff HEAD -- skills/procurement-ingestion/procontract-discover/SKILL.md | wc -l  # ~153

# 3.4.b Verify the 5 UNTRACKED-FILE additions from stash (these now appear on disk as new files):
[ -x bin/ensure-cdp-9222 ] && echo "✅ bin/ensure-cdp-9222 (exec)" || echo "❌ bin/ensure-cdp-9222 missing or not exec"
[ -f crates/daemon/tests/cdp_ensure_helper.rs ] && echo "✅ cdp_ensure_helper.rs" || echo "❌ cdp_ensure_helper.rs missing"
[ -f crates/daemon/tests/auth_signin_lint.rs ] && echo "✅ auth_signin_lint.rs" || echo "❌ auth_signin_lint.rs missing"
[ -f docs/handoff-2026-05-17-cdp-9222-auto-launch.md ] && echo "✅ auto-launch handoff" || echo "❌ auto-launch handoff missing"
[ -f docs/handoff-2026-05-17-cdp-auto-signin.md ] && echo "✅ auto-signin handoff" || echo "❌ auto-signin handoff missing"

# 3.4.c First-line sanity for the helper (shebang)
head -1 bin/ensure-cdp-9222
# Expected: #!/usr/bin/env bash
```

If any of the 3.4.a spot-checks shows 0 lines of diff, OR any 3.4.b check fails, OR 3.4.c shebang is wrong, the stash didn't apply cleanly. STOP and investigate.

### 3.5 Stash is now consumed

```bash
git stash list                                                                   # should be empty
```

If it still shows stash@{0}, you ran `git stash apply` not `git stash pop` — drop it:

```bash
git stash drop stash@{0}
```

---

## 4. Verify `bin/ensure-cdp-9222` (from stash pop) + smoke-test

The helper script already exists from the §3.3 stash pop. **You are NOT creating it.** This section verifies its post-pop properties and runs the §4.1 smoke tests.

```bash
# 4.0.a Existence + executability
[ -x bin/ensure-cdp-9222 ] || { echo "STOP: helper missing or non-executable after stash pop"; ls -la bin/ensure-cdp-9222; exit 1; }

# 4.0.b Shebang + spec sanity (matches the pre-verified blob a6b6b04e)
head -1 bin/ensure-cdp-9222 | grep -q "^#!/usr/bin/env bash" || { echo "STOP: wrong shebang"; head -3 bin/ensure-cdp-9222; exit 1; }
grep -q "json_escape()" bin/ensure-cdp-9222 && echo "✅ pure-bash json_escape present (no python3 dep)"
grep -q "AGENT_BROWSER_CDP_PORT" bin/ensure-cdp-9222 && echo "✅ honors AGENT_BROWSER_CDP_PORT env"
grep -q "CDP_USER_DATA_DIR" bin/ensure-cdp-9222 && echo "✅ honors CDP_USER_DATA_DIR env"
grep -q "helper.*ensure-cdp-9222" bin/ensure-cdp-9222 && echo "✅ diagnostic JSON tagged with helper name"

# 4.0.c Size sanity (the blob is ~60-80 lines based on Read; if truncated, something is wrong)
lines=$(wc -l < bin/ensure-cdp-9222)
echo "Helper line count: $lines"
[ "$lines" -gt 40 ] || { echo "STOP: helper looks truncated"; exit 1; }
```

If any 4.0 check fails: the stash pop in §3.3 didn't restore the helper correctly. Check `git stash show stash@{0}` to confirm `bin/ensure-cdp-9222` was in the untracked-files component (blob `a6b6b04e`); if so, manually `git checkout stash@{0}^3:bin/ensure-cdp-9222 bin/ensure-cdp-9222` (the `^3` is the untracked-files commit in stash structure) — but only if §3.3 didn't auto-restore.

The spec for the helper lives at `docs/handoff-2026-05-17-cdp-9222-auto-launch.md` (also restored from stash). Read it now if you haven't already — it details every exit code, env override, and behaviour the smoke tests below verify.

### 4.1 Smoke-test the helper standalone

```bash
# Scenario A: Chrome not running on 9222 (kill any first)
pkill -f "remote-debugging-port=9222" 2>/dev/null || true
sleep 2
pgrep -fl "remote-debugging-port=9222" && { echo "FAIL: Chrome still running"; exit 1; }
bin/ensure-cdp-9222 && echo "PASS: launched Chrome on 9222"                      # should launch and exit 0
curl -sf http://localhost:9222/json/version | head -3                            # should return JSON

# Scenario B: Chrome already running
bin/ensure-cdp-9222 && echo "PASS: detected existing Chrome on 9222"             # should exit 0 silently/instantly

# Scenario C: Port collision (non-Chrome process on 9222) — simulate
pkill -f "remote-debugging-port=9222" 2>/dev/null || true; sleep 2
nc -l 9222 &                                                                     # background netcat holding the port
NC_PID=$!
sleep 1
bin/ensure-cdp-9222 && { echo "FAIL: helper should have detected port-not-Chrome"; kill $NC_PID; exit 1; }
echo "PASS: helper correctly refused to launch over non-Chrome occupant"
kill $NC_PID 2>/dev/null || true
```

All three scenarios must pass before moving on.

---

## 5. Add `skills/auth-signin/` to git

```bash
ls skills/auth-signin/                                                           # expect 6 dirs: alibaba, ebay, freightos, google-account-chooser, procontract, searates
# Sanity: every sub-skill has its SKILL.md
for d in skills/auth-signin/*/; do
  [ -f "$d/SKILL.md" ] || { echo "STOP: $d missing SKILL.md"; exit 1; }
done

git add skills/auth-signin/
git diff --cached --stat -- skills/auth-signin/ | tail -5                       # confirm scope
```

Do NOT commit yet — bundle the commits per §10.

---

## 6. Daemon build + lint tests (full AGENTS.md "Stage Validation Test Gate")

The stash restored: `+92 lines to crates/daemon/src/main.rs`, `+16 lines to crates/daemon/tests/skill_lint.rs`, and two NEW test files: `crates/daemon/tests/cdp_ensure_helper.rs` (ut_ehlpr_001..) and `crates/daemon/tests/auth_signin_lint.rs` (ut_asln_001..). All must build green AND their tests pass.

Per AGENTS.md "Compiler Checks (CRITICAL)" + "Stage Validation Test Gate": narrow gates have produced false-greens before. Use the full gate.

```bash
# 6.1 Format check (cheapest gate — fail fast)
cargo fmt --check 2>&1 | tee /tmp/cargo-fmt.log
[ "${PIPESTATUS[0]}" = "0" ] && echo "✅ cargo fmt --check" || { echo "❌ cargo fmt FAILED — run cargo fmt and re-check"; head /tmp/cargo-fmt.log; }

# 6.2 Compiler check (all targets — includes the 2 new test files)
cargo check --all-targets 2>&1 | tee /tmp/cargo-check.log
grep -q "Finished" /tmp/cargo-check.log && echo "✅ cargo check --all-targets" || { echo "❌ cargo check FAILED"; tail -40 /tmp/cargo-check.log; }

# 6.3 Clippy (pedantic; warnings treated as errors per AGENTS.md)
cargo clippy --all-targets -- -D warnings 2>&1 | tee /tmp/cargo-clippy.log
grep -qE "warning|error" /tmp/cargo-clippy.log && { echo "❌ clippy found issues"; grep -E "warning:|error:" /tmp/cargo-clippy.log | head -20; } || echo "✅ cargo clippy clean"

# 6.4 Daemon binary unit tests
cargo test -p daemon --bin daemon 2>&1 | tee /tmp/cargo-test-bin.log | tail -10
grep -q "test result: ok" /tmp/cargo-test-bin.log && echo "✅ daemon bin tests" || { echo "❌ daemon bin tests FAILED"; tail -40 /tmp/cargo-test-bin.log; }

# 6.5 Integration test files — one per target (per AGENTS.md "Stage Validation Test Gate")
fail_count=0
for t in crates/daemon/tests/*.rs; do
  name=$(basename "$t" .rs)
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

# 6.6 Specifically verify the 3 CDP-stack tests (after the broader gate, focused verification)
cargo test --test skill_lint 2>&1 | tail -5            # ut_skl_302 — catches missing requires_cdp + missing helper invocation
cargo test --test cdp_ensure_helper 2>&1 | tail -5     # ut_ehlpr_001..099 — helper structural smoke tests
cargo test --test auth_signin_lint 2>&1 | tail -5      # ut_asln_001..099 — auth-signin family structural lints
```

If anything fails:
- Failure in 6.1 (fmt): `cargo fmt` to auto-fix; re-run.
- Failure in 6.2/6.3 (check/clippy): read the error; if it references a symbol Zack removed in revert, refer to Handoff 1's audit doc § "Build-required Zack contributions" (a kept-Zack file should have been documented; if not, the revert is incomplete).
- Failure in 6.4/6.5 (tests): identify the test, read the failure, diagnose. Common causes for our CDP-stack tests specifically:
  - `auth_signin_lint` fails because a production skill body doesn't mention `auth-signin/<site>` → wire it.
  - `cdp_ensure_helper` fails because helper has wrong shebang or missing env var → check §4 spec sanity.
  - `skill_lint` (ut_skl_302) fails because a skill declares `requires_cdp: true` but doesn't invoke `bin/ensure-cdp-9222` → add the invocation, OR remove `requires_cdp: true` if the skill doesn't actually need CDP.

Iterate fix-loop until 6.1–6.6 are all green. Pre-existing failures (from Handoff 1's `## Pre-existing test failures` audit section) are allowed as a baseline ONLY if explicitly noted there.

---

## 7. Live CDP validation tests

Now run the 8 test areas from `docs/handoff-2026-05-17-cdp-stack-validation-test.md`. That handoff has the full procedure for each area; this section gives you the order and the success criteria.

| Area | What it tests | Success criteria |
|---|---|---|
| A. Helper standalone | `bin/ensure-cdp-9222` cold-start + warm-start + port collision | All 3 scenarios pass (already done in §4.1 — record results from there) |
| B. Helper inside a CDP skill (thin) | One of: source-health/fts, marketplace-search/amazon-uk | Skill kicks off CDP, page loads, returns success result |
| C. Helper inside a CDP skill (heavy) | marketplace-search/alibaba (priority) | Skill launches CDP, navigates to Alibaba, returns listing data |
| D. Session Recovery — fresh session | marketplace-search/ebay-uk (priority) | Skill succeeds with logged-in eBay account |
| E. Session Recovery — expired session simulation | marketplace-search/alibaba | Skill detects expired session, invokes `skills/auth-signin/alibaba`, retries, succeeds |
| F. Google OAuth chooser | Any skill that triggers it (e.g., a fresh `skills/auth-signin/ebay`) | Chooser → Continue with Google → select Dieter Danicek → returns to site logged in |
| G. ProContract password autofill | skills/auth-signin/procontract | Autofills `sales@sorelon.com`; no Touch ID prompt (was disabled at chrome://settings/passwords on 2026-05-17); skill succeeds |
| H. Daemon end-to-end with cron skill | A scheduled skill (whichever the test handoff names) | Daemon launches helper, helper launches Chrome, skill runs, returns result, daemon writes result to SQLite |

For each area: run the test, write a row to `docs/testing/2026-05-18-l5-readiness-validation-report.md`. Use this format:

```markdown
| Area | Skill | Run timestamp | Result | Evidence path | Notes |
|---|---|---|---|---|---|
| A | bin/ensure-cdp-9222 (standalone) | 2026-05-18T... | ✅ PASS | (inline output captured) | cold/warm/collision all pass |
| B | source-health/fts | 2026-05-18T... | ❌ FAIL | /tmp/B-evidence.log | helper launched but skill timed out — diagnosing |
| ... |
```

**Fix loop:** For every FAIL row:
1. Read evidence
2. Diagnose root cause (helper bug? skill body bug? Session Recovery bug? selector drift?)
3. Fix the offending file
4. Re-run that ONE area
5. Update the row to ✅ PASS with new timestamp
6. Note the fix in the Notes column

Do not advance to §8 until every area is ✅ PASS or has been explicitly marked `DEFERRED — operator review` with a written reason (e.g., "ProContract has a 24h cooldown after 3 failed attempts — defer until tomorrow").

---

## 8. Verify no Zack/Elias paths reintroduced

The stash pop added content. Verify nothing crept in:

```bash
git diff HEAD -- $(git diff --cached --name-only) | grep -nE '/Users/zacfarrell|/Users/elias' && {
  echo "STOP: stash pop reintroduced a zac/elias path — find and fix it"
  exit 1
}
git grep -nE '/Users/zacfarrell|/Users/elias' | grep -vE '(handoff-2026-05-05|session-log-2026-05-05|planmaxer/archived)' | head -20
# Above should be EMPTY (only the documented exceptions from Handoff 1's audit doc remain).
```

If anything appears, fix it before committing.

---

## 9. Final L5-readiness checklist (run this in order)

These are the binary gates the operator needs before authorizing L5 step 01.

```bash
# 9.1 The two operator-built skills exist and have READY test reports
[ -f skills/landed-cost/freightos-freight-estimate/SKILL.md ] && echo "✅ freightos skill present" || echo "❌"
[ -f skills/landed-cost/taric-cross-check/SKILL.md ] && echo "✅ taric skill present" || echo "❌"
grep -q "Verdict: READY" docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/freightos-freight-estimate/TEST_REPORT.md 2>/dev/null && echo "✅ freightos READY" || echo "❌"
grep -q "Verdict: READY" docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/taric-cross-check/TEST_REPORT.md 2>/dev/null && echo "✅ taric READY" || echo "❌"

# 9.2 Helper exists and is executable
[ -x bin/ensure-cdp-9222 ] && echo "✅ bin/ensure-cdp-9222 ready" || echo "❌"

# 9.3 Daemon builds
cargo check -p daemon 2>&1 | tail -1 | grep -q "Finished" && echo "✅ daemon builds" || echo "❌"

# 9.4 Skill lint passes
cargo test -p daemon skill_lint 2>&1 | tail -3 | grep -q "test result: ok" && echo "✅ skill_lint green" || echo "❌"

# 9.5 AGENTS.md has the CDP Prerequisite + Session Recovery sections
grep -q "CDP Browser Prerequisite" AGENTS.md && echo "✅ AGENTS.md CDP section present" || echo "❌"
grep -q "Session Recovery" AGENTS.md && echo "✅ AGENTS.md Session Recovery section present" || echo "❌"

# 9.6 manifest.yaml agentmail key is alinaaitova
grep -q "project_key: /Users/alinaaitova/Desktop/Testproject1" docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml && echo "✅ manifest project_key correct" || echo "❌"

# 9.7 All 8 production skills have Session Recovery wiring
for s in skills/marketplace-search/{alibaba,ebay-uk} skills/landed-cost/{searates-freight-estimate,shipment-sizing} skills/source-health/alibaba skills/procurement-ingestion/procontract-{discover,enrich-tender,register-interest}; do
  grep -qE "auth-signin|SESSION_REFRESH|Session Recovery" "$s/SKILL.md" && echo "✅ $s wired" || echo "❌ $s NOT wired"
done

# 9.8 All 16 CDP-requiring skills invoke bin/ensure-cdp-9222
for s in skills/source-health/{alibaba,fts} skills/auth-refresh/{alibaba,1688} skills/marketplace-search/{alibaba,amazon-uk,ebay-uk,gem-imports,mx-wholesale} skills/procurement-ingestion/{contracts-finder,fts,procontract-discover,procontract-enrich-tender,procontract-register-interest} skills/landed-cost/{searates-freight-estimate,shipment-sizing}; do
  grep -q "ensure-cdp-9222" "$s/SKILL.md" && echo "✅ $s invokes helper" || echo "❌ $s does NOT invoke helper"
done

# 9.9 auth-signin tree present and staged
ls skills/auth-signin/ | wc -l | xargs -I{} test {} -eq 6 && echo "✅ 6 auth-signin sub-skills present" || echo "❌"
git diff --cached --name-only | grep -q "skills/auth-signin/" && echo "✅ auth-signin staged" || echo "❌"

# 9.10 Validation report written
[ -f docs/testing/2026-05-18-l5-readiness-validation-report.md ] && echo "✅ validation report exists" || echo "❌"

# 9.11 AgentMail health (the skill the operator referenced — must be green)
# Invoke MCP tool: mcp__mcp-agent-mail__health_check
# Expected: status === "healthy" (or whatever the tool's healthy schema says)
# Also: which am 2>/dev/null && am guard status /Users/alinaaitova/Desktop/Testproject1 | head -3
# Wrapper sanity:
for w in bin/br-claim bin/br-complete bin/br-sync; do
  [ -x "$w" ] && echo "✅ $w" || echo "❌ $w missing"
done
```

Every line must show ✅ before §10.

---

## 10. Commit (logical chunks) + Push to origin/main

Three logical commits to keep history reviewable:

### 10.1 Commit A — stash-pop content (CDP stack)

```bash
# Tracked-file modifications from stash:
git add AGENTS.md crates/daemon/src/main.rs crates/daemon/tests/skill_lint.rs \
        skills/auth-refresh/ skills/source-health/ skills/marketplace-search/ \
        skills/procurement-ingestion/ skills/landed-cost/searates-freight-estimate \
        skills/landed-cost/shipment-sizing \
        docs/pipeline/l5-expanded-commercial-intelligence/01-validate-spec.md \
        docs/pipeline/l5-expanded-commercial-intelligence/03-create-tdd.md \
        docs/pipeline/l5-expanded-commercial-intelligence/05-create-strategy.md \
        docs/pipeline/l5-expanded-commercial-intelligence/07-create-beads.md \
        docs/pipeline/l5-expanded-commercial-intelligence/09-audit-beads.md \
        docs/pipeline/l5-expanded-commercial-intelligence/12-completion-audit.md \
        docs/pipeline/l5-expanded-commercial-intelligence/EXECUTION-GUIDE.md \
        docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml \
        docs/pipeline/stage-11-cdp-live-demos-continuation-handoff.md

# NEW untracked test files from stash (these were in the untracked-files component bfa127f):
git add crates/daemon/tests/cdp_ensure_helper.rs crates/daemon/tests/auth_signin_lint.rs

git status --short | head                                                        # verify ONLY expected files staged

git commit -m "$(cat <<'EOF'
cdp: restore stash-pop CDP-stack integration (helper invocation, Session Recovery, daemon wiring, structural lints)

Restores ~1700 lines from stash@{0} ("freightos-build-temp-stash") that were
stashed during the Freightos build session 2026-05-17 and never popped back:

Tracked-file modifications:
- AGENTS.md (+97): CDP Browser Prerequisite + Session Recovery sections
- crates/daemon/src/main.rs (+92): ensure_cdp_9222 daemon integration
- crates/daemon/tests/skill_lint.rs (+16): ut_skl_302 lint catches missing requires_cdp
- 8 production skills wired with full Session Recovery: marketplace-search/{alibaba,ebay-uk},
  source-health/alibaba, landed-cost/{searates-freight-estimate,shipment-sizing},
  procurement-ingestion/procontract-{discover,enrich-tender,register-interest}
- 8 thinner CDP skills wired with helper invocation
- L5 pipeline docs updated with helper paths and operator decisions

New test files (from stash's untracked-files component):
- crates/daemon/tests/cdp_ensure_helper.rs (ut_ehlpr_001..099): structural smoke tests for bin/ensure-cdp-9222
- crates/daemon/tests/auth_signin_lint.rs  (ut_asln_001..099): structural lint for auth-signin/{site}/ family

All AGENTS.md "Stage Validation Test Gate" gates green: cargo fmt --check, cargo check --all-targets,
cargo clippy --all-targets -- -D warnings, cargo test -p daemon --bin daemon, and every integration test
file individually (see §6 of docs/handoff-2026-05-18-goal-cdp-l5-ready.md).

Co-Authored-By: <your-model-id> <noreply@anthropic.com>
EOF
)"
```

### 10.2 Commit B — the helper script + auth-signin family

`bin/ensure-cdp-9222` was restored from the stash's untracked-files component (not created by this handoff); `skills/auth-signin/` was on disk uncommitted from a prior session. Both land in this single commit.

```bash
git add bin/ensure-cdp-9222 skills/auth-signin/

git commit -m "$(cat <<'EOF'
cdp: commit bin/ensure-cdp-9222 helper + skills/auth-signin/ family

bin/ensure-cdp-9222 (pure bash, no python3 dep — restored from stash@{0} blob a6b6b04e):
- detects Chrome on $CDP_PORT (default 9222 via $AGENT_BROWSER_CDP_PORT alias), no-op if reachable
- launches with $CDP_USER_DATA_DIR (default ~/.chrome-automation) and $CDP_PROFILE_DIRECTORY (default "Default") if not
- polls /json/version up to $CDP_LAUNCH_TIMEOUT_SECONDS (default 15)
- emits structured JSON diagnostic on failure (helper, status, port, message, ts) to stderr
- exit codes: 0 ok, 1 launch failed, 2 unsupported OS, 3 Chrome binary missing
- macOS-only today (Chrome at /Applications/Google Chrome.app/...)

skills/auth-signin/ family (6 sub-skills, on disk uncommitted from prior session):
google-account-chooser, ebay, alibaba, searates, freightos, procontract. Each
declares requires_cdp: true and invokes bin/ensure-cdp-9222 to guarantee
Chrome is up before signing in.

Together with the stash-pop integration these complete the CDP stack:
production skills detect expired sessions, invoke the corresponding
auth-signin sub-skill, and resume on success.

Verified by ut_ehlpr_001..099 (cdp_ensure_helper.rs) and ut_asln_001..099
(auth_signin_lint.rs) — both green per §6 of the goal handoff.

Co-Authored-By: <your-model-id> <noreply@anthropic.com>
EOF
)"
```

### 10.3 Commit C — handoff docs + test reports + L5 operator-built docs

```bash
git add docs/handoff-2026-05-17-cdp-9222-auto-launch.md \
        docs/handoff-2026-05-17-cdp-auto-signin.md \
        docs/handoff-2026-05-17-cdp-stack-validation-test.md \
        docs/handoff-2026-05-18-zack-elias-cleanup.md \
        docs/handoff-2026-05-18-goal-cdp-l5-ready.md \
        docs/testing/ \
        docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/

git commit -m "$(cat <<'EOF'
docs: commit CDP-stack handoffs + test reports + L5 operator-built-skills

- docs/handoff-2026-05-17-cdp-{9222-auto-launch,auto-signin,stack-validation-test}.md:
  the three handoffs that specified the CDP stack work
- docs/handoff-2026-05-18-zack-elias-cleanup.md: Handoff 1 (revert + audit)
- docs/handoff-2026-05-18-goal-cdp-l5-ready.md: Handoff 2 (this handoff — verify, fix, test, push)
- docs/testing/: 4 test reports (auto-launch, auto-signin, stack-validation, L5-readiness)
- docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/:
  the two operator-built skills' build instructions + screenshots + test evidence

Co-Authored-By: <your-model-id> <noreply@anthropic.com>
EOF
)"
```

### 10.4 Verify locally then PUSH to origin/main

```bash
# 10.4.1 Sanity before push
git log --oneline -5                                                             # show the three commits
git status                                                                       # should be clean
git rev-parse --abbrev-ref HEAD                                                  # expect: main

# 10.4.2 Re-fetch — has anyone (operator? CI?) pushed since you started?
git fetch origin
git rev-list origin/main..HEAD --count                                           # should equal 3 (your commits)
git rev-list HEAD..origin/main --count                                           # should equal 0
# If origin moved (i.e. HEAD..origin/main returns > 0): STOP. Do NOT --force.
#   git pull --rebase origin main
#   Re-run §6 (build) and §9 (readiness) to confirm rebase didn't break anything.
#   Then continue with push.

# 10.4.3 Run the AgentMail health check ONE more time before push (must still be green)
# Invoke MCP tool: mcp__mcp-agent-mail__health_check
# Capture the JSON. If unhealthy, STOP and investigate — pushing while AgentMail is broken
# will block the next operator workflow that depends on agent-mail discipline.

# 10.4.4 Push (non-force)
git push origin main
git log origin/main..HEAD                                                        # expect: empty (push succeeded)
echo "Push succeeded at $(date -u +%FT%TZ)"
```

If push is rejected with "non-fast-forward": someone pushed to origin/main since you started. Do NOT `--force`. `git pull --rebase`, re-run §6/§9, then push again.

If push is rejected for any other reason (e.g., pre-commit hook on the server): read the error, fix the root cause, commit the fix as a 4th commit, push again. Never bypass hooks.

---

## 11. Final L5-readiness declaration

Open `docs/testing/2026-05-18-l5-readiness-validation-report.md` and append a `## L5-readiness declaration` section:

```markdown
## L5-readiness declaration (2026-05-18)

Pre-requisites verified (per §9 of handoff-2026-05-18-goal-cdp-l5-ready.md):

- [x] Both operator-built skills (freightos + taric) present with READY test reports
- [x] bin/ensure-cdp-9222 helper exists, executable, smoke-tested (cold/warm/collision)
- [x] Daemon builds cleanly
- [x] skill_lint test passes (ut_skl_302)
- [x] AGENTS.md has CDP Browser Prerequisite + Session Recovery sections
- [x] manifest.yaml agentmail project_key = /Users/alinaaitova/Desktop/Testproject1
- [x] All 8 production skills have Session Recovery wiring
- [x] All 16 CDP-requiring skills invoke bin/ensure-cdp-9222
- [x] skills/auth-signin/ family committed (6 sub-skills)
- [x] Live validation tests A–H all passed (or have documented operator-deferral)
- [x] AgentMail health check passes (MCP `mcp__mcp-agent-mail__health_check` + `am guard status`)
- [x] All three commits pushed to origin/main

**Status: L5 IS READY TO START.** Operator may run L5 step 01 (`docs/pipeline/l5-expanded-commercial-intelligence/01-validate-spec.md`) at their discretion.

Three commits pushed to origin/main:
1. <SHA-A> cdp: restore stash-pop CDP-stack integration
2. <SHA-B> cdp: add bin/ensure-cdp-9222 helper + skills/auth-signin/ family
3. <SHA-C> docs: commit CDP-stack handoffs + test reports + L5 operator-built docs
```

---

## 12. Report block to print at the end

Print this exact block to the operator (filling in the numbers):

```
HANDOFF 2 COMPLETE — L5 IS READY AND PUSHED.

Re-verification of "missing" items:
  - bin/ensure-cdp-9222: [MISSING — created from spec | EXISTED — smoke-tested only | RECOVERED from git history]
  - manifest project_key: [ALINA — Handoff 1 fixed it | ZACK — Handoff 2 sed-fixed it]
  - skills/auth-signin/ tracking: [UNTRACKED — Handoff 2 added | ALREADY TRACKED — no-op]
Stash pop: ✅ applied cleanly (1573 lines restored)  [or "applied with N resolved conflicts"]
Helper: ✅ bin/ensure-cdp-9222 executable, 3/3 smoke tests pass
auth-signin: ✅ 6 sub-skills committed
Daemon build: ✅ clean (cargo build -p daemon)
skill_lint: ✅ green (ut_skl_302 passing)
Live CDP tests: ✅ N/8 areas passing, M/8 deferred (see report)
AgentMail health: ✅ MCP healthy + am guard status healthy (or N/A — am not on PATH)
Path purity: ✅ no /Users/zacfarrell or /Users/elias outside documented exceptions
Commits: 3 (A: <SHA>, B: <SHA>, C: <SHA>)
Pushed to origin/main: ✅ at 2026-05-18T<time>

L5 is now READY. To start: read docs/pipeline/l5-expanded-commercial-intelligence/EXECUTION-GUIDE.md and run step 01.
```

---

## 13. Failure-mode triage guide (when /goal hits a wall)

| Symptom | Diagnostic | Fix |
|---|---|---|
| `git stash pop` reports merge conflicts on manifest.yaml (or any L5 pipeline file) | Handoff 1 reverted these files; stash hunks have stale context lines | Hand-merge per §3.2 policy: keep stash hunks for CDP-stack additions, keep alinaaitova for any path strings; grep for zacfarrell after every file resolved |
| `cargo build -p daemon` fails after stash pop | Read error; likely a function signature mismatch because Handoff 1 reverted a file the stash depends on | Inspect the +92 line additions to main.rs; if Handoff 1's audit doc § "Build-required Zack contributions" already noted this dependency, the file is restored — re-pop with conflict handling. Otherwise add to Handoff 1's exception list and reconcile. |
| `cargo test -p daemon skill_lint` fails with "missing requires_cdp" | A skill body invokes `bin/ensure-cdp-9222` but lacks YAML frontmatter | Add `requires_cdp: true` and `cdp_default_port: 9222` to the SKILL.md frontmatter |
| `bin/ensure-cdp-9222` fails Scenario A (cold launch) | `curl -sf http://localhost:9222/json/version` returns nothing | Check Chrome flags — `--remote-allow-origins=*` must be single-quoted; profile dir must be writable |
| `bin/ensure-cdp-9222` fails Scenario C (port collision) | Helper tries to launch on top of existing process | Helper logic should detect non-Chrome process via `/json/version` and bail with clear error; check the spec's §3.5 |
| Session Recovery test (Area E) — alibaba doesn't re-sign-in after expiry simulation | Skill body's session-expiry detection regex doesn't match the actual logout page | Open Alibaba logged-out in CDP, capture the actual logout-page selector, update the skill's detection block |
| Google chooser test (Area F) — wrong account selected | `dieter.x.s.danicek@gmail.com` selector drifted | Snapshot the chooser DOM via `agent-browser snapshot`, find the new attr-customurl value, update `skills/auth-signin/google-account-chooser/SKILL.md` |
| ProContract Touch ID prompt appears (Area G) | Operator's Chrome setting reverted | Open `chrome://settings/passwords` and disable "Use phone or tablet to fill in passwords" again; re-test |
| `cargo test` hangs in a CI-style fork | Lockfile collision on shared resource | Re-run with `--test-threads=1`; if still hangs, the test has a real concurrency bug — flag for operator |

---

## 13.5 What you must NOT do (AGENTS.md compliance)

- ❌ **`git reset --hard`, `git clean -fd`, `rm -rf`** — absolutely forbidden per AGENTS.md "Irreversible Git" rules.
- ❌ **`git push --force`** or any history rewrite.
- ❌ **`git commit --no-verify`** or skip pre-commit hooks. The AgentMail guard hook (`.git/hooks/hooks.d/pre-commit/50-agent-mail.py`) MUST pass — if it doesn't, fix the underlying cause (often manifest project_key, often a missing file reservation).
- ❌ **`sed -i`** or other script-based file mutations — AGENTS.md "No Script-Based Changes" forbids it. Use the Edit tool. (§3.0.b's edge-case manifest fix specifies Edit-tool.)
- ❌ **`cargo test -p daemon --bin daemon` alone as your build gate** — AGENTS.md says this is INSUFFICIENT (it silently excludes 39 integration test files). Use the full Stage Validation Test Gate as documented in §6.
- ❌ **`git add -A` or `git add .`** — stage the specific files §10.1/10.2/10.3 list.
- ❌ **Skip the AgentMail health check** (§9.11 + §10.4.3).
- ❌ **Run any L5 pipeline step** (step 01–end of `docs/pipeline/l5-expanded-commercial-intelligence/`).

### 13.6 Bead discipline (AGENTS.md RULE 2 clarification)

This work is **NOT bead-tracked**. AGENTS.md RULE 2 says *"For any bead-tracked work in this repo, you MUST use the project wrappers"* — this handoff is a one-off operator-authorized CDP-stack finalization, not a bead. So you do NOT run `bin/br-claim` / `bin/br-complete`. The pre-commit AgentMail guard hook still applies (it runs on every commit) — if it blocks you, fix the underlying cause.

If you discover during work that this *should* have been bead-tracked, STOP and ask operator before proceeding.

### 13.7 Estimated time

| Phase | Time |
|---|---|
| Read context (§1) — including 2 handoffs in stash via `git show` | 45–60 min |
| Pre-flight + AgentMail health (§2) | 15 min |
| Stash pop with conflict resolution (§3) | 30–90 min (depending on conflicts after Handoff 1's revert) |
| Helper verification + smoke tests (§4) | 20 min |
| Auth-signin git add (§5) | 5 min |
| Full Stage Validation Test Gate (§6) — fmt + check + clippy + bin tests + per-integration tests | 30–60 min |
| Live CDP validation tests (§7) — 8 areas | 2–4 hours |
| Path purity check (§8) | 5 min |
| Final readiness checklist (§9) | 15 min |
| Commit + push with AgentMail re-check (§10) | 20 min |
| Validation report + L5-readiness declaration (§11) | 15 min |
| **Total** | **~5–9 hours** |

Sub-phase 7 (live tests) is the biggest variable — selector drift on Alibaba/eBay can take an extra hour to repair.

---

## 14. Why this matters

L5 is the commercial-intelligence engine — the layer that turns scraped procurement signals into margin/landed-cost/RFQ decisions worth real money. Every L5 skill depends on:
- CDP browser being up (helper)
- The 8 production skills being able to recover from session expiry without operator intervention (Session Recovery)
- The daemon being able to detect-and-launch the helper itself (daemon integration)
- The lint catching skills that forget to declare `requires_cdp` (skill_lint)

If any one of these is broken when L5 starts, the L5 pipeline will hit step 09 (bead audit) and fail because at least one CDP-dependent skill will fail under live execution. That blocks step 10 (kickoff), which blocks all 50+ L5 beads.

**Better to spend 4 hours getting this 100% green now than to spend 12 hours unblocking a half-run L5 next week.** That's the whole point of this handoff.
