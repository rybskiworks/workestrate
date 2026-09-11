//! Component-wise lexical mount-root-relative paths (spec 22 §6).
//!
//! [`LexicalPath`] is a pure path representation: no canonicalization, no
//! filesystem access. Construction rejects absolute paths and `..`
//! components; non-UTF-8 input is handled fail-closed — the path is marked
//! non-UTF-8 and the evaluator masks it (a path that cannot be represented
//! as UTF-8 cannot be matched against the pattern set, spec 22 §6).

use std::fmt;

/// A normalized, mount-root-relative path evaluated purely lexically.
///
/// Normalization drops empty segments (duplicate separators, trailing
/// slashes) and `.` components. The empty path (no components) is the mount
/// root itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalPath {
    /// The normalized relative path string (empty for the mount root).
    /// Meaningless when `non_utf8` is set.
    normalized: String,
    /// The normalized components, for prefix analysis (spec 22 §7).
    components: Vec<String>,
    /// Fail-closed marker (spec 22 §6): the original path was not valid
    /// UTF-8, so it cannot be matched and the evaluator masks it.
    non_utf8: bool,
}

impl LexicalPath {
    /// Build from a UTF-8 relative path. Rejects absolute paths (leading
    /// `/`) and any `..` component: policy paths must stay inside the mount
    /// root.
    pub fn new(path: &str) -> Result<Self, LexicalPathError> {
        if path.starts_with('/') {
            return Err(LexicalPathError::Absolute(path.to_string()));
        }
        let mut components = Vec::new();
        for component in path.split('/') {
            match component {
                "" | "." => {}
                ".." => return Err(LexicalPathError::ParentEscape(path.to_string())),
                other => components.push(other.to_string()),
            }
        }
        Ok(Self {
            normalized: components.join("/"),
            components,
            non_utf8: false,
        })
    }

    /// Build from raw bytes, fail-closed (spec 22 §6): valid UTF-8 behaves
    /// like [`LexicalPath::new`]; non-UTF-8 input yields a path marked
    /// non-UTF-8, which the evaluator masks.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LexicalPathError> {
        match std::str::from_utf8(bytes) {
            Ok(text) => Self::new(text),
            Err(_) => Ok(Self {
                normalized: String::new(),
                components: Vec::new(),
                non_utf8: true,
            }),
        }
    }

    /// True when the original path was not valid UTF-8 (fail-closed, spec 22
    /// §6). The evaluator masks such paths.
    pub fn is_non_utf8(&self) -> bool {
        self.non_utf8
    }

    /// True for the mount root itself (and not for a non-UTF-8 path).
    pub fn is_root(&self) -> bool {
        self.components.is_empty() && !self.non_utf8
    }

    /// The normalized relative path string, or `None` for non-UTF-8 paths.
    pub fn as_str(&self) -> Option<&str> {
        if self.non_utf8 {
            None
        } else {
            Some(&self.normalized)
        }
    }

    /// The normalized components, for prefix analysis (spec 22 §7).
    pub fn components(&self) -> &[String] {
        &self.components
    }

    /// `self` extended by one child name. The name must be a single path
    /// component that stays inside the mount root. A non-UTF-8 path stays
    /// non-UTF-8 (fail-closed propagates to children).
    pub fn child(&self, name: &str) -> Result<Self, LexicalPathError> {
        if self.non_utf8 {
            return Ok(self.clone());
        }
        if name.is_empty() || name == "." || name == ".." || name.contains('/') {
            return Err(LexicalPathError::InvalidChildName(name.to_string()));
        }
        let mut components = self.components.clone();
        components.push(name.to_string());
        Ok(Self {
            normalized: components.join("/"),
            components,
            non_utf8: false,
        })
    }
}

/// Why a lexical path was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexicalPathError {
    /// Leading `/`: policy paths are relative to the mount root.
    Absolute(String),
    /// A `..` component could resolve outside the mount root.
    ParentEscape(String),
    /// A child name must be exactly one path component.
    InvalidChildName(String),
}

impl fmt::Display for LexicalPathError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LexicalPathError::Absolute(path) => write!(
                f,
                "policy path '{path}' is absolute: paths are relative to the mount root"
            ),
            LexicalPathError::ParentEscape(path) => write!(
                f,
                "policy path '{path}' contains a '..' component: paths must not escape the \
                 mount root"
            ),
            LexicalPathError::InvalidChildName(name) => {
                write!(f, "child name '{name}' is not a single path component")
            }
        }
    }
}

impl std::error::Error for LexicalPathError {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_empty_and_dot_segments() {
        let path = LexicalPath::new("a/./b//c/").unwrap();
        assert_eq!(path.as_str(), Some("a/b/c"));
        assert_eq!(path.components(), &["a", "b", "c"]);
    }

    #[test]
    fn empty_path_is_the_mount_root() {
        let root = LexicalPath::new("").unwrap();
        assert!(root.is_root());
        assert_eq!(root.as_str(), Some(""));
        assert_eq!(root.components(), &[] as &[String]);
    }

    #[test]
    fn rejects_absolute_and_parent_escaping_paths() {
        assert_eq!(
            LexicalPath::new("/etc").unwrap_err(),
            LexicalPathError::Absolute("/etc".to_string())
        );
        assert_eq!(
            LexicalPath::new("a/../b").unwrap_err(),
            LexicalPathError::ParentEscape("a/../b".to_string())
        );
        assert!(LexicalPath::new("..").is_err());
    }

    #[test]
    fn non_utf8_bytes_are_marked_fail_closed() {
        let path = LexicalPath::from_bytes(b"\xff\xfe").unwrap();
        assert!(path.is_non_utf8());
        assert_eq!(path.as_str(), None);
        assert!(!path.is_root());
        // Fail-closed propagates to children.
        let child = path.child("x").unwrap();
        assert!(child.is_non_utf8());
    }

    #[test]
    fn valid_utf8_bytes_parse_normally() {
        let path = LexicalPath::from_bytes("a/b".as_bytes()).unwrap();
        assert!(!path.is_non_utf8());
        assert_eq!(path.as_str(), Some("a/b"));
    }

    #[test]
    fn child_extends_the_path_and_validates_the_name() {
        let dir = LexicalPath::new("a/b").unwrap();
        assert_eq!(dir.child("c").unwrap().as_str(), Some("a/b/c"));
        for bad in ["", ".", "..", "c/d"] {
            assert!(
                dir.child(bad).is_err(),
                "child name {bad:?} must be rejected"
            );
        }
    }
}
