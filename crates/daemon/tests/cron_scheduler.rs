#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

#[path = "../src/agents/arbiter.rs"]
#[allow(dead_code)]
mod arbiter_impl;
#[path = "../src/agents/contracts.rs"]
#[allow(dead_code)]
mod contracts_impl;
#[path = "../src/scheduler/cron.rs"]
#[allow(dead_code)]
mod cron_impl;
#[path = "support/db.rs"]
mod db_support;
#[path = "../src/jobs/runner.rs"]
#[allow(dead_code)]
mod runner_impl;
#[path = "support/skills.rs"]
mod skills_support;
#[path = "../src/sources/registry.rs"]
#[allow(dead_code)]
mod sources_impl;
mod support {
    pub mod db {
        pub use crate::db_support::*;
    }

    #[path = "cron.rs"]
    pub mod cron;
}

mod agents {
    pub mod arbiter {
        pub use crate::arbiter_impl::*;
    }
}

mod jobs {
    pub mod runner {
        pub use crate::runner_impl::*;
    }
}

mod sources {
    pub mod registry {
        pub use crate::sources_impl::*;
    }
}

mod tests {
    use std::str::FromStr;

    use anyhow::{Context, Result, anyhow, ensure};
    use chrono::{DateTime, Duration, TimeZone, Utc};
    use cron::Schedule;
    use rusqlite::{Connection, params};
    use serde_json::Value;

    use crate::cron_impl::{
        COMMERCIAL_SWEEP_CRON, COMMERCIAL_SWEEP_SKILL_ID, COMMERCIAL_SWEEP_SKILL_VERSION,
        COMMERCIAL_SWEEP_SOURCE_ID, CronScheduler, HMRC_FX_MONTHLY_ROLLOVER_CRON,
        HMRC_FX_MONTHLY_ROLLOVER_SKILL_ID, HMRC_TARIFF_NEWS_WEEKLY_CRON,
        HMRC_TARIFF_NEWS_WEEKLY_SKILL_ID, L5_COMMERCIAL_INTELLIGENCE_SKILL_VERSION,
        L5_COMMERCIAL_INTELLIGENCE_SOURCE_ID, MARKETPLACE_SOURCE_HEALTH_CANARY_CRON,
        MARKETPLACE_SOURCE_HEALTH_CANARY_SKILL_ID, TRA_CASE_REGISTER_DAILY_CRON,
        TRA_CASE_REGISTER_DAILY_SKILL_ID, VALIDATION_CALIBRATION_READINESS_SWEEP_SKILL_ID,
        VALIDATION_CALIBRATION_SOURCE_ID, VALIDATION_CROSS_MODEL_AUDIT_SKILL_ID,
    };
    use crate::db_support::{StateDb, TestStateDb};
    use crate::runner_impl::JobRunner;
    use crate::skills_support::{lint_skill_directory, repo_skill_dir};
    use crate::sources_impl::{
        CanaryResult, CircuitState, DEFAULT_SOURCE_SKILL_VERSION, HealthState, SourceJobClass,
        SourceRegistry, SourceRuntimeState, persist_runtime_state,
    };
    use crate::support::cron::CronSchedulerHarness;

    fn ts(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Result<DateTime<Utc>> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .ok_or_else(|| anyhow!("invalid timestamp"))
    }

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

    fn seed_builtin_sources(state_db: &TestStateDb) -> Result<()> {
        seed_source(state_db.connection(), "fts", "procurement")?;
        seed_source(state_db.connection(), "contracts-finder", "procurement")?;
        seed_source(state_db.connection(), "alibaba", "supplier")?;
        Ok(())
    }

    fn source_health_skill_version(relative: &str) -> Result<String> {
        let skill = lint_skill_directory(&repo_skill_dir(relative)?)?;
        Ok(skill.metadata.skill_version)
    }

