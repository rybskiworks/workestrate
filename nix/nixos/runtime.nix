{
  config,
  lib,
  ...
}:
let
  cfg = config.programs.workestrate.runtime;
in
{
  options.programs.workestrate.runtime = {
    enable = lib.mkEnableOption "local Workestrate microVM execution";
    users = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "alice" ];
      description = ''
        Existing local accounts allowed to use /dev/kvm. This adds only the
        kvm supplementary group; it does not grant Nix daemon trust or access
        to another account's Workestrate configuration and state.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    programs.workestrate.enable = true;
    users.groups.kvm = { };
    users.users = lib.genAttrs cfg.users (_name: {
      extraGroups = [ "kvm" ];
    });
    services.udev.extraRules = ''
      KERNEL=="kvm", GROUP="kvm", MODE="0660"
    '';
    # The hardware configuration owns the Intel/AMD driver and nested policy.
    # A guest receives its kernel and device support from its outer runtime.
    boot.kernelModules = lib.mkIf (!config.boot.isContainer) [ "kvm" ];
  };
}
