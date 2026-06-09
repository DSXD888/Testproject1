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
#[path = "../src/sources/registry.rs"]
#[allow(dead_code)]
mod sources_impl;
#[path = "../src/scheduler/tick.rs"]
#[allow(dead_code)]
mod tick_impl;

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

mod scheduler {
    pub mod cron {
        pub use crate::cron_impl::*;
    }
}

mod sources {
    pub mod registry {
        pub use crate::sources_impl::*;
    }
}

mod support {
    pub mod db {
        pub use crate::db_support::*;
    }
}

mod tests {
    use anyhow::{Context, Result, anyhow, ensure};
    use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};
    use rusqlite::Connection;
    use rusqlite::params;
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    use crate::cron_impl::CronScheduler;
    use crate::db_support::{StateDb, TestStateDb};
    use crate::runner_impl::{EnqueueJobRequest, JobRunner};
    use crate::sources_impl::{SourceRegistry, set_source_enabled, source_registry_generation};
    use crate::tick_impl::run_scheduler_tick;

    struct SchedulerOneShotHarness {
        _temp: tempfile::TempDir,
        state_root: PathBuf,
        runtime_root: PathBuf,
        artifact_root: PathBuf,
        fixture_path: PathBuf,
        db_path: PathBuf,
    }

    struct SchedulerOneShotResult {
        stdout: String,
        stderr: String,
        success: bool,
    }

    struct FakeClaudeCli {
        program: PathBuf,
        result_path: PathBuf,
        checkpoint_path: PathBuf,
    }

    impl SchedulerOneShotHarness {
        fn new() -> Result<Self> {
            let temp = tempfile::tempdir()?;
            let root = temp.path().to_path_buf();
            let state_root = root.join("state");
            let runtime_root = root.join("runtime");
            let artifact_root = root.join("artifacts");
            fs::create_dir_all(&state_root)?;
            fs::create_dir_all(&runtime_root)?;
            fs::create_dir_all(&artifact_root)?;
            Ok(Self {
                _temp: temp,
                db_path: state_root.join("state.db"),
                fixture_path: root.join("fixture.json"),
                state_root,
                runtime_root,
                artifact_root,
            })
        }

        fn run_scheduler_once(&self, tick_now: &str) -> Result<SchedulerOneShotResult> {
            let fixture = default_fixture();
            self.run_scheduler_once_with_fixture(tick_now, &fixture)
        }

        fn run_scheduler_once_with_fixture(
            &self,
            tick_now: &str,
            fixture: &Value,
        ) -> Result<SchedulerOneShotResult> {
            fs::write(&self.fixture_path, serde_json::to_vec_pretty(&fixture)?)
                .with_context(|| format!("writing {}", self.fixture_path.display()))?;
            let fake_claude = self.prepare_fake_claude(fixture)?;

            let output = Command::new(env!("CARGO_BIN_EXE_daemon"))
                .arg("--run-scheduler-once")
                .arg("--tick-now")
                .arg(tick_now)
                .arg("--agent-cli")
                .arg("claude")
                .arg("--agent-cli-program")
                .arg(&fake_claude.program)
                .arg("--state-root")
                .arg(&self.state_root)
                .arg("--runtime-dir")
                .arg(&self.runtime_root)
                .arg("--artifact-root")
                .arg(&self.artifact_root)
                .arg("--fixture-path")
                .arg(&self.fixture_path)
                .env("DAEMON_TEST_CLAUDE_RESULT_PATH", &fake_claude.result_path)
                .env(
                    "DAEMON_TEST_CLAUDE_CHECKPOINT_PATH",
                    &fake_claude.checkpoint_path,
                )
                .env_remove("SUPABASE_URL")
                .env_remove("NEXT_PUBLIC_SUPABASE_URL")
                .env_remove("SUPABASE_SERVICE_ROLE_KEY")
                .env_remove("SUPABASE_SERVICE_KEY")
                .output()
                .context("running daemon scheduler one-shot")?;

            Ok(SchedulerOneShotResult {
                stdout: String::from_utf8(output.stdout).context("decoding stdout")?,
                stderr: String::from_utf8(output.stderr).context("decoding stderr")?,
                success: output.status.success(),
            })
        }

        fn connection(&self) -> Result<Connection> {
            Connection::open(&self.db_path)
                .with_context(|| format!("opening {}", self.db_path.display()))
        }

        fn seed_completed_fts_source_health_slot(
            &self,
            scheduled_for: DateTime<Utc>,
        ) -> Result<()> {
            let mut state_db = StateDb::open(&self.db_path)?;
            let job_id = "job-seeded-fts-source-health".to_owned();
            JobRunner::new("scheduler-one-shot-history").enqueue_job(
                &mut state_db,
                &EnqueueJobRequest {
                    job_id: job_id.clone(),
                    kind: "source_health".to_owned(),
                    process_id: None,
                    notice_id: None,
                    scope_kind: Some("source".to_owned()),
                    scope_id: Some("fts".to_owned()),
                    parent_job_id: None,
                    continuation_chain_id: None,
                    validation_required: false,
                    scheduled_for: Some(scheduled_for),
                    input_payload: json!({
                        "source_id": "fts",
                        "skill_id": "source-health/fts",
                        "skill_version": "1.0.0",
                        "cron_expression": "0 */30 * * * *",
                        "scheduled_for": scheduled_for.to_rfc3339(),
                        "schedule_key": "fts::source-health/fts",
                    }),
                    idempotency_key: format!(
                        "cron:fts:source-health/fts:{}",
                        scheduled_for.to_rfc3339()
                    ),
                },
            )?;
            state_db.connection().execute(
                "\
                UPDATE workflow_jobs
                SET status = 'completed',
                    completed_at = ?1
                WHERE id = ?2
                ",
                params![Utc::now().to_rfc3339(), job_id],
            )?;
            Ok(())
        }

        fn seed_commercial_stage11_graph(&self) -> Result<()> {
            let state_db = StateDb::open(&self.db_path)?;
            seed_stage11_minimal_graph(state_db.connection(), "GB")
        }

        fn prepare_fake_claude(&self, fixture: &Value) -> Result<FakeClaudeCli> {
            let root = self
                .state_root
                .parent()
                .ok_or_else(|| anyhow!("state root missing parent"))?;
            let state_dir = root.join("fake-claude");
            fs::create_dir_all(&state_dir)?;
            let probe_json = state_dir.join("probe.json");
            fs::write(
                &probe_json,
                serde_json::to_vec_pretty(&json!({
                    "auth_healthy": true,
                    "structured_output": true,
                    "pty_viable": true,
                    "browser_tools": true
                }))?,
            )?;
            let result_source = state_dir.join("result-source.json");
            fs::write(
                &result_source,
                serde_json::to_vec_pretty(&manifest_result_payload(fixture)?)?,
            )?;
            let program = state_dir.join("claude");
            fs::write(
                &program,
                format!(
                    "#!/bin/sh\nset -eu\nstate_dir=\"{}\"\nif [ \"${{1-}}\" = \"--version\" ] || [ \"${{1-}}\" = \"-v\" ]; then\n  printf 'claude 1.2.3\\n'\n  exit 0\nfi\nif [ \"${{1-}}\" = \"probe\" ]; then\n  cat \"$state_dir/probe.json\"\n  exit 0\nfi\nmode=pty\nfor arg in \"$@\"; do\n  if [ \"$arg\" = \"--output-format\" ]; then\n    mode=contract\n    break\n  fi\ndone\nif [ \"$mode\" = contract ]; then\n  cp \"$state_dir/result-source.json\" \"$DAEMON_TEST_CLAUDE_RESULT_PATH\"\n  printf '{{\"event\":\"result\",\"path\":\"%s\"}}\\n' \"$DAEMON_TEST_CLAUDE_RESULT_PATH\"\nelse\n  : > \"$DAEMON_TEST_CLAUDE_CHECKPOINT_PATH\"\n  printf '{{\"event\":\"checkpoint\",\"seq\":1,\"path\":\"%s\"}}\\n' \"$DAEMON_TEST_CLAUDE_CHECKPOINT_PATH\"\nfi\n",
                    state_dir.display()
                ),
            )?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&program)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&program, perms)?;
            }
            Ok(FakeClaudeCli {
                program,
                result_path: self.artifact_root.join("fake-claude-result.json"),
                checkpoint_path: self.artifact_root.join("fake-claude-checkpoint.json"),
            })
        }
    }

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

    fn count_jobs(state_db: &TestStateDb) -> Result<i64> {
        Ok(state_db
            .connection()
            .query_row("SELECT COUNT(*) FROM workflow_jobs", [], |row| row.get(0))?)
    }

    fn set_fts_source_enabled(conn: &Connection, enabled: bool) -> Result<bool> {
        set_source_enabled(conn, "fts", enabled)
    }

    fn count_fts_jobs_at(conn: &Connection, scheduled_for: &str) -> Result<i64> {
        Ok(conn.query_row(
            "\
            SELECT COUNT(*)
            FROM workflow_jobs
            WHERE kind = 'sync_source'
              AND scope_id = 'fts'
              AND scheduled_for = ?1
            ",
            params![scheduled_for],
            |row| row.get(0),
        )?)
    }

    fn count_fts_jobs_at_status(
        conn: &Connection,
        scheduled_for: &str,
        status: &str,
    ) -> Result<i64> {
        Ok(conn.query_row(
            "\
            SELECT COUNT(*)
            FROM workflow_jobs
            WHERE kind = 'sync_source'
              AND scope_id = 'fts'
              AND scheduled_for = ?1
              AND status = ?2
            ",
            params![scheduled_for, status],
            |row| row.get(0),
        )?)
    }

    fn fts_schedule_state(scheduler: &CronScheduler) -> Result<(Option<String>, Option<String>)> {
        let definition = scheduler
            .definitions()
            .get("fts::procurement-ingestion/fts")
            .ok_or_else(|| anyhow!("missing FTS cron definition"))?;
        Ok((
            definition.last_run_at.as_ref().map(DateTime::to_rfc3339),
            definition.next_run_at.as_ref().map(DateTime::to_rfc3339),
        ))
    }

    fn query_single_fts_job(
        conn: &Connection,
        scheduled_for: &str,
    ) -> Result<(String, String, String, String)> {
        Ok(conn.query_row(
            "\
            SELECT kind, scope_id, scheduled_for, input_payload
            FROM workflow_jobs
            WHERE kind = 'sync_source'
              AND scope_id = 'fts'
              AND scheduled_for = ?1
            ",
            params![scheduled_for],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?)
    }

    fn query_latest_root_job(conn: &Connection) -> Result<(String, String, String, String)> {
        Ok(conn.query_row(
            "\
            SELECT kind, status, scheduled_for, input_payload
            FROM workflow_jobs
            WHERE parent_job_id IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            ",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?)
    }

    fn count_root_jobs(conn: &Connection) -> Result<i64> {
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM workflow_jobs WHERE parent_job_id IS NULL",
            [],
            |row| row.get(0),
        )?)
    }

    fn count_materialized_fts_notice_rows(conn: &Connection) -> Result<i64> {
        Ok(conn.query_row(
            "\
            SELECT COUNT(*)
            FROM procurement_processes pp
            JOIN contract_notices cn
              ON cn.process_id = pp.id
             AND cn.id = pp.primary_notice_id
            JOIN raw_documents rd
              ON rd.id = cn.raw_document_id
            WHERE pp.id = 'process-fts-001'
              AND cn.id = 'notice-fts-001'
              AND cn.source_id = 'fts'
              AND cn.source_notice_id = 'ocds-001'
            ",
            [],
            |row| row.get(0),
        )?)
    }

    fn query_completed_commercial_sweep_job(
        conn: &Connection,
    ) -> Result<(String, String, String, String)> {
        Ok(conn.query_row(
            "\
            SELECT id, status, scheduled_for, output_payload
            FROM workflow_jobs
            WHERE kind = 'commercial_sweep'
            ORDER BY created_at DESC
            LIMIT 1
            ",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?)
    }

    fn count_commercial_sweep_jobs_at(conn: &Connection, scheduled_for: &str) -> Result<i64> {
        Ok(conn.query_row(
            "\
            SELECT COUNT(*)
            FROM workflow_jobs
            WHERE kind = 'commercial_sweep'
              AND scheduled_for = ?1
            ",
            params![scheduled_for],
            |row| row.get(0),
        )?)
    }

    fn completed_commercial_sweep_idempotency_key(
        conn: &Connection,
        scheduled_for: &str,
    ) -> Result<String> {
        let input_payload: String = conn.query_row(
            "\
            SELECT input_payload
            FROM workflow_jobs
            WHERE kind = 'commercial_sweep'
              AND scheduled_for = ?1
              AND status = 'completed'
            ORDER BY created_at DESC
            LIMIT 1
            ",
            params![scheduled_for],
            |row| row.get(0),
        )?;
        let payload: Value = serde_json::from_str(&input_payload)?;
        payload
            .pointer("/runner_meta/idempotency_key")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("commercial_sweep input_payload missing idempotency key"))
    }

    fn count_landed_cost_rows_for_stage(conn: &Connection, stage_run_id: &str) -> Result<i64> {
        Ok(conn.query_row(
            "\
            SELECT COUNT(*)
            FROM landed_cost_estimates
            WHERE process_id = 'process-stage11'
              AND product_candidate_id = 'candidate-stage11'
              AND supplier_quote_id = 'quote-stage11'
              AND stage_run_id = ?1
            ",
            params![stage_run_id],
            |row| row.get(0),
        )?)
    }

    fn net_margin_basis_hash_for_stage(conn: &Connection, stage_run_id: &str) -> Result<String> {
        Ok(conn.query_row(
            "\
            SELECT basis_hash
            FROM candidate_net_margin_assessments
            WHERE process_id = 'process-stage11'
              AND product_candidate_id = 'candidate-stage11'
              AND stage_run_id = ?1
            ",
            params![stage_run_id],
            |row| row.get(0),
        )?)
    }

    fn manifest_state_for_stage(conn: &Connection, stage_run_id: &str) -> Result<(String, String)> {
        Ok(conn.query_row(
            "\
            SELECT pa.pipeline_state, pa.decision_basis_hash
            FROM manifests m
            JOIN pipeline_assessments pa ON pa.process_id = m.process_id
            WHERE m.process_id = 'process-stage11'
              AND m.stage_run_id = ?1
            ",
            params![stage_run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?)
    }

    fn assert_m2_commercial_sweep_side_effects(conn: &Connection) -> Result<()> {
        let scheduled_for = "2026-06-05T07:00:00+00:00";
        ensure!(
            count_commercial_sweep_jobs_at(conn, scheduled_for)? == 1,
            "M2 expected exactly one commercial_sweep job at the 07:00 slot"
        );
        ensure!(
            completed_commercial_sweep_idempotency_key(conn, scheduled_for)?
                == "cron:commercial-sweep:pipeline/commercial-sweep:2026-06-05T07:00:00+00:00",
            "M2 expected the commercial sweep to use the shared per-slot cron dedup key"
        );

        let (_job_id, status, job_scheduled_for, output_payload) =
            query_completed_commercial_sweep_job(conn)?;
        ensure!(
            status == "completed",
            "M2 commercial_sweep did not complete"
        );
        ensure!(
            job_scheduled_for == scheduled_for,
            "M2 commercial_sweep scheduled_for drifted: {job_scheduled_for}"
        );
        let output_payload: Value = serde_json::from_str(&output_payload)?;
        let stage_run_id = output_payload
            .get("stage_run_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("M2 commercial_sweep output missing stage_run_id"))?;
        let net_margin_stage_run_id = output_payload
            .get("net_margin_stage_run_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("M2 commercial_sweep output missing net_margin_stage_run_id"))?;
        let manifest_stage_run_id = output_payload
            .get("manifest_stage_run_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("M2 commercial_sweep output missing manifest_stage_run_id"))?;

        ensure!(
            count_landed_cost_rows_for_stage(conn, stage_run_id)? > 0,
            "M2 expected landed_cost_estimates row for commercial sweep stage {stage_run_id}"
        );
        ensure!(
            !net_margin_basis_hash_for_stage(conn, net_margin_stage_run_id)?
                .trim()
                .is_empty(),
            "M2 expected non-empty net-margin basis_hash"
        );
        let (pipeline_state, decision_basis_hash) =
            manifest_state_for_stage(conn, manifest_stage_run_id)?;
        ensure!(
            pipeline_state == "manifest_ready",
            "M2 expected manifest_ready pipeline state, got {pipeline_state}"
        );
        ensure!(
            !decision_basis_hash.trim().is_empty(),
            "M2 expected non-empty pipeline decision_basis_hash"
        );
        Ok(())
    }

    fn assert_commercial_sweep_body_avoids_manual_rerun() -> Result<()> {
        let main_source = include_str!("../src/main.rs");
        let commercial_body_start = main_source
            .find("async fn run_commercial_sweep_workflow_job(")
            .ok_or_else(|| anyhow!("run_commercial_sweep_workflow_job missing"))?;
        let commercial_body = main_source
            .get(commercial_body_start..)
            .ok_or_else(|| anyhow!("commercial sweep source slice out of bounds"))?;
        let commercial_body_len = commercial_body
            .find("fn parse_tick_now(")
            .ok_or_else(|| anyhow!("commercial sweep source slice terminator missing"))?;
        let commercial_body = commercial_body
            .get(..commercial_body_len)
            .ok_or_else(|| anyhow!("commercial sweep body slice out of bounds"))?;
        for forbidden in [
            "trigger_pipeline_rerun",
            "stop_daemon_for_exclusive_pipeline_rerun",
            "PIPELINE_RERUN_INGEST_STAGES",
            "--ingest landed-cost",
            "--ingest net-gate",
            "--ingest manifest",
        ] {
            ensure!(
                !commercial_body.contains(forbidden),
                "M2 commercial_sweep body must not reuse manual rerun marker `{forbidden}`"
            );
        }
        Ok(())
    }

    fn enqueue_fts_history(db_path: &Path, scheduled_for: DateTime<Utc>) -> Result<()> {
        let mut state_db = StateDb::open(db_path)?;
        JobRunner::new("scheduler-tick-test").enqueue_job(
            &mut state_db,
            &EnqueueJobRequest {
                job_id: "job-scheduler-history".to_owned(),
                kind: "sync_source".to_owned(),
                process_id: None,
                notice_id: None,
                scope_kind: Some("source".to_owned()),
                scope_id: Some("fts".to_owned()),
                parent_job_id: None,
                continuation_chain_id: None,
                validation_required: false,
                scheduled_for: Some(scheduled_for),
                input_payload: json!({
                    "source_id": "fts",
                    "skill_id": "procurement-ingestion/fts",
                    "skill_version": "1.0.0",
                    "cron_expression": "0 15 4 * * *",
                    "scheduled_for": scheduled_for.to_rfc3339(),
                    "schedule_key": "fts::procurement-ingestion/fts",
                }),
                idempotency_key: format!(
                    "cron:fts:procurement-ingestion/fts:{}",
                    scheduled_for.to_rfc3339()
                ),
            },
        )?;
        Ok(())
    }

    fn manifest_result_payload(fixture: &Value) -> Result<Value> {
        let notices = fixture
            .get("notices")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("fixture missing notices"))?;
        if notices.is_empty() {
            let markdown = "# Opportunity Manifest\n\n## Header\n\n- Process: process-empty\n- Title: Empty batch\n\n## Decision Summary\n\n## Per-Candidate Detail\n\n## Operator Input Requests\n\n## Evidence Trail\n\n## Coverage Metrics\n\n- Candidate count: 0\n";
            return Ok(json!({
                "process_id": "process-empty",
                "decisionBasisHash": "empty",
                "manifestMarkdown": format!(
                    "---\nmanifest_version: v1.0-2026-05-01\nprocess_id: process-empty\nbasis_hash: empty\nstoplight: review\nsuggested_action: Operator review required\nclarification_prompts: []\n---\n{markdown}"
                ),
                "stoplight": "review",
                "suggested_action": "Operator review required",
                "suggested_action_reasoning": "process-empty has no Stage 12 candidates, so the manifest remains in review.",
                "clarification_prompts": [],
                "candidate_count": 0,
                "priced_candidate_count": 0,
                "clarification_prompt_count": 0,
            }));
        }
        let first_notice = notices
            .first()
            .ok_or_else(|| anyhow!("fixture missing first notice"))?;
        let process_id = first_notice
            .get("processId")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("fixture missing processId"))?;
        let title = first_notice
            .get("title")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("fixture missing title"))?;
        let candidate_count = first_notice
            .get("lots")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("fixture missing lots"))?
            .iter()
            .filter_map(|lot| lot.get("candidateCount").and_then(Value::as_u64))
            .sum::<u64>();
        let body = format!(
            "# Opportunity Manifest\n\n## Header\n\n- Process: {process_id}\n- Title: {title}\n\n## Decision Summary\n\n## Per-Candidate Detail\n\n## Operator Input Requests\n\n## Evidence Trail\n\n## Coverage Metrics\n\n- Candidate count: {candidate_count}\n"
        );
        let decision_basis_hash = format!("{:x}", Sha256::digest(body.as_bytes()));
        let markdown = format!(
            "---\nmanifest_version: v1.0-2026-05-01\nprocess_id: {process_id}\nbasis_hash: {decision_basis_hash}\nstoplight: review\nsuggested_action: Operator review required\nclarification_prompts: []\n---\n{body}"
        );
        Ok(json!({
            "process_id": process_id,
            "decisionBasisHash": decision_basis_hash,
            "manifestMarkdown": markdown,
            "stoplight": "review",
            "suggested_action": "Operator review required",
            "suggested_action_reasoning": format!("{title} requires operator review before the manifest can drive a decision."),
            "clarification_prompts": [],
            "candidate_count": candidate_count,
            "priced_candidate_count": 0,
            "clarification_prompt_count": 0,
        }))
    }

    fn default_fixture() -> Value {
        json!({
          "cursorBefore": "cursor-000",
          "cursorAfter": "cursor-001",
          "continuationRequired": false,
          "fingerprintState": "matched",
          "localeState": "en-GB",
          "currencyState": "GBP",
          "ptyTranscriptLines": [
            "fingerprint matched",
            "transcript says 99 candidates",
            "ingestion complete"
          ],
          "notices": [
            {
              "processId": "process-fts-001",
              "noticeId": "notice-fts-001",
              "sourceNoticeId": "ocds-001",
              "sourceUrl": "https://www.find-tender.service.gov.uk/notice/001",
              "title": "Supply of industrial widgets",
              "buyerName": "Widgetshire Council",
              "publishedAt": "2026-03-25T00:00:00Z",
              "sourceUpdatedAt": "2026-03-25T00:30:00Z",
              "ocid": "ocds-1234",
              "procurementStage": "tender",
              "lots": [
                {
                  "lotId": "lot-001",
                  "sourceLotId": "source-lot-001",
                  "title": "Widgets lot",
                  "candidateCount": 2,
                  "estimatedValuePence": 120_000
                },
                {
                  "lotId": "lot-002",
                  "sourceLotId": "source-lot-002",
                  "title": "Replacement widgets",
                  "candidateCount": 1,
                  "estimatedValuePence": 120_000
                }
              ]
            }
          ]
        })
    }

    fn commercial_stage_fixture() -> Value {
        json!({
          "cursorBefore": "cursor-commercial-000",
          "cursorAfter": "cursor-commercial-001",
          "continuationRequired": false,
          "fingerprintState": "matched",
          "localeState": "en-GB",
          "currencyState": "GBP",
          "ptyTranscriptLines": ["commercial sweep manifest fixture"],
          "notices": [
            {
              "processId": "process-stage11",
              "noticeId": "notice-stage11",
              "sourceNoticeId": "source-stage11",
              "sourceUrl": "https://example.test/stage11",
              "title": "Stage 11 notice",
              "buyerName": "Buyer",
              "publishedAt": "2026-04-01T00:00:00Z",
              "sourceUpdatedAt": "2026-04-01T00:00:00Z",
              "ocid": "ocds-stage11",
              "procurementStage": "tender",
              "lots": [
                {
                  "lotId": "lot-stage11",
                  "sourceLotId": "source-lot-stage11",
                  "title": "Stage 11 lot",
                  "candidateCount": 1,
                  "estimatedValuePence": 1_000
                }
              ]
            }
          ]
        })
    }

    fn seed_process(conn: &Connection, process_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO procurement_processes (
              id, current_stage, title, latest_published_at, latest_updated_at, owner_id
            ) VALUES (?1, 'manifest', 'Process title', '2026-03-30T00:00:00Z',
                      '2026-03-30T00:00:00Z', 'local-owner')
            ",
            params![process_id],
        )?;
        Ok(())
    }

    fn seed_stage11_minimal_graph(conn: &Connection, quote_country: &str) -> Result<()> {
        conn.execute(
            "INSERT OR IGNORE INTO sources (id, kind, config) VALUES ('stage11-test', 'procurement', '{}')",
            [],
        )?;
        seed_process(conn, "process-stage11")?;
        conn.execute(
            "\
            INSERT INTO raw_documents (
              id, source_id, content_type, source_url, payload_json, metadata
            ) VALUES ('raw-stage11', 'stage11-test', 'application/json',
                      'https://example.test/stage11', '{}', '{}')
            ",
            [],
        )?;
        conn.execute(
            "\
            INSERT INTO contract_notices (
              id, process_id, source_id, source_notice_id, raw_document_id, title,
              summary, buyer_name, procurement_stage, published_at, source_updated_at,
              normalized_payload, owner_id
            ) VALUES (
              'notice-stage11', 'process-stage11', 'stage11-test', 'source-stage11',
              'raw-stage11', 'Stage 11 notice', 'Supply test goods', 'Buyer',
              'tender', '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', '{}',
              'local-owner'
            )
            ",
            [],
        )?;
        conn.execute(
            "\
            INSERT INTO notice_lots (
              id, notice_id, title, description, owner_id
            ) VALUES (
              'lot-stage11', 'notice-stage11', 'Stage 11 lot', 'Test lot',
              'local-owner'
            )
            ",
            [],
        )?;
        conn.execute(
            "\
            INSERT INTO product_candidates (
              id, process_id, notice_id, lot_id, name, candidate_type,
              match_archetype, head_noun, quantity, quantity_unit, owner_id
            ) VALUES (
              'candidate-stage11', 'process-stage11', 'notice-stage11',
              'lot-stage11', 'Test goods', 'goods', 'product', 'goods',
              2, 'unit', 'local-owner'
            )
            ",
            [],
        )?;
        conn.execute(
            "\
            INSERT INTO supplier_quotes (
              id, process_id, notice_id, product_candidate_id, product_name, region,
              quote_class, source_name, ship_from_country, quoted_quantity,
              quoted_quantity_unit, unit_price_original_minor, currency,
              unit_price_gbp_pence, includes_shipping, price_signal_role,
              evidence, owner_id
            ) VALUES (
              'quote-stage11', 'process-stage11', 'notice-stage11',
              'candidate-stage11', 'Test goods quote', 'GB', 'spot',
              'test-source', ?1, 2, 'unit', 100, 'GBP', 100, 0,
              'test_quote', '{}', 'local-owner'
            )
            ",
            params![quote_country],
        )?;
        conn.execute(
            "\
            INSERT INTO candidate_pricing_assessments (
              id, process_id, notice_id, product_candidate_id, selected_quote_id,
              selected_quote_row_version, quote_cost_boundary, revenue_basis_type,
              expected_addressable_revenue_ex_vat_pence, revenue_confidence_bps,
              gross_margin_bps, gross_estimate_confidence_bps, identity_gate_passed,
              usability_gate_passed, pricing_input_confidence_bps, owner_id
            ) VALUES (
              'pricing-stage11', 'process-stage11', 'notice-stage11',
              'candidate-stage11', 'quote-stage11', 1, 'unit', 'expected_value',
              1000, 8000, 8000, 7000, 1, 1, 7000, 'local-owner'
            )
            ",
            [],
        )?;
        Ok(())
    }

    fn child_output_text(output: &std::process::Output) -> String {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        format!(
            "status: {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status
        )
    }

    fn scheduler_tick_db_snapshot(db_path: &Path) -> String {
        let Ok(conn) = Connection::open(db_path) else {
            return "db: unavailable".to_owned();
        };
        let executor_count = conn
            .query_row(
                "SELECT COUNT(*) FROM executors WHERE id = 'local-daemon'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_or_else(|error| format!("error: {error}"), |count| count.to_string());
        let job_rows = match conn.prepare(
            "\
            SELECT job_id, kind, scope_id, scheduled_for, status
            FROM workflow_jobs
            ORDER BY created_at, job_id
            ",
        ) {
            Ok(mut statement) => match statement.query_map([], |row| {
                Ok(format!(
                    "{}|{}|{}|{}|{}",
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    row.get::<_, String>(4)?
                ))
            }) {
                Ok(rows) => rows.filter_map(Result::ok).collect::<Vec<_>>().join("\n"),
                Err(error) => format!("error: {error}"),
            },
            Err(error) => format!("error: {error}"),
        };
        format!("executors.local-daemon: {executor_count}\nworkflow_jobs:\n{job_rows}")
    }

    #[test]
    fn tick_001_due_slot_enqueues_exactly_one_row() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;

        let mut tick_db = StateDb::open(state_db.db_path())?;
        let enqueued = run_scheduler_tick(
            &mut scheduler,
            &registry,
            &mut tick_db,
            ts(2026, 6, 5, 4, 15, 0)?,
        )?;
        ensure!(enqueued == 1, "tick_001 expected exactly one enqueue");

        let (kind, scope_id, scheduled_for, input_payload) =
            query_single_fts_job(state_db.connection(), "2026-06-05T04:15:00+00:00")?;
        ensure!(matches!(kind.as_str(), "sync_source"));
        ensure!(matches!(scope_id.as_str(), "fts"));
        ensure!(matches!(
            scheduled_for.as_str(),
            "2026-06-05T04:15:00+00:00"
        ));

        let payload: Value = serde_json::from_str(&input_payload)?;
        ensure!(matches!(
            payload
                .pointer("/request/source_id")
                .and_then(Value::as_str),
            Some("fts")
        ));
        ensure!(matches!(
            payload.pointer("/request/skill_id").and_then(Value::as_str),
            Some("procurement-ingestion/fts")
        ));
        ensure!(matches!(
            payload
                .pointer("/request/schedule_key")
                .and_then(Value::as_str),
            Some("fts::procurement-ingestion/fts")
        ));
        ensure!(matches!(
            payload
                .pointer("/runner_meta/idempotency_key")
                .and_then(Value::as_str),
            Some("cron:fts:procurement-ingestion/fts:2026-06-05T04:15:00+00:00")
        ));
        Ok(())
    }

    #[test]
    fn m1_one_shot_scheduler_enqueues_due_fts_slot_and_negative_control() -> Result<()> {
        let early_harness = SchedulerOneShotHarness::new()?;
        early_harness.seed_completed_fts_source_health_slot(ts(2026, 6, 5, 3, 0, 0)?)?;
        let seeded_root_jobs = count_root_jobs(&early_harness.connection()?)?;
        let early_result = early_harness.run_scheduler_once("2026-06-05T03:00:00Z")?;
        ensure!(
            early_result.success,
            "M1 negative one-shot should succeed without work: stdout={} stderr={}",
            early_result.stdout,
            early_result.stderr
        );
        ensure!(
            early_result.stdout.contains("enqueued=0") && early_result.stdout.contains("drained=0"),
            "M1 negative one-shot expected no enqueued or drained jobs, got stdout={}",
            early_result.stdout
        );
        let early_conn = early_harness.connection()?;
        ensure!(
            count_root_jobs(&early_conn)? == seeded_root_jobs,
            "M1 negative one-shot expected no new root workflow jobs"
        );
        ensure!(
            count_fts_jobs_at(&early_conn, "2026-06-05T04:15:00+00:00")? == 0,
            "M1 negative one-shot expected no FTS production slot row"
        );

        let due_harness = SchedulerOneShotHarness::new()?;
        let due_result = due_harness.run_scheduler_once("2026-06-05T04:15:00Z")?;
        ensure!(
            due_result.success,
            "M1 due one-shot should succeed: stdout={} stderr={}",
            due_result.stdout,
            due_result.stderr
        );
        ensure!(
            due_result.stdout.contains("enqueued=1") && due_result.stdout.contains("drained=1"),
            "M1 due one-shot expected one enqueued and drained cron job, got stdout={}",
            due_result.stdout
        );

        let due_conn = due_harness.connection()?;
        let (kind, status, scheduled_for, input_payload) = query_latest_root_job(&due_conn)?;
        ensure!(
            kind == "sync_source",
            "M1 due one-shot expected latest root job kind sync_source, got {kind}"
        );
        ensure!(
            status == "completed",
            "M1 due one-shot expected latest root sync_source job to be completed, got {status}"
        );
        ensure!(
            scheduled_for == "2026-06-05T04:15:00+00:00",
            "M1 due one-shot expected FTS scheduled_for to be preserved, got {scheduled_for}"
        );
        let payload: Value = serde_json::from_str(&input_payload)?;
        ensure!(
            payload
                .pointer("/request/source_id")
                .and_then(Value::as_str)
                == Some("fts"),
            "M1 due one-shot expected FTS source_id in cron request"
        );
        ensure!(
            payload.pointer("/request/skill_id").and_then(Value::as_str)
                == Some("procurement-ingestion/fts"),
            "M1 due one-shot expected FTS skill_id in cron request"
        );
        ensure!(
            payload
                .pointer("/runner_meta/idempotency_key")
                .and_then(Value::as_str)
                == Some("cron:fts:procurement-ingestion/fts:2026-06-05T04:15:00+00:00"),
            "M1 due one-shot expected per-slot cron idempotency key"
        );
        ensure!(
            count_materialized_fts_notice_rows(&due_conn)? > 0,
            "M1 due one-shot expected FTS rows in procurement_processes, contract_notices, and raw_documents"
        );
        Ok(())
    }

    #[test]
    fn m2_commercial_stage_autonomy_g6_one_shot_drains_without_manual_rerun() -> Result<()> {
        let harness = SchedulerOneShotHarness::new()?;
        harness.seed_commercial_stage11_graph()?;
        harness.seed_completed_fts_source_health_slot(ts(2026, 6, 5, 7, 0, 0)?)?;
        let fixture = commercial_stage_fixture();
        let result = harness.run_scheduler_once_with_fixture("2026-06-05T07:00:00Z", &fixture)?;
        ensure!(
            result.success,
            "M2 commercial one-shot should succeed: stdout={} stderr={}",
            result.stdout,
            result.stderr
        );
        ensure!(
            result.stdout.contains("enqueued=1") && result.stdout.contains("drained=1"),
            "M2 expected one commercial_sweep enqueue and in-process drain, got stdout={}",
            result.stdout
        );
        for forbidden in [
            "trigger_pipeline_rerun",
            "stop_daemon_for_exclusive_pipeline_rerun",
            "PIPELINE_RERUN_INGEST_STAGES",
        ] {
            ensure!(
                !result.stdout.contains(forbidden) && !result.stderr.contains(forbidden),
                "M2 observed forbidden manual rerun marker `{forbidden}` in one-shot output"
            );
        }

        let conn = harness.connection()?;
        assert_m2_commercial_sweep_side_effects(&conn)?;
        assert_commercial_sweep_body_avoids_manual_rerun()?;
        Ok(())
    }

    #[test]
    fn tick_002_repeated_ticks_same_slot_idempotent() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let due_slot = "2026-06-05T04:15:00+00:00";
        let now = ts(2026, 6, 5, 4, 15, 0)?;

        let mut first_scheduler = CronScheduler::from_registry(&registry);
        first_scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;
        let mut tick_db = StateDb::open(state_db.db_path())?;
        let first_count = run_scheduler_tick(&mut first_scheduler, &registry, &mut tick_db, now)?;
        ensure!(first_count == 1, "tick_002 expected first tick to enqueue");
        ensure!(
            count_fts_jobs_at(state_db.connection(), due_slot)? == 1,
            "tick_002 expected one pending FTS row after first tick"
        );

        let mut pending_scheduler = CronScheduler::from_registry(&registry);
        pending_scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;
        let pending_count =
            run_scheduler_tick(&mut pending_scheduler, &registry, &mut tick_db, now)?;
        ensure!(
            pending_count == 1,
            "tick_002 expected pending duplicate to use the same dispatch tuple"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), due_slot)? == 1,
            "tick_002 expected pending idempotency to keep one FTS row"
        );

        state_db.connection().execute(
            "\
            UPDATE workflow_jobs
            SET status = 'completed'
            WHERE kind = 'sync_source'
              AND scope_id = 'fts'
              AND scheduled_for = ?1
            ",
            params![due_slot],
        )?;

        let mut completed_scheduler = CronScheduler::from_registry(&registry);
        completed_scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;
        let completed_count =
            run_scheduler_tick(&mut completed_scheduler, &registry, &mut tick_db, now)?;
        ensure!(
            completed_count == 1,
            "tick_002 expected completed duplicate to use the same dispatch tuple"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), due_slot)? == 1,
            "tick_002 expected completed idempotency to keep one FTS row"
        );

        let main_source = include_str!("../src/main.rs");
        ensure!(
            !main_source.contains("enqueue_startup_cron_jobs"),
            "tick_002 expected startup cron enqueue shim to be removed"
        );
        ensure!(
            !main_source.contains("fn enqueue_due_cron_jobs("),
            "tick_002 expected no second cron dispatch implementation in main.rs"
        );
        let cron_source = include_str!("../src/scheduler/cron.rs");
        ensure!(
            !main_source.contains("cron_dedup") && !cron_source.contains("cron_dedup"),
            "tick_002 expected no auxiliary cron dedup table or flag"
        );
        Ok(())
    }

    #[test]
    fn tick_003_long_gap_catchup() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;
        let mut tick_db = StateDb::open(state_db.db_path())?;

        let first_slot = ts(2026, 6, 5, 4, 15, 0)?;
        let first_enqueued =
            run_scheduler_tick(&mut scheduler, &registry, &mut tick_db, first_slot)?;
        ensure!(
            first_enqueued == 1,
            "tick_003 expected first normal FTS slot enqueue"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), "2026-06-05T04:15:00+00:00")? == 1,
            "tick_003 expected first FTS row at the normal slot"
        );
        let (last_after_first, next_after_first) = fts_schedule_state(&scheduler)?;
        ensure!(
            last_after_first.as_deref() == Some("2026-06-05T04:15:00+00:00"),
            "tick_003 expected record_enqueued to store first last_run_at"
        );
        ensure!(
            next_after_first.as_deref() == Some("2026-06-06T04:15:00+00:00"),
            "tick_003 expected next FTS run after first slot"
        );

        let gap_now = ts(2026, 6, 8, 6, 0, 0)?;
        let due_dispatches = scheduler.dispatch_due_jobs(&registry, gap_now)?;
        let due_keys = due_dispatches
            .iter()
            .map(|dispatch| dispatch.definition_key.as_str())
            .collect::<Vec<_>>();
        let fts_due = due_dispatches
            .iter()
            .find(|dispatch| dispatch.definition_key == "fts::procurement-ingestion/fts")
            .ok_or_else(|| anyhow!("tick_003 expected FTS catch-up dispatch in {due_keys:?}"))?;
        ensure!(
            fts_due.enqueue_request.scheduled_for == Some(ts(2026, 6, 6, 4, 15, 0)?),
            "tick_003 expected FTS catch-up slot to follow schedule.after(last_run_at); due keys: {due_keys:?}"
        );

        let gap_enqueued = run_scheduler_tick(&mut scheduler, &registry, &mut tick_db, gap_now)?;
        ensure!(
            gap_enqueued >= 1,
            "tick_003 expected long-gap tick to enqueue overdue work"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), "2026-06-06T04:15:00+00:00")? == 1,
            "tick_003 expected FTS catch-up row at the next missed slot"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), "2026-06-08T04:15:00+00:00")? == 0,
            "tick_003 expected catch-up not to jump directly to the current-day slot"
        );
        let (last_after_gap, next_after_gap) = fts_schedule_state(&scheduler)?;
        ensure!(
            last_after_gap.as_deref() == Some("2026-06-06T04:15:00+00:00"),
            "tick_003 expected record_enqueued to store catch-up last_run_at"
        );
        ensure!(
            next_after_gap.as_deref() == Some("2026-06-07T04:15:00+00:00"),
            "tick_003 expected record_enqueued to advance next_run_at in memory"
        );

        let main_source = include_str!("../src/main.rs");
        let thread_tick_start = main_source
            .find("fn run_scheduler_thread_tick(")
            .ok_or_else(|| anyhow!("run_scheduler_thread_tick function missing"))?;
        let thread_tick_source = main_source
            .get(thread_tick_start..)
            .ok_or_else(|| anyhow!("scheduler thread tick source slice out of bounds"))?;
        let thread_tick_len = thread_tick_source
            .find("fn seed_cron_scheduler_from_history(")
            .ok_or_else(|| anyhow!("scheduler thread tick source slice out of bounds"))?;
        let thread_tick_end = thread_tick_start
            .checked_add(thread_tick_len)
            .ok_or_else(|| anyhow!("scheduler thread tick source slice overflow"))?;
        let thread_tick_body = main_source
            .get(thread_tick_start..thread_tick_end)
            .ok_or_else(|| anyhow!("scheduler thread tick source slice out of bounds"))?;
        ensure!(
            !thread_tick_body.contains("detect_host_sleep"),
            "tick_003 expected host-sleep detection not to suppress scheduler ticks"
        );
        Ok(())
    }

    #[test]
    fn tick_005_registry_reload_on_generation_bump() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let mut tick_db = StateDb::open(state_db.db_path())?;
        let _ = set_fts_source_enabled(state_db.connection(), false)?;

        let disabled_registry = SourceRegistry::load(state_db.connection())?;
        let mut disabled_scheduler = CronScheduler::from_registry(&disabled_registry);
        disabled_scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;
        let disabled_definition = disabled_scheduler
            .definitions()
            .get("fts::procurement-ingestion/fts")
            .ok_or_else(|| anyhow!("missing disabled FTS cron definition"))?;
        ensure!(
            !disabled_definition.enabled,
            "tick_005 expected FTS cron definition to start disabled"
        );

        let due_now = ts(2026, 6, 5, 4, 15, 0)?;
        let disabled_count = run_scheduler_tick(
            &mut disabled_scheduler,
            &disabled_registry,
            &mut tick_db,
            due_now,
        )?;
        ensure!(
            disabled_count == 0,
            "tick_005 expected disabled source to enqueue nothing"
        );
        ensure!(
            count_jobs(&state_db)? == 0,
            "tick_005 expected no workflow rows before source enablement"
        );

        let observed_generation = source_registry_generation();
        let enabled_source_changed = set_fts_source_enabled(state_db.connection(), true)?;
        ensure!(
            enabled_source_changed,
            "tick_005 expected enabling FTS to mutate the source registry"
        );
        let bumped_generation = source_registry_generation();
        ensure!(
            bumped_generation > observed_generation,
            "tick_005 expected source mutation path to bump Q9 generation"
        );

        let reloaded_registry = SourceRegistry::load(state_db.connection())?;
        let mut reloaded_scheduler = CronScheduler::from_registry(&reloaded_registry);
        let reload_anchor = due_now
            .checked_sub_signed(ChronoDuration::seconds(1))
            .ok_or_else(|| anyhow!("tick_005 reload anchor underflow"))?;
        reloaded_scheduler.refresh_next_run_at(reload_anchor)?;
        let enabled_definition = reloaded_scheduler
            .definitions()
            .get("fts::procurement-ingestion/fts")
            .ok_or_else(|| anyhow!("missing reloaded FTS cron definition"))?;
        ensure!(
            enabled_definition.enabled,
            "tick_005 expected reloaded FTS cron definition to be enabled"
        );

        let reloaded_count = run_scheduler_tick(
            &mut reloaded_scheduler,
            &reloaded_registry,
            &mut tick_db,
            due_now,
        )?;
        ensure!(
            reloaded_count == 1,
            "tick_005 expected reloaded source to enqueue exactly the due slot"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), "2026-06-05T04:15:00+00:00")? == 1,
            "tick_005 expected one FTS job at the current due slot"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), "2026-06-04T04:15:00+00:00")? == 0,
            "tick_005 expected reload reseed to avoid older catch-up rows"
        );

        let (last_run_at, next_run_at) = fts_schedule_state(&reloaded_scheduler)?;
        ensure!(
            last_run_at.as_deref() == Some("2026-06-05T04:15:00+00:00"),
            "tick_005 expected record_enqueued to store the reloaded due slot"
        );
        ensure!(
            next_run_at.as_deref() == Some("2026-06-06T04:15:00+00:00"),
            "tick_005 expected next run to advance from the reloaded due slot"
        );

        Ok(())
    }

    #[test]
    fn tick_006_failed_slot_reenqueueable() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let scheduled_for = ts(2026, 6, 5, 4, 15, 0)?;
        let due_slot = scheduled_for.to_rfc3339();
        enqueue_fts_history(state_db.db_path(), scheduled_for)?;
        state_db.connection().execute(
            "\
            UPDATE workflow_jobs
            SET status = 'failed'
            WHERE kind = 'sync_source'
              AND scope_id = 'fts'
              AND scheduled_for = ?1
            ",
            params![due_slot],
        )?;
        ensure!(
            count_fts_jobs_at_status(state_db.connection(), &due_slot, "failed")? == 1,
            "tick_006 expected one failed seed row"
        );

        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;
        let mut tick_db = StateDb::open(state_db.db_path())?;
        let enqueued = run_scheduler_tick(&mut scheduler, &registry, &mut tick_db, scheduled_for)?;
        ensure!(enqueued == 1, "tick_006 expected failed slot to re-enqueue");
        ensure!(
            count_fts_jobs_at(state_db.connection(), &due_slot)? == 2,
            "tick_006 expected failed row plus new pending row for same slot"
        );
        ensure!(
            count_fts_jobs_at_status(state_db.connection(), &due_slot, "pending")? == 1,
            "tick_006 expected one new pending retry row"
        );
        ensure!(
            count_fts_jobs_at_status(state_db.connection(), &due_slot, "failed")? == 1,
            "tick_006 expected original failed row to remain failed"
        );
        Ok(())
    }

    #[test]
    fn tick_004_clock_seam() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.refresh_next_run_at(ts(2026, 6, 5, 3, 0, 0)?)?;
        scheduler.record_enqueued("fts::source-health/fts", ts(2026, 6, 5, 3, 0, 0)?)?;

        let mut tick_db = StateDb::open(state_db.db_path())?;
        let before_count = run_scheduler_tick(
            &mut scheduler,
            &registry,
            &mut tick_db,
            ts(2026, 6, 5, 3, 0, 0)?,
        )?;
        ensure!(
            before_count == 0,
            "tick_004 expected no before-slot enqueues"
        );
        ensure!(
            count_jobs(&state_db)? == 0,
            "tick_004 expected no workflow jobs before FTS slot"
        );

        let due_count = run_scheduler_tick(
            &mut scheduler,
            &registry,
            &mut tick_db,
            ts(2026, 6, 5, 4, 15, 0)?,
        )?;
        ensure!(due_count > 0, "tick_004 expected due-slot enqueues");

        let fts_count: i64 = state_db.connection().query_row(
            "\
            SELECT COUNT(*)
            FROM workflow_jobs
            WHERE kind = 'sync_source'
              AND scope_id = 'fts'
              AND scheduled_for = ?1
            ",
            params!["2026-06-05T04:15:00+00:00"],
            |row| row.get(0),
        )?;
        ensure!(
            fts_count == 1,
            "tick_004 expected exactly one due FTS sync_source row, got {fts_count}"
        );

        let source = include_str!("../src/scheduler/tick.rs");
        let tick_body_start = source
            .find("pub fn run_scheduler_tick(")
            .ok_or_else(|| anyhow!("run_scheduler_tick function missing"))?;
        let tick_body = source
            .get(tick_body_start..)
            .ok_or_else(|| anyhow!("run_scheduler_tick source slice out of bounds"))?;
        ensure!(
            !tick_body.contains("Utc::now()"),
            "tick_004 run_scheduler_tick must use injected time, not Utc::now()"
        );
        Ok(())
    }

    #[test]
    fn tick_007_serve_scheduler_thread_spawn_poll_terminate() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let state_root = temp.path().join("state");
        let runtime_root = temp.path().join("runtime");
        let artifact_root = temp.path().join("artifacts");
        fs::create_dir_all(&state_root)?;
        fs::create_dir_all(&runtime_root)?;
        fs::create_dir_all(&artifact_root)?;
        let db_path = state_root.join("state.db");
        enqueue_fts_history(&db_path, ts(2026, 6, 5, 4, 15, 0)?)?;

        let mut child = Command::new(env!("CARGO_BIN_EXE_daemon"))
            .arg("--serve")
            .arg("--state-root")
            .arg(&state_root)
            .arg("--runtime-dir")
            .arg(&runtime_root)
            .arg("--artifact-root")
            .arg(&artifact_root)
            .env("DAEMON_SCHEDULER_TICK_INTERVAL_SECS", "1")
            .env_remove("SUPABASE_URL")
            .env_remove("NEXT_PUBLIC_SUPABASE_URL")
            .env_remove("SUPABASE_SERVICE_ROLE_KEY")
            .env_remove("SUPABASE_SERVICE_KEY")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| anyhow!("spawning daemon --serve: {error}"))?;

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut observed_thread_tick = false;
        while Instant::now() < deadline {
            if child.try_wait()?.is_some() {
                let output = child.wait_with_output()?;
                anyhow::bail!(
                    "daemon --serve exited before scheduler tick:\n{}",
                    child_output_text(&output)
                );
            }
            if let Ok(conn) = Connection::open(&db_path)
                && let Ok(count) = count_fts_jobs_at(&conn, "2026-06-06T04:15:00+00:00")
                && count == 1
            {
                observed_thread_tick = true;
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }

        if child.try_wait()?.is_none() {
            child.kill()?;
        }
        let output = child.wait_with_output()?;
        ensure!(
            observed_thread_tick,
            "tick_007 expected serve scheduler thread to enqueue cold-start catch-up:\n{}\n{}",
            child_output_text(&output),
            scheduler_tick_db_snapshot(&db_path)
        );

        let source = include_str!("../src/main.rs");
        ensure!(
            source.contains("struct SchedulerTickThread"),
            "tick_007 expected a dedicated scheduler thread owner"
        );
        ensure!(
            source.contains("fn initialize_scheduler_tick_thread(")
                && source.contains("open_state_db(state_db_path, DEFAULT_SCHEMA_EPOCH)"),
            "tick_007 expected scheduler thread initialization to open its own StateDb connection"
        );
        ensure!(
            source.contains("recv_timeout(interval)"),
            "tick_007 expected scheduler thread to use recv_timeout loop"
        );
        ensure!(
            source.contains("SchedulerTickThread::spawn("),
            "tick_007 expected serve path to spawn the scheduler thread"
        );
        ensure!(
            source.contains("impl Drop for SchedulerTickThread")
                && source.contains("let _ = stop_tx.send(());")
                && source.contains("handle.join()"),
            "tick_007 expected scheduler thread shutdown to signal stop and join cleanly"
        );
        Ok(())
    }
}
