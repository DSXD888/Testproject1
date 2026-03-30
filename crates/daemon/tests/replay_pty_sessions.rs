#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::all)]
#![allow(clippy::pedantic, clippy::nursery)]

#[allow(dead_code, unused_imports)]
#[path = "../src/agents/adapter.rs"]
pub mod adapter_impl;

mod agents {
    pub use crate::adapter_impl as adapter;
}

#[allow(dead_code, unused_imports)]
#[path = "../src/pty/capture.rs"]
pub mod capture_impl;

mod pty {
    pub use crate::capture_impl as capture;
}

#[allow(dead_code)]
#[path = "../src/tracing/forensic.rs"]
mod forensic_impl;
#[allow(dead_code)]
#[path = "../src/tracing/structured.rs"]
mod structured_impl;

#[cfg(unix)]
mod tests {
    use std::fs;
    use std::path::Path;

    use anyhow::Result;
    use chrono::Utc;
    use portable_pty::ExitStatus;
    use tempfile::TempDir;

    use crate::agents::adapter::{
        AuthChallengeKind, CapabilityReport, ControlEvent, ExecutionMode,
    };
    use crate::capture_impl::CapturePaths;
    use crate::forensic_impl::{
        ForensicIdentity, ForensicPtyCapture, pty_capture_retention_metadata, replayable_pty_bytes,
    };
    use crate::pty::capture::{CaptureConfig, compressed_payload_sha256, decode_chunk};
    use crate::structured_impl::{
        StructuredProtocolEventKind, StructuredTrace, TraceIdentity, trace_contains_artifact_ref,
    };

    #[test]
    fn l3_t_replay_pty_sessions_001_reassembles_split_reads_in_capture_order() -> Result<()> {
        let sandbox = TempDir::new()?;
        let identity = ForensicIdentity::new("session-replay-pty").with_job_id("job-pty-1");
        let mut capture = ForensicPtyCapture::new(
            sandbox.path(),
            identity.clone(),
            Utc::now(),
            CaptureConfig::default(),
        )?;
        capture.append(1, b"hello ", Utc::now())?;
        capture.append(2, b"world\n", Utc::now())?;
        let summary = capture.finalize("world".to_owned())?;
        let paths = CapturePaths::for_session(sandbox.path(), identity.session_id(), Utc::now())?;
        let replayed = replayable_pty_bytes(&paths)?;
        assert_eq!(replayed, b"hello world\n");
        assert_eq!(summary.identity.job_id.as_deref(), Some("job-pty-1"));
        Ok(())
    }

    #[test]
    fn l3_t_replay_pty_sessions_002_ansi_heavy_payloads_round_trip_without_corruption() -> Result<()>
    {
        let sandbox = TempDir::new()?;
        let identity = ForensicIdentity::new("session-ansi-pty");
        let mut capture = ForensicPtyCapture::new(
            sandbox.path(),
            identity.clone(),
            Utc::now(),
            CaptureConfig {
                compressed_chunk_target_bytes: 8,
            },
        )?;
        let payload = b"\x1b[31mred\x1b[0m\n\x1b[2Kdone\n";
        capture.append(1, payload, Utc::now())?;
        let _ = capture.finalize("done".to_owned())?;
        let paths = CapturePaths::for_session(sandbox.path(), identity.session_id(), Utc::now())?;
        let replayed = replayable_pty_bytes(&paths)?;
        assert_eq!(replayed, payload);
        Ok(())
    }

    #[test]
    fn l3_t_replay_pty_sessions_003_structured_trace_and_forensic_refs_remain_joinable()
    -> Result<()> {
        let sandbox = TempDir::new()?;
        let identity = ForensicIdentity::new("session-join-pty")
            .with_job_id("job-join-pty")
            .with_process_id("process-join-pty");
        let mut capture = ForensicPtyCapture::new(
            sandbox.path(),
            identity.clone(),
            Utc::now(),
            CaptureConfig::default(),
        )?;
        capture.append(1, b"joinable\n", Utc::now())?;
        let summary = capture.finalize("joinable".to_owned())?;

        let trace_identity = TraceIdentity::new("job-join-pty", "session-join-pty")
            .with_process_id("process-join-pty");
        let caps = capability_report();
        let mut trace = StructuredTrace::new(ExecutionMode::Pty, ["created", "spawned"])
            .with_capability_report(&caps);
        trace.extend_from_adapter_events(
            &trace_identity,
            &[ControlEvent::Started {
                pid: 1,
                pgid: 1,
                sid: 1,
                mode: ExecutionMode::Pty,
            }],
        );
        trace.insert_metadata(
            "forensic_artifact_refs",
            serde_json::to_value(&summary.artifact_refs)?,
        );
        let raw = trace.to_json_string()?;
        assert!(trace_contains_artifact_ref(
            &raw,
            Path::new(&summary.artifact_refs[0].path),
        )?);
        Ok(())
    }

