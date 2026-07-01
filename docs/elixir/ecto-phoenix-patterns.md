# Ecto and Phoenix Architectural Patterns

## Purpose

Provide repo-independent, architectural guidance for future AI agents writing, reviewing, and refactoring Elixir/Phoenix code that uses Ecto. This doc is not tied to any specific project; it describes the patterns that keep contexts, changesets, queries, transactions, and directory structure coherent across Phoenix codebases. Use it when you need to decide where a query belongs, how to validate external params safely, when to reach for `Ecto.Multi`, or how to split business logic from the web layer.

## Sources used

- https://hexdocs.pm/ecto/Ecto.html (PRIMARY)
- https://hexdocs.pm/ecto/Ecto.Changeset.html (PRIMARY)
- https://hexdocs.pm/ecto/Ecto.Query.html
- https://hexdocs.pm/ecto/Ecto.Query.API.html
- https://hexdocs.pm/ecto/Ecto.Multi.html
- https://hexdocs.pm/ecto/Ecto.Repo.html (PRIMARY)
- https://hexdocs.pm/phoenix/contexts.html (PRIMARY)
- https://hexdocs.pm/phoenix/your_first_context.html
- https://hexdocs.pm/phoenix/directory_structure.html (PRIMARY)

This page reflects Ecto v3.14.0 / Phoenix v1.8.8 docs.

## Ecto Overview

