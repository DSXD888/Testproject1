#![forbid(unsafe_code)]

#[path = "../src/agents/arbiter.rs"]
#[allow(dead_code)]
mod arbiter_impl;
#[path = "../src/validation/calibration.rs"]
#[allow(dead_code)]
mod calibration_impl;
#[path = "../src/agents/contracts.rs"]
#[allow(dead_code)]
mod contracts_impl;
#[path = "../src/scheduler/cron.rs"]
#[allow(dead_code)]
mod cron_impl;
#[path = "../src/ipc/handlers/decisions.rs"]
#[allow(dead_code)]
mod decisions_handler_impl;
#[allow(dead_code, clippy::missing_const_for_fn)]
#[path = "../src/errors/manifest.rs"]
mod errors_manifest_impl;
#[allow(dead_code, clippy::missing_const_for_fn)]
#[path = "../src/errors/markdown.rs"]
mod errors_markdown_impl;
#[path = "../src/validation/feedback.rs"]
#[allow(dead_code)]
mod feedback_impl;
#[path = "../src/jobs/runner.rs"]
#[allow(dead_code)]
mod runner_impl;
#[path = "../src/sources/registry.rs"]
#[allow(dead_code)]
mod sources_impl;
mod support;

mod agents {
    pub mod arbiter {
        pub use crate::arbiter_impl::*;
    }
}

mod jobs {
    pub mod runner {
        pub use crate::runner_impl::*;
    }
}

mod db {
    pub mod sqlite {
        pub use crate::support::db::*;
    }
}

mod sources {
    pub mod registry {
        pub use crate::sources_impl::*;
    }
}

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::str;

use anyhow::{Context, Result, ensure};
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use decisions_handler_impl::{
    DECISION_CONFLICT_TOPIC, DecisionConflictEventParams, decision_conflict_ipc_event,
};
use errors_manifest_impl::{
    ErrorCode, ErrorManifestDraft, ErrorManifestStore, ErrorRuntimeContext,
};
use feedback_impl::{OverrideFeedbackRequest, ingest_override};
use runner_impl::{
    JobRunner, MutationAck, PersistEnforcementProvisionalRequest, PersistValidationVerdictRequest,
    PromoteProvisionalRequest, ValidatorEnqueueRequest, ValidatorJobPurpose, ValidatorSkillBundle,
};
use support::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb, StateDb};
use support::redaction::RedactionSentinelFixture;
use support::sync::{
    EnforcementHarness, OperatorDecisionCasHarness, OutboxHarness, PullCursorDomain,
    PullCursorHarness, PushOwnedFixture, REMOTE_CONFIG_OPERATIONAL_WARN_AFTER_SECS,
    REMOTE_CONFIG_SAFETY_CRITICAL_TTL_SECS, RemoteConfigStalenessAction,
    RemoteConfigStalenessClass, RemoteConfigStalenessSubject, SchemaEpochGuard,
    SqliteSnapshotArtifact, SqliteSnapshotScheduler, SupabaseRpcClient, SyncCyclePhase,
    SyncCycleTrigger, SyncHealth, ViewerSyncStatus, WriterBindingConfig, WriterBindingMode,
    WriterLeaseAcquireResult, WriterLeaseHarness, canonical_content_hash, parse_payload,
};

use calibration_impl::{
    CALIBRATION_MODEL_FAMILY_ALTERNATE, CalibrationCaseSeed, CalibrationCorpusKind,
    calibration_mutation_families, calibration_skill_families, canonical_calibration_cases,
    record_cross_model_audit_case, seed_calibration_corpus_cases, select_cross_model_audit_slices,
};
use cron_impl::{
    CronJobClass, CronScheduler, VALIDATION_CALIBRATION_SOURCE_ID,
    VALIDATION_CROSS_MODEL_AUDIT_SKILL_ID,
};
use sources_impl::SourceRegistry;

