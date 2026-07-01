# Crawl: stdlib/ets.html (focused)
- seed_url: https://www.erlang.org/doc/apps/stdlib/ets.html
- canonical_url: https://www.erlang.org/doc/apps/stdlib/ets.html
- family: Erlang/OTP stdlib module docs
- fetch: 200
- otp_version: OTP 29.0.2 (stdlib 8.0.1)
- feeds_docs: ets-data.md (or a data-storage topic)

## Purpose
`ets` is the interface to Erlang's built-in term storage BIFs: dynamic tables of
tuples with constant-time access (logarithmic for `ordered_set`). Tables are
created by a process, are automatically destroyed when the owner terminates
(unless an `heir` is set or ownership is transferred via `give_away/3`), and
have access rights set at creation. No automatic GC for tables: even with no
references a table lives until its owner dies or `delete/1` is called.

Four table types: `set`, `ordered_set`, `bag`, `duplicate_bag`.
- `set`/`ordered_set`: one object per key.
- `bag`: many objects per key, but only one instance of each object.
- `duplicate_bag`: many objects per key, including duplicate copies.

Insert/lookup is constant for `set`; proportional to #objects with same key for
`bag`/`duplicate_bag` (hash-bucket linear search does not yield); logarithmic
for `ordered_set` (binary search tree).

Failures: functions raise `error` exceptions with reason `badarg` (bad arg
format, invalid table id, denied access on protected/private) or `system_limit`
(unrepresentable value, e.g. counter overflow; or excessive match-spec nesting
causing scheduler stack exhaustion).

## new/2 table types + protection + options
`new(Name, Options) -> table()`  (Name :: atom())

Spec:
```
Option :: Type | Access | named_table |
           {keypos, Pos} |
           {heir, Pid} | {heir, Pid, HeirData} | {heir, none} |
           Tweaks
Type  :: table_type()            % set | ordered_set | bag | duplicate_bag
Access:: table_access()          % public | protected | private
Tweaks:: {write_concurrency, boolean() | auto} |
         {read_concurrency, boolean()} |
         {decentralized_counters, boolean()} |
         compressed
Pos   :: pos_integer()
Pid   :: pid()
HeirData :: term()
```

Default options (i.e. `[]`) equals:
`[set, protected, {keypos,1}, {heir,none}, {write_concurrency,false},
  {read_concurrency,false}, {decentralized_counters,false}]`

Table types:
- `set` (default): one key, one object, no order among objects.
- `ordered_set`: one key, one object, ordered in Erlang term order (the order
  implied by `<`/`>`). Different behavior: keys regarded equal when they
  *compare equal*, not only when they *match*. So `integer 1` and `float 1.0`
  are equal keys; the key used to lookup need not match the key stored.
- `bag`: many objects, only one instance of each object, per key.
- `duplicate_bag`: many objects, including multiple copies, per key.

Protection (access rights):
- `public`: any process can read or write.
- `protected` (default): owner can read/write; other processes can only read.
- `private`: only owner can read or write.

Options:
- `named_table`: table registered under `Name`; `Name` returned instead of tid;
  use `whereis/1` to get the tid. Named-table lookup perf degrades if many
  named tables exist and `ERL_MAX_ETS_TABLES` not increased (the old hard limit
  is removed; only the internal named-table table is sized by it).
- `{keypos,Pos}`: which tuple element is the key (default 1). Useful for
  records. Any stored tuple must have at least `Pos` elements.
- `{heir,Pid,HeirData} | {heir,Pid} | {heir,none}`: set a process as heir.
  Heir inherits the table if owner terminates. With `HeirData`, message
  `{'ETS-TRANSFER',tid(),FromPid,HeirData}` is sent to heir. With `{heir,Pid}`
  (no data) NO `'ETS-TRANSFER'` message is sent — caller must notify heir via
  link/monitor. Heir must be a local process. Default `none` destroys table on
  owner termination.
