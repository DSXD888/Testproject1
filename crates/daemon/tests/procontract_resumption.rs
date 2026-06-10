#![forbid(unsafe_code)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, ensure};
use assert_cmd::Command;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use tempfile::TempDir;

use crate::support::db::StateDb;

const TRACKED_PROCESS_ID: &str = "proc-pc-resume-001";
const TRACKED_NOTICE_ID: &str = "pc-resume-001";
const BACKFILL_PROCESS_ID: &str = "proc-pc-backfill-001";
const BACKFILL_NOTICE_ID: &str = "pc-backfill-001";

struct ProcontractRunHarness {
    _temp: TempDir,
    state_root: PathBuf,
    runtime_root: PathBuf,
    artifact_root: PathBuf,
    result_path: PathBuf,
    checkpoint_path: PathBuf,
    fake_state_dir: PathBuf,
    fake_program: PathBuf,
    fake_invocation_log: PathBuf,
    db_path: PathBuf,
}

struct RunResult {
    stdout: String,
    stderr: String,
    success: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct PendingRow {
    state: String,
    observation_count: i64,
    interest_registered_at: String,
    deadline_at: Option<String>,
    last_checked_at: Option<String>,
    enriched_at: Option<String>,
}

impl ProcontractRunHarness {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().to_path_buf();
        let state_root = root.join("state");
        let runtime_root = root.join("runtime");
        let artifact_root = root.join("artifacts");
        let fake_state_dir = root.join("fake-claude");
        fs::create_dir_all(&state_root)?;
        fs::create_dir_all(&runtime_root)?;
        fs::create_dir_all(&artifact_root)?;
        fs::create_dir_all(&fake_state_dir)?;

        let harness = Self {
            _temp: temp,
            db_path: state_root.join("state.db"),
            result_path: artifact_root.join("fake-claude-result.json"),
            checkpoint_path: artifact_root.join("fake-claude-checkpoint.json"),
            fake_program: fake_state_dir.join("claude"),
            fake_invocation_log: fake_state_dir.join("invocations.log"),
            state_root,
            runtime_root,
            artifact_root,
            fake_state_dir,
        };
        harness.write_fake_claude()?;
        Ok(harness)
    }

    fn seed_notice(&self, process_id: &str, notice_id: &str, title: &str) -> Result<()> {
        let state_db = StateDb::open(&self.db_path)?;
        seed_minimal_procontract_notice(state_db.connection(), process_id, notice_id, title)
    }

    fn run_ingest(&self, source_id: &str, result_contract: &Value) -> Result<RunResult> {
        fs::write(
            self.fake_state_dir.join("result-source.json"),
            serde_json::to_vec_pretty(result_contract)?,
        )
        .context("writing fake Claude result")?;

        let output = Command::cargo_bin("daemon")?
            .arg("--ingest")
            .arg(source_id)
            .arg("--agent-cli")
            .arg("claude")
            .arg("--agent-cli-program")
            .arg(&self.fake_program)
            .arg("--state-root")
            .arg(&self.state_root)
            .arg("--runtime-dir")
            .arg(&self.runtime_root)
            .arg("--artifact-root")
            .arg(&self.artifact_root)
            .env("DAEMON_TEST_CLAUDE_RESULT_PATH", &self.result_path)
            .env("DAEMON_TEST_CLAUDE_CHECKPOINT_PATH", &self.checkpoint_path)
            .env_remove("SUPABASE_URL")
            .env_remove("NEXT_PUBLIC_SUPABASE_URL")
            .env_remove("SUPABASE_SERVICE_ROLE_KEY")
            .env_remove("SUPABASE_SERVICE_KEY")
            .output()
            .with_context(|| format!("running daemon --ingest {source_id}"))?;

        Ok(RunResult {
            stdout: String::from_utf8(output.stdout).context("decoding stdout")?,
            stderr: String::from_utf8(output.stderr).context("decoding stderr")?,
            success: output.status.success(),
        })
    }

