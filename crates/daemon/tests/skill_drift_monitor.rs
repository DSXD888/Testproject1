#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

#[path = "support/db.rs"]
mod db_support;
#[path = "../src/errors/manifest.rs"]
#[allow(dead_code)]
mod errors_manifest_impl;
#[path = "../src/errors/markdown.rs"]
#[allow(dead_code)]
mod errors_markdown_impl;
#[path = "support/skills.rs"]
mod skills_support;
#[path = "../src/sources/registry.rs"]
#[allow(dead_code)]
mod sources_impl;
mod support;

mod tests {
    use std::fs;

    use anyhow::{Result, anyhow, ensure};
    use chrono::{DateTime, Duration, Utc};
    use rusqlite::{Connection, params};
    use serde_json::Value;

    use crate::db_support::TestStateDb;
    use crate::errors_manifest_impl::ErrorCode;
    use crate::skills_support::{lint_skill_directory, repo_skill_dir};
    use crate::sources_impl::{
        CanaryResult, CircuitState, DEFAULT_BREAKER_CHALLENGE_THRESHOLD,
        DEFAULT_BREAKER_COOLDOWN_MINUTES, DEFAULT_SOURCE_HEALTH_CADENCE_MINUTES, HealthState,
        SourceRegistry, SourceRuntimeState, persist_runtime_state,
    };
    use crate::support::source_health::{
        CanaryOutcomeKind, CanaryResult as FixtureCanaryResult, CircuitBreakerFixture,
        CircuitEvent, CircuitState as FixtureCircuitState, SourceHealthFixture,
    };

    fn seed_source(conn: &Connection, source_id: &str, kind: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT OR IGNORE INTO sources (id, kind, enabled, config)
            VALUES (?1, ?2, 1, '{}')
            ",
            params![source_id, kind],
        )?;
        Ok(())
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

    fn failure_codes_from_markdown(markdown: &str) -> Vec<&str> {
        let mut in_section = false;
        let mut codes = Vec::new();
        for line in markdown.lines() {
            let trimmed = line.trim();
            if trimmed == "## Failure Codes" {
                in_section = true;
                continue;
            }
            if in_section && trimmed.starts_with("## ") {
                break;
            }
            if in_section && let Some(code) = trimmed.strip_prefix("- ") {
                codes.push(code);
            }
        }
        codes
    }

    fn fresh_timestamp(runtime_state: &SourceRuntimeState) -> DateTime<Utc> {
        [
            runtime_state.last_canary_at,
            runtime_state.updated_at,
            runtime_state.circuit_opened_at,
            runtime_state.created_at,
        ]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or_else(Utc::now)
            + Duration::minutes(5)
    }

    #[test]
    fn l3_t_skill_drift_monitor_001_canary_packages_expose_six_checks_and_error_codes() -> Result<()>
    {
        let inventory = error_inventory();
        for relative in ["source-health/fts", "source-health/alibaba"] {
            let skill_dir = repo_skill_dir(relative)?;
            let linted = lint_skill_directory(&skill_dir)?;
            let schema_path = skill_dir.join("result.schema.json");
            let schema = serde_json::from_str::<Value>(&fs::read_to_string(schema_path)?)?;
            let required_checks = schema["properties"]["checks"]["required"]
                .as_array()
                .ok_or_else(|| anyhow!("L3-T-skill_drift_monitor-001 missing checks.required"))?
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();

            ensure!(
                required_checks
                    == vec![
                        "reachability",
                        "fingerprint",
                        "auth_state",
                        "locale",
                        "challenge_rate",
                        "minimal_extraction",
                    ],
                "L3-T-skill_drift_monitor-001 expected fixed six-check contract for {relative}"
            );

            for code in failure_codes_from_markdown(&linted.skill_markdown) {
                ensure!(
                    inventory.contains(&code),
                    "L3-T-skill_drift_monitor-001 failure code `{code}` missing from ErrorCode inventory"
                );
            }
        }

        Ok(())
    }

