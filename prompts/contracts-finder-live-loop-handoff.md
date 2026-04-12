# Handoff: Wire Contracts Finder Into Full Live Loop & Validate

> ULTRATHINK and be meticulous. This task wires a proven skill into production scheduling and validates the complete operational pipeline.

> **PREREQUISITE**: The Contracts Finder ingestion skill from Step 1 must be working — the agent scrapes real data and the daemon persists it to SQLite via `--skill-path`. This handoff assumes that is complete.

## Mission

Wire the working Contracts Finder ingestion skill into the daemon's **automated live loop** with:
1. Cron-scheduled execution (daily ingestion + periodic health canary)
2. Full pipeline flow: ingest → persist → manifest generation → finalize
3. Continuation support for large result sets (>100 notices)
4. Complete monitoring: PTY forensic capture, run manifests, structured traces, error manifests
5. Incremental cursor-based scraping (only new/updated contracts)
6. Desktop app integration: contracts visible in Queue, detail, sessions, errors views
7. Validated via computer use on the Tauri desktop app

---

## Codebase Orientation

### Key Files to Modify

```
crates/daemon/src/main.rs           # --ingest handler, live loop, finalize_success
crates/daemon/src/sources/registry.rs  # Source definitions (CF already registered)
crates/daemon/src/scheduler/cron.rs    # Cron dispatch logic
crates/daemon/src/db/sqlite.rs         # Source seeding
```

### Key Files to Read (understand, not modify)

```
crates/daemon/src/main.rs            # run_fts_live_loop() — the pattern to follow
crates/daemon/src/jobs/runner.rs     # Job claiming, lease heartbeat
crates/daemon/src/jobs/claim.rs      # Atomic job claim
crates/daemon/src/agents/arbiter.rs  # Resource arbiter (browser slots, agent slots)
crates/daemon/src/sessions/reaper.rs # Cleanup verdicts
crates/daemon/src/tracing/forensic.rs  # PTY capture artifacts
crates/daemon/src/tracing/structured.rs # Control/protocol events
crates/daemon/src/validation/policy.rs  # Validation dry-run
crates/daemon/src/errors/manifest.rs    # Error manifest generation
```

### Daemon Startup Commands

```bash
# Kill any existing daemon
pkill -f "target/release/daemon"

# Run live loop for contracts-finder
cargo run --release --bin daemon -- --ingest contracts-finder \
  --state-root "$HOME/Library/Application Support/com.procintel.desktop/state" \
  --runtime-dir "$HOME/Library/Application Support/com.procintel.desktop/runtime" \
  --artifact-root "$HOME/Library/Application Support/com.procintel.desktop/artifacts"

# Run daemon in serve mode (for desktop app + cron)
cargo run --release --bin daemon -- --serve \
  --state-root "$HOME/Library/Application Support/com.procintel.desktop/state" \
  --runtime-dir "$HOME/Library/Application Support/com.procintel.desktop/runtime" \
  --artifact-root "$HOME/Library/Application Support/com.procintel.desktop/artifacts"
```

---

## What Must Be Built

### Part 1: Enable `--ingest contracts-finder`

**File: `crates/daemon/src/main.rs`** (around line 949-960)

Current code:
```rust
if let Some(source_id) = &cli.ingest {
    ensure!(source_id == "fts", "only `fts` is supported in Layer 1");
    let fixture = load_fixture(cli.fixture_path.as_deref())?;
    let report = run_fts_live_loop(&mut runtime, fixture, &cli).await?;
```

Change to dispatch by source_id:
```rust
if let Some(source_id) = &cli.ingest {
    let report = match source_id.as_str() {
        "fts" => {
            let fixture = load_fixture(cli.fixture_path.as_deref())?;
            run_fts_live_loop(&mut runtime, fixture, &cli).await?
        }
        "contracts-finder" => {
            run_contracts_finder_live_loop(&mut runtime, &cli).await?
        }
        other => bail!("unsupported source: {other}"),
    };
```

### Part 2: Implement `run_contracts_finder_live_loop()`

Create a new function that follows the same pattern as `run_fts_live_loop()` but uses real agent output instead of fixtures.

**Flow:**

