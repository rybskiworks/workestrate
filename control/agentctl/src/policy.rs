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

/// Core-defined secret→host binding allowlist. Each secret may only bind
/// to listed hosts. Replaces the const SecretDefinition hosts field.
pub const SECRET_HOST_BINDINGS: &[(&str, &[&str])] = &[
    ("LITELLM_MASTER_KEY", &["host.microsandbox.internal"]),
    ("OPENROUTER_API_KEY", &["openrouter.ai"]),
    ("KIMI_CODE_API_KEY", &["api.kimi.com"]),
    ("NEURALWATT_API_KEY", &["api.neuralwatt.com"]),
    ("MINIMAX_CODING_API_KEY", &["api.minimax.io"]),
    ("GITHUB_TOKEN", &["github.com", "api.github.com"]),
    ("ODYSSEUS_ADMIN_PASSWORD", &[]), // internal, no egress binding
];

/// Core-defined package vocabulary for nix-layered images.
pub const ALLOWED_PACKAGES: &[&str] = &[
    "cacert",
    "busybox",
    "fakeNss",
    "nodejs_24",
    "nmap",
    "dnsutils",
];

/// Core-defined entitlement: workloads allowed to use default_deny = false.
/// All other workloads are forced to default_deny = true regardless of config.
pub const DEFAULT_DENY_FALSE_ENTITLEMENT: &[&str] = &["tempest", "example-offensive"];

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
    fn secret_host_bindings_hosts_are_subset_of_allowlist() {
        assert!(!SECRET_HOST_BINDINGS.is_empty());
        for (secret, hosts) in SECRET_HOST_BINDINGS {
            assert!(!secret.is_empty(), "secret name must not be empty");
            for host in *hosts {
                assert!(
                    ALLOWED_EGRESS_HOSTS.contains(host),
                    "SECRET_HOST_BINDINGS[{secret}] references '{host}' which is not in \
                     ALLOWED_EGRESS_HOSTS"
                );
            }
        }
        // The host-bridge binding is present (litellm proxy auth).
        let master = SECRET_HOST_BINDINGS
            .iter()
            .find(|(name, _)| *name == "LITELLM_MASTER_KEY")
            .expect("LITELLM_MASTER_KEY binding must exist");
        assert_eq!(master.1, &["host.microsandbox.internal"]);
        // Internal-only secrets bind no hosts.
        let odysseus = SECRET_HOST_BINDINGS
            .iter()
            .find(|(name, _)| *name == "ODYSSEUS_ADMIN_PASSWORD")
            .expect("ODYSSEUS_ADMIN_PASSWORD binding must exist");
        assert!(odysseus.1.is_empty());
    }

    #[test]
    fn secret_host_bindings_names_are_unique() {
        let names: Vec<&str> = SECRET_HOST_BINDINGS.iter().map(|(n, _)| *n).collect();
        assert_no_dupes(&names, "SECRET_HOST_BINDINGS names");
    }

    #[test]
    fn allowed_packages_non_empty_no_dupes() {
        assert!(!ALLOWED_PACKAGES.is_empty());
        assert_no_dupes(ALLOWED_PACKAGES, "ALLOWED_PACKAGES");
        for pkg in ["cacert", "busybox", "nodejs_24"] {
            assert!(
                ALLOWED_PACKAGES.contains(&pkg),
                "ALLOWED_PACKAGES missing '{pkg}'"
            );
        }
    }

    #[test]
    fn default_deny_false_entitlement_contains_expected_workloads() {
        assert!(DEFAULT_DENY_FALSE_ENTITLEMENT.contains(&"tempest"));
        assert!(DEFAULT_DENY_FALSE_ENTITLEMENT.contains(&"example-offensive"));
        assert_no_dupes(
            DEFAULT_DENY_FALSE_ENTITLEMENT,
            "DEFAULT_DENY_FALSE_ENTITLEMENT",
        );
    }
}
