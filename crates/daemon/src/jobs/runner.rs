#![forbid(unsafe_code)]

#[allow(clippy::duplicate_mod)]
#[cfg(test)]
#[path = "claim.rs"]
mod claim;
#[cfg(not(test))]
use crate::jobs::claim;

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, ensure};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

#[cfg(not(test))]
use crate::agents::contracts;
#[cfg(test)]
use crate::contracts_impl as contracts;
#[cfg(not(test))]
use crate::db::sqlite::StateDb;
#[cfg(test)]
use crate::support::db::StateDb;

use contracts::{
    StageValidationWrite, ValidationDisposition, ValidationOutcome, persist_stage_validation,
};

pub const SQLITE_WRITE_FAILED: &str = "SQLITE_WRITE_FAILED";
pub const JOB_OBSOLETE: &str = "JOB_OBSOLETE";
const DEFAULT_EXECUTOR_OWNER_ID: &str = "local-owner";

pub use claim::JobLeaseHeartbeat;

pub const CANONICAL_SCOPE_KINDS: [&str; 6] = [
    "process",
    "notice",
    "lot",
    "candidate",
    "candidate_source_pair",
    "candidate_quote_scenario",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageCommitSemantics {
    Precursor,
    SetReplacement,
    Additive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessState {
    Fresh,
    SoftStale,
    HardInvalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageScopeRule {
    pub scope_kind: Option<&'static str>,
    pub commit_semantics: StageCommitSemantics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedJob {
    pub id: String,
    pub kind: String,
    pub process_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub input_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationAck {
    pub accepted: bool,
    pub visible_after_seq: i64,
    pub continuation_job_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistSuccessRequest {
    pub job_id: String,
    pub idempotency_key: String,
    pub session_id: String,
    pub output_payload: Value,
    pub stage: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub stage_run_id: String,
    pub process_id: String,
    pub owner_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueueJobRequest {
    pub job_id: String,
    pub kind: String,
    pub process_id: Option<String>,
    pub notice_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub parent_job_id: Option<String>,
    pub continuation_chain_id: Option<String>,
    pub validation_required: bool,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub input_payload: Value,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistContinuationRequest {
    pub job_id: String,
    pub idempotency_key: String,
    pub continuation_job_id: String,
    pub continuation_chain_id: String,
    pub next_job_kind: String,
    pub process_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub resume_token: Value,
    pub owner_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObsoleteJobRequest {
    pub job_id: String,
    pub idempotency_key: String,
    pub rerun_job_id: String,
    pub next_job_kind: String,
    pub process_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub rerun_from_stage: String,
    pub expected_input_basis_hash: String,
    pub actual_input_basis_hash: String,
}

#[derive(Debug, Clone)]
pub struct JobRunner {
    executor_id: String,
}

impl JobRunner {
    #[must_use]
    pub fn new(executor_id: impl Into<String>) -> Self {
        Self {
            executor_id: executor_id.into(),
        }
    }

    pub fn claim_next_job(&self, conn: &Connection) -> Result<Option<ClaimedJob>> {
        touch_executor(conn, &self.executor_id)?;
        claim::claim_next_job(conn, &self.executor_id).context("claiming next workflow job")
    }

    pub fn persist_success(
        &self,
        state_db: &mut StateDb,
        request: &PersistSuccessRequest,
        outcome: &ValidationOutcome,
    ) -> Result<MutationAck> {
        if let Some(existing) = find_existing_ack(
            state_db.connection(),
            &request.job_id,
            &request.idempotency_key,
        )? {
            return Ok(existing);
        }

        ensure!(
            outcome.disposition == ValidationDisposition::Accepted,
            "persist_success requires accepted outcome"
        );
        ensure!(
            !request.stage_run_id.is_empty(),
            "stage_run_id must not be empty"
        );
        let stage_rule = validate_stage_scope(&request.stage, &request.scope_kind)?;
        ensure!(
            stage_rule.commit_semantics != StageCommitSemantics::Precursor,
            "ingest precursor stage does not use process_stage_status"
        );

        let (predicted_visible_after_seq, visible_after_seq) =
            state_db.with_write_transaction(|tx| {
                let predicted_visible_after_seq = current_commit_seq(tx)? + 1;
                let stage_write = StageValidationWrite {
                    process_id: &request.process_id,
                    stage: &request.stage,
                    scope_kind: &request.scope_kind,
                    scope_id: &request.scope_id,
                    stage_run_id: &request.stage_run_id,
                    owner_id: &request.owner_id,
                };
                persist_stage_validation(tx, &stage_write, outcome)?;

                let stored_payload = json!({
                    "result": request.output_payload,
                    "runner_meta": {
                        "idempotency_key": request.idempotency_key,
                        "visible_after_seq": predicted_visible_after_seq,
                        "executor_id": self.executor_id,
                        "continuation_job_id": Value::Null,
                    }
                })
                .to_string();

                let updated = tx.execute(
                    "\
                    UPDATE workflow_jobs
                    SET status = 'completed',
                        completed_at = ?1,
                        output_payload = ?2
                    WHERE id = ?3
                    ",
                    params![Utc::now().to_rfc3339(), stored_payload, request.job_id],
                )?;
                ensure!(updated == 1, "workflow job missing for {}", request.job_id);
                Ok(predicted_visible_after_seq)
            })?;

        ensure!(
            predicted_visible_after_seq == visible_after_seq,
            "predicted commit fence diverged from committed fence"
        );

        Ok(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: None,
        })
    }

    pub fn enqueue_job(
        &self,
        state_db: &mut StateDb,
        request: &EnqueueJobRequest,
    ) -> Result<MutationAck> {
        if let Some(existing) = find_existing_enqueue_ack(state_db.connection(), request)? {
            return Ok(existing);
        }

        let (predicted_visible_after_seq, visible_after_seq) =
            state_db.with_write_transaction(|tx| {
                let predicted_visible_after_seq = current_commit_seq(tx)? + 1;
                let stored_input_payload = json!({
                    "request": request.input_payload,
                    "runner_meta": {
                        "idempotency_key": request.idempotency_key,
                        "visible_after_seq": predicted_visible_after_seq,
                        "executor_id": self.executor_id,
                    }
                })
                .to_string();

                tx.execute(
                    "\
                    INSERT INTO workflow_jobs (
                      id, kind, process_id, notice_id, scope_kind, scope_id, parent_job_id,
                      continuation_chain_id, validation_required, status, scheduled_for,
                      input_payload
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending', ?10, ?11)
                    ",
                    params![
                        request.job_id,
                        request.kind,
                        request.process_id,
                        request.notice_id,
                        request.scope_kind,
                        request.scope_id,
                        request.parent_job_id,
                        request.continuation_chain_id,
                        i64::from(request.validation_required),
                        request
                            .scheduled_for
                            .map(|scheduled_for| scheduled_for.to_rfc3339()),
                        stored_input_payload,
                    ],
                )?;

                Ok(predicted_visible_after_seq)
            })?;

        ensure!(
            predicted_visible_after_seq == visible_after_seq,
            "predicted commit fence diverged from committed fence"
        );

        Ok(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: None,
        })
    }

    pub fn persist_continuation(
        &self,
        state_db: &mut StateDb,
        request: &PersistContinuationRequest,
    ) -> Result<MutationAck> {
        if let Some(existing) = find_existing_ack(
            state_db.connection(),
            &request.job_id,
            &request.idempotency_key,
        )? {
            return Ok(existing);
        }

        let (predicted_visible_after_seq, visible_after_seq) =
            state_db.with_write_transaction(|tx| {
                let predicted_visible_after_seq = current_commit_seq(tx)? + 1;

                tx.execute(
                    "\
                    INSERT INTO workflow_jobs (
                      id, kind, process_id, scope_kind, scope_id, parent_job_id,
                      continuation_chain_id, status, input_payload
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8)
                    ",
                    params![
                        request.continuation_job_id,
                        request.next_job_kind,
                        request.process_id,
                        request.scope_kind,
                        request.scope_id,
                        request.job_id,
                        request.continuation_chain_id,
                        json!({"resume_token": request.resume_token}).to_string(),
                    ],
                )?;

                let stored_payload = json!({
                    "runner_meta": {
                        "idempotency_key": request.idempotency_key,
                        "visible_after_seq": predicted_visible_after_seq,
                        "executor_id": self.executor_id,
                        "continuation_job_id": request.continuation_job_id,
                    }
                })
                .to_string();

                let updated = tx.execute(
                    "\
                    UPDATE workflow_jobs
                    SET status = 'completed',
                        chain_closed = 0,
                        completed_at = ?1,
                        output_payload = ?2
                    WHERE id = ?3
                    ",
                    params![Utc::now().to_rfc3339(), stored_payload, request.job_id],
                )?;
                ensure!(updated == 1, "workflow job missing for {}", request.job_id);

                Ok(predicted_visible_after_seq)
            })?;

        ensure!(
            predicted_visible_after_seq == visible_after_seq,
            "predicted commit fence diverged from committed fence"
        );

        Ok(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: Some(request.continuation_job_id.clone()),
        })
    }

    pub fn mark_obsolete_and_enqueue_rerun(
        &self,
        state_db: &mut StateDb,
        request: &ObsoleteJobRequest,
    ) -> Result<MutationAck> {
        if let Some(existing) = find_existing_ack(
            state_db.connection(),
            &request.job_id,
            &request.idempotency_key,
        )? {
            return Ok(existing);
        }

        ensure!(
            request.expected_input_basis_hash != request.actual_input_basis_hash,
            "basis hash mismatch handler requires differing hashes"
        );

        let (predicted_visible_after_seq, visible_after_seq) =
            state_db.with_write_transaction(|tx| {
                let predicted_visible_after_seq = current_commit_seq(tx)? + 1;
                tx.execute(
                    "\
                    INSERT INTO workflow_jobs (
                      id, kind, process_id, scope_kind, scope_id, parent_job_id,
                      status, input_payload
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7)
                    ",
                    params![
                        request.rerun_job_id,
                        request.next_job_kind,
                        request.process_id,
                        request.scope_kind,
                        request.scope_id,
                        request.job_id,
                        json!({
                            "rerun_from_stage": request.rerun_from_stage,
                            "expected_input_basis_hash": request.expected_input_basis_hash,
                        })
                        .to_string(),
                    ],
                )?;

                let updated = tx.execute(
                    "\
                    UPDATE workflow_jobs
                    SET status = 'obsolete',
                        last_error = ?1,
                        completed_at = ?2,
                        output_payload = ?3
                    WHERE id = ?4
                    ",
                    params![
                        format!(
                            "{}: expected {} got {}",
                            JOB_OBSOLETE,
                            request.expected_input_basis_hash,
                            request.actual_input_basis_hash
                        ),
                        Utc::now().to_rfc3339(),
                        json!({
                            "runner_meta": {
                                "idempotency_key": request.idempotency_key,
                                "visible_after_seq": predicted_visible_after_seq,
                                "executor_id": self.executor_id,
                                "continuation_job_id": request.rerun_job_id,
                            }
                        })
                        .to_string(),
                        request.job_id,
                    ],
                )?;
                ensure!(updated == 1, "workflow job missing for {}", request.job_id);
                Ok(predicted_visible_after_seq)
            })?;

        ensure!(
            predicted_visible_after_seq == visible_after_seq,
            "predicted commit fence diverged from committed fence"
        );

        Ok(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: Some(request.rerun_job_id.clone()),
        })
    }
}

#[must_use]
pub fn start_job_lease_heartbeat(
    state_db_path: PathBuf,
    job_id: impl Into<String>,
    lease_owner: impl Into<String>,
) -> JobLeaseHeartbeat {
    JobLeaseHeartbeat::start(state_db_path, job_id.into(), lease_owner.into())
}

fn touch_executor(conn: &Connection, executor_id: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO executors (
          id, device_name, status, last_heartbeat_at, owner_id, updated_at
        ) VALUES (?1, ?2, 'ready', CURRENT_TIMESTAMP, ?3, CURRENT_TIMESTAMP)
        ON CONFLICT(id) DO UPDATE SET
          device_name = excluded.device_name,
          status = excluded.status,
          owner_id = excluded.owner_id,
          last_heartbeat_at = CURRENT_TIMESTAMP,
          updated_at = CURRENT_TIMESTAMP
        ",
        params![executor_id, executor_id, DEFAULT_EXECUTOR_OWNER_ID],
    )
    .context("touching executor identity")?;
    Ok(())
}

pub fn classify_sqlite_write_failed(error: &rusqlite::Error) -> anyhow::Error {
    anyhow!("{SQLITE_WRITE_FAILED}: {error}")
}

#[must_use]
pub fn allocate_stage_run_id() -> String {
    format!("stage-run-{}", Uuid::new_v4())
}

pub fn stage_scope_rule(stage: &str) -> Result<StageScopeRule> {
    let normalized = stage.to_ascii_lowercase();
    let rule = match normalized.as_str() {
        "ingest" => StageScopeRule {
            scope_kind: None,
            commit_semantics: StageCommitSemantics::Precursor,
        },
        "normalize" | "prequalify notice" | "acquire docs" | "extract lots" => StageScopeRule {
            scope_kind: Some("notice"),
            commit_semantics: StageCommitSemantics::SetReplacement,
        },
        "relate" | "manifest" | "operator decision" | "bid workflow" => StageScopeRule {
            scope_kind: Some("process"),
            commit_semantics: StageCommitSemantics::SetReplacement,
        },
        "qualify lot" => StageScopeRule {
            scope_kind: Some("lot"),
            commit_semantics: StageCommitSemantics::SetReplacement,
        },
        "extract products" => StageScopeRule {
            scope_kind: Some("lot"),
            commit_semantics: StageCommitSemantics::Additive,
        },
        "score quoteability" | "gross margin gate" | "net margin gate" => StageScopeRule {
            scope_kind: Some("candidate"),
            commit_semantics: StageCommitSemantics::Additive,
        },
        "price-signal match" => StageScopeRule {
            scope_kind: Some("candidate_source_pair"),
            commit_semantics: StageCommitSemantics::Additive,
        },
        "landed cost" => StageScopeRule {
            scope_kind: Some("candidate_quote_scenario"),
            commit_semantics: StageCommitSemantics::Additive,
        },
        _ => return Err(anyhow!("unknown stage `{stage}`")),
    };
    Ok(rule)
}

pub fn validate_scope_kind(scope_kind: &str) -> Result<()> {
    ensure!(
        CANONICAL_SCOPE_KINDS.contains(&scope_kind),
        "scope_kind `{scope_kind}` is not canonical"
    );
    Ok(())
}

pub fn validate_stage_scope(stage: &str, scope_kind: &str) -> Result<StageScopeRule> {
    validate_scope_kind(scope_kind)?;
    let rule = stage_scope_rule(stage)?;
    let expected_scope_kind = rule
        .scope_kind
        .ok_or_else(|| anyhow!("stage `{stage}` is a precursor and has no scope_kind"))?;
    ensure!(
        expected_scope_kind == scope_kind,
        "stage `{stage}` must use scope_kind `{expected_scope_kind}`"
    );
    Ok(rule)
}

pub fn set_stage_head_freshness(
    conn: &Connection,
    process_id: &str,
    stage: &str,
    scope_kind: &str,
    scope_id: &str,
    freshness_state: FreshnessState,
) -> Result<()> {
    let updated = conn.execute(
        "\
        UPDATE process_stage_heads
        SET freshness_state = ?1,
            updated_at = CURRENT_TIMESTAMP
        WHERE process_id = ?2 AND stage = ?3 AND scope_kind = ?4 AND scope_id = ?5
        ",
        params![
            freshness_state.as_str(),
            process_id,
            stage,
            scope_kind,
            scope_id,
        ],
    )?;
    ensure!(updated == 1, "stage head missing for freshness update");
    Ok(())
}

pub fn persist_failed_set_replacement_attempt(
    conn: &Connection,
    write: &StageValidationWrite<'_>,
    error_manifest_id: Option<&str>,
) -> Result<()> {
    let rule = validate_stage_scope(write.stage, write.scope_kind)?;
    ensure!(
        rule.commit_semantics == StageCommitSemantics::SetReplacement,
        "partial-failure helper is only valid for set-replacement stages"
    );

    conn.execute(
        "\
        INSERT INTO process_stage_status (
          id, process_id, stage, scope_kind, scope_id, stage_run_id, state,
          freshness_state, started_at, completed_at, error_manifest_id,
          input_revision, output_revision, owner_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'failed', 'fresh', ?7, ?8, ?9, 0, 0, ?10)
        ",
        params![
            format!("{}-failed", write.stage_run_id),
            write.process_id,
            write.stage,
            write.scope_kind,
            write.scope_id,
            write.stage_run_id,
            Utc::now().to_rfc3339(),
            Utc::now().to_rfc3339(),
            error_manifest_id,
            write.owner_id,
        ],
    )?;
    Ok(())
}

#[must_use]
pub fn basis_hash_mismatch(expected: &str, actual: &str) -> bool {
    expected != actual
}

impl FreshnessState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::SoftStale => "soft_stale",
            Self::HardInvalid => "hard_invalid",
        }
    }
}

