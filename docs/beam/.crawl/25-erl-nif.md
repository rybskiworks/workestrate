# Crawl: erts/erl_nif.html (focused)
- seed_url: https://www.erlang.org/doc/apps/erts/erl_nif.html
- canonical_url: https://www.erlang.org/doc/apps/erts/erl_nif.html
- family: Erlang/OTP ERTS NIF docs
- fetch: 200
- otp_version: OTP 29.0.2 (erts 17.0.2)
- feeds_docs: nifs-ffi.md, externals/interop topic

## Purpose
A NIF library contains native (C) implementations of some functions of an Erlang
module. NIFs are called like any other Erlang functions with no difference to
the caller. The library is a dynamically linked `.so`/`dll` loaded at runtime by
`erlang:load_nif/2`. Once loaded, a NIF library is persistent and is not
unloaded until the module instance it belongs to is purged.

The `-nifs([f/A])` module attribute declares which functions are replaced by NIFs.
Each NIF must have an Erlang stub implementation invoked if the NIF library is
not yet loaded (typical stub: `erlang:nif_error(...)`). A NIF need not be
exported (can be local), but unused local stubs are optimized away by the
compiler and will cause NIF loading to fail.

WARNING (verbatim spirit): use NIFs with extreme care. A native function is
executed as a direct extension of the VM's native code — no preemptive
scheduling, no memory protection. A misbehaving NIF makes the whole VM
misbehave; a crashing NIF crashes the whole VM.

## ERL_NIF_INIT + lifecycle callbacks (load/reload/upgrade/unload)

```c
ERL_NIF_INIT(MODULE, ErlNifFunc funcs[], load, NULL, upgrade, unload)
```
- Magic macro initializing a NIF library; evaluated at global file scope.
- `MODULE`: Erlang module name as a C identifier (stringified by the macro).
- `funcs`: static array of `ErlNifFunc` descriptors.
- `load`, `upgrade`, `unload`: function pointers (may be NULL where noted).
- The 4th argument MUST be `NULL`. It was the deprecated `reload` callback,
  **no longer supported since OTP 20**.

`ErlNifFunc`:
```c
typedef struct {
    const char* name;
    unsigned arity;
    ERL_NIF_TERM (*fptr)(ErlNifEnv* env, int argc, const ERL_NIF_TERM argv[]);
    unsigned flags;   // 0 for regular NIF; dirty flags for dirty NIFs
} ErlNifFunc;
```

Lifecycle callbacks (all receive a *callback environment* `caller_env`, a
temporary pseudo-process that "terminates" when the callback returns — terms
created there are only valid during the callback):

- `int (*load)(ErlNifEnv* caller_env, void** priv_data, ERL_NIF_TERM load_info)`
  — called when the library is loaded and no prior library exists for the
  module. `*priv_data` initialized to NULL; set it to keep state between NIF
  calls (retrieve later via `enif_priv_data`). `load_info` is the 2nd arg to
  `erlang:load_nif/2`. Library fails to load if `load` returns non-zero. May be
  NULL. **`enif_open_resource_type` may ONLY be called in `load`/`upgrade`.**

- `int (*upgrade)(ErlNifEnv* caller_env, void** priv_data, void** old_priv_data, ERL_NIF_TERM load_info)`
  — called when the library is loaded and old code with a loaded NIF library
  exists. Like `load` but `*old_priv_data` holds the previous `load`/`upgrade`
  value. `*priv_data` initialized to NULL. May write to both. Library fails to
  load if `upgrade` returns non-zero OR if `upgrade` is NULL (so upgrade path
  requires a real function).

- `void (*unload)(ErlNifEnv* caller_env, void* priv_data)` — called when the
  module instance the library belongs to is purged as old. New code of the same
  module may or may not exist.

- `reload` — DEPRECATED, removed in OTP 20. Pass NULL as the 4th `ERL_NIF_INIT`
  argument.

`load` and `upgrade` are thread-safe even for shared state data; ordinary NIFs
are thread-safe only when acting as pure functions reading their arguments.

## Dirty schedulers (CPU-bound vs IO-bound; when mandatory)

A NIF that cannot be split and cannot execute in ~1 ms is a "dirty NIF". Dirty
NIFs run on a separate set of **dirty schedulers** and are not subject to the
normal-NIF duration restriction. Two job classes:

- `ERL_NIF_DIRTY_JOB_CPU_BOUND` — CPU-bound work.
- `ERL_NIF_DIRTY_JOB_IO_BOUND` — I/O-bound work (expected to block waiting for
  I/O and/or spend limited time moving data).

