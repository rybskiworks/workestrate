---
name: constraint-beam-nif-safety
description: |
  Enforces BEAM NIF safety invariants during code execution — dirty schedulers
  for unbounded work, no long NIFs (scheduler latency), NIF crash = VM crash,
  type safety at FFI boundaries, and validation of untrusted binary_to_term.
  Load when writing or reviewing NIFs in Erlang, Elixir, or Gleam-on-BEAM. Does
  NOT cover general process isolation (see constraint-beam-process-isolation).
metadata:
  org.kind: constraint
---

# Constraint: BEAM NIF Safety

This constraint enforces the cardinal NIF safety rules. A NIF runs as a direct
extension of the VM with no preemption and no memory protection; a crashing NIF
crashes the whole VM. Violations stall schedulers or take down the runtime.

## Triggers

Load this skill when:

- Writing or reviewing a NIF (`ERL_NIF_INIT`, `enif_*` calls, `-nifs` attribute).
- Deciding whether work belongs in a NIF, a dirty NIF, or a port.
- Reviewing FFI boundaries or `binary_to_term` on untrusted data.

## Rules

1. A NIF that may run longer than ~1 ms MUST be dirty
   (`ERL_NIF_DIRTY_JOB_CPU_BOUND` or `ERL_NIF_DIRTY_JOB_IO_BOUND`), split into
   yielding chunks (`enif_schedule_nif` + `enif_consume_timeslice`), or offloaded
   to a managed thread. Classify dirty jobs correctly — misclassifying CPU-bound
   as IO-bound can starve ordinary schedulers.
2. A NIF crash crashes the whole VM — there is no memory protection and no
   preemption inside a NIF.
3. Every NIF MUST have an Erlang stub (`erlang:nif_error/1`); the `-nifs([f/A])`
   attribute declares NIFs; `-on_load({init,0})` loads the library; check the
   `erlang:load_nif/2` return value.
4. The 4th `ERL_NIF_INIT` argument MUST be `NULL` (the `reload` callback was
   removed in OTP 20).
5. `enif_open_resource_type` may ONLY be called in `load`/`upgrade`; create atoms
   during `load`/`upgrade` and store them in static globals (atoms from
   `load`/`upgrade` are valid in any environment).
6. Never store a process-bound environment pointer across NIF calls (valid only
   in the calling thread until the NIF returns); pair each `enif_keep_resource`
   with an `enif_release_resource`.
7. Return native data via resource handles (`enif_make_resource`), never as raw
   native pointers.
8. Treat NIFs as a last resort — "Rewriting Erlang code to a NIF to make it
   faster should be seen as a last resort." Prefer ports/linked-in drivers for
   blocking I/O and long-running work.
9. On untrusted serialized data, use `binary_to_term/2` with the `safe` option
   (prevents new atoms) AND still validate/sanitize the result; prefer JSON/XML
   for untrusted input. Never `list_to_atom`/`binary_to_atom` on untrusted input
   (atom exhaustion crashes the VM).

## References

- Operational skill: `beam-observability-debugging`.
- Docs: `docs/beam/nifs.md`, `docs/beam/common-mistakes.md`.

## Out of scope

- General process isolation and message passing — see
  `constraint-beam-process-isolation`.
- Supervision of NIF-loading modules — see `constraint-beam-supervision`.

## Violation examples

### Long NIF on an ordinary scheduler

```c
/* FORBIDDEN: >1 ms NIF without dirty flag or yielding */
static ERL_NIF_TERM heavy(ErlNifEnv* env, int argc, const ERL_NIF_TERM argv[]) {
    /* ... long computation, no enif_consume_timeslice ... */
}
static ErlNifFunc funcs[] = { {"heavy", 1, heavy, 0} };  /* flags=0, not dirty */
```

Correct: set the dirty flag (`ERL_NIF_DIRTY_JOB_CPU_BOUND`) or yield via
`enif_schedule_nif` + `enif_consume_timeslice`.

### `enif_open_resource_type` outside `load`/`upgrade`

```c
/* FORBIDDEN: only callable in load/upgrade */
static ERL_NIF_TERM make(ErlNifEnv* env, int argc, const ERL_NIF_TERM argv[]) {
    enif_open_resource_type(env, NULL, "my_res", NULL, ERL_NIF_RT_CREATE, NULL);
}
```

Correct: open the resource type once in `load`/`upgrade`, store the type in a
static global.

### `binary_to_term/1` on untrusted data

```erlang
%% FORBIDDEN: atom exhaustion and harmful-term injection
Term = binary_to_term(UntrustedBinary)
```

Correct: `binary_to_term(UntrustedBinary, [safe])` and still validate/sanitize the
result; prefer JSON for untrusted input.

### Missing Erlang stub

```erlang
%% FORBIDDEN: NIF without a stub fails to load cleanly
-export([fast_hash/1]).
%% no -nifs, no stub, no -on_load
```

Correct: `-nifs([fast_hash/1])`,
`fast_hash(_B) -> erlang:nif_error({nif_not_loaded, ?MODULE}).`,
`-on_load({init,0})`, `init() -> erlang:load_nif(?MODULE, 0).`.

## How to check

```bash
# Runtime: trace:system/3 with long_schedule monitor flags NIFs exceeding ~1 ms.
#          erlang:system_info({dirty_cpu_schedulers, ...}) for dirty scheduler sizing.
#          erlang:system_info(atom_count) / atom_limit for atom-exhaustion watch.
```

Manual review:

- Every NIF returns within ~1 ms or is dirty/split/threaded.
- Dirty jobs classified CPU-bound vs IO-bound correctly.
- Resource types opened only in `load`/`upgrade`; atoms created at load time.
- `enif_keep_resource` paired with `enif_release_resource`; no stored
  process-bound env pointers.
- `binary_to_term/2` with `safe` on any untrusted data; no
  `list_to_atom`/`binary_to_atom` on untrusted input.
