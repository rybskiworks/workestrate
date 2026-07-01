# Binaries (Bit Syntax, Representations, Match Contexts, Efficient Building)

## Purpose

Covers bit syntax construction & matching (segment types/endianness/unit), binary
representations (refc/heap/sub-binaries), match contexts, and efficient building
(append optimization vs repeated prepend). Grounded in the Bit Syntax chapter and
the Efficiency Guide "Binary Handling" chapter. The `binary` module API and
iolists/iodata are noted as out-of-scope (see Sources used).

## Sources used

- `.crawl/36-binaries.md` — https://www.erlang.org/doc/system/bit_syntax.html
  (PRIMARY — bit syntax construction & matching, segment syntax, types,
  endianness, unit, defaults, construction rules, matching rules, appending).
- `.crawl/43-binaryhandling.md` — https://www.erlang.org/doc/system/binaryhandling.html
  (PRIMARY — binary representations: refc/heap/sub-binaries, match contexts, append
  optimization, forced-copy circumstances, `bin_opt_info`).

> NOTE: The Bit Syntax chapter (crawl 36) addresses construction/matching semantics
> only. Refc/sub-binaries, match contexts, and append optimization are from the
> Efficiency Guide Binary Handling chapter (crawl 43). iolists/iodata and the
> `binary` module API (`binary:match/2,3`, `binary:split/2,3`, `binary:copy/2`,
> etc.) are NOT covered on either crawled page — those topics are absent from the
> crawled sources. See Related docs for where to look.

## Core guidance

### Bit syntax construction & matching

A **Bin** is `<<E1, E2, ... En>>`. "A Bin is a low-level sequence of bits or
bytes." "A bitstring is a sequence of zero or more bits, where the number of bits
does not need to be divisible by 8. If the number of bits is divisible by 8, the
bitstring is also a binary." "Each element specifies a certain segment of the
bitstring. A segment is a set of contiguous bits of the binary (not necessarily on
a byte boundary)."

Segment syntax: `Value:Size/TypeSpecifierList`. Variants: `Value` |
`Value:Size` | `Value/TypeSpecifierList`. TypeSpecifierList (hyphen-separated):
Type (`integer` default, `float`, `binary`, `bitstring`, `utf8`/`utf16`/`utf32`);
Signedness (`signed`|`unsigned` default); Endianness (`big` default | `little` |
`native`); Unit (`unit:IntegerLiteral`, range 1-256).

"The Size part of the segment multiplied by the unit in TypeSpecifierList ... gives
the number of bits for the segment. In construction, Size is any expression that
evaluates to an integer. In matching, Size must be a constant expression or a
variable." "Native-endian means that the endian is resolved at load time, to be
either big-endian or little-endian, depending on what is 'native' for the CPU."
"The unit size is given as unit:IntegerLiteral. The allowed range is 1-256."

### Defaults

"The default type for a segment is integer. The default type does not depend on the
value, even if the value is a literal. For example, the default type in <<3.14>> is
integer, not float." Default Size: integer=8, float=64, binary=all of the binary.
"For binary it is all of the binary. In matching, this default value is only valid
for the last element. All other binary elements in matching must have a size
specification." Default unit: integer/float/bitstring=1; binary=8. Default
signedness=unsigned; default endianness=big.

### Construction rules

"Unlike when constructing lists or tuples, the construction of a binary can fail
with a badarg exception." `<<>>` constructs a zero-length binary. "Since the
default alignment for the binary type is 8, the size of a binary segment must be a
multiple of 8 bits, that is, only whole bytes." "For clarity, it is recommended
not to change the unit size for binaries. Instead, use binary when you need byte
alignment and bitstring when you need bit alignment." "both Value and Size must be
enclosed in parentheses if the expression consists of anything more than a single
literal or a variable." (`<<(X+1):8>>`, not `<<X+1:8>>`) Literal strings:
`<<"hello">>` is sugar for `<<$h,$e,$l,$l,$o>>`.

### Matching rules

"Binary patterns cannot be nested. The pattern <<>> matches a zero length binary."
"A segment of type float must have size 64 or 32." "When matching Value, Value
must be either a variable or an integer, or a floating point literal. Expressions
are not allowed." "Size must be a guard expression, which can use literals and
previously bound variables." "Before OTP 23, Size was restricted to be an integer
or a variable bound to an integer." "Starting in OTP 23, the size can be a guard
expression." "It is possible to match and bind a variable, and use it as a size
within the same binary pattern." Lexical note: "Notice that 'B=<<1>>' will be
interpreted as 'B =< <1>>', which is a syntax error. The correct way to write the
expression is: B = <<1>>."

### Binary representations (refc / heap / sub-binaries)

Four internal binary object types:

- **Refc binaries** (reference-counted): a `ProcBin` on the process heap + the
  binary object outside all process heaps. Reference counter; any number of
  ProcBins from any process can reference it. All ProcBins in a process are on a
  linked list for GC.
