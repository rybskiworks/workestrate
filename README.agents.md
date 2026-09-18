# Workestrate: architecture and operating reference

This is the text-first project README for agents and technical readers. It is
**optional context, not an instruction manifest or a prerequisite for every
subtask**. Read the sections relevant to the task. Repository-wide working
instructions live in [AGENTS.md](AGENTS.md); the human introduction lives in
[README.md](README.md).

This guide describes `main`, the integration branch. The former migration line
was integrated in [PR #32](https://github.com/rybskiworks/workestrate/pull/32).
[SPEC.md](SPEC.md) maps the maintained system contracts. Focused guides and ADRs
own detailed semantics; this document maps them rather than replacing them.
When implementation, specification, and an older decision disagree, identify
that discrepancy explicitly instead of treating historical intent as capability.

## Navigation

[System model](#system-model) / [Ownership](#ownership-and-repository-map) /
[Configuration](#configuration-model) / [Lifecycle](#lifecycle-and-runtime-state) /
[CLI](#cli-reference) / [Setup](#setup) / [Secrets](#secrets-workflow) /
[Development](#development-workflow) / [Task map](#task-to-source-map) /
[Status](#runtime-status)

## System model

Workestrate is generic, configuration-driven workload orchestration, built
agent-first. The Rust CLI resolves declarations and drives Microsandbox; the Nix
flake builds the tool, pins its dependencies, and exports image-building recipes.
An agent is one workload kind, not a hardcoded provider or harness integration.

```text
nix-tooling                 shared compiler, package-set and development inputs
       |                    followed by Workestrate and its Microsandbox input
       v
Workestrate tool            Rust CLI + Nix packages and reusable image recipes
       ^
       | declarations       TOML, layers, policy, mounts, seeds, secret bindings
fleet / config repositories own application choices and workload image flakes
       |
       v
resolved plan -> lifecycle -> pinned Microsandbox SDK / runtime / guest agent
                                      |
                                      +-- service microVM: detached execution
                                      +-- agent microVM: interactive execution
```

The diagram describes responsibility, not a network topology. Host-bound secret
substitution, guest execution, and SSH custody are distinct mechanisms. Runtime
capabilities and enforcement depend on the selected backend, image, and host.

### Vocabulary

| Term | Meaning here |
| :--- | :--- |
| Workload | A named declaration of image, command, environment, mounts, seeds, resources, and policy. |
| Fleet / config repository | Operator-owned composition. The current CLI calls registered configuration repositories `config`; a fleet can contain multiple workload capsules. |
| Context | An ordered selection of configuration layers. It also affects runtime slot identity. |
| Slot / instance | A singleton workload identity, optionally qualified by context, or a parallel instance with its own ID. |
| Tool home | Registry, config clones, sources, overrides, secrets ciphertext, and tool state; normally `$WORKESTRATE_HOME`, default `~/.workestrate`. |
| Microsandbox home | Backend runtime state, selected separately through `MSB_HOME`; packaged defaults use `~/.microsandbox/current`. |
| Build runtime | Immutable SDK compilation inputs selected by `MSB_BUILD_RUNTIME`, not a mutable runtime home. |

## Ownership and repository map

| Owner / path | Responsibility |
| :--- | :--- |
| [control/agentctl/](control/agentctl/) | Rust CLI, configuration resolution, planning, policy integration, lifecycle, and backend integration. |
| [flake.nix](flake.nix), [flake.lock](flake.lock) | Selected upstream inputs, package composition, checks, and reusable flake exports. |
| [nix/](nix/) | Packages and shared build recipes. `nix/packages/` packages the tool; `nix/lib/` owns reusable configuration/image helpers. |
| [config.reference/](config.reference/) | Synthetic fallback with placeholder workloads, not an operator fleet. |
| [templates/](templates/), [schemas/](schemas/) | Scaffolding and repository-owned configuration schemas. |
| [scripts/](scripts/) | Verification, provisioning, secrets helpers, Git hooks, and store utilities. Some scripts are explicitly state-changing. |
| [.github/](.github/) | Workflow implementation, review ownership, and repository automation. |
| [docs/](docs/README.md) | Focused guides, specifications, and decisions. [ADR index](docs/migration/50-decisions/README.md) records rationale and supersession. |
| [.beads/](.beads/README.md) | Existing task tracker; acceptance criteria and dependencies belong with the task. |
| Fleet / workload repositories | Application images, provider choices, deployment configuration, and application-specific acceptance tests. |

### The dependency boundary

[nix-tooling](https://github.com/rybskiworks/nix-tooling) owns the shared nixpkgs,
Fenix, devenv, and formatting versions. Workestrate follows those inputs rather
than independently choosing a second toolchain. Inspect `flake.nix` and
`flake.lock` for the selected revisions instead of copying volatile pins here.

CI derives its exact Rust release from the locked Fenix manifest. Validate the
supplier relationship without building the application:

```sh
python3 scripts/ci/toolchain.py check --role consumer
```

The [Microsandbox fork](https://github.com/rybskiworks/microsandbox) owns the SDK,
runtime, and static `agentd` packages. Workestrate consumes paired artifacts from
that pinned source. SDK version pins and root Cargo patches must stay consistent;
Cargo does not inherit a dependency workspace's patches. The vendor SDK path is
a prepared symlink, not a checked-in copy of the fork.

Workload images belong to the config repository or workload capsule that declares
them. The tool exports `lib` recipes, including `buildImagesFromConfig`,
`buildWorkloadImage`, and configuration checks. It does not own a personal fleet's
application builds. See [Nix ownership](docs/nix-build.md) and
[workload ownership](docs/workloads.md).

## Configuration model

A config repository uses `workestrate.toml` or directory-mode `workestrate/`
capsules. It owns its secrets schema, `.env.enc`, and `.sops.yaml`. Registered
clones live under `$WORKESTRATE_HOME/config-repos/<name>/`; managed source
checkouts live under `$WORKESTRATE_HOME/sources/<name>/`.

`WORKESTRATE_CONFIG_DIR` is an explicit development/testing **single-layer
bypass**, not the first layer in a merged stack. Otherwise resolution uses the
reference fallback or selected context layers in declared order, followed by
user-global overrides, trusted project configuration, and trusted local overrides.
The project files are `./workestrate.toml` and `./workestrate.local.toml`;
untrusted working directories do not silently contribute these layers.
Use `workestrate workload plan <name> --show-source` to inspect field provenance.

Run workload verbs from the intended project directory. The CLI captures the
invocation directory once at entry (`WORKESTRATE_INVOKE_CWD`); `${CWD}` mount
hosts, project-config discovery, and per-directory slots resolve against that
captured value, not against a later or detached child's working directory.

Merge behavior is security-aware, not generic last-write-wins: deny and egress
rules are additive unions, environment bindings merge by key, and other values
use merge-patch semantics. Egress hosts are checked against the closed
`ALLOWED_EGRESS_HOSTS` vocabulary. The policy ladder permits standalone `allow`;
a home-level `final` seals its veto. The retired `entitlements` key is a hard
parse error. Consult the [security model](docs/migration/30-security-model.md)
and ADR 0035 before changing this behavior.

### Images, seeds, and workload ownership

A workload's image flake can live beside its declaration in a fleet capsule or
at the config-repository root. The CLI selects the nearest declaring-source
flake for `nix-layered` images. Adding a flake does not convert a registry-image
workload into a Nix-built image. Locks are explicit; builds refuse implicit
lock updates. Relative seed and mount paths retain their configuration roots,
not the image flake's directory. See [workload layouts](docs/workloads.md).

`[[seed_files]]` copies files before start. `template = true` renders `${VAR}`
against the guest-visible environment; host-bound credentials remain `$MSB_<name>`
placeholders, and missing variables fail. `glob` seeds sorted matches beneath the
target. `only_if_missing = false` permits re-rendering; `--reseed` forces template
re-rendering. The [target specification](docs/migration/20-target-system-spec.md)
owns the complete seed contract.

## Lifecycle and runtime state

Services use `workload up` for detached, dependency-ordered execution; agents use
`workload exec` for interactive attachment. A configured service or agent name is
not a built-in CLI integration.

`up` and `exec` target a slot: `<workload>`, `<context>-<workload>`, or a parallel
`<slot>@<id>` instance. Occupied slots refuse by default. `--replace` explicitly
recycles; `--instance <id>` and `--new` create parallel instances with independent
loopback addressing. `host = 0` port declarations and `--port-auto` probe a free
port at boot. See [operating semantics](docs/operating-model.md) and ADRs 0021/0026.

Scoped teardown takes exactly one selector per invocation. The safe,
narrow example is `workestrate down --context <ctx>`. Alternatives are
`down --all`, `down --config-ref <branch>`, and the double-gated
`down --everything --everything --yes`; these are alternatives, not one
command to paste verbatim. The instance/workload rung stays on
`workload <name> down [--instance <id> | --all-instances]`.
`workestrate clean` handles state/cache hygiene and does not tear down VMs.

The packaged runtime defaults to `~/.microsandbox/current`, pointing to a
state generation for the pinned runtime. A nonempty `MSB_HOME` is a separate,
explicit backend-state override. A tool `--home` does not relocate every backend
root or erase inherited backend configuration. Generation convergence and live
migration are explicit operations, not consequences of a build or shell entry.

`MSB_BUILD_RUNTIME` and `MSB_AGENTD_PATH` select immutable build artifacts.
`just shell` prepares the SDK development environment, but neither it nor
`just bootstrap` provisions a fleet, installs hooks, or migrates runtime state.
`workestrate msb -- <args>` reaches the paired runtime; `workestrate versions`
reports the tool/runtime/guest-agent identities without starting guest bootstrap.

Guest `init` handoff and managed `root_disk_mib` capacity are create-time settings,
not live reconfiguration or resize guarantees. Reusing an existing sandbox keeps
its stored specification. A returned handle is not proof of guest readiness or
clean shutdown. The [runtime guide](docs/runtime-provisioning.md) owns the exact
contracts, environment override matrix, generation behavior, and SSH integration.

## CLI reference

Placeholders such as `<name>` refer to your selected configuration. The synthetic
reference includes `example-service`, `example-agent`, and `example-offensive`;
it supports inspection, not deployment of real applications.

```sh
workestrate workload plan example-service
workestrate workload plan <name> --show-source
workestrate workload up <service>                # detached service and dependencies
workestrate workload up <service> --foreground   # run until Ctrl-C
workestrate workload logs <service>
workestrate workload exec <agent>                # interactive attachment
workestrate workload down <name>
workestrate workloads
workestrate ps --json
```

| Area | Commands and purpose |
| :--- | :--- |
| Home | `home init`; `home clone <src> [dest]`; `migrate-home` for the documented legacy layout migration. These mutate state. |
| Config | `config add <url> <name>`; `config new <name> [dest]`; `config update [name]`; `config list`; `config trust <dir>`. Updating refuses dirty clones; trust explicitly enables project layers. |
| Sources | `source clone`, `source build`, `source list`, `source reset` manage source checkouts. Review mutation/reset scope before use. |
| Diagnostics | `check`, `doctor`, `versions` inspect layout, provisioning, and selected pins. |
| Schema | `validate-config`, `secrets-schema`, `generate-env-example`, `generate-schema`, `schemas update`. These have different output/write behavior; inspect command help. |
| Shells | `completions <shell>` emits completion definitions; shell completion support is not host-platform support. |
| Runtime | `msb -- <args>` is a passthrough to the exact pinned Microsandbox CLI. |
| Operator escape hatch | `run -- <cmd>` decrypts secrets into a host process environment. It is deliberately not a sandboxed workload. |

## Setup

The packaged system targets x86_64 Linux. Runtime execution needs accessible
`/dev/kvm`, enabled virtualization, and host prerequisites; the host provisioning
scripts target Debian/Ubuntu. The introductory workflow is in
[README.md](README.md#take-a-look), with operator setup in the
[getting-started guide](docs/getting-started.md). `just host-check` checks
prerequisites. Sizing depends on the workload, image builds, and retained Nix
store; minimum estimates are not capacity guarantees for a real fleet.

Once the host is provisioned, provision the tool home with a config repo
attached, then pair `--home` (WHERE the registry lives) with `--config`
(WHICH registered config to target) on every secrets command:

```sh
workestrate home init --config <your-config-repo-url> --name personal
# Second machine from an existing home:
# workestrate home clone <src-home-or-git-url> [dest-dir]
workestrate --home <tool-home-path> config list
workestrate context current
workestrate validate-config
workestrate --home <tool-home-path> secrets update --config personal
workestrate secrets init --config personal   # first bootstrap only; use update after
workestrate check
```

`--home` without `--config` on a `secrets` command selects no target. Enable
a project directory's local layer explicitly with
`workestrate config trust <dir>`; never trust arbitrary checkouts.

These commands initialize operator state. Scaffolding a config is not installing
a working agent stack. Define real workloads and their image builds, inspect the
resolved plan, then use `workload up <service>` or `workload exec <agent>` with
names from that configuration. The [workload guide](docs/workloads.md) links
independently maintained baseline and communication fixtures.

## Secrets workflow

Each config repository carries SOPS/age-encrypted `.env.enc` and `.sops.yaml`.
The optional user-global `$WORKESTRATE_HOME/secrets/.env.local.enc` overlays values
per key. Later selected secret layers win; process environment is lowest priority.
Required variable names belong to the active config's schema, not a fixed list
of personal providers in this tool README.

```sh
workestrate secrets init --config <name>
workestrate secrets update --config <name>
workestrate --home <tool-home-path> secrets update --config <name>
workestrate secrets update --config-dir /path/to/config-repo
```

`--home` selects WHERE the registry is read from; `--config` (or
`--config-dir`) selects WHICH target to provision. `--home` without
`--config` selects no target.

`init` creates an age key with mode 0600 when needed, prepares recipients, and
opens an editor for required values; it refuses to overwrite existing ciphertext.
Use `update` for an existing file, including `--global update` for the global
layer. Named targets resolve through `workestrate secrets target`, including
registry/file/key overrides; unknown names fail without selecting another target.

The default age private key is
`~/.config/sops/age/ai-workbench-secrets.txt`. It stays on the host, outside
repositories and the tool home, which may be exposed through workload mounts.
Back up the key separately; ciphertext alone cannot recover the secrets. Missing
keys fail closed. [docs/secrets.md](docs/secrets.md) owns the threat model.

Host-bound credentials expose placeholders to guests and substitute real values
on approved host-side traffic to `allowed_hosts`. Guest-bound bindings intentionally
place the real value in the guest, for workloads that need to verify or use it
there. These are different exposure decisions, not equivalent isolation claims.
`workestrate run -- <cmd>` intentionally grants its host process decrypted secrets;
commands such as `plan`, `check`, and `completions` do not need secret decryption.
SSH custody has separate runtime acceptance requirements.

## Development workflow

Run `just <recipe>` from a plain host shell. The [Nix build guide](docs/nix-build.md)
is the canonical reference for package ownership, SDK preparation, offline
staging, local overrides, and verification. Use `just shell -c <command>` or
`just bootstrap -c <command>` for explicit shell commands. Follow
[CONTRIBUTING.md](CONTRIBUTING.md) for integration and review conventions.

| Recipe | Scope |
| :--- | :--- |
| `just shell` | Full pinned development environment and SDK setup. |
| `just bootstrap` | Tooling-only shell when the application/runtime is not buildable. |
| `just verify` | Source guards and sandboxed package, Rust, unit, TOML, formatting, schema, and dependency-policy checks. No fleet deployment. |
| `just verify-full` | Verification plus an explicit package build. |
| `just check` / `just test` | Focused Rust checks / unit tests. |
| `just deny-check` | Dependency policy from `deny.toml`. |
| `just host-check` / `just provision-check` | Host prerequisites / read-only provisioning inspection. |
| `just host-provision` | Explicit host provisioning, not a build check. |
| `just kvm-tests` | Separate host-side ignored KVM tests with an explicitly safe test environment. |
| `just lint-nix` / `just tombi-check` | Purity and TOML gates. |
| `just gc` / `just store-audit` | Store hygiene / inspection, not runtime acceptance. |

Both shells and bare `just` preserve an explicit external absolute
`CARGO_TARGET_DIR`. Otherwise the default is
`${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target`. Relative or
checkout-internal targets, including symlink aliases, are rejected before setup.
Stage new required files before Nix evaluates the Git source. See
[Nix purity](docs/nix-purity.md) and [devshell rules](docs/nix/devshells.md).

### Work tracking, hooks, and review

`just beads ready` lists actionable work; `just beads show <id>` exposes criteria
and dependencies in the [existing tracker](.beads/README.md). The repository-wide
tracking contract is in [BEADS.md](BEADS.md). Beads is pinned by nix-tooling.
Embedded storage permits one writer at a time. The wrapper preserves nonempty
`BEADS_DIR` and `DOLT_ROOT_PATH`, otherwise defaulting to `$PWD/.beads` and
`$BEADS_DIR/dolt-global`. Explicit absolute paths select shared state; the
wrapper does not initialize a tracker, bind remotes, or coordinate writers.

`nix run .#install-hooks` installs pinned hooks explicitly and preserves the
legacy hook chain. Hooks are check-only; fix findings with hooks enabled rather
than bypassing verification. Use conventional commit subjects (`feat:`, `fix:`,
`docs:`, `chore:`, `ci:`, `build:`). ADR numbers belong in bodies/footers. Commit
ciphertext, never decrypted secret material.

Workflow/check selection is owned by [.github/](.github/) and
[scripts/ci/](scripts/ci/). The [CI foundation guide](docs/ci-release-foundation.md)
explains docs-only selection and the required-check contract. Heavy `nix-ci` runs
are owner/maintainer-triggered, not an agent self-applied label. Prefer these
sources to a second, easily stale CI implementation description here.

Repository schema checks cover owned templates. Inspect a deployed tool home
explicitly with `workestrate --home <tool-home-path> schemas update --check`
(where the registry/schema state of that home is the inspection target);
consumer homes are not inputs to the repository verification gate.

<a id="flag-glossary"></a>

## Flag glossary

The global `--context` spelling and the `down --context` spelling select
different things. Do not merge them. No flag is renamed by this change.

| Flag or variable | Selects | Does not select |
| :--- | :--- | :--- |
| `--home <DIR>` | Tool-home root: registry (`config.toml`), overrides, state/store dirs, config-repo checkouts. Highest-precedence `WORKESTRATE_HOME` step. | Which config layers are active; backend runtime state (`MSB_HOME`); full process isolation (see `AGENTS.md`). |
| `--context <NAME>` (global) | Active named layer bundle (`[contexts.<name>] layers`), plus the `<ctx>-` slot-prefix namespace. | Which sandbox records `down` stops; the pinned git ref. |
| `--config-ref <REF>` | Pinned-consumption rung: every git-backed config entry is read at this branch/sha; a branch-shaped ref also feeds context derivation. | A sandbox-record selector (see `down --config-ref`). |
| `WORKESTRATE_CONFIG_DIR` | Explicit single-layer development/testing bypass, resolved before the registry. | The operator path; the first layer of a merged stack. |
| `down --context <CTX>` | Sandbox records whose record context (primary) or `<ctx>-` slot prefix (corroborating) matches. | The config layer bundle. |
| `down --config-ref <REF>` | Records whose implied context matches a validated branch-shaped ref; a sha implies nothing and is refused. | The pinned-consumption layer. |
| `MSB_HOME` | Backend Microsandbox runtime state, separate from the tool home. Packaged default `~/.microsandbox/current`. | Tool-home registry, overrides, or config checkouts. |
| `WORKESTRATE_INVOKE_CWD` | The operator's invocation directory, captured once at CLI entry and inherited by re-exec'd or detached children. | A live re-resolution of the current working directory. |

Related names that are not these flags: the `home` subcommand (dotfiles-style
tool-home provisioning), `migrate-home` (one-time legacy XDG consolidation),
`--config <name>` (which registered config a `secrets` command targets),
`--config-dir <DIR>` (explicit config directory target), and the `context`
subcommand family (`list | current | use <name>`; contexts are otherwise
hand-edited in the registry TOML).

## Task-to-source map

| Task | Read for the relevant boundary |
| :--- | :--- |
| Understand the whole system | [Specification map](SPEC.md), [target specification](docs/migration/20-target-system-spec.md), and the [ADR index](docs/migration/50-decisions/README.md). |
| Change config merge, defaults, or trust | [Security model](docs/migration/30-security-model.md), ADR 0035, `control/agentctl/`, `schemas/`, and `templates/`. |
| Change image builds or fleet layout | [Workload layouts](docs/workloads.md), `nix/lib/`, and the declaring fleet/workload repository. |
| Change slot identity or teardown | [Operating model](docs/operating-model.md), target specification, ADRs 0021/0026. |
| Change runtime pins, state, init, or disk capacity | [Runtime provisioning](docs/runtime-provisioning.md), [Nix ownership](docs/nix-build.md), and the exact selected fork source. |
| Change secrets or credential exposure | [Secrets](docs/secrets.md), [security model](docs/migration/30-security-model.md), and applicable broker/runtime acceptance evidence. |
| Change compiler, SDK, or dependency packaging | [Nix ownership](docs/nix-build.md), [purity](docs/nix-purity.md), `flake.nix`, `flake.lock`, and `control/agentctl/` Cargo manifests/patches. |
| Add a regression or runtime experiment | [Testing](docs/testing.md); keep tool contracts here and application fixtures with their workload owner. |
| Change CI or release gates | [CI foundation](docs/ci-release-foundation.md), [governance](docs/github-governance.md), `.github/`, `scripts/ci/`, and `deny.toml`. |
| Report a vulnerability | [Security reporting](SECURITY.md); keep sensitive details out of public issues. |
| Change documentation or visual assets | This document for orientation, [docs index](docs/README.md) for detailed ownership, [asset notes](docs/assets/README.md) for presentation. |

## Runtime status

Build success proves neither fleet deployment nor a security property. Separate
configuration/plan checks, package/unit checks, image checks, disposable VM tests,
and deployed application acceptance. Record the exact tool, runtime, and image
revisions with positive controls, observed refusals, readiness, and cleanup.

`virtualization.nested` requests nested virtualization on supported hosts. The
bundled runtime/firmware can boot nested guests, but the current Linux off flag
is not an enforced confinement boundary. Successful nesting does not establish
that an omitted or disabled setting prevents it. Validate both directions.

Lifecycle readiness/cleanup, mount-policy enforcement, SSH custody, and the full
agent/memory-service composition have separate outstanding runtime gates. Older
Pi/Prime startup results and ignored KVM tests do not close those gates.

Fleet-local image flakes and standalone config repositories exist; fleet-level
importing/pinning of standalone workload repositories is not yet implemented.
The current `workload new` scaffolder is not directory-mode aware. See the explicit
limitations in [docs/workloads.md](docs/workloads.md). Shared-store, alternative
backend, snapshot, or fork explorations are not implementation evidence simply
because they appear in a design note.

Runnable baseline, communication, and nested fixtures belong to their owners;
the [personal fleet repository](https://github.com/rybskiworks/workestrate-fleet-georgrybski)
maintains examples under `tests/workloads/`. It is a reference consumer, not a
mandatory dependency of this tool's checks. [docs/testing.md](docs/testing.md)
and [docs/runtime-provisioning.md](docs/runtime-provisioning.md) describe how to
separate those acceptance claims.
