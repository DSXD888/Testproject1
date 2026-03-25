#![forbid(unsafe_code)]

use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, Result, bail};
use chrono::Utc;
use fs4::FileExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DESCRIPTOR_FILENAME: &str = "daemon.info.json";
pub const LOCK_FILENAME: &str = "daemon.lock";
pub const SOCKET_FILENAME: &str = "daemon.sock";
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct RuntimePaths {
    state_root: PathBuf,
    runtime_dir: PathBuf,
    socket_path: PathBuf,
    descriptor_path: PathBuf,
    lock_path: PathBuf,
}

impl RuntimePaths {
    pub fn new(state_root: PathBuf, runtime_dir: PathBuf) -> Self {
        let socket_path = runtime_dir.join(SOCKET_FILENAME);
        let descriptor_path = runtime_dir.join(DESCRIPTOR_FILENAME);
        let lock_path = runtime_dir.join(LOCK_FILENAME);
        Self {
            state_root,
            runtime_dir,
            socket_path,
            descriptor_path,
            lock_path,
        }
    }

    pub fn state_root(&self) -> &Path {
        &self.state_root
    }

    pub fn runtime_dir(&self) -> &Path {
        &self.runtime_dir
    }

    #[allow(dead_code)]
    pub fn state_db_path(&self) -> PathBuf {
        self.state_root.join("state.db")
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn descriptor_path(&self) -> &Path {
        &self.descriptor_path
    }

    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DaemonDescriptor {
    pub instance_id: String,
    pub pid: u32,
    pub socket_path: String,
    pub protocol_version: u32,
    pub schema_epoch: i64,
    pub state_root: String,
    pub started_at: String,
    pub ready: bool,
}

impl DaemonDescriptor {
    pub fn new(socket_path: &Path, state_root: &Path, schema_epoch: i64) -> Self {
        Self {
            instance_id: Uuid::new_v4().to_string(),
            pid: process::id(),
            socket_path: socket_path.display().to_string(),
            protocol_version: PROTOCOL_VERSION,
            schema_epoch,
            state_root: state_root.display().to_string(),
            started_at: Utc::now().to_rfc3339(),
            ready: true,
        }
    }
}

pub struct LockGuard {
    file: File,
}

impl LockGuard {
    pub fn acquire(paths: &RuntimePaths) -> Result<Self> {
        ensure_runtime_dir(paths.runtime_dir())?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(paths.lock_path())
            .with_context(|| format!("opening {}", paths.lock_path().display()))?;
        file.try_lock_exclusive()
            .with_context(|| format!("locking {}", paths.lock_path().display()))?;
        Ok(Self { file })
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

pub fn ensure_runtime_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("creating {}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("setting permissions on {}", path.display()))?;
    Ok(())
}

pub fn bind_listener(paths: &RuntimePaths) -> Result<UnixListener> {
    ensure_runtime_dir(paths.runtime_dir())?;
    cleanup_stale_socket(paths.socket_path(), paths.descriptor_path())?;
    let listener = UnixListener::bind(paths.socket_path())
        .with_context(|| format!("binding {}", paths.socket_path().display()))?;
    Ok(listener)
}

pub fn write_descriptor_atomic(path: &Path, descriptor: &DaemonDescriptor) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("descriptor path has no parent: {}", path.display()))?;
    ensure_runtime_dir(parent)?;
    let temp_path = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(descriptor).context("serializing daemon descriptor")?;
    fs::write(&temp_path, bytes).with_context(|| format!("writing {}", temp_path.display()))?;
    fs::rename(&temp_path, path)
        .with_context(|| format!("renaming {} to {}", temp_path.display(), path.display()))?;
    Ok(())
}

fn cleanup_stale_socket(socket_path: &Path, descriptor_path: &Path) -> Result<()> {
    match fs::symlink_metadata(socket_path) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("probing {}", socket_path.display()));
        }
    }

    match UnixStream::connect(socket_path) {
        Ok(_) => {
            let instance_hint = read_descriptor_if_present(descriptor_path)?
                .map(|descriptor| format!(" for daemon instance {}", descriptor.instance_id))
                .unwrap_or_default();
            bail!(
                "socket {} is already accepting connections{}",
                socket_path.display(),
                instance_hint
            );
        }
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::ConnectionRefused | ErrorKind::NotFound | ErrorKind::ConnectionAborted
            ) =>
        {
            fs::remove_file(socket_path)
                .with_context(|| format!("removing stale socket {}", socket_path.display()))?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error)
                .with_context(|| format!("probing live socket {}", socket_path.display()));
        }
    }

    Ok(())
}

fn read_descriptor_if_present(path: &Path) -> Result<Option<DaemonDescriptor>> {
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents)
            .with_context(|| format!("parsing {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
    }
}
