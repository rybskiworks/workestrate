//! `agentctl providers check` — load `.env` and validate the four key
//! pieces of provider configuration per provider.
//!
//! The POC's threat model says:
//!
//!   * The LiteLLM process sees ONLY a dummy provider key.
//!   * The real provider key is held in `infra/litellm/.env` and read only
//!     by the host-side egress proxy.
//!   * The `FAKE_PROVIDER_API_BASE` URL points at the egress proxy (in
//!     POC mode) or at the real provider host (in production with
//!     microsandbox).
//!
//! `providers check` reports, for each provider, whether its dummy key,
//! real key, and base URL are present, and (where the field exists) the
//! host portion of the base URL matches what we expect.

use std::collections::BTreeMap;

use crate::config;
use crate::envfile;

#[derive(Debug)]
struct Provider {
    name: String,
    /// Env var name for the dummy key (what the LiteLLM process holds).
    dummy_key_var: String,
    /// Env var name for the real key (held by the host, never passed to
    /// LiteLLM).
    real_key_var: String,
    /// Env var name for the API base URL.
    base_var: String,
    /// Whether the dummy key must be a real (non-placeholder) value.
    /// `false` for the threat-model providers (the dummy *is* a placeholder).
    dummy_must_be_real: bool,
    /// Expected host portion of the base URL (e.g. `127.0.0.1:8082` for
    /// POC, or `api.openai.com` for production). If set, the check fails
    /// loudly if the configured base points elsewhere.
    expected_base_host: Option<String>,
}

pub fn check() -> Result<u8, String> {
    let env_path = config::env_file_path();
    if !env_path.exists() {
        return Err(format!(
            ".env not found at {}. Run `agentctl init` first.",
            env_path.display()
        ));
    }
    let entries = envfile::load(&env_path)?;
    let map: BTreeMap<String, String> = entries
        .into_iter()
        .map(|e| (e.key, e.value))
        .collect();

    // The "dummy" key for a provider is, by threat-model design,
    // expected to be a placeholder string. We therefore invert the
    // "placeholder = bad" check for the *dummy* key: it is OK for the
    // dummy key to be a placeholder; in fact, that is the whole point.
    // The *real* key must not be a placeholder.
    let providers = vec![
        Provider {
            name: "litellm".to_string(),
            // LiteLLM doesn't talk to a "real" provider directly in this
            // POC; the master key is what gates its own API. Use the
            // master key as both the "dummy" and "real" so the report
            // reads consistently. (It is *not* a provider key.)
            dummy_key_var: "LITELLM_MASTER_KEY".to_string(),
            real_key_var: "LITELLM_MASTER_KEY".to_string(),
            base_var: "FAKE_PROVIDER_API_BASE".to_string(),
            // In POC mode the LiteLLM base is the egress proxy.
            expected_base_host: Some("127.0.0.1:8082".to_string()),
            dummy_must_be_real: true, // the master key really is a real key
        },
        Provider {
            name: "fake_provider".to_string(),
            dummy_key_var: "FAKE_PROVIDER_DUMMY_KEY".to_string(),
            real_key_var: "FAKE_PROVIDER_REAL_KEY".to_string(),
            base_var: "FAKE_PROVIDER_API_BASE".to_string(),
            expected_base_host: Some("127.0.0.1:8082".to_string()),
            dummy_must_be_real: false, // dummy SHOULD be a placeholder
        },
    ];

    let mut all_ok = true;
    let mut rows: Vec<Row> = Vec::new();
    for p in &providers {
        let dummy = map.get(&p.dummy_key_var).cloned();
        let real = map.get(&p.real_key_var).cloned();
        let base = map.get(&p.base_var).cloned();
        let (base_host, base_host_ok) = match &base {
            Some(b) => match host_of(b) {
                Some(h) => match &p.expected_base_host {
                    Some(exp) if h != *exp => (Some(h), false),
                    _ => (Some(h), true),
                },
                None => (None, false),
            },
            None => (None, false),
        };
        let dummy_ok = match (p.dummy_must_be_real, &dummy) {
            (true, Some(v)) => !v.is_empty() && !is_placeholder(v),
            (true, None) => false,
            (false, Some(v)) => !v.is_empty(),
            (false, None) => false,
        };
        let real_ok = real.as_ref().is_some_and(|v| !v.is_empty() && !is_placeholder(v));
        let base_ok = base.is_some() && base_host_ok;

        let status = if dummy_ok && real_ok && base_ok { "OK" } else { "FAIL" };
        if status == "FAIL" {
            all_ok = false;
        }
        let bh_status = base_host_status(base_host.as_deref(), base_host_ok).to_string();
        rows.push(Row {
            provider: p.name.clone(),
            dummy_key_var: p.dummy_key_var.clone(),
            dummy_key_status: present(dummy.as_deref(), dummy_ok),
            real_key_var: p.real_key_var.clone(),
            real_key_status: present(real.as_deref(), real_ok),
            base_var: p.base_var.clone(),
            base_value: base.clone(),
            base_host,
            base_host_status: bh_status,
            expected_base_host: p.expected_base_host.clone(),
            status: status.to_string(),
        });
    }

    print_rows(&rows);
    println!();
    println!("env file: {}", env_path.display());
    if all_ok {
        println!("providers check: OK");
        Ok(0)
    } else {
        println!("providers check: FAIL");
        Ok(1)
    }
}

