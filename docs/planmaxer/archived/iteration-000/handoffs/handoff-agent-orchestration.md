# Handoff: Agent Orchestration & PTY Runtime

## Expert Framing

You are a **systems reliability engineer** with deep experience in process supervision, terminal emulation, and daemon design. You have built or contributed to systems like process supervisors (systemd, supervisord), terminal multiplexers (tmux, screen), or container runtimes. You understand the real-world failure modes of long-running background processes on developer workstations. You know what it takes to make a daemon run reliably every day without operator intervention.

Draw from real-world systems: launchd/systemd service management, tmux/screen PTY handling, the portable-pty and pty-process Rust crates, Tokio's process and signal handling, and the failure patterns of CI runners that manage headless browser sessions.

## Mission

ULTRATHINK and be meticulous.

You are reviewing the **agent orchestration and PTY runtime layer** of a UK public procurement intelligence platform. This layer is the critical infrastructure — the entire data pipeline (procurement scraping, supplier price enrichment, bid automation) runs through agent sessions orchestrated by a Rust daemon on the operator's local machine.

The core value proposition: a Rust daemon manages headless PTY sessions that run AI agents (Claude/Codex CLIs) with source-specific skills (markdown playbooks) to scrape procurement data, supplier pricing, and automate bid request workflows. The daemon handles scheduling (cron + manual triggers), health monitoring, full terminal output capture, and error manifest generation.

**Don't just answer my questions — the questions I DON'T KNOW to ask are more important than the ones I do.**

## User's Strong Beliefs (Engage With These)

1. **"PTY-first from day 1 is non-negotiable for robust daily pipeline operations."** The owner chose PTY over simpler non-interactive child processes (`codex exec`, `claude --print`) because PTY provides real terminal semantics. Challenge or strengthen this: is PTY complexity justified for every agent interaction, or could a hybrid approach (PTY for interactive flows, simpler modes for deterministic tasks) reduce blast radius?

2. **"Full terminal output capture is critical infrastructure, not a nice-to-have."** Every byte of agent output is captured and stored. Challenge: what's the realistic storage and parsing cost? When does raw terminal output become noise vs. signal? Is there a better tracing model?

3. **"The skill system (markdown playbooks) makes source maintenance easier than API client packages."** Skills are the primary data retrieval mechanism — the agent reads a SKILL.md and follows it to scrape a website. Challenge: what happens to skill reliability at scale (10+ skills, daily runs, concurrent sessions)? What's the failure correlation when agent CLIs update?

## Essential Context

**What the project is:** A greenfield procurement intelligence platform that discovers UK public contracts, estimates landed cost from Chinese/global suppliers, and surfaces profitable arbitrage opportunities. The entire data pipeline runs through AI agents on the operator's local machine.

**Key terms:**
- **Skill:** A markdown file (SKILL.md) that tells the agent how to navigate a specific website and extract data via browser automation.
- **Daemon:** A Rust background process that manages the agent lifecycle — scheduling, PTY creation, health monitoring, log capture, SQLite storage, Supabase sync.
- **PTY session:** A headless pseudo-terminal that provides real terminal semantics to the agent CLI. The daemon creates the master/slave pair, the agent runs in the slave, the daemon reads from the master.
- **Error manifest:** A structured error record + markdown artifact generated when pipeline steps fail, providing enough context to debug.

**Technical constraints:**
- Runs on the operator's macOS/Linux workstation (not a server fleet)
- Must handle both Claude CLI and Codex CLI (different tools, same PTY model)
- Agent CLIs are third-party — their behavior, flags, and output format can change
- Single-daemon architecture (one daemon per machine for MVP)
- Local SQLite for all persistent state

## Current Architecture

### PTY Session Lifecycle
```
Daemon creates PTY pair (master/slave)
  → Agent CLI spawned in slave PTY with skill loaded
  → Daemon reads master side for output capture
  → Health monitor watches for:
      - Output timeout (AGENT_STALLED)
      - Re-login prompts (AGENT_SESSION_EXPIRED)
      - API errors (AGENT_INTERNAL_ERROR)
      - Skill failures (AGENT_SKILL_FAILED)
  → On completion: parse structured result envelope, store in SQLite
  → On failure: create error manifest entry + markdown artifact
```

### Daemon Components
- **PTY Session Manager** — create, monitor, manage headless PTY sessions
- **Cron Scheduler** — local cron expressions trigger agent jobs
- **Job Runner** — claim jobs from SQLite queue, execute via PTY
- **Log/Trace Capture** — capture full terminal output, parse into structured traces
- **Health Monitor** — detect stalls, expired sessions, errors
- **Sync Engine** — push results to Supabase after completion

### Agent CLI Launchers
```rust
// Separate launchers for Claude and Codex
crates/daemon/src/agents/codex.rs  // Codex PTY launcher
crates/daemon/src/agents/claude.rs // Claude PTY launcher
```

### Structured Trace Model
```json
{
  "steps": [
    {"type": "navigate", "url": "...", "timestamp": "..."},
    {"type": "extract", "count": 12, "entity": "notices", "timestamp": "..."},
    {"type": "tool_call", "tool": "browser_click", "selector": "...", "timestamp": "..."},
    {"type": "error", "message": "Element not found", "step": 4, "timestamp": "..."}
  ]
}
```

### Health Detection Matrix
| Signal | Detection | Response |
|--------|-----------|----------|
| No output for N seconds | Configurable timeout | AGENT_STALLED, kill session |
| Login/auth prompt in output | Pattern match | AGENT_SESSION_EXPIRED, alert operator |
| API error pattern | Pattern match | AGENT_INTERNAL_ERROR, capture context |
| Agent exits with error | Exit code + output | AGENT_SKILL_FAILED, capture skill path |