    #[test]
    fn l3_t_replay_pty_sessions_004_recovery_truncates_torn_trailing_index_line() -> Result<()> {
        let sandbox = TempDir::new()?;
        let identity = ForensicIdentity::new("session-torn-pty");
        let mut capture = ForensicPtyCapture::new(
            sandbox.path(),
            identity.clone(),
            Utc::now(),
            CaptureConfig::default(),
        )?;
        capture.append(1, b"first\n", Utc::now())?;
        let _ = capture.finalize("first".to_owned())?;

        let paths = CapturePaths::for_session(sandbox.path(), identity.session_id(), Utc::now())?;
        let mut content = fs::read_to_string(&paths.index_path)?;
        content.push_str("{\"chunk\":\"partial");
        fs::write(&paths.index_path, content)?;

        let recovered = ForensicPtyCapture::recover(&identity, &paths, String::new())?;
        let replayed = replayable_pty_bytes(&paths)?;
        assert_eq!(replayed, b"first\n");
        assert_eq!(recovered.artifact_refs.len(), 1);
        Ok(())
    }

    #[test]
    fn l3_t_replay_pty_sessions_005_auth_challenge_replay_preserves_typed_protocol_event()
    -> Result<()> {
        let trace_identity = TraceIdentity::new("job-auth-pty", "session-auth-pty");
        let caps = capability_report();
        let mut trace =
            StructuredTrace::new(ExecutionMode::Pty, ["created"]).with_capability_report(&caps);
        trace.extend_from_adapter_events(
            &trace_identity,
            &[
                ControlEvent::Started {
                    pid: 11,
                    pgid: 11,
                    sid: 11,
                    mode: ExecutionMode::Pty,
                },
                ControlEvent::AuthChallenge {
                    kind: AuthChallengeKind::ReLogin,
                    detail: "session expired".to_owned(),
                },
                ControlEvent::Exited {
                    status: ExitStatus::with_exit_code(0),
                },
            ],
        );
        assert_eq!(
            trace.control_events,
            vec!["started".to_owned(), "exited".to_owned()]
        );
        assert_eq!(trace.protocol_events.len(), 1);
        assert_eq!(
            trace.protocol_events[0].event,
            StructuredProtocolEventKind::AuthChallenge
        );
        Ok(())
    }

    #[test]
    fn l3_t_replay_pty_sessions_006_manifest_refs_and_sha_match_the_durable_chunks() -> Result<()> {
        let sandbox = TempDir::new()?;
        let identity = ForensicIdentity::new("session-sha-pty");
        let mut capture = ForensicPtyCapture::new(
            sandbox.path(),
            identity.clone(),
            Utc::now(),
            CaptureConfig::default(),
        )?;
        capture.append(1, b"sha-check\n", Utc::now())?;
        let summary = capture.finalize("sha-check".to_owned())?;
        let paths = CapturePaths::for_session(sandbox.path(), identity.session_id(), Utc::now())?;
        let retained = pty_capture_retention_metadata(&identity, &paths)?;
        let chunk_path = paths.session_root.join(&summary.artifact_refs[0].path);
        assert_eq!(
            summary.artifact_refs[0].sha256,
            compressed_payload_sha256(&chunk_path)?
        );
        assert_eq!(retained.terminal_output_hash, summary.terminal_output_hash);
        assert_eq!(decode_chunk(&chunk_path)?, b"sha-check\n");
        Ok(())
    }

    fn capability_report() -> CapabilityReport {
        CapabilityReport {
            cli_version: "test-1.0.0".to_owned(),
            auth_healthy: true,
            structured_output: true,
            pty_viable: true,
            browser_tools: false,
            is_canary: false,
        }
    }
}
