# Goods-Led Install/Maintenance — Bead Audit (Spec Drift Detection)

## First Action

Invoke the `/bead-plan-audit` skill.

---

## Your Mission

Perform a **dedicated, systematic audit** of all `goods-led-install` beads. This is the LAST GATE before execution. Verify **zero spec drift**: every spec requirement is covered, every TDD test is assigned to a bead, no silent scope cuts, phase ordering matches the strategy, and every bead is self-contained.

> **{ENRICHMENT — after Batch 4}** If the enrichment pass has run, the actual bead inventory is pinned here: implementation epic `<epic-id>`, `<N total beads>`, `<N ready>`, per-phase distribution `<…>`. Use these as the expected coverage numbers — if the live `br list --label goods-led-install --json` count differs from the strategy's expected per-phase count, that gap is itself a scope-drift signal to investigate.

---

## Source Documents

### Beads (TARGET — audit EVERY bead, not a sample)
```bash
br list --label goods-led-install --json     # full inventory
br show <id> --json                           # each bead in detail
br dep tree <epic-id>                         # dependency structure
```

### Verify Against
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md       — SOT #1 — every requirement must have a bead
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md          — SOT #2 — every test ID must be assigned
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-strategy.md     — SOT #3 — phase ordering must match
```

### Codebase (verify bead references are accurate)
```
packages/intelligence/src/qualification/goods-qualification.ts:319-372
packages/intelligence/src/customs/description-quality.ts:23-36,100-113,360
crates/daemon/src/main.rs:5361-5432,8114,8548,9136-9290
packages/contracts-schema/src/index.ts:36,166,216
docs/strategy/layers/layer-0-gold-set-evaluation.md
```

---

## Audit Methodology

`/bead-plan-audit` handles the detailed methodology. Add these feature-specific checks.

### Feature-Specific Checks
- **Both rollup sites:** confirm a bead covers `main.rs:8114` (domestic) AND `main.rs:8548` (import). A bead naming only one is a defect.
- **Migration nullability:** confirm the schema bead specifies nullable/default-0 columns (so `decision_basis_hash.rs` / `customs_fx_freight_engine.rs` direct INSERTs don't break).
- **Schema-epoch 4-constant bump:** confirm one bead covers daemon sqlite.rs, desktop main.rs, daemon_bridge.rs, query_service.rs + Supabase `schema_metadata` + the projection migration. None missed.
- **Service gate `goods_led_mixed`:** confirm the bead ADDS to the existing `DescriptionQualityVerdict` union (not a parallel path) AND creates the NEW `description-quality.test.ts`.
- **Phase-1 vs Phase-2:** confirm the early provisional-admit bead does NOT depend on a cost-based share (circular), and the Phase-2-gate bead depends on the rollup+labour beads.
- **Goods-only subtotal:** confirm a bead persists the goods-only subtotal / line items so `goods_value_share_bps` is reconstructable from one row.
- **Skill profile:** confirm the labour-skill bead declares `http_public`, ships 4 reference JSONs, and forbids CDP membership.
- **Scenario-1 band:** confirm a test bead carries the hard assert `210000 ≤ installation_labour_cost_gbp_pence ≤ 265000`.

### Scope Drift Detection (MANDATORY — BLOCKING DIMENSION)

**Red-flag language scan** — search all **Tier-A** beads for "post-MVP," "Phase 2+," "stretch goal," "nice to have," "future iteration," "out of scope," "can be added later," "simplified for initial implementation," "deferred to." If found: BLOCKING failure → replace with full implementation content. (Tier-B `research-spike` beads legitimately carry research/design framing — that is their operator-decided treatment, not a hit.)

**Coverage verification (two tiers — operator-decided 2026-06-02; see `manifest.yaml` SCOPE TIERS)** — for EVERY spec feature/subsystem:
- Identify which bead(s) implement it.
- **Tier-A** (MVP + §2.4 "Phase 2 / Should Have": recurring maintenance, access-equipment auto-selection, rate-refresh skill, overlay, basic banded permit/TM): if ANY has ZERO full beads → BLOCKING failure → create them.
- **Tier-B** (the three §2.4 "Future" / §10 "requiring further research" items: live enrichment lane, installer-quote ingestion, detailed per-authority/per-zone permit sub-model): each needs exactly one `research-spike` bead. Missing → BLOCKING. Ballooned into a full epic → flag for downgrade (false precision). The research-spike framing on these beads is correct, NOT a red-flag hit.

**Phase coverage equality** — count beads per strategy phase. If later phases have significantly fewer beads than their spec surface justifies → investigate. Later phases get the same depth as early ones.

**Every TDD test assigned** — walk the TDD coverage matrix; confirm each test ID (TS-QUAL-*, TS-COV-*, the 8 truth-table rows, 11 Appendix-E manifests, 7 canary cases) is owned by a bead.

---

## Follow-Up Pass
> After the audit is complete and all fixes applied:

Re-read your own fixes. Did fixing one bead break another's dependency chain? Re-run the coverage matrix — is it still 100%? Re-confirm:
- Every spec feature has ≥1 bead.
- Every TDD test ID is assigned.
- Both rollup sites + the 4 schema-epoch constants are covered.
- ZERO scope-drift language remains in any bead.
- Bead count per phase matches the strategy phase task estimates.
Run `bin/br-sync` and confirm `.beads/issues.jsonl` is current. If coverage is <100% on ANY dimension, the audit FAILS — fix and re-audit before signing off.
