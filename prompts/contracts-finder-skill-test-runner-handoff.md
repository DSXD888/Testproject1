# Handoff: Test & Iterate the Contracts Finder Ingestion Skill

> ULTRATHINK and be meticulous. Your job is to RUN the skill, diagnose failures, fix them, and loop until data flows end-to-end.

## Mission

The Contracts Finder ingestion skill has been built (by a prior session using `contracts-finder-skill-build-handoff.md`). Your job is to:

1. **Verify prerequisites** are in place
2. **Run the skill** via the daemon's `--skill-path` mode
3. **Inspect the output** — agent result JSON, SQLite tables, artifacts
4. **Diagnose and fix** any failures (skill playbook issues, schema mismatches, daemon persistence bugs)
5. **Loop** until real Contracts Finder data appears correctly in the database
6. **Validate visually** by starting the desktop app and confirming data shows up

You are the QA engineer + fixer. Expect multiple iterations.

---

## Order of Operations

```
Step 1: Verify prerequisites (daemon compiles, agent-browser works, DB accessible)
     ↓
Step 2: Kill any running daemon
     ↓  
Step 3: Run the skill via --skill-path
     ↓
Step 4: Check exit code and daemon logs
     ↓
Step 5: Inspect SQLite tables for real data
     ↓
Step 6: If failures → diagnose → fix → go to Step 2
     ↓
Step 7: Once data flows, start daemon in --serve mode + desktop app
     ↓
Step 8: Visually confirm data in desktop Queue and Detail views
```

---

## Step 1: Verify Prerequisites

Run ALL of these before attempting the skill:

```bash
cd /Users/zacfarrell/Desktop/Testproject1

# 1a. Daemon compiles cleanly
cargo check --all-targets
cargo clippy --all-targets -- -D warnings

# 1b. agent-browser is installed and works
which agent-browser
agent-browser --version
# If not installed: npm i -g agent-browser && agent-browser install

# 1c. Claude CLI is available
which claude

# 1d. Database exists and is accessible
DB="$HOME/Library/Application Support/com.procintel.desktop/state/state.db"
sqlite3 "$DB" "SELECT COUNT(*) FROM sqlite_master WHERE type='table';"
# Should return 32+

# 1e. 'contracts-finder' source exists in DB
sqlite3 "$DB" "SELECT id, kind, enabled FROM sources WHERE id='contracts-finder';"
# If empty, the build handoff didn't seed it. Fix:
# sqlite3 "$DB" "INSERT OR IGNORE INTO sources (id, kind, enabled, config) VALUES ('contracts-finder', 'procurement', 1, '{}');"
# sqlite3 "$DB" "INSERT OR IGNORE INTO source_runtime_state (source_id, last_health_state) VALUES ('contracts-finder', 'unknown');"

# 1f. Skill files exist
ls -la skills/procurement-ingestion/contracts-finder/SKILL.md
ls -la skills/procurement-ingestion/contracts-finder/result.schema.json
ls -la skills/procurement-ingestion/contracts-finder/checkpoint.schema.json

# 1g. No daemon is running (would block on lock file)
pkill -f "target/release/daemon" 2>/dev/null; sleep 1
```

**If any prerequisite fails**, fix it before proceeding. The build handoff (`contracts-finder-skill-build-handoff.md`) documents what should have been created.

---

## Step 2: Run the Skill

```bash
cd /Users/zacfarrell/Desktop/Testproject1

# Build release binary (in case code changed)
cargo build --release --bin daemon

# Run the skill
RUST_LOG=debug cargo run --release --bin daemon -- \
  --skill-path skills/procurement-ingestion/contracts-finder \
  --state-root "$HOME/Library/Application Support/com.procintel.desktop/state" \
  --runtime-dir "$HOME/Library/Application Support/com.procintel.desktop/runtime" \
  --artifact-root "$HOME/Library/Application Support/com.procintel.desktop/artifacts"
```

**What to watch for in output:**
- `daemon bootstrap ready` — daemon started
- `skill loaded` or similar — SKILL.md parsed successfully
- `adapter probe` — Claude CLI capability check
- `session spawned` / PTY creation messages
- Agent output lines (agent-browser commands, extracted data)
- `result contract validated` — schema check passed
- `persist` / `committed` — data written to SQLite
- `completed` — success

**If it exits immediately with an error**, read the error message carefully. Common issues:
- Lock file held → another daemon is running → `pkill -f daemon`
- Skill directory invalid → check SKILL.md YAML frontmatter
- Schema epoch mismatch → check `EXPECTED_SCHEMA_EPOCH` in Tauri main.rs (should be 2)
- Missing source in DB → seed 'contracts-finder' in sources table

---

## Step 3: Inspect Results

### 3a. Check exit code

```bash
echo $?
# 0 = success, non-zero = failure
```

### 3b. Check SQLite for real data

