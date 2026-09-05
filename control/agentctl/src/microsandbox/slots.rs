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

/// true iff the instance name's slot (strip a trailing `@<id>` via
/// [`slot_of_instance`]) equals [`slot_for`]`(workload, context)`.
///
/// This is the A1 context-at-create verification invariant: a registry
/// record may only be written for an `(instance, workload, context)` triple
/// whose instance slot IS the workload's slot in that context — e.g.
/// `("personal-litellm", "litellm", Some("personal"))` is consistent, while
/// `("personal-litellm", "litellm", None)` and
/// `("personal-litellm", "litellm", Some("work"))` are not. Pure string
/// check; no I/O.
pub fn context_consistent_with_instance(
    instance: &str,
    workload: &str,
    context: Option<&str>,
) -> bool {
    slot_of_instance(instance) == slot_for(workload, context)
}

/// Return the parallel-id portion of an instance name, if any.
///
/// Inverse of [`instance_name`]; used by `ps` to classify a row as singleton
/// vs parallel (ADR 0021 §7 `kind` field).
pub fn instance_id_of(instance: &str) -> Option<&str> {
    instance.split_once('@').map(|(_, id)| id)
}

/// Derive the `per-dir` strategy instance id (ADR 0030 V-addendum §V1):
/// `<dirname-slug>-<shorthash>` of the CANONICALIZED invocation cwd.
///
/// - `canonical_cwd` MUST already be canonicalized (`fs::canonicalize`) by
///   the caller — this function is pure string work, so every spelling of
///   one directory maps to one id and two different directories never share
///   one.
/// - The shorthash is FNV-1a 64-bit over the canonical path's UTF-8 bytes,
///   rendered as lowercase hex, FIRST 8 chars (hand-rolled — no new crate
///   dependency, so the nix vendor surface is unchanged).
/// - The dirname-slug (see [`dirname_slug`]) is capped at 23 chars so the
///   composed id is at most 23 + 1 + 8 = 32 = [`MAX_INSTANCE_ID_LEN`].
///
/// The derived id always contains `-` (slug + `-` + 8 hex chars) and the
/// hash suffix is never purely numeric, so [`validate_instance_id`] accepts
/// it unchanged.
pub fn per_dir_instance_id(canonical_cwd: &str) -> String {
    format!(
        "{}-{}",
        dirname_slug(canonical_cwd),
        fnv1a64_hex8(canonical_cwd.as_bytes())
    )
}

/// The dirname half of [`per_dir_instance_id`]: the last path component of
/// the canonical path, ASCII-lowercased, every maximal run of
/// non-`[a-z0-9]` chars collapsed to one `-`, leading/trailing `-` trimmed.
/// An empty result (a dirname with no ASCII alphanumerics at all) falls back
/// to `"dir"`; the filesystem root `/` slugs to `"root"`. Truncated to 23
/// chars max with any trailing `-` re-trimmed.
fn dirname_slug(canonical_cwd: &str) -> String {
    if canonical_cwd == "/" {
        return "root".to_string();
    }
    // Canonical paths are absolute with no trailing slash, so the final
    // `/`-separated component is the dirname (a leading empty component from
    // the root slash is never last here — the `/` case returned above).
    let last = canonical_cwd.rsplit('/').next().unwrap_or(canonical_cwd);
    let mut slug = String::with_capacity(last.len().min(24));
    let mut in_separator_run = false;
    for c in last.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if in_separator_run && !slug.is_empty() {
                slug.push('-');
            }
            in_separator_run = false;
            slug.push(c);
        } else {
            in_separator_run = true;
        }
    }
    if slug.is_empty() {
        return "dir".to_string();
    }
    // Cap at 23 chars (23 + 1 + 8 = 32 = MAX_INSTANCE_ID_LEN). All bytes
    // pushed are ASCII, so byte truncation is char-safe; a truncation can
    // land right after a separator `-`, so re-trim.
    slug.truncate(23);
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        return "dir".to_string();
    }
    slug
}

