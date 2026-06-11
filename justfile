check:
    cargo fmt --manifest-path control/agentctl/Cargo.toml -- --check
    cargo clippy --manifest-path control/agentctl/Cargo.toml -- -D warnings
    cargo check --manifest-path control/agentctl/Cargo.toml

fmt:
    cargo fmt --manifest-path control/agentctl/Cargo.toml

agentctl *args:
    cargo run --manifest-path control/agentctl/Cargo.toml -- {{args}}

plan:
    cargo run --manifest-path control/agentctl/Cargo.toml -- litellm plan
    cargo run --manifest-path control/agentctl/Cargo.toml -- agent plan pi
    cargo run --manifest-path control/agentctl/Cargo.toml -- agent plan odysseus
