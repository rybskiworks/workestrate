/// Core-defined allowlist of egress hosts. Config may only reference hosts
/// from this set. Enforced at validate-config, plan (fail-closed), and
/// runtime apply_plan_secrets.
pub const ALLOWED_EGRESS_HOSTS: &[&str] = &[
    "openrouter.ai",
    "api.kimi.com",
    "api.neuralwatt.com",
    "api.minimax.io",
    "github.com",
    "api.github.com",
    "huggingface.co",
    "cdn-lfs.huggingface.co",
    "cdn-lfs-us-1.huggingface.co",
    "host.microsandbox.internal",
];

/// GitHub egress hosts shared by the `github` recipe and the `agent_base`
/// plan recipe. Single source of truth; both reference this const.
pub const GITHUB_HOSTS: &[&str] = &["github.com", "api.github.com"];

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
    fn allowed_egress_hosts_is_non_empty_and_covers_key_providers() {
        assert!(!ALLOWED_EGRESS_HOSTS.is_empty());
        for host in [
            "openrouter.ai",
            "api.kimi.com",
            "api.neuralwatt.com",
            "api.minimax.io",
            "github.com",
            "api.github.com",
            "huggingface.co",
            "host.microsandbox.internal",
        ] {
            assert!(
                ALLOWED_EGRESS_HOSTS.contains(&host),
                "ALLOWED_EGRESS_HOSTS missing '{host}'"
            );
        }
        assert_no_dupes(ALLOWED_EGRESS_HOSTS, "ALLOWED_EGRESS_HOSTS");
    }

    #[test]
    fn github_hosts_matches_github_and_api_only() {
        assert_eq!(GITHUB_HOSTS, &["github.com", "api.github.com"]);
        assert_no_dupes(GITHUB_HOSTS, "GITHUB_HOSTS");
        for host in GITHUB_HOSTS {
            assert!(
                ALLOWED_EGRESS_HOSTS.contains(host),
                "GITHUB_HOSTS entry '{host}' must be in ALLOWED_EGRESS_HOSTS"
            );
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