Correct classification is important: misclassifying CPU-bound as I/O-bound can
starve ordinary schedulers (dirty I/O schedulers might starve ordinary
schedulers). A job alternating between classes can be reclassified and
rescheduled via `enif_schedule_nif`.

Two ways to declare/schedule a dirty NIF:
1. Set the `flags` field in the NIF's `ErlNifFunc` entry to
   `ERL_NIF_DIRTY_JOB_CPU_BOUND` or `ERL_NIF_DIRTY_JOB_IO_BOUND`.
2. Call `enif_schedule_nif(...)` from inside a NIF, passing a function pointer
   and the appropriate `flags` value.

`enif_schedule_nif`:
```c
ERL_NIF_TERM enif_schedule_nif(
    ErlNifEnv* caller_env,   // must be process-bound env of the calling NIF
    const char* fun_name,    // name; badarg exception if not convertible to atom
    int flags,               // 0 | ERL_NIF_DIRTY_JOB_CPU_BOUND | ERL_NIF_DIRTY_JOB_IO_BOUND
    ERL_NIF_TERM (*fp)(ErlNifEnv*, int, const ERL_NIF_TERM[]),
    int argc, const ERL_NIF_TERM argv[]);
```
- Schedules `fp` for future execution; the calling NIF does NOT block waiting
  for it. The calling NIF must use `enif_schedule_nif`'s return value as its own
  return value. (Available since OTP 17.3.)

When mandatory: any NIF that may run longer than ~1 ms (e.g. calls third-party
blocking libraries, unbounded computation) MUST be dirty, OR split into chunks
(yielding NIF via `enif_schedule_nif` of regular NIFs), OR offloaded to a
managed thread (Threaded NIF). Yielding/splitting is always preferred over dirty
NIFs for both performance and system characteristics.

Caveats while a process executes a dirty NIF:
- Suspend / GC of that process cannot happen until the dirty NIF returns;
  other processes waiting on such ops may wait a long time.
- Blocking multi-scheduling (`erlang:system_flag(multi_scheduling, block)`)
  waits for all dirty ops on all dirty schedulers to complete.
- Process termination completes only up to a point: registered name, ETS
  tables, links/monitors are released, but NIF execution is NOT stopped. The NIF
  can check liveness via `enif_is_current_process_alive`. Deallocation of
  process heap / PCB is delayed until the dirty NIF completes.

`enif_consume_timeslice(env, percent)` (OTP R16B): cooperative-scheduling hint;
lengthy NIFs should call it frequently and return when it returns 1 (timeslice
exhausted). Percent 1..100; env must be the calling process env.

## Resource objects (open/alloc/make/release/keep/destroy/monitor)

Resource objects are the SAFE way to return pointers to native data structures
from a NIF. A resource is a memory block allocated with `enif_alloc_resource`; a
"safe pointer" handle term is returned to Erlang via `enif_make_resource`. The
term is opaque — storable/passable between processes, but its only real use is
to be passed back as a NIF argument, where `enif_get_resource` recovers a
guaranteed-valid pointer. A resource is deallocated only when the last handle
term is GC'd AND the resource is released with `enif_release_resource` (order
not guaranteed).

All resources are instances of a **resource type** (created at load time),
uniquely identified by a name string + the implementing module. A type may have
a user-supplied **destructor** called automatically when objects of that type are
released (by GC or `enif_release_resource`).

Key functions:

- `ErlNifResourceType* enif_open_resource_type(env, module_str/*NULL*/, name, dtor, flags, *tried)`
  (OTP R13B04) — create or take over a resource type. `flags`:
  `ERL_NIF_RT_CREATE` (create new) and/or `ERL_NIF_RT_TAKEOVER` (take over
  existing type + all its instances; supplied `dtor` called for both existing
  and new instances). `*tried` set to what was done. **Only callable in
  `load`/`upgrade`; the type is created/taken over only if load/upgrade returns
  successfully.** `dtor` may be NULL.

- `enif_open_resource_type_x(env, name, ErlNifResourceTypeInit* init, flags, *tried)`
  (OTP 20.0) — like above but accepts `dtor`, `stop` (select stop), `down`
  (monitor down) callbacks (for use with `enif_select` / `enif_monitor_process`).

- `enif_init_resource_type(env, name, init, flags, *tried)` (OTP 24.0) —
  additionally accepts the `dyncall` callback (for
  `enif_dynamic_resource_call`). `init->members` must be set to the number of
  initialized callbacks counted from the top of the struct (4 = all).

- `void* enif_alloc_resource(ErlNifResourceType* type, unsigned size)` (OTP
  R13B04) — allocate a memory-managed resource object.

