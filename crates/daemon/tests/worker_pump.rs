#![forbid(unsafe_code)]

mod tests {
    use anyhow::{Context, Result, ensure};

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
