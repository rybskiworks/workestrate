# Inverted-dependency flake: takes workestrate core as input.
# This flake builds nix-layered images from this config repo's workestrate.toml.
# Phase 2 feature — optional.
{
  inputs = {
    workestrate.url = "{{ core_flake_url }}";
    nixpkgs.follows = "workestrate/nixpkgs";
  };

  outputs = { self, workestrate, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      config = builtins.fromTOML (builtins.readFile ./workestrate.toml);
      workestrate-cli = workestrate.packages.${system}.workestrate;
    in {
      # Build this repo's nix-layered images using core's exported recipes
      packages.${system} = workestrate.lib.${system}.buildImagesFromConfig {
        inherit pkgs config;
      };

      # Config-repo CI: validate against core's schema
      checks.${system} = workestrate.lib.${system}.checks.validateConfig {
        inherit pkgs;
        # The arg is `workestrate` (the agentctl CLI package); bound via a
        # let-alias to avoid shadowing the `workestrate` flake input.
        workestrate = workestrate-cli;
        config = builtins.readFile ./workestrate.toml;
      };
    };
}
