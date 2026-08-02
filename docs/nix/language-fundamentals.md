---
type: Reference
resource: https://nix.dev/tutorials/nix-language.html
title: Nix Language Fundamentals
description: The Nix expression language — syntax, types, operators, builtins, let-in, with, rec, lambdas, attribute sets, and string interpolation.
tags: [nix, language, syntax, types, builtins]
timestamp: 2026-07-24T00:00:00Z
---

# Nix language fundamentals

## Purpose
The concrete syntax and primitive semantics every Nix expression relies on: data
types, attribute sets, `let ... in`, `with`, `rec`, lambda functions, operators,
built-in functions, string interpolation, `import`, `assert`, and conditionals.
This is the bedrock for reading and writing Nixpkgs, NixOS modules, and flake
outputs.

The Nix language is "a domain-specific, purely functional, lazily evaluated,
dynamically typed programming language" designed "for conveniently creating and
composing *derivations* – precise descriptions of how contents of existing files
are used to derive new files." [nix.dev language tutorial, §Nix language basics]

## Sources used
- Crawl: `docs/nix/.crawl/10-nix-language-basics.md` — https://nix.dev/tutorials/nix-language.html
  (sections: Names and values; Attribute set; Recursive attribute set; let ... in;
  Attribute access; with; inherit; String interpolation; Indented strings; File
  system paths; Lookup paths; Functions; Calling functions; Multiple arguments;
  Attribute set argument; Default values; Additional attributes; Named attribute
  set argument; builtins; import; pkgs.lib; Impurities; Fetchers; Derivations;
  Worked examples)
- Crawl: `docs/nix/.crawl/56-nix-language-types.md` — https://nix.dev/manual/nix/2.34/language/types
  (sections: Primitives — Integer, Float, Boolean, String, Path, Null; Compound
  values — Attribute set, List; Function; External)
- Crawl: `docs/nix/.crawl/57-nix-language-operators.md` — https://nix.dev/manual/nix/2.34/language/operators
  (sections: Attribute selection; Function application; Has attribute; Arithmetic;
  String concatenation; Path concatenation; Update; Comparison; Equality; Logical
  conjunction; Logical disjunction; Logical implication; Pipe operators)
- Crawl: `docs/nix/.crawl/58-nix-language-builtins.md` — https://nix.dev/manual/nix/2.34/language/builtins
  (sections: builtins constant; import; derivation; toString; fetchurl;
  fetchTarball; fetchGit; head; tail; length; elemAt; elem; map; filter;
  foldl'; sort; attrNames; attrValues; hasAttr; getAttr; removeAttrs;
  listToAttrs; mapAttrs; genList; concatLists; concatStringsSep; replaceStrings;
  substring; stringLength; toJSON; fromJSON; fromTOML; toFile; pathExists;
  readFile; readDir; typeOf; tryEval; throw; abort; seq; deepSeq; trace;
  isInt; isFloat; isBool; isString; isPath; isAttrs; isList; isFunction;
  isNull; add; sub; mul; div; ceil; floor; lessThan; currentSystem)

## Related Nix guidance
- `docs/nix/source-map.md` — Provenance index mapping all crawled nix.dev sources
  to the Nix documentation topic files. This doc draws from crawl files 10, 56,
  57, and 58.
- The Nix manual (crawl 42) is the authoritative language reference; this doc
  distills the tutorial (crawl 10) and the manual's types/operators/builtins pages
  (crawls 56–58) into an operational reference.

## Core guidance

### Built-in types
"Every value in the Nix language has one of the following types: Integer, Float,
Boolean, String, Path, Null, Attribute set, List, Function, External."
[Nix manual, §Data Types]

- **Integer** — "a signed 64-bit integer." Non-negative integers are literals;
  negative integers use the arithmetic negation operator. `builtins.isInt` tests
  for integers. [Nix manual, §Integer]
- **Float** — "a 64-bit IEEE 754 floating-point number." `builtins.isFloat` tests
  for floats. "Numbers will retain their type unless mixed with other numeric
  types: Pure integer operations will always return integers, whereas any
  operation involving at least one floating point number returns a floating point
  number." [Nix manual, §Float; §Arithmetic]
- **Boolean** — "one of *true* or *false*." Available as `builtins.true` and
  `builtins.false` (also as top-level `true` and `false`). `builtins.isBool`
  tests for booleans. [Nix manual, §Boolean]
- **String** — "an immutable, finite-length sequence of bytes, along with a
  string context." Double-quoted: `"hello"`. `builtins.isString` tests for
  strings. [Nix manual, §String]
- **Path** — "an immutable, finite-length sequence of bytes starting with `/`,
  representing a POSIX-style, canonical file system path. Path values are distinct
  from string values, even if they contain the same sequence of bytes."
  `builtins.isPath` tests for paths. [Nix manual, §Path]
