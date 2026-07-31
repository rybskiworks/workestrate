//! Standing CI guard for `docs/migration/20-target-system-spec.md` (WP4 / D1).
//!
//! Extracts every fenced `toml` code block from the spec and asserts that each
//! non-fragment block deserializes into the real `workestrate::config::ConfigFile`
//! schema. This permanently closes the D1 class of drift (spec examples that do
//! not parse against the implementation, e.g. the `rw` vs `read_only`
//! field-name split).
//!
//! Skip policy (a block is skipped if ANY of these hold):
//!   1. It carries a leading `# spec-test: skip` marker comment, OR
//!   2. It does not declare `schema_version` (fragments / registry files /
//!      override-layer files / single-feature snippets).
//!
//! A block that declares `schema_version` and is NOT marked skip MUST parse
//! cleanly into the real `ConfigFile`. The real `MountPlan.read_only` is a
//! required `bool` (no `#[serde(default)]`), so any regression to the legacy
//! `rw` field name fails deserialization with "missing field `read_only`".
//!
//! NOTE: the crate has had a lib target (`workestrate`) since 45a42fb, so this
//! test now imports the real `workestrate::config::ConfigFile`; the former
//! local schema mirror is retired (a mirror can drift from the real schema
//! silently). Consequence: the real types carry
//! `#[serde(deny_unknown_fields)]` everywhere (the mirror did not), so any
//! spec example that relied on unknown-field tolerance will now FAIL — that
//! is intentional and net-positive: spec examples must match the shipped
//! schema exactly.

use std::fs;
use std::path::Path;
use workestrate::config::ConfigFile;

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
