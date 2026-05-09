# Handoff: Build & Test Contracts Finder Ingestion Skill

> ULTRATHINK and be meticulous. This is a complex integration task touching skill definition, daemon Rust code, and browser automation. Every detail matters.

## Mission

Build a **real, working Contracts Finder procurement ingestion skill** that:
1. Uses the `agent-browser` CLI for browser-based data extraction from https://www.contractsfinder.service.gov.uk/Search/Results
2. Returns structured contract data matching an expanded result schema
3. Is executed by the daemon via `--skill-path` in PTY mode
4. Has its output **parsed by the daemon and written to real SQLite pipeline tables** (replacing the current fixture-materialization pattern)
5. Produces data visible in the Tauri desktop app

This is the first real agent skill in the procurement intelligence pipeline. The infrastructure (daemon, PTY, IPC, desktop app) is already built and working. The gap is: the daemon currently writes fixture data instead of real agent output. You are closing that gap for the ingestion stage.

---

## Prerequisites — Do These First

### 1. Install agent-browser

```bash
npm i -g agent-browser
agent-browser install    # Downloads Chrome for Testing
agent-browser --version  # Verify installation
```

**agent-browser** is a Vercel CLI tool for AI-driven browser automation. Key commands:
- `agent-browser open <url>` — Navigate to URL
- `agent-browser snapshot -i` — Get interactive elements with refs (@e1, @e2...)
- `agent-browser snapshot -c` — Compact full page content
- `agent-browser click @ref` — Click an element
- `agent-browser get text @ref` — Extract text from element
- `agent-browser get text` — Full page text
- `agent-browser screenshot <file> --annotate` — Visual debug with numbered labels
- `agent-browser scroll down/up` — Scroll page
- `agent-browser wait` — Wait for network idle
- `agent-browser close` — Close session

It runs a background daemon process that persists browser state between commands.

### 2. Verify daemon compiles

```bash
cd /Users/zacfarrell/Desktop/Testproject1
cargo check --all-targets
```

---

## Codebase Orientation

### Project Structure

```
/Users/zacfarrell/Desktop/Testproject1/
  crates/daemon/src/main.rs          # 224KB — daemon core, live loop, pipeline materialization
  crates/daemon/src/agents/claude.rs # Claude CLI adapter (spawns real claude process)
  crates/daemon/src/agents/contracts.rs # Result contract validation
  crates/daemon/src/prompt/renderer.rs  # Skill prompt injection
  crates/daemon/src/sources/registry.rs # Source registration (contracts-finder ALREADY registered)
  crates/daemon/src/db/sqlite.rs     # SQLite management, migrations, commit fence
  skills/procurement-ingestion/contracts-finder/  # Skill directory (skeleton exists)
  migrations/0001_sqlite_state_initial.sql        # Core schema (32 tables)
  apps/desktop/                       # Tauri desktop app
```

### Key Conventions (from AGENTS.md)

- **Rust edition 2024 (nightly)**, Clippy pedantic+nursery denied, unsafe forbidden
- **After every substantive change**: `cargo check --all-targets && cargo clippy --all-targets -- -D warnings`
- **No file deletion** without explicit permission
- **No backwards-compat shims** — fix directly (early-stage project, no users)
- **Bun** for all TypeScript/Node.js (never npm/yarn)
- **Package manager discipline**: Cargo for Rust, Bun for TS

### Daemon Startup Command (for testing)

```bash
cargo run --release --bin daemon -- --skill-path skills/procurement-ingestion/contracts-finder \
  --state-root "$HOME/Library/Application Support/com.procintel.desktop/state" \
  --runtime-dir "$HOME/Library/Application Support/com.procintel.desktop/runtime" \
  --artifact-root "$HOME/Library/Application Support/com.procintel.desktop/artifacts"
```

---

## The Target: Contracts Finder Website

**URL**: https://www.contractsfinder.service.gov.uk/Search/Results

### Data Available

**Search results page** (20 results/page, paginated):
- Contract title (linked to detail page at `/notice/{UUID}`)
- Buyer/organisation name
- Description (truncated)
- Procurement stage: Early engagement, Future opportunity, Opportunity, Awarded contract
- Notice status: Open / Closed
- Closing date/time
- Location (region/postcode)
- Value range (low-high)
- Publication date

