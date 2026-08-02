---
type: Crawl Source
title: "nixpkgs — lib Functions (attrsets, strings, trivial, lists, sources)"
description: "Commonly used nixpkgs library functions."
resource: https://nixos.org/manual/nixpkgs/stable/#sec-functions-library
tags: [nix, nixpkgs-manual, lib, attrsets, strings, lists, trivial, sources]
timestamp: 2026-07-24T00:00:00Z
---

# Crawl: nixpkgs-lib-functions

- seed_url: https://nixos.org/manual/nixpkgs/stable/
- canonical_url: https://nixos.org/manual/nixpkgs/stable/
- family: nixpkgs Manual
- fetch: 200
- version: nixpkgs 26.05
- feeds_docs: nix-lib.md

## Content

### lib.attrsets: attribute set functions 

Operations on attribute sets.

#### `lib.attrsets.attrByPath`

Returns an attribute from nested attribute sets.

Nix has an [attribute selection operator `.`](<https://nixos.org/manual/nix/stable/language/operators#attribute-selection>) which is sufficient for such queries, as long as the number of attributes is static. For example:
    
    
    (x.a.b or 6) == attrByPath ["a" "b"] 6 x
    # and
    (x.${f p}."example.com" or 6) == attrByPath [ (f p) "example.com" ] 6 x
    

##### Inputs 

`attrPath`
    

A list of strings representing the attribute path to return from `set`

`default`
    

Default value if `attrPath` does not resolve to an existing value

`set`
    

The nested attribute set to select values from

##### Type 
    
    
    attrByPath :: [String] -> Any -> AttrSet -> Any
    

##### Examples 

**Example 5.`lib.attrsets.attrByPath` usage example**
    
    
    x = { a = { b = 3; }; }
    # ["a" "b"] is equivalent to x.a.b
    # 6 is a default value to return if the path does not exist in attrset
    attrByPath ["a" "b"] 6 x
    => 3
    attrByPath ["z" "z"] 6 x
    => 6
    

  

Located at [lib/attrsets.nix:88](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L88>) in `<nixpkgs>`.

#### `lib.attrsets.hasAttrByPath`

Returns if an attribute from nested attribute set exists.

Nix has a [has attribute operator `?`](<https://nixos.org/manual/nix/stable/language/operators#has-attribute>), which is sufficient for such queries, as long as the number of attributes is static. For example:
    
    
    (x?a.b) == hasAttrByPath ["a" "b"] x
    # and
    (x?${f p}."example.com") == hasAttrByPath [ (f p) "example.com" ] x
    

**Laws** :

  1. hasAttrByPath [] x == true
         

##### Inputs 

`attrPath`
    

A list of strings representing the attribute path to check from `set`

`set`
    

The nested attribute set to check

##### Type 
    
    
    hasAttrByPath :: [String] -> AttrSet -> Bool
    

##### Examples 

**Example 6.`lib.attrsets.hasAttrByPath` usage example**
    
    
    x = { a = { b = 3; }; }
    hasAttrByPath ["a" "b"] x
    => true
    hasAttrByPath ["z" "z"] x
    => false
    hasAttrByPath [] (throw "no need")
    => true
    

  

Located at [lib/attrsets.nix:156](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L156>) in `<nixpkgs>`.

#### `lib.attrsets.longestValidPathPrefix`

Returns the longest prefix of an attribute path that refers to an existing attribute in a nesting of attribute sets.

Can be used after [`mapAttrsRecursiveCond`](</manual/nixpkgs/stable/#function-library-lib.attrsets.mapAttrsRecursiveCond> "lib.attrsets.mapAttrsRecursiveCond") to apply a condition, although this will evaluate the predicate function on sibling attributes as well.

Note that the empty attribute path is valid for all values, so this function only throws an exception if any of its inputs does.

**Laws** :

  1. attrsets.longestValidPathPrefix [] x == []
         

  2. hasAttrByPath (attrsets.longestValidPathPrefix p x) x == true
         

##### Inputs 

`attrPath`
    

A list of strings representing the longest possible path that may be returned.

`v`
    

The nested attribute set to check.

##### Type 
    
    
    longestValidPathPrefix :: [String] -> AttrSet -> [String]
    

##### Examples 

**Example 7.`lib.attrsets.longestValidPathPrefix` usage example**
    
    
    x = { a = { b = 3; }; }
    attrsets.longestValidPathPrefix ["a" "b" "c"] x
    => ["a" "b"]
    attrsets.longestValidPathPrefix ["a"] x
    => ["a"]
    attrsets.longestValidPathPrefix ["z" "z"] x
    => []
    attrsets.longestValidPathPrefix ["z" "z"] (throw "no need")
    => []
    

  

Located at [lib/attrsets.nix:225](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L225>) in `<nixpkgs>`.

#### `lib.attrsets.setAttrByPath`

Create a new attribute set with `value` set at the nested attribute location specified in `attrPath`.

##### Inputs 

`attrPath`
    

A list of strings representing the attribute path to set

`value`
    

The value to set at the location described by `attrPath`

##### Type 
    
    
    setAttrByPath :: [String] -> Any -> AttrSet
    

##### Examples 

**Example 8.`lib.attrsets.setAttrByPath` usage example**
    
    
    setAttrByPath ["a" "b"] 3
    => { a = { b = 3; }; }
    

  

Located at [lib/attrsets.nix:285](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L285>) in `<nixpkgs>`.

#### `lib.attrsets.getAttrFromPath`

Like `attrByPath`, but without a default value. If it doesn’t find the path it will throw an error.

Nix has an [attribute selection operator](<https://nixos.org/manual/nix/stable/language/operators#attribute-selection>) which is sufficient for such queries, as long as the number of attributes is static. For example:
    
    
    x.a.b == getAttrFromPath ["a" "b"] x
    # and
    x.${f p}."example.com" == getAttrFromPath [ (f p) "example.com" ] x
    

##### Inputs 

`attrPath`
    

A list of strings representing the attribute path to get from `set`

`set`
    

The nested attribute set to find the value in.

##### Type 
    
    
    getAttrFromPath :: [String] -> AttrSet -> Any
    

##### Examples 

**Example 9.`lib.attrsets.getAttrFromPath` usage example**
    
    
    x = { a = { b = 3; }; }
    getAttrFromPath ["a" "b"] x
    => 3
    getAttrFromPath ["z" "z"] x
    => error: cannot find attribute `z.z'
    

  

Located at [lib/attrsets.nix:335](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L335>) in `<nixpkgs>`.

#### `lib.attrsets.concatMapAttrs`

Map each attribute in the given set and merge them into a new attribute set.

##### Inputs 

`f`
    

1\. Function argument

`v`
    

2\. Function argument

##### Type 
    
    
    concatMapAttrs :: (String -> Any -> AttrSet) -> AttrSet -> AttrSet
    

##### Examples 

**Example 10.`lib.attrsets.concatMapAttrs` usage example**
    
    
    concatMapAttrs
      (name: value: {
        ${name} = value;
        ${name + value} = value;
      })
      { x = "a"; y = "b"; }
    => { x = "a"; xa = "a"; y = "b"; yb = "b"; }
    

  

Located at [lib/attrsets.nix:374](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L374>) in `<nixpkgs>`.

#### `lib.attrsets.updateManyAttrsByPath`

Update or set specific paths of an attribute set.

Takes a list of updates to apply and an attribute set to apply them to, and returns the attribute set with the updates applied. Updates are represented as `{ path = ...; update = ...; }` values, where `path` is a list of strings representing the attribute path that should be updated, and `update` is a function that takes the old value at that attribute path as an argument and returns the new value it should be.

Properties:

  * Updates to deeper attribute paths are applied before updates to more shallow attribute paths

  * Multiple updates to the same attribute path are applied in the order they appear in the update list

  * If any but the last `path` element leads into a value that is not an attribute set, an error is thrown

  * If there is an update for an attribute path that doesn’t exist, accessing the argument in the update function causes an error, but intermediate attribute sets are implicitly created as needed

##### Type 
    
    
    updateManyAttrsByPath :: [{ path :: [String]; update :: (Any -> Any); }] -> AttrSet -> AttrSet
    

##### Examples 

**Example 11.`lib.attrsets.updateManyAttrsByPath` usage example**
    
    
    updateManyAttrsByPath [
      {
        path = [ "a" "b" ];
        update = old: { d = old.c; };
      }
      {
        path = [ "a" "b" "c" ];
        update = old: old + 1;
      }
      {
        path = [ "x" "y" ];
        update = old: "xy";
      }
    ] { a.b.c = 0; }
    => { a = { b = { d = 1; }; }; x = { y = "xy"; }; }
    

  

Located at [lib/attrsets.nix:436](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L436>) in `<nixpkgs>`.

#### `lib.attrsets.attrVals`

Returns the specified attributes from a set.

##### Inputs 

`nameList`
    

The list of attributes to fetch from `set`. Each attribute name must exist on the attribute set

`set`
    

The set to get attribute values from

##### Type 
    
    
    attrVals :: [String] -> { [String] :: a } -> [a]
    

##### Examples 

**Example 12.`lib.attrsets.attrVals` usage example**
    
    
    attrVals ["a" "b" "c"] as
    => [as.a as.b as.c]
    

  

Located at [lib/attrsets.nix:535](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L535>) in `<nixpkgs>`.

#### `lib.attrsets.attrValues`

Returns the values of all attributes in the given set, sorted by attribute name.

##### Type 
    
    
    attrValues :: { [String] :: a } -> [a]
    

##### Examples 

**Example 13.`lib.attrsets.attrValues` usage example**
    
    
    attrValues {c = 3; a = 1; b = 2;}
    => [1 2 3]
    

  

Located at [lib/attrsets.nix:558](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L558>) in `<nixpkgs>`.

#### `lib.attrsets.getAttrs`

Given a set of attribute names, return the set of the corresponding attributes from the given set.

##### Inputs 

`names`
    

A list of attribute names to get out of `set`

`set`
    

The set to get the named attributes from

##### Type 
    
    
    getAttrs :: [String] -> { [String] :: a } -> { [String] :: a }
    

##### Examples 

**Example 14.`lib.attrsets.getAttrs` usage example**
    
    
    getAttrs [ "a" "b" ] { a = 1; b = 2; c = 3; }
    => { a = 1; b = 2; }
    

  

Located at [lib/attrsets.nix:591](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L591>) in `<nixpkgs>`.

#### `lib.attrsets.catAttrs`

Collect each attribute named `attr` from a list of attribute sets. Sets that don’t contain the named attribute are ignored.

##### Inputs 

`attr`
    

The attribute name to get out of the sets.

`list`
    

The list of attribute sets to go through

##### Type 
    
    
    catAttrs :: String -> [{ [String] :: a }] -> [a]
    

##### Examples 

**Example 15.`lib.attrsets.catAttrs` usage example**
    
    
    catAttrs "a" [{a = 1;} {b = 0;} {a = 2;}]
    => [1 2]
    

  

Located at [lib/attrsets.nix:624](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L624>) in `<nixpkgs>`.

#### `lib.attrsets.filterAttrs`

Filter an attribute set by removing all attributes for which the given predicate return false.

##### Inputs 

`pred`
    

Predicate taking an attribute name and an attribute value, which returns `true` to include the attribute, or `false` to exclude the attribute.

If possible, decide on `name` first and on `value` only if necessary. This avoids evaluating the value if the name is already enough, making it possible, potentially, to have the argument reference the return value. (Depending on context, that could still be considered a self reference by users; a common pattern in Nix.)

`filterAttrs` is occasionally the cause of infinite recursion in configuration systems that allow self-references. To support the widest range of user-provided logic, perform the `filterAttrs` call as late as possible. Typically that’s right before using it in a derivation, as opposed to an implicit conversion whose result is accessible to the user’s expressions.

`set`
    

The attribute set to filter

##### Type 
    
    
    filterAttrs :: (String -> a -> Bool) -> { [String] :: a } -> { [String] :: a }
    

##### Examples 

**Example 16.`lib.attrsets.filterAttrs` usage example**
    
    
    filterAttrs (n: v: n == "foo") { foo = 1; bar = 2; }
    => { foo = 1; }
    

  

Located at [lib/attrsets.nix:667](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L667>) in `<nixpkgs>`.

#### `lib.attrsets.filterAttrsRecursive`

Filter an attribute set recursively by removing all attributes for which the given predicate return false.

##### Inputs 

`pred`
    

Predicate taking an attribute name and an attribute value, which returns `true` to include the attribute, or `false` to exclude the attribute.

`set`
    

The attribute set to filter

##### Type 
    
    
    filterAttrsRecursive :: (String -> Any -> Bool) -> AttrSet -> AttrSet
    

##### Examples 

**Example 17.`lib.attrsets.filterAttrsRecursive` usage example**
    
    
    filterAttrsRecursive (n: v: v != null) { foo = { bar = null; }; }
    => { foo = {}; }
    

  

Located at [lib/attrsets.nix:700](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/attrsets.nix#L700>) in `<nixpkgs>`.

#### `lib.attrsets.foldlAttrs`

Like [`lib.lists.foldl'`](</manual/nixpkgs/stable/#function-library-lib.lists.foldl-prime> "lib.lists.foldl'") but for attribute sets. Iterates over every name-value pair in the given attribute set. The result of the callback function is often called `acc` for accumulator. It is passed between callbacks from left to right and the final `acc` is the return value of `foldlAttrs`.

### lib.strings: string manipulation functions 

String manipulation functions.

#### `lib.strings.join`

Concatenates a list of strings with a separator between each element.

##### Inputs 

`sep`
    

Separator to add between elements

`list`
    

List of strings that will be joined

##### Type 
    
    
    join :: String -> [String] -> String
    

##### Examples 

**Example 54.`lib.strings.join` usage example**
    
    
    join ", " ["foo" "bar"]
    => "foo, bar"
    

  

Located at [lib/strings.nix:73](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L73>) in `<nixpkgs>`.

#### `lib.strings.concatStrings`

Concatenate a list of strings.

##### Type 
    
    
    concatStrings :: [String] -> String
    

##### Examples 

**Example 55.`lib.strings.concatStrings` usage example**
    
    
    concatStrings ["foo" "bar"]
    => "foobar"
    

  

Located at [lib/strings.nix:95](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L95>) in `<nixpkgs>`.

#### `lib.strings.concatMapStrings`

Map a function over a list and concatenate the resulting strings.

##### Inputs 

`f`
    

1\. Function argument

`list`
    

2\. Function argument

##### Type 
    
    
    concatMapStrings :: (a -> String) -> [a] -> String
    

##### Examples 

**Example 56.`lib.strings.concatMapStrings` usage example**
    
    
    concatMapStrings (x: "a" + x) ["foo" "bar"]
    => "afooabar"
    

  

Located at [lib/strings.nix:125](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L125>) in `<nixpkgs>`.

#### `lib.strings.concatImapStrings`

Like `concatMapStrings` except that the function `f` also gets the position as a parameter.

##### Inputs 

`f`
    

1\. Function argument

`list`
    

2\. Function argument

##### Type 
    
    
    concatImapStrings :: (Int -> a -> String) -> [a] -> String
    

##### Examples 

**Example 57.`lib.strings.concatImapStrings` usage example**
    
    
    concatImapStrings (pos: x: "${toString pos}-${x}") ["foo" "bar"]
    => "1-foo2-bar"
    

  

Located at [lib/strings.nix:156](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L156>) in `<nixpkgs>`.

#### `lib.strings.intersperse`

Place an element between each element of a list

##### Inputs 

`separator`
    

Separator to add between elements

`list`
    

Input list

##### Type 
    
    
    intersperse :: a -> [a] -> [a]
    

##### Examples 

**Example 58.`lib.strings.intersperse` usage example**
    
    
    intersperse "/" ["usr" "local" "bin"]
    => ["usr" "/" "local" "/" "bin"].
    

  

Located at [lib/strings.nix:186](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L186>) in `<nixpkgs>`.

#### `lib.strings.concatStringsSep`

Concatenate a list of strings with a separator between each element

##### Inputs 

`sep`
    

Separator to add between elements

`list`
    

List of input strings

##### Type 
    
    
    concatStringsSep :: String -> [String] -> String
    

##### Examples 

**Example 59.`lib.strings.concatStringsSep` usage example**
    
    
    concatStringsSep "/" ["usr" "local" "bin"]
    => "usr/local/bin"
    

  

Located at [lib/strings.nix:226](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L226>) in `<nixpkgs>`.

#### `lib.strings.concatMapStringsSep`

Maps a function over a list of strings and then concatenates the result with the specified separator interspersed between elements.

##### Inputs 

`sep`
    

Separator to add between elements

`f`
    

Function to map over the list

`list`
    

List of input strings

##### Type 
    
    
    concatMapStringsSep :: String -> (a -> String) -> [a] -> String
    

##### Examples 

**Example 60.`lib.strings.concatMapStringsSep` usage example**
    
    
    concatMapStringsSep "-" (x: toUpper x)  ["foo" "bar" "baz"]
    => "FOO-BAR-BAZ"
    

  

Located at [lib/strings.nix:261](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L261>) in `<nixpkgs>`.

#### `lib.strings.concatImapStringsSep`

Same as `concatMapStringsSep`, but the mapping function additionally receives the position of its argument.

##### Inputs 

`sep`
    

Separator to add between elements

`f`
    

Function that receives elements and their positions

`list`
    

List of input strings

##### Type 
    
    
    concatIMapStringsSep :: String -> (Int -> a -> String) -> [a] -> String
    

##### Examples 

**Example 61.`lib.strings.concatImapStringsSep` usage example**
    
    
    concatImapStringsSep "-" (pos: x: toString (x / pos)) [ 6 6 6 ]
    => "6-3-2"
    

  

Located at [lib/strings.nix:297](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L297>) in `<nixpkgs>`.

#### `lib.strings.concatMapAttrsStringSep`

Like [`concatMapStringsSep`](</manual/nixpkgs/stable/#function-library-lib.strings.concatMapStringsSep> "lib.strings.concatMapStringsSep") but takes an attribute set instead of a list.

##### Inputs 

`sep`
    

Separator to add between item strings

`f`
    

Function that takes each key and value and return a string

`attrs`
    

Attribute set to map from

##### Type 
    
    
    concatMapAttrsStringSep :: String -> (String -> a -> String) -> { [String] :: a } -> String
    

##### Examples 

**Example 62.`lib.strings.concatMapAttrsStringSep` usage example**
    
    
    concatMapAttrsStringSep "\n" (name: value: "${name}: foo-${value}") { a = "0.1.0"; b = "0.2.0"; }
    => "a: foo-0.1.0\nb: foo-0.2.0"
    

  

Located at [lib/strings.nix:334](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L334>) in `<nixpkgs>`.

#### `lib.strings.concatLines`

Concatenate a list of strings, adding a newline at the end of each one.

##### Inputs 

`list`
    

List of strings. Any element that is not a string will be implicitly converted to a string.

##### Type 
    
    
    concatLines :: [String] -> String
    

##### Examples 

**Example 63.`lib.strings.concatLines` usage example**
    
    
    concatLines [ "foo" "bar" ]
    => "foo\nbar\n"
    

  

Located at [lib/strings.nix:363](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L363>) in `<nixpkgs>`.

#### `lib.strings.replaceString`

Given string `s`, replace every occurrence of the string `from` with the string `to`.

##### Inputs 

`from`
    

The string to be replaced

`to`
    

The string to replace with

`s`
    

The original string where replacements will be made

##### Type 
    
    
    replaceString :: String -> String -> String -> String
    

##### Examples 

**Example 64.`lib.strings.replaceString` usage example**
    
    
    replaceString "world" "Nix" "Hello, world!"
    => "Hello, Nix!"
    replaceString "." "_" "v1.2.3"
    => "v1_2_3"
    

  

Located at [lib/strings.nix:398](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L398>) in `<nixpkgs>`.

#### `lib.strings.replicate`

Repeat a string `n` times, and concatenate the parts into a new string.

##### Inputs 

`n`
    

1\. Function argument

`s`
    

2\. Function argument

##### Type 
    
    
    replicate :: Int -> String -> String
    

##### Examples 

**Example 65.`lib.strings.replicate` usage example**
    
    
    replicate 3 "v"
    => "vvv"
    replicate 5 "hello"
    => "hellohellohellohellohello"
    

  

Located at [lib/strings.nix:431](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L431>) in `<nixpkgs>`.

#### `lib.strings.trim`

Remove leading and trailing whitespace from a string `s`.

Whitespace is defined as any of the following characters: " ", “\t” “\r” “\n”

##### Inputs 

`s`
    

The string to trim

##### Type 
    
    
    trim :: String -> String
    

##### Examples 

**Example 66.`lib.strings.trim` usage example**
    
    
    trim "   hello, world!   "
    => "hello, world!"
    

  

Located at [lib/strings.nix:461](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L461>) in `<nixpkgs>`.

#### `lib.strings.trimWith`

Remove leading and/or trailing whitespace from a string `s`.

To remove both leading and trailing whitespace, you can also use [`trim`](</manual/nixpkgs/stable/#function-library-lib.strings.trim> "lib.strings.trim")

Whitespace is defined as any of the following characters: " ", “\t” “\r” “\n”

##### Inputs 

`config` (Attribute set)
    

`start`
    

Whether to trim leading whitespace (`false` by default)

    

`end`
    

Whether to trim trailing whitespace (`false` by default)

`s`
    

The string to trim

##### Type 
    
    
    trimWith :: { start :: Bool; end :: Bool; } -> String -> String
    

##### Examples 

**Example 67.`lib.strings.trimWith` usage example**
    
    
    trimWith { start = true; } "   hello, world!   "}
    => "hello, world!   "
    
    trimWith { end = true; } "   hello, world!   "}
    => "   hello, world!"
    

  

Located at [lib/strings.nix:505](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L505>) in `<nixpkgs>`.

#### `lib.strings.makeSearchPath`

Construct a Unix-style, colon-separated search path consisting of the given `subDir` appended to each of the given paths.

##### Inputs 

`subDir`
    

Directory name to append

`paths`
    

List of base paths

##### Type 
    
    
    makeSearchPath :: String -> [String] -> String
    

##### Examples 

**Example 68.`lib.strings.makeSearchPath` usage example**
    
    
    makeSearchPath "bin" ["/root" "/usr" "/usr/local"]
    => "/root/bin:/usr/bin:/usr/local/bin"
    makeSearchPath "bin" [""]
    => "/bin"
    

  

Located at [lib/strings.nix:566](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L566>) in `<nixpkgs>`.

#### `lib.strings.makeSearchPathOutput`

Construct a Unix-style search path by appending the given `subDir` to the specified `output` of each of the packages.

If no output by the given name is found, fallback to `.out` and then to the default.

##### Inputs 

`output`
    

Package output to use

`subDir`
    

Directory name to append

`pkgs`
    

List of packages

##### Type 
    
    
    makeSearchPathOutput :: String -> String -> [Derivation] -> String
    

##### Examples 

**Example 69.`lib.strings.makeSearchPathOutput` usage example**
    
    
    makeSearchPathOutput "dev" "bin" [ pkgs.openssl pkgs.zlib ]
    => "/nix/store/9rz8gxhzf8sw4kf2j2f1grr49w8zx5vj-openssl-1.0.1r-dev/bin:/nix/store/wwh7mhwh269sfjkm6k5665b5kgp7jrk2-zlib-1.2.8/bin"
    

  

Located at [lib/strings.nix:604](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L604>) in `<nixpkgs>`.

#### `lib.strings.makeLibraryPath`

Construct a library search path (such as RPATH) containing the libraries for a set of packages

##### Inputs 

`packages`
    

List of packages

##### Type 
    
    
    makeLibraryPath :: [Derivation] -> String
    

##### Examples 

**Example 70.`lib.strings.makeLibraryPath` usage example**
    
    
    makeLibraryPath [ "/usr" "/usr/local" ]
    => "/usr/lib:/usr/local/lib"
    pkgs = import <nixpkgs> { }
    makeLibraryPath [ pkgs.openssl pkgs.zlib ]
    => "/nix/store/9rz8gxhzf8sw4kf2j2f1grr49w8zx5vj-openssl-1.0.1r/lib:/nix/store/wwh7mhwh269sfjkm6k5665b5kgp7jrk2-zlib-1.2.8/lib"
    

  

Located at [lib/strings.nix:637](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L637>) in `<nixpkgs>`.

#### `lib.strings.makeIncludePath`

Construct an include search path (such as C_INCLUDE_PATH) containing the header files for a set of packages or paths.

##### Inputs 

`packages`
    

List of packages

##### Type 
    
    
    makeIncludePath :: [Derivation] -> String
    

##### Examples 

**Example 71.`lib.strings.makeIncludePath` usage example**
    
    
    makeIncludePath [ "/usr" "/usr/local" ]
    => "/usr/include:/usr/local/include"
    pkgs = import <nixpkgs> { }
    makeIncludePath [ pkgs.openssl pkgs.zlib ]
    => "/nix/store/9rz8gxhzf8sw4kf2j2f1grr49w8zx5vj-openssl-1.0.1r-dev/include:/nix/store/wwh7mhwh269sfjkm6k5665b5kgp7jrk2-zlib-1.2.8-dev/include"
    

  

Located at [lib/strings.nix:668](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L668>) in `<nixpkgs>`.

#### `lib.strings.makeBinPath`

Construct a binary search path (such as $PATH) containing the binaries for a set of packages.

##### Inputs 

`packages`
    

List of packages

##### Type 
    
    
    makeBinPath :: [Derivation] -> String
    

##### Examples 

**Example 72.`lib.strings.makeBinPath` usage example**
    
    
    makeBinPath ["/root" "/usr" "/usr/local"]
    => "/root/bin:/usr/bin:/usr/local/bin"
    

  

Located at [lib/strings.nix:696](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L696>) in `<nixpkgs>`.

#### `lib.strings.normalizePath`

Normalize path, removing extraneous /s

##### Inputs 

`s`
    

1\. Function argument

##### Type 
    
    
    normalizePath :: String -> String
    

##### Examples 

**Example 73.`lib.strings.normalizePath` usage example**
    
    
    normalizePath "/a//b///c/"
    => "/a/b/c/"
    

  

Located at [lib/strings.nix:723](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L723>) in `<nixpkgs>`.

#### `lib.strings.optionalString`

Depending on the boolean `cond`, return either the given string or the empty string. Useful to concatenate against a bigger string.

##### Inputs 

`cond`
    

Condition

`string`
    

String to return if condition is true

##### Type 
    
    
    optionalString :: Bool -> String -> String
    

##### Examples 

**Example 74.`lib.strings.optionalString` usage example**
    
    
    optionalString true "some-string"
    => "some-string"
    optionalString false "some-string"
    => ""
    

  

Located at [lib/strings.nix:766](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L766>) in `<nixpkgs>`.

#### `lib.strings.hasPrefix`

Determine whether a string has given prefix.

##### Inputs 

`pref`
    

Prefix to check for

`str`
    

Input string

##### Type 
    
    
    hasPrefix :: String -> String -> Bool
    

##### Examples 

**Example 75.`lib.strings.hasPrefix` usage example**
    
    
    hasPrefix "foo" "foobar"
    => true
    hasPrefix "foo" "barfoo"
    => false
    

  

Located at [lib/strings.nix:798](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L798>) in `<nixpkgs>`.

#### `lib.strings.hasSuffix`

Determine whether a string has given suffix.

##### Inputs 

`suffix`
    

Suffix to check for

`content`
    

Input string

##### Type 
    
    
    hasSuffix :: String -> String -> Bool
    

##### Examples 

**Example 76.`lib.strings.hasSuffix` usage example**
    
    
    hasSuffix "foo" "foobar"
    => false
    hasSuffix "foo" "barfoo"
    => true
    

  

Located at [lib/strings.nix:841](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L841>) in `<nixpkgs>`.

#### `lib.strings.hasInfix`

Determine whether a string contains the given infix

##### Inputs 

`infix`
    

1\. Function argument

`content`
    

2\. Function argument

##### Type 
    
    
    hasInfix :: String -> String -> Bool
    

##### Examples 

**Example 77.`lib.strings.hasInfix` usage example**
    
    
    hasInfix "bc" "abcd"
    => true
    hasInfix "ab" "abcd"
    => true
    hasInfix "cd" "abcd"
    => true
    hasInfix "foo" "abcd"
    => false
    

  

Located at [lib/strings.nix:891](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L891>) in `<nixpkgs>`.

#### `lib.strings.stringToCharacters`

Convert a string `s` to a list of characters (i.e. singleton strings). This allows you to, e.g., map a function over each character. However, note that this will likely be horribly inefficient; Nix is not a general purpose programming language. Complex string manipulations should, if appropriate, be done in a derivation. Also note that Nix treats strings as a list of bytes and thus doesn’t handle unicode.

##### Inputs 

`s`
    

1\. Function argument

##### Type 
    
    
    stringToCharacters :: String -> [String]
    

##### Examples 

**Example 78.`lib.strings.stringToCharacters` usage example**
    
    
    stringToCharacters ""
    => [ ]
    stringToCharacters "abc"
    => [ "a" "b" "c" ]
    stringToCharacters "🦄"
    => [ "�" "�" "�" "�" ]
    

  

Located at [lib/strings.nix:938](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L938>) in `<nixpkgs>`.

#### `lib.strings.stringAsChars`

Manipulate a string character by character and replace them by strings before concatenating the results.

##### Inputs 

`f`
    

Function to map over each individual character

`s`
    

Input string

##### Type 
    
    
    stringAsChars :: (String -> String) -> String -> String
    

##### Examples 

**Example 79.`lib.strings.stringAsChars` usage example**
    
    
    stringAsChars (x: if x == "a" then "i" else x) "nax"
    => "nix"
    

  

Located at [lib/strings.nix:969](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L969>) in `<nixpkgs>`.

#### `lib.strings.charToInt`

Convert char to ascii value, must be in printable range

##### Inputs 

`c`
    

1\. Function argument

##### Type 
    
    
    charToInt :: String -> Int
    

##### Examples 

**Example 80.`lib.strings.charToInt` usage example**
    
    
    charToInt "A"
    => 65
    charToInt "("
    => 40
    

  

Located at [lib/strings.nix:1003](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1003>) in `<nixpkgs>`.

#### `lib.strings.escape`

Escape occurrence of the elements of `list` in `string` by prefixing it with a backslash.

##### Inputs 

`list`
    

1\. Function argument

`string`
    

2\. Function argument

##### Type 
    
    
    escape :: [String] -> String -> String
    

##### Examples 

**Example 81.`lib.strings.escape` usage example**
    
    
    escape ["(" ")"] "(foo)"
    => "\\(foo\\)"
    

  

Located at [lib/strings.nix:1034](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1034>) in `<nixpkgs>`.

#### `lib.strings.escapeC`

Escape occurrence of the element of `list` in `string` by converting to its ASCII value and prefixing it with \x. Only works for printable ascii characters.

##### Inputs 

`list`
    

1\. Function argument

`string`
    

2\. Function argument

##### Type 
    
    
    escapeC :: [String] -> String -> String
    

##### Examples 

**Example 82.`lib.strings.escapeC` usage example**
    
    
    escapeC [" "] "foo bar"
    => "foo\\x20bar"
    

  

Located at [lib/strings.nix:1066](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1066>) in `<nixpkgs>`.

#### `lib.strings.escapeURL`

Escape the `string` so it can be safely placed inside a URL query.

##### Inputs 

`string`
    

1\. Function argument

##### Type 
    
    
    escapeURL :: String -> String
    

##### Examples 

**Example 83.`lib.strings.escapeURL` usage example**
    
    
    escapeURL "foo/bar baz"
    => "foo%2Fbar%20baz"
    

  

Located at [lib/strings.nix:1098](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1098>) in `<nixpkgs>`.

#### `lib.strings.escapeShellArg`

Quote `string` to be used safely within the Bourne shell if it has any special characters.

##### Inputs 

`string`
    

1\. Function argument

##### Type 
    
    
    escapeShellArg :: String -> String
    

##### Examples 

**Example 84.`lib.strings.escapeShellArg` usage example**
    
    
    escapeShellArg "esc'ape\nme"
    => "'esc'\\''ape\nme'"
    

  

Located at [lib/strings.nix:1200](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1200>) in `<nixpkgs>`.

#### `lib.strings.escapeShellArgs`

Quote all arguments that have special characters to be safely passed to the Bourne shell.

##### Inputs 

`args`
    

1\. Function argument

##### Type 
    
    
    escapeShellArgs :: [String] -> String
    

##### Examples 

**Example 85.`lib.strings.escapeShellArgs` usage example**
    
    
    escapeShellArgs ["one" "two three" "four'five"]
    => "one 'two three' 'four'\\''five'"
    

  

Located at [lib/strings.nix:1236](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1236>) in `<nixpkgs>`.

#### `lib.strings.isValidPosixName`

Test whether the given `name` is a valid POSIX shell variable name.

##### Inputs 

`name`
    

1\. Function argument

##### Type 
    
    
    isValidPosixName :: String -> Bool
    

##### Examples 

**Example 86.`lib.strings.isValidPosixName` usage example**
    
    
    isValidPosixName "foo_bar000"
    => true
    isValidPosixName "0-bad.jpg"
    => false
    

  

Located at [lib/strings.nix:1265](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1265>) in `<nixpkgs>`.

#### `lib.strings.toShellVar`

Translate a Nix value into a shell variable declaration, with proper escaping.

The value can be a string (mapped to a regular variable), a list of strings (mapped to a Bash-style array) or an attribute set of strings (mapped to a Bash-style associative array). Note that “string” includes string-coercible values like paths or derivations.

Strings are translated into POSIX sh-compatible code; lists and attribute sets assume a shell that understands Bash syntax (e.g. Bash or ZSH).

##### Inputs 

`name`
    

1\. Function argument

`value`
    

2\. Function argument

##### Type 
    
    
    toShellVar :: String -> (String | [String] | { [String] :: String }) -> String
    

##### Examples 

**Example 87.`lib.strings.toShellVar` usage example**
    
    
    ''
      ${toShellVar "foo" "some string"}
      [[ "$foo" == "some string" ]]
    ''
    

  

Located at [lib/strings.nix:1305](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1305>) in `<nixpkgs>`.

#### `lib.strings.toShellVars`

Translate an attribute set `vars` into corresponding shell variable declarations using `toShellVar`.

##### Inputs 

`vars`
    

1\. Function argument

##### Type 
    
    
    toShellVars :: {
      [String] :: String | [String] | { [String] :: String };
    } -> String
    

##### Examples 

**Example 88.`lib.strings.toShellVars` usage example**
    
    
    let
      foo = "value";
      bar = foo;
    in ''
      ${toShellVars { inherit foo bar; }}
      [[ "$foo" == "$bar" ]]
    ''
    

  

Located at [lib/strings.nix:1351](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1351>) in `<nixpkgs>`.

#### `lib.strings.escapeNixString`

Turn a string `s` into a Nix expression representing that string

##### Inputs 

`s`
    

1\. Function argument

##### Type 
    
    
    escapeNixString :: String -> String
    

##### Examples 

**Example 89.`lib.strings.escapeNixString` usage example**
    
    
    escapeNixString "hello\${}\n"
    => "\"hello\\\${}\\n\""
    

  

Located at [lib/strings.nix:1378](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1378>) in `<nixpkgs>`.

#### `lib.strings.escapeRegex`

Turn a string `s` into an exact regular expression

##### Inputs 

`s`
    

1\. Function argument

##### Type 
    
    
    escapeRegex :: String -> String
    

##### Examples 

**Example 90.`lib.strings.escapeRegex` usage example**
    
    
    escapeRegex "[^a-z]*"
    => "\\[\\^a-z]\\*"
    

  

Located at [lib/strings.nix:1405](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1405>) in `<nixpkgs>`.

#### `lib.strings.escapeNixIdentifier`

Quotes a string `s` if it can’t be used as an identifier directly.

##### Inputs 

`s`
    

1\. Function argument

##### Type 
    
    
    escapeNixIdentifier :: String -> String
    

##### Examples 

**Example 91.`lib.strings.escapeNixIdentifier` usage example**
    
    
    escapeNixIdentifier "hello"
    => "hello"
    escapeNixIdentifier "0abc"
    => "\"0abc\""
    

  

Located at [lib/strings.nix:1434](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1434>) in `<nixpkgs>`.

#### `lib.strings.escapeXML`

Escapes a string `s` such that it is safe to include verbatim in an XML document.

##### Inputs 

`s`
    

1\. Function argument

##### Type 
    
    
    escapeXML :: String -> String
    

##### Examples 

**Example 92.`lib.strings.escapeXML` usage example**
    
    
    escapeXML ''"test" 'test' < & >''
    => "&quot;test&quot; &apos;test&apos; &lt; &amp; &gt;"
    

  

Located at [lib/strings.nix:1483](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1483>) in `<nixpkgs>`.

#### `lib.strings.toLower`

Converts an ASCII string `s` to lower-case.

##### Inputs 

`s`
    

The string to convert to lower-case.

##### Type 
    
    
    toLower :: String -> String
    

##### Examples 

**Example 93.`lib.strings.toLower` usage example**
    
    
    toLower "HOME"
    => "home"
    

  

Located at [lib/strings.nix:1517](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1517>) in `<nixpkgs>`.

#### `lib.strings.toUpper`

Converts an ASCII string `s` to upper-case.

##### Inputs 

`s`
    

The string to convert to upper-case.

##### Type 
    
    
    toUpper :: String -> String
    

##### Examples 

**Example 94.`lib.strings.toUpper` usage example**
    
    
    toUpper "home"
    => "HOME"
    

  

Located at [lib/strings.nix:1544](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1544>) in `<nixpkgs>`.

#### `lib.strings.toSentenceCase`

Converts the first character of a string `s` to upper-case.

##### Inputs 

`str`
    

The string to convert to sentence case.

##### Type 
    
    
    toSentenceCase :: String -> String
    

##### Examples 

**Example 95.`lib.strings.toSentenceCase` usage example**
    
    
    toSentenceCase "home"
    => "Home"
    

  

Located at [lib/strings.nix:1571](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1571>) in `<nixpkgs>`.

#### `lib.strings.toCamelCase`

Converts a string to camelCase. Handles snake_case, PascalCase, kebab-case strings as well as strings delimited by spaces.

##### Inputs 

`string`
    

The string to convert to camelCase

##### Type 
    
    
    toCamelCase :: String -> String
    

##### Examples 

**Example 96.`lib.strings.toCamelCase` usage example**
    
    
    toCamelCase "hello-world"
    => "helloWorld"
    toCamelCase "hello_world"
    => "helloWorld"
    toCamelCase "hello world"
    => "helloWorld"
    toCamelCase "HelloWorld"
    => "helloWorld"
    

  

Located at [lib/strings.nix:1615](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/strings.nix#L1615>) in `<nixpkgs>`.

#### `lib.strings.addContextFrom`

Appends string context from string like object `src` to `target`.

### lib.trivial: miscellaneous functions 

#### `lib.trivial.id`

The identity function For when you need a function that does “nothing”.

##### Inputs 

`x`
    

The value to return

##### Type 
    
    
    id :: a -> a
    

Located at [lib/trivial.nix:63](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L63>) in `<nixpkgs>`.

#### `lib.trivial.const`

The constant function

Ignores the second argument. If called with only one argument, constructs a function that always returns a static value.

##### Inputs 

`x`
    

Value to return

`y`
    

Value to ignore

##### Type 
    
    
    const :: a -> b -> a
    

##### Examples 

**Example 133.`lib.trivial.const` usage example**
    
    
    let f = const 5; in f 10
    => 5
    

  

Located at [lib/trivial.nix:98](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L98>) in `<nixpkgs>`.

#### `lib.trivial.pipe`

Pipes a value through a list of functions, left to right.

##### Inputs 

`value`
    

Value to start piping.

`fns`
    

List of functions to apply sequentially.

##### Type 
    
    
    pipe :: a -> [(a -> b) (b -> c) ... (x -> y) (y -> z)] -> z
    

##### Examples 

**Example 134.`lib.trivial.pipe` usage example**
    
    
    pipe 2 [
        (x: x + 2)  # 2 + 2 = 4
        (x: x * 2)  # 4 * 2 = 8
      ]
    => 8
    
    # ideal to do text transformations
    pipe [ "a/b" "a/c" ] [
    
      # create the cp command
      (map (file: ''cp "${src}/${file}" $out\n''))
    
      # concatenate all commands into one string
      lib.concatStrings
    
      # make that string into a nix derivation
      (pkgs.runCommand "copy-to-out" {})
    
    ]
    => <drv which copies all files to $out>
    
    The output type of each function has to be the input type
    of the next function, and the last function returns the
    final value.
    

  

Located at [lib/trivial.nix:152](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L152>) in `<nixpkgs>`.

#### `lib.trivial.concat`

Concatenate two lists

##### Inputs 

`x`
    

1\. Function argument

`y`
    

2\. Function argument

##### Type 
    
    
    concat :: [a] -> [a] -> [a]
    

##### Examples 

**Example 135.`lib.trivial.concat` usage example**
    
    
    concat [ 1 2 ] [ 3 4 ]
    => [ 1 2 3 4 ]
    

  

Located at [lib/trivial.nix:191](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L191>) in `<nixpkgs>`.

#### `lib.trivial."or"` {#function-library-lib.trivial.“or”} 

boolean “or”

##### Inputs 

`x`
    

1\. Function argument

`y`
    

2\. Function argument

##### Type 
    
    
    or :: Bool -> Bool -> Bool
    

#### `lib.trivial.and`

boolean “and”

##### Inputs 

`x`
    

1\. Function argument

`y`
    

2\. Function argument

##### Type 
    
    
    and :: Bool -> Bool -> Bool
    

Located at [lib/trivial.nix:233](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L233>) in `<nixpkgs>`.

#### `lib.trivial.xor`

boolean “exclusive or”

##### Inputs 

`x`
    

1\. Function argument

`y`
    

2\. Function argument

##### Type 
    
    
    xor :: bool -> bool -> bool
    

Located at [lib/trivial.nix:256](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L256>) in `<nixpkgs>`.

#### `lib.trivial.bitNot`

bitwise “not”

##### Type 
    
    
    bitNot :: Number -> Number
    

Located at [lib/trivial.nix:267](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L267>) in `<nixpkgs>`.

#### `lib.trivial.boolToString`

Convert a boolean to a string.

This function uses the strings “true” and “false” to represent boolean values. Calling `toString` on a bool instead returns “1” and “” (sic!).

##### Inputs 

`b`
    

1\. Function argument

##### Type 
    
    
    boolToString :: Bool -> String
    

Located at [lib/trivial.nix:288](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L288>) in `<nixpkgs>`.

#### `lib.trivial.boolToYesNo`

Converts a boolean to a string.

This function uses the strings “yes” and “no” to represent boolean values.

##### Inputs 

`b`
    

The boolean to convert

##### Type 
    
    
    boolToYesNo :: Bool -> String
    

Located at [lib/trivial.nix:308](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L308>) in `<nixpkgs>`.

#### `lib.trivial.mergeAttrs`

Merge two attribute sets shallowly, right side trumps left

##### Inputs 

`x`
    

Left attribute set

`y`
    

Right attribute set (higher precedence for equal keys)

##### Type 
    
    
    mergeAttrs :: AttrSet -> AttrSet -> AttrSet
    

##### Examples 

**Example 136.`lib.trivial.mergeAttrs` usage example**
    
    
    mergeAttrs { a = 1; b = 2; } { b = 3; c = 4; }
    => { a = 1; b = 3; c = 4; }
    

  

Located at [lib/trivial.nix:340](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L340>) in `<nixpkgs>`.

#### `lib.trivial.flip`

Flip the order of the arguments of a binary function.

##### Inputs 

`f`
    

1\. Function argument

`a`
    

2\. Function argument

`b`
    

3\. Function argument

##### Type 
    
    
    flip :: (a -> b -> c) -> (b -> a -> c)
    

##### Examples 

**Example 137.`lib.trivial.flip` usage example**
    
    
    flip concat [1] [2]
    => [ 2 1 ]
    

  

Located at [lib/trivial.nix:376](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L376>) in `<nixpkgs>`.

#### `lib.trivial.defaultTo`

Returns `maybeValue` if not null, otherwise return `default`.

##### Inputs 

`default`
    

1\. Function argument

`maybeValue`
    

2\. Function argument

##### Type 
    
    
    defaultTo :: a -> (b | Null) -> (b | a)
    

##### Examples 

**Example 138.`lib.trivial.defaultTo` usage example**
    
    
    defaultTo "default" null
    => "default"
    defaultTo "default" "foo"
    => "foo"
    defaultTo "default" false
    => false
    

  

Located at [lib/trivial.nix:414](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L414>) in `<nixpkgs>`.

#### `lib.trivial.mapNullable`

Apply function if the supplied argument is non-null.

##### Inputs 

`f`
    

Function to call

`a`
    

Argument to check for null before passing it to `f`

##### Type 
    
    
    mapNullable :: (a -> b) -> (a | Null) -> (b | Null)
    

##### Examples 

**Example 139.`lib.trivial.mapNullable` usage example**
    
    
    mapNullable (x: x+1) null
    => null
    mapNullable (x: x+1) 22
    => 23
    

  

Located at [lib/trivial.nix:448](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L448>) in `<nixpkgs>`.

#### `lib.trivial.version`

Returns the current full nixpkgs version number.

Located at [lib/trivial.nix:455](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L455>) in `<nixpkgs>`.

#### `lib.trivial.release`

Returns the current nixpkgs release number as string.

Located at [lib/trivial.nix:460](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L460>) in `<nixpkgs>`.

#### `lib.trivial.oldestSupportedRelease`

The latest release that is supported, at the time of release branch-off, if applicable.

Ideally, out-of-tree modules should be able to evaluate cleanly with all supported Nixpkgs versions (master, release and old release until EOL). So if possible, deprecation warnings should take effect only when all out-of-tree expressions/libs/modules can upgrade to the new way without losing support for supported Nixpkgs versions.

This release number allows deprecation warnings to be implemented such that they take effect as soon as the oldest release reaches end of life.

Located at [lib/trivial.nix:475](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L475>) in `<nixpkgs>`.

#### `lib.trivial.isInOldestRelease`

Whether a feature is supported in all supported releases (at the time of release branch-off, if applicable). See `oldestSupportedRelease`.

##### Inputs 

`release`
    

Release number of feature introduction as an integer, e.g. 2111 for 21.11. Set it to the upcoming release, matching the nixpkgs/.version file.

Located at [lib/trivial.nix:490](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L490>) in `<nixpkgs>`.

#### `lib.trivial.oldestSupportedReleaseIsAtLeast`

Alias for `isInOldestRelease` introduced in 24.11. Use `isInOldestRelease` in expressions outside of Nixpkgs for greater compatibility.

Located at [lib/trivial.nix:499](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L499>) in `<nixpkgs>`.

#### `lib.trivial.codeName`

Returns the current nixpkgs release code name.

On each release the first letter is bumped and a new animal is chosen starting with that new letter.

Located at [lib/trivial.nix:507](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L507>) in `<nixpkgs>`.

#### `lib.trivial.versionSuffix`

Returns the current nixpkgs version suffix as string.

Located at [lib/trivial.nix:512](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L512>) in `<nixpkgs>`.

#### `lib.trivial.revisionWithDefault`

Attempts to return the the current revision of nixpkgs and returns the supplied default value otherwise.

##### Inputs 

`default`
    

Default value to return if revision can not be determined

##### Type 
    
    
    revisionWithDefault :: String -> String
    

Located at [lib/trivial.nix:534](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L534>) in `<nixpkgs>`.

#### `lib.trivial.inNixShell`

Determine whether the function is being called from inside a Nix shell.

##### Type 
    
    
    inNixShell :: Bool
    

Located at [lib/trivial.nix:559](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L559>) in `<nixpkgs>`.

#### `lib.trivial.inPureEvalMode`

Determine whether the function is being called from inside pure-eval mode by seeing whether `builtins` contains `currentSystem`. If not, we must be in pure-eval mode.

##### Type 
    
    
    inPureEvalMode :: Bool
    

Located at [lib/trivial.nix:572](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L572>) in `<nixpkgs>`.

#### `lib.trivial.min`

Returns minimum of two numbers.

##### Inputs 

`x`
    

1\. Function argument

`y`
    

2\. Function argument

##### Type 
    
    
    min :: Number -> Number -> Number
    

Located at [lib/trivial.nix:595](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L595>) in `<nixpkgs>`.

#### `lib.trivial.max`

Returns maximum of two numbers.

##### Inputs 

`x`
    

1\. Function argument

`y`
    

2\. Function argument

##### Type 
    
    
    max :: Number -> Number -> Number
    

Located at [lib/trivial.nix:616](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L616>) in `<nixpkgs>`.

#### `lib.trivial.mod`

Integer modulus

##### Inputs 

`base`
    

1\. Function argument

`int`
    

2\. Function argument

##### Type 
    
    
    mod :: Int -> Int -> Int
    

##### Examples 

**Example 140.`lib.trivial.mod` usage example**
    
    
    mod 11 10
    => 1
    mod 1 10
    => 1
    

  

Located at [lib/trivial.nix:650](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L650>) in `<nixpkgs>`.

#### `lib.trivial.compare`

C-style comparisons

a < b, compare a b => -1 a == b, compare a b => 0 a > b, compare a b => 1

##### Inputs 

`a`
    

1\. Function argument

`b`
    

2\. Function argument

##### Type 
    
    
    compare :: a -> a -> Int
    

Located at [lib/trivial.nix:677](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L677>) in `<nixpkgs>`.

#### `lib.trivial.splitByAndCompare`

Split type into two subtypes by predicate `p`, take all elements of the first subtype to be less than all the elements of the second subtype, compare elements of a single subtype with `yes` and `no` respectively.

##### Inputs 

`p`
    

Predicate

`yes`
    

Comparison function if predicate holds for both values

`no`
    

Comparison function if predicate holds for neither value

`a`
    

First value to compare

`b`
    

Second value to compare

##### Type 
    
    
    splitByAndCompare :: (a -> Bool) -> (a -> a -> Int) -> (a -> a -> Int) -> (a -> a -> Int)
    

##### Examples 

**Example 141.`lib.trivial.splitByAndCompare` usage example**
    
    
    let cmp = splitByAndCompare (hasPrefix "foo") compare compare; in
    
    cmp "a" "z" => -1
    cmp "fooa" "fooz" => -1
    
    cmp "f" "a" => 1
    cmp "fooa" "a" => -1
    # while
    compare "fooa" "a" => 1
    

  

Located at [lib/trivial.nix:738](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L738>) in `<nixpkgs>`.

#### `lib.trivial.importJSON`

Reads a JSON file.

##### Examples 

**Example 142.`lib.trivial.importJSON` usage example**

example.json
    
    
    {
      "title": "Example JSON",
      "hello": {
        "world": "foo",
        "bar": {
          "foobar": true
        }
      }
    }
    
    
    
    importJSON ./example.json
    => {
      title = "Example JSON";
      hello = {
        world = "foo";
        bar = {
          foobar = true;
        };
      };
    }
    

  

##### Inputs 

`path`
    

1\. Function argument

##### Type 
    
    
    importJSON :: Path -> Any
    

Located at [lib/trivial.nix:794](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L794>) in `<nixpkgs>`.

#### `lib.trivial.importTOML`

Reads a TOML file.

##### Examples 

**Example 143.`lib.trivial.importTOML` usage example**

example.toml
    
    
    title = "TOML Example"
    
    [hello]
    world = "foo"
    
    [hello.bar]
    foobar = true
    
    
    
    importTOML ./example.toml
    => {
      title = "TOML Example";
      hello = {
        world = "foo";
        bar = {
          foobar = true;
        };
      };
    }
    

  

##### Inputs 

`path`
    

1\. Function argument

##### Type 
    
    
    importTOML :: Path -> Any
    

Located at [lib/trivial.nix:841](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L841>) in `<nixpkgs>`.

#### `lib.trivial.warn`

`warn` _`message`_ _`value`_

Print a warning before returning the second argument.

See [`builtins.warn`](<https://nix.dev/manual/nix/latest/language/builtins.html#builtins-warn>) (Nix >= 2.23). On older versions, the Nix 2.23 behavior is emulated with [`builtins.trace`](<https://nix.dev/manual/nix/latest/language/builtins.html#builtins-warn>), including the [`NIX_ABORT_ON_WARN`](<https://nix.dev/manual/nix/latest/command-ref/conf-file#conf-abort-on-warn>) behavior, but not the `nix.conf` setting or command line option.

##### Inputs 

_`message`_ (String)
    

Warning message to print before evaluating _`value`_.

_`value`_ (any value)
    

Value to return as-is.

##### Type 
    
    
    warn :: String -> a -> a
    

Located at [lib/trivial.nix:867](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L867>) in `<nixpkgs>`.

#### `lib.trivial.warnIf`

`warnIf` _`condition`_ _`message`_ _`value`_

Like `warn`, but only warn when the first argument is `true`.

##### Inputs 

_`condition`_ (Boolean)
    

`true` to trigger the warning before continuing with _`value`_.

_`message`_ (String)
    

Warning message to print before evaluating

 _`value`_ (any value)
    

Value to return as-is.

##### Type 
    
    
    warnIf :: Bool -> String -> a -> a
    

Located at [lib/trivial.nix:914](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L914>) in `<nixpkgs>`.

#### `lib.trivial.warnIfNot`

`warnIfNot` _`condition`_ _`message`_ _`value`_

Like `warnIf`, but negated: warn if the first argument is `false`.

##### Inputs 

_`condition`_
    

`false` to trigger the warning before continuing with `val`.

_`message`_
    

Warning message to print before evaluating _`value`_.

_`value`_
    

Value to return as-is.

##### Type 
    
    
    warnIfNot :: Bool -> String -> a -> a
    

Located at [lib/trivial.nix:941](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L941>) in `<nixpkgs>`.

#### `lib.trivial.throwIfNot`

Like the `assert b; e` expression, but with a custom error message and without the semicolon.

If true, return the identity function, `r: r`.

If false, throw the error message.

Calls can be juxtaposed using function application, as `(r: r) a = a`, so `(r: r) (r: r) a = a`, and so forth.

##### Inputs 

`cond`
    

1\. Function argument

`msg`
    

2\. Function argument

##### Type 
    
    
    throwIfNot :: Bool -> String -> a -> (a | Never)
    

##### Examples 

**Example 144.`lib.trivial.throwIfNot` usage example**
    
    
    throwIfNot (lib.isList overlays) "The overlays argument to nixpkgs must be a list."
    lib.foldr (x: throwIfNot (lib.isFunction x) "All overlays passed to nixpkgs must be functions.") (r: r) overlays
    pkgs
    

  

Located at [lib/trivial.nix:982](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L982>) in `<nixpkgs>`.

#### `lib.trivial.throwIf`

Like `throwIfNot`, but negated (throw if the first argument is `true`).

##### Inputs 

`cond`
    

1\. Function argument

`msg`
    

2\. Function argument

##### Type 
    
    
    throwIf :: Bool -> String -> a -> (a | Never)
    

Located at [lib/trivial.nix:1003](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L1003>) in `<nixpkgs>`.

#### `lib.trivial.checkListOfEnum`

Check if the elements in a list are valid values from a enum, returning the identity function, or throwing an error message otherwise.

##### Inputs 

`msg`
    

1\. Function argument

`valid`
    

2\. Function argument

`given`
    

3\. Function argument

##### Type 
    
    
    checkListOfEnum :: String -> [a] -> [a] -> ((b -> b) | Never)
    

##### Examples 

**Example 145.`lib.trivial.checkListOfEnum` usage example**
    
    
    let colorVariants = ["bright" "dark" "black"]
    in checkListOfEnum "color variants" [ "standard" "light" "dark" ] colorVariants;
    =>
    error: color variants: bright, black unexpected; valid ones: standard, light, dark
    

  

Located at [lib/trivial.nix:1041](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L1041>) in `<nixpkgs>`.

#### `lib.trivial.setFunctionArgs`

Add metadata about expected function arguments to a function. The metadata should match the format given by builtins.functionArgs, i.e. a set from expected argument to a bool representing whether that argument has a default or not.

This function is necessary because you can’t dynamically create a function of the `{ a, b ? foo, ... }:` format, but some facilities like `callPackage` expect to be able to query expected arguments.

##### Inputs 

`f`
    

1\. Function argument

`args`
    

2\. Function argument

##### Type 
    
    
    setFunctionArgs : (a -> b) -> { [String] :: Bool } -> (a -> b)
    

Located at [lib/trivial.nix:1081](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L1081>) in `<nixpkgs>`.

#### `lib.trivial.functionArgs`

Extract the expected function arguments from a function. This works both with nix-native `{ a, b ? foo, ... }:` style functions and functions with args set with `setFunctionArgs`. It has the same return type and semantics as `builtins.functionArgs`.

##### Inputs 

`f`
    

1\. Function argument

##### Type 
    
    
    functionArgs : (a -> b) -> { [String] :: Bool }
    

Located at [lib/trivial.nix:1105](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L1105>) in `<nixpkgs>`.

#### `lib.trivial.isFunction`

Check whether something is a function or something annotated with function args.

##### Inputs 

`f`
    

1\. Function argument

##### Type 
    
    
    isFunction : Any -> Bool
    

Located at [lib/trivial.nix:1127](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L1127>) in `<nixpkgs>`.

#### `lib.trivial.mirrorFunctionArgs`

`mirrorFunctionArgs f g` creates a new function `g'` with the same behavior as `g` (`g' x == g x`) but its function arguments mirroring `f` (`lib.functionArgs g' == lib.functionArgs f`).

##### Inputs 

`f`
    

Function to provide the argument metadata

`g`
    

Function to set the argument metadata to

##### Type 
    
    
    mirrorFunctionArgs :: (a -> b) -> (a -> c) -> (a -> c)
    

##### Examples 

**Example 146.`lib.trivial.mirrorFunctionArgs` usage example**
    
    
    addab = {a, b}: a + b
    addab { a = 2; b = 4; }
    => 6
    lib.functionArgs addab
    => { a = false; b = false; }
    addab1 = attrs: addab attrs + 1
    addab1 { a = 2; b = 4; }
    => 7
    lib.functionArgs addab1
    => { }
    addab1' = lib.mirrorFunctionArgs addab addab1
    addab1' { a = 2; b = 4; }
    => 7
    lib.functionArgs addab1'
    => { a = false; b = false; }
    

  

Located at [lib/trivial.nix:1177](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L1177>) in `<nixpkgs>`.

#### `lib.trivial.toFunction`

Turns any non-callable values into constant functions. Returns callable values as is.

##### Inputs 

`v`
    

Any value

##### Examples 

**Example 147.`lib.trivial.toFunction` usage example**
    
    
    nix-repl> lib.toFunction 1 2
    1
    
    nix-repl> lib.toFunction (x: x + 1) 2
    3
    

  

Located at [lib/trivial.nix:1211](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L1211>) in `<nixpkgs>`.

#### `lib.trivial.fromHexString`

Convert a hexadecimal string to it’s integer representation.

##### Type 
    
    
    fromHexString :: String -> Int
    

##### Examples 

**Example 148.`lib.trivial.fromHexString` usage examples**
    
    
    fromHexString "FF"
    => 255
    
    fromHexString "0x7fffffffffffffff"
    => 9223372036854775807
    

  

Located at [lib/trivial.nix:1234](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L1234>) in `<nixpkgs>`.

#### `lib.trivial.toHexString`

Convert the given positive integer to a string of its hexadecimal representation.

##### Type 
    
    
    toHexString :: Int -> String
    

##### Examples 

**Example 149.`lib.trivial.toHexString` usage example**
    
    
    toHexString 0 => "0"
    
    toHexString 16 => "10"
    
    toHexString 250 => "FA"
    

  

Located at [lib/trivial.nix:1274](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L1274>) in `<nixpkgs>`.

#### `lib.trivial.toBaseDigits`

`toBaseDigits base i` converts the positive integer `i` to a list of its digits in the given base.

##### Inputs 

`base`
    

1\. Function argument

`i`
    

2\. Function argument

##### Type 
    
    
    toBaseDigits :: Int -> Int -> [Int]
    

##### Examples 

**Example 150.`lib.trivial.toBaseDigits`**
    
    
    toBaseDigits 10 123 => [ 1 2 3 ]
    
    toBaseDigits 2 6 => [ 1 1 0 ]
    
    toBaseDigits 16 250 => [ 15 10 ]
    

  

Located at [lib/trivial.nix:1321](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/trivial.nix#L1321>) in `<nixpkgs>`.

### lib.lists: list manipulation functions 

General list operations.

#### `lib.lists.singleton`

Create a list consisting of a single element. `singleton x` is sometimes more convenient with respect to indentation than `[x]` when x spans multiple lines.

##### Inputs 

`x`
    

1\. Function argument

##### Type 
    
    
    singleton :: a -> [a]
    

##### Examples 

**Example 156.`lib.lists.singleton` usage example**
    
    
    singleton "foo"
    => [ "foo" ]
    

  

Located at [lib/lists.nix:59](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/lists.nix#L59>) in `<nixpkgs>`.

#### `lib.lists.forEach`

Apply the function to each element in the list. Same as `map`, but arguments flipped.

##### Inputs 

`xs`
    

1\. Function argument

`f`
    

2\. Function argument

##### Type 
    
    
    forEach :: [a] -> (a -> b) -> [b]
    

##### Examples 

**Example 157.`lib.lists.forEach` usage example**
    
    
    forEach [ 1 2 ] (x:
      toString x
    )
    => [ "1" "2" ]
    

  

Located at [lib/lists.nix:94](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/lists.nix#L94>) in `<nixpkgs>`.

#### `lib.lists.foldr`

“right fold” a binary function `op` between successive elements of `list` with `nul` as the starting value, i.e., `foldr op nul [x_1 x_2 ... x_n] == op x_1 (op x_2 ... (op x_n nul))`.

##### Inputs 

`op`
    

1\. Function argument

`nul`
    

2\. Function argument

`list`
    

3\. Function argument

##### Type 
    
    
    foldr :: (a -> b -> b) -> b -> [a] -> b
    

##### Examples 

**Example 158.`lib.lists.foldr` usage example**
    
    
    concat = foldr (a: b: a + b) "z"
    concat [ "a" "b" "c" ]
    => "abcz"
    # different types
    strange = foldr (int: str: toString (int + 1) + str) "a"
    strange [ 1 2 3 4 ]
    => "2345a"
    

  

Located at [lib/lists.nix:137](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/lists.nix#L137>) in `<nixpkgs>`.

#### `lib.lists.fold`

`fold` is an alias of `foldr` for historic reasons.

### lib.sources: source filtering functions 

#### `lib.sources.commitIdFromGitRepo`

Get the commit id of a git repo.

##### Inputs 

`path`
    

1\. Function argument

##### Examples 

**Example 261.`commitIdFromGitRepo` usage example**
    
    
    commitIdFromGitRepo <nixpkgs/.git>
    

  

Located at [lib/sources.nix:520](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/sources.nix#L520>) in `<nixpkgs>`.

#### `lib.sources.cleanSource`

Filters a source tree removing version control files and directories using `cleanSourceFilter`.

##### Inputs 

`src`
    

1\. Function argument

##### Examples 

**Example 262.`cleanSource` usage example**
    
    
    cleanSource ./.
    

  

Located at [lib/sources.nix:522](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/sources.nix#L522>) in `<nixpkgs>`.

#### `lib.sources.cleanSourceWith`

Like `builtins.filterSource`, except it will compose with itself, allowing you to chain multiple calls together without any intermediate copies being put in the nix store.

##### Examples 

**Example 263.`cleanSourceWith` usage example**
    
    
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
    

  

Located at [lib/sources.nix:523](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/sources.nix#L523>) in `<nixpkgs>`.

#### `lib.sources.cleanSourceFilter`

A basic filter for `cleanSourceWith` that removes directories of version control system, backup files (`*~`) and some generated files.

##### Inputs 

`name`
    

1\. Function argument

`type`
    

2\. Function argument

Located at [lib/sources.nix:524](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/sources.nix#L524>) in `<nixpkgs>`.

#### `lib.sources.sourceByRegex`

Filter sources by a list of regular expressions.

##### Inputs 

`src`
    

1\. Function argument

`regexes`
    

2\. Function argument

##### Examples 

**Example 264.`sourceByRegex` usage example**
    
    
    src = sourceByRegex ./my-subproject [".*\\.py$" "^database\\.sql$"]
    

  

Located at [lib/sources.nix:533](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/sources.nix#L533>) in `<nixpkgs>`.

#### `lib.sources.sourceFilesBySuffices`

Get all files ending with the specified suffices from the given source directory or its descendants, omitting files that do not match any suffix. The result of the example below will include files like `./dir/module.c` and `./dir/subdir/doc.xml` if present.

##### Inputs 

`src`
    

Path or source containing the files to be returned

`exts`
    

A list of file suffix strings

##### Type 
    
    
    sourceFilesBySuffices :: SourceLike -> [String] -> Source
    

##### Examples 

**Example 265.`sourceFilesBySuffices` usage example**
    
    
    sourceFilesBySuffices ./. [ ".xml" ".c" ]
    

  

Located at [lib/sources.nix:534](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/sources.nix#L534>) in `<nixpkgs>`.

#### `lib.sources.trace`

Add logging to a source, for troubleshooting the filtering behavior.

##### Inputs 

`src`
    

Source to debug. The returned source will behave like this source, but also log its filter invocations.

##### Type 
    
    
    sources.trace :: SourceLike -> Source
    

Located at [lib/sources.nix:536](<https://github.com/NixOS/nixpkgs/blob/95ca1e203c0750115fd4a6f17d5a245dfe6b1edd/lib/sources.nix#L536>) in `<nixpkgs>`.
