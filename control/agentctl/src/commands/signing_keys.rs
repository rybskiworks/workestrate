//! Explicit, single-target provisioning of SOPS-encrypted SSH signing keys.
//! Only ciphertext is staged on disk. SOPS owns its encrypted format and
//! recipient metadata; this command never enables runtime signing authority.

use anyhow::{Context, Result, ensure};
use clap::Subcommand;
use serde::Serialize;
use ssh_key::{Algorithm, HashAlg, LineEnding, PrivateKey, rand_core::OsRng};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use zeroize::{Zeroize, Zeroizing};

use super::secrets_target::{SecretsTarget, resolve_secrets_target};

const MAX_CIPHERTEXT: u64 = 8 * 1024 * 1024;
const MAX_OUTPUT: u64 = 16 * 1024 * 1024;

#[derive(Subcommand)]
pub enum CredentialAction {
    /// Generate or inspect signing keys in one encrypted fleet secrets file
    Signing {
        #[command(subcommand)]
        action: SigningKeyAction,
    },
}

#[derive(Subcommand)]
pub enum SigningKeyAction {
    /// Add a new Ed25519 private key to an existing SOPS file; never overwrite
    Generate {
        /// Environment name storing the key, e.g. MACHINE_GIT_SIGNING_KEY
        name: String,
        /// Explicit registered fleet write target, not an effective context
        #[arg(long, required = true)]
        fleet: String,
    },
    /// Print only the public key and fingerprint of an existing signing key
    PublicKey {
        /// Environment name storing the private key
        name: String,
        #[arg(long, required = true)]
        fleet: String,
    },
}

#[derive(Debug, Serialize)]
struct PublicKeyInfo {
    name: String,
    fleet: String,
    secrets_file: PathBuf,
    public_key: String,
    fingerprint: String,
    created: bool,
}

struct SecretValues(HashMap<String, String>);

impl Drop for SecretValues {
    fn drop(&mut self) {
        for value in self.0.values_mut() {
            value.zeroize();
        }
    }
}

pub fn run(action: CredentialAction, json: bool) -> Result<()> {
    let CredentialAction::Signing { action } = action;
    let (name, fleet, create) = match action {
        SigningKeyAction::Generate { name, fleet } => (name, fleet, true),
        SigningKeyAction::PublicKey { name, fleet } => (name, fleet, false),
    };
    valid_name(&name)?;
    let target = resolve_secrets_target(&fleet)?;
    let info = provision(&target, &name, &fleet, create)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("{}", info.public_key);
        eprintln!(
            "{}: {} ({})",
            info.name,
            info.fingerprint,
            info.secrets_file.display()
        );
        if info.created {
            eprintln!(
                "Encrypted key saved. Register the public key on the intended machine account as a signing key; workload grants and Git configuration are unchanged."
            );
        }
    }
    Ok(())
}

fn valid_name(name: &str) -> Result<()> {
    let mut bytes = name.bytes();
    ensure!(
        name.len() <= 128
            && matches!(bytes.next(), Some(b) if b.is_ascii_alphabetic() || b == b'_')
            && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_'),
        "signing key name must be an environment variable name (at most 128 bytes)"
    );
    ensure!(
        !name.to_ascii_lowercase().starts_with("sops_"),
        "reserved SOPS metadata name"
    );
    Ok(())
}

fn owned_path(path: &Path, directory: bool) -> Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("required path unavailable: {}", path.display()))?;
    // A caller-selected secret target must not traverse symlinks or accept
    // another user's writable files. Ciphertext may be publicly readable.
    ensure!(
        if directory {
            metadata.is_dir()
        } else {
            metadata.is_file()
        },
        "expected an ordinary {}: {}",
        if directory { "directory" } else { "file" },
        path.display()
    );
    ensure!(
        path.canonicalize()? == path,
        "indirect path refused: {}",
        path.display()
    );
    ensure!(
        metadata.uid() == current_uid() && metadata.mode() & 0o022 == 0,
        "path is not exclusively owner-writable: {}",
        path.display()
    );
    Ok(metadata)
}

#[allow(unsafe_code)]
fn current_uid() -> u32 {
    // SAFETY: geteuid has no arguments or memory preconditions.
    unsafe { libc::geteuid() }
}

fn ciphertext(path: &Path) -> Result<Vec<u8>> {
    let metadata = owned_path(path, false)?;
    ensure!(
        metadata.len() <= MAX_CIPHERTEXT,
        "encrypted secrets file exceeds size limit"
    );
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_CIPHERTEXT + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_CIPHERTEXT,
        "encrypted secrets file exceeds size limit"
    );
    Ok(bytes)
}

