# Crawl: kernel/dets.html
- seed_url: https://www.erlang.org/doc/apps/kernel/dets.html
- canonical_url: https://www.erlang.org/doc/apps/stdlib/dets.html
- family: Erlang/OTP stdlib module docs (disk-based term storage)
- fetch: 200 (seed URL `/apps/kernel/dets.html` returned 404; canonical is under stdlib; reached via `/doc/man/dets.html` redirect)
- otp_version: OTP 29.0.2 (stdlib v8.0.1)
- feeds_docs: ets-data.md

## Purpose
`dets` is a disk-based term storage. It stores Erlang terms ("objects" = tuples with one element designated as key) in a file. A Dets table is a collection of such objects on disk. It is the disk-only counterpart to `ets`, used internally by Mnesia. Mnesia adds transactions, queries, and distribution on top. Intended for users who need efficient storage of Erlang terms on disk only.

## Key functions (exact arities)
- `open_file/1` — `open_file(Filename) -> {ok, Reference} | {error, Reason}`. Opens an existing table; repairs if not properly closed. Returns a `reference()` (not the table name) — most useful for debugging.
- `open_file/2` — `open_file(Name, Args) -> {ok, Name} | {error, Reason}`. Opens/creates a table; `Name` is the atom used in all subsequent ops. Empty table created if no file exists. Multiple processes may share one table by the same name.
- `close/1` — `close(Name) -> ok | {error, Reason}`. Only opener processes may close. All tables must be closed before system stop. Reopening an improperly-closed table triggers automatic repair.
- `insert/2` — `insert(Name, Objects) -> ok | {error, Reason}`. `Objects :: object() | [object()]`. For `set`, existing object with matching key is replaced.
- `insert_new/2` — `insert_new(Name, Objects) -> boolean() | {error, Reason}`. Inserts only if no object with matching key exists; returns `false` otherwise (no update).
- `lookup/2` — `lookup(Name, Key) -> Objects | {error, Reason}`, `Objects :: [object()]`. Returns all objects with key `Key`. For `set`: empty or one-element list. For `bag`/`duplicate_bag`: arbitrary length. Order unspecified (not insertion order).
- `delete/2` — `delete(Name, Key) -> ok | {error, Reason}`. Deletes all objects with key `Key`.
- `delete_object/2` — `delete_object(Name, Object) -> ok | {error, Reason}`. Deletes all instances of a specific object (useful for `bag`/`duplicate_bag` to remove only some objects of a key).
- `delete_all_objects/1` — `delete_all_objects(Name) -> ok | {error, Reason}`. Almost-constant time; if table is fixed, equivalent to `match_delete(T, '_')`.
- `match/1` — `match(Continuation) -> {[Match], Continuation2} | '$end_of_table' | {error, Reason}`. Continuation from `match/1` or `match/3`.
- `match/2` — `match(Name, Pattern) -> [Match] | {error, Reason}`. Returns bindings matching Pattern (see `ets:match/2`). Bound keypos → only matching-key objects; unbound → all.
- `match/3` — `match(Name, Pattern, N) -> {[Match], Continuation} | '$end_of_table' | {error, Reason}`. `N :: default | non_neg_integer()`. Table MUST be `safe_fixtable/2`-protected before calling `match/3`, else `match/1` errors can occur.
- `match_object/1,2,3` — like `match` but returns whole objects instead of bindings.
- `match_delete/2` — `match_delete(Name, Pattern) -> ok | {error, Reason}`.
- `select/1` — `select(Continuation) -> {Selection, Continuation2} | '$end_of_table' | {error, Reason}`.
- `select/2` — `select(Name, MatchSpec) -> Selection | {error, Reason}`. Applies match spec to all/some objects. More efficient than `first/1`+`next/2` or `slot/2` for full traversal.
- `select/3` — `select(Name, MatchSpec, N) -> {Selection, Continuation} | '$end_of_table' | {error, Reason}`. `N :: default | non_neg_integer()`. MUST `safe_fixtable/2` before `select/3`.
- `select_delete/2` — `select_delete(Name, MatchSpec) -> ...`.
- `foldl/3` — `foldl(Function, Acc0, Name) -> Acc | {error, Reason}`. Equivalent to `foldr/3`. Traverses in unspecified order. `Function :: fun((Object, AccIn) -> AccOut)`.
- `foldr/3` — `foldr(Function, Acc0, Name) -> Acc | {error, Reason}`. Same semantics as `foldl/3`.
- `first/1` — `first(Name) -> Key | '$end_of_table'`. (Exits process with error tuple on failure — exception to the `{error, Reason}` rule.)
- `next/2` — `next(Name, Key1) -> Key | '$end_of_table'`. (Also exits process on failure.)
- `info/1` — `info(Name) -> InfoList | undefined`. Tuples: `{file_size}`, `{filename}`, `{keypos}`, `{size}`, `{type}`.
- `info/2` — `info(Name, Item) -> Value | undefined`. Items: `access | auto_save | bchunk_format | hash | file_size | filename | keypos | memory | no_keys | no_objects | no_slots | owner | ram_file | safe_fixed | safe_fixed_monotonic_time | size | type`.
- `sync/1` — `sync(Name) -> ok | {error, Reason}`. Flushes all updates (and the in-RAM buddy system) to disk. Also flushes `ram_file` tables. Can take time if fragmented.
- `slot/2` — `slot(Name, I) -> '$end_of_table' | Objects | {error, Reason}`. Objects distributed among slots 0..n.
- `safe_fixtable/2` — `safe_fixtable(Name, Fix)`. Required before `match/3`/`select/3` chunked traversal.
- `bchunk/2` — bulk chunked traversal (binary format).
- `from_ets/2`, `to_ets/2` — bulk copy between ETS and DETS tables.
- `init_table/2,3` — replace table contents via an InitFun.
- `is_dets_file/1`, `is_compatible_bchunk_format/2`, `repair_continuation/2`, `pid2name/1`, `table/1,2`, `traverse/2`, `update_counter/3`, `member/2`.