/// FNV-1a 64-bit hash of `bytes`, rendered as 16 lowercase hex chars and
/// truncated to the FIRST 8 (the per-dir shorthash). Hand-rolled (~10 lines)
/// so no new crate dependency touches the nix vendor surface.
fn fnv1a64_hex8(bytes: &[u8]) -> String {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(PRIME);
    }
    let hex = format!("{hash:016x}");
    hex[..8].to_string()
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
    if let Some(first) = chars.next()
        && !(first.is_ascii_lowercase() || first.is_ascii_digit())
    {
        anyhow::bail!(
            "instance id '{}' must start with [a-z0-9]; \
                 pattern: ^[a-z0-9][a-z0-9-]{{0,31}}$",
            id
        );
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

/// Derive an instance id from an arbitrary config-ref string (A5 Session 3b;
/// ADR 0032 addendum §Selection ladder rung 3): the inline `name:ref`
/// override implies a parallel instance id = the sanitized ref.
///
/// Same slugging rules as [`dirname_slug`] (the per-dir machinery):
/// ASCII-lowercased, every maximal run of non-`[a-z0-9]` chars collapsed to
/// one `-`, leading/trailing `-` trimmed, capped at [`MAX_INSTANCE_ID_LEN`]
/// (32) chars with any trailing `-` re-trimmed.
///
/// FAIL-CLOSED (unlike `dirname_slug`, which falls back to `"dir"`): the
/// result MUST pass [`validate_instance_id`] — an all-symbols ref
/// (e.g. `"!!!"` → empty) or a purely-numeric one (e.g. `"123"`) is a hard
/// error naming the ref, never a silently fabricated id.
pub fn sanitize_instance_id(raw: &str) -> Result<String> {
    let mut slug = String::with_capacity(raw.len().min(MAX_INSTANCE_ID_LEN + 1));
    let mut in_separator_run = false;
    for c in raw.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if in_separator_run && !slug.is_empty() {
                slug.push('-');
            }
            in_separator_run = false;
            slug.push(c);
        } else {
            in_separator_run = true;
        }
    }
    // All bytes pushed are ASCII, so byte truncation is char-safe; a
    // truncation can land right after a separator `-`, so re-trim.
    slug.truncate(MAX_INSTANCE_ID_LEN);
    while slug.ends_with('-') {
        slug.pop();
    }
    validate_instance_id(&slug).map_err(|e| {
        anyhow::anyhow!(
            "cannot derive an instance id from ref '{raw}' (sanitized to '{slug}'): {e}"
        )
    })?;
    Ok(slug)
}

// ---- msb sandbox-name encoding (ADR 0030 addendum 2026-08-26) ----
//
// The microsandbox SDK validates sandbox names at every create/get
// (microsandbox-types 0.6.15 `lib/validation.rs::validate_sandbox_name`):
// non-empty, at most 128 BYTES, first char ASCII alphanumeric, every char
// ASCII alphanumeric or '.' '-' '_'. Workestrate identities are
// `<slot>@<id>` (parallel / per-dir / scoped-dep shapes) and the `@` is
// ILLEGAL — before this fix every parallel-instance create failed with
// "invalid config: sandbox name must start with an alphanumeric ...".
//
// The fix: the workestrate identity STAYS `slot@id` everywhere (registries,
// slots, down-scopes, CLI); only the name handed to the SDK is ENCODED,
// deterministically and collision-resistantly (NOT injectively — see the
// [`msb_name_of_instance`] durability note), at each
// SDK name boundary. Plain (already-legal) names pass through UNCHANGED, so
// existing singleton homes keep their exact on-disk sandbox names.

/// Maximum length of an msb sandbox name in BYTES — the mirror of
/// microsandbox-types 0.6.15 `MAX_SANDBOX_NAME_BYTES`, pinned here so the
/// encoder can clamp without depending on SDK internals.
pub const MAX_MSB_NAME_BYTES: usize = 128;

/// true iff `name` passes the microsandbox SDK's own sandbox-name rule —
/// the exact mirror of microsandbox-types 0.6.15
/// `lib/validation.rs::validate_sandbox_name`: non-empty, at most
/// [`MAX_MSB_NAME_BYTES`] (128) bytes, first char ASCII alphanumeric, and
/// every char ASCII alphanumeric or one of '.' '-' '_'. Pure string check;
/// no I/O.
pub fn is_legal_msb_name(name: &str) -> bool {
    if name.is_empty() || name.len() > MAX_MSB_NAME_BYTES {
        return false;
    }
    let mut chars = name.chars();
    // `name` is non-empty here, so `chars.next()` is always Some.
    let first_alphanumeric = chars.next().is_some_and(|c| c.is_ascii_alphanumeric());
    first_alphanumeric
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
}

