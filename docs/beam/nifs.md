# NIFs (Native Implemented Functions)

## Purpose

A NIF library contains native (C) implementations of some functions of an Erlang
module. NIFs are called like any other Erlang function with no difference to the
caller. This doc covers `ERL_NIF_INIT` and lifecycle callbacks, dirty schedulers,
resource objects, environments/term construction, the cardinal safety rules,
versioning/static NIFs, and ports-vs-NIFs guidance.

## Sources used

- Crawl `25-erl-nif.md` — erts `erl_nif.html` — https://www.erlang.org/doc/apps/erts/erl_nif.html
- Crawl `33-nif-guide.md` — system `nif.html` — https://www.erlang.org/doc/system/nif.html

## Core guidance

### What NIFs are; when to use

"NIFs are a simpler and more efficient way of calling C-code than using port
drivers. NIFs are most suitable for synchronous functions ... that do some
relatively short calculations without side effects and return the result." "As a
NIF library is dynamically linked into the emulator process, this is the fastest
way of calling C-code from Erlang (alongside port drivers). Calling NIFs requires
no context switches. But it is also the least safe, because a crash in a NIF
brings the emulator down too."

An Erlang module is always required even if all functions are NIFs: (1) the NIF
library must be explicitly loaded by Erlang code in the same module via
`erlang:load_nif/2`; (2) all NIFs must have an Erlang stub implementation
(normally `erlang:nif_error/1`). The `-nifs([f/A])` attribute declares which
functions are NIFs. The `-on_load(...)` directive runs `init` automatically; if
it returns non-`ok`, the module is unloaded.

### ERL_NIF_INIT + lifecycle callbacks

```c
ERL_NIF_INIT(MODULE, ErlNifFunc funcs[], load, NULL, upgrade, unload)
```
- `MODULE`: Erlang module name as a C identifier (stringified by the macro).
- `funcs`: static array of `ErlNifFunc { name, arity, fptr, flags }`. `flags` is
  `0` for regular NIFs, or a dirty flag for dirty NIFs.
- The 4th argument MUST be `NULL` — it was the deprecated `reload` callback,
  "no longer supported since OTP 20."

Lifecycle callbacks (all receive a callback environment — a temporary
pseudo-process that "terminates" when the callback returns):
- `int (*load)(caller_env, void** priv_data, ERL_NIF_TERM load_info)` — called
  when the library is loaded and no prior library exists. `*priv_data` initialized
  to NULL; set it to keep state (retrieve via `enif_priv_data`). Library fails to
  load if `load` returns non-zero. **`enif_open_resource_type` may ONLY be called
  in `load`/`upgrade`.**
- `int (*upgrade)(caller_env, void** priv_data, void** old_priv_data, load_info)`
  — called when old code with a loaded NIF library exists. Library fails to load
  if `upgrade` returns non-zero OR if `upgrade` is NULL.
- `void (*unload)(caller_env, void* priv_data)` — called when the module instance
  is purged as old.
- `reload` — DEPRECATED, removed in OTP 20. Pass NULL.

`load` and `upgrade` are thread-safe even for shared state; ordinary NIFs are
thread-safe only when acting as pure functions reading their arguments.

### Dirty schedulers (CPU-bound vs IO-bound; when mandatory)

"A NIF that cannot be split and cannot execute in a millisecond or less is
called a 'dirty NIF'." Dirty NIFs run on separate dirty schedulers. Two job
classes:
- `ERL_NIF_DIRTY_JOB_CPU_BOUND` — CPU-bound work.
- `ERL_NIF_DIRTY_JOB_IO_BOUND` — I/O-bound work (expected to block on I/O).

"It is important to classify the dirty job correct. ... If you should classify
CPU bound jobs as I/O bound jobs, dirty I/O schedulers might starve ordinary
schedulers." A job alternating between classes can be reclassified via
`enif_schedule_nif`.

Two ways to declare/schedule: (1) set `flags` in `ErlNifFunc` to a dirty flag;
(2) call `enif_schedule_nif(caller_env, name, flags, fp, argc, argv)` from inside
a NIF (the calling NIF must use the return value as its own return).

When mandatory: any NIF that may run longer than ~1 ms MUST be dirty, OR split
into chunks (yielding NIF via `enif_schedule_nif` of regular NIFs — preferred),
OR offloaded to a managed thread (Threaded NIF). `enif_consume_timeslice(env,
percent)` (1..100) is the cooperative-scheduling hint; lengthy NIFs should call
it frequently and return when it returns 1.

Caveats while a process executes a dirty NIF: suspend/GC cannot happen until it
returns; process termination releases name/ETS/links but does NOT stop the NIF
(check liveness via `enif_is_current_process_alive`); heap/PCB deallocation is
delayed until the dirty NIF completes.

### Resource objects

Resource objects are the SAFE way to return pointers to native data from a NIF.
A resource is allocated with `enif_alloc_resource`; a "safe pointer" handle term
is returned via `enif_make_resource`; `enif_get_resource` recovers a
guaranteed-valid pointer. "A resource object is not deallocated until the last
handle term is garbage collected by the VM and the resource is released with
enif_release_resource (not necessarily in that order)."

