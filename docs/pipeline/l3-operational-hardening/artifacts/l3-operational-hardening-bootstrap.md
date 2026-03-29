# L3 Operational Hardening — Test Bootstrap

## 1. Scope

This document defines how the Layer 3 operational-hardening tests are bootstrapped and run. It is the executable companion to:

- `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md`
- `docs/strategy/layers/layer-3-operational-hardening.md`

Layer 3 is not greenfield. The bootstrap extends the existing Layer 1 and Layer 2 harnesses, migrations, and package scripts rather than creating a second parallel test stack.

## 2. Success Criteria Coverage Summary

| DoD Item | Bootstrap Surface | Primary Tests / Commands |
|---------|-------------------|--------------------------|
| 1. Cron scheduler runs with explicit skill binding | `scheduler/cron.rs`, `CronSchedulerHarness` | `L3-T-cron_scheduler-001`-`010`; `cargo test -p daemon --test cron_scheduler` |
| 2. Batched unattended ingestion respects quotas and catchup | `ResourceArbiterFixture`, `SourceHealthFixture`, `DaemonHarness` | `L3-T-job_claim-006`-`009`; `L3-T-fault_host_events-003`-`006` |
| 3. Relation resolution classifies all four outcomes correctly | `packages/intelligence/src/relations/resolve-process-relations.ts` | `L3-T-resolve_process_relations-001`-`012`; `bun test packages/intelligence/src/relations/resolve-process-relations.test.ts` |
| 4. PTY hardening is reliable under real process conditions | `PtySessionHarness`, `ProcessTreeHelper`, `SessionReaper` | `L3-T-fault_process_cleanup-*`, `L3-T-fault_pty_edge_cases-*` |
| 5. Host sleep/wake and pressure are first-class runtime conditions | `TelemetryProbe`, `ResourceArbiterFixture` | `L3-T-fault_host_events-*` |
| 6. Typed quotas are enforced and auditable | `agents/arbiter.rs`, `jobs/claim.rs` | `L3-T-job_claim-006`-`009` |
| 7. Skill promotion flow is linear and validator-gated | skill fixtures + quality telemetry | `L3-T-skill_golden_journeys-*`, `L3-T-skill_shadow_compare-*` |
| 8. Source health, auth refresh, and breaker flow are operational | source-health skills, `source_runtime_state` | `L3-T-skill_drift_monitor-*` |
| 9. Worker-validator dry-run logs verdicts without enforcement | `validation/*`, `validation_verdicts`, scratch artifacts | `L3-T-validation_*` suites |
| 10. Error-manifest control-plane rendering is usable | `packages/manifests` renderer + daemon error paths | `L3-T-render_error_manifest-*` |
| 11. Validator canary health runs every 15-30 minutes | synthetic canary fixtures + telemetry | `L3-T-validation_canary_health-*` |
| 12. Health state transitions fire on failure spikes | validator telemetry + blackout banner source | `L3-T-validation_canary_health-003`-`008` |
| 13. Retroactive backlog accumulates and drains on recovery | backlog queue + stale-decision propagation | `L3-T-validation_retro_backlog-*` |
| 14. System reaches unattended target | live cron + live canary rehearsal | `T4` smoke + Manual Gate |
| 15. L3 schema activation matches DDL | `0005_l3_operational_hardening.sql` | `L3-T-schema_activation-*` |
| 16. Operator can trace verdict to bytes | four-layer observability fixtures | `L3-T-replay_*`, `L3-T-observability_telemetry-*` |
| 17. Telemetry emits at declared update points | `TelemetryProbe`, status IPC commands | `L3-T-observability_telemetry-*`, `CI-L3-008` |

## 3. Environment Prerequisites

