#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result, anyhow, bail, ensure};
use rusqlite::{Connection, Transaction, TransactionBehavior, params};
use serde_json::json;

#[allow(dead_code)]
static WRITE_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
const L0_MIGRATION_PATH: &str = "migrations/0001_sqlite_state_initial.sql";
const L1_MIGRATION_PATH: &str = "migrations/0003_sqlite_runtime.sql";

pub const DEFAULT_SCHEMA_EPOCH: i64 = 1;
pub const DEFAULT_OWNER_ID: &str = "local-owner";
pub const FTS_SOURCE_ID: &str = "fts";

#[derive(Clone, Debug)]
pub struct StateDbConfig {
    pub schema_epoch: i64,
    #[allow(dead_code)]
    pub owner_id: String,
    pub fts_base_url: String,
    pub fts_ocds_api_url: String,
    pub fts_requests_per_minute: u32,
}

impl Default for StateDbConfig {
    fn default() -> Self {
        Self {
            schema_epoch: DEFAULT_SCHEMA_EPOCH,
            owner_id: DEFAULT_OWNER_ID.to_owned(),
            fts_base_url: "https://www.find-tender.service.gov.uk".to_owned(),
            fts_ocds_api_url: "https://www.find-tender.service.gov.uk/api/1.0/ocdsReleasePackages"
                .to_owned(),
            fts_requests_per_minute: 60,
        }
    }
}

pub struct StateDb {
    conn: Connection,
    #[allow(dead_code)]
    config: StateDbConfig,
}

#[allow(dead_code)]
impl StateDb {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_config(path, StateDbConfig::default())
    }

    pub fn open_with_config(path: impl AsRef<Path>, config: StateDbConfig) -> Result<Self> {
        let repo_root = repo_root()?;
        Self::open_in_repo(path, &repo_root, config)
    }

    pub fn open_in_repo(
        path: impl AsRef<Path>,
        repo_root: &Path,
        config: StateDbConfig,
    ) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating state db parent {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("opening SQLite database {}", path.display()))?;
        configure_pragmas(&conn)?;
        apply_migrations(&conn, repo_root, config.schema_epoch)?;
        seed_commit_clock(&conn)?;
        seed_sources(&conn, &config)?;
        run_quick_check(&conn)?;

        Ok(Self { conn, config })
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn schema_epoch(&self) -> i64 {
        self.config.schema_epoch
    }

    pub fn current_commit_seq(&self) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT commit_seq FROM commit_clock WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .context("reading commit clock")
    }

    pub fn with_write_transaction<T, F>(&mut self, f: F) -> Result<(T, i64)>
    where
        F: FnOnce(&Transaction<'_>) -> Result<T>,
    {
        let write_guard = write_mutex()
            .lock()
            .map_err(|_| anyhow!("write mutex poisoned"))?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("starting SQLite write transaction")?;
        let value = f(&tx)?;
        let commit_seq = increment_commit_clock(&tx)?;
        tx.commit().context("committing SQLite write transaction")?;
        drop(write_guard);
        Ok((value, commit_seq))
    }
}

pub fn open_state_db(path: impl AsRef<Path>, schema_epoch: i64) -> Result<StateDb> {
    let config = StateDbConfig {
        schema_epoch,
        ..StateDbConfig::default()
    };
    StateDb::open_with_config(path, config)
}

#[allow(dead_code)]
fn write_mutex() -> &'static Mutex<()> {
    WRITE_MUTEX.get_or_init(|| Mutex::new(()))
}

fn configure_pragmas(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "\
        PRAGMA journal_mode = WAL;\
        PRAGMA synchronous = NORMAL;\
        PRAGMA foreign_keys = ON;\
        PRAGMA busy_timeout = 5000;\
        PRAGMA cache_size = -64000;\
        ",
    )
    .context("configuring SQLite PRAGMAs")?;
    Ok(())
}

