# Pre-Prep: L3+L4 Validation Pipeline & Sync Engine

## Domain: L3+L4 Validation & Sync
## Files This Agent Can Edit

| File | Scope | Notes |
|------|-------|-------|
| `docs/strategy/layers/layer-3-operational-hardening.md` | Full file | Validation dry-run, hardening |
| `docs/strategy/layers/layer-4-shared-visibility.md` | Full file | Sync engine, enforcement |
| `docs/strategy/layer-strategy.md` | §§10-11 only (interface contracts, testing evolution) | Cross-layer governance |
| `docs/strategy/cross-reference-index.md` | Full file | Navigation updates |
| `docs/strategy/README.md` | Full file | If overview changes |

**Do NOT edit:** L0, L1, L2, L5, L6 layer docs. If changes are needed there, add a cross-domain note at the end of the file you're editing, clearly labeled `<!-- CROSS-DOMAIN NOTE FOR L{N}: ... -->`.

---

## Step 1: Pre-Prep Research

Read these documents in order:

1. **`AGENTS.md`** — Agent guidelines, rules, project overview
2. **`docs/strategy/layers/layer-3-operational-hardening.md`** — Read cover to cover (~900 lines). Pay special attention to:
   - Worker-validator system architecture (policy engine, executor, verdicts)
   - Dry-run isolation guarantees (no authoritative rows advanced, no outbox entries)
   - Risk-stratified sampling (8 risk factors, not uniform random)
   - Per-slice enforcement readiness concept
   - Skill promotion flow (6 stages, 5 version dimensions)
   - Relation resolution engine (4 classifications, heuristic scoring weights)
   - Source health monitoring and circuit breaker
   - Resource arbiter (typed quotas)
   - Calibration harness (3 corpora, initial targets)
   - 12 new Rust modules, 10 new skills, 5 new tables
3. **`docs/strategy/layers/layer-4-shared-visibility.md`** — Read cover to cover (~800 lines). Pay special attention to:
   - Sync engine push protocol (transactional outbox, push dispatcher contract)
   - Pull protocol (cursor-based, remote_change_seq)
   - Pull-before-push ordering
   - CAS for operator decisions (idempotent, command_id)
   - Active-writer lease
   - Schema epoch guard
   - Supabase schema (18 + 5 + 3 tables)
   - RLS policy matrix (4 roles)
   - Web portal architecture (Next.js + Supabase)
   - Validation enforcement transition (dry-run → production)
   - 4 domain-specific validator overlays
   - Formal calibration harness (Wilson-bound precision gates)
   - Degraded-mode UX (3 states)
4. **`docs/strategy/layer-strategy.md`** — Read §§10-11 (interface contracts, testing evolution)
5. **`docs/planmaxer/active/handoffs/handoff-l3l4-validation-sync.md`** — Read the handoff that produced the feedback

Build a deep mental model of:
- The full validation lifecycle: L3 dry-run → L4 enforcement → per-slice activation
- The sync protocol: outbox → push dispatcher → Supabase RPC → ack
- The pull protocol: cursor → pull → apply → advance
- CAS semantics for operator decisions
- How degraded mode (sync failure) affects desktop vs web
- The calibration math: Wilson-bound precision, minimum targets, what "enough data" means

---

## Step 2: Feedback Integration

Based on the Layer 3 / Layer 4 / §§10–11 material you included, the design is solid in the right places. The big choices are mostly right: transactional outbox over SQLite, remote-owned decisions with CAS, single active writer for MVP, and per-slice enforcement instead of a global validator switch.

The weak points are not in the obvious places. They are in the seams:

1. **L4 enforcement currently collides with the outbox invariant unless you add a local promotion buffer.**
2. **Your sync design needs fencing and idempotency details, not new infrastructure.**
3. **Fail-open is workable, but only if you make trust degradation explicit and retroactive validation first-class.**
4. **Per-slice activation is the right actuation unit, but raw `worker_version` granularity will make most slices never mature.**
5. **Late merges are not just dedupe; they are decision-invalidating identity changes.**

## Executive judgment

I would not change the constitutional choices. I would change the plumbing around them.

The five highest-value fixes before implementation are:

| Priority | Fix                                                                        | Why it matters                                                                                                          |
| -------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| P0       | Add a **provisional result/promotion buffer** for enforcement slices       | Without it, rejected rows can already be committed/outboxed before validation completes                                 |
| P0       | Add **writer fencing tokens** to the active-writer lease                   | Prevents stale daemons from pushing after lease loss                                                                    |
| P0       | Add a **durable CAS command ledger**                                       | `command_id` on the head row is not enough for true idempotency                                                         |
| P0       | Add **hi-water fenced pull** and **epoch-repair reproject**                | Prevents pull races and permanent remote divergence after epoch mismatch                                                |
| P1       | Introduce **cluster/canonical process identity** before late merges matter | Otherwise merges will corrupt decisions, manifests, and deep links semantically even if rows are technically consistent |

---

## 1) Validation calibration: the math, the timeline, and what is actually practical

### The core math

Your stated slice gate is:

* at least **30 adjudicated reject/flag cases**
* at least **60 audited approvals**
* **Wilson lower bound** on reject precision at 95% confidence ≥ **70%**
* **false-pass upper bound** < **5%**

A few hard facts:

* With **30 adjudicated rejects**, you need roughly **26 correct out of 30** to get the Wilson lower bound above 70%.

  * 25/30 is **not enough**
  * 26/30 is barely enough
* For the false-pass gate, **0 silent misses in 60 audited approvals** only just gets you under ~5% by the rule-of-three logic.

  * **1 miss in 60 fails**
  * So your approve-audit bar is actually stricter than it looks

That means your live-data bottleneck is almost always the **30 confirmed reject cases**, not the approvals.

### A usable timeline formula

For a given slice:

* `weeks_to_30_rejects = 30 / (runs_per_week × validation_rate × reject_yield × adjudication_rate)`
* `weeks_to_60_approves = 60 / (runs_per_week × validation_rate × approval_rate × approval_audit_rate)`

Where `reject_yield` is among **validated** runs, not all runs. Risk-stratified sampling can increase it.

### What this means in practice

| Slice profile   | Runs/week | Validation rate during bootstrap | Reject yield among validated | Weeks to 30 rejects | Weeks to 60 audited approvals | Realistic earliest full enforcement |
| --------------- | --------: | -------------------------------: | ---------------------------: | ------------------: | ----------------------------: | ----------------------------------: |
| Hot slice       |       100 |                              50% |                          15% |                   4 |                           1.4 |                          ~4–6 weeks |
| Warm slice      |        40 |                              50% |                          10% |                  15 |                           3.3 |                        ~15–18 weeks |
| Long-tail slice |        10 |                             100% |                          10% |                  30 |                           6.7 |                          ~30+ weeks |
| Your example    |       100 |                             7.5% |                          20% |                  20 |                            10 |                          ~20+ weeks |

Your example answers itself:

> 200 runs in 2 weeks, 15 validations, 3 rejects

That is nowhere near enough. At the same rate:

* 3 rejects / 2 weeks → **20 weeks** to reach 30 reject cases
* 12 approvals / 2 weeks → **10 weeks** to reach 60 audited approvals

So no, two weeks of dry-run at ~7.5% effective validation is not a meaningful path to enforcement.

### What this says about your per-slice belief

Your per-slice actuation model is right. Your per-slice **evidence acquisition** model is too slow unless you explicitly bootstrap it.

The practical answer is:

* **Hot slices** can become enforceable in **4–8 weeks**
* **Medium slices** take **months**
* **Long-tail slices** may **never** become enforcement-ready before the worker or overlay changes

That is not a flaw. That should be an explicit product truth: **many slices will remain permanently dry-run / flag-only**.

### Minimum viable calibration

There are really two gates, not one.

#### A. Overlay/family prequalification

This is what lets you trust the validator architecture at all.

Use:

* 20–30 adjudicated clean-real
* 15–20 weird-but-valid
* 30–50 mutated-bad

This proves the overlay is not insane.

#### B. Exact-slice enforcement readiness

This is what lets you block for one exact worker/source/version combination.

Use your current 30/60 gate, but only for slices that have enough traffic.

### What I would change

I would keep your per-slice enforcement key, but I would change **what counts as a “version”**.

Right now the tuple is too fine if `worker_version` means every prompt tweak or packaging change. That guarantees endless re-calibration.

