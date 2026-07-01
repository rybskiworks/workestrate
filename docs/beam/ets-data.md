# ETS Data

## Purpose

ETS (Erlang Term Storage) is the interface to Erlang's built-in term storage
BIFs: dynamic tables of tuples with constant-time access (logarithmic for
`ordered_set`). This doc covers table creation (types, protection, options),
CRUD and traversal, match/select with match specifications, ownership/heir/
`give_away`, `ordered_set` quirks, and `safe_fixtable`.

## Sources used

- Crawl `24-ets.md` — stdlib `ets.html` — https://www.erlang.org/doc/apps/stdlib/ets.html
- `.crawl/41-dets.md` — stdlib `dets.html` — https://www.erlang.org/doc/apps/stdlib/dets.html (NOTE: DETS is in STDLIB, not kernel; the seed URL `/apps/kernel/dets.html` returns 404, canonical is under stdlib)
- `.crawl/42-mnesia-chapter.md` — mnesia overview — https://www.erlang.org/doc/apps/mnesia/mnesia_overview.html (NOTE: the seed URL `mnesia_chapter.html` returns 404; the live canonical overview is `mnesia_overview.html`)

## Core guidance

### new/2 — table types, protection, options

`new(Name, Options) -> table()` (`Name :: atom()`).
```
Option :: Type | Access | named_table |
           {keypos, Pos} |
           {heir, Pid} | {heir, Pid, HeirData} | {heir, none} |
           Tweaks
Type  :: set | ordered_set | bag | duplicate_bag
Access:: public | protected | private
Tweaks:: {write_concurrency, boolean() | auto} |
         {read_concurrency, boolean()} |
         {decentralized_counters, boolean()} |
         compressed
```
Default `[]` equals: `[set, protected, {keypos,1}, {heir,none},
{write_concurrency,false}, {read_concurrency,false},
{decentralized_counters,false}]`.

Table types:
- `set` (default): one key, one object, no order.
- `ordered_set`: one key, one object, ordered in Erlang term order. Keys regarded
  equal when they *compare equal* (`==`), not only when they *match* (`=:=`) — so
  `integer 1` and `float 1.0` are the same key.
- `bag`: many objects per key, only one instance of each object.
- `duplicate_bag`: many objects per key, including duplicate copies.

Protection:
- `public`: any process can read or write.
- `protected` (default): owner read/write; others read-only.
- `private`: only owner can read or write.

Options:
- `named_table`: registered under `Name`; use `whereis/1` for the tid.
- `{keypos,Pos}`: key element (default 1); stored tuples must have at least `Pos`
  elements.
- `{heir,Pid,HeirData} | {heir,Pid} | {heir,none}`: heir inherits the table if
  the owner terminates. With `HeirData`, sends `{'ETS-TRANSFER',tid(),FromPid,
  HeirData}`. With `{heir,Pid}` (no data) NO transfer message is sent. Heir must
  be local. Default `none` destroys the table on owner termination.
- `{write_concurrency, boolean() | auto}` (default `false`): optimize concurrent
  writes. `auto` (OTP 25.0+) adjusts at runtime. Does NOT change
  atomicity/isolation. No effect on `ordered_set` prior to stdlib-3.7 (OTP 22.0).
- `{read_concurrency, boolean()}` (default `false`): optimize concurrent reads.
- `{decentralized_counters, boolean()}` (OTP 23.0): optimizes concurrent
  size/memory mutations; makes `info/1,2` `size`/`memory` slower.
- `compressed` (OTP R14B01): more compact storage, slower operations (especially
  `match`/`select`); the key element is NOT compressed.

### CRUD and traversal

- `insert(Table, ObjectOrObjects)` — `set`: matching key replaces; `ordered_set`:
  compare-equal key replaces; `bag`: skips if whole object present; `duplicate_bag`:
  adds. Atomic+isolated for a list.
- `insert_new(Table, ObjectOrObjects) -> boolean()` — returns `false` if any key
  exists; nothing inserted unless all keys absent.
- `lookup(Table, Key) -> [Object]` — `set`/`ordered_set`: empty or one-element;
  `bag`/`duplicate_bag`: insertion order preserved.
- `lookup_element(Table, Key, Pos) -> Elem` — `badarg` if no key or `Pos` too
  large.
