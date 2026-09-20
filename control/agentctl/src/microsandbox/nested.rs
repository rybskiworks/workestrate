//! Nested-virtualization host probe + pure up-gate decision (ADR 0036).
//!
//! This module owns the ADVISORY host probe ([`NestedProbe`] /
//! [`read_nested_probe`]) and the pure fail-closed decision
//! ([`nested_up_decision`]) shared by BOTH the `plan` warn path and the
//! `up` refuse path, so the two can never disagree about what "host lacks
//! KVM" means. It also owns the pure policy-ladder resolution
//! ([`resolve_virtualization`] / [`resolve_for_workload`]) that maps the
//! workload's `[workloads.<name>.virtualization]` ASK against the collected
//! config `[policy.virtualization]` seal fragments.
//!
//! Layering (ADR 0036 §§3-4):
//! - `validate-config`: static only (closed vocab at parse; cross-rung
//!   freeze is plan-time, not a validate error) — nothing here.
//! - `plan`: [`resolve_for_workload`] + advisory [`read_nested_probe`];
//!   warn, never fail (stays portable).
//! - `up`: the same resolution + probe through [`nested_up_decision`],
//!   evaluated in the build path immediately pre-create so no partial
//!   sandbox is left behind; `prefer` never refuses.
//! - `doctor`: inventory only (`dev_kvm` kept verbatim; the new
//!   `nested_virt` check in `commands::doctor` reuses [`read_nested_probe`]).
//!
//! Phase honesty (plan D6): the Track 1 VMM flag is INERT until the
//! companion firmware rebuild lands (Phase 2) — `nested=require` failing
//! closed on a host without nested KVM is the intended Phase 1 posture,
//! not a claim that nesting works end to end.

use crate::config::{NestedMode, VirtualizationPolicyFragment};
use anyhow::Result;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Advisory host probe
// ---------------------------------------------------------------------------

/// Advisory snapshot of the host's nested-virtualization offering. Built by
/// [`read_nested_probe`] from `/dev/kvm`, `/proc/cpuinfo`, and the
/// `kvm_intel`/`kvm_amd` `nested` module parameters; constructed literally
/// in unit tests (the "mocked probe" matrix — no host I/O there).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedProbe {
    /// `/dev/kvm` exists.
    pub kvm_present: bool,
    /// `/dev/kvm` opens for reading (the invoking user's access).
    pub kvm_accessible: bool,
    /// `/proc/cpuinfo` carries a `vmx` (Intel VT-x) or `svm` (AMD-V) flag.
    pub cpu_flag: bool,
    /// The `kvm_intel`/`kvm_amd` `nested` module parameter: `Some(true)` =
    /// `Y`/`y`/`1`, `Some(false)` = `N`/`n`/`0`, `None` = neither parameter
    /// file present/readable (unknown — WARN, never a refusal by itself).
    pub nested_param: Option<bool>,
    /// `true` only on Linux x86_64 (the sole arch this Linux-microVM stack
    /// supports; anything else fails closed for `require`).
    pub arch_supported: bool,
}

impl NestedProbe {
    /// A probe reporting NO host offering (non-KVM hosts, non-Linux CI).
    pub fn absent() -> Self {
        Self {
            kvm_present: false,
            kvm_accessible: false,
            cpu_flag: false,
            nested_param: None,
            arch_supported: false,
        }
    }

    /// A probe reporting the FULL host offering (nested-capable KVM host).
    pub fn full() -> Self {
        Self {
            kvm_present: true,
            kvm_accessible: true,
            cpu_flag: true,
            nested_param: Some(true),
            arch_supported: true,
        }
    }
}

/// Default host paths read by [`read_nested_probe`].
const KVM_PATH: &str = "/dev/kvm";
const CPUINFO_PATH: &str = "/proc/cpuinfo";
const SYS_MODULE_ROOT: &str = "/sys/module";

