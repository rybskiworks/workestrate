//! SSH epoch provisioning over the console agent channel.
//!
//! Host-side sender for the generation-8 `core.ssh_epoch.provision` /
//! `core.ssh_epoch.ack` family (wire names, payload shapes, and the gen-8
//! introduction technically described by the fork's
//! `crates/protocol/schema/gen-8.json`): the host provisions the current
//! per-CID wire epoch to the guest after launch, and the guest acks it.
//! Provisioning is REPEATABLE — the re-issue path ([`crate::microsandbox::broker::ssh_lifecycle::reprovision_epoch`])
//! bumps the persisted sequence and sends again on re-attestation or fork
//! recovery.
//!
//! Console mechanism actually used: workestrate speaks the console protocol
//! nowhere else — there is no pre-existing host-side console client in this
//! codebase (it drives sandboxes through the `SandboxBuilder`/`Sandbox` SDK
//! API). The channel here is the SDK's agent client
//! (`microsandbox::agent::AgentClient`, the console agent channel to the
//! sandbox's agent relay socket, resolved by sandbox name): a one-shot
//! [`ConsoleChannel::request`] (the client's `request_raw` — flags plus the
//! CBOR envelope body) whose correlation id the client assigns from the
//! relay-handed range (nonzero by construction; the response is matched to
//! the request by that id, so a mismatched ack cannot be mistaken for ours).
//! Flags are always 0 for both directions of this family.
//!
//! Wire encoding is hand-rolled onto these contract shapes (outer frame
//! `[len:u32BE][id:u32BE][flags:u8][CBOR]`, envelope `{v,t,p}` with `t` as
//! the wire string and `p` as a CBOR byte string) with `ciborium` (already a
//! direct dependency) plus `serde_bytes` for the byte-string `p`, mirroring
//! the fork protocol crate's envelope exactly. This is deliberate: the
//! pinned fork predates the typed gen-8 additions, so naming their SDK types
//! would not compile against the pin. The byte layout is asserted by the
//! frame-shape tests below and stays source-compatible across the pin move.
//!
//! Fail-closed throughout: a peer negotiated below
//! [`EPOCH_INTRO_GENERATION`] is refused LOCALLY (no new frame is ever
//! emitted to an old peer); send failures, undecodable acks, and ack
//! mismatches (wrong cid/epoch, `ok: false`, wrong wire type) are typed
//! [`EpochProvisionError`]s, never silent success.

use serde::{Deserialize, Serialize};

/// Protocol generation that introduced the SSH epoch family: the sender
/// refuses locally against any peer negotiated below this (never emits the
/// new frames to old peers).
pub const EPOCH_INTRO_GENERATION: u8 = 8;

/// Maximum accepted distance between `issued_at` and the broker clock, in
/// either direction (seconds). Past it the provision is stale or from the
/// future; either way fail closed. Matches the divert prelude's skew window
/// idiom ([`crate::microsandbox::broker::shim::MAX_DIVERT_EPOCH_SKEW_SECS`]).
pub const MAX_PROVISION_CLOCK_SKEW_SECS: u64 = 300;

/// Timeout for dialing the sandbox's agent relay socket. Matches the SDK
/// client's own default handshake timeout.
pub const CONSOLE_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Wire name of the host-to-guest provision message.
pub const SSH_EPOCH_PROVISION_WIRE: &str = "core.ssh_epoch.provision";

/// Wire name of the guest-to-host ack message.
pub const SSH_EPOCH_ACK_WIRE: &str = "core.ssh_epoch.ack";

/// Payload for `core.ssh_epoch.provision`: which sandbox/CID this wire
/// epoch belongs to, the epoch itself, and its validity window (Unix
/// seconds). Field-for-field the fork's gen-8 shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshEpochProvision {
    pub instance: String,
    pub cid: u32,
    pub epoch: u64,
    pub issued_at: u64,
    pub not_before: u64,
}

