#!/usr/bin/env python3
"""OKF v0.1 compliance validator.

Walks an OKF (Open Knowledge Format) knowledge bundle directory tree and
checks every ``.md`` file for conformance: frontmatter presence/shape,
required keys, concept ID path segments, reserved filename structure
(``index.md`` / ``log.md``), cross-link style, citation format, and
frontmatter key order.

Exit codes:
    0 - all files PASS
    1 - at least one FAIL violation
    2 - at least one WARN (and no FAIL), or usage/setup error

Standard library only; no pyyaml dependency.
"""

import argparse
import os
import re
import sys
from pathlib import Path

# --------------------------------------------------------------------------
# Constants
# --------------------------------------------------------------------------

RESERVED_FILENAMES = {"index.md", "log.md"}
RESERVED_STEMS = {"index", "log"}

REQUIRED_FRONTMATTER_KEYS = ("type", "title", "description", "timestamp")

RECOMMENDED_KEY_ORDER = (
    "type",
    "resource",
    "title",
    "description",
    "tags",
    "timestamp",
)

# Concept ID path-segment pattern (paths.py _SEGMENT_RE).
SEGMENT_RE = re.compile(r"[A-Za-z0-9_][A-Za-z0-9_.\-]*")

# Markdown link pattern: [text](target)
LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+)\)")

# URL scheme pattern, e.g. "http:", "https:", "mailto:".
SCHEME_RE = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")

# ISO date heading in log.md: "## YYYY-MM-DD".
ISO_DATE_HEADING_RE = re.compile(r"^## (\d{4})-(\d{2})-(\d{2})\s*$")

# Citation entry format: "[n] [Title](url)".
CITATION_RE = re.compile(r"^\[\d+\]\s+\[.+?\]\(.+?\)")

# ANSI colors (enabled only when stdout is a TTY).
COLORS = {
    "PASS": "\033[32m",  # green
    "WARN": "\033[33m",  # yellow
    "FAIL": "\033[31m",  # red
}
RESET = "\033[0m"

USE_COLOR = sys.stdout.isatty()


def colorize(status: str, text: str) -> str:
    """Wrap text in the ANSI color for the given status when on a TTY."""
    if not USE_COLOR:
        return text
    return f"{COLORS.get(status, '')}{text}{RESET}"


# --------------------------------------------------------------------------
# Minimal YAML frontmatter parser
# --------------------------------------------------------------------------


def _parse_scalar(value: str):
    """Parse a scalar value: strip quotes, keep everything else as string.

    OKF only needs string-ish non-empty checks, so booleans/nulls/numbers
    are intentionally kept as their raw string forms (non-empty strings
    are truthy, which is what the required-key check relies on).
    """
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ("'", '"'):
        return value[1:-1]
    return value


def _parse_inline_list(value: str) -> list:
    """Parse an inline list like ``[a, b, 'c']`` into a list of scalars."""
    inner = value.strip()[1:-1].strip()
    if not inner:
        return []
    return [_parse_scalar(part) for part in inner.split(",")]


