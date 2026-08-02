---
type: Reference
resource: https://nix.dev/concepts/flakes.html
title: Flake Anatomy
description: Reference for the flake.nix structure — inputs, outputs, flake.lock, flake registry, and flake commands.
tags: [nix, flakes, flake-nix, inputs, outputs]
timestamp: 2026-07-24T01:30:00Z
---

# Flake Anatomy

## Purpose

This document is the authoritative reference for the `flake.nix` structure —
inputs, outputs, `flake.lock`, the flake registry, and the flake command set.
Future agents who author, review, or validate a Nix flake should follow these
rules so the flake matches the official reference exactly and remains
reproducible.

## Sources used

- `.crawl/46-flakes.md` — https://nix.dev/concepts/flakes.html (PRIMARY — flake
  definition, `flake.nix` structure, inputs, outputs, `flake.lock`, registry,
  commands, purity, history, alternatives)
- `.crawl/45-concepts-faq.md` — https://nix.dev/concepts/faq.html (experimental
  feature status, `nix-command flakes`)
- `.crawl/43-pinning-nixpkgs.md` — https://nix.dev/reference/pinning-nixpkgs.html
  (lock file pinning)
- https://wiki.nixos.org/wiki/Flakes (community wiki — command reference,
  registry)
- https://github.com/NixOS/rfcs/pull/49 (RFC 49 — original flakes proposal)

## Related Nix guidance

- `source-map.md` — provenance index for all crawled Nix sources.
- `nix-purity.md` — deeper treatment of hermetic evaluation and impure escapes.
- `nix-store-accumulation-report.md` — GC roots and store hygiene in practice.

## Core guidance

A flake is a self-contained, reproducible source for Nix code. Flakes are an
**experimental feature** requiring **Nix 2.4+** and the
`nix-command flakes` experimental-features flag.

> Flakes offer an entrypoint file `flake.nix` aimed at sharing Nix code. They
> make it easy to build programs with the same version.
> — [nix.dev/flakes]

### What is a flake?

A flake is a directory containing a `flake.nix` file (and a generated
`flake.lock`). It declares its dependencies (`inputs`) and what it exports
(`outputs`) with a standard structure.

> `flake.nix` is a file that declares inputs and outputs with a standard
> structure.
> — [nix.dev/flakes]

Flakes replace ad-hoc `import` chains and channel-based pinning with a
declarative, lockable dependency graph. Every input is pinned by content hash in
`flake.lock`, making evaluation reproducible across machines and time.

### flake.nix structure

The top-level is a function `{ ... }: { ... }` returning an attribute set with
three keys: `description`, `inputs`, and `outputs`.

Minimal example from the primary source:

```nix
{
  description = "My example flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }: {
    packages.x86_64-linux = {
      default = self.packages.x86_64-linux.hello;
      hello = nixpkgs.legacyPackages.x86_64-linux.hello;
    };
  };
}
```

### inputs attribute

The `inputs` attribute set maps input names to flake references. Each input has
a `url` field using a schema, plus optional modifiers.

#### Input URL schemas

| schema | example | meaning |
|--------|---------|---------|
| `github:` | `github:owner/repo` | GitHub repo at default branch |
| `github:` | `github:owner/repo/ref` | GitHub repo at a branch/tag |
| `github:` | `github:owner/repo?ref=branch` | GitHub repo with query-form ref |
| `path:` | `path:./subdir` | Local path (relative or absolute) |
| `tarball:` | `tarball+https://example.org/x.tar.gz` | Tarball fetched over HTTPS |
| `git+https:` | `git+https://example.org/repo.git` | Git repo over HTTPS |
| `git+ssh:` | `git+ssh://git@example.org/repo.git` | Git repo over SSH |
| `https:` | `https://example.org/repo.git` | Generic HTTPS fetch |

#### Input options

| option | meaning |
|--------|---------|
| `inputs.<name>.url` | Flake reference (required). |
| `inputs.<name>.follows` | Override a transitive input to follow your version of `<name>`. |
| `inputs.<name>.flake = false` | Import a non-flake repo as a plain source path (no `outputs` expected). |
| `inputs.<name>.overrides` | Override sub-inputs of a transitive dependency. |

The `follows` mechanism keeps a single nixpkgs version across the dependency
graph. Example from the primary source — making `home-manager` use the root
flake's `nixpkgs`:

```nix
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
}
```

### outputs attribute

`outputs` is a function that receives `{ self, ...inputs }`. The `self`
argument is the flake's own outputs (for self-reference). The function returns an
attribute set of typed outputs.

> [`outputs`] include various [built-in types], but can be [extended].
> — [nix.dev/flakes]

