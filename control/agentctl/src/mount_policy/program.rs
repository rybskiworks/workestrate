//! The compiled mount policy program and its pure evaluator (spec 22 §4,
//! §7, §10, §12, §13).
//!
//! The program is the compiler's output and the runtime's input: it
//! serializes to JSON for the v1 transmission channel (spec 22 §12) and
//! evaluates paths with full provenance traces (the same evaluation backs
//! runtime, `explain`, and `preview` — spec 22 §13).

use crate::mount_policy::lexical::LexicalPath;
use crate::mount_policy::rule::{PathPolicyRule, RuleEffect, RuleOrigin};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// The three visibility states a path can be in (spec 22 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// The path is visible to the sandboxed agent.
    Visible,
    /// The path is hidden (lookup ENOENT, readdir omission — spec 22 §14).
    Masked,
    /// A directory that is itself masked but may contain an unmasked
    /// descendant: traversable, with contents restricted to what leads to
    /// unmasked descendants (spec 22 §7). Only meaningful for directories; a
    /// caller deciding a non-directory must treat this as [`Decision::Masked`].
    TraversalOnly,
}

/// Write decision. Tagging successful writes is performed by the msb
/// enforcement layer; this pure library only decides allow/deny.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteDecision {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteRuleEffect {
    Allow,
    Deny,
    Protect,
}

/// Compiled pattern buckets. Cascade/rename/hardlink handling remains in msb.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CompiledRuleSet {
    pub allow: Vec<PathPolicyRule>,
    pub deny: Vec<PathPolicyRule>,
}

pub type WritePolicy = CompiledRuleSet;

/// Case sensitivity of pattern matching, recorded in the compiled program so
/// the runtime's behavior is pinned by the program, not by runtime defaults
/// (spec 22 §6). v1's explicit default is case-sensitive; the compiler
/// rejects any other fragment value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaseSensitivity {
    /// Patterns match case-sensitively (the v1 default).
    #[default]
    Sensitive,
    /// Reserved for a future version; v1 compiles no such program.
    Insensitive,
}

/// One matching rule in an explain trace (spec 22 §13): every match is
/// retained, including matches frozen out by an earlier terminal rule.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleMatch {
    /// Index of the rule in the program's compile-ordered rule list.
    pub rule_index: usize,
    /// The rule's effect (mask / unmask).
    pub effect: RuleEffect,
    /// Whether the rule is terminal (`overridable = false`).
    pub terminal: bool,
    /// Where the rule was declared (spec 22 §11).
    pub origin: RuleOrigin,
    /// True when an earlier terminal rule had already frozen the decision
    /// for this path: the match is recorded but had no effect (spec 22 §4).
    pub frozen_out: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WriteRuleMatch {
    pub rule_index: usize,
    pub effect: WriteRuleEffect,
    pub terminal: bool,
    pub origin: RuleOrigin,
    pub frozen_out: bool,
}

/// A decision plus its full provenance trace (spec 22 §13): every matching
/// rule in compile order, and which rule froze the decision, if any.
#[derive(Debug, Clone, PartialEq)]
pub struct Explained<T, M = RuleMatch> {
    /// The final visibility decision.
    pub decision: T,
    /// Every matching rule, in compile order (spec 22 §13).
    pub matches: Vec<M>,
    /// The terminal rule that froze the decision, if any (`frozen_by`,
    /// spec 22 §4, §13).
    pub frozen_by: Option<RuleOrigin>,
    /// True when the decision is `Masked` purely because the path is not
    /// valid UTF-8 (fail-closed, spec 22 §6).
    pub fail_closed_non_utf8: bool,
}

