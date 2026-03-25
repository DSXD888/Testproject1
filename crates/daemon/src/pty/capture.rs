#![forbid(unsafe_code)]

use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_COMPRESSED_CHUNK_TARGET_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureConfig {
    pub compressed_chunk_target_bytes: usize,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            compressed_chunk_target_bytes: DEFAULT_COMPRESSED_CHUNK_TARGET_BYTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamIndexEntry {
    pub chunk: String,
    pub first_seq: u64,
    pub last_seq: u64,
    pub byte_length: usize,
    pub wall_ts_start: String,
    pub wall_ts_end: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturePaths {
    pub session_root: PathBuf,
    pub stream_dir: PathBuf,
    pub index_path: PathBuf,
}

impl CapturePaths {
    pub fn for_session(artifact_root: &Path, session_id: &str, now: DateTime<Utc>) -> Result<Self> {
        let session_root = artifact_root
            .join("sessions")
            .join(format!(
                "{:04}",
                now.format("%Y").to_string().parse::<u32>()?
            ))
            .join(format!(
                "{:02}",
                now.format("%m").to_string().parse::<u32>()?
            ))
            .join(session_id);
        let stream_dir = session_root.join("stream");
        let index_path = session_root.join("stream.index.jsonl");
        fs::create_dir_all(&stream_dir)
            .with_context(|| format!("creating {}", stream_dir.display()))?;
        Ok(Self {
            session_root,
            stream_dir,
            index_path,
        })
    }
}

pub struct PtyCapture {
    paths: CapturePaths,
    config: CaptureConfig,
    buffer: Vec<u8>,
    first_seq: Option<u64>,
    last_seq: Option<u64>,
    chunk_index: u64,
    wall_ts_start: Option<DateTime<Utc>>,
    wall_ts_end: Option<DateTime<Utc>>,
}

impl PtyCapture {
    pub fn new(
        artifact_root: &Path,
        session_id: &str,
        now: DateTime<Utc>,
        config: CaptureConfig,
    ) -> Result<Self> {
        let paths = CapturePaths::for_session(artifact_root, session_id, now)?;
        if !paths.index_path.exists() {
            fs::write(&paths.index_path, [])
                .with_context(|| format!("creating {}", paths.index_path.display()))?;
        }
        Ok(Self {
            paths,
            config,
            buffer: Vec::new(),
            first_seq: None,
            last_seq: None,
            chunk_index: 1,
            wall_ts_start: None,
            wall_ts_end: None,
        })
    }

    pub fn append(&mut self, seq: u64, bytes: &[u8], wall_ts: DateTime<Utc>) -> Result<()> {
        if self.first_seq.is_none() {
            self.first_seq = Some(seq);
            self.wall_ts_start = Some(wall_ts);
        }
        self.last_seq = Some(seq);
        self.wall_ts_end = Some(wall_ts);
        self.buffer.extend_from_slice(bytes);

        if estimated_compressed_len(&self.buffer)? >= self.config.compressed_chunk_target_bytes {
            self.flush_current()?;
        }

        Ok(())
    }

    pub fn finalize(&mut self) -> Result<Vec<StreamIndexEntry>> {
        if !self.buffer.is_empty() {
            self.flush_current()?;
        }
        read_stream_index(&self.paths.index_path)
    }

    pub const fn paths(&self) -> &CapturePaths {
        &self.paths
    }

    pub fn recover(paths: &CapturePaths) -> Result<Vec<StreamIndexEntry>> {
        truncate_torn_index_line(&paths.index_path)?;

        for entry in fs::read_dir(&paths.stream_dir)
            .with_context(|| format!("reading {}", paths.stream_dir.display()))?
        {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if Path::new(&file_name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("tmp"))
            {
                fs::remove_file(entry.path())
                    .with_context(|| format!("removing temp chunk {}", entry.path().display()))?;
            }
        }

        let mut index_entries = read_stream_index(&paths.index_path)?;
        let durable_chunks = durable_chunk_paths(&paths.stream_dir)?;

        if index_entries.len() != durable_chunks.len() {
            index_entries = rebuild_index_from_chunks(paths, &durable_chunks)?;
        }

        Ok(longest_contiguous_prefix(index_entries))
    }

    fn flush_current(&mut self) -> Result<()> {
        let Some(first_seq) = self.first_seq else {
            return Ok(());
        };
        let Some(last_seq) = self.last_seq else {
            return Ok(());
        };
        let Some(wall_ts_start) = self.wall_ts_start else {
            return Ok(());
        };
        let Some(wall_ts_end) = self.wall_ts_end else {
            return Ok(());
        };

        let compressed =
            zstd::stream::encode_all(self.buffer.as_slice(), 0).context("compressing PTY chunk")?;
        let chunk_file_name = format!("{:06}.pty.zst", self.chunk_index);
        let temp_path = self.paths.stream_dir.join(format!("{chunk_file_name}.tmp"));
        let final_path = self.paths.stream_dir.join(&chunk_file_name);

        fs::write(&temp_path, &compressed)
            .with_context(|| format!("writing {}", temp_path.display()))?;
        fs::rename(&temp_path, &final_path).with_context(|| {
            format!(
                "atomically renaming {} to {}",
                temp_path.display(),
                final_path.display()
            )
        })?;

        let sha256 = format!("{:x}", Sha256::digest(&compressed));
        let entry = StreamIndexEntry {
            chunk: format!("stream/{chunk_file_name}"),
            first_seq,
            last_seq,
            byte_length: compressed.len(),
            wall_ts_start: wall_ts_start.to_rfc3339(),
            wall_ts_end: wall_ts_end.to_rfc3339(),
            sha256,
        };
        append_stream_index(&self.paths.index_path, &entry)?;

        self.buffer.clear();
        self.first_seq = None;
        self.last_seq = None;
        self.wall_ts_start = None;
        self.wall_ts_end = None;
        self.chunk_index += 1;
        Ok(())
    }
}

pub fn read_stream_index(path: &Path) -> Result<Vec<StreamIndexEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("parsing stream index line"))
        .collect()
}

