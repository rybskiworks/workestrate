---
name: validation-rust-supply-chain
description: |
  Verifies no known supply chain vulnerabilities or policy violations. Load
  when dependencies change or as periodic CI gate. Does NOT cover dependency
  selection strategy (see rust-cargo-and-deps operational skill).
metadata:
  org.kind: validation
---

# Validation: Rust Supply Chain

This gate verifies that the dependency tree has no known advisories and no
license/ban/source policy violations. It runs when dependencies change or as a
periodic CI gate.

## Triggers

Load this skill when:

- `Cargo.toml` or `Cargo.lock` dependency entries change.
- A new dependency is added or an existing one is upgraded.
- As a periodic CI gate (e.g., nightly/weekly).
- After running `cargo update`.

## Command

```bash
cargo audit
cargo deny check
```

Run both commands. If `cargo deny` is not configured, run `cargo audit` alone
(see Notes).

## Pass criteria

- No advisories reported by `cargo audit`.
- No license, ban, or source violations reported by `cargo deny check`.

## Fail criteria

- Advisories found by `cargo audit` (CVEs / RUSTSEC IDs).
- `deny.toml` violations (banned crates, disallowed licenses, disallowed
  sources, duplicate versions beyond limits).

## Evidence to report

- Exit code for each command.
- Advisory count and specific CVEs / RUSTSEC IDs (package, version, severity,
  patched version).
- License/ban/source violations from `cargo deny` (package, rule, detail).

## Notes

- If `cargo deny` is not configured (no `deny.toml`), record that fact and run
  `cargo audit` alone.
- If `cargo audit` is not installed, record that fact and skip this gate; do
  not fabricate a pass. Install via `cargo install cargo-audit` /
  `cargo install cargo-deny` as appropriate.
- `cargo audit` checks the advisory database (RUSTSEC) against `Cargo.lock`.
- `cargo deny check` enforces policy from `deny.toml`: licenses, banned
  crates, allowed sources, duplicate-version limits. Run `cargo deny check
  advisories` separately to also check advisories via deny.
- This gate does not cover dependency selection strategy — for guidance on
  choosing crates, see the `rust-cargo-and-deps` operational skill.
- Run inside `nix develop` (see `nix-usage` skill); `cargo-audit` /
  `cargo-deny` may need to be installed in the dev shell.
