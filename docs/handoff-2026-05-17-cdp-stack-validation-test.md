# Handoff: CDP Stack End-to-End Validation Test

> **Generated:** 2026-05-17
> **Trigger:** Operator (and another agent in between turns) has executed both prior CDP handoffs (auto-launch + auto-signin). On-disk inspection shows: `bin/ensure-cdp-9222` exists; 21/21 CDP-requiring skills updated; 6/6 auth-signin skill directories exist; BUT the 8 production-skill Session Recovery integrations are NOT done; and the daemon Rust integration is unverified. This handoff systematically tests what works, surfaces gaps, applies fixes inline where feasible, and produces a TEST_REPORT.md audit artifact.
> **Depends on:**
>   - `docs/handoff-2026-05-17-cdp-9222-auto-launch.md` (auto-launch — created `bin/ensure-cdp-9222`)
>   - `docs/handoff-2026-05-17-cdp-auto-signin.md` (auto-signin — created `skills/auth-signin/{site}/`)
> **Outcome when done:** TEST_REPORT.md exists with a per-area pass/fail verdict, all execution-blocking gaps fixed inline, deferred work documented as separate follow-up handoffs (if any), and an end-to-end empirical demonstration: kill Chrome → log out of Alibaba → trigger `marketplace-search/alibaba` → daemon auto-launches Chrome → skill detects logout → auto-signin runs → second invocation completes successfully. That single demonstration proves the entire stack works.

---

## Mission

Validate the CDP stack works end-to-end empirically. NOT just "files exist" — actually exercise the auto-launch + auto-signin paths against the live Chrome and live sites.

8 test areas, each with a Validate → Test → Fix loop:

| Area | Subject | Expected outcome | Fix-loop scope |
|---|---|---|---|
| **A** | `bin/ensure-cdp-9222` script behaviour | All 10 sub-tests pass | Edit helper if bugs found |
| **B** | 21 CDP-requiring skills correctly invoke the helper | All 21 verified | Fix any skill missed during Step 2 of auto-launch handoff |
| **C** | Daemon Rust integration (pre-flight gate, helper resolver, blocked-result writer) | All exist + compile + tests green | If absent: SEPARATE handoff; document gap, do not block |
| **D** | 6 auth-signin skills exist with REAL selectors (not TBD placeholders) | All 6 have Phase A discovery completed | Re-do Phase A for any skill found with placeholder selectors |
| **E** | 8 production skills wired with Session Recovery (**KNOWN GAP: 0/8 done**) | All 8 wired | Apply Step 4 of auto-signin handoff to each |
| **F** | AGENTS.md updated with both new sections | Both sections present | Add missing section if absent |
| **G** | Lint tests + smoke tests pass | All cargo tests green | Fix test failures |
| **H** | End-to-end empirical demonstration | Kill Chrome → log out of Alibaba → run skill → completes successfully | This is the headline test — must pass |

After all areas: write `docs/testing/2026-05-17-cdp-stack-validation-test-report.md` with verdict, fixes applied, and any deferred work.

---

## Operator action touchpoints (do these IN-ORDER as the test reaches each area)

The test pauses for you at these specific moments:

| When | What you do | Time |
|---|---|---|
| Area A.4 setup | Just be present — Chrome may briefly visible-launch | ~10s |
| Area D.6 setup (eBay) | Manually click avatar → Sign out in the persistent Chrome's eBay tab | ~30s |
| Area D.7 setup (Alibaba — PRIORITY) | Manually click avatar → Logout in Alibaba tab | ~30s |
| Area D.8 setup (SeaRates) | Manually click avatar → Sign out in SeaRates | ~30s |
| Area D.9 setup (Freightos) | Manually click avatar → Logout in Freightos (only if Freightos production skill exists; otherwise skip) | ~30s |
| Area D.10 setup (ProContract) | Manually click logout in ProContract | ~30s |
| Area D.5 (Google chooser test) | Open `https://accounts.google.com/AccountChooser?continue=https://myaccount.google.com/` manually; confirm `Dieter Danicek` row appears | ~30s |
| If Google 2FA / captcha prompts during ANY auth-signin test | Complete the 2FA / captcha manually in Chrome | up to ~2 min |
| Area H end-to-end demo | Watch the daemon log + Chrome window in real time | ~5 min |
| Final sign-off | Read TEST_REPORT.md verdict; approve or request fixes | ~10 min |

**Touch ID for ProContract: NOT REQUIRED** — you've already toggled "Always allow autofill without authentication" in `chrome://settings/passwords`. The test will verify autofill happens silently.

**Total operator-attention time:** ~10-15 min spread over the ~4-7 hour test execution. The rest is automated.

---

## Setup (do this once before starting)

### S.0 — Confirm CDP browser is up with the operator's persistent profile

```bash
# 1. CDP probe
curl -sfS -m 3 "http://localhost:9222/json/version" | jq '{browser: .Browser, userAgent: .["User-Agent"]}'
# Expected: Chrome/<version>; if fails, run the helper to launch it.

# 2. If CDP is not up, launch via the helper
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
# Expected: exit 0 within ~5s; Chrome window appears

# 3. Confirm operator is signed into the relevant sites (open each tab manually):
#    - https://www.ebay.co.uk/        ← should show user avatar top-right
#    - https://www.alibaba.com/        ← should show "Hi <name>" / My Alibaba menu
#    - https://www.searates.com/       ← should show user avatar
#    - https://ship.freightos.com/     ← should show "D" avatar
#    - https://procontract.due-north.com/   ← should show dashboard (not /Login)
# If any show signed-out → log in manually now (the test will exercise auto-signin
# from this baseline; tests need a known-signed-in starting state).
```

### S.1 — Create test-evidence directory

```bash
mkdir -p /Users/alinaaitova/Desktop/Testproject1/docs/testing/cdp-stack-validation-2026-05-17
EVID=/Users/alinaaitova/Desktop/Testproject1/docs/testing/cdp-stack-validation-2026-05-17
```

