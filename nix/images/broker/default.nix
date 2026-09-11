{
  pkgs,
  guest,
  base,
  brokerd,
  hostPrincipal,
}:
let
  inherit (pkgs) lib;
  validLabel =
    label:
    builtins.stringLength label <= 63 && builtins.match "[a-z0-9]([a-z0-9-]*[a-z0-9])?" label != null;
  validPrincipal =
    builtins.isString hostPrincipal
    && builtins.stringLength hostPrincipal <= 253
    && builtins.all validLabel (lib.splitString "." hostPrincipal);
  unitName = "workestrate-broker.service";
  unitDirectory = "usr/local/lib/systemd/system";
  # Read only the template bytes, not a path retaining the repository source.
  unitText =
    builtins.replaceStrings [ "@brokerd@" "@hostPrincipal@" ] [ "${brokerd}/bin/brokerd" hostPrincipal ]
      (builtins.readFile ./workestrate-broker.service.in);
  unit = pkgs.writeText unitName unitText;
  units = pkgs.runCommand "workestrate-broker-units" { } ''
    mkdir -p "$out/${unitDirectory}/multi-user.target.wants"
    install -m 0444 ${unit} "$out/${unitDirectory}/${unitName}"
    ln -s ../${unitName} "$out/${unitDirectory}/multi-user.target.wants/${unitName}"
  '';
  image = guest.mkNixosLayer {
    inherit pkgs base;
    name = "workestrate-broker";
    registrationName = "broker";
    tag = "latest";
    # The unit's store reference retains the complete broker executable closure.
    contents = [ units ];
  };
in
assert lib.assertMsg validPrincipal "broker image requires an exact lowercase DNS host principal";
assert lib.assertMsg (lib.isDerivation brokerd) "brokerd must be an explicit immutable package";
assert lib.assertMsg (
  base.guestInit.executable == "/init"
  && base.guestInit.args == [ ]
  && base.guestInit.env.container == "microsandbox"
  && base.guestInit.workingDir == "/"
) "broker image requires the shared NixOS init contract";
image.overrideAttrs (old: {
  passthru = (old.passthru or { }) // {
    brokerService = {
      inherit
        unit
        unitText
        units
        unitName
        hostPrincipal
        ;
      credentialsMount = "/broker-credentials";
      managementPort = 3024;
      divertPort = 3022;
      egressPort = 3023;
    };
  };
})
