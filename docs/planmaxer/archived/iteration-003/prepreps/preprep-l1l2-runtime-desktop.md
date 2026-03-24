# Pre-Prep: L1+L2 Daemon Runtime & Desktop Experience

## Domain: L1+L2 Runtime & Desktop
## Files This Agent Can Edit

| File | Scope | Notes |
|------|-------|-------|
| `docs/strategy/layers/layer-1-thin-local-runner.md` | Full file | L1 runtime |
| `docs/strategy/layers/layer-2-single-operator-desktop.md` | Full file | L2 desktop |
| `docs/strategy/layer-strategy.md` | §10 only (interface contracts) | Cross-layer contracts |
| `docs/strategy/cross-reference-index.md` | Full file | Navigation updates |
| `docs/strategy/README.md` | Full file | If overview changes |

**Do NOT edit:** L0, L3–L6 layer docs. If changes are needed there, add a cross-domain note at the end of the file you're editing, clearly labeled `<!-- CROSS-DOMAIN NOTE FOR L{N}: ... -->`.

---

## Step 1: Pre-Prep Research

Read these documents in order:

1. **`AGENTS.md`** — Agent guidelines, rules, project overview
2. **`docs/strategy/layers/layer-1-thin-local-runner.md`** — Read cover to cover (~1300 lines). Pay special attention to:
   - 14 Rust file specifications (entry point, adapters, sessions, jobs, artifacts, errors)
   - Session lifecycle (7 phases, transitions, cleanup verdicts)
   - Process-group supervision (setpgid, cleanup escalation, orphan sweep)
   - Run manifest system (hybrid-authored, agent_claimed vs daemon_observed)
   - Agent Adapter trait (mode selection, unified event stream)
   - Job Runner and stage tracking (stage_run_id, process_stage_heads)
   - Artifact store design
   - Error manifest system
3. **`docs/strategy/layers/layer-2-single-operator-desktop.md`** — Read cover to cover (~700 lines). Pay special attention to:
   - Tauri v2 architecture (invoke, channels, plugins)
   - QueryService (read-only SQLite, QueryEnvelope<T>)
   - DaemonBridge (UDS/named-pipe, MutationAck)
   - Channel<T> streaming (SessionFrame types)
   - Bootstrap state machine (healthy/browse-only/recovery)
   - Shared UI components (packages/ui — 5 components)
   - Manifest rendering (packages/manifests)
   - Coverage metrics and decision capture
   - Keyset pagination
4. **`docs/strategy/layer-strategy.md`** — Read §10 (interface contracts, 14 defined contracts). Note:
   - Agent Adapter trait contract
   - QueryEnvelope<T> contract
   - DaemonBridge command contract
   - Channel<T> streaming contract
   - Skill directory standard
   - Result contract format
5. **`docs/planmaxer/active/handoffs/handoff-l1l2-runtime-desktop.md`** — Read the handoff that produced the feedback

Build a deep mental model of:
- How the daemon spawns, supervises, and reaps agent sessions
- How the desktop discovers and communicates with the daemon
- How live PTY streaming reaches the desktop UI
- The boundary between QueryService (read) and DaemonBridge (write)
- Shared component architecture (packages/ui, packages/manifests)

---

## Step 2: Feedback Integration

Bluntly: the macro-architecture is right. The micro-architecture still has a few failure paths that will bite hard before Layer 3.

Keep the daemon separate. Keep PTY first-class. Keep the desktop as the operator surface. But I would not ship L1/L2 as currently specified without fixing five things first:

1. The desktop/daemon consistency model is underspecified.
2. The PTY model is missing a few L1-critical details that cannot wait for L3.
3. The adapter trait is trying to unify the wrong thing.
4. The artifact format does not match the reconnect/history UX you want.
5. The `state.db` / `artifacts.db` split weakens crash consistency more than it helps.

## Overall judgment

**Belief 1 — separate daemon from Tauri:** correct. Tauri’s own IPC model already gives you one app-internal boundary: commands for request/response and channels for ordered streaming, while the event system is explicitly not the high-throughput path. Keeping a second boundary between the Tauri bridge and the daemon over UDS/named pipe is the cleanest way to preserve daemon authority, survive desktop restarts, and avoid localhost-port baggage. ([Tauri][1])

**Belief 2 — PTY from day 1:** also correct. The mistake would be treating PTY as “just another subprocess mode.” PTY needs real terminal/session semantics, careful fd hygiene, replayable capture, and a cleanup model that assumes descendants will misbehave.