fn current_commit_seq(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT commit_seq FROM commit_clock WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .context("reading current commit_seq")
}

fn find_existing_ack(
    conn: &Connection,
    job_id: &str,
    idempotency_key: &str,
) -> Result<Option<MutationAck>> {
    let output_payload: Option<String> = conn
        .query_row(
            "SELECT output_payload FROM workflow_jobs WHERE id = ?1 AND status = 'completed'",
            params![job_id],
            |row| row.get(0),
        )
        .optional()
        .context("loading workflow job output payload")?;

    let Some(output_payload) = output_payload else {
        return Ok(None);
    };

    let parsed: Value =
        serde_json::from_str(&output_payload).context("parsing workflow output payload")?;
    let meta = parsed
        .get("runner_meta")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("completed workflow job missing runner_meta payload"))?;

    if meta.get("idempotency_key").and_then(Value::as_str) != Some(idempotency_key) {
        return Ok(None);
    }

    let visible_after_seq = meta
        .get("visible_after_seq")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("runner_meta missing visible_after_seq"))?;
    let continuation_job_id = meta
        .get("continuation_job_id")
        .and_then(Value::as_str)
        .map(str::to_owned);

    Ok(Some(MutationAck {
        accepted: true,
        visible_after_seq,
        continuation_job_id,
    }))
}

