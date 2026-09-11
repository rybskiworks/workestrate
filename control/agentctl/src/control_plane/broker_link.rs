//! Direct managed-broker transport owned by the existing control-service owner.
//!
//! A verified stream is a prerequisite: Hello identifiers do not authenticate
//! an endpoint or a VM. The runtime owner supplies the protected, launch-bound
//! stream and a retained ownership check. No guest exec, endpoint discovery,
//! credential lookup, second registry or automatic reconnect occurs here.
//!
//! The owner must select on `next_probe_at` alongside its ordinary requests and
//! call `probe`, even while no policy changes occur. These probes renew only the
//! management lease, never policy readiness. Dropping an in-flight exchange
//! closes its stream and invalidates the controller session synchronously.
//! The owner must call `close` before dropping an idle link during shutdown or
//! loss handling: an idle link owns its stream, not a borrow of the controller.

use std::time::Duration;

use microsandbox_protocol::broker as wire;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::Instant;

use crate::config::SecretViolationPolicy;
use crate::microsandbox::plan::CredentialBinding;

use super::custody::{PreparedSshPolicy, SshController, SshPolicyTransaction};
use super::types::{
    AppliedOutcome, BrokerFailure, BrokerObservation, BrokerSession, ControlError, LaunchRef,
    OpaqueId, SshStatus,
};

const EXCHANGE_BUDGET: Duration = Duration::from_secs(10);

#[cfg(test)]
#[path = "broker_link_tests.rs"]
mod tests;

/// One already authenticated transport, not another owner of desired state.
/// `check_owner` must perform only bounded synchronous retained-owner checks;
/// expensive discovery, secret resolution and runtime selection belong outside
/// this leaf. The owner also calls `close` on a runtime/state-loss notification.
pub(crate) struct BrokerLink<S, G> {
    stream: Option<S>,
    check_owner: G,
    session: BrokerSession,
    lease_deadline: Instant,
    next_probe: Instant,
}

