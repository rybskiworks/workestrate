# Hermetic build of the T3MP3ST offensive-security agent for the workestrate
# runtime tree. Produces $out laid out so that
#   node $out/dist/cli.js
# resolves external deps from $out/node_modules.
#
# Purity model (fixed-output derivation pattern):
#  * buildNpmPackage internally splits the build into TWO stages:
#      (1) a fixed-output derivation (fetchNpmDeps) that runs `npm ci`
#          against package-lock.json with a hash-bounded output (npmDepsHash);
#          this FOD is the ONLY step permitted network access.
#      (2) the main derivation that runs `npm run build` against the cached
#          node_modules offline (no network).
#  * npmDepsHash is set to lib.fakeHash below; with a fake hash the FOD build
#    FAILS LOUDLY on first build, printing the correct hash in the standard
#    `specified: ... got: ...` format. Inline the `got:` value to complete
#    the host-side hash computation (HOST-GATE; see comment at npmDepsHash).
#
# Other design choices (unchanged from the prior impure version):
#  * T3MP3ST is a single-package TypeScript app (no npm workspaces), so the
#    build is simpler than pi: `npm run build` runs `tsc` and emits dist/.
#  * --ignore-scripts: skips any prepare/lifecycle hooks (defense-in-depth,
#    matching pi.nix's policy).
#  * NODE_ENV is intentionally NOT set: in the nix sandbox NODE_ENV is unset,
#    so `npm ci` installs devDependencies (typescript, @types/*) which the
#    build needs.
#  * installPhase is overridden: the default buildNpmPackage installPhase puts
#    things under $out/lib/node_modules (library style). T3MP3ST is an app
#    whose runtime expects dist/ + node_modules + package.json at the root,
#    so we reproduce that tree at $out.
#
# Output contract (unchanged): $out is the app tree
#   $out/{dist,node_modules,package.json,package-lock.json}.
{ tempest
, buildNpmPackage
, nodejs_24
, lib
}:

buildNpmPackage {
  pname = "t3mp3st";
  version = "1.0.0";

  src = tempest;

  # HOST-GATE: replace lib.fakeHash with the output of:
  #   nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json
  # OR (equivalently) build once and read the `got:` line from the failure:
  #   nix build .#tempest 2>&1 | grep 'got:'
  # The fake hash fails loudly (hash mismatch) so the build cannot succeed
  # silently with an uncomputed hash. With lib.fakeHash, `nix eval
  # .#tempest.drvPath` still succeeds (drv instantiation does not require the
  # FOD output hash to be correct — only the build does).
  npmDepsHash = lib.fakeHash;

  # Skip lifecycle scripts (defense-in-depth, matching pi.nix).
  npmFlags = [ "--ignore-scripts" ];

  nodejs = nodejs_24;

  # App-style output: reproduce the runtime tree at $out so that
  # `node $out/dist/cli.js` resolves external deps from $out/node_modules.
  installPhase = ''
    runHook preInstall

    mkdir -p $out
    cp -r dist $out/dist
    cp -r node_modules $out/node_modules
    cp package.json $out/package.json
    cp package-lock.json $out/package-lock.json

    runHook postInstall
  '';

  dontStrip = true;

  meta = with lib; {
    description = "T3MP3ST offensive-security multi-agent framework (hermetic nix build)";
    mainProgram = "t3mp3st";
    platforms = platforms.linux;
  };
}
