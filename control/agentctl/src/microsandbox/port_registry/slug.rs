use super::store::list_records;
use anyhow::Result;
use std::path::Path;

/// Alphabet for `--new` instance slugs: lowercase base32 (RFC 4648), i.e.
/// `a-z` + `2-7` — the symbols `0`/`1`/`8`/`9` are excluded (ambiguous with
/// letters). 32 symbols means each byte of entropy maps cleanly (`256 % 32
/// == 0`), so `byte % 32` is a uniform, bias-free draw onto the alphabet.
///
/// Normative for ADR 0021 §2 (`--new` allocation): slug length is fixed at 4
/// (≈ 20 bits, namespace 32^4 = 1,048,576 per slot), drawn from `/dev/urandom`.
pub(crate) const SLUG_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";

/// Slug length produced by [`auto_allocate_slug`]. Normative (ADR 0021 §2).
pub(crate) const SLUG_LEN: usize = 4;

/// Maximum collision/validity retries before `--new` gives up (ADR 0021 §2).
pub(crate) const SLUG_MAX_RETRIES: u32 = 8;

/// Auto-allocate a fresh instance slug for `--new`.
///
/// Draws 4-char `[a-z2-7]` slugs from `/dev/urandom` over [`SLUG_ALPHABET`],
/// retrying (up to [`SLUG_MAX_RETRIES`] attempts) when a draw either:
///   - collides with an existing `<slot>@<slug>` record, OR
///   - is purely numeric (e.g. `2345`), which [`validate_instance_id`]
///     (the uniform id rule) rejects.
///
/// The returned slug is therefore **guaranteed** to satisfy the instance-id
/// slug rule and to be unique among the slot's parallel instances. Returns a
/// hard error only if every attempt is rejected — effectively unreachable
/// given the namespace size, but fail-closed by construction.
///
/// [`validate_instance_id`]: crate::microsandbox::slots::validate_instance_id
pub fn auto_allocate_slug(state_dir: &Path, slot: &str) -> Result<String> {
    auto_allocate_slug_with(state_dir, slot, SLUG_MAX_RETRIES, random_slug)
}

/// Testable core of [`auto_allocate_slug`]: takes an explicit retry budget and
/// a `draw` closure (so tests can inject deterministic candidate sequences)
/// but is otherwise identical to the public entry point.
fn auto_allocate_slug_with<F>(
    state_dir: &Path,
    slot: &str,
    retries: u32,
    mut draw: F,
) -> Result<String>
where
    F: FnMut() -> Result<String>,
{
    let used = used_slugs_for_slot(state_dir, slot)?;
    for _ in 0..retries {
        let candidate = draw()?;
        // Reject purely-numeric draws up front (validate_instance_id would)
        // and reject collisions with existing parallel instances.
        if candidate.chars().any(|c| c.is_ascii_alphabetic()) && !used.contains(&candidate) {
            return Ok(candidate);
        }
    }
    anyhow::bail!(
        "could not allocate a non-colliding instance slug for slot '{}' after {} attempts          (namespace unexpectedly saturated)",
        slot,
        retries,
    )
}

/// Collect the parallel-instance suffixes already in use for `slot` — the
/// `<id>` portion of every `<slot>@<id>` record (any shape; integer slugs,
/// named slugs, etc. all count). Singleton records (`<slot>` with no `@`) and
/// records for other slots are ignored.
fn used_slugs_for_slot(state_dir: &Path, slot: &str) -> Result<Vec<String>> {
    let prefix = format!("{}@", slot);
    let mut out = Vec::new();
    for r in list_records(state_dir)? {
        if let Some(suffix) = r.instance.strip_prefix(&prefix) {
            out.push(suffix.to_string());
        }
    }
    Ok(out)
}