- **Null** — "There is a single value of type *null* in the Nix language."
  Available as `builtins.null` (also top-level `null`). [Nix manual, §Null]
- **Attribute set** — a collection of name-value pairs where names must be unique.
  Constructed with `{ key = value; }`. `builtins.isAttrs` tests for attrsets.
  [Nix manual, §Attribute set]
- **List** — constructed with `[ a b c ]` (space-separated, not comma-separated).
  `builtins.isList` tests for lists. [Nix manual, §List]
- **Function** — "A function always takes exactly one argument." Constructed with
  lambda syntax. `builtins.isFunction` tests for functions. [Nix manual, §Function]
- **External** — "an opaque value created by a Nix plugin." Not user-constructible.
  [Nix manual, §External]

### Attribute set `{ ... }`
"An attribute set is a collection of name-value-pairs, where names must be unique."
[nix.dev language tutorial, §Attribute set]

Assignments use a single `=` with a terminating `;`:
"Whenever you encounter an equal sign (`=`) in Nix language code: On its left is
the assigned name. On its right is the value, delimited by a semicolon (`;`)."
[nix.dev language tutorial, §Names and values]

```nix
{
  string = "hello";
  integer = 1;
  float = 3.141;
  bool = true;
  null = null;
  list = [ 1 "two" false ];
  attribute-set = {
    a = "hello";
    b = 2;
    c = 2.718;
    d = false;
  };
}
```

"If you are familiar with JSON, imagine the Nix language as *JSON with functions*."
[nix.dev language tutorial, §Attribute set]

#### Recursive attribute set `rec { ... }`
"You will sometimes see attribute sets declared with `rec` prepended. This allows
access to attributes from within the set." [nix.dev language tutorial, §Recursive
attribute set]

```nix
rec {
  one = 1;
  two = one + 1;
  three = two + 1;
}
# => { one = 1; three = 3; two = 2; }
```

"Elements in an attribute set can be declared in any order, and are ordered on
evaluation." [nix.dev language tutorial, §Recursive attribute set]

Without `rec`, referencing another attribute in the same set is an error:
```nix
{
  one = 1;
  two = one + 1;  # error: undefined variable 'one'
}
```

### `let ... in ...`
"`let` expressions allow assigning names to values for repeated use."
[nix.dev language tutorial, §let ... in ...]

```nix
let
  a = 1;
in
a + a
# => 2
```

"Names can be assigned in any order, and expressions on the right of the
assignment (`=`) can refer to other assigned names." [nix.dev language tutorial,
§let ... in ...]

```nix
let
  b = a + 1;
  a = 1;
in
a + b
# => 3
```

"This is similar to recursive attribute sets: in both, the order of assignments
does not matter, and names on the left can be used in expressions on the right of
the assignment (`=`)." [nix.dev language tutorial, §let ... in ...]

"The difference is that while a recursive attribute set evaluates to an attribute
set, any expression can follow after the `in` keyword." [nix.dev language tutorial,
§let ... in ...]

"Only expressions within the `let` expression itself can access the newly declared
names. The bindings have local scope." [nix.dev language tutorial, §let ... in ...]

### Attribute access
"Attributes in a set are accessed with a dot (`.`) and the attribute name."
[nix.dev language tutorial, §Attribute access]

```nix
let
  attrset = { x = 1; };
in
attrset.x
# => 1
```

Nested access: `attrset.a.b.c`. The dot notation also works for assignment:
`{ a.b.c = 1; }` evaluates to `{ a = { b = { c = 1; }; }; }`.

The `.` operator supports an `or` default: `_attrset_ . _attrpath_ [ or _expr_ ]`.
"If the attribute doesn't exist, return the _expr_ after `or` if provided,
otherwise abort evaluation." [Nix manual, §Attribute selection]

### `with ...; ...`
"The `with` expression allows access to attributes without repeatedly referencing
their attribute set." [nix.dev language tutorial, §with]

```nix
let
  a = {
    x = 1;
    y = 2;
    z = 3;
  };
in
with a; [ x y z ]
# => [ 1 2 3 ]
```

`with a; [ x y z ]` is equivalent to `[ a.x a.y a.z ]`.

"Attributes made available through `with` are only in scope of the expression
following the semicolon (`;`)." [nix.dev language tutorial, §with]

### `inherit ...`
"`inherit` is shorthand for assigning the value of a name from an existing scope
to the same name in a nested scope." [nix.dev language tutorial, §inherit]

```nix
let
  x = 1;
  y = 2;
in
{
  inherit x y;
}
# => { x = 1; y = 2; }
```

`inherit x y;` is equivalent to `x = x; y = y;`.

"It is also possible to `inherit` names from a specific attribute set by
enclosing its name in parentheses." [nix.dev language tutorial, §inherit (...)]