### S.2 — Install validators (if not already from prior handoffs)

```bash
# JSON schema validator
which ajv || bun add -g ajv-cli ajv-formats
which jq  # required throughout
```

### S.3 — Record environment versions in TEST_REPORT preamble

```bash
cat > $EVID/environment.txt <<EOF
Test date:        $(date -u +%FT%TZ)
Bash:             $(bash --version | head -1)
Zsh:              $(zsh --version 2>/dev/null || echo "n/a")
jq:               $(jq --version)
ajv:              $(ajv --version 2>/dev/null || echo "not installed")
agent-browser:    $(agent-browser --version 2>/dev/null || echo "unknown")
Chrome (browser): $(curl -sfS http://localhost:9222/json/version 2>/dev/null | jq -r .Browser)
macOS:            $(sw_vers -productVersion)
EOF
cat $EVID/environment.txt
```

---

## Test areas

### Area A — `bin/ensure-cdp-9222` script behaviour (~30 min)

#### A.1 — Helper exists, executable, correct shebang

```bash
[[ -x /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222 ]] \
  && head -1 /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222 \
  || echo "FAIL: not executable or missing"
# Expected: "#!/usr/bin/env bash"
echo "PASS at $(date -u +%FT%TZ)" > $EVID/A.1.status
```

#### A.2 — Fast path (CDP up → exits 0 in <2s)

```bash
# Make sure CDP is up first
curl -sfS -m 2 http://localhost:9222/json/version >/dev/null || /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222

# Time the helper
START=$SECONDS
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
EXIT=$?
ELAPSED=$((SECONDS - START))
[[ $EXIT -eq 0 && $ELAPSED -lt 2 ]] && echo "PASS ($ELAPSED s)" || echo "FAIL: exit=$EXIT elapsed=${ELAPSED}s"
```

Record to `$EVID/A.2.status` either "PASS at ..." or "FAIL: <reason>".

#### A.3 — Launch path (CDP down → helper launches Chrome → CDP comes up)

```bash
# Bring CDP down
pkill -f "remote-debugging-port=9222" 2>/dev/null
sleep 2
curl -sfS -m 1 http://localhost:9222/json/version >/dev/null && echo "WARN: Chrome respawned"

# Time the helper to bring it back
START=$SECONDS
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222 2> $EVID/A.3.stderr
EXIT=$?
ELAPSED=$((SECONDS - START))
curl -sfS -m 2 http://localhost:9222/json/version | jq -r .Browser > $EVID/A.3.cdp-after-launch
[[ $EXIT -eq 0 && $ELAPSED -lt 15 ]] && echo "PASS ($ELAPSED s)" || echo "FAIL: exit=$EXIT elapsed=${ELAPSED}s"
```

Also verify the diagnostic JSON on stderr is well-formed:

```bash
# A.3.stderr should contain a JSON line {"helper": "ensure-cdp-9222", "status": "launched", ...}
grep -q '"helper":"ensure-cdp-9222"' $EVID/A.3.stderr && jq . < $EVID/A.3.stderr >/dev/null \
  && echo "JSON OK" || echo "JSON malformed"
```

#### A.4 — Env override `CDP_PORT`

```bash
# Should launch a SECOND Chrome on port 9333 (NOT kill the 9222 one)
CDP_PORT=9333 /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
EXIT=$?
curl -sfS -m 2 http://localhost:9333/json/version | jq -r .Browser
# Then kill the test Chrome to clean up
pkill -f "remote-debugging-port=9333" 2>/dev/null
sleep 1
[[ $EXIT -eq 0 ]] && echo "PASS" > $EVID/A.4.status || echo "FAIL" > $EVID/A.4.status
```

#### A.5 — Env override `CDP_USER_DATA_DIR`

```bash
# Use a throwaway profile dir
TMP_PROFILE=$(mktemp -d -t cdp-test-profile-XXXXXX)
CDP_PORT=9334 CDP_USER_DATA_DIR=$TMP_PROFILE /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
EXIT=$?
# Verify the throwaway dir got populated (Chrome creates Default/ subdir on first launch)
[[ -d $TMP_PROFILE/Default ]] && echo "Chrome populated throwaway profile" \
  || echo "WARN: Chrome did not write to throwaway profile"
pkill -f "remote-debugging-port=9334" 2>/dev/null
rm -rf $TMP_PROFILE
[[ $EXIT -eq 0 ]] && echo "PASS" > $EVID/A.5.status || echo "FAIL" > $EVID/A.5.status
```

#### A.6 — Env override `CDP_LAUNCH_TIMEOUT_SECONDS`

```bash
# Very short timeout — should still succeed on the fast path (CDP already up)
CDP_LAUNCH_TIMEOUT_SECONDS=2 /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
[[ $? -eq 0 ]] && echo "PASS" > $EVID/A.6.status || echo "FAIL" > $EVID/A.6.status
```

#### A.7 — Unsupported OS path (exit 2)

```bash
# Bring CDP down so the helper attempts launch
pkill -f "remote-debugging-port=9222" 2>/dev/null; sleep 2

# Spoof OSTYPE (bash respects env override on startup)
OSTYPE=linux-gnu CDP_PORT=9335 /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222 2> $EVID/A.7.stderr
EXIT=$?
grep -q '"status":"unsupported_os"' $EVID/A.7.stderr && echo "diagnostic OK" || echo "diagnostic MISSING"
[[ $EXIT -eq 2 ]] && echo "PASS" > $EVID/A.7.status || echo "FAIL: exit=$EXIT" > $EVID/A.7.status

# Bring real Chrome back up for subsequent tests
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
```

#### A.8 — Missing Chrome binary (exit 3)

This is hard to test without uninstalling Chrome. Skip and mark "deferred — destructive to test" unless the operator wants it:

```bash
echo "DEFERRED: requires Chrome binary to be removed; not run." > $EVID/A.8.status
```

#### A.9 — Diagnostic JSON parses cleanly (validates helper's pure-bash json_escape)

