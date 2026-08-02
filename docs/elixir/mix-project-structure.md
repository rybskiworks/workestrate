# Mix, Project Structure, and Build Tooling

## Purpose

Describe how Mix projects are organized, configured, and built in this repo. Future agents who create, review, refactor, or debug Elixir projects should follow these rules so project structure is consistent, predictable, and aligned with the official Mix and Elixir documentation.

## Sources used

- https://hexdocs.pm/mix/Mix.html (PRIMARY)
- https://hexdocs.pm/mix/Mix.Project.html (PRIMARY)
- https://hexdocs.pm/mix/Mix.Task.html
- https://hexdocs.pm/mix/Mix.Tasks.Help.html
- https://hexdocs.pm/mix/Mix.Tasks.New.html
- https://hexdocs.pm/mix/Mix.Tasks.Compile.App.html
- https://hexdocs.pm/elixir/introduction-to-mix.html
- https://hexdocs.pm/elixir/Config.html
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/new.ex
- https://hexdocs.pm/mix/Mix.Tasks.Format.html (PRIMARY)
- https://hexdocs.pm/elixir/Code.html
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/format.ex
- https://hexdocs.pm/mix/Mix.Tasks.Deps.html (PRIMARY)
- https://hexdocs.pm/mix/Mix.Tasks.Deps.Get.html
- https://hexdocs.pm/mix/Mix.Tasks.Deps.Clean.html
- https://hexdocs.pm/mix/Mix.Tasks.Deps.Compile.html
- https://hexdocs.pm/mix/Mix.Tasks.Deps.Tree.html
- https://hexdocs.pm/mix/Mix.Tasks.Deps.Unlock.html
- https://hexdocs.pm/mix/Mix.Tasks.Deps.Update.html
- https://hexdocs.pm/mix/Mix.SCM.html
- https://hexdocs.pm/elixir/Version.html
- https://hexdocs.pm/mix/Mix.Tasks.Release.html (PRIMARY)
- https://hexdocs.pm/mix/Mix.Tasks.Release.Init.html
- https://hexdocs.pm/mix/Mix.Release.html
- https://hexdocs.pm/elixir/Config.Provider.html
- https://hexdocs.pm/mix/Mix.Tasks.Test.html
- https://hexdocs.pm/mix/Mix.Tasks.Clean.html
- https://hexdocs.pm/mix/Mix.Tasks.Run.html
- https://hexdocs.pm/mix/Mix.Tasks.Cmd.html
- https://hexdocs.pm/mix/Mix.Tasks.Do.html
- https://hexdocs.pm/mix/Mix.Tasks.Xref.html
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix.ex
- https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/task.ex
- https://hexdocs.pm/mix/Mix.Tasks.Archive.html
- https://hexdocs.pm/mix/Mix.Tasks.Archive.Install.html
- https://hexdocs.pm/mix/Mix.Tasks.Archive.Uninstall.html
- https://hexdocs.pm/mix/Mix.Tasks.Archive.Build.html
- https://hexdocs.pm/mix/Mix.Tasks.Local.html
- https://hexdocs.pm/mix/Mix.Tasks.Local.Hex.html
- https://hexdocs.pm/mix/Mix.Tasks.Local.Rebar.html
- https://hexdocs.pm/mix/Mix.Tasks.Escript.Build.html
- https://hexdocs.pm/elixir/optional-syntax.html
- https://hexdocs.pm/elixir/sigils.html
- https://hexdocs.pm/elixir/OptionParser.html
- https://www.erlang.org/doc/apps/kernel/code.html

This page reflects Mix v1.20.2 / Elixir v1.20.2 docs.

## Mix Overview

From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Mix is a build tool that provides tasks for creating, compiling, and testing Elixir projects, managing its dependencies, and more."

> "The foundation of Mix is a project. A project can be defined by using `Mix.Project` in a module, usually placed in a file named `mix.exs`"

A minimal Mix project looks like this:

```elixir
defmodule MyApp.MixProject do
  use Mix.Project

  def project do
    [
      app: :my_app,
      version: "1.0.0"
    ]
  end
end
```

Once a project is defined, the default tasks can be run from the command line. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Once the project is defined, a number of default Mix tasks can be run directly from the command line:
> - `mix compile` - compiles the current project
> - `mix test` - runs tests for the given project
> - `mix run` - runs a particular command inside the project"

> "Each task has its own options and sometimes specific configuration to be defined in the `project/0` function. You can use `mix help` to list all available tasks and `mix help NAME` to show help for a particular task."