- `{write_concurrency, boolean() | auto}` (default `false`): optimize for
  concurrent writes; different objects mutable by concurrent processes at the
  cost of memory and sequential/read perf. `auto` (only OTP-25.0+) adjusts
  synchronization granularity at runtime — recommended for OTP 25+. Does NOT
  change atomicity/isolation guarantees. Prior to stdlib-3.7 (OTP-22.0) had no
  effect on `ordered_set`.
- `{read_concurrency, boolean()}` (Since OTP R14B; default `false`): optimize
  for concurrent reads; reads cheaper (esp. multi-processor), read/write
  switching more expensive. Combine with `write_concurrency` for large
  read/write bursts.
- `{decentralized_counters, boolean()}` (Since OTP 23.0): defaults `true` when
  `write_concurrency` is `auto`, and also `true` for `ordered_set` when
  `write_concurrency` is `true`; otherwise `false`. No effect if
  `write_concurrency` is `false`. Optimizes frequent concurrent size/memory
  mutations (`insert/2`, `delete/2`) but makes `info/1,2` with `size`/`memory`
  much slower. Counters distributed over cache lines; `+dcg` controls count.
- `compressed` (Since OTP R14B01): more compact storage, slower operations
  (especially `match`/`select`); the key element is NOT compressed.

## CRUD + traversal functions (exact arities)
- `insert(Table, ObjectOrObjects)` — insert object or list of objects. `set`:
  matching key replaces old object. `ordered_set`: *compare-equal* key replaces.
  `bag`: object not inserted if whole object already present. `duplicate_bag`:
  adds. Entire operation atomic+isolated even for a list. For `bag`/
  `duplicate_bag`, identical-key objects inserted in list order (head→tail);
  `lookup` returns them in that order. (Note: bag insertion order was
  accidentally reversed OTP 23.0 → fixed OTP 25.3; duplicate_bag same faulty
  reverse but unpredictable.)
- `insert_new(Table, ObjectOrObjects) -> boolean()` — like `insert/2` but
  returns `false` instead of overwriting/adding when key exists. For a list,
  checks every key before inserting anything; nothing inserted unless all keys
  absent. Atomic+isolated.
- `lookup(Table, Key) -> [Object]` — all objects with key. `set`/`ordered_set`:
  empty or one-element list. `bag`/`duplicate_bag`: arbitrary length, insertion
  order preserved. `ordered_set` matches on *compare equal* (=:=/== difference
  → e.g. insert with integer `1`, lookup with float `1.0` returns it).
- `lookup_element(Table, Key, Pos) -> Elem` — `set`/`ordered_set`: `Pos`th
  element of the object; `bag`/`duplicate_bag`: list of `Pos`th elements.
  `badarg` if no such key or `Pos` > tuple size. Same ordered_set compare-equal
  semantics.
- `lookup_element(Table, Key, Pos, Default) -> Elem | Default` (since OTP 26.0)
  — like /3 but returns `Default` when key absent (still `badarg` if `Pos` too
  large).
- `member(Table, Key) -> boolean()` — like `lookup/2` but returns only
  `true`/`false`, no objects.
- `delete(Table)` — delete entire table.
- `delete(Table, Key)` — delete all objects with key `Key`; succeeds even if
  none exist.
- `delete_object(Table, Object)` — delete the exact object (useful for `bag`);
  in `duplicate_bag` all instances of that object are deleted.
- `delete_all_objects(Table)` — delete all objects; atomic+isolated.
- `take(Table, Key) -> [Object]` — return and remove all objects with key.
- `first(Table) -> Key | '$end_of_table'` — first key; `ordered_set` = first
  in Erlang term order, others = internal order. `'$end_of_table'` if empty.
  (`'$end_of_table'` must never be used as a key.)
- `last(Table)` — `ordered_set`: last in term order; others = synonymous with
  `first/1`.
