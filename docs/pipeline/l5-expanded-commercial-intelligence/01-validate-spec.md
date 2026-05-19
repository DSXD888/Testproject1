# L5 Expanded Commercial Intelligence — Validate Existing Spec for 3-SOT Pipeline

> **Pattern B handoff** — the L5 layer doc IS the spec. This is a validation pass, NOT research-to-spec.

## Your Mission

The spec for Layer 5 (Expanded Commercial Intelligence) already exists at `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` (520 lines). Before it enters the TDD pipeline, you must verify it has the structure needed for downstream coverage verification, surface the **nine cross-domain spec callouts** identified during pipemaxxer's codebase research (the 7 from `manifest.yaml#spec_callouts` plus 2 added 2026-05-08 after the operator validated existing landed-cost skill state), and resolve them in-place.

**This is NOT a rewrite.** The spec content has been deeply iterated on through three planmaxer cycles + research-doc harmonization. Your job is structural: requirement IDs, ACs, scope-drift scan, and resolution of the seven spec-vs-codebase mismatches captured below.

---

## Source Documents (Read In This Order)

### Primary Input (THE SPEC — READ FULLY)
```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md   — 520 lines
```

### Cross-Layer Context (SKIM)
```
docs/strategy/layer-strategy.md                                    — §10 Interface Contracts (L5↔L6 readiness gate)
docs/strategy/cross-reference-index.md                             — 280 lines; L5 referenced in 14 topic rows
docs/strategy/layers/layer-0-gold-set-evaluation.md                — gold-set + canary infrastructure L5 extends
docs/strategy/layers/layer-4-shared-visibility.md                  — sync engine + projection model L5 plugs into
```

### L4 Quality Bar (MATCH OR EXCEED THIS RIGOR)
```
docs/pipeline/l4-shared-visibility/archived/2026-05-08-l4-shared-visibility/01-validate-spec.md  — 30.7 KB; the bar
docs/pipeline/l4-shared-visibility/artifacts/l4-completion-audit-2026-05-08.md                   — 14.4 KB; final L4 audit verdict
```

