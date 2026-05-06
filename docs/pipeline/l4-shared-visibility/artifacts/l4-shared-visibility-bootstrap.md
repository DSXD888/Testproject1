# L4 Shared Visibility - Test Bootstrap

## 1. Scope

This document defines how the Layer 4 Shared Visibility test plan is bootstrapped and run. It is the executable companion to:

- `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md`
- `docs/strategy/layers/layer-4-shared-visibility.md`

Layer 4 extends the existing Rust daemon, SQLite migration path, L3 validation system, Bun workspaces, shared UI package, and desktop E2E conventions. It adds Supabase, `apps/web`, sync-state SQLite tables, RLS policy tests, and validation enforcement calibration infrastructure.

The chosen Supabase test approach is the local Supabase CLI Docker stack. This provides Postgres, PostgREST RPC, Auth, and Realtime Broadcast in one harness, so sync tests exercise the same HTTP/RLS/Broadcast surfaces used by the daemon and web portal. The Rust daemon must call Supabase RPCs through `reqwest`; tests may use SQL connections only for migration setup, fixture seeding, role checks, and state assertions.

## 2. Success Criteria Coverage Summary

| DoD Item | Bootstrap Surface | Primary Tests / Commands |
|---------|-------------------|--------------------------|
| 1. Transactional outbox entries | `migrations/0012_l4_sync_state.sql`, `OutboxHarness` | `L4-T-sqlite_supabase_sync-001`-`003` |
| 2. Transactional Supabase RPC apply | `apply_push_batch`, daemon `reqwest` client | `L4-T-sqlite_supabase_sync-012`-`019` |
| 3. Cursor pull with hi-water fence | `pull_cursors`, remote-owned sequence triggers | `L4-T-sqlite_supabase_sync-025`-`029` |
| 4. Operator decisions through CAS | `pending_operator_intents`, `update_operator_decision`, command ledger | `L4-T-sqlite_supabase_sync-030`-`038` |
| 5. Active writer lease and fencing | `active_writer_lease`, `viewer_sync_status` | `L4-T-sqlite_supabase_sync-039`-`043` |
| 6. RLS for all roles | `infra/supabase/migrations/0004_rls_policies.sql` | `L4-T-redaction_boundary-001`-`009`, `021`-`026` |
| 7. Web viewing portal | `apps/web`, Supabase client, shared UI | `L4-T-contracts_spec-001`-`004`, `011`, `012`, `020` |
| 8. Broadcast invalidation | Supabase Realtime, page-scoped topics | `L4-T-contracts_spec-008`-`010`, `019` |
| 9. Sync freshness states | `viewer_sync_status`, `packages/ui/freshness-badge.tsx` | `L4-T-sqlite_supabase_sync-046`, `L4-T-contracts_spec-005`-`007` |
| 10. Validation enforcement and calibration | `provisional_worker_outputs`, `enforcement_readiness`, calibration cases | `L4-T-sqlite_supabase_sync-022`-`024`, `050`-`057`, `062`-`077` |
| 11. Four domain overlays | `skills/validation/overlay-*/` | `L4-T-sqlite_supabase_sync-058`-`061` |
| 12. Two users view one pipeline | desktop-seeded sync fixture plus web viewer | `L4-T-contracts_spec-017`, `018` |
| 13. Stale writer rejected | writer epoch in lease and RPC | `L4-T-sqlite_supabase_sync-041` |
| 14. CAS command ledger replay | `operator_decision_commands` | `L4-T-sqlite_supabase_sync-034`-`038` |
| 15. Provisional promotion buffer | runner/executor promotion path | `L4-T-sqlite_supabase_sync-022`-`024`, `079`, `080` |
| 16. Equal-version retry semantics | Supabase RPC row-version checks | `L4-T-sqlite_supabase_sync-016`-`019` |
| 17. Viewer-scoped credentials | web env, RLS, Playwright network checks | `L4-T-redaction_boundary-009`, `025`, `L4-T-contracts_spec-013`-`015` |
| 18. Redaction boundary | summary projections and sentinel fixtures | `L4-T-redaction_boundary-010`-`020` |

## 3. Environment Prerequisites