#[test]
fn l4_t_sqlite_supabase_sync_001_push_owned_mutation_and_outbox_commit_atomically() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-001");
    println!("[SETUP] L4TestStateDb, OutboxHarness");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;

    println!("[EXEC] mutate push-owned row through transactional outbox writer");
    let mutated = harness.mutate_push_owned_row()?;

    println!("[ASSERT] exactly one matching outbox row exists with shared commit_group_id");
    let outbox_rows = harness.outbox_rows_for_entity(&mutated.entity_kind, &mutated.entity_id)?;
    ensure!(
        outbox_rows.len() == 1,
        "expected one outbox row for {}:{}, got {}",
        mutated.entity_kind,
        mutated.entity_id,
        outbox_rows.len()
    );
    let outbox = &outbox_rows[0];
    ensure!(
        outbox.local_seq == mutated.local_seq,
        "outbox local_seq {} did not match returned local_seq {}",
        outbox.local_seq,
        mutated.local_seq
    );
    ensure!(
        outbox.commit_group_id == mutated.commit_group_id,
        "outbox commit_group_id did not match transaction commit group"
    );
    ensure!(
        outbox.stage_run_id == mutated.stage_run_id,
        "outbox stage_run_id did not match mutation stage run"
    );
    ensure!(
        outbox.schema_epoch == DEFAULT_SCHEMA_EPOCH,
        "outbox schema_epoch {} did not match default epoch {DEFAULT_SCHEMA_EPOCH}",
        outbox.schema_epoch
    );
    ensure!(
        outbox.writer_epoch.is_none(),
        "local_outbox.writer_epoch must remain NULL before batch envelope assignment"
    );
    ensure!(
        outbox.content_hash == mutated.content_hash,
        "content_hash persisted in outbox changed"
    );
    let payload = parse_payload(&outbox.payload_json)?;
    ensure!(
        payload
            .get("row_version")
            .and_then(serde_json::Value::as_i64)
            == Some(outbox.row_version),
        "outbox row_version did not come from payload_json"
    );
    let last_local_seq = harness.process_last_local_seq(&mutated.entity_id)?;
    ensure!(
        last_local_seq == Some(outbox.local_seq),
        "push-owned row last_local_seq did not point at the committed outbox row"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-001 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_002_missing_outbox_rolls_back_push_owned_mutation() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-002");
    println!("[SETUP] L4TestStateDb, OutboxHarness");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;
    let before_process_count = harness.count_rows("procurement_processes")?;
    let before_outbox_count = harness.count_rows("local_outbox")?;

    println!("[EXEC] attempt push-owned mutation without matching local_outbox row");
    let result = harness.mutate_without_outbox("l4-sync-missing-outbox-process");

    println!("[ASSERT] transaction returns error and rolls back both data and outbox state");
    ensure!(
        result.is_err(),
        "push-owned transaction without outbox unexpectedly succeeded"
    );
    ensure!(
        harness.count_rows("procurement_processes")? == before_process_count,
        "push-owned row committed despite missing local_outbox entry"
    );
    ensure!(
        harness.count_rows("local_outbox")? == before_outbox_count,
        "local_outbox row count changed after rejected transaction"
    );
    ensure!(
        harness
            .process_last_local_seq("l4-sync-missing-outbox-process")?
            .is_none(),
        "missing-outbox process row remained visible after rollback"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-002 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_003_rejected_provisional_output_creates_no_outbox() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-003");
    println!("[SETUP] L4TestStateDb, EnforcementHarness");
    let state_db = L4TestStateDb::new()?;
    let mut harness = EnforcementHarness::new(&state_db)?;
    let before_outbox_count = harness.count_rows("local_outbox")?;
    let before_process_count = harness.count_rows("procurement_processes")?;

    println!("[EXEC] persist enforcement-active rejected provisional output");
    harness.reject_provisional_output("l4-rejected-worker-job")?;

    println!(
        "[ASSERT] rejected provisional result remains local-only and creates no push-owned mutation"
    );
    ensure!(
        harness.count_rows("provisional_worker_outputs")? == 1,
        "rejected provisional output was not preserved locally"
    );
    ensure!(
        harness.count_rows("local_outbox")? == before_outbox_count,
        "rejected provisional output created a local_outbox row"
    );
    ensure!(
        harness.count_rows("procurement_processes")? == before_process_count,
        "rejected provisional output created a push-owned row"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-003 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_004_single_in_flight_batch_per_stream() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-004");
    println!("[SETUP] two dispatchable outbox rows in separate commit groups");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-batch-first", 1))?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-batch-second", 2))?;

    println!("[EXEC] materialize first batch with max_rows=1");
    let first_batch = harness
        .assemble_batch(1, 7)?
        .expect("first dispatchable batch");
    let second_attempt = harness.assemble_batch(1, 7);

    println!("[ASSERT] second in-flight batch for same stream is refused");
    ensure!(
        second_attempt.is_err(),
        "assembler created a second in-flight batch for the same stream"
    );
    ensure!(
        harness.sync_batch_count()? == 1,
        "second in-flight attempt inserted a duplicate sync_batches row"
    );

    println!("[EXEC] acknowledge first batch and then assemble next range");
    harness.mark_batch_acknowledged(&first_batch.batch_id)?;
    let second_batch = harness
        .assemble_batch(1, 7)?
        .expect("second dispatchable batch after ack");

    println!("[ASSERT] next range dispatches only after prior batch is terminal");
    ensure!(
        second_batch.first_local_seq > first_batch.last_local_seq,
        "second batch did not advance to the later local_seq range"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-004 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_005_restart_reloads_in_flight_batch_without_duplicate() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-005");
    println!("[SETUP] persisted in-flight sync batch");
    let state_db = L4TestStateDb::new()?;
    {
        let mut harness = OutboxHarness::new(&state_db)?;
        harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-restart-batch", 1))?;
        harness
            .assemble_batch(10, 11)?
            .expect("persisted in-flight batch");
    }

    println!("[EXEC] reopen DB and ask assembler for active work");
    let mut reopened = OutboxHarness::new(&state_db)?;
    let active = reopened
        .load_active_batch()?
        .expect("active batch should survive DB reopen");
    let duplicate_attempt = reopened.assemble_batch(10, 11);

    println!("[ASSERT] active payload is loaded and no duplicate batch is created");
    ensure!(
        active.status == "in_flight",
        "reopened active batch had unexpected status {}",
        active.status
    );
    ensure!(
        duplicate_attempt.is_err(),
        "assembler created a duplicate while an in-flight batch existed"
    );
    ensure!(
        reopened.sync_batch_count()? == 1,
        "restart path inserted a duplicate sync_batches row"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-005 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_006_timeout_retries_identical_envelope() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-006");
    println!("[SETUP] in-flight batch plus later outbox row");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-timeout-a", 1))?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-timeout-b", 2))?;
    let batch = harness
        .assemble_batch(10, 17)?
        .expect("initial in-flight batch");
    let original_payload = batch.payload_json.clone();
    let original_hash = batch.payload_hash.clone();
    let original_rows = batch
        .rows
        .iter()
        .map(|row| row.local_seq)
        .collect::<Vec<_>>();

    println!("[EXEC] timeout marks batch retry after a new outbox row arrives");
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-timeout-later", 3))?;
    let retry = harness.mark_batch_retry(&batch.batch_id)?;

    println!("[ASSERT] retry preserves exact payload and excludes later outbox rows");
    ensure!(
        retry.status == "retry",
        "retry batch status was {}",
        retry.status
    );
    ensure!(
        retry.payload_json == original_payload,
        "retry rebuilt payload_json after timeout"
    );
    ensure!(
        retry.payload_hash == original_hash,
        "retry changed payload_hash after timeout"
    );
    ensure!(
        retry
            .rows
            .iter()
            .map(|row| row.local_seq)
            .collect::<Vec<_>>()
            == original_rows,
        "retry row set changed after timeout"
    );
    ensure!(
        harness.assemble_batch(10, 17).is_err(),
        "new outbox row was enveloped while retry batch was active"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-006 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_007_crash_retry_precedes_later_rows() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-007");
    println!("[SETUP] persisted batch before simulated crash");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-crash-before", 1))?;
    let batch = harness
        .assemble_batch(10, 19)?
        .expect("initial in-flight batch");
    drop(harness);

    println!("[EXEC] reopen DB, add later row, then inspect active batch");
    let mut restarted = OutboxHarness::new(&state_db)?;
    restarted.mutate_push_owned_fixture(PushOwnedFixture::new("l4-crash-after", 2))?;
    let active = restarted
        .load_active_batch()?
        .expect("crashed in-flight batch should reload");

    println!("[ASSERT] restart retries persisted envelope before later rows");
    ensure!(
        active.batch_id == batch.batch_id,
        "restart loaded a different active batch"
    );
    ensure!(
        active.rows.len() == 1 && active.rows[0].entity_id == "l4-crash-before",
        "restart active batch mixed in later outbox rows"
    );
    ensure!(
        restarted.assemble_batch(10, 19).is_err(),
        "restart built a later batch before resolving persisted in-flight batch"
    );
    ensure!(
        restarted.sync_batch_count()? == 1,
        "restart created duplicate sync_batches rows"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-007 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_008_stale_writer_reenvelopes_same_rows_with_new_epoch() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-008");
    println!("[SETUP] in-flight batch assigned to stale writer epoch");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-stale-a", 1))?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-stale-b", 2))?;
    let old_batch = harness
        .assemble_batch(10, 3)?
        .expect("initial in-flight batch");
    let old_rows = old_batch
        .rows
        .iter()
        .map(|row| row.local_seq)
        .collect::<Vec<_>>();

    println!("[EXEC] terminal stale-writer rejection response re-envelopes same rows");
    let new_batch =
        harness.record_stale_writer_rejection_after_reacquire(&old_batch.batch_id, 4)?;

    println!("[ASSERT] old batch rejected and replacement uses only new writer_epoch");
    ensure!(
        harness.batch_by_id(&old_batch.batch_id)?.status == "rejected",
        "stale batch was not terminally rejected"
    );
    ensure!(
        new_batch.writer_epoch == 4,
        "replacement batch did not use new writer_epoch"
    );
    ensure!(
        new_batch
            .rows
            .iter()
            .map(|row| row.local_seq)
            .collect::<Vec<_>>()
            == old_rows,
        "stale-writer replacement did not preserve the same outbox rows"
    );
    for local_seq in old_rows {
        ensure!(
            harness.outbox_row_by_seq(local_seq)?.writer_epoch == Some(4),
            "local_outbox row {local_seq} was not reassigned to new writer_epoch"
        );
    }
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-008 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_009_batch_rejects_mixed_schema_epoch() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-009");
    println!("[SETUP] dispatchable outbox rows from two schema epochs");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-epoch-a", 1))?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-epoch-b", 2).schema_epoch(10))?;

    println!("[EXEC] attempt batch assembly");
    let result = harness.assemble_batch(10, 5);

    println!("[ASSERT] mixed schema_epoch is rejected without creating a batch");
    ensure!(
        result.is_err(),
        "mixed schema_epoch batch unexpectedly succeeded"
    );
    ensure!(
        harness.sync_batch_count()? == 0,
        "mixed schema_epoch rejection still persisted sync_batches row"
    );
    ensure!(
        harness
            .all_outbox_rows()?
            .iter()
            .all(|row| row.writer_epoch.is_none()),
        "mixed schema_epoch rejection assigned writer_epoch to local_outbox rows"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-009 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_010_batch_rejects_mixed_stage_run() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-010");
    println!("[SETUP] dispatchable outbox rows from two stage runs");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;
    harness.mutate_push_owned_fixture(
        PushOwnedFixture::new("l4-stage-a", 1).stage_run_id("l4-stage-run-a"),
    )?;
    harness.mutate_push_owned_fixture(
        PushOwnedFixture::new("l4-stage-b", 2).stage_run_id("l4-stage-run-b"),
    )?;

    println!("[EXEC] attempt batch assembly");
    let result = harness.assemble_batch(10, 5);

    println!("[ASSERT] mixed stage_run_id is rejected without creating a batch");
    ensure!(
        result.is_err(),
        "mixed stage_run_id batch unexpectedly succeeded"
    );
    ensure!(
        harness.sync_batch_count()? == 0,
        "mixed stage_run_id rejection still persisted sync_batches row"
    );
    ensure!(
        harness
            .all_outbox_rows()?
            .iter()
            .all(|row| row.writer_epoch.is_none()),
        "mixed stage_run_id rejection assigned writer_epoch to local_outbox rows"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-010 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_011_commit_group_is_never_split_by_batch_size() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-011");
    println!("[SETUP] two rows share one commit_group_id with max_rows=1");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;
    harness.mutate_push_owned_fixture(
        PushOwnedFixture::new("l4-commit-group-a", 1).commit_group_id("l4-shared-group"),
    )?;
    harness.mutate_push_owned_fixture(
        PushOwnedFixture::new("l4-commit-group-b", 2).commit_group_id("l4-shared-group"),
    )?;

    println!("[EXEC] assemble with configured max_rows below commit group size");
    let batch = harness
        .assemble_batch(1, 6)?
        .expect("commit-group batch should materialize");

    println!("[ASSERT] whole commit group is sent together");
    ensure!(
        batch.rows.len() == 2,
        "batch split shared commit_group_id despite max_rows=1"
    );
    ensure!(
        batch.commit_group_ids == vec!["l4-shared-group".to_owned()],
        "batch commit_group_ids_json did not contain exactly the shared group"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-011 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_012_batch_orders_by_row_version_then_entity_key() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-012");
    println!("[SETUP] same row_version rows inserted in reverse entity order");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-order-b", 1))?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-order-a", 1))?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-order-c", 2))?;

    println!("[EXEC] assemble batch");
    let batch = harness
        .assemble_batch(10, 8)?
        .expect("ordered batch should materialize");
    let ordered = batch
        .rows
        .iter()
        .map(|row| row.entity_id.as_str())
        .collect::<Vec<_>>();

    println!("[ASSERT] row order is deterministic by row_version, entity_kind, entity_id");
    ensure!(
        ordered == ["l4-order-a", "l4-order-b", "l4-order-c"],
        "unexpected batch row order: {ordered:?}"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-012 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_013_content_hash_is_canonical_and_reused_on_retry() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-013");
    println!("[SETUP] outbox row with canonical payload hash");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;
    let mutated = harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-hash-a", 1))?;
    let outbox = harness.outbox_row_by_seq(mutated.local_seq)?;
    let expected_hash = canonical_content_hash(&outbox.payload_json)?;

    println!("[EXEC] assemble and mark retry");
    let batch = harness
        .assemble_batch(10, 9)?
        .expect("hash batch should materialize");
    let retry = harness.mark_batch_retry(&batch.batch_id)?;

    println!("[ASSERT] content_hash comes from canonical payload and retry reuses payload bytes");
    ensure!(
        outbox.content_hash == expected_hash,
        "outbox content_hash was not canonical"
    );
    ensure!(
        batch.rows[0].content_hash == expected_hash,
        "batch row did not carry canonical content_hash"
    );
    ensure!(
        retry.payload_json == batch.payload_json && retry.payload_hash == batch.payload_hash,
        "retry recomputed or changed batch envelope"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-013 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_014_apply_push_batch_contract_is_transactional() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-014");
    println!("[SETUP] inspect Supabase RPC migration contract");
    let migration = read_repo_file("infra/supabase/migrations/0005_sync_rpcs.sql")?;
    let apply_push_batch_body = migration
        .split("CREATE OR REPLACE FUNCTION update_operator_decision")
        .next()
        .context("missing apply_push_batch section before update_operator_decision")?;

    println!("[ASSERT] apply_push_batch is implemented as a two-pass transactional RPC");
    ensure!(
        apply_push_batch_body.contains(
            "CREATE OR REPLACE FUNCTION apply_push_batch(batch_json jsonb, writer_epoch integer)"
        ),
        "apply_push_batch signature is missing or changed"
    );
    ensure!(
        apply_push_batch_body.contains("SECURITY DEFINER"),
        "apply_push_batch must execute behind the RPC security boundary"
    );
    ensure!(
        !apply_push_batch_body.contains("'not_implemented'"),
        "apply_push_batch still returns the Phase 1 placeholder"
    );
    ensure!(
        apply_push_batch_body.contains("l4_is_any_role(ARRAY['daemon', 'admin'])"),
        "apply_push_batch must be daemon/admin scoped"
    );
    ensure!(
        apply_push_batch_body.contains("FROM schema_metadata")
            && apply_push_batch_body.contains("FROM active_writer_lease"),
        "apply_push_batch must validate schema_epoch and writer_epoch before apply"
    );
    ensure!(
        apply_push_batch_body.contains("sync_applied_row_hashes")
            && apply_push_batch_body.contains("l4_content_hash(row_payload)"),
        "apply_push_batch must persist and validate content_hash metadata"
    );
    ensure!(
        migration.contains("CREATE OR REPLACE FUNCTION l4_canonical_json(payload jsonb)")
            && migration.contains("digest(l4_canonical_json(payload), 'sha256')"),
        "Supabase content hash must use the same compact canonical JSON contract as the daemon"
    );

    let rejection_gate = apply_push_batch_body
        .find("IF jsonb_array_length(invalid_rows) > 0 THEN")
        .context("missing invalid-row rejection gate")?;
    let apply_statement = apply_push_batch_body
        .find("PERFORM l4_apply_push_owned_payload")
        .context("missing push-owned apply helper call")?;
    ensure!(
        rejection_gate < apply_statement,
        "invalid-row rejection must happen before any target table mutation"
    );
    ensure!(
        apply_push_batch_body
            .matches("jsonb_array_elements(rows_json)")
            .count()
            >= 2,
        "apply_push_batch should validate all rows before the apply loop"
    );

    println!("[CLEANUP] static migration inspection only");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-014 PASS");
    Ok(())
}

#[tokio::test]
async fn l4_t_sqlite_supabase_sync_015_daemon_uses_http_rpc_not_direct_upsert() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-015");
    println!("[SETUP] persisted batch plus one-shot local HTTP capture server");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;
    harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-rpc-client", 1))?;
    let batch = harness
        .assemble_batch(10, 77)?
        .expect("RPC client batch should materialize");
    let (api_url, capture) = spawn_rpc_capture_server(json!({
        "status": "applied",
        "accepted": true,
        "row_count": 1,
        "applied_rows": [{"entity_kind": "procurement_processes", "entity_id": "l4-rpc-client"}],
        "noop_rows": [],
        "writer_epoch": 77
    }))
    .await?;

    println!("[EXEC] call SupabaseRpcClient::apply_push_batch");
    let client = SupabaseRpcClient::new(api_url, "service-role-test-key")?;
    let response = client.apply_push_batch(&batch).await?;
    let raw_request = capture.await.context("joining capture server task")??;

    println!("[ASSERT] daemon request goes through /rpc/apply_push_batch only");
    ensure!(response.accepted, "RPC response was not accepted");
    ensure!(
        response.status == "applied",
        "unexpected RPC response status {}",
        response.status
    );
    ensure!(
        raw_request.starts_with("POST /rest/v1/rpc/apply_push_batch "),
        "daemon client did not call the apply_push_batch REST RPC: {raw_request}"
    );
    let lower_request = raw_request.to_ascii_lowercase();
    ensure!(
        lower_request.contains("authorization: bearer service-role-test-key"),
        "daemon client did not send bearer service-role authorization"
    );
    ensure!(
        lower_request.contains("apikey: service-role-test-key"),
        "daemon client did not send Supabase apikey header"
    );
    ensure!(
        !lower_request.contains("/rest/v1/procurement_processes"),
        "daemon client attempted a direct projection table upsert"
    );

    let body = http_body(&raw_request)?;
    let request_json: Value =
        serde_json::from_str(body).context("parsing captured apply_push_batch request body")?;
    ensure!(
        request_json.get("writer_epoch").and_then(Value::as_i64) == Some(batch.writer_epoch),
        "request writer_epoch did not match the batch writer_epoch"
    );
    ensure!(
        request_json
            .get("batch_json")
            .and_then(|value| value.get("rows"))
            .and_then(Value::as_array)
            .is_some_and(|rows| rows.len() == 1),
        "request body did not wrap the persisted batch envelope as batch_json"
    );

    let sync_source = read_repo_file("crates/daemon/src/db/sync.rs")?;
    ensure!(
        sync_source.contains("/rest/v1/rpc/apply_push_batch"),
        "SupabaseRpcClient source does not contain the RPC endpoint"
    );
    ensure!(
        !sync_source.contains("/rest/v1/procurement_processes"),
        "SupabaseRpcClient source contains a direct projection table endpoint"
    );

    println!("[CLEANUP] tempfile-owned state DB and one-shot capture server");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-015 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_016_equal_version_matching_hash_is_noop_success() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-016");
    println!("[SETUP] inspect apply_push_batch equal-version retry branch");
    let body = apply_push_batch_body()?;
    let validation_noop = body
        .find("IF row_version = remote_row_version AND row_content_hash = remote_content_hash THEN")
        .context("missing equal-version matching-hash validation branch")?;
    let rejection_gate = body
        .find("IF jsonb_array_length(invalid_rows) > 0 THEN")
        .context("missing invalid-row rejection gate")?;
    let apply_noop = body
        .find(
            "IF remote_row_version IS NOT NULL\n       AND row_version = remote_row_version\n       AND row_content_hash = remote_content_hash THEN",
        )
        .context("missing apply-loop idempotent no-op branch")?;
    let apply_helper = body
        .find("PERFORM l4_apply_push_owned_payload")
        .context("missing push-owned apply helper call")?;

    println!("[ASSERT] equal version + matching hash is accepted as no-op before apply");
    ensure!(
        validation_noop < rejection_gate,
        "matching-hash retry must be accepted during the prevalidation pass"
    );
    ensure!(
        apply_noop < apply_helper,
        "matching-hash retry must skip target table mutation in the apply pass"
    );
    ensure!(
        body.contains("noop_rows := noop_rows || jsonb_build_array(jsonb_build_object(")
            && body.contains("'accepted', true")
            && body.contains("'noop_rows', noop_rows"),
        "no-op retry response must be an accepted success with noop_rows"
    );

    println!("[CLEANUP] static migration inspection only");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-016 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_017_equal_version_different_hash_is_hard_conflict() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-017");
    println!("[SETUP] inspect apply_push_batch hard-conflict branch");
    let body = apply_push_batch_body()?;
    let hard_conflict = body
        .find("ELSIF row_version = remote_row_version AND row_content_hash <> remote_content_hash THEN")
        .context("missing equal-version different-hash branch")?;
    let rejection_gate = body
        .find("IF jsonb_array_length(invalid_rows) > 0 THEN")
        .context("missing invalid-row rejection gate")?;
    let apply_helper = body
        .find("PERFORM l4_apply_push_owned_payload")
        .context("missing push-owned apply helper call")?;

    println!("[ASSERT] different hash is classified as data corruption and blocks mutation");
    ensure!(
        hard_conflict < rejection_gate && rejection_gate < apply_helper,
        "hard conflict must be recorded before the all-or-nothing rejection gate"
    );
    ensure!(
        body.contains("'error', 'hard_conflict'")
            && body.contains("'classification', 'data_corruption'")
            && body.contains("'entity_kind', row_table")
            && body.contains("'entity_id', row_entity_id"),
        "hard conflict must return structured data-corruption payload for the offending row"
    );
    ensure!(
        body.contains("'status', 'rejected'")
            && body.contains("'accepted', false")
            && body.contains("'invalid_rows', invalid_rows"),
        "hard conflict must reject without treating the error as retryable success"
    );

    println!("[CLEANUP] static migration inspection only");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-017 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_018_remote_plus_one_applies_once() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-018");
    println!("[SETUP] inspect apply_push_batch normal advance branch");
    let body = apply_push_batch_body()?;
    let normal_advance = body
        .find("ELSIF row_version = remote_row_version + 1 THEN")
        .context("missing remote row_version + 1 branch")?;
    let rejection_gate = body
        .find("IF jsonb_array_length(invalid_rows) > 0 THEN")
        .context("missing invalid-row rejection gate")?;
    let apply_helper = body
        .find("PERFORM l4_apply_push_owned_payload")
        .context("missing push-owned apply helper call")?;
    let applied_rows = body
        .find("applied_rows := applied_rows || jsonb_build_array(jsonb_build_object(")
        .context("missing applied_rows response append")?;

    println!("[ASSERT] remote + 1 passes validation and records the applied sequence");
    ensure!(
        normal_advance < rejection_gate
            && rejection_gate < apply_helper
            && apply_helper < applied_rows,
        "normal advance must validate first, apply once, then record applied_rows"
    );
    ensure!(
        body.contains("last_local_seq = EXCLUDED.last_local_seq")
            && body.contains("'applied_rows', applied_rows")
            && body.contains("'local_seq', row_local_seq"),
        "normal advance must update last_local_seq and surface applied row metadata"
    );

    println!("[CLEANUP] static migration inspection only");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-018 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_019_older_or_larger_jump_rejects_without_mutation() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-019");
    println!("[SETUP] inspect apply_push_batch stale/out-of-order branch");
    let body = apply_push_batch_body()?;
    let stale_or_jump = body
        .find("'error', 'stale_or_out_of_order'")
        .context("missing stale/out-of-order rejection classification")?;
    let rejection_gate = body
        .find("IF jsonb_array_length(invalid_rows) > 0 THEN")
        .context("missing invalid-row rejection gate")?;
    let apply_helper = body
        .find("PERFORM l4_apply_push_owned_payload")
        .context("missing push-owned apply helper call")?;

    println!("[ASSERT] stale versions and larger jumps reject before any projection write");
    ensure!(
        stale_or_jump < rejection_gate && rejection_gate < apply_helper,
        "stale/out-of-order rows must be rejected before any target table mutation"
    );
    ensure!(
        body.contains("'remote_row_version', remote_row_version")
            && body.contains("'incoming_row_version', row_version"),
        "stale/out-of-order response must include current and incoming versions"
    );
    ensure!(
        body.contains("'status', 'rejected'")
            && body.contains("'accepted', false")
            && body.contains("'invalid_rows', invalid_rows"),
        "stale/out-of-order rows must return a structured rejection payload"
    );

    println!("[CLEANUP] static migration inspection only");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-019 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_020_outbox_envelope_fields_survive_reopen() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-020");
    println!("[SETUP] local_outbox row before envelope assignment");
    let state_db = L4TestStateDb::new()?;
    let original = {
        let mut harness = OutboxHarness::new(&state_db)?;
        let mutated = harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-reopen-a", 1))?;
        harness.outbox_row_by_seq(mutated.local_seq)?
    };

    println!("[EXEC] reopen DB and read local_outbox row");
    let reopened = OutboxHarness::new(&state_db)?;
    let after_reopen = reopened.outbox_row_by_seq(original.local_seq)?;

    println!("[ASSERT] local_seq payload/hash/commit_group/writer_epoch remain stable");
    ensure!(
        after_reopen.local_seq == original.local_seq,
        "local_seq changed across reopen"
    );
    ensure!(
        after_reopen.payload_json == original.payload_json,
        "payload_json changed across reopen"
    );
    ensure!(
        after_reopen.content_hash == original.content_hash,
        "content_hash changed across reopen"
    );
    ensure!(
        after_reopen.commit_group_id == original.commit_group_id,
        "commit_group_id changed across reopen"
    );
    ensure!(
        after_reopen.writer_epoch.is_none(),
        "writer_epoch should remain NULL before envelope assignment"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-020 PASS");
    Ok(())
}

fn repo_root() -> Result<&'static Path> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .context("daemon crate should live two levels below the repository root")
}

fn ts(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> Result<DateTime<Utc>> {
    Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
        .single()
        .context("invalid timestamp")
}

fn seed_canonical_calibration_rows(
    conn: &Connection,
    adjudicated_at: &str,
) -> Result<Vec<CalibrationCaseSeed>> {
    let cases = canonical_calibration_cases();
    seed_calibration_corpus_cases(conn, &cases, adjudicated_at)?;
    Ok(cases)
}

fn assert_calibration_fixture_surface(case: &CalibrationCaseSeed) -> Result<()> {
    let artifact_path = repo_root()?.join(case.source_artifact_path());
    ensure!(
        artifact_path.exists(),
        "expected calibration artifact to exist for {}: {}",
        case.case_id(),
        artifact_path.display()
    );
    let metadata_path = repo_root()?
        .join("tests/fixtures/calibration")
        .join(case.slice_id())
        .join(case.corpus_kind())
        .join("metadata.json");
    ensure!(
        metadata_path.exists(),
        "expected calibration metadata fixture for {}: {}",
        case.case_id(),
        metadata_path.display()
    );
    Ok(())
}

fn assert_required_calibration_metadata(case: &CalibrationCaseSeed) -> Result<()> {
    let metadata = case.metadata_json();
    for required in [
        "case_id",
        "slice_id",
        "corpus_kind",
        "expected_verdict",
        "ground_truth_label",
        "mutation_family",
        "pair_id",
        "model_family",
        "source_artifact_path",
    ] {
        ensure!(
            metadata.get(required).is_some(),
            "case {} missing required metadata field `{required}`",
            case.case_id()
        );
    }
    ensure!(metadata["case_id"] == case.case_id());
    ensure!(metadata["slice_id"] == case.slice_id());
    ensure!(metadata["corpus_kind"] == case.corpus_kind());
    ensure!(metadata["expected_verdict"] == case.expected_verdict());
    ensure!(metadata["ground_truth_label"] == case.ground_truth_label());
    ensure!(metadata["source_artifact_path"] == case.source_artifact_path());
    Ok(())
}

fn assert_calibration_corpus_count(
    corpus_kind: CalibrationCorpusKind,
    min: usize,
    max: usize,
) -> Result<Vec<CalibrationCaseSeed>> {
    let state_db = L4TestStateDb::new()?;
    let cases = seed_canonical_calibration_rows(state_db.connection(), "2026-05-08T03:00:00Z")?;

    for family in calibration_skill_families() {
        let rows = cases
            .iter()
            .filter(|case| {
                case.slice_id() == family.slice_id && case.corpus_kind() == corpus_kind.as_str()
            })
            .collect::<Vec<_>>();
        ensure!(
            (min..=max).contains(&rows.len()),
            "slice {} expected {min}-{max} {} cases, got {}",
            family.slice_id,
            corpus_kind.as_str(),
            rows.len()
        );
        for case in rows {
            assert_required_calibration_metadata(case)?;
            assert_calibration_fixture_surface(case)?;
        }

        let stored_count: i64 = state_db.connection().query_row(
            "\
            SELECT COUNT(*)
            FROM validation_calibration_cases
            WHERE slice_id = ?1 AND corpus_kind = ?2
            ",
            params![family.slice_id, corpus_kind.as_str()],
            |row| row.get(0),
        )?;
        let stored_count =
            usize::try_from(stored_count).context("stored calibration case count was negative")?;
        ensure!(
            (min..=max).contains(&stored_count),
            "SQLite row count for {} / {} outside expected range: {stored_count}",
            family.slice_id,
            corpus_kind.as_str()
        );
    }

    Ok(cases)
}

fn assert_mutation_family(family_id: &str, expected_issue_category: &str) -> Result<()> {
    let state_db = L4TestStateDb::new()?;
    let cases = seed_canonical_calibration_rows(state_db.connection(), "2026-05-08T03:00:00Z")?;
    let rows = cases
        .iter()
        .filter(|case| {
            case.corpus_kind() == CalibrationCorpusKind::Mutated.as_str()
                && case.mutation_family() == Some(family_id)
        })
        .collect::<Vec<_>>();
    ensure!(
        rows.len() >= calibration_skill_families().len(),
        "expected at least one mutated {family_id} case per skill family, got {}",
        rows.len()
    );
    for case in rows {
        ensure!(
            case.expected_verdict() == "rejected",
            "{family_id} case {} should expect rejection",
            case.case_id()
        );
        ensure!(
            case.expected_issue_category() == Some(expected_issue_category),
            "{family_id} case {} emitted wrong issue category: {:?}",
            case.case_id(),
            case.expected_issue_category()
        );
        assert_required_calibration_metadata(case)?;
    }
    Ok(())
}

fn seed_eligible_readiness_for_cross_model(conn: &Connection, slice_id: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO enforcement_readiness (
          slice_id, worker_skill_id, source_id, worker_behavior_version,
          validator_overlay_version, enforcement_state, last_evaluated_at,
          wilson_lower_bound_bps, false_pass_upper_bound_bps,
          adjudicated_count, audited_approvals_count,
          dominant_failure_families_json, missing_failure_families_json,
          demotion_thresholds_json, updated_at
        ) VALUES (
          ?1, 'validation/cross-model-worker', 'validation-calibration',
          '2026.05.06-1', '2026.05.06-1', 'eligible', '2026-05-08T03:00:00Z',
          8000, 300, 40, 70, '[]', '[]', '{}', '2026-05-08T03:00:00Z'
        )
        ON CONFLICT(slice_id) DO UPDATE SET
          enforcement_state = excluded.enforcement_state,
          last_evaluated_at = excluded.last_evaluated_at,
          updated_at = excluded.updated_at
        ",
        [slice_id],
    )?;
    Ok(())
}

fn seed_runner_promotion_process(conn: &Connection, process_id: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO procurement_processes (
          id, canonical_process_id, current_stage, title,
          latest_published_at, latest_updated_at, owner_id
        ) VALUES (?1, ?1, 'manifest', 'L4 runner promotion process',
                  '2026-05-08T00:00:00Z', '2026-05-08T00:00:00Z', 'local-owner')
        ",
        params![process_id],
    )?;
    Ok(())
}

fn seed_runner_promotion_worker(
    conn: &Connection,
    worker_job_id: &str,
    process_id: &str,
) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO workflow_jobs (
          id, kind, process_id, scope_kind, scope_id, validation_required,
          status, input_payload
        ) VALUES (?1, 'manifest', ?2, 'process', ?2, 1, 'running', ?3)
        ",
        params![
            worker_job_id,
            process_id,
            json!({"worker_job_id": worker_job_id}).to_string()
        ],
    )?;
    Ok(())
}

