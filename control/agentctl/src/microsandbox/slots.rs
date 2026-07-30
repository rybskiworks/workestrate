//! Slot and instance-name logic for the instance lifecycle model (ADR 0021).
//!
//! A **slot** is the singleton namespace for a workload in a context:
//!   - `<workload>` when no context is active
//!   - `<context>-<workload>` when a context is active
//!
//! An **instance name** is `<slot>` (the singleton instance) or `<slot>@<id>`
//! (a parallel instance). `<id>` is an instance slug (see [`validate_instance_id`]).

use anyhow::Result;

/// Maximum length of an instance id slug (inclusive).
pub const MAX_INSTANCE_ID_LEN: usize = 32;

/// Compute the slot string for a workload in an optional context.
///
/// - No context: returns `workload` verbatim.
/// - With context: returns `format!("{}-{}", context, workload)`.
pub fn slot_for(workload: &str, context: Option<&str>) -> String {
    match context {
        Some(ctx) => format!("{}-{}", ctx, workload),
        None => workload.to_string(),
    }
}

/// Compute the instance name from a slot and an optional parallel id.
///
/// - No id: returns `slot` (the singleton instance name).
/// - With id: returns `format!("{}@{}", slot, id)`.
pub fn instance_name(slot: &str, instance_id: Option<&str>) -> String {
    match instance_id {
        Some(id) => format!("{}@{}", slot, id),
        None => slot.to_string(),
    }
}

/// Return the slot portion of an instance name (split at the first `@`).
/// If the instance has no `@`, returns the whole string.
///
/// Inverse of [`instance_name`]; used by `ps` to derive each row's `slot`
/// field (ADR 0021 §7) from the composed instance name.
pub fn slot_of_instance(instance: &str) -> &str {
    match instance.split_once('@') {
        Some((slot, _)) => slot,
        None => instance,
    }
}

/// Return the parallel-id portion of an instance name, if any.
///
/// Inverse of [`instance_name`]; used by `ps` to classify a row as singleton
/// vs parallel (ADR 0021 §7 `kind` field).
pub fn instance_id_of(instance: &str) -> Option<&str> {
    instance.split_once('@').map(|(_, id)| id)
}

