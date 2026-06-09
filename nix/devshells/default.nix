# Dev shell for the AI workbench.
#
# Provides the full toolchain called out in the plan:
#   rustc, cargo, rust-analyzer, rustfmt, clippy
#   pkg-config, openssl, git, just, jq, yq, curl, ripgrep, fd, cosign
#   postgresql_16
#
# Not currently usable in this environment because the Nix CLI is not
# installed; see README.md and the deviations list.
{ pkgs }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc
    cargo
    rust-analyzer
    rustfmt
    clippy
    pkg-config
    openssl
    git
    just
    jq
    yq
    curl
    ripgrep
    fd
    cosign
    postgresql_16
  ];

  shellHook = ''
    echo "ai-workbench dev shell"
    echo "  rustc   : $(rustc --version 2>/dev/null || echo unavailable)"
    echo "  cargo   : $(cargo --version 2>/dev/null || echo unavailable)"
    echo "  just    : $(just --version 2>/dev/null || echo unavailable)"
  '';
}
