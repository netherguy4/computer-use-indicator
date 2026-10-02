"""Runs the proxy against a fake MCP server and checks passthrough and events."""

import json
import os
import socket
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROXY = os.path.join(ROOT, "computer-use-indicator-proxy")
FAKE_SERVER = (
    "import sys, json\n"
    "for line in sys.stdin:\n"
    "    m = json.loads(line)\n"
    "    if 'id' in m:\n"
    "        print(json.dumps({'jsonrpc': '2.0', 'id': m['id'], 'result': {}}), flush=True)\n"
)


def call(i, name, **arguments):
    return {"jsonrpc": "2.0", "id": i, "method": "tools/call", "params": {"name": name, "arguments": arguments}}


class ProxyTest(unittest.TestCase):
    def run_proxy(self, requests, **env):
        with tempfile.TemporaryDirectory() as runtime:
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
            sock.bind(os.path.join(runtime, "computer-use-indicator.sock"))
            sock.settimeout(0.5)
            stdin = "".join(json.dumps(r) + "\n" for r in requests).encode()
            proc = subprocess.run(
                [sys.executable, PROXY, "Claude", "--", sys.executable, "-c", FAKE_SERVER],
                input=stdin,
                capture_output=True,
                timeout=30,
                env={**os.environ, "XDG_RUNTIME_DIR": runtime, "LANG": "C", **env},
            )
            events = []
            while True:
                try:
                    events.append(json.loads(sock.recv(4096)))
                except socket.timeout:
                    break
        replies = [json.loads(line)["id"] for line in proc.stdout.splitlines()]
        return proc.returncode, replies, events

    def test_passthrough_and_events(self):
        code, replies, events = self.run_proxy([
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            call(2, "click", x=100, y=200),
            call(3, "screenshot"),
            call(4, "press_key", key="ctrl+l"),
            call(5, "type_text", text="hello"),
            call(6, "list_windows"),
        ])
        self.assertEqual(code, 0)
        self.assertEqual(replies, [1, 2, 3, 4, 5, 6])
        self.assertEqual(events[0], {"agent": "Claude", "tool": "click", "label": "click", "x": 100, "y": 200})
        self.assertEqual(events[1], {"hide": True}, "captures hide a visible overlay first")
        keys = next(e for e in events if e.get("tool") == "press_key")
        self.assertEqual(keys["keys"], ["ctrl", "l"])
        typed = next(e for e in events if e.get("tool") == "type_text")
        self.assertEqual(typed["text"], "hello")
        self.assertIn("looking at the screen", [e.get("label") for e in events])
        self.assertNotIn("list_windows", [e.get("tool") for e in events])

    def test_hidden_text_and_relative_coordinates(self):
        _, _, events = self.run_proxy(
            [call(1, "type_text", text="secret"), call(2, "click", x=5, y=5, relative=True)],
            COMPUTER_USE_INDICATOR_HIDE_TEXT="1",
        )
        self.assertEqual(events[0]["text"], "••••••")
        self.assertNotIn("x", events[1], "window-relative points cannot be placed")

    def test_overlay_absent(self):
        with tempfile.TemporaryDirectory() as runtime:
            proc = subprocess.run(
                [sys.executable, PROXY, "Claude", "--", sys.executable, "-c", FAKE_SERVER],
                input=(json.dumps(call(1, "click", x=1, y=1)) + "\n").encode(),
                capture_output=True,
                timeout=30,
                env={**os.environ, "XDG_RUNTIME_DIR": runtime},
            )
        self.assertEqual([json.loads(l)["id"] for l in proc.stdout.splitlines()], [1])


if __name__ == "__main__":
    unittest.main()
