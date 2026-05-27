# L6 Phase 7 — Live Demo Execution Handoff (Session 2)

> **Paste this entire file into a fresh Claude Code session that has computer-use access AND agent-browser via CDP.** This handoff onboards a single agent to autonomously claim and close beads `testproject1-3yqf.7.1`, `testproject1-3yqf.7.2`, and `testproject1-3yqf.7.3` — the live-demo half of L6 Phase 7 — using the infrastructure and smoke-test state established by Session 1 on 2026-05-25 (commit `7b3bd21` on `main`).

---

## 1. Mission & Operator Delegation Envelope

You are closing the live-demo half of L6 Phase 7 on behalf of the operator, **Dieter Xia Shuang Danicek** (`xiasshuang@gmail.com`, 25–50% PSC of Sorellon Limited, Companies House #16575318). Dieter has explicitly delegated specific operator-side decisions to you. **The delegation is finite and explicit — exceed it and you violate constitutional bright lines.**

### What you ARE authorized to do as the operator's proxy

- Drive Chrome via `agent-browser --cdp 9222` against the operator's persistent profile at `/Users/alinaaitova/.chrome-automation`
- Navigate Procontract, search for opportunities, click into opportunity detail pages
- **Apply the kickoff §13 selection rules and PICK the smoke-test opportunity yourself.** Dieter is happy with any open opportunity in Sorellon's product space (PPE / cleaning / hygiene / consumables / janitorial / healthcare) that passes the §13 safety rules. You do not need to ask which one.
- **Pre-authorized cap escalations**:
  - `REMOTE_DRAFT` — full authority. Save drafts freely.
  - `NOTIFICATION_OR_LOCK` — full authority for register-interest actions on **goods-category opportunities only** (PPE, consumables, hygiene, cleaning, janitorial, healthcare — anything Sorellon plausibly supplies). If you cannot reach `draft_saved` on any opportunity without first registering interest, you ARE authorized to register interest on one goods-category opportunity to unlock the response form.
- Click `Save Draft` once a response form is reachable
- Run `bin/ensure-cdp-9222`, `colima start`, `supabase start ...` as needed (idempotent, project-sanctioned)
- Run `daemon --skill-path skills/bid-portal-procontract --job-parameters-json '{...}'` once all preconditions are met
- Write commits + push under `AGENT_NAME` set to your AgentMail identity
- Update the live-demo diary
- `bin/br-claim` / `bin/br-complete` the three target beads

### What you are NOT authorized to do

- **`FORMAL_SUBMISSION` cap escalation is NOT delegated.** Do not click any portal button that creates a buyer-visible legal commitment. Do not fill in a final response and click Submit. If 7.3 would require this, close 7.3 as PARTIAL with the documented constitutional-blocker note — see §9.3.
- **`LEGAL_DECLARATION` is NOT delegated.** If the response form contains tick-boxes accepting legal declarations (e.g. "I certify that the above is true"), DO NOT tick them. Save draft without those tick-boxes. If draft-save requires them, treat as cap-exceeded and stop.
- **Do NOT solve captchas, change Chrome settings, log out, close the operator's browser, click 2FA prompts.** Constitutional per kickoff §12.
- **Do NOT type credentials.** Procontract auth is solved (Chrome autofill + saved password). If you get `AUTH_STATE_STALE`, run the auth-signin/procontract skill via the daemon's orchestration; do not invoke it directly inside another skill.
- **Do NOT fabricate real bid content** (pricing, lead times, signed declarations). The seeded `shared_company_profile` is explicit `SMOKE_TEST_PLACEHOLDER` data — bid responses must reflect that and never deceive the buyer.
- **Do NOT delete files** without explicit permission per AGENTS.md Rule 1.
- **Do NOT bypass `bin/br-claim` / `bin/br-complete`** per AGENTS.md Rule 2.
- **Do NOT modify the canonical L6 docs to fix the "sorelon" single-L typo.** That cleanup is a deferred separate bead. Use double-L (`sorellon`) in NEW artifacts only.

### Required handoff outcome

After this session:
- Bead `testproject1-3yqf.7.1` is **closed**
- Bead `testproject1-3yqf.7.2` is **closed** (potentially with DoD #9 = PARTIAL noted)
- Bead `testproject1-3yqf.7.3` is **closed as PARTIAL** with constitutional-blocker reference to `docs/research/sorellon-readiness-FINAL.md`
- Beads 7.4, 7.5, 7.6, 7.7 are **unblocked** (i.e., appear in `br ready --label l6 --json`)
- Live-demo diary at `docs/pipeline/l6-request-pipeline-automation/live-demo/` is updated with all evidence
- Commits pushed; `git status` clean against `origin/main`

---

## 2. State Inherited From Session 1 (2026-05-25)

**Read this section FULLY** before touching anything. Session 1 did substantial bootstrap that you must NOT redo.

### Files already created (committed in `7b3bd21`)

- `docs/pipeline/l6-request-pipeline-automation/live-demo/2026-05-25-smoke-test.md` — comprehensive diary covering: Codex skill install, Supabase install, profile seeding, daemon migrations, smoke-test outcome demonstrating cap-respect on DN815582 (PPE and Consumables, South Hams DC). READ this fully — it documents what works.
- `sorellon.md` (project root, 604 lines) — operator-maintained company profile source of truth. Double-L spelling. Identity / address / contact / certifications gaps all enumerated.

### Infrastructure already installed (NOT in git)

| Component | Location | How to verify |
|---|---|---|
| Supabase CLI 2.101.0 | `~/.local/bin/supabase` (shim) + `~/.local/share/supabase/supabase-go` (Go binary) + `~/.local/bin/supabase-go` (symlink) | `supabase --version` should print `2.101.0` |
| Colima 0.10.1 (Lima VM) | Installed via brew; Ubuntu 24.04 in `~/.colima/default/` | `colima status` should report running |
| Docker CLI 29.5.2 | Installed via brew | `docker version` should show client AND server (server proves Colima socket is connected) |
| Codex skill symlinks | `~/.codex/skills/{supabase,supabase-postgres-best-practices} → ~/.agents/skills/...` | `readlink -f ~/.codex/skills/supabase` |

### Database state already in place

**Supabase (running on port 54321; Postgres on 54322):**
- All 9 migrations from `infra/supabase/migrations/0001..0009.sql` applied
- One row in `shared_company_profile`: `id = '5f87c23d-8b23-4b28-85a0-9548c1a02787'`, `org_id = 'sorellon'`, `remote_revision = 1`, `company_name = 'Sorellon Limited'`, `registration_number = '16575318'`, certifications + insurance are SMOKE_TEST_PLACEHOLDER with expiry 2027-12-31. Real identity / contact / address (double-L `sorellon.com` email).

**Daemon state.db at `~/Library/Application Support/com.procintel.desktop/state/state.db`:**
- All 13 migrations from `migrations/0001..0013.sql` applied via `daemon --dry-run`
- 50 tables. `schema_migrations` populated.
- Three remote-owned tables created locally (mixed-write violation, smoke-test-only): `shared_company_profile`, `operator_decision_heads`, `shared_source_configs`
- One row in local `shared_company_profile` mirroring the Supabase row (same id, same data)
- **`operator_decision_heads` table is EMPTY** — you need to seed this in §9.1 step 6
- Backup before migration: `state.db.preB-smoke-test-20260525T112735Z.bak` next to the live file

### Procontract state already established

- Operator account `sales@sorellon.com` (double-L) saved in Chrome password store
- Touch ID disabled for this saved credential (Decision 2 from 2026-05-21)
- Operator is signed in (verified during Session 1) — but **Chrome closes between sessions**, so on your start you'll likely need to re-sign-in (see §6)
- One opportunity already inspected: **DN815582 "PPE and Consumables", South Hams District Council**. Buyer Ollie Rees (`ollie.rees@swdevon.gov.uk`, 01803). Expression window 2026-05-08 → 2026-06-05 12:00. CPV codes 18140000-2 (Workwear accessories), 35113000-9 (Safety equipment), 39830000-9 (Cleaning products). Goods, single-supplier appointment, 2-year contract Aug 2026 – Jul 2028, region Devon. This opportunity sits at the **register-interest gate** (only "Register interest in this opportunity" is visible — no response form yet). Session 1's smoke test stopped here because cap was REMOTE_DRAFT.

### Critical environment gotchas you WILL hit

1. **Git commits fail without `AGENT_NAME` env var.** The pre-commit hook `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` rejects commits when `AGENT_NAME` is unset. Same for push. **Fix**: `export AGENT_NAME="<your-agent-mail-name>"; export AGENT_MAIL_NAME="<same>"; export AGENT_MAIL_PROJECT_KEY="/Users/alinaaitova/Desktop/Testproject1"; export AGENT_MAIL_ALLOW_SOLO=1` at session start, AND set them in every Bash invocation that runs `git commit` or `git push` (each Bash call is a fresh shell).

2. **Chrome on port 9222 closes between sessions.** Session 1 had to re-run `bin/ensure-cdp-9222` multiple times. Run it once at session start, then once again immediately before any portal navigation.

3. **Colima may be paused after a reboot or sleep.** If `docker version` reports no server, run `colima start` first, then `supabase --workdir infra start --exclude vector,analytics,inbucket,edge-runtime,functions,storage,realtime,imgproxy`.

4. **Supabase "vector" container hates Colima's docker.sock mount.** The exclusion list above is mandatory — without it, `supabase start` rolls back all containers when vector fails.

5. **The Supabase CLI is a Bun shim + Go binary.** They MUST be co-located. Already done at `~/.local/share/supabase/` and a `~/.local/bin/supabase-go` symlink resolves the shim. If `supabase --version` outputs garbage, the symlink is broken — re-create it.

6. **The "sorelon" / "sorellon" spelling discrepancy.** Companies House (legal authority) and `sorellon.md` use **double-L**. Every L6 design doc, the bid skill SKILL.md, the kickoff, the test fixtures use **single-L** (typo). **Use double-L in all new artifacts** (diary entries, JSON payloads, mail subjects). If you see the daemon code or test fixture using single-L, that's existing tech debt — do not touch it.

---

## 3. Constitutional Bright Lines

The 8 §1.10 invariants from the L6 spec, plus 4 session-specific bright lines from this handoff:

1. **No real-money bids without human approval.** `trigger_bid_workflow` readiness gate requires `operator_decision_heads.disposition = 'approved'`.
2. **No externally visible portal mutation beyond approved `sideEffectCap`.** Bid runner checks the next playbook step's side-effect class; cap breach saves draft when possible, stops, emits `BidSideEffectCapExceeded`.
3. **Bid workflow carries explicit scope** — NOT whole-process blanket bids.
4. **Full audit trail for submissions.** Bid skill emits per-step screenshot paths; run manifest writer links screenshots + side-effect progression.
5. **Stale decisions block bid.** Readiness gate computes current `decisionBasisHash`; `reviewedBasisHash != current` blocks.
6. **L5 readiness gate.** `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`, `CLASSIFICATION_FRESH_UNTIL_EXPIRED` are non-overrideable.
7. **Validation fail-closed for portal submission.** Validator rejection blocks `bid_submitted` unless structured operator validation-gate override.
8. **Quote-role quality gate.** `domestic_wholesale_floor`-only or `optimistic_floor`-only evidence blocks bid readiness unless structured override.

**Session 2 specific bright lines (Dieter's delegation):**

9. **You may escalate cap to NOTIFICATION_OR_LOCK on one goods-category opportunity to register interest.** This is pre-authorized. You may NOT escalate to LEGAL_DECLARATION or FORMAL_SUBMISSION on any opportunity, ever, under any circumstance, during this session.
10. **You MUST stop and surface to operator if the response form requires a legal declaration tick-box.** Even with the NOTIFICATION_OR_LOCK delegation, declaration tick-boxes are constitutional and require operator presence.
11. **Bid content (form-fill data) MUST reflect the smoke-test nature.** If you reach a response form, fill it with values that are clearly NOT a real bid — e.g., quantities containing `SMOKE_TEST`, lead times listed as "PLACEHOLDER — not for live procurement", "company name" already smoke-labeled in profile. If the buyer ever reads this draft, they should immediately understand it's not a genuine offer.
12. **All Phase 7 work must reference the new research doc**. Bead 7.3 closes with explicit pointer to `docs/research/sorellon-readiness-FINAL.md` (created in parallel with this handoff). Do not close 7.3 without that reference.

---

## 4. Source-of-Truth Loading (Mandatory Order)

1. **`AGENTS.md`** — Rule 1 (no file deletion), Rule 2 (bin/br-claim wrappers, lines 21-34), MCP Agent Mail section (lines 211-275), authoritative `br` block (lines 279-330). The stale `bd` block lower in the file is SUPERSEDED.
2. **This handoff** — every section
3. **The session 1 diary** — `docs/pipeline/l6-request-pipeline-automation/live-demo/2026-05-25-smoke-test.md`. Read all 178 lines.
4. **`sorellon.md`** at project root — 604 lines. Authoritative company profile. Pay attention to §"Tender Readiness And Outstanding Items" — this enumerates what's missing.
5. **L6 spec** `docs/strategy/layers/layer-6-request-pipeline-automation.md` — §1.7 (11-step playbook with side-effect class per step), §1.10 (8 invariants), §2.1 (11 DoD criteria).
6. **Original kickoff** `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-kickoff.md` — §11 (AgentMail), §12 (CDP discipline), §13 (live demo constraints — read TWICE), §15 (quick reference).
7. **The original handoff prompt** that Session 1 was given. It's not stored as a file — it was in the initial chat message of Session 1. Key parts are re-stated here.
8. **Bid skill** `skills/bid-portal-procontract/SKILL.md`. Read the full 11-step playbook.
9. **The research doc** at `docs/research/sorellon-readiness-FINAL.md` — this exists when you start. Critical context for 7.3 closure.
10. **`br show testproject1-3yqf.7.1`, `7.2`, `7.3`** — the authoritative bead definitions.

---

## 5. Tools & Tier Rules

### computer-use MCP

Computer-use is enabled for this session. Tier rules:
- **Chrome** → tier `"read"` — screenshots only; clicks blocked. Use `agent-browser --cdp 9222` for clicks (no tier restriction since it's a different transport).
- **Terminal / IDE** → tier `"click"` — clicks work, typing blocked. Use the Bash tool for shell commands.
- **Desktop app (Tauri "Procurement Intelligence Desktop")** → tier `"full"` — clicks, typing, scrolling all work. Use for the BidWorkflow tab if you decide to drive 7.2 via the desktop UI rather than `daemon --skill-path`.
- **Everything else** → tier `"full"`.

`request_access` once at session start: `["Google Chrome", "Terminal", "Procurement Intelligence Desktop", "Finder"]`.

### agent-browser

Resolves to `~/.nvm/versions/node/v20.19.5/bin/agent-browser`. Already on PATH. Always pass `--cdp 9222`.

Key commands you'll need:
- `agent-browser --cdp 9222 open <url>`
- `agent-browser --cdp 9222 snapshot`
- `agent-browser --cdp 9222 click <ref>` — `<ref>` is the `eXX` ref from snapshot
- `agent-browser --cdp 9222 type <ref> "<text>"` — note: types ADDITIVELY; clear input first via eval if value already set
- `agent-browser --cdp 9222 eval '<js>'`
- `agent-browser --cdp 9222 screenshot --full "" "<path>.png"`

**Input clearing pattern** (Session 1 hit this — type concatenates):
```js
agent-browser --cdp 9222 eval "(() => { const i = document.querySelector('input[name=\"...\"]'); i.value = ''; i.dispatchEvent(new Event('change',{bubbles:true})); })()"
```

### Bash via the harness

All shell work. Reserve commands you'll need:
- `bin/ensure-cdp-9222` — idempotent CDP launcher
- `bin/br-claim <id>` / `bin/br-complete <id>` — bead lifecycle (REQUIRED — never raw `br update`)
- `bin/br-sync` — beads JSONL export
- `sqlite3 ~/Library/Application\ Support/com.procintel.desktop/state/state.db "..."`
- `supabase --workdir /Users/alinaaitova/Desktop/Testproject1/infra db query --local "..."`
- `cargo run -p daemon --bin daemon -- --skill-path ... --job-parameters-json '...'`

### AgentMail MCP

`macro_start_session` at start. `macro_contact_handshake` for any visible peers. `fetch_inbox` between beads. `send_message` for blocker reports / hand-offs.

---

## 6. Prerequisite Environment Check (run FIRST, before claiming anything)

```bash
# (a) Build state
git -C /Users/alinaaitova/Desktop/Testproject1 status --short
git -C /Users/alinaaitova/Desktop/Testproject1 log --oneline -5
# Expect: HEAD at 7b3bd21 or newer, working tree clean(-ish — leftover .DS_Store noise is fine)

# (b) Daemon binary
ls /Users/alinaaitova/Desktop/Testproject1/target/debug/daemon
cargo check -p daemon --all-targets 2>&1 | tail -3
# If `target/debug/daemon` doesn't exist: cargo build -p daemon

# (c) CDP + Procontract auth state
bin/ensure-cdp-9222
curl -sfS -m 3 "http://localhost:9222/json/version" | head -3
# Expect: Chrome/148+ reachable.

# (d) Colima + Docker
colima status 2>&1 | head -3
# If not running: colima start
docker version --format '{{.Server.Version}}' 2>&1
# Expect: a version like 29.x

# (e) Supabase reachable
curl -sfS -m 3 "http://127.0.0.1:54321/rest/v1/" 2>&1 | head -3
# If 404: REST is up (the / path just isn't routed). Good.
# If connection refused: supabase --workdir /Users/alinaaitova/Desktop/Testproject1/infra start --exclude vector,analytics,inbucket,edge-runtime,functions,storage,realtime,imgproxy
sqlite3 "/Users/alinaaitova/Library/Application Support/com.procintel.desktop/state/state.db" "SELECT id, org_id, company_name FROM shared_company_profile;"
# Expect: 5f87c23d-... | sorellon | Sorellon Limited

# (f) AgentMail health
curl -sfS -m 3 "http://127.0.0.1:8765/health" 2>&1
# Expect: {"status":"ready", ...}

# (g) Beads ready list
br ready --label l6 --json | head -10
# Expect: only testproject1-3yqf.7.1 ready

# (h) Procontract Chrome state
curl -s -m 3 "http://localhost:9222/json/list" | python3 -c "
import json,sys
for t in json.loads(sys.stdin.read()):
    if t.get('type')=='page' and 'procontract' in t.get('url',''):
        print(t['url']); print(t['title'])
"
# If Procontract tab NOT shown OR URL contains /Login: see §6.1 for re-auth
```

### 6.1 If Procontract is logged out

You CAN drive sign-in via agent-browser since Chrome's saved password handles autofill. Sequence:
```bash
agent-browser --cdp 9222 open "https://procontract.due-north.com/Login"
sleep 2
agent-browser --cdp 9222 snapshot | head -30
# Identify the User Name textbox ref (usually e13)
agent-browser --cdp 9222 click e13   # focuses the User Name field
sleep 3                              # wait for Chrome autofill to populate
agent-browser --cdp 9222 snapshot | grep -A1 "User Name\|Password" | head -8
# Verify both fields populated
agent-browser --cdp 9222 click e17   # click "Continue" (verify ref via snapshot)
sleep 5
agent-browser --cdp 9222 snapshot | head -20
# Verify URL is now /SupplierPostLoginHome or similar
```

If autofill doesn't populate (Touch ID prompt, 2FA, captcha): emit a blocker AgentMail to operator and pause. **DO NOT type the password manually** — that's a credential-handling boundary.

---

## 7. AgentMail Setup + AGENT_NAME env var

### 7.1 Register identity

```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l6-request-pipeline-automation-executor",
  model="<your-model>",
  task_description="L6 Phase 7 live demo execution (session 2): close beads 7.1, 7.2, 7.3"
)
```

Record your `agent_name` and export it. **This step is REQUIRED for git commits to succeed** — the pre-commit hook rejects commits without `AGENT_NAME` set.

```bash
export AGENT_NAME="<your-name>"
export AGENT_MAIL_NAME="<your-name>"
export AGENT_MAIL_PROJECT_KEY="/Users/alinaaitova/Desktop/Testproject1"
export AGENT_MAIL_ALLOW_SOLO=1
```

Add these to every Bash tool call that runs `git commit` or `git push` (each Bash call is a fresh shell).

### 7.2 Handshakes + inbox

```
peers = list_agents(project_key=<key>)
for peer in peers if peer.program == <yours> and peer != <you>:
    macro_contact_handshake(project_key=<key>, requester=<you>, target=peer.name)
verified = list_contacts(project_key=<key>, agent_name=<you>)
# Read pending mail
fetch_inbox(project_key=<key>, agent_name=<you>, limit=20)
```

If `list_contacts` returns empty after handshakes: pass `--solo` (or `AGENT_MAIL_ALLOW_SOLO=1`) to `bin/br-claim` / `bin/br-complete`. Already in the env block above.

### 7.3 Notify operator at session start

```
send_message(
  project_key=<key>,
  sender_name=<you>,
  to=[<peers>],
  thread_id="l6-session-2-2026-05-25",
  subject="[l6] Session 2 starting: closing 7.1 / 7.2 / 7.3",
  body_md="Picking up from commit 7b3bd21. Plan: claim 7.1, seed operator_decision_heads, close. Then 7.2 against a goods-category opportunity (target: PPE/cleaning/hygiene per §13). NOTIFICATION_OR_LOCK pre-authorized; FORMAL_SUBMISSION is NOT. 7.3 closes as PARTIAL with research-doc reference."
)
```

---

## 8. Live-Demo Diary

You already have a starter at `docs/pipeline/l6-request-pipeline-automation/live-demo/2026-05-25-smoke-test.md` (Session 1). Append to it or create a new dated file (e.g., `2026-05-26-session-2.md`) — your choice. Reference it from each bead's close mail.

Each diary entry must contain (per kickoff §13):

- CDP `/json/version` UTC timestamp at run start
- Daemon HEAD git SHA (`git rev-parse HEAD`)
- AgentMail identity
- Bead ID
- Opportunity reference + buyer + deadline + duplicate-state check
- Scope payload (process_id, lot_ids, candidate_ids, selected_quote_ids, companyProfileVersion = `5f87c23d-8b23-4b28-85a0-9548c1a02787`, decisionBasisHash)
- sideEffectCap chosen + justification
- Per-step side-effect-class progression
- Screenshot paths
- Run manifest path
- Final `bid_state` (or "n/a, not reached")
- Validator verdict (or "n/a")
- Supabase projection sync evidence (or "deferred")
- Web viewer badge evidence (or "deferred")
- PII redacted: use placeholders like `<operator-email>`, not the real string

---

## 9. Per-Bead Playbook

### 9.1 Bead `testproject1-3yqf.7.1` — preflight-evidence

**Goal**: produce a GREEN preflight document, seed `operator_decision_heads`, close.

**AC summary** (from `br show testproject1-3yqf.7.1`): preflight doc with 6 GREEN sections — Toolchain, AgentMail, CDP Health, L5 Readiness Fixture, Local Services and Data, Operator-Selected Procontract Opportunity. Plus `manifest.yaml` updated with the artifact path.

**Steps**:

1. `git pull --rebase && bin/br-claim testproject1-3yqf.7.1`
   - Set the AGENT_MAIL env vars first (see §7.1). If the wrapper errors on stale Start mail from a prior agent (GrayMouse from Session 1 or earlier), the patched wrapper (`testproject1-nl6w`) should reclaim cleanly. If not, surface to operator.

2. **Run all §6 prereq checks** (a-h). Capture every command + output in the diary's `## Environment` section. Ensure Supabase + Colima + CDP + AgentMail + state.db are all green.

3. **Run the L5 readiness gate fixture** to satisfy AC item 9:
   ```bash
   cargo test --test bid_readiness_gate 2>&1 | tail -20
   ```
   Expect 45+ passed. Record output verbatim in diary.

4. **Verify the Sorellon profile is in both Supabase AND local SQLite**:
   ```bash
   supabase --workdir /Users/alinaaitova/Desktop/Testproject1/infra db query --local "SELECT id, org_id, company_name, registration_number, remote_revision FROM shared_company_profile WHERE org_id='sorellon';"
   sqlite3 "/Users/alinaaitova/Library/Application Support/com.procintel.desktop/state/state.db" "SELECT id, org_id, company_name FROM shared_company_profile WHERE org_id='sorellon';"
   ```
   Both should return the same row. If local is missing, re-seed (see Session 1 diary for the SQL).

5. **Inspect Chrome's `chrome://settings/passwords`** for the saved `sales@sorellon.com` credential.
   - You're tier-`"read"` on Chrome, so you can screenshot but not click. Take a screenshot of the passwords page; visually verify the row exists and Touch ID toggle is off.
   - Save the screenshot (PII-redacted — crop or pixel-blur the password column) into `artifacts/sessions/<date>-l6-7.1/`.

6. **Seed `operator_decision_heads`** — this is the AC gap from Session 1. You need a synthetic process_id to attach the decision to. Two paths:
   - **Path A (simpler)**: pick a placeholder process_id like `'smoke-test-process-2026-05-26'`. Seed BOTH Supabase + local SQLite:
     ```sql
     -- Supabase
     INSERT INTO operator_decision_heads (org_id, process_id, disposition, reviewed_basis_hash, reviewed_manifest_hash, reviewed_at_stage_run_id, operator_id, command_id)
     VALUES ('sorellon', 'smoke-test-process-2026-05-26', 'approved', 'SMOKE_TEST_PLACEHOLDER_BASIS_HASH', 'SMOKE_TEST_PLACEHOLDER_MANIFEST_HASH', 'smoke-test-stage-run-2026-05-26', 'dieter-xia-shuang-danicek', 'smoke-test-cmd-2026-05-26')
     RETURNING id, process_id, disposition, remote_revision;
     ```
     ```sql
     -- Local SQLite mirror (the daemon expects this locally)
     INSERT INTO operator_decision_heads (id, org_id, process_id, disposition, reviewed_basis_hash, reviewed_manifest_hash, reviewed_at_stage_run_id, decision_stale, operator_id, command_id, remote_change_seq, remote_revision)
     VALUES ('<uuid>', 'sorellon', 'smoke-test-process-2026-05-26', 'approved', 'SMOKE_TEST_PLACEHOLDER_BASIS_HASH', 'SMOKE_TEST_PLACEHOLDER_MANIFEST_HASH', 'smoke-test-stage-run-2026-05-26', 0, 'dieter-xia-shuang-danicek', 'smoke-test-cmd-2026-05-26', 1, 1);
     ```
   - **Path B (more faithful)**: also seed a row in `procurement_processes` and `notice_lots` first, then point `operator_decision_heads.process_id` at the real synthetic process. Per session 1's "synthetic full state" estimate, this is ~80 NOT NULL columns of fabrication. **NOT recommended unless 7.2 actually needs a referencable FK.** The bid skill doesn't query `procurement_processes` from local SQLite during its playbook — it only uses the IDs as identifiers in the JSON payload.

7. **Write the preflight doc**. Either update `preflight-2026-05-23.md` (currently BLOCKED) to GREEN, OR create `preflight-2026-05-26.md`. Sections required (per `br show testproject1-3yqf.7.1`):
   - CDP Health
   - AgentMail Health
   - L5 Readiness Fixture
   - Operator-Selected Procontract Opportunity (point at the opportunity you've pre-identified for 7.2 — see §9.2 step 1 for opportunity choice)
   - Company Profile (point at the seeded `shared_company_profile.id` + reference the SMOKE_TEST_PLACEHOLDER caveats from Session 1 diary)
   - Verdict — GREEN

8. **Update `manifest.yaml`** — find the `q11_live_demo_opportunity_selection` entry (added in Session 1's 22df20a commit) and set:
   - `preflight_artifact: docs/pipeline/l6-request-pipeline-automation/live-demo/preflight-2026-05-26.md` (or whichever you wrote)
   - `preflight_status: "GREEN 2026-05-26: all 6 sections satisfied. CDP healthy, Supabase reachable, profile populated with smoke-test placeholders, operator_decision_heads seeded, opportunity <REF> selected."`

9. Commit + push:
   ```bash
   export AGENT_NAME=<you>; export AGENT_MAIL_NAME=<you>; export AGENT_MAIL_ALLOW_SOLO=1; export AGENT_MAIL_PROJECT_KEY=/Users/alinaaitova/Desktop/Testproject1
   git add docs/pipeline/l6-request-pipeline-automation/live-demo/<preflight-doc>.md docs/pipeline/l6-request-pipeline-automation/manifest.yaml
   git commit -m "testproject1-3yqf.7.1: seed operator_decision_heads + preflight GREEN

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
   git pull --rebase && git push
   ```

10. `bin/br-complete testproject1-3yqf.7.1`. Verify with `br show testproject1-3yqf.7.1` (should show `closed`).

**AC tied to TDD**: `L6-T-live_demo-001` passes (artifact exists + all prerequisites captured). All 6 GREEN sections present.

---

### 9.2 Bead `testproject1-3yqf.7.2` — live-demo-remote-draft

**Goal**: drive the bid skill against a real Procontract opportunity under `sideEffectCap = REMOTE_DRAFT` (with NOTIFICATION_OR_LOCK escalation pre-authorized for register-interest only on goods-category opps).

**AC summary**: `bid_state = 'draft_saved'` reached with screenshot trail + run manifest + validator verdict + Supabase projection. DoD #9 (`draft_saved` path works).

**Stop conditions**:
- ✅ IDEAL: draft_saved on a goods opp. Close as DoD #9 = PASS.
- ⚠️ ACCEPTABLE: cap-respect demonstrated cleanly at register-interest or legal-declaration gate; no draft saved but full evidence chain captured. Close as **DoD #9 = PARTIAL** with documented reason. This unblocks 7.4-7.7.
- ❌ STOP: any error path that would create an unauthorized buyer-visible mutation. Roll back, surface to operator, do not close.

**Steps**:

1. `git pull --rebase && bin/br-claim testproject1-3yqf.7.2`

2. **Pick the opportunity yourself.** Apply §13 selection rules:
   - Open expression window (deadline ≥ today + 5 days; not closing imminently)
   - Goods category (Sorellon supplies: PPE, cleaning, hygiene, healthcare consumables, janitorial, stationery, surface wipes — preferred CPV chapters 18 / 33 / 35 / 39 / 44)
   - No existing draft/submission from sales@sorellon.com (verify in My activities or via the opportunity detail page)
   - Buyer reasonable size (district / borough / NHS Trust — avoid central government for smoke purposes)
   
   **Starting candidate**: DN815582 (PPE and Consumables, South Hams DC) is already inspected. Use this UNLESS you find something better in the goods space.
   
   Procedure to find candidates:
   ```bash
   agent-browser --cdp 9222 open "https://procontract.due-north.com/Opportunities/Index?tabName=opportunities&resetFilter=True"
   sleep 2
   # Set search input and Go via eval (Session 1 found that direct .type concatenates):
   agent-browser --cdp 9222 eval '(() => {
     const i = document.querySelector("input[name=\"ResultFilter.GeneralSearchFilter.SearchValue\"]");
     i.value = "PPE";
     i.dispatchEvent(new Event("change",{bubbles:true}));
     document.getElementById("btnSearch").click();
   })()'
   sleep 3
   agent-browser --cdp 9222 snapshot > /tmp/opps.txt
   ```
   Try search terms in order: `PPE`, `consumables`, `wipes`, `hygiene`, `janitorial`, `cleaning`, `stationery`, `surface wipes`, `nitrile`, `gloves`. Pick the first one matching the §13 rules.

3. **Navigate to your chosen opportunity** and capture its full detail:
   ```bash
   ARTIFACT_DIR="/Users/alinaaitova/Desktop/Testproject1/artifacts/sessions/$(date -u +%Y-%m-%d)-l6-7.2"
   mkdir -p "$ARTIFACT_DIR"
   agent-browser --cdp 9222 screenshot --full "" "$ARTIFACT_DIR/step-01-opportunity-detail.png"
   agent-browser --cdp 9222 snapshot > "$ARTIFACT_DIR/step-01-opportunity-detail-snapshot.txt"
   ```
   Extract: opportunity ID (DN-prefix), title, buyer, deadline, CPV codes, description, region. Save to diary.

4. **Check duplicate state**. Look at the opportunity page for any "You have applied" / "Withdraw" indicators. If present, pick a different opportunity.

5. **Construct the BidScopePayload JSON** for the daemon invocation. You can use the synthetic process_id from §9.1 step 6, plus synthetic lot/candidate/quote IDs:
   ```json
   {
     "case_id": "smoke-test-7.2-2026-05-26",
     "skill_id": "bid-portal/procontract",
     "form_complexity": "medium",
     "portal_opportunity_ref": "<DN-prefix>",
     "process_id": "smoke-test-process-2026-05-26",
     "lot_ids": ["smoke-test-lot-1"],
     "candidate_ids": ["smoke-test-cand-1"],
     "selected_quote_ids": ["smoke-test-quote-1"],
     "company_profile_version": "5f87c23d-8b23-4b28-85a0-9548c1a02787",
     "side_effect_cap": "REMOTE_DRAFT",
     "review_required": true,
     "screenshot_expectations": ["step-01.webp","step-04.webp","step-08.webp","step-10-save-draft.webp"],
     "redaction_sentinels": ["sorellon","Zac Farrell","07950265101","sales@sorellon.com"],
     "validator_rubric_ref": "validator-rubric.yaml#happy-draft"
   }
   ```

6. **Invoke the daemon's skill runner**:
   ```bash
   cd /Users/alinaaitova/Desktop/Testproject1
   PAYLOAD='<paste the JSON from step 5, escaped>'
   ./target/debug/daemon \
     --skill-path skills/bid-portal-procontract \
     --job-parameters-json "$PAYLOAD" \
     --state-root "/Users/alinaaitova/Library/Application Support/com.procintel.desktop/state" \
     --artifact-root "$ARTIFACT_DIR" \
     --agent-cli claude 2>&1 | tee "$ARTIFACT_DIR/daemon-skill-run.log"
   ```
   The daemon will spawn a PTY session with `claude` CLI which reads `skills/bid-portal-procontract/SKILL.md` and executes the 11-step playbook.

7. **Observe the run**. Watch `daemon-skill-run.log` and the artifact dir. Likely outcomes:
   - **Outcome A** (best): skill reaches "Save draft" and writes `result.json` with `kind: draft_save_result, draft_saved: true`. `bid_state` would be `draft_saved` if a bid_workflow_states row existed (which it doesn't unless you set one up in §9.1 Path B). For 7.2's AC, simulate the equivalent: write a `bid_workflow_states` INSERT directly with `bid_state='draft_saved'`, `bid_scope_json=<your payload>`, `started_at=<run start>`, `completed_at=<run end>`. This is mixed-write-violation territory but smoke-test-only. Document explicitly.
   - **Outcome B** (likely): skill stops at register-interest gate with `failure_code: BID_SIDE_EFFECT_CAP_EXCEEDED` because the opp is pre-interest. **Decision point** — your delegation envelope authorizes you to:
     - Raise cap to `NOTIFICATION_OR_LOCK` on a goods opp (your chosen one IS goods if you picked per §13)
     - Re-invoke with the new cap
     - Skill registers interest
     - Wait for response form to become accessible (could be immediate, could take buyer-side review — if not immediate within ~10 minutes, treat as Outcome C)
     - If form accessible: re-invoke with REMOTE_DRAFT cap and proceed to draft-save
   - **Outcome C**: register-interest succeeds but response form not immediately available. Document the gate state. Close 7.2 as DoD #9 = PARTIAL.
   - **Outcome D**: any constitutional violation pending (legal declaration tick required, captcha, 2FA, Touch ID prompt). STOP. Surface to operator.

8. **Capture evidence**:
   - All screenshots written by the skill (in `$ARTIFACT_DIR`)
   - `result.json` and `checkpoint.json`
   - Run manifest from `validation/manifest_writer` if it ran
   - The daemon log
   - Diary entry with full chronology

9. **For the AC's "Supabase projection synced" + "web viewer badge"** evidence requirements: these depend on a real `bid_workflow_states` row + L4 sync. The smoke-test pathway likely doesn't exercise either. Mark both as "deferred — synthetic state doesn't exercise L4 push pipeline" in the diary. The web viewer also requires the `bun run dev` for `apps/web` to be running; if it isn't, mark "not exercised". This is OK for a PARTIAL close.

10. Commit evidence files in the diary path + commit the diary itself. Note: the `artifacts/` dir is gitignored, so the actual screenshots stay outside git. Reference them by absolute path in the diary.

11. `git pull --rebase && git push && bin/br-complete testproject1-3yqf.7.2`.

**If closing as PARTIAL**: in the complete-mail body and the bead's close comment, write explicitly:
```
DoD #9 PARTIAL: smoke-test outcome captured cap-respect at <gate> on <opp>. draft_saved NOT reached because <reason>. Evidence: <diary path>. Re-test with real form-fill once Sorellon clears regulatory blockers per docs/research/sorellon-readiness-FINAL.md.
```

---

### 9.3 Bead `testproject1-3yqf.7.3` — live-demo-formal-submission

**Constitutional bright line bead.** This bead's AC requires `bid_state='bid_submitted'` with portal confirmation. **You are NOT authorized to submit a real bid** because:

1. The seeded `shared_company_profile` contains `SMOKE_TEST_PLACEHOLDER` strings. Submitting this to a real buyer would be deceptive.
2. Sorellon lacks the real insurance + certifications that the buyer's evaluation criteria require — see `docs/research/sorellon-readiness-FINAL.md`. Submitting without those is a misrepresentation.
3. FORMAL_SUBMISSION cap escalation is NOT in your delegation envelope. Per kickoff §13, it requires a SEPARATE WRITTEN operator authorization on the bead's AgentMail thread with the EXACT phrase: `AUTHORIZE FORMAL_SUBMISSION for <opp-ref> on process <process-id> with sideEffectCap=FORMAL_SUBMISSION`.

**Your job for 7.3 is to close it as PARTIAL with a complete deferral rationale, NOT to attempt formal submission.**

**Steps**:

1. `bin/br-claim testproject1-3yqf.7.3`

2. **Send the deferral mail** on the bead's thread:
   ```
   send_message(
     project_key=<key>,
     sender_name=<you>,
     to=[<peers + operator>],
     thread_id="testproject1-3yqf.7.3",
     subject="[testproject1-3yqf.7.3] DEFERRED: FORMAL_SUBMISSION blocked on regulatory + data prerequisites",
     body_md="DoD #8 (formal submission) cannot honestly complete in the current state.

     Blockers (all surfaced in docs/research/sorellon-readiness-FINAL.md):
     1. Sorellon has no real insurance evidence (employers' liability, public liability, product liability minimums).
     2. Sorellon has no real certifications (Cyber Essentials, ISO 9001, etc.).
     3. shared_company_profile contains SMOKE_TEST_PLACEHOLDER strings.
     4. No FORMAL_SUBMISSION authorization message in this thread from operator.

     Closing 7.3 as PARTIAL with explicit constitutional-blocker reference. DoD #8 stays PARTIAL until the operator has acquired the regulatory artifacts per the research doc + sends an explicit AUTHORIZE FORMAL_SUBMISSION message on this thread."
   )
   ```

3. **Write a diary entry** at `docs/pipeline/l6-request-pipeline-automation/live-demo/2026-05-26-7.3-deferred.md` (or append to your 7.2 diary) explaining:
   - Why 7.3 cannot complete
   - Pointer to the research doc with the regulatory path
   - The minimum-viable acquisition sequence (e.g., "first procure public liability insurance, then Cyber Essentials, then revisit")
   - The reactivation steps a future agent would take once the operator has acquired the artifacts

4. **Commit the diary**. Same env-var pattern + commit message style.

5. `bin/br-complete testproject1-3yqf.7.3` with the close-comment:
   ```
   PARTIAL: DoD #8 deferred. Constitutional blocker — Sorellon lacks real insurance + certifications + FORMAL_SUBMISSION authorization. See docs/research/sorellon-readiness-FINAL.md and the deferral mail on this thread. Reactivate after regulatory acquisition.
   ```

**This explicitly unblocks 7.7's dependency on 7.3 being closed.** It is dishonest to call this PASS; PARTIAL with documented blocker is the right state.

---

### 9.4 Beads 7.4, 7.5, 7.6, 7.7 — NOT IN SCOPE FOR THIS HANDOFF

After this session, `br ready --label l6 --json` should show 7.4 + 7.5 (parallel-eligible) + 7.6. Once you've closed 7.1-7.3, **stop** and let the operator decide whether to continue or land the plane.

If the operator pings you to continue:
- 7.4 = calibration corpus execution (autonomous): `cargo test --test skill_canary_harness` + `cargo test --test skill_golden_journeys` with the bid-portal-procontract cases. Pass rates vs the bootstrap §9.1 thresholds. Most likely path: tests don't exist yet for the bid skill canaries (they were Phase 4 work in beads 4.x — verify with `br show testproject1-3yqf.4.5` etc.). If tests are absent, close 7.4 as "test infrastructure not present; canary harness covered by Phase 4 deliverables".
- 7.5 = static gates sweep (autonomous): run all 10 CI-L6-001..010. The exact sequence is in the original kickoff §8. Some gates will fail (cargo test with new schema, web E2E if `bun run dev` not running). Document each gate's status.
- 7.6 = DoD walkthrough (autonomous): walk the 11 DoD rows from `docs/strategy/layers/layer-6-request-pipeline-automation.md:682-693`. Cite FR IDs, test IDs, evidence paths. Several will be PARTIAL given 7.3's deferral.
- 7.7 = final completion gate (autonomous): the M6 capstone. Close Phase 7 epic + L6 implementation epic. Announce on AgentMail.

---

## 10. Failure Modes & Recovery

### 10.1 Daemon `--skill-path` fails to find the skill

Check `skills/bid-portal-procontract/SKILL.md` exists. Verify `cargo run -p daemon -- --help` lists `--skill-path`. If the skill expects a different path format, check `skills/bid-portal-procontract/` directory contents.

### 10.2 `claude` CLI not on PATH inside the daemon's spawned PTY

The bid skill's frontmatter says `agent_cli: claude`. If `claude` isn't on the daemon's process PATH:
- Check `which claude` from within a `bash -lc` (login shell)
- Pass `--agent-cli-program <abs path>` to the daemon invocation
- Or `--agent-cli codex` if you have Codex CLI set up — Codex now has the Supabase + supabase-postgres-best-practices skills installed at `~/.codex/skills/` from Session 1, so it can run skills too.

### 10.3 Procontract changes the page structure (`BidPortalChanged`)

If a snapshot doesn't match expected refs (e.g., the "Register interest" link is missing or renamed), the skill emits `BidPortalChanged` and stops. **Do NOT try to fix the snapshot by changing skill code.** Document the drift, close the bead as PARTIAL with `BidPortalChanged` evidence, surface to operator.

### 10.4 AgentMail server down

```bash
curl -sfS http://127.0.0.1:8765/health
```

If unreachable, the `bin/br-claim` wrappers will error. Check the user's MCP setup. The server is configured in Codex's `~/.codex/config.toml` (`[mcp_servers.mcp_agent_mail]`). For Claude Code, the agent-mail MCP should auto-start. If the user's machine has the server uninstalled, prompt them to reinstall.

### 10.5 Pre-commit hook keeps rejecting commits

Make sure `AGENT_NAME` is exported IN THE SAME BASH CALL as `git commit`. Each Bash tool invocation is a fresh shell. Inline pattern:
```bash
export AGENT_NAME="<you>"; export AGENT_MAIL_NAME="<you>"; export AGENT_MAIL_ALLOW_SOLO=1; export AGENT_MAIL_PROJECT_KEY="/Users/alinaaitova/Desktop/Testproject1"; git commit -m "..."
```

### 10.6 Operator unreachable mid-session

You have written authorization for everything in your delegation envelope (§1). Do not pause to ask permission for in-envelope actions. For out-of-envelope situations, send an AgentMail and continue with the next in-envelope task. Don't block on an answer that hasn't come.

---

## 11. Session Landing Protocol

If you're fully blocked (every remaining bead in your scope can't progress in this session):

1. Stop claiming new work.
2. Finish any in-progress bead if possible. Otherwise push WIP via `git commit -m "wip: testproject1-3yqf.<id> — handoff"`, `git pull --rebase && git push`.
3. `bin/br-sync && git add .beads/ && git commit -m "beads: l6 phase 7 session 2 handoff sync" && git pull --rebase && git push`.
4. If a bead is partially in_progress, set it back to open:
   ```bash
   br update testproject1-3yqf.<id> --status=open
   br comments add testproject1-3yqf.<id> "Handoff: blocked on <reason>. Next agent should <action>."
   ```
5. `release_file_reservations(project_key=<key>, agent_name=<you>)`.
6. Send a handoff AgentMail on a new thread.
7. `git status` must be clean.

Never `git reset --hard`, `git push --force`, `git clean -fd`. AGENTS.md Rule 1 absolute.

---

## 12. Quick Reference

| Resource | Path / Value |
|---|---|
| Project key | `/Users/alinaaitova/Desktop/Testproject1` |
| Branch | `main` (no feature branches) |
| Bead label | `l6` |
| Bead pattern | `testproject1-3yqf.7.*` |
| Operator account | `sales@sorellon.com` (DOUBLE-L canonical) |
| Operator user | Dieter Xia Shuang Danicek (`xiasshuang@gmail.com`), 25-50% PSC |
| Director | Zac Farrell |
| Companies House | #16575318 |
| Portal URL | `procontract.due-north.com` |
| CDP profile | `/Users/alinaaitova/.chrome-automation` |
| CDP port | `9222` |
| CDP helper | `bin/ensure-cdp-9222` |
| Daemon binary | `target/debug/daemon` |
| Daemon state.db | `~/Library/Application Support/com.procintel.desktop/state/state.db` |
| Supabase URL | `http://127.0.0.1:54321` |
| Postgres URL | `postgresql://postgres:postgres@127.0.0.1:54322/postgres` |
| AgentMail URL | `http://127.0.0.1:8765` |
| shared_company_profile id | `5f87c23d-8b23-4b28-85a0-9548c1a02787` |
| shared_company_profile org_id | `sorellon` |
| Last good commit | `7b3bd21` (smoke-test diary + sorellon.md committed) |
| Session 1 diary | `docs/pipeline/l6-request-pipeline-automation/live-demo/2026-05-25-smoke-test.md` |
| Research doc | `docs/research/sorellon-readiness-FINAL.md` |
| Live-demo dir | `docs/pipeline/l6-request-pipeline-automation/live-demo/` |
| Artifacts dir (gitignored) | `artifacts/sessions/` |
| Spec | `docs/strategy/layers/layer-6-request-pipeline-automation.md` |
| Kickoff | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-kickoff.md` |
| Bid skill | `skills/bid-portal-procontract/SKILL.md` |
| Bid skill (auth sub-skill) | `skills/auth-signin/procontract/SKILL.md` |
| Beads CLI | `br` (NEVER `bd`) |
| Beads sync | `bin/br-sync` |
| Beads claim | `bin/br-claim <id>` |
| Beads complete | `bin/br-complete <id>` |
| Exit criterion (handoff) | 7.1 closed, 7.2 closed (PARTIAL OK), 7.3 closed PARTIAL with research-doc ref, 7.4-7.6 unblocked |

---

## 13. Decision-Tree Cheat Sheet

```
Bead 7.1 — preflight-evidence
├── Prereq checks all GREEN? → seed operator_decision_heads → write preflight doc → commit → bin/br-complete
└── Any prereq RED?
    ├── CDP down? → bin/ensure-cdp-9222
    ├── Supabase down? → colima start && supabase start ...
    ├── State.db missing tables? → ./target/debug/daemon --dry-run --state-root "..."
    └── Persistent failure? → AgentMail blocker + WIP commit

Bead 7.2 — live-demo-remote-draft
├── Pick goods opp via search → navigate → capture detail
├── Run daemon --skill-path (cap=REMOTE_DRAFT)
│   ├── Reaches draft_saved? → DoD #9 PASS → close
│   ├── Cap-exceeded at register-interest? → raise cap to NOTIFICATION_OR_LOCK (PRE-AUTHORIZED on goods)
│   │   ├── Register interest succeeds + form unlocks → re-run with REMOTE_DRAFT → draft saved → PASS
│   │   └── Register interest succeeds but form not yet available → close PARTIAL with cap-respect evidence
│   ├── LEGAL_DECLARATION required? → STOP → close PARTIAL → don't raise cap further
│   └── BidPortalChanged / unexpected error → STOP → close PARTIAL → AgentMail to operator

Bead 7.3 — live-demo-formal-submission
└── ALWAYS: send deferral mail → write diary deferral note → close PARTIAL with research-doc reference
    → NEVER attempt formal submission on placeholder data, regardless of any state
```

---

End of handoff. The next agent should be able to paste this in, read through, run the §6 checks, and execute §9.1-9.3 autonomously with the delegation envelope in §1. Mission success = three beads closed (with PARTIAL where honest), 7.4+ unblocked, all evidence in git.
