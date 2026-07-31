---
type: Crawl Source
title: "Module system"
description: "Much of the power in Nixpkgs and NixOS comes from the module system."
resource: https://nix.dev/tutorials/module-system/index.html
tags: [Tutorials, Module System]
timestamp: 2026-07-24T00:00:00Z
---
> **seed_url**: https://nix.dev/tutorials/module-system/index.html
> **canonical_url**: https://nix.dev/tutorials/module-system/index.html
> **family**: Tutorials
> **fetch**: cloned from github.com/nixos/nix.dev
> **version**: 139034be5e14320c05f792872e6150bd981490d5 (2026-07-21)
> **feeds_docs**: TBD

# Verbatim source content

(module-system-tutorial)=
# Module system

Much of the power in Nixpkgs and NixOS comes from the module system.

The module system is a Nix language library that enables you to
- Declare one attribute set using many separate Nix expressions.
- Imposes type constraints on values in that attribute set.
- Define values for the same attribute in different Nix expressions and merge these values automatically according to their type.

These Nix expressions are called modules and must have a particular structure.

In this tutorial series you'll learn
- What a module is and how to create one.
- What options are and how to declare them.
- How to express dependencies between modules.

## What do you need?

- Familiarity with data types and general programming concepts
- A {ref}`Nix installation <install-nix>` to run the examples
- Intermediate proficiency in reading and writing the {ref}`Nix language <reading-nix-language>`

## How long will it take?

This is a very long tutorial.
Prepare for at least 3 hours of work.

```{toctree}
:maxdepth: 1
:caption: Lessons
:numbered:
a-basic-module/index.md
deep-dive.md
```
