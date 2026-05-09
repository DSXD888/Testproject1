# Handoff: Build, Test & Integrate NHS Supply Chain (Jaggaer) Procurement Ingestion Source

> ULTRATHINK and be meticulous. This task involves portal exploration, skill authoring, agentic validation loops, Rust daemon integration, and database persistence. Every detail matters.

## Mission

Build a **complete, tested, working NHS Supply Chain procurement ingestion source** that:
1. Uses `agent-browser` to log in to the NHS Supply Chain Jaggaer e-Sourcing portal and extract live procurement opportunity data
2. Returns structured contract data matching the ingestion result contract schema
3. Downloads and tracks PDF tender documents for downstream PDF extraction pipeline processing
4. Is validated via an agentic test loop with error manifest feedback before daemon integration
5. Integrates into the daemon's live loop, source registry, and automated scheduling

This is the **second authenticated procurement source** in the platform. Unlike Contracts Finder (public OCDS API) and FTS (public web), this source requires browser-based login and HTML scraping from a Jaggaer-hosted e-sourcing portal. The credential-in-skill pattern replaces the auth-refresh skill cycle used by supplier sources (alibaba/1688).

---

## Source Details

- **Platform:** NHS Supply Chain via Jaggaer e-Sourcing
- **Portal URL:** `https://nhssupplychain.app.jaggaer.com/esop/ogc-host/private/nhs_supply_chain/second_s.jst?_ncp=1776085033026.79411-1`
- **Login:** `zacfarrell@icloud.com` / `Z4c1sG4y@2026`
- **Source ID:** `nhs-supply-chain`
- **Source Kind:** Procurement
- **Auth approach:** Credentials embedded directly in SKILL.md playbook. No separate auth-refresh skill.

---

## Prerequisites

### 1. Install agent-browser

```bash
npm i -g agent-browser
agent-browser install    # Downloads Chrome for Testing
agent-browser --version  # Verify installation
```

