//! `PolicyValue`: the compact / expanded entry forms (spec 22 §3).
//!
//! Each entry in a scope's read/write deny/allow list is a [`PolicyValue`]:
//! either the compact string form (the pattern itself, `final = false` by
//! default) or the expanded table form `{ pattern = "...", final = true }`;
//! `final` defaults to `false` in both forms and must be explicit to be
//! `true`.
//!
//! Parsing follows the `deserialize_any` visitor idiom — the precedent is
//! the `RawBinding` / `RawBindingVisitor` pair in `config/types.rs` (spec 16):
//! `visit_str` / `visit_string` produce the compact form, `visit_map`
//! delegates to a `deny_unknown_fields` raw struct via
//! `MapAccessDeserializer` so unknown-field errors stay verbatim for inline
//! tables. `#[serde(untagged)]` is deliberately NOT used: untagged enums
//! buffer and retry, producing positionless, context-free errors.

use crate::mount_policy::pattern::Pattern;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::marker::PhantomData;

/// A policy value carrying its finality (spec 22 §3): `final` defaults to
/// `false` in both the compact and expanded forms and must be explicit in
/// expanded form to be `true`. A final value becomes a terminal rule at
/// compile time (spec 22 §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct PolicyValue<T> {
    /// The scalar payload (a raw pattern string, or a compiled [`Pattern`]).
    pub value: T,
    /// Whether this entry is final: frozen, so a later scope's matching rule
    /// may NOT supersede it (spec 22 §4). Defaults to `false`; `true` makes
    /// the compiled rule terminal. Serialized under the config-surface name
    /// `final`.
    #[serde(rename = "final")]
    pub terminal: bool,
}

impl<T> PolicyValue<T> {
    /// A relaxable (non-final) value (the compact-form default).
    pub fn relaxable(value: T) -> Self {
        Self {
            value,
            terminal: false,
        }
    }

    /// A final (terminal) value (spec 22 §4).
    pub fn terminal(value: T) -> Self {
        Self {
            value,
            terminal: true,
        }
    }
}

/// The scalar payload types a [`PolicyValue`] can wrap: a raw pattern string
/// (the collected-fragment form; the compiler validates it against the
/// declaring origin) or an already-compiled [`Pattern`].
pub trait PolicyScalar: Sized {
    /// The expanded-form key naming the scalar (e.g. `pattern`).
    const EXPANDED_KEY: &'static str;
    /// Human description used in deserializer `expecting` messages.
    const DESCRIBE: &'static str;
    /// Build from the compact string form. `Err` is shown verbatim as the
    /// deserialization error (the `RawBindingVisitor` error-message style:
    /// name the offending value, state the accepted forms).
    fn from_compact(raw: String) -> Result<Self, String>;
}

impl PolicyScalar for String {
    const EXPANDED_KEY: &'static str = "value";
    const DESCRIBE: &'static str = "a string";
    fn from_compact(raw: String) -> Result<Self, String> {
        Ok(raw)
    }
}

impl PolicyScalar for Pattern {
    const EXPANDED_KEY: &'static str = "pattern";
    const DESCRIBE: &'static str = "a glob pattern string";
    fn from_compact(raw: String) -> Result<Self, String> {
        // Origin-free parse: the declaring layer is not known at this level;
        // `Pattern::compile` is the origin-naming form used by the compiler.
        Pattern::parse(&raw).map_err(|err| err.to_string())
    }
}

/// Parse-time intermediate for the expanded table form (spec 22 §3).
/// `deny_unknown_fields` hard-rejects stray keys inside the inline table
/// (including the removed `overridable` key); the `pattern` alias keeps the
/// mount-policy surface (`{ pattern = ... }`) working for every scalar type,
/// including the raw-string fragment entries the compiler later validates
/// against the declaring origin.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExpanded<T> {
    #[serde(alias = "pattern")]
    value: T,
    #[serde(rename = "final")]
    r#final: Option<bool>,
}

/// Parse-time visitor for the compact/expanded forms (spec 22 §3), following
/// the `RawBindingVisitor` idiom in `config/types.rs`.
struct PolicyValueVisitor<T> {
    scalar: PhantomData<fn() -> T>,
}

impl<'de, T> Visitor<'de> for PolicyValueVisitor<T>
where
    T: PolicyScalar + Deserialize<'de>,
{
    type Value = PolicyValue<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} (compact form) or an inline table like {{ {} = ..., final = ... }} \
             (expanded form)",
            T::DESCRIBE,
            T::EXPANDED_KEY
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        T::from_compact(v.to_string())
            .map(PolicyValue::relaxable)
            .map_err(de::Error::custom)
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        T::from_compact(v)
            .map(PolicyValue::relaxable)
            .map_err(de::Error::custom)
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        // Delegate to the raw struct so `deny_unknown_fields` errors are
        // preserved verbatim for inline-table values.
        let raw = RawExpanded::<T>::deserialize(de::value::MapAccessDeserializer::new(map))?;
        Ok(PolicyValue {
            value: raw.value,
            terminal: raw.r#final.unwrap_or(false),
        })
    }
}