| Requirement | Why it is needed | Validation |
|------------|------------------|------------|
| Rust nightly with 2024 edition support | daemon crate and tests target Rust 2024 | `rustup toolchain list | rg nightly` |
| Cargo | Rust build/test/lint entrypoint | `cargo --version` |
| Bun `1.3.10` compatible with root `packageManager` | TypeScript, packages, desktop, web, Playwright | `bun --version` |
| Docker running | local Supabase CLI stack | `docker info` |
| Supabase CLI `>= 2.98.1` | local Postgres/PostgREST/Auth/Realtime stack and migration application | `supabase --version` |
| `curl` and `jq` | prerequisite probes and RPC smoke checks | `curl --version`, `jq --version` |
| `rg` | inventory and policy scans | `rg --version` |
| writable temp filesystem | SQLite DBs, artifacts, Supabase temp output, Playwright traces | create/write temp file |
| Layer 0-L3 migrations present | L4 extends existing SQLite authority | `test -f migrations/0001_sqlite_state_initial.sql`, `0004`, `0005` |
| `crates/daemon/Cargo.toml` has no pre-existing HTTP client before L4 edit | confirms L4 owns the daemon HTTP client addition | `rg -n 'reqwest|ureq|hyper|tokio-postgres' crates/daemon/Cargo.toml` |
| Supabase project configuration | local and hosted sync target setup | `test -f infra/supabase/config.toml` after scaffold |
| Supabase env vars | daemon sync, web viewer auth, tests | see env var table below |
| `apps/web` workspace wiring | web portal build/test/lint execution | root `bun run build`, `bun run test`, `bun run lint` include web |
| `packages/ui` import boundary | shared UI stays presentation-only | root lint `rg` boundary check |
| calibration corpus root | enforcement readiness tests | `test -d tests/fixtures/calibration` |

Required environment variables:

| Variable | Consumer | Required Shape |
|----------|----------|----------------|
| `SUPABASE_URL` | daemon, web, tests | local or hosted Supabase API URL |
| `SUPABASE_ANON_KEY` | web viewer client | anon/viewer-scoped key with RLS enabled |
| `SUPABASE_SERVICE_ROLE_KEY` | daemon/test harness only | service role key; never exposed to browser |
| `SUPABASE_JWT_SECRET` | local role token generation | JWT secret from local stack |
| `L4_SUPABASE_DB_URL` | migration and assertion harness | Postgres connection string for local Supabase DB |
| `L4_VIEWER_EMAIL` | Playwright auth seed | viewer test account email |
| `L4_VIEWER_PASSWORD` | Playwright auth seed | viewer test account password |
| `L4_OPERATOR_EMAIL` | CAS/RLS tests | operator test account email |
| `L4_OPERATOR_PASSWORD` | CAS/RLS tests | operator test account password |
| `L4_DAEMON_EXECUTOR_ID` | sync tests | deterministic executor ID |
| `L4_ORG_ID` | all Supabase tests | single-org fixture ID |
| `NEXT_PUBLIC_SUPABASE_URL` | `apps/web` | same API URL, browser-safe |
| `NEXT_PUBLIC_SUPABASE_ANON_KEY` | `apps/web` | browser-safe anon key only |

Minimum role/grant scaffold:

```sql
create role viewer nologin;
create role operator nologin;
create role admin nologin;
create role daemon nologin;

grant usage on schema public to viewer, operator, admin, daemon;

-- Table-specific grants are applied in infra/supabase/migrations/0004_rls_policies.sql.
-- Mutation rights remain behind apply_push_batch and update_operator_decision.
```

## 4. Validation Script

Run this from the repository root before implementation or before executing the full L4 suite:

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
curl --version >/dev/null 2>&1 || fail 'curl missing'
jq --version >/dev/null 2>&1 || fail 'jq missing'

docker info >/dev/null 2>&1 || fail 'Docker is not running'
supabase --version >/dev/null 2>&1 || fail 'Supabase CLI missing'

test -f docs/strategy/layers/layer-4-shared-visibility.md || fail 'L4 spec missing'
test -f docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md || fail 'L4 TDD missing'
test -f docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md || fail 'L4 bootstrap missing'

test -f migrations/0001_sqlite_state_initial.sql || fail 'L0 migration missing'
test -f migrations/0004_l2_pending_operator_intents.sql || fail 'L2 pending-intents migration missing'
test -f migrations/0005_l3_operational_hardening.sql || fail 'L3 validation migration missing'

if test -f migrations/0012_l4_sync_state.sql; then
  pass 'L4 SQLite migration present'
else
  warn 'L4 SQLite migration not created yet: migrations/0012_l4_sync_state.sql'
fi

