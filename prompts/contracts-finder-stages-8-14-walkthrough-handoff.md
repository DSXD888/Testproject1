# Contracts Finder — Walk One Process Through Stages 8–14 Manually

ULTRATHINK and be meticulous.

## MISSION

The multi-source ingestion handoff (`f769b23`) successfully ingested 100 Contracts Finder OCDS notices and ran them through Stages 1–7. But none of those CF processes have product candidates today (Stage 8 is gated on agent-driven document acquisition + PDF semantic extraction, which currently auto-runs only for ProContract via `procontract-enrich-tender`). Bead `MS-ENRICH-1` tracks the auto-trigger work as a feature build.

This handoff is the **manual proof** that Stages 8–14 work on non-ProContract data. Pick one CF notice with usable `tender.documents[]`, manually invoke the existing PDF extraction skills + downstream pipeline, walk one CF process from "Stage 7 complete" all the way to a Stage 14 operator decision. Validates the L1-L3 product is genuinely source-agnostic without committing to a CF-specific enrichment skill.

You will **drive → diagnose any source-shape gap → fix narrowly OR document → repeat** until R.1–R.13 are GREEN.

```
┌────────────────────────────────────────────────────────────┐
│                          THE LOOP                            │
│   1. Pre-flight (§0): pick a CF process                     │
│   2. Manually trigger doc acquisition + PDF extraction      │
│   3. Validate Stage 8 emits product_candidates              │
│   4. Drive Stages 9-13 deterministically                    │
│   5. Capture Stage 14 decision via CLI or UI                │
│   6. For each stage that breaks: diagnose, fix or document  │
│   7. All 14 stages walked → validation manifest             │
└────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## §0 — PRE-FLIGHT

| Claim | Verify |
|---|---|
| HEAD = `4ba771d`. 100 CF notices ingested + Stages 1–7 succeeded for them | `sqlite3 state/state.db "SELECT source_id, COUNT(*) FROM contract_notices GROUP BY source_id"` (expect contracts-finder=100) |
| Existing PDF extraction skills are in `skills/pdf-extraction/` (semantic, structural, cascade) | `ls skills/pdf-extraction/` |
| ProContract reference: `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` shows the existing pattern (download docs from URLs → invoke pdf-extraction → emit product_candidates) | `head -30 skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` |
| CF notices' `tender.documents[]` URLs point to Contracts Finder landing pages (NOT direct PDFs) — the agent needs to navigate the page to find actual document URLs. Sample: `https://www.contractsfinder.service.gov.uk/Notice/<uuid>` | `sqlite3 state/state.db "SELECT json_extract(normalized_payload, '\$.tender.documents[0].url') FROM contract_notices WHERE source_id='contracts-finder' LIMIT 5"` |
| Stage 8 in code: where is the trigger from? Reading `crates/daemon/src/main.rs` to identify how product_candidates get inserted (search for INSERT INTO product_candidates) | `grep -nE "INSERT INTO product_candidates\|fn.*product_candidates\|fn extract_products" crates/daemon/src/main.rs \| head -10` |
| `agent-browser` + Codex CLI available (likely needed for the doc download step) | `which agent-browser; which codex` |
| `agent-mail` healthy; commits need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Parallel-safety**: this handoff drives daemon ingestion + may add to state.db (new product_candidates, lce row, manifest). Conflicts with: CDP live demos (both touch landed_cost path), Stage 14 Phase 2 live validation (touches Stage 14 state). Safe to run alongside docs hygiene + flake cleanup + post-merge gate sweep.

---

## §1 — DECISIONS ALREADY MADE

### D1 — Pick a CF process with rich documents

Not every CF notice has documents. Of the 100, find one with:

- `tender.documents[]` length ≥ 1
- Reasonable contract value (not £0, not £1B — aim £100k–£10M for realistic L3 demo)
- Category in the L3 whitelist (per strategy docs / product_candidate emit logic — typically goods/services tradeable on Alibaba/Amazon-UK/eBay-UK; avoid pure construction / professional services where price discovery is hard)
- Tender stage (not award) so the documents are usable as tender specs

Query suggestions:

```sql
SELECT cn.id, cn.title, cn.estimated_value_gbp_ex_vat_pence,
       json_extract(cn.normalized_payload, '$.tender.title') AS tender_title,
       json_array_length(json_extract(cn.normalized_payload, '$.tender.documents')) AS doc_count
FROM contract_notices cn
WHERE cn.source_id='contracts-finder'
  AND cn.procurement_stage='tender'
  AND json_array_length(json_extract(cn.normalized_payload, '$.tender.documents')) > 0
  AND cn.estimated_value_gbp_ex_vat_pence BETWEEN 10000000 AND 1000000000   -- £100k to £10M
ORDER BY cn.estimated_value_gbp_ex_vat_pence DESC
LIMIT 10;
```

