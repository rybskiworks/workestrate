//! Host-owned issuance of the managed broker's SSH host identity.
//!
//! The existing secret owner supplies the CA; this module does not load or
//! persist another secret store. Only the broker public key is needed to issue
//! its certificate. The CA private key never becomes part of guest material.

use std::io;

use ssh_key::{Algorithm, Certificate, HashAlg, PrivateKey, PublicKey, certificate};

/// A literal DNS-shaped alias used only for managed broker SSH destinations.
/// It cannot introduce wildcards, negation, extra hosts or configuration lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerHostPrincipal(String);

/// Public identity artifacts only. The service receives its own separately
/// provisioned host private key, never the host's certificate authority key.
#[derive(Clone, Debug)]
pub struct BrokerHostTrust {
    principal: BrokerHostPrincipal,
    certificate: String,
    ca_public_key: String,
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

impl BrokerHostPrincipal {
    pub fn new(value: &str) -> io::Result<Self> {
        if value.is_empty()
            || value.len() > 253
            || value.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || !label.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                    })
            })
        {
            return Err(invalid(
                "managed broker principal must be a literal lowercase DNS alias",
            ));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Issue a host certificate with one exact principal and finite validity.
    /// The lifecycle owner supplies a durable serial and renews before expiry.
    /// No default-user certificate or all-principals certificate is issued.
    pub fn issue(
        &self,
        ca: &PrivateKey,
        broker_public_key: &PublicKey,
        serial: u64,
        valid_after: u64,
        valid_before: u64,
    ) -> io::Result<BrokerHostTrust> {
        if ca.algorithm() != Algorithm::Ed25519
            || ca.is_encrypted()
            || broker_public_key.algorithm() != Algorithm::Ed25519
            || ca.public_key().key_data() == broker_public_key.key_data()
            || serial == 0
            || valid_before <= valid_after
            || valid_before == u64::MAX
        {
            return Err(invalid("invalid managed broker certificate parameters"));
        }
        let mut nonce = [0u8; 32];
        getrandom::fill(&mut nonce)
            .map_err(|_| io::Error::other("broker certificate entropy unavailable"))?;
        let certificate = (|| -> ssh_key::Result<Certificate> {
            let mut builder = certificate::Builder::new(
                nonce.to_vec(),
                broker_public_key.key_data().clone(),
                valid_after,
                valid_before,
            )?;
            builder.serial(serial)?;
            builder.cert_type(certificate::CertType::Host)?;
            builder.key_id(self.as_str())?;
            builder.valid_principal(self.as_str())?;
            builder.sign(ca)
        })()
        .map_err(|_| invalid("managed broker certificate issuance failed"))?;
        certificate
            .validate_at(valid_after, &[ca.public_key().fingerprint(HashAlg::Sha256)])
            .map_err(|_| invalid("issued broker certificate did not validate"))?;
        // Strip comments from the public key: even an operator-supplied key
        // comment must not add another line to a guest known_hosts file.
        let public = PublicKey::new(ca.public_key().key_data().clone(), "");
        Ok(BrokerHostTrust {
            principal: self.clone(),
            certificate: certificate
                .to_openssh()
                .map_err(|_| invalid("broker certificate encoding failed"))?,
            ca_public_key: public
                .to_openssh()
                .map_err(|_| invalid("broker certificate authority encoding failed"))?,
        })
    }
}

impl BrokerHostTrust {
    pub fn principal(&self) -> &BrokerHostPrincipal {
        &self.principal
    }

    pub fn certificate(&self) -> &str {
        &self.certificate
    }

    pub fn ca_public_key(&self) -> &str {
        &self.ca_public_key
    }

