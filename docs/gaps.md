# Known Gaps

## Blocked by Environment

- Microsandbox runtime testing is blocked: this container lacks /dev/kvm and Docker.
- Host with KVM/nested virtualization is required for actual sandbox execution.

## Unverified Agent Integration

- Pi does not honor `OPENAI_BASE_URL`. The `models.json` seeding approach is documented but untested.
- Odysseus does not honor `OPENAI_BASE_URL`. The `data/settings.json` approach is documented but untested.
- Exact Pi RPC/headless handshake is unknown.
- Exact Odysseus entrypoint customization for LiteLLM proxy mode is unknown.

## Unverified Microsandbox Behavior

- Egress enforcement has not been tested at runtime.
- Secret injection (`secret_env`) has not been tested at runtime.
- Network policy behavior (`default_deny` + explicit allow) is documented from SDK source but untested.

## Build and Packaging

- `nix build .#agentctl` evaluates but the full build is best-effort for milestone 1.
- `Cargo.lock` is tracked; regenerating it after dependency changes is manual.

## Documentation

- Profile docs are based on upstream README/source inspection, not direct runtime verification.
- `agents/pi/` is not present locally; Pi profile is based on upstream `georgrybski/pi` docs.