```nix
let
  a = { x = 1; y = 2; };
in
{
  inherit (a) x y;
}
# => { x = 1; y = 2; }
```

`inherit (a) x y;` is equivalent to `x = a.x; y = a.y;`. `inherit` also works
inside `let` expressions.

### String interpolation `${ ... }`
"The value of a Nix expression can be inserted into a character string with the
dollar-sign and braces (`${ }`)." [nix.dev language tutorial, §String interpolation]

```nix
let
  name = "Nix";
in
"hello ${name}"
# => "hello Nix"
```

"Only character strings or values that can be represented as a character string
are allowed." [nix.dev language tutorial, §String interpolation]

```nix
let
  x = 1;
in
"${x} + ${x} = ${x + x}"
# error: cannot coerce an integer to a string
```

"Interpolated expressions can be arbitrarily nested. (This can become hard to
read. Avoid it in practice.)" [nix.dev language tutorial, §String interpolation]

**Warning:** "You may encounter strings that use the dollar sign (`$`) before an
assigned name, but no braces (`{ }`): These are *not* interpolated strings, but
usually denote variables in a shell script." [nix.dev language tutorial, §String
interpolation]

### Indented strings
"Also known as 'multi-line strings'." Denoted by double single quotes (`'' ''`).
[nix.dev language tutorial, §Indented strings]

```nix
''
  multi
  line
  string
''
# => "multi\nline\nstring\n"
```

"Equal amounts of prepended white space are trimmed from the result." [nix.dev
language tutorial, §Indented strings]

Indented strings also support string interpolation.

### File system paths
"The Nix language offers convenience syntax for file system paths." [nix.dev
language tutorial, §File system paths]

- Absolute paths always start with a slash: `/absolute/path`
- "Paths are relative when they contain at least one slash (`/`) but do not start
  with one. They evaluate to the path relative to the file containing the
  expression." [nix.dev language tutorial, §File system paths]
  - `./relative` → `/current/directory/relative`
  - `relative/path` → `/current/directory/relative/path`
- One dot (`.`) denotes the current directory: `./.` → `/current/directory`
- Two dots (`..`) denote the parent directory: `../.` → `/current`

"Paths can be used in interpolated expressions — an impure operation." [nix.dev
language tutorial, §File system paths]

#### Lookup paths
"Also known as 'angle bracket syntax'." `<nixpkgs>` resolves to a file system path
determined by `builtins.nixPath`. "While you will encounter many such examples,
avoid lookup paths in production code, as they are impurities which are not
reproducible." [nix.dev language tutorial, §Lookup paths]

### Lambda functions
"Functions are everywhere in the Nix language and deserve particular attention."
[nix.dev language tutorial, §Functions]

"A function always takes exactly one argument. Argument and function body are
separated by a colon (`:`)." [nix.dev language tutorial, §Functions]

"Functions in the Nix language have no names. They are anonymous, and such a
function is called a *lambda*." [nix.dev language tutorial, §Functions]

```nix
x: x + 1
# => <LAMBDA>
```

#### Multiple arguments (currying)
"Nix functions take exactly one argument. Multiple arguments can be handled by
nesting functions." [nix.dev language tutorial, §Multiple arguments]

```nix
x: y: x + y
# equivalent to: x: (y: x + y)
```

```nix
let
  f = x: y: x + y;
in
f 1 2
# => 3
```

#### Attribute set argument (destructuring)
"Nix functions can be declared to require an attribute set with specific structure
as argument. This is denoted by listing the expected attribute names separated by
commas (`,`) and enclosed in braces (`{ }`)." [nix.dev language tutorial, §Attribute
set argument]

```nix
{a, b}: a + b
```

"The argument defines the exact attributes that have to be in that set. Leaving
out or passing additional attributes is an error." [nix.dev language tutorial,
§Attribute set argument]

#### Default values
"Destructured arguments can have default values for attributes. This is denoted by
separating the attribute name and its default value with a question mark (`?`)."
[nix.dev language tutorial, §Default values]

```nix
let
  f = {a, b ? 0}: a + b;
in
f { a = 1; }
# => 1
```

#### Additional attributes
"Additional attributes are allowed with an ellipsis (`...`)." [nix.dev language
tutorial, §Additional attributes]

```nix
let
  f = {a, b, ...}: a + b;
in
f { a = 1; b = 2; c = 3; }
# => 3
```

#### Named attribute set argument (`@` pattern)
"An attribute set argument can be given a name to be accessible as a whole. This
is denoted by prepending or appending the name to the attribute set argument,
separated by the at sign (`@`)." [nix.dev language tutorial, §Named attribute set
argument]

```nix
{a, b, ...}@args: a + b + args.c
# or equivalently:
args@{a, b, ...}: a + b + args.c
```

```nix
let
  f = {a, b, ...}@args: a + b + args.c;
in
f { a = 1; b = 2; c = 3; }
# => 6
```