impl<'de, T> Deserialize<'de> for PolicyValue<T>
where
    T: PolicyScalar + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(PolicyValueVisitor {
            scalar: PhantomData,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct StringWrapper {
        entry: PolicyValue<String>,
    }

    #[derive(Debug, Deserialize)]
    struct PatternWrapper {
        entry: PolicyValue<Pattern>,
    }

    #[test]
    fn compact_string_form_defaults_to_relaxable() {
        let parsed: StringWrapper = toml::from_str(r#"entry = ".env""#).unwrap();
        assert_eq!(
            parsed.entry,
            PolicyValue {
                value: ".env".to_string(),
                terminal: false,
            }
        );
    }

    #[test]
    fn expanded_form_defaults_final_to_false() {
        let parsed: StringWrapper = toml::from_str(r#"entry = { value = ".env" }"#).unwrap();
        assert_eq!(
            parsed.entry,
            PolicyValue {
                value: ".env".to_string(),
                terminal: false,
            }
        );
    }

    #[test]
    fn expanded_form_accepts_explicit_final() {
        let parsed: StringWrapper =
            toml::from_str(r#"entry = { value = ".env", final = true }"#).unwrap();
        assert_eq!(
            parsed.entry,
            PolicyValue {
                value: ".env".to_string(),
                terminal: true,
            }
        );
    }

    #[test]
    fn removed_overridable_key_is_rejected_as_an_unknown_field() {
        let err =
            toml::from_str::<StringWrapper>(r#"entry = { value = ".env", overridable = false }"#)
                .unwrap_err();
        let text = err.to_string();
        assert!(
            text.contains("unknown field") && text.contains("overridable"),
            "the removed `overridable` key must be a hard unknown-field error: {text}"
        );
    }

    #[test]
    fn pattern_scalar_uses_the_pattern_key_in_expanded_form() {
        let parsed: PatternWrapper =
            toml::from_str(r#"entry = { pattern = ".git/", final = true }"#).unwrap();
        assert!(parsed.entry.terminal);
        assert_eq!(parsed.entry.value.raw(), ".git/");
        assert!(parsed.entry.value.is_dir_only());
    }

    #[test]
    fn pattern_scalar_accepts_the_compact_string_form() {
        let parsed: PatternWrapper = toml::from_str(r#"entry = ".env""#).unwrap();
        assert!(!parsed.entry.terminal);
        assert_eq!(parsed.entry.value.raw(), ".env");
    }

    #[test]
    fn pattern_key_is_accepted_for_string_scalars_too() {
        // The mount-policy surface writes `{ pattern = "..." }` even though
        // collected fragments hold raw strings for the compiler to validate.
        let parsed: StringWrapper = toml::from_str(r#"entry = { pattern = ".env" }"#).unwrap();
        assert_eq!(parsed.entry.value, ".env");
        assert!(!parsed.entry.terminal);
    }

    #[test]
    fn unknown_fields_in_the_inline_table_are_rejected_verbatim() {
        let err =
            toml::from_str::<StringWrapper>(r#"entry = { value = "x", bogus = 1 }"#).unwrap_err();
        let text = err.to_string();
        assert!(
            text.contains("unknown field") && text.contains("bogus"),
            "unknown-field error must be preserved verbatim: {text}"
        );
    }

    #[test]
    fn serialized_form_uses_the_final_key() {
        // `PolicyValue` is reachable via `MountsFragment: Serialize` (plan
        // JSON `policy` field); the wire name is the config-surface `final`.
        let json = serde_json::to_value(PolicyValue::terminal(".env".to_string())).unwrap();
        assert_eq!(json, serde_json::json!({"value": ".env", "final": true}));
    }

    #[test]
    fn pattern_scalar_rejects_invalid_patterns_with_the_raw_value_named() {
        let err = toml::from_str::<PatternWrapper>(r#"entry = "/etc/passwd""#).unwrap_err();
        let text = err.to_string();
        assert!(
            text.contains("/etc/passwd") && text.contains("absolute"),
            "error must name the offending value and reason: {text}"
        );
    }

    #[test]
    fn non_string_non_table_forms_are_rejected_with_the_accepted_forms() {
        let err = toml::from_str::<StringWrapper>("entry = 42").unwrap_err();
        let text = err.to_string();
        assert!(
            text.contains("compact form") && text.contains("expanded form"),
            "error must state the accepted forms: {text}"
        );
    }
}
