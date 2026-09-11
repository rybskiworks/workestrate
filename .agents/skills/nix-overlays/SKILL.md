---
name: nix-overlays
description: |
  Operational reference for nixpkgs overlays — the final:prev signature,
  composition, scoping, flake overlays.default, and override/extend patterns.
  Load when writing or reviewing overlays, modifying the nixpkgs package set,
  or exporting overlays from a flake. Does NOT cover module system (see
  nix-modules) or packaging recipes (see nix-packaging-recipes).
---

# Nixpkgs Overlays

Compact operational skill. Full detail in the authoritative long-form
reference: `docs/nix/overlays.md` (overlay signature, composition, scoping,
flake overlays, common patterns). Do not link upstream URLs from here — cite
the local doc.

## Triggers

Load when:

- Writing or reviewing overlays.
- Using the `final: prev:` signature.
- Modifying/extending the nixpkgs package set.
- Using `composeExtensions`/`composeManyExtensions`.
- Exporting `overlays.default` from a flake.
- Setting the NixOS `nixpkgs.overlays` option.
- Choosing `override` vs `overrideAttrs` in overlay context.
- Using `makeScope`/`makeExtensible`.
- Reviewing an overlay PR.

Do NOT load for: module system (use `nix-modules`), packaging recipes/callPackage
detail (use `nix-packaging-recipes`), flake output structure (use `nix-usage`).

## Overlay Signature

An overlay is a function `final: prev: { ... }` returning an attrset of
overridden/new packages. Overlays are applied in order; order matters when
multiple overlays touch the same package.

```nix
final: prev: {
  boost = prev.boost.override { python = final.python3; };
  rr = prev.callPackage ./pkgs/rr { stdenv = final.stdenv_32bit; };
}
```

| Argument | Newer name | Older name | Refers to | Use for |
|---|---|---|---|---|
| first | `final` | `self` | final, fully-overlaid package set | dependencies of packages in your overlay; refer to other overlays' results |
| second | `prev` | `super` | previous stage (pre-this-overlay) | the original package being overridden; nixpkgs helper functions (`callPackage`, `stdenv`, etc.) |

