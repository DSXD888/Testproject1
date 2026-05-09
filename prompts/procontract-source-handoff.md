# Handoff: Build, Test & Integrate ProContract (Due North) Three-Stage Procurement Ingestion Pipeline

> ULTRATHINK and be meticulous. This is a multi-skill pipeline with three sequential agent runners, portal side effects (registering interest on real tenders), database schema implications, and document processing. Every detail matters.

## Mission

Build a **complete, tested, working three-stage ProContract procurement pipeline** consisting of:

1. **Skill 1: Opportunity Discovery** -- Scrape all available opportunities from the ProContract portal, visit each detail page, extract high-level contract data, save to database
2. **Skill 2: Interest Registration** -- Pre-filter new contracts using early criteria (goods not services, expiry > 1 month), navigate to qualifying contracts and click "Request Interest", track registration status in database
3. **Skill 3: Tender Enrichment** -- Check the "Recently Added" activity page for tenders that responded to our interest request, extract full tender details, download ALL attachments (PDFs, DOCX, ZIP), process them, update database with complete contract information

Each skill runs independently on its own cron schedule and has its own agentic validation loop. The daemon orchestrates them as sequential pipeline stages for the `procontract` source.

**CRITICAL:** Skill 2 has real portal side effects -- it clicks "Request Interest" on live tenders. This is intentional and authorized. The `side_effect_class` must reflect this.

---

## Source Details

- **Platform:** ProContract by Due North
- **Source ID:** `procontract`
- **Source Kind:** Procurement
- **Login:** `sales@sorellon.com` / `9928fJG3!5nk6nc`
- **Auth approach:** Credentials embedded directly in each skill's SKILL.md playbook. No separate auth-refresh skill.

### Portal URLs

