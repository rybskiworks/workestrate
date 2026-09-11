---
type: Reference
resource: https://nix.dev/tutorials/module-system/
title: Modules and Configuration
description: Reference for the NixOS module system — module structure, options, config, mkOption, option types, config merging, submodule, module composition, and evalModules.
tags: [nix, modules, options, config, mkOption]
timestamp: 2026-07-24T00:00:00Z
---

# Modules and Configuration

## Purpose

This document is a reference for the NixOS module system. It is intended for future AI agents who write, review, refactor, debug, or validate Nix modules. It covers module structure, option declaration (`mkOption`), option types (`lib.types.*`), config merging and priorities (`mkDefault`/`mkForce`/`mkOverride`/`mkIf`/`mkMerge`), the `submodule` type, module composition (`imports`, `_module.args`), and evaluation via `lib.evalModules`.

The module system is the backbone of NixOS, nix-darwin, home-manager, and many flake-based library designs. Understanding it is prerequisite to writing reusable, composable Nix configurations.

## Sources used

- https://nix.dev/tutorials/module-system/index.html (PRIMARY — module system overview)
- https://nix.dev/tutorials/module-system/a-basic-module/index.html (PRIMARY — basic module tutorial)
- https://nix.dev/tutorials/module-system/deep-dive.html (PRIMARY — deep dive: submodule, types, mkIf, mkDefault, either, enum)
- https://nixos.org/manual/nixos/stable/#sec-configuration-syntax (NixOS configuration syntax, imports, merging, mkForce, mkBefore)
- https://nixos.org/manual/nixos/stable/#sec-writing-modules (Writing NixOS Modules)
- https://nixos.org/manual/nixpkgs/stable/#function-library-lib.options.mkOption (lib.mkOption)
- https://nixos.org/manual/nixpkgs/stable/#module-system-lib-evalModules (lib.evalModules)
- https://nixos.org/manual/nixos/stable/#sec-option-types-basic (Option types)

The crawl files `docs/nix/.crawl/15-module-system.md`, `docs/nix/.crawl/16-a-basic-module.md`, `docs/nix/.crawl/17-module-system-deep-dive.md`, and `docs/nix/.crawl/72-nixos-configuration-syntax.md` are from nix.dev commit 139034be (2026-07-21) and NixOS 26.05 respectively.

## Core guidance

### What is the module system?

> "The module system is a Nix language library that enables you to - Declare one attribute set using many separate Nix expressions. - Imposes type constraints on values in that attribute set. - Define values for the same attribute in different Nix expressions and merge these values automatically according to their type."

(nix.dev, "Module system")

These Nix expressions are called *modules* and must have a particular structure. A module is fundamentally a function that returns an attribute set:

> "A module is a function that takes an attribute set and returns an attribute set."

> "It may declare options, telling which attributes are allowed in the final outcome."

> "It may define values, for options declared by itself or other modules."

> "When evaluated by the module system, it produces an attribute set based on the declarations and definitions."

(nix.dev, "A basic module")

The simplest possible module is a function that takes any attributes and returns an empty attribute set:

```nix
{ ... }:
{
}
```

### Module structure

A module is a function with the shape:

```nix
{ config, options, lib, pkgs, ... }: {
  options = { ... };
  config = { ... };
  imports = [ ... ];
}
```

> "The first line (`{ config, pkgs, ... }:`) denotes that this is actually a function that takes at least the two arguments `config` and `pkgs`."

(NixOS manual, "Configuration Syntax")

The function returns a set of option definitions. The `...` (ellipsis) is necessary because the module system can pass arbitrary arguments to modules.

