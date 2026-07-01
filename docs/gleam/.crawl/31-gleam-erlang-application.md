# Crawl: gleam/erlang/application.html
- seed_url: https://hexdocs.pm/gleam_erlang/gleam/erlang/application.html
- canonical_url: https://gleam-erlang.hexdocs.pm/gleam/erlang/application.html
- family: Gleam core package module
- fetch: 200
- gleam_erlang_version: v1.3.0
- feeds_docs: erlang-interop.md

## Purpose
`gleam/erlang/application` exposes a minimal, typed wrapper over Erlang/OTP
**application** concepts. Per the module docstring: "An Erlang application is a
collection of code that can be loaded into the Erlang virtual machine and even
started and stopped if they define a start module and supervision tree. Each
Gleam package is an Erlang application."

In v1.3.0 the module surface is intentionally tiny: it models the OTP
`start_type` callback argument and provides access to an application's `priv`
directory. It does **not** (in this version) wrap `application:start/1-2`,
`application:stop/1`, `application:ensure_started/1`, `application:get_env/1-3`,
or `application:set_env/1-3` — those OTP calls are not exposed here.

## Key functions (signatures)
Verbatim from the page (v1.3.0):

```gleam
pub type StartType {
  Normal
  Takeover(previous: node.Node)
  Failover(previous: node.Node)
}

pub fn priv_directory(name: String) -> Result(String, Nil)
```

- `StartType` — type modelling the argument Erlang/OTP passes to an
  application's `start` callback (Module:start/2). Constructors:
  - `Normal` — a normal application start.
  - `Takeover(previous)` — distributed application started at the current node
    because of a takeover from `previous` (via `application:takeover/2` or
    because the current node has higher priority than the previous node).
  - `Failover(previous)` — distributed application started at the current node
    because of a failover from the previous node.
- `priv_directory(name)` — returns the path of an application's `priv`
  directory (where extra non-Gleam/Erlang files are typically kept). Each Gleam
  package is an Erlang application. Returns `Error(Nil)` if no application was
  found with the given name.
  - Example: `application.priv_directory("my_app") // -> Ok("/some/location/my_app/priv")`

Note: `start` / `stop` / `ensure_started` / `get_env` / `set_env` are **not**
present in this module in v1.3.0. The task brief anticipated them; the actual
page does not define them. They are raw Erlang `:application` BIFs a Gleam
program would call via FFI/externals rather than through this module.

## OTP-version requirements
The page declares no explicit Erlang/OTP version requirement. The only version
stamp on the page is the package version: `gleam_erlang v1.3.0`. No
`OTP-XX+`/`since OTP X` annotation appears in any docstring on this page.

## Mapping to BEAM applications (→ docs/beam/applications.md)
This module is the Gleam-side projection of the concepts documented in
[docs/beam/applications.md](../../beam/applications.md):

- **Application = Gleam package.** The docstring states "Each Gleam package is
  an Erlang application." This maps directly to the BEAM application resource
  (`.app`/`.appup`) and the `application` module described in
  `docs/beam/applications.md` — a Gleam package, when compiled, produces an
  OTP application that can be loaded/started/stopped by the BEAM code server
  and application controller.
- **`StartType` ↔ `start_type` argument of `Module:start/2`.** In OTP, an
  application's start callback is `start(StartType, StartArgs)` where
  `StartType` is `normal | {takeover,Node} | {failover,Node}`. `gleam/erlang/
  application.StartType` is a 1:1 typed mirror of that Erlang type:
  `Normal` ↔ `normal`, `Takeover(previous)` ↔ `{takeover, Node}`,
  `Failover(previous)` ↔ `{failover, Node}`. See the application callback
  contract in `docs/beam/applications.md`.
- **`priv_directory` ↔ `code:priv_dir/1` / `application` priv dir.** Maps to
  the priv-directory concept covered in `docs/beam/applications.md` (the
  `priv/` subtree of an application's code directory, accessed at runtime via
  `code:priv_dir/1`).
- **Not wrapped here:** `application:start/stop/ensure_started`,
  `get_env/set_env`, the `.app` resource file, start phases, included
  applications, `appup`/`relup` hot code upgrades, and the code server. All of
  those are documented in `docs/beam/applications.md` and are intentionally
  out of scope for this thin Gleam interop module.

## Strict rules
- Treat each Gleam package as an OTP application; do not invent a separate
  application-loading mechanism in Gleam code.
- When implementing an OTP application start callback in Gleam, pattern-match
  on `StartType` (`Normal` / `Takeover(previous)` / `Failover(previous)`)
  rather than on raw Erlang tuples.
- Use `priv_directory(name)` to locate bundled non-code assets; do not hardcode
  `priv/` paths. Handle `Error(Nil)` (no such application) explicitly.
- Do not assume `start`/`stop`/`ensure_started`/`get_env`/`set_env` wrappers
  exist in this module — they do not in v1.3.0. Call the underlying Erlang
  `:application` functions via FFI/externals if needed, or track future
  versions for additions.

## Verbatim quotes
- "An Erlang application is a collection of code that can be loaded into the
  Erlang virtual machine and even started and stopped if they define a start
  module and supervision tree. Each Gleam package is an Erlang application."
- `StartType` — "The Erlang/OTP application start callback takes a start-type
  as an argument, indicating the context in which the application is being
  started."
  - `Normal` — "A normal application start."
  - `Takeover(previous)` — "The application is distributed and started at the
    current node because of a takeover from Node, either because Erlang's
    `application:takeover/2` function has been called, or because the current
    node has higher priority than the previous node."
  - `Failover(previous)` — "The application is distributed and started at the
    current node because of a failover from the previous node."
- `priv_directory` — "Returns the path of an application's priv directory,
  where extra non-Gleam or Erlang files are typically kept. Each Gleam package
  is an Erlang application. Returns an error if no application was found with
  the given name."
  - Example: `application.priv_directory("my_app") // -> Ok("/some/location/my_app/priv")`

## Version notes
- Page version stamp: `gleam_erlang v1.3.0` (from `<title>` and sidebar).
- No Erlang/OTP minimum version stated on this page.
- Source links point to
  `https://github.com/gleam-lang/erlang/blob/v1.3.0/src/gleam/erlang/application.gleam#L10`
  (StartType) and `...#L37` (priv_directory).

## Discovered links

### Relevant (crawl later)
- https://hexdocs.pm/gleam_erlang/gleam/erlang/atom.html
- https://hexdocs.pm/gleam_erlang/gleam/erlang/charlist.html
- https://hexdocs.pm/gleam_erlang/gleam/erlang/node.html
- https://hexdocs.pm/gleam_erlang/gleam/erlang/port.html
- https://hexdocs.pm/gleam_erlang/gleam/erlang/process.html
- https://hexdocs.pm/gleam_erlang/gleam/erlang/reference.html
- https://github.com/gleam-lang/erlang/blob/v1.3.0/src/gleam/erlang/application.gleam (source)

### Skipped
- https://gleam.run/ (project website, not API docs)
- https://hex.pm/packages/gleam_erlang (package listing, not API docs)
- https://github.com/gleam-lang/erlang (repo root)
- README link, internal `#icon-*` anchors, search/nav chrome
