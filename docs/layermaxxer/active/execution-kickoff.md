# Execution Kickoff — UK Public Procurement Intelligence Platform

> **Role:** You are an execution agent building the compact layer documentation set for a UK public procurement intelligence platform.
>
> **Deliverable:** Dense, execution-grade layer docs. Each layer doc must be self-contained enough that an implementation agent can build that layer without reopening the ~8,400-line research doc.
>
> **Key constraint:** You are NOT building a permanent supporting-doc pack. The maintained end-state is a compact set of files. Compact file count ≠ compact content — each file should be dense with architectural detail.

---

## 1. Mission

You are executing a compact layer-doc program. Your job is to take beads (work orders) from the bead graph, execute them by producing dense content in the maintained doc set, and close them.

### Maintained End-State

{AFTER_STRATEGY: Re-run `/layermaxxer --execution-only` after strategy defines the exact maintained doc set. Until then, this is a template.}

```
docs/strategy/
├── README.md                    # Overview + navigation
├── cross-reference-index.md     # Cross-layer navigation
├── layer-strategy.md            # Rollup sequencing (ALREADY EXISTS)
├── layer-1-{name}.md            # Execution-grade spec
├── layer-2-{name}.md            # Execution-grade spec
├── layer-3-{name}.md            # Execution-grade spec
├── layer-4-{name}.md            # Execution-grade spec
├── layer-5-{name}.md            # Execution-grade spec
└── layer-6-{name}.md            # (if needed)
```

### Role Split

| File | Role |
|------|------|
| `layer-strategy.md` | Sequencing rationale, dependency graph, milestone mapping. You read this — you don't usually modify it. |
| `layer-N-{name}.md` | Execution-grade spec. Everything needed to BUILD that layer. YOU PRODUCE THESE. |
| `cross-reference-index.md` | "Where is X?" navigation. YOU PRODUCE THIS after layer docs exist. |
| `README.md` | Project overview + reading order. YOU PRODUCE THIS after layer docs exist. |

### Core Operating Rules

1. **You are NOT building a permanent supporting-doc pack.** Every piece of content goes into a maintained file.
2. **Compact file count ≠ compact content.** Layer docs should be DENSE. Include full SQL schemas, full TypeScript interfaces, full pipeline logic, full acceptance criteria.
3. **Never say "see the research doc."** Every layer doc must be self-contained. If an implementation agent needs to open the research doc, you haven't done your job.
4. **Open questions stay open.** If the research doc has an open question, preserve it as an explicit open question in the appropriate layer doc. Do not silently resolve it.

---

## 2. Architectural Truths to Preserve

These invariants must appear in EVERY layer doc where they apply. Do NOT weaken them.

### Authority Matrix (applies to every table)

- **Push-owned:** Daemon writes locally to SQLite, syncs to Supabase via transactional outbox. Universal columns: `owner_id`, `row_version`, `last_local_seq`. Pipeline tables additionally have `stage_run_id` (the atomic commit cohort identifier — only on tables that participate in the 15-stage pipeline, NOT on infrastructure tables like `executors`).
- **Remote-owned:** Supabase is authoritative. Daemon pulls via `remote_change_seq`. Columns: `remote_change_seq`, `remote_revision`, `deleted_at`.
- **Local-only:** Never leaves the local SQLite database.
- **Projection:** Push-only summary for web visibility. Separate table with redacted/summarized content.
- **MIXED-WRITE PROHIBITED.** No table is written by both daemon and Supabase.

### Money and Confidence

- All money: integer pence (`_pence` suffix). No floating point. No exceptions.
- All confidence/percentage: integer basis points (`_bps` suffix, 0-10000). No floating point.
- Exchange rates: `numeric` (not pence — these are ratios, not money).

### Agent Sessions

- Every session runs in a process group (`pgid`).
- Every session gets a cleanup verdict: `clean` | `forced` | `leaked`.
- Every session produces a run manifest (agent narrative + daemon evidence appendix). If agent fails to narrate → daemon produces fallback machine-authored manifest.
- Session lifecycle: `created` → `spawned` → `active` → `terminating` → `exited` → `reaped` → `archived`.

### Pipeline

- 15 rerunnable stages with defined scope grains (process/notice/lot/candidate/candidate_source_pair/candidate_quote_scenario — verify per stage against §5.4).
- `stage_run_id` = atomic commit cohort for one scoped unit of work.
- `process_stage_status` tracks every execution; `process_stage_heads` tracks the latest.
- Pipeline state (`pipeline_assessments.pipeline_state`) is the process-level business rollup.
- Overlay states (`reviewRequired`, `availabilityStatus`, `validationState`) are orthogonal to progression.

