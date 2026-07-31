---
type: Reference
resource: https://nix.dev/guides/best-practices.html
title: Conventions and Style
description: Nix coding conventions, style, formatting, nixpkgs contribution patterns, and anti-patterns from the nix.dev best practices guide.
tags: [nix, conventions, style, formatting, nixfmt, best-practices]
timestamp: 2026-07-24T00:00:00Z
---

# Conventions and style

## Purpose

The conventions, idioms, and anti-patterns that govern idiomatic Nix code —
naming, scoping (`let` vs `rec` vs `with`), URL quoting, lookup paths,
attribute set merging, option priority (`mkDefault`/`mkForce`), formatting,
`meta` attributes, the `callPackage` pattern, and reproducible source paths.
This doc complements `language-fundamentals.md` (which covers the *syntax
mechanics*) by focusing on the *conventions and best-practices angle*: what to
prefer, what to avoid, and why. Cross-links to `language-fundamentals.md` are
used for syntax mechanics rather than re-teaching them.

The guidance here is drawn primarily from the nix.dev best practices guide
[best-practices], the nix.dev documentation style guide [style-guide], the
nix.dev FAQ [faq], the module system deep dive [module-system], the NixOS
configuration syntax reference [nixos-config], the `callPackage` tutorial
[callpackage], and the Nixpkgs `stdenv.mkDerivation` reference [stdenv].

## Sources used

- Crawl: `docs/nix/.crawl/29-best-practices.md` — https://nix.dev/guides/best-practices.html
  (sections: URLs; Recursive attribute set `rec { ... }`; `with` scopes;
  `<...>` lookup paths; Reproducible Nixpkgs configuration; Updating nested
  attribute sets; Reproducible source paths)
- Crawl: `docs/nix/.crawl/53-style-guide.md` — https://nix.dev/contributing/documentation/style-guide.html
  (sections: Writing style; Markup and source; Code samples; Headers; One line
  per sentence; Links)
- Crawl: `docs/nix/.crawl/30-guides-faq.md` — https://nix.dev/guides/faq.html
  (sections: How to format Nix language code automatically?; How to build
  reverse dependencies of a package?)
- Crawl: `docs/nix/.crawl/31-troubleshooting.md` — https://nix.dev/guides/troubleshooting.html
- Crawl: `docs/nix/.crawl/17-module-system-deep-dive.md` — https://nix.dev/tutorials/module-system/deep-dive.html
  (section: Module option priority, `mkDefault`)
- Crawl: `docs/nix/.crawl/72-nixos-configuration-syntax.md` — https://nix.dev/manual/nixos/stable/#sec-configuration-syntax
  (section: `mkForce` for option precedence)
- Crawl: `docs/nix/.crawl/12-callpackage.md` — https://nix.dev/tutorials/callpackage.html
  (sections: Automatic function calls; Parameterised builds; Overrides;
  Interdependent package sets)
- Crawl: `docs/nix/.crawl/66-nixpkgs-stdenv-mkDerivation.md` — https://nixos.org/manual/nixpkgs/stable/#sec-using-stdenv
  (section: `srcs` / `src`, `sourceProvenance` and `license` in `meta`)
- Sibling: `docs/nix/language-fundamentals.md` — syntax mechanics (cross-linked,
  not duplicated)
- Project: `docs/nix-purity.md` — project purity rules (cross-linked)

## Related Nix guidance

- `language-fundamentals.md` — the syntax mechanics of `let`, `with`, `rec`,
  `//`, `assert`, string interpolation, and comments. This doc references it
  for *how* a construct works; it focuses on *when* and *whether* to use it.
- `derivations-and-builds.md` — `derivation`, `stdenv.mkDerivation`, build
  phases, fetchers.
- `nixpkgs-library.md` — `pkgs.lib` functions, `callPackage`, `override`,
  `overrideAttrs`, overlays.
- `flake-anatomy.md` — `flake.nix` structure, `inputs`/`outputs`, `flake.lock`.
- `modules-and-config.md` — NixOS modules, `mkOption`, `mkIf`, `mkMerge`, the
  priority system.
- `purity-and-sandboxing.md` — eval-time and build-time purity, the sandbox.
- `../nix-purity.md` — the ai-workbench project purity rules (the project
  mandates `builtins.path { name = ...; filter = ...; }`, no `--impure`, no
  `<nixpkgs>` in flakes).

## Core guidance

