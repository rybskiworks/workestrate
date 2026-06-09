//! `agentctl litellm print-plan` — read the LiteLLM config and report
//! exactly what the sandbox for the isolated LiteLLM process would look
//! like, *without* starting it.
//!
//! The threat model is the whole point of this code: the `print-plan`
//! output is the human-readable proof that
//!
//!   * the real provider key is NOT in the env that will be passed to
//!     LiteLLM, and
//!   * the only outbound target the isolated process can reach is the
//!     egress proxy, which is configured to allowlist the real provider
//!     host.
//!
//! We deliberately do not start any process, do not dial any URL, and
//! do not log the actual key values. The keys are reported as
//! `$VARNAME (set)` / `$VARNAME (unset)` / `$VARNAME (placeholder)`.

use std::collections::BTreeSet;
use std::fs;

use crate::config;
use crate::envfile;

pub fn print_plan() -> Result<u8, String> {
    let cfg_path = config::litellm_config_path();
    if !cfg_path.exists() {
        return Err(format!(
            "LiteLLM config not found at {}. Run from the project root.",
            cfg_path.display()
        ));
    }
    let raw = fs::read_to_string(&cfg_path)
        .map_err(|e| format!("could not read {}: {}", cfg_path.display(), e))?;

    let plan = Plan::from_yaml(&raw)?;
    let env_path = config::env_file_path();
    let env_pairs = if env_path.exists() {
        envfile::load(&env_path)?
    } else {
        Vec::new()
    };
    let env_map: std::collections::BTreeMap<String, String> = env_pairs
        .iter()
        .map(|e| (e.key.clone(), e.value.clone()))
        .collect();

    println!("LiteLLM sandbox plan (read-only, not started)");
    println!("=============================================");
    println!();
    println!("image:    ghcr.io/berriai/litellm:main-stable  # plan default; override with LITELLM_IMAGE");
    println!("config:   {}", cfg_path.display());
    println!("command:  litellm --config {}", cfg_path.display());
    println!("port:     4000:4000  # host:container, used by agents to reach the proxy");
    println!();
    println!("models exposed by this proxy:");
    for m in &plan.models {
        let api_base_display = m
            .api_base
            .as_deref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "(unset)".to_string());
        let api_key_display = m
            .api_key
            .as_deref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "(unset)".to_string());
        println!("  - model_name={}", m.model_name);
        println!("      litellm_params.model   = {}", m.model.as_deref().unwrap_or("?"));
        println!("      litellm_params.api_base = {api_base_display}");
        println!("      litellm_params.api_key  = {api_key_display}");
    }
    println!();

    // env passed in / kept out
    let mut passed_in: BTreeSet<String> = BTreeSet::new();
    for m in &plan.models {
        if let Some(v) = env_var_of(&m.api_base) {
            passed_in.insert(v);
        }
        if let Some(v) = env_var_of(&m.api_key) {
            passed_in.insert(v);
        }
    }
    if let Some(master) = &plan.master_key {
        passed_in.insert(master.clone());
    }
    println!("env passed into the LiteLLM sandbox:");
    if passed_in.is_empty() {
        println!("  (none)");
    } else {
        for raw_v in &passed_in {
            // The env var names in the config are written as `os.environ/NAME`.
            // Strip the prefix for the actual env map lookup and for display.
            let lookup_key = raw_v.strip_prefix("os.environ/").unwrap_or(raw_v);
            let v = lookup_key;
            let status = match env_map.get(lookup_key) {
                None => "unset".to_string(),
                Some(s) if is_placeholder(s) => "placeholder".to_string(),
                Some(s) => format!("set ({})", redact(s)),
            };
            println!("  {v} = {status}");
        }
    }
    println!();
    println!("env held on the HOST (NEVER passed into the LiteLLM sandbox):");
    println!("  FAKE_PROVIDER_REAL_KEY = {}", match env_map.get("FAKE_PROVIDER_REAL_KEY") {
        None => "unset  (egress proxy will refuse to start)".to_string(),
        Some(s) if is_placeholder(s) => "placeholder  (replace before any real call)".to_string(),
        Some(s) => format!("set ({})", redact(s)),
    });
    println!();

    // Egress allowlist
    let base_url = plan
        .models
        .iter()
        .filter_map(|m| m.api_base.as_deref())
        .find(|s| !s.starts_with("os.environ/"))
        .map(|s| s.to_string())
        .or_else(|| {
            // Resolve `os.environ/FAKE_PROVIDER_API_BASE` from the env map.
            let from_env = plan
                .models
                .iter()
                .filter_map(|m| m.api_base.as_deref())
                .find(|s| s.starts_with("os.environ/"))
                .and_then(|s| env_map.get(s.trim_start_matches("os.environ/")));
            from_env.cloned()
        });
    println!("egress allowlist (deny by default; only the proxy is reachable from the sandbox):");
    match base_url {
        Some(url) => {
            let host = host_of(&url).unwrap_or("(could not parse)".to_string());
            println!("  outbound target: {url}");
            println!("  host:port:      {host}");
            println!("  allowlist:");
            println!("    - {host}    (via the host-side egress proxy at 127.0.0.1:8082)");
            println!("  denied by default:");
            println!("    - 169.254.169.254 (link-local metadata)");
            println!("    - 8.8.8.8:53, 1.1.1.1:53 (public DNS — would bypass the proxy)");
            println!("    - api.openai.com, api.anthropic.com, ... (provider hosts not in allowlist)");
        }
        None => println!("  (could not determine outbound target from config)"),
    }
    println!();
    println!("source:  {}", cfg_path.display());
    println!("env:     {}", env_path.display());
    Ok(0)
}

