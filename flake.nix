{
  description = "ai-workbench — local AI workbench for Pi/Odysseus through Microsandbox + LiteLLM";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    pi = {
      url = "github:georgrybski/pi";
      flake = false;
    };

    odysseus = {
      url = "github:georgrybski/odysseus";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, pi, odysseus, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.${system}.default = import ./nix/devshells/default.nix {
        inherit pkgs;
      };

      packages.${system}.agentctl = pkgs.callPackage ./nix/packages/agentctl.nix { inherit pi odysseus; };
    };
}
