# L6 Request Pipeline Automation — Step 09: Bead Audit (Spec Drift Detection)

## First Action

Invoke the `/bead-plan-audit` skill.

---

## Your Mission

This is the **LAST GATE before execution**. You are performing a dedicated, systematic
audit of every L6 bead against the 3 SOTs (Spec + TDD + Strategy). The verdict is
binary:

- **PASS** — proceed to step 10 (generate kickoff)
- **FAIL** — fix beads, re-run audit, do NOT proceed

The single most important audit dimension is **ZERO SCOPE DRIFT**: every spec FR has
at least one implementing bead, every TDD test ID has exactly one owning bead, no
scope-reduction language anywhere.

If a previous step (01-08) missed something, this is the last chance to catch it
before agents start executing.

---

## Source Documents

### Beads (TARGET — audit EVERY bead)
```bash
br list --label l6 --json                              # Full inventory
br show <id> --json                                    # Individual bead detail (walk EVERY one)
br ready --label l6 --json                             # Should reflect the dependency DAG
br dep tree <epic-id>                                  # Verify hierarchy
```

### Companion artifacts
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bulk-import.md
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-bead-alias-map.json
```

### Verify Against (THE THREE SOTs — every fact in beads must trace to here)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md                                      # SOT #1
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md     # SOT #2
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md # Environment + quality gate
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md # SOT #3
docs/pipeline/l6-request-pipeline-automation/manifest.yaml                                       # decisions + existing_files + new_modules
```

### Codebase (GROUND TRUTH — verify every bead file-path reference)
```
crates/daemon/src/errors/manifest.rs:19-114
crates/daemon/src/main.rs:1513-1517      # **Job-kind dispatch — Phase 2 added run_bid_workflow arm HERE, NOT in jobs/runner.rs**; existing extract_document_* arms are reference patterns
crates/daemon/src/jobs/runner.rs         # Shared types only (1656 lines); verify Phase 2 did NOT misroute the dispatch here
crates/daemon/src/ipc/handlers/jobs.rs
crates/daemon/src/validation/{policy,executor,manifest_writer}.rs
crates/daemon/src/main.rs:5256-5260,5510-5527,5537-5542,5781-5787
crates/daemon/src/agents/{adapter,artifacts}.rs
crates/daemon/src/pty/{session,capture}.rs
crates/daemon/src/sources/registry.rs
crates/daemon/src/db/sync.rs:24-50
skills/auth-signin/procontract/SKILL.md
skills/procurement-ingestion/procontract-register-interest/SKILL.md
skills/validation/overlay-supplier-enrichment/SKILL.md
packages/contracts-schema/src/index.ts
packages/intelligence/                                  # L5 surface — L6 reads, doesn't modify
packages/ui/                                            # extension target
apps/desktop/                                           # extension target
apps/web/                                               # extension target
migrations/0001_sqlite_state_initial.sql:437-451       # bid_workflow_states schema
```

### Quality Bar (READ THE L4 AND L5 COMPLETION AUDIT VERDICTS)
```
docs/pipeline/l4-shared-visibility/artifacts/l4-completion-audit-2026-05-08.md         # 172 lines, COMPLETE verdict
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-2026-05-20.md  # 270 lines, COMPLETE verdict
docs/pipeline/l5-expanded-commercial-intelligence/archived/2026-05-21-l5-expanded-commercial-intelligence/09-audit-beads.md  # 332 lines — L5 step 09 reference
```

---

## Cardinal Rule: ZERO SCOPE DRIFT — BLOCKING DIMENSION

This audit's dedicated, blocking dimension is **scope drift**. The other dimensions
(coverage matrix, dependency DAG, bead self-containment) are also blocking, but
scope drift is the one that masquerades as "execution flexibility" and slips past
casual review.

