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
      agentctl = pkgs.callPackage ./nix/packages/agentctl.nix {
        inherit microsandbox microsandbox-filesystem-patched;
      };

      # Wrap the raw `msb` binary with a stable MSB_HOME so that `msb list`
      # and other runtime commands look in ~/.microsandbox (where agentctl
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
        runtimeInputs = [ pkgs.sops pkgs.jq agentctl ];
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
            echo "usage: run-with-secrets <agentctl-args...>" >&2
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

          exec agentctl "$@"
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
        inherit pkgs microsandbox microsandbox-filesystem-patched agentctl msb-wrapped with-secrets run-with-secrets decrypt-env write-env setup-secrets
          pi odysseus opencode;
      };

      packages.${system} = {
        inherit agentctl microsandbox microsandbox-filesystem-patched msb-wrapped with-secrets run-with-secrets decrypt-env write-env setup-secrets;
        default = agentctl;
      };

      apps.${system}.default = {
        type = "app";
        program = "${agentctl}/bin/agentctl";
      };
    };
}
