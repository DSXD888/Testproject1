#![forbid(unsafe_code)]

#[allow(clippy::missing_const_for_fn)]
#[path = "../src/errors/manifest.rs"]
mod errors_manifest_impl;
#[allow(clippy::missing_const_for_fn)]
#[path = "../src/errors/markdown.rs"]
mod errors_markdown_impl;
#[allow(clippy::missing_const_for_fn)]
#[path = "../src/db/sqlite.rs"]
mod sqlite_impl;

use anyhow::Result;
use chrono::Utc;
use rusqlite::params;
use serde_json::json;

use crate::errors_manifest_impl::{
    ErrorCode, ErrorManifestDraft, ErrorManifestStore, ErrorRuntimeContext,
};
use crate::sqlite_impl::{DEFAULT_OWNER_ID, DEFAULT_SCHEMA_EPOCH, StateDb};

fn open_temp_db() -> Result<(tempfile::TempDir, StateDb)> {
    let temp = tempfile::tempdir()?;
    let db_path = temp.path().join("state.db");
    let state_db = sqlite_impl::open_state_db(&db_path, DEFAULT_SCHEMA_EPOCH)?;
    Ok((temp, state_db))
}

fn seed_process(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO procurement_processes (
          id, canonical_process_id, current_stage, title, latest_published_at,
          latest_updated_at, owner_id
        ) VALUES ('process-1', 'process-1', 'review', 'Seed process',
                  '2026-03-28T09:00:00Z', '2026-03-28T09:00:00Z', ?1)
        ",
        params![DEFAULT_OWNER_ID],
    )?;
    Ok(())
}

fn draft(error_type: ErrorCode) -> ErrorManifestDraft {
    ErrorManifestDraft {
        job_id: None,
        process_id: Some("process-1".to_owned()),
        pipeline_stage: "Enrich".to_owned(),
        workflow_step: "supplier_lookup".to_owned(),
        error_type,
        error_message: "Failure details".to_owned(),
        error_context: json!({ "step": "supplier_lookup" }),
        skill_path: Some("skills/procurement-ingestion/fts/SKILL.md".to_owned()),
        agent_session_id: None,
        runtime_context: ErrorRuntimeContext {
            cli_vendor: Some("fixture-cli".to_owned()),
            cli_version: Some("1.0.0".to_owned()),
            adapter_version: Some("daemon-test".to_owned()),
            model_id: None,
            config_hash: Some("config-hash".to_owned()),
            skill_version: Some("1.0.0".to_owned()),
            pid: Some(123),
            pgid: Some(123),
            cleanup_verdict: Some("forced".to_owned()),
            last_valid_checkpoint_sequence: Some(2),
            daemon_instance_id: Some("daemon-1".to_owned()),
            commit_seq: Some(10),
        },
        session_artifact_path: Some("artifacts/session.log".to_owned()),
        next_debugging_action: "Inspect the session transcript".to_owned(),
    }
}

#[test]
fn ag_err_001_store_create_persists_error_manifest_row_and_markdown() -> Result<()> {
    let (temp, state_db) = open_temp_db()?;
    seed_process(state_db.connection())?;
    let store = ErrorManifestStore::new(temp.path().join("artifacts"))?;

    let stored = store.create(
        state_db.connection(),
        &draft(ErrorCode::AgentSkillFailed),
        Utc::now(),
    )?;

    let count: i64 = state_db.connection().query_row(
        "SELECT COUNT(*) FROM error_manifests WHERE id = ?1",
        params![stored.id],
        |row| row.get(0),
    )?;
    assert_eq!(count, 1);
    assert!(stored.markdown_path.exists());
    Ok(())
}

#[test]
fn ag_err_002_resolve_is_idempotent_for_existing_manifest() -> Result<()> {
    let (temp, state_db) = open_temp_db()?;
    seed_process(state_db.connection())?;
    let store = ErrorManifestStore::new(temp.path().join("artifacts"))?;
    let stored = store.create(
        state_db.connection(),
        &draft(ErrorCode::AgentOutputContractInvalid),
        Utc::now(),
    )?;

    ErrorManifestStore::resolve(state_db.connection(), &stored.id, "desktop", Utc::now())?;
    let resolved: i64 = state_db.connection().query_row(
        "SELECT resolved FROM error_manifests WHERE id = ?1",
        params![stored.id],
        |row| row.get(0),
    )?;
    assert_eq!(resolved, 1);
    Ok(())
}

#[test]
fn ag_err_003_error_types_round_trip_for_rerunnable_failures() -> Result<()> {
    let (temp, state_db) = open_temp_db()?;
    seed_process(state_db.connection())?;
    let store = ErrorManifestStore::new(temp.path().join("artifacts"))?;
    let stored = store.create(
        state_db.connection(),
        &draft(ErrorCode::AgentSuspectOutput),
        Utc::now(),
    )?;

    let record = ErrorManifestStore::load_record(state_db.connection(), &stored.id)?;
    assert_eq!(record.error_type, ErrorCode::AgentSuspectOutput);
    assert_eq!(record.pipeline_stage, "Enrich");
    Ok(())
}

#[test]
fn ag_err_004_cleanup_and_session_linkage_remain_visible_to_ui_consumers() -> Result<()> {
    let (temp, state_db) = open_temp_db()?;
    seed_process(state_db.connection())?;
    state_db.connection().execute(
        "\
        INSERT INTO agent_session_logs (
          id, skill_path, execution_mode, status
        ) VALUES ('session-1', 'skills/procurement-ingestion/fts/SKILL.md', 'pty', 'failed')
        ",
        [],
    )?;
    let store = ErrorManifestStore::new(temp.path().join("artifacts"))?;
    let mut draft = draft(ErrorCode::AgentCleanupIncomplete);
    draft.agent_session_id = Some("session-1".to_owned());
    let stored = store.create(state_db.connection(), &draft, Utc::now())?;

    let record = ErrorManifestStore::load_record(state_db.connection(), &stored.id)?;
    assert_eq!(record.agent_session_id.as_deref(), Some("session-1"));
    assert_eq!(record.process_id.as_deref(), Some("process-1"));
    Ok(())
}
