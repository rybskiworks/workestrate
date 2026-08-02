# Config File Management (`include` / split-config)

## What it controls

Splitting a single `config.yaml` across multiple YAML files via the top-level
`include` directive, so model definitions, routing, and proxy settings can live
in separately-reviewed files.

## Where it appears

Top-level `include:` key in `config.yaml`. Documented on the upstream
"File Management" page.

## Verified behavior (verbatim)

> "You can use `include` to include external YAML files in a config.yaml."
> — `/docs/proxy/config_management`

The `include` key takes **a single file or a list of files**. Each listed file is
merged into the parent config at load time. The proxy is started by pointing
`--config` at the **parent** file; the `include` directive is then resolved by
the loader.

### Verbatim syntax — include a single file

```yaml
include:
  - model_config.yaml
```

### Verbatim syntax — include multiple files

```yaml
include:
  - model_config.yaml
  - another_config.yaml
```

### Verbatim end-to-end example

Parent config (`parent_config.yaml`):

```yaml
include:
  - model_config.yaml # 👈 Key change, will include the contents of model_config.yaml

litellm_settings:
  callbacks: ["prometheus"] 
```

Included child config (`model_config.yaml`) — note the child contains **only** a
top-level `model_list:`, with no wrapper:

```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: openai/gpt-4o
      api_base: https://exampleopenaiendpoint-production.up.railway.app/
  - model_name: fake-anthropic-endpoint
    litellm_params:
      model: anthropic/fake
      api_base: https://exampleanthropicendpoint-production.up.railway.app/
```

Start command (verbatim):

```
litellm --config parent_config.yaml --detailed_debug
```

## Child file shape

A child file may contain **only a top-level section** (e.g. `model_list:`) with no
wrapper — confirmed by the verbatim upstream example where `model_config.yaml`
contains exactly a top-level `model_list:`. This lets you split by area: one
child for `model_list`, one for `router_settings`, one for `general_settings`.

## Hot-reload / config-reload behavior

**NOT documented on the `config_management` page.** The page does not state
whether the proxy picks up changes to `include`d files without a restart.
Cross-reference: the `config_settings` page documents **startup-time Pydantic
validation** — config is loaded and validated at startup, and on invalid config
the proxy **fails to start**. This strongly implies config (including `include`d
files) is resolved at load time, not hot-reloaded. Treat hot-reload as
**not available** unless a separate upstream page documents otherwise.

## Config file rotation / management operations

**NOT documented on the `config_management` page.** No verbatim mention of
config rotation, file-swapping, or management operations beyond the `include`
merge directive.

## CLI flags for config management

The only CLI command on the page is the standard proxy start command:

```
litellm --config parent_config.yaml --detailed_debug
```

`--config` selects the parent file (the `include` directive is resolved by the
loader from there); `--detailed_debug` is a verbosity flag. There is **no
config-management-specific CLI flag** documented on this page.

## DB / Redis / Enterprise requirements

**NOT documented on the `config_management` page.** The `include` directive
itself has no stated DB, Redis, or Enterprise requirement.

## Pitfalls

- **No hot-reload documented.** Do not assume `include`d file changes are picked
  up without a restart. The proxy loads and validates config at startup.
- **Child file shape.** A child file contains top-level sections directly (e.g.
  `model_list:`), not a nested wrapper. The verbatim example confirms this.
- **`include` is a top-level key**, not a CLI flag. It is resolved by pointing
  `--config` at the parent file that contains the `include:` directive.
- **Page is short.** The upstream page is ~30 lines and covers only `include`.
  Hot-reload, rotation, and management operations are simply not on it.

## Links to local

- [config-yaml-overview.md](config-yaml-overview.md) — top-level structure (includes `include`)
- [../examples/split-config-parent.yaml](../examples/split-config-parent.yaml) — worked example (parent)
- [../examples/split-config-models.yaml](../examples/split-config-models.yaml) — worked example (child)
- [../extracted/p0-config_management.md](../extracted/p0-config_management.md) — verbatim extraction
- [../schemas/config-yaml.option-index.json](../schemas/config-yaml.option-index.json) — `include` registered under `top_level`

## Sources

- https://docs.litellm.ai/docs/proxy/config_management (PRIMARY — "File Management")
- Raw: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/proxy/config_management.md
- Local: `extracted/p0-config_management.md`

## Workestrate notes

[PROJECT CONTEXT — NOT upstream docs]

The real config at `infra/litellm/config.yaml` is **split**: the parent holds
`general_settings` / `router_settings` / `litellm_settings` and `include:`s
`models.yaml`, which holds `model_list` (with YAML anchors for dedup). This is
the split pattern described below, now adopted.

If adopted, a split config would separate `model_list` (per-provider child files)
from `general_settings` / `router_settings` / `litellm_settings` (parent).

**Critical microsandbox constraint:** the `infra/litellm/` directory is mounted
**read-only** at `/app/config` and the proxy is started with
`--config /app/config/config.yaml --host 0.0.0.0`. Because the mount is a
read-only directory, the `include:`d `models.yaml` resolves too. Because it is
read-only:

- The `include:`d child (`models.yaml`) is mounted read-only via the same
  directory mount as the parent.
- **Hot-reload is blocked regardless** — a read-only mount cannot be rewritten,
  so config changes require a sandbox restart. (Hot-reload is also not documented
  upstream on the `config_management` page in any case.)
- Config changes therefore flow through the Nix flake / image rebuild → sandbox
  restart, not through live file edits.
