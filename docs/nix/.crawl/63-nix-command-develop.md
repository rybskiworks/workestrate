---
type: Crawl Source
title: "Nix Command Reference — nix develop"
description: "Reference for the nix develop command."
resource: https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-develop
tags: [nix, nix-manual, command-ref, nix-develop]
timestamp: 2026-07-24T00:00:00Z
---

# Crawl: nix-command-develop

- seed_url: https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-develop
- canonical_url: https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-develop
- family: Nix Manual
- fetch: 200
- version: Nix 2.34
- feeds_docs: nix-commands.md

## Content

> **Warning**   
>  This program is [**experimental**](</manual/nix/2.34/development/experimental-features#xp-feature-nix-command>) and its interface is subject to change.

# Name

`nix develop` \- run a bash shell that provides the build environment of a derivation

# Synopsis

`nix develop` [_option_...] _installable_

# Examples

  * Start a shell with the build environment of the default package of the flake in the current directory:
        
        # nix develop
        

Typical commands to run inside this shell are:
        
        # configurePhase
        # buildPhase
        # installPhase
        

Alternatively, you can run whatever build tools your project uses directly, e.g. for a typical Unix project:
        
        # ./configure --prefix=$out
        # make
        # make install
        

  * Run a particular build phase directly:
        
        # nix develop --unpack
        # nix develop --configure
        # nix develop --build
        # nix develop --check
        # nix develop --install
        # nix develop --installcheck
        

  * Start a shell with the build environment of GNU Hello:
        
        # nix develop nixpkgs#hello
        

  * Record a build environment in a profile:
        
        # nix develop --profile /tmp/my-build-env nixpkgs#hello
        

  * Use a build environment previously recorded in a profile:
        
        # nix develop /tmp/my-build-env
        

  * Replace all occurrences of the store path corresponding to `glibc.dev` with a writable directory:
        
        # nix develop --redirect nixpkgs#glibc.dev ~/my-glibc/outputs/dev
        

Note that this is useful if you're running a `nix develop` shell for `nixpkgs#glibc` in `~/my-glibc` and want to compile another package against it.

  * Run a series of script commands:
        
        # nix develop --command bash -c "mkdir build && cmake .. && make"
        

# Description

`nix develop` starts a `bash` shell that provides an interactive build environment nearly identical to what Nix would use to build [_installable_](</manual/nix/2.34/command-ref/new-cli/nix#installables>). Inside this shell, environment variables and shell functions are set up so that you can interactively and incrementally build your package.

Nix determines the build environment by building a modified version of the derivation _installable_ that just records the environment initialised by `stdenv` and exits. This build environment can be recorded into a profile using `--profile`.

The prompt used by the `bash` shell can be customised by setting the `bash-prompt`, `bash-prompt-prefix`, and `bash-prompt-suffix` settings in `nix.conf` or in the flake's `nixConfig` attribute.

# Flake output attributes

If no flake output attribute is given, `nix develop` tries the following flake output attributes:

  * `devShells.<system>.default`

  * `packages.<system>.default`

If a flake output _name_ is given, `nix develop` tries the following flake output attributes:

  * `devShells.<system>.<name>`

  * `packages.<system>.<name>`

  * `legacyPackages.<system>.<name>`

# Options

  * `--build`

Run the `build` phase.

  * `--check`

Run the `check` phase.

  * `--command` / `-c` _command_ _args_

Instead of starting an interactive shell, start the specified command and arguments.

  * `--configure`

Run the `configure` phase.

  * `--install`

Run the `install` phase.

  * `--installcheck`

Run the `installcheck` phase.

  * `--phase` _phase-name_

The stdenv phase to run (e.g. `build` or `configure`).

  * `--profile` _path_

The profile to operate on.

  * `--redirect` _installable_ _outputs-dir_

Redirect a store path to a mutable location.

  * `--unpack`

Run the `unpack` phase.

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

## Options that change environment variables

  * `--ignore-env` / `-i`

Clear the entire environment, except for those specified with `--keep-env-var`.

  * `--keep-env-var` / `-k` _name_

Keep the environment variable _name_ , when using `--ignore-env`.

  * `--set-env-var` / `-s` _name_ _value_

Sets an environment variable _name_ with _value_.

  * `--unset-env-var` / `-u` _name_

Unset the environment variable _name_.

## Options that change the interpretation of [installables](</manual/nix/2.34/command-ref/new-cli/nix#installables>)

  * `--expr` _expr_

Interpret [_installables_](</manual/nix/2.34/command-ref/new-cli/nix#installables>) as attribute paths relative to the Nix expression _expr_.

  * `--file` / `-f` _file_

Interpret [_installables_](</manual/nix/2.34/command-ref/new-cli/nix#installables>) as attribute paths relative to the Nix expression stored in _file_. If _file_ is the character -, then a Nix expression is read from standard input. Implies `--impure`.

> **Note**
> 
> See [`man nix.conf`](</manual/nix/2.34/command-ref/conf-file#command-line-flags>) for overriding configuration settings with command line flags.
