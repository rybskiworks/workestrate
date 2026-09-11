---
type: Crawl Source
title: "nixpkgs — mkShell"
description: "The pkgs.mkShell function for creating development shells."
resource: https://nixos.org/manual/nixpkgs/stable/#sec-mkShell
tags: [nix, nixpkgs-manual, mkShell, devshell]
timestamp: 2026-07-24T00:00:00Z
---

# Crawl: nixpkgs-mkShell

- seed_url: https://nixos.org/manual/nixpkgs/stable/
- canonical_url: https://nixos.org/manual/nixpkgs/stable/
- family: nixpkgs Manual
- fetch: 200
- version: nixpkgs 26.05
- feeds_docs: nix-shell.md

## Content

## pkgs.mkShell 

[Usage](</manual/nixpkgs/stable/#sec-pkgs-mkShell-usage>)
[Attributes](</manual/nixpkgs/stable/#sec-pkgs-mkShell-attributes>)
[Variants](</manual/nixpkgs/stable/#sec-pkgs-mkShell-variants>)
[Building the shell](</manual/nixpkgs/stable/#sec-pkgs-mkShell-building>)

`pkgs.mkShell` is a specialized `stdenv.mkDerivation` that removes some repetition when using it with `nix-shell` (or `nix develop`).

### Usage 

Here is a common usage example:
    
    
    {
      pkgs ? import <nixpkgs> { },
    }:
    pkgs.mkShell {
      packages = [ pkgs.gnumake ];
    
      inputsFrom = [
        pkgs.hello
        pkgs.gnutar
      ];
    
      shellHook = ''
        export DEBUG=1
      '';
    }
    

### Attributes 

  * `name` (default: `nix-shell`). Set the name of the derivation.

  * `packages` (default: `[]`). Add executable packages to the `nix-shell` environment.

  * `inputsFrom` (default: `[]`). Add build dependencies of the listed derivations to the `nix-shell` environment.

  * `shellHook` (default: `""`). Bash statements that are executed by `nix-shell`.

… all the attributes of `stdenv.mkDerivation`.

### Variants 

`pkgs.mkShellNoCC` is a variant that uses `stdenvNoCC` instead of `stdenv` as base environment. This is useful if no C compiler is needed in the shell environment.

### Building the shell 

This derivation output will contain a text file that contains a reference to all the build inputs. This is useful in CI where we want to make sure that every derivation, and its dependencies, build properly. Or when creating a GC root so that the build dependencies don’t get garbage-collected.