/// The compiled mount policy program: the compiler's output, transmitted to
/// the guest filesystem as JSON (spec 22 §12) and enforced for the mount's
/// lifetime.
#[derive(Debug, Clone, PartialEq)]
pub struct MountPolicyProgram {
    pub version: u32,
    /// The compiled rules, in compile order (= authority order, spec 22 §2).
    pub rules: Vec<PathPolicyRule>,
    /// Write behavior at masked paths (spec 22 §10).
    pub protect: Vec<PathPolicyRule>,
    pub writes: WritePolicy,
    /// Recorded case sensitivity, pinning the runtime's behavior (spec 22
    /// §6). v1: always `Sensitive`.
    pub case_sensitivity: CaseSensitivity,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MountPolicyProgramWire {
    version: Option<u32>,
    rules: Vec<PathPolicyRule>,
    protect: Vec<PathPolicyRule>,
    writes: WritePolicy,
    case_sensitivity: CaseSensitivity,
}

impl Serialize for MountPolicyProgram {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        MountPolicyProgramWire {
            version: Some(1),
            rules: self.rules.clone(),
            protect: self.protect.clone(),
            writes: self.writes.clone(),
            case_sensitivity: self.case_sensitivity,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MountPolicyProgram {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = MountPolicyProgramWire::deserialize(deserializer)?;
        match wire.version {
            Some(1) => Ok(Self {
                version: 1,
                rules: wire.rules,
                protect: wire.protect,
                writes: wire.writes,
                case_sensitivity: wire.case_sensitivity,
            }),
            Some(version) => Err(de::Error::custom(format!(
                "unsupported mount policy program version {version}; supported version is 1"
            ))),
            None => Err(de::Error::custom(
                "mount policy program version is required (expected version 1)",
            )),
        }
    }
}

impl MountPolicyProgram {
    /// Decide the visibility of a mount-root-relative path (spec 22 §4, §7).
    ///
    /// Rules evaluate in compile order; overridable matches are provisional
    /// (a later scope's matching rule supersedes them) and a terminal match
    /// freezes the decision for the path. Every matching rule is retained in
    /// the explanation, including frozen-out matches.
    ///
    /// A masked path that may contain an unmasked descendant reports
    /// [`Decision::TraversalOnly`] (spec 22 §7): only meaningful for
    /// directories — a caller deciding a non-directory must treat
    /// `TraversalOnly` as `Masked`. Paths are matched with fail-closed union
    /// semantics (a dir-only pattern matching the path exactly counts; the
    /// type-aware runtime uses [`crate::mount_policy::Pattern::matches_path`]
    /// / [`crate::mount_policy::Pattern::matches_dir`] instead).
    pub fn decide(&self, path: &LexicalPath) -> Explained<Decision> {
        // Non-UTF-8 paths are fail-closed (spec 22 §6): they cannot be
        // matched against the pattern set, so they are masked.
        let Some(text) = path.as_str() else {
            return Explained {
                decision: Decision::Masked,
                matches: Vec::new(),
                frozen_by: None,
                fail_closed_non_utf8: true,
            };
        };
        // Protection is a higher read boundary than ordinary mask/unmask
        // rules: a protected path is always hidden and cannot be reopened.
        let protected: Vec<_> = self
            .protect
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.pattern.matches_unknown(text))
            .collect();
        if !protected.is_empty() {
            let frozen_by = protected
                .iter()
                .find(|(_, rule)| rule.is_terminal())
                .map(|(_, rule)| rule.origin.clone());
            return Explained {
                decision: Decision::Masked,
                matches: protected
                    .into_iter()
                    .map(|(rule_index, rule)| RuleMatch {
                        rule_index,
                        effect: RuleEffect::Mask,
                        terminal: rule.is_terminal(),
                        origin: rule.origin.clone(),
                        frozen_out: false,
                    })
                    .collect(),
                frozen_by,
                fail_closed_non_utf8: false,
            };
        }
        let mut matches = Vec::new();
        let mut current: Option<Decision> = None;
        let mut frozen_by: Option<RuleOrigin> = None;
        for (rule_index, rule) in self.rules.iter().enumerate() {
            if !rule.pattern.matches_unknown(text) {
                continue;
            }
            let frozen_out = frozen_by.is_some();
            matches.push(RuleMatch {
                rule_index,
                effect: rule.effect,
                terminal: rule.is_terminal(),
                origin: rule.origin.clone(),
                frozen_out,
            });
            if frozen_out {
                continue;
            }
            current = Some(match rule.effect {
                RuleEffect::Mask => Decision::Masked,
                RuleEffect::Unmask => Decision::Visible,
            });
            if rule.is_terminal() {
                frozen_by = Some(rule.origin.clone());
            }
        }
        let mut decision = current.unwrap_or(Decision::Visible);
        if decision == Decision::Masked && self.may_unmask_descendant(path) {
            decision = Decision::TraversalOnly;
        }
        Explained {
            decision,
            matches,
            frozen_by,
            fail_closed_non_utf8: false,
        }
    }