### Naming conventions

- `pname` uses lowercase-hyphen (e.g. `pname = "my-package"`). This is the
  symbolic name used to derive the store path name.
- `version` is a separate string (e.g. `version = "1.2.3"`). Do not bake the
  version into `pname` (`pname = "my-package-1.2.3"` defeats reproducibility
  and breaks `override`).
- Attribute names in nixpkgs use lowercase with dashes (`lib.optional`,
  `buildInputs`, `stdenv.mkDerivation`).
- File names use the `.nix` extension and lowercase-hyphen
  (`my-package.nix`, `default.nix`, `flake.nix`).
- A package recipe file is named after the package (`hello.nix`) and is
  invoked via `callPackage ./hello.nix { ... }` [callpackage, §Automatic
  function calls].

### `let ... in` scoping

Prefer explicit `let` bindings over `with` and `rec`. The nix.dev best
practices guide states verbatim:

> Avoid `rec`. Use `let ... in`.
>
> Example:
>
> ```nix
> let
>   a = 1;
> in {
>   a = a;
>   b = a + 2;
> }
> ```
>
> [best-practices, §Recursive attribute set]

And for self-reference without `rec`:

> Self-reference can be achieved by explicitly naming the attribute set:
>
> ```nix
> let
>   argset = {
>     a = 1;
>     b = argset.a + 2;
>   };
> in
>   argset
> ```
>
> [best-practices, §Recursive attribute set]

See `language-fundamentals.md` for the scoping mechanics of `let ... in`.

### `rec` usage

Use `rec { ... }` only when the attribute set must self-reference and the
result must itself be an attribute set (not a `let` binding). The canonical
nixpkgs example is `mkDerivation rec { pname = ...; src = ...${pname}...; }`,
where `pname` is referenced inside `src`.

The danger of `rec` is shadowing-induced infinite recursion. The best
practices guide gives the simplest example:

> A common pitfall is to introduce a hard-to-debug error `infinite recursion`
> when shadowing a name. The simplest example for this is:
>
> ```nix
> let a = 1; in rec { a = a; }
> ```
>
> [best-practices, §Recursive attribute set]

Here the `a` inside `rec` refers to itself (the `rec`-scoped `a`), not the
outer `let`-bound `a`, producing infinite recursion. With `let`, the same
pattern works because `let` bindings are resolved in the enclosing scope.

### `with` usage

`with attrs; expr` brings all attributes of `attrs` into scope for `expr`.
The best practices guide lists three concrete problems:

> This approach has problems:
>
> - Static analysis can't reason about the code, because it would have to
>   actually evaluate this file to see which names are in scope.
> - When more than one `with` is used, it's not clear anymore where the names
>   are coming from.
> - Scoping rules for `with` are not intuitive, see this [Nix issue for
>   details](https://github.com/NixOS/nix/issues/490).
>
> [best-practices, §`with` scopes]

The verbatim tip:

> Do not use `with` at the top of a Nix file.
> Explicitly assign names in a `let` expression.
>
> Example:
>
> ```nix
> let
>   pkgs = import <nixpkgs> {};
>   inherit (pkgs) curl jq;
> in
>
> # ...
> ```
>
> [best-practices, §`with` scopes]

For the common `buildInputs = with pkgs; [ curl jq ];` idiom, the best
practices guide offers a `with`-free alternative:

> If you want to avoid `with` altogether, try replacing expressions of this
> form
>
> ```nix
> buildInputs = with pkgs; [ curl jq ];
> ```
>
> with the following:
>
> ```nix
> buildInputs = builtins.attrValues {
>   inherit (pkgs) curl jq;
> };
> ```
>
> [best-practices, §`with` scopes]

Contextual note: `with lib;` inside `meta` blocks is a widespread nixpkgs
convention (e.g. `meta = with lib; { license = licenses.gpl3Plus; };`), but
the best-practices guide recommends avoiding `with` altogether. See
[Strict vs contextual guidance](#strict-vs-contextual-guidance) and
[Policy decisions for individual repos](#policy-decisions-for-individual-repos).

### `<nixpkgs>` lookup paths

`<nixpkgs>` is special syntax that reads from the `$NIX_PATH` environment
variable. The best practices guide explains why this is an anti-pattern:

> This means the value of a lookup path depends on external system state.
> When using lookup paths, the same Nix expression can produce different
> results.
>
> [best-practices, §`<...>` lookup paths]

The verbatim tip:

> Declare dependencies explicitly using the techniques shown in
> [pinning-nixpkgs].
>
> Do not use lookup paths, except in minimal examples.
>
> [best-practices, §`<...>` lookup paths]

In flakes, `<nixpkgs>` is doubly wrong: flakes pin inputs via `flake.lock`,
and lookup paths are impure (they read `$NIX_PATH`). The ai-workbench project
forbids `<nixpkgs>` in flake code — see `../nix-purity.md` for the project
rule and the sanctioned pinning strategy.

### URL quoting

The Nix language supports bare URLs, but this is deprecated:

> The Nix language syntax supports bare URLs, so one could write
> `https://example.com` instead of `"https://example.com"`
>
> [RFC 45](https://github.com/NixOS/rfcs/pull/45) was accepted to deprecate
> unquoted URLs and provides a number of arguments for how this feature does
> more harm than good.
>
> [best-practices, §URLs]

The verbatim tip:

> Always quote URLs.
>
> [best-practices, §URLs]

Always quote URLs in `fetchurl`, `fetchFromGitHub`, `meta.homepage`, and
anywhere else a URL appears.

### `lib` usage

Prefer `lib.optional` / `lib.optionals` over conditional list literals:

```nix
# Bad
buildInputs = if stdenv.isDarwin then [ darwinDeps ] else [];

# Good
buildInputs = lib.optional stdenv.isDarwin darwinDeps;
```

```nix
# Bad
buildInputs = if cond then [ a b ] else [];

# Good
buildInputs = lib.optionals cond [ a b ];
```

Prefer `lib.optionalString` over `if cond then str else ""`:

```nix
# Bad
configureFlags = [ (if stdenv.isDarwin then "--with-darwin" else "") ];

# Good
configureFlags = [ (lib.optionalString stdenv.isDarwin "--with-darwin") ];
```

See `nixpkgs-library.md` for the full `lib` function index.

### String interpolation

Use `${var}` for string interpolation; it is more idiomatic than `+`
concatenation for composing strings:

```nix
# Preferred
url = "mirror://gnu/${pname}/${pname}-${version}.tar.gz";

# Works but less idiomatic
url = "mirror://gnu/" + pname + "/" + pname + "-" + version + ".tar.gz";
```

Note that only string-coercible values can be interpolated — integers must be
coerced with `toString`. See `language-fundamentals.md` for the mechanics of
string interpolation and coercion.

### Attribute set merging

The `//` operator merges two attribute sets; the right operand wins on key
conflicts, and the merge is shallow. The best practices guide gives the
canonical example:

> ```nix
> { a = 1; b = 2; } // { b = 3; c = 4; }
> ```
>
> ```nix
> { a = 1; b = 3; c = 4; }
> ```
>
> [best-practices, §Updating nested attribute sets]

The shallow-merge gotcha:

> However, names on the right take precedence, and updates are shallow.
>
> ```nix
> { a = { b = 1; }; } // { a = { c = 3; }; }
> ```
>
> ```nix
> { a = { c = 3; }; }
> ```
>
> Here, key `b` was completely removed, because the whole `a` value was
> replaced.
>
> [best-practices, §Updating nested attribute sets]

For a deep merge, use `lib.recursiveUpdate`:

> Use the [`pkgs.lib.recursiveUpdate`] Nixpkgs function:
>
> ```nix
> let pkgs = import <nixpkgs> {}; in
> pkgs.lib.recursiveUpdate { a = { b = 1; }; } { a = { c = 3;}; }
> ```
>
> ```nix
> { a = { b = 1; c = 3; }; }
> ```
>
> [best-practices, §Updating nested attribute sets]

### `assert` for validation

`assert cond; expr` evaluates `expr` only if `cond` is true; otherwise it
throws an assertion error. Use `assert` for invariants that should fail
loudly at evaluation time (e.g. `assert lib.versionAtLeast version "1.0";`).

`builtins.tryEval` catches `assert` and `throw` errors but **not** `abort`
errors. See `language-fundamentals.md` for the syntax mechanics of `assert`,
`throw`, `abort`, and `tryEval`.

### `mkDefault` / `mkForce` / `mkOverride` priority

Module options have a priority system. The module system deep dive explains:

> Module options have a *priority*, represented as an integer, which determines
> the precedence for setting the option to a particular value.
> When merging values, the priority with lowest numeric value wins.
>
> The `lib.mkDefault` modifier sets the priority of its argument value to 1000,
> the lowest precedence.
>
> This ensures that other values set for the same option will prevail.
>
> [module-system, §Module option priority]

`mkForce` raises precedence so a definition wins over others. The NixOS
configuration syntax reference gives the example:

> When that happens, it's possible to force one definition take precedence over
> the others:
>
> ```nix
> { services.httpd.adminAddr = pkgs.lib.mkForce "bob@example.org"; }
> ```
>
> [nixos-config, §Configuration syntax]

Priority table (lower number = higher precedence):

| Modifier              | Priority | Meaning                                   |
| --------------------- | -------- | ----------------------------------------- |
| `mkOverride 10`       | 10       | Explicit high precedence                  |
| `mkForce`             | 50       | Force precedence over default definitions |
| (default, no modifier)| 100      | Normal definition                         |
| `mkOptionDefault`     | 1500     | Lower than default                        |
| `mkDefault`           | 1000     | Lowest precedence (easily overridden)    |

See `modules-and-config.md` for the full module system and `mkMerge`/`mkIf`.

### Formatting tools

`nixfmt` is the official formatter:

> [`nixfmt`](https://github.com/NixOS/nixfmt) is the official formatter for
> Nix language code.
>
> `nixfmt` is used to format all code in Nixpkgs.
>
> [faq, §How to format Nix language code automatically?]

Alternatives:

- `alejandra` — a fork of `nixfmt`, more opinionated (stricter line-wrapping
  and attribute ordering).
- `nixpkgs-fmt` — an older alternative, largely superseded by `nixfmt`.
- `nix fmt` — an experimental command that invokes the configured formatter
  (set via `formatter` in `flake.nix` or the `nix.formatter` NixOS option).

Note: the ai-workbench project does **not** currently use a Nix formatter.
`just lint-nix` is a static guard (`check-nix-paths.sh`) that enforces
source-path purity, not formatting. See `../nix-purity.md`.

### Comment conventions

- `#` for single-line comments.
- `/* ... */` for block comments.

See `language-fundamentals.md` for the syntax. The nix.dev style guide advises
restraint:

> Use comments in code samples very sparingly, for instance to highlight a
> particular aspect.
>
> [style-guide, §Code samples]

Prefer self-documenting code and prose explanation outside the code block over
inline comments.

### `meta` attributes

Every derivation should declare a `meta` attribute set with at least
`description`, `homepage`, `license`, `maintainers`, and `platforms`. For
binary packages, also set `sourceProvenance`. The Nixpkgs `stdenv` reference
states:

> These should ideally actually be sources and licensed under a FLOSS license.
> If you have to use a binary upstream release or package non-free software,
> make sure you correctly mark your derivation as such in the
> [`sourceProvenance`] and [`license`] fields of the [`meta`] section.
>
> [stdenv, §`srcs` / `src`]

Example `meta` block:

```nix
meta = with lib; {
  description = "A short, one-line description of the package";
  homepage = "https://example.com/my-package";
  license = licenses.mit;
  maintainers = with maintainers; [ alice bob ];
  platforms = platforms.unix;
  sourceProvenance = [ sourceTypes.binaryNativeCode ];
};
```

### nixpkgs contribution conventions

The `callPackage` pattern is the central nixpkgs convention. The `callPackage`
tutorial explains:

> It has established a convention of composing parameterised packages with
> automatic settings through a function named `callPackage`.
>
> For every attribute in the function's argument, `callPackage` passes an
> attribute from the `pkgs` attribute set if it exists.
>
> [callpackage, §Automatic function calls]

A package recipe is a function that takes its dependencies as arguments and
returns a derivation:

```nix
# hello.nix
{ lib, stdenv, fetchurl }:
stdenv.mkDerivation rec {
  pname = "hello";
  version = "2.12";
  src = fetchurl {
    url = "mirror://gnu/${pname}/${pname}-${version}.tar.gz";
    sha256 = "1ayhp9v4m4rdhjmnl2bq3cibrbqqkgjbl3s7yk2nhlh8vj3ay16g";
  };
  meta = with lib; {
    license = licenses.gpl3Plus;
  };
}
```

Invoked via `pkgs.callPackage ./hello.nix { }`. Customization uses `override`
(override function arguments) and `overrideAttrs` (override the derivation
attributes). See `nixpkgs-library.md` for the full `callPackage`/`override`
reference.

### Reproducible source paths

`src = ./.` is impure: the store path name is derived from the working
directory name, so the same expression produces different store paths on
different machines. The best practices guide gives the fix:

> Use [`builtins.path`] with the `name` attribute set to something fixed.
>
> This will derive the symbolic name of the store path from `name` instead of
> the working directory:
>
> ```nix
> let pkgs = import <nixpkgs> {}; in
>
> pkgs.stdenv.mkDerivation {
>   name = "foo";
>   src = builtins.path { path = ./.; name = "myproject"; };
> }
> ```
>
> [best-practices, §Reproducible source paths]

The ai-workbench project mandates `builtins.path` with **both** an explicit
`name` **and** a `filter` predicate — see `../nix-purity.md` for the project
rule and the filter predicate that excludes `target/`, `result*`,
`node_modules/`, and `agents/*/build/`.

### Reproducible Nixpkgs configuration

Even with `<nixpkgs>` pinned, `import <nixpkgs> {}` can be impure because the
Nixpkgs top-level expression reads config from the filesystem. The best
practices guide gives the fix:

> Explicitly set `config` and `overlays` when importing Nixpkgs:
>
> ```nix
> import <nixpkgs> { config = {}; overlays = []; }
> ```
>
> [best-practices, §Reproducible Nixpkgs configuration]

In flakes, this is handled by the flake's `inputs` and `overlays` — but if
you `import` Nixpkgs directly, always pass explicit `config` and `overlays`.

## Practical rules

- Quote all URLs — bare URLs are deprecated by RFC 45 [best-practices, §URLs].
- Prefer `let ... in` over `rec { ... }`; use `rec` only when the attrset must
  self-reference and the result must be an attrset [best-practices, §Recursive
  attribute set].
- Do not use `with` at the top of a Nix file; use `let inherit (attrs) x; in`
  instead [best-practices, §`with` scopes].
- Replace `buildInputs = with pkgs; [ curl jq ];` with
  `buildInputs = builtins.attrValues { inherit (pkgs) curl jq; };` when
  avoiding `with` entirely [best-practices, §`with` scopes].
- Do not use `<nixpkgs>` lookup paths in production code; pin Nixpkgs via
  `flake.lock` (flakes) or `fetchTarball` + revision hash [best-practices,
  §`<...>` lookup paths]. The project forbids `<nixpkgs>` in flakes — see
  `../nix-purity.md`.
- Use `builtins.path { path = ./.; name = "myproject"; }` instead of bare
  `src = ./.` to get a reproducible store path name [best-practices,
  §Reproducible source paths]. The project additionally requires a `filter`.
- Pass `config = {}; overlays = [];` when importing Nixpkgs directly
  [best-practices, §Reproducible Nixpkgs configuration].
- Use `//` for shallow attrset merges (right operand wins); use
  `lib.recursiveUpdate` for deep merges [best-practices, §Updating nested
  attribute sets].
- Prefer `lib.optional cond x` over `if cond then [x] else []`; prefer
  `lib.optionals cond xs` over `if cond then xs else []`.
- Prefer `lib.optionalString cond str` over `if cond then str else ""`.
- Use `assert cond; expr` for invariants that should fail loudly at eval time.
- Terminate every attribute assignment with `;`.
- Use `pname` + `version` (separate strings), never bake the version into
  `pname`.
- Declare a `meta` block on every derivation: `description`, `homepage`,
  `license`, `maintainers`, `platforms`; add `sourceProvenance` for binary
  packages [stdenv, §`srcs` / `src`].
- Follow the `callPackage` pattern: a package recipe is a function taking its
  dependencies as arguments, invoked via `pkgs.callPackage ./pkg.nix { }`
  [callpackage, §Automatic function calls].
- Use `nixfmt` (the official formatter) to format Nix code; `nixfmt` is used
  to format all code in Nixpkgs [faq, §How to format Nix language code
  automatically?].
- Use comments sparingly; prefer self-documenting code and prose outside the
  code block [style-guide, §Code samples].

## Review checklist

- [ ] Are all URLs quoted (no bare `https://...`)?
- [ ] Is `rec` used only where the attrset must self-reference?
- [ ] Is `with` avoided at the top of the file?
- [ ] Are `with pkgs; [ ... ]` list idioms replaced with
      `builtins.attrValues { inherit (pkgs) ...; }` where a `with`-free style
      is desired?
- [ ] Are `<nixpkgs>` lookup paths absent (except in minimal examples)?
- [ ] Is `src = ./.` replaced with `builtins.path { path = ./.; name = ...; }`
      (and, in this project, a `filter`)?
- [ ] Does `import <nixpkgs>` pass explicit `config = {}; overlays = [];`?
- [ ] Are shallow `//` merges not used where a deep merge is needed
      (`lib.recursiveUpdate`)?
- [ ] Are `lib.optional` / `lib.optionals` / `lib.optionalString` used instead
      of conditional list/string literals?
- [ ] Does every derivation have a `meta` block with `description`, `homepage`,
      `license`, `maintainers`, `platforms`?
- [ ] Is `sourceProvenance` set on binary derivations?
- [ ] Are `pname` and `version` separate strings?
- [ ] Do package recipes follow the `callPackage` pattern (function of
      dependencies → derivation)?
- [ ] Are `assert` statements used only for invariants, not control flow?
- [ ] Is the code formatted with `nixfmt` (or the repo's chosen formatter)?
- [ ] Are comments used sparingly?

## Implementation checklist

- [ ] `pname` is lowercase-hyphen; `version` is a separate string.
- [ ] All URLs are quoted strings.
- [ ] `let ... in` is used for local bindings; `rec` only for self-referencing
      attrsets.
- [ ] No `with` at the top of the file; `with` (if used) is scoped to a small
      expression.
- [ ] No `<nixpkgs>` lookup paths in flake code.
- [ ] `src` uses `builtins.path { path = ...; name = ...; filter = ...; }`
      (project rule — see `../nix-purity.md`).
- [ ] `import <nixpkgs>` (if used) passes `config = {}; overlays = [];`.
- [ ] `//` for shallow merges; `lib.recursiveUpdate` for deep merges.
- [ ] `lib.optional` / `lib.optionals` / `lib.optionalString` used for
      conditional lists/strings.
- [ ] `meta` block present with required attributes.
- [ ] Package recipes are functions invoked via `callPackage`.
- [ ] `assert` used only for invariants.
- [ ] Code is formatted with the repo's formatter (`nixfmt` by default).
- [ ] Comments are sparse and highlight non-obvious aspects only.

## Validation hooks

- `nix fmt` — format Nix code (requires `nixfmt` or `alejandra` configured as
  the formatter). [faq, §How to format Nix language code automatically?]
- `nix flake check` — validate flake outputs (builds all outputs, runs
  checks).
- `just lint-nix` — the ai-workbench project's static guard
  (`check-nix-paths.sh`); enforces source-path purity, **not** formatting. See
  `../nix-purity.md`.
- `nix-instantiate --eval --strict file.nix` — evaluate a Nix file and force
  deep evaluation to catch lazy errors.
- `nixpkgs-review` — check reverse dependencies when modifying nixpkgs
  packages [faq, §How to build reverse dependencies of a package?]:
  ```shell-session
  $ nix-shell -p nixpkgs-review --run "nixpkgs-review wip"
  ```

## Examples

### Good vs bad `rec`

```nix
# Bad: rec with shadowing risk
let a = 1; in rec { a = a; }  # infinite recursion

# Good: let bindings
let
  a = 1;
in {
  a = a;
  b = a + 2;
}

# Good: rec only when the attrset must self-reference
stdenv.mkDerivation rec {
  pname = "hello";
  version = "2.12";
  src = fetchurl {
    url = "mirror://gnu/${pname}/${pname}-${version}.tar.gz";
    sha256 = "...";
  };
}
```

### Good vs bad `with`

```nix
# Bad: with at the top of a file
with (import <nixpkgs> {});
# ... lots of code with ambiguous scope

# Good: explicit let bindings
let
  pkgs = import <nixpkgs> {};
  inherit (pkgs) curl jq;
in
# ...

# with-free alternative to `buildInputs = with pkgs; [ curl jq ];`
buildInputs = builtins.attrValues {
  inherit (pkgs) curl jq;
};
```

### URL quoting

```nix
# Bad: bare URL (deprecated by RFC 45)
src = fetchurl {
  url = https://example.com/foo-1.0.tar.gz;
  sha256 = "...";
};

# Good: quoted URL
src = fetchurl {
  url = "https://example.com/foo-1.0.tar.gz";
  sha256 = "...";
};
```

### `lib.optional`

```nix
# Bad
buildInputs = [ coreutils ] ++ (if stdenv.isDarwin then [ darwinDeps ] else []);

# Good
buildInputs = [ coreutils ] ++ lib.optional stdenv.isDarwin darwinDeps;

# Good (multiple)
buildInputs = [ coreutils ] ++ lib.optionals stdenv.isDarwin [ libobjc IOKit ];
```

### `//` merge and `recursiveUpdate`

```nix
# Shallow merge: right operand wins
{ a = 1; b = 2; } // { b = 3; c = 4; }
# => { a = 1; b = 3; c = 4; }

# Shallow merge gotcha: nested key `b` is lost
{ a = { b = 1; }; } // { a = { c = 3; }; }
# => { a = { c = 3; }; }   # b is gone!

# Deep merge with lib.recursiveUpdate
lib.recursiveUpdate { a = { b = 1; }; } { a = { c = 3; }; }
# => { a = { b = 1; c = 3; }; }
```

### `mkDefault` / `mkForce`

```nix
# mkDefault: low precedence — other definitions win
{ services.foo.enable = lib.mkDefault true; }

# mkForce: high precedence — this definition wins
{ services.httpd.adminAddr = pkgs.lib.mkForce "bob@example.org"; }

# mkOverride N: explicit priority (lower number = higher precedence)
{ services.foo.port = lib.mkOverride 10 8080; }
```

### `meta` block

```nix
meta = with lib; {
  description = "A short description of the package";
  homepage = "https://example.com/my-package";
  license = licenses.mit;
  maintainers = with maintainers; [ alice bob ];
  platforms = platforms.unix;
  sourceProvenance = [ sourceTypes.binaryNativeCode ];
};
```

### `callPackage` pattern

```nix
# hello.nix — a package recipe is a function of its dependencies
{ lib, stdenv, fetchurl }:
stdenv.mkDerivation rec {
  pname = "hello";
  version = "2.12";
  src = fetchurl {
    url = "mirror://gnu/${pname}/${pname}-${version}.tar.gz";
    sha256 = "1ayhp9v4m4rdhjmnl2bq3cibrbqqkgjbl3s7yk2nhlh8vj3ay16g";
  };
  meta = with lib; {
    license = licenses.gpl3Plus;
  };
}

# default.nix — invoke via callPackage
let
  pkgs = import <nixpkgs> {};
in
{
  hello = pkgs.callPackage ./hello.nix {};
  # Override arguments after the fact:
  hello-custom = (pkgs.callPackage ./hello.nix {}).overrideAttrs (_: {
    version = "2.12.1";
  });
}
```

### `builtins.path` with `name`

```nix
# Bad: impure store path name derived from working directory
pkgs.stdenv.mkDerivation {
  name = "foo";
  src = ./.;
}

# Good: fixed store path name
pkgs.stdenv.mkDerivation {
  name = "foo";
  src = builtins.path { path = ./.; name = "myproject"; };
}

# Project rule (ai-workbench): also pass a filter — see ../nix-purity.md
src = builtins.path {
  path = ./.;
  name = "myproject";
  filter = path: type: baseNameOf path != "target";
};
```

## Common mistakes

- Using `rec` when `let` suffices — risks `infinite recursion` on name
  shadowing (`let a = 1; in rec { a = a; }`) [best-practices, §Recursive
  attribute set].
- `with` at the top of a file — static analysis can't reason about scope;
  multiple `with`s make name provenance unclear [best-practices, §`with`
  scopes].
- Unquoted URLs (`url = https://...`) — deprecated by RFC 45 [best-practices,
  §URLs].
- `<nixpkgs>` in flakes — impure (reads `$NIX_PATH`), non-reproducible
  [best-practices, §`<...>` lookup paths]. The project forbids it — see
  `../nix-purity.md`.
- Shallow `//` merge when a deep merge is needed — nested keys are replaced,
  not merged (`{ a = { b = 1; }; } // { a = { c = 3; }; }` loses `b`). Use
  `lib.recursiveUpdate` [best-practices, §Updating nested attribute sets].
- `if cond then [x] else []` instead of `lib.optional cond x` — more verbose
  and less idiomatic.
- `src = ./.` without `builtins.path { name = ...; }` — impure store path name
  derived from the working directory [best-practices, §Reproducible source
  paths]. The project additionally requires a `filter`.
- Forgetting `meta` attributes on derivations — `description`, `homepage`,
  `license`, `maintainers`, `platforms` should always be present; binary
  packages need `sourceProvenance` [stdenv, §`srcs` / `src`].
- Baking the version into `pname` (`pname = "foo-1.0"`) — breaks `override`
  and reproducibility; use separate `pname` and `version`.
- `import <nixpkgs> {}` without explicit `config = {}; overlays = [];` —
  impure config/overlays may leak in [best-practices, §Reproducible Nixpkgs
  configuration].
- Over-commenting code — the style guide recommends sparse comments
  [style-guide, §Code samples].

## Strict vs contextual guidance

- **Strict**: quote all URLs; no `<nixpkgs>` lookup paths in flakes; terminate
  assignments with `;`; use `builtins.path` with an explicit `name` (and, in
  this project, a `filter`); pass `config = {}; overlays = [];` when importing
  Nixpkgs directly; `pname` + `version` are separate strings.
- **Contextual**: `rec` vs `let` (prefer `let` unless the result must be an
  attrset that self-references); `with` (avoid, but `with lib;` in `meta`
  blocks is a widespread nixpkgs convention); formatter choice (`nixfmt` is
  official, `alejandra` is stricter); whether to enforce `meta` on all
  derivations or only nixpkgs-bound ones; pinning strategy (flakes vs
  `fetchTarball`).

## Policy decisions for individual repos

- **Formatter choice**: `nixfmt` (official, used by Nixpkgs), `alejandra`
  (stricter fork), or none. The ai-workbench project currently uses none;
  `just lint-nix` is a purity guard, not a formatter.
- **`with` tolerance**: ban entirely, allow only `with lib;` in `meta`/module
  code, or allow freely. The best-practices guide recommends avoiding `with`
  altogether.
- **`meta` enforcement**: require `meta` on all derivations, or only on
  derivations intended for nixpkgs contribution.
- **Pinning strategy**: flakes with `flake.lock` (the project default), or
  `fetchTarball` + revision hash for non-flake code.
- **`rec` policy**: allow freely, or require `let` unless the attrset must
  self-reference.
- **Lookup-path policy**: ban `<nixpkgs>` everywhere, or allow in minimal
  examples only (the best-practices recommendation).

## Related docs

- `language-fundamentals.md` — syntax mechanics of `let`, `with`, `rec`, `//`,
  `assert`, string interpolation, comments.
- `derivations-and-builds.md` — `derivation`, `stdenv.mkDerivation`, build
  phases, fetchers.
- `nixpkgs-library.md` — `pkgs.lib` functions, `callPackage`, `override`,
  `overrideAttrs`, overlays.
- `flake-anatomy.md` — `flake.nix` structure, `inputs`/`outputs`,
  `flake.lock`.
- `modules-and-config.md` — NixOS modules, `mkOption`, `mkIf`, `mkMerge`, the
  priority system.
- `purity-and-sandboxing.md` — eval-time and build-time purity, the sandbox.
- `../nix-purity.md` — the ai-workbench project purity rules (`builtins.path`
  with `name` and `filter`, no `--impure`, no `<nixpkgs>` in flakes).

## Related skills

- `nix-usage` — Nix flake, dev shell, Rust toolchain, and Microsandbox runtime
  in the ai-workbench.

## Citations

[1] [Best practices — nix.dev](https://nix.dev/guides/best-practices.html)
[2] [Style guide — nix.dev](https://nix.dev/contributing/documentation/style-guide.html)
[3] [FAQ — nix.dev](https://nix.dev/guides/faq.html)
[4] [Troubleshooting — nix.dev](https://nix.dev/guides/troubleshooting.html)
[5] [RFC 45 (deprecate unquoted URLs)](https://github.com/NixOS/rfcs/pull/45)
[6] [nixfmt — GitHub](https://github.com/NixOS/nixfmt)
[7] [callPackage tutorial — nix.dev](https://nix.dev/tutorials/callpackage.html)
[8] [Nix issue #490 (`with` scoping rules)](https://github.com/NixOS/nix/issues/490)
[9] [Module system deep dive — nix.dev](https://nix.dev/tutorials/module-system/deep-dive.html)
[10] [NixOS configuration syntax — nix.dev manual](https://nix.dev/manual/nixos/stable/#sec-configuration-syntax)
[11] [Nixpkgs stdenv mkDerivation — nixos.org manual](https://nixos.org/manual/nixpkgs/stable/#sec-using-stdenv)