    fn connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
            .with_context(|| format!("opening {}", self.db_path.display()))
    }

    fn fake_pty_invocation_count(&self) -> Result<usize> {
        if !self.fake_invocation_log.exists() {
            return Ok(0);
        }
        Ok(fs::read_to_string(&self.fake_invocation_log)?
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count())
    }

    fn write_fake_claude(&self) -> Result<()> {
        fs::write(
            self.fake_state_dir.join("probe.json"),
            serde_json::to_vec_pretty(&json!({
                "auth_healthy": true,
                "structured_output": true,
                "pty_viable": true,
                "browser_tools": true
            }))?,
        )?;
        fs::write(
            &self.fake_program,
            format!(
                "#!/bin/sh\nset -eu\nstate_dir=\"{}\"\nlog_path=\"$state_dir/invocations.log\"\nif [ \"${{1-}}\" = \"--version\" ] || [ \"${{1-}}\" = \"-v\" ]; then\n  printf 'claude 1.2.3\\n'\n  exit 0\nfi\nif [ \"${{1-}}\" = \"probe\" ]; then\n  cat \"$state_dir/probe.json\"\n  exit 0\nfi\nmode=pty\nfor arg in \"$@\"; do\n  if [ \"$arg\" = \"--output-format\" ]; then\n    mode=contract\n    break\n  fi\ndone\nif [ \"$mode\" = contract ]; then\n  cp \"$state_dir/result-source.json\" \"$DAEMON_TEST_CLAUDE_RESULT_PATH\"\n  printf '{{\"event\":\"result\",\"path\":\"%s\"}}\\n' \"$DAEMON_TEST_CLAUDE_RESULT_PATH\"\nelse\n  printf 'run\\n' >> \"$log_path\"\n  cp \"$state_dir/result-source.json\" result.json\n  : > \"$DAEMON_TEST_CLAUDE_CHECKPOINT_PATH\"\n  printf '{{\"event\":\"checkpoint\",\"seq\":1,\"path\":\"%s\"}}\\n' \"$DAEMON_TEST_CLAUDE_CHECKPOINT_PATH\"\nfi\n",
                self.fake_state_dir.display()
            ),
        )?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.fake_program)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&self.fake_program, perms)?;
        }
        Ok(())
    }
}

fn seed_minimal_procontract_notice(
    conn: &Connection,
    process_id: &str,
    notice_id: &str,
    title: &str,
) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO procurement_processes (
          id, canonical_ocid, primary_notice_id, current_stage, buyer_name, title,
          latest_deadline_at, latest_published_at, latest_updated_at, owner_id
        ) VALUES (?1, ?2, ?3, 'normalize', 'Buyer', ?4, '2026-08-31T12:00:00Z',
                  '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', 'local-owner')
        ON CONFLICT(id) DO NOTHING
        ",
        params![process_id, notice_id, notice_id, title],
    )?;
    conn.execute(
        "\
        INSERT INTO contract_notices (
          id, process_id, source_id, source_notice_id, raw_document_id, ocid, title,
          summary, buyer_name, procurement_stage, access_route, deadline_at, published_at,
          source_updated_at, main_category, normalized_payload, owner_id
        ) VALUES (?1, ?2, 'procontract', ?3, ?4, ?5, ?6,
                  'Goods supply fixture', 'Buyer', 'tender', 'portal_authenticated',
                  '2026-08-31T12:00:00Z', '2026-04-01T00:00:00Z',
                  '2026-04-01T00:00:00Z', 'Goods', '{}', 'local-owner')
        ON CONFLICT(id) DO NOTHING
        ",
        params![
            notice_id,
            process_id,
            notice_id,
            format!("raw-{notice_id}"),
            notice_id,
            title,
        ],
    )?;
    Ok(())
}

