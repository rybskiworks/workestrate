# Nix Guidance Index

## Purpose

This corpus is a set of repo-independent Nix guidance documents for future AI coding agents that write, review, refactor, debug, validate, and package Nix code. The 25 files cover the Nix expression language, the Nix store, derivations, flakes, the module system, NixOS, packaging, and operational concerns (purity, caching, CI/CD, secrets, supply-chain security). Each document derives its rules from official Nix/Nixpkgs/NixOS sources and records the policy decisions that individual repositories must still make for themselves.

The corpus is sourced from 73 crawl files persisted in `docs/nix/.crawl/` (extracted from a pinned clone of `github.com/nixos/nix.dev`, commit `139034be`). The provenance index is `docs/nix/source-map.md`, which maps every crawled source URL to the topic files that use it.

Nix is the foundational build tool for the entire ai-workbench project — it provides the dev shell, the Rust toolchain, the Microsandbox runtime, and all agent infrastructure. Every other language corpus (Rust, Elixir, Gleam) runs inside an environment that Nix builds.

## How to use this corpus

### Reading paths

Agents should not read all 25 files linearly. Consult the [Recommended reading paths](#recommended-reading-paths) section to find the 2–5 files most relevant to the current task, then read those files' relevant sections.

### When to consult which docs

- **New to Nix** → start with `overview.md`, then `language-fundamentals.md` and `nix-store-and-paths.md`.
- **Writing Nix code** → `language-fundamentals.md`, `conventions-and-style.md`, then the domain file (`flake-anatomy.md`, `devshells.md`, `modules-and-config.md`, etc.).
- **Reviewing a PR/diff** → `conventions-and-style.md`, `purity-and-sandboxing.md`, `validation.md`, plus the domain file matching the changed code.
- **Debugging a build failure** → `error-handling-and-debugging.md`, `derivations-and-builds.md`, `purity-and-sandboxing.md`.
- **Setting up a project** → `flake-anatomy.md`, `devshells.md`, `modules-and-config.md`.
- **Packaging software** → `derivations-and-builds.md`, `packaging-recipes.md`, `nixpkgs-library.md`.
- **Purity/hardening review** → `purity-and-sandboxing.md`, `supply-chain-security.md`, `store-hygiene-and-gc.md`.
- **Deploying** → `deployment-and-runtime.md`, `docker-images.md`, `nixos-configurations.md`.
- **Working with the store** → `nix-store-and-paths.md`, `store-hygiene-and-gc.md`, `caching-and-binary-caches.md`.

### Conventions used in every file

Each topic file follows a consistent structure:

1. **Purpose** — what the file covers and who it is for.
2. **Sources used** — the crawl files and canonical URLs consulted, with version notes.
3. **Related Nix guidance** — cross-references to sibling files in this corpus.
4. **Core guidance** — verbatim quotations from the official docs, with inline citations.
5. **Practical rules** — concrete decision rules and API contracts.
6. **Review/Implementation checklists** — copy-pasteable checklists.
7. **Common mistakes** — anti-patterns and traps.
8. **Policy decisions for individual repos** — open questions each repo must answer.

## Relationship to other corpora

Nix is infrastructure, not a language — it provides the build environment in which Rust, Elixir, and Gleam code compiles and runs. Cross-references:

- `docs/rust/index.md` — Rust language corpus; Nix builds the Rust toolchain and dev shell.
- `docs/elixir/index.md` — Elixir corpus; Nix provides the Elixir/Erlang toolchain.
- `docs/gleam/index.md` — Gleam corpus; Nix provides the Gleam toolchain.
- `docs/beam/` — BEAM runtime concepts (processes, supervision, etc.).
- `docs/litellm/` — LiteLLM proxy config; Nix deploys the LiteLLM proxy.

Nix is the outermost layer: it builds the environment in which all other language toolchains run. The language corpora (Rust, Elixir, Gleam) cover language semantics; the Nix corpus covers how to build, package, and deploy them.

## Generated files

### overview.md

- **Path:** `docs/nix/overview.md`
- **Purpose:** Nix mental model: what Nix is, purity, reproducibility, the store, derivations, flakes. Entry point for the corpus.
- **Main topics:**
  - What Nix is and why it exists
  - Purity and reproducibility as core principles
  - The Nix store and content addressing
  - Derivations as the build unit
  - Flakes as the project unit
  - How Nix compares to other build tools
- **When a future agent should read it:** First, as the conceptual entry point before any Nix work.
- **Related skill files:** `.agents/skills/nix-usage/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/implementation.md`, `docs/nix/workflows/code-review.md`

### language-fundamentals.md

- **Path:** `docs/nix/language-fundamentals.md`
- **Purpose:** The Nix expression language: types, attribute sets, let-in, with, rec, lambdas, operators, builtins, string interpolation, import, assert.
- **Main topics:**
  - Basic types (integers, floats, strings, paths, booleans, null, lists, attrsets)
  - Attribute sets: `let … in`, `with`, `rec`, nested sets, attribute selection
  - Functions: lambdas, pattern matching on arguments, partial application
  - Operators and builtins (`map`, `filter`, `foldl'`, `attrNames`, `attrValues`, etc.)
  - String interpolation `"${expr}"` and multi-line strings
  - `import`, `assert`, `builtins.trace` for debugging
- **When a future agent should read it:** Before writing or reviewing any Nix expression; the bedrock for all other topic docs.
- **Related skill files:** `.agents/skills/nix-language/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/implementation.md`, `docs/nix/workflows/code-review.md`

### nix-store-and-paths.md

- **Path:** `docs/nix/nix-store-and-paths.md`
- **Purpose:** The Nix store: store paths, closures, garbage collection, content addressing, store hygiene, and the 29 GB accumulation incident.
- **Main topics:**
  - Store path anatomy (`/nix/store/<hash>-<name>`)
  - Closures and runtime dependency graphs
  - Content addressing and input addressing
  - Garbage collection and GC roots
  - Store hygiene and the 29 GB incident
  - `nix-store` commands (`--query`, `--gc`, `--verify`)
- **When a future agent should read it:** When debugging store-related issues, understanding why a path exists, or auditing store growth.
- **Related skill files:** `.agents/skills/nix-store-gc/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/debugging.md`, `docs/nix/workflows/hardening.md`

### derivations-and-builds.md

- **Path:** `docs/nix/derivations-and-builds.md`
- **Purpose:** Derivations: the `derivation` builtin, `stdenv.mkDerivation`, build phases, hooks, source fetchers, multi-output, and fixed-output derivations (FODs).
- **Main topics:**
  - The `derivation` builtin and `.drv` files
  - `stdenv.mkDerivation` and its attribute interface
  - Build phases (`unpackPhase`, `patchPhase`, `configurePhase`, `buildPhase`, `checkPhase`, `installPhase`, `fixupPhase`, `installCheckPhase`)
  - Build hooks and setup hooks
  - Source fetchers (`fetchurl`, `fetchzip`, `fetchgit`, `fetchFromGitHub`, `fetchFromGitLab`, `fetchpatch`)
  - Multi-output derivations (`outputs`, `bin`, `dev`, `lib`, `doc`, `out`)
  - Fixed-output derivations (FODs) for network-dependent builds
- **When a future agent should read it:** When writing, reviewing, or debugging a derivation or build recipe.
- **Related skill files:** `.agents/skills/nix-derivations/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/implementation.md`, `docs/nix/workflows/packaging.md`, `docs/nix/workflows/debugging.md`

### flake-anatomy.md

- **Path:** `docs/nix/flake-anatomy.md`
- **Purpose:** flake.nix structure: inputs, outputs, flake.lock, registry, and flake commands.
- **Main topics:**
  - Flake schema: `description`, `inputs`, `outputs`, `nixConfig`
  - Input types (`github`, `gitlab`, `git`, `path`, `tarball`, `flake`) and follows
  - Output attributes (`packages`, `devShells`, `checks`, `overlays`, `nixosModules`, `nixosConfigurations`, `apps`, `formatter`)
  - `flake.lock` and input pinning
  - The flake registry and `nix registry`
  - Flake commands (`nix flake new`, `nix flake check`, `nix flake show`, `nix flake update`, `nix flake lock`)
- **When a future agent should read it:** When creating, modifying, or reviewing a `flake.nix` file.
- **Related skill files:** `.agents/skills/nix-flake-anatomy/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/implementation.md`, `docs/nix/workflows/code-review.md`

### devshells.md

- **Path:** `docs/nix/devshells.md`
- **Purpose:** Dev shells: ad-hoc shells, declarative `shell.nix`, `mkShell`, `shellHook`, `nix develop`, and direnv integration.
- **Main topics:**
  - Ad-hoc shells (`nix shell nixpkgs#hello`)
  - Declarative `shell.nix` and `default.nix`
  - `mkShell` and `packages` vs `buildInputs`
  - `shellHook` for environment setup
  - `nix develop` for flake-based shells
  - direnv integration (`use flake`, `use nix`)
- **When a future agent should read it:** When setting up a development environment or debugging shell issues.
- **Related skill files:** `.agents/skills/nix-devshells/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/implementation.md`

### modules-and-config.md

- **Path:** `docs/nix/modules-and-config.md`
- **Purpose:** The Nix module system: `mkOption`, types, config merging, `submodule`, and `evalModules`.
- **Main topics:**
  - Module structure (`options` and `config` attributes)
  - `mkOption` and option declarations
  - Type system (`types.str`, `types.int`, `types.bool`, `types.enum`, `types.attrsOf`, `types.submodule`, etc.)
  - Config merging and priority
  - `submodule` and nested options
  - `evalModules` and module evaluation
- **When a future agent should read it:** When writing or reviewing NixOS modules, Home Manager modules, or flake modules.
- **Related skill files:** `.agents/skills/nix-modules/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/implementation.md`, `docs/nix/workflows/code-review.md`

### nixpkgs-library.md

- **Path:** `docs/nix/nixpkgs-library.md`
- **Purpose:** The nixpkgs library (`lib`): attrsets, lists, strings, trivial, sources, modules, types, customisation, debug, and path utilities.
- **Main topics:**
  - `lib.attrsets` (`attrNames`, `attrValues`, `mapAttrs`, `filterAttrs`, `zipAttrs`, etc.)
  - `lib.lists` (`map`, `filter`, `foldl'`, `flatten`, `unique`, `sort`, etc.)
  - `lib.strings` (`concatStrings`, `splitString`, `toUpper`, `toLower`, `escapeShellArg`, etc.)
  - `lib.trivial` (`id`, `const`, `compose`, `flip`, `pipe`, etc.)
  - `lib.sources` (`sourceFilesBySuffices`, `cleanSource`, `cleanSourceWith`)
  - `lib.modules` and `lib.types` for module authors
  - `lib.customisation` (`override`, `overrideAttrs`, `makeOverridable`)
  - `lib.debug` (`traceVal`, `traceSeq`, `traceIf`)
  - `lib.path` (`append`, `splitRoot`, `subpath`)
- **When a future agent should read it:** When writing or reviewing Nix code that manipulates data structures or uses nixpkgs helpers.
- **Related skill files:** `.agents/skills/nix-nixpkgs-library/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/implementation.md`

### overlays.md

- **Path:** `docs/nix/overlays.md`
- **Purpose:** Overlays: the `final:prev` signature, composition, scoping, and override/extend patterns.
- **Main topics:**
  - The overlay signature (`final: prev: { … }`)
  - `final` vs `prev` and when to use each
  - Composition with `composeExtensions` and `lib.composeManyExtensions`
  - Scoping (`self`, `super`, `callPackage`)
  - `override` and `overrideAttrs` patterns
  - Extending `pkgs` with new packages
- **When a future agent should read it:** When customizing nixpkgs, adding packages, or modifying existing packages.
- **Related skill files:** `.agents/skills/nix-overlays/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/implementation.md`, `docs/nix/workflows/code-review.md`

### packaging-recipes.md

- **Path:** `docs/nix/packaging-recipes.md`
- **Purpose:** Language-specific packaging: `buildNpmPackage`, `buildPythonApplication`, `buildGoModule`, bun, `callPackage`, and override/overrideAttrs patterns.
- **Main topics:**
  - `callPackage` and package composition
  - `buildNpmPackage` and `npmDepsHash`
  - `buildPythonApplication` and `propagatedBuildInputs`
  - `buildGoModule` and `vendorHash`
  - Bun packaging with `stdenv.mkDerivation`
  - `override` and `overrideAttrs` for customizing existing packages
  - `overrideScope` and `overrideScope'` for scoped overrides
- **When a future agent should read it:** When packaging a new piece of software or modifying an existing package.
- **Related skill files:** `.agents/skills/nix-packaging-recipes/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/packaging.md`

### nixos-configurations.md

- **Path:** `docs/nix/nixos-configurations.md`
- **Purpose:** NixOS: `nixosSystem`, `configuration.nix`, `nixos-rebuild`, generations, `nixos-install`, containers, VMs, and `nixosTest`.
- **Main topics:**
  - `nixosSystem` and system configuration
  - `configuration.nix` and hardware configuration
  - `nixos-rebuild` (`switch`, `boot`, `test`, `build`, `dry-build`)
  - Generations and rollback
  - `nixos-install` for bare-metal installation
  - NixOS containers and VMs
  - `nixosTest` for system-level testing
- **When a future agent should read it:** When configuring NixOS systems, debugging boot issues, or writing NixOS tests.
- **Related skill files:** None (to be generated)
- **Related workflow files:** `docs/nix/workflows/implementation.md`, `docs/nix/workflows/debugging.md`

### conventions-and-style.md

- **Path:** `docs/nix/conventions-and-style.md`
- **Purpose:** Style: naming, scoping, formatting, nixpkgs patterns, and anti-patterns.
- **Main topics:**
  - Naming conventions (camelCase for variables, kebab-case for files)
  - Scoping discipline (`let` over `with`, explicit over implicit)
  - Formatting with `nixfmt`, `nixpkgs-fmt`, or `alejandra`
  - nixpkgs coding patterns and idioms
  - Anti-patterns (overuse of `with`, `rec`, deep nesting, etc.)
- **When a future agent should read it:** When reviewing Nix code or setting up formatting/linting.
- **Related skill files:** `.agents/skills/constraint-nix-scope-discipline/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/code-review.md`, `docs/nix/workflows/implementation.md`

### nix-commands.md

- **Path:** `docs/nix/nix-commands.md`
- **Purpose:** CLI: `nix build`, `develop`, `run`, `flake`, `profile`, `store`, `eval`, `fmt`.
- **Main topics:**
  - `nix build` and build outputs
  - `nix develop` and dev shells
  - `nix run` and app execution
  - `nix flake` subcommands
  - `nix profile` for user environments
  - `nix store` for store manipulation
  - `nix eval` for expression evaluation
  - `nix fmt` for formatting
- **When a future agent should read it:** When using the Nix CLI or debugging command failures.
- **Related skill files:** `.agents/skills/nix-usage/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/implementation.md`, `docs/nix/workflows/debugging.md`

### purity-and-sandboxing.md

- **Path:** `docs/nix/purity-and-sandboxing.md`
- **Purpose:** Purity: eval-time and build-time purity, sandboxing, FODs, impurity patterns, and enforcement.
- **Main topics:**
  - Eval-time purity (no network, no filesystem access, no time)
  - Build-time purity and sandboxing
  - Fixed-output derivations (FODs) for network-dependent builds
  - Impurity patterns and their risks
  - Sandbox enforcement levels and `__impure` escape hatch
  - Store path leakage and how to prevent it
- **When a future agent should read it:** When reviewing for purity violations, hardening builds, or debugging sandbox failures.
- **Related skill files:** `.agents/skills/constraint-nix-purity/SKILL.md`, `.agents/skills/constraint-nix-sandbox-safety/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/hardening.md`, `docs/nix/workflows/code-review.md`

### validation.md

- **Path:** `docs/nix/validation.md`
- **Purpose:** Validation: `nix flake check`, build, eval, `flake show`, the `just verify` pipeline, `lint-nix`, and store-audit.
- **Main topics:**
  - `nix flake check` as the primary validation gate
  - `nix build` for build validation
  - `nix eval` for expression validation
  - `nix flake show` for output inspection
  - The `just verify` pipeline and its stages
  - `lint-nix` and `store-audit` for lint and store checks
- **When a future agent should read it:** When setting up CI gates or running validation before merge.
- **Related skill files:** `.agents/skills/validation-nix-build/SKILL.md`, `.agents/skills/validation-nix-eval/SKILL.md`, `.agents/skills/validation-nix-flake-check/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/validation.md`

### error-handling-and-debugging.md

- **Path:** `docs/nix/error-handling-and-debugging.md`
- **Purpose:** Debugging: eval errors, build failures, log inspection, `nix repl`, and `builtins.trace`.
- **Main topics:**
  - Eval errors and how to read them
  - Build failures and log inspection (`nix log`, `--keep-going`, `-L`)
  - `nix repl` for interactive exploration
  - `builtins.trace` for debugging expressions
  - `--show-trace` for stack traces
  - Debugging sandbox failures and network issues
- **When a future agent should read it:** When debugging a Nix build failure or eval error.
- **Related skill files:** None (to be generated)
- **Related workflow files:** `docs/nix/workflows/debugging.md`

### testing.md

- **Path:** `docs/nix/testing.md`
- **Purpose:** Testing: `nix flake check`, `checks` output, `checkPhase`/`doCheck`, `nixosTests`, `testers`, and `runCommand`.
- **Main topics:**
  - `nix flake check` and the `checks` output
  - `checkPhase` and `doCheck` in derivations
  - `nixosTest` for system-level testing
  - `testers` (`testVersion`, `testEqualContents`, `runNixOSTest`)
  - `runCommand` for lightweight tests
  - Testing strategies for different artifact types
- **When a future agent should read it:** When writing or reviewing tests for Nix code.
- **Related skill files:** `.agents/skills/nix-testing/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/validation.md`, `docs/nix/workflows/implementation.md`

### supply-chain-security.md

- **Path:** `docs/nix/supply-chain-security.md`
- **Purpose:** Supply chain: lock auditing, reproducible builds, CVE scanning, FOD integrity, and the trust model.
- **Main topics:**
  - Lock file auditing (`flake.lock` review)
  - Reproducible builds and `nix build --check`
  - CVE scanning with `nix audit` or external tools
  - FOD hash verification and integrity
  - The Nix trust model (substituters, signing keys, trusted users)
  - Supply-chain hardening for CI
- **When a future agent should read it:** When reviewing for supply-chain risks or setting up security gates.
- **Related skill files:** `.agents/skills/validation-nix-supply-chain/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/hardening.md`

### store-hygiene-and-gc.md

- **Path:** `docs/nix/store-hygiene-and-gc.md`
- **Purpose:** Store hygiene: garbage collection, GC roots, accumulation patterns, auditing, and 29 GB incident remediation.
- **Main topics:**
  - Garbage collection mechanics (`nix store gc`, `nix-collect-garbage`)
  - GC roots and how they prevent collection
  - Accumulation patterns (devshells, profiles, result links)
  - Store auditing and size analysis
  - The 29 GB incident and its remediation
  - GC policies for dev vs CI vs production
- **When a future agent should read it:** When the store is growing unexpectedly or when setting up GC policies.
- **Related skill files:** `.agents/skills/nix-store-gc/SKILL.md`, `.agents/skills/constraint-nix-store-hygiene/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/hardening.md`, `docs/nix/workflows/debugging.md`

### caching-and-binary-caches.md

- **Path:** `docs/nix/caching-and-binary-caches.md`
- **Purpose:** Caching: substituters, `trusted-public-keys`, Cachix, self-hosted caches, `nix copy`, eval caching, and CI caching.
- **Main topics:**
  - Substituters and `substituters` option
  - `trusted-public-keys` and cache verification
  - Cachix for hosted binary caching
  - Self-hosted binary caches (`nix-serve`, `attic`, `harmonia`)
  - `nix copy` and `nix copy --to` for pushing to caches
  - Eval caching and its invalidation
  - CI caching strategies (GitHub Actions, Cachix)
- **When a future agent should read it:** When setting up or debugging binary caches, or optimizing CI build times.
- **Related skill files:** None (to be generated)
- **Related workflow files:** `docs/nix/workflows/hardening.md`

### ci-cd-integration.md

- **Path:** `docs/nix/ci-cd-integration.md`
- **Purpose:** CI/CD: GitHub Actions, `cachix/install-nix-action`, `nix flake check` as a CI gate, and Cachix integration.
- **Main topics:**
  - GitHub Actions workflows for Nix
  - `cachix/install-nix-action` for Nix installation
  - `nix flake check` as a CI gate
  - Cachix integration for build caching
  - Build matrix strategies (systems, configurations)
  - Deployment triggers and release automation
- **When a future agent should read it:** When setting up CI/CD for a Nix project.
- **Related skill files:** `.agents/skills/nix-ci-cd/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/validation.md`, `docs/nix/workflows/hardening.md`

### cross-compilation.md

- **Path:** `docs/nix/cross-compilation.md`
- **Purpose:** Cross-compilation: build/host/target, `pkgsCross`, `pkgsStatic`, qemu-user, and multi-platform flakes.
- **Main topics:**
  - Build/host/target platform triplets
  - `pkgsCross` for cross-compilation
  - `pkgsStatic` for static linking
  - qemu-user for running foreign binaries
  - Multi-platform flake outputs
  - Cross-compilation testing requirements
- **When a future agent should read it:** When building for a different architecture or setting up multi-platform CI.
- **Related skill files:** `.agents/skills/nix-cross-compilation/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/packaging.md`

### deployment-and-runtime.md

- **Path:** `docs/nix/deployment-and-runtime.md`
- **Purpose:** Deployment: `nixos-rebuild`, generations, `nixos-anywhere`, Terraform, containers, and Docker images.
- **Main topics:**
  - `nixos-rebuild` and generation management
  - `nixos-anywhere` for remote installation
  - Terraform integration with Nix
  - NixOS containers and OCI containers
  - Docker images and container deployment
  - Rollback and disaster recovery
- **When a future agent should read it:** When deploying NixOS systems or Nix-built artifacts.
- **Related skill files:** None (to be generated)
- **Related workflow files:** `docs/nix/workflows/implementation.md`

### docker-images.md

- **Path:** `docs/nix/docker-images.md`
- **Purpose:** Docker: `dockerTools`, `buildImage`, `buildLayeredImage`, `streamLayeredImage`, `pullImage`, and layer strategy.
- **Main topics:**
  - `dockerTools.buildImage` for simple images
  - `dockerTools.buildLayeredImage` for optimized layer caching
  - `dockerTools.streamLayeredImage` for streaming builds
  - `dockerTools.pullImage` for base images
  - Layer strategy and cache optimization
  - Image naming, tagging, and registry push
- **When a future agent should read it:** When building Docker/OCI images with Nix.
- **Related skill files:** `.agents/skills/nix-docker-images/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/packaging.md`

### secrets-and-sops.md

- **Path:** `docs/nix/secrets-and-sops.md`
- **Purpose:** Secrets: why the store leaks secrets, SOPS with age, `sops-nix`, agenix, and the project secret workflow.
- **Main topics:**
  - Why the Nix store leaks secrets (world-readable store paths)
  - SOPS with age for secret encryption
  - `sops-nix` for NixOS secret management
  - agenix for age-encrypted secrets
  - The project secret workflow (encrypt, commit, decrypt at runtime)
  - Secret rotation and audit requirements
- **When a future agent should read it:** When handling secrets in Nix code or NixOS configurations.
- **Related skill files:** `.agents/skills/constraint-nix-secret-hygiene/SKILL.md`
- **Related workflow files:** `docs/nix/workflows/hardening.md`

> **Note:** `source-map.md` is the provenance index for the entire corpus. It maps all 73 crawled source URLs to the topic files that use them. It is a meta-doc, not a topic doc, and is maintained separately.

## Recommended reading paths

### New to Nix / writing first Nix code

1. `overview.md` — mental model, purity, store, derivations, flakes.
2. `language-fundamentals.md` — expression language syntax and builtins.
3. `nix-store-and-paths.md` — store paths, closures, GC.
4. `derivations-and-builds.md` — `mkDerivation`, phases, fetchers.
5. `conventions-and-style.md` — naming, scoping, formatting.

### Reviewing a Nix PR

1. `conventions-and-style.md` — style bar and anti-patterns.
2. `purity-and-sandboxing.md` — purity violations and sandbox issues.
3. `validation.md` — gates to run before merge.
4. Domain file matching the change (e.g., `flake-anatomy.md` for flake changes, `devshells.md` for shell changes).

### Debugging a Nix build failure

1. `error-handling-and-debugging.md` — eval errors, log inspection, `nix repl`.
2. `derivations-and-builds.md` — build phases and hooks.
3. `purity-and-sandboxing.md` — sandbox failures and network issues.
4. `nix-commands.md` — CLI flags for debugging (`-L`, `--keep-going`, `--show-trace`).

### Setting up a Nix project

1. `flake-anatomy.md` — `flake.nix` structure, inputs, outputs.
2. `devshells.md` — `mkShell`, `shellHook`, direnv.
3. `modules-and-config.md` — module system for configuration.
4. `nixpkgs-library.md` — `lib` helpers for data manipulation.
5. `conventions-and-style.md` — formatting and naming.

### Packaging software with Nix

1. `derivations-and-builds.md` — `mkDerivation`, phases, FODs.
2. `packaging-recipes.md` — language-specific builders and patterns.
3. `nixpkgs-library.md` — `lib` helpers and source utilities.
4. `overlays.md` — customizing and extending nixpkgs.
5. `purity-and-sandboxing.md` — FOD policy and impurity avoidance.

### Hardening and purity review

1. `purity-and-sandboxing.md` — purity rules and sandbox enforcement.
2. `supply-chain-security.md` — lock auditing, CVE scanning, trust model.
3. `store-hygiene-and-gc.md` — store accumulation and GC policies.
4. `secrets-and-sops.md` — secret handling and store leakage.
5. `validation.md` — gates to enforce in CI.

### Deploying with Nix

1. `deployment-and-runtime.md` — `nixos-rebuild`, generations, containers.
2. `docker-images.md` — `dockerTools`, layer strategy, registry.
3. `nixos-configurations.md` — `nixosSystem`, `configuration.nix`, VMs.
4. `caching-and-binary-caches.md` — substituters, Cachix, CI caching.
5. `ci-cd-integration.md` — GitHub Actions, build matrix, deployment triggers.

### Working with the Nix store

1. `nix-store-and-paths.md` — store paths, closures, content addressing.
2. `store-hygiene-and-gc.md` — GC roots, accumulation patterns, auditing.
3. `caching-and-binary-caches.md` — substituters and cache management.
4. `purity-and-sandboxing.md` — store path leakage prevention.

## Skill derivation map

The following skills have been generated under `.agents/skills/` from the corpus. Each skill encodes the actionable, repeatable procedure derived from its source doc(s).

### Operational skills (14)

| Skill | Source docs |
|---|---|
| `nix-usage` | `overview.md`, `nix-commands.md` |
| `nix-language` | `language-fundamentals.md` |
| `nix-flake-anatomy` | `flake-anatomy.md` |
| `nix-derivations` | `derivations-and-builds.md` |
| `nix-devshells` | `devshells.md` |
| `nix-modules` | `modules-and-config.md` |
| `nix-nixpkgs-library` | `nixpkgs-library.md` |
| `nix-overlays` | `overlays.md` |
| `nix-packaging-recipes` | `packaging-recipes.md` |
| `nix-store-gc` | `nix-store-and-paths.md`, `store-hygiene-and-gc.md` |
| `nix-testing` | `testing.md` |
| `nix-ci-cd` | `ci-cd-integration.md` |
| `nix-cross-compilation` | `cross-compilation.md` |
| `nix-docker-images` | `docker-images.md` |

### Constraint skills (6)

| Skill | Source docs |
|---|---|
| `constraint-nix-purity` | `purity-and-sandboxing.md` |
| `constraint-nix-reproducibility` | `purity-and-sandboxing.md`, `supply-chain-security.md` |
| `constraint-nix-sandbox-safety` | `purity-and-sandboxing.md` |
| `constraint-nix-scope-discipline` | `conventions-and-style.md` |
| `constraint-nix-secret-hygiene` | `secrets-and-sops.md` |
| `constraint-nix-store-hygiene` | `store-hygiene-and-gc.md` |

### Validation skills (7)

| Skill | Source docs |
|---|---|
| `validation-nix-build` | `validation.md` |
| `validation-nix-eval` | `validation.md` |
| `validation-nix-flake-check` | `validation.md` |
| `validation-nix-format` | `conventions-and-style.md` |
| `validation-nix-lint` | `conventions-and-style.md`, `purity-and-sandboxing.md` |
| `validation-nix-supply-chain` | `supply-chain-security.md` |
| `validation-nix-test` | `testing.md` |

## Workflow map

Workflow skills exist under `.agents/skills/workflow-nix-*` and reference `docs/nix/workflows/` for the underlying step-by-step procedures.

| Workflow | Orchestration | Phases | Purpose |
|---|---|---|---|
| Implementation | `workflow-nix-implementation-00-orchestration` | 01-scope → 02-design → 03-implement → 04-test → 05-verify | New Nix code |
| Code review | `workflow-nix-code-review-00-orchestration` | 01-scope → 02-analyze → 03-check → 04-review → 05-verdict | PR/diff review |
| Debugging | `workflow-nix-debugging-00-orchestration` | 01-reproduce → 02-diagnose → 03-fix → 04-regression → 05-verify | Fix build failures |
| Refactoring | `workflow-nix-refactoring-00-orchestration` | 01-baseline → 02-plan → 03-execute → 04-verify → 05-confirm | Restructure without behavior change |
| Validation | `workflow-nix-validation-00-orchestration` | 01-scope → 02-compile → 03-lint-test → 04-format-doc → 05-report | Run all gates |
| Hardening | `workflow-nix-hardening-00-orchestration` | 01-scope → 02-plan → 03-implement → 04-validate → 05-verify | Production hardening pass |
| Packaging | `workflow-nix-packaging-00-orchestration` | 01-scope → 02-design → 03-implement → 04-test → 05-verify | Package new software |

## Open policy decisions

Each topic file ends with a "Policy decisions for individual repos" section listing the questions that are deliberately left unanswered because they depend on the consuming repository's conventions, CI setup, and risk tolerance. Below is a consolidated summary of the key decision points that appear across the corpus. Each consuming repo should record its answers in a project-level `CONTRIBUTING.md`.

### Flake and project structure

- Flake output naming conventions (`packages`, `devShells`, `checks`, `overlays`).
- Whether to use `flake-parts` or raw `flake.nix`.
- Input pinning policy (exact `rev` vs `branch`).
- `flake.lock` update cadence and CI enforcement.

### Purity and sandboxing

- Sandbox enforcement level (strict vs relaxed).
- FOD policy for network-dependent builds.
- Impurity escape policy (when `__impure` is acceptable).
- Store path leakage policy in CI.

### Formatting and style

- Formatter choice (`nixfmt`, `nixpkgs-fmt`, `alejandra`).
- Format-on-save policy.
- Max line width and indentation style.
- `rec` vs `let` vs `with` scoping preference.

### Lint and validation

- `statix`/`deadnix` enforcement level.
- Whether lint failures block CI.
- Custom lint rule additions.
- Eval cache invalidation policy.

### Store hygiene and GC

- GC frequency and retention policy.
- GC root pinning policy for devshells.
- Store size alerting threshold.
- Automatic vs manual GC triggering.

### Caching and binary caches

- Trusted substituters list.
- Self-hosted vs Cachix vs `cache.nixos.org`.
- Cache signing key management.
- CI cache upload policy.

### Secrets management

- Secret backend (SOPS+age vs agenix vs manual).
- Key rotation policy.
- Secret access audit requirements.
- Development vs production secret separation.

### Supply chain security

- CVE scanning cadence and tooling.
- Dependency review requirements for new packages.
- FOD hash verification policy.
- Reproducibility verification policy.

### CI/CD

- CI runner Nix installation method.
- Which flake outputs are CI gates.
- Build matrix strategy (systems, configurations).
- Deployment trigger policy.

### Cross-compilation

- Supported target platforms.
- Static vs dynamic linking default.
- Cross-compilation testing requirements.
- Emulator availability policy.

### Docker/OCI images

- Image naming and tagging convention.
- Layer strategy (`buildImage` vs `buildLayeredImage` vs `streamLayeredImage`).
- Image registry and push policy.
- Base image policy.

### NixOS

- Configuration organization (single file vs modules).
- `nixos-rebuild` target policy (`switch` vs `boot` vs `test`).
- Generation retention policy.
- Remote deployment tool choice.
