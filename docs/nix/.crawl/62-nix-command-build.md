---
type: Crawl Source
title: "Nix Command Reference — nix build"
description: "Reference for the nix build command."
resource: https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-build
tags: [nix, nix-manual, command-ref, nix-build]
timestamp: 2026-07-24T00:00:00Z
---

# Crawl: nix-command-build

- seed_url: https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-build
- canonical_url: https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-build
- family: Nix Manual
- fetch: 200
- version: Nix 2.34
- feeds_docs: nix-commands.md

## Content

> **Warning**   
>  This program is [**experimental**](</manual/nix/2.34/development/experimental-features#xp-feature-nix-command>) and its interface is subject to change.

# Name

`nix build` \- build a derivation or fetch a store path

# Synopsis

`nix build` [_option_...] _installables_...

# Examples

  * Build the default package from the flake in the current directory:
        
        # nix build
        

  * Build and run GNU Hello from the `nixpkgs` flake:
        
        # nix build nixpkgs#hello
        # ./result/bin/hello
        Hello, world!
        

  * Build GNU Hello and Cowsay, leaving two result symlinks:
        
        # nix build nixpkgs#hello nixpkgs#cowsay
        # ls -l result*
        lrwxrwxrwx 1 … result -> /nix/store/10l19qifk7hjjq47px8m2prqk1gv4isy-hello-2.10
        lrwxrwxrwx 1 … result-1 -> /nix/store/frzgk3v1ycnarpfc2rkynravng27a86d-cowsay-3.03+dfsg2
        

  * Build GNU Hello and print the resulting store path.
        
        # nix build nixpkgs#hello --print-out-paths
        /nix/store/10l19qifk7hjjq47px8m2prqk1gv4isy-hello-2.10
        

  * Build a specific output:
        
        # nix build nixpkgs#glibc.dev
        # ls -ld ./result-dev
        lrwxrwxrwx 1 … ./result-dev -> /nix/store/hb4lb9n3gv855llky72hrs4pglpxq70m-glibc-2.32-dev
        

  * Build all outputs:
        
        # nix build "nixpkgs#openssl^*" --print-out-paths
        /nix/store/ah1slww3lfsj02w563wjf1xcz5fayj36-openssl-3.0.13-bin
        /nix/store/vswlynn75s0bpba3vl6bi3wyzjym95yi-openssl-3.0.13-debug
        /nix/store/z71nwwni9dcxdmd3v3a7j24v70c7v7z3-openssl-3.0.13-dev
        /nix/store/iabzsa5c73p4f10zfmf5r2qsrn0hl4lk-openssl-3.0.13-doc
        /nix/store/zqmfrpxvcll69a2lyawnpvp15zh421v2-openssl-3.0.13-man
        /nix/store/l3nlzki957anyy7yb25qvwk6cqrnvb67-openssl-3.0.13
        

  * Build attribute `build.x86_64-linux` from (non-flake) Nix expression `release.nix`:
        
        # nix build --file release.nix build.x86_64-linux
        

  * Build a NixOS system configuration from a flake, and make a profile point to the result:
        
        # nix build --profile /nix/var/nix/profiles/system \
            ~/my-configurations#nixosConfigurations.machine.config.system.build.toplevel
        

(This is essentially what `nixos-rebuild` does.)

  * Build an expression specified on the command line:
        
        # nix build --impure --expr \
            'with import <nixpkgs> {};
             runCommand "foo" {
               buildInputs = [ hello ];
             }
             "hello > $out"'
        # cat ./result
        Hello, world!
        

Note that `--impure` is needed because we're using `<nixpkgs>`, which relies on the `$NIX_PATH` environment variable.

  * Fetch a store path from the configured substituters, if it doesn't already exist:
        
        # nix build /nix/store/frzgk3v1ycnarpfc2rkynravng27a86d-cowsay-3.03+dfsg2
        

# Description

`nix build` builds the specified _installables_. [Installables](</manual/nix/2.34/command-ref/new-cli/nix#installables>) that resolve to derivations are built (or substituted if possible). Store path installables are substituted.

Unless `--no-link` is specified, after a successful build, it creates symlinks to the store paths of the installables. These symlinks have the prefix `./result` by default; this can be overridden using the `--out-link` option. Each symlink has a suffix `-<N>-<outname>`, where _N_ is the index of the installable (with the left-most installable having index 0), and _outname_ is the symbolic derivation output name (e.g. `bin`, `dev` or `lib`). `-<N>` is omitted if _N_ = 0, and `-<outname>` is omitted if _outname_ = `out` (denoting the default output).

# Options

  * `--dry-run`

Show what this command would do without doing it.

  * `--json`

Produce output in JSON format, suitable for consumption by another program.

  * `--no-link`

Do not create symlinks to the build results.

  * `--no-pretty`

Print compact JSON output on a single line, even when the output is a terminal. Some commands may print multiple JSON objects on separate lines.
        
        See `--pretty`.
        

  * `--out-link` / `-o` _path_

Use _path_ as prefix for the symlinks to the build results. It defaults to `result`.

  * `--pretty`

Print multi-line, indented JSON output for readability.
        
        Default: indent if output is to a terminal.
        
                      This option is only effective when `--json` is also specified.
        

  * `--print-out-paths`

Print the resulting output paths

  * `--profile` _path_

The profile to operate on.

  * `--rebuild`

Rebuild an already built package and compare the result to the existing store paths.

  * `--stdin`

Read installables from the standard input. No default installable applied.

## Common evaluation options

  * `--arg` _name_ _expr_

Pass the value _expr_ as the argument _name_ to Nix functions.

  * `--arg-from-file` _name_ _path_

Pass the contents of file _path_ as the argument _name_ to Nix functions.

  * `--arg-from-stdin` _name_

Pass the contents of stdin as the argument _name_ to Nix functions.

  * `--argstr` _name_ _string_

Pass the string _string_ as the argument _name_ to Nix functions.

  * `--debugger`

Start an interactive environment if evaluation fails.

  * `--eval-store` _store-url_

The [URL of the Nix store](</manual/nix/2.34/store/types/#store-url-format>) to use for evaluation, i.e. to store derivations (`.drv` files) and inputs referenced by them.

  * `--impure`

Allow access to mutable paths and repositories.

  * `--include` / `-I` _path_

Add _path_ to search path entries used to resolve [lookup paths](</manual/nix/2.34/language/constructs/lookup-path>)

This option may be given multiple times.

Paths added through `-I` take precedence over the [`nix-path` configuration setting](</manual/nix/2.34/command-ref/conf-file#conf-nix-path>) and the [`NIX_PATH` environment variable](</manual/nix/2.34/command-ref/env-common#env-NIX_PATH>).

  * `--override-flake` _original-ref_ _resolved-ref_

Override the flake registries, redirecting _original-ref_ to _resolved-ref_.

## Common flake-related options

  * `--commit-lock-file`

Commit changes to the flake's lock file.

  * `--inputs-from` _flake-url_

Use the inputs of the specified flake as registry entries.

  * `--no-registries`

Don't allow lookups in the flake registries.

> **DEPRECATED**
> 
> Use [`--no-use-registries`](</manual/nix/2.34/command-ref/conf-file#conf-use-registries>) instead.

  * `--no-update-lock-file`

Do not allow any updates to the flake's lock file.

  * `--no-write-lock-file`

Do not write the flake's newly generated lock file.

  * `--output-lock-file` _flake-lock-path_

Write the given lock file instead of `flake.lock` within the top-level flake.

  * `--override-input` _input-path_ _flake-url_

Override a specific flake input (e.g. `dwarffs/nixpkgs`). The input path must not be empty. This implies `--no-write-lock-file`.

  * `--recreate-lock-file`

Recreate the flake's lock file from scratch.

> **DEPRECATED**
> 
> Use [`nix flake update`](</manual/nix/2.34/command-ref/new-cli/nix3-flake-update>) instead.

  * `--reference-lock-file` _flake-lock-path_

Read the given lock file instead of `flake.lock` within the top-level flake.

  * `--update-input` _input-path_

Update a specific flake input (ignoring its previous entry in the lock file).

> **DEPRECATED**
> 
> Use [`nix flake update`](</manual/nix/2.34/command-ref/new-cli/nix3-flake-update>) instead.

## Logging-related options

  * `--debug`

Set the logging verbosity level to 'debug'.

  * `--log-format` _format_

Set the format of log output; one of `raw`, `internal-json`, `bar` or `bar-with-logs`.

  * `--print-build-logs` / `-L`

Print full build logs on standard error.

  * `--quiet`

Decrease the logging verbosity level.

  * `--verbose` / `-v`

Increase the logging verbosity level.

## Miscellaneous global options

  * `--help`

Show usage information.

  * `--offline`

Disable substituters and consider all previously downloaded files up-to-date.

  * `--option` _name_ _value_

Set the Nix configuration setting _name_ to _value_ (overriding `nix.conf`).

  * `--refresh`

Consider all previously downloaded files out-of-date.

  * `--repair`

During evaluation, rewrite missing or corrupted files in the Nix store. During building, rebuild missing or corrupted store paths.

  * `--version`

Show version information.

## Options that change the interpretation of [installables](</manual/nix/2.34/command-ref/new-cli/nix#installables>)

  * `--expr` _expr_

Interpret [_installables_](</manual/nix/2.34/command-ref/new-cli/nix#installables>) as attribute paths relative to the Nix expression _expr_.

  * `--file` / `-f` _file_

Interpret [_installables_](</manual/nix/2.34/command-ref/new-cli/nix#installables>) as attribute paths relative to the Nix expression stored in _file_. If _file_ is the character -, then a Nix expression is read from standard input. Implies `--impure`.

> **Note**
> 
> See [`man nix.conf`](</manual/nix/2.34/command-ref/conf-file#command-line-flags>) for overriding configuration settings with command line flags.
