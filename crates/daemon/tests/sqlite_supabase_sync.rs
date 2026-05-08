#![forbid(unsafe_code)]

#[path = "../src/ipc/handlers/decisions.rs"]
#[allow(dead_code)]
mod decisions_handler_impl;
mod support;

mod jobs {
    pub mod runner {
        #[allow(dead_code)]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct MutationAck {
            pub accepted: bool,
            pub visible_after_seq: i64,
            pub continuation_job_id: Option<String>,
        }
    }
}

use std::collections::BTreeSet;
use std::path::Path;
use std::str;

use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use decisions_handler_impl::{
    DECISION_CONFLICT_TOPIC, DecisionConflictEventParams, decision_conflict_ipc_event,
};
use support::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb};
use support::sync::{
    EnforcementHarness, OperatorDecisionCasHarness, OutboxHarness, PullCursorDomain,
    PullCursorHarness, PushOwnedFixture, REMOTE_CONFIG_OPERATIONAL_WARN_AFTER_SECS,
    REMOTE_CONFIG_SAFETY_CRITICAL_TTL_SECS, RemoteConfigStalenessAction,
    RemoteConfigStalenessClass, RemoteConfigStalenessSubject, SchemaEpochGuard,
    SqliteSnapshotArtifact, SqliteSnapshotScheduler, SupabaseRpcClient, SyncCyclePhase,
    SyncCycleTrigger, SyncHealth, ViewerSyncStatus, WriterBindingConfig, WriterBindingMode,
    WriterLeaseAcquireResult, WriterLeaseHarness, canonical_content_hash, parse_payload,
};

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
