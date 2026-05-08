#![forbid(unsafe_code)]

mod support;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, ensure};

use support::db::{
    LogicalColumn, LogicalSchema, TestStateDb, logical_schema_for_tables, logical_schema_hash,
    normalize_sql_type,
};
use support::supabase::SupabaseLocalStack;

const PUSH_OWNED_TABLES: &[&str] = &[
    "procurement_processes",
    "contract_notices",
    "notice_lots",
    "tender_document_sets",
    "qualification_runs",
    "product_candidates",
    "search_profiles",
    "search_attempts",
    "supplier_quotes",
    "candidate_pricing_assessments",
    "landed_cost_estimates",
    "manifests",
    "pipeline_assessments",
    "bid_workflow_states",
    "process_stage_status",
    "process_stage_heads",
    "process_relations",
    "process_aliases",
    "document_extractions",
    "document_semantic_extractions",
];

const LOCAL_ORIGIN_SUMMARY_TABLES: &[&str] = &[
    "workflow_job_summaries",
    "error_manifest_summaries",
    "validation_verdict_summaries",
    "viewer_sync_status",
    "executors",
];

const SYNC_TRACKING_COLUMNS: &[&str] = &[
    "owner_id",
    "row_version",
    "last_local_seq",
    "stage_run_id",
    "created_at",
    "updated_at",
];

