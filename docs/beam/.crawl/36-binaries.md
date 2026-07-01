# Crawl: system/binaries.html
- seed_url: https://www.erlang.org/doc/system/binaries.html
- canonical_url: https://www.erlang.org/doc/system/bit_syntax.html
- family: Erlang/OTP system docs
- fetch: 404 (seed URL); 200 (resolved canonical bit_syntax.html)
- otp_version: v29.0.2 (Erlang System Documentation)
- feeds_docs: binaries.md (new)

> NOTE: The seed URL `system/binaries.html` returns HTTP 404 on OTP 29.0.2.
> The "Constructing and Matching Binaries" content lives at `system/bit_syntax.html`
> (page title: "Bit Syntax"). This crawl extracts that canonical page.
> Topics not present here (refc/sub-binaries, contexts, iolists, the `binary` module,
> performance caveats) belong to the Efficiency Guide "Binary Handling" chapter —
> listed under Discovered links for a later crawl.

## Purpose
Provide the complete bit-syntax chapter for constructing and matching binaries and
bitstrings: segment syntax `<<E1,E2,...En>>`, the `Value:Size/TypeSpecifierList`
form, type/signedness/endianness/unit specifiers, defaults, construction rules,
matching rules, and efficient appending to binaries.

## Key concepts (bit syntax construction & matching; segment types/endianness/unit)
- A **Bin** is written `<<E1, E2, ... En>>`. Used to construct binaries
  (`Bin = <<E1,...>>`, all elements bound) or match (`<<E1,...>> = Bin`, Bin bound,
  elements bound/unbound as in any match).
- A Bin does NOT need a whole number of bytes.
- **bitstring** = sequence of 0+ bits, count not necessarily divisible by 8.
  If divisible by 8, the bitstring is also a **binary**.
- Each element specifies a contiguous **segment** of bits (not necessarily byte-aligned).
  First element = initial segment, second = next, etc.
- Segment general syntax: `Value:Size/TypeSpecifierList`. Size and/or
  TypeSpecifierList may be omitted; defaults apply. Allowed variants:
  `Value` | `Value:Size` | `Value/TypeSpecifierList`.
- **Value**: any expression in construction; in matching must be a literal or variable.
- **Size**: `Size * unit` = number of bits for the segment. In construction, any
  expression evaluating to an integer. In matching, must be a constant expression or
  a variable (OTP 23+: a guard expression).
