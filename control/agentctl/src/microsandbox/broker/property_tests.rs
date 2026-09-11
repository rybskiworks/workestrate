//! Generated checks of the compiled-policy and launch-identity contracts.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;

use super::epoch::EpochToken;
use super::registry::{CID_ALLOC_BASE, CidRegistry};
use super::signing::GrantStore;
use crate::config::SecretViolationPolicy;
use crate::microsandbox::plan::{CredentialBinding, CredentialsPlan, SshGrantPlan};

#[derive(Debug)]
struct LaunchModel {
    cid: u32,
    owner: String,
    token: EpochToken,
    live: bool,
    wire_sequence: u64,
}

fn credentials(ssh: Vec<SshGrantPlan>) -> CredentialsPlan {
    CredentialsPlan {
        ssh,
        signing: Vec::new(),
        strict: false,
        strict_origin: None,
    }
}

proptest! {
    #[test]
    fn launch_tokens_bind_every_byte_and_the_exact_cid(
        bytes in any::<[u8; 32]>(), cid in 3u32..u32::MAX,
        index in 0usize..32, difference in 1u8..=255,
    ) {
        let expected = EpochToken::from_bytes(bytes);
        prop_assert!(expected.verify(cid, &expected, cid).is_ok());
        let mut other = bytes;
        other[index] ^= difference;
        prop_assert!(EpochToken::from_bytes(other).verify(cid, &expected, cid).is_err());
        prop_assert!(expected.verify(cid, &expected, cid ^ 1).is_err());
        let encoded: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        prop_assert_eq!(expected.to_hex(), encoded.as_str());
        prop_assert_eq!(EpochToken::from_hex(&encoded).unwrap(), expected);
        prop_assert!(EpochToken::from_hex(&encoded[..encoded.len() - 2]).is_err());
        prop_assert!(EpochToken::from_hex(&(encoded + "00")).is_err());
    }

    #[test]
    fn destination_selection_preserves_records_and_narrowing_never_adds_authority(
        rows in proptest::collection::vec((0u8..5, 0u8..4, any::<bool>(), any::<bool>()), 0..17),
        queried_host in 0u8..4, queried_port in 0u8..4, uppercase in any::<bool>(),
    ) {
        let full: Vec<_> = rows.iter().enumerate().map(|(i, &(host, port, broker, _))| SshGrantPlan {
            name: format!("credential-{i}"), material: format!("MATERIAL_{i}"),
            hosts: vec![if host == 4 { "*.example.test".to_owned() } else { format!("node-{host}.example.test") }],
            ports: vec![2200 + u16::from(port)], users: vec![format!("user-{i}")],
            binding: if broker { CredentialBinding::Broker } else { CredentialBinding::Guest },
            on_violation: SecretViolationPolicy::Passthrough,
        }).collect();
        let narrowed: Vec<_> = full.iter().zip(&rows)
            .filter(|(_, row)| row.3).map(|(record, _)| record.clone()).collect();
        let expected: Vec<_> = full.iter().zip(&rows)
            .filter(|(_, row)| (row.0 == 4 || row.0 == queried_host) && row.1 == queried_port)
            .map(|(record, _)| record.clone()).collect();
        let expected_narrow: Vec<_> = full.iter().zip(&rows)
            .filter(|(_, row)| row.3 && (row.0 == 4 || row.0 == queried_host) && row.1 == queried_port)
            .map(|(record, _)| record.clone()).collect();
        let mut host = format!("node-{queried_host}.example.test");
        if uppercase { host.make_ascii_uppercase(); }
        let port = 2200 + u16::from(queried_port);
        let full = credentials(full);
        let narrowed = credentials(narrowed);
        let store = GrantStore::compile(&[("current", &full), ("candidate", &narrowed)]);
        let actual: Vec<_> = store.ssh_credentials_for_destination("current", &host, port).cloned().collect();
        let actual_narrow: Vec<_> = store.ssh_credentials_for_destination("candidate", &host, port).cloned().collect();
        prop_assert_eq!(&actual, &expected);
        prop_assert_eq!(&actual_narrow, &expected_narrow);
        prop_assert!(actual_narrow.iter().all(|record| actual.contains(record)));
        prop_assert_eq!(store.ssh_is_broker_bound("current", &host, port), expected.iter().any(|record| record.binding == CredentialBinding::Broker));
        prop_assert!(!store.ssh_authorized("unknown", &host, port));
        prop_assert!(!store.ssh_authorized("current", "outside.invalid", port));
    }

    #[test]
    fn registry_sequences_keep_independent_launches_and_durable_revisions(
        operations in proptest::collection::vec((0u8..4, 0usize..16, any::<bool>()), 1..17),
    ) {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = CidRegistry::open(dir.path()).unwrap();
        let mut model: Vec<LaunchModel> = Vec::new();
        for (operation, selector, exact) in operations {
            if operation == 0 || model.is_empty() {
                let owner = format!("workload-{}", selector % 3);
                let token = EpochToken::from_bytes([model.len() as u8; 32]);
                let cid = registry.allocate_at(&owner, &token, 100).unwrap();
                prop_assert_eq!(cid, CID_ALLOC_BASE + model.len() as u32);
                model.push(LaunchModel { cid, owner, token, live: true, wire_sequence: 0 });
            } else {
                let index = selector % model.len();
                let entry = &mut model[index];
                match operation {
                    1 => {
                        let result = registry.bump_wire_epoch(entry.cid);
                        if entry.live {
                            entry.wire_sequence += 1;
                            prop_assert_eq!(result.unwrap(), entry.wire_sequence);
                        } else { prop_assert!(result.is_err()); }
                    },
                    2 => {
                        let owner = if exact { entry.owner.as_str() } else { "wrong-owner" };
                        let expired = registry.expire_binding(entry.cid, owner, &entry.token).unwrap();
                        prop_assert_eq!(expired, entry.live && exact);
                        if expired { entry.live = false; }
                    },
                    _ => registry = CidRegistry::open(dir.path()).unwrap(),
                }
            }
            // Reopen independently after every transition: checking only the
            // same object's cache would miss failed publication or reloads.
            let observer = CidRegistry::open(dir.path()).unwrap();
            prop_assert_eq!(observer.live_count().unwrap(), model.iter().filter(|entry| entry.live).count());
            for expected in &model {
                let actual = observer.lookup(expected.cid).unwrap();
                if expected.live {
                    let actual = actual.unwrap();
                    prop_assert_eq!(&actual.instance, &expected.owner);
                    prop_assert_eq!(actual.epoch_hex, expected.token.to_hex());
                    prop_assert_eq!(actual.wire_epoch, expected.wire_sequence);
                } else { prop_assert!(actual.is_none()); }
            }
        }
    }
}
