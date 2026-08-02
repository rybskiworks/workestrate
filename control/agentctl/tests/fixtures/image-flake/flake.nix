# Fixture image flake for the spec-21 phase-D pipeline e2e
# (`tests/image_pipeline_e2e.rs`).
#
# A `dockerTools.buildLayeredImage` whose ONLY content is a static text file:
# no network fixed-output derivations, a ~20 KiB tarball — safe to build
# in-container (disk-frugal) and on the host. The nixpkgs input pins the SAME
# rev as this repo's `flake.lock`, so eval + build need no network once that
# rev is realized in the store (it is, in the dev container: the devshell
# closure already pulled it).
#
# The e2e copies this file to a temp dir before building: path flakes inside
# a git worktree eval through git+file semantics (tracked files only,
# whole-tree copies); a plain temp dir keeps the e2e hermetic.
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/a799d3e3886da994fa307f817a6bc705ae538eeb";
  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      packages.${system}.wk-fixture-image = pkgs.dockerTools.buildLayeredImage {
        name = "wk-fixture-image";
        tag = "latest";
        contents = [
          (pkgs.writeTextDir "/hello.txt" "hello from the fixture image\n")
        ];
      };
    };
}
