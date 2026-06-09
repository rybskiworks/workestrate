#!/usr/bin/env python3
"""Find the Authorization header line for a request whose body contains the marker.

Walks the fake-provider log file: lines starting with '[' are request
headers, lines starting with whitespace are body dumps. Pairs them.
Prints the header line for the first request whose body contains
'secret-injection-marker'. Exits 0 on success, 2 if not found.
"""
import sys

if len(sys.argv) < 2:
    print("usage: _find_evidence.py <log>", file=sys.stderr)
    sys.exit(3)

log_path = sys.argv[1]
header = None
matched = False
try:
    with open(log_path, "r", encoding="utf-8") as f:
        for line in f:
            if line.startswith("["):
                header = line.rstrip("\n")
            elif "secret-injection-marker" in line and header:
                print(header)
                matched = True
                break
except FileNotFoundError:
    print(f"log not found: {log_path}", file=sys.stderr)
    sys.exit(3)

if not matched:
    sys.exit(2)
