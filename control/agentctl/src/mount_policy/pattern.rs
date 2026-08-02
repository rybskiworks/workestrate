//! The mount-policy glob pattern dialect (spec 22 §6).
//!
//! [`Pattern`] is a newtype wrapping [`GlobMatcher`] that preserves the raw
//! authored string — diagnostics and the exact-duplicate conflict check of
//! spec 22 §4 need it — plus the dir-only flag (a trailing `/` in the raw
//! pattern).
//!
//! Dialect (spec 22 §6):
//!
//! - `*` and `?` never cross `/` (`literal_separator(true)`); `**` crosses
//!   directory boundaries.
//! - Patterns are root-anchored by default: they match from the mount root.
//!   A leading `**/` makes a pattern floating (match at any depth).
//! - A trailing `/` marks the pattern dir-only (`.git/` matches the
//!   directory, not a file named `.git`).
//! - Rejected at construction: absolute patterns, NUL bytes, `..`-escaping
//!   patterns, empty patterns. [`Pattern::compile`] names the origin (layer,
//!   file, scope kind) in its errors; [`Pattern::parse`] is the origin-free
//!   form used where no origin is known (e.g. program deserialization).

use crate::mount_policy::rule::RuleOrigin;
use globset::{GlobBuilder, GlobMatcher};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A compiled mount-policy glob pattern (spec 22 §6).
#[derive(Debug, Clone)]
pub struct Pattern {
    /// The raw authored string, preserved verbatim (spec 22 §6: diagnostics
    /// and the §4 exact-duplicate check compare raw strings).
    raw: String,
    matcher: GlobMatcher,
    /// Stem matcher for trailing-`/**` patterns: globset's `a/**` does not
    /// match `a` itself, but masking `a/**` must mask the directory `a`
    /// (spec 22 §7's traversal-only analysis depends on the masked stem).
    /// The stem (`a`) is compiled as a second matcher consulted alongside
    /// the main one.
    stem: Option<GlobMatcher>,
    /// True when the raw pattern ended in `/` (dir-only, spec 22 §6).
    dir_only: bool,
}

impl Pattern {
    /// Compile a raw pattern, naming `origin` in any rejection (spec 22 §6:
    /// pattern errors are reported against the layer/file/scope that
    /// declared them).
    pub fn compile(raw: &str, origin: &RuleOrigin) -> Result<Self, PatternError> {
        Self::compile_inner(raw).map_err(|kind| PatternError {
            kind,
            origin: Some(origin.clone()),
        })
    }

    /// Compile a raw pattern without provenance context. Used where no
    /// declaring origin exists (deserializing a compiled program, the
    /// compact `PolicyValue<Pattern>` form). Prefer [`Pattern::compile`]
    /// when an origin is known.
    pub fn parse(raw: &str) -> Result<Self, PatternError> {
        Self::compile_inner(raw).map_err(|kind| PatternError { kind, origin: None })
    }

    fn compile_inner(raw: &str) -> Result<Self, PatternErrorKind> {
        if raw.is_empty() {
            return Err(PatternErrorKind::Empty);
        }
        if raw.contains('\0') {
            return Err(PatternErrorKind::NulByte(raw.to_string()));
        }
        if raw.starts_with('/') {
            return Err(PatternErrorKind::Absolute(raw.to_string()));
        }
        let dir_only = raw.ends_with('/');
        let body = if dir_only { &raw[..raw.len() - 1] } else { raw };
        if body.is_empty() {
            // Only reachable for raw == "/", which is already rejected as
            // absolute; kept as a defense-in-depth guard.
            return Err(PatternErrorKind::Empty);
        }
        if body.split('/').any(|component| component == "..") {
            return Err(PatternErrorKind::ParentEscape(raw.to_string()));
        }
        let matcher = GlobBuilder::new(body)
            .literal_separator(true)
            .build()
            .map_err(|source| PatternErrorKind::InvalidGlob {
                pattern: raw.to_string(),
                message: source.to_string(),
            })?
            .compile_matcher();
        let stem = body
            .strip_suffix("/**")
            .filter(|stem| !stem.is_empty())
            .map(|stem| {
                GlobBuilder::new(stem)
                    .literal_separator(true)
                    .build()
                    .map(|glob| glob.compile_matcher())
                    .map_err(|source| PatternErrorKind::InvalidGlob {
                        pattern: raw.to_string(),
                        message: source.to_string(),
                    })
            })
            .transpose()?;
        Ok(Self {
            raw: raw.to_string(),
            matcher,
            stem,
            dir_only,
        })
    }

