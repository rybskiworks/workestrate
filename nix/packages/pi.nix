# Hermetic build of the pi agent (npm-workspaces monorepo) for the workestrate
# runtime tree. Produces $out laid out so that
#   node $out/packages/coding-agent/dist/cli.js
# resolves sibling-workspace imports (../ai, ../agent, ../tui) and external deps.
#
# Design choices:
#  * --ignore-scripts: pi's own AGENTS.md mandates `npm ci --ignore-scripts`.
#    This skips husky's `prepare` lifecycle hook AND skips canvas's node-gyp
#    native build (packages/ai devDepends on canvas@3.2.3, which would otherwise
#    need cairo/pango). The existing dev-shell shellHook proved this works for
#    building pi. We deliberately do NOT add cairo/pango/canvas native support
#    in this first cut.
#  * NODE_ENV is intentionally NOT set: in the nix sandbox NODE_ENV is unset, so
#    `npm ci` installs devDependencies (tsgo, esbuild, shx, etc.) which the
#    build needs. (The dev-shell sets NODE_ENV=development explicitly; here the
#    absence has the same effect.)
#  * installPhase is overridden: the default buildNpmPackage installPhase puts
#    things under $out/lib/node_modules (library style). pi is an app whose
#    runtime expects the monorepo layout (packages/* + node_modules + root
#    package.json) at the mount root, so we reproduce that tree at $out.
#  * dontNpmBuild + custom buildPhase: pi's ai workspace build script is
#    `npm run generate-models && npm run generate-image-models && tsgo ...`.
#    The generate-* scripts DELETE the committed per-provider *.models.ts
#    catalogs then attempt to fetch live provider data (models.dev, OpenRouter,
#    Vercel AI Gateway) to regenerate them. In the offline nix sandbox the
#    fetches fail, leaving the catalogs deleted and tsgo unable to resolve
#    imports (TS2307). We therefore skip the generate step entirely and build
#    directly with `tsgo -p tsconfig.build.json` against the COMMITTED
#    catalogs. The committed catalogs at the current fork rev are fully
#    populated (35 providers, ~1000 model entries), so the built pi has real
#    model knowledge without needing live data. The four workspace builds
#    (tui -> ai -> agent -> coding-agent) are chained manually in dependency
#    order, matching the root `build` script.
{ pi
, npmDepsHash
, buildNpmPackage
, nodejs_24
, autoPatchelfHook
, stdenv
, libcap_ng
, lib
}:

buildNpmPackage {
  pname = "pi";
  version = "0.79.10";

  src = pi;

  # Computed via `nix run nixpkgs#prefetch-npm-deps -- <src>/package-lock.json`.
  npmDepsHash = npmDepsHash;

  # Skip the default `npm run build` (which chains generate-models +
  # generate-image-models + tsgo). The generate scripts delete committed
  # catalogs and can't regenerate them offline. See header comment.
  dontNpmBuild = true;

  # Skip lifecycle scripts (husky prepare, canvas node-gyp). See header comment.
  npmFlags = [ "--ignore-scripts" ];

  nodejs = nodejs_24;

  # tsgo (@typescript/native-preview) and esbuild ship prebuilt native ELF
  # binaries inside node_modules. autoPatchelfHook patches their interpreter /
  # RPATH so they run in the sandbox. stdenv.cc provides libstdc++.
  nativeBuildInputs = [ autoPatchelfHook ];
  # libcap-ng: gondolin's libkrun (node_modules/@earendil-works/gondolin-krun-runner-linux-x64)
  # needs libcap-ng.so.0 at runtime; autoPatchelfHook wires the RPATH.
  buildInputs = [ stdenv.cc.cc.lib libcap_ng ];

  # Build the four workspaces in dependency order, skipping the ai workspace's
  # generate-models/generate-image-models (offline; committed catalogs used).
  # tsgo is invoked directly via node_modules/.bin since we bypass `npm run`.
  buildPhase = ''
    runHook preBuild
    cd packages/tui && npm run build && cd ../..
    cd packages/ai && ../../node_modules/.bin/tsgo -p tsconfig.build.json && cd ../..
    cd packages/agent && npm run build && cd ../..
    cd packages/coding-agent && npm run build && cd ../..
    runHook postBuild
  '';

  # App-style output: reproduce the monorepo runtime tree at $out so that
  # `node $out/packages/coding-agent/dist/cli.js` resolves workspace siblings
  # (../ai, ../agent, ../tui) and external deps from $out/node_modules.
  installPhase = ''
    runHook preInstall

    mkdir -p $out
    cp -r packages $out/packages
    cp -r node_modules $out/node_modules
    cp package.json $out/package.json
    cp package-lock.json $out/package-lock.json

    runHook postInstall
  '';

  dontStrip = true;

  meta = with lib; {
    description = "pi coding agent (hermetic nix build of the monorepo runtime tree)";
    mainProgram = "pi";
    platforms = platforms.linux;
  };
}