Key functions:
- `enif_open_resource_type(env, module_str/*NULL*/, name, dtor, flags, *tried)`
  — create/take over a resource type. `flags`: `ERL_NIF_RT_CREATE` and/or
  `ERL_NIF_RT_TAKEOVER`. **Only callable in `load`/`upgrade`.**
- `enif_open_resource_type_x` (OTP 20.0) — accepts `dtor`, `stop`, `down`.
- `enif_init_resource_type` (OTP 24.0) — adds `dyncall`.
- `enif_alloc_resource(type, size) -> void*`.
- `enif_make_resource(env, obj) -> ERL_NIF_TERM` — opaque handle; no ownership
  transfer; still needs `enif_release_resource` (may be called immediately).
- `enif_get_resource(env, term, type, void** objp) -> int` — recover pointer;
  does NOT add a reference.
- `enif_release_resource(obj)` — remove a reference; each `release` must balance
  a prior `alloc` or `keep`. References from `enif_make_resource` can ONLY be
  removed by the GC.
- `enif_keep_resource(obj)` (OTP R14B) — add a reference; each `keep` must
  balance a `release`.
- `enif_monitor_process(caller_env, obj, target_pid, ErlNifMonitor* mon)` (OTP
  20.0) — monitor a process from a resource; `down` callback on exit.
- `enif_make_resource_binary(env, obj, data, size)` (OTP R14B) — binary term
  memory-managed by a resource.

Destructor: `void ErlNifResourceDtor(caller_env, void* obj)` — guaranteed last
callback before deallocation. Upgrade semantics: a loaded NIF library can take
over an existing resource type and inherit all objects; unloading is postponed as
long as resource objects with a destructor exist.

### Environments and term construction (summary)

`ERL_NIF_TERM` — opaque; valid only as API args/return values, belongs to an
`ErlNifEnv`. Three environment kinds:
1. **Process-bound env** — 1st arg to every NIF; valid only in the calling thread
   until the NIF returns. Never store pointers to it across calls.
2. **Callback env** — passed to `load`/`upgrade`/`unload`/`dtor`/`down`/`stop`;
   temporary pseudo-process.
3. **Process-independent env** — `enif_alloc_env` (OTP R14B); stores terms between
   calls, sends via `enif_send`; valid until `enif_free_env`/`enif_send`.

Atoms created during `load`/`upgrade` are special: valid as a term in ANY
`ErlNifEnv`. Best practice: create all atoms during loading, store in static
globals. Term construction: `enif_make_*` (atom/int/tuple/list/map/resource/...),
inspection `enif_get_*`, queries `enif_is_*`. Binaries via `ErlNifBinary`,
`enif_make_new_binary`, `enif_inspect_binary`,
`enif_inspect_iolist_as_binary`. Bitstrings with arbitrary bit length are NOT
supported.

### Cardinal rules

1. **Latency / no preemption**: "A native function is executed as a direct
   extension of the native code of the VM. ... The VM cannot provide the same
   services as provided when executing Erlang code, such as pre-emptive
   scheduling or memory protection." A well-behaving NIF returns within ~1 ms.
2. **Crash = VM crash**: "A native function that crashes will crash the whole
   VM."
3. **No safe environment**: no preemptive scheduling, no memory protection.
4. **Resource lifetime**: deallocated only when last handle term is GC'd AND
   `enif_release_resource` is called; pair each `enif_keep_resource` with a
   `enif_release_resource`.
5. **Environment lifetime**: process-bound envs valid only in the calling thread
   until return; never store pointers across calls (atoms from `load`/`upgrade`
   excepted).
6. **Concurrency**: thread-safe without synchronization only as a pure function;
   writes to shared state require your own synchronization.

### Versioning / static NIFs

`ERL_NIF_MAJOR_VERSION` (incremented on incompatible runtime changes) and
`ERL_NIF_MINOR_VERSION` (incremented when features are added). The runtime
refuses to load a library if major versions differ, or if majors are equal and
the library's minor version is greater than the runtime's. Old libraries with
lower majors are allowed for a transition of two major releases after a bump. A
loaded NIF library is tied to the Erlang module instance; on upgrade the new
instance must load its own library. Use `*priv_data` to keep per-version private
state. Static NIFs (`--enable-static-nifs`): define `STATIC_ERLANG_NIF_LIBNAME`
before including `erl_nif.h`.

### Ports vs NIFs guidance

From the NIF tutorial: NIFs are "a simpler and more efficient way of calling
C-code than using port drivers" and (with port drivers) the fastest (no context
switch), "but it is also the least safe, because a crash in a NIF brings the
emulator down too." The fuller guidance — prefer ports/linked-in drivers for
blocking I/O and long-running work; use dirty NIFs only when a port is
impractical — lives on the `erl_nif` reference page (crawl 25): yielding/splitting
is always preferred over dirty NIFs; "Rewriting Erlang code to a NIF to make it
faster should be seen as a last resort" (Common Caveats, crawl 34).

