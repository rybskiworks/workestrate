{
  description = "ai-workbench — local AI workbench for Pi/Odysseus/OpenCode through Microsandbox + LiteLLM";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    pi = {
      url = "github:georgrybski/pi";
      flake = false;
    };

    odysseus = {
      url = "github:georgrybski/odysseus";
      flake = false;
    };

    opencode = {
      url = "github:georgrybski/opencode";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, pi, odysseus, opencode, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      microsandbox = pkgs.callPackage ./nix/packages/microsandbox.nix {};
      microsandbox-filesystem-patched = pkgs.callPackage ./nix/packages/microsandbox-filesystem-patched.nix {};
      workestrate = pkgs.callPackage ./nix/packages/agentctl.nix {
        inherit microsandbox microsandbox-filesystem-patched;
      };

      # Hermetic nix build of the pi agent monorepo (runtime tree mounted at /app).
      # Single canonical source: the remote fork (github:georgrybski/pi). One
      # npmDepsHash for the fork's package-lock.json. Local pi hacking uses the
      # hashless `just dev-build-pi` (native npm into agents/pi/build), not a
      # nix override — avoids the lockfile-hash wall.
      pi-built = pkgs.callPackage ./nix/packages/pi.nix { pi = pi; npmDepsHash = "sha256-1EGs8lX8XoAnRtS+pw4lBRm24U/vtVB2loVRmZyd4Z8="; };

      # Standalone Bun-compiled pi binary (self-contained executable, Bun
      # runtime embedded). Reuses the npm-built pi tree + `bun build --compile`.
      pi-bun-built = pkgs.callPackage ./nix/packages/pi-bun.nix { pi-built = pi-built; };

      # Nix-built Docker image for the pi sandbox (dockerTools.buildLayeredImage).
      # Provides nix glibc 2.42 matching the pi-bun binary's PT_INTERP; replaces
      # node:24-bookworm-slim (glibc 2.36) which crashed the bun binary.
      # Load into microsandbox with `just load-pi-image`.
      pi-image = pkgs.callPackage ./nix/packages/pi-image.nix {};

      # Single source of truth for nix-built workload sandbox images. Adding a
      # new workload's image = one entry here; the `load-images` script and
      # the dev-shell check pick it up automatically. No per-image recipes.
      workload-images = {
        workestrator-pi = pkgs.callPackage ./nix/packages/pi-image.nix {};
        # Future: workestrator-odysseus = ...; workestrator-opencode = ...;
      };

      # General loader: iterates `workload-images` and loads each into
      # microsandbox. Driven by the attrset — no hardcoded image names.
      load-images = pkgs.writeShellApplication {
        name = "load-images";
        runtimeInputs = [ msb-wrapped pkgs.gzip ];
        text = let
          names = builtins.attrNames workload-images;
          load-one = name: ''
            echo "Loading ${name}..."
            nix build .#${name} --out-link /tmp/${name}.tar.gz
            gunzip -c /tmp/${name}.tar.gz | msb load -t ${name}:latest
          '';
        in pkgs.lib.concatMapStringsSep "\n" load-one names + ''
          echo ""
          echo "Loaded images:"
          msb image ls
        '';
      };

      # Reusable wrapper around workestrate that bakes WORKESTRATE_PI_BUILD
      # (pointing at the given pi build) into the environment, so `nix run .` /
      # `.#workestrator` runs the pi sandbox without extra env. Wraps the
      # already-wrapped `${workestrate}/bin/workestrate` (which sets MSB_HOME
      # via its postInstall wrapProgram); makeWrapper preserves that inner
      # wrapper's env by exec'ing it, so MSB_HOME is retained.
      workestrator-wrapper = { pi-build }: pkgs.runCommand "workestrator" {
        nativeBuildInputs = [ pkgs.makeWrapper ];
      } ''
        mkdir -p $out/bin
        makeWrapper ${workestrate}/bin/workestrate $out/bin/workestrate \
          --set WORKESTRATE_PI_BUILD ${pi-build}
      '';

      # .#workestrator (default): canonical pi-bun standalone binary (Bun runtime
      # embedded). Behavior unchanged from the previous inline wrapper.
      workestrator = workestrator-wrapper { pi-build = pi-bun-built; };

      # .#workestrator-node: npm/node fallback (the .#pi JS tree). One-command
      # switch — no manual WORKESTRATE_PI_BUILD export needed.
      workestrator-node = workestrator-wrapper { pi-build = pi-built; };

      # Wrap the raw `msb` binary with a stable MSB_HOME so that `msb list`
      # and other runtime commands look in ~/.microsandbox (where workestrate
      # stores the SDK cache/db), not the per-shell build staging directory
      # set by the dev shell's shellHook. $HOME is expanded at wrapper
      # execution time, not at build time.
      msb-wrapped = pkgs.runCommand "msb-wrapped" {
        nativeBuildInputs = [ pkgs.makeWrapper ];
      } ''
        mkdir -p $out/bin
        makeWrapper ${microsandbox}/bin/msb $out/bin/msb \
          --run 'export MSB_HOME="$HOME/.microsandbox"'
      '';

      with-secrets = pkgs.writeShellApplication {
        name = "with-secrets";
        runtimeInputs = [ pkgs.sops pkgs.jq ];
        text = ''
          set -euo pipefail

          : "''${SOPS_AGE_KEY_FILE:=$HOME/.config/sops/age/ai-workbench-secrets.txt}"
          export SOPS_AGE_KEY_FILE

          secret_file="''${SECRET_FILE:-.env.enc}"

          if [ ! -f "$secret_file" ]; then
            echo "error: secret file not found: $secret_file" >&2
            exit 1
          fi

          if [ "$#" -eq 0 ]; then
            echo "usage: with-secrets <command...>" >&2
            echo "example: with-secrets nix run . -- litellm up" >&2
            exit 2
          fi

          # sops exec-env cannot force dotenv input/output type, so we decrypt to
          # JSON and use jq to emit safely shell-escaped export statements.
          eval "$(
            sops decrypt --input-type dotenv --output-type json "$secret_file" \
              | jq -r '
                  to_entries[]
                  | select(.key | test("^[A-Za-z_][A-Za-z0-9_]*$"; "s"))
                  | "export " + .key + "=" + (.value | @sh)
                '
          )"

          exec "$@"
        '';
      };

      run-with-secrets = pkgs.writeShellApplication {
        name = "run-with-secrets";
        runtimeInputs = [ pkgs.sops pkgs.jq workestrate ];
        text = ''
          set -euo pipefail

          : "''${SOPS_AGE_KEY_FILE:=$HOME/.config/sops/age/ai-workbench-secrets.txt}"
          export SOPS_AGE_KEY_FILE

          secret_file="''${SECRET_FILE:-.env.enc}"

          if [ ! -f "$secret_file" ]; then
            echo "error: secret file not found: $secret_file" >&2
            exit 1
          fi

          if [ "$#" -eq 0 ]; then
            echo "usage: run-with-secrets <workestrate-args...>" >&2
            echo "example: run-with-secrets litellm up" >&2
            exit 2
          fi

          # sops exec-env cannot force dotenv input/output type, so we decrypt to
          # JSON and use jq to emit safely shell-escaped export statements.
          eval "$(
            sops decrypt --input-type dotenv --output-type json "$secret_file" \
              | jq -r '
                  to_entries[]
                  | select(.key | test("^[A-Za-z_][A-Za-z0-9_]*$"; "s"))
                  | "export " + .key + "=" + (.value | @sh)
                '
          )"

          exec workestrate "$@"
        '';
      };

      decrypt-env = pkgs.writeShellApplication {
        name = "decrypt-env";
        runtimeInputs = [ pkgs.sops ];
        text = ''
          set -euo pipefail

          : "''${SOPS_AGE_KEY_FILE:=$HOME/.config/sops/age/ai-workbench-secrets.txt}"
          export SOPS_AGE_KEY_FILE

          secret_file="''${SECRET_FILE:-.env.enc}"

          if [ ! -f "$secret_file" ]; then
            echo "error: secret file not found: $secret_file" >&2
            exit 1
          fi

          exec sops decrypt --input-type dotenv --output-type dotenv "$secret_file"
        '';
      };

      write-env = pkgs.writeShellApplication {
        name = "write-env";
        runtimeInputs = [ pkgs.sops pkgs.coreutils ];
        text = ''
          set -euo pipefail

          : "''${SOPS_AGE_KEY_FILE:=$HOME/.config/sops/age/ai-workbench-secrets.txt}"
          export SOPS_AGE_KEY_FILE

          secret_file="''${SECRET_FILE:-.env.enc}"

          if [ ! -f "$secret_file" ]; then
            echo "error: secret file not found: $secret_file" >&2
            exit 1
          fi

          if [ -f .env ]; then
            echo "error: .env already exists; remove it first" >&2
            exit 1
          fi

          umask 077
          tmp=$(mktemp)
          trap 'rm -f "$tmp"' EXIT
          sops decrypt --input-type dotenv --output-type dotenv "$secret_file" > "$tmp"
          mv "$tmp" .env
          chmod 600 .env
          echo "wrote plaintext .env; delete it when done"
        '';
      };

      setup-secrets = pkgs.writeShellApplication {
        name = "setup-secrets";
        runtimeInputs = [ pkgs.sops pkgs.age pkgs.coreutils pkgs.gnugrep pkgs.gnused ];
        text = ''
          set -euo pipefail

          : "''${SOPS_AGE_KEY_FILE:=$HOME/.config/sops/age/ai-workbench-secrets.txt}"
          export SOPS_AGE_KEY_FILE

          exec ${./scripts/setup-secrets.sh} "$@"
        '';
      };
    in {
      devShells.${system}.default = import ./nix/devshells/default.nix {
        inherit pkgs microsandbox microsandbox-filesystem-patched workestrate msb-wrapped with-secrets run-with-secrets decrypt-env write-env setup-secrets load-images
          odysseus opencode pi-bun-built;
        # devshell populates agents/pi/repo from the canonical remote fork.
        pi = pi;
        imageNames = builtins.attrNames workload-images;
      };

      packages.${system} = workload-images // {
        inherit workestrate workestrator workestrator-node microsandbox microsandbox-filesystem-patched msb-wrapped with-secrets run-with-secrets decrypt-env write-env setup-secrets load-images;
        # .#pi = npm/node JS tree (canonical remote fork).
        # .#pi-bun = standalone Bun binary (Bun runtime embedded).
        # Both from one source, one npmDepsHash. Local dev: `just dev-build-pi`.
        pi = pi-built;
        pi-bun = pi-bun-built;
        pi-image = pi-image;
        default = workestrate;
      };

      apps.${system}.default = {
        type = "app";
        program = "${workestrator}/bin/workestrate";
      };
    };
}
