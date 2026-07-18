# nix-layered image recipe: builds a dockerTools.buildLayeredImage.
# Parameters: name, tag, contents (list of package name strings),
# binary (derivation from a build recipe), baked_files, features.
{ pkgs, vocab }:

{ name, tag ? "latest", contents ? [], binary ? null, bakedFiles ? [], features ? [] }:
let
  contentsList = vocab.resolvePackages contents;
  allContents = contentsList ++ (if binary != null then [binary] else []);
  featureShell = vocab.resolveFeatures features;
  bakedShell = vocab.resolveBakedFiles bakedFiles;
in
pkgs.dockerTools.buildLayeredImage {
  inherit name tag;
  contents = allContents;
  extraCommands = featureShell + "\n" + bakedShell;
}
