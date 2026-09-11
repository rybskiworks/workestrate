pub mod network_policy;

/// Core-defined package vocabulary for nix-layered images.
pub const ALLOWED_PACKAGES: &[&str] = &[
    "cacert",
    "busybox",
    // prime's IPython %%bash cells spawn `bash` by name (busybox only has sh).
    "bash",
    "fakeNss",
    "nodejs_24",
    "nmap",
    "dnsutils",
    // prime-agent kernel env (Python 3.11 + ipykernel) + file/tooling utils.
    "python311_kernel",
    "ripgrep",
    "fd",
    "gnutar",
];

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;

    fn assert_no_dupes(list: &[&str], what: &str) {
        let mut seen = std::collections::HashSet::new();
        for item in list {
            assert!(seen.insert(item), "{what} contains duplicate '{item}'");
        }
    }

    #[test]
    fn allowed_packages_non_empty_no_dupes() {
        assert!(!ALLOWED_PACKAGES.is_empty());
        assert_no_dupes(ALLOWED_PACKAGES, "ALLOWED_PACKAGES");
        for pkg in [
            "cacert",
            "busybox",
            "fakeNss",
            "nodejs_24",
            "nmap",
            "dnsutils",
            "python311_kernel",
            "ripgrep",
            "fd",
            "gnutar",
        ] {
            assert!(
                ALLOWED_PACKAGES.contains(&pkg),
                "ALLOWED_PACKAGES missing '{pkg}'"
            );
        }
    }
}