**Belief 3 — desktop as control panel, not browser:** directionally correct, but L2 does **not yet** deliver the full promise. It gives read views, triggers, and monitoring. It does **not yet** give complete operational control, because it is missing command fencing, replayable live-session reconnect, input/cancel paths for active PTY sessions, and a realistic daemon-crash recovery story.

---

## The biggest issues you did not ask about

### 1) The two-database split is a trap

Your docs put hot state in `state.db`, session/error/document metadata in `artifacts.db`, and the heavy bytes on the filesystem. That split buys very little because the big payloads are already outside SQLite, but it costs you atomicity. SQLite WAL improves reader/writer concurrency, but it does **not** give cross-database atomicity as a set; even attached-database transactions are atomic per database, not across all databases together. ([SQLite][2])

For L1/L2, I would collapse **all metadata** back into **one SQLite database** and keep only the large immutable payloads on disk:

* keep `agent_session_logs`, `error_manifests`, `raw_documents` metadata in `state.db`
* keep PTY/stdout/stderr bytes, manifests, checkpoint files, downloaded documents themselves in the artifact store

That removes an ugly class of “job row committed, session metadata missing” and “error manifest exists but stage status did not move” crash splits.

### 2) Your docs are too optimistic about daemon-crash recovery for PTY sessions

Desktop restart while the daemon stays alive is fine.

Daemon crash while a PTY session is alive is **not** the same thing. In the current design the PTY master lives inside the daemon process. If the daemon dies, that master fd dies with it. That means “re-attach stream from last seq” is usually not a real option for an active PTY session. The honest recovery path is:

* on daemon restart, detect sessions that were `spawned|active|terminating`
* inspect whether descendants survived
* kill what is still alive
* synthesize an error manifest such as `AGENT_DAEMON_CRASH_DURING_SESSION`
* never pretend the PTY stream was resumed

That is a spec bug today. The docs currently conflate **desktop reconnect** with **daemon restart**. They must be treated differently.

### 3) `snapshotSeq` is underspecified, which will create ghost states

Right now `QueryEnvelope<T>.snapshotSeq` exists, but the docs never make it the control-plane fence. That means a mutation can succeed, the live stream can show progress, and the list/detail page can still show an older DB snapshot with no principled way to know when the write became visible.

You need a **global local commit fence**:

* every daemon write transaction increments a monotonic `commit_seq`
* every `MutationAck` returns `visible_after_seq`
* every `QueryEnvelope<T>` returns the DB snapshot’s `snapshotSeq`
* the UI only treats a mutation as visible when `snapshotSeq >= visible_after_seq`

Without that, the desktop will feel haunted.

### 4) The artifact store format and the L2 viewer model do not line up

L1 describes monolithic files like `raw-pty.zst`. L2 assumes chunked replay, gap recovery, lazy scrollback, and resumable streaming. Those are not the same design.

If you want:

* reconnect after desktop restart
* gap recovery after slow consumer
* lazy history fetch
* “live tail” handing off to “historical transcript”

then capture must be **chunked from day 1**. Not later.

Use something like:

```text
artifacts/sessions/YYYY/MM/<session_id>/
  stream/
    000001.pty.zst
    000002.pty.zst
    000003.pty.zst
  stream.index.jsonl
  result.json
  run-manifest.md
  checkpoint-001.json
```

Each chunk needs:

* `first_seq`, `last_seq`
* byte length
* wall timestamp range
* SHA-256

Then `SessionViewer` can recover from `gap` by asking for chunks covering `fromSeq..toSeq`.

### 5) The spec defers a few PTY items to L3 that really belong in L1

These cannot wait:

* controlling-terminal/session setup
* `CLOEXEC` hygiene
* EIO/EOF drain behavior
* wait/reap correctness
* chunked capture
* interactive stop/input/resize API

Those are not “hardening extras.” They are the difference between “Experiment 4 is real” and “Experiment 4 passed on a good week.”

---

## Daemon ↔ desktop protocol review

### UDS / named pipe is the right IPC

Keep it.

The clean pattern is:

* **Tauri webview ↔ Tauri Rust**: `invoke()` for snapshots/mutations, `Channel<T>` for frontend streaming
* **Tauri Rust ↔ daemon**: UDS on macOS/Linux, named pipe on Windows
* **No HTTP**
* **No embedded web server**

That matches the actual data types: JSON-ish commands and acknowledgements on one side, replayable ordered stream frames on the other. Tauri’s own docs support commands for request/response and channels for ordered streaming; the event system is explicitly not meant for bigger/high-throughput messages. ([Tauri][1])

### What the current protocol is missing

You need four concrete things:

**A. A runtime descriptor file**

