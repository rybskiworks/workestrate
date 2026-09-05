//! Per-workload inline ref override (`name[:config-ref][@instance]`; A5
//! Session 3b — ADR 0032 addendum §Selection ladder rung 3).
//!
//! The inline override is a CAPSULE-ONLY substitution: the named workload's
//! declaration is read at `<config-ref>` from its declaring config repo's
//! pinned archive while everything else stays home-scoped (or
//! `--config-ref`-scoped when that rung is also set — the inline ref wins
//! over `--config-ref` for THIS workload's declaring repo only).
//!
//! ## Two-phase process state (deps NEVER follow the override in v1)
//!
//! The override must NOT be visible to dependency auto-start
//! (`auto_start_dependencies` → `load_config` sees the HOME-scoped config)
//! but MUST be applied before the dependent's `ConfigWorkload` is
//! constructed. This is modeled as process-global two-phase state:
//!
//! 1. [`set_pending_inline_override`] records `(workload, config_ref)` at
//!    CLI parse (or, in a detached child, from the inherited
//!    [`WORKLOAD_REF_ENV`] env var via
//!    [`set_pending_inline_override_from_env`] — env inheritance reaches the
//!    child, whose argv carries only the bare name + `--instance`). The env
//!    pickup is NAME-GATED: it records pending ONLY when the current
//!    invocation's workload positional equals the override's workload name,
//!    so a DEPENDENCY's detached child (spawned by `auto_start_dependencies`
//!    with the same inherited env) never picks up the dependent's override.
//! 2. [`arm_inline_override`] flips the pending override LIVE. Arming points
//!    (main.rs): `plan` arms immediately (it never starts deps); `up`/`exec`
//!    arm AFTER `auto_start_dependencies` returns and BEFORE the
//!    `ConfigWorkload` is constructed ([`verb_arms_after_dep_autostart`]).
//!    `down`/`logs` and every other verb NEVER arm — a leftover exported
//!    [`WORKLOAD_REF_ENV`] must not arm the substitution on their config
//!    loads.
//! 3. `load_config` applies the substitution ONLY when armed
//!    ([`armed_inline_override`]).
//!
//! Same `std::sync::Mutex` rationale as `ACTIVE_CONTEXT` (config/mod.rs):
//! process-level state on a tokio multi-thread runtime, no thread affinity,
//! poisoning recovered with `into_inner()`.
//!
//! ## Grammar
//!
//! [`parse_workload_selector`] splits the workload positional as
//! `name[:config-ref][@instance]` (`:` = config branch, `@` = instance id —
//! consistent with `name:ctx.sha` image tags). Bare `name@id` WITHOUT `:`
//! is a hard error (fail-closed, recorded design): instance ids are passed
//! with `--instance`.

use anyhow::Result;

/// Env var carrying the inline override to a detached child
/// (`"<name>:<config-ref>"`). The parent sets it process-wide at CLI parse;
/// spawn env inheritance propagates it. The child's argv carries the bare
/// name + `--instance` from `detach_args`, so instance identity rides
/// `--instance` and the substitution rides this env var.
pub const WORKLOAD_REF_ENV: &str = "WORKESTRATE_WORKLOAD_REF";

/// The parsed workload positional: bare name plus the optional inline
/// override components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineOverride {
    /// Bare workload name (the part before the first `:`).
    pub name: String,
    /// Inline config ref (`name:<ref>`), when present.
    pub config_ref: Option<String>,
    /// Inline instance id (`name:<ref>@<id>`), when present. Already
    /// validated against
    /// [`crate::microsandbox::slots::validate_instance_id`].
    pub instance: Option<String>,
}

