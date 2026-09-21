//! Bootstrap commands: `workestrate workload new` (workload scaffold),
//! plus the reference-fixture walker shared with `fleet new
//! --from-reference`.

use std::io::Write;

use anyhow::Result;

use crate::config;

/// Validate a workload name for `workestrate new`. Closes review finding A20.
///
/// Pattern: `^[a-z0-9][a-z0-9-]{0,62}$` — starts with an alphanumeric, allows
/// lowercase letters / digits / hyphens, max 63 characters (DNS-label length).
/// Rejects:
/// - empty / overlong names
/// - uppercase, underscores, dots, slashes, shell metacharacters
/// - anything starting with a hyphen (would create a hidden dir or flag-like
///   arg)
///
/// "Escape nothing — reject instead" is the policy: workload names flow into
/// both filesystem paths and TOML keys, so the safe set is the intersection.
///
/// Delegates to [`crate::config::validation::validate_identifier`] — the ONE
/// shared identifier rule also enforced at config-load time
/// (`validate_config`'s workload-key gate) and by `validate_fleet_name` —
/// so the create-time and load-time rules cannot drift. Trailing hyphens
/// remain allowed BY DESIGN (the pattern permits them; see the test pin).
pub fn validate_workload_name(name: &str) -> Result<()> {
    crate::config::validation::validate_identifier(name, "workload name")
}

pub fn cmd_new(name: &str, kind: &str) -> Result<()> {
    // Defense in depth: clap's value_parser constrains --kind at the CLI
    // boundary, but cmd_new is a library entry point — validate here too.
    if !matches!(kind, "agent" | "service") {
        anyhow::bail!(
            "invalid workload kind '{}' (expected \"agent\" or \"service\")",
            kind
        );
    }
    // WP1 / A20: validate the workload name BEFORE using it as a directory
    // name or interpolating it into TOML. Reject everything that is not a
    // safe lowercase-hyphen identifier; this prevents both path escape
    // (`../pwned`) and TOML injection (`a]b` breaking out of the workload
    // table).
    validate_workload_name(name)?;

    // Resolve the active config directory (trusted project, registry, or env).
    let config_dir = config::resolve_active_fleet_dir()?;
    let agent_dir = config_dir.join("agents").join(name);

    if agent_dir.exists() {
        anyhow::bail!("agents/{} already exists in {}", name, config_dir.display());
    }

    // Create the agent dir with create_dir (NOT create_dir_all): the
    // exists-check above + create_dir's fail-if-exists together close the
    // TOCTOU race — a concurrent `workestrate new <name>` that wins the
    // race turns our create_dir into an AlreadyExists error instead of
    // silently sharing the dir (FS-17). The `agents/` parent is ensured
    // first (create_dir does not create intermediates).
    if let Some(agents_parent) = agent_dir.parent() {
        std::fs::create_dir_all(agents_parent)?;
    }
    std::fs::create_dir(&agent_dir).map_err(|e| {
        anyhow::anyhow!(
            "failed to create agents/{} in {} (already exists?): {}",
            name,
            config_dir.display(),
            e
        )
    })?;
    let config_subdir = agent_dir.join("config");
    std::fs::create_dir_all(&config_subdir)?;
    std::fs::write(config_subdir.join(".gitkeep"), "")?;

    // Append a default workload entry to the fleet's workestrate.toml.
    let config_path = config_dir.join("workestrate.toml");
    let toml_entry = format!(
        "\n[workloads.{}]\n\
        kind = \"{}\"\n\
        image = {{ recipe = \"registry\", ref = \"node:24-bookworm-slim\" }}\n\
        workdir = \"/work\"\n\
        cpus = 2\n\
        memory_mib = 2048\n\
        command = []\n\
        log_stop_errors = false\n\n\
        [[workloads.{}.mounts]]\n\
        host = \"${{CWD}}\"\n\
        guest = \"/work\"\n\
        mode = \"rw\"\n\n\
        [workloads.{}.network.defaults]\n\
        egress = \"deny\"\n\n\
        [[workloads.{}.policy.egress.allow.host]]\n\
        ports = [53]\n\
        protocols = [\"tcp\", \"udp\"]\n\
        [[workloads.{}.policy.egress.allow.host]]\n\
        ports = [4000]\n\
        protocols = [\"tcp\"]\n\
        [[workloads.{}.policy.egress.allow.domain]]\n\
        domains = [\"github.com\", \"api.github.com\"]\n\
        port = 443\n\
        protocol = \"tcp\"\n",
        name, kind, name, name, name, name, name
    );

    // Write the TOML entry atomically (FS-17): read the current content,
    // append, write to a create_new scratch file in the SAME directory
    // (fail-if-exists guards against a torn concurrent append), then
    // rename(2) over workestrate.toml. A direct create+append open lets two
    // concurrent `workestrate new` invocations interleave writes.
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let existing = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let scratch = config_dir.join(format!(".workestrate.toml.new-{}", std::process::id()));
    // Belt-and-suspenders: the scratch path is pid-unique, but remove any
    // stale leftover from a crashed same-pid run so create_new cannot fail
    // spuriously.
    let _ = std::fs::remove_file(&scratch);
    {
        let mut scratch_file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&scratch)?;
        scratch_file.write_all(existing.as_bytes())?;
        scratch_file.write_all(toml_entry.as_bytes())?;
        scratch_file.sync_all()?;
    }
    std::fs::rename(&scratch, &config_path)?;

    println!(
        "Created agents/{}/config/ in {}",
        name,
        config_dir.display()
    );
    println!("Appended [workloads.{}] to {}", name, config_path.display());
    println!();
    println!("Edit {} to configure:", config_path.display());
    println!("  - Set image (recipe + ref, or recipe + contents for nix-layered)");
    println!("  - Set command");
    println!("  - Add env/mounts as needed");
    println!();
    println!("Test: workestrate {} plan", name);

    Ok(())
}