/// Parse the effective uid from `/proc/self/status` text (the `Uid:` line
/// carries real/effective/saved/fs uids; the SECOND is the euid). `None`
/// when the line is absent/unparseable — the refusal message then reports
/// `uid=unknown`. Dependency-free: `unsafe_code` is forbidden workspace-wide
/// and no uid crate is in the build graph, so libc `getuid` is unavailable.
pub(crate) fn parse_euid_from_status(text: &str) -> Option<u32> {
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("Uid:") else {
            continue;
        };
        let mut fields = rest.split_whitespace();
        fields.next()?;
        return fields.next()?.parse::<u32>().ok();
    }
    None
}

/// The invoking effective uid for the "exists but not accessible by uid=…"
/// refusal detail (ADR 0036 §4). Linux-only source; everywhere else (and on
/// read/parse failure) reports `unknown` rather than failing.
fn current_euid_for_message() -> String {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|text| parse_euid_from_status(&text))
        .map(|uid| uid.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// `/dev/kvm` presence + readability at `path` (parameterized for tests;
/// production passes `/dev/kvm`).
fn kvm_status(kvm: &Path) -> (bool, bool) {
    if !kvm.exists() {
        return (false, false);
    }
    match std::fs::File::open(kvm) {
        Ok(_) => (true, true),
        Err(_) => (true, false),
    }
}

/// Whether `cpuinfo` text advertises VT-x/AMD-V (the same `vmx|svm`
/// substring match `scripts/host-check.sh` uses).
fn cpu_flag_from(cpuinfo: &str) -> bool {
    cpuinfo.contains("vmx") || cpuinfo.contains("svm")
}

/// Parse one `nested` module-parameter file body (`Y`/`y`/`1` → true,
/// `N`/`n`/`0` → false, anything else → None).
fn parse_nested_param(body: &str) -> Option<bool> {
    match body.trim() {
        "Y" | "y" | "1" => Some(true),
        "N" | "n" | "0" => Some(false),
        _ => None,
    }
}

/// Read the `nested` parameter, Intel first then AMD, under the
/// `<root>/kvm_intel|kvm_amd/parameters/nested` paths (parameterized for
/// tests; production passes `/sys/module`). First parseable file wins;
/// unreadable/missing both → None (unknown).
fn nested_param_from(sys_module_root: &Path) -> Option<bool> {
    for module in ["kvm_intel", "kvm_amd"] {
        let path: PathBuf = sys_module_root
            .join(module)
            .join("parameters")
            .join("nested");
        if let Ok(body) = std::fs::read_to_string(&path)
            && let Some(value) = parse_nested_param(&body)
        {
            return Some(value);
        }
    }
    None
}

/// Advisory host probe: read `/dev/kvm`, `/proc/cpuinfo`, and the
/// `kvm_intel`/`kvm_amd` `nested` parameters. Never fails — every read
/// degrades to absent/unknown (the DECISION fails closed, not the probe).
pub fn read_nested_probe() -> NestedProbe {
    let (kvm_present, kvm_accessible) = kvm_status(Path::new(KVM_PATH));
    let cpu_flag = std::fs::read_to_string(CPUINFO_PATH)
        .map(|body| cpu_flag_from(&body))
        .unwrap_or(false);
    let nested_param = nested_param_from(Path::new(SYS_MODULE_ROOT));
    let arch_supported = std::env::consts::OS == "linux" && std::env::consts::ARCH == "x86_64";
    NestedProbe {
        kvm_present,
        kvm_accessible,
        cpu_flag,
        nested_param,
        arch_supported,
    }
}

/// The FULL host offering: KVM present AND accessible AND a CPU flag AND
/// the nested parameter affirmatively enabled, on a supported arch. The
/// `prefer` degrade flag and the `plan` note/warn split key off this.
pub fn host_offers_nested(probe: &NestedProbe) -> bool {
    probe.arch_supported
        && probe.kvm_present
        && probe.kvm_accessible
        && probe.cpu_flag
        && probe.nested_param == Some(true)
}

// ---------------------------------------------------------------------------
// Policy-ladder resolution (pure)
// ---------------------------------------------------------------------------

/// Pure resolution of ONE workload's nested-virt posture (ADR 0036 §§3/5,
/// D8): the workload's `[workloads.<name>.virtualization]` ASK against the
/// collected config `[policy.virtualization]` seal rungs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualizationResolution {
    /// Effective ask (`None` at the call site = omitted → Off).
    pub effective: NestedMode,
    /// Operator grant after walking the seal rungs (absent fragment = no
    /// restriction → true).
    pub allowed: bool,
    /// `effective ≠ off` but the seal denies it (plan reports, up refuses).
    pub frozen_out: bool,
    /// Origin label of the rung that denied/sealed (the ladder's declaring
    /// layer name or `config-registry`), when frozen out.
    pub frozen_by: Option<String>,
    /// Where the ASK came from (merge-provenance layer label) — named in
    /// the `plan` warn line.
    pub origin: String,
    /// A `final` seal stopped the rung walk (even when it did not deny —
    /// a seal on an allow still freezes lower rungs).
    pub sealed: bool,
}