Pick one. Document the choice + reasoning in the validation manifest.

### D2 — Manual doc acquisition: 2 sub-paths

The CF "documents" URLs typically point at the landing page of the notice, not the actual PDF. Two paths:

- **Path A (preferred): use the existing `procontract-enrich-tender` skill as a template.** Invoke it manually with adjusted parameters pointing at the CF URL. The skill's browser-driving pattern may work; if not, document where it diverges (CF UI is different from ProContract UI).

- **Path B (fallback): manual download.** Use a browser (or curl + manual inspection) to find the actual PDF URLs on the CF landing page, download to `artifacts/downloads/contracts-finder/<process_id>/`, then invoke `pdf-extraction` skills directly on the local files.

Document which path you used and why.

### D3 — Use existing pdf-extraction skills directly

Once PDFs are local, invoke the existing extraction skills (`pdf-extraction/structural`, `pdf-extraction/semantic`, `pdf-extraction/cascade`) via `--skill-path` flag. Don't write a new CF-specific skill — that's MS-ENRICH-1 territory, out of scope here.

```bash
cargo run -p daemon -- \
  --skill-path skills/pdf-extraction/cascade \
  --job-parameters-json '{"document_paths": ["artifacts/downloads/contracts-finder/<pid>/spec.pdf"], "process_id": "<pid>"}'
```

Adjust the parameters JSON per the skill's `result.schema.json` and `SKILL.md` requirements.

### D4 — Drive Stages 9-13 with --ingest

Once product_candidates exist for the CF process, the deterministic stages should pick them up via the next `--ingest` runs:

- `--ingest amazon-uk` / `--ingest mx-wholesale` / etc. for Stage 9 (search/quotes) — pick whichever marketplace the candidate type best fits
- `--ingest landed-cost` for Stage 11 (after Stage 10 gross-margin gate runs)
- `--ingest net-gate` for Stage 12
- `--ingest manifest` for Stage 13

Stage 10 (gross margin gate) runs as part of `--ingest landed-cost`; doesn't have its own flag.

### D5 — Stage 14 via CLI

Use `--capture-decision` + `--capture-clarification` on the CLI. Phase 2 UI demo would also work but is exercised separately.

### D6 — Document gaps; don't build features

If CF data shape breaks a stage that ProContract doesn't break, that's a finding to document — NOT a feature build. The fix path is:

1. Trivial fix (e.g., tolerate missing optional field): apply + add regression test
2. Larger gap (e.g., CF documents have different MIME types, requiring new PDF parser): file as bead

The line is "1 hour or less" — anything bigger goes to a bead.

---

## §2 — EXECUTION ORDER

1. **Pre-flight + pick target** (§0 + D1).
2. **Doc acquisition** (D2 path A or B). Capture evidence (downloaded files in `artifacts/downloads/contracts-finder/<pid>/`).
3. **PDF extraction**. Invoke `pdf-extraction/cascade` (or sub-skills) per D3. Validate output (product candidates inserted into DB).
4. **Stage 8 verification**: `sqlite3 state/state.db "SELECT id, name FROM product_candidates WHERE process_id='<pid>'"` returns ≥1 row.
5. **Stage 9 search/quotes**: pick a marketplace per candidate kind. Drive `--ingest <marketplace>` if applicable, OR document why no quotes can be found.
6. **Stage 10-12**: drive `--ingest landed-cost && --ingest net-gate`. Validate lce row + net_margin_assessment row.
7. **Stage 13 manifest**: `--ingest manifest`. Validate manifest row exists for the CF process.
8. **Stage 14 decision**: `cargo run -p daemon -- --capture-decision <pid> --disposition watch --idempotency-key cf-walkthrough-001 --reasoning "Manual CF walkthrough validation"`. Verify intent row.
9. **Quality gates** + verify R.* invariants (other CF processes + ProContract universe unchanged).
10. **Validation manifest** + commit.

---

## §3 — REQUIRED TESTS

This handoff adds tests ONLY if a code fix was needed (per D6):

- A trivial fix → regression test alongside it. Naming: `cf_<gap_description>_regression`.
- No code fix → no new tests.

If Path A (re-using procontract-enrich-tender skill against a CF URL) fully worked, that's a positive finding — no new tests needed; the cross-source generalization is documented.

---

