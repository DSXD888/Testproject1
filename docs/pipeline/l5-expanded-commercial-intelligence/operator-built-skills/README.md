# L5 Operator-Built Skill Prerequisites

> Two CDP-browser skills the operator personally builds BEFORE the L5 pipeline's step 09 bead audit can pass. These are NOT pipeline beads — they are prerequisites the audit verifies exist on disk AND have passing empirical test evidence.
>
> Recorded operator decision 2026-05-08: replace paid Freightos / Taric Support APIs with free CDP-browser skills, matching the project's "skill-first" architecture (`MEMORY.md` data strategy).

## The two handoffs

| Handoff | Target skill | Replaces |
|---|---|---|
| `HANDOFF-freightos-freight-estimate.md` (~715 lines + Test/Validate/Fix loop additions) | `skills/landed-cost/freightos-freight-estimate/` | Freightos paid API |
| `HANDOFF-taric-cross-check.md` (~673 lines + Test/Validate/Fix loop additions) | `skills/landed-cost/taric-cross-check/` | Taric Support paid vendor |

## Directory layout this handoff set produces

```
docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/
├── README.md                                       (this file)
├── HANDOFF-freightos-freight-estimate.md           (full build spec + Test/Validate/Fix loop)
├── HANDOFF-taric-cross-check.md                    (full build spec + Test/Validate/Fix loop)
├── screenshots/
│   ├── freightos/
│   │   ├── freightos-step-01.png    (empty homepage)
│   │   ├── freightos-step-02.png    (Origin popover)
│   │   ├── freightos-step-03.png    (Destination popover)
│   │   ├── freightos-step-04.png    (Load — Loose Cargo)
│   │   ├── freightos-step-05.png    (Load — Containers)
│   │   ├── freightos-step-06.png    (Goods popover)
│   │   ├── freightos-step-07.png    (filled-out Tianjin→Portsmouth)
│   │   ├── freightos-step-08.png    (Recommended Services)
│   │   └── freightos-step-09.png    (Results page with 4 tabs)
│   └── taric/
│       └── taric-consultation-new-ui-2026-05-17.png   (current UI)
└── test-evidence/                                   (POPULATED DURING BUILD — initially empty)
    ├── freightos-freight-estimate/
    │   ├── canary.inputs.jsonl                       (4 canary test cases — written in C.1)
    │   ├── golden.assertions.yaml                    (expected outcomes per canary — C.1)
    │   ├── failure-paths.jsonl                       (4 failure scenarios — C.2)
    │   ├── canary-01-*-input.json                    (one input per canary, captured during run)
    │   ├── canary-01-*-result.json                   (one result per canary, captured during run)
    │   ├── canary-01-*-status.txt                    ("PASS at <timestamp>" — one per canary)
    │   ├── (similarly canary-02, 03, 04 + fp-01, 02, 03, 04)
    │   └── TEST_REPORT.md                            (final audit artifact — verdict READY)
    └── taric-cross-check/
        ├── canary.inputs.jsonl                       (5 canary test cases — written in C.1)
        ├── golden.assertions.yaml                    (expected outcomes per canary — C.1)
        ├── failure-paths.jsonl                       (4 failure scenarios — C.2)
        ├── canary-01-*-input.json + result.json + status.txt
        ├── (similarly canary-02..05 + fp-01..04)
        └── TEST_REPORT.md                            (incl. HMRC cross-agreement section)
```

## Build sequence recommendation