**Key rule:** `final` for deps (so they pick up other overlays' overrides);
`prev` for the base being overridden and for nixpkgs functions. Prefer
`final`/`prev` over the older `self`/`super`.

## Installing Overlays

Three mechanisms:

1. **Explicit on import:** `import <nixpkgs> { overlays = [ o1 o2 ]; }`.
2. **NixOS option:** `nixpkgs.overlays = [ ... ];` — does NOT affect `nix-env`
   (looked up independently).
3. **Config lookup:** `~/.config/nixpkgs/overlays.nix` OR
   `~/.config/nixpkgs/overlays/` directory. **Error if both exist.** Directory
   contents ordered lexicographically; `.nix` files imported, subdirectories use
   their `default.nix`.

Avoid `pkgs.extend`/`pkgs.appendOverlays` inside nixpkgs itself — they
recompute the fixpoint expensively.

## packageOverrides vs overlays

`packageOverrides` acts as an overlay with only the `prev` argument — it
cannot reference other overlays' results (no `final`). Appropriate for basic
use only. **Prefer overlays** — they compose and distribute better.

## Composition

```nix
# Compose two: g applied first (inner), f second (outer)
lib.composeExtensions f g

# Compose a list: leftmost applied LAST (outermost)
lib.composeManyExtensions [ o1 o2 o3 ]

# foldl' idiom: first list element is outermost
lib.foldl' (lhs: rhs: lib.composeExtensions rhs lhs) [] [ o1 o2 o3 ]
```

- `composeExtensions f g` — `g` applied first/inner, `f` second/outer; `f`
  sees `g`'s results in `prev`.
- `composeManyExtensions` — leftmost applied last/outermost.
- `foldl'` idiom reverses so the first list element is outermost.

Ordering is a common confusion source — document it carefully. **Do NOT
compose with `//`** — it doesn't thread `final`/`prev` correctly.

## Flake Overlays

```nix
{
  outputs = { self, nixpkgs }: {
    overlays.default = final: prev: {
      my-tool = prev.callPackage ./my-tool.nix { };
    };
  };
}
```

Consumers apply via `import nixpkgs { overlays = [ my-flake.overlays.default ]; }`
or NixOS `nixpkgs.overlays = [ my-flake.overlays.default ];`.

Note: this project's flake uses direct `callPackage` into
`packages.${system}` instead of overlays — valid when only consuming own
packages and external consumers don't need to overlay them.

## Common Patterns

```nix
# Override args
final: prev: { boost = prev.boost.override { python = final.python3; }; }

# Add new package
final: prev: { rr = prev.callPackage ./pkgs/rr { stdenv = final.stdenv_32bit; }; }

# Modify attrs (overrideAttrs)
final: prev: {
  foo = prev.foo.overrideAttrs (old: {
    buildInputs = old.buildInputs ++ [ final.bar ];
    patches = (old.patches or []) ++ [ ./foo-fix.patch ];
  });
}

# Extend with new package set
final: prev: {
  mySet = {
    helper = final.callPackage ./my-set/helper.nix { };
    tool   = final.callPackage ./my-set/tool.nix { };
  };
}

# BLAS/LAPACK alternative
final: prev: {
  blas = prev.blas.override { blasProvider = final.mkl; };
  lapack = prev.lapack.override { lapackProvider = final.mkl; };
}

# MPI switch
final: prev: { mpi = final.mpich; }
```

## Scoping

- `lib.makeExtensible` — makes an attrset extensible (adds `extend` method).
- `lib.makeScope`/`newScope` — scoped package set with its own `callPackage`
  for interdependent package families (e.g. `haskellPackages`,
  `pythonPackages`). Cross-ref `nix-packaging-recipes` (`callPackageWith`).

## Quick Commands

```bash
nix build --override-input nixpkgs <path>     # build against a custom nixpkgs
nix repl                                       # :p (import <nixpkgs> { overlays=[(import ./overlay.nix)]; }).myPackage
nix-build -E 'import <nixpkgs> { overlays = [ (import ./overlay.nix) ]; }' -A myPackage
nix flake check                                # validate flake overlays output
nix flake show                                 # confirm overlays appears in attr tree
nixos-rebuild build                            # eval nixpkgs.overlays without switching
```

## Review Checklist

1. Signature is `final: prev: { ... }` (or `self: super` in legacy)?
2. Deps of overlay packages come from `final`, not `prev`?
3. Overridden base + nixpkgs fns come from `prev`?
4. `override` (args) vs `overrideAttrs` (attrs) chosen correctly?
5. Multiple overlays composed with `composeExtensions`/`composeManyExtensions`, not `//`?
6. No `pkgs.extend`/`appendOverlays` in nixpkgs-internal code?
7. Flake exports `overlays.default` when consumers need it?
8. NixOS `nixpkgs.overlays` is a list; aware it doesn't affect `nix-env`?
9. No `packageOverrides` where composition needed?
10. Overlay returns an attrset shaped like `all-packages.nix` (attr → derivation)?

## Common Mistakes

1. **Using `prev` for deps instead of `final`.** Dep won't pick up other
   overlays' overrides → version skew.
2. **Confusing `override` vs `overrideAttrs`.** `override` changes callPackage
   args; `overrideAttrs` changes derivation attrs.
3. **`packageOverrides` where composition needed.** Only sees `prev` (no
   `final`); cannot reference other overlays' results.
4. **`pkgs.extend` inside nixpkgs.** Recomputes the fixpoint expensively.
5. **Assuming `nixpkgs.overlays` affects `nix-env`.** It does not; looked up
   independently.
6. **Forgetting overlay order matters** when multiple overlays override the
   same package.
7. **Composing with `//` instead of `composeExtensions`.** `//` doesn't thread
   `final`/`prev` correctly.
8. **Both `overlays.nix` and `overlays/` dir present.** It is an error if both
   exist.
9. **`self`/`super` in new code.** Prefer `final`/`prev` (older names still
   work but are discouraged).

## Related Docs

- `docs/nix/overlays.md` — full overlays reference.
- `docs/nix/flake-anatomy.md` — flake outputs including `overlays.<name>`.
- `docs/nix/packaging-recipes.md` — callPackage patterns and recipes.

## Related Skills

- `nix-usage` — ai-workbench flake, dev shell, Rust toolchain, Microsandbox.
- `nix-packaging-recipes` — callPackage, override/overrideAttrs detail.
- `nix-nixpkgs-library` — `composeExtensions`, `makeScope`.
