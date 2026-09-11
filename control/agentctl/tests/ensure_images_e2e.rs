//! REAL end-to-end test of the spec-21 phase-E ensure-images pre-flight
//! wiring (`images::ensure` in the lifecycle verbs), driven through the REAL
//! `workestrate` binary against a temp-homed msb store and a temp config
//! repo whose flake builds the same ~20 KiB fixture image as the phase-D e2e
//! (zero network FODs).
//!
//! What is asserted (all via pre-spawn observable side effects — NO KVM
//! needed, sandbox boot is never required):
//!
//! 1. **Parent ensures, detached child skips (§2.1/§2.2).** `workload up
//!    svc-dep` from a flake-less cwd: the parent runs the ensure pass
//!    (stderr line + `images.json` record + store tag) BEFORE spawning; the
//!    detached child's log shows it reached `build_sandbox` (it fails at the
//!    F2 lazy flake-root gate, which needs no KVM) WITHOUT any
//!    `ensure-images:` line — the `--images-ready` token suppressed the
//!    child-side pre-flight (no double nix eval / store probe inside the
//!    FS-8 window).
//! 2. **Skew matrix through the verbs.** Second plain `up` → `skip`;
//!    `--reload-images` → `rebuild (forced)` with the §3.1 gate-skip note.
//! 3. **Dep auto-start inherits ensure (§2.1/§2.4).** `up svc-top` ensures
//!    the NAMED workload FIRST (fail fast), then the dep's own ensure runs
//!    (unforced) before its spawn.
//! 4. **Batch force scope (§5.2, USER DECISION D3).** Bare `workload up
//!    --reload-images` forces EVERY eligible service workload in the batch.
//!
//! ENV GATES (skip-with-note, the phase-D e2e precedent):
//!
//! - `nix` on PATH (real drvPath evals + builds);
//! - `MSB_PATH` pointing at an msb binary that HONORS `MSB_HOME` — NOT the
//!   devshell's wrapped msb (it forces `MSB_HOME=$HOME/.microsandbox`;
//!   running through the wrapper would write fixture images into the REAL
//!   home store);
//! - `gunzip` on PATH.
//!
//! The `#[ignore]`'d test at the bottom is the HOST-KVM variant: it lets the
//! detached child actually BOOT the freshly-rebuilt image (create() sees the
//! loaded tag — the §11 item-2 host remainder), gated honestly like
//! `lifecycle_detached.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use workestrate::images::state::{ImagesState, image_key};

const BIN: &str = env!("CARGO_BIN_EXE_workestrate");