fn registration_success_contract() -> Value {
    json!({
        "cursor": "{\"last_processed_at\":\"2026-04-13T15:00:00Z\"}",
        "continuation_required": false,
        "registrations": [
            {
                "notice_id": TRACKED_NOTICE_ID,
                "process_id": TRACKED_PROCESS_ID,
                "source_url": "https://procontract.due-north.com/Advert?advertId=pc-resume-001",
                "action": "registered",
                "registered_at": "2026-04-13T15:02:30Z",
                "pre_filter_result": {
                    "goods_check": true,
                    "expiry_check": true,
                    "category": "Goods",
                    "days_until_deadline": 83
                }
            }
        ],
        "summary": {
            "total_evaluated": 1,
            "registered": 1,
            "already_registered": 0,
            "skipped_service": 0,
            "skipped_expiry": 0,
            "failed": 0
        }
    })
}

fn keep_and_backfill_contract() -> Value {
    json!({
        "cursor": "{\"last_checked_at\":\"2026-04-13T17:00:00Z\"}",
        "continuation_required": false,
        "not_yet_responded": [
            {
                "notice_id": TRACKED_NOTICE_ID,
                "process_id": TRACKED_PROCESS_ID,
                "advert_id": TRACKED_NOTICE_ID,
                "source_notice_id": TRACKED_NOTICE_ID,
                "observed_at": "2026-04-13T17:00:00Z"
            },
            {
                "notice_id": BACKFILL_NOTICE_ID,
                "process_id": BACKFILL_PROCESS_ID,
                "advert_id": BACKFILL_NOTICE_ID,
                "source_notice_id": BACKFILL_NOTICE_ID,
                "interest_registered_at": "2026-04-12T11:00:00Z",
                "observed_at": "2026-04-13T17:00:00Z"
            }
        ],
        "enriched_tenders": [],
        "summary": {
            "total_checked": 2,
            "enriched": 0,
            "not_yet_responded": 2,
            "failed": 0
        }
    })
}

fn pending_row(conn: &Connection, process_id: &str, advert_id: &str) -> Result<PendingRow> {
    conn.query_row(
        "\
        SELECT state, observation_count, interest_registered_at, deadline_at,
               last_checked_at, enriched_at
        FROM procontract_pending_enrichment
        WHERE process_id = ?1
          AND notice_id = ?2
          AND advert_id = ?2
        ",
        params![process_id, advert_id],
        |row| {
            Ok(PendingRow {
                state: row.get(0)?,
                observation_count: row.get(1)?,
                interest_registered_at: row.get(2)?,
                deadline_at: row.get(3)?,
                last_checked_at: row.get(4)?,
                enriched_at: row.get(5)?,
            })
        },
    )
    .with_context(|| format!("loading pending enrichment row for {process_id}/{advert_id}"))
}

fn interest_registration_state(conn: &Connection, process_id: &str) -> Result<Option<String>> {
    conn.query_row(
        "\
        SELECT state
        FROM process_stage_status
        WHERE process_id = ?1 AND stage = 'interest_registration'
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        ",
        params![process_id],
        |row| row.get(0),
    )
    .optional()
    .context("loading interest registration state")
}

fn count_interest_registration_rows(conn: &Connection, process_id: &str) -> Result<i64> {
    conn.query_row(
        "\
        SELECT COUNT(*)
        FROM process_stage_status
        WHERE process_id = ?1 AND stage = 'interest_registration'
        ",
        params![process_id],
        |row| row.get(0),
    )
    .context("counting interest registration stage rows")
}

fn count_pending_rows(conn: &Connection, process_id: &str, advert_id: &str) -> Result<i64> {
    conn.query_row(
        "\
        SELECT COUNT(*)
        FROM procontract_pending_enrichment
        WHERE process_id = ?1 AND advert_id = ?2
        ",
        params![process_id, advert_id],
        |row| row.get(0),
    )
    .context("counting pending enrichment rows")
}

fn latest_job_output(conn: &Connection, scope_id: &str) -> Result<Value> {
    let payload: String = conn.query_row(
        "\
        SELECT output_payload
        FROM workflow_jobs
        WHERE kind = 'sync_source'
          AND scope_id = ?1
          AND parent_job_id IS NULL
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        ",
        params![scope_id],
        |row| row.get(0),
    )?;
    serde_json::from_str(&payload).context("parsing latest job output payload")
}