    /// The raw authored pattern string, verbatim.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// True when the pattern is dir-only (trailing `/`, spec 22 §6).
    pub fn is_dir_only(&self) -> bool {
        self.dir_only
    }

    /// Match a path known to be a non-directory. A dir-only pattern never
    /// matches the file itself but still matches files UNDER a matching
    /// directory (a masked directory masks its subtree).
    pub fn matches_path(&self, path: &str) -> bool {
        (!self.dir_only && self.is_match(path)) || self.matches_any_ancestor(path)
    }

    /// Match a path known to be a directory. Dir-only and ordinary patterns
    /// alike match the directory itself; a matching ancestor directory also
    /// matches (a masked directory masks its subtree).
    pub fn matches_dir(&self, dir: &str) -> bool {
        self.is_match(dir) || self.matches_any_ancestor(dir)
    }

    /// Fail-closed union of [`Pattern::matches_path`] and
    /// [`Pattern::matches_dir`], used by the pure evaluator when the path's
    /// type is unknown: a dir-only pattern matching the path exactly counts
    /// as a match (the conservative direction; the type-aware runtime uses
    /// the two helpers above instead).
    pub(crate) fn matches_unknown(&self, path: &str) -> bool {
        self.is_match(path) || self.matches_any_ancestor(path)
    }

    /// The main matcher plus the trailing-`/**` stem matcher.
    fn is_match(&self, path: &str) -> bool {
        self.matcher.is_match(path) || self.stem.as_ref().is_some_and(|stem| stem.is_match(path))
    }

    /// A masked (or unmasked) directory applies to its whole subtree: the
    /// pattern matches when any proper ancestor of `path` matches.
    fn matches_any_ancestor(&self, path: &str) -> bool {
        let mut rest = path;
        while let Some(idx) = rest.rfind('/') {
            rest = &rest[..idx];
            if self.is_match(rest) {
                return true;
            }
        }
        false
    }

    /// The fixed literal prefix of the pattern (spec 22 §7 literal-prefix
    /// analysis), or `None` when the pattern has no fixed literal prefix (a
    /// leading `**`, or a glob metacharacter in the first component) —
    /// callers fall back to a conservative answer for such patterns.
    pub fn literal_prefix(&self) -> Option<LiteralPrefix> {
        let body = self.raw.strip_suffix('/').unwrap_or(&self.raw);
        let total_components = body.split('/').count();
        let mut prefix = Vec::new();
        for component in body.split('/') {
            if component == "**" || component.contains(['*', '?', '[', ']', '{', '}']) {
                break;
            }
            prefix.push(component.to_string());
        }
        if prefix.is_empty() {
            None
        } else {
            Some(LiteralPrefix {
                extends_below: prefix.len() < total_components,
                components: prefix,
            })
        }
    }
}

/// The fixed literal leading components of a pattern, plus whether the
/// pattern reaches BELOW that prefix (spec 22 §7): `private/public/**` has
/// prefix `[private, public]` and extends below it; `docs/secrets/README.md`
/// is exactly its own prefix and does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralPrefix {
    components: Vec<String>,
    extends_below: bool,
}

impl LiteralPrefix {
    /// The fixed literal leading components.
    pub fn components(&self) -> &[String] {
        &self.components
    }

    /// True when the pattern can match paths strictly below its literal
    /// prefix (e.g. a trailing `/**`).
    pub fn extends_below(&self) -> bool {
        self.extends_below
    }
}