```bash
# Trigger a diagnostic emission (unsupported OS path is the cheapest)
OSTYPE=linux-gnu CDP_PORT=9336 /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222 2> $EVID/A.9.stderr
# Each line on stderr should be valid JSON
while IFS= read -r line; do
  echo "$line" | jq . >/dev/null 2>&1 || { echo "INVALID JSON: $line"; FAIL=1; }
done < $EVID/A.9.stderr
[[ -z "${FAIL:-}" ]] && echo "PASS" > $EVID/A.9.status || echo "FAIL" > $EVID/A.9.status
```

#### A.10 — Pure-bash `json_escape` handles edge cases

The helper's `json_escape` function should handle quotes, backslashes, newlines. Test by spoofing a tricky `CDP_USER_DATA_DIR` value with quotes:

```bash
TRICKY_DIR='/tmp/test"with"quotes-$$/path'
CDP_PORT=9337 CDP_USER_DATA_DIR="$TRICKY_DIR" OSTYPE=linux-gnu \
  /Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222 2> $EVID/A.10.stderr
# The diagnostic should be valid JSON despite the quotes
jq . < $EVID/A.10.stderr >/dev/null 2>&1 && echo "PASS" > $EVID/A.10.status || echo "FAIL" > $EVID/A.10.status
```

#### Area A loop-exit gate

- [ ] A.1-A.10 all PASS (or A.8 explicitly DEFERRED with operator approval)
- [ ] If any FAIL: read the stderr/diagnostic, fix the helper script, re-run the failing test, document fix in TEST_REPORT.md "Fixes applied"

---

### Area B — CDP skill helper integration (21 SKILL.md updates) (~20 min)

#### B.1 — Every CDP-requiring skill invokes `ensure-cdp-9222`

```bash
MISSING=$(find /Users/alinaaitova/Desktop/Testproject1/skills -name SKILL.md -type f \
  | xargs grep -l "requires_cdp:.*true" \
  | xargs grep -L "ensure-cdp-9222")
if [[ -z "$MISSING" ]]; then
  echo "PASS (all 21 covered)" > $EVID/B.1.status
else
  echo "FAIL: missing helper invocation in:" > $EVID/B.1.status
  echo "$MISSING" >> $EVID/B.1.status
fi
```

If MISSING is non-empty: apply Step 2 of the auto-launch handoff to each listed file. Then re-run.

#### B.2 — Each invocation is in the "Mandatory CDP Discipline" section (not random)

```bash
for f in $(find /Users/alinaaitova/Desktop/Testproject1/skills -name SKILL.md -type f \
  | xargs grep -l "requires_cdp:.*true"); do
  # Check that ensure-cdp-9222 mention appears between "Mandatory CDP Discipline" and the next ## heading
  if awk '/^## Mandatory CDP Discipline/,/^## [^M]/' "$f" | grep -q "ensure-cdp-9222"; then
    : # OK
  else
    echo "WARN: $f mentions ensure-cdp-9222 but not in Mandatory CDP Discipline section"
  fi
done > $EVID/B.2.report
[[ ! -s $EVID/B.2.report ]] && echo "PASS" > $EVID/B.2.status || echo "WARN: see B.2.report" > $EVID/B.2.status
```

(WARN is acceptable — agent might have intentionally put it elsewhere; spot-check the 1-2 outliers.)

#### B.3 — Absolute path used (not relative `bin/...`)

```bash
RELATIVE=$(find /Users/alinaaitova/Desktop/Testproject1/skills -name SKILL.md -type f \
  | xargs grep -l "requires_cdp:.*true" \
  | xargs grep -l "^bin/ensure-cdp-9222\|[^/]bin/ensure-cdp-9222")
if [[ -z "$RELATIVE" ]]; then
  echo "PASS (all absolute paths)" > $EVID/B.3.status
else
  echo "FAIL: relative path in:" > $EVID/B.3.status
  echo "$RELATIVE" >> $EVID/B.3.status
fi
```

If any FAIL: fix by replacing relative path with `/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222`.

#### Area B loop-exit gate

- [ ] B.1 PASS
- [ ] B.2 PASS (WARNs spot-checked acceptable)
- [ ] B.3 PASS

---

### Area C — Daemon Rust integration (~30 min check; up to 4hr to FIX if not done)

Per the audit, the daemon integration is the most likely place where Step 4 of the auto-launch handoff was skipped. Check empirically:

#### C.1 — `ensure_cdp_9222()` function exists in `crates/daemon/src/main.rs`

```bash
grep -n "fn ensure_cdp_9222" /Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs > $EVID/C.1.grep
[[ -s $EVID/C.1.grep ]] && echo "PASS" > $EVID/C.1.status || echo "FAIL: function missing" > $EVID/C.1.status
```

#### C.2 — `default_ensure_cdp_helper_path()` exists

```bash
grep -n "fn default_ensure_cdp_helper_path" /Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs > $EVID/C.2.grep
[[ -s $EVID/C.2.grep ]] && echo "PASS" > $EVID/C.2.status || echo "FAIL: function missing" > $EVID/C.2.status
```

#### C.3 — `write_cdp_blocked_result()` exists

```bash
grep -n "fn write_cdp_blocked_result" /Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs > $EVID/C.3.grep
[[ -s $EVID/C.3.grep ]] && echo "PASS" > $EVID/C.3.status || echo "FAIL: function missing" > $EVID/C.3.status
```

#### C.4 — Pre-flight is wired at agent-spawn sites

```bash
# The integration should call ensure_cdp_9222(...) right before each Command::new
# that spawns a skill with requires_cdp. Grep for the call pattern.
grep -n "ensure_cdp_9222(" /Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs > $EVID/C.4.grep
WIRED=$(wc -l < $EVID/C.4.grep)
# We expect at least 4 calls (the 3 spawn sites + the Alibaba pre-check at line 8064)
[[ $WIRED -ge 4 ]] && echo "PASS ($WIRED call sites)" > $EVID/C.4.status \
  || echo "FAIL: only $WIRED call sites" > $EVID/C.4.status
```