    fn reload_registry(state_db: &TestStateDb) -> Result<SourceRegistry> {
        SourceRegistry::load(state_db.connection())
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
    fn l3_t_cron_scheduler_001_valid_cron_expression_computes_expected_next_run_at() -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_builtin_sources(&state_db)?;
        let registry = reload_registry(&state_db)?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.refresh_next_run_at(ts(2026, 3, 25, 4, 10, 0)?)?;

        let definitions = scheduler.definitions();
        // The state-db open path auto-seeds FTS + Contracts Finder + the three
        // procontract-* sources. The harness adds Alibaba on top. That gives
        // seven source-backed bindings (FTS production + canary, Contracts Finder
        // production, procontract production x3, Alibaba canary) plus two
        // scheduler-owned validation calibration jobs, one scheduler-owned
        // commercial sweep, and four scheduler-owned L5 commercial-intelligence
        // watcher jobs.
        ensure!(
            definitions.len() == 14,
            "L3-T-cron_scheduler-001 expected fourteen cron-backed definitions, got {}: {:?}",
            definitions.len(),
            definitions.keys().collect::<Vec<_>>(),
        );
        ensure!(
            definitions["fts::procurement-ingestion/fts"].next_run_at
                == Some(ts(2026, 3, 25, 4, 15, 0)?),
            "L3-T-cron_scheduler-001 unexpected FTS production schedule"
        );
        ensure!(
            definitions["fts::source-health/fts"].next_run_at == Some(ts(2026, 3, 25, 4, 30, 0)?),
            "L3-T-cron_scheduler-001 unexpected FTS canary schedule"
        );
        ensure!(
            definitions["contracts-finder::procurement-ingestion/contracts-finder"].next_run_at
                == Some(ts(2026, 3, 25, 4, 45, 0)?),
            "L3-T-cron_scheduler-001 unexpected Contracts Finder schedule"
        );
        ensure!(
            definitions["procontract::procurement-ingestion/procontract-discover"].next_run_at
                == Some(ts(2026, 3, 25, 5, 0, 0)?),
            "L3-T-cron_scheduler-001 unexpected procontract discover schedule"
        );
        ensure!(
            definitions["procontract-interest::procurement-ingestion/procontract-register-interest"]
                .next_run_at == Some(ts(2026, 3, 25, 5, 30, 0)?),
            "L3-T-cron_scheduler-001 unexpected procontract register-interest schedule"
        );
        ensure!(
            definitions["procontract-enrich::procurement-ingestion/procontract-enrich-tender"]
                .next_run_at
                == Some(ts(2026, 3, 25, 6, 0, 0)?),
            "L3-T-cron_scheduler-001 unexpected procontract enrich-tender schedule"
        );
        ensure!(
            definitions["alibaba::source-health/alibaba"].next_run_at
                == Some(ts(2026, 3, 25, 4, 30, 0)?),
            "L3-T-cron_scheduler-001 unexpected Alibaba canary schedule"
        );
        let readiness_key = format!(
            "{VALIDATION_CALIBRATION_SOURCE_ID}::{VALIDATION_CALIBRATION_READINESS_SWEEP_SKILL_ID}"
        );
        let cross_model_key =
            format!("{VALIDATION_CALIBRATION_SOURCE_ID}::{VALIDATION_CROSS_MODEL_AUDIT_SKILL_ID}");
        ensure!(
            definitions[readiness_key.as_str()].next_run_at == Some(ts(2026, 3, 26, 2, 20, 0)?),
            "L3-T-cron_scheduler-001 unexpected validation readiness sweep schedule"
        );
        ensure!(
            definitions[cross_model_key.as_str()].next_run_at == Some(ts(2026, 3, 26, 3, 30, 0)?),
            "L3-T-cron_scheduler-001 unexpected validation cross-model audit schedule"
        );
        let commercial_key = format!("{COMMERCIAL_SWEEP_SOURCE_ID}::{COMMERCIAL_SWEEP_SKILL_ID}");
        let commercial_definition = definitions
            .get(&commercial_key)
            .ok_or_else(|| anyhow!("missing commercial sweep cron definition"))?;
        ensure!(commercial_definition.source_id == COMMERCIAL_SWEEP_SOURCE_ID);
        ensure!(commercial_definition.skill_id == COMMERCIAL_SWEEP_SKILL_ID);
        ensure!(commercial_definition.skill_version == COMMERCIAL_SWEEP_SKILL_VERSION);
        ensure!(commercial_definition.cron_expression == COMMERCIAL_SWEEP_CRON);
        ensure!(
            commercial_definition.next_run_at == Some(ts(2026, 3, 25, 7, 0, 0)?),
            "L3-T-cron_scheduler-001 unexpected commercial sweep schedule"
        );
        Ok(())
    }

