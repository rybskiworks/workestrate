---
type: Crawl Source
title: "Nix Language — Derivations"
description: "Derivations in the Nix expression language."
resource: https://nix.dev/manual/nix/2.34/language/derivations
tags: [nix, nix-manual, language, derivations]
timestamp: 2026-07-24T00:00:00Z
---

# Crawl: nix-language-derivations

- seed_url: https://nix.dev/manual/nix/2.34/language/derivations
- canonical_url: https://nix.dev/manual/nix/2.34/language/derivations
- family: Nix Manual
- fetch: 200
- version: Nix 2.34
- feeds_docs: derivations.md, nix-language.md

## Content

# Derivations

The most important built-in function is `derivation`, which is used to describe a single store-layer [store derivation](</manual/nix/2.34/glossary#gloss-store-derivation>). Consult the [store chapter](</manual/nix/2.34/store/derivation/>) for what a store derivation is; this section just concerns how to create one from the Nix language.

This builtin function takes as input an attribute set, the attributes of which specify the inputs to the process. It outputs an attribute set, and produces a [store derivation](</manual/nix/2.34/glossary#gloss-store-derivation>) as a side effect of evaluation.

## Input attributes

### Required

  * `name` ([String](</manual/nix/2.34/language/types#type-string>))

A symbolic name for the derivation. See [derivation outputs](</manual/nix/2.34/store/derivation/outputs/#outputs>) for what this is affects.

> **Example**
>         
>         derivation {
>           name = "hello";
>           # ...
>         }
>         
> 
> The derivation's path will be `/nix/store/<hash>-hello.drv`. The output paths will be of the form `/nix/store/<hash>-hello[-<output>]`

  * `system` ([String](</manual/nix/2.34/language/types#type-string>))

See [system](</manual/nix/2.34/store/derivation/#system>).

> **Example**
> 
> Declare a derivation to be built on a specific system type:
>         
>         derivation {
>           # ...
>           system = "x86_64-linux";
>           # ...
>         }
>         

> **Example**
> 
> Declare a derivation to be built on the system type that evaluates the expression:
>         
>         derivation {
>           # ...
>           system = builtins.currentSystem;
>           # ...
>         }
>         
> 
> [`builtins.currentSystem`](</manual/nix/2.34/language/builtins#builtins-currentSystem>) has the value of the [`system` configuration option], and defaults to the system type of the current Nix installation.

  * `builder` ([Path](</manual/nix/2.34/language/types#type-path>) | [String](</manual/nix/2.34/language/types#type-string>))

See [builder](</manual/nix/2.34/store/derivation/#builder>).

> **Example**
> 
> Use the file located at `/bin/bash` as the builder executable:
>         
>         derivation {
>           # ...
>           builder = "/bin/bash";
>           # ...
>         };
>         

> **Example**
> 
> Copy a local file to the Nix store for use as the builder executable:
>         
>         derivation {
>           # ...
>           builder = ./builder.sh;
>           # ...
>         };
>         

> **Example**
> 
> Use a file from another derivation as the builder executable:
>         
>         let pkgs = import <nixpkgs> {}; in
>         derivation {
>           # ...
>           builder = "${pkgs.python}/bin/python";
>           # ...
>         };
>         

### Optional

  * `args` ([List](</manual/nix/2.34/language/types#type-list>) of [String](</manual/nix/2.34/language/types#type-string>))

Default: `[ ]`

See [args](</manual/nix/2.34/store/derivation/#args>).

> **Example**
> 
> Pass arguments to Bash to interpret a shell command:
>         
>         derivation {
>           # ...
>           builder = "/bin/bash";
>           args = [ "-c" "echo hello world > $out" ];
>           # ...
>         };
>         

  * `outputs` ([List](</manual/nix/2.34/language/types#type-list>) of [String](</manual/nix/2.34/language/types#type-string>))

Default: `[ "out" ]`

Symbolic outputs of the derivation. Each output name is passed to the `builder` executable as an environment variable with its value set to the corresponding [store path](</manual/nix/2.34/store/store-path>).

By default, a derivation produces a single output called `out`. However, derivations can produce multiple outputs. This allows the associated [store objects](</manual/nix/2.34/store/store-object>) and their [closures](</manual/nix/2.34/glossary#gloss-closure>) to be copied or garbage-collected separately.

> **Example**
> 
> Imagine a library package that provides a dynamic library, header files, and documentation. A program that links against such a library doesn’t need the header files and documentation at runtime, and it doesn’t need the documentation at build time. Thus, the library package could specify:
>         
>         derivation {
>           # ...
>           outputs = [ "lib" "dev" "doc" ];
>           # ...
>         }
>         
> 
> This will cause Nix to pass environment variables `lib`, `dev`, and `doc` to the builder containing the intended store paths of each output. The builder would typically do something like
>         
>         ./configure \
>           --libdir=$lib/lib \
>           --includedir=$dev/include \
>           --docdir=$doc/share/doc
>         
> 
> for an Autoconf-style package.

The name of an output is combined with the name of the derivation to create the name part of the output's store path, unless it is `out`, in which case just the name of the derivation is used.

> **Example**
>         
>         derivation {
>           name = "example";
>           outputs = [ "lib" "dev" "doc" "out" ];
>           # ...
>         }
>         
> 
> The store derivation path will be `/nix/store/<hash>-example.drv`. The output paths will be
> 
>     * `/nix/store/<hash>-example-lib`
>     * `/nix/store/<hash>-example-dev`
>     * `/nix/store/<hash>-example-doc`
>     * `/nix/store/<hash>-example`

You can refer to each output of a derivation by selecting it as an attribute. The first element of `outputs` determines the _default output_ and ends up at the top-level.

> **Example**
> 
> Select an output by attribute name:
>     
>     let
>       myPackage = derivation {
>         name = "example";
>         outputs = [ "lib" "dev" "doc" "out" ];
>         # ...
>       };
>     in myPackage.dev
>     
> 
> Since `lib` is the first output, `myPackage` is equivalent to `myPackage.lib`.

  * See [Advanced Attributes](</manual/nix/2.34/language/advanced-attributes>) for more, infrequently used, optional attributes.

  * Every other attribute is passed as an environment variable to the builder. Attribute values are translated to environment variables as follows:

    * Strings are passed unchanged.

    * Integral numbers are converted to decimal notation.

    * Floating point numbers are converted to simple decimal or scientific notation with a preset precision.

    * A _path_ (e.g., `../foo/sources.tar`) causes the referenced file to be copied to the store; its location in the store is put in the environment variable. The idea is that all sources should reside in the Nix store, since all inputs to a derivation should reside in the Nix store.

    * A _derivation_ causes that derivation to be built prior to the present derivation. The environment variable is set to the [store path](</manual/nix/2.34/store/store-path>) of the derivation's default output.

    * Lists of the previous types are also allowed. They are simply concatenated, separated by spaces.

    * `true` is passed as the string `1`, `false` and `null` are passed as an empty string.
