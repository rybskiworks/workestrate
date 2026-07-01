# Crawl: language-server-reference/
- seed_url: https://gleam.run/language-server-reference/
- canonical_url: https://gleam.run/language-server/
- family: Gleam official guide
- fetch: 404 on seed (https://gleam.run/language-server-reference/); 200 on fallback (https://gleam.run/language-server/)
- gleam_version: not present on page (no version pinned; LSP ships inside the `gleam` binary)
- feeds_docs: project-structure-and-cli.md, validation.md

## Purpose
The Gleam Language Server is a program that provides IDE features to text
editors that implement the Language Server Protocol (LSP), such as VS Code and
Neovim. This document details the current state of the language server and its
features. It is an official Gleam project, the newest part of the Gleam
toolchain, and is actively being developed and rapidly improving — it does not
yet have all features found in more mature language servers for older
languages.

## LSP features
The language server is bundled inside the regular `gleam` binary; installing
Gleam installs the language server. Editors run `gleam lsp` from the workspace
root.

Core capabilities:
- **Multiple project support** — open files from multiple Gleam projects in one
  editor session; the server understands which project each file belongs to.
- **Project compilation** — automatically compiles code in opened Gleam
  projects. Code generation and Erlang compilation are NOT performed. Unsaved
  edits are used when compiling. The target specified in `gleam.toml` is used;
  if none is specified it defaults to Erlang.
- **Error and warning diagnostics** — errors/warnings found while compiling are
  surfaced as LSP diagnostics.
- **Code formatting** — formats Gleam code using the Gleam formatter; can be
  configured to run on save.
- **Hover** — shows documentation, types, and other info when hovering on:
  constants; import statements (incl. unqualified values and types); module
  functions; module qualifiers; patterns; record fields; the `..` used to
  ignore additional fields in record patterns; type annotations; values.
- **Go-to definition** — supported for: constants; functions; import statements
  (incl. unqualified values and types); type annotations; variables.
- **Go-to type definition** — when triggered on an expression, identifies the
  types of all values used in the expression and presents their definitions to
  view and jump to.
- **Find references** — supported for: functions; function arguments;
  constants; types; custom type variants; variables.
- **Code completion** — supported for: function arguments; functions and
  constants defined in other modules (auto-adding import statements if the
  module is not yet imported); functions and constants defined in the same
  module; locally defined variables; modules in import statements; record
  fields; type constructors in type annotations; unqualified types and values
  in import statements.
- **Rename** — supported for: functions; function arguments; constants; types;
  custom type variants; variables.
- **Document symbols** — lists document symbols (functions, constants, etc.)
  for the current Gleam file.
- **Signature help** — shows the type of each argument when calling a
  function, along with the labels of arguments that have them.
- **Code folding** — reports foldable ranges for contiguous import blocks and
  multi-line top-level definitions (function bodies, custom types, constants,
  type aliases).

### Code actions
The server offers a large set of code actions (quick fixes / refactors):
- Add and remove anonymous function wrappers
- Add annotations (type annotations to assignments and functions; also `let`
  and `use` assignments)
- Add missing import
- Add missing patterns (to inexhaustive case expressions)
- Add missing type parameter (to custom type definitions)
- Add omitted labels (to function/record constructor calls)
- Case correction (corrects names written with the wrong case)
- Collapse nested case expressions
- Convert to and from pipe (`|>` <-> regular call)
- Convert to and from `use` (`use` <-> regular call)
- Create unknown module (creates a new empty module for an imported-but-
  nonexistent module)
- Discard unused result (assigns unused results to `_`)
- Expand function capture (function capture syntax -> anonymous function)
- Extract constant
- Extract variable
- Fill labels (adds expected labels to a call)
- Fill unused fields (adds unmatched fields in a pattern)
- Generate decoder (generates a `dynamic` decoder from a custom type)
- Generate function (generates definition of a local function used but not
  yet defined)
- Generate to-JSON function (generates a JSON encoder using `gleam_json`)
- Inexhaustive let to case
- Inline variable (inlines a variable used only once)
- Interpolate string (splits a string to interpolate a value)
- Pattern match (generates an exhaustive case expression for a variable/arg)
- Qualify and unqualify (add/remove module qualifiers for types and values)
- Remove block (removes blocks around single expressions)
- Remove echo (removes `echo` debug expressions)
- Remove opaque from private type
- Remove redundant record update
- Remove redundant tuples (from case subjects and patterns)
- Remove unreachable clauses
- Remove unused imports
- Replace `_` with type (replaces a type hole with the full type)
- Use label shorthand syntax
- Wrap in block

## Editor configuration (VS Code / Neovim / Helix / Emacs)
- **Gram** — supports the language server out-of-the-box; no configuration
  required; auto-starts when a Gleam file is opened.
- **Helix** — supports the language server out-of-the-box; no configuration
  required; auto-starts when a Gleam file is opened.
- **Neovim** — `nvim-lspconfig` includes configuration for Gleam. Install
  `nvim-lspconfig` and add the language server to `init.lua`:
  - Nvim 0.11+ and nvim-lspconfig 2.1+: `vim.lsp.enable('gleam')`
  - Nvim <= 0.10: `require('lspconfig').gleam.setup({})`
  - The language server auto-starts when a Gleam file is opened.
  - With `nvim-treesitter`, run `:TSInstall gleam` for syntax highlighting
    and other tree-sitter features.
- **VS Code** — install the VS Code Gleam plugin
  (https://marketplace.visualstudio.com/items?itemName=Gleam.gleam). The
  language server auto-starts when a Gleam file is opened. If VS Code cannot
  run the language server, ensure the `gleam` binary is on VS Code's PATH and
  consider restarting VS Code.
- **Zed** — when a Gleam file is opened, Zed suggests installing the Gleam
  plugin; once installed the language server auto-starts.
- **Other editors / Emacs** — any editor supporting the Language Server
  Protocol can use the Gleam Language Server. Configure the editor to run
  `gleam lsp` from the root of the workspace. (No Emacs-specific instructions
  are given on the page; Emacs users should use an LSP client such as
  `lsp-mode` or `eglot` configured to run `gleam lsp`.)

## Limitations
- The language server is the newest part of the Gleam toolchain and is rapidly
  improving; it does not yet have all features found in more mature language
  servers for older languages.
- **Project compilation** performs analysis compilation only — code generation
  and Erlang compilation are NOT performed by the language server.
- **Use outside Gleam projects** — the language server is unable to build
  Gleam code that is not in a Gleam project. When such a file is opened the
  language server provides code formatting only; other features are not
  available.
- The target used is the one specified in `gleam.toml`; if none is specified it
  defaults to Erlang.
- No version number is pinned on the page; the LSP ships inside the `gleam`
  binary so its version tracks the installed Gleam version.

## Strict rules
- The language server does NOT perform code generation or compile Erlang or
  Elixir code, so there is no chance of code execution occurring from opening
  a file in an editor using the Gleam language server (security guarantee).
- Unsaved edits in the editor are used when compiling in the language server.
- The target specified in `gleam.toml` is used by the language server;
  defaults to Erlang when unspecified.
- For non-Gleam-project files, only code formatting is available.

## Verbatim quotes
- "The Gleam Language Server is included in the regular gleam binary, so if
  you have Gleam installed then you have the Gleam language server installed."
- "The language server will automatically compile code in Gleam projects opened
  in the editor. Code generation and Erlang compilation are not performed."
- "If any files are edited in the editor but not yet saved then these edited
  versions will be used when compiling in the language server."
- "The target specified in gleam.toml is used by the language server. If no
  target is specified then it defaults to Erlang."
- "The language server does not perform code generation or compile Erlang or
  Elixir code, so there is no chance of any code execution occurring due to
  opening a file in an editor using the Gleam language server."
- "The language server is unable to build Gleam code that are not in Gleam
  projects. When one of these files is opened the language server will provide
  code formatting but other features are not available."
- "Any other editor that supports the Language Server Protocol can use the
  Gleam Language Server. Configure your editor to run gleam lsp from the root
  of your workspace."
- Neovim (Nvim 0.11+ and nvim-lspconfig 2.1+): `vim.lsp.enable('gleam')`
- Neovim (Nvim <= 0.10): `require('lspconfig').gleam.setup({})`

## Version notes
- No Gleam version is stated on the page.
- The LSP is part of the `gleam` binary, so the LSP version equals the
  installed Gleam toolchain version.
- Neovim integration notes reference Nvim 0.11+ / nvim-lspconfig 2.1+ vs
  Nvim <= 0.10 APIs.

## Discovered links
### Relevant (crawl later)
- https://gleam.run/install/  (Gleam installation — prerequisite for the LSP)
- https://gleam.run/documentation/  (Gleam documentation index)
- https://gleam.run/roadmap/  (Gleam roadmap — referenced for LSP work in progress)
- https://github.com/gleam-lang  (Gleam source on GitHub)
- https://github.com/neovim/nvim-lspconfig  (Neovim LSP config)
- https://github.com/nvim-treesitter/nvim-treesitter  (Neovim treesitter)
- https://marketplace.visualstudio.com/items?itemName=Gleam.gleam  (VS Code Gleam plugin)
- https://gram.liten.app/docs/languages/gleam/  (Gram editor Gleam docs)

### Skipped
- https://gleam.run/  (site root)
- https://gleam.run/case-studies
- https://gleam.run/community
- https://gleam.run/news
- https://gleam.run/sponsor
- https://gleam.run/feed.xml
- https://discord.gg/Fm8Pwmy  (Discord invite)
- https://gleamweekly.com/
- https://github.com/gleam-lang/gleam/blob/main/CODE_OF_CONDUCT.md
- https://gleam.run/images/lucy/lucy.svg
- https://gleam.run/styles/main.css?v=Z-ZGfcMyjtIqcShXOGZXFej9jKc-QZ44doz1mPOyzEA
