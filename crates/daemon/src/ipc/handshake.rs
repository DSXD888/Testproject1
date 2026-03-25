#![forbid(unsafe_code)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};

use anyhow::{Context, Result, anyhow, bail};
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::sys::socket::{getsockopt, sockopt};
use nix::unistd::Uid;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
use nix::unistd::getpeereid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSessions {
    pub count: usize,
    pub ids: Vec<String>,
    pub states: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthState {
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct HandshakeContext {
    pub protocol_version: u32,
    pub daemon_version: String,
    pub instance_id: String,
    pub schema_epoch: i64,
    pub commit_seq: i64,
    pub health_state: HealthState,
    pub active_sessions: ActiveSessions,
    pub mutations_allowed: bool,
    pub expected_peer_uid: Option<u32>,
    pub lifecycle_frames: Vec<LifecycleFrame>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolRange {
    pub min: u32,
    pub max: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "messageType",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ClientMessage {
    Hello {
        desktop_version: String,
        supported_protocol_range: ProtocolRange,
        expected_schema_epoch: i64,
        last_seen_instance_id: Option<String>,
    },
    Mutation {
        command: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "messageType",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ServerMessage {
    Welcome {
        selected_protocol_version: u32,
        daemon_version: String,
        instance_id: String,
        schema_epoch: i64,
        commit_seq: i64,
        health_state: HealthState,
        mutations_allowed: bool,
        active_sessions: ActiveSessions,
        reconnect_outcome: ReconnectOutcome,
        stream_resume_permitted: bool,
    },
    Rejection {
        code: String,
        message: String,
        supported_protocol_range: Option<ProtocolRange>,
        expected_schema_epoch: Option<i64>,
        actual_schema_epoch: Option<i64>,
    },
    Lifecycle {
        seq: u64,
        session_id: String,
        phase: LifecyclePhase,
        #[serde(skip_serializing_if = "Option::is_none")]
        cleanup_verdict: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        archive_seq: Option<i64>,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReconnectOutcome {
    FreshConnection,
    SameDaemonReconnect,
    DaemonRestartInvalidated,
}

impl ReconnectOutcome {
    pub const fn stream_resume_permitted(self) -> bool {
        matches!(self, Self::SameDaemonReconnect)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LifecyclePhase {
    Created,
    Spawned,
    Active,
    Terminating,
    Exited,
    Reaped,
    Archived,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleFrame {
    pub seq: u64,
    pub session_id: String,
    pub phase: LifecyclePhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup_verdict: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_seq: Option<i64>,
}

impl LifecycleFrame {
    fn validate(&self) -> Result<()> {
        if self.phase == LifecyclePhase::Reaped && self.cleanup_verdict.is_none() {
            bail!(
                "reaped lifecycle frame {} is missing cleanupVerdict",
                self.seq
            );
        }
        if self.phase != LifecyclePhase::Reaped && self.cleanup_verdict.is_some() {
            bail!(
                "only reaped lifecycle frames may carry cleanupVerdict (seq {})",
                self.seq
            );
        }
        if self.phase == LifecyclePhase::Archived && self.archive_seq.is_none() {
            bail!(
                "archived lifecycle frame {} is missing archiveSeq",
                self.seq
            );
        }
        if self.phase != LifecyclePhase::Archived && self.archive_seq.is_some() {
            bail!(
                "only archived lifecycle frames may carry archiveSeq (seq {})",
                self.seq
            );
        }
        Ok(())
    }
}

pub fn serve_one(listener: &UnixListener, context: &HandshakeContext) -> Result<()> {
    let (stream, _) = listener
        .accept()
        .context("accepting handshake connection")?;
    handle_connection(stream, context)
}

pub fn handle_connection(mut stream: UnixStream, context: &HandshakeContext) -> Result<()> {
    let expected_uid = context
        .expected_peer_uid
        .unwrap_or_else(|| Uid::current().as_raw());
    if let Err(error) = verify_peer_uid(&stream, expected_uid) {
        write_message(
            &mut stream,
            &ServerMessage::Rejection {
                code: "peer_uid_mismatch".to_owned(),
                message: error.to_string(),
                supported_protocol_range: None,
                expected_schema_epoch: None,
                actual_schema_epoch: None,
            },
        )?;
        return Ok(());
    }

    let client_message = read_message(&stream)?;
    match client_message {
        ClientMessage::Hello {
            supported_protocol_range,
            expected_schema_epoch,
            last_seen_instance_id,
            ..
        } => {
            if supported_protocol_range.min > context.protocol_version
                || supported_protocol_range.max < context.protocol_version
            {
                write_message(
                    &mut stream,
                    &ServerMessage::Rejection {
                        code: "protocol_range_mismatch".to_owned(),
                        message: "protocol ranges do not overlap".to_owned(),
                        supported_protocol_range: Some(ProtocolRange {
                            min: context.protocol_version,
                            max: context.protocol_version,
                        }),
                        expected_schema_epoch: None,
                        actual_schema_epoch: None,
                    },
                )?;
                return Ok(());
            }

            if expected_schema_epoch != context.schema_epoch {
                write_message(
                    &mut stream,
                    &ServerMessage::Rejection {
                        code: "schema_epoch_mismatch".to_owned(),
                        message: "schema epoch mismatch".to_owned(),
                        supported_protocol_range: None,
                        expected_schema_epoch: Some(expected_schema_epoch),
                        actual_schema_epoch: Some(context.schema_epoch),
                    },
                )?;
                return Ok(());
            }

            let reconnect_outcome =
                reconnect_outcome(last_seen_instance_id.as_deref(), &context.instance_id);
            write_message(
                &mut stream,
                &ServerMessage::Welcome {
                    selected_protocol_version: context.protocol_version,
                    daemon_version: context.daemon_version.clone(),
                    instance_id: context.instance_id.clone(),
                    schema_epoch: context.schema_epoch,
                    commit_seq: context.commit_seq,
                    health_state: context.health_state.clone(),
                    mutations_allowed: context.mutations_allowed,
                    active_sessions: context.active_sessions.clone(),
                    reconnect_outcome,
                    stream_resume_permitted: reconnect_outcome.stream_resume_permitted(),
                },
            )?;

            for frame in &context.lifecycle_frames {
                frame.validate()?;
                write_message(&mut stream, &ServerMessage::from(frame.clone()))?;
            }
        }
        ClientMessage::Mutation { .. } => {
            write_message(
                &mut stream,
                &ServerMessage::Rejection {
                    code: "mutations_unavailable".to_owned(),
                    message: "layer 1 only accepts hello/welcome handshake traffic".to_owned(),
                    supported_protocol_range: None,
                    expected_schema_epoch: None,
                    actual_schema_epoch: None,
                },
            )?;
        }
    }

    Ok(())
}

fn reconnect_outcome(last_seen_instance_id: Option<&str>, instance_id: &str) -> ReconnectOutcome {
    match last_seen_instance_id {
        None => ReconnectOutcome::FreshConnection,
        Some(last_seen) if last_seen == instance_id => ReconnectOutcome::SameDaemonReconnect,
        Some(_) => ReconnectOutcome::DaemonRestartInvalidated,
    }
}

fn read_message(stream: &UnixStream) -> Result<ClientMessage> {
    let mut reader = BufReader::new(stream);
    let mut buffer = String::new();
    let bytes = reader
        .read_line(&mut buffer)
        .context("reading client handshake frame")?;
    if bytes == 0 {
        bail!("client closed handshake stream before sending a frame");
    }
    serde_json::from_str(buffer.trim_end()).context("parsing client handshake frame")
}

fn write_message(stream: &mut UnixStream, message: &ServerMessage) -> Result<()> {
    let payload = serde_json::to_string(message).context("serializing handshake response")?;
    stream
        .write_all(payload.as_bytes())
        .context("writing handshake response")?;
    stream
        .write_all(b"\n")
        .context("writing handshake response terminator")?;
    stream.flush().context("flushing handshake response")?;
    Ok(())
}

fn verify_peer_uid(stream: &UnixStream, expected_uid: u32) -> Result<()> {
    let peer_uid = peer_uid(stream)?;
    if peer_uid != expected_uid {
        bail!("peer uid mismatch: expected {expected_uid}, got {peer_uid}");
    }
    Ok(())
}

fn peer_uid(stream: &UnixStream) -> Result<u32> {
    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        let (uid, _) = getpeereid(stream).context("reading peer uid with getpeereid")?;
        return Ok(uid.as_raw());
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let credentials =
            getsockopt(stream, sockopt::PeerCredentials).context("reading peer credentials")?;
        return Ok(credentials.uid());
    }

    #[allow(unreachable_code)]
    Err(anyhow!(
        "peer credential verification is unsupported on this platform"
    ))
}

impl From<LifecycleFrame> for ServerMessage {
    fn from(frame: LifecycleFrame) -> Self {
        Self::Lifecycle {
            seq: frame.seq,
            session_id: frame.session_id,
            phase: frame.phase,
            cleanup_verdict: frame.cleanup_verdict,
            archive_seq: frame.archive_seq,
        }
    }
}
