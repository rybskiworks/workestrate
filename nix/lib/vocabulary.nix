# Closed package vocabulary for nix-layered images.
# Config declares package names as strings; this module maps them to pkgs attrs.
# The set of allowed names is defined in policy.rs ALLOWED_PACKAGES.
{ pkgs }:
rec
{
  # String name → nixpkgs derivation
  packages = {
    cacert = pkgs.cacert;
    busybox = pkgs.busybox;
    fakeNss = pkgs.dockerTools.fakeNss;
    nodejs_24 = pkgs.nodejs_24;
    nmap = pkgs.nmap;
    dnsutils = pkgs.bind.dnsutils;
  };

  # Named shell snippets for extraCommands (closed vocabulary)
  features = {
    create_tmp = ''
      mkdir -p tmp
      chmod 1777 tmp
    '';
  };

  # Safe baked_file → shell generation.
  # Path must be relative (no leading /), no ".." traversal.
  # Content is a string (no evaluation).
  bakedFileToShell = { path, content }:
    assert builtins.isString path;
    assert builtins.substring 0 1 path != "/";
    assert !builtins.match ".*\\.\\..*" path;  # no ".." anywhere
    ''
      mkdir -p $(dirname ${path})
      cat > ${path} <<'WORKESTRATE_BAKED_EOF'
      ${content}
      WORKESTRATE_BAKED_EOF
    '';

  # Resolve a list of package name strings to derivations.
  resolvePackages = names:
    builtins.map (n: builtins.getAttr n packages) names;

  # Resolve a list of feature name strings to shell text.
  resolveFeatures = names:
    builtins.concatStringsSep "\n" (builtins.map (n: builtins.getAttr n features) names);

  # Resolve a list of baked_file attrs to shell text.
  resolveBakedFiles = files:
    builtins.concatStringsSep "\n" (builtins.map bakedFileToShell files);
}