**agent-browser** key commands:
- `agent-browser open <url>` -- Navigate to URL
- `agent-browser snapshot -i` -- Interactive elements with refs (@e1, @e2...)
- `agent-browser snapshot -c` -- Compact full page content
- `agent-browser click @ref` -- Click an element
- `agent-browser type @ref "text"` -- Type text into element
- `agent-browser get text @ref` -- Extract text from element
- `agent-browser get text` -- Full page text
- `agent-browser screenshot <file> --annotate` -- Visual debug with numbered labels
- `agent-browser scroll down/up` -- Scroll page
- `agent-browser wait` -- Wait for network idle
- `agent-browser close` -- Close session

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
  crates/daemon/src/main.rs              # Daemon core, live loop functions, persist_ingestion_from_agent_output
  crates/daemon/src/sources/registry.rs  # Source definitions (builtin_definitions())
  crates/daemon/src/agents/claude.rs     # Claude CLI adapter
  crates/daemon/src/agents/contracts.rs  # Result contract validation
  crates/daemon/src/prompt/renderer.rs   # Skill prompt injection
  crates/daemon/src/pdf/engine.rs        # Structural PDF extraction (Docling)
  crates/daemon/src/pdf/policy.rs        # Document role classification & overlay routing
  crates/daemon/src/pdf/chunking.rs      # Token-based PDF chunking
  crates/daemon/src/errors/manifest.rs   # Error manifest system
  crates/daemon/src/jobs/runner.rs       # Job lifecycle management
  crates/daemon/src/db/sqlite.rs         # SQLite management, migrations
  skills/procurement-ingestion/contracts-finder/  # Reference skill package
  skills/procurement-ingestion/fts/               # Reference skill package
  skills/pdf-extraction/*/                        # PDF overlay skills
  migrations/                                     # Database schema
  prompts/                                        # Handoff prompts
  state/state.db                                  # SQLite database
```

### Key Conventions

- **Rust edition 2024** (nightly), Clippy pedantic+nursery denied, unsafe forbidden
- **After every substantive Rust change:** `cargo check --all-targets && cargo clippy --all-targets -- -D warnings`
- **Bun** for all TypeScript/Node.js (never npm/yarn)
- **No backwards-compat shims** -- fix directly (early-stage project)
- **Read AGENTS.md first** -- it is the single source of truth for all agent instructions in this project

### Daemon Commands

```bash
# Test a skill standalone (Phase 1 — before live loop integration)
cargo run --release --bin daemon -- --skill-path skills/procurement-ingestion/nhs-supply-chain \
  --state-root state --runtime-dir runtime --artifact-root artifacts

# Run live loop ingestion (Phase 2 — after skill is validated)
cargo run --release --bin daemon -- --ingest nhs-supply-chain \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

**Note on state paths:** During development/testing, use the local project directories (`state/`, `runtime/`, `artifacts/`). The Tauri desktop app uses `$HOME/Library/Application Support/com.procintel.desktop/` — only use those paths when validating desktop app integration.

---

## Critical Reference: How Ingestion Persistence Works

The daemon has a **source-agnostic** persistence function `persist_ingestion_from_agent_output` (main.rs ~line 5401) that ALL ingestion skills must be compatible with. Your result contract MUST produce data this function can consume.

### How the function actually reads the result JSON (embedded from source)

```rust
// Top-level reads:
let notices = result_json.get("notices").and_then(Value::as_array);     // REQUIRED — errors if missing
let cursor = result_json.get("cursor").and_then(Value::as_str);         // falls back to ""

// Per-notice reads (for each object in notices[]):
let notice_id = notice.get("notice_id").and_then(Value::as_str);        // REQUIRED — errors if missing
let source_notice_id = notice.get("source_notice_id")...;              // falls back to notice_id
let ocid = notice.get("ocid")...;                                      // falls back to ""
let title = notice.get("title").and_then(Value::as_str);                // REQUIRED — errors if missing
let summary = notice.get("summary")...;                                 // optional
let buyer_name = notice.get("buyer_name")...;                           // optional
let procurement_stage = notice.get("procurement_stage")...;             // falls back to "unknown"
let main_category = notice.get("main_category")...;                     // optional
let deadline_at = notice.get("deadline_at")...;                         // optional; CLEARED if stage == "awarded"
let published_at = notice.get("published_at").and_then(Value::as_str);  // REQUIRED — errors if missing
let source_updated_at = notice.get("source_updated_at")...;             // REQUIRED — errors if missing
let estimated_value_low_pence = notice.get("estimated_value_low_pence").and_then(Value::as_i64); // optional
let estimated_value_currency = notice.get("estimated_value_currency")...; // optional
let access_route = notice.get("access_route")...;                       // falls back to "public_web"
let source_url = notice.get("source_url")...;                           // falls back to ""
let raw_release_json = notice.get("raw_release_json")...;               // falls back to "{}"

// ID derivation (hardcoded in the function):
let raw_doc_id = format!("raw-{notice_id}");    // e.g., "raw-nhs-abc123"
let process_id = format!("proc-{ocid}");         // e.g., "proc-nhs-abc123"

// Per-lot reads (for each object in notice.lots[]):
let lot_id = lot.get("lot_id")...;               // falls back to source_lot_id or "unknown"
let source_lot_id = lot.get("source_lot_id")...;
let lot_title = lot.get("title")...;
let lot_description = lot.get("description")...;
let lot_value_pence = lot.get("estimated_value_pence").and_then(Value::as_i64);
let lot_deadline = lot.get("deadline_at")...;
```

### Critical implication for your `ocid` field

The function derives `process_id` as `format!("proc-{ocid}")`. Since Jaggaer has no standard OCID, set `ocid` to the same value as `notice_id` (e.g., `"nhs-abc123"`). This produces `process_id = "proc-nhs-abc123"` — unique per source.

**If you leave `ocid` empty**, the process_id becomes `"proc-"` which will collide across all notices. You MUST populate `ocid`.

### Top-level result contract shape

```json
{
  "cursor": "<string: serialized cursor for continuation>",
  "continuation_required": true|false,
  "notices": [ ... array of notice objects ... ]
}
```

### Database tables written to (in one transaction)

1. **`raw_documents`** -- raw scraped JSON per notice
2. **`procurement_processes`** -- canonical process record (UPSERT on id, keeps latest dates)
3. **`contract_notices`** -- notice details (UPSERT on source_id + source_notice_id unique constraint)
4. **`notice_lots`** -- lot breakdowns (UPSERT on id)
5. **`source_cursors`** -- updated cursor watermark

### ID derivation pattern

```
notice_id:  "nhs-{portal_notice_id}"
process_id: "proc-{ocid}" (where ocid can be notice_id if no OCID available)
raw_doc_id: "raw-{notice_id}"
lot_id:     "nhs-lot-{portal_lot_id}"
```

---

## Critical Reference: PDF Document Pipeline

These portal-based sources often attach PDF tender packs. The platform has a **complete PDF extraction pipeline** ready to process them:

### How PDFs flow

1. **Ingestion skill** downloads PDF, stores in `raw_documents` with `storage_path` pointing to local file
2. **Structural extraction** (Docling via `pdf/engine.rs`) parses PDF into markdown/tables/sections, stores in `document_extractions`
3. **Policy routing** (`pdf/policy.rs`) classifies the document role and picks semantic overlay skills
4. **Semantic extraction** (Claude skills) extracts domain-specific data, stores in `document_semantic_extractions`

### Document roles that trigger overlay skills

| Document Role | Overlay Skill |
|---------------|---------------|
| TechnicalSpecification, InvitationToTender, StatementOfRequirements | `pdf-extraction/tender-specification` |
| PricingSchedule | `pdf-extraction/pricing-schedule` |
| Boq | `pdf-extraction/bill-of-quantities` |
| MixedTenderPack, CombinedPack | tender-specification + conditional pricing/boq |
| Administrative, CoverLetter, TermsAndConditions, Unknown | No overlay (skip) |

### What the ingestion skill must do for PDFs

When the portal has downloadable tender documents:
1. Note the document URLs/download links in the notice's `documents` array within the result contract
2. If time/budget permits, download the PDF to `artifacts/downloads/{source_id}/{notice_id}/` and include the local path
3. The daemon-side pipeline will handle structural + semantic extraction automatically

For Phase 1, focus on correctly identifying and listing available documents. Actual download integration happens in Phase 2.

---

## Critical Reference: Existing Skill Package Structure

Every procurement ingestion skill contains exactly 5 files:

### SKILL.md (reference: contracts-finder)

```yaml
---
skill_id: procurement-ingestion/contracts-finder
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 2.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
maxPages: 10
maxScrollCycles: 2
---
```

Key frontmatter fields your skill MUST include:
- `skill_id`: `procurement-ingestion/nhs-supply-chain`
- `interaction_profile`: `pty_interactive` (browser automation)
- `model_profile`: `browse_stable_low_cost`
- `agent_cli`: `claude`
- `side_effect_class`: `none`

### result.schema.json

Must define the same top-level shape: `cursor`, `continuation_required`, `notices[]`. The per-notice fields can vary based on what this portal provides, but MUST include at minimum: `notice_id`, `title`, `published_at`, `source_updated_at`.

### checkpoint.schema.json (reference)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["seq", "cursor", "notices_processed"],
  "properties": {
    "seq": { "type": "integer" },
    "cursor": { "type": "string" },
    "notices_processed": { "type": "integer" }
  }
}
```

### canary.inputs.jsonl

Single-line JSON with minimal parameters for a health-check run (e.g., limit=5).

### golden.assertions.yaml

```yaml
expected_fields:
  - cursor
  - continuation_required
  - notices
minimum_notice_count: 1
expected_interaction_profile: pty_interactive
```

---

## Critical Reference: Source Registry Pattern

In `crates/daemon/src/sources/registry.rs`, the `builtin_definitions()` function at line 562 registers all known sources. Here's the contracts-finder entry as a template:

```rust
definitions.insert(
    "contracts-finder".to_owned(),
    SourceDefinition {
        source_id: "contracts-finder".to_owned(),
        kind: SourceKind::Procurement,
        enabled: true,
        source_structure_version: "contracts-finder-2026-03-30".to_owned(),
        config: json!({}),
        production_binding: Some(stable_binding(
            "procurement-ingestion/contracts-finder",
            DEFAULT_SOURCE_SKILL_VERSION,
            Some("0 45 4 * * *"),
            "procurement-ingestion",
            "contracts-finder",
        )),
        canary_binding: None,
        auth_refresh_binding: None,
        policy: SourcePolicy::default(),
        default_health_state: HealthState::Green,
    },
);
```

---

## Critical Reference: Live Loop Function Pattern

In `crates/daemon/src/main.rs`, `run_contracts_finder_live_loop` (line 4321) shows the exact pattern to follow:

```rust
async fn run_contracts_finder_live_loop(
    runtime: &mut RuntimeBootstrap,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    // 1. Load skill from filesystem
    let repo_root = repo_root()?;
    let ingest_skill = load_skill(
        &repo_root.join("skills/procurement-ingestion/contracts-finder"),
        "skills/procurement-ingestion/contracts-finder/SKILL.md",
    )?;

    // 2. Read existing cursor from source_cursors table
    let existing_cursor: Option<String> = runtime.state_db.connection()
        .query_row(
            "SELECT cursor_value FROM source_cursors \
             WHERE source_id = 'contracts-finder' AND cursor_key = 'notice_cursor'",
            [], |row| row.get(0),
        ).optional()?;

    // 3. Build job parameters
    let job_parameters = json!({
        "source_id": "contracts-finder",
        "publishedFrom": published_from,
        "limit": 100,
        "stages": "tender,award",
    });

    // 4. Create workflow_jobs row
    // 5. Claim job
    // 6. Execute PTY skill session via execute_skill_session()
    // 7. Parse result.json from agent output
    // 8. Call persist_ingestion_from_agent_output("contracts-finder", &result_json)
    // 9. Handle continuation (enqueue follow-up job if continuation_required=true)
    // 10. Mark job completed
}
```

The `--ingest` match block at line 986:
```rust
if let Some(source_id) = &cli.ingest {
    let report = match source_id.as_str() {
        "fts" => { ... }
        "contracts-finder" => run_contracts_finder_live_loop(&mut runtime, &cli).await?,
        other => bail!("unsupported source: {other}"),
    };
    ...
}
```

---

## Critical Reference: Error Manifest System

When skill validation fails, create an error manifest using the system in `crates/daemon/src/errors/manifest.rs`. Key ErrorCode variants relevant to this source:

- `AgentRunFailed` -- skill execution crashed
- `AgentOutputContractInvalid` -- result.json doesn't match schema
- `AgentAuthChallenge` -- login failed or CAPTCHA
- `SourceSoftBlocked` -- rate limited or temporarily blocked
- `PageShapeDrift` -- portal UI changed unexpectedly
- `AgentStalled` -- no progress within timeout

---

## Execution Plan

### PHASE 1: Skill Build + Agentic Validation Loop

#### Step 1: Deep Codebase Study (MANDATORY before any code)

Do NOT write any code or SKILL.md until you have completed this entire research phase.

##### 1A. Understand How Skills Work (the daemon→skill→daemon flow)

The execution model works like this:
```
Daemon (Rust)                          Skill (Claude CLI in PTY)
─────────────                          ──────────────────────────
1. load_skill() reads SKILL.md
   and result.schema.json
2. Creates workflow_jobs row
3. Spawns Claude CLI in PTY via
   execute_skill_session()
4. Passes job_parameters as JSON  ──►  5. Agent reads SKILL.md playbook
   + skill_markdown as prompt          6. Agent uses agent-browser to
                                          navigate portal, extract data
                                       7. Agent writes result.json to
                                          working directory
8. Daemon reads result.json       ◄──  
9. Validates against schema
10. Calls persist_ingestion_from_agent_output()
11. Writes to SQLite tables
```

**The skill has NO database access.** It receives context via job_parameters and returns results via result.json. The daemon handles all persistence.

Read these files to understand this flow:
- `crates/daemon/src/main.rs` lines 4321-4530 -- `run_contracts_finder_live_loop` (the complete execution pattern from skill load to persist)
- `crates/daemon/src/main.rs` lines 2772-2806 -- `execute_skill_session` (how Claude CLI is spawned)

You must be able to answer:
1. What does `load_skill()` return and what files does it read from the skill directory?
2. How does the daemon pass job parameters to the skill?
3. Where does the skill write its result, and how does the daemon find it?

##### 1B. Understand the Result Contract Schema (what your skill MUST produce)

Read `skills/procurement-ingestion/contracts-finder/result.schema.json` -- this is the reference schema your skill must replicate. Here is the full schema:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["cursor", "continuation_required", "notices"],
  "properties": {
    "cursor": { "type": "string" },
    "continuation_required": { "type": "boolean" },
    "notices": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["notice_id", "source_notice_id", "source_url", "title",
                      "published_at", "source_updated_at", "ocid", "procurement_stage"],
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
          "deadline_at": { "type": ["string", "null"] },
          "published_at": { "type": "string" },
          "source_updated_at": { "type": "string" },
          "estimated_value_low_pence": { "type": ["integer", "null"] },
          "estimated_value_currency": { "type": ["string", "null"] },
          "access_route": { "type": "string" },
          "raw_release_json": { "type": "string" },
          "lots": { "type": "array", "items": { "type": "object", "properties": {
            "lot_id": { "type": "string" },
            "source_lot_id": { "type": "string" },
            "title": { "type": ["string", "null"] },
            "description": { "type": ["string", "null"] },
            "estimated_value_pence": { "type": ["integer", "null"] },
            "deadline_at": { "type": ["string", "null"] }
          }}},
          "buyer": { "type": "object" },
          "documents": { "type": "array" },
          "cpv_codes": { "type": "array", "items": { "type": "string" } }
        }
      }
    }
  }
}
```

You must be able to answer:
1. Which top-level fields are required?
2. Which per-notice fields are required vs optional?
3. How would you adapt this for a portal that uses different field names?

##### 1C. Study the Reference Skill Playbook

Read `skills/procurement-ingestion/contracts-finder/SKILL.md` -- the complete reference playbook. Pay special attention to the **Playbook** section (lines 56-217) which shows exactly how a skill instructs the agent to use `agent-browser`. Key patterns to replicate:

- Step-by-step numbered instructions with exact `agent-browser` commands
- Field mapping table showing portal data → result contract fields
- Cursor serialization for continuation support
- Writing result.json to the current working directory
- Failure code documentation
- Run manifest instructions

Also read `skills/procurement-ingestion/contracts-finder/canary.inputs.jsonl` and `golden.assertions.yaml` for the testing package format.

You must be able to answer:
1. What `agent-browser` commands does the contracts-finder skill use to extract data?
2. How does the skill handle pagination?
3. What format is the cursor serialized as?

##### 1D. Understand the Database Schema (what gets persisted)

Read these migration files to understand the tables your data flows into:
- `migrations/0001_sqlite_state_initial.sql` -- focus on: `contract_notices` (unique constraint on source_id + source_notice_id), `procurement_processes` (canonical_ocid), `notice_lots`
- `migrations/0003_sqlite_runtime.sql` -- focus on: `raw_documents` (storage_path for PDFs), `source_cursors` (cursor watermark)
- `migrations/0006_pdf_extraction_toolkit.sql` -- focus on: `document_extractions`, `tender_document_sets` (for PDF processing)

You must be able to answer:
1. What is the unique constraint on `contract_notices`? (This determines upsert behavior)
2. How does `raw_documents` store PDFs? (the `storage_path` vs `payload_json` pattern)
3. What triggers downstream PDF processing after a `raw_documents` entry is created?

##### 1E. Understand Source Registration

Read `crates/daemon/src/sources/registry.rs` -- focus on `builtin_definitions()` (line 562) and the `stable_binding()` helper (line 688). Understand how sources are registered with skill bindings, cron expressions, and circuit breaker policies.

##### 1F. Reference Reading (skim for awareness)

- `crates/daemon/src/pdf/policy.rs` -- how document roles map to extraction overlay skills
- `crates/daemon/src/errors/manifest.rs` -- ErrorCode variants for error manifest creation
- `prompts/contracts-finder-skill-build-handoff.md` -- how the first source was built (detailed reference)

#### Step 2: Portal Exploration (MANDATORY before writing SKILL.md)

**WARNING: No screenshots of the Jaggaer portal are available.** You are entering this portal blind. The exploration step is therefore the MOST CRITICAL step. Spend significant time here. Do not rush to write the SKILL.md until you thoroughly understand the portal.

Navigate the Jaggaer portal interactively using `agent-browser` to map the portal structure:

```bash
agent-browser open "https://nhssupplychain.app.jaggaer.com/esop/ogc-host/private/nhs_supply_chain/second_s.jst"
agent-browser wait
agent-browser snapshot -i
```

**If the URL redirects or shows a login page**, that's expected. Jaggaer portals typically redirect unauthenticated users. Find the login form and authenticate.

**Discovery protocol (follow in order):**

1. **Find and complete login:**
   - [ ] Screenshot the login page (`agent-browser screenshot login-page.png --annotate`)
   - [ ] Identify form fields (email/username, password, submit button)
   - [ ] Login with: `zacfarrell@icloud.com` / `Z4c1sG4y@2026`
   - [ ] Screenshot the post-login landing page
   - [ ] If 2FA or additional verification is required, document it and STOP — report this as a blocker

2. **Find the opportunities/tenders listing:**
   - [ ] Jaggaer portals typically have a "Sourcing" or "Opportunities" or "Tenders" section in the navigation
   - [ ] Screenshot the navigation menu/sidebar
   - [ ] Navigate to the listing page
   - [ ] Screenshot the listing page layout

3. **Map the listing page structure:**
   - [ ] How many results per page?
   - [ ] What columns/data appear in the listing table?
   - [ ] What pagination mechanism exists (next page link, infinite scroll, page numbers)?
   - [ ] What filters/search/sort options are available?
   - [ ] Can you filter by status, date range, category?

4. **Explore detail pages (visit at least 3):**
   - [ ] Click into an individual opportunity and screenshot the detail page
   - [ ] What fields are available? (title, buyer, dates, values, categories, status, description)
   - [ ] Where are tender document PDFs listed/downloadable?
   - [ ] What is the notice URL pattern? (extract the ID format from the URL)
   - [ ] Are there lots/line items on the detail page?
   - [ ] Is there a unique identifier for each opportunity? (this becomes your `source_notice_id`)

5. **Map data to result contract:**
   - [ ] For each field the portal shows, decide which result contract field it maps to
   - [ ] Check for OCID or any standard procurement identifiers
   - [ ] Check what procurement stages/statuses the portal uses and how they map to: "tender", "awarded", "closed", "future_opportunity"
   - [ ] If values are shown as "GBP 50,000.00", note the format for parsing to pence

6. **Note operational concerns:**
   - [ ] Any anti-bot, CAPTCHA, or rate-limit behaviors?
   - [ ] Session timeout behavior (how long before logout)?
   - [ ] Does the portal use heavy JavaScript / AJAX? (affects `agent-browser wait` timing)
   - [ ] Is data extraction best done from listing page only, or must you visit each detail page?

**Save all findings** as structured notes in a file (e.g., `skills/procurement-ingestion/nhs-supply-chain/EXPLORATION-NOTES.md`) before writing the SKILL.md. These notes are your evidence for the field mapping and playbook design.

#### Step 3: Build the Skill Package

Create `skills/procurement-ingestion/nhs-supply-chain/` with all 5 files:

**SKILL.md must include:**
- YAML frontmatter matching the template above (skill_id: `procurement-ingestion/nhs-supply-chain`)
- Login credentials embedded in the playbook
- Step-by-step playbook instructions covering:
  1. Open portal URL
  2. Login with embedded credentials
  3. Navigate to opportunity listing
  4. Paginate through results (respect maxPages bound)
  5. For each opportunity: extract all available fields
  6. Map portal fields to result contract schema
  7. Build notices array with cursor and continuation state
  8. Write result.json to current working directory
- Field mapping table (portal field -> result contract field)
- Failure codes and resume strategy
- Run manifest instructions (use contracts-finder as template)

**result.schema.json** must define the same top-level shape (`cursor`, `continuation_required`, `notices[]`). Per-notice required fields must include at minimum: `notice_id`, `title`, `published_at`, `source_updated_at`. Include optional fields for whatever this portal provides.

**checkpoint.schema.json** -- adapt from contracts-finder template.

**canary.inputs.jsonl** -- a minimal health-check input (e.g., `{"limit":3}`).

**golden.assertions.yaml** -- expected fields, minimum notice count, interaction profile.

#### Step 4: Agentic Validation Loop

After building the skill package, you MUST validate it works by testing the skill end-to-end. This is an iterative loop:

```
┌─────────────────────────────────────────────────────────┐
│                 AGENTIC VALIDATION LOOP                  │
│                                                          │
│  1. Invoke sub-agent to execute the skill playbook       │
│     - Sub-agent opens portal, logs in, extracts data     │
│     - Sub-agent writes result.json                       │
│                                                          │
│  2. Validate result.json                                 │
│     - Does it match result.schema.json?                  │
│     - Does it have the required top-level fields?        │
│     - Are notice_id values prefixed with "nhs-"?         │
│     - Are dates in ISO 8601 format?                      │
│     - Are monetary values in pence (integers)?           │
│     - Can persist_ingestion_from_agent_output consume it?│
│                                                          │
│  3. If validation FAILS:                                 │
│     - Use error-manifest skill to create error manifest  │
│     - Diagnose the root cause from the manifest          │
│     - Fix the SKILL.md playbook or result.schema.json    │
│     - Go back to step 1                                  │
│                                                          │
│  4. If validation PASSES:                                │
│     - Proceed to Phase 2                                 │
│                                                          │
│  Loop until: login works, extraction works, result       │
│  contract is valid, PDF documents are identified         │
└─────────────────────────────────────────────────────────┘
```

**How to practically test the skill:**

**Option A: Sub-agent execution (recommended for first validation):**

Use the Agent tool to spawn a sub-agent. Give it this prompt:

```
You are a procurement data extraction agent. Read the skill playbook at
skills/procurement-ingestion/nhs-supply-chain/SKILL.md and execute it step-by-step
using agent-browser. Follow every instruction in the Playbook section exactly.
Write the final result contract as result.json in the current working directory.
Report what you extracted: how many notices, any errors encountered, any fields
you could not populate.
```

The sub-agent will follow the SKILL.md playbook, interact with the portal via `agent-browser`, and write `result.json`.

**Option B: Daemon `--skill-path` mode (tests daemon integration):**

```bash
cargo run --release --bin daemon -- --skill-path skills/procurement-ingestion/nhs-supply-chain \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

This spawns a real Claude CLI session in PTY mode, exactly as the daemon would. The result is captured in the artifacts directory.

**After the sub-agent or daemon produces `result.json`, validate it yourself:**

1. Parse the JSON: `cat result.json | python3 -m json.tool`
2. Check top-level fields: `cursor` (string), `continuation_required` (boolean), `notices` (array)
3. Verify at least 1 notice exists
4. For each notice verify:
   - `notice_id` starts with `"nhs-"` and is non-empty
   - `title` is non-empty
   - `published_at` and `source_updated_at` are valid ISO 8601 strings
   - `ocid` is populated (NOT empty — see the critical implication above)
5. If `estimated_value_low_pence` is present, verify it's an integer (not a float or string like "50000.00")
6. Simulate persistence: mentally trace through the `persist_ingestion_from_agent_output` field reads above — every REQUIRED field must be present and non-null
7. If `lots[]` is present, each lot must have `lot_id` starting with `"nhs-lot-"`

**On failure — create an error manifest:**

Use the `/error-manifest` skill to create a structured error manifest documenting:
- The specific validation failure (e.g., "notice_id missing nhs- prefix")
- The relevant part of result.json that failed
- The expected vs actual values
- Root cause analysis (e.g., "SKILL.md Step 4 doesn't instruct prefixing")
- Concrete fix (e.g., "Add instruction: prefix all notice_id values with 'nhs-'")

Then fix the SKILL.md and re-test. Repeat until validation passes cleanly.

---

### PHASE 2: Daemon Integration + Automated Runtime Test

**Only begin Phase 2 after Phase 1 validation passes completely.**

#### Step 1: Source Registry Entry

Add to `builtin_definitions()` in `crates/daemon/src/sources/registry.rs`:

```rust
definitions.insert(
    "nhs-supply-chain".to_owned(),
    SourceDefinition {
        source_id: "nhs-supply-chain".to_owned(),
        kind: SourceKind::Procurement,
        enabled: true,
        source_structure_version: "nhs-supply-chain-2026-04-13".to_owned(),
        config: json!({}),
        production_binding: Some(stable_binding(
            "procurement-ingestion/nhs-supply-chain",
            DEFAULT_SOURCE_SKILL_VERSION,
            Some("0 30 5 * * *"),   // 5:30 AM daily (staggered from CF at 4:45)
            "procurement-ingestion",
            "nhs-supply-chain",
        )),
        canary_binding: None,
        auth_refresh_binding: None,
        policy: SourcePolicy::default(),
        default_health_state: HealthState::Green,
    },
);
```

#### Step 2: Sources Table Insert

Ensure the source row exists in SQLite. Add to the daemon's source-seeding logic:

```sql
INSERT OR IGNORE INTO sources (id, kind, enabled, config)
VALUES ('nhs-supply-chain', 'procurement', 1, '{}');
```

#### Step 3: Live Loop Function

Add `run_nhs_supply_chain_live_loop()` in `crates/daemon/src/main.rs` following the exact pattern of `run_contracts_finder_live_loop`. Key differences:

- Skill path: `skills/procurement-ingestion/nhs-supply-chain`
- Source ID: `nhs-supply-chain`
- Cursor key: `notice_cursor`
- Job parameters: adapted to whatever inputs the NHS skill needs

Add the match arm in the `--ingest` handler:
```rust
"nhs-supply-chain" => run_nhs_supply_chain_live_loop(&mut runtime, &cli).await?,
```

#### Step 4: Compile and Test

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo run --release --bin daemon -- --ingest nhs-supply-chain \
  --state-root "$HOME/Library/Application Support/com.procintel.desktop/state" \
  --runtime-dir "$HOME/Library/Application Support/com.procintel.desktop/runtime" \
  --artifact-root "$HOME/Library/Application Support/com.procintel.desktop/artifacts"
```

#### Step 5: E2E Validation

After running ingestion, verify the data landed correctly:

```bash
# Use the local state database (same --state-root you passed to the daemon)
sqlite3 state/state.db "SELECT id, title, source_id, procurement_stage FROM contract_notices WHERE source_id = 'nhs-supply-chain' LIMIT 10;"

sqlite3 state/state.db "SELECT id, title, buyer_name FROM procurement_processes WHERE id LIKE 'proc-nhs-%' LIMIT 10;"

sqlite3 state/state.db "SELECT source_id, cursor_key, cursor_value FROM source_cursors WHERE source_id = 'nhs-supply-chain';"

# Verify no orphaned records (process_id in notices must exist in processes)
sqlite3 state/state.db "SELECT cn.id FROM contract_notices cn LEFT JOIN procurement_processes pp ON cn.process_id = pp.id WHERE cn.source_id = 'nhs-supply-chain' AND pp.id IS NULL;"
```

If the result set is empty for the orphan check, the foreign key relationships are correct. If data appears in all three tables and the cursor is persisted, the integration is working.

---

## Deliverables Checklist

### Phase 1 Deliverables
- [ ] Portal exploration notes (what fields are available, pagination mechanism, document locations)
- [ ] `skills/procurement-ingestion/nhs-supply-chain/SKILL.md` -- complete playbook
- [ ] `skills/procurement-ingestion/nhs-supply-chain/result.schema.json` -- result contract schema
- [ ] `skills/procurement-ingestion/nhs-supply-chain/checkpoint.schema.json` -- checkpoint schema
- [ ] `skills/procurement-ingestion/nhs-supply-chain/canary.inputs.jsonl` -- canary input
- [ ] `skills/procurement-ingestion/nhs-supply-chain/golden.assertions.yaml` -- assertions
- [ ] At least 1 successful validation loop run producing a valid result.json
- [ ] Any error manifests created during the validation loop (for audit trail)

### Phase 2 Deliverables
- [ ] Source registry entry in `registry.rs`
- [ ] Source row insert in database seeding
- [ ] `run_nhs_supply_chain_live_loop()` function in `main.rs`
- [ ] `--ingest nhs-supply-chain` match arm
- [ ] Successful `cargo check --all-targets && cargo clippy --all-targets -- -D warnings`
- [ ] Successful E2E run: `--ingest nhs-supply-chain` writes data to SQLite
- [ ] Verified data in contract_notices, procurement_processes, notice_lots tables
- [ ] Cursor state persisted in source_cursors

---

## Constraints & Guardrails

1. **Result contract compatibility is non-negotiable.** The result MUST have `cursor`, `continuation_required`, `notices[]` at the top level. `persist_ingestion_from_agent_output` is source-agnostic and reads these exact fields.
2. **Notice ID prefix.** All notice_id values must start with `nhs-` to avoid collisions with other sources.
3. **Process ID derivation.** Use `proc-{ocid}` pattern. If no OCID exists, use `proc-nhs-{portal_id}`.
4. **Monetary values in pence.** All `estimated_value_low_pence`, `estimated_value_pence` values must be integers representing British pence. Convert from pounds if needed (multiply by 100).
5. **ISO 8601 dates.** All date fields must be ISO 8601 strings (e.g., `2026-04-13T10:30:00Z`).
6. **No auth-refresh skill.** Login credentials are embedded directly in the SKILL.md playbook.
7. **PTY interactive mode.** This skill requires browser automation, so `interaction_profile: pty_interactive`.
8. **PDF awareness.** Identify and list available tender documents in the result contract. The existing PDF pipeline handles extraction downstream.
9. **Continuation semantics.** If there are more results than can be fetched in one session, set `continuation_required: true` and provide a cursor for the next run.
10. **Access route.** Set `access_route` to `"portal_authenticated"` (not `"public_web"`) since this source requires login.
