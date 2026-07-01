# Crawl: hexdocs.pm/gleam_http/ (index)
- seed_url: https://hexdocs.pm/gleam_http/
- canonical_url: https://gleam-http.hexdocs.pm/
- family: Gleam core package (gleam_http)
- fetch: 200
- gleam_http_version: 4.3.0
- feeds_docs: http-and-services.md

## Purpose
gleam_http provides HTTP types and functions for HTTP clients and servers in Gleam.
It defines the core data structures (Request, Response, Headers, Method, Status) and a
sans-IO service abstraction that lets a single Gleam function serve as both an HTTP
server handler and an HTTP client target. The package itself ships no I/O: concrete
networking is provided by external adapter packages (server adapters and client adapters).

## Module list (http/request/response/service/cookie + one-line each)
- `gleam/http` — core HTTP types: Method, Status, Scheme, Headers, and header helpers.
- `gleam/http/cookie` — cookie parsing/serialization helpers for Set-Cookie and Cookie headers.
- `gleam/http/request` — Request type and constructors/accessors (method, headers, body, scheme, host, path, query).
- `gleam/http/response` — Response type and constructors/accessors (status, headers, body) plus helpers.
- `gleam/http/service` — the sans-IO `Service(conn, body)` type and the request→response service contract.

## Sans-IO service abstraction (Service type, request→response)
The `gleam/http/service` module defines the `Service(conn, body)` type: a function
`fn(Request(body)) -> Response(body)` parameterized by a connection type `conn` and a
body type `body`. A service is a pure function from Request to Response and performs no
I/O itself. This makes services target-agnostic: the same service function can be handed
to any server adapter (to handle inbound requests) or invoked by any client adapter (to
produce a response from an outbound request). Adapters supply the `conn`/`body` concrete
types and the actual socket/transport plumbing; the service logic stays pure and reusable
across targets (Erlang/JavaScript, server/client).

## Adapter ecosystem (mist/wisp/gleam_httpc/gleam_hackney/gleam_fetch)
Server adapters (run a gleam_http Service as a server), per the index page:
- Mist — high performance pure Gleam HTTP/1.1 server (github.com/rawhat/mist).
- cgi — Common Gateway Interface adapter (github.com/lpil/cgi).
- gleam_cowboy — Cowboy (Erlang HTTP/2 & HTTP/1.1) adapter (github.com/gleam-lang/cowboy).
- gleam_elli — Elli (Erlang HTTP/1.1) adapter (github.com/gleam-lang/elli).
- gleam_plug — Plug (Elixir web application interface) adapter (github.com/gleam-lang/plug).

Client adapters (send HTTP requests over the network), per the index page:
- gleam_fetch — JavaScript fetch API client (github.com/gleam-lang/fetch).
- gleam_hackney — Hackney client for Erlang (github.com/gleam-lang/hackney).
- gleam_httpc — httpc client bundled with Erlang/OTP (github.com/gleam-lang/httpc).

Note: wisp is not listed on the gleam_http index page itself; wisp is a separate web
framework built on top of Mist and is part of the broader Gleam HTTP ecosystem rather
than a direct adapter enumerated here.

## Version notes
- Page reports gleam_http v4.3.0 (HexDocs canonical host: gleam-http.hexdocs.pm).
- Doc site built with ExDoc/LexDocs-style index (v1.13.0-rc1 assets).
- README on the page lists the server/client adapter table summarized above.

## Discovered links

### Relevant (crawl later)
- https://gleam-http.hexdocs.pm/gleam/http.html (core types module)
- https://gleam-http.hexdocs.pm/gleam/http/service.html (Service abstraction)
- https://gleam-http.hexdocs.pm/gleam/http/request.html
- https://gleam-http.hexdocs.pm/gleam/http/response.html
- https://gleam-http.hexdocs.pm/gleam/http/cookie.html
- https://hex.pm/packages/gleam_http (Hex package page)
- https://github.com/gleam-lang/http (source repo)

### Skipped
- https://gleam.run/ (Gleam language site, out of scope)
- https://github.com/sponsors/lpil (sponsorship)
- Adapter upstream repos (cowboy/elli/plug/hackney/httpc/fetch/mist/cgi) — third-party, not gleam_http modules
- https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API (MDN reference)
- https://erlang.org/doc/man/httpc.html (Erlang httpc reference)
- Internal anchors and CSS/JS assets