#[test]
fn l4_t_redaction_boundary_026_schema_hash_coordination_gate() -> Result<()> {
    let sqlite = TestStateDb::new()?;
    let sqlite_schema = logical_schema_for_tables(sqlite.connection(), PUSH_OWNED_TABLES)?;
    assert_push_owned_tracking_fields("SQLite", &sqlite_schema)?;

    let supabase = SupabaseLocalStack::start_or_reuse()?;
    let applied = supabase.apply_migrations()?;
    ensure!(
        applied.paths.len() == 8,
        "L4-T-redaction_boundary-026 expected all eight Supabase migrations to apply"
    );
    let supabase_schema = supabase.logical_schema_from_migrations(PUSH_OWNED_TABLES)?;
    assert_push_owned_tracking_fields("Supabase", &supabase_schema)?;

    let sqlite_hash = logical_schema_hash(&sqlite_schema);
    let supabase_hash = logical_schema_hash(&supabase_schema);
    ensure!(
        sqlite_schema == supabase_schema,
        "L4-T-redaction_boundary-026 logical schema mismatch\nSQLite hash: {sqlite_hash}\nSupabase hash: {supabase_hash}\n{}",
        schema_diff(&sqlite_schema, &supabase_schema)
    );

    let summary_schema = supabase.logical_schema_from_migrations(LOCAL_ORIGIN_SUMMARY_TABLES)?;
    let expected_summary_schema = expected_local_origin_summary_schema();
    let summary_hash = logical_schema_hash(&summary_schema);
    let expected_summary_hash = logical_schema_hash(&expected_summary_schema);
    ensure!(
        summary_schema == expected_summary_schema,
        "L4-T-redaction_boundary-026 summary schema mismatch\nExpected hash: {expected_summary_hash}\nSupabase hash: {summary_hash}\n{}",
        schema_diff(&expected_summary_schema, &summary_schema)
    );

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_023_daemon_projection_mutation_stays_behind_rpc() -> Result<()> {
    let rls = read_repo_file("infra/supabase/migrations/0004_rls_policies.sql")?;
    let rpc = read_repo_file("infra/supabase/migrations/0005_sync_rpcs.sql")?;
    let apply_push_batch_rpc = rpc
        .split("CREATE OR REPLACE FUNCTION update_operator_decision")
        .next()
        .context("L4-T-redaction_boundary-023 missing apply_push_batch section")?;

    ensure!(
        apply_push_batch_rpc.contains(
            "CREATE OR REPLACE FUNCTION apply_push_batch(batch_json jsonb, writer_epoch integer)"
        ) && apply_push_batch_rpc.contains("SECURITY DEFINER"),
        "L4-T-redaction_boundary-023 expected apply_push_batch security-definer RPC"
    );
    ensure!(
        apply_push_batch_rpc.contains("l4_is_any_role(ARRAY['daemon', 'admin'])")
            && apply_push_batch_rpc.contains("PERFORM l4_apply_push_owned_payload"),
        "L4-T-redaction_boundary-023 expected daemon/admin RPC gate and internal apply helper"
    );
    ensure!(
        !apply_push_batch_rpc.contains("'not_implemented'"),
        "L4-T-redaction_boundary-023 found placeholder apply_push_batch body"
    );

    ensure!(
        rls.contains("FOR SELECT USING (l4_is_any_role(ARRAY[''viewer'',''operator'',''admin'']))"),
        "push-owned projection tables should expose SELECT policies only to viewer/operator/admin"
    );
    ensure!(
        !rls.contains("procurement_processes_daemon_insert")
            && !rls.contains("procurement_processes_daemon_update"),
        "push-owned projection tables must not expose direct daemon INSERT/UPDATE policies"
    );
    ensure!(
        rls.contains(
            "GRANT INSERT, UPDATE, DELETE ON shared_company_profile, shared_source_configs, operator_decision_heads TO authenticated;"
        ),
        "authenticated mutation grant should remain limited to remote-owned/CAS tables"
    );
    ensure!(
        !rls.contains("GRANT INSERT, UPDATE, DELETE ON procurement_processes TO authenticated")
            && !rls.contains(
                "GRANT INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO authenticated"
            ),
        "daemon/operator direct projection table grants must stay absent outside apply_push_batch"
    );

    Ok(())
}

fn assert_push_owned_tracking_fields(source: &str, schema: &LogicalSchema) -> Result<()> {
    ensure!(
        schema.len() == PUSH_OWNED_TABLES.len(),
        "{source} schema covered {} push-owned tables; expected {}",
        schema.len(),
        PUSH_OWNED_TABLES.len()
    );

    for table in PUSH_OWNED_TABLES {
        let columns = schema
            .get(*table)
            .ok_or_else(|| anyhow::anyhow!("{source} schema missing table `{table}`"))?;
        for column in SYNC_TRACKING_COLUMNS {
            ensure!(
                columns.iter().any(|candidate| candidate.name == *column),
                "{source} table `{table}` missing sync tracking column `{column}`"
            );
        }
    }

    Ok(())
}

fn expected_local_origin_summary_schema() -> LogicalSchema {
    let mut schema = LogicalSchema::default();
    schema.insert(
        "workflow_job_summaries".to_owned(),
        columns(&[
            ("job_id", "text", false),
            ("org_id", "text", false),
            ("job_kind", "text", false),
            ("status", "text", false),
            ("skill_path", "text", true),
            ("started_at", "timestamptz", true),
            ("completed_at", "timestamptz", true),
            ("error_type", "text", true),
            ("owner_id", "text", false),
            ("row_version", "bigint", false),
            ("last_local_seq", "bigint", true),
            ("stage_run_id", "text", true),
            ("updated_at", "timestamptz", false),
            ("created_at", "timestamptz", false),
        ]),
    );
    schema.insert(
        "error_manifest_summaries".to_owned(),
        columns(&[
            ("manifest_id", "text", false),
            ("org_id", "text", false),
            ("job_id", "text", true),
            ("error_type", "text", false),
            ("error_code", "text", true),
            ("incident_signature", "text", true),
            ("owner_id", "text", false),
            ("row_version", "bigint", false),
            ("last_local_seq", "bigint", true),
            ("stage_run_id", "text", true),
            ("updated_at", "timestamptz", false),
            ("created_at", "timestamptz", false),
        ]),
    );
    schema.insert(
        "validation_verdict_summaries".to_owned(),
        columns(&[
            ("verdict_id", "text", false),
            ("org_id", "text", false),
            ("worker_job_id", "text", false),
            ("verdict", "text", false),
            ("basis_sufficiency", "text", true),
            ("enforcement_eligible", "boolean", false),
            ("owner_id", "text", false),
            ("row_version", "bigint", false),
            ("last_local_seq", "bigint", true),
            ("stage_run_id", "text", true),
            ("updated_at", "timestamptz", false),
            ("created_at", "timestamptz", false),
        ]),
    );
    schema.insert(
        "viewer_sync_status".to_owned(),
        columns(&[
            ("org_id", "text", false),
            ("sync_health", "text", false),
            ("last_push_success_at", "timestamptz", true),
            ("last_push_attempt_at", "timestamptz", true),
            ("last_acked_local_seq", "bigint", true),
            ("pending_outbox_count", "integer", false),
            ("schema_epoch", "integer", false),
            ("projection_generation", "bigint", false),
            ("daemon_version", "text", true),
            ("updated_at", "timestamptz", false),
        ]),
    );
    schema.insert(
        "executors".to_owned(),
        columns(&[
            ("executor_id", "text", false),
            ("org_id", "text", false),
            ("heartbeat_at", "timestamptz", false),
            ("daemon_version", "text", true),
            ("started_at", "timestamptz", false),
            ("owner_id", "text", false),
            ("row_version", "bigint", false),
            ("last_local_seq", "bigint", true),
            ("stage_run_id", "text", true),
            ("updated_at", "timestamptz", false),
            ("created_at", "timestamptz", false),
        ]),
    );

    schema
}

fn columns(definitions: &[(&str, &str, bool)]) -> Vec<LogicalColumn> {
    let mut columns = definitions
        .iter()
        .map(|(name, raw_type, nullable)| LogicalColumn {
            name: (*name).to_owned(),
            logical_type: normalize_sql_type(raw_type, name),
            nullable: *nullable,
        })
        .collect::<Vec<_>>();
    columns.sort_by(|left, right| left.name.cmp(&right.name));
    columns
}

fn schema_diff(expected: &LogicalSchema, actual: &LogicalSchema) -> String {
    let mut output = String::new();
    for table in expected.keys().chain(actual.keys()) {
        let expected_columns = expected.get(table);
        let actual_columns = actual.get(table);
        match (expected_columns, actual_columns) {
            (Some(_), None) => {
                let _ = writeln!(output, "missing actual table `{table}`");
            }
            (None, Some(_)) => {
                let _ = writeln!(output, "unexpected actual table `{table}`");
            }
            (Some(expected_columns), Some(actual_columns)) => {
                append_column_diff(&mut output, table, expected_columns, actual_columns);
            }
            (None, None) => {}
        }
    }
    if output.is_empty() {
        "schema hashes differed but no structural diff was rendered\n".to_owned()
    } else {
        output
    }
}

fn append_column_diff(
    output: &mut String,
    table: &str,
    expected_columns: &[LogicalColumn],
    actual_columns: &[LogicalColumn],
) {
    let expected_by_name = expected_columns
        .iter()
        .map(|column| (&column.name, column))
        .collect::<BTreeMap<_, _>>();
    let actual_by_name = actual_columns
        .iter()
        .map(|column| (&column.name, column))
        .collect::<BTreeMap<_, _>>();

    for name in expected_by_name.keys().chain(actual_by_name.keys()) {
        match (expected_by_name.get(name), actual_by_name.get(name)) {
            (Some(_), None) => {
                let _ = writeln!(output, "table `{table}` missing column `{name}`");
            }
            (None, Some(_)) => {
                let _ = writeln!(output, "table `{table}` has unexpected column `{name}`");
            }
            (Some(expected), Some(actual)) if expected != actual => {
                let _ = writeln!(
                    output,
                    "table `{table}` column `{name}` differs: expected {expected:?}, actual {actual:?}"
                );
            }
            _ => {}
        }
    }
}

fn repo_root() -> Result<&'static Path> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .context("daemon crate should live two levels below the repository root")
}

fn read_repo_file(path: &str) -> Result<String> {
    std::fs::read_to_string(repo_root()?.join(path))
        .with_context(|| format!("reading repository file {path}"))
}
