# Stage 4b Document Extraction Docling Runtime Unavailable - QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| Type | machine_setup_blocker / integration_mismatch |
| Severity | high |
| Reproducibility | deterministic on this machine |
| Affected Component | `crates/daemon/src/main.rs::run_extract_document_structure_job`, `crates/daemon/src/pdf/engine.rs::PdfExtractionEngine::is_available`, `scripts/docling-extract.py` |
| Working Analogue | Stage 4b ProContract enrichment now produces a valid result contract and persists downloaded raw documents |
| Scope | The E2E run cannot advance from downloaded tender documents to document extractions/product candidates/search profiles because Docling is not installed/configured locally. |
| Environment | macOS 26.3.1 arm64; Cargo 1.96 nightly; Bun 1.3.13; Python 3.13.12 |
| Git State | `main` at `39a7935 fix procontract auth-blocked summary count`; shared checkout has unrelated staged `.DS_Store` and `bin/__pycache__/*` artifacts |

---

## REPRODUCTION PROTOCOL

Follow these steps exactly from `/Users/zacfarrell/Desktop/Testproject1`.

### Prerequisites

- Existing E2E state root: `/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z`
- Existing E2E artifact root: `/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z`
- Existing Stage 4b retry log: `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-04b-procontract-enrich-retry-after-followup-fix.log`
- No local Docling runtime installed/configured in Python or `$HOME/.docling-venv`.

### Step 1: Verify Stage 4b enrichment completed and extraction drain ran

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,220p' "$E2E_LOG_ROOT/stage-04b-procontract-enrich-retry-after-followup-fix.log"
```

Expected: Stage 4b should enrich tender documents and then extract them.

Actual:

```text
$ env AGENT_BROWSER_CDP_PORT=9222 /tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon --state-root /Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z --runtime-dir /Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z --artifact-root /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z --ingest procontract-enrich
2026-05-14T15:24:54.969174Z  INFO daemon bootstrap ready socket_path=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z/daemon.sock state_db=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/state.db artifact_root=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
2026-05-14T15:24:54.970047Z  WARN L4 startup sync degraded before cross-authority dispatch; local operation continues failure=remote-owned pull unavailable for shared_source_configs
2026-05-14T15:40:23.505956Z  INFO PTY skill produced a valid result contract; dispatching SIGTERM after stable period runtime_ids_pid=45679 runtime_ids_pgid=45679
2026-05-14T15:40:23.848526Z  INFO extraction job drain complete processed=3
procontract tender enrichment completed: job_id=job-109da01929119faeea0cebfb7e4785c12d3153038521766e980900aa5e0c3ae6 enriched=1 extractions=3
```

### Step 2: Verify extraction jobs failed

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT kind, status, COUNT(*) AS count
FROM workflow_jobs
GROUP BY kind,status
ORDER BY kind,status;

SELECT id, kind, status, scope_kind, scope_id, attempt_count, last_error, created_at, started_at, completed_at
FROM workflow_jobs
WHERE kind='extract_document_structure'
ORDER BY created_at DESC LIMIT 10;
"
```

Expected: three structural extraction jobs should be completed or should have produced extraction rows.

Actual:

```text
kind                        status     count
--------------------------  ---------  -----
extract_document_structure  failed     3
sync_source                 completed  8
sync_source                 failed     4
sync_source                 pending    2
sync_source                 retry      3

id                                                                            kind                        status  scope_kind  scope_id                                 attempt_count  last_error                                                                                                 created_at           started_at                        completed_at
----------------------------------------------------------------------------  --------------------------  ------  ----------  ---------------------------------------  -------------  ---------------------------------------------------------------------------------------------------------  -------------------  --------------------------------  --------------------------------
job-extract-ca6b80ef3ce373a7c1778c03b5a0dcf41f32ca1cf461c2c2bd2e1e44a39f04c1  extract_document_structure  failed  notice      pc-ecd07d9f-5249-f111-813c-005056b64545  1              Docling runtime unavailable for wrapper /Users/zacfarrell/Desktop/Testproject1/scripts/docling-extract.py  2026-05-14 15:40:23  2026-05-14T15:40:23.846834+00:00  2026-05-14T15:40:23.848046+00:00
job-extract-1cb941ce7e90472ce26cdc084fe33f844796cd69eedd365f31b34209122cf593  extract_document_structure  failed  notice      pc-ecd07d9f-5249-f111-813c-005056b64545  1              Docling runtime unavailable for wrapper /Users/zacfarrell/Desktop/Testproject1/scripts/docling-extract.py  2026-05-14 15:40:23  2026-05-14T15:40:23.848171+00:00  2026-05-14T15:40:23.848277+00:00
job-extract-47b82860ecd39dacf4d1e54377080b2cc94f5f7b8f34db94bf111023f132600e  extract_document_structure  failed  notice      pc-ecd07d9f-5249-f111-813c-005056b64545  1              Docling runtime unavailable for wrapper /Users/zacfarrell/Desktop/Testproject1/scripts/docling-extract.py  2026-05-14 15:40:23  2026-05-14T15:40:23.848362+00:00  2026-05-14T15:40:23.848448+00:00
```

### Step 3: Verify documents were downloaded but extraction products are missing

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT COUNT(*) AS raw_docs_with_files FROM raw_documents WHERE storage_path IS NOT NULL;
SELECT COUNT(*) AS document_extractions FROM document_extractions;
SELECT COUNT(*) AS semantic_extractions FROM document_semantic_extractions;
SELECT COUNT(*) AS product_candidates FROM product_candidates;
SELECT COUNT(*) AS search_profiles FROM search_profiles;
"
```

Expected: downloaded documents should lead to structural/semantic extraction rows and then candidate/search-profile state.

Actual:

```text
raw_docs_with_files
-------------------
3
document_extractions
--------------------
0
semantic_extractions
--------------------
0
product_candidates
------------------
0
search_profiles
---------------
0
```

### Step 4: Verify Docling is absent from the local runtime

```bash
which docling || true
ls -l "$HOME/.docling-venv/bin/docling" 2>&1 || true
python3 scripts/docling-extract.py --version
```

Expected: a configured Docling runtime should print an installed Docling version.

Actual:

```text
docling not found
ls: /Users/zacfarrell/.docling-venv/bin/docling: No such file or directory
docling import unavailable: No module named 'docling'
```

---

## EVIDENCE BUNDLE

### E0 - Environment Evidence

**E0.1 - Current git state**

```bash
git status --short --branch
git log --oneline -5
```

```text
## main...origin/main
M  .DS_Store
A  bin/__pycache__/br-claimcpython-313.pyc
A  bin/__pycache__/br-completecpython-313.pyc
39a7935 fix procontract auth-blocked summary count
75aea0a docs: add procontract enrich current-state handoff
9731b16 docs: record procontract enrichment followup timeout
a20cd98 docs: record claude quota pause for e2e
413c5a7 fix procontract enrichment missing-result failure
```

**E0.2 - Runtime versions**

```bash
uname -a
sw_vers
cargo --version
rustc --version
bun --version
python3 --version
```

```text
Darwin Mac 25.3.0 Darwin Kernel Version 25.3.0: Wed Jan 28 20:56:34 PST 2026; root:xnu-12377.91.3~2/RELEASE_ARM64_T8122 arm64
ProductName:		macOS
ProductVersion:		26.3.1
ProductVersionExtra:	(a)
BuildVersion:		25D771280a
cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)
rustc 1.96.0-nightly (bcf3d36c9 2026-03-19)
1.3.13
Python 3.13.12
```

**E0.3 - E2E roots**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
printf 'state=%s\nruntime=%s\nartifact=%s\nlog=%s\ndaemon=%s\n' \
  "$E2E_STATE_ROOT" "$E2E_RUNTIME_DIR" "$E2E_ARTIFACT_ROOT" "$E2E_LOG_ROOT" "$E2E_DAEMON"
```