**Definition of scope drift for L6:**
- Any spec FR with zero corresponding beads → FAIL
- Any TDD test ID with zero owning beads → FAIL
- Any DoD criterion not satisfied by some bead → FAIL
- Any invariant without an enforcing bead → FAIL
- Any phase labeled "stretch" / "optional" / "post-MVP" → FAIL
- Any bead body containing "post-MVP" / "deferred" / "nice to have" → FAIL
- Validator overlay bead missing (decision_3 made it L6 deliverable) → FAIL
- Live demo bead missing (DoD #8 + decision_5) → FAIL
- Live demo bead missing operator-action gates → FAIL
- Phase 7 has fewer than 4 beads (anomalously thin late phase) → FAIL

---

## Audit Methodology (Invoke `/bead-plan-audit` to drive the systematic walk)

The skill runs a 5-pass audit. Here are L6-specific instructions for each pass:

### Pass 1 — Coverage Matrix (Spec → Bead)

Build a matrix: rows = L6-FR-X.Y.Z (from spec), cols = bead IDs.
Cell value: 1 if the bead implements the FR (per its body), else 0.
Sum each row. Sum each column.

**Audit verdict:**
- Every row sums to ≥1 → PASS this pass
- Any row sums to 0 → FAIL — name the FR, file as a missing bead

Specifically check (don't trust the matrix without spot-checks):
- L6-FR-1.3.1...N (the 7 lifecycle states + **14** transitions + 8 prerequisite check conditions) → all covered. Spec §1.3 has 14 distinct From→To pairs (not 11 — re-verify yourself).
- L6-FR-1.4.1...N (BidScopePayload fields + SideEffectClass values) → all covered
- L6-FR-1.5.1...19 (the 19 bid workflow flow steps) → all covered
- L6-FR-1.7.1...11 (the 11 skill playbook steps) → all covered
- L6-FR-1.8.1...6 (the 6 audit-trail artifact types) → all covered
- L6-FR-1.10.1...8 (the 8 invariants) → all covered with explicit enforcement
- L6-FR-2.1.1...11 (the 11 DoD criteria) → all covered

### Pass 2 — Test Coverage (TDD → Bead, 1:1)

Open `l6-bead-alias-map.json`. For every L6-T-* test ID:
- Count owning beads
- Verify count == 1 (no orphans, no double-ownership)

**Audit verdict:**
- All counts == 1 → PASS
- Any count == 0 → FAIL (orphan test) — assign to a bead
- Any count >= 2 → FAIL (double ownership) — pick one owner

Specifically verify the 3 spec-named test files:
- `bid-workflow-states.test.ts` → owned by one Phase 3 or Phase 5 bead
- `portal-side-effects.test.ts` → owned by one Phase 3 or Phase 5 bead
- `bid-readiness-gate.test.ts` → owned by one Phase 3 bead

Plus the Rust integration test extensions:
- `crates/daemon/tests/skill_canary_harness.rs` extensions (e2e_can_010+) → owned by Phase 4 portal-skill bead
- `crates/daemon/tests/skill_golden_journeys.rs` extensions (l3_t_skill_golden_journeys_009+) → owned by Phase 4 portal-skill bead OR Phase 5 validator-overlay bead
- `crates/daemon/tests/redaction_boundary.rs` extensions → owned by Phase 3 readiness-gate bead OR a dedicated PII-redaction bead

### Pass 3 — Dependency DAG Audit

Run:
```bash
br dep tree <epic-id>
br ready --label l6 --json
br lint
```

**Audit verdict:**
- `br lint` reports zero issues → PASS this pass
- `br ready` returns only true phase-1 roots (no beads with unresolved dependencies marked ready) → PASS
- DAG has no cycles → PASS

Specifically verify:
- Phase 1 beads have NO dependencies
- Phase 2 beads depend on Phase 1 ErrorCode bead (clippy requires new variants)
- Phase 3 beads depend on Phase 2 dispatch + IPC handler
- Phase 4 portal-skill beads depend on Phase 1 types
- Phase 5 validator-overlay + packages/bidding beads depend on Phase 1 types + the worker_behavior_version contract (locked in Phase 4 or earlier)
- Phase 6 UI beads depend on Phase 2 IPC handler + Phase 5 validator overlay
- Phase 7 live-demo beads depend on ALL prior phases

If a Phase 6 bead is marked ready without Phase 2 being closed, the DAG is broken.

### Pass 4 — Bead Self-Containment

Walk EVERY bead. For each:
- [ ] Spec FR text quoted verbatim (not paraphrased)
- [ ] L6-T-* test IDs listed
- [ ] Files to Create/Modify section is non-empty and exact (no "and related files")
- [ ] Acceptance criteria are testable bullets
- [ ] Dependencies listed (or "(none — phase root)")
- [ ] Phase number matches strategy

**Sampling minimum:** audit at least 50% of beads in detail; spot-check the rest. If
any sampled bead fails, sample more aggressively.

### Pass 5 — Scope Drift Scan (DEDICATED, BLOCKING)

**Red-flag language scan:**
- Search every bead body + the bulk-import doc + every artifact for:
  - "post-MVP", "post-L6", "Phase 8+"
  - "stretch goal", "nice to have"
  - "out of scope", "deferred"
  - "future iteration", "later"
  - "simplified for initial implementation"
  - "if time permits", "could add"
  - "optional" (in bead descriptions, not in option-vs-required wording)
  - "skip for now", "TODO" (in bodies, not AC checkboxes)

**Zero hits expected. ANY hit → BLOCKING failure → fix the bead → re-audit.**

**Coverage verification:**
- Every spec feature/subsystem/capability identified → which bead(s) implement it
- If any spec feature has ZERO corresponding beads → BLOCKING failure
- Create missing beads, sync, commit, re-audit

**Phase coverage equality:**
- Count beads per strategy phase
- If later phases (especially Phase 7 live demo) have significantly fewer beads than early phases (Phase 4 portal skill), investigate
- Later phases MUST have same depth as early phases
- Phases are execution order, NOT importance tiers
- Phase 7 thin = scope drift signal (the M6 capstone reduced to "just one quick run")

---

## L6-Specific Audit Checks (the things that historically slip past)

### Check A: ErrorCode Propagation Bead
Phase 1 ErrorCode bead must:
- List the **TDD-determined** new variant set (typically 3-6; final list authored in step 03 TDD after auditing existing ErrorCodes in `crates/daemon/src/errors/manifest.rs:19-114` for redundancy). Likely-new: `BID_SUBMISSION_FAILED`, `BID_DUPLICATE_DETECTED`, `BID_SIDE_EFFECT_CAP_EXCEEDED`, `BID_READINESS_GATE_FAILED`. Likely-redundant (verify): `BID_PORTAL_CHANGED` (vs `PAGE_SHAPE_DRIFT`), `BID_PORTAL_AUTH_EXPIRED` (vs `AUTH_STATE_STALE`).
- For each NEW variant: list exhaustive-match propagation sites (5-15 per L5 precedent) — e.g., `errors/markdown.rs`, `validation/policy.rs`, `validation/executor.rs`, `tracing/structured.rs`, `ipc/handlers/*.rs`
- AC: `cargo clippy --all-targets -- -D warnings` passes after extension
- AC: round-trip test (`parse_error_code(variant.as_str()) == Ok(variant)`) for every NEW variant
- AC: bead's count matches what the TDD specified (no silent expansion or shrinkage)

### Check B: Naming Reconciliation Enforced (decision_1)
Walk every bead's Files section. Verify:
- New files use `procontract` naming (not `proactis`)
- `packages/bidding/procontract-workflow.ts` (not `proactis-workflow.ts`)
- `skills/bid-portal-procontract/` (not `skills/bid-portal-proactis/`)
- `skills/validation/overlay-bid-portal-procontract/` (not `overlay-bid-portal-proactis/`)
- `BidScopePayload.bidPortalType` value is `'procontract'` (not `'proactis'`)

### Check C: Auth Delegation Bead (decision_2)
Phase 4 portal-skill bead must:
- Reference `skills/auth-signin/procontract/` as the delegated auth surface
- AC: bid-portal-procontract detects session expiry (URL contains `/Login` OR text matches `Session expired|Please log in`) and returns `AUTH_STATE_STALE` (does NOT invoke auth-signin directly — daemon-orchestrated retry pattern, identical to `procurement-ingestion/procontract-register-interest/SKILL.md`)
- AC: bid skill does NOT implement portal login inline (no credential typing, no autofill triggering)
- AC: daemon's existing AUTH_STATE_STALE handler runs auth-signin/procontract as sub-job, then re-runs original bid job (this is daemon code, not bid-skill code)
- AC: identical CDP discipline (attach persistent profile, no fresh browser, no captcha, no logout)

### Check C.5: Skill Frontmatter Convention Compliance (NEW — added 2026-05-21)
Phase 4 bid-portal-procontract skill bead must:
- AC: SKILL.md frontmatter includes `source_structure_version: procontract-bid-portal-<launch-date>` (Procontract convention — verified across procontract-discover, procontract-enrich-tender, procontract-register-interest at 2026-04-13)
- AC: SKILL.md frontmatter `model_profile` value — if `portal_form_high_reliability` (per spec §1.7), there's a paired daemon bead in Phase 1 that adds the match arm in `crates/daemon/src/main.rs`'s `match skill.metadata.model_profile.as_str()` block; if fallback `browse_stable_low_cost`, no daemon change needed (record the choice in the bead body)
- AC: SKILL.md frontmatter includes `validation_policy: always` (verified: this field exists — see skills/manifest-generation/SKILL.md:6 for precedent)
- AC: SKILL.md frontmatter includes `requires_cdp: true` + `cdp_default_port: 9222` (Procontract convention — all 3 existing procontract skills)
- AC: SKILL.md frontmatter includes `worker_behavior_version: bid-portal/procontract@<skill-version>+<schema-version>` (validator overlay's `worker_behavior_version` must match)

### Check D: Validator Overlay Bead (decision_3)
Phase 5 must include a bead for `skills/validation/overlay-bid-portal-procontract/`:
- Files: `skills/validation/overlay-bid-portal-procontract/SKILL.md` (create)
- AC: frontmatter follows overlay-supplier-enrichment shape (worker_behavior_version + validator_behavior_version + validator_overlay_compatible_versions)
- AC: overlay assertions cover (a) opportunity ref match, (b) screenshot count appropriate to run shape (first run via 10a OR 10b path → ~11 screenshots; continuation run from draft_saved → variable per TDD rubric), (c) side-effect class monotonicity + never exceeds BidScopePayload.sideEffectCap, (d) submission confirmation ref well-formed (if present), (e) agent narrative cites BidScopePayload lot/candidate IDs verbatim, (f) no PII leakage in narrative (cross-check redaction_boundary tests)
- AC: fail-closed test passes (validator rejection blocks bid_submitted)
- AC: fail-closed-on-outage test passes (validator unavailable blocks bid_submitted)

### Check E.5: Operator Override Mechanism Tests (NEW — added 2026-05-21)
Phase 3 readiness-gate bead + Phase 5 validator-overlay bead must include:
- AC: quote-role override (floor-only evidence) test in `bid-readiness-gate.test.ts` covers both `no override → blocked` AND `override present → allowed (with audit-trail entry on run manifest)`
- AC: validation-gate override (final submission with validator rejection or outage) test covers both `no override → blocked at bid_submitted transition` AND `explicit override → allowed (logged in run manifest)`
- AC: 5 L5 hard blockers + operator-approval + staleness + upstream-trust have NO override path tested (these are constitutional; attempting override returns same blocked result)
- AC: every override generates an audit-trail entry that the validator overlay can read; missing override-citation in a manifest with floor-only evidence is flagged as anomaly

See `manifest.yaml#cross_layer_context.operator_override_mechanisms` for the full override-vs-non-override taxonomy.

### Check E: Live Demo Bead Gates (decision_5)
Phase 7 live demo beads must:
- Bead "first live demo": sideEffectCap=REMOTE_DRAFT, reaches draft_saved, screenshot trail present, run manifest written, Supabase visible
- Bead "second live demo (operator-authorized)": sideEffectCap allows FORMAL_SUBMISSION, reaches bid_submitted, portal confirmation ref captured, all 11 DoD criteria green
- AC for first: operator confirms CDP profile + opportunity ref + first-run constraint
- AC for second: operator EXPLICITLY authorizes FORMAL_SUBMISSION run

### Check F: Sister Epic Hierarchy (decision_4)
- `br show testproject1-rc0 --json` → status closed
- All task beads have `parent_id` == the new sister epic ID
- `br dep tree <new-epic-id>` shows the full L6 implementation tree

### Check G: PII Redaction Bead
A dedicated PII redaction bead (or AC on the readiness-gate bead) covers redaction of:
- `companyName`, `registrationNumber`, `vatNumber`
- Address fields
- `primaryContact.{name,email,phone,role}`
- `certifications[].number`
- `insurancePolicies[].policyNumber`, `coverAmountPence`
- `financialStatements.companiesHouseRef`

From: error manifests, run manifest narrative, Supabase projections, log output.

### Check H: Sync Engine No-Op Verification
A bead (or AC on the Phase 2 dispatch bead) explicitly verifies:
- `crates/daemon/src/db/sync.rs:24-50` `PUSH_OWNED_RESEED_TABLES` array already contains `bid_workflow_states` at index 14 (line 39) — no changes needed
- Bid workflow mutations auto-propagate to Supabase via existing sync engine
- Test: mutate bid_workflow_states locally → outbox → sync → Supabase row matches

---

## Audit Verdict Format

After all 5 passes complete + L6-specific checks A-H done, write the verdict in
the bead update:

```bash
# If PASS — use `br comments add` (NOT `br update --comment` — that flag does NOT exist in br;
# verified via `br update --help` 2026-05-21):
br comments add <epic-id> "L6 bead audit PASS (2026-MM-DD): all 5 passes green + L6-specific checks A-H green. Coverage matrix: NN/NN. Test ownership: NN/NN. DAG: clean. Self-containment: NN/NN sampled, NN clean. Scope drift: zero hits. Ready for step 10 (generate kickoff)."

# If FAIL:
br comments add <epic-id> "L6 bead audit FAIL (2026-MM-DD): N gaps found. See audit-fix beads created: [bead-ids]. Re-audit after fix beads close."
# Then create audit-fix beads for each gap.

# Alternative comment-add syntax (read from file for long verdicts):
# br comments add <epic-id> --file /tmp/l6-audit-verdict.md
```

---

## Follow-Up Pass (After Audit Complete)

> ULTRATHINK. Re-read YOUR audit output and any fix beads you created.

1. **Did fixing one bead break another's dependencies?** Run `br lint` again.
2. **Re-run the coverage matrix.** Is it still 100%?
3. **Verify bead count matches strategy phase task count.** Each phase has the expected ~N beads.
4. **Verify ZERO scope-drift language remains** in any bead.
5. **Verify every spec feature has at least one bead mapping** in `l6-bead-alias-map.json`.
6. **Open `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-2026-05-20.md`.** Compare your audit's rigor — L5's completion audit walked every L5-FR-X.Y.Z. Did yours match?
7. **Re-run `bin/br-sync` + commit + push** if you made any bead updates.

---

## Output

- Audit verdict comment on the L6 implementation epic
- (If FAIL) Audit-fix beads created with explicit gap descriptions
- (If PASS) Ready to proceed to step 10

---

## Key Reminders

- This is the LAST GATE before execution — do NOT skip
- Verdict is binary: PASS or FAIL
- ZERO SCOPE DRIFT is the DEDICATED, BLOCKING dimension
- L6-specific checks A-H catch the historically common slips
- The TDD-determined NEW ErrorCode variant set (typically 3-6) + naming reconciliation + auth delegation + validator overlay + live demo gates are NON-NEGOTIABLE
- Match L4/L5 completion audit rigor (walk every FR; no sampling shortcuts on coverage)
- Use `bin/br-sync` (NEVER raw `br sync`)
- If FAIL, do NOT proceed to step 10 — fix beads, re-audit
