---
name: nix-nixpkgs-library
description: |
  Operational reference for the nixpkgs lib attribute set — lib.attrsets,
  lib.lists, lib.strings, lib.trivial, lib.sources, lib.path, lib.debug,
  lib.customisation, lib.modules, lib.types, and builtins vs lib. Load when
  using lib functions, choosing builtins vs lib, or reviewing Nix code that
  manipulates attrsets/lists/strings/sources. Does NOT cover module system
  authoring (see nix-modules) or packaging (see nix-packaging-recipes).
---

# Nixpkgs Library (lib)

Compact operational skill. Full detail in the authoritative long-form
reference: `docs/nix/nixpkgs-library.md` (attrsets, lists, strings, trivial,
sources, path, debug, customisation, modules, types). Do not link upstream
URLs from here — cite the local doc.

## Triggers

Load when:

- Using `lib.*` functions.
- Choosing `builtins` vs `lib`.
- Manipulating attrsets/lists/strings.
- Source filtering (`cleanSourceWith`).
- Using `lib.trivial.pipe`/`flip`/`const`.
- Setting module option priorities (`mkDefault`/`mkForce`/`mkOverride`).
- Using `lib.types.*`.
- Reviewing Nix code for lib usage correctness.

Do NOT load for: authoring NixOS modules (use `nix-modules`), packaging (use
`nix-packaging-recipes`), flake structure (use `nix-usage`).

## builtins vs lib

| Aspect | `builtins` | `lib` (pkgs.lib) |
|---|---|---|
| Implementation | C++ (primops) | Nix language |
| Availability | Always available, no import | Requires `import <nixpkgs> {}` or flake input |
| Access | `builtins.functionName` | `pkgs.lib.functionName` or `lib.functionName` |
| Examples | `builtins.map`, `builtins.filter`, `builtins.attrNames`, `builtins.readFile`, `builtins.fromTOML` | `lib.attrsets.mapAttrs`, `lib.lists.foldl'`, `lib.strings.concatStringsSep` |
| Notable exception | `import` available at top level (not just `builtins.import`) | `lib` often passed directly as a function arg |

## lib.attrsets

| Function | Signature / Notes |
|---|---|
| `attrByPath` | `[String] -> Any -> AttrSet -> Any` — nested access with default. |
| `getAttrFromPath` | `[String] -> AttrSet -> Any` — like `attrByPath` but throws if missing. |
| `attrValues` | `{ [String] :: a } -> [a]` — values **sorted by name** (not insertion order). |
| `attrNames` | Names sorted lexicographically (alias of `builtins.attrNames`). |
| `filterAttrs` | `(String -> a -> Bool) -> AttrSet -> AttrSet` — predicate gets name first. |
| `mapAttrs` | `(String -> a -> b) -> AttrSet -> AttrSet` — name + value. |
| `mapAttrsToList` | Like `mapAttrs` but returns a list. |
| `listToAttrs` | `[{ name; value; }] -> AttrSet` — pair with `nameValuePair`. |
| `nameValuePair` | `String -> a -> { name; value; }`. |
| `optionalAttrs` | `Bool -> AttrSet -> AttrSet` — attrset or `{}`. Prefer over `if` around attrset. |
| `hasAttrByPath` | `[String] -> AttrSet -> Bool`. Law: `hasAttrByPath [] x == true`. |

## lib.lists

| Function | Notes |
|---|---|
| `head` / `tail` / `length` / `elemAt` / `elem` / `filter` / `map` | `builtins` aliases. |
| `foldl'` | Left fold, **strict** accumulator. Always prefer over `foldl` (avoids stack overflow). |
| `foldr` | Right fold: `foldr op nul [x1 x2 ... xn] == op x1 (op x2 ... (op xn nul))`. |
| `concatLists` | `[[a]] -> [a]`. |
| `flatten` | Deeply nested list → single-level. |
| `unique` | Remove duplicates. |
| `intersectLists` / `subtractLists` | Set ops on lists. |
| `zipListsWith` | `(a -> b -> c) -> [a] -> [b] -> [c]`. |
| `genList` | `(Int -> a) -> Int -> [a]`. |
| `range` | `Int -> Int -> [Int]` inclusive. |

## lib.strings

| Function | Notes |
|---|---|
| `stringToCharacters` | `String -> [String]` (inefficient; no unicode). |
| `stringLength` | `builtins` alias. |
| `substring` | `Int -> Int -> String -> String` — **2nd arg is LENGTH, not end index**. |
| `replaceStrings` | `builtins` — lists of from/to patterns. |
| `replaceString` | `lib` — single from/to pair. Different function from `replaceStrings`! |
| `splitString` | `String -> String -> [String]`. |
| `concatStringsSep` | `String -> [String] -> String`. |
| `concatMapStringsSep` | `String -> (a -> String) -> [a] -> String`. |
| `toLower` / `toUpper` | ASCII only. |
| `hasPrefix` / `hasSuffix` | `String -> String -> Bool`. |
| `removePrefix` / `removeSuffix` | Remove if present. |
| `escape` | `[String] -> String -> String` — backslash-escape. |
| `trim` | Strip leading/trailing whitespace (` `, `\t`, `\r`, `\n`). |
| `optionalString` | `Bool -> String -> String` — string or `""`. |

## lib.trivial

| Function | Notes |
|---|---|
| `id` | `a -> a` — identity. |
| `const` | `a -> b -> a` — constant; ignore second arg. |
| `pipe` | `a -> [(a->b) (b->c) ...] -> z` — left-to-right composition. |
| `flip` | `(a -> b -> c) -> (b -> a -> c)` — flip arg order. |
| `composeExtensions` | Compose two overlay-style `final: prev:` fns. |
| `composeManyExtensions` | Compose a list of overlay-style fns. |