fn nix_on_path() -> bool {
    std::process::Command::new("nix")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn gunzip_on_path() -> bool {
    std::process::Command::new("gunzip")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// An msb binary that honors MSB_HOME (see the module docs for why the
/// devshell's wrapped `msb` is NOT acceptable here).
fn msb_for_e2e() -> Option<String> {
    let bin = std::env::var("MSB_PATH").ok()?;
    let ok = std::process::Command::new(&bin)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok { Some(bin) } else { None }
}

fn uniq_tmp(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "workestrate-ensure-e2e-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

/// The e2e fixture flake: two TOP-LEVEL image attrs (the config-repo flake
/// shape — `image.name` is the verbatim flake attr), both zero-FOD
/// `dockerTools.buildLayeredImage`s of a static text file. `-b` carries
/// different content so the two tags are distinct derivations.
const E2E_FLAKE: &str = r#"{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/a799d3e3886da994fa307f817a6bc705ae538eeb";
  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      wk-e2e-image = pkgs.dockerTools.buildLayeredImage {
        name = "wk-e2e-image";
        tag = "latest";
        contents = [ (pkgs.writeTextDir "/hello.txt" "hello from svc-dep\n") ];
      };
      wk-e2e-image-b = pkgs.dockerTools.buildLayeredImage {
        name = "wk-e2e-image-b";
        tag = "latest";
        contents = [ (pkgs.writeTextDir "/hello.txt" "hello from svc-top\n") ];
      };
    };
}
"#;

/// svc-dep (nix-layered, one declared port) and svc-top (nix-layered,
/// depends_on svc-dep). Both run `true` — the tests never need a live
/// service, only the pre-spawn lifecycle.
const E2E_CONFIG_TOML: &str = r#"
schema_version = 1

[workloads.svc-dep]
kind = "service"
image = { recipe = "nix-layered", name = "wk-e2e-image", tag = "latest" }
command = ["true"]
log_stop_errors = true

[[workloads.svc-dep.ports]]
host = 45871
guest = 45871

[workloads.svc-dep.network.defaults]
egress = "deny"

[workloads.svc-top]
kind = "service"
image = { recipe = "nix-layered", name = "wk-e2e-image-b", tag = "latest" }
command = ["true"]
log_stop_errors = true

[workloads.svc-top.depends_on.svc-dep]
env = "DEP_URL"

[workloads.svc-top.network.defaults]
egress = "deny"
"#;

struct E2eFixture {
    tmp: PathBuf,
    home: PathBuf,
    repo: PathBuf,
    cwd: PathBuf,
    msb_home: PathBuf,
}

impl E2eFixture {
    fn new(label: &str) -> Self {
        let tmp = uniq_tmp(label);
        let home = tmp.join("home");
        let repo = tmp.join("repo");
        let cwd = tmp.join("cwd");
        let msb_home = common::short_msb_home();
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::create_dir_all(&msb_home).unwrap();
        std::fs::write(repo.join("flake.nix"), E2E_FLAKE).unwrap();
        // The committed fixture lock pins nixpkgs to this repo's flake.lock
        // rev (already realized in the store) — eval+build need no network.
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("image-flake")
                .join("flake.lock"),
            repo.join("flake.lock"),
        )
        .unwrap();
        std::fs::write(repo.join("workestrate.toml"), E2E_CONFIG_TOML).unwrap();
        assert!(
            !cwd.join("flake.nix").exists(),
            "the run cwd must be flake-less (the F2-gate short-circuit)"
        );
        Self {
            tmp,
            home,
            repo,
            cwd,
            msb_home,
        }
    }

    /// A `workestrate` [`Command`] fully isolated from the operator's real
    /// home/msb store: temp HOME (+ XDG dirs), temp MSB_HOME, temp
    /// WORKESTRATE_HOME, the fixture repo as the single config layer
    /// (WORKESTRATE_CONFIG_DIR), MSB_PATH to the unwrapped msb, and every
    /// project_root env tier scrubbed so the flake-less `cwd` decides.
    fn cmd(&self, msb: &str) -> Command {
        let mut c = Command::new(BIN);
        c.current_dir(&self.cwd);
        c.env("HOME", &self.home);
        c.env("XDG_CONFIG_HOME", self.home.join(".config"));
        c.env("XDG_DATA_HOME", self.home.join(".local").join("share"));
        c.env("XDG_STATE_HOME", self.home.join(".local").join("state"));
        c.env("WORKESTRATE_HOME", self.home.join("wk"));
        c.env("WORKESTRATE_CONFIG_DIR", &self.repo);
        c.env("MSB_HOME", &self.msb_home);
        c.env("MSB_PATH", msb);
        c.env_remove("AGENTCTL_ROOT");
        c.env_remove("CARGO_MANIFEST_DIR");
        c.env_remove("WORKESTRATE_STATE_DIR");
        c.env_remove("WORKESTRATE_NO_PROJECT_CONFIG");
        c.env_remove("WORKESTRATE_CONTEXT");
        c
    }

    fn state_dir(&self) -> PathBuf {
        self.home.join("wk").join("state")
    }

    /// Records in `images.json` whose `tag` matches (values of the
    /// `<repo>#<tag>` map; the repo_key is the repo's canonical path in
    /// config-dir mode).
    fn records_for(&self, tag: &str) -> Vec<workestrate::images::state::ImageRecord> {
        ImagesState::load(&self.state_dir())
            .images
            .values()
            .filter(|r| r.tag == tag)
            .cloned()
            .collect()
    }

    /// A2 (ADR 0032 §Image tags): learn the content-addressed tag an ensure
    /// pass loaded for `attr` from the images.json record (the sha segment
    /// is not knowable before the eval/build).
    fn tag_for_attr(&self, attr: &str) -> Option<String> {
        ImagesState::load(&self.state_dir())
            .images
            .values()
            .find(|r| r.attr == attr)
            .map(|r| r.tag.clone())
    }

    /// The detached child's log for `slot` (svc-dep/svc-top — no context).
    /// Post-074fc02 the child logs to the state dir
    /// (`<state_dir>/logs/<instance>/workestrate.log`); the fixture pins
    /// WORKESTRATE_HOME (Env home kind), so the state dir is
    /// `<WORKESTRATE_HOME>/state`.
    fn child_log(&self, slot: &str) -> String {
        let path = self
            .home
            .join("wk")
            .join("state")
            .join("logs")
            .join(slot)
            .join("workestrate.log");
        std::fs::read_to_string(path).unwrap_or_default()
    }
}