When the daemon is fully ready, atomically write:

```json
{
  "instanceId": "uuid",
  "pid": 12345,
  "socketPath": "...",
  "protocolVersion": 3,
  "schemaEpoch": 1,
  "stateRoot": "...",
  "startedAt": "...",
  "ready": true
}
```

Store it next to:

* `daemon.lock`
* `daemon.sock`

This solves discovery, stale sockets, version checks, and multiple-daemon confusion.

**B. An OS-level single-writer lock**

`executors` is not enough for local uniqueness. You need a real local lock file. The daemon must acquire it before opening the write connection and before binding the socket.

**C. A real handshake**

First message should be `hello`:

* desktop version
* supported protocol range
* expected schema epoch
* state root id
* last seen daemon instance id

Response should include:

* selected protocol version
* daemon version
* daemon instance id
* schema epoch
* current `commit_seq`
* current health state
* whether mutations are allowed
* active sessions summary

**D. A better ack**

Today `MutationAck` is too vague. Replace it with something like:

```ts
type MutationAck = {
  accepted: boolean
  idempotencyKey: string
  durability: 'queued' | 'committed'
  visibleAfterSeq?: number
  jobId?: string
  sessionId?: string
  conflictReason?: string
}
```

`accepted: true` should never mean “I have it in RAM.” It should mean at least “durably queued in SQLite.”

### Discovery and security

Use a **pathname UDS**, not an abstract Linux socket. Pathname sockets let you use directory ownership/permissions. On Linux, pathname socket creation honors directory permissions, but the manual also warns not to rely on socket-file permissions alone for portable security. On macOS, `getpeereid()` gives you reliable peer credentials on a UNIX-domain stream socket; on Linux you can verify the peer via UNIX-socket credentials as well. ([Man7][3])

So the rule should be:

* runtime dir `0700`
* socket inside that dir
* verify peer uid on connect
* reject any peer not matching the daemon/desktop user

### Desktop startup ordering

For L2:

* desktop tries handshake
* if no daemon, desktop launches detached daemon once
* desktop retries handshake
* only then falls back to browse-only

Do **not** require manual daemon startup for L2. That undermines the control-panel promise.

### Desktop close / reopen behavior

This part is fine **if the daemon stayed alive**:

* close desktop: drop subscriptions, daemon continues
* reopen desktop: read descriptor, handshake, pull active-session summary, subscribe from last seen seq
* no SQLite read-lock issue **if** QueryService transactions are short-lived

SQLite WAL is a good fit for this, but long-lived readers will starve checkpoints and let the WAL grow. ([SQLite][4])

---

## PTY session management audit

### `setpgid` alone is not enough

Your docs talk a lot about process groups. For a real interactive PTY you also need **session** and **controlling terminal** semantics.

On Linux and BSD/macOS, `openpty()` gives you the master/slave pair, and `login_tty()` / `forkpty()` do the crucial extra work: new session, make the slave the controlling terminal, hook it to stdin/stdout/stderr. `setsid()` creates a new session and new process group. Window-size changes are propagated with `TIOCSWINSZ`, which sends `SIGWINCH` to the foreground process group. Negative `pid` in `kill()` targets a process group. ([Man7][5])

So in `pty/session.rs` / `sessions/pty.rs`, the spawn contract should be:

* create PTY
* create **new session**
* establish controlling terminal
* create/record `sid` as well as `pid` and `pgid`
* only then `exec`

If you only do `openpty + setpgid`, some interactive CLIs will still behave wrong.

### Pull these PTY items into L1

**EIO / EOF handling**

* PTY master `EIO` is not automatically an error
* treat it as hangup / slave-side closure signal
* finalize only after exit observed or drain timeout

**Close-on-exec hygiene**

* master fd and all control fds not intended for the child must be `CLOEXEC`
* otherwise EOF may never arrive because a grandchild inherited the master

**Drain state**

* keep the 7 public phases
* add internal substate like `pty_hangup_seen`, `draining`, `drain_deadline`

**Stop sequence**

1. adapter in-band interrupt if available
2. write `^C` / equivalent if appropriate
3. `SIGINT` to pgid
4. `SIGTERM` to pgid
5. `SIGKILL` to pgid
6. orphan sweep
7. verdict

### Wait/reap correctness

Do not use `kill(pid, 0)` as your notion of process completion. POSIX explicitly points out that an application must use `waitpid()`/`waitid()` for child termination detection, not `kill()`. ([Man7][6])

So `sessions/reaper.rs` should own:

* `SIGCHLD` handling
* `waitpid(-1, WNOHANG)` drain loop
* `pid -> session_id` map
* explicit orphan/leak scan
* final `cleanup_report.json`

### Linux vs macOS

This is where the spec is too hand-wavy.

**Linux**

* optional `PR_SET_CHILD_SUBREAPER` is genuinely useful for double-fork escapees and daemonized grandchildren; the man page explicitly calls out session-management frameworks as a use case. ([Man7][7])
* later, `systemd-run --scope` is worth considering when you want cgroup containment

**macOS**

* no Linux subreaper equivalent
* leak detection must come from process inspection and conservative session cleanup
* peer verification uses `getpeereid()` over UDS, not Linux socket options. ([Apple Developer][8])

### One subtle implementation hazard

Because your daemon is multithreaded Tokio, be careful with any custom PTY spawn path that does real work after `fork()`. In a multithreaded program, the child may safely call only async-signal-safe functions until `execve()`. If the PTY crate cannot express the exact pre-exec logic you need, use a tiny spawn helper executable rather than growing custom child-side Rust logic inside the daemon. ([Man7][9])

### Terminal geometry

Set a **stable default geometry** in L1. I would use `120x40`.

Then:

* PTY geometry is for terminal/UI behavior
* browser viewport is a separate explicit browser-launch setting
* do **not** let browser viewport silently depend on PTY size

Also add:

* `resize_pty(session_id, cols, rows)` in the bridge
* only for attached sessions
* if never resized, keep the same geometry forever so transcripts are reproducible

### Chrome stress case

If the agent launches Chrome and Chrome forks helpers:

* many children will stay in the same session/pgid if launched normally
* some helpers can escape
* process-group kill is necessary, not sufficient

So the real answer to your Chrome stress test is:

* `kill(-pgid, …)` first
* descendant sweep second
* per-session browser profile dir
* leak verdict if anything survives
* later on Linux, subreaper or cgroup scope

---

## Adapter trait review

The current trait shape is the weakest part of the design.

### Where it leaks

1. **It unifies PTY bytes and contract JSON into one event enum.**
   That is the wrong abstraction boundary. Lossless transport and semantic progress are different lanes.

2. **It makes the adapter own result validation.**
   That is wrong. `result.schema.json` is a skill contract, not a vendor contract. Validation belongs in `agents/contracts.rs`, not in `codex.rs` or `claude.rs`.

3. **It makes the adapter build a concrete `Command`.**
   That leaks spawn mechanics into vendor logic. PTY/session setup should stay centralized in the runtime.

4. **It allows unsafe fallback from contract mode to PTY.**
   A skill declared as `contract_json` should not silently slide into PTY unless the skill explicitly allows that fallback.

### What to unify instead

Unify the **control plane**, not the transport.

Use two lanes:

* **Output lane**: lossless raw transport for UI/forensics
  `Pty(Bytes) | Stdout(Bytes) | Stderr(Bytes)`

* **Control lane**: semantic daemon events
  `Started | Checkpoint | Progress | AuthChallenge | Warning | FinalResultReady | Exited`

That lets PTY stay raw and lets contract mode stay structured without pretending they are the same thing.

### Concrete trait shape

`crates/daemon/src/adapters/trait.rs` should move toward something like this:

```rust
pub enum OutputFrame {
    Pty(bytes::Bytes),
    Stdout(bytes::Bytes),
    Stderr(bytes::Bytes),
}

pub enum ControlEvent {
    Started { pid: u32, pgid: u32, sid: u32, mode: ExecutionMode },
    Checkpoint { seq: u32, path: PathBuf },
    Progress { completed: u32, total: Option<u32> },
    AuthChallenge { kind: AuthChallengeKind, detail: String },
    Warning { code: String, detail: String },
    FinalResultReady { path: PathBuf },
    Exited { status: ExitStatus },
}

pub struct LaunchSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub env: BTreeMap<OsString, OsString>,
    pub cwd: PathBuf,
}

#[async_trait]
pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> &'static str;

    async fn probe(&self, ctx: &ProbeCtx) -> Result<CapabilityReport>;

    fn resolve_mode(
        &self,
        skill: &SkillManifest,
        caps: &CapabilityReport,
    ) -> Result<ExecutionMode>;

    fn build_launch_spec(
        &self,
        req: &SpawnRequest,
        mode: ExecutionMode,
    ) -> Result<LaunchSpec>;

    fn parser(&self, mode: ExecutionMode) -> Box<dyn AdapterParser>;
}

#[async_trait]
pub trait AdapterParser: Send {
    async fn on_output(&mut self, frame: &OutputFrame) -> Result<Vec<ControlEvent>>;
    async fn finish(&mut self) -> Result<Vec<ControlEvent>>;
}
```