def _parse_frontmatter_block(fm_text: str):
    """Parse the YAML-ish frontmatter block into a dict.

    Supports OKF-style flat frontmatter:
      - ``key: value`` pairs
      - inline lists: ``key: [a, b, c]``
      - block lists: ``key:`` followed by indented ``- item`` lines
      - single/double-quoted values
      - blank lines and full-line ``#`` comments

    Returns a plain dict (insertion order preserved).

    Raises ``ValueError`` on any line that cannot be interpreted.
    """
    result = {}
    lines = fm_text.splitlines()
    i = 0
    while i < len(lines):
        raw = lines[i]
        stripped = raw.strip()

        # Skip blank lines and full-line comments.
        if not stripped or stripped.startswith("#"):
            i += 1
            continue

        # A top-level "- item" with no preceding key => the block is a
        # list, not a mapping.
        if stripped.startswith("- "):
            raise ValueError(
                f"Frontmatter must be a YAML mapping, not a list: {raw!r}"
            )

        # Must be a "key: ..." line.
        if ":" not in raw:
            raise ValueError(f"Invalid YAML line: {raw!r}")
        key, _, value = raw.partition(":")
        key = key.strip()
        if not key:
            raise ValueError(f"Invalid YAML line (empty key): {raw!r}")
        value = value.strip()

        if value.startswith("[") and value.endswith("]"):
            # Inline list.
            result[key] = _parse_inline_list(value)
            i += 1
        elif value:
            # Plain scalar; absorb YAML folded continuation lines (more-
            # indented lines that continue the value, e.g. a wrapped
            # description). Joined with a single space per YAML folding.
            parts = [value]
            j = i + 1
            while j < len(lines):
                cont_raw = lines[j]
                cont_stripped = cont_raw.strip()
                if not cont_stripped or cont_stripped.startswith("#"):
                    break
                indent = len(cont_raw) - len(cont_raw.lstrip())
                if indent > 0 and ":" not in cont_stripped:
                    parts.append(cont_stripped)
                    j += 1
                else:
                    break
            result[key] = _parse_scalar(" ".join(parts))
            i = j
        else:
            # Empty value: either null or the start of a block list.
            items = []
            j = i + 1
            while j < len(lines):
                item_raw = lines[j]
                item_stripped = item_raw.strip()
                if item_stripped.startswith("- "):
                    items.append(_parse_scalar(item_stripped[2:]))
                    j += 1
                elif not item_stripped or item_stripped.startswith("#"):
                    # Blank/comment line inside a block list region: only
                    # continue consuming if we have already seen items and
                    # the next meaningful line is still a list item. To
                    # keep the parser simple, stop the block list here.
                    break
                else:
                    break
            if items:
                result[key] = items
                i = j
            else:
                # Empty value, no block list: treat as null-ish empty.
                result[key] = None
                i += 1

    return result


def parse_frontmatter(text: str):
    """Split text into (frontmatter_dict_or_None, error_or_None, body).

    - No leading ``---`` delimiter: (None, None, text) — caller decides
      whether missing frontmatter is an error.
    - Unterminated block: (None, "Unterminated ...", "").
    - Parse failure / non-mapping: (None, "<error message>", body).
    - Success: (dict, None, body) where body is the text after the closing
      ``---`` with one leading blank line stripped (matching document.py).
    """
    if not text:
        return None, None, text

    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return None, None, text

    # Find the closing delimiter (a line whose stripped content is "---").
    close_idx = None
    for idx in range(1, len(lines)):
        if lines[idx].strip() == "---":
            close_idx = idx
            break

    if close_idx is None:
        return None, "Unterminated YAML frontmatter block", ""

    fm_text = "\n".join(lines[1:close_idx])
    body_lines = lines[close_idx + 1:]
    # Strip one leading blank line, matching document.py behavior.
    if body_lines and not body_lines[0].strip():
        body_lines = body_lines[1:]
    body = "\n".join(body_lines)

    try:
        parsed = _parse_frontmatter_block(fm_text)
    except ValueError as exc:
        return None, str(exc), body
    except Exception as exc:  # defensive: unexpected parser failure
        return None, f"Invalid YAML in frontmatter: {exc}", body

    if not isinstance(parsed, dict):
        return None, "Frontmatter must be a YAML mapping", body

    return parsed, None, body


# --------------------------------------------------------------------------
# Individual checks
# --------------------------------------------------------------------------


def check_required_keys(fm: dict) -> list:
    """[FAIL] type/title/description/timestamp must be present + non-empty."""
    violations = []
    for key in REQUIRED_FRONTMATTER_KEYS:
        if not fm.get(key):
            violations.append(
                ("FAIL", f"Missing or empty required frontmatter key: {key}")
            )
    return violations


def check_concept_id(rel_path: Path, filename: str) -> list:
    """[FAIL] Every path segment (dirs + stem) must match _SEGMENT_RE."""
    violations = []
    stem = filename[:-3] if filename.endswith(".md") else filename
    if stem in RESERVED_STEMS:
        return violations
    segments = list(rel_path.parent.parts) + [stem]
    for seg in segments:
        if not seg or seg in (".", os.sep):
            continue
        if not SEGMENT_RE.fullmatch(seg):
            violations.append(("FAIL", f"Invalid concept ID segment: {seg!r}"))
    return violations