/// Walk up from `start` looking for `<root>/config.reference/workestrate.toml`.
/// Used by `--from-reference` to seed the scaffold with the canonical
/// 5-workload fixture. Returns the file path on success; bails with an
/// actionable message if not found within 10 levels.
pub fn find_reference_workestrate(start: &std::path::Path) -> Result<std::path::PathBuf> {
    let mut cursor = start.to_path_buf();
    for _ in 0..10 {
        let candidate = cursor.join("config.reference").join("workestrate.toml");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !cursor.pop() {
            break;
        }
    }
    anyhow::bail!(
        "could not locate config.reference/workestrate.toml by walking up from '{}'; \
         run `workestrate fleet new --from-reference` from inside the ai-workbench checkout",
        start.display()
    );
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn cmd_new_resolves_active_config_dir() -> Result<()> {
        // Hold the env-mutation lock for the whole body: this test mutates
        // HOME / WORKESTRATE_FLEET_DIR and must not race any other env-mutating
        // test (would otherwise corrupt env reads and poison ENV_TEST_LOCK).
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        // Create a temp fleet with a minimal workestrate.toml
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-cmd-new-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(tmp.join("workestrate.toml"), "schema_version = 1\n")?;

        // Point WORKESTRATE_FLEET_DIR at the temp repo
        let old = std::env::var("WORKESTRATE_FLEET_DIR").ok();
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", &tmp) };

        // Verify resolve_active_fleet_dir returns the temp dir
        let config_dir = config::resolve_active_fleet_dir()?;
        assert_eq!(
            config_dir, tmp,
            "resolve_active_fleet_dir should return the WORKESTRATE_FLEET_DIR path"
        );

        // Simulate what cmd_new does: create agent config dir + append to workestrate.toml
        let agent_config = tmp.join("agents").join("test-agent").join("config");
        std::fs::create_dir_all(&agent_config)?;
        std::fs::write(agent_config.join(".gitkeep"), "")?;

        let config_path = tmp.join("workestrate.toml");
        let toml_entry = "\n[workloads.test-agent]\nkind = \"agent\"\n";
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&config_path)?;
        use std::io::Write;
        file.write_all(toml_entry.as_bytes())?;

        // Restore env
        match old {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("WORKESTRATE_FLEET_DIR") },
        }

        // Verify the agent config dir was created in the fleet
        assert!(
            agent_config.exists(),
            "agent config dir should exist in fleet"
        );

        // Verify workestrate.toml was appended
        let toml_content = std::fs::read_to_string(tmp.join("workestrate.toml"))?;
        assert!(
            toml_content.contains("[workloads.test-agent]"),
            "workestrate.toml should contain the new workload entry"
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn cmd_new_no_config_repo_produces_clear_error() {
        // Hold the env-mutation lock for the whole body (see note above).
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        // Use a temp HOME so no registry exists and no project config is found.
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-no-config-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp_home).ok();

        let old_home = std::env::var("HOME").ok();
        let old_config = std::env::var("WORKESTRATE_FLEET_DIR").ok();
        let old_no_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").ok();

        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &tmp_home) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe {
            std::env::set_var(
                "XDG_CONFIG_HOME",
                tmp_home.join(".config").to_string_lossy().as_ref(),
            )
        };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("XDG_DATA_HOME", tmp_home.join(".local").join("share")) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_FLEET_DIR") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1") };

        let result = config::resolve_active_fleet_dir();

        // Restore env
        match old_home {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("HOME") },
        }
        match old_config {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("WORKESTRATE_FLEET_DIR") },
        }
        match old_no_project {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("WORKESTRATE_NO_PROJECT_CONFIG") },
        }
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("XDG_DATA_HOME") };

        let _ = std::fs::remove_dir_all(&tmp_home);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("no active fleet") || err.contains("workestrate config init"),
            "error should mention 'no active fleet' or 'workestrate config init'; got: {err}"
        );
    }

    // ---- FS-17: cmd_new TOCTOU ----

    /// FS-17: cmd_new against an EXISTING agents/<name> dir must fail — the
    /// exists-check + create_dir(fail-if-exists) pair is the race-safe
    /// refusal. Uses WORKESTRATE_FLEET_DIR to pin the active fleet.
    // ENV_TEST_LOCK held for the whole body: this test mutates process env
    // and must not race any other env-mutating test.
    #[test]
    fn cmd_new_fails_when_agent_dir_already_exists() -> Result<()> {
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::CONFIG_ENV_KEYS,
        );

        let tmp = crate::config::test_support::uniq_dir("fs17-new-exists");
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(tmp.join("workestrate.toml"), "schema_version = 1\n")?;
        std::fs::create_dir_all(tmp.join("agents").join("dupe"))?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", &tmp) };

        let result = cmd_new("dupe", "agent");
        assert!(result.is_err(), "existing agents/dupe must fail");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("already exists") || err.contains("failed to create"),
            "error must name the conflict: {err}"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    /// FS-17: the TOML entry write is atomic — after cmd_new the
    /// workestrate.toml contains the appended entry AND no scratch file is
    /// left behind (create_new + rename leaves no `.workestrate.toml.new-*`
    /// residue on the success path).
    #[test]
    fn cmd_new_appends_entry_atomically_and_leaves_no_scratch() -> Result<()> {
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::CONFIG_ENV_KEYS,
        );

        let tmp = crate::config::test_support::uniq_dir("fs17-new-atomic");
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(
            tmp.join("workestrate.toml"),
            "schema_version = 1\n\n[workloads.existing]\nkind = \"agent\"\n",
        )?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", &tmp) };

        cmd_new("fresh-agent", "agent")?;

        let content = std::fs::read_to_string(tmp.join("workestrate.toml"))?;
        assert!(
            content.contains("[workloads.existing]"),
            "pre-existing content must be preserved"
        );
        assert!(
            content.contains("[workloads.fresh-agent]"),
            "the new workload entry must be appended"
        );
        // No scratch residue.
        let scratch_leftovers: Vec<_> = std::fs::read_dir(&tmp)?
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(".workestrate.toml.new-")
            })
            .collect();
        assert!(
            scratch_leftovers.is_empty(),
            "no scratch files must remain after the atomic rename: {:?}",
            scratch_leftovers
                .iter()
                .map(|e| e.file_name())
                .collect::<Vec<_>>()
        );
        // The agent dir was created.
        assert!(
            tmp.join("agents")
                .join("fresh-agent")
                .join("config")
                .exists()
        );

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    /// W6a: `workload new --kind service` interpolates the kind into the
    /// generated [workloads.<name>] entry.
    #[test]
    fn cmd_new_with_service_kind_writes_service_entry() -> Result<()> {
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::CONFIG_ENV_KEYS,
        );

        let tmp = crate::config::test_support::uniq_dir("w6a-new-service");
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(tmp.join("workestrate.toml"), "schema_version = 1\n")?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", &tmp) };

        cmd_new("svc", "service")?;

        let content = std::fs::read_to_string(tmp.join("workestrate.toml"))?;
        assert!(
            content.contains("[workloads.svc]"),
            "the new workload entry must be appended"
        );
        assert!(
            content.contains("kind = \"service\""),
            "the generated entry must carry kind = \"service\":\n{content}"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    /// W6a: cmd_new is a library entry point — an invalid kind is rejected
    /// even though clap's value_parser already constrains --kind.
    #[test]
    fn cmd_new_rejects_invalid_kind() {
        let result = cmd_new("ok-name", "banana");
        assert!(result.is_err(), "invalid kind must error");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("invalid workload kind"),
            "error must name the invalid kind: {err}"
        );
        assert!(
            err.contains("agent") && err.contains("service"),
            "error must list the valid kinds: {err}"
        );
    }

    // ---- A20 regression: cmd_new rejects invalid workload names ----

    #[test]
    fn validate_workload_name_accepts_legitimate_names() {
        for ok in [
            "pi",
            "opencode",
            "example-agent",
            "my-cool-workload",
            "abc",
            "a1b",
            "a",
            "0",
            "1agent",
            &"a".repeat(63),
        ] {
            validate_workload_name(ok)
                .unwrap_or_else(|e| panic!("legitimate name '{ok}' rejected: {e}"));
        }
    }

    #[test]
    fn validate_workload_name_rejects_hostile_inputs() {
        // Each must fail. Categories: path escape, TOML injection,
        // shell-meta, uppercase, underscore, leading-hyphen, overlong, empty.
        let hostile = [
            "../pwned",      // path escape
            "/etc/pwned",    // absolute path
            "a]b",           // TOML table close-bracket injection
            "a.b",           // dot (TOML nested-key separator)
            "a b",           // whitespace
            "a$b",           // shell meta
            "a;b",           // shell meta
            "Agent",         // uppercase
            "my_agent",      // underscore (DNS-label style disallows)
            "-leading",      // leading hyphen
            "",              // empty
            &"x".repeat(64), // overlong (64 > 63)
        ];
        for h in hostile {
            let result = validate_workload_name(h);
            assert!(
                result.is_err(),
                "hostile name '{h}' should be rejected, but was accepted"
            );
        }
    }

    #[test]
    fn validate_workload_name_trailing_hyphen_is_allowed_by_design() {
        // The regex ^[a-z0-9][a-z0-9-]{0,62}$ permits trailing hyphens.
        // DNS labels disallow them, but workestrate workload names are not
        // DNS labels — they're filesystem path components and TOML keys.
        // If a future decision tightens this, update both the regex and this
        // test together.
        validate_workload_name("foo-").expect("trailing hyphen is allowed");
    }

    // ---- config::validate_fleet_name (used by `workestrate fleet new`) ----

    #[test]
    fn validate_fleet_name_accepts_legitimate_names() {
        for ok in [
            "personal",
            "work",
            "team",
            "prod",
            "a",
            "0",
            "p1",
            "my-config-2",
        ] {
            config::validate_fleet_name(ok)
                .unwrap_or_else(|e| panic!("legitimate config name '{ok}' rejected: {e}"));
        }
    }

    #[test]
    fn validate_fleet_name_rejects_hostile_inputs() {
        // Same safe-set as validate_workload_name: config names flow into
        // both filesystem paths and registry TOML keys, so the
        // intersection [a-z0-9-] is the only safe charset.
        let hostile = [
            "../pwned",
            "/etc/pwned",
            "a]b",
            "a.b",
            "a b",
            "Personal", // uppercase
            "my_config",
            "-leading",
            "",
            &"x".repeat(64),
        ];
        for h in hostile {
            let result = config::validate_fleet_name(h);
            assert!(
                result.is_err(),
                "hostile config name '{h}' should be rejected, but was accepted"
            );
        }
    }

    #[test]
    fn validate_fleet_name_error_mentions_config_name() {
        // Error label is interpolated from the validator, not hardcoded —
        // this catches a copy-paste regression where the workload-name
        // message would leak through.
        let err = config::validate_fleet_name("BAD").unwrap_err().to_string();
        assert!(
            err.contains("config name"),
            "error should mention 'config name'; got: {err}"
        );
        assert!(
            !err.contains("workload name"),
            "error should NOT mention 'workload name'; got: {err}"
        );
    }
}