Then put these elsewhere:

* PTY/session spawn in `pty/session.rs`
* result schema validation in `agents/contracts.rs`
* OQG in daemon pipeline code
* lifecycle/reap in `sessions/lifecycle.rs` + `sessions/reaper.rs`

### Interaction profile change

Replace the current fallback logic with:

* `PtyRequired`
* `ContractRequired`
* `ContractPreferred { allow_pty_fallback: bool }`

Anything else will hide capability drift.

### PTY mode still needs structured checkpoints

This part of your docs is right, and it should become stricter:

PTy transcripts should **never** be the authoritative source of checkpoints or final results.

For PTY skills, structured outputs should arrive through:

* a sidecar result file path
* a checkpoint directory
* or a dedicated structured pipe/fd

The transcript is for humans and forensics. The checkpoint/result side channel is for the daemon.

---

## Read-your-writes and consistency model

Direct SQLite reads are fine here. WAL mode gives you concurrent readers and writer with snapshot isolation. The footgun is long-lived readers, not the model itself. ([SQLite][4])

What you need is a **commit-fence model**.

### Recommended model

**Daemon side**

* every successful write transaction increments `commit_seq`
* after commit, emit `db.commit { commitSeq, touchedScopes }`

**Ack side**

* every mutation returns `visibleAfterSeq`

**Query side**

* every page query runs inside **one read transaction**
* every envelope returns the snapshot’s `snapshotSeq`

**Frontend side**

* optimistic UI can show “queued/committed”
* authoritative view flips only when `snapshotSeq >= visibleAfterSeq`

That gives you read-your-writes without routing reads through the daemon.

### `snapshotSeq` source

Do **not** use `PRAGMA data_version` itself as the user-visible snapshot clock. SQLite is explicit that `data_version` is only meaningful when you compare values from the **same connection** at two different points in time. It is excellent for invalidation, not as a global fence. Also set QueryService connections to `PRAGMA query_only=ON`. ([SQLite][10])

So:

* `data_version` = QueryService invalidation hint
* `commit_seq` = daemon-authored global visibility fence

### `QueryEnvelope<T>` should change

I would change it to:

```ts
type QueryEnvelope<T> = {
  data: T
  snapshotSeq: number
  snapshotAt: string
}
```

And move live health out of it:

```ts
type ControlPlaneState = {
  daemonInstanceId: string
  daemonVersion: string
  protocolVersion: number
  schemaEpoch: number
  storage: 'ready' | 'repairing' | 'unreadable' | 'schema_mismatch'
  daemon: 'starting' | 'ready' | 'degraded' | 'unavailable' | 'incompatible'
  control: 'full' | 'limited' | 'read_only'
  load: 'normal' | 'busy' | 'saturated'
  reasons: string[]
}
```

Right now `sourceMeta` in `QueryEnvelope` mixes two clock domains:

* DB snapshot data
* live daemon status

That is an abstraction leak.

### Live → historical session handoff

Add `archive_seq` to the session lifecycle event stream.

Then:

* `SessionRegistry` owns live session truth while active
* when daemon emits `Archived { archiveSeq }`, the UI waits until a query returns `snapshotSeq >= archiveSeq`
* only then does `SessionViewer` switch to historical mode

That removes the “session finished but is missing from Sessions page” seam.

---

## Bootstrap state machine

Three flattened states are not enough.

You need either:

* a fourth user-facing state, or
* better, a structured health model with derived UX

### The states you actually have

At minimum, L2 already needs to distinguish:

* **ready/full-control**
* **ready/control-limited**
  daemon up, DB fine, but daemon overloaded / reconciling / refusing new work
* **browse-only**
* **incompatible**
  protocol mismatch, schema mismatch, wrong daemon build still running
* **recovery-required**

“Daemon reachable but unhealthy” is not the same as “daemon unreachable.”

### Startup behavior I would specify

`apps/desktop/src-tauri/main.rs` should do this:

1. verify DB readability
2. compare schema epoch
3. read descriptor file
4. attempt handshake
5. if daemon missing, launch detached daemon once
6. retry handshake
7. if still unavailable, browse-only
8. if protocol/schema incompatible, incompatible/recovery-required
9. if daemon reports `recovering_reap` or `saturated`, enter control-limited

### One hard truth for the spec

Do not promise “reattach session streams after daemon restart” for PTY sessions in the current design. Promise it only for:

* desktop restart
* bridge restart
* webview reload

Not for daemon crash.

---