**OCDS API** (recommended primary data source — richer, structured, no bot protection):
- `GET /Published/Notices/OCDS/Search?publishedFrom=YYYY-MM-DD&size=100&page=1`
- Returns JSON with `hitsCount`, `maxPage`, `releases[]`
- Each release contains: `ocid`, `tag`, `tender` (title, description, status, classification/CPV, value, procurementMethod, tenderPeriod, contractPeriod, suitability, documents[]), `parties[]` (buyer/supplier with identifier, address, contactPoint), `awards[]` (value, suppliers, contractPeriod, documents[]), `buyer`
- Paginated: 100 results/page, filterable by `publishedFrom`, `publishedTo`, `stages`, `orderBy`, `order`
- **No authentication required** — public under OGL v3.0
- Rate limit: HTTP 403 with 5-minute cooldown

**Recommended approach**: Use the OCDS API as the primary data source (structured JSON, no bot issues). Use agent-browser for the HTML site as verification/enrichment or as a fallback. The skill should demonstrate the agent-browser workflow pattern even if the API is the primary extraction channel.

### Detail Page Fields (from OCDS release)

Per release:
```
ocid, id, language, date, tag[], initiationType
tender: id, title, description, status, classification (CPV scheme+id+description),
        additionalClassifications[], items[] (deliveryAddresses), minValue, value,
        procurementMethod, procurementMethodDetails, mainProcurementCategory,
        tenderPeriod.endDate, contractPeriod (start/end), suitability (sme/vcse),
        datePublished, documents[]
parties[]: id, name, roles[], identifier (legalName, scheme, id),
           address (street, locality, postalCode, region, country),
           contactPoint (name, email, telephone), details (scale, vcse, url)
buyer: id, name
awards[]: id, status, date, datePublished, value, suppliers[], contractPeriod, documents[]
```

---

## What Must Be Built

### Part 1: Update the Skill Definition

**File: `skills/procurement-ingestion/contracts-finder/SKILL.md`**

Rewrite the SKILL.md with a detailed playbook that instructs the agent to:

1. **Receive job parameters** — `publishedFrom` (ISO date), `limit` (max notices to fetch), `stages` (comma-separated)
2. **Use agent-browser** to navigate to the Contracts Finder OCDS API endpoint and extract structured JSON data. The skill MUST demonstrate using agent-browser commands (open, snapshot, get text) even if the primary extraction path is API-based.
3. **For each notice**: Extract all fields needed for the pipeline tables (see schema below)
4. **Emit checkpoints** at batch boundaries (every 10-20 notices)
5. **Return a result contract** matching the expanded `result.schema.json`
6. **Handle pagination** — if more results exist than the limit, set `continuation_required: true`
7. **Handle rate limits** — back off on HTTP 403, emit warning in run manifest
8. **Write a run manifest** with the required YAML frontmatter and narrative sections

The SKILL.md YAML frontmatter must include:
```yaml
skill_id: procurement-ingestion/contracts-finder
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 2.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
maxPages: 10
maxScrollCycles: 2
```

**File: `skills/procurement-ingestion/contracts-finder/result.schema.json`**

Expand to include ALL fields needed for pipeline table writes:

