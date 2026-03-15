# Execution Push — UK Public Procurement Intelligence Platform

> **Role:** You are a continuing execution agent building compact layer documentation. This is a push prompt — you've been working before and need to re-orient and continue.
>
> **Constraint:** Converge toward the maintained doc set. Do not let the graph drift back into document sprawl.

---

## 1. Re-Orient

### 1.1 Re-Read Source Docs (If Context Is Stale)

If your context has been compressed or you're resuming from a previous session:

1. Re-read `docs/strategy/layer-strategy.md` — understand the layer structure
2. Re-read any layer docs you've already produced — verify you know the current state
3. Re-read `AGENTS.md` — refresh project constraints

### 1.2 Confirm Convergence

Before picking work, verify:
- [ ] All content you've produced so far is in files within the maintained doc set (`docs/strategy/`)
- [ ] No extra files have been created outside the maintained set
- [ ] No layer doc has thin sections that should be denser
- [ ] The cross-reference index (if started) matches reality

### 1.3 Check Bead Status

```bash
br ready --json    # What's available
br list --status=in_progress  # What you or others have claimed
```

---

## 2. Check Coordination (If Multi-Agent)

### 2.1 Check Inbox

```
fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="your-name")
```

Acknowledge any messages. Note any conflicts or coordination requests.

### 2.2 Check File Reservations

```
file_reservation_paths(project_key, agent_name, paths=["docs/strategy/**"])
```

Verify your reservations are still active. Avoid claiming work on files another agent has reserved.

### 2.3 Pull Latest

```bash
git pull --rebase
```

Check if other agents have modified files you're working on. If so, re-read those files before continuing.

---

## 3. Pick Real Work

### 3.1 Select Next Bead

```bash
br ready --json
```

Choose the highest-priority non-epic bead. Do NOT pick:
- Epic beads (organizational containers, not content producers)
- Beads with unresolved dependencies
- Beads targeting files currently reserved by another agent

### 3.2 Verify Target

Before executing the bead:
1. Confirm the target file is in the maintained doc set
2. Confirm you understand what section of that file this bead owns
3. Confirm you have the research doc context needed for this section

### 3.3 Claim

```bash
br update {id} --status=in_progress
```

Reserve the file and announce (if multi-agent).

---

## 4. Execute

### 4.1 Merge, Consolidate, Absorb

For the claimed bead:
1. Read the research doc sections this bead draws from
2. Read the current state of the target file
3. Write the section content — DENSE, execution-grade, self-contained
4. Include: full SQL schemas, full TypeScript interfaces, full algorithms, full acceptance criteria

### 4.2 Preservation Rules

- **Include every invariant** that applies to this section's content
- **Include every table definition** in full (not just names)
- **Include every interface** in full (not just type names)
- **Include every computation** with steps (not just "computes X")
- **Preserve open questions** as explicit "Open Questions" subsections

### 4.3 Do NOT

- Create extra files — merge into maintained files
- Summarize away detail — be dense
- Defer to the research doc — be self-contained
- Silently resolve open questions — preserve them
- Weaken invariants — enforce them

---

## 5. Verification

After writing content for each bead:

### 5.1 File-Count Discipline

```bash
ls docs/strategy/
```

Count files. Compare against maintained set. If extra files exist → merge and flag for cleanup.

### 5.2 Density Check

For the section you just wrote:
- Is it at least as detailed as the research doc source?
- Are SQL schemas complete (not abbreviated)?
- Are interfaces complete (not abbreviated)?
- Are algorithms step-by-step (not summarized)?

If anything is thin → expand it now, not later.

### 5.3 Self-Containment Check

Ask: "Could an implementation agent build this component from this section alone, without opening the research doc?"

If the answer is no → add the missing context.

---

## 6. Finish Properly

### 6.1 Close Bead

```bash
br close {id} --reason "Completed: {brief summary}"
br sync --flush-only
```

### 6.2 Commit and Push

```bash
git add docs/strategy/{target-file} .beads/
git commit -m "docs: {layer-N}: {bead description}

br-{id}"
git push
```

### 6.3 Release and Announce

Release file reservations. Announce completion if multi-agent.

### 6.4 Continue

Pick the next ready bead. **Do not ask what to work on next.** Do not stop between beads unless you've completed a full layer or hit a blocker.

**Do not let the graph drift back into document sprawl.**

---

## 7. End-of-Session

When ending work:

1. Close or update all in-progress beads
2. `br sync --flush-only`
3. `git add .beads/ docs/strategy/`
4. `git commit -m "docs: session progress"`
5. `git push`
6. Release all file reservations
7. Announce session end

**Work is NOT complete until `git push` succeeds.**
