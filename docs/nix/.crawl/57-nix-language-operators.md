---
type: Crawl Source
title: "Nix Language — Operators"
description: "Operators in the Nix expression language."
resource: https://nix.dev/manual/nix/2.34/language/operators
tags: [nix, nix-manual, language, operators]
timestamp: 2026-07-24T00:00:00Z
---

# Crawl: nix-language-operators

- seed_url: https://nix.dev/manual/nix/2.34/language/operators
- canonical_url: https://nix.dev/manual/nix/2.34/language/operators
- family: Nix Manual
- fetch: 200
- version: Nix 2.34
- feeds_docs: nix-language.md

## Content

# Operators

Name| Syntax| Associativity| Precedence  
---|---|---|---  
Attribute selection|  _attrset_ `.` _attrpath_ [ `or` _expr_ ]| none| 1  
Function application|  _func_ _expr_|  left| 2  
Arithmetic negation| `-` _number_|  none| 3  
Has attribute|  _attrset_ `?` _attrpath_|  none| 4  
List concatenation|  _list_ `++` _list_|  right| 5  
Multiplication|  _number_ `*` _number_|  left| 6  
Division|  _number_ `/` _number_|  left| 6  
Subtraction|  _number_ `-` _number_|  left| 7  
Addition|  _number_ `+` _number_|  left| 7  
String concatenation|  _string_ `+` _string_|  left| 7  
Path concatenation|  _path_ `+` _path_|  left| 7  
Path and string concatenation|  _path_ `+` _string_|  left| 7  
String and path concatenation|  _string_ `+` _path_|  left| 7  
Logical negation (`NOT`)| `!` _bool_|  none| 8  
Update|  _attrset_ `//` _attrset_|  right| 9  
Less than|  _expr_ `<` _expr_|  none| 10  
Less than or equal to|  _expr_ `<=` _expr_|  none| 10  
Greater than|  _expr_ `>` _expr_|  none| 10  
Greater than or equal to|  _expr_ `>=` _expr_|  none| 10  
Equality|  _expr_ `==` _expr_|  none| 11  
Inequality|  _expr_ `!=` _expr_|  none| 11  
Logical conjunction (`AND`)| _bool_ `&&` _bool_|  left| 12  
Logical disjunction (`OR`)| _bool_ `||` _bool_|  left| 13  
Logical implication|  _bool_ `->` _bool_|  right| 14  
Pipe operator (experimental)| _expr_ `|>` _func_|  left| 15  
Pipe operator (experimental)| _func_ `<|` _expr_|  right| 15  
  
## Attribute selection

> **Syntax**
> 
> _attrset_ `.` _attrpath_ [ `or` _expr_ ]