struct Row {
    provider: String,
    dummy_key_var: String,
    dummy_key_status: String,
    real_key_var: String,
    real_key_status: String,
    base_var: String,
    base_value: Option<String>,
    base_host: Option<String>,
    base_host_status: String,
    expected_base_host: Option<String>,
    status: String,
}

fn print_rows(rows: &[Row]) {
    for r in rows {
        println!("== provider: {} [{}]", r.provider, r.status);
        println!(
            "   dummy key: ${} = {}",
            r.dummy_key_var, r.dummy_key_status
        );
        println!(
            "   real key:  ${} = {}",
            r.real_key_var, r.real_key_status
        );
        println!(
            "   base:      ${} = {}",
            r.base_var,
            r.base_value.as_deref().unwrap_or("(unset)")
        );
        if let Some(h) = &r.base_host {
            let expected = r
                .expected_base_host
                .as_deref()
                .map(|e| format!(" (expected {e})"))
                .unwrap_or_default();
            println!("     host:    {h}{expected}  {}", r.base_host_status);
        }
    }
}

fn present(v: Option<&str>, ok: bool) -> String {
    match v {
        None => "(unset)".to_string(),
        Some(s) if s.is_empty() => "(empty)".to_string(),
        Some(s) if !ok => format!("PLACEHOLDER  ({})", redact(s)),
        Some(s) => format!("set ({})", redact(s)),
    }
}

fn base_host_status(_h: Option<&str>, ok: bool) -> &'static str {
    if ok {
        "[OK]"
    } else {
        "[FAIL]"
    }
}

fn is_placeholder(s: &str) -> bool {
    let u = s.to_ascii_uppercase();
    u.contains("CHANGE-ME")
        || u.contains("DO-NOT-LEAK")
        || u.contains("REPLACE-ME")
        || u.contains("EXAMPLE")
        || s == "dummy-key-injected-at-egress"
}

/// Strip the middle of a key, leaving the prefix and a few trailing chars.
fn redact(s: &str) -> String {
    let len = s.chars().count();
    if len <= 8 {
        return "<redacted>".to_string();
    }
    let prefix: String = s.chars().take(4).collect();
    let suffix: String = s.chars().rev().take(4).collect::<String>().chars().rev().collect();
    format!("{prefix}…{suffix}")
}

/// Extract the host:port (or host) portion of a URL.
fn host_of(url: &str) -> Option<String> {
    let after = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    let host = after.split('/').next().unwrap_or(after);
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