fn runner_provisional_request(
    worker_job_id: &str,
    process_id: &str,
    stage_run_id: &str,
) -> PersistEnforcementProvisionalRequest {
    PersistEnforcementProvisionalRequest {
        job_id: worker_job_id.to_owned(),
        idempotency_key: format!("stage-provisional-{worker_job_id}"),
        output_payload: json!({
            "worker_job_id": worker_job_id,
            "result": "candidate shared-visible payload",
        }),
        result_contract_blob_path: Some(PathBuf::from(format!(
            "artifacts/provisional/{worker_job_id}.json"
        ))),
        process_id: process_id.to_owned(),
        stage: "Manifest".to_owned(),
        scope_kind: "process".to_owned(),
        scope_id: process_id.to_owned(),
        stage_run_id: stage_run_id.to_owned(),
        owner_id: "local-owner".to_owned(),
    }
}

fn runner_validator_request(
    worker_job_id: &str,
    validator_job_id: &str,
    process_id: &str,
) -> ValidatorEnqueueRequest {
    ValidatorEnqueueRequest {
        validator_job_id: Some(validator_job_id.to_owned()),
        idempotency_key: Some(format!("validator-{worker_job_id}")),
        worker_job_id: worker_job_id.to_owned(),
        worker_session_id: Some(format!("session-{worker_job_id}")),
        process_id: Some(process_id.to_owned()),
        scope_kind: Some("process".to_owned()),
        scope_id: Some(process_id.to_owned()),
        continuation_chain_id: Some(format!("chain-{worker_job_id}")),
        run_manifest_path: PathBuf::from(format!("artifacts/manifests/{worker_job_id}.md")),
        run_manifest_hash: format!("manifest-hash-{worker_job_id}"),
        drift_delta_json: json!({"fingerprint_delta_bps": 0}),
        oqg_anomaly_summary: Vec::new(),
        representative_item_sample: json!({"worker_job_id": worker_job_id}),
        corpus_provenance_tag: Some("clean_real".to_owned()),
        purpose: ValidatorJobPurpose::LiveValidation,
        canary_case: None,
        retroactive_backlog: None,
    }
}

fn provisional_status(conn: &Connection, worker_job_id: &str) -> Result<Option<String>> {
    conn.query_row(
        "\
        SELECT validation_status
        FROM provisional_worker_outputs
        WHERE worker_job_id = ?1
        ",
        [worker_job_id],
        |row| row.get(0),
    )
    .optional()
    .context("loading provisional worker output status")
}

fn count_runner_rows(conn: &Connection, table: &str) -> Result<i64> {
    ensure!(
        matches!(
            table,
            "local_outbox"
                | "process_stage_heads"
                | "process_stage_status"
                | "provisional_worker_outputs"
                | "validation_verdicts"
                | "workflow_jobs"
        ),
        "unsupported runner row count table `{table}`"
    );
    let sql = format!("SELECT COUNT(*) FROM {table}");
    conn.query_row(&sql, [], |row| row.get(0))
        .with_context(|| format!("counting rows in {table}"))
}

fn assert_runner_staged_provisional(
    conn: &Connection,
    enqueued_job_id: &str,
    worker_job_id: &str,
    validator_job_id: &str,
) -> Result<()> {
    let (parent_job_id, validation_required): (Option<String>, i64) = conn.query_row(
        "SELECT parent_job_id, validation_required FROM workflow_jobs WHERE id = ?1",
        [validator_job_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    ensure!(enqueued_job_id == validator_job_id);
    ensure!(
        parent_job_id.as_deref() == Some(worker_job_id),
        "validator job was not linked to its worker parent"
    );
    ensure!(
        validation_required == 0,
        "validator sub-job unexpectedly requires recursive validation"
    );
    ensure!(
        provisional_status(conn, worker_job_id)?.as_deref() == Some("pending"),
        "runner did not stage a pending provisional output"
    );
    ensure!(
        count_runner_rows(conn, "local_outbox")? == 0,
        "staging provisional output created a push outbox row before approval"
    );
    ensure!(
        count_runner_rows(conn, "process_stage_heads")? == 0,
        "staging provisional output advanced a stage head before approval"
    );
    Ok(())
}

fn persist_runner_approved_verdict(
    runner: &JobRunner,
    state_db: &mut StateDb,
    worker_job_id: &str,
    validator_job_id: &str,
    validator_skill: ValidatorSkillBundle,
) -> Result<()> {
    runner.persist_validation_verdict(
        state_db,
        &PersistValidationVerdictRequest {
            verdict_id: "verdict-l4-runner-079".to_owned(),
            worker_job_id: worker_job_id.to_owned(),
            worker_session_id: Some("worker-session-l4-runner-079".to_owned()),
            validator_job_id: validator_job_id.to_owned(),
            validator_session_id: Some("validator-session-l4-runner-079".to_owned()),
            run_manifest_hash: "manifest-hash-l4-runner-079".to_owned(),
            validator_skill,
            payload: json!({
                "verdict": "approved",
                "basis_sufficiency": "high",
                "enforcement_eligible": true,
                "confidence_bps": 9800,
                "issues": [],
                "summary": "approved enforcement-active output",
            }),
            oqg_signals_summary: Some(json!({"oqg": "clean"})),
            incident_signature: None,
            quality_assessment_json: Some(json!({"quality": "approved"})),
            skill_improvement_suggestions_json: Some(json!([])),
            override_verdict: None,
            override_reason: None,
            override_by: None,
            override_at: None,
            verdict_rendered_at: Some("2026-05-08T00:00:00Z".to_owned()),
        },
    )?;
    Ok(())
}

fn persist_runner_rejected_verdict(
    runner: &JobRunner,
    state_db: &mut StateDb,
    worker_job_id: &str,
    validator_job_id: &str,
    validator_skill: ValidatorSkillBundle,
) -> Result<()> {
    runner.persist_validation_verdict(
        state_db,
        &PersistValidationVerdictRequest {
            verdict_id: "verdict-l4-feedback-077".to_owned(),
            worker_job_id: worker_job_id.to_owned(),
            worker_session_id: Some("worker-session-l4-feedback-077".to_owned()),
            validator_job_id: validator_job_id.to_owned(),
            validator_session_id: Some("validator-session-l4-feedback-077".to_owned()),
            run_manifest_hash: "manifest-hash-l4-feedback-077".to_owned(),
            validator_skill,
            payload: json!({
                "verdict": "rejected",
                "basis_sufficiency": "high",
                "enforcement_eligible": true,
                "confidence_bps": 9700,
                "issues": [
                    {
                        "severity": "critical",
                        "category": "wrong_source_domain",
                        "description": "Source URL belonged to a reseller mirror",
                        "evidence_refs": ["artifact://source-url"]
                    }
                ],
                "summary": "rejected enforcement-active output",
            }),
            oqg_signals_summary: Some(json!({"oqg": "suspect_source"})),
            incident_signature: Some("incident-source-identity-l4-feedback-077".to_owned()),
            quality_assessment_json: Some(json!({
                "dominant_failure_family": "source_identity",
                "slice_id": "procurement-ingestion:tenders-direct:2026.05.06-1",
                "override_estimator_policy": "biased_demotion_signal_only"
            })),
            skill_improvement_suggestions_json: Some(json!([
                {
                    "mutation_family": "source_identity",
                    "template": {
                        "replace_source_domain_with": "mirror.invalid",
                        "expected_verdict": "rejected"
                    }
                }
            ])),
            override_verdict: None,
            override_reason: None,
            override_by: None,
            override_at: None,
            verdict_rendered_at: Some("2026-05-08T00:10:00Z".to_owned()),
        },
    )?;
    Ok(())
}

fn runner_promotion_request(
    worker_job_id: &str,
    process_id: &str,
    stage_run_id: &str,
) -> PromoteProvisionalRequest {
    PromoteProvisionalRequest {
        worker_job_id: worker_job_id.to_owned(),
        idempotency_key: "promote-l4-runner-079".to_owned(),
        process_id: process_id.to_owned(),
        stage: "Manifest".to_owned(),
        scope_kind: "process".to_owned(),
        scope_id: process_id.to_owned(),
        stage_run_id: stage_run_id.to_owned(),
        owner_id: "local-owner".to_owned(),
        override_verdict: None,
        override_reason: None,
        override_by: None,
        override_at: None,
    }
}

fn assert_runner_approved_promotion(
    conn: &Connection,
    worker_job_id: &str,
    before_replay_outbox_count: i64,
) -> Result<()> {
    ensure!(
        provisional_status(conn, worker_job_id)?.as_deref() == Some("approved"),
        "approved verdict did not approve the provisional output"
    );
    ensure!(
        count_runner_rows(conn, "local_outbox")? == before_replay_outbox_count + 1,
        "approval did not write exactly one outbox row"
    );
    ensure!(
        count_runner_rows(conn, "process_stage_heads")? == 1,
        "approval did not advance one stage head"
    );
    Ok(())
}

fn assert_runner_promotion_replay(
    runner: &JobRunner,
    state_db: &mut StateDb,
    request: &PromoteProvisionalRequest,
    promotion_ack: &MutationAck,
    before_replay_outbox_count: i64,
) -> Result<()> {
    let replay_ack = runner.promote_enforcement_provisional_output(state_db, request)?;
    ensure!(
        replay_ack.visible_after_seq == promotion_ack.visible_after_seq,
        "promotion replay did not return the stored ack"
    );
    ensure!(
        count_runner_rows(state_db.connection(), "local_outbox")? == before_replay_outbox_count + 1,
        "promotion replay created a duplicate outbox row"
    );
    Ok(())
}

fn override_feedback_promotion_request(
    worker_job_id: &str,
    process_id: &str,
    stage_run_id: &str,
) -> PromoteProvisionalRequest {
    PromoteProvisionalRequest {
        worker_job_id: worker_job_id.to_owned(),
        idempotency_key: "override-feedback-l4-077".to_owned(),
        process_id: process_id.to_owned(),
        stage: "Manifest".to_owned(),
        scope_kind: "process".to_owned(),
        scope_id: process_id.to_owned(),
        stage_run_id: stage_run_id.to_owned(),
        owner_id: "local-owner".to_owned(),
        override_verdict: Some("approved".to_owned()),
        override_reason: Some("operator confirmed official-source evidence manually".to_owned()),
        override_by: Some("operator-l4-feedback".to_owned()),
        override_at: Some("2026-05-08T00:15:00Z".to_owned()),
    }
}

fn assert_override_feedback_rows(
    conn: &Connection,
    worker_job_id: &str,
    before_outbox_count: i64,
) -> Result<()> {
    assert_override_audit_and_promotion(conn, worker_job_id, before_outbox_count)?;
    assert_override_calibration_case(&load_override_calibration_case(conn)?)?;
    assert_override_incident_cluster(&load_override_incident_cluster(conn)?)?;
    Ok(())
}

fn assert_override_audit_and_promotion(
    conn: &Connection,
    worker_job_id: &str,
    before_outbox_count: i64,
) -> Result<()> {
    let (override_verdict, override_reason, override_by, override_at): (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = conn.query_row(
        "\
        SELECT override_verdict, override_reason, override_by, override_at
        FROM validation_verdicts
        WHERE id = 'verdict-l4-feedback-077'
        ",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    ensure!(override_verdict.as_deref() == Some("approved"));
    ensure!(
        override_reason
            .as_deref()
            .is_some_and(|value| value.contains("official-source"))
    );
    ensure!(override_by.as_deref() == Some("operator-l4-feedback"));
    ensure!(override_at.as_deref() == Some("2026-05-08T00:15:00Z"));
    ensure!(provisional_status(conn, worker_job_id)?.as_deref() == Some("approved"));
    ensure!(count_runner_rows(conn, "process_stage_heads")? == 1);
    ensure!(count_runner_rows(conn, "local_outbox")? == before_outbox_count + 1);
    Ok(())
}

struct OverrideCalibrationCaseRow {
    case_id: String,
    corpus_kind: String,
    expected_verdict: Option<String>,
    actual_verdict: Option<String>,
    ground_truth_label: Option<String>,
    mutation_family: Option<String>,
    metadata_json: String,
}

fn load_override_calibration_case(conn: &Connection) -> Result<OverrideCalibrationCaseRow> {
    conn.query_row(
        "\
        SELECT case_id, corpus_kind, expected_verdict, actual_verdict,
               ground_truth_label, mutation_family, metadata_json
        FROM validation_calibration_cases
        WHERE validation_verdict_id = 'verdict-l4-feedback-077'
        ",
        [],
        |row| {
            Ok(OverrideCalibrationCaseRow {
                case_id: row.get(0)?,
                corpus_kind: row.get(1)?,
                expected_verdict: row.get(2)?,
                actual_verdict: row.get(3)?,
                ground_truth_label: row.get(4)?,
                mutation_family: row.get(5)?,
                metadata_json: row.get(6)?,
            })
        },
    )
    .context("loading override calibration case")
}

fn assert_override_calibration_case(row: &OverrideCalibrationCaseRow) -> Result<()> {
    ensure!(row.case_id == "override-verdict-l4-feedback-077");
    ensure!(row.corpus_kind == "override");
    ensure!(row.expected_verdict.as_deref() == Some("rejected"));
    ensure!(row.actual_verdict.as_deref() == Some("approved"));
    ensure!(row.ground_truth_label.as_deref() == Some("operator_override"));
    ensure!(row.mutation_family.as_deref() == Some("source_identity"));
    let metadata: Value = serde_json::from_str(&row.metadata_json)?;
    ensure!(
        metadata
            .get("override_estimator_policy")
            .and_then(Value::as_str)
            == Some("biased_demotion_signal_only")
    );
    Ok(())
}

struct OverrideIncidentClusterRow {
    dominant_failure_family: Option<String>,
    affected_slice_ids_json: String,
    candidate_mutation_template_json: String,
    override_case_ids_json: String,
    calibration_case_count: i64,
    override_count: i64,
}

fn load_override_incident_cluster(conn: &Connection) -> Result<OverrideIncidentClusterRow> {
    conn.query_row(
        "\
        SELECT dominant_failure_family, affected_slice_ids_json,
               candidate_mutation_template_json, override_case_ids_json,
               calibration_case_count, override_count
        FROM validation_incident_clusters
        WHERE incident_signature = 'incident-source-identity-l4-feedback-077'
        ",
        [],
        |row| {
            Ok(OverrideIncidentClusterRow {
                dominant_failure_family: row.get(0)?,
                affected_slice_ids_json: row.get(1)?,
                candidate_mutation_template_json: row.get(2)?,
                override_case_ids_json: row.get(3)?,
                calibration_case_count: row.get(4)?,
                override_count: row.get(5)?,
            })
        },
    )
    .context("loading override incident cluster")
}

fn assert_override_incident_cluster(row: &OverrideIncidentClusterRow) -> Result<()> {
    ensure!(row.dominant_failure_family.as_deref() == Some("source_identity"));
    ensure!(
        row.affected_slice_ids_json
            .contains("procurement-ingestion:tenders-direct")
    );
    ensure!(
        row.candidate_mutation_template_json
            .contains("wrong_source_domain")
    );
    ensure!(
        row.override_case_ids_json
            .contains("override-verdict-l4-feedback-077")
    );
    ensure!(row.calibration_case_count == 1);
    ensure!(row.override_count == 1);
    Ok(())
}

fn assert_override_feedback_replay(
    state_db: &mut StateDb,
    request: &OverrideFeedbackRequest,
    first_ack: &MutationAck,
    before_outbox_count: i64,
) -> Result<()> {
    let replay_ack = ingest_override(state_db, request)?;
    ensure!(replay_ack.visible_after_seq == first_ack.visible_after_seq);
    ensure!(
        count_runner_rows(state_db.connection(), "local_outbox")? == before_outbox_count + 1,
        "override feedback replay created duplicate outbox"
    );
    ensure!(
        state_db.connection().query_row(
            "SELECT COUNT(*) FROM validation_calibration_cases WHERE validation_verdict_id = 'verdict-l4-feedback-077'",
            [],
            |row| row.get::<_, i64>(0)
        )? == 1,
        "override feedback replay created duplicate calibration case"
    );
    ensure!(
        state_db.connection().query_row(
            "SELECT override_count FROM validation_incident_clusters WHERE incident_signature = 'incident-source-identity-l4-feedback-077'",
            [],
            |row| row.get::<_, i64>(0)
        )? == 1,
        "override feedback replay incremented incident override_count"
    );
    Ok(())
}

fn read_repo_file(path: &str) -> Result<String> {
    std::fs::read_to_string(repo_root()?.join(path))
        .with_context(|| format!("reading repository file {path}"))
}

fn apply_push_batch_body() -> Result<String> {
    let migration = read_repo_file("infra/supabase/migrations/0005_sync_rpcs.sql")?;
    Ok(migration
        .split("CREATE OR REPLACE FUNCTION update_operator_decision")
        .next()
        .context("missing apply_push_batch section before update_operator_decision")?
        .to_owned())
}

struct OverlaySkillFixture<'a> {
    label: &'a str,
    dir: &'a str,
    skill_id: &'a str,
    worker_behavior_version: &'a str,
    checkpoint_kind: &'a str,
    domain_checks: &'a [&'a str],
}

const PROCUREMENT_DOMAIN_CHECKS: [&str; 6] = [
    "notice_completeness",
    "ocds_field_presence",
    "source_locale",
    "notice_type_consistency",
    "expiry_plausibility",
    "cpv_validity",
];
const SUPPLIER_DOMAIN_CHECKS: [&str; 6] = [
    "price_plausibility",
    "currency_correctness",
    "pack_basis_consistency",
    "seller_diversity",
    "listing_freshness",
    "regulated_goods_compliance_evidence",
];

const fn procurement_overlay_fixture() -> OverlaySkillFixture<'static> {
    OverlaySkillFixture {
        label: "procurement-ingestion",
        dir: "skills/validation/overlay-procurement-ingestion",
        skill_id: "validation/overlay-procurement-ingestion",
        worker_behavior_version: "procurement-ingestion/*@2026.05.06-1+1.0",
        checkpoint_kind: "procurement_ingestion_overlay_checkpoint",
        domain_checks: &PROCUREMENT_DOMAIN_CHECKS,
    }
}

const fn supplier_overlay_fixture() -> OverlaySkillFixture<'static> {
    OverlaySkillFixture {
        label: "supplier-enrichment",
        dir: "skills/validation/overlay-supplier-enrichment",
        skill_id: "validation/overlay-supplier-enrichment",
        worker_behavior_version: "supplier-enrichment/*@2026.05.06-1+1.0",
        checkpoint_kind: "supplier_enrichment_overlay_checkpoint",
        domain_checks: &SUPPLIER_DOMAIN_CHECKS,
    }
}

fn overlay_path(fixture: &OverlaySkillFixture<'_>, file: &str) -> String {
    format!("{}/{file}", fixture.dir)
}