def check_reserved_files(filename: str, is_root: bool, fm) -> list:
    """[FAIL] Structural rules for index.md / log.md frontmatter."""
    violations = []
    if filename == "index.md":
        if fm is not None:
            if is_root:
                if "okf_version" not in fm:
                    violations.append(
                        (
                            "FAIL",
                            "Root index.md frontmatter must contain "
                            "okf_version",
                        )
                    )
            else:
                violations.append(
                    ("FAIL", "Non-root index.md must not contain frontmatter")
                )
    elif filename == "log.md":
        if fm is not None and "type" in fm:
            violations.append(
                (
                    "FAIL",
                    "log.md must not contain concept frontmatter "
                    "(type key present)",
                )
            )
    return violations


def check_index_structure(body: str) -> list:
    """[WARN] index.md body should group concepts under '# <Type>' headings."""
    violations = []
    has_heading = any(
        line.startswith("# ") for line in body.splitlines() if line.strip()
    )
    if not has_heading:
        violations.append(
            ("WARN", "index.md body has no '# <Type>' section headings")
        )
    return violations


def check_log_structure(body: str) -> list:
    """[WARN] log.md heading shape, ISO date headings, newest-first order."""
    violations = []
    lines = body.splitlines()

    # Top heading: first non-empty line should be a '# ... Update Log'.
    first = next((line for line in lines if line.strip()), "")
    if not (first.startswith("# ") and "Update Log" in first):
        violations.append(
            (
                "WARN",
                "log.md should start with a '# Directory Update Log' heading",
            )
        )

    # Collect '## ' headings; validate ISO format and newest-first order.
    dates = []
    for line in lines:
        if not line.startswith("## "):
            continue
        match = ISO_DATE_HEADING_RE.match(line.rstrip())
        if match:
            dates.append(
                (int(match.group(1)), int(match.group(2)), int(match.group(3)))
            )
        else:
            violations.append(
                ("WARN", f"log.md date heading not ISO format: {line!r}")
            )

    for earlier, later in zip(dates, dates[1:]):
        if earlier < later:
            violations.append(
                ("WARN", "log.md dates not in newest-first order")
            )
            break

    return violations


def check_cross_links(body: str) -> list:
    """[WARN] Bundle-absolute ('/'-prefixed) cross-links; prefer relative."""
    violations = []
    seen = set()
    for match in LINK_RE.finditer(body):
        target = match.group(1).strip()
        if not target.startswith("/") or target.startswith("//"):
            continue
        if SCHEME_RE.match(target):
            continue  # a URL, not a bundle path
        if target in seen:
            continue
        seen.add(target)
        violations.append(
            (
                "WARN",
                f"Bundle-absolute cross-link (prefer file-relative): {target}",
            )
        )
    return violations


def check_citations(body: str) -> list:
    """[WARN] Entries under '# Citations' should match '[n] [Title](url)'."""
    violations = []
    lines = body.splitlines()
    in_citations = False
    for line in lines:
        stripped = line.strip()
        if stripped == "# Citations":
            in_citations = True
            continue
        if in_citations and line.startswith("# "):
            break  # next top-level section ends the citations block
        if not in_citations or not stripped:
            continue
        if not CITATION_RE.match(stripped):
            violations.append(
                (
                    "WARN",
                    "Citation line does not follow [n] [Title](url) format: "
                    f"{stripped}",
                )
            )
    return violations


def check_key_order(fm: dict) -> list:
    """[WARN] Present keys should follow the recommended relative order."""
    violations = []
    recommended_positions = {
        key: idx for idx, key in enumerate(RECOMMENDED_KEY_ORDER)
    }
    present_in_order = [
        key for key in fm.keys() if key in recommended_positions
    ]
    positions = [recommended_positions[key] for key in present_in_order]
    if positions != sorted(positions):
        violations.append(
            (
                "WARN",
                "Frontmatter key order differs from recommended (type, "
                "resource, title, description, tags, timestamp); actual: "
                + ", ".join(fm.keys()),
            )
        )
    return violations


