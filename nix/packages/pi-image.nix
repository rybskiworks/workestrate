# Nix-built Docker image for the pi sandbox.
#
# Background: the `.#pi-bun` binary's PT_INTERP points at nix glibc 2.42
# (e.g. /nix/store/...-glibc-2.42-61/lib/ld-linux-x86-64.so.2). The previous
# sandbox image (`node:24-bookworm-slim`) ships glibc 2.36, so the interpreter
# path does not exist and the binary exits 1 immediately.
#
# Fix: build the image with `dockerTools.buildLayeredImage` from the same
# nixpkgs as pi-bun. The glibc closure (pulled in transitively via cacert +
# busybox) lives at the same store path the binary requests, so the bind-mounted
# `/app/bin/pi` (via WORKESTRATE_PI_BUILD) finds its interpreter inside the
# image. busybox provides `/bin/sh` + `tail` (for the tail -f /dev/null
# entrypoint used by the relay); cacert provides CA roots for TLS egress.
#
# Load into microsandbox with: `just load-pi-image`.
{ pkgs }:

pkgs.dockerTools.buildLayeredImage {
  name = "workestrator-pi";
  tag = "latest";

  # cacert: CA roots for TLS egress (github.com, LiteLLM proxy TLS interception).
  # busybox: /bin/sh + coreutils (tail -f /dev/null keeps the sandbox alive
  #          for the relay's exec_stream; also provides /tmp if needed).
  # Both closures pull in nix glibc 2.42 transitively — matching pi-bun's
  # PT_INTERP exactly (same nixpkgs, same flake).
  contents = [ pkgs.cacert pkgs.busybox ];

  extraCommands = ''
    # /tmp is needed by some bun internals and by tools that honor TMPDIR.
    # busybox does not create it by default.
    mkdir -p tmp
    chmod 1777 tmp
  '';
}
