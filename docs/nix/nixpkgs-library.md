---
type: Reference
resource: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library
title: Nixpkgs Library
description: Reference for the nixpkgs lib attribute set — attrsets, lists, strings, trivial, sources, and module/customisation helpers.
tags: [nix, nixpkgs, lib, library, functions]
timestamp: 2026-07-24T00:00:00Z
---

# Nixpkgs Library

## Purpose

This document is a multi-section reference for the nixpkgs `lib` attribute set. It is intended for future AI agents who write, review, refactor, debug, or validate Nix code. Each section explains a sub-attribute set of `lib` — its functions, type signatures, common idioms, and pitfalls — so agents can reason about nixpkgs library usage locally without re-reading the nixpkgs manual from scratch.

This file covers `lib.attrsets`, `lib.lists`, `lib.strings`, `lib.trivial`, `lib.sources`, `lib.path`, `lib.debug`, `lib.customisation`, `lib.modules`, and `lib.types`.

## Sources used

- https://nixos.org/manual/nixpkgs/stable/#sec-functions-library (PRIMARY)
- https://nix.dev/tutorials/nix-language.html#function-libraries
- https://nix.dev/manual/nix/stable/language/builtins.html
- https://github.com/NixOS/nixpkgs/blob/master/lib/default.nix
- https://github.com/NixOS/nixpkgs/blob/master/lib/attrsets.nix
- https://github.com/NixOS/nixpkgs/blob/master/lib/strings.nix
- https://github.com/NixOS/nixpkgs/blob/master/lib/lists.nix
- https://github.com/NixOS/nixpkgs/blob/master/lib/trivial.nix
- https://github.com/NixOS/nixpkgs/blob/master/lib/sources.nix
- https://github.com/NixOS/nixpkgs/blob/master/lib/modules.nix
- https://github.com/NixOS/nixpkgs/blob/master/lib/types.nix
- https://github.com/NixOS/nixpkgs/blob/master/lib/customisation.nix
- https://github.com/NixOS/nixpkgs/blob/master/lib/debug.nix
- https://github.com/NixOS/nixpkgs/blob/master/lib/path.nix

The crawl file `docs/nix/.crawl/70-nixpkgs-lib-functions.md` covers nixpkgs 26.05. The crawl file `docs/nix/.crawl/10-nix-language-basics.md` is from nix.dev commit 139034be (2026-07-21).

## builtins vs lib

> "In addition to the built-in operators (`+`, `==`, `&&`, etc.), there are two widely used libraries that *together* can be considered standard for the Nix language."

> "Nix comes with many functions that are built into the language. They are implemented in C++ as part of the Nix language interpreter."

> "Most built-in functions are only accessible through `builtins`. A notable exception is `import`, which is also available at the top level."

> "The `nixpkgs` repository contains an attribute set called `lib`, which provides a large number of useful functions. They are implemented in the Nix language, as opposed to `builtins`, which are part of the language itself."

> "These functions are usually accessed through `pkgs.lib`, as the Nixpkgs attribute set is given the name `pkgs` by convention."

| Aspect | `builtins` | `lib` (pkgs.lib) |
|---|---|---|
| Implementation | C++ (primops) | Nix language |
| Availability | Always available, no import needed | Requires `import <nixpkgs> {}` or flake input |
| Access | `builtins.functionName` | `pkgs.lib.functionName` or `lib.functionName` |
| Examples | `builtins.map`, `builtins.filter`, `builtins.attrNames`, `builtins.readFile`, `builtins.fromTOML` | `lib.attrsets.mapAttrs`, `lib.lists.foldl'`, `lib.strings.concatStringsSep` |
| Notable exception | `import` is available at top level (not just `builtins.import`) | `lib` is often passed directly as a function argument |

## lib.attrsets

Attribute set functions. Source: nixpkgs manual, `lib/attrsets.nix`.

### `attrByPath`

```nix
attrByPath :: [String] -> Any -> AttrSet -> Any
```

> "Returns an attribute from nested attribute sets."

(nixpkgs manual, `lib/attrsets.nix`)

```nix
x = { a = { b = 3; }; }
attrByPath ["a" "b"] 6 x
=> 3
attrByPath ["z" "z"] 6 x
=> 6
```

### `getAttrFromPath`

```nix
getAttrFromPath :: [String] -> AttrSet -> Any
```