    /// Decide a write. Successful writes are tagged by the msb enforcement
    /// layer; cascade, rename, hardlink, and symlink contracts are outside
    /// this pure policy library.
    pub fn decide_write(&self, path: &LexicalPath) -> Explained<WriteDecision, WriteRuleMatch> {
        let Some(text) = path.as_str() else {
            return Explained {
                decision: WriteDecision::Deny,
                matches: Vec::new(),
                frozen_by: None,
                fail_closed_non_utf8: true,
            };
        };
        let mut matches = Vec::new();
        let mut protected = false;
        let mut frozen_by = None;
        for (index, rule) in self.protect.iter().enumerate() {
            if !rule.pattern.matches_unknown(text) {
                continue;
            }
            let frozen_out = frozen_by.is_some();
            matches.push(WriteRuleMatch {
                rule_index: index,
                effect: WriteRuleEffect::Protect,
                terminal: rule.is_terminal(),
                origin: rule.origin.clone(),
                frozen_out,
            });
            if !frozen_out {
                protected = true;
                if rule.is_terminal() {
                    frozen_by = Some(rule.origin.clone());
                }
            }
        }
        let mut decision = if protected {
            WriteDecision::Deny
        } else {
            WriteDecision::Allow
        };
        // Reconstitute authority order after the public allow/deny split.
        // Deny is evaluated after allow within a scope, while a terminal deny
        // freezes all lower-authority rules (including later allows).
        let mut ordered = Vec::new();
        for (bucket, rules) in [
            (WriteRuleEffect::Allow, &self.writes.allow),
            (WriteRuleEffect::Deny, &self.writes.deny),
        ] {
            for (index, rule) in rules.iter().enumerate() {
                ordered.push((rule.origin.scope_kind.authority(), bucket, index, rule));
            }
        }
        ordered.sort_by_key(|(authority, bucket, index, _)| {
            (
                *authority,
                if *bucket == WriteRuleEffect::Allow {
                    0
                } else {
                    1
                },
                *index,
            )
        });
        for (_, bucket, index, rule) in ordered {
            if !rule.pattern.matches_unknown(text) {
                continue;
            }
            let frozen_out = frozen_by.is_some();
            let rule_index = self.protect.len()
                + if bucket == WriteRuleEffect::Deny {
                    self.writes.allow.len() + index
                } else {
                    index
                };
            matches.push(WriteRuleMatch {
                rule_index,
                effect: bucket,
                terminal: rule.is_terminal(),
                origin: rule.origin.clone(),
                frozen_out,
            });
            if frozen_out || protected {
                continue;
            }
            if bucket == WriteRuleEffect::Deny {
                decision = WriteDecision::Deny;
                if rule.is_terminal() {
                    frozen_by = Some(rule.origin.clone());
                }
            } else if decision != WriteDecision::Deny {
                decision = WriteDecision::Allow;
            }
        }
        Explained {
            decision,
            matches,
            frozen_by,
            fail_closed_non_utf8: false,
        }
    }

    pub fn is_protected(&self, path: &LexicalPath) -> bool {
        self.protect.iter().any(|rule| {
            path.as_str()
                .is_some_and(|text| rule.pattern.matches_unknown(text))
        })
    }

    /// Decide the visibility of `dir/name` (the readdir/lookup surface, spec
    /// 22 §14). An invalid child name fails closed: the child is `Masked`.
    pub fn decide_child(&self, dir: &LexicalPath, name: &str) -> Explained<Decision> {
        match dir.child(name) {
            Ok(path) => self.decide(&path),
            Err(_) => Explained {
                decision: Decision::Masked,
                matches: Vec::new(),
                frozen_by: None,
                fail_closed_non_utf8: false,
            },
        }
    }

