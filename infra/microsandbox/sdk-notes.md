# Microsandbox SDK Notes

## Crate
- Name: `microsandbox`
- Version: `0.5.6` (pinned)
- Edition: 2024
- License: Apache-2.0
- Source: `github:superradcompany/microsandbox`

## API Surface
- Async-only (requires Tokio runtime)
- Features used: `net` (default-on, explicitly listed)
- Key types: `Sandbox`, `SandboxBuilder`, `NetworkPolicy`, `NetworkPolicyBuilder`
- Builder methods verified: `.image()`, `.cpus()`, `.memory()`, `.port()`, `.env()`, `.secret_env()`, `.volume()`, `.network()`, `.replace()`

## Compile Status
- `cargo check` passes
- `cargo check --examples` passes
- Zero warnings

## Runtime Status
- **NOT TESTED** — host lacks /dev/kvm
- KVM or Apple Silicon required for Microsandbox microVMs

## SDK Support Matrix
| Feature | Status |
|---------|--------|
| Env vars | ✓ compile-checked |
| Mounts | ✓ compile-checked |
| Ports | ✓ compile-checked |
| Command execution | ✓ compile-checked (async) |
| Network/egress | ✓ compile-checked |
| Secrets | ✓ compile-checked |

## Sources
- GitHub: `https://github.com/superradcompany/microsandbox`
- Example: `examples/rust/net-policy/bin/main.rs`