- `next(Table, Key1) -> Key2 | '$end_of_table'` — next key after `Key1`.
  `ordered_set`: next in term order. For `set`/`bag`/`duplicate_bag` (unless
  fixated) `next/2` FAILS if `Key1` no longer exists; `ordered_set` always
  returns the next key after `Key1` in term order regardless of whether `Key1`
  ever existed.
- `prev(Table, Key1)` — `ordered_set`: previous in term order; others =
  synonymous with `next/2`.
- `first_lookup/1`, `next_lookup/2`, `last_lookup/1`, `prev_lookup/2` (since
  OTP 27.0) — like first/next/last/prev but return `{Key,[Object]}`; more
  efficient than first/next + lookup.
- `foldl(Function, Acc0, Table)` — like `lists:foldl/3`; unspecified order
  except `ordered_set` (first→last). Returns `Acc0` if empty.
- `foldr(Function, Acc0, Table)` — like `lists:foldr/3`; unspecified order
  except `ordered_set` (last→first).
- `tab2list(Table) -> [Object]` — all objects as a list.
- `i()` — display info about all ETS tables on terminal.
- `i(Table)` — browse table on terminal.
- `info(Table) -> InfoList | undefined` — list of `{Item,Value}` tuples
  (compressed, decentralized_counters, heir, id, keypos, memory, name,
  named_table, node, owner, protection, size, type, write_concurrency,
  read_concurrency). `undefined` if table gone; `badarg` if not a table id.
- `info(Table, Item) -> Value | undefined` — items above plus: `binary`,
  `fixed`, `safe_fixed`, `safe_fixed_monotonic_time`, `stats`.
- `whereis(TableName) -> tid() | undefined` (since OTP 21.0) — tid of named
  table; lets a sequence of calls target the same table even if name is
  concurrently recycled.
- `rename(Table, Name)` — rename a named table; old name no longer usable.
  Renaming an unnamed table has no effect.
- `update_counter(Table, Key, UpdateOp [, Default])` /3,/4 — atomic counter
  update(s); `set`/`ordered_set` only; key matched (set) or compare-equal
  (ordered_set). `badarg` if element to update is the key, or not an integer.
- `update_element(Table, Key, ElementSpec [, Default])` /3,/4 (Default since
  OTP 27.0) — destructive element update; `true` if key found else `false`.

## match / match_object / select + match_spec
- `match(Table, Pattern) -> [Match]` — match objects against `Pattern`; returns
  list of bound-variable lists (not the objects).
- `match(Table, Pattern, Limit) -> {[Match], Continuation} | '$end_of_table'`
  — limited chunk + continuation.
- `match(Continuation)` — continue a `match/3`.
- `match_object(Table, Pattern)` / `match_object/3` / `match_object/1` — like
  `match` but returns the whole matching objects.
- `match_delete(Table, Pattern)` — delete all objects matching pattern.
- `select(Table, MatchSpec) -> [Match]` — more general than match/match_object;
  uses a match specification.
- `select(Table, MatchSpec, Limit) -> {[Match], Continuation} | '$end_of_table'`
  — limited chunk + continuation. Use `safe_fixtable/2` for safe subsequent
  `select/1` calls.
- `select(Continuation)` — continue a `select/3`.
- `select_count(Table, MatchSpec) -> NumMatched` — count objects for which
  match spec returns `true`.
- `select_delete(Table, MatchSpec) -> NumDeleted` — delete objects for which
  match spec returns `true`.
- `select_replace(Table, MatchSpec) -> NumReplaced` — replace each matched
  object with the match-spec result.
- `select_reverse/1,2,3` — like select but reverse order for `ordered_set`
  (traversal from last→first); identical to select for other types.
- `fun2ms(LiteralFun) -> MatchSpec` — parse-transform pseudo function
  converting a *literal* fun (textually written as the argument, not held in a
  variable) into a match specification.
- `match_spec_compile(MatchSpec) -> CompiledMatchSpec` — compile to opaque
  internal form (valid only on the compiling node).
- `match_spec_run(List, CompiledMatchSpec) -> [term()]` — run a compiled match
  spec against a list of terms.
