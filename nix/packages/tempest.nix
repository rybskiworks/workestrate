# Hermetic build of the T3MP3ST offensive-security agent for the workestrate
# runtime tree. Produces $out laid out so that
#   node $out/dist/cli.js
# resolves external deps from $out/node_modules.
#
# Design choices:
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
{ tempest
, npmDepsHash
, buildNpmPackage
, nodejs_24
, lib
}:

buildNpmPackage {
  pname = "t3mp3st";
  version = "1.0.0";

  src = tempest;

  # Computed via `nix run nixpkgs#prefetch-npm-deps -- <src>/package-lock.json`.
  # Placeholder until computed in a nix-capable environment.
  npmDepsHash = npmDepsHash;

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
