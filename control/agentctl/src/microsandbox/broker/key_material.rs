//! Sealed key custody: SOPS-backed OpenSSH key material and the real
//! SSHSIG signing backend.
//!
//! Custody chain: credential material lives SOPS-encrypted at rest in
//! `.env.enc` layers; [`crate::microsandbox::secrets_loader::load_secrets`]
//! decrypts those layers into a plain map; [`SopsKeyMaterial`] resolves ONE
//! secret out of that map (secret ID → env var → OpenSSH private-key text)
//! and parses it into an Ed25519 keypair. The key text itself is never
//! retained: only the parsed [`ssh_key::PrivateKey`] is kept, whose Ed25519
//! scalar `ssh-key` redacts from `Debug` and zeroizes on drop. Every failure
//! is a typed [`KeyMaterialError`] and fails closed (no signing, and no
//! partially-wired backend — construction aborts on the first bad key).
//!
//! Key-reference mapping: the signing service passes the grant's key
//! reference to the backend, while the grant's `material` field names the
//! SECRET holding the key. The backend therefore needs a
//! key-reference → secret-ID map built at construction
//! ([`SealedKeyBackend::from_secrets`]); deployers derive it from the grant
//! set (grant name → grant material, see
//! [`SealedKeyBackend::from_credentials_plan`]). The map holds one secret
//! per key reference, so two grants sharing a reference must agree on the
//! secret — callers merge multi-instance plans before constructing.
//!
//! SOPS reuse: this module never shells out to `sops` and never parses
//! `.env.enc` itself. Production entry points ([`SopsKeyMaterial::load`],
//! [`SealedKeyBackend::load`]) call the existing `load_secrets()`
//! abstraction; the anyhow failure it can return (missing age key, `sops`
//! binary absent, undecryptable layer) maps to
//! [`KeyMaterialError::Undecryptable`], which is reserved for that layer.

use crate::microsandbox::plan::CredentialsPlan;
use crate::microsandbox::secrets::SecretDefinition;
use std::collections::HashMap;
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Typed custody errors
// ---------------------------------------------------------------------------

/// Every way sealed-key custody can refuse. Variants carry secret IDs and
/// static failure details only — never key text — so they are safe to log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyMaterialError {
    /// The secret is missing from the decrypted map, or holds only
    /// whitespace. Fail-closed: an absent key never signs.
    MaterialAbsent {
        /// Secret ID that could not be resolved (an identifier, not a secret).
        secret_id: String,
    },
    /// Reserved for SOPS-layer failures: the `load_secrets()` call itself
    /// failed (missing age key, `sops` binary absent, undecryptable layer).
    /// Carries the anyhow message, which names files/layers, not secrets.
    Undecryptable {
        /// Underlying SOPS-layer failure detail.
        detail: String,
    },
    /// The resolved value is not a parseable OpenSSH private key
    /// (garbage, PKCS#8/PKCS#1 PEM, truncated armor, ...).
    ParseError {
        /// Parser failure detail (static `ssh-key` messages, no key echo).
        detail: String,
    },
    /// The value parses as an OpenSSH key but is unusable here: not
    /// Ed25519 (e.g. RSA), or passphrase-encrypted (broker custody holds
    /// no passphrase path, so encrypted keys stay closed).
    WrongKeyType {
        /// What was found instead of a cleartext Ed25519 key.
        detail: String,
    },
}

impl std::fmt::Display for KeyMaterialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyMaterialError::MaterialAbsent { secret_id } => write!(
                f,
                "sealed key material absent: secret '{secret_id}' is missing or empty"
            ),
            KeyMaterialError::Undecryptable { detail } => {
                write!(f, "sealed key material undecryptable: {detail}")
            }
            KeyMaterialError::ParseError { detail } => {
                write!(f, "sealed key material unparsable: {detail}")
            }
            KeyMaterialError::WrongKeyType { detail } => {
                write!(f, "sealed key material has wrong key type: {detail}")
            }
        }
    }
}

impl std::error::Error for KeyMaterialError {}

// ---------------------------------------------------------------------------
// Sealed material: one parsed Ed25519 key
// ---------------------------------------------------------------------------

/// One Ed25519 keypair resolved from the SOPS-decrypted secrets layer.
///
/// The struct retains ONLY the parsed keypair: resolution borrows the
/// caller's decrypted map (or a zeroizing resolver buffer — see
/// [`SopsKeyMaterial::from_resolver`]), and the parse copies the scalar
/// into `ssh-key`'s own zeroizing buffers. Dropping this value drops the
/// keypair, whose Ed25519 scalar is zeroized by `ssh-key`'s `Drop` impl.
pub struct SopsKeyMaterial {
    /// Secret ID this material was resolved from (provenance label, not a
    /// secret — safe to log, and shown by the redacting `Debug` impl).
    secret_id: String,
    /// Parsed keypair. Field-private: signing goes through
    /// [`SealedKeyBackend`]; only the trusted broker adapter can project a
    /// zeroizing seed for the protected custody transport.
    private_key: ssh_key::PrivateKey,
}

// Custom redacting Debug: the derived impl would print through to the key.
// `ssh-key` already redacts the scalar itself; this additionally hides the
// whole key behind a marker and shows only the provenance label.
impl std::fmt::Debug for SopsKeyMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SopsKeyMaterial")
            .field("secret_id", &self.secret_id)
            .field("algorithm", &self.private_key.algorithm())
            .field("key", &"<redacted>")
            .finish()
    }
}

