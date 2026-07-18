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
