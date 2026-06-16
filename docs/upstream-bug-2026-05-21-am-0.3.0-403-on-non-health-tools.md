> **Filing note:** `gh auth status` reports *not logged in* on this machine — operator must file manually (or run `gh auth login` first).
>
> Suggested command once authenticated:
> ```
> gh issue create -R Dicklesworthstone/mcp_agent_mail_rust \
>   -F docs/upstream-bug-2026-05-21-am-0.3.0-403-on-non-health-tools.md \
>   -t "0.3.0: HTTP 403 on all MCP tools except health_check"
> ```

# 0.3.0: HTTP 403 on all MCP tools except `health_check`

## Summary

`mcp-agent-mail` **0.3.0** (release tag `v0.3.0`, 2026-05-20) returns **HTTP 403 Forbidden** for every MCP `tools/call` invocation **except** `health_check`. The rejection happens *before* the MCP routing layer — no daemon log entry is produced — and it occurs **even when the server is started with `--no-auth`**, so it is not a bearer-token problem. `initialize` and `tools/list` still succeed. This breaks all multi-agent coordination workflows on 0.3.0; only health probes work.

**Workaround:** downgrade to **0.2.51** (`am self-update --version 0.2.51` or equivalent). 0.2.54 has a *different* unrelated regression (sticky `recovery.mode=degraded_read_only` latch), so 0.2.51 is the safest known-good.

## Environment

| | |
|---|---|
| Binary | `am 0.3.0` (upgraded from 0.2.54 → 0.3.0 via `am self-update` on 2026-05-20) |
| Platform | macOS Darwin 24.5.0, arm64 |
| Service | LaunchAgent `com.agent-mail`, bound `127.0.0.1:8765` |
| Storage root | `~/.mcp_agent_mail_git_mailbox_repo` (healthy: 2 projects, 26 agents, 306 messages, `integrity_check=ok`) |
| Auth config | `HTTP_BEARER_TOKEN` in `~/.config/mcp-agent-mail/config.env`, also synced to `~/.claude/settings.json` and `~/.claude.json` via `am setup run` |

The underlying mailbox repo is intact — this is purely an HTTP-layer regression.

## Reproduction

All curl invocations use the **same** bearer token and **same** headers. Only the JSON-RPC `method`/`params` change.

### 1. `health_check` works (HTTP 200)

```bash
TOKEN=REDACTED-loopback-bearer-token

curl -sS -X POST http://127.0.0.1:8765/mcp/ \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Authorization: Bearer $TOKEN" \
  --data '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"health_check","arguments":{}}}'
# → HTTP 200, returns health_level=green, semantic_readiness=ok, full payload
```

### 2. Any other tool returns 403 (same token, same headers)

```bash
curl -sS -X POST http://127.0.0.1:8765/mcp/ \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Authorization: Bearer $TOKEN" \
  --data '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_agents","arguments":{"project_key":"~/Desktop/testproject1"}}}'
# → HTTP 403, {"detail":"Forbidden"}
```

Confirmed 403 on:

- `list_agents`
- `ensure_project`
- `list_contacts`
- `send_message` (inferred — observed via wrapper failures, same path)

### 3. `--no-auth` does NOT help — strongest evidence this is not an auth issue

```bash
launchctl bootout gui/$UID/com.agent-mail
am serve-http --host 127.0.0.1 --port 8765 --no-tui --no-auth &
# wait for healthz

curl -sS -X POST http://127.0.0.1:8765/mcp/ \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  --data '{"jsonrpc":"2.0","id":99,"method":"tools/call","params":{"name":"list_agents","arguments":{"project_key":"~/Desktop/testproject1"}}}'
# → HTTP 403, {"detail":"Forbidden"}
```

### 4. Origin header variations — all 403

Tried `Origin: 127.0.0.1`, `localhost`, `http://127.0.0.1`, `http://localhost`, `null`, `*`, and omitted entirely. All returned 403.

### 5. MCP handshake works

```bash
curl -sS -X POST http://127.0.0.1:8765/mcp/ \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Authorization: Bearer $TOKEN" \
  --data '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
# → HTTP 200, serverInfo: {"name":"mcp-agent-mail","version":"0.3.0"}
```

So `initialize` and `tools/list` succeed; `tools/call` is selectively blocked for everything except `health_check`.

## Daemon log behavior

`~/Library/Logs/agent-mail/stderr.log` shows **no entry** corresponding to the 403 responses. The rejection appears to be returned by an HTTP middleware layer **before** the MCP router dispatches the call, which makes this impossible to diagnose from logs alone.

## Predecessor versions

| Version | Behavior |
|---|---|
| **0.2.51** | Known-good (documented in our internal 2026-05-05 post-incident handoff). |
| **0.2.54** | Does **not** have the 403 issue, but has an unrelated regression: sticky `recovery.mode=degraded_read_only` latch that persists even after a successful `am doctor reconstruct`. |
| **0.3.0** | This bug — 403 on every non-`health_check` tool, including under `--no-auth`. |

## Why this matters

This blocks **all** multi-agent coordination workflows since 0.3.0. Agents using mcp-agent-mail cannot:

- discover peers (`list_agents`, `list_contacts`)
- register themselves (`ensure_project`, `register_agent`, `create_agent_identity`)
- send or reply to mail (`send_message`, `reply_message`)
- reserve files (`file_reservation_paths`, `release_file_reservations`)
- run any macro flow (`macro_start_session`, `macro_prepare_thread`, etc.)

Only `health_check` probing works, which means the server *looks* up from outside but is functionally inert for its actual purpose.

## Hypothesis

Some HTTP middleware shipped in 0.3.0 — most likely a new CSRF guard, a tightened CORS policy, or a tool-scope/allowlist policy — is rejecting `tools/call` requests for every tool name except a hardcoded `health_check` exception. It runs ahead of the auth check (hence `--no-auth` doesn't bypass it) and ahead of the MCP router (hence no log entry).

## Asks

1. **Acknowledge the regression** so other operators hitting this know to downgrade rather than chasing token/config issues.
2. **Identify the offending middleware** in 0.3.0 (CSRF? CORS? new per-tool policy?) and either:
   - **(a)** revert it / loosen it back to 0.2.x behavior for loopback bearer-authenticated requests, OR
   - **(b)** document the required client-side opt-in (header, session step, capability declaration) that 0.3.0 now expects.
3. **Ship 0.3.1** with the fix so downgrading is no longer the recommended path.
4. (Nice-to-have) **Emit a log line** for HTTP-layer rejections so future middleware bugs are debuggable without source-diving.

Happy to test a candidate fix against this environment and report back.