| Requirement | Why it is needed | Validation |
|------------|------------------|------------|
| Rust nightly with 2024 edition support | daemon crate and tests target Rust 2024 | `rustup toolchain list | rg nightly` |
| Cargo | build/test/lint entrypoint | `cargo --version` |
| Bun | TypeScript package tests and desktop/browser surfaces | `bun --version` |
| `rg` | fixture discovery / validation scripts | `rg --version` |
| Unix-like OS (macOS/Linux) | PTY, process groups, signals, `waitpid`, UDS | `uname -s` in `{Darwin,Linux}` |
| writable temp filesystem | SQLite DBs, artifacts, UDS sockets, scratch manifests | create/write temp file |
| Layer 0/L1/L2 migrations present | Layer 3 builds on those schemas | `test -f migrations/0001_sqlite_state_initial.sql` etc. |
| existing helper binary support | real PTY/reaper/process tests | `cargo test -p daemon --features test-support --test agent_pty_session -- --list` |
| optional `codex` CLI | validator / live agent smoke on target host | `command -v codex` |
| optional internet access | live source-health `T4` lanes | HTTPS probe to FTS / Alibaba |

## 4. Validation Script

Run this script before implementing or executing the Layer 3 test plan:

```bash
#!/usr/bin/env bash
set -euo pipefail

pass() { printf '[PASS] %s\n' "$1"; }
warn() { printf '[WARN] %s\n' "$1"; }
fail() { printf '[FAIL] %s\n' "$1"; exit 1; }

rustup toolchain list | rg -q 'nightly' || fail 'Rust nightly toolchain missing'
cargo --version >/dev/null 2>&1 || fail 'cargo missing'
bun --version >/dev/null 2>&1 || fail 'bun missing'
rg --version >/dev/null 2>&1 || fail 'ripgrep missing'

case "$(uname -s)" in
  Darwin|Linux) pass 'Unix-like host detected' ;;
  *) fail 'Layer 3 PTY/process tests are Unix-only in this bootstrap' ;;
esac

test -f docs/strategy/layers/layer-3-operational-hardening.md || fail 'Layer 3 spec missing'
test -f docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md || fail 'Layer 3 TDD missing'
test -f migrations/0001_sqlite_state_initial.sql || fail 'L0 migration missing'
test -f migrations/0003_sqlite_runtime.sql || fail 'L1 migration missing'
test -f migrations/0004_l2_pending_operator_intents.sql || fail 'L2 migration missing'

if test -f migrations/0005_l3_operational_hardening.sql; then
  pass 'L3 migration present'
else
  warn 'L3 migration missing: create migrations/0005_l3_operational_hardening.sql before running L3 suites'
fi

for dir in \
  skills/auth-refresh/alibaba \
  skills/auth-refresh/1688 \
  skills/source-health/fts \
  skills/source-health/alibaba \
  skills/validation/base
do
  if test -d "$dir"; then
    pass "skill directory present: $dir"
  else
    warn "skill directory missing: $dir"
  fi
done

if command -v codex >/dev/null 2>&1; then
  pass 'codex CLI present'
else
  warn 'codex CLI absent: validator live-smoke lanes will be skipped'
fi

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
touch "$tmp_root/probe" || fail 'temp filesystem is not writable'
pass 'temp filesystem writable'

if curl -IsSf https://www.find-tender.service.gov.uk >/dev/null 2>&1; then
  pass 'FTS reachable for live smoke'
else
  warn 'FTS unreachable: T4 source-health smoke should be skipped'
fi
```

## 5. Dependency And Manifest Additions

### 5.1 Exact Rust Dependency Commands

From the repository root:

```bash
cargo add --manifest-path crates/daemon/Cargo.toml cron@0.16.0
cargo add --manifest-path crates/daemon/Cargo.toml sysinfo@0.38.4
```

Expected result in `crates/daemon/Cargo.toml`:

```toml
[dependencies]
cron = "0.16.0"
sysinfo = "0.38.4"
```

### 5.2 Required Manifest / Script Adjustments

- `crates/daemon/src/db/sqlite.rs`
  Add `const L3_MIGRATION_PATH: &str = "migrations/0005_l3_operational_hardening.sql";`
  Apply it after `0004_l2_pending_operator_intents.sql` and before `ensure_runtime_schema`.

- `packages/manifests/package.json`
  Expand the test script from one file to all Layer 3 renderer tests:

