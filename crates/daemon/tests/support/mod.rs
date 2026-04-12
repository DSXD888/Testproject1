#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

pub mod cron;
pub mod db;
pub mod pdf;
pub mod process;
pub mod resources;
pub mod source_health;
pub mod status_router;
pub mod telemetry;
pub mod validation;

use std::path::PathBuf;

use tempfile::TempDir;

#[allow(dead_code)]
pub struct TestContext {
    pub temp: TempDir,
    pub state_root: PathBuf,
    pub runtime_root: PathBuf,
    pub artifact_root: PathBuf,
    pub db_path: PathBuf,
}

#[allow(dead_code)]
impl TestContext {
    pub fn new() -> anyhow::Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().to_path_buf();
        let state_root = root.join("state");
        Ok(Self {
            temp,
            runtime_root: root.join("runtime"),
            artifact_root: root.join("artifacts"),
            db_path: state_root.join("state.db"),
            state_root,
        })
    }
}
