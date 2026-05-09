# L5 Expanded Commercial Intelligence — Archive Pipeline

## Your Mission

The L5 implementation pipeline is complete. Archive all pipeline artifacts and clean up for the next pipeline run (L6 — Request Pipeline Automation).

---

## Pre-Archive Verification

Before archiving, verify ALL of the following are green. If ANY check fails, STOP — the pipeline is not yet complete and archive is premature.

### 1. Bead State

```bash
br list --status=open --label=l5 | wc -l                # must be 0
br list --status=open --label=audit-fix | wc -l         # must be 0
br list --status=done --label=l5 | wc -l                # full L5 bead count (≥40)
```

- [ ] All L5-prefixed beads are `done`.
- [ ] All `audit-fix` beads from step 12 are `done`.

### 2. Completion Audit Verdict

```bash
ls docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-*.md
```

- [ ] Latest completion audit report exists.
- [ ] Verdict is **COMPLETE — zero gaps**.
- [ ] If verdict was INCOMPLETE on first audit, fix-bead loop completed AND a follow-up audit produced COMPLETE verdict.

### 3. Quality Gate (full)

```bash
# Rust
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done

# TypeScript
bun run build
bun run lint
bun run test

# Apps (only if L5 added desktop/web touch points — typically unchanged from L4)
(cd apps/desktop && bun run test:e2e) && (cd apps/web && bun run test:e2e)
```

- [ ] All commands succeed with zero warnings, zero failed tests.
- [ ] Cumulative test count meets or exceeds 57 (per spec §3.2).

### 4. Definition of Done Walkthrough

Walk all DoD criteria from spec §3.1. Each MUST be green:

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Alibaba/1688 search skills operational with Chinese keyword clusters | ☐ |
| 2 | Amazon search skill operational | ☐ |
| 3 | PCS (Scotland) ingestion skill operational | ☐ (REQUIRED — Scotland IN-scope per operator decision 2026-05-08) |
| 4 | HMRC FPO API integration for HS code classification candidates | ☐ |
| 5 | Multi-source recall produces candidates from ≥2 independent sources | ☐ |
| 6 | 10-digit resolution via HMRC Trade Tariff API produces UK-specific commodity codes | ☐ |
| 7 | Measure scanner detects AD/CVD/safeguard/quota measures and models interactions | ☐ |
| 8 | HMRC Exchange Rate Service provides customs valuation FX (monthly rolling) | ☐ |
| 9 | Xe commercial FX provides spot rates | ☐ |
| 10 | Freightos + SeaRates provide freight market data | ☐ |
| 11 | Decision-grade landed cost per validated category (with full audit trail) | ☐ |
| 12 | Category-specific gold-set expansion completed for ≥3 categories | ☐ |
| 13 | (if added in step 01) Local quality gate verified (no .github/workflows/) | ☐ |

- [ ] All criteria pass.

### 5. Stage 11 Live Demo

- [ ] Decision-grade landed cost has been produced end-to-end for a real Contracts Finder lot.
- [ ] Manifest renders correctly in desktop + web portal with new fields visible per RLS classification.
- [ ] Demo artifacts retained for archive (e.g., `runtime-stage14-l5-decision-grade-{date}/`).

### 6. L5→L6 Readiness Gate Producible

- [ ] `CLASSIFICATION_AMBIGUOUS` emission verified with test fixture.
- [ ] `TARIFF_BRANCH_DIVERGENCE` emission verified with test fixture.
- [ ] `SUPPLEMENTARY_UNIT_MISSING` emission verified with test fixture.
- [ ] `decision_grade = broker_required` set in appropriate test fixture.
- [ ] Classification freshness expiry triggers verified.

L6 implementation pipeline (next layer) depends on these gate producers. If any are missing, L6 cannot be started.

### 7. Git State Clean

```bash
git status                  # clean working tree
git log --oneline -5        # last 5 commits relate to L5 closure
git push                    # already pushed
```

- [ ] No uncommitted L5-related work.
- [ ] No unpushed commits.

---

## Archive Steps

### Step 1: Create Archive Folder

```bash
ARCHIVE_DATE=$(date +%Y-%m-%d)
mkdir -p docs/pipeline/l5-expanded-commercial-intelligence/archived/${ARCHIVE_DATE}-l5-expanded-commercial-intelligence
```

### Step 2: Move Pipeline Handoffs

```bash
ARCHIVE_DIR=docs/pipeline/l5-expanded-commercial-intelligence/archived/${ARCHIVE_DATE}-l5-expanded-commercial-intelligence

mv docs/pipeline/l5-expanded-commercial-intelligence/01-validate-spec.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/02-optimize-spec.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/03-create-tdd.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/04-optimize-tdd.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/05-create-strategy.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/06-optimize-strategy.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/07-create-beads.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/08-optimize-beads.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/09-audit-beads.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/10-generate-kickoff.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/11-push.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/12-completion-audit.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/13-fix-beads-kickoff.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/14-fix-beads-push.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/15-archive.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/EXECUTION-GUIDE.md "$ARCHIVE_DIR/"
mv docs/pipeline/l5-expanded-commercial-intelligence/manifest.yaml "$ARCHIVE_DIR/"
cp docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-*.md "$ARCHIVE_DIR/"  # preserve in both places
```