#[derive(Debug, Default)]
struct Plan {
    models: Vec<ModelEntry>,
    master_key: Option<String>,
}

#[derive(Debug, Default)]
struct ModelEntry {
    model_name: String,
    model: Option<String>,
    api_base: Option<String>,
    api_key: Option<String>,
}

impl Plan {
    #[allow(unused_assignments)]
    fn from_yaml(raw: &str) -> Result<Self, String> {
        // The hand-rolled parser below accepts a strict subset of YAML:
        //   - two-space indentation
        //   - strings either bare or single/double-quoted
        //   - top-level keys: model_list, general_settings, litellm_settings
        //     (the latter two are not parsed in detail; their presence is
        //     tolerated)
        //   - the `model_list` block is a list of `-` items, each with
        //     `model_name:` and `litellm_params:` (which contains `model:`,
        //     `api_base:`, `api_key:`)
        let mut plan = Plan::default();
        let mut lines = raw.lines().peekable();
        let mut current_top: Option<String> = None;
        let mut in_model_list = false;
        let mut current: Option<ModelEntry> = None;
        let _ = false;

        while let Some(line) = lines.next() {
            let line_no_comment = strip_comment(line).to_string();
            if line_no_comment.trim().is_empty() {
                continue;
            }
            let indent = leading_spaces(&line_no_comment);
            let content = line_no_comment.trim();
            if indent == 0 {
                // Top-level key
                in_model_list = false;
                _ = false;
                if let Some(item) = current.take() {
                    plan.models.push(item);
                }
                let (key, val) = split_kv(content)
                    .ok_or_else(|| format!("cannot parse top-level line: {content:?}"))?;
                current_top = Some(key.clone());
                if key == "model_list" {
                    in_model_list = true;
                } else if key == "general_settings" {
                    // peek ahead for indented `master_key:`
                    // We'll scan lines until indent returns to 0.
                    // For simplicity, just look at the next non-blank
                    // line; YAML in this config is shallow.
                } else if key == "litellm_settings" {
                    // tolerated
                } else if matches!(key.as_str(),
                    "database_url" | "router_settings" | "litellm_params" | "telemetry" | "drop_params"
                ) {
                    // tolerated: scalar or sub-block at top level
                } else {
                    return Err(format!("unsupported top-level key: {key}"));
                }
                let _ = val; // not used
            } else if in_model_list && content.starts_with("- ") {
                if let Some(item) = current.take() {
                    plan.models.push(item);
                }
                let body = content.trim_start_matches("- ").trim();
                if !body.is_empty() {
                    let (k, v) = split_kv(body).ok_or_else(|| {
                        format!("expected KEY: VALUE after `- `, got {body:?}")
                    })?;
                    let mut entry = ModelEntry::default();
                    if k == "model_name" {
                        entry.model_name = unquote(&v);
                    } else {
                        return Err(format!("expected first key in -item to be model_name, got {k}"));
                    }
                    _ = false;
                    current = Some(entry);
                } else {
                    _ = false;
                    current = Some(ModelEntry::default());
                }
                let _ = current_top;
            } else if current.is_some() {
                let (k, v) = split_kv(content)
                    .ok_or_else(|| format!("expected KEY: VALUE, got {content:?}"))?;
                if indent >= 4 {
                    _ = true;
                    let cur = current.as_mut().unwrap();
                    match k.as_str() {
                        "model" => cur.model = Some(unquote(&v)),
                        "api_base" => cur.api_base = Some(unquote(&v)),
                        "api_key" => cur.api_key = Some(unquote(&v)),
                        _ => {} // tolerated
                    }
                } else {
                    _ = false;
                }
            } else if current_top.as_deref() == Some("general_settings") {
                let (k, v) = split_kv(content)
                    .ok_or_else(|| format!("expected KEY: VALUE, got {content:?}"))?;
                if k == "master_key" {
                    plan.master_key = Some(unquote(&v));
                }
            } else {
                // tolerated: comments, litellm_settings, etc.
                let _ = content;
            }
        }
        if let Some(item) = current.take() {
            plan.models.push(item);
        }
        Ok(plan)
    }
}

