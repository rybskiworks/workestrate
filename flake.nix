{
  description = "AI workestrator";

  # The Nix flake is the canonical build entry point for agentctl and
  # the canonical dev shell. To use it:
  #
  #   nix develop            # enter the dev shell (rustc, cargo, jq, etc.)
  #   nix build .#agentctl   # build the agentctl package
  #
  # A project-local toolchain at .toolchain/ (rustup + zig cc) is kept
  # as a fallback for environments without Nix, but the flake is the
  # primary path. See README.md and nix/devshells/default.nix for the
  # dev shell contents.
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = inputs:
    inputs.flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import inputs.nixpkgs { inherit system; };
        lib = pkgs.lib;
        agentctl = import ./nix/packages/agentctl.nix {
          inherit pkgs lib;
          crane = inputs.crane;
        };
        devShell = import ./nix/devshells/default.nix {
          inherit pkgs;
        };
      in {
        packages.default = agentctl;
        packages.agentctl = agentctl;
        apps.default = { type = "app"; program = "${agentctl}/bin/agentctl"; };
        apps.agentctl = { type = "app"; program = "${agentctl}/bin/agentctl"; };
        devShells.default = devShell;
        formatter = pkgs.nixpkgs-fmt;
      });
}