fn assert_overlay_required_files(fixture: &OverlaySkillFixture<'_>) -> Result<()> {
    for file in [
        "SKILL.md",
        "result.schema.json",
        "checkpoint.schema.json",
        "canary.inputs.jsonl",
        "golden.assertions.yaml",
        "validator-rubric.yaml",
        "references/domain-checks.md",
    ] {
        let path = repo_root()?.join(overlay_path(fixture, file));
        ensure!(path.exists(), "{} is missing", path.display());
    }
    ensure!(
        repo_root()?.join(fixture.dir).join("references").is_dir(),
        "{} overlay references directory is missing",
        fixture.label
    );
    Ok(())
}

fn assert_overlay_frontmatter(fixture: &OverlaySkillFixture<'_>) -> Result<()> {
    let skill = read_repo_file(&overlay_path(fixture, "SKILL.md"))?;
    let required_fields = [
        format!("skill_id: {}", fixture.skill_id),
        "skill_version: 2026.05.06-1".to_owned(),
        "interaction_profile: headless_contract".to_owned(),
        "result_schema_version: 1.0".to_owned(),
        "checkpoint_schema_version: 1.0".to_owned(),
        "validator_overlay_version: 2026.05.06-1".to_owned(),
        "worker_behavior_versions:".to_owned(),
        fixture.worker_behavior_version.to_owned(),
        "model_profile: structured_extract_low_cost".to_owned(),
        "agent_cli: claude".to_owned(),
        "side_effect_class: read_only".to_owned(),
        "promotion_stage: candidate".to_owned(),
    ];
    for required in required_fields {
        ensure!(skill.contains(&required), "SKILL.md missing `{required}`");
    }
    Ok(())
}

fn assert_overlay_schema_covers_domain_checks(fixture: &OverlaySkillFixture<'_>) -> Result<()> {
    let result_schema: Value = serde_json::from_str(&read_repo_file(&overlay_path(
        fixture,
        "result.schema.json",
    ))?)?;
    let categories = result_schema
        .pointer("/properties/issues/items/properties/category/enum")
        .and_then(Value::as_array)
        .context("result schema missing issue category enum")?;
    for &check in fixture.domain_checks {
        ensure!(
            categories.iter().any(|value| value.as_str() == Some(check)),
            "result schema missing issue category `{check}`"
        );
    }

    let checkpoint_schema: Value = serde_json::from_str(&read_repo_file(&overlay_path(
        fixture,
        "checkpoint.schema.json",
    ))?)?;
    ensure!(
        checkpoint_schema
            .pointer("/properties/kind/const")
            .and_then(Value::as_str)
            == Some(fixture.checkpoint_kind),
        "checkpoint schema has the wrong checkpoint kind"
    );
    Ok(())
}

fn assert_overlay_canaries(fixture: &OverlaySkillFixture<'_>) -> Result<()> {
    let canaries = read_repo_file(&overlay_path(fixture, "canary.inputs.jsonl"))?;
    let mut corpora = BTreeSet::new();
    let mut checks = BTreeSet::new();
    for line in canaries.lines().filter(|line| !line.trim().is_empty()) {
        let row: Value = serde_json::from_str(line)?;
        corpora.insert(
            row.get("corpus")
                .and_then(Value::as_str)
                .context("canary row missing corpus")?
                .to_owned(),
        );
        for check in row
            .get("domain_checks")
            .and_then(Value::as_array)
            .context("canary row missing domain_checks")?
        {
            checks.insert(
                check
                    .as_str()
                    .context("domain check must be a string")?
                    .to_owned(),
            );
        }
    }
    ensure!(
        corpora
            == BTreeSet::from([
                "clean-real".to_owned(),
                "mutated-bad".to_owned(),
                "pairwise".to_owned(),
                "weird-valid".to_owned(),
            ]),
        "{} overlay canaries did not cover the required corpora",
        fixture.label
    );
    for &check in fixture.domain_checks {
        ensure!(checks.contains(check), "canaries missing `{check}`");
    }
    Ok(())
}

fn assert_overlay_golden_and_rubric(fixture: &OverlaySkillFixture<'_>) -> Result<()> {
    let golden = read_repo_file(&overlay_path(fixture, "golden.assertions.yaml"))?;
    let rubric = read_repo_file(&overlay_path(fixture, "validator-rubric.yaml"))?;
    let _: serde_yaml::Value = serde_yaml::from_str(&golden)?;
    let _: serde_yaml::Value = serde_yaml::from_str(&rubric)?;
    for &check in fixture.domain_checks {
        ensure!(
            golden.contains(check),
            "golden assertions missing `{check}`"
        );
        ensure!(rubric.contains(check), "rubric missing `{check}`");
    }
    Ok(())
}

fn assert_overlay_contract(fixture: &OverlaySkillFixture<'_>) -> Result<()> {
    println!("[ASSERT] overlay declares the six-file contract and references");
    assert_overlay_required_files(fixture)?;
    assert_overlay_frontmatter(fixture)?;

    println!("[ASSERT] schemas, canaries, golden assertions, and rubric cover domain checks");
    assert_overlay_schema_covers_domain_checks(fixture)?;
    assert_overlay_canaries(fixture)?;
    assert_overlay_golden_and_rubric(fixture)
}

fn bail_with_conflict<T>(message: &str, conflict: &impl std::fmt::Debug) -> Result<T> {
    anyhow::bail!("{message}: {conflict:?}")
}

async fn spawn_rpc_capture_server(
    response_json: Value,
) -> Result<(String, JoinHandle<Result<String>>)> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("binding one-shot RPC capture server")?;
    let address = listener
        .local_addr()
        .context("reading capture server addr")?;
    let response_body = serde_json::to_string(&response_json)?;
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener
            .accept()
            .await
            .context("accepting RPC client connection")?;
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 1024];
        let request_len = loop {
            let read = stream
                .read(&mut chunk)
                .await
                .context("reading captured HTTP request")?;
            ensure!(read > 0, "HTTP client closed before request was complete");
            buffer.extend_from_slice(&chunk[..read]);
            if let Some(request_len) = complete_http_request_len(&buffer)? {
                break request_len;
            }
        };
        buffer.truncate(request_len);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream
            .write_all(response.as_bytes())
            .await
            .context("writing captured HTTP response")?;
        String::from_utf8(buffer).context("captured HTTP request was not UTF-8")
    });
    Ok((format!("http://{address}"), handle))
}