| Page | URL | Purpose |
|------|-----|---------|
| Opportunities Listing | `https://procontract.due-north.com/Opportunities/Index?tabName=opportunities&resetFilter=True` | Step 1: Browse all available opportunities |
| Individual Opportunity | `https://procontract.due-north.com/Supplier/Advert/View?advertId={id}&fromProjectDashboard=False&fromRfxSummary=False&selectWorkgroup=False&fromAdvertEvent=False&fromActivityDashboard=False` | Step 1: Detail page per opportunity / Step 2: Request Interest button |
| Home Page (Activities) | `https://procontract.due-north.com/SupplierPostLoginHome#` | Step 3: Check "Recently Added" tab |
| Activity Dashboard | `https://procontract.due-north.com/SupplierPostLoginHome/ActivityDashboard?projectId={id}&callingPage=Home#` | Step 3: Find new messages with full tender |
| RFx Response (Full Tender) | `https://procontract.due-north.com/RfxResponse/Index?rfxId={id}` | Step 3: Extract full tender details & download documents |

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
  crates/daemon/src/main.rs              # Daemon core, live loops, persist_ingestion_from_agent_output
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
  skills/procurement-ingestion/contracts-finder/  # Reference skill (public API source)
  skills/procurement-ingestion/fts/               # Reference skill (public web source)
  skills/pdf-extraction/*/                        # PDF overlay skills (tender-spec, pricing, BoQ, base)
  migrations/                                     # Database schema
  state/state.db                                  # SQLite database
```

### Key Conventions

- **Rust edition 2024** (nightly), Clippy pedantic+nursery denied, unsafe forbidden
- **After every substantive Rust change:** `cargo check --all-targets && cargo clippy --all-targets -- -D warnings`
- **Bun** for all TypeScript/Node.js (never npm/yarn)
- **Read AGENTS.md first** -- single source of truth for all agent instructions

### Daemon Commands

```bash
# Test a skill standalone (Phase 1 — before live loop integration)
cargo run --release --bin daemon -- --skill-path skills/procurement-ingestion/procontract-discover \
  --state-root state --runtime-dir runtime --artifact-root artifacts

# Run live loop ingestion (Phase 2 — after skill is validated)
cargo run --release --bin daemon -- --ingest procontract \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

**Note on state paths:** During development/testing, use the local project directories (`state/`, `runtime/`, `artifacts/`). The Tauri desktop app uses `$HOME/Library/Application Support/com.procintel.desktop/` — only reference those paths when validating desktop integration.

### CRITICAL ARCHITECTURAL CONSTRAINT: Skills Cannot Access the Database

**Skills run as Claude CLI sessions in a PTY.** They interact with the browser via `agent-browser`. They do NOT have SQLite access. They cannot query `state.db`.

The **daemon** provides context to skills via `job_parameters` (a JSON object passed at session launch). For Steps 2 and 3, the daemon's live loop function must:
1. Query the database to determine what work needs doing
2. Pass that work list to the skill via `job_parameters`
3. The skill executes portal actions based on those parameters
4. The skill returns results in `result.json`
5. The daemon persists the results to the database

This means Steps 2 and 3 need **daemon-side query logic** in their live loop functions, not skill-side database queries.

---

## The Three-Stage Pipeline — Detailed Design

```
                     ┌───────────────────────────────────────────────────────────────────┐
                     │              PROCONTRACT THREE-STAGE PIPELINE                      │
                     │                                                                   │
 ┌──────────┐       │  STEP 1              STEP 2                   STEP 3              │
 │  CRON    │──────►│  Opportunity    ──►  Interest          ──►   Tender               │
 │ (daily)  │       │  Discovery           Registration             Enrichment          │
 │          │       │                                                                   │
 │ 5:00 AM  │       │  - Scrape listing    - Query DB for new      - Check "Recently    │
 │ 5:30 AM  │       │  - Visit detail pgs    unregistered contracts   Added" activity    │
 │ 6:00 AM  │       │  - Extract fields    - Pre-filter:            - Find starred/new   │
 │          │       │  - Save to DB          * goods not service      tenders            │
 │          │       │                        * expiry > 1 month    - Open Activity Dash  │
 │          │       │  DB writes:          - Navigate to portal    - Open NEW message    │
 │          │       │  • contract_notices   - Click "Request          (NOT interest one) │
 │          │       │  • procurement_procs    Interest"             - Extract full info   │
 │          │       │  • notice_lots       - Update DB stage        - Download ALL docs  │
 │          │       │  • raw_documents       status                   (PDF, DOCX, ZIP)   │
 │          │       │  • source_cursors                            - Process documents   │
 │          │       │                      DB writes:              - Update DB with full │
 │          │       │                      • process_stage_status     contract details    │
 │          │       │                        (interest_registered)  - Update deadline     │
 │          │       │                                                                   │
 │          │       │  side_effect: none    side_effect: external   side_effect: none     │
 │          │       │  profile: pty_inter   profile: pty_inter      profile: pty_inter   │
 └──────────┘       └───────────────────────────────────────────────────────────────────┘
```

---

## STEP 1: Opportunity Discovery Skill

### Skill ID: `procurement-ingestion/procontract-discover`

### What It Does

Navigates to the ProContract Opportunities listing page, scrapes all available opportunities, visits each individual opportunity detail page to extract the full high-level information, and saves everything to the database.

### Portal Flow (from screenshots)

**Opportunities Listing Page** (`/Opportunities/Index?tabName=opportunities&resetFilter=True`):

The listing page shows a table with columns:
- Title (clickable link to detail page)
- Expression Date(?)
- Expression Date
- Estimated value

Left sidebar has filters:
- Bureau
- Organisation
- Workgroup
- Est. Category (with checkboxes for categories)
- Status (7 status categories available for selection)
- Award status
- Published date range (Start date / End date)

**Individual Opportunity Detail Page** (`/Supplier/Advert/View?advertId={id}...`):

Each contract detail page shows:
- Full title (e.g., "SQ0417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment")
- Two tabs: "Main contract details" and "Expression of interest registered"
- Under "Main contract details":
  - Full description text
  - Category/classification
  - Estimated value
  - Published date
  - Expression deadline date
  - Buyer/organisation name
  - Workgroup
  - Bureau
  - Contact information
- A **"Request Interest"** button (used in Step 2, NOT in Step 1)

### Data Extraction Mapping

From each opportunity detail page, extract and map:

```
notice_id:              "pc-{advertId}"  (extract UUID from URL parameter)
source_notice_id:       "{advertId}"     (the raw UUID)
source_url:             Full URL to the advert page
title:                  Full contract title from the page header
summary:                Full description text from "Main contract details"
buyer_name:             Organisation name shown on the page
procurement_stage:      Map from portal status:
                          Open/Live → "tender"
                          Closed → "closed"
                          Awarded → "awarded"
                          Pre-tender/Pipeline → "future_opportunity"
main_category:          Category/classification shown on the page
deadline_at:            Expression deadline date (ISO 8601)
published_at:           Published date (ISO 8601)
source_updated_at:      Published date or last updated date (ISO 8601)
estimated_value_low_pence:  Estimated value converted to pence (integer)
estimated_value_currency:   "GBP"
access_route:           "portal_authenticated"
ocid:                   "pc-{advertId}" (no standard OCID on this platform)
raw_release_json:       JSON.stringify of all raw scraped data from the page
```

### Result Contract Shape

```json
{
  "cursor": "{\"page\": 2, \"last_published\": \"2026-04-13\"}",
  "continuation_required": false,
  "notices": [
    {
      "notice_id": "pc-83a118df-7c2e-f111-813c-005056b64545",
      "source_notice_id": "83a118df-7c2e-f111-813c-005056b64545",
      "source_url": "https://procontract.due-north.com/Supplier/Advert/View?advertId=83a118df-7c2e-f111-813c-005056b64545&...",
      "title": "SQ0417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment",
      "summary": "Design & Technical Services are looking to procure...",
      "buyer_name": "Stockport Council",
      "procurement_stage": "tender",
      "main_category": "Professional Services",
      "deadline_at": "2026-05-27T12:00:00Z",
      "published_at": "2026-04-01T00:00:00Z",
      "source_updated_at": "2026-04-01T00:00:00Z",
      "estimated_value_low_pence": null,
      "estimated_value_currency": "GBP",
      "access_route": "portal_authenticated",
      "ocid": "pc-83a118df-7c2e-f111-813c-005056b64545",
      "raw_release_json": "{...}"
    }
  ]
}
```

### Persistence

Uses the existing `persist_ingestion_from_agent_output` function — no code changes needed. The function writes to `raw_documents`, `procurement_processes`, `contract_notices`, `notice_lots`, and `source_cursors`.

### Skill Package Files

Create `skills/procurement-ingestion/procontract-discover/`:

**SKILL.md frontmatter:**
```yaml
---
skill_id: procurement-ingestion/procontract-discover
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
maxPages: 10
maxScrollCycles: 3
---
```

The SKILL.md playbook must include:
- Login credentials: `sales@sorellon.com` / `9928fJG3!5nk6nc`
- Step-by-step navigation: login -> opportunities listing -> paginate -> visit each detail page -> extract data
- The skill must OPEN EACH INDIVIDUAL CONTRACT PAGE to get the full description, category, value, etc. The listing page alone does not have enough detail.
- Cursor semantics: track page number and last published date for incremental runs
- Pagination: use the filter sidebar's published date range + page numbers
- Failure codes and resume strategy
- Run manifest instructions

**result.schema.json** -- same top-level shape as contracts-finder (`cursor`, `continuation_required`, `notices[]`)

**checkpoint.schema.json** -- seq, cursor, notices_processed

**canary.inputs.jsonl** -- `{"limit":3}`

**golden.assertions.yaml** -- expected fields, minimum_notice_count: 1

---

## STEP 2: Interest Registration Skill

### Skill ID: `procurement-ingestion/procontract-register-interest`

### What It Does

**The daemon live loop** (not the skill itself) handles the database query:
1. The daemon queries the local database for ProContract contracts where no `process_stage_status` exists for stage `'interest_registration'`
2. The daemon runs the pre-filtering logic:
   - Is this a **good** (not a service)? Check `main_category` and `summary`
   - Is the deadline **more than 1 month away**?
3. The daemon passes qualifying contract URLs and metadata to the skill via `job_parameters`
4. **The skill** (running in PTY via Claude CLI) receives the list, navigates to each contract page, and clicks **"Request Interest"**
5. After the confirmation modal appears ("Expression of interest successful!"), the skill reports success in its result contract
6. **The daemon** persists the registration status to `process_stage_status`

**Why this split:** Skills run as Claude CLI in a PTY — they have no SQLite access. The daemon provides context via `job_parameters` and handles all database reads/writes.

### CRITICAL: This Skill Has Real Side Effects

Clicking "Request Interest" on the ProContract portal is a **real external action** that notifies the buyer organization. This is intentional and authorized. The `side_effect_class` must be `external_mutation`.

### Portal Flow (from screenshots)

**Request Interest flow:**

1. Navigate to the individual opportunity page: `/Supplier/Advert/View?advertId={id}...`
2. The page shows the contract details with a **"Request Interest"** button
3. Click the "Request Interest" button
4. A confirmation modal appears with:
   - Title: "Expression of interest successful!"
   - Message: "You have successfully registered for the following opportunity for entry..."
   - "What happens next?" section explaining: "You will be contacted as a participant in the Request for Information event..."
   - Contact details of the procurement team
5. The page now shows a different tab state -- "Expression of interest registered" tab becomes active

### Pre-Filtering Logic (DAEMON-SIDE, not skill-side)

The **daemon live loop function** performs this analysis before launching the skill:

```
-- Daemon queries DB for candidates:
SELECT cn.id, cn.source_url, cn.title, cn.summary, cn.main_category,
       cn.deadline_at, cn.procurement_stage
FROM contract_notices cn
WHERE cn.source_id = 'procontract'
  AND cn.procurement_stage IN ('tender', 'future_opportunity')
  AND NOT EXISTS (
    SELECT 1 FROM process_stage_status pss
    WHERE pss.process_id = cn.process_id
      AND pss.stage = 'interest_registration'
  );

-- For each result, daemon applies pre-filtering:
FOR EACH contract:
  1. CHECK: Is deadline_at > NOW() + 30 days?
     - If no: SKIP (write process_stage_status with state='skipped', mark as expiry_too_close)

  2. CHECK: Is this likely a GOODS contract (not pure services)?
     - Inspect main_category: look for "Goods", "Supplies", "Equipment", "Products"
     - Inspect summary/title: look for physical product indicators
     - If purely services (e.g., "consultancy", "design team", "advisory"):
       SKIP (write process_stage_status with state='skipped', mark as pure_service)

  3. If BOTH checks pass: ADD TO job_parameters.contracts_to_register[]
```

The daemon passes the filtered list to the skill as `job_parameters`:
```json
{
  "source_id": "procontract-interest",
  "contracts_to_register": [
    {
      "notice_id": "pc-83a118df-...",
      "source_url": "https://procontract.due-north.com/Supplier/Advert/View?advertId=83a118df-...",
      "title": "Supply of Industrial Widgets"
    }
  ]
}
```

The **skill** then iterates over `contracts_to_register` and handles the portal interaction for each one.

### Result Contract Shape

This skill's result is different from the discovery skill -- it reports what actions were taken:

```json
{
  "cursor": "{\"last_processed_at\": \"2026-04-13T15:00:00Z\"}",
  "continuation_required": false,
  "registrations": [
    {
      "notice_id": "pc-83a118df-7c2e-f111-813c-005056b64545",
      "process_id": "proc-pc-83a118df-7c2e-f111-813c-005056b64545",
      "source_url": "https://procontract.due-north.com/Supplier/Advert/View?advertId=83a118df-...",
      "action": "registered",
      "pre_filter_result": {
        "goods_check": true,
        "expiry_check": true,
        "category": "Goods - Industrial Equipment",
        "days_until_deadline": 45
      },
      "registered_at": "2026-04-13T15:02:30Z"
    },
    {
      "notice_id": "pc-aaaa-bbbb-cccc",
      "process_id": "proc-pc-aaaa-bbbb-cccc",
      "action": "skipped",
      "skip_reason": "pure_service",
      "pre_filter_result": {
        "goods_check": false,
        "expiry_check": true,
        "category": "Professional Services - Design",
        "days_until_deadline": 60
      }
    }
  ],
  "summary": {
    "total_evaluated": 12,
    "registered": 5,
    "skipped_service": 4,
    "skipped_expiry": 2,
    "skipped_already_registered": 1
  }
}
```

### Database Writes

This skill writes to `process_stage_status` to record interest registration:

```sql
-- Record interest registration for a contract
INSERT INTO process_stage_status (
  id, process_id, stage, scope_kind, scope_id, stage_run_id,
  state, freshness_state, started_at, completed_at, owner_id, row_version, last_local_seq
) VALUES (
  'pss-interest-{process_id}',
  '{process_id}',
  'interest_registration',   -- new stage name
  'process',
  '{process_id}',
  '{stage_run_id}',
  'completed',               -- or 'skipped' for filtered-out contracts
  'fresh',
  '{started_at}',
  '{completed_at}',
  'local-executor-01',
  1,
  {next_seq}
);

-- Advance the stage head
INSERT INTO process_stage_heads (
  process_id, stage, scope_kind, scope_id, current_stage_run_id,
  freshness_state, owner_id, row_version, last_local_seq
) VALUES (
  '{process_id}',
  'interest_registration',
  'process',
  '{process_id}',
  '{stage_run_id}',
  'fresh',
  'local-executor-01',
  1,
  {next_seq}
)
ON CONFLICT(process_id, stage, scope_kind, scope_id) DO UPDATE SET
  current_stage_run_id = excluded.current_stage_run_id,
  freshness_state = excluded.freshness_state,
  row_version = process_stage_heads.row_version + 1,
  last_local_seq = excluded.last_local_seq,
  updated_at = CURRENT_TIMESTAMP;
```

The daemon persistence for this skill will need a **custom persist function** (not `persist_ingestion_from_agent_output`) since the output shape is different. This is the `registrations` array, not `notices`.

### Skill Package Files

Create `skills/procurement-ingestion/procontract-register-interest/`:

**SKILL.md frontmatter:**
```yaml
---
skill_id: procurement-ingestion/procontract-register-interest
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: external_mutation
maxPages: 5
maxScrollCycles: 1
---
```

**IMPORTANT:** The `side_effect_class` is `external_mutation` because this skill clicks "Request Interest" on real procurement portals, which notifies the buying organization.

The SKILL.md playbook must include:
- Login credentials: `sales@sorellon.com` / `9928fJG3!5nk6nc`
- Instructions to read `contracts_to_register` from the job parameters (daemon provides the pre-filtered list)
- Step-by-step portal navigation for EACH contract: login -> open contract URL -> click "Request Interest" -> verify confirmation
- What the confirmation modal looks like ("Expression of interest successful!")
- How to handle already-registered contracts (the tab shows "Expression of interest registered" -- skip and report as `already_registered`)
- How to handle failed registrations (button not found, portal error, etc.)
- Result contract format documenting what happened for each contract

---

## STEP 3: Tender Enrichment Skill

### Skill ID: `procurement-ingestion/procontract-enrich-tender`

### What It Does

Unlike Steps 1 and 2, this skill does NOT receive a pre-filtered list from the daemon. Instead, it **discovers** which tenders are ready by checking the portal directly. The "Recently Added" page is the only way to know if a buyer has responded to our interest request.

1. Logs in and navigates to the Home page: `https://procontract.due-north.com/SupplierPostLoginHome#`
2. Clicks the **"Recently added"** tab in the Activities section
3. Identifies tenders with **stars** (meaning they've been updated/renewed -- the buyer approved our interest request)
4. For each starred tender:
   a. Clicks through to the **Activity Dashboard** page
   b. Opens the **NEW MESSAGE** (NOT the "where you expressed interest" entry -- see critical navigation note below)
   c. Arrives at the **RFx Response / Activity Summary** page with full tender details
   d. Extracts all contract information: full description, deadline, buyer details
   e. Downloads **ALL attachments**: PDFs, DOCX files, ZIP archives
   f. Updates the database with the enriched contract data and new deadline

### CRITICAL: Navigation on Activity Dashboard

From the screenshots, the Activity Dashboard (`/SupplierPostLoginHome/ActivityDashboard?projectId={id}...`) shows multiple events. There are typically TWO entries:

1. **"Expression of interest"** entry -- this is where you PREVIOUSLY registered interest. **DO NOT click this one.**
2. **A NEW message/event** -- this contains the actual tender information. **Click "open" on THIS one.**

The new message entry typically has:
- An "open" button/link
- A description like "Request for Information" or similar
- This is the entry that leads to the full RFx Response page

### RFx Response Page (Full Tender Details)

The Activity Summary page (`/RfxResponse/Index?rfxId={id}`) contains:

**Activity Summary section:**
- Reference number (e.g., "020417")
- Full title
- Buyer organization with logo
- Complete description text (multiple paragraphs)

**Deadline & time remaining:**
- Exact deadline (e.g., "27 May 2026 at 12:00 PM")
- Time remaining countdown (days, hours, minutes)
- **This deadline MUST be extracted and used to UPDATE the contract's `deadline_at` in the database**

**Activity documentation, files & links:**
- Table with columns: Title, Type, Size
- Typically contains:
  - `.docx` files: Invitation to Participate, Specifications, Response Documents, T&Cs, Glossary
  - `.pdf` files: Briefs, technical documents
  - `.zip` files: Appendices, supporting materials
- **ALL of these must be downloaded**

**Terms & conditions:**
- Separate section with T&C documents

**Messages & clarifications:**
- Any buyer communications about the tender

### Document Download & Processing

From the screenshots, typical document sets include:

```
1A. Invitation to Participate and Instructions (To be read).docx    265 KB
2. Specification (To be read) (2).docx                               341 KB
3. Response Document (To be returned).docx                           162 KB
4. T&Cs (with novation) (To be read).DOCX                           204 KB
5. Glossary FINAL (To be read).docx                                   165 KB
6. Stockport Council Priority Cohort and Themes (To be read).docx    269 KB
7. Stockport Town Centre School Brief (To be read).pdf                1 MB
Appendices.zip                                                        1 MB
```

**Document processing strategy:**
- **PDF files** (.pdf): Download to `artifacts/downloads/procontract/{notice_id}/`, store reference in `raw_documents` with `storage_path`. The existing Docling + semantic extraction pipeline handles these automatically.
- **DOCX files** (.docx): Download to same location, store reference in `raw_documents` with `content_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document"`. Use the `docx` extraction approach (Docling supports DOCX or use pandoc conversion to markdown).
- **ZIP files** (.zip): Download, extract contents, process each inner file individually.
- **Other types**: Download and store as raw documents for future processing.

All downloaded documents should be tracked in `raw_documents` and linked via `tender_document_sets`.

### Result Contract Shape

```json
{
  "cursor": "{\"last_checked_at\": \"2026-04-13T16:00:00Z\"}",
  "continuation_required": false,
  "enriched_tenders": [
    {
      "notice_id": "pc-83a118df-7c2e-f111-813c-005056b64545",
      "process_id": "proc-pc-83a118df-7c2e-f111-813c-005056b64545",
      "rfx_id": "c3957abe-e22c-f111-813c-005056b64545",
      "rfx_url": "https://procontract.due-north.com/RfxResponse/Index?rfxId=c3957abe-...",
      "updated_title": "SQ0417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment",
      "updated_description": "Design & Technical Services are looking to procure multi-disciplinary design team...",
      "updated_deadline_at": "2026-05-27T12:00:00Z",
      "buyer_name": "Stockport Metropolitan Borough Council",
      "buyer_contact": {
        "name": "Mr Barry Leake",
        "email": "barry.leake@stockport.gov.uk",
        "organisation": "Stockport Metropolitan Borough Council"
      },
      "documents_downloaded": [
        {
          "filename": "1A. Invitation to Participate and Instructions (To be read).docx",
          "content_type": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
          "size_bytes": 271360,
          "local_path": "artifacts/downloads/procontract/pc-83a118df-.../1a-invitation.docx",
          "document_role_hint": "invitation_to_tender"
        },
        {
          "filename": "2. Specification (To be read) (2).docx",
          "content_type": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
          "size_bytes": 349184,
          "local_path": "artifacts/downloads/procontract/pc-83a118df-.../2-specification.docx",
          "document_role_hint": "technical_specification"
        },
        {
          "filename": "7. Stockport Town Centre School Brief (To be read).pdf",
          "content_type": "application/pdf",
          "size_bytes": 1048576,
          "local_path": "artifacts/downloads/procontract/pc-83a118df-.../7-school-brief.pdf",
          "document_role_hint": "technical_specification"
        }
      ],
      "documents_count": 8,
      "enriched_at": "2026-04-13T16:05:00Z"
    }
  ],
  "summary": {
    "total_checked": 5,
    "enriched": 2,
    "not_yet_responded": 3
  }
}
```

### Database Writes

This skill needs a **custom persist function** that:

1. **Updates `contract_notices`** with enriched data:
   ```sql
   UPDATE contract_notices
   SET summary = ?,
       deadline_at = ?,
       normalized_payload = ?,
       row_version = row_version + 1,
       updated_at = CURRENT_TIMESTAMP
   WHERE id = ?;
   ```

2. **Updates `procurement_processes`** with new deadline:
   ```sql
   UPDATE procurement_processes
   SET latest_deadline_at = ?,
       latest_updated_at = ?,
       row_version = row_version + 1,
       updated_at = CURRENT_TIMESTAMP
   WHERE id = ?;
   ```

3. **Creates `raw_documents`** for each downloaded file:
   ```sql
   INSERT INTO raw_documents (id, source_id, content_type, source_url, storage_path, metadata, created_at)
   VALUES (?, 'procontract', ?, ?, ?, ?, CURRENT_TIMESTAMP);
   ```

4. **Creates `tender_document_sets`** linking all documents to the process:
   ```sql
   INSERT INTO tender_document_sets (
     id, process_id, notice_id, document_set_hash, completeness_state,
     document_roles_json, documents_json, acquired_at, extraction_state,
     owner_id, row_version, last_local_seq, stage_run_id
   ) VALUES (...);
   ```

5. **Creates `process_stage_status`** for stage = `'tender_enrichment'`

### Skill Package Files

Create `skills/procurement-ingestion/procontract-enrich-tender/`:

**SKILL.md frontmatter:**
```yaml
---
skill_id: procurement-ingestion/procontract-enrich-tender
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
maxPages: 5
maxScrollCycles: 2
---
```

The SKILL.md playbook must include:
- Login credentials: `sales@sorellon.com` / `9928fJG3!5nk6nc`
- Navigation to Home page -> "Recently added" tab
- How to identify starred/updated tenders
- **CRITICAL:** On the Activity Dashboard, click the NEW MESSAGE "open" button, NOT the "Expression of Interest" entry
- Full data extraction from RFx Response page
- Document download instructions (all file types: PDF, DOCX, ZIP)
- Deadline extraction and database update
- How to process DOCX files (Docling supports DOCX, or use pandoc to convert to markdown)

---

## Critical Reference: Persistence Function Compatibility

### Step 1 (Discovery) -- Uses existing `persist_ingestion_from_agent_output`

The function at main.rs ~line 5401 reads fields from the result JSON. Here's how it actually works (embedded from source):

```rust
// REQUIRED — function errors if missing:
notice.get("notice_id").and_then(Value::as_str)         // e.g., "pc-83a118df-..."
notice.get("title").and_then(Value::as_str)              // contract title
notice.get("published_at").and_then(Value::as_str)       // ISO 8601
notice.get("source_updated_at").and_then(Value::as_str)  // ISO 8601

// CRITICAL — the function derives process_id as format!("proc-{ocid}"):
notice.get("ocid").and_then(Value::as_str)               // MUST be populated, e.g., "pc-83a118df-..."
// If ocid is empty, process_id becomes "proc-" which COLLIDES across all notices!

// Optional with defaults:
notice.get("source_notice_id")...   // falls back to notice_id
notice.get("summary")...            // optional
notice.get("buyer_name")...         // optional
notice.get("procurement_stage")...  // falls back to "unknown"
notice.get("main_category")...      // optional
notice.get("deadline_at")...        // optional; CLEARED if stage == "awarded"
notice.get("estimated_value_low_pence").and_then(Value::as_i64)  // optional
notice.get("access_route")...       // falls back to "public_web"
notice.get("source_url")...         // falls back to ""
notice.get("raw_release_json")...   // falls back to "{}"
```

**ID derivation pattern (hardcoded in function):**
```
raw_doc_id = format!("raw-{notice_id}")   → "raw-pc-83a118df-..."
process_id = format!("proc-{ocid}")        → "proc-pc-83a118df-..."
```

**You MUST set `ocid` to the notice_id value** (e.g., `"pc-83a118df-..."`) since ProContract has no standard OCID.

### Steps 2 & 3 -- Need custom persist functions

Steps 2 and 3 have different output shapes (`registrations[]` and `enriched_tenders[]` instead of `notices[]`). They need custom persist functions in the daemon. These should:
- Follow the same transactional pattern (write within `state_db.with_write_transaction`)
- Increment `commit_clock` via `next_commit_seq()`
- Write `process_stage_status` and `process_stage_heads` entries
- Use the same `stage_run_id` allocation pattern

---

---

## Critical Reference: Source Registry

The ProContract source needs THREE skill bindings -- one per pipeline step. However, the current `SourceDefinition` struct only supports `production_binding`, `canary_binding`, and `auth_refresh_binding`. For a multi-step pipeline, there are two approaches:

**Approach A: Three separate source definitions** (simpler, recommended for now):
```rust
// Step 1: Discovery
definitions.insert("procontract".to_owned(), SourceDefinition {
    source_id: "procontract".to_owned(),
    kind: SourceKind::Procurement,
    enabled: true,
    source_structure_version: "procontract-2026-04-13".to_owned(),
    config: json!({}),
    production_binding: Some(stable_binding(
        "procurement-ingestion/procontract-discover",
        DEFAULT_SOURCE_SKILL_VERSION,
        Some("0 0 5 * * *"),     // 5:00 AM daily
        "procurement-ingestion",
        "procontract",
    )),
    canary_binding: None,
    auth_refresh_binding: None,
    policy: SourcePolicy::default(),
    default_health_state: HealthState::Green,
});

// Step 2: Interest registration (runs after discovery)
definitions.insert("procontract-interest".to_owned(), SourceDefinition {
    source_id: "procontract-interest".to_owned(),
    kind: SourceKind::Procurement,
    enabled: true,
    source_structure_version: "procontract-interest-2026-04-13".to_owned(),
    config: json!({}),
    production_binding: Some(stable_binding(
        "procurement-ingestion/procontract-register-interest",
        DEFAULT_SOURCE_SKILL_VERSION,
        Some("0 30 5 * * *"),    // 5:30 AM daily (after discovery)
        "procurement-ingestion",
        "procontract-interest",
    )),
    canary_binding: None,
    auth_refresh_binding: None,
    policy: SourcePolicy::default(),
    default_health_state: HealthState::Green,
});

// Step 3: Tender enrichment (runs after interest)
definitions.insert("procontract-enrich".to_owned(), SourceDefinition {
    source_id: "procontract-enrich".to_owned(),
    kind: SourceKind::Procurement,
    enabled: true,
    source_structure_version: "procontract-enrich-2026-04-13".to_owned(),
    config: json!({}),
    production_binding: Some(stable_binding(
        "procurement-ingestion/procontract-enrich-tender",
        DEFAULT_SOURCE_SKILL_VERSION,
        Some("0 0 6 * * *"),     // 6:00 AM daily (after interest registration)
        "procurement-ingestion",
        "procontract-enrich",
    )),
    canary_binding: None,
    auth_refresh_binding: None,
    policy: SourcePolicy::default(),
    default_health_state: HealthState::Green,
});
```

**Approach B: Single source with multiple pipeline stages** (more elegant, future work). For now, use Approach A.

---

## Critical Reference: Error Manifest System

Key ErrorCode variants for these skills:

- `AgentRunFailed` -- skill execution crashed
- `AgentOutputContractInvalid` -- result doesn't match schema
- `AgentAuthChallenge` -- login failed
- `SourceSoftBlocked` -- portal blocked us
- `PageShapeDrift` -- portal UI changed
- `AgentStalled` -- no progress within timeout
- `AgentPartialFailure` -- some items succeeded, some failed

---

## Execution Plan

### PHASE 1A: Build & Test Skill 1 (Opportunity Discovery)

#### Step 1: Deep Codebase Study (MANDATORY — do NOT write any code until complete)

##### 1A. Understand the Daemon→Skill→Daemon Execution Flow

The execution model:
```
Daemon (Rust)                          Skill (Claude CLI in PTY)
─────────────                          ──────────────────────────
1. load_skill() reads SKILL.md
   and result.schema.json
2. Creates workflow_jobs row
3. Spawns Claude CLI in PTY via
   execute_skill_session()
4. Passes job_parameters as JSON  ──►  5. Agent reads playbook
   + skill_markdown as prompt          6. Agent uses agent-browser
                                       7. Agent writes result.json
8. Daemon reads result.json       ◄──  
9. Validates against schema
10. Persists to SQLite
```

**Skills have NO database access.** The daemon provides context via `job_parameters` and handles all DB reads/writes. This is especially important for Skills 2 and 3.

Read: `crates/daemon/src/main.rs` lines 4321-4530 (`run_contracts_finder_live_loop`) and lines 2772-2806 (`execute_skill_session`).

You must be able to answer:
1. How does the daemon pass parameters to a skill session?
2. Where does the skill write its result.json and how does the daemon find it?
3. Why can't a skill query the database directly?

##### 1B. Study the Result Contract Schema and Persistence

Read `skills/procurement-ingestion/contracts-finder/result.schema.json` — Skill 1's result MUST replicate this top-level shape. The required top-level fields are: `cursor` (string), `continuation_required` (boolean), `notices` (array). Per-notice required fields: `notice_id`, `source_notice_id`, `source_url`, `title`, `published_at`, `source_updated_at`, `ocid`, `procurement_stage`.

Read `crates/daemon/src/main.rs` lines 5401-5668 (`persist_ingestion_from_agent_output`) — the embedded field extraction code in the "Persistence Function Compatibility" section below shows exactly what this function reads.

You must be able to answer:
1. What happens if `ocid` is empty? (Answer: `process_id` becomes `"proc-"` — collision.)
2. What is the unique constraint on `contract_notices`? (Answer: `source_id` + `source_notice_id`.)

##### 1C. Study the Reference Skill Playbook

Read `skills/procurement-ingestion/contracts-finder/SKILL.md` — focus on the **Playbook** section (lines 56-217). This shows the exact pattern for writing agent-browser instructions: numbered steps, `agent-browser` commands, field mapping, cursor handling, result.json writing.

Also read `skills/procurement-ingestion/contracts-finder/canary.inputs.jsonl` and `golden.assertions.yaml`.

You must be able to answer:
1. What `agent-browser` commands does the skill use to extract data?
2. How does the skill serialize cursor state for continuation?

##### 1D. Database Schema, PDF Pipeline, Error Manifests

Read (focused sections only):
- `migrations/0001_sqlite_state_initial.sql` — focus on `contract_notices`, `procurement_processes`, `notice_lots`, `process_stage_status`, `process_stage_heads`, `tender_document_sets`
- `migrations/0003_sqlite_runtime.sql` — focus on `raw_documents` (storage_path for files), `source_cursors`
- `migrations/0006_pdf_extraction_toolkit.sql` — focus on `document_extractions`
- `crates/daemon/src/pdf/policy.rs` — how document roles map to extraction overlays
- `crates/daemon/src/sources/registry.rs` lines 562-661 — `builtin_definitions()`
- `crates/daemon/src/errors/manifest.rs` — ErrorCode variants

#### Step 2: Portal Exploration for Skill 1

Navigate the Opportunities listing using `agent-browser`:

```bash
agent-browser open "https://procontract.due-north.com/Opportunities/Index?tabName=opportunities&resetFilter=True"
agent-browser wait
agent-browser snapshot -i
```

**Discovery checklist for Skill 1:**
- [ ] Does the listing page load without login? (It appears to be public)
- [ ] If login required, find login form and authenticate
- [ ] Map the listing table columns and data available
- [ ] Understand pagination (page numbers, how many results per page)
- [ ] Click into 3+ individual opportunity detail pages
- [ ] Extract all fields available on detail pages
- [ ] Note the URL pattern for advertId extraction
- [ ] Check what statuses/categories are available in filters
- [ ] Document the "Request Interest" button location (for Step 2 awareness)

#### Step 3: Build Skill 1 Package

Create `skills/procurement-ingestion/procontract-discover/` with 5 files.

#### Step 4: Agentic Validation Loop for Skill 1

**How to test:** Use the Agent tool to spawn a sub-agent with this prompt:

```
You are a procurement data extraction agent. Read the skill playbook at
skills/procurement-ingestion/procontract-discover/SKILL.md and execute it
step-by-step using agent-browser. Follow every instruction in the Playbook
section exactly. Write the result contract as result.json in the current
working directory. Report: how many notices extracted, any errors, any
fields you could not populate.
```

Alternatively, test via the daemon: `cargo run --release --bin daemon -- --skill-path skills/procurement-ingestion/procontract-discover --state-root state --runtime-dir runtime --artifact-root artifacts`

**Validate result.json yourself after the sub-agent finishes:**

```
┌─────────────────────────────────────────────────────────┐
│           SKILL 1 VALIDATION CHECKLIST                   │
│                                                          │
│  1. JSON is valid (parse with python3 -m json.tool)      │
│  2. Top-level: cursor, continuation_required, notices    │
│  3. At least 1 notice in the array                       │
│  4. Each notice has: notice_id, title, published_at,     │
│     source_updated_at                                    │
│  5. notice_id starts with "pc-"                          │
│  6. ocid is populated (NOT empty — see persistence note) │
│  7. Dates are valid ISO 8601                             │
│  8. Values are integers (pence) if present               │
│                                                          │
│  If FAILS: /error-manifest, diagnose, fix SKILL.md,     │
│            re-test until passing                         │
│  If PASSES: Skill 1 complete, move to Skill 2           │
└─────────────────────────────────────────────────────────┘
```

---

### PHASE 1B: Build & Test Skill 2 (Interest Registration)

#### Step 1: Portal Exploration for Skill 2

Using `agent-browser`, test the interest registration flow:

```bash
# Navigate to a specific opportunity
agent-browser open "https://procontract.due-north.com/Supplier/Advert/View?advertId=83a118df-7c2e-f111-813c-005056b64545&fromProjectDashboard=False&fromRfxSummary=False&selectWorkgroup=False&fromAdvertEvent=False&fromActivityDashboard=False"
agent-browser wait
agent-browser snapshot -i
# Find the "Request Interest" button and understand its behavior
```

**Discovery checklist for Skill 2:**
- [ ] Locate the "Request Interest" button on the detail page
- [ ] Understand what happens after clicking (confirmation modal)
- [ ] Verify the confirmation text matches expectations
- [ ] Check if already-registered contracts show differently (tab state change)
- [ ] Test with a real contract (this is authorized)

#### Step 2: Build Skill 2 Package

Create `skills/procurement-ingestion/procontract-register-interest/` with 5 files.

**Key differences from Skill 1:**
- `side_effect_class: external_mutation`
- Result shape uses `registrations[]` not `notices[]`
- Needs database query logic (find unregistered contracts)
- Includes pre-filtering analysis logic
- Must write `process_stage_status` entries

#### Step 3: Agentic Validation Loop for Skill 2

```
┌─────────────────────────────────────────────────────────┐
│           SKILL 2 VALIDATION LOOP                        │
│                                                          │
│  1. Ensure Skill 1 has run (contracts exist in DB)       │
│                                                          │
│  2. Invoke sub-agent to execute interest registration    │
│     - Sub-agent queries DB for new contracts             │
│     - Sub-agent runs pre-filtering logic                 │
│     - Sub-agent navigates to portal, clicks Request      │
│       Interest on qualifying contracts                   │
│     - Sub-agent writes result.json with registrations    │
│                                                          │
│  3. Validate result.json:                                │
│     - Has registrations array                            │
│     - Each entry has notice_id, action, pre_filter_result│
│     - Actions are "registered" or "skipped"              │
│     - Skip reasons are meaningful                        │
│     - Summary counts are consistent                      │
│                                                          │
│  4. Verify database state:                               │
│     - process_stage_status entries exist for              │
│       stage = 'interest_registration'                    │
│                                                          │
│  5. If FAILS: create error manifest, fix, retry          │
│  6. If PASSES: Skill 2 complete, move to Skill 3        │
└─────────────────────────────────────────────────────────┘
```

---

### PHASE 1C: Build & Test Skill 3 (Tender Enrichment)

#### Step 1: Portal Exploration for Skill 3

Navigate the Home page and Activity Dashboard:

```bash
# Go to home page
agent-browser open "https://procontract.due-north.com/SupplierPostLoginHome#"
agent-browser wait
agent-browser snapshot -i

# Click "Recently added" tab
# Look for starred/updated entries
# Click through to Activity Dashboard
# Find the NEW message (not the interest expression)
# Navigate to RFx Response page
# Map all available data
# Identify document download links
```

**Discovery checklist for Skill 3:**
- [ ] Find the "Recently added" tab on Home page Activities section
- [ ] Identify how starred/updated tenders are visually marked
- [ ] Navigate to Activity Dashboard for a tender
- [ ] Distinguish between "Expression of Interest" entry and NEW message entry
- [ ] Click "open" on the new message to reach RFx Response page
- [ ] Map all fields on the Activity Summary page
- [ ] Find the deadline/time remaining display
- [ ] Locate the "Activity documentation, files & links" section
- [ ] Identify how to download each document
- [ ] Test downloading a document
- [ ] Check "Terms & conditions" section
- [ ] Check "Messages & clarifications" section

#### Step 2: Build Skill 3 Package

Create `skills/procurement-ingestion/procontract-enrich-tender/` with 5 files.

**Key differences from Skills 1 & 2:**
- Result shape uses `enriched_tenders[]`
- Must download actual files (PDF, DOCX, ZIP)
- Must update existing database records (UPDATE not INSERT)
- Must create `raw_documents` entries with `storage_path`
- Must create `tender_document_sets`
- Must update `deadline_at` in both `contract_notices` and `procurement_processes`
- Must write `process_stage_status` for stage = `'tender_enrichment'`
- For DOCX processing: Docling supports DOCX natively, or convert to markdown first

#### Step 3: Agentic Validation Loop for Skill 3

```
┌─────────────────────────────────────────────────────────┐
│           SKILL 3 VALIDATION LOOP                        │
│                                                          │
│  NOTE: This requires Skill 2 to have run first           │
│  (interest must be registered for tenders to appear       │
│  in "Recently added")                                    │
│                                                          │
│  1. Invoke sub-agent to execute enrichment playbook      │
│     - Sub-agent navigates Home -> Recently Added         │
│     - Sub-agent identifies new/starred tenders           │
│     - Sub-agent opens Activity Dashboard                 │
│     - Sub-agent clicks NEW message "open" button         │
│     - Sub-agent extracts full tender details             │
│     - Sub-agent downloads ALL documents                  │
│     - Sub-agent writes result.json                       │
│                                                          │
│  2. Validate result.json:                                │
│     - Has enriched_tenders array                         │
│     - Each entry has notice_id, rfx_id, updated_deadline │
│     - Documents array has local_path entries             │
│     - Downloaded files actually exist on disk            │
│                                                          │
│  3. Verify database state:                               │
│     - contract_notices.deadline_at updated               │
│     - raw_documents entries created for each file        │
│     - tender_document_sets entry created                 │
│     - process_stage_status for 'tender_enrichment'       │
│                                                          │
│  4. If FAILS: create error manifest, fix, retry          │
│  5. If PASSES: All three skills validated!               │
└─────────────────────────────────────────────────────────┘
```

---

### PHASE 2: Daemon Integration + Automated Runtime

**Only begin Phase 2 after ALL THREE skills pass their validation loops.**

#### Step 1: Source Registry Entries

Add three entries to `builtin_definitions()` in `crates/daemon/src/sources/registry.rs`:
- `procontract` (discovery, 5:00 AM)
- `procontract-interest` (interest registration, 5:30 AM)
- `procontract-enrich` (tender enrichment, 6:00 AM)

#### Step 2: Sources Table Inserts

```sql
INSERT OR IGNORE INTO sources (id, kind, enabled, config) VALUES ('procontract', 'procurement', 1, '{}');
INSERT OR IGNORE INTO sources (id, kind, enabled, config) VALUES ('procontract-interest', 'procurement', 1, '{}');
INSERT OR IGNORE INTO sources (id, kind, enabled, config) VALUES ('procontract-enrich', 'procurement', 1, '{}');
```

#### Step 3: Live Loop Functions

Add three live loop functions in `main.rs`:

1. `run_procontract_discover_live_loop()` -- follows `run_contracts_finder_live_loop` pattern, uses `persist_ingestion_from_agent_output`
2. `run_procontract_register_interest_live_loop()` -- custom persist function for `registrations[]` output + `process_stage_status` writes
3. `run_procontract_enrich_tender_live_loop()` -- custom persist function for `enriched_tenders[]` output + document download + DB updates

Add match arms in `--ingest` handler:
```rust
"procontract" => run_procontract_discover_live_loop(&mut runtime, &cli).await?,
"procontract-interest" => run_procontract_register_interest_live_loop(&mut runtime, &cli).await?,
"procontract-enrich" => run_procontract_enrich_tender_live_loop(&mut runtime, &cli).await?,
```

#### Step 4: Custom Persist Functions

Write two new persist functions:
- `persist_interest_registrations()` -- reads `registrations[]`, writes `process_stage_status`
- `persist_tender_enrichments()` -- reads `enriched_tenders[]`, updates `contract_notices`, creates `raw_documents`, creates `tender_document_sets`

#### Step 5: Compile and Test

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings

# Run each stage sequentially (use local state paths)
cargo run --release --bin daemon -- --ingest procontract \
  --state-root state --runtime-dir runtime --artifact-root artifacts

cargo run --release --bin daemon -- --ingest procontract-interest \
  --state-root state --runtime-dir runtime --artifact-root artifacts

cargo run --release --bin daemon -- --ingest procontract-enrich \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

#### Step 6: E2E Validation

```bash
# Verify Step 1: discovery data landed
sqlite3 state/state.db "SELECT count(*), source_id FROM contract_notices WHERE source_id = 'procontract' GROUP BY source_id;"
sqlite3 state/state.db "SELECT id, title, procurement_stage, deadline_at FROM contract_notices WHERE source_id = 'procontract' LIMIT 5;"

# Verify Step 2: interest registrations recorded
sqlite3 state/state.db "SELECT process_id, stage, state FROM process_stage_status WHERE stage = 'interest_registration' LIMIT 10;"

# Verify Step 3: enriched data and documents
sqlite3 state/state.db "SELECT id, content_type, storage_path FROM raw_documents WHERE source_id IN ('procontract', 'procontract-enrich') AND storage_path IS NOT NULL LIMIT 10;"
sqlite3 state/state.db "SELECT id, process_id, completeness_state FROM tender_document_sets WHERE process_id LIKE 'proc-pc-%' LIMIT 10;"

# Verify no orphaned records
sqlite3 state/state.db "SELECT cn.id FROM contract_notices cn LEFT JOIN procurement_processes pp ON cn.process_id = pp.id WHERE cn.source_id = 'procontract' AND pp.id IS NULL;"
```

---

## Deliverables Checklist

### Phase 1A: Skill 1 (Discovery)
- [ ] Portal exploration notes
- [ ] `skills/procurement-ingestion/procontract-discover/SKILL.md`
- [ ] `skills/procurement-ingestion/procontract-discover/result.schema.json`
- [ ] `skills/procurement-ingestion/procontract-discover/checkpoint.schema.json`
- [ ] `skills/procurement-ingestion/procontract-discover/canary.inputs.jsonl`
- [ ] `skills/procurement-ingestion/procontract-discover/golden.assertions.yaml`
- [ ] Successful validation loop (valid result.json)

### Phase 1B: Skill 2 (Interest Registration)
- [ ] Interest registration portal flow documented
- [ ] `skills/procurement-ingestion/procontract-register-interest/SKILL.md`
- [ ] `skills/procurement-ingestion/procontract-register-interest/result.schema.json`
- [ ] `skills/procurement-ingestion/procontract-register-interest/checkpoint.schema.json`
- [ ] `skills/procurement-ingestion/procontract-register-interest/canary.inputs.jsonl`
- [ ] `skills/procurement-ingestion/procontract-register-interest/golden.assertions.yaml`
- [ ] Successful validation loop (interest registered, DB updated)

### Phase 1C: Skill 3 (Tender Enrichment)
- [ ] Activity Dashboard + RFx Response flow documented
- [ ] `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md`
- [ ] `skills/procurement-ingestion/procontract-enrich-tender/result.schema.json`
- [ ] `skills/procurement-ingestion/procontract-enrich-tender/checkpoint.schema.json`
- [ ] `skills/procurement-ingestion/procontract-enrich-tender/canary.inputs.jsonl`
- [ ] `skills/procurement-ingestion/procontract-enrich-tender/golden.assertions.yaml`
- [ ] Successful validation loop (documents downloaded, DB enriched)

### Phase 2: Daemon Integration
- [ ] Three source registry entries in `registry.rs`
- [ ] Three source rows in SQLite
- [ ] `run_procontract_discover_live_loop()` in `main.rs`
- [ ] `run_procontract_register_interest_live_loop()` in `main.rs`
- [ ] `run_procontract_enrich_tender_live_loop()` in `main.rs`
- [ ] `persist_interest_registrations()` custom persist function
- [ ] `persist_tender_enrichments()` custom persist function
- [ ] Three `--ingest` match arms
- [ ] Successful `cargo check && cargo clippy`
- [ ] Successful sequential E2E run of all three stages
- [ ] Data verified in all relevant tables

---

## Constraints & Guardrails

1. **Result contract compatibility:** Step 1 MUST use the standard `notices[]` shape for `persist_ingestion_from_agent_output`. Steps 2 and 3 use custom shapes that need custom persist functions.
2. **Notice ID prefix:** All IDs start with `pc-` for ProContract.
3. **Side effect awareness:** Skill 2 (`external_mutation`) clicks real buttons on a live portal. Only register interest on contracts that pass pre-filtering.
4. **Sequential dependency:** Step 2 depends on Step 1 data. Step 3 depends on Step 2 actions. The cron schedule must enforce this ordering (5:00 AM -> 5:30 AM -> 6:00 AM).
5. **Document types:** The portal serves DOCX files (not just PDFs). The extraction pipeline supports DOCX via Docling. ZIP files must be extracted first.
6. **Activity Dashboard navigation:** On the Activity Dashboard, there are multiple events. ALWAYS open the NEW message, NEVER click the "Expression of Interest" entry. The screenshots annotate this clearly.
7. **Deadline updates:** Step 3 MUST update `deadline_at` in both `contract_notices` AND `procurement_processes` with the actual deadline from the RFx Response page.
8. **Monetary values in pence.** Convert from pounds to pence (multiply by 100).
9. **ISO 8601 dates.** All date fields in ISO 8601 format.
10. **PTY interactive mode** for all three skills (browser automation required).
11. **Access route:** Set to `"portal_authenticated"` for all ProContract data.