#### C.5 — Existing Alibaba pre-check at `main.rs:8064` was updated

```bash
# Look at the lines around the Alibaba shipment-sizing check
sed -n '8060,8075p' /Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs > $EVID/C.5.context
# Should now reference ensure_cdp_9222, not just cdp_port_reachable
grep -q "ensure_cdp_9222" $EVID/C.5.context && echo "PASS" > $EVID/C.5.status \
  || echo "FAIL: still uses old cdp_port_reachable only" > $EVID/C.5.status
```

#### C.6 — `cargo check` compiles

```bash
cd /Users/alinaaitova/Desktop/Testproject1
cargo check --all-targets 2> $EVID/C.6.stderr
[[ $? -eq 0 ]] && echo "PASS" > $EVID/C.6.status || echo "FAIL: see C.6.stderr" > $EVID/C.6.status
```

#### C.7 — `cargo clippy` passes with `-D warnings`

```bash
cd /Users/alinaaitova/Desktop/Testproject1
cargo clippy --all-targets -- -D warnings 2> $EVID/C.7.stderr
[[ $? -eq 0 ]] && echo "PASS" > $EVID/C.7.status || echo "FAIL: see C.7.stderr" > $EVID/C.7.status
```

#### C.8 — Full integration test sweep

```bash
cd /Users/alinaaitova/Desktop/Testproject1
cargo test -p daemon --bin daemon 2> $EVID/C.8.stderr
RES=$?
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || RES=$?
done
[[ $RES -eq 0 ]] && echo "PASS" > $EVID/C.8.status || echo "FAIL: see C.8.stderr" > $EVID/C.8.status
```

#### Area C loop-exit gate (DECISION POINT)

If **C.1-C.5 all FAIL**: daemon integration was NOT done. **Two options:**

**Option α (recommended if operator has time):** apply Step 4 of the auto-launch handoff now. ~2-4 hours of focused Rust work. Then re-run C.1-C.8.

**Option β (recommended if operator is time-constrained):** mark Area C as **DEFERRED to a separate "Daemon CDP pre-flight integration" handoff**. Stack still works WITHOUT daemon integration — the skill-level defense-in-depth (Area B) covers the same ground, just with one extra subprocess call per skill invocation. Daemon integration is an optimization, not a correctness requirement.

Document the decision in TEST_REPORT.md under "Deferred work".

If C.1-C.5 PASS but C.6-C.8 FAIL: actual bugs in the Rust integration — fix-in-place required (this IS the loop part of the validate-test-fix).

---

### Area D — auth-signin skills (Phase A discovery completeness + end-to-end) (~3-4 hr)

#### D.1 — All 6 directories exist with required files

```bash
EXPECTED=("auth-signin/google-account-chooser" "auth-signin/ebay" "auth-signin/alibaba" "auth-signin/searates" "auth-signin/freightos" "auth-signin/procontract")
for s in "${EXPECTED[@]}"; do
  d="/Users/alinaaitova/Desktop/Testproject1/skills/$s"
  if [[ -f "$d/SKILL.md" && -f "$d/result.schema.json" ]]; then
    echo "  ✓ $s"
  else
    echo "  ✗ $s (missing SKILL.md or result.schema.json)"
    FAIL=1
  fi
done > $EVID/D.1.report
[[ -z "${FAIL:-}" ]] && echo "PASS" > $EVID/D.1.status || echo "FAIL: see D.1.report" > $EVID/D.1.status
```

#### D.2 — Each declares `requires_cdp: true` + `cdp_default_port: 9222`

```bash
for s in "${EXPECTED[@]}"; do
  f="/Users/alinaaitova/Desktop/Testproject1/skills/$s/SKILL.md"
  grep -q "requires_cdp:.*true" "$f" && grep -q "cdp_default_port:.*9222" "$f" \
    || echo "  ✗ $s (missing requires_cdp:true or cdp_default_port:9222)"
done > $EVID/D.2.report
[[ ! -s $EVID/D.2.report ]] && echo "PASS" > $EVID/D.2.status || echo "FAIL: see D.2.report" > $EVID/D.2.status
```

#### D.3 — Each invokes `bin/ensure-cdp-9222`

```bash
for s in "${EXPECTED[@]}"; do
  grep -q "ensure-cdp-9222" "/Users/alinaaitova/Desktop/Testproject1/skills/$s/SKILL.md" \
    || echo "  ✗ $s (no ensure-cdp-9222 invocation)"
done > $EVID/D.3.report
[[ ! -s $EVID/D.3.report ]] && echo "PASS" > $EVID/D.3.status || echo "FAIL: see D.3.report" > $EVID/D.3.status
```

#### D.4 — Each has REAL selectors (no `TBD` / `<...>` placeholders left)

```bash
for s in "${EXPECTED[@]}"; do
  f="/Users/alinaaitova/Desktop/Testproject1/skills/$s/SKILL.md"
  # Count obvious placeholder markers
  TBD=$(grep -c "TBD during build\|<CSS selector for\|<selector \(matching\|TODO" "$f")
  if [[ $TBD -gt 0 ]]; then
    echo "  ⚠ $s has $TBD placeholder markers — Phase A discovery incomplete"
  fi
done > $EVID/D.4.report
[[ ! -s $EVID/D.4.report ]] && echo "PASS" > $EVID/D.4.status || echo "PARTIAL: see D.4.report" > $EVID/D.4.status
```

If PARTIAL: for each skill with placeholders, do Phase A discovery now (DevTools selector inspection per the auto-signin handoff §Empirical Phase A discovery). Update the SKILL.md inline. Re-run D.4.

#### D.5 — `auth-signin/google-account-chooser/` end-to-end against live Google chooser

**Operator action:** open this URL manually in the persistent Chrome (port 9222):
`https://accounts.google.com/AccountChooser?continue=https://myaccount.google.com/`

