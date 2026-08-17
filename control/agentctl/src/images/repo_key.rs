//! `repo_key` resolution (spec 21 §4.3, §8): the config-repo identity half of
//! the `<repo>#<tag>` record key.
//!
//! Rule (spec §4.3): the repo_key is the **registered-repo NAME** when the
//! workload's declaring-layer dir is under a registered checkout, else the
//! **canonical filesystem path** of the declaring dir. Records keyed by name
//! survive a checkout relocation; the path fallback keeps unregistered
//! (ad-hoc) declaring dirs distinct and honest.
//!
//! [`repo_key_for`] is the PURE core — it takes the registered (name,
//! checkout-path) pairs explicitly so tests drive it with fixture dirs. The
//! impure wrappers ([`registered_repo_checkouts`], [`resolve_repo_key`]) build
//! those pairs from the live registry:
//!
//! - **local-path entries** (`config new`; [`crate::config::entry_is_local_path`]):
//!   the entry `url` IS the checkout path (tilde-expanded);
//! - **managed git clones** (`config add <url>`): the checkout is
//!   [`crate::config::config_repo_dir`] —
//!   `resolve_store_dir()/config-repos/<name>` — the same resolution the
//!   `config`/`source` commands use.
//!
//! Containment is `Path::starts_with` on CANONICALIZED paths (symlink-safe);
//! when nested checkouts both contain the declaring dir, the LONGEST checkout
//! path wins (the most specific registration).

use std::path::{Path, PathBuf};

use crate::images::state::RepoIdentity;

