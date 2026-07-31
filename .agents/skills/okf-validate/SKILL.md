---
name: okf-validate
description: |
  Verifies an OKF (Open Knowledge Format) v0.1 knowledge bundle directory is
  compliant: frontmatter presence/shape, required keys, concept ID path segments,
  reserved filename structure (index.md / log.md), cross-link style, citation
  format, and frontmatter key order. Load after authoring or editing OKF concept
  documents, before committing an OKF bundle, or as a CI gate. Does NOT cover
  runtime proxy validation, Rust/Elixir/Gleam code, or LiteLLM config.
metadata:
  org.kind: validation
---

# Validation: OKF v0.1 Bundle Compliance

This gate verifies that an OKF (Open Knowledge Format) v0.1 knowledge bundle
directory tree is conformant: every concept document carries well-formed
frontmatter, concept IDs are valid path segments, reserved filenames
(`index.md` / `log.md`) follow their structural rules, and soft authoring
guidance (cross-link style, citations, key order) is respected.

## Triggers

Load this skill when:

- After authoring or editing any OKF concept `.md` document.
- Before committing or pushing an OKF knowledge bundle.
- As a CI gate on OKF bundle directories.
- When diagnosing why an OKF consumer/reference-agent rejected a document.

Do NOT load for: non-OKF markdown, LiteLLM config, code in Rust/Elixir/Gleam,
or runtime proxy checks.

## Command

```bash
python3 .agents/skills/okf-validate/scripts/validate_okf.py <bundle_dir>
```

`<bundle_dir>` is the root of the OKF bundle (the directory containing
`index.md`, concept `.md` files, and subdirectories). The script walks the
entire tree recursively, skipping `.crawl/` directories.

## Pass / Fail criteria

- **PASS** (exit 0): every checked `.md` file has only PASS status (no WARN,
  no FAIL).
- **WARN-only** (exit 2): at least one WARN, zero FAIL. The bundle is
  OKF-conformant per the normative spec (§9) but violates soft/recommended
  guidance.
- **FAIL** (exit 1): at least one FAIL. The bundle violates a normative
  OKF v0.1 conformance rule and is not conformant.

Normative (FAIL) vs soft (WARN) classification is listed per-check below.

## Evidence to report

- Overall verdict (PASS / WARN-only / FAIL) and exit code.
- Per-file status line: `<relative_path>: PASS|WARN|FAIL`.
- For each file with violations, list each violation with its severity tag
  `[FAIL]` or `[WARN]` and a one-line message.
- Summary counts: files checked, files PASS, files WARN, files FAIL, total
  violations.
- If run in a terminal, output uses ANSI color (green=PASS, yellow=WARN,
  red=FAIL); auto-disabled when not a TTY.

## Validation checks

All 10 checks below. FAIL = normative per SPEC §9 / reference `document.py`;
WARN = soft/recommended guidance.

1. **Frontmatter presence & delimiters** — [FAIL] Every non-reserved `.md`
   file must start with a `---` line and contain a closing `---` line.
   Unterminated or missing frontmatter block fails. (SPEC §9.1, §4.1;
   `document.py` parse raises `OKFDocumentError`.)

2. **Frontmatter is a YAML mapping** — [FAIL] The frontmatter block must
   parse to a YAML mapping (dict), not a scalar or list. (`document.py`:
   `isinstance(fm, dict)`.)

3. **Required keys non-empty** — [FAIL] Frontmatter must contain non-empty
   values for `type`, `title`, `description`, `timestamp`. (`document.py`
   `REQUIRED_FRONTMATTER_KEYS`; `validate()` checks
   `not self.frontmatter.get(k)`.) Note: the normative SPEC §9 only strictly
   requires non-empty `type`; `title`/`description`/`timestamp` are
   tooling-compatible required keys enforced by the reference agent — this
   validator enforces all four as FAIL to match the reference agent.

4. **Concept ID path segments** — [FAIL] Every path segment of a concept's
   location (directory names + filename stem, excluding the `.md` suffix and
   excluding reserved filenames `index`/`log`) must match
   `[A-Za-z0-9_][A-Za-z0-9_.\-]*` (fullmatch). (`paths.py` `_SEGMENT_RE`.)

