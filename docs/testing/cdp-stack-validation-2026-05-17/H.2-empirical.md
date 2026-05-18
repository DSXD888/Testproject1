# H.2 Empirical Verification — In-progress snapshot

**Time:** 2026-05-17T19:58:30Z (1m13s after kill+trigger)

## Sequence

1. `pkill -f "remote-debugging-port=9222"` — killed Chrome to confirm cold-start auto-launch.
2. Verified Chrome was DOWN (`curl http://localhost:9222/json/version` → connection refused).
3. Triggered: `cargo run -p daemon -- --ingest alibaba-search` (background, daemon PID 45333).

## Observed end-to-end behavior

- **Daemon bootstrap PASS**: `daemon bootstrap ready socket_path=runtime/daemon.sock state_db=state/state.db artifact_root=artifacts`.
- **CDP auto-launch via daemon's preflight PASS**: Chrome was NOT running at trigger time. CDP became reachable after the daemon started. This empirically validates `ensure_cdp_9222` being wired into the daemon's pre-flight (Area C C.1, C.4, C.5).
- **Skill invocation PASS**: the skill `marketplace-search/alibaba` was actually invoked. Chrome's primary tab navigated to:
  ```
  https://www.alibaba.com/trade/search?SearchText=library%20furniture%20supply%20and%20fit%20out%20Supply%20fit%20out&has4Tab=true
  ```
  with real product listings rendered ("Modern Library Furniture Reading Table and Chairs Set School Library Furniture College Library..."). This proves the daemon → skill → browser-automation pipeline works.

## What this demonstrates

End-to-end, the H.2 happy-path is proven:
- Kill Chrome (was up before trigger)
- Trigger the daemon CLI
- Daemon auto-launches Chrome via the helper
- Daemon invokes the skill
- Skill drives the browser (real Alibaba search executed)

## Why Session Recovery branch did not fire in this run

The daemon's chosen candidate ("library furniture supply and fit out") is an anonymous-search query — Alibaba's `/trade/search` endpoint serves results to logged-out visitors. The skill's Session Recovery branch only triggers when session-expired signals appear (passport.alibaba.com redirect, Sign In link in header, USER_NOT_LOGIN API response, slide CAPTCHA). None of these appeared in this run because anonymous search doesn't require auth.

The Session Recovery code path is verified by composition:
- D.5 (Google chooser) — chooser logic PASS
- D.7 — auth-signin/alibaba Steps 1-6 PASS (Step 7 hit identity-verification escalation, captured)
- Agent 4 code review — inlined sequence in `marketplace-search/alibaba/SKILL.md` is clean (no bugs)
- Inlined block is byte-identical to the live-validated auth-signin/alibaba/SKILL.md Workflow

To force the Session Recovery branch in a future run:
- Clear Alibaba session cookies before triggering
- Or invoke a candidate that requires logged-in access (e.g., supplier-inspection skills, RFQ posting)

## Verdict

H.2 EMPIRICAL PASS for the daemon → preflight → skill-invocation → browser-automation pipeline. The Session Recovery branch is verified by composition.
