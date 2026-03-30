#![forbid(unsafe_code)]

#[path = "../src/agents/artifacts.rs"]
#[allow(dead_code)]
mod artifacts_impl;
#[path = "../src/errors/manifest.rs"]
#[allow(dead_code)]
mod errors_manifest_impl;
#[path = "../src/errors/markdown.rs"]
#[allow(dead_code)]
mod errors_markdown_impl;
#[path = "../src/pty/health.rs"]
#[allow(dead_code)]
mod health_impl;

mod tests {
    use anyhow::{Result, ensure};
    use serde_json::Value as JsonValue;
    use serde_yaml::Value as YamlValue;

    use crate::artifacts_impl::{DaemonEvidenceAppendix, OutcomeClass, RunManifestFrontmatter};
    use crate::errors_manifest_impl::ErrorCode;
    use crate::health_impl::HealthIssue;

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
            ErrorCode::GoodsClassificationLowConfidence.as_str(),
            ErrorCode::SupplierMatchLowConfidence.as_str(),
            ErrorCode::CommodityCodeLowConfidence.as_str(),
            ErrorCode::RevenueBasisUncertain.as_str(),
            ErrorCode::FpoAccessUnavailable.as_str(),
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

    #[test]
    fn l3_t_observability_telemetry_012_error_code_inventory_is_exact_and_canary_classes_map_onto_it()
    -> Result<()> {
        let inventory = error_inventory();
        let expected = vec![
            "SOURCE_CHANGED",
            "NOTICE_PARSE_FAILED",
            "GOODS_CLASSIFICATION_LOW_CONFIDENCE",
            "SUPPLIER_MATCH_LOW_CONFIDENCE",
            "COMMODITY_CODE_LOW_CONFIDENCE",
            "REVENUE_BASIS_UNCERTAIN",
            "FPO_ACCESS_UNAVAILABLE",
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
            inventory.len() == 28,
            "L3-T-observability_telemetry-012 expected exactly 28 error codes"
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
            appendix_keys.len() == 19,
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
            error_codes.len() == 28,
            "CI-L3-007 expected exact ErrorCode inventory coverage count"
        );
        Ok(())
    }
}