### Concurrency Model
- Single-writer SQLite (one daemon)
- Multiple concurrent PTY sessions (capped)
- Job queue with lease-based claiming

## Tensions to Resolve

### Tension 1: PTY Universality vs. Mode-Appropriate Execution
**The conflict:** The architecture mandates PTY for ALL agent interactions. But some tasks (e.g., a simple HMRC API-driven qualification step that the agent executes) might work fine with `claude --print` (non-interactive, simpler, less overhead). PTY adds real complexity: ANSI escape sequence handling, terminal size negotiation, signal forwarding, zombie process cleanup.

**Current resolution:** PTY for everything, justified by "real terminal semantics."

**Why it might be wrong:** If 60% of agent tasks are deterministic and non-interactive, you're paying PTY overhead for no benefit. A stalled non-interactive process is also easier to detect and kill than a stalled PTY session.

**Stress test:** An agent skill that only calls APIs (no browser automation) — does it need a PTY? What about a skill that runs for 30 seconds vs. one that runs for 30 minutes?

### Tension 2: Agent CLI Fragility vs. Architecture Stability
**The conflict:** The daemon deeply integrates with third-party agent CLIs (Claude, Codex). These tools are actively evolving — their flags, output formats, authentication flows, and error messages can change without warning.

**Current resolution:** Abstract behind launcher modules (`agents/codex.rs`, `agents/claude.rs`).

**Why it might be wrong:** Abstraction doesn't prevent breakage — it just localizes it. A major Claude CLI update that changes how tools are invoked or how output is structured could break the structured trace parser, the health detector, and the result envelope parser simultaneously.

**Stress test:** Claude CLI ships a new version that changes its output format from text to a structured JSON stream. How much of the daemon needs to change? What if Codex is deprecated and replaced by a new tool?

### Tension 3: Terminal Output as Truth vs. Structured Agent Contracts
**The conflict:** The architecture captures raw terminal output and then parses it into structured traces. This is inherently fragile — you're parsing human-readable text to extract machine-readable state.

**Current resolution:** "Structured result envelope (JSON payload or artifact reference)" that the daemon parses.

**Why it might be wrong:** The plan mentions both raw terminal parsing AND structured result envelopes but doesn't clearly specify which is the source of truth. If the agent outputs a JSON result AND terminal text, which do you trust? If the JSON is malformed but the terminal shows success, what happens?

**Stress test:** An agent produces valid JSON output at step 5, then crashes at step 8 with partial output. The health monitor sees the crash but the result parser sees valid JSON. What's the correct job state?

### Tension 4: Single-Machine Daemon vs. Realistic Availability
**The conflict:** The entire pipeline depends on a daemon running on the operator's laptop/workstation. Laptops sleep, restart, lose network, and run out of battery.

**Current resolution:** "Freshness SLA depends on the daemon being online" + job queue in SQLite.

**Why it might be wrong:** For a daily procurement intelligence pipeline where notices expire, "daemon was offline" is a real operational risk. The architecture has no fallback execution model.

**Stress test:** Operator goes on vacation for a week. Laptop is closed. What happens to the pipeline? What happens to time-sensitive contract notices?

### Tension 5: Concurrency Limits on Workstations
**The conflict:** Running multiple concurrent PTY sessions with browser automation agents on a developer workstation is resource-intensive. Each agent may spawn a browser instance.

**Current resolution:** "Daemon concurrency caps on PTY sessions" (not specified further).

**Why it might be wrong:** The plan lists 7+ supplier sources and 3+ procurement sources. A full daily pipeline could need 10+ agent sessions. On a workstation that's also running Tauri desktop, a browser, and normal dev tools, this could saturate CPU/RAM.

**Stress test:** Full daily run: 3 procurement ingestion skills + 4 supplier enrichment skills + 2 qualification skills running concurrently. How does the daemon decide priority? What's the memory footprint?

## What I Don't Want (Anti-Patterns)

- **Don't suggest switching to a cloud-hosted execution model.** Local execution is a design constraint, not a cost optimization.
- **Don't suggest replacing agent skills with traditional API clients.** The skill-based approach is an architectural decision.
- **Don't propose a microservices decomposition of the daemon.** It's a single Rust binary for good reason.
- **Don't recommend deferring PTY to later.** The owner has made this decision. Instead, help make it work well.

## What I Do Want

1. **Concrete recommendations for PTY session lifecycle management** — what does robust session cleanup look like in Rust? How do you prevent zombie processes? How do you handle signals?

2. **A strategy for agent CLI version resilience** — how should the daemon absorb agent CLI changes without cascading failures?

3. **An opinion on the structured trace parsing approach** — is parsing terminal output the right model, or should the architecture mandate a structured output contract with agents?

4. **Practical concurrency and resource management guidance** — what's a realistic concurrency model for a workstation daemon?

5. **Ideas I haven't asked about** — failure modes I haven't considered, patterns from production daemon systems that apply here, edge cases in PTY management that will bite.

6. **Recommendations in git-diff style** — reference specific sections of the research document (sections 4.9, 4.10, 5.2, 5.6, 5.11, 6.5, 6.6) when proposing changes.

## Context Documents to Attach

When sending this to a reviewer, attach:
- The full research document: `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
- Pay special attention to: Sections 4.9 (Agent Orchestration), 4.10 (Skills), 5.2 (Components), 5.6 (Ingestion Data Flow), 5.11 (Error Handling), 6.5 (Testing Strategy), 6.6 (Observability)
