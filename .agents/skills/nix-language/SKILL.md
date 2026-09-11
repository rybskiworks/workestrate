---
name: nix-language
description: |
  Operational guide for the Nix expression language — types, operators,
  builtins, let-in, with, rec, lambdas, attribute sets, string interpolation,
  import, and conditionals. Load when writing or reviewing Nix expressions,
  using builtins, working with attribute sets and lambdas, or debugging Nix
  evaluation errors. Does NOT cover derivations/mkDerivation (see
  nix-derivations), flake structure (see nix-flake-anatomy), or devshells
  (see nix-devshells).
---

# Nix Language Fundamentals

Distilled operational reference. Full theory and citations live in
`docs/nix/language-fundamentals.md` (which cites the nix.dev tutorial and the
Nix manual types/operators/builtins pages).

## Triggers

Load this skill when:

- Writing or reviewing Nix expressions: attribute sets, `let ... in`, `with`,
  `rec`, lambdas, string interpolation.
- Using `builtins.*` functions or selecting among `builtins` vs `pkgs.lib`.
- Debugging Nix evaluation errors (undefined variable, type coercion, infinite
  recursion, attribute missing).
- Deciding between `rec`, `let`, `with`, or explicit attribute access.
- Reviewing a PR for Nix language correctness.

## References

This skill is an index into the docs corpus. Read the relevant doc for full
detail; upstream source URLs are listed in each doc's `## Sources used`
section.

- `docs/nix/language-fundamentals.md`
  - https://nix.dev/tutorials/nix-language.html
  - https://nix.dev/manual/nix/2.34/language/types
  - https://nix.dev/manual/nix/2.34/language/operators
  - https://nix.dev/manual/nix/2.34/language/builtins

## Key Rules

### Built-in types
Every value is one of: Integer, Float, Boolean, String, Path, Null, Attribute
set, List, Function, External. `builtins.typeOf` returns `"int"`, `"bool"`,
`"string"`, `"path"`, `"null"`, `"set"`, `"list"`, `"lambda"`, or `"float"`.

- Integers are 64-bit signed; floats are 64-bit IEEE 754. Pure integer ops
  return integers; any op involving a float returns a float.
- Strings are immutable byte sequences with a string context. Double-quoted
  `"..."` or indented `'' ... ''`.
- Paths are distinct from strings even with the same bytes. `./relative`
  resolves relative to the containing file. Absolute paths start with `/`.
- `true`/`false`/`null` are regular names that can be shadowed (not keywords).

### Attribute sets
- Assignments use `=` with a terminating `;`: `{ x = 1; y = 2; }`.
- Nested paths: `{ a.b.c = 1; }` => `{ a = { b = { c = 1; }; }; }`.
- `rec { ... }` allows self-reference; without `rec`, referencing a sibling
  attribute is an error. Use `rec` only when the attrset must self-reference
  (e.g. `mkDerivation rec { pname = ...; src = ...${pname}...; }`).
- `//` merges attrsets; the right operand wins on key conflicts.
- `.attr or default` for safe access with a fallback.
- `?` / `builtins.hasAttr` to test existence.
- `inherit (attrs) a b c;` is shorthand for `a = attrs.a; b = attrs.b; ...`.

### let ... in ...
- Local bindings; order of assignments does not matter.
- Scope is local to the expression after `in`.
- Prefer `let` over `rec` unless the result must be an attrset.

### with
- `with attrs; expr` brings attrs into scope for `expr` only.
- Use sparingly — it can shadow names and make scope ambiguous. Prefer
  explicit `attrs.x` or `let inherit (attrs) x; in ...`.

### Lambdas
- A function always takes exactly one argument: `x: x + 1`.
- Multiple arguments via currying: `x: y: x + y`.
- Destructuring: `{a, b}: a + b` (exact attributes required unless `...`).
- Defaults: `{a, b ? 0}: a + b`.
- Extra attributes allowed: `{a, b, ...}: ...`.
- Named pattern (`@`): `{a, b, ...}@args: ...` or `args@{a, b, ...}: ...`.
- Application is space-separated: `f 1 2`, NOT `f(1, 2)`.
- `[ (f a) ]` is one element; `[ f a ]` is two elements.

### String interpolation
- `${expr}` inside `"..."` or `'' ... ''`.
- Only string-coercible values allowed. Integers are NOT coercible — use
  `"${toString 1}"`.
