# Reads a workestrate.toml config via builtins.fromTOML.
# Default: config.reference/workestrate.toml (tracked, sanitized).
#
# B14: the default configDir is a filtered builtins.path that copies ONLY
# workestrate.toml into the store, not the entire repo (the previous
# `configDir ? ../..` default copied 401M+ of agents/pi/repo/node_modules
# on every eval). Callers may still pass an explicit configDir pointing at
# a directory containing workestrate.toml. The filter ensures only the
# TOML file is included; the directory name bounds the store path.
{ configDir ?
    builtins.path {
      path = ../../config.reference;
      filter = path: _type: baseNameOf path == "workestrate.toml";
      name = "workestrate-config-reference";
    } }:
let
  configPath = "${configDir}/workestrate.toml";
  raw = builtins.fromTOML (builtins.readFile configPath);
in {
  secrets = raw.secrets or {};
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
