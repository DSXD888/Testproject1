# Area H — Daemon-Driven E2E Demo TEST_REPORT

**Date:** 2026-05-18
**Operator:** Alina Aitova
**Handoff agent:** Codex GPT-5
**Agent identity:** BrownBison
**Pre-flight:** ✅ all 8 checks green (see pre-flight.txt)

---

## Option A — source-health/fts daemon-driven E2E

**Verdict:** READY

| Sub-criterion | Result | Evidence |
|---|---|---|
| Daemon started without panics | ✅ | A.daemon.log |
| ensure_cdp_9222 invoked before skill spawn | ✅ | A.checkpoints.txt |
| Chrome opened visibly | ✅ | A.cdp-live-check.txt; A.fts-canary.png |
| Curl /json/version returned JSON | ✅ | A.cdp-live-check.txt |
| skill_lint shows skill running | ✅ | A.daemon.log |
| Agent-browser navigated to find-a-tender | ✅ | A.cdp-live-check.txt; A.result.json |
| Result landed in agent_session_logs | ✅ | A.sqlite-row.txt |
| Result JSON is well-formed | ✅ | A.result.json |
| No orphaned processes | ✅ | A.post-cleanup.txt |

**Iterations to PASS:** 2
**Fixes applied during iteration (if any):**
- `crates/daemon/src/main.rs`: generic `--skill-path` PTY execution now runs daemon CDP pre-flight before skill spawn and logs success/no-op/launch diagnostics.
- `bin/ensure-cdp-9222`: helper now probes `127.0.0.1` and launches Chrome with `--remote-debugging-address=127.0.0.1`, matching `agent-browser --cdp 9222` instead of accepting an IPv6-only localhost socket.

**Total duration:** 2026-05-18T19:24:16Z → 2026-05-18T19:30:16Z (6 minutes)

---

## Option C — marketplace-search/alibaba Session Recovery

**Verdict:** BLOCKED

| Sub-criterion | Result | Evidence |
|---|---|---|
| Pre-Option-C: Alibaba logged in | ✅ | C.0a.logged-in.png; C.0a.logged-in-state.txt |
| Operator manual logout completed | ⚠️ | Operator attempted logout, but Chrome repeatedly exited; per operator instruction to move on, forced expiry used targeted Alibaba cookie deletion in C.0b.forced-logout.txt |
| Daemon triggered marketplace-search/alibaba | ✅ | C.daemon.log |
| Session-expired detection fired | ✅ | C.result.json blocker and evidence |
| auth-signin/alibaba invoked | ✅ | C.result.json evidence: inline auth-signin/alibaba recovery sequence |
| "Continue with Google" clicked | ✅ | C.result.json evidence |
| Dieter Danicek auto-selected in chooser | ✅ | C.5.chooser-selection.png; C.5.chooser-state.txt |
| Returned to alibaba.com logged in | ❌ | C.live-check-3.txt: Alibaba returned `login.alibaba.com/iv/verify_mode.htm` |
| Production skill resumed inline (no daemon retry) | ❌ | Recovery blocked before signed-in state; no listing step executed |
| Listing data returned with ≥1 product | ❌ | C.result.json has `quotes: []` and `SESSION_REFRESH_FAILED` |
| Result landed in agent_session_logs | ✅ | C.sqlite-row.txt |

**Iterations to PASS:** 1 (max 4)
**Fixes applied:**
- `skills/marketplace-search/alibaba/SKILL.md`: successful recovery now resumes the marketplace search inline instead of exiting for a daemon retry, and the playbook asks for auditable recovery markers.

**Total duration:** 2026-05-18T19:35Z → 2026-05-18T19:48Z (13 minutes)

---

## Discoveries / quirks / selector drift noted

- `agent-browser --cdp 9222` probes `127.0.0.1:9222`; the prior helper only proved `localhost:9222`, which can resolve to an IPv6-only Chrome socket on macOS. The helper now binds and probes IPv4 loopback.
- Generic `--skill-path` PTY runs did not invoke the daemon-level CDP pre-flight. The daemon now does so before PTY spawn.
- Alibaba Google OAuth reached the expected Google account chooser and Dieter was present/selected, but Alibaba risk controls routed the OAuth return to `login.alibaba.com/iv/verify_mode.htm` and required phone/SMS verification.
- Because the operator reported the CDP browser repeatedly closing during manual logout, the forced-expiry setup used targeted CDP deletion of Alibaba-domain cookies only. Google cookies were not cleared.

---

## Overall verdict

**BLOCKED — see Option C**

Blocking criteria:
- Alibaba Session Recovery cannot complete without manual identity verification on `login.alibaba.com/iv/verify_mode.htm`.
- No listing data can be returned until the operator completes Alibaba phone/SMS verification in the persistent Chrome profile.

Operator action required:
- Complete the Alibaba identity verification challenge visible in the persistent Chrome profile, then rerun Option C from the logged-out/expired-session setup.

---

## Commit

This report is committed as: `area-h: daemon-driven E2E demo (Options A+C) — verdict BLOCKED`
SHA: pending until commit; final response records the pushed SHA.
Pushed to origin/main: pending until push; final response records push status.
