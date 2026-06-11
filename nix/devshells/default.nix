{ pkgs }:

pkgs.mkShell {
  packages = with pkgs; [
    cargo
    clippy
    git
    just
    libcap_ng
    openssl
    pkg-config
    rustc
    rustfmt
  ];

  shellHook = ''
    echo "ai-workbench dev shell"
  '';
}