/// Payload for `core.ssh_epoch.ack`: the provision being acknowledged plus
/// whether the guest accepted it. Field-for-field the fork's gen-8 shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshEpochAck {
    pub cid: u32,
    pub epoch: u64,
    pub ok: bool,
}

/// The console envelope body (`{v,t,p}`): generation, wire-type string, and
/// CBOR payload bytes as a byte string (the fork protocol crate encodes `p`
/// with `serde_bytes` — an array-of-ints encoding here would fail the
/// guest's envelope decode).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ConsoleEnvelope<'a> {
    v: u8,
    t: &'a str,
    #[serde(with = "serde_bytes")]
    p: Vec<u8>,
}

/// Decode side of [`ConsoleEnvelope`] (owned wire-type string).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct DecodedEnvelope {
    v: u8,
    t: String,
    #[serde(with = "serde_bytes")]
    p: Vec<u8>,
}

/// Epoch delivery failure. Every variant is fail-closed: the caller must
/// treat the guest as unprovisioned (dispatch keeps rejecting its signing
/// preludes; the re-issue path retries explicitly).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpochProvisionError {
    /// The peer's negotiated generation predates the epoch family: refused
    /// locally, nothing was sent.
    GenerationGate { peer_generation: u8, required: u8 },
    /// The provision's clock fields are incoherent (`not_before` after
    /// `issued_at`), too far from the broker clock, or there is no broker
    /// clock to check against: refused locally, nothing was sent.
    ClockAnomaly { detail: String },
    /// Dialing the agent relay, the handshake, the pre-send registry bump,
    /// or the send itself failed. Covers everything before an ack exists.
    SendFailed { detail: String },
    /// An ack arrived but is unusable: undecodable envelope, wrong wire
    /// type, or a cid/epoch/`ok` that does not match the provision
    /// (stale echo, spoof, or guest-side rejection).
    AckMismatch { detail: String },
    /// No live registry binding exists for the CID (unknown or tombstoned):
    /// there is nothing to provision.
    NotBound { cid: u32 },
}

impl std::fmt::Display for EpochProvisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EpochProvisionError::GenerationGate {
                peer_generation,
                required,
            } => write!(
                f,
                "epoch provision refused: peer speaks generation {peer_generation}, \
                 the SSH epoch family needs generation {required} (nothing sent)"
            ),
            EpochProvisionError::ClockAnomaly { detail } => {
                write!(f, "epoch provision refused: clock anomaly: {detail}")
            }
            EpochProvisionError::SendFailed { detail } => {
                write!(f, "epoch provision send failed: {detail}")
            }
            EpochProvisionError::AckMismatch { detail } => {
                write!(f, "epoch provision ack rejected: {detail}")
            }
            EpochProvisionError::NotBound { cid } => {
                write!(f, "epoch provision refused: CID {cid} has no live binding")
            }
        }
    }
}

impl std::error::Error for EpochProvisionError {}

/// One console response: the transport-assigned correlation id plus the raw
/// frame's flags and CBOR envelope body. The id echoes the request's id (the
/// channel matches them); the body is decoded by [`provision_epoch`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleResponse {
    pub id: u32,
    pub flags: u8,
    pub body: Vec<u8>,
}

/// The console agent channel [`provision_epoch`] sends through. The
/// production implementation is [`AgentConsoleChannel`] (the SDK agent
/// client over the sandbox's relay socket); tests inject fakes. Kept to
/// plain bytes (flags + envelope body) so no fork protocol types cross this
/// boundary — the pin predates their typed gen-8 additions.
pub trait ConsoleChannel {
    /// The generation negotiated with the peer at handshake (the capability
    /// gate is checked against this BEFORE any byte is sent).
    fn negotiated_generation(&self) -> u8;

    /// One-shot request: send one `(flags, envelope body)` frame and wait
    /// for the frame with the matching correlation id. Desugared (rather
    /// than `async fn`) so callers may require `Send` on the future
    /// (the multi-thread runtime needs it; `async fn` would drop the bound).
    #[allow(clippy::manual_async_fn)]
    fn request(
        &self,
        flags: u8,
        body: Vec<u8>,
    ) -> impl Future<Output = Result<ConsoleResponse, String>> + Send;
}

