# Crawl: kernel/config.html
- seed_url: https://www.erlang.org/doc/apps/kernel/config.html
- canonical_url: https://www.erlang.org/doc/apps/kernel/config.html
- family: Erlang/OTP kernel module docs
- fetch: 200
- otp_version: OTP 29.0.2 (kernel 11.0.2)
- feeds_docs: logger-and-config.md, applications.md

## Purpose
A *configuration file* contains values for configuration parameters for the
applications in the system. This page documents the on-disk `.config` file
format consumed by the Erlang runtime via the `-config` / `-configfd` command-line
arguments, the `sys.config` convention used in embedded mode and by release
handling, and the include-file mechanism for composing multiple `.config` files.

It is a *file-format* reference (akin to `config(4)`), NOT a module with
callable functions. There is no `config:string_to_term`, no `Config.Reader`,
and no `${VAR}` environment-variable interpolation on this page — those concepts
belong to Elixir releases / `config_provider`s, not to the OTP kernel config
file described here.

## Config file format (sys.config, term shape)
A `.config` file contains a **single Erlang term** ending in `.` with shape:

```erlang
[{Application1, [{Par11, Val11}, ...]},
 ...
 {ApplicationN, [{ParN1, ValN1}, ...]}].
```

- `Application = atom()` — application name.
- `Par = atom()` — name of a configuration parameter.
- `Val = term()` — value of a configuration parameter.

The file is named `Name.config` (any `Name`). In embedded mode exactly one
system configuration file is assumed, named `sys.config`, located in
`$ROOT/releases/Vsn` where `$ROOT` is the OTP root installation directory and
`Vsn` is the release version. Release handling relies on this assumption:
installing a new release version reads the new `sys.config` and uses it to
update application configurations.

Note: `sys.config.src` is NOT mentioned on this page (that is a
rebar3/distillery build-time source concept). This page covers only the
runtime `sys.config` / `Name.config` term format.

## Env-var expansion / interpolation
**None on this page.** No `${VAR}` / environment-variable interpolation is
described for `.config` files. The only environment-like reference is the
literal `$ROOT` placeholder denoting the OTP root installation directory (used
to describe the `sys.config` location), not a runtime expansion mechanism.

## Multiple config files: merge & override precedence
Precedence (lowest → highest):

1. **Application resource files** (`app(4)`, the `.app` files) — baseline.
2. **Configuration files / file descriptors** given via `-config` / `-configfd`.
   - Data is read **in command-line order**. Example:
     `erl -config a -configfd 3 -config b -configfd 4` reads `a.config`, fd `3`,
     `b.config`, fd `4` in that order.
   - If a parameter is specified more than once across files/fds, **the last
     one overrides the previous ones**.
3. **Command-line flags** (`erts:erl(1)`) — always override config-file values.

So: `app(4) < -config/-configfd (last wins) < command-line flags`.

### Include-file mechanism (within sys.config / -configfd)
A `sys.config` (or a `-configfd` configuration) may include other `.config`
files. The extended syntax is:

```
[{Application, [{Par, Val}]} | IncludeFile].
```

- `IncludeFile = string()` — name of a `.config` file; the `.config` extension
  can be omitted. Absolute paths recommended.
- Relative-path resolution:
  - In a `sys.config`: searched first relative to the `sys.config` directory,
    then relative to the emulator's current working directory.
  - In a `-configfd` configuration: searched first relative to the directory
    containing the boot script (see `-boot`), then relative to the emulator's
    current working directory.

### Merge semantics
When traversing a `sys.config` / `-configfd` configuration:
- A filename entry → its contents are read and **merged** with the result so far.
- An `{Application, Env}` tuple → **merged** with the result so far.
- "Merging means that new parameters are added and existing parameter values
  are overwritten."

Worked example from the page:
```
sys.config:
["/home/user/myconfig1"
 {myapp,[{par1,val1},{par2,val2}]},
 "/home/user/myconfig2"].

myconfig1.config:
[{myapp,[{par0,val0},{par1,val0},{par2,val0}]}].

myconfig2.config:
[{myapp,[{par2,val3},{par3,val4}]}].
```
Yields environment for `myapp`:
```
[{par0,val0},{par1,val1},{par2,val3},{par3,val4}]
```
(par0 added from myconfig1; par1 overwritten by sys.config to val1; par2
overwritten by sys.config to val2 then by myconfig2 to val3; par3 added from
myconfig2.)

### Error handling for includes
- The run-time system **aborts before startup** if an include file specified in
  `sys.config` or a `-configfd` configuration does not exist or is erroneous.
- However, **installing a new release version will NOT fail** on an include-file
  error; an error message is returned and the erroneous file is ignored.