fn apply_migrations(conn: &Connection, repo_root: &Path, schema_epoch: i64) -> Result<()> {
    let current_epoch: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .context("reading PRAGMA user_version")?;

    if current_epoch != 0 && current_epoch != schema_epoch {
        bail!("schema epoch mismatch: database has {current_epoch}, daemon expects {schema_epoch}");
    }

    apply_sql_file(conn, repo_root, L0_MIGRATION_PATH)?;
    apply_sql_file(conn, repo_root, L1_MIGRATION_PATH)?;
    ensure_runtime_schema(conn)?;
    conn.pragma_update(None, "user_version", schema_epoch)
        .context("writing PRAGMA user_version")?;
    Ok(())
}

fn apply_sql_file(conn: &Connection, repo_root: &Path, relative_path: &str) -> Result<()> {
    let path = repo_root.join(relative_path);
    let ddl = fs::read_to_string(&path)
        .with_context(|| format!("reading migration {}", path.display()))?;
    conn.execute_batch(&ddl)
        .with_context(|| format!("applying migration {}", path.display()))?;
    Ok(())
}

fn ensure_runtime_schema(conn: &Connection) -> Result<()> {
    ensure_table_columns(
        conn,
        "agent_session_logs",
        &[
            (
                "adapter_id",
                "ALTER TABLE agent_session_logs ADD COLUMN adapter_id TEXT",
            ),
            (
                "sid",
                "ALTER TABLE agent_session_logs ADD COLUMN sid INTEGER",
            ),
            (
                "exit_code",
                "ALTER TABLE agent_session_logs ADD COLUMN exit_code INTEGER",
            ),
        ],
    )?;
    Ok(())
}

fn ensure_table_columns(conn: &Connection, table: &str, columns: &[(&str, &str)]) -> Result<()> {
    let existing_columns = table_columns(conn, table)?;
    for (column_name, alter_sql) in columns {
        if existing_columns
            .iter()
            .any(|existing| existing == column_name)
        {
            continue;
        }
        conn.execute(alter_sql, [])
            .with_context(|| format!("adding {column_name} to {table}"))?;
    }
    Ok(())
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .with_context(|| format!("preparing table_info query for {table}"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("reading column metadata for {table}"))
}

fn seed_commit_clock(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO commit_clock (id, commit_seq) VALUES (1, 0)",
        [],
    )
    .context("seeding commit_clock")?;
    Ok(())
}

fn seed_sources(conn: &Connection, config: &StateDbConfig) -> Result<()> {
    let source_config = json!({
        "baseUrl": config.fts_base_url,
        "ocdsApiUrl": config.fts_ocds_api_url,
        "rateLimits": {
            "requestsPerMinute": config.fts_requests_per_minute
        }
    })
    .to_string();

    conn.execute(
        "\
        INSERT INTO sources (id, kind, enabled, config)
        VALUES (?1, 'procurement', 1, ?2)
        ON CONFLICT(id) DO UPDATE SET
          kind = excluded.kind,
          enabled = excluded.enabled,
          config = excluded.config
        ",
        params![FTS_SOURCE_ID, source_config],
    )
    .context("seeding FTS source definition")?;

    conn.execute(
        "\
        INSERT OR IGNORE INTO source_runtime_state (source_id, last_health_state)
        VALUES (?1, 'unknown')
        ",
        params![FTS_SOURCE_ID],
    )
    .context("seeding FTS source runtime state")?;

    Ok(())
}

fn run_quick_check(conn: &Connection) -> Result<()> {
    let quick_check: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .context("running PRAGMA quick_check")?;
    ensure!(
        quick_check == "ok",
        "PRAGMA quick_check failed: {quick_check}"
    );
    Ok(())
}

#[allow(dead_code)]
fn increment_commit_clock(tx: &Transaction<'_>) -> Result<i64> {
    let updated = tx
        .execute(
            "UPDATE commit_clock SET commit_seq = commit_seq + 1 WHERE id = 1",
            [],
        )
        .context("incrementing commit_clock")?;
    ensure!(updated == 1, "commit_clock singleton row missing");

    tx.query_row(
        "SELECT commit_seq FROM commit_clock WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .context("reading incremented commit_seq")
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