fn provision(
    target: &SecretsTarget,
    name: &str,
    fleet: &str,
    create: bool,
) -> Result<PublicKeyInfo> {
    disable_core_dumps()?;
    valid_name(name)?;
    owned_path(&target.dir, true)?;
    ensure!(
        !target.secrets_file.is_empty()
            && Path::new(&target.secrets_file)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "secrets file must be a relative path inside the selected fleet"
    );
    let path = target.dir.join(&target.secrets_file);
    let parent = path.parent().context("secrets file has no parent")?;
    owned_path(parent, true)?;
    let key_metadata = owned_path(&target.age_key_file, false)?;
    ensure!(
        key_metadata.mode() & 0o077 == 0,
        "age private key must have mode 0600 or stricter"
    );
    let before = ciphertext(&path)
        .context("initialize this fleet's encrypted secrets before provisioning a signing key")?;
    let sops = Sops {
        age_key_file: target.age_key_file.clone(),
    };

    if !create {
        let values = sops.decrypt(&path)?;
        let key = existing_key(&values, name)?;
        return public_info(&key, name, fleet, path, false);
    }

    // Directory flock serializes these short transactions without creating a
    // permanent lock artifact in the fleet. Other editors need
    // not use this lock; their observed changes are checked before replacement.
    let lock = File::open(parent)?;
    lock.try_lock()
        .context("another signing-key transaction holds this fleet directory")?;
    ensure!(
        ciphertext(&path)? == before,
        "encrypted file changed before key generation; retry"
    );
    let values = sops.decrypt(&path)?;
    ensure!(
        !values.0.contains_key(name),
        "secret '{name}' already exists; refusing to rotate or overwrite it"
    );
    let key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519)
        .map_err(|_| anyhow::anyhow!("Ed25519 key generation failed"))?;
    let private = key
        .to_openssh(LineEnding::LF)
        .map_err(|_| anyhow::anyhow!("OpenSSH key encoding failed"))?;
    let input = Zeroizing::new(serde_json::to_vec(private.as_str())?);
    let staged = tempfile::Builder::new()
        .prefix(".signing-key-")
        .tempdir_in(parent)?;
    let staged_path = staged.path().join("secrets.env.enc");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&staged_path)?;
    file.write_all(&before)?;
    file.sync_all()?;
    drop(file);
    sops.set(&staged_path, name, &input)?;
    let after = sops.decrypt(&staged_path)?;
    ensure!(
        after.0.len() == values.0.len() + 1
            && values.0.iter().all(|(k, v)| after.0.get(k) == Some(v))
            && after.0.get(name).map(String::as_str) == Some(private.as_str()),
        "encrypted key roundtrip changed unexpected values; original file retained"
    );
    let recovered = existing_key(&after, name)?;
    ensure!(
        recovered.public_key() == key.public_key(),
        "encrypted public key mismatch"
    );
    let final_bytes = ciphertext(&staged_path)?;
    ensure!(
        !final_bytes
            .windows(b"BEGIN OPENSSH PRIVATE KEY".len())
            .any(|w| w == b"BEGIN OPENSSH PRIVATE KEY"),
        "unencrypted key marker in candidate; original file retained"
    );
    ensure!(
        ciphertext(&path)? == before,
        "encrypted file changed during provisioning; original file retained"
    );
    fs::set_permissions(&staged_path, fs::Permissions::from_mode(0o600))?;
    File::open(&staged_path)?.sync_all()?;
    fs::rename(&staged_path, &path).context("could not atomically publish encrypted key")?;
    lock.sync_all().context("encrypted key was saved, but directory durability could not be confirmed; inspect with public-key before retrying")?;
    public_info(&key, name, fleet, path, true)
}

#[allow(unsafe_code)]
fn disable_core_dumps() -> Result<()> {
    let limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: setrlimit reads a correctly initialized structure. This command
    // handles private material; its SOPS children inherit the disabled limit.
    let result = unsafe { libc::setrlimit(libc::RLIMIT_CORE, &limit) };
    ensure!(result == 0, "could not disable signing-command core dumps");
    Ok(())
}

fn existing_key(values: &SecretValues, name: &str) -> Result<PrivateKey> {
    let value = values
        .0
        .get(name)
        .with_context(|| format!("secret '{name}' does not exist"))?;
    let key = PrivateKey::from_openssh(value)
        .map_err(|_| anyhow::anyhow!("secret '{name}' is not an OpenSSH private key"))?;
    ensure!(
        !key.is_encrypted() && key.algorithm() == Algorithm::Ed25519,
        "signing custody requires an Ed25519 key wrapped by SOPS, without an additional SSH passphrase"
    );
    Ok(key)
}