#### Built-in output types

| output | purpose |
|--------|---------|
| `packages.<system>.<name>` | derivations (buildable) |
| `devShells.<system>.<name>` | development shell environments |
| `lib.<system>` or `lib` | library functions |
| `apps.<system>.<name>` | runnable programs |
| `checks.<system>.<name>` | test derivations |
| `nixosModules.<name>` | NixOS modules |
| `nixosConfigurations.<name>` | full NixOS system configs |
| `overlays.<name>` | nixpkgs overlays |
| `templates.<name>` | project templates |
| `formatter.<system>` | formatter (e.g. `nix fmt`) |

The `default` name (e.g. `packages.<system>.default`, `devShells.<system>.default`)
is the canonical entry point for each output class.

### flake.lock

The lock file pins all input versions by content hash. It is generated and
updated automatically by Nix.

> Nix creates a `flake.lock` to pin dependencies once you run a `nix` command.
> — [nix.dev/flakes]

#### Lock file structure

| key | meaning |
|-----|---------|
| `nodes` | Object keyed by node name. Each node has `original` (the declared ref), `locked` (the pinned rev + hash), and `inputs` (sub-input references). |
| `root` | Points to the root node (the flake itself). |
| `version` | Lock file schema version (currently `7`). |

Abbreviated real `flake.lock` from this project:

```json
{
  "nodes": {
    "nixpkgs": {
      "locked": {
        "lastModified": 1780749050,
        "narHash": "sha256-3av0pIjlOWQ6rDbNOmpUSvbNnJkGORQKKjb4LtCZsIY=",
        "owner": "NixOS",
        "repo": "nixpkgs",
        "rev": "a799d3e3886da994fa307f817a6bc705ae538eeb",
        "type": "github"
      },
      "original": {
        "owner": "NixOS",
        "ref": "nixos-unstable",
        "repo": "nixpkgs",
        "type": "github"
      }
    },
    "root": {
      "inputs": {
        "fenix": "fenix",
        "nixpkgs": "nixpkgs"
      }
    }
  },
  "root": "root",
  "version": 7
}
```

#### Lock update commands

| command | meaning |
|---------|---------|
| `nix flake update` | Re-lock all inputs (regenerate `flake.lock`). |
| `nix flake lock --update-input <name>` | Update a single input. |
| `nix flake lock` | Create the lock file without updating existing entries. |

### Flake registry

`nix registry` manages aliases that map short names (e.g. `nixpkgs`) to flake
references. The default `nixpkgs` resolves via the registry.

> Aliases to flakes are stored in a registry. This can be extended by
> [command-line] or by NixOS option `nix.registry`.
> — [nix.dev/flakes]

| command | meaning |
|---------|---------|
| `nix registry list` | List all registry aliases. |
| `nix registry add <name> <ref>` | Add an alias. |
| `nix registry pin <name>` | Pin an alias to its current locked version. |

### Flake commands

| command | purpose |
|---------|---------|
| `nix build .#<attr>` | build a flake output |
| `nix develop` | enter the devShell |
| `nix run .#<app>` | run an app |
| `nix flake check` | validate flake structure |
| `nix flake show` | show all outputs |
| `nix flake init` | create `flake.nix` from template in current dir |
| `nix flake new` | create `flake.nix` in a new dir |
| `nix flake update` | update `flake.lock` |
| `nix flake lock` | create/update lock file |

All flake commands require the experimental-features flag:

```sh
nix --experimental-features 'nix-command flakes' flake show
```

Or set persistently in `~/.config/nix/nix.conf` or `/etc/nix/nix.conf`:

```
experimental-features = nix-command flakes
```

### Flake purity

Flakes default to **pure mode** — builds are isolated from the host environment.

> Flakes default to pure mode, isolating builds from the host environment. This
> is also called hermetic evaluation, and prevents evaluating (non-network)
> impure functions.
> — [nix.dev/flakes]

Consequences of pure mode:

- **Git-tracked files only** — untracked files are invisible to the flake. Run
  `git add` (even without committing) before referencing new files.
- No access to environment variables unless explicitly passed.
- No network access during evaluation (fetches happen via inputs, not ad-hoc
  `builtins.fetchurl`).

Escapes:

- `--impure` flag allows impure evaluation (breaks reproducibility).
- `--override-input <name> <ref>` overrides an input at the command line
  (useful for testing local forks without editing `flake.nix`).

### Flake history and criticism

> Flakes were proposed in RFC 49, and introduced in a blog post.
> — [nix.dev/flakes]

> The flakes proposal was criticised for trying to solve too many problems at
> once and at the wrong abstraction layer.
> — [nix.dev/flakes]