/// Parse the workload positional as `name[:config-ref][@instance]`.
///
/// - Split at the FIRST `:` → `(name, rest)`. An empty name is a hard error.
/// - `rest`: `rsplit_once('@')` → `(config_ref, instance)`; the instance id
///   MUST pass [`crate::microsandbox::slots::validate_instance_id`] (hard
///   error naming the bad id) and the ref must be non-empty. No `@` →
///   `instance = None`.
/// - Bare `name@id` WITHOUT `:` → HARD ERROR (fail-closed, recorded design):
///   instance ids are passed with `--instance`.
///
/// Legal forms: `prime`, `prime:feat-x`, `prime:feat-x@canary`.
pub fn parse_workload_selector(positional: &str) -> Result<InlineOverride> {
    let (name, rest) = match positional.split_once(':') {
        Some((n, r)) => {
            if n.is_empty() {
                anyhow::bail!(
                    "workload selector '{positional}' has an empty name before ':' \
                     (the form is name[:ref][@id])"
                );
            }
            (n, Some(r))
        }
        None => (positional, None),
    };

    let (config_ref, instance) = match rest {
        None => {
            if name.contains('@') {
                anyhow::bail!(
                    "instance ids are passed with --instance; the @ form is only valid \
                     in the combined name:ref@id override"
                );
            }
            (None, None)
        }
        Some(rest) => match rest.rsplit_once('@') {
            Some((r, id)) => {
                if r.is_empty() {
                    anyhow::bail!(
                        "workload selector '{positional}' has an empty config ref \
                         (the form is name:ref[@id])"
                    );
                }
                crate::microsandbox::slots::validate_instance_id(id).map_err(|e| {
                    anyhow::anyhow!("invalid instance id '{id}' in selector '{positional}': {e}")
                })?;
                (Some(r.to_string()), Some(id.to_string()))
            }
            None => {
                if rest.is_empty() {
                    anyhow::bail!(
                        "workload selector '{positional}' has an empty config ref after ':' \
                         (the form is name:ref[@id])"
                    );
                }
                (Some(rest.to_string()), None)
            }
        },
    };

    Ok(InlineOverride {
        name: name.to_string(),
        config_ref,
        instance,
    })
}

// ---------------------------------------------------------------------------
// Two-phase process-global state (see the module doc)
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct InlineOverrideState {
    /// `(workload, config_ref)` recorded at CLI parse / child env startup.
    pending: Option<(String, String)>,
    /// Whether `load_config` applies the substitution. Only a PENDING
    /// override can be armed.
    armed: bool,
}

static INLINE_OVERRIDE: std::sync::Mutex<InlineOverrideState> =
    std::sync::Mutex::new(InlineOverrideState {
        pending: None,
        armed: false,
    });

/// Record the pending inline override for this process (CLI parse; phase 1).
/// Overwrites any previously pending override — in a detached child the
/// env-derived pending was set from the SAME override, so overwrite is
/// idempotent in practice.
pub fn set_pending_inline_override(workload: &str, config_ref: &str) {
    let mut state = INLINE_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner());
    state.pending = Some((workload.to_string(), config_ref.to_string()));
}

/// Detached-child pickup (phase 1 from the environment): read
/// [`WORKLOAD_REF_ENV`] (`"<name>:<config-ref>"`, set process-wide by the
/// parent and inherited through the detach spawn) and record it pending —
/// but ONLY when `invocation_name` (the current invocation's BARE workload
/// positional; `None` for the bare batch `workload up` and for non-workload
/// commands) EQUALS the override's workload name.
///
/// NAME GATE (A5 review MEDIUM-1): spawn env inheritance reaches EVERY
/// detached child, including a DEPENDENCY's (`auto_start_dependencies` →
/// `start_service_detached_instance` → `workload up <dep>`). Without the
/// gate the dep's child would record the dependent's override pending and
/// arm it at its own up/exec arming point — resolving/archiving/lock-writing
/// the dependent's ref from the dep's process (racing the parent's lock
/// write, mis-attributing resolution errors to the dep, spamming the dep's
/// log with the dependent's notice). A `None` invocation name never sets
/// pending.
///
/// No-op when the var is absent or empty; a MALFORMED value is a hard error
/// (fail-closed — the parent only ever writes the well-formed shape).
pub fn set_pending_inline_override_from_env(invocation_name: Option<&str>) -> Result<()> {
    let Ok(value) = std::env::var(WORKLOAD_REF_ENV) else {
        return Ok(());
    };
    if value.is_empty() {
        return Ok(());
    }
    let selector = parse_workload_selector(&value).map_err(|e| {
        anyhow::anyhow!("malformed {WORKLOAD_REF_ENV}='{value}' (expected name:ref): {e}")
    })?;
    let Some(config_ref) = selector.config_ref else {
        anyhow::bail!("malformed {WORKLOAD_REF_ENV}='{value}' (expected name:ref)");
    };
    if invocation_name != Some(selector.name.as_str()) {
        return Ok(());
    }
    set_pending_inline_override(&selector.name, &config_ref);
    Ok(())
}

