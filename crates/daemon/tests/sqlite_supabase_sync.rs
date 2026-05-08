#![forbid(unsafe_code)]

mod support;

use std::path::Path;
use std::str;

use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use support::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb};
use support::sync::{
    EnforcementHarness, OutboxHarness, PullCursorDomain, PullCursorHarness, PushOwnedFixture,
    SupabaseRpcClient, WriterBindingConfig, WriterBindingMode, WriterLeaseAcquireResult,
    WriterLeaseHarness, canonical_content_hash, parse_payload,
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
