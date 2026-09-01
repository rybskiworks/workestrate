# Recipe aggregator: exports all recipe functions.
{ pkgs }:
let
  vocab = import ./vocabulary.nix { inherit pkgs; };
in
{
  build = {
    npm-build = import ./recipes/npm-build.nix {
      inherit pkgs;
      inherit (pkgs)
        buildNpmPackage
        nodejs_24
        autoPatchelfHook
        stdenv
        libcap_ng
        lib
        ;
    };
    bun-compile = import ./recipes/bun-compile.nix {
      inherit pkgs;
      inherit (pkgs) bun;
      inherit (pkgs) stdenv;
      inherit (pkgs) lib;
      inherit (pkgs) removeReferencesTo;
    };
    pip-install = import ./recipes/pip-install.nix {
      inherit pkgs;
      inherit (pkgs) python312;
      inherit (pkgs) stdenv;
    };
    bun-install = import ./recipes/bun-install.nix {
      inherit pkgs;
      inherit (pkgs) bun;
      inherit (pkgs) nodejs_24;
      inherit (pkgs) stdenv;
    };
  };
  image = {
    registry = import ./recipes/registry.nix;
    nix-layered = import ./recipes/nix-layered.nix { inherit pkgs vocab; };
  };
  inherit vocab;
}