- `ERL_NIF_TERM enif_make_resource(ErlNifEnv* env, void* obj)` (OTP R13B04) —
  create an opaque handle term. No ownership transfer; the resource still needs
  `enif_release_resource` (which may be called immediately after, leaving only
  the GC-owned term reference). Since ERTS 9.0 (OTP 20.0) resource terms have
  defined comparison/serialization behavior (compare equal iff same underlying
  pointer; serializable via `term_to_binary`, recreated if still alive, stale
  otherwise; `enif_get_resource` returns false for stale terms).

- `int enif_get_resource(env, term, type, void** objp)` (OTP R13B04) — recover
  pointer from a handle term. Does NOT add a reference; pointer guaranteed valid
  at least as long as the handle term is valid.

- `void enif_release_resource(void* obj)` (OTP R13B04) — remove a reference;
  object destructed when last reference removed. Each `release` must correspond
  to a prior `alloc` or `keep`. References created by `enif_make_resource` can
  ONLY be removed by the GC. No guarantee when the destructor of an
  unreferenced resource runs (may be direct or scheduled later, possibly another
  thread).

- `int enif_keep_resource(void* obj)` (OTP R14B) — add a reference; each `keep`
  must be balanced by a `release` before destruction.

- `int enif_monitor_process(caller_env, void* obj, const ErlNifPid* target_pid, ErlNifMonitor* mon)`
  (OTP 20.0) — monitor a process from a resource; on process exit the resource
  type's `down` callback is invoked. `caller_env` is the calling thread env or
  NULL for custom threads. Returns 0 on success, <0 if no `down` callback, >0 if
  process already dead/undefined. Thread-safe. Monitor auto-removed when it
  triggers or resource is deallocated. `ErlNifMonitor` storage is provided by the
  caller (not stored by RTS); compare via `enif_compare_monitors`.

- `enif_make_resource_binary(env, obj, data, size)` (OTP R14B) — create a
  binary term memory-managed by a resource; destructor responsible for releasing
  `data`. Several binaries can share one resource; destructor fires when last
  binary is GC'd.

Destructor prototype: `void ErlNifResourceDtor(ErlNifEnv* caller_env, void* obj)`
— only allowed use of `obj` in the destructor is to access its user data one
final time; guaranteed to be the last callback before deallocation.

Upgrade semantics: resource types support runtime upgrade — a loaded NIF library
can take over an existing resource type and "inherit" all existing objects; the
new library's destructor thereafter runs for inherited objects and the old
library can be safely unloaded. Unloading is postponed as long as resource
objects with a destructor exist in the library. Existing resource objects of an
upgraded module must either be deleted or taken over by the new NIF library.

## Environments & term construction (summary)

`ERL_NIF_TERM` — opaque C type referring to any Erlang term; only usable as API
args/return values. Every term belongs to an `ErlNifEnv`; a term is valid until
its environment is destroyed (cannot be destructed individually).

`ErlNifEnv` — opaque; three kinds:
1. **Process-bound env** — passed as 1st arg to every NIF; holds function args
   and must host the return value. Only valid in the thread where supplied,
   until the NIF returns. Storing pointers to it between NIF calls is useless
   and dangerous.
2. **Callback env** — passed to non-NIF callbacks (`load`, `upgrade`, `unload`,
   `dtor`, `down`, `stop`, `dyncall`); behaves like process-bound but with a
   temporary pseudo-process that "terminates" when the callback returns.
3. **Process-independent env** — created by `enif_alloc_env` (OTP R14B); can
   store terms between NIF calls and send terms via `enif_send`; valid until
   `enif_free_env` or `enif_send`. Terms can be copied between envs with
   `enif_make_copy`.

Atoms created during `load`/`upgrade` are special: they can be referred to as a
term in ANY `ErlNifEnv`. Best practice: create all atoms during loading and
store them in static/global variables.

Term construction/inspection (summary, not exhaustive):
- `enif_make_*` family: `enif_make_atom`, `enif_make_atom_len`, `enif_make_int`,
  `enif_make_uint`, `enif_make_int64`, `enif_make_uint64`, `enif_make_long`,
  `enif_make_ulong`, `enif_make_double`, `enif_make_tuple`/`tuple1..9`,
  `enif_make_list`/`list1..9`/`list_from_array`/`list_cell`,
  `enif_make_new_map`, `enif_make_map_put/update/remove`, `enif_make_map_from_arrays`,
  `enif_make_string`/`string_len`, `enif_make_sub_binary`, `enif_make_ref`,
  `enif_make_pid`, `enif_make_resource`, `enif_make_resource_binary`,
  `enif_make_unique_integer`, `enif_make_badarg`, `enif_raise_exception`.