fn public_info(
    key: &PrivateKey,
    name: &str,
    fleet: &str,
    path: PathBuf,
    created: bool,
) -> Result<PublicKeyInfo> {
    Ok(PublicKeyInfo {
        name: name.into(),
        fleet: fleet.into(),
        secrets_file: path,
        public_key: key.public_key().to_openssh()?,
        fingerprint: key.public_key().fingerprint(HashAlg::Sha256).to_string(),
        created,
    })
}

struct Sops {
    age_key_file: PathBuf,
}

impl Sops {
    fn decrypt(&self, path: &Path) -> Result<SecretValues> {
        let output = self.command(
            &["decrypt", "--input-type", "dotenv", "--output-type", "json"],
            path,
            &[],
            None,
        )?;
        let values = serde_json::from_slice(&output)
            .map_err(|_| anyhow::anyhow!("SOPS did not return a string-valued secret document"))?;
        Ok(SecretValues(values))
    }

    fn set(&self, path: &Path, name: &str, value: &[u8]) -> Result<()> {
        let selector = serde_json::to_string(&[name])?;
        self.command(
            &[
                "set",
                "--input-type",
                "dotenv",
                "--output-type",
                "dotenv",
                "--value-stdin",
            ],
            path,
            &[selector.as_str()],
            Some(value),
        )?;
        Ok(())
    }

