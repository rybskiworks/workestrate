# Reads a workestrate.toml config via builtins.fromTOML.
# Default: config.reference/workestrate.toml (tracked, sanitized).
{ configDir ? ../.. }:
let
  configPath = "${configDir}/config.reference/workestrate.toml";
  raw = builtins.fromTOML (builtins.readFile configPath);
in {
  inherit (raw) secrets;
  workloads = raw.workloads or {};
  workloadNames = builtins.attrNames (raw.workloads or {});

  # Workloads with image.recipe = "nix-layered"
  nixLayeredImages = builtins.filter (name:
    let wl = raw.workloads.${name}; in
    (wl.image.recipe or "") == "nix-layered"
  ) (builtins.attrNames (raw.workloads or {}));

  # Workloads with local_build defined
  localBuilds = builtins.filter (name:
    raw.workloads.${name} ? local_build
  ) (builtins.attrNames (raw.workloads or {}));
}