/// Encode a workestrate instance identity into the SANDBOX NAME handed to
/// the microsandbox SDK (ADR 0030 addendum 2026-08-26). Deterministic,
/// pure, total; the workestrate identity itself never changes.
///
/// - If [`is_legal_msb_name(instance)`] the instance is returned UNCHANGED:
///   plain slots (`litellm`, `personal-litellm`) and any already-legal name
///   keep their exact observable sandbox name — zero churn for existing
///   homes.
/// - Otherwise the `@` (and any other illegal char) must be mapped into the
///   SDK charset WITHOUT colliding with literal names. Encoding: replace
///   EVERY illegal char with `--` (so the single `@` becomes `--`), then
///   append `-` + the FNV-1a 64 hex8 of the ORIGINAL identity
///   ([`fnv1a64_hex8`]). The hash suffix disambiguates encoded names from
///   legal names that literally contain `--`: identity `"a--b"` stays
///   `"a--b"` unsuffixed while `"a@b"` becomes `"a--b-<hash>"`.
/// - Corner guard: an identity whose FIRST char maps to a non-alphanumeric
///   (a leading illegal char, or a leading '.'/'_'/'-' which the charset
///   allows but the SDK's first-char rule rejects) would still encode to an
///   illegal name; such bases are anchored with a legal `x` prefix so the
///   output is ALWAYS SDK-legal. Unreachable for real workestrate identities
///   (slots start `[a-z0-9]`).
/// - Length clamp: if the encoded form exceeds [`MAX_MSB_NAME_BYTES`] the
///   BASE is truncated by bytes (at a char boundary; identities are ASCII in
///   practice) to fit base + suffix, and the suffix is re-appended intact.
///
/// Collision-resistant, NOT injective: distinct identities USUALLY yield
/// distinct encodings because the hash suffix keys the original bytes, but
/// FNV-1a is not cryptographic — adversarially constructed identities can
/// collide on the same hex8 suffix. Accepted for the local single-operator
/// threat model; registry records, not names, remain the identity authority
/// (ADR 0030 addendum durability disclaimer).
pub fn msb_name_of_instance(instance: &str) -> String {
    if is_legal_msb_name(instance) {
        return instance.to_string();
    }
    let mut base = String::with_capacity(instance.len() + 8);
    for c in instance.chars() {
        if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
            base.push(c);
        } else {
            base.push_str("--");
        }
    }
    // Corner guard (see doc): anchor a base the SDK's first-char rule would
    // reject. Only fires when the identity ITSELF starts non-alphanumeric.
    if !base.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        base.insert(0, 'x');
    }
    let suffix = format!("-{}", fnv1a64_hex8(instance.as_bytes()));
    let max_base = MAX_MSB_NAME_BYTES - suffix.len();
    if base.len() > max_base {
        // Byte truncation at a char boundary (defensive floor scan; the
        // mapped base is pure ASCII in practice).
        let mut cut = max_base;
        while cut > 0 && !base.is_char_boundary(cut) {
            cut -= 1;
        }
        base.truncate(cut);
    }
    base.push_str(&suffix);
    base
}

/// Best-effort REVERSE of [`msb_name_of_instance`] (ADR 0030 addendum
/// 2026-08-26). REGISTRY RECORDS REMAIN THE SOURCE OF TRUTH — callers must
/// not rely on decode for correctness, only for display/lookup convenience.
///
/// If `name` ends with `-` + 8 lowercase hex chars, strip that suffix, map
/// every `--` in the remainder back to `@`, and accept the candidate ONLY
/// when its fnv1a64 hex8 equals the stripped suffix (self-verifying).
/// Otherwise — plain/legacy names, tampered suffixes, and the documented
/// corner where a literal `--` inside the base breaks reconstruction — the
/// input is returned unchanged.
///
/// Returns [`std::borrow::Cow`] so the overwhelmingly common
/// already-decodable case borrows; only a successful reconstruction
/// allocates. (The spec-shaped `-> &str` is unrepresentable in Rust: a
/// reconstructed `@`-identity is new text, not a slice of the input.)
pub fn instance_of_msb_name(name: &str) -> std::borrow::Cow<'_, str> {
    const HEX_SUFFIX_LEN: usize = 8;
    if name.len() > HEX_SUFFIX_LEN {
        let split = name.len() - HEX_SUFFIX_LEN;
        // Defensive: never split mid-char (multibyte names are possible in
        // foreign dir listings).
        if name.is_char_boundary(split) {
            let (base, suffix) = name.split_at(split);
            let lower_hex = suffix
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
            if lower_hex && let Some(stripped) = base.strip_suffix('-') {
                let candidate = stripped.replace("--", "@");
                if fnv1a64_hex8(candidate.as_bytes()) == suffix {
                    return std::borrow::Cow::Owned(candidate);
                }
            }
        }
    }
    std::borrow::Cow::Borrowed(name)
}

