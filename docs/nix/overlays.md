---
type: Reference
resource: https://nixos.org/manual/nixpkgs/stable/#sec-overlays
title: Overlays
description: Reference for nixpkgs overlays — the final:prev overlay signature, composition, scoping, and common override/extend patterns.
tags: [nix, overlays, nixpkgs, package-overrides]
timestamp: 2026-07-24T02:00:00Z
---

# Overlays

## Purpose

This document is the operational reference for nixpkgs overlays: what they are, the `final: prev` signature, how they compose, how they are installed, and the common override/extend patterns. It is intended for contributors who need to customize or extend the nixpkgs package set without forking it, and for reviewers checking overlay correctness.

## Sources used

- `.crawl/65-nixpkgs-overlays.md` — https://nixos.org/manual/nixpkgs/stable/#sec-overlays (PRIMARY — overlay definition, final/prev signature, installing overlays, defining overlays, alternatives BLAS/LAPACK, MPI)
- `.crawl/12-callpackage.md` — https://nix.dev/tutorials/callpackage.html (callPackage, override, interdependent package sets, callPackageWith)
- `flake.nix` — this project's flake (callPackage usage; note: this project does NOT currently use overlays — it uses direct callPackage into `packages.${system}`)

## Related Nix guidance

- `flake-anatomy.md` — flake outputs including the `overlays.<name>` output type.
- `packaging-recipes.md` — callPackage patterns and package recipes.
- `nixpkgs-library.md` — lib functions including `composeExtensions`.

## Core guidance

### What is an overlay?

> "Overlays are used to add layers in the fixed-point used by Nixpkgs to compose the set of all packages." — [nixpkgs-manual/overlays]

> "Nixpkgs can be configured with a list of overlays, which are applied in order. This means that the order of the overlays can be significant if multiple layers override the same package." — [nixpkgs-manual/overlays]

An overlay is a Nix function that modifies or extends the nixpkgs package set via a fixed-point. Overlays let you override existing packages, add new packages, and create alternative package sets without forking nixpkgs.

### Overlay signature: `final: prev: { ... }`

> "Overlays are Nix functions which accept two arguments, conventionally called either `final` and `prev` in newer code or `self` and `super` in older code, and return a set of packages." — [nixpkgs-manual/overlays]

Canonical example from the manual:

```nix
final: prev:

{
  boost = prev.boost.override { python = final.python3; };
  rr = prev.callPackage ./pkgs/rr { stdenv = final.stdenv_32bit; };
}
```

### `final` vs `prev`

> "The first argument (`final`, `self`) corresponds to the final package set. You should use this set for the dependencies of all packages specified in your overlay." — [nixpkgs-manual/overlays]

> "The second argument (`prev`, `super`) corresponds to the result of the evaluation of the previous stages of Nixpkgs. It does not contain any of the packages added by the current overlay, nor any of the following overlays. This set should be used either to refer to packages you wish to override, or to access functions defined in Nixpkgs." — [nixpkgs-manual/overlays]

| argument | name (newer) | name (older) | refers to | use for |
|----------|--------------|--------------|-----------|---------|
| first | `final` | `self` | the final, fully-overlaid package set | dependencies of packages in your overlay; refer to other overlays' results |
| second | `prev` | `super` | the previous stage (pre-this-overlay) | the original package you want to override; nixpkgs helper functions (callPackage, etc.) |