/// Verb scope of the POST-AUTO-START arming point (A5 review MEDIUM-1):
/// only `up`/`exec` arm there (main.rs, after `auto_start_dependencies`
/// returns and before the `ConfigWorkload` is constructed). `down`/`logs` —
/// and every other verb's config loads — NEVER arm: a leftover exported
/// [`WORKLOAD_REF_ENV`] must not arm the substitution during teardown or
/// log reads. (`plan` is exempt from this predicate by construction: it
/// never starts deps, so it arms at its own EARLIER point at CLI parse —
/// and only from the inline `name:ref` selector on its own argv.)
pub fn verb_arms_after_dep_autostart(verb: &str) -> bool {
    matches!(verb, "up" | "exec")
}

/// Flip the pending override LIVE (phase 2). No-op when nothing is pending
/// (the common case — every non-override invocation).
pub fn arm_inline_override() {
    let mut state = INLINE_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner());
    if state.pending.is_some() {
        state.armed = true;
    }
}

/// The armed `(workload, config_ref)` override, when one is live. Read by
/// `load_config` (config/loading.rs) — the substitution applies ONLY when
/// this returns `Some`. Also read by
/// [`crate::images::state::image_tag_context`] (A2: the armed override ref
/// IS the image tag/pointer context segment).
pub(crate) fn armed_inline_override() -> Option<(String, String)> {
    let state = INLINE_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner());
    if state.armed {
        state.pending.clone()
    } else {
        None
    }
}

/// The PENDING (not necessarily armed) `(workload, config_ref)` override,
/// when one is recorded. main.rs reads this for the A2/A5 ensure-ordering
/// seam ([`crate::images::ensure::ensure_after_arming`]): a pending override
/// on `up`/`exec` moves the ensure pre-flight to AFTER dep auto-start +
/// arming.
pub fn pending_inline_override() -> Option<(String, String)> {
    let state = INLINE_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner());
    state.pending.clone()
}

