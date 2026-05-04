# Full Pipeline E2E Caveat Fixes Handoff

ULTRATHINK and be meticulous.

## Mission

Fix the first two caveats from the full L1-L3 end-to-end validation run:

1. Stage 8 semantic extraction times out or stalls on the large Woodhouses mechanical specification.
2. Stage 9 eBay marketplace PTY skill writes a valid result contract, but the agent process can hang before exit and requires manual termination before the daemon persists the result.

These are real product defects surfaced by continuous live-data flow. They are not Layer 4, Layer 5, Layer 6, or Stage 15 work. Keep the fixes scoped to the shipped L1-L3 pipeline and add regression coverage.

## Current Context

Validation manifest:

- `docs/pipeline/full-pipeline-e2e-validation-manifest.md`

E2E runtime roots:

- State root: `state-e2e-20260504T113157Z`
- Runtime dir: `runtime-e2e-20260504T113157Z`
- Artifact root: `artifacts-e2e-20260504T113157Z`

Spotlight process:

- Process ID: `proc-pc-956d1aaa-8841-f111-813c-005056b64545`
- Notice title: `4) Woodhouses Voluntary Controlled Primary School - Boilers and BMS Panel Replacement (Four of five school summer shutdown projects)`
- Buyer: Oldham Council
- Source: ProContract

Follow-up beads already filed:

- `testproject1-qnl3`: Stage 8 semantic extraction times out on large mechanical specifications.
- `testproject1-hf42`: Marketplace PTY skill can write result contract but hang before exit.

## Non-Negotiable Constraints

- Do not modify live `state/state.db` while reproducing. Use the E2E roots above or create a new isolated state root.
- Do not delete files. This repo has a strict no-delete rule.
- Do not run destructive git commands.
- Work on `main`.
- If code changes are substantive, run the full project gates listed below before declaring green.
- If a fix would exceed the reasonable L1-L3 bug-fix window, narrow it into a bead with exact reproduction. Do not hide the gap.

## Preflight

Run:

```bash
git log --oneline -1
git status --short --branch
rg -n "semantic_skill_parameters_include_prepared_extraction_artifacts|contract_child_output_timeout_reaps_child|run_marketplace_search_live_loop|run_gross_margin_gate_stage" crates/daemon/src/main.rs crates/daemon/tests
sqlite3 state-e2e-20260504T113157Z/state.db "PRAGMA integrity_check"
```

Confirm these files exist:

```bash
ls -la artifacts-e2e-20260504T113157Z/downloads/procontract/785205/
ls -la /tmp/e2e_woodhouses_ebay_job.json /tmp/e2e_stage9_ebay_woodhouses_manual.log 2>/dev/null
```

If `/tmp/e2e_woodhouses_ebay_job.json` is missing, reconstruct the eBay job from the E2E DB by querying the Woodhouses product candidates and marketplace search source definition. Do not modify the live DB to do this.

## Caveat 1: Stage 8 Large Mechanical Spec

### What Happened

Structural extraction worked for the large mechanical specification:

- Document extraction ID: `02b4ad86-2b7a-4262-8488-26d703cd937b`
- PDF: `artifacts-e2e-20260504T113157Z/downloads/procontract/785205/App E - Woodhouses_28 286_Mechanical_Services_Specification_Revision_A_Final.pdf`
- Parsed size: 41 pages, 224 sections, 21 tables.

Semantic extraction for that large spec did not complete cleanly during the E2E run. In contrast, the short boiler schedule semantic extraction did succeed:

- PDF: `artifacts-e2e-20260504T113157Z/downloads/procontract/785205/Woodhouses_Boiler_Schedule_02_04_2026.pdf`
- Candidates produced:
  - B01 Ideal Evomax 2 80kW Wall-Hung Gas Boiler
  - B02 Ideal Evomax 2 60kW Wall-Hung Gas Boiler

This means the Stage 8 path is not fundamentally broken; it is brittle for large, mixed mechanical specifications.

