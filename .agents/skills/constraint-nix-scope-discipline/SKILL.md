---
name: constraint-nix-scope-discipline
description: |
  Enforces Nix expression scope discipline during code execution — no `with`
  in large scopes, no `rec` when `let` suffices, no `<nixpkgs>` channel
  references in flake code, explicit `lib.` prefixes, narrow `let` bindings,
  and system-scoped flake outputs. Load when writing or reviewing Nix
  expressions, managing imports, or structuring flakes. Does NOT cover secret
  handling (see constraint-nix-secret-hygiene) or build sandbox purity (see
  constraint-nix-sandbox-safety).
metadata:
  org.kind: constraint
---

# Constraint: Nix Scope Discipline

Nix scope discipline keeps expressions statically analyzable, reproducible,
and free of impure channel references. `with` defeats static analysis (the
analyzer must evaluate to know what is in scope); `rec` risks infinite
recursion on shadowing; `<nixpkgs>` reads impure `$NIX_PATH`. Violations
produce non-reproducible builds, hard-to-debug recursion errors, and
ambiguous name provenance — defects that surface far from the offending line.
A reader should be able to determine where any name comes from without
evaluating the expression, and the same flake should resolve to the same
inputs on every host.

## Triggers

Load this skill when:

- Writing or reviewing Nix expressions (`flake.nix`, `nix/**/*.nix`, `default.nix`).
- Managing imports, `let` bindings, or attribute set structure.
- Structuring flake outputs (`perSystem`, `eachDefaultSystem`).
- Reviewing a PR for `with`/`rec`/`<nixpkgs>` usage.

## Rules

1. No `with` in large scopes — use explicit `lib.` prefix or
   `let inherit (attrs) x; in`.
   - `with` injects an entire attrset into scope, hiding where each name
     originates.
2. No `rec` when explicit `let` bindings suffice — `rec` risks
   `infinite recursion` on name shadowing.
   - `rec` makes every attribute lazily self-referential, so a typo can
     produce an infinite loop instead of an undefined-name error.
3. No `<nixpkgs>` channel references in flake code — `<nixpkgs>` reads
   impure `$NIX_PATH`; use flake inputs.
   - `$NIX_PATH` varies per machine, so the same flake evaluates to
     different inputs on different hosts.
4. No `import <nixpkgs>` — use flake inputs (or pass `config = {}; overlays =
   [];` if importing directly outside flakes).
   - Importing a channel pulls in whatever version happens to be pinned
     locally, breaking reproducibility.
5. Attribute sets should be explicitly scoped — prefer `let` bindings over
   `with` for name provenance.
   - Explicit bindings make refactors safe: a reader can grep for the
     binding site.
6. `let` bindings should be at the narrowest scope that uses them — do not
   hoist bindings to file top if used in one branch.
   - Narrow scopes keep unrelated branches from accidentally depending on a
     binding they should not see.
7. No global mutable state — no `builtins.readFile` of mutable files (e.g.
   `builtins.readFile ./version.txt` that changes between evals); use flake
   inputs or committed, filtered sources.
   - A mutable file read at eval time makes the evaluation result depend on
     filesystem state outside the flake.
8. Flake outputs should be system-scoped (`perSystem` or `eachDefaultSystem`)
   — never produce system-agnostic outputs that embed system-specific paths.
   - System-specific paths in a system-agnostic output produce a build that
     only works on one platform while claiming to be portable.

## References

- Docs: `docs/nix/conventions-and-style.md`, `docs/nix/language-fundamentals.md`.
- Sibling skill: `nix-usage` (flake structure, anti-accumulation patterns).

## Out of scope

- Secret handling in Nix expressions — see `constraint-nix-secret-hygiene`.
- Build sandbox purity and FODs — see `constraint-nix-sandbox-safety`.

## Violation examples

### `with lib;` at the top of a file

```nix
# FORBIDDEN: static analysis can't reason about scope; name provenance unclear
with lib;

{
  options.services.foo.enable = mkEnableOption "foo";
  config = mkIf config.services.foo.enable { ... };
}
```

Correct: use explicit `lib.` prefix or
`let inherit (lib) mkEnableOption mkIf; in`. Explicit prefixes let a reader
(and tooling) jump to the definition of `mkEnableOption` without evaluating
the whole module.

### `rec` with shadowing risk

```nix
# FORBIDDEN: infinite recursion — the inner `a` refers to itself
let a = 1; in rec { a = a; }
```

Correct: use `let` bindings — `let a = 1; in { a = a; }`. Use `rec` only
when the attrset must self-reference and the result must be an attrset
(e.g. `mkDerivation rec { pname = ...; src = ...${pname}...; }`). Even then,
prefer binding the shared value in a `let` outside the `rec` when possible.

### `<nixpkgs>` in flake code

```nix
# FORBIDDEN: impure — reads $NIX_PATH, non-reproducible
{
  outputs = { self, ... }@inputs:
  let pkgs = import <nixpkgs> {}; in {
    packages.x86_64-linux.default = pkgs.hello;
  };
}
```

Correct: use the flake input —
`let pkgs = inputs.nixpkgs.legacyPackages.${system}; in ...`. Flake inputs
are pinned in `flake.lock`, so every host resolves the same nixpkgs
revision.

### `builtins.readFile` of a mutable file

```nix
# FORBIDDEN: reads a mutable file at eval time — non-reproducible
version = builtins.readFile ./VERSION;
```

Correct: pin the version in `flake.nix` or read it from a committed,
filtered source via `builtins.path { name = ...; filter = ...; }`. A filtered
`builtins.path` includes only the files you name, so the eval result depends
only on committed content.

## How to check

```bash
nix flake check                          # validates flake outputs
just lint-nix                            # static purity guard (catches <nixpkgs>, --impure)
grep -rn "with lib;\|with pkgs;" nix/ flake.nix   # review each for scope
grep -rn "<nixpkgs>" nix/ flake.nix      # must be absent in flake code
grep -rn "rec {" nix/ flake.nix          # review each for shadowing risk
```

A clean run has `nix flake check` exit 0, no `<nixpkgs>` hits in flake code,
and every `with`/`rec` hit justified by a comment or scoped to a small
expression. Any `<nixpkgs>` or `import <nixpkgs>` hit in flake code is a
blocking failure — the expression is impure and will not reproduce across
hosts.

Manual review:

- No `with` at the top of a file; `with` (if any) scoped to a small
  expression.
- `rec` used only where the attrset must self-reference.
- No `<nixpkgs>` or `import <nixpkgs>` in flake code.
- `let` bindings at the narrowest scope.
- No `builtins.readFile` of mutable files.
- Flake outputs system-scoped (`perSystem`/`eachDefaultSystem`).
- Every name's provenance is greppable — no attrset injected wholesale into
  scope.
