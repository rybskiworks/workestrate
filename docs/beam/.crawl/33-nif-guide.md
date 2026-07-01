# Crawl: system/nif.html
- seed_url: https://www.erlang.org/doc/system/nif.html
- canonical_url: https://www.erlang.org/doc/system/nif.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: OTP 29.0.2 (major-vsn 29; "Erlang System Documentation v29.0.2")
- feeds_docs: nifs-ffi.md

## Purpose
This page is the **NIF tutorial example** in the Erlang/OTP System Documentation
(Tutorial section). It walks through solving the "Problem Example" (complex6)
using Native Implemented Functions. It is a *worked example*, not the full NIF
guidance/reference. It complements the `erl_nif` C API reference page
(`erts/erl_nif.html`), which holds the deeper guidance (dirty NIFs, resource
objects, versioning, static NIFs).

Stated purpose of NIFs:
> "NIFs are a simpler and more efficient way of calling C-code than using port
> drivers. NIFs are most suitable for synchronous functions ... that do some
> relatively short calculations without side effects and return the result."

A NIF is a function implemented in C instead of Erlang; it appears as any other
function to callers, belongs to a module, and is compiled/linked into a
dynamically loadable shared library (SO on UNIX, DLL on Windows) that must be
loaded at runtime by the Erlang code of the module.

## When to use / not use a NIF (guidance)
**Use when:** synchronous functions doing relatively short calculations, no side
effects, returning a result. NIFs are the fastest way to call C from Erlang
(alongside port drivers) because calling a NIF requires no context switches.

**Do NOT use / caution:** NIFs are the *least safe* interop mechanism because
the NIF library is dynamically linked into the emulator process — a crash in a
NIF brings the whole emulator down.

> "As a NIF library is dynamically linked into the emulator process, this is the
> fastest way of calling C-code from Erlang (alongside port drivers). Calling
> NIFs requires no context switches. But it is also the least safe, because a
> crash in a NIF brings the emulator down too."

NOTE: This page does NOT contain the broader "prefer ports/linked-in drivers
for blocking I/O" recommendation, nor dirty-NIF latency guidance — those are on
the `erl_nif` reference page, not here.

## Load/upgrade flow
- An Erlang module is always required even if all functions are NIFs, for two
  reasons:
  1. The NIF library must be explicitly loaded by Erlang code in the same module.
  2. All NIFs of a module must have an Erlang implementation as well (normally
     minimal stubs that throw; can also serve as fallbacks for architectures
     without native implementations).
- Loading is done via `erlang:load_nif/2` (name of shared lib + an arbitrary
  init term passed to the library).
- The `-on_load(...)` directive runs `init` automatically when the module loads;
  if `init` returns anything other than `ok` (e.g. NIF load fails), the module
  is unloaded and calls to its functions fail.
- Loading the NIF library overrides the stub implementations; calls are then
  dispatched to the NIF implementations.
- On the C side, `ERL_NIF_INIT(module, nif_funcs, load, reload, upgrade, unload)`
  takes the module name as a C identifier (stringified by the macro), the
  `ErlNifFunc` array, and **four callback pointers** for library initialization
  (load/reload/upgrade/unload). In this simple example all four are `NULL`.
  These callbacks are the hook points for the load/upgrade lifecycle (full
  semantics documented on `erl_nif` reference page, not here).

## Resource objects pattern
NOT covered on this page. Resource objects (`enif_alloc_resource`,
`ErlNifResourceType`, `enif_make_resource`, destructors) are documented on the
`erl_nif` C API reference page (`erts/erl_nif.html`), not in this tutorial.

## Dirty NIF guidance
NOT covered on this page. Dirty schedulers / dirty NIFs
(`ERL_NIF_DIRTY_JOB_CPU`, `ERL_NIF_DIRTY_JOB_IO`, `enif_schedule_nif`) are
documented on the `erl_nif` reference page. This tutorial only shows normal
(scheduler-bound) synchronous NIFs.

