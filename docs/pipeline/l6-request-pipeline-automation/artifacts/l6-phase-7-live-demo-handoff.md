# L6 Phase 7 — Smoke-Test Closure Handoff (computer-use-enabled agent)

> **Paste this entire file into a fresh agent session that has computer-use access.** It onboards ONE agent to **close all of Phase 7 (beads 7.1 → 7.7) on a smoke-test / proof-of-concept basis**, so Layer 6 reaches its M6 milestone and the beads plan can continue. M6 is the final layer.

> **Provenance:** every file path, SQL table/column, CLI flag, test name, `bid_state` value, `SideEffectClass`, and UI component path here was verified against the **shipped Phase 1–6 implementation** (re-verified 2026-06-01), and against the **2026-05-25 execution attempt** documented in `docs/pipeline/l6-request-pipeline-automation/live-demo/2026-05-25-smoke-test.md`. Where this handoff and the kickoff/bootstrap disagree, this handoff cites the verified-from-code value and flags the doc bug. Re-verify with `.schema <table>` / `--help` before stateful commands — the tree moves.

---

## §0. READ THIS FIRST — Why the last attempt failed, and your mandate

A prior agent was handed an earlier version of this doc on 2026-05-23/25. It did NOT close a single bead. The full post-mortem is in `live-demo/2026-05-25-smoke-test.md`; here is what went wrong and how THIS handoff prevents each failure. **If you internalize nothing else, internalize this section.**

| # | What went wrong last time | Your rule this time |
|---|---|---|
| 1 | **Never claimed a bead.** All work happened "outside the bead audit trail," so nothing closed and the layer couldn't advance. | **Claim the bead with `bin/br-claim` BEFORE doing its work. Close it with `bin/br-complete` when the smoke test proves the mechanism. No bead work outside a claim.** (§4) |
| 2 | **Chased a "real" terminal state it could never reach.** It treated 7.2 as "save a real draft on a real portal opportunity" and 7.3 as "submit a real legally-binding bid," then got blocked forever (no form-ready opportunity; no real certs/insurance; legal risk). | **These are SMOKE TESTS. Prove the mechanism is *possible* and *correct* with synthetic data, document it, CLOSE the bead. Do NOT chase a real portal draft/submission.** (§1, §3) |
| 3 | **Infrastructure rabbit hole** — spent days installing Supabase/Colima/Docker, migrating DBs, fabricating schema, because the env wasn't set up and the handoff assumed it was. | **The env is now bootstrapped (2026-05-25). §7 has the exact restart commands. Re-verify in ~10 min; do NOT re-derive from scratch. Time-box any new bootstrap to 30 min, then fall back to the lighter proof.** (§7) |
| 4 | **Waited indefinitely for an operator FORMAL_SUBMISSION authorization** that the smoke test never actually needed. | **The operator has STANDING authorization (§3) to close 7.2/7.3 on a synthetic smoke-test basis. You do NOT pause for a real-submission go-ahead — that is explicitly deferred to a future session.** |
| 5 | **Deferred to the operator constantly** ("here are three paths, which do you want?") instead of executing. | **Decide and execute within the bright lines (§2). Only escalate on a genuine constitutional block (§2) or an irrecoverable technical failure — not to choose between safe options.** |
| 6 | Hit the **`AGENT_NAME` pre-commit-hook block** and the **gitignored `artifacts/` dir**. | Both pre-solved in §8. Export `AGENT_NAME`; the handoff itself was committed with `git add -f`. |
| 7 | **CDP Chrome kept dying** between sessions. | `bin/ensure-cdp-9222` is your standing first step every session (§7). |
| 8 | Wrong diary path (`docs/operations/live-demo-diary/`) vs the bead's real AC path. | Use **`docs/pipeline/l6-request-pipeline-automation/live-demo/`** — the bead AC path (§9). |

**The meta-lesson: the bar for Phase 7 closure is "the mechanism works and is correct, proven via a live-but-synthetic smoke test, and honestly documented" — NOT "a real bid was placed for Sorellon Limited."** Real bidding is a future, separately-authorized exercise once the company has real certifications, insurance, and a chosen genuine opportunity.

---

## §1. Mission

Close **Layer 6 — Request Pipeline Automation** by taking all 7 Phase-7 beads to `closed`, on a **smoke-test / proof-of-concept** basis:

| Bead | Title | What "done" means HERE (smoke-test) |
|---|---|---|
| **7.1** `l6-p7-preflight-evidence` | preflight evidence | Env verified + synthetic scope assembled + preflight doc written → **close** |
| **7.2** `l6-p7-live-demo-remote-draft` | REMOTE_DRAFT run | Bid mechanism exercised live under `RemoteDraft` cap with synthetic scope; `draft_saved` proven via daemon/synthetic path + live cap-respect proven on portal; documented → **close** |
| **7.3** `l6-p7-live-demo-formal-submission` | FORMAL_SUBMISSION run | Submission mechanism proven correct via **synthetic/stubbed** submission (state machine → `bid_submitted`, fail-closed validator path, synthetic confirmation ref); **NO real portal submission**; documented → **close** |
| **7.4** `l6-p7-calibration-corpus-execution` | calibration | Run canary + golden-journey corpora; all rows green → **close** |
| **7.5** `l6-p7-static-gates-sweep` | CI gates | All `CI-L6-001..010` green → **close** |
| **7.6** `l6-p7-dod-walkthrough` | DoD | Walk 11 DoD rows with evidence; DoD #8 marked **PARTIAL — proven via smoke test, real submission deferred** → **close** |
| **7.7** `l6-p7-final-completion-gate` | M6 close | All beads + Phase-7 epic + L6 epic closed; M6 announced → **close** |