```
run_contracts_finder_live_loop()
  │
  ├─ 1. Load skill from skills/procurement-ingestion/contracts-finder/
  ├─ 2. Read source_cursors for 'contracts-finder' to get last cursor
  ├─ 3. Build job parameters: { publishedFrom: cursor, limit: 100, stages: "tender,award" }
  ├─ 4. Create workflow_jobs row (kind: 'sync_source', scope_id: 'contracts-finder')
  ├─ 5. Execute skill via execute_pty_skill_session() — launches real Claude+agent-browser
  ├─ 6. Parse result contract from executed session's result_path
  ├─ 7. Validate against result.schema.json
  ├─ 8. Call persist_ingestion_from_agent_output() (from Step 1 handoff)
  ├─ 9. If continuation_required: enqueue follow-up job with updated cursor
  ├─ 10. Build manifest from database (like finalize_success does for FTS)
  ├─ 11. Execute manifest generation skill (headless_contract mode)
  ├─ 12. Run live validation (dry-run) if policy requires
  ├─ 13. Finalize: write manifests table, pipeline_assessments, update workflow_jobs
  └─ 14. Report completion
```

**Key differences from run_fts_live_loop():**
- NO fixture loading — all data comes from agent output
- source_id is 'contracts-finder' throughout
- Job parameters include `publishedFrom` from source_cursors
- Continuation jobs pass the agent's returned `cursor` as the new `publishedFrom`

### Part 3: Continuation Support

When the agent returns `continuation_required: true`:
1. Read the `cursor` field from the agent's result
2. Enqueue a new workflow_jobs row with:
   - `kind`: 'sync_source'
   - `scope_id`: 'contracts-finder'
   - `continuation_chain_id`: same as parent job
   - `input_payload`: `{ "publishedFrom": result.cursor, "limit": 100 }`
3. The cron scheduler or job runner picks up the continuation job next cycle

### Part 4: Cron Scheduling

The contracts-finder source is **already registered** in `registry.rs` (line 591-611) with:
- Production binding: `procurement-ingestion/contracts-finder` at cron `0 45 4 * * *` (4:45 AM daily)
- No canary binding yet

**Add a canary binding** for source health monitoring:

```rust
canary_binding: Some(stable_binding(
    "source-health/contracts-finder",  // You may need to create this skill
    DEFAULT_SOURCE_SKILL_VERSION,
    Some("0 */30 * * * *"),           // Every 30 minutes
    "source-health",
    "contracts-finder",
)),
```

**Verify cron dispatch**: In `--serve` mode, the daemon's `enqueue_startup_cron_jobs()` should pick up the contracts-finder definition and schedule jobs. Verify by checking the handshake response's `operationalStatus.cron.definitions[]`.

### Part 5: Source Seeding

**File: `crates/daemon/src/db/sqlite.rs`**

The `seed_sources()` function currently only seeds 'fts'. Add 'contracts-finder':

```rust
// After the FTS seed block, add:
conn.execute(
    "INSERT INTO sources (id, kind, enabled, config) \
     VALUES ('contracts-finder', 'procurement', 1, '{}') \
     ON CONFLICT(id) DO UPDATE SET kind = excluded.kind, enabled = excluded.enabled, config = excluded.config",
    [],
)?;
conn.execute(
    "INSERT OR IGNORE INTO source_runtime_state (source_id, last_health_state) \
     VALUES ('contracts-finder', 'unknown')",
    [],
)?;
```

---

## Monitoring Validation

### Forensic Capture

After a successful live loop run, verify:

```bash
ARTIFACTS="$HOME/Library/Application Support/com.procintel.desktop/artifacts"

# Session directories
ls "$ARTIFACTS/sessions/2026/04/"

# Each session should have:
# - stream/ (compressed PTY chunks + stream.index.jsonl)
# - run-manifest.md (agent narrative + daemon appendix)
# - checkpoints/ (if any)
# - result contracts
```

### Run Manifest

```bash
# Find the most recent run manifest
find "$ARTIFACTS" -name "run-manifest.md" -newer "$ARTIFACTS" | head -5

# Check it has required sections
grep -E "^## (Summary|Steps Taken|Decisions Made|Warnings|Evidence)" \
  "$ARTIFACTS/sessions/2026/04/*/run-manifest.md"
```

### Database State

```bash
DB="$HOME/Library/Application Support/com.procintel.desktop/state/state.db"

# Workflow jobs
sqlite3 "$DB" "SELECT id, kind, status, started_at, completed_at FROM workflow_jobs ORDER BY created_at DESC LIMIT 10;"

# Session logs
sqlite3 "$DB" "SELECT id, skill_path, execution_mode, status, cleanup_verdict FROM agent_session_logs ORDER BY created_at DESC LIMIT 5;"

# Error manifests (should be empty on success)
sqlite3 "$DB" "SELECT id, error_type, error_message FROM error_manifests WHERE resolved = 0;"

# Source cursors (should advance after each run)
sqlite3 "$DB" "SELECT * FROM source_cursors WHERE source_id = 'contracts-finder';"

# Pipeline data
sqlite3 "$DB" "SELECT COUNT(*) as processes FROM procurement_processes;"
sqlite3 "$DB" "SELECT COUNT(*) as notices FROM contract_notices WHERE source_id = 'contracts-finder';"
sqlite3 "$DB" "SELECT COUNT(*) as lots FROM notice_lots;"

# Stage tracking
sqlite3 "$DB" "SELECT stage, state, COUNT(*) FROM process_stage_status GROUP BY stage, state;"
```