impl<S, G> BrokerLink<S, G>
where
    S: AsyncRead + AsyncWrite + Unpin,
    G: Fn() -> Result<(), ControlError>,
{
    /// The caller has verified this stream's protected transport and exact
    /// broker runtime ownership. The callback retains that ownership domain;
    /// a successfully decoded Welcome alone supplies no such proof.
    pub(crate) async fn connect_verified(
        mut stream: S,
        check_owner: G,
        controller: &mut SshController,
    ) -> Result<(Self, Vec<SshPolicyTransaction>), ControlError> {
        let started = Instant::now();
        check_owner()?;
        // A new Hello may retire the old broker fence before a Welcome arrives.
        // Invalidate old readiness before any bytes, including on cancellation
        // or an invalid/absent replacement reply.
        controller.begin_broker_connection()?;
        let hello = wire::Request::Hello(wire::Hello {
            version: wire::VERSION,
            controller: id(controller.incarnation())?,
        });
        if Instant::now() >= started + EXCHANGE_BUDGET {
            return Err(ControlError::BrokerUnavailable);
        }
        let reply = tokio::time::timeout_at(started + EXCHANGE_BUDGET, async {
            wire::write_request(&mut stream, &hello).await?;
            wire::read_reply(&mut stream).await
        })
        .await
        .map_err(|_| ControlError::BrokerUnavailable)?
        .map_err(|_| ControlError::BrokerUnavailable)?;
        check_owner()?;
        if Instant::now() >= started + EXCHANGE_BUDGET {
            return Err(ControlError::BrokerUnavailable);
        }
        let wire::Reply::Hello(welcome) = reply else {
            return Err(ControlError::StaleObservation);
        };
        let session = session_from_wire(welcome.session);
        let pending = controller.broker_session_authenticated(session.clone())?;
        Ok((
            Self {
                stream: Some(stream),
                check_owner,
                session,
                // Start-of-exchange timing is conservative: peer processing
                // and reply latency never extend the absolute local lease.
                lease_deadline: started + Duration::from_secs(wire::MANAGEMENT_LEASE_SECS),
                next_probe: started + Duration::from_secs(wire::MANAGEMENT_PROBE_SECS),
            },
            pending,
        ))
    }

    pub(crate) fn next_probe_at(&self) -> Instant {
        self.next_probe
    }

    pub(crate) fn session(&self) -> &BrokerSession {
        &self.session
    }

    /// Close before returning state/endpoint loss to the supervising owner.
    /// A stale link cannot invalidate a newer authenticated controller session.
    pub(crate) fn close(&mut self, controller: &mut SshController) {
        drop(self.stream.take());
        let _ = controller.broker_disconnected(&self.session);
    }

    /// Apply material already resolved by the trusted owner, never caller data.
    /// Material/trust versions and independent pins must come from that owner;
    /// no raw credential bytes enter public control requests or durable state.
    /// `expected_revision` is the last accepted revision on this connection,
    /// supplied by the single reconciler; an ambiguous write is never retried.
    pub(crate) async fn apply(
        &mut self,
        controller: &mut SshController,
        intent: &SshPolicyTransaction,
        policy: wire::Policy,
        expected_revision: Option<u64>,
    ) -> Result<(PreparedSshPolicy, SshStatus), ControlError> {
        let mut exchange = self.begin(controller)?;
        validate_projection(intent, &policy)?;
        let digest = wire::policy_digest(&policy).map_err(|_| ControlError::InvalidPolicy)?;
        let prepared = exchange
            .controller
            .prepare_policy(intent, OpaqueId::from_bytes(digest.bytes()))?;
        let target = transaction(&prepared)?;
        let request = wire::Request::Apply(wire::Apply {
            transaction: target.clone(),
            expected_revision,
            policy,
        });
        let reply = exchange.request(&request, Some(&prepared)).await?;
        let status = exchange.observe(reply, &target)?;
        if status
            .observed
            .as_ref()
            .is_some_and(|observation| observation.outcome != AppliedOutcome::StateLost)
        {
            exchange.keep_open(false)?;
        }
        Ok((prepared, status))
    }

    /// Poll the exact prepared transaction. Pending is not cancellation or
    /// process-exit proof; only the broker's complete Applied receipt is ready.
    pub(crate) async fn finish(
        &mut self,
        controller: &mut SshController,
        prepared: &PreparedSshPolicy,
    ) -> Result<SshStatus, ControlError> {
        let mut exchange = self.begin(controller)?;
        let target = transaction(prepared)?;
        let reply = exchange
            .request(&wire::Request::Finish(target.clone()), Some(prepared))
            .await?;
        let status = exchange.observe(reply, &target)?;
        if status
            .observed
            .as_ref()
            .is_some_and(|observation| observation.outcome != AppliedOutcome::StateLost)
        {
            exchange.keep_open(false)?;
        }
        Ok(status)
    }

    /// Fenced liveness only. It neither prepares nor acknowledges any policy.
    pub(crate) async fn probe(
        &mut self,
        controller: &mut SshController,
    ) -> Result<(), ControlError> {
        let mut exchange = self.begin(controller)?;
        let session = session_to_wire(&exchange.link.session)?;
        let reply = exchange
            .request(&wire::Request::Probe(session), None)
            .await?;
        if !matches!(reply, wire::Reply::Probe(probe) if probe.current && probe.session == session)
        {
            return Err(ControlError::StaleObservation);
        }
        exchange.keep_open(true)?;
        Ok(())
    }

    fn begin<'a>(
        &'a mut self,
        controller: &'a mut SshController,
    ) -> Result<Exchange<'a, S, G>, ControlError> {
        let started = Instant::now();
        let valid = (self.check_owner)()
            .and_then(|()| controller.validate_broker_session(&self.session))
            .and_then(|()| {
                if self.stream.is_some() && started < self.lease_deadline {
                    Ok(())
                } else {
                    Err(ControlError::BrokerUnavailable)
                }
            });
        if let Err(error) = valid {
            self.close(controller);
            return Err(error);
        }
        let stream = self.stream.take();
        Ok(Exchange {
            link: self,
            controller,
            stream,
            started,
        })
    }
}

/// Ownership is removed from the reusable link before the first await. Drop
/// therefore fences an interrupted write/read, even if its caller cancels the
/// future without polling an error. No background task or unbounded queue exists.
struct Exchange<'a, S, G> {
    link: &'a mut BrokerLink<S, G>,
    controller: &'a mut SshController,
    stream: Option<S>,
    started: Instant,
}

