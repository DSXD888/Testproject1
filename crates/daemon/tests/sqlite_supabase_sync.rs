#![forbid(unsafe_code)]

mod support;

use anyhow::{Result, ensure};

use support::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb};
use support::sync::{EnforcementHarness, OutboxHarness, parse_payload};

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
