# Operator Authorization — FORMAL_SUBMISSION Smoke Test — 2026-06-01

> **THIS IS NOT AUTHORIZATION FOR A REAL LEGALLY-BINDING SUBMISSION.**
> It is the operator's standing authorization to close bead `testproject1-3yqf.7.3` on a **synthetic / stubbed smoke-test basis**. No real bid is submitted on Sorellon's Procontract account.

## Operator standing authorization (verbatim, 2026-06-01)
> "For the beads that are sensitive, legally-binding issues [7.2, 7.3] — it's just a smoke test, just proof of concept. Just make sure it's possible, make sure it's right. That's how you should test it, and then close the bead. We will be testing that more seriously in the future. Make sure the handoff tells it to close the beads so we can continue and finish building this layer. If [sorellon.md] is not enough, the handoff should include a section where it creates the materials itself but still does a live, dynamic, real test with fake information."

## What this authorizes
- Prove the FORMAL_SUBMISSION mechanism (`draft_saved → bid_in_progress → bid_submitted`) and the fail-closed validator path using **synthetic, stubbed** data with a clearly-synthetic confirmation reference.
- Close 7.3 once the mechanism is proven possible + correct.
- Mark DoD #8 as **PARTIAL** (smoke-proven; real submission deferred) and still declare M6.

## What this explicitly does NOT authorize (constitutional bright lines — spec §1.10, handoff §2)
- ❌ A real `FormalSubmission` or `FundsCommitted` on the live `sales@sorellon.com` Procontract account.
- ❌ Notifying a real buyer (`NotificationOrLock`) on a real opportunity.
- ❌ Raising the live `sideEffectCap` to FORMAL_SUBMISSION against a real opportunity.

A *real* formal submission remains a future, separately-authorized exercise once Sorellon has real certifications + insurance and a chosen genuine opportunity. **No such authorization is given or implied here.**

## Authorized scope for the smoke test
- Opportunity: **synthetic only** (`SMOKE-TEST-DN000000` / synthetic process). No real opportunity is selected, registered, or submitted to.
- `sideEffectCap` = `FORMAL_SUBMISSION` **only inside the synthetic test payload / state-machine context**, never against the live portal.
- `reviewRequired` = `false` in the synthetic submission path (to exercise the no-review branch).
- Recorded BEFORE any system action (this file precedes the diary's execution log). Hard gate satisfied on a smoke-test basis.