fn latest_job_input(conn: &Connection, scope_id: &str) -> Result<Value> {
    let payload: String = conn.query_row(
        "\
        SELECT input_payload
        FROM workflow_jobs
        WHERE kind = 'sync_source'
          AND scope_id = ?1
          AND parent_job_id IS NULL
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        ",
        params![scope_id],
        |row| row.get(0),
    )?;
    serde_json::from_str(&payload).context("parsing latest job input payload")
}

fn assert_run_success(result: &RunResult, label: &str) -> Result<()> {
    ensure!(
        result.success,
        "{label} should succeed\nstdout:\n{}\nstderr:\n{}",
        result.stdout,
        result.stderr
    );
    Ok(())
}

fn require_object_field<'a>(value: &'a Value, field: &str) -> Result<&'a Value> {
    value
        .get(field)
        .ok_or_else(|| anyhow!("missing JSON field `{field}`"))
}

fn repo_root() -> Result<PathBuf> {
    fs::canonicalize(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .context("resolving repository root")
}

fn read_text_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

fn relative_display<'a>(path: &'a Path, root: &'a Path) -> std::path::Display<'a> {
    path.strip_prefix(root)
        .map_or_else(|_| path.display(), |relative| relative.display())
}

fn ensure_terms_absent_outside_enrich(
    root: &Path,
    enrich_dir: &Path,
    repo_root: &Path,
    terms: &[&str],
) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.starts_with(enrich_dir) {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            ensure_terms_absent_outside_enrich(&path, enrich_dir, repo_root, terms)?;
        } else if file_type.is_file() {
            let contents = read_text_file(&path)?;
            for term in terms {
                ensure!(
                    !contents.contains(term),
                    "`{term}` leaked outside ProContract enrich skill into {}",
                    relative_display(&path, repo_root)
                );
            }
        }
    }
    Ok(())
}

#[test]
fn guard_scope_001_skill_selector_boundary() -> Result<()> {
    const WORKLIST_BOUNDARY_TERMS: &[&str] = &[
        "awaitingAccessTargets",
        "direct_target_worklist",
        "direct_targets_received",
        "direct_targets_visited",
        "scan_direct_union_count",
        "direct_visit_attempted",
        "directly_visited",
        "target_evidence",
    ];

    let repo_root = repo_root()?;
    let skills_root = repo_root.join("skills");
    let enrich_dir = skills_root
        .join("procurement-ingestion")
        .join("procontract-enrich-tender");

    let enrich_contract = [
        read_text_file(&enrich_dir.join("SKILL.md"))?,
        read_text_file(&enrich_dir.join("result.schema.json"))?,
        read_text_file(&enrich_dir.join("checkpoint.schema.json"))?,
        read_text_file(&enrich_dir.join("canary.inputs.jsonl"))?,
        read_text_file(&enrich_dir.join("golden.assertions.yaml"))?,
    ]
    .join("\n");
    for term in WORKLIST_BOUNDARY_TERMS {
        ensure!(
            enrich_contract.contains(term),
            "ProContract enrich skill contract should contain `{term}`"
        );
    }

    ensure_terms_absent_outside_enrich(
        &skills_root,
        &enrich_dir,
        &repo_root,
        WORKLIST_BOUNDARY_TERMS,
    )?;

    Ok(())
}

