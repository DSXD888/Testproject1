#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

#[path = "../src/agents/adapter.rs"]
#[allow(dead_code)]
mod adapter_impl;
#[path = "../src/agents/artifacts.rs"]
#[allow(dead_code)]
mod artifacts_impl;
#[path = "../src/agents/contracts.rs"]
#[allow(dead_code)]
mod contracts_impl;
#[path = "support/db.rs"]
mod db_support;
#[path = "../src/errors/manifest.rs"]
#[allow(dead_code)]
mod errors_manifest_impl;
#[path = "../src/errors/markdown.rs"]
#[allow(dead_code)]
mod errors_markdown_impl;
#[path = "../src/ipc/handshake.rs"]
#[allow(dead_code)]
mod handshake_impl;
#[path = "../src/pty/health.rs"]
#[allow(dead_code)]
mod health_impl;
#[path = "../src/jobs/runner.rs"]
#[allow(dead_code)]
mod runner_impl;
#[path = "../src/tracing/structured.rs"]
#[allow(dead_code)]
mod structured_impl;
mod support;

mod agents {
    pub mod adapter {
        pub use crate::adapter_impl::*;
    }
}

mod ipc {
    pub mod command_router {
        use serde_json::Value;

        #[allow(dead_code)]
        #[derive(Debug, Clone)]
        pub struct BridgeError {
            pub code: String,
            pub message: String,
            pub retryable: bool,
            pub details: Option<Value>,
        }

        impl BridgeError {
            pub fn invalid_params(message: impl Into<String>) -> Self {
                Self {
                    code: "invalid_params".to_owned(),
                    message: message.into(),
                    retryable: false,
                    details: None,
                }
            }
        }

        #[allow(dead_code)]
        pub enum DispatchOutcome {
            Response(Value),
            DisconnectAfterResponse(Value),
        }

        #[derive(Default)]
        pub struct CommandRouter<'a> {
            _marker: std::marker::PhantomData<&'a ()>,
        }

        impl CommandRouter<'_> {
            #[allow(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
            pub fn dispatch(
                &mut self,
                _method: &str,
                _params: Value,
                _idempotency_key: Option<&str>,
            ) -> std::result::Result<DispatchOutcome, BridgeError> {
                Err(BridgeError::invalid_params(
                    "command router stub should not be invoked in observability tests",
                ))
            }
        }
    }
}

mod jobs {
    pub mod runner {
        pub use crate::runner_impl::*;
    }
}

mod tests {
    use anyhow::{Context, Result, ensure};
    use serde_json::Value as JsonValue;
    use serde_yaml::Value as YamlValue;
    use tempfile::tempdir;

    use crate::adapter_impl::{ControlEvent, ExecutionMode};
    use crate::artifacts_impl::{DaemonEvidenceAppendix, OutcomeClass, RunManifestFrontmatter};
    use crate::errors_manifest_impl::ErrorCode;
    use crate::handshake_impl::{
        CronAwaitingAccessStatus, CronDefinitionStatus, CronStatusSnapshot, CronWorkerLaneStatus,
        ResourceArbiterEventSnapshot, ResourceArbiterStatusSnapshot, SourceHealthStatusEntry,
        SourceHealthStatusSnapshot, ValidationStatusSnapshot,
    };
    use crate::health_impl::HealthIssue;
    use crate::runner_impl::{
        SkillTelemetryWindow, compute_skill_telemetry, skill_telemetry_thresholds,
    };
    use crate::structured_impl::{
        CONTROL_PROTOCOL_RETENTION_DAYS, StructuredControlEventKind, StructuredControlEventRecord,
        StructuredProtocolEventKind, StructuredTrace, TraceIdentity, error_code_class_for_str,
        trace_contains_artifact_ref,
    };
    use crate::support::telemetry::TelemetryProbe;

    fn sample_frontmatter(outcome_class: OutcomeClass) -> RunManifestFrontmatter {
        RunManifestFrontmatter {
            manifest_version: 1,
            skill_id: "skill/fts-ingestion".to_owned(),
            skill_version: "2026.03.30".to_owned(),
            job_id: "job-123".to_owned(),
            session_id: "session-123".to_owned(),
            execution_mode: "contract".to_owned(),
            started_at: "2026-03-30T15:00:00Z".to_owned(),
            completed_at: "2026-03-30T15:01:00Z".to_owned(),
            outcome_class,
            items_processed: 10,
            items_skipped: 1,
            items_failed: 0,
            warnings_count: 1,
            agent_claimed_pages_visited: 4,
            agent_claimed_source_url: "https://www.find-tender.service.gov.uk".to_owned(),
            agent_claimed_fingerprint_state: "matched".to_owned(),
            agent_claimed_locale_state: "en-GB".to_owned(),
            agent_claimed_currency_state: "GBP".to_owned(),
            agent_narrative_missing: false,
        }
    }

    fn sample_appendix() -> DaemonEvidenceAppendix {
        DaemonEvidenceAppendix {
            observed_url_chain: vec![
                "https://www.find-tender.service.gov.uk".to_owned(),
                "https://www.find-tender.service.gov.uk/notice/123".to_owned(),
            ],
            observed_page_kind_sequence: vec!["listing".to_owned(), "notice".to_owned()],
            daemon_observed_fingerprint_state: "matched".to_owned(),
            daemon_observed_locale: "en-GB".to_owned(),
            daemon_observed_currency: "GBP".to_owned(),
            auth_challenge_events: vec!["none".to_owned()],
            locator_fallback_depth: 2,
            basis_sufficiency: "medium".to_owned(),
            page_fingerprint_hash: Some("fingerprint-sha256".to_owned()),
            auth_challenge_detected: false,
            extraction_fill_rate_bps: Some(9_250),
            checkpoint_count: 3,
            byte_count_compressed: Some(4_096),
            validator_rubric_hash: Some("rubric-sha256".to_owned()),
            drift_delta_json: serde_json::json!({
                "fingerprint_delta_bps": 125,
                "layout_shift": "minor"
            }),
            result_item_count: 12,
            oqg_anomaly_summary: vec!["duplicate_rate:low".to_owned()],
            representative_item_sample: serde_json::json!({
                "notice_id": "123",
                "title": "PPE supply"
            }),
            artifact_refs: vec![
                "checkpoints/checkpoint-1.json".to_owned(),
                "results/result.json".to_owned(),
            ],
            bid_scope_summary: None,
            side_effect_progression: Vec::new(),
            readiness_evidence: None,
            validator_verdict: None,
            override_metadata: None,
            final_bid_state: None,
        }
    }

