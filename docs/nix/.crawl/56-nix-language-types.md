---
type: Crawl Source
title: "Nix Language — Data Types"
description: "Data types in the Nix expression language."
resource: https://nix.dev/manual/nix/2.34/language/types
tags: [nix, nix-manual, language, types]
timestamp: 2026-07-24T00:00:00Z
---

# Crawl: nix-language-types

- seed_url: https://nix.dev/manual/nix/2.34/language/types
- canonical_url: https://nix.dev/manual/nix/2.34/language/types
- family: Nix Manual
- fetch: 200
- version: Nix 2.34
- feeds_docs: nix-language.md

## Content

# Data Types

Every value in the Nix language has one of the following types:

  * Integer
  * Float
  * Boolean
  * String
  * Path
  * Null
  * Attribute set
  * List
  * Function
  * External

## Primitives

### Integer

An _integer_ in the Nix language is a signed 64-bit integer.

Non-negative integers can be expressed as [integer literals](</manual/nix/2.34/language/syntax#number-literal>). Negative integers are created with the [arithmetic negation operator](</manual/nix/2.34/language/operators#arithmetic>). The function [`builtins.isInt`](</manual/nix/2.34/language/builtins#builtins-isInt>) can be used to determine if a value is an integer.

### Float

A _float_ in the Nix language is a 64-bit [IEEE 754](<https://en.wikipedia.org/wiki/IEEE_754>) floating-point number.

Most non-negative floats can be expressed as [float literals](</manual/nix/2.34/language/syntax#number-literal>). Negative floats are created with the [arithmetic negation operator](</manual/nix/2.34/language/operators#arithmetic>). The function [`builtins.isFloat`](</manual/nix/2.34/language/builtins#builtins-isFloat>) can be used to determine if a value is a float.

### Boolean

A _boolean_ in the Nix language is one of _true_ or _false_.

These values are available as attributes of [`builtins`](</manual/nix/2.34/language/builtins#builtins-builtins>) as [`builtins.true`](</manual/nix/2.34/language/builtins#builtins-true>) and [`builtins.false`](</manual/nix/2.34/language/builtins#builtins-false>). The function [`builtins.isBool`](</manual/nix/2.34/language/builtins#builtins-isBool>) can be used to determine if a value is a boolean.

### String

A _string_ in the Nix language is an immutable, finite-length sequence of bytes, along with a [string context](</manual/nix/2.34/language/string-context>). Nix does not assume or support working natively with character encodings.

String values without string context can be expressed as [string literals](</manual/nix/2.34/language/string-literals>). The function [`builtins.isString`](</manual/nix/2.34/language/builtins#builtins-isString>) can be used to determine if a value is a string.

### Path

A _path_ in the Nix language is an immutable, finite-length sequence of bytes starting with `/`, representing a POSIX-style, canonical file system path. Path values are distinct from string values, even if they contain the same sequence of bytes. Operations that produce paths will simplify the result as the standard C function [`realpath`](<https://pubs.opengroup.org/onlinepubs/9699919799/functions/realpath.html>) would, except that there is no symbolic link resolution.

Paths are suitable for referring to local files, and are often preferable over strings.

  * Path values do not contain trailing or duplicate slashes, `.`, or `..`.
  * Relative path literals are automatically resolved relative to their [base directory](</manual/nix/2.34/glossary#gloss-base-directory>).
  * Tooling can recognize path literals and provide additional features, such as autocompletion, refactoring automation and jump-to-file.

A file is not required to exist at a given path in order for that path value to be valid, but a path that is converted to a string with [string interpolation](</manual/nix/2.34/language/string-interpolation#interpolated-expression>) or [string-and-path concatenation](</manual/nix/2.34/language/operators#string-and-path-concatenation>) must resolve to a readable file or directory which will be copied into the Nix store. For instance, evaluating `"${./foo.txt}"` will cause `foo.txt` from the same directory to be copied into the Nix store and result in the string `"/nix/store/<hash>-foo.txt"`. Operations such as [`import`](</manual/nix/2.34/language/builtins#builtins-import>) can also expect a path to resolve to a readable file or directory.

> **Note**
> 
> The Nix language assumes that all input files will remain _unchanged_ while evaluating a Nix expression. For example, assume you used a file path in an interpolated string during a `nix repl` session. Later in the same session, after having changed the file contents, evaluating the interpolated string with the file path again might not return a new [store path](</manual/nix/2.34/store/store-path>), since Nix might not re-read the file contents. Use `:r` to reset the repl as needed.

Path values can be expressed as [path literals](</manual/nix/2.34/language/syntax#path-literal>). The function [`builtins.isPath`](</manual/nix/2.34/language/builtins#builtins-isPath>) can be used to determine if a value is a path.

### Null

There is a single value of type _null_ in the Nix language.

This value is available as an attribute on the [`builtins`](</manual/nix/2.34/language/builtins#builtins-builtins>) attribute set as [`builtins.null`](</manual/nix/2.34/language/builtins#builtins-null>).

## Compound values

### Attribute set

An attribute set can be constructed with an [attribute set literal](</manual/nix/2.34/language/syntax#attrs-literal>). The function [`builtins.isAttrs`](</manual/nix/2.34/language/builtins#builtins-isAttrs>) can be used to determine if a value is an attribute set.

### List

A list can be constructed with a [list literal](</manual/nix/2.34/language/syntax#list-literal>). The function [`builtins.isList`](</manual/nix/2.34/language/builtins#builtins-isList>) can be used to determine if a value is a list.

## Function

A function can be constructed with a [function expression](</manual/nix/2.34/language/syntax#functions>). The function [`builtins.isFunction`](</manual/nix/2.34/language/builtins#builtins-isFunction>) can be used to determine if a value is a function.

## External

An _external_ value is an opaque value created by a Nix [plugin](</manual/nix/2.34/command-ref/conf-file#conf-plugin-files>). Such a value can be substituted in Nix expressions but only created and used by plugin code.
