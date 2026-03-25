#![forbid(unsafe_code)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde_yaml::Value;

use crate::support::db::{DEFAULT_SCHEMA_EPOCH, FTS_SOURCE_ID, TestStateDb};

#[test]
fn ci_core_004_runtime_inventory_matches_manifest() -> Result<()> {
    let manifest_path = repo_root()?.join("docs/pipeline/l1-daemon-core/manifest.yaml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: Value = serde_yaml::from_str(&manifest_text)
        .with_context(|| format!("parsing {}", manifest_path.display()))?;

    let runtime_entries = manifest
        .get("key_files_to_create")
        .and_then(Value::as_sequence)
        .ok_or_else(|| anyhow!("manifest.yaml missing key_files_to_create"))?;

    let runtime_paths: Vec<&str> = runtime_entries
        .iter()
        .filter_map(|entry| entry.get("path").and_then(Value::as_str))
        .collect();

    let daemon_source_paths = runtime_paths
        .iter()
        .filter(|path| path.starts_with("crates/daemon/src/"))
        .count();
    let skill_paths = runtime_paths
        .iter()
        .filter(|path| path.starts_with("skills/"))
        .count();

    assert_eq!(daemon_source_paths, 17);
    assert_eq!(skill_paths, 2);
    assert!(runtime_paths.contains(&"migrations/0003_sqlite_runtime.sql"));

    Ok(())
}

#[test]
fn it_ipc_013_state_db_bootstrap_uses_one_database() -> Result<()> {
    let ctx = support::TestContext::new()?;
    assert!(ctx.db_path.ends_with("state/state.db"));
    assert!(ctx.state_root.ends_with("state"));
    assert!(ctx.runtime_root.ends_with("runtime"));
    assert!(ctx.artifact_root.ends_with("artifacts"));
    assert!(ctx.temp.path().exists());

    let state_db = TestStateDb::new()?;
    let conn = state_db.connection();

    let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
    assert_eq!(journal_mode.to_lowercase(), "wal");

    let quick_check: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    assert_eq!(quick_check, "ok");

    let user_version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    assert_eq!(user_version, DEFAULT_SCHEMA_EPOCH);

    let commit_seq: i64 = conn.query_row(
        "SELECT commit_seq FROM commit_clock WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(commit_seq, 0);

    let (kind, enabled, config): (String, i64, String) = conn.query_row(
        "SELECT kind, enabled, config FROM sources WHERE id = ?1",
        [FTS_SOURCE_ID],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    assert_eq!(kind, "procurement");
    assert_eq!(enabled, 1);
    assert!(config.contains("find-tender.service.gov.uk"));

    let expected_tables = [
        "sources",
        "workflow_jobs",
        "process_stage_status",
        "process_stage_heads",
        "agent_session_logs",
        "raw_documents",
        "error_manifests",
        "executors",
        "source_cursors",
        "commit_clock",
    ];

    for table_name in expected_tables {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table_name],
            |row| row.get(0),
        )?;
        assert_eq!(exists, 1, "expected table {table_name} to exist");
    }

    let sibling_entries: Vec<String> =
        fs::read_dir(state_db.db_path().parent().ok_or_else(|| {
            anyhow!(
                "state db path has no parent: {}",
                state_db.db_path().display()
            )
        })?)?
        .map(|entry| entry.map(|dir_entry| dir_entry.file_name().to_string_lossy().into_owned()))
        .collect::<std::result::Result<_, _>>()?;

    assert!(!sibling_entries.iter().any(|name| name == "artifacts.db"));

    Ok(())
}

fn repo_root() -> Result<PathBuf> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            anyhow!(
                "failed to resolve repository root from {}",
                crate_dir.display()
            )
        })
}