## §4 — ACCEPTANCE CRITERIA — R.1 to R.13

| # | Criterion |
|---|---|
| R.1 | One CF process selected per D1 criteria; choice + reasoning in manifest |
| R.2 | Doc acquisition path (A or B) documented with command excerpts + downloaded file count |
| R.3 | PDF extraction succeeded: at least one PDF processed by `pdf-extraction/cascade` skill, evidence in agent_session_logs |
| R.4 | Stage 8: ≥1 row in `product_candidates` for the chosen CF process |
| R.5 | Stage 9: either ≥1 supplier quote found, OR documented "no quotes — explain why" (e.g., no marketplace match) |
| R.6 | Stage 10-11: if Stage 9 produced quotes, lce row exists with reasonable freight/customs/economic_landed_cost OR documented blocker |
| R.7 | Stage 12: net-margin assessment row exists for the candidate |
| R.8 | Stage 13: manifest row exists with stoplight + suggested_action |
| R.9 | Stage 14: pending_operator_intents row exists with the test idempotency_key, status='queued' |
| R.10 | Other 99 CF processes UNCHANGED (no rogue product_candidates inserted for the wrong processes) |
| R.11 | ProContract universe UNCHANGED: 7 manifests, 4 lce rows, 17 net-margin verdicts, demo intents intact, DGU operator override intact |
| R.12 | Any code fixes applied are minimal (1 hour or less) + accompanied by regression tests; larger gaps filed as beads |
| R.13 | Quality gates: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings`; daemon binary tests still pass; per-file integration loop unchanged |

---

## §5 — VALIDATION QUERIES

```bash
TARGET_PID="proc-ocds-b5fd17-XXXXXXXX"   # set after D1

# After Stage 8 (PDF extraction → product_candidates)
sqlite3 -header state/state.db "
SELECT '=== Stage 8 product_candidates for target CF process ===';
SELECT id, name, candidate_kind, identity_confidence_bps, quoteability_score_bps
FROM product_candidates WHERE process_id='$TARGET_PID';

SELECT '=== Stage 8 unchanged for OTHER CF processes ===';
SELECT process_id, COUNT(*) AS candidate_count
FROM product_candidates WHERE process_id LIKE 'proc-ocds-b5fd17-%' AND process_id != '$TARGET_PID'
GROUP BY process_id;
"

# After Stage 9 (search/quotes)
sqlite3 -header state/state.db "
SELECT id, source_label, listing_url, unit_price_gbp_pence, currency
FROM supplier_quotes WHERE product_candidate_id IN
  (SELECT id FROM product_candidates WHERE process_id='$TARGET_PID');
"

# After Stage 11 (landed cost)
sqlite3 -header state/state.db "
SELECT lce.decision_grade, lce.economic_landed_cost_gbp_pence, lce.commercial_estimate_confidence_bps,
       cpa.gross_margin_bps
FROM landed_cost_estimates lce
JOIN candidate_pricing_assessments cpa ON cpa.product_candidate_id=lce.product_candidate_id AND cpa.selected_quote_id=lce.supplier_quote_id
WHERE lce.product_candidate_id IN (SELECT id FROM product_candidates WHERE process_id='$TARGET_PID');
"

# After Stage 12 (net margin)
sqlite3 state/state.db "
SELECT verdict, reasons_json FROM candidate_net_margin_assessments
WHERE product_candidate_id IN (SELECT id FROM product_candidates WHERE process_id='$TARGET_PID');
"

# After Stage 13 (manifest)
sqlite3 state/state.db "SELECT process_id, stoplight, suggested_action, manifest_basis_hash FROM manifests WHERE process_id='$TARGET_PID';"

# After Stage 14 (decision)
sqlite3 state/state.db "
SELECT idempotency_key, status, desired_disposition, decision_note FROM pending_operator_intents
WHERE process_id='$TARGET_PID' ORDER BY queued_at DESC LIMIT 5;
"