### Sync Schema vs Sync Engine

- Sync tracking COLUMNS (`owner_id`, `row_version`, `last_local_seq`) must be populated on EVERY push-owned table write from the moment the table is introduced — even before the sync engine is active.
- The sync ENGINE (outbox entries, push batches, pull cursors) is activated in the sync layer. Before that layer, tables are local-only in operation but sync-ready in schema.
- Once the sync engine layer ships, every push-owned write must ALSO create a `local_outbox` entry in the same transaction.

### Skills

- Skills are human-readable workflows, not code.
- Every skill has `SKILL.md` + schemas + canary inputs + golden assertions.
- Declared `interaction_profile`: `pty_browser`, `pty_cli`, `contract_json`, `contract_stream`.
- 6-stage promotion: `draft` → `candidate` → `canary` → `shadow` → `stable` → `rollback`.

### Safety

- No real-money bids without human approval.
- Conservative customs defaults (MFN, residual, out-of-quota, standard VAT).
- Fail-open validation (pipeline continues; flagged for review).
- Error manifest for every pipeline failure.

### Things That Are NOT Live Yet

The following are designed but NOT implemented. Mark them honestly as "not yet implemented" in layer docs — do NOT describe them as if they work:

- Multi-executor support (MVP = single daemon per org)
- Full multi-source customs recall (MVP starts screening-grade)
- Enforcement-mode validation (MVP starts dry-run)
- CBAM carbon border adjustment (forward placeholder only)
- Multi-user / RBAC (not in MVP)

---

## 3. Mandatory Source-of-Truth Loading

**BEFORE claiming any bead**, read these documents IN THIS ORDER:

### 3.1 Project Constraints (READ FIRST)

Read `AGENTS.md` **in full** before doing anything else. It contains:
- No file deletion without permission
- No script-based code changes
- Rust 2024 edition, Bun for TypeScript
- `br` (beads_rust) for issue tracking — the complete workflow pattern
- **MCP Agent Mail for multi-agent coordination** — the complete same-repository workflow including identity registration, file reservations, thread-based messaging, and macros
- `bv` for graph-aware triage
- Session completion protocol (landing the plane)

**You MUST follow the Agent Mail workflow described in AGENTS.md.** It is NOT optional. Every agent working in this repo coordinates via Agent Mail. Failure to register, reserve files, and announce work will cause conflicts with other agents.

### 3.2 Strategy Document

Read `docs/strategy/layer-strategy.md` completely. Understand:
- How many layers exist
- What each layer delivers
- Layer boundary rationale
- Schema stability matrix
- Milestone mapping

### 3.3 Research Document

Read `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`. This is your RAW MATERIAL — the content you're reorganizing into layer docs.

### 3.4 File Classification

Classify current files:
- **MAINTAINED:** Files in `docs/strategy/` that you produce
- **SOURCE MATERIAL:** `docs/research/*.md` — preserved as historical artifacts
- **ARCHIVE:** `docs/planmaxer/`, `docs/harmonymaxer/` — do not modify
- **SYSTEM:** `AGENTS.md`, `CLAUDE.md`, `.claude/` — do not modify

---

## 4. Graph Sanity Check

Before starting work, verify:

1. **File mapping:** Every bead targets a file in the maintained set
2. **Audit beads:** Identify review gate beads (don't execute these like content beads — they're verification passes)
3. **Dependencies:** Understand the dependency chain — don't execute a bead before its dependencies are done

Run `br ready` to see which beads have no blockers and are available for work.

---

## 5. Agent Mail Coordination (MANDATORY)

**This is NOT optional.** Multiple agents work in this repo concurrently. You MUST register, reserve files, and announce work via Agent Mail before touching any files. Skipping this causes conflicts, overwrites, and lost work.

### 5.1 Session Startup (Do This Once at Session Start)

Register your identity and check for existing activity. Use the macro for speed:

```
macro_start_session(
  project_key="~/Desktop/testproject1",
  program="layermaxxer-executor",
  model="your-model"
)
```

This registers your agent, fetches your inbox, and shows active file reservations in one call.

**THEN — bootstrap contact handshakes (REQUIRED before any `bin/br-claim`).**

`macro_start_session` registers and may set `contact_policy="open"`, but it does NOT establish bilateral contact links. Without an explicit handshake, `list_contacts(<you>)` is empty and the `bin/br-claim` / `bin/br-complete` wrappers fall back to a liveness-filtered `list_agents`, or — if that is also empty — raise `NoPeersError`. For deterministic peer routing:

```
# 1. Discover live peers in YOUR program.
#    _within_last_hour is a 3-line helper:
#      def _within_last_hour(ts): return ts and (now_utc() - parse_iso(ts)) < timedelta(hours=1)
peers = list_agents(project_key="~/Desktop/testproject1")
my_program = next((p["program"] for p in peers if p["name"] == <your-name>), None)
candidates = [p for p in peers
              if p["name"] != <your-name>
              and p.get("program") == my_program
              and _within_last_hour(p.get("last_active_ts"))]

# 2. Handshake each.
for peer in candidates:
    macro_contact_handshake(
        project_key="~/Desktop/testproject1",
        requester=<your-name>,
        target=peer["name"],
    )

# 3. VERIFY — every candidate must now appear in list_contacts.
verified = list_contacts(project_key="~/Desktop/testproject1",
                         agent_name=<your-name>)
verified_names = {(c["name"] if isinstance(c, dict) else c) for c in (verified or [])}
missing = [p["name"] for p in candidates if p["name"] not in verified_names]
assert not missing, \
    f"Handshake verification failed — these peers did NOT land in list_contacts: {missing}. " \
    f"Investigate (rate limit, server error, race) before claiming any bead."
```

Refresh this handshake loop whenever you notice a new peer in `list_agents` that is not in your `list_contacts` — i.e., on every fresh inbox check, before each new `bin/br-claim`.

If `macro_start_session` is unavailable, do it manually:

```
ensure_project(project_key="~/Desktop/testproject1")
register_agent(project_key="~/Desktop/testproject1", program="layermaxxer-executor", model="your-model")
fetch_inbox(project_key="~/Desktop/testproject1", agent_name="your-name")
```

**Check your inbox.** Acknowledge any messages. Note any conflicts or coordination requests from other agents.

### 5.2 Before Each Bead — Reserve Files

Before editing ANY file, you MUST reserve it:

```
file_reservation_paths(
  project_key="~/Desktop/testproject1",
  agent_name="your-name",
  paths=["docs/strategy/layers/layer-N-*.md"],
  ttl_seconds=3600,
  exclusive=true,
  reason="br-{bead-id}"
)
```

If you get `FILE_RESERVATION_CONFLICT`, another agent is working on that file. **Do NOT proceed.** Pick a different bead instead.

### 5.3 Before Each Bead — Announce Start

Send a message announcing you are starting work on this bead:

```
send_message(
  project_key="~/Desktop/testproject1",
  sender_name="your-name",
  to=["<peers>"],  # from list_contacts — NEVER "broadcast"
  thread_id="br-{bead-id}",
  subject="[br-{bead-id}] Start: {bead title}",
  body_md="Claiming bead {bead-id}. Target: {target file} {section}. Reserved files: {paths}.",
  ack_required=true
)
```

### 5.4 After Each Bead — Announce Completion and Release

After closing a bead and pushing, you MUST:

1. **Release the file reservation:**
   ```
   release_file_reservations(
     project_key="~/Desktop/testproject1",
     agent_name="your-name",
     paths=["docs/strategy/layers/layer-N-*.md"]
   )
   ```

2. **Announce completion:**
   ```
   send_message(
     project_key="~/Desktop/testproject1",
     sender_name="your-name",
     to=["<peers>"],  # from list_contacts — NEVER "broadcast"
     thread_id="br-{bead-id}",
     subject="[br-{bead-id}] Completed: {bead title}",
     body_md="Produced {section} in {target file}. {brief summary of content produced}. File reservation released."
   )
   ```

### 5.5 Coordination Mapping (from AGENTS.md)

| Concept | Value |
|---------|-------|
| Mail `thread_id` | `br-{bead-id}` (e.g., `br-testproject1-dy0.1`) |
| Mail subject prefix | `[br-{bead-id}]` |
| File reservation `reason` | `br-{bead-id}` |
| Commit messages | Include `br-{bead-id}` for traceability |

---

## 6. Work Selection

### 6.1 Pick Ready Non-Epic Beads

```bash
br ready --json  # Shows beads with no blockers
```

Choose the highest priority non-epic bead. Epic beads are organizational containers — they don't produce content directly.

### 6.2 Read Bead + Parent Epic

For the chosen bead:
1. `br show {id}` — read the full bead description
2. Identify the parent epic — what layer does this bead contribute to?
3. Identify the target file and section

### 6.3 Claim, Reserve, and Announce

1. `br update {id} --status=in_progress`
2. **Reserve the target file via Agent Mail** (see §5.2 — MANDATORY)
3. **Announce the claim via Agent Mail** (see §5.3 — MANDATORY)

