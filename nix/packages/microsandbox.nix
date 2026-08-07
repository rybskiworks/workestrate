# microsandbox — msb CLI + runtime libraries, built from the user's fork.
#
# Source provenance: fork branch fix/filesystem-agentd-path-override, pinned
# via the `microsandbox-fork` flake input at validated rev 74919059. The fork is a 0.6.8 workspace
# (edition 2024, resolver 3). msb is built from source via buildRustPackage
# with the fenix-pinned toolchain (same as agentctl.nix) for host-toolchain
# consistency. agentd is built separately (nix/packages/agentd.nix, musl
# static) and assembled here. libkrunfw comes from the upstream release
# tarball (Branch A, interim) — see CONTINGENCY below.

{ pkgs, rustToolchain, agentd, microsandbox-fork }:

let
  # Mirror agentctl.nix's rustPlatform pattern: fenix-pinned toolchain so the
  # nix build and the dev shell agree on the exact rustc (1.97.1, edition 2024).
  rustPlatform = pkgs.makeRustPlatform {
    rustc = rustToolchain.rustc;
    cargo = rustToolchain.cargo;
  };

  # -----------------------------------------------------------------------
  # libkrunfw — CONTINGENCY
  # -----------------------------------------------------------------------
  # Branch A (default): fetch the upstream v0.6.8 release tarball and extract
  # ONLY libkrunfw.so* from it. The tar sha256 below (line ~44) is REAL and
  # verified against the GitHub v0.6.8 release digest (SRI mSvmbOim... decodes
  # to hex 992be66ce8a61965...). If the tar 404s (release doesn't exist),
  # Branch B becomes mandatory.
  #
  # Branch B (spike, NOT implemented): build libkrunfw from the fork's
  # vendor/libkrunfw submodule (gitlink commit c5503d82, repo
  # https://github.com/superradcompany/libkrunfw.git branch krunfw). The
  # submodule is NOT populated locally. Building it requires kernel build
  # deps (gcc, make, flex, bison, libelf) and produces libkrunfw.so.5.6.1.
  # TODO: if Branch A fails, implement a libkrunfw.nix that fetchGit's the
  # submodule repo at c5503d82 and builds via `make` (see fork justfile
  # build-libkrunfw recipe).
  #
  # The fork's LIBKRUNFW_VERSION is "5.6.1" (ABI "5") — NOT 5.2.1 as in the
  # old 0.5.6 release tarball. The symlink layout must match: libkrunfw.so.5.6.1
  # -> libkrunfw.so.5 -> libkrunfw.so.
  libkrunfwTar = pkgs.fetchurl {
    url = "https://github.com/superradcompany/microsandbox/releases/download/v0.6.8/microsandbox-linux-x86_64.tar.gz";
    sha256 = "sha256-mSvmbOimGWWzrHczvOWNapioKSoXLoXIdRB0oq0W9p0=";
  };
