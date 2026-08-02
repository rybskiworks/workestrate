---
name: validation-rust-docs
description: |
  Verifies Rust documentation builds without warnings. Load when pub items or
  doc comments change. Does NOT cover API doc content rules (see
  constraint-rust-api-docs).
metadata:
  org.kind: validation
---

# Validation: Rust Docs

This gate verifies that `cargo doc` builds the documentation without warnings,
including broken intra-doc links. It does not enforce doc content rules — those
live in `constraint-rust-api-docs`.

## Triggers

Load this skill when:

- `pub` items or doc comments are added or modified.
- As part of the standard validation suite.
- After changing intra-doc links or re-export structure.

## Command

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
```

## Pass criteria

- Exit code 0.
- No rustdoc warnings (broken links, missing docs where required, etc.).

## Fail criteria

- Warnings or errors in the doc build.
- Broken intra-doc links.

## Evidence to report

- Exit code.
- Warning count.
- Specific broken links or missing-doc warnings (file, item, message).

## Notes

- `RUSTDOCFLAGS="-D warnings"` denies warnings so they fail the gate. Without
  it, rustdoc warnings do not fail the build.
- `--no-deps` skips building docs for dependencies (faster, and avoids
  third-party warnings).
- `--document-private-items` ensures internal docs build too, surfacing
  broken links in private items.
- This gate checks doc build integrity, not doc content. For content rules
  (every `pub` item documented, `# Errors`/`# Panics`/`# Safety` sections,
  intra-doc links), see `constraint-rust-api-docs`.
- Run inside `nix develop` (see `nix-usage` skill).
