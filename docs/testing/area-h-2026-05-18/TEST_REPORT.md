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

**Verdict:** READY

| Sub-criterion | Result | Evidence |
|---|---|---|
| Pre-Option-C: Alibaba logged in | ✅ | C2.0a.logged-in.png; C2.0a.logged-in-state.json |
| Operator manual logout completed | ✅ | Operator completed Alibaba phone/SMS identity verification first; forced expiry then used targeted Alibaba cookie deletion in C2.0b.forced-logout-pagews.txt |
| Daemon triggered marketplace-search/alibaba | ✅ | C2.daemon.log |
| Session-expired detection fired | ✅ | C2.0b.logged-out-state.json; C2.chrome-history-sequence.txt |
| auth-signin/alibaba invoked | ✅ | C2.chrome-history-sequence.txt: `auth_signin_alibaba_google_start` |
| "Continue with Google" clicked | ✅ | C2.chrome-history-sequence.txt: Google OAuth route starts at 20:08:27Z |
| Dieter Danicek auto-selected in chooser | ✅ | C.resume.chooser-before-click.png; C2.0a.logged-in-state.json; Chrome profile observed as Dieter |
| Returned to alibaba.com logged in | ✅ | C2.chrome-history-sequence.txt: `alibaba_home` at 20:09:15Z |
| Production skill resumed inline (no daemon retry) | ✅ | C2.sequence.txt; single session `session-manual-de8bd8608edbe93fb6f9b450f6ceee08831d39e1f981e52d8a4ae512bb3f33ca` |
| Listing data returned with ≥1 product | ✅ | C2.result.json; C2.result-summary.txt (`quotes=2`) |
| Result landed in agent_session_logs | ✅ | C2.sqlite-row.txt |

**Iterations to PASS:** 2 (max 4)
**Fixes applied:**
- `skills/marketplace-search/alibaba/SKILL.md`: successful recovery now resumes the marketplace search inline instead of exiting for a daemon retry, and the playbook asks for auditable recovery markers.
- Operator completed Alibaba phone/SMS identity verification once in the persistent Chrome profile; the resumed daemon run then completed Session Recovery and listing extraction without another identity challenge.

**Total duration:** 2026-05-18T19:35Z → 2026-05-18T20:15Z (40 minutes including manual identity verification)

---

## Discoveries / quirks / selector drift noted

- `agent-browser --cdp 9222` probes `127.0.0.1:9222`; the prior helper only proved `localhost:9222`, which can resolve to an IPv6-only Chrome socket on macOS. The helper now binds and probes IPv4 loopback.
- Generic `--skill-path` PTY runs did not invoke the daemon-level CDP pre-flight. The daemon now does so before PTY spawn.
- Alibaba Google OAuth initially reached the expected Google account chooser and Dieter was present/selected, but Alibaba risk controls routed the OAuth return to `login.alibaba.com/iv/verify_mode.htm` and required one phone/SMS verification. After the operator completed it in the persistent profile, the rerun completed OAuth, returned to Alibaba logged in, resumed the original search, and extracted quotes.
- Because the operator reported the CDP browser repeatedly closing during manual logout, the forced-expiry setup used targeted CDP deletion of Alibaba-domain cookies only. Google cookies were not cleared.
- The raw helper launch can briefly report CDP reachable and then exit if a non-CDP Chrome instance is already registered. For the verification step, `open -na "Google Chrome" --args ...` kept the visible CDP window alive.

---

## Overall verdict

**READY FOR L5 STEP 01**

Operator may run `docs/pipeline/l5-expanded-commercial-intelligence/01-validate-spec.md` immediately. Do not start L5 from this Area H handoff agent.

---

## Commit

This report is committed as:
- `area-h: daemon-driven E2E demo (Options A+C) — verdict BLOCKED` (initial blocked evidence)
- `area-h: complete Alibaba Session Recovery after verification` (resumed READY evidence)

SHA: pending until final commit; final response records the pushed SHA.
Pushed to origin/main: pending until final push; final response records push status.
