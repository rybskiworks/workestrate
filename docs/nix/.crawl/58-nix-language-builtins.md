---
type: Crawl Source
title: "Nix Language — Built-ins"
description: "Built-in functions in the Nix expression language."
resource: https://nix.dev/manual/nix/2.34/language/builtins
tags: [nix, nix-manual, language, builtins]
timestamp: 2026-07-24T00:00:00Z
---

# Crawl: nix-language-builtins

- seed_url: https://nix.dev/manual/nix/2.34/language/builtins
- canonical_url: https://nix.dev/manual/nix/2.34/language/builtins
- family: Nix Manual
- fetch: 200
- version: Nix 2.34
- feeds_docs: nix-language.md

## Content

# Built-ins

This section lists the values and functions built into the Nix language evaluator. All built-ins are available through the global `builtins` constant.

Some built-ins are also exposed directly in the global scope:

  * `derivation`
  * `derivationStrict`
  * `abort`
  * `baseNameOf`
  * `break`
  * `dirOf`
  * `false`
  * `fetchGit`
  * `fetchMercurial`
  * `fetchTarball`
  * `fetchTree`
  * `fromTOML`
  * `import`
  * `isNull`
  * `map`
  * `null`
  * `placeholder`
  * `removeAttrs`
  * `scopedImport`
  * `throw`
  * `toString`
  * `true`

`derivation attrs`
    

derivation is described in [its own section](</manual/nix/2.34/language/derivations>).

`abort s`
    

Abort Nix expression evaluation and print the error message _s_.

`add e1 e2`
    

Return the sum of the numbers _e1_ and _e2_.

`addDrvOutputDependencies s`
    