impl SopsKeyMaterial {
    /// Resolve `secret_id` from an already-decrypted secrets map (as
    /// returned by `load_secrets()`), reading the `env_var` entry.
    ///
    /// # Errors
    /// - [`KeyMaterialError::MaterialAbsent`] when the entry is missing or
    ///   blank (fail-closed).
    /// - [`KeyMaterialError::ParseError`] when the value is not an OpenSSH
    ///   private key.
    /// - [`KeyMaterialError::WrongKeyType`] when it parses but is not a
    ///   cleartext Ed25519 key.
    pub fn from_decrypted(
        secrets: &HashMap<String, String>,
        secret_id: &str,
        env_var: &str,
    ) -> Result<Self, KeyMaterialError> {
        let text = secrets.get(env_var).map(String::as_str).unwrap_or("");
        Self::parse(secret_id, text)
    }

    /// Resolve `secret_id` through secret definitions (secret ID → env var
    /// via [`SecretDefinition::source_env_var`], defaulting to the secret
    /// ID itself when no definition entry exists — the same
    /// defaults-after-merge rule `build_secret_definitions` applies).
    ///
    /// # Errors
    /// Same as [`SopsKeyMaterial::from_decrypted`].
    pub fn from_definitions(
        secrets: &HashMap<String, String>,
        definitions: &HashMap<String, SecretDefinition>,
        secret_id: &str,
    ) -> Result<Self, KeyMaterialError> {
        let env_var = definitions
            .get(secret_id)
            .map_or(secret_id, |def| def.source_env_var.as_str());
        Self::from_decrypted(secrets, secret_id, env_var)
    }