impl PartialEq for Pattern {
    /// Patterns compare by their raw authored string (spec 22 §4's
    /// exact-duplicate check compares raw strings).
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl Eq for Pattern {}

impl Serialize for Pattern {
    /// The program wire form of a pattern is its raw string (spec 22 §12);
    /// the matcher is recompiled on load.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.raw)
    }
}

impl<'de> Deserialize<'de> for Pattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Pattern::parse(&raw).map_err(de::Error::custom)
    }
}

/// Why a pattern was rejected (spec 22 §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternErrorKind {
    /// The empty pattern matches nothing and names no intent.
    Empty,
    /// Leading `/`: patterns are relative to the mount root.
    Absolute(String),
    /// Patterns must be valid text.
    NulByte(String),
    /// A `..` component could resolve outside the mount root.
    ParentEscape(String),
    /// The globset dialect rejected the pattern.
    InvalidGlob {
        /// The offending raw pattern.
        pattern: String,
        /// The glob compiler's own message.
        message: String,
    },
}

impl fmt::Display for PatternErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PatternErrorKind::Empty => write!(
                f,
                "mount policy pattern is empty: a pattern must name at least one path component"
            ),
            PatternErrorKind::Absolute(pattern) => write!(
                f,
                "mount policy pattern '{pattern}' is absolute: patterns are relative to the \
                 mount root (drop the leading '/')"
            ),
            PatternErrorKind::NulByte(pattern) => write!(
                f,
                "mount policy pattern {pattern:?} contains a NUL byte: patterns must be valid text"
            ),
            PatternErrorKind::ParentEscape(pattern) => write!(
                f,
                "mount policy pattern '{pattern}' contains a '..' component: patterns must not \
                 escape the mount root"
            ),
            PatternErrorKind::InvalidGlob { pattern, message } => {
                write!(
                    f,
                    "mount policy pattern '{pattern}' is not a valid glob: {message}"
                )
            }
        }
    }
}

/// A pattern rejection, optionally carrying the declaring origin (spec 22
/// §6: errors name the origin; origin-free forms come from [`Pattern::parse`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternError {
    kind: PatternErrorKind,
    origin: Option<RuleOrigin>,
}

impl PatternError {
    /// The rejection reason.
    pub fn kind(&self) -> &PatternErrorKind {
        &self.kind
    }
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.origin {
            Some(origin) => write!(f, "{} [declared at {}]", self.kind, origin),
            None => write!(f, "{}", self.kind),
        }
    }
}

impl std::error::Error for PatternError {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::mount_policy::scope::ScopeKind;
    use std::path::PathBuf;

    fn origin() -> RuleOrigin {
        RuleOrigin {
            layer: "personal".to_string(),
            file: PathBuf::from("config.toml"),
            scope_kind: ScopeKind::HomeRegistry,
        }
    }

    fn compile(raw: &str) -> Pattern {
        Pattern::compile(raw, &origin()).unwrap()
    }

    #[test]
    fn anchored_patterns_match_from_the_mount_root_only() {
        let pattern = compile("x");
        assert!(pattern.matches_path("x"));
        assert!(!pattern.matches_path("a/x"));
        assert!(!pattern.matches_path("a/b/x"));
    }

    #[test]
    fn floating_patterns_match_at_any_depth() {
        let pattern = compile("**/x");
        assert!(pattern.matches_path("x"));
        assert!(pattern.matches_path("a/x"));
        assert!(pattern.matches_path("a/b/x"));
        // A path UNDER a match counts (subtree semantics): `x/y` sits under
        // the matched `x`, so it is conservatively covered.
        assert!(pattern.matches_path("x/y"));
    }

