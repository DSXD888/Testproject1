#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::Duration;

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
use serde_json::{Value, json};

use crate::ipc::command_router::{BridgeError, CommandRouter, DispatchOutcome};

const SUBSCRIPTION_IDLE_TIMEOUT: Duration = Duration::from_secs(1);

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
    pub operational_status: OperationalStatusSnapshot,
    pub mutations_allowed: bool,
    pub expected_peer_uid: Option<u32>,
    pub lifecycle_frames: Vec<LifecycleFrame>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CronDefinitionStatus {
    pub definition_key: String,
    pub source_id: String,
    pub skill_id: String,
    pub skill_version: String,
    pub cron_expression: String,
    pub enabled: bool,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CronStatusSnapshot {
    pub definitions: Vec<CronDefinitionStatus>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealthStatusEntry {
    pub source_id: String,
    pub health_state: String,
    pub circuit_state: String,
    pub production_allowed: bool,
    pub cooldown_until: Option<String>,
    pub last_canary_at: Option<String>,
    pub last_canary_result: Option<String>,
    pub last_error_code: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealthStatusSnapshot {
    pub sources: Vec<SourceHealthStatusEntry>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationStatusSnapshot {
    pub validator_approval_rate_bps: u16,
    pub validator_rejection_rate_bps: u16,
    pub validator_flag_rate_bps: u16,
    pub validator_cost_per_run_pence: u32,
    pub operator_override_rate_bps: u16,
    pub validator_execution_failure_rate_bps: u16,
    pub validator_canary_pass_rate_bps: u16,
    pub validator_health_state: String,
    pub pending_backlog_count: u32,
    pub pending_canary_count: u32,
    pub validator_job_count: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceArbiterEventSnapshot {
    pub kind: String,
    pub job_id: String,
    pub session_id: Option<String>,
    pub priority: String,
    pub denial_reason: Option<String>,
    pub release_cause: Option<String>,
    pub browser_slots_in_use: u32,
    pub agent_slots_in_use: u32,
    pub source_lane_key: Option<String>,
    pub source_lane_in_use: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceArbiterStatusSnapshot {
    pub browser_slots_limit: u32,
    pub browser_slots_in_use: u32,
    pub agent_slots_limit: u32,
    pub agent_slots_in_use: u32,
    pub procurement_source_lane_limit: u32,
    pub supplier_source_lane_limit: u32,
    pub source_lanes_in_use: BTreeMap<String, u32>,
    pub concurrency_in_use: BTreeMap<String, u32>,
    pub recent_events: Vec<ResourceArbiterEventSnapshot>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationalStatusSnapshot {
    pub cron: CronStatusSnapshot,
    pub source_health: SourceHealthStatusSnapshot,
    pub validation: ValidationStatusSnapshot,
    pub resource_arbiter: ResourceArbiterStatusSnapshot,
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
    Command {
        request_id: String,
        method: String,
        params: Value,
        idempotency_key: Option<String>,
    },
    Subscribe {
        request_id: String,
        topic: String,
        from_seq: Option<u64>,
    },
    Unsubscribe {
        request_id: String,
        topic: String,
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
        operational_status: Box<OperationalStatusSnapshot>,
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
    CommandResponse {
        request_id: String,
        payload: Value,
    },
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        code: String,
        message: String,
        retryable: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<Value>,
    },
    TopicEvent {
        topic: String,
        payload: Value,
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

pub fn serve_one(
    listener: &UnixListener,
    context: &HandshakeContext,
    command_router: Option<&mut CommandRouter<'_>>,
) -> Result<()> {
    let (stream, _) = listener
        .accept()
        .context("accepting handshake connection")?;
    handle_connection(stream, context, command_router)
}

#[allow(clippy::too_many_lines)]
pub fn handle_connection(
    mut stream: UnixStream,
    context: &HandshakeContext,
    command_router: Option<&mut CommandRouter<'_>>,
) -> Result<()> {
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

    let reader_stream = stream
        .try_clone()
        .context("cloning handshake stream for reads")?;
    let mut reader = BufReader::new(reader_stream);
    let client_message = read_message(&mut reader)?;
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
                    operational_status: Box::new(context.operational_status.clone()),
                    reconnect_outcome,
                    stream_resume_permitted: reconnect_outcome.stream_resume_permitted(),
                },
            )?;

            for frame in &context.lifecycle_frames {
                frame.validate()?;
                write_message(&mut stream, &ServerMessage::from(frame.clone()))?;
            }

            if let Some(router) = command_router {
                let mut close_when_subscription_idle = false;
                loop {
                    let message = match read_message(&mut reader) {
                        Ok(message) => message,
                        Err(error) if client_closed(&error) => break,
                        Err(error) if close_when_subscription_idle && read_timed_out(&error) => {
                            break;
                        }
                        Err(error) => return Err(error),
                    };
                    if close_when_subscription_idle {
                        reader
                            .get_mut()
                            .set_read_timeout(None)
                            .context("clearing subscription idle timeout")?;
                        close_when_subscription_idle = false;
                    }

                    let outcome = match message {
                        ClientMessage::Command {
                            request_id,
                            method,
                            params,
                            idempotency_key,
                        } => handle_command_message(
                            &mut stream,
                            router,
                            request_id,
                            &method,
                            params,
                            idempotency_key.as_deref(),
                        )?,
                        ClientMessage::Subscribe {
                            request_id,
                            topic,
                            from_seq,
                        } => {
                            handle_subscription_message(
                                &mut stream,
                                context,
                                request_id,
                                &topic,
                                from_seq,
                            )?;
                            // Subscriptions currently replay buffered topic frames over the
                            // accepted socket. The daemon accepts bridge sockets serially, so
                            // close idle subscription sockets after replay while preserving
                            // clients that immediately unsubscribe on the same socket.
                            reader
                                .get_mut()
                                .set_read_timeout(Some(SUBSCRIPTION_IDLE_TIMEOUT))
                                .context("setting subscription idle timeout")?;
                            close_when_subscription_idle = true;
                            false
                        }
                        ClientMessage::Unsubscribe { request_id, topic } => {
                            handle_unsubscription_message(&mut stream, request_id, &topic)?;
                            false
                        }
                        ClientMessage::Hello { .. } => {
                            write_bridge_error(
                                &mut stream,
                                None,
                                BridgeError::invalid_params(
                                    "hello is only valid as the first frame on the socket",
                                ),
                            )?;
                            false
                        }
                        ClientMessage::Mutation { .. } => {
                            write_bridge_error(
                                &mut stream,
                                None,
                                BridgeError {
                                    code: "mutations_unavailable".to_owned(),
                                    message: "legacy mutation frames are unsupported once the command router is active".to_owned(),
                                    retryable: false,
                                    details: None,
                                },
                            )?;
                            false
                        }
                    };

                    if outcome {
                        break;
                    }
                }
            }
        }
        ClientMessage::Command { .. }
        | ClientMessage::Subscribe { .. }
        | ClientMessage::Unsubscribe { .. }
        | ClientMessage::Mutation { .. } => {
            write_message(
                &mut stream,
                &ServerMessage::Rejection {
                    code: "mutations_unavailable".to_owned(),
                    message:
                        "command traffic is unavailable until the layer 2 command router is active"
                            .to_owned(),
                    supported_protocol_range: None,
                    expected_schema_epoch: None,
                    actual_schema_epoch: None,
                },
            )?;
        }
    }

    Ok(())
}

fn handle_command_message(
    stream: &mut UnixStream,
    router: &mut CommandRouter<'_>,
    request_id: String,
    method: &str,
    params: Value,
    idempotency_key: Option<&str>,
) -> Result<bool> {
    match router.dispatch(method, params, idempotency_key) {
        Ok(DispatchOutcome::Response(payload)) => {
            write_message(
                stream,
                &ServerMessage::CommandResponse {
                    request_id,
                    payload,
                },
            )?;
            Ok(false)
        }
        Ok(DispatchOutcome::DisconnectAfterResponse(payload)) => {
            write_message(
                stream,
                &ServerMessage::CommandResponse {
                    request_id,
                    payload,
                },
            )?;
            Ok(true)
        }
        Err(error) => {
            write_bridge_error(stream, Some(request_id), error)?;
            Ok(false)
        }
    }
}

fn handle_subscription_message(
    stream: &mut UnixStream,
    context: &HandshakeContext,
    request_id: String,
    topic: &str,
    from_seq: Option<u64>,
) -> Result<()> {
    if !supported_subscription_topic(topic) {
        write_bridge_error(
            stream,
            Some(request_id),
            BridgeError::invalid_params(format!("unsupported subscription topic `{topic}`")),
        )?;
        return Ok(());
    }

    write_message(
        stream,
        &ServerMessage::CommandResponse {
            request_id,
            payload: Value::Null,
        },
    )?;

    if topic == "daemon.health" {
        write_message(
            stream,
            &ServerMessage::TopicEvent {
                topic: topic.to_owned(),
                payload: json!({
                    "daemon": context.health_state.status,
                    "control": if context.mutations_allowed { "full" } else { "read_only" },
                    "load": "normal",
                    "reasons": Vec::<String>::new(),
                    "daemonInstanceId": context.instance_id,
                    "daemonVersion": context.daemon_version,
                    "protocolVersion": context.protocol_version,
                    "schemaEpoch": context.schema_epoch,
                }),
            },
        )?;
        return Ok(());
    }

    let session_id = topic
        .strip_prefix("session.meta.")
        .or_else(|| topic.strip_prefix("session.stream."));
    let min_seq = from_seq.unwrap_or(0);
    for frame in context.lifecycle_frames.iter().filter(|frame| {
        frame.seq > min_seq
            && session_id.is_none_or(|expected_session_id| frame.session_id == expected_session_id)
    }) {
        frame.validate()?;
        write_message(
            stream,
            &ServerMessage::TopicEvent {
                topic: topic.to_owned(),
                payload: json!({
                    "sessionId": frame.session_id,
                    "frame": {
                        "kind": "lifecycle",
                        "seq": frame.seq,
                        "state": lifecycle_phase_label(frame.phase),
                        "cleanupVerdict": frame.cleanup_verdict,
                        "archiveSeq": frame.archive_seq,
                        "ts": Value::Null,
                    },
                }),
            },
        )?;
    }

    Ok(())
}

fn handle_unsubscription_message(
    stream: &mut UnixStream,
    request_id: String,
    topic: &str,
) -> Result<()> {
    if !supported_subscription_topic(topic) {
        write_bridge_error(
            stream,
            Some(request_id),
            BridgeError::invalid_params(format!("unsupported subscription topic `{topic}`")),
        )?;
        return Ok(());
    }

    write_message(
        stream,
        &ServerMessage::CommandResponse {
            request_id,
            payload: Value::Null,
        },
    )
}

fn supported_subscription_topic(topic: &str) -> bool {
    topic == "daemon.health"
        || topic == "sessions"
        || topic.starts_with("session.meta.")
        || topic.starts_with("session.stream.")
}

const fn lifecycle_phase_label(phase: LifecyclePhase) -> &'static str {
    match phase {
        LifecyclePhase::Created => "created",
        LifecyclePhase::Spawned => "spawned",
        LifecyclePhase::Active => "active",
        LifecyclePhase::Terminating => "terminating",
        LifecyclePhase::Exited => "exited",
        LifecyclePhase::Reaped => "reaped",
        LifecyclePhase::Archived => "archived",
    }
}

fn write_bridge_error(
    stream: &mut UnixStream,
    request_id: Option<String>,
    error: BridgeError,
) -> Result<()> {
    write_message(
        stream,
        &ServerMessage::Error {
            request_id,
            code: error.code,
            message: error.message,
            retryable: error.retryable,
            details: error.details,
        },
    )
}

fn client_closed(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.to_string().contains("client closed handshake stream"))
}

fn read_timed_out(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io_error| {
                matches!(
                    io_error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                )
            })
    })
}

fn reconnect_outcome(last_seen_instance_id: Option<&str>, instance_id: &str) -> ReconnectOutcome {
    match last_seen_instance_id {
        None => ReconnectOutcome::FreshConnection,
        Some(last_seen) if last_seen == instance_id => ReconnectOutcome::SameDaemonReconnect,
        Some(_) => ReconnectOutcome::DaemonRestartInvalidated,
    }
}

fn read_message<R>(reader: &mut R) -> Result<ClientMessage>
where
    R: BufRead,
{
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