/// Canonicalize, falling back to the path itself when canonicalization fails
/// (e.g. a not-yet-created dir). Keeps the pure core total.
fn canonicalize_or_self(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// The registered checkout containing `declaring_canon`, if any: longest
/// (most specific) canonical checkout path wins. Checkouts that fail to
/// canonicalize (not yet cloned) are skipped.
fn match_registered(
    declaring_canon: &Path,
    registered: &[(String, PathBuf)],
) -> Option<(String, PathBuf)> {
    let mut best: Option<(&String, PathBuf, usize)> = None;
    for (name, checkout) in registered {
        let Ok(canon) = checkout.canonicalize() else {
            continue;
        };
        if declaring_canon.starts_with(&canon) {
            let depth = canon.components().count();
            let better = match &best {
                Some((_, _, d)) => depth > *d,
                None => true,
            };
            if better {
                best = Some((name, canon, depth));
            }
        }
    }
    best.map(|(name, canon, _)| (name.clone(), canon))
}

/// Pure core: resolve the repo_key for a workload whose declaring-layer dir
/// is `declaring_dir`, against explicit `registered` (name, checkout-path)
/// pairs. Registered (contained) → the repo NAME; unregistered → the
/// canonical declaring-dir path (spec §4.3).
pub fn repo_key_for(declaring_dir: &Path, registered: &[(String, PathBuf)]) -> String {
    repo_key_for_optional(declaring_dir, registered).unwrap_or_else(|| {
        canonicalize_or_self(declaring_dir)
            .to_string_lossy()
            .into_owned()
    })
}

/// Like [`repo_key_for`] but returns `None` when the declaring dir is NOT
/// contained in a registered config-repo checkout (i.e. it is a synthetic /
/// single-file / test layer with no repo identity). Used by the ADR 0030
/// namespace resolution: a non-repo layer has no namespace and falls back to
/// "default".
pub fn repo_key_for_optional(
    declaring_dir: &Path,
    registered: &[(String, PathBuf)],
) -> Option<String> {
    let declaring_canon = canonicalize_or_self(declaring_dir);
    match_registered(&declaring_canon, registered).map(|(name, _)| name)
}

/// The (name, checkout-path) pairs for every registered config repo, resolved
/// from the live registry. Local-path entries contribute their `url` as the
/// checkout; managed git clones contribute `config-repos/<name>` under the
/// store dir. A missing/corrupt registry yields an EMPTY list (with a stderr
/// note on corruption) — every declaring dir then keys by canonical path,
/// which stays truthful under the advisory-record posture (spec §3.2).
pub fn registered_repo_checkouts() -> Vec<(String, PathBuf)> {
    let registry = match crate::config::load_registry() {
        Ok(Some(registry)) => registry,
        Ok(None) => return Vec::new(),
        Err(e) => {
            eprintln!(
                "note: unreadable registry ({e:#}); resolving image repo_keys by filesystem path only"
            );
            return Vec::new();
        }
    };
    registry
        .configs
        .iter()
        .map(|(name, entry)| {
            let checkout = if crate::config::entry_is_local_path(entry) {
                // Local-path repo (`config new`): the url IS the checkout.
                crate::config::expand_tilde(&entry.url)
            } else {
                // Managed clone (`config add <url>`): the store checkout, the
                // same resolution config_cmd/git.rs use.
                crate::config::config_repo_dir(name)
            };
            (name.clone(), checkout)
        })
        .collect()
}

/// Impure convenience wrapper: [`repo_key_for`] against the live registry.
pub fn resolve_repo_key(declaring_dir: &Path) -> String {
    repo_key_for(declaring_dir, &registered_repo_checkouts())
}

/// Build the [`RepoIdentity`] for a record: the repo_key rule for `name`, the
/// matched checkout (or the declaring dir when unregistered) for `path`, and
/// the nearest-ancestor `flake.nix` walk for `flake_root` (reuses
/// [`crate::commands::source::find_flake_root`], the same helper `source
/// clone` uses).
///
/// Returns `None` when no ancestor contains a `flake.nix` — the caller treats
/// that per the spec §7 failure table (a nix-layered workload without a flake
/// root is a hard error naming the repo; under batch scopes it is
/// skip-with-note).
pub fn repo_identity_for(
    declaring_dir: &Path,
    registered: &[(String, PathBuf)],
) -> Option<RepoIdentity> {
    let declaring_canon = canonicalize_or_self(declaring_dir);
    let flake_root = crate::commands::source::find_flake_root(&declaring_canon)?;
    let (name, path) = match match_registered(&declaring_canon, registered) {
        Some((name, checkout)) => (name, checkout),
        None => {
            let key = declaring_canon.to_string_lossy().into_owned();
            (key, declaring_canon)
        }
    };
    Some(RepoIdentity {
        name,
        path,
        flake_root,
    })
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
    use crate::config::test_support::{uniq_dir, EnvGuard, ENV_TEST_LOCK, HOME_ENV_KEYS};

    /// Fixture: a registered checkout at `<tmp>/checkout` and a declaring
    /// dir nested inside it. Returns (tmp, checkout, declaring).
    fn checkout_fixture(label: &str) -> (PathBuf, PathBuf, PathBuf) {
        let tmp = uniq_dir(label);
        let checkout = tmp.join("checkout");
        let declaring = checkout.join("workestrate").join("workloads").join("pi");
        std::fs::create_dir_all(&declaring).unwrap();
        (tmp, checkout, declaring)
    }

    #[test]
    fn declaring_dir_under_registered_checkout_keys_by_name() {
        let (tmp, checkout, declaring) = checkout_fixture("repokey-registered");
        let registered = vec![("personal".to_string(), checkout.clone())];

        assert_eq!(repo_key_for(&declaring, &registered), "personal");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn deeply_nested_declaring_dir_still_keys_by_name() {
        let (tmp, checkout, declaring) = checkout_fixture("repokey-nested");
        let deep = declaring.join("capsule").join("inner");
        std::fs::create_dir_all(&deep).unwrap();
        let registered = vec![("personal".to_string(), checkout.clone())];

        assert_eq!(repo_key_for(&deep, &registered), "personal");
        // The checkout root itself is also "under" the checkout.
        assert_eq!(repo_key_for(&checkout, &registered), "personal");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn unregistered_declaring_dir_keys_by_canonical_path() {
        let tmp = uniq_dir("repokey-unregistered");
        let declaring = tmp.join("ad-hoc").join("workestrate");
        std::fs::create_dir_all(&declaring).unwrap();
        let registered = vec![("personal".to_string(), tmp.join("someone-elses-checkout"))];

        let key = repo_key_for(&declaring, &registered);
        assert_eq!(
            key,
            declaring.canonicalize().unwrap().to_string_lossy(),
            "unregistered → canonical filesystem path (spec §4.3)"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn nested_checkouts_resolve_to_the_most_specific_registration() {
        let tmp = uniq_dir("repokey-longest");
        let outer = tmp.join("outer");
        let inner = outer.join("vendor").join("inner");
        let declaring = inner.join("workestrate");
        std::fs::create_dir_all(&declaring).unwrap();
        let registered = vec![
            ("outer-repo".to_string(), outer.clone()),
            ("inner-repo".to_string(), inner.clone()),
        ];

        assert_eq!(
            repo_key_for(&declaring, &registered),
            "inner-repo",
            "the LONGEST containing checkout wins"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn repo_identity_registered_uses_checkout_and_flake_root() {
        let (tmp, checkout, declaring) = checkout_fixture("repokey-identity");
        std::fs::write(checkout.join("flake.nix"), "{}\n").unwrap();
        let registered = vec![("personal".to_string(), checkout.clone())];

        let identity = repo_identity_for(&declaring, &registered).expect("flake root found");
        assert_eq!(identity.name, "personal");
        assert_eq!(identity.path, checkout.canonicalize().unwrap());
        assert_eq!(identity.flake_root, checkout.canonicalize().unwrap());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn repo_identity_unregistered_falls_back_to_declaring_dir() {
        let tmp = uniq_dir("repokey-identity-unreg");
        let declaring = tmp.join("repo").join("workestrate").join("workloads");
        std::fs::create_dir_all(&declaring).unwrap();
        std::fs::write(tmp.join("repo").join("flake.nix"), "{}\n").unwrap();

        let identity = repo_identity_for(&declaring, &[]).expect("flake root found");
        let canon = declaring.canonicalize().unwrap();
        assert_eq!(identity.name, canon.to_string_lossy());
        assert_eq!(identity.path, canon);
        assert_eq!(
            identity.flake_root,
            tmp.join("repo").canonicalize().unwrap(),
            "flake_root walks to the nearest ancestor with flake.nix"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn repo_identity_none_without_flake_root() {
        let tmp = uniq_dir("repokey-flakeless");
        let declaring = tmp.join("workestrate");
        std::fs::create_dir_all(&declaring).unwrap();

        // No flake.nix anywhere in the temp tree; assert against the tree
        // itself so ancestors above /tmp cannot leak in.
        assert!(
            repo_identity_for(&declaring, &[]).is_none_or(|id| !id.flake_root.starts_with(&tmp)),
            "no flake.nix inside the temp tree must not resolve into it"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// The impure wrapper: local-path entries contribute their `url` as the
    /// checkout; managed (git-url) clones contribute
    /// `resolve_store_dir()/config-repos/<name>`.
    #[test]
    fn registered_repo_checkouts_maps_local_and_managed_entries() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = uniq_dir("repokey-wrapper-home");
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("WORKESTRATE_HOME", &home);

        let local_checkout = home.join("my-local-repo");
        std::fs::write(
            home.join("config.toml"),
            format!(
                "layers = []\n\n\
                 [configs.local]\nurl = \"{}\"\n\n\
                 [configs.managed]\nurl = \"https://example.invalid/managed.git\"\n\
                 ref = \"main\"\nrev = \"abc123\"\n",
                local_checkout.display()
            ),
        )
        .unwrap();

        let checkouts = registered_repo_checkouts();
        let by_name: std::collections::HashMap<_, _> = checkouts.into_iter().collect();
        assert_eq!(
            by_name.get("local"),
            Some(&local_checkout),
            "local-path entry: the url IS the checkout path"
        );
        assert_eq!(
            by_name.get("managed"),
            Some(&home.join("config-repos").join("managed")),
            "managed clone: resolve_store_dir()/config-repos/<name>"
        );

        let _ = std::fs::remove_dir_all(&home);
    }

    /// End-to-end: a declaring dir under a registered local-path checkout
    /// resolves to the repo NAME through the live registry.
    #[test]
    fn resolve_repo_key_end_to_end_via_live_registry() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = uniq_dir("repokey-e2e-home");
        let checkout = home.join("checkout");
        let declaring = checkout.join("workestrate").join("workloads").join("pi");
        std::fs::create_dir_all(&declaring).unwrap();
        std::env::set_var("WORKESTRATE_HOME", &home);
        std::fs::write(
            home.join("config.toml"),
            format!(
                "layers = []\n\n[configs.personal]\nurl = \"{}\"\n",
                checkout.display()
            ),
        )
        .unwrap();

        assert_eq!(resolve_repo_key(&declaring), "personal");
        // Outside every checkout → canonical path.
        let stray = home.join("stray");
        std::fs::create_dir_all(&stray).unwrap();
        assert_eq!(
            resolve_repo_key(&stray),
            stray.canonicalize().unwrap().to_string_lossy()
        );

        let _ = std::fs::remove_dir_all(&home);
    }
}