## lib.sources

| Function | Notes |
|---|---|
| `cleanSourceWith` | `{ src, filter, name } -> Source` — **composes** (prefer over `builtins.filterSource`). |
| `sourceByRegex` | `SourceLike -> [String] -> Source`. |
| `sourceFilesBySuffices` | `SourceLike -> [String] -> Source`. |
| `pathIsDirectory` | `Path -> Bool`. |

Note: `pathExists` is `builtins.pathExists` — there is NO `lib.pathExists`.

`builtins.filterSource` does NOT compose — it creates intermediate store
copies. Use `cleanSourceWith` to chain filters without intermediate copies.

## lib.path

Newer (nixpkgs 23.11+) well-typed path ops on the `Path` type:

- `lib.path.append :: Path -> Path -> Path`
- `lib.path.components :: Path -> [String]`
- `lib.path.hasPrefix :: Path -> Path -> Bool`

## lib.debug

| Function | Notes |
|---|---|
| `traceVal` | `a -> a` — print to stderr, return value. |
| `traceSeq` | `a -> b -> b` — force first arg, return second. |
| `traceValFn` | `(a -> b) -> a -> a` — trace after applying fn. |
| `traceIf` | `Bool -> a -> a` — trace only if condition true. |

## lib.customisation

- `callPackageWith :: AttrSet -> Path -> AttrSet -> a`
- `callPackagesWith :: AttrSet -> Path -> AttrSet -> AttrSet`
- `extendDerivation :: Bool -> AttrSet -> Derivation -> Derivation`
- `makeOverridable :: (a -> b) -> a -> b`

## lib.modules + lib.types

Cross-ref `nix-modules` for full module authoring. Priority table (lower wins):

| Function | Priority | Meaning |
|---|---|---|
| `mkOptionDefault` | 1500 | Option default (lowest precedence) |
| `mkDefault` | 1000 | Default value |
| `mkOverride 900` | 900 | Override (medium) |
| `mkForce` | 50 | Force (high) |
| `mkOverride 0` | 0 | Highest precedence |

Also: `mkOption`, `mkEnableOption`, `mkIf`, `mkMerge`. Types: `types.str`,
`types.int`, `types.bool`, `types.listOf`, `types.attrsOf`, `types.submodule`.

## Quick Commands

```bash
nix repl                                  # interactive evaluation
nix eval .#<attr>                         # eval a flake attr
nix-instantiate --eval expr.nix           # eval a file (lazy)
nix-instantiate --eval --strict expr.nix  # force deep evaluation
nix flake check                           # validate flake lib exports
```

## Review Checklist

1. Correct `lib` namespace (not `builtins` for lib fns)?
2. `builtins.*` used for primops; `lib.*` for nixpkgs library fns?
3. `attrByPath`/`getAttrFromPath` for nested access (not manual `.` chains)?
4. `filterAttrs` predicate checks name first (avoid unnecessary value eval)?
5. `foldl'` (strict) not `foldl` (stack overflow on large lists)?
6. `concatStringsSep`/`concatMapStringsSep` not manual string concat?
7. `hasPrefix`/`removePrefix` not `substring`+length?
8. `cleanSourceWith` not `builtins.filterSource` for composition?
9. `optionalAttrs` not `if` around the entire attrset?
10. `listToAttrs`+`nameValuePair` when converting lists to attrsets?
11. Module option priorities (`mkDefault`/`mkForce`/`mkOverride`) correct?
12. `lib` passed as function arg, not hardcoded `import <nixpkgs> {}`?

## Common Mistakes

1. **`builtins.filterSource` for composing filters.** Does not compose — creates
   intermediate store copies. Use `lib.sources.cleanSourceWith`.
2. **`foldl` instead of `foldl'`.** Non-strict `foldl` builds unevaluated thunks
   → stack overflow. Always use `foldl'` (with the prime).
3. **`substring` arg order.** `substring start length s` — second arg is LENGTH,
   not end index. `substring 0 2 "hello"` → `"he"`, not `"hel"`.
4. **`builtins.attrNames` when `lib.attrNames` canonical.** Both work; prefer
   `lib.*` in nixpkgs code for consistency.
5. **`attrValues` sorts by name.** `attrValues {c=3; a=1; b=2;}` → `[1 2 3]`,
   not `[3 1 2]`. Nix attrsets are unordered.
6. **`if cond then {x=1;} else {}` instead of `optionalAttrs`.** `optionalAttrs
   cond { x = 1; }` is more idiomatic and composes with `//`.
7. **Not passing `lib` as arg.** Hardcoding `import <nixpkgs> {}` inside
   library code breaks purity/reproducibility. Accept `lib`/`pkgs` as args.
8. **`replaceStrings` (builtins) vs `replaceString` (lib) confusion.** Former
   takes lists of from/to patterns; latter takes a single from/to pair.
9. **`toString true` returns `"1"`** not `"true"`. Use `lib.boolToString` for
   the string `"true"`/`"false"`.
10. **`hasAttrByPath [] x` returns `true`.** Empty path is always valid —
    documented law: `hasAttrByPath [] x == true`.

## Related Docs

- `docs/nix/nixpkgs-library.md` — full lib reference.
- `docs/nix/language-fundamentals.md` — Nix language basics.

## Related Skills

- `nix-usage` — ai-workbench flake, dev shell, Rust toolchain, Microsandbox.
- `nix-modules` — module system authoring (`lib.modules`, `lib.types`).