### Calling functions
"Calling a function with an argument means writing the argument after the
function." [nix.dev language tutorial, §Calling functions]

```nix
let
  f = x: x + 1;
in f 1
# => 2
```

"Since function and argument are separated by white space, sometimes parentheses
(`( )`) are required to achieve the desired result." [nix.dev language tutorial,
§Calling functions]

```nix
(x: x + 1) 1
# => 2
```

"List elements are also separated by white space, therefore the following are
different: `[ (f a) ]` (one element, the result) vs `[ f a ]` (two elements: the
function and the value)." [nix.dev language tutorial, §Calling functions]

### Operators

| Name | Syntax | Associativity | Precedence |
|------|--------|---------------|------------|
| Attribute selection | `attrset . attrpath [ or expr ]` | none | 1 |
| Function application | `func expr` | left | 2 |
| Arithmetic negation | `- number` | none | 3 |
| Has attribute | `attrset ? attrpath` | none | 4 |
| List concatenation | `list ++ list` | right | 5 |
| Multiplication | `number * number` | left | 6 |
| Division | `number / number` | left | 6 |
| Subtraction | `number - number` | left | 7 |
| Addition | `number + number` | left | 7 |
| String concatenation | `string + string` | left | 7 |
| Path concatenation | `path + path` | left | 7 |
| Path and string concat | `path + string` | left | 7 |
| String and path concat | `string + path` | left | 7 |
| Logical negation (NOT) | `! bool` | none | 8 |
| Update | `attrset // attrset` | right | 9 |
| Less than | `expr < expr` | none | 10 |
| Less than or equal | `expr <= expr` | none | 10 |
| Greater than | `expr > expr` | none | 10 |
| Greater than or equal | `expr >= expr` | none | 10 |
| Equality | `expr == expr` | none | 11 |
| Inequality | `expr != expr` | none | 11 |
| Logical conjunction (AND) | `bool && bool` | left | 12 |
| Logical disjunction (OR) | `bool || bool` | left | 13 |
| Logical implication | `bool -> bool` | right | 14 |
| Pipe operator (experimental) | `expr |> func` | left | 15 |
| Pipe operator (experimental) | `func <| expr` | right | 15 |

[Nix manual, §Operators — full table]

#### Key operator semantics
- **Attribute selection** (`.`): "Select the attribute denoted by attribute path
  _attrpath_ from attribute set _attrset_. If the attribute doesn't exist, return
  the _expr_ after `or` if provided, otherwise abort evaluation." [Nix manual,
  §Attribute selection]
- **Has attribute** (`?`): "Test whether attribute set _attrset_ contains the
  attribute denoted by _attrpath_. The result is a Boolean value." [Nix manual,
  §Has attribute]
- **List concatenation** (`++`): right-associative; concatenates two lists.
- **String concatenation** (`+`): "Concatenate two strings and merge their string
  contexts." [Nix manual, §String concatenation] The `+` operator is overloaded to
  also work on strings and paths.
- **Update** (`//`): "Update attribute set _attrset1_ with names and values from
  _attrset2_. The returned attribute set will have all of the attributes in
  _attrset1_ and _attrset2_. If an attribute name is present in both, the attribute
  value from the latter is taken." [Nix manual, §Update]
- **Equality** (`==`): Attribute sets are compared first by attribute names then
  by items until a difference is found. Lists are compared first by length then by
  items. "Comparison of distinct functions returns `false`, but identical functions
  may be subject to value identity optimization." [Nix manual, §Equality]
- **Logical AND** (`&&`): "Equivalent to `if bool1 then bool2 else false`." Strict
  in `bool1`, only evaluates `bool2` if `bool1` is `true` (short-circuiting).
  [Nix manual, §Logical conjunction]
- **Logical OR** (`||`): "Equivalent to `if bool1 then true else bool2`." Strict
  in `bool1`, only evaluates `bool2` if `bool1` is `false` (short-circuiting).
  [Nix manual, §Logical disjunction]
- **Logical implication** (`->`): "Equivalent to `!bool1 || bool2`." Strict in
  `bool1`, only evaluates `bool2` if `bool1` is `true`. [Nix manual, §Logical
  implication]
- **Pipe operators** (`|>`, `<|`): experimental; `a |> b` is equivalent to `b a`.
  [Nix manual, §Pipe operators]

#### Arithmetic
"Numbers will retain their type unless mixed with other numeric types: Pure
integer operations will always return integers, whereas any operation involving at
least one floating point number returns a floating point number." [Nix manual,
§Arithmetic]

"Evaluation of the following numeric operations throws an evaluation error:
Division by zero; Integer overflow." [Nix manual, §Arithmetic]

#### Comparison
"Comparison is arithmetic for numbers; lexicographic for strings and paths;
item-wise lexicographic for lists: elements at the same index in both lists are
compared according to their type and skipped if they are equal." [Nix manual,
§Comparison]