Note: there is NO `truncate/N` function in `dets`. The closest equivalents are `delete_all_objects/1` (clears objects, keeps file) and `sync/1` (flush). To fully reset/truncate, close + delete the file + reopen, or use `init_table/2,3`.

## open_file table types + options (note: NO ordered_set)
Table types (`{type, type()}`, default `set`):
- `set` — at most one object per key; inserting an object with an existing key overwrites it.
- `bag` — zero or more *different* objects per key.
- `duplicate_bag` — zero or more *possibly matching* (identical) objects per key.
- **`ordered_set` is NOT supported by DETS.** Only ETS provides `ordered_set`. No Erlang/OTP library currently supports ordered disk-based term storage.

`open_file/2` `Args` options (`[OpenArg]`):
- `{access, access()}` — `read_write` (default) or `read_only`. Read-only tables are NOT subjected to the automatic repair algorithm if later opened after a crash.
- `{auto_save, auto_save()}` — integer `Time` ms (flush to disk after `Time` ms idle; flushed tables need no repair after uncontrolled halt) or `infinity` (disabled). Default `180000` (3 min).
- `{estimated_no_objects, non_neg_integer()}` — equivalent to `min_no_slots`.
- `{file, file:name()}` — file to open; defaults to the table name.
- `{max_no_slots, no_slots()}` — max slots; default 32 M (the maximal value). Higher → more fragmentation; lower → less fragmentation but slower.
- `{min_no_slots, no_slots()}` — estimated number of distinct keys at creation; enhances performance. Default 256 (minimum value).
- `{keypos, keypos()}` — key element position; default 1. Convenient for storing records (first element = record name).
- `{ram_file, boolean()}` — keep table in RAM; on close, contents written to disk file. Default `false`. Useful for open→insert-set→close workloads.
- `{repair, boolean() | force}` — invoke automatic file repair. Default `true`. `false` → no repair attempt, returns `{error, {needs_repair, FileName}}` if repair needed. `force` → repair even if properly closed (seldom needed). Ignored if table already open.

## DETS vs ETS differences (disk-backed, durability, size limits, repair)
- **Disk-backed**: DETS stores terms on a file; ETS is in-memory. Every DETS lookup involves a series of disk seek+read operations, so DETS is *much slower* than the corresponding ETS functions despite the similar API.
- **File-per-table**: each DETS table is one file (`{file, ...}` option, defaults to table name). ETS tables live in RAM with no file.
- **No `ordered_set`**: DETS supports only `set`, `bag`, `duplicate_bag`. ETS additionally supports `ordered_set`.
- **No safe concurrent `first`/`next` on fixed tables**: the limited ETS support that makes `first`+`next` safe on fixed ETS tables is NOT provided by DETS. Safe concurrency requires Mnesia (or user-implemented locking).
- **Size limit**: DETS files cannot exceed **2 GB**. Larger tables require Mnesia table fragmentation. ETS is bounded only by memory.
- **Repair on open**: if a table is not properly closed, DETS automatically repairs it on next open — this can take substantial time for large tables. Read-only opens skip repair. `repair: force` repairs even properly-closed tables. ETS has no such repair concept (RAM-only).
- **Durability / failure model**: 
  - Tables must be opened before use and properly closed when finished.
  - A table is closed when the opening process terminates; multiple openers share the table and it closes only when the last user closes/terminates.
  - Tables are NOT properly closed if the Erlang runtime system terminates abnormally (e.g. `^C` break-handler on Unix) → repair required on reopen.
  - `auto_save` periodically flushes the table so a flushed table needs no repair after an uncontrolled emulator halt.
  - `sync/1` forces a flush of all updates plus the in-RAM buddy-system space-management structures.
  - `ram_file` tables hold data in RAM and only persist on close/sync.