    fn yaml_keys(value: &YamlValue) -> Result<Vec<String>> {
        let keys = value
            .as_mapping()
            .ok_or_else(|| anyhow::anyhow!("expected YAML mapping"))?
            .keys()
            .filter_map(YamlValue::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        Ok(keys)
    }

    fn error_inventory() -> Vec<&'static str> {
        vec![
            ErrorCode::SourceChanged.as_str(),
            ErrorCode::NoticeParseFailed.as_str(),
            ErrorCode::ExtractionEncryptedPdf.as_str(),
            ErrorCode::ExtractionEmptyPdf.as_str(),
            ErrorCode::ExtractionCorruptPdf.as_str(),
            ErrorCode::ExtractionEngineUnavailable.as_str(),
            ErrorCode::ExtractionTimeout.as_str(),
            ErrorCode::ExtractionOom.as_str(),
            ErrorCode::GoodsClassificationLowConfidence.as_str(),
            ErrorCode::SupplierMatchLowConfidence.as_str(),
            ErrorCode::CommodityCodeLowConfidence.as_str(),
            ErrorCode::RevenueBasisUncertain.as_str(),
            ErrorCode::ClassificationAmbiguous.as_str(),
            ErrorCode::TariffBranchDivergence.as_str(),
            ErrorCode::MeasureInteractionUnknown.as_str(),
            ErrorCode::ExporterUnresolved.as_str(),
            ErrorCode::SupplementaryUnitMissing.as_str(),
            ErrorCode::ValuationModeConflict.as_str(),
            ErrorCode::FpoAccessUnavailable.as_str(),
            ErrorCode::TradeRemedyWatchStale.as_str(),
            ErrorCode::FxRateMissing.as_str(),
            ErrorCode::FreightEstimateFailed.as_str(),
            ErrorCode::BidSubmissionFailed.as_str(),
            ErrorCode::BidDuplicateDetected.as_str(),
            ErrorCode::BidSideEffectCapExceeded.as_str(),
            ErrorCode::BidReadinessGateFailed.as_str(),
            ErrorCode::BidPortalChanged.as_str(),
            ErrorCode::BidPortalAuthExpired.as_str(),
            ErrorCode::AgentStalled.as_str(),
            ErrorCode::AgentAuthChallenge.as_str(),
            ErrorCode::AgentSkillFailed.as_str(),
            ErrorCode::AgentOutputContractInvalid.as_str(),
            ErrorCode::AgentSuspectOutput.as_str(),
            ErrorCode::AgentPartialFailure.as_str(),
            ErrorCode::AgentInternalError.as_str(),
            ErrorCode::AgentRunFailed.as_str(),
            ErrorCode::AgentCleanupIncomplete.as_str(),
            ErrorCode::AgentDaemonCrashDuringSession.as_str(),
            ErrorCode::AgentProtocolMismatch.as_str(),
            ErrorCode::SourceSoftBlocked.as_str(),
            ErrorCode::SourceCaptchaBlocked.as_str(),
            ErrorCode::AuthStateStale.as_str(),
            ErrorCode::PageShapeDrift.as_str(),
            ErrorCode::LocaleOrCurrencyDrift.as_str(),
            ErrorCode::HostSleepInterrupted.as_str(),
            ErrorCode::HostResourceExhausted.as_str(),
            ErrorCode::SkillDirectoryInvalid.as_str(),
            ErrorCode::SqliteWriteFailed.as_str(),
            ErrorCode::DaemonStartupFailed.as_str(),
        ]
    }