/// [`ConsoleChannel`] over the sandbox's agent relay socket via the SDK
/// agent client (the console agent channel).
pub struct AgentConsoleChannel {
    client: microsandbox::agent::AgentClient,
}

impl std::fmt::Debug for AgentConsoleChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentConsoleChannel")
            .field("negotiated_generation", &self.negotiated_generation())
            .finish()
    }
}

impl AgentConsoleChannel {
    /// Dial the sandbox's agent relay (`sandbox_name` is the microsandbox
    /// sandbox name, resolved to its relay socket by the SDK) and run the
    /// relay handshake, which yields the negotiated generation.
    pub async fn connect(sandbox_name: &str) -> Result<Self, EpochProvisionError> {
        Self::connect_with_timeout(
            sandbox_name,
            std::time::Duration::from_secs(CONSOLE_CONNECT_TIMEOUT_SECS),
        )
        .await
    }

    /// [`Self::connect`] with an explicit dial/handshake timeout.
    pub async fn connect_with_timeout(
        sandbox_name: &str,
        timeout: std::time::Duration,
    ) -> Result<Self, EpochProvisionError> {
        let client = microsandbox::agent::connect_sandbox_with_timeout(sandbox_name, timeout)
            .await
            .map_err(|e| EpochProvisionError::SendFailed {
                detail: format!("console dial for sandbox '{sandbox_name}' failed: {e}"),
            })?;
        Ok(Self { client })
    }
}

impl ConsoleChannel for AgentConsoleChannel {
    fn negotiated_generation(&self) -> u8 {
        self.client.negotiated_version()
    }

    #[allow(clippy::manual_async_fn)]
    fn request(
        &self,
        flags: u8,
        body: Vec<u8>,
    ) -> impl Future<Output = Result<ConsoleResponse, String>> + Send {
        async move {
            let frame = self
                .client
                .request_raw(flags, body)
                .await
                .map_err(|e| e.to_string())?;
            Ok(ConsoleResponse {
                id: frame.id,
                flags: frame.flags,
                body: frame.body,
            })
        }
    }
}

/// Current Unix epoch seconds (test seam: [`provision_epoch`] takes the
/// clock explicitly; this is the production default).
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Validate the provision's clock fields against `now_secs` (fail-closed,
/// before anything is sent): `not_before` must not postdate `issued_at`,
/// and `issued_at` must sit within [`MAX_PROVISION_CLOCK_SKEW_SECS`] of now
/// in either direction. A zero broker clock fails closed — without a clock,
/// freshness is unverifiable (the divert path's zero-clock rule).
fn check_clock(issued_at: u64, not_before: u64, now_secs: u64) -> Result<(), EpochProvisionError> {
    if now_secs == 0 {
        return Err(EpochProvisionError::ClockAnomaly {
            detail: "broker clock unavailable, freshness cannot be verified".to_string(),
        });
    }
    if not_before > issued_at {
        return Err(EpochProvisionError::ClockAnomaly {
            detail: format!("not_before ({not_before}) postdates issued_at ({issued_at})"),
        });
    }
    let skew = issued_at.abs_diff(now_secs);
    if skew > MAX_PROVISION_CLOCK_SKEW_SECS {
        let direction = if issued_at > now_secs {
            "in the future"
        } else {
            "stale"
        };
        return Err(EpochProvisionError::ClockAnomaly {
            detail: format!(
                "issued_at ({issued_at}) is {direction} (skew {skew}s exceeds {MAX_PROVISION_CLOCK_SKEW_SECS}s)"
            ),
        });
    }
    Ok(())
}

/// Encode one console envelope body (`{v,t,p}` CBOR).
fn encode_envelope(
    generation: u8,
    wire: &str,
    payload_cbor: Vec<u8>,
) -> Result<Vec<u8>, EpochProvisionError> {
    let envelope = ConsoleEnvelope {
        v: generation,
        t: wire,
        p: payload_cbor,
    };
    let mut body = Vec::new();
    ciborium::into_writer(&envelope, &mut body).map_err(|e| EpochProvisionError::SendFailed {
        detail: format!("provision envelope encode failed: {e}"),
    })?;
    Ok(body)
}

