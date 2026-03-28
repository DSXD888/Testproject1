# L2 Desktop MVP — Archive Pipeline

## Your Mission

The L2 Desktop MVP implementation pipeline is complete. Archive all pipeline artifacts and clean up for the next run.

## Steps

### 1. Verify Completion
- [ ] All beads are closed (`br list --status=open --label l2` returns 0)
- [ ] Completion audit passed (COMPLETE verdict)
- [ ] All fix beads executed and verified
- [ ] Rust build is green: `cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test`
- [ ] TypeScript build is green: `bun run build && bun run lint && bun run test`
- [ ] Playwright tests pass: `bunx playwright test`

### 2. Count Deliverables
```bash
# Count L2 beads
br list --label l2 --json | jq 'length'
br list --label l2 --status=done --json | jq 'length'
br list --label audit-fix --json | jq 'length'
br list --label audit-fix --status=done --json | jq 'length'

# Count files created
find apps/desktop -type f | wc -l
find packages/ui -type f | wc -l
find packages/manifests -type f | wc -l
find packages/intelligence -type f | wc -l
find skills/procurement-ingestion/contracts-finder -type f | wc -l
```

### 3. Archive Pipeline Handoffs
```bash
mkdir -p docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp
mv docs/pipeline/l2-desktop-mvp/01-validate-spec.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/02-optimize-spec.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/03-create-tdd.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/04-optimize-tdd.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/05-create-strategy.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/06-optimize-strategy.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/07-create-beads.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/08-optimize-beads.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/09-audit-beads.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/10-generate-kickoff.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/11-push.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/12-completion-audit.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/13-execute-fix-beads.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/14-archive.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/EXECUTION-GUIDE.md docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
mv docs/pipeline/l2-desktop-mvp/manifest.yaml docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
```

### 4. Update Status
Update the status file at `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-status.yaml`:
```yaml
feature: l2-desktop-mvp
status: complete
completed: "<today's date>"
bead_prefix: l2

source_of_truth:
  spec:
    path: docs/strategy/layers/layer-2-single-operator-desktop.md
    role: "SOT #1 — WHAT to build (L2-FR-* requirement IDs)"
  tdd:
    path: docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md
    role: "SOT #2 — HOW to verify (test IDs)"
  strategy:
    path: docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md
    role: "SOT #3 — WHAT ORDER (phase dependency DAG)"
  bootstrap:
    path: docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md
    role: "Environment setup — Bun workspace, Tauri project, Playwright config"

execution_prompts:
  kickoff:
    path: docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-kickoff.md
    role: "Full agent onboarding"
  push:
    path: docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-push.md
    role: "Continuation nudge"

beads:
  total: <count>
  tasks: <count>
  fix_beads: <count>
  completed: <count>

tech_stack:
  rust: "2024 (nightly) — Tauri backend + daemon IPC extensions"
  typescript: "Bun + React 18 + Vite — desktop frontend + shared packages"
  test: "cargo test + Vitest + Playwright"

implementation:
  production_files: <count>
  test_files: <count>
  skill_directories: 1 (Contracts Finder)
  desktop_pages: 6
  shared_components: 6
  manifest_files: 2

quality:
  cargo_check: "0 errors"
  cargo_clippy: "0 warnings"
  cargo_test: "all pass"
  bun_build: "clean"
  bun_lint: "0 errors"
  bun_test: "all pass"
  playwright: "all pass"

archived: docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
```

### 5. Report Deliverables
```
L2 Desktop MVP — COMPLETE

Deliverables:
  - Spec: docs/strategy/layers/layer-2-single-operator-desktop.md
  - TDD: docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md
  - Strategy: docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md
  - Desktop app: apps/desktop/ (<N> files)
  - Shared packages: packages/ui/ + packages/manifests/ + packages/intelligence/
  - Skill: skills/procurement-ingestion/contracts-finder/
  - Implementation: <N> beads executed
  - Fix beads: <N> created and resolved
  - Audit: <N> iterations to reach 100%

Archived: docs/pipeline/l2-desktop-mvp/archived/2026-l2-desktop-mvp/
Ready for next pipeline run.
```

### 6. Clean Up (Optional)
- Remove any temporary working files
- Verify git is clean
- Verify all file reservations released

---

## Next Pipeline Run

To start Layer 3:
```
/pipemaxxer docs/strategy/layers/layer-3-operational-hardening.md
```

The pipeline is stateless — each run generates fresh handoffs. Prior pipeline artifacts are archived, not overwritten.