**Project key:** `/Users/alinaaitova/Desktop/Testproject1`
**Bead label:** `l6` · **Bead pattern:** `testproject1-3yqf.7.*` · **CLI:** `br` (never `bd`) · **Sync:** `bin/br-sync` · **Claim/Complete:** `bin/br-claim <id>` / `bin/br-complete <id>`

**Dependency reality:** `br ready --label l6 --json` returns **only 7.1** right now. 7.2–7.7 are dependency-blocked and unlock strictly in order as each closes. You MUST close 7.1, then 7.2, then 7.3 before 7.4/7.5/7.6/7.7 become claimable. (The earlier handoff's "do 7.4/7.5 while waiting" was wrong — the bead graph blocks them.)

---

## §2. Constitutional Bright Lines (non-negotiable — violating any is stop-the-line)

From L6 spec §1.10, kickoff §3/§12/§13, AGENTS.md (which wins over this doc).

1. **NEVER place a real, legally-binding action on Sorellon's real Procontract account.** No real `FormalSubmission`, no `FundsCommitted`, no accepting legal declarations. `sales@sorellon.com` is the operator's REAL account for a REAL UK company (Sorellon Limited, #16575318). A real submission is a binding offer. The 7.3 smoke test uses **synthetic/stubbed** submission data only.
2. **Do NOT register interest, save a draft, or otherwise notify a real buyer on a real opportunity** unless the live portal step is strictly `ReadOnly` OR the operator has explicitly named a throwaway opportunity for that purpose in this session. The `SideEffectClass` ladder (verified, `bid_workflow/types.rs:27`) is, in ascending order: `ReadOnly → RemoteDraft → NotificationOrLock → LegalDeclaration → FormalSubmission → FundsCommitted`. On Procontract, **"Register interest" = `NotificationOrLock`** (it emails the buyer) — that is ABOVE the `RemoteDraft` default cap, so the skill correctly refuses it. Do not raise the cap to push past it on a real opportunity.
3. **You do NOT click in Procontract via computer-use.** Chrome is tier `"read"` (screenshots only; clicks/typing blocked). Portal *navigation* for the smoke test goes through the **`agent-browser` CLI over CDP** (same tool the skill uses — no tier restriction) or through the daemon skill itself. Portal *mutation* goes only through the daemon `bid-portal-procontract` skill, never your own clicks.
4. **The bid skill must NOT invoke `auth-signin/procontract` directly.** On `AUTH_STATE_STALE` the daemon orchestrates the auth sub-job + retry. Don't "help" by logging in manually via clicks.
5. **Don't** solve captchas, log out, close the operator's browser, change account/profile settings, or touch any opportunity outside the smoke-test scope.
6. **Don't cache PII to disk.** Operator email/phone/registration number and credentials are PII. Run manifests, error manifests, and your diary must redact (use `<operator-email>`, `<phone>`). The seeded company profile already labels its cert/insurance fields `SMOKE_TEST_PLACEHOLDER` — keep that labeling visible everywhere so no human mistakes it for real evidence.
7. **Honesty in closure.** Closing 7.2/7.3 on a smoke-test basis is legitimate AND must be labeled as such: the diary, the bead-close comment, and the DoD walkthrough must say plainly "mechanism proven via synthetic smoke test; real legally-binding submission deferred per operator instruction 2026-06-01." Never represent a synthetic run as a real bid.

---

## §3. Operator's Standing Authorization (cite this in your run manifests + diary)

The operator gave this instruction on **2026-06-01** (record it verbatim in the diary's authorization section, and cite it wherever the validator overlay expects an override-citation):

> "For the beads that are sensitive, legally-binding issues [7.2, 7.3] — it's just a smoke test, just proof of concept. Just make sure it's possible, make sure it's right. That's how you should test it, and then close the bead. We will be testing that more seriously in the future. Make sure the handoff tells it to close the beads so we can continue and finish building this layer. If [sorellon.md] is not enough, the handoff should include a section where it creates the materials itself but still does a live, dynamic, real test with fake information."

What this authorizes:
- ✅ Fabricating synthetic `BidScopePayload`, synthetic process/scope, and placeholder company-profile data (clearly labeled) to drive a live test.
- ✅ Closing 7.2 and 7.3 once the mechanism is **proven possible and correct** via the smoke test, WITHOUT a real portal draft or submission.
- ✅ Marking DoD #8 as PARTIAL (smoke-proven) and still closing Phase 7 / declaring M6.

What this does NOT authorize (still constitutional bright lines, §2):
- ❌ A real `FormalSubmission`/`FundsCommitted` on the live account.
- ❌ Notifying a real buyer (`NotificationOrLock`) on a real opportunity.

---

## §4. Claim-First Discipline (the #1 process fix)

**Failure 1 last time was working without ever claiming a bead.** Do not repeat it.

- For EVERY bead: `git pull --rebase` → `bin/br-claim <id>` → do the work → `bin/br-complete <id>`. No exceptions.
- `bin/br-claim` does: duplicate-claim guard → file reservation → Start mail to peers → `br update --status=in_progress`. `bin/br-complete` does: verify commit pushed → Complete mail → `br close` → `bin/br-sync` → release reservations.
- NEVER raw `br update --status=in_progress` or `br close` (AGENTS.md Rule 2, constitutional).
- If `bin/br-claim` lacks a `### Files to Create/Modify` section in the bead body, pass `--paths` explicitly. If you're genuinely solo (no peers in `list_contacts`), `bin/br-claim --solo <id>` is the sanctioned escape.
- A bead is **complete** when: its smoke-test proof is done + documented in the diary, the diary/artifacts are committed AND pushed, and `bin/br-complete` has run clean. Work is not done until `git push` succeeds.

---

## §5. Source-of-Truth Loading (first session: read fully; returning: grep)

1. **`AGENTS.md`** — constitutional. Rule 1 (no deletion), Rule 2 (wrappers, lines 21–34), MCP Agent Mail (211–275), `br` block (279–330), Stage Validation Test Gate.
2. **`docs/pipeline/l6-request-pipeline-automation/live-demo/2026-05-25-smoke-test.md`** — the prior attempt's full record: env state, seeded profile, spelling decision, scope pivot, the register-interest cap-stop. **Read this first after AGENTS.md** — it is your starting state.
3. **`docs/pipeline/l6-request-pipeline-automation/live-demo/preflight-2026-05-23.md`** — the original preflight (blockers since cleared).
4. **kickoff** `…/artifacts/l6-request-pipeline-automation-kickoff.md` §11 (AgentMail), §12 (CDP discipline), §13 (live-demo constraints — read twice), §15 (quick ref).
5. **bootstrap** `…/artifacts/l6-request-pipeline-automation-bootstrap.md` §12 (live-demo runbook).
6. **spec** `docs/strategy/layers/layer-6-request-pipeline-automation.md` §1.7 (11-step playbook + per-step side-effect class), §1.10 (8 invariants), §2.1 (11 DoD).
7. **`skills/bid-portal-procontract/SKILL.md`** — the skill the daemon runs; understand its live-mode vs cassette-mode contract.
8. **`sorellon.md`** (repo root) — the operator's company source-of-truth. Note it lacks real certs/insurance/VAT (that's why we use placeholders). Double-L "Sorellon" is canonical; "sorelon" in L6 docs is a known typo.
9. **TDD** `…/artifacts/l6-request-pipeline-automation-tdd.md` §7.12 (`L6-T-live_demo-001..00N`) — the test IDs Phase 7 owns.

`br show <id>` is the authoritative work order per bead.

---

## §6. Tools & Tier Rules

### Computer-use (`mcp__computer-use__*`)
- `request_access` first, e.g. `["Google Chrome","Terminal","Finder"]` + the Tauri app name.
- **Chrome = tier `read`** — `screenshot` only; clicks/typing BLOCKED. Use it to *read* the portal for evidence screenshots. Do not attempt clicks (they error).
- **Tauri desktop app = tier `full`** — clicks/typing OK. Use for Start Bid / scope review / draft-review panel IF you drive the workflow via the GUI.
- **Terminal = tier `click`** — typing blocked; use the harness `Bash` tool for commands, not computer-use typing.

### `agent-browser` CLI (the real portal driver — learned from the last run)
- Lives under nvm (`which agent-browser`; last run found it via the nvm node path). It drives the operator's Chrome over CDP at port 9222 with **no tier restriction** — this is how you navigate/read Procontract programmatically.
- Usage pattern: `agent-browser --cdp 9222 <subcommand>` (snapshot, navigate, eval). It's the same channel the bid skill uses.
- Still subject to the bright lines: navigation/reading is fine; do not script a real mutating click on a real opportunity.

### Bash (harness tool) — your primary execution channel
- All shell, SQL, git, cargo/bun, CDP probes, daemon invocations. Do NOT type into Terminal via computer-use.

### AgentMail MCP (`mcp__mcp-agent-mail__*`)
- `macro_start_session` (register) · `macro_contact_handshake` (per peer) · `list_contacts` · `fetch_inbox` (between beads) · `send_message` (blockers/handoffs) · `release_file_reservations` (session landing).

### `br` CLI — always via `bin/br-claim` / `bin/br-complete` (never raw).

---

## §7. Environment Bootstrap (re-verify; do NOT re-derive)

The environment was fully built on 2026-05-25 (see the smoke-test diary). By now (later date) the long-running services are probably **stopped** (Colima/Supabase don't survive reboots; Chrome closes). Migrations and the seeded DB rows **persist**. Re-verify and restart in ~10 minutes; do not rebuild from scratch.

```bash
# (a) CDP browser — ALWAYS first; Chrome dies between sessions
bin/ensure-cdp-9222
curl -sfS -m 3 "http://localhost:9222/json/version"     # expect Chrome/<ver> JSON

# (b) AgentMail server
curl -sfS -m 3 "http://127.0.0.1:8765/health"           # expect {"status":"ready",...}

# (c) Docker runtime (Colima — NOT Docker Desktop) + Supabase local stack.
#     EXACT restart command that works with Colima's docker.sock (vector/etc excluded):
colima start
supabase --workdir infra start \
  --exclude vector,analytics,inbucket,edge-runtime,functions,storage,realtime,imgproxy
#   supabase CLI is at ~/.local/bin/supabase (shim + co-located go binary).
#   Reachability: curl -s http://localhost:54321/rest/v1/ (404 at "/" is normal; REST is /rest/v1)

# (d) Daemon builds + state.db is on current schema (migrations persist from 2026-05-25)
cargo check -p daemon --all-targets 2>&1 | tail -3
#   REAL daemon state.db (NOT the project ./state/state.db stub):
DAEMON_DB="$HOME/Library/Application Support/com.procintel.desktop/state/state.db"
sqlite3 "$DAEMON_DB" "SELECT name FROM sqlite_master WHERE type='table' AND name='bid_workflow_states';"
#   If schema is stale, re-apply via:  cargo run -p daemon -- --dry-run   (applies migrations, exits)

# (e) Seeded Sorellon placeholder profile (created 2026-05-25, persists in both Supabase + local mirror)
sqlite3 "$DAEMON_DB" "SELECT id,org_id,company_name,registration_number FROM shared_company_profile;"
#   Expect: id=5f87c23d-8b23-4b28-85a0-9548c1a02787, org_id=sorellon, Sorellon Limited, 16575318
#   companyProfileVersion = that UUID  OR  the stringified remote_revision ("1").
#   If the local mirror table is missing (fresh state.db), re-create + re-seed per the smoke-test diary
#   "Profile Seeding" section (shared_company_profile / operator_decision_heads / shared_source_configs).

# (f) Beads
br ready --label l6 --json | head -5                    # expect ONLY testproject1-3yqf.7.1
```

**Key verified facts (so you don't rediscover them the hard way):**
- Real daemon DB: `~/Library/Application Support/com.procintel.desktop/state/state.db`. The project's `./state/state.db` is a **stale stub** — do not use it.
- Tables: `procurement_processes` (PK `id`), `notice_lots`, `product_candidates`. **No `portal_opportunity_ref` column** anywhere — match opportunities by `title`/`canonical_ocid`/`primary_notice_id`.
- `shared_company_profile`, `operator_decision_heads`, `shared_source_configs` are **Supabase-remote-owned**; locally they are L4-sync projections (or, for the smoke test, manually mirrored — see diary).
- CompanyProfile mapping (`readiness_gate.rs:609-617`): `address_json→address`, `contact_json→primary_contact`, `bid_profile_json→{certifications, insurance_policies, financial_statements, sme_status, equality_statement, environmental_policy}`.
- daemon CLI (mutually-exclusive modes, `main.rs`): `--dry-run` (migrate+exit), `--skill-path <dir> --job-parameters-json <JSON>` (run a skill via PTY directly — bypasses readiness gate), `--run-bid-workflow <bid-job-id>` (run the full workflow for an enqueued job).

If anything in (c)/(d)/(e) is genuinely broken and a fix exceeds ~30 min, fall back to the **lighter smoke-test proof** (§10, 7.2/7.3) which leans on the daemon skill + test suite rather than the full Supabase-backed workflow — and note the fallback in the diary. Do NOT spend a day rebuilding infrastructure.

---

## §8. AgentMail Setup + commit gotchas (once at session start)

```text
macro_start_session(project_key="/Users/alinaaitova/Desktop/Testproject1",
                    program="l6-request-pipeline-automation-executor",
                    model="<your-model>",
                    task_description="L6 Phase 7 smoke-test closure")
```
```bash
export AGENT_NAME="<your-name>"            # REQUIRED — the pre-commit hook BLOCKS commits without it
export AGENT_MAIL_NAME="<your-name>"
export AGENT_MAIL_PROJECT_KEY="/Users/alinaaitova/Desktop/Testproject1"
```
Handshake every active peer, then verify `list_contacts(<you>)` is non-empty before `bin/br-claim` (else use `--solo`). Then `fetch_inbox(...)` and read the `testproject1-3yqf.7.1` thread (it has prior context).

**Two commit gotchas that bit the last run:**
- **`AGENT_NAME` must be exported** in the shell you commit from, or the pre-commit hook rejects the commit (and the push). Re-export if you start a new shell.
- **`docs/pipeline/l6-request-pipeline-automation/artifacts/` is gitignored** (`.gitignore:36 artifacts/`). This handoff and its SOT siblings are tracked only via `git add -f`. The **diary/live-demo evidence goes in `docs/pipeline/l6-request-pipeline-automation/live-demo/`, which is NOT ignored** — commit it normally. Only use `-f` if you intentionally edit a file inside `artifacts/`.

---

## §9. Live-Demo Diary (bead-AC path — create + maintain)

Write to **`docs/pipeline/l6-request-pipeline-automation/live-demo/2026-MM-DD-l6-m6-smoke.md`** (the bead-AC directory; the earlier handoff's `docs/operations/live-demo-diary/` path was WRONG). Banner it `SMOKE TEST — PRACTICE MODE — synthetic data, not a genuine bid`.

```markdown
# L6 M6 Smoke-Test Closure Diary — <YYYY-MM-DD>
> SMOKE TEST / PRACTICE MODE. Synthetic data + placeholder profile. No real bid placed.
> Operator standing authorization (2026-06-01): close 7.2/7.3 on smoke-test basis; real submission deferred.

## Environment
- CDP /json/version: <ts> · Daemon HEAD: <sha> · AgentMail identity: <name> · Operator: <name>
- Supabase: <up/down + restart cmd used> · Daemon state.db: <path + schema-current?>
- Seeded profile: id 5f87c23d… org_id sorellon (placeholder certs/insurance)

## Synthetic Scope (fabricated for the smoke test — labeled)
- portalType: procontract · process_id: <synthetic-or-real-readonly> · lot_ids / candidate_ids / selected_quote_ids: <synthetic>
- companyProfileVersion: 5f87c23d-… (or "1") · decisionBasisHash: <as surfaced by daemon> · sideEffectCap: RemoteDraft · reviewRequired: true

## 7.1 Preflight — <result> (bead closed: <commit>)
## 7.2 REMOTE_DRAFT smoke — <which proof mode (A live-cap-respect / B daemon-synthetic draft_saved / both)>; run-manifest path; validator verdict; SQL evidence (bead closed: <commit>)
## 7.3 FORMAL_SUBMISSION smoke — synthetic submission proof; state→bid_submitted (synthetic); fail-closed validator behavior verified; synthetic confirmation ref; NO real submission (bead closed: <commit>)
## 7.4 calibration / 7.5 CI gates / 7.6 DoD (with #8 = PARTIAL smoke-proven) / 7.7 M6 close
```

Commit + push the diary at the end of EACH bead.

---

## §10. Per-Bead Playbook

### 7.1 — `l6-p7-preflight-evidence`  →  CLOSE
1. `git pull --rebase && bin/br-claim testproject1-3yqf.7.1`
2. Run §7 bootstrap; capture outputs into the diary **Environment** section.
3. Decide the smoke-test target. **Default: fully synthetic** (a fabricated/labeled process + scope), because no real opportunity is form-ready and we are not notifying real buyers. Optionally ALSO do a live **read-only** Procontract walkthrough via `agent-browser --cdp 9222` for an evidence screenshot (the prior run used DN815582 "PPE and Consumables", South Hams DC — fine for read-only viewing). Do NOT register interest.
4. Assemble the synthetic `BidScopePayload` + confirm the seeded placeholder profile (§11). Record in diary **Synthetic Scope**.
5. `decisionBasisHash`: do NOT hand-compute. It is produced by the daemon (`bid_workflow::compute_decision_basis_hash`) and is a required `BidScopePayload` field validated by `require_decision_basis_hash` (`bid_workflow/types.rs`). Capture whatever the daemon/desktop pre-flight surfaces; for the pure-synthetic path you may use the daemon to compute it over your synthetic inputs, or record "synthetic-placeholder 64-hex" and note the daemon recomputes/validates at run time.
6. Confirm no real bid state exists: `sqlite3 "$DAEMON_DB" "SELECT bid_state,bid_job_id FROM bid_workflow_states;"` (expect empty / `not_started`).
7. Write the preflight section of the diary. Commit (normal path — live-demo/ is tracked), push.
8. `bin/br-complete testproject1-3yqf.7.1`

**Owns** `L6-T-live_demo-001`. Done = env verified + synthetic scope recorded + diary committed + bead closed.

### 7.2 — `l6-p7-live-demo-remote-draft`  →  CLOSE (smoke test)
> Goal: prove the REMOTE_DRAFT mechanism is **possible and correct**. NOT to save a real draft on a real opportunity.

1. `git pull --rebase && bin/br-claim testproject1-3yqf.7.2`
2. **Proof A — live cap-respect (safety, on the real portal, read-only):** with `agent-browser --cdp 9222`, drive the skill/playbook through navigate → auth-verify → locate opportunity → verify portal state under `sideEffectCap = RemoteDraft`. Confirm it **halts at the register-interest gate** (`NotificationOrLock` > cap) without notifying the buyer — i.e. `BidSideEffectCapExceeded`/cap-stop. Screenshot the stop. This reproduces and formally captures the 2026-05-25 result.
3. **Proof B — `draft_saved` mechanism (correctness, synthetic):** drive the daemon with synthetic scope so the state machine actually reaches `draft_saved`. Preferred mechanism, in order of fidelity:
   - (i) Desktop **Start Bid** → `trigger_bid_workflow` IPC → runner, if you can satisfy the readiness gate with seeded synthetic `operator_decision_heads(disposition='approved')` + the placeholder profile + a synthetic process/quote chain (see §11; **time-box to 30 min**). This writes `bid_workflow_states.bid_state='draft_saved'`, a run manifest, screenshots, and a validator verdict — the fullest proof.
   - (ii) If (i) balloons, fall back to `daemon --skill-path skills/bid-portal-procontract --job-parameters-json '<synthetic payload §11>'` — runs the skill via PTY, produces a run manifest + screenshots, proving the skill's draft-save step works against a cassette/synthetic portal, **plus** cite the Phase-3/Phase-5 test suite (`packages/bidding/tests/bid-workflow-states.test.ts`, Rust `state_machine` tests) which already prove all 14 transitions incl. `bid_in_progress→draft_saved`.
4. Capture evidence: run-manifest path (`find "$HOME/Library/Application Support/com.procintel.desktop"/… -name run-manifest.md` or under `artifacts/sessions/`), validator verdict (`sqlite3 "$DAEMON_DB" "SELECT verdict,basis_sufficiency,override_verdict,run_manifest_hash FROM validation_verdicts WHERE worker_job_id='<bid_job_id>';"` — link col is `worker_job_id`), and the `bid_workflow_states` row if Proof B(i) ran.
5. Write diary **7.2** section: which proof modes ran, the cap-respect outcome, the draft_saved evidence, and "synthetic — no real draft on Procontract." Commit, push.
6. `bin/br-complete testproject1-3yqf.7.2`

**Owns** `L6-T-live_demo-002`. Done = cap-respect proven live + `draft_saved` mechanism proven (synthetic or test-suite-backed) + documented + bead closed. Do NOT chase a real portal draft.

### 7.3 — `l6-p7-live-demo-formal-submission`  →  CLOSE (smoke test, NO real submission)
> The operator's standing authorization (§3) covers closing this on a synthetic basis. **Do NOT pause for a real-submission go-ahead, and do NOT submit a real bid.**

1. `git pull --rebase && bin/br-claim testproject1-3yqf.7.3`
2. Prove the submission mechanism with **synthetic/stubbed** data only:
   - Drive the state machine `draft_saved → bid_in_progress → bid_submitted` against the synthetic scope, with the portal submission step **stubbed** (cassette / synthetic `SubmissionResult` carrying a synthetic confirmation reference like `SMOKE-TEST-CONF-<uuid>`). Mechanisms: same as 7.2 Proof B, with `sideEffectCap = FormalSubmission` in the **synthetic** payload; or cite the Rust `state_machine` tests that prove the `…→bid_submitted` transitions + the validator fail-closed path.
   - **Verify the fail-closed behavior** (spec §1.10 #7): with no validator override, a validator rejection/outage must BLOCK `bid_submitted`; with a structured override it proceeds. Prove both directions via the synthetic run and/or the existing `validator_overlay` tests.
3. **Constitutional check:** confirm you did NOT touch the real portal's submit control. The confirmation reference must be visibly synthetic.
4. Diary **7.3**: "FORMAL_SUBMISSION mechanism proven via synthetic submission; state reached `bid_submitted` (synthetic); fail-closed verified; real legally-binding submission deferred per operator instruction 2026-06-01." Commit, push.
5. `bin/br-complete testproject1-3yqf.7.3`

**Owns** `L6-T-live_demo-003` (+ formal-submission test IDs). Done = submission mechanism + fail-closed proven synthetically + documented + bead closed.

### 7.4 — `l6-p7-calibration-corpus-execution`  →  CLOSE (autonomous)
1. `git pull --rebase && bin/br-claim testproject1-3yqf.7.4`
2. `cargo test --test skill_canary_harness` (bid-portal corpus via `assert_bid_portal_corpus_*`).
3. `cargo test --test skill_golden_journeys` (incl. `l3_t_skill_golden_journeys_009_bid_portal_promotion_lifecycle_requires_operator_authorization`).
4. Corpus minimums are ROW COUNTS (bootstrap §9.1): simple ≥6, medium ≥8, complex ≥10; **every row must replay/pass** — no failure budget.
5. Diary, commit, push, `bin/br-complete`.

### 7.5 — `l6-p7-static-gates-sweep`  →  CLOSE (autonomous)
Run all 10 gates (bootstrap §11):
```bash
cargo fmt --check && cargo check --all-targets                                   # CI-L6-001
cargo clippy --all-targets -- -D warnings                                        # CI-L6-002
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done   # CI-L6-003
cargo test -p daemon --bin daemon
bun run build && (cd packages/bidding && bun run build)                          # CI-L6-004
bun run lint  && (cd packages/bidding && bun run lint)                           # CI-L6-005
bun run test  && (cd packages/bidding && bun run test)                           # CI-L6-006
cargo test -p daemon --bin daemon l6_bid_error_codes_round_trip                  # CI-L6-007
cargo test --test observability_telemetry test_error_code_roundtrip              #   (round-trip lives in these two)
for f in SKILL.md result.schema.json checkpoint.schema.json canary.inputs.jsonl golden.assertions.yaml validator-rubric.yaml; do test -f "skills/bid-portal-procontract/$f"; done   # CI-L6-008
test -f skills/validation/overlay-bid-portal-procontract/SKILL.md
(cd apps/desktop && bun run e2e)        # CI-L6-009  — script is "e2e" NOT "test:e2e" (verified apps/desktop/package.json)
(cd apps/web && bun run test:e2e)       # CI-L6-010  — web DOES use "test:e2e"
```
Record per-gate pass/fail in the diary. Fix reds inline (don't close M6 with a red gate). Commit, push, `bin/br-complete`.

> Flag for operator: kickoff §8 / bootstrap §11 say `bun run test:e2e` for desktop, but the shipped `apps/desktop/package.json` exposes it as `e2e`. Reconcile (add a `test:e2e` alias, or fix the docs). Note it in 7.6.

### 7.6 — `l6-p7-dod-walkthrough`  →  CLOSE (autonomous)
Walk all 11 DoD rows (`docs/strategy/layers/layer-6-request-pipeline-automation.md:682-693`): cite FR IDs (§2.1 coverage map) + test IDs + evidence (run-manifest/SQL/screenshot). Mark PASS/PARTIAL/FAIL honestly.
- **DoD #8 = PARTIAL** — "prepare+submit end-to-end" proven via smoke test (mechanism correct), real legally-binding submission deferred per operator 2026-06-01. Do NOT mark it PASS.
- Also record the desktop e2e script discrepancy.
Diary, commit, push, `bin/br-complete`.

### 7.7 — `l6-p7-final-completion-gate`  →  CLOSE (autonomous)
1. `git pull --rebase && bin/br-claim testproject1-3yqf.7.7`
2. `br list --label l6 --status open --limit 100` — confirm only 7.7 (+ epics) remain.
3. Confirm 10 CI gates green + DoD walkthrough done (with #8 PARTIAL documented).
4. Close 7.7, then the phase + implementation epics:
   ```bash
   bin/br-complete testproject1-3yqf.7.7
   bin/br-claim testproject1-3yqf.7 && bin/br-complete testproject1-3yqf.7        # Phase-7 epic
   bin/br-claim testproject1-3yqf   && bin/br-complete testproject1-3yqf          # L6 implementation epic
   ```
5. Announce via AgentMail (`thread_id="l6-m6-closure"`): "M6 achieved on a SMOKE-TEST basis. 63/63 task beads closed. 10/10 CI gates green. DoD 1–11 walked (#8 PARTIAL — real legally-binding submission deferred). Live REMOTE_DRAFT cap-respect + synthetic draft_saved/bid_submitted mechanisms proven. Diary: live-demo/<date>-l6-m6-smoke.md. L6 is the final layer; no L7."
6. Commit final diary, push.

---

## §11. Synthetic Materials Appendix (fabricate these; label them)

**Placeholder company profile** — already seeded 2026-05-25 (`shared_company_profile` id `5f87c23d-8b23-4b28-85a0-9548c1a02787`, org_id `sorellon`): real identity/address/contact (double-L), `bid_profile_json` certs+insurance are `SMOKE_TEST_PLACEHOLDER` with expiry 2027-12-31, `sme_status: micro`. Re-seed from the smoke-test diary's "Profile Seeding" block if the DB was reset.

**Synthetic `BidScopePayload`** (shape from `packages/bidding/bid-scope-types.ts` + `bid_workflow/types.rs`; all values labeled synthetic):
```json
{
  "bidPortalType": "procontract",
  "processId": "SMOKE-TEST-<uuid>",
  "lotIds": ["SMOKE-TEST-lot-1"],
  "candidateIds": ["SMOKE-TEST-cand-1"],
  "selectedQuoteIds": ["SMOKE-TEST-quote-1"],
  "portalOpportunityRef": "SMOKE-TEST-DN000000",
  "decisionBasisHash": "<64-hex; daemon computes/validates at run time>",
  "sideEffectCap": "RemoteDraft",            // FormalSubmission only in the 7.3 synthetic run
  "companyProfileVersion": "5f87c23d-8b23-4b28-85a0-9548c1a02787",
  "reviewRequired": true
}
```

**If you drive the full workflow (7.2 Proof B(i)),** the readiness gate needs a minimal synthetic upstream chain + `operator_decision_heads(disposition='approved', reviewed_basis_hash=<matching>)`. The 2026-05-25 session judged this ~80 NOT-NULL-column fabrication too heavy and pivoted to `--skill-path`. **Time-box it to 30 min; if it balloons, take the §10-7.2 Proof B(ii) fallback and cite the test suite.** Either path legitimately closes the bead under the smoke-test mandate.

**Never** present any of this as real. Every synthetic artifact carries a `SMOKE-TEST` / `SMOKE_TEST_PLACEHOLDER` marker.

---

## §12. Gotcha / Failure-Mode Catalog (everything that bit the last run)

| Symptom | Cause | Fix |
|---|---|---|
| Commit/push rejected: "AGENT_NAME required" | pre-commit hook needs AgentMail identity | `export AGENT_NAME=<you>` in the commit shell (§8) |
| `git add` says path ignored | `artifacts/` is gitignored | diary goes in `live-demo/` (tracked); use `git add -f` only for `artifacts/` edits |
| CDP `/json/version` refused | Chrome closed between sessions | `bin/ensure-cdp-9222` (standing first step) |
| Supabase down | Colima/Supabase stopped on reboot | §7(c) restart with the `--exclude` list (vector breaks on Colima's docker.sock) |
| `bid_workflow_states`/profile missing locally | wrong DB (project `./state/state.db` stub) or fresh state.db | use `~/Library/Application Support/com.procintel.desktop/state/state.db`; `--dry-run` to migrate; re-seed mirror per diary |
| `br ready` shows only 7.1 | 7.2–7.7 dependency-blocked | correct — close in order; don't try to claim ahead |
| Skill stops at register-interest | `NotificationOrLock` > `RemoteDraft` cap | EXPECTED and CORRECT — it's the safety proof; do not raise the cap on a real opportunity |
| Readiness gate blocks synthetic run | no approved `operator_decision_heads` / incomplete chain | either seed the minimal chain (time-boxed) or take the `--skill-path` fallback; a *blocked* gate on incomplete data is itself a valid correctness proof — document it |
| Tempted to wait for FORMAL_SUBMISSION authorization | misreading 7.3 | §3 standing authorization covers the synthetic close; don't wait, don't submit real |
| `BidDuplicateDetected` | a prior real draft/submission exists | for synthetic runs, use a fresh synthetic processId |

---

## §13. Session Landing (only if genuinely blocked)

If a constitutional block or irrecoverable technical failure stops you: push WIP (`git commit -m "wip: <id> — handoff"`, set `AGENT_NAME`), `bin/br-sync && git add .beads/ && commit && push`, reset the in-progress bead to `open` with a `br comments add` note, `release_file_reservations(...)`, send an `[l6] handoff` AgentMail with done/blocked/next, confirm `git status` clean, stop. Never `git reset --hard` / `git push --force`. **But first re-read §0 #2/#5 — most "blocks" last time were the agent over-scoping a real bid; the smoke-test path is almost always completable.**

---

## §14. Quick Reference

| Thing | Value |
|---|---|
| Project key | `/Users/alinaaitova/Desktop/Testproject1` |
| Beads | label `l6`, pattern `testproject1-3yqf.7.*`, phase epic `…3yqf.7`, impl epic `…3yqf` |
| Operator account | `sales@sorellon.com` (REAL — double-L; "sorelon" in docs is a typo) |
| Portal | `procontract.due-north.com` · CDP profile `/Users/alinaaitova/.chrome-automation` · port `9222` · helper `bin/ensure-cdp-9222` |
| Portal driver | `agent-browser --cdp 9222` (no tier limit); computer-use Chrome = read-only screenshots |
| Daemon DB (REAL) | `~/Library/Application Support/com.procintel.desktop/state/state.db` (NOT `./state/state.db`) |
| 7 bid states | `not_started, ready_to_bid, bid_in_progress, draft_saved, bid_submitted, bid_failed, portal_state_unknown` (`bid_workflow/types.rs:8`) |
| SideEffectClass (asc) | `ReadOnly < RemoteDraft < NotificationOrLock < LegalDeclaration < FormalSubmission < FundsCommitted` (default cap `RemoteDraft`) |
| daemon modes | `--dry-run` · `--skill-path <dir> --job-parameters-json <JSON>` · `--run-bid-workflow <bid-job-id>` |
| Supabase restart | `colima start && supabase --workdir infra start --exclude vector,analytics,inbucket,edge-runtime,functions,storage,realtime,imgproxy` |
| Seeded profile | `shared_company_profile` id `5f87c23d-8b23-4b28-85a0-9548c1a02787`, org_id `sorellon`, placeholder certs/insurance |
| Diary dir (tracked) | `docs/pipeline/l6-request-pipeline-automation/live-demo/` |
| Prior attempt record | `…/live-demo/2026-05-25-smoke-test.md` |
| Bid skill / overlay | `skills/bid-portal-procontract/` · `skills/validation/overlay-bid-portal-procontract/` |
| Bid runner | `crates/daemon/src/bid_workflow/{runner,state_machine,readiness_gate,decision_basis_hash,side_effect_guard,types}.rs` |
| Validation verdict SQL | link col is `worker_job_id` (NOT `bid_job_id`) |
| UI components (kebab, `packages/ui/`) | `bid-workflow-status-badge.tsx`, `bid-workflow-start-button.tsx`, `bid-scope-summary.tsx`, `draft-saved-review-panel.tsx`; desktop tab `apps/desktop/src/pages/BidWorkflowTab.tsx` |
| Commit prereq | `export AGENT_NAME=<you>` · diary in `live-demo/` (tracked) · `git add -f` only for `artifacts/` |
| AgentMail | `http://127.0.0.1:8765` · Branch `main` (no feature branches) |
| Exit criterion | All 7 Phase-7 beads + epics closed on smoke-test basis; DoD #8 PARTIAL (real submission deferred); M6 announced |
