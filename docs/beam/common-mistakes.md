# Common BEAM Mistakes

## Purpose
Catalogue the BEAM/Erlang pitfalls enumerated in the Erlang/OTP System
Documentation "Common Caveats" chapter, each with the problem and the
recommended practice. These are performance and correctness traps that recur
across BEAM projects.

## Sources used
- Crawl file: `crawl/34-commoncaveats.md`
- Canonical URL cited: `https://www.erlang.org/doc/system/commoncaveats.html`
- Documented OTP version: Erlang System Documentation v29.0.2 (ExDoc v0.40.3)
- `.crawl/36-binaries.md` — https://www.erlang.org/doc/system/bit_syntax.html (bit syntax construction & matching pitfalls)
- `.crawl/43-binaryhandling.md` — https://www.erlang.org/doc/system/binaryhandling.html (refc/sub-binaries, match contexts, append cost, iolists)
- `.crawl/47-secure-coding.md` — https://www.erlang.org/doc/system/secure_coding.html (secure coding: binary_to_term, atom exhaustion, code injection, distribution/cookie, ports/NIF input validation)

## Core guidance
The Common Caveats chapter lists "a few constructs to watch out for." Each is
a performance or correctness pitfall with a DO/DO-NOT form. The caveats covered
by the crawled source are: the `timer` module, the `++` operator, accidental
copying and loss of sharing, atom creation (`list_to_atom`/`binary_to_atom`),
`length/1`, `setelement/3`, `size/1`, and NIFs.

> Note on scope: The Common Caveats chapter (crawl 34) covers performance
> pitfalls. Binary-handling pitfalls are from the Bit Syntax and Binary Handling
> chapters (crawls 36, 43). Security pitfalls are from the Secure Coding
> guidelines (crawl 47).

## Practical rules

### 1. The `timer` module bottleneck
- **Problem:** "The `timer` module uses a separate process to manage the
  timers." Before OTP 25 the management overhead was substantial and scaled
  poorly. "Still, the timer server remains a single process, and it may at
  some point become a bottleneck of an application."
- **Practice:** "Creating timers using `erlang:send_after/3` and
  `erlang:start_timer/3`, is more efficient than using the timers provided by
  the `timer` module in STDLIB."
- **Exception:** "The functions in the `timer` module that do not manage timers
  (such as `timer:tc/3` or `timer:sleep/1`), do not call the timer-server
  process and are therefore harmless."

### 2. The `++` operator copies its left operand
- **Problem:** "The `++` operator copies its left-hand side operand." Using it
  in a loop where the growing result is the left operand (e.g.
  `naive_reverse(T) ++ [H]`) causes quadratic copying.
- **Practice:** Keep the growing accumulator as the right operand:
  `naive_but_ok_reverse(T, [H] ++ Acc)`. Better: write `[H|Acc]` directly —
  "In practice, the compiler rewrites `[H] ++ Acc` to `[H|Acc]`."

### 3. Accidental copying and loss of sharing
- **Problem:** When spawning a process with a fun (or sending a fun to another
  process) that closes over a record/map, the entire record/map is copied even
  if only one field is used. "When a term is copied to another process, sharing
  of subterms will be lost and the copied term can be many times larger than
  the original term" (example: 32 heap words → 131070 heap words).
- **Practice:** "outside of the fun extract only the fields of the record that
  are actually used" (and likewise extract only the needed map elements with
  `map_get/2` outside the fun) before spawning/sending.

### 4. Atom exhaustion (`list_to_atom/1`, `binary_to_atom/1,2`)
- **Problem:** "Atoms are not garbage-collected. Once an atom is created, it is
  never removed. The emulator terminates if the limit for the number of atoms
  (1,048,576 by default) is reached." Converting untrusted input to atoms is a
  denial-of-service risk. Building an atom dynamically for `apply/3` is "quite
  expensive."