- `is_compiled_ms(Term) -> boolean()` — check valid compiled match spec.
- `test_ms(Tuple, MatchSpec) -> {ok, Result} | {error, Errors}` — test a match
  spec for correctness and run it against a tuple.
- `repair_continuation(Continuation, MatchSpec)` — restore an opaque
  continuation that passed through external term format.

`match_spec()` type: `[{match_pattern(), [_], [_]}]` — always a list of
arity-3 tuples `{MatchHead, [Guard], [Result]}`:
- `MatchHead` = pattern as in `ets:match` (e.g. `{'$1','$2','$3'}`).
- `[Guard]` = 0+ guard tests as tuples `{TestName, ...}` (only `is_`-prefixed
  tests allowed: `is_float`, `is_atom`, ...); also logic/arithmetic in prefix
  notation.
- `[Result]` = term construct using bound match variables, plus special
  `'$_'` (whole matching object) and `'$$'` (all match variables as a list).
  Tuples to construct must be written as a 1-tuple wrapping the desired tuple
  (an ordinary tuple would be mistaken for a Guard).

Equivalences:
- `ets:match(Table,{'$1','$2','$3'})` ≡
  `ets:select(Table,[{{'$1','$2','$3'},[],['$$']}])`
- `ets:match_object(Table,{'$1','$2','$1'})` ≡
  `ets:select(Table,[{{'$1','$2','$1'},[],['$_']}])`

For `ordered_set`, `select` visits objects in `first`/`next` order and the
result list is in that order. A match pattern with a fully bound key optimizes
to a single key lookup (no full traversal); for `ordered_set` a partially bound
key (list/tuple prefix) limits traversal to a term-order subset.

Excessive match-spec nesting raises `system_limit` (scheduler stack
exhaustion; stack size configurable at runtime start).

## Ownership / heir / give_away
- Default owner = the process that created the table.
- Table is automatically destroyed when the owner terminates, UNLESS:
  - option `heir` is set (heir inherits the table), or
  - ownership is transferred via `give_away/3`.
- No automatic GC: a table with no references is NOT destroyed unless the owner
  terminates; destroy explicitly with `delete/1`.
- `give_away(Table, Pid, GiftData) -> true` — make `Pid` the new owner. On
  success sends `{'ETS-TRANSFER',Table,FromPid,GiftData}` to the new owner.
  `Pid` must be alive, local, and not already the owner. Caller must be the
  owner. Does NOT affect the `heir` option (owner can set `heir` to itself,
  give the table away, and reclaim it if the receiver dies).
- `setopts(Table, Opts) -> true` — set table options AFTER creation. The ONLY
  allowed option is `heir` (`{heir,Pid} | {heir,Pid,HeirData} | {heir,none}`).
  Caller must be the owner.
- Heir must be a local process; default `none` destroys the table on owner
  termination.

