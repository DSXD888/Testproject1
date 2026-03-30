#![forbid(unsafe_code)]

#[allow(dead_code)]
#[allow(clippy::missing_const_for_fn)]
#[path = "../../src/db/sqlite.rs"]
mod sqlite_impl;

use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::Connection;
use tempfile::TempDir;

#[allow(unused_imports)]
pub use sqlite_impl::{DEFAULT_SCHEMA_EPOCH, FTS_SOURCE_ID, StateDb};

#[allow(dead_code)]
pub struct TestStateDb {
    _temp: TempDir,
    db_path: PathBuf,
    state_db: StateDb,
}

#[allow(dead_code)]
impl TestStateDb {
    pub fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = sqlite_impl::open_state_db(&db_path, DEFAULT_SCHEMA_EPOCH)?;
        Ok(Self {
            _temp: temp,
            db_path,
            state_db,
        })
    }

    pub fn connection(&self) -> &Connection {
        self.state_db.connection()
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}