- **Practice:** Use `list_to_existing_atom/1`, `binary_to_existing_atom/1`, or
  `binary_to_existing_atom/2`. "All atoms that are allowed must have been
  created earlier, for example, by using all of them in a module and loading
  that module." Avoid `apply(list_to_atom("some_prefix"++Var), foo, Args)` and
  the `binary_to_atom` equivalents.

### 5. `length/1` is O(n)
- **Problem:** "The time for calculating the length of a list is proportional
  to the length of the list", unlike `tuple_size/1`, `byte_size/1`,
  `bit_size/1` which are O(1).
- **Practice:** Replace `length(L) >= 3` guards with pattern matching:
  `foo([_,_,_|_]=L) -> ...`. (Caveat: `length/1` fails on improper lists
  whereas the pattern accepts them.)

### 6. `setelement/3` copies the tuple
- **Problem:** "`setelement/3` copies the tuple it modifies. Therefore,
  updating a tuple in a loop using `setelement/3` creates a new copy of the
  tuple on each iteration."
- **Practice:** The compiler can coalesce multiple `setelement/3` calls into a
  single copy when: the tuple size is known at compile time; indices are
  integer literals; no intervening expressions; and the result of one call is
  used only in the next. "Since OTP 26 the descending-order requirement was
  dropped." Otherwise prefer records or direct tuple construction.

### 7. `size/1` is too generic
- **Problem:** `size/1` works for both tuples and binaries, giving the
  compiler/runtime/Dialyzer less type information than the specific BIFs.
- **Practice:** "Using the BIFs `tuple_size/1` and `byte_size/1` gives the
  compiler and the runtime system more opportunities for optimization. Another
  advantage is that those BIFs give Dialyzer more type information."

### 8. NIFs — too much or too little work
- **Problem:** "Doing too much work in each NIF call will degrade
  responsiveness of the VM. Doing too little work can mean that the gain of
  the faster processing in the NIF is eaten up by the overhead of calling the
  NIF and checking the arguments."
- **Practice:** "Rewriting Erlang code to a NIF to make it faster should be
  seen as a last resort." Read about Long-running NIFs
  (`erts/erl_nif.html#lengthy_work`) before writing a NIF.

### 9. Prepending data to a binary in a loop copies every time
- **Problem:** "Prepending data to a binary in a loop is not efficient" — when
  the binary to be appended to is NOT the first segment, `Acc` is copied every
  iteration. Example: `rev_list_to_binary([H|T], Acc) -> rev_list_to_binary(T, <<H,Acc/binary>>)`
  copies `Acc` each time.
- **Practice:** The binary to be appended to must be the FIRST segment:
  `<<Acc/binary, H>>`. "the binary to be appended to is always given as the
  first segment." The runtime allocates extra space (2x or 256 bytes, whichever
  is larger) so subsequent appends are cheap.
- **Forced copying:** "only the binary returned from the latest append
  operation will support further cheap append operations." Appending to
  anything other than the latest-append result forces a copy. Sending as a
  message, inserting into ETS, `erlang:port_command/2`, `enif_inspect_binary`
  in a NIF, or matching a binary shrinks it; the next append copies. "If a
  binary is sent as a message to a process or port, the binary will be shrunk
  and any further append operation will copy the binary data into a new
  binary."

### 10. Matching a binary shrinks it (breaks append optimization)
- **Problem:** "Matching a binary will also cause it to shrink and the next
  append operation will copy the binary data" — because a match context holds
  a direct pointer to the binary data.
- **Practice:** Avoid interleaving matching and appending on the same binary in
  a hot loop. If you must match, do all matching first, then build a fresh
  binary.

### 11. Construction can raise badarg (unlike lists/tuples)
- **Problem:** "Unlike when constructing lists or tuples, the construction of a
  binary can fail with a badarg exception." A 17-bit value in a `/binary`
  segment raises badarg (binary defaults to unit:8, requiring whole bytes).
- **Practice:** Validate inputs before binary construction. Use `bitstring`
  (unit:1) for bit-level alignment, `binary` (unit:8) for byte alignment. "For
  clarity, it is recommended not to change the unit size for binaries. Instead,
  use binary when you need byte alignment and bitstring when you need bit
  alignment."

