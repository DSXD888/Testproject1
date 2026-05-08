#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

#[path = "../src/validation/provisional.rs"]
#[allow(dead_code)]
mod provisional_impl;
use provisional_impl::{
    InsertProvisionalOutput, ProvisionalValidationStatus, ProvisionalWorkerOutputStore,
};
use support::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb, StateDb};
use support::supabase::{SupabaseLocalStack, logical_schema_from_sql};

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

const JUNCTION_TABLES: &[&str] = &[
    "notice_cpv_codes",
    "product_commodity_codes",
    "lot_cpv_codes",
];

const SUMMARY_TABLES: &[&str] = &[
    "workflow_job_summaries",
    "error_manifest_summaries",
    "validation_verdict_summaries",
    "viewer_sync_status",
    "executors",
    "operator_decision_summaries",
];

const REMOTE_OWNED_TABLES: &[&str] = &[
    "shared_company_profile",
    "shared_source_configs",
    "operator_decision_heads",
];

const SQLITE_SYNC_TABLES: &[&str] = &[
    "local_outbox",
    "sync_batches",
    "pull_cursors",
    "provisional_worker_outputs",
    "enforcement_readiness",
    "projection_generations",
    "validation_calibration_cases",
    "validation_incident_clusters",
];

const SYNC_TRACKING_COLUMNS: &[&str] = &[
    "owner_id",
    "row_version",
    "last_local_seq",
    "stage_run_id",
    "created_at",
    "updated_at",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct SqliteColumn {
    raw_type: String,
    not_null: bool,
    primary_key_position: i64,
}

#[test]
#[doc = "L4-T-sqlite_schema-001"]
fn l4_t_sqlite_schema_001_open_state_db_applies_l4_migration() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();

    let user_version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    ensure!(
        user_version == DEFAULT_SCHEMA_EPOCH,
        "expected PRAGMA user_version {DEFAULT_SCHEMA_EPOCH}, got {user_version}"
    );

    let applied: Option<String> = conn
        .query_row(
            "SELECT id FROM schema_migrations WHERE id = '0012_l4_sync_state'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    ensure!(
        applied.as_deref() == Some("0012_l4_sync_state"),
        "schema_migrations did not record 0012_l4_sync_state"
    );

    for table in SQLITE_SYNC_TABLES {
        ensure!(
            table_exists(conn, table)?,
            "missing L4 SQLite table `{table}`"
        );
    }
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-002"]
fn l4_t_sqlite_schema_002_local_outbox_shape_and_monotonic_seq() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    let columns = sqlite_table_columns(conn, "local_outbox")?;

    ensure_columns(
        "local_outbox",
        &columns,
        &[
            "local_seq",
            "executor_id",
            "entity_kind",
            "entity_id",
            "op",
            "row_version",
            "stage_run_id",
            "schema_epoch",
            "commit_group_id",
            "writer_epoch",
            "payload_json",
            "content_hash",
            "created_at",
        ],
    )?;
    ensure!(
        columns
            .get("local_seq")
            .is_some_and(|column| column.primary_key_position > 0),
        "local_outbox.local_seq must be the monotonic primary key"
    );
    ensure_column_nullable("local_outbox", &columns, "writer_epoch", true)?;

    insert_outbox_row(
        conn,
        "insert",
        Some("l4-stage-run-1"),
        Some(DEFAULT_SCHEMA_EPOCH),
        Some("l4-commit-group-1"),
        "hash-local-outbox-1",
    )?;
    insert_outbox_row(
        conn,
        "update",
        Some("l4-stage-run-1"),
        Some(DEFAULT_SCHEMA_EPOCH),
        Some("l4-commit-group-2"),
        "hash-local-outbox-2",
    )?;

    let seqs = query_i64s(
        conn,
        "SELECT local_seq FROM local_outbox ORDER BY local_seq",
    )?;
    ensure!(
        seqs.len() == 2 && seqs[0] < seqs[1],
        "local_outbox.local_seq did not advance monotonically: {seqs:?}"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-003"]
fn l4_t_sqlite_schema_003_sync_batches_envelope_shape() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    let columns = sqlite_table_columns(conn, "sync_batches")?;

    ensure_columns(
        "sync_batches",
        &columns,
        &[
            "batch_id",
            "executor_id",
            "stream_id",
            "status",
            "first_local_seq",
            "last_local_seq",
            "schema_epoch",
            "stage_run_id",
            "commit_group_ids_json",
            "writer_epoch",
            "payload_json",
            "payload_hash",
            "retry_count",
            "last_attempt_at",
            "acknowledged_at",
            "rejected_at",
            "response_json",
            "created_at",
        ],
    )?;
    ensure_column_type_contains("sync_batches", &columns, "writer_epoch", "INT")?;
    ensure_column_type_contains("sync_batches", &columns, "schema_epoch", "INT")?;
    ensure_column_type_contains("sync_batches", &columns, "first_local_seq", "INT")?;
    ensure_column_type_contains("sync_batches", &columns, "last_local_seq", "INT")?;

    conn.execute(
        "\
        INSERT INTO sync_batches (
          batch_id, executor_id, stream_id, status, first_local_seq, last_local_seq,
          schema_epoch, stage_run_id, commit_group_ids_json, writer_epoch, payload_json,
          payload_hash
        ) VALUES (
          'l4-batch-1', 'executor-1', 'push-owned', 'in_flight', 1, 2,
          ?1, 'stage-run-1', '[\"commit-1\"]', 7, '{\"rows\":[]}', 'payload-hash-1'
        )
        ",
        [DEFAULT_SCHEMA_EPOCH],
    )?;

    let invalid_status = conn.execute(
        "\
        INSERT INTO sync_batches (
          batch_id, executor_id, stream_id, status, first_local_seq, last_local_seq,
          schema_epoch, stage_run_id, commit_group_ids_json, writer_epoch, payload_json,
          payload_hash
        ) VALUES (
          'l4-batch-invalid', 'executor-1', 'push-owned', 'done', 1, 2,
          ?1, 'stage-run-1', '[\"commit-1\"]', 7, '{\"rows\":[]}', 'payload-hash-2'
        )
        ",
        [DEFAULT_SCHEMA_EPOCH],
    );
    ensure!(
        invalid_status.is_err(),
        "sync_batches.status accepted a value outside the L4 retry envelope states"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-004"]
fn l4_t_sqlite_schema_004_pull_cursors_remote_owned_domains() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    let columns = sqlite_table_columns(conn, "pull_cursors")?;

    ensure_columns(
        "pull_cursors",
        &columns,
        &[
            "domain",
            "last_remote_change_seq",
            "last_pulled_at",
            "updated_at",
        ],
    )?;
    ensure!(
        columns
            .get("domain")
            .is_some_and(|column| column.primary_key_position > 0),
        "pull_cursors.domain must be the independent per-domain primary key"
    );

    let domains = query_strings(
        conn,
        "SELECT domain FROM pull_cursors ORDER BY domain COLLATE BINARY",
    )?;
    ensure!(
        domains
            == vec![
                "operator_decision_heads".to_owned(),
                "shared_company_profile".to_owned(),
                "shared_source_configs".to_owned()
            ],
        "pull_cursors seeded unexpected domains: {domains:?}"
    );
    let cursors = query_i64s(
        conn,
        "SELECT last_remote_change_seq FROM pull_cursors ORDER BY domain",
    )?;
    ensure!(
        cursors == vec![0, 0, 0],
        "pull_cursors must start each remote-owned domain at seq 0: {cursors:?}"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-005"]
fn l4_t_sqlite_schema_005_provisional_worker_outputs_shape() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    let columns = sqlite_table_columns(conn, "provisional_worker_outputs")?;

    ensure_columns(
        "provisional_worker_outputs",
        &columns,
        &[
            "id",
            "worker_job_id",
            "payload_json",
            "result_contract_blob_path",
            "validation_status",
            "created_at",
            "updated_at",
        ],
    )?;
    ensure!(
        index_names(conn, "provisional_worker_outputs")?
            .contains("idx_provisional_worker_outputs_worker_job"),
        "provisional_worker_outputs.worker_job_id index missing"
    );
    let table_sql = sqlite_table_sql(conn, "provisional_worker_outputs")?;
    assert_sql_contains_all(&table_sql, &["'pending'", "'approved'", "'rejected'"])?;

    seed_workflow_job(conn, "l4-provisional-worker")?;
    conn.execute(
        "\
        INSERT INTO provisional_worker_outputs (
          id, worker_job_id, payload_json, validation_status
        ) VALUES ('l4-provisional-1', 'l4-provisional-worker', '{\"ok\":true}', 'pending')
        ",
        [],
    )?;
    let invalid_status = conn.execute(
        "\
        INSERT INTO provisional_worker_outputs (
          id, worker_job_id, payload_json, validation_status
        ) VALUES ('l4-provisional-invalid', 'l4-provisional-worker', '{\"ok\":false}', 'maybe')
        ",
        [],
    );
    ensure!(
        invalid_status.is_err(),
        "provisional_worker_outputs.validation_status accepted an invalid state"
    );

    let mut access_conn = Connection::open(db.db_path())?;
    access_conn.execute_batch("PRAGMA foreign_keys = ON")?;
    let before_outbox_count = count_rows(&access_conn, "local_outbox")?;
    seed_workflow_job(&access_conn, "l4-provisional-access-worker")?;
    seed_workflow_job(&access_conn, "l4-provisional-reject-worker")?;

    let tx = access_conn.transaction()?;
    let inserted = ProvisionalWorkerOutputStore::insert_provisional(
        &tx,
        InsertProvisionalOutput {
            id: "l4-provisional-access-1",
            worker_job_id: "l4-provisional-access-worker",
            payload_json: r#"{"worker_job_id":"l4-provisional-access-worker","ok":true}"#,
            result_contract_blob_path: Some("artifacts/provisional/access-worker.json"),
        },
    )?;
    ensure!(
        inserted.validation_status == ProvisionalValidationStatus::Pending,
        "inserted provisional output was not pending"
    );
    let approved = ProvisionalWorkerOutputStore::mark_approved(&tx, "l4-provisional-access-1")?;
    ensure!(
        approved.validation_status == ProvisionalValidationStatus::Approved,
        "mark_approved did not persist approved status"
    );

    let inserted_for_reject = ProvisionalWorkerOutputStore::insert_provisional(
        &tx,
        InsertProvisionalOutput {
            id: "l4-provisional-reject-1",
            worker_job_id: "l4-provisional-reject-worker",
            payload_json: r#"{"worker_job_id":"l4-provisional-reject-worker","ok":false}"#,
            result_contract_blob_path: None,
        },
    )?;
    ensure!(
        inserted_for_reject.validation_status == ProvisionalValidationStatus::Pending,
        "second inserted provisional output was not pending"
    );
    let rejected = ProvisionalWorkerOutputStore::mark_rejected(&tx, "l4-provisional-reject-1")?;
    ensure!(
        rejected.validation_status == ProvisionalValidationStatus::Rejected,
        "mark_rejected did not persist rejected status"
    );
    tx.commit()?;

    let by_worker = ProvisionalWorkerOutputStore::lookup_by_worker_job_id(
        &access_conn,
        "l4-provisional-access-worker",
    )?;
    ensure!(
        by_worker.as_ref().map(|row| row.id.as_str()) == Some("l4-provisional-access-1"),
        "lookup_by_worker_job_id did not return the inserted provisional output"
    );
    ensure!(
        count_rows(&access_conn, "local_outbox")? == before_outbox_count,
        "provisional access layer wrote to local_outbox"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-006"]
fn l4_t_sqlite_schema_006_enforcement_readiness_shape() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    let columns = sqlite_table_columns(conn, "enforcement_readiness")?;

    ensure_columns(
        "enforcement_readiness",
        &columns,
        &[
            "slice_id",
            "worker_skill_id",
            "source_id",
            "worker_behavior_version",
            "validator_overlay_version",
            "enforcement_state",
            "last_evaluated_at",
            "wilson_lower_bound_bps",
            "false_pass_upper_bound_bps",
            "adjudicated_count",
            "audited_approvals_count",
            "dominant_failure_families_json",
            "missing_failure_families_json",
            "demotion_reason",
            "demotion_thresholds_json",
            "active_demotion_trigger_json",
            "created_at",
            "updated_at",
        ],
    )?;
    ensure!(
        columns
            .get("slice_id")
            .is_some_and(|column| column.primary_key_position > 0),
        "enforcement_readiness.slice_id must be the deterministic primary key"
    );
    assert_sql_contains_all(
        &sqlite_table_sql(conn, "enforcement_readiness")?,
        &["'eligible'", "'dry_run'", "'demoted'"],
    )?;

    conn.execute(
        "\
        INSERT INTO enforcement_readiness (
          slice_id, worker_skill_id, source_id, worker_behavior_version,
          validator_overlay_version, enforcement_state, wilson_lower_bound_bps,
          false_pass_upper_bound_bps, adjudicated_count, audited_approvals_count,
          demotion_thresholds_json
        ) VALUES (
          'slice-1', 'worker-skill', 'source-1', 'worker@1', 'overlay@1', 'eligible',
          7100, 400, 30, 60, '{\"wilson\":7000}'
        )
        ",
        [],
    )?;
    let invalid_state = conn.execute(
        "\
        INSERT INTO enforcement_readiness (
          slice_id, worker_skill_id, source_id, worker_behavior_version,
          validator_overlay_version, enforcement_state
        ) VALUES ('slice-invalid', 'worker-skill', 'source-1', 'worker@1', 'overlay@1', 'paused')
        ",
        [],
    );
    ensure!(
        invalid_state.is_err(),
        "enforcement_readiness.enforcement_state accepted an invalid state"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-007"]
fn l4_t_sqlite_schema_007_projection_generations_shape() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    let columns = sqlite_table_columns(conn, "projection_generations")?;

    ensure_columns(
        "projection_generations",
        &columns,
        &[
            "generation_id",
            "schema_epoch",
            "status",
            "started_at",
            "completed_at",
            "max_local_seq_before_reseed",
            "remote_acked_at",
            "error_json",
        ],
    )?;
    ensure_column_type_contains("projection_generations", &columns, "schema_epoch", "INT")?;
    ensure_column_type_contains(
        "projection_generations",
        &columns,
        "max_local_seq_before_reseed",
        "INT",
    )?;
    assert_sql_contains_all(
        &sqlite_table_sql(conn, "projection_generations")?,
        &["'started'", "'published'", "'failed'"],
    )?;

    conn.execute(
        "\
        INSERT INTO projection_generations (
          schema_epoch, status, max_local_seq_before_reseed
        ) VALUES (?1, 'started', 99)
        ",
        [DEFAULT_SCHEMA_EPOCH],
    )?;
    let invalid_status = conn.execute(
        "\
        INSERT INTO projection_generations (
          schema_epoch, status, max_local_seq_before_reseed
        ) VALUES (?1, 'queued', 99)
        ",
        [DEFAULT_SCHEMA_EPOCH],
    );
    ensure!(
        invalid_status.is_err(),
        "projection_generations.status accepted an invalid state"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-008"]
fn l4_t_sqlite_schema_008_validation_calibration_cases_shape() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    let columns = sqlite_table_columns(conn, "validation_calibration_cases")?;

    ensure_columns(
        "validation_calibration_cases",
        &columns,
        &[
            "case_id",
            "slice_id",
            "corpus_kind",
            "source_artifact_path",
            "expected_verdict",
            "actual_verdict",
            "ground_truth_label",
            "mutation_family",
            "pair_id",
            "model_family",
            "worker_job_id",
            "validation_verdict_id",
            "metadata_json",
            "created_at",
            "adjudicated_at",
        ],
    )?;
    assert_sql_contains_all(
        &sqlite_table_sql(conn, "validation_calibration_cases")?,
        &[
            "'clean'",
            "'weird'",
            "'mutated'",
            "'pairwise'",
            "'live'",
            "'override'",
            "'cross_model'",
        ],
    )?;

    conn.execute(
        "\
        INSERT INTO validation_calibration_cases (
          case_id, slice_id, corpus_kind, source_artifact_path, expected_verdict,
          actual_verdict, ground_truth_label, mutation_family, pair_id, model_family
        ) VALUES (
          'case-1', 'slice-1', 'mutated', 'tests/fixtures/calibration/case-1.json',
          'rejected', 'rejected', 'bad', 'source_identity', 'pair-1', 'claude'
        )
        ",
        [],
    )?;
    let invalid_corpus = conn.execute(
        "\
        INSERT INTO validation_calibration_cases (
          case_id, slice_id, corpus_kind
        ) VALUES ('case-invalid', 'slice-1', 'synthetic')
        ",
        [],
    );
    ensure!(
        invalid_corpus.is_err(),
        "validation_calibration_cases.corpus_kind accepted an invalid kind"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-009"]
fn l4_t_sqlite_schema_009_validation_incident_clusters_shape() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    let columns = sqlite_table_columns(conn, "validation_incident_clusters")?;

    ensure_columns(
        "validation_incident_clusters",
        &columns,
        &[
            "incident_signature",
            "dominant_failure_family",
            "affected_slice_ids_json",
            "candidate_mutation_template_json",
            "override_case_ids_json",
            "calibration_case_count",
            "override_count",
            "first_seen_at",
            "last_seen_at",
            "created_at",
            "updated_at",
        ],
    )?;
    conn.execute(
        "\
        INSERT INTO validation_incident_clusters (
          incident_signature, dominant_failure_family, affected_slice_ids_json,
          candidate_mutation_template_json, override_case_ids_json, calibration_case_count,
          override_count
        ) VALUES (
          'incident-1', 'source_identity', '[\"slice-1\"]',
          '{\"mutation\":\"wrong_source\"}', '[\"case-1\"]', 1, 1
        )
        ",
        [],
    )?;
    let affected: String = conn.query_row(
        "SELECT affected_slice_ids_json FROM validation_incident_clusters WHERE incident_signature = 'incident-1'",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        affected.contains("slice-1"),
        "validation_incident_clusters did not store affected slice linkage"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-010"]
fn l4_t_sqlite_schema_010_push_owned_tables_retain_sync_tracking() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    for table in PUSH_OWNED_TABLES {
        let columns = sqlite_table_columns(conn, table)?;
        ensure_columns(table, &columns, SYNC_TRACKING_COLUMNS)?;
    }
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-011"]
fn l4_t_sqlite_schema_011_money_and_bps_columns_are_integer_affinity() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    let mut checked_columns = Vec::new();

    for table in PUSH_OWNED_TABLES {
        for (column_name, column) in sqlite_table_columns(conn, table)? {
            if column_name.ends_with("_pence") || column_name.ends_with("_bps") {
                ensure!(
                    column.raw_type.to_ascii_uppercase().contains("INT"),
                    "{table}.{column_name} must use INTEGER affinity, got `{}`",
                    column.raw_type
                );
                checked_columns.push(format!("{table}.{column_name}"));
            }
        }
    }
    ensure!(
        checked_columns.iter().any(|name| name.ends_with("_pence"))
            && checked_columns.iter().any(|name| name.ends_with("_bps")),
        "expected both _pence and _bps integer columns, checked {checked_columns:?}"
    );

    db.seed_push_owned_rows()?;
    let pence_type: String = conn.query_row(
        "SELECT typeof(estimated_value_pence) FROM notice_lots WHERE id = 'l4-lot-fixture'",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        pence_type == "integer",
        "integer _pence fixture stored with SQLite type `{pence_type}`"
    );

    let invalid_bps = conn.execute(
        "\
        INSERT INTO enforcement_readiness (
          slice_id, worker_skill_id, source_id, worker_behavior_version,
          validator_overlay_version, wilson_lower_bound_bps
        ) VALUES ('slice-bps-invalid', 'worker', 'source', 'worker@1', 'overlay@1', 'not-integer')
        ",
        [],
    );
    ensure!(
        invalid_bps.is_err(),
        "_bps constraint accepted a non-integer fixture value"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-012"]
fn l4_t_sqlite_schema_012_l0_does_not_precreate_l4_outbox() -> Result<()> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    run_repo_migration(&conn, "migrations/0001_sqlite_state_initial.sql")?;
    ensure!(
        !table_exists(&conn, "local_outbox")?,
        "L0 migration must not create local_outbox"
    );

    run_repo_migration(&conn, "migrations/0012_l4_sync_state.sql")?;
    ensure!(
        table_exists(&conn, "local_outbox")?,
        "L4 migration must create local_outbox"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-013"]
fn l4_t_sqlite_schema_013_reopening_db_does_not_duplicate_l4_state() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let db_path = temp.path().join("state.db");

    {
        let state_db = StateDb::open(&db_path)?;
        let conn = state_db.connection();
        ensure!(table_exists(conn, "local_outbox")?, "local_outbox missing");
    }

    let state_db = StateDb::open(&db_path)?;
    let conn = state_db.connection();
    let table_count = query_count(
        conn,
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN (
          'local_outbox', 'sync_batches', 'pull_cursors', 'provisional_worker_outputs',
          'enforcement_readiness', 'projection_generations', 'validation_calibration_cases',
          'validation_incident_clusters'
        )",
    )?;
    let cursor_count = query_count(conn, "SELECT count(*) FROM pull_cursors")?;
    let migration_count = query_count(
        conn,
        "SELECT count(*) FROM schema_migrations WHERE id = '0012_l4_sync_state'",
    )?;
    let unique_index_count = query_count(
        conn,
        "SELECT count(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_enforcement_readiness_slice'",
    )?;

    ensure!(table_count == 8, "unexpected L4 table count {table_count}");
    ensure!(
        cursor_count == 3,
        "pull_cursors seed rows duplicated or missing"
    );
    ensure!(
        migration_count == 1,
        "0012_l4_sync_state migration log duplicated"
    );
    ensure!(
        unique_index_count == 1,
        "idx_enforcement_readiness_slice duplicated or missing"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-014"]
fn l4_t_sqlite_schema_014_required_indices_exist() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();

    ensure!(
        index_names(conn, "local_outbox")?.contains("idx_local_outbox_dispatch"),
        "local_outbox dispatch index missing"
    );
    ensure!(
        index_names(conn, "sync_batches")?.contains("idx_sync_batches_in_flight"),
        "sync_batches in-flight index missing"
    );
    ensure!(
        sqlite_table_columns(conn, "pull_cursors")?
            .get("domain")
            .is_some_and(|column| column.primary_key_position > 0),
        "pull_cursors domain primary-key lookup missing"
    );
    ensure!(
        index_names(conn, "provisional_worker_outputs")?
            .contains("idx_provisional_worker_outputs_worker_job"),
        "provisional worker_job_id index missing"
    );
    ensure!(
        index_names(conn, "enforcement_readiness")?.contains("idx_enforcement_readiness_slice"),
        "enforcement slice index missing"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-015"]
fn l4_t_sqlite_schema_015_local_outbox_constraints_reject_incomplete_rows() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();

    ensure!(
        insert_outbox_row(
            conn,
            "delete",
            Some("stage-run"),
            Some(DEFAULT_SCHEMA_EPOCH),
            Some("commit-group"),
            "hash-invalid-op",
        )
        .is_err(),
        "local_outbox accepted op outside insert/update"
    );
    ensure!(
        insert_outbox_row(
            conn,
            "insert",
            Some("stage-run"),
            Some(DEFAULT_SCHEMA_EPOCH),
            Some("commit-group"),
            "",
        )
        .is_err(),
        "local_outbox accepted empty content_hash"
    );
    ensure!(
        insert_outbox_row(
            conn,
            "insert",
            Some("stage-run"),
            Some(DEFAULT_SCHEMA_EPOCH),
            None,
            "hash-missing-commit-group",
        )
        .is_err(),
        "local_outbox accepted missing commit_group_id"
    );
    ensure!(
        insert_outbox_row(
            conn,
            "insert",
            None,
            Some(DEFAULT_SCHEMA_EPOCH),
            Some("commit-group"),
            "hash-missing-stage-run",
        )
        .is_err(),
        "local_outbox accepted missing stage_run_id"
    );
    ensure!(
        insert_outbox_row(
            conn,
            "insert",
            Some("stage-run"),
            None,
            Some("commit-group"),
            "hash-missing-schema-epoch",
        )
        .is_err(),
        "local_outbox accepted missing schema_epoch"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-016"]
fn l4_t_sqlite_schema_016_pending_operator_intents_l2_queue_shape() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    let columns = sqlite_table_columns(conn, "pending_operator_intents")?;

    ensure_columns(
        "pending_operator_intents",
        &columns,
        &[
            "id",
            "process_id",
            "desired_disposition",
            "reviewed_basis_hash",
            "reviewed_manifest_hash",
            "reviewed_at_stage_run_id",
            "reviewed_at",
            "base_remote_revision",
            "idempotency_key",
            "status",
            "superseded_by_intent_id",
            "queued_at",
        ],
    )?;
    ensure!(
        index_names(conn, "pending_operator_intents")?
            .contains("idx_pending_operator_intents_latest"),
        "pending_operator_intents latest-intent ordering index missing"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-017"]
fn l4_t_sqlite_schema_017_validation_verdicts_join_provisional_outputs() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    let columns = sqlite_table_columns(conn, "validation_verdicts")?;

    ensure_columns(
        "validation_verdicts",
        &columns,
        &[
            "id",
            "worker_job_id",
            "worker_session_id",
            "validator_job_id",
            "validator_session_id",
            "validator_skill_path",
            "validator_skill_version",
            "verdict",
            "confidence_bps",
            "summary",
            "basis_sufficiency",
            "incident_signature",
            "enforcement_eligible",
            "issues_json",
            "run_manifest_hash",
            "override_verdict",
            "override_reason",
            "override_by",
            "override_at",
            "verdict_rendered_at",
            "created_at",
        ],
    )?;

    seed_workflow_job(conn, "l4-worker-job")?;
    seed_workflow_job(conn, "l4-validator-job")?;
    conn.execute(
        "\
        INSERT INTO provisional_worker_outputs (
          id, worker_job_id, payload_json, validation_status
        ) VALUES ('l4-provisional-join', 'l4-worker-job', '{\"candidate\":true}', 'pending')
        ",
        [],
    )?;
    conn.execute(
        "\
        INSERT INTO validation_verdicts (
          id, worker_job_id, validator_job_id, validator_skill_path, verdict, confidence_bps,
          summary, run_manifest_hash, verdict_rendered_at
        ) VALUES (
          'l4-verdict-join', 'l4-worker-job', 'l4-validator-job', 'validation/base',
          'approved', 9000, 'fixture verdict', 'manifest-hash', '2026-05-06T00:00:00Z'
        )
        ",
        [],
    )?;
    let joined = query_count(
        conn,
        "\
        SELECT count(*)
        FROM validation_verdicts vv
        JOIN provisional_worker_outputs pwo ON pwo.worker_job_id = vv.worker_job_id
        WHERE vv.id = 'l4-verdict-join'
        ",
    )?;
    ensure!(
        joined == 1,
        "validation_verdicts.worker_job_id did not join to provisional_worker_outputs"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-018"]
fn l4_t_sqlite_schema_018_supabase_config_and_eight_migrations_exist() -> Result<()> {
    let root = repo_root()?;
    ensure!(
        root.join("infra/supabase/config.toml").is_file(),
        "infra/supabase/config.toml missing"
    );

    let migration_dir = root.join("infra/supabase/migrations");
    let mut names = fs::read_dir(&migration_dir)
        .with_context(|| format!("reading {}", migration_dir.display()))?
        .map(|entry| {
            let entry = entry?;
            Ok(entry.file_name().to_string_lossy().into_owned())
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    names.retain(|name| {
        Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sql"))
    });
    names.sort();

    let expected = (1..=8)
        .map(|ordinal| format!("{ordinal:04}_"))
        .collect::<Vec<_>>();
    ensure!(
        names.len() == 8,
        "expected exactly eight Supabase migrations, got {names:?}"
    );
    for (name, prefix) in names.iter().zip(expected) {
        ensure!(
            name.starts_with(&prefix),
            "migration `{name}` did not have expected prefix `{prefix}`"
        );
    }
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-019"]
fn l4_t_sqlite_schema_019_supabase_projection_tables_match_inventory() -> Result<()> {
    let stack = SupabaseLocalStack::start_or_reuse()?;
    let _applied = stack.apply_migrations()?;
    let sql = read_repo_file("infra/supabase/migrations/0001_projection_tables.sql")?;
    let table_names = PUSH_OWNED_TABLES
        .iter()
        .chain(JUNCTION_TABLES.iter())
        .copied()
        .collect::<Vec<_>>();
    let schema = logical_schema_from_sql(&sql, &table_names)?;

    for table in table_names {
        let columns = schema
            .get(table)
            .with_context(|| format!("missing Supabase projection table `{table}`"))?;
        ensure_logical_columns(table, columns, SYNC_TRACKING_COLUMNS)?;
    }
    ensure!(
        !sql.to_ascii_lowercase().contains(" references "),
        "Supabase projection tables must not define foreign keys to local-only tables"
    );
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-020"]
fn l4_t_sqlite_schema_020_supabase_summary_tables_are_redacted_shape() -> Result<()> {
    let sql = [
        read_repo_file("infra/supabase/migrations/0003_summary_tables.sql")?,
        read_repo_file("infra/supabase/migrations/0008_decision_summaries.sql")?,
    ]
    .join("\n");
    let schema = logical_schema_from_sql(&sql, SUMMARY_TABLES)?;
    let forbidden_columns = [
        "company_name",
        "registration_number",
        "vat_number",
        "address_json",
        "contact_json",
        "operator_id",
        "command_id",
        "response_json",
        "validator_session_id",
        "validator_skill_path",
        "validator_reasoning",
        "issues_json",
        "quality_assessment_json",
        "skill_improvement_suggestions_json",
        "input_payload",
        "output_payload",
        "last_error",
        "markdown",
        "raw_supplier_data",
        "raw_metadata",
    ];

    for table in SUMMARY_TABLES {
        let columns = schema
            .get(*table)
            .with_context(|| format!("missing summary table `{table}`"))?;
        for forbidden in forbidden_columns {
            ensure!(
                !columns.iter().any(|column| column.name == forbidden),
                "summary table `{table}` exposes forbidden raw-context column `{forbidden}`"
            );
        }
    }
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-021"]
fn l4_t_sqlite_schema_021_supabase_remote_owned_tables_have_revision_triggers() -> Result<()> {
    let sql = read_repo_file("infra/supabase/migrations/0002_remote_owned_tables.sql")?;
    let schema = logical_schema_from_sql(&sql, REMOTE_OWNED_TABLES)?;

    for table in REMOTE_OWNED_TABLES {
        let columns = schema
            .get(*table)
            .with_context(|| format!("missing remote-owned table `{table}`"))?;
        ensure_logical_columns(
            table,
            columns,
            &[
                "org_id",
                "remote_change_seq",
                "remote_revision",
                "deleted_at",
                "created_at",
                "updated_at",
            ],
        )?;
    }
    assert_sql_contains_all(
        &sql,
        &[
            "CREATE SEQUENCE shared_company_profile_change_seq",
            "CREATE SEQUENCE shared_source_configs_change_seq",
            "CREATE SEQUENCE operator_decision_heads_change_seq",
            "CREATE OR REPLACE FUNCTION l4_remote_owned_touch",
            "CREATE TRIGGER shared_company_profile_remote_change_seq",
            "CREATE TRIGGER shared_source_configs_remote_change_seq",
            "CREATE TRIGGER operator_decision_heads_remote_change_seq",
        ],
    )?;
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-022"]
fn l4_t_sqlite_schema_022_supabase_rls_policy_file_covers_role_matrix() -> Result<()> {
    let sql = read_repo_file("infra/supabase/migrations/0004_rls_policies.sql")?;
    let lower_sql = sql.to_ascii_lowercase();

    assert_sql_contains_all(
        &lower_sql,
        &[
            "enable row level security",
            "viewer",
            "operator",
            "admin",
            "daemon",
            "grant usage on schema public",
            "grant select on all tables in schema public to anon, authenticated",
            "grant all on all tables in schema public to service_role",
        ],
    )?;
    for table in PUSH_OWNED_TABLES
        .iter()
        .chain(JUNCTION_TABLES.iter())
        .chain(&[
            "workflow_job_summaries",
            "error_manifest_summaries",
            "validation_verdict_summaries",
            "viewer_sync_status",
            "executors",
            "shared_company_profile",
            "shared_source_configs",
            "operator_decision_heads",
        ])
    {
        ensure!(
            lower_sql.contains(table),
            "RLS policy migration does not mention table `{table}`"
        );
    }
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-023"]
fn l4_t_sqlite_schema_023_supabase_schema_metadata_and_writer_lease_split() -> Result<()> {
    let sql = read_repo_file("infra/supabase/migrations/0006_schema_metadata.sql")?;
    let schema = logical_schema_from_sql(&sql, &["schema_metadata", "active_writer_lease"])?;
    let metadata = schema
        .get("schema_metadata")
        .context("missing schema_metadata table")?;
    let lease = schema
        .get("active_writer_lease")
        .context("missing active_writer_lease table")?;

    ensure_logical_columns(
        "schema_metadata",
        metadata,
        &[
            "org_id",
            "schema_epoch",
            "projection_generation",
            "applied_at",
            "updated_at",
        ],
    )?;
    for lease_only in ["holder_executor_id", "lease_expires_at", "writer_epoch"] {
        ensure!(
            !metadata.iter().any(|column| column.name == lease_only),
            "schema_metadata must not own writer lease column `{lease_only}`"
        );
    }
    ensure_logical_columns(
        "active_writer_lease",
        lease,
        &[
            "org_id",
            "holder_executor_id",
            "lease_expires_at",
            "writer_epoch",
            "updated_at",
        ],
    )?;
    assert_sql_contains_all(
        &sql,
        &[
            "INSERT INTO schema_metadata",
            "VALUES ('default', 9, 0)",
            "ALTER TABLE schema_metadata ENABLE ROW LEVEL SECURITY",
            "ALTER TABLE active_writer_lease ENABLE ROW LEVEL SECURITY",
        ],
    )?;
    Ok(())
}

#[test]
#[doc = "L4-T-sqlite_schema-024"]
fn l4_t_sqlite_schema_024_workspace_scripts_and_reqwest_are_wired() -> Result<()> {
    let package_json: Value = serde_json::from_str(&read_repo_file("package.json")?)?;
    let workspaces = package_json
        .get("workspaces")
        .and_then(Value::as_array)
        .context("root package.json workspaces must be an array")?;
    ensure!(
        workspaces
            .iter()
            .any(|entry| entry.as_str() == Some("apps/*")),
        "root workspaces must include apps/*"
    );
    ensure!(
        workspaces
            .iter()
            .any(|entry| entry.as_str() == Some("packages/*")),
        "root workspaces must include packages/*"
    );

    let scripts = package_json
        .get("scripts")
        .and_then(Value::as_object)
        .context("root package.json scripts must be an object")?;
    for (script_name, web_command) in [
        ("build", "cd apps/web && bun run build"),
        ("test", "cd apps/web && bun run test"),
        ("lint", "cd apps/web && bun run lint"),
    ] {
        let script = scripts
            .get(script_name)
            .and_then(Value::as_str)
            .with_context(|| format!("missing root script `{script_name}`"))?;
        ensure!(
            script.contains(web_command),
            "root `{script_name}` script does not execute `{web_command}`: {script}"
        );
    }
    let lint_script = scripts
        .get("lint")
        .and_then(Value::as_str)
        .context("missing lint script")?;
    for boundary in [
        "@tauri-apps/api",
        "apps/desktop",
        "crates/daemon",
        "supabase",
        "transport",
    ] {
        ensure!(
            lint_script.contains(boundary),
            "root lint script lost packages/ui boundary for `{boundary}`"
        );
    }

    let cargo_toml = read_repo_file("crates/daemon/Cargo.toml")?;
    assert_sql_contains_all(
        &cargo_toml,
        &[
            "reqwest = { version = \"0.12\"",
            "default-features = false",
            "rustls-tls",
            "json",
        ],
    )?;
    ensure!(
        !cargo_toml.contains("tokio-postgres")
            && !cargo_toml.contains("ureq")
            && !cargo_toml.contains("hyper ="),
        "daemon Cargo.toml contains a disallowed direct HTTP/Postgres dependency"
    );
    Ok(())
}

fn repo_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("daemon crate must live two levels below repository root")
}

fn read_repo_file(relative_path: &str) -> Result<String> {
    let path = repo_root()?.join(relative_path);
    fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
}

fn run_repo_migration(conn: &Connection, relative_path: &str) -> Result<()> {
    let sql = read_repo_file(relative_path)?;
    conn.execute_batch(&sql)
        .with_context(|| format!("applying {relative_path}"))
}

fn sqlite_table_columns(conn: &Connection, table: &str) -> Result<BTreeMap<String, SqliteColumn>> {
    ensure!(table_exists(conn, table)?, "missing SQLite table `{table}`");
    let pragma = format!("PRAGMA table_info({})", sqlite_identifier(table));
    let mut stmt = conn.prepare(&pragma)?;
    let mut columns = BTreeMap::new();
    let rows = stmt.query_map([], |row| {
        let name: String = row.get("name")?;
        Ok((
            name,
            SqliteColumn {
                raw_type: row.get("type")?,
                not_null: row.get::<_, i64>("notnull")? != 0,
                primary_key_position: row.get("pk")?,
            },
        ))
    })?;
    for row in rows {
        let (name, column) = row?;
        columns.insert(name, column);
    }
    ensure!(!columns.is_empty(), "table `{table}` had no columns");
    Ok(columns)
}

fn ensure_columns(
    table: &str,
    columns: &BTreeMap<String, SqliteColumn>,
    expected: &[&str],
) -> Result<()> {
    for column in expected {
        ensure!(
            columns.contains_key(*column),
            "table `{table}` missing required column `{column}`"
        );
    }
    Ok(())
}

fn ensure_logical_columns(
    table: &str,
    columns: &[support::db::LogicalColumn],
    expected: &[&str],
) -> Result<()> {
    let names = columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<BTreeSet<_>>();
    for column in expected {
        ensure!(
            names.contains(column),
            "table `{table}` missing required column `{column}`"
        );
    }
    Ok(())
}

fn ensure_column_type_contains(
    table: &str,
    columns: &BTreeMap<String, SqliteColumn>,
    column: &str,
    expected_type_fragment: &str,
) -> Result<()> {
    let actual = columns
        .get(column)
        .with_context(|| format!("table `{table}` missing column `{column}`"))?;
    ensure!(
        actual
            .raw_type
            .to_ascii_uppercase()
            .contains(expected_type_fragment),
        "{table}.{column} type `{}` did not contain `{expected_type_fragment}`",
        actual.raw_type
    );
    Ok(())
}

fn ensure_column_nullable(
    table: &str,
    columns: &BTreeMap<String, SqliteColumn>,
    column: &str,
    expected_nullable: bool,
) -> Result<()> {
    let actual = columns
        .get(column)
        .with_context(|| format!("table `{table}` missing column `{column}`"))?;
    let nullable = !actual.not_null && actual.primary_key_position == 0;
    ensure!(
        nullable == expected_nullable,
        "{table}.{column} nullable={nullable}, expected {expected_nullable}"
    );
    Ok(())
}

fn sqlite_table_sql(conn: &Connection, table: &str) -> Result<String> {
    conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get(0),
    )
    .with_context(|| format!("reading CREATE TABLE SQL for `{table}`"))
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some();
    Ok(exists)
}

fn index_names(conn: &Connection, table: &str) -> Result<BTreeSet<String>> {
    let pragma = format!("PRAGMA index_list({})", sqlite_identifier(table));
    let mut stmt = conn.prepare(&pragma)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>("name"))?;
    let mut names = BTreeSet::new();
    for row in rows {
        names.insert(row?);
    }
    Ok(names)
}

fn query_strings(conn: &Connection, sql: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }
    Ok(values)
}

fn query_i64s(conn: &Connection, sql: &str) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }
    Ok(values)
}

fn query_count(conn: &Connection, sql: &str) -> Result<i64> {
    conn.query_row(sql, [], |row| row.get(0))
        .with_context(|| format!("query count failed: {sql}"))
}

fn insert_outbox_row(
    conn: &Connection,
    op: &str,
    stage_run_id: Option<&str>,
    schema_epoch: Option<i64>,
    commit_group_id: Option<&str>,
    content_hash: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "\
        INSERT INTO local_outbox (
          executor_id, entity_kind, entity_id, op, row_version, stage_run_id,
          schema_epoch, commit_group_id, payload_json, content_hash
        ) VALUES (
          'executor-1', 'procurement_processes', 'process-1', ?1, 1, ?2,
          ?3, ?4, '{\"id\":\"process-1\"}', ?5
        )
        ",
        params![
            op,
            stage_run_id,
            schema_epoch,
            commit_group_id,
            content_hash
        ],
    )
}

fn seed_workflow_job(conn: &Connection, job_id: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO workflow_jobs (
          id, kind, status, input_payload
        ) VALUES (?1, 'validation', 'queued', '{}')
        ON CONFLICT(id) DO NOTHING
        ",
        [job_id],
    )?;
    Ok(())
}

fn count_rows(conn: &Connection, table: &str) -> Result<i64> {
    let sql = format!("SELECT COUNT(*) FROM {}", sqlite_identifier(table));
    conn.query_row(&sql, [], |row| row.get(0))
        .with_context(|| format!("counting rows in {table}"))
}

fn assert_sql_contains_all(sql: &str, needles: &[&str]) -> Result<()> {
    for needle in needles {
        ensure!(
            sql.contains(needle),
            "SQL/text missing required fragment `{needle}`"
        );
    }
    Ok(())
}

fn sqlite_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}