> "Like `attrByPath`, but without a default value. If it doesn't find the path it will throw an error."

(nixpkgs manual, `lib/attrsets.nix`)

### `attrValues`

```nix
attrValues :: { [String] :: a } -> [a]
```

> "Returns the values of all attributes in the given set, sorted by attribute name."

```nix
attrValues {c = 3; a = 1; b = 2;}
=> [1 2 3]
```

### `attrNames`

Returns the names of all attributes in the given set, sorted lexicographically. (Note: this is actually a `builtins` function, but `lib.attrNames` is an alias.)

### `filterAttrs`

```nix
filterAttrs :: (String -> a -> Bool) -> { [String] :: a } -> { [String] :: a }
```

> "Filter an attribute set by removing all attributes for which the given predicate return false."

(nixpkgs manual, `lib/attrsets.nix`)

```nix
filterAttrs (n: v: n == "foo") { foo = 1; bar = 2; }
=> { foo = 1; }
```

### `mapAttrs`

```nix
mapAttrs :: (String -> a -> b) -> { [String] :: a } -> { [String] :: b }
```

Apply a function to each attribute in the given set, receiving both name and value. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `mapAttrsToList`

```nix
mapAttrsToList :: (String -> a -> b) -> { [String] :: a } -> [b]
```

Like `mapAttrs` but returns a list instead of an attrset. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `listToAttrs`

```nix
listToAttrs :: [{ name :: String; value :: a; }] -> { [String] :: a }
```

Construct an attribute set from a list of name-value pairs. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `nameValuePair`

```nix
nameValuePair :: String -> a -> { name :: String; value :: a; }
```

Construct a name-value pair. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `optionalAttrs`

```nix
optionalAttrs :: Bool -> { [String] :: a } -> { [String] :: a }
```

Return the given attribute set if the condition is true, otherwise an empty attrset. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `hasAttrByPath`

```nix
hasAttrByPath :: [String] -> AttrSet -> Bool
```

> "Returns if an attribute from nested attribute set exists."

(nixpkgs manual, `lib/attrsets.nix`)

Law: `hasAttrByPath [] x == true`.

```nix
x = { a = { b = 3; }; }
hasAttrByPath ["a" "b"] x
=> true
hasAttrByPath ["z" "z"] x
=> false
hasAttrByPath [] (throw "no need")
=> true
```

## lib.lists

List manipulation functions. Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library.

### `head`

```nix
head :: [a] -> a
```

Return the first element of a list. (`builtins.head` alias)

### `tail`

```nix
tail :: [a] -> [a]
```

Return the list without its first element. (`builtins.tail` alias)

### `length`

```nix
length :: [a] -> Int
```

Return the length of a list. (`builtins.length` alias)

### `elemAt`

```nix
elemAt :: [a] -> Int -> a
```

Return the element at the given index. (`builtins.elemAt` alias)

### `elem`

```nix
elem :: a -> [a] -> Bool
```

Return true if the given value is in the list. (`builtins.elem` alias)

### `filter`

```nix
filter :: (a -> Bool) -> [a] -> [a]
```

Filter a list by a predicate. (`builtins.filter` alias)

### `map`

```nix
map :: (a -> b) -> [a] -> [b]
```

Apply a function to each element. (`builtins.map` alias)

### `foldl'`

```nix
foldl' :: (b -> a -> b) -> b -> [a] -> b
```

Left fold with strict accumulator. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library) The `'` suffix means strict evaluation of the accumulator — the accumulator is forced to weak head normal form at each step, preventing the buildup of unevaluated thunks.

### `foldr`

```nix
foldr :: (a -> b -> b) -> b -> [a] -> b
```

> "right fold" a binary function `op` between successive elements of `list` with `nul` as the starting value, i.e., `foldr op nul [x_1 x_2 ... x_n] == op x_1 (op x_2 ... (op x_n nul))`.

(nixpkgs manual, `lib/lists.nix`)

```nix
concat = foldr (a: b: a + b) "z"
concat [ "a" "b" "c" ]
=> "abcz"
```

### `concatLists`

```nix
concatLists :: [[a]] -> [a]
```

Concatenate a list of lists into a single list. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `flatten`

```nix
flatten :: [Any] -> [Any]
```

Flatten a deeply nested list into a single-level list. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `unique`

```nix
unique :: [a] -> [a]
```

Remove duplicate elements from a list. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `intersectLists`