- `lookup_element(Table, Key, Pos, Default) -> Elem | Default` (OTP 26.0).
- `member(Table, Key) -> boolean()`.
- `delete(Table)` / `delete(Table, Key)` / `delete_object(Table, Object)` /
  `delete_all_objects(Table)`.
- `take(Table, Key) -> [Object]` — return and remove all objects with key.
- `first(Table) -> Key | '$end_of_table'` — `ordered_set` = first in term order;
  others = internal order. `'$end_of_table'` must never be used as a key.
- `last(Table)`, `next(Table, Key1)`, `prev(Table, Key1)`.
- `first_lookup/1`, `next_lookup/2`, `last_lookup/1`, `prev_lookup/2` (OTP 27.0) —
  return `{Key,[Object]}`; more efficient than first/next + lookup.
- `foldl(Function, Acc0, Table)` / `foldr/3` — unspecified order except
  `ordered_set` (first->last / last->first).
- `tab2list(Table) -> [Object]`.
- `i()` / `i(Table)` — terminal browsing.
- `info(Table) -> InfoList | undefined` — items: `compressed`,
  `decentralized_counters`, `heir`, `id`, `keypos`, `memory`, `name`,
  `named_table`, `node`, `owner`, `protection`, `size`, `type`,
  `write_concurrency`, `read_concurrency`.
- `info(Table, Item)` — adds `binary`, `fixed`, `safe_fixed`,
  `safe_fixed_monotonic_time`, `stats`.
- `whereis(TableName) -> tid() | undefined` (OTP 21.0).
- `update_counter(Table, Key, UpdateOp [, Default])` — `set`/`ordered_set` only.
- `update_element(Table, Key, ElementSpec [, Default])` (Default OTP 27.0).

### match / match_object / select + match_spec

- `match(Table, Pattern) -> [Match]` — returns bound-variable lists.
- `match(Table, Pattern, Limit) -> {[Match], Continuation} | '$end_of_table'`.
- `match_object(Table, Pattern)` — returns whole matching objects.
- `match_delete(Table, Pattern)`.
- `select(Table, MatchSpec) -> [Match]` — more general; uses a match spec.
- `select(Table, MatchSpec, Limit) -> {[Match], Continuation} | '$end_of_table'`.
- `select_count(Table, MatchSpec) -> NumMatched`.
- `select_delete(Table, MatchSpec) -> NumDeleted`.
- `select_replace(Table, MatchSpec) -> NumReplaced`.
- `select_reverse/1,2,3` — reverse order for `ordered_set`.
- `fun2ms(LiteralFun) -> MatchSpec` — parse-transform; fun must be a literal.
- `match_spec_compile/1`, `match_spec_run/2`, `is_compiled_ms/1`, `test_ms/2`,
  `repair_continuation/2`.

`match_spec()` is always a list of arity-3 tuples `{MatchHead, [Guard],
[Result]}`. `[Guard]` only `is_`-prefixed tests + logic/arithmetic in prefix
notation. `[Result]` uses bound match variables, `'$_'` (whole object), `'$$'`
(all match variables). Tuples to construct must be wrapped in a 1-tuple.
Equivalences:
- `ets:match(Table,{'$1','$2','$3'})` ==
  `ets:select(Table,[{{'$1','$2','$3'},[],['$$']}])`.
- `ets:match_object(Table,{'$1','$2','$1'})` ==
  `ets:select(Table,[{{'$1','$2','$1'},[],['$_']}])`.

For `ordered_set`, `select` visits in `first`/`next` order. A fully bound key
pattern optimizes to a single lookup; a partially bound key (prefix) limits
traversal. Excessive match-spec nesting raises `system_limit` (scheduler stack
exhaustion).

### Ownership / heir / give_away

- Default owner = the creating process. "When the process terminates, the table
  is automatically destroyed."
- "There is no automatic garbage collection for tables. Even if there are no
  references to a table from any process, it is not automatically destroyed unless
  the owner process terminates. To destroy a table explicitly, use function
  `delete/1`."
- `give_away(Table, Pid, GiftData) -> true` — make `Pid` the new owner; sends
  `{'ETS-TRANSFER',Table,FromPid,GiftData}`. `Pid` must be alive, local, not
  already the owner. Does NOT affect the `heir` option.
