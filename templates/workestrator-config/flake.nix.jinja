# Inverted-dependency flake: takes workestrator core as input.
# This flake builds nix-layered images from this config repo's workestrate.toml.
# Phase 2 feature — optional.
{
  inputs = {
    workestrator.url = "{{ core_flake_url }}";
    nixpkgs.follows = "workestrator/nixpkgs";
  };

  outputs = { self, workestrator, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      config = builtins.fromTOML (builtins.readFile ./workestrate.toml);
    in {
      # Build this repo's nix-layered images using core's exported recipes
      packages.${system} = workestrator.lib.${system}.buildImagesFromConfig {
        inherit pkgs config;
      };

      # Config-repo CI: validate against core's schema
      checks.${system} = workestrator.lib.${system}.checks.validateConfig {
        inherit pkgs;
        workestrate = workestrator.packages.${system}.workestrate;
        config = builtins.readFile ./workestrate.toml;
      };
    };
}