    #[test]
    fn l3_t_skill_drift_monitor_002_canary_cadence_defaults_and_outcomes_persist_explicitly()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        ensure!(state_db.db_path().ends_with("state.db"));
        seed_source(state_db.connection(), "fts", "procurement")?;
        seed_source(state_db.connection(), "alibaba", "supplier")?;
        seed_source(state_db.connection(), "contracts-finder", "procurement")?;
        let registry = SourceRegistry::load(state_db.connection())?;
        for (source_id, result, health_state, last_error_code) in [
            ("fts", CanaryResult::Passed, HealthState::Green, None),
            (
                "alibaba",
                CanaryResult::Failed,
                HealthState::Red,
                Some("SOURCE_SOFT_BLOCKED"),
            ),
            (
                "contracts-finder",
                CanaryResult::TimedOut,
                HealthState::Red,
                Some("HOST_SLEEP_INTERRUPTED"),
            ),
        ] {
            let registered = registry
                .get(source_id)
                .ok_or_else(|| anyhow!("missing source {source_id}"))?;
            let observed_at = fresh_timestamp(&registered.runtime_state);
            ensure!(
                registered.definition.policy.canary_cadence_minutes
                    == DEFAULT_SOURCE_HEALTH_CADENCE_MINUTES,
                "L3-T-skill_drift_monitor-002 unexpected canary cadence for {source_id}"
            );

            let next = registered
                .runtime_state
                .with_canary_outcome(
                    &registered.definition,
                    observed_at,
                    result,
                    health_state,
                    last_error_code.map(str::to_owned),
                )
                .ok_or_else(|| anyhow!("missing canary outcome for {source_id}"))?;
            persist_runtime_state(state_db.connection(), &next)?;
        }

        // Restrict the assertion to the three sources this test exercises. The
        // state-db open path also auto-seeds the procontract-* sources whose
        // runtime rows live in the same table, but they are out of scope for
        // this canary-outcome check.
        let mut statement = state_db.connection().prepare(
            "\
            SELECT source_id, last_health_state, last_canary_result, last_error_code
            FROM source_runtime_state
            WHERE source_id IN ('alibaba', 'contracts-finder', 'fts')
            ORDER BY source_id
            ",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        let observed = rows.collect::<std::result::Result<Vec<_>, _>>()?;

        ensure!(
            observed
                == vec![
                    (
                        "alibaba".to_owned(),
                        "red".to_owned(),
                        Some("failed".to_owned()),
                        Some("SOURCE_SOFT_BLOCKED".to_owned()),
                    ),
                    (
                        "contracts-finder".to_owned(),
                        "red".to_owned(),
                        Some("timed_out".to_owned()),
                        Some("HOST_SLEEP_INTERRUPTED".to_owned()),
                    ),
                    (
                        "fts".to_owned(),
                        "green".to_owned(),
                        Some("passed".to_owned()),
                        None,
                    ),
                ],
            "L3-T-skill_drift_monitor-002 expected explicit persisted canary outcomes"
        );
        Ok(())
    }

