#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use anyhow::{Context, Result, ensure};
use rusqlite::{OptionalExtension, Transaction, params};
use serde_json::{Value, json};

use crate::db::sqlite::StateDb;
use crate::jobs::runner::{
    MutationAck, PromoteProvisionalRequest, find_existing_ack,
    promote_enforcement_provisional_output_tx,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverrideFeedbackRequest {
    pub executor_id: String,
    pub validation_verdict_id: String,
    pub promotion: PromoteProvisionalRequest,
    pub slice_id: String,
    pub source_artifact_path: Option<String>,
    pub dominant_failure_family: String,
    pub candidate_mutation_template_json: Value,
}

#[derive(Debug, Clone)]
struct OverrideVerdictRow {
    worker_job_id: String,
    verdict: String,
    summary: String,
    incident_signature: Option<String>,
    issues_json: String,
    quality_assessment_json: Option<String>,
    skill_improvement_suggestions_json: Option<String>,
    override_verdict: String,
    override_reason: String,
    override_by: String,
    override_at: String,
}

#[derive(Debug, Clone)]
struct IncidentClusterState {
    affected_slice_ids_json: String,
    override_case_ids_json: String,
    calibration_case_count: i64,
}

pub fn ingest_override(
    state_db: &mut StateDb,
    request: &OverrideFeedbackRequest,
) -> Result<MutationAck> {
    validate_override_feedback_request(request)?;
    if let Some(existing) = find_existing_ack(
        state_db.connection(),
        &request.promotion.worker_job_id,
        &request.promotion.idempotency_key,
    )? {
        ensure!(
            override_feedback_case_exists(state_db.connection(), &request.validation_verdict_id)?,
            "existing override promotion is missing calibration feedback for `{}`",
            request.validation_verdict_id
        );
        return Ok(existing);
    }

    let schema_epoch =
        i32::try_from(state_db.schema_epoch()).context("schema_epoch exceeds i32")?;
    let (ack, visible_after_seq) = state_db.with_write_transaction(|tx| {
        let ack = promote_enforcement_provisional_output_tx(
            tx,
            &request.executor_id,
            schema_epoch,
            &request.promotion,
        )?;
        ingest_override_feedback_tx(tx, request)?;
        Ok(ack)
    })?;

    ensure!(
        ack.visible_after_seq == visible_after_seq,
        "predicted commit fence diverged from committed fence"
    );
    Ok(ack)
}

fn validate_override_feedback_request(request: &OverrideFeedbackRequest) -> Result<()> {
    ensure!(
        !request.executor_id.trim().is_empty(),
        "executor_id must not be empty"
    );
    ensure!(
        !request.validation_verdict_id.trim().is_empty(),
        "validation_verdict_id must not be empty"
    );
    ensure!(
        !request.slice_id.trim().is_empty(),
        "slice_id must not be empty"
    );
    ensure!(
        !request.dominant_failure_family.trim().is_empty(),
        "dominant_failure_family must not be empty"
    );
    ensure!(
        request.promotion.override_verdict.as_deref() == Some("approved"),
        "override feedback only records operator approvals of rejected provisional output"
    );
    ensure!(
        request
            .promotion
            .override_reason
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
        "override feedback requires override_reason"
    );
    ensure!(
        request
            .promotion
            .override_by
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
        "override feedback requires override_by"
    );
    ensure!(
        request
            .promotion
            .override_at
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
        "override feedback requires override_at"
    );
    Ok(())
}

fn override_feedback_case_exists(
    conn: &rusqlite::Connection,
    validation_verdict_id: &str,
) -> Result<bool> {
    let count: i64 = conn
        .query_row(
            "\
            SELECT COUNT(*)
            FROM validation_calibration_cases
            WHERE validation_verdict_id = ?1
              AND corpus_kind = 'override'
            ",
            params![validation_verdict_id],
            |row| row.get(0),
        )
        .context("checking override calibration feedback case")?;
    Ok(count > 0)
}

fn ingest_override_feedback_tx(
    tx: &Transaction<'_>,
    request: &OverrideFeedbackRequest,
) -> Result<()> {
    let verdict = load_override_verdict(tx, request)?;
    let case_id = override_case_id(&request.validation_verdict_id);
    upsert_override_calibration_case(tx, request, &verdict, &case_id)?;
    upsert_override_incident_cluster(tx, request, &verdict, &case_id)?;
    Ok(())
}

fn load_override_verdict(
    tx: &Transaction<'_>,
    request: &OverrideFeedbackRequest,
) -> Result<OverrideVerdictRow> {
    let verdict = tx
        .query_row(
            "\
            SELECT worker_job_id, verdict, summary, incident_signature, issues_json,
                   quality_assessment_json, skill_improvement_suggestions_json,
                   override_verdict, override_reason, override_by, override_at
            FROM validation_verdicts
            WHERE id = ?1
            ",
            params![request.validation_verdict_id],
            |row| {
                Ok(OverrideVerdictRow {
                    worker_job_id: row.get(0)?,
                    verdict: row.get(1)?,
                    summary: row.get(2)?,
                    incident_signature: row.get(3)?,
                    issues_json: row.get(4)?,
                    quality_assessment_json: row.get(5)?,
                    skill_improvement_suggestions_json: row.get(6)?,
                    override_verdict: row.get(7)?,
                    override_reason: row.get(8)?,
                    override_by: row.get(9)?,
                    override_at: row.get(10)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "validation verdict missing for override `{}`",
                request.validation_verdict_id
            )
        })?;
    ensure!(
        verdict.worker_job_id == request.promotion.worker_job_id,
        "override verdict `{}` is linked to worker `{}`, not `{}`",
        request.validation_verdict_id,
        verdict.worker_job_id,
        request.promotion.worker_job_id
    );
    ensure!(
        verdict.verdict == "rejected",
        "override feedback expected rejected verdict, got `{}`",
        verdict.verdict
    );
    ensure!(
        verdict.override_verdict == "approved",
        "override audit was not recorded before feedback ingestion"
    );
    Ok(verdict)
}

fn upsert_override_calibration_case(
    tx: &Transaction<'_>,
    request: &OverrideFeedbackRequest,
    verdict: &OverrideVerdictRow,
    case_id: &str,
) -> Result<()> {
    let metadata_json = json!({
        "case_id": case_id,
        "source": "operator_override",
        "validation_verdict_id": request.validation_verdict_id,
        "worker_job_id": verdict.worker_job_id,
        "summary": verdict.summary,
        "override_reason": verdict.override_reason,
        "override_by": verdict.override_by,
        "override_at": verdict.override_at,
        "original_verdict": verdict.verdict,
        "override_verdict": verdict.override_verdict,
        "override_estimator_policy": "biased_demotion_signal_only",
        "override_can_trigger_demotion": true,
        "override_is_sole_precision_estimator": false,
        "quality_assessment": parse_json_or_wrapped(verdict.quality_assessment_json.as_deref()),
        "issues": parse_json_or_wrapped(Some(verdict.issues_json.as_str())),
        "skill_improvement_suggestions": parse_json_or_wrapped(
            verdict.skill_improvement_suggestions_json.as_deref()
        ),
        "candidate_mutation_template": request.candidate_mutation_template_json,
    })
    .to_string();
    tx.execute(
        "\
        INSERT INTO validation_calibration_cases (
          case_id, slice_id, corpus_kind, source_artifact_path,
          expected_verdict, actual_verdict, ground_truth_label,
          mutation_family, worker_job_id, validation_verdict_id,
          metadata_json, adjudicated_at
        ) VALUES (?1, ?2, 'override', ?3, ?4, ?5, 'operator_override',
                  ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(case_id) DO UPDATE SET
          slice_id = excluded.slice_id,
          source_artifact_path = excluded.source_artifact_path,
          expected_verdict = excluded.expected_verdict,
          actual_verdict = excluded.actual_verdict,
          ground_truth_label = excluded.ground_truth_label,
          mutation_family = excluded.mutation_family,
          worker_job_id = excluded.worker_job_id,
          validation_verdict_id = excluded.validation_verdict_id,
          metadata_json = excluded.metadata_json,
          adjudicated_at = excluded.adjudicated_at
        ",
        params![
            case_id,
            request.slice_id,
            request.source_artifact_path.as_deref(),
            verdict.verdict,
            verdict.override_verdict,
            request.dominant_failure_family,
            verdict.worker_job_id,
            request.validation_verdict_id,
            metadata_json,
            verdict.override_at,
        ],
    )
    .context("upserting override validation calibration case")?;
    Ok(())
}

fn upsert_override_incident_cluster(
    tx: &Transaction<'_>,
    request: &OverrideFeedbackRequest,
    verdict: &OverrideVerdictRow,
    case_id: &str,
) -> Result<()> {
    let incident_signature = incident_signature(request, verdict);
    let existing = load_incident_cluster(tx, &incident_signature)?;
    let affected_slice_ids_json = merged_json_array(
        existing
            .as_ref()
            .map(|row| row.affected_slice_ids_json.as_str()),
        &request.slice_id,
    )?;
    let override_case_ids_json = merged_json_array(
        existing
            .as_ref()
            .map(|row| row.override_case_ids_json.as_str()),
        case_id,
    )?;
    let override_count = json_array_len(&override_case_ids_json)?;
    let calibration_case_count = existing
        .map(|row| row.calibration_case_count)
        .unwrap_or_default()
        .max(override_count);
    let candidate_mutation_template_json = json!({
        "source": "operator_override",
        "incident_signature": incident_signature,
        "validation_verdict_id": request.validation_verdict_id,
        "override_case_id": case_id,
        "dominant_failure_family": request.dominant_failure_family,
        "override_reason": verdict.override_reason,
        "override_estimator_policy": "biased_demotion_signal_only",
        "override_is_sole_precision_estimator": false,
        "template": request.candidate_mutation_template_json,
        "issues": parse_json_or_wrapped(Some(verdict.issues_json.as_str())),
        "skill_improvement_suggestions": parse_json_or_wrapped(
            verdict.skill_improvement_suggestions_json.as_deref()
        ),
    })
    .to_string();
    tx.execute(
        "\
        INSERT INTO validation_incident_clusters (
          incident_signature, dominant_failure_family, affected_slice_ids_json,
          candidate_mutation_template_json, override_case_ids_json,
          calibration_case_count, override_count, first_seen_at, last_seen_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
        ON CONFLICT(incident_signature) DO UPDATE SET
          dominant_failure_family = excluded.dominant_failure_family,
          affected_slice_ids_json = excluded.affected_slice_ids_json,
          candidate_mutation_template_json = excluded.candidate_mutation_template_json,
          override_case_ids_json = excluded.override_case_ids_json,
          calibration_case_count = MAX(
            validation_incident_clusters.calibration_case_count,
            excluded.calibration_case_count
          ),
          override_count = excluded.override_count,
          last_seen_at = excluded.last_seen_at,
          updated_at = CURRENT_TIMESTAMP
        ",
        params![
            incident_signature,
            request.dominant_failure_family,
            affected_slice_ids_json,
            candidate_mutation_template_json,
            override_case_ids_json,
            calibration_case_count,
            override_count,
            verdict.override_at,
        ],
    )
    .context("upserting override validation incident cluster")?;
    Ok(())
}

fn load_incident_cluster(
    tx: &Transaction<'_>,
    incident_signature: &str,
) -> Result<Option<IncidentClusterState>> {
    tx.query_row(
        "\
        SELECT affected_slice_ids_json, override_case_ids_json, calibration_case_count
        FROM validation_incident_clusters
        WHERE incident_signature = ?1
        ",
        params![incident_signature],
        |row| {
            Ok(IncidentClusterState {
                affected_slice_ids_json: row.get(0)?,
                override_case_ids_json: row.get(1)?,
                calibration_case_count: row.get(2)?,
            })
        },
    )
    .optional()
    .context("loading validation incident cluster")
}

fn incident_signature(request: &OverrideFeedbackRequest, verdict: &OverrideVerdictRow) -> String {
    verdict.incident_signature.clone().unwrap_or_else(|| {
        format!(
            "override-{}-{}",
            request.dominant_failure_family, request.slice_id
        )
    })
}

fn override_case_id(validation_verdict_id: &str) -> String {
    format!("override-{validation_verdict_id}")
}

fn parse_json_or_wrapped(raw_json: Option<&str>) -> Value {
    raw_json.map_or(Value::Null, |raw| {
        serde_json::from_str(raw).unwrap_or_else(|_| json!({ "raw": raw }))
    })
}

fn merged_json_array(existing_json: Option<&str>, next_value: &str) -> Result<String> {
    let mut values: BTreeSet<String> = existing_json
        .map(|raw| serde_json::from_str::<Vec<String>>(raw).unwrap_or_default())
        .unwrap_or_default()
        .into_iter()
        .collect();
    values.insert(next_value.to_owned());
    serde_json::to_string(&values.into_iter().collect::<Vec<_>>())
        .context("serializing merged JSON array")
}

fn json_array_len(raw_json: &str) -> Result<i64> {
    let values: Vec<String> =
        serde_json::from_str(raw_json).context("parsing merged JSON array")?;
    i64::try_from(values.len()).context("JSON array length exceeds i64")
}
