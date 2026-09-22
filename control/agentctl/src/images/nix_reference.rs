//! Nix source references for immutable fleet archives.

use std::path::{Component, Path};

/// Archives contain the committed fleet tree, without a Git checkout. Explicit
/// path references prevent Nix from filtering them through an operator's parent
/// repository. A nested flake retains the whole archive as its source so imports
/// above the capsule directory still resolve inside that committed tree.
pub(super) fn flake_reference(flake_root: &Path, attr: &str) -> String {
    let store = crate::config::archive_store_root();
    let store = match store.canonicalize() {
        Ok(canonical) => canonical,
        Err(_) => store,
    };
    reference_under_archive_store(flake_root, attr, &store)
}

fn reference_under_archive_store(flake_root: &Path, attr: &str, store: &Path) -> String {
    let ordinary = || format!("{}#{attr}", flake_root.display());
    let Ok(relative) = flake_root.strip_prefix(store) else {
        return ordinary();
    };
    let mut components = relative.components();
    let Some(Component::Normal(sha)) = components.next() else {
        return ordinary();
    };
    let Some(sha) = sha.to_str() else {
        return ordinary();
    };
    if crate::config::archive_dir(sha).is_err()
        || components
            .clone()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return ordinary();
    }
    let source = encode_path(&store.join(sha));
    let subdir = components.as_path();
    if subdir.as_os_str().is_empty() {
        format!("path:{source}#{attr}")
    } else {
        format!("path:{source}?dir={}#{attr}", encode_path(subdir))
    }
}