fn complete_http_request_len(buffer: &[u8]) -> Result<Option<usize>> {
    let Some(header_end) = find_subsequence(buffer, b"\r\n\r\n") else {
        return Ok(None);
    };
    let header_text =
        str::from_utf8(&buffer[..header_end]).context("captured HTTP headers were not UTF-8")?;
    let content_length = header_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>())
        })
        .transpose()
        .context("invalid captured content-length")?
        .unwrap_or(0);
    let request_len = header_end + 4 + content_length;
    if buffer.len() >= request_len {
        Ok(Some(request_len))
    } else {
        Ok(None)
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn http_body(raw_request: &str) -> Result<&str> {
    raw_request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .context("captured HTTP request missing body separator")
}

#[test]
fn l4_t_sqlite_supabase_sync_021_writer_epoch_assigned_only_by_batch_and_retry_immutable()
-> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-021");
    println!("[SETUP] dispatchable outbox row with NULL writer_epoch");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OutboxHarness::new(&state_db)?;
    let mutated = harness.mutate_push_owned_fixture(PushOwnedFixture::new("l4-writer-epoch", 1))?;
    ensure!(
        harness
            .outbox_row_by_seq(mutated.local_seq)?
            .writer_epoch
            .is_none(),
        "writer_epoch was assigned before batch envelope creation"
    );

    println!("[EXEC] assign batch writer_epoch and mark retry");
    let batch = harness
        .assemble_batch(10, 42)?
        .expect("writer epoch batch should materialize");
    let retry = harness.mark_batch_retry(&batch.batch_id)?;

    println!("[ASSERT] outbox row is assigned once and retry keeps immutable batch epoch");
    ensure!(
        harness.outbox_row_by_seq(mutated.local_seq)?.writer_epoch == Some(42),
        "local_outbox writer_epoch was not assigned by batch creation"
    );
    ensure!(
        batch.writer_epoch == 42 && retry.writer_epoch == 42,
        "retry changed writer_epoch from the persisted batch envelope"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-021 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_022_enforcement_approval_promotes_provisional_atomically() -> Result<()>
{
    println!("[TEST_START] L4-T-sqlite_supabase_sync-022");
    println!("[SETUP] enforcement harness with an approved validator verdict");
    let state_db = L4TestStateDb::new()?;
    let mut harness = EnforcementHarness::new(&state_db)?;
    let worker_job_id = "l4-approved-worker-job";
    let before_outbox_count = harness.count_rows("local_outbox")?;
    let before_head_count = harness.count_rows("process_stage_heads")?;
    let before_status_count = harness.count_rows("process_stage_status")?;
    let before_verdict_count = harness.count_rows("validation_verdicts")?;

    println!("[EXEC] approve enforcement-active provisional worker output");
    harness.approve_provisional_output(worker_job_id)?;

    println!("[ASSERT] approved provisional output promotes with outbox and stage head");
    let linked_verdict_id: Option<String> = state_db.connection().query_row(
        "\
        SELECT validation_verdict_id
        FROM workflow_jobs
        WHERE id = ?1
        ",
        [worker_job_id],
        |row| row.get(0),
    )?;
    ensure!(
        harness
            .provisional_validation_status(worker_job_id)?
            .as_deref()
            == Some("approved"),
        "approved worker output did not update provisional status"
    );
    ensure!(
        harness.count_rows("validation_verdicts")? == before_verdict_count + 1,
        "approved validator verdict was not logged"
    );
    ensure!(
        linked_verdict_id.as_deref() == Some("verdict-l4-approved-worker-job"),
        "approved worker job did not link the validator verdict"
    );
    ensure!(
        harness.count_rows("local_outbox")? > before_outbox_count,
        "approved enforcement output did not create an outbox row"
    );
    ensure!(
        harness.count_rows("process_stage_heads")? > before_head_count,
        "approved enforcement output did not advance stage heads"
    );
    ensure!(
        harness.count_rows("process_stage_status")? > before_status_count,
        "approved enforcement output did not persist stage status"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-022 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_023_enforcement_rejection_emits_agent_suspect_output_manifest()
-> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-023");
    println!("[SETUP] enforcement harness, redaction sentinels, and error manifest store");
    let state_db = L4TestStateDb::new()?;
    let fixture = RedactionSentinelFixture::new();
    seed_error_manifest_process(state_db.connection(), &fixture.process_id)?;
    let mut harness = EnforcementHarness::new(&state_db)?;
    let before_outbox_count = harness.count_rows("local_outbox")?;

    println!("[EXEC] reject enforcement-active worker output and create operator-alert manifest");
    harness.reject_provisional_output(&fixture.job_id)?;
    let artifact_root = state_db
        .db_path()
        .parent()
        .context("L4 state DB path should have a temp parent")?
        .join("artifacts");
    let store = ErrorManifestStore::new(artifact_root)?;
    let stored = store.create(
        state_db.connection(),
        &enforcement_suspect_draft(&fixture),
        Utc::now(),
    )?;

    println!(
        "[ASSERT] rejection remains local-only and surfaces AGENT_SUSPECT_OUTPUT operator alert"
    );
    let record = ErrorManifestStore::load_record(state_db.connection(), &stored.id)?;
    let manifest_count = count_error_manifests_by_type(
        state_db.connection(),
        &fixture.job_id,
        ErrorCode::AgentSuspectOutput,
    )?;
    let markdown = std::fs::read_to_string(&stored.markdown_path)
        .with_context(|| format!("reading {}", stored.markdown_path.display()))?;
    ensure!(
        record.error_type == ErrorCode::AgentSuspectOutput,
        "enforcement rejection was classified as {:?}",
        record.error_type
    );
    ensure!(
        manifest_count == 1,
        "expected one AGENT_SUSPECT_OUTPUT manifest, found {manifest_count}"
    );
    ensure!(
        harness
            .provisional_validation_status(&fixture.job_id)?
            .as_deref()
            == Some("rejected"),
        "provisional worker output did not stay rejected"
    );
    ensure!(
        harness.count_rows("local_outbox")? == before_outbox_count,
        "rejected enforcement output created a push outbox row"
    );
    ensure!(
        markdown.contains("## Operator Alert")
            && markdown.contains("AGENT_SUSPECT_OUTPUT")
            && markdown.contains("Do not promote or outbox this worker output"),
        "operator alert markdown did not contain the quarantine guidance"
    );
    fixture.assert_no_sentinels(&markdown)?;

    println!("[CLEANUP] tempfile-owned state DB and artifacts");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-023 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_024_dry_run_rejection_uses_l3_direct_write_path() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-024");
    println!("[SETUP] enforcement harness in dry-run rejection mode");
    let state_db = L4TestStateDb::new()?;
    let mut harness = EnforcementHarness::new(&state_db)?;
    let worker_job_id = "l4-dry-run-rejected-worker";
    let before_provisional_count = harness.count_rows("provisional_worker_outputs")?;
    let before_outbox_count = harness.count_rows("local_outbox")?;
    let before_head_count = harness.count_rows("process_stage_heads")?;
    let before_verdict_count = harness.count_rows("validation_verdicts")?;

    println!("[EXEC] reject validator output while slice is dry-run/non-enforcement");
    harness.dry_run_rejected_direct_write(worker_job_id)?;

    println!("[ASSERT] dry-run rejection records verdict and still advances L3 write path");
    let (linked_verdict_id, output_payload): (Option<String>, Option<String>) =
        state_db.connection().query_row(
            "\
            SELECT validation_verdict_id, output_payload
            FROM workflow_jobs
            WHERE id = ?1
            ",
            [worker_job_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
    let output: Value = serde_json::from_str(
        output_payload
            .as_deref()
            .context("dry-run worker output payload missing")?,
    )?;
    ensure!(
        harness.count_rows("provisional_worker_outputs")? == before_provisional_count,
        "dry-run rejection wrote to the provisional buffer"
    );
    ensure!(
        harness.count_rows("validation_verdicts")? == before_verdict_count + 1,
        "dry-run rejected validator verdict was not logged"
    );
    ensure!(
        linked_verdict_id.as_deref() == Some("verdict-l4-dry-run-rejected-worker"),
        "dry-run worker job did not link the validator verdict"
    );
    ensure!(
        output.get("validation_marker").and_then(Value::as_str) == Some("VALIDATION_FLAGGED"),
        "dry-run rejected worker output did not keep the L3 flagged marker"
    );
    ensure!(
        output.get("validation_marker").and_then(Value::as_str) != Some("AGENT_SUSPECT_OUTPUT"),
        "dry-run rejection was misclassified as enforcement quarantine"
    );
    ensure!(
        harness.count_rows("local_outbox")? > before_outbox_count,
        "dry-run rejection did not create an outbox row"
    );
    ensure!(
        harness.count_rows("process_stage_heads")? > before_head_count,
        "dry-run rejection did not advance stage heads"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-024 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_058_procurement_ingestion_overlay_contract() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-058");
    println!("[SETUP] procurement-ingestion overlay fixture");

    assert_overlay_contract(&procurement_overlay_fixture())?;

    println!("[TEST_END] L4-T-sqlite_supabase_sync-058 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_059_supplier_enrichment_overlay_contract() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-059");
    println!("[SETUP] supplier-enrichment overlay fixture");

    assert_overlay_contract(&supplier_overlay_fixture())?;

    println!("[TEST_END] L4-T-sqlite_supabase_sync-059 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_062_each_skill_family_has_clean_real_cases() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-062");
    println!("[SETUP] canonical calibration corpus");
    assert_calibration_corpus_count(CalibrationCorpusKind::Clean, 25, 30)?;
    println!("[TEST_END] L4-T-sqlite_supabase_sync-062 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_063_each_skill_family_has_weird_valid_cases() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-063");
    println!("[SETUP] canonical calibration corpus");
    let cases = assert_calibration_corpus_count(CalibrationCorpusKind::Weird, 15, 20)?;

    println!("[ASSERT] weird-valid set covers edge cases without forced rejection");
    for family in calibration_skill_families() {
        let mut weird_families = BTreeSet::new();
        for case in cases.iter().filter(|case| {
            case.slice_id() == family.slice_id
                && case.corpus_kind() == CalibrationCorpusKind::Weird.as_str()
        }) {
            ensure!(
                case.expected_verdict() == "approved",
                "weird-valid case {} must remain approvable",
                case.case_id()
            );
            let metadata = case.metadata_json();
            let weird_family = metadata
                .get("weird_family")
                .and_then(Value::as_str)
                .context("weird-valid metadata missing weird_family")?;
            weird_families.insert(weird_family.to_owned());
        }
        ensure!(
            weird_families
                == BTreeSet::from([
                    "layout_change".to_owned(),
                    "localization".to_owned(),
                    "partial_honest_data".to_owned(),
                    "low_result_count".to_owned(),
                ]),
            "slice {} missing weird-valid coverage: {weird_families:?}",
            family.slice_id
        );
    }

    println!("[TEST_END] L4-T-sqlite_supabase_sync-063 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_064_each_skill_family_has_mutated_bad_cases() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-064");
    println!("[SETUP] canonical calibration corpus");
    let cases = assert_calibration_corpus_count(CalibrationCorpusKind::Mutated, 30, 50)?;

    println!("[ASSERT] every mutation family appears for every skill slice");
    for family in calibration_skill_families() {
        let families = cases
            .iter()
            .filter(|case| {
                case.slice_id() == family.slice_id
                    && case.corpus_kind() == CalibrationCorpusKind::Mutated.as_str()
            })
            .filter_map(|case| case.mutation_family().map(str::to_owned))
            .collect::<BTreeSet<_>>();
        let expected = calibration_mutation_families()
            .iter()
            .map(|family| family.family_id.to_owned())
            .collect::<BTreeSet<_>>();
        ensure!(
            families == expected,
            "slice {} mutated corpus missing mutation coverage: {families:?}",
            family.slice_id
        );
    }

    println!("[TEST_END] L4-T-sqlite_supabase_sync-064 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_065_wrong_source_domain_rejects_source_identity() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-065");
    assert_mutation_family("wrong_source_domain", "source_identity")?;
    println!("[TEST_END] L4-T-sqlite_supabase_sync-065 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_066_wrong_page_kind_rejects_page_kind() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-066");
    assert_mutation_family("wrong_page_kind", "page_kind")?;
    println!("[TEST_END] L4-T-sqlite_supabase_sync-066 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_067_locale_currency_swap_rejects_currency_locale() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-067");
    assert_mutation_family("locale_currency_swap", "currency_locale")?;
    println!("[TEST_END] L4-T-sqlite_supabase_sync-067 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_068_pack_unit_distortion_rejects_unit_basis() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-068");
    assert_mutation_family("pack_unit_distortion", "unit_basis")?;
    println!("[TEST_END] L4-T-sqlite_supabase_sync-068 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_069_accessory_primary_confusion_rejects_product_identity() -> Result<()>
{
    println!("[TEST_START] L4-T-sqlite_supabase_sync-069");
    assert_mutation_family("accessory_primary_confusion", "product_identity")?;
    println!("[TEST_END] L4-T-sqlite_supabase_sync-069 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_070_selective_warning_omission_rejects_warning_omission() -> Result<()>
{
    println!("[TEST_START] L4-T-sqlite_supabase_sync-070");
    assert_mutation_family("selective_warning_omission", "warning_omission")?;
    println!("[TEST_END] L4-T-sqlite_supabase_sync-070 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_071_count_mismatch_rejects_count_mismatch() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-071");
    assert_mutation_family("count_mismatch", "count_mismatch")?;
    println!("[TEST_END] L4-T-sqlite_supabase_sync-071 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_072_fabricated_evidence_rejects_fabricated_evidence() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-072");
    assert_mutation_family("fabricated_evidence", "fabricated_evidence")?;
    println!("[TEST_END] L4-T-sqlite_supabase_sync-072 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_073_cross_segment_duplicates_gaps_rejects_continuation_integrity()
-> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-073");
    assert_mutation_family("cross_segment_duplicates_gaps", "continuation_integrity")?;
    println!("[TEST_END] L4-T-sqlite_supabase_sync-073 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_074_pairwise_siblings_share_pair_id_and_rank_trust() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-074");
    let state_db = L4TestStateDb::new()?;
    let cases = seed_canonical_calibration_rows(state_db.connection(), "2026-05-08T03:00:00Z")?;

    println!("[ASSERT] pairwise clean/corrupted siblings share pair IDs with trust ordering");
    let mut by_pair = BTreeMap::<String, Vec<&CalibrationCaseSeed>>::new();
    for case in cases
        .iter()
        .filter(|case| case.corpus_kind() == CalibrationCorpusKind::Pairwise.as_str())
    {
        by_pair
            .entry(
                case.pair_id()
                    .context("pairwise case missing pair_id")?
                    .to_owned(),
            )
            .or_default()
            .push(case);
    }
    ensure!(
        by_pair.len() >= calibration_mutation_families().len() * calibration_skill_families().len(),
        "pairwise corpus should include at least one pair per mutation family and skill slice"
    );
    for (pair_id, siblings) in by_pair {
        ensure!(
            siblings.len() == 2,
            "pair {pair_id} should have exactly two siblings, got {}",
            siblings.len()
        );
        let clean = siblings
            .iter()
            .find(|case| case.expected_verdict() == "approved")
            .context("pair missing clean approvable sibling")?;
        let corrupted = siblings
            .iter()
            .find(|case| case.expected_verdict() == "rejected")
            .context("pair missing corrupted rejected sibling")?;
        ensure!(
            corrupted.trust_rank() < clean.trust_rank(),
            "corrupted sibling {} should rank less trustworthy than clean sibling {}",
            corrupted.case_id(),
            clean.case_id()
        );
    }

    println!("[TEST_END] L4-T-sqlite_supabase_sync-074 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_076_cross_model_audit_cron_dispatches_and_records_disagreement()
-> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-076");
    let state_db = L4TestStateDb::new()?;
    let cases = seed_canonical_calibration_rows(state_db.connection(), "2026-05-08T03:00:00Z")?;
    let active_slice = cases
        .first()
        .context("canonical calibration corpus should not be empty")?
        .slice_id()
        .to_owned();
    seed_eligible_readiness_for_cross_model(state_db.connection(), &active_slice)?;

    println!("[EXEC] run daily validation cross-model cron through CronScheduler");
    let registry = SourceRegistry::load(state_db.connection())?;
    let mut scheduler = CronScheduler::from_registry(&registry);
    scheduler.refresh_next_run_at(ts(2026, 5, 8, 3, 29, 0)?)?;
    let dispatches = scheduler.dispatch_due_jobs(&registry, ts(2026, 5, 8, 3, 31, 0)?)?;
    let dispatch = dispatches
        .iter()
        .find(|dispatch| {
            dispatch.definition.source_id == VALIDATION_CALIBRATION_SOURCE_ID
                && dispatch.definition.skill_id == VALIDATION_CROSS_MODEL_AUDIT_SKILL_ID
        })
        .context("expected validation cross-model audit cron dispatch")?;
    ensure!(
        dispatch.job_class == CronJobClass::ValidationCalibration,
        "cross-model audit should use validation calibration job class"
    );
    ensure!(
        dispatch.enqueue_request.kind == "validation_cross_model_audit",
        "cross-model audit dispatched wrong job kind: {}",
        dispatch.enqueue_request.kind
    );
    ensure!(
        dispatch.enqueue_request.scope_kind.as_deref() == Some("validation_calibration"),
        "cross-model audit should be scoped to validation calibration"
    );
    ensure!(
        dispatch.enqueue_request.input_payload["alternate_model_family"]
            == CALIBRATION_MODEL_FAMILY_ALTERNATE,
        "cross-model audit should freeze alternate model family in payload"
    );

    println!("[EXEC] select active slice with new traffic and record alternate-model result");
    let selections =
        select_cross_model_audit_slices(state_db.connection(), "2026-05-07T03:31:00Z")?;
    let selection = selections
        .iter()
        .find(|selection| selection.slice_id == active_slice)
        .context("active enforcement slice with new traffic was not selected")?;
    let case_id = record_cross_model_audit_case(
        state_db.connection(),
        selection,
        "rejected",
        "2026-05-08T03:31:00Z",
    )?;
    let (corpus_kind, model_family, metadata_raw): (String, String, String) =
        state_db.connection().query_row(
            "\
            SELECT corpus_kind, model_family, metadata_json
            FROM validation_calibration_cases
            WHERE case_id = ?1
            ",
            [case_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    let metadata: Value = serde_json::from_str(&metadata_raw)?;
    ensure!(corpus_kind == "cross_model");
    ensure!(model_family == CALIBRATION_MODEL_FAMILY_ALTERNATE);
    ensure!(
        metadata.get("disagreement").and_then(Value::as_bool) == Some(true),
        "cross-model audit should record primary/alternate disagreement metadata"
    );
    ensure!(
        metadata
            .get("active_traffic_window_days")
            .and_then(Value::as_i64)
            == Some(30),
        "cross-model audit should document the once-per-30-days active traffic window"
    );

    println!("[TEST_END] L4-T-sqlite_supabase_sync-076 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_077_operator_override_feedback_promotes_and_calibrates() -> Result<()>
{
    println!("[TEST_START] L4-T-sqlite_supabase_sync-077");
    println!("[SETUP] enforcement-active provisional output rejected by validator");
    let temp = tempfile::tempdir()?;
    let db_path = temp.path().join("state.db");
    let mut state_db = StateDb::open(&db_path)?;
    let runner = JobRunner::new("l4-override-feedback");
    let worker_job_id = "l4-feedback-worker-077";
    let validator_job_id = "l4-feedback-validator-077";
    let process_id = "process-l4-feedback-077";
    let stage_run_id = "stage-run-l4-feedback-077";
    seed_runner_promotion_process(state_db.connection(), process_id)?;
    seed_runner_promotion_worker(state_db.connection(), worker_job_id, process_id)?;
    let provisional_request = runner_provisional_request(worker_job_id, process_id, stage_run_id);
    let validator_request = runner_validator_request(worker_job_id, validator_job_id, process_id);
    let enqueued = runner.persist_enforcement_provisional_and_enqueue_validator(
        repo_root()?,
        &mut state_db,
        &provisional_request,
        &validator_request,
    )?;
    persist_runner_rejected_verdict(
        &runner,
        &mut state_db,
        worker_job_id,
        validator_job_id,
        enqueued.skill,
    )?;

    println!("[EXEC] operator override ingests promotion feedback in one write path");
    let before_outbox_count = count_runner_rows(state_db.connection(), "local_outbox")?;
    let feedback_request = OverrideFeedbackRequest {
        executor_id: "l4-override-feedback".to_owned(),
        validation_verdict_id: "verdict-l4-feedback-077".to_owned(),
        promotion: override_feedback_promotion_request(worker_job_id, process_id, stage_run_id),
        slice_id: "procurement-ingestion:tenders-direct:2026.05.06-1".to_owned(),
        source_artifact_path: Some("artifacts/manifests/l4-feedback-077.md".to_owned()),
        dominant_failure_family: "source_identity".to_owned(),
        candidate_mutation_template_json: json!({
            "mutation_family": "source_identity",
            "issue_category": "wrong_source_domain",
            "template_source": "operator_override",
            "replace_source_domain_with": "mirror.invalid"
        }),
    };
    let first_ack = ingest_override(&mut state_db, &feedback_request)?;

    println!("[ASSERT] override audit, promotion, calibration case, and incident cluster landed");
    assert_override_feedback_rows(state_db.connection(), worker_job_id, before_outbox_count)?;
    assert_override_feedback_replay(
        &mut state_db,
        &feedback_request,
        &first_ack,
        before_outbox_count,
    )?;

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-077 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_079_runner_promotes_only_after_approved_verdict() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-079");
    println!("[SETUP] runner-owned enforcement worker and validator request");
    let temp = tempfile::tempdir()?;
    let db_path = temp.path().join("state.db");
    let mut state_db = StateDb::open(&db_path)?;
    let runner = JobRunner::new("l4-runner-promotion");
    let worker_job_id = "l4-runner-worker-079";
    let validator_job_id = "l4-runner-validator-079";
    let process_id = "process-l4-runner-079";
    let stage_run_id = "stage-run-l4-runner-079";
    seed_runner_promotion_process(state_db.connection(), process_id)?;
    seed_runner_promotion_worker(state_db.connection(), worker_job_id, process_id)?;
    let provisional_request = runner_provisional_request(worker_job_id, process_id, stage_run_id);
    let validator_request = runner_validator_request(worker_job_id, validator_job_id, process_id);

    println!("[EXEC] stage provisional worker output and enqueue linked validator job");
    let enqueued = runner.persist_enforcement_provisional_and_enqueue_validator(
        repo_root()?,
        &mut state_db,
        &provisional_request,
        &validator_request,
    )?;

    println!("[ASSERT] runner staged local provisional output without advancing shared heads");
    assert_runner_staged_provisional(
        state_db.connection(),
        &enqueued.job_id,
        worker_job_id,
        validator_job_id,
    )?;

    println!("[EXEC] persist approved validator verdict and promote provisional output");
    persist_runner_approved_verdict(
        &runner,
        &mut state_db,
        worker_job_id,
        validator_job_id,
        enqueued.skill,
    )?;
    let before_replay_outbox_count = count_runner_rows(state_db.connection(), "local_outbox")?;
    let promotion_request = runner_promotion_request(worker_job_id, process_id, stage_run_id);
    let promotion_ack =
        runner.promote_enforcement_provisional_output(&mut state_db, &promotion_request)?;

    println!("[ASSERT] approved promotion advances heads and replays idempotently");
    assert_runner_approved_promotion(
        state_db.connection(),
        worker_job_id,
        before_replay_outbox_count,
    )?;
    assert_runner_promotion_replay(
        &runner,
        &mut state_db,
        &promotion_request,
        &promotion_ack,
        before_replay_outbox_count,
    )?;

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-079 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_025_pull_cursors_are_independent_per_remote_domain() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-025");
    println!("[SETUP] initialize pull cursor harness");
    let state_db = L4TestStateDb::new()?;
    let harness = PullCursorHarness::new(&state_db)?;

    println!("[ASSERT] all remote-owned domains start at seq 0");
    let initial = harness.all()?;
    ensure!(
        initial.len() == 3,
        "pull cursor initialization seeded unexpected domain count: {initial:?}"
    );
    ensure!(
        initial
            .iter()
            .all(|cursor| cursor.last_remote_change_seq == 0 && cursor.last_pulled_at.is_none()),
        "pull cursors must start unadvanced and unpulled: {initial:?}"
    );

    println!("[EXEC] advance source config cursor and fail company profile pull");
    let advanced = harness.record_success(PullCursorDomain::SharedSourceConfigs, 17)?;
    let failed_company = harness.record_failure(PullCursorDomain::SharedCompanyProfile)?;
    let decision_head = harness.load(PullCursorDomain::OperatorDecisionHeads)?;

    println!("[ASSERT] success advances only its domain and failure leaves its cursor unchanged");
    ensure!(
        advanced.last_remote_change_seq == 17 && advanced.last_pulled_at.is_some(),
        "successful pull did not advance source config cursor: {advanced:?}"
    );
    ensure!(
        failed_company.last_remote_change_seq == 0 && failed_company.last_pulled_at.is_none(),
        "failed company profile pull advanced or marked its cursor: {failed_company:?}"
    );
    ensure!(
        decision_head.last_remote_change_seq == 0 && decision_head.last_pulled_at.is_none(),
        "unrelated decision-head cursor changed: {decision_head:?}"
    );

    println!("[EXEC] advance a second domain");
    let company = harness.record_success(PullCursorDomain::SharedCompanyProfile, 3)?;
    let source = harness.load(PullCursorDomain::SharedSourceConfigs)?;

    println!("[ASSERT] second-domain success does not disturb first-domain high-water");
    ensure!(
        company.last_remote_change_seq == 3 && company.last_pulled_at.is_some(),
        "company profile cursor did not advance independently: {company:?}"
    );
    ensure!(
        source.last_remote_change_seq == 17,
        "source config cursor changed during company profile pull: {source:?}"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-025 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_026_hiwater_pagination_defers_concurrent_remote_writes() -> Result<()>
{
    println!("[TEST_START] L4-T-sqlite_supabase_sync-026");
    println!("[SETUP] seed three source config changes before the pull starts");
    let state_db = L4TestStateDb::new()?;
    let harness = PullCursorHarness::new(&state_db)?;
    let domain = PullCursorDomain::SharedSourceConfigs;
    harness.seed_remote_owned_change(domain, "source-a", &json!({"source_id": "source-a"}))?;
    harness.seed_remote_owned_change(domain, "source-b", &json!({"source_id": "source-b"}))?;
    harness.seed_remote_owned_change(domain, "source-c", &json!({"source_id": "source-c"}))?;

    println!("[EXEC] read hiwater, inject a concurrent write above it, then page with size 2");
    let first_pull = harness.pull_fenced_with_concurrent_change(
        domain,
        2,
        "source-d",
        &json!({"source_id": "source-d", "concurrent": true}),
    )?;

    println!("[ASSERT] first pull applies only rows inside cursor < seq <= hiwater");
    ensure!(
        first_pull.start_cursor == 0,
        "first pull started from a nonzero cursor"
    );
    ensure!(
        first_pull.hiwater == 3,
        "first pull did not freeze the pre-injection hiwater: {first_pull:?}"
    );
    ensure!(
        first_pull.applied_entity_ids == ["source-a", "source-b", "source-c"],
        "first pull applied rows outside the fenced range or out of order: {first_pull:?}"
    );
    ensure!(
        first_pull.page_boundaries.iter().all(|boundary| {
            let uses_frozen_hiwater = boundary.hiwater_inclusive == first_pull.hiwater;
            let lower_stays_inside_fence = boundary.lower_exclusive < boundary.hiwater_inclusive;
            uses_frozen_hiwater && lower_stays_inside_fence && boundary.limit == 2
        }),
        "page requests were not bounded by the frozen hiwater: {first_pull:?}"
    );
    ensure!(
        harness.load(domain)?.last_remote_change_seq == first_pull.hiwater,
        "cursor did not advance exactly to the fenced hiwater"
    );
    ensure!(
        harness.applied_remote_entity_ids(domain)? == ["source-a", "source-b", "source-c"],
        "concurrent write was applied before the next pull"
    );

    println!("[EXEC] run the next fenced pull after the cursor advanced");
    let second_pull = harness.pull_fenced(domain, 2)?;

    println!("[ASSERT] next pull sees the deferred concurrent write without duplicates");
    ensure!(
        second_pull.start_cursor == 3 && second_pull.hiwater == 4,
        "second pull did not start after the prior hiwater and fence the deferred row: {second_pull:?}"
    );
    ensure!(
        second_pull.applied_entity_ids == ["source-d"],
        "second pull applied unexpected rows: {second_pull:?}"
    );
    ensure!(
        harness.applied_remote_change_seqs(domain)? == [1, 2, 3, 4],
        "applied remote_change_seq history skipped or duplicated a row"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-026 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_027_remote_change_seq_is_monotonic_and_non_reused_per_domain()
-> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-027");
    println!("[SETUP] initialize pull cursor harness and read remote-owned migration SQL");
    let state_db = L4TestStateDb::new()?;
    let harness = PullCursorHarness::new(&state_db)?;
    let migration_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("infra/supabase/migrations/0002_remote_owned_tables.sql");
    let migration_sql = std::fs::read_to_string(&migration_path)
        .context("reading remote-owned Supabase migration")?;

    for domain in PullCursorDomain::all() {
        println!("[EXEC] simulate insert, update, and tombstone writes for {domain:?}");
        let seqs = harness.exercise_remote_change_sequence(domain, "remote-owned-fixture")?;
        let unique = seqs.iter().copied().collect::<BTreeSet<_>>();

        println!("[ASSERT] {domain:?} sequence values strictly increase and are not reused");
        ensure!(
            seqs.len() == 3 && unique.len() == 3 && seqs.windows(2).all(|pair| pair[0] < pair[1]),
            "{domain:?} did not assign monotonic non-reused change seqs: {seqs:?}"
        );
        ensure!(
            harness.remote_change_history(domain)? == seqs,
            "{domain:?} history diverged from observed write sequence"
        );
    }

    println!("[ASSERT] Supabase migration uses per-domain sequences and INSERT/UPDATE triggers");
    for (domain, sequence_name) in [
        (
            PullCursorDomain::SharedCompanyProfile,
            "shared_company_profile_change_seq",
        ),
        (
            PullCursorDomain::SharedSourceConfigs,
            "shared_source_configs_change_seq",
        ),
        (
            PullCursorDomain::OperatorDecisionHeads,
            "operator_decision_heads_change_seq",
        ),
    ] {
        let table = domain.as_str();
        ensure!(
            migration_sql.contains(&format!("CREATE SEQUENCE {sequence_name} AS bigint")),
            "missing per-domain sequence {sequence_name}"
        );
        ensure!(
            migration_sql.contains(&format!(
                "remote_change_seq bigint NOT NULL DEFAULT nextval('{sequence_name}')"
            )),
            "missing default nextval for {table}"
        );
        ensure!(
            migration_sql.contains(&format!("BEFORE INSERT OR UPDATE ON {table}"))
                && migration_sql.contains(&format!(
                    "EXECUTE FUNCTION l4_remote_owned_touch('{sequence_name}')"
                )),
            "missing INSERT/UPDATE trigger binding for {table}"
        );
    }
    ensure!(
        migration_sql.contains("NEW.remote_change_seq := nextval(TG_ARGV[0]::regclass);"),
        "update/tombstone path does not force a fresh remote_change_seq"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-027 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_028_cycle_pulls_before_reconcile_cas_and_push() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-028");
    println!("[SETUP] seed remote-owned changes for all pull domains");
    let state_db = L4TestStateDb::new()?;
    let harness = PullCursorHarness::new(&state_db)?;
    for domain in PullCursorDomain::all() {
        harness.seed_remote_owned_change(
            domain,
            &format!("{}-fixture", domain.as_str()),
            &json!({"domain": domain.as_str()}),
        )?;
    }

    println!("[EXEC] run startup and scheduled sync cycles");
    let startup = harness.run_sync_cycle(SyncCycleTrigger::Startup)?;
    let scheduled = harness.run_sync_cycle(SyncCycleTrigger::Scheduled)?;

    println!("[ASSERT] every cycle orders all remote pulls before intent/CAS/push stages");
    for outcome in [startup, scheduled] {
        ensure!(
            outcome.sync_health == SyncHealth::Healthy,
            "successful pull-before-push cycle did not report healthy: {outcome:?}"
        );
        ensure!(
            outcome.cas_flush_allowed && outcome.push_dispatch_allowed,
            "successful pull-before-push cycle did not allow downstream dispatch: {outcome:?}"
        );
        ensure!(
            outcome.phases
                == [
                    SyncCyclePhase::RemotePullCompleted(PullCursorDomain::SharedSourceConfigs),
                    SyncCyclePhase::RemotePullCompleted(PullCursorDomain::SharedCompanyProfile),
                    SyncCyclePhase::RemotePullCompleted(PullCursorDomain::OperatorDecisionHeads),
                    SyncCyclePhase::PendingIntentReconciled,
                    SyncCyclePhase::CasFlushed,
                    SyncCyclePhase::PushDispatched,
                ],
            "sync cycle phases violated pull-before-push ordering: {outcome:?}"
        );
    }
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-028 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_029_pull_failure_pauses_cross_authority_dispatch_only() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-029");
    println!("[SETUP] initialize pull cursor harness with one source-config change");
    let state_db = L4TestStateDb::new()?;
    let harness = PullCursorHarness::new(&state_db)?;
    harness.seed_remote_owned_change(
        PullCursorDomain::SharedSourceConfigs,
        "source-before-failure",
        &json!({"source_id": "source-before-failure"}),
    )?;

    println!("[EXEC] fail the company-profile pull during a manual sync cycle");
    let degraded = harness.run_sync_cycle_with_pull_failure(
        SyncCycleTrigger::Manual,
        PullCursorDomain::SharedCompanyProfile,
    )?;

    println!("[ASSERT] CAS/push dispatch are paused and local SQLite operation stays available");
    ensure!(
        degraded.sync_health == SyncHealth::Degraded,
        "pull failure did not mark the sync cycle degraded: {degraded:?}"
    );
    ensure!(
        !degraded.cas_flush_allowed && !degraded.push_dispatch_allowed,
        "cross-authority dispatch continued after pull failure: {degraded:?}"
    );
    ensure!(
        degraded.local_operation_available,
        "local operation was not preserved in degraded mode: {degraded:?}"
    );
    ensure!(
        degraded.phases
            == [
                SyncCyclePhase::RemotePullCompleted(PullCursorDomain::SharedSourceConfigs),
                SyncCyclePhase::RemotePullFailed(PullCursorDomain::SharedCompanyProfile),
                SyncCyclePhase::CrossAuthorityDispatchPaused,
                SyncCyclePhase::LocalOperationContinues,
            ],
        "pull failure phases did not stop before reconciliation/CAS/push: {degraded:?}"
    );

    println!("[EXEC] write a local process row after degraded sync");
    harness.write_local_process_marker("l4-local-write-during-degraded-sync")?;

    println!("[ASSERT] local SQLite writes still succeed while cross-authority dispatch is paused");
    ensure!(
        harness.local_process_exists("l4-local-write-during-degraded-sync")?,
        "local SQLite operation was blocked by degraded sync state"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-029 PASS");
    Ok(())
}

#[tokio::test]
async fn l4_t_sqlite_supabase_sync_030_operator_decision_cas_rpc_applies_matching_base()
-> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-030");
    println!("[SETUP] queued operator intent with matching base revision");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OperatorDecisionCasHarness::new(&state_db)?;
    harness.seed_head("l4-cas-process-030", "pending", 7)?;
    harness.queue_pending_operator_intent(
        "l4-cas-process-030",
        "approve",
        7,
        "intent-030",
        "l4-cmd-030",
        "2026-05-08T03:00:00Z",
    )?;
    let (api_url, capture) = spawn_rpc_capture_server(json!({
        "accepted": true,
        "status": "applied",
        "process_id": "l4-cas-process-030",
        "command_id": "l4-cmd-030",
        "applied_revision": 8
    }))
    .await?;

    println!(
        "[EXEC] reconcile matching-base intent, then flush through the Supabase CAS RPC client"
    );
    let client = SupabaseRpcClient::new(api_url, "service-role-test-key")?;
    let plan = harness.prepare_pending_intents()?;
    ensure!(
        plan.superseded_intent_ids.is_empty()
            && plan.conflicted_intent_ids.is_empty()
            && plan.commands_to_flush.len() == 1,
        "reconciler did not produce exactly one matching-base command: {plan:?}"
    );
    let command = plan
        .commands_to_flush
        .first()
        .context("missing command from reconciliation plan")?;
    let rpc_response = client.update_operator_decision(command).await?;
    let raw_request = capture.await.context("joining capture server task")??;
    harness.record_pending_cas_response(&rpc_response)?;

    println!(
        "[ASSERT] reconciler emits the expected CAS command and local intent is marked applied"
    );
    ensure!(
        command.process_id == "l4-cas-process-030"
            && command.disposition == "approved"
            && command.base_revision == 7
            && command.command_id == "l4-cmd-030",
        "reconciler emitted the wrong CAS command: {command:?}"
    );
    ensure!(rpc_response.accepted, "CAS RPC response was not accepted");
    ensure!(
        rpc_response.applied_revision == Some(8),
        "CAS RPC response did not carry the applied remote revision"
    );
    ensure!(
        raw_request.starts_with("POST /rest/v1/rpc/update_operator_decision "),
        "daemon client did not call the update_operator_decision REST RPC: {raw_request}"
    );
    let body = http_body(&raw_request)?;
    let request_json: Value =
        serde_json::from_str(body).context("parsing captured update_operator_decision body")?;
    ensure!(
        request_json.get("process_id").and_then(Value::as_str) == Some("l4-cas-process-030"),
        "CAS request body did not carry process_id"
    );
    ensure!(
        request_json.get("disposition").and_then(Value::as_str) == Some("approved"),
        "CAS request body did not carry disposition"
    );
    ensure!(
        request_json.get("base_revision").and_then(Value::as_i64) == Some(7),
        "CAS request body did not carry base_revision"
    );
    ensure!(
        request_json.get("command_id").and_then(Value::as_str) == Some("l4-cmd-030"),
        "CAS request body did not carry command_id"
    );

    let intent = harness.pending_intent_status("intent-030")?;
    ensure!(
        intent.status == "applied" && intent.last_error.is_none(),
        "queued operator intent was not marked applied: {intent:?}"
    );

    println!("[CLEANUP] tempfile-owned state DB and one-shot capture server");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-030 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_031_concurrent_decisions_conflict_without_overwrite() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-031");
    println!("[SETUP] two queued operator intents from the same base revision");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OperatorDecisionCasHarness::new(&state_db)?;
    harness.seed_head("l4-cas-process-031", "pending", 9)?;
    harness.queue_pending_operator_intent(
        "l4-cas-process-031",
        "approve",
        9,
        "intent-031-a",
        "l4-cmd-031-a",
        "2026-05-08T03:01:00Z",
    )?;

    println!("[EXEC] reconcile and apply first command, then queue a stale concurrent command");
    let first_plan = harness.prepare_pending_intents()?;
    let first_command = first_plan
        .commands_to_flush
        .first()
        .context("missing first command")?;
    let first = harness.apply_pending_command_to_fixture(first_command)?;
    harness.record_pending_cas_response(&first)?;
    harness.queue_pending_operator_intent(
        "l4-cas-process-031",
        "reject",
        9,
        "intent-031-b",
        "l4-cmd-031-b",
        "2026-05-08T03:02:00Z",
    )?;
    let second_plan = harness.prepare_pending_intents()?;

    println!("[ASSERT] first command advances remote head and second is a local conflict");
    let head = harness.head("l4-cas-process-031")?;
    ensure!(first.accepted, "first CAS command should apply");
    ensure!(
        second_plan.commands_to_flush.is_empty()
            && second_plan.conflicted_intent_ids == ["intent-031-b"],
        "second stale intent should conflict before CAS flush: {second_plan:?}"
    );
    ensure!(
        head.disposition == "approved" && head.remote_revision == 10,
        "stale concurrent command overwrote the accepted decision: {head:?}"
    );
    let losing_intent = harness.pending_intent_status("intent-031-b")?;
    ensure!(
        losing_intent.status == "conflicted" && losing_intent.superseded_by_intent_id.is_none(),
        "stale concurrent intent was not left in conflicted state: {losing_intent:?}"
    );
    let conflict_json: Value = serde_json::from_str(
        losing_intent
            .last_error
            .as_deref()
            .context("conflicted intent did not persist conflict_json")?,
    )
    .context("parsing persisted conflict_json")?;
    ensure!(
        conflict_json
            .get("current_remote_revision")
            .and_then(Value::as_i64)
            == Some(10),
        "conflict_json did not carry the current remote head: {conflict_json}"
    );
    ensure!(
        conflict_json
            .get("base_remote_revision")
            .and_then(Value::as_i64)
            == Some(9),
        "conflict_json did not carry rejected payload details: {conflict_json}"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-031 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_032_coalesces_latest_pending_intent_per_process() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-032");
    println!("[SETUP] three unsuperseded intents for one process");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OperatorDecisionCasHarness::new(&state_db)?;
    harness.seed_head("l4-cas-process-032", "pending", 20)?;
    harness.queue_pending_operator_intent(
        "l4-cas-process-032",
        "approve",
        20,
        "intent-032-a",
        "l4-cmd-032-a",
        "2026-05-08T03:03:00Z",
    )?;
    harness.queue_pending_operator_intent(
        "l4-cas-process-032",
        "watch",
        20,
        "intent-032-b",
        "l4-cmd-032-b",
        "2026-05-08T03:04:00Z",
    )?;
    harness.queue_pending_operator_intent(
        "l4-cas-process-032",
        "reject",
        20,
        "intent-032-c",
        "l4-cmd-032-c",
        "2026-05-08T03:04:00Z",
    )?;

    println!("[EXEC] reconcile pending intents before CAS flush");
    let plan = harness.prepare_pending_intents()?;

    println!("[ASSERT] latest by queued_at DESC, id DESC wins and older rows are superseded");
    let superseded: BTreeSet<_> = plan.superseded_intent_ids.iter().cloned().collect();
    ensure!(
        superseded == BTreeSet::from(["intent-032-a".to_owned(), "intent-032-b".to_owned()]),
        "older intents were not superseded before flush: {plan:?}"
    );
    ensure!(
        plan.commands_to_flush.len() == 1
            && plan.commands_to_flush[0].command_id == "l4-cmd-032-c"
            && plan.commands_to_flush[0].disposition == "rejected",
        "reconciler did not flush only the newest command: {plan:?}"
    );
    for older_id in ["intent-032-a", "intent-032-b"] {
        let status = harness.pending_intent_status(older_id)?;
        ensure!(
            status.status == "superseded"
                && status.superseded_by_intent_id.as_deref() == Some("intent-032-c"),
            "older intent was not linked to the coalescing winner: {status:?}"
        );
    }
    let winner = harness.pending_intent_status("intent-032-c")?;
    ensure!(
        winner.status == "queued" && winner.superseded_by_intent_id.is_none(),
        "winning intent should remain queued until CAS response is recorded: {winner:?}"
    );

    println!("[EXEC] apply only the winning command and record the CAS response");
    let response = harness.apply_pending_command_to_fixture(&plan.commands_to_flush[0])?;
    harness.record_pending_cas_response(&response)?;

    println!("[ASSERT] only the winner applies through CAS");
    let head = harness.head("l4-cas-process-032")?;
    let winner = harness.pending_intent_status("intent-032-c")?;
    ensure!(
        head.disposition == "rejected" && head.remote_revision == 21,
        "winning command did not advance the fixture head: {head:?}"
    );
    ensure!(
        winner.status == "applied" && winner.last_error.is_none(),
        "winning intent was not marked applied: {winner:?}"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-032 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_033_conflict_response_drives_decision_conflict_event() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-033");
    println!("[SETUP] CAS RPC conflict response with current head and rejected payload");
    let response = support::sync::OperatorDecisionApplyResponse::from_rpc_value(json!({
        "accepted": false,
        "status": "conflict",
        "process_id": "l4-cas-process-033",
        "command_id": "l4-cmd-033",
        "current_revision": 12,
        "current_disposition": "watching",
        "rejected_payload": {
            "process_id": "l4-cas-process-033",
            "disposition": "approved",
            "base_revision": 11,
            "command_id": "l4-cmd-033"
        }
    }))?;

    println!("[EXEC] convert RPC conflict into IPC decision.conflict topic event");
    let current_remote_revision = response
        .current_revision
        .context("conflict response missing current revision for IPC event")?;
    let current_disposition = response
        .current_disposition
        .as_deref()
        .context("conflict response missing current disposition for IPC event")?;
    let rejected_payload = response
        .rejected_payload
        .as_ref()
        .context("conflict response missing rejected payload for IPC event")?;
    let event = decision_conflict_ipc_event(&DecisionConflictEventParams {
        process_id: &response.process_id,
        command_id: &response.command_id,
        current_remote_revision,
        current_disposition,
        rejected_payload,
    })?;

    println!("[ASSERT] decision.conflict event carries complete conflict details");
    ensure!(
        response.is_conflict(),
        "response should be classified as CAS conflict"
    );
    ensure!(
        event.topic == DECISION_CONFLICT_TOPIC,
        "IPC event did not route on the decision.conflict topic: {event:?}"
    );
    ensure!(
        event.payload.get("event").and_then(Value::as_str) == Some("decision.conflict"),
        "IPC payload did not identify the decision.conflict event"
    );
    ensure!(
        event.payload.get("process_id").and_then(Value::as_str) == Some("l4-cas-process-033")
            && event.payload.get("command_id").and_then(Value::as_str) == Some("l4-cmd-033"),
        "IPC payload lost CAS command identity: {}",
        event.payload
    );
    ensure!(
        event
            .payload
            .get("current_remote_revision")
            .and_then(Value::as_i64)
            == Some(12)
            && event
                .payload
                .get("current_disposition")
                .and_then(Value::as_str)
                == Some("watching"),
        "IPC payload lost current remote head details: {}",
        event.payload
    );
    ensure!(
        event
            .payload
            .get("rejected_payload")
            .and_then(|payload| payload.get("base_revision"))
            .and_then(Value::as_i64)
            == Some(11),
        "IPC payload lost rejected payload details: {}",
        event.payload
    );

    println!("[CLEANUP] pure payload conversion test");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-033 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_034_applied_cas_command_writes_command_ledger() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-034");
    println!("[SETUP] CAS command harness with matching base revision");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OperatorDecisionCasHarness::new(&state_db)?;
    harness.seed_head("l4-cas-process-034", "pending", 20)?;
    harness.queue_intent("l4-cas-process-034", "approved", 20, "l4-cmd-034")?;

    println!("[EXEC] flush matching-base command through ledger-aware CAS transaction");
    let response = harness.flush_next_intent()?;

    println!("[ASSERT] decision head, local intent, and command ledger update atomically");
    ensure!(response.accepted, "CAS command should apply");
    ensure!(
        response.applied_revision == Some(21),
        "applied response did not return the next remote revision: {response:?}"
    );
    let head = harness.head("l4-cas-process-034")?;
    ensure!(
        head.disposition == "approved" && head.remote_revision == 21,
        "decision head did not advance with applied command: {head:?}"
    );
    let intent = harness.intent_status("l4-cmd-034")?;
    ensure!(
        intent.status == "applied" && intent.applied_revision == Some(21),
        "operator intent was not marked applied: {intent:?}"
    );
    let ledger = harness.ledger_row("l4-cmd-034")?;
    ensure!(
        ledger.command_id == "l4-cmd-034"
            && ledger.process_id == "l4-cas-process-034"
            && ledger.base_revision == 20
            && ledger.applied_revision == Some(21)
            && ledger.disposition == "approved"
            && ledger.result == "applied",
        "applied command ledger row lost required fields: {ledger:?}"
    );
    let ledger_response: Value = serde_json::from_str(&ledger.response_payload_json)
        .context("parsing applied command ledger response JSON")?;
    ensure!(
        ledger_response.get("accepted").and_then(Value::as_bool) == Some(true)
            && ledger_response
                .get("applied_revision")
                .and_then(Value::as_i64)
                == Some(21),
        "applied command ledger response JSON is incomplete: {ledger_response}"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-034 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_035_replayed_command_returns_stored_result() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-035");
    println!("[SETUP] two applied commands followed by a lost-response retry");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OperatorDecisionCasHarness::new(&state_db)?;
    harness.seed_head("l4-cas-process-035", "pending", 30)?;
    harness.queue_intent("l4-cas-process-035", "approved", 30, "l4-cmd-035-a")?;
    harness.queue_intent("l4-cas-process-035", "rejected", 31, "l4-cmd-035-b")?;

    println!("[EXEC] apply command 1, apply command 2, then retry command 1");
    let first = harness.flush_next_intent()?;
    let second = harness.flush_next_intent()?;
    harness.queue_intent("l4-cas-process-035", "approved", 30, "l4-cmd-035-a")?;
    let replay = harness.flush_next_intent()?;

    println!("[ASSERT] retry returns command 1 ledger response without re-evaluating head");
    ensure!(
        first.accepted && first.applied_revision == Some(31),
        "first command did not apply as expected: {first:?}"
    );
    ensure!(
        second.accepted && second.applied_revision == Some(32),
        "second command did not apply as expected: {second:?}"
    );
    ensure!(
        replay.accepted && replay.applied_revision == Some(31),
        "replayed command did not return the original stored response: {replay:?}"
    );
    let head = harness.head("l4-cas-process-035")?;
    ensure!(
        head.disposition == "rejected" && head.remote_revision == 32,
        "replayed command re-evaluated or rewound the current decision head: {head:?}"
    );
    ensure!(
        harness.command_ledger_row_count("l4-cmd-035-a")? == 1,
        "replay inserted a duplicate command ledger row"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-035 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_036_conflicted_command_writes_command_ledger() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-036");
    println!("[SETUP] stale-base CAS command");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OperatorDecisionCasHarness::new(&state_db)?;
    harness.seed_head("l4-cas-process-036", "watching", 40)?;
    harness.queue_intent("l4-cas-process-036", "approved", 39, "l4-cmd-036")?;

    println!("[EXEC] flush stale-base command");
    let response = harness.flush_next_intent()?;

    println!("[ASSERT] conflict result is durable and decision head is unchanged");
    ensure!(
        response.is_conflict() && response.current_revision == Some(40),
        "stale command should return a structured conflict: {response:?}"
    );
    let head = harness.head("l4-cas-process-036")?;
    ensure!(
        head.disposition == "watching" && head.remote_revision == 40,
        "conflicted command overwrote current decision head: {head:?}"
    );
    let ledger = harness.ledger_row("l4-cmd-036")?;
    ensure!(
        ledger.base_revision == 39
            && ledger.applied_revision.is_none()
            && ledger.result == "conflict"
            && ledger.disposition == "approved",
        "conflict command ledger row lost required fields: {ledger:?}"
    );
    let ledger_response: Value = serde_json::from_str(&ledger.response_payload_json)
        .context("parsing conflict command ledger response JSON")?;
    ensure!(
        ledger_response
            .get("current_revision")
            .and_then(Value::as_i64)
            == Some(40)
            && ledger_response
                .get("rejected_payload")
                .and_then(|payload| payload.get("base_revision"))
                .and_then(Value::as_i64)
                == Some(39),
        "conflict command ledger response JSON is incomplete: {ledger_response}"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-036 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_037_command_ledger_gc_preserves_recent_entries() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-037");
    println!("[SETUP] command ledger rows on both sides of the retention cutoff");
    let migration = read_repo_file("infra/supabase/migrations/0007_command_ledger.sql")?;
    ensure!(
        migration.contains("DEFAULT interval '30 days'")
            && migration.contains("DELETE FROM operator_decision_commands")
            && migration.contains("created_at < now() - retention"),
        "command ledger migration does not expose the 30-day GC contract"
    );
    let state_db = L4TestStateDb::new()?;
    let mut harness = OperatorDecisionCasHarness::new(&state_db)?;
    let now = 1_800_000_000_i64;
    let retention_days = 30_i64;
    let old_epoch = now - retention_days * 86_400 - 1;
    let recent_epoch = now - retention_days * 86_400 + 1;
    harness.seed_head("l4-cas-process-037", "pending", 50)?;
    harness.queue_intent("l4-cas-process-037", "approved", 50, "l4-cmd-037-old")?;
    harness.flush_next_intent()?;
    harness.set_ledger_created_at("l4-cmd-037-old", old_epoch)?;
    harness.queue_intent("l4-cas-process-037", "watching", 51, "l4-cmd-037-recent")?;
    harness.flush_next_intent()?;
    harness.set_ledger_created_at("l4-cmd-037-recent", recent_epoch)?;

    println!("[EXEC] run command ledger GC with 30-day retention");
    let before = harness.head("l4-cas-process-037")?;
    let deleted = harness.gc_command_ledger(retention_days, now)?;
    let after = harness.head("l4-cas-process-037")?;

    println!("[ASSERT] GC removes only expired command rows and never changes the head");
    ensure!(
        deleted == 1,
        "GC should delete only one expired row, got {deleted}"
    );
    ensure!(
        harness.command_ledger_row_count("l4-cmd-037-old")? == 0,
        "expired command ledger row was not removed"
    );
    ensure!(
        harness.command_ledger_row_count("l4-cmd-037-recent")? == 1,
        "recent command ledger row was removed before retention elapsed"
    );
    ensure!(
        before == after && after.disposition == "watching" && after.remote_revision == 52,
        "GC changed current operator decision head: before={before:?} after={after:?}"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-037 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_038_post_gc_replay_is_evaluated_as_new_command() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-038");
    println!("[SETUP] applied command whose ledger row has expired");
    let state_db = L4TestStateDb::new()?;
    let mut harness = OperatorDecisionCasHarness::new(&state_db)?;
    let now = 1_800_000_000_i64;
    harness.seed_head("l4-cas-process-038", "pending", 60)?;
    harness.queue_intent("l4-cas-process-038", "approved", 60, "l4-cmd-038")?;
    let first = harness.flush_next_intent()?;
    harness.set_ledger_created_at("l4-cmd-038", now - 31 * 86_400)?;
    let deleted = harness.gc_command_ledger(30, now)?;

    println!("[EXEC] replay the same command id after GC with the stale base revision");
    harness.queue_intent("l4-cas-process-038", "approved", 60, "l4-cmd-038")?;
    let replay = harness.flush_next_intent()?;

    println!("[ASSERT] post-GC replay is treated as a new stale command and conflicts");
    ensure!(
        first.accepted && first.applied_revision == Some(61),
        "initial command did not apply: {first:?}"
    );
    ensure!(deleted == 1, "expired command ledger row was not removed");
    ensure!(
        replay.is_conflict() && replay.current_revision == Some(61),
        "post-GC replay did not evaluate against the current head: {replay:?}"
    );
    let head = harness.head("l4-cas-process-038")?;
    ensure!(
        head.disposition == "approved" && head.remote_revision == 61,
        "post-GC replay changed the decision head: {head:?}"
    );
    let ledger = harness.ledger_row("l4-cmd-038")?;
    ensure!(
        ledger.result == "conflict" && ledger.applied_revision.is_none(),
        "post-GC replay did not persist the new conflict ledger result: {ledger:?}"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-038 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_039_first_daemon_acquires_and_renews_writer_lease() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-039");
    println!("[SETUP] two-daemon writer lease harness");
    let state_db = L4TestStateDb::new()?;
    let mut harness = WriterLeaseHarness::new(&state_db)?;

    println!("[EXEC] first daemon acquires, then renews before expiry");
    let first = match harness.acquire("executor-a", 1_000)? {
        WriterLeaseAcquireResult::Acquired(snapshot) => snapshot,
        WriterLeaseAcquireResult::Conflict(conflict) => {
            bail_with_conflict("first acquire unexpectedly conflicted", &conflict)?
        }
    };
    let renewed = match harness.acquire("executor-a", 1_030)? {
        WriterLeaseAcquireResult::Acquired(snapshot) => snapshot,
        WriterLeaseAcquireResult::Conflict(conflict) => {
            bail_with_conflict("renew unexpectedly conflicted", &conflict)?
        }
    };

    println!("[ASSERT] first acquisition increments epoch and renew preserves it");
    ensure!(
        first.holder_executor_id.as_deref() == Some("executor-a"),
        "first lease holder was not executor-a"
    );
    ensure!(
        first.writer_epoch == 1,
        "first acquisition should increment writer_epoch from 0 to 1"
    );
    ensure!(
        first.lease_expires_at_epoch_secs >= 1_120,
        "first acquisition did not extend expiry at least 120 seconds"
    );
    ensure!(
        renewed.writer_epoch == first.writer_epoch,
        "same-holder renewal must preserve writer_epoch"
    );
    ensure!(
        renewed.lease_expires_at_epoch_secs >= 1_150,
        "same-holder renewal did not extend expiry from renewal time"
    );
    ensure!(
        harness.current_epoch()? == Some(1),
        "current writer epoch did not persist"
    );
    let migration = read_repo_file("infra/supabase/migrations/0006_schema_metadata.sql")?;
    ensure!(
        migration.contains("CREATE OR REPLACE FUNCTION acquire_active_writer_lease")
            && migration.contains("FOR UPDATE")
            && migration.contains("current_epoch + 1"),
        "active writer lease migration must provide a CAS acquisition function"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-039 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_040_second_daemon_conflict_surfaces_holder_and_expiry() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-040");
    println!("[SETUP] valid active writer lease held by executor-a");
    let state_db = L4TestStateDb::new()?;
    let mut harness = WriterLeaseHarness::new(&state_db)?;
    let first = match harness.acquire("executor-a", 2_000)? {
        WriterLeaseAcquireResult::Acquired(snapshot) => snapshot,
        WriterLeaseAcquireResult::Conflict(conflict) => {
            bail_with_conflict("first acquire unexpectedly conflicted", &conflict)?
        }
    };

    println!("[EXEC] second daemon attempts to acquire while lease is valid");
    let conflict = match harness.acquire("executor-b", 2_030)? {
        WriterLeaseAcquireResult::Acquired(snapshot) => {
            anyhow::bail!("second writer unexpectedly acquired lease: {snapshot:?}")
        }
        WriterLeaseAcquireResult::Conflict(conflict) => conflict,
    };

    println!("[ASSERT] conflict payload is suitable for desktop sync status");
    ensure!(
        conflict.current_holder_executor_id == "executor-a",
        "conflict did not include current holder executor id"
    );
    ensure!(
        conflict.lease_expires_at_epoch_secs == first.lease_expires_at_epoch_secs,
        "conflict expiry did not match active lease expiry"
    );
    ensure!(
        conflict.next_safe_action.contains("viewer-only")
            && conflict.next_safe_action.contains("explicit rehome"),
        "conflict next action must mention viewer-only and explicit rehome"
    );

    let body = apply_push_batch_body()?;
    ensure!(
        body.contains("holder_executor_id")
            && body.contains("batch_executor_id")
            && body.contains("'error', 'writer_lease_conflict'"),
        "apply_push_batch must reject a different executor_id while a lease is valid"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-040 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_041_partitioned_old_epoch_rejected_before_apply() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-041");
    println!("[SETUP] inspect apply_push_batch stale writer branch");
    let body = apply_push_batch_body()?;

    let writer_epoch_gate = body
        .find("IF remote_writer_epoch <> writer_epoch THEN")
        .context("missing remote writer_epoch stale-writer gate")?;
    let stale_writer_response = body
        .find("'error', 'stale_writer'")
        .context("missing stale_writer rejection response")?;
    let holder_conflict_response = body
        .find("'error', 'writer_lease_conflict'")
        .context("missing writer lease conflict response")?;
    let row_prevalidation = body
        .find("FOR row_item IN SELECT value FROM jsonb_array_elements(rows_json)")
        .context("missing row prevalidation loop")?;
    let apply_statement = body
        .find("PERFORM l4_apply_push_owned_payload")
        .context("missing projection apply statement")?;

    println!("[ASSERT] old writer_epoch rejects before row validation or projection apply");
    ensure!(
        writer_epoch_gate < stale_writer_response
            && stale_writer_response < holder_conflict_response
            && holder_conflict_response < row_prevalidation
            && row_prevalidation < apply_statement,
        "stale writer rejection must happen before row validation and projection apply"
    );
    ensure!(
        body.contains("'remote_writer_epoch', remote_writer_epoch")
            && body.contains("'writer_epoch', writer_epoch"),
        "stale writer response must include remote and rejected writer epochs"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-041 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_042_heartbeat_extends_expiry_and_expiry_allows_reacquire() -> Result<()>
{
    println!("[TEST_START] L4-T-sqlite_supabase_sync-042");
    println!("[SETUP] active lease held by executor-a");
    let state_db = L4TestStateDb::new()?;
    let mut harness = WriterLeaseHarness::new(&state_db)?;
    let acquired = match harness.acquire("executor-a", 3_000)? {
        WriterLeaseAcquireResult::Acquired(snapshot) => snapshot,
        WriterLeaseAcquireResult::Conflict(conflict) => {
            bail_with_conflict("first acquire unexpectedly conflicted", &conflict)?
        }
    };

    println!("[EXEC] heartbeat at 30 seconds and competing writer attempts before expiry");
    let heartbeat = match harness.heartbeat("executor-a", 3_030)? {
        WriterLeaseAcquireResult::Acquired(snapshot) => snapshot,
        WriterLeaseAcquireResult::Conflict(conflict) => {
            bail_with_conflict("heartbeat unexpectedly conflicted", &conflict)?
        }
    };
    let before_expiry = harness.acquire("executor-b", heartbeat.lease_expires_at_epoch_secs - 1)?;

    println!("[ASSERT] heartbeat cadence and TTL satisfy the lease contract");
    ensure!(
        heartbeat.writer_epoch == acquired.writer_epoch,
        "heartbeat must preserve writer_epoch"
    );
    ensure!(
        heartbeat.lease_expires_at_epoch_secs >= 3_150,
        "heartbeat did not extend expiry at least 120 seconds"
    );
    ensure!(
        heartbeat.heartbeat_interval_secs <= 30,
        "heartbeat interval must be at most 30 seconds"
    );
    ensure!(
        matches!(before_expiry, WriterLeaseAcquireResult::Conflict(_)),
        "another writer acquired before lease expiry"
    );

    println!("[EXEC] competing writer retries after expiry");
    let after_expiry =
        match harness.acquire("executor-b", heartbeat.lease_expires_at_epoch_secs + 1)? {
            WriterLeaseAcquireResult::Acquired(snapshot) => snapshot,
            WriterLeaseAcquireResult::Conflict(conflict) => {
                bail_with_conflict("post-expiry acquire unexpectedly conflicted", &conflict)?
            }
        };
    ensure!(
        after_expiry.writer_epoch == acquired.writer_epoch + 1,
        "post-expiry acquisition must increment writer_epoch"
    );
    ensure!(
        after_expiry.holder_executor_id.as_deref() == Some("executor-b"),
        "post-expiry holder was not executor-b"
    );
    let migration = read_repo_file("infra/supabase/migrations/0006_schema_metadata.sql")?;
    ensure!(
        migration.contains("CREATE OR REPLACE FUNCTION heartbeat_active_writer_lease")
            && migration.contains("heartbeat_interval_seconds', 30")
            && migration.contains("GREATEST(COALESCE(lease_ttl_seconds, 120), 120)"),
        "active writer lease migration must pin 30s heartbeat and at least 120s TTL"
    );
    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-042 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_043_unapproved_second_writer_config_rejected() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-043");
    println!("[SETUP] unapproved writer-mode config for a second installation");
    let decision =
        WriterBindingConfig::new("default", "executor-b", WriterBindingMode::Writer, false)?
            .validate();

    println!("[ASSERT] unapproved writer mode falls back to explicit safe actions");
    ensure!(!decision.accepted, "unapproved writer config was accepted");
    ensure!(
        decision.effective_mode == WriterBindingMode::ViewerOnly,
        "unapproved second writer must be forced to viewer-only"
    );
    ensure!(
        decision
            .next_safe_actions
            .iter()
            .any(|action| action.contains("viewer-only"))
            && decision
                .next_safe_actions
                .iter()
                .any(|action| action.contains("explicit rehome")),
        "config rejection did not surface viewer-only and explicit rehome actions"
    );

    let spec = read_repo_file("docs/strategy/layers/layer-4-shared-visibility.md")?;
    ensure!(
        spec.contains("One bound machine per org")
            && spec.contains("viewer-only")
            && spec.contains("explicit rehome"),
        "operator/admin docs must state the one-writer binding and safe actions"
    );
    println!("[CLEANUP] config/doc fixture only");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-043 PASS");
    Ok(())
}

#[tokio::test]
async fn l4_t_sqlite_supabase_sync_044_schema_epoch_mismatch_pauses_sync_only() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-044");
    println!("[SETUP] outbox batch at local PRAGMA schema epoch");
    let state_db = L4TestStateDb::new()?;
    let mut outbox = OutboxHarness::new(&state_db)?;
    outbox.mutate_push_owned_fixture(PushOwnedFixture::new("l4-epoch-pause-row", 1))?;
    let batch = outbox
        .assemble_batch(10, 7)?
        .context("schema epoch test batch was not materialized")?;
    let local_schema_epoch = SchemaEpochGuard::local_schema_epoch(outbox.connection())?;
    ensure!(
        local_schema_epoch == DEFAULT_SCHEMA_EPOCH && batch.schema_epoch == local_schema_epoch,
        "batch did not use the local PRAGMA schema epoch: local={local_schema_epoch} batch={batch:?}"
    );
    let (api_url, capture) = spawn_rpc_capture_server(json!({
        "accepted": false,
        "status": "rejected",
        "error": "schema_epoch_mismatch",
        "remote_schema_epoch": DEFAULT_SCHEMA_EPOCH + 1,
        "batch_schema_epoch": DEFAULT_SCHEMA_EPOCH
    }))
    .await?;

    println!("[EXEC] remote apply rejects incompatible schema epoch");
    let client = SupabaseRpcClient::new(api_url, "service-role-test-key")?;
    let response = client.apply_push_batch(&batch).await?;
    let raw_request = capture.await.context("joining capture server task")??;
    outbox.record_apply_response(&batch.batch_id, &response)?;

    println!("[ASSERT] mismatch records operator-visible pause and stops projection dispatch");
    ensure!(
        response.is_schema_epoch_mismatch()
            && response.remote_schema_epoch() == Some(DEFAULT_SCHEMA_EPOCH + 1),
        "apply response was not classified as schema epoch mismatch: {response:?}"
    );
    let body = http_body(&raw_request)?;
    let request_json: Value =
        serde_json::from_str(body).context("parsing captured apply_push_batch body")?;
    ensure!(
        request_json
            .get("batch_json")
            .and_then(|batch_json| batch_json.get("schema_epoch"))
            .and_then(Value::as_i64)
            == Some(DEFAULT_SCHEMA_EPOCH),
        "apply request did not carry schema_epoch in batch_json: {request_json}"
    );
    let paused = outbox.schema_epoch_status()?;
    ensure!(
        paused.paused
            && paused.reason.as_deref() == Some("schema_epoch_mismatch")
            && paused.local_schema_epoch == DEFAULT_SCHEMA_EPOCH
            && paused.remote_schema_epoch == Some(DEFAULT_SCHEMA_EPOCH + 1)
            && !paused.push_dispatch_allowed()
            && !paused.pull_dispatch_allowed()
            && paused.local_operation_available
            && paused.operator_message.contains("epoch repair"),
        "schema epoch mismatch did not persist the expected pause state: {paused:?}"
    );
    let viewer_status = outbox.viewer_sync_status()?;
    ensure!(
        viewer_status.sync_health == SyncHealth::Offline
            && viewer_status.last_push_attempt_at_epoch_secs.is_some()
            && viewer_status.schema_epoch == DEFAULT_SCHEMA_EPOCH,
        "viewer sync status did not surface the schema pause: {viewer_status:?}"
    );
    ensure!(
        outbox.assemble_batch(10, 8)?.is_none(),
        "projection writes resumed while schema epoch pause was active"
    );

    println!("[EXEC] write local pipeline data while remote sync is paused");
    outbox.mutate_push_owned_fixture(
        PushOwnedFixture::new("l4-local-write-during-epoch-pause", 2)
            .commit_group_id("l4-local-write-during-epoch-pause"),
    )?;

    println!("[ASSERT] local SQLite operation continues under sync pause");
    ensure!(
        outbox
            .process_last_local_seq("l4-local-write-during-epoch-pause")?
            .is_some(),
        "local pipeline write was blocked by schema epoch sync pause"
    );
    let rpc_body = apply_push_batch_body()?;
    ensure!(
        rpc_body.contains("FROM schema_metadata")
            && rpc_body.contains("'error', 'schema_epoch_mismatch'")
            && rpc_body.contains("'remote_schema_epoch', remote_schema_epoch"),
        "apply_push_batch RPC no longer exposes the remote schema epoch guard"
    );

    println!("[CLEANUP] tempfile-owned state DB and one-shot capture server");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-044 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_046_viewer_sync_status_updates_after_ack_and_heartbeat() -> Result<()>
{
    println!("[TEST_START] L4-T-sqlite_supabase_sync-046");
    println!("[SETUP] outbox harness with a dispatchable batch");
    let state_db = L4TestStateDb::new()?;
    {
        let mut outbox = OutboxHarness::new(&state_db)?;
        outbox.mutate_push_owned_fixture(PushOwnedFixture::new("l4-status-acked-row", 1))?;
        let batch = outbox
            .assemble_batch(10, 7)?
            .context("status test batch was not materialized")?;

        println!("[EXEC] record an accepted apply_push_batch response through the assembler");
        outbox.record_successful_apply_response(&batch.batch_id)?;
        let applied_status = outbox.viewer_sync_status()?;

        println!("[ASSERT] accepted apply response writes viewer_sync_status automatically");
        assert_automatic_apply_status(&applied_status, batch.last_local_seq)?;
    }

    println!("[SETUP] writer lease harness with one pending local outbox row");
    let harness = WriterLeaseHarness::new(&state_db)?;
    harness.insert_pending_outbox_marker("l4-status-pending-one")?;

    println!("[EXEC] record a successful remote apply acknowledgement");
    let healthy =
        harness.record_viewer_sync_apply_success(1_000, 77, 4, "daemon-l4-status-test")?;

    println!("[ASSERT] successful apply writes the complete healthy viewer_sync_status row");
    assert_apply_success_status(&healthy, 1_000, 77, 1, 4)?;

    println!("[EXEC] refresh heartbeat/status-only rows without changing projection data");
    let degraded = harness.record_viewer_sync_heartbeat(1_090, 4, "daemon-l4-status-test")?;
    let offline = harness.record_viewer_sync_heartbeat(1_301, 4, "daemon-l4-status-test")?;

    println!("[ASSERT] heartbeat-only commits advance attempt/update time and derive freshness");
    assert_heartbeat_status(&degraded, SyncHealth::Degraded, 1_090)?;
    assert_heartbeat_status(&offline, SyncHealth::Offline, 1_301)?;

    println!("[EXEC] record a later apply acknowledgement after another pending row appears");
    harness.insert_pending_outbox_marker("l4-status-pending-two")?;
    let refreshed =
        harness.record_viewer_sync_apply_success(1_320, 88, 5, "daemon-l4-status-test")?;

    println!("[ASSERT] later apply returns to healthy and refreshes ack/projection fields");
    assert_apply_success_status(&refreshed, 1_320, 88, 2, 5)?;
    assert_viewer_sync_status_migration_contract()?;

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-046 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_047_remote_config_staleness_ttl_classes() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-047");
    println!("[SETUP] pull cursor harness and fixed staleness evaluation clock");
    let state_db = L4TestStateDb::new()?;
    let harness = PullCursorHarness::new(&state_db)?;
    let now_epoch_secs = 100_000;
    ensure!(
        REMOTE_CONFIG_OPERATIONAL_WARN_AFTER_SECS == 3_600
            && REMOTE_CONFIG_SAFETY_CRITICAL_TTL_SECS == 14_400,
        "remote config default staleness thresholds drifted"
    );

    println!("[EXEC] evaluate stale company profile, source display/schedule, enabled, decisions");
    let company_profile = harness.evaluate_remote_config_staleness(
        RemoteConfigStalenessSubject::SharedCompanyProfile,
        now_epoch_secs,
        now_epoch_secs - 86_400,
    )?;
    let display = harness.evaluate_remote_config_staleness(
        RemoteConfigStalenessSubject::SharedSourceConfigsDisplay,
        now_epoch_secs,
        now_epoch_secs - REMOTE_CONFIG_OPERATIONAL_WARN_AFTER_SECS - 1,
    )?;
    let schedule = harness.evaluate_remote_config_staleness(
        RemoteConfigStalenessSubject::SharedSourceConfigsSchedule,
        now_epoch_secs,
        now_epoch_secs - REMOTE_CONFIG_OPERATIONAL_WARN_AFTER_SECS - 1,
    )?;
    let enabled_before_ttl = harness.evaluate_remote_config_staleness(
        RemoteConfigStalenessSubject::SharedSourceConfigsEnabled,
        now_epoch_secs,
        now_epoch_secs - REMOTE_CONFIG_SAFETY_CRITICAL_TTL_SECS + 1,
    )?;
    let enabled_after_ttl = harness.evaluate_remote_config_staleness(
        RemoteConfigStalenessSubject::SharedSourceConfigsEnabled,
        now_epoch_secs,
        now_epoch_secs - REMOTE_CONFIG_SAFETY_CRITICAL_TTL_SECS - 1,
    )?;
    let decision_heads = harness.evaluate_remote_config_staleness(
        RemoteConfigStalenessSubject::OperatorDecisionHeads,
        now_epoch_secs,
        now_epoch_secs - REMOTE_CONFIG_OPERATIONAL_WARN_AFTER_SECS - 1,
    )?;

    println!("[ASSERT] company profile stays informational and cached even when very stale");
    ensure!(
        company_profile.class == RemoteConfigStalenessClass::Informational
            && company_profile.action == RemoteConfigStalenessAction::UseCached
            && !company_profile.warns()
            && !company_profile.degrades_source(),
        "company profile staleness must remain informational: {company_profile:?}"
    );

    println!("[ASSERT] source display and schedule fields warn after one hour but use cache");
    for staleness in [display, schedule] {
        ensure!(
            staleness.class == RemoteConfigStalenessClass::Operational
                && staleness.action == RemoteConfigStalenessAction::WarnUseCached
                && staleness.warns()
                && !staleness.degrades_source(),
            "operational source config staleness did not warn/use cache: {staleness:?}"
        );
    }

    println!("[ASSERT] enabled/disabled source authority degrades only after safety TTL");
    ensure!(
        enabled_before_ttl.class == RemoteConfigStalenessClass::SafetyCritical
            && enabled_before_ttl.action == RemoteConfigStalenessAction::WarnUseCached
            && !enabled_before_ttl.degrades_source(),
        "enabled flag degraded before the safety-critical TTL: {enabled_before_ttl:?}"
    );
    ensure!(
        enabled_after_ttl.class == RemoteConfigStalenessClass::SafetyCritical
            && enabled_after_ttl.action == RemoteConfigStalenessAction::DegradeSource
            && enabled_after_ttl.degrades_source(),
        "enabled flag did not degrade after the safety-critical TTL: {enabled_after_ttl:?}"
    );

    println!("[ASSERT] stale operator decisions remain CAS-protected operational state");
    ensure!(
        decision_heads.class == RemoteConfigStalenessClass::Operational
            && decision_heads.action == RemoteConfigStalenessAction::WarnUseCached
            && decision_heads.cas_still_required
            && !decision_heads.degrades_source(),
        "operator decision heads staleness bypassed CAS semantics: {decision_heads:?}"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-047 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_048_sqlite_snapshot_scheduler_warns_without_blocking() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-048");
    println!("[SETUP] local state DB with WAL-aware snapshot scheduler");
    let state_db = L4TestStateDb::new()?;
    let mut outbox = OutboxHarness::new(&state_db)?;
    outbox.mutate_push_owned_fixture(PushOwnedFixture::new("l4-backup-before-snapshot", 1))?;
    let scheduler =
        SqliteSnapshotScheduler::new(3_600, "target/l4-sqlite-snapshots", "test-key-v1")?;

    println!("[EXEC] run a due encrypted snapshot");
    let success = scheduler.run_due(outbox.connection(), state_db.db_path(), 1_000, |request| {
        ensure!(
            request.wal_checkpointed
                && request.destination_path.ends_with(".sqlite.enc")
                && request.encryption_key_id == "test-key-v1",
            "snapshot request was not WAL-aware/encrypted-targeted: {request:?}"
        );
        Ok(SqliteSnapshotArtifact::encrypted(
            request.destination_path.clone(),
            4_096,
        ))
    })?;

    println!("[ASSERT] success records encrypted WAL-aware snapshot metadata");
    ensure!(
        success.status == "succeeded"
            && success.wal_checkpointed
            && success.encrypted
            && success.warning_code.is_none()
            && success.local_operation_available
            && success.bytes_written == 4_096,
        "successful snapshot did not record the expected metadata: {success:?}"
    );

    println!("[EXEC] poll again before cadence elapses");
    let skipped =
        scheduler.run_due(outbox.connection(), state_db.db_path(), 1_100, |_request| {
            anyhow::bail!("snapshot runner should not execute before cadence elapses")
        })?;

    println!("[ASSERT] cadence prevents duplicate snapshot work");
    ensure!(
        skipped.status == "skipped" && !skipped.wal_checkpointed,
        "scheduler did not skip before cadence elapsed: {skipped:?}"
    );

    println!("[EXEC] run a later snapshot whose encrypted sink fails");
    let failure = scheduler.run_due(outbox.connection(), state_db.db_path(), 4_600, |request| {
        ensure!(
            request.wal_checkpointed,
            "failed snapshot path still must checkpoint WAL before handoff"
        );
        anyhow::bail!("simulated encrypted snapshot sink failure")
    })?;

    println!("[ASSERT] failure is an operational warning and does not block local writes");
    ensure!(
        failure.status == "failed"
            && failure.warning_code.as_deref() == Some("SQLITE_SNAPSHOT_FAILED")
            && failure
                .warning_message
                .as_deref()
                .is_some_and(|message| message.contains("simulated encrypted snapshot"))
            && failure.local_operation_available,
        "snapshot failure was not recorded as a non-blocking warning: {failure:?}"
    );
    outbox.mutate_push_owned_fixture(
        PushOwnedFixture::new("l4-backup-local-after-warning", 2)
            .commit_group_id("l4-backup-local-after-warning"),
    )?;
    ensure!(
        outbox
            .process_last_local_seq("l4-backup-local-after-warning")?
            .is_some(),
        "local pipeline write was blocked by snapshot warning"
    );

    let records = SqliteSnapshotScheduler::records(outbox.connection())?;
    ensure!(
        records.len() == 3
            && records.iter().any(|record| record.status == "succeeded")
            && records.iter().any(|record| record.status == "skipped")
            && records.iter().any(|record| record.status == "failed"),
        "snapshot run ledger did not retain success/skip/failure history: {records:?}"
    );
    let spec = read_repo_file("docs/strategy/layers/layer-4-shared-visibility.md")?;
    ensure!(
        spec.contains("Encrypted WAL-aware snapshots")
            || spec.contains("encrypted WAL-aware snapshots"),
        "operator/admin documentation must mention encrypted WAL-aware snapshots"
    );
    ensure!(
        spec.contains("Single-device durability is a known MVP limitation"),
        "operator/admin documentation must state the single-device pre-push loss limitation"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-048 PASS");
    Ok(())
}

#[test]
fn l4_t_sqlite_supabase_sync_080_validator_crash_uses_l3_fail_open_marker() -> Result<()> {
    println!("[TEST_START] L4-T-sqlite_supabase_sync-080");
    println!("[SETUP] enforcement harness with no persisted validator verdicts");
    let state_db = L4TestStateDb::new()?;
    let mut harness = EnforcementHarness::new(&state_db)?;
    let worker_job_id = "l4-validator-crash-worker";
    let before_suspect_manifests = count_error_manifests_by_type(
        state_db.connection(),
        worker_job_id,
        ErrorCode::AgentSuspectOutput,
    )?;

    println!("[EXEC] record validator crash/timeout as fail-open worker completion");
    harness.record_validator_fail_open(worker_job_id, "validator timeout")?;

    println!("[ASSERT] fail-open path keeps worker completed without AGENT_SUSPECT_OUTPUT");
    let (status, validation_required, validation_verdict_id, output_payload): (
        String,
        i64,
        Option<String>,
        String,
    ) = state_db.connection().query_row(
        "\
        SELECT status, validation_required, validation_verdict_id, output_payload
        FROM workflow_jobs
        WHERE id = ?1
        ",
        [worker_job_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let output: Value = serde_json::from_str(&output_payload)?;
    ensure!(status == "completed", "worker did not complete fail-open");
    ensure!(
        validation_required == 1,
        "worker did not retain validation_required marker"
    );
    ensure!(
        validation_verdict_id.is_none(),
        "fail-open worker unexpectedly linked a validator verdict"
    );
    ensure!(
        output.get("validation_marker").and_then(Value::as_str)
            == Some("VALIDATION_SKIPPED_VALIDATOR_ERROR"),
        "validator crash did not use the L3 fail-open marker: {output}"
    );
    ensure!(
        output.get("validation_marker").and_then(Value::as_str) != Some("AGENT_SUSPECT_OUTPUT"),
        "validator crash was misclassified as suspect output"
    );
    ensure!(
        harness.count_rows("validation_verdicts")? == 0,
        "validator crash persisted a verdict row"
    );
    ensure!(
        count_error_manifests_by_type(
            state_db.connection(),
            worker_job_id,
            ErrorCode::AgentSuspectOutput,
        )? == before_suspect_manifests,
        "validator crash emitted an AGENT_SUSPECT_OUTPUT manifest"
    );

    println!("[CLEANUP] tempfile-owned state DB");
    println!("[TEST_END] L4-T-sqlite_supabase_sync-080 PASS");
    Ok(())
}

fn assert_automatic_apply_status(status: &ViewerSyncStatus, last_local_seq: i64) -> Result<()> {
    ensure!(
        status.sync_health == SyncHealth::Healthy,
        "accepted apply response did not mark sync healthy: {status:?}"
    );
    ensure!(
        status.last_push_success_at_epoch_secs.is_some()
            && status.last_push_attempt_at_epoch_secs == status.last_push_success_at_epoch_secs,
        "accepted apply response did not refresh success/attempt timestamps: {status:?}"
    );
    ensure!(
        status.last_acked_local_seq == Some(last_local_seq),
        "accepted apply response did not store the batch's last local seq: {status:?}"
    );
    ensure!(
        status.pending_outbox_count == 0 && status.schema_epoch == DEFAULT_SCHEMA_EPOCH,
        "accepted apply response did not carry pending count/schema epoch: {status:?}"
    );
    Ok(())
}

fn assert_apply_success_status(
    status: &ViewerSyncStatus,
    expected_epoch: i64,
    expected_last_acked: i64,
    expected_pending: i64,
    expected_generation: i64,
) -> Result<()> {
    ensure!(
        status.sync_health == SyncHealth::Healthy,
        "successful apply did not mark sync healthy: {status:?}"
    );
    ensure!(
        status.last_push_success_at_epoch_secs == Some(expected_epoch)
            && status.last_push_attempt_at_epoch_secs == Some(expected_epoch)
            && status.last_acked_local_seq == Some(expected_last_acked),
        "successful apply did not refresh success/attempt/ack: {status:?}"
    );
    ensure!(
        status.pending_outbox_count == expected_pending
            && status.schema_epoch == DEFAULT_SCHEMA_EPOCH
            && status.projection_generation == expected_generation
            && status.daemon_version == "daemon-l4-status-test",
        "successful apply did not store pending/schema/generation/version: {status:?}"
    );
    Ok(())
}

fn assert_heartbeat_status(
    status: &ViewerSyncStatus,
    expected_health: SyncHealth,
    expected_attempt_epoch: i64,
) -> Result<()> {
    ensure!(
        status.sync_health == expected_health,
        "heartbeat-only status had wrong health: {status:?}"
    );
    ensure!(
        status.last_push_success_at_epoch_secs == Some(1_000)
            && status.last_push_attempt_at_epoch_secs == Some(expected_attempt_epoch)
            && status.last_acked_local_seq == Some(77),
        "heartbeat changed immutable success/ack fields or missed attempt: {status:?}"
    );
    Ok(())
}

fn assert_viewer_sync_status_migration_contract() -> Result<()> {
    let migration = read_repo_file("infra/supabase/migrations/0003_summary_tables.sql")?;
    ensure!(
        migration.contains("CREATE TABLE viewer_sync_status")
            && migration.contains("sync_health IN ('healthy', 'degraded', 'offline')")
            && migration.contains("last_acked_local_seq")
            && migration.contains("pending_outbox_count")
            && migration.contains("projection_generation")
            && migration.contains("daemon_version"),
        "Supabase summary migration does not expose the viewer_sync_status freshness contract"
    );
    Ok(())
}

fn seed_error_manifest_process(conn: &rusqlite::Connection, process_id: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO procurement_processes (
          id, canonical_process_id, current_stage, title, latest_published_at,
          latest_updated_at, owner_id
        ) VALUES (?1, ?1, 'review', 'L4 enforcement process',
                  '2026-03-28T09:00:00Z', '2026-03-28T09:00:00Z', 'local-owner')
        ",
        params![process_id],
    )?;
    Ok(())
}

fn enforcement_suspect_draft(fixture: &RedactionSentinelFixture) -> ErrorManifestDraft {
    let payloads = fixture.redacted_summary_payloads();
    ErrorManifestDraft {
        job_id: Some(fixture.job_id.clone()),
        process_id: Some(fixture.process_id.clone()),
        pipeline_stage: "Validation enforcement".to_owned(),
        workflow_step: "provisional_promotion".to_owned(),
        error_type: ErrorCode::AgentSuspectOutput,
        error_message: "Validator rejected enforcement-active worker output".to_owned(),
        error_context: json!({
            "workflow_job_summary": payloads.workflow_job_summary,
            "error_manifest_summary": payloads.error_manifest_summary,
            "validation_verdict_summary": payloads.validation_verdict_summary,
            "operator_decision_summary": payloads.operator_decision_summary
        }),
        skill_path: Some("skills/validation/base/SKILL.md".to_owned()),
        agent_session_id: None,
        runtime_context: ErrorRuntimeContext {
            cli_vendor: Some("fixture-cli".to_owned()),
            cli_version: Some("1.0.0".to_owned()),
            adapter_version: Some("daemon-test".to_owned()),
            model_id: Some("validator-fixture".to_owned()),
            config_hash: Some("validator-config-hash".to_owned()),
            skill_version: Some("1.0.0".to_owned()),
            pid: None,
            pgid: None,
            cleanup_verdict: None,
            last_valid_checkpoint_sequence: Some(7),
            daemon_instance_id: Some("daemon-l4-enforcement".to_owned()),
            commit_seq: Some(44),
        },
        session_artifact_path: Some("artifacts/validation/session.log".to_owned()),
        next_debugging_action:
            "Inspect validator evidence, keep output quarantined, and rerun from checkpoint 7"
                .to_owned(),
    }
}

fn count_error_manifests_by_type(
    conn: &rusqlite::Connection,
    job_id: &str,
    error_type: ErrorCode,
) -> Result<i64> {
    conn.query_row(
        "\
        SELECT COUNT(*)
        FROM error_manifests
        WHERE job_id = ?1 AND error_type = ?2
        ",
        params![job_id, error_type.as_str()],
        |row| row.get(0),
    )
    .context("counting error manifests by type")
}