/// Encode any serde value to CBOR bytes (payload layer).
fn encode_payload<T: Serialize>(value: &T) -> Result<Vec<u8>, EpochProvisionError> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf).map_err(|e| EpochProvisionError::SendFailed {
        detail: format!("provision payload encode failed: {e}"),
    })?;
    Ok(buf)
}

/// Decode the ack frame body and check it against the provision (fail-closed
/// on every mismatch): the envelope must decode, its wire type must be the
/// ack, the inner payload must decode, and cid/epoch/`ok` must all agree
/// with what was sent. A wrong epoch or cid is a stale echo or a spoof —
/// rejected, never accepted.
fn check_ack(
    body: &[u8],
    expected_cid: u32,
    expected_epoch: u64,
) -> Result<(), EpochProvisionError> {
    let mismatch = |detail: String| EpochProvisionError::AckMismatch { detail };
    let envelope: DecodedEnvelope = ciborium::from_reader(body)
        .map_err(|e| mismatch(format!("ack envelope undecodable: {e}")))?;
    if envelope.t != SSH_EPOCH_ACK_WIRE {
        return Err(mismatch(format!(
            "ack wire type is '{}', expected '{SSH_EPOCH_ACK_WIRE}'",
            envelope.t
        )));
    }
    let ack: SshEpochAck = ciborium::from_reader(&envelope.p[..])
        .map_err(|e| mismatch(format!("ack payload undecodable: {e}")))?;
    if ack.cid != expected_cid {
        return Err(mismatch(format!(
            "ack cid {} does not match provisioned cid {expected_cid}",
            ack.cid
        )));
    }
    if ack.epoch != expected_epoch {
        return Err(mismatch(format!(
            "ack epoch {} does not match provisioned epoch {expected_epoch}",
            ack.epoch
        )));
    }
    if !ack.ok {
        return Err(mismatch(format!(
            "guest rejected the provision for cid {expected_cid} epoch {expected_epoch}"
        )));
    }
    let _ = envelope.v;
    Ok(())
}

/// Provision one epoch over the console agent channel: validate clocks,
/// gate on the negotiated generation (refused locally below
/// [`EPOCH_INTRO_GENERATION`] — the new frames never reach old peers), send
/// with flags 0, and check the ack against the provision. Returns the
/// provisioned wire epoch. The envelope generation written is the negotiated
/// peer generation (self-describing per the protocol's own `v` rule).
pub async fn provision_epoch(
    channel: &impl ConsoleChannel,
    provision: &SshEpochProvision,
    now_secs: u64,
) -> Result<u64, EpochProvisionError> {
    check_clock(provision.issued_at, provision.not_before, now_secs)?;
    let negotiated = channel.negotiated_generation();
    if negotiated < EPOCH_INTRO_GENERATION {
        return Err(EpochProvisionError::GenerationGate {
            peer_generation: negotiated,
            required: EPOCH_INTRO_GENERATION,
        });
    }
    let payload = encode_payload(provision)?;
    let body = encode_envelope(negotiated, SSH_EPOCH_PROVISION_WIRE, payload)?;
    // Flags are always 0 for this family; the channel assigns the nonzero
    // correlation id and matches the response to it.
    let response =
        channel
            .request(0, body)
            .await
            .map_err(|detail| EpochProvisionError::SendFailed {
                detail: format!("console request failed: {detail}"),
            })?;
    check_ack(&response.body, provision.cid, provision.epoch)?;
    Ok(provision.epoch)
}

