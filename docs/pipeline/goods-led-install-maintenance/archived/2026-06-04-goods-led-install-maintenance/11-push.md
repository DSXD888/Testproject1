# Goods-Led Install/Maintenance — Push (Continuation Nudge)

> Used as needed DURING execution to re-orient a continuing or fresh agent. This is a meta-pointer: the real push prompt is the GENERATED artifact at
> `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-push.md` (produced by step 10).
> Send the agent that generated artifact. This file documents what it must contain and how to use it.

---

## How To Use

When an agent stalls, finishes a batch of beads, or a fresh session picks up mid-implementation, paste the generated push artifact:
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-push.md
```

It is the compact continuation nudge (the kickoff is the full onboarding; the push is the "you already know this — keep going" reminder).

---

## The Push Artifact MUST Contain (verify after step 10 generates it)

1. **Re-Orient** — re-read AGENTS.md; one-line feature description; spec + strategy paths; "your work orders are beads: `br ready --label goods-led-install --json`."
2. **Core Contracts (compact)** — integer pence / bps; screening-grade ±20%, never a firm quote; fail-soft (`blocked` labour → review, no crash); labour summed at BOTH rollup sites (main.rs:8114 + 8548), new columns nullable/default-0, labour added only when an `InstallationRequirement` exists; near-floor value-share → `review`; labour skill is `http_public` (no CDP).
3. **Bead Completion Policy (compact, firm)** — `bin/br-claim`/`bin/br-complete` only (Rule 2). Every code bead fully complete: all code + all tests + fmt/clippy/test + committed AND pushed + `br-complete` clean. No partial work. **Research-spike carve-out (the 3 Tier-B `research-spike` beads):** complete = the committed research/design output (per the bead body), no test gate — mirror the kickoff's carve-out so push and kickoff agree. If blocked: diagnose, fix yourself; escalate via AgentMail only if the blocker is in another agent's code.
4. **Work Loop (compact)** — inbox → `br ready --label goods-led-install` → read bead → `bin/br-claim` → TDD implement → verify (full AGENTS.md stage gate) → commit+push → `bin/br-complete` → loop. "Do not stop to ask what to work on next."
5. **Verification Commands** — the full AGENTS.md stage gate (fmt + check + clippy + bin + every integration target + bun test).
6. **Dependency Spine (compact)** — one line per phase.
7. **File Surfaces (compact table)** — epic → key files.
8. **AgentMail Quick Reference (compact)** — NO broadcast; discover peers via `list_contacts`; wrappers send Start/Complete.

---

⛔ **MANDATORY STOP after the execution wave completes** — Human triggers the completion audit (step 12) before declaring done.