pub fn append_stream_index(path: &Path, entry: &StreamIndexEntry) -> Result<()> {
    let mut existing = fs::read_to_string(path).unwrap_or_default();
    existing.push_str(&serde_json::to_string(entry).context("serializing stream index entry")?);
    existing.push('\n');
    fs::write(path, existing).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn estimated_compressed_len(bytes: &[u8]) -> Result<usize> {
    Ok(zstd::stream::encode_all(bytes, 0)
        .context("estimating compressed size")?
        .len())
}

fn truncate_torn_index_line(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if content.is_empty() || content.last() == Some(&b'\n') {
        return Ok(());
    }
    let Some(last_newline) = content.iter().rposition(|byte| *byte == b'\n') else {
        fs::write(path, []).with_context(|| format!("truncating {}", path.display()))?;
        return Ok(());
    };
    fs::write(path, &content[..=last_newline])
        .with_context(|| format!("truncating {}", path.display()))?;
    Ok(())
}

fn durable_chunk_paths(stream_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(stream_dir)
        .with_context(|| format!("reading {}", stream_dir.display()))?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("zst"))
        .collect();
    paths.sort();
    Ok(paths)
}

fn rebuild_index_from_chunks(
    paths: &CapturePaths,
    durable_chunks: &[PathBuf],
) -> Result<Vec<StreamIndexEntry>> {
    let mut rebuilt = Vec::new();
    for (index, chunk_path) in durable_chunks.iter().enumerate() {
        let chunk = fs::read(chunk_path)
            .with_context(|| format!("reading durable chunk {}", chunk_path.display()))?;
        let file_name = chunk_path
            .file_name()
            .and_then(|name| name.to_str())
            .context("reading durable chunk file name")?;
        let seq = (index + 1) as u64;
        rebuilt.push(StreamIndexEntry {
            chunk: format!("stream/{file_name}"),
            first_seq: seq,
            last_seq: seq,
            byte_length: chunk.len(),
            wall_ts_start: Utc::now().to_rfc3339(),
            wall_ts_end: Utc::now().to_rfc3339(),
            sha256: format!("{:x}", Sha256::digest(&chunk)),
        });
    }

    let serialized = rebuilt
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("serializing rebuilt stream index")?
        .join("\n");
    let final_content = if serialized.is_empty() {
        String::new()
    } else {
        format!("{serialized}\n")
    };
    fs::write(&paths.index_path, final_content)
        .with_context(|| format!("rewriting {}", paths.index_path.display()))?;
    Ok(rebuilt)
}

fn longest_contiguous_prefix(entries: Vec<StreamIndexEntry>) -> Vec<StreamIndexEntry> {
    let mut prefix = Vec::new();
    let mut expected = 1;
    for entry in entries {
        if entry.first_seq != expected {
            break;
        }
        expected = entry.last_seq + 1;
        prefix.push(entry);
    }
    prefix
}

pub fn compressed_payload_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn decode_chunk(path: &Path) -> Result<Vec<u8>> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    zstd::stream::decode_all(bytes.as_slice()).context("decoding zstd chunk")
}

pub fn ensure_contiguous(entries: &[StreamIndexEntry]) -> Result<()> {
    let mut expected = None;
    for entry in entries {
        if let Some(next) = expected {
            ensure!(entry.first_seq == next, "non-contiguous stream index");
        }
        expected = Some(entry.last_seq + 1);
    }
    Ok(())
}
