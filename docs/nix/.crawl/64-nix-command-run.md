---
type: Crawl Source
title: "Nix Command Reference — nix run"
description: "Reference for the nix run command."
resource: https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-run
tags: [nix, nix-manual, command-ref, nix-run]
timestamp: 2026-07-24T00:00:00Z
---

# Crawl: nix-command-run

- seed_url: https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-run
- canonical_url: https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-run
- family: Nix Manual
- fetch: 200
- version: Nix 2.34
- feeds_docs: nix-commands.md

## Content

> **Warning**   
>  This program is [**experimental**](</manual/nix/2.34/development/experimental-features#xp-feature-nix-command>) and its interface is subject to change.

# Name

`nix run` \- run a Nix application

# Synopsis

`nix run` [_option_...] _installable_ _args_...

# Examples

  * Run the default app from the `blender-bin` flake:
        
        # nix run blender-bin
        

  * Run a non-default app from the `blender-bin` flake:
        
        # nix run blender-bin#blender_2_83
        

Tip: you can find apps provided by this flake by running `nix flake show blender-bin`.

  * Run `vim` from the `nixpkgs` flake:
        
        # nix run nixpkgs#vim
        

Note that `vim` (as of the time of writing of this page) is not an app but a package. Thus, Nix runs the eponymous file from the `vim` package.

  * Run `vim` with arguments:
        
        # nix run nixpkgs#vim -- --help
        

  * Run the default app from the current directory with arguments:
        
        # nix run . -- arg1 arg2
        

Note: The first positional argument is always treated as the _installable_ , even after `--`. To pass arguments to the default installable, specify it explicitly: `nix run . -- arg1 arg2` or `nix run -- . arg1 arg2`.

# Description

`nix run` builds and runs [_installable_](</manual/nix/2.34/command-ref/new-cli/nix#installables>), which must evaluate to an _app_ or a regular Nix derivation.

If _installable_ evaluates to an _app_ (see below), it executes the program specified by the app definition.

If _installable_ evaluates to a derivation, it will try to execute the program `<out>/bin/<name>`, where _out_ is the primary output store path of the derivation, and _name_ is the first of the following that exists:

  * The `meta.mainProgram` attribute of the derivation.
  * The `pname` attribute of the derivation.
  * The name part of the value of the `name` attribute of the derivation.

For instance, if `name` is set to `hello-1.10`, `nix run` will run `$out/bin/hello`.

# Flake output attributes

If no flake output attribute is given, `nix run` tries the following flake output attributes:

  * `apps.<system>.default`

  * `packages.<system>.default`

If an attribute _name_ is given, `nix run` tries the following flake output attributes:

  * `apps.<system>.<name>`

  * `packages.<system>.<name>`

  * `legacyPackages.<system>.<name>`

# Apps

An app is specified by a flake output attribute named `apps.<system>.<name>`. It looks like this:
    
    
    apps.x86_64-linux.blender_2_79 = {
      type = "app";
      program = "${self.packages.x86_64-linux.blender_2_79}/bin/blender";
      meta.description = "Run Blender, a free and open-source 3D creation suite.";
    };
    

The only supported attributes are:

  * `type` (required): Must be set to `app`.

  * `program` (required): The full path of the executable to run. It must reside in the Nix store.

  * `meta.description` (optional): A description of the app.

# Options

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