"All comparison operators are implemented in terms of `<`." [Nix manual,
§Comparison]

### Conditional `if ... then ... else ...`
```nix
if 1 > 0 then "yes" else "no"
# => "yes"
```

### `assert` expressions
```nix
assert 1 + 1 == 2; "ok"
# => "ok"
```

`assert cond; expr` evaluates `expr` only if `cond` is `true`; otherwise it throws
an assertion error.

### Comments
- `#` for single-line comments.
- `/* ... */` for block comments.

```nix
{
  x = 1; # this is a comment
  /* this is a
     block comment */
  y = 2;
}
```

### Built-in functions (`builtins`)
"Nix comes with many functions that are built into the language. They are
implemented in C++ as part of the Nix language interpreter." [nix.dev language
tutorial, §builtins]

"These functions are available under the `builtins` constant." [nix.dev language
tutorial, §builtins]

"Some built-ins are also exposed directly in the global scope: `derivation`,
`derivationStrict`, `abort`, `baseNameOf`, `break`, `dirOf`, `false`, `fetchGit`,
`fetchMercurial`, `fetchTarball`, `fetchTree`, `fromTOML`, `import`, `isNull`,
`map`, `null`, `placeholder`, `removeAttrs`, `scopedImport`, `throw`, `toString`,
`true`." [Nix manual, §Built-ins]

#### `import`
"Most built-in functions are only accessible through `builtins`. A notable
exception is `import`, which is also available at the top level." [nix.dev language
tutorial, §import]

"`import` takes a path to a Nix file, reads it to evaluate the contained Nix
expression, and returns the resulting value. If the path points to a directory,
the file `default.nix` in that directory is used instead." [nix.dev language
tutorial, §import]

```nix
import ./file.nix
```

"Since a Nix file can contain any Nix expression, `import`ed functions can be
applied to arguments immediately." [nix.dev language tutorial, §import]

```nix
import ./file.nix 1  # if file.nix contains: x: x + 1
# => 2
```

"Unlike some languages, `import` is a regular function in Nix." [Nix manual,
§import]

"A Nix expression loaded by `import` must not contain any *free variables*, that
is, identifiers that are not defined in the Nix expression itself and are not
built-in. Therefore, it cannot refer to variables that are in scope at the call
site." [Nix manual, §import]

#### `derivation`
"The Nix language primitive to declare a derivation is the built-in impure function
`derivation`. It is usually wrapped by the Nixpkgs build mechanism
`stdenv.mkDerivation`." [nix.dev language tutorial, §Derivations]

"The evaluation result of `derivation` (and `mkDerivation`) is an attribute set
with a certain structure and a special property: It can be used in string
interpolation, and in that case evaluates to the Nix store path of its build
result." [nix.dev language tutorial, §Derivations]

#### `toString`
"Convert the expression _e_ to a string." [Nix manual, §toString] Accepts:
- A string (returned unmodified)
- A path (`toString /foo/bar` → `"/foo/bar"`)
- A set containing `{ __toString = self: ...; }` or `{ outPath = ...; }`
- An integer
- A list (elements joined with spaces)
- A Boolean (`false` → `""`, `true` → `"1"`)
- `null` (yields the empty string)

#### Fetchers
"The Nix language provides built-in impure functions to fetch files over the
network during evaluation: `builtins.fetchurl`, `builtins.fetchTarball`,
`builtins.fetchGit`, `builtins.fetchClosure`." [nix.dev language tutorial,
§Fetchers]

"These functions evaluate to a file system path in the Nix store." [nix.dev
language tutorial, §Fetchers]

```nix
builtins.fetchurl "https://example.com/file.tar.gz"
```

#### List operations
- `builtins.head list` — "Return the first element of a list; abort evaluation if
  the argument isn't a list or is an empty list." [Nix manual, §head]
- `builtins.tail list` — "Return the list without its first item; abort
  evaluation if the argument isn't a list or is an empty list." Warning: "This
  function should generally be avoided since it's inefficient: unlike Haskell's
  `tail`, it takes O(n) time, so recursing over a list by repeatedly calling
  `tail` takes O(n^2) time." [Nix manual, §tail]
- `builtins.length e` — "Return the length of the list _e_." [Nix manual, §length]
- `builtins.elemAt xs n` — "Return element _n_ from the list _xs_. Elements are
  counted starting from 0. A fatal error occurs if the index is out of bounds."
  [Nix manual, §elemAt]
- `builtins.elem x xs` — "Return `true` if a value equal to _x_ occurs in the list
  _xs_, and `false` otherwise." [Nix manual, §elem]
- `builtins.map f list` — "Apply the function _f_ to each element in the list
  _list_." [Nix manual, §map]
- `builtins.filter f list` — "Return a list consisting of the elements of _list_
  for which the function _f_ returns `true`." [Nix manual, §filter]