`docs/pipeline/l5-expanded-commercial-intelligence/artifacts/` retains the working artifacts (TDD, strategy, kickoff, push, status, bootstrap, alias-map, bulk-import) — those are PERMANENT references for L6 and beyond.

### Step 3: Update Status

```bash
cat > docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-status.yaml << EOF
feature: l5-expanded-commercial-intelligence
status: complete
completed: ${ARCHIVE_DATE}
spec: docs/strategy/layers/layer-5-expanded-commercial-intelligence.md
tdd: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md
bootstrap: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md
strategy: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md
kickoff: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-kickoff.md
push: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-push.md
completion_audit: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-${ARCHIVE_DATE}.md
bead_prefix: l5
total_beads: <fill from br list --status=done --label=l5 | wc -l>
fix_beads: <fill from br list --status=done --label=audit-fix | wc -l>
audit_iterations: <fill — usually 1 if first audit returned COMPLETE; 2+ if fix-bead loop required>
ready_for_l6: true
l5_to_l6_gate_producible: true
EOF
```

### Step 4: Update Memory

After archive completes, update auto-memory with the L5 close:

```text
# Add a new project memory entry:
- L5 Expanded Commercial Intelligence completed on ${ARCHIVE_DATE}.
- Archive at docs/pipeline/l5-expanded-commercial-intelligence/archived/${ARCHIVE_DATE}-l5-expanded-commercial-intelligence/
- Status: complete; ${TOTAL_BEADS} beads + ${FIX_BEADS} audit-fix beads closed; ${AUDIT_ITERATIONS} audit iteration(s) to COMPLETE verdict.
- L6 implementation pipeline now eligible: L5→L6 readiness gate producible (5 blocker conditions verified).
- Next pipeline run: /pipemaxxer docs/strategy/layers/layer-6-request-pipeline-automation.md
```

### Step 5: Commit + Push

```bash
git add docs/pipeline/l5-expanded-commercial-intelligence/
git commit -m "$(cat <<'EOF'
l5-archive: complete pipeline archive for L5 Expanded Commercial Intelligence

All L5 beads closed; completion audit verdict COMPLETE; quality gates green;
DoD all criteria passing; Stage 11 live demo green; L5→L6 readiness gate
producible. Archived at docs/pipeline/l5-expanded-commercial-intelligence/
archived/${ARCHIVE_DATE}-l5-expanded-commercial-intelligence/.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
git pull --rebase && git push
```

### Step 6: Report Deliverables

```text
✅ L5 Expanded Commercial Intelligence — COMPLETE

📦 Deliverables:
  - Spec: docs/strategy/layers/layer-5-expanded-commercial-intelligence.md
  - TDD: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md
  - Bootstrap: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md
  - Strategy: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md
  - Kickoff: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-kickoff.md
  - Push: docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-push.md
  - Implementation: <N> beads executed
  - Fix beads: <M> created and resolved
  - Audit iterations: <K> to reach COMPLETE
  - 19 new TS files in packages/intelligence/src/{customs,fx,logistics,validation}/
  - 1 new SQLite migration (product_commodity_codes)
  - 10 new ErrorCode variants in daemon/errors/manifest.rs (enum + as_str() matcher)
  - 4-5 new RegisteredSource entries in daemon/sources/registry.rs
  - 4 new CronJobDefinition entries in daemon/scheduler/cron.rs
  - 3-4 marketplace skills matured to stable (alibaba/amazon/1688 + optional pcs)
  - ≥3 gold-set categories with calibration corpus
  - CBAM forward placeholder shipping

📁 Archived: docs/pipeline/l5-expanded-commercial-intelligence/archived/${ARCHIVE_DATE}-l5-expanded-commercial-intelligence/
🔄 L6 implementation pipeline now eligible.
```

### Step 7: Clean Up (Optional)

- Verify git is clean: `git status`.
- No need to delete `runtime*/` or `state*/` directories — those are project working areas, not pipeline artifacts.
- Reset bead workspace not required — `br` state persists for L6.

---

## Next Pipeline Run

To start L6:

```bash
/pipemaxxer docs/strategy/layers/layer-6-request-pipeline-automation.md
```

The L6 pipeline is stateless — pipemaxxer generates fresh handoffs. L5 artifacts are archived, not overwritten.

---

## Key Reminders

- Archive ONLY after every gate is green. Premature archive hides incomplete work.
- The L5→L6 readiness gate is the load-bearing handoff. Verify it's producible BEFORE archiving.
- Memory update closes the loop for future pipemaxxer runs.