```nix
intersectLists :: [a] -> [a] -> [a]
```

Return the intersection of two lists. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `subtractLists`

```nix
subtractLists :: [a] -> [a] -> [a]
```

Subtract the second list from the first. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `zipListsWith`

```nix
zipListsWith :: (a -> b -> c) -> [a] -> [b] -> [c]
```

Zip two lists with a combining function. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `genList`

```nix
genList :: (Int -> a) -> Int -> [a]
```

Generate a list of length `n` by applying the function to each index. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `range`

```nix
range :: Int -> Int -> [Int]
```

Generate a list of integers from `first` to `last` inclusive. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

## lib.strings

String manipulation functions. Source: nixpkgs manual, `lib/strings.nix`.

### `stringToCharacters`

```nix
stringToCharacters :: String -> [String]
```

> "Convert a string `s` to a list of characters (i.e. singleton strings). This allows you to, e.g., map a function over each character. However, note that this will likely be horribly inefficient; Nix is not a general purpose programming language. Complex string manipulations should, if appropriate, be done in a derivation. Also note that Nix treats strings as a list of bytes and thus doesn't handle unicode."

(nixpkgs manual, `lib/strings.nix`)

```nix
stringToCharacters "abc"
=> [ "a" "b" "c" ]
```

### `stringLength`

Return the length of a string. (`builtins.stringLength` alias)

### `substring`

```nix
substring :: Int -> Int -> String -> String
```

Return a substring. (`builtins.substring` alias) Note: the second argument is length, not end index.

### `replaceStrings`

```nix
replaceStrings :: [String] -> [String] -> String -> String
```

Replace occurrences of strings. (`builtins.replaceStrings` alias) Note: `lib.strings.replaceString` is a different function (single replacement).

### `replaceString`

> "Given string `s`, replace every occurrence of the string `from` with the string `to`."

(nixpkgs manual, `lib/strings.nix`)

```nix
replaceString "world" "Nix" "Hello, world!"
=> "Hello, Nix!"
```

### `splitString`

```nix
splitString :: String -> String -> [String]
```

Split a string by a separator. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `concatStringsSep`

```nix
concatStringsSep :: String -> [String] -> String
```

> "Concatenate a list of strings with a separator between each element."

(nixpkgs manual, `lib/strings.nix`)

```nix
concatStringsSep "/" ["usr" "local" "bin"]
=> "usr/local/bin"
```

### `concatMapStringsSep`

```nix
concatMapStringsSep :: String -> (a -> String) -> [a] -> String
```

> "Maps a function over a list of strings and then concatenates the result with the specified separator interspersed between elements."

(nixpkgs manual, `lib/strings.nix`)

```nix
concatMapStringsSep "-" (x: toUpper x)  ["foo" "bar" "baz"]
=> "FOO-BAR-BAZ"
```

### `toLower`

```nix
toLower :: String -> String
```

> "Converts an ASCII string `s` to lower-case."

(nixpkgs manual, `lib/strings.nix`)

```nix
toLower "HOME"
=> "home"
```

### `toUpper`

```nix
toUpper :: String -> String
```

> "Converts an ASCII string `s` to upper-case."

(nixpkgs manual, `lib/strings.nix`)

```nix
toUpper "home"
=> "HOME"
```

### `hasPrefix`

```nix
hasPrefix :: String -> String -> Bool
```

> "Determine whether a string has given prefix."

(nixpkgs manual, `lib/strings.nix`)

```nix
hasPrefix "foo" "foobar"
=> true
hasPrefix "foo" "barfoo"
=> false
```

### `hasSuffix`

```nix
hasSuffix :: String -> String -> Bool
```

> "Determine whether a string has given suffix."

(nixpkgs manual, `lib/strings.nix`)

```nix
hasSuffix "foo" "foobar"
=> false
hasSuffix "foo" "barfoo"
=> true
```

### `removePrefix`

```nix
removePrefix :: String -> String -> String
```

Remove a prefix from a string if present. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `removeSuffix`

```nix
removeSuffix :: String -> String -> String
```

Remove a suffix from a string if present. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `escape`

```nix
escape :: [String] -> String -> String
```

> "Escape occurrence of the elements of `list` in `string` by prefixing it with a backslash."

(nixpkgs manual, `lib/strings.nix`)

```nix
escape ["(" ")"] "(foo)"
=> "\\(foo\\)"
```