### 12. Size in matching must be a guard expression (not arbitrary expression)
- **Problem:** In matching, `Size` must be a guard expression (literals +
  previously bound variables), NOT an arbitrary expression.
  `foo(N, <<X:N,T/binary>>) -> {X,T}.` is invalid — the size-field `N` is
  unbound.
- **Practice:** Bind the size variable first, then match:
  `foo(N, Bin) -> <<X:N,T/binary>> = Bin, {X,T}.` Exception: a variable may be
  matched/bound and used as size in the SAME pattern:
  `bar(<<Sz:8,Payload:Sz/binary-unit:8,Rest/binary>>) -> {Payload,Rest}.`
  Since OTP 23, size can be a guard expression:
  `bar(<<Sz:8,Payload:((Sz-1)*8)/binary,Rest/binary>>) -> {Payload,Rest}.`

### 13. binary_to_term on untrusted input (DSG-011, CWE-502)
- **Problem:** "Erlang/OTP provides various functionality that serializes and
  deserializes general Erlang terms. Such functionality is intended to be used
  in a trusted environment and is not suitable for communication with untrusted
  entities." `binary_to_term/1` on untrusted data can cause atom exhaustion and
  inject harmful terms (e.g. into a Mnesia table). `file:consult/1` and
  `file:path_consult/2` are also listed as Potentially Unsafe.
- **Practice:** Use `binary_to_term/2` with the `safe` option to prevent new
  atoms. BUT: "`binary_to_term/2` with the `safe` option will prevent new atoms
  from being created. However, note that even if the `safe` option is used and
  the data originates from an untrusted source, it still has to be validated
  and sanitized, since it can still be harmful to the Erlang application in
  other ways. In general, it is best to avoid using such functions altogether
  on untrusted data, even with the `safe` option." Prefer JSON (`json` module)
  or XML (`xmerl_sax_parser`) for untrusted input.

### 14. Atom exhaustion via list_to_atom/binary_to_atom (DSG-003, CWE-770)
- **Problem:** "the `binary_to_atom/1,2` and `list_to_atom/1` functions will
  create a new atom if it does not already exist. Doing so without care might
  cause the system to crash." "The amount of atoms in the system is limited
  (see system limits and CWE-770) and cannot be reclaimed once created."
- **Practice:** Use `binary_to_existing_atom/1,2` and
  `list_to_existing_atom/1` — they throw instead of creating new atoms. BUT:
  "before using the `*_to_existing_atom()` functions, consider whether an
  explicit conversion is more appropriate. Quite often there are only a few
  atoms that are valid in the context the conversion is done, and in those cases
  it is better to translate them yourself and reject invalid inputs, as
  `*_to_existing_atom()` can return any atom that exists in the system, not
  just those expected in the context."
- DO/DO-NOT:
```erlang
%% DO, AND PREFER (see STL-001)
input_to_atom(<<"foo">>) -> foo;
input_to_atom(<<"bar">>) -> bar;
input_to_atom(<<"quux">>) -> quux.

%% DO
input_to_atom(Text) -> binary_to_existing_atom(Text).

%% DO NOT
input_to_atom(Text) -> binary_to_atom(Text).
```

### 15. Code injection via eval/apply on untrusted MFA (CWE-94, MSC-005)
- **Problem:** All loaded code is assumed trusted; there is no built-in sandbox
  for untrusted Erlang code. "A malicious BEAM module can do anything,
  including breaking the memory safety protections of the runtime system and
  crashing the virtual machine." Match specifications (ETS) are vulnerable to
  injection if constructed from untrusted input.
- **Practice:** Never `apply/3` on untrusted module/function/atom names. For
  ETS match specs with untrusted data, wrap in `{const, UntrustedData}`:
```erlang
%% DO
find(Table, Needle) ->
    ets:match(Table, {'_', {const, Needle}, '$1'}).

%% DO NOT
find(Table, Needle) ->
    ets:match(Table, {'_', Needle, '$1'}).
```
- Also: never use undocumented functionality (DSG-004); avoid debug
  functionality in production like `erlang:list_to_pid/1` (MSC-007).

### 16. Distribution/cookie security (DEP-001, CWE-200/668)
- **Problem:** "By default, communication is performed over an unencrypted TCP
  connection with a rudimentary cookie based authentication only present in
  order to prevent mistakes. This configuration should only be used in a trusted
  network." "Once a node is connected to the cluster, it gains complete access
  to the resources and operations of all other nodes, making node
  trustworthiness a critical security consideration." EPMD leaks node info to
  unauthenticated requests.
- **Practice:** "For secure communication over an unsecured network, the
  distribution should be configured to use TLS with client certificate
  verification." Disable default EPMD; use a custom EPMD module with static port
  assignment. All nodes in a cluster MUST be trusted.

### 17. Ports/NIF input validation (MSC-003, CWE-78)
- **Problem:** "Calling `open_port/2` with the `{spawn, _}` argument will either
  start an instance of a driver, or spawn an external program using the passed
  command line. When spawning an external program, some platforms will start a
  shell, which will search in the PATH in order to find the program."
  `os:cmd/1,2` suffers from the same issues.
- **Practice:** Use `open_port/2` with `{spawn_executable, _}` (no shell; args
  explicitly listed via `{args, Args}`; no env var expansion). For drivers, use
  `{spawn_driver, _}`. "Drivers and NIF libraries are typically written in C or
  C++ and have, if not full access, access to a large part of the runtime
  system internals." Prefer Erlang code over native code whenever possible.

## Review checklist
- [ ] Are timers created with `erlang:send_after/3` / `erlang:start_timer/3`
  rather than `timer:send_after/3`?
- [ ] Is `++` avoided in loops with a growing left operand?
- [ ] Are only needed record fields / map elements extracted before
  spawning/sending a fun?
- [ ] Is untrusted input converted with `*_existing_atom` (never
  `list_to_atom`/`binary_to_atom`)?
- [ ] Are `length/1` guards on long lists replaced with patterns?
- [ ] Are `setelement/3` loops replaced with records or coalesced calls?
- [ ] Is `size/1` replaced with `tuple_size/1` / `byte_size/1`?
- [ ] Are NIFs a last resort, with work sized to avoid VM degradation?
- [ ] Is the binary to be appended to always the FIRST segment in `<<>>`
  construction?
- [ ] Is matching and appending on the same binary avoided in hot loops?
- [ ] Are binary construction inputs validated (construction can raise
  `badarg`)?
- [ ] Is `binary` used for byte alignment and `bitstring` for bit alignment
  (unit not changed)?
- [ ] Is `binary_to_term/2` with `safe` used on any data not from a fully
  trusted source?
- [ ] Is untrusted input converted with `*_existing_atom` (or explicit
  conversion), never `list_to_atom`/`binary_to_atom`?
- [ ] Are ETS match specs with untrusted data wrapped in `{const, ...}`?
- [ ] Is `apply/3` never used on untrusted MFA atoms?
- [ ] Is Erlang distribution configured for TLS on untrusted networks?
- [ ] Is `open_port/2` using `{spawn_executable, _}` (not `{spawn, _}`) for
  external programs?
- [ ] Are NIFs/drivers a last resort, with input validation?

## Implementation checklist
- [ ] Audit `timer:` usage; keep only non-timer-managing calls (`timer:tc/3`,
  `timer:sleep/1`).
- [ ] Rewrite list-building loops to grow the accumulator on the right
  (`[H|Acc]`).
- [ ] Extract record fields / map values outside funs passed to `spawn`/`!`.
- [ ] Replace dynamic atom creation with `*_existing_atom` variants.
- [ ] Replace `length(L) >= N` with `[_,...,_|_]` patterns.
- [ ] Replace `size/1` with `tuple_size/1` or `byte_size/1`.
- [ ] Audit `<<>>` construction loops: appended-to binary is the first segment.
- [ ] Use `bin_opt_info` (`erlc +bin_opt_info`) to find non-optimized match
  clauses (not permanent).