    #[test]
    fn trailing_double_star_also_matches_its_stem_directory() {
        // globset's `a/**` does not match `a` itself, but masking `a/**`
        // must mask the directory `a` (spec 22 §7 traversal-only).
        let pattern = compile("docs/secrets/**");
        assert!(pattern.matches_dir("docs/secrets"));
        assert!(pattern.matches_unknown("docs/secrets"));
        // Non-stem directories do not match.
        assert!(!pattern.matches_dir("docs"));
        assert!(!pattern.matches_unknown("docs/public"));
    }

    #[test]
    fn star_and_question_never_cross_slash() {
        let star = compile("*.pem");
        assert!(star.matches_path("a.pem"));
        assert!(!star.matches_path("certs/a.pem"));
        let question = compile("a?c");
        assert!(question.matches_path("abc"));
        assert!(!question.matches_path("a/c"));
    }

    #[test]
    fn double_star_crosses_directories() {
        let pattern = compile("a/**/c");
        assert!(pattern.matches_path("a/b/c"));
        assert!(pattern.matches_path("a/b/d/c"));
        assert!(!pattern.matches_path("b/a/c"));
    }

    #[test]
    fn masked_directory_masks_its_subtree() {
        let pattern = compile("docs/secrets/**");
        assert!(pattern.matches_path("docs/secrets/a.md"));
        assert!(pattern.matches_path("docs/secrets/deep/a.md"));
        assert!(!pattern.matches_path("docs/public/a.md"));
    }

    #[test]
    fn trailing_slash_marks_dir_only() {
        let pattern = compile(".git/");
        assert!(pattern.is_dir_only());
        // The directory itself matches under dir semantics…
        assert!(pattern.matches_dir(".git"));
        // …but a file of the same name does not…
        assert!(!pattern.matches_path(".git"));
        // …while everything under the directory does.
        assert!(pattern.matches_path(".git/config"));
        assert!(pattern.matches_dir(".git/hooks"));
    }

    #[test]
    fn rejects_absolute_nul_parent_escape_and_empty() {
        let cases = ["/etc/passwd", "a\0b", "a/../b", "..", ""];
        for raw in cases {
            assert!(
                Pattern::compile(raw, &origin()).is_err(),
                "pattern {raw:?} must be rejected"
            );
        }
    }

    #[test]
    fn rejection_errors_name_the_origin() {
        let err = Pattern::compile("/etc", &origin()).unwrap_err();
        let text = err.to_string();
        assert!(
            text.contains("personal"),
            "error must name the layer: {text}"
        );
        assert!(
            text.contains("config.toml"),
            "error must name the file: {text}"
        );
        let err = Pattern::compile("../escape", &origin()).unwrap_err();
        assert_eq!(
            err.kind(),
            &PatternErrorKind::ParentEscape("../escape".to_string())
        );
    }

    #[test]
    fn parse_is_the_origin_free_form() {
        let err = Pattern::parse("/abs").unwrap_err();
        assert!(!err.to_string().contains("personal"));
        assert!(Pattern::parse(".env").is_ok());
    }

    #[test]
    fn literal_prefix_stops_at_the_first_glob_component() {
        let prefix = compile("private/public/**").literal_prefix().unwrap();
        assert_eq!(prefix.components(), &["private", "public"]);
        assert!(prefix.extends_below());
        let prefix = compile("docs/secrets/README.md").literal_prefix().unwrap();
        assert_eq!(prefix.components(), &["docs", "secrets", "README.md"]);
        assert!(!prefix.extends_below());
        // No fixed literal prefix: callers fall back to conservative true.
        assert_eq!(compile("**/x").literal_prefix(), None);
        assert_eq!(compile("*.pem").literal_prefix(), None);
    }

    #[test]
    fn serde_round_trip_uses_the_raw_string() {
        let pattern = compile(".git/");
        let json = serde_json::to_string(&pattern).unwrap();
        assert_eq!(json, "\".git/\"");
        let back: Pattern = serde_json::from_str(&json).unwrap();
        assert_eq!(back, pattern);
        assert!(back.is_dir_only());
        assert!(serde_json::from_str::<Pattern>("\"/abs\"").is_err());
    }
}
