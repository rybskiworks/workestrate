# Recipe aggregator: exports all recipe functions.
{ pkgs }:
let
  vocab = import ./vocabulary.nix { inherit pkgs; };
in {
  build = {
    npm-build = import ./recipes/npm-build.nix {
      inherit pkgs;
      inherit (pkgs) buildNpmPackage nodejs_24 autoPatchelfHook stdenv libcap_ng lib;
    };
    bun-compile = import ./recipes/bun-compile.nix {
      inherit pkgs;
      bun = pkgs.bun;
      stdenv = pkgs.stdenv;
      lib = pkgs.lib;
      removeReferencesTo = pkgs.removeReferencesTo;
    };
    pip-install = import ./recipes/pip-install.nix {
      inherit pkgs;
      python312 = pkgs.python312;
      stdenv = pkgs.stdenv;
    };
    bun-install = import ./recipes/bun-install.nix {
      inherit pkgs;
      bun = pkgs.bun;
      nodejs_24 = pkgs.nodejs_24;
      stdenv = pkgs.stdenv;
    };
  };
  image = {
    registry = import ./recipes/registry.nix;
    nix-layered = import ./recipes/nix-layered.nix { inherit pkgs vocab; };
  };
  inherit vocab;
}
