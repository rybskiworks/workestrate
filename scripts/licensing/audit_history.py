#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Georg Rybski
# SPDX-License-Identifier: Apache-2.0
"""Inventory potentially relevant reachable Git objects, without changing Git.

Names are hints reported by rev-list, not every historical pathname. This is a
triage inventory, not license detection, a secrets scanner, or a violation claim.
It does not contact remotes, inspect binary caches, or rewrite history.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import subprocess


def git(repo: Path, *args: str, input_text: str | None = None) -> str:
    return subprocess.run(["git", "-C", str(repo), *args], input=input_text, text=True,
                          check=True, stdout=subprocess.PIPE).stdout


def inventory(repo: Path) -> dict:
    head = git(repo, "rev-parse", "HEAD").strip()
    shallow = git(repo, "rev-parse", "--is-shallow-repository").strip() == "true"
    objects = {}
    for line in git(repo, "rev-list", "--objects", "--all").splitlines():
        identity, _, hint = line.partition(" ")
        objects[identity] = hint
    records = []
    if objects:
        rows = git(repo, "cat-file", "--batch-check=%(objectname) %(objecttype) %(objectsize)",
                   input_text="\n".join(objects) + "\n")
        for row in rows.splitlines():
            identity, kind, size = row.split()
            hint = objects[identity]
            if kind != "blob":
                continue
            reason = None
            if "/.crawl/" in hint or hint.startswith(".agents/skills/"):
                reason = "reference-or-skill-provenance"
            elif hint.lower().endswith((".gz", ".xz", ".zip", ".bin", ".dll", ".so", ".dylib", ".wasm", ".exe")) or ".so." in hint:
                reason = "possible-bundled-binary-or-archive"
            elif "license" in hint.lower() or "notice" in hint.lower():
                reason = "historical-legal-material"
            if reason:
                records.append({"object": identity, "path_hint": hint, "size": int(size), "review": reason})
    return {"schema": 1, "head": head, "shallow": shallow,
            "scope": "objects reachable from refs currently present in this local clone",
            "limitations": ["path hints are not exhaustive", "no remote/cache/release audit", "no licensing conclusions"],
            "objects": sorted(records, key=lambda item: (item["path_hint"], item["object"]))}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    args = parser.parse_args()
    try:
        print(json.dumps(inventory(args.repo), indent=2, sort_keys=True))
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"history inventory: {error}\n")


if __name__ == "__main__":
    main()