### Codebase Reality Check (STUDY ONLY THE NAMED LINES)
```
migrations/0001_sqlite_state_initial.sql:243-296     — supplier_quotes table (closes at line 296)
migrations/0001_sqlite_state_initial.sql:322-394     — landed_cost_estimates table (closes at line 394); ALL 36 decision-grade columns nullable from L0
migrations/0001_sqlite_state_initial.sql              — entire 545-line file; verify product_commodity_codes is ABSENT
infra/supabase/migrations/0001_projection_tables.sql  — verify product_commodity_codes IS present in projection
crates/daemon/src/errors/manifest.rs:20-55          — ErrorCode enum with 44 current variants
crates/daemon/src/db/sync.rs:24-49                    — PUSH_OWNED_RESEED_TABLES; landed_cost_estimates at line 35
crates/daemon/src/sources/registry.rs:570-680         — supplier-enrichment / procurement-ingestion source block
crates/daemon/src/scheduler/cron.rs:16-34            — CronJobDefinition + existing entries
skills/supplier-price-enrichment/{ebay,alibaba,amazon,1688}/SKILL.md — verify all four exist; record promotion_stage
skills/procurement-ingestion/                         — verify pcs/ does NOT exist
AGENTS.md:262-305                                     — authoritative `br` rule block
AGENTS.md:666-723                                     — duplicated `bd` block (deprecated — flag in working notes)
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**If it's in the L5 layer doc, it gets implemented. No exceptions.**

Your job is to IMPROVE quality, clarity, and implementability. It is NEVER to reduce scope.

You may improve HOW a feature is described, resequence WHEN it is built, or clarify ambiguous wording. You may NEVER:

- Remove features from scope
- Defer features to "post-MVP," "Phase 2+," or any non-existent future iteration
- Label features as "nice to have," "stretch goal," or "out of scope" (the spec already lists explicit Open Questions in §3.6 — those are the ONLY deferred items, and they have research-back-references)
- Silently drop features during validation
- Reduce acceptance criteria below what the spec defines
- Treat later subsections as less important than early ones

The five Open Questions in §3.6 (logistics defaults, Chinese supplier sources, customs-cost spread materiality threshold, packaging reclassification, CBAM rate timeline) are explicitly back-referenced to research §10 Further Research #2/#3/#8/#9/#10. Those references are evidence the spec authors meant to leave them open. You do NOT close them and you do NOT reduce L5 scope to exclude them — you preserve the open question record verbatim.

---

## Key Context From Codebase Research (CRITICAL — Read Before Validating)

These are findings from pipemaxxer's Phase 1 research. Several are blocking spec issues.

1. **All 36 decision-grade columns on `landed_cost_estimates` already exist as nullable from L0.** Verified against `migrations/0001_sqlite_state_initial.sql` lines 322-394. L5 adds NO columns to this table. Spec wording "No new tables. No new columns." in §1.3 is accurate at the local SQLite level.

2. **`product_commodity_codes` junction table is NOT in any local SQLite migration (0001..0012).** It exists ONLY in `infra/supabase/migrations/0001_projection_tables.sql` and `0004_rls_policies.sql`. The spec's §1.3 row "`product_commodity_codes` junction, existed since L0" is wrong as written about local state. **This is spec callout #1 — must be resolved.**

3. **`skills/supplier-price-enrichment/{alibaba,amazon,1688}/` ALL ALREADY EXIST.** Each contains `SKILL.md`, `checkpoint.schema.json`, `result.schema.json`. All at `promotion_stage: candidate`. The Alibaba SKILL.md is the most developed (241 lines). The spec's §1.15 "New Skills (Layer 5)" framing is misleading — the actual L5 task is to MATURE these from candidate to stable. **Spec callout #2.**

4. **`skills/procurement-ingestion/pcs/` does NOT exist.** Truly net-new — operator confirmed IN-SCOPE 2026-05-08 (manifest `recorded_decisions.decision_2_pcs_scotland_scope`). Phase 5 creates the full skill directory. **Spec callout #3 — RESOLVED.**

5. **`ErrorCode` enum in `crates/daemon/src/errors/manifest.rs:20+` has ~44 variants today.** (NOT in `validation/policy.rs` — that file holds `ValidationPolicyMode` / `ValidationTrigger` for L3 logic.) L5 adds 10 more variants (§3.5). This is a Rust enum — recompilation and exhaustive match updates are required across the daemon. The skill plug-and-play rule (L0 S4) does NOT apply here. **Spec callout #4.**

6. **HMRC FPO endpoint name is unverified.** Spec §1.4 already flags this: "FPO on HMRC's developer portal refers to Fast Parcel Operator — verify the correct API endpoint name during implementation". **Spec callout #5 — keep visible to TDD.**

7. **Cron scheduler at `crates/daemon/src/scheduler/cron.rs` already has 2 entries** (validation calibration sweep, cross-model audit). L5 §1.11 + §2 add 4 more (HMRC tariff news watcher, TRA case register, FX monthly rollover, Freightos/SeaRates source-health). Pattern is established. **Spec callout #8.**

8. **No `.github/workflows/` exists.** The L4 quality gate is local-only. L5 inherits this. **Spec callout #9.**

---

## Step-by-Step Workflow

### Step 0: Read All Source Documents (MANDATORY)

Read these in order before touching the spec:
1. `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — **READ FULLY** (520 lines).
2. `docs/strategy/layer-strategy.md` §10 Interface Contracts — **REFERENCE** (L5↔L6 gate).
3. `docs/strategy/cross-reference-index.md` — **SKIM** (find every L5 mention).
4. `docs/pipeline/l4-shared-visibility/archived/2026-05-08-l4-shared-visibility/01-validate-spec.md` — **STUDY** (this is your quality bar; match its rigor).
5. The codebase reality-check files listed above — **STUDY ONLY THE NAMED LINES** (do not read entire daemon source files).

### Step 1: Validation Checklist (work through systematically)

#### 1.1 Requirement IDs

Today the spec uses ad-hoc subsection numbering (§1.1 through §3.10). Downstream coverage verification needs stable, unique requirement IDs.

- [ ] Add stable requirement IDs of the form `L5-FR-X.Y.Z` to every requirement, where X = main section (1, 2, 3), Y = subsection (1..16 in §1, 1..4 in §2, 1..10 in §3), Z = ordinal within the subsection (1, 2, 3, ...).
- [ ] Every functional requirement has exactly one ID. The 12 DoD criteria in §3.1 become `L5-FR-3.1.1` through `L5-FR-3.1.12`. The 10 customs error types in §3.5 become `L5-FR-3.5.1` through `L5-FR-3.5.10`.
- [ ] Add IDs for sub-rules within each subsection. Examples: `L5-FR-1.4.1` "Five parallel recall sources", `L5-FR-1.4.2` "Critical rule — FPO agreement increases confidence; FPO alone is not evidence", `L5-FR-1.4.3` "UK-specificity rule — Taric Support / HS Ping never overrule HMRC on UK national measures".
- [ ] A `grep -E '^L5-FR-[0-9]+\.[0-9]+\.[0-9]+'` over the spec (or a shell script that extracts IDs) must return ALL requirements without ambiguity.