## Does L2 actually deliver “desktop = control panel”?

Not yet. It is close, but it still leaves too much operational control outside the desktop.

### Commands missing from `DaemonBridge`

Add these in `apps/desktop/src-tauri/daemon_bridge.rs`:

* `stop_session(session_id, mode)`
* `send_session_input(session_id, bytes)`
* `resize_session(session_id, cols, rows)`
* `retry_job(job_id | process_id, stage?)`
* `restart_daemon()`
* `reopen_error(error_manifest_id)` or `rerun_from_error(...)`

Without at least stop/input/retry/restart, the operator still needs terminal-level escape hatches.

### `SessionViewer.tsx` should be optionally interactive

For auth prompts and stuck interactive flows:

* allow keyboard input/paste to PTY
* allow resize
* show lifecycle state and cleanup verdict live
* clearly indicate when viewer is historical and input is disabled

That still satisfies “control panel, not browser.” It is a managed console, not a general browser shell.

### Browser-auth challenge handling

There is one gap even with PTY input: some MFA/CAPTCHA flows are browser-only.

So for L2, add a minimal intervention path:

* `open_managed_browser_session(session_id)` or
* “reveal profile path” + clear instructions + reattach button

Otherwise `AGENT_AUTH_CHALLENGE` is visible but not actionable.

### Daemon operations page

You also need a small daemon status screen:

* current daemon instance id/version
* socket path
* DB path
* artifact root
* WAL size / disk usage
* startup/recovery reason
* buttons: restart daemon, open logs, open artifact root

That is still operator surface, not infrastructure bloat.

---

## Shared UI and manifest boundary

The boundary is basically right. The leak is in the prop shapes.

### `packages/ui`

**Good**

* `ContractCardFrame` with slot composition
* `ManifestViewer`
* `PipelineBadge`

**Needs refactor**

* `SyncAgeBadge` should not take raw desktop/web control-plane metadata
* give it a tiny surface-neutral VM instead:

```ts
type FreshnessBadgeVM = {
  label: string
  tone: 'good' | 'warn' | 'bad' | 'muted'
  tooltip: string
}
```

Desktop and web compute that separately.

### `packages/manifests`

The deterministic renderer is the right authority.

But `decisionBasisHash` should **not** be owned by the manifest renderer template or by an L1 skill. It should be daemon-authored from a canonical serialization of operator-relevant fields. The renderer consumes the already-computed hash. Otherwise you will eventually get hash drift across Rust/TS/agent implementations.

### Security issue you did not call out

`ManifestViewer.tsx` is rendering agent-authored Markdown. Treat that as untrusted input:

* sanitize Markdown
* disable raw HTML
* strip `javascript:` / unsafe `file://` links
* never render arbitrary embedded HTML from agent text

Same for error manifests.

And for `SessionViewer.tsx`:

* sanitize dangerous OSC sequences
* be careful with OSC 8 hyperlinks and clipboard-related terminal escapes

That is a real desktop/webview security boundary.

### Testing shared components

Use fixture-based view-model tests, not backend-coupled tests:

* `packages/ui/fixtures/*.ts`
* snapshot/visual tests at the package level
* surface-specific integration tests stay in `apps/desktop` and later `apps/web`

That keeps sharing maintainable.

---

## Specific file-level changes

### Daemon

**`crates/daemon/src/main.rs`**

* acquire exclusive `daemon.lock`
* write `daemon.info.json` atomically only after ready
* distinguish desktop reconnect vs daemon restart recovery
* reconcile `running` jobs/sessions on boot and kill surviving PTY descendants

**`crates/daemon/src/db/schema.rs` / `db/queries.rs`**

* add a local-only `state_meta` / `commit_clock` table
* add unique idempotency keys for bridge commands
* pin page queries to current `process_stage_heads`
* collapse `artifacts.db` metadata into `state.db`

**`crates/daemon/src/jobs/runner.rs`**

* return `visible_after_seq`
* commit durable job/intent rows before ack
* emit `db.commit` after each successful transaction
* make `expected_input_basis_hash` rejection explicit and observable

**`crates/daemon/src/jobs/stage_tracker.rs`**

* expose a transaction guard for set-replacement vs additive stages
* stage outputs + head advance + visibility fence should be one atomic unit

**`crates/daemon/src/adapters/trait.rs`**

* split transport from control plane
* remove result validation from adapters
* return `LaunchSpec`, not a ready-made `Command`
* add stop/input/resize hooks

**`crates/daemon/src/adapters/codex.rs` and `claude.rs`**

