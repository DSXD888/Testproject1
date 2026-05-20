#![forbid(unsafe_code)]

mod support;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};

use support::db::{
    LogicalColumn, LogicalSchema, TestStateDb, logical_schema_for_tables, logical_schema_hash,
    normalize_sql_type,
};
use support::supabase::SupabaseLocalStack;

const PUSH_OWNED_TABLES: &[&str] = &[
    "procurement_processes",
    "contract_notices",
    "notice_lots",
    "tender_document_sets",
    "qualification_runs",
    "product_candidates",
    "search_profiles",
    "search_attempts",
    "supplier_quotes",
    "candidate_pricing_assessments",
    "landed_cost_estimates",
    "manifests",
    "pipeline_assessments",
    "bid_workflow_states",
    "process_stage_status",
    "process_stage_heads",
    "process_relations",
    "process_aliases",
    "document_extractions",
    "document_semantic_extractions",
];

const LOCAL_ORIGIN_SUMMARY_TABLES: &[&str] = &[
    "workflow_job_summaries",
    "error_manifest_summaries",
    "validation_verdict_summaries",
    "viewer_sync_status",
    "executors",
];

const VIEWER_VISIBLE_SUMMARY_TABLES: &[&str] = &[
    "workflow_job_summaries",
    "error_manifest_summaries",
    "validation_verdict_summaries",
    "viewer_sync_status",
    "executors",
    "operator_decision_summaries",
];

const SYNC_TRACKING_COLUMNS: &[&str] = &[
    "owner_id",
    "row_version",
    "last_local_seq",
    "stage_run_id",
    "created_at",
    "updated_at",
];

