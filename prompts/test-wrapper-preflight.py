#!/usr/bin/env python3
"""Smoke test for bin/br-claim and bin/br-complete preflight_health().

Spins up a local HTTP server on 127.0.0.1:18765 that returns a canned MCP
response, then invokes the wrappers with AGENT_MAIL_MCP_URL pointed at it.

Two cases covered:
  RED  — health_level=red + 'archive inventory is ahead' detail
         → wrapper MUST exit non-zero before any mutating call,
           and MUST echo recovery.next_action verbatim in stderr.
  GREEN — health_level=green
         → preflight is silent, wrapper proceeds past it (and then fails on
           something unrelated, which is acceptable — we just need to confirm
           the preflight didn't fire).

Stdlib only. No external test runner.
"""

import http.server
import json
import os
import socketserver
import subprocess
import sys
import threading
import time

HOST = "127.0.0.1"
PORT = 18765
URL = f"http://{HOST}:{PORT}/mcp/"

RED_RESPONSE = {
    "status": "error",
    "health_level": "red",
    "semantic_readiness": {
        "status": "fail",
        "detail": (
            "archive inventory is ahead of the sqlite index "
            "(archive projects=1, agents=3, messages=1, latest_id=1, "
            "db projects=1, agents=0, messages=0, max_id=0)"
        ),
    },
    "recovery": {
        "mode": "degraded_read_only",
        "next_action": (
            "Run `am doctor reconstruct --dry-run` to inspect archive-backed "
            "recovery, then `am doctor reconstruct --yes` if needed"
        ),
        "recovery_lock_active": False,
    },
}

GREEN_RESPONSE = {
    "status": "ok",
    "health_level": "green",
    "semantic_readiness": {"status": "ok", "detail": "aligned"},
}


def make_handler(payload):
    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, fmt, *args):  # silence
            pass

        def do_POST(self):
            length = int(self.headers.get("Content-Length", "0"))
            _ = self.rfile.read(length)
            envelope = {
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "content": [{"type": "text", "text": json.dumps(payload)}]
                },
            }
            body = json.dumps(envelope).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    return Handler


class ReusableTCPServer(socketserver.TCPServer):
    allow_reuse_address = True


def run_server(payload):
    srv = ReusableTCPServer((HOST, PORT), make_handler(payload))
    t = threading.Thread(target=srv.serve_forever, daemon=True)
    t.start()
    return srv


def invoke(wrapper, bead, env):
    return subprocess.run(
        ["python3", wrapper, bead, "--dry-run"],
        capture_output=True, text=True, env=env, timeout=15,
    )


def main():
    base_env = os.environ.copy()
    base_env["AGENT_MAIL_MCP_URL"] = URL
    base_env["AGENT_MAIL_NAME"] = "TealHarbor"
    base_env["AGENT_MAIL_PROJECT_KEY"] = "/tmp/fake-project"
    base_env["AGENT_MAIL_BEARER_TOKEN"] = "test-token"

    failures = 0

    # ── RED case ────────────────────────────────────────────────────────
    srv = run_server(RED_RESPONSE)
    try:
        for wrapper in ("bin/br-claim", "bin/br-complete"):
            r = invoke(wrapper, "fake-1", base_env)
            print(f"\n[RED  ] {wrapper} exit={r.returncode}")
            print(f"  stderr (last 6 lines):")
            for line in (r.stderr.strip().splitlines() or [""])[-6:]:
                print(f"    {line}")
            if r.returncode == 0:
                print("  FAIL: expected non-zero exit on red preflight")
                failures += 1
                continue
            ok_signature = "archive inventory is ahead" in r.stderr
            ok_action = "am doctor reconstruct" in r.stderr
            ok_degraded = "degraded_read_only" in r.stderr
            if not (ok_signature and ok_action and ok_degraded):
                print(
                    f"  FAIL: missing expected message bits "
                    f"(signature={ok_signature}, action={ok_action}, degraded={ok_degraded})"
                )
                failures += 1
            else:
                print("  PASS: refused with verbatim recovery.next_action echoed")
    finally:
        srv.shutdown()
        srv.server_close()

    # ── GREEN case ─────────────────────────────────────────────────────
    time.sleep(0.2)
    srv = run_server(GREEN_RESPONSE)
    try:
        for wrapper in ("bin/br-claim", "bin/br-complete"):
            r = invoke(wrapper, "definitely-not-a-real-bead-id-xyz", base_env)
            print(f"\n[GREEN] {wrapper} exit={r.returncode}")
            print(f"  stderr (last 3 lines):")
            for line in (r.stderr.strip().splitlines() or [""])[-3:]:
                print(f"    {line}")
            # We expect non-zero because the bead doesn't exist — but the
            # error must NOT be the preflight error.  Anything that mentions
            # 'br show' or 'br-claim:' / 'br-complete:' followed by a
            # non-preflight error is fine.
            if "archive inventory is ahead" in r.stderr or "degraded_read_only" in r.stderr:
                print("  FAIL: preflight fired on green server")
                failures += 1
            else:
                print("  PASS: preflight silent on green; later step failed as expected")
    finally:
        srv.shutdown()
        srv.server_close()

    print()
    if failures:
        print(f"TEST RESULT: {failures} FAILURE(S)")
        sys.exit(1)
    print("TEST RESULT: all checks passed")


if __name__ == "__main__":
    main()
