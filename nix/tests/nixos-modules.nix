{
  pkgs,
  nixpkgs,
  tooling,
  modules,
  workestrate,
  microsandbox,
  guest,
  sharedGuest,
  sharedClosure,
}:
let
  inherit (pkgs) lib;
  hostSettings = {
    nixpkgs.pkgs = pkgs;
    system.stateVersion = "26.05";
    fileSystems."/" = {
      device = "/dev/disk/by-label/workestrate-test";
      fsType = "ext4";
    };
    boot.loader.grub.enable = false;
    networking.hostName = "workestrate-test";
    users.users.alice = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
    };
  };
  system =
    extraModules:
    nixpkgs.lib.nixosSystem {
      modules = [
        hostSettings
        tooling.nixosModules.lixSystem
      ]
      ++ extraModules;
    };
  baseline = (system [ ]).config;
  disabled = (system [ modules.workestrateHost ]).config;
  host =
    (system [
      modules.workestrateHost
      {
        programs.workestrate.runtime = {
          enable = true;
          users = [ "alice" ];
        };
      }
    ]).config;
  common =
    (system [
      modules.workestrate
      { programs.workestrate.enable = true; }
    ]).config;
  replacement =
    (system [
      modules.workestrate
      {
        programs.workestrate = {
          enable = true;
          package = pkgs.hello;
        };
      }
    ]).config;
  guestConfig = guest.guestSystem.config;
  hasPackage =
    config: package:
    builtins.any (entry: entry.outPath == package.outPath) config.environment.systemPackages;
  assertions = {
    disabledPackages = disabled.environment.systemPackages == baseline.environment.systemPackages;
    disabledGroups = disabled.users.users.alice.extraGroups == baseline.users.users.alice.extraGroups;
    disabledKernel = disabled.boot.kernelModules == baseline.boot.kernelModules;
    disabledUdev = disabled.services.udev.extraRules == baseline.services.udev.extraRules;
    commonPackage = hasPackage common workestrate;
    commonStateTool = hasPackage common workestrate.stateInit;
    guestStateTool = hasPackage guestConfig workestrate.stateInit;
    commonSourceTools = hasPackage common pkgs.git && hasPackage common pkgs.gnutar;
    commonKernel = common.boot.kernelModules == baseline.boot.kernelModules;
    commonGroups = common.users.users.alice.extraGroups == baseline.users.users.alice.extraGroups;
    commonUdev = common.services.udev.extraRules == baseline.services.udev.extraRules;
    explicitPackage =
      replacement.programs.workestrate.package.drvPath == pkgs.hello.drvPath
      && hasPackage replacement pkgs.hello
      && !(hasPackage replacement workestrate);
    hostEnabled = host.programs.workestrate.enable && hasPackage host workestrate;
    hostNormalKernel = !host.boot.isContainer && host.boot.kernel.enable;
    hostNetworkPreserved =
      host.networking.hostName == baseline.networking.hostName
      && host.networking.firewall.enable == baseline.networking.firewall.enable
      && host.networking.useDHCP == baseline.networking.useDHCP;
    hostKvm =
      builtins.elem "kvm" host.boot.kernelModules
      && builtins.elem "kvm" host.users.users.alice.extraGroups
      && builtins.elem "wheel" host.users.users.alice.extraGroups;
    noNixTrustGrant = host.nix.settings.trusted-users == baseline.nix.settings.trusted-users;
    noStateRedirection =
      !(host.environment.variables ? MSB_HOME)
      && !(host.environment.variables ? WORKESTRATE_CONFIG)
      && !(host.environment.sessionVariables ? MSB_HOME)
      && !(host.environment.sessionVariables ? WORKESTRATE_CONFIG);
    noRuntimeService =
      builtins.attrNames host.systemd.services == builtins.attrNames baseline.systemd.services;
    guestNixos =
      guestConfig.boot.isContainer
      && !guestConfig.boot.kernel.enable
      && guest.guestInit.executable == "/init"
      && guest.guestInit.env.container == "microsandbox";
    guestPackage = hasPackage guestConfig workestrate;
    samePackageDerivation =
      host.programs.workestrate.package.drvPath == guestConfig.programs.workestrate.package.drvPath;
    samePackageOutput =
      host.programs.workestrate.package.outPath == guestConfig.programs.workestrate.package.outPath;
    sameRuntime =
      host.programs.workestrate.package.microsandbox.drvPath == microsandbox.drvPath
      && guestConfig.programs.workestrate.package.microsandbox.outPath == microsandbox.outPath;
    sameLix =
      host.nix.package.drvPath == guestConfig.nix.package.drvPath
      && guestConfig.nix.package.pname == "lix";
    guestRegistration = builtins.elem "guest-store-registration.service" guestConfig.systemd.services.nix-daemon.requires;
    ordinaryImageSelfContained = guest.guestExternalClosures == [ ];
    sharedImageExport = map toString sharedGuest.guestExternalClosures == [ (toString sharedClosure) ];
    sharedImageSameSystem = sharedGuest.guestToplevel.outPath == guest.guestToplevel.outPath;
    sharedImageSamePackage =
      sharedGuest.guestSystem.config.programs.workestrate.package.outPath == workestrate.outPath;
    sharedClosureExactRoot =
      map toString sharedClosure.guestClosureExport.roots == [ workestrate.outPath ];
    sharedClosureSchema = sharedClosure.guestClosureExport.schemaVersion == 1;
    moduleAssertions = builtins.all (entry: entry.assertion) (
      host.assertions ++ guestConfig.assertions
    );
  };
in
assert lib.assertMsg (builtins.all (value: value) (builtins.attrValues assertions)) (
  "Workestrate NixOS module contract failed: "
  + lib.concatStringsSep ", " (builtins.attrNames (lib.filterAttrs (_name: value: !value) assertions))
);
pkgs.writeText "workestrate-nixos-module-contract.json" (builtins.toJSON assertions)