#[test]
fn m3_step_001_register_creates_awaiting_access_without_refire() -> Result<()> {
    let harness = ProcontractRunHarness::new()?;
    harness.seed_notice(
        TRACKED_PROCESS_ID,
        TRACKED_NOTICE_ID,
        "Supply of awaiting-access test widgets",
    )?;

    let first = harness.run_ingest("procontract-interest", &registration_success_contract())?;
    assert_run_success(&first, "first register-interest run")?;

    let conn = harness.connection()?;
    let pending = pending_row(&conn, TRACKED_PROCESS_ID, TRACKED_NOTICE_ID)?;
    ensure!(pending.state == "awaiting_access");
    ensure!(pending.observation_count == 0);
    ensure!(pending.interest_registered_at == "2026-04-13T15:02:30Z");
    ensure!(pending.deadline_at.as_deref() == Some("2026-08-31T12:00:00Z"));
    ensure!(pending.last_checked_at.is_none());
    ensure!(pending.enriched_at.is_none());
    ensure!(
        interest_registration_state(&conn, TRACKED_PROCESS_ID)?.as_deref() == Some("completed")
    );
    ensure!(count_interest_registration_rows(&conn, TRACKED_PROCESS_ID)? == 1);
    ensure!(count_pending_rows(&conn, TRACKED_PROCESS_ID, TRACKED_NOTICE_ID)? == 1);
    ensure!(harness.fake_pty_invocation_count()? == 1);

    let second = harness.run_ingest("procontract-interest", &registration_success_contract())?;
    assert_run_success(&second, "second register-interest run")?;

    let conn = harness.connection()?;
    ensure!(harness.fake_pty_invocation_count()? == 1);
    ensure!(count_interest_registration_rows(&conn, TRACKED_PROCESS_ID)? == 1);
    ensure!(count_pending_rows(&conn, TRACKED_PROCESS_ID, TRACKED_NOTICE_ID)? == 1);
    let output = latest_job_output(&conn, "procontract-interest")?;
    ensure!(require_object_field(&output, "registrations")?.as_i64() == Some(0));
    ensure!(
        interest_registration_state(&conn, TRACKED_PROCESS_ID)?.as_deref() == Some("completed")
    );
    Ok(())
}

#[test]
fn m3_step_002_enrich_keep_and_backfill() -> Result<()> {
    let harness = ProcontractRunHarness::new()?;
    harness.seed_notice(
        TRACKED_PROCESS_ID,
        TRACKED_NOTICE_ID,
        "Supply of tracked awaiting-access widgets",
    )?;
    harness.seed_notice(
        BACKFILL_PROCESS_ID,
        BACKFILL_NOTICE_ID,
        "Supply of backfilled awaiting-access widgets",
    )?;
    assert_run_success(
        &harness.run_ingest("procontract-interest", &registration_success_contract())?,
        "register-interest setup run",
    )?;

    let enrich = harness.run_ingest("procontract-enrich", &keep_and_backfill_contract())?;
    assert_run_success(&enrich, "enrich keep/backfill run")?;

    let conn = harness.connection()?;
    let input_payload = latest_job_input(&conn, "procontract-enrich")?;
    let awaiting_targets = input_payload
        .pointer("/job_parameters/awaitingAccessTargets")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("procontract-enrich input missing awaitingAccessTargets"))?;
    ensure!(
        awaiting_targets
            .iter()
            .any(|target| target["advertId"] == TRACKED_NOTICE_ID
                && target["sourceNoticeId"] == TRACKED_NOTICE_ID
                && target["processId"] == TRACKED_PROCESS_ID),
        "expected enrich job input to carry tracked awaiting-access target"
    );

    let tracked = pending_row(&conn, TRACKED_PROCESS_ID, TRACKED_NOTICE_ID)?;
    ensure!(tracked.state == "awaiting_access");
    ensure!(tracked.observation_count == 1);
    ensure!(tracked.interest_registered_at == "2026-04-13T15:02:30Z");
    ensure!(tracked.last_checked_at.as_deref() == Some("2026-04-13T17:00:00Z"));
    ensure!(tracked.enriched_at.is_none());

    let backfilled = pending_row(&conn, BACKFILL_PROCESS_ID, BACKFILL_NOTICE_ID)?;
    ensure!(backfilled.state == "awaiting_access");
    ensure!(backfilled.observation_count == 1);
    ensure!(backfilled.interest_registered_at == "2026-04-12T11:00:00Z");
    ensure!(backfilled.last_checked_at.as_deref() == Some("2026-04-13T17:00:00Z"));
    ensure!(backfilled.enriched_at.is_none());
    Ok(())
}