### `trim`

```nix
trim :: String -> String
```

> "Remove leading and trailing whitespace from a string `s`."

(nixpkgs manual, `lib/strings.nix`) Whitespace is defined as any of the following characters: ' ', '\t' '\r' '\n'.

```nix
trim "   hello, world!   "
=> "hello, world!"
```

### `optionalString`

```nix
optionalString :: Bool -> String -> String
```

> "Depending on the boolean `cond`, return either the given string or the empty string. Useful to concatenate against a bigger string."

(nixpkgs manual, `lib/strings.nix`)

```nix
optionalString true "some-string"
=> "some-string"
optionalString false "some-string"
=> ""
```

## lib.trivial

Miscellaneous functions. Source: nixpkgs manual, `lib/trivial.nix`.

### `id`

```nix
id :: a -> a
```

> "The identity function For when you need a function that does "nothing"."

(nixpkgs manual, `lib/trivial.nix`)

### `const`

```nix
const :: a -> b -> a
```

> "The constant function. Ignores the second argument. If called with only one argument, constructs a function that always returns a static value."

(nixpkgs manual, `lib/trivial.nix`)

```nix
let f = const 5; in f 10
=> 5
```

### `pipe`

```nix
pipe :: a -> [(a -> b) (b -> c) ... (x -> y) (y -> z)] -> z
```

> "Pipes a value through a list of functions, left to right."

(nixpkgs manual, `lib/trivial.nix`)

```nix
pipe 2 [
    (x: x + 2)  # 2 + 2 = 4
    (x: x * 2)  # 4 * 2 = 8
  ]
=> 8
```

### `flip`

```nix
flip :: (a -> b -> c) -> (b -> a -> c)
```

> "Flip the order of the arguments of a binary function."

(nixpkgs manual, `lib/trivial.nix`)

```nix
flip concat [1] [2]
=> [ 2 1 ]
```

### `composeExtensions`

```nix
composeExtensions :: (a -> a -> a) -> (a -> a -> a) -> (a -> a -> a)
```

Compose two overlay-style functions. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library) This is used for nixpkgs overlays, combining two `final: prev:` functions into one.

### `composeManyExtensions`

```nix
composeManyExtensions :: [(a -> a -> a)] -> (a -> a -> a)
```

Compose many overlay-style functions. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

## lib.sources

Source filtering functions. Source: nixpkgs manual, `lib/sources.nix`.

### `cleanSourceWith`

```nix
cleanSourceWith :: { src :: SourceLike, filter :: String -> String -> Bool, name :: String } -> Source
```

> "Like `builtins.filterSource`, except it will compose with itself, allowing you to chain multiple calls together without any intermediate copies being put in the nix store."

(nixpkgs manual, `lib/sources.nix`)

```nix
lib.cleanSourceWith {
  filter = f;
  src = lib.cleanSourceWith {
    filter = g;
    src = ./.;
  };
}
# Succeeds!

builtins.filterSource f (builtins.filterSource g ./.)
# Fails!
```

### `sourceByRegex`

```nix
sourceByRegex :: SourceLike -> [String] -> Source
```

> "Filter sources by a list of regular expressions."

(nixpkgs manual, `lib/sources.nix`)

```nix
src = sourceByRegex ./my-subproject [".*\\.py$" "^database\\.sql$"]
```

### `sourceFilesBySuffices`

```nix
sourceFilesBySuffices :: SourceLike -> [String] -> Source
```

> "Get all files ending with the specified suffices from the given source directory or its descendants, omitting files that do not match any suffix."

(nixpkgs manual, `lib/sources.nix`)

```nix
sourceFilesBySuffices ./. [ ".xml" ".c" ]
```

### `pathIsDirectory`

```nix
pathIsDirectory :: Path -> Bool
```

Check if a path is a directory. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `pathExists`

Note: this is actually `builtins.pathExists`, not `lib.pathExists`. There is no `lib.pathExists`; use `builtins.pathExists` directly.

## lib.path

Path manipulation functions. Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library-path.

`lib.path` is a newer addition (introduced in nixpkgs 23.11+) providing well-typed path operations that avoid common string-path pitfalls. Unlike string-based path manipulation, `lib.path` functions operate on the `Path` type and enforce type safety at the boundary.

### `lib.path.subpath`

Subpath manipulation utilities. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library-path)

### `lib.path.append`