    /// Match only the managed HostKeyAlias, never every remote host. Client
    /// configuration must select this alias only for broker-bound destinations
    /// and require certificate authentication with StrictHostKeyChecking.
    pub fn known_hosts_entry(&self) -> String {
        format!(
            "@cert-authority {} {}\n",
            self.principal.as_str(),
            self.ca_public_key
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn key() -> PrivateKey {
        PrivateKey::random(&mut ssh_key::rand_core::OsRng, Algorithm::Ed25519).unwrap()
    }

    #[test]
    fn identity_is_an_exact_host_certificate_signed_by_the_supplied_authority() {
        let ca = key();
        let broker = key();
        let principal = BrokerHostPrincipal::new("broker.personal.invalid").unwrap();
        let issued = principal
            .issue(&ca, broker.public_key(), 17, 100, 200)
            .unwrap();
        let certificate = Certificate::from_openssh(&issued.certificate).unwrap();
        assert_eq!(certificate.cert_type(), certificate::CertType::Host);
        assert_eq!(certificate.valid_principals(), &["broker.personal.invalid"]);
        assert_eq!(certificate.public_key(), broker.public_key().key_data());
        assert_eq!(certificate.serial(), 17);
        assert!(certificate.critical_options().is_empty());
        assert!(certificate.extensions().is_empty());
        let fingerprint = ca.public_key().fingerprint(HashAlg::Sha256);
        certificate
            .validate_at(100, &[fingerprint.clone()])
            .unwrap();
        certificate
            .validate_at(199, &[fingerprint.clone()])
            .unwrap();
        assert!(certificate.validate_at(99, &[fingerprint.clone()]).is_err());
        assert!(certificate.validate_at(200, &[fingerprint]).is_err());
        assert!(
            certificate
                .validate_at(150, &[key().public_key().fingerprint(HashAlg::Sha256)])
                .is_err()
        );
    }

    #[test]
    fn principal_rejects_ssh_configuration_and_pattern_injection() {
        for value in [
            "",
            "*",
            "!broker",
            "broker,other",
            "broker other",
            "broker\nHost *",
            "broker\r",
            "broker\0",
            "UPPER",
            "[broker]:22",
            "-broker",
            "broker-",
            "broker..invalid",
            "broker.",
            ".broker",
            "broker%h",
        ] {
            assert!(
                BrokerHostPrincipal::new(value).is_err(),
                "accepted {value:?}"
            );
        }
        assert!(BrokerHostPrincipal::new(&"a".repeat(64)).is_err());
        assert!(
            BrokerHostPrincipal::new(&format!(
                "{}.{}.{}.{}",
                "a".repeat(63),
                "b".repeat(63),
                "c".repeat(63),
                "d".repeat(63)
            ))
            .is_err()
        );
        assert!(BrokerHostPrincipal::new("broker-1.state-domain.invalid").is_ok());
    }

    #[test]
    fn existing_sealed_material_owner_issues_without_exposing_its_private_key() {
        use super::super::key_material::SopsKeyMaterial;

        let ca = key();
        let encoded = ca.to_openssh(ssh_key::LineEnding::LF).unwrap();
        let material = SopsKeyMaterial::from_resolver("broker-ca", "BROKER_CA_KEY", &|name| {
            (name == "BROKER_CA_KEY").then(|| encoded.to_string())
        })
        .unwrap();
        let issued = material
            .issue_broker_host_certificate(
                &BrokerHostPrincipal::new("broker.personal.invalid").unwrap(),
                key().public_key(),
                3,
                100,
                200,
            )
            .unwrap();
        Certificate::from_openssh(issued.certificate())
            .unwrap()
            .validate_at(150, &[ca.public_key().fingerprint(HashAlg::Sha256)])
            .unwrap();
        assert_eq!(material.secret_id(), "broker-ca");
        assert!(!format!("{material:?}{issued:?}").contains(encoded.as_str()));
    }

    #[test]
    fn no_ca_private_material_or_key_comments_enter_public_trust_artifacts() {
        let mut ca = key();
        ca.set_comment("operator\n@cert-authority * unexpected");
        let broker = key();
        let issued = BrokerHostPrincipal::new("broker.fleet.invalid")
            .unwrap()
            .issue(&ca, broker.public_key(), 1, 100, 200)
            .unwrap();
        let known = issued.known_hosts_entry();
        assert_eq!(known.lines().count(), 1);
        assert!(known.starts_with("@cert-authority broker.fleet.invalid ssh-ed25519 "));
        assert!(!known.contains("operator"));
        assert!(!known.contains('*'));
        assert!(!format!("{issued:?}").contains("PRIVATE KEY"));
        assert_eq!(
            PublicKey::from_openssh(&issued.ca_public_key)
                .unwrap()
                .key_data(),
            ca.public_key().key_data()
        );
    }

    #[test]
    fn broker_cannot_reuse_the_ca_key_or_obtain_an_unbounded_certificate() {
        let ca = key();
        let broker = key();
        let principal = BrokerHostPrincipal::new("broker.fleet.invalid").unwrap();
        assert!(principal.issue(&ca, ca.public_key(), 1, 100, 200).is_err());
        for (serial, after, before) in [
            (0, 100, 200),
            (1, 100, 100),
            (1, 200, 100),
            (1, 100, u64::MAX),
        ] {
            assert!(
                principal
                    .issue(&ca, broker.public_key(), serial, after, before)
                    .is_err()
            );
        }
        let first = principal
            .issue(&ca, broker.public_key(), 1, 100, 200)
            .unwrap();
        let second = principal
            .issue(&ca, broker.public_key(), 2, 100, 200)
            .unwrap();
        assert_ne!(
            Certificate::from_openssh(&first.certificate)
                .unwrap()
                .nonce(),
            Certificate::from_openssh(&second.certificate)
                .unwrap()
                .nonce()
        );
    }
}
