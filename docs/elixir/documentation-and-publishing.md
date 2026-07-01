# Documentation and Publishing

## Purpose

Provide repo-independent guidance for writing documentation in Elixir. Future agents who write, review, refactor, or validate Elixir code should follow these rules so public APIs are discoverable, accurate, and maintainable, and so documentation stays consistent with the official Elixir documentation tooling.

## Sources used

- https://hexdocs.pm/elixir/writing-documentation.html (PRIMARY)
- https://hexdocs.pm/elixir/Module.html
- https://hexdocs.pm/ex_unit/ExUnit.DocTest.html
- https://hexdocs.pm/elixir/sigils.html
- https://hexdocs.pm/ex_doc/readme.html
- https://hexdocs.pm/ex_doc/Mix.Tasks.Docs.html
- https://hexdocs.pm/ex_doc/ExDoc.html
- https://hexdocs.pm/hex/Mix.Tasks.Hex.Publish.html
- https://hexdocs.pm/hex/Mix.Tasks.Hex.Retire.html
- https://hexdocs.pm/hex/Mix.Tasks.Hex.User.html
- https://hexdocs.pm/hex/Mix.Tasks.Hex.Build.html
- https://hex.pm/docs/publish

This page reflects Elixir v1.20.2 docs.

## Writing Documentation

### Foundational philosophy