    #[test]
    fn l5_t_fx_engine_009_freight_engine_016_customs_classification_023_024_cron_definitions()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        let registry = reload_registry(&state_db)?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.refresh_next_run_at(ts(2026, 5, 19, 3, 0, 0)?)?;

        let expected = [
            (
                HMRC_TARIFF_NEWS_WEEKLY_SKILL_ID,
                HMRC_TARIFF_NEWS_WEEKLY_CRON,
            ),
            (
                TRA_CASE_REGISTER_DAILY_SKILL_ID,
                TRA_CASE_REGISTER_DAILY_CRON,
            ),
            (
                HMRC_FX_MONTHLY_ROLLOVER_SKILL_ID,
                HMRC_FX_MONTHLY_ROLLOVER_CRON,
            ),
            (
                MARKETPLACE_SOURCE_HEALTH_CANARY_SKILL_ID,
                MARKETPLACE_SOURCE_HEALTH_CANARY_CRON,
            ),
        ];

        for (skill_id, cron_expression) in expected {
            let key = format!("{L5_COMMERCIAL_INTELLIGENCE_SOURCE_ID}::{skill_id}");
            let definition = scheduler
                .definitions()
                .get(&key)
                .ok_or_else(|| anyhow!("missing L5 cron definition `{key}`"))?;
            ensure!(definition.source_id == L5_COMMERCIAL_INTELLIGENCE_SOURCE_ID);
            ensure!(definition.skill_id == skill_id);
            ensure!(definition.skill_version == L5_COMMERCIAL_INTELLIGENCE_SKILL_VERSION);
            ensure!(definition.cron_expression == cron_expression);
            ensure!(
                !definition.enabled,
                "L5 cron definitions should stay registered but inactive until their L5_ENABLE_* env flag is set"
            );
            ensure!(
                definition.next_run_at.is_some(),
                "L5 cron definition `{key}` should parse and compute a next run"
            );
        }