The `mix help` task supports several forms. From [Mix.Tasks.Help.html](https://hexdocs.pm/mix/Mix.Tasks.Help.html):

> "$ mix help                  - prints all aliases, tasks and their short descriptions
> $ mix help --search PATTERN - prints all tasks and aliases that contain PATTERN in the name
> $ mix help --names          - prints all task names and aliases (useful for autocompletion)
> $ mix help --aliases        - prints all aliases
> $ mix help TASK/ALIAS       - prints full docs for the given task/alias
> $ mix help MODULE           - prints the documentation for the given module
> $ mix help MODULE.FUN       - prints the documentation for the given module+function
> $ mix help app:APP          - prints a summary of all public modules in application
> $ mix help c:MODULE.NAME    - prints the documentation for the given callback
> $ mix help t:MODULE.NAME    - prints the documentation for the given type"

### Writing custom Mix tasks

Custom tasks are defined by implementing the `Mix.Task` behaviour. From [Mix.Task.html](https://hexdocs.pm/mix/Mix.Task.html):

> "To create a new Mix task, you'll need to:
> 1. Create a module whose name begins with `Mix.Tasks.` (for example, `Mix.Tasks.MyTask`).
> 2. Call `use Mix.Task` in that module.
> 3. Implement the `Mix.Task` behaviour in that module (that is, implement the `run/1` callback).
>
> Typically, task modules live inside the `lib/mix/tasks/` directory, and their file names use dot separators instead of underscores (for example, `deps.clean.ex`) - although ultimately the file name is not relevant."

The command name derives from the module name. From [Mix.Task.html](https://hexdocs.pm/mix/Mix.Task.html):

> "The command name will correspond to the portion of the module name following `Mix.Tasks.`. For example, a module name of `Mix.Tasks.Deps.Clean` corresponds to a task name of `deps.clean`."

> "The `run/1` function will receive a list of all command line arguments passed, according to the user's terminal."

A minimal custom task:

```elixir
# lib/mix/tasks/echo.ex
defmodule Mix.Tasks.Echo do
  @moduledoc "Printed when the user requests `mix help echo`"
  @shortdoc "Echoes arguments"

  use Mix.Task

  @impl Mix.Task
  def run(args) do
    Mix.shell().info(Enum.join(args, " "))
  end
end
```

Task attributes control visibility and behaviour. From [Mix.Task.html](https://hexdocs.pm/mix/Mix.Task.html):

> "Define the `@shortdoc` attribute if you wish to make the task publicly visible on `mix help`. Omit this attribute if you do not want your task to be listed via `mix help`."

> "The `@moduledoc` attribute may override `@shortdoc`. The task will not appear in `mix help` if documentation for the entire module is hidden with `@moduledoc false`."

> "Set `@recursive true` if you want the task to run on each umbrella child in an umbrella project."

> "Sets the preferred Mix environment for this task. For example, if your task is meant to be used for testing, you could set `@preferred_cli_env :test`."

### Environments and targets

Mix supports environments and targets. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Mix supports different environments. Environments allow developers to prepare and organize their project specifically for different scenarios. By default, Mix provides three environments:
> - `:dev` - the default environment
> - `:test` - the environment `mix test` runs on
> - `:prod` - the environment your dependencies run on
>
> The environment can be changed via the command line by setting the `MIX_ENV` environment variable, for example:
> $ MIX_ENV=prod mix run server.exs"

From [introduction-to-mix.html](https://hexdocs.pm/elixir/introduction-to-mix.html):

> "Mix will default to the `:dev` environment, except for the `test` task that will default to the `:test` environment. The environment can be changed via the `MIX_ENV` environment variable:
> $ MIX_ENV=prod mix compile"

> "Mix in production: Mix is a build tool and, as such, it is not expected to be available in production. Therefore, it is recommended to access `Mix.env/0` only in configuration files and inside `mix.exs`, never in your application code (`lib`)."

Use `Mix.env/0` to read the current environment. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Returns the current Mix environment.
>
> This function should not be used at runtime in application code (as opposed to infrastructure and build code like Mix tasks). Mix is a build tool and may not be available after the code is compiled (for example in a release).
>
> To differentiate the program behavior depending on the environment, it is recommended to use application environment through `Application.get_env/3`."

Targets work similarly. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Besides environments, Mix supports targets. Targets are useful when a project needs to compile to different architectures and some of the dependencies are only available to some of them. By default, the target is `:host` but it can be set via the `MIX_TARGET` environment variable."

> "The target can be read via `Mix.target/0`."

Use `Mix.env/1` and `Mix.target/1` only when necessary. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Changes the current Mix environment to `env`. Be careful when invoking this function as any project configuration won't be reloaded."

> "Changes the current Mix target to `target`."

### Compilers

Mix uses a default set of compilers. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Returns the default compilers used by Mix.
> It can be used in your `mix.exs` to prepend or append new compilers to Mix:"

```elixir
def project do
  [compilers: Mix.compilers() ++ [:foo, :bar]]
end
```

The default compiler list is `[:erlang, :elixir, :app]`.

### Aliases

Aliases are project-specific shortcuts. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Aliases are shortcuts or tasks specific to the current project."

```elixir
defp aliases do
  [
    c: "compile",
    hello: &hello/1,
    paid_task: &paid_task/1
  ]
end
```

> "If the alias is overriding an existing task, the arguments given to the alias will be forwarded to the original task in order to preserve semantics. Otherwise arguments given to the alias are appended to the arguments of the last task in the list."

> "Finally, aliases defined in the current project do not affect its dependencies and aliases defined in dependencies are not accessible from the current project, with the exception of umbrella projects."

## Mix.Project and mix.exs

The `mix.exs` file is the heart of a Mix project. From [introduction-to-mix.html](https://hexdocs.pm/elixir/introduction-to-mix.html):

```elixir
defmodule KV.MixProject do
  use Mix.Project

  def project do
    [
      app: :kv,
      version: "0.1.0",
      elixir: "~> 1.11",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  # Run "mix help compile.app" to learn about applications
  def application do
    [
      extra_applications: [:logger]
    ]
  end

  # Run "mix help deps" to learn about dependencies
  defp deps do
    [
      # {:dep_from_hexpm, "~> 0.3.0"},
      # {:dep_from_git, git: "https://github.com/elixir-lang/my_dep.git", tag: "0.1.0"},
    ]
  end
end
```

> "Our `mix.exs` defines two public functions: `project`, which returns project configuration like the project name and version, and `application`, which is used to generate an application file.
> There is also a private function named `deps`, which is invoked from the `project` function, that defines our project dependencies. Defining `deps` as a separate function is not required, but it helps keep the project configuration tidy."

Using `Mix.Project` wires the module into Mix. From [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html):

> "When you `use Mix.Project`, it notifies Mix that a new project has been defined, so all Mix tasks use your module as a starting point."

> "In order to configure Mix, the module that calls `use Mix.Project` should export a `project/0` function that returns a keyword list representing configuration for the project.
> This configuration can be read using `Mix.Project.config/0`. Note that `config/0` won't fail if a project is not defined; this allows many Mix tasks to work without a project.
> If a task requires a project to be defined or needs to access a special function within the project, the task can call `Mix.Project.get!/0` which fails with `Mix.NoProjectError` in the case a project is not defined.
> There isn't a comprehensive list of all the options that can be returned by `project/0` since many Mix tasks define their own options that they read from this configuration."

### Common project/0 options

The following options are documented in [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html):

| Option | Purpose | Default |
|---|---|---|
| `:app` | OTP application name as an atom | required for `.app` generation |
| `:version` | Project version string | required for `.app` generation |
| `:elixir` | Elixir version requirement, e.g. `"~> 1.11"` | none |
| `:start_permanent` | Whether `:start_permanent` is true in `:prod` | `false` |
| `:deps` | List of dependencies | `[]` |
| `:deps_path` | Directory where dependencies are stored | `"deps"` |
| `:lockfile` | Lockfile used by `mix deps.*` | `"mix.lock"` |
| `:build_path` | Build output directory | `"_build"` |
| `:build_per_environment` | If `true`, build per `Mix.env/0`; if `false`, use `_build/shared` | `true` |
| `:config_path` | Path to main config file | `"config/config.exs"` |
| `:elixirc_paths` | Paths compiled by the `:elixir` compiler | `["lib"]` |
| `:compilers` | Compiler list | `[:erlang, :elixir, :app]` |
| `:apps_path` | Umbrella apps directory | `"apps"` |
| `:aliases` | Task aliases | `[]` |
| `:consolidate_protocols` | Consolidate protocols | `true` |
| `:prune_code_paths` | Prune code paths | `true` |
| `:language` | Project language (`:elixir` or `:erlang`) | `:elixir` |

From [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html):

> ":build_per_environment — if `true`, builds will be *per-environment*. If `false`, builds will go in `_build/shared` regardless of the Mix environment. Defaults to `true`."

> ":aliases — a list of task aliases. ... Defaults to `[]`."

> ":config_path — a string representing the path of the main config file. See `config_files/0` for more information. Defaults to `"config/config.exs"`."

> ":deps — a list of dependencies of this project. Refer to the documentation for the `Mix.Tasks.Deps` task for more information. Defaults to `[]`."

> ":deps_path — directory where dependencies are stored. Also see `deps_path/1`. Defaults to `"deps"`."

> ":lockfile — the name of the lockfile used by the `mix deps.*` family of tasks. Defaults to `"mix.lock"`."

Keep `project/0` fast. From [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html):

> "Keep `project/0` fast. `project/0` is called whenever your `mix.exs` is loaded, so heavy computation should be avoided. If a task requires a potentially complex configuration value, it should allow its configuration to be set to an anonymous function or similar, so that it can be invoked only when needed by the task itself."

### application/0 options

The `application/0` function controls OTP application metadata. From [Mix.Tasks.Compile.App.html](https://hexdocs.pm/mix/Mix.Tasks.Compile.App.html):

> "The most commonly used keys are:
> - `:extra_applications` - a list of OTP applications your application depends on which are not included in `:deps` (usually defined in `deps/0` in your `mix.exs`). For example, here you can declare a dependency on applications that ship with Erlang/OTP or Elixir, like `:crypto` or `:logger`. Optional extra applications can be declared as a tuple, such as `{:ex_unit, :optional}`. Mix guarantees all non-optional applications are started before your application starts.
> - `:registered` - the name of all registered processes in the application. If your application defines a local GenServer with name `MyServer`, it is recommended to add `MyServer` to this list.
> - `:env` - the default values for the application environment."

Example:

```elixir
def application do
  [
    extra_applications: [:logger, :crypto, ex_unit: :optional],
    env: [key: :value],
    registered: [MyServer]
  ]
end
```

> "Other options include:
> - `:applications` - all applications your application depends on at runtime. By default, this list is automatically inferred from your dependencies.
> - `:mod` - specifies a module to invoke when the application is started. It must be in the format `{Mod, args}` where args is often an empty list. The module specified must implement the callbacks defined by the `Application` module.
> - `:start_phases` - specifies a list of phases and their arguments to be called after the application is started.
> - `:included_applications` - specifies a list of applications that will be included in the application."

### Reading project configuration

`Mix.Project.config/0` returns the merged project configuration. From [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html):

> "Returns the project configuration.
> If there is no project defined, it still returns a keyword list with default values. This allows many Mix tasks to work without the need for an underlying project.
> Note that this configuration is cached once the project is pushed onto the stack. Calling it multiple times won't cause it to be recomputed.
> Do not use `Mix.Project.config/0` to find the runtime configuration. Use it only to configure aspects of your project (like compilation directories) and not your application runtime."

### .app file generation

Mix generates the OTP `.app` file automatically. From [Mix.Tasks.Compile.App.html](https://hexdocs.pm/mix/Mix.Tasks.Compile.App.html):

> "A `.app` file is a file containing Erlang terms that defines your application. Mix automatically generates this file based on your `mix.exs` configuration."

> "In order to generate the `.app` file, Mix expects your project to have both the `:app` and `:version` keys."

### Build path layout

Build paths are derived from project configuration. From [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html):

```elixir
Mix.Project.compile_path()
#=> "/path/to/project/_build/dev/lib/my_app/ebin"
Mix.Project.app_path()
#=> "/path/to/project/_build/shared/lib/my_app"
Mix.Project.build_path()
#=> "/path/to/project/_build/shared"
```

> "The build path is built based on the `:build_path` configuration (which defaults to `"_build"`) and a subdirectory. The subdirectory is built based on two factors:
> - If `:build_per_environment` is set (the default), the subdirectory is the value of `Mix.env/0` (which can be set via `MIX_ENV`). Otherwise it is set to \"shared\".
> - If `Mix.target/0` is set (often via the `MIX_TARGET` environment variable), it will be used as a prefix to the subdirectory."

### CLI configuration

Projects can declare default environments, targets, and task preferences. From [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html):

> "The following CLI configuration are available:
> - `:default_env` - the default environment to use when none is given and `MIX_ENV` is not set
> - `:default_target` - the default target to use when none is given and `MIX_TARGET` is not set
> - `:default_task` - the default task to invoke when none is given
> - `:preferred_envs` - a keyword list of `{task, env}` tuples where `task` is the task name as an atom (for example, `:\"deps.get\"`) and `env` is the preferred environment (for example, `:test`)
> - `:preferred_targets` - a keyword list of `{task, target}` tuples"

Example:

```elixir
def cli do
  [
    default_env: :local,
    preferred_envs: [docs: :docs]
  ]
end
```

## Project Directory Structure

### Creating a new project

Use `mix new` to scaffold a project. From [Mix.Tasks.New.html](https://hexdocs.pm/mix/Mix.Tasks.New.html):

> "A project at the given PATH will be created. The application name and module name will be retrieved from the path, unless `--module` or `--app` is given.
> An `--app` option can be given in order to name the OTP application for the project.
> A `--module` option can be given in order to name the modules in the generated code skeleton.
> A `--sup` option can be given to generate an OTP application skeleton including a supervision tree. Normally an app is generated without a supervisor and without the app callback.
> An `--umbrella` option can be given to generate an umbrella project."

> "Examples:
> $ mix new hello_world        # equivalent to: $ mix new hello_world --module HelloWorld
> $ mix new hello_world --sup  # generate an app with a supervision tree and an application callback
> $ mix new hello_world --umbrella  # then cd hello_world/apps; mix new child_app"

### Standard project layout

Running `mix new my_app` produces the following layout, confirmed by [introduction-to-mix.html](https://hexdocs.pm/elixir/introduction-to-mix.html) and the [mix.new source](https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/new.ex):

```text
my_app/
├── .formatter.exs
├── .gitignore
├── README.md
├── mix.exs
├── lib/
│   └── my_app.ex
└── test/
    ├── test_helper.exs
    └── my_app_test.exs
```

> **Note:** A `config/config.exs` is **not** generated by `mix new` for non-umbrella projects in v1.20.2 — only umbrella projects get one. `mix.lock` is created on first `mix deps.get`, not at `mix new` time.

The generated `mix.exs`:

```elixir
defmodule MyApp.MixProject do
  use Mix.Project

  def project do
    [
      app: :my_app,
      version: "0.1.0",
      elixir: "~> 1.20",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger]
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      # {:dep_from_hexpm, "~> 0.3.0"},
      # {:dep_from_git, git: "https://github.com/elixir-lang/my_dep.git", tag: "0.1.0"}
    ]
  end
end
```

The generated `lib/my_app.ex`:

```elixir
defmodule MyApp do
  @moduledoc """
  Documentation for `MyApp`.
  """

  @doc """
  Hello world.

  ## Examples

      iex> MyApp.hello()
      :world

  """
  def hello do
    :world
  end
end
```

The generated test files:

```elixir
# test/my_app_test.exs
defmodule MyAppTest do
  use ExUnit.Case
  doctest MyApp

  test "greets the world" do
    assert MyApp.hello() == :world
  end
end
```

```elixir
# test/test_helper.exs
ExUnit.start()
```

The generated `.formatter.exs`:

```elixir
# Used by "mix format"
[
  inputs: ["{mix,.formatter}.exs", "{config,lib,test}/**/*.{ex,exs}"]
]
```

The generated `.gitignore`:

```gitignore
# The directory Mix will write compiled artifacts to.
/_build/

# If you run "mix test --cover", coverage assets end up here.
/cover/

# The directory Mix downloads your dependencies sources to.
/deps/

# Where third-party dependencies like ExDoc output generated docs.
/doc/

# Temporary files, for example, from tests.
/tmp/

# If the VM crashes, it generates a dump, let's ignore it too.
erl_crash.dump

# Also ignore archive artifacts (built via "mix archive.build").
*.ez

# Ignore package tarball (built via "mix hex.build").
my_app-*.tar
```

### Supervision tree option

With `--sup`, `mix new` adds an application callback module. From [Mix.Tasks.New.html](https://hexdocs.pm/mix/Mix.Tasks.New.html):

```elixir
defmodule MyApp.Application do
  # See https://hexdocs.pm/elixir/Application.html
  # for more information on OTP Applications
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      # Starts a worker by calling: MyApp.Worker.start_link(arg)
      # {MyApp.Worker, arg}
    ]

    opts = [strategy: :one_for_one, name: MyApp.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
```

With `--sup`, the `application/0` function in `mix.exs` also gains `mod: {MyApp.Application, []}`.

### Umbrella projects

Umbrella projects group multiple applications. From [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html):

> "Umbrella projects are a convenience to help you organize and manage multiple applications. While it provides a degree of separation between applications, those applications are not fully decoupled, as they share the same configuration and the same dependencies."

> "In an umbrella project, you have an `apps/` folder where you store each application. Then, instead of each app in the umbrella having its own configuration, build cache, lockfile and so, they all point to the parent project by specifying the following configuration in their `mix.exs`:"

A `mix new my_umbrella --umbrella` layout:

```text
my_umbrella/
├── .formatter.exs
├── .gitignore
├── README.md
├── mix.exs
├── apps/        # empty; child apps are created here via: cd apps && mix new child_app
└── config/
    └── config.exs
```

Umbrella root `mix.exs`:

```elixir
defmodule MyUmbrella.MixProject do
  use Mix.Project

  def project do
    [
      apps_path: "apps",
      version: "0.1.0",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  # Dependencies listed here are available only for this
  # project and cannot be accessed from applications inside
  # the apps folder.
  defp deps do
    []
  end
end
```

Each umbrella child points at the parent's shared build, deps, lock, and config:

```elixir
defmodule MyApp.MixProject do
  use Mix.Project

  def project do
    [
      app: :my_app,
      version: "0.1.0",
      build_path: "../../_build",
      config_path: "../../config/config.exs",
      deps_path: "../../deps",
      lockfile: "../../mix.lock",
      elixir: "~> 1.20",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      # {:sibling_app_in_umbrella, in_umbrella: true}
    ]
  end
end
```

Cross-application dependencies inside the umbrella use `in_umbrella: true`. From [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html):

> "When an umbrella application needs to depend on another one, it can be done by passing the `in_umbrella: true` option to your dependency."

### Project naming derivation

`mix new` derives the OTP app name and module name from the path. From [Mix.Tasks.New.html](https://hexdocs.pm/mix/Mix.Tasks.New.html):

> "The application name and module name will be retrieved from the path, unless `--module` or `--app` is given."

The derivation logic in the [mix.new source](https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/tasks/new.ex) is:

- `app = opts[:app] || Path.basename(Path.expand(path))` — snake_case OTP app name, used as `app: :APP`.
- `mod = opts[:module] || Macro.camelize(app)` — CamelCase module name.
- `lib` file name = `Macro.underscore(mod) <> ".ex"`.

Validation rules from the mix.new source:

- Application name must match `^[a-z][a-z0-9_]*$`:

> "Application name must start with a lowercase ASCII letter, followed by lowercase ASCII letters, numbers, or underscores, got: #{inspect(name)}"

- Module name must match `^[A-Z]\w*(\.[A-Z]\w*)*$`:

> "Module name must be a valid Elixir alias (for example: Foo.Bar), got: #{inspect(name)}"

Reserved application names include OTP/Elixir names such as `kernel`, `stdlib`, `ssl`, `logger`, `mix`, `elixir`, `eex`, `ex_unit`, `iex`, `crypto`, etc.

Concrete example for `mix new my_app`:

| Concept | Value | Notes |
|---|---|---|
| OTP app name | `:my_app` | snake_case atom |
| Top-level module | `MyApp` | CamelCase |
| Library file | `lib/my_app.ex` | snake_case |
| Test file | `test/my_app_test.exs` | module `MyAppTest` |

With `--module Foo.Bar.Baz`, the library file becomes `lib/foo/bar/baz.ex` and the module is `Foo.Bar.Baz`.

### Configuration files

Build-time configuration lives in `config/config.exs` and imported environment files. From [Config.html](https://hexdocs.pm/elixir/Config.html):

> "This module is most commonly used to define application configuration, typically in `config/config.exs`:"

```elixir
import Config

config :some_app,
  key1: "value1",
  key2: "value2"

import_config "#{config_env()}.exs"
```

> "`import Config` will import the functions `config/2`, `config/3` `config_env/0`, `config_target/0`, and `import_config/1` to help you manage your configuration."

> "Finally, the line `import_config "#{config_env()}.exs"` will import other config files based on the current configuration environment, such as `config/dev.exs` and `config/test.exs`."

> "For runtime configuration, you can use the `config/runtime.exs` file. It is executed right before applications start in both Mix and releases (assembled with `mix release`)."

Build-time configuration is loaded on every `mix` invocation. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Build-time configuration: Whenever you invoke a `mix` command, Mix loads the configuration in `config/config.exs`, if said file exists... We say `config/config.exs` and all imported files are build-time configuration as they are evaluated whenever you compile your code."

Runtime configuration is loaded at startup. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Runtime configuration: To enable runtime configuration in your release, all you need to do is to create a file named `config/runtime.exs`:"

```elixir
import Config
config :my_app, :secret_key, System.fetch_env!("MY_APP_SECRET_KEY")
```

> "This file is executed whenever your project runs. If you assemble a release with `mix release`, it also executes every time your release starts."

> "Avoiding the application environment: The application environment is discouraged for libraries. See Elixir's Library Guidelines for more information."

Environment-specific dependencies are declared in `deps/0`. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "You can also specify that certain dependencies are available only for certain environments:
> {:some_test_dependency, \"~> 1.0\", only: :test}"

## Compilation

### What mix compile does

`mix compile` is the main entry point for compiling a Mix project. From [Mix.Tasks.Compile](https://hexdocs.pm/mix/Mix.Tasks.Compile.html):

> "The main entry point to compile source files. It simply runs the compilers registered in your project and returns a tuple with the compilation status and a list of diagnostics."

Before compilation begins, Mix performs several checks and prunes the code path. From [Mix.Tasks.Compile](https://hexdocs.pm/mix/Mix.Tasks.Compile.html):

> "Before compiling code, it performs a series of checks to ensure all dependencies are compiled and the project is up to date. Then the code path of your Elixir system is pruned to only contain the dependencies and applications that you have explicitly listed in your `mix.exs`."

Compilers are run by `Mix.Task.Compiler.run/2`. From [Mix.Task.Compiler](https://hexdocs.pm/mix/Mix.Task.Compiler.html):

> "Runs the given list of compilers with the given arguments. It returns a `{status, diagnostics}` tuple. If a compiler errors, following compilers do not run."

This function has been available since v1.19.0. The pipeline is:

1. Pre-flight checks: dependencies compiled, project up to date, code path pruned.
2. Iterate over the `:compilers` list, invoking each compiler as a `Mix.Task.Compiler`.
3. The first compiler that returns `:error` halts the pipeline; remaining compilers do not run.
4. When `consolidate_protocols: true` (the default), protocol consolidation runs after the `:elixir` compiler.
5. The task returns `{status, diagnostics}`.

### The compiler list and defaults

The `:compilers` key in `def project` controls which compilers run and in which order. From [Mix.Tasks.Compile](https://hexdocs.pm/mix/Mix.Tasks.Compile.html):

> "compilers to run, defaults to `Mix.compilers/0`, which are `[:erlang, :elixir, :app]`."

The default compiler list is returned by `Mix.compilers/0`. From [Mix](https://hexdocs.pm/mix/Mix.html):

> "Returns the default compilers used by Mix. It can be used in your `mix.exs` to prepend or append new compilers to Mix:"

The canonical way to extend the list is:

```elixir
compilers: Mix.compilers() ++ [:foo, :bar]
```

Built-in compilers include `:yecc`, `:leex`, `:erlang`, `:elixir`, and `:app`, but `:yecc` and `:leex` must be opted into explicitly. From [Mix.Tasks.Compile.Yecc](https://hexdocs.pm/mix/Mix.Tasks.Compile.Yecc.html):

> "You must add `compilers: [:yecc] ++ Mix.compilers()` in the `def project` section of your `mix.exs` to run this compiler."

From [Mix.Tasks.Compile.Leex](https://hexdocs.pm/mix/Mix.Tasks.Compile.Leex.html):

> "You must add `compilers: [:leex] ++ Mix.compilers()` in the `def project` section of your `mix.exs` to run this compiler."

Protocol consolidation is enabled by default; `consolidate_protocols: true` is the default. To discover all compilers for the current project, use `Mix.Task.Compiler.compilers/1`. Note that `Mix.Tasks.Compile.compilers/1` and `Mix.Tasks.Compile.manifests/0` are deprecated in favor of `Mix.Task.Compiler.compilers/1`.

### Command-line options

`mix compile` supports the following flags. From [Mix.Tasks.Compile](https://hexdocs.pm/mix/Mix.Tasks.Compile.html):

| Flag | Effect |
|---|---|
| `--all-warnings` / `--no-all-warnings` | Prints all warnings including those from previous compilations. Default: `true` except on errors. |
| `--erl-config PATH` | Path to an Erlang term file that will be loaded as Mix config. |
| `--force` | Forces compilation. Also accept `--force-#{compiler}` (e.g. `--force-elixir`) to force a specific compiler; it requires the compiler to respect the `--force` option, which is advised. |
| `--list` | Lists all enabled compilers (calls `Mix.Task.Compiler.compilers/1`). |
| `--listeners` | Starts Mix listeners (default; suppressed by `--no-listeners` or `--no-deps-check`). |
| `--no-app-loading` | Does not load the `.app` resource file after compilation. |
| `--no-archives-check` | Skips checking of archives. |
| `--no-compile` | Does not compile, only loads code and performs checks. |
| `--no-deps-check` | Skips checking of dependencies. |
| `--no-elixir-version-check` | Does not check the Elixir version. |
| `--no-listeners` | Does not start Mix listeners. |
| `--no-optional-deps` | Does not compile or load optional deps. Passing this flag will force a full recompilation. |
| `--no-prune-code-paths` | Does not prune code paths before compilation. |
| `--no-protocol-consolidation` | Skips protocol consolidation. |
| `--no-validate-compile-env` | Does not validate the application compile environment. |
| `--return-errors` | Returns error status and diagnostics instead of exiting on error. |
| `--warnings-as-errors` | exit with non-zero status if compilation has one or more warnings. Only concerns compilation warnings from the project, not its dependencies. |

The two most commonly used flags are `--warnings-as-errors`, which is useful in CI and strict workflows, and `--force` or `--force-#{compiler}`, which trigger a clean rebuild of all or one compiler. To remove `_build` artifacts entirely, use `mix clean`; there is no `--purge` flag for `mix compile`.

### Mix.Task.Compiler behaviour

A custom compiler is a module whose name starts with `Mix.Tasks.Compile.` and that calls `use Mix.Task.Compiler`. From [Mix.Task.Compiler](https://hexdocs.pm/mix/Mix.Task.Compiler.html):

> "A Mix compiler task can be defined by simply using `Mix.Task.Compiler` in a module whose name starts with `Mix.Tasks.Compile.` and defining the `run/1` function."

The behaviour defines one required callback and three optional callbacks:

```elixir
@callback run([binary()]) ::
          status() |
          {status(), [Mix.Task.Compiler.Diagnostic.t()]}
@callback manifests() :: [Path.t()]            # optional
@callback clean() :: any()                      # optional
@callback diagnostics() :: [Mix.Task.Compiler.Diagnostic.t()]  # optional
```

where `@type status() :: :ok | :noop | :error`.

From [Mix.Task.Compiler](https://hexdocs.pm/mix/Mix.Task.Compiler.html):

> "Receives command-line arguments and performs compilation. If it produces errors, warnings, or any other diagnostic information, it should return a tuple with the status and a list of diagnostics."

> "If the compiler uses manifest files to track stale sources, it should define `manifests/0`, and if it writes any output to disk it should also define `clean/0`."

Custom compilers support the same attributes as regular Mix tasks: `@shortdoc`, `@moduledoc`, `@requirements`, `@recursive`, and `@preferred_cli_env`.

Diagnostics are represented by the `Mix.Task.Compiler.Diagnostic` struct. From [Mix.Task.Compiler.Diagnostic](https://hexdocs.pm/mix/Mix.Task.Compiler.Diagnostic.html), its fields include `compiler_name`, `severity` (`:error | :warning | :information | :hint`), `file`, `position`, `span`, `message`, `details`, `stacktrace`, and `source`.

### Writing a custom compiler

The minimal custom compiler looks like this:

```elixir
defmodule Mix.Tasks.Compile.MyLanguage do
  use Mix.Task.Compiler

  def run(_args) do
    :ok
  end
end
```

The module name maps to a Mix task name. From [Mix.Task](https://hexdocs.pm/mix/Mix.Task.html):

> "The command name will correspond to the portion of the module name following `Mix.Tasks.`. For example, a module name of `Mix.Tasks.Deps.Clean` corresponds to a task name of `deps.clean`."

So `Mix.Tasks.Compile.MyLanguage` becomes `mix compile.my_language`, and the file is conventionally placed at `lib/mix/tasks/compile.my_language.ex`. Register it by adding the compiler atom to `:compilers`:

```elixir
compilers: [:my_compiler] ++ Mix.compilers()
```

To support incremental compilation, define `manifests/0`:

```elixir
def manifests, do: [Path.join(Mix.Project.manifest_path(), "compile.my_compiler")]
```

### Manifests and the _build directory

Manifests track compiled sources so compilers can decide what to recompile. They live in the path returned by `Mix.Project.manifest_path/0`. From [Mix.Project](https://hexdocs.pm/mix/Mix.Project.html):

> "Returns the path where manifests are stored. By default they are stored in the app path inside the build directory. Umbrella applications have the manifest path set to the root of the build directory."

`Mix.Task.Compiler.manifests/1` lists manifest files for all compilers in the current project. The two conventional manifest files are `.compile.elixir` (used by the Elixir compiler) and `.compile.erlang` (used by the Erlang compiler), both located under `_build/<env>/lib/<app>/.mix/`.

A typical `_build` layout is:

```text
_build/
└── dev/                      # MIX_ENV directory ("shared" if build_per_environment: false)
    └── lib/
        └── my_app/
            ├── ebin/         # compiled .beam files (Mix.Project.compile_path/0)
            ├── .mix/         # manifests (Mix.Project.manifest_path/0)
            └── consolidated/ # protocol consolidation (Mix.Project.consolidation_path/0)
```

The APIs that locate these directories are:

| Concern | API | Example |
|---|---|---|
| Build path | `Mix.Project.build_path/0` | `_build/dev` (or `_build/shared`) |
| App path | `Mix.Project.app_path/0` | `_build/dev/lib/my_app` |
| Compile (ebin) path | `Mix.Project.compile_path/0` | `_build/dev/lib/my_app/ebin` |
| Manifest path | `Mix.Project.manifest_path/0` | `_build/dev/lib/my_app/.mix` |
| Consolidation path | `Mix.Project.consolidation_path/0` | `_build/dev/lib/my_app/consolidated` (umbrella: `_build/dev/consolidated`) |

The `:build_path` configuration defaults to `"_build"`, and `:build_per_environment` defaults to `true`. When `:build_per_environment` is `false`, the build path becomes `_build/shared` instead of `_build/dev`. The `MIX_BUILD_PATH` and `MIX_BUILD_ROOT` environment variables can override these paths. The `_build` directory is reproducible and should be git-ignored. To remove build artifacts, run `mix clean`; use `--deps` to also clean dependencies and `--only ENV` to limit cleaning to a specific environment.

### elixirc options

The `:elixirc_paths` option (default `["lib"]`) lists the directories scanned for `*.ex` and `*.exs` source files. The `:elixirc_options` keyword list is passed to Elixir's compiler. From [Mix.Tasks.Compile.Elixir](https://hexdocs.pm/mix/Mix.Tasks.Compile.Elixir.html):

> "compilation options that apply to Elixir's compiler. It supports many of the options above plus the options listed in `Code.put_compiler_option/2`. In case conflicting options are given, the ones given through the command line are used."

Key `elixirc_options` and their matching CLI flags are:

| Config / CLI | Effect |
|---|---|
| `:docs` / `--docs`/`--no-docs` | Attaches (or not) documentation to compiled modules. |
| `:debug_info` / `--debug-info`/`--no-debug-info` | Attaches (or not) debug info; passing the flag forces a full recompilation. |
| `:ignore_module_conflict` / `--ignore-module-conflict` | Does not emit warnings if a module was previously defined. |
| `:warnings_as_errors` / `--warnings-as-errors` | Exit non-zero if any warning. |
| `:long_compilation_threshold N` / `--long-compilation-threshold N` | Sets long-compilation threshold (seconds). |
| `:profile` / `--profile` | If set to `time`, outputs compilation timing. |
| `:verbose` / `--verbose` | Prints each file being compiled. |
| `[xref: [exclude: ...]]` | Modules/`{m,f,a}` to exclude from undefined-module/application warnings. |

Example configuration:

```elixir
def project do
  [
    app: :my_app,
    version: "0.1.0",
    elixirc_paths: ["lib"],
    elixirc_options: [warnings_as_errors: true, docs: true]
  ]
end
```

### erlc options

Erlang-related compilers share a set of paths and options. `:erlc_paths` (default `["src"]`) is used by `compile.erlang`, `compile.yecc`, and `compile.leex`. `:erlc_include_path` defaults to `"include"`, and `:erlc_options` defaults to `[]`. From [Mix.Tasks.Compile.Erlang](https://hexdocs.pm/mix/Mix.Tasks.Compile.Erlang.html):

> "compilation options that apply to Erlang's compiler. Defaults to `[]`. For a complete list of options, see `:compile.file/2`. The option `:debug_info` is always added to the end of it. You can disable that using: `erlc_options: [debug_info: false]`."

`:yecc_options` and `:leex_options` configure the Yecc and Leex compilers respectively (see `:yecc.file/1` and `:leex.file/2`). The options `:report`, `:return_errors`, and `:return_warnings` are overridden by Mix.

The `ERL_COMPILER_OPTIONS` environment variable can supply default Erlang compiler options. From [Mix.Tasks.Compile.Erlang](https://hexdocs.pm/mix/Mix.Tasks.Compile.Erlang.html):

> "can be used to give default compile options. The value must be a valid Erlang term."

`mix compile.erlang` accepts its own flags: `--all-warnings`/`--no-all-warnings`, `--force`, and `--verbose`.

### Incremental compilation

Elixir recompiles only what has changed plus its compile-time dependents. From [Mix.Tasks.Compile.Elixir](https://hexdocs.pm/mix/Mix.Tasks.Compile.Elixir.html):

> "Elixir is smart enough to recompile only files that have changed and their dependencies. This means if `lib/a.ex` is invoking a function defined over `lib/b.ex` at compile time, whenever `lib/b.ex` changes, `lib/a.ex` is also recompiled. More details about dependencies between files can be found in the documentation of `mix xref`."

A file is considered changed when both its mtime and contents differ from the last compilation. From [Mix.Tasks.Compile.Elixir](https://hexdocs.pm/mix/Mix.Tasks.Compile.Elixir.html):

> "Elixir considers a file as changed if its source file has changed on disk since the last compilation AND its contents are no longer the same."

Modules can declare external resources. From [Mix.Tasks.Compile.Elixir](https://hexdocs.pm/mix/Mix.Tasks.Compile.Elixir.html):

> "If a module depends on external files, those can be annotated with the `@external_resource` module attribute. If these files change, the Elixir module is automatically recompiled."

Custom recompilation rules can be provided via `__mix_recompile__?/0`. From [Mix.Tasks.Compile.Elixir](https://hexdocs.pm/mix/Mix.Tasks.Compile.Elixir.html):

> "A module may export a `__mix_recompile__?/0` function that can cause the module to be recompiled using custom rules."

This function is called for every module being or already compiled, so it should be kept cheap. It is skipped for modules that set `@compile {:autoload, false}`.

Erlang, Yecc, and Leex use simpler mtime-based incremental checks. From [Mix.Tasks.Compile.Erlang](https://hexdocs.pm/mix/Mix.Tasks.Compile.Erlang.html):

> "When this task runs, it will first check the modification times of all files to be compiled and if they haven't been changed since the last compilation, it will not compile them. If any of them have changed, it compiles everything."

Project-wide recompilation can be triggered by configuration changes. `Mix.Project.config_mtime/0` returns the latest mtime of config files, and `Mix.Project.config_files/0` lists the lockfile and `config/*.exs` files.

To force recompilation, use `mix compile --force`, `mix compile --force-#{compiler}`, `mix compile.elixir --force`, or `mix clean`.

### Review hooks

For CI and strict workflows, run `mix compile --warnings-as-errors`. For a clean rebuild, use `mix compile --force`. To purge `_build` entirely, run `mix clean`.

## Formatting

Elixir ships a code formatter that enforces a consistent style across a project. The `mix format` task formats source files according to a project-wide `.formatter.exs` configuration, and is designed to run automatically from editors and CI.

### What mix format does

From [Mix.Tasks.Format.html](https://hexdocs.pm/mix/Mix.Tasks.Format.html):

> "This task formats the given files and writes the changes back to the filesystem. ... If no files are given, the files specified by the `.formatter.exs` configuration file are formatted."

When any argument is `-`, the formatter reads from stdin and writes to stdout:

> "If any of the files is `-`, the input is read from stdin and the output is written to stdout."

### Command-line flags

From [Mix.Tasks.Format.html](https://hexdocs.pm/mix/Mix.Tasks.Format.html):

| Flag | Effect |
|---|---|
| `--check-formatted` | Checks that all files are already formatted. Does not write changes to disk. Exits non-zero if any file is not formatted. Useful for pre-commit hooks and CI. |
| `--no-exit` | Only valid with `--check-formatted`. Reports format errors but does not exit non-zero. |
| `--dry-run` | Does not save files after formatting. |
| `--force` | Forces formatting of all files, bypassing the cache. |
| `--no-compile` | Does not compile, even if compilation is required to load formatter plugins. Errors if a plugin cannot be loaded. |
| `--verbose` | Prints the names of files that were formatted. |
| `--dot-formatter PATH` | Path to the formatter config file. Defaults to `.formatter.exs`. |
| `--stdin-filename PATH` | Path of the file being formatted via stdin. Important when plugins support custom file types (e.g. `.heex`). Defaults to `"stdin.exs"`. |
| `--migrate` | Enables the `:migrate` formatter option, which rewrites the AST. Run in its own commit. |

The two most common flags in practice are `--check-formatted` (CI and pre-commit hooks) and `--dot-formatter` (pointing at a non-default config).

### The .formatter.exs file

The `.formatter.exs` file is a keyword list read from the current directory. From [Mix.Tasks.Format.html](https://hexdocs.pm/mix/Mix.Tasks.Format.html):

> "The formatter configuration is read from the current directory and it must return a keyword list."

The file generated by `mix new` (shown above in Project Directory Structure) is:

```elixir
# Used by "mix format"
[
  inputs: ["{mix,.formatter}.exs", "{config,lib,test}/**/*.{ex,exs}"]
]
```

The keyword list supports two categories of options:

1. **Task-specific keys** understood by `mix format`: `:inputs`, `:excludes`, `:plugins`, `:subdirectories`, `:import_deps`, `:export`.
2. **Formatter options** forwarded to `Code.format_string!/2` (see [Code.html](https://hexdocs.pm/elixir/Code.html#format_string!/2)): `:line_length`, `:locals_without_parens`, `:force_do_end_blocks`, `:file`, `:line`, and the `:migrate*` family.

### Inputs and globs

The `:inputs` key lists the paths and glob patterns to format. From [Mix.Tasks.Format.html](https://hexdocs.pm/mix/Mix.Tasks.Format.html):

> "`:inputs` - a list of paths and patterns to use as default inputs to the task. Each path/pattern is expanded via `Path.wildcard/2`."

Standard glob syntax is supported (`*`, `**`, and brace expansion `{}`). An expanded inputs list:

```elixir
[
  inputs: [
    "mix.exs",
    ".formatter.exs",
    "{config,lib,test}/**/*.{ex,exs}"
  ]
]
```

The `:excludes` key (since Elixir v1.19.0) removes paths from the `:inputs` set:

> "`:excludes` - a list of paths and patterns to exclude from the inputs. Each path/pattern is expanded via `Path.wildcard/2`."

```elixir
[
  inputs: ["{config,lib,test}/**/*.{ex,exs}"],
  excludes: ["config/runtime.exs"]
]
```

### import_deps and export

Dependencies can export formatter rules that consuming projects import. The mechanism is `:export` in the dependency's `.formatter.exs` paired with `:import_deps` in the consumer's `.formatter.exs`.

From [Mix.Tasks.Format.html](https://hexdocs.pm/mix/Mix.Tasks.Format.html):

> "`:import_deps` - a list of dependencies to import formatter configuration from. ... A dependency that wants to export formatter configuration should have a `.formatter.exs` at its root with an `:export` option."

Currently only `:locals_without_parens` is supported under `:export`. A dependency declares what it exports:

```elixir
# deps/my_dep/.formatter.exs
[
  export: [
    locals_without_parens: [assert_schema: 2, defschema: 2]
  ]
]
```

and a consumer imports it:

```elixir
# my_app/.formatter.exs
[
  import_deps: [:my_dep],
  inputs: ["{mix,.formatter}.exs", "{config,lib,test}/**/*.{ex,exs}"]
]
```

The dependency must be present in `mix.exs` for the current `Mix.env/0` and fetched (`mix deps.get`); otherwise the formatter reports `Unknown dependency ... given to :import_deps`.

> **Note:** An older `format: true` option inside `mix.exs` deps was used by early Phoenix/Elixir versions to share formatter rules. It is **not** a current dependency option in Mix v1.20.2; the `:import_deps` + `:export` mechanism above replaces it.

### Subdirectories

The `:subdirectories` key points at subdirectories that have their own `.formatter.exs`. From [Mix.Tasks.Format.html](https://hexdocs.pm/mix/Mix.Tasks.Format.html):

> "`:subdirectories` - a list of paths and patterns specifying subdirectories that have their own `.formatter.exs`."

Each subdirectory must contain its own `.formatter.exs`, and configuration is **not inherited** between parent and child. From [Mix.Tasks.Format.html](https://hexdocs.pm/mix/Mix.Tasks.Format.html):

> "Each subdirectory must have its own `.formatter.exs`. Keep in mind that the parent `.formatter.exs` will be completely unavailable to the subdirectories. So, if a parent defines `import_deps: [:my_dep]`, it will be unavailable to the subdirectories' formatter."

> "The parent `.formatter.exs` should not include the files inside the subdirectory in its `inputs`. If it does, it is unspecified which formatter configuration wins."

This is the typical umbrella layout:

```elixir
# .formatter.exs (umbrella root)
[
  inputs: ["{mix,.formatter}.exs", "config/*.exs"],
  subdirectories: ["apps/*"]
]
```

```elixir
# apps/my_app/.formatter.exs
[
  inputs: ["{mix,.formatter}.exs", "{config,lib,test}/**/*.{ex,exs}"]
]
```

### Plugins

The `:plugins` key (since Elixir v1.13.0) lists modules that extend formatting to custom sigils and file extensions. From [Mix.Tasks.Format.html](https://hexdocs.pm/mix/Mix.Tasks.Format.html):

> "`:plugins` - a list of modules implementing the `Mix.Tasks.Format` behaviour to use when formatting."

A plugin implements two callbacks:

```elixir
@callback features(keyword()) :: [sigils: [atom()], extensions: [binary()]]
@callback format(input :: binary(), keyword()) :: binary()
```

`features/1` declares which sigils (e.g. `:M` for `~M`) and file extensions (e.g. `".md"`, `".heex"`) the plugin handles. `format/2` receives the file/sigil contents plus the merged `.formatter.exs` options and returns the formatted contents. The order of plugins in the list determines the order in which they run.

```elixir
defmodule MixMarkdownFormatter do
  @behaviour Mix.Tasks.Format

  def features(_opts), do: [sigils: [:M], extensions: [".md", ".markdown"]]

  def format(contents, opts) do
    # format markdown; opts includes :extension, :sigil, :modifiers
    contents
  end
end
```

```elixir
# .formatter.exs
[
  plugins: [MixMarkdownFormatter],
  inputs: ["{mix,.formatter}.exs", "{config,lib,test}/**/*.{ex,exs}", "posts/*.md"]
]
```

Using a plugin forces the project to compile so the plugin module can be loaded; pass `--no-compile` to skip this (the task then errors if the plugin is unavailable). A well-known ecosystem plugin is Phoenix LiveView's `Phoenix.LiveView.HTMLFormatter`, which formats `.heex` templates and the `~H` sigil — for stdin use it relies on `--stdin-filename`.

### line_length and Code formatting options

The formatter forwards options to `Code.format_string!/2`. From [Code.html](https://hexdocs.pm/elixir/Code.html#format_string!/2):

> "`:line_length` - defaults to 98. Note this value is used as a reference but is not guaranteed, since the formatter may need to exceed it (for example when a string or a keyword list cannot be broken up)."

```elixir
[
  line_length: 100,
  inputs: ["{mix,.formatter}.exs", "{config,lib,test}/**/*.{ex,exs}"]
]
```

Other commonly used options:

| Option | Effect | Since |
|---|---|---|
| `:line_length` | Target line length (a target, not a hard limit). Default `98`. | — |
| `:locals_without_parens` | Keyword list of `{name, arity}` pairs kept without parentheses; arity may be `:*`. Augments the built-in list. | — |
| `:force_do_end_blocks` | Converts inline `do: ...` blocks to `do`/`end` blocks. Default `false`. Convergent: once enabled, re-running with `false` does not revert. | v1.9.0 |
| `:migrate` | Enables all `:migrate_*` rewrites at once. | v1.18.0 |
| `:migrate_bitstring_modifiers` | Normalizes bitstring modifiers (e.g. `<<foo::binary()>>` to `<<foo::binary>>`). | v1.18.0 |
| `:migrate_charlists_as_sigils` | Rewrites charlists as sigils (`'foo'` to `~c"foo"`). | v1.18.0 |
| `:migrate_unless` | Rewrites `unless` as `if` with a negated condition. | v1.18.0 |
| `:migrate_call_parens_on_pipe` | Adds parens to unqualified calls on the right of `\|>`. | v1.19.0 |

The `:migrate*` options change the AST; run them deliberately (ideally via `mix format --migrate`) in a dedicated commit. The formatter is deliberately low-configuration: from [Code.html](https://hexdocs.pm/elixir/Code.html#format_string!/2), strings, charlists, atoms, and sigils are preserved as written, and the formatter does not escape or unescape characters.

### mix format --check-formatted (CI)

The intended CI command is `mix format --check-formatted`. From [Mix.Tasks.Format.html](https://hexdocs.pm/mix/Mix.Tasks.Format.html):

> "`--check-formatted` - checks that all files are already formatted. Does not save newly formatted files to disk. Exits with a non-zero status if any file is not formatted. Useful as a pre-commit hook and in CI."

```yaml
# Example: GitHub Actions step
- name: Check formatting
  run: mix format --check-formatted
```

Because formatter output can change between Elixir versions as the formatter improves, pin the Elixir version in CI for stable results. Combine with compilation and tests for a strict pipeline:

```bash
mix format --check-formatted
mix compile --warnings-as-errors
mix test
```

### Editor integration

The formatter is designed to run from the editor on save. From [Mix.Tasks.Format.html](https://hexdocs.pm/mix/Mix.Tasks.Format.html):

> "We recommend developers to format code directly in their editors, either automatically when saving a file or via an explicit command or key binding. If such option is not available in your editor of choice, adding the required integration is usually a matter of invoking:
> $ cd $project && mix format $file
> where `$file` refers to the current file and `$project` is the root of your project."

For custom file types handled by a plugin (e.g. `.heex`), use `--stdin-filename` so the formatter selects the correct plugin.

## Dependencies

Mix manages dependencies through the `mix deps.*` task family. Dependencies are declared in the `deps/0` function in `mix.exs`, fetched from Source Code Managers (SCMs), compiled into the project, and pinned to exact revisions in `mix.lock`.

From [Mix.Tasks.Deps.html](https://hexdocs.pm/mix/Mix.Tasks.Deps.html):

> "Dependencies must be specified in the `mix.exs` file in one of the following formats":
> ```
> {app, requirement}
> {app, opts}
> {app, requirement, opts}
> ```
> "app is an atom, requirement is a Version requirement or a regular expression, opts is a keyword list of options."

### The deps/0 function and dependency formats

`:deps` is a project option returned by `project/0`. From [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html):

> ":deps — a list of dependencies of this project. Refer to the documentation for the `Mix.Tasks.Deps` task for more information. Defaults to `[]`."

The conventional wiring is `deps: deps()` in `project/0` and a private `deps/0` function that returns the list. Each entry is one of:

| Shape | Meaning |
|---|---|
| `{app, requirement}` | Hex-style dependency with a version requirement. |
| `{app, opts}` | Dependency whose source is fully determined by options (`:git`, `:path`, etc.). |
| `{app, requirement, opts}` | Version requirement plus additional options. |

`app` is an atom, `requirement` is a [Version](https://hexdocs.pm/elixir/Version.html) requirement or regex, and `opts` is a keyword list.

```elixir
defp deps do
  [
    # Hex (default SCM)
    {:jason, "~> 1.4"},

    # Git repository
    {:my_lib, git: "https://github.com/example/my_lib.git", tag: "0.5.1"},

    # Local path
    {:local_lib, path: "../local_lib"},

    # Umbrella sibling
    {:sibling_app, in_umbrella: true}
  ]
end
```

### The mix deps.* task family

The primary task module is [Mix.Tasks.Deps.html](https://hexdocs.pm/mix/Mix.Tasks.Deps.html). The family of tasks for fetching, compiling, and inspecting dependencies is summarized below.

| Task | Description | Notable options |
|---|---|---|
| `mix deps` | "Lists dependencies and their status." | `--all`, dep names to filter (v1.20.0) |
| `mix deps.get` | "Fetches unavailable and out of date dependencies." | `--check-locked`, `--no-archives-check`, `--only` |
| `mix deps.compile` | "Compiles dependencies." | `--force`, `--skip-umbrella-children`, `--skip-local-deps` |
| `mix deps.clean` | "Deletes the given dependencies' files, including build artifacts and fetched sources." | `--unlock`, `--build`, `--all`, `--unused`, `--only env` |
| `mix deps.tree` | "Prints the dependency tree." | `--only`, `--target`, `--exclude`, `--umbrella-only` (v1.17.0), `--format`, `--output` (v1.20.0) |
| `mix deps.unlock` | "Unlocks the given dependencies." | `--all`, `--filter`, `--unused`, `--check-unused` |
| `mix deps.update` | "Updates the given dependencies." | `--all`, `--only`, `--target`, `--no-archives-check` |

#### `mix deps`

`mix deps` prints each dependency as `APP VERSION (SCM) (MANAGER)`, followed by `[locked at REF]` and a status. Status values include `ok`, `locked`, `outdated`, and others that indicate whether the dependency needs to be fetched, updated, or unlocked. Pass `--all` to show dependencies that would otherwise be hidden; since v1.20.0, pass dependency names to filter the output (for example, `mix deps phoenix phoenix_live_view`).

#### `mix deps.get`

`mix deps.get` fetches dependencies that are unavailable or out of date with respect to `mix.lock` and the declared requirements. Mix installs Hex automatically on first use. Common flags:

- `--check-locked` — fails if `mix.lock` must be updated.
- `--no-archives-check` — skips archive re-checking.
- `--only env` — fetches only dependencies for the given environment.

#### `mix deps.compile`

`mix deps.compile` compiles dependencies. Mix detects the dependency manager from files present in each dependency's root:

| Detected file | Manager command |
|---|---|
| `mix.exs` | `mix compile` |
| `rebar.config` | `rebar3 compile` |
| `Makefile.win` | `nmake /F Makefile.win` (Windows) |
| `Makefile` | `gmake` (DragonFly/FreeBSD/NetBSD/OpenBSD) or `make` elsewhere |

A per-dependency compile command can be supplied with the `:compile` option:

```elixir
{:some_dependency, "0.1.0", compile: "command to compile"}
```

Options:

- `--force` — forces recompilation.
- `--skip-umbrella-children` — skips umbrella child apps.
- `--skip-local-deps` — skips local (`:path` / `:in_umbrella`) dependencies.

When the environment variable `MIX_OS_DEPS_COMPILE_PARTITION_COUNT` is set to a value greater than `1`, dependencies are compiled concurrently across OS processes.

#### `mix deps.clean`

`mix deps.clean` is destructive: it deletes the given dependencies' files, including build artifacts and fetched sources. It only acts when given dependency names or one of the qualifying flags/options.

Options:

- `--unlock` — also unlocks the dependency in `mix.lock`.
- `--build` — removes compiled files only.
- `--all` — cleans all dependencies.
- `--unused` — cleans dependencies no longer in `mix.exs`.
- `--only env` — limits cleaning to the given environment.

#### `mix deps.tree`

`mix deps.tree` prints the dependency tree.

Options:

- `--only env` — restrict to dependencies for the environment.
- `--target target` — restrict to dependencies for the target.
- `--exclude app` — exclude a dependency from the tree (repeatable).
- `--umbrella-only` — show only umbrella children (v1.17.0).
- `--format pretty | plain | dot` — output format; default is `pretty`.
- `--output path` — write the `dot` output to a file; `-` writes to standard output (v1.20.0).

#### `mix deps.unlock`

`mix deps.unlock` is destructive: it removes lock entries so that the next `mix deps.get` can resolve newer versions. It only acts when given dependency names or qualifying flags/options.

Options:

- `--all` — unlocks all dependencies.
- `--filter pattern` — unlocks dependencies matching a regular expression.
- `--unused` — unlocks dependencies that are no longer declared in `mix.exs`.
- `--check-unused` — exits with a non-zero status if unused lock entries exist. Useful in CI or pre-commit hooks to reject stale lock entries.

#### `mix deps.update`

`mix deps.update` is destructive: it unlocks the given dependencies, plus their dependents, and updates them to the latest version matching the declared requirements; all dependencies are then recompiled. It only acts when given dependency names or `--all`.

Options:

- `--all` — updates all dependencies.
- `--only env` — updates only dependencies for the environment.
- `--target target` — updates only dependencies for the target.
- `--no-archives-check` — skips archive re-checking.

A more granular alternative is to unlock a specific dependency and then refetch:

```bash
mix deps.unlock some_dep && mix deps.get
```

### Source Code Managers (SCMs)

Each dependency is sourced through a Source Code Manager. Mix tries each registered SCM in order by calling `accepts_options/2`; the first SCM that consumes the dependency's options wins.

| Options present | Selected SCM | Fetchable? |
|---|---|---|
| `:path` or `:in_umbrella` | `Mix.SCM.Path` | No |
| `:git` or `:github` | `Mix.SCM.Git` | Yes |
| otherwise | `Mix.SCM.Hex` (default) | Yes |

`Mix.Project.deps_scms/1` returns the SCM map for the current project's dependencies.

#### Hex packages (`:hex`)

When no `:git` or `:path` option is given, Mix treats the dependency as a Hex package. Hex is installed automatically on first use.

```elixir
{:plug, "~> 1.14"}
```

Hex-specific options:

| Option | Purpose |
|---|---|
| `:hex` | Hex package name, defaults to the app name. |
| `:repo` | Repository name, defaults to `"hexpm"`. |
| `:warn_if_outdated` | Warns if a newer version is available. |

From [Mix.Tasks.Deps.html](https://hexdocs.pm/mix/Mix.Tasks.Deps.html):

> "Hex expects the dependency requirement to always be given and it will warn otherwise."

#### Local path dependencies (`:path`)

A path dependency points at a local directory.

```elixir
{:local_lib, path: "../local_lib"}
```

| Option | Purpose |
|---|---|
| `:path` | Path to the dependency's source directory. |

Path dependencies are not fetchable; Mix expects the source to already exist on disk. From [Mix.Tasks.Deps.html](https://hexdocs.pm/mix/Mix.Tasks.Deps.html):

> "Path and in umbrella dependencies are automatically recompiled by the parent project whenever they change."

#### Git and GitHub (`:git`, `:github`)

Git dependencies fetch a remote repository.

```elixir
# Git with a tag
{:my_lib, git: "https://github.com/example/my_lib.git", tag: "0.5.1"}

# Git with a branch
{:my_lib, git: "https://github.com/example/my_lib.git", branch: "main"}

# GitHub shorthand
{:my_lib, github: "example/my_lib", ref: "abc123..."}
```

Git-specific options:

| Option | Purpose |
|---|---|
| `:git` | Git repository URL. |
| `:github` | Shorthand for `https://github.com/org/repo.git`; value is `"org/repo"`. |
| `:ref` | A branch, commit, or tag reference. |
| `:branch` | Branch name. |
| `:tag` | Tag name. |
| `:submodules` | Whether to initialize submodules. |
| `:sparse` | Sparse checkout path. |
| `:subdir` | Subdirectory inside the repository to use as the dependency root (v1.13.0). |
| `:depth` | Shallow-clone depth in commits (v1.17.0); requires a full 40-character SHA-1 when used with `:ref`. |

Authentication is delegated to Git; configure `url.<base>.insteadOf` or credential helpers as needed.

#### Umbrella siblings (`:in_umbrella`)

Inside an umbrella project, sibling applications are referenced with `in_umbrella: true`:

```elixir
{:my_app, in_umbrella: true}
```

This is equivalent to a path dependency pointing at `"../my_app"`. Umbrella siblings share the parent's environment and are automatically recompiled when they change.

#### The `Mix.SCM` behaviour

The [Mix.SCM.html](https://hexdocs.pm/mix/Mix.SCM.html) module defines the behaviour for dependency sources. Implementing modules provide the following callbacks:

| Callback | Purpose |
|---|---|
| `accepts_options/2` | Returns updated options if the SCM handles this dependency, otherwise `nil`. |
| `checked_out?/1` | Returns whether the dependency source is already checked out. |
| `checkout/1` | Fetches the dependency. |
| `equal?/2` | Compares two lock entries for equality. |
| `fetchable?/0` | Returns whether the SCM supports fetching. |
| `format/1` | Formats dependency information for display. |
| `format_lock/1` | Formats a lock entry for display. |
| `lock_status/1` | Returns `:mismatch`, `:outdated`, or `:ok` for a lock entry. |
| `managers/1` | Returns the list of build managers applicable to the dependency. |
| `update/1` | Updates the checked-out dependency. |

SCM registry functions include `append/1`, `available/0`, `prepend/1`, and `delete/1` (since v1.16.2). The three built-in SCMs — Hex, Git, and Path — are documented within [Mix.Tasks.Deps.html](https://hexdocs.pm/mix/Mix.Tasks.Deps.html); they do not have separate Hexdocs pages.

### Dependency options

Dependency options are supplied in the third element of the dependency tuple (or the second element when no requirement is given).

| Option | Default | Effect |
|---|---|---|
| `:only` | all environments | Restricts the dependency to one or more Mix environments (`:dev`, `:test`, `:prod`, etc.). |
| `:targets` | all targets | Restricts the dependency to one or more targets. |
| `:optional` | `false` | Current project always includes it; dependents are not forced to use it. Optional dependencies are not started as applications. |
| `:runtime` | `true` | Whether the dependency is part of the runtime applications. If `:applications` is not set in `def application`, Mix auto-includes all deps unless `runtime: false`. |
| `:override` | `false` | When `true`, overrides other definitions of this dependency elsewhere in the dependency tree. |
| `:env` | `:prod` | Environment in which to run the dependency; `:in_umbrella` dependencies share the parent environment. |
| `:manager` | auto-detected | Override the detected build manager (`:mix`, `:rebar3`, or `:make`); on conflict the first in `[:mix, :rebar3, :make]` wins. |
| `:system_env` | `[]` | Enumerable of `{key, value}` binaries set when loading/compiling the dependency. |
| `:compile` | auto-detected | String command used to compile the dependency, overriding manager detection. |
| `:app` | `true` | When `false`, Mix does not read the dependency's `.app` file. |

Combined example:

```elixir
defp deps do
  [
    # Hex package used only at runtime in :test
    {:mox, "~> 1.0", only: :test},

    # Dev/test-only tooling that does not start at runtime
    {:credo, "~> 1.7", only: [:dev, :test], runtime: false},
    {:ex_doc, "~> 0.31", only: :dev, runtime: false},

    # Git dependency on a branch
    {:feature_lib, git: "https://github.com/example/feature_lib.git", branch: "main"},

    # Optional dependency
    {:cachex, "~> 3.6", optional: true},

    # Force this version over transitive definitions
    {:jason, "~> 1.4", override: true},

    # Nerves-style target restriction
    {:nerves_runtime, "~> 0.13", targets: :rpi4}
  ]
end
```

### Version requirements (SemVer)

Dependency requirements follow Semantic Versioning and are parsed by the [Version](https://hexdocs.pm/elixir/Version.html) module. A version has the form `MAJOR.MINOR.PATCH`, optionally followed by a pre-release segment (`-`) and build metadata (`+`).

```text
1.0.0
1.0.0-alpha.3
1.0.0+build.123
```

Supported operators:

| Operator | Meaning |
|---|---|
| `==` | Exactly equal. |
| `!=` | Not equal. |
| `>` | Greater than. |
| `>=` | Greater than or equal. |
| `<` | Less than. |
| `<=` | Less than or equal. |
| `~>` | Pessimistic greater-than-or-equal (see table below). |
| `and` / `or` | Combines requirements. |

A bare version such as `"2.0.0"` is interpreted as `== 2.0.0`.

`~>` translations:

| `~>` | Translation |
|---|---|
| `~> 2.0.0` | `>= 2.0.0 and < 2.1.0` |
| `~> 2.1.2` | `>= 2.1.2 and < 2.2.0` |
| `~> 2.0` | `>= 2.0.0 and < 3.0.0` |
| `~> 2.1` | `>= 2.1.0 and < 3.0.0` |

`~>` never includes pre-release versions of its upper bound. `Version.match?/3` accepts an `:allow_pre` option that defaults to `true`, but Hex uses `false` so that pre-releases are not matched by default.

### The `mix.lock` lock file

`mix.lock` pins the exact resolved version and checksum (for Hex) or commit (for Git) of each dependency, ensuring reproducible fetches across machines and builds.

The lock file lives at the project root. The `:lockfile` project option defaults to `"mix.lock"`; umbrella children typically point it at `"../../mix.lock"` to share a single lock file with the parent project.

A representative lock file is an Elixir map keyed by app atom:

```elixir
%{
  jason: {:hex, :jason, "1.4.4", "...checksum...", [:mix], [{:decimal, "~> 1.0 or ~> 2.0", [hex: :decimal, optional: true]}], "hexpm", "...outer_checksum..."},
  plug: {:hex, :plug, "1.14.2", "...checksum...", [:mix], [{:mime, "~> 1.0 or ~> 2.0", [hex: :mime, optional: false]}, {:plug_crypto, "~> 1.1.1 or ~> 1.2 or ~> 2.0", [hex: :plug_crypto, optional: false]}, {:telemetry, "~> 0.4.3 or ~> 1.0", [hex: :telemetry, optional: false]}], "hexpm", "...outer_checksum..."},
  gettext: {:git, "https://github.com/elixir-lang/gettext.git", "abc123...", [branch: "main"]}
}
```

From [Mix.Tasks.Deps.html](https://hexdocs.pm/mix/Mix.Tasks.Deps.html):

> "the lockfile (usually named `mix.lock`) is ignored when a project is used as a dependency".

`mix deps.unlock` and `mix deps.update` modify the lock file. Hex verifies checksums on fetch. Pass `--no-archives-check` to `mix deps.get` to skip the archive re-check.

### Cross-references

For full details, see [Mix.Tasks.Deps.html](https://hexdocs.pm/mix/Mix.Tasks.Deps.html) and the task pages for [Deps.Get](https://hexdocs.pm/mix/Mix.Tasks.Deps.Get.html), [Deps.Compile](https://hexdocs.pm/mix/Mix.Tasks.Deps.Compile.html), [Deps.Clean](https://hexdocs.pm/mix/Mix.Tasks.Deps.Clean.html), [Deps.Tree](https://hexdocs.pm/mix/Mix.Tasks.Deps.Tree.html), [Deps.Unlock](https://hexdocs.pm/mix/Mix.Tasks.Deps.Unlock.html), and [Deps.Update](https://hexdocs.pm/mix/Mix.Tasks.Deps.Update.html). Project-level configuration is documented in [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html) under the `:deps`, `:deps_path`, and `:lockfile` options. SCM behaviour is documented in [Mix.SCM.html](https://hexdocs.pm/mix/Mix.SCM.html), version requirements in [Version.html](https://hexdocs.pm/elixir/Version.html), and runtime dependency loading in [Mix.html](https://hexdocs.pm/mix/Mix.html) and [Mix.install/2](https://hexdocs.pm/mix/Mix.html#install/2).

## Aliases

Aliases are project-specific shortcuts that compose existing Mix tasks and Elixir code into a single command. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Aliases are shortcuts or tasks specific to the current project."

They live in the `:aliases` key of `project/0`. Unlike a custom `Mix.Task` module — which ships with a project and is available to its dependents — an alias is private to the project that defines it. Aliases are the primary mechanism for composing multi-step workflows (project setup, database-backed test runs, lint, CI) out of tasks that already exist.

> **Note on placement:** A short orientation to aliases also appears under [Mix Overview](#mix-overview). This section is the in-depth reference: full value syntax, argument forwarding, the run-once rule, umbrella behaviour, and idiomatic patterns.

### Defining aliases in mix.exs

The conventional wiring is `aliases: aliases()` in `project/0` plus a private `aliases/0` function, mirroring the `deps/0` pattern. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

```elixir
defmodule MyApp.MixProject do
  use Mix.Project

  def project do
    [
      app: :my_app,
      version: "1.0.0",
      aliases: aliases()
    ]
  end

  defp aliases do
    [
      c: "compile",
      hello: &hello/1,
      paid_task: &paid_task/1
    ]
  end

  defp hello(_) do
    Mix.shell().info("Hello world")
  end

  defp paid_task(_) do
    Mix.Task.run("paid.task", [
      "first_arg",
      "second_arg",
      "--license-key",
      System.fetch_env!("SOME_LICENSE_KEY")
    ])
  end
end
```

> "In the example above, we have defined three aliases. One is `mix c` which is a shortcut for `mix compile`. Another is named `mix hello` and the third is named `mix paid_task`, which executes the code inside a custom function to invoke the `paid.task` task with several arguments, including one pulled from an environment variable."

The `:aliases` project option defaults to empty. From [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html):

> ":aliases — a list of task aliases. For more information, check out the \"Aliases\" section in the documentation for the `Mix` module. Defaults to `[]`."

### Alias value syntax

Each alias is a `{name, value}` pair where `name` is an atom and `value` is one of the forms below. The authoritative resolution lives in the [`mix/task.ex`](https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/task.ex) source.

| Form | Example | Behaviour |
|---|---|---|
| Binary string | `c: "compile"` | A task name plus optional args; the string is split with `OptionParser.split/1`, so `"deps.get --only dev"` resolves to task `deps.get` with arg `--only dev`. |
| List of strings/functions | `all: ["deps.get", "compile"]` | Elements run consecutively; lists may freely mix strings and 1-arity functions. |
| 1-arity anonymous function | `hello: &hello/1` | Receives the list of args passed to the alias (only when it is the last element of the alias list). |

Mix v1.20.2 accepts **only** binary strings and 1-arity anonymous functions as alias elements. There is no `{task, args}` tuple form and no 2-arity function form; the "task + args" combination is expressed by embedding the arguments in the string. Any other value raises an error from [`mix/task.ex`](https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/task.ex):

> "Invalid Mix alias format, aliases can be either a string (representing a Mix task with arguments) or a function that takes one argument (a list of alias arguments), got: ..."

A list alias runs its elements in order. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Aliases may also be lists, specifying multiple tasks to be run consecutively:"

```elixir
[all: [&hello/1, "deps.get --only #{Mix.env()}", "compile"]]
```

> "In the example above, we have defined an alias named `mix all`, that prints \"Hello world\", then fetches dependencies specific to the current environment, and compiles the project."

### Argument forwarding

Arguments the user passes on the command line are routed to the alias according to two rules. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "If the alias is overriding an existing task, the arguments given to the alias will be forwarded to the original task in order to preserve semantics. Otherwise arguments given to the alias are appended to the arguments of the last task in the list."

Concretely:

- **Override case** — an alias whose name matches a real task (e.g. `test: [...]`) forwards the user's args to that underlying task so its semantics are preserved.
- **Non-override case** — for an alias composed of multiple list elements, the user's args are appended to the last element only. If the last element is a string, the args reach that task; if it is a function, the function receives them as its single argument; if an element is not the last, it receives `[]`.

For example, given `test: ["ecto.create --quiet", "ecto.migrate --quiet", "test"]`, running `mix test --seed 0` appends `--seed 0` to the trailing `test` task. To forward args explicitly from a function element, call `Mix.Task.run/2` inside it.

### Augmenting and overriding existing tasks

Because an alias whose name collides with a task name is resolved as an alias, aliases are commonly used to wrap or extend built-in tasks. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Aliases can also be used to augment existing tasks. Let's suppose you want to augment `mix clean` to clean another directory Mix does not know about:"

```elixir
[clean: ["clean", &clean_extra/1]]
```

> "Where `&clean_extra/1` would be a function in your `mix.exs` with extra cleanup logic."

`Mix.Task.run/2` resolves aliases transparently. From [Mix.Task.html](https://hexdocs.pm/mix/Mix.Task.html):

> "If there is an [alias](Mix.html#module-aliases) defined for the given task name, the alias will be invoked instead of the original task."

This is how a `test:` alias intercepts the built-in `mix test` task while still preserving its argument semantics via the override rule above.

### Running scripts and shell commands

Aliases compose not just Mix tasks but any task reachable from the command line, including `mix run` (Elixir scripts) and `mix cmd` (shell commands). From [Mix.html](https://hexdocs.pm/mix/Mix.html):

```elixir
# priv/hello1.exs
IO.puts("Hello One")

# priv/hello2.exs
IO.puts("Hello Two")

# priv/world.sh
#!/bin/sh
echo "world!"

# mix.exs
defp aliases do
  [
    some_alias: ["hex.info", "run priv/hello1.exs", "cmd priv/world.sh"]
  ]
end
```

> "In the example above we have created the alias `some_alias` that will run the task `mix hex.info`, then `mix run` to run an Elixir script, then `mix cmd` to execute a command line shell script. This shows how powerful aliases mixed with Mix tasks can be."

### Tasks run only once (and reenabling)

Mix tasks are memoized: each task runs at most once per invocation, so dependencies like `mix compile` are compiled a single time even when many tasks require them. This affects aliases that invoke the same task more than once. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "One common pitfall of aliases comes when trying to invoke the same task multiple times. Mix tasks are designed to run only once. This prevents the same task from being executed multiple times. For example, if there are several tasks depending on `mix compile`, the code will be compiled only once."
>
> "Similarly, `mix format` can only be invoked once. So if you have an alias that attempts to invoke `mix format` multiple times, it won't work unless it is explicitly reenabled using `Mix.Task.reenable/1`:"

```elixir
another_alias: [
  "format --check-formatted priv/hello1.exs",
  "cmd priv/world.sh",
  fn _ -> Mix.Task.reenable("format") end,
  "format --check-formatted priv/hello2.exs"
]
```

> "Some tasks are automatically reenabled though, as they are expected to be invoked multiple times, such as: `mix cmd`, `mix do`, `mix xref`, etc."

From [Mix.Task.html](https://hexdocs.pm/mix/Mix.Task.html):

> "If the task or alias has already been invoked, subsequent calls to `run/2` will _abort_ without executing and return `:noop`."

> "Reenables a given task so it can be executed again down the stack. ... If an umbrella project reenables a task, it is re-enabled for all child projects."

To rerun a task deliberately rather than reenabling it, use `Mix.Task.rerun/2`.

### Default tasks vs. aliases

Mix ships default **tasks** (`compile`, `test`, `run`, ...), not default aliases. The `:aliases` configuration defaults to `[]`, so a freshly generated project has no aliases until they are added to `mix.exs`. The `mix test` task in particular is a built-in task (not an alias): it defaults to the `:test` environment (see [Environments and target](#environments-and-target)) and only becomes an alias when a project defines `test: [...]` to wrap it — at which point the [argument-forwarding](#argument-forwarding) override rule keeps its semantics intact.

> **Note on `mix test` aliases:** The [Mix.Tasks.Test](https://hexdocs.pm/mix/Mix.Tasks.Test.html) documentation in v1.20.2 does not contain an aliases example. The widely-used pattern of wrapping `mix test` with database setup (e.g. `test: ["ecto.create --quiet", "ecto.migrate --quiet", "test"]`) originates in the Phoenix project generator, not in the core Mix docs; it is reproduced below as an idiomatic convention.

### Common alias patterns

These are idiomatic conventions assembled from Mix's composition primitives and the wider ecosystem — not verbatim from the Mix docs. They show the typical ways aliases script project workflows: a `setup` entry point, reusable sub-workflows, a DB-backed `test` override, and composite `lint`, `typecheck`, and `ci` targets.

```elixir
defp aliases do
  [
    # Fetch dependencies and run DB setup (create, migrate, seed).
    setup: ["deps.get", "ecto.setup"],

    # Reusable ecto sub-workflows used by setup/test above.
    "ecto.setup": ["ecto.create", "ecto.migrate", "ecto.seed"],
    "ecto.reset": ["ecto.drop", "ecto.setup"],

    # Recreate and migrate the test database, then run the suite.
    # The trailing "test" element receives any args passed on the
    # command line (e.g. `mix test --seed 0` forwards `--seed 0`).
    test: ["ecto.create --quiet", "ecto.migrate --quiet", "test"],

    # Static analysis: formatting check plus a strict linter.
    lint: ["format --check-formatted", "credo --strict"],

    # Type checking via Dialyzer (requires dialyxir as a dependency).
    typecheck: ["dialyzer"],

    # Composite CI target combining the strict gates.
    ci: ["format --check-formatted", "compile --warnings-as-errors", "test"]
  ]
end
```

These assume the referenced tasks (`ecto.*`, `credo`) are available as dependencies; otherwise use the plain built-in equivalents.

### Umbrella projects

Aliases do not cross dependency boundaries, with one exception for umbrellas. From [Mix.html](https://hexdocs.pm/mix/Mix.html):

> "Finally, aliases defined in the current project do not affect its dependencies and aliases defined in dependencies are not accessible from the current project, with the exception of umbrella projects. Umbrella projects will run the aliases of its children when the umbrella project itself does not define said alias and there is no task with said name."

In practice this means an umbrella root can rely on an alias defined by one of its children when the root defines neither a matching alias nor a task of that name.

### Listing and inspecting aliases

Aliases surface in `mix help`. From [Mix.Tasks.Help.html](https://hexdocs.pm/mix/Mix.Tasks.Help.html):

> "$ mix help                  - prints all aliases, tasks and their short descriptions
> $ mix help --aliases        - prints all aliases"

Programmatic introspection is available through the `Mix.Task` API. From [Mix.Task.html](https://hexdocs.pm/mix/Mix.Task.html):

> "Checks if the given `task` name is an alias. Returns `false` if the given name is not an alias or if it is not a task." (`Mix.Task.alias?/1`)

### Aliases vs. custom Mix tasks

Aliases and custom `Mix.Task` modules are complementary composition mechanisms:

| Concern | Alias | Custom Mix task |
|---|---|---|
| Where defined | `:aliases` in `mix.exs` | `Mix.Tasks.*` module under `lib/mix/tasks/` |
| Visibility | Private to the defining project (umbrellas aside) | Public to anyone who depends on the project |
| Composition | Strings/functions over existing tasks | Implements `run/1`; may be referenced from aliases by its dot name |
| Distribution | Cannot be shared as a dependency | Distributable via Hex/git as part of `lib/` |
| Use when | Gluing existing tasks for a project workflow | A reusable, distributable command |

A custom task is referenced from an alias by its dot-separated name (e.g. `"my.task"`) and invoked programmatically with `Mix.Task.run("my.task", args)`. Custom task modules support the `@shortdoc`, `@moduledoc`, `@recursive`, `@preferred_cli_env`, and `@requirements` attributes; see [Writing custom Mix tasks](#writing-custom-mix-tasks) earlier in this document.

### Cross-references

For full details, see the [Aliases section of Mix.html](https://hexdocs.pm/mix/Mix.html) and the [Mix.Task](https://hexdocs.pm/mix/Mix.Task.html) module (`run/2`, `rerun/2`, `reenable/1`, `alias?/1`). The `:aliases` project option is documented in [Mix.Project.html](https://hexdocs.pm/mix/Mix.Project.html) and `mix help --aliases` in [Mix.Tasks.Help.html](https://hexdocs.pm/mix/Mix.Tasks.Help.html). Tasks referenced from aliases include [Clean](https://hexdocs.pm/mix/Mix.Tasks.Clean.html), [Run](https://hexdocs.pm/mix/Mix.Tasks.Run.html), [Cmd](https://hexdocs.pm/mix/Mix.Tasks.Cmd.html), [Do](https://hexdocs.pm/mix/Mix.Tasks.Do.html), and [Xref](https://hexdocs.pm/mix/Mix.Tasks.Xref.html). The authoritative alias-resolution source is [`lib/mix/lib/mix/task.ex`](https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/task.ex).

## Releases

Releases assemble a project, its dependencies, the Elixir and Erlang runtimes, and all compiled code into a single self-contained artifact that can be deployed without Mix, without source code, and — by default — without a separately installed Erlang/Elixir on the target. Releases are produced by the `mix release` task and configured under the `:releases` key in `project/0`.

From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "Assembles a self-contained release for the current project:
> ```
> $ MIX_ENV=prod mix release
> $ MIX_ENV=prod mix release NAME
> ```
> Once a release is assembled, it can be packaged and deployed to a target, as long as the target runs on the same operating system (OS) distribution and version as the machine running the `mix release` command. Windows releases also require Microsoft Visual C++ Runtime."

### Why releases

The benefits of releases are documented in [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "Releases allow developers to precompile and package all of their code and the runtime into a single unit. The benefits of releases are:
> - Code preloading. The VM has two mechanisms for loading code: interactive and embedded. By default, it runs in the interactive mode which dynamically loads modules when they are used for the first time... With releases, the system preloads all modules and guarantees your system is ready to handle requests after booting.
> - Configuration and customization. Releases give developers fine grained control over system configuration and the VM flags used to start the system.
> - Self-contained. A release does not require the source code to be included in your production artifacts. All of the code is precompiled and packaged. Releases do not even require Erlang or Elixir in your servers, as it includes the Erlang VM and its runtime by default...
> - Multiple releases. You can assemble different releases with different configuration per application or even with different applications altogether.
> - Management scripts. Releases come with scripts to start, restart, connect to the running system remotely, execute RPC calls, run as daemon, run as a Windows service, and more."

### Building a release

Always build releases with `MIX_ENV=prod`. The canonical build sequence from [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

```bash
$ git clone remote://path/to/my_app.git my_app_source
$ cd my_app_source
$ mix deps.get --only prod
$ MIX_ENV=prod mix release
$ _build/prod/rel/my_app/bin/my_app start
```

If `mix release` is invoked with no releases configured, a release is assembled using the application name and default values. A specific release is selected positionally:

```bash
$ MIX_ENV=prod mix release demo
```

The `:path` option default references the active environment: it defaults to `"_build/MIX_ENV/rel/RELEASE_NAME"`. Because `:build_per_environment` is the default, the assembled release lives under `_build/prod/`.

#### Command-line flags

From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

| Flag | Effect |
|---|---|
| `--force` | Forces recompilation. |
| `--no-archives-check` | Does not check archive. |
| `--no-deps-check` | Does not check dependencies. |
| `--no-elixir-version-check` | Does not check Elixir version. |
| `--no-compile` | Does not compile before assembling the release. |
| `--overwrite` | Overwrite existing files instead of prompting the user for action. |
| `--path` | The path of the release. |
| `--quiet` | Does not write progress to the standard output. |
| `--version` | The version of the release. |

The release name is supplied as a positional argument (`mix release NAME`); there is no `--name` flag. `--overwrite` is the common flag in CI, where prompting for confirmation is not possible.

### Defining releases in mix.exs

Releases are declared under the `:releases` key inside `project/0`. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "A release can be configured in your `mix.exs` file under the `:releases` key inside `def project`:
> ```
> def project do
>   [
>     releases: [
>       demo: [
>         include_executables_for: [:unix],
>         applications: [runtime_tools: :permanent]
>       ],
>       ...
>     ]
>   ]
> end
> ```
> You can specify multiple releases where the key is the release name and the value is a keyword list with the release configuration. Releasing a certain name is done with:
> ```
> $ MIX_ENV=prod mix release demo
> ```
> If the given name does not exist, an error is raised.
> If `mix release` is invoked, without specifying a release name, and there are multiple releases configured, an error will be raised unless you set `default_release: NAME` at the root of your project configuration.
> If `mix release` is invoked and there are no releases configured, a release is assembled using the application name and default values."

A release definition may be an anonymous function, which is useful when an attribute is expensive to compute:

```elixir
def project do
  [
    releases: [
      demo: fn ->
        [version: @version <> "+" <> git_ref()]
      end
    ]
  ]
end
```

> **Note:** The conventional wiring is a private `releases/0` function, mirroring the `deps/0` pattern. The active environment for release assembly is the single global `MIX_ENV` (typically `prod`); there is no per-environment release mapping in Mix v1.20.2.

### Release options

The following options are documented on [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html). Each is a key in the per-release keyword list.

| Option | Default | Effect |
|---|---|---|
| `:applications` | current app + all deps, recursively | Keyword list of `{app, mode}`. Includes new apps or changes the mode of existing ones. Modes: `:permanent` (default), `:transient`, `:temporary`, `:load`, `:none`. |
| `:cookie` | random cookie written to `releases/COOKIE` | String Erlang Distribution cookie. At runtime, `RELEASE_COOKIE` is tried first, then `releases/COOKIE`. |
| `:version` | current app version | Release version as a string, or `{:from_app, app_name}` (useful in umbrellas). |
| `:path` | `"_build/MIX_ENV/rel/RELEASE_NAME"` | Where the release is installed. |
| `:include_executables_for` | `[:unix, :windows]` | List of atoms naming the OSes for which executable scripts are generated (`:unix`, `:windows`, or `[]`). |
| `:include_erts` | `true` | Boolean, string path to an ERTS install, or arity-0 function returning either. `true` is recommended; `false` disables hot code upgrades and requires an exact ERTS version match on the target. |
| `:strip_beams` | `true` | Strip debug info, docs, and non-essential metadata from BEAM files. Also accepts `[keep: ["Docs", "Dbgi"]]` and `[compress: true]`. |
| `:validate_compile_env` | `true` | Validates runtime config against `Application.compile_env/3` markers; mismatches fail to boot. |
| `:rel_templates_path` | `"rel"` | Path to template files (`vm.args.eex`, `remote.vm.args.eex`, `env.sh.eex`/`env.bat.eex`, and `overlays`). |
| `:overlays` | `["rel/overlays"]` | Extra directories copied as-is to the release root. |
| `:steps` | `[:assemble]` | Custom step pipeline; must contain the `:assemble` atom; may contain `:tar`. |
| `:skip_mode_validation_for` | `[]` | Applications to skip strict "unsafe" mode validation. Use with care. |
| `:quiet` | `false` | Suppress step progress output. |
| `:runtime_config_path` | `"config/runtime.exs"` | Path resolved at build time; set to `false` to omit runtime config from the release. |

#### `:applications` and modes

From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "`:applications` - a keyword list with application names as keys and their mode as value. By default `:applications` includes the current application and all applications the current application depends on, recursively... The supported values are:
> - `:permanent` (default) - the application is started and the node shuts down if the application terminates, regardless of reason
> - `:transient` - the application is started and the node shuts down if the application terminates abnormally
> - `:temporary` - the application is started and the node does not shut down if the application terminates
> - `:load` - the application is only loaded
> - `:none` - the application is part of the release but it is neither loaded nor started"

The recommended mode for all applications is `:permanent`.

#### `:cookie`

From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "`:cookie` - a string representing the Erlang Distribution cookie. If this option is not set, a random cookie is written to the `releases/COOKIE` file when the first release is assembled. At runtime, we will first attempt to fetch the cookie from the `RELEASE_COOKIE` environment variable and then we'll read the `releases/COOKIE` file.
> If you are setting this option manually, we recommend the cookie option to be a long and randomly generated string, such as: `Base.encode32(:crypto.strong_rand_bytes(40))`."

#### `:include_erts`

From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "`:include_erts` - a boolean, string, or anonymous function of arity zero... You may also set this option to `false` if you desire to use the ERTS version installed on the target. Note, however, that the ERTS version on the target must have **the exact version** as the ERTS version used when the release is assembled. Setting it to `false` also disables hot code upgrades. Therefore, `:include_erts` should be set to `false` with caution and only if you are assembling the release on the same server that runs it."

### Custom steps (`:steps`) and tarballs

From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "It is possible to add one or more steps before and after the release is assembled. This can be done with the `:steps` option:
> ```
> releases: [
>   demo: [
>     steps: [&set_configs/1, :assemble, &copy_extra_files/1]
>   ]
> ]
> ```
> The `:steps` option must be a list and it must always include the atom `:assemble`, which does most of the release assembling. You can pass anonymous functions before and after the `:assemble` to customize your release assembling pipeline. Those anonymous functions will receive a `Mix.Release` struct and must return the same or an updated `Mix.Release` struct. It is also possible to build a tarball of the release by passing the `:tar` step anywhere after `:assemble`. If the release `:path` is not configured, the tarball is created in `_build/MIX_ENV/RELEASE_NAME-RELEASE_VSN.tar.gz` Otherwise it is created inside the configured `:path`."

The `Mix.Release` struct's `:steps` field has the type `[(t() -> t()) | :assemble, ...]` and the struct is documented in [Mix.Release.html](https://hexdocs.pm/mix/Mix.Release.html). Developers should not modify release files directly after assembly; from [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "We document this structure for completeness. In practice, developers should not modify any of those files after the release is assembled. Instead use env scripts, custom config provider, overlays, and all other mechanisms described here to configure how your release works."

### Overlays

From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "Often it is necessary to copy extra files to the release root after the release is assembled. This can be easily done by placing such files in the `rel/overlays` directory. Any file in there is copied as is to the release root. For example, if you have placed a `rel/overlays/Dockerfile` file, the \"Dockerfile\" will be copied as is to the release root.
> If you want to specify extra overlay directories, you can do so with the `:overlays` option. If you need to copy files dynamically, see the \"Steps\" section."

For custom steps that add files, populate the struct's `:overlays` field (a list of paths relative to the release root) so the files are considered by later commands such as `:tar`.

### Runtime configuration (`config/runtime.exs`)

Runtime configuration belongs in `config/runtime.exs`. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "Your `config/runtime.exs` file needs to follow three important rules:
> - It MUST `import Config` at the top instead of the deprecated `use Mix.Config`
> - It MUST NOT import any other configuration file via `import_config`
> - It MUST NOT access `Mix` in any way, as `Mix` is a build tool and it is not available inside releases
>
> If a `config/runtime.exs` exists, it will be copied to your release and executed early in the boot process, when only Elixir and Erlang's main applications have been started."

This contrasts with build-time configuration in `config/config.exs`, which is evaluated when the release is assembled. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "We say that this configuration is a build-time configuration as it is evaluated whenever you compile your code or whenever you assemble the release... if your configuration does something like:
> ```
> import Config
> config :my_app, :secret_key, System.fetch_env!(\"MY_APP_SECRET_KEY\")
> ```
> The `:secret_key` key under `:my_app` will be computed on the host machine, whenever the release is built. Therefore if the machine assembling the release not have access to all environment variables used to run your code, loading the configuration will fail as the environment variable is missing."

`config_env/0` is available inside `runtime.exs`. From [Config.html](https://hexdocs.pm/elixir/Config.html):

> "`config_env()` - Returns the environment this configuration file is executed on. In Mix projects this function returns the environment this configuration file is executed on. In releases, returns the `MIX_ENV` specified when running `mix release`."

To detect that code is running inside a release, check release-specific environment variables. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "If you want to detect you are inside a release, you can check for release specific environment variables, such as `RELEASE_NODE` or `RELEASE_MODE`"

> **Note:** In Mix v1.20.2 the documented release-detection mechanism is reading `RELEASE_*` environment variables; there is no `releases()` or `release_name()` helper documented for use inside `runtime.exs`. To change which file is used, set `:runtime_config_path` per release; set it to `false` to omit runtime config entirely.

### Config providers

Beyond `runtime.exs`, releases support custom config providers that load external configuration during boot. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "`:config_providers` - a list of tuples with custom config providers. See `Config.Provider` for more information. Defaults to `[]`."

> "Releases also supports custom mechanisms, called config providers, to load any sort of runtime configuration to the system while it boots. For instance, if you need to access a vault or load configuration from a JSON file, it can be achieved with config providers. The runtime configuration outlined in the previous section is handled by the `Config.Reader` provider."

Each provider is a `{module, term}` tuple where the module implements the `Config.Provider` behaviour. The `term` is passed to `Config.Provider.init/1` (run at assembly time) and the resulting state is passed to `load/2` (run at boot). From [Config.Provider.html](https://hexdocs.pm/elixir/Config.Provider.html):

> "`init(term)` - Invoked when initializing a config provider... because the state returned by `init/1` can be written to text-based config files, it should be restricted only to simple data types...
> `load(config, state)` - Loads configuration (typically during system boot)... Merging should be done with `Config.Reader.merge/2`, as it performs deep merge. It should return the updated config."

A custom provider that reads JSON, from [Config.Provider.html](https://hexdocs.pm/elixir/Config.Provider.html):

```elixir
defmodule JSONConfigProvider do
  @behaviour Config.Provider

  @impl true
  def init(path) when is_binary(path), do: path

  @impl true
  def load(config, path) do
    {:ok, _} = Application.ensure_all_started(:jason)
    json = path |> File.read!() |> Jason.decode!()

    Config.Reader.merge(
      config,
      my_app: [
        some_value: json["my_app_some_value"],
        another_value: json["my_app_another_value"]
      ]
    )
  end
end
```

```elixir
# mix.exs
releases: [
  demo: [
    config_providers: [{JSONConfigProvider, "/etc/config.json"}]
  ]
]
```

Several release options tune the config-provider boot sequence (documented under "Config providers" in [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html)):

| Option | Default | Effect |
|---|---|---|
| `:reboot_system_after_config` | `true` only with deprecated `config/releases.exs`, else `false` | Reboots the system after config so `:kernel`/`:stdlib` can be configured in `runtime.exs`. When `true`, the release boots in interactive mode, computes config into `tmp`, then reboots in `RELEASE_MODE`. |
| `:start_distribution_during_config` | `false` | Starts Erlang VM distribution during config evaluation. |
| `:prune_runtime_sys_config_after_boot` | `false` | Removes the computed config file after boot; disables internal `System.restart/0` and `bin/APP restart`. |

### vm.args, env.sh, and `mix release.init`

VM flags and environment variables are customized through EEx template files under `:rel_templates_path` (default `"rel"`). `mix release.init` scaffolds them. From [Mix.Tasks.Release.Init.html](https://hexdocs.pm/mix/Mix.Tasks.Release.Init.html):

> "Generates sample files for releases.
> ```
> $ mix release.init
> * creating rel/vm.args.eex
> * creating rel/remote.vm.args.eex
> * creating rel/env.sh.eex
> * creating rel/env.bat.eex
> ```"

From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "Developers may want to customize the VM flags and environment variables given when the release starts. The simplest way to customize those files is by running `mix release.init`. The Mix task will copy custom `rel/vm.args.eex`, `rel/remote.vm.args.eex`, `rel/env.sh.eex`, and `rel/env.bat.eex` files to your project root. You can modify those files and they will be evaluated every time you perform a new release. Those files are regular EEx templates and they have a single assign, called `@release`, with the `Mix.Release` struct."

The role of each template, from [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "- `rel/vm.args.eex` and `rel/remote.vm.args.eex` - template files that are copied into every release and provides static configuration of the Erlang Virtual Machine and other runtime flags. `vm.args` runs on `start`, `daemon`, and `eval` commands. `remote.vm.args` configures the VM for `remote` and `rpc` commands
> - `rel/env.sh.eex` and `rel/env.bat.eex` - template files that are copied into every release and are executed on every command to set up environment variables, including specific ones to the VM, and the general environment"

The `env.sh` (Unix) and `env.bat` (Windows) scripts are rendered from those templates and placed at `releases/RELEASE_VSN/env.sh` and `env.bat` inside the assembled release. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "The `env.sh` and `env.bat` is used to set environment variables. In there, you can set vars such as `RELEASE_NODE`, `RELEASE_COOKIE`, and `RELEASE_TMP` to customize your node name, cookie and tmp directory respectively. Whenever `env.sh` or `env.bat` is invoked, the variables `RELEASE_ROOT`, `RELEASE_NAME`, `RELEASE_VSN`, and `RELEASE_COMMAND` have already been set, so you can rely on them."

Because `vm.args` is static, dynamic VM options are set via `env.sh` using `ELIXIR_ERL_OPTIONS`. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

```bash
case $RELEASE_COMMAND in
  start*|daemon*)
    ELIXIR_ERL_OPTIONS="-kernel inet_dist_listen_min $BEAM_PORT inet_dist_listen_max $BEAM_PORT"
    export ELIXIR_ERL_OPTIONS
    ;;
  *)
    ;;
esac
```

> **Note:** `mix release.init` does not create `rel/overlays`; that directory is created manually when you need to copy extra files. The official file names are `env.sh`/`env.bat` (rendered) and `env.sh.eex`/`env.bat.eex` (templates) — there is no `_env` file in the Mix v1.20.2 release tooling.

### Environment variables

Releases are controlled at runtime through `RELEASE_*` environment variables. Some are set early and are only readable from `env.sh`/`env.bat`; others may be set before invoking the release. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

| Variable | Settable? | Purpose |
|---|---|---|
| `RELEASE_ROOT` | computed only | Root of the release; equals `:code.root_dir/0` when ERTS is included. |
| `RELEASE_COMMAND` | computed only | The command given (`start`, `remote`, `eval`, ...); may be empty/unvalidated in `env.sh`. |
| `RELEASE_PROG` | computed | The command-line executable used to start the release. |
| `RELEASE_NAME` | settable | The release name. |
| `RELEASE_VSN` | settable | The release version; must exist under `releases/`; otherwise the latest is used. |
| `RELEASE_COOKIE` | settable | Release cookie; defaults to `releases/COOKIE`. |
| `RELEASE_NODE` | settable | Node name, `name` or `name@host` in distributed mode. |
| `RELEASE_SYS_CONFIG` | settable | Location of `sys.config` (without the `.config` extension). |
| `RELEASE_VM_ARGS` | settable | Location of the `vm.args` file. |
| `RELEASE_REMOTE_VM_ARGS` | settable | Location of the `remote.vm.args` file. |
| `RELEASE_TMP` | settable | Temp dir for the release; defaults to `$RELEASE_ROOT/tmp`. |
| `RELEASE_MODE` | settable | `embedded` (default) or `interactive`; applies to `start`/`daemon`/`install`. |
| `RELEASE_DISTRIBUTION` | settable | `name` (long), `sname` (short, default), or `none`. |
| `RELEASE_BOOT_SCRIPT` | settable | Boot script name for `start`/`daemon`; defaults to `start`. |
| `RELEASE_BOOT_SCRIPT_CLEAN` | settable | Clean boot script for `eval`/`rpc`/`remote`; defaults to `start_clean`. |

The cookie precedence is: the `RELEASE_COOKIE` environment variable is checked first, then the `releases/COOKIE` file written from the `:cookie` option.

### Release modes: embedded vs interactive

`RELEASE_MODE` selects how the VM loads code. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "`RELEASE_MODE` - if the release should load code on demand (interactive) or preload it (embedded). Defaults to \"embedded\", which increases boot time but it means the runtime will respond faster as it doesn't have to load code. Choose interactive if you need to decrease boot time and reduce memory usage on boot. It applies only to start/daemon/install commands"

> **Note:** The Mix v1.20.2 docs name the two modes **embedded** and **interactive**. "Standalone" is not a release mode in the official docs (it refers to the older escript/`:standalone` concept, which is unrelated). When `:reboot_system_after_config` is `true`, the release first boots in interactive mode to compute config, then reboots in the configured `RELEASE_MODE`.

### `ERL_AFLAGS` and VM flags

`ERL_AFLAGS` is used to pass flags to the Erlang VM in release build contexts. The documented use is when cross-building images under emulation, where the JIT may not behave correctly. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "However, when building such images on your machine, those technologies use emulation which may not interplay well with Erlang VM's JIT (just-in time) compiler. To address this, you can set this environment variable on your build stage:
> ```
> ENV ERL_AFLAGS \"+JMsingle true\"
> ```"

> **Note:** In the Mix v1.20.2 release docs only `ERL_AFLAGS` is documented; `ERL_ZFLAGS` is not mentioned. For runtime VM flags inside an assembled release, use the `ELIXIR_ERL_OPTIONS` variable in `env.sh` (shown above) rather than `ERL_AFLAGS`.

### Starting, stopping, and managing releases

An assembled release exposes a `bin/RELEASE_NAME` script. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

| Command | Description |
|---|---|
| `start` | Starts the system. |
| `start_iex` | Starts the system with IEx attached. |
| `daemon` | Starts the system as a daemon (Unix-like only). |
| `daemon_iex` | Starts the system as a daemon with IEx attached (Unix-like only). |
| `install` | Installs this system as a Windows service (Windows only). |
| `eval "EXPR"` | Executes the given expression on a new, non-booted system. |
| `rpc "EXPR"` | Executes the given expression remotely on the running system. |
| `remote` | Connects to the running system via a remote shell. |
| `restart` | Restarts the running system via a remote command. |
| `stop` | Stops the running system via a remote command. |
| `pid` | Prints the operating system PID of the running system via a remote command. |
| `version` | Prints the release name and version to be booted. |

`eval` and `rpc` run code in the release:

```bash
$ bin/RELEASE_NAME eval "IO.puts(:hello)"
$ bin/RELEASE_NAME rpc "IO.puts(:hello)"
```

From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "The `eval` command starts its own instance of the VM but without starting any of the applications in the release and without starting distribution."

Shutting down is done by signal or explicitly:

> "Once a system is deployed, shutting down the system can be done by sending SIGINT/SIGTERM to the system, which is what most containers, platforms and tools do, or by explicitly invoking `bin/RELEASE_NAME stop`. Once the system receives the shutdown request, each application and their respective supervision trees will stop, one by one, in the opposite order that they were started."

On Unix, daemon mode backgrounds via `run_erl` and logs to `tmp/log/`; on Windows, `install` registers an `erlsrv` service that is not started automatically.

### Umbrella releases

Releases integrate with umbrella projects and allow releasing subsets of children. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

> "Releases are well integrated with umbrella projects, allowing you to release one or more subsets of your umbrella children. The only difference between performing a release in the umbrella project compared to a regular application is that umbrellas require you to explicitly list your release and the starting point for each release."

```elixir
releases: [
  web_and_event_processing: [
    applications: [
      my_app_event_processing: :permanent,
      my_app_web: :permanent
    ]
  ],
  web_only: [
    applications: [my_app_web: :permanent]
  ],
  event_processing_only: [
    applications: [my_app_event_processing: :permanent]
  ]
]
```

> "Note you don't need to define all applications in `:applications`, only the entry points. Also remember that the recommended mode for all applications in the system is `:permanent`."

`{:from_app, app_name}` for `:version` is particularly useful in umbrellas to reference another application's version. Releases may be assembled from the umbrella root or from an individual child.

### Elixir releases vs Distillery

`mix release` shipped in Elixir v1.9 as the official, built-in release assembler and is the canonical mechanism documented by the Mix v1.20.2 docs. The docs assume `mix release` as the only release tooling and never reference external release packagers. Distillery was the de-facto third-party release tool before Elixir v1.9; with `mix release` now built into Elixir, Distillery is considered legacy/superseded and is not referenced by the current docs. New projects should use `mix release`. (The "Distillery is deprecated" framing comes from ecosystem guides and the Distillery project itself, not from `Mix.Tasks.Release.html`.)

### Release directory structure

A release is organized as follows. From [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html):

```text
bin/
  RELEASE_NAME
erts-ERTS_VSN/
lib/
  APP_NAME-APP_VSN/
    ebin/
    include/
    priv/
releases/
  RELEASE_VSN/
    consolidated/
    elixir
    elixir.bat
    env.bat
    env.sh
    iex
    iex.bat
    remote.vm.args
    runtime.exs
    start.boot
    start.script
    start_clean.boot
    start_clean.script
    sys.config
    vm.args
  COOKIE
  start_erl.data
tmp/
```

### Cross-references

For full details, see [Mix.Tasks.Release.html](https://hexdocs.pm/mix/Mix.Tasks.Release.html), [Mix.Tasks.Release.Init.html](https://hexdocs.pm/mix/Mix.Tasks.Release.Init.html), and the [Mix.Release](https://hexdocs.pm/mix/Mix.Release.html) module (struct fields and typespecs). Config providers are documented in [Config.Provider.html](https://hexdocs.pm/elixir/Config.Provider.html) and runtime configuration in [Config.html](https://hexdocs.pm/elixir/Config.html). Runtime vs build-time configuration is also discussed under "Configuration files" earlier in this document.

## Archives

Mix archives are ZIP-based packages (with the `.ez` extension) that install new Mix tasks locally so they become available across all Mix projects on a machine. Archives are the standard mechanism for distributing Mix itself's extensions — Hex and Rebar are both installed as archives — and for shipping small project-independent tooling that should run as `mix my_task` from anywhere.

From [Mix.Tasks.Archive.Build.html](https://hexdocs.pm/mix/Mix.Tasks.Archive.Build.html):

> "Archives are designed to contain small projects installed locally. They become available to all Mix projects once installed."

> "archives do not include dependencies, as those would conflict with any dependency in a Mix project after the archive is installed. In general, we recommend the usage of archives to be limited for extensions of Mix, such as custom SCMs, package managers, and the like. For general scripts to be distributed to developers, please see `mix escript.build`."

### The .ez file format

An archive is an Erlang archive: a ZIP file with the `.ez` extension whose internal layout mirrors a regular OTP application directory. From the Erlang [`code` module docs](https://www.erlang.org/doc/apps/kernel/code.html), archiving an application such as `mnesia-4.4.7` produces `mnesia-4.4.7.ez` containing a top directory `mnesia-4.4.7` with the usual `ebin/`, `priv/`, etc. The code path may then reference the archive as a virtual path, e.g. `$OTPROOT/lib/mnesia-4.4.7.ez/mnesia-4.4.7/ebin`. Best practice is to store `.beam` and `.app` files uncompressed inside the archive for fast code loading.

### Where archives live

Installed archives are stored under `~/.mix/archives` by default. The directory is controlled by the `MIX_ARCHIVES` environment variable. From [Mix.Tasks.Archive.html](https://hexdocs.pm/mix/Mix.Tasks.Archive.html):

> "Archives install at `~/.mix/archives` by default; the path can be customized via the `MIX_ARCHIVES` env var. Archives are version-specific to Elixir; build tools can swap `MIX_ARCHIVES` based on the Elixir installation."

`Mix.path_for/1` (since v1.10.0) returns the path for local archives or escripts:

```elixir
@spec path_for(:archives | :escripts) :: String.t()
```

Related environment variables (documented in [Mix.html](https://hexdocs.pm/mix/Mix.html)):

| Variable | Default | Purpose |
|---|---|---|
| `MIX_ARCHIVES` | `~/.mix/archives` | Directory where archives are installed. |
| `MIX_HOME` | `~/.mix` | Mix home directory for configs and scripts. |
| `MIX_XDG` | unset | Follow the XDG Base Directory spec when set. |

### mix archive

`mix archive` lists all installed archives. From [Mix.Tasks.Archive.html](https://hexdocs.pm/mix/Mix.Tasks.Archive.html):

```bash
$ mix archive
```

Because archives are version-specific to Elixir, build tools and version managers typically swap `MIX_ARCHIVES` per Elixir installation so that an archive built for one Elixir version is not loaded against another.

### mix archive.install

`mix archive.install` installs an archive locally. From [Mix.Tasks.Archive.Install.html](https://hexdocs.pm/mix/Mix.Tasks.Archive.Install.html), supported argument forms:

```bash
mix archive.install archive.ez
mix archive.install path/to/archive.ez
mix archive.install git https://path/to/git/repo
mix archive.install git https://path/to/git/repo branch git_branch
mix archive.install git https://path/to/git/repo tag git_tag
mix archive.install git https://path/to/git/repo ref git_ref
mix archive.install github user/project
mix archive.install github user/project branch git_branch
mix archive.install github user/project tag git_tag
mix archive.install github user/project ref git_ref
mix archive.install hex hex_package
mix archive.install hex hex_package 1.2.3
```

A local path installs a pre-built `.ez` directly; Git/GitHub/Hex sources fetch the source and **build** the archive before installing it. If no argument is supplied but a prebuilt archive exists in the project root (created by `mix archive.build`), it is installed — this is the basis of the common `mix do archive.build + archive.install` workflow.

After installation, the tasks the archive provides become available locally as `mix some_task` from any project.

> **Security:** From [Mix.Tasks.Archive.Install.html](https://hexdocs.pm/mix/Mix.Tasks.Archive.Install.html): "Archives must be installed only from sources you trust. Installing an archive from Git, GitHub, or Hex executes code from the source during installation, unless a pre-built archive is given. Once an archive is installed, Mix may load code from it as a plugin on any Mix command, even if no archive command is executed."

Command-line options:

| Option | Effect |
|---|---|
| `--sha512` | Verify a SHA-512 checksum (local-path installs only). |
| `--force` | Skip the shell prompt (for build automation such as Make). |
| `--submodules` | Fetch git submodules before building. |
| `--sparse` | Checkout a single directory from the Git repo and use it as the archive root. |
| `--app` | Custom app name when building from Git/GitHub/Hex. |
| `--organization` | For Hex private packages belonging to an organization. |
| `--repo` | Self-hosted Hex instance; defaults to `hexpm`. |

### mix archive.uninstall

`mix archive.uninstall` removes an installed archive. From [Mix.Tasks.Archive.Uninstall.html](https://hexdocs.pm/mix/Mix.Tasks.Archive.Uninstall.html):

```bash
$ mix archive.uninstall archive.ez
```

It accepts `--force` to skip the confirmation prompt (useful in automation).

### mix archive.build

`mix archive.build` builds an archive in the Erlang `.ez` format. From [Mix.Tasks.Archive.Build.html](https://hexdocs.pm/mix/Mix.Tasks.Archive.Build.html), the default behaviour is:

```bash
mix archive.build -i _build/ENV/lib/APP -o APP-VERSION.ez
```

Command-line options:

| Option | Default | Effect |
|---|---|---|
| `-o` | `APP-VERSION.ez` (when `mix.exs` exists) | Output file name. |
| `-i` | `_build/ENV/lib/APP` (when `mix.exs` exists) | Input directory to archive. |
| `--no-compile` | off | Skip compilation (only when `mix.exs` is available). |
| `--include-dot-files` | off | Include dot files from the `priv` directory. |

The generated `.gitignore` from `mix new` already ignores archive artifacts via `*.ez` (see [Project Directory Structure](#project-directory-structure)).

### How archives extend Mix with local tasks

An archive ships Mix task modules named `Mix.Tasks.*`. When installed, the archive's `ebin/` directories are added to the Erlang code path (via `MIX_ARCHIVES`); Mix auto-loads tasks from code paths, so any `Mix.Tasks.*` modules inside the archive become available as `mix my_task` from any project. This is the mechanism Hex and Rebar use: `mix archive.install hex hex` and `mix archive.install github rebar/rebar3`.

### mix local and related tasks

The `mix local.*` family manages locally-installed tooling. From [Mix.Tasks.Local.html](https://hexdocs.pm/mix/Mix.Tasks.Local.html):

> "`mix local` lists tasks installed locally via archives."

| Task | Description |
|---|---|
| `mix local` | Lists tasks installed locally via archives. |
| `mix local.hex [version]` | Installs Hex locally; default is the latest compatible version. Falls back to `mix archive.install github hexpm/hex branch latest` when the precompiled install fails. Options: `--force`, `--if-missing`. Mirror via `HEX_BUILDS_URL`. |
| `mix local.rebar [rebar3 PATH]` | Fetches a copy of `rebar3` from a path or URL (default: Hex's CDN). Stored in `MIX_HOME` keyed to the current Elixir version. Options: `--sha512`, `--force`, `--if-missing`. |

`--if-missing` is the recommended flag for automation scripts, since it avoids reinstalling an already-present tool.

### Archives vs. escripts vs. releases

These three packaging mechanisms serve different purposes. From [Mix.Tasks.Archive.Build.html](https://hexdocs.pm/mix/Mix.Tasks.Archive.Build.html) and [Mix.Tasks.Escript.Build.html](https://hexdocs.pm/mix/Mix.Tasks.Escript.Build.html):

| Mechanism | What it is | When to use |
|---|---|---|
| **Archive** (`.ez`) | A ZIP-based OTP application installed into `MIX_ARCHIVES` that extends Mix with new tasks. No dependencies allowed inside. Per-Elixir-version specific. | Distributing Mix extensions: custom SCMs, package managers, new `mix my_task` commands. |
| **Escript** | A self-contained executable that embeds Elixir (and optionally dependencies). Runs on any machine with Erlang/OTP installed; Elixir need not be present. | Distributing standalone scripts/CLIs to other developers. Not a deployment mechanism. |
| **Release** | A self-contained production artifact assembled by `mix release` that includes ERTS, all code, and management scripts. | Production deployment. See [Releases](#releases). |

From [Mix.Tasks.Escript.Build.html](https://hexdocs.pm/mix/Mix.Tasks.Escript.Build.html):

> "An escript can run on any machine that has Erlang/OTP installed and by default does not require Elixir to be installed, as Elixir is embedded as part of the escript."

> "Escripts should be used as a mechanism to share scripts between developers and not as a deployment mechanism. For running live systems, consider using `mix run` or building releases."

An escript requires an `:escript` key in `mix.exs` with a `:main_module` whose module exports `main/1`:

```elixir
defmodule MyApp.MixProject do
  use Mix.Project

  def project do
    [app: :my_app, version: "0.0.1", escript: escript()]
  end

  def escript do
    [main_module: MyApp.CLI]
  end
end

defmodule MyApp.CLI do
  def main(_args) do
    IO.puts("Hello from MyApp!")
  end
end
```

### When to use archives

Use archives when you need to ship a small, dependency-free Mix extension that should be invokable as `mix my_task` from any project on a developer's machine. Do not use archives to distribute application code, libraries with dependencies, or production artifacts — use Hex packages, escripts, or releases respectively. Because archives execute code at install time and may be loaded as plugins on any Mix command, install them only from trusted sources.

### Cross-references

For full details, see [Mix.Tasks.Archive.html](https://hexdocs.pm/mix/Mix.Tasks.Archive.html), [Mix.Tasks.Archive.Install.html](https://hexdocs.pm/mix/Mix.Tasks.Archive.Install.html), [Mix.Tasks.Archive.Uninstall.html](https://hexdocs.pm/mix/Mix.Tasks.Archive.Uninstall.html), [Mix.Tasks.Archive.Build.html](https://hexdocs.pm/mix/Mix.Tasks.Archive.Build.html), [Mix.Tasks.Local.html](https://hexdocs.pm/mix/Mix.Tasks.Local.html), [Mix.Tasks.Local.Hex.html](https://hexdocs.pm/mix/Mix.Tasks.Local.Hex.html), [Mix.Tasks.Local.Rebar.html](https://hexdocs.pm/mix/Mix.Tasks.Local.Rebar.html), and [Mix.Tasks.Escript.Build.html](https://hexdocs.pm/mix/Mix.Tasks.Escript.Build.html). The `Mix.path_for/1` function and `MIX_ARCHIVES`/`MIX_HOME` environment variables are documented in [Mix.html](https://hexdocs.pm/mix/Mix.html). The `.ez` archive format is documented in the Erlang [`code` module](https://www.erlang.org/doc/apps/kernel/code.html).

## Custom Mix Tasks

A short orientation to writing custom Mix tasks appears under [Mix Overview](#writing-custom-mix-tasks). This section is the in-depth reference: the `Mix.Task` behaviour, its module attributes, the task-discovery mechanism, and the `Mix.Task` module API for running tasks programmatically.

### The Mix.Task behaviour

Custom tasks implement the `Mix.Task` behaviour. From [Mix.Task.html](https://hexdocs.pm/mix/Mix.Task.html), creating a task requires three steps:

> "To create a new Mix task, you'll need to:
> 1. Create a module whose name begins with `Mix.Tasks.` (for example, `Mix.Tasks.MyTask`).
> 2. Call `use Mix.Task` in that module.
> 3. Implement the `Mix.Task` behaviour in that module (that is, implement the `run/1` callback)."

The behaviour declares a single required callback:

```elixir
@callback run(command_line_args :: [binary()]) :: any
```

`run/1` receives the raw command-line arguments as a list of binaries, exactly as the user typed them after the task name. For example, `mix echo 'A and B' C --test` calls `run(["A and B", "C", "--test"])`. Calling `use Mix.Task` sets `@behaviour Mix.Task` and applies default values for the documented module attributes.

### Task discovery and naming

Mix discovers tasks by the `Mix.Tasks.*` module-name pattern. From [Mix.Task.html](https://hexdocs.pm/mix/Mix.Task.html):

> "The command name will correspond to the portion of the module name following `Mix.Tasks.`. For example, a module name of `Mix.Tasks.Deps.Clean` corresponds to a task name of `deps.clean`."

Task modules are conventionally placed under `lib/mix/tasks/`, and their file names use dot separators instead of underscores (e.g. `deps.clean.ex`), although the file name is not relevant to discovery. Mix loads tasks from code paths via `Mix.Task.load_all/0` and `Mix.Task.load_tasks/1`; built-in tasks ship as `Mix.Tasks.<Name>` modules inside Elixir itself and follow the same convention. Archives (see [Archives](#archives)) and dependencies contribute their own `Mix.Tasks.*` modules to the code path, which is how their tasks become available.

### Module attributes

A task module supports the following attributes. From [Mix.Task.html](https://hexdocs.pm/mix/Mix.Task.html):

| Attribute | Effect |
|---|---|
| `@shortdoc` | One-line description shown in `mix help` listings. Omit it to keep the task out of `mix help`. |
| `@moduledoc` | Multi-paragraph documentation shown by `mix help my_task`. Overrides `@shortdoc` for the help-list display. `@moduledoc false` hides the task from `mix help` entirely. |
| `@recursive` | Set to `true` so the task runs on each umbrella child in an umbrella project. |
| `@preferred_cli_env` | Sets the preferred Mix environment for the task (e.g. `@preferred_cli_env :test`). Deprecated in favour of `cli/0` in `mix.exs` (see [CLI configuration](#cli-configuration)). |
| `@requirements` (since v1.11.0) | Lists other Mix tasks that must run before this one, e.g. `@requirements ["app.config"]`. |

The common `@requirements` values and their semantics:

| Requirement | What it ensures |
|---|---|
| `"loadpaths"` | Dependencies are available and compiled. Recommended when a task does not interact with user code. |
| `"app.config"` | Additionally compiles and loads runtime configuration. Recommended minimum when a task must invoke or interact with user code. |
| `"app.start"` | Additionally starts the supervision tree of the project and dependencies. |

### A complete custom task

```elixir
# lib/mix/tasks/greet.ex
defmodule Mix.Tasks.Greet do
  @moduledoc """
  Greets a name given on the command line.

  ## Usage

      mix greet World

  """
  @shortdoc "Greets a name"

  use Mix.Task

  @impl Mix.Task
  def run(args) do
    name = List.first(args) || "stranger"
    Mix.shell().info("Hello, #{name}!")
  end
end
```

Invoke with `mix greet World`. The `@impl Mix.Task` annotation marks `run/1` as the behaviour callback implementation and silences warnings about unimplemented callbacks.

### Parsing arguments with OptionParser

For any non-trivial task, parse `args` with `OptionParser` (see [OptionParser.html](https://hexdocs.pm/elixir/OptionParser.html)). `OptionParser.parse/2` returns `{parsed, remaining, invalid}`; `OptionParser.parse_head/2` splits flags from a trailing list of positional arguments.

```elixir
defmodule Mix.Tasks.Greet do
  @shortdoc "Greets a name"
  use Mix.Task

  @impl Mix.Task
  def run(args) do
    {opts, name, _} =
      OptionParser.parse(args, switches: [loud: :boolean], aliases: [l: :loud])

    greeting = "Hello, #{List.first(name) || "stranger"}!"
    Mix.shell().info(if opts[:loud], do: String.upcase(greeting), else: greeting)
  end
end
```

### Recursive tasks in umbrellas

Setting `@recursive true` makes Mix invoke the task once per umbrella child. From [Mix.Task.html](https://hexdocs.pm/mix/Mix.Task.html):

> "Set `@recursive true` if you want the task to run on each umbrella child in an umbrella project."

Inside a recursive task, `Mix.Task.recursing?/0` (since v1.8.0) returns `true` while the task is being run for umbrella children, so the body can adapt its behaviour. To run a recursive task on a specific subset of children, use `Mix.Task.run_in_apps/3` (since v1.14.0).

### The Mix.Task module API

Beyond the `run/1` callback, the `Mix.Task` module provides functions for loading, running, and introspecting tasks programmatically. From [Mix.Task.html](https://hexdocs.pm/mix/Mix.Task.html):

| Function | Purpose |
|---|---|
| `Mix.Task.run(task, args \\ [])` | Runs a task/alias if it has not already been invoked; returns `:noop` if it has. Tasks run at most once per process by default. |
| `Mix.Task.rerun(task, args \\ [])` | Re-enables a task and runs it again. |
| `Mix.Task.reenable(task)` | Re-enables a task so it can run again down the stack. In an umbrella, re-enables for all child projects. |
| `Mix.Task.get(task)` / `get!(task)` | Looks up the module for a task name; `get!/1` raises `Mix.NoTaskError` or `Mix.InvalidTaskError`. |
| `Mix.Task.task?(module)` | Returns `true` if `module` implements the `Mix.Task` behaviour. |
| `Mix.Task.task_name(module)` | Returns the task name for a module, e.g. `Mix.Task.task_name(Mix.Tasks.Test)` ⇒ `"test"`. |
| `Mix.Task.alias?(task)` | Returns whether the name is an alias. |
| `Mix.Task.all_modules()` | Returns all loaded task modules. |
| `Mix.Task.load_all()` | Loads every task in all code paths. |
| `Mix.Task.load_tasks(dirs)` | Loads all tasks in the given directories. |
| `Mix.Task.shortdoc(module)` / `moduledoc(module)` | Returns the `@shortdoc` / `@moduledoc` of a task module. |
| `Mix.Task.recursive(module)` | Checks if a task is marked recursive. |
| `Mix.Task.recursing?()` (v1.8.0) | `true` if the current task is recursive and running in an umbrella. |
| `Mix.Task.requirements(module)` (v1.11.0) | Returns the list of `@requirements` for a task. |
| `Mix.Task.clear()` | Clears all invoked tasks so they may run again (non-recursive). |

The run-once rule is the reason aliases that invoke the same task multiple times must call `Mix.Task.reenable/1` between invocations (see [Tasks run only once](#tasks-run-only-once-and-reenabling)).

### Visibility in mix help

`mix help` lists all aliases, tasks, and their short descriptions. From [Mix.Tasks.Help.html](https://hexdocs.pm/mix/Mix.Tasks.Help.html):

> "$ mix help                  - prints all aliases, tasks and their short descriptions
> $ mix help TASK/ALIAS       - prints full docs for the given task/alias"

A task appears in the `mix help` listing only when it defines `@shortdoc`; setting `@moduledoc false` hides it entirely. The full documentation shown by `mix help my_task` is the task module's `@moduledoc`.

### Custom tasks vs. aliases vs. compilers

Custom `Mix.Task` modules, aliases, and custom compilers are complementary extension points:

| Mechanism | Where defined | Visibility | Use when |
|---|---|---|---|
| Custom task | `Mix.Tasks.*` module under `lib/mix/tasks/` | Public to dependents; distributable via Hex/git | A reusable, distributable command. |
| Alias | `:aliases` in `mix.exs` | Private to the defining project (umbrellas aside) | Gluing existing tasks into a project workflow. |
| Custom compiler | `Mix.Tasks.Compile.*` module using `Mix.Task.Compiler` | Public to dependents | Adding a new language/compiler to the build pipeline. See [Mix.Task.Compiler behaviour](#mix-task-compiler-behaviour). |

A custom task is referenced from an alias by its dot-separated name (e.g. `"my.task"`) and invoked programmatically with `Mix.Task.run("my.task", args)`.

### Cross-references

For full details, see [Mix.Task.html](https://hexdocs.pm/mix/Mix.Task.html) (behaviour, attributes, and the `Mix.Task` module API), [Mix.Tasks.Help.html](https://hexdocs.pm/mix/Mix.Tasks.Help.html) (task discovery and `mix help`), [OptionParser.html](https://hexdocs.pm/elixir/OptionParser.html) (argument parsing), and the authoritative source [`lib/mix/lib/mix/task.ex`](https://github.com/elixir-lang/elixir/blob/v1.20.2/lib/mix/lib/mix/task.ex). Custom compilers are documented under [Compilation](#compilation); aliases under [Aliases](#aliases).

## Review checklist

- [ ] The project has a valid `mix.exs` with `use Mix.Project`, a public `project/0` function, and an `application/0` function when the project is an OTP application.
- [ ] `:app` and `:version` keys are present in `project/0` so Mix can generate the `.app` file.
- [ ] `project/0` avoids heavy computation; expensive configuration is deferred or passed as functions.
- [ ] `Mix.env/0` is used only in `mix.exs`, config files, or Mix tasks — never in runtime `lib/` code.
- [ ] `Mix.Project.config/0` is used for project/build configuration, not runtime application configuration.
- [ ] Environment-specific and target-specific concerns use `MIX_ENV`/`MIX_TARGET`, `config_env/0`, `config_target/0`, or `Application.get_env/3` rather than `Mix.env/0` in application code.
- [ ] Custom Mix tasks live under `lib/mix/tasks/`, follow the `Mix.Tasks.*` naming convention, and implement `run/1`.
- [ ] Custom tasks declare `@shortdoc` for visibility in `mix help`, and `@moduledoc`/`@recursive`/`@preferred_cli_env` as appropriate.
- [ ] File layout follows `mix new` conventions: `lib/` for source, `test/` for tests, `.formatter.exs`, `.gitignore`, `README.md`, and `mix.exs` at the project root.
- [ ] Module-to-file path mirrors the dotted module name; test files mirror source files with `_test.exs` suffix.
- [ ] Umbrella children point `build_path`, `config_path`, `deps_path`, and `lockfile` to the umbrella root, and use `in_umbrella: true` for sibling deps.
- [ ] Runtime secrets and environment-dependent values live in `config/runtime.exs`, not `config/config.exs`.

## Implementation checklist

- [ ] Run `mix new my_app` (or `mix new my_app --sup`/`--umbrella`) to scaffold the project.
- [ ] Set `:app` to a snake_case atom matching the project name, and `:version` to a valid version string.
- [ ] Add `:extra_applications` in `application/0` for required OTP apps such as `:logger` or `:crypto`.
- [ ] Add `mod: {MyApp.Application, []}` in `application/0` when using `--sup` or a custom application callback.
- [ ] Declare dependencies in a private `deps/0` function and keep the list tidy.
- [ ] Configure `def cli` if the project needs non-default environments or preferred task environments.
- [ ] Create `config/config.exs` only when build-time configuration is needed; add `config/runtime.exs` for runtime configuration.
- [ ] Add custom tasks under `lib/mix/tasks/` and expose them via `mix help` with `@shortdoc`.
- [ ] Verify generated files with `mix compile`, `mix test`, and `mix help`.

## Validation hooks

- `mix compile` — verifies the project compiles, `.app` generation succeeds, and compiler warnings are surfaced.
- `mix test` — runs the test suite; defaults to the `:test` environment.
- `mix help` and `mix help TASK` — verify task discovery and custom task documentation.
- `mix format --check-formatted` — checks formatting against `.formatter.exs`.
- `mix deps.get` and `mix deps.unlock --unused` — manage and audit dependencies.
- `mix release` — validates release assembly when applicable.
- Note explicitly: Mix is a build tool and may be unavailable in production; do not rely on `Mix.env/0` or other Mix APIs at runtime in `lib/` code.

## Examples

### Minimal Mix project

```elixir
defmodule MyApp.MixProject do
  use Mix.Project

  def project do
    [
      app: :my_app,
      version: "1.0.0"
    ]
  end
end
```

### Typical mix.exs with dependencies

```elixir
defmodule MyApp.MixProject do
  use Mix.Project

  def project do
    [
      app: :my_app,
      version: "0.1.0",
      elixir: "~> 1.20",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      # {:dep_from_hexpm, "~> 0.3.0"},
      # {:dep_from_git, git: "https://github.com/elixir-lang/my_dep.git", tag: "0.1.0"}
    ]
  end
end
```

### Custom Mix task

```elixir
# lib/mix/tasks/echo.ex
defmodule Mix.Tasks.Echo do
  @moduledoc "Printed when the user requests `mix help echo`"
  @shortdoc "Echoes arguments"

  use Mix.Task

  @impl Mix.Task
  def run(args) do
    Mix.shell().info(Enum.join(args, " "))
  end
end
```

Invoke with:

```text
mix echo hello world
```

### Environment-specific dependency

```elixir
defp deps do
  [
    {:jason, "~> 1.4"},
    {:mox, "~> 1.0", only: :test}
  ]
end
```

### CLI configuration

```elixir
def cli do
  [
    default_env: :local,
    preferred_envs: [docs: :docs]
  ]
end
```

### Umbrella child mix.exs

```elixir
defmodule MyApp.MixProject do
  use Mix.Project

  def project do
    [
      app: :my_app,
      version: "0.1.0",
      build_path: "../../_build",
      config_path: "../../config/config.exs",
      deps_path: "../../deps",
      lockfile: "../../mix.lock",
      elixir: "~> 1.20",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:sibling_app_in_umbrella, in_umbrella: true}
    ]
  end
end
```

## Common mistakes

- Calling `Mix.env/0` inside `lib/` modules to change runtime behaviour — use `Application.get_env/3` instead.
- Treating `config/config.exs` as runtime configuration — it is build-time configuration.
- Putting secrets or environment-specific values directly into `config/config.exs` instead of `config/runtime.exs` or environment variables.
- Forgetting `:app` or `:version` in `project/0`, which prevents `.app` file generation.
- Defining `project/0` with expensive computation or side effects.
- Using `Mix.Project.config/0` to read runtime application configuration.
- Defining custom Mix tasks without the `Mix.Tasks.` namespace prefix.
- Omitting `@shortdoc` and expecting the task to appear in `mix help`.
- Naming custom task files with underscores instead of dot separators — prefer `deps.clean.ex` over `deps_clean.ex`.
- Forgetting to set `mod: {MyApp.Application, []}` when using `--sup` or a supervision tree.
- Hard-coding umbrella paths incorrectly in child `mix.exs` files.
- Using reserved OTP/Elixir application names such as `kernel`, `stdlib`, `logger`, or `mix`.

## Strict vs contextual guidance

### Strict

- A Mix project MUST have a `mix.exs` file that calls `use Mix.Project` and exports `project/0`.
- `:app` and `:version` keys in `project/0` are REQUIRED for Mix to generate the `.app` file.
- OTP application names MUST match `^[a-z][a-z0-9_]*$` (lowercase letter start, then lowercase letters, digits, or underscores).
- Module names MUST match `^[A-Z]\w*(\.[A-Z]\w*)*$` (valid Elixir alias).
- Module names MUST start with an uppercase letter; function names MUST start with a lowercase letter or underscore.
- `Mix.env/0`, `Mix.target/0`, and other Mix APIs MUST NOT be used in runtime `lib/` code — they are build-time tooling and may be unavailable in releases.
- Custom Mix task module names MUST begin with `Mix.Tasks.` and implement the `run/1` callback.
- The `.app` file is generated automatically by Mix from `mix.exs`; do not hand-edit it.

### Conventions (not enforced by the compiler or Mix)

- Keep `project/0` fast; defer expensive configuration to anonymous functions or task-specific callbacks.
- Put `deps/0` in a private helper function to keep `project/0` tidy.
- Place custom tasks in `lib/mix/tasks/` and use dot separators in file names.
- Use `snake_case` for OTP app atoms and file paths; use `CamelCase` for module names.
- Mirror dotted module names in directory paths (`MyApp.Foo.Bar` -> `lib/my_app/foo/bar.ex`).
- Use `_test.exs` suffixes and mirror source paths under `test/`.
- Declare `:extra_applications` such as `:logger` and `:crypto` in `application/0`.
- Use `config/runtime.exs` for runtime configuration; use `config/config.exs` for build-time configuration.
- Add `config/dev.exs` and `config/test.exs` imported from `config/config.exs` for environment-specific build-time config.
- Use `@shortdoc` to make tasks visible in `mix help`, and `@moduledoc false` to hide them.
- Use `@recursive true` for tasks that should run in each umbrella child.
- Use `@preferred_cli_env` to hint the right environment for a custom task.

### Contextual tradeoffs

- Umbrella projects share config/deps/lock but are not fully decoupled — use them for organizational convenience, not strict isolation.
- `Mix.compilers()` returns the default compilers; prepend or append custom compilers only when needed.
- `Mix.env/1` and `Mix.target/1` can change the active environment/target but do not reload project configuration — use sparingly.
- Targets are useful for cross-compilation scenarios; most host-only projects can ignore `Mix.target/0`.
- The application environment is discouraged for libraries; application code may use it, but libraries should accept configuration via function arguments.

## Policy decisions for individual repos

- Whether to require `--sup` for all new Mix applications or allow plain libraries.
- Whether to permit umbrella projects and how to structure shared config/deps.
- Whether to enforce module-to-file path mirroring in CI or via `mix credo`.
- Which lint stack to run in CI: `mix format --check-formatted`, `mix credo --strict`, `mix dialyzer`, and/or `mix test`.
- Whether to require a non-bang variant for every `!` function in Elixir code (see `docs/elixir/naming-conventions.md`).
- Whether all applications must include `config/runtime.exs` even when no runtime secrets are needed.
- Whether custom tasks must declare `@preferred_cli_env` and `@recursive` explicitly.
- Whether to allow `Mix.env/0` in test helper files or config only.
- How to manage environment-specific dependencies and whether `:only` constraints are required for dev/test-only deps.

## Related docs

- `docs/elixir/naming-conventions.md` — cross-reference for module, function, atom, and file naming.
- Related Elixir corpus docs: `docs/elixir/language-fundamentals.md`, `docs/elixir/typespecs-and-dialyzer.md`, `docs/elixir/error-handling.md`.
- See the "Formatting" section of this doc (`mix format`, `.formatter.exs`, `mix format --check-formatted`) for formatting conventions and CI tone.

## Related BEAM guidance

- `../beam/releases.md` — BEAM release assembly, boot scripts, and hot code upgrade semantics that underpin `mix release`.
- `../beam/applications.md` — OTP application resource (`.app`) generation and application environment that Mix emits from `mix.exs`.

## Related skills

- None defined yet.
