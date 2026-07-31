---
name: nix-modules
description: |
  Operational reference for the NixOS module system — module structure, mkOption,
  option types, config merging, submodule, mkDefault/mkForce/mkIf/mkMerge,
  imports, and evalModules. Load when writing, reviewing, or debugging NixOS
  modules, mkOption declarations, or module-system config. Does NOT cover flake
  anatomy (see nix-usage) or packaging recipes (see nix-packaging-recipes).
---

# NixOS Module System

Compact operational skill. Full detail in the authoritative long-form
reference: `docs/nix/modules-and-config.md` (module structure, mkOption,
option types, config merging, submodule, composition, evalModules). Do not
link upstream URLs from here — cite the local doc.

## Triggers

Load when:

- Writing or reviewing a NixOS module (NixOS, nix-darwin, home-manager,
  flake `nixosModules`).
- Declaring options with `lib.mkOption`.
- Choosing option types (`lib.types.*`).
- Controlling config merging priorities (`mkDefault`/`mkForce`/`mkOverride`/
  `mkIf`/`mkMerge`).
- Using `types.submodule` for nested structured options.
- Splitting configs via `imports`.
- Evaluating modules standalone with `lib.evalModules`.
- Reviewing a module PR.

Do NOT load for: flake output structure (use `nix-usage`), packaging recipes
(use `nix-packaging-recipes`), lib function reference (use
`nix-nixpkgs-library`).

## Module Structure

A module is a function returning an attrset with `options`, `config`, and/or
`imports`. The `...` (ellipsis) is required — the module system passes
arbitrary arguments.

```nix
{ config, options, lib, pkgs, ... }: {
  options = { ... };
  config = { ... };
  imports = [ ... ];
}
```

