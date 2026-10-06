#![allow(unsafe_code)]

use super::test_support::{CONFIG_ENV_KEYS, ENV_TEST_LOCK, EnvGuard, PROVENANCE_STORAGE_TEST_LOCK};
use crate::microsandbox::workload::{ConfigWorkload, Workload};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const ENV_KEYS: &[&str] = &[
    "WORKESTRATE_CONFIG",
    "WORKESTRATE_FLEET",
    "WORKESTRATE_FLEET_DIR",
    "WORKESTRATE_CONFIG_REF",
    "WORKESTRATE_REFERENCE_CONFIG",
    "WORKESTRATE_NO_PROJECT_CONFIG",
    "WORKESTRATE_INVOKE_CWD",
    "WORKESTRATE_STATE_DIR",
];
const SERVICE: &str = r#"
[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "node:24" }
command = []
[[workloads.svc.ports]]
host = 4000
guest = 4000
"#;
const CLIENT: &str = r#"
[workloads.client]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
[workloads.client.depends_on.svc]
env = "SERVICE_ADDR"
required = true
"#;

struct Fixture {
    // Environment restoration must precede release of the test lock.
    _env: EnvGuard,
    _config_env: EnvGuard,
    _provenance_lock: std::sync::MutexGuard<'static, ()>,
    _lock: std::sync::MutexGuard<'static, ()>,
    root: tempfile::TempDir,
    config: PathBuf,
    sha: String,
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .current_dir(dir)
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

impl Fixture {
    fn new(directory_mode: bool) -> Result<Self> {
        let lock = ENV_TEST_LOCK
            .lock()
            .map_err(|e| anyhow::anyhow!("environment test lock: {e}"))?;
        let provenance_lock = PROVENANCE_STORAGE_TEST_LOCK
            .lock()
            .map_err(|e| anyhow::anyhow!("provenance test lock: {e}"))?;
        let env = EnvGuard::capture(ENV_KEYS);
        let config_env = EnvGuard::capture(CONFIG_ENV_KEYS);
        super::clear_inline_override();
        let root = tempfile::tempdir()?;
        let config = root.path().join("config");
        let source = root.path().join("source");
        std::fs::create_dir_all(&config)?;
        std::fs::create_dir_all(&source)?;
        // SAFETY: ENV_TEST_LOCK remains held until the saved environment is restored.
        unsafe {
            for key in ENV_KEYS {
                std::env::remove_var(key);
            }
            std::env::set_var("HOME", root.path().join("home"));
            std::env::set_var("XDG_CONFIG_HOME", root.path().join("xdg-config"));
            std::env::set_var("XDG_DATA_HOME", root.path().join("xdg-data"));
            std::env::set_var("XDG_STATE_HOME", root.path().join("xdg-state"));
            std::env::set_var("WORKESTRATE_CONFIG", &config);
            std::env::set_var("WORKESTRATE_FLEET", "alpha");
            std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1");
            std::env::set_var("WORKESTRATE_INVOKE_CWD", root.path());
        }
        if directory_mode {
            let workloads = source.join("workestrate/workloads");
            std::fs::create_dir_all(&workloads)?;
            std::fs::write(
                source.join("workestrate/default.toml"),
                "schema_version = 1\n",
            )?;
            std::fs::write(workloads.join("svc.toml"), SERVICE)?;
            std::fs::write(workloads.join("client.toml"), CLIENT)?;
        } else {
            std::fs::write(
                source.join("workestrate.toml"),
                format!("schema_version = 1\n{SERVICE}\n{CLIENT}"),
            )?;
        }
        git(&source, &["init", "--quiet", "-b", "main"])?;
        git(&source, &["config", "user.name", "Namespace fixture"])?;
        git(
            &source,
            &["config", "user.email", "namespace@example.invalid"],
        )?;
        git(&source, &["add", "."])?;
        git(&source, &["commit", "--quiet", "-m", "fixture"])?;
        let sha = git(&source, &["rev-parse", "HEAD"])?;
        let mut registry =
            String::from("layers = [\"alpha\"]\n[settings]\ndefault_fleet = \"alpha\"\n");
        for name in ["alpha", "beta", "project", "local", "global-override"] {
            let clone = config.join("fleets").join(name);
            std::fs::create_dir_all(clone.parent().context("clone parent")?)?;
            let source_arg = source.to_str().context("UTF-8 source path")?;
            let clone_arg = clone.to_str().context("UTF-8 clone path")?;
            git(
                root.path(),
                &["clone", "--quiet", "--no-hardlinks", source_arg, clone_arg],
            )?;
            registry.push_str(&format!(
                "\n[fleets.{name}]\nurl = \"git+file://{}\"\nref = \"main\"\nrev = \"{sha}\"\n",
                source.display()
            ));
        }
        std::fs::write(config.join("config.toml"), registry)?;
        Ok(Self {
            _env: env,
            _config_env: config_env,
            _provenance_lock: provenance_lock,
            _lock: lock,
            root,
            config,
            sha,
        })
    }