```json
{
  "scripts": {
    "test": "bun test tests/render-*.test.ts"
  }
}
```

- `packages/intelligence/package.json`
  Keep `bun test ./src`; Layer 3 relation tests live under `src/relations/`.

### 5.3 Main Module Declarations

`crates/daemon/src/main.rs` must add the Layer 3 modules explicitly instead of burying them in ad hoc local modules:

```rust
mod scheduler {
    pub mod cron;
}

mod sources {
    pub mod registry;
}

mod jobs {
    pub mod claim;
    pub mod runner;
}

mod pty {
    pub mod capture;
    pub mod hardening;
    pub mod health;
    pub mod session;
}

mod tracing {
    pub mod forensic;
    pub mod structured;
}

mod validation {
    pub mod executor;
    pub mod manifest_writer;
    pub mod policy;
}
```

## 6. Migration Scaffold

Create `migrations/0005_l3_operational_hardening.sql`.

### 6.1 Required Tables

The migration must create:

- `validation_verdicts`
- `source_runtime_state`
- `listing_evidence`
- `process_relations`
- `process_aliases`

### 6.2 Required Alter Statements

The migration must also activate existing forward-compatible columns:

- `workflow_jobs.parent_job_id`
- `workflow_jobs.validation_required`
- `workflow_jobs.validation_verdict_id`
- `agent_session_logs.run_manifest_path`
- `agent_session_logs.run_manifest_hash`

### 6.3 Minimum SQL Skeleton

```sql
-- Layer 3 operational hardening

create table if not exists validation_verdicts (
  id text primary key,
  worker_job_id text not null references workflow_jobs(id),
  worker_session_id text references agent_session_logs(id),
  validator_job_id text not null references workflow_jobs(id),
  validator_session_id text references agent_session_logs(id),
  validator_skill_path text not null,
  validator_skill_version text,
  verdict text not null,
  confidence_bps integer,
  summary text not null,
  basis_sufficiency text not null default 'medium',
  anchor_conflict_count integer not null default 0,
  incident_signature text,
  enforcement_eligible integer not null default 0,
  issues_json text not null default '[]',
  quality_assessment_json text,
  skill_improvement_suggestions_json text,
  run_manifest_hash text not null,
  oqg_signals_summary text,
  override_verdict text,
  override_reason text,
  override_by text,
  override_at text,
  verdict_rendered_at text not null,
  created_at text not null default current_timestamp
);

create table if not exists source_runtime_state (
  source_id text primary key references sources(id),
  last_health_state text not null default 'unknown',
  circuit_state text not null default 'closed',
  challenge_event_count integer not null default 0,
  challenge_window_started_at text,
  circuit_opened_at text,
  cooldown_until text,
  last_canary_at text,
  last_canary_result text,
  last_error_code text,
  updated_at text not null default current_timestamp,
  created_at text not null default current_timestamp
);

create table if not exists listing_evidence (
  id text primary key,
  source text not null,
  source_listing_id text not null,
  normalized_title text not null,
  structured_attributes_json text,
  parsed_standards_json text,
  parsed_units_json text,
  seller_identifier text,
  minimum_order_quantity numeric,
  shipping_terms text,
  condition text,
  compliance_evidence_tier text not null,
  intended_use_language text,
  is_ready_to_ship integer,
  fetched_at text not null,
  detail_page_fetched integer not null default 0,
  trust_state text not null default 'unverified',
  source_job_id text,
  created_at text not null default current_timestamp,
  unique (source, source_listing_id)
);

create table if not exists process_relations (
  id text primary key,
  process_id_a text not null references procurement_processes(id),
  process_id_b text not null references procurement_processes(id),
  relation_type text not null,
  confidence_bps integer not null,
  evidence text,
  owner_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at text not null default current_timestamp,
  created_at text not null default current_timestamp
);

create table if not exists process_aliases (
  alias_process_id text primary key,
  canonical_process_id text not null references procurement_processes(id),
  alias_reason text not null default 'merged_same_process',
  owner_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at text not null default current_timestamp,
  created_at text not null default current_timestamp
);
```

### 6.4 Migration Assertions