```nix
lib.path.append :: Path -> Path -> Path
```

Append two paths. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library-path)

### `lib.path.components`

```nix
lib.path.components :: Path -> [String]
```

Split a path into its components. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library-path)

### `lib.path.hasPrefix`

```nix
lib.path.hasPrefix :: Path -> Path -> Bool
```

Check if a path is a prefix of another. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library-path)

## lib.debug

Debugging and tracing functions. Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library.

### `traceVal`

```nix
traceVal :: a -> a
```

Trace a value (print to stderr, return the value). Uses `builtins.trace` under the hood. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `traceSeq`

```nix
traceSeq :: a -> b -> b
```

Force evaluation of the first argument before returning the second. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `traceValFn`

```nix
traceValFn :: (a -> b) -> a -> a
```

Trace a value after applying a function to it. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `traceIf`

```nix
traceIf :: Bool -> a -> a
```

Trace only if the condition is true. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

## lib.customisation

Package customisation functions. Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library.

### `callPackageWith`

```nix
callPackageWith :: AttrSet -> Path -> AttrSet -> a
```

Call a package function with auto-filled arguments from an attribute set. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `callPackagesWith`

```nix
callPackagesWith :: AttrSet -> Path -> AttrSet -> AttrSet
```

Like `callPackageWith` but for packages that return an attrset. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `extendDerivation`

```nix
extendDerivation :: Bool -> AttrSet -> Derivation -> Derivation
```

Extend a derivation with additional attributes. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `makeOverridable`

```nix
makeOverridable :: (a -> b) -> a -> b
```

Make a function result overridable. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

## lib.modules

Module system functions. Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library.

### `mkOption`

```nix
mkOption :: { ... } -> Option
```

Declare a module option. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library) Key fields: `type` (a `lib.types.*` value), `default` (the default value), `description` (free-text documentation), `example` (an illustrative value).

### `mkEnableOption`

```nix
mkEnableOption :: String -> Option
```

Create a boolean option that defaults to `false`, with an `enable` prefix. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `mkIf`

```nix
mkIf :: Bool -> AttrSet -> AttrSet
```

Conditionally include an attribute set. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `mkDefault`

```nix
mkDefault :: a -> a
```

Set a default priority (1000) value. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `mkForce`

```nix
mkForce :: a -> a
```

Set a force priority (50) value. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `mkOverride`

```nix
mkOverride :: Int -> a -> a
```

Set a value with a custom priority. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library) Lower priority number = higher precedence.

### `mkMerge`

```nix
mkMerge :: [AttrSet] -> AttrSet
```

Merge multiple attribute sets into one. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### Priority table

| Function | Priority | Meaning |
|---|---|---|
| `mkOptionDefault` | 1500 | Option default (lowest precedence) |
| `mkDefault` | 1000 | Default value |
| `mkOverride 900` | 900 | Override (medium) |
| `mkForce` | 50 | Force (high) |
| `mkOverride 0` | 0 | Highest precedence |

## lib.types

Option types. Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library.

### `types.str`

A string type. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `types.int`

An integer type. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `types.bool`

A boolean type. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `types.listOf`

A list of a given type: `types.listOf types.str`. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `types.attrsOf`

An attribute set of a given value type: `types.attrsOf types.int`. (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)

### `types.submodule`

A submodule (nested module system). (Source: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library) This is the most powerful type, allowing nested option declarations — the submodule's `options` are themselves declared with `mkOption`, and its `config` is merged into the parent.

### Type table

| Type | Description | Example |
|---|---|---|
| `types.str` | String | `"hello"` |
| `types.int` | Integer | `42` |
| `types.bool` | Boolean | `true` |
| `types.listOf types.str` | List of strings | `["a" "b"]` |
| `types.attrsOf types.int` | Attrset of integers | `{ a = 1; b = 2; }` |
| `types.submodule` | Nested module | `{ options = ...; config = ...; }` |

## Real-world examples from the project

**Example 1: `builtins.attrNames` + `builtins.filter` in `nix/lib/config.nix`:**

```nix
# From nix/lib/config.nix
workloadNames = builtins.attrNames (raw.workloads or {});

nixLayeredImages = builtins.filter (name:
  let wl = raw.workloads.${name}; in
  (wl.image.recipe or "") == "nix-layered"
) (builtins.attrNames (raw.workloads or {}));
```

