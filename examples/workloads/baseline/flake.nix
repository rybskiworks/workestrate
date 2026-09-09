{
  description = "Minimal isolated Workestrate workload image";

  inputs = {
    tooling.url = "github:rybskiworks/nix-tooling/34c287290245c20e9103f7cd0fcdaa244edd310b";
    nixpkgs.follows = "tooling/nixpkgs";
    flake-parts.follows = "tooling/flake-parts";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      perSystem =
        { pkgs, ... }:
        let
          image = import ./image.nix { inherit pkgs; };
        in
        {
          packages = {
            default = image;
            workestrate-smoke = image;
          };
          checks.image =
            pkgs.runCommand "workestrate-smoke-image-check" { nativeBuildInputs = [ pkgs.python3 ]; }
              ''
                python ${./check-image.py} ${image} workestrate-smoke:latest
                touch "$out"
              '';
          formatter = pkgs.nixfmt;
        };
    };
}