| Attribute | Purpose |
|---|---|
| `options` | Declare which attributes are allowed, with types and defaults (`lib.mkOption`). |
| `config` | Define values for declared options (this module's contributions). |
| `imports` | Incorporate further modules (file paths or inline modules). |
| `meta` | Metadata about the module (description, maintainers, etc.). |
| `freeformType` | A `lib.types.*` value allowing arbitrary untyped attributes in the config (advanced). |

The `lib` argument is passed automatically by the module system, making nixpkgs library functions available in each module's function body.

### mkOption

Options are declared under the top-level `options` attribute with `lib.mkOption`:

```nix
mkOption :: { ... } -> Option
```

Key fields:

| Field | Type | Purpose |
|---|---|---|
| `type` | `lib.types.*` | Which values are valid for the option (required for merging behavior). |
| `default` | Any | The value used if the option is not specified otherwise. |
| `description` | String | Free-text documentation shown in `nixos-option` and the manual. |
| `example` | Any | An illustrative value for documentation. |
| `apply` | `a -> b` | Transform the final value (e.g. coerce to a string). |

> "The attribute `type` in the argument to `lib.mkOption` specifies which values are valid for an option."

(nix.dev, "A basic module")

Here we declare an option `name` of type `str`:

```nix
{ lib, ... }: {
  options = {
    name = lib.mkOption {
      type = lib.types.str;
    };
  };
}
```

The module system will expect a string when a value is defined for `name`.

### Option types

Option types live under `lib.types`. They do two things: check values for validity, and specify how multiple definitions of an option are combined (merged).

| Type | Description | Example value |
|---|---|---|
| `types.str` | A string; multiple definitions not allowed | `"hello"` |
| `types.int` | An integer | `42` |
| `types.bool` | A boolean | `true` |
| `types.listOf t` | A list of elements of type `t` | `["a" "b"]` |
| `types.attrsOf t` | An attrset whose values have type `t` | `{ a = 1; b = 2; }` |
| `types.submodule` | Nested module with its own options | `{ options = ...; config = ...; }` |
| `types.either t1 t2` | Accepts either of two types | `"red"` or `42` |
| `types.enum [ ... ]` | One of a list of allowed values | `"medium"` |
| `types.path` | A Nix store path | `./foo.nix` |
| `types.package` | A derivation | `pkgs.curl` |
| `types.lines` | Strings; multiple defs joined with newlines | `"line1\nline2"` |
| `types.nullOr t` | Values of type `t` or `null` | `null` or `5` |
| `types.strMatching regex` | Strings matching a regex | `"A"` (for `"[A-Z0-9]"`) |
| `types.ints.between lo hi` | Integers in an inclusive range | `10` (for `1 20`) |

> "The `lines` type means that the only valid values are strings, and that multiple definitions should be joined with newlines."

(nix.dev, "Module system deep dive")

The difference between `str` and `lines` is in their merging behavior:

> "For `lines`, multiple definitions get merged by concatenation with newlines. For `str`, multiple definitions are not allowed."

(nix.dev, "Module system deep dive")

Assigning an integer to a `lines` option produces a type error:

```console
$ nix-instantiate --eval eval.nix -A config.scripts.output
error: A definition for option `scripts.output' is not of type `strings concatenated with "\n"'. Definition values:
- In `/home/nix-user/default.nix': 42
```

(nix.dev, "Module system deep dive")

### mkEnableOption and mkPackageOption

```nix
mkEnableOption :: String -> Option
```

`mkEnableOption` creates a boolean option that defaults to `false`, with a description prefixed by "Whether to enable ...". It is the idiomatic way to declare an enable flag:

```nix
{ lib, ... }: {
  options = {
    services.myApp.enable = lib.mkEnableOption "myApp";
  };
}
```

`mkPackageOption` declares a package option with a default from `pkgs`:

```nix
mkPackageOption :: AttrSet -> String -> { default :: [String], ... } -> Option
```

```nix
{ lib, pkgs, ... }: {
  options = {
    services.myApp.package = lib.mkPackageOption pkgs "myApp" {
      default = [ "myApp" ];
    };
  };
}
```

### Config merging

When multiple modules define the same option, the module system merges them according to the option's type. For list types, definitions are concatenated; for `lines`, joined with newlines; for `str` (unique), multiple definitions are an error unless priorities disambiguate.

Module options have a *priority*, represented as an integer. Lower numeric value wins.

> "Module options have a *priority*, represented as an integer, which determines the precedence for setting the option to a particular value. When merging values, the priority with lowest numeric value wins. The `lib.mkDefault` modifier sets the priority of its argument value to 1000, the lowest precedence."

(nix.dev, "Module system deep dive")

This ensures that other values set for the same option will prevail over a `mkDefault` value.

#### Priority table

| Function | Priority | Meaning |
|---|---|---|
| `mkOptionDefault` | 1500 | Option default (lowest precedence) |
| `mkDefault` | 1000 | Default value |
| `mkOverride 900` | 900 | Override (medium) |
| `mkForce` | 50 | Force (high) |
| `mkOverride 0` | 0 | Highest precedence |

(See also `./nixpkgs-library.md` § lib.modules for the same table.)

#### mkForce

When two modules define a unique option (like `services.httpd.adminAddr`), the merge fails. To force one definition to take precedence:

```nix
{ services.httpd.adminAddr = pkgs.lib.mkForce "bob@example.org"; }
```

(NixOS manual, "Configuration Syntax")

#### mkBefore

For list-type options, the value in `configuration.nix` is merged last. To make a list element appear first, use `mkBefore`:

```nix
{ boot.kernelModules = mkBefore [ "kvm-intel" ]; }
```

> "This causes the `kvm-intel` kernel module to be loaded before any other kernel modules."

(NixOS manual, "Configuration Syntax")

#### mkIf

> "use the `mkIf <condition> <definition>` function, which only adds the definition if the condition evaluates to `true`."

(nix.dev, "Module system deep dive")

```nix
config = {
  requestParams = [
    "size=640x640"
    "scale=2"
    (lib.mkIf (config.map.zoom != null)
      "zoom=${toString config.map.zoom}")
  ];
};
```

#### mkMerge

```nix
mkMerge :: [AttrSet] -> AttrSet
```

Merge multiple attribute sets into one, useful when a single `config` block needs to set options at different priorities or conditionally:

```nix
config = lib.mkMerge [
  { services.foo.enable = true; }
  (lib.mkIf config.services.bar.enable { services.foo.extra = "x"; })
];
```

### lib.types.submodule

> "This type allows you to define nested modules with their own options."

(nix.dev, "Module system deep dive")

A `submodule` is declared by passing an attribute set (or function) to `lib.types.submodule`. The submodule has its own `options` and `config`, type-checked during evaluation of the top-level `config`.

The `markerType` submodule from the deep-dive tutorial:

```nix
markerType = lib.types.submodule {
  options = {
    location = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
    };
    style.label = lib.mkOption {
      type = lib.types.nullOr (lib.types.strMatching "[A-Z0-9]");
      default = null;
    };
    style.color = lib.mkOption {
      type = colorType;
      default = "red";
    };
    style.size = lib.mkOption {
      type = lib.types.enum [ "tiny" "small" "medium" "large" ];
      default = "medium";
    };
  };
};
```

#### Nested submodules

Submodules can nest. A `userType` uses `attrsOf` to allow multiple named users, each with a `departure` marker of type `markerType`:

```nix
userType = lib.types.submodule {
  options = {
    departure = lib.mkOption {
      type = markerType;
      default = {};
    };
  };
};

# In options:
users = lib.mkOption {
  type = lib.types.attrsOf userType;
};
```

#### Submodule with function argument

By transforming the argument to `lib.types.submodule` into a function, you can access special arguments. One such argument is `name`, which (under `attrsOf`) gives the attribute name the submodule is defined under:

```nix
userType = lib.types.submodule ({ name, ... }: {
  options = {
    departure = lib.mkOption {
      type = markerType;
      default = {};
    };
  };
  config = {
    departure.style.label = lib.mkDefault (firstUpperAlnum name);
  };
});
```

Here `mkDefault` (priority 1000) ensures an explicitly-set label prevails over the computed default.

### Module composition

#### imports

> "The module schema includes the `imports` attribute, which allows incorporating further modules, for example to split a large configuration into multiple files."

(nix.dev, "Module system deep dive")

```nix
{ config, pkgs, ... }:
{
  imports = [
    ./vpn.nix
    ./kde.nix
  ];
  services.httpd.enable = true;
  environment.systemPackages = [ pkgs.emacs ];
}
```

(NixOS manual, "Configuration Syntax")

Both `configuration.nix` and `kde.nix` can define the same option (e.g. `environment.systemPackages`); NixOS merges the definitions. For list types, the lists are concatenated.

#### Generating modules with code

Modules can be generated programmatically rather than written as files:

```nix
{ config, pkgs, ... }:
let
  netConfig = hostName: {
    networking.hostName = hostName;
    networking.useDHCP = false;
  };
in
{
  imports = [ (netConfig "nixos.localdomain") ];
}
```

(NixOS manual, "Configuration Syntax")

This has the same effect as importing a file that sets those options.

#### _module.args

To make `pkgs` (or other values) available as a module function argument inside `evalModules`, set `config._module.args`:

```nix
({ config, ... }: { config._module.args = { inherit pkgs; }; })
```

(nix.dev, "Module system deep dive")

This mechanism is currently only documented in the module system code (`lib/modules.nix`), and that documentation is incomplete and out of date.

### evalModules

> "Modules are evaluated by `lib.evalModules` from the Nixpkgs library. It takes an attribute set as an argument, where the `modules` attribute is a list of modules to merge and evaluate. The output of `evalModules` contains information about all evaluated modules, and the final values appear in the attribute `config`."

(nix.dev, "A basic module")

The `eval.nix` helper from the deep-dive tutorial:

```nix
let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-23.11";
  pkgs = import nixpkgs { config = {}; overlays = []; };
in
pkgs.lib.evalModules {
  modules = [
    ({ config, ... }: { config._module.args = { inherit pkgs; }; })
    ./default.nix
  ];
}
```

(nix.dev, "Module system deep dive")

As long as every definition has a corresponding declaration, evaluation succeeds. If an option is defined but not declared, or the defined value has the wrong type, the module system throws an error.

## Practical rules

- A module is a function returning an attrset with `options`, `config`, and/or `imports`.
- Always include `...` in the module function signature — the module system passes arbitrary arguments.
- Every option must have a `type` (from `lib.types.*`) for merging behavior to work.
- Access option values through the `config` argument, not by reading the `config` attribute directly.
- Use `mkDefault` for low-precedence defaults and `mkForce` for high-precedence overrides; remember lower numeric priority wins.
- Use `mkIf` for conditional definitions and `mkMerge` to combine multiple config blocks.
- Use `submodule` for structured/nested options; use `attrsOf submodule` for named collections.
- Use `imports` to split large configurations into multiple files.
- Pass `pkgs` into `evalModules` via `config._module.args = { inherit pkgs; };` when modules need it.
- Prefer `mkEnableOption` over a manual `types.bool` option with `default = false`.

## Review checklist

- [ ] Module function signature includes `...` (ellipsis)
- [ ] Every declared option has a `type`
- [ ] `config` argument used (not the `config` attribute) for cross-module value access
- [ ] `mkDefault`/`mkForce`/`mkOverride` priorities chosen intentionally (lower wins)
- [ ] `str` not used where multiple definitions are expected (use `lines` instead)
- [ ] `nullOr` options have an explicit `default` (does not auto-default to `null`)
- [ ] `submodule` used for structured/nested option data
- [ ] `imports` used to split large configs; paths are relative
- [ ] `mkIf` used for conditional definitions, not raw `if` inside attrset values where merging matters
- [ ] `mkMerge` used when a single `config` block needs multiple priorities or conditions
- [ ] `_module.args` used to inject `pkgs`/`lib` into `evalModules` scope
- [ ] No hardcoded `import <nixpkgs> {}` inside module bodies — accept `pkgs`/`lib` as arguments

## Implementation checklist

- [ ] Define the module function: `{ lib, config, ... }: { ... }`
- [ ] Declare options under `options` with `lib.mkOption { type = ...; }`
- [ ] Define values under `config` referencing declared options
- [ ] Add `imports = [ ./path.nix ];` for file-split modules
- [ ] Use `lib.mkEnableOption` for enable flags
- [ ] Use `lib.mkPackageOption` for package-selection options
- [ ] Use `lib.types.submodule` for nested structured options
- [ ] Use `lib.mkIf`/`lib.mkMerge` for conditional/compound config
- [ ] Use `lib.mkDefault`/`lib.mkForce`/`lib.mkOverride` for priority control
- [ ] Evaluate with `lib.evalModules { modules = [ ... ]; }` and read `result.config`

## Validation hooks

- `nix-instantiate --eval eval.nix -A config.<path>` — evaluate a specific option path
- `nix-instantiate --eval eval.nix -A config` — evaluate the full merged config
- `nix-build eval.nix -A config.<path>` — build a derivation-valued option
- `nixos-option <option>` — inspect the final value of a NixOS option
- `nix repl -f '<nixpkgs/nixos>'` — interactively explore `config.*`
- `nix flake check` — validate flake-exposed `nixosModules`/`lib` outputs
- `nix eval .#nixosConfigurations.<name>.config.<path>` — evaluate a flake NixOS config option

## Examples

### Example 1: `nix/lib/config.nix` — function-returns-attrset pattern

This project file is not a full NixOS module, but follows the same function-returns-attrset shape. It takes a `configDir` argument (with a filtered `builtins.path` default) and returns an attrset of derived values:

```nix
# From nix/lib/config.nix
{ configDir ?
    builtins.path {
      path = ../../config.reference;
      filter = path: _type: baseNameOf path == "workestrate.toml";
      name = "workestrate-config-reference";
    } }:
let
  configPath = "${configDir}/workestrate.toml";
  raw = builtins.fromTOML (builtins.readFile configPath);
in {
  secrets = raw.secrets or {};
  workloads = raw.workloads or {};
  workloadNames = builtins.attrNames (raw.workloads or {});

  nixLayeredImages = builtins.filter (name:
    let wl = raw.workloads.${name}; in
    (wl.image.recipe or "") == "nix-layered"
  ) (builtins.attrNames (raw.workloads or {}));

  localBuilds = builtins.filter (name:
    raw.workloads.${name} ? local_build
  ) (builtins.attrNames (raw.workloads or {}));
}
```

The filtered `builtins.path` default copies only `workestrate.toml` into the store, not the entire repo.

### Example 2: `nix/lib/vocabulary.nix` — `{ pkgs }: rec { ... }` pattern

A function taking an attrset argument and returning an attrset with `packages`, `features`, and helper functions. This is the module-like "closed vocabulary" pattern:

```nix
# From nix/lib/vocabulary.nix
{ pkgs }:
rec
{
  packages = {
    cacert = pkgs.cacert;
    busybox = pkgs.busybox;
    fakeNss = pkgs.dockerTools.fakeNss;
    nodejs_24 = pkgs.nodejs_24;
    nmap = pkgs.nmap;
    dnsutils = pkgs.bind.dnsutils;
  };

  features = {
    create_tmp = ''
      mkdir -p tmp
      chmod 1777 tmp
    '';
  };

  bakedFileToShell = { path, content }:
    assert builtins.isString path;
    assert builtins.substring 0 1 path != "/";
    assert builtins.match ".*\\.\\..*" path == null;
    let contentFile = builtins.toFile "baked-file-content" content; in
    ''
      mkdir -p $(dirname ${path})
      cp ${contentFile} ${path}
    '';

  resolvePackages = names:
    builtins.map (n: builtins.getAttr n packages) names;

  resolveFeatures = names:
    builtins.concatStringsSep "\n" (builtins.map (n: builtins.getAttr n features) names);

  resolveBakedFiles = files:
    builtins.concatStringsSep "\n" (builtins.map bakedFileToShell files);
}
```

`resolvePackages` maps name strings to derivations; `resolveFeatures` maps name strings to shell text; `bakedFileToShell` generates safe shell from a `{ path, content }` attrset.

### Example 3: Minimal evalModules

A synthesized minimal example combining declaration, definition, and evaluation:

```nix
# options.nix — declare
{ lib, ... }: {
  options = {
    name = lib.mkOption {
      type = lib.types.str;
    };
  };
}

# config.nix — define
{ ... }: {
  config = {
    name = "Boaty McBoatface";
  };
}

# default.nix — evaluate
let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-23.11";
  pkgs = import nixpkgs { config = {}; overlays = []; };
in
pkgs.lib.evalModules {
  modules = [
    ./options.nix
    ./config.nix
  ];
}
```

Evaluating `nix-instantiate --eval default.nix -A config.name` yields `"Boaty McBoatface"`.

### Example 4: Submodule (markerType)

The `markerType` submodule from the deep-dive tutorial, declaring a structured marker with location and style:

```nix
# marker.nix
{ lib, config, ... }:
let
  colorType = lib.types.either
    (lib.types.strMatching "0x[0-9A-F]{6}")
    (lib.types.enum [
      "black" "brown" "green" "purple" "yellow"
      "blue" "gray" "orange" "red" "white" ]);

  markerType = lib.types.submodule {
    options = {
      location = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
      };
      style.label = lib.mkOption {
        type = lib.types.nullOr (lib.types.strMatching "[A-Z0-9]");
        default = null;
      };
      style.color = lib.mkOption {
        type = colorType;
        default = "red";
      };
      style.size = lib.mkOption {
        type = lib.types.enum [ "tiny" "small" "medium" "large" ];
        default = "medium";
      };
    };
  };
in {
  options = {
    map.markers = lib.mkOption {
      type = lib.types.listOf markerType;
    };
  };
}
```

## Common mistakes

1. **Confusing the `config` argument with the `config` attribute.** The `config` *argument* holds the result of the module system's lazy evaluation across all modules; the `config` *attribute* exposes one module's option values to the system. They are not the same thing.

> "The `config` *argument* is **not** the same as the `config` *attribute*:
> - The `config` *argument* holds the result of the module system's lazy evaluation, which takes into account all modules passed to `evalModules` and their `imports`.
> - The `config` *attribute* of a module exposes that particular module's option values to the module system for evaluation."

(nix.dev, "Module system deep dive")

2. **Forgetting `...` in the module function signature.** The ellipsis is necessary because the module system can pass arbitrary arguments to modules. Without `...`, the module will fail when the system passes arguments it doesn't list.

> "The ellipsis `...` is necessary because the module system can pass arbitrary arguments to modules."

(nix.dev, "A basic module")

3. **Setting an option value from within the same module's `config` and expecting it directly.** Option values can't be accessed directly from the same module without the `config` argument. The module system evaluates all modules it receives, and any of them can define a particular option's value; what happens when an option is set by multiple modules is determined by that option's type. Use `config.<path>` to read the merged value.

4. **Using `mkForce` when `mkDefault` is intended.** `mkForce` sets priority 50 (high precedence), overriding almost everything including user config. `mkDefault` sets priority 1000 (low precedence), yielding to explicit definitions. Swapping them causes silent precedence inversions.

5. **Declaring an option without a `type`.** The `type` field is required for merging behavior. Without it, the module system cannot type-check values or merge multiple definitions, leading to evaluation errors or undefined behavior.

6. **Forgetting that `str` does not allow multiple definitions while `lines` does.** `str` is a unique type — two modules setting the same `str` option is an error. `lines` merges by concatenation with newlines. Choose `lines` when multiple modules contribute text.

7. **Not handling `nullOr` defaults.** `nullOr t` allows `null` values but does *not* automatically default to `null`. You must still set `default = null;` explicitly:

> "This _does not_ automatically mean that when the option isn't defined, the value of such an option is `null` -- we still need to define a default value."

(nix.dev, "Module system deep dive")

8. **Trying to access option values directly instead of through `config.<path>`.** Option values are only available through the `config` argument passed to the module function. Reading a local `options` binding does not give the merged value.

9. **Using `builtins.map` when `lib.map` semantics differ in submodule context.** When an option is named `map` (as in the deep-dive tutorial's `config.map`), calling `map` resolves to the option, not the list function. Use `builtins.map` or `lib.map` explicitly to avoid confusion:

> "To avoid confusion with the `map` option setting and the final `config.map` configuration value, here we use the `map` function explicitly as `builtins.map`."

(nix.dev, "Module system deep dive")

## Strict vs contextual guidance

Strict: a module is a function returning an attrset with `options`/`config`/`imports`; every option has a `type`; the function signature includes `...`; the `config` argument is used for cross-module value access; priorities are integers and lower numeric value wins; `evalModules` produces the final values in the `config` attribute.

Contextual: whether to split modules into multiple files via `imports`; choice of option types (`submodule` vs `attrsOf` for complex data); whether to use `mkEnableOption` vs a manual bool option; granularity of submodule nesting; whether to expose `pkgs` via `_module.args` or pass as a function argument; whether to use `evalModules` directly or via the NixOS/flake module system.

## Policy decisions

- Decide whether to use `evalModules` directly or via the NixOS/flake module system (NixOS `nixosSystem`, flake `nixosModules`, home-manager, etc.).
- Decide option type granularity: `submodule` for structured/nested data vs flat `attrsOf` for simple maps.
- Decide default values and whether options should be required (no `default`) or optional (with `default`).
- Decide module file organization: single file vs `imports` split across multiple files.
- Decide whether to expose `pkgs` via `_module.args` or pass as a function argument to each module.
- Decide priority strategy: prefer `mkDefault` for low-precedence defaults; reserve `mkForce`/`mkOverride 0` for cases where user config must be overridden.

## Related docs

- [Nixpkgs Library](./nixpkgs-library.md) — `lib.modules` and `lib.types` reference (priority table, type table)
- [Nix Language Fundamentals](./language-fundamentals.md) — Nix language basics, data types, syntax
- [Flake Anatomy](./flake-anatomy.md) — flake structure (flakes use the module system for `nixosModules`)
- [Nix Source Map](./source-map.md) — provenance index for all crawled nix.dev sources

## Related skills

- `nix-usage` — operational reference for the ai-workbench Nix flake, dev shell, Rust toolchain, and Microsandbox runtime

## Citations

[1] [Module system — nix.dev](https://nix.dev/tutorials/module-system/index.html)
[2] [A basic module — nix.dev](https://nix.dev/tutorials/module-system/a-basic-module/index.html)
[3] [Module system deep dive — nix.dev](https://nix.dev/tutorials/module-system/deep-dive.html)
[4] [NixOS Configuration Syntax — NixOS manual](https://nixos.org/manual/nixos/stable/#sec-configuration-syntax)
[5] [Writing NixOS Modules — NixOS manual](https://nixos.org/manual/nixos/stable/#sec-writing-modules)
[6] [lib.mkOption — nixpkgs manual](https://nixos.org/manual/nixpkgs/stable/#function-library-lib.options.mkOption)
[7] [lib.evalModules — nixpkgs manual](https://nixos.org/manual/nixpkgs/stable/#module-system-lib-evalModules)
[8] [Option types — NixOS manual](https://nixos.org/manual/nixos/stable/#sec-option-types-basic)
