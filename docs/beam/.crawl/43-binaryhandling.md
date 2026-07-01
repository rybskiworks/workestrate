# Crawl: efficiency_guide/binaryhandling.html
- seed_url: https://www.erlang.org/doc/efficiency_guide/binaryhandling.html
- canonical_url: https://www.erlang.org/doc/system/binaryhandling.html
- family: Erlang/OTP efficiency guide
- fetch: 200
- otp_version: OTP 29.0.2 (Erlang System Documentation v29.0.2)
- feeds_docs: binaries.md (new) or common-mistakes.md

## Purpose
Gives examples of efficient binary handling and an in-depth look at how binaries
are implemented internally, so users can take advantage of compiler/runtime
optimizations for constructing and matching binaries.

## Binary representations (refc / heap / sub-binaries)
Internally, binaries and bitstrings are implemented the same way. Four types of
binary objects exist internally:

- **Containers** for binary data:
  - **Refc binaries** (reference-counted): two parts — a `ProcBin` object stored
    on the process heap, and the binary object itself stored outside all process
    heaps. The binary object has a reference counter; any number of ProcBins from
    any number of processes can reference it. All ProcBins in a process are on a
    linked list so the GC can decrement refcounters when a ProcBin disappears.
  - **Heap binaries**: small binaries, **up to 64 bytes**, stored directly on the
    process heap. Copied on GC and when sent as a message. No special GC handling.
- **References** to part of a binary:
  - **Sub binaries**: created by `split_binary/2` and when a binary is matched
    out in a binary pattern. A sub binary references part of another binary (refc
    or heap, **never another sub binary**). Matching out a binary is cheap
    because data is never copied.
  - **Match contexts**: see next section.

## Match contexts
A **match context** is similar to a sub binary but optimized for binary matching;
it contains a direct pointer to the binary data. For each field matched out, the
position in the match context is incremented.

The compiler avoids generating code that creates a sub binary only to shortly
afterwards create a new match context and discard the sub binary — instead the
match context is kept. This optimization is only applied when the compiler knows
the match context will not be shared (otherwise referential transparency would
break).

In `my_binary_to_list/1` (tail-recursive match over `<<H,T/binary>>`), only one
match context and no sub binaries are created. If iteration stops before the end
(e.g. `after_zero/1`), the compiler still reuses the match context in the
recursive clause but builds a sub binary in the clause that returns `T`.

## Appending cost + iolist/iodata building
Appending to a binary is specially optimized to avoid copying:

```erlang
<<Binary/binary, ...>>
%% - OR -
<<Binary/bitstring, ...>>
```

The binary to be appended to must be the **first segment** for the optimization
to apply. The runtime allocates extra space (either 2x the size or 256 bytes,
whichever is larger) so subsequent appends are cheap until space runs out.

**Appending (DO):**
```erlang
my_list_to_binary([H|T], Acc) ->
    my_list_to_binary(T, <<Acc/binary,H>>);
my_list_to_binary([], Acc) ->
    Acc.
```
Efficient — runtime avoids copying `Acc` each iteration.

**Prepending (DO NOT):**
```erlang
rev_list_to_binary([H|T], Acc) ->
    rev_list_to_binary(T, <<H,Acc/binary>>);
```
Not efficient for long lists — `Acc` is copied every time. Fix by reversing the
list first, or by building the tail recursively and appending the head last:
```erlang
rev_list_to_binary([H|T]) ->
    RevTail = rev_list_to_binary(T),
    <<RevTail/binary,H>>;
rev_list_to_binary([]) ->
    <<>>.
```

**Circumstances that force copying** (the append optimization requires a single
ProcBin and a single reference to it):
- Appending to anything other than the result of the latest append forces a copy.
- Sending the binary as a message to a process/port shrinks it; next append copies.
- Inserting into an ETS table, `erlang:port_command/2`, or `enif_inspect_binary`
  in a NIF has the same effect.
- Matching a binary (`<<X,Y,Z,T/binary>> = Bin1`) shrinks it; next append copies,
  because a match context holds a direct pointer to the binary data.
- GC can shrink binaries kept in loop data / process dictionary; appending to a
  shrunk binary reallocates the object.

**Compiler support (OTP 26+):** when the compiler can determine no copy
situations need handling and the append cannot fail, it emits a more efficient
variant — e.g. rewriting `<<>>` creation to a refc binary with 256 bytes
pre-reserved (see `repack/2` example).

