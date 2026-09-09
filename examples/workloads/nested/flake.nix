{
  description = "Isolated Workestrate-in-Workestrate workload smoke";

  inputs = {
    tooling.url = "github:rybskiworks/nix-tooling/34c287290245c20e9103f7cd0fcdaa244edd310b";
    nixpkgs.follows = "tooling/nixpkgs";
    flake-parts.follows = "tooling/flake-parts";
    # Preserve the known runtime closure independently of the image tooling.
    workestrate.url = "git+ssh://git@github.com/rybskiworks/workestrate.git?rev=fed92957ccf6a77990efd7abc6c7bf2b8b711f5c";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      perSystem =
        { pkgs, system, ... }:
        let
          workestrate = inputs.workestrate.packages.${system}.default;
          image = import ./image.nix { inherit pkgs workestrate; };
        in
        {
          packages = {
            default = image;
            workestrate-nested-smoke = image;
            inherit workestrate;
          };
          checks.image =
            pkgs.runCommand "workestrate-nested-image-check" { nativeBuildInputs = [ pkgs.python3 ]; }
              ''
                python ${./check-image.py} ${image}
                touch "$out"
              '';
          formatter = pkgs.nixfmt;
        };
    };
}
