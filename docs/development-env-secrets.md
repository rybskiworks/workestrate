# Environment-only development secrets

An inner Workestrate development process can use credentials already present
in its environment without an age key or decrypting the fleet's SOPS files:

```sh
workestrate --secrets-source env workload plan example
workestrate --secrets-source env workload exec example
workestrate --secrets-source env workload up database
```

This is an explicit development opt-in. Omit it for normal `layered` loading:
process values followed by configured encrypted layers. For workload and
`run` secret input, `env` returns before
encrypted-layer resolution and never invokes SOPS, reads an encrypted file,
or looks up an age key. It does not change home, context, project trust or
discovery, configuration layering, image selection, or sandbox configuration.
Explicit credential provisioning, such as `credentials signing generate`,
still operates on its selected SOPS file; this option does not turn that
operation into an environment write or disable its encryption.

## Declared sources and aliases

```toml
[secrets.MACHINE]
env_var = "MACHINE_GITHUB_TOKEN"
required = true
allowed_hosts = ["github.com", "api.github.com"]

[workloads.example.env]
GITHUB_TOKEN = { secret = "MACHINE", bound = "host" }
GH_TOKEN = { secret = "MACHINE", bound = "host" }
```

Lookup first uses the canonical `MACHINE_GITHUB_TOKEN` (`env_var`, or the
secret ID when omitted). Only if that name is absent does it try the selected
workload's declared aliases, here `GITHUB_TOKEN` and `GH_TOKEN`. Literal env
entries and arbitrary inherited variables are not secret inputs. Results use
canonical names; aliases do not rewrite workload plans or credential grants.

- A present canonical value wins even if aliases differ. A present but blank
  canonical value does not fall back to an alias.
- Missing or whitespace-only required values fail; optional values are omitted.
  Nonblank values are preserved byte-for-byte, without trimming their contents.
- Multiple nonblank fallback aliases must be identical. A fallback alias owned
  by multiple selected sources, or also used as another selected canonical
  source, is ambiguous and fails. Set canonical sources explicitly.
- Configured example placeholders are rejected. Errors name bindings, not values.

Named `up`/`exec` resolves only that workload's secret env bindings and declared
SSH/signing credential materials. Dependencies resolve their own bindings when
starting or replacing, not when reusing a healthy service. Unrelated workload
credentials are not required. Bare `workload up` checks the selected service
starts before their image/start pass (and before any individual replacement
teardown). Generic `run -- COMMAND`
resolves all configured secret definitions and their aliases across workloads;
use canonical names when those aliases conflict. Like normal `run`, the child
still inherits the caller's existing environment: this is not an env scrubber.

Required-input preflight precedes a normal named launch's image/instance work;
fresh dependency preflight precedes allocation, and other dependency preflight
precedes the reconciled start/replacement action. Existing inline
workload-ref ordering remains: home-scoped dependencies start before arming
and checking the dependent's alternate configuration. A failed launch is not
a transactional rollback of dependency work. `plan` remains metadata-only:
it does not resolve values or prove credential availability.

## Inheritance and explicit overrides

The selected mode is carried by the nonsecret
`WORKESTRATE_SECRETS_SOURCE=env|layered` setting. Dependency and detached host
children, and Workestrate invoked inside `run`, inherit it. An explicit CLI
option always wins, including over an invalid inherited value:

```sh
workestrate --secrets-source env run -- workestrate workload plan example
workestrate --secrets-source env run -- workestrate --secrets-source layered run -- COMMAND
```

Explicitly setting `WORKESTRATE_SECRETS_SOURCE=env` has the same opt-in effect.
Unset means `layered`; blank or unknown values fail. Workload guest env remains
controlled by the workload configuration, not automatically expanded to include
this host setting or all host credentials. An inner CLI needs its own explicit
selection unless the fleet intentionally supplies that setting.

## Authority and nested-runtime limits

This changes credential input, not authority. Host/guest delivery, allowed
hosts, violation handling, SSH/signing grants, broker custody and normal key
format validation remain in force. It neither mounts an age key nor forwards
an operator SSH agent nor creates a delegation grant.

An opaque nonblank MSB credential placeholder can be selected unchanged. That
does not prove that a second runtime can substitute it, that a nested network
path is authorized, or that it represents a usable SSH private key. End-to-end
parent-to-child delegation needs separate real-runtime acceptance. Do not
expose real broker-only key material in a guest to satisfy environment lookup.

## Verification

Resolver unit tests and `env_secrets` CLI integration tests use synthetic values
and isolated homes. They cover precedence, ambiguity, required/optional inputs,
opaque placeholders, credential references, inherited/overridden modes,
unchanged plans and SOPS/image/runtime spies. They start no VM or provider.

Run through the [pinned Rust validation environment](nix-build.md):

```sh
cargo test --locked --lib microsandbox::secrets_loader::environment::tests
cargo test --locked --test env_secrets
```