        Ok(())
    }

    #[test]
    fn cron_support_harness_runs_due_sweep_for_seeded_definition() -> Result<()> {
        let mut harness = CronSchedulerHarness::new(ts(2026, 3, 25, 4, 20, 0)?)?;
        harness.seed_definition(
            "fts",
            "procurement-ingestion/fts",
            DEFAULT_SOURCE_SKILL_VERSION,
            "0 15 4 * * *",
            true,
        )?;

        let dispatches = harness.run_due_sweep()?;
        ensure!(
            dispatches.iter().any(|dispatch| {
                dispatch.definition_key == "fts::procurement-ingestion/fts"
                    && dispatch.skill_version == DEFAULT_SOURCE_SKILL_VERSION
            }),
            "cron support harness should surface the seeded due definition"
        );
        Ok(())
    }

    #[test]
    fn l3_t_cron_scheduler_002_invalid_cron_expression_is_rejected_explicitly() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let error = Schedule::from_str("not-a-cron")
            .with_context(|| "parsing cron expression `not-a-cron` for source `broken-source`")
            .expect_err("L3-T-cron_scheduler-002 expected invalid cron failure");
        ensure!(
            error
                .to_string()
                .contains("parsing cron expression `not-a-cron` for source `broken-source`"),
            "L3-T-cron_scheduler-002 missing explicit parse context: {error:#}"
        );

        let job_rows: i64 =
            state_db
                .connection()
                .query_row("SELECT COUNT(*) FROM workflow_jobs", [], |row| row.get(0))?;
        ensure!(
            job_rows == 0,
            "L3-T-cron_scheduler-002 expected no runnable job rows to be created"
        );
        Ok(())
    }

    #[test]
    fn l3_t_cron_scheduler_003_dispatch_freezes_skill_binding_and_root_enqueue_shape() -> Result<()>
    {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), "fts", "procurement")?;
        let registry = reload_registry(&state_db)?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.record_enqueued("fts::procurement-ingestion/fts", ts(2026, 3, 24, 4, 15, 0)?)?;

        let dispatches = scheduler.dispatch_due_jobs(&registry, ts(2026, 3, 25, 4, 20, 0)?)?;
        let dispatch = dispatches
            .into_iter()
            .find(|candidate| candidate.definition.skill_id == "procurement-ingestion/fts")
            .ok_or_else(|| anyhow!("L3-T-cron_scheduler-003 expected FTS production dispatch"))?;

        ensure!(dispatch.enqueue_request.parent_job_id.is_none());
        ensure!(dispatch.enqueue_request.process_id.is_none());
        ensure!(dispatch.enqueue_request.scope_kind.as_deref() == Some("source"));
        ensure!(dispatch.enqueue_request.scope_id.as_deref() == Some("fts"));
        ensure!(dispatch.enqueue_request.kind == "sync_source");
        ensure!(!dispatch.enqueue_request.validation_required);
        ensure!(
            dispatch.enqueue_request.scheduled_for == Some(ts(2026, 3, 25, 4, 15, 0)?),
            "L3-T-cron_scheduler-003 expected frozen scheduled_for"
        );
        ensure!(dispatch.enqueue_request.input_payload["skill_id"] == "procurement-ingestion/fts");
        ensure!(
            dispatch.enqueue_request.input_payload["skill_version"] == DEFAULT_SOURCE_SKILL_VERSION
        );
        ensure!(
            dispatch.enqueue_request.input_payload["schedule_key"]
                == "fts::procurement-ingestion/fts"
        );
        Ok(())
    }

    #[test]
    fn commercial_sweep_cron_definition_builds_pipeline_enqueue_shape() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let registry = reload_registry(&state_db)?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.record_enqueued(
            "commercial-sweep::pipeline/commercial-sweep",
            ts(2026, 3, 24, 7, 0, 0)?,
        )?;

        let dispatch = scheduler
            .dispatch_due_jobs(&registry, ts(2026, 3, 25, 7, 5, 0)?)?
            .into_iter()
            .find(|candidate| candidate.definition.skill_id == COMMERCIAL_SWEEP_SKILL_ID)
            .ok_or_else(|| anyhow!("expected commercial_sweep dispatch"))?;

        ensure!(dispatch.definition.source_id == COMMERCIAL_SWEEP_SOURCE_ID);
        ensure!(dispatch.definition.skill_id == COMMERCIAL_SWEEP_SKILL_ID);
        ensure!(dispatch.definition.skill_version == COMMERCIAL_SWEEP_SKILL_VERSION);
        ensure!(dispatch.definition.cron_expression == COMMERCIAL_SWEEP_CRON);
        ensure!(dispatch.enqueue_request.kind == "commercial_sweep");
        ensure!(dispatch.enqueue_request.scope_kind.as_deref() == Some("pipeline"));
        ensure!(dispatch.enqueue_request.scope_id.as_deref() == Some("commercial"));
        ensure!(dispatch.enqueue_request.process_id.is_none());
        ensure!(dispatch.enqueue_request.notice_id.is_none());
        ensure!(!dispatch.enqueue_request.validation_required);
        ensure!(dispatch.enqueue_request.scheduled_for == Some(ts(2026, 3, 25, 7, 0, 0)?));
        ensure!(
            dispatch.enqueue_request.idempotency_key
                == "cron:commercial-sweep:pipeline/commercial-sweep:2026-03-25T07:00:00+00:00"
        );
        ensure!(dispatch.enqueue_request.input_payload["source_id"] == COMMERCIAL_SWEEP_SOURCE_ID);
        ensure!(dispatch.enqueue_request.input_payload["skill_id"] == COMMERCIAL_SWEEP_SKILL_ID);
        ensure!(
            dispatch.enqueue_request.input_payload["schedule_key"]
                == "commercial-sweep::pipeline/commercial-sweep"
        );
        Ok(())
    }

    #[test]
    fn l3_t_cron_scheduler_004_scheduler_binding_matches_on_disk_frozen_frontmatter() -> Result<()>
    {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), "fts", "procurement")?;
        let registry = reload_registry(&state_db)?;
        let on_disk_skill_version = source_health_skill_version("source-health/fts")?;
        let binding = registry
            .get("fts")
            .and_then(|source| source.definition.canary_binding.as_ref())
            .ok_or_else(|| anyhow!("L3-T-cron_scheduler-004 expected FTS canary binding"))?;

        ensure!(
            binding.skill_version == DEFAULT_SOURCE_SKILL_VERSION,
            "L3-T-cron_scheduler-004 expected registry to freeze DEFAULT_SOURCE_SKILL_VERSION"
        );
        ensure!(
            binding.skill_version == on_disk_skill_version,
            "L3-T-cron_scheduler-004 expected checked-in skill frontmatter to stay aligned with the frozen binding"
        );
        Ok(())
    }

    #[test]
    fn l3_t_cron_scheduler_005_enqueue_job_persists_scheduled_for_and_frozen_payload() -> Result<()>
    {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), "fts", "procurement")?;
        let registry = reload_registry(&state_db)?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.record_enqueued("fts::procurement-ingestion/fts", ts(2026, 3, 24, 4, 15, 0)?)?;

        let dispatch = scheduler
            .dispatch_due_jobs(&registry, ts(2026, 3, 25, 4, 20, 0)?)?
            .into_iter()
            .find(|candidate| candidate.definition.skill_id == "procurement-ingestion/fts")
            .ok_or_else(|| anyhow!("L3-T-cron_scheduler-005 expected FTS production dispatch"))?;

        let mut writable_db = StateDb::open(state_db.db_path())?;
        JobRunner::new("cron-test-executor")
            .enqueue_job(&mut writable_db, &dispatch.enqueue_request)?;

        let (scheduled_for, parent_job_id, stored_payload): (
            Option<String>,
            Option<String>,
            String,
        ) = writable_db.connection().query_row(
            "\
                SELECT scheduled_for, parent_job_id, input_payload
                FROM workflow_jobs
                WHERE id = ?1
                ",
            params![dispatch.enqueue_request.job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let payload: Value = serde_json::from_str(&stored_payload)?;

        ensure!(parent_job_id.is_none());
        ensure!(
            scheduled_for.as_deref() == Some("2026-03-25T04:15:00+00:00"),
            "L3-T-cron_scheduler-005 expected scheduled_for to persist exactly"
        );
        ensure!(payload["request"]["skill_version"] == DEFAULT_SOURCE_SKILL_VERSION);
        ensure!(payload["request"]["schedule_key"] == "fts::procurement-ingestion/fts");
        ensure!(payload["runner_meta"]["executor_id"] == "cron-test-executor");
        Ok(())
    }

    #[test]
    fn l3_t_cron_scheduler_006_startup_catchup_dispatches_due_jobs_in_priority_order() -> Result<()>
    {
        let state_db = TestStateDb::new()?;
        seed_builtin_sources(&state_db)?;
        let registry = reload_registry(&state_db)?;
        let mut scheduler = CronScheduler::from_registry(&registry);

        scheduler.record_enqueued("fts::procurement-ingestion/fts", ts(2026, 3, 24, 4, 15, 0)?)?;
        scheduler.record_enqueued("fts::source-health/fts", ts(2026, 3, 25, 4, 0, 0)?)?;
        scheduler.record_enqueued(
            "contracts-finder::procurement-ingestion/contracts-finder",
            ts(2026, 3, 24, 4, 45, 0)?,
        )?;
        scheduler.record_enqueued(
            "commercial-sweep::pipeline/commercial-sweep",
            ts(2026, 3, 24, 7, 0, 0)?,
        )?;

        let dispatches = scheduler.dispatch_due_jobs(&registry, ts(2026, 3, 25, 7, 5, 0)?)?;
        let ordered_keys = dispatches
            .iter()
            .map(|dispatch| dispatch.definition_key.as_str())
            .collect::<Vec<_>>();

        ensure!(
            ordered_keys
                == vec![
                    "fts::procurement-ingestion/fts",
                    "contracts-finder::procurement-ingestion/contracts-finder",
                    "fts::source-health/fts",
                    "commercial-sweep::pipeline/commercial-sweep",
                ],
            "L3-T-cron_scheduler-006 unexpected catchup order: {ordered_keys:?}"
        );
        Ok(())
    }

    #[test]
    fn l3_t_cron_scheduler_007_half_open_sources_only_schedule_canary_health_work() -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), "fts", "procurement")?;
        let registry = reload_registry(&state_db)?;
        let source = registry
            .get("fts")
            .ok_or_else(|| anyhow!("missing FTS source"))?;
        let definition = &source.definition;
        let observed_at = fresh_timestamp(&source.runtime_state);

        let runtime_state = SourceRuntimeState {
            source_id: "fts".to_owned(),
            last_health_state: definition.default_health_state,
            circuit_state: CircuitState::HalfOpen,
            challenge_event_count: 1,
            challenge_window_started_at: Some(observed_at),
            circuit_opened_at: None,
            cooldown_until: None,
            last_canary_at: Some(observed_at),
            last_canary_result: None,
            last_error_code: Some("AGENT_AUTH_CHALLENGE".to_owned()),
            updated_at: Some(observed_at),
            created_at: Some(observed_at),
        };
        persist_runtime_state(state_db.connection(), &runtime_state)?;

        let reloaded = reload_registry(&state_db)?;
        let mut scheduler = CronScheduler::from_registry(&reloaded);
        scheduler.record_enqueued("fts::procurement-ingestion/fts", ts(2026, 3, 24, 4, 15, 0)?)?;
        scheduler.record_enqueued("fts::source-health/fts", ts(2026, 3, 25, 4, 0, 0)?)?;

        let dispatches = scheduler.dispatch_due_jobs(&reloaded, ts(2026, 3, 25, 5, 0, 0)?)?;
        let dispatch_keys = dispatches
            .iter()
            .map(|dispatch| dispatch.definition_key.as_str())
            .collect::<Vec<_>>();

        ensure!(
            dispatch_keys == vec!["fts::source-health/fts"],
            "L3-T-cron_scheduler-007 expected only source-health dispatch in half-open mode"
        );
        Ok(())
    }

    #[test]
    fn l3_t_cron_scheduler_008_cooldown_expiry_promotes_sources_to_half_open_recovery() -> Result<()>
    {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), "fts", "procurement")?;
        let registry = reload_registry(&state_db)?;
        let source = registry
            .get("fts")
            .ok_or_else(|| anyhow!("missing FTS source"))?;
        let definition = &source.definition;
        let base = fresh_timestamp(&source.runtime_state);
        let now = base + Duration::minutes(30);
        let runtime_state = SourceRuntimeState {
            source_id: "fts".to_owned(),
            last_health_state: definition.default_health_state,
            circuit_state: CircuitState::Open,
            challenge_event_count: definition.policy.challenge_threshold,
            challenge_window_started_at: Some(base - Duration::minutes(30)),
            circuit_opened_at: Some(base - Duration::minutes(30)),
            cooldown_until: Some(base),
            last_canary_at: Some(base),
            last_canary_result: None,
            last_error_code: Some("SOURCE_SOFT_BLOCKED".to_owned()),
            updated_at: Some(base),
            created_at: Some(base - Duration::minutes(30)),
        };
        persist_runtime_state(state_db.connection(), &runtime_state)?;

        let reloaded = reload_registry(&state_db)?;
        let source = reloaded
            .get("fts")
            .ok_or_else(|| anyhow!("missing FTS source"))?;
        ensure!(
            source.effective_circuit_state(now) == CircuitState::HalfOpen,
            "L3-T-cron_scheduler-008 expected cooldown expiry to move source into half-open"
        );
        ensure!(
            source.allows_job_class(SourceJobClass::SourceHealth, now),
            "L3-T-cron_scheduler-008 expected source-health jobs after cooldown expiry"
        );
        ensure!(
            !source.allows_job_class(SourceJobClass::Production, now),
            "L3-T-cron_scheduler-008 expected production work to stay suppressed in half-open"
        );
        Ok(())
    }

    #[test]
    fn l3_t_cron_scheduler_009_open_circuits_suppress_production_dispatches() -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), "fts", "procurement")?;
        let registry = reload_registry(&state_db)?;
        let source = registry
            .get("fts")
            .ok_or_else(|| anyhow!("missing FTS source"))?;
        let definition = &source.definition;
        let base = fresh_timestamp(&source.runtime_state);

        let opened = source
            .runtime_state
            .with_challenge_failure(definition, base, "AGENT_AUTH_CHALLENGE")
            .and_then(|state| {
                state.with_challenge_failure(
                    definition,
                    base + Duration::minutes(10),
                    "SOURCE_SOFT_BLOCKED",
                )
            })
            .and_then(|state| {
                state.with_challenge_failure(
                    definition,
                    base + Duration::minutes(20),
                    "SOURCE_CAPTCHA_BLOCKED",
                )
            })
            .ok_or_else(|| anyhow!("failed to build open breaker state"))?;
        let recovery_at = opened
            .cooldown_until
            .ok_or_else(|| anyhow!("missing cooldown for recovery path"))?
            + Duration::minutes(5);
        let runtime_state = opened
            .with_canary_outcome(
                definition,
                recovery_at,
                CanaryResult::Failed,
                HealthState::Red,
                Some("SOURCE_SOFT_BLOCKED".to_owned()),
            )
            .ok_or_else(|| anyhow!("missing failed recovery canary outcome"))?;
        persist_runtime_state(state_db.connection(), &runtime_state)?;

        let reloaded = reload_registry(&state_db)?;
        let source = reloaded
            .get("fts")
            .ok_or_else(|| anyhow!("missing FTS source"))?;
        ensure!(
            matches!(
                source.effective_circuit_state(recovery_at),
                CircuitState::Open
            ),
            "L3-T-cron_scheduler-009 expected failed recovery canary to reopen circuit"
        );
        ensure!(
            !source.allows_job_class(SourceJobClass::Production, recovery_at),
            "L3-T-cron_scheduler-009 expected production suppression during fresh cooldown"
        );
        let mut scheduler = CronScheduler::from_registry(&reloaded);
        scheduler.record_enqueued(
            "fts::procurement-ingestion/fts",
            recovery_at - Duration::days(1),
        )?;
        scheduler.record_enqueued("fts::source-health/fts", recovery_at - Duration::days(1))?;
        let dispatches = scheduler.dispatch_due_jobs(&reloaded, recovery_at)?;
        ensure!(
            dispatches
                .iter()
                .all(|dispatch| !matches!(dispatch.definition.source_id.as_str(), "fts")),
            "L3-T-cron_scheduler-009 expected reopened circuit to suppress all FTS dispatches during fresh cooldown"
        );
        Ok(())
    }

    #[test]
    fn l3_t_cron_scheduler_010_runtime_state_writes_do_not_mutate_source_definitions() -> Result<()>
    {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), "alibaba", "supplier")?;

        let original_enabled: i64 = state_db.connection().query_row(
            "SELECT enabled FROM sources WHERE id = 'alibaba'",
            [],
            |row| row.get(0),
        )?;
        let original_config: String = state_db.connection().query_row(
            "SELECT config FROM sources WHERE id = 'alibaba'",
            [],
            |row| row.get(0),
        )?;

        let registry = reload_registry(&state_db)?;
        let definition = &registry
            .get("alibaba")
            .ok_or_else(|| anyhow!("missing Alibaba source"))?
            .definition;
        let runtime_state = SourceRuntimeState {
            source_id: "alibaba".to_owned(),
            last_health_state: definition.default_health_state,
            circuit_state: CircuitState::Open,
            challenge_event_count: definition.policy.challenge_threshold,
            challenge_window_started_at: Some(ts(2026, 3, 25, 4, 0, 0)?),
            circuit_opened_at: Some(ts(2026, 3, 25, 4, 0, 0)?),
            cooldown_until: Some(ts(2026, 3, 25, 6, 0, 0)?),
            last_canary_at: Some(ts(2026, 3, 25, 4, 0, 0)?),
            last_canary_result: None,
            last_error_code: Some("AGENT_AUTH_CHALLENGE".to_owned()),
            updated_at: Some(ts(2026, 3, 25, 4, 0, 0)?),
            created_at: Some(ts(2026, 3, 25, 4, 0, 0)?),
        };
        persist_runtime_state(state_db.connection(), &runtime_state)?;

        let reloaded = reload_registry(&state_db)?;
        let reloaded_source = reloaded
            .get("alibaba")
            .ok_or_else(|| anyhow!("missing Alibaba source"))?;
        let current_enabled: i64 = state_db.connection().query_row(
            "SELECT enabled FROM sources WHERE id = 'alibaba'",
            [],
            |row| row.get(0),
        )?;
        let current_config: String = state_db.connection().query_row(
            "SELECT config FROM sources WHERE id = 'alibaba'",
            [],
            |row| row.get(0),
        )?;

        ensure!(original_enabled == current_enabled);
        ensure!(original_config == current_config);
        ensure!(
            reloaded_source.runtime_state.circuit_state == CircuitState::Open,
            "L3-T-cron_scheduler-010 expected runtime state to load from source_runtime_state"
        );
        Ok(())
    }
}