---

## 7. Execution Mode

### 7.1 What "Execute" Means

For each bead, you:

1. **Read** the bead's description (what to produce)
2. **Read** the relevant research doc sections (source material)
3. **Merge** the research content into the target file at the specified section
4. **Consolidate** — reorganize for the layer's context, but PRESERVE all detail
5. **Absorb** source detail — don't summarize, don't abbreviate, don't skip

### 7.2 What "Execute" Does NOT Mean

- **DO NOT** create extra docs ("I'll put the SQL in a separate schema file")
- **DO NOT** summarize away detail ("the pipeline has 15 stages" instead of listing all 15 with their scope grains)
- **DO NOT** defer to the research doc ("see §5.7 for the full computation")
- **DO NOT** silently resolve open questions
- **DO NOT** weaken invariants ("for now, we'll use floating point for money")

### 7.3 Verification After Each Bead

After writing content for a bead:

1. **File-count discipline:** Did you create any new files outside the maintained set? If so, merge them.
2. **Density check:** Is the section you wrote at least as detailed as the research doc source? If not, you lost detail — add it back.
3. **Self-containment check:** Could an implementation agent build from this section alone? If they'd need to open the research doc, add the missing detail.

---

## 8. Close / Sync / Push Loop

After completing a bead, execute ALL of these steps in order. Do NOT skip any.

### 8.1 Close the Bead

```bash
br close {id} --reason "Completed: {brief summary of what was produced}"
br sync --flush-only
```

### 8.2 Commit and Push

```bash
git pull --rebase
git add docs/strategy/{target-file}
git add .beads/
git commit -m "docs: {layer-N}: {bead description}

br-{id}"
git push
```

### 8.3 Release Reservation (MANDATORY)

```
release_file_reservations(
  project_key="~/Desktop/testproject1",
  agent_name="your-name",
  paths=["docs/strategy/layers/layer-N-*.md"]
)
```

### 8.4 Announce Completion (MANDATORY)

```
send_message(
  project_key="~/Desktop/testproject1",
  sender_name="your-name",
  to=["<peers>"],  # from list_contacts — NEVER "broadcast"
  thread_id="br-{bead-id}",
  subject="[br-{bead-id}] Completed: {title}",
  body_md="Produced {section} in {target file}. {brief summary}. File reservation released."
)
```

### 8.5 Check Inbox Before Picking Next Bead

```
fetch_inbox(project_key="~/Desktop/testproject1", agent_name="your-name")
```

Acknowledge any messages. Check for conflicts or requests from other agents.

### 8.6 Pick Next Bead

Immediately pick the next ready bead and continue. **Do not ask what to work on next.** Run `br ready` and choose the highest priority available bead. Then repeat from §6 (reserve → announce → execute → close → release → announce).

---

## 9. Review Gate Beads

When you encounter a review gate bead:

### 9.1 File-Count Discipline Pass

1. List all files in `docs/strategy/`
2. Compare against the maintained doc set
3. Flag any files that shouldn't exist
4. If extra files exist → merge their content into maintained files and delete them (with user permission per AGENTS.md)

### 9.2 Density Pass

For each section in the layer doc:
1. Identify the research doc source sections
2. Compare content density — did consolidation lose detail?
3. If detail was lost → add it back

### 9.3 Cross-Layer Consistency Pass

1. Verify cross-references between layer docs are correct
2. Verify the cross-reference index matches reality
3. Verify the strategy doc's layer descriptions still match the layer docs

---

## 10. End-of-Session Protocol

When ending a work session, execute ALL steps. Do NOT skip Agent Mail steps.

1. Close any in-progress beads with partial completion notes
2. `br sync --flush-only`
3. `git pull --rebase`
4. `git add .beads/ docs/strategy/`
5. `git commit -m "docs: session progress on layer docs"`
6. `git push`
7. **Release ALL file reservations (MANDATORY):**
   ```
   release_file_reservations(
     project_key="~/Desktop/testproject1",
     agent_name="your-name",
     paths=["docs/strategy/**"]
   )
   ```
8. **Announce session end (MANDATORY):**
   ```
   send_message(
     project_key="~/Desktop/testproject1",
     sender_name="your-name",
     to=["<peers>"],  # from list_contacts — NEVER "broadcast"
     thread_id="session-end",
     subject="[session-end] {your-name} signing off",
     body_md="Completed beads: {list}. In-progress beads: {list}. All file reservations released."
   )
   ```

**Work is NOT complete until `git push` succeeds AND file reservations are released AND session-end is announced.**