    #[test]
    fn l3_t_skill_drift_monitor_003_challenge_threshold_opens_circuit_and_sets_cooldown()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), "alibaba", "supplier")?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let registered = registry
            .get("alibaba")
            .ok_or_else(|| anyhow!("missing alibaba source"))?;
        let base = fresh_timestamp(&registered.runtime_state);

        let first = registered
            .runtime_state
            .with_challenge_failure(&registered.definition, base, "AGENT_AUTH_CHALLENGE")
            .ok_or_else(|| anyhow!("first challenge failure missing"))?;
        let second = first
            .with_challenge_failure(
                &registered.definition,
                base + Duration::minutes(10),
                "SOURCE_SOFT_BLOCKED",
            )
            .ok_or_else(|| anyhow!("second challenge failure missing"))?;
        let third = second
            .with_challenge_failure(
                &registered.definition,
                base + Duration::minutes(20),
                "SOURCE_CAPTCHA_BLOCKED",
            )
            .ok_or_else(|| anyhow!("third challenge failure missing"))?;

        ensure!(
            third.challenge_event_count == DEFAULT_BREAKER_CHALLENGE_THRESHOLD,
            "L3-T-skill_drift_monitor-003 expected challenge count to reach threshold"
        );
        ensure!(
            third.circuit_state == CircuitState::Open,
            "L3-T-skill_drift_monitor-003 expected breaker to open at threshold"
        );
        ensure!(
            third.last_health_state == HealthState::Red,
            "L3-T-skill_drift_monitor-003 expected red health after breaker trip"
        );
        ensure!(
            third.cooldown_until == Some(base + Duration::minutes(140)),
            "L3-T-skill_drift_monitor-003 expected 120-minute cooldown"
        );
        for (offset_minutes, result, expected_error_code) in [
            (5, CanaryResult::Failed, "SOURCE_SOFT_BLOCKED"),
            (10, CanaryResult::TimedOut, "HOST_SLEEP_INTERRUPTED"),
        ] {
            let observed_at = third
                .cooldown_until
                .ok_or_else(|| anyhow!("missing cooldown for recovery canary"))?
                + Duration::minutes(offset_minutes);
            let reopened = third
                .with_canary_outcome(
                    &registered.definition,
                    observed_at,
                    result,
                    HealthState::Red,
                    Some(expected_error_code.to_owned()),
                )
                .ok_or_else(|| anyhow!("recovery canary outcome missing"))?;

            ensure!(
                reopened.circuit_state == CircuitState::Open,
                "L3-T-skill_drift_monitor-003 expected failed recovery canary to reopen circuit"
            );
            ensure!(
                reopened.circuit_opened_at == Some(observed_at),
                "L3-T-skill_drift_monitor-003 expected fresh circuit_opened_at after recovery failure"
            );
            ensure!(
                reopened.cooldown_until == Some(observed_at + Duration::minutes(120)),
                "L3-T-skill_drift_monitor-003 expected fresh cooldown after recovery failure"
            );
            ensure!(
                reopened.last_health_state == HealthState::Red
                    && reopened.last_canary_result == Some(result)
                    && reopened.last_error_code.as_deref() == Some(expected_error_code),
                "L3-T-skill_drift_monitor-003 expected recovery failure metadata persistence"
            );
        }
        Ok(())
    }

    #[test]
    fn l3_t_skill_drift_monitor_004_stale_canary_success_cannot_close_newer_open_state()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), "alibaba", "supplier")?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let registered = registry
            .get("alibaba")
            .ok_or_else(|| anyhow!("missing alibaba source"))?;
        let base = fresh_timestamp(&registered.runtime_state);

        let open_state = registered
            .runtime_state
            .with_challenge_failure(&registered.definition, base, "AGENT_AUTH_CHALLENGE")
            .and_then(|state| {
                state.with_challenge_failure(
                    &registered.definition,
                    base + Duration::minutes(10),
                    "SOURCE_SOFT_BLOCKED",
                )
            })
            .and_then(|state| {
                state.with_challenge_failure(
                    &registered.definition,
                    base + Duration::minutes(20),
                    "SOURCE_CAPTCHA_BLOCKED",
                )
            })
            .ok_or_else(|| anyhow!("failed to build open breaker state"))?;
        ensure!(persist_runtime_state(state_db.connection(), &open_state)?);

        let stale_success = SourceRuntimeState {
            source_id: open_state.source_id.clone(),
            last_health_state: HealthState::Green,
            circuit_state: CircuitState::Closed,
            challenge_event_count: 0,
            challenge_window_started_at: None,
            circuit_opened_at: None,
            cooldown_until: None,
            last_canary_at: Some(base + Duration::minutes(5)),
            last_canary_result: Some(CanaryResult::Passed),
            last_error_code: None,
            updated_at: Some(base + Duration::minutes(5)),
            created_at: open_state.created_at,
        };
        ensure!(
            !persist_runtime_state(state_db.connection(), &stale_success)?,
            "L3-T-skill_drift_monitor-004 expected stale success write to be ignored"
        );

        let reloaded = SourceRegistry::load(state_db.connection())?;
        let reloaded_source = reloaded
            .get("alibaba")
            .ok_or_else(|| anyhow!("missing alibaba source"))?;
        ensure!(
            reloaded_source.runtime_state.circuit_state == CircuitState::Open,
            "L3-T-skill_drift_monitor-004 stale success must not close newer open circuit"
        );
        Ok(())
    }

    #[test]
    fn l3_t_skill_drift_monitor_005_manual_reset_moves_open_circuit_to_half_open() -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), "alibaba", "supplier")?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let runtime_state = &registry
            .get("alibaba")
            .ok_or_else(|| anyhow!("missing alibaba source"))?
            .runtime_state;
        let base = fresh_timestamp(runtime_state);

        let open_state = SourceRuntimeState {
            source_id: "alibaba".to_owned(),
            last_health_state: HealthState::Red,
            circuit_state: CircuitState::Open,
            challenge_event_count: DEFAULT_BREAKER_CHALLENGE_THRESHOLD,
            challenge_window_started_at: Some(base),
            circuit_opened_at: Some(base),
            cooldown_until: Some(base + Duration::hours(2)),
            last_canary_at: Some(base),
            last_canary_result: None,
            last_error_code: Some("SOURCE_CAPTCHA_BLOCKED".to_owned()),
            updated_at: Some(base),
            created_at: Some(base),
        };
        persist_runtime_state(state_db.connection(), &open_state)?;
        let reset = open_state
            .with_manual_reset(base + Duration::minutes(30))
            .ok_or_else(|| anyhow!("manual reset should return a new state"))?;
        persist_runtime_state(state_db.connection(), &reset)?;

        let reloaded = SourceRegistry::load(state_db.connection())?;
        let reloaded_source = reloaded
            .get("alibaba")
            .ok_or_else(|| anyhow!("missing alibaba source"))?;
        ensure!(
            reloaded_source.runtime_state.circuit_state == CircuitState::HalfOpen,
            "L3-T-skill_drift_monitor-005 expected half-open after manual reset"
        );
        ensure!(
            reloaded_source.runtime_state.cooldown_until.is_none(),
            "L3-T-skill_drift_monitor-005 expected cooldown to clear on manual reset"
        );
        ensure!(
            reloaded_source.definition.policy.cooldown_minutes == DEFAULT_BREAKER_COOLDOWN_MINUTES,
            "L3-T-skill_drift_monitor-005 source definition should remain unchanged"
        );
        Ok(())
    }

    #[test]
    fn l3_t_skill_drift_monitor_006_optional_metrics_are_not_required_or_zero_defaulted()
    -> Result<()> {
        for relative in ["source-health/fts", "source-health/alibaba"] {
            let schema_path = repo_skill_dir(relative)?.join("result.schema.json");
            let schema = serde_json::from_str::<Value>(&fs::read_to_string(schema_path)?)?;

            ensure!(
                !schema["required"]
                    .as_array()
                    .ok_or_else(|| anyhow!("missing top-level required set"))?
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|value| value == "sample_item_count"),
                "L3-T-skill_drift_monitor-006 expected sample_item_count to stay optional for {relative}"
            );
            ensure!(
                schema["properties"]["sample_item_count"]
                    .get("default")
                    .is_none(),
                "L3-T-skill_drift_monitor-006 expected no synthetic zero default for {relative}"
            );
        }
        Ok(())
    }

    #[test]
    fn source_health_support_fixtures_cover_runtime_and_breaker_paths() -> Result<()> {
        let canary_fixture = SourceHealthFixture::from_fixture("fts")?;
        let canary_state = canary_fixture.run_canary(
            "fts",
            CanaryOutcomeKind::Failed("SOURCE_SOFT_BLOCKED".to_owned()),
        )?;
        ensure!(canary_state.last_canary_result == Some(FixtureCanaryResult::Failed));
        ensure!(canary_state.last_error_code.as_deref() == Some("SOURCE_SOFT_BLOCKED"));

        let breaker_fixture = CircuitBreakerFixture::from_fixture("alibaba")?;
        let base = Utc::now();
        for (offset_minutes, error_code) in [
            (0, "AGENT_AUTH_CHALLENGE"),
            (10, "SOURCE_SOFT_BLOCKED"),
            (20, "SOURCE_CAPTCHA_BLOCKED"),
        ] {
            let _ = breaker_fixture.transition(
                "alibaba",
                CircuitEvent::ChallengeFailure(error_code.to_owned()),
                base + Duration::minutes(offset_minutes),
            )?;
        }
        breaker_fixture.assert_state(
            "alibaba",
            FixtureCircuitState::Open,
            Some("SOURCE_CAPTCHA_BLOCKED"),
        )?;
        Ok(())
    }
}