```text
state=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z
runtime=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z
artifact=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
log=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs
daemon=/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
```

### E2 - Reproduction Evidence

**E2.1 - Stage 4b retry produced a result contract and started extraction drain**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
tail -120 "$E2E_LOG_ROOT/stage-04b-procontract-enrich-retry-after-followup-fix.log"
```

```text
2026-05-14T15:40:23.505956Z  INFO PTY skill produced a valid result contract; dispatching SIGTERM after stable period runtime_ids_pid=45679 runtime_ids_pgid=45679
2026-05-14T15:40:23.848526Z  INFO extraction job drain complete processed=3
procontract tender enrichment completed: job_id=job-109da01929119faeea0cebfb7e4785c12d3153038521766e980900aa5e0c3ae6 enriched=1 extractions=3
```

**E2.2 - Latest ProContract enrichment session completed after result contract**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, job_id, skill_path, execution_mode, cli_vendor, status, exit_code,
       cleanup_verdict, terminal_output_bytes, terminal_output_path, run_manifest_path, created_at
FROM agent_session_logs
WHERE job_id='job-109da01929119faeea0cebfb7e4785c12d3153038521766e980900aa5e0c3ae6';
"
```

```text
id                                                                            job_id                                                                skill_path                                                       execution_mode  cli_vendor  status     exit_code  cleanup_verdict                   terminal_output_bytes  terminal_output_path                                                                                                                                                                                 run_manifest_path  created_at
----------------------------------------------------------------------------  --------------------------------------------------------------------  ---------------------------------------------------------------  --------------  ----------  ---------  ---------  --------------------------------  ---------------------  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------  -----------------  -------------------
session-pty-d1a6eade36cd8d59fd0bba0bcb938aa1e51aa13ff2f05e395620f27d81bf9896  job-109da01929119faeea0cebfb7e4785c12d3153038521766e980900aa5e0c3ae6  skills/procurement-ingestion/procontract-enrich-tender/SKILL.md  pty             claude      completed  143        terminated_after_result_contract  77                     /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/sessions/2026/05/session-pty-d1a6eade36cd8d59fd0bba0bcb938aa1e51aa13ff2f05e395620f27d81bf9896/stream.index.jsonl                     2026-05-14 15:40:23
```

**E2.3 - Stage 4b result contract was valid but partially blocked upstream**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,240p' \
  "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-d1a6eade36cd8d59fd0bba0bcb938aa1e51aa13ff2f05e395620f27d81bf9896/workdir/result.json"
```

```text
"status": "blocked"
"documents_count": 3
"documents_available_total": 12
"summary": {
  "total_checked": 1,
  "enriched": 1,
  "not_yet_responded": 1,
  "failed": 1
}
"blockers": [
  {
    "code": "AGENT_STALLED",
    "message": "15-minute processing budget exhausted. Collected 40 activities across all 4 pages of the Recently added list. Processed 1 of 40 activities (BEST START FAMILY HUBS OUTREACH SERVICES, Oldham Council). Enriched core RFx data successfully but only 3 of 12 documents downloaded -- 9 documents failed due to CDP daemon resource contention (EAGAIN errors on concurrent download attempts). Remaining 39 activities on pages 1-4 were not individually processed."
  }
]
```

This upstream blocked status is not the immediate blocker in this manifest. The immediate blocker is that even the three successfully downloaded documents cannot be extracted on this machine.

**E2.4 - Downloaded raw documents exist**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, source_id, content_type, storage_path, substr(source_url,1,90) AS source_url_prefix, created_at
FROM raw_documents
WHERE storage_path IS NOT NULL
ORDER BY created_at DESC LIMIT 10;
"
```