if test -f infra/supabase/config.toml; then
  pass 'Supabase config present'
else
  warn 'Supabase config not created yet: infra/supabase/config.toml'
fi

expected_supabase_migrations=(
  infra/supabase/migrations/0001_projection_tables.sql
  infra/supabase/migrations/0002_remote_owned_tables.sql
  infra/supabase/migrations/0003_summary_tables.sql
  infra/supabase/migrations/0004_rls_policies.sql
  infra/supabase/migrations/0005_sync_rpcs.sql
  infra/supabase/migrations/0006_schema_metadata.sql
  infra/supabase/migrations/0007_command_ledger.sql
  infra/supabase/migrations/0008_decision_summaries.sql
)

for migration in "${expected_supabase_migrations[@]}"; do
  if test -f "$migration"; then
    pass "Supabase migration present: $migration"
  else
    warn "Supabase migration missing: $migration"
  fi
done

for dir in \
  skills/validation/overlay-procurement-ingestion \
  skills/validation/overlay-supplier-enrichment \
  skills/validation/overlay-document-acquisition \
  skills/validation/overlay-product-extraction
do
  if test -d "$dir"; then
    pass "overlay directory present: $dir"
  else
    warn "overlay directory missing: $dir"
  fi
done

if rg -n '^import .*(@tauri-apps/api|.*apps/desktop|.*crates/daemon|.*supabase|.*transport)' packages/ui; then
  fail 'packages/ui import boundary violated'
else
  pass 'packages/ui import boundary clean'
fi

if rg -n -F 'reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }' crates/daemon/Cargo.toml; then
  pass 'daemon reqwest dependency present'
else
  warn 'daemon reqwest dependency line missing'
fi

tmp_root="$(mktemp -d)"
touch "$tmp_root/probe" || fail 'temp filesystem not writable'
pass 'temp filesystem writable'

for name in SUPABASE_URL SUPABASE_ANON_KEY SUPABASE_SERVICE_ROLE_KEY SUPABASE_JWT_SECRET L4_ORG_ID; do
  if test -n "${!name:-}"; then
    pass "env present: $name"
  else
    warn "env missing: $name"
  fi
