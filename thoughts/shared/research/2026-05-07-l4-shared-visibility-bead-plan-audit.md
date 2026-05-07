# L4 Shared Visibility Bead Plan Audit

Date: 2026-05-07
Scope: `testproject1-4zq` epic and child beads for Layer 4 Shared Visibility
Method: `/bead-plan-audit` workflow plus L4-specific drift checks requested in the gate prompt

## Result

Status: pass after corrections

The initial audit found bead hygiene drift in dependency ordering and mirror text. Those issues were corrected, the bead snapshots were refreshed, and the final pass is clean:

- Every `L4-FR-*` requirement in [layer-4-shared-visibility.md](/Users/alinaaitova/Desktop/Testproject1/docs/strategy/layers/layer-4-shared-visibility.md) is referenced by at least one live L4 bead: 147 / 147.
- Every `CI-L4-*` and `L4-T-*` ID in [l4-shared-visibility-tdd.md](/Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md) is referenced by at least one live L4 bead: 158 / 158.
- All 14 constitutional invariants from spec §1.9 are covered by live beads through `L4-FR-1.9.1` through `L4-FR-1.9.14`.
- All 18 Definition of Done rows from spec §4.1 are owned by the Phase 6 final stage gate and by the implementation beads named in that walkthrough.
- Static drift scan found no red-flag scope language in the refreshed live bead corpus or the bulk-import mirror.
- `br lint` is clean: `No template warnings found (74 issues checked)`.
- `br dep cycles --json` reports zero cycles.

## Corrections Applied

Bead corpus:
- `testproject1-4zq` root epic now carries the required Source of Truth warning naming spec, TDD, strategy, and bootstrap documents.
- Phase 6 dependency edges now enforce the strategy prerequisite that Phase 2, 3, 4, and 5 gates pass before Phase 6 integration work:
  - `testproject1-4zq.6.1` now depends on P2, P3, P4, and P5 review gates.
  - `testproject1-4zq.6.2` now depends on P2, P3, P4, and P5 review gates plus the Phase 2 schema-epoch bead.
  - `testproject1-4zq.6.3` now depends on P2, P3, P4, and P5 review gates plus the Phase 1 Supabase/schema-hash owners.
  - `testproject1-4zq.6.5` now depends on P2, P3, P4, and P5 review gates plus the remote-config TTL bead.
- The affected Phase 6 bead descriptions were updated so their textual `## Dependencies` sections match the actual graph.

Bulk-import mirror:
- Removed load-bearing scope-drift wording from the Phase 4 calibration handoff text.
- Updated Phase 6 dependency text to match the corrected live graph.
- Corrected the Phase 6 provenance bead file ownership to use `crates/daemon/src/db/sync.rs` and existing stage-write modules; `crates/daemon/src/jobs/runner.rs` remains exclusively owned by the Phase 4 runner bead.

## Coverage Matrix

| Area | Source requirement | Bead coverage | Status |
| --- | --- | --- | --- |
| Push protocol | §1.3, §1.9 push invariants | P1 schema/outbox owners, P2 outbox writer, batch envelope, RPC, equal-version, status writer, lease/fencing, epoch pause, P2 gate | Covered |
| Pull protocol | §1.4 | P3 pull cursors, hi-water pagination, pull-before-push service contract, P6 startup wiring | Covered |
| CAS and command ledger | §1.5 | P3 CAS RPC, command-ledger retention, pending-intent reconciliation, decision-conflict IPC | Covered |
| Active-writer lease | §1.6 | P2 active-writer lease, stale-writer re-envelope, P6 startup wiring | Covered |
| Schema epoch guard | §1.7, §5.2 | P2 schema-epoch pause, P6 epoch-repair generation | Covered |
| Degraded mode | §1.8, §3.3, §3.7 | P2 viewer sync status, P3 remote-config TTL, P5 freshness UX, P6 cross-authority/TTL | Covered |
| Invariants | §1.9.1..1.9.14 | All 14 exact `L4-FR-1.9.*` IDs appear in live beads | Covered |
| Push-owned projections | §2.1 | P1 SQLite/Supabase migrations and P2 push engine beads | Covered |
| Summary projections | §2.2 | P1 Supabase migration bundle and P6 redaction closure | Covered |
| Remote-owned tables | §2.3 | P1 Supabase migrations, P3 pull/CAS, P6 RLS/redaction | Covered |
| RLS matrix | §2.5 | P1 Supabase migration bundle, P1 schema-hash gate, P6 final RLS/redaction closure | Covered |
| Migrations | §2.6 | P1 SQLite migration, registry epoch, Supabase migration bundle | Covered |
| Cross-authority crossings | §2.7 | P6 cross-authority provenance and TTL | Covered |
| Web portal files | §3.1 | P5 package/config, Supabase boundary, Broadcast client, layout/redirect, list, detail, status, freshness boundary | Covered |
| Broadcast invalidation | §3.2 | P5 Broadcast client, list/detail/status pages, P6 epoch generation | Covered |
| Enforcement transitions | §3.4 | P4 provisional storage, policy readiness, executor quarantine, runner promotion, feedback loop | Covered |
| Validator overlays | §3.5 | Four P4 overlay beads, each owning the six-file overlay contract | Covered |
| Calibration harness | §3.6 | P4 calibration corpora/cases, math readiness, override feedback loop, overlay gate | Covered |
| viewer_sync_status | §3.7 | P1 DDL, P2 status writer, P5 freshness UX, P6 wiring | Covered |
| Definition of Done | §4.1 | P6 stage gate owns rows 1..18 with implementation bead links | Covered |
| L4 stubs | §5.2 | P6 epoch repair, P6 logical consistency auditor stub | Covered |