/// [`provision_epoch`] with the production clock (`issued_at`/`not_before`
/// stamped at send time, so the provision is immediately valid).
pub async fn provision_now(
    channel: &impl ConsoleChannel,
    instance: &str,
    cid: u32,
    wire_epoch: u64,
) -> Result<u64, EpochProvisionError> {
    let now = now_secs();
    provision_epoch(
        channel,
        &SshEpochProvision {
            instance: instance.to_string(),
            cid,
            epoch: wire_epoch,
            issued_at: now,
            not_before: now,
        },
        now,
    )
    .await
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// How the fake channel answers a provision.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FakeAck {
        /// Decode the provision and ack it exactly (the happy path).
        Echo,
        /// Ack a different CID (spoof/wrong-binding).
        WrongCid,
        /// Ack a different epoch (stale echo).
        WrongEpoch,
        /// Well-formed ack with `ok: false` (guest-side rejection).
        NotOk,
        /// Answer with the provision wire type (not the ack).
        WrongType,
        /// Answer with undecodable bytes.
        Corrupt,
    }

    /// In-memory [`ConsoleChannel`]: records `(flags, body)` per request and
    /// answers from [`FakeAck`]. The response id is nonzero (11, mirroring
    /// the fork's own epoch codec test) — the request id itself is assigned
    /// inside the real client, out of this layer's reach.
    struct FakeConsoleChannel {
        negotiated: u8,
        ack: FakeAck,
        fail_send: Option<String>,
        sent: Mutex<Vec<(u8, Vec<u8>)>>,
    }

    impl FakeConsoleChannel {
        fn echoing(negotiated: u8) -> Self {
            Self {
                negotiated,
                ack: FakeAck::Echo,
                fail_send: None,
                sent: Mutex::new(Vec::new()),
            }
        }

        fn sent_bodies(&self) -> Vec<(u8, Vec<u8>)> {
            self.sent.lock().expect("fake channel lock").clone()
        }
    }

    /// Decode a request body back into the provision (what the guest sees).
    fn decode_provision(body: &[u8]) -> (u8, String, SshEpochProvision) {
        let envelope: DecodedEnvelope = ciborium::from_reader(body).expect("envelope decodes");
        let provision: SshEpochProvision =
            ciborium::from_reader(&envelope.p[..]).expect("provision decodes");
        (envelope.v, envelope.t, provision)
    }

    fn ack_body(cid: u32, epoch: u64, ok: bool) -> Vec<u8> {
        let payload = encode_payload(&SshEpochAck { cid, epoch, ok }).expect("ack encodes");
        encode_envelope(EPOCH_INTRO_GENERATION, SSH_EPOCH_ACK_WIRE, payload).expect("body encodes")
    }

    impl ConsoleChannel for FakeConsoleChannel {
        fn negotiated_generation(&self) -> u8 {
            self.negotiated
        }

        #[allow(clippy::manual_async_fn)]
        fn request(
            &self,
            flags: u8,
            body: Vec<u8>,
        ) -> impl Future<Output = Result<ConsoleResponse, String>> + Send {
            async move {
                if let Some(detail) = &self.fail_send {
                    return Err(detail.clone());
                }
                self.sent
                    .lock()
                    .expect("fake channel lock")
                    .push((flags, body.clone()));
                let (_, _, provision) = decode_provision(&body);
                let response_body = match self.ack {
                    FakeAck::Echo => ack_body(provision.cid, provision.epoch, true),
                    FakeAck::WrongCid => ack_body(provision.cid + 1, provision.epoch, true),
                    FakeAck::WrongEpoch => ack_body(provision.cid, provision.epoch + 1, true),
                    FakeAck::NotOk => ack_body(provision.cid, provision.epoch, false),
                    FakeAck::WrongType => {
                        let payload = encode_payload(&provision).expect("provision encodes");
                        encode_envelope(EPOCH_INTRO_GENERATION, SSH_EPOCH_PROVISION_WIRE, payload)
                            .expect("body encodes")
                    }
                    FakeAck::Corrupt => b"\xff\xfe not cbor".to_vec(),
                };
                Ok(ConsoleResponse {
                    id: 11,
                    flags: 0,
                    body: response_body,
                })
            }
        }
    }

    const NOW: u64 = 1_700_000_000;

    fn provision() -> SshEpochProvision {
        SshEpochProvision {
            instance: "personal-pi".to_string(),
            cid: 7,
            epoch: 1,
            issued_at: NOW,
            not_before: NOW,
        }
    }

    #[tokio::test]
    async fn provision_round_trip_returns_the_epoch() {
        let channel = FakeConsoleChannel::echoing(8);
        let got = provision_epoch(&channel, &provision(), NOW).await.unwrap();
        assert_eq!(got, 1);
        // Exactly one frame went out.
        assert_eq!(channel.sent_bodies().len(), 1);
    }

    #[tokio::test]
    async fn provision_frame_matches_the_wire_contract() {
        let channel = FakeConsoleChannel::echoing(8);
        provision_epoch(&channel, &provision(), NOW).await.unwrap();
        let sent = channel.sent_bodies();
        assert_eq!(sent.len(), 1);
        let (flags, body) = &sent[0];
        // Flags are always 0 for this family.
        assert_eq!(*flags, 0);
        // Envelope {v,t,p}: negotiated generation, provision wire string,
        // byte-string payload decoding to the provision fields.
        let (v, t, decoded) = decode_provision(body);
        assert_eq!(v, 8);
        assert_eq!(t, SSH_EPOCH_PROVISION_WIRE);
        assert_eq!(t, "core.ssh_epoch.provision");
        assert_eq!(decoded, provision());
    }

    #[tokio::test]
    async fn envelope_generation_follows_negotiation() {
        let channel = FakeConsoleChannel::echoing(9);
        provision_epoch(&channel, &provision(), NOW).await.unwrap();
        let sent = channel.sent_bodies();
        let (v, _, _) = decode_provision(&sent[0].1);
        assert_eq!(
            v, 9,
            "the envelope generation written is the negotiated one"
        );
    }

    #[tokio::test]
    async fn generation_gate_refuses_old_peers_before_sending() {
        for peer in [0, 1, 2, 4, 5, 6, 7] {
            let channel = FakeConsoleChannel::echoing(peer);
            let err = provision_epoch(&channel, &provision(), NOW)
                .await
                .unwrap_err();
            assert_eq!(
                err,
                EpochProvisionError::GenerationGate {
                    peer_generation: peer,
                    required: EPOCH_INTRO_GENERATION,
                },
                "peer generation {peer} must gate locally"
            );
            assert!(
                channel.sent_bodies().is_empty(),
                "a gated peer must see no bytes at all"
            );
        }
    }

    #[tokio::test]
    async fn ack_with_wrong_epoch_is_rejected() {
        let channel = FakeConsoleChannel {
            ack: FakeAck::WrongEpoch,
            ..FakeConsoleChannel::echoing(8)
        };
        let err = provision_epoch(&channel, &provision(), NOW)
            .await
            .unwrap_err();
        assert!(
            matches!(err, EpochProvisionError::AckMismatch { .. }),
            "stale ack epoch must reject: {err}"
        );
        assert!(
            err.to_string().contains('2'),
            "message names the acked epoch: {err}"
        );
    }

    #[tokio::test]
    async fn ack_with_wrong_cid_is_rejected() {
        let channel = FakeConsoleChannel {
            ack: FakeAck::WrongCid,
            ..FakeConsoleChannel::echoing(8)
        };
        let err = provision_epoch(&channel, &provision(), NOW)
            .await
            .unwrap_err();
        assert!(
            matches!(err, EpochProvisionError::AckMismatch { .. }),
            "spoofed ack cid must reject: {err}"
        );
    }

    #[tokio::test]
    async fn negative_ack_is_rejected() {
        let channel = FakeConsoleChannel {
            ack: FakeAck::NotOk,
            ..FakeConsoleChannel::echoing(8)
        };
        let err = provision_epoch(&channel, &provision(), NOW)
            .await
            .unwrap_err();
        assert!(
            matches!(err, EpochProvisionError::AckMismatch { .. }),
            "guest rejection (ok=false) must reject: {err}"
        );
    }

    #[tokio::test]
    async fn ack_with_wrong_wire_type_is_rejected() {
        let channel = FakeConsoleChannel {
            ack: FakeAck::WrongType,
            ..FakeConsoleChannel::echoing(8)
        };
        let err = provision_epoch(&channel, &provision(), NOW)
            .await
            .unwrap_err();
        assert!(
            matches!(err, EpochProvisionError::AckMismatch { .. }),
            "non-ack wire type must reject: {err}"
        );
        assert!(
            err.to_string().contains("core.ssh_epoch.provision"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn corrupt_ack_is_rejected() {
        let channel = FakeConsoleChannel {
            ack: FakeAck::Corrupt,
            ..FakeConsoleChannel::echoing(8)
        };
        let err = provision_epoch(&channel, &provision(), NOW)
            .await
            .unwrap_err();
        assert!(
            matches!(err, EpochProvisionError::AckMismatch { .. }),
            "undecodable ack must reject: {err}"
        );
    }

    #[tokio::test]
    async fn send_failure_is_typed_fail_closed() {
        let channel = FakeConsoleChannel {
            fail_send: Some("relay hung up".to_string()),
            ..FakeConsoleChannel::echoing(8)
        };
        let err = provision_epoch(&channel, &provision(), NOW)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            EpochProvisionError::SendFailed {
                detail: "console request failed: relay hung up".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn clock_anomaly_rejects_before_sending() {
        // not_before after issued_at is incoherent.
        let channel = FakeConsoleChannel::echoing(8);
        let bad = SshEpochProvision {
            not_before: NOW + 1,
            ..provision()
        };
        let err = provision_epoch(&channel, &bad, NOW).await.unwrap_err();
        assert!(
            matches!(err, EpochProvisionError::ClockAnomaly { .. }),
            "{err}"
        );
        assert!(channel.sent_bodies().is_empty());
        // Stale and future issued_at beyond the skew bound reject.
        for issued_at in [
            NOW - MAX_PROVISION_CLOCK_SKEW_SECS - 1,
            NOW + MAX_PROVISION_CLOCK_SKEW_SECS + 1,
        ] {
            let channel = FakeConsoleChannel::echoing(8);
            let bad = SshEpochProvision {
                issued_at,
                not_before: issued_at.min(NOW),
                ..provision()
            };
            let skew_case = if issued_at > NOW { "future" } else { "stale" };
            let err = provision_epoch(&channel, &bad, NOW).await.unwrap_err();
            assert!(
                matches!(err, EpochProvisionError::ClockAnomaly { .. }),
                "{skew_case} issued_at must reject: {err}"
            );
            assert!(channel.sent_bodies().is_empty());
        }
        // The window edge itself still sends.
        for issued_at in [
            NOW - MAX_PROVISION_CLOCK_SKEW_SECS,
            NOW + MAX_PROVISION_CLOCK_SKEW_SECS,
        ] {
            let channel = FakeConsoleChannel::echoing(8);
            let edge = SshEpochProvision {
                issued_at,
                not_before: issued_at.min(NOW),
                ..provision()
            };
            provision_epoch(&channel, &edge, NOW).await.unwrap();
        }
        // A zero broker clock fails closed (freshness unverifiable).
        let channel = FakeConsoleChannel::echoing(8);
        let err = provision_epoch(&channel, &provision(), 0)
            .await
            .unwrap_err();
        assert!(
            matches!(err, EpochProvisionError::ClockAnomaly { .. }),
            "{err}"
        );
        assert!(channel.sent_bodies().is_empty());
    }

    #[tokio::test]
    async fn provision_payloads_round_trip_cbor() {
        let bytes = encode_payload(&provision()).unwrap();
        let back: SshEpochProvision = ciborium::from_reader(&bytes[..]).unwrap();
        assert_eq!(back, provision());
        let ack = SshEpochAck {
            cid: 7,
            epoch: 3,
            ok: false,
        };
        let bytes = encode_payload(&ack).unwrap();
        let back: SshEpochAck = ciborium::from_reader(&bytes[..]).unwrap();
        assert_eq!(back, ack);
    }
}
