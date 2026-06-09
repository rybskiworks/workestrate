#!/usr/bin/env python3
"""Fake OpenAI-compatible provider for the ai-workbench POC.

Listens on 127.0.0.1:8081. Logs every request's Authorization header
and full body to var/log/fake-provider.log (and stderr). Returns a
minimal but valid OpenAI-compatible /v1/chat/completions response.

This stands in for a real provider (OpenAI, Anthropic, etc.). In
production, the egress boundary in microsandbox would rewrite the
dummy Authorization header to the real provider key before the
request reached the real provider; in this POC, the egress proxy
does the rewrite, and the fake provider receives the rewritten
request.
"""
import json
import os
import sys
import time
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

LISTEN_HOST = os.environ.get("FAKE_PROVIDER_BIND", "127.0.0.1")
LISTEN_PORT = int(os.environ.get("FAKE_PROVIDER_PORT", "8081"))
LOG_PATH = os.environ.get("FAKE_PROVIDER_LOG", "var/log/fake-provider.log")

os.makedirs(os.path.dirname(LOG_PATH), exist_ok=True)

_lock = threading.Lock()


def log_request_line(line: str) -> None:
    with _lock:
        with open(LOG_PATH, "a", encoding="utf-8") as f:
            f.write(line + "\n")
    sys.stderr.write(line + "\n")


class Handler(BaseHTTPRequestHandler):
    server_version = "FakeProvider/0.1"

    def _log_incoming(self) -> None:
        auth = self.headers.get("Authorization", "<missing>")
        body_len = int(self.headers.get("Content-Length", "0") or "0")
        log_request_line(
            f"[{time.strftime('%Y-%m-%dT%H:%M:%S')}] {self.command} {self.path} "
            f"Authorization={auth!r} Content-Length={body_len} "
            f"From={self.client_address[0]}:{self.client_address[1]}"
        )

    def do_GET(self):
        self._log_incoming()
        if self.path == "/v1/models":
            payload = {
                "object": "list",
                "data": [
                    {
                        "id": "fake-gpt-4o",
                        "object": "model",
                        "created": int(time.time()),
                        "owned_by": "fake-provider",
                    }
                ],
            }
            self._json(200, payload)
            return
        if self.path == "/healthz":
            self._json(200, {"ok": True})
            return
        self._json(404, {"error": "not found", "path": self.path})

    def do_POST(self):
        self._log_incoming()
        length = int(self.headers.get("Content-Length", "0") or "0")
        raw = self.rfile.read(length) if length else b""
        try:
            body = json.loads(raw.decode("utf-8")) if raw else {}
        except Exception:
            body = {"_raw": raw.decode("utf-8", errors="replace")}
        log_request_line(f"  body: {json.dumps(body)[:500]}")

        if self.path == "/v1/chat/completions":
            user_msg = ""
            try:
                user_msg = body["messages"][-1]["content"]
            except Exception:
                pass
            response = {
                "id": f"chatcmpl-fake-{int(time.time() * 1000)}",
                "object": "chat.completion",
                "created": int(time.time()),
                "model": body.get("model", "fake-gpt-4o"),
                "choices": [
                    {
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": (
                                f"[fake-provider echo] you said: {user_msg!r}. "
                                f"I received Authorization="
                                f"{self.headers.get('Authorization', '<missing>')!r}"
                            ),
                        },
                        "finish_reason": "stop",
                    }
                ],
                "usage": {
                    "prompt_tokens": max(1, len(user_msg) // 4),
                    "completion_tokens": 20,
                    "total_tokens": max(21, len(user_msg) // 4 + 20),
                },
            }
            self._json(200, response)
            return

        self._json(404, {"error": "not found", "path": self.path})

    def _json(self, status: int, payload: dict) -> None:
        data = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, fmt, *args):
        # Quiet the default per-request stderr noise; we already log.
        return


def main() -> int:
    srv = ThreadingHTTPServer((LISTEN_HOST, LISTEN_PORT), Handler)
    log_request_line(
        f"fake-provider listening on http://{LISTEN_HOST}:{LISTEN_PORT} "
        f"(log -> {LOG_PATH})"
    )
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        log_request_line("fake-provider shutting down")
    finally:
        srv.server_close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