done
```

## 5. Dependency And Manifest Additions

### 5.1 Daemon HTTP Client

Add the daemon HTTP client dependency exactly to `crates/daemon/Cargo.toml`:

```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
```

Do not add `tokio-postgres` as a production daemon dependency for L4 sync. The daemon sync path calls Supabase REST RPCs with `reqwest`. SQL connections in tests are harness-only for migration setup and state assertions.

Verification:

```bash
cargo check --all-targets
cargo test -p daemon --test sqlite_supabase_sync -- --nocapture
```

### 5.2 Root `package.json`

The root workspace already includes `apps/*`; keep that entry. Update scripts so the web workspace is actually exercised.

Required script shape:

```json
{
  "scripts": {
    "build": "(cd packages/ui && bun run build) && (cd packages/intelligence && bun run build) && (cd packages/manifests && bun run build) && bunx tsc -p apps/desktop/tsconfig.json --noEmit && (cd apps/web && bun run build)",
    "lint": "if rg -n '^import .*(@tauri-apps/api|.*apps/desktop|.*crates/daemon|.*supabase|.*transport)' packages/ui; then exit 1; fi && (cd packages/intelligence && bun run lint) && (cd packages/manifests && bun run lint) && (cd apps/web && bun run lint)",
    "test": "(cd packages/ui && bun run test) && (cd apps/desktop && bun run test) && (cd packages/intelligence && bun run test) && (cd packages/manifests && bun run test) && (cd apps/web && bun run test)"
  }
}
```

### 5.3 `apps/web/package.json`

Create `apps/web/package.json` with explicit versions and no dependency ranges. It must include:

```json
{
  "name": "@project/web",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "next build",
    "lint": "bunx tsc -p tsconfig.json --noEmit",
    "test": "bunx tsc -p tsconfig.json --noEmit",
    "test:e2e": "playwright test"
  },
  "dependencies": {
    "@supabase/supabase-js": "2.103.2",
    "next": "16.1.0",
    "react": "19.2.4",
    "react-dom": "19.2.4"
  },
  "devDependencies": {
    "@playwright/test": "1.58.2",
    "@types/node": "25.5.0",
    "@types/react": "19.2.14",
    "@types/react-dom": "19.2.3",
    "typescript": "6.0.2"
  }
}
```

### 5.4 `packages/ui` Invariants

`packages/ui` remains presentation-only:

- no `@tauri-apps/api`
- no `apps/desktop`
- no `crates/daemon`
- no `supabase`
- no `transport`
- no direct Broadcast client

`apps/web/src/lib/supabase.ts` is the single direct Supabase client boundary. `apps/web/src/lib/broadcast.ts` imports from that boundary rather than importing Supabase directly.

## 6. Migration Scaffold

### 6.1 SQLite Migration

Create `migrations/0012_l4_sync_state.sql`.

Required tables:

- `local_outbox`
- `sync_batches`
- `pull_cursors`
- `provisional_worker_outputs`
- `enforcement_readiness`
- `projection_generations`
- `validation_calibration_cases`
- `validation_incident_clusters`

Minimum table responsibilities:

| Table | Required Assertions |
|-------|---------------------|
| `local_outbox` | full replay envelope, monotonic `local_seq`, `commit_group_id`, nullable `writer_epoch`, persisted `content_hash`, op check |
| `sync_batches` | immutable envelope, in-flight range, status, retry count, response payload, writer epoch, schema epoch, stage run, commit group coverage |
| `pull_cursors` | one row per remote-owned domain, last applied `remote_change_seq`, no cross-domain serializability requirement |
| `provisional_worker_outputs` | local-only worker output staging with status check and `worker_job_id` index |
| `enforcement_readiness` | deterministic slice key, worker/source/behavior versions, state check, bps metrics, count metrics, demotion reason, threshold snapshot |
| `projection_generations` | epoch-repair ledger with status, max local seq, remote ack, error JSON |
| `validation_calibration_cases` | clean/weird/mutated/pairwise/live evidence, expected/actual verdicts, mutation family, pair id, model family |
| `validation_incident_clusters` | incident signature, dominant failure family, affected slices, candidate mutation template JSON |

`crates/daemon/src/db/sqlite.rs` must add:

```rust
const L4_SYNC_STATE_MIGRATION_PATH: &str = "migrations/0012_l4_sync_state.sql";
```

and register:

```rust
("0012_l4_sync_state", 9, L4_SYNC_STATE_MIGRATION_PATH)
```

`DEFAULT_SCHEMA_EPOCH` becomes `9`.

### 6.2 Supabase Migrations

Create `infra/supabase/config.toml` and `infra/supabase/migrations/`.

Initialize the local project from the repository root:

```bash
mkdir -p infra
(cd infra && supabase init)
(cd infra/supabase && supabase start)
(cd infra/supabase && supabase status --output env)
```

Persist only non-secret scaffold files in the repo. Copy generated local keys into an untracked `.env.local` or CI secret store; never commit service-role credentials.

Required migrations:

| Migration | Purpose | Required Tests |
|-----------|---------|----------------|
| `0001_projection_tables.sql` | all 20 push-owned projection tables plus 3 junction tables | `L4-T-sqlite_schema-019` |
| `0002_remote_owned_tables.sql` | `shared_company_profile`, `shared_source_configs`, `operator_decision_heads`, per-domain sequences/triggers | `L4-T-sqlite_schema-021`, `L4-T-sqlite_supabase_sync-027` |
| `0003_summary_tables.sql` | `workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status`, `executors` | `L4-T-sqlite_schema-020`, redaction tests |
| `0004_rls_policies.sql` | RLS policies and grants for viewer/operator/admin/daemon | `L4-T-sqlite_schema-022`, `L4-T-redaction_boundary-001`-`009` |
| `0005_sync_rpcs.sql` | `apply_push_batch` and `update_operator_decision` | `L4-T-sqlite_supabase_sync-014`-`019`, `030`-`038` |
| `0006_schema_metadata.sql` | `schema_metadata` and `active_writer_lease` | `L4-T-sqlite_schema-023`, writer-lease tests |
| `0007_command_ledger.sql` | `operator_decision_commands` | `L4-T-sqlite_supabase_sync-034`-`038` |
| `0008_decision_summaries.sql` | `operator_decision_summaries` projection | `L4-T-redaction_boundary-006`, `015` |

### 6.3 Supabase Config Skeleton

`infra/supabase/config.toml` minimum:

```toml
project_id = "procurement-intelligence-l4"

[api]
enabled = true
port = 54321
schemas = ["public", "realtime"]
extra_search_path = ["public", "extensions"]
max_rows = 1000

[db]
port = 54322
major_version = 15

[auth]
enabled = true
site_url = "http://127.0.0.1:3000"
additional_redirect_urls = ["http://127.0.0.1:3000"]
jwt_expiry = 3600

[realtime]
enabled = true
```

### 6.4 Schema Coordination Gate

The schema hash gate must:

1. Apply SQLite migrations through `0012_l4_sync_state.sql` from scratch.
2. Apply Supabase migrations `0001` through `0008` from scratch.
3. Introspect logical projection table names, columns, nullability, checks where portable, and sync tracking fields.
4. Fail if the SQLite logical source and Supabase projection target hash differ.

## 7. Skill Directory Scaffold

Canonical overlay paths follow the L4 spec:

```text
skills/validation/
  overlay-procurement-ingestion/
  overlay-supplier-enrichment/
  overlay-document-acquisition/
  overlay-product-extraction/
```

The short slug names are `procurement-ingestion`, `supplier-enrichment`, `document-acquisition`, and `product-extraction`; on disk each uses the `overlay-` prefix required by the spec.

Each overlay directory must contain the same six required files as `skills/validation/base/`, plus the spec-required `references/` directory:

```text
SKILL.md
result.schema.json
checkpoint.schema.json
canary.inputs.jsonl
golden.assertions.yaml
validator-rubric.yaml
references/
```

Every `SKILL.md` frontmatter must include:

```yaml
skill_id: validation/overlay-procurement-ingestion
skill_version: 2026.05.06-1
interaction_profile: headless_contract
result_schema_version: 1.0
checkpoint_schema_version: 1.0
validator_overlay_version: 2026.05.06-1
validator_overlay_compatible_versions:
  worker_behavior_versions:
    - procurement-ingestion/*@2026.05.06-1+1.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: read_only
promotion_stage: candidate
```

Domain checks by overlay:

| Overlay | Required Golden Assertions |
|---------|----------------------------|
| `overlay-procurement-ingestion` | notice completeness, OCDS field presence, source locale, notice-type consistency, expiry plausibility, CPV validity |
| `overlay-supplier-enrichment` | price plausibility, currency correctness, pack basis, seller diversity, listing freshness, regulated-goods compliance evidence |
| `overlay-document-acquisition` | document type, download completeness, content type, file size plausibility, document-set hash consistency |
| `overlay-product-extraction` | CPV-family candidate plausibility, archetype assignment, slot completeness, quantity/unit plausibility, bundle decomposition |

`canary.inputs.jsonl` must include:

- one clean-real approval case
- one weird-valid case that must not be rejected solely for unusual shape
- one mutated-bad rejection case
- one pairwise clean/corrupted pair when the overlay owns any mutation family

`golden.assertions.yaml` must include all mutation families relevant to the overlay and at least one assertion per L4 section 3.6 family across the full overlay set.

## 8. Test Workspace Layout

```text
crates/daemon/tests/
  support/
    db.rs                         # extend existing TestStateDb with L4 helpers
    supabase.rs                   # local Supabase CLI stack and role sessions
    sync.rs                       # outbox, batch, pull, CAS, lease helpers
    redaction.rs                  # sentinel fixtures
    calibration.rs                # corpora, mutations, Wilson/rule-of-three helpers
    validation_enforcement.rs     # provisional promotion harness
  sqlite_schema.rs
  sqlite_supabase_sync.rs
  redaction_boundary.rs

apps/web/
  package.json
  tsconfig.json
  next.config.ts
  playwright.config.ts
  e2e/
    contracts.spec.ts
    fixtures/
      supabase-auth.ts
      projection-seed.ts
  src/
    app/
      layout.tsx
      page.tsx
      contracts/page.tsx
      contracts/[id]/page.tsx
      status/page.tsx
    lib/
      supabase.ts
      broadcast.ts

tests/fixtures/calibration/
  {slice_id}/
    clean/
    weird/
    mutated/
    pairwise/
```

The L4 TDD planned file inventory remains exactly four test files. Support modules and fixtures exist to execute those files without diluting the declared test surfaces.

## 9. Shared Support Extensions

### 9.1 `crates/daemon/tests/support/supabase.rs`

Must provide:

- `SupabaseLocalStack::start_or_reuse()`
- `SupabaseLocalStack::apply_migrations()`
- `SupabaseLocalStack::reset_schema()`
- `SupabaseLocalStack::api_url()`
- `SupabaseLocalStack::db_url()`
- `SupabaseLocalStack::anon_key()`
- `SupabaseLocalStack::service_role_key()`
- `SupabaseLocalStack::viewer_client()`
- `SupabaseLocalStack::operator_client()`
- `SupabaseLocalStack::admin_session()`
- `SupabaseLocalStack::daemon_client()`
- `assert_rls_denied(result, table, role)`

### 9.2 `crates/daemon/tests/support/sync.rs`

Must provide:

- `OutboxHarness::mutate_push_owned_row(...)`
- `OutboxHarness::assemble_next_batch(stream)`
- `OutboxHarness::persist_in_flight_batch(...)`
- `OutboxHarness::retry_persisted_batch(...)`
- `OutboxHarness::assert_identical_envelope(...)`
- `SupabaseRpcClient::apply_push_batch(batch, writer_epoch)`
- `SupabaseRpcClient::update_operator_decision(process_id, disposition, base_revision, command_id)`
- `PullCursorHarness::pull_domain_with_hiwater(domain, page_size)`
- `CasCommandHarness::seed_intent(...)`
- `WriterLeaseHarness::acquire(executor_id)`
- `WriterLeaseHarness::heartbeat(executor_id)`

### 9.3 `crates/daemon/tests/support/redaction.rs`

Must provide:

- `RedactionSentinelFixture::seed_all_sensitive_values()`
- `assert_summary_absent(table, sentinel)`
- `assert_viewer_can_read(table)`
- `assert_viewer_cannot_read(table)`
- `assert_operator_cannot_direct_update(table)`
- `assert_daemon_can_pull_tombstone(table)`

Sensitive sentinels must include operator identity, company registration, VAT number, address/contact JSON, raw supplier URL, tender document metadata, full error markdown, validator reasoning, command id, local DB path, service key, and raw job payload.

### 9.4 `crates/daemon/tests/support/calibration.rs`

Must provide:

- `CalibrationHarness::seed_clean(slice_id, count)`
- `CalibrationHarness::seed_weird(slice_id, count)`
- `CalibrationHarness::seed_mutated(slice_id, family, count)`
- `CalibrationHarness::seed_pairwise(slice_id, pair_id)`
- `CalibrationHarness::wilson_lower_bound_bps(correct, total)`
- `CalibrationHarness::rule_of_three_false_pass_upper_bound_bps(misses, audited)`
- `CalibrationHarness::evaluate_readiness(slice_id)`
- `CalibrationHarness::trigger_override_spike(slice_id)`
- `CalibrationHarness::trigger_challenge_regression(slice_id)`
- `CalibrationHarness::trigger_anomaly_error_rise(slice_id)`
- `CalibrationHarness::ingest_override(verdict_id)`

### 9.5 `apps/web/e2e/fixtures`

Must provide:

- `signInViewer(page)`
- `seedProjectionRows(client, orgId)`
- `seedViewerSyncStatus(client, state)`
- `emitBroadcast(topic, payload)`
- `expectRlsDenied(response)`
- `expectNoMutationControls(page)`
- `expectNoServiceKeyInBrowser(page)`

## 10. Fixture And Corpus Bootstrap

Create calibration corpora under:

```text
tests/fixtures/calibration/{slice_id}/
  clean/
  weird/
  mutated/
  pairwise/
```

Minimum per skill family:

| Corpus | Count | Required Content |
|--------|-------|------------------|
| `clean` | `25-30` | successful real outputs with evidence artifacts |
| `weird` | `15-20` | layout change, localization, partial honest data, low-result-count valid examples |
| `mutated` | `30-50` | realistic failures derived from clean runs |
| `pairwise` | at least one per dominant failure family | clean run plus one-corruption sibling sharing `pair_id` |

Mutation families:

| Family | Fixture Name Prefix |
|--------|---------------------|
| wrong source/domain in URL chain | `url-chain-swap` |
| wrong page kind | `page-kind-confusion` |
| locale/currency swap | `locale-currency-swap` |
| pack/unit distortion | `pack-unit-distortion` |
| accessory-vs-primary confusion | `accessory-primary-confusion` |
| selective warning omission | `selective-omission` |
| manifest/result count mismatch | `count-mismatch` |
| fabricated evidence references | `fabricated-evidence` |
| cross-segment duplicates/gaps | `cross-segment-duplicates` |

Each corpus case must include:

```text
input.json
expected.json
evidence/
manifest.md
metadata.json
```

`metadata.json` required fields:

```json
{
  "case_id": "string",
  "slice_id": "string",
  "corpus_kind": "clean | weird | mutated | pairwise",
  "expected_verdict": "approved | rejected | flagged",
  "ground_truth_label": "valid | invalid",
  "mutation_family": "string or null",
  "pair_id": "string or null",
  "model_family": "string",
  "source_artifact_path": "string"
}
```

## 11. Cron Configuration Seeding

L4 adds two scheduler-owned calibration entries and one runtime interval owned by sync:

| Entry | Type | Cadence | Purpose |
|-------|------|---------|---------|
| `validation-calibration-readiness-sweep` | cron job | daily | recompute per-slice readiness and demotion state from durable calibration evidence |
| `validation-cross-model-audit-sample` | cron job | every 7 days | sample enforcement-active slices with new traffic through an alternate model family; guarantees the at-least-once-per-30-days requirement |
| `active-writer-lease-heartbeat` | sync runtime interval | every 30 seconds while sync is active | extend `lease_expires_at` to at least 120 seconds and update `viewer_sync_status` |

Seed example:

```json
[
  {
    "source_id": "validation-calibration",
    "skill_id": "validation/calibration-readiness-sweep",
    "skill_version": "2026.05.06-1",
    "cron_expression": "0 20 2 * * *",
    "enabled": true
  },
  {
    "source_id": "validation-calibration",
    "skill_id": "validation/cross-model-audit-sample",
    "skill_version": "2026.05.06-1",
    "cron_expression": "0 30 3 * * 1",
    "enabled": true
  }
]
```

The lease heartbeat is not seeded as a cron row because it is coupled to active sync lease ownership and must stop immediately when sync is inactive.

## 12. Command Matrix

| Lane | Command |
|------|---------|
| Static | `cargo fmt --check` |
| Static | `cargo check --all-targets` |
| Static | `cargo clippy --all-targets -- -D warnings` |
| Static | `bun run lint` |
| Static | `bun run test` |
| Static | `ubs docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md` |
| Stage Gate | `cargo test -p daemon --bin daemon` |
| Stage Gate | `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done` |
| SQLite schema | `cargo test -p daemon --test sqlite_schema -- --nocapture` |
| Sync/RPC | `cargo test -p daemon --test sqlite_supabase_sync -- --nocapture` |
| RLS/redaction | `cargo test -p daemon --test redaction_boundary -- --nocapture` |
| Web type/test | `(cd apps/web && bun run test)` |
| Web E2E | `(cd apps/web && bun run test:e2e)` |
| Supabase reset | `supabase db reset --local` |
| Supabase migration | `supabase migration up --local` |

Full L4 validation command:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
bun run lint
bun run test
(cd apps/web && bun run test:e2e)
```

## 13. Artifact Checklist

Before the full L4 suite can pass, these artifacts must exist:

- `migrations/0012_l4_sync_state.sql`
- `infra/supabase/config.toml`
- `infra/supabase/migrations/0001_projection_tables.sql`
- `infra/supabase/migrations/0002_remote_owned_tables.sql`
- `infra/supabase/migrations/0003_summary_tables.sql`
- `infra/supabase/migrations/0004_rls_policies.sql`
- `infra/supabase/migrations/0005_sync_rpcs.sql`
- `infra/supabase/migrations/0006_schema_metadata.sql`
- `infra/supabase/migrations/0007_command_ledger.sql`
- `infra/supabase/migrations/0008_decision_summaries.sql`
- `crates/daemon/src/db/sync.rs`
- `crates/daemon/src/validation/calibration.rs`
- `crates/daemon/tests/sqlite_schema.rs`
- `crates/daemon/tests/sqlite_supabase_sync.rs`
- `crates/daemon/tests/redaction_boundary.rs`
- `apps/web/package.json`
- `apps/web/tsconfig.json`
- `apps/web/playwright.config.ts`
- the seven `apps/web/src/` files declared in the spec
- four overlay skill directories with required files
- calibration corpora under `tests/fixtures/calibration/`
- Playwright viewer auth fixtures
- Supabase seed data for viewer/operator/admin/daemon role checks

## 14. Structured Logging Requirements

Every L4 Rust test emits:

- `[TEST_START] <test-id>`
- `[SETUP] <fixture names>`
- `[EXEC] <operation>`
- `[ASSERT] <specific assertion>`
- `[CLEANUP] <resource>`
- `[TEST_END] <test-id> PASS|FAIL`

Sync tests additionally log:

- `executor_id`
- `writer_epoch`
- `schema_epoch`
- `stage_run_id`
- `commit_group_id`
- `local_seq` range
- `sync_batch_id`
- `content_hash`
- RPC response status/body classification

CAS tests additionally log:

- `process_id`
- `command_id`
- `base_revision`
- `applied_revision`
- ledger result
- local intent status

Calibration tests additionally log:

- `slice_id`
- worker behavior version
- validator overlay version
- corpus kind
- mutation family
- pair id
- Wilson lower bound bps
- false-pass upper bound bps
- demotion reason

Playwright tests additionally capture:

- trace on failure
- screenshot on failure
- network request/response log for RLS checks
- Broadcast payload log for invalidation tests

## 15. False Positive Prevention

| Risk | Prevention |
|------|------------|
| Outbox tests pass without atomicity | assert data row and outbox row inside the same transaction and rollback both on injected failure |
| Retry tests rebuild envelopes | persist `sync_batches` payload and compare row set, order, hashes, epoch, stage run, commit groups, writer epoch byte-for-byte |
| RPC apply tests pass with partial writes | inject one invalid row and assert every target table remains unchanged |
| Equal-version tests miss corruption | test matching hash no-op and different hash hard conflict as separate IDs |
| Pull tests miss pagination races | write `remote_change_seq > hiwater` between pages and assert next pull sees it exactly once |
| CAS replay re-evaluates current state | apply command 1, apply command 2, retry command 1, assert stored command 1 response returns |
| RLS tests only check UI hiding | use role-scoped API/DB requests and assert network/database denial |
| Redaction tests miss renamed leaks | seed sentinel values and scan all web-visible summary fields for each sentinel |
| Web viewer accidentally has privileged keys | inspect runtime env and browser bundle for service/admin/operator/daemon tokens |
| Calibration precision passes from sample-size gaps | assert >=30 adjudicated rejected/flagged and >=60 audited approvals before bps thresholds are considered |
| Pairwise challenge sets are ignored | require clean/corrupted siblings with shared `pair_id` and feed pairwise failures into readiness/demotion |
| Cross-model audit becomes cosmetic | enforce weekly sampling for active slices with new traffic and at-least-once-per-30-days assertion in test data |
| Override rate becomes sole estimator | override rate may demote but promotion still requires adjudicated rejects, audited approvals, Wilson bound, rule-of-three, and challenge coverage |
| Auto-demotion thresholds drift weaker | store threshold config snapshot in `enforcement_readiness` and assert default lower bounds are not weakened |
| `packages/ui` gains data fetching | root lint fails on Supabase, Broadcast, Tauri, daemon, desktop, or transport imports |

Default demotion thresholds:

- rolling 7-day window has at least 10 operator-reviewed enforcement rejections and `operator_override_rate_bps >= 1500`, or at least 2x trailing 30-day baseline
- any critical mutated-bad assertion is approved
- challenge-bank pass rate drops below 95%
- anomaly-triggered validation has at least 20 samples and validator error rate exceeds 5% or 2x trailing 30-day baseline
- worker behavior version exits overlay compatible range

## 16. Notes For Implementation

- Keep the four L4 test files intact: `sqlite_schema.rs`, `sqlite_supabase_sync.rs`, `redaction_boundary.rs`, and `apps/web/e2e/contracts.spec.ts`.
- AgentMail project key for coordination is `/Users/alinaaitova/Desktop/Testproject1`.
- Extend `crates/daemon/src/db/sqlite.rs` through the existing migration registry and PRAGMA-backed schema epoch path.
- Add `crates/daemon/src/db/sync.rs` as the sync engine module; keep `sqlite.rs` as the single connection wrapper.
- Extend `validation/policy.rs`, `validation/executor.rs`, `jobs/runner.rs`, and `ipc/handlers/decisions.rs` in place. Do not fork L3 validation or decision capture contracts.
- Keep `AGENT_SUSPECT_OUTPUT` wired through the existing error manifest surfaces and the TypeScript renderer.
- Use `br` terminology in implementation handoffs and beads. Sync beads with `bin/br-sync` or `br sync --flush-only`; do not use raw `br sync`.
- Apply all Supabase migrations through the local CLI stack before running Rust RLS/RPC tests.
- Playwright web tests must authenticate as viewer and must fail any attempt to use privileged browser credentials.
- The Stage Validation Test Gate is mandatory for any manifest that claims L4 green.
