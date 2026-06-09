#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

#[allow(dead_code)]
#[path = "../src/agents/arbiter.rs"]
mod arbiter_impl;

mod tests {
    use anyhow::{Context, Result, ensure};

    use crate::arbiter_impl::{
        AdmissionPriority, AdmissionRequest, ArbiterDecision, ArbiterEventKind, DenialReason,
        ReleaseCause, ResourceArbiter, ResourceBudgets, SourceKind, SourceLaneRequest,
    };

    fn browser_request(job_id: &str, source_id: &str) -> AdmissionRequest {
        AdmissionRequest {
            job_id: job_id.to_owned(),
            session_id: Some(format!("session-{job_id}")),
            priority: AdmissionPriority::ProcurementIngestion,
            browser_session: true,
            adapter_session: true,
            source_lane: Some(SourceLaneRequest {
                source_id: source_id.to_owned(),
                source_kind: SourceKind::Procurement,
            }),
            validator_sub_job: false,
            concurrency_key: Some("procurement-ingestion/fts".to_owned()),
            max_concurrency: None,
        }
    }

    fn agent_request(job_id: &str) -> AdmissionRequest {
        AdmissionRequest {
            job_id: job_id.to_owned(),
            session_id: Some(format!("session-{job_id}")),
            priority: AdmissionPriority::AuthRefreshOrHealthProbe,
            browser_session: false,
            adapter_session: true,
            source_lane: None,
            validator_sub_job: false,
            concurrency_key: None,
            max_concurrency: None,
        }
    }

    fn skill_cap_request(job_id: &str, key: &str) -> AdmissionRequest {
        AdmissionRequest {
            job_id: job_id.to_owned(),
            session_id: Some(format!("session-{job_id}")),
            priority: AdmissionPriority::Qualification,
            browser_session: false,
            adapter_session: false,
            source_lane: None,
            validator_sub_job: false,
            concurrency_key: Some(key.to_owned()),
            max_concurrency: Some(1),
        }
    }