```bash
DB="$HOME/Library/Application Support/com.procintel.desktop/state/state.db"

echo "=== PROCESS COUNT ==="
sqlite3 "$DB" "SELECT COUNT(*) FROM procurement_processes;"

echo "=== RECENT PROCESSES ==="
sqlite3 "$DB" -header -column "SELECT id, title, buyer_name, current_stage FROM procurement_processes ORDER BY created_at DESC LIMIT 10;"

echo "=== NOTICES ==="
sqlite3 "$DB" -header -column "SELECT id, source_id, title, procurement_stage, published_at FROM contract_notices WHERE source_id='contracts-finder' ORDER BY created_at DESC LIMIT 10;"

echo "=== LOTS ==="
sqlite3 "$DB" -header -column "SELECT nl.id, nl.title, nl.estimated_value_pence, cn.title as notice_title FROM notice_lots nl JOIN contract_notices cn ON nl.notice_id = cn.id LIMIT 10;"

echo "=== RAW DOCUMENTS ==="
sqlite3 "$DB" "SELECT id, source_id, content_type, length(payload_json) as payload_size FROM raw_documents WHERE source_id='contracts-finder' LIMIT 5;"

echo "=== SOURCE CURSORS ==="
sqlite3 "$DB" "SELECT * FROM source_cursors WHERE source_id='contracts-finder';"

echo "=== WORKFLOW JOBS ==="
sqlite3 "$DB" -header -column "SELECT id, kind, status, started_at, completed_at FROM workflow_jobs ORDER BY created_at DESC LIMIT 5;"

echo "=== SESSION LOGS ==="
sqlite3 "$DB" -header -column "SELECT id, skill_path, execution_mode, status, cleanup_verdict, exit_code FROM agent_session_logs ORDER BY created_at DESC LIMIT 5;"

echo "=== STAGE TRACKING ==="
sqlite3 "$DB" "SELECT stage, scope_kind, state, COUNT(*) FROM process_stage_status GROUP BY stage, scope_kind, state;"

echo "=== ERRORS ==="
sqlite3 "$DB" -header -column "SELECT id, error_type, error_message FROM error_manifests WHERE resolved=0 LIMIT 5;"

echo "=== COMMIT SEQ ==="
sqlite3 "$DB" "SELECT commit_seq FROM commit_clock WHERE id=1;"
```

### 3c. Check artifacts

```bash
ARTIFACTS="$HOME/Library/Application Support/com.procintel.desktop/artifacts"

# Find recent session directories
find "$ARTIFACTS/sessions" -type d -name "20*" 2>/dev/null | sort | tail -5

# Check for run manifest
find "$ARTIFACTS" -name "run-manifest.md" 2>/dev/null | sort | tail -3

# Check for result contract
find "$ARTIFACTS" -name "result*" 2>/dev/null | sort | tail -3

# If a result.json exists, inspect it
RESULT=$(find "$ARTIFACTS" -name "result*.json" 2>/dev/null | sort | tail -1)
if [ -n "$RESULT" ]; then
  echo "=== RESULT CONTRACT ==="
  python3 -m json.tool "$RESULT" | head -50
fi
```

---

## Step 4: Diagnose Failures

### Failure: Daemon exits with no agent output

**Cause**: Claude CLI not launching, or prompt rendering failed.
**Fix**: Check `RUST_LOG=debug` output for adapter probe results. Verify `which claude` works. Check SKILL.md YAML frontmatter is valid.

### Failure: Agent runs but returns empty/invalid result

**Cause**: SKILL.md playbook unclear, agent-browser commands failing, or website changed.
**Fix**: Read the agent's PTY output (in daemon logs or artifacts). Check what agent-browser returned. Edit SKILL.md to fix the playbook.

Test agent-browser manually:
```bash
agent-browser open "https://www.contractsfinder.service.gov.uk/Search/Results"
agent-browser snapshot -c | head -100
agent-browser close
```

### Failure: Result validates but nothing in SQLite

**Cause**: The daemon's persistence function isn't being called, or the agent's result JSON doesn't map correctly to the expected fields.
**Fix**: Check if `persist_ingestion_from_agent_output()` (or equivalent) exists in main.rs and is called from `run_bound_skill()`. Verify field names match between result.schema.json and the Rust code.

### Failure: Schema validation error

**Cause**: Agent's result JSON doesn't match result.schema.json.
**Fix**: Compare the agent's actual output (from artifacts) against the schema. Check required fields, types. Edit either the schema or the SKILL.md instructions.

### Failure: SQLite constraint violation

**Cause**: Duplicate notice_id, missing foreign key, etc.
**Fix**: Check the UNIQUE constraint on `contract_notices(source_id, source_notice_id)`. Use `INSERT ... ON CONFLICT DO UPDATE` in the daemon code. Ensure process_id is generated before notice_id references it.

### Failure: agent-browser not found by Claude

**Cause**: PTY environment doesn't include PATH.
**Fix**: In `crates/daemon/src/agents/claude.rs`, ensure PATH is passed:
```rust
env.insert("PATH".to_owned(), std::env::var("PATH").unwrap_or_default());
```
Rebuild: `cargo build --release --bin daemon`

---

## Step 5: Fix and Re-run

