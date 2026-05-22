#![forbid(unsafe_code)]

#[allow(clippy::missing_const_for_fn)]
#[path = "../src/errors/manifest.rs"]
mod errors_manifest_impl;
#[allow(clippy::missing_const_for_fn)]
#[path = "../src/errors/markdown.rs"]
mod errors_markdown_impl;
#[allow(clippy::missing_const_for_fn)]
#[path = "../src/db/sqlite.rs"]
mod sqlite_impl;

use anyhow::Result;
use chrono::Utc;
use rusqlite::params;
use serde_json::json;

use crate::errors_manifest_impl::{
    ErrorCode, ErrorManifestDraft, ErrorManifestStore, ErrorRuntimeContext,
};
use crate::sqlite_impl::{DEFAULT_OWNER_ID, DEFAULT_SCHEMA_EPOCH, StateDb};

fn open_temp_db() -> Result<(tempfile::TempDir, StateDb)> {
    let temp = tempfile::tempdir()?;
    let db_path = temp.path().join("state.db");
    let state_db = sqlite_impl::open_state_db(&db_path, DEFAULT_SCHEMA_EPOCH)?;
    Ok((temp, state_db))
}

fn seed_process(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO procurement_processes (
          id, canonical_process_id, current_stage, title, latest_published_at,
          latest_updated_at, owner_id
        ) VALUES ('process-1', 'process-1', 'review', 'Seed process',
                  '2026-03-28T09:00:00Z', '2026-03-28T09:00:00Z', ?1)
        ",
        params![DEFAULT_OWNER_ID],
    )?;
    Ok(())
}

fn draft(error_type: ErrorCode) -> ErrorManifestDraft {
    ErrorManifestDraft {
        job_id: None,
        process_id: Some("process-1".to_owned()),
        pipeline_stage: "Enrich".to_owned(),
        workflow_step: "supplier_lookup".to_owned(),
        error_type,
        error_message: "Failure details".to_owned(),
        error_context: json!({ "step": "supplier_lookup" }),
        skill_path: Some("skills/procurement-ingestion/fts/SKILL.md".to_owned()),
        agent_session_id: None,
        runtime_context: ErrorRuntimeContext {
            cli_vendor: Some("fixture-cli".to_owned()),
            cli_version: Some("1.0.0".to_owned()),
            adapter_version: Some("daemon-test".to_owned()),
            model_id: None,
            config_hash: Some("config-hash".to_owned()),
            skill_version: Some("1.0.0".to_owned()),
            pid: Some(123),
            pgid: Some(123),
            cleanup_verdict: Some("forced".to_owned()),
            last_valid_checkpoint_sequence: Some(2),
            daemon_instance_id: Some("daemon-1".to_owned()),
            commit_seq: Some(10),
        },
        session_artifact_path: Some("artifacts/session.log".to_owned()),
        next_debugging_action: "Inspect the session transcript".to_owned(),
    }
}

