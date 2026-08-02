//! WP11 — PRODUCTION-PATH integration tests.
//!
//! Drives the REAL config resolution chain end-to-end by executing the
//! compiled `workestrate` binary as a child process:
//!
//!   reference layer → context config-repo layers (<home>/config-repos/<name>/)
//!   → user-global overrides (<home>/overrides.toml) → trusted project layer
//!   (<cwd>/workestrate.toml) → local layer.
//!
//! Unlike the existing suites, these tests NEVER set `WORKESTRATE_CONFIG_DIR`
//! (that env var bypasses the whole layering/discovery pipeline — single dev
//! layer, no merge, no trust, no context, no overrides). Instead each test
//! builds a fresh `WORKESTRATE_HOME` (ADR 0023 single-home layout):
//!
//!   registry_path  = <home>/config.toml
//!   overrides_path = <home>/overrides.toml
//!   store_dir      = <home>            (resolve_store_dir falls back to home)
//!   state_dir      = <home>/state
//!
//! and writes fixture TOML inline (registry, per-layer
//! `<home>/config-repos/<layer>/workestrate.toml`, project `workestrate.toml`).
//!
//! The repo's `config.reference/workestrate.toml` is prepended as the
//! "reference" base layer on every invocation — explicitly opted in via
//! `WORKESTRATE_REFERENCE_CONFIG=1` since cleanup phase 2 made the base
//! layer opt-in (discovery stays compile-time-anchored via the forwarded
//! CARGO_MANIFEST_DIR). It defines `example-service`
//! (kind=service), `example-agent`, `example-offensive` with
//! schema_version=1. Fixtures either override `example-service` fields or
//! add fresh workloads alongside it.
//!
//! All tests are read-only (`plan`, `plan --show-source`, `config trust`) or
//! fail before any KVM/sandbox work (untrusted-layer skip and schema_version
//! refusal both happen during config load, before `up` touches microsandbox).
//! No KVM, msb daemon, sops/age keys, network, or loaded images required.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::path::PathBuf;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_workestrate");

// ---------------------------------------------------------------------------
// Test harness: an isolated WORKESTRATE_HOME + scratch project dirs.
// ---------------------------------------------------------------------------

/// Isolated production-path sandbox. Creates:
///   <root>/home/            → WORKESTRATE_HOME (registry, overrides,
///                             config-repos/)
///   <root>/operator-home/   → HOME (kept separate from WORKESTRATE_HOME so
///                             tilde expansion and default-home discovery can
///                             never leak between tests)
///   <root>/scratch/         → cwd for "neutral" invocations (no project toml)
///
/// Drop removes the whole tree best-effort.
struct ProdHome {
    root: PathBuf,
    home: PathBuf,
    operator_home: PathBuf,
    scratch: PathBuf,
}

