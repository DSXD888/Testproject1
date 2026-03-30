#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::pty::capture::{
    CaptureConfig, CapturePaths, PtyCapture, StreamIndexEntry, compressed_payload_sha256,
    decode_chunk, ensure_contiguous,
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
#[serde(rename_all = "snake_case")]
pub enum ForensicArtifactKind {
    PtyChunk,
    ContractStdout,
    ContractStderr,
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
pub struct ForensicIdentity {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_manifest_hash: Option<String>,
}

impl ForensicIdentity {
    #[must_use]
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            job_id: None,
            process_id: None,
            run_manifest_hash: None,
        }
    }

    #[must_use]
    pub fn with_job_id(mut self, job_id: impl Into<String>) -> Self {
        self.job_id = Some(job_id.into());
        self
    }

    #[must_use]
    pub fn with_process_id(mut self, process_id: impl Into<String>) -> Self {
        self.process_id = Some(process_id.into());
        self
    }

    #[must_use]
    pub fn with_run_manifest_hash(mut self, run_manifest_hash: impl Into<String>) -> Self {
        self.run_manifest_hash = Some(run_manifest_hash.into());
        self
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForensicArtifactRef {
    pub mode: ForensicMode,
    pub kind: ForensicArtifactKind,
    pub path: String,
    pub sha256: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForensicCaptureSummary {
    pub identity: ForensicIdentity,
    pub mode: ForensicMode,
    pub root_path: PathBuf,
    pub terminal_output_path: PathBuf,
    pub terminal_output_hash: String,
    pub artifact_refs: Vec<ForensicArtifactRef>,
    pub tail: String,
    pub retention: ForensicRetention,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractCapture {
    pub identity: ForensicIdentity,
    pub session_root: PathBuf,
    pub index_path: PathBuf,
    pub index_hash: String,
    pub stdout_ref: ForensicArtifactRef,
    pub stderr_ref: ForensicArtifactRef,
    pub retention: ForensicRetention,
    pub tail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ContractLaneArtifact {
    pub path: PathBuf,
    pub sha256: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ContractArtifactIndex {
    pub identity: ForensicIdentity,
    pub retention: ForensicRetention,
    pub stdout: ContractLaneArtifact,
    pub stderr: ContractLaneArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractReplay {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub struct ForensicPtyCapture {
    identity: ForensicIdentity,
    inner: PtyCapture,
}

impl ForensicPtyCapture {
    pub fn new(
        artifact_root: &Path,
        identity: ForensicIdentity,
        started_at: DateTime<Utc>,
        config: CaptureConfig,
    ) -> Result<Self> {
        Ok(Self {
            inner: PtyCapture::new(artifact_root, identity.session_id(), started_at, config)?,
            identity,
        })
    }

    pub fn append(&mut self, seq: u64, bytes: &[u8], wall_ts: DateTime<Utc>) -> Result<()> {
        self.inner.append(seq, bytes, wall_ts)
    }

    pub fn finalize(mut self, tail: impl Into<String>) -> Result<ForensicCaptureSummary> {
        let entries = self.inner.finalize()?;
        ensure_contiguous(&entries)?;
        summarize_pty_capture(&self.identity, self.inner.paths(), &entries, tail.into())
    }

    #[must_use]
    pub const fn paths(&self) -> &CapturePaths {
        self.inner.paths()
    }

    pub fn recover(
        identity: &ForensicIdentity,
        paths: &CapturePaths,
        tail: impl Into<String>,
    ) -> Result<ForensicCaptureSummary> {
        let entries = PtyCapture::recover(paths)?;
        summarize_pty_capture(identity, paths, &entries, tail.into())
    }
}

pub fn capture_contract_output(
    artifact_root: &Path,
    identity: ForensicIdentity,
    started_at: DateTime<Utc>,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<ContractCapture> {
    let session_root = session_root_for(artifact_root, identity.session_id(), started_at)?;
    let stdout_path = session_root.join("contract.stdout.zst");
    let stderr_path = session_root.join("contract.stderr.zst");
    let index_path = session_root.join("contract-output.index.json");

    let stdout_artifact = write_compressed_artifact(&stdout_path, stdout)?;
    let stderr_artifact = write_compressed_artifact(&stderr_path, stderr)?;
    let retention = ForensicRetention::default();
    let index = ContractArtifactIndex {
        identity: identity.clone(),
        retention,
        stdout: stdout_artifact.clone(),
        stderr: stderr_artifact.clone(),
    };
    let serialized_index =
        serde_json::to_vec_pretty(&index).context("serializing contract index")?;
    write_bytes_atomically(&index_path, &serialized_index)?;

    Ok(ContractCapture {
        identity,
        session_root,
        index_hash: sha256_hex(&serialized_index),
        index_path,
        stdout_ref: build_contract_ref(&stdout_artifact, ForensicArtifactKind::ContractStdout),
        stderr_ref: build_contract_ref(&stderr_artifact, ForensicArtifactKind::ContractStderr),
        retention,
        tail: tail_from_lanes(stdout, stderr),
    })
}

pub fn summarize_contract_capture(capture: ContractCapture) -> ForensicCaptureSummary {
    ForensicCaptureSummary {
        identity: capture.identity,
        mode: ForensicMode::Contract,
        root_path: capture.session_root,
        terminal_output_path: capture.index_path,
        terminal_output_hash: capture.index_hash,
        artifact_refs: vec![capture.stdout_ref, capture.stderr_ref],
        tail: capture.tail,
        retention: capture.retention,
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

pub fn replayable_contract_lanes(index_path: &Path) -> Result<ContractReplay> {
    let index = read_contract_index(index_path)?;
    let session_root = index_path
        .parent()
        .ok_or_else(|| anyhow!("index path {} has no parent", index_path.display()))?;
    Ok(ContractReplay {
        stdout: decode_contract_lane(session_root, &index.stdout.path)?,
        stderr: decode_contract_lane(session_root, &index.stderr.path)?,
    })
}

pub fn pty_capture_retention_metadata(
    identity: &ForensicIdentity,
    paths: &CapturePaths,
) -> Result<ForensicCaptureSummary> {
    let entries = PtyCapture::recover(paths)?;
    summarize_pty_capture(identity, paths, &entries, String::new())
}

pub fn contract_capture_retention_metadata(index_path: &Path) -> Result<ForensicCaptureSummary> {
    let index = read_contract_index(index_path)?;
    let session_root = index_path
        .parent()
        .ok_or_else(|| anyhow!("index path {} has no parent", index_path.display()))?;
    let index_hash = file_sha256(index_path)?;
    Ok(ForensicCaptureSummary {
        identity: index.identity,
        mode: ForensicMode::Contract,
        root_path: session_root.to_path_buf(),
        terminal_output_path: index_path.to_path_buf(),
        terminal_output_hash: index_hash,
        artifact_refs: vec![
            build_contract_ref(&index.stdout, ForensicArtifactKind::ContractStdout),
            build_contract_ref(&index.stderr, ForensicArtifactKind::ContractStderr),
        ],
        tail: tail_from_lanes(
            &decode_contract_lane(session_root, &index.stdout.path)?,
            &decode_contract_lane(session_root, &index.stderr.path)?,
        ),
        retention: index.retention,
    })
}

fn summarize_pty_capture(
    identity: &ForensicIdentity,
    paths: &CapturePaths,
    entries: &[StreamIndexEntry],
    tail: String,
) -> Result<ForensicCaptureSummary> {
    ensure_contiguous(entries)?;
    let artifact_refs = entries
        .iter()
        .map(|entry| {
            let chunk_path = paths.session_root.join(&entry.chunk);
            Ok(ForensicArtifactRef {
                mode: ForensicMode::Pty,
                kind: ForensicArtifactKind::PtyChunk,
                path: entry.chunk.clone(),
                sha256: compressed_payload_sha256(&chunk_path)?,
                size_bytes: i64::try_from(entry.byte_length)
                    .context("converting PTY chunk size to i64")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(ForensicCaptureSummary {
        identity: identity.clone(),
        mode: ForensicMode::Pty,
        root_path: paths.session_root.clone(),
        terminal_output_path: paths.index_path.clone(),
        terminal_output_hash: file_sha256(&paths.index_path)?,
        artifact_refs,
        tail,
        retention: ForensicRetention::default(),
    })
}

fn build_contract_ref(
    artifact: &ContractLaneArtifact,
    kind: ForensicArtifactKind,
) -> ForensicArtifactRef {
    ForensicArtifactRef {
        mode: ForensicMode::Contract,
        kind,
        path: artifact.path.to_string_lossy().into_owned(),
        sha256: artifact.sha256.clone(),
        size_bytes: artifact.size_bytes,
    }
}

fn decode_contract_lane(session_root: &Path, relative_path: &Path) -> Result<Vec<u8>> {
    let bytes = fs::read(session_root.join(relative_path)).with_context(|| {
        format!(
            "reading contract forensic artifact {}",
            session_root.join(relative_path).display()
        )
    })?;
    zstd::stream::decode_all(bytes.as_slice()).context("decoding contract forensic artifact")
}

fn read_contract_index(index_path: &Path) -> Result<ContractArtifactIndex> {
    let bytes =
        fs::read(index_path).with_context(|| format!("reading {}", index_path.display()))?;
    serde_json::from_slice(&bytes).context("parsing contract forensic index")
}

fn session_root_for(
    artifact_root: &Path,
    session_id: &str,
    started_at: DateTime<Utc>,
) -> Result<PathBuf> {
    let session_root = artifact_root
        .join("sessions")
        .join(started_at.format("%Y").to_string())
        .join(started_at.format("%m").to_string())
        .join(session_id);
    fs::create_dir_all(&session_root)
        .with_context(|| format!("creating {}", session_root.display()))?;
    Ok(session_root)
}

fn write_compressed_artifact(path: &Path, bytes: &[u8]) -> Result<ContractLaneArtifact> {
    let compressed = zstd::stream::encode_all(bytes, 0).context("compressing forensic lane")?;
    write_bytes_atomically(path, &compressed)?;
    Ok(ContractLaneArtifact {
        path: path
            .file_name()
            .map(PathBuf::from)
            .ok_or_else(|| anyhow!("artifact path {} has no terminal file name", path.display()))?,
        sha256: sha256_hex(&compressed),
        size_bytes: i64::try_from(compressed.len()).context("converting artifact size to i64")?,
    })
}

fn write_bytes_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp_path = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
    ));
    fs::write(&temp_path, bytes).with_context(|| format!("writing {}", temp_path.display()))?;
    fs::rename(&temp_path, path).with_context(|| {
        format!(
            "atomically renaming {} to {}",
            temp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

fn file_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(sha256_hex(&bytes))
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
