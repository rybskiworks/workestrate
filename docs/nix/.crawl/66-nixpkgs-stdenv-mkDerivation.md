---
type: Crawl Source
title: "nixpkgs — stdenv and mkDerivation"
description: "The standard environment (stdenv) and mkDerivation in nixpkgs."
resource: https://nixos.org/manual/nixpkgs/stable/#sec-using-stdenv
tags: [nix, nixpkgs-manual, stdenv, mkDerivation]
timestamp: 2026-07-24T00:00:00Z
---

# Crawl: nixpkgs-stdenv-mkDerivation

- seed_url: https://nixos.org/manual/nixpkgs/stable/
- canonical_url: https://nixos.org/manual/nixpkgs/stable/
- family: nixpkgs Manual
- fetch: 200
- version: nixpkgs 26.05
- feeds_docs: nix-stdenv.md

## Content

## Using `stdenv`

[Building a `stdenv` package in `nix-shell`](</manual/nixpkgs/stable/#sec-building-stdenv-package-in-nix-shell>)

To build a package with the standard environment, you use the function `stdenv.mkDerivation`, instead of the primitive built-in function `derivation`, e.g.
    
    
    stdenv.mkDerivation {
      name = "libfoo-1.2.3";
      src = fetchurl {
        url = "http://example.org/libfoo-1.2.3.tar.bz2";
        hash = "sha256-tWxU/LANbQE32my+9AXyt3nCT7NBVfJ45CX757EMT3Q=";
      };
    }
    

(`stdenv` needs to be in scope, so if you write this in a separate Nix expression from `pkgs/all-packages.nix`, you need to pass it as a function argument.) Specifying a `name` and a `src` is the absolute minimum Nix requires. For convenience, you can also use `pname` and `version` attributes and `mkDerivation` will automatically set `name` to `"${pname}-${version}"` by default. **Since[RFC 0035](<https://github.com/NixOS/rfcs/pull/35>), this is preferred for packages in Nixpkgs**, as it allows us to reuse the version easily:
    
    
    stdenv.mkDerivation (finalAttrs: {
      pname = "libfoo";
      version = "1.2.3";
      src = fetchurl {
        url = "http://example.org/libfoo-source-${finalAttrs.version}.tar.bz2";
        hash = "sha256-tWxU/LANbQE32my+9AXyt3nCT7NBVfJ45CX757EMT3Q=";
      };
    })
    

Many packages have dependencies that are not provided in the standard environment. It’s usually sufficient to specify those dependencies in the `buildInputs` attribute:
    
    
    stdenv.mkDerivation {
      pname = "libfoo";
      version = "1.2.3";
      # ...
      buildInputs = [
        libbar
        perl
        ncurses
      ];
    }
    

This attribute ensures that the `bin` subdirectories of these packages appear in the `PATH` environment variable during the build, that their `include` subdirectories are searched by the C compiler, and so on. (See [the section called “Package setup hooks”](</manual/nixpkgs/stable/#ssec-setup-hooks> "Package setup hooks") for details.)

Often it is necessary to override or modify some aspect of the build. To make this easier, the standard environment breaks the package build into a number of _phases_ , all of which can be overridden or modified individually: unpacking the sources, applying patches, configuring, building, and installing. (There are some others; see [the section called “Phases”](</manual/nixpkgs/stable/#sec-stdenv-phases> "Phases").) For instance, a package that doesn’t supply a makefile but instead has to be compiled “manually” could be handled like this:
    
    
    stdenv.mkDerivation {
      pname = "fnord";
      version = "4.5";
    
      # ...
    
      buildPhase = ''
        runHook preBuild
    
        gcc foo.c -o foo
    
        runHook postBuild
      '';
    
      installPhase = ''
        runHook preInstall
    
        mkdir -p $out/bin
        cp foo $out/bin
    
        runHook postInstall
      '';
    }
    

(Note the use of `''`-style string literals, which are very convenient for large multi-line script fragments because they don’t need escaping of `"` and `\`, and because indentation is intelligently removed.)

There are many other attributes to customise the build. These are listed in [the section called “Attributes”](</manual/nixpkgs/stable/#ssec-stdenv-attributes> "Attributes").

While the standard environment provides a generic builder, you can still supply your own build script:
    
    
    stdenv.mkDerivation {
      pname = "libfoo";
      version = "1.2.3";
      # ...
      builder = ./builder.sh;
    }
    

where `stdenv` sets up the environment automatically (e.g. by resetting `PATH` and populating it from build inputs). If you want, you can use `stdenv`’s generic builder:
    
    
    buildPhase() {
      echo "... this is my custom build phase ..."
      gcc foo.c -o foo
    }
    
    installPhase() {
      mkdir -p $out/bin
      cp foo $out/bin
    }
    
    genericBuild
    

### Building a `stdenv` package in `nix-shell`

To build a `stdenv` package in a [`nix-shell`](<https://nixos.org/manual/nix/unstable/command-ref/nix-shell.html>), enter a shell, find the [phases](</manual/nixpkgs/stable/#sec-stdenv-phases> "Phases") you wish to build, then invoke `genericBuild` manually:

Go to an empty directory, invoke `nix-shell` with the desired package, and from inside the shell, set the output variables to a writable directory:
    
    
    cd "$(mktemp -d)"
    nix-shell '<nixpkgs>' -A some_package
    export out=$(pwd)/out
    

Next, invoke the desired parts of the build. First, run the phases that generate a working copy of the sources, which will change directory to the sources for you:
    
    
    phases="${prePhases[*]:-} unpackPhase patchPhase" genericBuild
    

Then, run more phases up until the failure is reached. If the failure is in the build or check phase, the following phases would be required:
    
    
    phases="${preConfigurePhases[*]:-} configurePhase ${preBuildPhases[*]:-} buildPhase checkPhase" genericBuild
    

Use this command to run all install phases:
    
    
    phases="${preInstallPhases[*]:-} installPhase ${preFixupPhases[*]:-} fixupPhase installCheckPhase" genericBuild
    

Single phase can be re-run as many times as necessary to examine the failure like so:
    
    
    phases="buildPhase" genericBuild
    

To modify a [phase](</manual/nixpkgs/stable/#sec-stdenv-phases> "Phases"), first print it with
    
    
    echo "$buildPhase"
    

Or, if that is empty, for instance, if it is using a function:
    
    
    type buildPhase
    

then change it in a text editor, and paste it back to the terminal.

### Note

This method may have some inconsistencies in environment variables and behaviour compared to a normal build within the [Nix build sandbox](<https://nixos.org/manual/nix/unstable/language/derivations#builder-execution>). The following is a non-exhaustive list of such differences:

  * `TMP`, `TMPDIR`, and similar variables likely point to non-empty directories that the build might conflict with files in.

  * Output store paths are not writable, so the variables for outputs need to be overridden to writable paths.

  * Other environment variables may be inconsistent with a `nix-build` either due to `nix-shell`’s initialization script or due to the use of `nix-shell` without the `--pure` option.

If the build fails differently inside the shell than in the sandbox, consider using [`breakpointHook`](</manual/nixpkgs/stable/#breakpointhook> "breakpointHook") and invoking `nix-build` instead. The [`--keep-failed`](<https://nixos.org/manual/nix/unstable/command-ref/conf-file#conf-keep-failed>) option for `nix-build` may also be useful to examine the build directory of a failed build.

## Phases 

[Controlling phases](</manual/nixpkgs/stable/#ssec-controlling-phases>)
[The unpack phase](</manual/nixpkgs/stable/#ssec-unpack-phase>)
[The patch phase](</manual/nixpkgs/stable/#ssec-patch-phase>)
[The configure phase](</manual/nixpkgs/stable/#ssec-configure-phase>)
[The build phase](</manual/nixpkgs/stable/#build-phase>)
[The check phase](</manual/nixpkgs/stable/#ssec-check-phase>)
[The install phase](</manual/nixpkgs/stable/#ssec-install-phase>)
[The fixup phase](</manual/nixpkgs/stable/#ssec-fixup-phase>)
[The installCheck phase](</manual/nixpkgs/stable/#ssec-installCheck-phase>)
[The distribution phase](</manual/nixpkgs/stable/#ssec-distribution-phase>)

`stdenv.mkDerivation` sets the Nix [derivation](<https://nixos.org/manual/nix/stable/expressions/derivations.html#derivations>)’s builder to a script that loads the stdenv `setup.sh` bash library and calls `genericBuild`. Most packaging functions rely on this default builder.

This generic command either invokes a script at _buildCommandPath_ , or a _buildCommand_ , or a number of _phases_. Package builds are split into phases to make it easier to override specific parts of the build (e.g., unpacking the sources or installing the binaries).

Each phase can be overridden in its entirety either by setting the environment variable `namePhase` to a string containing some shell commands to be executed, or by redefining the shell function `namePhase`. The former is convenient to override a phase from the derivation, while the latter is convenient from a build script. However, typically one only wants to _add_ some commands to a phase, e.g. by defining `postInstall` or `preFixup`, as skipping some of the default actions may have unexpected consequences. The default script for each phase is defined in the file `pkgs/stdenv/generic/setup.sh`.

When overriding a phase, for example `installPhase`, it is important to start with `runHook preInstall` and end it with `runHook postInstall`, otherwise `preInstall` and `postInstall` will not be run. Even if you don’t use them directly, it is good practice to do so anyways for downstream users who would want to add a `postInstall` by overriding your derivation.

While inside an interactive `nix-shell`, if you wanted to run all phases in the order they would be run in an actual build, you can invoke `genericBuild` yourself.

### Controlling phases 

There are a number of variables that control what phases are executed and in what order:

#### Variables affecting phase control 

##### `phases`

Specifies the phases. You can change the order in which phases are executed, or add new phases, by setting this variable. If it’s not set, the default value is used, which is `$prePhases unpackPhase patchPhase $preConfigurePhases configurePhase $preBuildPhases buildPhase checkPhase $preInstallPhases installPhase fixupPhase installCheckPhase $preDistPhases distPhase $postPhases`.

The elements of `phases` must not contain spaces. If `phases` is specified as a Nix Language attribute, it should be specified as lists instead of strings. The same rules apply to the `*Phases` variables.

It is discouraged to set this variable, as it is easy to miss some important functionality hidden in some of the less obviously needed phases (like `fixupPhase` which patches the shebang of scripts). Usually, if you just want to add a few phases, it’s more convenient to set one of the `*Phases` variables below.

##### `prePhases`

Additional phases executed before any of the default phases.

##### `preConfigurePhases`

Additional phases executed just before the configure phase.

##### `preBuildPhases`

Additional phases executed just before the build phase.

##### `preInstallPhases`

Additional phases executed just before the install phase.

##### `preFixupPhases`

Additional phases executed just before the fixup phase.

##### `preDistPhases`

Additional phases executed just before the distribution phase.

##### `postPhases`

Additional phases executed after any of the default phases.

### The unpack phase 

The unpack phase is responsible for unpacking the source code of the package. The default implementation of `unpackPhase` unpacks the source files listed in the `src` environment variable to the current directory. It supports the following files by default:

#### Tar files 

These can optionally be compressed using `gzip` (`.tar.gz`, `.tgz` or `.tar.Z`), `bzip2` (`.tar.bz2`, `.tbz2` or `.tbz`) or `xz` (`.tar.xz`, `.tar.lzma` or `.txz`).

#### Zip files 

Zip files are unpacked using `unzip`. However, `unzip` is not in the standard environment, so you should add it to `nativeBuildInputs` yourself.

#### Directories in the Nix store 

These are copied to the current directory. The hash part of the file name is stripped, e.g. `/nix/store/1wydxgby13cz...-my-sources` would be copied to `my-sources`.

Additional file types can be supported by setting the `unpackCmd` variable (see below).

#### Variables controlling the unpack phase 

##### `srcs` / `src`

The list of source files or directories to be unpacked or copied. One of these must be set. Note that if you use `srcs`, you should also set `sourceRoot` or `setSourceRoot`.

These should ideally actually be sources and licensed under a FLOSS license. If you have to use a binary upstream release or package non-free software, make sure you correctly mark your derivation as such in the [`sourceProvenance`](</manual/nixpkgs/stable/#var-meta-sourceProvenance> "sourceProvenance") and [`license`](</manual/nixpkgs/stable/#sec-meta-license> "Licenses") fields of the [`meta`](</manual/nixpkgs/stable/#chap-meta> "Meta-attributes") section.

##### `sourceRoot`

After unpacking all of `src` and `srcs`, if neither of `sourceRoot` and `setSourceRoot` are set, `unpackPhase` of the generic builder checks that the unpacking produced a single directory and moves the current working directory into it.

If `unpackPhase` produces multiple source directories, you should set `sourceRoot` to the name of the intended directory. You can also set `sourceRoot = ".";` if you want to control it yourself in a later phase.

For example, if you want your build to start in a sub-directory inside your sources, and you are using `fetchzip`-derived `src` (like `fetchFromGitHub` or similar), you need to set `sourceRoot = "${src.name}/my-sub-directory"`.

##### `setSourceRoot`

Alternatively to setting `sourceRoot`, you can set `setSourceRoot` to a shell command to be evaluated by the unpack phase after the sources have been unpacked. This command must set `sourceRoot`.

For example, if you are using `fetchurl` on an archive file that gets unpacked into a single directory the name of which changes between package versions, and you want your build to start in its sub-directory, you need to set `setSourceRoot = "sourceRoot=$(echo */my-sub-directory)";`, or in the case of multiple sources, you could use something more specific, like `setSourceRoot = "sourceRoot=$(echo ${pname}-*/my-sub-directory)";`.

##### `preUnpack`

Hook executed at the start of the unpack phase.

##### `postUnpack`

Hook executed at the end of the unpack phase.

##### `dontUnpack`

Set to true to skip the unpack phase.

##### `dontMakeSourcesWritable`

If set to `1`, the unpacked sources are _not_ made writable. By default, they are made writable to prevent problems with read-only sources. For example, copied store directories would be read-only without this.

##### `unpackCmd`

The unpack phase evaluates the string `$unpackCmd` for any unrecognised file. The path to the current source file is contained in the `curSrc` variable.

### The patch phase 

The patch phase applies the list of patches defined in the `patches` variable.

#### Variables controlling the patch phase 

##### `dontPatch`

Set to true to skip the patch phase.

##### `patches`

The list of patches. They must be in the format accepted by the `patch` command, and may optionally be compressed using `gzip` (`.gz`), `bzip2` (`.bz2`) or `xz` (`.xz`).

##### `patchFlags`

Flags to be passed to `patch`. If not set, the argument `-p1` is used, which causes the leading directory component to be stripped from the file names in each patch.

##### `prePatch`

Hook executed at the start of the patch phase.

##### `postPatch`

Hook executed at the end of the patch phase.

### The configure phase 

The configure phase prepares the source tree for building. The default `configurePhase` runs `./configure` (typically an Autoconf-generated script) if it exists.

#### Variables controlling the configure phase 

##### `configureScript`

The name of the configure script. It defaults to `./configure` if it exists; otherwise, the configure phase is skipped. This can actually be a command (like `perl ./Configure.pl`).

##### `configureFlags`

A list of strings passed as additional arguments to the configure script.

##### `dontConfigure`

Set to true to skip the configure phase.

##### `configureFlagsArray`

A shell array containing additional arguments passed to the configure script. You must use this instead of `configureFlags` if the arguments contain spaces.

##### `dontAddPrefix`

By default, `./configure` is passed the concatenation of [`prefixKey`](</manual/nixpkgs/stable/#var-stdenv-prefixKey> "prefixKey") and [`prefix`](</manual/nixpkgs/stable/#var-stdenv-prefix> "prefix") on the command line. Disable this by setting `dontAddPrefix` to `true`.

##### `prefix`

The prefix under which the package must be installed, passed via the `--prefix` option to the configure script. It defaults to `$out`.

##### `prefixKey`

The key to use when specifying the installation [`prefix`](</manual/nixpkgs/stable/#var-stdenv-prefix> "prefix"). By default, this is set to `--prefix=` as that is used by the majority of packages. Other packages may need `--prefix ` (with a trailing space) or `PREFIX=`.

##### `dontAddStaticConfigureFlags`

By default, when building statically, `stdenv` will try to add build system appropriate configure flags to try to enable static builds.

If this is undesirable, set this variable to true.

##### `dontAddDisableDepTrack`

By default, the flag `--disable-dependency-tracking` is added to the configure flags to speed up Automake-based builds. If this is undesirable, set this variable to true.

##### `dontFixLibtool`

By default, the configure phase applies some special hackery to all files called `ltmain.sh` before running the configure script in order to improve the purity of Libtool-based packages [[4]](</manual/nixpkgs/stable/#footnote-stdenv-sys-lib-search-path>) . If this is undesirable, set this variable to true.

##### `dontDisableStatic`

By default, when the configure script has `--enable-static`, the option `--disable-static` is added to the configure flags.

If this is undesirable, set this variable to true. It is automatically set to true when building statically, for example through `pkgsStatic`.

##### `configurePlatforms`

By default, when cross compiling, the configure script has `--build=...` and `--host=...` passed. Packages can instead pass `[ "build" "host" "target" ]` or a subset to control exactly which platform flags are passed. Compilers and other tools can use this to also pass the target platform. [[5]](</manual/nixpkgs/stable/#footnote-stdenv-build-time-guessing-impurity>)

##### `preConfigure`

Hook executed at the start of the configure phase.

##### `postConfigure`

Hook executed at the end of the configure phase.

### The build phase 

The build phase is responsible for actually building the package (e.g. compiling it). The default `buildPhase` calls `make` if a file named `Makefile`, `makefile` or `GNUmakefile` exists in the current directory (or the `makefile` is explicitly set); otherwise it does nothing.

#### Variables controlling the build phase 

##### `dontBuild`

Set to true to skip the build phase.

##### `makefile`

The file name of the Makefile.

##### `makeFlags`

A list of strings passed as additional flags to `make`. These flags are also used by the default install and check phase. For setting make flags specific to the build phase, use `buildFlags` (see below).
    
    
    { makeFlags = [ "PREFIX=$(out)" ]; }
    

### Note

The flags are quoted in bash, but environment variables can be specified by using the make syntax.

##### `makeFlagsArray`

A shell array containing additional arguments passed to `make`. You must use this instead of `makeFlags` if the arguments contain spaces, e.g.
    
    
    {
      preBuild = ''
        makeFlagsArray+=(CFLAGS="-O0 -g" LDFLAGS="-lfoo -lbar")
      '';
    }
    

Note that shell arrays cannot be passed through environment variables, so you cannot set `makeFlagsArray` in a derivation attribute (because those are passed through environment variables): you have to define them in shell code.

##### `buildFlags` / `buildFlagsArray`

A list of strings passed as additional flags to `make`. Like `makeFlags` and `makeFlagsArray`, but only used by the build phase. Any build targets should be specified as part of the `buildFlags`.

##### `preBuild`

Hook executed at the start of the build phase.

##### `postBuild`

Hook executed at the end of the build phase.

You can set flags for `make` through the `makeFlags` variable.

Before and after running `make`, the hooks `preBuild` and `postBuild` are called, respectively.

### The check phase 

The check phase checks whether the package was built correctly by running its test suite. The default `checkPhase` calls `make $checkTarget`, but only if the [`doCheck` variable](</manual/nixpkgs/stable/#var-stdenv-doCheck> "doCheck") is enabled.

It is highly recommended, for packages’ sources that are not distributed with any tests, to at least use [`versionCheckHook`](</manual/nixpkgs/stable/#versioncheckhook> "versionCheckHook") to test that the resulting executable is basically functional.

#### Variables controlling the check phase 

##### `doCheck`

Controls whether the check phase is executed. By default it is skipped, but if `doCheck` is set to true, the check phase is usually executed. Thus you should set
    
    
    { doCheck = true; }
    

in the derivation to enable checks. The exception is cross compilation. Cross compiled builds never run tests, no matter how `doCheck` is set, as the newly-built program won’t run on the platform used to build it.

##### `makeFlags` / `makeFlagsArray` / `makefile`

See the [build phase](</manual/nixpkgs/stable/#var-stdenv-makeFlags> "makeFlags") for details.

##### `checkTarget`

The `make` target that runs the tests. If unset, use `check` if it exists, otherwise `test`; if neither is found, do nothing.

##### `checkFlags` / `checkFlagsArray`

A list of strings passed as additional flags to `make`. Like `makeFlags` and `makeFlagsArray`, but only used by the check phase. Unlike with `buildFlags`, the `checkTarget` is automatically added to the `make` invocation in addition to any `checkFlags` specified.

##### `checkInputs`

A list of host dependencies used by the phase, usually libraries linked into executables built during tests. This gets included in `buildInputs` when `doCheck` is set.

##### `nativeCheckInputs`

A list of native dependencies used by the phase, notably tools needed on `$PATH`. This gets included in `nativeBuildInputs` when `doCheck` is set.

##### `preCheck`

Hook executed at the start of the check phase.

##### `postCheck`

Hook executed at the end of the check phase.

### The install phase 

The install phase is responsible for installing the package in the Nix store under `out`. The default `installPhase` creates the directory `$out` and calls `make install`.

#### Variables controlling the install phase 

##### `dontInstall`

Set to true to skip the install phase.

##### `makeFlags` / `makeFlagsArray` / `makefile`

See the [build phase](</manual/nixpkgs/stable/#var-stdenv-makeFlags> "makeFlags") for details.

##### `installTargets`

The make targets that perform the installation. Defaults to `install`. Example:
    
    
    { installTargets = "install-bin install-doc"; }
    

##### `installFlags` / `installFlagsArray`

A list of strings passed as additional flags to `make`. Like `makeFlags` and `makeFlagsArray`, but only used by the install phase. Unlike with `buildFlags`, the `installTargets` are automatically added to the `make` invocation in addition to any `installFlags` specified.

##### `preInstall`

Hook executed at the start of the install phase.

##### `postInstall`

Hook executed at the end of the install phase.

### The fixup phase 

The fixup phase performs (Nix-specific) post-processing actions on the files installed under `$out` by the install phase. The default `fixupPhase` does the following:

  * It moves the `man/`, `doc/` and `info/` subdirectories of `$out` to `share/`.

  * It strips libraries and executables of debug information.

  * On Linux, it applies the `patchelf` command to ELF executables and libraries to remove unused directories from the `RPATH` in order to prevent unnecessary runtime dependencies.

  * It rewrites the interpreter paths of shell scripts to paths found in `PATH`. E.g., `/usr/bin/perl` will be rewritten to `/nix/store/some-perl/bin/perl` found in `PATH`. See [the section called “`patch-shebangs.sh`”](</manual/nixpkgs/stable/#patch-shebangs.sh> "patch-shebangs.sh") for details.

#### Variables controlling the fixup phase 

##### `dontFixup`

Set to true to skip the fixup phase.

##### `dontStrip`

If set, libraries and executables are not stripped. By default, they are.

##### `dontStripHost`

Like `dontStrip`, but only affects the `strip` command targeting the package’s host platform. Useful when supporting cross compilation, but otherwise feel free to ignore.

##### `dontStripTarget`

Like `dontStrip`, but only affects the `strip` command targeting the packages’ target platform. Useful when supporting cross compilation, but otherwise feel free to ignore.

##### `dontMoveSbin`

If set, files in `$out/sbin` are not moved to `$out/bin`. By default, they are.

##### `stripAllList`

List of directories to search for libraries and executables from which _all_ symbols should be stripped. By default, it’s empty. Stripping all symbols is risky, since it may remove not just debug symbols but also ELF information necessary for normal execution.

##### `stripAllListTarget`

Like `stripAllList`, but only applies to packages’ target platform. By default, it’s empty. Useful when supporting cross compilation.

##### `stripAllFlags`

Flags passed to the `strip` command applied to the files in the directories listed in `stripAllList`. Defaults to `-s -p` (i.e. `--strip-all --preserve-dates`).

##### `stripDebugList`

List of directories to search for libraries and executables from which only debugging-related symbols should be stripped. It defaults to `lib lib32 lib64 libexec bin sbin`.

##### `stripDebugListTarget`

Like `stripDebugList`, but only applies to packages’ target platform. By default, it’s empty. Useful when supporting cross compilation.

##### `stripDebugFlags`

Flags passed to the `strip` command applied to the files in the directories listed in `stripDebugList`. Defaults to `-S -p` (i.e. `--strip-debug --preserve-dates`).

##### `stripExclude`

A list of filenames or path patterns to avoid stripping. A file is excluded if its name _or_ path (from the derivation root) matches.

This example prevents all `*.rlib` files from being stripped:
    
    
    stdenv.mkDerivation {
      # ...
      stripExclude = [ "*.rlib" ];
    }
    

This example prevents files within certain paths from being stripped:
    
    
    stdenv.mkDerivation {
      # ...
      stripExclude = [ "lib/modules/*/build/*" ];
    }
    

##### `dontPatchELF`

If set, the `patchelf` command is not used to remove unnecessary `RPATH` entries. Only applies to Linux.

##### `dontPatchShebangs`

If set, scripts starting with `#!` do not have their interpreter paths rewritten to paths in the Nix store. See [the section called “`patch-shebangs.sh`”](</manual/nixpkgs/stable/#patch-shebangs.sh> "patch-shebangs.sh") on how patching shebangs works.

##### `dontPruneLibtoolFiles`

If set, libtool `.la` files associated with shared libraries won’t have their `dependency_libs` field cleared.

##### `forceShare`

The list of directories that must be moved from `$out` to `$out/share`. Defaults to `man doc info`.

##### `setupHook`

A package can export a [setup hook](</manual/nixpkgs/stable/#ssec-setup-hooks> "Package setup hooks") by setting this variable. The setup hook, if defined, is copied to `$out/nix-support/setup-hook`. Environment variables are then substituted in it using `substituteAll`.

##### `preFixup`

Hook executed at the start of the fixup phase.

##### `postFixup`

Hook executed at the end of the fixup phase.

##### `separateDebugInfo`

If set to `true`, the standard environment will enable debug information in C/C++ builds. After installation, the debug information will be separated from the executables and stored in the output named `debug`. (This output is enabled automatically; you don’t need to set the `outputs` attribute explicitly.) To be precise, the debug information is stored in `debug/lib/debug/.build-id/XX/YYYY…`, where <XXYYYY…> is the <build ID> of the binary — a SHA-1 hash of the contents of the binary. Debuggers like GDB use the build ID to look up the separated debug information.

**Example 302. Enable debug symbols for use with GDB**

To make GDB find debug information for the `socat` package and its dependencies, you can use the following `shell.nix`:
    
    
    {
      pkgs ? import <nixpkgs> {
        config = { };
        overlays = [
          (final: prev: {
            ncurses = prev.ncurses.overrideAttrs { separateDebugInfo = true; };
            readline = prev.readline.overrideAttrs { separateDebugInfo = true; };
          })
        ];
      },
    }:
    pkgs.mkShell {
      NIX_DEBUG_INFO_DIRS = pkgs.lib.makeSearchPathOutput "debug" "lib/debug" [
        pkgs.glibc
        pkgs.ncurses
        pkgs.openssl
        pkgs.readline
      ];
    
      packages = [
        pkgs.gdb
        pkgs.socat
      ];
    
      shellHook = ''
        gdb socat
      '';
    }
    

This setup works as follows:

  * Add [`overlays`](</manual/nixpkgs/stable/#chap-overlays> "Overlays") to the package set, since debug symbols are disabled for `ncurses` and `readline` by default.

  * Set the environment variable `NIX_DEBUG_INFO_DIRS` in the shell. Nixpkgs patches `gdb` to use this variable for looking up debug symbols. [`lib.makeSearchPathOutput`](</manual/nixpkgs/stable/#function-library-lib.strings.makeSearchPathOutput> "lib.strings.makeSearchPathOutput") constructs a colon-separated search path, pointing to the directories containing the debug symbols of the listed packages.

  * Run `gdb` on the `socat` binary on shell startup in the [`shellHook`](</manual/nixpkgs/stable/#sec-pkgs-mkShell> "pkgs.mkShell").

  

### The installCheck phase 

The installCheck phase checks whether the package was installed correctly by running its test suite against the installed directories. The default `installCheck` calls `make installcheck`.

It is often better to add tests that are not part of the source distribution to `passthru.tests` (see [the section called “`passthru.tests`”](</manual/nixpkgs/stable/#var-passthru-tests> "passthru.tests")). This avoids adding overhead to every build and enables us to run them independently.

#### Variables controlling the installCheck phase 

##### `doInstallCheck`

Controls whether the installCheck phase is executed. By default it is skipped, but if `doInstallCheck` is set to true, the installCheck phase is usually executed. Thus you should set
    
    
    { doInstallCheck = true; }
    

in the derivation to enable install checks. The exception is cross compilation. Cross compiled builds never run tests, no matter how `doInstallCheck` is set, as the newly-built program won’t run on the platform used to build it.

##### `installCheckTarget`

The make target that runs the install tests. Defaults to `installcheck`.

##### `installCheckFlags` / `installCheckFlagsArray`

A list of strings passed as additional flags to `make`. Like `makeFlags` and `makeFlagsArray`, but only used by the installCheck phase.

##### `installCheckInputs`

A list of host dependencies used by the phase, usually libraries linked into executables built during tests. This gets included in `buildInputs` when `doInstallCheck` is set.

##### `nativeInstallCheckInputs`

A list of native dependencies used by the phase, notably tools needed on `$PATH`. This gets included in `nativeBuildInputs` when `doInstallCheck` is set.

##### `preInstallCheck`

Hook executed at the start of the installCheck phase.

##### `postInstallCheck`

Hook executed at the end of the installCheck phase.

### The distribution phase 

The distribution phase is intended to produce a source distribution of the package. The default `distPhase` first calls `make dist`, then it copies the resulting source tarballs to `$out/tarballs/`. This phase is only executed if the attribute `doDist` is set.

#### Variables controlling the distribution phase 

##### `doDist`

If set, the distribution phase is executed.

##### `distTarget`

The make target that produces the distribution. Defaults to `dist`.

##### `distFlags` / `distFlagsArray`

Additional flags passed to `make`.

##### `tarballs`

The names of the source distribution files to be copied to `$out/tarballs/`. It can contain shell wildcards. The default is `*.tar.gz`.

##### `dontCopyDist`

If set, no files are copied to `$out/tarballs/`.

##### `preDist`

Hook executed at the start of the distribution phase.

##### `postDist`

Hook executed at the end of the distribution phase.

## Specifying dependencies 

[Overview](</manual/nixpkgs/stable/#ssec-stdenv-dependencies-overview>)
[Reference](</manual/nixpkgs/stable/#ssec-stdenv-dependencies-reference>)
[Dependency propagation](</manual/nixpkgs/stable/#ssec-stdenv-dependencies-propagated>)

Build systems often require more dependencies than just what `stdenv` provides. This section describes attributes accepted by `stdenv.mkDerivation` that can be used to make these dependencies available to the build system.

### Overview 

A full reference of the different kinds of dependencies is provided in [the section called “Reference”](</manual/nixpkgs/stable/#ssec-stdenv-dependencies-reference> "Reference"), but here is an overview of the most common ones. It should cover most use cases.

Add dependencies to `nativeBuildInputs` if they are executed during the build:

  * those which are needed on `$PATH` during the build, for example `cmake` and `pkg-config`

  * [setup hooks](</manual/nixpkgs/stable/#ssec-setup-hooks> "Package setup hooks"), for example [`makeWrapper`](</manual/nixpkgs/stable/#fun-makeWrapper> "makeWrapper <executable> <wrapperfile> <args>")

  * interpreters needed by [`patchShebangs`](</manual/nixpkgs/stable/#patch-shebangs.sh> "patch-shebangs.sh") for build scripts (with the `--build` flag), which can be the case for e.g. `perl`

Add dependencies to `buildInputs` if they will end up copied or linked into the final output or otherwise used at runtime:

  * libraries used by compilers, for example `zlib`,

  * interpreters needed by [`patchShebangs`](</manual/nixpkgs/stable/#patch-shebangs.sh> "patch-shebangs.sh") for scripts which are installed, which can be the case for e.g. `perl`

### Note

These criteria are independent.

For example, software using Wayland usually needs the `wayland` library at runtime, so `wayland` should be added to `buildInputs`. But it also executes the `wayland-scanner` program as part of the build to generate code, so `wayland` should also be added to `nativeBuildInputs`.

Dependencies needed only to run tests are similarly classified between native (executed during build) and non-native (executed at runtime):

  * `nativeCheckInputs` for test tools needed on `$PATH` (such as `ctest`) and [setup hooks](</manual/nixpkgs/stable/#ssec-setup-hooks> "Package setup hooks") (for example [`pytestCheckHook`](</manual/nixpkgs/stable/#python> "Python"))

  * `checkInputs` for libraries linked into test executables (for example the `qcheck` OCaml package)

These dependencies are only injected when [`doCheck`](</manual/nixpkgs/stable/#var-stdenv-doCheck> "doCheck") is set to `true`.

#### Example 

Consider for example this simplified derivation for `solo5`, a sandboxing tool:
    
    
    stdenv.mkDerivation (finalAttrs: {
      pname = "solo5";
      version = "0.7.5";
    
      src = fetchurl {
        url = "https://github.com/Solo5/solo5/releases/download/v${finalAttrs.version}/solo5-v${finalAttrs.version}.tar.gz";
        hash = "sha256-viwrS9lnaU8sTGuzK/+L/PlMM/xRRtgVuK5pixVeDEw=";
      };
    
      nativeBuildInputs = [
        makeWrapper
        pkg-config
      ];
    
      buildInputs = [ libseccomp ];
    
      postInstall = ''
        substituteInPlace $out/bin/solo5-virtio-mkimage \
          --replace-fail "/usr/lib/syslinux" "${syslinux}/share/syslinux" \
          --replace-fail "/usr/share/syslinux" "${syslinux}/share/syslinux" \
          --replace-fail "cp " "cp --no-preserve=mode "
    
        wrapProgram $out/bin/solo5-virtio-mkimage \
          --prefix PATH : ${
            lib.makeBinPath [
              dosfstools
              mtools
              parted
              syslinux
            ]
          }
      '';
    
      doCheck = true;
      nativeCheckInputs = [
        util-linux
        qemu
      ];
      # `checkPhase` elided
    })
    

  * `makeWrapper` is a setup hook, i.e., a shell script sourced by the generic builder of `stdenv`. It is thus executed during the build and must be added to `nativeBuildInputs`.

  * `pkg-config` is a build tool which the configure script of `solo5` expects to be on `$PATH` during the build: therefore, it must be added to `nativeBuildInputs`.

  * `libseccomp` is a library linked into `$out/bin/solo5-elftool`. As it is used at runtime, it must be added to `buildInputs`.

  * Tests need `qemu` and `getopt` (from `util-linux`) on `$PATH`, these must be added to `nativeCheckInputs`.

  * Some dependencies are injected directly in the shell code of phases: `syslinux`, `dosfstools`, `mtools`, and `parted`. In this specific case, they will end up in the output of the derivation (`$out` here). As Nix marks dependencies whose absolute path is present in the output as runtime dependencies, adding them to `buildInputs` is not required.

For more complex cases, like libraries linked into an executable which is then executed as part of the build system, see [the section called “Reference”](</manual/nixpkgs/stable/#ssec-stdenv-dependencies-reference> "Reference").

### Reference 

As described in the Nix manual, almost any `*.drv` store path in a derivation’s attribute set will induce a dependency on that derivation. `mkDerivation`, however, takes a few attributes intended to include all the dependencies of a package. This is done both for structure and consistency, but also so that certain other setup can take place. For example, certain dependencies need their bin directories added to the `PATH`. That is built-in, but other setup is done via a pluggable mechanism that works in conjunction with these dependency attributes. See [the section called “Package setup hooks”](</manual/nixpkgs/stable/#ssec-setup-hooks> "Package setup hooks") for details.

Dependencies can be broken down along these axes: their host and target platforms relative to the new derivation’s. The platform distinctions are motivated by cross compilation; see [_Cross-compilation_](</manual/nixpkgs/stable/#chap-cross> "Cross-compilation") for exactly what each platform means. [[1]](</manual/nixpkgs/stable/#footnote-stdenv-ignored-build-platform>) But even if one is not cross compiling, the platforms imply whether a dependency is needed at run-time or build-time.

The extension of `PATH` with dependencies, alluded to above, proceeds according to the relative platforms alone. The process is carried out only for dependencies whose host platform matches the new derivation’s build platform i.e. dependencies which run on the platform where the new derivation will be built. [[2]](</manual/nixpkgs/stable/#footnote-stdenv-native-dependencies-in-path>) For each dependency <dep> of those dependencies, `dep/bin`, if present, is added to the `PATH` environment variable.

### Dependency propagation 

Propagated dependencies are made available to all downstream dependencies. This is particularly useful for interpreted languages, where all transitive dependencies have to be present in the same environment. Therefore it is used for the Python infrastructure in Nixpkgs.

### Note

Propagated dependencies should be used with care, because they obscure the actual build inputs of dependent derivations and cause side effects through setup hooks. This can lead to conflicting dependencies that cannot easily be resolved.

**Example 300. A propagated dependency**
    
    
    with import <nixpkgs> { };
    let
      bar = stdenv.mkDerivation {
        name = "bar";
        dontUnpack = true;
        # `hello` is also made available to dependents, such as `foo`
        propagatedBuildInputs = [ hello ];
        postInstall = "mkdir $out";
      };
      foo = stdenv.mkDerivation {
        name = "foo";
        dontUnpack = true;
        # `bar` is a direct dependency, which implicitly includes the propagated `hello`
        buildInputs = [ bar ];
        # The `hello` binary is available!
        postInstall = "hello > $out";
      };
    in
    foo
    

  

Dependency propagation takes cross compilation into account, meaning that dependencies that cross platform boundaries are properly adjusted.

To determine the exact rules for dependency propagation, we start by assigning to each dependency a couple of ternary numbers (`-1` for `build`, `0` for `host`, and `1` for `target`) representing its [dependency type](</manual/nixpkgs/stable/#possible-dependency-types> "Possible dependency types"), which captures how its host and target platforms are each “offset” from the depending derivation’s host and target platforms. The following table summarize the different combinations that can be obtained:

Dependency type| attribute name| offset| typical purpose  
---|---|---|---  
`build → build`| `depsBuildBuild`| `-1, -1`| compilers for build helpers  
`build → host`| `nativeBuildInputs`| `-1, 0`| build tools, compilers, setup hooks  
`build → target`| `depsBuildTarget`| `-1, 1`| compilers to build stdlibs to run on target  
`host → host`| `depsHostHost`| `0, 0`| compilers to build C code at runtime (rare)  
`host → target`| `buildInputs`| `0, 1`| libraries  
`target → target`| `depsTargetTarget`| `1, 1`| stdlibs to run on target  
  
Algorithmically, we traverse propagated inputs, accumulating every propagated dependency’s propagated dependencies and adjusting them to account for the “shift in perspective” described by the current dependency’s platform offsets. This results in a sort of transitive closure of the dependency relation, with the offsets being approximately summed when two dependency links are combined. We also prune transitive dependencies whose combined offsets go out-of-bounds, which can be viewed as a filter over that transitive closure removing dependencies that are blatantly absurd.

We can define the process precisely with [Natural Deduction](<https://en.wikipedia.org/wiki/Natural_deduction>) using the inference rules below. This probably seems a bit obtuse, but so is the bash code that actually implements it! [[3]](</manual/nixpkgs/stable/#footnote-stdenv-find-inputs-location>) They’re confusing in very different ways so… hopefully if something doesn’t make sense in one presentation, it will in the other!

**Definitions:**

`dep(h_offset, t_offset, X, Y)`
    

Package X has a direct dependency on Y in a position with host offset `h_offset` and target offset `t_offset`.

For example, `nativeBuildInputs = [ Y ]` means `dep(-1, 0, X, Y)`.

`propagated-dep(h_offset, t_offset, X, Y)`
    

Package X has a propagated dependency on Y in a position with host offset `h_offset` and target offset `t_offset`.

For example, `depsBuildTargetPropagated = [ Y ]` means `propagated-dep(-1, 1, X, Y)`.

`mapOffset(h, t, i) = offs`
    

In a package X with a dependency on Y in a position with host offset `h` and target offset `t`, Y’s transitive dependency Z in a position with offset `i` is mapped to offset `offs` in X.

**Example 301. Truth table of`mapOffset(h, t, i)`**

`x` means that the dependency was discarded because `h + i ∉ {-1, 0, 1}`.
    
    
      h |   t  || i=-1 |  i=0 |  i=1
    ----|------||------|------|-----
     -1 |  -1  ||   x  |  -1  |  -1
     -1 |   0  ||   x  |  -1  |   0
     -1 |   1  ||   x  |  -1  |   1
      0 |   0  ||  -1  |   0  |   0
      0 |   1  ||  -1  |   0  |   1
      1 |   1  ||   0  |   1  |   x
    

  

    
    
    let mapOffset(h, t, i) = i + (if i <= 0 then h else t - 1)
    
    propagated-dep(h0, t0, A, B)
    propagated-dep(h1, t1, B, C)
    h0 + h1 in {-1, 0, 1}
    h0 + t1 in {-1, 0, 1}
    -------------------------------------- Transitive property
    propagated-dep(mapOffset(h0, t0, h1),
                   mapOffset(h0, t0, t1),
                   A, C)
    
    
    
    let mapOffset(h, t, i) = i + (if i <= 0 then h else t - 1)
    
    dep(h0, t0, A, B)
    propagated-dep(h1, t1, B, C)
    h0 + h1 in {-1, 0, 1}
    h0 + t1 in {-1, 0, 1}
    ----------------------------- Take immediate dependencies' propagated dependencies
    propagated-dep(mapOffset(h0, t0, h1),
                   mapOffset(h0, t0, t1),
                   A, C)
    
    
    
    propagated-dep(h, t, A, B)
    ----------------------------- Propagated dependencies count as dependencies
    dep(h, t, A, B)
    

Some explanation of this monstrosity is in order. In the common case of `nativeBuildInputs` or `buildInputs`, the target offset of a dependency is one greater than the host offset: `t = h + 1`. That means that:
    
    
    let f(h, t, i) = i + (if i <= 0 then h else t - 1)
    let f(h, h + 1, i) = i + (if i <= 0 then h else (h + 1) - 1)
    let f(h, h + 1, i) = i + (if i <= 0 then h else h)
    let f(h, h + 1, i) = i + h
    

This is where “sum-like” comes in from above: We can just sum all of the host offsets to get the host offset of the transitive dependency. The target offset is the transitive dependency is the host offset + 1, just as it was with the dependencies composed to make this transitive one; it can be ignored as it doesn’t add any new information.

Because of the bounds checks, the uncommon cases are `h = t` (`depsBuildBuild`, etc) and `h + 2 = t` (`depsBuildTarget`).

In the former case, the motivation for `mapOffset` is that since its host and target platforms are the same, no transitive dependency of it should be able to “discover” an offset greater than its reduced target offsets. `mapOffset` effectively “squashes” all its transitive dependencies’ offsets so that none will ever be greater than the target offset of the original `h = t` package.

In the other case, `h + 1` (0) is skipped over between the host (-1) and target (1) offsets. Instead of squashing the offsets, we need to “rip” them apart so no transitive dependency’s offset is 0.

Overall, the unifying theme here is that propagation shouldn’t be introducing transitive dependencies involving platforms the depending package is unaware of. [One can imagine the depending package asking for dependencies with the platforms it knows about; other platforms it doesn’t know how to ask for. The platform description in that scenario is a kind of unforgeable capability.] The offset bounds checking and definition of `mapOffset` together ensure that this is the case. Discovering a new offset is discovering a new platform, and since those platforms weren’t in the derivation “spec” of the needing package, they cannot be relevant. From a capability perspective, we can imagine that the host and target platforms of a package are the capabilities a package requires, and the depending package must provide the capability to the dependency.

#### Variables specifying dependencies 

##### `depsBuildBuild`

A list of dependencies whose host and target platforms are the new derivation’s build platform. These are programs and libraries used at build time that produce programs and libraries also used at build time. If the dependency doesn’t care about the target platform (i.e. isn’t a compiler or similar tool), put it in `nativeBuildInputs` instead. The most common use of this `buildPackages.stdenv.cc` (the compiler for `buildPackages`, which means that it’s from the package set `buildPackages.buildPackages = pkgsBuildBuild`), the default C compiler for this role. That example crops up more than one might think in old commonly used C libraries.

Since these packages are able to be run at build-time, they are always added to the `PATH`, as described above. But since these packages are only guaranteed to be able to run then, they shouldn’t persist as run-time dependencies. This isn’t currently enforced, but could be in the future.

##### `nativeBuildInputs`

A list of dependencies whose host platform is the new derivation’s build platform, and target platform is the new derivation’s host platform. These are programs and libraries used at build-time that, if they are a compiler or similar tool, produce code to run at run-time—i.e. tools used to build the new derivation. If the dependency doesn’t care about the target platform (i.e. isn’t a compiler or similar tool), put it here, rather than in `depsBuildBuild` or `depsBuildTarget`. This could be called `depsBuildHost`, but `nativeBuildInputs` is used for historical continuity.

Since these packages are able to be run at build-time, they are added to the `PATH`, as described above. But since these packages are only guaranteed to be able to run then, they shouldn’t persist as run-time dependencies. This isn’t currently enforced, but could be in the future.

##### `depsBuildTarget`

A list of dependencies whose host platform is the new derivation’s build platform, and target platform is the new derivation’s target platform. These are programs used at build time that produce code to run with code produced by the depending package. Most commonly, these are tools used to build the runtime or standard library that the currently-being-built compiler will inject into any code it compiles. In many cases, the currently-being-built-compiler is itself employed for that task, but when that compiler won’t run (i.e. its build and host platform differ) this is not possible. Other times, the compiler relies on some other tool, like binutils, that is always built separately so that the dependency is unconditional.

This is a somewhat confusing concept to wrap one’s head around, and for good reason. As the only dependency type where the platform offsets, `-1` and `1`, are not adjacent integers, it requires thinking of a bootstrapping stage _two_ away from the current one. It and its use-case go hand in hand and are both considered poor form: try to not need this sort of dependency, and try to avoid building standard libraries and runtimes in the same derivation as the compiler produces code using them. Instead strive to build those like a normal library, using the newly-built compiler just as a normal library would. In short, do not use this attribute unless you are packaging a compiler and are sure it is needed.

Since these packages are able to run at build time, they are added to the `PATH`, as described above. But since these packages are only guaranteed to be able to run then, they shouldn’t persist as run-time dependencies. This isn’t currently enforced, but could be in the future.

##### `depsHostHost`

A list of dependencies whose host and target platforms match the new derivation’s host platform. In practice, this would usually be tools used by compilers for macros or a metaprogramming system, or libraries used by the macros or metaprogramming code itself. It’s always preferable to use a `depsBuildBuild` dependency in the derivation being built over a `depsHostHost` on the tool doing the building for this purpose.

##### `buildInputs`

A list of dependencies whose host platform and target platform match the new derivation’s. This would be called `depsHostTarget` but for historical continuity. If the dependency doesn’t care about the target platform (i.e. isn’t a compiler or similar tool), put it here, rather than in `depsBuildBuild`.

These are often programs and libraries used by the new derivation at _run_ -time, but that isn’t always the case. For example, the machine code in a statically-linked library is only used at run-time, but the derivation containing the library is only needed at build-time. Even in the dynamic case, the library may also be needed at build-time to appease the linker.

##### `depsTargetTarget`

A list of dependencies whose host platform matches the new derivation’s target platform. These are packages that run on the target platform, e.g. the standard library or run-time deps of standard library that a compiler insists on knowing about. It’s poor form in almost all cases for a package to depend on another from a future stage [future stage corresponding to positive offset]. Do not use this attribute unless you are packaging a compiler and are sure it is needed.

##### `depsBuildBuildPropagated`

The propagated equivalent of `depsBuildBuild`. This perhaps never ought to be used, but it is included for consistency [see below for the others].

##### `propagatedNativeBuildInputs`

The propagated equivalent of `nativeBuildInputs`. This would be called `depsBuildHostPropagated` but for historical continuity. For example, if package `Y` has `propagatedNativeBuildInputs = [X]`, and package `Z` has `buildInputs = [Y]`, then package `Z` will be built as if it included package `X` in its `nativeBuildInputs`. Note that if instead, package `Z` has `nativeBuildInputs = [Y]`, then `X` will not be included at all.

##### `depsBuildTargetPropagated`

The propagated equivalent of `depsBuildTarget`. This is prefixed for the same reason of alerting potential users.

##### `depsHostHostPropagated`

The propagated equivalent of `depsHostHost`.

##### `propagatedBuildInputs`

The propagated equivalent of `buildInputs`. This would be called `depsHostTargetPropagated` but for historical continuity.

##### `depsTargetTargetPropagated`

The propagated equivalent of `depsTargetTarget`. This is prefixed for the same reason of alerting potential users.

##### `strictDeps`

When using native compilation, `stdenv` is lenient towards incorrect placement of a dependency into one of the dependency lists described above. That means a dependency needed at runtime often works, even if it is only present in `nativeBuildInputs`. Vice-versa, dependencies containing binaries that need to be executed during the build will work even if they are only listed in `buildInputs`.

While convenient for getting to a package quickly, this behavior can break cross-compilation. Adding `strictDeps = true` as a parameter to `mkDerivation` or any of its language specific wrappers disables this behavior.

The specialized `build*` functions for dlang, emacs, go, nim, ocaml, python, and rust enable this option by default.

## Shell functions and utilities 

[`makeWrapper` <executable> <wrapperfile> <args>](</manual/nixpkgs/stable/#fun-makeWrapper>)
[`remove-references-to -t` <storepath> [ `-t` <storepath> … ] <file> …](</manual/nixpkgs/stable/#fun-remove-references-to>)
[`runHook` <hook>](</manual/nixpkgs/stable/#fun-runHook>)
[`substitute` <infile> <outfile> <subs>](</manual/nixpkgs/stable/#fun-substitute>)
[`substituteInPlace` <multiple files> <subs>](</manual/nixpkgs/stable/#fun-substituteInPlace>)
[`substituteAll` <infile> <outfile>](</manual/nixpkgs/stable/#fun-substituteAll>)
[`substituteAllInPlace` <file>](</manual/nixpkgs/stable/#fun-substituteAllInPlace>)
[`stripHash` <path>](</manual/nixpkgs/stable/#fun-stripHash>)
[`wrapProgram` <executable> <makeWrapperArgs>](</manual/nixpkgs/stable/#fun-wrapProgram>)
[`prependToVar` <variableName> <elements…>](</manual/nixpkgs/stable/#fun-prependToVar>)
[`appendToVar` <variableName> <elements…>](</manual/nixpkgs/stable/#fun-appendToVar>)

The standard environment provides a number of useful functions.

### `makeWrapper` <executable> <wrapperfile> <args>

Constructs a wrapper for a program with various possible arguments. It is defined as part of 2 setup-hooks named `makeWrapper` and `makeBinaryWrapper` that implement the same bash functions. Hence, to use it you have to add `makeWrapper` to your `nativeBuildInputs`. Here’s an example usage:
    
    
    # adds `FOOBAR=baz` to `$out/bin/foo`’s environment
    makeWrapper $out/bin/foo $wrapperfile --set FOOBAR baz
    
    # Prefixes the binary paths of `hello` and `git`
    # and suffixes the binary path of `xdg-utils`.
    # Be advised that paths often should be patched in directly
    # (via string replacements or in `configurePhase`).
    makeWrapper $out/bin/foo $wrapperfile \
      --prefix PATH : ${lib.makeBinPath [ hello git ]} \
      --suffix PATH : ${lib.makeBinPath [ xdg-utils ]}
    

Packages may expect or require other utilities to be available at runtime. `makeWrapper` can be used to add packages to a `PATH` environment variable local to a wrapper.

Use `--prefix` to explicitly set dependencies in `PATH`.

### Note

`--prefix` essentially hard-codes dependencies into the wrapper. They cannot be overridden without rebuilding the package.

If dependencies should be resolved at runtime, use `--suffix` to append fallback values to `PATH`.

There’s many more kinds of arguments, they are documented in `nixpkgs/pkgs/build-support/setup-hooks/make-wrapper.sh` for the `makeWrapper` implementation and in `nixpkgs/pkgs/by-name/ma/makeBinaryWrapper/make-binary-wrapper.sh` for the `makeBinaryWrapper` implementation.

`wrapProgram` is a convenience function you probably want to use most of the time, implemented by both `makeWrapper` and `makeBinaryWrapper`.

Using the `makeBinaryWrapper` implementation is usually preferred, as it creates a tiny _compiled_ wrapper executable, that can be used as a shebang interpreter. This is needed mostly on Darwin, where shebangs cannot point to scripts, [due to a limitation with the `execve`-syscall](<https://stackoverflow.com/questions/67100831/macos-shebang-with-absolute-path-not-working>). Compiled wrappers generated by `makeBinaryWrapper` can be inspected with `less <path-to-wrapper>` \- by scrolling past the binary data you should be able to see the shell command that generated the executable and there see the environment variables that were injected into the wrapper.

However, `makeWrapper` is more flexible and implements more arguments. Use `makeWrapper` if you need the wrapper to use shell features (e.g. look up environment variables) at runtime.

### `remove-references-to -t` <storepath> [ `-t` <storepath> … ] <file> … 

Removes the references of the specified files to the specified store files. This is done without changing the size of the file by replacing the hash by `eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee`, and should work on compiled executables. This is meant to be used to remove the dependency of the output on inputs that are known to be unnecessary at runtime. Of course, reckless usage will break the patched programs. To use this, add `removeReferencesTo` to `nativeBuildInputs`.

As `remove-references-to` is an actual executable and not a shell function, it can be used with `find`. Example removing all references to the compiler in the output:
    
    
    {
      postInstall = ''
        find "$out" -type f -exec remove-references-to -t ${stdenv.cc} '{}' +
      '';
    }
    

### `runHook` <hook>

Execute <hook> and the values in the array associated with it. The array’s name is determined by removing `Hook` from the end of <hook> and appending `Hooks`.

For example, `runHook postHook` would run the hook `postHook` and all of the values contained in the `postHooks` array, if it exists.

### `substitute` <infile> <outfile> <subs>

Performs string substitution on the contents of <infile>, writing the result to <outfile>. The substitutions in <subs> are of the following form:

#### `--replace-fail` <s1> <s2>

Replace every occurrence of the string <s1> by <s2>. Will error if no change is made.

#### `--replace-warn` <s1> <s2>

Replace every occurrence of the string <s1> by <s2>. Will print a warning if no change is made.

#### `--replace-quiet` <s1> <s2>

Replace every occurrence of the string <s1> by <s2>. Will do nothing if no change can be made.

#### `--subst-var` <varName>

Replace every occurrence of `@varName@` by the contents of the environment variable <varName>. This is useful for generating files from templates, using `@...@` in the template as placeholders.

#### `--subst-var-by` <varName> <s>

Replace every occurrence of `@varName@` by the string <s>.

Example:
    
    
    substitute ./foo.in ./foo.out \
        --replace-fail /usr/bin/bar $bar/bin/bar \
        --replace-fail "a string containing spaces" "some other text" \
        --subst-var someVar
    

### `substituteInPlace` <multiple files> <subs>

Like `substitute`, but performs the substitutions in place on the files passed.

### `substituteAll` <infile> <outfile>

Replaces every occurrence of `@varName@`, where <varName> is any environment variable, in <infile>, writing the result to <outfile>. For instance, if <infile> has the contents
    
    
    #! @bash@/bin/sh
    PATH=@coreutils@/bin
    echo @foo@
    

and the environment contains `bash=/nix/store/bmwp0q28cf21...-bash-3.2-p39` and `coreutils=/nix/store/68afga4khv0w...-coreutils-6.12`, but does not contain the variable `foo`, then the output will be
    
    
    #! /nix/store/bmwp0q28cf21...-bash-3.2-p39/bin/sh
    PATH=/nix/store/68afga4khv0w...-coreutils-6.12/bin
    echo @foo@
    

That is, no substitution is performed for undefined variables.

Environment variables that start with an uppercase letter or an underscore are filtered out, to prevent global variables (like `HOME`) or private variables (like `__ETC_PROFILE_DONE`) from accidentally getting substituted. The variables also have to be valid bash “names”, as defined in the bash manpage (alphanumeric or `_`, must not start with a number).

### `substituteAllInPlace` <file>

Like `substituteAll`, but performs the substitutions in place on the file <file>.

### `stripHash` <path>

Strips the directory and hash part of a store path, outputting the name part to `stdout`. For example:
    
    
    # prints coreutils-8.24
    stripHash "/nix/store/9s9r019176g7cvn2nvcw41gsp862y6b4-coreutils-8.24"
    

If you wish to store the result in another variable, then the following idiom may be useful:
    
    
    name="/nix/store/9s9r019176g7cvn2nvcw41gsp862y6b4-coreutils-8.24"
    someVar=$(stripHash $name)
    

### `wrapProgram` <executable> <makeWrapperArgs>

Convenience function for `makeWrapper` that replaces `<executable>` with a wrapper that executes the original program. It takes all the same arguments as `makeWrapper`, except for `--inherit-argv0` (used by the `makeBinaryWrapper` implementation) and `--argv0` (used by both `makeWrapper` and `makeBinaryWrapper` wrapper implementations).

If you will apply it multiple times, it will overwrite the wrapper file and you will end up with double wrapping, which should be avoided.

### `prependToVar` <variableName> <elements…>

Prepend elements to a variable.

Example:
    
    
    $ configureFlags="--disable-static"
    $ prependToVar configureFlags --disable-dependency-tracking --enable-foo
    $ echo $configureFlags
    --disable-dependency-tracking --enable-foo --disable-static
    

### `appendToVar` <variableName> <elements…>

Append elements to a variable.

Example:
    
    
    $ configureFlags="--disable-static"
    $ appendToVar configureFlags --disable-dependency-tracking --enable-foo
    $ echo $configureFlags
    --disable-static --disable-dependency-tracking --enable-foo
