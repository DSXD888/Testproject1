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

    use crate::agents::adapter::{CapabilityReport, ControlEvent, ExecutionMode};
    use crate::forensic_impl::{
        ForensicIdentity, capture_contract_output, contract_capture_retention_metadata,
        replayable_contract_lanes, summarize_contract_capture,
    };
    use crate::structured_impl::{
        StructuredProtocolEventKind, StructuredTrace, TraceIdentity, trace_contains_artifact_ref,
    };

    #[test]
    fn l3_t_replay_contract_sessions_001_preserves_separate_stdout_and_stderr_lanes() -> Result<()>
    {
        let sandbox = TempDir::new()?;
        let identity =
            ForensicIdentity::new("session-contract-lanes").with_job_id("job-contract-1");
        let capture = capture_contract_output(
            sandbox.path(),
            identity,
            Utc::now(),
            b"stdout-line\n",
            b"stderr-line\n",
        )?;
        let replay = replayable_contract_lanes(&capture.index_path)?;
        assert_eq!(replay.stdout, b"stdout-line\n");
        assert_eq!(replay.stderr, b"stderr-line\n");
        Ok(())
    }

    #[test]
    fn l3_t_replay_contract_sessions_002_malformed_structured_payloads_fail_explicitly() {
        let error = StructuredTrace::parse("{not-json").expect_err("parse should fail");
        assert!(error.to_string().contains("parsing structured trace"));
    }

    #[test]
    fn l3_t_replay_contract_sessions_003_final_result_ready_stays_protocol_only() -> Result<()> {
        let identity = TraceIdentity::new("job-contract-final", "session-contract-final");
        let caps = capability_report();
        let mut trace = StructuredTrace::new(ExecutionMode::Contract, ["created", "active"])
            .with_capability_report(&caps);
        trace.extend_from_adapter_events(
            &identity,
            &[
                ControlEvent::Started {
                    pid: 2,
                    pgid: 2,
                    sid: 2,
                    mode: ExecutionMode::Contract,
                },
                ControlEvent::FinalResultReady {
                    path: Path::new("/tmp/result.json").to_path_buf(),
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
            StructuredProtocolEventKind::FinalResultReady
        );
        Ok(())
    }

    #[test]
    fn l3_t_replay_contract_sessions_004_retention_metadata_round_trips_from_index() -> Result<()> {
        let sandbox = TempDir::new()?;
        let identity = ForensicIdentity::new("session-contract-retention")
            .with_job_id("job-contract-retention");
        let capture = capture_contract_output(
            sandbox.path(),
            identity,
            Utc::now(),
            b"stdout\n",
            b"stderr\n",
        )?;
        let summary = summarize_contract_capture(capture);
        let retained = contract_capture_retention_metadata(&summary.terminal_output_path)?;
        assert_eq!(retained.retention.base_retention_days, 14);
        assert_eq!(retained.artifact_refs.len(), 2);
        Ok(())
    }

    #[test]
    fn l3_t_replay_contract_sessions_005_trace_and_forensic_refs_remain_joinable() -> Result<()> {
        let sandbox = TempDir::new()?;
        let identity = ForensicIdentity::new("session-contract-join")
            .with_job_id("job-contract-join")
            .with_process_id("process-contract-join");
        let capture = summarize_contract_capture(capture_contract_output(
            sandbox.path(),
            identity,
            Utc::now(),
            b"contract-stdout\n",
            b"contract-stderr\n",
        )?);
        let trace_identity = TraceIdentity::new("job-contract-join", "session-contract-join")
            .with_process_id("process-contract-join");
        let caps = capability_report();
        let mut trace = StructuredTrace::new(ExecutionMode::Contract, ["created"])
            .with_capability_report(&caps);
        trace.insert_metadata(
            "forensic_artifact_refs",
            serde_json::to_value(&capture.artifact_refs)?,
        );
        let raw = trace.to_json_string()?;
        assert!(trace_contains_artifact_ref(
            &raw,
            Path::new(&capture.artifact_refs[0].path),
        )?);
        let _ = trace_identity;
        Ok(())
    }

    #[test]
    fn l3_t_replay_contract_sessions_006_contract_artifacts_are_compressed_and_indexed()
    -> Result<()> {
        let sandbox = TempDir::new()?;
        let identity = ForensicIdentity::new("session-contract-compressed");
        let capture = summarize_contract_capture(capture_contract_output(
            sandbox.path(),
            identity,
            Utc::now(),
            b"stdout-compressed\n",
            b"stderr-compressed\n",
        )?);
        let index_bytes = fs::read(&capture.terminal_output_path)?;
        assert!(!index_bytes.is_empty());
        assert_eq!(capture.artifact_refs.len(), 2);
        assert!(
            capture
                .artifact_refs
                .iter()
                .all(|artifact_ref| artifact_ref.path.ends_with(".zst"))
        );
        Ok(())
    }

    fn capability_report() -> CapabilityReport {
        CapabilityReport {
            cli_version: "test-1.0.0".to_owned(),
            auth_healthy: true,
            structured_output: true,
            pty_viable: false,
            browser_tools: false,
            is_canary: false,
        }
    }
}
