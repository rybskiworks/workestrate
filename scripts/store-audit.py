#!/usr/bin/env python3
"""store-audit report — extracted from the justfile `store-audit` recipe.

Reads `nix path-info --all --json` output from stdin and prints the top-20
store paths by closure size. NON-BLOCKING: on any read/parse failure it
prints an informational note and exits 0 so the `verify` gate cannot fail
on this step. Stdlib only (json, sys) — no third-party deps.
"""

import json
import sys


def main() -> None:
    try:
        data = json.load(sys.stdin)
    except Exception as e:
        print(f"(could not read nix path-info: {e})")
        return

    paths = []
    for p, info in data.items():
        try:
            size = int(info.get("closureSize", info.get("size", 0)) or 0)
        except Exception:
            size = 0
        paths.append((size, p))

    paths.sort(reverse=True)
    for size, p in paths[:20]:
        print(f"{size:>14,d}  {p}")


if __name__ == "__main__":
    main()
