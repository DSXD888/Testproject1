#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

#[allow(dead_code)]
mod support;

mod tests {
    use anyhow::{Result, ensure};
    use rusqlite::{Connection, params};
    use serde_json::json;
    use tempfile::tempdir;

    use crate::support::db::{StateDb, TestStateDb};

    fn table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table_name],
            |row| row.get(0),
        )?;
        Ok(count == 1)
    }

    fn table_columns(conn: &Connection, table_name: &str) -> Result<Vec<String>> {
        let pragma = format!("PRAGMA table_info({table_name})");
        let mut statement = conn.prepare(&pragma)?;
        let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    fn foreign_key_targets(conn: &Connection, table_name: &str) -> Result<Vec<(String, String)>> {
        let pragma = format!("PRAGMA foreign_key_list({table_name})");
        let mut statement = conn.prepare(&pragma)?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(3)?, row.get::<_, String>(2)?))
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    fn seed_process(conn: &Connection, process_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO procurement_processes (
              id, current_stage, title, latest_published_at, latest_updated_at, owner_id
            ) VALUES (?1, 'qualification', 'Process title', '2026-03-25T00:00:00Z',
                      '2026-03-25T00:00:00Z', 'local-owner')
            ",
            params![process_id],
        )?;
        Ok(())
    }

    fn seed_job(conn: &Connection, job_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO workflow_jobs (id, kind, status, input_payload)
            VALUES (?1, 'stage-job', 'pending', ?2)
            ",
            params![job_id, json!({ "job_id": job_id }).to_string()],
        )?;
        Ok(())
    }

    #[test]
    fn l3_t_schema_activation_001_migration_creates_l3_tables_with_declared_columns_and_keys()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();

        for table_name in [
            "validation_verdicts",
            "source_runtime_state",
            "listing_evidence",
            "process_relations",
            "process_aliases",
        ] {
            ensure!(
                table_exists(conn, table_name)?,
                "L3-T-schema_activation-001 missing table {table_name}"
            );
        }

        let validation_columns = table_columns(conn, "validation_verdicts")?;
        for column in [
            "worker_job_id",
            "validator_job_id",
            "basis_sufficiency",
            "enforcement_eligible",
            "run_manifest_hash",
            "override_verdict",
            "override_reason",
            "override_by",
            "override_at",
        ] {
            ensure!(
                validation_columns
                    .iter()
                    .any(|candidate| candidate == column),
                "L3-T-schema_activation-001 validation_verdicts missing column {column}"
            );
        }

        let validation_fk_targets = foreign_key_targets(conn, "validation_verdicts")?;
        ensure!(
            validation_fk_targets
                .iter()
                .any(|(from, target)| from == "worker_job_id" && target == "workflow_jobs"),
            "L3-T-schema_activation-001 expected worker_job_id foreign key"
        );
        ensure!(
            validation_fk_targets
                .iter()
                .any(|(from, target)| from == "validator_job_id" && target == "workflow_jobs"),
            "L3-T-schema_activation-001 expected validator_job_id foreign key"
        );

        let listing_columns = table_columns(conn, "listing_evidence")?;
        for column in [
            "source",
            "source_listing_id",
            "trust_state",
            "source_job_id",
        ] {
            ensure!(
                listing_columns.iter().any(|candidate| candidate == column),
                "L3-T-schema_activation-001 listing_evidence missing column {column}"
            );
        }

        let relation_columns = table_columns(conn, "process_relations")?;
        for column in [
            "process_id_a",
            "process_id_b",
            "relation_type",
            "confidence_bps",
            "owner_id",
            "row_version",
            "last_local_seq",
            "stage_run_id",
        ] {
            ensure!(
                relation_columns.iter().any(|candidate| candidate == column),
                "L3-T-schema_activation-001 process_relations missing column {column}"
            );
        }

        let alias_columns = table_columns(conn, "process_aliases")?;
        for column in [
            "alias_process_id",
            "canonical_process_id",
            "alias_reason",
            "owner_id",
            "row_version",
            "last_local_seq",
            "stage_run_id",
        ] {
            ensure!(
                alias_columns.iter().any(|candidate| candidate == column),
                "L3-T-schema_activation-001 process_aliases missing column {column}"
            );
        }

        Ok(())
    }

    #[test]
    fn l3_t_schema_activation_002_source_runtime_state_shape_activates_at_migration_time()
    -> Result<()> {
        let temp = tempdir()?;
        let db_path = temp.path().join("state.db");
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "\
            CREATE TABLE sources (
              id TEXT PRIMARY KEY,
              kind TEXT NOT NULL,
              enabled INTEGER NOT NULL DEFAULT 1,
              config TEXT NOT NULL,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE source_runtime_state (
              source_id TEXT PRIMARY KEY REFERENCES sources(id),
              last_health_state TEXT NOT NULL DEFAULT 'unknown',
              challenge_event_count INTEGER NOT NULL DEFAULT 0,
              challenge_window_started_at TEXT,
              circuit_opened_at TEXT,
              cooldown_until TEXT,
              last_canary_at TEXT,
              last_error_code TEXT,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;
        drop(conn);

        let state_db = StateDb::open(&db_path)?;
        let columns = table_columns(state_db.connection(), "source_runtime_state")?;
        ensure!(
            columns.iter().any(|column| column == "circuit_state"),
            "L3-T-schema_activation-002 expected circuit_state at migration time"
        );
        ensure!(
            columns.iter().any(|column| column == "last_canary_result"),
            "L3-T-schema_activation-002 expected last_canary_result at migration time"
        );
        Ok(())
    }

    #[test]
    fn l3_t_schema_activation_003_validation_verdict_linkage_commits_atomically_or_not_at_all()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();
        seed_job(conn, "worker-job")?;
        seed_job(conn, "validator-job")?;

        conn.execute_batch("SAVEPOINT schema_activation_003_fail")?;
        conn.execute(
            "\
            INSERT INTO validation_verdicts (
              id, worker_job_id, validator_job_id, validator_skill_path, verdict, summary,
              run_manifest_hash, verdict_rendered_at
            ) VALUES (?1, ?2, ?3, ?4, 'approved', ?5, ?6, ?7)
            ",
            params![
                "verdict-atomic-fail",
                "worker-job",
                "validator-job",
                "skills/validation/base/SKILL.md",
                "summary",
                "manifest-hash",
                "2026-03-25T00:00:00Z",
            ],
        )?;
        conn.execute(
            "UPDATE workflow_jobs SET validation_verdict_id = ?1 WHERE id = ?2",
            params!["verdict-atomic-fail", "worker-job"],
        )?;
        let duplicate_insert = conn.execute(
            "\
            INSERT INTO validation_verdicts (
              id, worker_job_id, validator_job_id, validator_skill_path, verdict, summary,
              run_manifest_hash, verdict_rendered_at
            ) VALUES (?1, ?2, ?3, ?4, 'approved', ?5, ?6, ?7)
            ",
            params![
                "verdict-atomic-fail",
                "worker-job",
                "validator-job",
                "skills/validation/base/SKILL.md",
                "summary",
                "manifest-hash",
                "2026-03-25T00:00:00Z",
            ],
        );
        ensure!(
            duplicate_insert.is_err(),
            "L3-T-schema_activation-003 expected duplicate insert failure"
        );
        conn.execute_batch(
            "ROLLBACK TO SAVEPOINT schema_activation_003_fail; RELEASE SAVEPOINT schema_activation_003_fail",
        )?;

        let failed_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM validation_verdicts WHERE id = ?1",
            params!["verdict-atomic-fail"],
            |row| row.get(0),
        )?;
        let failed_link: Option<String> = conn.query_row(
            "SELECT validation_verdict_id FROM workflow_jobs WHERE id = ?1",
            params!["worker-job"],
            |row| row.get(0),
        )?;
        ensure!(failed_count == 0);
        ensure!(failed_link.is_none());

        conn.execute_batch("BEGIN IMMEDIATE")?;
        conn.execute(
            "\
            INSERT INTO validation_verdicts (
              id, worker_job_id, validator_job_id, validator_skill_path, verdict, summary,
              run_manifest_hash, verdict_rendered_at
            ) VALUES (?1, ?2, ?3, ?4, 'approved', ?5, ?6, ?7)
            ",
            params![
                "verdict-atomic-pass",
                "worker-job",
                "validator-job",
                "skills/validation/base/SKILL.md",
                "summary",
                "manifest-hash",
                "2026-03-25T00:00:00Z",
            ],
        )?;
        conn.execute(
            "UPDATE workflow_jobs SET validation_verdict_id = ?1 WHERE id = ?2",
            params!["verdict-atomic-pass", "worker-job"],
        )?;
        conn.execute_batch("COMMIT")?;

        let passed_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM validation_verdicts WHERE id = ?1",
            params!["verdict-atomic-pass"],
            |row| row.get(0),
        )?;
        let passed_link: Option<String> = conn.query_row(
            "SELECT validation_verdict_id FROM workflow_jobs WHERE id = ?1",
            params!["worker-job"],
            |row| row.get(0),
        )?;
        ensure!(passed_count == 1);
        ensure!(passed_link.as_deref() == Some("verdict-atomic-pass"));
        Ok(())
    }

    #[test]
    fn l3_t_schema_activation_004_listing_evidence_enforces_uniqueness_defaults_and_provenance()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();
        seed_job(conn, "source-job")?;

        conn.execute(
            "\
            INSERT INTO listing_evidence (
              id, source, source_listing_id, normalized_title, compliance_evidence_tier,
              fetched_at, source_job_id
            ) VALUES (?1, ?2, ?3, ?4, 'absent', ?5, ?6)
            ",
            params![
                "listing-evidence-1",
                "alibaba",
                "listing-1",
                "Normalized title",
                "2026-03-25T00:00:00Z",
                "source-job",
            ],
        )?;

        let (trust_state, source_job_id): (String, Option<String>) = conn.query_row(
            "SELECT trust_state, source_job_id FROM listing_evidence WHERE id = ?1",
            params!["listing-evidence-1"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        ensure!(trust_state == "unverified");
        ensure!(source_job_id.as_deref() == Some("source-job"));

        let duplicate = conn.execute(
            "\
            INSERT INTO listing_evidence (
              id, source, source_listing_id, normalized_title, compliance_evidence_tier, fetched_at
            ) VALUES (?1, ?2, ?3, ?4, 'absent', ?5)
            ",
            params![
                "listing-evidence-2",
                "alibaba",
                "listing-1",
                "Normalized title",
                "2026-03-25T00:00:00Z",
            ],
        );
        ensure!(
            duplicate.is_err(),
            "L3-T-schema_activation-004 expected (source, source_listing_id) uniqueness"
        );
        Ok(())
    }

    #[test]
    fn l3_t_schema_activation_005_process_relations_disallow_separate_rows() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();
        seed_process(conn, "process-a")?;
        seed_process(conn, "process-b")?;

        for (relation_id, relation_type) in [
            ("relation-merged", "merged_same_process"),
            ("relation-related", "related_not_merged"),
            ("relation-possible", "possible_duplicate"),
        ] {
            conn.execute(
                "\
                INSERT INTO process_relations (
                  id, process_id_a, process_id_b, relation_type, confidence_bps, owner_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, 'local-owner')
                ",
                params![relation_id, "process-a", "process-b", relation_type, 9500],
            )?;
        }

        let separate = conn.execute(
            "\
            INSERT INTO process_relations (
              id, process_id_a, process_id_b, relation_type, confidence_bps, owner_id
            ) VALUES (?1, ?2, ?3, 'separate', ?4, 'local-owner')
            ",
            params!["relation-separate", "process-a", "process-b", 100],
        );
        ensure!(
            separate.is_err(),
            "L3-T-schema_activation-005 expected `separate` relation rows to be rejected"
        );
        Ok(())
    }

    #[test]
    fn l3_t_schema_activation_006_process_aliases_preserve_redirect_and_push_owned_columns()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();
        seed_process(conn, "canonical-process")?;
        let alias_columns = table_columns(conn, "process_aliases")?;
        for column in ["owner_id", "row_version", "last_local_seq", "stage_run_id"] {
            ensure!(
                alias_columns.iter().any(|candidate| candidate == column),
                "L3-T-schema_activation-006 process_aliases missing column {column}"
            );
        }

        conn.execute(
            "\
            INSERT INTO process_aliases (
              alias_process_id, canonical_process_id, alias_reason, owner_id, stage_run_id
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                "alias-process",
                "canonical-process",
                "merged_same_process",
                "local-owner",
                "stage-run-123",
            ],
        )?;

        let (canonical_process_id, stage_run_id): (String, Option<String>) = conn.query_row(
            "SELECT canonical_process_id, stage_run_id FROM process_aliases WHERE alias_process_id = ?1",
            params!["alias-process"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        ensure!(canonical_process_id == "canonical-process");
        ensure!(stage_run_id.as_deref() == Some("stage-run-123"));
        Ok(())
    }

    #[test]
    fn l3_t_schema_activation_007_live_l3_columns_round_trip_through_runtime_paths() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();
        seed_job(conn, "parent-job")?;
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, parent_job_id, validation_required, status, input_payload
            ) VALUES (?1, 'stage-job', ?2, 1, 'pending', ?3)
            ",
            params![
                "child-job",
                "parent-job",
                json!({"source_id":"fts"}).to_string()
            ],
        )?;

        seed_job(conn, "validator-job")?;
        conn.execute(
            "\
            INSERT INTO validation_verdicts (
              id, worker_job_id, validator_job_id, validator_skill_path, verdict, summary,
              run_manifest_hash, verdict_rendered_at
            ) VALUES (?1, ?2, ?3, ?4, 'approved', ?5, ?6, ?7)
            ",
            params![
                "verdict-007",
                "child-job",
                "validator-job",
                "skills/validation/base/SKILL.md",
                "summary",
                "manifest-hash-007",
                "2026-03-25T00:00:00Z",
            ],
        )?;
        conn.execute(
            "UPDATE workflow_jobs SET validation_verdict_id = ?1 WHERE id = ?2",
            params!["verdict-007", "child-job"],
        )?;
        conn.execute(
            "\
            INSERT INTO agent_session_logs (
              id, job_id, skill_path, execution_mode, status, run_manifest_path, run_manifest_hash
            ) VALUES (?1, ?2, ?3, 'contract', 'completed', ?4, ?5)
            ",
            params![
                "session-007",
                "child-job",
                "skills/manifest-generation/SKILL.md",
                "artifacts/manifests/session-007.md",
                "manifest-hash-007",
            ],
        )?;

        let (parent_job_id, validation_required, validation_verdict_id): (
            Option<String>,
            i64,
            Option<String>,
        ) = conn.query_row(
            "\
            SELECT parent_job_id, validation_required, validation_verdict_id
            FROM workflow_jobs
            WHERE id = ?1
            ",
            params!["child-job"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        ensure!(parent_job_id.as_deref() == Some("parent-job"));
        ensure!(validation_required == 1);
        ensure!(validation_verdict_id.as_deref() == Some("verdict-007"));

        let (run_manifest_path, run_manifest_hash): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT run_manifest_path, run_manifest_hash FROM agent_session_logs WHERE id = ?1",
                params!["session-007"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
        ensure!(
            run_manifest_path.as_deref() == Some("artifacts/manifests/session-007.md"),
            "L3-T-schema_activation-007 expected run_manifest_path population"
        );
        ensure!(
            run_manifest_hash.as_deref() == Some("manifest-hash-007"),
            "L3-T-schema_activation-007 expected run_manifest_hash population"
        );
        Ok(())
    }

    #[test]
    fn l3_t_schema_activation_008_authority_shapes_match_local_vs_push_owned_contracts()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();

        for local_only_table in [
            "validation_verdicts",
            "source_runtime_state",
            "listing_evidence",
        ] {
            let columns = table_columns(conn, local_only_table)?;
            ensure!(
                !columns.iter().any(|column| column == "row_version"),
                "L3-T-schema_activation-008 local-only table {local_only_table} should not carry push-owned sync columns"
            );
            ensure!(
                !columns.iter().any(|column| column == "last_local_seq"),
                "L3-T-schema_activation-008 local-only table {local_only_table} should not carry push-owned sync columns"
            );
        }

        for push_owned_table in ["process_relations", "process_aliases"] {
            let columns = table_columns(conn, push_owned_table)?;
            for column in ["owner_id", "row_version", "last_local_seq", "stage_run_id"] {
                ensure!(
                    columns.iter().any(|candidate| candidate == column),
                    "L3-T-schema_activation-008 push-owned table {push_owned_table} missing sync column {column}"
                );
            }
        }

        Ok(())
    }
}