fn find_existing_enqueue_ack(
    conn: &Connection,
    request: &EnqueueJobRequest,
) -> Result<Option<MutationAck>> {
    let mut statement = conn.prepare(
        "\
        SELECT input_payload
        FROM workflow_jobs
        WHERE kind = ?1
          AND process_id IS ?2
          AND notice_id IS ?3
          AND scope_kind IS ?4
          AND scope_id IS ?5
          AND parent_job_id IS ?6
          AND continuation_chain_id IS ?7
          AND status IN ('pending', 'running', 'retry', 'completed')
        ORDER BY created_at DESC
        ",
    )?;
    let rows = statement.query_map(
        params![
            request.kind,
            request.process_id,
            request.notice_id,
            request.scope_kind,
            request.scope_id,
            request.parent_job_id,
            request.continuation_chain_id,
        ],
        |row| row.get::<_, String>(0),
    )?;

    for row in rows {
        let raw_payload = row?;
        let parsed: Value =
            serde_json::from_str(&raw_payload).context("parsing workflow input payload")?;
        let Some(meta) = parsed.get("runner_meta").and_then(Value::as_object) else {
            continue;
        };
        if meta.get("idempotency_key").and_then(Value::as_str)
            != Some(request.idempotency_key.as_str())
        {
            continue;
        }
        let visible_after_seq = meta
            .get("visible_after_seq")
            .and_then(Value::as_i64)
            .ok_or_else(|| anyhow!("runner_meta missing visible_after_seq"))?;
        return Ok(Some(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: None,
        }));
    }

    Ok(None)
}