After the migration runs, `schema_activation.rs` must prove:

- all five tables exist
- `source_runtime_state.circuit_state` and `source_runtime_state.last_canary_result` exist immediately after migration
- `listing_evidence` enforces `unique(source, source_listing_id)`
- `process_relations.relation_type` is validated in code to exclude `separate`
- `validation_verdicts` and `workflow_jobs.validation_verdict_id` link atomically

## 7. Skill Directory Scaffold

### 7.1 New Directories

Create these directories:

```text
skills/
  auth-refresh/
    alibaba/
    1688/
  source-health/
    fts/
    alibaba/
  validation/
    base/
      references/
```

### 7.2 Required Files Per Operational Skill

Each operational skill directory must contain:

```text
SKILL.md
result.schema.json
checkpoint.schema.json
canary.inputs.jsonl
golden.assertions.yaml
```

Required frontmatter for `SKILL.md`:

```yaml
skill_id: auth-refresh/alibaba
skill_version: 2026.03.30-1
interaction_profile: pty_interactive
result_schema_version: 1.0
model_profile: browser_interactive
side_effect_class: read_only
```

Use `source-health/*` frontmatter similarly, with `skill_id` and `interaction_profile: pty_interactive`.

### 7.3 Required Files For `skills/validation/base/`

```text
skills/validation/base/
  SKILL.md
  result.schema.json
  checkpoint.schema.json
  canary.inputs.jsonl
  golden.assertions.yaml
  validator-rubric.yaml
  references/manifest-corpus-index.md
```

Validation skill frontmatter:

```yaml
skill_id: validation/base
skill_version: 2026.03.30-1
interaction_profile: headless_contract
result_schema_version: 1.0
model_profile: structured_extract_low_cost
side_effect_class: read_only
```

### 7.4 Fixture Content Requirements

- `canary.inputs.jsonl`
  Must include one known-good manifest, one known-bad manifest, and one known-insufficient-basis manifest.
- `golden.assertions.yaml`
  Must assert approve/reject/flag expectations plus basis-sufficiency expectations.
- `validator-rubric.yaml`
  Must be runtime-bounded and hashable; never inject raw gold assertions into live prompts.
- `references/manifest-corpus-index.md`
  Must map corpus items to `clean-real`, `weird-but-valid`, and `mutated-bad`.

## 8. Test Workspace Layout

```text
crates/daemon/tests/
  support/
    mod.rs
    db.rs
    artifacts.rs
    process.rs
    skills.rs
    assertions.rs
    cron.rs
    resources.rs
    validation.rs
    telemetry.rs
  fault_process_cleanup.rs
  fault_pty_edge_cases.rs
  fault_host_events.rs
  fault_partial_failure.rs
  replay_pty_sessions.rs
  replay_contract_sessions.rs
  cron_scheduler.rs
  job_claim.rs
  skill_golden_journeys.rs
  skill_shadow_compare.rs
  skill_drift_monitor.rs
  validation_policy.rs
  validation_executor.rs
  validation_dry_run.rs
  validation_golden_verdicts.rs
  validation_canary_health.rs
  validation_retro_backlog.rs
  schema_activation.rs
  observability_telemetry.rs

packages/intelligence/src/relations/
  resolve-process-relations.ts
  resolve-process-relations.test.ts

packages/manifests/tests/
  render-error-manifest.test.ts
```

## 9. Shared Support Extensions

### 9.1 `crates/daemon/tests/support/cron.rs`

Must provide:

- `CronSchedulerHarness::new(now: DateTime<Utc>)`
- `seed_definition(source_id, skill_id, skill_version, cron_expression, enabled)`
- `seed_source_runtime_state(source_id, circuit_state, cooldown_until, last_canary_result)`
- `run_due_sweep() -> Vec<EnqueuedJobSummary>`
- `advance_to(instant)`

### 9.2 `crates/daemon/tests/support/resources.rs`

Must provide:

- `ResourceArbiterFixture::with_defaults(browser_slots, agent_slots)`
- `seed_source_lane(source_id, lanes)`
- `admit(job_kind, source_id, uses_browser, is_validator)`
- `release(job_id)`
- `captured_events() -> Vec<GrantDenyReleaseEvent>`

### 9.3 `crates/daemon/tests/support/validation.rs`

Must provide:

- `ValidationPolicyFixture`
- `ValidatorHarness`
- `SyntheticCanaryFixture`
- `RetroBacklogFixture`

Critical assertions the support module must expose helpers for:

- worker `validation_required` flag transitions
- atomic insert of `validation_verdicts` plus worker linkage
- scratch artifact root assertions
- coarse process `validation_state` mapping assertions
- retroactive backlog ordering assertions
- exact `RunManifestFrontmatter` key parity assertions
- exact `DaemonEvidenceAppendix` key parity assertions

### 9.4 `crates/daemon/tests/support/telemetry.rs`

Must provide:

- `TelemetryProbe::record_skill_metric(...)`
- `TelemetryProbe::record_validation_metric(...)`
- `TelemetryProbe::transition_health_state(...)`
- `TelemetryProbe::blackout_incidents()`
- `TelemetryProbe::operational_status_snapshot()`

### 9.5 `crates/daemon/tests/support/status_router.rs`

Must provide:

- `OperationalStatusHarness::dispatch("get_cron_status", params)`
- `OperationalStatusHarness::dispatch("get_source_health_status", params)`
- `OperationalStatusHarness::dispatch("get_validation_status", params)`
- `OperationalStatusHarness::dispatch("get_resource_arbiter_status", params)`
- `assert_read_only_status_call(method)` proving unchanged `commit_seq`
- `assert_invalid_params(method, params)` proving `BridgeError::invalid_params`

The bootstrap should implement these as explicit `CommandRouter` dispatch tests, not as generic stringly typed smoke checks hidden inside another harness.

## 10. Fixture And Corpus Bootstrap

Create these fixture families:

| Fixture Family | Purpose |
|---------------|---------|
| `fixtures/manifests/clean-real/*.md` | approved validator corpus |
| `fixtures/manifests/weird-valid/*.md` | unusual-but-correct validator corpus |
| `fixtures/manifests/mutated-bad/*.md` | known-bad validator corpus |
| `fixtures/cron/europe-london-dst.json` | spring-forward and fall-back schedule cases |
| `fixtures/source-health/*.json` | canary pass/fail/timeout payloads and breaker windows |
| `fixtures/relations/*.json` | deterministic notice pairs and transitive cluster cases |
| `fixtures/status/*.json` | cron/source-health/validation/resource status snapshots for IPC tests |

Minimum corpus size for bootstrap:

- `20-30` clean-real manifests
- `15-20` weird-but-valid manifests
- `30-50` mutated-bad manifests

## 11. Cron Configuration Seeding

Layer 3 tests need deterministic cron seeds. Seed definitions in fixtures, not ad hoc test-local string literals only.

Example seed set:

```json
[
  {
    "source_id": "fts",
    "skill_id": "procurement-ingestion/fts",
    "skill_version": "2026.03.30-1",
    "cron_expression": "0 15 4 * * *",
    "enabled": true
  },
  {
    "source_id": "contracts-finder",
    "skill_id": "procurement-ingestion/contracts-finder",
    "skill_version": "2026.03.30-1",
    "cron_expression": "0 45 4 * * *",
    "enabled": true
  },
  {
    "source_id": "alibaba",
    "skill_id": "source-health/alibaba",
    "skill_version": "2026.03.30-1",
    "cron_expression": "0 */30 * * * *",
    "enabled": true
  }
]
```

Seed matching `source_runtime_state` defaults for all active sources:

```sql
insert or ignore into source_runtime_state (
  source_id,
  last_health_state,
  circuit_state,
  challenge_event_count
) values
  ('fts', 'green', 'closed', 0),
  ('contracts-finder', 'green', 'closed', 0),
  ('alibaba', 'unknown', 'closed', 0),
  ('1688', 'unknown', 'closed', 0);
```

## 12. Command Matrix

