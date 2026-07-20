# ADR Index

Architecture Decision Records for the workestrator tool+XDG migration.

| ID | Title | Status | Decision (one line) |
|---|---|---|---|
| 0001 | Data-driven workloads | Accepted | Migrate 5 workloads from Rust `workloads/*.rs` to `workestrate.toml`; delete Rust files after golden parity |
| 0002 | TOML config format | Accepted | TOML with `toml` crate (Nix `builtins.fromTOML` interop; `serde_yaml` archived) |
| 0003 | Config purity + closed recipe vocabulary | Accepted | Config = data+secrets+static files only; all logic = named recipes in core (Kustomize no-templating anchor) |
| 0004 | Security allowlist in policy.rs | Accepted | Rust consts (`ALLOWED_EGRESS_HOSTS`, `SECRET_HOST_BINDINGS`, `ALLOWED_PACKAGES`) + per-recipe scoping |
| 0005 | Security-aware merge | Accepted | Monotonic `default_deny` + additive deny/egress/secret_env; RFC 7396 for non-security fields only |
| 0006 | Hybrid CLI dispatch | Accepted | Typed clap subcommands for known names + catch-all for config-defined (not pure external_subcommand, not build.rs codegen) |
| 0007 | Tool+XDG+dotfiles organization model | Accepted | workestrate-as-tool + XDG paths + registry in `~/.config/workestrate/config.toml` (user's dotfiles) |
| 0008 | Config repos via uniform `workestrate config add` | Accepted | All config repos consumed via git clone into managed store (no parent-flake-input editing) |
| 0009 | Agent configs move to config repo | Accepted | `agents/*/config/` moves to config repo; mount resolution becomes config-relative |
| 0010 | Source override naming | Accepted | "source override" concept; `workestrate source` commands; `vendor` reserved for frozen deps |
| 0011 | Microsandbox vendor → git-fork dependency | Accepted | Migrate vendor symlink to git-fork dependency (fork-carries-compat) |
| 0012 | Odysseus/opencode nix derivations prerequisite | Accepted | Create `.#odysseus-built`/`.#opencode-built` as Phase 0a prerequisite (verified gap) |
| 0013 | Layering via ordered registry layers | Accepted | Ordered `layers = [...]` in registry; named contexts deferred until 3+ layers |
| 0014 | Trust-gated project config | Accepted | `[trusted_projects]` in registry; `workestrate config trust <dir>`; `--no-project-config` escape |
| 0015 | Per-workload repos rejected | Accepted | Workload defs are ~20-line data entries; recipe vocabulary is the distribution unit |
| 0016 | Additive migration / deferred repo strip-down | Accepted | Root `workestrate.toml` keeps working as project layer; migration is additive |
| 0017 | Synthetic reference config and final strip-down | Accepted | `config.reference/` becomes synthetic fixture; root user files move to personal config repo; flake outputs stay explicit |
| 0018 | Secrets layering + per-repo secrets config | Accepted | Per-key value merge across layers; per-repo secrets_file/age_key_file; process env lowest precedence |
| 0019 | Contexts + user-global overrides | Accepted | `[contexts.<name>] layers=[...]` in registry; one context per invocation (`--context`/env/settings/default); instance namespacing; user-global overrides + secrets |
| 0020 | Review adjudications (2026-07) | Accepted | env union-by-name; entitlement checked before monotonic-true; `run`/`WORKESTRATE_CONFIG_DIR` = document-not-harden (with `local.toml` exception); spec-code consistency CI guard |
| 0021 | Instance lifecycle + AI-native surfaces | Accepted | Refuse-on-occupied default with `--replace`/`--instance`/`--new`; `ps`/`down --all`/`--port-offset`/`--json`; `generate-schema` + committed schema + CI drift guard + taplo `#:schema` |