#### 1.2 Acceptance Criteria

The spec mostly uses tables and bullet lists which is good. But several requirements are buried in narrative.

- [ ] Every requirement with an `L5-FR-X.Y.Z` ID has explicit, bullet-listed acceptance criteria immediately below it (or in a §3.3-style scenario row that maps back to the ID).
- [ ] §1.7 Measure Interaction Engine — extract narrative AC bullets (additive vs substitutive vs conditional rules per measure family) into a numbered AC list under a stable ID.
- [ ] §1.10 Valuation Mode — extract narrative ("Invoice price is NOT customs value...") into numbered AC.
- [ ] §1.13 Customs Branch Divergence — the screening-grade auto-pass condition (lines 207-210) becomes three numbered ACs.
- [ ] No AC says "handle appropriately", "as needed", "if relevant", or any vagueness equivalent.
- [ ] If an AC is currently un-testable from the spec text alone, either (a) tighten the wording with measurable thresholds (basis points, exact field values) or (b) note "AC depends on calibration during step 03 TDD" and leave the bar specific enough that the TDD step can produce a test.

#### 1.3 Structure

- [ ] Section numbering is consistent (3 main sections, no skipped subsection numbers).
- [ ] Every table that defines a behavior (recall sources, exporter rate basis, conservative defaults, freshness vectors, customs error types, evidence tiers) is preceded by its requirement ID(s).
- [ ] §2.4 Existing Field Activation table — every row has a corresponding requirement ID anchoring it to populating logic in §1 / §2.1 / §2.2.
- [ ] No "TBD" sections; no `[placeholder]`; no half-written paragraphs.

#### 1.4 Completeness

- [ ] Error paths are defined for each new external API: HMRC Trade Tariff (rate-limit, schema drift, downtime), HMRC FPO (unavailable, malformed response), HMRC Exchange Rates (missing month, parse failure), Xe (rate-limit, auth fail), Freightos (no quote, mode unsupported), SeaRates (no quote, schema drift), Taric Support (cross-check disagreement), HS Ping (advisory only).
- [ ] Edge cases for FX monthly rollover are explicit: what happens when an in-flight estimate spans the rollover boundary, what happens when a historical month's rate later changes (HMRC corrections).
- [ ] CBAM forward placeholder behavior is explicit for tender delivery windows that cross 2027-01-01 (today is 2026-05-08, so this boundary is within ~8 months).
- [ ] No "to be determined later" items — if it's in §3.6 Open Questions, it has a research-back-reference and stays open; if it's anywhere else, it must be defined.

#### 1.5 Scope Preservation

- [ ] Scan the entire spec for: `post-MVP`, `Phase 2+`, `stretch goal`, `nice to have`, `deferred`, `out of scope`, `future iteration`, `simplified for initial implementation`, `can be added later`. Use grep:
      ```bash
      rg -in 'post.?mvp|phase 2\+|stretch goal|nice to have|deferred|out of scope|future iteration|simplified for initial|can be added later' docs/strategy/layers/layer-5-expanded-commercial-intelligence.md
      ```
- [ ] Each match must be either (a) an Open Question in §3.6 with a research-back-reference, (b) the CBAM forward placeholder note (which is explicitly bounded by regulation timing, not de-scoping), or (c) removed/rewritten to remove the scope-reduction implication.
- [ ] The five §3.6 Open Questions retain their research back-references. Do NOT remove them. Do NOT close them prematurely. Annotate that step 05 strategy must convert them into either pre-implementation decisions or risk-managed unknowns — not into silent de-scopings.

### Step 2: Resolve the Nine Cross-Domain Spec Callouts

Callouts 1-5 and 8-9 are the original 7 from `manifest.yaml#spec_callouts`. Callouts 6-7 were added 2026-05-08 after the operator validated existing landed-cost skill state (skill-first architecture confirmation, operator-built prerequisites).

Each of these MUST end with a recorded resolution in the spec (or a deliberate non-resolution note). Edit the spec inline; commit message convention: `l5-validate-spec: resolve callout N (<topic>)`.

#### Callout 1 — `product_commodity_codes` locality

**Edit `§1.3 Data Changes`.** The current row reads "`product_commodity_codes` junction (existed since L0)". Replace with the chosen resolution (RECOMMENDED: option A):

- **Option A (recommended):** L5 creates the SQLite junction in a new migration `migrations/0013_l5_product_commodity_codes.sql` (or whatever next-free `NNNN_*.sql` is at the time). Spec gains a one-line note: "L5 introduces the local SQLite junction; the Supabase projection of the same name (created in L4) becomes the push target."
- **Option B:** Store commodity-code candidates as a JSONB column on `product_candidates` (existing) and the L4 sync layer denormalizes to the Supabase junction. Requires a sync engine change — more invasive. Document the additional work explicitly.