/// Walk the seal `rungs` authority-ASCENDING (config registry first, then
/// config layers in stack order, then the workload capsule's rungs — the
/// secrets-ladder walk shape) and resolve the `ask` against them.
///
/// - Later `allow_nested` declarations win; absent everywhere = no
///   restriction (`allowed = true`).
/// - `final = true` is a terminal freeze: the walk stops, lower rungs
///   cannot override (a bare final with no `allow_nested` freezes whatever
///   the higher rungs resolved so far — the documented secrets deviation).
/// - `frozen_out` = ask ≠ off but denied. `validate` passes per-rung-legal
///   regardless (freeze is plan-time, never a validate error).
pub fn resolve_virtualization(
    ask: Option<NestedMode>,
    ask_origin: &str,
    rungs: &[(&str, &VirtualizationPolicyFragment)],
) -> VirtualizationResolution {
    let effective = ask.unwrap_or(NestedMode::Off);
    let mut allowed = true;
    let mut denying_rung: Option<String> = None;
    let mut sealed = false;
    for (label, fragment) in rungs {
        if let Some(value) = fragment.allow_nested {
            allowed = value;
            if value {
                denying_rung = None;
            } else {
                denying_rung = Some((*label).to_string());
            }
        }
        if fragment.r#final {
            sealed = true;
            break;
        }
    }
    let frozen_out = effective != NestedMode::Off && !allowed;
    let frozen_by = if frozen_out { denying_rung } else { None };
    VirtualizationResolution {
        effective,
        allowed,
        frozen_out,
        frozen_by,
        origin: ask_origin.to_string(),
        sealed,
    }
}

/// Resolve ONE workload against the process-global collected ladder (set at
/// load time by `config::loading`; absent — synthetic/test paths without a
/// load — degrades to the ask alone, no restriction). `ask_origin` is the
/// merge-provenance label for `workloads.<name>.virtualization` (falls back
/// to `"declared"` when the merge recorded no provenance).
pub fn resolve_for_workload(
    workload_name: &str,
    ask: Option<NestedMode>,
    ask_origin: Option<&str>,
) -> VirtualizationResolution {
    let ladder = crate::merge::get_virtualization_ladder().unwrap_or_default();
    let mut rungs: Vec<(String, VirtualizationPolicyFragment)> = Vec::new();
    if let Some((origin, fragment)) = &ladder.config {
        rungs.push((origin.clone(), fragment.clone()));
    }
    for (origin, fragment) in &ladder.layers {
        rungs.push((origin.clone(), fragment.clone()));
    }
    if let Some(workload_rungs) = ladder.workloads.get(workload_name) {
        for (origin, fragment) in workload_rungs {
            rungs.push((origin.clone(), fragment.clone()));
        }
    }
    let rung_refs: Vec<(&str, &VirtualizationPolicyFragment)> =
        rungs.iter().map(|(o, f)| (o.as_str(), f)).collect();
    resolve_virtualization(ask, ask_origin.unwrap_or("declared"), &rung_refs)
}

