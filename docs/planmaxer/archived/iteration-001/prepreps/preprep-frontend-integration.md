# Pre-Prep: Frontend & Desktop-Daemon Integration Architecture

## Step 1: Pre-Prep Research

Read the following documents in this exact order. Do not skip any.

1. **AGENTS.md** — project constraints, toolchain, rules
2. **The research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Read the ENTIRE document, but pay special attention to:
     - §5.8 (Control Panel and Viewing Portal Data Flow)
     - §5.10 (API Design)
     - §8.2 (File Reference Map — Desktop Shell and Web Shell sections)
     - ADR-003 (Tauri v2 decision)
     - ADR-008 (Desktop/Web split)
     - §6.4 (Performance Considerations)
     - §4.7 (Database Architecture — sync protocol, authority matrix)
     - §4.9 (Agent Orchestration — daemon internal API)
3. **The handoff prompt that produced the feedback:** `docs/planmaxer/active/handoffs/handoff-frontend-integration.md`

After reading, you should be able to explain:
- How the desktop communicates with the daemon (current spec vs gaps)
- What the web portal reads from Supabase and how
- The planned shared component architecture in `packages/ui` and `packages/contracts-schema`
- The desktop file map (which pages, what each does)
- The sync protocol and how it affects web portal staleness
- The daemon's internal API endpoints

**File Ownership:** This pre-prep owns the following sections for editing:
- §5.8 (Control Panel and Viewing Portal Data Flow) — FULL ownership
- §5.10 (API Design) — FULL ownership
- §8.2 — Desktop Shell and Web Shell file map subsections ONLY
- ADR-003 — FULL ownership
- ADR-008 — FULL ownership
- §6.4 (Performance Considerations) — Frontend-related subsections only
- §1 (Executive Summary) — Frontend-related bullet points only

Do NOT edit: §4.7, §4.9, §5.2, §5.4 (data models), sync protocol details, daemon internals. If the reviewer suggests changes to those areas, note them as "deferred to relevant domain owner" in a comment.

---

## Step 2: Feedback Integration

<INSERT REVIEWER RESPONSE HERE>

For each change proposed by the reviewer, evaluate:

### Integration Rules

1. **For proposed changes to §5.8, §5.10, ADR-003, ADR-008:** Integrate directly. These sections are unreviewed — the reviewer's feedback is the first expert input.

2. **For proposed changes to shared component architecture:** Integrate into §5.8 and §8.2. If the reviewer proposes a new shared package or significant restructuring of `packages/ui`, add it to §8.2's file map.

3. **For proposed desktop ↔ daemon communication protocol:** Integrate into §5.10 (API Design) and §5.8. If the reviewer recommends Tauri IPC + HTTP hybrid, document the split clearly: what goes through IPC vs what goes through HTTP.

4. **For real-time streaming architecture (agent session monitoring):** Integrate into §5.8. This is new architecture — add a dedicated subsection for "Real-Time Event Streaming" if the reviewer provides a concrete design.

5. **For web portal subscription design:** Integrate into §5.8. Document which Supabase features are used (Realtime, Broadcast, etc.) and the subscription granularity.

6. **For desktop lifecycle management (startup, shutdown, daemon crash):** Integrate into §5.8 or create a new subsection. This is currently missing from the plan.

7. **For changes touching daemon internals, sync protocol, or data models:** Do NOT integrate directly. Instead, add a clearly marked note: `<!-- CROSS-DOMAIN: Reviewer suggested [X] for [section]. Needs coordination with data-architecture/agent-orchestration domain. -->`

### Evaluation Framework

For each proposed change, assess:
- **Agree strongly:** Integrates cleanly, addresses a real gap. Apply directly.
- **Agree partially:** Good direction but needs adaptation to project constraints. Apply with modifications, note what you changed and why.
- **Disagree:** Conflicts with settled decisions (ADRs) or project constraints. Note the disagreement with rationale; do not apply.

### Preservation Rules
- Preserve all ADR numbers and their core decisions (you may expand rationale or add subsections)
- Preserve the authority matrix (desktop reads local SQLite, web reads Supabase)
- Preserve file naming conventions from §8.2
- Use "as-if-always" language — write as though the plan always included these details, not "based on reviewer feedback..."

---

## Step 3: Fresh-Eyes Optimization

After completing Step 2, re-read the sections you edited with fresh eyes. Check for:

1. **§5.8 ↔ §5.10 consistency:** Does the Control Panel Data Flow reference the correct API endpoints? Does the API Design section match the communication protocol described in §5.8?

2. **§5.8 ↔ §4.9 consistency:** Does the desktop-daemon communication protocol align with the daemon's session lifecycle and adapter model described in §4.9? If §5.8 now describes event streaming, does it reference the correct daemon components?

3. **§8.2 file map consistency:** Do any new files mentioned in §5.8 or §5.10 appear in the file map? Are removed/renamed files cleaned up?

4. **ADR-003 ↔ §5.8 consistency:** Does the Tauri decision rationale reflect the communication architecture now described in §5.8?

5. **§5.8 ↔ §4.7 consistency:** Does the desktop's SQLite read pattern align with the database architecture? Does the web portal's Supabase usage match the sync protocol's projection tables?

6. **§6.4 performance:** If you added performance budgets or latency requirements, do they align with the daemon's operational characteristics described elsewhere?

7. **Terminology:** Ensure consistent use of "desktop" vs "control panel", "web portal" vs "viewing portal", "daemon API" vs "internal API" throughout all edited sections.

Fix any inconsistencies found. Make targeted edits, not wholesale rewrites.

---

## Step 4: Sync Pass

1. **Update §1 (Executive Summary):** If the frontend architecture gained significant new detail (communication protocol, real-time streaming, lifecycle management), ensure the executive summary reflects this at a high level. Use as-if-always language.

2. **Update §8.2 (File Reference Map):** If the reviewer proposed new files (e.g., Tauri command handlers, shared hooks, event stream utilities), add them to the appropriate file map sections. Maintain the existing format.

3. **Update §11 (Next Steps):** If the reviewer identified new prerequisites or changed the implementation order for frontend components, update the milestone prerequisites accordingly.

4. **Cross-reference check:** Search for any references to §5.8, §5.10, ADR-003, or ADR-008 elsewhere in the document and verify they still make sense with the updated content.

5. **Appendix E (Changelog):** Add a concise entry describing what was integrated in this review cycle.