## Practical rules

- Every NIF must have an Erlang stub (`erlang:nif_error/1`).
- The 4th `ERL_NIF_INIT` argument MUST be `NULL` (reload removed in OTP 20).
- `enif_open_resource_type` only in `load`/`upgrade`.
- A NIF that may exceed ~1 ms MUST be dirty, split, or threaded.
- Classify dirty jobs correctly (CPU-bound vs IO-bound).
- Never store process-bound env pointers across NIF calls.
- Create atoms in `load`/`upgrade`; store in static globals.
- Pair `enif_keep_resource` with `enif_release_resource`.
- Use `enif_consume_timeslice` for cooperative scheduling in lengthy NIFs.

## Review checklist

- [ ] Does every NIF return within ~1 ms, or is it dirty/split/threaded?
- [ ] Are dirty jobs classified CPU-bound vs IO-bound correctly?
- [ ] Are resource types opened only in `load`/`upgrade`?
- [ ] Are atoms created at load time, not per-call?
- [ ] Is `*priv_data` used for per-version state?
- [ ] Are NIF stubs present and `erlang:nif_error/1`-based?

## Implementation checklist

- [ ] `-nifs([f/A])` attribute declares NIF functions.
- [ ] `-on_load({init, 0})` loads the library at module load.
- [ ] `ERL_NIF_INIT` with `load`, `NULL`, `upgrade`, `unload`.
- [ ] `enif_consume_timeslice` called in lengthy regular NIFs.
- [ ] Resource handles returned via `enif_make_resource`.
- [ ] `enif_release_resource` paired with `enif_alloc_resource`/`enif_keep_resource`.

## Runtime / debugging checklist

- [ ] `erlang:load_nif/2` return value checked (`ok` vs `{error, _}`).
- [ ] `enif_is_current_process_alive` checked in long dirty NIFs.
- [ ] `trace:system/3` `long_schedule` monitor flags NIFs exceeding ~1 ms.
- [ ] Watch for VM-wide stalls correlating with NIF calls (no preemption).

## Validation hooks

- After `erlang:load_nif/2`, assert `ok` (module usable) vs `{error, Reason}`.
- Assert NIF stubs raise `nif_error` when the library is not loaded.
- In tests, assert dirty NIFs do not stall ordinary schedulers under load.
- Assert resource destructors fire on GC + release (via test resource type).

## Examples

Erlang side:
```erlang
-module(my_nif).
-nifs([fast_hash/1]).
-on_load({init, 0}).
-export([fast_hash/1]).
fast_hash(_Bin) -> erlang:nif_error({nif_not_loaded, ?MODULE}).
init() -> ok = erlang:load_nif(?MODULE, 0).
```

C side (sketch):
```c
static ERL_NIF_TERM fast_hash(ErlNifEnv* env, int argc, const ERL_NIF_TERM argv[]) {
    ErlNifBinary bin;
    if (!enif_inspect_binary(env, argv[0], &bin)) return enif_make_badarg(env);
    /* ... compute ... */
    return enif_make_int64(env, result);
}
static int load(ErlNifEnv* env, void** priv, ERL_NIF_TERM info) { *priv = NULL; return 0; }
static ErlNifFunc funcs[] = { {"fast_hash", 1, fast_hash, 0} };
ERL_NIF_INIT(my_nif, funcs, load, NULL, NULL, NULL);
```

## Common mistakes

- Running a >1 ms NIF on an ordinary scheduler (VM stalls).
- Misclassifying CPU-bound work as IO-bound (scheduler starvation).
- Storing a process-bound env pointer across NIF calls (dangling).
- Creating atoms per-call instead of at load (wasteful, not global).
- Forgetting the Erlang stub (NIF load fails for unused local stubs).
- Returning native pointers directly instead of resource handles (unsafe).

## Strict vs contextual guidance

Strict:
- NIF crash = VM crash; no preemption; no memory protection.
- `enif_open_resource_type` only in `load`/`upgrade`.
- 4th `ERL_NIF_INIT` arg MUST be `NULL`.
- NIFs > ~1 ms MUST be dirty/split/threaded.

Contextual:
- Dirty NIF vs yielding NIF vs Threaded NIF — prefer yielding, then dirty.
- `enif_consume_timeslice` granularity — tune by NIF length.
- Static NIFs vs dynamic — choose by deployment model.

## Policy decisions for individual repos

- Whether NIFs are permitted at all (and the review bar).
- Maximum regular-NIF duration before dirty is mandatory.
- Whether dirty CPU/IO schedulers are sized for the workload.
- Required `ERL_NIF_MAJOR_VERSION` baseline for supported OTP range.

## Related docs

- `ports-io.md` — ports and linked-in drivers (the safer interop alternative).
- `runtime-debugging.md` — `long_schedule` monitoring for NIFs.
- `validation.md` — NIF load and latency validation hooks.
- `common-mistakes.md` — NIF overuse caveat.

## Related skills

- `beam-observability-debugging`
- `beam-processes`
- `beam-errors-failures`
- `beam-applications-releases`
- `beam-logger-config`