/// Clear both phases. Production runs one command per process and never
/// needs this; tests (which share the process) call it for isolation, under
/// `ENV_TEST_LOCK` like every process-global mutation.
pub fn clear_inline_override() {
    let mut state = INLINE_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner());
    state.pending = None;
    state.armed = false;
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
    use crate::config::test_support::{ENV_TEST_LOCK, EnvGuard};

    // ---- Grammar parse matrix ----

    #[test]
    fn parse_bare_name_unchanged() {
        let parsed = parse_workload_selector("prime").unwrap();
        assert_eq!(
            parsed,
            InlineOverride {
                name: "prime".to_string(),
                config_ref: None,
                instance: None,
            }
        );
    }

    #[test]
    fn parse_name_with_ref() {
        let parsed = parse_workload_selector("prime:feat-x").unwrap();
        assert_eq!(parsed.name, "prime");
        assert_eq!(parsed.config_ref.as_deref(), Some("feat-x"));
        assert_eq!(parsed.instance, None);
    }

    #[test]
    fn parse_name_with_ref_and_instance() {
        let parsed = parse_workload_selector("prime:feat-x@canary").unwrap();
        assert_eq!(parsed.name, "prime");
        assert_eq!(parsed.config_ref.as_deref(), Some("feat-x"));
        assert_eq!(parsed.instance.as_deref(), Some("canary"));
    }

    #[test]
    fn parse_splits_at_the_first_colon() {
        // A sha-with-colon is not legal git, but a registry-style ref like
        // `host:5000/branch` must keep everything after the FIRST ':'.
        let parsed = parse_workload_selector("prime:feat:x").unwrap();
        assert_eq!(parsed.name, "prime");
        assert_eq!(parsed.config_ref.as_deref(), Some("feat:x"));
    }

    #[test]
    fn parse_bare_at_form_is_a_hard_error() {
        let err = parse_workload_selector("prime@canary")
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "instance ids are passed with --instance; the @ form is only valid \
             in the combined name:ref@id override"
        );
    }

    #[test]
    fn parse_trailing_colon_is_a_hard_error() {
        let err = parse_workload_selector("prime:").unwrap_err().to_string();
        assert!(
            err.contains("empty config ref"),
            "error must name the empty ref: {err}"
        );
    }

    #[test]
    fn parse_empty_name_is_a_hard_error() {
        let err = parse_workload_selector(":feat-x").unwrap_err().to_string();
        assert!(
            err.contains("empty name"),
            "error must name the empty name: {err}"
        );
    }

    #[test]
    fn parse_invalid_instance_id_is_a_hard_error_naming_the_id() {
        let err = parse_workload_selector("prime:feat-x@UPPER")
            .unwrap_err()
            .to_string();
        assert!(err.contains("UPPER"), "error must name the bad id: {err}");
    }

    #[test]
    fn parse_rsplit_keeps_at_inside_the_ref() {
        // `feat@x` is a legal git refname component; the LAST '@' splits.
        let parsed = parse_workload_selector("prime:feat@x@canary").unwrap();
        assert_eq!(parsed.config_ref.as_deref(), Some("feat@x"));
        assert_eq!(parsed.instance.as_deref(), Some("canary"));
    }

    // ---- Two-phase state ----

    #[test]
    fn pending_is_invisible_until_armed() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        clear_inline_override();
        set_pending_inline_override("prime", "feat-x");
        assert_eq!(armed_inline_override(), None, "pending but not armed");
        arm_inline_override();
        assert_eq!(
            armed_inline_override(),
            Some(("prime".to_string(), "feat-x".to_string()))
        );
        clear_inline_override();
        assert_eq!(armed_inline_override(), None);
    }

    #[test]
    fn arm_without_pending_is_a_no_op() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        clear_inline_override();
        arm_inline_override();
        assert_eq!(armed_inline_override(), None);
    }

    #[test]
    fn env_pickup_sets_pending_unarmed() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(&[WORKLOAD_REF_ENV]);
        clear_inline_override();
        std::env::set_var(WORKLOAD_REF_ENV, "prime:feat-x");
        set_pending_inline_override_from_env(Some("prime")).unwrap();
        assert_eq!(
            armed_inline_override(),
            None,
            "env pickup records PENDING only; arming is the verb's job"
        );
        arm_inline_override();
        assert_eq!(
            armed_inline_override(),
            Some(("prime".to_string(), "feat-x".to_string()))
        );
        clear_inline_override();
    }

    #[test]
    fn env_pickup_is_a_no_op_when_unset() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(&[WORKLOAD_REF_ENV]);
        clear_inline_override();
        std::env::remove_var(WORKLOAD_REF_ENV);
        set_pending_inline_override_from_env(Some("prime")).unwrap();
        arm_inline_override();
        assert_eq!(armed_inline_override(), None);
    }

    #[test]
    fn env_pickup_rejects_malformed_values() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(&[WORKLOAD_REF_ENV]);
        clear_inline_override();
        for bad in ["prime", "prime:", ":feat-x"] {
            std::env::set_var(WORKLOAD_REF_ENV, bad);
            let err = set_pending_inline_override_from_env(Some("prime"))
                .unwrap_err()
                .to_string();
            assert!(
                err.contains(WORKLOAD_REF_ENV),
                "error must name the env var for '{bad}': {err}"
            );
        }
        clear_inline_override();
    }

    // ---- A5 review MEDIUM-1: the name gate + verb-scoped arming ----

    /// A DEPENDENCY's detached child inherits WORKESTRATE_WORKLOAD_REF but
    /// its invocation name differs from the override's workload name: the
    /// pickup records NOTHING, so even the child's own up/exec arming point
    /// is a no-op (no substitution, no archive/lock writes from the dep's
    /// process).
    #[test]
    fn env_pickup_name_gates_non_matching_invocations() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(&[WORKLOAD_REF_ENV]);
        clear_inline_override();
        std::env::set_var(WORKLOAD_REF_ENV, "prime:feat-x");
        set_pending_inline_override_from_env(Some("litellm")).unwrap();
        arm_inline_override();
        assert_eq!(
            armed_inline_override(),
            None,
            "a non-matching invocation name must never arm the override"
        );
        clear_inline_override();
    }

    /// Bare batch `workload up` (no name) and non-workload commands carry no
    /// invocation name: the pickup never sets pending, even when the var
    /// names a workload the batch will start.
    #[test]
    fn env_pickup_never_sets_pending_without_an_invocation_name() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(&[WORKLOAD_REF_ENV]);
        clear_inline_override();
        std::env::set_var(WORKLOAD_REF_ENV, "prime:feat-x");
        set_pending_inline_override_from_env(None).unwrap();
        arm_inline_override();
        assert_eq!(
            armed_inline_override(),
            None,
            "a name-less invocation (bare batch up / non-workload verb) must never arm"
        );
        clear_inline_override();
    }

    /// The matching-name path: the env-derived pending arms at the verb's
    /// arming point (the dependent's own detached child).
    #[test]
    fn env_pickup_matching_invocation_name_arms() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(&[WORKLOAD_REF_ENV]);
        clear_inline_override();
        std::env::set_var(WORKLOAD_REF_ENV, "prime:feat-x");
        set_pending_inline_override_from_env(Some("prime")).unwrap();
        arm_inline_override();
        assert_eq!(
            armed_inline_override(),
            Some(("prime".to_string(), "feat-x".to_string())),
            "a matching invocation name arms the env-derived override"
        );
        clear_inline_override();
    }

    /// down/logs (and every verb outside up/exec) never arm at the
    /// post-auto-start point: a leftover exported WORKESTRATE_WORKLOAD_REF
    /// with a MATCHING name stays pending-but-invisible on their config
    /// loads.
    #[test]
    fn down_and_logs_routes_never_arm_a_leftover_env_override() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(&[WORKLOAD_REF_ENV]);
        clear_inline_override();
        std::env::set_var(WORKLOAD_REF_ENV, "prime:feat-x");
        set_pending_inline_override_from_env(Some("prime")).unwrap();
        for verb in ["down", "logs", "plan", "build"] {
            assert!(
                !verb_arms_after_dep_autostart(verb),
                "{verb} must not arm at the post-auto-start point"
            );
        }
        // Simulating the down/logs route: no arming call fires, so the
        // override stays invisible to load_config.
        assert_eq!(armed_inline_override(), None);
        // ...while up/exec DO arm there.
        for verb in ["up", "exec"] {
            assert!(verb_arms_after_dep_autostart(verb));
        }
        arm_inline_override();
        assert_eq!(
            armed_inline_override(),
            Some(("prime".to_string(), "feat-x".to_string()))
        );
        clear_inline_override();
    }
}