Create a copy of the given string where a single [constant](</manual/nix/2.34/language/string-context#string-context-constant>) string context element is turned into a [derivation deep](</manual/nix/2.34/language/string-context#string-context-element-derivation-deep>) string context element.

The store path that is the constant string context element should point to a valid derivation, and end in `.drv`.

The original string context element must not be empty or have multiple elements, and it must not have any other type of element other than a constant or derivation deep element. The latter is supported so this function is idempotent.

This is the opposite of `builtins.unsafeDiscardOutputDependency`.

`all pred list`
    

Return `true` if the function _pred_ returns `true` for all elements of _list_ , and `false` otherwise.

`any pred list`
    

Return `true` if the function _pred_ returns `true` for at least one element of _list_ , and `false` otherwise.

`attrNames set`
    

Return the names of the attributes in the set _set_ in an alphabetically sorted list. For instance, `builtins.attrNames { y = 1; x = "foo"; }` evaluates to `[ "x" "y" ]`.

`attrValues set`
    

Return the values of the attributes in the set _set_ in the order corresponding to the sorted attribute names.

`baseNameOf x`
    

Return the _base name_ of either a [path value](</manual/nix/2.34/language/types#type-path>) _x_ or a string _x_ , depending on which type is passed, and according to the following rules.

For a path value, the _base name_ is considered to be the part of the path after the last directory separator, including any file extensions. This is the simple case, as path values don't have trailing slashes.

When the argument is a string, a more involved logic applies. If the string ends with a `/`, only this one final slash is removed.

After this, the _base name_ is returned as previously described, assuming `/` as the directory separator. (Note that evaluation must be platform independent.)

This is somewhat similar to the [GNU `basename`](<https://www.gnu.org/software/coreutils/manual/html_node/basename-invocation.html>) command, but GNU `basename` strips any number of trailing slashes.

`bitAnd e1 e2`
    

Return the bitwise AND of the integers _e1_ and _e2_.

`bitOr e1 e2`
    

Return the bitwise OR of the integers _e1_ and _e2_.

`bitXor e1 e2`
    

Return the bitwise XOR of the integers _e1_ and _e2_.

`break v`
    

In debug mode (enabled using `--debugger`), pause Nix expression evaluation and enter the REPL. Otherwise, return the argument `v`.

`builtins` (set) 
    

Contains all the built-in functions and values.

Since built-in functions were added over time, [testing for attributes](</manual/nix/2.34/language/operators#has-attribute>) in `builtins` can be used for graceful fallback on older Nix installations:
    
    
    # if hasContext is not available, we assume `s` has a context
    if builtins ? hasContext then builtins.hasContext s else true
    

`catAttrs attr list`
    

Collect each attribute named _attr_ from a list of attribute sets. Attrsets that don't contain the named attribute are ignored. For example,
    
    
    builtins.catAttrs "a" [{a = 1;} {b = 0;} {a = 2;}]
    

evaluates to `[1 2]`.

`ceil number`
    

Rounds and converts _number_ to the next higher NixInt value if possible, i.e. `ceil *number* >= *number*` and `ceil *number* - *number* < 1`.

An evaluation error is thrown, if there exists no such NixInt value `ceil *number*`. Due to bugs in previous Nix versions an evaluation error might be thrown, if the datatype of _number_ is a NixInt and if `*number* < -9007199254740992` or `*number* > 9007199254740992`.

If the datatype of _number_ is neither a NixInt (signed 64-bit integer) nor a NixFloat (IEEE-754 double-precision floating-point number), an evaluation error is thrown.

`compareVersions s1 s2`
    

Compare two strings representing versions and return `-1` if version _s1_ is older than version _s2_ , `0` if they are the same, and `1` if _s1_ is newer than _s2_. The version comparison algorithm is the same as the one used by [`nix-env -u`](</manual/nix/2.34/command-ref/nix-env/upgrade>).

`concatLists lists`
    

Concatenate a list of lists into a single list.

`concatMap f list`
    

This function is equivalent to `builtins.concatLists (map f list)` but is more efficient.

`concatStringsSep separator list`
    

Concatenate a list of strings with a separator between each element, e.g. `concatStringsSep "/" ["usr" "local" "bin"] == "usr/local/bin"`.

`convertHash args`
    

Return the specified representation of a hash string, based on the attributes presented in _args_ :

  * `hash`

The hash to be converted. The hash format is detected automatically.

  * `hashAlgo`

The algorithm used to create the hash. Must be one of

    * `"md5"`
    * `"sha1"`
    * `"sha256"`
    * `"sha512"`

The attribute may be omitted when `hash` is an [SRI hash](<https://www.w3.org/TR/SRI/#the-integrity-attribute>) or when the hash is prefixed with the hash algorithm name followed by a colon. That `<hashAlgo>:<hashBody>` syntax is supported for backwards compatibility with existing tooling.

  * `toHashFormat`

The format of the resulting hash. Must be one of

    * `"base16"`
    * `"nix32"`
    * `"base32"` (deprecated alias for `"nix32"`)
    * `"base64"`
    * `"sri"`

The result hash is the _toHashFormat_ representation of the hash _hash_.

> **Example**
> 
> Convert a SHA256 hash in Base16 to SRI:
>     
>     
>     builtins.convertHash {
>       hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
>       toHashFormat = "sri";
>       hashAlgo = "sha256";
>     }
>     
>     
>     
>     "sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU="
>     

> **Example**
> 
> Convert a SHA256 hash in SRI to Base16:
>     
>     
>     builtins.convertHash {
>       hash = "sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=";
>       toHashFormat = "base16";
>     }
>     
>     
>     
>     "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
>     

> **Example**
> 
> Convert a hash in the form `<hashAlgo>:<hashBody>` in Base16 to SRI:
>     
>     
>     builtins.convertHash {
>       hash = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
>       toHashFormat = "sri";
>     }
>     
>     
>     
>     "sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU="
>     

`currentSystem` (string) 
    

The value of the [`eval-system`](</manual/nix/2.34/command-ref/conf-file#conf-eval-system>) or else [`system`](</manual/nix/2.34/command-ref/conf-file#conf-system>) configuration option.

It can be used to set the `system` attribute for [`builtins.derivation`](</manual/nix/2.34/language/derivations>) such that the resulting derivation can be built on the same system that evaluates the Nix expression:
    
    
     builtins.derivation {
       # ...
       system = builtins.currentSystem;
    }
    

It can be overridden in order to create derivations for different system than the current one:
    
    
    $ nix-instantiate --system "mips64-linux" --eval --expr 'builtins.currentSystem'
    "mips64-linux"
    

> **Note**
> 
> Not available in [pure evaluation mode](</manual/nix/2.34/command-ref/conf-file#conf-pure-eval>).

`currentTime` (integer) 
    

Return the [Unix time](<https://en.wikipedia.org/wiki/Unix_time>) at first evaluation. Repeated references to that name re-use the initially obtained value.

Example:
    
    
    $ nix repl
    Welcome to Nix 2.15.1 Type :? for help.
    
    nix-repl> builtins.currentTime
    1683705525
    
    nix-repl> builtins.currentTime
    1683705525
    

The [store path](</manual/nix/2.34/store/store-path>) of a derivation depending on `currentTime` differs for each evaluation, unless both evaluate `builtins.currentTime` in the same second.

> **Note**
> 
> Not available in [pure evaluation mode](</manual/nix/2.34/command-ref/conf-file#conf-pure-eval>).

`deepSeq e1 e2`
    

This is like `seq e1 e2`, except that _e1_ is evaluated _deeply_ : if it’s a list or set, its elements or attributes are also evaluated recursively.

`dirOf s`
    

Return the directory part of the string _s_ , that is, everything before the final slash in the string. This is similar to the GNU `dirname` command.

`div e1 e2`
    

Return the quotient of the numbers _e1_ and _e2_.

`elem x xs`
    

Return `true` if a value equal to _x_ occurs in the list _xs_ , and `false` otherwise.

`elemAt xs n`
    

Return element _n_ from the list _xs_. Elements are counted starting from 0. A fatal error occurs if the index is out of bounds.

`false` (Boolean) 
    

Primitive value.

It can be returned by [comparison operators](</manual/nix/2.34/language/operators#comparison>) and used in [conditional expressions](</manual/nix/2.34/language/syntax#conditionals>).

The name `false` is not special, and can be shadowed:
    
    
    nix-repl> let false = 1; in false
    1
    

`fetchClosure args`
    

> **Note**
> 
> This function is only available if the [`fetch-closure` experimental feature](</manual/nix/2.34/development/experimental-features#xp-feature-fetch-closure>) is enabled.
> 
> For example, include the following in [`nix.conf`](</manual/nix/2.34/command-ref/conf-file>):
>     
>     
>     extra-experimental-features = fetch-closure
>     

Fetch a store path [closure](</manual/nix/2.34/glossary#gloss-closure>) from a binary cache, and return the store path as a string with context.

This function can be invoked in three ways that we will discuss in order of preference.

**Fetch a content-addressed store path**

Example:
    
    
    builtins.fetchClosure {
      fromStore = "https://cache.nixos.org";
      fromPath = /nix/store/ldbhlwhh39wha58rm61bkiiwm6j7211j-git-2.33.1;
    }
    

This is the simplest invocation, and it does not require the user of the expression to configure [`trusted-public-keys`](</manual/nix/2.34/command-ref/conf-file#conf-trusted-public-keys>) to ensure their authenticity.

If your store path is [input addressed](</manual/nix/2.34/glossary#gloss-input-addressed-store-object>) instead of content addressed, consider the other two invocations.

**Fetch any store path and rewrite it to a fully content-addressed store path**

Example:
    
    
    builtins.fetchClosure {
      fromStore = "https://cache.nixos.org";
      fromPath = /nix/store/nph9br6y2dmciy6q3dj3fwk2brdlr4gh-git-2.33.1;
      toPath = /nix/store/ldbhlwhh39wha58rm61bkiiwm6j7211j-git-2.33.1;
    }
    

This example fetches `/nix/store/r2jd...` from the specified binary cache, and rewrites it into the content-addressed store path `/nix/store/ldbh...`.

Like the previous example, no extra configuration or privileges are required.

To find out the correct value for `toPath` given a `fromPath`, use [`nix store make-content-addressed`](</manual/nix/2.34/command-ref/new-cli/nix3-store-make-content-addressed>):
    
    
    # nix store make-content-addressed --from https://cache.nixos.org /nix/store/nph9br6y2dmciy6q3dj3fwk2brdlr4gh-git-2.33.1
    rewrote '/nix/store/nph9br6y2dmciy6q3dj3fwk2brdlr4gh-git-2.33.1' to '/nix/store/ldbhlwhh39wha58rm61bkiiwm6j7211j-git-2.33.1'
    

Alternatively, set `toPath = ""` and find the correct `toPath` in the error message.

**Fetch an input-addressed store path as is**

Example:
    
    
    builtins.fetchClosure {
      fromStore = "https://cache.nixos.org";
      fromPath = /nix/store/nph9br6y2dmciy6q3dj3fwk2brdlr4gh-git-2.33.1;
      inputAddressed = true;
    }
    

It is possible to fetch an [input-addressed store path](</manual/nix/2.34/glossary#gloss-input-addressed-store-object>) and return it as is. However, this is the least preferred way of invoking `fetchClosure`, because it requires that the input-addressed paths are trusted by the Nix configuration.

**`builtins.storePath`**

`fetchClosure` is similar to `builtins.storePath` in that it allows you to use a previously built store path in a Nix expression. However, `fetchClosure` is more reproducible because it specifies a binary cache from which the path can be fetched. Also, using content-addressed store paths does not require users to configure [`trusted-public-keys`](</manual/nix/2.34/command-ref/conf-file#conf-trusted-public-keys>) to ensure their authenticity.

`fetchGit args`
    

Fetch a path from git. _args_ can be a URL, in which case the HEAD of the repo at that URL is fetched. Otherwise, it can be an attribute with the following attributes (all except `url` optional):

  * `url`

The URL of the repo.

  * `name` (default: `source`)

The name of the directory the repo should be exported to in the store.

  * `rev` (default: _the tip of`ref`_)

The [Git revision](<https://git-scm.com/docs/git-rev-parse#_specifying_revisions>) to fetch. This is typically a commit hash.

  * `ref` (default: `HEAD`)

The [Git reference](<https://git-scm.com/book/en/v2/Git-Internals-Git-References>) under which to look for the requested revision. This is often a branch or tag name.

This option has no effect once `shallow` cloning is enabled.

By default, the `ref` value is prefixed with `refs/heads/`. As of 2.3.0, Nix doesn't prefix `refs/heads/` if `ref` starts with `refs/`.

  * `submodules` (default: `false`)

A Boolean parameter that specifies whether submodules should be checked out.

  * `exportIgnore` (default: `true`)

A Boolean parameter that specifies whether `export-ignore` from `.gitattributes` should be applied. This approximates part of the `git archive` behavior.

Enabling this option is not recommended because it is unknown whether the Git developers commit to the reproducibility of `export-ignore` in newer Git versions.

  * `shallow` (default: `false`)

Make a shallow clone when fetching the Git tree. When this is enabled, the options `ref` and `allRefs` have no effect anymore.

  * `lfs` (default: `false`)

A boolean that when `true` specifies that [Git LFS](<https://git-lfs.com/>) files should be fetched.

  * `allRefs`

Whether to fetch all references (eg. branches and tags) of the repository. With this argument being true, it's possible to load a `rev` from _any_ `ref`. (by default only `rev`s from the specified `ref` are supported).

This option has no effect once `shallow` cloning is enabled.

  * `verifyCommit` (default: `true` if `publicKey` or `publicKeys` are provided, otherwise `false`)

Whether to check `rev` for a signature matching `publicKey` or `publicKeys`. If `verifyCommit` is enabled, then `fetchGit` cannot use a local repository with uncommitted changes. Requires the [`verified-fetches` experimental feature](</manual/nix/2.34/development/experimental-features#xp-feature-verified-fetches>).

  * `publicKey`

The public key against which `rev` is verified if `verifyCommit` is enabled. Requires the [`verified-fetches` experimental feature](</manual/nix/2.34/development/experimental-features#xp-feature-verified-fetches>).

  * `keytype` (default: `"ssh-ed25519"`)

The key type of `publicKey`. Possible values:

    * `"ssh-dsa"`
    * `"ssh-ecdsa"`
    * `"ssh-ecdsa-sk"`
    * `"ssh-ed25519"`
    * `"ssh-ed25519-sk"`
    * `"ssh-rsa"` Requires the [`verified-fetches` experimental feature](</manual/nix/2.34/development/experimental-features#xp-feature-verified-fetches>).
  * `publicKeys`

The public keys against which `rev` is verified if `verifyCommit` is enabled. Must be given as a list of attribute sets with the following form:
        
        {
          key = "<public key>";
          type = "<key type>"; # optional, default: "ssh-ed25519"
        }
        

Requires the [`verified-fetches` experimental feature](</manual/nix/2.34/development/experimental-features#xp-feature-verified-fetches>).

Here are some examples of how to use `fetchGit`.

  * To fetch a private repository over SSH:
        
        builtins.fetchGit {
          url = "git@github.com:my-secret/repository.git";
          ref = "master";
          rev = "adab8b916a45068c044658c4158d81878f9ed1c3";
        }
        

  * To fetch an arbitrary reference:
        
        builtins.fetchGit {
          url = "https://github.com/NixOS/nix.git";
          ref = "refs/heads/0.5-release";
        }
        

  * If the revision you're looking for is in the default branch of the git repository you don't strictly need to specify the branch name in the `ref` attribute.

However, if the revision you're looking for is in a future branch for the non-default branch you will need to specify the the `ref` attribute as well.
        
        builtins.fetchGit {
          url = "https://github.com/nixos/nix.git";
          rev = "841fcbd04755c7a2865c51c1e2d3b045976b7452";
          ref = "1.11-maintenance";
        }
        

> **Note**
> 
> It is nice to always specify the branch which a revision belongs to. Without the branch being specified, the fetcher might fail if the default branch changes. Additionally, it can be confusing to try a commit from a non-default branch and see the fetch fail. If the branch is specified the fault is much more obvious.

  * If the revision you're looking for is in the default branch of the git repository you may omit the `ref` attribute.
        
        builtins.fetchGit {
          url = "https://github.com/nixos/nix.git";
          rev = "841fcbd04755c7a2865c51c1e2d3b045976b7452";
        }
        

  * To fetch a specific tag:
        
        builtins.fetchGit {
          url = "https://github.com/nixos/nix.git";
          ref = "refs/tags/1.9";
        }
        

  * To fetch the latest version of a remote branch:
        
        builtins.fetchGit {
          url = "ssh://git@github.com/nixos/nix.git";
          ref = "master";
        }
        

  * To verify the commit signature:
        
        builtins.fetchGit {
          url = "ssh://git@github.com/nixos/nix.git";
          verifyCommit = true;
          publicKeys = [
              {
                type = "ssh-ed25519";
                key = "AAAAC3NzaC1lZDI1NTE5AAAAIArPKULJOid8eS6XETwUjO48/HKBWl7FTCK0Z//fplDi";
              }
          ];
        }
        

Nix refetches the branch according to the [`tarball-ttl`](</manual/nix/2.34/command-ref/conf-file#conf-tarball-ttl>) setting.

This behavior is disabled in [pure evaluation mode](</manual/nix/2.34/command-ref/conf-file#conf-pure-eval>).

  * To fetch the content of a checked-out work directory:
        
        builtins.fetchGit ./work-dir
        

If the URL points to a local directory, and no `ref` or `rev` is given, `fetchGit` uses the current content of the checked-out files, even if they are not committed or added to Git's index. It only considers files added to the Git repository, as listed by `git ls-files`.

`fetchTarball args`
    

Download the specified URL, unpack it and return the path of the unpacked tree. The file must be a tape archive (`.tar`) compressed with `gzip`, `bzip2` or `xz`. If the tarball consists of a single directory, then the top-level path component of the files in the tarball is removed. The typical use of the function is to obtain external Nix expression dependencies, such as a particular version of Nixpkgs, e.g.
    
    
    with import (fetchTarball https://github.com/NixOS/nixpkgs/archive/nixos-14.12.tar.gz) {};
    
    stdenv.mkDerivation { … }
    

The fetched tarball is cached for a certain amount of time (1 hour by default) in `~/.cache/nix/tarballs/`. You can change the cache timeout either on the command line with `--tarball-ttl` _number-of-seconds_ or in the Nix configuration file by adding the line `tarball-ttl = ` _number-of-seconds_.

Note that when obtaining the hash with `nix-prefetch-url` the option `--unpack` is required.

This function can also verify the contents against a hash. In that case, the function takes a set instead of a URL. The set requires the attribute `url` and the attribute `sha256`, e.g.
    
    
    with import (fetchTarball {
      url = "https://github.com/NixOS/nixpkgs/archive/nixos-14.12.tar.gz";
      sha256 = "1jppksrfvbk5ypiqdz4cddxdl8z6zyzdb2srq8fcffr327ld5jj2";
    }) {};
    
    stdenv.mkDerivation { … }
    

Not available in [restricted evaluation mode](</manual/nix/2.34/command-ref/conf-file#conf-restrict-eval>).

`fetchTree input`
    

> **Note**
> 
> This function is only available if the [`fetch-tree` experimental feature](</manual/nix/2.34/development/experimental-features#xp-feature-fetch-tree>) is enabled.
> 
> For example, include the following in [`nix.conf`](</manual/nix/2.34/command-ref/conf-file>):
>     
>     
>     extra-experimental-features = fetch-tree
>     

Fetch a file system tree or a plain file using one of the supported backends and return an attribute set with:

  * the resulting fixed-output [store path](</manual/nix/2.34/store/store-path>)
  * the corresponding [NAR](</manual/nix/2.34/store/file-system-object/content-address#serial-nix-archive>) hash
  * backend-specific metadata (currently not documented). 

_input_ must be an attribute set with the following attributes:

  * `type` (String, required)

One of the supported source types. This determines other required and allowed input attributes.

  * `narHash` (String, optional)

The `narHash` parameter can be used to substitute the source of the tree. It also allows for verification of tree contents that may not be provided by the underlying transfer mechanism. If `narHash` is set, the source is first looked up is the Nix store and [substituters](</manual/nix/2.34/command-ref/conf-file#conf-substituters>), and only fetched if not available.

A subset of the output attributes of `fetchTree` can be re-used for subsequent calls to `fetchTree` to produce the same result again. That is, `fetchTree` is idempotent.

Downloads are cached in `$XDG_CACHE_HOME/nix`. The remote source is fetched from the network if both are true:

  * A NAR hash is supplied and the corresponding store path is not [valid](</manual/nix/2.34/glossary#gloss-validity>), that is, not available in the store

> **Note**
> 
> [Substituters](</manual/nix/2.34/command-ref/conf-file#conf-substituters>) are not used in fetching.

  * There is no cache entry or the cache entry is older than [`tarball-ttl`](</manual/nix/2.34/command-ref/conf-file#conf-tarball-ttl>)

## Source types

The following source types and associated input attributes are supported.

  * `"file"`

Place a plain file into the Nix store. This is similar to [`builtins.fetchurl`](</manual/nix/2.34/language/builtins#builtins-fetchurl>)

    * `lastModified` (String, required)

    * `name` (String, required)

    * `narHash` (String, required)

    * `rev` (String, required)

    * `revCount` (String, required)

    * `unpack` (String, required)

    * `url` (String, required)

Supported protocols:

      * `https`

> **Example**
>             
>             fetchTree {
>               type = "file";
>               url = "https://example.com/index.html";
>             }
>             

      * `http`

Insecure HTTP transfer for legacy sources.

> **Warning**
> 
> HTTP performs no encryption or authentication. Use a `narHash` known in advance to ensure the output has expected contents.

      * `file`

A file on the local file system.

> **Example**
>             
>             fetchTree {
>               type = "file";
>               url = "file:///home/eelco/nix/README.md";
>             }
>             

  * `"git"`

Fetch a Git tree and copy it to the Nix store. This is similar to [`builtins.fetchGit`](</manual/nix/2.34/language/builtins#builtins-fetchGit>).

    * `allRefs` (Bool, optional)

By default, this has no effect. This becomes relevant only once `shallow` cloning is disabled.

Whether to fetch all references (eg. branches and tags) of the repository. With this argument being true, it's possible to load a `rev` from _any_ `ref`. (Without setting this option, only `rev`s from the specified `ref` are supported).

Default: `false`

    * `dirtyRev` (String, required)

    * `dirtyShortRev` (String, required)

    * `exportIgnore` (String, required)

    * `keytype` (String, required)

    * `lastModified` (Integer, optional)

Unix timestamp of the fetched commit.

If set, pass through the value to the output attribute set. Otherwise, generated from the fetched Git tree.

    * `lfs` (Bool, optional)

Fetch any [Git LFS](<https://git-lfs.com/>) files.

Default: `false`

    * `name` (String, required)

    * `narHash` (String, required)

    * `publicKey` (String, required)

    * `publicKeys` (String, required)

    * `ref` (String, optional)

By default, this has no effect. This becomes relevant only once `shallow` cloning is disabled.

A [Git reference](<https://git-scm.com/book/en/v2/Git-Internals-Git-References>), such as a branch or tag name.

Default: `"HEAD"`

    * `rev` (String, optional)

A Git revision; a commit hash.

Default: the tip of `ref`

    * `revCount` (Integer, optional)

Number of revisions in the history of the Git repository before the fetched commit.

If set, pass through the value to the output attribute set. Otherwise, generated from the fetched Git tree.

    * `shallow` (Bool, optional)

Make a shallow clone when fetching the Git tree. When this is enabled, the options `ref` and `allRefs` have no effect anymore.

Default: `true`

    * `submodules` (Bool, optional)

Also fetch submodules if available.

Default: `false`

    * `url` (String, required)

The URL formats supported are the same as for Git itself.

> **Example**
>           
>           fetchTree {
>             type = "git";
>             url = "git@github.com:NixOS/nixpkgs.git";
>           }
>           

> **Note**
> 
> If the URL points to a local directory, and no `ref` or `rev` is given, Nix only considers files added to the Git index, as listed by `git ls-files` but uses the _current file contents_ of the Git working directory.

    * `verifyCommit` (String, required)

  * `"github"`

    * `host` (String, required)

    * `lastModified` (String, required)

    * `narHash` (String, required)

    * `owner` (String, required)

    * `ref` (String, required)

    * `repo` (String, required)

    * `rev` (String, required)

    * `treeHash` (String, required)

  * `"gitlab"`

    * `host` (String, required)

    * `lastModified` (String, required)

    * `narHash` (String, required)

    * `owner` (String, required)

    * `ref` (String, required)

    * `repo` (String, required)

    * `rev` (String, required)

    * `treeHash` (String, required)

  * `"hg"`

    * `name` (String, required)

    * `narHash` (String, required)

    * `ref` (String, required)

    * `rev` (String, required)

    * `revCount` (String, required)

    * `url` (String, required)

  * `"indirect"`

    * `id` (String, required)

    * `narHash` (String, required)

    * `ref` (String, required)

    * `rev` (String, required)

  * `"path"`

    * `lastModified` (String, required)

    * `narHash` (String, required)

    * `path` (String, required)

    * `rev` (String, required)

    * `revCount` (String, required)

  * `"sourcehut"`

    * `host` (String, required)

    * `lastModified` (String, required)

    * `narHash` (String, required)

    * `owner` (String, required)

    * `ref` (String, required)

    * `repo` (String, required)

    * `rev` (String, required)

    * `treeHash` (String, required)

  * `"tarball"`

Download a tar archive and extract it into the Nix store. This has the same underlying implementation as [`builtins.fetchTarball`](</manual/nix/2.34/language/builtins#builtins-fetchTarball>)

    * `lastModified` (String, required)

    * `name` (String, required)

    * `narHash` (String, required)

    * `rev` (String, required)

    * `revCount` (String, required)

    * `unpack` (String, required)

    * `url` (String, required)

> **Example**
>           
>           fetchTree {
>             type = "tarball";
>             url = "https://github.com/NixOS/nixpkgs/tarball/nixpkgs-23.11";
>           }
>           

The following input types are still subject to change:

  * `"path"`
  * `"github"`
  * `"gitlab"`
  * `"sourcehut"`
  * `"mercurial"`

_input_ can also be a [URL-like reference](</manual/nix/2.34/command-ref/new-cli/nix3-flake#flake-references>). The additional input types and the URL-like syntax requires the [`flakes` experimental feature](</manual/nix/2.34/development/experimental-features#xp-feature-flakes>) to be enabled.

> **Example**
> 
> Fetch a GitHub repository using the attribute set representation:
>     
>     
>     builtins.fetchTree {
>       type = "github";
>       owner = "NixOS";
>       repo = "nixpkgs";
>       rev = "ae2e6b3958682513d28f7d633734571fb18285dd";
>     }
>     
> 
> This evaluates to the following attribute set:
>     
>     
>     {
>       lastModified = 1686503798;
>       lastModifiedDate = "20230611171638";
>       narHash = "sha256-rA9RqKP9OlBrgGCPvfd5HVAXDOy8k2SmPtB/ijShNXc=";
>       outPath = "/nix/store/l5m6qlvfs9sdw14ja3qbzpglcjlb6j1x-source";
>       rev = "ae2e6b3958682513d28f7d633734571fb18285dd";
>       shortRev = "ae2e6b3";
>     }
>     

> **Example**
> 
> Fetch the same GitHub repository using the URL-like syntax:
>     
>     
>     builtins.fetchTree "github:NixOS/nixpkgs/ae2e6b3958682513d28f7d633734571fb18285dd"
>     

`fetchurl arg`
    

Download the specified URL and return the path of the downloaded file. `arg` can be either a string denoting the URL, or an attribute set with the following attributes:

  * `url`

The URL of the file to download.

  * `name` (default: the last path component of the URL)

A name for the file in the store. This can be useful if the URL has any characters that are invalid for the store.

Not available in [restricted evaluation mode](</manual/nix/2.34/command-ref/conf-file#conf-restrict-eval>).

`filter f list`
    

Return a list consisting of the elements of _list_ for which the function _f_ returns `true`.

`filterSource e1 e2`
    

> **Warning**
> 
> `filterSource` should not be used to filter store paths. Since `filterSource` uses the name of the input directory while naming the output directory, doing so produces a directory name in the form of `<hash2>-<hash>-<name>`, where `<hash>-<name>` is the name of the input directory. Since `<hash>` depends on the unfiltered directory, the name of the output directory indirectly depends on files that are filtered out by the function. This triggers a rebuild even when a filtered out file is changed. Use `builtins.path` instead, which allows specifying the name of the output directory.

This function allows you to copy sources into the Nix store while filtering certain files. For instance, suppose that you want to use the directory `source-dir` as an input to a Nix expression, e.g.
    
    
    stdenv.mkDerivation {
      ...
      src = ./source-dir;
    }
    

However, if `source-dir` is a Subversion working copy, then all of those annoying `.svn` subdirectories are also copied to the store. Worse, the contents of those directories may change a lot, causing lots of spurious rebuilds. With `filterSource` you can filter out the `.svn` directories:
    
    
    src = builtins.filterSource
      (path: type: type != "directory" || baseNameOf path != ".svn")
      ./source-dir;
    

Thus, the first argument _e1_ must be a predicate function that is called for each regular file, directory or symlink in the source tree _e2_. If the function returns `true`, the file is copied to the Nix store, otherwise it is omitted. The function is called with two arguments. The first is the full path of the file. The second is a string that identifies the type of the file, which is either `"regular"`, `"directory"`, `"symlink"` or `"unknown"` (for other kinds of files such as device nodes or fifos — but note that those cannot be copied to the Nix store, so if the predicate returns `true` for them, the copy fails). If you exclude a directory, the entire corresponding subtree of _e2_ is excluded.

`findFile search-path lookup-path`
    

Find _lookup-path_ in _search-path_.

[Lookup path](</manual/nix/2.34/language/constructs/lookup-path>) expressions are [desugared](<https://en.wikipedia.org/wiki/Syntactic_sugar>) using this and `builtins.nixPath`:
    
    
    <nixpkgs>
    

is equivalent to:
    
    
    builtins.findFile builtins.nixPath "nixpkgs"
    

A search path is represented as a list of [attribute sets](</manual/nix/2.34/language/types#type-attrs>) with two attributes:

  * `prefix` is a relative path.
  * `path` denotes a file system location

Examples of search path attribute sets:

  * {
          prefix = "";
          path = "/nix/var/nix/profiles/per-user/root/channels";
        }
        

  * {
          prefix = "nixos-config";
          path = "/etc/nixos/configuration.nix";
        }
        

  * {
          prefix = "nixpkgs";
          path = "https://github.com/NixOS/nixpkgs/tarballs/master";
        }
        

  * {
          prefix = "nixpkgs";
          path = "channel:nixpkgs-unstable";
        }
        

  * {
          prefix = "flake-compat";
          path = "flake:github:edolstra/flake-compat";
        }
        

The lookup algorithm checks each entry until a match is found, returning a [path value](</manual/nix/2.34/language/types#type-path>) of the match:

  * If a prefix of `lookup-path` matches `prefix`, then the remainder of _lookup-path_ (the "suffix") is searched for within the directory denoted by `path`. The contents of `path` may need to be downloaded at this point to look inside.

  * If the suffix is found inside that directory, then the entry is a match. The combined absolute path of the directory (now downloaded if need be) and the suffix is returned.

> **Example**
> 
> A _search-path_ value
>     
>     
>     [
>       {
>         prefix = "";
>         path = "/home/eelco/Dev";
>       }
>       {
>         prefix = "nixos-config";
>         path = "/etc/nixos";
>       }
>     ]
>     
> 
> and a _lookup-path_ value `"nixos-config"` causes Nix to try `/home/eelco/Dev/nixos-config` and `/etc/nixos` in that order and return the first path that exists.

If `path` starts with `http://` or `https://`, it is interpreted as the URL of a tarball to be downloaded and unpacked to a temporary location. The tarball must consist of a single top-level directory.

The URLs of the tarballs from the official `nixos.org` channels can be abbreviated as `channel:<channel-name>`. See [documentation on `nix-channel`](</manual/nix/2.34/command-ref/nix-channel>) for details about channels.

> **Example**
> 
> These two search path entries are equivalent:
> 
>   * {
>           prefix = "nixpkgs";
>           path = "channel:nixpkgs-unstable";
>         }
>         
> 
>   * {
>           prefix = "nixpkgs";
>           path = "https://channels.nixos.org/nixos-unstable/nixexprs.tar.xz";
>         }
>         
> 
> 

Search paths can also point to source trees using [flake URLs](</manual/nix/2.34/command-ref/new-cli/nix3-flake#url-like-syntax>).

> **Example**
> 
> The search path entry
>     
>     
>     {
>       prefix = "nixpkgs";
>       path = "flake:nixpkgs";
>     }
>     
> 
> specifies that the prefix `nixpkgs` shall refer to the source tree downloaded from the `nixpkgs` entry in the flake registry.
> 
> Similarly
>     
>     
>     {
>       prefix = "nixpkgs";
>       path = "flake:github:nixos/nixpkgs/nixos-22.05";
>     }
>     
> 
> makes `<nixpkgs>` refer to a particular branch of the `NixOS/nixpkgs` repository on GitHub.

`flakeRefToString attrs`
    

> **Note**
> 
> This function is only available if the [`flakes` experimental feature](</manual/nix/2.34/development/experimental-features#xp-feature-flakes>) is enabled.
> 
> For example, include the following in [`nix.conf`](</manual/nix/2.34/command-ref/conf-file>):
>     
>     
>     extra-experimental-features = flakes
>     

Convert a flake reference from attribute set format to URL format.

For example:
    
    
    builtins.flakeRefToString {
      dir = "lib"; owner = "NixOS"; ref = "23.05"; repo = "nixpkgs"; type = "github";
    }
    

evaluates to
    
    
    "github:NixOS/nixpkgs/23.05?dir=lib"
    

`floor number`
    

Rounds and converts _number_ to the next lower NixInt value if possible, i.e. `floor *number* <= *number*` and `*number* - floor *number* < 1`.

An evaluation error is thrown, if there exists no such NixInt value `floor *number*`. Due to bugs in previous Nix versions an evaluation error might be thrown, if the datatype of _number_ is a NixInt and if `*number* < -9007199254740992` or `*number* > 9007199254740992`.

If the datatype of _number_ is neither a NixInt (signed 64-bit integer) nor a NixFloat (IEEE-754 double-precision floating-point number), an evaluation error will be thrown.

`foldl' op nul list`
    

Reduce a list by applying a binary operator, from left to right, e.g. `foldl' op nul [x0 x1 x2 ...] = op (op (op nul x0) x1) x2) ...`.

For example, `foldl' (acc: elem: acc + elem) 0 [1 2 3]` evaluates to `6` and `foldl' (acc: elem: { "${elem}" = elem; } // acc) {} ["a" "b"]` evaluates to `{ a = "a"; b = "b"; }`.

The first argument of `op` is the accumulator whereas the second argument is the current element being processed. The return value of each application of `op` is evaluated immediately, even for intermediate values.

`fromJSON e`
    

Convert a JSON string to a Nix value. For example,
    
    
    builtins.fromJSON ''{"x": [1, 2, 3], "y": null}''
    

returns the value `{ x = [ 1 2 3 ]; y = null; }`.

`fromTOML e`
    

Convert a TOML string to a Nix value. For example,
    
    
    builtins.fromTOML ''
      x=1
      s="a"
      [table]
      y=2
    ''
    

returns the value `{ s = "a"; table = { y = 2; }; x = 1; }`.

`functionArgs f`
    

Return a set containing the names of the formal arguments expected by the function _f_. The value of each attribute is a Boolean denoting whether the corresponding argument has a default value. For instance, `functionArgs ({ x, y ? 123}: ...) = { x = false; y = true; }`.

"Formal argument" here refers to the attributes pattern-matched by the function. Plain lambdas are not included, e.g. `functionArgs (x: ...) = { }`.

`genList generator length`
    

Generate list of size _length_ , with each element _i_ equal to the value returned by _generator_ `i`. For example,
    
    
    builtins.genList (x: x * x) 5
    

returns the list `[ 0 1 4 9 16 ]`.

`genericClosure attrset`
    

`builtins.genericClosure` iteratively computes the transitive closure over an arbitrary relation defined by a function.

It takes _attrset_ with two attributes named `startSet` and `operator`, and returns a list of attribute sets:

  * `startSet`: The initial list of attribute sets.

  * `operator`: A function that takes an attribute set and returns a list of attribute sets. It defines how each item in the current set is processed and expanded into more items.

Each attribute set in the list `startSet` and the list returned by `operator` must have an attribute `key`, which must support equality comparison. The value of `key` can be one of the following types:

  * [Int](</manual/nix/2.34/language/types#type-int>)
  * [Float](</manual/nix/2.34/language/types#type-float>)
  * [Boolean](</manual/nix/2.34/language/types#type-bool>)
  * [String](</manual/nix/2.34/language/types#type-string>)
  * [Path](</manual/nix/2.34/language/types#type-path>)
  * [List](</manual/nix/2.34/language/types#type-list>)

The result is produced by calling the `operator` on each `item` that has not been called yet, including newly added items, until no new items are added. Items are compared by their `key` attribute.

Common usages are:

  * Generating unique collections of items, such as dependency graphs.
  * Traversing through structures that may contain cycles or loops.
  * Processing data structures with complex internal relationships.

> **Example**
>     
>     
>     builtins.genericClosure {
>       startSet = [ {key = 5;} ];
>       operator = item: [{
>         key = if (item.key / 2 ) * 2 == item.key
>              then item.key / 2
>              else 3 * item.key + 1;
>       }];
>     }
>     
> 
> evaluates to
>     
>     
>     [ { key = 5; } { key = 16; } { key = 8; } { key = 4; } { key = 2; } { key = 1; } ]
>     

`getAttr s set`
    

`getAttr` returns the attribute named _s_ from _set_. Evaluation aborts if the attribute doesn’t exist. This is a dynamic version of the `.` operator, since _s_ is an expression rather than an identifier.

`getContext s`
    

Return the string context of _s_.

The string context tracks references to derivations within a string. It is represented as an attribute set of [store derivation](</manual/nix/2.34/glossary#gloss-store-derivation>) paths mapping to output names.

Using [string interpolation](</manual/nix/2.34/language/string-interpolation>) on a derivation adds that derivation to the string context. For example,
    
    
    builtins.getContext "${derivation { name = "a"; builder = "b"; system = "c"; }}"
    

evaluates to
    
    
    { "/nix/store/arhvjaf6zmlyn8vh8fgn55rpwnxq0n7l-a.drv" = { outputs = [ "out" ]; }; }
    

`getEnv s`
    

`getEnv` returns the value of the environment variable _s_ , or an empty string if the variable doesn’t exist. This function should be used with care, as it can introduce all sorts of nasty environment dependencies in your Nix expression.

`getEnv` is used in Nix Packages to locate the file `~/.nixpkgs/config.nix`, which contains user-local settings for Nix Packages. (That is, it does a `getEnv "HOME"` to locate the user’s home directory.)

`getFlake args`
    

> **Note**
> 
> This function is only available if the [`flakes` experimental feature](</manual/nix/2.34/development/experimental-features#xp-feature-flakes>) is enabled.
> 
> For example, include the following in [`nix.conf`](</manual/nix/2.34/command-ref/conf-file>):
>     
>     
>     extra-experimental-features = flakes
>     

Fetch a flake from a flake reference, and return its output attributes and some metadata. For example:
    
    
    (builtins.getFlake "nix/55bc52401966fbffa525c574c14f67b00bc4fb3a").packages.x86_64-linux.nix
    

Unless impure evaluation is allowed (`--impure`), the flake reference must be "locked", e.g. contain a Git revision or content hash. An example of an unlocked usage is:
    
    
    (builtins.getFlake "github:edolstra/dwarffs").rev
    

`groupBy f list`
    

Groups elements of _list_ together by the string returned from the function _f_ called on each element. It returns an attribute set where each attribute value contains the elements of _list_ that are mapped to the same corresponding attribute name returned by _f_.

For example,
    
    
    builtins.groupBy (builtins.substring 0 1) ["foo" "bar" "baz"]
    

evaluates to
    
    
    { b = [ "bar" "baz" ]; f = [ "foo" ]; }
    

`hasAttr s set`
    

`hasAttr` returns `true` if _set_ has an attribute named _s_ , and `false` otherwise. This is a dynamic version of the `?` operator, since _s_ is an expression rather than an identifier.

`hasContext s`
    

Return `true` if string _s_ has a non-empty context. The context can be obtained with `getContext`.

> **Example**
> 
> Many operations require a string context to be empty because they are intended only to work with "regular" strings, and also to help users avoid unintentionally loosing track of string context elements. `builtins.hasContext` can help create better domain-specific errors in those case.
>     
>     
>     name: meta:
>     
>     if builtins.hasContext name
>     then throw "package name cannot contain string context"
>     else { ${name} = meta; }
>     

`hashFile type p`
    

Return a base-16 representation of the cryptographic hash of the file at path _p_. The hash algorithm specified by _type_ must be one of `"md5"`, `"sha1"`, `"sha256"` or `"sha512"`.

`hashString type s`
    

Return a base-16 representation of the cryptographic hash of string _s_. The hash algorithm specified by _type_ must be one of `"md5"`, `"sha1"`, `"sha256"` or `"sha512"`.

`head list`
    

Return the first element of a list; abort evaluation if the argument isn’t a list or is an empty list. You can test whether a list is empty by comparing it with `[]`.

`import path`
    

Load, parse, and return the Nix expression in the file _path_.

> **Note**
> 
> Unlike some languages, `import` is a regular function in Nix.

The _path_ argument must meet the same criteria as an [interpolated expression](</manual/nix/2.34/language/string-interpolation#interpolated-expression>).

If _path_ is a directory, the file `default.nix` in that directory is used if it exists.

> **Example**
>     
>     
>     $ echo 123 > default.nix
>     
> 
> Import `default.nix` from the current directory.
>     
>     
>     import ./.
>     
>     
>     
>     123
>     

Evaluation aborts if the file doesn’t exist or contains an invalid Nix expression.

A Nix expression loaded by `import` must not contain any _free variables_ , that is, identifiers that are not defined in the Nix expression itself and are not built-in. Therefore, it cannot refer to variables that are in scope at the call site.

> **Example**
> 
> If you have a calling expression
>     
>     
>     rec {
>       x = 123;
>       y = import ./foo.nix;
>     }
>     
> 
> then the following `foo.nix` throws an error:
>     
>     
>     # foo.nix
>     x + 456
>     
> 
> since `x` is not in scope in `foo.nix`. If you want `x` to be available in `foo.nix`, pass it as a function argument:
>     
>     
>     rec {
>       x = 123;
>       y = import ./foo.nix x;
>     }
>     
> 
> and
>     
>     
>     # foo.nix
>     x: x + 456
>     
> 
> The function argument doesn’t have to be called `x` in `foo.nix`; any name would work.

`intersectAttrs e1 e2`
    

Return a set consisting of the attributes in the set _e2_ which have the same name as some attribute in _e1_.

Performs in O(_n_ log _m_) where _n_ is the size of the smaller set and _m_ the larger set's size.

`isAttrs e`
    

Return `true` if _e_ evaluates to a set, and `false` otherwise.

`isBool e`
    

Return `true` if _e_ evaluates to a bool, and `false` otherwise.

`isFloat e`
    

Return `true` if _e_ evaluates to a float, and `false` otherwise.

`isFunction e`
    

Return `true` if _e_ evaluates to a function, and `false` otherwise.

`isInt e`
    

Return `true` if _e_ evaluates to an integer, and `false` otherwise.

`isList e`
    

Return `true` if _e_ evaluates to a list, and `false` otherwise.

`isNull e`
    

Return `true` if _e_ evaluates to `null`, and `false` otherwise.

This is equivalent to `e == null`.

`isPath e`
    

Return `true` if _e_ evaluates to a path, and `false` otherwise.

`isString e`
    

Return `true` if _e_ evaluates to a string, and `false` otherwise.

`langVersion` (integer) 
    

The current version of the Nix language.

`length e`
    

Return the length of the list _e_.

`lessThan e1 e2`
    

Return `true` if the value _e1_ is less than the value _e2_ , and `false` otherwise. Evaluation aborts if either _e1_ or _e2_ does not evaluate to a number, string or path. Furthermore, it aborts if _e2_ does not match _e1_ 's type according to the aforementioned classification of number, string or path.

`listToAttrs e`
    

Construct a set from a list specifying the names and values of each attribute. Each element of the list should be a set consisting of a string-valued attribute `name` specifying the name of the attribute, and an attribute `value` specifying its value.

In case of duplicate occurrences of the same name, the first takes precedence.

Example:
    
    
    builtins.listToAttrs
      [ { name = "foo"; value = 123; }
        { name = "bar"; value = 456; }
        { name = "bar"; value = 420; }
      ]
    

evaluates to
    
    
    { foo = 123; bar = 456; }
    

`map f list`
    

Apply the function _f_ to each element in the list _list_. For example,
    
    
    map (x: "foo" + x) [ "bar" "bla" "abc" ]
    

evaluates to `[ "foobar" "foobla" "fooabc" ]`.

`mapAttrs f attrset`
    

Apply function _f_ to every element of _attrset_. For example,
    
    
    builtins.mapAttrs (name: value: value * 10) { a = 1; b = 2; }
    

evaluates to `{ a = 10; b = 20; }`.

`match regex str`
    

Returns a list if the [extended POSIX regular expression](<http://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap09.html#tag_09_04>) _regex_ matches _str_ precisely, otherwise returns `null`. Each item in the list is a regex group.
    
    
    builtins.match "ab" "abc"
    

Evaluates to `null`.
    
    
    builtins.match "abc" "abc"
    

Evaluates to `[ ]`.
    
    
    builtins.match "a(b)(c)" "abc"
    

Evaluates to `[ "b" "c" ]`.
    
    
    builtins.match "[[:space:]]+([[:upper:]]+)[[:space:]]+" "  FOO   "
    

Evaluates to `[ "FOO" ]`.

`mul e1 e2`
    

Return the product of the numbers _e1_ and _e2_.

`nixPath` (list) 
    

A list of search path entries used to resolve [lookup paths](</manual/nix/2.34/language/constructs/lookup-path>). Its value is primarily determined by the [`nix-path` configuration setting](</manual/nix/2.34/command-ref/conf-file#conf-nix-path>), which are

  * Overridden by the [`NIX_PATH`](</manual/nix/2.34/command-ref/env-common#env-NIX_PATH>) environment variable or the `--nix-path` option
  * Extended by the [`-I` option](</manual/nix/2.34/command-ref/opt-common#opt-I>) or `--extra-nix-path`

> **Example**
>     
>     
>     $ NIX_PATH= nix-instantiate --eval --expr "builtins.nixPath" -I foo=bar --no-pure-eval
>     [ { path = "bar"; prefix = "foo"; } ]
>     

Lookup path expressions are [desugared](<https://en.wikipedia.org/wiki/Syntactic_sugar>) using this and [`builtins.findFile`](</manual/nix/2.34/language/builtins#builtins-findFile>):
    
    
    <nixpkgs>
    

is equivalent to:
    
    
    builtins.findFile builtins.nixPath "nixpkgs"
    

`nixVersion` (string) 
    

The version of Nix.

For example, where the command line returns the current Nix version,
    
    
    $ nix --version
    nix (Nix) 2.16.0
    

the Nix language evaluator returns the same value:
    
    
    nix-repl> builtins.nixVersion
    "2.16.0"
    

`null` (null) 
    

Primitive value.

The name `null` is not special, and can be shadowed:
    
    
    nix-repl> let null = 1; in null
    1
    

`outputOf derivation-reference output-name`
    

> **Note**
> 
> This function is only available if the [`dynamic-derivations` experimental feature](</manual/nix/2.34/development/experimental-features#xp-feature-dynamic-derivations>) is enabled.
> 
> For example, include the following in [`nix.conf`](</manual/nix/2.34/command-ref/conf-file>):
>     
>     
>     extra-experimental-features = dynamic-derivations
>     

Return the output path of a derivation, literally or using an [input placeholder string](</manual/nix/2.34/store/derivation/#input-placeholder>) if needed.

If the derivation has a statically-known output path (i.e. the derivation output is input-addressed, or fixed content-addressed), the output path is returned. But if the derivation is content-addressed or if the derivation is itself not-statically produced (i.e. is the output of another derivation), an input placeholder is returned instead.

_`derivation reference`_ must be a string that may contain a regular store path to a derivation, or may be an input placeholder reference. If the derivation is produced by a derivation, you must explicitly select `drv.outPath`. This primop can be chained arbitrarily deeply. For instance,
    
    
    builtins.outputOf
      (builtins.outputOf myDrv "out")
      "out"
    

returns an input placeholder for the output of the output of `myDrv`.

This primop corresponds to the `^` sigil for [deriving paths](</manual/nix/2.34/glossary#gloss-deriving-path>), e.g. as part of installable syntax on the command line.

`parseDrvName s`
    

Split the string _s_ into a package name and version. The package name is everything up to but not including the first dash not followed by a letter, and the version is everything following that dash. The result is returned in a set `{ name, version }`. Thus, `builtins.parseDrvName "nix-0.12pre12876"` returns `{ name = "nix"; version = "0.12pre12876"; }`.

`parseFlakeRef flake-ref`
    

> **Note**
> 
> This function is only available if the [`flakes` experimental feature](</manual/nix/2.34/development/experimental-features#xp-feature-flakes>) is enabled.
> 
> For example, include the following in [`nix.conf`](</manual/nix/2.34/command-ref/conf-file>):
>     
>     
>     extra-experimental-features = flakes
>     

Parse a flake reference, and return its exploded form.

For example:
    
    
    builtins.parseFlakeRef "github:NixOS/nixpkgs/23.05?dir=lib"
    

evaluates to:
    
    
    { dir = "lib"; owner = "NixOS"; ref = "23.05"; repo = "nixpkgs"; type = "github"; }
    

`partition pred list`
    

Given a predicate function _pred_ , this function returns an attrset containing a list named `right`, containing the elements in _list_ for which _pred_ returned `true`, and a list named `wrong`, containing the elements for which it returned `false`. For example,
    
    
    builtins.partition (x: x > 10) [1 23 9 3 42]
    

evaluates to
    
    
    { right = [ 23 42 ]; wrong = [ 1 9 3 ]; }
    

`path args`
    

An enrichment of the built-in path type, based on the attributes present in _args_. All are optional except `path`:

  * path  
The underlying path.

  * name  
The name of the path when added to the store. This can used to reference paths that have nix-illegal characters in their names, like `@`.

  * filter  
A function of the type expected by `builtins.filterSource`, with the same semantics.

  * recursive  
When `false`, when `path` is added to the store it is with a [flat hash](</manual/nix/2.34/store/file-system-object/content-address#serial-flat>), rather than a hash of the [NAR serialization](</manual/nix/2.34/store/file-system-object/content-address#serial-nix-archive>) of the file. Thus, `path` must refer to a regular file, not a directory. This allows similar behavior to `fetchurl`. Defaults to `true`.

  * sha256  
When provided, this is the expected [content hash](</manual/nix/2.34/store/file-system-object/content-address>) of the path. Evaluation fails if the hash is incorrect, and providing a hash allows `builtins.path` to be used even when the `pure-eval` nix config option is on.

`pathExists path`
    

Return `true` if the path _path_ exists at evaluation time, and `false` otherwise.

`placeholder output`
    

Return an [output placeholder string](</manual/nix/2.34/store/derivation/#output-placeholder>) for the specified _output_ that will be substituted by the corresponding [output path](</manual/nix/2.34/glossary#gloss-output-path>) at build time.

Typical outputs would be `"out"`, `"bin"` or `"dev"`.

`readDir path`
    

Return the contents of the directory _path_ as a set mapping directory entries to the corresponding file type. For instance, if directory `A` contains a regular file `B` and another directory `C`, then `builtins.readDir ./A` returns the set
    
    
    { B = "regular"; C = "directory"; }
    

The possible values for the file type are `"regular"`, `"directory"`, `"symlink"` and `"unknown"`.

`readFile path`
    

Return the contents of the file _path_ as a string.

`readFileType p`
    

Determine the directory entry type of a filesystem node, being one of `"directory"`, `"regular"`, `"symlink"`, or `"unknown"`.

`removeAttrs set list`
    

Remove the attributes listed in _list_ from _set_. The attributes don’t have to exist in _set_. For instance,
    
    
    removeAttrs { x = 1; y = 2; z = 3; } [ "a" "x" "z" ]
    

evaluates to `{ y = 2; }`.

`replaceStrings from to s`
    

Given string _s_ , replace every occurrence of the strings in _from_ with the corresponding string in _to_.

The argument _to_ is lazy, that is, it is only evaluated when its corresponding pattern in _from_ is matched in the string _s_

Example:
    
    
    builtins.replaceStrings ["oo" "a"] ["a" "i"] "foobar"
    

evaluates to `"fabir"`.

`scopedImport scope path`
    

Load, parse, and return the Nix expression in the file _path_ , with the attributes from _scope_ available as variables in the lexical scope of the imported file.

This function is similar to `import`, but allows you to provide additional variables that will be available in the scope of the imported expression. The _scope_ argument must be an attribute set; each attribute becomes a variable available in the imported file. Built-in functions and values remain accessible unless shadowed by _scope_ attributes.

> **Note**
> 
> Variables from _scope_ shadow built-ins with the same name, allowing you to override built-ins for the imported expression.

> **Note**
> 
> Unlike `import`, `scopedImport` does not memoize evaluation results. While the parsing result may be reused, each call produces a distinct value. This is observable through performance and side effects such as `builtins.trace`.

The _path_ argument must meet the same criteria as an [interpolated expression](</manual/nix/2.34/language/string-interpolation#interpolated-expression>).

If _path_ is a directory, the file `default.nix` in that directory is used if it exists.

> **Example**
> 
> Create a file `greet.nix`:
>     
>     
>     # greet.nix
>     "${greeting}, ${name}!"
>     
> 
> Import it with additional variables in scope:
>     
>     
>     scopedImport { greeting = "Hello"; name = "World"; } ./greet.nix
>     
>     
>     
>     "Hello, World!"
>     

Evaluation aborts if the file doesn't exist or contains an invalid Nix expression.

`seq e1 e2`
    

Evaluate _e1_ , then evaluate and return _e2_. This ensures that a computation is strict in the value of _e1_.

`sort comparator list`
    

Return _list_ in sorted order. It repeatedly calls the function _comparator_ with two elements. The comparator should return `true` if the first element is less than the second, and `false` otherwise. For example,
    
    
    builtins.sort builtins.lessThan [ 483 249 526 147 42 77 ]
    

produces the list `[ 42 77 147 249 483 526 ]`.

This is a stable sort: it preserves the relative order of elements deemed equal by the comparator.

_comparator_ must impose a strict weak ordering on the set of values in the _list_. This means that for any elements _a_ , _b_ and _c_ from the _list_ , _comparator_ must satisfy the following relations:

  1. Transitivity

If a is less than b and b is less than c, then it follows that a is less than c.
    
    
    comparator a b && comparator b c -> comparator a c
    

  1. Irreflexivity

    
    
    comparator a a == false
    

  1. Transitivity of equivalence

First, two values a and b are considered equivalent with respect to the comparator if:
    
    
    !comparator a b && !comparator b a
    

In other words, neither is considered "less than" the other.

Transitivity of equivalence means:

If a is equivalent to b, and b is equivalent to c, then a must also be equivalent to c.
    
    
    let
      equiv = x: y: (!comparator x y && !comparator y x);
    in
      equiv a b && equiv b c -> equiv a c
    

If the _comparator_ violates any of these properties, then `builtins.sort` reorders elements in an unspecified manner.

`split regex str`
    

Returns a list composed of non matched strings interleaved with the lists of the [extended POSIX regular expression](<http://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap09.html#tag_09_04>) _regex_ matches of _str_. Each item in the lists of matched sequences is a regex group.
    
    
    builtins.split "(a)b" "abc"
    

Evaluates to `[ "" [ "a" ] "c" ]`.
    
    
    builtins.split "([ac])" "abc"
    

Evaluates to `[ "" [ "a" ] "b" [ "c" ] "" ]`.
    
    
    builtins.split "(a)|(c)" "abc"
    

Evaluates to `[ "" [ "a" null ] "b" [ null "c" ] "" ]`.
    
    
    builtins.split "([[:upper:]]+)" " FOO "
    

Evaluates to `[ " " [ "FOO" ] " " ]`.

`splitVersion s`
    

Split a string representing a version into its components, by the same version splitting logic underlying the version comparison in [`nix-env -u`](</manual/nix/2.34/command-ref/nix-env/upgrade>).

`storeDir` (string) 
    

Logical file system location of the [Nix store](</manual/nix/2.34/glossary#gloss-store>) currently in use.

This value is determined by the `store` parameter in [Store URLs](</manual/nix/2.34/store/types/#store-url-format>):
    
    
    $ nix-instantiate --store 'dummy://?store=/blah' --eval --expr builtins.storeDir
    "/blah"
    

`storePath path`
    

This function allows you to define a dependency on an already existing store path. For example, the derivation attribute `src = builtins.storePath /nix/store/f1d18v1y…-source` causes the derivation to depend on the specified path, which must exist or be substitutable. Note that this differs from a plain path (e.g. `src = /nix/store/f1d18v1y…-source`) in that the latter causes the path to be _copied_ again to the Nix store, resulting in a new path (e.g. `/nix/store/ld01dnzc…-source-source`).

Not available in [pure evaluation mode](</manual/nix/2.34/command-ref/conf-file#conf-pure-eval>).

See also `builtins.fetchClosure`.

`stringLength e`
    

Return the number of bytes of the string _e_. If _e_ is not a string, evaluation is aborted.

`sub e1 e2`
    

Return the difference between the numbers _e1_ and _e2_.

`substring start len s`
    

Return the substring of _s_ from byte position _start_ (zero-based) up to but not including _start + len_. If _start_ is greater than the length of the string, an empty string is returned. If _start + len_ lies beyond the end of the string or _len_ is `-1`, only the substring up to the end of the string is returned. _start_ must be non-negative. For example,
    
    
    builtins.substring 0 3 "nixos"
    

evaluates to `"nix"`.

`tail list`
    

Return the list without its first item; abort evaluation if the argument isn’t a list or is an empty list.

> **Warning**
> 
> This function should generally be avoided since it's inefficient: unlike Haskell's `tail`, it takes O(n) time, so recursing over a list by repeatedly calling `tail` takes O(n^2) time.

`throw s`
    

Throw an error message _s_. This usually aborts Nix expression evaluation, but in `nix-env -qa` and other commands that try to evaluate a set of derivations to get information about those derivations, a derivation that throws an error is silently skipped (which is not the case for `abort`).

`toFile name s`
    

Store the string _s_ in a file in the Nix store and return its path. The file has suffix _name_. This file can be used as an input to derivations. One application is to write builders “inline”. For instance, the following Nix expression combines the Nix expression for GNU Hello and its build script into one file:
    
    
    { stdenv, fetchurl, perl }:
    
    stdenv.mkDerivation {
      name = "hello-2.1.1";
    
      builder = builtins.toFile "builder.sh" "
        source $stdenv/setup
    
        PATH=$perl/bin:$PATH
    
        tar xvfz $src
        cd hello-*
        ./configure --prefix=$out
        make
        make install
      ";
    
      src = fetchurl {
        url = "http://ftp.nluug.nl/pub/gnu/hello/hello-2.1.1.tar.gz";
        sha256 = "1md7jsfd8pa45z73bz1kszpp01yw6x5ljkjk2hx7wl800any6465";
      };
      inherit perl;
    }
    

It is even possible for one file to refer to another, e.g.,
    
    
    builder = let
      configFile = builtins.toFile "foo.conf" "
        # This is some dummy configuration file.
        ...
      ";
    in builtins.toFile "builder.sh" "
      source $stdenv/setup
      ...
      cp ${configFile} $out/etc/foo.conf
    ";
    

Note that `${configFile}` is a [string interpolation](</manual/nix/2.34/language/types#type-string>), so the result of the expression `configFile` (i.e., a path like `/nix/store/m7p7jfny445k...-foo.conf`) will be spliced into the resulting string.

It is however _not_ allowed to have files mutually referring to each other, like so:
    
    
    let
      foo = builtins.toFile "foo" "...${bar}...";
      bar = builtins.toFile "bar" "...${foo}...";
    in foo
    

This is not allowed because it would cause a cyclic dependency in the computation of the cryptographic hashes for `foo` and `bar`.

It is also not possible to reference the result of a derivation. If you are using Nixpkgs, the `writeTextFile` function is able to do that.

`toJSON e`
    

Return a string containing a JSON representation of _e_. Strings, integers, floats, booleans, nulls and lists are mapped to their JSON equivalents. Sets (except derivations) are represented as objects. Derivations are translated to a JSON string containing the derivation’s output path. Paths are copied to the store and represented as a JSON string of the resulting store path.

`toPath s`
    

**DEPRECATED.** Use `/. + "/path"` to convert a string into an absolute path. For relative paths, use `./. + "/path"`.

`toString e`
    

Convert the expression _e_ to a string. _e_ can be:

  * A string (in which case the string is returned unmodified).

  * A path (e.g., `toString /foo/bar` yields `"/foo/bar"`.

  * A set containing `{ __toString = self: ...; }` or `{ outPath = ...; }`.

  * An integer.

  * A list, in which case the string representations of its elements are joined with spaces.

  * A Boolean (`false` yields `""`, `true` yields `"1"`).

  * `null`, which yields the empty string.

`toXML e`
    

Return a string containing an XML representation of _e_. The main application for `toXML` is to communicate information with the builder in a more structured format than plain environment variables.

Here is an example where this is the case:
    
    
    { stdenv, fetchurl, libxslt, jira, uberwiki }:
    
    stdenv.mkDerivation (rec {
      name = "web-server";
    
      buildInputs = [ libxslt ];
    
      builder = builtins.toFile "builder.sh" "
        source $stdenv/setup
        mkdir $out
        echo "$servlets" | xsltproc ${stylesheet} - > $out/server-conf.xml ①
      ";
    
      stylesheet = builtins.toFile "stylesheet.xsl" ②
       "<?xml version='1.0' encoding='UTF-8'?>
        <xsl:stylesheet xmlns:xsl='http://www.w3.org/1999/XSL/Transform' version='1.0'>
          <xsl:template match='/'>
            <Configure>
              <xsl:for-each select='/expr/list/attrs'>
                <Call name='addWebApplication'>
                  <Arg><xsl:value-of select=\"attr[@name = 'path']/string/@value\" /></Arg>
                  <Arg><xsl:value-of select=\"attr[@name = 'war']/path/@value\" /></Arg>
                </Call>
              </xsl:for-each>
            </Configure>
          </xsl:template>
        </xsl:stylesheet>
      ";
    
      servlets = builtins.toXML [ ③
        { path = "/bugtracker"; war = jira + "/lib/atlassian-jira.war"; }
        { path = "/wiki"; war = uberwiki + "/uberwiki.war"; }
      ];
    })
    

The builder is supposed to generate the configuration file for a [Jetty servlet container](<http://jetty.mortbay.org/>). A servlet container contains a number of servlets (`*.war` files) each exported under a specific URI prefix. So the servlet configuration is a list of sets containing the `path` and `war` of the servlet (①). This kind of information is difficult to communicate with the normal method of passing information through an environment variable, which just concatenates everything together into a string (which might just work in this case, but wouldn’t work if fields are optional or contain lists themselves). Instead the Nix expression is converted to an XML representation with `toXML`, which is unambiguous and can easily be processed with the appropriate tools. For instance, in the example an XSLT stylesheet (at point ②) is applied to it (at point ①) to generate the XML configuration file for the Jetty server. The XML representation produced at point ③ by `toXML` is as follows:
    
    
    <?xml version='1.0' encoding='utf-8'?>
    <expr>
      <list>
        <attrs>
          <attr name="path">
            <string value="/bugtracker" />
          </attr>
          <attr name="war">
            <path value="/nix/store/d1jh9pasa7k2...-jira/lib/atlassian-jira.war" />
          </attr>
        </attrs>
        <attrs>
          <attr name="path">
            <string value="/wiki" />
          </attr>
          <attr name="war">
            <path value="/nix/store/y6423b1yi4sx...-uberwiki/uberwiki.war" />
          </attr>
        </attrs>
      </list>
    </expr>
    

Note that we used the `toFile` built-in to write the builder and the stylesheet “inline” in the Nix expression. The path of the stylesheet is spliced into the builder using the syntax `xsltproc ${stylesheet}`.

`trace e1 e2`
    

Evaluate _e1_ and print its abstract syntax representation on standard error. Then return _e2_. This function is useful for debugging.

If the [`debugger-on-trace`](</manual/nix/2.34/command-ref/conf-file#conf-debugger-on-trace>) option is set to `true` and the `--debugger` flag is given, the interactive debugger is started when `trace` is called (like [`break`](</manual/nix/2.34/language/builtins#builtins-break>)).

`traceVerbose e1 e2`
    

Evaluate _e1_ and print its abstract syntax representation on standard error if `--trace-verbose` is enabled. Then return _e2_. This function is useful for debugging.

`true` (Boolean) 
    

Primitive value.

It can be returned by [comparison operators](</manual/nix/2.34/language/operators#comparison>) and used in [conditional expressions](</manual/nix/2.34/language/syntax#conditionals>).

The name `true` is not special, and can be shadowed:
    
    
    nix-repl> let true = 1; in true
    1
    

`tryEval e`
    

Try to shallowly evaluate _e_. Return a set containing the attributes `success` (`true` if _e_ evaluated successfully, `false` if an error was thrown) and `value`, equalling _e_ if successful and `false` otherwise. `tryEval` only prevents errors created by `throw` or `assert` from being thrown. Errors `tryEval` doesn't catch are, for example, those created by `abort` and type errors generated by builtins. Also note that this doesn't evaluate _e_ deeply, so `let e = { x = throw ""; }; in (builtins.tryEval e).success` is `true`. Using `builtins.deepSeq` one can get the expected result: `let e = { x = throw ""; }; in (builtins.tryEval (builtins.deepSeq e e)).success` is `false`.

`tryEval` intentionally does not return the error message, because that risks bringing non-determinism into the evaluation result, and it would become very difficult to improve error reporting without breaking existing expressions. Instead, use [`builtins.addErrorContext`](</manual/nix/2.34/language/builtins#builtins-addErrorContext>) to add context to the error message, and use a Nix unit testing tool for testing.

`typeOf e`
    

Return a string representing the type of the value _e_ , namely `"int"`, `"bool"`, `"string"`, `"path"`, `"null"`, `"set"`, `"list"`, `"lambda"` or `"float"`.

`unsafeDiscardOutputDependency s`
    

Create a copy of the given string where every [derivation deep](</manual/nix/2.34/language/string-context#string-context-element-derivation-deep>) string context element is turned into a [constant](</manual/nix/2.34/language/string-context#string-context-constant>) string context element.

This is the opposite of `builtins.addDrvOutputDependencies`.

This is unsafe because it allows us to "forget" store objects we would have otherwise referred to with the string context, whereas Nix normally tracks all dependencies consistently. Safe operations "grow" but never "shrink" string contexts. `builtins.addDrvOutputDependencies` in contrast is safe because "derivation deep" string context element always refers to the underlying derivation (among many more things). Replacing a constant string context element with a "derivation deep" element is a safe operation that just enlargens the string context without forgetting anything.

`unsafeDiscardStringContext s`
    

Discard the [string context](</manual/nix/2.34/language/string-context>) from a value that can be coerced to a string.

`unsafeGetAttrPos s set`
    

`unsafeGetAttrPos` returns the position of the attribute named _s_ from _set_. This is used by Nixpkgs to provide location information in error messages.

`warn e1 e2`
    

Evaluate _e1_ , which must be a string, and print it on standard error as a warning. Then return _e2_. This function is useful for non-critical situations where attention is advisable.

If the [`debugger-on-trace`](</manual/nix/2.34/command-ref/conf-file#conf-debugger-on-trace>) or [`debugger-on-warn`](</manual/nix/2.34/command-ref/conf-file#conf-debugger-on-warn>) option is set to `true` and the `--debugger` flag is given, the interactive debugger will be started when `warn` is called (like [`break`](</manual/nix/2.34/language/builtins#builtins-break>)).

If the [`abort-on-warn`](</manual/nix/2.34/command-ref/conf-file#conf-abort-on-warn>) option is set, the evaluation is aborted after the warning is printed. This is useful to reveal the stack trace of the warning, when the context is non-interactive and a debugger can not be launched.

`zipAttrsWith f list`
    

Transpose a list of attribute sets into an attribute set of lists, then apply `mapAttrs`.

`f` receives two arguments: the attribute name and a non-empty list of all values encountered for that attribute name.

The result is an attribute set where the attribute names are the union of the attribute names in each element of `list`. The attribute values are the return values of `f`.
    
    
    builtins.zipAttrsWith
      (name: values: { inherit name values; })
      [ { a = "x"; } { a = "y"; b = "z"; } ]
    

evaluates to
    
    
    {
      a = { name = "a"; values = [ "x" "y" ]; };
      b = { name = "b"; values = [ "z" ]; };
    }
