# Claude CLI Quota Blocker Before ProContract Enrichment Retry - QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | machine_tooling_blocker / agent_cli_unavailable |
| **Severity** | high |
| **Reproducibility** | Reproducible on the current machine at 2026-05-13 00:38 BST with the selected `claude` CLI |
| **Affected Component** | Live PTY skill execution for skills declaring `agent_cli: claude`, including `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` |
| **Working Analogue** | Stage 4b daemon regressions passed after commit `413c5a7`; pinned daemon rebuild succeeded |
| **Scope** | Cannot safely retry live Stage 4b because the selected Claude CLI exits before running the skill due quota exhaustion |
| **Environment** | macOS Darwin 25.3.0 arm64; Cargo 1.96.0-nightly; Bun 1.3.13; `agent-browser` 0.27.0; Chrome CDP 148.0.7778.97 |
| **Git State** | `main` at `413c5a7 fix procontract enrichment missing-result failure` with QA manifest edits pending; unrelated `.DS_Store` and `bin/__pycache__` entries left untouched |

---

## REPRODUCTION PROTOCOL

From repo root:

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged resume-claude-quota-probe-after-procontract-enrich-fix \
  claude -p "Return exactly OK." --dangerously-skip-permissions
```

Observed:

```text
$ claude -p Return\ exactly\ OK. --dangerously-skip-permissions
You've hit your limit · resets 2:50am (Europe/London)
```

The ProContract enrichment skill declares `agent_cli: claude`; this QA run must not silently switch it to another adapter.

---

## EVIDENCE BUNDLE

### E0 - Preserved E2E Context

```text
E2E_STATE_ROOT=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z
E2E_RUNTIME_DIR=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z
E2E_ARTIFACT_ROOT=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
E2E_LOG_ROOT=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs
E2E_DAEMON=/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
PRAGMA user_version=9
```

### E1 - Fix Commit And Pinned Daemon Rebuild

```bash
git log --oneline -3
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged resume-daemon-rebuild-after-procontract-enrich-fix \
  env CARGO_TARGET_DIR="$E2E_TARGET_DIR" cargo build -p daemon --bin daemon
```

Observed:

```text
413c5a7 fix procontract enrichment missing-result failure
d1d6ba8 docs: record procontract enrichment timeout blocker
66420a0 docs: record procontract enrichment auth pause

$ env CARGO_TARGET_DIR=/tmp/testproject1-target-machine-fork-20260512T195251Z cargo build -p daemon --bin daemon
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.39s
```

### E2 - Focused Stage 4b Regression Tests Passed

```bash
run_logged regression-procontract-enrich-missing-result-unit \
  cargo test -p daemon --bin daemon procontract_enrich_missing_result_marks_job_failed
run_logged regression-procontract-enrich-timeout-unit \
  cargo test -p daemon --bin daemon procontract_enrich_timeout_missing_result_marks_job_failed
run_logged regression-procontract-enrich-blocked-result-unit \
  cargo test -p daemon --bin daemon procontract_enrich_blocked_result_is_durable
run_logged regression-procontract-enrich-persist-docs-unit \
  cargo test -p daemon --bin daemon persist_tender_enrichments_updates_notice_and_documents
```

Observed:

```text
procontract_enrich_missing_result_marks_job_failed ... ok
procontract_enrich_timeout_missing_result_marks_job_failed ... ok
procontract_enrich_blocked_result_is_durable ... ok
persist_tender_enrichments_updates_notice_and_documents ... ok
```

### E3 - Agent CLI Quota Block

Primary log:

```text
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/resume-claude-quota-probe-after-procontract-enrich-fix.log
```

Observed output:

```text
You've hit your limit · resets 2:50am (Europe/London)
```

Because the selected agent cannot make a minimal model call, the live ProContract enrichment retry was not run in this QA turn.

---

## ROOT CAUSE HYPOTHESES

### H1: Claude Code quota is exhausted for the operator account - LIKELIHOOD HIGH

The direct minimal model probe returns a quota-limit message before any skill execution. This is outside ProContract/CDP and outside the daemon stage logic.

### H2: Live Stage 4b would now fail durably but still cannot validate enrichment - LIKELIHOOD HIGH

Commit `413c5a7` and focused tests prove the daemon missing-result paths now fail jobs durably, but the actual enrichment workflow cannot be validated while the selected CLI is quota-blocked.

---

## FIX CONSTRAINTS

1. Do not switch the ProContract enrichment skill from Claude to another adapter without an explicit product decision.
2. Do not weaken result-contract validation.
3. Do not delete preserved E2E state, runtime, artifacts, logs, or session directories.
4. Do not close or replace the persistent CDP browser.
5. Do not rerun live Stage 4b until `claude -p "Return exactly OK." --dangerously-skip-permissions` exits 0.

---

## ACCEPTANCE CRITERIA

- The direct Claude probe exits 0 and returns `OK` or equivalent minimal response.
- The same shell environment can run the pinned daemon with the selected Claude CLI.
- Then Stage 4b is retried with:

```bash
run_logged stage-04b-procontract-enrich-retry-after-timeout-fix \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest procontract-enrich
```

---

## RESUME POINT

After the quota resets or the operator restores Claude CLI capacity:

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged resume-claude-quota-probe-after-reset \
  claude -p "Return exactly OK." --dangerously-skip-permissions
run_logged stage-04b-procontract-login-open-after-claude-quota-reset \
  agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"
run_logged stage-04b-procontract-enrich-retry-after-timeout-fix \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest procontract-enrich
```