Ecto is a toolkit for mapping data from a data store into Elixir structs and back. From [Ecto.html](https://hexdocs.pm/ecto/Ecto.html):

> "Ecto is split into 4 main components: Ecto.Repo, Ecto.Schema, Ecto.Query and Ecto.Changeset."

The four components answer four questions:

| Component | Question it answers | Canonical role |
|---|---|---|
| `Ecto.Repo` | WHERE the data lives | Wrapper around the data store; defined via `use Ecto.Repo` and started in the supervision tree. |
| `Ecto.Schema` | WHAT the data is | Maps external data into Elixir structs; a `schema` block declares tables/columns and types. |
| `Ecto.Query` | HOW TO READ the data | Composable, Elixir-syntax query language built with `from`. |
| `Ecto.Changeset` | HOW TO CHANGE the data | Tracks and validates changes before they reach the database. |

The components are intentionally decoupled. Storage (Repo) and data shape (Schema) are independent: you can call `Repo.all/2` with a string table name and never define a schema. A typical Repo definition is:

```elixir
defmodule MyApp.Repo do
  use Ecto.Repo,
    otp_app: :my_app,
    adapter: Ecto.Adapters.Postgres
end
```

The Repo is started under the application supervisor (see the `application.ex` example in the Directory Structure section). All other Ecto usage flows through the context modules.

A useful mental model of the data-flow stack:

```text
+------------------------------------+
| Web layer (controllers / LiveViews)|  lib/my_app_web/
+------------------------------------+
              |
              v
+------------------------------------+
| Context boundary (public intent API)|  lib/my_app/<context>.ex
+------------------------------------+
              |
    +---------+---------+
    v                   v
+---------+       +-----------+
| Schema  |       | Changeset |  lib/my_app/<context>/<schema>.ex
+---------+       +-----------+
    |                   |
    +---------+---------+
              v
+------------------------------------+
| Query composition / Ecto.Multi     |
+------------------------------------+
              |
              v
+------------------------------------+
| Repo (the only DB boundary)        |  lib/my_app/repo.ex
+------------------------------------+
              |
              v
+------------------------------------+
| Database (Postgres / MySQL / etc.) |
+------------------------------------+
```

## Changesets

Changesets are the canonical gatekeeper for writes. From [Ecto.Changeset.html](https://hexdocs.pm/ecto/Ecto.Changeset.html):

> "Changesets allow filtering, casting, validation and definition of constraints when manipulating structs."

### `cast/4` vs `change/2`

There are two main ways to build a changeset:

| Function | Use for | Filters params? | Converts string keys? |
|---|---|---|---|
| `cast/4` | External data (forms, APIs, CLI) | Yes, against an explicit permitted list | Yes |
| `change/2` | Internal data already typed and trusted | No | No |

From [Ecto.Changeset.html](https://hexdocs.pm/ecto/Ecto.Changeset.html):

> "The cast/4 function is used to receive external parameters from a form, API or command line, and convert them to the types defined in your Ecto.Schema. change/2 is used to modify data directly from your application, assuming the data given is valid and matches the existing types."

```elixir
def changeset(user, params \\ %{}) do
  user
  |> cast(params, [:name, :email, :age])
  |> validate_required([:name, :email])
  |> validate_format(:email, ~r/@/)
  |> unique_constraint(:email)
end
```

During casting, only permitted fields whose values match the schema type are converted to atoms and stored. Everything else is ignored. Useful options include `:trim_values`, `:empty_values`, `:force_changes`, and `:message`.

### Validations vs constraints

| Aspect | Validations (`validate_*`) | Constraints (`*_constraint`) |
|---|---|---|
| Runs before DB? | Yes | At DB time |
| Safety | Best-effort; race conditions possible | Always safe against races |
| Example helpers | `validate_required`, `validate_format`, `validate_length`, `validate_number`, `validate_inclusion`, `validate_exclusion`, `validate_change`, `validate_confirmation`, `validate_acceptance`, `validate_subset` | `unique_constraint`, `foreign_key_constraint`, `check_constraint`, `exclusion_constraint`, `assoc_constraint`, `no_assoc_constraint` |
| Ordering | Always checked first | Checked after validations |

From [Ecto.Changeset.html](https://hexdocs.pm/ecto/Ecto.Changeset.html):

> "Most validations can be executed without a need to interact with the database ... Constraints rely on the database and are always safe. As a consequence, validations are always checked before constraints."

A critical rule: validations only validate changed fields. From [Ecto.Changeset.html](https://hexdocs.pm/ecto/Ecto.Changeset.html):

> "if a field was not given as a parameter, it won't be validated at all."

If you need a field to be present and validated, mark it `validate_required/3`.

### Actions and lifecycle

A single changeset works for both insert and update; the action is determined by the Repo call. For delete, mark the action manually. From [Ecto.Changeset.html](https://hexdocs.pm/ecto/Ecto.Changeset.html):

```elixir
def changeset(comment, %{"delete" => "true"}) do
  %{Ecto.Changeset.change(comment, delete: true) | action: :delete}
end
```

> "we don't call cast/4 in this case because we don't want to prevent deletion if a change is invalid."

Two useful functions for extracting results:

- `apply_changes/1` — returns the underlying data with changes applied regardless of validity.
- `apply_action/2` — returns `{:ok, data}` or `{:error, changeset}` and enforces validity.

### Embeds and schemaless changesets

Embedded schemas store child data alongside the parent instead of a separate table. PostgreSQL uses JSONB/ARRAY columns. Use `embeds_one`/`embeds_many` in the schema and `cast_embed/3` for external data or `put_embed/4` for internal data.

Schemaless changesets are built by passing a `{data, types}` tuple to `cast/4`, useful for plain structs/maps.

### Multiple changeset functions per schema

Do not overload one `changeset/2` for every intent. Define purpose-named functions:

```elixir
defmodule MyApp.Accounts.User do
  use Ecto.Schema
  import Ecto.Changeset

  schema "users" do
    field :email, :string
    field :name, :string
    field :age, :integer
    field :password_hash, :string

    timestamps()
  end

  def registration_changeset(user, attrs) do
    user
    |> cast(attrs, [:email, :name, :age])
    |> validate_required([:email, :name])
    |> validate_format(:email, ~r/@/)
    |> unique_constraint(:email)
  end

  def profile_changeset(user, attrs) do
    user
    |> cast(attrs, [:name, :age])
    |> validate_required([:name])
    |> validate_number(:age, greater_than_or_equal_to: 0)
  end

  def password_changeset(user, attrs) do
    user
    |> cast(attrs, [:password])
    |> validate_required([:password])
    |> validate_length(:password, min: 12)
    # |> put_pass_hash()  # internal step
  end
end
```

## Queries

[Ecto.Query.html](https://hexdocs.pm/ecto/Ecto.Query.html) provides a type-safe, composable query language written in Elixir syntax.

### Keyword vs pipe/macro syntax

The same query can be written two ways:

```elixir
# Keyword syntax
from u in User, where: u.age > 18, select: u.name

# Pipe / macro syntax
"users"
|> where([u], u.age > 18)
|> select([u], u.name)
```

Available keywords include: `:distinct`, `:where`, `:order_by`, `:offset`, `:limit`, `:lock`, `:group_by`, `:having`, `:join`, `:select`, `:preload`.

### Pinning external values

External values must be pinned with `^` so Ecto can parameterize them safely:

```elixir
min = 18
from u in User, where: u.age > ^min
```

Explicit typing is available with `type(^age, :integer)`.

### Nil comparisons

Comparing against `nil` with `==` or `!=` is forbidden in queries; it is a security measure to avoid nil-column traversal attacks. Use `is_nil/1` or `is_not_nil/1`:

```elixir
from u in User, where: is_nil(u.deactivated_at)
```

### Composition and bindings

Queries compose by refining an existing query. Positional binding names do not need to match across refinements:

```elixir
query =
  User
  |> where([u], u.active == true)
  |> order_by([u], desc: u.inserted_at)

# refine later; `u` here is a fresh name for the same binding position
query = from u in query, select: u.email
```

Multi-binding access:

```elixir
from [p, c] in query, select: {p.title, c.body}
```

Wildcard `...` skips intermediate positions:

```elixir
from [p, ..., c] in posts_with_comments, ...
```

Named bindings are declared with `as: :name` and referenced as `from [p, comment: c] in query, ...`. Late binding uses `as(:posts).id` or `parent_as(:posts).id` for subqueries referring to an outer scope.

### Bindless queries

Useful for dynamic query building from keyword lists:

```elixir
from Post,
  where: [category: "fresh"],
  order_by: [desc: :published_at]

# or interpolate keyword lists
from Post, where: ^where, order_by: ^order_by, select: ^select
```

### Joins, fragments, subqueries, CTEs

Joins support qualifiers `:inner`, `:left`, `:right`, `:cross`, `:cross_lateral`, `:full`, `:inner_lateral`, `:left_lateral`. Sources can be schemas, interpolated queries, associations, subqueries, or fragments.

```elixir
from c in Comment,
  join: p in Post, on: p.id == c.post_id,
  select: {p.title, c.text}
```

Use `dynamic/2` for optional filters:

```elixir
import Ecto.Query

def list_users(filters) do
  User
  |> where([u], u.active == true)
  |> maybe_filter_age(filters[:min_age])
  |> maybe_filter_city(filters[:city])
  |> order_by([u], desc: u.inserted_at)
  |> Repo.all()
end

defp maybe_filter_age(query, nil), do: query
defp maybe_filter_age(query, min_age) do
  where(query, [u], u.age >= ^min_age)
end

defp maybe_filter_city(query, nil), do: query
defp maybe_filter_city(query, city) do
  where(query, [u], ilike(u.city, ^"%#{city}%"))
end
```

Fragments are the escape hatch when Ecto does not model a SQL expression. From [Ecto.Query.html](https://hexdocs.pm/ecto/Ecto.Query.html):

> "Ecto is unable to do any type casting when fragments are used"

Compensate with `type/2` when needed:

```elixir
from p in Post, where: fragment("lower(?)", p.title) == ^title
```

Subqueries use `subquery/2`. CTEs use `with_cte/3` and `recursive_ctes/2`.

### Where queries live

Build queries inside context modules, not controllers or LiveViews. The web layer should call `Catalog.list_products/1`, not `Repo.all(from p in Product, ...)`. This keeps query logic testable, reusable, and isolated from HTTP concerns.

## Ecto.Multi and Transactions

`Ecto.Multi` groups multiple Repo operations into a single database transaction. From [Ecto.Multi.html](https://hexdocs.pm/ecto/Ecto.Multi.html):

> "Ecto.Multi is a data structure for grouping multiple Repo operations. Ecto.Multi makes it possible to pack operations that should be performed in a single database transaction and provides a way to introspect the queued operations without actually performing them. Each operation is given a name that is unique and will identify its result in case of either success or failure."

Treat the struct as opaque; use `Ecto.Multi.to_list/1` for introspection.

### Building a Multi

```elixir
alias Ecto.Multi

def reset_password(account, params) do
  Multi.new()
  |> Multi.update(:account, Account.password_reset_changeset(account, params))
  |> Multi.insert(:log, Log.password_reset_changeset(account, params))
  |> Multi.delete_all(:sessions, Ecto.assoc(account, :sessions))
  |> Repo.transact()
end
```

The caller matches on the result shape:

```elixir
case reset_password(account, params) do
  {:ok, %{account: account}} ->
    {:ok, account}

  {:error, :account, changeset, _changes_so_far} ->
    {:error, changeset}
end
```

### Success and failure shapes

- Success: `{:ok, %{name => result, ...}}`
- Failure: `{:error, failed_operation, failed_value, changes_so_far}`

### Core operations

All core operations take `(multi, name, value_or_fn, opts \\ [])`:

| Operation | Use |
|---|---|
| `insert/4` | Insert a struct/changeset |
| `update/4` | Update a changeset |
| `delete/4` | Delete a struct/changeset |
| `insert_or_update/4` | Upsert a changeset |
| `insert_all/5` | Bulk insert |
| `update_all/5` | Bulk update |
| `delete_all/4` | Bulk delete |
| `run/3` / `run/5` | Arbitrary callback `(repo, changes) -> {:ok, value} \| {:error, value}` |
| `all/4`, `one/4`, `exists?/4` | Query operations inside the transaction |
| `put/3` | Pre-computed value; does not touch the DB |
| `error/3` | Explicitly fail the Multi |

Names can be any term as long as they are unique; tuples such as `{:delete_post, 5}` are useful in loops.

### Changeset pre-validation short-circuit

From [Ecto.Multi.html](https://hexdocs.pm/ecto/Ecto.Multi.html):

> "If a Multi contains operations that accept changesets (like insert/4, update/4 or delete/4), they will be checked before starting the transaction. If any changeset has errors, the transaction will not be started and the error will be immediately returned."

Functions-of-changesets bypass this check, so prefer passing changesets directly when possible.

### Multi vs plain `Repo.transact/2`

From [Ecto.Multi.html](https://hexdocs.pm/ecto/Ecto.Multi.html):

> "Ecto.Multi is particularly useful when the set of operations to perform is dynamic. For most other use cases, using regular control flow within Repo.transact(fun) and returning {:ok, result} or {:error, reason} is more straightforward."

A plain transaction example:

```elixir
def transfer_funds(from_id, to_id, amount) do
  Repo.transact(fn ->
    from = Repo.get!(Account, from_id)

    case Account.debit_changeset(from, amount) do
      {:ok, from} ->
        to = Repo.get!(Account, to_id)

        case Account.credit_changeset(to, amount) do
          {:ok, to} ->
            Repo.update!(from)
            Repo.update!(to)
            {:ok, {from, to}}

          {:error, reason} ->
            Repo.rollback(reason)
        end

      {:error, reason} ->
        Repo.rollback(reason)
    end
  end)
end
```

Note: `Repo.transaction/2` is deprecated in favor of `Repo.transact/2`. Use `transact/2` in new code.

## Repos and Data Boundaries

`Ecto.Repo` is the single database boundary. From [Ecto.Repo.html](https://hexdocs.pm/ecto/Ecto.Repo.html), it wraps a data store via an adapter (`Ecto.Adapters.Postgres` by default; `Ecto.Adapters.MyXQL` for MySQL; `Ecto.Adapters.Tds` for MSSQL). It is configured under the `:otp_app` and started in the supervision tree.

### Query API vs schema API

| Family | Representative functions |
|---|---|
| Query API | `all/2`, `one/2`, `one!/2`, `get/3`, `get!/2`, `get_by/3`, `get_by!/3`, `exists?/2`, `aggregate/2-4`, `update_all/3`, `delete_all/2`, `stream/2` |
| Schema API | `insert/2`, `update/2`, `delete/2`, `insert_or_update/2`, `insert_all/3`, `load/2`, `preload/2`, `reload/2` |

### Upserts

`insert/2` supports `:on_conflict` with values such as `:raise` (default), `:nothing`, `:replace_all`, `{:replace_all_except, fields}`, `{:replace, fields}`, keyword updates, or an `Ecto.Query`. Use `:conflict_target` to name the constraint/index.

### Preload

Explicit preload:

```elixir
posts = Repo.all(Post)
Repo.preload(posts, :comments)
```

In-query preload:

```elixir
Repo.all(from p in Post, preload: [:comments])
```

### Streams

`Repo.stream/2` returns a lazy stream. From [Ecto.Repo.html](https://hexdocs.pm/ecto/Ecto.Repo.html):

> "SQL adapters ... can only enumerate a stream inside a transaction"

Default `:max_rows` is 500.

### Transactions and dynamic repos

Use `Repo.transact(fun_or_multi, opts)` as the canonical transaction API. `Repo.transaction/2` is deprecated. `Repo.rollback/1` aborts a transaction. Helpers include `checkout/2`, `in_transaction?/0`, and `checked_out?/0`.

Dynamic repos allow runtime repo swapping via `get_dynamic_repo/0` and `put_dynamic_repo/1`, useful for multi-tenancy or test sharding.

### Telemetry

[Ecto.Repo.html](https://hexdocs.pm/ecto/Ecto.Repo.html) emits `[:ecto, :repo, :init]` on start, and per-query adapter events such as `[:my_app, :repo, :query]` with measurements for idle_time, queue_time, query_time, decode_time, and total_time. Use these events for performance dashboards and slow-query alerts.

### The web layer never calls Repo directly

This is an architectural boundary, not a compiler rule. Controllers, LiveViews, and channels call context functions. Context functions call the Repo. This preserves the ability to swap storage implementations or add cross-cutting concerns without touching web code.

## Phoenix Contexts

Contexts are the public boundary around related data access and validation. From [contexts.html](https://hexdocs.pm/phoenix/contexts.html):

> "the most important part of your web application is often where we encapsulate data access and data validation. We call these modules contexts. They often talk to a database, using Ecto, or APIs, using an HTTP client such as Req."

### Why contexts matter

From [contexts.html](https://hexdocs.pm/phoenix/contexts.html):

> "Our Phoenix controller is the web interface into our greater application. It shouldn't be concerned with the details of how products are fetched from the database or persisted into storage. ... This is great because our business logic and storage details are decoupled from the web layer of our application. If we move to a full-text storage engine later for fetching products instead of a SQL query, our controller doesn't need to be changed."

Contexts group related schemas and expose an intent-named public API. From [contexts.html](https://hexdocs.pm/phoenix/contexts.html):

> "Contexts are also useful to nest resources. ... contexts help you group related schemas, instead of having several dozens of schemas with no insights on how they relate to each other."

### Public API shape

A context usually exposes:

```elixir
MyApp.Catalog.list_products/0
MyApp.Catalog.get_product!/1
MyApp.Catalog.create_product/1
MyApp.Catalog.update_product/2
MyApp.Catalog.delete_product/1
MyApp.Catalog.change_product/2
```

The schema's `changeset/2` is marked `@doc false`. From [contexts.html](https://hexdocs.pm/phoenix/contexts.html):

> "Callers, such as our controller actions, do not access Product.changeset/2 directly. All interaction with our product changesets is done through the public Catalog context."

### Thin context bodies with room for intent

Most context functions are thin wrappers that add intent:

```elixir
defmodule MyApp.Catalog do
  alias MyApp.Catalog.Product
  alias MyApp.Repo

  def list_products do
    Repo.all(Product)
  end

  def get_product!(id), do: Repo.get!(Product, id)

  def create_product(attrs \\ %{}) do
    %Product{}
    |> Product.changeset(attrs)
    |> Repo.insert()
  end

  def update_product(%Product{} = product, attrs) do
    product
    |> Product.changeset(attrs)
    |> Repo.update()
  end

  def delete_product(%Product{} = product) do
    Repo.delete(product)
  end

  def change_product(%Product{} = product, attrs \\ %{}) do
    Product.changeset(product, attrs)
  end
end
```

Contexts can also encapsulate non-trivial logic. The canonical example from [contexts.html](https://hexdocs.pm/phoenix/contexts.html) is a race-condition-safe atomic increment:

```elixir
def inc_page_views(%Product{} = product) do
  {1, [%Product{views: views}]} =
    from(p in Product, where: p.id == ^product.id, select: [:views])
    |> Repo.update_all(inc: [views: 1])

  put_in(product.views, views)
end
```

### Naming and reuse

Context modules use a plural business noun: `Catalog`, `Accounts`, `Orders`, `Identity`. Schemas nest under them: `MyApp.Catalog.Product`, `MyApp.Accounts.User`. From [contexts.html](https://hexdocs.pm/phoenix/contexts.html):

> "Pick a name that is clear and obvious to everyone who works (and might work) in the project."

Contexts are reused across interfaces: controllers, LiveViews, channels, mix tasks, CSV importers, and background jobs all call the same functions.

### Further Phoenix guidance

Phoenix also documents In-context relationships, Async Contexts, and Cross-context dependencies. Treat those as follow-on reading once the basic context boundary is in place.

## Directory Structure

Phoenix splits source into two sibling trees: `lib/my_app` for business logic and `lib/my_app_web` for the web layer. From [directory_structure.html](https://hexdocs.pm/phoenix/directory_structure.html), `mix phx.new` creates them.

### The two-tree rule

- Business files (contexts, schemas, Repo, Mailer, Application) live in `lib/my_app/`.
- Web files (controllers, components, layouts, router, endpoint, telemetry, gettext) live in `lib/my_app_web/`.
- Never place web files in `lib/my_app/`.
- Never place context files in `lib/my_app_web/`.
- Ecto migrations live in `priv/repo/migrations/`.

### Typical layout

```text
lib/
  my_app/
    application.ex       # OTP supervision tree
    repo.ex              # MyApp.Repo
    mailer.ex            # Swoosh mailer
    catalog.ex           # context module
    catalog/
      product.ex         # schema + changesets
    accounts.ex
    accounts/
      user.ex
  my_app_web/
    endpoint.ex
    router.ex
    telemetry.ex
    gettext.ex
    controllers/
      product_controller.ex
    components/
    live/
      product_live/
    layouts/
config/
  config.exs
  dev.exs
  test.exs
  prod.exs
  runtime.exs
priv/
  repo/
    migrations/
  static/
assets/
test/
  my_app/
    catalog_test.exs
  my_app_web/
    controllers/
      product_controller_test.exs
  support/
    fixtures.ex
```

### Supervision tree

From [directory_structure.html](https://hexdocs.pm/phoenix/directory_structure.html):

```elixir
children = [
  HelloWeb.Telemetry,
  Hello.Repo,
  {Phoenix.PubSub, name: Hello.PubSub},
  HelloWeb.Endpoint
]
```

Children start in declaration order and stop in reverse order. The Repo must be available before endpoint requests can use it.

## Review checklist

- [ ] No `MyApp.Repo` calls exist in `lib/my_app_web/`.
- [ ] Controllers and LiveViews call context functions only.
- [ ] Every `cast/4` lists an explicit permitted-fields list.
- [ ] Every unique database index has a matching `unique_constraint/3` in the relevant changeset.
- [ ] Queries are built in the context module, not in controllers or LiveViews.
- [ ] External params reach the database only through `cast/4` or `Ecto.Multi` operations that accept changesets.
- [ ] `validate_required/3` is used when a field must be present regardless of whether it changed.
- [ ] `nil` comparisons in queries use `is_nil/1` or `is_not_nil/1`, never `== nil` or `!= nil`.
- [ ] Transactions use `Repo.transact/2`, not the deprecated `Repo.transaction/2`.
- [ ] `Ecto.Multi` operations have unique names and are matched with the correct success/failure shape.
- [ ] Schema files live under the matching context directory (`lib/my_app/<context>/<schema>.ex`).
- [ ] Migrations are in `priv/repo/migrations/`.
- [ ] The schema's `changeset/2` is marked `@doc false` and is not called directly from web code.
- [ ] Preloads are explicit (either `Repo.preload/3` or in-query `preload:`), avoiding N+1 queries.
- [ ] `Repo.stream/2` is consumed inside a transaction.
- [ ] Context names are plural business nouns (`Catalog`, `Accounts`).
- [ ] Fragments are used with parameter pinning (`^`) and `type/2` when casting matters.

## Implementation checklist

- [ ] Identify the context for the new feature; create or extend `lib/my_app/<context>.ex`.
- [ ] Place the schema at `lib/my_app/<context>/<schema>.ex`.
- [ ] Define one or more purpose-named changeset functions in the schema.
- [ ] Expose `list_/get_/create_/update_/delete_` functions plus a `change_/2` for forms.
- [ ] Generate the migration in `priv/repo/migrations/` and keep migration names timestamped.
- [ ] Use `cast/4` at every param boundary; prefer `change/2` only for trusted internal data.
- [ ] Add `validate_required/3`, format/length/number validations, and matching `*_constraint` helpers.
- [ ] Build reusable queries as private functions in the context; expose them through intent-named public functions.
- [ ] Use `Ecto.Multi` when the operation set is dynamic or spans multiple tables; otherwise use `Repo.transact/2`.
- [ ] Match Multi results with `{:ok, %{...}}` and `{:error, failed_operation, failed_value, _changes}`.
- [ ] Wire the new supervision children into `lib/my_app/application.ex` if needed (e.g., a new GenServer).
- [ ] Add tests under `test/my_app/` for context behavior and `test/my_app_web/` for HTTP/LiveView behavior.
- [ ] Add telemetry handlers or logging for slow queries if operational visibility is required.

## Validation hooks

- `mix format --check-formatted` — ensures consistent formatting across context, schema, and controller files.
- `mix credo` — flags cross-boundary issues, overly complex functions, and non-idiomatic naming.
- `mix ecto.create` and `mix ecto.migrate` — verify migrations run cleanly in the target environment.
- `mix test` — exercises contexts and controllers; tests use `Ecto.Adapters.SQL.Sandbox` for transactional isolation.
- `mix phx.routes` — sanity-checks that routes point to existing controllers/actions.
- Telemetry event inspection — subscribe to `[:my_app, :repo, :query]` to measure idle/queue/query/decode/total time and catch N+1 or slow queries.
- Credo custom checks — add rules that forbid `MyApp.Repo` calls in `lib/my_app_web/` or require context prefixes.

## Examples

### 1. Full context + schema + controller slice

Schema:

```elixir
defmodule MyApp.Catalog.Product do
  use Ecto.Schema
  import Ecto.Changeset

  schema "products" do
    field :title, :string
    field :description, :string
    field :price, :decimal
    field :views, :integer, default: 0

    timestamps()
  end

  @doc false
  def changeset(product, attrs) do
    product
    |> cast(attrs, [:title, :description, :price])
    |> validate_required([:title, :price])
    |> validate_number(:price, greater_than: 0)
  end
end
```

Context:

```elixir
defmodule MyApp.Catalog do
  import Ecto.Query
  alias MyApp.Catalog.Product
  alias MyApp.Repo

  def list_products do
    Repo.all(Product)
  end

  def get_product!(id), do: Repo.get!(Product, id)

  def create_product(attrs \\ %{}) do
    %Product{}
    |> Product.changeset(attrs)
    |> Repo.insert()
  end

  def update_product(%Product{} = product, attrs) do
    product
    |> Product.changeset(attrs)
    |> Repo.update()
  end

  def delete_product(%Product{} = product) do
    Repo.delete(product)
  end

  def change_product(%Product{} = product, attrs \\ %{}) do
    Product.changeset(product, attrs)
  end
end
```

Controller:

```elixir
defmodule MyAppWeb.ProductController do
  use MyAppWeb, :controller

  alias MyApp.Catalog

  def index(conn, _params) do
    products = Catalog.list_products()
    render(conn, :index, products: products)
  end

  def create(conn, %{"product" => product_params}) do
    case Catalog.create_product(product_params) do
      {:ok, product} ->
        conn
        |> put_flash(:info, "Product created.")
        |> redirect(to: ~p"/products/#{product}")

      {:error, %Ecto.Changeset{} = changeset} ->
        render(conn, :new, changeset: changeset)
    end
  end
end
```

### 2. Changeset with cast, validations, and constraints

```elixir
defmodule MyApp.Accounts.User do
  use Ecto.Schema
  import Ecto.Changeset

  schema "users" do
    field :email, :string
    field :name, :string
    field :age, :integer

    timestamps()
  end

  def registration_changeset(user, attrs) do
    user
    |> cast(attrs, [:email, :name, :age])
    |> validate_required([:email, :name])
    |> validate_format(:email, ~r/@/)
    |> validate_length(:name, min: 2, max: 100)
    |> validate_number(:age, greater_than_or_equal_to: 13)
    |> unique_constraint(:email)
  end
end
```

### 3. Composable query with optional dynamic filters

```elixir
defmodule MyApp.Catalog do
  import Ecto.Query
  alias MyApp.Catalog.Product
  alias MyApp.Repo

  def list_products(filters \\ %{}) do
    Product
    |> filter_published(filters[:published])
    |> filter_min_price(filters[:min_price])
    |> order_by([p], desc: p.inserted_at)
    |> Repo.all()
  end

  defp filter_published(query, nil), do: query
  defp filter_published(query, true), do: where(query, [p], not is_nil(p.published_at))
  defp filter_published(query, false), do: where(query, [p], is_nil(p.published_at))

  defp filter_min_price(query, nil), do: query
  defp filter_min_price(query, min) when is_number(min) do
    where(query, [p], p.price >= ^min)
  end
end
```

### 4. Ecto.Multi workflow with success/error handling

```elixir
defmodule MyApp.Orders do
  alias Ecto.Multi
  alias MyApp.Repo
  alias MyApp.Orders.{Order, OrderLine}
  alias MyApp.Inventory

  def place_order(user, attrs) do
    Multi.new()
    |> Multi.insert(:order, Order.changeset(%Order{}, attrs))
    |> Multi.merge(fn %{order: order} ->
      Enum.reduce(order.lines, Multi.new(), fn line, multi ->
        Multi.insert(multi, {:line, line.id}, OrderLine.changeset(order, line))
      end)
    end)
    |> Multi.run(:inventory, fn _repo, %{order: order} ->
      Inventory.reserve(order)
    end)
    |> Repo.transact()
    |> case do
      {:ok, %{order: order}} ->
        {:ok, order}

      {:error, :order, changeset, _changes} ->
        {:error, changeset}

      {:error, failed_op, failed_value, _changes} ->
        {:error, {failed_op, failed_value}}
    end
  end
end
```

### 5. Preload example

```elixir
# Explicit preload after fetch
posts = Repo.all(Post)
posts_with_comments = Repo.preload(posts, :comments)

# In-query preload
posts_with_comments = Repo.all(from p in Post, preload: [:comments])

# Nested preload
posts_with_comments_and_authors =
  Repo.all(from p in Post, preload: [comments: :author])
```

## Common mistakes

- Calling `MyApp.Repo` directly from a controller or LiveView instead of going through a context.
- Building `from` queries in the web layer rather than inside a context module.
- Using `change/2` for external params or `cast/4` for trusted internal data.
- Forgetting `unique_constraint/3` for a unique database index, creating a race-condition window.
- Comparing against `nil` with `== nil` or `!= nil` in a query instead of `is_nil/1`.
- Using the deprecated `Repo.transaction/2` instead of `Repo.transact/2`.
- Placing a schema file under `lib/my_app_web/`.
- Writing a single monolithic `changeset/2` overloaded for every intent.
- Calling `Product.changeset/2` directly from a controller instead of `Catalog.change_product/2`.
- Streaming with `Repo.stream/2` outside a transaction.
- String-concatenating SQL instead of using `fragment/1` plus pinned parameters.
- Ignoring that `validate_*` functions skip fields that were not supplied.
- Forgetting to match the `{:error, failed_operation, failed_value, changes_so_far}` shape from `Ecto.Multi`.
- Preloading associations in a loop, causing N+1 queries.
- Using `Repo.get!/2` when a `nil` result should produce a 404 from the context, not an exception.
- Duplicating operation names inside an `Ecto.Multi`.

## Strict vs contextual guidance

### Strict

- `nil` comparison in queries is forbidden by Ecto; use `is_nil/1` or `is_not_nil/1`.
- Bang variants (`get!`, `one!`, etc.) raise on failure; non-bang variants return `{:ok, _}` / `{:error, _}` or `nil`.
- `Ecto.Multi` checks changeset validity before starting the transaction and short-circuits if any changeset is invalid.
- `Repo.stream/2` must be enumerated inside a transaction for SQL adapters.
- `Repo.transaction/2` is deprecated; `Repo.transact/2` is the canonical API.
- `cast/4` filters parameters against the permitted list and converts string keys; fields not in the list are silently dropped.
- Schema modules must live under the matching context directory, not under `lib/my_app_web/`.

### Conventions (not enforced)

- Use `cast/4` at every external param boundary.
- Access the Repo only through context modules.
- Name contexts with plural business nouns (`Catalog`, `Accounts`, `Orders`).
- Mark schema `changeset/2` functions `@doc false` and route callers through the context.
- Use named bindings in reusable query helpers to keep refinements readable.
- Match `Ecto.Multi` results with both `{:ok, %{...}}` and `{:error, _, _, _}` clauses.
- Commit `mix.lock` and run dependency/security audits in CI.

### Contextual tradeoffs

- Keyword syntax vs pipe syntax for queries: both compile to the same query; choose the style that reads best for the team.
- `Ecto.Multi` vs plain `Repo.transact/2`: prefer Multi when the operation set is dynamic; otherwise use a plain transaction for clarity.
- Embedded schema vs associated schema: embeds keep related data in the same row; associations normalize it. Choose based on query patterns and lifecycle.
- `get_by/3` vs custom query: use `get_by` for simple key lookups; compose a query when filtering is non-trivial.
- One changeset vs many changeset functions: start with one per schema, split into intent-named functions as validation rules diverge.
- `fragment/1` escape hatch: use it when Ecto does not model an expression, but prefer native query API when possible.

## Policy decisions for individual repos

- Which context boundaries and names apply to each domain area.
- Whether to allow cross-context calls directly or require an anti-corruption layer / dedicated boundary module.
- Whether the deprecated `Repo.transaction/2` is still permitted during a migration period.
- Which lint stack to enforce (`mix format --check-formatted`, `mix credo`, custom Credo checks).
- Telemetry event naming and prefix conventions (e.g., `[:my_app, :repo, :query]`).
- Whether to use embedded schemas or join tables for one-to-few relationships.
- Testing sandbox strategy: shared vs ownership-based checkout, async test boundaries.
- Whether controllers may use bang context functions (`get_product!/1`) or must handle `nil` via `get_product/1`.

## Related docs

- `docs/elixir/naming-conventions.md` — module, function, and atom naming.
- `docs/elixir/mix-project-structure.md` — Mix project layout and umbrella projects.
- `docs/elixir/configuration-and-runtime.md` — environment-specific configuration and runtime concerns.
- `docs/elixir/testing-exunit.md` — testing with ExUnit and Ecto.Adapters.SQL.Sandbox.
- `docs/elixir/error-handling.md` — `:ok`/`:error` tuple patterns and exceptions.
- `docs/elixir/dependencies-and-packages.md` — dependency hygiene and Hex auditing.

## Related skills

- None defined yet.