impl Drop for E2eFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.msb_home);
        let _ = std::fs::remove_dir_all(&self.tmp);
    }
}

/// Poll until `cond(log)` holds for the child's log (or timeout); returns
/// the final log content.
fn poll_child_log(
    fx: &E2eFixture,
    slot: &str,
    timeout: Duration,
    cond: impl Fn(&str) -> bool,
) -> String {
    let deadline = Instant::now() + timeout;
    loop {
        let log = fx.child_log(slot);
        if cond(&log) || Instant::now() >= deadline {
            return log;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Store-tag presence via the msb CLI (NOT the SDK — the test process must
/// not pin the SDK's process-global DB pool on the temp MSB_HOME).
fn store_has_tag(fx: &E2eFixture, msb: &str, tag: &str) -> bool {
    Command::new(msb)
        .args(["image", "inspect", tag])
        .env("MSB_HOME", &fx.msb_home)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// The full phase-E lifecycle through the real CLI: parent-ensures /
/// child-skips, skew verbs, dep inheritance, and the batch force scope.
#[test]
fn ensure_images_parent_child_token_reload_and_batch_scope() {
    if !nix_on_path() {
        eprintln!("note: nix not on PATH; skipping the phase-E ensure e2e");
        return;
    }
    if !gunzip_on_path() {
        eprintln!("note: gunzip not on PATH; skipping the phase-E ensure e2e");
        return;
    }
    let Some(msb) = msb_for_e2e() else {
        eprintln!(
            "note: MSB_PATH is not set to a working msb binary; skipping the phase-E \
             ensure e2e (set MSB_PATH to an msb that HONORS MSB_HOME — NOT the \
             devshell's wrapped msb, which forces MSB_HOME to the real home store)"
        );
        return;
    };
    let fx = E2eFixture::new("lifecycle");

    // ---- Step 1: parent ensures BEFORE spawn; the detached child skips.
    let out = fx
        .cmd(&msb)
        .args(["workload", "up", "svc-dep"])
        .output()
        .expect("spawn workload up svc-dep");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        stderr.contains("ensure-images: wk-e2e-image:")
            && stderr.contains(": build — built+loaded+recorded"),
        "the parent ran the ensure pass before spawning; stderr:\n{stderr}"
    );
    // A2: the loaded tag is the computed content tag `wk-e2e-image:<sha>`
    // (no ctx in config-dir single-layer mode) — learn it from the record.
    let tag = fx
        .tag_for_attr("wk-e2e-image")
        .expect("the parent's ensure wrote the images.json record");
    assert!(
        tag.starts_with("wk-e2e-image:") && !tag.ends_with(":latest"),
        "A2 content-addressed tag, not the declared `latest`: {tag}"
    );
    let record = fx.records_for(&tag).into_iter().next().unwrap();
    assert!(
        !record.out_path.is_empty(),
        "a full phase-D record: {record:?}"
    );
    // A2: the current-pointer moved to the same tag.
    let pointers = &ImagesState::load(&fx.state_dir()).pointers;
    assert!(
        pointers.values().any(|p| p.tag == tag),
        "the current-pointer record moved to {tag}; pointers: {pointers:?}"
    );
    assert!(
        store_has_tag(&fx, &msb, &tag),
        "the store holds the freshly-loaded tag (ground truth, spec §3.2)"
    );

    // The detached child (token) skips ensure and reaches build_sandbox —
    // evidenced by its F2 flake-root failure (the run cwd is flake-less)
    // with NO ensure-images line anywhere in its log.
    let log = poll_child_log(&fx, "svc-dep", Duration::from_secs(30), |l| {
        l.contains("requires a flake project root")
    });
    assert!(
        log.contains("===== workestrate "),
        "the child ran (spawn delimiter present); log:\n{log}"
    );
    assert!(
        log.contains("uses nix-layered image, which requires a flake project root"),
        "the child reached build_sandbox (F2 gate — no KVM needed); log:\n{log}"
    );
    assert!(
        !log.contains("ensure-images:"),
        "REGRESSION: the detached child re-ran the ensure pre-flight despite \
         the --images-ready token (double-build inside the FS-8 window); log:\n{log}"
    );

    // ---- Step 2: plain up → the content-addressed skip (same evaluated
    // out_path → same tag → fresh record → skip; no rebuild churn).
    let out = fx
        .cmd(&msb)
        .args(["workload", "up", "svc-dep"])
        .output()
        .expect("spawn workload up svc-dep (2)");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        stderr.contains(&format!("ensure-images: {tag}: skip — none (up to date)")),
        "fresh record + tag present → skip (§3.4 row 1); stderr:\n{stderr}"
    );

    // ---- Step 3: --reload-images forces the rebuild; the §3.1 outPath
    // gate then skips the redundant msb load.
    let out = fx
        .cmd(&msb)
        .args(["workload", "up", "svc-dep", "--reload-images"])
        .output()
        .expect("spawn workload up svc-dep --reload-images");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        stderr.contains(&format!(
            "ensure-images: {tag}: rebuild (forced) — image unchanged in store; tag already current"
        )),
        "--reload-images flips the matrix to a forced rebuild (§5.2); stderr:\n{stderr}"
    );

    // ---- Step 4: dep auto-start inherits ensure (§2.1); the NAMED
    // workload is ensured FIRST (§2.4 fail-fast ordering).
    let out = fx
        .cmd(&msb)
        .args(["workload", "up", "svc-top"])
        .output()
        .expect("spawn workload up svc-top");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let tag_b = fx
        .tag_for_attr("wk-e2e-image-b")
        .expect("the dep-inherited pass recorded svc-top's image");
    let named = stderr.find(&format!(
        "ensure-images: {tag_b}: build — built+loaded+recorded"
    ));
    let dep = stderr.find(&format!("ensure-images: {tag}: skip — none (up to date)"));
    assert!(
        named.is_some(),
        "svc-top's own image was ensured (build); stderr:\n{stderr}"
    );
    assert!(
        dep.is_some(),
        "the dep's ensure ran (inherited, unforced → skip); stderr:\n{stderr}"
    );
    assert!(
        named.unwrap() < dep.unwrap(),
        "the NAMED workload is ensured BEFORE dependency auto-start (§2.4); stderr:\n{stderr}"
    );

    // ---- Step 5: bare up + --reload-images forces EVERY eligible service
    // workload in the batch (USER DECISION D3), all before any spawn.
    let out = fx
        .cmd(&msb)
        .args(["workload", "up", "--reload-images"])
        .output()
        .expect("spawn bare workload up --reload-images");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    for tag in [&tag, &tag_b] {
        assert!(
            stderr.contains(&format!("ensure-images: {tag}: rebuild (forced)")),
            "D3 batch scope: {tag} forced by the batch --reload-images; stderr:\n{stderr}"
        );
    }
}

/// The bare-up flag-reject loop (no nix/msb needed — the rejection fires
/// before any config or ensure work): `--images-ready` and the name-scoped
/// flags are hard errors; `--reload-images` is ACCEPTED (it proceeds past
/// the guard to the batch command, which then fails later in this
/// unprovisioned env — never with the flag-reject message).
#[test]
fn bare_up_rejects_name_scoped_flags_accepts_reload_images() {
    let fx = E2eFixture::new("reject");
    for flag in [
        "--images-ready",
        "--foreground",
        "--replace",
        "--new",
        "--port-auto",
        "--no-deps",
    ] {
        let out = fx
            .cmd("msb-unused")
            .args(["workload", "up", flag])
            .output()
            .expect("spawn bare up with a rejected flag");
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        assert!(!out.status.success(), "{flag} must fail on bare up");
        assert!(
            stderr.contains(&format!(
                "{flag} is not meaningful for bare `workestrate workload up`"
            )),
            "{flag} must hit the reject loop; stderr:\n{stderr}"
        );
    }
    // --reload-images passes the guard. With a nix-layered fixture repo the
    // batch ensure then needs nix+msb, so only assert on the ABSENCE of the
    // reject message (the downstream failure mode depends on the env).
    let out = fx
        .cmd("msb-unused")
        .args(["workload", "up", "--reload-images"])
        .output()
        .expect("spawn bare up --reload-images");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        !stderr.contains("is not meaningful for bare"),
        "--reload-images must be accepted on bare up (D3); stderr:\n{stderr}"
    );
}

/// HOST-KVM variant (spec §10 phase-E gate): after an image CONTENT edit,
/// `up` rebuilds BEFORE spawn and the detached child actually BOOTS the
/// rebuilt image — create() sees the freshly-loaded tag (the §11 item-2 host
/// remainder: no docker.io pull fallback). Run on a KVM host with nix +
/// MSB_PATH set:
///
/// ```text
/// cargo test --manifest-path control/agentctl/Cargo.toml \
///   --test ensure_images_e2e -- --ignored --nocapture
/// ```
///
/// Honest fixture note: this e2e's flake is not TOML-generated (production
/// config repos derive the image derivation from the TOML), so the "image
/// edit" is a CONTENT edit in the fixture flake. A2 (ADR 0032 §Image tags):
/// the capsule's declared `tag` field is INERT — editing `tag = "latest"` →
/// `"v2"` changes NOTHING (the store tag is the computed content tag
/// `<name>:<sha>`); a content edit changes the evaluated outPath → a NEW
/// sha tag → absent record + absent tag → build+load, which is exactly the
/// rebuild-before-spawn path under test.
#[test]
#[ignore = "HOST-KVM: boots a real sandbox from the rebuilt image; needs KVM + nix + MSB_PATH (unwrapped msb)"]
fn kvm_up_after_image_content_edit_rebuilds_before_spawn() {
    assert!(nix_on_path(), "KVM e2e needs nix on PATH");
    let Some(msb) = msb_for_e2e() else {
        panic!("KVM e2e needs MSB_PATH to an unwrapped msb");
    };
    let fx = E2eFixture::new("kvm");

    // First up: parent ensures (build+load+record) and the child boots.
    // The child inherits the flake-less cwd BUT the F2 gate resolves via
    // AGENTCTL_ROOT pointed at the fixture repo (tier 1) — the ensure
    // pre-flight itself needs no flake root beyond the declaring repo's.
    let mut up = fx.cmd(&msb);
    up.env("AGENTCTL_ROOT", &fx.repo);
    let out = up
        .args(["workload", "up", "svc-dep"])
        .output()
        .expect("spawn workload up svc-dep (KVM)");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        out.status.success(),
        "detached up must succeed on a KVM host; stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("Sandbox 'svc-dep' started in background"),
        "the child booted; stdout:\n{stdout}"
    );
    let named_pos = stderr
        .find("ensure-images: wk-e2e-image:")
        .unwrap_or(usize::MAX);
    assert!(
        named_pos < usize::MAX,
        "the ensure pass ran before the spawn; stderr:\n{stderr}"
    );
    let tag_v1 = fx
        .tag_for_attr("wk-e2e-image")
        .expect("the first up recorded the image");
    let log = poll_child_log(&fx, "svc-dep", Duration::from_secs(90), |l| {
        l.contains("started (Ctrl-C to stop)") || l.contains("exited")
    });
    assert!(
        !log.contains("ensure-images:"),
        "the booted child carried the token and skipped ensure; log:\n{log}"
    );

    // A2: a CONTENT edit in the flake (the declared `tag` field is inert).
    // The next up must REBUILD (new outPath → new sha tag: absent record +
    // absent tag → build+load) BEFORE the spawn.
    // E2E_FLAKE is a raw string, so the Nix `\n` escape is two literal
    // chars on disk; the needle (and replacement) must be raw strings too,
    // or the replace matches nothing and the edit silently no-ops (the
    // 2026-08-29 host failure at the assert below).
    let flake = std::fs::read_to_string(fx.repo.join("flake.nix")).unwrap();
    let edited = flake.replace(r"hello from svc-dep\n", r"hello again from svc-dep v2\n");
    assert_ne!(flake, edited, "the flake edit applied");
    std::fs::write(fx.repo.join("flake.nix"), edited).unwrap();

    let mut up = fx.cmd(&msb);
    up.env("AGENTCTL_ROOT", &fx.repo);
    let out = up
        .args(["workload", "up", "svc-dep"])
        .output()
        .expect("spawn workload up svc-dep after content edit");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        stderr.contains("ensure-images: wk-e2e-image:")
            && stderr.contains(": build — built+loaded+recorded"),
        "the content edit triggered a rebuild BEFORE spawn; stderr:\n{stderr}"
    );
    assert!(
        out.status.success() && stdout.contains("started in background"),
        "the rebuilt image booted (create() saw the fresh tag — no docker.io \
         pull fallback); stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let tag_v2 = fx
        .tag_for_attr("wk-e2e-image")
        .expect("the rebuild recorded the image");
    assert_ne!(
        tag_v1, tag_v2,
        "a content edit yields a NEW content-addressed tag"
    );
    assert!(
        store_has_tag(&fx, &msb, &tag_v2),
        "the store holds the rebuilt tag"
    );
    let key_found = ImagesState::load(&fx.state_dir())
        .images
        .keys()
        .any(|k| k == &image_key(&fx.repo.canonicalize().unwrap().to_string_lossy(), &tag_v2));
    assert!(key_found, "the v2 record landed in images.json");

    // Best-effort teardown.
    let _ = fx.cmd(&msb).args(["workload", "down", "svc-dep"]).output();
}