```text
id                                                                                                            source_id           content_type                                                             storage_path                                                                                                                                               source_url_prefix                                                                           created_at
------------------------------------------------------------------------------------------------------------  ------------------  -----------------------------------------------------------------------  ---------------------------------------------------------------------------------------------------------------------------------------------------------  ------------------------------------------------------------------------------------------  -------------------
raw-pc-ecd07d9f-5249-f111-813c-005056b64545-0e7e8e1d05fb47625925cbdc743cefcd692954904ca9b9b0257d47c4fb27302b  procontract-enrich  application/vnd.openxmlformats-officedocument.wordprocessingml.document  /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/downloads/procontract/786663/ITT_Template_Part_A_Family_Hubs_v2.docx    https://procontract.due-north.com/FileUploads/ViewAttachment?attachmentId=d0afb5ef-b34e-f1  2026-05-14 15:40:23
raw-pc-ecd07d9f-5249-f111-813c-005056b64545-2c1a2890931f208f3f1340b6b0d5b98b5b027047916f4341533952276a92a8b6  procontract-enrich  application/vnd.openxmlformats-officedocument.wordprocessingml.document  /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/downloads/procontract/786663/ITT_Template_Part_B_Family_Hubs_v1.docx    https://procontract.due-north.com/FileUploads/ViewAttachment?attachmentId=ce0a3cc8-f14a-f1  2026-05-14 15:40:23
raw-pc-ecd07d9f-5249-f111-813c-005056b64545-06cbb57ca2a43c2b1de001cbdfa9d948b86595cef4f84131269f1fc139c832d5  procontract-enrich  application/vnd.openxmlformats-officedocument.wordprocessingml.document  /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/downloads/procontract/786663/Copy_of_Appendix_A_for_completion_v1.docx  https://procontract.due-north.com/FileUploads/ViewAttachment?attachmentId=c90f4eca-ee4a-f1  2026-05-14 15:40:23
```

**E2.5 - No downstream extraction/candidate/search rows exist**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT COUNT(*) AS document_extractions FROM document_extractions;
SELECT COUNT(*) AS semantic_extractions FROM document_semantic_extractions;
SELECT COUNT(*) AS product_candidates FROM product_candidates;
SELECT COUNT(*) AS search_profiles FROM search_profiles;
"
```

```text
document_extractions
--------------------
0
semantic_extractions
--------------------
0
product_candidates
------------------
0
search_profiles
---------------
0
```

**E2.6 - Docling package and model artifacts are absent**

```bash
which docling || true
ls -l "$HOME/.docling-venv/bin/docling" 2>&1 || true
ls -ld "$HOME/.cache/docling" "$HOME/.cache/docling/models" 2>&1 || true
python3 - <<'PY'
try:
    import docling
    print('docling import ok', getattr(docling, '__version__', '<no version>'))
except Exception as exc:
    print(type(exc).__name__ + ': ' + str(exc))
PY
```

```text
docling not found
ls: /Users/zacfarrell/.docling-venv/bin/docling: No such file or directory
ls: /Users/zacfarrell/.cache/docling: No such file or directory
ls: /Users/zacfarrell/.cache/docling/models: No such file or directory
ModuleNotFoundError: No module named 'docling'
```

### E3 - Codepath Evidence

**E3.1 - Extraction job aborts when the engine is unavailable**

`crates/daemon/src/main.rs:1538`

```rust
let engine_config = PdfEngineConfig {
    docling_wrapper_path: request
        .docling_wrapper_path
        .clone()
        .unwrap_or(default_docling_wrapper_path()?),
    model_artifacts_path: request
        .model_artifacts_path
        .clone()
        .unwrap_or(default_docling_artifacts_path()?),
    document_timeout_secs: request.document_timeout_secs.unwrap_or(300),
    wall_clock_timeout_secs: request.wall_clock_timeout_secs.unwrap_or(330),
    python_bin: default_docling_python_bin(),
};
let engine = PdfExtractionEngine::new(engine_config.clone());
if !engine.is_available()? {
    bail!(
        "Docling runtime unavailable for wrapper {}",
        engine_config.docling_wrapper_path.display()
    );
}
```

**E3.2 - Default Docling locations**

`crates/daemon/src/main.rs:2464`

```rust
fn default_docling_wrapper_path() -> Result<PathBuf> {
    Ok(repo_root()?.join("scripts/docling-extract.py"))
}

