---
type: Crawl Source
title: "Nix Store — Store Path"
description: "Store paths in the Nix store."
resource: https://nix.dev/manual/nix/2.34/store/store-path
tags: [nix, nix-manual, store, store-path]
timestamp: 2026-07-24T00:00:00Z
---

# Crawl: nix-store-path

- seed_url: https://nix.dev/manual/nix/2.34/store/store-path
- canonical_url: https://nix.dev/manual/nix/2.34/store/store-path
- family: Nix Manual
- fetch: 200
- version: Nix 2.34
- feeds_docs: nix-store.md

## Content

# Store Path

> **Example**
> 
> `/nix/store/jf6gn2dzna4nmsfbdxsd7kwhsk6gnnlr-git-2.38.1`
> 
> A rendered store path

Nix implements references to [store objects](</manual/nix/2.34/store/store-object>) as _store paths_.

Think of a store path as an [opaque](<https://en.m.wikipedia.org/wiki/Opaque_data_type>), [unique identifier](<https://en.m.wikipedia.org/wiki/Unique_identifier>): The only way to obtain store path is by adding or building store objects. A store path will always reference exactly one store object.

Store paths are pairs of

  * A 20-byte digest for identification
  * A symbolic name for people to read

> **Example**
> 
>   * Digest: `q06x3jll2yfzckz2bzqak089p43ixkkq`
>   * Name: `firefox-33.1`
> 

To make store objects accessible to operating system processes, stores have to expose store objects through the file system.

A store path is rendered to a file system path as the concatenation of

  * Store directory (typically `/nix/store`)
  * Path separator (`/`)
  * Digest rendered in [Nix32](</manual/nix/2.34/protocols/nix32>), a variant of base-32 (20 hash bytes become 32 ASCII characters)
  * Hyphen (`-`)
  * Name

> **Example**
>     
>     
>       /nix/store/q06x3jll2yfzckz2bzqak089p43ixkkq-firefox-33.1
>       |--------| |------------------------------| |----------|
>     store directory            digest                 name
>     

Exactly how the digest is calculated depends on the type of store path. Store path digests are _supposed_ to be opaque, and so for most operations, it is not necessary to know the details. That said, the manual has a full [specification of store path digests](</manual/nix/2.34/protocols/store-path>).

## Store Directory

Every [Nix store](</manual/nix/2.34/store/>) has a store directory.

Not every store can be accessed through the file system. But if the store has a file system representation, the store directory contains the store’s [file system objects](</manual/nix/2.34/store/file-system-object>), which can be addressed by store paths.

This means a store path is not just derived from the referenced store object itself, but depends on the store that the store object is in.

> **Note**
> 
> The store directory defaults to `/nix/store`, but is in principle arbitrary.

It is important which store a given store object belongs to: Files in the store object can contain store paths, and processes may read these paths. Nix can only guarantee referential integrity if store paths do not cross store boundaries.

Therefore one can only copy store objects to a different store if

  * The source and target stores' directories match

or

  * The store object in question has no references, that is, contains no store paths

One cannot copy a store object to a store with a different store directory. Instead, it has to be rebuilt, together with all its dependencies. It is in general not enough to replace the store directory string in file contents, as this may render executables unusable by invalidating their internal offsets or checksums.