    /// Resolve `env_var` through a caller-supplied lookup (e.g. a test
    /// double or an alternate secret store). The resolver-owned copy is
    /// held in a zeroizing buffer; the parse only borrows it.
    ///
    /// # Errors
    /// Same as [`SopsKeyMaterial::from_decrypted`].
    pub fn from_resolver(
        secret_id: &str,
        env_var: &str,
        resolver: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Self, KeyMaterialError> {
        let owned = Zeroizing::new(resolver(env_var).unwrap_or_default());
        Self::parse(secret_id, owned.as_str())
    }

    /// Production path: decrypt the SOPS layers via the existing
    /// `load_secrets()` pipeline, then resolve as in
    /// [`SopsKeyMaterial::from_decrypted`].
    ///
    /// # Errors
    /// - [`KeyMaterialError::Undecryptable`] when the SOPS layer itself
    ///   fails (reserved for that layer — see the module docs).
    /// - Otherwise same as [`SopsKeyMaterial::from_decrypted`].
    pub fn load(secret_id: &str, env_var: &str) -> Result<Self, KeyMaterialError> {
        let secrets = crate::microsandbox::secrets_loader::load_secrets().map_err(|e| {
            KeyMaterialError::Undecryptable {
                detail: e.to_string(),
            }
        })?;
        Self::from_decrypted(&secrets, secret_id, env_var)
    }

    /// Secret ID this material was resolved from (provenance label).
    #[must_use]
    pub fn secret_id(&self) -> &str {
        &self.secret_id
    }

    /// Project the already-validated key for direct broker custody transport.
    /// This is not a public credential accessor or a new material store. The
    /// trusted resolver must attach its selected grant and versions, and move
    /// these bytes into the shared wire's redacted, zeroizing secret buffer.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the production broker policy resolver does not yet consume validated host-side key seeds"
        )
    )]
    pub(crate) fn broker_key_seed(&self) -> Result<Zeroizing<[u8; 32]>, KeyMaterialError> {
        match self.private_key.key_data() {
            ssh_key::private::KeypairData::Ed25519(keypair) => {
                Ok(Zeroizing::new(keypair.private.to_bytes()))
            }
            _ => Err(KeyMaterialError::WrongKeyType {
                detail: "broker custody requires a cleartext Ed25519 key".into(),
            }),
        }
    }

    /// Issue public broker host trust from a host-owned CA secret. The trusted
    /// lifecycle owner chooses this CA reference and durable serial; delegated
    /// workload requests cannot select the authority. The parsed private key
    /// remains inside existing sealed-material custody.
    pub fn issue_broker_host_certificate(
        &self,
        principal: &super::host_trust::BrokerHostPrincipal,
        broker_public_key: &ssh_key::PublicKey,
        serial: u64,
        valid_after: u64,
        valid_before: u64,
    ) -> std::io::Result<super::host_trust::BrokerHostTrust> {
        principal.issue(
            &self.private_key,
            broker_public_key,
            serial,
            valid_after,
            valid_before,
        )
    }

    /// Shared parse backend: blank → absent, OpenSSH decode, then
    /// cleartext-Ed25519 gate. `text` is only borrowed — the caller keeps
    /// owning the plaintext (the decrypted map, or a zeroizing buffer).
    fn parse(secret_id: &str, text: &str) -> Result<Self, KeyMaterialError> {
        // SOPS dotenv values commonly carry a trailing newline; surrounding
        // whitespace is never part of the key.
        let text = text.trim();
        if text.is_empty() {
            return Err(KeyMaterialError::MaterialAbsent {
                secret_id: secret_id.to_string(),
            });
        }
        let private_key =
            ssh_key::PrivateKey::from_openssh(text).map_err(|e| KeyMaterialError::ParseError {
                detail: e.to_string(),
            })?;
        if private_key.is_encrypted() {
            return Err(KeyMaterialError::WrongKeyType {
                detail: "key is passphrase-encrypted: broker custody holds no passphrase path"
                    .to_string(),
            });
        }
        match private_key.key_data() {
            ssh_key::private::KeypairData::Ed25519(_) => Ok(Self {
                secret_id: secret_id.to_string(),
                private_key,
            }),
            _ => {
                let found = private_key.algorithm().to_string();
                Err(KeyMaterialError::WrongKeyType {
                    detail: format!("expected an Ed25519 OpenSSH key, found {found}"),
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Backend: real SSHSIG signatures behind the KeyBackend trait
// ---------------------------------------------------------------------------

/// A [`KeyBackend`](super::signing::KeyBackend) holding one parsed Ed25519
/// key per key reference, signing real SSHSIG structures.
///
/// Signature shape (draft-ietf-secsh-sshsig, built by `ssh-key`
/// internally): the signed preimage is `"SSHSIG" || u32-be(len(namespace))
/// || namespace || reserved(empty) || u32-be(len(hash_alg_name)) ||
/// hash_alg_name || u32-be(len(hash)) || SHA-256(payload)` — the requested
/// namespace is therefore cryptographically bound into every signature, so
/// a signature minted for one namespace never verifies under another (see
/// [`verify_sshsig`]). The Ed25519 signature over that preimage is armored
/// as a `-----BEGIN SSH SIGNATURE-----` PEM block (version 1, embedded
/// public key in authorized_keys shape, namespace, `sha256` hash id).
/// Only the `ssh-sig` scheme is served; anything else (including `raw`) is
/// refused even though the service layer already denies it first.
#[derive(Debug)]
pub struct SealedKeyBackend {
    keys: HashMap<String, SopsKeyMaterial>,
}

// The derived Debug prints the map: keys are key references (non-secret
// identifiers, useful in diagnostics) while SopsKeyMaterial redacts itself
// (see its Debug impl — that redaction is the invariant keeping key text
// out of logs, and must survive any future field additions there).

impl SealedKeyBackend {
    /// Build from an explicit key-reference → secret-ID map plus the
    /// already-decrypted secrets map. `secret_env_vars` maps secret ID →
    /// env var and defaults an unmapped secret ID to an env var of its own
    /// name. Aborts on the FIRST unusable key (fail-closed: no partial
    /// backend that would sign for some references and deny others on
    /// wiring grounds — denials stay the service layer's audited job).
    ///
    /// # Errors
    /// The first [`KeyMaterialError`] encountered while resolving the map.
    pub fn from_secrets(
        key_refs: &HashMap<String, String>,
        secrets: &HashMap<String, String>,
        secret_env_vars: &HashMap<String, String>,
    ) -> Result<Self, KeyMaterialError> {
        let mut keys = HashMap::with_capacity(key_refs.len());
        for (key_ref, secret_id) in key_refs {
            let env_var = secret_env_vars
                .get(secret_id)
                .map_or(secret_id.as_str(), String::as_str);
            let material = SopsKeyMaterial::from_decrypted(secrets, secret_id, env_var)?;
            keys.insert(key_ref.clone(), material);
        }
        Ok(Self { keys })
    }

    /// Build the key-reference map from a credential plan: the grant name
    /// IS the broker key reference and the grant material IS the secret ID
    /// (see `SigningGrantPlan`). Binding-agnostic by design — a guest-bound
    /// grant's material is still loaded here; refusing to SIGN for it is
    /// the service layer's audited denial, not the backend's. Grant-pipeline
    /// semantics are untouched: this only reads the plan.
    ///
    /// # Errors
    /// Same as [`SealedKeyBackend::from_secrets`].
    pub fn from_credentials_plan(
        plan: &CredentialsPlan,
        secrets: &HashMap<String, String>,
        secret_env_vars: &HashMap<String, String>,
    ) -> Result<Self, KeyMaterialError> {
        let key_refs: HashMap<String, String> = plan
            .signing
            .iter()
            .map(|grant| (grant.name.clone(), grant.material.clone()))
            .collect();
        Self::from_secrets(&key_refs, secrets, secret_env_vars)
    }

    /// Production path: decrypt the SOPS layers via the existing
    /// `load_secrets()` pipeline, then wire as in
    /// [`SealedKeyBackend::from_secrets`].
    ///
    /// # Errors
    /// - [`KeyMaterialError::Undecryptable`] when the SOPS layer itself fails.
    /// - Otherwise same as [`SealedKeyBackend::from_secrets`].
    pub fn load(
        key_refs: &HashMap<String, String>,
        secret_env_vars: &HashMap<String, String>,
    ) -> Result<Self, KeyMaterialError> {
        let secrets = crate::microsandbox::secrets_loader::load_secrets().map_err(|e| {
            KeyMaterialError::Undecryptable {
                detail: e.to_string(),
            }
        })?;
        Self::from_secrets(key_refs, &secrets, secret_env_vars)
    }

    /// Whether this backend holds material for `key_ref`.
    #[must_use]
    pub fn contains_key(&self, key_ref: &str) -> bool {
        self.keys.contains_key(key_ref)
    }
}

impl super::signing::KeyBackend for SealedKeyBackend {
    fn sign(
        &self,
        key_ref: &str,
        scheme: &super::signing::SignatureScheme,
        namespace: &str,
        payload: &[u8],
    ) -> Result<Vec<u8>, String> {
        if *scheme != super::signing::SignatureScheme::SshSig {
            return Err(format!(
                "sealed backend refuses scheme '{scheme}': only ssh-sig is supported"
            ));
        }
        let material = self.keys.get(key_ref).ok_or_else(|| {
            format!("sealed backend holds no material for key reference '{key_ref}'")
        })?;
        // `ssh-key` builds the SSHSIG preimage (namespace-bound, SHA-256),
        // signs it Ed25519, and armors the result; its error Displays are
        // static strings, safe to forward without leaking key text.
        let signature = material
            .private_key
            .sign(namespace, ssh_key::HashAlg::Sha256, payload)
            .map_err(|e| format!("sealed backend signing failed: {e}"))?;
        let armored = signature
            .to_pem(ssh_key::LineEnding::LF)
            .map_err(|e| format!("sealed backend armor failed: {e}"))?;
        Ok(armored.into_bytes())
    }
}

// ---------------------------------------------------------------------------
// Verification self-check (tests + audit)
// ---------------------------------------------------------------------------

/// How SSHSIG verification can fail. Carries no key text and no payload —
/// only the failure class (plus static parser detail), so results are safe
/// to log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshSigVerifyError {
    /// The armored signature or the authorized_keys public key does not
    /// parse (static `ssh-key` detail, no key echo).
    ParseError {
        /// Parser failure detail.
        detail: String,
    },
    /// The signature's embedded public key is not the given key.
    KeyMismatch,
    /// The given namespace differs from the signature's namespace. Fires
    /// before any cryptography: the namespace is checked as a string AND
    /// bound into the signed preimage, so cross-namespace replay fails
    /// twice over.
    NamespaceMismatch,
    /// Key and namespace match, but the Ed25519 check over the namespace-
    /// bound preimage failed (wrong payload, or tampered signature).
    SignatureInvalid,
}

impl std::fmt::Display for SshSigVerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SshSigVerifyError::ParseError { detail } => {
                write!(f, "sshsig parse failed: {detail}")
            }
            SshSigVerifyError::KeyMismatch => {
                write!(f, "sshsig public key does not match the given key")
            }
            SshSigVerifyError::NamespaceMismatch => {
                write!(f, "sshsig namespace does not match the given namespace")
            }
            SshSigVerifyError::SignatureInvalid => {
                write!(f, "sshsig signature invalid for the given payload")
            }
        }
    }
}

impl std::error::Error for SshSigVerifyError {}

/// Verify an armored SSHSIG signature against `payload`, binding BOTH the
/// given `namespace` and the given authorized_keys-format `public_key`.
///
/// The namespace passed here is authoritative for the check: it must equal
/// the namespace embedded in the armor (string check) and it recomputes
/// the signed preimage, so `sign(git)` verifies under `"git"` and fails
/// under `"file"`, and vice versa.
///
/// # Errors
/// - [`SshSigVerifyError::ParseError`] when the armor or public key does
///   not parse.
/// - [`SshSigVerifyError::KeyMismatch`] / [`SshSigVerifyError::NamespaceMismatch`] /
///   [`SshSigVerifyError::SignatureInvalid`] per the failure class above.
pub fn verify_sshsig(
    signature: &[u8],
    payload: &[u8],
    namespace: &str,
    public_key: &str,
) -> Result<(), SshSigVerifyError> {
    let signature =
        ssh_key::SshSig::from_pem(signature).map_err(|e| SshSigVerifyError::ParseError {
            detail: e.to_string(),
        })?;
    let public_key: ssh_key::PublicKey =
        public_key
            .parse()
            .map_err(|e: ssh_key::Error| SshSigVerifyError::ParseError {
                detail: e.to_string(),
            })?;
    public_key
        .verify(namespace, payload, &signature)
        .map_err(|e| match e {
            ssh_key::Error::PublicKey => SshSigVerifyError::KeyMismatch,
            ssh_key::Error::Namespace => SshSigVerifyError::NamespaceMismatch,
            _ => SshSigVerifyError::SignatureInvalid,
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::super::signing::{
        GrantStore, KeyBackend, LimitsConfig, SignRequest, SignatureScheme, SigningService,
    };
    use super::*;
    use crate::config::SecretViolationPolicy;
    use crate::microsandbox::broker::audit::{AuditLog, AuditResult};
    use crate::microsandbox::plan::{CredentialBinding, CredentialsPlan, SigningGrantPlan};

    /// OS RNG via `ssh-key`'s `rand_core` re-export (no new RNG dep for
    /// key generation). `OsRng` panics internally on failure, so no
    /// `UnwrapErr` adapter is needed.
    fn test_rng() -> ssh_key::rand_core::OsRng {
        ssh_key::rand_core::OsRng
    }

    /// Fresh Ed25519 keypair, generated in-test (never committed).
    fn test_keypair() -> ssh_key::PrivateKey {
        ssh_key::PrivateKey::random(&mut test_rng(), ssh_key::Algorithm::Ed25519).unwrap()
    }

    /// Fresh Ed25519 keypair serialized to OpenSSH text, as a SOPS layer
    /// would deliver it (trailing newline included, like dotenv values).
    fn ed25519_openssh() -> String {
        test_keypair()
            .to_openssh(ssh_key::LineEnding::LF)
            .unwrap()
            .to_string()
    }

    /// Authorized_keys line for a generated keypair (the verify-side key).
    fn authorized_keys(key: &ssh_key::PrivateKey) -> String {
        key.public_key().to_openssh().unwrap()
    }

    /// One-entry decrypted layer: env var → key text.
    fn layer(env_var: &str, text: &str) -> HashMap<String, String> {
        HashMap::from([(env_var.to_string(), text.to_string())])
    }

    /// Minimal secret definition mapping a secret ID to its env var.
    fn definition(env_var: &str) -> SecretDefinition {
        SecretDefinition {
            source_env_var: env_var.to_string(),
            allowed_hosts: Vec::new(),
            required: true,
            placeholder: None,
            on_violation: SecretViolationPolicy::Block,
        }
    }

    fn backend_for(pem: &str) -> SealedKeyBackend {
        let secrets = layer("SIGN_KEY_ENV", pem);
        let key_refs = HashMap::from([("rel".to_string(), "SIGN_KEY".to_string())]);
        let env_vars = HashMap::from([("SIGN_KEY".to_string(), "SIGN_KEY_ENV".to_string())]);
        SealedKeyBackend::from_secrets(&key_refs, &secrets, &env_vars).unwrap()
    }

    // --- loader: happy paths ---

    #[test]
    fn broker_seed_preserves_selected_key_without_reloading_material() {
        for _ in 0..2 {
            let key = test_keypair();
            let text = key.to_openssh(ssh_key::LineEnding::LF).unwrap();
            let mut secrets = layer("SELECTED_ENV", text.as_str());
            secrets.insert("UNRELATED_ENV".into(), ed25519_openssh());
            let material =
                SopsKeyMaterial::from_decrypted(&secrets, "selected-key", "SELECTED_ENV").unwrap();
            // The parsed owner does not consult a changed source map again.
            secrets.insert("SELECTED_ENV".into(), ed25519_openssh());
            let seed = material.broker_key_seed().unwrap();
            let reconstructed =
                ssh_key::PrivateKey::from(ssh_key::private::Ed25519Keypair::from_seed(&seed));
            assert_eq!(reconstructed.public_key(), key.public_key());
            assert_eq!(material.secret_id(), "selected-key");
            assert!(format!("{material:?}").contains("<redacted>"));
        }
    }

    #[test]
    fn loader_resolves_secret_id_through_env_var() {
        let pem = ed25519_openssh();
        let secrets = layer("SIGN_KEY_ENV", &pem);
        let mat = SopsKeyMaterial::from_decrypted(&secrets, "SIGN_KEY", "SIGN_KEY_ENV").unwrap();
        assert_eq!(mat.secret_id(), "SIGN_KEY");

        // Definitions path honors source_env_var ...
        let defs = HashMap::from([("SIGN_KEY".to_string(), definition("SIGN_KEY_ENV"))]);
        let mat = SopsKeyMaterial::from_definitions(&secrets, &defs, "SIGN_KEY").unwrap();
        assert_eq!(mat.secret_id(), "SIGN_KEY");

        // ... and defaults the env var to the secret ID with no entry
        // (the same defaults-after-merge rule build_secret_definitions
        // applies).
        let secrets = layer("SIGN_KEY", &pem);
        let mat = SopsKeyMaterial::from_definitions(&secrets, &HashMap::new(), "SIGN_KEY").unwrap();
        assert_eq!(mat.secret_id(), "SIGN_KEY");
    }

    #[test]
    fn loader_resolver_path_serves_lookups() {
        let pem = ed25519_openssh();
        let secrets = layer("SIGN_KEY_ENV", &pem);
        let mat = SopsKeyMaterial::from_resolver("SIGN_KEY", "SIGN_KEY_ENV", &|var| {
            secrets.get(var).cloned()
        })
        .unwrap();
        assert_eq!(mat.secret_id(), "SIGN_KEY");
    }

    // --- loader: custody failures ---

    #[test]
    fn loader_absent_and_empty_fail_closed() {
        // Missing entry ...
        let err = SopsKeyMaterial::from_decrypted(&HashMap::new(), "SIGN_KEY", "SIGN_KEY_ENV")
            .unwrap_err();
        assert!(
            matches!(err, KeyMaterialError::MaterialAbsent { .. }),
            "missing secret must be MaterialAbsent, got: {err}"
        );
        // ... and the log-safe Display names the secret ID, never a value.
        assert!(err.to_string().contains("SIGN_KEY"));

        // Empty and whitespace-only values are absent too (a blank dotenv
        // value must not reach the parser, let alone sign).
        for blank in ["", "   ", "\n  \n"] {
            let secrets = layer("SIGN_KEY_ENV", blank);
            let err =
                SopsKeyMaterial::from_decrypted(&secrets, "SIGN_KEY", "SIGN_KEY_ENV").unwrap_err();
            assert!(
                matches!(err, KeyMaterialError::MaterialAbsent { .. }),
                "blank secret must be MaterialAbsent, got: {err}"
            );
        }

        // A resolver returning None is absent as well.
        let err =
            SopsKeyMaterial::from_resolver("SIGN_KEY", "SIGN_KEY_ENV", &|_| None).unwrap_err();
        assert!(matches!(err, KeyMaterialError::MaterialAbsent { .. }));
    }

    #[test]
    fn loader_garbage_fails_parse() {
        let secrets = layer("SIGN_KEY_ENV", "definitely-not-a-key");
        let err =
            SopsKeyMaterial::from_decrypted(&secrets, "SIGN_KEY", "SIGN_KEY_ENV").unwrap_err();
        assert!(
            matches!(err, KeyMaterialError::ParseError { .. }),
            "garbage must be ParseError, got: {err}"
        );

        // A PKCS#8 PEM is a real key but NOT an OpenSSH key: still a parse
        // failure, never a signing key.
        let pkcs8 =
            "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEINTZ\r\n-----END PRIVATE KEY-----\n";
        let secrets = layer("SIGN_KEY_ENV", pkcs8);
        let err =
            SopsKeyMaterial::from_decrypted(&secrets, "SIGN_KEY", "SIGN_KEY_ENV").unwrap_err();
        assert!(
            matches!(err, KeyMaterialError::ParseError { .. }),
            "non-OpenSSH PEM must be ParseError, got: {err}"
        );
    }

    #[test]
    fn loader_rsa_key_fails_wrong_type() {
        // A REAL RSA key, generated in-test (2048-bit: the smallest the
        // key crate generates; production never signs with it — the
        // loader rejects it first).
        let rsa = ssh_key::private::RsaKeypair::random(&mut test_rng(), 2048).unwrap();
        let rsa_key =
            ssh_key::PrivateKey::new(ssh_key::private::KeypairData::from(rsa), "test").unwrap();
        let pem = rsa_key.to_openssh(ssh_key::LineEnding::LF).unwrap();
        let secrets = layer("SIGN_KEY_ENV", pem.as_str());

        let err =
            SopsKeyMaterial::from_decrypted(&secrets, "SIGN_KEY", "SIGN_KEY_ENV").unwrap_err();
        assert!(
            matches!(err, KeyMaterialError::WrongKeyType { .. }),
            "RSA key must be WrongKeyType, got: {err}"
        );
        assert!(
            err.to_string().contains("ssh-rsa"),
            "the error names the found algorithm, got: {err}"
        );
    }

    #[test]
    fn loader_encrypted_key_fails_wrong_type_without_passphrase_path() {
        // A REAL passphrase-encrypted key, generated in-test: broker
        // custody holds no passphrase, so it must fail closed here rather
        // than prompt, block, or — worst — sign.
        let encrypted = test_keypair()
            .encrypt(&mut test_rng(), "test-password")
            .unwrap();
        assert!(encrypted.is_encrypted());
        let pem = encrypted.to_openssh(ssh_key::LineEnding::LF).unwrap();
        let secrets = layer("SIGN_KEY_ENV", pem.as_str());

        let err =
            SopsKeyMaterial::from_decrypted(&secrets, "SIGN_KEY", "SIGN_KEY_ENV").unwrap_err();
        assert!(
            matches!(err, KeyMaterialError::WrongKeyType { .. }),
            "encrypted key must be WrongKeyType, got: {err}"
        );
        assert!(
            err.to_string().contains("passphrase"),
            "the error must say why it stays closed, got: {err}"
        );
    }

    #[allow(unsafe_code)]
    #[test]
    fn production_load_wires_the_decrypted_layer() {
        // The SOPS step itself stays behind the existing load_secrets()
        // abstraction (the `sops` binary is unavailable in unit-test
        // environments by design), so this drives load() against a temp
        // config dir: load_secrets() succeeds with an unsatisfied-optional
        // map, and load() then fails closed on the missing secret — proving
        // the production path resolves through the decrypted layer instead
        // of a parallel SOPS stack. Follows the secrets_loader.rs test
        // pattern (global env lock + temp config dir), plus the
        // provenance-storage lock: load() drives the real load_secrets()
        // pipeline, which re-fills the process-global policy-ladder storage
        // as a side effect — without the storage lock this test could
        // clobber the ladder-storage tests mid-test (lock order matches
        // commands/deps.rs: env first, storage second).
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _storage = crate::config::test_support::PROVENANCE_STORAGE_TEST_LOCK
            .lock()
            .unwrap();
        let probe = "WORKESTRATE_SEALED_LOAD_PROBE";
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test).
        unsafe { std::env::remove_var(probe) };
        let tmp =
            std::env::temp_dir().join(format!("workestrate-sealed-load-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let config = "schema_version = 1\n\n[secrets.WORKESTRATE_SEALED_LOAD_PROBE]\nenv_var = \"WORKESTRATE_SEALED_LOAD_PROBE\"\nrequired = false\n";
        std::fs::write(tmp.join("workestrate.toml"), config).unwrap();
        let old = std::env::var("WORKESTRATE_FLEET_DIR").ok();
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test).
        unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", &tmp) };

        let result = SopsKeyMaterial::load(probe, probe);

        match old {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test).
            Some(v) => unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test).
            None => unsafe { std::env::remove_var("WORKESTRATE_FLEET_DIR") },
        }
        std::fs::remove_dir_all(&tmp).unwrap();

        // The decrypted layer held no such secret: typed fail-closed, NOT
        // an undecryptable-layer error (the SOPS step succeeded).
        let err = result.unwrap_err();
        assert!(
            matches!(err, KeyMaterialError::MaterialAbsent { .. }),
            "production load with no such secret must be MaterialAbsent, got: {err}"
        );
    }

    // --- backend: construction ---

    #[test]
    fn backend_construction_is_all_or_nothing() {
        let pem = ed25519_openssh();
        let secrets = layer("SIGN_KEY_ENV", &pem);
        let env_vars = HashMap::from([("SIGN_KEY".to_string(), "SIGN_KEY_ENV".to_string())]);

        // One good reference wires fine.
        let key_refs = HashMap::from([("rel".to_string(), "SIGN_KEY".to_string())]);
        let backend = SealedKeyBackend::from_secrets(&key_refs, &secrets, &env_vars).unwrap();
        assert!(backend.contains_key("rel"));
        assert!(!backend.contains_key("nope"));

        // One bad reference poisons the whole construction: no partial
        // backend that signs for some references while others fail on
        // wiring grounds.
        let key_refs = HashMap::from([
            ("rel".to_string(), "SIGN_KEY".to_string()),
            ("broken".to_string(), "MISSING_SECRET".to_string()),
        ]);
        let err = SealedKeyBackend::from_secrets(&key_refs, &secrets, &env_vars).unwrap_err();
        assert!(
            matches!(err, KeyMaterialError::MaterialAbsent { .. }),
            "one bad key must fail the whole backend, got: {err}"
        );
    }

    #[test]
    fn backend_derives_key_refs_from_grant_plan() {
        let pem = ed25519_openssh();
        let secrets = layer("SIGN_KEY_ENV", &pem);
        let env_vars = HashMap::from([("SIGN_KEY".to_string(), "SIGN_KEY_ENV".to_string())]);
        let plan = CredentialsPlan {
            ssh: Vec::new(),
            signing: vec![SigningGrantPlan {
                name: "rel".to_string(),
                material: "SIGN_KEY".to_string(),
                namespace: "git".to_string(),
                on_violation: SecretViolationPolicy::Block,
                binding: CredentialBinding::Broker,
            }],
            strict: false,
            strict_origin: None,
        };
        let backend = SealedKeyBackend::from_credentials_plan(&plan, &secrets, &env_vars).unwrap();
        assert!(backend.contains_key("rel"));
    }

    // --- backend: sign + verify round-trip ---

    #[test]
    fn sign_emits_armored_sshsig_and_verifies() {
        let key = test_keypair();
        let pem = key.to_openssh(ssh_key::LineEnding::LF).unwrap();
        let backend = backend_for(pem.as_str());
        let pubkey = authorized_keys(&key);

        let payload = b"commit 0123456789abcdef";
        let sig = backend
            .sign("rel", &SignatureScheme::SshSig, "git", payload)
            .unwrap();
        let text = std::str::from_utf8(&sig).unwrap();
        assert!(
            text.starts_with("-----BEGIN SSH SIGNATURE-----"),
            "backend must emit armored SSHSIG, got: {text}"
        );

        // Round-trip under the minting namespace.
        verify_sshsig(&sig, payload, "git", &pubkey).unwrap();
    }

    #[test]
    fn namespace_binding_fails_both_directions() {
        let key = test_keypair();
        let pem = key.to_openssh(ssh_key::LineEnding::LF).unwrap();
        let backend = backend_for(pem.as_str());
        let pubkey = authorized_keys(&key);
        let payload = b"the same bytes";

        // The backend itself serves any namespace string (grant scoping is
        // the service layer's job); the CRYPTO binds it.
        let git_sig = backend
            .sign("rel", &SignatureScheme::SshSig, "git", payload)
            .unwrap();
        let file_sig = backend
            .sign("rel", &SignatureScheme::SshSig, "file", payload)
            .unwrap();

        verify_sshsig(&git_sig, payload, "git", &pubkey).unwrap();
        verify_sshsig(&file_sig, payload, "file", &pubkey).unwrap();

        // Cross-namespace replay fails both directions.
        let err = verify_sshsig(&git_sig, payload, "file", &pubkey).unwrap_err();
        assert_eq!(err, SshSigVerifyError::NamespaceMismatch);
        let err = verify_sshsig(&file_sig, payload, "git", &pubkey).unwrap_err();
        assert_eq!(err, SshSigVerifyError::NamespaceMismatch);
    }

    #[test]
    fn verify_rejects_wrong_key_tampered_payload_and_garbage() {
        let key = test_keypair();
        let other = test_keypair();
        let pem = key.to_openssh(ssh_key::LineEnding::LF).unwrap();
        let backend = backend_for(pem.as_str());
        let pubkey = authorized_keys(&key);
        let other_pubkey = authorized_keys(&other);
        let payload = b"release artifact v1.2.3";

        let sig = backend
            .sign("rel", &SignatureScheme::SshSig, "git", payload)
            .unwrap();

        // Right signature, wrong key.
        let err = verify_sshsig(&sig, payload, "git", &other_pubkey).unwrap_err();
        assert_eq!(err, SshSigVerifyError::KeyMismatch);

        // Right key, tampered payload (one flipped byte changes the
        // namespace-bound preimage hash).
        let mut tampered = payload.to_vec();
        tampered[0] ^= 0x01;
        let err = verify_sshsig(&sig, &tampered, "git", &pubkey).unwrap_err();
        assert_eq!(err, SshSigVerifyError::SignatureInvalid);

        // Garbage armor and garbage public key are parse failures.
        let err = verify_sshsig(b"not-a-signature", payload, "git", &pubkey).unwrap_err();
        assert!(matches!(err, SshSigVerifyError::ParseError { .. }));
        let err = verify_sshsig(&sig, payload, "git", "not-a-key").unwrap_err();
        assert!(matches!(err, SshSigVerifyError::ParseError { .. }));
    }

    #[test]
    fn backend_refuses_raw_scheme_and_unknown_keys() {
        let key = test_keypair();
        let pem = key.to_openssh(ssh_key::LineEnding::LF).unwrap();
        let backend = backend_for(pem.as_str());

        // Defense in depth: the service denies `raw` first, but the backend
        // never serves it either.
        let err = backend
            .sign("rel", &SignatureScheme::Raw, "git", b"x")
            .unwrap_err();
        assert!(
            err.contains("raw"),
            "raw refusal must name the scheme, got: {err}"
        );

        // Unknown key references fail as strings (the service maps these to
        // BackendError denials); the reference itself is log-safe.
        let err = backend
            .sign("nope", &SignatureScheme::SshSig, "git", b"x")
            .unwrap_err();
        assert!(err.contains("nope"), "got: {err}");
    }

    // --- backend selection behind the service trait ---

    fn sealed_service(backend: SealedKeyBackend) -> SigningService<SealedKeyBackend> {
        let plan = CredentialsPlan {
            ssh: Vec::new(),
            signing: vec![SigningGrantPlan {
                name: "rel".to_string(),
                material: "SIGN_KEY".to_string(),
                namespace: "git".to_string(),
                on_violation: SecretViolationPolicy::Block,
                binding: CredentialBinding::Broker,
            }],
            strict: false,
            strict_origin: None,
        };
        let store = GrantStore::compile(&[("personal-pi", &plan)]);
        SigningService::new(store, LimitsConfig::default(), backend)
    }

    #[test]
    fn service_with_sealed_backend_signs_and_audits_allow() {
        let key = test_keypair();
        let pem = key.to_openssh(ssh_key::LineEnding::LF).unwrap();
        let secrets = layer("SIGN_KEY_ENV", pem.as_str());
        let env_vars = HashMap::from([("SIGN_KEY".to_string(), "SIGN_KEY_ENV".to_string())]);
        let plan = CredentialsPlan {
            ssh: Vec::new(),
            signing: vec![SigningGrantPlan {
                name: "rel".to_string(),
                material: "SIGN_KEY".to_string(),
                namespace: "git".to_string(),
                on_violation: SecretViolationPolicy::Block,
                binding: CredentialBinding::Broker,
            }],
            strict: false,
            strict_origin: None,
        };
        let backend = SealedKeyBackend::from_credentials_plan(&plan, &secrets, &env_vars).unwrap();
        let svc = sealed_service(backend);

        let audit = AuditLog::memory();
        let req = SignRequest {
            operation_id: "op-1".to_string(),
            key_reference: "rel".to_string(),
            signature_scheme: SignatureScheme::SshSig,
            namespace: "git".to_string(),
            payload: b"payload-bytes".to_vec(),
            optional_context: None,
        };
        let resp = svc
            .sign_at("personal-pi", 7, &req, &audit, "2026-09-06T00:00:00Z", 1000)
            .unwrap();
        assert_eq!(resp.operation_id, "op-1");
        assert_eq!(resp.key_id, "SIGN_KEY");
        let text = std::str::from_utf8(&resp.signature).unwrap();
        assert!(
            text.starts_with("-----BEGIN SSH SIGNATURE-----"),
            "service must serve real armor through the sealed backend, got: {text}"
        );

        // The armored payload round-trips against the same key.
        verify_sshsig(
            &resp.signature,
            b"payload-bytes",
            "git",
            &authorized_keys(&key),
        )
        .unwrap();

        let records = audit.snapshot();
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0].result, AuditResult::Allow));

        // Grant-pipeline denials still fire ahead of the backend: unknown
        // keys never reach sealed material.
        let mut bad = req.clone();
        bad.key_reference = "nope".to_string();
        let err = svc
            .sign_at("personal-pi", 7, &bad, &audit, "t", 1001)
            .unwrap_err();
        assert!(
            matches!(err, super::super::signing::Denial::UnknownKey { .. }),
            "unknown key must deny at the grant layer, got: {err}"
        );
    }

    // --- custody: no plaintext in Debug ---

    #[test]
    fn debug_impls_never_carry_plaintext() {
        // Sound redaction check (no memory scans): the armored PEM body is
        // distinctive base64 — if any Debug impl echoed key text, the body
        // would appear verbatim in the rendering.
        let pem = ed25519_openssh();
        let secrets = layer("SIGN_KEY_ENV", &pem);
        let mat = SopsKeyMaterial::from_decrypted(&secrets, "SIGN_KEY", "SIGN_KEY_ENV").unwrap();
        let rendered = format!("{mat:?}");
        assert!(
            rendered.contains("SIGN_KEY"),
            "Debug keeps the provenance label, got: {rendered}"
        );
        assert!(
            rendered.contains("<redacted>"),
            "Debug marks the hidden key, got: {rendered}"
        );
        for line in pem.lines().skip(1) {
            let line = line.trim();
            if line.len() > 16 {
                assert!(!rendered.contains(line), "Debug must not echo key text");
            }
        }

        let backend = backend_for(&pem);
        let rendered = format!("{backend:?}");
        assert!(
            rendered.contains("rel"),
            "backend Debug keeps key references, got: {rendered}"
        );
        for line in pem.lines().skip(1) {
            let line = line.trim();
            if line.len() > 16 {
                assert!(
                    !rendered.contains(line),
                    "backend Debug must not echo key text"
                );
            }
        }
    }
}