impl<S, G> Exchange<'_, S, G>
where
    S: AsyncRead + AsyncWrite + Unpin,
    G: Fn() -> Result<(), ControlError>,
{
    async fn request(
        &mut self,
        request: &wire::Request,
        prepared: Option<&PreparedSshPolicy>,
    ) -> Result<wire::Reply, ControlError> {
        (self.link.check_owner)()?;
        self.controller
            .validate_broker_session(&self.link.session)?;
        if let Some(prepared) = prepared {
            self.controller.validate_prepared(prepared)?;
        }
        let deadline = self.link.lease_deadline.min(self.started + EXCHANGE_BUDGET);
        if Instant::now() >= deadline {
            return Err(ControlError::BrokerUnavailable);
        }
        let stream = self
            .stream
            .as_mut()
            .ok_or(ControlError::BrokerUnavailable)?;
        let reply = tokio::time::timeout_at(deadline, async {
            wire::write_request(stream, request).await?;
            wire::read_reply(stream).await
        })
        .await
        .map_err(|_| ControlError::BrokerUnavailable)?
        .map_err(|_| ControlError::BrokerUnavailable)?;
        (self.link.check_owner)()?;
        if Instant::now() >= deadline {
            return Err(ControlError::BrokerUnavailable);
        }
        self.controller
            .validate_broker_session(&self.link.session)?;
        if let Some(prepared) = prepared {
            self.controller.validate_prepared(prepared)?;
        }
        Ok(reply)
    }

    fn observe(
        &mut self,
        reply: wire::Reply,
        target: &wire::TransactionRef,
    ) -> Result<SshStatus, ControlError> {
        let wire::Reply::Observation(observation) = reply else {
            return Err(ControlError::StaleObservation);
        };
        if observation.transaction != *target {
            return Err(ControlError::StaleObservation);
        }
        self.controller.observe(BrokerObservation {
            session: session_from_wire(target.session),
            launch: launch_from_wire(&target.launch),
            revision: target.revision,
            policy_digest: OpaqueId::from_bytes(target.policy_digest.bytes()),
            outcome: match observation.outcome {
                wire::Outcome::Pending => AppliedOutcome::Pending,
                wire::Outcome::Applied => AppliedOutcome::Applied,
                wire::Outcome::Rejected => AppliedOutcome::Rejected,
                wire::Outcome::StateLost => AppliedOutcome::StateLost,
            },
            failure: observation.failure.map(|failure| match failure {
                wire::Failure::InvalidPolicy => BrokerFailure::InvalidPolicy,
                wire::Failure::MissingCredential => BrokerFailure::MissingCredential,
                wire::Failure::InvalidHostTrust => BrokerFailure::InvalidHostTrust,
                wire::Failure::StaleLaunch => BrokerFailure::StaleLaunch,
                wire::Failure::RetirementIncomplete => BrokerFailure::RetirementIncomplete,
                wire::Failure::Unavailable => BrokerFailure::Unavailable,
            }),
        })
    }

    fn keep_open(mut self, renew_lease: bool) -> Result<(), ControlError> {
        (self.link.check_owner)()?;
        self.controller
            .validate_broker_session(&self.link.session)?;
        if Instant::now() >= self.link.lease_deadline.min(self.started + EXCHANGE_BUDGET) {
            return Err(ControlError::BrokerUnavailable);
        }
        if renew_lease {
            // The broker renews only an exact current Probe. Policy traffic
            // must not postpone the owner's next heartbeat or local expiry.
            self.link.lease_deadline =
                self.started + Duration::from_secs(wire::MANAGEMENT_LEASE_SECS);
            self.link.next_probe = self.started + Duration::from_secs(wire::MANAGEMENT_PROBE_SECS);
        }
        self.link.stream = self.stream.take();
        Ok(())
    }
}

impl<S, G> Drop for Exchange<'_, S, G> {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.take() {
            drop(stream);
            let _ = self.controller.broker_disconnected(&self.link.session);
        }
    }
}

fn id(value: &OpaqueId) -> Result<wire::Id, ControlError> {
    let mut bytes = [0; 32];
    hex::decode_to_slice(value.as_str(), &mut bytes).map_err(|_| ControlError::InvalidPolicy)?;
    wire::Id::from_bytes(bytes).map_err(|_| ControlError::InvalidPolicy)
}

