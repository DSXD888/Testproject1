#!/usr/bin/env python3
"""Tests for discover_peers(...) in bin/br-claim.

Runs against the live mcp-agent-mail HTTP server on 127.0.0.1:8765. Each test
uses a unique throwaway project_key so it does not pollute any real project's
archive. The throwaway project directories are intentionally left in /tmp.

Run:
    python3 -m unittest bin.tests.test_peer_discovery
"""

import importlib.machinery
import importlib.util
import contextlib
import io
import os
import socket
import tempfile
import unittest
from datetime import datetime, timedelta, timezone


REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
BR_CLAIM = os.path.join(REPO_ROOT, "bin", "br-claim")
MCP_URL = os.environ.get("AGENT_MAIL_MCP_URL", "http://127.0.0.1:8765/mcp/")


def _load_module(path, name):
    loader = importlib.machinery.SourceFileLoader(name, path)
    spec = importlib.util.spec_from_loader(name, loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


def _agentmail_reachable():
    try:
        with socket.create_connection(("127.0.0.1", 8765), timeout=1):
            return True
    except OSError:
        return False


class StartMailClaimTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.br_claim = _load_module(BR_CLAIM, "br_claim_start_mail_under_test")

    def test_recent_other_start_is_not_stale(self):
        now = datetime(2026, 5, 23, 8, 0, tzinfo=timezone.utc)
        _, other = self.br_claim._latest_start_mail_claim(
            [{
                "subject": "[test-bead] Start: work",
                "from": "BlueLake",
                "created_ts": (now - timedelta(minutes=10)).isoformat(),
            }],
            "test-bead",
            "GrayMouse",
            now=now,
        )
        self.assertEqual(other["sender"], "BlueLake")
        self.assertFalse(other["stale"])

    def test_stale_other_start_can_be_reclaimed_by_open_bead_path(self):
        now = datetime(2026, 5, 23, 8, 0, tzinfo=timezone.utc)
        _, other = self.br_claim._latest_start_mail_claim(
            [{
                "subject": "[test-bead] Start: work",
                "from": "BlueLake",
                "created_ts": (now - timedelta(hours=5)).isoformat(),
            }],
            "test-bead",
            "GrayMouse",
            now=now,
        )
        self.assertEqual(other["sender"], "BlueLake")
        self.assertTrue(other["stale"])

    def test_self_start_remains_idempotent_marker(self):
        now = datetime(2026, 5, 23, 8, 0, tzinfo=timezone.utc)
        prior_self, other = self.br_claim._latest_start_mail_claim(
            [{
                "subject": "[test-bead] Start: work",
                "from": "GrayMouse",
                "created_ts": now.isoformat(),
            }],
            "test-bead",
            "GrayMouse",
            now=now,
        )
        self.assertTrue(prior_self)
        self.assertIsNone(other)


@unittest.skipUnless(_agentmail_reachable(), "mcp-agent-mail not reachable on 127.0.0.1:8765")
class DiscoverPeersTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.br_claim = _load_module(BR_CLAIM, "br_claim_under_test")
        cls.token = cls.br_claim.resolve_bearer_token()

    def setUp(self):
        # Resolve symlinks (macOS tempfile.mkdtemp uses /var/folders/.../T/ which is
        # a symlink to /private/var/folders/...). mcp-agent-mail 0.2.51 refuses to
        # create archive directories through symlinked paths and enters degraded
        # state, which then makes list_agents/list_contacts fail across the whole
        # daemon (not just this project). Resolving the symlink up front bypasses
        # that. Real project paths (e.g. ~/Desktop/testproject1)
        # don't hit this — only test-tempdir paths do.
        raw_tmp = tempfile.mkdtemp(prefix="test-peer-discovery-")
        self.project_key = os.path.realpath(raw_tmp)
        self._mcp_call("ensure_project", {
            "human_key": self.project_key,
        })

    def _mcp_call(self, tool_name, arguments):
        stderr = io.StringIO()
        try:
            with contextlib.redirect_stderr(stderr):
                return self.br_claim.mcp_call(MCP_URL, self.token, tool_name, arguments)
        except SystemExit:
            msg = stderr.getvalue()
            lowered = msg.lower()
            if (
                "degraded_read_only" in lowered
                or "not write-safe" in lowered
                or "mutating mcp calls are refused" in lowered
            ):
                self.skipTest(f"mcp-agent-mail is not write-safe: {msg.strip()}")
            raise

    def _register(self, suffix):
        res = self._mcp_call("register_agent", {
            "project_key": self.project_key,
            "program": "peer-discovery-test",
            "model": "test",
            "task_description": f"peer-discovery test agent {suffix}",
        })
        name = res.get("name") if isinstance(res, dict) else None
        self.assertIsNotNone(name, f"register_agent returned no name: {res!r}")
        self._mcp_call("set_contact_policy", {
            "project_key": self.project_key,
            "agent_name": name,
            "policy": "open",
        })
        return name

    def _handshake(self, requester, target):
        self._mcp_call("macro_contact_handshake", {
            "project_key": self.project_key,
            "requester": requester,
            "target": target,
            "auto_accept": True,
        })

    def test_T1_contacts_present_returns_contacts(self):
        a = self._register("T1a")
        b = self._register("T1b")
        self._handshake(a, b)
        peers, source = self.br_claim.discover_peers(
            MCP_URL, self.token, self.project_key, a,
        )
        self.assertEqual(source, "contacts")
        self.assertIn(b, peers)
        self.assertNotIn(a, peers)

    def test_T2_no_contacts_list_agents_live(self):
        a = self._register("T2a")
        b = self._register("T2b")
        peers, source = self.br_claim.discover_peers(
            MCP_URL, self.token, self.project_key, a,
            max_idle_seconds=3600,
        )
        self.assertEqual(source, "list_agents")
        self.assertIn(b, peers)
        self.assertNotIn(a, peers)

    def test_T3_stale_only_raises_or_returns_live(self):
        a = self._register("T3a")
        self._register("T3b")
        try:
            peers, source = self.br_claim.discover_peers(
                MCP_URL, self.token, self.project_key, a,
                max_idle_seconds=0,
            )
            self.assertNotEqual(peers, [a])
            self.assertEqual(source, "list_agents")
        except self.br_claim.NoPeersError:
            pass

    def test_T4_solo_raises(self):
        a = self._register("T4a")
        with self.assertRaises(self.br_claim.NoPeersError) as cm:
            self.br_claim.discover_peers(
                MCP_URL, self.token, self.project_key, a,
            )
        msg = str(cm.exception)
        self.assertIn("macro_contact_handshake", msg)
        self.assertIn("--solo", msg)

    def test_T5_allow_solo_returns_self(self):
        a = self._register("T5a")
        peers, source = self.br_claim.discover_peers(
            MCP_URL, self.token, self.project_key, a,
            allow_solo=True,
        )
        self.assertEqual(peers, [a])
        self.assertEqual(source, "solo")

    def test_T6_dedup_and_self_filter(self):
        a = self._register("T6a")
        b = self._register("T6b")
        self._handshake(a, b)
        peers, _ = self.br_claim.discover_peers(
            MCP_URL, self.token, self.project_key, a,
        )
        self.assertEqual(peers.count(b), 1)
        self.assertNotIn(a, peers)


if __name__ == "__main__":
    unittest.main()
