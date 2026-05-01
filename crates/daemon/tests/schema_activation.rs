#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

#[allow(dead_code)]
mod support;

mod tests {
    use std::fs;
    use std::path::PathBuf;

    use anyhow::{Result, ensure};
    use rusqlite::{Connection, OptionalExtension, params};
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

    fn index_sql(conn: &Connection, index_name: &str) -> Result<Option<String>> {
        conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [index_name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(Into::into)
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn apply_migration(conn: &Connection, relative_path: &str) -> Result<()> {
        let ddl = fs::read_to_string(repo_root().join(relative_path))?;
        conn.execute_batch(&ddl)?;
        Ok(())
    }

    fn seed_applied_migrations(
        conn: &Connection,
        migration_ids: &[&str],
        schema_epoch: i64,
    ) -> Result<()> {
        conn.execute_batch(
            "\
            CREATE TABLE IF NOT EXISTS schema_migrations (
              id TEXT PRIMARY KEY,
              applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;
        for migration_id in migration_ids {
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (id) VALUES (?1)",
                [*migration_id],
            )?;
        }
        conn.pragma_update(None, "user_version", schema_epoch)?;
        Ok(())
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

    fn seed_notice(conn: &Connection, process_id: &str, notice_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO contract_notices (
              id, process_id, source_id, source_notice_id, raw_document_id, title,
              procurement_stage, published_at, source_updated_at, normalized_payload, owner_id
            ) VALUES (?1, ?2, 'fts', ?3, ?4, 'Notice title', 'tender',
                      '2026-03-25T00:00:00Z', '2026-03-25T00:00:00Z', '{}', 'local-owner')
            ",
            params![
                notice_id,
                process_id,
                format!("source-{notice_id}"),
                format!("raw-{notice_id}")
            ],
        )?;
        Ok(())
    }

    fn seed_document_set(
        conn: &Connection,
        document_set_id: &str,
        process_id: &str,
        notice_id: &str,
    ) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO tender_document_sets (
              id, process_id, notice_id, document_set_hash, completeness_state, document_roles_json,
              documents_json, acquired_at, owner_id
            ) VALUES (?1, ?2, ?3, ?4, 'complete', ?5, ?6, '2026-03-25T00:00:00Z', 'local-owner')
            ",
            params![
                document_set_id,
                process_id,
                notice_id,
                format!("hash-{document_set_id}"),
                json!([]).to_string(),
                json!([]).to_string(),
            ],
        )?;
        Ok(())
    }

    fn insert_document_extraction(
        conn: &Connection,
        extraction_id: &str,
        raw_document_id: &str,
        process_id: &str,
        notice_id: &str,
        document_set_id: &str,
        is_current: i64,
    ) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO document_extractions (
              id, raw_document_id, process_id, notice_id, document_set_id, source_content_hash,
              extraction_engine, extraction_engine_version, processing_strategy, is_current,
              page_count, extraction_policy_version, expected_overlays_json, extracted_markdown,
              extracted_sections_json, extracted_tables_json, extracted_metadata_json, table_count,
              section_count, owner_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'docling', '2.85.0', 'standard', ?7,
                      3, '2026.04.12', ?8, ?9, ?10, ?11, ?12, 1, 2, 'local-owner')
            ",
            params![
                extraction_id,
                raw_document_id,
                process_id,
                notice_id,
                document_set_id,
                format!("hash-{extraction_id}"),
                is_current,
                json!(["pdf-extraction/tender-specification"]).to_string(),
                "# Extracted markdown",
                json!([
                    {
                        "heading": "Section 1",
                        "page_start": 1
                    }
                ])
                .to_string(),
                json!([
                    {
                        "title": "Pricing table",
                        "page": 2
                    }
                ])
                .to_string(),
                json!({
                    "source": "test-fixture"
                })
                .to_string(),
            ],
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