fn session_to_wire(value: &BrokerSession) -> Result<wire::BrokerSession, ControlError> {
    Ok(wire::BrokerSession {
        controller: id(&value.controller)?,
        broker: id(&value.broker)?,
        connection: value.connection,
    })
}

fn session_from_wire(value: wire::BrokerSession) -> BrokerSession {
    BrokerSession {
        controller: OpaqueId::from_bytes(value.controller.bytes()),
        broker: OpaqueId::from_bytes(value.broker.bytes()),
        connection: value.connection,
    }
}

fn transaction(value: &PreparedSshPolicy) -> Result<wire::TransactionRef, ControlError> {
    let target = &value.intent.launch;
    Ok(wire::TransactionRef {
        session: session_to_wire(&value.intent.session)?,
        launch: wire::LaunchRef {
            instance: wire::InstanceRef {
                workload: wire::WorkloadRef {
                    context: target.instance.workload.context.clone(),
                    name: target.instance.workload.name.clone(),
                },
                instance: target.instance.instance.clone(),
            },
            generation: id(&target.generation)?,
        },
        revision: value.intent.revision,
        policy_digest: id(&value.policy_digest)?,
    })
}

fn launch_from_wire(value: &wire::LaunchRef) -> LaunchRef {
    LaunchRef {
        instance: super::types::InstanceRef {
            workload: super::types::WorkloadRef {
                context: value.instance.workload.context.clone(),
                name: value.instance.workload.name.clone(),
            },
            instance: value.instance.instance.clone(),
        },
        generation: OpaqueId::from_bytes(value.generation.bytes()),
    }
}

/// Preserve the complete compiled grant relationship when expanding its exact
/// host/user/port lists. Wildcards, ambiguous tuples and silently omitted
/// combinations are refused, never interpreted as resolved-IP authorization.
/// Key/trust provenance and scanner material remain the trusted resolver's
/// responsibility; this validates their catalog attachment, not their origin.
fn validate_projection(
    intent: &SshPolicyTransaction,
    policy: &wire::Policy,
) -> Result<(), ControlError> {
    policy.validate().map_err(|_| ControlError::InvalidPolicy)?;
    if policy.destroyed != intent.destroyed || intent.credentials.len() > wire::MAX_RECORDS {
        return Err(ControlError::InvalidPolicy);
    }
    let mut expected = 0usize;
    for grant in &intent.credentials {
        let count = grant
            .hosts
            .len()
            .checked_mul(grant.users.len())
            .and_then(|count| count.checked_mul(grant.ports.len()))
            .ok_or(ControlError::InvalidPolicy)?;
        expected = expected
            .checked_add(count)
            .ok_or(ControlError::InvalidPolicy)?;
        if grant.binding != CredentialBinding::Broker || count == 0 || expected > wire::MAX_RECORDS
        {
            return Err(ControlError::InvalidPolicy);
        }
    }
    if expected != policy.credentials.len() {
        return Err(ControlError::InvalidPolicy);
    }
    for record in &policy.credentials {
        if !intent.credentials.iter().any(|grant| {
            grant.name == record.name
                && grant.material == record.material
                && grant.hosts.contains(&record.host)
                && grant.users.contains(&record.user)
                && grant.ports.contains(&record.port)
                && matches!(
                    (grant.on_violation, &record.on_violation),
                    (
                        SecretViolationPolicy::Passthrough,
                        wire::Violation::Passthrough
                    ) | (SecretViolationPolicy::Block, wire::Violation::Block)
                        | (
                            SecretViolationPolicy::BlockAndLog,
                            wire::Violation::BlockAndLog
                        )
                        | (
                            SecretViolationPolicy::BlockAndTerminate,
                            wire::Violation::BlockAndTerminate
                        )
                )
        }) {
            return Err(ControlError::InvalidPolicy);
        }
    }
    if policy.patterns.iter().any(|pattern| {
        !intent
            .credentials
            .iter()
            .any(|grant| grant.name == pattern.credential_id)
    }) {
        return Err(ControlError::InvalidPolicy);
    }
    Ok(())
}
