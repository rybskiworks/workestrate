# Crawl: system/commoncaveats.html
- seed_url: https://www.erlang.org/doc/system/commoncaveats.html
- canonical_url: https://www.erlang.org/doc/system/commoncaveats.html
- family: Erlang/OTP system docs
- fetch: 200
- otp_version: Erlang System Documentation v29.0.2 (ExDoc v0.40.3)
- feeds_docs: common-mistakes.md, timers.md, binary-serialization.md, processes-and-messages.md

## Purpose
This section lists a few constructs to watch out for. It is the canonical
"common caveats" chapter of the Erlang/OTP System Documentation, enumerating
performance and correctness pitfalls (operator `++`, the `timer` module,
accidental copying/loss of sharing, atom creation, `length/1`, `setelement/3`,
`size/1`, and NIFs) with DO/DO NOT examples and recommended practices.

## Caveats (each with: name, problem, recommended practice — verbatim where possible)

### The timer module
- **Problem:** The `timer` module uses a separate single process to manage
  timers. Before Erlang/OTP 25 this management overhead was substantial and
  scaled poorly with the number of (especially short-lived) timers, so the
  timer server could become overloaded and unresponsive. In OTP 25 most of the
  management overhead was removed, but "the timer server remains a single
  process, and it may at some point become a bottleneck of an application."
- **Recommended practice:** "Creating timers using `erlang:send_after/3` and
  `erlang:start_timer/3`, is more efficient than using the timers provided by
  the `timer` module in STDLIB." Note: "The functions in the `timer` module
  that do not manage timers (such as `timer:tc/3` or `timer:sleep/1`), do not
  call the timer-server process and are therefore harmless."

### Operator ++
- **Problem:** "The `++` operator copies its left-hand side operand." Using it
  in a loop where the growing result is the left operand (e.g.
  `naive_reverse(T) ++ [H]`) causes quadratic copying.
- **Recommended practice:** Keep the growing accumulator as the *right-hand*
  operand so each element is copied only once:
  `naive_but_ok_reverse(T, [H] ++ Acc)`. Better still, write `[H|Acc]`
  directly: "In practice, the compiler rewrites `[H] ++ Acc` to `[H|Acc]`."

### Accidental Copying and Loss of Sharing
- **Problem:** When spawning a process with a fun (or sending a fun to another
  process) that closes over a record/map, the *entire* record/map is copied to
  the new process even if only one field is used. Worse, if the term contains
  shared subterms, copying loses sharing and the copied term can be many times
  larger than the original (example: 32 heap words → 131070 heap words after
  loss of sharing).
- **Recommended practice:** "outside of the fun extract only the fields of the
  record that are actually used" (and likewise extract only the needed map
  elements with `map_get/2` outside the fun) before spawning/sending.

### list_to_atom/1, binary_to_atom/1,2 (atom exhaustion)
- **Problem:** "Atoms are not garbage-collected. Once an atom is created, it
  is never removed. The emulator terminates if the limit for the number of
  atoms (1,048,576 by default) is reached." Converting arbitrary untrusted
  input to atoms is therefore a denial-of-service risk. Also, building an atom
  dynamically to pass to `apply/3` is "quite expensive."
- **Recommended practice:** Use `list_to_existing_atom/1`,
  `binary_to_existing_atom/1`, or `binary_to_existing_atom/2` to guard against
  DoS — "All atoms that are allowed must have been created earlier, for
  example, by using all of them in a module and loading that module." Avoid
  `apply(list_to_atom("some_prefix"++Var), foo, Args)` and the
  `binary_to_atom` equivalents.

### length/1
- **Problem:** "The time for calculating the length of a list is proportional
  to the length of the list", unlike `tuple_size/1`, `byte_size/1`,
  `bit_size/1` which are O(1). In time-critical code with potentially very
  long lists this can matter.
- **Recommended practice:** Replace `length(L) >= 3` guards with pattern
  matching: `foo([_,_,_|_]=L) -> ...`. (Caveat: `length/1` fails on improper
  lists whereas the pattern accepts them.)

