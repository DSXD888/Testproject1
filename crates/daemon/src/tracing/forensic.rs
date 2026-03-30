#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::pty::capture::{
    CaptureConfig, CapturePaths, PtyCapture, StreamIndexEntry, compressed_payload_sha256,
    decode_chunk, ensure_contiguous, read_stream_index,
};

pub const FORENSIC_BYTES_RETENTION_DAYS: u32 = 14;
pub const MANIFEST_LINKED_HOLD_DAYS: u32 = 180;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForensicMode {
    Pty,
    Contract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForensicRetention {
    pub base_retention_days: u32,
    pub linked_hold_days: u32,
}

impl Default for ForensicRetention {
    fn default() -> Self {
        Self {
            base_retention_days: FORENSIC_BYTES_RETENTION_DAYS,
            linked_hold_days: MANIFEST_LINKED_HOLD_DAYS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForensicArtifactRef {
    pub mode: ForensicMode,
    pub path: String,
    pub sha256: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForensicCaptureSummary {
    pub mode: ForensicMode,
    pub root_path: PathBuf,
    pub terminal_output_path: PathBuf,
    pub artifact_refs: Vec<ForensicArtifactRef>,
    pub tail: String,
    pub retention: ForensicRetention,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractCapture {
    pub path: PathBuf,
    pub sha256: String,
    pub size_bytes: i64,
    pub tail: String,
}

pub struct ForensicPtyCapture {
    inner: PtyCapture,
}

impl ForensicPtyCapture {
    pub fn new(
        artifact_root: &Path,
        session_id: &str,
        started_at: DateTime<Utc>,
        config: CaptureConfig,
    ) -> Result<Self> {
        Ok(Self {
            inner: PtyCapture::new(artifact_root, session_id, started_at, config)?,
        })
    }

    pub fn append(&mut self, seq: u64, bytes: &[u8], wall_ts: DateTime<Utc>) -> Result<()> {
        self.inner.append(seq, bytes, wall_ts)
    }

    pub fn finalize(mut self, tail: impl Into<String>) -> Result<ForensicCaptureSummary> {
        let entries = self.inner.finalize()?;
        ensure_contiguous(&entries)?;
        summarize_pty_capture(self.inner.paths(), &entries, tail.into())
    }

    #[must_use]
    pub const fn paths(&self) -> &CapturePaths {
        self.inner.paths()
    }

    pub fn recover(paths: &CapturePaths, tail: impl Into<String>) -> Result<ForensicCaptureSummary> {
        let entries = PtyCapture::recover(paths)?;
        summarize_pty_capture(paths, &entries, tail.into())
    }
}

pub fn capture_contract_output(
    workdir: &Path,
    session_id: &str,
    started_at: DateTime<Utc>,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<ContractCapture> {
    let session_dir = workdir
        .parent()
        .ok_or_else(|| anyhow!("workdir {} has no parent", workdir.display()))?;
    let path = session_dir.join("contract-output.log");
    let mut payload = Vec::new();
    payload.extend_from_slice(b"--- stdout ---\n");
    payload.extend_from_slice(stdout);
    payload.extend_from_slice(b"\n--- stderr ---\n");
    payload.extend_from_slice(stderr);
    fs::write(&path, &payload).with_context(|| format!("writing {}", path.display()))?;
    let _ = (session_id, started_at);
    Ok(ContractCapture {
        path,
        sha256: sha256_hex(&payload),
        size_bytes: i64::try_from(payload.len()).context("converting contract capture size")?,
        tail: tail_from_lanes(stdout, stderr),
    })
}

pub fn summarize_contract_capture(capture: ContractCapture) -> ForensicCaptureSummary {
    let artifact_ref = ForensicArtifactRef {
        mode: ForensicMode::Contract,
        path: capture.path.to_string_lossy().into_owned(),
        sha256: capture.sha256,
        size_bytes: capture.size_bytes,
    };
    ForensicCaptureSummary {
        mode: ForensicMode::Contract,
        root_path: capture
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")),
        terminal_output_path: capture.path,
        artifact_refs: vec![artifact_ref],
        tail: capture.tail,
        retention: ForensicRetention::default(),
    }
}

pub fn replayable_pty_bytes(paths: &CapturePaths) -> Result<Vec<u8>> {
    let entries = PtyCapture::recover(paths)?;
    let mut bytes = Vec::new();
    for entry in entries {
        let chunk_path = paths.session_root.join(&entry.chunk);
        bytes.extend_from_slice(&decode_chunk(&chunk_path)?);
    }
    Ok(bytes)
}

pub fn pty_capture_retention_metadata(paths: &CapturePaths) -> Result<ForensicCaptureSummary> {
    let entries = read_stream_index(&paths.index_path)?;
    summarize_pty_capture(paths, &entries, String::new())
}

fn summarize_pty_capture(
    paths: &CapturePaths,
    entries: &[StreamIndexEntry],
    tail: String,
) -> Result<ForensicCaptureSummary> {
    let artifact_refs = entries
        .iter()
        .map(|entry| {
            let chunk_path = paths.session_root.join(&entry.chunk);
            Ok(ForensicArtifactRef {
                mode: ForensicMode::Pty,
                path: entry.chunk.clone(),
                sha256: compressed_payload_sha256(&chunk_path)?,
                size_bytes: i64::try_from(entry.byte_length)
                    .context("converting PTY chunk size to i64")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(ForensicCaptureSummary {
        mode: ForensicMode::Pty,
        root_path: paths.session_root.clone(),
        terminal_output_path: paths.index_path.clone(),
        artifact_refs,
        tail,
        retention: ForensicRetention::default(),
    })
}

fn tail_from_lanes(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout_text = String::from_utf8_lossy(stdout).into_owned();
    let stderr_text = String::from_utf8_lossy(stderr).into_owned();
    stdout_text
        .lines()
        .last()
        .or_else(|| stderr_text.lines().last())
        .unwrap_or_default()
        .to_owned()
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