You should see the Google account chooser with `Dieter Danicek (dieter.x.s.danicek@gmail.com)` as a clickable row.

```bash
# After confirming you see the chooser, run:
# Mimic what the skill's eval does: find the dieter row by selector
DIETER_PRESENT=$(agent-browser --cdp 9222 eval '
  (() => {
    const rows = document.querySelectorAll("[data-identifier],[data-email]");
    for (const r of rows) {
      const id = r.getAttribute("data-identifier") || r.getAttribute("data-email") || "";
      if (id.toLowerCase().includes("dieter.x.s.danicek")) return true;
    }
    return /dieter\.x\.s\.danicek/i.test(document.body.innerText);
  })()
' 2>/dev/null | tr -d '[:space:]')

if [[ "$DIETER_PRESENT" == "true" ]]; then
  echo "PASS — Dieter row detectable" > $EVID/D.5.status
else
  echo "FAIL — Dieter row not found via selectors. Check the auth-signin/google-account-chooser/SKILL.md selectors match the current Google chooser DOM." > $EVID/D.5.status
fi
```

If FAIL: open DevTools, inspect the Dieter row, find the actual selector. Update `auth-signin/google-account-chooser/SKILL.md` with the corrected selector. Re-run.

#### D.6 — `auth-signin/ebay/` end-to-end

**Operator action:** in the persistent Chrome, navigate to `https://www.ebay.co.uk/`. Click avatar → Sign out. Confirm you see the "Sign in or register" link top-right.

Then run the auth-signin/ebay skill by extracting its click sequence and executing manually:

```bash
CDP_PORT=9222
# 1. Open eBay
agent-browser --cdp $CDP_PORT open "https://www.ebay.co.uk/"
agent-browser --cdp $CDP_PORT wait 3000

# 2. Find Sign In button selector from auth-signin/ebay/SKILL.md and click it
# (Paste the selectors from the actual SKILL.md here — they were discovered in Phase A)
agent-browser --cdp $CDP_PORT click '<actual selector from SKILL.md>'
agent-browser --cdp $CDP_PORT wait 3000

# 3. Find Continue with Google button on eBay's signin page
agent-browser --cdp $CDP_PORT click '<actual selector from SKILL.md>'
agent-browser --cdp $CDP_PORT wait 3000

# 4. Click Dieter row in Google chooser (from auth-signin/google-account-chooser)
agent-browser --cdp $CDP_PORT click '<actual selector from SKILL.md>'
agent-browser --cdp $CDP_PORT wait 5000

# 5. Verify signed in
SIGNED_IN=$(agent-browser --cdp $CDP_PORT eval '
  document.querySelector("a[href*=\"signin.ebay.co.uk\"]") === null &&
  /Hi |Hello |\bMy eBay\b/i.test(document.body.innerText)
' 2>/dev/null | tr -d '[:space:]')

[[ "$SIGNED_IN" == "true" ]] && echo "PASS" > $EVID/D.6.status || echo "FAIL: not signed in after sequence" > $EVID/D.6.status
```

If FAIL: examine which step broke (open DevTools, observe the page state after each command). Update the SKILL.md selectors. Re-run.

#### D.7 — `auth-signin/alibaba/` end-to-end (PRIORITY per operator)

**Same pattern as D.6** but for Alibaba.

**Operator action:** logout in `https://www.alibaba.com/` (click avatar → Log Out).

```bash
agent-browser --cdp 9222 open "https://login.alibaba.com/"
agent-browser --cdp 9222 wait 2000
agent-browser --cdp 9222 click '<Continue with Google selector from SKILL.md>'
agent-browser --cdp 9222 wait 3000
agent-browser --cdp 9222 click '<Dieter row selector>'
agent-browser --cdp 9222 wait 5000

SIGNED_IN=$(agent-browser --cdp 9222 eval '
  document.querySelector("[class*=\"account-menu\"], [class*=\"user-info\"]") !== null
' 2>/dev/null | tr -d '[:space:]')
[[ "$SIGNED_IN" == "true" ]] && echo "PASS" > $EVID/D.7.status || echo "FAIL" > $EVID/D.7.status
```

Likely Alibaba-specific failure: a "Slide to verify" CAPTCHA appears after Continue with Google. If so → mark as PARTIAL with note "Alibaba shows slide-CAPTCHA — escalation case per auth-signin/alibaba SKILL.md failure list. Manually solve and verify the rest of the sequence completes."

#### D.8 — `auth-signin/searates/` end-to-end

**Same pattern.** Operator logs out in searates.com. Run sequence. Verify `localStorage.getItem("s-token")` is non-null after.

```bash
# Verify s-token absent before
S_TOKEN_BEFORE=$(agent-browser --cdp 9222 eval 'localStorage.getItem("s-token") || ""' 2>/dev/null)
[[ -z "$S_TOKEN_BEFORE" ]] || echo "WARN: s-token still present despite logout — re-logout"

# Run the click sequence (extract from SKILL.md)
# ...

# Verify s-token present after
S_TOKEN_AFTER=$(agent-browser --cdp 9222 eval 'localStorage.getItem("s-token") || ""' 2>/dev/null)
[[ -n "$S_TOKEN_AFTER" && "$S_TOKEN_AFTER" != "" ]] && echo "PASS" > $EVID/D.8.status || echo "FAIL" > $EVID/D.8.status
```

#### D.9 — `auth-signin/freightos/` end-to-end

**Status check first:** does the actual Freightos production skill exist?

```bash
[[ -f /Users/alinaaitova/Desktop/Testproject1/skills/landed-cost/freightos-freight-estimate/SKILL.md ]] \
  && SKILL_EXISTS=yes || SKILL_EXISTS=no
```

- If SKILL_EXISTS=yes: run the same end-to-end pattern (logout Freightos → run sequence → verify "D" avatar back).
- If SKILL_EXISTS=no: mark `D.9` as `DEFERRED — Freightos production skill not yet built; auth-signin/freightos selectors will be verified during Freightos build handoff Phase A.9.`