> While there were still outstanding concerns about the design, the
> implementation was merged without the RFC having been accepted (and in fact
> being withdrawn on merge), raising questions about proper process.
> — [nix.dev/flakes]

The community split that followed: **Determinate Systems** declared flakes
stable and ships them enabled-by-default in their Nix installer; the **Lix**
fork consolidated the de-facto "v1" of the flakes interface. Upstream Nix still
marks flakes experimental, though the feature is widely considered production-ready
in practice.

### Alternatives to flakes

| tool | role |
|------|------|
| `niv` | Older source-pinning tool (`niv init`, `niv update`). Pre-dates flakes. |
| `npins` | Newer, flake-compatible pinning tool. |
| channels | Traditional Nix mechanism (`nix-channel --add`). Mutable, not reproducible across machines. |
| `flake-compat` | Expose flake outputs to non-flake users via a `default.nix` / `shell.nix` shim. |
| `builtins.fetchTree` | From the experimental `fetch-tree` feature; lower-level than full flakes. |

## Practical rules

1. A flake is a directory with a `flake.nix`; the top-level is a function
   returning `{ description, inputs, outputs }`.
2. Flakes require Nix 2.4+ and `experimental-features = nix-command flakes`.
3. Every input has a `url`; the `github:` schema is the most common.
4. Use `inputs.<name>.flake = false` for non-flake repos (plain source paths).
5. Use `inputs.<name>.inputs.<dep>.follows = "<root-input>"` to deduplicate
   nixpkgs (or any shared input) across the dependency graph.
6. `outputs` receives `{ self, ...inputs }`; `self` is the flake's own outputs.
7. Use the `default` name for the canonical entry point of each output class.
8. `flake.lock` pins all inputs by content hash; commit it to the repo for
   reproducibility.
9. `nix flake update` re-locks everything; `--update-input <name>` updates one.
10. Untracked files are invisible to a pure flake — `git add` new files first.
11. `--impure` and `--override-input` break reproducibility; use deliberately.
12. The flake registry maps short names (`nixpkgs`) to refs; `nix registry pin`
    freezes an alias.

## Review checklist

- [ ] `description` string present and meaningful.
- [ ] Every input has a `url`.
- [ ] Non-flake inputs set `flake = false`.
- [ ] Shared inputs (nixpkgs) use `follows` to avoid version divergence.
- [ ] `outputs` function signature includes `self` and all named inputs.
- [ ] `default` entry points exist for each output class used.
- [ ] `flake.lock` is committed and matches `flake.nix` inputs.
- [ ] No `builtins.getFlake` or impure fetches in pure evaluation paths.
- [ ] `--impure` / `--override-input` are not baked into committed scripts.

## Implementation checklist

- [ ] Start from `nix flake init` (or `nix flake new`) scaffold.
- [ ] Set `description` to a one-line project summary.
- [ ] Add `nixpkgs.url` (prefer a tagged ref like `nixos-unstable`).
- [ ] Add project-specific inputs; mark non-flake repos with `flake = false`.
- [ ] Add `follows` for any input that depends on nixpkgs.
- [ ] Implement `outputs` with the output classes you need (`packages`,
      `devShells`, `apps`, `lib`, `checks`).
- [ ] Provide `packages.<system>.default` and `devShells.<system>.default`.
- [ ] Run `nix flake lock` to generate the lock file.
- [ ] `git add flake.nix flake.lock` (and any new files referenced).
- [ ] Run `nix flake check` and `nix flake show` to validate.

## Validation hooks

- `nix flake check` — validates flake structure and runs `checks`.
- `nix flake show` — lists all outputs (confirms the attr tree).
- `nix build .#<attr>` — builds an output (dry-run via `--dry-run`).
- `nix flake update --dry-run` — preview lock changes without writing.
- `nix develop` — enters the devShell (smoke test that the shell evaluates).

## Examples

### Real-world flake (this project)

Abbreviated `flake.nix` from `/home/node/Development/ai-workbench/flake.nix`,
highlighting the key structures:

```nix
{
  description = "ai-workbench — local AI workbench for Pi/Odysseus/OpenCode through Microsandbox + LiteLLM";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # Non-flake inputs — plain source paths
    pi = {
      url = "github:georgrybski/pi";
      flake = false;
    };

    odysseus = {
      url = "github:georgrybski/odysseus";
      flake = false;
    };

    opencode = {
      url = "github:georgrybski/opencode";
      flake = false;
    };

    tempest = {
      url = "github:georgrybski/T3MP3ST";
      flake = false;
    };

    # Flake input with follows — fenix uses the root nixpkgs
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, pi, odysseus, opencode, tempest, fenix, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      rustToolchain = fenix.packages.${system}.stable;
      # ... (package definitions elided) ...
    in {
      devShells.${system}.default = import ./nix/devshells/default.nix {
        inherit pkgs microsandbox workestrate rustToolchain;
        # ...
      };

      lib.${system} = libForSystem { inherit pkgs; };

      packages.${system} = workload-images // {
        inherit workestrate workestrator microsandbox;
        default = workestrate;
      };

      apps.${system}.default = {
        type = "app";
        program = "${workestrator}/bin/workestrate";
      };
    };
}
```