- `setopts(Table, Opts) -> true` — the ONLY allowed option after creation is
  `heir`; caller must be the owner.

### ordered_set quirks

- Binary search tree; insert/lookup proportional to log(#objects).
- Keys equal when they *compare equal* (`==`), not only when they *match* (`=:=`).
  `1` and `1.0` are the same key.
- `next/2` always returns the next key after `Key1` in term order, regardless of
  whether `Key1` ever existed (unlike `set`/`bag`/`duplicate_bag` where `next/2`
  fails if `Key1` was deleted, unless fixated).
- `safe_fixtable/2` is NOT necessary for `ordered_set` (nor for single-call
  traversals like `select/2`).
- `write_concurrency` had no effect on `ordered_set` prior to stdlib-3.7 (OTP
  22.0).

### safe_fixtable

`safe_fixtable(Table, true|false)` fixes `set`/`bag`/`duplicate_bag` for safe
traversal under concurrent updates. Deleted objects are NOT freed while fixed; a
process that fixes but never releases leaks memory and degrades perf. Use
`info(Table, safe_fixed_monotonic_time)` (time-warp safe) to inspect fixations;
`safe_fixed` is NOT time-warp safe. "A table traversal is safe if either the
table is of type `ordered_set`, the entire table traversal is done within one ETS
function call, [or] function `safe_fixtable/2` is used."

### DETS — disk-based term storage

DETS is "A disk-based term storage." It stores Erlang terms (objects = tuples
with one element designated as key) in a file. "This module is used by the
Mnesia application, and is provided \"as is\" for users who are interested in
efficient storage of Erlang terms on disk only."

Table types (`{type, type()}`, default `set`):
- `set` — at most one object per key; inserting an object with an existing key
  overwrites it.
- `bag` — zero or more *different* objects per key.
- `duplicate_bag` — zero or more *possibly matching* (identical) objects per key.
- **NO `ordered_set`.** "Notice that type ordered_set in Ets is not yet provided
  by Dets, neither is the limited support for concurrent updates that makes a
  sequence of first and next calls safe to use on fixed ETS tables. ...
  Currently, no Erlang/OTP library has support for ordered disk-based term
  storage."

`open_file/1` and `open_file/2`:
- `open_file(Filename) -> {ok, Reference} | {error, Reason}` — opens an existing
  table; returns a `reference()` (not the table name); most useful for
  debugging. Repairs if not properly closed.
- `open_file(Name, Args) -> {ok, Name} | {error, Reason}` — opens/creates a
  table; `Name` is the atom used in all subsequent ops. "An empty Dets table is
  created if no file exists." Multiple processes may share one table by the
  same name.

`open_file/2` `Args` options (`[OpenArg]`):
- `{access, read_write | read_only}` (default `read_write`). Read-only tables
  are NOT subjected to the automatic repair algorithm if later opened after a
  crash.
- `{auto_save, Time | infinity}` — integer `Time` ms (flush to disk after `Time`
  ms idle; flushed tables need no repair after uncontrolled halt) or `infinity`
  (disabled). Default `180000` (3 min).
- `{file, file:name()}` — file to open; defaults to the table name.
- `{keypos, keypos()}` — key element position; default 1. Convenient for
  storing records (first element = record name).
- `{max_no_slots, no_slots()}` — max slots; default 32 M (the maximal value).
  Higher → more fragmentation; lower → less fragmentation but slower.
- `{min_no_slots, no_slots()}` — estimated number of distinct keys at creation;
  enhances performance. Default 256 (minimum value).
- `{ram_file, boolean()}` — keep table in RAM; on close, contents written to
  disk file. Default `false`. Useful for open→insert-set→close workloads.
- `{repair, boolean() | force}` — invoke automatic file repair. Default `true`.
  `false` → no repair attempt, returns `{error, {needs_repair, FileName}}` if
  repair needed. `force` → repair even if properly closed (seldom needed).
  Ignored if table already open.

CRUD and traversal (similar API to ETS but returns `{error, Reason}` on error):
- `insert(Name, Objects) -> ok | {error, Reason}` — `set`: matching key
  replaces.
- `insert_new(Name, Objects) -> boolean() | {error, Reason}` — inserts only if
  no object with matching key exists.
- `lookup(Name, Key) -> Objects | {error, Reason}` — `set`: empty or
  one-element; `bag`/`duplicate_bag`: arbitrary length, order unspecified.
- `delete(Name, Key) -> ok | {error, Reason}` — deletes all objects with key.
- `delete_object(Name, Object) -> ok | {error, Reason}` — deletes all instances
  of a specific object (useful for `bag`/`duplicate_bag`).
- `delete_all_objects(Name) -> ok | {error, Reason}` — almost-constant time.
- `match/1,2,3`, `match_object/1,2,3`, `match_delete/2`, `select/1,2,3`,
  `select_delete/2`, `foldl/3`, `foldr/3` — similar to ETS but return
  `{error, Reason}` on error. Traversal order unspecified.

`safe_fixtable/2` required before chunked traversal: "The table is always to be
protected using safe_fixtable/2 before calling match/3, otherwise errors can
occur when calling match/1." (Analogously for `select/3`.)

`sync/1`: "Ensures that all updates made to table Name are written to disk."
Flushes all updates plus the in-RAM buddy-system space-management structures;
also flushes `ram_file` tables. Can take time if fragmented.

`from_ets/2`, `to_ets/2` — bulk copy between ETS and DETS tables.

`info/1` returns tuples `{file_size}`, `{filename}`, `{keypos}`, `{size}`,
`{type}`. `info/2` items: `access | auto_save | bchunk_format | hash |
file_size | filename | keypos | memory | no_keys | no_objects | no_slots |
owner | ram_file | safe_fixed | safe_fixed_monotonic_time | size | type`.

### DETS vs ETS differences

- **Disk-backed**: "As all operations performed by Dets are disk operations, it
  is important to realize that a single look-up operation involves a series of
  disk seek and read operations. The Dets functions are therefore much slower
  than the corresponding ets functions, although Dets exports a similar
  interface."
- **File-per-table**: each DETS table is one file (`{file, ...}` option,
  defaults to table name); ETS tables live in RAM.
- **No `ordered_set`** in DETS (see above).
- **No safe concurrent `first`/`next` on fixed tables** (unlike ETS): the
  limited ETS support that makes `first`+`next` safe on fixed ETS tables is NOT
  provided by DETS. Safe concurrency requires Mnesia (or user-implemented
  locking).
- **Size limit**: "The size of Dets files cannot exceed 2 GB. If larger tables
  are needed, table fragmentation in Mnesia can be used."
- **Repair on open**: "Dets tables must be opened before they can be updated or
  read, and when finished they must be properly closed. If a table is not
  properly closed, Dets automatically repairs the table. This can take a
  substantial time if the table is large."
- **Closing**: "A Dets table is closed when the process which opened the table
  terminates. If many Erlang processes (users) open the same Dets table, they
  share the table. The table is properly closed when all users have either
  terminated or closed the table. Dets tables are not properly closed if the
  Erlang runtime system terminates abnormally."
- `auto_save` periodically flushes the table so a flushed table needs no repair
  after an uncontrolled emulator halt.
- **Internal structure**: "Dets organizes data as a linear hash list and the
  hash list grows gracefully as more data is inserted into the table. Space
  management on the file is performed by what is called a buddy system. The
  current implementation keeps the entire buddy system in RAM, which implies
  that if the table gets heavily fragmented, quite some memory can be used up.
  The only way to defragment a table is to close it and then open it again
  with option repair set to force."
- **Error convention**: "All Dets functions return {error, Reason} if an error
  occurs (first/1 and next/2 are exceptions, they exit the process with the
  error tuple). If badly formed arguments are specified, all functions exit the
  process with a badarg message."
- `first/1` and `next/2` EXIT the process (not return `{error,_}`) on failure.
- Traversal order is unspecified; do not rely on insertion order.

### Mnesia — transactional distributed DBMS

Mnesia is "a multiuser distributed DBMS specifically designed for
industrial-grade telecommunications applications written in Erlang, which is
also the intended target language." It "almost turns Erlang into a database
programming language." Mnesia is a transactional, optionally-distributed DBMS
layered on top of Erlang's ETS (in-memory) and DETS (on-disk) table primitives
(DETS itself backs onto `disk_log`). It is not a SQL database and not an
external server process; it lives in the Erlang runtime, in the same address
space as the application.

Key properties (verbatim where available):
- "The database schema can be dynamically reconfigured at runtime."
- "Tables can be moved or replicated to several nodes to improve fault
  tolerance. Other nodes in the system can still access the tables to read,
  write, and delete records."
- "Table locations are transparent to the programmer. Programs address table
  names and the system itself keeps track of table locations."
- "Multiple transactions can run concurrently and their execution is fully
  synchronized by Mnesia, ensuring that no two processes manipulate the same
  data simultaneously."
- "Transactions can be bypassed using dirty operations, which reduce overheads
  and run fast."

Storage types (per-replica property):
- `ram_copies` — in-memory only (ETS-backed); fastest; lost on node restart.
- `disc_copies` — in-memory + on-disc replica (ETS + DETS/disk_log); fast reads,
  durable writes, survives restart.
- `disc_only_copies` — on-disc only (DETS-backed); slower, lower RAM footprint,
  no in-memory copy.

> NOTE: these storage-type identifiers are stable Mnesia facts but were NOT
> quoted from the overview page; verify exact option names in
> `mnesia_chap1.html`.

A single table can have different storage types on different nodes (e.g. ram on
hot node, disc on backup node), which is how Mnesia trades speed vs durability
vs fault tolerance per replica. The schema itself is a Mnesia table; schema is
dynamically reconfigurable at runtime (add/move/drop replicas without stopping
the system).

Transactions vs dirty:
- Transactions are the consistency model: distributed, multi-operation, fully
  synchronized, mutually exclusive on overlapping data. Use them when several
  records must be updated atomically/safely together.
- Dirty operations bypass transactions: lower overhead, faster, but no
  isolation/atomicity guarantees across multiple ops. Use for hot-path
  single-record reads/writes where you accept the trade-off (e.g.
  `dirty_read`, `dirty_write`, `dirty_delete`).
- Rule of thumb: prefer transactions for correctness when multiple
  records/nodes are involved; use dirty ops only for performance-critical
  single-record access where stale/inconsistent reads are tolerable.

Replication/distribution:
- Replication is declarative: a table is declared to have replicas on a set of
  nodes; Mnesia keeps them in sync. Replicas improve fault tolerance.
- Distribution is transparent: programs address table names, not nodes; Mnesia
  routes to the right replica.
- Dynamic reconfiguration: tables can be moved or replicated to additional
  nodes at runtime without service disruption.

Fragmentation (splitting a large table across nodes by key range) is a Mnesia
feature but is NOT described on the overview page; it is covered in the deeper
Mnesia chapters (not crawled). Decision note: fragmentation is the answer when
a single table's size or write load exceeds what one node can hold,
complementing replication (which is about fault tolerance, not scale-out).

### ETS vs DETS vs Mnesia vs external store — decision table

Synthesized from the Mnesia overview "When to Use Mnesia" section:

| Need | Choose |
|---|---|
| In-memory key/value, single process, no persistence, fastest | ETS |
| On-disc key/value dictionary, single node, no transactions | DETS |
| Append-only disc log / WAL semantics | disk_log |
| Replicated, transactional, distributed, soft-real-time, complex Erlang records | Mnesia |
| Hard real-time deadlines | neither Mnesia nor ETS guarantees; redesign |
| SQL, ad-hoc reporting, huge datasets beyond a few nodes | external SQL/NoSQL store |

Verbatim "When to Use Mnesia" guidance:

Mnesia is a great fit for applications that:
- "Need to replicate data. Perform complex data queries. Need to use atomic
  transactions to safely update several records simultaneously. Require soft
  real-time characteristics."

Mnesia is not as appropriate for applications that:
- "Process plain text or binary data files. Merely need a lookup dictionary
  that can be stored on disc. ... Need disc logging facilities. ... Require
  hard real-time characteristics."

### Mnesia limitations

- No SQL interface; querying via QLC (list comprehensions) or match/select API.
- Soft real-time only — not suitable for hard real-time.
- Not a bulk file/blob store.
- Distribution scales to a limited number of nodes; full-mesh transaction
  coordination not designed for very large clusters.
- Tight Erlang coupling; no wire protocol for non-Erlang clients.

### Revisit later

Mnesia API details (create_table signatures, exact storage-type option names,
transaction/dirty-op function signatures, fragmentation configuration) live in
`mnesia_chap1.html` and `mnesia_chap2.html`, which were NOT crawled. Mnesia
coverage here is decision-level only. Revisit: crawl the Mnesia chapters for
source-verified API coverage.

## Practical rules

- A table dies with its owner unless `heir` is set or ownership was transferred.
- Access rights are fixed at creation; violations raise `badarg`.
- `ordered_set` uses compare-equal (`==`); others use match (`=:=`).
- `'$end_of_table'` must never be used as a key.
- Every object insert/lookup copies the object.
- `setopts/2` only allows changing `heir`, and only the owner may call it.
- Heir must be local; `{heir,Pid}` (no data) sends NO transfer message.
- Use `safe_fixtable` for multi-call traversals on `set`/`bag`/`duplicate_bag`.
- Use `info(Table, safe_fixed_monotonic_time)` (not `safe_fixed`) in time-warp-
  safe code.
- A DETS table MUST be opened before use and properly closed when finished.
- All open DETS tables MUST be closed before system stop.
- Only processes that opened a table may close it.
- Before `match/3`/`select/3` on DETS, the table MUST be `safe_fixtable/2`-protected.
- DETS file size MUST NOT exceed 2 GB (use Mnesia fragmentation for larger).
- Do not assume `ordered_set` semantics in DETS — it does not provide it.
- Do not rely on `first`/`next` for safe concurrent DETS traversal — use Mnesia or external locking.
- DETS `first/1`/`next/2` exit the process on failure (not `{error,_}`).
- Use Mnesia transactions for atomic multi-record/multi-node updates; dirty ops only for single-record hot paths.

## Review checklist

- [ ] Is the table type chosen for the access pattern (set vs ordered_set vs bag)?
- [ ] Is protection `protected` (default) or stricter, unless writers are many?
- [ ] Is an `heir` set for tables that must survive owner restarts?
- [ ] Are traversals safe (ordered_set, single-call, or `safe_fixtable`)?
- [ ] Are match specs compiled once and reused (not rebuilt per call)?
- [ ] Is `'$end_of_table'` excluded from the key space?
- [ ] Is DETS used only when disk persistence is needed without transactions?
- [ ] Are DETS tables properly closed (and all closed before system stop)?
- [ ] Is `safe_fixtable/2` called before DETS `match/3`/`select/3`?
- [ ] Is the DETS file under 2 GB (or Mnesia fragmentation used)?
- [ ] For transactional/distributed needs, is Mnesia chosen over raw ETS/DETS?

## Implementation checklist

- [ ] `ets:new/2` with explicit type, protection, and `{keypos,Pos}` for records.
- [ ] Use `named_table` + `whereis/1` for stable references.
- [ ] Use `insert_new/2` for create-if-absent semantics.
- [ ] Use `select/3` with `Limit` + continuation for large traversals.
- [ ] Use `fun2ms/1` (literal fun) to author match specs safely.
- [ ] Pair `safe_fixtable(T, true)` with `safe_fixtable(T, false)` in a `try/after`.
- [ ] `dets:open_file/2` with explicit `{type, ...}`, `{keypos, ...}`, `{file, ...}`.
- [ ] Pair `dets:open_file/2` with `dets:close/1` in a `try/after`.
- [ ] Use `dets:sync/1` before critical close if durability is required mid-session.
- [ ] Use `safe_fixtable(T, true)` ... `safe_fixtable(T, false)` around DETS `match/3`/`select/3`.

## Runtime / debugging checklist

- [ ] `ets:i()` to list all tables; `ets:i(Table)` to browse one.
- [ ] `ets:info(Table)` / `info(Table, Item)` for owner, size, memory, fixation.
- [ ] `ets:info(Table, safe_fixed_monotonic_time)` to find leaked fixations.
- [ ] `erlang:system_info(ets_count)` / `ets_limit` for table-count pressure.
- [ ] `erlang:memory(ets)` for ETS memory share.
- [ ] `dets:info(Name)` / `info(Name, Item)` for file_size, size, type, owner, fixation.
- [ ] `dets:info(Name, safe_fixed_monotonic_time)` to find leaked DETS fixations.
- [ ] Watch for DETS repair time on reopen after abnormal termination.

## Validation hooks

- After `ets:new/2`, assert the returned `tid()`/name is valid.
- `ets:info(Table, owner)` asserts the expected owner.
- `ets:info(Table, size)` asserts expected object count.
- `ets:info(Table, safe_fixed_monotonic_time)` asserts no leaked fixations.
- `ets:member/2` / `lookup/2` assertions in tests.
- `ets:test_ms/2` to validate a match spec before deployment.

## Examples

Create a named set keyed by record field:
```erlang
-record(user, {id, name, email}).
Tab = ets:new(users, [set, named_table, public, {keypos, #user.id}]),
ets:insert(Tab, #user{id = 1, name = <<"Ada">>, email = <<"ada@x">>}),
[#user{id = 1, name = Name}] = ets:lookup(Tab, 1).
```

Safe traversal with a continuation:
```erlang
{Matches, Cont} = ets:select(Tab, [{{'$1','$2'},[],['$$']}], 100),
loop(Cont, Matches).
loop('$end_of_table', Acc) -> Acc;
loop(Cont, Matches) ->
    {More, Next} = ets:select(Cont),
    loop(Next, Matches ++ More).
```

DETS open/insert/lookup/sync/close:
```erlang
{ok, users} = dets:open_file(users, [{type, set}, {keypos, 2}, {file, "users.db"}]),
dets:insert(users, {user, 1, <<"Ada">>}),
[{user, 1, Name}] = dets:lookup(users, 1),
ok = dets:sync(users),
ok = dets:close(users).
```

## Common mistakes

- Letting the owner die without an `heir` (table lost).
- Using `next/2` on a `set` without `safe_fixtable` under concurrent deletes.
- Treating `1` and `1.0` as distinct keys in `ordered_set` (they are equal).
- Rebuilding match specs per call instead of compiling once.
- Forgetting to release `safe_fixtable` (memory leak, perf degradation).
- Using `safe_fixed` (not time-warp safe) in time-warp-safe code.
- Forgetting to close a DETS table (triggers repair on next open).
- Using `match/3`/`select/3` on DETS without `safe_fixtable/2`.
- Expecting `ordered_set` from DETS (not supported).
- Exceeding the 2 GB DETS file limit (use Mnesia fragmentation).
- Treating DETS `first/1`/`next/2` errors as `{error,_}` (they exit the process).
- Using ETS where transactions/replication are needed (use Mnesia).

## Strict vs contextual guidance

Strict:
- Tables die with the owner unless `heir`/`give_away`.
- `ordered_set` uses compare-equal; others use match.
- `setopts/2` only allows `heir`, owner-only.
- `'$end_of_table'` is a reserved key.
- DETS tables MUST be opened before use and closed when finished.
- DETS file size MUST NOT exceed 2 GB.
- DETS does NOT support `ordered_set`.
- `safe_fixtable/2` MUST be called before DETS `match/3`/`select/3`.
- Mnesia transactions for atomic multi-record/multi-node updates.

Contextual:
- `write_concurrency`/`read_concurrency`/`decentralized_counters` — tune by load.
- `compressed` — trade space for speed.
- `fun2ms` vs hand-written match specs — `fun2ms` is safer but fun must be literal.
- DETS `auto_save` interval tuning.
- DETS `ram_file` for open→insert-set→close workloads.
- Mnesia dirty ops vs transactions (performance vs consistency trade-off).
- Mnesia storage type per replica (ram_copies vs disc_copies vs disc_only_copies).

## Policy decisions for individual repos

- Default protection level (`protected` vs `private`) for internal tables.
- Whether `heir` is mandatory for long-lived tables.
- Whether `write_concurrency, auto` is the default (OTP 25+).
- Match-spec nesting depth limits (vs `system_limit`/`+sss`).
- Whether DETS is used at all, or Mnesia/external store is preferred for persistence.
- DETS `auto_save` interval policy.
- Whether Mnesia dirty ops are permitted on hot paths.
- Mnesia storage-type defaults per table class.
- Whether Mnesia fragmentation is used for large tables.

## Related docs

- `runtime-debugging.md` — `ets:info`, `system_info(ets_count)`.
- `validation.md` — ETS validation hooks.
- `processes-and-messages.md` — table ownership and process lifecycle.
- `runtime-environment.md` — `file` module (DETS uses `file:name()` type).
- `distribution.md` — distributed Mnesia, node trust model.

## Related skills

- `beam-observability-debugging`
- `beam-processes`
- `beam-errors-failures`
- `beam-applications-releases`
- `beam-logger-config`
