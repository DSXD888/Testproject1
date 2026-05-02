# End-to-End Golden Journey on a Real ProContract Process (DGU)

ULTRATHINK and be meticulous.

## MISSION

Stages 1–14 have each been validated independently. There is **no single document** that walks one real procurement opportunity from initial ingestion through operator decision, showing every transition + every artifact + every state change in continuous narrative. Stakeholders ask "what does this product actually do?" — this handoff produces the answer as a stakeholder-ready trace document.

Target process: **DGU — Double-glazed unit (BS EN 1279-compliant)** at `proc-pc-7046e8ec-2734-ed11-8119-005056b64545`. This process already has the richest demo state in the live DB: 1 lot, 1 product candidate, 1 supplier quote, 1 landed cost estimate (with active operator override at qty=250), 1 net-margin assessment (review verdict), 1 manifest (clarify stoplight), 2 prior decision intents (1 superseded, 1 withdrawn).

This handoff is **discover → write → validate → refine in a loop** rather than build → test → fix. The deliverable is a polished narrative document, not new code (with one exception: if querying the DB exposes a data-quality issue significant enough to break the demo story, that's an in-scope finding to surface, not necessarily fix).

```
┌────────────────────────────────────────────────────────────┐
│                          THE LOOP                            │
│   1. Pre-flight (§0)                                         │
│   2. For each of stages 1-14: query DB + capture artifacts │
│      + write narrative section                              │
│   3. Read aloud — does it flow? does it answer "what       │
│      does this product do?"                                 │
│   4. Refine (terminology, transitions, evidence depth)     │
│   5. Validate against R.1..R.12                             │
│   6. Stakeholder-ready → done                                │
└────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## §0 — PRE-FLIGHT

| Claim | Verify |
|---|---|
| DGU process exists with a single notice + lot + candidate + quote + lce + net-margin + manifest + intents + override | `sqlite3 state/state.db "SELECT 'notice', COUNT(*) FROM contract_notices WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545' UNION ALL SELECT 'lot', COUNT(*) FROM notice_lots WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545' UNION ALL SELECT 'candidate', COUNT(*) FROM product_candidates WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545' UNION ALL SELECT 'quote', COUNT(*) FROM supplier_quotes WHERE product_candidate_id LIKE 'pc-7046e8ec-%' UNION ALL SELECT 'lce', COUNT(*) FROM landed_cost_estimates WHERE supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1' UNION ALL SELECT 'net_margin', COUNT(*) FROM candidate_net_margin_assessments WHERE product_candidate_id LIKE 'pc-7046e8ec-%' UNION ALL SELECT 'manifest', COUNT(*) FROM manifests WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545' UNION ALL SELECT 'intents', COUNT(*) FROM pending_operator_intents WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545' UNION ALL SELECT 'overrides', COUNT(*) FROM operator_input_overrides WHERE product_candidate_id LIKE 'pc-7046e8ec-%'"` |
| `process_stage_status` and `process_stage_heads` have rows for DGU showing each stage's history | `sqlite3 state/state.db "SELECT stage, scope_kind, state, updated_at FROM process_stage_status WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545' ORDER BY updated_at"` |
| `agent_session_logs` has entries for the agent-driven stages (Stage 1 procontract-discover, Stage 4 procontract-enrich-tender, Stage 11 hmrc_customs, Stage 13 manifest-generation if skill mode) | `sqlite3 state/state.db "SELECT skill_path, status, started_at FROM agent_session_logs WHERE skill_path LIKE '%enrich%' OR skill_path LIKE '%hmrc%' OR skill_path LIKE '%manifest%' ORDER BY started_at DESC LIMIT 20"` |
| Stage artifacts exist on disk under `artifacts/` for the agent-driven stages | `find artifacts/ -path '*7046e8ec*' \| head -10` |
| 14-stage pipeline definition matches the layered design: Stages 1-3 (Ingest/Normalize/Relate), Stage 4-6 (Acquire docs/Lots/Qualify), Stage 7-9 (Reclassify/Quoteability/Search), Stage 10-12 (Gross-margin gate/Landed cost/Net-margin gate), Stage 13-14 (Manifest/Decision) | Cross-reference `docs/strategy/layers/layer-3-operational-hardening.md` |

**Parallel-safety**: this handoff is read-only on production code AND state.db (queries only). Safe to run alongside any other handoff. If you want to re-run a stage to capture fresh artifacts (optional), document that as a state-modifying step.

---

## §1 — DECISIONS ALREADY MADE

### D1 — Use the EXISTING state, don't re-run unless necessary

The DGU process has been through the pipeline multiple times during prior validation work. Its current state IS the demo. Don't re-trigger stages unless capturing a "before/after" transition is essential to the story.

### D2 — Audience: technical stakeholder unfamiliar with this codebase

The narrative document targets someone who knows procurement (UK public sector buyer/supplier dynamics) but has NOT read the research doc, strategy docs, or any prior validation manifests. Define every domain term on first use. Provide cross-references to the strategy docs (`docs/strategy/layers/layer-N-*.md`) for readers who want to dig deeper.

### D3 — Format: narrative + evidence callouts

Markdown document at `docs/pipeline/golden-journey-DGU.md`. Each stage is a section with:

```markdown
## Stage N: <name>

### What this stage does (1 paragraph, plain English)
### What ran on DGU at this stage (timestamp, agent invoked, artifacts produced)
### What ended up in the DB (specific row IDs, key fields)
### What the operator (or downstream stage) sees
### Notable observations (rough edges, data-quality issues, gotchas)

> 📎 Strategy doc reference: `docs/strategy/layers/layer-N-...md` § ...
```

Total target: 1500-3000 lines. Keep prose tight; rely on tables for state inspections.

### D4 — Capture data-quality issues as findings, not bugs

If a stage's output looks wrong (e.g., the manifest stoplight is `clarify` but the reasoning isn't clearly displayed; a confidence score is suspiciously low; a basis_hash doesn't make sense), document it in a "Findings" section but do NOT fix unless it's a clear regression. The audience cares about understanding, not perfection.

### D5 — Optional fresh "live" demo at the end

After documenting the existing trajectory, OPTIONALLY do a fresh live capture: launch `bun tauri dev` (if Phase 2 live operational validation already completed), do a fresh decision on DGU, screenshot. Add as Appendix A "Operator's view today (live screenshot)". Skip if Phase 2 live validation hasn't happened yet.

---

## §2 — EXECUTION ORDER

1. Read `docs/strategy/layers/layer-3-operational-hardening.md` to refresh on the 14-stage definition.
2. For each stage 1 → 14:
   a. Query DB for DGU's data at that stage.
   b. Locate any agent_session_logs + artifacts files.
   c. Identify which skills ran, what they returned, what got persisted.
   d. Write the stage section per D3 template.
3. After all 14 sections written: read end-to-end. Smooth transitions. Add a 1-page executive summary at the top.
4. Add a "Findings" section listing any rough edges (data-quality, demo-readiness, missing evidence).
5. Add a "Strategy doc cross-reference index" appendix linking each stage to its strategy spec.
6. Optional Appendix A: live screenshot from desktop UI (if Phase 2 live validation done).
7. Validate against R.1–R.12.
8. Quality gates (minimal — this is mostly docs).

---

## §3 — STAGES TO COVER

The 14-stage definition (verify against strategy docs; this is the working assumption):

| # | Name | Layer | Agent or deterministic? |
|---|---|---|---|
| 1 | Ingest | L1 | Agent (procurement-ingestion/procontract-discover) |
| 2 | Normalize | L1 | Deterministic |
| 3 | Relate | L1 | Deterministic |
| 4 | Acquire docs | L1 | Agent (procurement-ingestion/procontract-enrich-tender) |
| 5 | Extract lots | L1 | Deterministic |
| 6 | Qualify lot | L1 | Deterministic |
| 7 | Reclassify candidates / Extract products | L2 | Agent or deterministic (varies) |
| 8 | Score quoteability | L2 | Deterministic |
| 9 | Search / price-signal match | L2 | Agent (marketplace-search) |
| 10 | Gross margin gate | L3 | Deterministic |
| 11 | Landed cost | L3 | Agent (HMRC customs + sizing + SeaRates + FX) |
| 12 | Net margin gate | L3 | Deterministic |
| 13 | Manifest generation | L3 | Agent (manifest-generation skill, with daemon fallback) |
| 14 | Operator decision | L3 | CLI / UI driven |

Confirm this list against the actual code dispatcher (`crates/daemon/src/main.rs:1246-1267` — the `--ingest <source_id>` match arms).

---

## §4 — REQUIRED TESTS

This handoff adds **no new tests**. The deliverable is documentation.

If a data-quality issue surfaces that's clearly a code bug (not just an aesthetic rough edge), file as follow-up bead. Don't fix in this handoff.

---

## §5 — ACCEPTANCE CRITERIA — R.1 to R.12

| # | Criterion |
|---|---|
| R.1 | Document `docs/pipeline/golden-journey-DGU.md` exists |
| R.2 | All 14 stages have a section with the D3 template (what it does / what ran / DB state / operator view / observations) |
| R.3 | Each stage section has at least 1 evidence callout (DB row IDs, artifact paths, or agent_session_logs entries) |
| R.4 | Each stage section cross-references the relevant strategy doc layer |
| R.5 | Executive summary (1 page max) at the top: "DGU is a UK university tender for double-glazed units; this document traces it through the 14-stage pipeline" |
| R.6 | Findings section listing data-quality / demo-readiness rough edges (e.g., "Aggregates' basis_undetermined warning is correct but reads as 'broken' to a stakeholder unfamiliar with the override mechanism") |
| R.7 | Strategy doc cross-reference appendix |
| R.8 | Document is reviewable by a technical stakeholder unfamiliar with the codebase (terminology defined; no unexplained jargon) |
| R.9 | Document is correct — every claim about DB state is verifiable via the included query |
| R.10 | DGU's state in state.db is unchanged after this handoff (or, if optional Appendix A re-ran a live demo, document the new state too) |
| R.11 | Optional: Appendix A includes 1 live screenshot from desktop UI |
| R.12 | Quality gates: `cargo fmt --check && cargo check --all-targets` clean (no code touched, but verify); 169 daemon tests still pass; per-file integration loop unchanged |

---

## §6 — VALIDATION QUERIES

Use these to populate evidence callouts. The pattern is the same per stage; substitute the table.

```bash
# Stage 1: ingestion
sqlite3 state/state.db "
SELECT id, source_id, source_notice_id, title, published_at, source_updated_at
FROM contract_notices WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
"

# Stage 2: normalize
sqlite3 state/state.db "
SELECT id, canonical_ocid, primary_notice_id, current_stage, buyer_name, title, latest_published_at
FROM procurement_processes WHERE id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
"

# Stage 3: relate
sqlite3 state/state.db "
SELECT * FROM process_relations WHERE process_id_a='proc-pc-7046e8ec-...' OR process_id_b='proc-pc-7046e8ec-...';
"
# Expected: empty (no related processes)

# Stage 4: acquire docs (look for procontract-enrich-tender session)
sqlite3 state/state.db "
SELECT skill_path, status, started_at, finished_at, run_manifest_path
FROM agent_session_logs WHERE skill_path LIKE '%procontract-enrich%' AND
  EXISTS (SELECT 1 FROM agent_session_jobs j WHERE j.session_id = agent_session_logs.session_id
          AND j.job_parameters_json LIKE '%7046e8ec%')
ORDER BY started_at DESC LIMIT 5;
"

# Stage 5: extract lots
sqlite3 state/state.db "
SELECT id, lot_key, title, estimated_value_pence, lot_kind
FROM notice_lots WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
"

# Stage 6: qualify lot
sqlite3 state/state.db "
SELECT id, lot_id, qualification_state, qualification_reasons_json, basis_hash
FROM lot_qualifications WHERE lot_id LIKE 'pc-7046e8ec-%';
"

# Stage 7-8: extract products / score quoteability
sqlite3 state/state.db "
SELECT id, name, candidate_kind, quoteability_score_bps, identity_confidence_bps
FROM product_candidates WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
"

# Stage 9: search / quotes
sqlite3 state/state.db "
SELECT id, source_label, listing_url, unit_price_gbp_pence, currency, listing_pack_qty
FROM supplier_quotes WHERE product_candidate_id LIKE 'pc-7046e8ec-%';
"

# Stage 10-11: gross margin / landed cost
sqlite3 state/state.db "
SELECT cpa.id, cpa.gross_margin_bps, cpa.gross_margin_verdict,
       lce.id AS lce_id, lce.decision_grade, lce.economic_landed_cost_gbp_pence,
       lce.commercial_estimate_confidence_bps, lce.confidence_breakdown
FROM candidate_pricing_assessments cpa
LEFT JOIN landed_cost_estimates lce ON lce.product_candidate_id=cpa.product_candidate_id AND lce.supplier_quote_id=cpa.selected_quote_id
WHERE cpa.product_candidate_id LIKE 'pc-7046e8ec-%';
"

# Stage 12: net margin gate
sqlite3 state/state.db "
SELECT product_candidate_id, verdict, reasons_json, basis_hash
FROM candidate_net_margin_assessments WHERE product_candidate_id LIKE 'pc-7046e8ec-%';
"

# Stage 13: manifest
sqlite3 state/state.db "
SELECT process_id, stoplight, suggested_action, manifest_basis_hash, artifact_hash,
       json_array_length(clarification_prompts_json) AS prompts
FROM manifests WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
"
# Also: locate artifact at artifacts/ to embed manifest content excerpt

# Stage 14: decisions + overrides
sqlite3 state/state.db "
SELECT idempotency_key, status, desired_disposition, decision_note, queued_at, superseded_by_intent_id
FROM pending_operator_intents WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545' ORDER BY queued_at;

SELECT product_candidate_id, reason_code, payload_json, provided_at, withdrawn_at, superseded_by_override_id
FROM operator_input_overrides WHERE product_candidate_id LIKE 'pc-7046e8ec-%' ORDER BY provided_at;
"

# Stage tracking summary
sqlite3 -header -column state/state.db "
SELECT stage, scope_kind, state, updated_at
FROM process_stage_status WHERE process_id='proc-pc-7046e8ec-2734-ed11-8119-005056b64545' ORDER BY updated_at;
"
```

For each agent-driven stage, also locate the run manifest:

```bash
find artifacts/ -path '*7046e8ec*' | head -20
# For each agent_session_logs.run_manifest_path: cat the file (it's MD), excerpt the relevant section
```

---

## §7 — FAILURE-MODE PLAYBOOK

| Symptom | Cause | Fix |
|---|---|---|
| A stage section has no DB rows | DGU didn't go through that stage in the current state.db; or queries are looking at wrong table | Cross-check against process_stage_status. If genuinely never ran, document and explain why (e.g., Stage 3 produced no relations because no other processes share its OCID — that's normal) |
| Manifest content excerpt unreadable | Artifact file path wrong | The manifest is rendered to a file by Stage 13 — locate via `artifacts/` and the `artifact_hash` |
| Strategy doc cross-references stale | Strategy docs have evolved; specific anchors don't exist | Use the file-level reference + a section name; don't anchor to specific anchors that may break. Optionally verify each reference resolves |
| Document reads as "checklist" not narrative | Sections too templated; no prose | Add 1-2 sentences per section that EXPLAIN the transition. The reader should feel a sense of motion, not just see tables |
| Aggregates / Carpet processes look more interesting | Tempted to write golden journeys for them too | Out of scope for this handoff — DGU is the chosen target. Other processes can get their own document later |
| Found a real bug in stage logic during inspection | Stage logic wrong; data is correct given inputs but inputs themselves are stale | File as follow-up bead. Document the finding here as "noted" but DO NOT FIX in this handoff |

---

## §8 — CONSTRAINTS

1. **No code changes** — this handoff is documentation only.
2. **State.db read-only** unless optional Appendix A re-runs a live demo (in which case state.db is incrementally modified, not destroyed).
3. **Audience-appropriate language** — define domain terms; avoid unexplained acronyms.
4. **DGU only** — other processes are out of scope (they can get their own golden journey docs later).
5. **Strategy doc cross-references** — link to `docs/strategy/layers/*.md` not the research doc.
6. **No silent test exclusion** — quality gates per R.12.

---

## §9 — DELIVERABLE

### MANDATORY: Golden journey document

Path: `docs/pipeline/golden-journey-DGU.md`. Outline:

```markdown
# Golden Journey — DGU (Double-Glazed Unit, BS EN 1279)

> Generated: <ISO timestamp>
> Process: proc-pc-7046e8ec-2734-ed11-8119-005056b64545
> Scope: trace one real procurement opportunity through all 14 pipeline stages

## Executive Summary

<1 page max. What is DGU? What did the pipeline do? What's the operator's call? What does this prove about the L1-L3 product?>

## Setup
<state.db snapshot date, daemon HEAD, env constraints (e.g. no CDP browser)>

## Stage-by-stage

### Stage 1: Ingest
<D3 template>

### Stage 2: Normalize
<D3 template>

... (Stages 3-14)

## Findings

| # | Finding | Severity | Disposition |

## Cross-reference index

| Stage | Strategy doc | Section |

## Appendix A (optional): Operator's view today
<screenshot from desktop UI if Phase 2 live validation done>
```

### Quality gates (minimal)

```
cargo fmt --check                        # no code touched, sanity check
cargo check --all-targets                # sanity check
cargo test -p daemon --bin daemon        # 169 / 169 still pass
# Per-file integration loop: not strictly required since no code touched
```

---

## §10 — REMINDERS

- ULTRATHINK and be meticulous.
- The audience is a technical stakeholder unfamiliar with this code. Read each section asking "would they understand this?"
- The deliverable is a STORY, not a checklist. Tables are evidence; prose is the connective tissue.
- DGU is the target — resist the urge to compare against other processes mid-narrative (Aggregates, carpets). Mention them once if relevant, link out.
- Stages where DGU has nothing to show (e.g., Stage 3 produced no relations) are STILL worth a section — the absence IS the data. Explain why.
- Findings ≠ bugs. Findings are "I noticed this looks rough or could confuse a reader." File the real bugs as beads.
- This handoff prepares for stakeholder demos. Quality of prose matters more than completeness of database queries.