in
rustPlatform.buildRustPackage rec {
  pname = "microsandbox";
  version = "0.6.8";

  src = microsandbox-fork;

  # The flake input unpacks to source/; the whole workspace is needed for cargo to
  # resolve the cli crate's workspace siblings.
  sourceRoot = "source";

  cargoLock = {
    lockFile = src + "/Cargo.lock";
  };

  # Build only the cli crate. Features: net + ssh (matching the fork justfile's
  # build-msb recipe exactly) via --no-default-features, which deliberately
  # excludes `prebuilt` and `keyring`. NOTE: the CLI DOES define `prebuilt` in
  # its default feature set (fork crates/cli/Cargo.toml: prebuilt =
  # ["microsandbox-runtime/prebuilt", "microsandbox/prebuilt"]) — an earlier
  # comment claiming it was not a CLI feature was wrong. With prebuilt
  # excluded, the fork's filesystem crate build.rs takes the NON-prebuilt
  # branch, which requires <workspace>/build/agentd — hence the preBuild
  # staging below is required and correct. agentctl (SDK consumer) keeps its
  # default features (keyring+prebuilt+net) and uses the prebuilt branch via
  # MSB_AGENTD_PATH; feature trimming is a deliberate, deferred decision — do
  # not change any features.
  cargoBuildFlags = [
    "-p" "microsandbox-cli"
    "--no-default-features"
    "--features" "net,ssh"
  ];

  nativeBuildInputs = with pkgs; [
    autoPatchelfHook
    pkg-config
  ];

  buildInputs = with pkgs; [
    libcap_ng
    stdenv.cc.cc.lib
  ];

  preBuild = ''
    # The fork's filesystem crate build.rs (without the 'prebuilt' feature)
    # looks for a pre-built agentd at <workspace>/build/agentd. Stage it here
    # from the agentd derivation. touch ensures the mtime is newer than the
    # source tree (the build.rs staleness check compares against crates/agentd
    # and crates/protocol mtimes — nix source files have fixed mtimes).
    mkdir -p build
    cp ${agentd}/libexec/agentd build/agentd
    touch build/agentd
  '';
  doCheck = false;

  # Assemble the runtime layout the tool expects:
  #   $out/bin/msb           — from cargo target/release/msb
  #   $out/libexec/agentd    — from the agentd derivation (musl static)
  #   $out/lib/libkrunfw.so* — from the upstream release tarball (Branch A)
  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin $out/lib $out/libexec

    # msb CLI binary (regular glibc build — runs on the host).
    # msb CLI binary. buildRustPackage may place it at target/release/msb
    # OR target/<host-triple>/release/msb depending on whether --target is set.
    # Find it robustly and fail loudly if missing (same pattern as agentd.nix).
    msb_bin=$(find target -type f -name msb -path '*/release/*' ! -name '*.d' | head -n1)
    if [ -z "$msb_bin" ]; then
      echo "error: msb binary not found under target/*/release/" >&2
      find target -type f -name msb >&2 || true
      exit 1
    fi
    install -Dm755 "$msb_bin" $out/bin/msb

    # agentd (static musl — runs in the guest microVM).
    install -Dm755 ${agentd}/libexec/agentd $out/libexec/agentd

    # libkrunfw: extract from the upstream release tarball (Branch A).
    # The tar contains lib/libkrunfw.so.5.6.1 (+ possibly other libs).
    tar xzf ${libkrunfwTar} -C $TMPDIR
    if [ -d "$TMPDIR/lib" ]; then
      for f in "$TMPDIR"/lib/libkrunfw.so*; do
        [ -e "$f" ] && cp -P "$f" $out/lib/
      done
    fi
    # Also check the flat layout (some releases put libs at the root).
    for f in "$TMPDIR"/libkrunfw.so*; do
      [ -e "$f" ] && cp -P "$f" $out/lib/
    done

    # Ensure the libkrunfw soname symlinks exist (ABI 5, version 5.6.1).
    if [ -f "$out/lib/libkrunfw.so.5.6.1" ]; then
      ln -sfn libkrunfw.so.5.6.1 $out/lib/libkrunfw.so.5
      ln -sfn libkrunfw.so.5 $out/lib/libkrunfw.so
    elif [ -f "$out/lib/libkrunfw.so.5" ]; then
      ln -sfn libkrunfw.so.5 $out/lib/libkrunfw.so
    fi

    # Fail-closed: libkrunfw is mandatory for the microVM runtime. If the
    # release tarball didn't contain it (wrong version, missing lib/, or the
    # tar 404'd and Branch B is needed), abort loudly rather than shipping a
    # broken msb with no KVM firmware.
    if ! ls $out/lib/libkrunfw.so* >/dev/null 2>&1; then
      echo "error: libkrunfw.so* not found in release tar — fill the hash or implement Branch B (submodule build)" >&2
      exit 1
    fi

    runHook postInstall
  '';

  meta = with pkgs.lib; {
    description = "Microsandbox CLI and runtime libraries (built from fork)";
    homepage = "https://github.com/superradcompany/microsandbox";
    license = licenses.asl20;
    platforms = [ "x86_64-linux" ];
    mainProgram = "msb";
  };
}
