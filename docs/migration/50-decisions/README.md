# ADR Index

Architecture Decision Records for the workestrator tool+XDG migration.

| ID | Title | Status | Decision (one line) |
|---|---|---|---|
| 0001 | Data-driven workloads | Accepted | Migrate 5 workloads from Rust `workloads/*.rs` to `workestrate.toml`; delete Rust files after golden parity |
| 0002 | TOML config format | Accepted | TOML with `toml` crate (Nix `builtins.fromTOML` interop; `serde_yaml` archived) |
| 0003 | Config purity + closed recipe vocabulary | Accepted | Config = data+secrets+static files only; all logic = named recipes in core (Kustomize no-templating anchor) |
| 0004 | Security allowlist in policy.rs | Accepted | Rust consts (`ALLOWED_EGRESS_HOSTS`, `SECRET_HOST_BINDINGS`, `ALLOWED_PACKAGES`) + per-recipe scoping |
| 0005 | Security-aware merge | Accepted | Monotonic `default_deny` + additive deny/egress/secret_env; RFC 7396 for non-security fields only |
| 0006 | Hybrid CLI dispatch | Superseded by 0027 | Typed clap subcommands for known names + catch-all for config-defined (not pure external_subcommand, not build.rs codegen) |
| 0007 | Tool+XDG+dotfiles organization model | Accepted | workestrate-as-tool + XDG paths + registry in `~/.config/workestrate/config.toml` (user's dotfiles) |
| 0008 | Config repos via uniform `workestrate config add` | Accepted | All config repos consumed via git clone into managed store (no parent-flake-input editing) |
| 0009 | Agent configs move to config repo | Accepted | `agents/*/config/` moves to config repo; mount resolution becomes config-relative |
| 0010 | Source override naming | Accepted | "source override" concept; `workestrate source` commands; `vendor` reserved for frozen deps |
| 0011 | Microsandbox vendor → git-fork dependency | Accepted | Migrate vendor symlink to git-fork dependency (fork-carries-compat); addendum 2026-07-30: carrier REVERSED — fork never consumed as dependency; interim nix-side patch retained; upstream PR pending user push |
| 0012 | Odysseus/opencode nix derivations prerequisite | Accepted | Create `.#odysseus-built`/`.#opencode-built` as Phase 0a prerequisite (verified gap) |
| 0013 | Layering via ordered registry layers | Accepted | Ordered `layers = [...]` in registry; named contexts deferred until 3+ layers; partially superseded by ADR 0019 (the context-deferral is superseded by 0019's contexts; layer ordering stands) |
| 0014 | Trust-gated project config | Accepted | `[trusted_projects]` in registry; `workestrate config trust <dir>`; `--no-project-config` escape |
| 0015 | Per-workload repos rejected | Accepted | Workload defs are ~20-line data entries; recipe vocabulary is the distribution unit |
| 0016 | Additive migration / deferred repo strip-down | Accepted | Root `workestrate.toml` keeps working as project layer; migration is additive |
| 0017 | Synthetic reference config and final strip-down | Accepted | `config.reference/` becomes synthetic fixture; root user files move to personal config repo; flake outputs stay explicit |
| 0018 | Secrets layering + per-repo secrets config | Accepted | Per-key value merge across layers; per-repo secrets_file/age_key_file; process env lowest precedence; 2026-08-01 addenda: unified secret model, then final per-binding bound model (delivery-on-def removed; hosts→allowed_hosts; schema_version stays 1) |
| 0019 | Contexts + user-global overrides | Accepted | `[contexts.<name>] layers=[...]` in registry; one context per invocation (`--context`/env/settings/default); instance namespacing; user-global overrides + secrets |
| 0020 | Review adjudications (2026-07) | Accepted | env union-by-name; entitlement checked before monotonic-true; `run`/`WORKESTRATE_CONFIG_DIR` = document-not-harden (with `local.toml` exception); spec-code consistency CI guard |
| 0021 | Instance lifecycle + AI-native surfaces | Accepted | Refuse-on-occupied default with `--replace`/`--instance`/`--new`; `ps`/`down --all`/`--json`; `generate-schema` + committed schema + CI drift guard + taplo `#:schema`; addenda: 2026-07-30 `--port-offset` removed pre-release (superseded by 0026's 127.0.0.N slots + `--port-auto`); 2026-08-01 bare `workload up` starts all service-kind workloads |
| 0022 | Config-repo scaffolding: native `config new` + copier interop | Accepted | Rust-canonical embedded skeleton (`str::replace`, no engine, no conditionals by design) + copier retained-and-narrowed to advanced/team path; `.copier-answers.yml` interop; 4 CI guards; `register_config()` shared helper; age-key-path drift fix; addenda 2026-08-01: tombi TOML toolchain template additions (spec 15), then `config new <name> [dest]` positional dest (`--path` removed; registration only in-store; `<store>/repos/` → `config-repos/`) |
| 0023 | Single tool home (WORKESTRATE_HOME) | Accepted | Collapse XDG three-home (config/data/state) into single tool home `$WORKESTRATE_HOME` with flat layout; `home_version` + `migrate-home`; legacy XDG read-only compat; addendum 2026-07-30: trusted-ancestor discovery tier removed (split-brain risk) + `repos/` → `config-repos/` rename |
| 0024 | Dotfiles-style home repo + working-copy config repos | Accepted | Home config-repo clones are working copies (remote is canonical, dirty-safe `config update`); home itself is an explicit-init git repo (never auto-init); `repos/` → `config-repos/` |
| 0025 | Home provisioning (`home clone`) + `workestrate.lock` | Accepted | `home init` (empty scaffold, zero positionals) + `home clone <src> [<dest>]` (provisioning; verb split per the 2026-07-31 addendum, supersedes `home init --from`); selective copy (never `state/`); generated `workestrate.lock` pins url/ref/rev — the single pin mechanism for `home clone`/`up --pin`/spawn provenance |
| 0026 | Per-instance addressing + discovery-lite | Accepted | Slot-based binding: singleton on shared 127.0.0.1, parallel slots on 127.0.0.N (locked allocator); collisions keyed (bind_ip, port); --port-auto; depends_on discovery-lite; --port-offset removed pre-release; addendum 2026-08-01: dependency lifecycle default-on (compose-mirrored topo start; `--no-deps` opt-out; mandatory cycle detection) |
| 0027 | Verb-first workload dispatch | Accepted | `workestrate workload {up,exec,plan,down,logs} <name>` + `workestrate workloads` discovery; names are args not subcommands; supersedes 0006 |
| 0028 | Location-independent execution (CWD-independent root resolution) | Accepted | Flake/image-build roots resolve from the declaring config repo (registry-known); CWD is never the origin unless the CWD IS the declaring repo; `AGENTCTL_ROOT` = explicit override, not a requirement |

| 0029 | Policy scopes: collect-and-compile (never merge) | Accepted | `[policy.mounts]` fragments collected per layer in stack order; a compiler owns precedence/freeze/trust; merge.rs untouched (0020 Ruling 1 unamended); spec 22 primary, spec 01 fallback |

| 0030 | Instance lifecycle + conflict management + namespacing (DRAFT) | Draft | Per-workload `instance` policy (strategy singleton/parallel/replace/reuse + on_conflict default + port fixed/dynamic); one authoritative registry↔msb↔liveness reconcile step used by up/exec/down/ps; `instances` verb + 5-state status; generalizes d452575 dep on_conflict to the named path |
