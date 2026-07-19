//! Standing CI guard for `docs/migration/20-target-system-spec.md` (WP4 / D1).
//!
//! Extracts every fenced `toml` code block from the spec and asserts that each
//! non-fragment block deserializes into the `ConfigFile` schema shape. This
//! permanently closes the D1 class of drift (spec examples that do not parse
//! against the implementation, e.g. the `rw` vs `read_only` field-name split).
//!
//! Skip policy (a block is skipped if ANY of these hold):
//!   1. It carries a leading `# spec-test: skip` marker comment, OR
//!   2. It does not declare `schema_version` (fragments / registry files /
//!      override-layer files / single-feature snippets).
//!
//! A block that declares `schema_version` and is NOT marked skip MUST parse
//! cleanly into the schema-mirroring `ConfigFile` defined below. The mirror
//! intentionally makes `MountPlan.read_only` a required `bool` (no
//! `#[serde(default)]`) so that any regression to the legacy `rw` field name
//! fails deserialization with "missing field `read_only`".
//!
//! NOTE: the agentctl crate is a binary-only crate (no `[lib]` target), so the
//! real `ConfigFile` in `src/config.rs` is not importable from this integration
//! test. The mirror below is field-for-field compatible with the real schema;
//! promoting it to the crate type requires adding a lib target (out of WP4
//! scope). The mirror + the explicit `read_only`-required invariant is
//! sufficient to guard D1.

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Mirror of `recipes::EgressRecipeRef` — tagged enum on `recipe`.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "recipe", rename_all = "snake_case")]
enum EgressRecipeRef {
    Dns,
    LitellmProxy,
    Github,
    AgentBase,
    Https { hosts: Vec<String> },
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Scope {
    Local,
    Public,
}

#[derive(Debug, Deserialize, PartialEq)]
struct DenyDomainRule {
    domain_suffix: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct IngressRule {
    protocol: Protocol,
    port: u16,
    scope: Scope,
}

#[derive(Debug, Deserialize, PartialEq)]
struct PortMapping {
    host: u16,
    guest: u16,
}

/// Mirror of `plan::MountPlan`. `read_only` is REQUIRED (no default) so a
/// regression to `rw = ...` fails deserialization.
#[derive(Debug, Deserialize, PartialEq)]
struct MountPlan {
    host: String,
    guest: String,
    read_only: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
struct EnvVarConfig {
    name: String,
    value: Option<String>,
    secret: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct SecretEnvConfig {
    secret: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct SeedFileConfig {
    source: String,
    target: String,
    only_if_missing: Option<bool>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct LocalBuildConfig {
    recipe: String,
    source: String,
    requirements_file: Option<String>,
    target: Option<String>,
    gating_file: Option<String>,
    env_override: Option<String>,
    fallback: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct BakedFileSpec {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct BinarySpec {
    recipe: String,
    src: String,
    entrypoint: Option<String>,
    worker: Option<String>,
    npm_deps_hash: Option<String>,
    install_layout: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Default)]
struct ImageSpec {
    recipe: String,
    #[serde(rename = "ref")]
    reference: Option<String>,
    name: Option<String>,
    tag: Option<String>,
    contents: Option<Vec<String>>,
    binary: Option<BinarySpec>,
    baked_files: Option<Vec<BakedFileSpec>>,
    features: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, PartialEq, Default)]
struct NetworkConfig {
    default_deny: Option<bool>,
    #[serde(default)]
    egress: Vec<EgressRecipeRef>,
    #[serde(default)]
    deny: Vec<DenyDomainRule>,
    #[serde(default)]
    ingress: Vec<IngressRule>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct SecretDefConfig {
    env_var: Option<String>,
    hosts: Option<Vec<String>>,
    required: Option<bool>,
    placeholder: Option<String>,
    source: Option<String>,
    exposed_as: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct WorkloadConfig {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    image: ImageSpec,
    workdir: Option<String>,
    cpus: Option<u8>,
    memory_mib: Option<u32>,
    #[serde(default)]
    command: Vec<String>,
    log_stop_errors: Option<bool>,
    #[serde(default)]
    env: Vec<EnvVarConfig>,
    #[serde(default)]
    secret_env: Vec<SecretEnvConfig>,
    #[serde(default)]
    ports: Vec<PortMapping>,
    #[serde(default)]
    mounts: Vec<MountPlan>,
    #[serde(default)]
    seed_files: Vec<SeedFileConfig>,
    local_build: Option<LocalBuildConfig>,
    #[serde(default)]
    network: NetworkConfig,
}

/// Mirror of `config::ConfigFile`.
#[derive(Debug, Deserialize, PartialEq)]
struct ConfigFile {
    #[serde(default)]
    schema_version: u32,
    #[serde(default)]
    secrets: HashMap<String, SecretDefConfig>,
    #[serde(default)]
    workloads: HashMap<String, WorkloadConfig>,
}

/// A fenced code block extracted from the spec.
struct CodeBlock {
    lang: String,
    start_line: usize, // 1-indexed line of the opening fence
    body: String,
}

/// Extract all fenced code blocks from the markdown source, preserving the
/// 1-indexed line number of each opening fence.
fn extract_fenced_blocks(md: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut lines = md.lines().enumerate().peekable();
    while let Some((idx, line)) = lines.next() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("```") {
            continue;
        }
        let lang = trimmed.trim_start_matches('`').trim().to_string();
        let mut body = String::new();
        let mut closed = false;
        for (body_idx, body_line) in lines.by_ref() {
            if body_line.trim_start().starts_with("```") {
                closed = true;
                let _ = body_idx;
                break;
            }
            body.push_str(body_line);
            body.push('\n');
        }
        if closed {
            blocks.push(CodeBlock {
                lang,
                start_line: idx + 1,
                body,
            });
        }
    }
    blocks
}

fn spec_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("migration")
        .join("20-target-system-spec.md")
}

/// True if the block is marked as an intentional fragment.
fn is_marked_skip(body: &str) -> bool {
    body.lines()
        .take(5)
        .any(|l| l.trim() == "# spec-test: skip")
}

/// True if the block declares `schema_version` (heuristic: a full config).
fn declares_schema_version(body: &str) -> bool {
    body.lines()
        .any(|l| l.trim_start().starts_with("schema_version"))
}

#[test]
fn spec_toml_blocks_parse_against_schema() -> Result<(), Box<dyn std::error::Error>> {
    let path = spec_path();
    let md = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read spec at {}: {e}", path.display()))?;

    let blocks = extract_fenced_blocks(&md);
    assert!(
        !blocks.is_empty(),
        "no fenced code blocks found in spec at {}",
        path.display()
    );

    let toml_blocks: Vec<&CodeBlock> = blocks
        .iter()
        .filter(|b| b.lang.eq_ignore_ascii_case("toml"))
        .collect();
    assert!(
        toml_blocks.len() >= 2,
        "expected multiple toml blocks in spec, found {}",
        toml_blocks.len()
    );

    let mut parsed = 0usize;
    let mut skipped = 0usize;
    for block in &toml_blocks {
        if is_marked_skip(&block.body) || !declares_schema_version(&block.body) {
            skipped += 1;
            continue;
        }
        let parsed_value: toml::Value = toml::from_str(&block.body).map_err(|e| {
            format!(
                "spec toml block at line {} is not valid TOML: {e}",
                block.start_line
            )
        })?;
        let _: ConfigFile = parsed_value.try_into().map_err(|e| {
            format!(
                "spec toml block at line {} failed to deserialize into ConfigFile: {e}",
                block.start_line
            )
        })?;
        parsed += 1;
    }
    assert!(
        parsed >= 1,
        "no spec toml block was parsed; the complete annotated example must be present and parse"
    );
    // Sanity: at least the six known fragment blocks are skipped.
    assert!(
        skipped >= 6,
        "expected at least 6 skipped fragment blocks, skipped {skipped}"
    );
    Ok(())
}

/// Direct D1 polarity guard: no `[[workloads.*.mounts]]` table anywhere in the
/// spec may declare the legacy `rw` field. This is a belt-and-suspenders check
/// that fails loudly even if the schema-mirror deserialization were weakened.
#[test]
fn spec_mounts_never_use_legacy_rw_field() -> Result<(), Box<dyn std::error::Error>> {
    let path = spec_path();
    let md = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read spec at {}: {e}", path.display()))?;
    let blocks = extract_fenced_blocks(&md);
    let mut offenders: Vec<(usize, String)> = Vec::new();
    for block in blocks
        .iter()
        .filter(|b| b.lang.eq_ignore_ascii_case("toml"))
    {
        for (offset, line) in block.body.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("rw ") || t.starts_with("rw=") || t == "rw" {
                offenders.push((block.start_line + offset, line.to_string()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "spec still uses legacy `rw` mount field (use `read_only` with flipped \
         polarity): {offenders:?}"
    );
    Ok(())
}
