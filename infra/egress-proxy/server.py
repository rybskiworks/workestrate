#!/usr/bin/env python3
"""Egress proxy for the ai-workestrator POC (fallback for microsandbox).

This proxy is the functional stand-in for microsandbox's egress layer
in the production design. It enforces three things:

  1. ALLOWLIST: only requests targeting hosts in EGRESS_ALLOWLIST are
     forwarded. Everything else is denied (HTTP 403 + audit log).
  2. SECRET INJECTION: incoming requests carry the dummy key
     `FAKE_PROVIDER_DUMMY_KEY` in the Authorization header. The proxy
     rewrites the header to the real key
     (FAKE_PROVIDER_REAL_KEY) BEFORE forwarding, so the real key
     never appears in the isolated LiteLLM process. The real key
     is read from the HOST environment by the proxy at startup and
     held only by the proxy.
  3. AUDIT: every decision (allow/deny, header rewrite) is logged
     to var/log/egress-proxy.log and stderr.

In production, microsandbox's egress layer would do this inside the
microVM boundary (see
https://docs.microsandbox.dev/core-concepts/secret-injection). The
proxy is the POC fallback because the sandbox's host requires
glibc >= 2.39 and KVM, neither of which is available in this
environment (see README.md for the full list of constraints).
"""
import argparse
import http.client
import json
import os
import re
import sys
import threading
import time
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

LISTEN_HOST = os.environ.get("EGRESS_PROXY_BIND", "127.0.0.1")
LISTEN_PORT = int(os.environ.get("EGRESS_PROXY_PORT", "8082"))
UPSTREAM_BASE = os.environ.get("EGRESS_UPSTREAM_BASE", "http://127.0.0.1:8081")
LOG_PATH = os.environ.get("EGRESS_PROXY_LOG", "var/log/egress-proxy.log")

# The dummy key the isolated LiteLLM process is configured to use.
DUMMY_KEY = os.environ.get(
    "FAKE_PROVIDER_DUMMY_KEY", "dummy-key-injected-at-egress"
)
# The real key. Read from the HOST environment only. Never logged
# in plaintext.
REAL_KEY = os.environ.get("FAKE_PROVIDER_REAL_KEY", "")

DEFAULT_ALLOWLIST = "127.0.0.1:8081,localhost:8081"
ALLOWLIST_RAW = os.environ.get("EGRESS_ALLOWLIST", DEFAULT_ALLOWLIST)

DENY_BY_DEFAULT = os.environ.get("EGRESS_DENY_BY_DEFAULT", "1") != "0"

os.makedirs(os.path.dirname(LOG_PATH), exist_ok=True)
_lock = threading.Lock()


def _allowlist_hosts() -> set:
    return {h.strip().lower() for h in ALLOWLIST_RAW.split(",") if h.strip()}


ALLOWLIST = _allowlist_hosts()


def log(line: str) -> None:
    with _lock:
        with open(LOG_PATH, "a", encoding="utf-8") as f:
            f.write(line + "\n")
    sys.stderr.write(line + "\n")