fn encode_path(path: &Path) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::new();
    for &byte in path.as_os_str().as_encoded_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 15)]));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::test_support::{ENV_TEST_LOCK, EnvGuard};
    use crate::images::detect::{DrvEvaluator, NixCliEvaluator};
    use crate::images::pipeline::{ImageBuilder, NixCliBuilder};
    use std::error::Error;
    use std::process::Command;

    const SHA: &str = "1234567890abcdef1234567890abcdef12345678";

    #[test]
    fn archive_references_keep_nested_imports_inside_one_source() -> Result<(), Box<dyn Error>> {
        let store = Path::new("/operator/state/cache/gitv3");
        let archive = store.join(SHA);
        assert_eq!(
            reference_under_archive_store(&archive, "image", store),
            format!("path:{}/{SHA}#image", store.display())
        );
        assert_eq!(
            reference_under_archive_store(
                &archive.join("capsules/code #1"),
                "image.drvPath",
                store
            ),
            format!(
                "path:{}/{SHA}?dir=capsules/code%20%231#image.drvPath",
                store.display()
            )
        );
        for path in [
            store.to_path_buf(),
            store.join("main"),
            store.join(format!("{SHA}.tmp-1")),
            store.join(SHA).join("../outside"),
            Path::new("/operator/fleets/work/capsules/code").to_path_buf(),
        ] {
            assert_eq!(
                reference_under_archive_store(&path, "image", store),
                format!("{}#image", path.display())
            );
        }
        assert_eq!(
            encode_path(Path::new("/operator #1/cache?")),
            "/operator%20%231/cache%3F"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    #[allow(unsafe_code)]
    fn eval_and_build_share_archive_source_and_preserve_git_checkouts() -> Result<(), Box<dyn Error>>
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let _lock = ENV_TEST_LOCK
            .lock()
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let _environment = EnvGuard::capture(&["WORKESTRATE_STATE_DIR"]);
        let tmp = tempfile::tempdir()?;
        let real_state = tmp.path().join("real-state");
        std::fs::create_dir(&real_state)?;
        let state_alias = tmp.path().join("state-alias");
        symlink(&real_state, &state_alias)?;
        // SAFETY: ENV_TEST_LOCK serializes this mutation and its restoration.
        unsafe { std::env::set_var("WORKESTRATE_STATE_DIR", &state_alias) };
        let archive = real_state.join("cache/gitv3").join(SHA);
        let nested = archive.join("capsules/code");
        let checkout = tmp.path().join("checkout");
        std::fs::create_dir_all(&nested)?;
        std::fs::create_dir(&checkout)?;
        let program = tmp.path().join("nix");
        std::fs::write(
            &program,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$0.args\"\nprintf '%s\\n' --END-- >> \"$0.args\"\nprintf '%s\\n' /nix/store/archive-fixture\n",
        )?;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))?;
        let mut evaluator = NixCliEvaluator::with_program(&program);
        let mut builder = NixCliBuilder::with_program(&program);
        for root in [&archive, &nested, &checkout] {
            evaluator.eval_drv_path(root, "image")?;
            evaluator.eval_out_path(root, "image")?;
            builder.build_out_path(root, "image")?;
        }
        let argv = std::fs::read_to_string(program.with_extension("args"))?;
        let references: Vec<&str> = argv
            .lines()
            .filter(|line| line.contains("#image"))
            .collect();
        let archive_ref = format!("path:{}", archive.display());
        let nested_ref = format!("{archive_ref}?dir=capsules/code");
        let checkout_ref = checkout.display().to_string();
        let expected: Vec<String> = [archive_ref, nested_ref, checkout_ref]
            .into_iter()
            .flat_map(|source| {
                [".drvPath", ".outPath", ""].map(|suffix| format!("{source}#image{suffix}"))
            })
            .collect();
        assert_eq!(references, expected);
        assert_eq!(
            argv.lines()
                .filter(|line| *line == "--no-update-lock-file")
                .count(),
            9
        );
        Ok(())
    }

    fn git(directory: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(directory)
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()?;
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    #[test]
    #[ignore = "requires a local Nix store; run the exported native fixture with Nix on PATH"]
    #[allow(unsafe_code)]
    fn archived_flakes_evaluate_and_build_without_touching_parent_index()
    -> Result<(), Box<dyn Error>> {
        use sha2::{Digest, Sha256};
        let _lock = ENV_TEST_LOCK
            .lock()
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let _environment = EnvGuard::capture(&["WORKESTRATE_STATE_DIR"]);
        let tmp = tempfile::tempdir()?;
        let operator = tmp.path().join("operator");
        let fleet = tmp.path().join("fleet");
        let capsule = fleet.join("capsules/code");
        std::fs::create_dir(&operator)?;
        std::fs::create_dir_all(&capsule)?;
        std::fs::write(operator.join(".gitignore"), "/state/\n")?;
        git(&operator, &["init", "--quiet"])?;
        git(&operator, &["add", ".gitignore"])?;
        git(&operator, &["commit", "--quiet", "-m", "fixture"])?;
        let index_before = std::fs::read(operator.join(".git/index"))?;
        let payload = "archive image fixture\n";
        let hash = hex::encode(Sha256::digest(payload.as_bytes()));
        std::fs::write(fleet.join("payload.txt"), payload)?;
        std::fs::write(
            fleet.join("image.nix"),
            format!(
                r#"{{ payload }}: builtins.derivation {{
  name = "archive-image-fixture";
  system = "x86_64-linux";
  builder = "builtin:fetchurl";
  url = "file://${{payload}}";
  outputHashMode = "flat";
  outputHashAlgo = "sha256";
  outputHash = "{hash}";
}}
"#
            ),
        )?;
        let lock = "{\"nodes\":{\"root\":{}},\"root\":\"root\",\"version\":7}\n";
        for (root, prefix) in [(&fleet, "./"), (&capsule, "../../")] {
            std::fs::write(
                root.join("flake.nix"),
                format!(
                    "{{ outputs = {{ self }}: {{ image = import {prefix}image.nix {{ payload = {prefix}payload.txt; }}; }}; }}\n"
                ),
            )?;
            std::fs::write(root.join("flake.lock"), lock)?;
        }
        git(&fleet, &["init", "--quiet"])?;
        git(&fleet, &["add", "."])?;
        git(&fleet, &["commit", "--quiet", "-m", "fixture"])?;
        std::fs::write(
            fleet.join("untracked-private.txt"),
            "excluded from archive\n",
        )?;
        let revision = git(&fleet, &["rev-parse", "HEAD"])?;
        // SAFETY: ENV_TEST_LOCK serializes this mutation and its restoration.
        unsafe { std::env::set_var("WORKESTRATE_STATE_DIR", operator.join("state")) };
        let archive = crate::config::ensure_archive(&fleet, &revision)?;
        assert!(!archive.join("untracked-private.txt").exists());
        let bare = Command::new("nix")
            .args([
                "eval",
                "--raw",
                "--offline",
                "--no-update-lock-file",
                "--extra-experimental-features",
                "nix-command flakes",
            ])
            .arg(format!("{}#image.drvPath", archive.display()))
            .current_dir(&archive)
            .output()?;
        assert!(
            !bare.status.success(),
            "the ignored archive must not be visible through its parent Git repository"
        );
        let bare_stderr = String::from_utf8_lossy(&bare.stderr);
        assert!(
            bare_stderr.contains("not tracked by Git"),
            "unexpected bare-reference failure: {bare_stderr}"
        );
        let mut evaluator = NixCliEvaluator::new();
        let mut builder = NixCliBuilder::new();
        let drv = evaluator.eval_drv_path(&archive, "image")?;
        let out = evaluator.eval_out_path(&archive, "image")?;
        assert_eq!(builder.build_out_path(&archive, "image")?, out);
        let nested = archive.join("capsules/code");
        assert_eq!(evaluator.eval_drv_path(&nested, "image")?, drv);
        assert_eq!(evaluator.eval_out_path(&nested, "image")?, out);
        assert_eq!(builder.build_out_path(&nested, "image")?, out);
        assert_eq!(std::fs::read_to_string(out)?, payload);
        assert_eq!(std::fs::read(archive.join("flake.lock"))?, lock.as_bytes());
        assert_eq!(std::fs::read(nested.join("flake.lock"))?, lock.as_bytes());
        assert_eq!(std::fs::read(operator.join(".git/index"))?, index_before);
        assert!(git(&operator, &["status", "--porcelain"])?.is_empty());
        Ok(())
    }
}