    /// Conservative literal-prefix analysis (spec 22 §7): could any unmask
    /// rule reach a descendant of `dir`? The compiler extracts the fixed
    /// literal prefix of each unmask pattern; the answer is `true` when
    /// `dir` is a proper prefix of such a literal prefix (the unmask target
    /// sits below `dir`), or when the pattern reaches below its literal
    /// prefix and that prefix contains `dir` or is contained in it. A
    /// literal-only unmask that names `dir` exactly does NOT reach a
    /// descendant. Patterns without a fixed literal prefix (e.g.
    /// `**/`-prefixed) fall back to `true` — TraversalOnly restriction is
    /// always the safe direction.
    pub fn may_unmask_descendant(&self, dir: &LexicalPath) -> bool {
        if dir.is_non_utf8() {
            return false;
        }
        let dir_components = dir.components();
        self.rules
            .iter()
            .filter(|rule| rule.effect == RuleEffect::Unmask)
            .any(|rule| match rule.pattern.literal_prefix() {
                None => true,
                Some(literal) => {
                    let prefix = literal.components();
                    let below_dir =
                        prefix.len() > dir_components.len() && prefix.starts_with(dir_components);
                    let within_prefix =
                        literal.extends_below() && dir_components.starts_with(prefix);
                    below_dir || within_prefix
                }
            })
    }
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Decision::Visible => f.write_str("visible"),
            Decision::Masked => f.write_str("masked"),
            Decision::TraversalOnly => f.write_str("traversal-only"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::mount_policy::compile::compile;
    use crate::mount_policy::scope::{MountsFragment, PolicyScope, ScopeKind};
    use crate::mount_policy::value::PolicyValue;

    fn scope(
        kind: ScopeKind,
        layer: &str,
        mask: Vec<PolicyValue<String>>,
        unmask: Vec<PolicyValue<String>>,
    ) -> PolicyScope {
        PolicyScope::new(
            kind,
            layer,
            format!("{layer}.toml"),
            MountsFragment {
                mask,
                unmask,
                protect: vec![],
                writes: None,
                case_sensitivity: None,
            },
        )
    }

    fn decide(program: &MountPolicyProgram, path: &str) -> Decision {
        program.decide(&LexicalPath::new(path).unwrap()).decision
    }

    fn carve_out_program() -> MountPolicyProgram {
        compile(vec![scope(
            ScopeKind::ConfigRepoLayer,
            "repo",
            vec![PolicyValue::overridable("private/**".to_string())],
            vec![PolicyValue::overridable("private/public/**".to_string())],
        )])
        .unwrap()
    }

    #[test]
    fn traversal_only_is_the_third_state() {
        let program = carve_out_program();
        // `private` is itself masked but contains an unmasked descendant.
        assert_eq!(decide(&program, "private"), Decision::TraversalOnly);
        // The unmasked descendant is visible.
        assert_eq!(decide(&program, "private/public"), Decision::Visible);
        assert_eq!(decide(&program, "private/public/x"), Decision::Visible);
        // Other contents stay masked, with no traversal carve-out.
        assert_eq!(decide(&program, "private/other"), Decision::Masked);
        assert_eq!(decide(&program, "docs"), Decision::Visible);
    }

    #[test]
    fn masked_file_under_an_unmasked_ancestor_is_unmasked() {
        let program = carve_out_program();
        // The unmask pattern's subtree is reachable; a file at any depth
        // under it is visible.
        assert_eq!(decide(&program, "private/public/deep/x"), Decision::Visible);
        assert_eq!(decide(&program, "private/other/deep/x"), Decision::Masked);
    }

    #[test]
    fn may_unmask_descendant_uses_literal_prefix_analysis() {
        let program = carve_out_program();
        let private = LexicalPath::new("private").unwrap();
        let docs = LexicalPath::new("docs").unwrap();
        let public_deeper = LexicalPath::new("private/public/deeper").unwrap();
        assert!(program.may_unmask_descendant(&private));
        assert!(!program.may_unmask_descendant(&docs));
        assert!(program.may_unmask_descendant(&public_deeper));
    }

    #[test]
    fn may_unmask_descendant_falls_back_to_true_for_floating_patterns() {
        let program = compile(vec![scope(
            ScopeKind::ConfigRepoLayer,
            "repo",
            vec![PolicyValue::overridable("a/**".to_string())],
            vec![PolicyValue::overridable("**/x".to_string())],
        )])
        .unwrap();
        let anywhere = LexicalPath::new("any/dir").unwrap();
        assert!(program.may_unmask_descendant(&anywhere));
        // A masked directory under a floating unmask becomes traversal-only.
        assert_eq!(decide(&program, "a"), Decision::TraversalOnly);
        assert_eq!(decide(&program, "a/x"), Decision::Visible);
        assert_eq!(decide(&program, "a/b"), Decision::TraversalOnly);
    }

    #[test]
    fn non_utf8_paths_are_masked_fail_closed() {
        let program = carve_out_program();
        let path = LexicalPath::from_bytes(b"\xff\xfe").unwrap();
        let explained = program.decide(&path);
        assert_eq!(explained.decision, Decision::Masked);
        assert!(explained.fail_closed_non_utf8);
        assert!(explained.matches.is_empty());
    }

    #[test]
    fn explain_retains_every_matching_rule_with_origins() {
        // Two scopes both match `.env`: the operator's mask (higher
        // authority) and the repo's unmask (lower authority, supersedes the
        // provisional mask).
        let program = compile(vec![
            scope(
                ScopeKind::HomeRegistry,
                "registry",
                vec![PolicyValue::overridable(".env".to_string())],
                vec![],
            ),
            scope(
                ScopeKind::ConfigRepoLayer,
                "repo",
                vec![],
                vec![PolicyValue::overridable(".env".to_string())],
            ),
        ])
        .unwrap();
        let explained = program.decide(&LexicalPath::new(".env").unwrap());
        assert_eq!(explained.decision, Decision::Visible);
        assert_eq!(explained.matches.len(), 2);
        assert_eq!(explained.matches[0].effect, RuleEffect::Mask);
        assert_eq!(explained.matches[0].origin.layer, "registry");
        assert_eq!(explained.matches[1].effect, RuleEffect::Unmask);
        assert_eq!(explained.matches[1].origin.layer, "repo");
        assert!(!explained.matches.iter().any(|m| m.frozen_out));
        assert_eq!(explained.frozen_by, None);
    }

    #[test]
    fn decide_child_evaluates_dir_name() {
        let program = carve_out_program();
        let private = LexicalPath::new("private").unwrap();
        assert_eq!(
            program.decide_child(&private, "other").decision,
            Decision::Masked
        );
        let public = LexicalPath::new("private/public").unwrap();
        assert_eq!(
            program.decide_child(&public, "x").decision,
            Decision::Visible
        );
        // Invalid child names fail closed.
        assert_eq!(
            program.decide_child(&private, "..").decision,
            Decision::Masked
        );
    }

    #[test]
    fn program_round_trips_through_json() {
        let program = carve_out_program();
        let json = serde_json::to_string_pretty(&program).unwrap();
        let back: MountPolicyProgram = serde_json::from_str(&json).unwrap();
        assert_eq!(back, program);
        assert_eq!(back.version, 1);
        assert_eq!(back.case_sensitivity, CaseSensitivity::Sensitive);
        // Decisions are identical before and after the round trip.
        for path in ["private", "private/other", "private/public/x"] {
            let lexical = LexicalPath::new(path).unwrap();
            assert_eq!(
                back.decide(&lexical).decision,
                program.decide(&lexical).decision
            );
        }
    }

    #[test]
    fn program_version_is_required_supported_and_unknown_fields_rejected() {
        let program = carve_out_program();
        let json = serde_json::to_value(&program).unwrap();
        assert_eq!(json["version"], 1);
        let mut missing = json.clone();
        missing.as_object_mut().unwrap().remove("version");
        let err = serde_json::from_value::<MountPolicyProgram>(missing).unwrap_err();
        assert!(err.to_string().contains("version"));
        let mut unsupported = json.clone();
        unsupported["version"] = serde_json::json!(2);
        let err = serde_json::from_value::<MountPolicyProgram>(unsupported).unwrap_err();
        assert!(err.to_string().contains("2"));
        let mut old = json;
        old.as_object_mut()
            .unwrap()
            .insert("masked_writes".into(), serde_json::json!("deny"));
        let err = serde_json::from_value::<MountPolicyProgram>(old).unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }
}