### Likely Root Cause Area

Start in `crates/daemon/src/main.rs` around the Stage 8 extraction and semantic job paths:

- Job queueing and claiming for PDF extraction.
- Structural Docling artifact preparation.
- `prepare_semantic_skill_job_parameters`.
- Semantic skill invocation for `skills/pdf-extraction/tender-specification`.
- Persistence into `document_semantic_extractions` and `product_candidates`.
- Timeout and failure-state behavior for semantic jobs.

Use `rg` to find the exact symbols:

```bash
rg -n "prepare_semantic_skill_job_parameters|document_semantic_extractions|tender-specification|pdf-extraction-inputs|drain_pending_extraction_jobs|semantic extraction|document_extractions" crates/daemon/src/main.rs crates/daemon/tests
```

### Reproduction

Use the E2E state root and artifact root. Do not run against live `state/state.db`.

First inspect current state:

```bash
sqlite3 -header -column state-e2e-20260504T113157Z/state.db "
SELECT id, process_id, document_id, extraction_kind, state, output_artifact_path, error_message
FROM document_extractions
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545';

SELECT id, process_id, document_extraction_id, extraction_kind, confidence_bps
FROM document_semantic_extractions
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545';

SELECT id, name, candidate_kind, identity_confidence_bps, quoteability_score_bps
FROM product_candidates
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545';
"
```

Then queue or rerun the semantic extraction for the large mechanical spec only. If there is already a queued job, reproduce by running the job runner with the E2E flags:

```bash
target/debug/daemon \
  --state-root state-e2e-20260504T113157Z \
  --runtime-dir runtime-e2e-20260504T113157Z \
  --artifact-root artifacts-e2e-20260504T113157Z \
  --run-next-job
```

If no queued job remains, create a narrow reproduction job for the large PDF only. Prefer the repo's existing Stage 8 queueing path over one-off DB edits.

### Fix Expectations

The fix should make large-spec semantic extraction bounded, resumable, and observable. It does not need to extract every possible component from a 41-page spec in one agent turn.

Good solution shapes include one or more of:

- Chunk the structural extraction into bounded page, section, or table batches before semantic extraction.
- Add a first-pass product-bearing-section filter so boiler/equipment schedules are prioritized over boilerplate.
- Persist partial semantic outputs per chunk, then merge into `product_candidates`.
- Ensure timeout failures write a deterministic failed/skipped semantic extraction record with enough context for retry.
- Ensure stale/running semantic jobs cannot block unrelated extraction work indefinitely.

Avoid solution shapes that:

- Increase timeouts as the only fix.
- Send the entire large markdown payload into one unbounded prompt.
- Treat "0 candidates" as success without recording why the spec had no product-bearing content.

### Regression Coverage

Add focused tests in existing daemon test surfaces. Do not create new files unless an existing file is genuinely the wrong home.

Minimum coverage:

- A large prepared structural extraction is split or filtered before semantic job invocation.
- A timed-out semantic extraction leaves the job/extraction state retryable or clearly failed, not stuck.
- A successful chunked semantic output still persists candidates with the correct `process_id`, `document_extraction_id`, and candidate identity fields.

Suggested test names:

- `large_semantic_extraction_is_chunked_before_skill_invocation`
- `timed_out_semantic_extraction_is_retryable`
- `chunked_semantic_outputs_merge_into_product_candidates`

Use the repo's existing test naming and helper conventions rather than inventing a new style.

### Acceptance Criteria

This caveat is fixed when:

- Re-driving Stage 8 for the Woodhouses large mechanical spec no longer hangs or sits indefinitely pending.
- The large spec either:
  - produces useful additional candidates, or
  - records a deterministic failure/skip with clear reason and retry behavior.
- The short boiler schedule path still produces the B01/B02 candidates.
- A regression test fails before the fix and passes after the fix.

## Caveat 2: eBay Marketplace PTY Hang After Valid Result