### Cron Verification

```bash
# In serve mode, connect and check cron status
python3 -c "
import socket, json, os
sock = os.path.expanduser('~/Library/Application Support/com.procintel.desktop/runtime/daemon.sock')
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(5)
s.connect(sock)
hello = {'messageType':'hello','desktopVersion':'qa','supportedProtocolRange':{'min':1,'max':1},'expectedSchemaEpoch':2,'lastSeenInstanceId':None}
s.sendall((json.dumps(hello)+'\n').encode())
data = b''
while b'\n' not in data: data += s.recv(4096)
w = json.loads(data.decode().strip())
for d in w['operationalStatus']['cron']['definitions']:
    print(f\"{d['skillId']} | cron={d['cronExpression']} | enabled={d['enabled']} | next={d.get('nextRunAt','?')}\")
s.close()
"
```

---

## Desktop App Validation

### What to Check (6 pages)

1. **Queue page** (`/`): Real contract titles, buyer names, value ranges, pipeline states. Keyset pagination works. Filters by source, pipeline state, decision.

2. **Detail page** (`/process/{id}`): Full contract details — buyer, value, deadline, procurement stage. Lots listed with values. Manifest rendered with real content.

3. **Sessions page** (`/sessions`): The agent-browser PTY session appears. Shows skill_path=procurement-ingestion/contracts-finder, execution_mode=pty, status, cleanup_verdict.

4. **Session viewer** (`/sessions/{id}`): Live or historical PTY output visible. Should show agent-browser commands (open, snapshot, click, get text).

5. **Errors page** (`/errors`): Empty on successful run. If failures occurred, error manifests with context visible.

6. **Daemon page** (`/daemon`): Shows daemon version, instance ID, cron definitions including contracts-finder, source health, resource arbiter state.

### Desktop App Startup

```bash
# Terminal 1: Daemon in serve mode
cargo run --release --bin daemon -- --serve \
  --state-root "$HOME/Library/Application Support/com.procintel.desktop/state" \
  --runtime-dir "$HOME/Library/Application Support/com.procintel.desktop/runtime" \
  --artifact-root "$HOME/Library/Application Support/com.procintel.desktop/artifacts"

# Terminal 2: Vite + Tauri
cd apps/desktop && bun run dev &
sleep 3 && cargo tauri dev
```

---

## Deliverables Checklist

- [ ] `--ingest contracts-finder` runs the full live loop end-to-end
- [ ] Real contract data persisted to procurement_processes, contract_notices, notice_lots
- [ ] source_cursors advances after each successful run
- [ ] Continuation jobs enqueued when result has continuation_required=true
- [ ] Manifest generation skill executes and writes to manifests table
- [ ] pipeline_assessments populated with real coverage metrics
- [ ] process_stage_status and process_stage_heads correctly tracked
- [ ] Cron definition visible in daemon handshake (operationalStatus.cron)
- [ ] PTY forensic capture: compressed bytes at artifacts/sessions/
- [ ] Run manifest written with agent narrative + daemon appendix
- [ ] Error manifests generated on any failures
- [ ] Cleanup verdict assigned (clean/forced/leaked) for every session
- [ ] Desktop app Queue shows real Contracts Finder contracts
- [ ] Desktop app Detail shows lots, buyer, value, manifest
- [ ] Desktop app Sessions shows the PTY session with agent-browser output
- [ ] `cargo check --all-targets` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes

---

## Constraints & Warnings

- The `run_fts_live_loop()` function is ~600 lines and deeply integrated. Don't modify it — create `run_contracts_finder_live_loop()` alongside it.
- The `finalize_success()` function can be reused for both FTS and CF — it reads from database state, not from fixture data.
- All SQLite writes MUST use `state_db.with_write_transaction()` for atomicity.
- The contracts-finder cron runs at 4:45 AM by default. For testing, you can trigger manually via IPC: send `trigger_ingestion` command with `sourceId: "contracts-finder"` to the daemon socket.
- The desktop app's Tauri binary has `EXPECTED_SCHEMA_EPOCH = 2` (already fixed). If you change the schema, bump this.
- Rate limiting: Contracts Finder returns 403 after excessive requests. The skill must handle this. The daemon's circuit breaker (source_runtime_state) should record failures.
- Run `cargo check` and `cargo clippy` after EVERY Rust change.