- Indented strings (`'' ... ''`) trim common leading whitespace.
- `$name` (no braces) is NOT interpolation — it is a shell variable in a
  shell script embedded in the string.

### Operators (precedence low→high)
- `.` attribute selection (with optional `or`); function application; `-`
  negation; `?` has-attr; `++` list concat; `*` `/`; `-` `+` (also string/path
  concat); `!` NOT; `//` update; `<` `<=` `>` `>=`; `==` `!=`; `&&`; `||`;
  `->` (implication); `|>` `<|` (experimental pipe).
- `&&` and `||` short-circuit. `->` is `!a || b`.
- Comparison is arithmetic for numbers, lexicographic for strings/paths,
  item-wise for lists.

### builtins
- Available under the `builtins` constant; some also in global scope
  (`import`, `derivation`, `map`, `toString`, `throw`, `abort`, `fetchGit`,
  `fetchTarball`, `fromTOML`, `removeAttrs`, `isNull`, `true`, `false`,
  `null`).
- List: `head`, `tail` (O(n) — avoid in recursion, O(n²)), `length`,
  `elemAt`, `elem`, `map`, `filter`, `foldl'`, `sort`, `genList`,
  `concatLists`.
- Attrset: `attrNames` (sorted), `attrValues`, `hasAttr`, `getAttr`,
  `removeAttrs`, `listToAttrs`, `mapAttrs`.
- String: `replaceStrings`, `substring`, `stringLength`, `concatStringsSep`.
- Serialization: `toJSON`, `fromJSON`, `fromTOML`, `toFile`.
- Filesystem: `pathExists`, `readFile`, `readDir`.
- Type predicates: `isInt`, `isFloat`, `isBool`, `isString`, `isPath`,
  `isNull`, `isAttrs`, `isList`, `isFunction`.
- Errors: `throw` (recoverable, skipped by `nix-env -qa`), `abort` (fatal),
  `tryEval` (catches `throw`/`assert`, not `abort`).
- Debug: `trace` (print AST to stderr, return second arg), `seq`, `deepSeq`.
- `pkgs.lib` is a separate Nix-language library (not builtins); access via
  `pkgs.lib.*`.

### import
- `import ./file.nix` reads and evaluates a Nix file. Directories use
  `default.nix`.
- Imported files cannot access the caller's scope — pass values as function
  arguments. No free variables allowed.
- `import` is a regular function, not a statement.

### Conditionals and assert
- `if cond then a else b` — both branches must be valid expressions.
- `assert cond; expr` — evaluates `expr` only if `cond` is `true`; otherwise
  throws an assertion error. Use for invariants.

## Quick Commands

```bash
nix eval .#attr                  # evaluate a flake attribute
nix eval --impure --expr '1 + 1' # evaluate a raw expression (impure)
nix repl                         # interactive REPL; :p forces deep eval
nix-instantiate --eval file.nix --strict  # eval a file, force deep
nix fmt                          # format (requires nixfmt or alejandra)
```

## Anti-patterns

- `with` in large scopes — shadows names, makes scope ambiguous. Prefer
  explicit `attrs.x` or `let inherit (attrs) x; in ...`.
- `rec` when avoidable — prefer `let ... in` unless the result must be an
  attrset that self-references.
- Unquoted URLs as strings — `https://example.com` is a path-like lookup, not
  a string. Use `"https://example.com"`.
- Lookup paths `<nixpkgs>` in production — impure and non-reproducible. Pin
  with `fetchTarball` + hash or flakes.
- Commas in lists (`[1, 2, 3]`) — Nix lists are space-separated `[ 1 2 3 ]`.
- Missing `;` after attribute assignments.
- Referencing sibling attributes in a non-`rec` attrset.
- Interpolating non-string values (`"${1}"`) — use `"${toString 1}"`.
- `builtins.tail` in recursive list processing — O(n²); use `foldl'` or
  `genList`.
- Passing extra attributes to a destructured arg without `...`.
- Confusing `import` (the builtin) with `imports` (a NixOS module attribute).
- Treating `true`/`false`/`null` as keywords — they are shadowable names.

## Related Skills

- nix-usage — ai-workbench Nix flake, dev shell, Rust toolchain, Microsandbox.
- nix-flake-anatomy — flake.nix structure, inputs, outputs, flake.lock.
- nix-derivations — mkDerivation, build phases, FODs, sandbox.
- nix-devshells — mkShell, shellHook, nix develop.