**Choose A unless Option B's sync change is small.** Record the choice in spec wording. This is a clarification of HOW, not a reduction of WHAT.

#### Callout 2 — Marketplace skill authorship framing

**Edit `§1.15 New Skills (Layer 5)`.** Replace the table heading "New Skills" with "Layer 5 Skill Work" and split rows into two groups:

- **Mature from candidate to decision-grade:** `alibaba/`, `1688/`, `amazon/` — list the actual delta per skill (from the existing SKILL.md to the L5 target):
  - Alibaba: add Chinese retry on zero/accessory/consumer hits, "Ready to Ship" filter with `quoteUsabilityConfidence` bump, source-health canary expansion, anti-bot circuit breaker, validator overlay binding.
  - 1688: short Chinese-keyword-only clusters (2-4 terms), domestic wholesale floor pricing, auth-refresh/1688 skill coordination, validator overlay binding.
  - Amazon: alias families (multiple short noun-phrase variants), structured title vocabulary targeting, browser-first strategy (PA-API 5.0 sunset 2026-04-30), validator overlay binding.
- **Truly new:** `pcs/` — operator confirmed IN-SCOPE 2026-05-08. Full skill directory standard (SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml). The row stays in §1.15.

#### Callout 3 — PCS Scotland scope decision (RESOLVED 2026-05-08: IN-SCOPE)

