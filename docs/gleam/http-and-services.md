# HTTP and services

## Purpose

gleam_http (v4.3.0) provides HTTP types and functions for HTTP clients and servers in
Gleam. It defines core data structures (`Request`, `Response`, `Headers`, `Method`,
`Status`) and a sans-IO service abstraction that lets a single Gleam function serve as
both an HTTP server handler and an HTTP client target. The package itself ships NO I/O —
concrete networking is provided by external adapter packages (server adapters and client
adapters).

## Sources used

- Crawl 35-gleam-http-index.md — hexdocs gleam_http index — https://gleam-http.hexdocs.pm/
- Crawl 32-gleam-json.md — hexdocs gleam/json module — https://gleam-json.hexdocs.pm/gleam/json.html

## Related BEAM guidance

gleam_http is Gleam-specific; the sans-IO `Service` abstraction has no direct BEAM-stdlib
equivalent. Adapter runtimes map to BEAM concepts: `gleam_httpc` uses Erlang/OTP's `inets`
`httpc`; `gleam_hackney` uses Hackney; Mist is pure Gleam on the BEAM. Cross-ref
`docs/beam/ports-io.md` (I/O) and `docs/beam/applications.md` (OTP app lifecycle for
servers). These are conceptual parallels, not identical APIs.

## Core guidance

### Modules (index-level — crawl 35 fetched only the index page)

- `gleam/http` — core HTTP types: `Method`, `Status`, `Scheme`, `Headers`, and header helpers.
- `gleam/http/cookie` — cookie parsing/serialization helpers for `Set-Cookie` and `Cookie` headers.
- `gleam/http/request` — `Request` type and constructors/accessors (method, headers, body, scheme, host, path, query).
- `gleam/http/response` — `Response` type and constructors/accessors (status, headers, body) plus helpers.
- `gleam/http/service` — the sans-IO `Service(conn, body)` type and the request→response service contract.

Per-module accessor signatures are index-level here; consult hexdocs for full signatures.

### Sans-IO Service abstraction

- `gleam/http/service` defines `Service(conn, body)`: a function
  `fn(Request(body)) -> Response(body)` parameterized by a connection type `conn` and a
  body type `body`.
- A service is a PURE function from `Request` to `Response` and performs NO I/O itself.
- This makes services target-agnostic: the same service function can be handed to any
  server adapter (to handle inbound requests) OR invoked by any client adapter (to produce
  a response from an outbound request).
- Adapters supply the concrete `conn`/`body` types and the actual socket/transport
  plumbing; the service logic stays pure and reusable across targets (Erlang/JavaScript,
  server/client).

### Adapter ecosystem (per the gleam_http index page)

Server adapters (run a gleam_http `Service` as a server):
- Mist — high performance pure Gleam HTTP/1.1 server.
- cgi — Common Gateway Interface adapter.
- gleam_cowboy — Cowboy (Erlang HTTP/2 & HTTP/1.1) adapter.
- gleam_elli — Elli (Erlang HTTP/1.1) adapter.
- gleam_plug — Plug (Elixir web application interface) adapter.

Client adapters (send HTTP requests over the network):
- gleam_fetch — JavaScript fetch API client.
- gleam_hackney — Hackney client for Erlang.
- gleam_httpc — httpc client bundled with Erlang/OTP.

IMPORTANT: wisp is NOT listed on the gleam_http index page itself; wisp is a separate web
framework built on top of Mist, part of the broader Gleam HTTP ecosystem rather than a
direct adapter enumerated here.

### JSON/HTTP integration (from crawl 32, gleam_json v3.1.0)

- Encode responses with gleam_json: build `Json` via builders, serialise with
  `to_string_tree` (preferred — BEAM VM IO optimised for `StringTree`) or `to_string`.
- Decode request bodies with `json.parse(from: body_string, using: decoder)` or
  `json.parse_bits(from: body_bitarray, using: decoder)` — returns
  `Result(t, DecodeError)`. Always handle `Error` (untrusted input).
- The parse→dynamic→decode→typed pipeline applies at the HTTP body boundary.

## Practical rules