Either order works (they're independent). Recommended: **Freightos first** because it's a closer mirror of the existing SeaRates skill (same dual-source pattern, same site type — commercial marketplace with login). Taric is simpler in some ways (no login, public EU data) but the Test/Validate/Fix loop includes an HMRC cross-agreement step that's unique to it.

**Estimated build time per skill (REVISED 2026-05-08):**

| Skill | Discovery | Build files | Test/Validate/Fix loop | Integration | TOTAL |
|---|---|---|---|---|---|
| Freightos | 60-90 min | 60-90 min | 2-4 hours | 15-30 min | **4-7 hours** |
| Taric | 30-60 min | 45-60 min | 1.5-3 hours | 15 min | **3-5 hours** |
| BOTH | | | | | **7-12 hours operator time** |

These can run in parallel with L5 pipeline steps 01-08.

## Verification before declaring complete (REVISED 2026-05-08)

Each handoff has a Verification checklist split into THREE sections:

1. **Build verification** — files exist, schemas are correct, content is complete (~10 items each)
2. **Empirical Test/Validate/Fix loop** — canary fixtures pass empirically against live CDP, failure paths handled, TEST_REPORT.md exists (~12-15 items each)
3. **Integration verification** — all committed and pushed (~3 items each)

**All three sections must be 100% green before declaring the skill complete.** Partial completion does not count, because:

- **3/4 canaries passing means 1 input shape the skill can't handle.** The L5 daemon will invoke this skill against many real procurement-pipeline inputs that vary on container type, package mode, route, and value range. If even one canary input causes the skill to fail or hang, the daemon will time-out on real inputs that share that shape.
- **3/4 failure paths passing means one error condition the skill mishandles silently.** The L5 freight engine's error-handling logic (e.g., emitting `FREIGHT_MODE_LOW_CONFIDENCE` on disagreement, retrying on CDP errors) depends on the skill returning the right `cdp_state` + `blocker_code` for each failure mode. A skill that returns `freight_status: "quoted"` when CDP is actually down (false-success) corrupts the engine's decision logic.
- **Step 09 bead audit reads TEST_REPORT.md as evidence**, not as opinion. Verdict BLOCKED or any "PASS" missing from the canary/failure-path tables fails the audit, which blocks step 10 (kickoff), which blocks all of L5 execution.
- **The L5→L6 readiness gate depends on the freight + customs engines being trustworthy.** A flaky skill at the bottom of the stack corrupts every downstream signal.

Partial completion is therefore not a "good enough" state — it's a guaranteed downstream regression. The Test/Validate/Fix loop iterates until 100% precisely because that's the production bar.

## What the Test/Validate/Fix loop actually does

The user (operator) explicitly requested: "after the handoff is done so that it actually uses CDP in the browser and tries to execute those things". The loop is now:

1. **C.0 Setup** — confirm CDP browser running, install schema validator (`ajv-cli` or `jsonschema`)
2. **C.1 Define canary inputs** — 4 Freightos canaries / 5 Taric canaries covering all gold-set categories
3. **C.2 Define failure-path matrix** — 4 scenarios per skill (captcha sim, CDP down, etc.)
4. **C.3 Execute loop** — for each canary: run 3-command happy path against live CDP → validate schema + golden assertions → if pass, save evidence; if fail, diagnose + fix + re-execute SAME fixture
5. **C.4 Execute failure paths** — verify error handling for each scenario
6. **C.5 (Taric only) HMRC cross-agreement** — semantically verify the cross-check signal actually agrees with HMRC on known codes
7. **C.6 Write TEST_REPORT.md** — final audit artifact with verdict + Phase A discoveries + quirks
8. **C.7 Commit + push** — both skill files and test evidence
9. **C.8 Loop-exit gate** — BLOCKING checkbox list; do not advance until 100% checked

## Step 09 bead audit verification

When step 09 (`docs/pipeline/l5-expanded-commercial-intelligence/09-audit-beads.md`) runs, it will verify:

- `skills/landed-cost/freightos-freight-estimate/SKILL.md` + `result.schema.json` exist
- `skills/landed-cost/taric-cross-check/SKILL.md` + `result.schema.json` exist
- `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/freightos-freight-estimate/TEST_REPORT.md` exists with verdict READY
- `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/taric-cross-check/TEST_REPORT.md` exists with verdict READY

If any are missing or any TEST_REPORT shows verdict BLOCKED, step 09 audit FAILS and step 10 (kickoff) cannot be authorized.

## When done

After both skills exist on disk, both TEST_REPORTs show verdict READY, both verification checklists are 100% green, and everything is pushed:

1. `git pull --rebase && git push` (one more time, to be sure)
2. Run `ls skills/landed-cost/` — confirm 6 skill directories (existing 4 + 2 new).
3. Run `ls docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/{freightos,taric}*/TEST_REPORT.md` — confirm both exist.
4. Run `br ready --label l5 --json | jq .` and proceed with L5 step 01.

## Why the loop is non-optional

The operator's exact ask was: "make sure that they are robust, completely functional, and these are the best handoffs you've ever made". A skill that *builds* but doesn't *empirically work end-to-end against live CDP* is not robust. The Test/Validate/Fix loop is the difference between "I have a SKILL.md" and "I have a working skill that has been proven against 4-5 real-world inputs and 4 failure modes". For a skill that will be invoked by the L5 daemon hundreds of times during production, the empirical-test evidence trail is the production-readiness gate.