fn default_docling_artifacts_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("DOCLING_ARTIFACTS_PATH") {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| {
        anyhow!("DOCLING_ARTIFACTS_PATH is unset and HOME is unavailable for default model path")
    })?;
    Ok(PathBuf::from(home).join(".cache/docling/models"))
}

fn default_docling_python_bin() -> PathBuf {
    // Prefer the docling venv if it exists, otherwise fall back to system python3.
    let venv_python = PathBuf::from(std::env::var_os("DOCLING_PYTHON_BIN").unwrap_or_else(|| {
        let home = std::env::var_os("HOME").unwrap_or_default();
        PathBuf::from(home)
            .join(".docling-venv/bin/python")
            .into_os_string()
    }));
    if venv_python.is_file() {
        return venv_python;
    }
    PathBuf::from("python3")
}
```

**E3.3 - Engine availability requires wrapper, model artifacts, and Python package**

`crates/daemon/src/pdf/engine.rs:143`

```rust
pub fn is_available(&self) -> Result<bool> {
    if !self.wrapper_exists()? || !self.artifacts_dir_exists()? {
        return Ok(false);
    }

    Ok(Self::probe_docling_version_with_bin(&self.config.python_bin)?.is_some())
}
```

`crates/daemon/src/pdf/engine.rs:315`

```rust
fn probe_docling_version_with_bin(python_bin: &Path) -> Result<Option<String>> {
    let output = StdCommand::new(python_bin)
        .arg("-c")
        .arg("from importlib.metadata import version; print(version('docling'))")
        .output()
        .context("spawning python for Docling health probe")?;

    if !output.status.success() {
        return Ok(None);
    }
```

**E3.4 - Wrapper itself reports missing Docling package**

`scripts/docling-extract.py:51`

```python
def installed_docling_version() -> str:
    try:
        return importlib_metadata.version("docling")
    except importlib_metadata.PackageNotFoundError:
        try:
            import docling  # type: ignore
        except ImportError as exc:
            raise ImportError(f"docling import unavailable: {exc}") from exc
```

---

## CODEPATH COMPARISON

### Working Path: ProContract Enrichment Persistence

Entry point: `crates/daemon/src/main.rs` ProContract enrichment live loop.

Flow:

1. The PTY skill wrote a valid `result.json`.
2. The daemon terminated the PTY session after observing the stable result contract.
3. The daemon persisted one tender document set and three `raw_documents` with `storage_path`.
4. The daemon enqueued and drained three structural extraction jobs.

Evidence: E2.1, E2.2, E2.4.

### Broken Path: Structural Document Extraction

Entry point: `crates/daemon/src/main.rs::run_extract_document_structure_job`.

Flow:

1. Each raw document created an `extract_document_structure` workflow job.
2. Each job built a `PdfEngineConfig` pointing at `scripts/docling-extract.py`, `$HOME/.cache/docling/models`, and either `$HOME/.docling-venv/bin/python` or `python3`.
3. `PdfExtractionEngine::is_available()` returned false.
4. The job failed with `Docling runtime unavailable for wrapper /Users/zacfarrell/Desktop/Testproject1/scripts/docling-extract.py`.

Evidence: E2.2, E2.5, E2.6, E3.1, E3.2, E3.3.

### Divergence Points

| # | Working Path | Broken Path | Potential Impact |
|---|--------------|-------------|------------------|
| 1 | ProContract skill and persistence can produce durable raw documents. | Extraction requires local Docling dependencies that are absent. | Blocks all downstream document understanding and product candidate generation. |
| 2 | Stage 4b command exits 0 after result contract. | Three extraction jobs fail during the drain. | The command-level success masks that Stage 5-8 document extraction remains incomplete. |
| 3 | Wrapper file exists and is executable. | Python cannot import `docling`, and model artifact directory is absent. | Engine availability check correctly rejects the extraction runtime. |

---

## ROOT CAUSE HYPOTHESES

### H1: Required Docling runtime and model artifacts are not installed on the duplicated machine - LIKELIHOOD: HIGH

Description: The daemon requires a local Python Docling package and offline model artifacts for extraction. This machine has neither `docling` on `PATH`, nor `$HOME/.docling-venv/bin/docling`, nor `$HOME/.cache/docling/models`, and Python cannot import `docling`.

Supporting evidence: E2.6, E3.2, E3.3, E3.4.

Contradicting evidence: none.

Verification steps:

1. Install/configure Docling in the expected runtime, or set `DOCLING_PYTHON_BIN` and `DOCLING_ARTIFACTS_PATH`.
2. Run `python3 scripts/docling-extract.py --version` or the configured Python equivalent.
3. Confirm it prints a Docling version and that the model artifact directory exists.

Fix scope: machine setup or documented bootstrap scripts. Product code may not need changes if this run expects Docling as an external prerequisite.

### H2: The daemon should preflight Docling before Stage 4b reports success - LIKELIHOOD: MEDIUM

Description: The Stage 4b command reported `procontract tender enrichment completed ... extractions=3` even though all three extraction jobs failed. The job failures are in SQLite, but the top-level command output can be misread as extraction success.

Supporting evidence: E2.1, E2.2, E2.5.

Contradicting evidence: The handoff classifies required local tool absence as a machine setup blocker, so the immediate pause is still correct.

Verification steps:

1. Decide whether command output should distinguish `extraction_jobs_processed` from `extraction_jobs_completed`.
2. Inspect `drain_pending_extraction_jobs` and any `LiveLoopReport` fields used for completion summaries.
3. Add/adjust regression coverage only if product behavior is deemed wrong after machine setup is repaired.

Fix scope: `crates/daemon/src/main.rs::drain_pending_extraction_jobs` and ProContract enrichment completion summary.

### H3: The Docling wrapper path exists but cannot use the daemon-selected Python interpreter - LIKELIHOOD: MEDIUM

Description: `scripts/docling-extract.py` is executable, but `default_docling_python_bin()` falls back to `python3` because `$HOME/.docling-venv/bin/python` is missing. System Python has no `docling` package. If a separate Docling installation exists elsewhere, the daemon is not pointed at it.

Supporting evidence: E2.6, E3.2, E3.4.

Contradicting evidence: No alternate Docling install was found via `which docling`.

Verification steps:

1. Locate any existing Docling Python environment.
2. If present, export `DOCLING_PYTHON_BIN=/path/to/python` and `DOCLING_ARTIFACTS_PATH=/path/to/models`.
3. Re-run the version probe and a structural extraction job.

Fix scope: machine environment variables, local bootstrap docs, or runtime preflight documentation.

---

## INVESTIGATION ROADMAP

### Step 1: Repair or prove Docling runtime setup

Run:

```bash
which docling || true
ls -l "$HOME/.docling-venv/bin/docling" 2>&1 || true
python3 scripts/docling-extract.py --version
ls -ld "$HOME/.cache/docling/models" 2>&1 || true
```

If these still show missing runtime or models, install/configure Docling before touching code.

If these pass, proceed to Step 2.

### Step 2: Re-run the smallest E2E extraction path

Source the existing E2E environment:

```bash
source /Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
cd "$REPO_ROOT"
```

Then use the approved repaired path from the fixer. If no explicit requeue path is added, retry the Stage 4b command against the same roots:

```bash
run_logged stage-04b-procontract-enrich-retry-after-docling-setup \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
  --state-root "$E2E_STATE_ROOT" \
  --runtime-dir "$E2E_RUNTIME_DIR" \
  --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest procontract-enrich
```

If extraction jobs still fail with a Docling runtime error, return to Step 1.

If extraction jobs complete, proceed to Step 3.

### Step 3: Verify downstream rows

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT kind, status, COUNT(*) FROM workflow_jobs GROUP BY kind,status ORDER BY kind,status;
SELECT COUNT(*) AS document_extractions FROM document_extractions;
SELECT COUNT(*) AS semantic_extractions FROM document_semantic_extractions;
SELECT COUNT(*) AS product_candidates FROM product_candidates;
SELECT COUNT(*) AS search_profiles FROM search_profiles;
"
```

If document extraction rows exist, resume the machine-fork E2E run from Stages 2-8 follow-up validation and spotlight selection.

If extraction rows are still absent after a healthy runtime, investigate H2 or create a follow-up product error manifest.

---

## FIX CONSTRAINTS

1. Do not delete the existing E2E state root, runtime dir, artifact root, downloaded documents, session logs, or failed job records.
2. Do not edit `PRAGMA user_version` or otherwise paper over schema state.
3. Do not rerun Stage 4a portal interest registration as a cleanup step.
4. Use Bun for TypeScript and Cargo for Rust. Do not use npm, yarn, or pnpm.
5. If product code changes are required, they must be fixed outside this QA validator session and verified before QA resumes.

---

## ACCEPTANCE CRITERIA

### Functional

- [ ] `python3 scripts/docling-extract.py --version` or the configured `DOCLING_PYTHON_BIN` equivalent prints a Docling version.
- [ ] `$HOME/.cache/docling/models` exists, or `DOCLING_ARTIFACTS_PATH` points to an existing model artifact directory.
- [ ] Re-running the approved Stage 4b/extraction retry no longer creates `extract_document_structure` jobs failed with `Docling runtime unavailable`.
- [ ] `document_extractions` contains rows for the downloaded ProContract documents.
- [ ] The deterministic pipeline can proceed to semantic extraction/product candidate/search-profile state, or emits a new structured blocker unrelated to Docling setup.

### Evidence-Based

- [ ] The workflow job query shows no latest Docling-runtime failure for the retried extraction jobs.
- [ ] The row-count query changes from `document_extractions=0` to one or more rows.
- [ ] The validation manifest records the before/after Docling setup evidence.

### Regression

- [ ] `cargo test -p daemon --bin daemon procontract_enrich_blocked_result_is_durable` passes.
- [ ] `cargo test -p daemon --bin daemon procontract_enrich_missing_result_marks_job_failed` passes.
- [ ] Any Docling-specific daemon tests either pass or auto-skip only when the runtime is intentionally absent.

### Test

- [ ] If code is changed, add or update a regression that prevents command output from implying extraction success when every drained extraction job failed.

---

## FILE MAP

| File | Role | Lines Referenced |
|------|------|------------------|
| `crates/daemon/src/main.rs` | Builds Docling extraction config, fails extraction job when runtime unavailable, drains extraction jobs after ProContract enrichment | 1538-1563, 2464-2490, 16908-16944 |
| `crates/daemon/src/pdf/engine.rs` | Checks wrapper existence, model artifact directory, and Docling Python package availability | 143-149, 299-359 |
| `scripts/docling-extract.py` | Python wrapper requiring the `docling` package and model artifacts | 1-90 |
| `docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md` | QA resume ledger updated with this pause | header, pause ledger, stage results, final decision |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-04b-procontract-enrich-retry-after-followup-fix.log` | Exact daemon command output for successful enrichment plus failed extraction-drain phase | full log |
| `state-e2e-machine-fork-20260512T195251Z/state.db` | SQLite evidence for failed extraction jobs and missing downstream rows | `workflow_jobs`, `raw_documents`, `document_extractions`, `document_semantic_extractions`, `product_candidates`, `search_profiles` |

