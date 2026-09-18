{
  description = "Explicit local recovery; distribution notices are incomplete";

  # Frozen at the user's original CLI and its matching runtime/firmware pins.
  # The input's committed lockfile supplies its entire dependency graph.
  inputs.workestrate.url =
    "github:rybskiworks/workestrate/bf6fce49037d9ae870b847d7734785fa94dcef79";

  outputs =
    { workestrate, ... }:
    let
      system = "x86_64-linux";
      lib = workestrate.inputs.nixpkgs.lib;
      upstream = workestrate.packages.${system}.default;

      # Deliberately scoped to the frozen recipe. A missing or duplicated
      # boundary must fail, never silently discard an unfamiliar build hook.
      fromUniqueMarker =
        marker: text:
        let
          parts = lib.splitString marker text;
        in
        if builtins.length parts != 2 then
          throw "local recovery: the pinned package hook no longer matches its reviewed boundary"
        else
          marker + builtins.elemAt parts 1;

      recovery = upstream.overrideAttrs (
        old:
        {
          pname = "workestrate-local-recovery";
          name = "workestrate-local-recovery-${old.version}";
          env = (old.env or { }) // {
            WORKESTRATE_REV = "bf6fce4-local-recovery-registry-fix";
          };

          # load_registry returns Result<Option<Registry>>. Match both wrappers
          # before reading fields; preserve the absent/error fallback behavior.
          patches = (old.patches or [ ]) ++ [ ./secrets-registry-option.patch ];

          # Drop the notice-manifest precondition, not SDK/runtime preparation.
          preBuild = ''
            echo "warning: LOCAL RECOVERY BUILD; distribution notices are incomplete" >&2
          '' + fromUniqueMarker "mkdir -p vendor .cargo" old.preBuild;

          # This frozen postBuild contains only the distribution notice job.
          # The normal upstream derivation retains that job without modification.
          postBuild = "";

          # Retain Workestrate's own legal files and the complete original
          # runtime/sops/age/MSB_HOME wrapper. Do not invent a notice manifest.
          postInstall = ''
            mkdir -p "$out/share/licenses/workestrate"
            for name in LICENSE NOTICE LICENSING.md THIRD-PARTY.md; do
              install -m644 "${workestrate}/$name" "$out/share/licenses/workestrate/$name"
            done
            printf '%s\n' \
              'LOCAL RECOVERY BUILD' \
              'Distribution-notice generation and completeness checks were intentionally omitted.' \
              'This package is for temporary local recovery, not release or binary-cache publication.' \
              'No complete compliance, vulnerability-review, or VM-acceptance result is claimed.' \
              'CLI source: bf6fce49037d9ae870b847d7734785fa94dcef79' \
              'CLI patch: secrets-registry-option.patch (Ok(Some(registry)) field access)' \
              'Microsandbox: 251b368a868d578ead123071c3e6bc8eec013817' \
              'libkrunfw: d575b13e79368b23246be3d93d7935899dec5a3b' \
              > "$out/share/licenses/workestrate/LOCAL-RECOVERY.txt"
          '' + fromUniqueMarker "wrapProgram $out/bin/workestrate" old.postInstall;

          passthru = (old.passthru or { }) // {
            localRecovery = true;
            distributionNoticesComplete = false;
          };
          meta = old.meta // {
            description = "Workestrate local recovery, incomplete distribution notices";
          };
        }
      );
    in
    assert workestrate.rev == "bf6fce49037d9ae870b847d7734785fa94dcef79";
    assert workestrate.inputs.microsandbox-fork.rev == "251b368a868d578ead123071c3e6bc8eec013817";
    assert
      workestrate.inputs.microsandbox-fork.inputs.libkrunfw.rev
      == "d575b13e79368b23246be3d93d7935899dec5a3b";
    {
      packages.${system} = {
        workestrate-local-recovery = recovery;
        default = recovery;
      };
    };
}
