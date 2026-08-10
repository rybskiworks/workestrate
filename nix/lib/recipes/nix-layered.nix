# nix-layered image recipe: builds a dockerTools.buildLayeredImage.
# Parameters: name, tag, contents (list of package name strings),
# binary (derivation from a build recipe), baked_files, features.
# env: optional attrset of NAME=VALUE image env vars, e.g.
# { PRIME_AGENT_KERNEL_PYTHON = "/nix/store/.../bin/python"; }, baked into
# the image's docker config.Env; empty default = back-compat no-op.
{ pkgs, vocab }:

{ name, tag ? "latest", contents ? [], binary ? null, bakedFiles ? [], features ? [], env ? {} }:
let
  contentsList = vocab.resolvePackages contents;
  allContents = contentsList ++ (if binary != null then [binary] else []);
  featureShell = vocab.resolveFeatures features;
  bakedShell = vocab.resolveBakedFiles bakedFiles;
  envList = builtins.map (n: "${n}=${env.${n}}") (builtins.attrNames env);
in
pkgs.dockerTools.buildLayeredImage ({
  inherit name tag;
  contents = allContents;
  extraCommands = featureShell + "\n" + bakedShell;
} // (if env == {} then {} else { config = { Env = envList; }; }))