    fn select(&self, fleet: &str) {
        // SAFETY: this fixture holds ENV_TEST_LOCK.
        unsafe {
            std::env::set_var("WORKESTRATE_FLEET", fleet);
        }
    }

    fn record(&self, instance: &str, namespace: &str, port: u16) -> Result<()> {
        let slot = instance.split('@').next().context("instance slot")?;
        let context = slot
            .strip_suffix("-svc")
            .context("service context prefix")?;
        crate::microsandbox::port_registry::check_and_register_sandbox_lifecycle(
            &self.config.join("state"),
            instance,
            Some(context),
            "svc",
            crate::microsandbox::plan::default_bind_ip(),
            &[port],
            &[crate::microsandbox::plan::PortMapping::new(port, 4000)],
            "2026-10-01T00:00:00Z",
            namespace,
            None,
            None,
            None,
            None,
        )
    }

    fn namespace(&self) -> Result<String> {
        super::load_config()?;
        let provenance = crate::merge::get_provenance().context("loaded provenance")?;
        let dirs = crate::merge::get_layer_dirs().context("loaded content roots")?;
        let fleets = crate::merge::get_field_fleets().unwrap_or_default();
        Ok(crate::commands::deps::namespace_for(
            Some(&provenance),
            &dirs,
            &fleets,
            "client",
        ))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        super::clear_inline_override();
    }
}

#[test]
fn pinned_aliases_share_content_but_resolve_their_own_services() -> Result<()> {
    for directory_mode in [false, true] {
        let fixture = Fixture::new(directory_mode)?;
        fixture.record("alpha-svc", "alpha", 4000)?;
        fixture.record("beta-svc", "beta", 4100)?;
        let archive = super::archive_dir(&fixture.sha)?;
        for (name, port) in [("alpha", 4000), ("beta", 4100)] {
            fixture.select(name);
            let workload = ConfigWorkload::new("client")?;
            assert_eq!(workload.namespace(), name);
            let plan = workload.plan();
            let env = plan
                .env
                .iter()
                .find(|env| env.name == "SERVICE_ADDR")
                .context("dependency export")?;
            assert_eq!(env.value, format!("host.microsandbox.internal:{port}"));
            let dirs = crate::merge::get_layer_dirs().context("content roots")?;
            assert!(dirs.values().all(|dir| dir.starts_with(&archive)));
        }
    }
    Ok(())
}

#[test]
fn pinned_fleet_rejects_foreign_default_records_even_with_use_override() -> Result<()> {
    let fixture = Fixture::new(true)?;
    fixture.record("foreign-svc@chosen", "default", 4100)?;
    for result in [
        ConfigWorkload::new("client"),
        ConfigWorkload::new_with_use_overrides(
            "client",
            &[("svc".to_string(), "chosen".to_string())],
        ),
    ] {
        assert!(matches!(&result, Err(error) if error.to_string().contains("namespace 'alpha'")));
    }
    Ok(())
}

#[test]
fn pinned_dependency_selection_accepts_explicit_instance_in_declaring_fleet() -> Result<()> {
    let fixture = Fixture::new(true)?;
    fixture.record("alpha-svc@chosen", "alpha", 4100)?;
    let workload = ConfigWorkload::new_with_use_overrides(
        "client",
        &[("svc".to_string(), "chosen".to_string())],
    )?;
    assert_eq!(workload.namespace(), "alpha");
    assert!(workload.plan().env.iter().any(|env| env.name == "SERVICE_ADDR" && env.value == "host.microsandbox.internal:4100"));
    Ok(())
}

#[test]
fn dependency_option_overrides_preserve_declaring_fleet_and_inline_ref_origin() -> Result<()> {
    let fixture = Fixture::new(true)?;
    std::fs::write(
        fixture.config.join("overrides.toml"),
        "[global.workloads.client.depends_on.svc]\nenv = \"SERVICE_ADDR\"\nrequired = false\n",
    )?;
    assert_eq!(fixture.namespace()?, "alpha");
    let project = fixture.root.path().join("project");
    std::fs::create_dir_all(&project)?;
    std::fs::write(
        project.join("workestrate.toml"),
        "schema_version = 1\n[workloads.client.depends_on.svc]\nenv = \"SERVICE_ADDR\"\nrequired = false\n",
    )?;
    super::trust_project(&project)?;
    // SAFETY: the fixture holds ENV_TEST_LOCK.
    unsafe {
        std::env::remove_var("WORKESTRATE_NO_PROJECT_CONFIG");
        std::env::set_var("WORKESTRATE_INVOKE_CWD", &project);
    }
    assert_eq!(fixture.namespace()?, "alpha");
    let clone = fixture.config.join("fleets/alpha");
    git(&clone, &["config", "user.name", "Namespace fixture"])?;
    git(
        &clone,
        &["config", "user.email", "namespace@example.invalid"],
    )?;
    git(&clone, &["checkout", "--quiet", "-b", "capsule"])?;
    std::fs::remove_file(clone.join("workestrate/workloads/client.toml"))?;
    std::fs::create_dir_all(clone.join("workestrate/workloads/client"))?;
    std::fs::write(
        clone.join("workestrate/workloads/client/workload.toml"),
        CLIENT,
    )?;
    git(&clone, &["add", "."])?;
    git(
        &clone,
        &["commit", "--quiet", "-m", "move client to capsule"],
    )?;
    super::set_pending_inline_override("client", "capsule");
    super::arm_inline_override();
    assert_eq!(fixture.namespace()?, "alpha");
    let provenance = crate::merge::get_provenance().context("inline provenance")?;
    assert_eq!(
        provenance.get("workloads.client.kind").map(String::as_str),
        Some("alpha#workestrate/workloads/client/workload.toml")
    );
    Ok(())
}

#[test]
fn synthetic_layers_do_not_borrow_matching_registered_labels_or_stale_origins() -> Result<()> {
    let fixture = Fixture::new(false)?;
    for name in ["project", "local", "global-override"] {
        fixture.select(name);
        assert_eq!(fixture.namespace()?, name);
        let project = fixture.root.path().join("synthetic");
        std::fs::create_dir_all(&project)?;
        std::fs::write(
            project.join("workestrate.toml"),
            format!("schema_version = 1\n{SERVICE}\n{CLIENT}"),
        )?;
        if name == "local" {
            // SAFETY: the fixture holds ENV_TEST_LOCK.
            unsafe {
                std::env::set_var("WORKESTRATE_FLEET_DIR", &project);
            }
        } else if name == "project" {
            super::trust_project(&project)?;
            // SAFETY: the fixture holds ENV_TEST_LOCK.
            unsafe {
                std::env::remove_var("WORKESTRATE_NO_PROJECT_CONFIG");
                std::env::set_var("WORKESTRATE_INVOKE_CWD", &project);
            }
        } else {
            std::fs::write(
                fixture.config.join("overrides.toml"),
                "[global.workloads.client]\nkind = \"agent\"\n",
            )?;
        }
        assert_eq!(fixture.namespace()?, "default");
        // SAFETY: the fixture holds ENV_TEST_LOCK.
        unsafe {
            std::env::remove_var("WORKESTRATE_FLEET_DIR");
            std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1");
        }
    }
    Ok(())
}

#[test]
fn failed_reload_clears_previous_registered_origins() -> Result<()> {
    let fixture = Fixture::new(true)?;
    assert_eq!(fixture.namespace()?, "alpha");
    std::fs::write(
        fixture.config.join("overrides.toml"),
        "[global.workloads.client]\nroot_disk_mib = 0\n",
    )?;
    assert!(super::load_config().is_err());
    assert!(crate::merge::get_field_fleets().is_none());
    Ok(())
}

#[test]
fn synthetic_option_overrides_preserve_fleets_with_colliding_display_labels() -> Result<()> {
    let fixture = Fixture::new(false)?;
    let project = fixture.root.path().join("option-only-project");
    std::fs::create_dir_all(&project)?;
    super::trust_project(&project)?;
    for fleet in ["project", "global-override"] {
        fixture.select(fleet);
        assert_eq!(fixture.namespace()?, fleet);
        let path = if fleet == "project" {
            // SAFETY: the fixture holds ENV_TEST_LOCK.
            unsafe {
                std::env::remove_var("WORKESTRATE_NO_PROJECT_CONFIG");
                std::env::set_var("WORKESTRATE_INVOKE_CWD", &project);
            }
            project.join("workestrate.toml")
        } else {
            fixture.config.join("overrides.toml")
        };
        let prefix = if fleet == "project" { "" } else { "global." };
        std::fs::write(&path, format!("[{prefix}workloads.client]\ncpus = 2\n"))?;
        assert_eq!(fixture.namespace()?, fleet);
        std::fs::write(
            &path,
            format!("[{prefix}workloads.client]\nkind = \"agent\"\n"),
        )?;
        assert_eq!(fixture.namespace()?, "default");
        std::fs::remove_file(path)?;
        // SAFETY: the fixture holds ENV_TEST_LOCK.
        unsafe {
            std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1");
        }
    }
    Ok(())
}

#[test]
fn inline_single_file_origin_update_is_limited_to_the_substituted_workload() -> Result<()> {
    let fixture = Fixture::new(false)?;
    std::fs::write(
        fixture.config.join("overrides.toml"),
        "[global.workloads.svc]\nkind = \"service\"\n",
    )?;
    assert_eq!(ConfigWorkload::new("svc")?.namespace(), "default");
    super::set_pending_inline_override("client", "main");
    super::arm_inline_override();
    assert_eq!(ConfigWorkload::new("svc")?.namespace(), "default");
    assert_eq!(fixture.namespace()?, "alpha");
    Ok(())
}