**Edit `§3.6 Open Questions`.** Under the Chinese supplier source question (§10 Further Research #3), record: "Scotland scope decision: IN-SCOPE — confirmed by operator 2026-05-08 via pipemaxxer AskUserQuestion. PCS skill is implemented in Phase 5; full bead set assumes Scotland coverage. Research back-reference (§10 Further Research #3) preserved as historical context." The PCS row in §1.15 stays in.

#### Callout 4 — ErrorCode enum extension framing

**Edit `§3.5 Customs-Specific Error Types`.** Add a one-paragraph preface: "These 10 error codes are added to the daemon's `ErrorCode` enum at compile time (Rust). They are NOT skill-overlay-only error codes. Adding them requires updating `crates/daemon/src/errors/manifest.rs` (the enum at line 20+ and its `as_str()` matcher in the same file) plus any exhaustive switch sites surfaced by `cargo clippy --all-targets -- -D warnings` — typically 5-15 sites including `validation/policy.rs`, `validation/executor.rs`, `errors/markdown.rs`, `tracing/structured.rs`, `ipc/handlers/*`. The skill plug-and-play rule (L0 S4) governs skill manifests, not daemon enum vocabulary."

#### Callout 5 — HMRC FPO endpoint verification (CORRECTED 2026-05-08)

**Edit `§1.4 Multi-Source HS Classification Recall`.** The current parenthetical "(note: 'FPO' on HMRC's developer portal refers to Fast Parcel Operator — verify the correct API endpoint name during implementation)" is good. Add: "Step 03 TDD produces an investigation bead `l5-customs-hmrc-fpo-endpoint-discovery` that determines: (a) does the existing public Trade Tariff `/api/search` / `/api/search_suggestions` endpoint suffice for ML-powered suggestion? (b) does a public FPO endpoint exist (no auth)? (c) only if both fail, does HMRC Developer Portal registration become a deferred operator decision."

**Important correction:** the HMRC **Trade Tariff classification API** itself (`trade-tariff.service.gov.uk/uk/api/commodities/{code}`) is fully PUBLIC — no auth, no API key, no developer portal needed. The existing skill `skills/landed-cost/hmrc-customs-classification/SKILL.md` already uses it without credentials. The "Developer Portal" concern only applies to the FPO endpoint specifically (if it exists as a separate authenticated service).

#### Callout 6 — Existing landed-cost skills DISCOVERED (RESOLVED 2026-05-08)

**This is a new resolved callout added 2026-05-08 after the operator validated existing skill state.**

Three landed-cost skills already exist (built during stages 1-14 QA after L3) and cover services the L5 spec listed as "Required" paid APIs:

| Skill | Spec service it replaces | Auth? | Free? |
|---|---|---|---|
| `skills/landed-cost/hmrc-customs-classification/` | HMRC Trade Tariff API | No (public) | ✓ |
| `skills/landed-cost/simple-fx-lookup/` (via Frankfurter) | Xe Currency Data API | No (free public) | ✓ |
| `skills/landed-cost/searates-freight-estimate/` (CDP browser) | SeaRates paid API | No (CDP session) | ✓ |

**Edit `§3.9 Dependencies`.** Update the dependencies table to reflect the skill-first reality:
- HMRC Trade Tariff: Required, NO API key (public endpoint)
- HMRC Exchange Rate Service: Required, NO API key (free monthly publication via gov.uk)
- Xe: DECLINED — Frankfurter (free, ECB-backed) replaces it
- SeaRates paid API: DECLINED — existing CDP skill replaces it
- Freightos paid API: DECLINED — operator-built CDP skill (skills/landed-cost/freightos-freight-estimate/) replaces it
- Taric Support paid vendor: DECLINED — operator-built CDP skill on EU TARIC database (skills/landed-cost/taric-cross-check/) replaces it
- HS Ping: DECLINED for L5 (4 other recall sources sufficient)
- Dutify/Zonos/Avalara: DECLINED for L5 (validation-only)

**This aligns L5 with project memory's "Skill-first agent scraping via browser automation (NOT API connectors)" architecture.** Zero new commercial API accounts needed.

#### Callout 7 — Operator-built prerequisites (NEW)

Two skills will be **built by the operator personally** before step 07 (bead creation) finalizes:

1. `skills/landed-cost/freightos-freight-estimate/` — CDP browser skill, parallel to existing SeaRates skill.
2. `skills/landed-cost/taric-cross-check/` — CDP browser skill targeting public EU TARIC database (taric.ec.europa.eu).

These are **prerequisites for L5 implementation**, NOT pipeline beads. The bead audit (step 09) verifies they exist before authorizing execution. Phase 3 (Freight) and Phase 4 (Customs multi-source recall) beads cannot start without them.

#### Callout 8 — Cron scheduler extension

**Edit `§1.11 Classification Freshness System`.** Add a one-line note: "Implementation: 4 new `CronJobDefinition` entries in `crates/daemon/src/scheduler/cron.rs` follow the existing pattern (see `VALIDATION_CALIBRATION_READINESS_SWEEP_CRON` and `VALIDATION_CROSS_MODEL_AUDIT_CRON` at lines 77-86)."

#### Callout 9 — No CI workflows

**Edit `§3.1 Definition of Done` (or `§3.7 Risks` if cleaner there).** Add criterion #13 (or risk row): "Local quality gate is the only gate (no `.github/workflows/`). The L5 implementation is verified by the same six-command sequence used in L4: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon && bun run build && bun run test`. If a CI workflow is desired, it is a separate, owner-authorized scope addition — NOT silently in scope."

### Step 3: Verify Output

After edits:
- [ ] `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` has stable `L5-FR-X.Y.Z` IDs throughout.
- [ ] All 9 callouts have recorded resolutions (or deliberate non-resolution notes for #5 — HMRC FPO endpoint discovery is deferred to step 03 TDD; Callout #3 PCS Scotland is RESOLVED IN-SCOPE 2026-05-08).
- [ ] No scope-drift language remains (run the grep from step 1.5).
- [ ] Line count is within +/- 100 lines of the original 520 (clarification + IDs add lines; deletions are minimal). Significantly more than +200 lines is a signal you've over-edited.
- [ ] Front-matter or top-of-doc note records the validation date (today: 2026-05-08) and pipemaxxer pipeline reference.

### Step 4: Follow-Up Pass (Sub-Optimization — same session)

> Run after Step 3 completes. Do not skip.

**Re-read your edits with fresh eyes. Specifically:**

1. Open `migrations/0001_sqlite_state_initial.sql` lines 322-394 and walk every column on `landed_cost_estimates`. For each column, find the L5-FR-X.Y.Z requirement that populates it. If a column has NO populating requirement, that's either dead schema (unlikely — it shipped in L0 deliberately) or a missed L5 capability (more likely). Add the missing requirement to the spec — the L0 column was reserved for a reason.

2. Open `crates/daemon/src/errors/manifest.rs:20-55` and list the 44 current `ErrorCode` variants. Cross-check against your spec's §3.5 — confirm the 10 new variants don't collide with existing names. (e.g., `FpoAccessUnavailable` already exists; the spec's `FPO_UNAVAILABLE` looks similar — decide whether they're the same code or genuinely distinct. Same-meaning duplicates are bugs.)

3. Open `crates/daemon/src/sources/registry.rs:570-680` and confirm the source registration block. The 4 marketplace skills (alibaba, amazon, 1688, [pcs]) need entries here. Is the file structured to accept a new row at the right place? Note the exact insertion point in your validation working notes (this feeds the bead creation step).

4. Open `skills/supplier-price-enrichment/alibaba/SKILL.md` (the most developed of the four) and read the Chinese retry section, anti-bot logic, and result schema. Compare against the spec's §2.3 Alibaba row. Are there contradictions? Are there features in SKILL.md that the spec doesn't mention? Bring discrepancies into the spec — the running SKILL.md is closer to ground truth than a months-old layer doc.

5. Open `docs/strategy/cross-reference-index.md` and grep for `L5`. Every cross-reference must resolve to a stable `L5-FR-X.Y.Z` ID after your edits. Update the index if any references became stale.

6. Re-run the scope-drift grep. Re-count `L5-FR-X.Y.Z` IDs. The total count should now be derivable mechanically and be the source-of-truth requirement count for the next step's TDD coverage matrix.

### Step 5: Scope-Preservation Check (MANDATORY — BLOCKING)

After the sub-optimization, perform this check:

1. **Enumerate** every numbered subsection in the spec (1.1 through 1.16, 2.1 through 2.4, 3.1 through 3.10).
2. **For each subsection**, list its requirement IDs. The total must be ≥ 60 (spec is 520 lines with dense subsection content; ~3 IDs/subsection is the floor).
3. **Verify** every numbered table row in §2.4 Existing Field Activation maps to an ID in §1.x or §2.x — each L0-reserved column has a populating requirement.
4. **Scan** for these red-flag phrases (case-insensitive): `post.?mvp`, `phase 2\+`, `stretch goal`, `nice to have`, `deferred to`, `out of scope`, `future iteration`, `simplified for initial`, `can be added later`. Allowed only inside §3.6 Open Questions with research back-references, and inside the CBAM §1.14 placeholder note.
5. **If any feature is missing or any red-flag remains** — BLOCKING failure. Fix before completing.

### Step 6: Transcribe Operator Decisions (MANDATORY — already resolved 2026-05-08)

The operator has pre-resolved all four required decisions via AskUserQuestion on 2026-05-08. They are recorded in `manifest.yaml#recorded_decisions`. Your job here is to **transcribe them verbatim into the spec** (front-matter or a new `## L5 Pipeline Decisions Recorded 2026-05-08` section near the bottom of the spec). Do NOT re-decide — these are operator decisions, not validator-agent decisions.

#### Decision 1 — `product_commodity_codes` ownership: **Option A** (L5 SQLite migration)

Transcribe to spec:
```
DECISION 1 — product_commodity_codes ownership: Option A — L5 SQLite migration
  Recorded: 2026-05-08 by operator (via pipemaxxer AskUserQuestion)
  Rationale: Recommended default. Cleaner local state; no sync-engine impact. Supabase projection (created in L4) becomes the push target.
  Implementation: migrations/NNNN_l5_product_commodity_codes.sql (NNNN = next-free at the time, currently 0013)
```

Step 01 §2 Callout #1 resolution: edit spec §1.3 to clarify "L5 introduces the local SQLite junction; the Supabase projection of the same name (created in L4) becomes the push target."

#### Decision 2 — PCS Scotland scope: **IN scope**

Transcribe to spec:
```
DECISION 2 — PCS Scotland scope: IN-SCOPE
  Recorded: 2026-05-08 by operator (via pipemaxxer AskUserQuestion)
  Implementation: skills/procurement-ingestion/pcs/ created in Phase 5 with full skill directory standard (SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml). Daemon registry entry in sources/registry.rs. Bootstrap covers PCS OCDS Web API setup. Adds ~3-5 beads to Phase 5.
```

Step 01 §2 Callout #3 resolution: edit spec §3.6 Q3 to record the closure "Decision made 2026-05-08: IN scope. See `## L5 Pipeline Decisions Recorded` below." Preserve the research back-reference (§10 Further Research #3) as historical context. PCS row stays in §1.15 unconditionally.

#### Decision 3 — Implementation epic structure: **Option (b) sister epic**

Transcribe to spec:
```
DECISION 3 — L5 implementation epic structure: Option (b) — sister epic
  Recorded: 2026-05-08 by operator (via pipemaxxer AskUserQuestion)
  Rationale: Recommended default. Clean separation between layer-doc-authoring epic (testproject1-4gh, children 4gh.1-4 already complete) and implementation work.
  Implementation: Step 07 runs `br create --type=epic --priority=P0 --title="Epic: L5 Implementation — Expanded Commercial Intelligence" --label l5` and captures the new epic ID into this section. All Phase 1-6 implementation beads thread under that new epic.
  New epic ID: <to be filled in step 07 after `br create` runs>
```

#### Decision 4 — Gold-set categories: **fasteners + packaging-cartons + examination-gloves**

Transcribe to spec:
```
DECISION 4 — Gold-set categories (DoD #12): fasteners + packaging-cartons + examination-gloves
  Recorded: 2026-05-08 by operator (via pipemaxxer AskUserQuestion)
  Rationale: Clear differentiation — remedy-active (fasteners) / packaging-easy (cartons) / regulated medical (examination gloves).
  Implementation: Phase 6 creates tests/fixtures/calibration/customs-classification/{fasteners,packaging-cartons,examination-gloves}/{canary.inputs.jsonl,golden.assertions.yaml,metadata.json,manifest.md} per L4 calibration corpus convention.
```

#### External API Account State (transcribe; do NOT escalate)

The operator confirmed on 2026-05-08 that **L5 has ZERO paid commercial API procurements** — see manifest `recorded_decisions.external_api_account_state` for the canonical inventory. Skill-first architecture replaces Xe/Freightos paid/SeaRates paid/Taric Support paid with existing or operator-built CDP browser skills + free public endpoints.

Bootstrap doc (step 03) must reflect, verbatim from the manifest:

- **Public APIs (no key, no portal):**
  - HMRC Trade Tariff classification API — public endpoint `https://www.trade-tariff.service.gov.uk/uk/api/commodities/{code}` consumed by existing skill `skills/landed-cost/hmrc-customs-classification/`. **No HMRC Developer Portal registration is required for this endpoint** (Callout 5 above corrects the earlier misconception).
  - HMRC Exchange Rates for Customs and VAT — free monthly publication via gov.uk (CSV/XML); extends existing `skills/landed-cost/simple-fx-lookup/` skill.
  - Commercial spot FX — Frankfurter (ECB-backed, no auth, free) via the same existing skill.

- **Existing CDP skills (already in place):**
  - `skills/landed-cost/searates-freight-estimate/` — browser-resident SeaRates Logistics Explorer session, NO paid API.

- **Operator-built prerequisites (built BEFORE step 07 bead creation; verified by step 09 audit):**
  - `skills/landed-cost/freightos-freight-estimate/` — CDP browser skill mirroring SeaRates pattern.
  - `skills/landed-cost/taric-cross-check/` — CDP browser skill targeting public EU TARIC database (`taric.ec.europa.eu`).

- **Declined / not in L5 scope:** Xe Currency Data API, Freightos paid API, SeaRates paid API, Taric Support paid vendor, HS Ping, Easyship/Shippo, CurrencyBeacon/OANDA, Dutify/Zonos/Avalara (the comparison adapters in `packages/intelligence/src/validation/` are comparison-only, not canonical — no vendor accounts).

This aligns L5 with project memory's "Skill-first agent scraping via browser automation (NOT API connectors)" architecture. Estimated annual savings vs the earlier paid-API plan: $3,000-10,000+ per year. The HMRC FPO endpoint name remains a TDD investigation item (Callout 5; bead `l5-customs-hmrc-fpo-endpoint-discovery`).

#### Marketplace Skill Maturation Baseline (transcribe; locks Phase 5 bead scope)

The operator confirmed alibaba/amazon/1688 worked cleanly during QA stages 1-14. Phase 5 maturation = **spec-listed deltas ONLY**. Transcribe to spec or store in handoff working notes:

- Alibaba: Chinese retry on zero/accessory hits + Ready-to-Ship filter + source-health canary expansion + validator overlay binding + candidate→stable promotion.
- 1688: Chinese-only short clusters (2-4 terms) + domestic wholesale floor pricing + auth-refresh/1688 binding + validator overlay binding + candidate→stable promotion.
- Amazon: alias families + structured title vocabulary + browser-first (PA-API 5.0 sunset 2026-04-30) + validator overlay binding + candidate→stable promotion.

NO additional gap-fill work. NO speculative hardening beyond spec §1.15 + §2.3.

These transcriptions are pre-conditions for step 03 (TDD) generation and step 07 (bead creation). Step 02 (optimize-spec) verifies they appear in the spec.

---

### Step 7: Final Fresh-Eyes Pass (MANDATORY — Run After All Prior Steps Complete)

> Step 4 was an early fresh-eyes pass over your structural edits. This step is a SECOND, final re-read AFTER the decision transcription (Step 6) is written into the spec. Skip ONLY if you completed Step 6 before Step 4 — otherwise the early pass missed the transcribed-decisions section, and a second pass is required.

Open the spec one more time as if you've never read it. The agent picking up step 02 (optimize-spec) and step 03 (TDD generation) will read this exact file. You are the last gate between them and confusion.

1. **End-to-end scan.** Read from front-matter through `## L5 Pipeline Decisions Recorded 2026-05-08` to the closing of §3.10. Did any of the four transcribed decisions silently contradict text earlier in the spec (e.g., §3.6 Q3 still hedging on PCS Scotland after Decision 2 declared IN-SCOPE)? Resolve contradictions in favor of the Decisions section — it's the latest authority.

2. **Decisions ↔ Callouts cross-check.** For each of the 4 transcribed decisions, find the callout(s) it resolved (Decision 1 ↔ Callout 1; Decision 2 ↔ Callout 3; Decision 4 ↔ DoD §3.1 #12 + Callout 6 implication for category-specific fixtures). Confirm each callout's resolution wording is consistent with the decision. If Callout 1 still says "Choose A unless Option B's sync change is small" while Decision 1 fixes Option A, tighten Callout 1 to read "Resolved: Option A (per Decision 1, 2026-05-08)".

3. **Skill-first invariant check.** Search the spec one more time for any residual paid-API procurement language outside the External API Account State block. Allowed: the comparison adapters in `packages/intelligence/src/validation/` (declared comparison-only — no vendor accounts). Forbidden: any DoD criterion, §3.9 Dependencies row, or risk that still names Xe / Freightos paid / SeaRates paid / Taric Support paid as a procurement item. If found, edit to match `## L5 Pipeline Decisions Recorded → External API Account State`.

4. **Requirement ID continuity.** Re-run `rg -o 'L5-FR-[0-9]+\.[0-9]+\.[0-9]+' docs/strategy/layers/layer-5-expanded-commercial-intelligence.md | sort -u | wc -l`. The number must match what you recorded after Step 4 (no IDs lost during Step 5/6 edits). Re-run the scope-drift grep one final time.

5. **L5→L6 gate preservation.** Search for `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`, `classification_fresh_until`. All five must appear in §3.5 (or §3.8 Forward-Compatibility) verbatim. Cross-reference-index.md row 229 cites these by name — the spec must keep them visible.

6. **Reader test.** Ask yourself: if you handed this spec cold to the TDD-author agent in step 03 with NO context about pipemaxxer history, would they:
   - Know what 4 decisions were pre-resolved on 2026-05-08? (Decisions section visible near the bottom.)
   - Know that L5 has zero paid-API procurement? (External API Account State block visible.)
   - Know the gold-set trio is fasteners + packaging-cartons + examination-gloves? (Decision 4 + DoD #12.)
   - Know PCS is unconditionally in scope? (Decision 2 + §1.15 PCS row + §3.6 Q3 closure note.)
   - Find every L5-FR-X.Y.Z with `grep`? (IDs stable, sequential, no gaps.)

   If ANY answer is "they'd have to ask" — fix the spec now. Do not punt to step 02.

7. **Commit.** `git add docs/strategy/layers/layer-5-expanded-commercial-intelligence.md && git commit -m "l5-validate-spec: <one-line summary>" && git pull --rebase && git push`. The spec is now ready for step 02 (optimize-spec) and step 03 (TDD generation).

---

## Output

The same spec file at `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`, validated and structured, with:
- Stable `L5-FR-X.Y.Z` requirement IDs throughout
- All 9 spec callouts resolved (Callouts 1, 2, 3, 4, 6, 7, 8, 9 RESOLVED; Callout 5 HMRC FPO endpoint deferred to TDD investigation bead `l5-customs-hmrc-fpo-endpoint-discovery` — explicit deferral with named bead, NOT silent de-scoping)
- All 4 operator decisions transcribed verbatim (`## L5 Pipeline Decisions Recorded 2026-05-08` section)
- Bumped version note at the top recording 2026-05-08 validation pass

Do NOT create a sibling validated-spec file. Edit the L5 layer doc in place; the layer doc IS the spec.

---

## Key Reminders

- The spec went through three full planmaxer cycles + harmonymaxer Run 002 — content is high quality. Do not rewrite well-written sections.
- Several seemingly-contradictory passages reflect deliberate hedges (e.g., "FPO is one recall source among several, not the primary classifier"). Preserve hedges; do not collapse them into simpler claims.
- The 10 customs error codes in §3.5 are tightly chosen — do not invent new ones during validation. If you find a missing error case, flag it for step 03 TDD discussion, not for spec editing.
- Bead CLI is `br` (NEVER `bd`). Bead sync is `bin/br-sync`. Cite AGENTS.md:262-305 (authoritative block); ignore the duplicated `bd` block at AGENTS.md:666-723.
- AgentMail key is `/Users/alinaaitova/Desktop/Testproject1`.
- **ZERO SCOPE DRIFT** — every feature in the L5 layer doc must remain in the validated spec.
