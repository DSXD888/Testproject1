#![forbid(unsafe_code)]

#[path = "../src/errors/manifest.rs"]
#[allow(dead_code)]
mod errors_manifest_impl;
#[path = "../src/errors/markdown.rs"]
#[allow(dead_code)]
mod errors_markdown_impl;

mod tests {
    use anyhow::{Result, ensure};

    use crate::errors_manifest_impl::ErrorCode;

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
}
