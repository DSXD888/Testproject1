# Goods-Led Install/Maintenance — Create Beads

## First Action

Invoke the `/beads-from-sources` skill.

---

## Your Mission

Create **self-contained, executable beads** from the 3 Sources of Truth (spec, TDD, strategy). Each bead embeds the relevant spec + TDD content **verbatim** — strategy provides ordering only, never content. An agent must be able to implement each bead from the bead alone.

**100% spec coverage is MANDATORY. If any spec feature has zero corresponding beads, this is a BLOCKING failure.**

> **{ENRICHMENT — after Batch 3}** If the enrichment pass has run, the strategy's actual phase names, count, and expected beads-per-phase are pinned here: `<phase names>`, `<N phases>`, `<~N beads total>`. The implementation epic ID (created in Step 1 below) is recorded in `manifest.yaml` → `implementation_epic` and the status file. If you still see this raw marker, read the strategy at `.../goods-led-install-maintenance-strategy.md` and mirror its phase→epic structure exactly.

---

## Source Documents (Read In This Order)

```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md       — SOT #1 — embed verbatim
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md          — SOT #2 — embed test IDs + assertions verbatim
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-strategy.md     — SOT #3 — ORDERING ONLY (phase → epic mapping, dependencies)
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-bootstrap.md     — environment setup — REFERENCE
```

### Codebase Context (so each bead names real files)
```
packages/intelligence/src/qualification/goods-qualification.ts:264-372   — Phase-1 branch + density + addressable value
packages/intelligence/src/customs/description-quality.ts:23-36,100-113,360 — service-gate split
crates/daemon/src/main.rs:5361-5432,8086,8114,8324,8548,8933,9064,9136-9290 — write struct, both rollups, persistence, live loop
packages/contracts-schema/src/index.ts:36,166,216                          — entities to extend
packages/intelligence/src/margins/coverage-rollup.ts                       — value-share calc + coverage metrics
skills/landed-cost/simple-fx-lookup/                                        — http_public skill template
skills/validation/overlay-supplier-enrichment/                             — overlay template
docs/strategy/layers/layer-0-gold-set-evaluation.md                        — whitelist doc to update
```

---

## Bead CLI + Coordination (THIS PROJECT)

- **Bead CLI:** `br` (NOT `bd`). Sync via `bin/br-sync` (NEVER raw `br sync`).
- **Issue prefix:** `testproject1` — all bead IDs are `testproject1-XXXX`.
- **Feature label:** apply `--labels goods-led-install` to EVERY bead in this feature.
- **Implementation epic:** create ONE sister epic (matches L4/L5/L6 pattern):
  ```bash
  br create --type=epic --priority=P0 \
    --title="Epic: Goods-Led Install/Maintenance Expansion + UK Labour-Cost Engine" \
    --labels goods-led-install
  ```
  Capture the resulting epic ID and write it into the status file + manifest under `implementation_epic`. Thread all phase sub-epics + task beads under it via `--parent`.

---

## Cardinal Rule: ZERO SCOPE DRIFT

**Every spec feature → at least one bead. 100% coverage is mandatory.** Two tiers (operator-decided 2026-06-02 — see `manifest.yaml` SCOPE TIERS):
- **TIER A** (MVP + §2.4 "Phase 2 / Should Have"): full implementation bead(s) with embedded spec + TDD content, a Files section, and named-test ACs.
- **TIER B** (the three §2.4 "Future" / §10 "requiring further research" items: live enrichment lane, installer-quote ingestion, detailed per-authority/per-zone permit sub-model): **one research-spike bead each** whose acceptance is a research/design output (extension-point design + a decision on the §10 question), NOT a code/test deliverable. Label them `goods-led-install` + `research-spike`.