- Write handlers as pure `Service` functions (`Request`→`Response`); keep I/O in adapters.
- Prefer `to_string_tree` for JSON response bodies (BEAM IO optimised).
- Always decode + handle `Error` on inbound JSON bodies (untrusted).
- Pick the adapter by target: Erlang server → Mist/cowboy/elli/plug; JS client → gleam_fetch; Erlang client → gleam_hackney/gleam_httpc.
- A service is reusable across server and client roles — don't bake transport into handler logic.

## Review checklist

- Handler is a pure `fn(Request(body)) -> Response(body)` with no socket/file I/O inside.
- Inbound JSON bodies are decoded with `parse`/`parse_bits` and the `Error` variant is handled.
- Response JSON bodies use `to_string_tree`, not `to_string`, where IO matters.
- Adapter choice matches the target runtime (Erlang vs JavaScript, server vs client).
- No transport/adapter-specific coupling inside the service function.
- wisp is not assumed to be a gleam_http adapter.

## Implementation checklist

- Choose body type (`String` vs `BitArray`) up front; pick `parse` vs `parse_bits` accordingly.
- Build the `Service` function against `Request(body)`/`Response(body)`.
- For JSON responses: construct `Json` via builders, then `to_string_tree`.
- For JSON requests: define a `decode.Decoder(t)` and call `parse`/`parse_bits`.
- Select the adapter package for the deployment target; wire the service into it.
- Keep adapter-specific code at the edges (the adapter call site), not in the service.

## Validation hooks

- Compile: `gleam build` (type-checks the `Service` contract and body types).
- Tests: `gleam test` — exercise the pure service directly with constructed
  `Request`/`Response` values (no adapter needed).
- JSON round-trip: assert `parse(to_string(json), decoder) == Ok(value)` for known shapes.
- Adapter integration: defer to the adapter package's own run/test instructions (out of
  scope for gleam_http itself).

## Examples

Illustrative Gleam syntax; the crawl is index-level, so these show the `Service` contract,
not per-module accessor signatures.

```gleam
import gleam/http/request.{type Request}
import gleam/http/response.{type Response}
import gleam/json
import gleam/dynamic/decode

// A pure service: Request(String) -> Response(String). No I/O here.
pub fn handler(req: Request(String)) -> Response(String) {
  let body = json.to_string_tree(json.object([
    #("path", json.string(req.path)),
  ]))
  response.new(200) |> response.set_body(body)
}

// Decoding an inbound JSON body (untrusted — handle Error).
pub fn parse_ints(body: String) -> Result(List(Int), json.DecodeError) {
  json.parse(from: body, using: decode.list(of: decode.int))
}
```

## Common mistakes

- Putting I/O/socket logic inside a `Service` function (breaks sans-IO purity / reusability).
- Assuming wisp is a gleam_http adapter (it's a framework on top of Mist, not enumerated on the index).
- Using `to_string` instead of `to_string_tree` for response bodies (slower; BEAM IO prefers `StringTree`).
- Not handling `parse` `Error` on inbound JSON (untrusted).
- Coupling handler logic to a specific adapter/transport.

## Strict vs contextual guidance

Strict:
- Services are pure (no I/O).
- Handle JSON `parse` `Error`.
- Use `to_string_tree` for IO-bound JSON bodies.

Contextual:
- Adapter choice (target/workload dependent).
- Body type (`String` vs `BitArray` → `parse` vs `parse_bits`).

## Policy decisions for individual repos

- Which server adapter to standardise on (Mist vs cowboy vs elli vs plug vs cgi).
- Which client adapter to standardise on (gleam_fetch vs gleam_hackney vs gleam_httpc).
- Canonical body type for services (`String` vs `BitArray`).
- Whether to centralise JSON encode/decode helpers around the HTTP body boundary.

## Related docs

- `http-and-services` (this doc)
- `json-dynamic-and-api-boundaries`
- `stdlib`
- `javascript-target`
- `erlang-interop`
- `deployment-and-runtime`
- `validation`
- `docs/beam/ports-io.md`
- `docs/beam/applications.md`

## Related skills

- `gleam-language`
- `gleam-packages-ffi`
- `gleam-otp-interop`
