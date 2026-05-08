#![forbid(unsafe_code)]

mod support;

use anyhow::{Result, ensure};

use support::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb};
use support::sync::{
    EnforcementHarness, OutboxHarness, PushOwnedFixture, canonical_content_hash, parse_payload,
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

    println!("[EXEC] terminal stale-writer rejection re-envelopes same rows");
    let new_batch = harness.re_envelope_after_stale_writer(&old_batch.batch_id, 4)?;

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
