#![forbid(unsafe_code)]

mod db {
    pub mod sqlite;
}

mod ipc {
    pub mod descriptor;
    pub mod handshake;
}

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use db::sqlite::{DEFAULT_SCHEMA_EPOCH, open_state_db};
use ipc::descriptor::{
    DaemonDescriptor, LockGuard, RuntimePaths, bind_listener, write_descriptor_atomic,
};
use ipc::handshake::{ActiveSessions, HandshakeContext, HealthState};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "daemon",
    version,
    about = "Layer 1 thin local runner bootstrap"
)]
struct Cli {
    /// Start the daemon, write readiness artifacts, and exit cleanly.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Runtime state root containing the local `SQLite` database.
    #[arg(long, default_value = "state")]
    state_root: PathBuf,

    /// Runtime directory containing the lock, socket, and descriptor.
    #[arg(long, default_value = "runtime")]
    runtime_dir: PathBuf,

    /// When enabled, accept exactly one handshake before exiting.
    #[arg(long, default_value_t = false)]
    serve_once: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();

    let runtime_paths = RuntimePaths::new(cli.state_root, cli.runtime_dir);
    let _lock = LockGuard::acquire(&runtime_paths)?;
    let state_db = open_state_db(runtime_paths.state_db_path(), DEFAULT_SCHEMA_EPOCH)?;
    let listener = bind_listener(&runtime_paths)?;

    let mut descriptor = DaemonDescriptor::new(
        runtime_paths.socket_path(),
        runtime_paths.state_root(),
        DEFAULT_SCHEMA_EPOCH,
    );
    write_descriptor_atomic(runtime_paths.descriptor_path(), &descriptor)?;

    tracing::info!(
        socket_path = %runtime_paths.socket_path().display(),
        state_db = %runtime_paths.state_db_path().display(),
        "daemon bootstrap ready"
    );

    if cli.serve_once {
        let handshake_context = HandshakeContext {
            protocol_version: descriptor.protocol_version,
            daemon_version: env!("CARGO_PKG_VERSION").to_owned(),
            instance_id: std::mem::take(&mut descriptor.instance_id),
            schema_epoch: DEFAULT_SCHEMA_EPOCH,
            commit_seq: state_db.current_commit_seq()?,
            health_state: HealthState {
                status: "ready".to_owned(),
            },
            active_sessions: ActiveSessions::default(),
            mutations_allowed: false,
            expected_peer_uid: None,
        };
        ipc::handshake::serve_one(&listener, &handshake_context)?;
    }

    Ok(())
}