- **Heap binaries**: "Heap binaries are small binaries, up to 64 bytes, and are
  stored directly on the process heap." Copied on GC and when sent as a message.
- **Sub binaries**: "A sub binary is a reference into a part of another binary
  (refc or heap binary, but never into another sub binary). Therefore, matching
  out a binary is relatively cheap because the actual binary data is never copied."
  Created by `split_binary/2` and by binary pattern matching.
- **Match contexts**: "A match context is similar to a sub binary, but is optimized
  for binary matching. For example, it contains a direct pointer to the binary
  data." For each field matched out, the position is incremented. "The compiler can
  only do this optimization if it knows that the match context will not be shared.
  If it would be shared, the functional properties (also called referential
  transparency) of Erlang would break."

### Efficient building (append optimization)

"Appending data to a binary as in the example is efficient because it is specially
optimized by the runtime system to avoid copying the `Acc` binary every time."
Pattern: `<<Acc/binary, H>>`. "Note that in each of the DO examples, the binary to
be appended to is always given as the first segment." The runtime allocates extra
space (2x the size or 256 bytes, whichever is larger) for cheap subsequent
appends. Prepending (`<<H, Acc/binary>>`) is NOT efficient: "This is not efficient
for long lists because the `Acc` binary is copied every time."

"The optimization of the binary append operation requires that there is a single
ProcBin and a single reference to the ProcBin for the binary." Forced-copy
circumstances: "only the binary returned from the latest append operation will
support further cheap append operations." Sending as message, inserting into ETS,
`erlang:port_command/2`, `enif_inspect_binary` in a NIF, or matching a binary
shrinks it; next append copies. "If a binary is sent as a message to a process or
port, the binary will be shrunk and any further append operation will copy the
binary data into a new binary." "Matching a binary will also cause it to shrink and
the next append operation will copy the binary data" — "The reason is that a match
context contains a direct pointer to the binary data."

GC can shrink binaries kept in loop data/process dictionary; "If only one such
binary is kept, it will not be shrunk." Compiler support (OTP 26+): more efficient
variant when compiler can determine no-copy situations. OTP 27 note: "In Erlang/OTP
27, the handling of binaries and bitstrings was rewritten. To fully leverage those
changes in the run-time system, the compiler needs to be updated, which is planned
for a future release."

### `binary` module

> NOTE: "This chapter does not cover iolists/iodata or the `binary` module for
> matching. Those topics are absent from this page." The `binary` module API
> (`binary:match/2,3`, `binary:matches/2,3`, `binary:split/2,3`, `binary:copy/2`,
> etc.) is NOT on either crawled page.

## Practical rules

- The binary to be appended to MUST be the first segment: `<<Acc/binary, H>>`, not
  `<<H, Acc/binary>>`.
- Avoid interleaving matching and appending on the same binary in a hot loop
  (matching shrinks it).
- Use `binary` for byte alignment (unit:8), `bitstring` for bit alignment (unit:1);
  do not change unit for binaries.
- In matching, all binary segments except the last need an explicit size.
- Size in matching must be a guard expression (OTP 23+); bind-and-use-same-pattern
  is the one exception.
- Construction can raise `badarg` — validate inputs.
- Enclose Value/Size in parentheses if more than a single literal/variable:
  `<<(X+1):8>>`.
- Float segments in matching must be size 32 or 64.
- Binary patterns cannot be nested.
- Write `B = <<1>>` not `B=<<1>>` (lexical pitfall).

## Review checklist

- [ ] Is the appended-to binary always the first segment in `<<>>` construction?
- [ ] Is matching and appending on the same binary avoided in hot loops?
- [ ] Is `binary` used for byte alignment and `bitstring` for bit alignment?
- [ ] Do all non-last binary segments in matching have explicit sizes?
- [ ] Are Value/Size expressions parenthesized when not bare literals/variables?
- [ ] Are float segments in matching size 32 or 64?
- [ ] Is `bin_opt_info` used during development to find non-optimized clauses?

## Implementation checklist

- [ ] Use `<<Acc/binary, H>>` (first segment) for accumulator-style binary building.
- [ ] Reverse the list first if you must prepend, then append in order.
- [ ] Use `bin_opt_info` (`erlc +bin_opt_info` or
      `export ERL_COMPILER_OPTIONS=bin_opt_info`) during development — NOT permanent.
- [ ] Bind size variables before matching if they are used as segment sizes.
- [ ] Use `bitstring` type for sub-byte segments; `binary` for whole-byte segments.

## Runtime / debugging checklist

- [ ] `bin_opt_info` compiler output: look for `OPTIMIZED: match context reused`
      vs `NOT OPTIMIZED: binary is returned from the function`.
- [ ] Profile binary-building loops for unexpected copying
      (matching/sending/ETS insertion shrinks binaries).