    #[test]
    fn pdf_t_schema_activation_001_fresh_db_creates_pdf_extraction_tables_and_indexes() -> Result<()>
    {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();

        for table_name in ["document_extractions", "document_semantic_extractions"] {
            ensure!(
                table_exists(conn, table_name)?,
                "PDF-T-schema_activation-001 missing table {table_name}"
            );
        }

        for index_name in [
            "idx_doc_extraction_current",
            "idx_doc_extraction_hash_lookup",
            "idx_doc_extraction_process",
            "idx_doc_extraction_semantic_state",
            "idx_doc_semantic_extraction",
            "idx_doc_semantic_process",
            "idx_doc_semantic_current",
        ] {
            ensure!(
                index_sql(conn, index_name)?.is_some(),
                "PDF-T-schema_activation-001 missing index {index_name}"
            );
        }

        let doc_extraction_current =
            index_sql(conn, "idx_doc_extraction_current")?.unwrap_or_default();
        ensure!(
            doc_extraction_current.contains("CREATE UNIQUE INDEX"),
            "PDF-T-schema_activation-001 current document extraction index must be unique"
        );
        ensure!(
            doc_extraction_current.contains("WHERE is_current = 1"),
            "PDF-T-schema_activation-001 current document extraction index must be partial"
        );

        let semantic_state_index =
            index_sql(conn, "idx_doc_extraction_semantic_state")?.unwrap_or_default();
        ensure!(
            semantic_state_index
                .contains("WHERE semantic_extraction_state IN ('pending', 'failed')"),
            "PDF-T-schema_activation-001 semantic state index must target pending/failed rows"
        );

        let semantic_current = index_sql(conn, "idx_doc_semantic_current")?.unwrap_or_default();
        ensure!(
            semantic_current.contains("CREATE UNIQUE INDEX"),
            "PDF-T-schema_activation-001 current semantic extraction index must be unique"
        );
        ensure!(
            semantic_current.contains("WHERE is_current = 1"),
            "PDF-T-schema_activation-001 current semantic extraction index must be partial"
        );

        Ok(())
    }

    #[test]
    fn pdf_t_schema_activation_002_upgrade_path_applies_0006_cleanly() -> Result<()> {
        let temp = tempdir()?;
        let db_path = temp.path().join("state.db");
        let conn = Connection::open(&db_path)?;

        for migration_path in [
            "migrations/0001_sqlite_state_initial.sql",
            "migrations/0003_sqlite_runtime.sql",
            "migrations/0004_l2_pending_operator_intents.sql",
            "migrations/0005_l3_operational_hardening.sql",
        ] {
            apply_migration(&conn, migration_path)?;
        }
        seed_applied_migrations(
            &conn,
            &[
                "0001_sqlite_state_initial",
                "0003_sqlite_runtime",
                "0004_l2_pending_operator_intents",
                "0005_l3_operational_hardening",
            ],
            2,
        )?;
        drop(conn);

        let state_db = StateDb::open(&db_path)?;
        let conn = state_db.connection();
        let tender_document_columns = table_columns(conn, "tender_document_sets")?;
        let pdf_migration_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE id = '0006_pdf_extraction_toolkit'",
            [],
            |row| row.get(0),
        )?;