- `enif_get_*` family: `enif_get_int/uint/int64/uint64/long/ulong`,
  `enif_get_double`, `enif_get_atom`/`atom_length`, `enif_get_string`/`string_length`,
  `enif_get_tuple`, `enif_get_list_cell`/`list_length`, `enif_get_map_size/value`,
  `enif_get_resource`, `enif_get_local_pid`/`local_port`.
- `enif_is_*` queries: `enif_is_atom/binary/list/tuple/map/pid/port/ref/fun/number`,
  `enif_is_identical`, `enif_compare`, `enif_is_exception`, `enif_is_process_alive`,
  `enif_is_current_process_alive`, `enif_is_port_alive`, `enif_is_pid_undefined`.

Binaries:
- `ErlNifBinary { size; unsigned char* data; }` — semi-opaque; only `size`/`data`
  readable. Instances allocated by the user (usually as locals). Raw `data` is
  mutable only after `enif_alloc_binary`/`enif_realloc_binary`; otherwise
  read-only. A mutable binary must eventually be `enif_release_binary`'d or made
  read-only via `enif_make_binary` (need not be in the same NIF call). Read-only
  binaries need not be released.
- `enif_make_new_binary(env, size, ERL_NIF_TERM* termp)` (OTP R14B) — shortcut:
  allocate + create owning term; data mutable until the NIF returns; cannot be
  kept across NIF calls or reallocated. Allocates small binaries on the process
  heap when possible.
- `enif_inspect_binary(env, bin_term, ErlNifBinary* bin)` — fill `bin` from a
  binary term; data is transient, no release needed unless later reallocated.
- `enif_inspect_iolist_as_binary(env, term, ErlNifBinary* bin)` (OTP R13B04) —
  continuous buffer with same byte content as an iolist.
- Bitstrings with arbitrary bit length are NOT supported.

## Cardinal rules (latency; crash=VM crash; no preemption; lifetime)

1. **Latency / no preemption**: a NIF runs as a direct extension of the VM's
   native code. The VM cannot preempt a running NIF. A native function doing
   lengthy work before returning degrades VM responsiveness and can cause
   extreme memory usage, bad scheduler load balancing, and other strange
   behaviors (which may vary between OTP releases). A well-behaving NIF should
   return within ~1 millisecond. Handle longer work via: (a) splitting into
   chunks (yielding NIF / `enif_schedule_nif` of regular NIFs — preferred), (b)
   dirty NIFs, or (c) Threaded NIFs. Use `enif_consume_timeslice` to cooperate.
2. **Crash = VM crash**: a native function that crashes will crash the whole
   VM. An erroneously implemented NIF can cause VM internal state inconsistency
   leading to a crash or miscellaneous misbehaviors at any point after the call.
3. **No safe environment**: the VM cannot provide the same services as for
   Erlang code — no preemptive scheduling, no memory protection.
4. **Resource lifetime**: a resource object is deallocated only when the last
   handle term is GC'd AND the resource is released with `enif_release_resource`
   (order not guaranteed; destructor may run synchronously or be scheduled later,
   possibly on another thread). References from `enif_make_resource` can only be
   removed by the GC; pair each `enif_keep_resource` with a `enif_release_resource`.
5. **Environment lifetime**: process-bound envs are valid only in the calling
   thread until the NIF returns — never store pointers to them across calls.
   Atoms created in `load`/`upgrade` are the exception (valid in any env).
6. **Concurrency**: a NIF is thread-safe without explicit synchronization only
   when it acts as a pure function reading its arguments. Writes to shared state
   (static vars, `enif_priv_data`, process-independent env terms, mutable
   resources) require your own synchronization.

## Versioning / static NIFs

Version management: NIF API version info is compiled into the library and
verified at load. `erl_nif.h` defines:
- `ERL_NIF_MAJOR_VERSION` — incremented on incompatible runtime changes.
  Normally a recompile suffices when it changes; rarely the NIF must be
  modified (documented if so).
- `ERL_NIF_MINOR_VERSION` — incremented when new features are added; the runtime
  uses it to decide which features to use.

The runtime refuses to load a library if major versions differ, OR if majors are
equal and the library's minor version is greater than the runtime's. Old
libraries with lower major versions are allowed for a transition period of two
major releases after a major bump, but may fail if deprecated features are used.

Module upgrade & static data: a loaded NIF library is tied to the Erlang module
instance that loaded it. On module upgrade the new instance must load its own
library (or reuse the same one — sharing the dynamic library means static data
is shared too). To avoid unintentionally shared static data, each module
version can keep private data via `*priv_data` (set in `load`/`upgrade`,
retrieved by `enif_priv_data`).