- `builtins.foldl' op nul list` — "Reduce a list by applying a binary operator,
  from left to right." [Nix manual, §foldl']
- `builtins.sort comparator list` — "Return _list_ in sorted order." Stable sort.
  [Nix manual, §sort]
- `builtins.genList generator length` — "Generate list of size _length_, with each
  element _i_ equal to the value returned by _generator_ `i`." [Nix manual,
  §genList]
- `builtins.concatLists lists` — "Concatenate a list of lists into a single list."
  [Nix manual, §concatLists]
- `list1 ++ list2` — list concatenation operator.

#### Attribute set operations
- `attrset . attrpath` — attribute selection (with optional `or` default).
- `attrset ? attrpath` — has-attribute test; returns Boolean.
- `attrset1 // attrset2` — update/merge; values from the latter win on conflict.
- `builtins.attrNames set` — "Return the names of the attributes in the set _set_
  in an alphabetically sorted list." [Nix manual, §attrNames]
- `builtins.attrValues set` — "Return the values of the attributes in the set
  _set_ in the order corresponding to the sorted attribute names." [Nix manual,
  §attrValues]
- `builtins.hasAttr s set` — "returns `true` if _set_ has an attribute named _s_,
  and `false` otherwise. This is a dynamic version of the `?` operator." [Nix
  manual, §hasAttr]
- `builtins.getAttr s set` — "returns the attribute named _s_ from _set_.
  Evaluation aborts if the attribute doesn't exist. This is a dynamic version of
  the `.` operator." [Nix manual, §getAttr]
- `removeAttrs set list` — "Remove the attributes listed in _list_ from _set_. The
  attributes don't have to exist in _set_." [Nix manual, §removeAttrs]
- `builtins.listToAttrs e` — "Construct a set from a list specifying the names and
  values of each attribute." [Nix manual, §listToAttrs]
- `builtins.mapAttrs f attrset` — "Apply function _f_ to every element of
  _attrset_." [Nix manual, §mapAttrs]

#### String operations
- `string1 + string2` — string concatenation (merges string contexts).
- `builtins.replaceStrings from to s` — "Given string _s_, replace every
  occurrence of the strings in _from_ with the corresponding string in _to_."
  [Nix manual, §replaceStrings]
- `builtins.substring start len s` — "Return the substring of _s_ from byte
  position _start_ (zero-based) up to but not including _start + len_." [Nix
  manual, §substring]
- `builtins.stringLength e` — "Return the number of bytes of the string _e_."
  [Nix manual, §stringLength]
- `builtins.concatStringsSep separator list` — concatenate a list of strings with
  a separator. [Nix manual, §concatStringsSep]

#### Type predicates
`builtins.isInt`, `builtins.isFloat`, `builtins.isBool`, `builtins.isString`,
`builtins.isPath`, `builtins.isNull`, `builtins.isAttrs`, `builtins.isList`,
`builtins.isFunction` — each returns `true` if the argument is of the
corresponding type. [Nix manual, §Built-ins]

`builtins.typeOf e` — "Return a string representing the type of the value _e_,
namely `"int"`, `"bool"`, `"string"`, `"path"`, `"null"`, `"set"`, `"list"`,
`"lambda"` or `"float"`." [Nix manual, §typeOf]

#### Serialization
- `builtins.toJSON e` — "Return a string containing a JSON representation of _e_."
  [Nix manual, §toJSON]
- `builtins.fromJSON e` — "Convert a JSON string to a Nix value." [Nix manual,
  §fromJSON]
- `builtins.fromTOML e` — "Convert a TOML string to a Nix value." [Nix manual,
  §fromTOML]
- `builtins.toFile name s` — "Store the string _s_ in a file in the Nix store and
  return its path." [Nix manual, §toFile]

#### File system
- `builtins.pathExists path` — "Return `true` if the path _path_ exists at
  evaluation time, and `false` otherwise." [Nix manual, §pathExists]
- `builtins.readFile path` — "Return the contents of the file _path_ as a string."
  [Nix manual, §readFile]
- `builtins.readDir path` — "Return the contents of the directory _path_ as a set
  mapping directory entries to the corresponding file type." [Nix manual, §readDir]

#### Error handling and debugging
- `throw s` — "Throw an error message _s_. This usually aborts Nix expression
  evaluation, but in `nix-env -qa` and other commands that try to evaluate a set
  of derivations to get information about those derivations, a derivation that
  throws an error is silently skipped (which is not the case for `abort`)." [Nix
  manual, §throw]
- `abort s` — "Abort Nix expression evaluation and print the error message _s_."
  [Nix manual, §abort]
- `builtins.tryEval e` — "Try to shallowly evaluate _e_. Return a set containing
  the attributes `success` (`true` if _e_ evaluated successfully, `false` if an
  error was thrown) and `value`." Only catches errors from `throw` or `assert`,
  not `abort` or type errors. [Nix manual, §tryEval]