# Final invariants (R.10/R.11)
sqlite3 -header state/state.db "
SELECT 'CF processes unchanged: ' || COUNT(*) FROM contract_notices WHERE source_id='contracts-finder';
SELECT 'ProContract intact: ' || COUNT(*) FROM contract_notices WHERE source_id='procontract';
SELECT 'demo intents:'; SELECT idempotency_key, status FROM pending_operator_intents WHERE idempotency_key LIKE 'dec-2026-05-01-dgu-%';
SELECT 'DGU override:'; SELECT json_extract(payload_json, '\$.qty') FROM operator_input_overrides WHERE product_candidate_id LIKE '%glass-dgu%' AND withdrawn_at IS NULL;
"
```

---

## §6 — FAILURE-MODE PLAYBOOK

| Symptom | Cause | Action |
|---|---|---|
| Path A: procontract-enrich-tender skill fails on CF URL because portal navigation differs | CF UI has different DOM structure than ProContract | Fall through to Path B (manual download via browser/curl). Document the divergence; this is the actual content of MS-ENRICH-1 |
| Path B: PDFs at the CF landing page are behind a "click to download" interaction | Manual operator action: click + download in a browser | Document the manual step. The validation still proves Stages 8-14 work; the gap is just at Stage 4 (acquire docs) |
| pdf-extraction/cascade rejects the PDF | Document type/encoding not supported by existing skills | Try the underlying sub-skills (semantic, structural). If all fail, the document is genuinely intractable — document and pick a different CF process |
| Stage 8 emits product_candidates but `quoteability_score_bps=0` for all | Quoteability scoring rejects them (out-of-whitelist, unclear specs) | This is normal for many CF tenders. Document. Maybe the chosen process is not a great demo candidate; consider re-picking per D1 |
| Stage 9 finds 0 quotes | Product candidate doesn't match any marketplace catalog | Try a different `--ingest <marketplace>` flag. If none find quotes, the candidate is genuinely not quoteable — document; you can still proceed to Stage 11 with a "screening-only" path |
| Stage 11 landed cost crashes on non-GBP supplier currency | Pre-existing FX caveat (deferred multiple times in earlier handoffs) | Document. Don't fix here — it's a known limitation. Pick a different CF candidate that has GBP quotes |
| Stage 12/13 succeed but the manifest reads as nonsense | Stage 11 output was poor; the manifest's automated narrative reflects that | Document as a finding. Manifest content quality on CF data is downstream of every prior stage's quality |
| Stage 14 decision capture errors with "no manifests row" | Stage 13 didn't run for this process | Re-run `--ingest manifest`. If still missing, debug Stage 13's gating |
| ProContract or other CF processes regressed | A `--ingest <stage>` invocation processed processes you didn't intend | The deterministic stages run on ALL eligible processes, not just the target. That's expected. Verify via R.11 query that the target's state changed but other processes' state didn't |

---

## §7 — CONSTRAINTS

1. **No new skills.** Use the existing `procurement-ingestion/procontract-enrich-tender` (re-purposed) and `pdf-extraction/*`. Building a `contracts-finder-enrich-tender` skill is MS-ENRICH-1 territory.
2. **No new daemon stage logic.** If Stage 8/9/10/11/12/13 has a CF-specific bug, fix only if 1-hour-or-less + obvious; otherwise file as bead.
3. **No new migrations.**
4. **One target CF process** — don't try to walk all 100. The point is to PROVE source-agnostic pipeline, not to enrich the entire CF dataset.
5. **State.db modifications**: this handoff adds product_candidates, supplier_quotes, lce, net_margin, manifest, intent rows for the target process. That's expected. Other processes must be unchanged (R.10/R.11).
6. **No silent test exclusion** — quality gates per R.13 mandatory.

---

## §8 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/contracts-finder-stages-8-14-walkthrough-validation-manifest.md`. Include:

- Target CF process: ID, title, value, why chosen
- Doc acquisition: path A or B + commands + file count
- Stage-by-stage outcomes (table: stage / what ran / DB state / observations)
- Findings: where did CF data shape diverge from ProContract? Was the divergence a bug, a gap, or just a difference?
- Code fixes applied (if any) + regression tests added
- R.1–R.13 status table
- Out-of-scope items deferred (probable: CF auto-enrich skill = MS-ENRICH-1)

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon                    # 169 / 169 (or +N if regressions added)
cargo test -p procintel-desktop --test query_service # 21 / 21
cd apps/desktop && bunx vitest run                    # 35 / 35
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || break; done   # baseline (38 / 39 + pre-existing)
```

---

## §9 — REMINDERS

- ULTRATHINK and be meticulous.
- The deliverable is PROOF that L1-L3 stages 8-14 work source-agnostically. One CF process is enough; don't over-build.
- Path A first, Path B fallback. Document which one worked.
- Aim for a CF process with reasonable value + tradeable goods + tender-stage docs (D1 query).
- If the CF auto-enrich gap (MS-ENRICH-1) feels like the right next step, that's a separate feature build — leave it as a bead.
- ProContract universe + DGU demo state must remain pristine. Validate R.10 + R.11 carefully.
- Be honest in the findings: if CF data has a meaningfully different shape that needs new code to handle gracefully, say so.