- **Internal structure**: linear hash list that grows gracefully as data is inserted; space managed by a buddy system. The *entire* buddy system is kept in RAM, so a heavily fragmented table can consume significant memory. Defragmentation requires close + reopen with `repair: force`.
- **Error convention**: all functions return `{error, Reason}` on error; `first/1` and `next/2` are exceptions that *exit the process* with the error tuple. Badly-formed arguments cause all functions to exit the process with `badarg`.

## Strict rules
- A DETS table MUST be opened before any read/update and MUST be properly closed when finished.
- All open tables MUST be closed before the system is stopped.
- Only processes that opened a table may close it.
- Before calling `match/3` or `select/3`, the table MUST be protected with `safe_fixtable/2`, otherwise `match/1`/`select/1` (continuation calls) can error.
- DETS file size MUST NOT exceed 2 GB (use Mnesia fragmentation for larger).
- Do not assume `ordered_set` semantics — DETS does not provide it.
- Do not rely on `first`/`next` for safe concurrent traversal — use Mnesia or external locking.
- Traversal order is unspecified; do not rely on insertion order in `lookup`/`match`/`select`/`foldl`/`foldr` results.
- `first/1` and `next/2` exit the process (not return `{error, _}`) on failure — handle accordingly.
- Read-only opens (`access: read_only`) bypass repair; a later crash-recovery open of such a table will not be auto-repaired.

## Verbatim quotes
- "A disk-based term storage."
- "This module is used by the Mnesia application, and is provided \"as is\" for users who are interested in efficient storage of Erlang terms on disk only."
- "The size of Dets files cannot exceed 2 GB. If larger tables are needed, table fragmentation in Mnesia can be used."
- "Dets tables must be opened before they can be updated or read, and when finished they must be properly closed. If a table is not properly closed, Dets automatically repairs the table. This can take a substantial time if the table is large."
- "A Dets table is closed when the process which opened the table terminates. If many Erlang processes (users) open the same Dets table, they share the table. The table is properly closed when all users have either terminated or closed the table. Dets tables are not properly closed if the Erlang runtime system terminates abnormally."
- "As all operations performed by Dets are disk operations, it is important to realize that a single look-up operation involves a series of disk seek and read operations. The Dets functions are therefore much slower than the corresponding ets functions, although Dets exports a similar interface."
- "Dets organizes data as a linear hash list and the hash list grows gracefully as more data is inserted into the table. Space management on the file is performed by what is called a buddy system. The current implementation keeps the entire buddy system in RAM, which implies that if the table gets heavily fragmented, quite some memory can be used up. The only way to defragment a table is to close it and then open it again with option repair set to force."
- "Notice that type ordered_set in Ets is not yet provided by Dets, neither is the limited support for concurrent updates that makes a sequence of first and next calls safe to use on fixed ETS tables. ... Currently, no Erlang/OTP library has support for ordered disk-based term storage."
- "All Dets functions return {error, Reason} if an error occurs (first/1 and next/2 are exceptions, they exit the process with the error tuple). If badly formed arguments are specified, all functions exit the process with a badarg message."
- "An empty Dets table is created if no file exists." (open_file/2)
- "The table is always to be protected using safe_fixtable/2 before calling match/3, otherwise errors can occur when calling match/1." (and analogously for select/3)
- "Ensures that all updates made to table Name are written to disk." (sync/1)

## Version notes
- Documented against OTP 29.0.2, stdlib v8.0.1.
- Source: `lib/stdlib/src/dets.erl` (OTP-29.0.2 tag).
- `ordered_set` and safe-concurrent `first`/`next` are explicitly noted as "may be provided by Dets in a future release of Erlang/OTP" — i.e. still absent as of OTP 29.
- `safe_fixed_monotonic_time` is the time-warp-safe variant of `safe_fixed` (which uses `erlang:timestamp/0` and is not time-warp safe).

## Discovered links
### Relevant (crawl later)
- https://www.erlang.org/doc/apps/stdlib/ets.html — ETS module (direct counterpart; `ets:match/2`, `ets:safe_fixtable/2`, match specs, `table/0` type). Core to ets-data.md.
- https://www.erlang.org/doc/apps/mnesia/mnesia.html — Mnesia (builds on DETS; transactions, queries, distribution, table fragmentation for >2 GB tables, ordered/concurrent storage).
- https://www.erlang.org/doc/apps/erts/time_correction.html#time-warp-modes — time warp modes (relevant to `safe_fixed` vs `safe_fixed_monotonic_time`).
- https://www.erlang.org/doc/apps/erts/time_correction.html#time-warp-safe-code — time-warp-safe code.
- https://www.erlang.org/doc/apps/kernel/file.html — `file:name/0` type used by DETS file option.

### Skipped
- GitHub source links to `lib/stdlib/src/dets.erl` (line anchors) — source code, not docs.
- Internal fragment anchors (`#open_file/1`, `#from_ets/2`, `#is_dets_file/1`, `#repair_continuation/2`, `#to_ets/2`, `#functions`).
- `dets.md` (markdown source mirror of same page).
- CSS/asset links.
