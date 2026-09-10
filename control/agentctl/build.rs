// build.rs — agentctl build script.
//
// The only responsibility of this script is version provenance: emit a static
// version literal so main.rs can use `env!("WORKESTRATE_VERSION")`. clap's
// `Command::version` needs `IntoResettable<Str>`; `&'static str` literals
// (which `env!` expands to) are the safe form — a runtime `String` is
// rejected by clap 4.6.0.
//
// `WORKESTRATE_REV` is set by the nix derivation (flake.nix call site ->
// agentctl.nix `env = { WORKESTRATE_REV = rev; }`); absent in plain cargo
// builds -> "dev". `CARGO_PKG_VERSION` is always set by cargo.
fn main() {
    let pkg_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".into());
    let rev = std::env::var("WORKESTRATE_REV").unwrap_or_else(|_| "dev".into());
    println!("cargo:rustc-env=WORKESTRATE_VERSION={pkg_version}-{rev}");
    // Retained Cargo targets must refresh provenance when the caller changes
    // or removes the revision, even if the source files are unchanged.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=WORKESTRATE_REV");
}