> NOTE: This chapter does **not** cover iolists/`iodata` or the `binary` module
> for matching. Those topics are absent from this page (see Discovered links).

## binary module matching
Not covered on this page. The chapter focuses on the compiler/runtime
optimizations for `<<>>` construction and pattern matching, not on the `binary`
module API. (No mention of `binary:match/2,3`, `binary:matches/2,3`,
`binary:split/2,3`, `binary:copy/2`, etc.)

## Practical perf rules (verbatim where possible)
- "the binary to be appended to is always given as the first segment."
- "Prepending data to a binary in a loop is not efficient" — `Acc` is copied
  every time.
- "only the binary returned from the latest append operation will support
  further cheap append operations" — appending to `Bin0` forces copy.
- "If a binary is sent as a message to a process or port, the binary will be
  shrunk and any further append operation will copy the binary data into a new
  binary."
- "The same happens if you insert a binary into an Ets table, send it to a port
  using `erlang:port_command/2`, or pass it to enif_inspect_binary in a NIF."
- "Matching a binary will also cause it to shrink and the next append operation
  will copy the binary data" — because a match context holds a direct pointer.
- "If a process simply keeps binaries ... the garbage collector can eventually
  shrink the binaries. If only one such binary is kept, it will not be shrunk."
- Use `bin_opt_info` (`erlc +bin_opt_info Mod.erl` or
  `export ERL_COMPILER_OPTIONS=bin_opt_info`) to print which clauses are
  `OPTIMIZED: match context reused` vs `NOT OPTIMIZED: binary is returned from
  the function`. Not meant as a permanent Makefile option.
- Unused variables in a match are skipped, not matched out — `count1/2`,
  `count2/2`, `count3/2` generate identical code.

## Verbatim quotes
- "Binaries can be efficiently built in the following way: ... `<<Acc/binary,H>>`"
- "Appending data to a binary as in the example is efficient because it is
  specially optimized by the runtime system to avoid copying the `Acc` binary
  every time."
- "This is not efficient for long lists because the `Acc` binary is copied every
  time."
- "Note that in each of the DO examples, the binary to be appended to is always
  given as the first segment."
- "Heap binaries are small binaries, up to 64 bytes, and are stored directly on
  the process heap."
- "A sub binary is a reference into a part of another binary (refc or heap
  binary, but never into another sub binary). Therefore, matching out a binary is
  relatively cheap because the actual binary data is never copied."
- "A match context is similar to a sub binary, but is optimized for binary
  matching. For example, it contains a direct pointer to the binary data."
- "The compiler can only do this optimization if it knows that the match context
  will not be shared. If it would be shared, the functional properties (also
  called referential transparency) of Erlang would break."
- "The optimization of the binary append operation requires that there is a
  single ProcBin and a single reference to the ProcBin for the binary."
- "The reason is that a match context contains a direct pointer to the binary
  data."
- "Notice that the `bin_opt_info` is not meant to be a permanent option added to
  your `Makefile`s, because all messages that it generates cannot be eliminated."

## Version notes
- Page title: "Constructing and Matching Binaries — Erlang System Documentation
  v29.0.2".
- "In Erlang/OTP 27, the handling of binaries and bitstrings was rewritten. To
  fully leverage those changes in the run-time system, the compiler needs to be
  updated, which is planned for a future release. Since, practically speaking,
  not much has changed from an efficiency and optimization perspective, the
  following description has not yet been updated to describe the implementation
  in Erlang/OTP 27."
- "The compiler support for making the optimization more efficient was added in
  Erlang/OTP 26." (stated twice)
- Source: https://github.com/erlang/otp/blob/OTP-29.0.2/system/doc/efficiency_guide/binaryhandling.md#L1

## Discovered links

### Relevant (crawl later)
- commoncaveats.html — Common Caveats (prev page in efficiency guide)
- maps.html — Maps (next page in efficiency guide)
- ../apps/erts/erl_nif.html#enif_inspect_binary — NIF binary inspection (erts)

### Skipped
- binaryhandling.html#forced_copying, #heap_binary, #refc_binary, #match_context,
  #sub_binary — same-page anchors
- llms.txt, .epub download, ex_doc, erlang.org, ericsson.com — tooling/branding