* emit vendor-specific health hints, not final health verdicts
* support explicit in-band interrupt
* no silent contract→PTY fallback unless skill allows it

**`crates/daemon/src/pty/session.rs` / `sessions/pty.rs`**

* switch to real session + controlling-terminal setup
* record `sid`
* make PTY writer/read side explicit
* add `write_input()` and `resize()`

**`crates/daemon/src/sessions/reaper.rs`**

* make this L1-critical, not L3-only hardening
* own `SIGCHLD` / `waitpid` drain loop
* Linux: optional subreaper
* emit `cleanup_report.json`

**`crates/daemon/src/artifacts/store.rs`**

* chunk output from day 1
* keep `stream.index`
* write temp files then atomic rename
* incremental hash while capturing, not after

**`crates/daemon/src/errors/manifest.rs`**

* include daemon instance id, `commit_seq`, `sid`, last good checkpoint, and leak report reference

### Desktop Rust bridge

**`apps/desktop/src-tauri/main.rs`**

* replace flat bootstrap enum with structured health
* auto-launch daemon once if absent
* incompatible daemon build should trigger restart-or-readonly path, not vague recovery

**`apps/desktop/src-tauri/query_service.rs`**

* open read-only connections
* `PRAGMA query_only=ON`
* one read transaction per page query
* `snapshotSeq` from commit clock
* `PRAGMA data_version` for invalidation, not user-visible fence. ([SQLite][10])

**`apps/desktop/src-tauri/daemon_bridge.rs`**

* handshake with version/schema negotiation
* peer credential verification
* persistent idempotency keys
* replayable subscription API with `fromSeq`
* separate backpressure path for command RPC vs stream subscriptions

### Desktop frontend

**`src/stores/SessionRegistry.ts`**

* key entries by `daemonInstanceId + sessionId`
* track `lastSeq`, `gapState`, `archiveSeq`, `reattachable`

**`src/stores/SessionBuffer.ts`**

* PTY mode stores bytes, not lines
* contract mode can store line/text frames
* explicit gap markers

**`src/pages/SessionViewer.tsx`**

* interactive PTY mode
* resend from `lastSeq`
* historical fallback from artifact chunks
* sanitize terminal escapes

**`src/pages/Sessions.tsx`**

* add stop / force-kill / reopen manifest actions

**`src/pages/Errors.tsx`**

* add retry/reopen/reveal-session actions

**`src/App.tsx`**

* consume `ControlPlaneState` separately from `QueryEnvelope`

### Shared packages

**`packages/ui/src/SyncAgeBadge.tsx`**

* use `FreshnessBadgeVM`

**`packages/ui/src/ManifestViewer.tsx`**

* sanitize Markdown and links

**`packages/manifests/src/render-opportunity.ts`**

* render deterministically from typed inputs only
* do not own canonical basis-hash generation

---

## A few non-obvious stress tests you need

* desktop closes and reopens while daemon stays alive and PTY session is active
* daemon crashes while PTY session is active
* child exits before PTY master reports hangup
* PTY master gets `EIO` before child exit is reaped
* master fd accidentally inherited by grandchild, preventing EOF
* lost `MutationAck` response after DB commit, retried with same idempotency key
* slow `SessionViewer` consumer causing gap and replay
* long-running QueryService read starving WAL checkpoints
* auth challenge requiring PTY input
* auth challenge requiring managed browser intervention
* keyset pagination under live updates with immutable sort key vs mutable sort key
* agent-authored Markdown containing raw HTML / link abuse
* ANSI output containing hyperlink/clipboard escape sequences

---

## Priority order

If I were sequencing the fixes, I would do them in this order:

1. **Collapse metadata into one DB and define `commit_seq` / `visible_after_seq`.**
2. **Fix PTY spawn semantics: session + controlling tty + `CLOEXEC` + reaper.**
3. **Change adapters to control-plane + transport-plane split.**
4. **Make capture chunked and replayable from day 1.**
5. **Replace the flat bootstrap enum with structured health.**
6. **Add desktop stop/input/retry/restart controls.**
7. **Harden shared renderers against untrusted Markdown and terminal output.**

The single change that removes the most future pain is this: **make the daemon publish a real local commit fence and make the desktop wait for it.** That one decision will clean up your mutation UX, your live→historical handoff, your reconnect story, and most of the apparent “split-brain” behavior before it happens.