/// Validate a user-supplied parallel instance id slug.
///
/// Rules (ADR 0021 §13):
/// 1. Matches `^[a-z0-9][a-z0-9-]{0,31}$` (1–32 chars; lowercase alphanumerics
///    and hyphens; must start alphanumeric).
/// 2. MUST NOT be `all` (reserved by `down --all-instances` / `down --all`).
/// 3. MUST NOT be purely numeric (reserved for future numeric-flag ambiguity;
///    also avoids collision with integer allocation from `--new`).
///
/// Returns `Ok(())` on success or an `anyhow::Error` whose `to_string()` names
/// the violated rule and the offending input.
pub fn validate_instance_id(id: &str) -> Result<()> {
    if id.is_empty() {
        anyhow::bail!("instance id cannot be empty");
    }
    if id.len() > MAX_INSTANCE_ID_LEN {
        anyhow::bail!(
            "instance id '{}' exceeds max length {} (got {})",
            id,
            MAX_INSTANCE_ID_LEN,
            id.len()
        );
    }
    let mut chars = id.chars();
    // First char must be [a-z0-9]. `id` is non-empty here (checked above), so
    // `chars.next()` is always `Some`; the `if let` is exhaustive in practice.
    if let Some(first) = chars.next() {
        if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
            anyhow::bail!(
                "instance id '{}' must start with [a-z0-9]; \
                 pattern: ^[a-z0-9][a-z0-9-]{{0,31}}$",
                id
            );
        }
    }
    for c in chars {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            anyhow::bail!(
                "instance id '{}' contains invalid character '{}' (allowed: [a-z0-9-]); \
                 pattern: ^[a-z0-9][a-z0-9-]{{0,31}}$",
                id,
                c
            );
        }
    }
    if id == "all" {
        anyhow::bail!(
            "instance id 'all' is reserved (used by --all-instances / --all); \
             choose a different id"
        );
    }
    // Purely numeric: every char is an ascii digit. Reject to avoid ambiguity
    // with the integer allocation range used by --new (and any future numeric
    // flag).
    if id.chars().all(|c| c.is_ascii_digit()) {
        anyhow::bail!(
            "instance id '{}' must not be purely numeric (ambiguous with --new integer \
             allocation); add at least one non-digit character",
            id
        );
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;

    #[test]
    fn slot_for_bare_without_context() {
        assert_eq!(slot_for("litellm", None), "litellm");
    }

    #[test]
    fn slot_for_namespaced_with_context() {
        assert_eq!(slot_for("litellm", Some("personal")), "personal-litellm");
    }

    #[test]
    fn instance_name_singleton() {
        assert_eq!(instance_name("personal-litellm", None), "personal-litellm");
    }

    #[test]
    fn instance_name_parallel() {
        assert_eq!(
            instance_name("personal-litellm", Some("canary")),
            "personal-litellm@canary"
        );
    }

    #[test]
    fn slot_of_instance_singleton() {
        assert_eq!(slot_of_instance("personal-litellm"), "personal-litellm");
    }

    #[test]
    fn slot_of_instance_parallel() {
        assert_eq!(
            slot_of_instance("personal-litellm@canary"),
            "personal-litellm"
        );
    }

    #[test]
    fn instance_id_of_singleton_is_none() {
        assert_eq!(instance_id_of("personal-litellm"), None);
    }

    #[test]
    fn instance_id_of_parallel() {
        assert_eq!(instance_id_of("personal-litellm@canary"), Some("canary"));
    }

    #[test]
    fn validate_instance_id_accepts_valid_slugs() {
        for ok in [
            "a",
            "canary",
            "canary-2",
            "blue-green",
            "abc123",
            "x-y-z",
            "0-abc",
        ] {
            validate_instance_id(ok)
                .unwrap_or_else(|e| panic!("legitimate id '{ok}' rejected: {e}"));
        }
    }

    #[test]
    fn validate_instance_id_rejects_empty() {
        assert!(validate_instance_id("").is_err());
    }

    #[test]
    fn validate_instance_id_rejects_overlong() {
        let long = "a".repeat(33);
        assert!(validate_instance_id(&long).is_err());
    }

    #[test]
    fn validate_instance_id_accepts_max_length() {
        let max = "a".repeat(32);
        validate_instance_id(&max).expect("32-char id is allowed");
    }

    #[test]
    fn validate_instance_id_rejects_bad_first_char() {
        for bad in ["-leading", "@foo", ".bar", "_baz", "Canary", "Canary-1"] {
            assert!(validate_instance_id(bad).is_err(), "should reject: {bad}");
        }
    }

    #[test]
    fn validate_instance_id_rejects_bad_interior_char() {
        for bad in ["foo_bar", "foo.bar", "foo/baz", "foo@bar", "foo baz"] {
            assert!(validate_instance_id(bad).is_err(), "should reject: {bad}");
        }
    }

    #[test]
    fn validate_instance_id_rejects_reserved_all() {
        let err = validate_instance_id("all").unwrap_err().to_string();
        assert!(
            err.contains("reserved"),
            "expected 'reserved' in error: {err}"
        );
    }

    #[test]
    fn validate_instance_id_rejects_purely_numeric() {
        for bad in ["1", "42", "12345"] {
            let err = validate_instance_id(bad).unwrap_err().to_string();
            assert!(
                err.contains("numeric"),
                "id '{bad}' should be rejected as numeric; got: {err}"
            );
        }
    }

    #[test]
    fn validate_instance_id_accepts_alpha_numeric_mix() {
        // Not purely numeric — has at least one letter.
        validate_instance_id("a1").expect("'a1' is valid (not purely numeric)");
        validate_instance_id("1a").expect("'1a' is valid (not purely numeric)");
        validate_instance_id("v2-canary").expect("'v2-canary' is valid");
    }
}