- **TypeSpecifierList**: hyphen-separated list of specifiers:
  - Type: `integer` (default), `float`, `binary`, `bitstring`, plus `utf8`/`utf16`/`utf32`
    (see Reference Manual "Bit Syntax Expressions" for full list).
  - Signedness: `signed` | `unsigned` (default). Signedness only matters for matching.
  - Endianness: `big` (default) | `little` | `native` (resolved at load time to the
    CPU's native endian).
  - Unit: `unit:IntegerLiteral`, allowed range 1-256. Multiplied by Size to give
    effective segment size. Specifies alignment for binary segments without size.
- Example: `X:4/little-signed-integer-unit:8` => total 4*8 = 32 bits, signed integer,
  little-endian.

## Defaults
- Default type = `integer`. Does NOT depend on value, even for literals
  (e.g. default type in `<<3.14>>` is `integer`, not `float`).
- Default Size depends on type: integer=8, float=64, binary=all of the binary.
  In matching, the "all of binary" default is valid ONLY for the last element;
  all other binary elements in matching must have a size specification.
- Default unit depends on type: integer/float/bitstring = 1; binary = 8.
- Default signedness = `unsigned`.
- Default endianness = `big`.

## Constructing Binaries and Bitstrings
- Construction can fail with `badarg` (unlike list/tuple construction).
- `<<>>` constructs a zero-length binary.
- No alignment rules for individual integer/float segments. For binaries/bitstrings
  without size, the unit specifies alignment. Since binary defaults to unit:8, a
  binary segment size must be a multiple of 8 bits (whole bytes only).
- `<<Bin/binary,Bitstring/bitstring>>`: `Bin` must be a whole number of bytes
  (binary defaults unit:8) — 17 bits raises badarg. `Bitstring` may be any number of
  bits (bitstring defaults unit:1).
- Recommendation: do NOT change unit for binaries. Use `binary` for byte alignment,
  `bitstring` for bit alignment.
- `<<X:1,Y:6>>` constructs a 7-bit bitstring (if X,Y integers).
- Value and Size may be any Erlang expression in construction, BUT both must be
  enclosed in parentheses if more than a single literal/variable: `<<X+1:8>>` is a
  syntax error; write `<<(X+1):8>>`.
- Literal strings: `<<"hello">>` is sugar for `<<$h,$e,$l,$l,$o>>`.

## Matching Binaries
- A binary pattern can occur wherever patterns are allowed, including inside other
  patterns. Binary patterns CANNOT be nested. `<<>>` matches a zero-length binary.
- binary segment: size must be evenly divisible by 8 (or by unit if unit changed).
- bitstring segment: no size restrictions.
- float segment: size must be 64 or 32.
- Value in matching must be a variable, integer literal, or float literal.
  Expressions are NOT allowed.
- Size must be a guard expression (literals + previously bound variables).
  `foo(N, <<X:N,T/binary>>) -> {X,T}.` is invalid — the two N are unrelated and the
  size-field N is unbound. Correct:
  ```
  foo(N, Bin) ->
     <<X:N,T/binary>> = Bin,
     {X,T}.
  ```
- Before OTP 23, Size was restricted to an integer or a variable bound to an integer.
- Exception: a variable may be matched/bound and used as size in the SAME pattern:
  ```
  bar(<<Sz:8,Payload:Sz/binary-unit:8,Rest/binary>>) -> {Payload,Rest}.
  ```
- OTP 23+: size can be a guard expression:
  ```
  bar(<<Sz:8,Payload:((Sz-1)*8)/binary,Rest/binary>>) -> {Payload,Rest}.
  ```
- Rest of binary: `foo(<<A:8,Rest/binary>>) ->` — tail size must be divisible by 8.
- Rest of bitstring: `foo(<<A:8,Rest/bitstring>>) ->` — no restriction on tail bits.

## Sub-binaries / refc binaries / contexts
- NOT covered on this page. The bit_syntax chapter addresses construction/matching
  semantics only. Refc binaries, sub-binaries (shared sub-binaries), and binary
  contexts (the `binary:copy/1` context optimization, sub-binary GC behavior) are
  documented in the Efficiency Guide "Binary Handling" chapter. See Discovered links.

## Efficient building (iolists, appending)
- Appending to a binary efficiently (accumulator pattern, grows the binary in place
  via the runtime's binary construction optimization):
  ```
  triples_to_bin(T) ->
      triples_to_bin(T, <<>>).
  triples_to_bin([{X,Y,Z} | T], Acc) ->
      triples_to_bin(T, <<Acc/binary,X:32,Y:32,Z:32>>);
  triples_to_bin([], Acc) ->
      Acc.
  ```
- iolists / iodata vs repeated `<<A/binary,B/binary>>` concatenation, and the
  `binary` module functions, are NOT on this page — covered in the Efficiency Guide
  "Binary Handling" chapter. See Discovered links.

## Strict rules & caveats
- Construction can raise `badarg` (unlike lists/tuples).
- `binary` type defaults to unit:8 => binary segments must be whole-byte aligned;
  a 17-bit value in a `/binary` segment raises badarg.
- In matching, the default "all of binary" size is valid ONLY for the last segment;
  all other binary segments need an explicit size.
- Binary patterns cannot be nested.
- float segments in matching must be size 32 or 64.
- Size in matching must be a guard expression; a size variable must be previously
  bound (the one exception: bind-and-use-same-pattern).
- Lexical note: `B=<<1>>` parses as `B =< <1>>` (syntax error). Write `B = <<1>>`.
- Value/Size expressions in construction need parentheses if not a bare literal/var:
  `<<(X+1):8>>`, not `<<X+1:8>>`.
- Signedness only matters for matching.

## Examples
```
%% Construction from constants / string literal
Bin11 = <<1, 17, 42>>,
Bin12 = <<"abc">>
%% binary_to_list(Bin11) => [1,17,42]; binary_to_list(Bin12) => [97,98,99]

%% Construction with size expression
A = 1, B = 17, C = 42,
Bin2 = <<A, B, C:16>>
%% size 4; binary_to_list(Bin2) => [1,17,00,42]

%% Matching
<<D:16, E, F/binary>> = Bin2
%% D = 273, E = 0, F = <<42>> (binary_to_list(F) => [42])

%% IP datagram header match (IPv4)
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
%% Match fails if: first 4 bits /= 4; HLen < 5; size(Dgram) < 4*HLen.

%% Segment with all specifiers
X:4/little-signed-integer-unit:8   %% 32-bit signed little-endian

%% Bind-and-use size in same pattern
bar(<<Sz:8,Payload:Sz/binary-unit:8,Rest/binary>>) -> {Payload,Rest}.

%% OTP 23+ guard-expression size
bar(<<Sz:8,Payload:((Sz-1)*8)/binary,Rest/binary>>) -> {Payload,Rest}.
```

## Verbatim quotes
- "A Bin is a low-level sequence of bits or bytes."
- "A bitstring is a sequence of zero or more bits, where the number of bits does
  not need to be divisible by 8. If the number of bits is divisible by 8, the
  bitstring is also a binary."
- "Each element specifies a certain segment of the bitstring. A segment is a set of
  contiguous bits of the binary (not necessarily on a byte boundary)."
- "The Size part of the segment multiplied by the unit in TypeSpecifierList ...
  gives the number of bits for the segment. In construction, Size is any expression
  that evaluates to an integer. In matching, Size must be a constant expression or
  a variable."
- "Native-endian means that the endian is resolved at load time, to be either
  big-endian or little-endian, depending on what is 'native' for the CPU."
- "The unit size is given as unit:IntegerLiteral. The allowed range is 1-256."
- "The default type for a segment is integer. The default type does not depend on
  the value, even if the value is a literal. For example, the default type in
  <<3.14>> is integer, not float."
- "For binary it is all of the binary. In matching, this default value is only valid
  for the last element. All other binary elements in matching must have a size
  specification."
- "Unlike when constructing lists or tuples, the construction of a binary can fail
  with a badarg exception."
- "Since the default alignment for the binary type is 8, the size of a binary
  segment must be a multiple of 8 bits, that is, only whole bytes."
- "For clarity, it is recommended not to change the unit size for binaries. Instead,
  use binary when you need byte alignment and bitstring when you need bit alignment."
- "both Value and Size must be enclosed in parentheses if the expression consists of
  anything more than a single literal or a variable."
- "Binary patterns cannot be nested. The pattern <<>> matches a zero length binary."
- "A segment of type float must have size 64 or 32."
- "When matching Value, Value must be either a variable or an integer, or a floating
  point literal. Expressions are not allowed."
- "Size must be a guard expression, which can use literals and previously bound
  variables."
- "Before OTP 23, Size was restricted to be an integer or a variable bound to an
  integer."
- "It is possible to match and bind a variable, and use it as a size within the same
  binary pattern."
- "Starting in OTP 23, the size can be a guard expression."
- "Notice that 'B=<<1>>' will be interpreted as 'B =< <1>>', which is a syntax
  error. The correct way to write the expression is: B = <<1>>."

## Version notes
- OTP version: 29.0.2 (page meta: "Erlang System Documentation v29.0.2").
- OTP 23 change: Size in matching generalized from integer/variable to a guard
  expression.
- Page built with ExDoc v0.40.3. Copyright 1996-2026 Ericsson AB.
- Seed URL `system/binaries.html` is a 404 in OTP 29; canonical page is
  `system/bit_syntax.html`.

## Discovered links
### Relevant (crawl later)
- https://www.erlang.org/doc/system/efficiency_guide.html (Efficiency Guide index;
  leads to "Binary Handling" chapter covering refc/sub-binaries, contexts, iolists,
  the `binary` module, and performance caveats not present on bit_syntax.html)
- https://www.erlang.org/doc/efficiency_guide/binaryhandling.html (confirmed HTTP 200,
  60 KB — the Binary Handling chapter; crawl for refc/sub-binary/context/iolist/perf topics)
- https://www.erlang.org/doc/system/expressions.html#bit-syntax-expressions (full bit
  syntax expression specification referenced by this chapter)
- https://www.erlang.org/doc/system/expressions.html#guard-expressions (guard expr rules
  for Size in matching)
- https://www.erlang.org/doc/apps/erts/erlang.html#binary_to_list/1 (erts erlang module;
  `binary` module functions live under erts too)
- https://www.erlang.org/doc/system/reference_manual.html (Reference Manual intro;
  "complete specification for the bit syntax appears in the Reference Manual")
- https://www.erlang.org/doc/system/list_comprehensions.html (previous page)

### Skipped
- https://www.erlang.org/doc/system/bit_syntax.md (markdown source mirror)
- https://www.erlang.org/doc/system/bit_syntax.html (self / canonical)
- https://www.erlang.org/doc/index.html (site root)
- https://erlang.org (site root)
- in-page anchors: #introduction, #examples, #lexical-note, #segments, #defaults,
  #constructing-binaries-and-bitstrings, #including-literal-strings,
  #matching-binaries, #binding-and-using-a-size-variable,
  #getting-the-rest-of-the-binary-or-bitstring, #appending-to-a-binary