        ensure!(
            table_exists(conn, "document_extractions")?,
            "PDF-T-schema_activation-002 expected document_extractions after upgrade"
        );
        ensure!(
            table_exists(conn, "document_semantic_extractions")?,
            "PDF-T-schema_activation-002 expected document_semantic_extractions after upgrade"
        );
        ensure!(
            tender_document_columns
                .iter()
                .any(|column| column == "extraction_state"),
            "PDF-T-schema_activation-002 expected extraction_state after upgrade"
        );
        ensure!(
            pdf_migration_count == 1,
            "PDF-T-schema_activation-002 expected 0006 migration log entry"
        );
        Ok(())
    }

    #[test]
    fn ngg_t_schema_activation_007_upgrade_path_applies_0007_cleanly() -> Result<()> {
        let temp = tempdir()?;
        let db_path = temp.path().join("state.db");
        let conn = Connection::open(&db_path)?;

        for migration_path in [
            "migrations/0001_sqlite_state_initial.sql",
            "migrations/0003_sqlite_runtime.sql",
            "migrations/0004_l2_pending_operator_intents.sql",
            "migrations/0005_l3_operational_hardening.sql",
            "migrations/0006_pdf_extraction_toolkit.sql",
        ] {
            apply_migration(&conn, migration_path)?;
        }
        seed_applied_migrations(
            &conn,
            &[
                "0001_sqlite_state_initial",
                "0003_sqlite_runtime",
                "0004_l2_pending_operator_intents",
                "0005_l3_operational_hardening",
                "0006_pdf_extraction_toolkit",
            ],
            3,
        )?;
        drop(conn);

        let state_db = StateDb::open(&db_path)?;
        let conn = state_db.connection();
        let net_margin_columns = table_columns(conn, "candidate_net_margin_assessments")?;
        let net_gate_migration_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE id = '0007_net_margin_gate_assessments'",
            [],
            |row| row.get(0),
        )?;

        ensure!(
            table_exists(conn, "candidate_net_margin_assessments")?,
            "NGG-T-schema_activation-007 expected candidate_net_margin_assessments after upgrade"
        );
        for column in [
            "id",
            "process_id",
            "notice_id",
            "lot_id",
            "product_candidate_id",
            "pricing_assessment_id",
            "landed_cost_estimate_id",
            "verdict",
            "reasons_json",
            "re_evaluation_eligible",
            "net_margin_low_bps",
            "net_margin_base_bps",
            "net_margin_high_bps",
            "revenue_used_pence",
            "revenue_basis_type",
            "economic_landed_cost_used_pence",
            "revenue_confidence_bps",
            "pricing_input_confidence_bps",
            "customs_resolution_confidence_bps",
            "commercial_estimate_confidence_bps",
            "pass_threshold_bps",
            "reject_threshold_bps",
            "thresholds_version",
            "basis_hash",
            "stage_run_id",
        ] {
            ensure!(
                net_margin_columns
                    .iter()
                    .any(|candidate| candidate == column),
                "NGG-T-schema_activation-007 candidate_net_margin_assessments missing column {column}"
            );
        }
        ensure!(
            net_gate_migration_count == 1,
            "NGG-T-schema_activation-007 expected 0007 migration log entry"
        );
        Ok(())
    }

    #[test]
    fn mfg_t_schema_activation_008_upgrade_path_applies_0008_cleanly() -> Result<()> {
        let temp = tempdir()?;
        let db_path = temp.path().join("state.db");
        let conn = Connection::open(&db_path)?;

        for migration_path in [
            "migrations/0001_sqlite_state_initial.sql",
            "migrations/0003_sqlite_runtime.sql",
            "migrations/0004_l2_pending_operator_intents.sql",
            "migrations/0005_l3_operational_hardening.sql",
            "migrations/0006_pdf_extraction_toolkit.sql",
            "migrations/0007_net_margin_gate_assessments.sql",
        ] {
            apply_migration(&conn, migration_path)?;
        }
        seed_applied_migrations(
            &conn,
            &[
                "0001_sqlite_state_initial",
                "0003_sqlite_runtime",
                "0004_l2_pending_operator_intents",
                "0005_l3_operational_hardening",
                "0006_pdf_extraction_toolkit",
                "0007_net_margin_gate_assessments",
            ],
            4,
        )?;
        drop(conn);

        let state_db = StateDb::open(&db_path)?;
        let conn = state_db.connection();
        let manifest_columns = table_columns(conn, "manifests")?;
        let manifest_migration_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE id = '0008_manifest_generation_columns'",
            [],
            |row| row.get(0),
        )?;

        for column in [
            "stoplight",
            "suggested_action",
            "suggested_action_reasoning",
            "clarification_prompts_json",
            "manifest_basis_hash",
            "manifest_template_version",
            "candidate_count",
            "priced_candidate_count",
            "clarification_prompt_count",
        ] {
            ensure!(
                manifest_columns.iter().any(|candidate| candidate == column),
                "MFG-T-schema_activation-008 manifests missing column {column}"
            );
        }
        ensure!(
            manifest_migration_count == 1,
            "MFG-T-schema_activation-008 expected 0008 migration log entry"
        );
        Ok(())
    }

    #[test]
    fn pdf_t_schema_activation_003_document_extractions_columns_match_contract() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let columns = table_columns(state_db.connection(), "document_extractions")?;
        let expected = vec![
            "id",
            "raw_document_id",
            "process_id",
            "notice_id",
            "document_set_id",
            "source_content_hash",
            "extraction_engine",
            "extraction_engine_version",
            "processing_strategy",
            "is_current",
            "superseded_by_extraction_id",
            "page_count",
            "document_title",
            "document_language",
            "document_role",
            "extraction_policy_version",
            "expected_overlays_json",
            "extracted_markdown",
            "extracted_sections_json",
            "extracted_tables_json",
            "extracted_metadata_json",
            "table_count",
            "section_count",
            "image_count",
            "has_ocr_content",
            "empty_page_count",
            "quality_warnings_json",
            "semantic_extraction_state",
            "owner_id",
            "row_version",
            "last_local_seq",
            "stage_run_id",
            "updated_at",
            "created_at",
        ];

        ensure!(
            columns == expected,
            "PDF-T-schema_activation-003 document_extractions columns diverged from contract"
        );
        Ok(())
    }

    #[test]
    fn pdf_t_schema_activation_004_document_semantic_extractions_columns_match_contract()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        let columns = table_columns(state_db.connection(), "document_semantic_extractions")?;
        let expected = vec![
            "id",
            "document_extraction_id",
            "process_id",
            "skill_id",
            "skill_version",
            "job_id",
            "session_id",
            "extraction_scope",
            "target_domain",
            "is_current",
            "superseded_by_semantic_extraction_id",
            "extracted_data_json",
            "field_confidences_json",
            "extraction_confidence_bps",
            "warnings_json",
            "evidence_anchors_json",
            "written_entity_refs_json",
            "owner_id",
            "row_version",
            "last_local_seq",
            "stage_run_id",
            "updated_at",
            "created_at",
        ];

        ensure!(
            columns == expected,
            "PDF-T-schema_activation-004 document_semantic_extractions columns diverged from contract"
        );
        Ok(())
    }

    #[test]
    fn pdf_t_schema_activation_005_tender_document_sets_has_extraction_state_default_none()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();
        seed_process(conn, "process-doc-set")?;
        seed_notice(conn, "process-doc-set", "notice-doc-set")?;
        seed_document_set(
            conn,
            "document-set-default",
            "process-doc-set",
            "notice-doc-set",
        )?;

        let extraction_state: String = conn.query_row(
            "SELECT extraction_state FROM tender_document_sets WHERE id = ?1",
            params!["document-set-default"],
            |row| row.get(0),
        )?;
        ensure!(
            extraction_state == "none",
            "PDF-T-schema_activation-005 expected extraction_state default to none"
        );
        Ok(())
    }

    #[test]
    fn pdf_t_schema_activation_006_document_extractions_current_index_is_unique() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();
        seed_process(conn, "process-current")?;
        seed_notice(conn, "process-current", "notice-current")?;
        seed_document_set(
            conn,
            "document-set-current",
            "process-current",
            "notice-current",
        )?;
        insert_document_extraction(
            conn,
            "document-extraction-1",
            "raw-document-current",
            "process-current",
            "notice-current",
            "document-set-current",
            1,
        )?;

        let duplicate = insert_document_extraction(
            conn,
            "document-extraction-2",
            "raw-document-current",
            "process-current",
            "notice-current",
            "document-set-current",
            1,
        );
        ensure!(
            duplicate.is_err(),
            "PDF-T-schema_activation-006 expected duplicate current extraction to fail"
        );
        Ok(())
    }

    #[test]
    fn pdf_t_schema_activation_007_document_extractions_has_no_fk_to_raw_documents() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let foreign_keys = foreign_key_targets(state_db.connection(), "document_extractions")?;
        ensure!(
            foreign_keys
                .iter()
                .all(|(from, target)| !(from == "raw_document_id" && target == "raw_documents")),
            "PDF-T-schema_activation-007 document_extractions must not reference raw_documents"
        );
        ensure!(
            foreign_keys
                .iter()
                .all(|(_, target)| target != "raw_documents"),
            "PDF-T-schema_activation-007 document_extractions must not target raw_documents"
        );
        Ok(())
    }

    #[test]
    fn pdf_t_schema_activation_008_document_semantic_extractions_has_no_fk_to_workflow_jobs_or_agent_session_logs()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        let foreign_keys =
            foreign_key_targets(state_db.connection(), "document_semantic_extractions")?;
        ensure!(
            foreign_keys
                .iter()
                .all(|(from, target)| !(from == "job_id" && target == "workflow_jobs")),
            "PDF-T-schema_activation-008 document_semantic_extractions must not reference workflow_jobs"
        );
        ensure!(
            foreign_keys
                .iter()
                .all(|(from, target)| !(from == "session_id" && target == "agent_session_logs")),
            "PDF-T-schema_activation-008 document_semantic_extractions must not reference agent_session_logs"
        );
        ensure!(
            foreign_keys
                .iter()
                .all(|(_, target)| target != "workflow_jobs" && target != "agent_session_logs"),
            "PDF-T-schema_activation-008 document_semantic_extractions must not target local-only tables"
        );
        Ok(())
    }

    #[test]
    fn pdf_t_schema_activation_009_default_schema_epoch_is_5() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let user_version: i64 =
            state_db
                .connection()
                .pragma_query_value(None, "user_version", |row| row.get(0))?;
        ensure!(
            crate::support::db::DEFAULT_SCHEMA_EPOCH == 5,
            "PDF-T-schema_activation-009 expected DEFAULT_SCHEMA_EPOCH to be 5"
        );
        ensure!(
            user_version == 5,
            "PDF-T-schema_activation-009 expected user_version to be 5"
        );
        Ok(())
    }

    #[test]
    fn pdf_t_schema_activation_010_raw_documents_still_has_no_page_count_column() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let raw_document_columns = table_columns(state_db.connection(), "raw_documents")?;
        ensure!(
            !raw_document_columns
                .iter()
                .any(|column| column == "page_count"),
            "PDF-T-schema_activation-010 raw_documents must not grow a page_count column"
        );
        Ok(())
    }
}