#### D.10 — `auth-signin/procontract/` end-to-end (Touch ID disabled per operator's recent change)

**Operator action:** logout in `https://procontract.due-north.com/` (navigate to logout link).

```bash
# Run click sequence from SKILL.md. Expected: autofill fires silently, login succeeds, NO Touch ID prompt (because operator disabled the Chrome setting).
# Verify by checking URL changes from /Login to dashboard within 12s.
```

If autofill DOESN'T fire silently: there's another autofill protection somewhere. Investigate `chrome://settings/passwords` again.
If Touch ID prompts despite the setting: it's ProContract-side 2FA (case 2 from the earlier analysis) — mark as `manual_required` and document.

#### Area D loop-exit gate

- [ ] D.1-D.4 all PASS (placeholders fixed where found)
- [ ] D.5 PASS (Google chooser works)
- [ ] D.6 PASS (eBay) ← PRIORITY
- [ ] D.7 PASS or PARTIAL with documented Alibaba CAPTCHA case ← PRIORITY
- [ ] D.8 PASS (SeaRates)
- [ ] D.9 PASS or DEFERRED (Freightos skill not yet built)
- [ ] D.10 PASS (ProContract with Touch ID disabled)

---

### Area E — Production skill Session Recovery (KNOWN GAP — 0/8 done) (~2-3 hr to fix)

**Pre-test status (verified earlier):** none of these 8 skills have Session Recovery sections yet. Step 4 of the auto-signin handoff was skipped during the previous execution.

```
marketplace-search/ebay-uk
marketplace-search/alibaba                      ← PRIORITY
landed-cost/searates-freight-estimate
landed-cost/shipment-sizing
source-health/alibaba
procurement-ingestion/procontract-discover
procurement-ingestion/procontract-enrich-tender
procurement-ingestion/procontract-register-interest
```

#### E.1-E.8 — Verify presence of Session Recovery in each

```bash
SKILLS=(
  "marketplace-search/ebay-uk"
  "marketplace-search/alibaba"
  "landed-cost/searates-freight-estimate"
  "landed-cost/shipment-sizing"
  "source-health/alibaba"
  "procurement-ingestion/procontract-discover"
  "procurement-ingestion/procontract-enrich-tender"
  "procurement-ingestion/procontract-register-interest"
)
for s in "${SKILLS[@]}"; do
  f="/Users/alinaaitova/Desktop/Testproject1/skills/$s/SKILL.md"
  if grep -q "Session Recovery\|SESSION_REFRESH_FAILED" "$f"; then
    echo "  ✓ $s wired"
  else
    echo "  ✗ $s — Session Recovery section missing"
  fi
done > $EVID/E.report

UNWIRED=$(grep -c "✗" $EVID/E.report || echo 0)
WIRED=$((8 - UNWIRED))
echo "Wired: $WIRED / 8" > $EVID/E.status
```

#### Fix-loop for unwired skills (PRIORITY ORDER — most important first)

Apply Step 4 of the auto-signin handoff (`docs/handoff-2026-05-17-cdp-auto-signin.md`) to each unwired skill. Priority order:

1. **`marketplace-search/alibaba`** — PRIORITY SITE per operator. Highest value.
2. **`marketplace-search/ebay-uk`** — PRIORITY SITE per operator.
3. **`source-health/alibaba`** — shares Alibaba session; trivial once #1 is done.
4. **`landed-cost/shipment-sizing`** — shares Alibaba session.
5. **`landed-cost/searates-freight-estimate`** — SeaRates.
6. **`procurement-ingestion/procontract-discover`** — ProContract.
7. **`procurement-ingestion/procontract-enrich-tender`** — ProContract.
8. **`procurement-ingestion/procontract-register-interest`** — ProContract.

For each, copy the Session Recovery template from the auto-signin handoff Step 4, customize for the skill's specific `<STATUS_FIELD>` and detection signals, and inline the click sequence from the matching `skills/auth-signin/<site>/SKILL.md` §Workflow.

After each fix, re-run E.report and verify that skill shows ✓.

#### E.9 — Re-run lint after all fixes

```bash
cd /Users/alinaaitova/Desktop/Testproject1
cargo test --test auth_signin_lint -- ut_asln_004_production_skills_invoke_auth_signin 2> $EVID/E.9.stderr
[[ $? -eq 0 ]] && echo "PASS" > $EVID/E.9.status || echo "FAIL: see E.9.stderr" > $EVID/E.9.status
```

#### Area E loop-exit gate

- [ ] All 8 skills show ✓ in E.report
- [ ] E.9 lint passes
- [ ] OR: minimum 2 PRIORITY skills (alibaba + ebay-uk) wired, rest documented as deferred work in TEST_REPORT.md

---

### Area F — AGENTS.md updates (~10 min)

#### F.1 — "CDP Browser Prerequisite (port 9222)" section present

```bash
grep -n "CDP Browser Prerequisite\|## Chrome DevTools Protocol" /Users/alinaaitova/Desktop/Testproject1/AGENTS.md > $EVID/F.1.grep
[[ -s $EVID/F.1.grep ]] && echo "PASS" > $EVID/F.1.status || echo "FAIL: section missing" > $EVID/F.1.status
```

#### F.2 — "Session Recovery for Authenticated CDP Sites" section present

```bash
grep -n "Session Recovery for Authenticated CDP Sites\|## Session Recovery" /Users/alinaaitova/Desktop/Testproject1/AGENTS.md > $EVID/F.2.grep
[[ -s $EVID/F.2.grep ]] && echo "PASS" > $EVID/F.2.status || echo "FAIL: section missing" > $EVID/F.2.status
```

If either FAIL: copy the section from the corresponding handoff Step 5/7 and append to AGENTS.md.

#### Area F loop-exit gate

- [ ] F.1 + F.2 both PASS

---

### Area G — Lint + smoke tests (~10 min)

#### G.1 — `ut_skl_302_cdp_skills_invoke_ensure_helper`