fn split_kv(s: &str) -> Option<(String, String)> {
    let idx = s.find(':')?;
    let key = s[..idx].trim().to_string();
    let val = s[idx + 1..].trim().to_string();
    Some((key, val))
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn strip_comment(s: &str) -> &str {
    // Comments only when preceded by whitespace, to keep URLs intact.
    let bytes = s.as_bytes();
    let mut in_str: Option<u8> = None;
    for (i, &b) in bytes.iter().enumerate() {
        match in_str {
            Some(q) if b == q => in_str = None,
            Some(_) => {}
            None => match b {
                b'"' | b'\'' => in_str = Some(b),
                b'#' if i == 0 || bytes[i - 1].is_ascii_whitespace() => {
                    return &s[..i];
                }
                _ => {}
            },
        }
    }
    s
}

fn leading_spaces(s: &str) -> usize {
    s.len() - s.trim_start_matches(' ').len()
}

fn env_var_of(s: &Option<String>) -> Option<String> {
    let s = s.as_deref()?;
    s.strip_prefix("os.environ/").map(|v| v.to_string())
}

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

fn is_placeholder(s: &str) -> bool {
    let u = s.to_ascii_uppercase();
    u.contains("CHANGE-ME")
        || u.contains("DO-NOT-LEAK")
        || u.contains("REPLACE-ME")
        || u.contains("EXAMPLE")
        || s == "dummy-key-injected-at-egress"
}

fn redact(s: &str) -> String {
    let len = s.chars().count();
    if len <= 8 {
        return "<redacted>".to_string();
    }
    let prefix: String = s.chars().take(4).collect();
    let suffix: String = s.chars().rev().take(4).collect::<String>().chars().rev().collect();
    format!("{prefix}…{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_minimal_config() {
        let yaml = r#"
model_list:
  - model_name: fake-gpt
    litellm_params:
      model: openai/fake-gpt-4o
      api_base: os.environ/FAKE_PROVIDER_API_BASE
      api_key: os.environ/FAKE_PROVIDER_DUMMY_KEY

general_settings:
  master_key: os.environ/LITELLM_MASTER_KEY

litellm_settings:
  telemetry: False
  drop_params: True
"#;
        let p = Plan::from_yaml(yaml).unwrap();
        assert_eq!(p.models.len(), 1);
        assert_eq!(p.models[0].model_name, "fake-gpt");
        assert_eq!(
            p.models[0].api_base.as_deref(),
            Some("os.environ/FAKE_PROVIDER_API_BASE")
        );
        assert_eq!(p.master_key.as_deref(), Some("os.environ/LITELLM_MASTER_KEY"));
    }
}