### Real-world flake.lock (this project)

The `fenix` node showing `inputs.nixpkgs` following root's nixpkgs, the `root`
node listing all inputs, and `version: 7`:

```json
{
  "nodes": {
    "fenix": {
      "inputs": {
        "nixpkgs": ["nixpkgs"],
        "rust-analyzer-src": "rust-analyzer-src"
      },
      "locked": {
        "lastModified": 1784709340,
        "narHash": "sha256-qsmQMPL+qInjXfdjX0RquObSKcs5h/4D63IWH9j4+Ro=",
        "owner": "nix-community",
        "repo": "fenix",
        "rev": "fa09e6473a0dfd673e6cb9a37741aec513b4bb2a",
        "type": "github"
      },
      "original": {
        "owner": "nix-community",
        "repo": "fenix",
        "type": "github"
      }
    },
    "root": {
      "inputs": {
        "fenix": "fenix",
        "nixpkgs": "nixpkgs",
        "odysseus": "odysseus",
        "opencode": "opencode",
        "pi": "pi",
        "tempest": "tempest"
      }
    }
  },
  "root": "root",
  "version": 7
}
```

Note how `fenix.inputs.nixpkgs` is `["nixpkgs"]` — an array path pointing at
the root's `nixpkgs` input, which is how `follows` is serialized in the lock
file.

## Common mistakes

- Forgetting `flake = false` on a non-flake repo (Nix tries to evaluate its
  `outputs` and fails).
- Not using `follows` for nixpkgs — results in multiple nixpkgs versions in the
  store, bloating builds and causing glibc mismatches.
- Referencing a file that is not `git add`-ed — the flake cannot see it in pure
  mode.
- Committing `flake.lock` with stale entries after editing `inputs` without
  running `nix flake lock`.
- Using `--impure` in CI scripts — breaks reproducibility silently.
- Setting `experimental-features` only in the user config, not in CI — flakes
  work locally but fail in CI.
- Confusing `nix flake update` (re-locks all) with `nix flake lock` (creates
  without updating existing).

## Strict vs contextual guidance

Strict: `flake.nix` top-level is `{ description, inputs, outputs }`; every input
has a `url`; non-flake inputs set `flake = false`; `outputs` receives
`{ self, ...inputs }`; `flake.lock` is committed; pure mode is the default;
`default` names are the canonical entry points.

Contextual: choice of nixpkgs ref (`nixos-unstable` vs a release tag); whether
to use `follows` for non-nixpkgs inputs; granularity of output classes
(`packages` only vs full `checks`/`nixosModules`/`templates`); whether to pin
the registry; whether to enable `--impure` for local dev convenience.

## Policy decisions for individual repos

- Choose nixpkgs ref (`nixos-unstable` vs `nixos-24.11`).
- Decide which inputs need `follows` (at minimum, nixpkgs).
- Decide which output classes to expose (`packages`, `devShells`, `apps`,
  `lib`, `checks`).
- Decide whether to ship a `default.nix` via `flake-compat` for non-flake
  users.
- Decide whether to pin the flake registry (`nix registry pin`).
- Decide CI's `experimental-features` configuration.

## Related docs

- `source-map.md` — provenance index for all crawled Nix sources.
- `nix-purity.md` — hermetic evaluation and impure escapes (TBD expansion).
- Future nix topic docs: `nix-devshells.md`, `nix-derivations.md`,
  `nix-overlays.md` (TBD).

## Related skills

- `nix-usage` — operational reference for the ai-workbench Nix flake, dev shell,
  Rust toolchain, and Microsandbox runtime.

## Citations

[1] [Flakes — nix.dev](https://nix.dev/concepts/flakes.html)
[2] [Frequently Asked Questions — nix.dev](https://nix.dev/concepts/faq.html)
[3] [Pinning Nixpkgs — nix.dev](https://nix.dev/reference/pinning-nixpkgs.html)
[4] [Flakes wiki article](https://wiki.nixos.org/wiki/Flakes)
[5] [RFC 49: Flakes](https://github.com/NixOS/rfcs/pull/49)