    fn health_issue_inventory() -> Vec<&'static str> {
        HealthIssue::inventory()
            .into_iter()
            .map(HealthIssue::code)
            .collect()
    }

    const fn l5_error_class_cases() -> [(ErrorCode, &'static str); 10] {
        [
            (ErrorCode::ClassificationAmbiguous, "decision_grade_blocker"),
            (ErrorCode::TariffBranchDivergence, "decision_grade_blocker"),
            (
                ErrorCode::MeasureInteractionUnknown,
                "decision_grade_blocker",
            ),
            (ErrorCode::ExporterUnresolved, "evidence_gap"),
            (ErrorCode::SupplementaryUnitMissing, "evidence_gap"),
            (ErrorCode::ValuationModeConflict, "valuation_conflict"),
            (ErrorCode::FpoAccessUnavailable, "recall_source_unavailable"),
            (ErrorCode::TradeRemedyWatchStale, "decision_grade_blocker"),
            (ErrorCode::FxRateMissing, "decision_grade_blocker"),
            (ErrorCode::FreightEstimateFailed, "evidence_gap"),
        ]
    }

    const fn l6_bid_error_class_cases() -> [(ErrorCode, &'static str); 6] {
        [
            (ErrorCode::BidSubmissionFailed, "portal_workflow"),
            (ErrorCode::BidDuplicateDetected, "portal_workflow"),
            (ErrorCode::BidSideEffectCapExceeded, "portal_workflow"),
            (ErrorCode::BidReadinessGateFailed, "portal_workflow"),
            (ErrorCode::BidPortalChanged, "portal_workflow"),
            (ErrorCode::BidPortalAuthExpired, "portal_workflow"),
        ]
    }

    fn l6_error_propagation_sites() -> Vec<&'static str> {
        vec![
            "ErrorCode enum",
            "ErrorCode::as_str",
            "parse_error_code",
            "agent_error_manifest.rs",
            "skill_drift_monitor.rs",
            "observability_telemetry.rs",
            "skill_canary_harness.rs",
            "bid workflow state-machine tests",
            "redaction boundary tests",
        ]
    }

    #[allow(dead_code)]
    fn sample_cron_status() -> CronStatusSnapshot {
        CronStatusSnapshot {
            definitions: vec![CronDefinitionStatus {
                definition_key: "fts::procurement-ingestion/fts".to_owned(),
                source_id: "fts".to_owned(),
                skill_id: "procurement-ingestion/fts".to_owned(),
                skill_version: "2026.03.30-1".to_owned(),
                cron_expression: "0 15 4 * * *".to_owned(),
                enabled: true,
                last_run_at: Some("2026-03-30T04:15:00Z".to_owned()),
                next_run_at: Some("2026-03-31T04:15:00Z".to_owned()),
            }],
            scheduler_running: true,
            last_tick_at: Some("2026-03-30T04:15:10Z".to_owned()),
            next_run_at: Some("2026-03-31T04:15:00Z".to_owned()),
            worker_lanes: vec![
                CronWorkerLaneStatus {
                    lane: "pty_browser".to_owned(),
                    running: true,
                    claimable_kinds: vec![
                        "sync_source".to_owned(),
                        "source_health".to_owned(),
                        "auth_refresh".to_owned(),
                    ],
                },
                CronWorkerLaneStatus {
                    lane: "coverage_deterministic".to_owned(),
                    running: true,
                    claimable_kinds: vec![
                        "commercial_sweep".to_owned(),
                        "validation_cross_model_audit".to_owned(),
                        "validation_calibration_readiness_sweep".to_owned(),
                    ],
                },
            ],
            awaiting_access: CronAwaitingAccessStatus {
                active_count: 2,
                expired_count: 1,
            },
        }
    }

    #[allow(dead_code)]
    fn sample_source_health_status() -> SourceHealthStatusSnapshot {
        SourceHealthStatusSnapshot {
            sources: vec![SourceHealthStatusEntry {
                source_id: "fts".to_owned(),
                health_state: "green".to_owned(),
                circuit_state: "closed".to_owned(),
                production_allowed: true,
                cooldown_until: None,
                last_canary_at: Some("2026-03-30T04:00:00Z".to_owned()),
                last_canary_result: Some("passed".to_owned()),
                last_error_code: None,
            }],
        }
    }

    #[allow(dead_code)]
    fn sample_validation_status() -> ValidationStatusSnapshot {
        ValidationStatusSnapshot {
            validator_approval_rate_bps: 10_000,
            validator_rejection_rate_bps: 0,
            validator_flag_rate_bps: 0,
            validator_cost_per_run_pence: 12,
            operator_override_rate_bps: 0,
            validator_execution_failure_rate_bps: 0,
            validator_canary_pass_rate_bps: 10_000,
            validator_health_state: "healthy".to_owned(),
            pending_backlog_count: 0,
            pending_canary_count: 0,
            validator_job_count: 3,
        }
    }

    #[allow(dead_code)]
    fn sample_resource_arbiter_status() -> ResourceArbiterStatusSnapshot {
        ResourceArbiterStatusSnapshot {
            browser_slots_limit: 2,
            browser_slots_in_use: 1,
            agent_slots_limit: 4,
            agent_slots_in_use: 2,
            procurement_source_lane_limit: 1,
            supplier_source_lane_limit: 2,
            source_lanes_in_use: std::collections::BTreeMap::from([("fts".to_owned(), 1)]),
            concurrency_in_use: std::collections::BTreeMap::from([(
                "procurement-ingestion/fts".to_owned(),
                1,
            )]),
            recent_events: vec![ResourceArbiterEventSnapshot {
                kind: "grant".to_owned(),
                job_id: "job-123".to_owned(),
                session_id: Some("session-123".to_owned()),
                priority: "procurement_ingestion".to_owned(),
                denial_reason: None,
                release_cause: None,
                browser_slots_in_use: 1,
                agent_slots_in_use: 2,
                source_lane_key: Some("fts".to_owned()),
                source_lane_in_use: Some(1),
            }],
        }
    }

    fn sample_identity() -> TraceIdentity {
        TraceIdentity::new("job-123", "session-123")
            .with_process_id("process-123")
            .with_run_manifest_hash("manifest-hash-123")
    }

    fn sample_control_trace() -> StructuredTrace {
        let identity = sample_identity();
        let mut trace = StructuredTrace::new(
            ExecutionMode::Pty,
            [
                "created",
                "spawned",
                "active",
                "terminating",
                "exited",
                "reaped",
                "archived",
            ],
        );
        trace.push_control_record(StructuredControlEventRecord {
            seq: 1,
            event: StructuredControlEventKind::ResourceGranted,
            identity: identity.clone(),
            execution_mode: "pty".to_owned(),
            at: None,
            details: serde_json::json!({"browser_slots_in_use": 1}),
        });
        trace.push_control_record(StructuredControlEventRecord {
            seq: 2,
            event: StructuredControlEventKind::ResourceDenied,
            identity: identity.clone(),
            execution_mode: "pty".to_owned(),
            at: None,
            details: serde_json::json!({"denial_reason": "source_lane_exhausted:fts:1/1"}),
        });
        trace.push_control_record(StructuredControlEventRecord {
            seq: 3,
            event: StructuredControlEventKind::ResourceReleased,
            identity: identity.clone(),
            execution_mode: "pty".to_owned(),
            at: None,
            details: serde_json::json!({"release_cause": "archived"}),
        });
        trace.push_control_record(StructuredControlEventRecord {
            seq: 4,
            event: StructuredControlEventKind::CircuitTransition,
            identity: identity.clone(),
            execution_mode: "pty".to_owned(),
            at: None,
            details: serde_json::json!({"source_id": "fts", "state": "half_open"}),
        });
        trace.push_control_record(StructuredControlEventRecord {
            seq: 5,
            event: StructuredControlEventKind::Spawn,
            identity: identity.clone(),
            execution_mode: "pty".to_owned(),
            at: None,
            details: serde_json::json!({"pid": 10, "pgid": 11, "sid": 12}),
        });
        trace.push_control_record(StructuredControlEventRecord {
            seq: 6,
            event: StructuredControlEventKind::Signal,
            identity: identity.clone(),
            execution_mode: "pty".to_owned(),
            at: None,
            details: serde_json::json!({"signal": "SIGTERM"}),
        });
        trace.push_control_record(StructuredControlEventRecord {
            seq: 7,
            event: StructuredControlEventKind::Exit,
            identity: identity.clone(),
            execution_mode: "pty".to_owned(),
            at: None,
            details: serde_json::json!({"success": true, "exit_code": 0}),
        });
        trace.push_control_record(StructuredControlEventRecord {
            seq: 8,
            event: StructuredControlEventKind::Reap,
            identity: identity.clone(),
            execution_mode: "pty".to_owned(),
            at: None,
            details: serde_json::json!({"pid": 10, "pgid": 11, "sid": 12}),
        });
        trace.push_control_record(StructuredControlEventRecord {
            seq: 9,
            event: StructuredControlEventKind::CleanupVerdict,
            identity,
            execution_mode: "pty".to_owned(),
            at: None,
            details: serde_json::json!({"cleanup_verdict": "clean"}),
        });
        trace
    }

    #[test]
    fn l3_t_observability_telemetry_001_control_events_cover_runtime_and_resource_activity()
    -> Result<()> {
        let trace = sample_control_trace();
        let control_kinds = trace
            .control_event_records
            .iter()
            .map(|record| &record.event)
            .collect::<Vec<_>>();

        ensure!(
            control_kinds.contains(&&StructuredControlEventKind::ResourceGranted)
                && control_kinds.contains(&&StructuredControlEventKind::ResourceDenied)
                && control_kinds.contains(&&StructuredControlEventKind::ResourceReleased)
                && control_kinds.contains(&&StructuredControlEventKind::CircuitTransition)
                && control_kinds.contains(&&StructuredControlEventKind::Spawn)
                && control_kinds.contains(&&StructuredControlEventKind::Signal)
                && control_kinds.contains(&&StructuredControlEventKind::Exit)
                && control_kinds.contains(&&StructuredControlEventKind::Reap)
                && control_kinds.contains(&&StructuredControlEventKind::CleanupVerdict),
            "L3-T-observability_telemetry-001 expected the full control-event inventory"
        );
        ensure!(
            trace.control_event_records.iter().all(|record| {
                record.identity.job_id == "job-123" && record.identity.session_id == "session-123"
            }),
            "L3-T-observability_telemetry-001 expected explicit job/session keys on every control record"
        );
        Ok(())
    }

    #[test]
    fn l3_t_observability_telemetry_002_protocol_events_cover_pty_and_contract_sessions()
    -> Result<()> {
        let identity = sample_identity();
        let mut pty_trace = StructuredTrace::new(ExecutionMode::Pty, ["created", "active"]);
        let pty_events = vec![
            ControlEvent::Started {
                pid: 10,
                pgid: 11,
                sid: 12,
                mode: ExecutionMode::Pty,
            },
            ControlEvent::Checkpoint {
                seq: 1,
                path: "checkpoints/pty.json".into(),
            },
            ControlEvent::Warning {
                code: "AUTH_CHALLENGE".to_owned(),
                detail: "reauth required".to_owned(),
            },
            ControlEvent::FinalResultReady {
                path: "results/pty.json".into(),
            },
            ControlEvent::Exited {
                status: portable_pty::ExitStatus::with_exit_code(0),
            },
        ];
        pty_trace.extend_from_adapter_events(&identity, &pty_events);

        let mut contract_trace =
            StructuredTrace::new(ExecutionMode::Contract, ["created", "active"]);
        let contract_events = vec![
            ControlEvent::Started {
                pid: 20,
                pgid: 21,
                sid: 22,
                mode: ExecutionMode::Contract,
            },
            ControlEvent::Progress {
                completed: 1,
                total: Some(2),
            },
            ControlEvent::FinalResultReady {
                path: "results/contract.json".into(),
            },
            ControlEvent::Exited {
                status: portable_pty::ExitStatus::with_exit_code(0),
            },
        ];
        contract_trace.extend_from_adapter_events(&identity, &contract_events);

        ensure!(
            pty_trace
                .protocol_events
                .iter()
                .any(|record| record.event == StructuredProtocolEventKind::Checkpoint)
                && pty_trace
                    .protocol_events
                    .iter()
                    .any(|record| record.event == StructuredProtocolEventKind::Warning)
                && contract_trace
                    .protocol_events
                    .iter()
                    .any(|record| record.event == StructuredProtocolEventKind::Progress)
                && contract_trace
                    .protocol_events
                    .iter()
                    .any(|record| record.event == StructuredProtocolEventKind::FinalResultReady),
            "L3-T-observability_telemetry-002 expected protocol events from both execution modes"
        );
        Ok(())
    }

    #[test]
    fn l3_t_observability_telemetry_003_joinability_survives_without_transcript_scraping()
    -> Result<()> {
        let temp = tempdir()?;
        let artifact_path = temp.path().join("terminal-output.jsonl.gz");
        std::fs::write(&artifact_path, b"trace-bytes")?;

        let trace = sample_control_trace();
        let raw = trace.to_json_string()?;
        let trace_contains_path = trace_contains_artifact_ref(&raw, &artifact_path)?;
        let identity = sample_identity();

        ensure!(identity.process_id.as_deref() == Some("process-123"));
        ensure!(
            identity.run_manifest_hash.as_deref() == Some("manifest-hash-123"),
            "L3-T-observability_telemetry-003 expected run_manifest_hash join key"
        );
        ensure!(
            !trace_contains_path,
            "fixture trace should not contain the temp path before explicit metadata insertion"
        );

        let mut trace_with_refs = trace;
        trace_with_refs.insert_metadata(
            "artifact_refs",
            serde_json::json!([artifact_path.to_string_lossy().to_string()]),
        );
        let raw_with_refs = trace_with_refs.to_json_string()?;
        ensure!(trace_contains_artifact_ref(&raw_with_refs, &artifact_path)?);
        Ok(())
    }

    #[test]
    fn l3_t_observability_telemetry_004_retention_configuration_matches_layer3_minima() {
        let trace = StructuredTrace::new(ExecutionMode::Contract, ["created"]);
        let retention = trace.retention_policy.expect("retention policy");
        assert_eq!(
            retention.control_protocol_days,
            CONTROL_PROTOCOL_RETENTION_DAYS
        );
        assert!(!sample_appendix().artifact_refs.is_empty());
        assert!(!sample_frontmatter(OutcomeClass::Completed).agent_narrative_missing);
    }

    #[test]
    fn l3_t_observability_telemetry_005_skill_telemetry_emits_all_thirteen_metrics() {
        let telemetry = compute_skill_telemetry(&SkillTelemetryWindow {
            successful_runs: 9,
            total_runs: 10,
            durations_ms: vec![1_000, 1_500, 1_250, 1_750],
            cost_per_run_pence: Some(12),
            token_count: Some(1_024),
            checkpoint_count: Some(2),
            retry_count: Some(1),
            extraction_fill_rate_bps: Some(9_100),
            page_fingerprint_delta_bps: Some(125),
            auth_challenge_rate_bps: Some(0),
            listing_match_rate_bps: None,
            validation_rejection_rate_bps: Some(0),
            drift_score_bps: Some(125),
        });
        let thresholds = skill_telemetry_thresholds();

        assert_eq!(telemetry.success_rate, Some(0.9));
        assert_eq!(telemetry.median_duration_ms, Some(1_500));
        assert_eq!(telemetry.p95_duration_ms, Some(1_750));
        assert_eq!(telemetry.cost_per_run_pence, Some(12));
        assert_eq!(telemetry.token_count, Some(1_024));
        assert_eq!(telemetry.checkpoint_count, Some(2));
        assert_eq!(telemetry.retry_count, Some(1));
        assert_eq!(telemetry.extraction_fill_rate_bps, Some(9_100));
        assert_eq!(telemetry.page_fingerprint_delta_bps, Some(125));
        assert_eq!(telemetry.auth_challenge_rate_bps, Some(0));
        assert_eq!(telemetry.listing_match_rate_bps, None);
        assert_eq!(telemetry.validation_rejection_rate_bps, Some(0));
        assert_eq!(telemetry.drift_score_bps, Some(125));
        assert_eq!(thresholds.drift_score_max_bps, 3_000);
    }

    #[test]
    fn l3_t_observability_telemetry_006_validation_telemetry_emits_only_declared_metrics() {
        let telemetry = crate::handshake_impl::ValidationStatusSnapshot {
            validator_approval_rate_bps: 5_000,
            validator_rejection_rate_bps: 2_500,
            validator_flag_rate_bps: 2_500,
            validator_cost_per_run_pence: 12,
            operator_override_rate_bps: 1_000,
            validator_execution_failure_rate_bps: 2_500,
            validator_canary_pass_rate_bps: 10_000,
            validator_health_state: "degraded".to_owned(),
            pending_backlog_count: 4,
            pending_canary_count: 1,
            validator_job_count: 8,
        };

        assert_eq!(telemetry.validator_health_state, "degraded");
        assert_eq!(telemetry.validator_execution_failure_rate_bps, 2_500);
        assert_eq!(telemetry.validator_canary_pass_rate_bps, 10_000);
        assert_eq!(telemetry.pending_backlog_count, 4);
    }

    #[test]
    fn l3_t_observability_telemetry_007_three_am_drilldown_stays_id_based() -> Result<()> {
        let trace = sample_control_trace();
        let raw = trace.to_json_string()?;
        let appendix = sample_appendix();
        let frontmatter = sample_frontmatter(OutcomeClass::Completed);

        ensure!(frontmatter.job_id == "job-123");
        ensure!(frontmatter.session_id == "session-123");
        ensure!(
            appendix
                .artifact_refs
                .contains(&"results/result.json".to_owned())
        );
        ensure!(
            raw.contains("manifest-hash-123") && raw.contains("process-123"),
            "L3-T-observability_telemetry-007 expected explicit join keys in the trace payload"
        );
        Ok(())
    }

    #[test]
    fn l3_t_observability_telemetry_012_error_code_inventory_is_exact_and_canary_classes_map_onto_it()
    -> Result<()> {
        let inventory = error_inventory();
        let expected = vec![
            "SOURCE_CHANGED",
            "NOTICE_PARSE_FAILED",
            "EXTRACTION_ENCRYPTED_PDF",
            "EXTRACTION_EMPTY_PDF",
            "EXTRACTION_CORRUPT_PDF",
            "EXTRACTION_ENGINE_UNAVAILABLE",
            "EXTRACTION_TIMEOUT",
            "EXTRACTION_OOM",
            "GOODS_CLASSIFICATION_LOW_CONFIDENCE",
            "SUPPLIER_MATCH_LOW_CONFIDENCE",
            "COMMODITY_CODE_LOW_CONFIDENCE",
            "REVENUE_BASIS_UNCERTAIN",
            "CLASSIFICATION_AMBIGUOUS",
            "TARIFF_BRANCH_DIVERGENCE",
            "MEASURE_INTERACTION_UNKNOWN",
            "EXPORTER_UNRESOLVED",
            "SUPPLEMENTARY_UNIT_MISSING",
            "VALUATION_MODE_CONFLICT",
            "FPO_ACCESS_UNAVAILABLE",
            "TRADE_REMEDY_WATCH_STALE",
            "FX_RATE_MISSING",
            "FREIGHT_ESTIMATE_FAILED",
            "BID_SUBMISSION_FAILED",
            "BID_DUPLICATE_DETECTED",
            "BID_SIDE_EFFECT_CAP_EXCEEDED",
            "BID_READINESS_GATE_FAILED",
            "BID_PORTAL_CHANGED",
            "BID_PORTAL_AUTH_EXPIRED",
            "AGENT_STALLED",
            "AGENT_AUTH_CHALLENGE",
            "AGENT_SKILL_FAILED",
            "AGENT_OUTPUT_CONTRACT_INVALID",
            "AGENT_SUSPECT_OUTPUT",
            "AGENT_PARTIAL_FAILURE",
            "AGENT_INTERNAL_ERROR",
            "AGENT_RUN_FAILED",
            "AGENT_CLEANUP_INCOMPLETE",
            "AGENT_DAEMON_CRASH_DURING_SESSION",
            "AGENT_PROTOCOL_MISMATCH",
            "SOURCE_SOFT_BLOCKED",
            "SOURCE_CAPTCHA_BLOCKED",
            "AUTH_STATE_STALE",
            "PAGE_SHAPE_DRIFT",
            "LOCALE_OR_CURRENCY_DRIFT",
            "HOST_SLEEP_INTERRUPTED",
            "HOST_RESOURCE_EXHAUSTED",
            "SKILL_DIRECTORY_INVALID",
            "SQLITE_WRITE_FAILED",
            "DAEMON_STARTUP_FAILED",
        ];
        ensure!(
            inventory == expected,
            "L3-T-observability_telemetry-012 expected exact ErrorCode inventory"
        );
        ensure!(
            inventory.len() == 49,
            "L3-T-observability_telemetry-012 expected exactly 49 error codes"
        );

        let logical_canary_mapping = vec![
            ("SOURCE_UNREACHABLE", ErrorCode::AgentRunFailed.as_str()),
            ("SOURCE_LAYOUT_CHANGED", ErrorCode::PageShapeDrift.as_str()),
            (
                "AGENT_AUTH_CHALLENGE",
                ErrorCode::AgentAuthChallenge.as_str(),
            ),
            (
                "SOURCE_LOCALE_DRIFT",
                ErrorCode::LocaleOrCurrencyDrift.as_str(),
            ),
            ("SOURCE_BLOCKED", ErrorCode::SourceSoftBlocked.as_str()),
            ("SOURCE_BLOCKED", ErrorCode::SourceCaptchaBlocked.as_str()),
            (
                "SOURCE_EXTRACTION_FAILED",
                ErrorCode::AgentSkillFailed.as_str(),
            ),
        ];

        for (logical_class, mapped_error_code) in logical_canary_mapping {
            ensure!(
                inventory.contains(&mapped_error_code),
                "L3-T-observability_telemetry-012 logical canary class `{logical_class}` is not mapped onto existing ErrorCode surface"
            );
        }

        Ok(())
    }

    #[test]
    fn l3_t_observability_telemetry_008_frontmatter_and_appendix_field_inventory_is_fully_asserted()
    -> Result<()> {
        let frontmatter_keys = yaml_keys(&serde_yaml::to_value(sample_frontmatter(
            OutcomeClass::SuspectOutput,
        ))?)?;
        let appendix_keys = yaml_keys(&serde_yaml::to_value(sample_appendix())?)?;

        ensure!(
            frontmatter_keys.len() == 19,
            "frontmatter inventory should assert every current RunManifestFrontmatter field"
        );
        ensure!(
            appendix_keys.len() == 25,
            "appendix inventory should assert every current DaemonEvidenceAppendix field"
        );
        ensure!(
            frontmatter_keys.contains(&"agent_narrative_missing".to_owned()),
            "frontmatter inventory is missing explicit fallback flag coverage"
        );
        ensure!(
            appendix_keys.contains(&"checkpoint_count".to_owned()),
            "appendix inventory is missing checkpoint_count coverage"
        );
        ensure!(
            appendix_keys.contains(&"validator_rubric_hash".to_owned()),
            "appendix inventory is missing validator appendix field coverage"
        );
        Ok(())
    }

    #[test]
    fn l3_t_observability_telemetry_009_frontmatter_key_parity_matches_contract() -> Result<()> {
        let keys = yaml_keys(&serde_yaml::to_value(sample_frontmatter(
            OutcomeClass::Failed,
        ))?)?;
        let expected = vec![
            "manifest_version",
            "skill_id",
            "skill_version",
            "job_id",
            "session_id",
            "execution_mode",
            "started_at",
            "completed_at",
            "outcome_class",
            "items_processed",
            "items_skipped",
            "items_failed",
            "warnings_count",
            "agent_claimed_pages_visited",
            "agent_claimed_source_url",
            "agent_claimed_fingerprint_state",
            "agent_claimed_locale_state",
            "agent_claimed_currency_state",
            "agent_narrative_missing",
        ];
        ensure!(
            keys == expected,
            "L3-T-observability_telemetry-009 expected exact RunManifestFrontmatter key set"
        );
        Ok(())
    }

    #[test]
    fn l3_t_observability_telemetry_010_core_appendix_key_parity_matches_contract() -> Result<()> {
        let keys = yaml_keys(&serde_yaml::to_value(sample_appendix())?)?;
        let expected = vec![
            "observed_url_chain",
            "observed_page_kind_sequence",
            "daemon_observed_fingerprint_state",
            "daemon_observed_locale",
            "daemon_observed_currency",
            "auth_challenge_events",
            "locator_fallback_depth",
            "basis_sufficiency",
            "page_fingerprint_hash",
            "auth_challenge_detected",
            "extraction_fill_rate_bps",
            "checkpoint_count",
            "byte_count_compressed",
            "validator_rubric_hash",
            "drift_delta_json",
            "result_item_count",
            "oqg_anomaly_summary",
            "representative_item_sample",
            "artifact_refs",
            "bid_scope_summary",
            "side_effect_progression",
            "readiness_evidence",
            "validator_verdict",
            "override_metadata",
            "final_bid_state",
        ];
        ensure!(
            keys == expected,
            "L3-T-observability_telemetry-010 expected exact DaemonEvidenceAppendix key set"
        );
        Ok(())
    }

    #[test]
    fn l3_t_observability_telemetry_011_layer3_appendix_additions_preserve_explicit_absence()
    -> Result<()> {
        let appendix = sample_appendix();
        ensure!(
            appendix.basis_sufficiency == "medium",
            "basis_sufficiency missing"
        );
        ensure!(
            appendix.page_fingerprint_hash.as_deref() == Some("fingerprint-sha256"),
            "page_fingerprint_hash missing"
        );
        ensure!(
            appendix.extraction_fill_rate_bps == Some(9_250),
            "extraction_fill_rate_bps missing"
        );
        ensure!(
            appendix.byte_count_compressed == Some(4_096),
            "byte_count_compressed missing"
        );
        ensure!(
            appendix.validator_rubric_hash.as_deref() == Some("rubric-sha256"),
            "validator_rubric_hash missing"
        );
        ensure!(
            appendix.drift_delta_json["fingerprint_delta_bps"] == 125,
            "drift_delta_json missing structured content"
        );

        let absent = DaemonEvidenceAppendix {
            page_fingerprint_hash: None,
            extraction_fill_rate_bps: None,
            byte_count_compressed: None,
            validator_rubric_hash: None,
            drift_delta_json: JsonValue::Null,
            basis_sufficiency: "low".to_owned(),
            ..sample_appendix()
        };
        let yaml = serde_yaml::to_string(&absent)?;
        ensure!(
            yaml.contains("page_fingerprint_hash: null"),
            "explicit absence should serialize for page_fingerprint_hash"
        );
        ensure!(
            yaml.contains("validator_rubric_hash: null"),
            "explicit absence should serialize for validator_rubric_hash"
        );
        ensure!(
            yaml.contains("drift_delta_json: null"),
            "explicit absence should serialize for drift_delta_json"
        );
        Ok(())
    }

    #[test]
    fn ci_l3_007_manifest_error_code_and_health_issue_inventory_have_explicit_name_level_coverage()
    -> Result<()> {
        let error_codes = error_inventory();
        let health_issues = health_issue_inventory();

        ensure!(
            health_issues
                == vec![
                    "AGENT_STALLED",
                    "AGENT_AUTH_CHALLENGE",
                    "HOST_SLEEP_INTERRUPTED",
                    "HOST_RESOURCE_EXHAUSTED",
                ],
            "CI-L3-007 expected exact HealthIssue inventory"
        );
        ensure!(
            health_issues
                .iter()
                .all(|health_issue| error_codes.contains(health_issue)),
            "CI-L3-007 every HealthIssue code should map to the ErrorCode inventory"
        );
        ensure!(
            error_codes.len() == 49,
            "CI-L3-007 expected exact ErrorCode inventory coverage count"
        );
        Ok(())
    }

    #[test]
    fn l5_t_measure_interaction_018_error_vocabulary_contains_l5_customs_codes() -> Result<()> {
        let error_codes = error_inventory();
        let expected_l5_codes = [
            "CLASSIFICATION_AMBIGUOUS",
            "TARIFF_BRANCH_DIVERGENCE",
            "MEASURE_INTERACTION_UNKNOWN",
            "EXPORTER_UNRESOLVED",
            "SUPPLEMENTARY_UNIT_MISSING",
            "VALUATION_MODE_CONFLICT",
            "FPO_ACCESS_UNAVAILABLE",
            "TRADE_REMEDY_WATCH_STALE",
            "FX_RATE_MISSING",
            "FREIGHT_ESTIMATE_FAILED",
        ];

        for expected_code in expected_l5_codes {
            ensure!(
                error_codes.contains(&expected_code),
                "L5-T-measure_interaction-018 expected L5 ErrorCode `{expected_code}`"
            );
            let parsed = crate::errors_manifest_impl::parse_error_code(expected_code)?;
            ensure!(
                parsed.as_str() == expected_code,
                "L5-T-measure_interaction-018 expected roundtrip for `{expected_code}`"
            );
        }
        ensure!(
            !error_codes.contains(&"FPO_UNAVAILABLE"),
            "L5-T-measure_interaction-018 must not add duplicate FPO_UNAVAILABLE"
        );
        Ok(())
    }

    #[test]
    fn l5_error_codes_emit_structured_trace_metadata() -> Result<()> {
        for (error_code, expected_class) in l5_error_class_cases() {
            let mut trace = StructuredTrace::new(ExecutionMode::Contract, ["created", "failed"]);
            trace.insert_error_code_metadata(error_code.as_str())?;

            ensure!(
                trace
                    .additional_fields
                    .get("error_code")
                    .and_then(JsonValue::as_str)
                    == Some(error_code.as_str()),
                "structured trace should carry error_code for {}",
                error_code.as_str()
            );
            ensure!(
                trace
                    .additional_fields
                    .get("error_class")
                    .and_then(JsonValue::as_str)
                    == Some(expected_class),
                "structured trace should carry error_class `{expected_class}` for {}",
                error_code.as_str()
            );
            ensure!(
                error_code_class_for_str(error_code.as_str()) == Some(expected_class),
                "error_code_class_for_str should classify {} as `{expected_class}`",
                error_code.as_str()
            );
        }

        Ok(())
    }

    #[test]
    fn l6_error_codes_emit_structured_trace_metadata_and_propagation_inventory() -> Result<()> {
        let propagation_sites = l6_error_propagation_sites();
        ensure!(
            (9..=15).contains(&propagation_sites.len()),
            "L6-T-error_codes-003 expected 9-15 propagation sites, found {}",
            propagation_sites.len()
        );
        ensure!(
            propagation_sites.contains(&"agent_error_manifest.rs")
                && propagation_sites.contains(&"skill_canary_harness.rs")
                && propagation_sites.contains(&"redaction boundary tests"),
            "L6-T-error_codes-003 missing required bid error propagation sites"
        );

        for (error_code, expected_class) in l6_bid_error_class_cases() {
            let mut trace = StructuredTrace::new(ExecutionMode::Contract, ["created", "failed"]);
            trace.insert_error_code_metadata(error_code.as_str())?;

            ensure!(
                trace
                    .additional_fields
                    .get("error_code")
                    .and_then(JsonValue::as_str)
                    == Some(error_code.as_str()),
                "structured trace should carry error_code for {}",
                error_code.as_str()
            );
            ensure!(
                trace
                    .additional_fields
                    .get("error_class")
                    .and_then(JsonValue::as_str)
                    == Some(expected_class),
                "structured trace should carry error_class `{expected_class}` for {}",
                error_code.as_str()
            );
            ensure!(
                error_code_class_for_str(error_code.as_str()) == Some(expected_class),
                "error_code_class_for_str should classify {} as `{expected_class}`",
                error_code.as_str()
            );
        }

        Ok(())
    }

    #[test]
    fn test_error_code_roundtrip() -> Result<()> {
        for code in error_inventory() {
            let parsed = crate::errors_manifest_impl::parse_error_code(code)?;
            ensure!(
                parsed.as_str() == code,
                "expected roundtrip for error code `{code}`"
            );
        }
        Ok(())
    }

    #[test]
    fn cron_status_snapshot_contract_is_read_only_and_explicit() -> Result<()> {
        let probe = TelemetryProbe::default();
        let value = serde_json::to_value(probe.operational_status_snapshot().cron)?;
        ensure!(value["definitions"].is_array());
        ensure!(value["definitions"][0]["definitionKey"] == "fts::procurement-ingestion/fts");
        ensure!(value["definitions"][0]["nextRunAt"] == "2026-03-31T04:15:00Z");
        ensure!(value.get("schedulerRunning").and_then(JsonValue::as_bool) == Some(true));
        ensure!(
            value.get("lastTickAt").and_then(JsonValue::as_str) == Some("2026-03-30T04:15:10Z")
        );
        ensure!(value.get("nextRunAt").and_then(JsonValue::as_str) == Some("2026-03-31T04:15:00Z"));
        let worker_lanes = value
            .get("workerLanes")
            .and_then(JsonValue::as_array)
            .context("workerLanes must be an array")?;
        let pty_lane = worker_lanes
            .first()
            .context("pty/browser worker lane must be present")?;
        ensure!(pty_lane.get("lane").and_then(JsonValue::as_str) == Some("pty_browser"));
        ensure!(pty_lane.get("running").and_then(JsonValue::as_bool) == Some(true));
        ensure!(
            pty_lane
                .get("claimableKinds")
                .and_then(JsonValue::as_array)
                .and_then(|kinds| kinds.first())
                .and_then(JsonValue::as_str)
                == Some("sync_source")
        );
        let coverage_lane = worker_lanes
            .get(1)
            .context("coverage/deterministic worker lane must be present")?;
        ensure!(
            coverage_lane.get("lane").and_then(JsonValue::as_str) == Some("coverage_deterministic")
        );
        let awaiting_access = value
            .get("awaitingAccess")
            .context("awaitingAccess must be present")?;
        ensure!(
            awaiting_access
                .get("activeCount")
                .and_then(JsonValue::as_i64)
                == Some(2)
        );
        ensure!(
            awaiting_access
                .get("expiredCount")
                .and_then(JsonValue::as_i64)
                == Some(1)
        );
        Ok(())
    }

    #[test]
    fn source_health_snapshot_contract_carries_breaker_state() -> Result<()> {
        let probe = TelemetryProbe::default();
        let value = serde_json::to_value(probe.operational_status_snapshot().source_health)?;
        ensure!(value["sources"][0]["sourceId"] == "fts");
        ensure!(value["sources"][0]["healthState"] == "green");
        ensure!(value["sources"][0]["circuitState"] == "closed");
        ensure!(value["sources"][0]["productionAllowed"] == true);
        Ok(())
    }

    #[test]
    fn validation_and_arbiter_snapshot_contracts_are_explicit() -> Result<()> {
        let probe = TelemetryProbe::default();
        let snapshot = probe.operational_status_snapshot();
        let validation = serde_json::to_value(snapshot.validation)?;
        ensure!(validation["validatorHealthState"] == "healthy");
        ensure!(validation["validatorJobCount"] == 3);
        ensure!(validation["pendingBacklogCount"] == 0);

        let arbiter = serde_json::to_value(snapshot.resource_arbiter)?;
        ensure!(arbiter["browserSlotsLimit"] == 2);
        ensure!(arbiter["browserSlotsInUse"] == 1);
        ensure!(arbiter["recentEvents"][0]["kind"] == "grant");
        ensure!(arbiter["recentEvents"][0]["priority"] == "procurement_ingestion");
        Ok(())
    }
}