```json
{
  "type": "object",
  "required": ["cursor", "continuation_required", "notices"],
  "properties": {
    "cursor": { "type": "string" },
    "continuation_required": { "type": "boolean" },
    "notices": {
      "type": "array",
      "items": {
        "type": "object",
        "required": [
          "notice_id", "source_notice_id", "source_url", "title",
          "published_at", "source_updated_at", "ocid", "procurement_stage"
        ],
        "properties": {
          "notice_id": { "type": "string" },
          "source_notice_id": { "type": "string" },
          "source_url": { "type": "string" },
          "ocid": { "type": "string" },
          "title": { "type": "string" },
          "summary": { "type": "string" },
          "buyer_name": { "type": "string" },
          "procurement_stage": { "type": "string" },
          "main_category": { "type": "string" },
          "deadline_at": { "type": "string" },
          "published_at": { "type": "string" },
          "source_updated_at": { "type": "string" },
          "estimated_value_low_pence": { "type": "integer" },
          "estimated_value_high_pence": { "type": "integer" },
          "estimated_value_currency": { "type": "string" },
          "access_route": { "type": "string" },
          "procurement_method": { "type": "string" },
          "raw_release_json": { "type": "string" },
          "lots": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "lot_id": { "type": "string" },
                "source_lot_id": { "type": "string" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "estimated_value_pence": { "type": "integer" },
                "deadline_at": { "type": "string" }
              }
            }
          },
          "buyer": {
            "type": "object",
            "properties": {
              "name": { "type": "string" },
              "identifier_scheme": { "type": "string" },
              "identifier_id": { "type": "string" },
              "contact_name": { "type": "string" },
              "contact_email": { "type": "string" },
              "contact_telephone": { "type": "string" },
              "address_locality": { "type": "string" },
              "address_region": { "type": "string" },
              "address_postal_code": { "type": "string" }
            }
          },
          "documents": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "id": { "type": "string" },
                "document_type": { "type": "string" },
                "url": { "type": "string" },
                "description": { "type": "string" }
              }
            }
          },
          "cpv_codes": {
            "type": "array",
            "items": { "type": "string" }
          }
        }
      }
    }
  }
}
```

### Part 2: Daemon Modifications (Rust)

**CRITICAL**: The daemon currently writes fixture data. You must add a code path that parses the agent's result contract and writes it to real SQLite tables.

#### 2a. Make source_id dynamic in persist functions

**File: `crates/daemon/src/main.rs`**

In `persist_pipeline_materialization()` (around line 3964), the contract_notices INSERT hardcodes `'fts'`. This must be parameterized. Search for all occurrences of hardcoded `'fts'` in SQL statements within this function and make them dynamic.

#### 2b. Add a real-data persistence function

Create a new function `persist_ingestion_from_agent_output()` that:

1. Takes the parsed result contract JSON (the agent's output matching `result.schema.json`)
2. For each notice in `result.notices[]`:
   - INSERT into `raw_documents` (payload_json = notice.raw_release_json)
   - INSERT into `procurement_processes` (id, canonical_ocid, title, buyer_name, etc.)
   - INSERT into `contract_notices` (all fields mapped from the notice object)
   - INSERT into `notice_lots` for each lot
   - INSERT process_stage_status and process_stage_heads for the ingest/normalize stages
3. UPDATE `source_cursors` with the new cursor value
4. All within a single `state_db.with_write_transaction()`

**Key column mappings** (notice JSON field → SQLite column):

```
notice.notice_id          → contract_notices.id
notice.source_notice_id   → contract_notices.source_notice_id
"contracts-finder"        → contract_notices.source_id
notice.ocid               → contract_notices.ocid, procurement_processes.canonical_ocid
notice.title              → contract_notices.title, procurement_processes.title
notice.summary            → contract_notices.summary
notice.buyer_name         → contract_notices.buyer_name, procurement_processes.buyer_name
notice.procurement_stage  → contract_notices.procurement_stage
notice.main_category      → contract_notices.main_category
notice.deadline_at        → contract_notices.deadline_at, procurement_processes.latest_deadline_at
notice.published_at       → contract_notices.published_at, procurement_processes.latest_published_at
notice.source_updated_at  → contract_notices.source_updated_at, procurement_processes.latest_updated_at
notice.estimated_value_low_pence → contract_notices.estimated_value_original_minor
notice.estimated_value_currency  → contract_notices.estimated_value_currency
notice.access_route       → contract_notices.access_route
notice.raw_release_json   → raw_documents.payload_json, contract_notices.normalized_payload
```

For `notice_lots`:
```
lot.lot_id                → notice_lots.id
lot.source_lot_id         → notice_lots.source_lot_id
lot.title                 → notice_lots.title
lot.description           → notice_lots.description
lot.estimated_value_pence → notice_lots.estimated_value_pence
lot.deadline_at           → notice_lots.deadline_at
```

Every row must include sync-tracking columns:
- `owner_id` = "local-daemon"
- `row_version` = 1
- `last_local_seq` = the commit_seq from the transaction
- `stage_run_id` = freshly generated UUID per transaction

#### 2c. Wire the --skill-path output to the new persistence function

In `run_bound_skill()` (line 1803), after `execute_skill_session()` returns, add logic to:
1. Check if the skill_id starts with "procurement-ingestion/"
2. If so, read `result_path` → parse JSON → call `persist_ingestion_from_agent_output()`
3. This replaces the current behavior where the skill output is only logged, not materialized

#### 2d. Register 'contracts-finder' source in SQLite

The source registry (`registry.rs`) already defines 'contracts-finder' in code, but the `seed_sources()` function in `db/sqlite.rs` only seeds 'fts'. Add seeding for 'contracts-finder':

```sql
INSERT INTO sources (id, kind, enabled, config)
VALUES ('contracts-finder', 'procurement', 1, '{}')
ON CONFLICT(id) DO UPDATE SET kind = excluded.kind, enabled = excluded.enabled;

INSERT OR IGNORE INTO source_runtime_state (source_id, last_health_state)
VALUES ('contracts-finder', 'unknown');
```

### Part 3: PTY Environment for agent-browser

The daemon launches Claude CLI in an **isolated environment** (only CLAUDE_* variables). The agent won't find `agent-browser` on PATH. Fix by passing PATH in the launch spec.

**File: `crates/daemon/src/agents/claude.rs`** (around line 240)

In `build_launch_spec()`, add the system PATH:
```rust
env.insert("PATH".to_owned(), std::env::var("PATH").unwrap_or_default());
```

This ensures the Claude CLI process inherits the daemon's PATH, allowing it to find `agent-browser`, `node`, and other tools.

---

## Database Schema Reference

### contract_notices (exact DDL)
```sql
CREATE TABLE IF NOT EXISTS contract_notices (
  id TEXT PRIMARY KEY,
  process_id TEXT REFERENCES procurement_processes(id),
  source_id TEXT NOT NULL REFERENCES sources(id),
  source_notice_id TEXT NOT NULL,
  raw_document_id TEXT NOT NULL,
  ocid TEXT,
  title TEXT NOT NULL,
  summary TEXT,
  buyer_name TEXT,
  procurement_stage TEXT NOT NULL,
  access_route TEXT,
  deadline_at TEXT,
  published_at TEXT NOT NULL,
  source_updated_at TEXT NOT NULL,
  main_category TEXT,
  estimated_value_original_minor INTEGER,
  estimated_value_currency TEXT,
  estimated_value_gbp_ex_vat_pence INTEGER,
  estimated_value_vat_inclusive INTEGER,
  normalized_payload TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (source_id, source_notice_id)
);
```

### procurement_processes (exact DDL)
```sql
CREATE TABLE IF NOT EXISTS procurement_processes (
  id TEXT PRIMARY KEY,
  canonical_ocid TEXT,
  primary_notice_id TEXT,
  canonical_process_id TEXT,
  current_stage TEXT NOT NULL,
  buyer_name TEXT,
  title TEXT NOT NULL,
  latest_deadline_at TEXT,
  latest_published_at TEXT NOT NULL,
  latest_updated_at TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### notice_lots (exact DDL)
```sql
CREATE TABLE IF NOT EXISTS notice_lots (
  id TEXT PRIMARY KEY,
  notice_id TEXT NOT NULL REFERENCES contract_notices(id),
  source_lot_id TEXT,
  lot_key TEXT,
  lot_number INTEGER,
  title TEXT,
  description TEXT,
  estimated_value_pence INTEGER,
  deadline_at TEXT,
  is_bidable_separately INTEGER NOT NULL DEFAULT 1,
  documents TEXT,
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### raw_documents (exact DDL)
```sql
CREATE TABLE IF NOT EXISTS raw_documents (
  id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL REFERENCES sources(id),
  fetched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  content_type TEXT,
  source_url TEXT,
  payload_json TEXT,
  storage_path TEXT,
  metadata TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (payload_json IS NOT NULL OR storage_path IS NOT NULL)
);
```

---

## Testing & Validation Protocol

### Step 1: Verify skill loads

```bash
cargo run --release --bin daemon -- --dry-run \
  --state-root "$HOME/Library/Application Support/com.procintel.desktop/state" \
  --runtime-dir "$HOME/Library/Application Support/com.procintel.desktop/runtime" \
  --artifact-root "$HOME/Library/Application Support/com.procintel.desktop/artifacts"
```

### Step 2: Run the skill

```bash
# Kill any running daemon first
pkill -f "target/release/daemon"

cargo run --release --bin daemon -- \
  --skill-path skills/procurement-ingestion/contracts-finder \
  --state-root "$HOME/Library/Application Support/com.procintel.desktop/state" \
  --runtime-dir "$HOME/Library/Application Support/com.procintel.desktop/runtime" \
  --artifact-root "$HOME/Library/Application Support/com.procintel.desktop/artifacts"
```

### Step 3: Verify data in SQLite

```bash
DB="$HOME/Library/Application Support/com.procintel.desktop/state/state.db"
sqlite3 "$DB" "SELECT COUNT(*) FROM procurement_processes;"
sqlite3 "$DB" "SELECT id, title, buyer_name FROM procurement_processes LIMIT 5;"
sqlite3 "$DB" "SELECT id, source_id, title, procurement_stage FROM contract_notices LIMIT 5;"
sqlite3 "$DB" "SELECT id, title, estimated_value_pence FROM notice_lots LIMIT 5;"
sqlite3 "$DB" "SELECT source_id, cursor_key, cursor_value FROM source_cursors;"
sqlite3 "$DB" "SELECT id, kind, status FROM workflow_jobs ORDER BY created_at DESC LIMIT 5;"
```

### Step 4: Verify in desktop app

```bash
# Start daemon in serve mode
cargo run --release --bin daemon -- --serve \
  --state-root "$HOME/Library/Application Support/com.procintel.desktop/state" \
  --runtime-dir "$HOME/Library/Application Support/com.procintel.desktop/runtime" \
  --artifact-root "$HOME/Library/Application Support/com.procintel.desktop/artifacts"

# In another terminal, start the desktop app
cd apps/desktop && bun run dev &
sleep 3 && cargo tauri dev
```

Check: Queue page shows real contracts. Detail page shows real buyer, value, lots.

### Step 5: Iterate

If data is missing or malformed:
1. Check the agent's result.json in the artifacts directory
2. Adjust the SKILL.md playbook
3. Fix the daemon's field mapping
4. Re-run the skill

---

## Deliverables Checklist

- [ ] `agent-browser` installed and working
- [ ] `skills/procurement-ingestion/contracts-finder/SKILL.md` — rewritten with agent-browser playbook
- [ ] `skills/procurement-ingestion/contracts-finder/result.schema.json` — expanded with all pipeline fields
- [ ] `crates/daemon/src/main.rs` — `persist_ingestion_from_agent_output()` function added
- [ ] `crates/daemon/src/main.rs` — `run_bound_skill()` wired to call persistence for ingestion skills
- [ ] `crates/daemon/src/agents/claude.rs` — PATH passed to PTY environment
- [ ] `crates/daemon/src/db/sqlite.rs` — 'contracts-finder' source seeded
- [ ] `cargo check --all-targets` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] Skill runs end-to-end: agent scrapes, daemon persists, data in SQLite
- [ ] Desktop app shows real Contracts Finder data

---

## Constraints & Warnings

- The daemon's `main.rs` is ~224KB. Be surgical — don't refactor unrelated code.
- The `persist_pipeline_materialization()` function is complex (~350 lines). Add a NEW function alongside it rather than modifying it, to avoid breaking the existing FTS fixture path.
- All SQLite writes must use `state_db.with_write_transaction()` for atomicity and commit_seq advancement.
- The `UNIQUE(source_id, source_notice_id)` constraint on `contract_notices` means you must handle duplicates (use `ON CONFLICT DO UPDATE` or check before insert).
- Contracts Finder rate limits at HTTP 403. The skill must handle this gracefully with backoff.
- Run `cargo check` and `cargo clippy` after EVERY Rust change before testing.