class Handler(BaseHTTPRequestHandler):
    server_version = "EgressProxy/0.1"

    def _resolve_target(self):
        explicit = self.headers.get("X-Egress-Target", "").strip()
        if explicit:
            return ("explicit", explicit, self.path)
        m = re.match(r"^/_proxy/([^/]+)(/.*)$", self.path)
        if m:
            host = m.group(1)
            new_path = m.group(2) or "/"
            return ("_proxy", host, new_path)
        u = urllib.parse.urlparse(UPSTREAM_BASE)
        return ("default", f"{u.hostname}:{u.port or 80}", self.path)

    def _is_allowed(self, hostport: str) -> bool:
        if not DENY_BY_DEFAULT:
            return True
        target = hostport.lower().strip("[]")
        return target in ALLOWLIST

    def _redact_key(self, value: str) -> str:
        if not value:
            return "<empty>"
        if value == DUMMY_KEY:
            return f"<dummy,len={len(value)}>"
        if value == REAL_KEY:
            return f"<real,len={len(value)}>"
        return f"<other,len={len(value)}>"

    def _forward(self, hostport: str, path: str, body: bytes) -> None:
        if ":" in hostport:
            host, port_s = hostport.rsplit(":", 1)
            port = int(port_s)
        else:
            host, port = hostport, 80
        out_headers = {}
        skip = {
            "host",
            "connection",
            "proxy-connection",
            "keep-alive",
            "transfer-encoding",
            "te",
            "trailers",
            "x-egress-target",
            "content-length",
        }
        auth_seen = None
        for k, v in self.headers.items():
            lk = k.lower()
            if lk in skip:
                continue
            if lk == "authorization":
                auth_seen = v
                continue
            out_headers[k] = v
        if auth_seen is None:
            log(
                f"[{self.log_date_time_string()}] WARN no Authorization header "
                f"on {self.command} {path} -> {hostport}"
            )
            self._json(401, {"error": "missing Authorization header"})
            return
        if auth_seen.startswith("Bearer "):
            presented = auth_seen[len("Bearer "):]
        else:
            presented = auth_seen
        if presented == DUMMY_KEY and REAL_KEY:
            out_headers["Authorization"] = f"Bearer {REAL_KEY}"
            log(
                f"[{self.log_date_time_string()}] INJECT "
                f"{self.command} {path} -> {hostport} "
                f"auth_in={self._redact_key(presented)} "
                f"auth_out={self._redact_key(REAL_KEY)}"
            )
        elif presented == REAL_KEY:
            out_headers["Authorization"] = auth_seen
            log(
                f"[{self.log_date_time_string()}] PASSTHRU-WARN "
                f"{self.command} {path} -> {hostport} "
                f"auth_in={self._redact_key(presented)}"
            )
        else:
            out_headers["Authorization"] = auth_seen
            log(
                f"[{self.log_date_time_string()}] PASSTHRU-OTHER "
                f"{self.command} {path} -> {hostport} "
                f"auth_in={self._redact_key(presented)}"
            )
        try:
            conn = http.client.HTTPConnection(host, port, timeout=30)
        except Exception as e:
            self._json(502, {"error": "upstream connect failed", "detail": str(e)})
            return
        try:
            conn.request(self.command, path, body=body, headers=out_headers)
            resp = conn.getresponse()
            data = resp.read()
            self.send_response(resp.status, resp.reason)
            for k, v in resp.getheaders():
                lk = k.lower()
                if lk in {
                    "transfer-encoding",
                    "connection",
                    "keep-alive",
                    "proxy-connection",
                    "content-length",
                }:
                    continue
                self.send_header(k, v)
            self.send_header("Content-Length", str(len(data)))
            self.send_header("X-Egress-Decision", "allowed")
            self.end_headers()
            self.wfile.write(data)
        except Exception as e:
            log(
                f"[{self.log_date_time_string()}] UPSTREAM-ERR {hostport} {e!r}"
            )
            try:
                self._json(502, {"error": "upstream error", "detail": str(e)})
            except Exception:
                pass
        finally:
            try:
                conn.close()
            except Exception:
                pass

    def _json(self, status: int, payload: dict) -> None:
        data = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        return self._handle()

    def do_POST(self):
        return self._handle()

    def do_PUT(self):
        return self._handle()

    def do_DELETE(self):
        return self._handle()

    def _handle(self):
        target = self._resolve_target()
        _, hostport, new_path = target
        if not self._is_allowed(hostport):
            log(
                f"[{self.log_date_time_string()}] DENY "
                f"{self.command} {self.path} -> {hostport} "
                f"reason=not-in-allowlist allowlist={sorted(ALLOWLIST)}"
            )
            self.send_response(403)
            self.send_header("Content-Type", "application/json")
            self.send_header("X-Egress-Decision", "denied")
            body = json.dumps(
                {
                    "error": "egress denied",
                    "target": hostport,
                    "allowlist": sorted(ALLOWLIST),
                }
            ).encode("utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        length = int(self.headers.get("Content-Length", "0") or "0")
        body = self.rfile.read(length) if length else b""
        self._forward(hostport, new_path, body)

    def log_message(self, fmt, *args):
        return


def main() -> int:
    global UPSTREAM_BASE, ALLOWLIST_RAW, ALLOWLIST, DENY_BY_DEFAULT
    p = argparse.ArgumentParser()
    p.add_argument("--bind", default=LISTEN_HOST)
    p.add_argument("--port", type=int, default=LISTEN_PORT)
    p.add_argument("--upstream", default=UPSTREAM_BASE)
    p.add_argument("--allowlist", default=ALLOWLIST_RAW)
    p.add_argument(
        "--allow-all",
        action="store_true",
        help="Disable deny-by-default (DANGEROUS; for debugging only).",
    )
    args = p.parse_args()

    UPSTREAM_BASE = args.upstream
    ALLOWLIST_RAW = args.allowlist
    ALLOWLIST = _allowlist_hosts()
    if args.allow_all:
        DENY_BY_DEFAULT = False

    if not REAL_KEY:
        log(
            "WARN: FAKE_PROVIDER_REAL_KEY is not set. "
            "The proxy will pass Authorization headers through unchanged."
        )

    srv = ThreadingHTTPServer((args.bind, args.port), Handler)
    log(
        f"egress-proxy listening on http://{args.bind}:{args.port} "
        f"upstream={UPSTREAM_BASE} "
        f"allowlist={sorted(ALLOWLIST)} "
        f"deny_by_default={DENY_BY_DEFAULT} "
        f"dummy_key_len={len(DUMMY_KEY)} real_key_len={len(REAL_KEY)} "
        f"log={LOG_PATH}"
    )
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        log("egress-proxy shutting down")
    finally:
        srv.server_close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
