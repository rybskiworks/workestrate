{ packages }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.workestrate;
in
{
  options.programs.workestrate = {
    enable = lib.mkEnableOption "the Workestrate CLI and fleet source tools";
    package = lib.mkOption {
      type = lib.types.package;
      default = packages.${pkgs.stdenv.hostPlatform.system}.workestrate;
      defaultText = lib.literalExpression "workestrate.packages.\${system}.workestrate";
      description = ''
        Workestrate package to install. Pass the same derivation to physical
        hosts and guests to retain its exact CLI and Microsandbox runtime.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = pkgs.stdenv.hostPlatform.system == "x86_64-linux";
        message = "Workestrate currently supports x86_64-linux systems.";
      }
    ];
    environment.systemPackages = [
      cfg.package
      pkgs.git
      pkgs.gnutar
    ]
    ++ lib.optional (cfg.package ? stateInit) cfg.package.stateInit;
  };
}