# --------------------------------------------------------------------------
# Per-file validation
# --------------------------------------------------------------------------


def validate_file(path: Path, bundle_root: Path) -> list:
    """Run all applicable checks on one .md file; return violations list."""
    violations = []

    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        return [("FAIL", f"Cannot read file: {exc}")]

    rel_path = path.relative_to(bundle_root)
    filename = path.name
    is_root = path.parent == bundle_root
    is_reserved = filename in RESERVED_FILENAMES

    fm, error, body = parse_frontmatter(text)

    # Checks 1 & 2: frontmatter presence / parse shape.
    if error is not None:
        violations.append(("FAIL", error))
    elif fm is None and not is_reserved:
        violations.append(
            ("FAIL", "Missing YAML frontmatter block (must start with '---')")
        )

    if fm is not None:
        # Check 3 (required keys) and 10 (key order): concept files only.
        if not is_reserved:
            violations.extend(check_required_keys(fm))
            violations.extend(check_key_order(fm))
        # Check 5: reserved filename frontmatter rules.
        violations.extend(check_reserved_files(filename, is_root, fm))

    # Check 4: concept ID path segments (path-based, frontmatter-independent).
    if not is_reserved:
        violations.extend(check_concept_id(rel_path, filename))

    # Checks 6 & 7: reserved-file structure (body-based).
    if filename == "index.md":
        violations.extend(check_index_structure(body))
    elif filename == "log.md":
        violations.extend(check_log_structure(body))

    # Checks 8 & 9: body checks applied to all files.
    violations.extend(check_cross_links(body))
    violations.extend(check_citations(body))

    return violations


# --------------------------------------------------------------------------
# Driver
# --------------------------------------------------------------------------


def find_markdown_files(bundle_root: Path) -> list:
    """Recursively find .md files, pruning any .crawl directories."""
    found = []
    for dirpath, dirnames, filenames in os.walk(bundle_root):
        dirnames[:] = [d for d in dirnames if d != ".crawl"]
        for name in filenames:
            if name.endswith(".md"):
                found.append(Path(dirpath) / name)
    found.sort(key=lambda p: str(p.relative_to(bundle_root)))
    return found


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate an OKF v0.1 knowledge bundle directory for "
        "compliance (frontmatter, concept IDs, reserved files, link and "
        "citation style)."
    )
    parser.add_argument(
        "bundle_dir",
        help="Path to the OKF bundle root directory (containing index.md "
        "and concept .md files).",
    )
    args = parser.parse_args()

    bundle_root = Path(args.bundle_dir)
    if not bundle_root.is_dir():
        print(
            f"error: not a directory: {args.bundle_dir}", file=sys.stderr
        )
        return 2
    bundle_root = bundle_root.resolve()

    files = find_markdown_files(bundle_root)

    n_pass = n_warn = n_fail = 0
    total_fails = total_warns = 0

    for path in files:
        rel = path.relative_to(bundle_root)
        violations = validate_file(path, bundle_root)
        fails = sum(1 for sev, _ in violations if sev == "FAIL")
        warns = sum(1 for sev, _ in violations if sev == "WARN")
        total_fails += fails
        total_warns += warns

        if fails:
            status = "FAIL"
            n_fail += 1
        elif warns:
            status = "WARN"
            n_warn += 1
        else:
            status = "PASS"
            n_pass += 1

        print(colorize(status, f"{rel}: {status}"))
        for severity, message in violations:
            print(colorize(status, f"    [{severity}] {message}"))

    print(
        f"Checked: {len(files)} files | PASS: {n_pass} | WARN: {n_warn} | "
        f"FAIL: {n_fail} | Violations: {total_fails} fails, "
        f"{total_warns} warns"
    )

    if n_fail:
        return 1
    if n_warn:
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