impl ProdHome {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "workestrate-production-path-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let home = root.join("home");
        let operator_home = root.join("operator-home");
        let scratch = root.join("scratch");
        for d in [&home, &operator_home, &scratch] {
            std::fs::create_dir_all(d).expect("create isolated dirs");
        }
        Self {
            root,
            home,
            operator_home,
            scratch,
        }
    }

    /// Path to the registry file (<home>/config.toml).
    fn registry(&self) -> PathBuf {
        self.home.join("config.toml")
    }

    /// Path to the user-global overrides file (<home>/overrides.toml).
    fn overrides(&self) -> PathBuf {
        self.home.join("overrides.toml")
    }

    /// Write the registry TOML.
    fn write_registry(&self, contents: &str) {
        std::fs::write(self.registry(), contents).expect("write registry");
    }

    /// Write a config-repo layer at <home>/config-repos/<name>/workestrate.toml.
    fn write_layer(&self, name: &str, contents: &str) {
        let dir = self.home.join("config-repos").join(name);
        std::fs::create_dir_all(&dir).expect("create layer dir");
        std::fs::write(dir.join("workestrate.toml"), contents).expect("write layer toml");
    }

    /// Write <home>/overrides.toml.
    fn write_overrides(&self, contents: &str) {
        std::fs::write(self.overrides(), contents).expect("write overrides");
    }

    /// Create a scratch "project" directory (a candidate trusted project)
    /// and return its path. `label` keeps it unique inside this sandbox.
    fn project_dir(&self, label: &str) -> PathBuf {
        let dir = self.root.join(format!("project-{label}"));
        std::fs::create_dir_all(&dir).expect("create project dir");
        dir
    }

    /// Build a `workestrate` Command with the full production-path env:
    /// WORKESTRATE_HOME set, bypass vars REMOVED, XDG pointed into the
    /// sandbox (defensive hermeticity), CARGO_MANIFEST_DIR forwarded plus
    /// WORKESTRATE_REFERENCE_CONFIG=1 (cleanup-phase-2 opt-in) so the
    /// reference layer resolves deterministically no matter how the test
    /// harness itself was launched.
    fn cmd(&self) -> Command {
        let mut c = Command::new(BIN);
        c.env("WORKESTRATE_HOME", &self.home);
        c.env("HOME", &self.operator_home);
        c.env("XDG_CONFIG_HOME", self.operator_home.join(".config"));
        c.env(
            "XDG_DATA_HOME",
            self.operator_home.join(".local").join("share"),
        );
        c.env(
            "XDG_STATE_HOME",
            self.operator_home.join(".local").join("state"),
        );
        // Reference-config discovery fallback (compile-time anchored), plus
        // the cleanup-phase-2 opt-in so the reference base layer is included.
        c.env("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR"));
        c.env("WORKESTRATE_REFERENCE_CONFIG", "1");
        // CRITICAL: never leak the bypass/discovery vars into the child.
        c.env_remove("WORKESTRATE_CONFIG_DIR");
        c.env_remove("WORKESTRATE_NO_PROJECT_CONFIG");
        c.env_remove("WORKESTRATE_CONTEXT");
        c.env_remove("AGENTCTL_ROOT");
        // Neutral cwd by default; callers override with .current_dir().
        c.current_dir(&self.scratch);
        c
    }

    /// Run a command and return its Output.
    fn run(&self, cmd: &mut Command) -> Output {
        cmd.output().expect("spawn workestrate child")
    }

    fn cleanup(&self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Drop for ProdHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Assert success and return stdout (with a useful failure dump otherwise).
fn expect_ok(out: &Output, what: &str) -> String {
    assert!(
        out.status.success(),
        "{what} failed (status={:?});\nstdout:\n{}\nstderr:\n{}",
        out.status,
        stdout_of(out),
        stderr_of(out)
    );
    stdout_of(out)
}

/// A minimal registry with NO contexts (bare `layers` backward-compat) and
/// the given config-repo entries. NOTE (TOML layout): the bare `layers` key
/// must appear BEFORE any `[table]` header, otherwise it lands inside the
/// last `[configs.<name>]` table instead of the registry root.
fn registry_bare_layers(layers: &[&str]) -> String {
    let mut s = String::from("layers = [");
    s.push_str(
        &layers
            .iter()
            .map(|l| format!("\"{l}\""))
            .collect::<Vec<_>>()
            .join(", "),
    );
    s.push_str("]\n\n[settings]\n");
    for name in layers {
        s.push_str(&format!(
            "\n[configs.{name}]\nurl = \"file:///unused/{name}\"\nref = \"main\"\n"
        ));
    }
    s
}

// ---------------------------------------------------------------------------
// 1. TRUSTED PROJECT FLOW
// ---------------------------------------------------------------------------

/// A trusted project's `./workestrate.toml` overrides a field of the
/// reference `example-service` workload through the real chain:
///   config trust <dir> → [[trusted_projects]] in the registry
///   → cwd = projdir → plan shows the override.
#[test]
fn production_trusted_project_layer_applies() {
    let env = ProdHome::new("trusted-applies");
    // Registry exists (bare, no layers) so trust-gating is active.
    env.write_registry("[settings]\n");

    let proj = env.project_dir("trusted");
    std::fs::write(
        proj.join("workestrate.toml"),
        "schema_version = 1\n\n[workloads.example-service]\ncpus = 7\n",
    )
    .expect("write project layer");

    // Trust the project dir via the real CLI (writes [[trusted_projects]]).
    let out = env.run(env.cmd().args(["config", "trust"]).arg(&proj));
    let trust_stdout = expect_ok(&out, "config trust");
    assert!(
        trust_stdout.contains("Trusted:"),
        "trust should report success; got:\n{trust_stdout}"
    );

    // The registry must now carry the trusted project entry.
    let registry = std::fs::read_to_string(env.registry()).expect("read registry");
    assert!(
        registry.contains("trusted_projects"),
        "registry should record [[trusted_projects]]; got:\n{registry}"
    );

    // From inside the project, plan must reflect the project-layer override.
    let out = env.run(
        env.cmd()
            .args(["example-service", "plan"])
            .current_dir(&proj),
    );
    let stdout = expect_ok(&out, "example-service plan (trusted)");
    assert!(
        stdout.contains("cpus: 7"),
        "trusted project layer should override cpus to 7; got:\n{stdout}"
    );

    env.cleanup();
}

// ---------------------------------------------------------------------------
// 2. UNTRUSTED REFUSAL
// ---------------------------------------------------------------------------

/// Same setup as (1) but WITHOUT `config trust`: the project layer must be
/// skipped (with a stderr warning naming `config trust`) and the plan shows
/// the reference value, not the override.
#[test]
fn production_untrusted_project_layer_skipped() {
    let env = ProdHome::new("untrusted-skipped");
    env.write_registry("[settings]\n");

    let proj = env.project_dir("untrusted");
    std::fs::write(
        proj.join("workestrate.toml"),
        "schema_version = 1\n\n[workloads.example-service]\ncpus = 7\n",
    )
    .expect("write project layer");

    let out = env.run(
        env.cmd()
            .args(["example-service", "plan"])
            .current_dir(&proj),
    );
    let stdout = expect_ok(&out, "example-service plan (untrusted)");
    let stderr = stderr_of(&out);

    // The gate warns and skips the layer (config.rs load_config step 4).
    assert!(
        stderr.contains("not trusted") && stderr.contains("config trust"),
        "stderr should carry the 'not trusted ... config trust' warning; got:\n{stderr}"
    );
    // Reference example-service has cpus = 2; the untrusted override (7)
    // must NOT appear anywhere in the plan.
    assert!(
        !stdout.contains("cpus: 7"),
        "untrusted project layer must NOT be applied; got:\n{stdout}"
    );
    assert!(
        stdout.contains("cpus: 2"),
        "plan should show the reference cpus=2; got:\n{stdout}"
    );

    env.cleanup();
}

// ---------------------------------------------------------------------------
// 3. 2-LAYER MERGE WITH PROVENANCE
// ---------------------------------------------------------------------------

/// Two stacked context layers [base, team]: team overrides a field base set
/// (memory_mib) and adds another (cpus); base contributes a base-only field
/// (workdir). `plan --show-source` must show merged values AND [layer]
/// provenance annotations naming the winning layer per field.
#[test]
fn production_two_layer_merge_with_provenance() {
    let env = ProdHome::new("two-layer-provenance");
    env.write_registry(&registry_bare_layers(&["base", "team"]));

    env.write_layer(
        "base",
        "schema_version = 1\n\n\
         [workloads.example-service]\n\
         memory_mib = 1024\n\
         workdir = \"/srv/base\"\n",
    );
    env.write_layer(
        "team",
        "schema_version = 1\n\n\
         [workloads.example-service]\n\
         memory_mib = 4096\n\
         cpus = 3\n",
    );

    // --show-source is a global clap flag; under verb-first dispatch (ADR
    // 0027) the workload name is a clap positional of `workload plan`, so
    // global flags parse in any position.
    let out = env.run(
        env.cmd()
            .args(["--show-source", "workload", "plan", "example-service"]),
    );
    let stdout = expect_ok(&out, "example-service plan --show-source");

    // Merged values: team wins memory_mib (last layer wins), team adds cpus,
    // base-only workdir survives.
    assert!(
        stdout.contains("memory: 4096 MiB"),
        "team memory_mib=4096 should win; got:\n{stdout}"
    );
    assert!(
        stdout.contains("cpus: 3"),
        "team cpus=3 should apply; got:\n{stdout}"
    );
    assert!(
        stdout.contains("workdir: /srv/base"),
        "base-only workdir should survive the merge; got:\n{stdout}"
    );

    // Provenance annotations: show_source pads each line then appends
    // `[layer]`; assert per-field attribution.
    let line_with = |needle: &str| -> String {
        stdout
            .lines()
            .find(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("no plan line containing '{needle}':\n{stdout}"))
            .to_string()
    };
    assert!(
        line_with("memory: 4096 MiB").ends_with("[team]"),
        "memory_mib must be attributed to team; line: {}",
        line_with("memory: 4096 MiB")
    );
    assert!(
        line_with("cpus: 3").ends_with("[team]"),
        "cpus must be attributed to team; line: {}",
        line_with("cpus: 3")
    );
    assert!(
        line_with("workdir: /srv/base").ends_with("[base]"),
        "workdir must be attributed to base; line: {}",
        line_with("workdir: /srv/base")
    );

    env.cleanup();
}

// ---------------------------------------------------------------------------
// 4. CONTEXT SELECTION
// ---------------------------------------------------------------------------

/// Registry defines two contexts (`personal` → [a], `work` → [b]) plus a
/// default_context. Without WORKESTRATE_CONTEXT the default wins; with
/// WORKESTRATE_CONTEXT=work the env-selected context's layer wins. Each
/// layer sets a distinct cpus value so the winner is observable in the plan.
#[test]
fn production_context_selection_env_overrides_default() {
    let env = ProdHome::new("context-selection");
    env.write_registry(
        "[settings]\ndefault_context = \"personal\"\n\n\
         [contexts.personal]\nlayers = [\"a\"]\n\n\
         [contexts.work]\nlayers = [\"b\"]\n\n\
         [configs.a]\nurl = \"file:///unused/a\"\nref = \"main\"\n\n\
         [configs.b]\nurl = \"file:///unused/b\"\nref = \"main\"\n",
    );
    env.write_layer(
        "a",
        "schema_version = 1\n\n[workloads.example-service]\ncpus = 3\n",
    );
    env.write_layer(
        "b",
        "schema_version = 1\n\n[workloads.example-service]\ncpus = 5\n",
    );

    // Default context (personal → layer a) applies when nothing selects one.
    let out = env.run(env.cmd().args(["example-service", "plan"]));
    let stdout = expect_ok(&out, "plan with default context");
    assert!(
        stdout.contains("cpus: 3"),
        "default context personal (layer a) should yield cpus=3; got:\n{stdout}"
    );
    assert!(
        !stdout.contains("cpus: 5"),
        "work context layer b must not leak into the default; got:\n{stdout}"
    );

    // WORKESTRATE_CONTEXT=work selects layer b instead.
    let mut cmd = env.cmd();
    cmd.args(["example-service", "plan"])
        .env("WORKESTRATE_CONTEXT", "work");
    let out = env.run(&mut cmd);
    let stdout = expect_ok(&out, "plan with WORKESTRATE_CONTEXT=work");
    assert!(
        stdout.contains("cpus: 5"),
        "WORKESTRATE_CONTEXT=work should select layer b (cpus=5); got:\n{stdout}"
    );
    assert!(
        !stdout.contains("cpus: 3"),
        "personal context layer a must not leak into work; got:\n{stdout}"
    );

    env.cleanup();
}

// ---------------------------------------------------------------------------
// 5. USER-GLOBAL OVERRIDES
// ---------------------------------------------------------------------------

/// A context layer sets cpus=3; <home>/overrides.toml [global.workloads.*]
/// sets cpus=6. User-global overrides sit between the context layers and the
/// project layer, so the override must win over the context layer.
/// A second assertion covers the per-config form
/// ([configs.<layer>.workloads.*]).
#[test]
fn production_user_global_overrides_win_over_context_layers() {
    let env = ProdHome::new("user-global-overrides");
    env.write_registry(&registry_bare_layers(&["base"]));
    env.write_layer(
        "base",
        "schema_version = 1\n\n[workloads.example-service]\ncpus = 3\n",
    );
    env.write_overrides("[global.workloads.example-service]\ncpus = 6\n");

    let out = env.run(env.cmd().args(["example-service", "plan"]));
    let stdout = expect_ok(&out, "plan with [global] override");
    assert!(
        stdout.contains("cpus: 6"),
        "user-global [global] override should win over the context layer; got:\n{stdout}"
    );
    assert!(
        !stdout.contains("cpus: 3"),
        "context layer value must be shadowed by the override; got:\n{stdout}"
    );

    env.cleanup();
}

/// Same precedence question for the per-config override form
/// `[configs.<layer>.workloads.<wl>]` (applies only when <layer> is in the
/// active context).
#[test]
fn production_per_config_override_applies_to_matching_layer() {
    let env = ProdHome::new("per-config-override");
    env.write_registry(&registry_bare_layers(&["base"]));
    env.write_layer(
        "base",
        "schema_version = 1\n\n[workloads.example-service]\ncpus = 3\n",
    );
    env.write_overrides("[configs.base.workloads.example-service]\ncpus = 8\n");

    let out = env.run(env.cmd().args(["example-service", "plan"]));
    let stdout = expect_ok(&out, "plan with [configs.base] override");
    assert!(
        stdout.contains("cpus: 8"),
        "[configs.base] override should apply (base is an active layer); got:\n{stdout}"
    );

    env.cleanup();
}

// ---------------------------------------------------------------------------
// 6. ENV UNION END-TO-END (WP6(a))
// ---------------------------------------------------------------------------

/// Base layer env A=1,B=2; a higher layer adds C=3 and overrides A=9. The
/// rendered plan's env lines must show A=9 exactly once (not A=1, not
/// duplicated), plus B=2 and C=3 — union-by-name through the real chain.
#[test]
fn production_env_union_by_name_end_to_end() {
    let env = ProdHome::new("env-union");
    env.write_registry(&registry_bare_layers(&["base", "team"]));

    env.write_layer(
        "base",
        "schema_version = 1\n\n\
         [workloads.example-service]\n\n\
         [[workloads.example-service.env]]\nname = \"A\"\nvalue = \"1\"\n\n\
         [[workloads.example-service.env]]\nname = \"B\"\nvalue = \"2\"\n",
    );
    env.write_layer(
        "team",
        "schema_version = 1\n\n\
         [workloads.example-service]\n\n\
         [[workloads.example-service.env]]\nname = \"A\"\nvalue = \"9\"\n\n\
         [[workloads.example-service.env]]\nname = \"C\"\nvalue = \"3\"\n",
    );

    let out = env.run(env.cmd().args(["example-service", "plan"]));
    let stdout = expect_ok(&out, "plan with env union");

    let count = |needle: &str| stdout.matches(needle).count();
    assert_eq!(
        count("env: A=9"),
        1,
        "A=9 must appear exactly once (union-by-name, last wins); got:\n{stdout}"
    );
    assert!(
        !stdout.contains("env: A=1"),
        "A=1 must be replaced, never rendered; got:\n{stdout}"
    );
    assert!(
        stdout.contains("env: B=2"),
        "B=2 (untouched base key) must survive; got:\n{stdout}"
    );
    assert!(
        stdout.contains("env: C=3"),
        "C=3 (new team key) must be appended; got:\n{stdout}"
    );

    env.cleanup();
}

// ---------------------------------------------------------------------------
// 7. SCHEMA_VERSION ERROR PATH
// ---------------------------------------------------------------------------

/// A context layer with `schema_version = 3` must fail validate_config at
/// config-load time — non-zero exit and the exact
/// `is not supported (expected 1)` message on stderr — before any KVM work.
/// (Spec 16: schema_version 1 is the native and only version; 2+ is
/// refused.)
#[test]
fn production_schema_version_3_refused_at_load() {
    let env = ProdHome::new("schema-version-refused");
    env.write_registry(&registry_bare_layers(&["bad"]));
    env.write_layer(
        "bad",
        "schema_version = 3\n\n[workloads.example-service]\ncpus = 3\n",
    );

    let out = env.run(env.cmd().args(["workload", "plan", "example-service"]));
    assert!(
        !out.status.success(),
        "schema_version=3 must fail; got success:\n{}",
        stdout_of(&out)
    );
    let stderr = stderr_of(&out);
    assert!(
        stderr.contains("is not supported (expected 1)"),
        "stderr must carry the schema_version refusal; got:\n{stderr}"
    );

    env.cleanup();
}

/// Same refusal via the (higher-precedence) project layer: trusted project
/// with schema_version = 3 outranks both reference and context layers.
#[test]
fn production_schema_version_3_refused_from_project_layer() {
    let env = ProdHome::new("schema-version-project");
    env.write_registry("[settings]\n");

    let proj = env.project_dir("schema3");
    std::fs::write(proj.join("workestrate.toml"), "schema_version = 3\n")
        .expect("write project layer");

    // Trust it so the layer actually loads (the gate must not mask the test).
    let out = env.run(env.cmd().args(["config", "trust"]).arg(&proj));
    expect_ok(&out, "config trust");

    let out = env.run(
        env.cmd()
            .args(["workload", "plan", "example-service"])
            .current_dir(&proj),
    );
    assert!(
        !out.status.success(),
        "schema_version=3 (project layer) must fail; got success:\n{}",
        stdout_of(&out)
    );
    let stderr = stderr_of(&out);
    assert!(
        stderr.contains("is not supported (expected 1)"),
        "stderr must carry the schema_version refusal; got:\n{stderr}"
    );

    env.cleanup();
}

// ---------------------------------------------------------------------------
// 8. FULL PLAN RENDER FROM A 2-LAYER FIXTURE
// ---------------------------------------------------------------------------

/// Two layers build up a full service workload (image, command, env, ports,
/// mounts, network); the default `plan` Display render must contain every
/// expected line for the merged result. This is the production-path analogue
/// of the committed golden render (which uses the WORKESTRATE_CONFIG_DIR
/// bypass) — here the bytes flow through reference → context layers → merge.
#[test]
fn production_full_plan_render_two_layer_fixture() {
    let env = ProdHome::new("full-render");
    env.write_registry(&registry_bare_layers(&["base", "team"]));

    // Base: image, command, one env var, one port, one mount, network base.
    env.write_layer(
        "base",
        "schema_version = 1\n\n\
         [workloads.example-service]\n\
         image = { recipe = \"registry\", ref = \"python:3.12-slim\" }\n\
         command = [\"python\", \"-m\", \"http.server\", \"9090\"]\n\n\
         [[workloads.example-service.env]]\n\
         name = \"APP_PORT\"\n\
         value = \"9090\"\n\n\
         [[workloads.example-service.ports]]\n\
         host = 9090\n\
         guest = 9090\n\n\
         [[workloads.example-service.mounts]]\n\
         host = \"workspaces/example-service-state\"\n\
         guest = \"/data\"\n\
         read_only = false\n\n\
         [workloads.example-service.network]\n\
         default_deny = true\n\n\
         [[workloads.example-service.network.egress]]\n\
         recipe = \"dns\"\n",
    );
    // Team: resources, an extra env var, an https egress recipe.
    env.write_layer(
        "team",
        "schema_version = 1\n\n\
         [workloads.example-service]\n\
         cpus = 4\n\
         memory_mib = 2048\n\n\
         [[workloads.example-service.env]]\n\
         name = \"TEAM_FLAG\"\n\
         value = \"on\"\n\n\
         [[workloads.example-service.network.egress]]\n\
         recipe = \"https\"\n\
         hosts = [\"github.com\"]\n",
    );

    let out = env.run(env.cmd().args(["example-service", "plan"]));
    let stdout = expect_ok(&out, "full plan render");

    // Instance name: bare-layers (no contexts) → the bare workload name.
    for needle in [
        "name: example-service",
        "image: python:3.12-slim",
        "command: python -m http.server 9090",
        "cpus: 4",
        "memory: 2048 MiB",
        "env: APP_PORT=9090",
        "env: TEAM_FLAG=on",
        "port: 9090:9090",
        "mount: workspaces/example-service-state:/data",
        "network: default_deny=true",
    ] {
        assert!(
            stdout.contains(needle),
            "rendered plan must contain '{needle}'; got:\n{stdout}"
        );
    }
    // Egress lines: dns recipe expands to a dns egress rule; the https
    // recipe expands to tcp:443 -> github.com.
    assert!(
        stdout.contains("egress:") && stdout.contains("github.com"),
        "plan must render the https egress rule for github.com; got:\n{stdout}"
    );

    env.cleanup();
}