After each fix:

```bash
# Rebuild
cargo build --release --bin daemon

# Verify it compiles
cargo check --all-targets
cargo clippy --all-targets -- -D warnings

# Kill stale daemon
pkill -f "target/release/daemon" 2>/dev/null; sleep 1

# Re-run
RUST_LOG=debug cargo run --release --bin daemon -- \
  --skill-path skills/procurement-ingestion/contracts-finder \
  --state-root "$HOME/Library/Application Support/com.procintel.desktop/state" \
  --runtime-dir "$HOME/Library/Application Support/com.procintel.desktop/runtime" \
  --artifact-root "$HOME/Library/Application Support/com.procintel.desktop/artifacts"
```

**Repeat Step 3 to check results.** Keep looping until:
- `procurement_processes` has rows with real titles and buyer names
- `contract_notices` has rows with `source_id='contracts-finder'` and real OCID values
- `notice_lots` has rows linked to the correct notices
- `workflow_jobs` shows a completed job
- `agent_session_logs` shows status=archived with cleanup_verdict=clean

---

## Step 6: Optionally Clear Bad Data Between Runs

If a failed run left garbage data, clean up before retrying:

```bash
DB="$HOME/Library/Application Support/com.procintel.desktop/state/state.db"

# Delete only contracts-finder data (preserves any FTS fixture data)
sqlite3 "$DB" "
  DELETE FROM notice_lots WHERE notice_id IN (SELECT id FROM contract_notices WHERE source_id='contracts-finder');
  DELETE FROM contract_notices WHERE source_id='contracts-finder';
  DELETE FROM procurement_processes WHERE id IN (SELECT process_id FROM contract_notices WHERE source_id='contracts-finder');
  DELETE FROM raw_documents WHERE source_id='contracts-finder';
  DELETE FROM source_cursors WHERE source_id='contracts-finder';
  DELETE FROM workflow_jobs WHERE kind='sync_source' AND scope_id='contracts-finder';
"
```

---

## Step 7: Validate in Desktop App

Once data is in SQLite, verify the desktop app displays it:

```bash
# Terminal 1: Start daemon in serve mode
pkill -f "target/release/daemon" 2>/dev/null; sleep 1
cargo run --release --bin daemon -- --serve \
  --state-root "$HOME/Library/Application Support/com.procintel.desktop/state" \
  --runtime-dir "$HOME/Library/Application Support/com.procintel.desktop/runtime" \
  --artifact-root "$HOME/Library/Application Support/com.procintel.desktop/artifacts"

# Terminal 2: Start desktop app
cd /Users/zacfarrell/Desktop/Testproject1/apps/desktop
bun run dev &
sleep 3
cargo tauri dev
```

**Check in the app:**
1. **Queue page** — real contract titles and buyer names visible
2. **Click a contract** — detail page shows lots, value, procurement stage
3. **Sessions page** — shows the PTY session from the skill run
4. **Errors page** — should be empty (if run succeeded)
5. **Daemon page** — shows daemon connected, schema epoch 2

---

## Success Criteria

The skill test is COMPLETE when ALL of these are true:

- [ ] `cargo check --all-targets` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `--skill-path` runs to completion (exit code 0)
- [ ] `procurement_processes` contains real contract data (not fixture)
- [ ] `contract_notices` has rows with `source_id='contracts-finder'` and real OCIDs
- [ ] `notice_lots` has rows with real lot titles
- [ ] `raw_documents` has rows with non-empty `payload_json`
- [ ] `source_cursors` has a cursor for 'contracts-finder'
- [ ] `workflow_jobs` shows a completed job for the skill run
- [ ] `agent_session_logs` shows cleanup_verdict='clean'
- [ ] No unresolved rows in `error_manifests`
- [ ] Desktop app Queue page shows the ingested contracts

**Once all criteria pass**, the skill is ready for Step 2 (live loop integration via `contracts-finder-live-loop-handoff.md`).

---

## Files You May Need to Edit

| File | Why |
|------|-----|
| `skills/procurement-ingestion/contracts-finder/SKILL.md` | Fix agent playbook (most common iteration target) |
| `skills/procurement-ingestion/contracts-finder/result.schema.json` | Adjust required/optional fields |
| `crates/daemon/src/main.rs` | Fix persistence mapping, field names, SQL |
| `crates/daemon/src/agents/claude.rs` | Fix PTY environment (PATH, env vars) |
| `crates/daemon/src/db/sqlite.rs` | Fix source seeding |

---

## Constraints

- **Do not modify** `run_fts_live_loop()` or `persist_pipeline_materialization()` — those are the FTS fixture path and must stay working.
- **After every Rust change**: `cargo check --all-targets && cargo clippy --all-targets -- -D warnings`
- **No file deletion** without explicit permission (AGENTS.md rule).
- **Contracts Finder rate limits** at HTTP 403 with 5-minute cooldown. If the agent hits this during testing, wait 5 minutes before re-running.
- **agent-browser leaves a background daemon running**. Clean up with `agent-browser close --all` if needed.