// ---------------------------------------------------------------------------
// Fail-closed up decision (pure)
// ---------------------------------------------------------------------------

/// Fail-closed `up` decision shared by the `plan` warn path (advisory) and
/// the `up` refuse path (hard) — ADR 0036 §4.
///
/// - `off` → always Ok (current behavior, no checks).
/// - `prefer` → NEVER refuses (degrades; the degraded state lands in the
///   plan provenance per D7).
/// - `require` → refuses when the config-final seal froze it out, when
///   `/dev/kvm` is missing or not accessible, when the arch is unsupported,
///   or when the CPU nested parameter is affirmatively disabled. An UNKNOWN
///   nested parameter (`None`) is a `plan` note / doctor WARN, never a
///   refusal — the two refusal shapes stay exactly the plan §4 pair.
/// - `frozen_by` carries the denying rung's origin for the seal refusal.
///
/// Errors carry the full `error:` + `hint:` text so the negative
/// exact-error test pins ONE site.
pub fn nested_up_decision(
    workload: &str,
    nested: NestedMode,
    probe: &NestedProbe,
    frozen_by: Option<&str>,
) -> Result<()> {
    if nested == NestedMode::Off {
        return Ok(());
    }
    if let Some(sealed_by) = frozen_by {
        anyhow::bail!(
            "error: workload '{workload}' requests nested virtualization (virtualization.nested=\"{nested}\") \
             but the config [policy.virtualization] seal forbids it (frozen by '{sealed_by}': allow_nested=false, final=true) — \
             see ADR 0036 §4 (final seals, never enables)\n\
             hint: ask the config operator to relax the seal, or set nested=\"off\" to opt out"
        );
    }
    if nested == NestedMode::Prefer {
        return Ok(());
    }
    // From here: require only.
    if !probe.kvm_present {
        anyhow::bail!(
            "error: workload '{workload}' requires KVM (virtualization.nested=\"require\") \
             but host /dev/kvm is not found — \
             see ADR 0036 §4 (guest /dev/kvm needs host KVM + nested; host gate scripts/host-check.sh)\n\
             hint: enable virtualization in BIOS + sudo modprobe kvm(_intel|_amd) + \
             sudo usermod -aG kvm $USER (re-login); or nested=\"prefer\" to degrade, or nested=\"off\" to opt out"
        );
    }
    if !probe.kvm_accessible {
        let uid = current_euid_for_message();
        anyhow::bail!(
            "error: workload '{workload}' requires KVM (virtualization.nested=\"require\") \
             but host /dev/kvm exists but not accessible by uid={uid} — \
             see ADR 0036 §4 (guest /dev/kvm needs host KVM + nested; host gate scripts/host-check.sh)\n\
             hint: enable virtualization in BIOS + sudo modprobe kvm(_intel|_amd) + \
             sudo usermod -aG kvm $USER (re-login); or nested=\"prefer\" to degrade, or nested=\"off\" to opt out"
        );
    }
    if !probe.arch_supported {
        anyhow::bail!(
            "error: workload '{workload}' requires KVM (virtualization.nested=\"require\") \
             but nested virtualization is only supported on Linux x86_64 (this host: {} {}) — \
             see ADR 0036 §4 (guest /dev/kvm needs host KVM + nested; host gate scripts/host-check.sh)\n\
             hint: run on a Linux x86_64 host with virtualization enabled in BIOS; \
             or nested=\"prefer\" to degrade, or nested=\"off\" to opt out",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    }
    if probe.nested_param == Some(false) {
        anyhow::bail!(
            "error: workload '{workload}' requires KVM (virtualization.nested=\"require\") \
             but host CPU nested virtualization is disabled (kvm_intel/kvm_amd nested parameter is N) — \
             see ADR 0036 §4 (guest /dev/kvm needs host KVM + nested; host gate scripts/host-check.sh)\n\
             hint: enable nested virtualization (sudo modprobe -r kvm_intel && sudo modprobe kvm_intel nested=1, \
             or the kvm_amd equivalent) + sudo modprobe kvm(_intel|_amd); \
             or nested=\"prefer\" to degrade, or nested=\"off\" to opt out"
        );
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use crate::config::VirtualizationPolicyFragment;

    fn fragment(allow: Option<bool>, sealed: bool) -> VirtualizationPolicyFragment {
        VirtualizationPolicyFragment {
            allow_nested: allow,
            r#final: sealed,
        }
    }

    // ---- unit matrix: kvm{present,absent,unreadable} x nested{off,prefer,require} ----

    #[test]
    fn up_decision_matrix_kvm_state_by_nested_mode() {
        let absent = NestedProbe::absent();
        let unreadable = NestedProbe {
            kvm_present: true,
            kvm_accessible: false,
            cpu_flag: true,
            nested_param: Some(true),
            arch_supported: true,
        };
        let full = NestedProbe::full();
        // off: always Ok regardless of host.
        for probe in [&absent, &unreadable, &full] {
            assert!(
                nested_up_decision("job", NestedMode::Off, probe, None).is_ok(),
                "off must never refuse"
            );
        }
        // prefer: never refuses regardless of host.
        for probe in [&absent, &unreadable, &full] {
            assert!(
                nested_up_decision("job", NestedMode::Prefer, probe, None).is_ok(),
                "prefer must never refuse"
            );
        }
        // require + full host: Ok.
        assert!(nested_up_decision("job", NestedMode::Require, &full, None).is_ok());
        // require + absent kvm: exact refusal shape (not found).
        let err = nested_up_decision("job", NestedMode::Require, &absent, None)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("workload 'job' requires KVM")
                && err.contains("virtualization.nested=\"require\"")
                && err.contains("not found")
                && err.contains("ADR 0036 §4")
                && err.contains("scripts/host-check.sh")
                && err.contains("hint:"),
            "require/absent refusal shape: {err}"
        );
        // require + unreadable kvm: exact refusal shape (uid detail).
        let err = nested_up_decision("job", NestedMode::Require, &unreadable, None)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("exists but not accessible by uid=") && err.contains("usermod -aG kvm"),
            "require/unreadable refusal shape: {err}"
        );
    }

    #[test]
    fn up_decision_require_refuses_affirmatively_disabled_nested_param() {
        let probe = NestedProbe {
            nested_param: Some(false),
            ..NestedProbe::full()
        };
        let err = nested_up_decision("kvm-job", NestedMode::Require, &probe, None)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("nested parameter is N") && err.contains("nested=1"),
            "second refusal cites the kvm_intel nested parameter: {err}"
        );
        // Unknown param (None) is NOT a refusal — plan note / doctor WARN only.
        let probe = NestedProbe {
            nested_param: None,
            ..NestedProbe::full()
        };
        assert!(nested_up_decision("kvm-job", NestedMode::Require, &probe, None).is_ok());
    }

    #[test]
    fn up_decision_require_fails_closed_off_arch() {
        let probe = NestedProbe {
            arch_supported: false,
            ..NestedProbe::full()
        };
        let err = nested_up_decision("job", NestedMode::Require, &probe, None)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("only supported on Linux x86_64"),
            "non-Linux/x86_64 + require fails closed: {err}"
        );
        // Frozen seal refuses for BOTH require and prefer (prefer never
        // refuses for HOST reasons, but a config-final ban still seals it).
        for mode in [NestedMode::Require, NestedMode::Prefer] {
            let err =
                nested_up_decision("job", mode, &NestedProbe::full(), Some("config-registry"))
                    .unwrap_err()
                    .to_string();
            assert!(
                err.contains("seal forbids it") && err.contains("config-registry"),
                "frozen {mode} must refuse citing the seal: {err}"
            );
        }
        // Off + frozen seal: no ask, no refusal.
        assert!(
            nested_up_decision(
                "job",
                NestedMode::Off,
                &NestedProbe::full(),
                Some("config-registry")
            )
            .is_ok()
        );
    }

    // ---- tri-state resolution transitions ----

    #[test]
    fn resolve_absent_everything_is_off_unrestricted() {
        let res = resolve_virtualization(None, "declared", &[]);
        assert_eq!(res.effective, NestedMode::Off);
        assert!(res.allowed);
        assert!(!res.frozen_out);
        assert_eq!(res.frozen_by, None);
        assert!(!res.sealed);
    }

    #[test]
    fn resolve_config_ban_freezes_require() {
        let config = fragment(Some(false), true);
        let rungs = [("config-registry", &config)];
        let res = resolve_virtualization(Some(NestedMode::Require), "personal", &rungs);
        assert_eq!(res.effective, NestedMode::Require);
        assert!(!res.allowed);
        assert!(res.frozen_out);
        assert_eq!(res.frozen_by.as_deref(), Some("config-registry"));
        assert!(res.sealed);
        assert_eq!(res.origin, "personal");
    }

    #[test]
    fn resolve_grant_enables_nothing_by_itself() {
        // `allow_nested=true` grants nothing: off stays off (the workload
        // still needs an explicit nested≠off).
        let config = fragment(Some(true), false);
        let rungs = [("config-registry", &config)];
        let res = resolve_virtualization(None, "declared", &rungs);
        assert_eq!(res.effective, NestedMode::Off);
        assert!(res.allowed);
        assert!(!res.frozen_out);
    }

    #[test]
    fn resolve_bare_final_freezes_so_far_without_denying() {
        // A bare final (no allow_nested) freezes the built-in allow: lower
        // rungs cannot enable OR deny past it, and nothing is frozen out.
        let config = fragment(None, true);
        let layer = fragment(Some(false), false);
        let rungs = [("config-registry", &config), ("personal", &layer)];
        let res = resolve_virtualization(Some(NestedMode::Prefer), "personal", &rungs);
        assert!(res.allowed);
        assert!(!res.frozen_out);
        assert!(res.sealed);
    }

    #[test]
    fn resolve_later_allow_wins_until_sealed() {
        // Non-final deny then layer allow: the later (more specific) rung wins.
        let config = fragment(Some(false), false);
        let layer = fragment(Some(true), false);
        let rungs = [("config-registry", &config), ("personal", &layer)];
        let res = resolve_virtualization(Some(NestedMode::Require), "personal", &rungs);
        assert!(res.allowed);
        assert!(!res.frozen_out);
        // Reversed: seal at config stops the layer allow from rescuing.
        let config = fragment(Some(false), true);
        let layer = fragment(Some(true), false);
        let rungs = [("config-registry", &config), ("personal", &layer)];
        let res = resolve_virtualization(Some(NestedMode::Require), "personal", &rungs);
        assert!(!res.allowed);
        assert_eq!(res.frozen_by.as_deref(), Some("config-registry"));
    }

    // ---- probe helpers ----

    #[test]
    fn probe_helpers_parse_shapes() {
        assert!(cpu_flag_from("flags : fpu vmx smx"));
        assert!(cpu_flag_from("flags : fpu svm skinit"));
        assert!(!cpu_flag_from("flags : fpu cx8 sep"));
        assert_eq!(parse_nested_param("Y\n"), Some(true));
        assert_eq!(parse_nested_param("1"), Some(true));
        assert_eq!(parse_nested_param("N\n"), Some(false));
        assert_eq!(parse_nested_param("0"), Some(false));
        assert_eq!(parse_nested_param(""), None);
        assert!(host_offers_nested(&NestedProbe::full()));
        assert!(!host_offers_nested(&NestedProbe::absent()));
        assert!(!host_offers_nested(&NestedProbe {
            nested_param: None,
            ..NestedProbe::full()
        }));
    }

    #[test]
    fn euid_parses_from_proc_status_shape() {
        let text =
            "Name:\tworkestrate\nUid:\t1000\t1000\t1000\t1000\nGid:\t1000\t1000\t1000\t1000\n";
        assert_eq!(parse_euid_from_status(text), Some(1000));
        assert_eq!(parse_euid_from_status("Name:\tx\n"), None);
    }
}