Use:

* `worker_behavior_version`
* `validator_behavior_version`

Only bump those when output semantics or decision boundaries materially change. Cosmetic prompt edits, observability changes, or cost-only tuning should not reset slice readiness.

Without that change, most slices never enforce because the evidence window is shorter than the version half-life.

### The practical rollout ladder

This is the cleanest operational path:

1. **Dry-run bootstrap**

   * 100% validate first 20 live runs for a new slice
   * 100% validate all high-risk runs
2. **Calibrating dry-run**

   * 50% validate until the slice is either clearly maturing or clearly long-tail
3. **Signature-limited enforcement**

   * quarantine only on calibrated, high-precision incident signatures
   * everything else remains flag-only
4. **Full slice enforcement**

   * once 30/60 + Wilson + false-pass criteria are met

This fits your existing design because you already have `incident_signature` and `enforcement_eligible`.

That is the key practicality move: **use calibrated incident families to accelerate enforcement without collapsing the per-slice model**.

---

## 2) The biggest architectural gap: L4 enforcement vs the outbox invariant

This is the one issue I would fix before anything else.

### The collision

Layer 4 says:

* rejected results are quarantined
* rejected results do not create outbox entries
* stage heads do not advance

But Layer 4 also says:

* every push-owned mutation creates an outbox entry in the same SQLite transaction

If the worker writes push-owned rows first and validation runs after, you already created the rows and the outbox. At that point “quarantine” is post hoc theater.

### The required fix

For enforcement-active slices, you need a **local provisional promotion phase**:

1. Worker completes
2. OQG runs
3. Validator runs
4. Only then:

   * promote to push-owned tables
   * create outbox entries
   * advance stage heads

If rejected:

* keep artifacts, verdicts, and error manifests locally
* do **not** promote to authoritative push-owned state

This means worker outputs for enforcement slices must first land in a **local-only staging form**:

* artifact JSON
* result contract blob
* temporary local table
* or equivalent

But **not** directly in push-owned stage tables.

Without this, L4 enforcement is logically inconsistent.

---

## 3) Sync correctness: what is sound, what is missing, and how to harden it

The outbox/pull/CAS pattern is right. The missing details are all classic distributed-systems footguns.

### A. Active-writer lease needs a fencing token

“Second daemon rejected” is not enough.

Failure case:

* daemon A holds lease
* daemon A stalls or partitions
* daemon B acquires lease
* daemon A recovers and keeps pushing

If the remote side only checks “who is current writer,” stale A can still race unless every batch carries a **writer epoch**.

Use a remote lease row like:

* `org_id`
* `holder_executor_id`
* `lease_expires_at`
* `writer_epoch`

On acquisition, increment `writer_epoch`.
Every `apply_push_batch` must include that epoch.
Remote rejects batches with stale epoch.

That is the correct MVP single-writer fence.

### B. `command_id` on the head row is not enough for CAS idempotency

Right now you store `command_id` on `operator_decision_heads`.

That only remembers the **latest** command. It does not provide durable idempotency if:

* the same command is retried after a later command already landed
* you need to distinguish “already applied” from “stale conflict”
* you want exact replay semantics after a lost response

You need a separate **command ledger**:

`operator_decision_commands(command_id, process_id, base_revision, applied_revision, response_payload, created_at)`

CAS flow becomes:

1. check command ledger for `command_id`
2. if present, return stored result
3. else compare `base_revision`
4. apply or conflict
5. write command ledger row transactionally

That is real idempotency.

### C. Pull-before-push still has a race unless you fence the pull

The right pull algorithm is:

1. read current `hiwater = max(remote_change_seq)` for the domain
2. fetch rows where `cursor < change_seq <= hiwater`
3. apply in order
4. advance cursor to `hiwater`

If you page without a hi-water fence, you can skip or duplicate rows when remote changes during pagination.

For operator decisions, CAS protects correctness anyway.
For configs, hi-water pull is enough.
You do not need cross-domain serializability; you do need **per-domain monotonic pull fences**.

### D. Equal-version retries must be idempotent no-ops, not conflicts

Lost-response retry is normal. So remote apply logic must define:

* incoming version = remote version and content hash equal → **no-op success**
* incoming version = remote version but content hash differs → **hard conflict**
* incoming version = remote version + 1 → **apply**
* anything older or larger jump → **reject**

If you do not define the equal-version case, identical-envelope retries will fail spuriously.

### E. Batch boundaries should preserve local transaction atomicity, not just `stage_run_id`

`stage_run_id` is a semantic cohort. It is not always enough to reconstruct the original SQLite transaction boundary.

Add a `local_txn_id` or `commit_group_id` to every outbox row.

Then batching rules become:

* never split a `local_txn_id`
* never cross `schema_epoch`
* never cross `stage_run_id` if stage semantics require it

That preserves actual local atomicity.

### F. Epoch mismatch without reproject is an eventual-divergence bug

This one is bigger than the docs treat it.

You currently say:

* if epoch mismatches, sync pauses
* local operation continues
* outbox entries with old epoch are never sent

That means remote can permanently miss everything created during the mismatch window unless you later do a **full or table-level reproject**.

So “full rebuild not implemented” cannot stay a future nice-to-have. It is required by your current mismatch behavior.

Minimum fix:

* after epoch repair, mark a `projection_generation`
* republish current row images from local authoritative tables
* discard stale-epoch outbox entries only after successful reseed

Without that, epoch mismatch causes durable remote inconsistency.

### G. Temporary cross-stage inconsistency is acceptable, but only with visibility fences

Your question about lot extraction arriving before re-qualification is the right one.

My answer: **yes, temporary remote inconsistency is acceptable**, because the remote side is a projection, not a control plane — but only if you enforce two fences:

1. **All rows for one stage cohort plus its stage-head update land atomically together**
2. Readers use `process_stage_heads` / `pipeline_assessments` as the stable visibility fence, not raw detail joins

Do not try to make the remote portal globally serializable.
Make it causally honest.

### H. Tombstone retention is underspecified

Cursor-based pull requires tombstones to remain visible longer than the maximum expected offline window.

If you GC tombstones too early:

* offline daemon misses deletes forever

You need either:

* tombstone retention ≥ max offline duration + safety margin
* or periodic anti-entropy / full-hash diff

Right now that gap is implicit.

---

## 4) Fail-open: constitutional is fine, fail-silent is not

I agree with fail-open for this system’s core pipeline.

I do **not** agree with “the cost of blocking good data is always higher than letting bad data through.” That is false in at least four places:

1. **Irreversible downstream actions**

   * bid submission
   * supplier outreach
   * customer-facing exports
2. **Cache poisoning**

   * bad `listing_evidence`
   * bad relation edges
   * bad product decomposition reused across processes
3. **Decision poisoning**

   * operator morning queue filled with wrong “high-value” opportunities
4. **Calibration poisoning**

   * bad unvalidated cases later treated as representative live data

So the correct constitutional framing is:

> **Fail-open for availability, not fail-open for trust equivalence.**

### What to add

#### A. Validation health states

Not just per-run failure flags. A subsystem state.

Use something like:

* `healthy`
* `degraded`
* `blind`

Transition triggers:

* validator execution failure rate spike
* canary misclassification
* timeout spike
* garbage/invalid verdict rate
* basis insufficiency spike
* provider/model drift

If `blind`:

* pipeline still runs
* enforcement auto-demotes on affected slices
* UI shows explicit validation-blackout banner
* retro-validation backlog starts accumulating

#### B. Synthetic validator canaries

Run the validator path itself every 15–30 minutes with:

* known good
* known bad
* known insufficient-basis

These should use the same executor, same model family, same overlay loading, same schema validation path as production.

A broken validator should be noticed fast, not at 9 AM after 500 unverified rows.

#### C. Retroactive validation backlog

Every result that passes through due to validator failure should be queued for later backfill validation.

Prioritize by:

* high-value opportunity
* operator-reviewed processes
* slices currently under enforcement
* recent outputs

When the validator recovers:

* validate backlog
* if later rejected, mark affected process stale/suspect
* invalidate decisions based on those stage runs
* push updated status to portal

This is the missing safety valve.

#### D. Trust-state propagation

Right now unverified results flow downstream identically. That is too permissive.

You do not need to block the pipeline. You do need a trust lattice.

At minimum:

* `verified`
* `flagged`
* `unverified`
* `suspect`

And downstream behavior should depend on it.

Examples:

* manifest generation: allowed
* internal rollups: allowed, but surfaced as unverified
* decision recommendations: lowered trust
* bid workflow / external actions: require `verified` or explicit human review

That is not fail-closed. That is sane trust propagation.

#### E. Cache tainting

Do not allow unverified or suspect outputs to silently seed reusable caches at full trust.

Examples:

* `listing_evidence`
* relation cache-sharing across clusters
* mutation templates for challenge bank

Use trust metadata there too.

### The 2 AM failure scenario

If the validator breaks at 2 AM and 500 results arrive:

The operator should not just see 500 yellow badges.

They should see:

* one prominent **validation blackout** incident
* affected time window
* affected slices
* sorted trust-debt queue
* “retro-validation running” status
* automatic stale marking on any decision made during that window

That is how fail-open becomes safe enough.

---

## 5) Web portal: keep L4 read-only, but make that a security boundary, not a UI convention

Your instinct to keep the web portal read-only in L4 is right.

The current problem is that in your text it is partly a **UI statement**, not clearly a **credential and RLS statement**.

### My recommendation

For L4:

* the web portal should use **viewer-scoped credentials only**
* the browser should not possess operator mutation rights
* the portal should ideally read **push projections / summary tables only**
* remote-owned tables should be minimally exposed

That means:

* **read-only must be a security property**
* not just “there is no button”

### Why I would not allow web decisions in L4

Because that turns a visibility layer into a control plane before you’ve earned it operationally.

The risks are real:

* browser-held operator credentials
* decisioning on stale projections during degraded sync
* bypassing daemon-side review context
* weaker audit/replay context than desktop
* more CAS conflicts and confused retries from mobile/browser sessions

### What if the operator is traveling?

That is a valid product need. It just should not be smuggled into the L4 viewer.

If you later add web decisions, do it as a **separate mutation surface** with:

* server-mediated CAS
* step-up authentication
* explicit freshness proof (`remote_revision`, `manifest_hash`, `basis_hash`)
* stale-data acknowledgement if sync is degraded
* full audit log

Same CAS core, different product surface.

### One doc inconsistency to fix

Your high-level matrix says viewers don’t access remote-owned tables. Later RLS text gives viewers SELECT on remote config and decisions.

Pick one.

My recommendation:

* `shared_company_profile`: **admin + daemon only**
* `shared_source_configs`: **admin + daemon**, maybe redacted summary projection for others
* operator decision display: preferably exposed via projection / summary, not raw remote-owned table

That is cleaner and safer.

---

## 6) Relation resolution: the current design is conservative enough, but not yet robust enough

The four-way classification is good.
The hard-blocker-first structure is good.
The current weight model is acceptable as a bootstrap, but it is too global.

### On the current weights

`0.35 ref + 0.20 buyer + 0.20 title + 0.10 CPV + 0.10 date + 0.05 value`

My read:

* **Reference = 0.35** is reasonable **if** you have strong normalized reference extraction
* **Value = 0.05** is also reasonable as a default because value is often noisy across notice lifecycle
* But these weights should not be global

They should vary by:

* source pair
* notice stage pair
* relation hypothesis

Examples:

* planning vs tender vs award values drift a lot
* call-offs can share buyer/CPV/title but differ on ref semantics
* FTS vs CF duplicates may normalize references differently

### What to improve

#### A. Add cluster identity now

This is the architectural fix.

You need one of:

* `cluster_id`
* `canonical_process_id`
* or equivalent

Because late merge is not just row aliasing. It affects:

* manifests
* rollups
* operator decisions
* deep links
* cache reuse

If two already-enriched processes merge later, you need a non-destructive identity layer.

My recommendation:

* keep original `process_id` immutable
* maintain `canonical_process_id` / cluster root
* recompute rollups at cluster view
* use `process_aliases` only for redirects, not as the full merge model

#### B. Treat late merge as a decision-staleness event

If two processes merge after review:

* mark all relevant operator decisions stale
* regenerate manifest
* recompute assessment
* surface “identity changed after review” explicitly

Otherwise you will have technically correct sync and semantically wrong decisions.

#### C. Use richer features

Add:

* normalized buyer registration / authority identifier
* contact email/domain similarity
* lot-count similarity
* postcode/region
* procurement method / notice type compatibility
* reference edit distance after stripping amendment / lot suffixes
* document hash overlap if available

#### D. Learn the weights offline

Keep the rule engine and hard blockers.
But fit the soft weights from adjudicated pairs.

A simple interpretable logistic model per source-pair / stage-pair is enough.
You do not need a black box.

#### E. Group before merge

To reduce false non-merge cost without destructive auto-merge:

* group `related_not_merged` and `possible_duplicate` in UI
* share caches cautiously within the cluster
* let the operator see one cluster card with relation confidence

That cuts confusion before you solve perfect merge semantics.

---

## 7) Local SQLite as authority: resilient for availability, incomplete for durability and control

“Local SQLite is the authority” is good for uptime. It does **not** solve these failure modes:

### A. Single-device durability

If the machine dies before push:

* the org loses authoritative data

You need:

* encrypted snapshots
* WAL-aware backups
* periodic integrity checks

Local authority without backup is availability theater.

### B. Split-brain by policy, not protocol

Active-writer lease only protects remote pushing.

It does not stop:

* a second offline daemon
* a cloned SQLite file
* a backup laptop running disconnected

That creates multiple local “authorities” even if only one can write remote.

For MVP, document the operational rule:

* one bound machine per org
* others are viewer-only unless explicitly rehomed

### C. Remote config staleness is a control-plane risk

If pull breaks, the daemon continues with stale:

* source configs
* kill switches
* company profile
* decision heads

That is not always safe.

Some config is cosmetic.
Some config is effectively a safety control.

You need config classes with TTLs:

* informational config: stale okay
* operational config: warn
* safety-critical config: degrade that function if stale too long

Example: if a source is remotely disabled for legal/compliance reasons, an offline daemon should not scrape it indefinitely just because “local operation continues.”

### D. Epoch mismatch plus continued writes

Covered above, but it bears repeating:

* continuing local writes during epoch mismatch requires later reproject
* otherwise remote authority is stale forever

---

## 8) Things you did not ask about, but should resolve now

### A. Override rate is a biased metric

Operator overrides are not a clean false-positive estimate.
They are selection-biased toward visible and contested cases.

Use overrides as:

* demotion signal
* incident source
* mutation template source

But not as your only precision estimator.

### B. `basis_sufficiency` enums are inconsistent

Your docs use:

* `sufficient | partial | insufficient`
* and elsewhere `high | medium | low`

Pick one. Better: separate

* `evidence_sufficiency` (daemon capture completeness)
* `decision_sufficiency` (validator confidence to judge)

Right now that field will become muddy quickly.

### C. Redaction boundary needs a hard test harness

The local-only / summary-table split is good, but it is easy to leak:

* operator identity
* company profile
* raw supplier data
* tender document metadata
* error context

Add explicit redaction tests for every summary projection.

### D. Provenance of remote-owned config used in a run

Record the exact revisions used:

* `source_config_revision`
* `company_profile_revision` if relevant
* `operator_decision_revision` when a workflow consumes a decision

Otherwise replay and blame assignment get murky.

### E. Outbox growth and compaction

Long offline windows can explode `local_outbox`.

Because your remote side is a projection, not an event log, you can eventually coalesce superseded unsent updates by entity key in some cases.

Not a P0 correctness fix, but it will matter operationally.

---

## Bottom line

### What is right

* Per-slice enforcement is the right actuation model
* Fail-open is acceptable for pipeline availability
* Local SQLite authority is the right MVP operational center
* Outbox + pull + CAS is the right sync pattern
* Read-only web in L4 is the right product cut

### What will break if left as-is

* L4 enforcement without a promotion buffer
* Active-writer lease without fencing
* CAS without a durable command ledger
* Epoch mismatch without reproject
* Late merge without cluster identity
* Fail-open without blackout/backfill/trust propagation

### The clean implementation order from here

1. Add provisional promotion for enforcement slices
2. Add lease fencing token
3. Add CAS command ledger
4. Add hi-water fenced pull
5. Add epoch-repair reproject path
6. Switch slice key to behavior versions
7. Add validator blackout + retro-validation backlog
8. Make web read-only at credential level
9. Add cluster/canonical process identity
10. Tighten RLS and summary redaction