| Attribute | Purpose |
|---|---|
| `options` | Declare allowed attributes with types/defaults (`lib.mkOption`). |
| `config` | Define values for declared options (this module's contributions). |
| `imports` | Incorporate further modules (file paths or inline modules). |
| `meta` | Metadata (description, maintainers). |
| `freeformType` | A `lib.types.*` allowing arbitrary untyped config attrs (advanced). |

`lib` is passed automatically by the module system.

## mkOption

```nix
mkOption :: { ... } -> Option
```

| Field | Type | Purpose |
|---|---|---|
| `type` | `lib.types.*` | Which values are valid; required for merging. |
| `default` | Any | Value used if option not specified. |
| `description` | String | Free-text docs shown in `nixos-option` and the manual. |
| `example` | Any | Illustrative value for documentation. |
| `apply` | `a -> b` | Transform the final value (e.g. coerce to string). |

Shortcuts:

- `mkEnableOption "myApp"` — boolean option defaulting to `false`, with
  description prefixed "Whether to enable ...".
- `mkPackageOption pkgs "myApp" { default = [ "myApp" ]; }` — package option
  with a default from `pkgs`.

## Option Types

| Type | Description | Example |
|---|---|---|
| `types.str` | String; multiple definitions NOT allowed (errors) | `"hello"` |
| `types.int` | Integer | `42` |
| `types.bool` | Boolean | `true` |
| `types.listOf t` | List of `t` (merges by concatenation) | `["a" "b"]` |
| `types.attrsOf t` | Attrset whose values have type `t` | `{ a = 1; b = 2; }` |
| `types.submodule` | Nested module with its own options | `{ options=...; config=...; }` |
| `types.either t1 t2` | Accepts either of two types | `"red"` or `42` |
| `types.enum [ ... ]` | One of a list of allowed values | `"medium"` |
| `types.path` | A Nix store path | `./foo.nix` |
| `types.package` | A derivation | `pkgs.curl` |
| `types.lines` | Strings; multiple defs joined with newlines | `"line1\nline2"` |
| `types.nullOr t` | Values of type `t` or `null` | `null` or `5` |
| `types.strMatching regex` | Strings matching a regex | `"A"` (for `"[A-Z0-9]"`) |
| `types.ints.between lo hi` | Integers in an inclusive range | `10` (for `1 20`) |

Key distinction: `str` is unique (two modules setting it is an error);
`lines` merges by concatenation with newlines. Choose `lines` when multiple
modules contribute text.

## Config Merging & Priority

When multiple modules define the same option, the module system merges by
type. Priorities are integers — **lower numeric value wins**.

| Function | Priority | Meaning |
|---|---|---|
| `mkOptionDefault` | 1500 | Option default (lowest precedence) |
| `mkDefault` | 1000 | Default value |
| `mkOverride 900` | 900 | Override (medium) |
| `mkForce` | 50 | Force (high) |
| `mkOverride 0` | 0 | Highest precedence |

- `mkIf cond def` — only adds `def` if `cond` is `true`. Prefer over raw `if`
  inside attrset values where merging matters.
- `mkMerge [ { ... } { ... } ]` — combine multiple config blocks (different
  priorities or conditions) into one `config`.
- `mkBefore [ ... ]` — for list-type options, make elements appear first
  (default merge appends last).

```nix
config = lib.mkMerge [
  { services.foo.enable = true; }
  (lib.mkIf config.services.bar.enable { services.foo.extra = "x"; })
];
```

## submodule

`lib.types.submodule` declares nested structured options. The submodule has
its own `options` and `config`, type-checked during evaluation of the
top-level `config`.

- `attrsOf submodule` — named collection of submodules (one per attribute).
- Submodule argument can be a function `{ name, ... }: { ... }` to access the
  attribute name (under `attrsOf`, `name` is the attr key).

```nix
userType = lib.types.submodule ({ name, ... }: {
  options = {
    departure = lib.mkOption { type = markerType; default = {}; };
  };
  config = {
    departure.style.label = lib.mkDefault (firstUpperAlnum name);
  };
});
users = lib.mkOption { type = lib.types.attrsOf userType; };
```

## Module Composition

- `imports = [ ./vpn.nix ./kde.nix ];` — split large configs into files;
  both files can define the same option and NixOS merges them.
- `_module.args = { inherit pkgs; };` — inject `pkgs`/`lib` into the
  `evalModules` scope so module function arguments resolve.

## evalModules

```nix
pkgs.lib.evalModules { modules = [ ... ]; }
```

Produces final values in `result.config`. As long as every definition has a
corresponding declaration, evaluation succeeds; undefined options or wrong
types throw.

```nix
let pkgs = import <nixpkgs> {}; in
pkgs.lib.evalModules {
  modules = [
    ({ config, ... }: { config._module.args = { inherit pkgs; }; })
    ./default.nix
  ];
}
```

## Quick Commands

```bash
nixos-rebuild build           # eval NixOS config without switching
nixos-rebuild switch          # build + activate
nixos-rebuild test            # build + activate, don't add to boot menu
nix eval .#nixosConfigurations.<name>.config.<path>
nix-instantiate --eval eval.nix -A config.<path>
nixos-option <option>         # inspect final value of a NixOS option
nix repl -f '<nixpkgs/nixos>' # interactively explore config.*
nix flake check                # validate flake-exposed nixosModules/lib
```

## Review Checklist

1. Module function signature includes `...` (ellipsis)?
2. Every declared option has a `type`?
3. `config` argument used (not the `config` attribute) for cross-module access?
4. `mkDefault`/`mkForce`/`mkOverride` priorities chosen intentionally (lower wins)?
5. `str` not used where multiple definitions expected (use `lines`)?
6. `nullOr` options have an explicit `default` (does not auto-default to `null`)?
7. `submodule` used for structured/nested option data?
8. `imports` paths relative?
9. `mkIf` used for conditional definitions, not raw `if` where merging matters?
10. `mkMerge` used when a single `config` block needs multiple priorities/conditions?
11. `_module.args` used to inject `pkgs`/`lib` into `evalModules` scope?
12. No hardcoded `import <nixpkgs> {}` inside module bodies — accept `pkgs`/`lib` as args?

## Common Mistakes

1. **Confusing the `config` argument with the `config` attribute.** The
   argument holds the lazy-evaluated merged result across all modules; the
   attribute exposes one module's values to the system. They are not the same.
2. **Forgetting `...` in the signature.** Without it the module fails when the
   system passes arguments it doesn't list.
3. **Setting an option from the same module and reading it directly.** Option
   values are only available via `config.<path>`, not a local `options` binding.
4. **`mkForce` when `mkDefault` intended.** `mkForce` (priority 50) overrides
   almost everything including user config; `mkDefault` (1000) yields to
   explicit definitions. Swapping causes silent precedence inversions.
5. **Declaring an option without a `type`.** Required for merging; without it
   the system cannot type-check or merge multiple definitions.
6. **`str` vs `lines`.** `str` is unique (two defs = error); `lines` merges by
   newline concatenation. Choose `lines` when multiple modules contribute text.
7. **`nullOr` missing explicit `default = null`.** `nullOr t` allows `null`
   values but does NOT auto-default to `null`.
8. **Accessing option values directly instead of `config.<path>`.** Reading a
   local `options` binding does not give the merged value.

## Related Docs

- `docs/nix/modules-and-config.md` — full module system reference.

## Related Skills

- `nix-usage` — ai-workbench flake, dev shell, Rust toolchain, Microsandbox.
- `nix-nixpkgs-library` — `lib.types`, `lib.modules` reference.