Static NIFs (`--enable-static-nifs`): for static inclusion, define
`STATIC_ERLANG_NIF_LIBNAME` (name of the archive without `.a`, only C-identifier
chars) before including `erl_nif.h`. The older macro `STATIC_ERLANG_NIF` requires
the archive name to match the module name. Multiple static NIF libraries can be
included in one archive by specifying `STATIC_ERLANG_NIF_LIBNAME` values
colon-separated after the archive name in configure:
`./configure --enable-static-nifs=/path/to/archive.a:nif_lib1:nif_lib2`

## Verbatim quotes (the key 4-8)

> "A native function is executed as a direct extension of the native code of the
> VM. Execution is not made in a safe environment. The VM cannot provide the
> same services as provided when executing Erlang code, such as pre-emptive
> scheduling or memory protection. If the native function does not behave well,
> the whole VM will misbehave."

> "A native function that crashes will crash the whole VM."

> "A native function doing lengthy work before returning degrades responsiveness
> of the VM, and can cause miscellaneous strange behaviors. Such strange
> behaviors include, but are not limited to, extreme memory usage, and bad load
> balancing between schedulers."

> "A NIF that cannot be split and cannot execute in a millisecond or less is
> called a 'dirty NIF', as it performs work that the ordinary schedulers of the
> Erlang runtime system cannot handle cleanly."

> "It is important to classify the dirty job correct. An I/O bound job should be
> classified as such, and a CPU bound job should be classified as such. If you
> should classify CPU bound jobs as I/O bound jobs, dirty I/O schedulers might
> starve ordinary schedulers."

> "A resource object is not deallocated until the last handle term is garbage
> collected by the VM and the resource is released with enif_release_resource
> (not necessarily in that order)."

> "enif_open_resource_type is only allowed to be called in the two callbacks
> load and upgrade. The resource type is only created or taken over if the
> calling load/upgrade function returns successfully."

> "The fourth argument NULL is ignored. It was earlier used for the deprecated
> reload callback which is no longer supported since OTP 20."

## Version notes (OTP-version callouts for dirty/resource features)

- Dirty NIFs / dirty schedulers: present since OTP 17.0 (ERTS 7.0); the
  `ERL_NIF_DIRTY_JOB_CPU_BOUND` / `ERL_NIF_DIRTY_JOB_IO_BOUND` flags and
  `enif_schedule_nif` (OTP 17.3) are the modern API.
- `enif_consume_timeslice`: OTP R16B.
- `enif_alloc_env` / `enif_clear_env` / `enif_make_copy` / `enif_keep_resource`:
  OTP R14B.
- `enif_make_resource` / `enif_make_resource_binary`: OTP R14B (resource terms
  gain defined comparison/serialization since ERTS 9.0 / OTP 20.0).
- `enif_open_resource_type_x` (dtor+stop+down): OTP 20.0.
- `enif_monitor_process` / `enif_demonitor_process` / `enif_compare_monitors`:
  OTP 20.0.
- `enif_init_resource_type` (adds `dyncall`): OTP 24.0.
- `enif_make_new_map` / map iterators: OTP 18.0.
- `enif_inspect_iovec` / I/O queue API: OTP 20.1.
- `enif_get_string_length` / `enif_make_new_atom` / `_len`: OTP 26.0.
- `reload` callback removed: OTP 20 (pass NULL as 4th `ERL_NIF_INIT` arg).
- `enif_make_badarg` return-value requirement lifted: ERTS 7.0 (OTP 18).

## Discovered links

### Relevant (crawl later)
- https://www.erlang.org/doc/system/nif.html — NIF User's Guide / introduction
  (tutorial, lifecycle, dirty NIF guidance, resource usage narrative).
- https://www.erlang.org/doc/system/modules.html#nifs_attribute — the `-nifs()`
  module attribute semantics.
- https://www.erlang.org/doc/apps/erts/erl_driver.html — erl_driver API (shared
  thread/mutex/rwlock primitives referenced by `enif_*` equivalents).
- https://www.erlang.org/doc/man/erlang.html#load_nif/2 — `erlang:load_nif/2`.
- https://www.erlang.org/doc/man/erlang.html#nif_error/1 — `erlang:nif_error/1`
  (standard stub for unloaded NIFs).
- https://www.erlang.org/doc/apps/erts/erts_alloc.html — erts_alloc (referenced
  for memory allocation context).

### Skipped
- All `#enif_*` and `#ErlNif*` in-page anchors (same page, already extracted).
- `erl_nif.md` (markdown source mirror of this same page).
- GitHub source link (OTP-29.0.2 raw markdown).