**Example 2: `builtins.listToAttrs` + `map` + `nameValuePair` pattern in `flake.nix`:**

```nix
# From flake.nix — buildImagesFromConfig
builtins.listToAttrs (map (name: {
  name = workloads.${name}.image.name;
  value = recipesForPkgs.image.nix-layered {
    inherit (workloads.${name}.image) name tag;
    contents = workloads.${name}.image.contents or [];
    # ...
  };
}) nixLayered);
```

**Example 3: `lib.hasPrefix` + `lib.removePrefix` in `flake.nix`:**

```nix
# From flake.nix — resolveSrc
resolveSrc = srcStr:
  if lib.hasPrefix "flake://" srcStr then
    let name = lib.removePrefix "flake://" srcStr; in
    if sources ? ${name} then
      sources.${name}
    else
      throw "buildImagesFromConfig: unresolved flake:// URI '${srcStr}'"
  else
    srcStr;
```

**Example 4: `pkgs.lib.concatMapStringsSep` in `flake.nix`:**

```nix
# From flake.nix — load-images
in pkgs.lib.concatMapStringsSep "\n" load-one names + ''
  echo ""
  echo "Loaded images:"
  msb image ls
'';
```

**Example 5: `builtins.concatStringsSep` + `builtins.map` + `builtins.getAttr` in `nix/lib/vocabulary.nix`:**

```nix
# From nix/lib/vocabulary.nix — resolveFeatures
resolveFeatures = names:
  builtins.concatStringsSep "\n" (builtins.map (n: builtins.getAttr n features) names);
```

**Example 6: `builtins.path` with filter in `nix/lib/config.nix`:**

```nix
# From nix/lib/config.nix — filtered builtins.path
configDir ? builtins.path {
  path = ../../config.reference;
  filter = path: _type: baseNameOf path == "workestrate.toml";
  name = "workestrate-config-reference";
}
```

**Example 7: `lib.optionalAttrs` in `nix/lib/recipes/npm-build.nix`:**

```nix
# From nix/lib/recipes/npm-build.nix
} // lib.optionalAttrs dontNpmBuild { inherit dontNpmBuild; }
  // lib.optionalAttrs (buildPhase != null) { inherit buildPhase; }
```

## Review checklist

- [ ] Correct `lib` namespace used (not `builtins` for lib functions)
- [ ] `builtins.*` used for primops; `lib.*` used for nixpkgs library functions
- [ ] `attrByPath` / `getAttrFromPath` used for nested attrset access instead of manual `.` chains
- [ ] `filterAttrs` predicate decides on `name` first to avoid unnecessary value evaluation
- [ ] `foldl'` (strict) used instead of `foldl` to avoid stack overflow on large lists
- [ ] `concatStringsSep` / `concatMapStringsSep` used instead of manual string concatenation
- [ ] `hasPrefix` / `removePrefix` used for prefix operations, not `substring` with manual length
- [ ] `cleanSourceWith` used (not `builtins.filterSource`) when composing multiple source filters
- [ ] `optionalAttrs` used for conditional attrset members, not `if` around the entire attrset
- [ ] `listToAttrs` + `nameValuePair` used when converting lists to attrsets
- [ ] Module option priorities (`mkDefault`/`mkForce`/`mkOverride`) used correctly
- [ ] `lib` passed as function argument, not hardcoded `import <nixpkgs> {}` inside library code

## Implementation checklist

- [ ] Import `lib` from the calling scope (`{ lib, ... }:`) or from `pkgs.lib`
- [ ] Use `lib.attrsets.*` for attrset operations, `lib.lists.*` for list operations
- [ ] Prefer `lib.strings.*` over raw `builtins.*` string functions for composability
- [ ] Use `lib.trivial.pipe` for left-to-right function composition
- [ ] Use `lib.sources.cleanSourceWith` for composable source filtering
- [ ] Use `lib.modules.mkIf` / `mkMerge` for conditional module configuration
- [ ] Use `lib.types.*` for option type declarations in modules
- [ ] Use `lib.customisation.callPackageWith` / `makeOverridable` for package parameterization

## Validation hooks

- `nix-instantiate --eval` on the expression to check it evaluates without error
- `nix flake check` to validate flake outputs including `lib` exports
- `nix build .#lib.<system>` (if lib is exposed as a flake output) to verify it evaluates
- `nix-instantiate --eval --strict` to force deep evaluation and catch lazy errors
- `nix eval .#lib.x86_64-linux.config.workloadNames` to verify specific lib outputs