## TDD Coverage

Exact ID extraction showed complete coverage:

- Static gates: `CI-L4-001` through `CI-L4-008` covered.
- SQLite schema tests: `L4-T-sqlite_schema-001` through `024` covered.
- SQLite/Supabase sync tests: `L4-T-sqlite_supabase_sync-001` through `080` covered.
- Redaction boundary tests: `L4-T-redaction_boundary-001` through `026` covered.
- Web contracts tests: `L4-T-contracts_spec-001` through `020` covered.

Commands used for the final coverage check:

```bash
rg -o 'L4-FR-[0-9]+(?:\.[0-9]+)+' docs/strategy/layers/layer-4-shared-visibility.md | sort -u | wc -l
rg -o 'L4-FR-[0-9]+(?:\.[0-9]+)+' /tmp/l4-beads-show.txt | sort -u | wc -l
rg -o '(?:CI-L4-[0-9]{3}|L4-T-[A-Za-z_]+-[0-9]{3})' docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md | sort -u | wc -l
rg -o '(?:CI-L4-[0-9]{3}|L4-T-[A-Za-z_]+-[0-9]{3})' /tmp/l4-beads-show.txt | sort -u | wc -l
```

Final counts were `147`, `147`, `158`, and `158`. The `comm -23` missing-ID checks for both FR IDs and TDD IDs returned no rows.

## L4-Specific Checks

Sync engine:
- All 11 push-engine constitutional invariants are covered across P1/P2/P6 beads.
- Equal-version semantics are assigned as four cases in `l4-p2-equal-version-semantics`.
- Hi-water fenced pull is assigned across all three pull domains in P3.
- Pull-before-push ordering is explicit in P3 service contract and P6 startup wiring.
- Daemon tombstone visibility is assigned to P6 tombstone retention and P6 RLS/redaction closure.
- Writer fencing with two daemons is assigned to P2 lease/fencing and P6 startup wiring.
- Schema epoch guard and minimum repair/reproject path are assigned to P2 and P6.

CAS and command ledger:
- CAS RPC exists with apply, conflict, idempotent replay, and command-unknown-after-GC cases.
- Command ledger durability is separate from `operator_decision_heads.command_id`.
- Pending intent reconciliation includes latest-per-process coalescing.
- Decision conflict IPC event is assigned to `l4-p3-decision-conflict-ipc`.

Validation enforcement:
- Provisional promotion buffer is named and assigned to P4 storage/executor/runner beads.
- Per-slice readiness gate owns all seven prerequisites.
- All four demotion triggers are testable acceptance items.
- `AGENT_SUSPECT_OUTPUT` is assigned to `crates/daemon/src/errors/manifest.rs`.
- Quarantine flow keeps rejected results local, writes an error manifest, and blocks stage-head advance.
- Four validator overlays are present as beads, each with `SKILL.md`, four schemas, and references.

Calibration harness:
- Three corpora, all nine mutation families, pairwise challenge sets, Wilson-bound precision math, rule-of-three false-pass math, cross-model sampling, and override feedback loop are assigned to P4 beads.

Web portal:
- All seven web source-file groups are assigned.
- Broadcast topic schemes are assigned for list, detail, and sync/status pages.
- Three-state degraded UX uses green, amber, and red thresholds at 90 seconds and 5 minutes.
- Viewer-credentials-only enforcement, Playwright RLS boundary, root `package.json` workspace integration, and `packages/ui` lint boundary are assigned.

RLS and redaction:
- 28 role/category RLS assertions are assigned.
- All six summary projections have redaction tests.
- Daemon-pull tombstone visibility is assigned and explicitly avoids a daemon-side `deleted_at IS NULL` filter on remote-owned tables.
- Direct operator `UPDATE` on `operator_decision_heads` is blocked by RLS; mutation goes through CAS RPC.

Cross-layer notes:
- L0 `commit_group_id` and `writer_epoch` are represented in the Phase 1 schema and Phase 2 push beads.
- L2 FreshnessBadge upgrade is assigned to P5 with Phase 6 verification.
- L6 remote-config staleness safety TTL is assigned to P3 and carried into P6 cross-authority/TTL verification.

## Phase Balance

All rows by phase:

| Phase | Count |
| --- | ---: |
| Phase 1 | 9 |
| Phase 2 | 10 |
| Phase 3 | 10 |
| Phase 4 | 14 |
| Phase 5 | 10 |
| Phase 6 | 9 |

Implementation tasks excluding review gates:

| Phase | Count | Expected |
| --- | ---: | --- |
| Phase 1 | 7 | 5-8 |
| Phase 2 | 8 | 8-12 |
| Phase 3 | 8 | 6-8 |
| Phase 4 | 12 | 10-15 |
| Phase 5 | 8 | 7-10 |
| Phase 6 | 7 | 6-10 |

Phase 6 is substantive: startup wiring, epoch repair, final RLS/redaction closure, tombstone retention proof, cross-authority provenance/TTL, logical consistency auditor stub, two-user verification, and final DoD gate.

## Chokepoint Discipline

| Chokepoint | Owning bead | Status |
| --- | --- | --- |
| `crates/daemon/src/main.rs` | `l4-p6-main-startup-sync-wiring` | Single owner |
| `crates/daemon/src/jobs/runner.rs` | `l4-p4-runner-provisional-promotion` | Single owner |
| `crates/daemon/src/validation/policy.rs` | `l4-p4-policy-readiness-contract` | Single owner |
| `crates/daemon/src/validation/executor.rs` | `l4-p4-executor-overlay-quarantine` | Single owner |
| `crates/daemon/src/ipc/handlers/decisions.rs` | `l4-p3-decision-conflict-ipc` | Single owner |
| `crates/daemon/src/errors/manifest.rs` | `l4-p4-error-manifest-classification` | Single owner |
| `crates/daemon/Cargo.toml` | `l4-p1-daemon-http-client` | Single owner |
| root `package.json` | `l4-p1-root-web-workspace-scripts` | Single owner |
| `packages/ui/freshness-badge.tsx` | `l4-p5-web-freshness-consumption-and-boundary` | Single owner |

## Dependency Validation

- `br dep cycles --json` returned `{"cycles":[],"count":0}`.
- Phase gates depend on the full implementation set for their phase.
- P6 integration tasks now respect the strategy's P2/P3/P4/P5 gate prerequisites, either directly or through a predecessor P6 task.
- P6 final stage gate depends on all seven P6 implementation beads; it also has transitive coverage for the Phase 5 gate through `l4-p6-final-rls-redaction-closure`, `l4-p6-cross-authority-provenance-and-ttl`, and `l4-p6-two-user-verification`.

## Final Verification Commands

```bash
br list --label layer-4 --json --limit 0 > /tmp/l4-beads-audit.json
br ready --label layer-4 --json --limit 0
br lint
br dep cycles --json
rg -in '<red-flag regex from the gate prompt>' /tmp/l4-beads-show.txt
rg -in '<red-flag regex from the gate prompt>' docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bulk-import.md
```

The two `rg` drift scans exited with code 1 and no matches, which is the expected clean result.

## Follow-Up Pass

Date: 2026-05-07
Verdict: warnings, investigated and non-blocking

The follow-up pass re-read the live bead graph after commit `27ae22f` and found no blocking drift:

- Full L4 inventory: 63 rows, `has_more=false`.
- FR coverage diff: empty, `147 / 147`.
- TDD/CI coverage diff: empty, `158 / 158`.
- Red-flag scope language grep over the full bead title/body corpus: no matches.
- Phase counts still match strategy expectations: implementation tasks `7 / 8 / 8 / 12 / 8 / 7`.
- Chokepoint target check was run against each bead's `## Files to Create/Modify` section, not incidental references. Each chokepoint has exactly one target owner.
- `br lint` remains clean and `br dep cycles --json` remains cycle-free.
- `bin/br-sync` reports no dirty issues; `git status --short --branch` is `## main...origin/main`.
- `br ready --label layer-4 --json --limit 0` returns four ready Phase 1 task beads: `l4-p1-daemon-http-client`, `l4-p1-sqlite-sync-migration`, `l4-p1-supabase-config-and-migration-bundle`, and `l4-p1-root-web-workspace-scripts`.
- `br show testproject1-4zq` shows all six child sub-epics as dependents of the root L4 epic. `br dep list testproject1-4zq --json` is empty because the root epic has dependents rather than parents.

Investigated warning:

- A raw count over all non-review task `## Tests to Pass` sections shows 41 `L4-T-*` IDs appearing in more than one implementation bead. This is not uncovered scope and not a broken dependency. These are shared integration pass gates where multiple implementation surfaces must keep the same end-to-end test green, such as schema DDL plus schema registry, CAS RPC plus IPC conflict event, or web freshness plus final RLS closure.
- The stricter coverage diff remains clean: all 150 `L4-T-*` IDs are present in implementation task test sections, with zero missing IDs and zero extra IDs.
- Review gates and sub-epic summaries are treated as aggregators, not primary owners.

No bead edits were required by this follow-up pass.
