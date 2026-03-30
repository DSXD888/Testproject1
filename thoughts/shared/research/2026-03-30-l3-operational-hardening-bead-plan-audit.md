# L3 Operational Hardening Bead Plan Audit

Date: 2026-03-30
Scope: `testproject1-hmw` epic and child beads for Layer 3 Operational Hardening
Method: direct SQLite audit of `.beads/beads.db` because `br list --json` / `br show --json` currently fail in this repo with `DATABASE_ERROR: UNIQUE constraint failed: issues.id`

## Result

Status: pass after bead corrections

Verified after correction:
- Every `L3-FR-*` requirement in [layer-3-operational-hardening.md](/Users/alinaaitova/Desktop/Testproject1/docs/strategy/layers/layer-3-operational-hardening.md) is referenced by at least one live L3 bead.
- Every `CI-L3-*` and `L3-T-*` ID in [l3-operational-hardening-tdd.md](/Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md) is referenced by at least one live L3 bead.
- No `L3-FR-*` requirement is now owned only by test-labeled beads.
- `pty/session.rs` is now explicitly owned by the PTY hardening bead.

## Drift Fixed

Updated bead ownership / traceability:
- `testproject1-hmw.11`
  Added explicit ownership of `crates/daemon/src/pty/session.rs` to close the PTY hardening integration gap.
- `testproject1-hmw.13`
  Added §4.4 observability ownership and replay / joinability TDD anchors for `tracing/structured.rs`.
- `testproject1-hmw.14`
  Added missing TDD coverage plus §4.4 forensic ownership for `tracing/forensic.rs`.
- `testproject1-hmw.17`
  Added explicit §2.2 manifest-field ownership and direct manifest-field TDD anchors.
- `testproject1-hmw.20`
  Added §2.9.1 and §2.9.2 calibration corpus / target ownership so calibration is not test-only.
- `testproject1-hmw.30`
  Added §2.9.3 Gate A / Gate B enforcement-readiness ownership.
- `testproject1-hmw.31`
  Added §4.6.1 and §4.6.2 validation-telemetry ownership.
- `testproject1-hmw.34`
  Added §4.5.1 skill-level telemetry ownership.

## L3-Specific Checks

12 daemon modules:
- Covered by dedicated beads: `scheduler/cron.rs`, `sources/registry.rs`, `jobs/claim.rs`, `pty/hardening.rs`, `agents/arbiter.rs`, `db/checkpoints.rs`, `tracing/structured.rs`, `tracing/forensic.rs`, `validation/policy.rs`, `validation/executor.rs`, `validation/manifest_writer.rs`.
- Existing-surface extension covered: `sessions/reaper.rs`.
- Integration correction applied: `pty/session.rs` is now explicitly named in the PTY hardening bead.

Migration bead:
- `testproject1-hmw.1` covers all five L3 tables plus Layer 3 `ALTER` usage on `workflow_jobs` and `agent_session_logs`.

Skill directories:
- Operational skill directories covered by `testproject1-hmw.8`.
- Validator base directory and calibration fixture ownership covered by `testproject1-hmw.20`.

TypeScript surfaces:
- `packages/intelligence/src/relations/resolve-process-relations.ts` covered by `testproject1-hmw.26` with Bun gates.
- `packages/manifests/render-error-manifest.ts` and `packages/manifests/error.md.ts` covered by `testproject1-hmw.28`.

18 test files:
- Fault injection, replay, scheduler/jobs, skill lifecycle, validation, and relation test files all assigned to beads.

Worker-validator system:
- Manifest writer now explicitly owns §2.1-§2.2 output shape.
- Policy bead owns mode / trigger / sampling semantics.
- Executor bead owns kickoff packet composition and verdict contract.
- Runner integration bead owns verdict handling.
- Calibration and readiness are no longer test-only.
- Validation telemetry and health transitions are no longer test-only.

## Notes

- No dedicated bead was added for new `ErrorCode` variants in `crates/daemon/src/errors/manifest.rs`. This is intentional and matches the spec: Layer 3 maps logical source-health classes onto the existing error-code surface rather than silently inventing new enum variants.
- The remaining non-bead issue is tool health: `br` JSON commands are still broken on this repo’s bead database. The audit used direct SQLite reads and direct SQLite updates to the affected bead rows.