Key rule: use `final` for dependencies (so they pick up other overlays' overrides), use `prev` for the thing being overridden and for nixpkgs functions.

### Installing overlays

There are three mechanisms.

#### 1. Explicit argument when importing nixpkgs

> "The list of overlays can be passed explicitly when importing nixpkgs, for example `import <nixpkgs> { overlays = [ overlay1 overlay2 ]; }`." — [nixpkgs-manual/overlays]

> "NOTE: DO NOT USE THIS in nixpkgs. Further overlays can be added by calling the `pkgs.extend` or `pkgs.appendOverlays`, although it is often preferable to avoid these functions, because they recompute the Nixpkgs fixpoint, which is somewhat expensive to do." — [nixpkgs-manual/overlays]

#### 2. NixOS `nixpkgs.overlays` option

> "On a NixOS system the value of the `nixpkgs.overlays` option, if present, is passed to the system Nixpkgs directly as an argument. Note that this does not affect the overlays for non-NixOS operations (e.g. `nix-env`), which are looked up independently." — [nixpkgs-manual/overlays]

#### 3. Configuration lookup

When no explicit `overlays` argument is given, nixpkgs looks up overlays from configuration:

- First, if an `overlays` argument to the Nixpkgs function itself is given, that is used and no path lookup is performed.
- Otherwise, if the Nix path entry `<nixpkgs-overlays>` exists, look for overlays there.
- If one of `~/.config/nixpkgs/overlays.nix` and `~/.config/nixpkgs/overlays/` exists, look there. It is an error if both exist.
- If a directory: contents ordered lexicographically; `.nix` files imported, subdirectories use their `default.nix`.

### `packageOverrides` vs overlays

> "Overlays are similar to other methods for customizing Nixpkgs, in particular the `packageOverrides` attribute... Indeed, `packageOverrides` acts as an overlay with only the `prev` argument. It is therefore appropriate for basic use, but overlays are more powerful and easier to distribute." — [nixpkgs-manual/overlays]

Prefer overlays over `packageOverrides`. `packageOverrides` only sees `prev` (no `final`), so it cannot reference other overlays' results, making composition fragile.

### Overlay composition

Overlays compose via `lib` functions:

```nix
# Compose two overlays (right-to-left: f2 applied first, then f1)
lib.composeExtensions f1 f2

# Compose a list of overlays
lib.composeManyExtensions [ overlay1 overlay2 overlay3 ]

# Order-preserving fold (first overlay in list applied first / outermost)
lib.foldl' (lhs: rhs: lib.composeExtensions rhs lhs) [] [ o1 o2 o3 ]
```

- `composeExtensions f g` produces an overlay where `g` is applied first (inner) and `f` second (outer), so `f` sees `g`'s results in `prev`.
- `composeManyExtensions` composes a left-to-right list where the leftmost is applied last (outermost).
- The `foldl'` idiom reverses this so the first list element is outermost.

Ordering is a common source of confusion — document it carefully when composing.

### Flake overlays: `overlays.default` output

Flakes expose overlays via the `overlays` output attribute. A flake can export an overlay for consumers to apply:

```nix
{
  outputs = { self, nixpkgs }: {
    overlays.default = final: prev: {
      my-tool = prev.callPackage ./my-tool.nix { };
    };
  };
}
```

Consumers apply it via `import nixpkgs { overlays = [ my-flake.overlays.default ]; }` or in NixOS `nixpkgs.overlays = [ my-flake.overlays.default ];`.

Note: this project's `flake.nix` does NOT currently export an `overlays` output — it builds packages directly via `pkgs.callPackage` into `packages.${system}`. This is a valid alternative when you only need to consume your own packages and don't need others to overlay them.

### Common overlay patterns

**Override a package's arguments:**

```nix
final: prev: {
  boost = prev.boost.override { python = final.python3; };
}
```

**Add a new package via callPackage:**

```nix
final: prev: {
  rr = prev.callPackage ./pkgs/rr { stdenv = final.stdenv_32bit; };
}
```

**Modify a package's attributes (overrideAttrs):**

```nix
final: prev: {
  foo = prev.foo.overrideAttrs (old: {
    buildInputs = old.buildInputs ++ [ final.bar ];
    patches = (old.patches or []) ++ [ ./foo-fix.patch ];
  });
}
```

**Extend with a new package set:**

```nix
final: prev: {
  mySet = {
    helper = final.callPackage ./my-set/helper.nix { };
    tool = final.callPackage ./my-set/tool.nix { };
  };
}
```

**Configure alternatives (BLAS/LAPACK) — verbatim from crawl:**

```nix
final: prev:
{
  blas = prev.blas.override { blasProvider = final.mkl; };
  lapack = prev.lapack.override { lapackProvider = final.mkl; };
}
```

**Switch MPI implementation — verbatim from crawl:**

```nix
final: prev:
{
  mpi = final.mpich;
}
```

### Overlay scoping: `makeExtensible`, `makeScope`

- `pkgs.extend` and `pkgs.appendOverlays` recompute the fixpoint (expensive — avoid in nixpkgs itself per the crawl warning).
- `lib.makeExtensible` makes an attrset extensible (adds an `extend` method).
- `lib.makeScope` creates a scoped package set with its own `callPackage`/`newScope` for building interdependent package families (e.g. `haskellPackages`, `pythonPackages`).

This connects to the callPackage tutorial's `callPackageWith` pattern for interdependent package sets. These are advanced patterns; see the callpackage tutorial for detail.

## Practical rules

1. An overlay is a function `final: prev: { ... }` returning an attrset of overridden/new packages.
2. Use `final` for dependencies of packages in your overlay (so they pick up other overlays' overrides).
3. Use `prev` for the package being overridden and for nixpkgs helper functions (`callPackage`, `stdenv`, etc.).
4. Prefer `final`/`prev` naming over the older `self`/`super`.
5. Overlays are applied in order; order matters when multiple overlays touch the same package.
6. Prefer overlays over `packageOverrides` — overlays compose; `packageOverrides` only sees `prev`.
7. Do not use `pkgs.extend`/`pkgs.appendOverlays` inside nixpkgs itself — they recompute the fixpoint expensively.
8. In a flake, export overlays via `overlays.<name>` (conventionally `overlays.default`).
9. In NixOS, set `nixpkgs.overlays = [ ... ];` — this does NOT affect `nix-env`/non-NixOS operations.
10. For interdependent package families, use `lib.makeScope`/`newScope` rather than manual `rec` attrsets.
11. Compose multiple overlays with `lib.composeManyExtensions` (or the `foldl'` idiom for explicit ordering).
12. `override` changes callPackage arguments; `overrideAttrs` changes derivation attributes — choose the right one.

## Review checklist

- [ ] Overlay signature is `final: prev: { ... }` (or `self: super` in legacy code).
- [ ] Dependencies of overlay packages come from `final`, not `prev`.
- [ ] The overridden package and nixpkgs functions come from `prev`.
- [ ] `override` (arguments) vs `overrideAttrs` (derivation attrs) chosen correctly.
- [ ] Multiple overlays composed with `composeExtensions`/`composeManyExtensions`, not manual `//`.
- [ ] No `pkgs.extend`/`appendOverlays` used inside nixpkgs-internal code.
- [ ] Flake exports `overlays.default` when consumers need to apply the overlay.
- [ ] NixOS `nixpkgs.overlays` set as a list; aware it doesn't affect `nix-env`.
- [ ] No `packageOverrides` used where an overlay is needed (composition required).
- [ ] Overlay returns an attrset shaped like `all-packages.nix` (attr -> derivation).

## Implementation checklist

- [ ] Write the overlay as `final: prev: { ... }` in a `.nix` file.
- [ ] For each package: decide override (args) vs overrideAttrs (attrs) vs callPackage (new).
- [ ] Source dependencies from `final`; the overridden base from `prev`.
- [ ] If multiple overlays, compose with `lib.composeManyExtensions` or `foldl'`.
- [ ] Export from flake as `overlays.default` if consumers need it.
- [ ] In NixOS, add to `nixpkgs.overlays`; for user-level, add to `~/.config/nixpkgs/overlays/`.
- [ ] Test with `nix repl` — `:p (import <nixpkgs> { overlays = [ (import ./overlay.nix) ]; }).myPackage`.
- [ ] Verify `nix flake check` passes if exporting from a flake.

## Validation hooks

- `nix repl` — evaluate `import <nixpkgs> { overlays = [ overlay ]; }` and inspect attributes.
- `nix-build -E 'import <nixpkgs> { overlays = [ (import ./overlay.nix) ]; }' -A myPackage` — build an overlaid package.
- `nix flake check` — validates flake structure including `overlays` outputs.
- `nix flake show` — confirms `overlays` output appears in the attr tree.
- NixOS: `nixos-rebuild build` — evaluates `nixpkgs.overlays` without switching.

## Examples

### Minimal override overlay (from crawl, verbatim)

```nix
final: prev:

{
  boost = prev.boost.override { python = final.python3; };
  rr = prev.callPackage ./pkgs/rr { stdenv = final.stdenv_32bit; };
}
```

### BLAS/LAPACK alternative (from crawl, verbatim)

```nix
final: prev:
{
  blas = prev.blas.override { blasProvider = final.mkl; };
  lapack = prev.lapack.override { lapackProvider = final.mkl; };
}
```

### MPI switch (from crawl, verbatim)

```nix
final: prev:
{
  mpi = final.mpich;
}
```

### Flake exporting an overlay

```nix
{
  outputs = { self, nixpkgs }: {
    overlays.default = final: prev: {
      my-tool = prev.callPackage ./my-tool.nix { };
    };
  };
}
```

### This project's approach (no overlays)

This project's `flake.nix` uses direct `pkgs.callPackage` into `packages.${system}` rather than overlays, because it only consumes its own packages and doesn't need external consumers to overlay them. Abbreviated snippet:

```nix
workestrate = pkgs.callPackage ./nix/packages/agentctl.nix {
  inherit microsandbox microsandbox-filesystem-patched rustToolchain;
};
```

This is the callPackage-direct pattern, not an overlay.

## Common mistakes

- Using `prev` for dependencies instead of `final` — the dependency won't pick up other overlays' overrides, causing version skew.
- Confusing `override` (callPackage arguments) with `overrideAttrs` (derivation attributes).
- Using `packageOverrides` where composition is needed — it only sees `prev`.
- Using `pkgs.extend` inside nixpkgs — recomputes the fixpoint expensively.
- Assuming `nixpkgs.overlays` (NixOS) affects `nix-env` — it does not; they're looked up independently.
- Forgetting that overlay order matters when multiple overlays override the same package.
- Composing overlays with `//` instead of `composeExtensions` — `//` doesn't thread `final`/`prev` correctly.
- Putting both `~/.config/nixpkgs/overlays.nix` and `~/.config/nixpkgs/overlays/` — it is an error if both exist.
- Using `self`/`super` in new code — prefer `final`/`prev` (the older names still work but are discouraged).

## Strict vs contextual guidance

**Strict:**

- Overlay signature is `final: prev: { ... }`.
- Dependencies come from `final`.
- Overridden base + nixpkgs functions come from `prev`.
- Prefer overlays over `packageOverrides`.
- Do not use `pkgs.extend` inside nixpkgs.
- Compose with `lib.composeExtensions`/`composeManyExtensions`.
- Flake overlays go in `overlays.<name>`.

**Contextual:**

- Whether to export `overlays.default` from a flake (only if external consumers need it).
- Whether to use `makeScope` for a package family vs flat overlays.
- Choice of `nixos-unstable` vs release.
- Whether to use `~/.config/nixpkgs/overlays/` directory vs a single `overlays.nix` file.

## Policy decisions for individual repos

- Decide whether to export `overlays.default` (needed if other flakes/nixos configs consume your packages).
- Decide override strategy per package: `override` (args), `overrideAttrs` (attrs), or `callPackage` (new).
- Decide whether to use a single `overlays.nix` or a directory of overlay files.
- Decide whether to use `makeScope` for interdependent package families.
- Decide nixpkgs ref and whether overlays target stable or unstable.

## Related docs

- `flake-anatomy.md` — flake outputs including the `overlays.<name>` output type.
- `packaging-recipes.md` — callPackage patterns and package recipes.
- `nixpkgs-library.md` — lib functions: `composeExtensions`, `composeManyExtensions`, `makeExtensible`, `makeScope`.
- `source-map.md` — provenance index for crawled Nix sources.

## Related skills

- `nix-usage` — operational reference for the ai-workbench Nix flake, dev shell, and runtime.

## Citations

[1] [Overlays — Nixpkgs manual](https://nixos.org/manual/nixpkgs/stable/#sec-overlays)
[2] [Package parameters and overrides with callPackage — nix.dev](https://nix.dev/tutorials/callpackage.html)
[3] [callPackageWith — Nixpkgs manual](https://nixos.org/manual/nixpkgs/stable/#function-library-lib.customisation.callPackageWith)
