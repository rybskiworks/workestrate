# Nix-built Docker image for the T3MP3ST sandbox.
#
# T3MP3ST runs as `node dist/cli.js` (interactive CLI TUI). The image provides:
#  * nodejs_24 — the Node.js runtime
#  * tempest-built — the compiled T3MP3ST tree (dist/ + node_modules/ + package.json)
#  * nmap — network recon (T3MP3ST's kill-chain drives nmap)
#  * bind.dnsutils — dig, nslookup (DNS recon)
#  * cacert — CA roots for TLS egress (LiteLLM proxy)
#  * busybox — /bin/sh + coreutils (tail -f /dev/null keeps the sandbox alive
#              for the relay's exec_stream)
#  * fakeNss — /etc/passwd + /etc/group (uid resolution)
#
# The defaultProvider:"local" config is baked into the image via extraCommands
# (root/.config/t3mp3st/config.json) so T3MP3ST uses the env-var-driven local
# provider without any conf-store secret. No secrets in this file — just
# provider selection.
#
# Load into microsandbox with: `just load-images`.
{ pkgs, tempest-built }:

pkgs.dockerTools.buildLayeredImage {
  name = "tempest";
  tag = "latest";

  contents = [
    pkgs.cacert
    pkgs.busybox
    pkgs.dockerTools.fakeNss
    pkgs.nodejs_24
    tempest-built
    pkgs.nmap
    pkgs.bind.dnsutils
  ];

  extraCommands = ''
    # /tmp is needed by some node internals and by tools that honor TMPDIR.
    # busybox does not create it by default.
    mkdir -p tmp
    chmod 1777 tmp

    # Bake defaultProvider:"local" into the image so T3MP3ST uses the
    # env-var-driven local provider. This is the ONLY conf-store dependency;
    # all secrets come from env vars (TEMPEST_LOCAL_API_KEY, etc.).
    mkdir -p root/.config/t3mp3st
    echo '{"defaultProvider":"local"}' > root/.config/t3mp3st/config.json
  '';
}