| Lane | Command |
|------|---------|
| Static | `cargo check --all-targets` |
| Static | `cargo clippy --all-targets -- -D warnings` |
| Static | `cargo fmt --check` |
| Static | `bun run test` |
| `T1` | `cargo test -p daemon --test validation_policy --test schema_activation --test observability_telemetry -- --nocapture` |
| `T1` | `bun test packages/intelligence/src/relations/resolve-process-relations.test.ts` |
| `T1` | `bun test packages/manifests/tests/render-error-manifest.test.ts` |
| `T2` | `cargo test -p daemon --test cron_scheduler --test job_claim --test validation_dry_run --test fault_pty_edge_cases -- --nocapture` |
| `T2` | `cargo test -p daemon --test fault_host_events --test fault_partial_failure -- --nocapture` |
| `T3` | `cargo test -p daemon --features test-support --test validation_executor --test validation_canary_health --test validation_retro_backlog -- --nocapture` |
| `T3` | `cargo test -p daemon --features test-support --test replay_pty_sessions --test replay_contract_sessions --test skill_golden_journeys --test skill_shadow_compare --test skill_drift_monitor -- --nocapture` |
| `T4` | live cron-triggered source-health / auth-refresh smoke on a networked runner |
| Manual Gate | five-day unattended run diary + zero-orphan verification + verdict-to-bytes drill-down |

## 13. Artifact Checklist

Before running the full Layer 3 suite, the following must exist:

- `migrations/0005_l3_operational_hardening.sql`
- new daemon modules under `crates/daemon/src/`
- new support modules under `crates/daemon/tests/support/`
- five new tables materialized in a migrated temp DB
- Layer 3 skill directories and validator/base file set
- manifest corpora (`clean-real`, `weird-valid`, `mutated-bad`)
- cron DST fixture set
- source-health canary fixture set
- relation-resolution fixture set
- renderer test fixtures for error manifests

## 14. Structured Logging Requirements

Every new Layer 3 test should emit:

- `[TEST_START] <test-id>`
- `[SETUP] ...`
- `[EXEC] ...`
- `[ASSERT] ...`
- `[CLEANUP] ...`
- `[TEST_END] <test-id> PASS|FAIL`

Additional requirements:

- validator-related tests must log worker job ID, validator job ID, and persisted verdict ID when present
- PTY/process tests must log root PID, PGID, and descendant count on teardown
- cron tests must log the seeded instant, timezone fixture, and expected enqueue count

## 15. False Positive Prevention

| Risk | Prevention |
|------|------------|
| Dry-run tests pass without proving isolation | assert no stage-head movement, no outbox row, scratch artifact root, and persisted verdict row in the same test |
| Relation tests pass with approximate language-only checks | assert exact classification enum, exact confidence threshold bucket, and canonical-root result |
| Breaker tests pass without persistence | assert the post-transition `source_runtime_state` row, not only in-memory state |
| Manifest tests pass on prose only | assert frontmatter fields, appendix fields, disagreement section, and file-size bound |
| Telemetry tests pass on missing metrics | assert `null` / `not_applicable` handling explicitly and reject silent zero-filling |
| IPC status tests mutate runtime accidentally | assert read-only snapshots and unchanged `commit_seq` after each status command |

## 16. Notes For Implementation

- Keep the 18 spec-declared test files intact. The three additive files exist to avoid burying schema, renderer, and operational-status coverage in unrelated suites.
- The bootstrap assumes the existing `test_agent` helper binary continues to be the one real process helper surface. Extend it rather than creating many tiny binaries.
- The packages/manifests renderer test is important because the Layer 3 spec names that shared library explicitly even though the current Rust daemon still owns the authoritative local error-manifest store.
- As of the current checked-in [`manifest.rs`](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/errors/manifest.rs), `ErrorCode` defines `28` variants, not `29`; the inventory-parity test should assert the exact current list and then separately prove the Layer 3 logical canary classes map onto that existing enum.
- The command-router operational status tests should name the exact method strings: `get_cron_status`, `get_source_health_status`, `get_validation_status`, and `get_resource_arbiter_status`.