/// Draw one [`SLUG_LEN`]-char `[a-z2-7]` slug from `/dev/urandom`.
///
/// No new dependency: `/dev/urandom` is read directly. Each byte maps
/// uniformly onto [`SLUG_ALPHABET`] (`256 % 32 == 0` → no modulo bias).
fn random_slug() -> Result<String> {
    use std::io::Read;
    let mut buf = [0u8; SLUG_LEN];
    let mut f = std::fs::File::open("/dev/urandom")?;
    f.read_exact(&mut buf)?;
    Ok(buf
        .iter()
        .map(|&b| SLUG_ALPHABET[(b as usize) % SLUG_ALPHABET.len()] as char)
        .collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::super::store::register_sandbox;
    use super::*;
    use crate::config::test_support::unique_state_dir;

    #[test]
    fn random_slug_is_4_chars_of_base32_alphabet() -> Result<()> {
        for _ in 0..256 {
            let s = random_slug()?;
            assert_eq!(
                s.len(),
                SLUG_LEN,
                "slug must be exactly {SLUG_LEN} chars: {s}"
            );
            assert!(
                s.bytes()
                    .all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b)),
                "slug '{s}' contains a char outside [a-z2-7]"
            );
        }
        Ok(())
    }

    #[test]
    fn auto_allocate_slug_shape_and_validates() -> Result<()> {
        let state_dir = unique_state_dir("slug-empty");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        let slug = auto_allocate_slug(&state_dir, "personal-litellm")?;
        assert_eq!(slug.len(), SLUG_LEN);
        assert!(
            slug.bytes()
                .all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b)),
            "slug '{slug}' outside [a-z2-7]"
        );
        crate::microsandbox::slots::validate_instance_id(&slug)
            .expect("allocated slug must satisfy the instance-id slug rule");
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn auto_allocate_slug_avoids_existing_slugs_via_retry() -> Result<()> {
        let state_dir = unique_state_dir("slug-collide");
        register_sandbox(
            &state_dir,
            "personal-litellm@ab2z",
            Some("personal"),
            "litellm",
            &[14000],
        )?;
        let mut calls = 0u32;
        let draw = || -> Result<String> {
            calls += 1;
            Ok(if calls == 1 {
                "ab2z".to_string()
            } else {
                "mnxy".to_string()
            })
        };
        let slug = auto_allocate_slug_with(&state_dir, "personal-litellm", SLUG_MAX_RETRIES, draw)?;
        assert_eq!(
            slug, "mnxy",
            "must skip the colliding draw and return the fresh one"
        );
        assert_eq!(calls, 2);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn auto_allocate_slug_retries_past_purely_numeric_draws() -> Result<()> {
        let state_dir = unique_state_dir("slug-numeric");
        let mut calls = 0u32;
        let draw = || -> Result<String> {
            calls += 1;
            Ok(if calls == 1 {
                "2345".to_string()
            } else {
                "abcd".to_string()
            })
        };
        let slug = auto_allocate_slug_with(&state_dir, "personal-litellm", SLUG_MAX_RETRIES, draw)?;
        assert_eq!(slug, "abcd");
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn auto_allocate_slug_ignores_non_slug_and_other_slot_records() -> Result<()> {
        let state_dir = unique_state_dir("slug-ignores");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        register_sandbox(
            &state_dir,
            "personal-litellm@2",
            Some("personal"),
            "litellm",
            &[14001],
        )?;
        register_sandbox(
            &state_dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            &[14002],
        )?;
        register_sandbox(
            &state_dir,
            "work-litellm@ab2z",
            Some("work"),
            "litellm",
            &[24000],
        )?;
        register_sandbox(&state_dir, "personal-pi", Some("personal"), "pi", &[3000])?;
        let used = used_slugs_for_slot(&state_dir, "personal-litellm")?;
        assert_eq!(used.len(), 2, "only the two personal-litellm@* suffixes");
        assert!(used.contains(&"2".to_string()));
        assert!(used.contains(&"canary".to_string()));
        let slug = auto_allocate_slug(&state_dir, "personal-litellm")?;
        assert!(!used.contains(&slug), "allocated slug must not collide");
        crate::microsandbox::slots::validate_instance_id(&slug)?;
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn auto_allocate_slug_errors_when_retries_exhausted() -> Result<()> {
        let state_dir = unique_state_dir("slug-exhaust");
        register_sandbox(
            &state_dir,
            "personal-litellm@ab2z",
            Some("personal"),
            "litellm",
            &[14000],
        )?;
        let draw = || -> Result<String> { Ok("ab2z".to_string()) };
        let err = auto_allocate_slug_with(&state_dir, "personal-litellm", 3, draw).unwrap_err();
        assert!(
            err.to_string().contains("could not allocate"),
            "expected exhaustion error; got: {err}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }
}