/// LEGACY COMPAT lookup order for an instance's possible msb sandbox names
/// (ADR 0030 addendum 2026-08-26): [`msb_name_of_instance`] first, then —
/// when different — the raw identity itself, so sandboxes created by old
/// forks under raw-`@` names remain manageable. Deduped; order matters
/// (encoded first). Used by the never-refuse teardown paths
/// (`runtime::teardown_for_replace` / `runtime::down_hardened`).
pub fn lookup_msb_names(instance: &str) -> Vec<String> {
    let mut out = Vec::with_capacity(2);
    out.push(msb_name_of_instance(instance));
    if out[0] != instance {
        out.push(instance.to_string());
    }
    out
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
    fn context_consistent_some_matching() {
        assert!(context_consistent_with_instance(
            "personal-litellm",
            "litellm",
            Some("personal")
        ));
    }

    #[test]
    fn context_consistent_some_mismatching() {
        assert!(!context_consistent_with_instance(
            "personal-litellm",
            "litellm",
            Some("work")
        ));
    }

    #[test]
    fn context_consistent_none_with_bare_name() {
        assert!(context_consistent_with_instance("litellm", "litellm", None));
    }

    #[test]
    fn context_consistent_none_with_namespaced_name_is_false() {
        assert!(!context_consistent_with_instance(
            "personal-litellm",
            "litellm",
            None
        ));
    }

    #[test]
    fn context_consistent_strips_parallel_id() {
        // "personal-litellm@canary": the @-id is stripped before comparing.
        assert!(context_consistent_with_instance(
            "personal-litellm@canary",
            "litellm",
            Some("personal")
        ));
    }

    #[test]
    fn context_consistent_id_only_difference_still_true() {
        // Two parallel ids of the SAME slot are equally consistent — the
        // check is slot-level, not instance-level.
        assert!(context_consistent_with_instance(
            "personal-litellm@blue",
            "litellm",
            Some("personal")
        ));
        assert!(context_consistent_with_instance(
            "personal-litellm@green",
            "litellm",
            Some("personal")
        ));
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

    // ---- A5 Session 3b: sanitize_instance_id (ref → parallel instance id) ----

    #[test]
    fn sanitize_instance_id_matrix() {
        let cases: &[(&str, &str)] = &[
            ("feat/x", "feat-x"),                   // slash collapses to one hyphen
            ("Feat-X", "feat-x"),                   // ASCII-lowercased
            ("feat--x", "feat-x"),                  // separator runs collapse to one hyphen
            ("feat/x/y", "feat-x-y"),               // multiple separators
            ("release/2026.08", "release-2026-08"), // dot is a separator
            ("--feat--", "feat"),                   // leading/trailing runs trimmed
            ("my_branch", "my-branch"),             // underscore is a separator
            ("v1.2.3-rc.1", "v1-2-3-rc-1"),
        ];
        for (raw, want) in cases {
            assert_eq!(
                sanitize_instance_id(raw).expect("sanitize must succeed"),
                *want,
                "sanitize({raw:?})"
            );
        }
    }

    #[test]
    fn sanitize_instance_id_caps_at_32_and_retrims() {
        let long = format!("{}/x", "a".repeat(40));
        let id = sanitize_instance_id(&long).expect("long ref sanitizes");
        assert!(id.len() <= MAX_INSTANCE_ID_LEN, "capped at 32: {id}");
        assert!(!id.ends_with('-'), "no trailing hyphen: {id}");
        validate_instance_id(&id).expect("the sanitized id must pass validation");
    }

    #[test]
    fn sanitize_instance_id_fail_closed_on_all_symbols() {
        let err = sanitize_instance_id("!!!").unwrap_err().to_string();
        assert!(
            err.contains("!!!"),
            "error must name the offending ref: {err}"
        );
    }

    #[test]
    fn sanitize_instance_id_fail_closed_on_purely_numeric() {
        // "123" sanitizes to "123", which validate_instance_id rejects.
        let err = sanitize_instance_id("123").unwrap_err().to_string();
        assert!(err.contains("123"), "error must name the ref: {err}");
    }

    #[test]
    fn sanitize_instance_id_is_deterministic() {
        assert_eq!(
            sanitize_instance_id("Feat/X.Y").unwrap(),
            sanitize_instance_id("Feat/X.Y").unwrap()
        );
    }

    // ---- ADR 0030 V-addendum §V1: per-dir instance id derivation ----
    //
    // Inputs are CANONICAL-style paths (absolute, no trailing slash, no
    // symlink/`..` spellings) — canonicalization is the caller's job
    // (`fs::canonicalize` at plan/spec time); these tests exercise the pure
    // derivation only.

    #[test]
    fn per_dir_id_shape_and_validity_matrix() {
        // (canonical cwd, expected dirname-slug prefix)
        let cases: &[(&str, &str)] = &[
            ("/home/node/work", "work"),
            ("/home/node/work/nested/deep", "deep"), // nesting: last component keys
            ("/home/node/My Project", "my-project"), // uppercase + space
            ("/home/node/my_project", "my-project"), // underscore is a separator
            ("/home/node/My__Weird  Dir", "my-weird-dir"), // separator runs collapse
            ("/home/node/123", "123"),               // purely-numeric dirname
            ("/home/node/-lead", "lead"),            // leading separator trimmed
            ("/home/node/trail-", "trail"),          // trailing separator trimmed
            ("/home/node/...hidden", "hidden"),      // dotfile
            ("/home/node/café", "caf"),              // non-ASCII collapses
            ("/home/node/日本語", "dir"),            // no ASCII alphanumerics → "dir"
            ("/", "root"),                           // filesystem root
        ];
        for (cwd, want_slug) in cases {
            let id = per_dir_instance_id(cwd);
            let (slug, hash) = id
                .rsplit_once('-')
                .unwrap_or_else(|| panic!("id '{id}' must contain a '-' separator"));
            assert_eq!(slug, *want_slug, "dirname slug for '{cwd}'");
            assert_eq!(hash.len(), 8, "shorthash is 8 chars: '{id}'");
            assert!(
                hash.chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "shorthash must be lowercase hex: '{id}'"
            );
            assert!(
                id.len() <= MAX_INSTANCE_ID_LEN,
                "id '{id}' exceeds {MAX_INSTANCE_ID_LEN}"
            );
            validate_instance_id(&id)
                .unwrap_or_else(|e| panic!("derived id '{id}' for '{cwd}' must validate: {e}"));
        }
    }

    #[test]
    fn per_dir_id_numeric_dirname_stays_valid() {
        // A purely-numeric dirname still derives a VALID id: the hash suffix
        // (hex, 8 chars) guarantees the composed id is never purely numeric.
        let id = per_dir_instance_id("/srv/123");
        assert!(id.starts_with("123-"), "slug keeps the dirname: {id}");
        validate_instance_id(&id).expect("numeric dirname id must validate");
    }

    #[test]
    fn per_dir_id_slug_truncates_to_length_cap() {
        let long = format!("/home/node/{}", "a".repeat(60));
        let id = per_dir_instance_id(&long);
        assert_eq!(id.len(), 23 + 1 + 8, "capped id is exactly 32: '{id}'");
        assert_eq!(&id[..23], "a".repeat(23));
        validate_instance_id(&id).expect("capped id must validate");
    }

    #[test]
    fn per_dir_id_truncation_retrims_trailing_dash() {
        // 23 chars ending mid separator-run: "…x-" would truncate to a
        // trailing '-'; the re-trim must drop it.
        let name = format!("{}-{}", "a".repeat(22), "zzz");
        let id = per_dir_instance_id(&format!("/home/node/{name}"));
        let slug = &id[..id.len() - 9];
        assert!(
            !slug.ends_with('-'),
            "truncated slug must not end in '-': '{slug}'"
        );
        assert_eq!(slug, "a".repeat(22));
    }

    #[test]
    fn per_dir_id_is_deterministic_and_distinguishes_dirs() {
        let a = per_dir_instance_id("/home/node/work");
        assert_eq!(
            a,
            per_dir_instance_id("/home/node/work"),
            "same input → same id"
        );
        let b = per_dir_instance_id("/home/node/other");
        assert_ne!(a, b, "different dirs → different ids");
        // Same dirname, different parents: the FULL path is hashed, so the
        // ids still differ (slug equal, hash differs).
        let c = per_dir_instance_id("/home/alice/work");
        assert_ne!(a, c, "same dirname in different parents must differ");
        assert!(
            a.rsplit_once('-').unwrap().0 == c.rsplit_once('-').unwrap().0,
            "same dirname → same slug half"
        );
    }

    #[test]
    fn per_dir_id_known_vector() {
        // Pin the exact FNV-1a derivation against an independently written
        // reference so a refactor cannot silently change every derived id.
        // The shorthash is the FIRST 8 hex chars of the 16-char lowercase
        // rendering (the HIGH 32 bits of the hash).
        fn fnv_reference(bytes: &[u8]) -> u64 {
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for &b in bytes {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
            h
        }
        let full = format!("{:016x}", fnv_reference(b"/home/node/work"));
        let expected = format!("work-{}", &full[..8]);
        assert_eq!(per_dir_instance_id("/home/node/work"), expected);
    }

    // ---- msb sandbox-name encoding (ADR 0030 addendum 2026-08-26) ----

    #[test]
    fn is_legal_msb_name_mirrors_the_sdk_rule() {
        for ok in [
            "a",
            "litellm",
            "personal-litellm",
            "feat--x-pi",
            "A.b_c-9",
            "9start",
            &"a".repeat(MAX_MSB_NAME_BYTES), // exactly at the cap
        ] {
            assert!(is_legal_msb_name(ok), "must be legal: {ok}");
        }
        for bad in [
            "",      // empty
            "-lead", // first char not alphanumeric
            ".dot",
            "_under",
            "@foo",                              // '@' is illegal
            "foo@bar",                           // '@' interior
            "foo bar",                           // space
            "café",                              // non-ASCII
            &"a".repeat(MAX_MSB_NAME_BYTES + 1), // one byte over
        ] {
            assert!(!is_legal_msb_name(bad), "must be illegal: {bad:?}");
        }
    }

    /// The encode matrix over ALL identity shapes (ADR 0030 addendum):
    /// bare singleton, context slot, parallel `<slot>@<id>`, per-dir
    /// `<slot>@<slug>-<hash8>`, scoped-dep `<dep>@<dependent>-<id>`, and a
    /// literal `--` legal name.
    #[test]
    fn msb_name_encode_matrix_over_all_shapes() {
        // Legal names pass through UNCHANGED (zero churn for existing homes).
        assert_eq!(msb_name_of_instance("litellm"), "litellm");
        assert_eq!(msb_name_of_instance("personal-litellm"), "personal-litellm");
        assert_eq!(msb_name_of_instance("feat--x-pi"), "feat--x-pi");

        // @-identities encode: every '@' → "--", plus "-<fnv1a64 hex8 of
        // the ORIGINAL identity>".
        for identity in [
            "personal-litellm@canary", // parallel
            "prime@feat-x-1a2b3c4d",   // per-dir
            "litellm@prime-1",         // scoped-dep
        ] {
            let encoded = msb_name_of_instance(identity);
            let want_base = identity.replace('@', "--");
            let want_suffix = format!("-{}", fnv1a64_hex8(identity.as_bytes()));
            assert_eq!(
                encoded,
                format!("{want_base}{want_suffix}"),
                "encode({identity})"
            );
        }
    }

    /// INJECTIVITY PIN: a legal name containing literal `--` stays
    /// unsuffixed while the corresponding @-identity encodes to the same
    /// base WITH the hash suffix — the two never collide.
    #[test]
    fn msb_name_encoding_is_injective_against_literal_dash_names() {
        let dashed = msb_name_of_instance("a--b");
        let at = msb_name_of_instance("a@b");
        assert_eq!(dashed, "a--b", "legal literal-dash name unchanged");
        assert_ne!(dashed, at, "encoded form must not collide with literal --");
        assert!(
            at.ends_with(&format!("-{}", fnv1a64_hex8(b"a@b"))),
            "encoded name carries the hash suffix: {at}"
        );
    }

    #[test]
    fn msb_name_encoding_is_deterministic() {
        for identity in ["personal-litellm@canary", "litellm", "café-wl@x"] {
            assert_eq!(
                msb_name_of_instance(identity),
                msb_name_of_instance(identity),
                "same input → same encoding"
            );
        }
    }

    /// LEGALITY PROPERTY: EVERY encoded output — matrix shapes plus
    /// adversarial inputs — must pass the SDK's own rule and fit the byte
    /// cap. This is the property the bug violated.
    #[test]
    fn msb_name_encoded_outputs_are_always_sdk_legal() {
        let identities = [
            // Matrix shapes.
            "litellm",
            "personal-litellm",
            "personal-litellm@canary",
            "prime@feat-x-1a2b3c4d",
            "litellm@prime-1",
            "feat--x-pi",
            // Adversarial inputs.
            "@leading",       // leading '@'
            "@",              // '@'-only
            "café-日本語@x",  // unicode chars
            &"a".repeat(200), // overlong identity
        ];
        for identity in identities {
            let encoded = msb_name_of_instance(identity);
            assert!(
                is_legal_msb_name(&encoded),
                "encode({identity:?}) → {encoded:?} must be SDK-legal"
            );
            assert!(
                encoded.len() <= MAX_MSB_NAME_BYTES,
                "encode({identity:?}) → {} bytes exceeds the cap",
                encoded.len()
            );
        }
    }

    /// The overlong case in detail: the BASE is truncated to fit and the
    /// hash suffix survives intact at the exact 128-byte cap.
    #[test]
    fn msb_name_overlong_identity_clamps_base_keeps_suffix() {
        let long = format!("{}@canary", "a".repeat(193)); // 200-char identity
        let encoded = msb_name_of_instance(&long);
        let suffix = format!("-{}", fnv1a64_hex8(long.as_bytes()));
        assert!(
            long.len() > MAX_MSB_NAME_BYTES,
            "precondition: input exceeds the cap"
        );
        assert_eq!(
            encoded.len(),
            MAX_MSB_NAME_BYTES,
            "clamped to exactly the cap: {encoded}"
        );
        assert!(
            encoded.ends_with(&suffix),
            "hash suffix preserved after truncation: {encoded}"
        );
        assert!(
            encoded.len() < long.len() + suffix.len(),
            "truncation happened"
        );
        assert!(is_legal_msb_name(&encoded));
    }

    /// Decode round-trips every matrix identity that has NO literal `--`
    /// component (the documented corner: literal `--` in the base breaks
    /// reconstruction; records remain the source of truth).
    #[test]
    fn msb_name_decode_round_trips_identities_without_literal_dashes() {
        for identity in [
            "litellm",
            "personal-litellm",
            "personal-litellm@canary",
            "prime@feat-x-1a2b3c4d",
            "litellm@prime-1",
        ] {
            let encoded = msb_name_of_instance(identity);
            assert_eq!(
                instance_of_msb_name(&encoded).as_ref(),
                identity,
                "round-trip({identity})"
            );
        }
    }

    #[test]
    fn msb_name_decode_leaves_plain_and_legacy_names_unchanged() {
        for name in [
            "litellm",
            "personal-litellm",
            "feat--x-pi",
            "some-legacy-sandbox",
        ] {
            assert_eq!(
                instance_of_msb_name(name).as_ref(),
                name,
                "decode must be identity on plain/legacy names"
            );
        }
    }

    #[test]
    fn msb_name_decode_rejects_a_tampered_hash_suffix() {
        let encoded = msb_name_of_instance("personal-litellm@canary");
        let real = &encoded[encoded.len() - 8..];
        let wrong = if real == "deadbeef" {
            "12345678"
        } else {
            "deadbeef"
        };
        let tampered = format!("{}{wrong}", &encoded[..encoded.len() - 8]);
        assert_ne!(tampered, encoded, "precondition: suffix actually changed");
        assert_eq!(
            instance_of_msb_name(&tampered).as_ref(),
            tampered,
            "a hash mismatch must return the input unchanged"
        );
    }
}