- `builtins.trace e1 e2` — "Evaluate _e1_ and print its abstract syntax
  representation on standard error. Then return _e2_. This function is useful for
  debugging." [Nix manual, §trace]
- `builtins.seq e1 e2` — "Evaluate _e1_, then evaluate and return _e2_. This
  ensures that a computation is strict in the value of _e1_." [Nix manual, §seq]
- `builtins.deepSeq e1 e2` — "like `seq e1 e2`, except that _e1_ is evaluated
  _deeply_: if it's a list or set, its elements or attributes are also evaluated
  recursively." [Nix manual, §deepSeq]

### `pkgs.lib`
"The `nixpkgs` repository contains an attribute set called `lib`, which provides a
large number of useful functions. They are implemented in the Nix language, as
opposed to `builtins`, which are part of the language itself." [nix.dev language
tutorial, §pkgs.lib]

"These functions are usually accessed through `pkgs.lib`, as the Nixpkgs attribute
set is given the name `pkgs` by convention." [nix.dev language tutorial, §pkgs.lib]

```nix
let
  pkgs = import <nixpkgs> {};
in
pkgs.lib.strings.toUpper "lookup paths considered harmful"
# => "LOOKUP PATHS CONSIDERED HARMFUL"
```

"For historical reasons, some of the functions in `pkgs.lib` are equivalent to
`builtins` of the same name." [nix.dev language tutorial, §pkgs.lib]

## Practical rules
- Use `let ... in` for local bindings; use `rec { ... }` only when the attrset
  must self-reference (e.g. `mkDerivation rec { pname = ...; src = ...${pname}...; }`).
- Prefer `inherit (attrs) a b c;` over `a = attrs.a; b = attrs.b; c = attrs.c;`.
- Use `with` sparingly — it can shadow names and make scope ambiguous. Prefer
  explicit `attrs.x` access or `let inherit (attrs) x; in ...`.
- Use `//` to merge attrsets; remember the right operand wins on key conflicts.
- Use `?` or `builtins.hasAttr` to test for attribute existence; use `.attr or
  default` for safe access with a fallback.
- Use `++` for list concatenation; use `builtins.map`, `builtins.filter`,
  `builtins.foldl'` for list processing. Avoid `builtins.tail` in recursive
  patterns (O(n²)); prefer `foldl'` or `genList`.
- Use `throw` for recoverable errors (skipped by `nix-env -qa`); use `abort` for
  unrecoverable errors. Use `builtins.tryEval` to catch `throw`/`assert` errors.
- Use `builtins.trace` for debugging; remove before shipping.
- Use `toString` to coerce paths, integers, booleans, and null to strings. Use
  string interpolation `${expr}` only for string-valued expressions — integers
  cannot be interpolated directly.
- Avoid lookup paths (`<nixpkgs>`) in production code; they are impure and not
  reproducible. Pin Nixpkgs with `fetchTarball` + revision hash or flakes.
- Use `import` to load `.nix` files; remember imported files cannot access the
  caller's scope — pass values as function arguments.
- Use `assert` for invariants that should fail loudly at evaluation time.

## Review checklist
- [ ] Are `let` bindings scoped correctly (no leakage outside `in`)?
- [ ] Is `rec` used only where self-reference is needed?
- [ ] Are attribute set arguments using `...` when extra attributes should be
      allowed?
- [ ] Are default values (`?`) used for optional destructured arguments?
- [ ] Is `//` used correctly for merging (right operand wins)?
- [ ] Are lookup paths (`<nixpkgs>`) avoided in production code?
- [ ] Are string interpolations only used with string-coercible values?
- [ ] Is `builtins.tail` avoided in recursive list processing?
- [ ] Are `throw`/`abort`/`tryEval` used appropriately (throw for recoverable,
      abort for fatal, tryEval to catch throw/assert)?

## Implementation checklist
- [ ] Attribute set assignments terminate with `;`.
- [ ] List elements are space-separated (not comma-separated).
- [ ] Nested attribute paths (`a.b.c = 1;`) produce the expected nested structure.
- [ ] Curried functions (`x: y: ...`) are applied with space-separated arguments
      (`f 1 2`), not `f(1, 2)`.
- [ ] `with` scope is limited to the expression after `;`.
- [ ] `import`ed files have no free variables — all dependencies passed as
      arguments.
- [ ] Indented strings (`'' ... ''`) trim common leading whitespace correctly.
- [ ] Path literals with `./` resolve relative to the containing file.

## Validation hooks
- `nix repl` — interactive evaluation; use `:p` to force deep evaluation of lazy
  values. [nix.dev language tutorial, §Interactive evaluation]