### What Happened

The eBay UK marketplace skill successfully found and wrote a valid supplier quote result for Woodhouses, but the PTY child did not exit on its own for more than 9 minutes. The previous agent manually sent `TERM` to the specific child process; only then did the daemon persist the result.

Persisted quote:

- Quote ID: `sq-5f0522a9d4dd45b9ba7b44348af86413-0`
- Candidate: `pc-956d1aaa-8841-f111-813c-005056b64545-lot-implicit-candidate-boiler-b02-ideal-evomax2-60kw`
- Product: `Ideal Evomax 2 Commercial Condensing Wall Hung Boiler LPG Version 60kw`
- Source: `ebay-uk`
- URL: `https://www.ebay.co.uk/itm/227226093138`
- Source listing ID: `227226093138`
- Price: GBP 306,670 pence, or GBP 3,066.70
- Ship-from country: `GB`
- Price signal role: `uk_retail_benchmark`

Evidence:

```bash
rg -n "Persisted 7 search attempts and 1 supplier quotes|result.json|ebay" /tmp/e2e_stage9_ebay_woodhouses_manual.log
sqlite3 -header -column state-e2e-20260504T113157Z/state.db "
SELECT id, source_name, source_url, source_listing_id, product_name,
       price_signal_role, unit_price_gbp_pence, currency, ship_from_country
FROM supplier_quotes
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545';
"
```

Amazon UK, by comparison, persisted 4 attempts and 0 quotes because the live site rendered non-GBP prices:

```bash
rg -n "Persisted 4 search attempts and 0 supplier quotes|locale_or_currency_drift" /tmp/e2e_stage9_amazon_woodhouses_manual.log
```

### Likely Root Cause Area

This is probably not an eBay scraping logic bug. The skill wrote the result contract. The daemon/PTY adapter should treat a valid result contract as terminal enough to finalize or terminate the child cleanly.

Start around:

- PTY skill execution and supervision.
- Marketplace live loop invocation.
- Result contract polling/loading.
- Timeout behavior for PTY skills versus contract-mode skills.
- Process group cleanup.

Use:

```bash
rg -n "PTY|pty|result.json|marketplace|run_marketplace_search_live_loop|skill timeout|timeout|child|process group|agent_session_logs|persist_marketplace" crates/daemon/src/main.rs crates/daemon/src crates/daemon/tests
```

### Reproduction

If the original job file still exists:

```bash
cat /tmp/e2e_woodhouses_ebay_job.json
```

Then re-run eBay against an isolated state root. Use a copied E2E state root if you need a destructive/retry reproduction. Do not mutate live `state/state.db`.

At minimum, build a controlled regression that simulates:

1. A PTY skill child writes a valid `result.json`.
2. The child keeps running or stops producing useful output.
3. The daemon finalizes the run, persists the result, and terminates/reaps the child without manual intervention.

### Fix Expectations

The daemon should not require manual process intervention when a marketplace skill has already produced a valid result contract.

Good solution shapes include:

- Poll for valid `result.json` during PTY execution and finalize once the contract validates.
- After valid result detection, send graceful termination to the PTY child, then escalate after a short grace period.
- Bound PTY marketplace skill runtime with a source-specific timeout that still persists any valid result contract already written.
- Record the exact terminal state in `agent_session_logs` so operators can distinguish `persisted_after_result_contract` from true timeout/failure.

Avoid solution shapes that:

- Persist partial/incomplete result files.
- Kill the child before validating the result contract.
- Treat every timeout as success.
- Special-case only eBay if the defect is in the shared PTY adapter.

### Regression Coverage

Minimum coverage:

- PTY child writes valid result and hangs; daemon persists result and exits the session cleanly.
- PTY child hangs without a valid result; daemon records a real timeout/failure and does not persist fake quotes.
- Existing contract-mode timeout test still passes.

Suggested test names:

- `pty_skill_valid_result_contract_finalizes_hung_child`
- `pty_skill_timeout_without_result_records_failure`
- `marketplace_result_contract_survives_child_timeout`

### Acceptance Criteria

This caveat is fixed when:

- Re-running the eBay Woodhouses marketplace job requires no manual `TERM`.
- The daemon persists the same class of `search_attempts` and `supplier_quotes`.
- `agent_session_logs` tells the truth about whether completion came from normal child exit, valid result contract finalization, or timeout.
- Regression tests cover both valid-result hang and no-result hang.

## Related Gross Margin Context

Do not conflate this eBay hang with the later commercial caveat about Alibaba/China-side pricing.

The eBay quote was enough for Stage 10 to compute a gross margin estimate, because Stage 10 compares selected supplier cost against allocated tender/lot revenue. However, because eBay is tagged `price_signal_role='uk_retail_benchmark'`, Stage 10 applies a default 30 percent retail-to-wholesale discount for margin math and marks quote usability as failed unless the quote carries enough usability confidence.

For the Woodhouses E2E state:

```bash
sqlite3 -header -column state-e2e-20260504T113157Z/state.db "
SELECT cpa.product_candidate_id, cpa.selected_quote_id,
       cpa.expected_addressable_revenue_ex_vat_pence, cpa.gross_margin_bps,
       cpa.gross_estimate_confidence_bps, cpa.identity_gate_passed,
       cpa.usability_gate_passed, cpa.pricing_input_confidence_bps,
       cpa.revenue_basis_type
FROM candidate_pricing_assessments cpa
WHERE cpa.product_candidate_id IN (
  SELECT id FROM product_candidates
  WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545'
);
"
```

Expected current result:

- Selected quote: the eBay quote above.
- Revenue basis: `expected_value`.
- Expected addressable revenue: 6,500,000 pence.
- Gross margin: 9,669 bps.
- Identity gate: passed.
- Usability gate: failed.
- Pricing confidence: limited.

The eBay fix should make result persistence reliable. It should not pretend eBay is an import-side supplier quote.

## Quality Gates

For code changes, run:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

Also run any focused tests you add before the full loop.

For docs-only bead updates, run the relevant `br sync --flush-only` flow according to `AGENTS.md`.

## Deliverables

- Code fix for Stage 8 large-spec semantic extraction brittleness.
- Code fix for Stage 9 PTY valid-result hang.
- Regression tests for both defects.
- Updated `docs/pipeline/full-pipeline-e2e-validation-manifest.md` or a follow-up manifest section summarizing the re-drive.
- Bead updates for `testproject1-qnl3` and `testproject1-hf42`.
- Commit and push to `origin/main`.

## Final Validation Queries

```bash
sqlite3 -header -column state-e2e-20260504T113157Z/state.db "
SELECT id, state, output_artifact_path, error_message
FROM document_extractions
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545';

SELECT id, document_extraction_id, extraction_kind, confidence_bps
FROM document_semantic_extractions
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545';

SELECT id, name, candidate_kind, identity_confidence_bps, quoteability_score_bps
FROM product_candidates
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545';

SELECT source, search_failure_mode, COUNT(*) AS attempts,
       SUM(result_count) AS results, SUM(shortlisted_count) AS shortlisted
FROM search_attempts
WHERE product_candidate_id IN (
  SELECT id FROM product_candidates
  WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545'
)
GROUP BY source, search_failure_mode;

SELECT id, source_name, source_url, unit_price_gbp_pence, currency, price_signal_role
FROM supplier_quotes
WHERE process_id='proc-pc-956d1aaa-8841-f111-813c-005056b64545';
"
```

## Stop Conditions

Stop and file a bead instead of broadening scope if:

- The semantic extraction issue requires a new extraction skill architecture rather than daemon-side chunking/retry/observability.
- The PTY hang requires replacement of the entire PTY adapter.
- The fix would require Layer 4 validator skills, Layer 5 commercial policy templates, Layer 6 request automation, or Stage 15.