```bash
cd /Users/alinaaitova/Desktop/Testproject1
cargo test --test skill_lint -- ut_skl_302_cdp_skills_invoke_ensure_helper 2> $EVID/G.1.stderr
[[ $? -eq 0 ]] && echo "PASS" > $EVID/G.1.status || echo "FAIL: see G.1.stderr" > $EVID/G.1.status
```

#### G.2 — `cdp_ensure_helper` smoke tests

```bash
cd /Users/alinaaitova/Desktop/Testproject1
cargo test --test cdp_ensure_helper 2> $EVID/G.2.stderr
[[ $? -eq 0 ]] && echo "PASS" > $EVID/G.2.status || echo "FAIL: see G.2.stderr" > $EVID/G.2.status
```

#### G.3 — `auth_signin_lint`

```bash
cd /Users/alinaaitova/Desktop/Testproject1
cargo test --test auth_signin_lint 2> $EVID/G.3.stderr
RES=$?
if [[ $RES -eq 0 ]]; then
  echo "PASS" > $EVID/G.3.status
elif grep -q "no test target" $EVID/G.3.stderr; then
  echo "FAIL: auth_signin_lint.rs test file not created — Step 8 of auto-signin handoff skipped" > $EVID/G.3.status
else
  echo "FAIL: see G.3.stderr" > $EVID/G.3.status
fi
```

If G.3 reports test file missing: create it per the auto-signin handoff Step 8 specification.

#### Area G loop-exit gate

- [ ] G.1-G.3 all PASS (or G.3 PARTIAL with fix applied)

---

### Area H — End-to-end empirical demonstration (~30-45 min) — THE HEADLINE TEST

This is the test that proves the architecture works. If H.1 + H.2 pass, the operator can claim "no more manual Chrome launches OR manual site re-signins for normal operations."

#### H.1 — Cold-start auto-launch

**Setup:** Bring Chrome completely down.

```bash
pkill -f "remote-debugging-port=9222" 2>/dev/null
sleep 2
curl -sfS -m 1 http://localhost:9222/json/version >/dev/null && echo "WARN: respawned"
```

**Test:** trigger ANY CDP-requiring skill via the daemon. Easiest path: invoke `source-health/alibaba` since it's a fast health-check skill.

```bash
# Trigger via your normal daemon-triggering mechanism (typically a manual job submission
# or letting the daemon's scheduler pick it up on its next cron tick).
# Watch the daemon logs for evidence of:
#   - bin/ensure-cdp-9222 fired (if Area C was done)
#   - Chrome launches
#   - source-health/alibaba completes (returns a health verdict, not CDP_CONNECT_FAILED)
```

```bash
echo "Manual verification — observe daemon logs + Chrome window:" > $EVID/H.1.status
echo "  ☐ Did Chrome launch within 15s of triggering the skill?" >> $EVID/H.1.status
echo "  ☐ Did source-health/alibaba complete WITHOUT a CDP_CONNECT_FAILED blocker?" >> $EVID/H.1.status
echo "Operator: edit this file to PASS/FAIL after observation." >> $EVID/H.1.status
```

#### H.2 — Auto-signin recovery (Alibaba — PRIORITY)

**Setup:** Chrome is up (from H.1). Manually log out of Alibaba (click avatar → Logout).

**Test:** trigger `source-health/alibaba` again.

**Expected behavior:**
- Skill body's Session Recovery section detects logged-out (Area E.2 must be wired for this to work — if not wired, this test is DEPENDENT on E.2 completion).
- Skill runs inline auth-signin/alibaba click sequence.
- Skill exits cleanly without writing result.json (signed-in, daemon will retry).
- Daemon retries source-health/alibaba.
- Second invocation completes successfully (health check green).

```bash
echo "Manual verification — observe daemon logs + Alibaba browser tab:" > $EVID/H.2.status
echo "  ☐ Did the skill log 'session expired, running auth-signin'?" >> $EVID/H.2.status
echo "  ☐ Did Chrome navigate through login.alibaba.com → accounts.google.com → back?" >> $EVID/H.2.status
echo "  ☐ Did the Alibaba browser tab show signed-in state (avatar/menu) after?" >> $EVID/H.2.status
echo "  ☐ Did the daemon retry the skill?" >> $EVID/H.2.status
echo "  ☐ Did the second invocation complete with a green health verdict?" >> $EVID/H.2.status
echo "Operator: edit this file to PASS/FAIL after observation." >> $EVID/H.2.status
```

#### H.3 — Auto-signin recovery (eBay)

Same pattern as H.2 but for eBay. Manually log out → trigger `marketplace-search/ebay-uk` (or any other ebay-uk-using flow) → observe recovery.

Skip H.3 if Area E.1 (ebay-uk Session Recovery) wasn't wired.

#### Area H loop-exit gate

- [ ] H.1 PASS (cold-start auto-launch works)
- [ ] H.2 PASS or DEPENDENT (Alibaba auto-signin recovery works; if E.2 not wired, mark as "blocked on Area E completion")
- [ ] H.3 PASS or DEPENDENT (eBay same)

**Single most important assertion:** H.2 PASS proves the entire stack works end-to-end for the priority site (Alibaba). If only ONE thing passes in this whole test, this is the one that matters.

---

## TEST_REPORT.md (write this to `docs/testing/2026-05-17-cdp-stack-validation-test-report.md` after all areas complete)

Template:

```markdown
# CDP Stack End-to-End Validation Test — Report

**Test date:** YYYY-MM-DD
**Tested by:** <operator name or agent name>
**Test duration:** <total hours>
**Environment:** see `test-evidence/cdp-stack-validation-2026-05-17/environment.txt`

## Verdict

**STATUS: READY FOR PRODUCTION** (or PARTIAL / BLOCKED)

## Per-area results

| Area | Subject | Verdict | Notes |
|---|---|---|---|
| A | `bin/ensure-cdp-9222` script behaviour | PASS (10/10) | A.8 deferred (destructive) |
| B | 21 CDP skills invoke helper | PASS | All absolute paths |
| C | Daemon Rust integration | <PASS / DEFERRED to separate handoff> | <if deferred, link the new follow-up handoff> |
| D | auth-signin skills (6) | <PASS / PARTIAL> | D.7 Alibaba CAPTCHA noted |
| E | Production skill Session Recovery | <8/8 wired / X/8 wired> | <list which were skipped> |
| F | AGENTS.md sections | PASS | |
| G | Lint + smoke tests | PASS | |
| H | End-to-end empirical | <PASS / DEPENDENT on E> | <observation notes> |

## Fixes applied inline during this test

(List each fix with timestamp + 1-line description + which test it unblocked.)

- 2026-05-17 14:32 — Updated `auth-signin/google-account-chooser/SKILL.md` selector from `[data-email=...]` to `[data-identifier=...]` (Google's chooser DOM changed). Unblocked D.5.
- 2026-05-17 15:10 — Applied Session Recovery to `marketplace-search/alibaba/SKILL.md` (Step 4 of auto-signin handoff). Unblocked H.2.
- ...

## Deferred work (separate handoffs needed)

- <e.g., Daemon Rust integration (Area C) — schedule a follow-up handoff "CDP daemon pre-flight Rust integration" — estimated ~4 hours of focused Rust work.>
- <e.g., Phase A discovery for auth-signin/freightos — handled as part of Freightos build per HANDOFF-freightos-freight-estimate.md Phase A.9; not a separate handoff.>

## Operator sign-off

I have reviewed the test report and verified:

- [ ] H.2 (Alibaba auto-recovery) demonstrated end-to-end with my own eyes.
- [ ] All "Fixes applied" entries match my recollection of what was changed.
- [ ] All "Deferred work" items are tracked elsewhere (separate handoffs / notes).
- [ ] I am OK to proceed to building Freightos + Taric skills (per `HANDOFF-freightos-freight-estimate.md` / `HANDOFF-taric-cross-check.md`).

Signed: <operator name>
Date: YYYY-MM-DD
```

---

## Loop-exit gate (BLOCKING — verify before declaring complete)

- [ ] All 8 area-level loop-exit gates green (or documented as DEFERRED with operator approval)
- [ ] TEST_REPORT.md written + committed
- [ ] Test-evidence directory pushed to git for audit trail
- [ ] H.2 (Alibaba end-to-end auto-recovery) PASS — this is the headline test
- [ ] Operator signed off on TEST_REPORT.md

---

## What to do if Area C (daemon integration) is found missing

If Area C tests show the Rust integration was never done, you have two clean paths:

**Path 1 (recommended if daemon work is desired):** create a follow-up handoff `docs/handoff-2026-05-1?-cdp-daemon-preflight-integration.md` that does ONLY the Rust work. Use Step 4 of the auto-launch handoff as the spec. Estimated effort: 2-4 hours focused Rust. Schedule after this test handoff completes.

**Path 2 (recommended if time-constrained):** mark Area C as deferred. The stack still works without daemon integration because the skill-level defense-in-depth (Area B's ensure-cdp-9222 invocation inside every SKILL.md) covers the same ground. Daemon integration is an optimization (avoids one subprocess call per skill invocation) not a correctness requirement.

Either way: document the choice in TEST_REPORT.md "Deferred work" section.

---

## What to do if auth-signin skills have placeholder selectors (Area D.4 PARTIAL)

This means whoever executed the auto-signin handoff built the SKILL.md skeletons but skipped Phase A empirical discovery. Two paths:

**Path 1:** Do Phase A discovery NOW for each affected skill. Open the relevant site in Chrome, use DevTools, find the real selectors, update the SKILL.md inline. ~15-30 min per skill. After: re-run D.4 then D.6-D.10 end-to-end tests.

**Path 2:** Defer to a separate "auth-signin Phase A discovery completion" handoff. Stack works for skills that DO have real selectors; documents the ones that don't as blocked.

Recommend Path 1 unless time-constrained.

---

## Rollback (if the test reveals catastrophic bugs)

The CDP stack is additive. To roll back:

1. **Revert SKILL.md edits:** `git restore $(find skills -name SKILL.md -type f)` — all 21 + 6 = 27 SKILL.md files return to pre-helper state.
2. **Revert daemon Rust changes** (if any were made): `git revert <commit>` for the daemon integration commit.
3. **Leave `bin/ensure-cdp-9222` in place:** harmless when not invoked.
4. **Operator manually launches Chrome** as before (the command in `stage-11-cdp-live-demos-continuation-handoff.md`).

Total rollback: < 15 min.

---

## Notes for the next agent / operator

- This test handoff is INTENTIONALLY long because it's exhaustive across two prior handoffs of work. Don't skip areas.
- Test areas D-H require operator presence at specific moments — see the touchpoint table above.
- The priority sites are Alibaba and eBay (per operator). If only those two pass D.6/D.7 + the H.2 demonstration, the test can be declared PARTIALLY READY and Freightos/Taric builds can begin.
- If you delegate to a coding agent, brief them: "Read the test handoff in full, then execute area-by-area. Pause at each operator touchpoint and request confirmation before proceeding."
- The 8 unwired production skills (Area E) are the biggest gap from the prior handoff execution. Fixing them ALONE makes the auto-signin investment pay off.

---

## After this test handoff completes

Recommended order of next handoffs:

1. **HANDOFF-freightos-freight-estimate.md** — build the Freightos skill. The auth-signin/freightos skill should be VERIFIED (D.9 = PASS or completed during this handoff) before Freightos build starts.
2. **HANDOFF-taric-cross-check.md** — build the Taric cross-check skill (in parallel with Freightos; they're independent).
3. **Optional:** the deferred work items (daemon Rust integration if it was missing, additional auth-signin Phase A discovery, etc.).

After both Freightos and Taric build successfully + their TEST_REPORTs are READY, the L5 step 09 bead audit can authorize step 10 (kickoff for L5 implementation).
