{
  description = "workestrate — control plane for sandboxed agent/service workloads on Microsandbox microVMs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, fenix, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

      # Pinned Rust toolchain via fenix. Both the dev shell and the nix build
      # (agentctl.nix) consume this so they always agree on the exact rustc.
      # fenix.stable tracks the latest stable at the fenix input revision pinned
      # in flake.lock — currently rustc 1.97.1 (manifest 2026-07-16).
      # The major.minor is echoed below for `just toolchain-check` to parse.
      # RUST_TOOLCHAIN_VERSION = "1.97"
      rustToolchain = fenix.packages.${system}.stable;

      # Config library: reads the tracked reference config from config.reference/
      referenceConfig = import ./nix/lib/config.nix {};

      # Per-system lib exports (Phase 2 core). The function takes pkgs so a
      # config-repo flake can call it with its own nixpkgs.
      libForSystem = { pkgs }:
        let
          recipesForPkgs = import ./nix/lib/recipes.nix { inherit pkgs; };
        in
        {
          recipes = recipesForPkgs;
          vocabulary = recipesForPkgs.vocab;
          # Exposes the parsed config.reference attrset (workloadNames,
          # nixLayeredImages, localBuilds, ...) so gates can eval
          # .#lib.<system>.config.* without --impure / getFlake.
          config = referenceConfig;
          buildWorkloadImage = recipesForPkgs.image.nix-layered;

          # B6 (WP9): buildImagesFromConfig resolves `flake://<name>` URIs in
          # binary.src against the `sources` attrset (name -> path/derivation).
          # This keeps the core flake pure (no builtins.getFlake / impure
          # fetches): the config-repo flake declares its source inputs and
          # passes them via `sources`. An unresolved `flake://` URI is a hard
          # error naming the URI so misconfiguration fails loudly at eval.
          buildImagesFromConfig = { pkgs, config, sources ? {} }:
            let
              recipesForPkgs = import ./nix/lib/recipes.nix { inherit pkgs; };
              lib = pkgs.lib;

              # Resolve a binary.src string: `flake://<name>` -> sources.<name>;
              # any other value is returned as-is (literal store path / path
              # string already resolved by the caller).
              resolveSrc = srcStr:
                if lib.hasPrefix "flake://" srcStr then
                  let name = lib.removePrefix "flake://" srcStr; in
                  if sources ? ${name} then
                    sources.${name}
                  else
                    throw "buildImagesFromConfig: unresolved flake:// URI '${srcStr}'; pass sources.${name} = <path/derivation> to buildImagesFromConfig"
                else
                  srcStr;

              buildBinary = binary:
                if binary.recipe == "bun-compile" then
                  recipesForPkgs.build.bun-compile {
                    src = resolveSrc binary.src;
                    entrypoint = binary.entrypoint;
                    worker = binary.worker;
                    # B4: runtime asset mirroring (list of {from, to}).
                    assets = binary.assets or [];
                    # binary_name / install_dir are nix-only enrichment
                    # fields: the Rust TOML schema has deny_unknown_fields,
                    # so config-repo flakes attach them post-parse (they are
                    # never present in a parsed workestrate.toml). Defaults
                    # preserve the historical $out/bin/app layout.
                    binaryName = binary.binary_name or "app";
                    installDir = binary.install_dir or "bin";
                  }
                else if binary.recipe == "npm-build" then
                  recipesForPkgs.build.npm-build {
                    src = resolveSrc binary.src;
                    npmDepsHash = binary.npm_deps_hash;
                    # B3: optional build/install overrides (pi's 4-workspace
                    # build order + monorepo install layout).
                    dontNpmBuild = binary.dont_npm_build or false;
                    buildPhase = binary.build_phase or null;
                    installPhase = binary.install_phase or null;
                  }
                else if binary.recipe == "pip-install" then
                  recipesForPkgs.build.pip-install {
                    source = resolveSrc binary.src;
                    requirementsFile = binary.requirements_file or "requirements.txt";
                    target = binary.target or ".deps";
                  }
                else if binary.recipe == "bun-install" then
                  recipesForPkgs.build.bun-install {
                    source = resolveSrc binary.src;
                  }
                else
                  throw "unknown binary recipe: ${binary.recipe}";

              workloads = config.workloads or {};
              names = builtins.attrNames workloads;
              nixLayered = builtins.filter (name:
                (workloads.${name}.image.recipe or "") == "nix-layered"
              ) names;
            in
            builtins.listToAttrs (map (name: {
              name = workloads.${name}.image.name;
              value = recipesForPkgs.image.nix-layered {
                inherit (workloads.${name}.image) name tag;
                contents = workloads.${name}.image.contents or [];
                binary = if workloads.${name}.image ? binary then
                  buildBinary workloads.${name}.image.binary
                else
                  null;
                bakedFiles = workloads.${name}.image.baked_files or [];
                features = workloads.${name}.image.features or [];
              };
            }) nixLayered);

          checks.validateConfig = { pkgs, config, workestrate }:
            let
              # config is the TOML text (string). A config-repo flake can pass
              # builtins.readFile ./workestrate.toml directly.
              configFile = pkgs.writeText "workestrate.toml" config;
            in
            pkgs.runCommand "validate-config" {
              nativeBuildInputs = [ workestrate ];
              passAsFile = [ ];
            } ''
              mkdir -p $out
              cp ${configFile} workestrate.toml
              workestrate validate-config
              touch $out/ok
            '';

          # B14 filtered-src pattern (mirror nix/lib/config.nix:10-15): the
          # builtins.path filter copies ONLY *.toml + *.schema.json into the
          # store (directories pass the filter so their subtrees are
          # traversed; non-TOML leaves are dropped). The schema lint resolves
          # [[schemas]] path = schemas/workestrate.schema.json relative to
          # the filtered root, so the schema file must survive the filter.
          # tombi is the 1.2.5 let-bound package (nix/packages/tombi.nix) —
          # passed as an argument like workestrate in validateConfig because
          # the nixpkgs pin ships tombi 0.11.6 (wrong config-key era).
          checks.tombiCheck = { pkgs, src, tombi }:
            pkgs.runCommand "tombi-check" {
              nativeBuildInputs = [ tombi ];
            } ''
              mkdir -p $out
              cd ${builtins.path {
                path = src;
                filter = path: type:
                  type == "directory"
                  || pkgs.lib.hasSuffix ".toml" (baseNameOf path)
                  || pkgs.lib.hasSuffix ".schema.json" (baseNameOf path);
                name = "tombi-check-src";
              }}
              export TOMBI_OFFLINE=true
              tombi format --check
              tombi lint --error-on-warnings
              touch $out/ok
            '';
        };

      agentd = pkgs.callPackage ./nix/packages/agentd.nix {};
      microsandbox = pkgs.callPackage ./nix/packages/microsandbox.nix {
        inherit rustToolchain agentd;
      };
      tombi = pkgs.callPackage ./nix/packages/tombi.nix {};
      microsandbox-filesystem-patched = pkgs.callPackage ./nix/packages/microsandbox-filesystem-patched.nix {};
      workestrate = pkgs.callPackage ./nix/packages/agentctl.nix {
        inherit microsandbox microsandbox-filesystem-patched rustToolchain;
        rev = self.shortRev or (if self ? rev then builtins.substring 0 8 self.rev else "dirty");
      };

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
        inherit pkgs microsandbox microsandbox-filesystem-patched workestrate msb-wrapped decrypt-env write-env setup-secrets
          tombi referenceConfig rustToolchain;
      };

      lib.${system} = libForSystem { inherit pkgs; };

      packages.${system} = {
        inherit workestrate microsandbox microsandbox-filesystem-patched msb-wrapped decrypt-env write-env setup-secrets tombi;
        default = workestrate;
      };

      apps.${system}.default = {
        type = "app";
        program = "${workestrate}/bin/workestrate";
      };
    };
}
