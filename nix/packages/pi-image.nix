# Nix-built Docker image for the pi sandbox.
#
# Background: the `.#pi-bun` binary's PT_INTERP points at nix glibc 2.42
# (e.g. /nix/store/...-glibc-2.42-61/lib/ld-linux-x86-64.so.2). The previous
# sandbox image (`node:24-bookworm-slim`) ships glibc 2.36, so the interpreter
# path does not exist and the binary exits 1 immediately.
#
# Fix: build the image with `dockerTools.buildLayeredImage` from the same
# nixpkgs as pi-bun. The glibc closure (pulled in transitively via cacert +
# busybox + pi-bun-built) lives at the same store path the binary requests,
# so the baked-in `/app/bin/pi` finds its interpreter inside the image.
# busybox provides `/bin/sh` + `tail` (for the tail -f /dev/null entrypoint
# used by the relay); cacert provides CA roots for TLS egress.
#
# The pi-bun standalone binary + assets are baked INTO the image (contents
# includes pi-bun-built; /app/bin symlinks to ${pi-bun-built}/bin). This
# avoids the "mount app: Permission denied" error when the daemon tries to
# bind-mount from /nix/store into the microVM.
#
# Load into microsandbox with: `just load-pi-image`.
{ pkgs, pi-bun-built }:

pkgs.dockerTools.buildLayeredImage {
  name = "workestrator-pi";
  tag = "latest";

  # cacert: CA roots for TLS egress (github.com, LiteLLM proxy TLS interception).
  # busybox: /bin/sh + coreutils (tail -f /dev/null keeps the sandbox alive
  #          for the relay's exec_stream; also provides /tmp if needed).
  # pi-bun-built: the self-contained Bun-compiled pi binary + assets.
  # All three closures pull in nix glibc 2.42 transitively — matching pi-bun's
  # PT_INTERP exactly (same nixpkgs, same flake).
  contents = [ pkgs.cacert pkgs.busybox pi-bun-built pkgs.dockerTools.fakeNss ];

  extraCommands = ''
    # /tmp is needed by some bun internals and by tools that honor TMPDIR.
    # busybox does not create it by default.
    mkdir -p tmp app
    chmod 1777 tmp
    # /app/bin → the pi-bun binary + assets (baked in, no bind mount needed).
    # The daemon can't bind-mount from /nix/store into the microVM, so the
    # binary must live inside the image itself.
    ln -s ${pi-bun-built}/bin app/bin
  '';
}