    fn command(
        &self,
        args: &[&str],
        path: &Path,
        trailing: &[&str],
        input: Option<&[u8]>,
    ) -> Result<Zeroizing<Vec<u8>>> {
        let mut child = Command::new("sops")
            .args(args)
            .arg(path)
            .args(trailing)
            .env("SOPS_AGE_KEY_FILE", &self.age_key_file)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .context("sops unavailable; use the pinned Workestrate package or development shell")?;
        let stdout = child.stdout.take().context("SOPS output pipe missing")?;
        let stdin = child.stdin.take();
        std::thread::scope(|scope| {
            let reader = scope.spawn(move || -> std::io::Result<Zeroizing<Vec<u8>>> {
                let mut bytes = Zeroizing::new(Vec::new());
                stdout.take(MAX_OUTPUT + 1).read_to_end(&mut bytes)?;
                Ok(bytes)
            });
            let writer = scope.spawn(move || -> std::io::Result<()> {
                if let (Some(mut stream), Some(value)) = (stdin, input) {
                    stream.write_all(value)?;
                }
                Ok(())
            });
            let deadline = Instant::now() + Duration::from_secs(30);
            let outcome = loop {
                match child.try_wait() {
                    Ok(Some(status)) => break Ok(status),
                    Err(error) => break Err(anyhow::anyhow!("SOPS process wait failed: {error}")),
                    Ok(None) if Instant::now() >= deadline => {
                        break Err(anyhow::anyhow!(
                            "SOPS operation timed out; original file retained"
                        ));
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                }
            };
            retire_group(child.id());
            let _ = child.wait();
            let output = reader
                .join()
                .map_err(|_| anyhow::anyhow!("SOPS output reader failed"))??;
            let written = writer
                .join()
                .map_err(|_| anyhow::anyhow!("SOPS input writer failed"))?;
            let status = outcome?;
            ensure!(
                status.success(),
                "SOPS operation failed; no private diagnostics emitted and original file retained"
            );
            written.context("SOPS input was not accepted")?;
            ensure!(
                output.len() as u64 <= MAX_OUTPUT,
                "SOPS output exceeded limit"
            );
            Ok(output)
        })
    }
}

#[allow(unsafe_code)]
fn retire_group(pid: u32) {
    if let Ok(pid) = i32::try_from(pid) {
        // SAFETY: the child was started in its own process group; only that
        // owned group is terminated, including helpers retaining pipe ends.
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, SecretsTarget) {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("fleet with spaces");
        fs::create_dir(&dir).unwrap();
        let age_key_file = tmp.path().join("age-key.txt");
        let generated = Command::new("age-keygen")
            .arg("-o")
            .arg(&age_key_file)
            .output()
            .unwrap();
        assert!(generated.status.success());
        let public = Command::new("age-keygen")
            .arg("-y")
            .arg(&age_key_file)
            .output()
            .unwrap();
        assert!(public.status.success());
        let recipient = String::from_utf8(public.stdout).unwrap();
        let mut child = Command::new("sops")
            .args([
                "encrypt",
                "--input-type",
                "json",
                "--output-type",
                "dotenv",
                "--age",
                recipient.trim(),
                "/dev/stdin",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(br#"{"KEEP":"synthetic-existing-value","MULTILINE":"first\nsecond"}"#)
            .unwrap();
        let encrypted = child.wait_with_output().unwrap();
        assert!(encrypted.status.success());
        fs::write(dir.join("custom.env.enc"), &encrypted.stdout).unwrap();
        (
            tmp,
            SecretsTarget {
                dir,
                secrets_file: "custom.env.enc".into(),
                age_key_file,
            },
        )
    }

    #[test]
    fn real_sops_generation_public_export_and_duplicate_refusal() {
        let (_tmp, target) = fixture();
        let path = target.dir.join(&target.secrets_file);
        let original = ciphertext(&path).unwrap();
        let created = provision(&target, "SIGNING_KEY", "fixture", true).unwrap();
        assert!(created.created);
        assert!(created.public_key.starts_with("ssh-ed25519 "));
        let exported = provision(&target, "SIGNING_KEY", "fixture", false).unwrap();
        assert!(!exported.created);
        assert_eq!(exported.public_key, created.public_key);
        assert_eq!(exported.fingerprint, created.fingerprint);
        let encrypted = ciphertext(&path).unwrap();
        assert_ne!(encrypted, original);
        assert!(!String::from_utf8_lossy(&encrypted).contains("PRIVATE KEY"));
        assert!(!String::from_utf8_lossy(&encrypted).contains("synthetic-existing-value"));
        assert!(
            provision(&target, "SIGNING_KEY", "fixture", true)
                .unwrap_err()
                .to_string()
                .contains("already exists")
        );
        assert_eq!(ciphertext(&path).unwrap(), encrypted);
        assert_eq!(fs::read_dir(&target.dir).unwrap().count(), 1);
        let sops = Sops {
            age_key_file: target.age_key_file.clone(),
        };
        let values = sops.decrypt(&path).unwrap();
        assert_eq!(values.0.get("KEEP").unwrap(), "synthetic-existing-value");
        assert_eq!(values.0.get("MULTILINE").unwrap(), "first\nsecond");
        assert!(values.0.get("SIGNING_KEY").unwrap().contains('\n'));
        let key = existing_key(&values, "SIGNING_KEY").unwrap();
        let payload = b"synthetic Git commit";
        let signature = key.sign("git", HashAlg::Sha256, payload).unwrap();
        key.public_key().verify("git", payload, &signature).unwrap();
    }

    #[test]
    fn wrong_age_identity_and_concurrent_writer_keep_original_ciphertext() {
        let (_tmp, mut target) = fixture();
        let path = target.dir.join(&target.secrets_file);
        let original = ciphertext(&path).unwrap();
        let lock = File::open(&target.dir).unwrap();
        lock.try_lock().unwrap();
        assert!(provision(&target, "SIGNING_KEY", "fixture", true).is_err());
        assert_eq!(ciphertext(&path).unwrap(), original);
        drop(lock);
        let (_other, other) = fixture();
        target.age_key_file = other.age_key_file;
        assert!(provision(&target, "SIGNING_KEY", "fixture", true).is_err());
        assert_eq!(ciphertext(&path).unwrap(), original);
        assert_eq!(fs::read_dir(&target.dir).unwrap().count(), 1);
    }

    #[test]
    fn names_cannot_address_json_paths_or_sops_metadata() {
        for name in ["KEY", "machine_key", "_KEY1"] {
            valid_name(name).unwrap();
        }
        for name in [
            "", "1KEY", "KEY\nX", "KEY[0]", "../KEY", "sops_age", "SOPS_MAC",
        ] {
            assert!(valid_name(name).is_err());
        }
    }

    #[test]
    fn generated_key_roundtrips_and_signs_only_public_payload_fixture() {
        let key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519).unwrap();
        let private = key.to_openssh(LineEnding::LF).unwrap();
        let values = SecretValues(HashMap::from([("KEY".into(), private.to_string())]));
        let restored = existing_key(&values, "KEY").unwrap();
        let payload = b"tree synthetic\nauthor fixture\n\nfixture commit\n";
        let signature = restored.sign("git", HashAlg::Sha256, payload).unwrap();
        restored
            .public_key()
            .verify("git", payload, &signature)
            .unwrap();
        assert!(
            restored
                .public_key()
                .verify("ssh", payload, &signature)
                .is_err()
        );
        let info = public_info(
            &restored,
            "KEY",
            "fixture",
            PathBuf::from("secrets.env.enc"),
            false,
        )
        .unwrap();
        let json = serde_json::to_string(&info).unwrap();
        assert!(!json.contains("PRIVATE KEY"));
        assert!(!json.contains(private.as_str()));
    }

    #[test]
    fn symlink_and_parent_escape_are_refused_before_sops() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real");
        fs::create_dir(&real).unwrap();
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(owned_path(&link, true).is_err());
        let target = SecretsTarget {
            dir: real,
            secrets_file: "../elsewhere".into(),
            age_key_file: tmp.path().join("absent"),
        };
        assert!(
            provision(&target, "KEY", "fixture", true)
                .unwrap_err()
                .to_string()
                .contains("relative path")
        );
    }
}
