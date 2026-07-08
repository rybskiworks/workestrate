# Nix-built Docker image for the pi sandbox.
#
# Background: the `.#pi-bun` binary's PT_INTERP points at nix glibc 2.42
# (e.g. /nix/store/...-glibc-2.42-61/lib/ld-linux-x86-64.so.2). The previous
# sandbox image (`node:24-bookworm-slim`) ships glibc 2.36, so the interpreter
# path does not exist and the binary exits 1 immediately.
#
# Fix: build the image with `dockerTools.buildLayeredImage` from the same
# nixpkgs as pi-bun. The glibc closure (pulled in transitively via cacert +
# busybox) lives at the same store path the binary requests, so the baked-in
# `/app/bin/pi` finds its interpreter inside the image.
# busybox provides `/bin/sh` + `tail` (for the tail -f /dev/null entrypoint
# used by the relay); cacert provides CA roots for TLS egress.
#
# SIZE OPTIMIZATION: pi-bun-built is NOT in `contents` (that pulled its entire
# nix closure — including pi-built's 438M node_modules — into the image).
# Instead, an intermediate `pi-binary-only` derivation copies ONLY the bun
# binary + assets (dereferenced) and strips the Nix store reference
# to pi-0.79.10 using `remove-references-to`. The bun binary is self-contained
# (bun build --compile embeds the JS); the pi-0.79.10 hash embedded in the
# compiled binary is not needed at runtime. Stripping it breaks the closure
# dependency so dockerTools doesn't pull pi-0.79.10 into the image.
# The glibc reference (needed for PT_INTERP) is preserved.
#
# Load into microsandbox with: `just load-pi-image`.
{ pkgs, pi-bun-built, pi-built }:

let
  # Intermediate derivation: copy the pi-bun binary + assets (dereferenced),
  # then strip the Nix store reference to pi-0.79.10 (the source package with
  # node_modules — 438M). The pi-0.79.10 hash is embedded 2998 times in the
  # compiled binary but is not needed at runtime. Stripping it breaks the
  # closure dependency so dockerTools doesn't pull pi-0.79.10's node_modules
  # into the image. The glibc reference (needed for PT_INTERP) is preserved.
  pi-binary-only = pkgs.runCommand "pi-binary-only" {
    nativeBuildInputs = [ pkgs.removeReferencesTo ];
  } ''
    mkdir -p $out/app/bin
    cp -rL ${pi-bun-built}/bin/* $out/app/bin/
    # Make copied files writable so we can strip references and remove
    # dev-time files below.
    chmod -R +w $out/app/bin
    # Strip the reference to pi-0.79.10 (node_modules bloat) from the binary.
    # The bun binary is self-contained (bun build --compile embeds the JS);
    # it does not need pi-0.79.10 at runtime.
    remove-references-to -t ${pi-built} $out/app/bin/pi
    # Remove the doom-overlay build.sh that references bash (not needed at
    # runtime; it's a dev-time build script).
    rm -f $out/app/bin/examples/extensions/doom-overlay/doom/build.sh
  '';
in

pkgs.dockerTools.buildLayeredImage {
  name = "workestrator-pi";
  tag = "latest";

  # cacert: CA roots for TLS egress (github.com, LiteLLM proxy TLS interception).
  # busybox: /bin/sh + coreutils (tail -f /dev/null keeps the sandbox alive
  #          for the relay's exec_stream; also provides /tmp if needed).
  # fakeNss: /etc/passwd + /etc/group (uid resolution for the pi binary).
  # pi-binary-only: the self-contained Bun-compiled pi binary + assets, with
  #                 the pi-0.79.10 reference stripped (no node_modules bloat).
  # All closures pull in nix glibc 2.42 transitively — matching pi-bun's
  # PT_INTERP exactly (same nixpkgs, same flake).
  contents = [ pkgs.cacert pkgs.busybox pkgs.dockerTools.fakeNss pi-binary-only ];

  extraCommands = ''
    # /tmp is needed by some bun internals and by tools that honor TMPDIR.
    # busybox does not create it by default.
    mkdir -p tmp
    chmod 1777 tmp
  '';
}
