#!/usr/bin/env python3
"""store-audit report — extracted from the justfile `store-audit` recipe.

Reads `nix path-info --all --json` output from stdin and prints the top-20
store paths by closure size.

With ``--warn-if-source-over <MB>`` the script additionally scans for
attributable LOCAL flake-input copies: paths whose basename is a 32-char
store hash plus one of the known local input names (workestrate, personal,
duelbits, nix-tooling), optionally suffixed ``-source`` — i.e. matching
``^[a-z0-9]{32}-(workestrate|personal|duelbits|nix-tooling)(-source)?$``.
Those names only appear when an input was declared ``path:``-style (nix
names such copies after the source basename), so a match IS attributable to
a local working copy. Any match whose closure size exceeds the threshold
(in MiB, 1 MB = 1_000_000 bytes) is printed to stderr as a WARN — the flag
is INFORMATIONAL and the script always exits 0.

Why the old blocking ``*ai-workbench*-source`` gate is retired: that naming
era is obsolete (the repo was renamed), and nix names git/tarball flake-input
copies ``<hash>-source`` regardless of provenance — a local
``git+file:///.../workestrate`` copy is byte-for-byte indistinguishable BY
NAME from a legitimate ``github:nixpkgs`` copy, so no static name pattern can
gate local ``-source`` copies without also hitting nixpkgs (~GB, legit). The
real defense against local source accumulation is the github-input swap
(host-pending) plus the active ``lint-nix`` gate (scripts/check-nix-paths.sh);
the top-20 report still surfaces oversized ``-source`` paths for human
triage.

NON-BLOCKING on input problems: on any read/parse failure it prints an
informational note and exits 0. Stdlib only (argparse, json, re, sys) — no
third-party deps.
"""

import argparse
import json
import re
import sys

# Basenames attributable to local `path:`-style flake-input copies
# (see module docstring for why generic *-source paths cannot be gated).
LOCAL_COPY_RE = re.compile(
    r"^[a-z0-9]{32}-(workestrate|personal|duelbits|nix-tooling)(-source)?$"
)


def path_size(info: object) -> int:
    """Return the closure size (or size) of a path-info entry, 0 on error."""
    try:
        return int(info.get("closureSize", info.get("size", 0)) or 0)  # type: ignore[union-attr]
    except Exception:
        return 0


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Report top-20 nix store paths by closure size; optionally "
        "warn (informationally) when local path-style input copies exceed a "
        "size threshold."
    )
    parser.add_argument(
        "--warn-if-source-over",
        type=int,
        metavar="MB",
        default=None,
        help="Print a WARN to stderr for every local path-style input copy "
        "(<hash>-{workestrate,personal,duelbits,nix-tooling}[-source]) whose "
        "closure size exceeds MB megabytes (1 MB = 1_000_000 bytes). "
        "Informational: always exits 0. Omit for the report-only mode.",
    )
    args = parser.parse_args()

    try:
        data = json.load(sys.stdin)
    except Exception as e:
        print(f"(could not read nix path-info: {e})")
        return

    paths = []
    for p, info in data.items():
        paths.append((path_size(info), p))

    paths.sort(reverse=True)
    for size, p in paths[:20]:
        print(f"{size:>14,d}  {p}")

    if args.warn_if_source_over is None:
        return

    threshold_bytes = args.warn_if_source_over * 1_000_000
    oversized = [
        (size, p)
        for size, p in paths
        if LOCAL_COPY_RE.match(p.rsplit("/", 1)[-1]) and size > threshold_bytes
    ]
    if oversized:
        print(
            f"WARN: {len(oversized)} local path-style input copy(ies) exceed "
            f"{args.warn_if_source_over} MB (informational, not a failure — "
            "see scripts/store-audit.py docstring; real defense: github "
            "input swap (host-pending) + lint-nix):",
            file=sys.stderr,
        )
        for size, p in sorted(oversized, reverse=True):
            print(f"  WARN {size:>14,d}  {p}", file=sys.stderr)
    else:
        print(
            f"OK: no local path-style input copy exceeds "
            f"{args.warn_if_source_over} MB."
        )


if __name__ == "__main__":
    main()