Select the attribute denoted by attribute path _attrpath_ from [attribute set](</manual/nix/2.34/language/types#type-attrs>) _attrset_. If the attribute doesn’t exist, return the _expr_ after `or` if provided, otherwise abort evaluation.

## Function application

> **Syntax**
> 
> _func_ _expr_

Apply the callable value _func_ to the argument _expr_. Note the absence of any visible operator symbol. A callable value is either:

  * a [user-defined function](</manual/nix/2.34/language/syntax#functions>)
  * a [built-in](</manual/nix/2.34/language/builtins>) function
  * an attribute set with a [`__functor` attribute](</manual/nix/2.34/language/syntax#attr-__functor>)

> **Warning**
> 
> [List](</manual/nix/2.34/language/types#type-list>) items are also separated by whitespace, which means that function calls in list items must be enclosed by parentheses.

## Has attribute

> **Syntax**
> 
> _attrset_ `?` _attrpath_

Test whether [attribute set](</manual/nix/2.34/language/types#type-attrs>) _attrset_ contains the attribute denoted by _attrpath_. The result is a [Boolean](</manual/nix/2.34/language/types#type-bool>) value.

See also: [`builtins.hasAttr`](</manual/nix/2.34/language/builtins#builtins-hasAttr>)

After evaluating _attrset_ and _attrpath_ , the computational complexity is O(log(_n_)) for _n_ attributes in the _attrset_

## Arithmetic

Numbers will retain their type unless mixed with other numeric types: Pure integer operations will always return integers, whereas any operation involving at least one floating point number returns a floating point number.

Evaluation of the following numeric operations throws an evaluation error:

  * Division by zero
  * Integer overflow, that is, any operation yielding a result outside of the representable range of [Nix language integers](</manual/nix/2.34/language/syntax#number-literal>)

See also Comparison and Equality.

The `+` operator is overloaded to also work on strings and paths.

## String concatenation

> **Syntax**
> 
> _string_ `+` _string_

Concatenate two [strings](</manual/nix/2.34/language/types#type-string>) and merge their [string contexts](</manual/nix/2.34/language/string-context>).

## Path concatenation

> **Syntax**
> 
> _path_ `+` _path_

Concatenate two [paths](</manual/nix/2.34/language/types#type-path>). The result is a path.

## Path and string concatenation

> **Syntax**
> 
> _path_ \+ _string_

Concatenate _[path](</manual/nix/2.34/language/types#type-path>)_ with _[string](</manual/nix/2.34/language/types#type-string>)_. The result is a path.

> **Note**
> 
> The string must not have a [string context](</manual/nix/2.34/language/string-context>) that refers to a [store path](</manual/nix/2.34/store/store-path>).

## String and path concatenation

> **Syntax**
> 
> _string_ \+ _path_

Concatenate _[string](</manual/nix/2.34/language/types#type-string>)_ with _[path](</manual/nix/2.34/language/types#type-path>)_. The result is a string.

> **Important**
> 
> The file or directory at _path_ must exist and is copied to the [store](</manual/nix/2.34/glossary#gloss-store>). The path appears in the result as the corresponding [store path](</manual/nix/2.34/store/store-path>).

## Update

> **Syntax**
> 
> _attrset1_ // _attrset2_

Update [attribute set](</manual/nix/2.34/language/types#type-attrs>) _attrset1_ with names and values from _attrset2_.

The returned attribute set will have all of the attributes in _attrset1_ and _attrset2_. If an attribute name is present in both, the attribute value from the latter is taken.

This operator is [strict](</manual/nix/2.34/language/evaluation#strictness>) in both _attrset1_ and _attrset2_. That means that both arguments are evaluated to [weak head normal form](</manual/nix/2.34/language/evaluation#values>), so the attribute sets themselves are evaluated, but their attribute values are not evaluated.

## Comparison

Comparison is

  * arithmetic for [numbers](</manual/nix/2.34/language/types#type-float>)
  * lexicographic for [strings](</manual/nix/2.34/language/types#type-string>) and [paths](</manual/nix/2.34/language/types#type-path>)
  * item-wise lexicographic for [lists](</manual/nix/2.34/language/types#type-list>): elements at the same index in both lists are compared according to their type and skipped if they are equal.

All comparison operators are implemented in terms of `<`, and the following equivalencies hold:

comparison| implementation  
---|---  
_a_ `<=` _b_| `! (` _b_ `<` _a_ `)`  
_a_ `>` _b_|  _b_ `<` _a_  
_a_ `>=` _b_| `! (` _a_ `<` _b_ `)`  
  
## Equality

  * [Attribute sets](</manual/nix/2.34/language/types#type-attrs>) are compared first by attribute names and then by items until a difference is found.
  * [Lists](</manual/nix/2.34/language/types#type-list>) are compared first by length and then by items until a difference is found.
  * Comparison of distinct [functions](</manual/nix/2.34/language/syntax#functions>) returns `false`, but identical functions may be subject to value identity optimization.
  * Numbers are type-compatible, see arithmetic operators.
  * Floating point numbers only differ up to a limited precision.

The `==` operator is [strict](</manual/nix/2.34/language/evaluation#strictness>) in both arguments; when comparing composite types ([attribute sets](</manual/nix/2.34/language/types#type-attrs>) and [lists](</manual/nix/2.34/language/types#type-list>)), it is partially strict in their contained values: they are evaluated until a difference is found. 

### Value identity optimization

Nix performs equality comparisons of nested values by pointer equality or more abstractly, _identity_. Nix semantics ideally do not assign a unique identity to values as they are created, but equality is an exception to this rule. The disputable benefit of this is that it is more efficient, and it allows cyclical structures to be compared, e.g. `let x = { x = x; }; in x == x` evaluates to `true`. However, as a consequence, it makes a function equal to itself when the comparison is made in a list or attribute set, in contradiction to a simple direct comparison.

## Logical conjunction

> **Syntax**
> 
> _bool1_ `&&` _bool2_

Logical AND. Equivalent to `if` _bool1_ `then` _bool2_ `else false`.

This operator is [strict](</manual/nix/2.34/language/evaluation#strictness>) in _bool1_ , but only evaluates _bool2_ if _bool1_ is `true`.

> **Example**
>     
>     
>     true && false
>     => false
>     
>     false && throw "never evaluated"
>     => false
>     

## Logical disjunction

> **Syntax**
> 
> _bool1_ `||` _bool2_

Logical OR. Equivalent to `if` _bool1_ `then true` `else` _bool2_.

This operator is [strict](</manual/nix/2.34/language/evaluation#strictness>) in _bool1_ , but only evaluates _bool2_ if _bool1_ is `false`.

> **Example**
>     
>     
>     true || false
>     => true
>     
>     true || throw "never evaluated"
>     => true
>     

### Precedence and disjunctive normal form

The precedence of `&&` and `||` aligns with disjunctive normal form. Without parentheses, an expression describes multiple "permissible situations" (connected by `||`), where each situation consists of multiple simultaneous conditions (connected by `&&`).

For example, `A || B && C || D && E` is parsed as `A || (B && C) || (D && E)`, describing three permissible situations: A holds, or both B and C hold, or both D and E hold.

## Logical implication

> **Syntax**
> 
> _bool1_ `->` _bool2_

Logical implication. Equivalent to `!`_bool1_ `||` _bool2_ (or `if` _bool1_ `then` _bool2_ `else true`).

This operator is [strict](</manual/nix/2.34/language/evaluation#strictness>) in _bool1_ , but only evaluates _bool2_ if _bool1_ is `true`.

> **Example**
>     
>     
>     true -> false
>     => false
>     
>     false -> throw "never evaluated"
>     => true
>     

## Pipe operators

  * _a_ `|>` _b_ is equivalent to _b_ _a_
  * _a_ `<|` _b_ is equivalent to _a_ _b_

> **Example**
>     
>     
>     nix-repl> 1 |> builtins.add 2 |> builtins.mul 3
>     9
>     
>     nix-repl> builtins.add 1 <| builtins.mul 2 <| 3
>     7
>     

> **Warning**
> 
> This syntax is part of an [experimental feature](</manual/nix/2.34/development/experimental-features>) and may change in future releases.
> 
> To use this syntax, make sure the [`pipe-operators` experimental feature](</manual/nix/2.34/development/experimental-features#xp-feature-pipe-operators>) is enabled. For example, include the following in [`nix.conf`](</manual/nix/2.34/command-ref/conf-file>):
>     
>     
>     extra-experimental-features = pipe-operators
>
