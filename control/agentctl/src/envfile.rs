//! Minimal `.env` file loader.
//!
//! This is *not* a general-purpose parser. It handles the subset that
//! the POC's `infra/litellm/.env` actually uses:
//!
//!   KEY=value           # assigned
//!   KEY="quoted value"  # double-quoted (with `\\` and `\"` escapes)
//!   KEY='quoted value'  # single-quoted (literal, no escapes)
//!   # comment lines and blank lines are skipped
//!
//! Values are returned as a `Vec<(String, String)>` to preserve order
//! (handy for the print-plan command, which prints the file in the
//! order it was loaded). We do *not* deduplicate.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub value: String,
}

pub fn load(path: &Path) -> Result<Vec<Entry>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("could not read {}: {}", path.display(), e))?;
    let mut out = Vec::new();
    for (lineno, line) in raw.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Optional `export ` prefix.
        let stripped = trimmed
            .strip_prefix("export ")
            .unwrap_or(trimmed)
            .trim_start();
        let (k, v) = stripped
            .split_once('=')
            .ok_or_else(|| format!("{}:{}: expected KEY=VALUE", path.display(), lineno + 1))?;
        let key = k.trim().to_string();
        if key.is_empty() {
            return Err(format!("{}:{}: empty key", path.display(), lineno + 1));
        }
        let value = parse_value(v.trim_start());
        out.push(Entry { key, value });
    }
    Ok(out)
}

fn parse_value(raw: &str) -> String {
    let raw = raw.trim();
    if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
        let inner = &raw[1..raw.len() - 1];
        return unescape_double(inner);
    }
    if raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2 {
        // Single quotes are literal in dotenv.
        return raw[1..raw.len() - 1].to_string();
    }
    // Bare value — strip a trailing inline comment.
    if let Some(idx) = raw.find(" #") {
        return raw[..idx].trim_end().to_string();
    }
    raw.to_string()
}

fn unescape_double(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_value_bare() {
        assert_eq!(parse_value("hello"), "hello");
        assert_eq!(parse_value("  hello  "), "hello");
    }

    #[test]
    fn parse_value_double_quoted() {
        assert_eq!(parse_value("\"hello\""), "hello");
        assert_eq!(parse_value("\"hello world\""), "hello world");
        assert_eq!(parse_value("\"with \\\"quote\\\"\""), "with \"quote\"");
    }

    #[test]
    fn parse_value_single_quoted() {
        assert_eq!(parse_value("'hello'"), "hello");
        assert_eq!(parse_value("'hello world'"), "hello world");
    }

    #[test]
    fn parse_value_inline_comment() {
        assert_eq!(parse_value("a # b"), "a");
        assert_eq!(parse_value("a#b"), "a#b");
    }

    #[test]
    fn loads_simple_file() {
        let dir = std::env::temp_dir();
        let p = dir.join("agentctl-envfile-test.env");
        std::fs::write(&p, "A=1\nB=\"two words\"\nC='three'\n# comment\n\nD=trailing # nope\n").unwrap();
        let entries = load(&p).unwrap();
        let map: std::collections::BTreeMap<_, _> = entries.iter().map(|e| (e.key.clone(), e.value.clone())).collect();
        assert_eq!(map.get("A").map(|s| s.as_str()), Some("1"));
        assert_eq!(map.get("B").map(|s| s.as_str()), Some("two words"));
        assert_eq!(map.get("C").map(|s| s.as_str()), Some("three"));
        assert_eq!(map.get("D").map(|s| s.as_str()), Some("trailing"));
        std::fs::remove_file(&p).ok();
    }
}