- [ ] Replace `binary_to_term/1` with `binary_to_term/2` + `safe` on untrusted
  data.
- [ ] Replace `list_to_atom`/`binary_to_atom` with `*_existing_atom` or explicit
  conversion.
- [ ] Wrap untrusted data in ETS match specs with `{const, ...}`.
- [ ] Replace `open_port/2` with `{spawn, _}` with `{spawn_executable, _}` +
  `{args, ...}`.
- [ ] Configure TLS distribution for untrusted networks; disable default EPMD.

## Runtime / debugging checklist
- [ ] Watch the atom count: `erlang:system_info(atom_count)` /
  `erlang:system_info(atom_limit)` (default 1,048,576).
- [ ] Profile timer-server load if `timer:` timer-managing calls remain.
- [ ] Measure heap growth for processes receiving large shared terms.
- [ ] `erlang:system_info(atom_count)` / `atom_limit` — watch for atom
  exhaustion from untrusted input.
- [ ] `bin_opt_info` compiler output to find non-optimized binary match/append
  clauses.
- [ ] Audit `os:cmd/1,2` usage — prefer `open_port/2` with
  `{spawn_executable, _}`.

## Validation hooks
- Dialyzer type information improves when `size/1` is replaced with
  `tuple_size/1`/`byte_size/1`.
- Compiler coalescing of `setelement/3` (OTP 26+ no longer needs descending
  index order).
- Atom-table limit enforcement (emulator terminates at 1,048,576 by default).
- `bin_opt_info` compiler option flags non-optimized binary clauses.
- Atom-table limit enforcement (emulator terminates at 1,048,576 by default) —
  relevant to DSG-003.
- `binary_to_term/2` with `safe` prevents atom creation from untrusted
  serialized data.

## Examples
```erlang
%% DO NOT: quadratic copying (left operand grows)
naive_reverse([H|T]) -> naive_reverse(T) ++ [H];

%% OK: accumulator on the right
naive_but_ok_reverse([H|T], Acc) -> naive_but_ok_reverse(T, [H] ++ Acc);
%% Better: the compiler rewrites [H] ++ Acc to [H|Acc]
naive_but_ok_reverse([H|T], Acc) -> naive_but_ok_reverse(T, [H|Acc]);

%% DO NOT: convert untrusted input to atoms
apply(list_to_atom("some_prefix" ++ Var), foo, Args);

%% OK: only existing atoms
apply(list_to_existing_atom("some_prefix" ++ Var), foo, Args);

%% DO NOT: length/1 guard on a long list
foo(L) when length(L) >= 3 -> ...

%% OK: pattern match
foo([_,_,_|_]=L) -> ...

%% DO NOT: prepend in a loop (Acc copied every time)
rev_list_to_binary([H|T], Acc) -> rev_list_to_binary(T, <<H,Acc/binary>>);

%% DO: append to first segment
my_list_to_binary([H|T], Acc) -> my_list_to_binary(T, <<Acc/binary,H>>);

%% DO NOT: binary_to_term on untrusted input
Term = binary_to_term(UntrustedBinary);

%% DO: safe option prevents atom creation (still validate/sanitize!)
Term = binary_to_term(UntrustedBinary, [safe]),

%% DO NOT: untrusted data in match spec
find(Table, Needle) -> ets:match(Table, {'_', Needle, '$1'});

%% DO: wrap in {const, ...}
find(Table, Needle) -> ets:match(Table, {'_', {const, Needle}, '$1'}),

%% DO NOT: open_port with {spawn, _} (shell, PATH search, env expansion)
open_port({spawn, "prog " ++ UserInput}, []);

%% DO: spawn_executable with explicit args
open_port({spawn_executable, "prog"}, [{args, [SafeArg1, SafeArg2]}]).
```

## Common mistakes
- Using `timer:send_after/3` under heavy timer load — the single timer-server
  process becomes a bottleneck.
