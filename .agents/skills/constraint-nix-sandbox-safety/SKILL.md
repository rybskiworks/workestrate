---
name: constraint-nix-sandbox-safety
description: |
  Enforces Nix build-sandbox safety invariants during code execution — sandbox
  enabled, no network in buildPhase/installPhase, no __noChroot without
  justification, all build dependencies declared, HOME set to TMPDIR, and no
  system file dependencies. Load when writing or reviewing derivations with
  build phases or configuring the Nix sandbox. Does NOT cover secret handling
  (see constraint-nix-secret-hygiene) or Nix expression scoping (see
  constraint-nix-scope-discipline).
metadata:
  org.kind: constraint
---

# Constraint: Nix Sandbox Safety

Nix build-time purity requires the build sandbox enabled (`sandbox = true` in
`nix.conf`). The sandbox has no network in `buildPhase`/`installPhase`; all
dependencies must arrive via fixed-output derivations (FODs) with declared
`outputHash`es. Violations break reproducibility, introduce undeclared system
dependencies, or escape the sandbox unsafely — producing builds that pass on
one host and fail (or silently differ) on another. A build that depends on
host state is not a build that can be trusted to reproduce, and a sandbox
escape is often the only visible symptom of an undeclared dependency that
will break under CI or on a fresh machine.

## Triggers

Load this skill when:

- Writing or reviewing derivations with `buildPhase`/`installPhase`.
- Configuring `nix.conf` sandbox settings (`sandbox`, `sandboxFallback`).
- Reviewing a derivation that sets `__noChroot` or uses `--option sandbox-build false`.
- Adding a fixed-output derivation (FOD) or a fetcher.

## Rules

1. Build sandbox must be enabled (`sandbox = true` in `nix.conf`) where the
   platform supports it.
   - On macOS without a Nix daemon, or platforms lacking user namespaces,
     document the exception.
2. No network access in `buildPhase`/`installPhase` — data must be
   committed or fetched via a FOD before the build runs.
   - `curl`, `wget`, `git clone`, and any network call belong in a FOD fetch
     phase, never in the build.
3. No `__noChroot = true;` without a justification comment explaining why the
   derivation cannot run sandboxed.
   - The comment must name the concrete resource the derivation needs outside
     the sandbox.
4. No `--option sandbox-build false` or `--impure` in scripts, justfile, or
   nix code.
   - These flags silently disable purity and are almost always a workaround
     for an undeclared dependency.
5. All build dependencies must be declared in `buildInputs`/
   `nativeBuildInputs` — no implicit system dependencies (no `/usr/bin`,
   `/etc/passwd`, `/etc/hosts`).
   - If a tool is used in a build phase, it must be in `nativeBuildInputs`;
     the sandbox will not provide host tools.
6. `HOME` must be set to `$TMPDIR` (or `$NIX_BUILD_TOP`) in derivations that
   write to `HOME`; never rely on the host `HOME`.
   - Tools that cache to `~/.cache` or write dotfiles will fail or pollute
     the host `HOME` otherwise.
7. No `/etc/passwd`, `/etc/hosts`, or other system file dependencies — the
   sandbox does not mount host `/etc` (except where `__noChroot` is
   justified).
   - Use `setupHook`s or `passthru` data rather than reading host
     configuration files.

## References

- Docs: `docs/nix/purity-and-sandboxing.md`, `docs/nix/derivations-and-builds.md`.
- Sibling skill: `nix-usage` (anti-accumulation patterns, FOD usage).

## Out of scope

- Secret handling in Nix expressions — see `constraint-nix-secret-hygiene`.
- Nix expression scoping (`with`, `rec`, `<nixpkgs>`) — see
  `constraint-nix-scope-discipline`.

## Violation examples

### `__noChroot` without justification

```nix
# FORBIDDEN: disables the sandbox with no documented reason
stdenv.mkDerivation {
  pname = "my-service";
  version = "0.1.0";
  __noChroot = true;
  # ...
}
```

Correct: `__noChroot` is almost never appropriate. If a derivation genuinely
cannot run sandboxed (e.g. a FOD needing `/etc`), add a comment:
`# __noChroot: required because <reason>` and prefer restructuring to avoid
it. Most "needs `__noChroot`" cases are actually undeclared dependencies in
disguise.

### Network access in buildPhase

```nix
# FORBIDDEN: network is unavailable in the sandbox
buildPhase = ''
  curl -o data.json https://example.com/data.json
  make build
'';
```

Correct: fetch the data via a FOD (`fetchurl`/`fetchFromGitHub` with
`outputHash`) before the build, or commit the data to the repo. FODs permit
network in the fetch phase because the output is verified by hash, so a
tampered or changed fetch fails loudly instead of producing a different
build.

### Undeclared system dependency

```nix
# FORBIDDEN: relies on host /usr/bin/python
buildPhase = ''
  /usr/bin/python3 scripts/generate.py
'';
```

Correct: add `python3` to `nativeBuildInputs` and call
`python3 scripts/generate.py` — the sandbox provides the declared
dependency, not the host system. Hardcoded absolute paths like `/usr/bin/...`
are a clear signal that a dependency was never declared.

### `--impure` in a build invocation

```bash
# FORBIDDEN: bypasses purity, copies the working tree each run
nix build .#my-service --impure
```

Correct: never use `--impure`. Fix the derivation's purity (use
`builtins.path` with a `filter`, FODs for fetching) instead of escaping the
sandbox. `--impure` makes the build depend on the live working tree, so two
runs seconds apart can produce different outputs.

## How to check

```bash
nix build .#<name> --sandbox-paths                # HOST-GATE: verify sandboxed build
just lint-nix                                      # static purity guard (catches --impure, getFlake)
grep -rn "__noChroot" nix/ flake.nix               # each must have a justification comment
nix show-derivation .#<name> | grep -i noChroot   # inspect derivation attrs
```

A clean sandboxed build succeeds without `--impure`; `just lint-nix` reports
no `--impure`/`getFlake` findings; and every `__noChroot` hit is preceded by
a justification comment. A build that only succeeds with `--impure` or
`--option sandbox-build false` is a blocking failure — the derivation has an
undeclared dependency that must be fixed, not bypassed.

Manual review:

- `sandbox = true` in `nix.conf` (where platform supports it).
- No `curl`/`wget`/network calls in `buildPhase`/`installPhase`.
- Every `__noChroot = true;` has a justification comment.
- No `--impure` or `--option sandbox-build false` in scripts or justfile.
- All build deps in `buildInputs`/`nativeBuildInputs`; no `/usr/bin` or
  `/etc` paths.
- Derivations writing to `HOME` set `HOME = "$TMPDIR"`.
- FODs declare `outputHash` (and `outputHashAlgo`); non-FOD builds never
  touch the network.
