#!/usr/bin/env python3
"""store-audit report — extracted from the justfile `store-audit` recipe.

Reads `nix path-info --all --json` output from stdin and prints the top-20
store paths by closure size.

With ``--fail-if-source-over <MB>`` the script additionally scans every path
whose name contains ``ai-workbench`` AND ends with ``-source`` (the impure
path-style copy probe). If any such path's closure size exceeds the given
threshold (in MiB, 1 MB = 1_000_000 bytes), the oversized paths are printed
to stderr and the script exits 1 — turning the previously passive probe into
a blocking gate. Without the flag the behavior is unchanged (top-20 report,
exit 0).

NON-BLOCKING on input problems: on any read/parse failure it prints an
informational note and exits 0 — the gate fails only on actual oversized
source paths, never on missing/malformed input. Stdlib only (argparse,
json, sys) — no third-party deps.
"""

import argparse
import json
import sys


def path_size(info: object) -> int:
    """Return the closure size (or size) of a path-info entry, 0 on error."""
    try:
        return int(info.get("closureSize", info.get("size", 0)) or 0)  # type: ignore[union-attr]
    except Exception:
        return 0


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Report top-20 nix store paths by closure size; optionally "
        "fail when ai-workbench *-source paths exceed a size threshold."
    )
    parser.add_argument(
        "--fail-if-source-over",
        type=int,
        metavar="MB",
        default=None,
        help="Exit 1 if any *ai-workbench*-source store path exceeds MB "
        "megabytes (1 MB = 1_000_000 bytes). Omit for the non-blocking "
        "report-only mode.",
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

    if args.fail_if_source_over is None:
        return

    threshold_bytes = args.fail_if_source_over * 1_000_000
    oversized = [
        (size, p)
        for size, p in paths
        if "ai-workbench" in p and p.endswith("-source") and size > threshold_bytes
    ]
    if oversized:
        print(
            f"FAIL: {len(oversized)} ai-workbench *-source path(s) exceed "
            f"{args.fail_if_source_over} MB (unbounded source copy into the store):",
            file=sys.stderr,
        )
        for size, p in sorted(oversized, reverse=True):
            print(f"  {size:>14,d}  {p}", file=sys.stderr)
        print(
            "Investigate the cleanSourceWith filter — an impure path-style "
            "copy of the repo working tree entered the store.",
            file=sys.stderr,
        )
        sys.exit(1)

    print(
        f"OK: no ai-workbench *-source path exceeds {args.fail_if_source_over} MB."
    )


if __name__ == "__main__":
    main()