You may improve HOW beads are decomposed. You may NEVER:
- Create beads that cover only a subset of the spec.
- Drop a Tier-A spec feature because it's "Phase 2" in the research doc — those are in scope; they get full beads.
- Drop a Tier-B item entirely — each still gets a tracked research-spike bead.
- Write a bead that says "implement the rest later."
- Give late Tier-A epics fewer/thinner beads than early ones when the spec assigns them equal surface.
- Over-scope a Tier-B item into a full implementation epic (false precision — it's a research spike).

If a Tier-A feature has no bead, or a Tier-B item has no research-spike bead, the bead set is INCOMPLETE.

---

## Key Context From Codebase Research — make beads concrete

- **Qualification beads** name `goods-qualification.ts:319-372` (Phase-1 branch after line 353; separable-mixed downgrade) and `description-quality.ts:23-36` (add `goods_led_mixed` to the union). The service-gate bead MUST also create the NEW `description-quality.test.ts` (zero coverage today — the bead is incomplete without it).
- **Rollup beads** name BOTH `main.rs:8114` (domestic) and `main.rs:8548` (import) — one bead must not fix only one site. New columns nullable/default-0. Persist the goods-only subtotal (audit finding #5).
- **Schema bead** touches all 4 epoch constants + Supabase `schema_metadata` + the projection migration — do not split this across beads or one constant gets missed.
- **Labour skill beads** declare `interaction_profile: http_public`, ship the 4 reference JSON files skill-local, include `## Non-goals`/`## Failure Codes`/`## Resume Strategy` + the run-manifest token block, and must NOT add the skill to `CDP_BROWSER_SKILLS`/`SIGNED_IN_BROWSER_SKILLS`.
- **Test beads** embed the exact test IDs + pence-explicit asserts (TS-QUAL-003a/b, TS-QUAL-006/007/008, TS-COV-007/008/009, the 8-row truth table, the 11 `installation_labour_engine.rs` manifests, the 7 canary cases incl. the Scenario-1 NW band 210000–265000).

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)
Read spec + TDD FULLY; strategy for phase→epic ordering; bootstrap for environment.

### Step 1: Create the implementation epic
Run the `br create --type=epic` command above. Capture the epic ID.

### Step 2: Invoke `/beads-from-sources`
- **Spec / TDD / Strategy:** the three artifact paths above.
- **Bead CLI:** `br`; label `goods-led-install`; parent = the epic from Step 1.
- Embed spec + TDD content verbatim into each bead. Strategy gives ordering only.

### Step 3: Feature-Specific Guidance
- Mirror the L4/L5/L6 structure: phase sub-epics under the implementation epic, task beads under phase sub-epics.
- Each bead's acceptance criteria = the exact TDD test(s) it must make pass, embedded verbatim with pence-exact numbers.
- Add a `### Files to Create/Modify` section to every code bead (the `bin/br-claim` wrapper reserves these files; without it, agents must pass `--paths`).
- Encode dependencies: schema/types beads block consumer beads; labour-skill + rollup beads block the Phase-2-gate bead.

### Step 4: Verify Output
- [ ] Implementation epic created; ID captured.
- [ ] Every spec requirement → ≥1 bead (run a coverage pass).
- [ ] Every bead has embedded spec + TDD content + a Files section.
- [ ] Dependencies encoded (`br dep` / `--parent`).
- [ ] Every bead labelled `goods-led-install`.
- [ ] `bin/br-sync` run; `.beads/issues.jsonl` updated.

### Step 5: Follow-Up Pass (Sub-Optimization)
> Send as a follow-up message in the same session.

ULTRATHINK. Build a spec-feature → bead coverage matrix:
1. For EACH spec requirement (FR-Q/L/R/U/V or your spec's IDs), list the bead(s) that implement it. Any requirement with zero beads is a BLOCKING gap — create the bead.
2. Open `crates/daemon/src/main.rs:8114` and `:8548`. Is there a bead (or beads) covering BOTH rollup sites? A bead that mentions only one site is incomplete.
3. Is there a bead for the NEW `description-quality.test.ts` (not just the production change)? The gate has zero coverage today.
4. Is there a distinct full bead for the `MaintenancePeriodEstimate` entity + per-period margin check, and for access-equipment auto-selection, the rate-refresh skill, `overlay-labour-cost`, and the basic banded permit/TM resolver (all Tier-A)? And a single `research-spike`-labelled bead each for the three Tier-B items (live enrichment lane, installer-quote ingestion, detailed per-authority/per-zone permit sub-model) — present but NOT expanded into implementation epics?
5. Count beads per phase epic. Are late epics realistically sized, or did they get 1–2 stub beads? Equalize to match spec surface.

### Step 6: Scope-Preservation Check (MANDATORY — BLOCKING)
1. Enumerate every spec feature (both tiers).
2. Map each TIER-A feature to ≥1 full bead; each TIER-B item to exactly one `research-spike` bead.
3. Scan TIER-A beads for "post-MVP," "Phase 2+," "stretch goal," "deferred," "out of scope," "simplified for initial," "add later." (A Tier-B research-spike bead legitimately frames its work as research/design — correct, not drift.)
4. If any Tier-A feature has zero beads, or any Tier-B item has no research-spike bead → BLOCKING failure. Create the missing beads before proceeding.

---

## Key Reminders

- Beads embed spec + TDD VERBATIM. Strategy is ordering only.
- `br` + `bin/br-sync`; label every bead `goods-led-install`; one sister epic.
- The two halves ship together — the Phase-2-gate bead depends on the rollup+labour-skill beads.
- Tier-A = full beads; Tier-B = one `research-spike` bead each (tracked, not dropped, not over-scoped).
- **100% spec coverage is MANDATORY. Any Tier-A feature with zero beads — or any Tier-B item with no research-spike bead — is a BLOCKING failure.**
