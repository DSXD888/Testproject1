#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use anyhow::Result;
use tempfile::TempDir;

#[path = "../../src/pty/capture.rs"]
mod capture_impl;

pub use capture_impl::{
    CaptureConfig, CapturePaths, PtyCapture, StreamIndexEntry, compressed_payload_sha256,
    decode_chunk, ensure_contiguous, read_stream_index,
};

pub struct ArtifactSandbox {
    _temp: TempDir,
    root: PathBuf,
}

impl ArtifactSandbox {
    pub fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("artifacts");
        std::fs::create_dir_all(&root)?;
        Ok(Self { _temp: temp, root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn session_paths(&self, session_id: &str) -> Result<CapturePaths> {
        CapturePaths::for_session(self.root(), session_id, chrono::Utc::now())
    }

    pub fn chunk_paths(&self, session_id: &str) -> Result<Vec<PathBuf>> {
        let mut paths: Vec<PathBuf> =
            std::fs::read_dir(self.session_paths(session_id)?.stream_dir)?
                .filter_map(|entry| entry.ok().map(|value| value.path()))
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("zst"))
                .collect();
        paths.sort();
        Ok(paths)
    }

    pub fn read_index(&self, session_id: &str) -> Result<Vec<StreamIndexEntry>> {
        read_stream_index(&self.session_paths(session_id)?.index_path)
    }
}