## ordered_set quirks
- Uses a binary search tree; insert/lookup time ∝ log(#objects).
- Keys are regarded equal when they *compare equal* (==), not only when they
  *match* (=:=). So `integer 1` and `float 1.0` are the same key. The lookup
  key need not match the stored key when ints/floats are mixed.
- No defined order exists between an `integer` and a `float` extending to the
  same value; hence `1` and `1.0` are regarded as equal in `ordered_set`.
- `next/2` always returns the next key after `Key1` in term order, regardless
  of whether `Key1` ever existed in the table (unlike `set`/`bag`/
  `duplicate_bag` where `next/2` fails if `Key1` was deleted, unless fixated).
- `safe_fixtable/2` is NOT necessary for `ordered_set` (nor for single-call
  traversals like `select/2`).
- `select`/`select_reverse` visit in `first`/`next` (or `last`/`prev`) order.
- `write_concurrency` had no effect on `ordered_set` prior to stdlib-3.7
  (OTP-22.0).
- Note re: requested "OTP-24 quirks (keypos, equality)": this OTP-29 page does
  NOT contain an OTP-24-specific quirk note. The `keypos` behavior (default 1,
  any stored tuple must have ≥ `Pos` elements) and the ordered_set
  compare-equal-vs-match equality rule are documented as general/current
  behavior, not as OTP-24 changes. No `since OTP 24` markers appear on the page.

## Strict rules (ownership death; protection; ordered_set equality)
1. A table dies with its owner process on termination unless `heir` is set or
   ownership was transferred with `give_away/3`. There is no GC of otherwise
   unreferenced tables.
2. Access rights are fixed at creation: `public` (all read/write),
   `protected` (owner read/write, others read-only; default), `private`
   (owner only). Violations raise `badarg`.
3. `ordered_set` uses *compare equal* (==) for key equality; all other types
   use *match* (=:=). The difference is the same as between `=:=` and `==`.
4. `'$end_of_table'` must never be used as a key (reserved by `first/1`/
   `next/2`).
5. Every object insert/lookup copies the object.
6. Single-object updates are atomic+isolated; some multi-object functions
   (`insert/2`, `delete_all_objects/1`, `insert_new/2`) guarantee atomicity+
   isolation for the whole operation. Traversal is NOT a consistent snapshot
   under concurrent updates unless the table is `ordered_set`, the traversal is
   a single ETS call, or `safe_fixtable/2` is used.
7. `setopts/2` only allows changing `heir`, and only the owner may call it.
8. Heir must be a local process; `{heir,Pid}` (no data) sends NO
   `'ETS-TRANSFER'` message — caller must notify the heir via link/monitor.
9. `safe_fixtable/2` fixes `set`/`bag`/`duplicate_bag` for safe traversal;
   deleted objects are NOT freed while fixed; a process that fixes but never
   releases leaks memory and degrades perf. Use
   `info(Table, safe_fixed_monotonic_time)` (time-warp safe) to inspect
   fixations; `safe_fixed` is NOT time-warp safe.

## Verbatim quotes
- "Data is organized as a set of dynamic tables, which can store tuples. Each
  table is created by a process. When the process terminates, the table is
  automatically destroyed."
- "Notice that there is no automatic garbage collection for tables. Even if
  there are no references to a table from any process, it is not automatically
  destroyed unless the owner process terminates. To destroy a table explicitly,
  use function `delete/1`. The default owner is the process that created the
  table. To transfer table ownership at process termination, use option `heir`
  or call `give_away/3`."
- "Two Erlang terms `match` if they are of the same type and have the same
  value, so that `1` matches `1`, but not `1.0`... Two Erlang terms compare
  equal if they either are of the same type and value, or if both are numeric
  types and extend to the same value, so that `1` compares equal to both `1`
  and `1.0`."
- "The `ordered_set` works on the Erlang term order and no defined order exists
  between an `integer/0` and a `float/0` that extends to the same value. Hence
  the key `1` and the key `1.0` are regarded as equal in an `ordered_set`
  table."
- "`ordered_set` - The table is a `ordered_set` table: one key, one object,
  ordered in Erlang term order... Most notably, the `ordered_set` tables regard
  keys as equal when they compare equal, not only when they match."
- "`{heir,Pid,HeirData} | {heir,Pid} | {heir,none}` - Set a process as heir. The
  heir inherits the table if the owner terminates... If `{heir,Pid}` is given,
  no `'ETS-TRANSFER'` message is sent. The user must then make sure the heir
  gets notified some other way (through a link or monitor for example)..."
- "`setopts(Table, Opts)` - Sets table options. The only allowed option to be
  set after the table has been created is `heir`. The calling process must be
  the table owner."
- "`give_away(Table, Pid, GiftData)` - Make process `Pid` the new owner of
  table `Table`. If successful, message `{'ETS-TRANSFER',Table,FromPid,
  GiftData}` is sent to the new owner... Notice that this function does not
  affect option `heir` of the table."
- "`safe_fixed` - `FixationTime` corresponds to the result returned by
  `erlang:timestamp/0`... the use of `safe_fixed` is not time warp safe. Time
  warp safe code must use `safe_fixed_monotonic_time` instead."
- "A table traversal is safe if either the table is of type `ordered_set`, the
  entire table traversal is done within one ETS function call, [or] function
  `safe_fixtable/2` is used to keep the table fixated during the entire
  traversal."
- "`MatchSpec = [MatchFunction]` / `MatchFunction = {MatchHead, [Guard],
  [Result]}` ... the match specification is always a list of one or more tuples
  (of arity 3)."

## Version notes
- Page version: OTP 29.0.2 / stdlib 8.0.1.
- `lookup_element/4` (Default): since OTP 26.0.
- `next_lookup/2`, `first_lookup/1`, `last_lookup/1`, `prev_lookup/2`:
  since OTP 27.0.
- `update_element/4` (Default): since OTP 27.0.
- `whereis/1`: since OTP 21.0.
- `{decentralized_counters,boolean()}`: since OTP 23.0.
- `{read_concurrency,boolean()}`: since OTP R14B.
- `compressed`: since OTP R14B01.
- `{write_concurrency, auto}`: only OTP-25.0 and above.
- `write_concurrency` had no effect on `ordered_set` prior to stdlib-3.7
  (OTP-22.0).
- `bag` insertion order accidentally reversed OTP 23.0, fixed OTP 25.3;
  `duplicate_bag` same faulty reverse (unpredictable) over the same range.
- Compiled match specs gained an external (node-specific reference)
  representation in STDLIB 3.4 (OTP 20.0).
- No OTP-24-specific quirk is documented on this page (see ordered_set quirks).

## Discovered links
### Relevant (crawl later)
- dets.html — `dets` module (disk-based term storage; `select/1`, `tab_name`)
- dets.html#select/1
- ../../apps/erts/match_spec.html — ERTS "Match Specifications in Erlang"
  (authoritative match_spec reference)
- ms_transform.html — `ms_transform` (parse_transform for `fun2ms`)
- qlc.html — `qlc` (Query List Comprehensions; `info/1`, `query_handle`)
- qlc.html#info/1
- ../../apps/runtime_tools/dbg.html — `dbg` (trace patterns use same match
  spec syntax)
- lists.html#foldl/3 — `lists:foldl/3` (foldl model)
- lists.html#foldr/3 — `lists:foldr/3` (foldr model)
- ../../apps/erts/erlang.html#monotonic_time/0 — `erlang:monotonic_time/0`
  (safe_fixed_monotonic_time basis)
- ../../apps/erts/erlang.html#timestamp/0 — `erlang:timestamp/0`
  (safe_fixed basis; not time-warp safe)
- ../../apps/erts/erlang.html#match_spec_test/3 — `erlang:match_spec_test/3`
- ../../apps/erts/time_correction.html#time-warp-modes — time warp modes
- ../../apps/erts/time_correction.html#time-warp-safe-code — time-warp-safe code
- ../../apps/erts/erl_cmd.html#%2Bdcg — `+dcg` flag (decentralized counters
  cache lines)
- ../../apps/erts/erl_cmd.html#%2Be — `+e` flag (ERL_MAX_ETS_TABLES legacy)
- ../../apps/erts/erl_cmd.html#sched_thread_stack_size — scheduler stack size
  (match_spec nesting system_limit)

### Skipped
- ../../index.html (site index)
- ../../apps/kernel/file.html#t:name/0 (type ref only)
- ../../apps/erts/erlang.html#t:* (primitive type refs: atom/0, boolean/0,
  float/0, function/0, integer/0, list/0, node/0, non_neg_integer/0, pid/0,
  pos_integer/0, string/0, term/0, tuple/0)
- ../../apps/erts/erlang.html#binary_to_term/1 (referenced in compiled_ms note)
- All in-page anchors (`ets.html#...`, `#from_dets/2`, `#match_spec_compile/1`,
  `#to_dets/2`, `#t:...`, etc.)
