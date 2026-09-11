# Reads a workestrate.toml config via builtins.fromTOML.
# Default: config.reference/workestrate.toml (tracked, sanitized).
#
# Read the file directly without copying its parent directory into the
# store. Path addition preserves path values, including during read-only
# flake checks where an intermediate filtered store copy is not realized.
{
  configDir ? ../../config.reference,
}:
let
  configPath = configDir + "/workestrate.toml";
  raw = builtins.fromTOML (builtins.readFile configPath);
in
{
  secrets = raw.secrets or { };
  workloads = raw.workloads or { };
  workloadNames = builtins.attrNames (raw.workloads or { });

  # Workloads with image.recipe = "nix-layered"
  nixLayeredImages = builtins.filter (
    name:
    let
      wl = raw.workloads.${name};
    in
    (wl.image.recipe or "") == "nix-layered"
  ) (builtins.attrNames (raw.workloads or { }));

  # Workloads with local_build defined
  localBuilds = builtins.filter (name: raw.workloads.${name} ? local_build) (
    builtins.attrNames (raw.workloads or { })
  );
}