5. **Reserved filenames** — [FAIL if structure wrong] `index.md` and
   `log.md` are reserved. `index.md` MUST NOT contain frontmatter EXCEPT a
   bundle-root `index.md` which MAY contain a frontmatter block with an
   `okf_version` key (SPEC §11). Non-root `index.md` with frontmatter = FAIL.
   Root `index.md` with frontmatter lacking `okf_version` = FAIL. `log.md`
   must not contain concept frontmatter (a `type` key in log.md frontmatter
   = FAIL).

6. **index.md structure** — [WARN] When `index.md` is present, its body
   SHOULD group concepts under `# <Type>` headings and list subdirectories.
   Warn if an `index.md` body has no `# ` headings, or if it contains
   concept-style frontmatter keys other than `okf_version`. (SPEC §6;
   `index.py` `_build_index_text`.)

7. **log.md structure** — [WARN] `log.md` SHOULD start with a
   `# Directory Update Log` heading and use `## YYYY-MM-DD` date headings
   (ISO 8601), newest first. Warn if the top heading is missing, if date
   headings are not ISO format, or if dates are not in newest-first order.
   (SPEC §7.)

8. **Cross-link style** — [WARN] Cross-links SHOULD be file-relative. Warn
   on links whose target path begins with `/` (bundle-absolute links). Only
   warn for markdown links `[text](target)` where `target` starts with `/`
   and is not a pure URL (i.e. not `http://`/`https://`/`#`/`mailto:`).
   (Note: SPEC §5.1 actually recommends bundle-relative `/`-absolute links;
   this validator follows the stricter file-relative guidance from the task
   analysis and emits only WARNs, never FAILs.)

9. **Citations format** — [WARN] If a `# Citations` heading is present,
   entries SHOULD follow `[n] [Title](url)` format. Warn per citation line
   under `# Citations` that does not match `^\[\d+\]\s+\[.+?\]\(.+?\)`.
   (SPEC §8.) Absence of a Citations section is not a violation.

10. **Frontmatter key order** — [WARN] The recommended key order is
    `type, resource, title, description, tags, timestamp`. Warn if the actual
    key order of the keys that ARE present differs from this recommended
    order (ignoring keys not in the recommended list, which may appear
    after). (SPEC §4.1 example ordering.)

## How to fix common violations

- Missing frontmatter: add `---\ntype: ...\n...\n---` at the very top of the
  file (first line must be `---`).
- Frontmatter not a mapping: ensure top-level YAML is `key: value` pairs,
  not a bare string or a `- item` list.
- Missing required key: add the missing key with a non-empty value
  (e.g. `timestamp: 2026-05-28T00:00:00Z`).
- Invalid concept ID segment: rename the file/directory so each segment
  starts with `[A-Za-z0-9_]` and contains only `[A-Za-z0-9_.\-]`.
- Non-root index.md with frontmatter: remove the frontmatter block; only the
  bundle-root index.md may carry `okf_version`.
- index.md missing type headings: regenerate via the reference agent
  `regenerate_indexes`, or add `# <Type>` sections listing
  `* [Title](file.md) - desc`.
- log.md bad dates: use `## YYYY-MM-DD` headings and order newest first.
- Absolute cross-links: change `(/path/to.md)` to a relative
  `(./path/to.md)` or `(path/to.md)` form (note: this is a soft warning).
- Bad citation format: rewrite as `[1] [Title](https://example.com)`.
- Key order: reorder frontmatter keys to
  `type, resource, title, description, tags, timestamp` (extra keys after).

## Notes

- The script uses only the Python 3 standard library (no pyyaml dependency);
  it ships a minimal YAML frontmatter parser sufficient for OKF's flat
  `key: value` / inline-list / block-list shapes.
- `.crawl/` directories are skipped entirely (source snapshots, not OKF
  concepts).
- Empty `.md` files are reported as FAIL (no frontmatter).
- The validator is read-only; it never modifies the bundle.