If you want the blunt version: **Layer 3 is close. Layer 4 is conceptually right but missing three non-negotiable distributed-systems details and one non-negotiable validation-state transition. Fix those, and the rest is tuning.**


### Integration Rules

For each proposed change from the reviewer, evaluate:

1. **Agree strongly** → Apply directly to the appropriate file (L3 or L4)
2. **Agree partially** → Apply the useful parts, note what you modified and why
3. **Disagree** → Do NOT apply. Note the disagreement and reasoning at the end of your response

### Specific Guidance

- **Validation system changes (policy engine, executor, calibration):** Acceptable for L3 (dry-run) and L4 (enforcement). Changes to the policy engine or calibration harness should be consistent across both layers (L3 introduces, L4 extends).
- **Sync protocol changes (outbox, push, pull, CAS):** Evaluate carefully for correctness. If the reviewer identifies ordering or atomicity issues, these are high-priority fixes. Changes should preserve: single-in-flight batch, identical-envelope retries, batch boundary rules.
- **Fail-open modifications:** The fail-open principle is constitutional. You can add monitoring/alerting/retroactive-validation mechanisms, but you CANNOT change fail-open to fail-closed.
- **Per-slice enforcement changes:** The per-slice concept is decided. You can adjust granularity (e.g., coarser slicing) or minimum targets, but don't remove per-slice entirely.
- **Relation resolution changes:** L3 owns relation resolution. Weight adjustments, new classification categories, or late-merge strategies are acceptable.
- **Web portal changes:** L4 web portal is read-only (ADR-008). If the reviewer proposes operator decisions from web, note this as a tension but don't implement without flagging it as a potential ADR-008 modification.
- **New RLS policies or roles:** Acceptable if justified by a security analysis.

### Preservation Rules

- Preserve fail-open as constitutional
- Preserve authority matrix (push/local/remote per table)
- Preserve web portal as read-only (ADR-008) unless explicitly flagging a change
- Preserve active-writer lease (one daemon per org)
- Preserve schema epoch guard concept
- Preserve all ADR references (especially ADR-002, ADR-004, ADR-005, ADR-008)
- Update table/file counts if entities are added

---

## Step 3: Fresh-Eyes Optimization

After completing the edits, re-read both L3 and L4 docs with fresh eyes. Check for:

1. **Validation continuity:** Does the L3 dry-run architecture cleanly extend to L4 enforcement? Are the same data structures reused? Is the transition path explicit?
2. **Sync protocol completeness:** Walk through a complete push cycle (local write → outbox → batch → RPC → ack) and a complete pull cycle (cursor → query → apply → advance). Are all error cases handled?
3. **CAS correctness:** Walk through an operator decision flow: operator approves → daemon pushes → another operator tries to override → CAS conflict → refresh → re-decide. Is every step specified?
4. **Calibration feasibility:** Given the traffic assumptions (L2 runs daily for 1 week, L3 runs unattended 5 days), how many validation samples will accumulate? Is it enough for per-slice calibration?
5. **RLS completeness:** For every table, verify the RLS role can perform the required operations. Check: Can the web viewer read all push-owned projections? Can the daemon write all push-owned tables? Can the operator update only via CAS RPC?
6. **Degraded-mode UX consistency:** The 3-state model (healthy/degraded/offline) is defined in L4. Verify it's referenced consistently in the web portal and shared component sections.

---

## Step 4: Sync Pass

1. Update `docs/strategy/cross-reference-index.md` if any new entities, protocols, or concepts were added
2. Update `docs/strategy/README.md` if L3 or L4 scope descriptions changed
3. Verify layer-strategy.md §§10-11 are consistent with any changes:
   - §10: Interface contracts (push batch RPC, CAS decision RPC, pull-before-push ordering)
   - §11: Testing evolution (test counts for L3 and L4)
4. Add cross-domain notes for any changes that affect other layers:
   - L1: if adapter trait or session lifecycle changed
   - L2: if desktop-daemon protocol or coverage metrics changed
   - L5: if validator overlay interface changed
   - L6: if bid workflow prerequisites changed

Write all updates using as-if-always language (not changelog style).