[1]: https://v2.tauri.app/concept/inter-process-communication/ "https://v2.tauri.app/concept/inter-process-communication/"
[2]: https://sqlite.org/wal.html "https://sqlite.org/wal.html"
[3]: https://man7.org/linux/man-pages/man7/unix.7.html "https://man7.org/linux/man-pages/man7/unix.7.html"
[4]: https://sqlite.org/isolation.html "https://sqlite.org/isolation.html"
[5]: https://man7.org/linux/man-pages/man3/openpty.3.html "https://man7.org/linux/man-pages/man3/openpty.3.html"
[6]: https://man7.org/linux/man-pages/man3/kill.3p.html "https://man7.org/linux/man-pages/man3/kill.3p.html"
[7]: https://man7.org/linux/man-pages/man2/PR_SET_CHILD_SUBREAPER.2const.html "https://man7.org/linux/man-pages/man2/PR_SET_CHILD_SUBREAPER.2const.html"
[8]: https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/getpeereid.3.html "https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/getpeereid.3.html"
[9]: https://man7.org/linux/man-pages/man2/fork.2.html?utm_source=chatgpt.com "fork(2) - Linux manual page"
[10]: https://sqlite.org/pragma.html "https://sqlite.org/pragma.html"


### Integration Rules

For each proposed change from the reviewer, evaluate:

1. **Agree strongly** → Apply directly to the appropriate file (L1 or L2)
2. **Agree partially** → Apply the useful parts, note what you modified and why
3. **Disagree** → Do NOT apply. Note the disagreement and reasoning at the end of your response

### Specific Guidance

- **Daemon architecture changes (new modules, modified lifecycle):** Acceptable for L1. Update the Rust file listing and module descriptions. Ensure any new files follow the existing naming pattern.
- **Desktop architecture changes (new components, modified protocol):** Acceptable for L2. Update the desktop file listing. Ensure shared components remain truly data-agnostic.
- **IPC protocol changes (UDS/named-pipe alternatives):** Evaluate carefully. The current choice was made for simplicity. If the reviewer proposes a more robust alternative, assess whether the complexity is warranted for MVP.
- **Bootstrap state machine additions:** Acceptable if the new states address real failure modes. Each state must have clear entry/exit conditions and user-visible behavior.
- **PTY hardening recommendations:** L1 describes basic PTY management; L3 adds hardening. If the reviewer suggests hardening that should be in L1 (not deferred to L3), evaluate whether it's critical for first live operation.
- **Adapter trait design:** If the reviewer proposes a concrete trait signature, evaluate whether it correctly abstracts PTY and contract modes. Include it in L1 if it's well-designed.

### Preservation Rules

- Preserve all ADR references (especially ADR-003, ADR-004, ADR-005)
- Preserve the 7-phase session lifecycle (add phases if needed, don't remove)
- Preserve the cleanup verdict taxonomy (clean/forced/leaked)
- Preserve the run manifest hybrid-authoring model
- Preserve the desktop-daemon separation (separate OS processes)
- Preserve the QueryService/DaemonBridge split
- Update file counts if files are added or removed

---

## Step 3: Fresh-Eyes Optimization

After completing the edits, re-read both L1 and L2 docs with fresh eyes. Check for:

1. **L1 → L2 interface completeness:** Every mutation the desktop needs to trigger (L2) must be supported by the daemon (L1). List all DaemonBridge commands and verify L1 has handlers for each.
2. **Session lifecycle → streaming consistency:** The 7-phase lifecycle (L1) must produce events that Channel<T> streaming (L2) can consume. Verify the SessionFrame types cover all lifecycle transitions.
3. **Error manifest → Errors page alignment:** Error manifests generated by L1 must contain fields that the L2 Errors page can display. Verify the schema is sufficient.
4. **Cleanup verdict → UI display:** Cleanup verdicts (clean/forced/leaked) should be displayable in the Sessions page. Are there UI states for each verdict?
5. **Forward-compatibility for L3:** L1 stubs scheduling (CLI only; cron in L3) and resource management (no quotas; arbiter in L3). Verify the stubs don't preclude L3 additions.
6. **Shared component portability:** packages/ui components must work in both desktop (L2) and web (L4) contexts. Verify they have no desktop-specific dependencies.

---

## Step 4: Sync Pass

1. Update `docs/strategy/cross-reference-index.md` if any new components, interfaces, or concepts were added
2. Update `docs/strategy/README.md` if the L1 or L2 scope descriptions changed
3. Verify layer-strategy.md §10 interface contracts are consistent with any changes to L1 or L2
4. Add cross-domain notes for any changes that affect other layers:
   - L0: if schema expectations changed
   - L3: if session lifecycle or adapter trait changed in ways L3 must accommodate
   - L4: if shared component interfaces changed in ways the web portal must respect

Write all updates using as-if-always language (not changelog style).