- [ ] Check that binaries kept in loop data are not being shrunk by GC (keep only
      one reference if possible).

## Validation hooks

- `bin_opt_info` flags non-optimized match/append clauses (development tool, not
  permanent).
- Construction raising `badarg` on invalid input is a built-in validation gate.
- Match failure (no clause matches) on malformed binary input is a built-in
  validation gate.

## Examples

```erlang
%% Construction from constants / string literal
Bin11 = <<1, 17, 42>>,           %% binary_to_list => [1,17,42]
Bin12 = <<"abc">>                %% binary_to_list => [97,98,99]

%% Construction with size expression (size 4 => [1,17,00,42])
A = 1, B = 17, C = 42,
Bin2 = <<A, B, C:16>>

%% Matching (D = 273, E = 0, F = <<42>>)
<<D:16, E, F/binary>> = Bin2

%% IP datagram header match (IPv4) — fails if first 4 bits /= 4, HLen < 5,
%% or size(Dgram) < 4*HLen.
-define(IP_VERSION, 4).
-define(IP_MIN_HDR_LEN, 5).
DgramSize = byte_size(Dgram),
case Dgram of
    <<?IP_VERSION:4, HLen:4, SrvcType:8, TotLen:16,
      ID:16, Flgs:3, FragOff:13,
      TTL:8, Proto:8, HdrChkSum:16,
      SrcIP:32,
      DestIP:32, RestDgram/binary>> when HLen>=5, 4*HLen=<DgramSize ->
        OptsLen = 4*(HLen - ?IP_MIN_HDR_LEN),
        <<Opts:OptsLen/binary,Data/binary>> = RestDgram,
    ...
end.

%% Segment with all specifiers (32-bit signed little-endian)
X:4/little-signed-integer-unit:8

%% Bind-and-use size in same pattern
bar(<<Sz:8,Payload:Sz/binary-unit:8,Rest/binary>>) -> {Payload,Rest}.

%% OTP 23+ guard-expression size
bar(<<Sz:8,Payload:((Sz-1)*8)/binary,Rest/binary>>) -> {Payload,Rest}.

%% Efficient append (DO)
my_list_to_binary([H|T], Acc) -> my_list_to_binary(T, <<Acc/binary,H>>);
my_list_to_binary([], Acc) -> Acc.

%% Inefficient prepend (DO NOT)
rev_list_to_binary([H|T], Acc) -> rev_list_to_binary(T, <<H,Acc/binary>>);

%% Fix for prepend: reverse first or build tail recursively
rev_list_to_binary([H|T]) -> RevTail = rev_list_to_binary(T), <<RevTail/binary,H>>;
rev_list_to_binary([]) -> <<>>.
```

## Common mistakes

- Prepending to a binary in a loop (`<<H,Acc/binary>>`) — copies `Acc` every
  iteration.
- Interleaving matching and appending on the same binary — matching shrinks it.
- Forgetting that construction can raise `badarg` (unlike lists/tuples).
- Using a 17-bit value in a `/binary` segment (requires whole bytes).
- Omitting size on non-last binary segments in matching.
- Using an unbound variable as size in matching without bind-and-use-same-pattern.
- Writing `B=<<1>>` (parses as `B =< <1>>`, syntax error).
- Forgetting parentheses around expressions: `<<X+1:8>>` (syntax error; use
  `<<(X+1):8>>`).
- Treating `bin_opt_info` as a permanent Makefile option.

## Strict vs contextual guidance

Strict:

- The appended-to binary MUST be the first segment for the append optimization.
- Construction can raise `badarg` — always handle or validate.
- Binary patterns cannot be nested.
- Float segments in matching must be size 32 or 64.
- Size in matching must be a guard expression.

Contextual:

- `bin_opt_info` is a development tool, not a permanent option.
- Whether to reverse-then-append or build-tail-recursively for prepend workloads.
- `native` endianness vs explicit `big`/`little` (portability vs performance).

## Policy decisions for individual repos

- Whether `bin_opt_info` is run in CI as a non-blocking check.
- Whether prepend-style binary building is banned via lint.
- Default endianness convention (`big` vs `native`) for protocol parsing.
- Whether iolist/iodata is preferred over binary concatenation for output (NOTE:
  iolists are not covered in the crawled sources — this is a repo-level convention).

## Related docs

- `common-mistakes.md` — binary append/prepend pitfalls, construction badarg.
- `ports-io.md` — iodata/iolist for port communication (NOTE: iolists not covered in
  crawled sources).
- `ets-data.md` — inserting binaries into ETS shrinks them (breaks append
  optimization).
- `nifs.md` — `enif_inspect_binary` shrinks binaries (breaks append optimization).
- `runtime-environment.md` — `file:read_file/1` returns binaries; `iodata()` for
  `file:write/2`.

## Related skills

- `beam-processes`