    fn section_between<'a>(source: &'a str, start: &str, end: &str) -> Result<&'a str> {
        let start_index = source
            .find(start)
            .with_context(|| format!("missing {start}"))?;
        let tail = source
            .get(start_index..)
            .with_context(|| format!("start marker {start} is not on a UTF-8 boundary"))?;
        let relative_end = tail
            .find(end)
            .with_context(|| format!("missing {end} after {start}"))?;
        let end_index = start_index
            .checked_add(relative_end)
            .with_context(|| format!("section end overflows after {start}"))?;
        source
            .get(start_index..end_index)
            .with_context(|| format!("section {start}..{end} is not on UTF-8 boundaries"))
    }

    fn assert_contains_all(section: &str, markers: &[&str]) -> Result<()> {
        for marker in markers {
            ensure!(section.contains(marker), "missing `{marker}`");
        }
        Ok(())
    }

    fn assert_worker_002_defer_path_markers(main_rs: &str) -> Result<()> {
        let worker_iteration = section_between(
            main_rs,
            "async fn run_shared_worker_iteration_for_lane",
            "fn defer_claimed_job_without_attempt",
        )?;
        assert_contains_all(
            worker_iteration,
            &[
                "claim_next_job_for_kinds",
                "lane.claimable_kinds()",
                "denial_cooldowns.is_active_for_claimed(lane, &claimed)",
                "defer_claimed_job_without_attempt(runtime, &claimed.id, \"active resource denial cooldown\")",
                "denial_cooldowns.record(lane, &claimed_for_cooldown, &deferred.reason)",
            ],
        )?;

        let defer_body = section_between(
            main_rs,
            "fn defer_claimed_job_without_attempt",
            "#[allow(clippy::future_not_send)]\nasync fn execute_claimed_workflow_job",
        )?;
        assert_contains_all(
            defer_body,
            &[
                "SET status = 'pending'",
                "lease_owner = NULL",
                "lease_expires_at = NULL",
                "attempt_count = CASE",
                "WHEN attempt_count > 0 THEN attempt_count - 1",
                "AND status = 'running'",
                "updated == 1",
            ],
        )?;

        let resource_admission = section_between(
            main_rs,
            "fn claimed_job_resource_lease",
            "fn admission_priority",
        )?;
        assert_contains_all(
            resource_admission,
            &[
                "arbiter.admit(&request)",
                "ArbiterDecision::Denied(reason)",
                "defer_claimed_job_without_attempt(runtime, job_id, \"resource arbiter denial\")",
                "ResourceAdmissionDeferred",
                "reason,",
            ],
        )?;

        let bin_test = section_between(
            main_rs,
            "fn autonomous_worker_denial_defer_clears_lease_and_restores_attempt_count",
            "#[test]\n    fn autonomous_worker_denial_cooldown_handles_every_reason_variant",
        )?;
        assert_contains_all(
            bin_test,
            &[
                "'job-denied', 'sync_source', 'running'",
                "'local-daemon'",
                "'2026-06-08T12:05:00Z'",
                "1, '{}'",
                "super::defer_claimed_job_without_attempt(&runtime, \"job-denied\", \"test resource denial\")",
                "SELECT status, lease_owner, lease_expires_at, attempt_count",
                "assert_eq!(row.0, \"pending\")",
                "assert_eq!(row.1, None)",
                "assert_eq!(row.2, None)",
                "assert_eq!(row.3, 0)",
            ],
        )?;
        Ok(())
    }

    fn assert_worker_002_browser_slot_denial() -> Result<()> {
        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
        let first = match arbiter.admit(&browser_request("held-browser-1", "fts-a")) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                anyhow::bail!("first browser-backed lease was denied: {reason:?}");
            }
        };
        let second = match arbiter.admit(&browser_request("held-browser-2", "fts-b")) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                anyhow::bail!("second browser-backed lease was denied: {reason:?}");
            }
        };

        ensure!(
            matches!(
                arbiter.admit(&browser_request("denied-browser-3", "fts-c")),
                ArbiterDecision::Denied(DenialReason::BrowserSlotsExhausted {
                    limit: 2,
                    in_use: 2,
                })
            ),
            "worker_002 must force BrowserSlotsExhausted at the default 2-slot cap"
        );
        let denied_snapshot = arbiter.snapshot();
        ensure!(denied_snapshot.browser_slots == 2);
        ensure!(!denied_snapshot.source_lanes.contains_key("fts-c"));
        ensure!(
            arbiter.events().iter().any(|event| {
                event.kind == ArbiterEventKind::Deny
                    && event.job_id == "denied-browser-3"
                    && matches!(
                        event.denial_reason,
                        Some(DenialReason::BrowserSlotsExhausted {
                            limit: 2,
                            in_use: 2,
                        })
                    )
            }),
            "worker_002 needs observable arbiter denial evidence"
        );

        arbiter.release(&first, ReleaseCause::Reaped);
        arbiter.release(&second, ReleaseCause::Reaped);
        ensure!(arbiter.snapshot().browser_slots == 0);
        Ok(())
    }

    #[test]
    fn worker_002_defer_on_denial_no_attempt_bump() -> Result<()> {
        let main_rs = include_str!("../src/main.rs");
        assert_worker_002_defer_path_markers(main_rs)?;
        assert_worker_002_browser_slot_denial()
    }

    #[test]
    fn worker_003_source_lane_serializes_same_source() -> Result<()> {
        let main_rs = include_str!("../src/main.rs");
        let cooldown_test = section_between(
            main_rs,
            "fn autonomous_worker_denial_cooldown_handles_every_reason_variant",
            "fn assert_commercial_sweep_skill_subpasses_use_arbiter_without_browser_lane",
        )?;
        assert_contains_all(
            cooldown_test,
            &[
                "DenialReason::SourceLaneExhausted",
                "source_id: FTS_SOURCE_ID.to_owned()",
                "limit: 1",
                "in_use: 1",
                "assert!(source_cooldowns.is_active_for_claimed(lane, &fts_job))",
                "assert!(!source_cooldowns.is_active_for_claimed(lane, &contracts_finder_job))",
            ],
        )?;

        let arbiter_defaults = section_between(
            main_rs,
            "fn build_resource_arbiter_status_snapshot",
            "fn resource_arbiter_event_snapshot",
        )?;
        assert_contains_all(
            arbiter_defaults,
            &[
                "procurement_source_lane_limit: budgets.procurement_source_lanes",
                "supplier_source_lane_limit: budgets.supplier_source_lanes",
                "source_lanes_in_use: snapshot.source_lanes",
            ],
        )?;

        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
        ensure!(arbiter.budgets().procurement_source_lanes == 1);
        ensure!(arbiter.budgets().supplier_source_lanes == 2);

        let held_fts = match arbiter.admit(&browser_request("held-fts-sync", "fts")) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                anyhow::bail!("first FTS sync_source was denied: {reason:?}");
            }
        };
        ensure!(
            matches!(
                arbiter.admit(&browser_request("deferred-fts-sync", "fts")),
                ArbiterDecision::Denied(DenialReason::SourceLaneExhausted {
                    source_id,
                    limit: 1,
                    in_use: 1,
                }) if source_id == "fts"
            ),
            "worker_003 must deny the second same-source procurement sync_source"
        );
        let denied_snapshot = arbiter.snapshot();
        ensure!(denied_snapshot.browser_slots == 1);
        ensure!(denied_snapshot.source_lanes.get("fts") == Some(&1));

        arbiter.release(&held_fts, ReleaseCause::Archived);
        ensure!(arbiter.snapshot().source_lanes.is_empty());
        ensure!(
            matches!(
                arbiter.admit(&browser_request("deferred-fts-sync", "fts")),
                ArbiterDecision::Granted(_)
            ),
            "worker_003 must allow the deferred same-source job after release"
        );
        ensure!(
            arbiter.events().iter().any(|event| {
                event.kind == ArbiterEventKind::Deny
                    && event.job_id == "deferred-fts-sync"
                    && matches!(
                        &event.denial_reason,
                        Some(DenialReason::SourceLaneExhausted {
                            source_id,
                            limit: 1,
                            in_use: 1,
                        }) if source_id == "fts"
                    )
            }),
            "worker_003 needs observable source-lane denial evidence"
        );
        ensure!(
            arbiter.events().iter().any(|event| {
                event.kind == ArbiterEventKind::Release
                    && event.job_id == "held-fts-sync"
                    && event.source_lane_key.as_deref() == Some("fts")
            }),
            "worker_003 must prove the source lane releases before the deferred run"
        );
        Ok(())
    }

    fn assert_worker_004_lane_split_markers(main_rs: &str) -> Result<()> {
        let lane_constants = section_between(
            main_rs,
            "const AUTONOMOUS_WORKER_LANES",
            "const AUTONOMOUS_WORKER_IDLE_BACKOFF_MS",
        )?;
        assert_contains_all(
            lane_constants,
            &[
                "AutonomousWorkerLane::PtyBrowser",
                "AutonomousWorkerLane::CoverageDeterministic",
                "\"sync_source\"",
                "\"source_health\"",
                "\"auth_refresh\"",
                "\"commercial_sweep\"",
                "\"validation_cross_model_audit\"",
                "\"validation_calibration_readiness_sweep\"",
            ],
        )?;

        let worker_spawn = section_between(
            main_rs,
            "fn spawn_autonomous_worker_threads",
            "fn serve_handshake_once",
        )?;
        assert_contains_all(
            worker_spawn,
            &[
                "Vec::with_capacity(AUTONOMOUS_WORKER_LANES.len())",
                "for lane in AUTONOMOUS_WORKER_LANES",
                "AutonomousWorkerThread::spawn(",
                "Arc::clone(&runtime.resource_arbiter)",
            ],
        )?;

        let worker_loop = section_between(
            main_rs,
            "fn run_autonomous_worker_loop",
            "fn autonomous_worker_wait_for_job_or_stop",
        )?;
        assert_contains_all(
            worker_loop,
            &[
                "run_shared_worker_iteration_for_lane(",
                "lane,",
                "autonomous_worker_wait_for_job_or_stop(stop_rx, idle_backoff)",
            ],
        )?;

        let worker_iteration = section_between(
            main_rs,
            "async fn run_shared_worker_iteration_for_lane",
            "fn defer_claimed_job_without_attempt",
        )?;
        assert_contains_all(
            worker_iteration,
            &[
                "claim_next_job_for_kinds",
                "lane.claimable_kinds()",
                "Ok(Some(report))",
                "Ok(None)",
            ],
        )?;
        Ok(())
    }

    #[test]
    fn worker_004_head_of_line() -> Result<()> {
        let main_rs = include_str!("../src/main.rs");
        assert_worker_004_lane_split_markers(main_rs)?;

        let lane_filter_test = section_between(
            main_rs,
            "fn autonomous_worker_lane_kind_filters_are_disjoint_and_complete",
            "#[test]\n    fn autonomous_worker_threads_spawn_both_lanes_and_shutdown_idle",
        )?;
        assert_contains_all(
            lane_filter_test,
            &[
                "assert_eq!(pty_kinds, [\"sync_source\", \"source_health\", \"auth_refresh\"])",
                "\"commercial_sweep\"",
                "assert!(pty_set.is_disjoint(&coverage_set))",
                "assert_eq!(pty_set.len() + coverage_set.len(), 6)",
            ],
        )?;
        Ok(())
    }

    fn assert_browser_quota_caps_at_two() -> Result<()> {
        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
        let first = match arbiter.admit(&browser_request("quota-browser-1", "fts-quota-a")) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                anyhow::bail!("first browser quota lease was denied: {reason:?}");
            }
        };
        let second = match arbiter.admit(&browser_request("quota-browser-2", "fts-quota-b")) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                anyhow::bail!("second browser quota lease was denied: {reason:?}");
            }
        };

        ensure!(
            matches!(
                arbiter.admit(&browser_request("quota-browser-3", "fts-quota-c")),
                ArbiterDecision::Denied(DenialReason::BrowserSlotsExhausted {
                    limit: 2,
                    in_use: 2,
                })
            ),
            "worker_005 must force BrowserSlotsExhausted at the two-slot cap"
        );
        let max_browser_slots = arbiter
            .events()
            .iter()
            .map(|event| event.browser_slots_in_use)
            .max()
            .unwrap_or_default();
        ensure!(max_browser_slots == 2);
        ensure!(
            arbiter
                .events()
                .iter()
                .all(|event| event.browser_slots_in_use <= 2)
        );

        arbiter.release(&first, ReleaseCause::Archived);
        arbiter.release(&second, ReleaseCause::Archived);
        ensure!(arbiter.snapshot().browser_slots == 0);
        Ok(())
    }

    fn assert_agent_slot_denial_and_release() -> Result<()> {
        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
        let mut leases = Vec::new();
        for index in 0..4 {
            let request = agent_request(&format!("quota-agent-{index}"));
            match arbiter.admit(&request) {
                ArbiterDecision::Granted(lease) => leases.push(lease),
                ArbiterDecision::Denied(reason) => {
                    anyhow::bail!("agent lease {index} was denied: {reason:?}");
                }
            }
        }

        ensure!(
            matches!(
                arbiter.admit(&agent_request("quota-agent-denied")),
                ArbiterDecision::Denied(DenialReason::AgentSlotsExhausted {
                    limit: 4,
                    in_use: 4,
                })
            ),
            "worker_005 must force AgentSlotsExhausted at the four-slot cap"
        );
        for lease in &leases {
            arbiter.release(lease, ReleaseCause::Reaped);
        }
        ensure!(arbiter.snapshot().agent_slots == 0);
        Ok(())
    }

    fn assert_source_lane_denial_and_release() -> Result<()> {
        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
        let held = match arbiter.admit(&browser_request("quota-source-held", "fts-quota")) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                anyhow::bail!("source-lane quota lease was denied: {reason:?}");
            }
        };

        ensure!(
            matches!(
                arbiter.admit(&browser_request("quota-source-denied", "fts-quota")),
                ArbiterDecision::Denied(DenialReason::SourceLaneExhausted {
                    source_id,
                    limit: 1,
                    in_use: 1,
                }) if source_id == "fts-quota"
            ),
            "worker_005 must force SourceLaneExhausted for same-source procurement"
        );
        arbiter.release(&held, ReleaseCause::Archived);
        ensure!(arbiter.snapshot().source_lanes.is_empty());
        Ok(())
    }

    fn assert_skill_concurrency_denial_and_release() -> Result<()> {
        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
        let key = "coverage/skill-cap";
        let held = match arbiter.admit(&skill_cap_request("quota-skill-held", key)) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                anyhow::bail!("skill concurrency quota lease was denied: {reason:?}");
            }
        };

        ensure!(
            matches!(
                arbiter.admit(&skill_cap_request("quota-skill-denied", key)),
                ArbiterDecision::Denied(DenialReason::SkillConcurrencyCapReached {
                    key: denied_key,
                    limit: 1,
                    in_use: 1,
                }) if denied_key == key
            ),
            "worker_005 must force SkillConcurrencyCapReached"
        );
        arbiter.release(&held, ReleaseCause::Archived);
        ensure!(arbiter.snapshot().concurrency.is_empty());
        Ok(())
    }

    #[test]
    fn worker_005_arbiter_quota_respected() -> Result<()> {
        assert_browser_quota_caps_at_two()?;
        assert_agent_slot_denial_and_release()?;
        assert_source_lane_denial_and_release()?;
        assert_skill_concurrency_denial_and_release()
    }

    #[test]
    fn worker_001_routes_each_cron_kind_through_real_worker_executor() -> Result<()> {
        let main_rs = include_str!("../src/main.rs");
        let dispatch = section_between(
            main_rs,
            "async fn execute_claimed_workflow_job",
            "async fn run_sync_source_workflow_job",
        )?;
        assert_contains_all(
            dispatch,
            &[
                "\"sync_source\" => run_sync_source_workflow_job",
                "\"source_health\" => run_source_health_workflow_job",
                "\"auth_refresh\" => run_auth_refresh_workflow_job",
                "\"validation_cross_model_audit\"",
                "\"validation_calibration_readiness_sweep\"",
                "\"commercial_sweep\" => run_commercial_sweep_workflow_job",
                "unsupported queued job kind",
            ],
        )?;

        let worker_001 = section_between(
            main_rs,
            "async fn autonomous_worker_routes_cron_kinds_to_side_effect_handlers",
            "struct FakeManifestSkillInvoker",
        )?;
        assert_contains_all(
            worker_001,
            &[
                "execute_claimed_workflow_job(&mut runtime, &cli, &runner, claimed.clone())",
                "assert_eq!(report.status, \"completed\")",
                "assert_eq!(row.0, \"completed\")",
                "output_payload[\"job_kind\"]",
                "output_payload[\"side_effect_table\"].as_str().is_some()",
                "\"source_health\"",
                "\"auth_refresh\"",
                "\"validation_cross_model_audit\"",
                "\"validation_calibration_readiness_sweep\"",
                "\"commercial_sweep\"",
                "assert_new_cron_handler_side_effects",
                "assert_commercial_sweep_skill_subpasses_use_arbiter_without_browser_lane",
            ],
        )?;
        Ok(())
    }

    #[test]
    fn worker_001_anti_hollow_side_effect_matrix_names_every_required_surface() -> Result<()> {
        let main_rs = include_str!("../src/main.rs");
        let side_effects = section_between(
            main_rs,
            "fn assert_new_cron_handler_side_effects",
            "#[test]\n    fn autonomous_worker_denial_defer",
        )?;
        assert_contains_all(
            side_effects,
            &[
                "FROM source_runtime_state",
                "last_canary_at IS NOT NULL",
                "last_canary_result = 'passed'",
                "circuit_state = 'closed'",
                "FROM auth_refresh_results",
                "SELECT source_id, signin_status, cdp_state, result_json",
                "fake-claude auth-refresh evidence",
                "FROM validation_incident_clusters",
                "incident_signature = 'cross-model-audit:job-cross-model'",
                "affected_slice_ids_json",
                "calibration_case_count",
                "override_count",
                "candidate_mutation_template_json",
                "FROM enforcement_readiness er",
                "JOIN validation_calibration_cases vcc ON vcc.slice_id = er.slice_id",
                "SELECT er.slice_id",
                "er.worker_skill_id",
                "er.source_id",
                "er.enforcement_state",
                "er.last_evaluated_at",
                "vcc.worker_job_id",
                "vcc.validation_verdict_id",
                "FROM landed_cost_estimates",
                "FROM candidate_net_margin_assessments",
                "basis_hash",
                "FROM manifests m",
                "JOIN pipeline_assessments pa ON pa.process_id = m.process_id",
                "pipeline_state",
                "decision_basis_hash",
            ],
        )?;
        Ok(())
    }

    #[test]
    fn worker_001_commercial_sweep_asserts_required_side_effect_rows() -> Result<()> {
        let main_rs = include_str!("../src/main.rs");
        let worker_001 = section_between(
            main_rs,
            "async fn autonomous_worker_routes_cron_kinds_to_side_effect_handlers",
            "assert_new_cron_handler_side_effects(runtime.state_db.connection())",
        )?;
        assert_contains_all(
            worker_001,
            &[
                "seed_stage11_minimal_graph(runtime.state_db.connection(), \"GB\")",
                "load_stage11_rows_for_manual_retry(runtime.state_db.connection())",
                "\"job-commercial-sweep\"",
                "\"commercial_sweep\"",
                "\"stage_run_id\": \"stage-commercial-sweep\"",
            ],
        )?;

        let commercial_assertions = section_between(
            main_rs,
            "let landed_cost_stage_run_id",
            "fn assert_health_auth_shadow_payload",
        )?;
        assert_contains_all(
            commercial_assertions,
            &[
                "FROM landed_cost_estimates",
                "process_id = 'process-stage11'",
                "product_candidate_id = 'candidate-stage11'",
                "supplier_quote_id = 'quote-stage11'",
                "assert_eq!(landed_cost_stage_run_id, \"stage-commercial-sweep\")",
                "FROM candidate_net_margin_assessments",
                "stage_run_id = 'stage-commercial-sweep-net-margin'",
                "assert!(!net_margin_basis_hash.trim().is_empty())",
                "FROM manifests m",
                "JOIN pipeline_assessments pa ON pa.process_id = m.process_id",
                "assert_eq!(manifest_row.0, \"stage-commercial-sweep-manifest\")",
                "assert_eq!(manifest_row.1, \"manifest_ready\")",
                "assert!(!manifest_row.2.trim().is_empty())",
            ],
        )?;
        Ok(())
    }

    #[test]
    fn worker_001_readiness_sweep_asserts_required_side_effect_columns() -> Result<()> {
        let main_rs = include_str!("../src/main.rs");
        let readiness_assertions = section_between(
            main_rs,
            "fn assert_new_cron_handler_side_effects",
            "let landed_cost_stage_run_id",
        )?;

        assert_contains_all(
            readiness_assertions,
            &[
                "SELECT er.slice_id",
                "er.worker_skill_id",
                "er.source_id",
                "er.enforcement_state",
                "er.last_evaluated_at",
                "vcc.worker_job_id",
                "vcc.validation_verdict_id",
                "assert_eq!(readiness_row.0, \"slice-readiness\")",
                "assert_eq!(readiness_row.1, \"procurement-ingestion/fts\")",
                "assert_eq!(readiness_row.2, FTS_SOURCE_ID)",
                "assert_eq!(readiness_row.3, \"dry_run\")",
                "assert!(!readiness_row.4.trim().is_empty())",
                "assert_eq!(readiness_row.5, \"job-readiness\")",
            ],
        )?;
        Ok(())
    }

    #[test]
    fn worker_001_sync_source_route_key_matrix_covers_all_bindings() -> Result<()> {
        let main_rs = include_str!("../src/main.rs");
        let route_matrix = section_between(
            main_rs,
            "fn sync_source_route_matrix_requires_source_and_skill_tuple",
            "#[test]\n    #[allow(clippy::too_many_lines)]",
        )?;
        assert_contains_all(
            route_matrix,
            &[
                "super::FTS_SOURCE_ID",
                "super::FTS_SKILL_ID",
                "super::SyncSourceRoute::Fts",
                "super::CONTRACTS_FINDER_SOURCE_ID",
                "super::CONTRACTS_FINDER_SKILL_ID",
                "super::SyncSourceRoute::ContractsFinder",
                "super::PROCONTRACT_DISCOVER_SOURCE_ID",
                "super::PROCONTRACT_DISCOVER_SKILL_ID",
                "super::SyncSourceRoute::ProcontractDiscover",
                "super::PROCONTRACT_INTEREST_SOURCE_ID",
                "super::PROCONTRACT_INTEREST_SKILL_ID",
                "super::SyncSourceRoute::ProcontractRegisterInterest",
                "super::PROCONTRACT_ENRICH_SOURCE_ID",
                "super::PROCONTRACT_ENRICH_SKILL_ID",
                "super::SyncSourceRoute::ProcontractEnrichTender",
                "skill_binding",
                "contracts-finder/SKILL.md",
                "\"request\"",
            ],
        )?;

        let state_derived_params = section_between(
            main_rs,
            "fn scheduled_sync_source_params_derive_from_daemon_state_not_cron_payload",
            "fn sync_source_route_rejects_source_only_marketplace_and_commercial_keys",
        )?;
        assert_contains_all(
            state_derived_params,
            &[
                "seed_sync_source_cursor",
                "\"payload-should-not-win\"",
                "claimed_fts_fixture",
                "fts_runtime_job_parameters",
                "contracts_finder_runtime_job_parameters",
                "procontract_discover_runtime_job_parameters",
                "procontract_enrich_runtime_job_parameters",
                "awaitingAccessTargets",
                "advert-state",
                "source-notice-state",
                "proc-state",
            ],
        )?;

        let rejected_routes = section_between(
            main_rs,
            "fn sync_source_route_rejects_source_only_marketplace_and_commercial_keys",
            "fn source_section_between",
        )?;
        assert_contains_all(
            rejected_routes,
            &[
                "super::PROCONTRACT_DISCOVER_SOURCE_ID",
                "super::PROCONTRACT_INTEREST_SKILL_ID",
                "\"alibaba-search\"",
                "\"l5-marketplace-commercial-intelligence\"",
                "super::claimed_sync_source_route(&job).is_err()",
            ],
        )?;
        Ok(())
    }
}
