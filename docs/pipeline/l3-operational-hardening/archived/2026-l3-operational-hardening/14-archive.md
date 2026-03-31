# L3 Operational Hardening — Archive Pipeline

## Your Mission

The L3 Operational Hardening implementation pipeline is complete. Archive all pipeline artifacts and clean up for the next run.

## Steps

### 1. Verify Completion
- [ ] All L3 beads are closed (`br list --status done --json | grep l3`)
- [ ] All fix beads are closed (`br list --label audit-fix --status done`)
- [ ] Completion audit passed (COMPLETE verdict)
- [ ] Build is green:
  ```bash
  cargo check --all-targets
  cargo test
  cargo clippy --all-targets -- -D warnings
  cargo fmt --check
  bun run build
  bun run test
  bun run lint
  ```
- [ ] System runs unattended for 5 consecutive days with >=95% skill success rate (manual verification)

### 2. Archive Pipeline Handoffs
```bash
mkdir -p docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening
mv docs/pipeline/l3-operational-hardening/01-validate-spec.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/02-optimize-spec.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/03-create-tdd.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/04-optimize-tdd.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/05-create-strategy.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/06-optimize-strategy.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/07-create-beads.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/08-optimize-beads.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/09-audit-beads.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/10-generate-kickoff.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/11-push.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/12-completion-audit.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/13-execute-fix-beads.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/14-archive.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/manifest.yaml docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
mv docs/pipeline/l3-operational-hardening/EXECUTION-GUIDE.md docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
```

### 3. Update Status
```bash
cat > docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-status.yaml << 'EOF'
feature: l3-operational-hardening
status: complete
completed: "2026-XX-XX"  # Fill in actual date
spec: docs/strategy/layers/layer-3-operational-hardening.md
tdd: docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md
strategy: docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md
bead_prefix: l3
total_beads: XX  # Fill in actual count
fix_beads: XX    # Fill in actual count
audit_iterations: XX  # Fill in actual count
exit_criteria_met: true
unattended_days: 5
skill_success_rate_bps: XXXX  # Fill in actual rate
EOF
```

### 4. Report Deliverables
```
L3 Operational Hardening — COMPLETE

Deliverables:
  - Spec: docs/strategy/layers/layer-3-operational-hardening.md
  - TDD: docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md
  - Strategy: docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md
  - Bootstrap: docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md
  - Kickoff: docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-kickoff.md
  - Push: docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-push.md
  - Implementation: XX beads executed
  - Fix beads: XX created and resolved
  - Audit: XX iterations to reach 100%

New Code:
  - 12 new Rust daemon modules in crates/daemon/src/
  - 5 new SQLite tables (migration 0005)
  - 10 new skills (4 operational + 6 validator base)
  - 3 new TypeScript modules
  - 18 new test files (cumulative 38)

Capabilities Added:
  - Cron-scheduled ingestion (FTS + CF runs without manual triggers)
  - Resource management (browser_slots=2, agent_slots=4, source_lanes)
  - PTY hardening (fd hygiene, EIO handling, terminal geometry)
  - Relation resolution (merged/related/possible_duplicate/separate)
  - Source health monitoring + circuit breaker
  - Skill promotion flow (6 stages)
  - Worker-validator dry-run (manifests, verdicts, calibration data)

Archived: docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/
Ready for next pipeline run (Layer 4: Shared Visibility).
```

### 5. Clean Up
- Verify git is clean: `git status`
- Commit archive: `git add docs/pipeline/l3-operational-hardening/ && git commit -m "chore: archive L3 operational hardening pipeline"`
- Push: `git push`
- Sync beads: `bin/br-sync && git add .beads/ && git commit && git push`

---

## Next Pipeline Run

To start Layer 4 (Shared Visibility):
```
/pipemaxxer docs/strategy/layers/layer-4-shared-visibility.md
```

The pipeline is stateless — each run generates fresh handoffs. Prior pipeline artifacts are archived, not overwritten.