## Common mistakes

1. **Using `builtins.filterSource` when composing filters.** `builtins.filterSource` does not compose — it creates intermediate store copies. Use `lib.sources.cleanSourceWith` instead, which composes without intermediate copies.

2. **Using `foldl` instead of `foldl'`.** The non-strict `foldl` builds up a chain of unevaluated thunks, causing stack overflow on large lists. Always use `foldl'` (with the prime) for strict accumulator evaluation.

3. **Confusing `builtins.substring` argument order.** `builtins.substring start length s` — the second argument is length, not end index. `builtins.substring 0 2 "hello"` returns `"he"`, not `"hel"`.

4. **Using `builtins.attrNames` when `lib.attrNames` is available.** Both work, but `lib.attrNames` is the canonical nixpkgs way. More importantly, some `builtins` functions like `builtins.map` and `builtins.filter` are fine to use directly, but for consistency, prefer `lib.lists.*` in nixpkgs code.

5. **Forgetting that `attrValues` sorts by name.** `attrValues {c = 3; a = 1; b = 2;}` returns `[1 2 3]`, not `[3 1 2]`. The result is always sorted by attribute name, not insertion order (Nix attrsets are unordered).

6. **Using `if cond then { x = 1; } else {}` instead of `optionalAttrs`.** `optionalAttrs cond { x = 1; }` is more idiomatic and composable with `//`.

7. **Not passing `lib` as a function argument.** Hardcoding `import <nixpkgs> {}` inside library code breaks purity and reproducibility. Always accept `lib` (or `pkgs`) as a function argument: `{ lib, ... }: ...`.

8. **Confusing `replaceStrings` (builtins) with `replaceString` (lib).** `builtins.replaceStrings` takes lists of from/to patterns; `lib.strings.replaceString` takes a single from/to pair. They are different functions.

9. **Using `toString` on a bool.** `toString true` returns `"1"`, not `"true"`. Use `lib.trivial.boolToString` for the string `"true"`/`"false"`.

10. **Expecting `hasAttrByPath [] x` to throw.** It returns `true` for any `x` — the empty path is always valid. This is a documented law: `hasAttrByPath [] x == true`.

## Related docs

- [Nix Language Fundamentals](./language-fundamentals.md) — Nix language basics, data types, syntax
- [Nix Source Map](./source-map.md) — Provenance index for all crawled nix.dev sources

## Related skills

- `nix-usage` — Nix flake, dev shell, Rust toolchain, and Microsandbox runtime reference

## Citations


[1] [Nixpkgs manual — lib functions](https://nixos.org/manual/nixpkgs/stable/#sec-functions-library)
[2] [Nix language basics — Function libraries](https://nix.dev/tutorials/nix-language.html#function-libraries)
[3] [Nix manual — Built-in Functions](https://nix.dev/manual/nix/stable/language/builtins.html)
[4] [nixpkgs lib/default.nix](https://github.com/NixOS/nixpkgs/blob/master/lib/default.nix)
[5] [nixpkgs lib/attrsets.nix](https://github.com/NixOS/nixpkgs/blob/master/lib/attrsets.nix)
[6] [nixpkgs lib/strings.nix](https://github.com/NixOS/nixpkgs/blob/master/lib/strings.nix)
[7] [nixpkgs lib/lists.nix](https://github.com/NixOS/nixpkgs/blob/master/lib/lists.nix)
[8] [nixpkgs lib/trivial.nix](https://github.com/NixOS/nixpkgs/blob/master/lib/trivial.nix)
[9] [nixpkgs lib/sources.nix](https://github.com/NixOS/nixpkgs/blob/master/lib/sources.nix)
[10] [nixpkgs lib/modules.nix](https://github.com/NixOS/nixpkgs/blob/master/lib/modules.nix)
[11] [nixpkgs lib/types.nix](https://github.com/NixOS/nixpkgs/blob/master/lib/types.nix)
[12] [nixpkgs lib/customisation.nix](https://github.com/NixOS/nixpkgs/blob/master/lib/customisation.nix)
[13] [nixpkgs lib/debug.nix](https://github.com/NixOS/nixpkgs/blob/master/lib/debug.nix)
[14] [nixpkgs lib/path.nix](https://github.com/NixOS/nixpkgs/blob/master/lib/path.nix)