const fn l5_operator_alert_cases() -> [(ErrorCode, &'static str); 10] {
    [
        (
            ErrorCode::ClassificationAmbiguous,
            "block auto-qualification and escalate classification to a broker",
        ),
        (
            ErrorCode::TariffBranchDivergence,
            "treat this as a hard blocker, not a scenario band",
        ),
        (
            ErrorCode::MeasureInteractionUnknown,
            "require manual customs review before any auto-decision",
        ),
        (
            ErrorCode::ExporterUnresolved,
            "apply the residual rate and alert the operator",
        ),
        (
            ErrorCode::SupplementaryUnitMissing,
            "abstain on that duty component until evidence exists",
        ),
        (
            ErrorCode::ValuationModeConflict,
            "preserve the uncertainty range for operator review",
        ),
        (
            ErrorCode::FpoAccessUnavailable,
            "continue with remaining recall sources and apply the confidence penalty",
        ),
        (
            ErrorCode::TradeRemedyWatchStale,
            "reclassify before relying on landed cost",
        ),
        (
            ErrorCode::FxRateMissing,
            "block landed-cost computation until the customs rate is present",
        ),
        (
            ErrorCode::FreightEstimateFailed,
            "require a manual freight estimate before decision-grade margin use",
        ),
    ]
}

const fn l6_bid_operator_alert_cases() -> [(ErrorCode, &'static str); 6] {
    [
        (
            ErrorCode::BidSubmissionFailed,
            "could not prove draft save or final submission",
        ),
        (
            ErrorCode::BidDuplicateDetected,
            "already shows a draft or submission for this opportunity",
        ),
        (
            ErrorCode::BidSideEffectCapExceeded,
            "attempted to exceed the approved side-effect cap",
        ),
        (
            ErrorCode::BidReadinessGateFailed,
            "bid-readiness prerequisites failed",
        ),
        (
            ErrorCode::BidPortalChanged,
            "Procontract bid portal shape no longer matches",
        ),
        (
            ErrorCode::BidPortalAuthExpired,
            "Procontract session expired during bid automation",
        ),
    ]
}

fn bid_error_context() -> serde_json::Value {
    json!({
        "portalOpportunityRef": "DN-OPP-123",
        "selector": "#response-form button.submit",
        "expectedSourceStructureVersion": "procontract-bid-portal-2026-05-22",
        "observedShape": {
            "formFingerprint": "changed",
            "lastUrl": "https://procontract.due-north.com/Opportunities/Index",
        },
        "bidScopePayload": {
            "processId": "process-1",
            "lotIds": ["lot-1"],
            "candidateIds": ["candidate-1"],
            "selectedQuoteIds": ["quote-1"],
            "operatorEmail": "operator-secret:l6@example.test",
            "companyProfileVersion": "company-profile-v7",
        },
        "companyProfile": {
            "registrationNumber": "registration-secret:12345678",
            "vatNumber": "vat-secret:GB123456789",
            "primaryContact": {
                "email": "sales-secret@sorelon.example",
                "phone": "+44-secret-phone",
            },
            "address": {
                "line1": "secret-address-line",
                "postcode": "SW1A-secret",
            },
        },
    })
}

#[test]
fn ag_err_001_store_create_persists_error_manifest_row_and_markdown() -> Result<()> {
    let (temp, state_db) = open_temp_db()?;
    seed_process(state_db.connection())?;
    let store = ErrorManifestStore::new(temp.path().join("artifacts"))?;

    let stored = store.create(
        state_db.connection(),
        &draft(ErrorCode::AgentSkillFailed),
        Utc::now(),
    )?;

    let count: i64 = state_db.connection().query_row(
        "SELECT COUNT(*) FROM error_manifests WHERE id = ?1",
        params![stored.id],
        |row| row.get(0),
    )?;
    assert_eq!(count, 1);
    assert!(stored.markdown_path.exists());
    Ok(())
}

#[test]
fn ag_err_002_resolve_is_idempotent_for_existing_manifest() -> Result<()> {
    let (temp, state_db) = open_temp_db()?;
    seed_process(state_db.connection())?;
    let store = ErrorManifestStore::new(temp.path().join("artifacts"))?;
    let stored = store.create(
        state_db.connection(),
        &draft(ErrorCode::AgentOutputContractInvalid),
        Utc::now(),
    )?;

    ErrorManifestStore::resolve(state_db.connection(), &stored.id, "desktop", Utc::now())?;
    let resolved: i64 = state_db.connection().query_row(
        "SELECT resolved FROM error_manifests WHERE id = ?1",
        params![stored.id],
        |row| row.get(0),
    )?;
    assert_eq!(resolved, 1);
    Ok(())
}

#[test]
fn ag_err_003_error_types_round_trip_for_rerunnable_failures() -> Result<()> {
    let (temp, state_db) = open_temp_db()?;
    seed_process(state_db.connection())?;
    let store = ErrorManifestStore::new(temp.path().join("artifacts"))?;
    let stored = store.create(
        state_db.connection(),
        &draft(ErrorCode::AgentSuspectOutput),
        Utc::now(),
    )?;

    let record = ErrorManifestStore::load_record(state_db.connection(), &stored.id)?;
    assert_eq!(record.error_type, ErrorCode::AgentSuspectOutput);
    assert_eq!(record.pipeline_stage, "Enrich");
    Ok(())
}

#[test]
fn ag_err_005_agent_suspect_output_manifest_contains_operator_alert() -> Result<()> {
    let (temp, state_db) = open_temp_db()?;
    seed_process(state_db.connection())?;
    let store = ErrorManifestStore::new(temp.path().join("artifacts"))?;
    let stored = store.create(
        state_db.connection(),
        &draft(ErrorCode::AgentSuspectOutput),
        Utc::now(),
    )?;

    let markdown = std::fs::read_to_string(&stored.markdown_path)?;
    assert!(markdown.contains("## Operator Alert"));
    assert!(markdown.contains("AGENT_SUSPECT_OUTPUT"));
    assert!(markdown.contains("Do not promote or outbox this worker output"));
    Ok(())
}

#[test]
fn l5_error_manifest_operator_alerts_cover_decision_grade_codes() -> Result<()> {
    let (temp, state_db) = open_temp_db()?;
    seed_process(state_db.connection())?;
    let store = ErrorManifestStore::new(temp.path().join("artifacts"))?;

    for (error_code, expected_phrase) in l5_operator_alert_cases() {
        let stored = store.create(state_db.connection(), &draft(error_code), Utc::now())?;
        let markdown = std::fs::read_to_string(&stored.markdown_path)?;

        assert!(markdown.contains("## Operator Alert"));
        assert!(markdown.contains(error_code.as_str()));
        assert!(
            markdown.contains(expected_phrase),
            "operator alert for {} should explain `{expected_phrase}`",
            error_code.as_str()
        );
    }

    Ok(())
}

#[test]
fn l6_error_manifest_operator_alerts_cover_bid_codes_and_redact_context() -> Result<()> {
    let (temp, state_db) = open_temp_db()?;
    seed_process(state_db.connection())?;
    let store = ErrorManifestStore::new(temp.path().join("artifacts"))?;
    let sensitive_markers = [
        "operator-secret:l6@example.test",
        "registration-secret:12345678",
        "vat-secret:GB123456789",
        "sales-secret@sorelon.example",
        "+44-secret-phone",
        "secret-address-line",
        "SW1A-secret",
    ];

    for (error_code, expected_phrase) in l6_bid_operator_alert_cases() {
        let mut draft = draft(error_code);
        draft.pipeline_stage = "Bid workflow".to_owned();
        draft.workflow_step = "portal_submit".to_owned();
        draft.error_context = bid_error_context();
        draft.skill_path = Some("skills/bid-portal-procontract/SKILL.md".to_owned());
        let stored = store.create(state_db.connection(), &draft, Utc::now())?;
        let record = ErrorManifestStore::load_record(state_db.connection(), &stored.id)?;
        let markdown = std::fs::read_to_string(&stored.markdown_path)?;
        let stored_context = record.error_context.to_string();

        assert_eq!(record.error_type, error_code);
        assert!(markdown.contains("## Operator Alert"));
        assert!(markdown.contains(error_code.as_str()));
        assert!(
            markdown.contains(expected_phrase),
            "operator alert for {} should explain `{expected_phrase}`",
            error_code.as_str()
        );
        assert!(
            markdown.contains("DN-OPP-123")
                && markdown.contains("#response-form button.submit")
                && markdown.contains("procontract-bid-portal-2026-05-22")
                && markdown.contains("https://procontract.due-north.com/Opportunities/Index"),
            "bid context should preserve portal-safe diagnostics for {}",
            error_code.as_str()
        );
        for marker in sensitive_markers {
            assert!(
                !markdown.contains(marker) && !stored_context.contains(marker),
                "{} leaked sensitive marker `{marker}`",
                error_code.as_str()
            );
        }
    }

    Ok(())
}

#[test]
fn ag_err_004_cleanup_and_session_linkage_remain_visible_to_ui_consumers() -> Result<()> {
    let (temp, state_db) = open_temp_db()?;
    seed_process(state_db.connection())?;
    state_db.connection().execute(
        "\
        INSERT INTO agent_session_logs (
          id, skill_path, execution_mode, status
        ) VALUES ('session-1', 'skills/procurement-ingestion/fts/SKILL.md', 'pty', 'failed')
        ",
        [],
    )?;
    let store = ErrorManifestStore::new(temp.path().join("artifacts"))?;
    let mut draft = draft(ErrorCode::AgentCleanupIncomplete);
    draft.agent_session_id = Some("session-1".to_owned());
    let stored = store.create(state_db.connection(), &draft, Utc::now())?;

    let record = ErrorManifestStore::load_record(state_db.connection(), &stored.id)?;
    assert_eq!(record.agent_session_id.as_deref(), Some("session-1"));
    assert_eq!(record.process_id.as_deref(), Some("process-1"));
    Ok(())
}
