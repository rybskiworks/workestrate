# Known Gaps

## Blocked by Environment

- Microsandbox runtime testing is blocked: this container lacks /dev/kvm and Docker.
- Host with KVM/nested virtualization is required for actual sandbox execution.

## Unverified Agent Integration

- Pi does not honor `OPENAI_BASE_URL`. The `models.json` seeding approach is documented but untested.
- Odysseus does not honor `OPENAI_BASE_URL`. The `data/settings.json` approach is documented but untested.
- Exact Pi RPC/headless handshake is unknown.
- Exact Odysseus entrypoint customization for LiteLLM proxy mode is unknown.
- Odysseus companion services (chromadb/searxng/ntfy) are not provisioned — the full analysis (docs/odysseus-full-capability.md) moved to the user's personal fleet, as it is personal-workload content rather than generic tooling.

## Unverified Microsandbox Behavior

- Egress enforcement has not been tested at runtime.
- Secret injection (`secret_env`) has not been tested at runtime.
  - `secret_env.allowed_host` must match the full SDK host alias (`host.microsandbox.internal`) because it is compared exactly against the HTTP `Host` header. The bare `host` value used in `egress_rules.allow_hosts` is not sufficient here.
- Network policy behavior (`default_deny` + explicit allow) is documented from SDK source but untested.
- The `up_*` SDK calls were updated to mirror `plan_*` (mounts, secrets, workdir, entrypoint, network policy), but this change is compile-checked only and has not been validated at runtime.

## Build and Packaging

- `nix build .#workestrate` builds the CLI in M1 and wraps the resulting binary with
  `MSB_PATH` pointing at the Nix-managed `msb` from `.#microsandbox`. The SDK's
  `build.rs` is satisfied at build time by staging `msb` and `libkrunfw.so.5.2.1` in
  `$MSB_HOME` (no network download), and at run time by `MSB_PATH`. So
  `nix run .#workestrate -- workload plan litellm` (or `nix run . -- workload plan litellm`) works
  without `nix develop` and without a pre-existing `~/.microsandbox/`.
- `microsandbox-filesystem`'s build.rs would normally download `agentd-x86_64` at
  compile time. We patch it to honor `$MSB_HOME/bin/agentd` and stage the binary
  from `nix/packages/microsandbox.nix` so `nix build .#workestrate` works without
  network access in the Nix sandbox.
- `Cargo.lock` is tracked; regenerating it after dependency changes is manual.

## CLI surface
- `up`/`down` subcommands exist but are only compile-checked; they require KVM to test.

## Documentation

- Profile docs are based on upstream README/source inspection, not direct runtime verification.
- `agents/pi/repo/` is not present locally; Pi profile is based on upstream `georgrybski/pi` docs.

## Secrets Management

- Secrets are managed via SOPS + age. Encrypted `.env.enc` is committed; the private key lives at `~/.config/sops/age/ai-workbench-secrets.txt` and is never committed. The previous "set env vars manually" approach is replaced by `workestrate` (which decrypts `.env.enc` internally), `decrypt-env`, and `write-env` (see `docs/secrets.md`). Manual plaintext `.env` files are no longer the supported path.