- Writing `Result ++ [Item]` in a hot loop — quadratic.
- Closing over a whole record/map in a spawn fun — copies the entire term and
  loses sharing.
- Calling `list_to_atom/1` on user input — atom table exhaustion crashes the
  VM.
- Using `size/1` — deprives the compiler and Dialyzer of type information.
- Treating NIFs as a default optimization path — they should be a last resort.
- Prepending to a binary in a loop (`<<H,Acc/binary>>`) — copies `Acc` every
  iteration.
- Interleaving matching and appending on the same binary — matching shrinks
  it, next append copies.
- Using `binary_to_term/1` on untrusted input — atom exhaustion and
  harmful-term injection.
- Using `list_to_atom`/`binary_to_atom` on untrusted input — atom table
  exhaustion crashes the VM.
- Building ETS match specs from untrusted input without `{const, ...}` —
  injection.
- Using `open_port/2` with `{spawn, _}` for external programs — shell, PATH
  search, env expansion.
- Exposing default Erlang distribution (unencrypted TCP, cookie-only auth) on
  untrusted networks.

## Strict vs contextual guidance
- **Strict:** Never convert untrusted input to atoms (use `*_existing_atom`).
  Never use `size/1` (use `tuple_size/1`/`byte_size/1`). Avoid `timer:` timer
  management under load. Treat NIFs as a last resort. Never
  `binary_to_term/1` on untrusted data; use `binary_to_term/2` with `safe` (and
  still validate). Never `list_to_atom`/`binary_to_atom` on untrusted input
  (use `*_existing_atom` or explicit conversion). Never `apply/3` on untrusted
  MFA atoms. Never build ETS match specs from untrusted input without
  `{const, ...}`. Never expose default distribution on untrusted networks (use
  TLS + client cert verification). Never use `open_port/2` with `{spawn, _}`
  for external programs (use `{spawn_executable, _}`).
- **Contextual:** `timer:tc/3` and `timer:sleep/1` are harmless (no timer
  server). `++` is fine when the left operand is small/constant. `length/1` is
  fine on short lists or outside hot paths. `bin_opt_info` is a development
  tool, not a permanent Makefile option. `binary_to_term/2` with `safe` still
  requires validation/sanitization of the decoded data.
  `*_to_existing_atom` can return any existing atom, not just contextually
  expected ones.

## Policy decisions for individual repos
- Whether to ban `timer:` timer-managing calls entirely (lint rule).
- Whether to ban `list_to_atom`/`binary_to_atom` in CI (Dialyzer/ELvis).
- Whether NIFs require an architecture review before introduction.
- Atom-table limit tuning (`+t`) per deployment.
- Whether `binary_to_term/2` with `safe` is permitted on untrusted data, or
  banned entirely.
- Whether `list_to_atom`/`binary_to_atom` are banned in CI (Dialyzer/ELvis).
- Whether TLS distribution is mandatory for all production deployments.
- Whether default EPMD is disabled in favor of a custom EPMD module.
- Whether `open_port/2` with `{spawn, _}` is banned via lint rule.
- Whether NIFs/drivers require an architecture review before introduction.

## Related docs
- [timers.md](timers.md)
- [nifs.md](nifs.md)
- [processes-and-messages.md](processes-and-messages.md)
- [ets-data.md](ets-data.md)
- `binaries.md` — bit syntax, binary representations, match contexts, efficient
  building.
- `runtime-environment.md` — `file` module (`file:consult/1` is Potentially
  Unsafe), `os` module (`os:cmd/1,2` caveats).
- `distribution.md` — distribution/cookie security, TLS distribution, EPMD.
- `ets-data.md` — match spec injection (`{const, ...}`), `safe` option.
- `ports-io.md` — `open_port/2` forms, `{spawn_executable, _}` vs
  `{spawn, _}`.
- `nifs.md` — NIF input validation, native code security.

## Related skills
- `beam-processes`
- `beam-observability-debugging`