## Ports vs NIFs guidance
This page only makes the comparative claim that NIFs are "a simpler and more
efficient way of calling C-code than using port drivers" and that NIFs + port
drivers are the fastest (no context switch) but NIFs are the least safe
(emulator crash on NIF crash).

The fuller guidance — *prefer ports / linked-in drivers for blocking I/O and
long-running work; use dirty NIFs only when a port is impractical* — is NOT on
this page; it lives on the `erl_nif` reference page.

## Versioning/stability
NOT covered on this page. NIF API versioning (`ERL_NIF_MAJOR_VERSION` /
`ERL_NIF_MINOR_VERSION`, `enif_system_info`, the `ERL_NIF_INIT` version checks,
API stability guarantees) is documented on the `erl_nif` reference page.

The `-nifs()` / `-nifs` module attribute: shown in the example
(`-nifs([foo/1, bar/1]).`) — declares which functions in the module are NIFs.
Static NIFs: NOT covered on this page.

## Verbatim quotes (the key recommendation quotes)
- "NIFs are a simpler and more efficient way of calling C-code than using port
  drivers. NIFs are most suitable for synchronous functions, such as `foo` and
  `bar` in the example, that do some relatively short calculations without side
  effects and return the result."
- "A NIF is a function that is implemented in C instead of Erlang. NIFs appear
  as any other functions to the callers. They belong to a module and are called
  like any other Erlang functions."
- "As a NIF library is dynamically linked into the emulator process, this is the
  fastest way of calling C-code from Erlang (alongside port drivers). Calling
  NIFs requires no context switches. But it is also the least safe, because a
  crash in a NIF brings the emulator down too."
- "Even if all functions of a module are NIFs, an Erlang module is still needed
  for two reasons: The NIF library must be explicitly loaded by Erlang code in
  the same module. All NIFs of a module must have an Erlang implementation as
  well."
- "Normally these are minimal stub implementations that throw an exception. But
  they can also be used as fallback implementations for functions that do not
  have native implementations on some architectures."
- "Loading the NIF library overrides the stub implementations and cause calls to
  `foo` and `bar` to be dispatched to the NIF implementations instead."
- "The remaining arguments [of ERL_NIF_INIT] are pointers to callback functions
  that can be used to initialize the library."

## Version notes
- Page built with ExDoc v0.40.3; project "Erlang System Documentation v29.0.2";
  OTP 29.0.2; major-vsn 29.
- Source: github.com/erlang/otp @ OTP-29.0.2, `system/doc/tutorial/nif.md`.
- Copyright © 1996-2026 Ericsson AB.
- This is the **Tutorial** NIF page (complex6 example). The deeper NIF guidance
  (dirty NIFs, resource objects, versioning, static NIFs, ports-vs-NIFs
  recommendation) is on the `erl_nif` C API reference page, which should be
  crawled separately to feed `nifs-ffi.md`.

## Discovered links

### Relevant (crawl later)
- https://www.erlang.org/doc/system/example.html — "Problem Example" (the
  canonical tutorial problem this page solves; useful context for the
  port/driver/NIF comparison series).
- https://www.erlang.org/doc/system/debugging.html — "Debugging NIFs and Port
  Drivers" (next page; directly relevant to NIF/FFI guidance).
- https://www.erlang.org/doc/apps/erts/erl_nif.html — `erl_nif` C API reference
  (NOT directly linked on this page, but the canonical companion that holds the
  dirty NIF / resource object / versioning / static-NIF guidance the task asks
  for; highest-priority next crawl for `nifs-ffi.md`).
- https://www.erlang.org/doc/apps/erts/erlang.html#load_nif/2 —
  `erlang:load_nif/2` (load entry point referenced in the example).

### Skipped
- https://www.erlang.org/doc/system/cnode.html — "C Nodes" (previous page;
  off-topic for NIF/FFI).
- https://www.erlang.org/doc/system/nif.md — markdown mirror of this same page.
- https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/tutorial/nif.md —
  source view of this page.
- https://www.erlang.org/doc/system/llms.txt, *.epub, ex_doc, erlang.org,
  ericsson.com — site chrome / non-content.