From [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

> "Elixir treats documentation as a first-class citizen. Documentation must be easy to write and easy to read."

And:

> "Documentation is an explicit contract between you and users of your Application Programming Interface (API), be they third-party developers, co-workers, or your future self. Modules and functions must always be documented if they are part of their API."

Documentation is stored in module bytecode via module attributes. It can be retrieved at runtime with `Code.fetch_docs/1` and rendered by ExDoc. Because it is a first-class contract for users of the API, public modules, public functions, public macros, public callbacks, and public types MUST have documentation.

### Documentation vs code comments

From [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

> "Elixir treats documentation and code comments as different concepts."

> "...documentation is a contract with users of your API, who may not necessarily have access to the source code, whereas code comments are for those who interact directly with the source."

Documentation attributes (`@moduledoc`, `@doc`, `@typedoc`) are attached to public entities and extracted by tools. Code comments (`#`) are for implementers.

From [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

> "Because private functions cannot be accessed externally, Elixir will warn if a private function has a `@doc` attribute and will discard its content."

Use `@doc`/`@moduledoc`/`@typedoc` only on public entities. Use `#` comments for private functions and internal rationale.

### `@moduledoc`

`@moduledoc` is a module attribute placed inside `defmodule ... do`, before function definitions. It accepts a string (usually a heredoc), `false`, or a keyword list of metadata.

```elixir
defmodule MyApp.Hello do
  @moduledoc """
  This is the Hello module.
  """
  @moduledoc since: "1.0.0"
end
```

`@moduledoc false` hides the whole module from documentation extraction tools. From [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

> "@moduledoc false will make the module invisible to documentation extraction tools like ExDoc."

`@moduledoc since: "1.0.0"` and other metadata (for example `@moduledoc authors: [...]`) are keyword lists.

From [Module.html](https://hexdocs.pm/elixir/Module.html):

> "it is possible to use these attributes more than once before an entity. However, the compiler will warn if used twice with binaries as that replaces the documentation text from the preceding use. Multiple uses with keyword lists will merge the lists into one."

> **Important caveat:** `@moduledoc false` does NOT make the module private. From [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

> "keep in mind @moduledoc false or @doc false do not make a function private. The function above can still be invoked as `MyApp.Sample.add(1, 2)`."

It only controls documentation visibility; the module can still be called, imported, and aliased.

### `@doc`

`@doc` is placed immediately before the function/macro/callback/macrocallback definition. For multi-clause functions, place it before the FIRST clause. From [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

> "Place documentation before the first clause of multi-clause functions. Documentation is always per function and arity and not per clause."

Canonical example from [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

```elixir
@doc """
Says hello to the given `name`.

Returns `:ok`.

## Examples

    iex> MyApp.Hello.world(:john)
    :ok

"""
@doc since: "1.3.0"
def world(name) do
  IO.puts("hello #{name}")
end
```

`@doc` accepts a string (heredoc), `false`, or a keyword list (since Elixir 1.7.0).

`@doc false` hides the function from ExDoc. The same caveat applies: it does not make the function private; it can still be called and even imported.

Metadata keys demonstrated on [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

- `:since` — from [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

  > "It annotates in which version that particular module, function, type, or callback was added."

  Example: `@doc since: "1.3.0"`.

- `:deprecated` — from [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

  > "emits a warning in the documentation, explaining that its usage is discouraged."

  Example: `@doc deprecated: "Use Foo.bar/2 instead"`.

  IMPORTANT distinction. From [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

  > "Note that the :deprecated key does not warn when a developer invokes the functions. If you want the code to also emit a warning, you can use the `@deprecated` attribute."

  The `@deprecated` attribute emits a compile-time warning; the `:deprecated` metadata only annotates the docs.

- `:group` — from [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

  > "The group a function, callback or type belongs to. This is used in iex for autocompleting and also to automatically by ExDoc to group items in the sidebar."

  Example: `@doc group: "Query"`.

**Accuracy note:** The official [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html) and [Module.html](https://hexdocs.pm/elixir/Module.html) pages document `:since`, `:deprecated`, and `:group` as the demonstrated metadata keys. Do NOT assert `@doc delegate_to` or `@doc guard` as part of the core Elixir documentation contract. If you mention them at all, flag clearly that they are not documented on the primary Elixir pages and may be ExDoc-specific or community conventions.

### `@typedoc`

`@typedoc` documents a type. Place it immediately before `@type`, `@typep`, or `@opaque`.

From [Module.html](https://hexdocs.pm/elixir/Module.html):

> "@doc is to be used with a function, macro, callback, or macrocallback, while @typedoc with a type (public or opaque)."

Example:

```elixir
@typedoc "This type"
@typedoc since: "1.1.0"
@type t :: term
```

The same multiple-use/merge rules as `@doc` apply.

### Module attributes reference

| Attribute | Applies to | Accepts | Effect |
|---|---|---|---|
| `@moduledoc` | the module | string / `false` / keyword | module docs; `false` hides the module from ExDoc |
| `@doc` | next function/macro/callback/macrocallback | string / `false` / keyword | docs + metadata (`:since`, `:deprecated`, `:group`) |
| `@typedoc` | next `@type`/`@typep`/`@opaque` | string / `false` / keyword | type docs |
| `@deprecated` | next function/type/module (attribute, NOT metadata) | string | compile-time deprecation warning + doc annotation (since Elixir 1.6.0) |
| `@impl` | next callback implementation | `true` / module | `@impl true` automatically marks the function `@doc false` unless `@doc` is explicitly set (since Elixir 1.5.0) |

### Heredocs and the `~S` sigil

Multi-line docs use heredocs (`""" ... """`). When docs contain backslashes (for example examples showing escaped quotes), a regular heredoc forces double-escaping, which is error-prone. Use `~S""" ... """` to disable interpolation and escaping.

From [sigils.html](https://hexdocs.pm/elixir/sigils.html):

> "The most common use case for heredoc sigils is when writing documentation."

Without `~S` (double-escaped, error-prone):

```elixir
@doc """
Converts double-quotes to single-quotes.

## Examples

    iex> convert("\\\"foo\\\"")
    "'foo'"

"""
def convert(...)
```

With `~S` (no escaping):

```elixir
@doc ~S"""
Converts double-quotes to single-quotes.

## Examples

    iex> convert("\"foo\"")
    "'foo'"

"""
def convert(...)
```

From [sigils.html](https://hexdocs.pm/elixir/sigils.html):

> "By using ~S, this problem can be avoided altogether."

Rule of thumb: prefer `~S"""` for documentation heredocs whenever the content contains backslashes or `#{}` that should appear literally.

### Doctests in documentation

Doctests are executable code examples inside `@doc`/`@moduledoc` heredocs, prefixed with `iex>`. They are parsed and run by ExUnit's `ExUnit.DocTest`.

From [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

> "We advise developers to include examples in their documentation, often under their own `## Examples` heading."

> "To ensure examples do not get out of date, Elixir's test framework (ExUnit) provides a feature called doctests that allows developers to test the examples in their documentation."

Format: examples go under a `## Examples` heading; the code block is indented 4 spaces; each test starts with `iex>`.

Multi-line expressions use `...>` continuation. From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "Multiline expressions can be used by prefixing subsequent lines with either `...>` (recommended) or `iex>`."

Example:

```elixir
iex> Enum.map([1, 2, 3], fn x ->
...>   x * 2
...> end)
[2, 4, 6]
```

Doctests are wired up in a test module:

```elixir
defmodule MyApp.HelloTest do
  use ExUnit.Case, async: true
  doctest MyApp.Hello
end
```

`doctest/2` is auto-imported with `ExUnit.Case`. It accepts `:only`, `:except`, `:import`, `:tags`, and `:inspect_opts`. Since Elixir 1.15.0, `doctest_file/2` supports doctests in standalone Markdown files.

When NOT to use doctests. From [ExUnit.DocTest.html](https://hexdocs.pm/ex_unit/ExUnit.DocTest.html):

> "In general, doctests are not recommended when your code examples contain side effects. For example, if a doctest prints to standard output, doctest will not try to capture the output."

> "doctests do not run in any kind of sandbox."

See `docs/elixir/testing-exunit.md` for details on doctest execution and options.

### Recommendations for writing good docs

From [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

> "Keep the first paragraph of the documentation concise and simple, typically one-line. Tools like ExDoc use the first line to generate a summary."

> "Reference modules by their full name."

> "Reference functions by name and arity if they are local, as in `world/1`, or by module, name and arity if pointing to an external module: `MyApp.Hello.world/1`."

> "Reference a `@callback` by prepending `c:`, as in `c:world/1`."

> "Reference a `@type` by prepending `t:`, as in `t:values/0`."

> "Start new sections with second level Markdown headers `##`. First level headers are reserved for module and function names."

> "Place documentation before the first clause of multi-clause functions. Documentation is always per function and arity and not per clause."

> "Use the `:since` key in the documentation metadata to annotate whenever new functions or modules are added to your API."

### Hiding internal modules and functions

Use `@doc false` to hide a particular function, or `@moduledoc false` to hide the whole module. From [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

> "setting @doc false to hide a particular function, or @moduledoc false to hide the whole module."

Re-state the critical caveat. From [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

> "keep in mind @moduledoc false or @doc false do not make a function private."

Two recommended patterns from [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

1. Move undocumented functions into a module marked `@moduledoc false` (for example `MyApp.Hidden`), so they are not accidentally exposed or imported.
2. Prefix the function name with one or two underscores (`__add__/2`). From [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html):

   > "Functions starting with underscore are automatically treated as hidden, although you can also be explicit and add @doc false. The compiler does not import functions with leading underscores."

Example:

```elixir
defmodule MyApp.Hidden do
  @moduledoc false

  @doc """
  This function won't be listed in docs.
  """
  def function_that_wont_be_listed_in_docs do
    # ...
  end
end
```

## ExDoc

ExDoc is the official tool that generates HTML, Markdown (including `llms.txt`), and EPUB documentation for Elixir/Erlang projects. It reads the `@moduledoc`, `@doc`, and `@typedoc` attributes embedded in compiled module bytecode — no separate doc source files are needed. Configuration lives under the `:docs` key in `mix.exs`; docs are built with `mix docs` (ExDoc >= 0.34 for Elixir 1.15+).

Sources: https://hexdocs.pm/ex_doc/readme.html, https://hexdocs.pm/ex_doc/Mix.Tasks.Docs.html, https://hexdocs.pm/ex_doc/ExDoc.html

### What ExDoc generates

From [readme.html](https://hexdocs.pm/ex_doc/readme.html):

> "Automatically generates offline-accessible HTML, Markdown (including `llms.txt`), and EPUB documents from your API documentation."

The default formatters are HTML, Markdown, and EPUB. The Markdown output includes `llms.txt`; every generated page footer links to "View llms.txt".

### How ExDoc reads documentation

ExDoc sources `@moduledoc`, `@doc`, and `@typedoc` from compiled BEAM/ebin directories — the `mix docs` task feeds those `ebin` paths, so no separate doc source files are required.

From [readme.html](https://hexdocs.pm/ex_doc/readme.html):

> "ExDoc will automatically link modules, functions, types or callbacks defined in your project and its dependencies (including Erlang and Elixir)."

### The `mix docs` task

From [Mix.Tasks.Docs.html](https://hexdocs.pm/ex_doc/Mix.Tasks.Docs.html):

> "`mix docs` — Uses ExDoc to generate a static web page from the project documentation."

| Flag | Meaning |
|---|---|
| `--formatter`, `-f` | Which formatters to use, `html`, `epub`, or `markdown`. This option can be given more than once. By default, `html`, `epub`, and `markdown` are generated. |
| `--output`, `-o` | Output directory for the generated docs, default: `"doc"` |
| `--language` | Specifies the language to annotate the EPUB output in valid BCP 47 |
| `--canonical`, `-n` | Indicate the preferred URL with `rel="canonical"` link element, defaults to no canonical path |
| `--proglang` | Chooses the main programming language: `elixir` or `erlang` |
| `--open` | open browser window pointed to the documentation |
| `--warnings-as-errors` | Exits with non-zero exit code if any warnings are found |

The command line options have higher precedence than the options specified in your `mix.exs` file.

### Adding ExDoc as a dependency

From [readme.html](https://hexdocs.pm/ex_doc/readme.html):

> "ExDoc requires Elixir v1.15 or later. Then add ExDoc as a dependency:"

```elixir
{:ex_doc, "~> 0.34", only: :dev, runtime: false, warn_if_outdated: true}
```

Use `runtime: false` and `only: :dev` so ExDoc is never shipped as a runtime dependency. Then run `mix deps.get`.

### Configuring docs in `mix.exs`

The `:docs` key lives in `def project`. It can be an inline keyword list or a function reference such as `&docs/0`.

From [readme.html](https://hexdocs.pm/ex_doc/readme.html):

```elixir
def project do
  [
    app: :my_app,
    version: "0.1.0-dev",
    deps: deps(),

    # Docs
    name: "MyApp",
    source_url: "https://github.com/USER/PROJECT",
    homepage_url: "http://YOUR_PROJECT_HOMEPAGE",
    docs: &docs/0
  ]
end

defp docs do
  [
    main: "readme", # can be changed to a module name, if you prefer
    logo: "path/to/logo.png",
    extras: ["README.md"]
  ]
end
```

The inline alternative from [Mix.Tasks.Docs.html](https://hexdocs.pm/ex_doc/Mix.Tasks.Docs.html):

```elixir
def project do
  [
    app: :my_app,
    version: "0.1.0-dev",
    deps: deps(),

    # Docs
    name: "My App",
    source_url: "https://github.com/USER/PROJECT",
    homepage_url: "http://YOUR_PROJECT_HOMEPAGE",
    docs: [
      main: "MyApp", # The main page in the docs
      favicon: "path/to/favicon.png",
      logo: "path/to/logo.png",
      extras: ["README.md"]
    ]
  ]
end
```

Project-level companions include `:name`, `:source_url`, and `:homepage_url`.

### `:docs` configuration reference

| Option | Type / Default | Meaning |
|---|---|---|
| `:main` | string | Main page of the documentation. It may be a module or a generated page, like `"Plug"` or `"api-reference"`. Default: `"api-reference"`. |
| `:extras` | list of paths | List of paths to additional Markdown (`.md`), Live Markdown (`.livemd`), Cheatsheets (`.cheatmd`), external urls (`:url` option), and plain text pages. Default: `[]`. |
| `:source_ref` | string | The branch/commit/tag used for source link inference. Default: `"main"`. |
| `:source_url` | string | The source URL fallback if `:source_url` is not given at the project configuration. |
| `:logo` | path | Path to a logo image file for the project. Must be PNG, JPEG or SVG. The image will be shown within a 48x48px area. |
| `:formatters` | list | Formatter to use; default: `["html", "markdown", "epub"]`, options: `"html"`, `"markdown"`, `"epub"`. |
| `:api_reference` | boolean | Whether to generate `api-reference.html`; default: `true`. If this is set to false, `:main` must also be set. |
| `:extra_section` | string | String that defines the section title of the additional Markdown and plain text pages; default: `"Pages"`. Example: `"Guides"`. |
| `:groups_for_extras` / `:groups_for_modules` / `:groups_for_docs` | keyword list | See the "Groups" section in the ExDoc docs. |
| `:nest_modules_by_prefix` | list of modules | See the "Nesting" section in the ExDoc docs. |
| `:before_closing_head_tag` | function | A function taking the formatter atom (`:html` or `:epub`) and returning a literal HTML string to include just before `</head>`. Useful for custom CSS. |
| `:before_closing_body_tag` | function | A function taking the formatter atom and returning a literal HTML string to include just before `</body>`. Useful for custom Javascript. |
| `:before_closing_footer_tag` | function | A function taking the formatter atom and returning a literal HTML string to include just before `</footer>`. Only affects the HTML formatter. |
| `:language` | BCP 47 tag | Identify the primary language of the documents. Default: `"en"`. |
| `:output` | string | Output directory for the generated docs. Default: `"doc"`. May be overridden by command line argument. |
| `:canonical` | string | String that defines the preferred URL with the `rel="canonical"` element; defaults to no canonical path. |
| `:cover` | path | Path to the EPUB cover image (only PNG or JPEG accepted). Around 1600x2400. No effect with the `"html"` formatter. |
| `:deps` | keyword list | Application names and their documentation URL. ExDoc defaults to HexDocs; override with values like `[plug: "https://myserver/plug/"]`. |
| `:skip_undefined_reference_warnings_on` | list or function | Controls when to skip warnings for unresolvable references; can be a list of strings (exact match on filename, node ID like `User.exists?/1`, or module name) or a function taking a reference returning boolean. |
| `:filter_modules` | function | Filter which modules are included. |
| `:ignore_apps` | list | Exclude umbrella child apps from the generated docs. |

### Nesting modules in the sidebar

From [ExDoc.html](https://hexdocs.pm/ex_doc/ExDoc.html):

> "ExDoc also allows module names in the sidebar to appear nested under a given prefix. The `:nest_modules_by_prefix` expects a list of module names, such as `[Foo.Bar, Bar.Baz]`. In this case, a module named `Foo.Bar.Baz` will appear nested within `Foo.Bar` and only the name `Baz` will be shown in the sidebar. Note the `Foo.Bar` module itself is not affected."

### Grouping modules and extras

From [readme.html](https://hexdocs.pm/ex_doc/readme.html):

```elixir
groups_for_modules: [
  "Data types": [Atom, Regex, URI],
  "Collections": [Enum, MapSet, Stream]
]
```

> "A regex or the string name of the module is also supported."

```elixir
groups_for_extras: [
  "Introduction": Path.wildcard("guides/introduction/*.md"),
  "Advanced": Path.wildcard("guides/advanced/*.md")
]
```

### Additional pages (extras)

From [readme.html](https://hexdocs.pm/ex_doc/readme.html):

> "You can publish additional pages in your project documentation by configuring them as `:extras`. The following formats and extensions are supported: Markdown (`.md` extension) ...; Cheatsheets (`.cheatmd` extension) ...; Livebooks (`.livemd` extension) ..."

Per-extra customization keys include `:title`, `:filename`, and `:source`:

```elixir
extras: ["README.md", "LICENSE", "tutorial.livemd", "cheatsheet.cheatmd"]
```

A customized example:

```elixir
extras: ["README.md", "LICENSE", "CONTRIBUTING.md": [filename: "contributing", title: "Contributing", source: "CONTRIBUTING.mdx"]]
```

### Injecting custom assets

From [ExDoc.html](https://hexdocs.pm/ex_doc/ExDoc.html):

> "`before_closing_head_tag` - a function that takes as argument an atom specifying the formatter being used (`:html` or `:epub`) and returns a literal HTML string to be included just before the closing head tag (`</head>`). ... Useful to inject custom assets, such as CSS stylesheets."

> "`before_closing_body_tag` - a function ... just before the closing body tag (`</body>`). ... Useful to inject custom assets, such as Javascript."

**Accuracy note:** The authoritative, complete options reference is `ExDoc.generate/4` ([ExDoc.html](https://hexdocs.pm/ex_doc/ExDoc.html)). The readme and `mix docs` task page document the happy path, but `ExDoc.generate/4` is the complete list.

## Hex Publishing

Hex.pm is the Elixir/Erlang package manager. Packages are published with `mix hex.publish`, which packages the project per the `:package` metadata in `mix.exs` and pushes it to hex.pm. ExDoc-generated documentation is published automatically alongside the package when configured.

Sources: https://hexdocs.pm/hex/Mix.Tasks.Hex.Publish.html, https://hexdocs.pm/hex/Mix.Tasks.Hex.Retire.html, https://hexdocs.pm/hex/Mix.Tasks.Hex.User.html, https://hexdocs.pm/hex/Mix.Tasks.Hex.Build.html, https://hex.pm/docs/publish

### `mix hex.publish`

From [Mix.Tasks.Hex.Publish.html](https://hexdocs.pm/hex/Mix.Tasks.Hex.Publish.html):

> "Publishes a new version of the package."

> "The current authenticated user will be the package owner. Only package owners can publish the package, new owners can be added with the `mix hex.owner` task."

> "Packages and documentation sizes are limited to 8mb compressed, and 64mb uncompressed."

### Command-line flags

| Flag | Meaning |
|---|---|
| `--dry-run` | Builds package and performs local checks without publishing, use `mix hex.build --unpack` to inspect package contents before publishing |
| `--yes` | Publishes the package without any confirmation prompts |
| `--replace` | Allows overwriting an existing package version if it exists. Private packages can always be overwritten, public packages can only be overwritten within one hour after they were initially published. |
| `--revert VERSION` | Revert given version. If the last version is reverted, the package is removed. |
| `--organization ORGANIZATION` | Set this for private packages belonging to an organization |

### Dry-run and inspecting the package

`--dry-run` builds the package and performs local checks without publishing. Inspect the built package with `mix hex.build --unpack`.

From [Mix.Tasks.Hex.Build.html](https://hexdocs.pm/hex/Mix.Tasks.Hex.Build.html):

> "Builds a new local version of your package. The package .tar file is created in the current directory, but is not pushed to the repository."

### Documentation is published automatically

From [Mix.Tasks.Hex.Publish.html](https://hexdocs.pm/hex/Mix.Tasks.Hex.Publish.html):

> "Documentation will be generated by running the `mix docs` task. ex_doc provides this task by default, but any library can be used. Or an alias can be used to extend the documentation generation. The expected result of the task is the generated documentation located in the `doc/` directory with an `index.html` file."

> "Documentation will be built and published automatically. To publish a package without documentation run `mix hex.publish package` or to only publish documentation run `mix hex.publish docs`."

Published docs are served at `https://hexdocs.pm/my_package/1.0.0`; `https://hexdocs.pm/my_package` redirects to the latest version.

### Authentication

Register a new account with `mix hex.user register`, then authorize the local machine with `mix hex.user auth`. Check the current user with `mix hex.user whoami`.

From [Mix.Tasks.Hex.User.html](https://hexdocs.pm/hex/Mix.Tasks.Hex.User.html):

> "Authorizes a new user on the local machine."

> "Print the current user"

For CI, generate a scoped API key:

```bash
$ mix hex.user key generate --key-name publish-ci --permission api:write
```

Set it in the `HEX_API_KEY` environment variable and publish non-interactively:

```bash
$ HEX_API_KEY=f48ac236bca15c3271e077c15c5320c4 mix hex.publish --yes
```

### `:package` metadata in `mix.exs`

From [hex.pm/docs/publish](https://hex.pm/docs/publish):

```elixir
defp package() do
  [
    # This option is only needed when you don't want to use the OTP application name
    name: "postgrex",
    # These are the default files included in the package
    files: ~w(lib priv .formatter.exs mix.exs README* readme* LICENSE*
              license* CHANGELOG* changelog* src),
    licenses: ["Apache-2.0"],
    links: %{"GitHub" => "https://github.com/elixir-ecto/postgrex"}
  ]
end
```

| Key | Scope | Meaning |
|---|---|---|
| `:name` | under `:package` | Set this if the package name is not the same as the application name. |
| `:files` | under `:package` | List of files and directories to include in the package, can include wildcards. Defaults to `["lib", "priv", ".formatter.exs", "mix.exs", "README*", "readme*", "LICENSE*", "license*", "CHANGELOG*", "changelog*", "src", "c_src", "Makefile*"]`. |
| `:exclude_patterns` | under `:package` | List of patterns matching files and directories to exclude from the package. |
| `:licenses` | under `:package` | List of licenses used by the package. This attribute is required. Valid license identifiers are available from SPDX. Custom licenses may use `LicenseRef-<idstring>` identifiers. |
| `:links` | under `:package` | Map of links relevant to the package. |
| `:build_tools` | under `:package` | List of build tools that can build the package. Hex will try to automatically detect the build tools. |
| `:organization` | under `:package` | Set this for private packages belonging to an organization. |

**IMPORTANT:** `:version`, `:description`, and `:app` are **top-level** `def project` keys, NOT under `:package`. `:licenses` is plural and required; valid identifiers come from SPDX ([https://spdx.org/licenses/](https://spdx.org/licenses/)).

### Semantic versioning

From [hex.pm/docs/publish](https://hex.pm/docs/publish):

> "All Hex packages are required to follow semantic versioning."

> "While your package version is at major version `"0"`, any breaking changes should be indicated by incrementing the minor version. For example, `0.1.0 -> 0.2.0`."

Dependency operator semantics:

- `~> 2.0.0` means `>= 2.0.0 and < 2.1.0`.
- `~> 2.0` means `>= 2.0.0 and < 3.0.0`.

### Immutability and the publish window

From [Mix.Tasks.Hex.Publish.html](https://hexdocs.pm/hex/Mix.Tasks.Hex.Publish.html):

> "A new package can be reverted or updated within 24 hours of its initial publish. A new version of an existing package can be reverted or updated within one hour. Documentation has no limitations on when it can be updated."

To update the package, run `mix hex.publish` again with `--replace` while the window is open. To revert a version, run `mix hex.publish --revert VERSION`; to revert only docs, run `mix hex.publish docs --revert VERSION`. If the last version is reverted, the package is removed. Once the window passes, a published version cannot be overwritten — you must publish a new version or retire the existing one.

### Retiring a package version

**IMPORTANT:** Retirement is the separate task `mix hex.retire`, NOT a flag on `mix hex.publish`.

From [Mix.Tasks.Hex.Retire.html](https://hexdocs.pm/hex/Mix.Tasks.Hex.Retire.html):

> "Retires a package version."

Syntax:

```bash
$ mix hex.retire PACKAGE VERSION REASON
$ mix hex.retire PACKAGE VERSION --unretire
```

> "Mark a package as retired when you no longer recommend its usage. A retired package is still resolvable and usable but it will be flagged as retired in the repository and a message will be displayed to users when they use the package."

| Reason | Meaning |
|---|---|
| `renamed` | The package has been renamed, including the new package name in the message |
| `deprecated` | The package has been deprecated, if there's a replacing package include it in the message |
| `security` | There are security issues with this package |
| `invalid` | The package is invalid, for example it does not compile correctly |
| `other` | Any other reason not included above, clarify the reason in the message |

`--message "MESSAGE"` is required (up to 140 characters). Example:

```bash
$ mix hex.retire your_app 0.1.1 invalid --message "Package has a breaking bug"
```

### Organizations (private packages)

From [Mix.Tasks.Hex.Publish.html](https://hexdocs.pm/hex/Mix.Tasks.Hex.Publish.html):

> "`--organization ORGANIZATION` - Set this for private packages belonging to an organization"

Or add the key in `:package` configuration from [hex.pm/docs/private](https://hex.pm/docs/private):

> "add the organization: \"acme\" option to the package configuration."

### Typical publish workflow

1. Register and authenticate: `mix hex.user register` then `mix hex.user auth`.
2. Configure `mix.exs`: set top-level `:version`, `:description`, `:app`, plus `:package`, `:deps`, and project-level `:name` / `:source_url` / `:homepage_url`.
3. Add ExDoc as a dev dependency: `{:ex_doc, "~> 0.34", only: :dev, runtime: false}`.
4. Build docs locally: `mix docs`.
5. Dry-run the publish (and optionally inspect with `mix hex.build --unpack`): `mix hex.publish --dry-run`.
6. Publish the package and docs: `mix hex.publish`.
7. Republish docs only if needed: `mix hex.publish docs`.
8. Retire a bad version if needed: `mix hex.retire PACKAGE VERSION REASON --message "..."`.

**Accuracy note:** The canonical option names are `:exclude_patterns` (NOT `:exclude_build` or `:exclude_paths`); `:licenses` (plural); and retirement uses `mix hex.retire`, which is distinct from `mix hex.publish --revert`.

## Review checklist

- [ ] Every public module has a `@moduledoc` (heredoc, first line is a concise summary).
- [ ] Every public function/macro/callback has a `@doc` placed before the first clause.
- [ ] Every public `@type`/`@opaque` has a `@typedoc`.
- [ ] Private functions use `#` code comments, NOT `@doc` (compiler warns otherwise).
- [ ] First line of each `@doc`/`@moduledoc` is a single concise summary sentence.
- [ ] Cross-references use full forms: `` `Mod.func/arity` ``, `` `c:name/arity` ``, `` `t:name/0` ``.
- [ ] New API additions carry `@doc since: "x.y.z"` / `@moduledoc since:` / `@typedoc since:`.
- [ ] Deprecations use `@deprecated "reason"` for compile-time warnings; `@doc deprecated:` is documentation-only.
- [ ] Documentation heredocs with backslashes/`#{}` use `~S"""` to avoid double-escaping.
- [ ] Internal modules are marked `@moduledoc false`; underscore-prefixed helpers are not relied upon for "privacy".
- [ ] Doctests under `## Examples` are wired up with `doctest Module` in a test module.

## Implementation checklist

- [ ] Add `@moduledoc """..."""` as the first attribute inside `defmodule`.
- [ ] Add `@doc` immediately before the first clause of each public function; repeat for macros/callbacks.
- [ ] Add `@typedoc` immediately before each public `@type`/`@opaque`.
- [ ] Choose `~S"""` vs `"""` based on whether the content contains backslashes or interpolation.
- [ ] Add a `## Examples` section with `iex>` doctests for non-side-effecting functions.
- [ ] Add `doctest MyModule` to the corresponding `_test.exs`.
- [ ] Mark internal-only modules with `@moduledoc false`; do not assume this makes them private.
- [ ] Annotate new additions with `:since`; deprecate with `@deprecated`.

## Validation hooks

- `mix docs` — builds ExDoc HTML locally so you can visually verify the generated docs (requires ExDoc configured). Note this does not validate that docs exist.
- `mix test` — runs `doctest Module` cases; failing doctests indicate stale or incorrect examples.
- `mix compile --warnings-as-errors` — surfaces the warning emitted when `@doc` is placed on a private function (its content is discarded).
- `mix credo` — community linter; can flag missing module/function documentation (`Credo.Check.Readability.ModuleDoc`, `Credo.Check.Readability.FunctionNameDoc`-style checks). Community tool, not official.
- `mix dialyzer` — cross-checks `@typedoc`'d types against `@spec`s; type-doc/reference mismatches surface here. See `docs/elixir/typespecs-and-dialyzer.md`.

There is no built-in compiler check that forces `@doc`/`@moduledoc` to exist on public entities — missing documentation compiles silently. `mix credo` is the typical enforcement layer.

## Examples

### A complete documented module

```elixir
defmodule MyApp.Counter do
  @moduledoc """
  A simple in-memory counter.

  ## Examples

      iex> alias MyApp.Counter
      iex> counter = Counter.new()
      iex> Counter.increment(counter)
      1

  """

  @typedoc """
  The counter state.
  """
  @typedoc since: "1.0.0"
  @type t :: integer

  @doc """
  Returns a new counter starting at zero.
  """
  @doc since: "1.0.0"
  @spec new() :: t
  def new, do: 0

  @doc """
  Increments the counter by one.

  ## Examples

      iex> MyApp.Counter.increment(5)
      6

  """
  @doc since: "1.0.0"
  @spec increment(t) :: t
  def increment(counter), do: counter + 1
end
```

Corresponding test module:

```elixir
defmodule MyApp.CounterTest do
  use ExUnit.Case, async: true
  doctest MyApp.Counter
end
```

### `~S"""` vs `"""`

Without `~S`:

```elixir
@doc """
Converts double-quotes to single-quotes.

## Examples

    iex> convert("\\\"foo\\\"")
    "'foo'"

"""
def convert(...)
```

With `~S`:

```elixir
@doc ~S"""
Converts double-quotes to single-quotes.

## Examples

    iex> convert("\"foo\"")
    "'foo'"

"""
def convert(...)
```

### Internal module pattern

```elixir
defmodule MyApp.Hidden do
  @moduledoc false

  @doc """
  This function won't be listed in docs.
  """
  def function_that_wont_be_listed_in_docs do
    # ...
  end
end
```

### `@deprecated`

```elixir
defmodule MyApp.Legacy do
  @moduledoc false

  @doc "Old name for new_logic/1."
  @doc deprecated: "Use MyApp.Legacy.new_logic/1 instead."
  @deprecated "Use MyApp.Legacy.new_logic/1 instead."
  def old_logic(x), do: new_logic(x)

  def new_logic(x), do: x * 2
end
```

The `@deprecated` attribute produces a compile-time warning when `old_logic/1` is called. The `@doc deprecated:` metadata only renders the deprecation note in the generated docs.

## Common mistakes

- Putting `@doc` on a `defp` (compiler warns and discards the doc) — use `#` comments instead.
- Assuming `@moduledoc false` / `@doc false` makes a function private — it does not; it only hides from ExDoc and the function can still be called/imported.
- Placing `@doc` on every clause of a multi-clause function (docs are per name+arity, attached to the first clause).
- Using `@doc deprecated:` and expecting a compile-time warning — only the `@deprecated` attribute warns at compile time; the metadata only annotates the docs.
- Using a plain `"""` heredoc for docs containing `\` or `#{}`, causing double-escaping bugs — use `~S"""`.
- Forgetting `:since` on newly added public API.
- Writing `@doc delegate_to:` or `@doc guard:` — these are NOT part of the core Elixir documentation contract (not documented on [writing-documentation.html](https://hexdocs.pm/elixir/writing-documentation.html) / [Module.html](https://hexdocs.pm/elixir/Module.html)); do not rely on them without an ExDoc-specific source.
- Letting doctests go stale — always run `doctest` via `mix test`; avoid doctests for functions with side effects or non-deterministic output (for example PIDs, timestamps).
- Using first-level `#` Markdown headers inside `@doc` (reserved for module/function names) — use `##` for sections.
- Referencing functions inconsistently (for example `Hello.world` without arity) — use the `name/arity` form.

## Strict vs contextual guidance

### Strict (language/compiler-enforced)

- A `@doc` on a `defp` triggers a compiler warning and the doc is discarded.
- `@doc`/`@typedoc`/`@moduledoc` placement binds to the immediately following entity (function/macro/callback/type); for multi-clause functions the doc attaches per name+arity at the first clause.
- Underscore-prefixed functions (`_foo`) are not auto-imported (language behavior), and are treated as hidden by doc tools.
- `@impl true` automatically sets `@doc false` unless an explicit `@doc` is given (since Elixir 1.5.0).
- Multiple string `@doc`/`@typedoc` before one entity warn; multiple keyword-list uses merge.

### Conventions (not enforced by the compiler)

- Documenting every public module/function/type (no compiler error if missing; typically enforced by credo).
- First line as a one-line summary (ExDoc consumes it, but nothing enforces it).
- `## Examples` heading + 4-space-indented `iex>` doctests.
- `:since`, `:deprecated` (metadata) usage.
- `~S"""` for backslash/interpolation-heavy docs.
- Referencing functions/types/callbacks in canonical `name/arity`, `t:name/0`, `c:name/arity` forms.

### Contextual tradeoffs

- `@doc false` vs moving code to an `@moduledoc false` module — the latter also prevents accidental `import`.
- Doctests vs plain `## Examples` — use doctests for pure/deterministic code; use non-doctest examples (or none) for side-effecting or non-deterministic functions.
- `@deprecated` (attribute, compile-time) vs `@doc deprecated:` (docs-only) — choose based on whether callers should be warned at compile time.

## Policy decisions for individual repos

- Whether to require `@moduledoc`/`@doc` on all public entities in CI (via `mix credo --strict` checks like `Credo.Check.Readability.ModuleDoc`).
- Whether doctests are mandatory for all pure public functions.
- Versioning policy for `:since` annotations (must it match `mix.exs` `:version`? semver-only?).
- Whether `@doc false` is allowed at all, or whether internal code must move into `@moduledoc false` modules.
- ExDoc configuration policy (`:main`, `:extras`, sidebar grouping) — covered in the `## ExDoc` section above.
- Publishing policy (hex.pm org, `:licenses`, `:links`, retirement) — covered in the `## Hex Publishing` section above.
- Lint stack ordering: `mix compile --warnings-as-errors`, `mix credo --strict`, `mix test`, `mix docs`.

## Related docs

- `docs/elixir/testing-exunit.md` — doctests are executed by ExUnit via `ExUnit.DocTest`; see the doctests section there.
- `docs/elixir/typespecs-and-dialyzer.md` — `@typedoc` documents `@type`/`@opaque`; type reference conventions.
- `docs/elixir/mix-project-structure.md` — ExDoc `:docs` config and `:package` metadata live in `mix.exs`.
- `docs/elixir/naming-conventions.md` — referencing functions/types by name and arity.
- `docs/elixir/static-analysis-credo.md` — credo checks that enforce documentation presence.
- `docs/elixir/configuration-and-runtime.md`
- `docs/elixir/core-modules.md`
- `docs/elixir/language-fundamentals.md`
- `docs/elixir/otp-supervision.md`

## Related skills

- None defined yet.