- `nix-instantiate --eval file.nix` — evaluate a Nix file; use `--strict` to
  force deep evaluation. [nix.dev language tutorial, §Evaluating Nix files]
- `nix-instantiate --eval file.nix --strict` — force evaluation of nested
  structures for comparison.
- `nix fmt` — format Nix code (requires `nixfmt` or `alejandra`).
- `nix flake check` — validate flake outputs (if using flakes).

## Examples
```nix
# A shell environment
{ pkgs ? import <nixpkgs> {} }:
let
  message = "hello world";
in
pkgs.mkShellNoCC {
  packages = with pkgs; [ cowsay ];
  shellHook = ''
    cowsay ${message}
  '';
}
```

```nix
# A NixOS configuration fragment
{ config, pkgs, ... }: {
  imports = [ ./hardware-configuration.nix ];
  environment.systemPackages = with pkgs; [ git ];
}
```

```nix
# A package declaration (simplified)
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

```nix
# Lambda with destructuring, defaults, and @ pattern
{ pkgs, lib, ... }@args:
let
  myPackage = pkgs.somePackage;
in
{
  options = lib.mkOption { type = lib.types.bool; default = false; };
}
```

```nix
# List and attrset operations
let
  xs = [ 3 1 2 ];
  sorted = builtins.sort builtins.lessThan xs;  # [ 1 2 3 ]
  doubled = builtins.map (x: x * 2) sorted;      # [ 2 4 6 ]
  sum = builtins.foldl' (acc: x: acc + x) 0 doubled;  # 12
  attrs = { a = 1; b = 2; } // { b = 3; c = 4; };  # { a = 1; b = 3; c = 4; }
in
{
  inherit sorted doubled sum attrs;
}
```

## Common mistakes
- Using commas in lists (`[1, 2, 3]`) — Nix lists are space-separated
  (`[ 1 2 3 ]`).
- Forgetting the semicolon after attribute assignments (`x = 1` without `;`).
- Referencing other attributes in a non-`rec` attrset (`{ a = 1; b = a + 1; }`
  fails; use `rec` or `let`).
- Interpolating non-string values (`"${1}"` fails — use `"${toString 1}"`).
- Confusing `import` (the built-in function) with `imports` (a regular attribute
  name in NixOS modules).
- Using `builtins.tail` recursively — O(n²); use `foldl'` or `genList` instead.
- Expecting `with` to persist outside its scope — `with a; ...` only applies to
  the expression after `;`.
- Passing extra attributes to a destructured function argument without `...` —
  this is an error.
- Using lookup paths (`<nixpkgs>`) in production code — impure and
  non-reproducible.
- Treating `true`/`false`/`null` as keywords — they are regular names that can be
  shadowed (`let true = 1; in true` evaluates to `1`).

## Strict vs contextual guidance
- Strict: assignments terminate with `;`; lists are space-separated; functions
  take exactly one argument (currying for multiple); `let` bindings are locally
  scoped; `with` scope is limited to the expression after `;`; imported files have
  no free variables; division by zero and integer overflow are errors.
- Contextual: whether to use `rec` vs `let` (prefer `let` unless the result must
  be an attrset); whether to use `with` (use sparingly); whether to use `throw`
  vs `abort` (throw for recoverable, abort for fatal); whether to annotate
  attribute set arguments with `...` (use when callers may pass extra attributes).

## Policy decisions for individual repos
- Maximum tolerance for `with` usage (some repos ban it entirely; others allow it
  for `with lib;` in module code)?
- Required pinning strategy for Nixpkgs (flakes with `flake.lock` vs
  `fetchTarball` with revision hash vs channels)?
- Whether to use `nixfmt` or `alejandra` as the formatter?
- Whether to allow lookup paths (`<nixpkgs>`) in non-production/example code?

## Related docs
- `source-map` — provenance index for all crawled nix.dev sources
- Future: `derivations-and-builds` — `derivation`, `stdenv.mkDerivation`, build
  phases, `fetchurl`/`fetchFromGitHub`
- Future: `nixpkgs-lib` — `pkgs.lib` functions, `callPackage`, `override`,
  `overrideAttrs`, overlays
- Future: `flakes` — `flake.nix` structure, `inputs`/`outputs`, `flake.lock`
- Future: `module-system` — NixOS modules, `mkOption`, `mkIf`, `mkMerge`,
  priority system

## Related skills
- `nix-usage` — Nix flake, dev shell, Rust toolchain, and Microsandbox runtime in
  the ai-workbench

## Citations

[1] [Nix language basics — nix.dev tutorial](https://nix.dev/tutorials/nix-language.html)
[2] [Nix manual: Data Types](https://nix.dev/manual/nix/2.34/language/types)
[3] [Nix manual: Operators](https://nix.dev/manual/nix/2.34/language/operators)
[4] [Nix manual: Built-ins](https://nix.dev/manual/nix/2.34/language/builtins)