## Functions (exact arities)
None. This page documents a file format, not an Erlang module API. The only
function referenced for *retrieving* configured values is
`application:get_env/1,2` (defined in the `kernel` `application` module, not
here).

## Strict rules
- A `.config` file is a **single Erlang term** terminated by `.`.
- Term shape must be `[{Application, [{Par, Val}]}]` (plus optional `IncludeFile`
  string entries when used as `sys.config` / `-configfd`).
- `Application` and `Par` are atoms; `Val` is any term.
- In embedded mode exactly one `sys.config` is assumed, in
  `$ROOT/releases/Vsn`.
- Specifying additional `.config` files beyond `sys.config` in embedded/release
  mode leads to **inconsistent updates** of application configurations — use the
  include-file mechanism instead.
- Config-file values override `.app` resource files but are always overridden by
  command-line flags.
- Across multiple `-config`/`-configfd` sources, **last wins** (command-line
  order).
- Include-file merge: new params added, existing param values overwritten.
- Missing/erroneous include file at startup → runtime aborts; at release
  install → file ignored, error returned, install continues.

## Verbatim quotes
- "A *configuration file* contains values for configuration parameters for the
  applications in the system."
- "The `erl` command-line argument `-config Name` tells the system to use data
  in the system configuration file `Name.config`."
- "The erl command-line argument `-configfd` works the same way as the
  `-config` option but specifies a file descriptor to read configuration data
  from instead of a file."
- "The configuration data from configuration files and file descriptors are
  read in the same order as they are given on the command line."
- "If a configuration parameter is specified more than once in the given files
  and file descriptors, the last one overrides the previous ones."
- "Configuration parameter values in a configuration file or file descriptor
  override the values in the application resource files (see `app(4)`)."
- "The values in the configuration file are always overridden by command-line
  flags (see `erts:erl(1)`)."
- "The value of a configuration parameter is retrieved by calling
  `application:get_env/1,2`."
- "When starting Erlang in embedded mode, it is assumed that exactly one system
  configuration file is used, named `sys.config`. This file is to be located in
  `$ROOT/releases/Vsn`..."
- "Release handling relies on this assumption. When installing a new release
  version, the new `sys.config` is read and used to update the application's
  configurations."
- "specifying another `.config` file, or more `.config` files, leads to an
  inconsistent update of application configurations."
- "Merging means that new parameters are added and existing parameter values are
  overwritten."
- "The run-time system will abort before staring up if an include file specified
  in `sys.config` or a `-configfd` configuration does not exist, or is
  erroneous. However, installing a new release version will not fail if there is
  an error while loading an include file, but an error message is returned and
  the erroneous file is ignored."

## Version notes
- Page generated by ExDoc v0.40.3.
- Project: kernel v11.0.2.
- OTP major version: 29 (meta `major-vsn` = 29); page title
  "config — OTP 29.0.2 (kernel 11.0.2)".
- Source: `github.com/erlang/otp` at tag `OTP-29.0.2`,
  `lib/kernel/doc/references/config.md`.
- Copyright © 1996-2026 Ericsson AB.
- No `sys.config.src`, no `Config.Reader`, no `config:string_to_term`, no
  `${VAR}` interpolation — these are absent from this OTP kernel page (they are
  Elixir/rebar3 release concepts).

## Discovered links
### Relevant (crawl later)
- https://www.erlang.org/doc/apps/kernel/app.html — `app(4)` application resource file format (baseline overridden by config).
- https://www.erlang.org/doc/apps/erts/erl_cmd.html — `erts:erl(1)` command-line flags incl. `-config`, `-configfd`, `-boot`.
- https://www.erlang.org/doc/apps/erts/erl_cmd.html#config — `-config Name` anchor.
- https://www.erlang.org/doc/apps/erts/erl_cmd.html#configfd — `-configfd` anchor.
- https://www.erlang.org/doc/apps/erts/erl_cmd.html#boot — `-boot` (boot script dir for -configfd relative includes).
- https://www.erlang.org/doc/system/design_principles.html — OTP Design Principles.
- https://www.erlang.org/doc/apps/sasl/script.html — boot script (`script`) referenced for -configfd relative include resolution.
- https://www.erlang.org/doc/system/typespec.html — Erlang type language (referenced for include syntax description).

### Skipped
- https://www.erlang.org/doc/apps/kernel/search.html — search UI.
- https://www.erlang.org/doc/apps/kernel/llms.txt — llms index.
- https://www.erlang.org/doc/apps/kernel/kernel.epub — ePub download.
- https://github.com/erlang/otp/blob/OTP-29.0.2/lib/kernel/doc/references/config.md — source mirror (already captured verbatim).
- https://github.com/elixir-lang/ex_doc — tooling.
- https://www.erlang.org — site root.
- https://www.ericsson.com — copyright holder.