### setelement/3
- **Problem:** "`setelement/3` copies the tuple it modifies. Therefore,
  updating a tuple in a loop using `setelement/3` creates a new copy of the
  tuple on each iteration."
- **Recommended practice (compiler coalescing):** The compiler can coalesce
  multiple `setelement/3` calls into a single copy when: the tuple size is
  known at compile time; indices are integer literals; no intervening
  expressions; and the result of one call is used only in the next. Since
  OTP 26 the descending-order requirement was dropped. Otherwise prefer
  records or direct tuple construction.

### size/1
- **Problem:** `size/1` works for both tuples and binaries, giving the
  compiler/runtime/Dialyzer less type information than the specific BIFs.
- **Recommended practice:** "Using the BIFs `tuple_size/1` and `byte_size/1`
  gives the compiler and the runtime system more opportunities for
  optimization. Another advantage is that those BIFs give Dialyzer more type
  information."

### Using NIFs
- **Problem:** "Doing too much work in each NIF call will degrade
  responsiveness of the VM. Doing too little work can mean that the gain of
  the faster processing in the NIF is eaten up by the overhead of calling the
  NIF and checking the arguments."
- **Recommended practice:** "Rewriting Erlang code to a NIF to make it faster
  should be seen as a last resort." Read about Long-running NIFs
  (`erts/erl_nif.html#lengthy_work`) before writing a NIF.

## Verbatim quotes
- "Creating timers using `erlang:send_after/3` and `erlang:start_timer/3`, is
  more efficient than using the timers provided by the `timer` module in
  STDLIB."
- "The timer module uses a separate process to manage the timers."
- "Still, the timer server remains a single process, and it may at some point
  become a bottleneck of an application."
- "The functions in the `timer` module that do not manage timers (such as
  `timer:tc/3` or `timer:sleep/1`), do not call the timer-server process and
  are therefore harmless."
- "The `++` operator copies its left-hand side operand."
- "Atoms are not garbage-collected. Once an atom is created, it is never
  removed. The emulator terminates if the limit for the number of atoms
  (1,048,576 by default) is reached."
- "When a term is copied to another process, sharing of subterms will be lost
  and the copied term can be many times larger than the original term."
- "`setelement/3` copies the tuple it modifies."
- "Rewriting Erlang code to a NIF to make it faster should be seen as a last
  resort."

## Version notes
- Page title: "Common Caveats — Erlang System Documentation v29.0.2".
- Built with ExDoc v0.40.3. Copyright © 1996-2026 Ericsson AB.
- OTP 25: `timer` module improved by removing most timer-management overhead.
- OTP 26: `setelement/3` coalescing no longer requires descending index order
  (pre-26 it did).
- Default atom table limit: 1,048,576 atoms.

## Discovered links

### Relevant (crawl later)
- ../apps/stdlib/timer.html — STDLIB `timer` module reference (feeds timers.md)
- ../apps/stdlib/gen_server.html — `gen_server` behaviour reference
- ../apps/erts/erl_nif.html#lengthy_work — Long-running NIFs guidance
- ../apps/erts/erl_nif.html#WARNING — NIF warnings
- Next chapter: "Constructing and Matching Binaries" (system/binaries.html) —
  feeds binary-serialization.md
- Previous chapter: "Introduction" (system/introduction.html)

### Skipped
- ../apps/erts/erlang.html#* (BIF anchors: apply/3, binary_to_atom/1,2,
  binary_to_existing_atom/1,2, bit_size/1, byte_size/1, length/1,
  list_to_atom/1, list_to_existing_atom/1, send_after/3, setelement/3,
  size/1, spawn/1, start_timer/3, tuple_size/1) — covered by ref-man crawl 06
- ../apps/stdlib/timer.html#sleep/1, #tc/3 — sub-anchors of timer.html above
- ../index.html, https://www.erlang.org/doc/system/commoncaveats.html (self)
- In-page fragment anchors (#operator, #timer-module, etc.)