#[test]
fn l4_t_redaction_boundary_001_push_owned_projection_role_matrix() -> Result<()> {
    let rls = read_repo_file("infra/supabase/migrations/0004_rls_policies.sql")?;
    let rpc = read_repo_file("infra/supabase/migrations/0005_sync_rpcs.sql")?;

    for table in PUSH_OWNED_TABLES {
        ensure!(
            rls.contains(&format!("'{table}'")),
            "L4-T-redaction_boundary-001 missing push-owned projection table `{table}` in RLS policy loop"
        );
        ensure!(
            rpc.contains(&format!("'{table}'")),
            "L4-T-redaction_boundary-001 missing push-owned projection table `{table}` in apply_push_batch allow-list"
        );
    }
    ensure!(
        rls.contains("FOR SELECT USING (l4_is_any_role(ARRAY[''viewer'',''operator'',''admin'']))"),
        "L4-T-redaction_boundary-001 push-owned projection SELECT policy must allow viewer/operator/admin"
    );
    assert_no_direct_projection_mutation_grants(&rls)?;
    assert_apply_push_batch_daemon_admin_only(&rpc, "L4-T-redaction_boundary-001")?;

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_002_push_owned_summary_role_matrix() -> Result<()> {
    let rls = read_repo_file("infra/supabase/migrations/0004_rls_policies.sql")?;
    let rpc = read_repo_file("infra/supabase/migrations/0005_sync_rpcs.sql")?;

    for table in LOCAL_ORIGIN_SUMMARY_TABLES {
        ensure!(
            rls.contains(&format!("'{table}'")),
            "L4-T-redaction_boundary-002 missing local-origin summary table `{table}` in RLS policy loop"
        );
        if *table != "viewer_sync_status" {
            ensure!(
                rpc.contains(&format!("'{table}'")),
                "L4-T-redaction_boundary-002 missing local-origin summary table `{table}` in apply_push_batch allow-list"
            );
        }
    }
    ensure!(
        rls.contains("FOR SELECT USING (l4_is_any_role(ARRAY[''viewer'',''operator'',''admin'']))"),
        "L4-T-redaction_boundary-002 summary SELECT policy must allow viewer/operator/admin"
    );
    assert_apply_push_batch_daemon_admin_only(&rpc, "L4-T-redaction_boundary-002")?;

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_003_shared_company_profile_remote_config_matrix() -> Result<()> {
    assert_remote_config_policy("shared_company_profile", "L4-T-redaction_boundary-003")
}

#[test]
fn l4_t_redaction_boundary_004_shared_source_configs_remote_config_matrix() -> Result<()> {
    assert_remote_config_policy("shared_source_configs", "L4-T-redaction_boundary-004")
}

#[test]
fn l4_t_redaction_boundary_005_operator_decision_heads_role_matrix() -> Result<()> {
    let rls = read_repo_file("infra/supabase/migrations/0004_rls_policies.sql")?;
    let section = rls
        .split("CREATE POLICY operator_decision_heads_operator_admin_daemon_select")
        .nth(1)
        .and_then(|section| {
            section
                .split("CREATE POLICY operator_decision_heads_admin_all")
                .next()
        })
        .context("L4-T-redaction_boundary-005 missing operator_decision_heads SELECT policy")?;

    ensure!(
        section.contains("l4_request_role() = 'daemon'")
            && section.contains("l4_request_role() = 'admin'")
            && section.contains("(l4_request_role() = 'operator' AND deleted_at IS NULL)")
            && !section.contains("l4_request_role() = 'viewer'"),
        "L4-T-redaction_boundary-005 raw decision heads must deny viewer, allow operator non-tombstone SELECT, and allow admin/daemon pull SELECT"
    );
    ensure!(
        rls.contains("CREATE POLICY operator_decision_heads_admin_all")
            && rls.contains("FOR ALL")
            && rls.contains("USING (l4_request_role() = 'admin')")
            && rls.contains("WITH CHECK (l4_request_role() = 'admin')"),
        "L4-T-redaction_boundary-005 admin CRUD policy missing for operator_decision_heads"
    );
    ensure!(
        !rls.contains("operator_decision_heads_operator_update")
            && !rls.contains("operator_decision_heads_operator_all"),
        "L4-T-redaction_boundary-005 operator direct UPDATE policy must stay absent"
    );

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_006_operator_decision_summaries_role_matrix() -> Result<()> {
    let decision_summaries =
        read_repo_file("infra/supabase/migrations/0008_decision_summaries.sql")?;
    let rpc = read_repo_file("infra/supabase/migrations/0005_sync_rpcs.sql")?;

    ensure!(
        decision_summaries
            .contains("ALTER TABLE operator_decision_summaries ENABLE ROW LEVEL SECURITY")
            && decision_summaries
                .contains("CREATE POLICY operator_decision_summaries_viewer_operator_admin_select")
            && decision_summaries
                .contains("USING (l4_is_any_role(ARRAY['viewer', 'operator', 'admin']))"),
        "L4-T-redaction_boundary-006 decision summary projection must be viewer/operator/admin readable"
    );
    ensure!(
        rpc.contains("'operator_decision_summaries'"),
        "L4-T-redaction_boundary-006 decision summary projection must be daemon-writeable through apply_push_batch"
    );
    assert_summary_projection_redaction(
        "operator_decision_summaries",
        &[
            "process_id",
            "org_id",
            "disposition",
            "decision_stale",
            "reviewed_at_stage_run_id",
            "updated_at",
        ],
        &[
            "operator_id",
            "company_name",
            "registration_number",
            "vat_number",
            "command_id",
            "decision_notes",
            "reviewed_basis_hash",
            "reviewed_manifest_hash",
            "remote_revision",
            "remote_change_seq",
        ],
        "L4-T-redaction_boundary-006",
    )
}

#[test]
fn l4_t_redaction_boundary_007_sync_infrastructure_role_matrix() -> Result<()> {
    let sync_metadata = read_repo_file("infra/supabase/migrations/0006_schema_metadata.sql")?;

    for table in ["schema_metadata", "active_writer_lease"] {
        ensure!(
            sync_metadata.contains(&format!("ALTER TABLE {table} ENABLE ROW LEVEL SECURITY")),
            "L4-T-redaction_boundary-007 `{table}` must enable RLS"
        );
        ensure!(
            sync_metadata.contains(&format!("CREATE POLICY {table}_admin_daemon_select"))
                && sync_metadata.contains("USING (l4_is_any_role(ARRAY['admin', 'daemon']))"),
            "L4-T-redaction_boundary-007 `{table}` SELECT must be admin/daemon only"
        );
    }
    ensure!(
        sync_metadata.contains("CREATE POLICY schema_metadata_admin_daemon_update")
            && sync_metadata.contains("CREATE POLICY active_writer_lease_admin_daemon_insert")
            && sync_metadata.contains("CREATE POLICY active_writer_lease_admin_daemon_update"),
        "L4-T-redaction_boundary-007 sync infrastructure mutation policies must be admin/daemon only"
    );
    let policy_section = sync_metadata
        .split("ALTER TABLE schema_metadata ENABLE ROW LEVEL SECURITY;")
        .nth(1)
        .context("L4-T-redaction_boundary-007 missing sync infrastructure policy section")?;
    ensure!(
        !policy_section.contains("l4_request_role() = 'viewer'")
            && !policy_section.contains("l4_request_role() = 'operator'")
            && !policy_section.contains("ARRAY['viewer'")
            && !policy_section.contains("ARRAY['operator'"),
        "L4-T-redaction_boundary-007 sync infrastructure policies must not mention viewer/operator"
    );

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_008_operator_decision_commands_role_matrix() -> Result<()> {
    let ledger = read_repo_file("infra/supabase/migrations/0007_command_ledger.sql")?;
    let rpc = read_repo_file("infra/supabase/migrations/0005_sync_rpcs.sql")?;

    ensure!(
        ledger.contains("ALTER TABLE operator_decision_commands ENABLE ROW LEVEL SECURITY")
            && ledger.contains("CREATE POLICY operator_decision_commands_admin_select")
            && ledger.contains("USING (l4_request_role() = 'admin')")
            && ledger.contains("CREATE POLICY operator_decision_commands_daemon_select")
            && ledger.contains("USING (l4_request_role() = 'daemon')"),
        "L4-T-redaction_boundary-008 command ledger must be admin/daemon SELECT only"
    );
    ensure!(
        !ledger.contains("operator_decision_commands_viewer")
            && !ledger.contains("operator_decision_commands_operator")
            && !ledger.contains("l4_request_role() = 'viewer'")
            && !ledger.contains("l4_request_role() = 'operator'"),
        "L4-T-redaction_boundary-008 command ledger must deny viewer/operator"
    );
    ensure!(
        rpc.contains("INSERT INTO operator_decision_commands")
            && rpc.contains("l4_is_any_role(ARRAY['operator', 'admin'])")
            && rpc.contains("SECURITY DEFINER"),
        "L4-T-redaction_boundary-008 daemon/CAS command writes must occur through security-definer RPC behavior"
    );

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_009_viewer_credentials_are_read_only() -> Result<()> {
    let rls = read_repo_file("infra/supabase/migrations/0004_rls_policies.sql")?;
    let rpc = read_repo_file("infra/supabase/migrations/0005_sync_rpcs.sql")?;
    let web_supabase = read_repo_file("apps/web/src/lib/supabase.ts")?;

    ensure!(
        web_supabase.contains("NEXT_PUBLIC_SUPABASE_URL")
            && web_supabase.contains("NEXT_PUBLIC_SUPABASE_ANON_KEY")
            && !web_supabase.contains("SUPABASE_SERVICE_ROLE")
            && !web_supabase.contains("SERVICE_ROLE"),
        "L4-T-redaction_boundary-009 web Supabase boundary must use viewer anon credentials only"
    );
    ensure!(
        !web_supabase.contains("apply_push_batch")
            && !web_supabase.contains("update_operator_decision")
            && !web_supabase.contains(".insert(")
            && !web_supabase.contains(".update(")
            && !web_supabase.contains(".delete("),
        "L4-T-redaction_boundary-009 web viewer boundary must not expose mutations"
    );
    ensure!(
        rls.contains("FOR SELECT USING (l4_is_any_role(ARRAY[''viewer'',''operator'',''admin'']))")
            && !rls.contains("l4_request_role() = ''viewer'' OR l4_request_role() = ''admin''")
            && !rls.contains("FOR ALL USING (l4_request_role() = ''viewer'')"),
        "L4-T-redaction_boundary-009 viewer role must be SELECT-only on projection/summary policies"
    );
    ensure!(
        rpc.contains("apply_push_batch")
            && rpc.contains("update_operator_decision")
            && !rpc.contains("l4_is_any_role(ARRAY['viewer'"),
        "L4-T-redaction_boundary-009 mutation RPCs must not admit viewer role"
    );

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_010_workflow_job_summary_redaction() -> Result<()> {
    assert_summary_projection_redaction(
        "workflow_job_summaries",
        &[
            "job_id",
            "org_id",
            "job_kind",
            "status",
            "skill_path",
            "started_at",
            "completed_at",
            "error_type",
            "updated_at",
        ],
        &[
            "payload_json",
            "full_payload_json",
            "prompt",
            "raw_session_context",
            "session_transcript",
            "context_json",
            "local_artifact_path",
            "stdout",
            "stderr",
        ],
        "L4-T-redaction_boundary-010",
    )
}

#[test]
fn l4_t_redaction_boundary_011_error_manifest_summary_redaction() -> Result<()> {
    assert_summary_projection_redaction(
        "error_manifest_summaries",
        &[
            "manifest_id",
            "org_id",
            "job_id",
            "error_type",
            "error_code",
            "incident_signature",
            "updated_at",
        ],
        &[
            "markdown",
            "body_md",
            "context_json",
            "runtime_context",
            "linked_artifact_details",
            "local_artifact_path",
            "stack_trace",
            "prompt",
        ],
        "L4-T-redaction_boundary-011",
    )
}

#[test]
fn l4_t_redaction_boundary_012_validation_verdict_summary_redaction() -> Result<()> {
    assert_summary_projection_redaction(
        "validation_verdict_summaries",
        &[
            "verdict_id",
            "org_id",
            "worker_job_id",
            "verdict",
            "basis_sufficiency",
            "enforcement_eligible",
            "updated_at",
        ],
        &[
            "validator_reasoning",
            "reasoning_json",
            "full_reasoning",
            "issues_json",
            "prompt_evidence",
            "quality_suggestions",
            "evidence_json",
        ],
        "L4-T-redaction_boundary-012",
    )
}

#[test]
fn l4_t_redaction_boundary_013_viewer_sync_status_redaction() -> Result<()> {
    assert_summary_projection_redaction(
        "viewer_sync_status",
        &[
            "org_id",
            "sync_health",
            "last_push_success_at",
            "last_push_attempt_at",
            "last_acked_local_seq",
            "pending_outbox_count",
            "schema_epoch",
            "projection_generation",
            "daemon_version",
            "updated_at",
        ],
        &[
            "local_db_path",
            "service_key",
            "executor_secret",
            "company_profile",
            "supabase_service_role_key",
            "socket_path",
        ],
        "L4-T-redaction_boundary-013",
    )
}

#[test]
fn l4_t_redaction_boundary_014_executors_redaction() -> Result<()> {
    assert_summary_projection_redaction(
        "executors",
        &[
            "executor_id",
            "org_id",
            "heartbeat_at",
            "daemon_version",
            "started_at",
            "updated_at",
        ],
        &[
            "socket_path",
            "host_username",
            "local_filesystem_path",
            "machine_secret",
            "home_dir",
            "env_json",
        ],
        "L4-T-redaction_boundary-014",
    )
}

#[test]
fn l4_t_redaction_boundary_015_operator_decision_summary_redaction() -> Result<()> {
    assert_summary_projection_redaction(
        "operator_decision_summaries",
        &[
            "process_id",
            "org_id",
            "disposition",
            "decision_stale",
            "reviewed_at_stage_run_id",
            "updated_at",
        ],
        &[
            "operator_id",
            "company_name",
            "registration_number",
            "vat_number",
            "command_id",
            "command_history",
            "decision_notes",
            "reviewed_basis_hash",
            "reviewed_manifest_hash",
        ],
        "L4-T-redaction_boundary-015",
    )
}

#[test]
fn l4_t_redaction_boundary_016_company_profile_sentinels_absent_from_summaries() -> Result<()> {
    assert_forbidden_columns_absent_from_viewer_summaries(
        &[
            "company_name",
            "registration_number",
            "vat_number",
            "address_json",
            "contact_json",
            "bid_profile_json",
        ],
        "L4-T-redaction_boundary-016",
    )
}

#[test]
fn l4_t_redaction_boundary_017_raw_supplier_source_and_tender_data_absent_from_summaries()
-> Result<()> {
    assert_forbidden_columns_absent_from_viewer_summaries(
        &[
            "raw_supplier_url",
            "supplier_url",
            "source_raw_payload",
            "raw_payload_json",
            "tender_document_metadata",
            "documents_json",
            "normalized_payload",
            "source_evidence_json",
            "extraction_internals",
        ],
        "L4-T-redaction_boundary-017",
    )
}

#[test]
fn l4_t_redaction_boundary_018_validator_reasoning_absent_from_summaries() -> Result<()> {
    assert_forbidden_columns_absent_from_viewer_summaries(
        &[
            "validator_reasoning",
            "reasoning",
            "reasoning_json",
            "full_reasoning",
            "issues_json",
            "prompt_evidence",
            "quality_suggestions",
        ],
        "L4-T-redaction_boundary-018",
    )
}

#[test]
fn l4_t_redaction_boundary_019_full_error_context_absent_from_summaries() -> Result<()> {
    assert_forbidden_columns_absent_from_viewer_summaries(
        &[
            "markdown",
            "body_md",
            "context_json",
            "runtime_context",
            "full_error_context",
            "stack_trace",
            "raw_session_context",
        ],
        "L4-T-redaction_boundary-019",
    )
}

#[test]
fn l4_t_redaction_boundary_020_command_ledger_absent_from_summaries_and_viewer_denied() -> Result<()>
{
    let ledger = read_repo_file("infra/supabase/migrations/0007_command_ledger.sql")?;
    assert_forbidden_columns_absent_from_viewer_summaries(
        &[
            "command_id",
            "command_history",
            "base_revision",
            "applied_revision",
            "response_payload_json",
            "rejected_payload",
        ],
        "L4-T-redaction_boundary-020",
    )?;
    ensure!(
        ledger.contains("CREATE POLICY operator_decision_commands_admin_select")
            && ledger.contains("CREATE POLICY operator_decision_commands_daemon_select")
            && !ledger.contains("l4_request_role() = 'viewer'")
            && !ledger.contains("l4_request_role() = 'operator'")
            && !ledger.contains("ARRAY['viewer'")
            && !ledger.contains("ARRAY['operator'"),
        "L4-T-redaction_boundary-020 command ledger must remain blocked from viewer/operator raw SELECT"
    );

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_026_schema_hash_coordination_gate() -> Result<()> {
    let sqlite = TestStateDb::new()?;
    let sqlite_schema = logical_schema_for_tables(sqlite.connection(), PUSH_OWNED_TABLES)?;
    assert_push_owned_tracking_fields("SQLite", &sqlite_schema)?;

    let supabase = SupabaseLocalStack::start_or_reuse()?;
    let applied = supabase.apply_migrations()?;
    ensure!(
        applied.paths.len() == 9,
        "L4-T-redaction_boundary-026 expected all nine Supabase migrations to apply"
    );
    ensure!(
        applied
            .paths
            .iter()
            .any(|path| path.ends_with("0009_l5_field_level_projection.sql")),
        "L4-T-redaction_boundary-026 expected the L5 field-level projection migration to apply"
    );
    let supabase_schema = supabase.logical_schema_from_migrations(PUSH_OWNED_TABLES)?;
    assert_push_owned_tracking_fields("Supabase", &supabase_schema)?;

    let sqlite_hash = logical_schema_hash(&sqlite_schema);
    let supabase_hash = logical_schema_hash(&supabase_schema);
    ensure!(
        sqlite_schema == supabase_schema,
        "L4-T-redaction_boundary-026 logical schema mismatch\nSQLite hash: {sqlite_hash}\nSupabase hash: {supabase_hash}\n{}",
        schema_diff(&sqlite_schema, &supabase_schema)
    );

    let summary_schema = supabase.logical_schema_from_migrations(LOCAL_ORIGIN_SUMMARY_TABLES)?;
    let expected_summary_schema = expected_local_origin_summary_schema();
    let summary_hash = logical_schema_hash(&summary_schema);
    let expected_summary_hash = logical_schema_hash(&expected_summary_schema);
    ensure!(
        summary_schema == expected_summary_schema,
        "L4-T-redaction_boundary-026 summary schema mismatch\nExpected hash: {expected_summary_hash}\nSupabase hash: {summary_hash}\n{}",
        schema_diff(&expected_summary_schema, &summary_schema)
    );

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_021_tombstoned_remote_rows_stay_pull_visible_only() -> Result<()> {
    let rls = read_repo_file("infra/supabase/migrations/0004_rls_policies.sql")?;
    let remote_owned = read_repo_file("infra/supabase/migrations/0002_remote_owned_tables.sql")?;
    let remote_config_policy = rls
        .split("FOREACH table_name IN ARRAY ARRAY[\n    'shared_company_profile'")
        .nth(1)
        .and_then(|section| section.split("END LOOP;").next())
        .context("L4-T-redaction_boundary-021 missing remote config policy loop")?;
    let decision_select_policy = rls
        .split("CREATE POLICY operator_decision_heads_operator_admin_daemon_select")
        .nth(1)
        .and_then(|section| {
            section
                .split("CREATE POLICY operator_decision_heads_admin_all")
                .next()
        })
        .context("L4-T-redaction_boundary-021 missing operator decision select policy")?;

    ensure!(
        remote_config_policy.contains(
            "FOR SELECT USING (l4_request_role() = ''daemon'' OR l4_request_role() = ''admin'')"
        ) && !remote_config_policy.contains("viewer")
            && !remote_config_policy.contains("operator")
            && !remote_config_policy.contains("deleted_at IS NULL"),
        "L4-T-redaction_boundary-021 remote config tombstones must remain visible only to daemon/admin pull"
    );
    ensure!(
        decision_select_policy.contains("l4_request_role() = 'daemon'")
            && decision_select_policy.contains("l4_request_role() = 'admin'")
            && decision_select_policy
                .contains("(l4_request_role() = 'operator' AND deleted_at IS NULL)")
            && !decision_select_policy.contains("l4_request_role() = 'viewer'"),
        "L4-T-redaction_boundary-021 operator decisions must hide tombstones from operator portal reads while daemon/admin can pull them"
    );
    for table in [
        "shared_company_profile",
        "shared_source_configs",
        "operator_decision_heads",
    ] {
        let table_section = remote_owned
            .split(&format!("CREATE TABLE {table} ("))
            .nth(1)
            .and_then(|section| section.split(");").next())
            .with_context(|| {
                format!("L4-T-redaction_boundary-021 missing remote-owned table `{table}`")
            })?;
        ensure!(
            table_section.contains("deleted_at timestamptz"),
            "L4-T-redaction_boundary-021 `{table}` must carry tombstone timestamp for pull retention"
        );
    }

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_022_operator_decision_mutation_requires_cas_rpc() -> Result<()> {
    let rls = read_repo_file("infra/supabase/migrations/0004_rls_policies.sql")?;
    let rpc = read_repo_file("infra/supabase/migrations/0005_sync_rpcs.sql")?;
    let update_operator_decision_rpc = rpc
        .split("CREATE OR REPLACE FUNCTION update_operator_decision")
        .nth(1)
        .context("L4-T-redaction_boundary-022 missing update_operator_decision RPC")?;

    ensure!(
        update_operator_decision_rpc.contains("process_id text")
            && update_operator_decision_rpc.contains("disposition text")
            && update_operator_decision_rpc.contains("base_revision bigint")
            && update_operator_decision_rpc.contains("command_id text")
            && update_operator_decision_rpc.contains("SECURITY DEFINER"),
        "L4-T-redaction_boundary-022 expected update_operator_decision security-definer RPC"
    );
    ensure!(
        update_operator_decision_rpc.contains("l4_is_any_role(ARRAY['operator', 'admin'])")
            && update_operator_decision_rpc.contains("FOR UPDATE")
            && update_operator_decision_rpc.contains("requested_base_revision"),
        "L4-T-redaction_boundary-022 expected operator/admin CAS gate with row-level lock"
    );
    ensure!(
        update_operator_decision_rpc.contains("operator_decision_commands")
            && update_operator_decision_rpc.contains("'status', 'conflict'")
            && update_operator_decision_rpc.contains("'rejected_payload'"),
        "L4-T-redaction_boundary-022 expected durable command ledger and conflict response"
    );
    ensure!(
        !update_operator_decision_rpc.contains("'not_implemented'"),
        "L4-T-redaction_boundary-022 found placeholder update_operator_decision body"
    );

    ensure!(
        rls.contains("CREATE POLICY operator_decision_heads_operator_admin_daemon_select")
            && rls.contains("CREATE POLICY operator_decision_heads_admin_all"),
        "operator_decision_heads should expose select plus admin direct-write policies only"
    );
    ensure!(
        !rls.contains("operator_decision_heads_operator_update")
            && !rls.contains("operator_decision_heads_daemon_update")
            && !rls.contains("operator_decision_heads_operator_all"),
        "operator direct UPDATE policies must stay absent; mutation belongs behind the CAS RPC"
    );
    ensure!(
        rpc.contains(
            "GRANT EXECUTE ON FUNCTION update_operator_decision(text, text, bigint, text)"
        ),
        "authenticated/service-role execution grant for the CAS RPC is missing"
    );

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_024_admin_maintenance_preserves_remote_owned_triggers() -> Result<()> {
    let rls = read_repo_file("infra/supabase/migrations/0004_rls_policies.sql")?;
    let remote_owned = read_repo_file("infra/supabase/migrations/0002_remote_owned_tables.sql")?;
    let remote_config_policy = rls
        .split("FOREACH table_name IN ARRAY ARRAY[\n    'shared_company_profile'")
        .nth(1)
        .and_then(|section| section.split("END LOOP;").next())
        .context("L4-T-redaction_boundary-024 missing remote config admin policy loop")?;
    let decision_admin_policy = rls
        .split("CREATE POLICY operator_decision_heads_admin_all")
        .nth(1)
        .and_then(|section| section.split("GRANT USAGE ON SCHEMA").next())
        .context("L4-T-redaction_boundary-024 missing decision admin policy")?;

    ensure!(
        remote_config_policy.contains(
            "CREATE POLICY %I ON %I FOR ALL USING (l4_request_role() = ''admin'') WITH CHECK (l4_request_role() = ''admin'')"
        ) && decision_admin_policy.contains("FOR ALL")
            && decision_admin_policy.contains("USING (l4_request_role() = 'admin')")
            && decision_admin_policy.contains("WITH CHECK (l4_request_role() = 'admin')"),
        "L4-T-redaction_boundary-024 admin maintenance policies must be explicit remote-owned CRUD paths"
    );
    ensure!(
        rls.contains("GRANT INSERT, UPDATE, DELETE ON shared_company_profile, shared_source_configs, operator_decision_heads TO authenticated;")
            && !rls.contains("GRANT INSERT, UPDATE, DELETE ON procurement_processes TO authenticated")
            && !rls.contains("GRANT INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO authenticated"),
        "L4-T-redaction_boundary-024 authenticated maintenance grants must stay limited to remote-owned config/decision tables"
    );
    ensure!(
        remote_owned.contains("CREATE OR REPLACE FUNCTION l4_remote_owned_touch()")
            && remote_owned.contains("NEW.remote_change_seq := nextval(TG_ARGV[0]::regclass);")
            && remote_owned.contains("NEW.remote_revision := OLD.remote_revision + 1;"),
        "L4-T-redaction_boundary-024 remote-owned trigger must advance change sequence and revision on maintenance updates"
    );
    for (table, sequence) in [
        (
            "shared_company_profile",
            "shared_company_profile_change_seq",
        ),
        ("shared_source_configs", "shared_source_configs_change_seq"),
        (
            "operator_decision_heads",
            "operator_decision_heads_change_seq",
        ),
    ] {
        let trigger = format!(
            "CREATE TRIGGER {table}_remote_change_seq\n  BEFORE INSERT OR UPDATE ON {table}\n  FOR EACH ROW\n  EXECUTE FUNCTION l4_remote_owned_touch('{sequence}');"
        );
        ensure!(
            remote_owned.contains(&trigger),
            "L4-T-redaction_boundary-024 `{table}` must preserve remote_change_seq trigger for admin maintenance"
        );
    }

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_023_daemon_projection_mutation_stays_behind_rpc() -> Result<()> {
    let rls = read_repo_file("infra/supabase/migrations/0004_rls_policies.sql")?;
    let rpc = read_repo_file("infra/supabase/migrations/0005_sync_rpcs.sql")?;
    let apply_push_batch_rpc = rpc
        .split("CREATE OR REPLACE FUNCTION update_operator_decision")
        .next()
        .context("L4-T-redaction_boundary-023 missing apply_push_batch section")?;

    ensure!(
        apply_push_batch_rpc.contains(
            "CREATE OR REPLACE FUNCTION apply_push_batch(batch_json jsonb, writer_epoch integer)"
        ) && apply_push_batch_rpc.contains("SECURITY DEFINER"),
        "L4-T-redaction_boundary-023 expected apply_push_batch security-definer RPC"
    );
    ensure!(
        apply_push_batch_rpc.contains("l4_is_any_role(ARRAY['daemon', 'admin'])")
            && apply_push_batch_rpc.contains("PERFORM l4_apply_push_owned_payload"),
        "L4-T-redaction_boundary-023 expected daemon/admin RPC gate and internal apply helper"
    );
    ensure!(
        !apply_push_batch_rpc.contains("'not_implemented'"),
        "L4-T-redaction_boundary-023 found placeholder apply_push_batch body"
    );

    ensure!(
        rls.contains("FOR SELECT USING (l4_is_any_role(ARRAY[''viewer'',''operator'',''admin'']))"),
        "push-owned projection tables should expose SELECT policies only to viewer/operator/admin"
    );
    ensure!(
        !rls.contains("procurement_processes_daemon_insert")
            && !rls.contains("procurement_processes_daemon_update"),
        "push-owned projection tables must not expose direct daemon INSERT/UPDATE policies"
    );
    ensure!(
        rls.contains(
            "GRANT INSERT, UPDATE, DELETE ON shared_company_profile, shared_source_configs, operator_decision_heads TO authenticated;"
        ),
        "authenticated mutation grant should remain limited to remote-owned/CAS tables"
    );
    ensure!(
        !rls.contains("GRANT INSERT, UPDATE, DELETE ON procurement_processes TO authenticated")
            && !rls.contains(
                "GRANT INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO authenticated"
            ),
        "daemon/operator direct projection table grants must stay absent outside apply_push_batch"
    );

    Ok(())
}

#[test]
fn l4_t_redaction_boundary_025_browser_viewer_credentials_have_no_privilege() -> Result<()> {
    let boundary_source = read_repo_file("apps/web/src/lib/supabase.ts")?;
    let web_sources = web_source_files("apps/web/src")?;

    ensure!(
        boundary_source.contains("NEXT_PUBLIC_SUPABASE_URL")
            && boundary_source.contains("NEXT_PUBLIC_SUPABASE_ANON_KEY"),
        "L4-T-redaction_boundary-025 web boundary must read only viewer-scoped public Supabase env names"
    );
    ensure!(
        !boundary_source.contains("SERVICE_ROLE")
            && !boundary_source.contains("SUPABASE_SERVICE_ROLE")
            && !boundary_source.contains("SUPABASE_SECRET")
            && !boundary_source.contains("ADMIN_KEY"),
        "L4-T-redaction_boundary-025 browser-held boundary must not name privileged Supabase tokens"
    );

    for path in web_sources {
        let source = std::fs::read_to_string(&path)
            .with_context(|| format!("reading web source {}", path.display()))?;
        let touches_supabase = source.contains("@supabase/supabase-js")
            || source.contains("createClient<")
            || source.contains("createClient(");
        if touches_supabase {
            let relative = path
                .strip_prefix(repo_root()?)
                .unwrap_or(path.as_path())
                .to_string_lossy();
            ensure!(
                relative == "apps/web/src/lib/supabase.ts",
                "L4-T-redaction_boundary-025 Supabase client touchpoint leaked outside apps/web/src/lib/supabase.ts: {relative}"
            );
        }
        ensure!(
            !source.contains("apply_push_batch")
                && !source.contains("update_operator_decision")
                && !source.contains("acquire_active_writer_lease"),
            "L4-T-redaction_boundary-025 web source {} must not call privileged mutation RPCs",
            path.display()
        );
    }

    Ok(())
}

fn assert_push_owned_tracking_fields(source: &str, schema: &LogicalSchema) -> Result<()> {
    ensure!(
        schema.len() == PUSH_OWNED_TABLES.len(),
        "{source} schema covered {} push-owned tables; expected {}",
        schema.len(),
        PUSH_OWNED_TABLES.len()
    );

    for table in PUSH_OWNED_TABLES {
        let columns = schema
            .get(*table)
            .ok_or_else(|| anyhow::anyhow!("{source} schema missing table `{table}`"))?;
        for column in SYNC_TRACKING_COLUMNS {
            ensure!(
                columns.iter().any(|candidate| candidate.name == *column),
                "{source} table `{table}` missing sync tracking column `{column}`"
            );
        }
    }

    Ok(())
}

fn expected_local_origin_summary_schema() -> LogicalSchema {
    let mut schema = LogicalSchema::default();
    schema.insert(
        "workflow_job_summaries".to_owned(),
        columns(&[
            ("job_id", "text", false),
            ("org_id", "text", false),
            ("job_kind", "text", false),
            ("status", "text", false),
            ("skill_path", "text", true),
            ("started_at", "timestamptz", true),
            ("completed_at", "timestamptz", true),
            ("error_type", "text", true),
            ("owner_id", "text", false),
            ("row_version", "bigint", false),
            ("last_local_seq", "bigint", true),
            ("stage_run_id", "text", true),
            ("updated_at", "timestamptz", false),
            ("created_at", "timestamptz", false),
        ]),
    );
    schema.insert(
        "error_manifest_summaries".to_owned(),
        columns(&[
            ("manifest_id", "text", false),
            ("org_id", "text", false),
            ("job_id", "text", true),
            ("error_type", "text", false),
            ("error_code", "text", true),
            ("incident_signature", "text", true),
            ("owner_id", "text", false),
            ("row_version", "bigint", false),
            ("last_local_seq", "bigint", true),
            ("stage_run_id", "text", true),
            ("updated_at", "timestamptz", false),
            ("created_at", "timestamptz", false),
        ]),
    );
    schema.insert(
        "validation_verdict_summaries".to_owned(),
        columns(&[
            ("verdict_id", "text", false),
            ("org_id", "text", false),
            ("worker_job_id", "text", false),
            ("verdict", "text", false),
            ("basis_sufficiency", "text", true),
            ("enforcement_eligible", "boolean", false),
            ("owner_id", "text", false),
            ("row_version", "bigint", false),
            ("last_local_seq", "bigint", true),
            ("stage_run_id", "text", true),
            ("updated_at", "timestamptz", false),
            ("created_at", "timestamptz", false),
        ]),
    );
    schema.insert(
        "viewer_sync_status".to_owned(),
        columns(&[
            ("org_id", "text", false),
            ("sync_health", "text", false),
            ("last_push_success_at", "timestamptz", true),
            ("last_push_attempt_at", "timestamptz", true),
            ("last_acked_local_seq", "bigint", true),
            ("pending_outbox_count", "integer", false),
            ("schema_epoch", "integer", false),
            ("projection_generation", "bigint", false),
            ("daemon_version", "text", true),
            ("updated_at", "timestamptz", false),
        ]),
    );
    schema.insert(
        "executors".to_owned(),
        columns(&[
            ("executor_id", "text", false),
            ("org_id", "text", false),
            ("heartbeat_at", "timestamptz", false),
            ("daemon_version", "text", true),
            ("started_at", "timestamptz", false),
            ("owner_id", "text", false),
            ("row_version", "bigint", false),
            ("last_local_seq", "bigint", true),
            ("stage_run_id", "text", true),
            ("updated_at", "timestamptz", false),
            ("created_at", "timestamptz", false),
        ]),
    );

    schema
}

fn columns(definitions: &[(&str, &str, bool)]) -> Vec<LogicalColumn> {
    let mut columns = definitions
        .iter()
        .map(|(name, raw_type, nullable)| LogicalColumn {
            name: (*name).to_owned(),
            logical_type: normalize_sql_type(raw_type, name),
            nullable: *nullable,
        })
        .collect::<Vec<_>>();
    columns.sort_by(|left, right| left.name.cmp(&right.name));
    columns
}

fn assert_remote_config_policy(table: &str, test_id: &str) -> Result<()> {
    let rls = read_repo_file("infra/supabase/migrations/0004_rls_policies.sql")?;
    let policy_loop = rls
        .split("FOREACH table_name IN ARRAY ARRAY[\n    'shared_company_profile'")
        .nth(1)
        .and_then(|section| section.split("END LOOP;").next())
        .with_context(|| format!("{test_id} missing remote config policy loop"))?;

    ensure!(
        rls.contains(&format!("'{table}'")),
        "{test_id} missing `{table}` in remote config policy loop"
    );
    ensure!(
        policy_loop.contains(
            "FOR SELECT USING (l4_request_role() = ''daemon'' OR l4_request_role() = ''admin'')"
        ) && policy_loop.contains(
            "FOR ALL USING (l4_request_role() = ''admin'') WITH CHECK (l4_request_role() = ''admin'')"
        ),
        "{test_id} `{table}` must allow daemon/admin SELECT and admin-only CRUD"
    );
    ensure!(
        !policy_loop.contains("viewer")
            && !policy_loop.contains("operator")
            && !policy_loop.contains("deleted_at IS NULL"),
        "{test_id} `{table}` must deny viewer/operator and keep tombstones daemon-pull-visible"
    );

    Ok(())
}

fn assert_apply_push_batch_daemon_admin_only(rpc: &str, test_id: &str) -> Result<()> {
    ensure!(
        rpc.contains(
            "CREATE OR REPLACE FUNCTION apply_push_batch(batch_json jsonb, writer_epoch integer)"
        ) && rpc.contains("SECURITY DEFINER")
            && rpc.contains("l4_is_any_role(ARRAY['daemon', 'admin'])")
            && rpc.contains("PERFORM l4_apply_push_owned_payload"),
        "{test_id} apply_push_batch must be security-definer daemon/admin mutation boundary"
    );
    ensure!(
        !rpc.contains("l4_is_any_role(ARRAY['viewer'")
            && !rpc.contains("l4_is_any_role(ARRAY['operator', 'viewer'")
            && !rpc.contains("l4_request_role() = 'viewer'"),
        "{test_id} apply_push_batch must not admit viewer credentials"
    );

    Ok(())
}

fn assert_no_direct_projection_mutation_grants(rls: &str) -> Result<()> {
    for table in PUSH_OWNED_TABLES {
        ensure!(
            !rls.contains(&format!(
                "GRANT INSERT, UPDATE, DELETE ON {table} TO authenticated"
            )) && !rls.contains(&format!("CREATE POLICY {table}_daemon_insert"))
                && !rls.contains(&format!("CREATE POLICY {table}_daemon_update"))
                && !rls.contains(&format!("CREATE POLICY {table}_daemon_all")),
            "push-owned projection table `{table}` must not expose direct daemon/authenticated mutation grants"
        );
    }
    ensure!(
        !rls.contains(
            "GRANT INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO authenticated"
        ),
        "authenticated mutation grants must not cover all tables"
    );

    Ok(())
}

fn assert_summary_projection_redaction(
    table: &str,
    exposed_columns: &[&str],
    forbidden_columns: &[&str],
    test_id: &str,
) -> Result<()> {
    let columns = column_names_for_table(table)?;
    for column in exposed_columns {
        ensure!(
            columns.iter().any(|candidate| candidate == column),
            "{test_id} `{table}` missing exposed summary column `{column}`"
        );
    }
    for column in forbidden_columns {
        ensure!(
            !columns.iter().any(|candidate| candidate == column),
            "{test_id} `{table}` leaks forbidden raw/redacted column `{column}`"
        );
    }

    Ok(())
}

fn assert_forbidden_columns_absent_from_viewer_summaries(
    forbidden_columns: &[&str],
    test_id: &str,
) -> Result<()> {
    for table in VIEWER_VISIBLE_SUMMARY_TABLES {
        assert_summary_projection_redaction(table, &[], forbidden_columns, test_id)?;
    }
    Ok(())
}

fn column_names_for_table(table: &str) -> Result<Vec<String>> {
    let supabase = SupabaseLocalStack::start_or_reuse()?;
    let schema = supabase.logical_schema_from_migrations(&[table])?;
    let columns = schema
        .get(table)
        .with_context(|| format!("missing logical schema for `{table}`"))?;
    Ok(columns.iter().map(|column| column.name.clone()).collect())
}

fn web_source_files(path: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_web_source_files(&repo_root()?.join(path), &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_web_source_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            collect_web_source_files(&path, files)?;
            continue;
        }
        let extension = path.extension().and_then(|extension| extension.to_str());
        if matches!(extension, Some("ts" | "tsx")) {
            files.push(path);
        }
    }
    Ok(())
}

fn schema_diff(expected: &LogicalSchema, actual: &LogicalSchema) -> String {
    let mut output = String::new();
    for table in expected.keys().chain(actual.keys()) {
        let expected_columns = expected.get(table);
        let actual_columns = actual.get(table);
        match (expected_columns, actual_columns) {
            (Some(_), None) => {
                let _ = writeln!(output, "missing actual table `{table}`");
            }
            (None, Some(_)) => {
                let _ = writeln!(output, "unexpected actual table `{table}`");
            }
            (Some(expected_columns), Some(actual_columns)) => {
                append_column_diff(&mut output, table, expected_columns, actual_columns);
            }
            (None, None) => {}
        }
    }
    if output.is_empty() {
        "schema hashes differed but no structural diff was rendered\n".to_owned()
    } else {
        output
    }
}

fn append_column_diff(
    output: &mut String,
    table: &str,
    expected_columns: &[LogicalColumn],
    actual_columns: &[LogicalColumn],
) {
    let expected_by_name = expected_columns
        .iter()
        .map(|column| (&column.name, column))
        .collect::<BTreeMap<_, _>>();
    let actual_by_name = actual_columns
        .iter()
        .map(|column| (&column.name, column))
        .collect::<BTreeMap<_, _>>();

    for name in expected_by_name.keys().chain(actual_by_name.keys()) {
        match (expected_by_name.get(name), actual_by_name.get(name)) {
            (Some(_), None) => {
                let _ = writeln!(output, "table `{table}` missing column `{name}`");
            }
            (None, Some(_)) => {
                let _ = writeln!(output, "table `{table}` has unexpected column `{name}`");
            }
            (Some(expected), Some(actual)) if expected != actual => {
                let _ = writeln!(
                    output,
                    "table `{table}` column `{name}` differs: expected {expected:?}, actual {actual:?}"
                );
            }
            _ => {}
        }
    }
}

fn repo_root() -> Result<&'static Path> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .context("daemon crate should live two levels below the repository root")
}

fn read_repo_file(path: &str) -> Result<String> {
    std::fs::read_to_string(repo_root()?.join(path))
        .with_context(|| format!("reading repository file {path}"))
}
